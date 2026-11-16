//! Context index: root/sub-index structure and stable nodes (plan §5.4).
//!
//! The root index is deliberately small: it exposes entrypoints only
//! (task, sources, knowledge, skills, artifacts), not the full tree.

use crate::error::Result;
use crate::ids::task_index_ref;
use crate::storage::db::{row_i64, row_str, Database};
use rusqlite::params;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct IndexNode {
    pub node_id: String,
    pub task_id: String,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub ref_id: String,
    pub revision: u32,
    pub parent: Option<String>,
}

#[derive(Clone)]
pub struct ContextIndex {
    pub db: Database,
}

impl ContextIndex {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upsert_node(
        &self,
        task_id: &str,
        kind: &str,
        title: &str,
        summary: &str,
        ref_id: &str,
        node_id: &str,
        parent: Option<&str>,
    ) -> Result<IndexNode> {
        let revision: u32 = self
            .db
            .query_one(
                "SELECT revision FROM index_nodes WHERE node_id = ?1",
                [node_id],
            )?
            .map(|r| row_i64(&r, "revision") as u32 + 1)
            .unwrap_or(1);
        self.db.execute(
            "INSERT OR REPLACE INTO index_nodes(node_id, task_id, kind, title, summary,
             ref_id, revision, parent, created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                node_id,
                task_id,
                kind,
                title,
                summary,
                ref_id,
                revision,
                parent,
                crate::storage::event_ledger::now()
            ],
        )?;
        Ok(IndexNode {
            node_id: node_id.to_string(),
            task_id: task_id.to_string(),
            kind: kind.to_string(),
            title: title.to_string(),
            summary: summary.to_string(),
            ref_id: ref_id.to_string(),
            revision,
            parent: parent.map(|s| s.to_string()),
        })
    }

    pub fn get_node(&self, node_id: &str) -> Result<Option<IndexNode>> {
        let row = self
            .db
            .query_one("SELECT * FROM index_nodes WHERE node_id = ?1", [node_id])?;
        Ok(row.as_ref().map(row_to_node))
    }

    pub fn list_nodes(
        &self,
        task_id: &str,
        parent: Option<&str>,
        limit: usize,
    ) -> Result<Vec<IndexNode>> {
        let rows = match parent {
            Some(p) => self.db.query(
                "SELECT * FROM index_nodes WHERE task_id = ?1 AND parent = ?2
                 ORDER BY created_at LIMIT ?3",
                params![task_id, p, limit as i64],
            )?,
            None => self.db.query(
                "SELECT * FROM index_nodes WHERE task_id = ?1 ORDER BY created_at LIMIT ?2",
                params![task_id, limit as i64],
            )?,
        };
        Ok(rows.iter().map(row_to_node).collect())
    }

    /// Entrypoint refs only — the model does not need every address.
    pub fn root_index(&self, task_id: &str) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        map.insert("task".into(), task_index_ref(task_id, 1));
        map.insert("sources".into(), format!("task://{task_id}/sources@1"));
        map.insert("knowledge".into(), format!("wiki://{task_id}/root@1"));
        map.insert("skills".into(), "skill://registry/index@1".into());
        map.insert("artifacts".into(), format!("task://{task_id}/artifacts@1"));
        map
    }
}

fn row_to_node(row: &crate::storage::db::RowMap) -> IndexNode {
    IndexNode {
        node_id: row_str(row, "node_id"),
        task_id: row_str(row, "task_id"),
        kind: row_str(row, "kind"),
        title: row_str(row, "title"),
        summary: row_str(row, "summary"),
        ref_id: row_str(row, "ref_id"),
        revision: row_i64(row, "revision") as u32,
        parent: row
            .get("parent")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    }
}
