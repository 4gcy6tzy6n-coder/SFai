//! Deterministic scripted model for offline runs and replay (plan M0).
//!
//! A ScriptedModel yields exactly one ActionIntent per step. Scripts can be
//! static (JSON), typed intents, dynamic (callables over the envelope), or
//! crash at a chosen step to simulate a process kill mid-run.

use crate::contracts::{ActionIntent, ContextEnvelope, IntentKind, ModelResponse};
use crate::core::model_adapter::{ModelAdapter, ModelError};
use crate::telemetry::cost::estimate_usage;
use serde_json::{json, Value};

pub enum Step {
    Intent(Box<ActionIntent>),
    Json(Value),
    Dyn(Box<dyn Fn(&ContextEnvelope) -> ActionIntent>),
}

pub struct ScriptedModel {
    steps: Vec<Step>,
    cursor: usize,
    pub calls: u32,
    pub crash_at: Option<u32>,
    model_id: String,
}

impl ScriptedModel {
    pub fn new(steps: Vec<Step>) -> Self {
        Self {
            steps,
            cursor: 0,
            calls: 0,
            crash_at: None,
            model_id: "fake/scripted".into(),
        }
    }

    pub fn with_crash(mut self, at: u32) -> Self {
        self.crash_at = Some(at);
        self
    }
}

impl ModelAdapter for ScriptedModel {
    fn model_id(&self) -> String {
        self.model_id.clone()
    }

    fn decide(&mut self, envelope: &ContextEnvelope) -> Result<ModelResponse, ModelError> {
        self.calls += 1;
        if Some(self.calls) == self.crash_at {
            return Err(ModelError::SimulatedCrash(format!(
                "simulated crash at step {}",
                self.calls
            )));
        }
        if self.cursor >= self.steps.len() {
            return Err(ModelError::Provider(
                "scripted model exhausted: no step left to emit".into(),
            ));
        }
        let step = &self.steps[self.cursor];
        self.cursor += 1;
        let intent = match step {
            Step::Intent(i) => (**i).clone(),
            Step::Json(v) => serde_json::from_value(v.clone())
                .map_err(|e| ModelError::Provider(format!("invalid scripted intent: {e}")))?,
            Step::Dyn(f) => f(envelope),
        };
        let usage = estimate_usage(
            &serde_json::to_string(envelope).unwrap_or_default(),
            &serde_json::to_string(&intent).unwrap_or_default(),
        );
        Ok(ModelResponse {
            model_id: self.model_id.clone(),
            intent,
            usage,
            latency_ms: 1.0,
        })
    }
}

/// Small rule-based model used by the demo scenario and integration tests.
///
/// It *reacts to runtime state* (open issues, open failures, recent results)
/// instead of following a fixed script, so the self-routing loop is genuinely
/// exercised: direct answers skip tools; tool tasks read their own output.
pub struct ReactiveDemoModel {
    pub calls: u32,
    memory: Vec<Value>,
}

impl ReactiveDemoModel {
    pub fn new() -> Self {
        Self {
            calls: 0,
            memory: Vec::new(),
        }
    }

    fn source_refs(envelope: &ContextEnvelope) -> Vec<String> {
        let mut refs: Vec<String> = Vec::new();
        for entry in &envelope.recent_results {
            if let Some(arr) = entry.get("result_refs").and_then(|r| r.as_array()) {
                for v in arr {
                    if let Some(s) = v.as_str() {
                        if s.starts_with("source://") && !refs.iter().any(|r| r == s) {
                            refs.push(s.to_string());
                        }
                    }
                }
            }
        }
        for s in &envelope.active_refs {
            if s.starts_with("source://") && !refs.iter().any(|r| r == s) {
                refs.push(s.clone());
            }
        }
        refs
    }

