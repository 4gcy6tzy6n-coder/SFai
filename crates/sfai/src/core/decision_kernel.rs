//! **Decide-Act** — the self-routing core of sfai (plan §4).
//!
//! *Decide-Act* is the project's core loop and name: each **decide** step, the
//! main model chooses exactly one action intent (answer / discover / retrieve /
//! activate / execute / verify / ask / pause / finish); each **act** step, the
//! runtime validates the request and executes it through policy, broker and
//! stores. The loop is model-native self-routing — not a fixed pipeline.
//!
//! Design rule: the main model is the policy for choosing *which action to
//! request*; the runtime is the authority for deciding whether and how it runs.
//!
//! The loop is deliberately not a Planner → Executor → Reviewer workflow. The
//! model may answer directly with zero discovery calls, discover capabilities,
//! read context by address, execute through the broker, verify, ask the user,
//! pause, or finish — in any useful order.

use crate::authority::tasks::TaskStore;
use crate::capabilities::registry::CapabilityRegistry;
use crate::capabilities::skills::{SkillActivation, SkillRegistry};
use crate::context::retrieval::ContextRetriever;
use crate::contracts::{
    ActionIntent, ContextEnvelope, IntentKind, ModelResponse, Task, TaskState, ToolResult,
};
use crate::core::context_envelope::EnvelopeBuilder;
use crate::core::model_adapter::ModelAdapter;
use crate::core::termination::{AcceptanceAssessment, AcceptanceEvaluator};
use crate::error::{Result, SfaiError};
use crate::ids::new_decision_id;
use crate::runtime::broker::ToolBroker;
use crate::storage::event_ledger::EventLedger;
use crate::telemetry::cost::CostLedger;
use crate::telemetry::tracing::Tracer;
use crate::verify::engine::VerificationEngine;
use rusqlite::params;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const MAX_STEPS: usize = 50;
pub const MAX_CONSECUTIVE_TOOL_ERRORS: u32 = 3;
pub const MAX_ACCEPTANCE_RETRIES: u32 = 2;

/// Lifecycle observer for streaming CLI / UI progress without coupling the
/// decision kernel to any presentation layer.
pub trait KernelObserver {
    fn on_run_start(&mut self, _task: &Task, _state: &TaskState) {}
    fn on_step_start(&mut self, _step: usize, _envelope: &ContextEnvelope) {}
    fn on_decision(&mut self, _step: usize, _response: &ModelResponse) {}
    fn on_tool_result(&mut self, _step: usize, _intent: &ActionIntent, _result: &ToolResult) {}
    fn on_acceptance_blocked(
        &mut self,
        _step: usize,
        _issues: &[Value],
        _retry: u32,
        _max_retries: u32,
    ) {
    }
    fn on_run_end(&mut self, _result: &TaskRunResult) {}
}

pub struct NoopObserver;
impl KernelObserver for NoopObserver {}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskRunResult {
    pub task_id: String,
    pub status: crate::contracts::TaskStatus,
    pub answer: Option<String>,
    pub steps: usize,
    pub cost: Value,
    pub open_issues: Vec<Value>,
    /// Routing-quality report (plan §13.4): what the model routed to and how.
    pub routing: Value,
}

/// Per-run routing statistics — the router block's own telemetry.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct RoutingStats {
    pub intent_kinds: BTreeMap<String, u32>,
    pub tool_calls: u32,
    /// Idempotent replays (duplicate intent safely absorbed).
    pub duplicate_replays: u32,
    /// Intents refused because the same action repeated without state change.
    pub loop_guard_trips: u32,
    /// Requests routed to unknown operations (misroutes).
    pub misroutes: u32,
    pub ask_user: u32,
    pub skill_activations: u32,
    pub discover_calls: u32,
}

impl RoutingStats {
    fn as_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

impl TaskRunResult {
    pub fn completed(task_id: &str, answer: Option<String>) -> Self {
        Self {
            task_id: task_id.to_string(),
            status: crate::contracts::TaskStatus::Completed,
            answer,
            steps: 0,
            cost: Value::Null,
            open_issues: Vec::new(),
            routing: Value::Null,
        }
    }

