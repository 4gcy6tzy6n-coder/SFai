//! Approval providers for consequential actions (plan §8 policy risk tiers).
//!
//! The model can *request* authorization; it can never grant it to itself.
//! Approvals are recorded as receipts on the ToolRun.

use crate::contracts::RiskClass;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ApprovalDecision {
    pub approved: bool,
    pub decided_by: String,
    #[serde(default)]
    pub receipt: Option<Value>,
}

pub trait ApprovalProvider {
    fn request(
        &self,
        task_id: &str,
        action_summary: &str,
        risk_class: RiskClass,
    ) -> ApprovalDecision;
}

pub struct DenyAllApprovals;

impl ApprovalProvider for DenyAllApprovals {
    fn request(
        &self,
        _task_id: &str,
        _action_summary: &str,
        _risk_class: RiskClass,
    ) -> ApprovalDecision {
        ApprovalDecision {
            approved: false,
            decided_by: "policy:deny_all".into(),
            receipt: None,
        }
    }
}

pub struct AutoApprove;

impl ApprovalProvider for AutoApprove {
    fn request(
        &self,
        _task_id: &str,
        action_summary: &str,
        risk_class: RiskClass,
    ) -> ApprovalDecision {
        ApprovalDecision {
            approved: true,
            decided_by: "policy:auto_approve".into(),
            receipt: Some(json!({"action": action_summary, "risk_class": risk_class.as_str()})),
        }
    }
}

pub struct ScriptedApprovals {
    decisions: RefCell<VecDeque<bool>>,
    pub requested: RefCell<Vec<String>>,
}

impl ScriptedApprovals {
    pub fn new(decisions: Vec<bool>) -> Self {
        Self {
            decisions: RefCell::new(decisions.into()),
            requested: RefCell::new(Vec::new()),
        }
    }
}

impl ApprovalProvider for ScriptedApprovals {
    fn request(
        &self,
        _task_id: &str,
        action_summary: &str,
        risk_class: RiskClass,
    ) -> ApprovalDecision {
        self.requested.borrow_mut().push(action_summary.to_string());
        let approved = self.decisions.borrow_mut().pop_front().unwrap_or(false);
        ApprovalDecision {
            approved,
            decided_by: "policy:scripted".into(),
            receipt: Some(json!({"action": action_summary, "risk_class": risk_class.as_str()})),
        }
    }
}
