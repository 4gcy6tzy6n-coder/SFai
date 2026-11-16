//! Multi-layer model memory (aligned with Claude Code's memory subsystem).
//!
//! Layers (see ADR-022):
//! 1. **project** — persistent project instructions (CLAUDE.md-style), always
//!    available in the envelope;
//! 2. **topic** — cross-task knowledge notes with `MEMORY.md`-style index
//!    (agent-writable, revisioned, searchable);
//! 3. **task_summary** — auto-memory written at task completion (goal, outcome,
//!    outputs, lessons);
//! 4. **log** — episodic run logs.
//!
//! The Wiki (M4) remains the *derived structured* layer; this store is the
//! agent-facing *memory* layer. Notes are plain markdown, indexed into FTS as
//! `kind='memory'` so `context.search` finds them, and the top-k relevant notes
//! ride into the envelope (bounded, Claude-Code-style progressive disclosure).

use crate::error::Result;
use crate::ids::new_id;
use crate::storage::db::{row_i64, row_str, Database};
use crate::storage::event_ledger::now;
use rusqlite::params;
use serde_json::{json, Value};

pub const MEMORY_ENVELOPE_LIMIT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    Project,
    Topic,
    TaskSummary,
    Log,
    /// The Way layer: per-task method, wins, and — emphasized — mistakes.
    /// Written by the agent itself (auto-extracted and/or model-authored).
    Way,
}

impl MemoryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryKind::Project => "project",
            MemoryKind::Topic => "topic",
            MemoryKind::TaskSummary => "task_summary",
            MemoryKind::Log => "log",
            MemoryKind::Way => "way",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryNote {
    pub note_id: String,
    pub task_id: Option<String>,
    pub kind: String,
    pub topic: String,
    pub title: String,
    pub body: String,
    pub revision: u32,
}

impl MemoryNote {
    pub fn to_value(&self) -> Value {
        json!({
            "note_id": self.note_id,
            "kind": self.kind,
            "topic": self.topic,
            "title": self.title,
            "body": self.body.chars().take(1200).collect::<String>(),
            "revision": self.revision,
        })
    }
}

pub struct MemoryStore {
    pub db: Database,
}