    pub fn blocked(task_id: &str) -> Self {
        Self {
            task_id: task_id.to_string(),
            status: crate::contracts::TaskStatus::Blocked,
            answer: None,
            steps: 0,
            cost: Value::Null,
            open_issues: Vec::new(),
            routing: Value::Null,
        }
    }
}

/// Canonical alias: the Decide-Act kernel.
pub type DecideActKernel<'a> = DecisionKernel<'a>;

pub struct DecisionKernel<'a> {
    pub model: &'a mut dyn ModelAdapter,
    pub broker: &'a ToolBroker,
    pub task_store: &'a TaskStore,
    pub ledger: &'a EventLedger,
    pub envelope_builder: &'a EnvelopeBuilder,
    pub acceptance: &'a AcceptanceEvaluator,
    pub verifier: &'a VerificationEngine,
    pub retriever: &'a ContextRetriever,
    pub registry: &'a CapabilityRegistry,
    pub skills: &'a SkillRegistry,
    pub tracer: &'a Tracer,
    cost: CostLedger,
    activated_skills: BTreeMap<String, SkillActivation>,
    cancel_flag: Option<Arc<AtomicBool>>,
    enforce_acceptance: bool,
    routing: RoutingStats,
    /// Recent dispatched action signatures (kind + payload hash) for loop detection.
    action_history: Vec<String>,
}

