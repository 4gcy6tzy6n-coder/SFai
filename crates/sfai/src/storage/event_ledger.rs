//! Append-only event ledger — the source of truth for task state (plan §5.1).
//!
//! Events are immutable once written. Materialized task state is a cache that
//! can always be recomputed by folding the ledger (`replay_state`), which is
//! exactly what crash recovery and the trace replayer rely on.

use crate::contracts::{Event, TaskState, TaskStatus};
use crate::error::Result;
use crate::storage::db::{json_param, row_i64, row_str, Database, RowMap};
use chrono::Utc;
use rusqlite::params;
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub fn now() -> String {
    Utc::now().to_rfc3339()
}

#[derive(Clone)]
pub struct EventLedger {
    pub db: Database,
}

impl EventLedger {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn append(
        &self,
        task_id: &str,
        actor: &str,
        event_type: &str,
        payload: Value,
        idempotency_key: Option<&str>,
    ) -> Result<Event> {
        if let Some(key) = idempotency_key {
            if let Some(row) = self
                .db
                .query_one("SELECT * FROM events WHERE idempotency_key = ?1", [key])?
            {
                return row_to_event(&row);
            }
        }
        let timestamp = now();
        let seq = self.db.insert(
            "INSERT INTO events(idempotency_key, task_id, actor, event_type, timestamp, payload)
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                idempotency_key.map(|s| s.to_string()),
                task_id,
                actor,
                event_type,
                timestamp,
                json_param(&payload)
            ],
        )?;
        Ok(Event {
            seq,
            idempotency_key: idempotency_key.map(|s| s.to_string()),
            task_id: task_id.to_string(),
            actor: actor.to_string(),
            event_type: event_type.to_string(),
            timestamp,
            payload,
        })
    }

    pub fn events_for(&self, task_id: &str, after_seq: i64) -> Result<Vec<Event>> {
        let rows = self.db.query(
            "SELECT * FROM events WHERE task_id = ?1 AND seq > ?2 ORDER BY seq",
            params![task_id, after_seq],
        )?;
        rows.iter().map(row_to_event).collect()
    }

    pub fn last_seq(&self, task_id: &str) -> Result<i64> {
        let row = self.db.query_one(
            "SELECT MAX(seq) AS m FROM events WHERE task_id = ?1",
            [task_id],
        )?;
        Ok(row.map(|r| row_i64(&r, "m")).unwrap_or(0))
    }

    /// Fold the ledger into task state. Pure: no cache reads.
    pub fn replay_state(&self, task_id: &str) -> Result<TaskState> {
        let mut state = TaskState {
            task_id: task_id.to_string(),
            ..Default::default()
        };
        let mut seen_errors: BTreeSet<String> = BTreeSet::new();
        for event in self.events_for(task_id, 0)? {
            state.last_seq = event.seq;
            let p = &event.payload;
            match event.event_type.as_str() {
                "task.created" => {
                    state.status = p
                        .get("status")
                        .and_then(|s| s.as_str())
                        .and_then(TaskStatus::from_str)
                        .unwrap_or(TaskStatus::Created);
                    state.pinned_constraints = p
                        .get("constraints")
                        .and_then(|c| c.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                }
                "task.status_changed" => {
                    if let Some(status) = p.get("status").and_then(|s| s.as_str()) {
                        if let Some(parsed) = TaskStatus::from_str(status) {
                            state.status = parsed;
                            state.version += 1;
                        }
                    }
                }
                "task.output_added" => {
                    if let Some(reference) = p.get("ref").and_then(|r| r.as_str()) {
                        state.completed_outputs.push(reference.to_string());
                    }
                }
                "task.error_opened" => {
                    let key = p
                        .get("key")
                        .and_then(|k| k.as_str())
                        .unwrap_or("")
                        .to_string();
                    state.unresolved_errors.retain(|e| {
                        e.get("key").and_then(|k| k.as_str()).map(|k| k.to_string())
                            != Some(key.clone())
                    });
                    state.unresolved_errors.push(p.clone());
                    seen_errors.insert(key);
                }
                "task.error_resolved" => {
                    if let Some(key) = p.get("key").and_then(|k| k.as_str()) {
                        seen_errors.remove(key);
                        state
                            .unresolved_errors
                            .retain(|e| e.get("key").and_then(|k| k.as_str()) != Some(key));
                    }
                }
                "task.issue_recorded" => state.open_issues.push(p.clone()),
                "verification.recorded" => state.verifications.push(p.clone()),
                "task.checkpoint" => state.checkpoint = p.clone(),
                "task.intent_started" => {
                    state.active_intent = p
                        .get("kind")
                        .and_then(|k| k.as_str())
                        .map(|s| s.to_string());
                    state.model_calls_made += 1;
                }
                "tool.committed" => {
                    state.tool_calls_made += 1;
                }
                _ => {}
            }
        }
        Ok(state)
    }

    /// Convenience: typed payload helpers used by the task store.
    pub fn append_status(&self, task_id: &str, status: &str, reason: &str) -> Result<Event> {
        self.append(
            task_id,
            "runtime",
            "task.status_changed",
            json!({"status": status, "reason": reason}),
            None,
        )
    }
}

fn row_to_event(row: &RowMap) -> Result<Event> {
    let payload: Value = serde_json::from_str(&row_str(row, "payload"))?;
    Ok(Event {
        seq: row_i64(row, "seq"),
        idempotency_key: row
            .get("idempotency_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        task_id: row_str(row, "task_id"),
        actor: row_str(row, "actor"),
        event_type: row_str(row, "event_type"),
        timestamp: row_str(row, "timestamp"),
        payload,
    })
}
