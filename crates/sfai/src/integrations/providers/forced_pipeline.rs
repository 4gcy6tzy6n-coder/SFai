//! E8 ablation: forced pipeline (self-routing disabled).
//!
//! The wrapper consults the wrapped model exactly as often as a self-routed
//! run would (same information, same call count), then **overrides routing**
//! with a fixed action plan. Content for deliverables is harvested from the
//! model's own suggested intent when available. This isolates the value of
//! *model self-routing* from model capability.

use crate::contracts::{ActionIntent, ContextEnvelope, ModelResponse};
use crate::core::action_intents::intent;
use crate::core::model_adapter::{ModelAdapter, ModelError};
use serde_json::json;

pub struct ForcedPipelineModel {
    inner: Box<dyn ModelAdapter>,
    suite: String,
    acceptance_rules: Vec<String>,
    step: usize,
    /// Content harvested from model suggestions across consults; the write
    /// step consumes the best candidate seen so far.
    content_hint: Option<String>,
}

impl ForcedPipelineModel {
    pub fn new(inner: Box<dyn ModelAdapter>, suite: &str, acceptance_rules: Vec<String>) -> Self {
        Self {
            inner,
            suite: suite.to_string(),
            acceptance_rules,
            step: 0,
            content_hint: None,
        }
    }

    /// Record content candidates from model suggestions: explicit file-write
    /// content ranks above plain response text (finish texts are ignored).
    fn record_content_hint(&mut self, model_intent: &ActionIntent) {
        if let Some(content) = model_intent
            .payload
            .get("arguments")
            .and_then(|a| a.get("content"))
            .and_then(|c| c.as_str())
        {
            if !content.trim().is_empty() {
                self.content_hint = Some(content.to_string());
            }
            return;
        }
        if model_intent.kind == crate::contracts::IntentKind::Respond {
            if let Some(text) = model_intent.payload.get("text").and_then(|t| t.as_str()) {
                if !text.trim().is_empty() && self.content_hint.is_none() {
                    self.content_hint = Some(text.to_string());
                }
            }
        }
    }

