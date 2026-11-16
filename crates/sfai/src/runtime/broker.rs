//! Policy-aware tool broker (plan §1 / §8).
//!
//! The model chooses *which action to request*; the broker decides whether and
//! how it executes:
//!
//! - unknown operations are rejected (capability discovery is explicit),
//! - risk class comes only from the capability manifest,
//! - external/high-impact actions require an explicit approval receipt,
//! - idempotency keys make retries safe: a duplicate intent replays the stored
//!   result instead of committing a second side effect,
//! - denials are auditable events, never executions,
//! - every run produces a ToolRun receipt and a `tool.committed` event.

use crate::authority::approvals::{ApprovalDecision, ApprovalProvider};
use crate::capabilities::registry::{CapabilityRegistry, RunContext, ToolExecError};
use crate::contracts::{ActionIntent, IntentKind, TaskState, ToolResult, ToolRun, ToolStatus};
use crate::error::Result;
use crate::ids::{canonical_hash, new_tool_run_id};
use crate::runtime::policy::PolicyEngine;
use crate::storage::db::{row_str, Database};
use crate::storage::event_ledger::{now, EventLedger};
use crate::storage::objects::ObjectStore;
use rusqlite::params;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub struct ToolBroker {
    pub registry: Rc<CapabilityRegistry>,
    pub policy: PolicyEngine,
    pub approvals: Rc<dyn ApprovalProvider>,
    pub objects: ObjectStore,
    pub ledger: EventLedger,
    pub workspace_root: PathBuf,
    pub invocations: RefCell<Vec<String>>,
}

impl ToolBroker {
    pub fn new(
        registry: Rc<CapabilityRegistry>,
        policy: PolicyEngine,
        approvals: Rc<dyn ApprovalProvider>,
        objects: ObjectStore,
        ledger: EventLedger,
        workspace_root: PathBuf,
    ) -> Self {
        Self {
            registry,
            policy,
            approvals,
            objects,
            ledger,
            workspace_root,
            invocations: RefCell::new(Vec::new()),
        }
    }

