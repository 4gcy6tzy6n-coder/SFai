//! Trace replayer (plan M0): rebuild and check a run from its JSONL trace.
//!
//! Two guarantees are checked here:
//!
//! 1. every trace entry carries a task id and run id (auditability), and
//! 2. recomputing cost totals from the per-step usage entries matches the
//!    final recorded cost summary (no hidden overhead).

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct ReplayReport {
    pub task_ids: Vec<String>,
    pub decisions: u32,
    pub tool_results: u32,
    pub recomputed_input_tokens: u64,
    pub recomputed_output_tokens: u64,
    pub recomputed_cost_usd: f64,
    pub recorded_cost: Option<Value>,
    pub anomalies: Vec<String>,
}

impl ReplayReport {
    pub fn ok(&self) -> bool {
        self.recorded_cost.is_some() && self.anomalies.is_empty()
    }
}

pub fn replay_trace(entries: &[Value]) -> ReplayReport {
    let mut report = ReplayReport::default();
    for entry in entries {
        let Some(task_id) = entry.get("task_id").and_then(|t| t.as_str()) else {
            report.anomalies.push("entry missing task_id".into());
            continue;
        };
        if entry.get("run_id").and_then(|r| r.as_str()).is_none() {
            report.anomalies.push("entry missing run_id".into());
            continue;
        }
        if !report.task_ids.iter().any(|t| t == task_id) {
            report.task_ids.push(task_id.to_string());
        }
        let kind = entry.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        let payload = entry.get("payload").cloned().unwrap_or(Value::Null);
        match kind {
            "decision" => {
                report.decisions += 1;
                let usage = payload.get("usage").cloned().unwrap_or(Value::Null);
                report.recomputed_input_tokens += usage
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                report.recomputed_output_tokens += usage
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                report.recomputed_cost_usd += usage
                    .get("cost_usd")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
            }
            "tool_result" => report.tool_results += 1,
            "cost" => report.recorded_cost = Some(payload),
            _ => {}
        }
    }
    if let Some(recorded) = &report.recorded_cost {
        let recorded_in = recorded
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(u64::MAX);
        if recorded_in != report.recomputed_input_tokens {
            report
                .anomalies
                .push("input token totals do not match per-step usage".into());
        }
        let recorded_out = recorded
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(u64::MAX);
        if recorded_out != report.recomputed_output_tokens {
            report
                .anomalies
                .push("output token totals do not match per-step usage".into());
        }
        let recorded_cost = recorded
            .get("cost_usd")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::MAX);
        if (recorded_cost - report.recomputed_cost_usd).abs() > 1e-6 {
            report
                .anomalies
                .push("cost totals do not match per-step usage".into());
        }
        let recorded_calls = recorded
            .get("model_calls")
            .and_then(|v| v.as_u64())
            .unwrap_or(u64::MAX);
        if recorded_calls != report.decisions as u64 {
            report
                .anomalies
                .push("model call count does not match decisions".into());
        }
    } else {
        report
            .anomalies
            .push("trace has no final cost summary".into());
    }
    report
}