    /// Prefer the harvested model content; fall back to refs/template.
    fn take_content(&mut self, envelope: &ContextEnvelope) -> String {
        if let Some(content) = self.content_hint.take() {
            return content;
        }
        let refs: Vec<String> = envelope
            .recent_results
            .iter()
            .flat_map(|r| {
                r.get("result_refs")
                    .and_then(|x| x.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect();
        format!(
            "# Report\n\nEvidence:\n{}\n",
            refs.iter()
                .take(3)
                .map(|r| format!("- source: {r}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    fn next_forced(
        &mut self,
        envelope: &ContextEnvelope,
        model_intent: &ActionIntent,
    ) -> ActionIntent {
        let task_id = envelope
            .root_index
            .get("task")
            .and_then(|r| r.strip_prefix("task://"))
            .and_then(|r| r.split('/').next())
            .unwrap_or("unknown")
            .to_string();
        let key = format!("{task_id}:forced:{}", self.step);
        let _ = model_intent;

        let forced = match (self.suite.as_str(), self.step) {
            // Fixed plans: identical steps for every run.
            ("tool-use", 0) => intent(
                crate::contracts::IntentKind::Execute,
                &task_id,
                "forced_plan",
                json!({"operation_id": "web.search", "arguments": {"query": envelope.goal}}),
                vec![],
                Some(format!("{key}:search")),
            ),
            ("tool-use", 1) => {
                let content = self.take_content(envelope);
                intent(
                    crate::contracts::IntentKind::Execute,
                    &task_id,
                    "forced_plan",
                    json!({"operation_id": "file.write", "arguments": {"path": "report.md", "content": content}}),
                    vec![],
                    Some(format!("{key}:write")),
                )
            }
            ("tool-use", 2) => intent(
                crate::contracts::IntentKind::Verify,
                &task_id,
                "forced_plan",
                json!({"criteria": self.acceptance_rules}),
                vec![],
                None,
            ),
            ("tool-use", _) => intent(
                crate::contracts::IntentKind::Finish,
                &task_id,
                "forced_plan",
                json!({"text": "done"}),
                vec![],
                None,
            ),
            ("dataset", 0) => intent(
                crate::contracts::IntentKind::Execute,
                &task_id,
                "forced_plan",
                json!({"operation_id": "file.read", "arguments": {"path": "sales.csv"}}),
                vec![],
                Some(format!("{key}:read")),
            ),
            ("dataset", 1) => {
                let content = self.take_content(envelope);
                intent(
                    crate::contracts::IntentKind::Execute,
                    &task_id,
                    "forced_plan",
                    json!({"operation_id": "file.write", "arguments": {"path": "answer.json", "content": content}}),
                    vec![],
                    Some(format!("{key}:write")),
                )
            }
            ("dataset", 2) => intent(
                crate::contracts::IntentKind::Verify,
                &task_id,
                "forced_plan",
                json!({"criteria": self.acceptance_rules}),
                vec![],
                None,
            ),
            ("dataset", _) => intent(
                crate::contracts::IntentKind::Finish,
                &task_id,
                "forced_plan",
                json!({"text": "done"}),
                vec![],
                None,
            ),
            // long-horizon: fixed retrieval route.
            ("long-horizon", 0) => intent(
                crate::contracts::IntentKind::Retrieve,
                &task_id,
                "forced_plan",
                json!({"op": "search", "query": envelope.goal, "limit": 3}),
                vec![],
                None,
            ),
            ("long-horizon", 1) => {
                let refs: Vec<String> = envelope
                    .recent_results
                    .last()
                    .and_then(|r| r.get("result_refs"))
                    .and_then(|x| x.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                intent(
                    crate::contracts::IntentKind::Retrieve,
                    &task_id,
                    "forced_plan",
                    json!({"op": "read", "ref_id": refs.first().cloned().unwrap_or_default(), "max_tokens": 200}),
                    vec![],
                    None,
                )
            }
            ("long-horizon", 2) => {
                let content = self.take_content(envelope);
                intent(
                    crate::contracts::IntentKind::Execute,
                    &task_id,
                    "forced_plan",
                    json!({"operation_id": "file.write", "arguments": {"path": "report.md", "content": content}}),
                    vec![],
                    Some(format!("{key}:write")),
                )
            }
            ("long-horizon", 3) => intent(
                crate::contracts::IntentKind::Verify,
                &task_id,
                "forced_plan",
                json!({"criteria": self.acceptance_rules}),
                vec![],
                None,
            ),
            _ => intent(
                crate::contracts::IntentKind::Finish,
                &task_id,
                "forced_plan",
                json!({"text": "done"}),
                vec![],
                None,
            ),
        };
        self.step += 1;
        forced
    }
}

impl ModelAdapter for ForcedPipelineModel {
    fn model_id(&self) -> String {
        format!("{}/forced-pipeline", self.inner.model_id())
    }

    fn decide(&mut self, envelope: &ContextEnvelope) -> Result<ModelResponse, ModelError> {
        // Each plan step consults the model for content. If a consult fails
        // (e.g. an exhausted scripted model in offline runs), routing proceeds
        // with template content — the ablation removes routing freedom, not
        // task completion.
        let inner = self.inner.decide(envelope).unwrap_or(ModelResponse {
            model_id: self.inner.model_id(),
            intent: ActionIntent {
                kind: crate::contracts::IntentKind::Respond,
                task_id: String::new(),
                reason_code: "consult_failed".into(),
                payload: json!({"text": ""}),
                expected_refs: vec![],
                idempotency_key: None,
            },
            usage: Default::default(),
            latency_ms: 0.0,
        });
        self.record_content_hint(&inner.intent);
        let forced = self.next_forced(envelope, &inner.intent);
        Ok(ModelResponse {
            model_id: self.model_id(),
            intent: forced,
            usage: inner.usage,
            latency_ms: inner.latency_ms,
        })
    }
}