    pub fn execute(&self, intent: &ActionIntent, state: &TaskState) -> Result<ToolResult> {
        if intent.kind != IntentKind::Execute {
            return Ok(ToolResult {
                status: ToolStatus::Error,
                operation_id: Some(intent.kind.as_str().to_string()),
                error_code: Some("BROKER_WRONG_INTENT_KIND".into()),
                diagnostic: Some(json!({
                    "verdict": "fail",
                    "error_code": "BROKER_WRONG_INTENT_KIND",
                    "repair_hint": format!("Broker handles 'execute' intents only, got {}", intent.kind.as_str()),
                })),
                ..Default::default()
            });
        }

        let operation_id = intent
            .payload
            .get("operation_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let args = intent
            .payload
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        // Idempotent replay: never commit the same intent twice.
        if let Some(key) = intent.idempotency_key.as_deref() {
            if let Some(stored) = self.find_run(key)? {
                return Ok(self.result_from_run(&stored, true, None));
            }
        }

        let (manifest, operation) = match self.registry.resolve_operation(&operation_id) {
            Ok(pair) => pair,
            Err(exc) => {
                return self.deny(
                    intent,
                    &operation_id,
                    &args,
                    "UNKNOWN_OPERATION",
                    &format!("{exc}. Use capabilities.discover to find valid operations."),
                    None,
                );
            }
        };

        let risk_class = operation.risk_class.as_str().to_string();
        let decision = self.policy.evaluate(&manifest, &operation, &intent.task_id);
        if !decision.allowed {
            return self.deny(
                intent,
                &operation_id,
                &args,
                "POLICY_DENIED",
                &decision.reason_code,
                None,
            );
        }

        let mut receipt: Option<Value> = None;
        if decision.requires_approval {
            let approval: ApprovalDecision = self.approvals.request(
                &intent.task_id,
                &decision.action_summary,
                operation.risk_class,
            );
            if !approval.approved {
                return self.deny(
                    intent,
                    &operation_id,
                    &args,
                    "APPROVAL_DENIED",
                    "User denied this action; do not retry without a new reason.",
                    Some(json!({
                        "denied_action": decision.action_summary,
                        "decided_by": approval.decided_by,
                    })),
                );
            }
            let mut merged = json!({
                "action": decision.action_summary,
                "decided_by": approval.decided_by,
                "risk_class": operation.risk_class.as_str(),
            });
            if let Some(extra) = approval.receipt {
                if let (Some(base), Some(Value::Object(extra))) =
                    (merged.as_object_mut(), Some(extra))
                {
                    for (k, v) in extra {
                        base.insert(k, v);
                    }
                }
            }
            receipt = Some(merged);
        }

        let mut ctx = RunContext {
            task_id: intent.task_id.clone(),
            operation_id: operation_id.clone(),
            objects: self.objects.clone(),
            task_state: state.clone(),
            workspace_root: self.workspace_root.join(&intent.task_id),
        };
        self.invocations.borrow_mut().push(operation_id.clone());

        let exec_result = match self.registry.handler_for(&operation_id) {
            Ok(handler) => handler(&mut ctx, &args),
            Err(_) => Err(ToolExecError {
                message: format!("no handler for {operation_id}"),
                error_code: "NO_HANDLER".into(),
            }),
        };

        match exec_result {
            Err(exc) => {
                let run = self.commit_run(
                    intent,
                    &operation_id,
                    &args,
                    ToolStatus::Error,
                    &exc.message,
                    &[],
                    &[],
                    None,
                    Some(&exc.error_code),
                    &risk_class,
                )?;
                Ok(self.result_from_run(
                    &run,
                    false,
                    Some(json!({
                        "verdict": "fail",
                        "error_code": exc.error_code,
                        "repair_hint": "Inspect tool error and retry with corrected arguments if safe.",
                    })),
                ))
            }
            Ok(exec) => {
                let mut final_receipt = if decision.requires_receipt {
                    receipt.clone()
                } else {
                    exec.receipt.clone()
                };
                if let (Some(existing), Some(extra)) =
                    (final_receipt.as_mut(), exec.receipt.clone())
                {
                    if let (Some(base), Value::Object(extra_map)) =
                        (existing.as_object_mut(), extra)
                    {
                        for (k, v) in extra_map {
                            base.insert(k, v);
                        }
                    }
                }
                let run = self.commit_run(
                    intent,
                    &operation_id,
                    &args,
                    ToolStatus::Ok,
                    &exec.preview,
                    &exec.result_refs,
                    &exec.side_effects,
                    final_receipt,
                    None,
                    &risk_class,
                )?;
                let mut result = self.result_from_run(&run, false, None);
                result.state_events = exec.state_events;
                result.result_data = exec.result_data;
                Ok(result)
            }
        }
    }

    // ---- helpers ----

    pub fn find_run(&self, idempotency_key: &str) -> Result<Option<ToolRun>> {
        let row = self.db().query_one(
            "SELECT * FROM tool_runs WHERE idempotency_key = ?1",
            [idempotency_key],
        )?;
        match row {
            Some(row) => Ok(Some(serde_json::from_str(&row_str(&row, "result_json"))?)),
            None => Ok(None),
        }
    }

    fn db(&self) -> &Database {
        &self.ledger.db
    }

    #[allow(clippy::too_many_arguments)]
    fn commit_run(
        &self,
        intent: &ActionIntent,
        operation_id: &str,
        args: &Value,
        status: ToolStatus,
        preview: &str,
        result_refs: &[String],
        side_effects: &[String],
        receipt: Option<Value>,
        error_code: Option<&str>,
        risk_class: &str,
    ) -> Result<ToolRun> {
        let run = ToolRun {
            tool_run_id: new_tool_run_id(),
            task_id: intent.task_id.clone(),
            operation_id: operation_id.to_string(),
            args_hash: canonical_hash(args),
            status,
            result_preview: preview.chars().take(2000).collect(),
            result_refs: result_refs.to_vec(),
            environment: "sandbox".into(),
            side_effects: side_effects.to_vec(),
            receipt,
            error_code: error_code.map(|s| s.to_string()),
            idempotency_key: intent.idempotency_key.clone(),
            created_at: now(),
        };
        self.db().execute(
            "INSERT INTO tool_runs(tool_run_id, task_id, operation_id, idempotency_key,
             args_hash, status, result_json, created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                run.tool_run_id,
                run.task_id,
                run.operation_id,
                run.idempotency_key,
                run.args_hash,
                status.as_str(),
                serde_json::to_string(&run)?,
                run.created_at
            ],
        )?;
        self.ledger.append(
            &intent.task_id,
            &format!("tool:{operation_id}"),
            "tool.committed",
            json!({
                "tool_run_id": run.tool_run_id,
                "operation_id": operation_id,
                "status": status.as_str(),
                "result_preview": run.result_preview,
                "result_refs": run.result_refs,
                "side_effects": run.side_effects,
                "args_hash": run.args_hash,
                "idempotency_key": intent.idempotency_key,
                "risk_class": risk_class,
            }),
            intent.idempotency_key.as_deref(),
        )?;
        Ok(run)
    }

    fn deny(
        &self,
        intent: &ActionIntent,
        operation_id: &str,
        args: &Value,
        error_code: &str,
        repair_hint: &str,
        receipt: Option<Value>,
    ) -> Result<ToolResult> {
        let run = self.commit_run(
            intent,
            operation_id,
            args,
            ToolStatus::Denied,
            &format!("denied: {error_code}"),
            &[],
            &[],
            receipt,
            Some(error_code),
            "",
        )?;
        Ok(self.result_from_run(
            &run,
            false,
            Some(json!({
                "verdict": "fail",
                "error_code": error_code,
                "repair_hint": repair_hint,
            })),
        ))
    }

    fn result_from_run(
        &self,
        run: &ToolRun,
        replayed: bool,
        diagnostic: Option<Value>,
    ) -> ToolResult {
        ToolResult {
            tool_run_id: Some(run.tool_run_id.clone()),
            operation_id: Some(run.operation_id.clone()),
            status: run.status,
            result_preview: run.result_preview.clone(),
            result_refs: run.result_refs.clone(),
            side_effects: run.side_effects.clone(),
            receipt: run.receipt.clone(),
            error_code: run.error_code.clone(),
            diagnostic,
            replayed,
            ..Default::default()
        }
    }
}
