//! M4 — Incremental Wiki compiler (plan §5 / M4).
//!
//! The Wiki is a *derived representation*, never a source of truth: it is
//! written only to its own tables, never to `tasks`/`events`, so a failed
//! compilation can never corrupt task authority.
//!
//! Behaviour:
//! - material-change classification: only knowledge-changing events compile;
//! - incremental: `wiki_state.last_compiled_seq` bounds the work per run;
//! - idempotent: node upserts compare content hashes (repeat ingestion does not
//!   duplicate facts) and edges are `UNIQUE` on (from, to, type);
//! - provenance: every node points back to its source/artifact refs;
//! - contradictions: two evidence nodes for the same claim title with different
//!   content keep *both* refs and gain `contradicts` edges;
//! - stale refs are flagged, not dropped silently;
//! - write-amplification and per-run cost are reported and logged.

use crate::error::Result;
use crate::ids::{content_hash, new_id, parse_ref};
use crate::storage::db::{json_param, row_i64, row_str, Database, RowMap};
use crate::storage::event_ledger::{now, EventLedger};
use crate::storage::objects::ObjectStore;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const NODE_KINDS: &[&str] = &[
    "task",
    "evidence",
    "artifact",
    "failure",
    "verification",
    "constraint",
    "decision",
];
pub const EDGE_TYPES: &[&str] = &[
    "supported_by",
    "produced_by",
    "contradicts",
    "verified_by",
    "relevant_to",
    "depends_on",
    "requires",
    "supersedes",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompileReport {
    pub events_processed: usize,
    pub nodes_created: usize,
    pub nodes_updated: usize,
    pub nodes_unchanged: usize,
    pub edges_added: usize,
    pub conflicts: usize,
    pub stale_refs: usize,
    pub write_amplification: f64,
    pub failures: Vec<String>,
}

impl CompileReport {
    pub fn as_value(&self) -> Value {
        json!(self)
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedRef {
    pub title: String,
    pub summary: String,
}

pub trait RefResolver {
    fn resolve(&self, ref_id: &str) -> Result<Option<ResolvedRef>>;
}

/// Resolves `source://` and `artifact://` refs against the stores, using the
/// derived search index for human titles when available.
pub struct StoreResolver {
    pub db: Database,
    pub objects: ObjectStore,
}

impl RefResolver for StoreResolver {
    fn resolve(&self, ref_id: &str) -> Result<Option<ResolvedRef>> {
        let parsed = match parse_ref(ref_id) {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
        let title: String = self
            .db
            .query_one("SELECT title FROM search_docs WHERE ref_id = ?1", [ref_id])?
            .map(|r| row_str(&r, "title"))
            .unwrap_or_default();
        match parsed.kind.as_str() {
            "source" => {
                let Some(source) = self.objects.get_source(&parsed.ident, parsed.revision)? else {
                    return Ok(None);
                };
                let title = if title.is_empty() {
                    source.origin.clone()
                } else {
                    title
                };
                Ok(Some(ResolvedRef {
                    title,
                    summary: format!("source from {}", source.origin),
                }))
            }
            "artifact" => {
                let Some(artifact) = self.objects.get_artifact(&parsed.ident)? else {
                    return Ok(None);
                };
                let title = if title.is_empty() {
                    artifact.path.clone()
                } else {
                    title
                };
                Ok(Some(ResolvedRef {
                    title,
                    summary: format!("artifact {}", artifact.path),
                }))
            }
            _ => Ok(None),
        }
    }
}

pub struct WikiCompiler {
    pub db: Database,
    pub ledger: EventLedger,
    pub resolver: Box<dyn RefResolver>,
}

impl WikiCompiler {
    pub fn new(db: Database, ledger: EventLedger, resolver: Box<dyn RefResolver>) -> Self {
        Self {
            db,
            ledger,
            resolver,
        }
    }

    /// Material-change classifier: events that can change derived knowledge.
    pub fn material_event_type(event_type: &str) -> bool {
        matches!(
            event_type,
            "task.created"
                | "task.status_changed"
                | "tool.committed"
                | "task.error_opened"
                | "task.error_resolved"
                | "task.issue_recorded"
        )
    }

    pub fn compile_task(&self, task_id: &str) -> Result<CompileReport> {
        let last_seq: i64 = self
            .db
            .query_one(
                "SELECT last_compiled_seq FROM wiki_state WHERE task_id = ?1",
                [task_id],
            )?
            .map(|r| row_i64(&r, "last_compiled_seq"))
            .unwrap_or(0);
        let events: Vec<_> = self
            .ledger
            .events_for(task_id, last_seq)?
            .into_iter()
            .filter(|e| Self::material_event_type(&e.event_type))
            .collect();
        let mut report = CompileReport::default();
        if events.is_empty() {
            self.log(task_id, last_seq, "empty", &report)?;
            return Ok(report);
        }

        let mut task_node: Option<String> = self.find_node_id(task_id, "task", "task")?;
        for event in &events {
            if let Err(exc) = self.apply_event(
                task_id,
                &event.event_type,
                &event.payload,
                &mut task_node,
                &mut report,
            ) {
                report.failures.push(format!("seq {}: {exc}", event.seq));
                self.log(task_id, event.seq, "failed", &report)?;
                return Err(exc);
            }
            report.events_processed += 1;
        }

        report.write_amplification = if report.events_processed == 0 {
            0.0
        } else {
            ((report.nodes_created + report.nodes_updated + report.edges_added) as f64
                / report.events_processed as f64
                * 1000.0)
                .round()
                / 1000.0
        };
        let seq = events.last().map(|e| e.seq).unwrap_or(last_seq);
        self.log(task_id, seq, "ok", &report)?;
        self.db.execute(
            "INSERT OR REPLACE INTO wiki_state(task_id, last_compiled_seq, updated_at)
             VALUES(?1,?2,?3)",
            params![task_id, seq, now()],
        )?;
        Ok(report)
    }

    // ---- event → nodes/edges ----

    fn apply_event(
        &self,
        task_id: &str,
        event_type: &str,
        payload: &Value,
        task_node: &mut Option<String>,
        report: &mut CompileReport,
    ) -> Result<()> {
        match event_type {
            "task.created" => {
                let goal = payload.get("goal").and_then(|g| g.as_str()).unwrap_or("");
                let constraints = payload
                    .get("constraints")
                    .and_then(|c| c.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                let node = self.upsert_node(
                    task_id,
                    "task",
                    "task",
                    goal,
                    &format!("status=created; constraints={constraints}"),
                    &format!("task://{task_id}/index@1"),
                    vec![],
                    report,
                )?;
                *task_node = Some(node);
            }
            "task.status_changed" => {
                let status = payload.get("status").and_then(|s| s.as_str()).unwrap_or("");
                let node = self.upsert_node(
                    task_id,
                    "task",
                    "task",
                    "task",
                    &format!("status={status}"),
                    &format!("task://{task_id}/index@1"),
                    vec![],
                    report,
                )?;
                *task_node = Some(node);
            }
            "tool.committed" => {
                let operation = payload
                    .get("operation_id")
                    .and_then(|o| o.as_str())
                    .unwrap_or("operation");
                let status = payload
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("ok");
                let refs: Vec<String> = payload
                    .get("result_refs")
                    .and_then(|r| r.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let error_code = payload
                    .get("error_code")
                    .and_then(|e| e.as_str())
                    .unwrap_or("TOOL_ERROR");

                if status == "ok" || status == "denied" {
                    for reference in &refs {
                        self.ingest_ref(task_id, reference, task_node, report)?;
                    }
                }
                if status != "ok" {
                    let failure_node = self.upsert_node(
                        task_id,
                        "failure",
                        &format!("{operation}:{error_code}"),
                        &format!("{operation} failed"),
                        &format!("status={status}; error_code={error_code}"),
                        &format!("event://{task_id}/failure"),
                        vec![format!("operation={operation}")],
                        report,
                    )?;
                    if let Some(node) = task_node {
                        self.add_edge(task_id, node, &failure_node, "relevant_to", report)?;
                    }
                }
            }
            "task.error_opened" => {
                let key = payload
                    .get("key")
                    .and_then(|k| k.as_str())
                    .unwrap_or("error");
                let error_code = payload
                    .get("error_code")
                    .and_then(|e| e.as_str())
                    .unwrap_or("TOOL_ERROR");
                let failure_node = self.upsert_node(
                    task_id,
                    "failure",
                    key,
                    &format!("failure: {error_code}"),
                    &format!("open failure {key}"),
                    &format!("event://{task_id}/failure/{key}"),
                    vec![],
                    report,
                )?;
                if let Some(node) = task_node {
                    self.add_edge(task_id, node, &failure_node, "relevant_to", report)?;
                }
            }
            "task.error_resolved" => {
                let key = payload
                    .get("key")
                    .and_then(|k| k.as_str())
                    .unwrap_or("error");
                if let Some(node) = self.find_node_id(task_id, "failure", key)? {
                    self.db.execute(
                        "UPDATE wiki_nodes SET status = 'resolved', updated_at = ?1
                         WHERE node_id = ?2",
                        params![now(), node],
                    )?;
                }
            }
            "task.issue_recorded" => {
                let criterion = payload
                    .get("criterion")
                    .and_then(|c| c.as_str())
                    .unwrap_or("issue");
                self.upsert_node(
                    task_id,
                    "verification",
                    criterion,
                    &format!("open issue: {criterion}"),
                    payload
                        .get("findings")
                        .and_then(|f| f.as_str())
                        .unwrap_or("acceptance issue recorded"),
                    &format!("event://{task_id}/issue/{criterion}"),
                    vec![],
                    report,
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn ingest_ref(
        &self,
        task_id: &str,
        reference: &str,
        task_node: &Option<String>,
        report: &mut CompileReport,
    ) -> Result<()> {
        let parsed = match parse_ref(reference) {
            Ok(p) => p,
            Err(_) => {
                report.stale_refs += 1;
                return Ok(());
            }
        };
        let kind = match parsed.kind.as_str() {
            "source" => "evidence",
            "artifact" => "artifact",
            _ => return Ok(()),
        };
        let resolved = self.resolver.resolve(reference)?;
        let (title, summary, status) = match resolved {
            Some(r) => (r.title, r.summary, "active"),
            None => {
                report.stale_refs += 1;
                (
                    reference.to_string(),
                    format!("unresolved ref {reference}"),
                    "stale",
                )
            }
        };
        let node = self.upsert_node(
            task_id,
            kind,
            reference,
            &title,
            &summary,
            reference,
            vec![reference.to_string()],
            report,
        )?;
        if kind == "evidence" {
            if let Some(task) = task_node {
                self.add_edge(task_id, task, &node, "supported_by", report)?;
            }
            self.detect_conflicts(task_id, &node, &title, reference, report)?;
        } else {
            if let Some(task) = task_node {
                self.add_edge(task_id, task, &node, "produced_by", report)?;
            }
        }
        if status == "stale" {
            self.db.execute(
                "UPDATE wiki_nodes SET status = 'stale' WHERE node_id = ?1",
                [node],
            )?;
        }
        Ok(())
    }

    /// Same claim title, different content → keep both nodes, link `contradicts`.
    fn detect_conflicts(
        &self,
        task_id: &str,
        node_id: &str,
        title: &str,
        reference: &str,
        report: &mut CompileReport,
    ) -> Result<()> {
        let normalized = title.trim().to_lowercase();
        if normalized.is_empty() || reference.is_empty() {
            return Ok(());
        }
        let rows = self.db.query(
            "SELECT node_id, content_hash, node_key FROM wiki_nodes
             WHERE task_id = ?1 AND kind = 'evidence' AND node_key != ?2",
            params![task_id, reference],
        )?;
        for row in rows {
            let other_id = row_str(&row, "node_id");
            let other_title: String = self
                .db
                .query_one(
                    "SELECT title FROM wiki_nodes WHERE node_id = ?1",
                    [other_id.clone()],
                )?
                .map(|r| row_str(&r, "title").trim().to_lowercase())
                .unwrap_or_default();
            if other_title != normalized {
                continue;
            }
            let this_hash: String = self
                .db
                .query_one(
                    "SELECT content_hash FROM wiki_nodes WHERE node_id = ?1",
                    [node_id],
                )?
                .map(|r| row_str(&r, "content_hash"))
                .unwrap_or_default();
            if this_hash != row_str(&row, "content_hash") {
                self.add_edge(task_id, node_id, &other_id, "contradicts", report)?;
                self.add_edge(task_id, &other_id, node_id, "contradicts", report)?;
                report.conflicts += 1;
                for id in [node_id, other_id.as_str()] {
                    self.db.execute(
                        "UPDATE wiki_nodes SET status = 'disputed', updated_at = ?1
                         WHERE node_id = ?2",
                        params![now(), id],
                    )?;
                }
            }
        }
        Ok(())
    }

    // ---- persistence helpers ----

    #[allow(clippy::too_many_arguments)]
    fn upsert_node(
        &self,
        task_id: &str,
        kind: &str,
        node_key: &str,
        title: &str,
        summary: &str,
        ref_id: &str,
        provenance: Vec<String>,
        report: &mut CompileReport,
    ) -> Result<String> {
        assert!(NODE_KINDS.contains(&kind), "unknown node kind {kind}");
        let new_hash = content_hash(format!("{title}|{summary}").as_bytes());
        match self.find_node(task_id, kind, node_key)? {
            None => {
                let node_id = new_id("wn");
                self.db.execute(
                    "INSERT INTO wiki_nodes(node_id, task_id, kind, node_key, title, summary,
                     content_hash, ref_id, revision, provenance_json, status, updated_at)
                     VALUES(?1,?2,?3,?4,?5,?6,?7,?8,1,?9,'active',?10)",
                    params![
                        node_id,
                        task_id,
                        kind,
                        node_key,
                        title,
                        summary,
                        new_hash,
                        ref_id,
                        json_param(&provenance),
                        now()
                    ],
                )?;
                report.nodes_created += 1;
                Ok(node_id)
            }
            Some(row) => {
                let node_id = row_str(&row, "node_id");
                let old_hash = row_str(&row, "content_hash");
                if old_hash == new_hash {
                    report.nodes_unchanged += 1;
                } else {
                    self.db.execute(
                        "UPDATE wiki_nodes SET title = ?1, summary = ?2, content_hash = ?3,
                         ref_id = ?4, provenance_json = ?5, revision = revision + 1,
                         updated_at = ?6 WHERE node_id = ?7",
                        params![
                            title,
                            summary,
                            new_hash,
                            ref_id,
                            json_param(&provenance),
                            now(),
                            node_id
                        ],
                    )?;
                    report.nodes_updated += 1;
                }
                Ok(node_id)
            }
        }
    }

    fn add_edge(
        &self,
        task_id: &str,
        from_node: &str,
        to_node: &str,
        edge_type: &str,
        report: &mut CompileReport,
    ) -> Result<()> {
        assert!(
            EDGE_TYPES.contains(&edge_type),
            "unknown edge type {edge_type}"
        );
        let inserted = self.db.execute(
            "INSERT OR IGNORE INTO wiki_edges(edge_id, task_id, from_node, to_node, edge_type, created_at)
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![new_id("we"), task_id, from_node, to_node, edge_type, now()],
        )?;
        if inserted > 0 {
            report.edges_added += 1;
        }
        Ok(())
    }

    fn find_node(&self, task_id: &str, kind: &str, node_key: &str) -> Result<Option<RowMap>> {
        self.db.query_one(
            "SELECT * FROM wiki_nodes WHERE task_id = ?1 AND kind = ?2 AND node_key = ?3",
            params![task_id, kind, node_key],
        )
    }

    fn find_node_id(&self, task_id: &str, kind: &str, node_key: &str) -> Result<Option<String>> {
        Ok(self
            .find_node(task_id, kind, node_key)?
            .map(|r| row_str(&r, "node_id")))
    }

    fn log(&self, task_id: &str, seq: i64, status: &str, report: &CompileReport) -> Result<()> {
        self.db.execute(
            "INSERT INTO wiki_compile_log(log_id, task_id, seq, status, report_json, created_at)
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                new_id("wlog"),
                task_id,
                seq,
                status,
                report.as_value().to_string(),
                now()
            ],
        )?;
        Ok(())
    }

    // ---- read API (CLI / tests) ----

    pub fn nodes_for(&self, task_id: &str) -> Result<Vec<Value>> {
        let rows = self.db.query(
            "SELECT * FROM wiki_nodes WHERE task_id = ?1 ORDER BY updated_at, node_id",
            [task_id],
        )?;
        Ok(rows.iter().map(row_to_node_value).collect())
    }

    pub fn edges_for(&self, task_id: &str) -> Result<Vec<Value>> {
        let rows = self.db.query(
            "SELECT * FROM wiki_edges WHERE task_id = ?1 ORDER BY created_at, edge_id",
            [task_id],
        )?;
        Ok(rows
            .iter()
            .map(|r| {
                json!({
                    "from_node": row_str(r, "from_node"),
                    "to_node": row_str(r, "to_node"),
                    "edge_type": row_str(r, "edge_type"),
                })
            })
            .collect())
    }

    pub fn summary(&self, task_id: &str) -> Result<Value> {
        let node_rows = self.db.query(
            "SELECT kind, COUNT(*) AS n FROM wiki_nodes WHERE task_id = ?1 GROUP BY kind",
            [task_id],
        )?;
        let mut kinds = serde_json::Map::new();
        for row in node_rows {
            kinds.insert(row_str(&row, "kind"), json!(row_i64(&row, "n")));
        }
        let edge_count: i64 = self
            .db
            .query_one(
                "SELECT COUNT(*) AS n FROM wiki_edges WHERE task_id = ?1",
                [task_id],
            )?
            .map(|r| row_i64(&r, "n"))
            .unwrap_or(0);
        let last_seq: i64 = self
            .db
            .query_one(
                "SELECT last_compiled_seq FROM wiki_state WHERE task_id = ?1",
                [task_id],
            )?
            .map(|r| row_i64(&r, "last_compiled_seq"))
            .unwrap_or(0);
        let stale: i64 = self
            .db
            .query_one(
                "SELECT COUNT(*) AS n FROM wiki_nodes WHERE task_id = ?1 AND status = 'stale'",
                [task_id],
            )?
            .map(|r| row_i64(&r, "n"))
            .unwrap_or(0);
        Ok(json!({
            "task_id": task_id,
            "nodes_by_kind": kinds,
            "edges": edge_count,
            "stale_refs": stale,
            "last_compiled_seq": last_seq,
        }))
    }
}

fn row_to_node_value(row: &RowMap) -> Value {
    json!({
        "node_id": row_str(row, "node_id"),
        "kind": row_str(row, "kind"),
        "node_key": row_str(row, "node_key"),
        "title": row_str(row, "title"),
        "summary": row_str(row, "summary"),
        "ref_id": row_str(row, "ref_id"),
        "revision": row_i64(row, "revision"),
        "status": row_str(row, "status"),
        "provenance": serde_json::from_str::<Value>(&row_str(row, "provenance_json"))
            .unwrap_or(Value::Null),
    })
}
