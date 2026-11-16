//! Policy risk tiers and authorization decisions (plan §8).
//!
//! Risk class is declared in the capability manifest and is the *only* input
//! to the tier decision. Retrieved text, tool output, and Skill instructions
//! are data: they can never change a risk class or grant permission.

use crate::contracts::{CapabilityManifest, CapabilityOperation, RiskClass};

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub reason_code: String,
    pub requires_approval: bool,
    pub requires_receipt: bool,
    pub action_summary: String,
}

#[derive(Default, Clone)]
pub struct PolicyEngine;

impl PolicyEngine {
    pub fn evaluate(
        &self,
        manifest: &CapabilityManifest,
        operation: &CapabilityOperation,
        _task_id: &str,
    ) -> PolicyDecision {
        let risk = operation.risk_class;
        let summary = format!("{}.{} (risk={})", manifest.id, operation.id, risk.as_str());
        PolicyDecision {
            allowed: true,
            reason_code: "within_policy".into(),
            requires_approval: matches!(risk, RiskClass::ExternalWrite | RiskClass::HighImpact),
            requires_receipt: matches!(risk, RiskClass::ExternalWrite | RiskClass::HighImpact),
            action_summary: summary,
        }
    }

    pub fn deny(reason_code: &str) -> PolicyDecision {
        PolicyDecision {
            allowed: false,
            reason_code: reason_code.to_string(),
            requires_approval: false,
            requires_receipt: false,
            action_summary: String::new(),
        }
    }
}