impl<'a> DecisionKernel<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model: &'a mut dyn ModelAdapter,
        broker: &'a ToolBroker,
        task_store: &'a TaskStore,
        ledger: &'a EventLedger,
        envelope_builder: &'a EnvelopeBuilder,
        acceptance: &'a AcceptanceEvaluator,
        verifier: &'a VerificationEngine,
        retriever: &'a ContextRetriever,
        registry: &'a CapabilityRegistry,
        skills: &'a SkillRegistry,
        tracer: &'a Tracer,
    ) -> Self {
        Self {
            model,
            broker,
            task_store,
            ledger,
            envelope_builder,
            acceptance,
            verifier,
            retriever,
            registry,
            skills,
            tracer,
            cost: CostLedger::new(),
            activated_skills: BTreeMap::new(),
            cancel_flag: None,
            enforce_acceptance: true,
            routing: RoutingStats::default(),
            action_history: Vec::new(),
        }
    }

    /// Cooperative cancellation: checked at the top of every step.
    pub fn with_cancel_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.cancel_flag = Some(flag);
        self
    }

    /// Ablation switch: skip acceptance and mandatory-verification gates.
    pub fn with_acceptance_enforcement(mut self, enabled: bool) -> Self {
        self.enforce_acceptance = enabled;
        self
    }

    pub fn run_task(&mut self, task_id: &str, max_steps: usize) -> Result<TaskRunResult> {
        let mut noop = NoopObserver;
        self.run_task_observed(task_id, max_steps, &mut noop)
    }

    pub fn run_task_observed(
        &mut self,
        task_id: &str,
        max_steps: usize,
        observer: &mut dyn KernelObserver,
    ) -> Result<TaskRunResult> {
        let mut task = self.task_store.load(task_id)?;
        if task.status.is_terminal() {
            let result = self.finish_result(task_id, task.status.as_str());
            observer.on_run_end(&result);
            return Ok(result);
        }
        if task.status != crate::contracts::TaskStatus::Active {
            self.task_store.set_status(
                task_id,
                crate::contracts::TaskStatus::Active,
                "run started",
            )?;
        }
        task.status = crate::contracts::TaskStatus::Active;
        let initial_state = self.task_store.state(task_id)?;
        observer.on_run_start(&task, &initial_state);

        let mut steps = 0usize;
        let mut consecutive_tool_errors = 0u32;
        let mut acceptance_retries = 0u32;

        while steps < max_steps {
            if let Some(flag) = &self.cancel_flag {
                if flag.load(Ordering::Relaxed) {
                    self.task_store.set_status(
                        task_id,
                        crate::contracts::TaskStatus::Cancelled,
                        "cancelled by user",
                    )?;
                    let mut result = self.finish_result(task_id, "cancelled");
                    result.steps = steps;
                    result.cost = self.cost_value();
                    self.tracer
                        .record(task_id, "final", json!({"status": "cancelled"}))?;
                    observer.on_run_end(&result);
                    return Ok(result);
                }
            }
            steps += 1;
            let state = self.task_store.state(task_id)?;
            let active_skills: Vec<SkillActivation> =
                self.activated_skills.values().cloned().collect();
            let envelope = self.envelope_builder.build(&task, &state, &active_skills)?;
            observer.on_step_start(steps, &envelope);
            let response = match self.decide(task_id, &envelope) {
                Ok(r) => r,
                Err(exc) => {
                    let _ = self.tracer.record(
                        task_id,
                        "model_error",
                        json!({"error": exc.to_string()}),
                    );
                    return Err(exc);
                }
            };
            let mut intent = response.intent.clone();
            // Runtime authority: the model cannot target another task.
            intent.task_id = task_id.to_string();
            *self
                .routing
                .intent_kinds
                .entry(intent.kind.as_str().to_string())
                .or_insert(0) += 1;
            if intent.kind == IntentKind::AskUser {
                self.routing.ask_user += 1;
            }
            if intent.kind == IntentKind::Discover {
                self.routing.discover_calls += 1;
            }
            if intent.payload.get("skill_id").is_some() {
                self.routing.skill_activations += 1;
            }
            // Runtime loop guard (plan §4): the same action repeated while no
            // state change occurred is refused instead of executed again.
            let action_sig = if matches!(
                intent.kind,
                IntentKind::Execute | IntentKind::Retrieve | IntentKind::Verify
            ) {
                Some(crate::ids::canonical_hash(&serde_json::json!([
                    intent.kind.as_str(),
                    intent.payload
                ])))
            } else {
                None
            };
            if let Some(sig) = &action_sig {
                let repeated = self.action_history.iter().rev().take(2).all(|h| h == sig)
                    && self.action_history.len() >= 2;
                if repeated {
                    self.routing.loop_guard_trips += 1;
                    // The model still made a decision; keep the audit trail exact.
                    self.task_store.record_decision_started(
                        task_id,
                        intent.kind.as_str(),
                        &intent.reason_code,
                    )?;
                    let guarded = crate::contracts::ToolResult {
                        status: crate::contracts::ToolStatus::Denied,
                        operation_id: Some(format!("{}.*", intent.kind.as_str())),
                        error_code: Some("LOOP_DETECTED".into()),
                        diagnostic: Some(json!({
                            "verdict": "fail",
                            "error_code": "LOOP_DETECTED",
                            "repair_hint": "The same action repeated 3x without state change;                                             choose a different action or report blocked.",
                        })),
                        ..Default::default()
                    };
                    self.tracer.record(
                        task_id,
                        "loop_guard",
                        json!({"kind": intent.kind.as_str(), "occurrence": 3}),
                    )?;
                    self.apply_result(task_id, &intent, &guarded)?;
                    continue;
                }
            }
            let mut observed_response = response.clone();
            observed_response.intent = intent.clone();
            observer.on_decision(steps, &observed_response);
            self.task_store.record_decision_started(
                task_id,
                intent.kind.as_str(),
                &intent.reason_code,
            )?;
            self.tracer.record(
                task_id,
                "decision",
                json!({
                    "step": steps,
                    "kind": intent.kind.as_str(),
                    "reason_code": intent.reason_code,
                    "model_id": response.model_id,
                    "usage": response.usage,
                }),
            )?;

            match intent.kind {
                IntentKind::Respond | IntentKind::Finish => {
                    let (done, result) = self.try_complete(task_id, &task, &intent)?;
                    if done {
                        let mut result = result.unwrap();
                        result.steps = steps;
                        result.cost = self.cost_value();
                        result.routing = self.routing.as_value();
                        self.tracer
                            .record(task_id, "routing", self.routing.as_value())?;
                        observer.on_run_end(&result);
                        return Ok(result);
                    }
                    acceptance_retries += 1;
                    consecutive_tool_errors = 0;
                    let open_issues = self.task_store.state(task_id)?.open_issues;
                    observer.on_acceptance_blocked(
                        steps,
                        &open_issues,
                        acceptance_retries,
                        MAX_ACCEPTANCE_RETRIES,
                    );
                    if acceptance_retries > MAX_ACCEPTANCE_RETRIES {
                        self.task_store.set_status(
                            task_id,
                            crate::contracts::TaskStatus::Blocked,
                            "acceptance criteria still failing",
                        )?;
                        let mut result = TaskRunResult {
                            routing: self.routing.as_value(),
                            ..TaskRunResult::blocked(task_id)
                        };
                        result.answer = intent
                            .payload
                            .get("text")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string());
                        result.open_issues = open_issues;
                        result.steps = steps;
                        result.cost = self.cost_value();
                        self.tracer
                            .record(task_id, "final", json!({"status": "blocked"}))?;
                        observer.on_run_end(&result);
                        return Ok(result);
                    }
                    continue;
                }
                IntentKind::Pause => {
                    self.task_store.set_status(
                        task_id,
                        crate::contracts::TaskStatus::Paused,
                        &intent.reason_code,
                    )?;
                    let mut result = self.finish_result(task_id, "paused");
                    result.steps = steps;
                    result.cost = self.cost_value();
                    observer.on_run_end(&result);
                    return Ok(result);
                }
                IntentKind::AskUser => {
                    self.task_store.apply(
                        task_id,
                        &[json!({
                            "type": "issue_recorded",
                            "issue": {
                                "key": format!("ask_user:{steps}"),
                                "question": intent.payload.get("question").and_then(|q| q.as_str()).unwrap_or(""),
                            },
                        })],
                    )?;
                    self.task_store.set_status(
                        task_id,
                        crate::contracts::TaskStatus::WaitingApproval,
                        &intent.reason_code,
                    )?;
                    let mut result = self.finish_result(task_id, "waiting_approval");
                    result.open_issues = self.task_store.state(task_id)?.open_issues;
                    result.steps = steps;
                    result.cost = self.cost_value();
                    observer.on_run_end(&result);
                    return Ok(result);
                }
                _ => {
                    let tool_result = self.dispatch(task_id, &intent, &state)?;
                    self.apply_result(task_id, &intent, &tool_result)?;
                    // Loop history tracks *successful* actions only: a failing
                    // action is retried under the consecutive-error bound, and a
                    // successful-but-unproductive repetition is what the loop
                    // guard exists to stop.
                    if tool_result.status == crate::contracts::ToolStatus::Ok {
                        if let Some(sig) = &action_sig {
                            self.action_history.push(sig.clone());
                            if self.action_history.len() > 8 {
                                self.action_history.remove(0);
                            }
                        }
                    }
                    observer.on_tool_result(steps, &intent, &tool_result);
                    if tool_result.status == crate::contracts::ToolStatus::Error {
                        consecutive_tool_errors += 1;
                        if consecutive_tool_errors >= MAX_CONSECUTIVE_TOOL_ERRORS {
                            // Bounded retry: visible blocked state, never silent success.
                            self.task_store.set_status(
                                task_id,
                                crate::contracts::TaskStatus::Blocked,
                                "tool errors exceeded bounded retry",
                            )?;
                            let mut result = TaskRunResult {
                                routing: self.routing.as_value(),
                                ..TaskRunResult::blocked(task_id)
                            };
                            result.open_issues = self.task_store.state(task_id)?.unresolved_errors;
                            result.steps = steps;
                            result.cost = self.cost_value();
                            self.tracer.record(
                                task_id,
                                "final",
                                json!({"status": "blocked", "reason": "tool errors exceeded bounded retry"}),
                            )?;
                            observer.on_run_end(&result);
                            return Ok(result);
                        }
                    } else {
                        consecutive_tool_errors = 0;
                    }
                    if let Err(reason) = self.check_budgets(task_id, &task) {
                        self.task_store.set_status(
                            task_id,
                            crate::contracts::TaskStatus::Blocked,
                            &reason,
                        )?;
                        let mut result = TaskRunResult::blocked(task_id);
                        result.steps = steps;
                        result.cost = self.cost_value();
                        self.tracer.record(
                            task_id,
                            "final",
                            json!({"status": "blocked", "reason": reason}),
                        )?;
                        observer.on_run_end(&result);
                        return Ok(result);
                    }
                }
            }
        }

        self.task_store.set_status(
            task_id,
            crate::contracts::TaskStatus::Blocked,
            "max steps exceeded",
        )?;
        let mut result = TaskRunResult::blocked(task_id);
        result.steps = steps;
        result.cost = self.cost_value();
        observer.on_run_end(&result);
        Ok(result)
    }

    // ---- internals ----

    fn cost_value(&mut self) -> Value {
        self.cost.report().as_value()
    }

    fn finish_result(&self, task_id: &str, status: &str) -> TaskRunResult {
        TaskRunResult {
            task_id: task_id.to_string(),
            status: crate::contracts::TaskStatus::from_str(status)
                .unwrap_or(crate::contracts::TaskStatus::Blocked),
            answer: None,
            steps: 0,
            cost: Value::Null,
            open_issues: Vec::new(),
            routing: self.routing.as_value(),
        }
    }

    fn decide(&mut self, task_id: &str, envelope: &ContextEnvelope) -> Result<ModelResponse> {
        let response = match self.model.decide(envelope) {
            Ok(r) => r,
            Err(e) => return Err(SfaiError::Model(e.to_string())),
        };
        self.ledger.db.execute(
            "INSERT INTO decisions(decision_id, task_id, model_id, input_context_hash,
             intent_json, usage_json, latency_ms, created_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,datetime('now'))",
            params![
                new_decision_id(),
                task_id,
                response.model_id,
                EnvelopeBuilder::envelope_hash(envelope),
                serde_json::to_string(&response.intent)?,
                serde_json::to_string(&response.usage)?,
                response.latency_ms,
            ],
        )?;
        self.cost.add_model_usage(&response.usage);
        Ok(response)
    }

    fn dispatch(
        &mut self,
        task_id: &str,
        intent: &ActionIntent,
        state: &crate::contracts::TaskState,
    ) -> Result<crate::contracts::ToolResult> {
        use crate::contracts::ToolResult;
        if intent.kind == IntentKind::Execute {
            return self.broker.execute(intent, state);
        }
        if intent.kind == IntentKind::Discover {
            let query = intent
                .payload
                .get("query")
                .and_then(|q| q.as_str())
                .unwrap_or("");
            let limit = intent
                .payload
                .get("limit")
                .and_then(|l| l.as_u64())
                .unwrap_or(5) as usize;
            let filters = intent.payload.get("filters");
            let found = self.registry.discover(query, limit, filters);
            return Ok(ToolResult {
                operation_id: Some("capabilities.discover".into()),
                status: crate::contracts::ToolStatus::Ok,
                result_preview: format!("{} capability(ies)", found.len()),
                result_data: json!({"capabilities": found}),
                ..Default::default()
            });
        }
        if intent.kind == IntentKind::Activate {
            if let Some(skill_id) = intent.payload.get("skill_id").and_then(|s| s.as_str()) {
                return match self.skills.activate(skill_id) {
                    Ok(activation) => {
                        let summary = format!(
                            "activated skill {} v{}",
                            activation.name, activation.version
                        );
                        self.activated_skills
                            .insert(activation.name.clone(), activation);
                        Ok(crate::contracts::ToolResult {
                            operation_id: Some("skills.activate".into()),
                            status: crate::contracts::ToolStatus::Ok,
                            result_preview: summary,
                            result_data: json!({"skill": skill_id}),
                            ..Default::default()
                        })
                    }
                    Err(exc) => Ok(crate::contracts::ToolResult {
                        operation_id: Some("skills.activate".into()),
                        status: crate::contracts::ToolStatus::Error,
                        error_code: Some("UNKNOWN_SKILL".into()),
                        diagnostic: Some(json!({
                            "verdict": "fail",
                            "error_code": "UNKNOWN_SKILL",
                            "repair_hint": exc.to_string(),
                        })),
                        ..Default::default()
                    }),
                };
            }
            let cid = intent
                .payload
                .get("capability_id")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            return match self.registry.activate(cid) {
                Ok(ops) => Ok(ToolResult {
                    operation_id: Some("capabilities.activate".into()),
                    status: crate::contracts::ToolStatus::Ok,
                    result_preview: format!("activated {cid}: {} operation(s)", ops.len()),
                    result_data: json!({"operations": ops}),
                    ..Default::default()
                }),
                Err(exc) => Ok(ToolResult {
                    operation_id: Some("capabilities.activate".into()),
                    status: crate::contracts::ToolStatus::Error,
                    error_code: Some("UNKNOWN_CAPABILITY".into()),
                    diagnostic: Some(json!({
                        "verdict": "fail",
                        "error_code": "UNKNOWN_CAPABILITY",
                        "repair_hint": exc.to_string(),
                    })),
                    ..Default::default()
                }),
            };
        }
        if intent.kind == IntentKind::Retrieve {
            return self.dispatch_retrieve(task_id, intent);
        }
        if intent.kind == IntentKind::Verify {
            let mut criteria: Vec<String> = intent
                .payload
                .get("criteria")
                .and_then(|c| c.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            if criteria.is_empty() {
                if let Ok(task) = self.task_store.load(task_id) {
                    criteria = task.acceptance_rules;
                }
            }
            if criteria.is_empty() {
                criteria.push("source_count_at_least:0".to_string());
            }
            let results = self.verifier.verify(task_id, &criteria)?;
            for result in &results {
                self.task_store.apply(
                    task_id,
                    &[json!({
                        "type": "verification_recorded",
                        "payload": serde_json::to_value(result)?,
                    })],
                )?;
            }
            let failed = results
                .iter()
                .filter(|r| r.verdict == crate::contracts::Verdict::Fail)
                .count();
            let diag = results
                .iter()
                .find(|r| r.verdict == crate::contracts::Verdict::Fail)
                .and_then(|r| r.diagnostics.first())
                .map(|d| json!(d));
            return Ok(ToolResult {
                operation_id: Some("verify.run".into()),
                status: if failed == 0 {
                    crate::contracts::ToolStatus::Ok
                } else {
                    crate::contracts::ToolStatus::Error
                },
                result_preview: format!("{} check(s), {} failure(s)", results.len(), failed),
                result_data: json!({"results": results}),
                error_code: if failed > 0 {
                    Some("VERIFICATION_FAILED".into())
                } else {
                    None
                },
                diagnostic: diag,
                ..Default::default()
            });
        }
        Ok(ToolResult {
            status: crate::contracts::ToolStatus::Error,
            error_code: Some("UNHANDLED_INTENT_KIND".into()),
            diagnostic: Some(json!({
                "verdict": "fail",
                "error_code": "UNHANDLED_INTENT_KIND",
                "repair_hint": format!("no dispatcher for kind {}", intent.kind.as_str()),
            })),
            ..Default::default()
        })
    }

    fn dispatch_retrieve(
        &self,
        task_id: &str,
        intent: &ActionIntent,
    ) -> Result<crate::contracts::ToolResult> {
        use crate::contracts::{ToolResult, ToolStatus};
        let op = intent
            .payload
            .get("op")
            .and_then(|o| o.as_str())
            .unwrap_or("search");
        if op == "search" {
            let query = intent
                .payload
                .get("query")
                .and_then(|q| q.as_str())
                .unwrap_or("");
            let limit = intent
                .payload
                .get("limit")
                .and_then(|l| l.as_u64())
                .unwrap_or(5) as usize;
            let scope_task =
                if intent.payload.get("task_scope").and_then(|s| s.as_str()) == Some("all") {
                    None
                } else {
                    Some(task_id)
                };
            let candidates = self.retriever.search(query, scope_task, None, limit)?;
            return Ok(ToolResult {
                operation_id: Some("context.search".into()),
                status: ToolStatus::Ok,
                result_preview: format!("{} candidate(s) for {:?}", candidates.len(), query),
                result_refs: candidates.iter().map(|c| c.ref_id.clone()).collect(),
                result_data: json!({"candidates": candidates.iter().map(|c| c.ref_id.clone()).collect::<Vec<_>>() }),
                ..Default::default()
            });
        }
        if op == "read" {
            let ref_id = intent
                .payload
                .get("ref_id")
                .and_then(|r| r.as_str())
                .unwrap_or("");
            let max_tokens = intent
                .payload
                .get("max_tokens")
                .and_then(|t| t.as_u64())
                .unwrap_or(600) as usize;
            return match self.retriever.read(ref_id, None, max_tokens) {
                Ok(payload) => Ok(ToolResult {
                    operation_id: Some("context.read".into()),
                    status: ToolStatus::Ok,
                    result_preview: payload.excerpt.chars().take(400).collect(),
                    result_refs: vec![ref_id.to_string()],
                    result_data: json!(payload),
                    ..Default::default()
                }),
                Err(exc) => Ok(ToolResult {
                    operation_id: Some("context.read".into()),
                    status: ToolStatus::Error,
                    error_code: Some("UNRESOLVABLE_REF".into()),
                    diagnostic: Some(json!({
                        "verdict": "fail",
                        "error_code": "UNRESOLVABLE_REF",
                        "repair_hint": exc.to_string(),
                    })),
                    ..Default::default()
                }),
            };
        }
        if op == "resolve" {
            let ref_ids: Vec<String> = intent
                .payload
                .get("ref_ids")
                .and_then(|r| r.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let resolved = self.retriever.resolve(&ref_ids)?;
            return Ok(ToolResult {
                operation_id: Some("context.resolve".into()),
                status: ToolStatus::Ok,
                result_preview: format!("{} ref(s) resolved", resolved.len()),
                result_refs: ref_ids,
                result_data: json!({"resolved": resolved}),
                ..Default::default()
            });
        }
        Ok(ToolResult {
            operation_id: Some("context.*".into()),
            status: ToolStatus::Error,
            error_code: Some("UNKNOWN_CONTEXT_OP".into()),
            diagnostic: Some(json!({
                "verdict": "fail",
                "error_code": "UNKNOWN_CONTEXT_OP",
                "repair_hint": format!("unknown context op {op:?}"),
            })),
            ..Default::default()
        })
    }

    fn apply_result(
        &mut self,
        task_id: &str,
        intent: &ActionIntent,
        result: &crate::contracts::ToolResult,
    ) -> Result<()> {
        self.tracer.record(
            task_id,
            "tool_result",
            json!({
                "kind": intent.kind.as_str(),
                "status": result.status.as_str(),
                "operation_id": result.operation_id,
                "result_refs": result.result_refs,
                "side_effects": result.side_effects,
                "replayed": result.replayed,
                "error_code": result.error_code,
            }),
        )?;
        if result.replayed {
            self.routing.duplicate_replays += 1;
        } else {
            self.cost.add_tool_call();
        }
        if result.error_code.as_deref() == Some("UNKNOWN_OPERATION") {
            self.routing.misroutes += 1;
        }
        // Meta operations bypass the broker, but their results must reach the
        // ledger so later envelopes can show previews and budgets stay exact.
        if intent.kind != crate::contracts::IntentKind::Execute && !result.replayed {
            self.task_store.apply(
                task_id,
                &[json!({
                    "type": "tool_committed",
                    "operation_id": result.operation_id,
                    "status": result.status.as_str(),
                    "result_preview": result.result_preview.chars().take(2000).collect::<String>(),
                    "result_refs": result.result_refs,
                    "side_effects": result.side_effects,
                    "idempotency_key": intent.idempotency_key,
                })],
            )?;
        }
        if !result.state_events.is_empty() {
            self.task_store.apply(task_id, &result.state_events)?;
        }
        let error_key = format!(
            "tool:{}",
            result
                .operation_id
                .as_deref()
                .unwrap_or(intent.kind.as_str())
        );
        if result.status == crate::contracts::ToolStatus::Error && result.diagnostic.is_some() {
            self.task_store.apply(
                task_id,
                &[json!({
                    "type": "error_opened",
                    "error": {
                        "key": error_key,
                        "error_code": result.error_code.clone().unwrap_or_else(|| "TOOL_ERROR".into()),
                        "diagnostic": result.diagnostic,
                    },
                })],
            )?;
        } else if result.status == crate::contracts::ToolStatus::Ok {
            self.task_store.apply(
                task_id,
                &[json!({"type": "error_resolved", "key": error_key})],
            )?;
        }
        Ok(())
    }

    fn try_complete(
        &self,
        task_id: &str,
        task: &Task,
        intent: &ActionIntent,
    ) -> Result<(bool, Option<TaskRunResult>)> {
        let state = self.task_store.state(task_id)?;
        let assessment = if self.enforce_acceptance {
            self.acceptance.evaluate_if_required(task, &state)?
        } else {
            AcceptanceAssessment::default()
        };
        let answer = intent
            .payload
            .get("text")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());
        if assessment.blocks_completion {
            for issue in &assessment.open_issues {
                self.task_store.apply(
                    task_id,
                    &[json!({"type": "issue_recorded", "issue": issue})],
                )?;
            }
            self.tracer.record(
                task_id,
                "acceptance_blocked",
                json!({"issues": assessment.open_issues}),
            )?;
            return Ok((false, None));
        }
        // Risk-based mandatory validation: a consequential action must be
        // followed by a recorded verification before the task may complete.
        if self.enforce_acceptance && self.mandatory_verification_missing(task_id)? {
            let issue = json!({
                "key": "mandatory_verification",
                "criterion": "mandatory_verification",
                "findings": "external/high-impact action executed without a later verification",
                "error_code": "MANDATORY_VERIFICATION_MISSING",
                "repair_hint": "Run verify.run after the consequential action.",
            });
            self.task_store.apply(
                task_id,
                &[json!({"type": "issue_recorded", "issue": issue})],
            )?;
            self.tracer.record(
                task_id,
                "mandatory_verification_blocked",
                json!({"reason": "MANDATORY_VERIFICATION_MISSING"}),
            )?;
            return Ok((false, None));
        }
        self.task_store
            .checkpoint(task_id, "acceptance passed", Some(&state.completed_outputs))?;
        let reason = if task.acceptance_rules.is_empty() {
            "finished"
        } else {
            "acceptance verified"
        };
        self.task_store
            .set_status(task_id, crate::contracts::TaskStatus::Completed, reason)?;
        self.tracer.record(
            task_id,
            "final",
            json!({"status": "completed", "answer": answer}),
        )?;
        Ok((true, Some(TaskRunResult::completed(task_id, answer))))
    }

    /// True when the most recent external/high-impact action has no later
    /// verification event in the ledger.
    fn mandatory_verification_missing(&self, task_id: &str) -> Result<bool> {
        let events = self.ledger.events_for(task_id, 0)?;
        let mut last_consequential = 0i64;
        let mut last_verification = 0i64;
        for event in &events {
            if event.event_type == "tool.committed" {
                if let Some(risk) = event.payload.get("risk_class").and_then(|r| r.as_str()) {
                    if matches!(risk, "external_write" | "high_impact") {
                        last_consequential = event.seq;
                    }
                }
            }
            if event.event_type == "verification.recorded" {
                last_verification = event.seq;
            }
        }
        Ok(last_consequential > 0 && last_verification < last_consequential)
    }

    fn check_budgets(&self, task_id: &str, task: &Task) -> std::result::Result<(), String> {
        let state = self.task_store.state(task_id).map_err(|e| e.to_string())?;
        if let Some(max) = task.budget.max_tool_calls {
            if state.tool_calls_made > max {
                return Err("tool call budget exceeded".into());
            }
        }
        let report = self.cost.report_snapshot();
        if let Some(max) = task.budget.max_cost_usd {
            if report.cost_usd > max {
                return Err("cost budget exceeded".into());
            }
        }
        if let Some(max) = task.budget.max_model_calls {
            if report.model_calls > max {
                return Err("model call budget exceeded".into());
            }
        }
        Ok(())
    }
}
