//! SQLite (WAL) connection + migration runner.
//!
//! Thin stdlib adapter behind a stable interface (ADR-013): persistence
//! internals are reachable only through this module and the stores built on
//! it. `Database` is a cheaply cloneable handle over one shared connection,
//! which is safe because the v0 runtime is single-threaded (sync kernel).

use crate::error::Result;
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use serde_json::{Number, Value};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub const SCHEMA_VERSION: i64 = 2;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS schema_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS tasks (
    task_id TEXT PRIMARY KEY,
    json TEXT NOT NULL,
    version INTEGER NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS events (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    idempotency_key TEXT UNIQUE,
    task_id TEXT NOT NULL,
    actor TEXT NOT NULL,
    event_type TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    payload TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_events_task ON events(task_id, seq);
CREATE TABLE IF NOT EXISTS tool_runs (
    tool_run_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    operation_id TEXT NOT NULL,
    idempotency_key TEXT UNIQUE,
    args_hash TEXT NOT NULL,
    status TEXT NOT NULL,
    result_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sources (
    source_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    origin TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    owner TEXT NOT NULL,
    trust_level TEXT NOT NULL,
    storage_uri TEXT NOT NULL,
    revision INTEGER NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS artifacts (
    artifact_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    path TEXT NOT NULL,
    checksum TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    version INTEGER NOT NULL,
    provenance_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS decisions (
    decision_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    input_context_hash TEXT NOT NULL,
    intent_json TEXT NOT NULL,
    usage_json TEXT NOT NULL,
    latency_ms REAL NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS checkpoints (
    task_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    state_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (task_id, seq)
);
CREATE TABLE IF NOT EXISTS search_docs (
    ref_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    revision INTEGER NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
    title, body, ref_id UNINDEXED, task_id UNINDEXED, kind UNINDEXED,
    tokenize='porter'
);
CREATE TABLE IF NOT EXISTS memory_notes (
    note_id TEXT PRIMARY KEY,
    task_id TEXT,
    kind TEXT NOT NULL,
    topic TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(task_id, kind, topic, title)
);
CREATE TABLE IF NOT EXISTS index_nodes (
    node_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    ref_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    parent TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS wiki_nodes (
    node_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    node_key TEXT NOT NULL,
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    ref_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    provenance_json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    updated_at TEXT NOT NULL,
    UNIQUE(task_id, kind, node_key)
);
CREATE TABLE IF NOT EXISTS wiki_edges (
    edge_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    from_node TEXT NOT NULL,
    to_node TEXT NOT NULL,
    edge_type TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(task_id, from_node, to_node, edge_type)
);
CREATE TABLE IF NOT EXISTS wiki_state (
    task_id TEXT PRIMARY KEY,
    last_compiled_seq INTEGER NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS wiki_compile_log (
    log_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    status TEXT NOT NULL,
    report_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
"#;

pub type RowMap = BTreeMap<String, Value>;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path.as_ref())?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.execute_batch(SCHEMA)?;
        conn.execute(
            "INSERT OR REPLACE INTO schema_meta(key, value) VALUES('version', ?1)",
            [SCHEMA_VERSION],
        )?;
        Ok(())
    }

    pub fn execute<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<usize> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        Ok(conn.execute(sql, params)?)
    }

    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute_batch(sql)?;
        Ok(())
    }

    pub fn query<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<Vec<RowMap>> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(sql)?;
        let columns: Vec<String> = stmt.column_names().iter().map(|c| c.to_string()).collect();
        let mut rows = stmt.query(params)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let mut map = RowMap::new();
            for (idx, name) in columns.iter().enumerate() {
                map.insert(name.clone(), value_ref_to_json(row.get_ref(idx)?));
            }
            out.push(map);
        }
        Ok(out)
    }

    pub fn query_one<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<Option<RowMap>> {
        Ok(self.query(sql, params)?.into_iter().next())
    }

    pub fn last_insert_rowid(&self) -> i64 {
        self.conn
            .lock()
            .expect("db mutex poisoned")
            .last_insert_rowid()
    }

    /// Helper for a single-statement insert with params, returning the row id.
    pub fn insert<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<i64> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute(sql, params)?;
        Ok(conn.last_insert_rowid())
    }
}

fn value_ref_to_json(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::Number(Number::from(i)),
        ValueRef::Real(f) => Value::Number(Number::from_f64(f).unwrap_or_else(|| Number::from(0))),
        ValueRef::Text(t) => Value::String(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Value::String(format!("<blob:{} bytes>", b.len())),
    }
}

pub fn row_str(row: &RowMap, key: &str) -> String {
    match row.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

pub fn row_i64(row: &RowMap, key: &str) -> i64 {
    match row.get(key) {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(Value::String(s)) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

pub fn row_f64(row: &RowMap, key: &str) -> f64 {
    match row.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

pub fn json_param<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_bootstraps_and_fts5_exists() {
        let db = Database::open_in_memory().unwrap();
        let rows = db
            .query("SELECT name FROM sqlite_master WHERE type='table'", [])
            .unwrap();
        let names: Vec<String> = rows.iter().map(|r| row_str(r, "name")).collect();
        for t in [
            "tasks",
            "events",
            "tool_runs",
            "sources",
            "artifacts",
            "search_docs",
        ] {
            assert!(names.contains(&t.to_string()), "missing table {t}");
        }
        // FTS5 must be compiled in (plan §2 retrieval v0).
        db.execute(
            "INSERT INTO search_fts(title, body, ref_id, task_id, kind) VALUES('t','b','r','k','source')",
            [],
        )
        .unwrap();
        let hits = db
            .query(
                "SELECT ref_id FROM search_fts WHERE search_fts MATCH 'b'",
                [],
            )
            .unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn row_helpers() {
        let db = Database::open_in_memory().unwrap();
        db.execute("CREATE TABLE x (name TEXT, n INTEGER)", [])
            .unwrap();
        db.execute("INSERT INTO x VALUES('a', 7)", []).unwrap();
        let row = db.query_one("SELECT * FROM x", []).unwrap().unwrap();
        assert_eq!(row_str(&row, "name"), "a");
        assert_eq!(row_i64(&row, "n"), 7);
    }
}
