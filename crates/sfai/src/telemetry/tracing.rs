//! Tracing: JSONL event logs per run (plan §2 tracing row).
//!
//! Every decision, tool result, verification and cost summary lands in one
//! append-only trace file so runs are reproducible and auditable.

use crate::error::Result;
use crate::ids::new_id;
use crate::storage::event_ledger::now;
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Tracer {
    pub trace_dir: PathBuf,
    pub run_id: String,
    pub path: PathBuf,
}

impl Tracer {
    pub fn new(trace_dir: impl AsRef<Path>) -> Result<Self> {
        let trace_dir = trace_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&trace_dir)?;
        let run_id = new_id("trace");
        let path = trace_dir.join(format!("{run_id}.jsonl"));
        Ok(Self {
            trace_dir,
            run_id,
            path,
        })
    }

    pub fn record(&self, task_id: &str, kind: &str, payload: Value) -> Result<()> {
        let entry = json!({
            "ts": now(),
            "run_id": self.run_id,
            "task_id": task_id,
            "kind": kind,
            "payload": payload,
        });
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{}", serde_json::to_string(&entry)?)?;
        Ok(())
    }
}

/// Collect all trace entries for a task across run files, ordered by time.
pub fn export_trace(trace_dir: impl AsRef<Path>, task_id: &str) -> Result<Vec<Value>> {
    let mut out: Vec<Value> = Vec::new();
    for entry in std::fs::read_dir(trace_dir.as_ref())? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let content = std::fs::read_to_string(&path)?;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(line)?;
            if value.get("task_id").and_then(|t| t.as_str()) == Some(task_id) {
                out.push(value);
            }
        }
    }
    out.sort_by(|a, b| {
        a.get("ts")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .cmp(b.get("ts").and_then(|t| t.as_str()).unwrap_or(""))
    });
    Ok(out)
}
