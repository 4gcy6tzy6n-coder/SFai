//! Termination and acceptance evaluation (plan §4 / §8).
//!
//! "No silent completion when acceptance is not verified": `finish` only
//! completes a task whose acceptance rules pass. Failed rules become visible
//! open issues and send the model back to work, bounded by retry limits.

use crate::contracts::{Task, TaskState, VerificationResult};
use crate::error::Result;
use crate::verify::engine::VerificationEngine;
use serde_json::{json, Value};

#[derive(Debug, Clone, Default)]
pub struct AcceptanceAssessment {
    pub blocks_completion: bool,
    pub results: Vec<VerificationResult>,
    pub open_issues: Vec<Value>,
}

pub struct AcceptanceEvaluator {
    pub verifier: VerificationEngine,
}

impl AcceptanceEvaluator {
    pub fn new(verifier: VerificationEngine) -> Self {
        Self { verifier }
    }

    pub fn evaluate_if_required(
        &self,
        task: &Task,
        _state: &TaskState,
    ) -> Result<AcceptanceAssessment> {
        if task.acceptance_rules.is_empty() {
            return Ok(AcceptanceAssessment::default());
        }
        let results = self
            .verifier
            .verify(&task.task_id, &task.acceptance_rules)?;
        let failures: Vec<&VerificationResult> = results
            .iter()
            .filter(|r| r.verdict == crate::contracts::Verdict::Fail)
            .collect();
        let issues: Vec<Value> = failures
            .iter()
            .map(|r| {
                let diag = r.diagnostics.first();
                json!({
                    "key": format!("acceptance:{}", r.criterion),
                    "criterion": r.criterion,
                    "findings": r.findings,
                    "error_code": diag.map(|d| d.error_code.clone()).unwrap_or_else(|| "ACCEPTANCE_FAILED".to_string()),
                    "repair_hint": diag.map(|d| d.repair_hint.clone()).unwrap_or_default(),
                })
            })
            .collect();
        Ok(AcceptanceAssessment {
            blocks_completion: !failures.is_empty(),
            results,
            open_issues: issues,
        })
    }
}
