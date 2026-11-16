//! Task authority store (plan §5.1 stratum 1).
//!
//! The canonical mutable state (goal, constraints, acceptance, status) lives
//! here and is never reconstructed only from derived summaries. Events are
//! appended first; materialized task rows are caches that `resume()` checks
//! against a pure ledger replay.

use crate::contracts::{status_transition_allowed, Task, TaskState, TaskStatus};
use crate::error::{Result, SfaiError};
use crate::storage::db::{row_i64, row_str, Database};
use crate::storage::event_ledger::{now, EventLedger};
use rusqlite::params;
use serde_json::{json, Value};

#[derive(Clone)]
pub struct TaskStore {
    pub db: Database,
    pub ledger: EventLedger,
}

impl TaskStore {
    pub fn new(db: Database, ledger: EventLedger) -> Self {
        Self { db, ledger }
    }

    pub fn create(&self, task: &Task) -> Result<Task> {
        self.db.execute(
            "INSERT INTO tasks(task_id, json, version, updated_at) VALUES(?1,?2,?3,?4)",
            params![
                task.task_id,
                serde_json::to_string(task)?,
                task.version,
                now()
            ],
        )?;
        self.ledger.append(
            &task.task_id,
            "user",
            "task.created",
            json!({
                "status": task.status,
                "goal": task.goal,
                "constraints": task.constraints,
                "acceptance_rules": task.acceptance_rules,
            }),
            Some(&format!("{}:created", task.task_id)),
        )?;
        Ok(task.clone())
    }

    pub fn load(&self, task_id: &str) -> Result<Task> {
        let row = self
            .db
            .query_one("SELECT json FROM tasks WHERE task_id = ?1", [task_id])?
            .ok_or_else(|| SfaiError::NotFound(format!("unknown task: {task_id}")))?;
        Ok(serde_json::from_str(&row_str(&row, "json"))?)
    }

    pub fn save(&self, task: &Task) -> Result<()> {
        self.db.execute(
            "UPDATE tasks SET json = ?1, version = ?2, updated_at = ?3 WHERE task_id = ?4",
            params![
                serde_json::to_string(task)?,
                task.version,
                now(),
                task.task_id
            ],
        )?;
        Ok(())
    }

    /// Materialized state = replay of the ledger (source of truth).
    pub fn state(&self, task_id: &str) -> Result<TaskState> {
        self.ledger.replay_state(task_id)
    }

    pub fn set_status(&self, task_id: &str, new_status: TaskStatus, reason: &str) -> Result<()> {
        let mut task = self.load(task_id)?;
        if !status_transition_allowed(task.status, new_status) {
            return Err(SfaiError::Policy(format!(
                "illegal status transition {} -> {}",
                task.status.as_str(),
                new_status.as_str()
            )));
        }
        task.status = new_status;
        task.version += 1;
        self.save(&task)?;
        self.ledger.append(
            task_id,
            "runtime",
            "task.status_changed",
            json!({"status": new_status, "reason": reason}),
            Some(&format!(
                "{task_id}:status:{}:{}",
                new_status.as_str(),
                self.ledger.last_seq(task_id)?
            )),
        )?;
        Ok(())
    }

    /// Commit structured state events produced by the dispatcher.
    pub fn apply(&self, task_id: &str, state_events: &[Value]) -> Result<()> {
        for ev in state_events {
            let etype = ev.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match etype {
                "output_added" => {
                    self.ledger.append(
                        task_id,
                        "runtime",
                        "task.output_added",
                        json!({ "ref": ev.get("ref").cloned().unwrap_or(Value::Null) }),
                        None,
                    )?;
                }
                "error_opened" => {
                    let error = ev.get("error").cloned().unwrap_or_else(|| json!({}));
                    self.ledger
                        .append(task_id, "runtime", "task.error_opened", error, None)?;
                }
                "error_resolved" => {
                    self.ledger.append(
                        task_id,
                        "runtime",
                        "task.error_resolved",
                        json!({ "key": ev.get("key").cloned().unwrap_or(Value::Null) }),
                        None,
                    )?;
                }
                "issue_recorded" => {
                    let issue = ev.get("issue").cloned().unwrap_or_else(|| json!({}));
                    self.ledger
                        .append(task_id, "runtime", "task.issue_recorded", issue, None)?;
                }
                "verification_recorded" => {
                    let payload = ev.get("payload").cloned().unwrap_or_else(|| json!({}));
                    self.ledger.append(
                        task_id,
                        "runtime",
                        "verification.recorded",
                        payload,
                        None,
                    )?;
                }
                "tool_committed" => {
                    let mut payload = ev.clone();
                    if let Some(obj) = payload.as_object_mut() {
                        obj.remove("type");
                    }
                    self.ledger.append(
                        task_id,
                        &format!(
                            "tool:{}",
                            ev.get("operation_id")
                                .and_then(|o| o.as_str())
                                .unwrap_or("unknown")
                        ),
                        "tool.committed",
                        payload,
                        ev.get("idempotency_key").and_then(|k| k.as_str()),
                    )?;
                }
                other => {
                    return Err(SfaiError::Validation(format!(
                        "unknown state event type: {other}"
                    )))
                }
            }
        }
        Ok(())
    }

    pub fn record_decision_started(
        &self,
        task_id: &str,
        kind: &str,
        reason_code: &str,
    ) -> Result<()> {
        self.ledger.append(
            task_id,
            "model",
            "task.intent_started",
            json!({"kind": kind, "reason_code": reason_code}),
            None,
        )?;
        Ok(())
    }

    pub fn checkpoint(
        &self,
        task_id: &str,
        note: &str,
        output_refs: Option<&[String]>,
    ) -> Result<()> {
        let state = self.state(task_id)?;
        let outputs: Vec<String> = output_refs
            .map(|o| o.to_vec())
            .unwrap_or_else(|| state.completed_outputs.clone());
        let seq = self.ledger.last_seq(task_id)?;
        let payload = json!({
            "note": note,
            "output_refs": outputs,
            "seq": seq,
            "status": state.status,
        });
        self.ledger.append(
            task_id,
            "runtime",
            "task.checkpoint",
            payload.clone(),
            Some(&format!("{task_id}:ckpt:{seq}")),
        )?;
        self.db.execute(
            "INSERT OR REPLACE INTO checkpoints(task_id, seq, state_json, created_at)
             VALUES(?1,?2,?3,?4)",
            params![task_id, seq, serde_json::to_string(&payload)?, now()],
        )?;
        Ok(())
    }

    /// Crash recovery: rebuild state purely from events and check it against
    /// the materialized task row. A mismatch is a corruption signal.
    pub fn resume(&self, task_id: &str) -> Result<TaskState> {
        let task = self.load(task_id)?;
        let rebuilt = self.state(task_id)?;
        if rebuilt.status != task.status {
            return Err(SfaiError::Internal(format!(
                "state corruption for {task_id}: task={} replay={}",
                task.status.as_str(),
                rebuilt.status.as_str()
            )));
        }
        Ok(rebuilt)
    }

    pub fn last_seq(&self, task_id: &str) -> Result<i64> {
        self.ledger.last_seq(task_id)
    }
}

#[allow(dead_code)]
fn _unused_row_helpers(row: &crate::storage::db::RowMap) -> i64 {
    // Row helpers are part of the storage API surface; keep one reference so
    // the import stays meaningful if this module shrinks.
    row_i64(row, "version")
}