    fn report_written(&self, envelope: &ContextEnvelope) -> bool {
        if let Some(last) = envelope
            .task_state
            .get("last_material_result")
            .and_then(|v| v.as_str())
        {
            if last.starts_with("artifact://") {
                return true;
            }
        }
        envelope.recent_results.iter().any(|entry| {
            entry
                .get("side_effects")
                .and_then(|s| s.as_array())
                .map(|arr| {
                    arr.iter().any(|v| {
                        v.as_str()
                            .map(|s| s.starts_with("file.written"))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        })
    }

    fn have_evidence(envelope: &ContextEnvelope) -> bool {
        !Self::source_refs(envelope).is_empty()
    }

    fn task_id_of(envelope: &ContextEnvelope) -> String {
        envelope
            .root_index
            .get("task")
            .and_then(|r| r.strip_prefix("task://"))
            .and_then(|r| r.split('/').next())
            .unwrap_or("task_unknown")
            .to_string()
    }

    fn next_intent(&mut self, envelope: &ContextEnvelope) -> ActionIntent {
        let task_id = Self::task_id_of(envelope);
        let open_issues = envelope
            .task_state
            .get("open_issues")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        let open_failure = envelope
            .task_state
            .get("open_failure")
            .filter(|v| !v.is_null());

        if open_issues > 0 {
            return self.write_report(envelope, &task_id, self.memory.len() + 1);
        }
        if self.report_written(envelope) {
            return crate::core::action_intents::intent(
                IntentKind::Finish,
                &task_id,
                "work_done",
                json!({"text": "Report delivered with verifiable citations."}),
                vec![],
                None,
            );
        }
        if let Some(failure) = open_failure {
            return self.handle_failure(envelope, &task_id, failure.clone());
        }
        if Self::have_evidence(envelope) {
            return self.write_report(envelope, &task_id, 1);
        }
        self.search(envelope, &task_id)
    }

    fn search(&self, envelope: &ContextEnvelope, task_id: &str) -> ActionIntent {
        crate::core::action_intents::intent(
            IntentKind::Execute,
            task_id,
            "information_needed",
            json!({"operation_id": "web.search", "arguments": {"query": envelope.goal}}),
            vec![],
            Some(format!("{task_id}:search:{}", self.calls)),
        )
    }

    fn handle_failure(
        &mut self,
        _envelope: &ContextEnvelope,
        task_id: &str,
        failure: Value,
    ) -> ActionIntent {
        self.memory.push(json!({"saw_failure": failure}));
        let retried_before = self
            .memory
            .iter()
            .rev()
            .nth(1)
            .map(|m| m.get("retried").and_then(|r| r.as_bool()).unwrap_or(false))
            .unwrap_or(false);
        if retried_before {
            return crate::core::action_intents::intent(
                IntentKind::Finish,
                task_id,
                "blocked_by_tool_error",
                json!({"text": "Unable to complete: tool keeps failing. See diagnostics."}),
                vec![],
                None,
            );
        }
        self.memory.push(json!({"retried": true}));
        crate::core::action_intents::intent(
            IntentKind::Execute,
            task_id,
            "retry_after_error",
            json!({"operation_id": "web.search", "arguments": {"query": "retry"}}),
            vec![],
            Some(format!("{task_id}:retry-search")),
        )
    }

    fn write_report(
        &self,
        envelope: &ContextEnvelope,
        task_id: &str,
        attempt: usize,
    ) -> ActionIntent {
        let refs = Self::source_refs(envelope);
        let citations: Vec<String> = refs
            .iter()
            .take(3)
            .map(|r| format!("- source: {r}"))
            .collect();
        let text = format!(
            "# Report\n\nGoal: {}\n\n## Findings\n\nEvidence gathered from {} source(s).\n\n## Citations\n\n{}\n",
            envelope.goal,
            refs.len(),
            citations.join("\n")
        );
        crate::core::action_intents::intent(
            IntentKind::Execute,
            task_id,
            "deliver_artifact",
            json!({"operation_id": "file.write", "arguments": {"path": "report.md", "content": text}}),
            vec![],
            Some(format!("{task_id}:write-report:{attempt}")),
        )
    }
}

impl Default for ReactiveDemoModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelAdapter for ReactiveDemoModel {
    fn model_id(&self) -> String {
        "fake/reactive-demo".into()
    }

    fn decide(&mut self, envelope: &ContextEnvelope) -> Result<ModelResponse, ModelError> {
        self.calls += 1;
        let intent = self.next_intent(envelope);
        let usage = estimate_usage(
            &serde_json::to_string(envelope).unwrap_or_default(),
            &serde_json::to_string(&intent).unwrap_or_default(),
        );
        Ok(ModelResponse {
            model_id: "fake/reactive-demo".into(),
            intent,
            usage,
            latency_ms: 1.0,
        })
    }
}
