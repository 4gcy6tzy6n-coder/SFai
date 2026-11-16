//! Retrieval: FTS5 lexical search, bounded reads, provenance resolution.
//!
//! `context.search` is a hint, not a statement of truth. `context.read`
//! returns a bounded excerpt with a stable reference and a path back to the
//! original source or artifact.

use crate::error::{Result, SfaiError};
use crate::ids::parse_ref;
use crate::storage::db::{row_f64, row_i64, row_str, Database};
use crate::storage::objects::ObjectStore;
use rusqlite::params;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct SearchCandidate {
    pub ref_id: String,
    pub kind: String,
    pub title: String,
    pub snippet: String,
    pub lexical_score: f64,
    pub revision: u32,
    pub token_estimate: usize,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadPayload {
    pub ref_id: String,
    pub revision: u32,
    pub excerpt: String,
    pub truncated: bool,
    pub token_estimate: usize,
    pub provenance: Value,
    pub original_ref: String,
}

#[derive(Clone)]
pub struct ContextRetriever {
    pub db: Database,
    pub objects: ObjectStore,
}

impl ContextRetriever {
    pub fn new(db: Database, objects: ObjectStore) -> Self {
        Self { db, objects }
    }

    // ---- indexing ----

    pub fn index_document(
        &self,
        ref_id: &str,
        task_id: &str,
        kind: &str,
        title: &str,
        body: &str,
        revision: u32,
    ) -> Result<()> {
        self.db.execute(
            "INSERT OR REPLACE INTO search_docs(ref_id, task_id, kind, title, body,
             revision, updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![
                ref_id,
                task_id,
                kind,
                title,
                body,
                revision,
                crate::storage::event_ledger::now()
            ],
        )?;
        self.db
            .execute("DELETE FROM search_fts WHERE ref_id = ?1", [ref_id])?;
        self.db.execute(
            "INSERT INTO search_fts(title, body, ref_id, task_id, kind) VALUES(?1,?2,?3,?4,?5)",
            params![title, body, ref_id, task_id, kind],
        )?;
        Ok(())
    }

    // ---- context.search ----

    pub fn search(
        &self,
        query: &str,
        task_id: Option<&str>,
        scope: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchCandidate>> {
        let sql = "SELECT d.ref_id, d.kind, d.title, d.revision, d.updated_at,
                   snippet(search_fts, 1, '[', ']', '…', 10) AS snip,
                   bm25(search_fts) AS rank_score,
                   length(d.body) AS blen
                   FROM search_fts JOIN search_docs d ON d.ref_id = search_fts.ref_id
                   WHERE search_fts MATCH ?1
                   AND (?2 IS NULL OR d.task_id = ?2)
                   AND (?3 IS NULL OR d.kind = ?3)
                   ORDER BY rank_score LIMIT ?4";
        match self
            .db
            .query(sql, params![query, task_id, scope, limit as i64])
        {
            Ok(rows) => Ok(rows
                .iter()
                .map(|r| SearchCandidate {
                    ref_id: row_str(r, "ref_id"),
                    kind: row_str(r, "kind"),
                    title: row_str(r, "title"),
                    snippet: row_str(r, "snip"),
                    lexical_score: (-row_f64(r, "rank_score")).max(0.0),
                    revision: row_i64(r, "revision") as u32,
                    token_estimate: (row_i64(r, "blen") as usize / 4).max(1),
                    updated_at: row_str(r, "updated_at"),
                })
                .collect()),
            // Unbalanced quotes / special characters: fall back to a LIKE scan.
            Err(_) => self.like_search(query, task_id, scope, limit),
        }
    }

    fn like_search(
        &self,
        query: &str,
        task_id: Option<&str>,
        scope: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchCandidate>> {
        let rows = self.db.query(
            "SELECT * FROM search_docs
             WHERE (title LIKE ?1 OR body LIKE ?1)
             AND (?2 IS NULL OR task_id = ?2)
             AND (?3 IS NULL OR kind = ?3)
             ORDER BY updated_at DESC LIMIT ?4",
            params![format!("%{query}%"), task_id, scope, limit as i64],
        )?;
        Ok(rows
            .iter()
            .map(|r| {
                let body = row_str(r, "body");
                SearchCandidate {
                    ref_id: row_str(r, "ref_id"),
                    kind: row_str(r, "kind"),
                    title: row_str(r, "title"),
                    snippet: body.chars().take(200).collect(),
                    lexical_score: 0.1,
                    revision: row_i64(r, "revision") as u32,
                    token_estimate: (body.chars().count() / 4).max(1),
                    updated_at: row_str(r, "updated_at"),
                }
            })
            .collect())
    }

    // ---- context.read ----

    pub fn read(
        &self,
        ref_id: &str,
        range_start: Option<usize>,
        max_tokens: usize,
    ) -> Result<ReadPayload> {
        let (text, provenance) = self
            .objects
            .resolve_ref_text(ref_id)?
            .ok_or_else(|| SfaiError::NotFound(format!("unresolvable ref: {ref_id}")))?;
        let parsed = parse_ref(ref_id)?;
        let chars: Vec<char> = text.chars().collect();
        let start = range_start.unwrap_or(0).min(chars.len());
        let take = max_tokens * 4;
        let end = (start + take).min(chars.len());
        let excerpt: String = chars[start..end].iter().collect();
        Ok(ReadPayload {
            ref_id: ref_id.to_string(),
            revision: parsed.revision,
            truncated: chars.len() > end,
            token_estimate: (excerpt.chars().count() / 4).max(1),
            excerpt,
            provenance,
            original_ref: ref_id.to_string(),
        })
    }

    // ---- context.resolve ----

    pub fn resolve(&self, ref_ids: &[String]) -> Result<Vec<Value>> {
        let mut out = Vec::new();
        for ref_id in ref_ids {
            match self.objects.resolve_ref_text(ref_id)? {
                Some((text, provenance)) => out.push(json!({
                    "ref_id": ref_id,
                    "resolved": true,
                    "length": text.chars().count(),
                    "provenance": provenance,
                })),
                None => out.push(json!({"ref_id": ref_id, "resolved": false})),
            }
        }
        Ok(out)
    }
}
