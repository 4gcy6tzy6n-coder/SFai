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

/// Interactive conversational model used by the CLI chat box when no external
/// `SFAI_MODEL_BASE_URL` endpoint is configured.
///
/// Routes through the real `DecisionKernel`:
/// - research / search / report prompts trigger `web.search` -> `file.write` -> `finish`;
/// - context / history / prior-source prompts trigger `context.search` -> `context.read` -> `respond`;
/// - conversational prompts use Direct-Answer Mode (`IntentKind::Respond` with zero tool calls).
pub struct ConversationalDemoModel {
    pub calls: u32,
    reactive: ReactiveDemoModel,
}

impl ConversationalDemoModel {
    pub fn new() -> Self {
        Self {
            calls: 0,
            reactive: ReactiveDemoModel::new(),
        }
    }

    fn is_file_edit_goal(goal: &str) -> Option<String> {
        let lower = goal.to_lowercase();
        let is_edit = [
            "修改",
            "编辑",
            "更新",
            "写入",
            "创建文件",
            "edit ",
            "update ",
            "modify ",
        ]
        .iter()
        .any(|kw| lower.contains(kw));
        if !is_edit {
            return None;
        }
        for tok in goal.split_whitespace() {
            let clean = tok.trim_matches(|c: char| ",.;:()\"'`".contains(c));
            if clean.ends_with(".md")
                || clean.ends_with(".rs")
                || clean.ends_with(".py")
                || clean.ends_with(".json")
                || clean.ends_with(".txt")
            {
                return Some(clean.to_string());
            }
        }
        Some("report.md".to_string())
    }

    fn is_research_goal(goal: &str) -> bool {
        let lower = goal.to_lowercase();
        [
            "search", "find", "source", "report", "paper", "write", "研究", "搜索", "查找", "报告",
            "文献",
        ]
        .iter()
        .any(|kw| lower.contains(kw))
    }

    fn is_context_lookup(goal: &str) -> bool {
        let lower = goal.to_lowercase();
        [
            "context",
            "earlier",
            "prior",
            "history",
            "quote",
            "retrieve",
            "之前",
            "历史",
            "上下文",
            "检索",
            "引用",
        ]
        .iter()
        .any(|kw| lower.contains(kw))
    }

    fn build_direct_reply(envelope: &ContextEnvelope) -> String {
        let goal = envelope.goal.trim();
        let lower = goal.to_lowercase();
        if lower.contains("2 + 2") || lower.contains("2+2") {
            return "4".to_string();
        }
        let active_refs = envelope.active_refs.len();
        let pinned = envelope.pinned_constraints.len();
        let has_cjk = goal.chars().any(|c| (c as u32) >= 0x4e00);
        if has_cjk {
            let mut lines = vec![
                format!("收到你的消息：**\"{goal}\"**"),
                String::new(),
                "当前通过 **sfai 自路由决策内核 (Direct-Answer Mode)** 直接响应（0 次工具发现开销）：".to_string(),
                format!(
                    "- **工作集状态**：`{active_refs}` 个活动引用 (`active_refs`)，`{pinned}` 条常驻硬约束 (`pinned_constraints`)"
                ),
                "- **可用交互**：你可以直接让我 **\"搜索并撰写一份关于 sparse context 的研究报告\"**、**\"检索之前的证据来源\"**，或输入 `/help` 查看斜杠命令。".to_string(),
            ];
            if !envelope.pinned_constraints.is_empty() {
                lines.push(format!(
                    "- **已钉住约束**：{}",
                    envelope.pinned_constraints.join("；")
                ));
            }
            lines.join("\n")
        } else {
            let mut lines = vec![
                format!("Received: **\"{goal}\"**"),
                String::new(),
                "Answered via **sfai Self-Routing Decision Kernel (Direct-Answer Mode)** with zero tool discovery calls:".to_string(),
                format!(
                    "- **Working set**: `{active_refs}` active ref(s), `{pinned}` pinned constraint(s)"
                ),
                "- **Next steps**: Ask me to *\"Find recent sources on sparse context and write a report\"*, *\"Retrieve prior context sources\"*, or type `/help`.".to_string(),
            ];
            if !envelope.pinned_constraints.is_empty() {
                lines.push(format!(
                    "- **Pinned constraints**: {}",
                    envelope.pinned_constraints.join("; ")
                ));
            }
            lines.join("\n")
        }
    }
}

impl Default for ConversationalDemoModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelAdapter for ConversationalDemoModel {
    fn model_id(&self) -> String {
        "sfai/local-assistant".into()
    }

    fn decide(&mut self, envelope: &ContextEnvelope) -> Result<ModelResponse, ModelError> {
        self.calls += 1;
        let task_id = ReactiveDemoModel::task_id_of(envelope);
        let intent = if let Some(target_path) = Self::is_file_edit_goal(&envelope.goal) {
            if self.calls == 1 {
                let updated = format!(
                    "# Report\n\nGoal: {}\n\n## Findings\n\nEvidence gathered and refined.\n\n## Conclusion\n\nUpdated per user instruction: {}\n",
                    envelope.goal, envelope.goal
                );
                crate::core::action_intents::intent(
                    IntentKind::Execute,
                    &task_id,
                    "modify_file",
                    json!({"operation_id": "file.write", "arguments": {"path": target_path, "content": updated}}),
                    vec![],
                    Some(format!("{task_id}:edit:1")),
                )
            } else {
                crate::core::action_intents::intent(
                    IntentKind::Finish,
                    &task_id,
                    "file_updated",
                    json!({"text": format!("Updated `{target_path}` and applied requested modifications.")}),
                    vec![],
                    None,
                )
            }
        } else if Self::is_research_goal(&envelope.goal) {
            self.reactive.next_intent(envelope)
        } else if Self::is_context_lookup(&envelope.goal) {
            if self.calls == 1 {
                crate::core::action_intents::intent(
                    IntentKind::Retrieve,
                    &task_id,
                    "information_needed",
                    json!({"op": "search", "query": "sparse context", "task_scope": "all", "limit": 5}),
                    vec![],
                    None,
                )
            } else {
                let refs = ReactiveDemoModel::source_refs(envelope);
                let cite = if refs.is_empty() {
                    "No prior indexed sources found yet (try running a research query first)."
                        .to_string()
                } else {
                    format!(
                        "Retrieved {} indexed source(s) from context store:\n{}",
                        refs.len(),
                        refs.iter()
                            .take(3)
                            .map(|r| format!("- {r}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                };
                crate::core::action_intents::intent(
                    IntentKind::Respond,
                    &task_id,
                    "direct_answer",
                    json!({"text": cite}),
                    vec![],
                    None,
                )
            }
        } else {
            crate::core::action_intents::intent(
                IntentKind::Respond,
                &task_id,
                "direct_answer",
                json!({"text": Self::build_direct_reply(envelope)}),
                vec![],
                None,
            )
        };
        let usage = estimate_usage(
            &serde_json::to_string(envelope).unwrap_or_default(),
            &serde_json::to_string(&intent).unwrap_or_default(),
        );
        Ok(ModelResponse {
            model_id: self.model_id(),
            intent,
            usage,
            latency_ms: 1.0,
        })
    }
}