impl MemoryStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Write (or revise) a memory note. Same (task, kind, topic, title) bumps
    /// the revision instead of duplicating — MEMORY.md-style maintenance.
    pub fn write_note(
        &self,
        kind: MemoryKind,
        task_id: Option<&str>,
        topic: &str,
        title: &str,
        body: &str,
    ) -> Result<MemoryNote> {
        let ts = now();
        let existing = self.db.query_one(
            "SELECT * FROM memory_notes WHERE COALESCE(task_id,'') = ?1 AND kind = ?2 AND topic = ?3 AND title = ?4",
            params![task_id.unwrap_or(""), kind.as_str(), topic, title],
        )?;
        if let Some(existing) = existing {
            let note_id = row_str(&existing, "note_id");
            let revision = row_i64(&existing, "revision") as u32 + 1;
            self.db.execute(
                "UPDATE memory_notes SET body = ?1, revision = ?2, updated_at = ?3 WHERE note_id = ?4",
                params![body, revision, ts, note_id],
            )?;
            self.index_note(&note_id, task_id, topic, title, body, revision)?;
            return Ok(MemoryNote {
                note_id,
                task_id: task_id.map(|s| s.to_string()),
                kind: kind.as_str().to_string(),
                topic: topic.to_string(),
                title: title.to_string(),
                body: body.to_string(),
                revision,
            });
        }
        let note_id = new_id("mem");
        self.db.execute(
            "INSERT INTO memory_notes(note_id, task_id, kind, topic, title, body, revision, created_at, updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,1,?7,?7)",
            params![note_id, task_id, kind.as_str(), topic, title, body, ts],
        )?;
        self.index_note(&note_id, task_id, topic, title, body, 1)?;
        Ok(MemoryNote {
            note_id,
            task_id: task_id.map(|s| s.to_string()),
            kind: kind.as_str().to_string(),
            topic: topic.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            revision: 1,
        })
    }

    fn index_note(
        &self,
        note_id: &str,
        task_id: Option<&str>,
        topic: &str,
        title: &str,
        body: &str,
        revision: u32,
    ) -> Result<()> {
        let ref_id = format!("memory://{note_id}@{revision}");
        self.db.execute(
            "DELETE FROM search_fts WHERE ref_id LIKE ?1",
            params![format!("memory://{note_id}@%")],
        )?;
        self.db.execute(
            "INSERT OR REPLACE INTO search_docs(ref_id, task_id, kind, title, body, revision, updated_at)
             VALUES(?1,?2,'memory',?3,?4,?5,?6)",
            params![ref_id, task_id.unwrap_or(""), title, format!("{topic}: {body}"), revision, now()],
        )?;
        self.db.execute(
            "INSERT INTO search_fts(title, body, ref_id, task_id, kind) VALUES(?1,?2,?3,?4,'memory')",
            params![title, format!("{topic}: {body}"), ref_id, task_id.unwrap_or("")],
        )?;
        Ok(())
    }

    /// Top-k notes relevant to `query`, most recent first on ties.
    pub fn relevant(
        &self,
        query: &str,
        task_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryNote>> {
        // OR-joined quoted terms: FTS5 barewords AND by default, which would
        // zero out recall for natural-language goal strings.
        let terms: Vec<String> = query
            .split_whitespace()
            .filter(|t| t.chars().count() > 2)
            .take(12)
            .map(|t| format!("\"{}\"", t.replace('"', "")))
            .collect();
        let fts_query = if terms.is_empty() {
            query.to_string()
        } else {
            terms.join(" OR ")
        };
        let rows = self.db.query(
            "SELECT n.*, bm25(search_fts) AS rank_score FROM search_fts
             JOIN memory_notes n ON search_fts.ref_id LIKE 'memory://' || n.note_id || '@%'
             WHERE search_fts MATCH ?1
             AND (?2 = '' OR n.task_id IS NULL OR n.task_id = ?2)
             ORDER BY rank_score LIMIT ?3",
            params![fts_query, task_id.unwrap_or(""), limit as i64],
        )?;
        rows.iter().map(row_to_note).collect()
    }

    pub fn list_notes(&self, kind: Option<&str>, limit: usize) -> Result<Vec<MemoryNote>> {
        let rows = self.db.query(
            "SELECT * FROM memory_notes WHERE (?1 = '' OR kind = ?1) ORDER BY updated_at DESC LIMIT ?2",
            params![kind.unwrap_or(""), limit as i64],
        )?;
        rows.iter().map(row_to_note).collect()
    }

    /// MEMORY.md-style index (topic list, bounded).
    pub fn index_markdown(&self) -> Result<String> {
        let notes = self.list_notes(None, 200)?;
        let mut out = String::from("# Memory index (MEMORY.md)\n\n");
        for note in notes.iter().take(200) {
            out.push_str(&format!(
                "- [{}] {} — {} (rev {})\n",
                note.kind, note.topic, note.title, note.revision
            ));
        }
        Ok(out)
    }

    /// Write a structured **Way** entry: how the task was done, what went
    /// well, what went wrong (the emphasis), and the distilled lesson.
    pub fn write_way(
        &self,
        task_id: &str,
        method: &str,
        wins: &[String],
        mistakes: &[String],
        lesson: &str,
    ) -> Result<MemoryNote> {
        let body = format!(
            "way-of-working for {task_id}\n## method\n{method}\n## wins (what went well)\n{}\n## mistakes (what went wrong — remember these)\n{}\n## lesson\n{lesson}",
            wins.iter().map(|w| format!("- {w}")).collect::<Vec<_>>().join("\n"),
            mistakes
                .iter()
                .map(|m| format!("- {m}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        self.write_note(
            MemoryKind::Way,
            Some(task_id),
            "way",
            &format!("way of {task_id}"),
            &body,
        )
    }

    /// Auto-extract a Way entry from run telemetry: mistakes come from the
    /// error/loop/acceptance history, wins from outputs and clean runs.
    /// Distinct identity from model-authored `write_way` entries so a model's
    /// own reflection is never overwritten by the auto-extraction.
    pub fn auto_way_from_run(
        &self,
        task_id: &str,
        method: &str,
        wins: &[String],
        mistakes: &[String],
        lesson: &str,
    ) -> Result<MemoryNote> {
        let body = format!(
            "way-of-working for {task_id}\n## method\n{method}\n## wins (what went well)\n{}\n## mistakes (what went wrong — remember these)\n{}\n## lesson\n{lesson}",
            wins.iter().map(|w| format!("- {w}")).collect::<Vec<_>>().join("\n"),
            mistakes
                .iter()
                .map(|m| format!("- {m}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        self.write_note(
            MemoryKind::Way,
            Some(task_id),
            "way",
            &format!("way of {task_id} (auto)"),
            &body,
        )
    }

    /// Way notes relevant to a goal — retrieval path: index → wiki → way.
    pub fn way_relevant(&self, query: &str, limit: usize) -> Result<Vec<MemoryNote>> {
        let mut notes = self.relevant(query, None, limit * 2)?;
        notes.retain(|n| n.kind == "way");
        notes.truncate(limit);
        Ok(notes)
    }

    /// Auto-memory: summarize a finished task into a task_summary note
    /// (Claude Code auto-memory analogue).
    pub fn auto_summarize_task(
        &self,
        task_id: &str,
        goal: &str,
        status: &str,
        outputs: &[String],
        issues: usize,
    ) -> Result<MemoryNote> {
        let body = format!(
            "task: {task_id}\ngoal: {goal}\nstatus: {status}\noutputs: {}\nopen_issues: {issues}",
            outputs.join(", ")
        );
        self.write_note(
            MemoryKind::TaskSummary,
            Some(task_id),
            "task-history",
            &format!("task {task_id}"),
            &body,
        )
    }
}

fn row_to_note(row: &crate::storage::db::RowMap) -> Result<MemoryNote> {
    Ok(MemoryNote {
        note_id: row_str(row, "note_id"),
        task_id: {
            let t = row_str(row, "task_id");
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        },
        kind: row_str(row, "kind"),
        topic: row_str(row, "topic"),
        title: row_str(row, "title"),
        body: row_str(row, "body"),
        revision: row_i64(row, "revision") as u32,
    })
}
