//! Persistent entity contracts (plan §3.1) and shared enums.
//!
//! `serde(deny_unknown_fields)` keeps producers and consumers honest at the
//! boundary. JSON shapes are the portable spec (ADR-012): the same documents
//! a Python or any other implementation would exchange.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Created,
    Active,
    WaitingApproval,
    Paused,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Created => "created",
            TaskStatus::Active => "active",
            TaskStatus::WaitingApproval => "waiting_approval",
            TaskStatus::Paused => "paused",
            TaskStatus::Blocked => "blocked",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<Self> {
        Some(match value {
            "created" => TaskStatus::Created,
            "active" => TaskStatus::Active,
            "waiting_approval" => TaskStatus::WaitingApproval,
            "paused" => TaskStatus::Paused,
            "blocked" => TaskStatus::Blocked,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            _ => return None,
        })
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        )
    }
}

/// §3.4 operational lifecycle only — never a task workflow.
pub fn status_transition_allowed(current: TaskStatus, next: TaskStatus) -> bool {
    match current {
        TaskStatus::Created => matches!(next, TaskStatus::Active | TaskStatus::Cancelled),
        TaskStatus::Active => matches!(
            next,
            TaskStatus::WaitingApproval
                | TaskStatus::Paused
                | TaskStatus::Blocked
                | TaskStatus::Completed
                | TaskStatus::Failed
                | TaskStatus::Cancelled
        ),
        TaskStatus::WaitingApproval => matches!(
            next,
            TaskStatus::Active | TaskStatus::Blocked | TaskStatus::Cancelled | TaskStatus::Failed
        ),
        TaskStatus::Paused => {
            matches!(
                next,
                TaskStatus::Active | TaskStatus::Cancelled | TaskStatus::Failed
            )
        }
        TaskStatus::Blocked => {
            matches!(
                next,
                TaskStatus::Active | TaskStatus::Failed | TaskStatus::Cancelled
            )
        }
        TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    ReadOnly,
    ReversibleWrite,
    ExternalWrite,
    HighImpact,
}

impl RiskClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskClass::ReadOnly => "read_only",
            RiskClass::ReversibleWrite => "reversible_write",
            RiskClass::ExternalWrite => "external_write",
            RiskClass::HighImpact => "high_impact",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IntentKind {
    Discover,
    Retrieve,
    Activate,
    Execute,
    Verify,
    Respond,
    AskUser,
    Pause,
    Finish,
}

impl IntentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            IntentKind::Discover => "discover",
            IntentKind::Retrieve => "retrieve",
            IntentKind::Activate => "activate",
            IntentKind::Execute => "execute",
            IntentKind::Verify => "verify",
            IntentKind::Respond => "respond",
            IntentKind::AskUser => "ask_user",
            IntentKind::Pause => "pause",
            IntentKind::Finish => "finish",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Trusted,
    Untrusted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Ok,
    #[default]
    Error,
    Denied,
}

impl ToolStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolStatus::Ok => "ok",
            ToolStatus::Error => "error",
            ToolStatus::Denied => "denied",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<Self> {
        Some(match value {
            "ok" => ToolStatus::Ok,
            "error" => ToolStatus::Error,
            "denied" => ToolStatus::Denied,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
    Uncertain,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Pass => "pass",
            Verdict::Fail => "fail",
            Verdict::Uncertain => "uncertain",
        }
    }
}

fn empty_object() -> Value {
    Value::Object(Default::default())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    #[serde(default = "default_max_tool_calls")]
    pub max_tool_calls: Option<u32>,
    #[serde(default)]
    pub max_model_calls: Option<u32>,
    #[serde(default)]
    pub max_cost_usd: Option<f64>,
    #[serde(default)]
    pub max_wall_clock_s: Option<f64>,
}

fn default_max_tool_calls() -> Option<u32> {
    Some(20)
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_tool_calls: Some(20),
            max_model_calls: None,
            max_cost_usd: None,
            max_wall_clock_s: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub task_id: String,
    pub user_id: String,
    #[serde(default = "default_tenant")]
    pub tenant_id: String,
    pub goal: String,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub acceptance_rules: Vec<String>,
    #[serde(default = "default_status")]
    pub status: TaskStatus,
    #[serde(default)]
    pub budget: Budget,
    #[serde(default = "default_version")]
    pub version: u32,
}

fn default_tenant() -> String {
    "default".to_string()
}
fn default_status() -> TaskStatus {
    TaskStatus::Created
}
fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TaskState {
    pub task_id: String,
    #[serde(default = "default_status")]
    pub status: TaskStatus,
    #[serde(default)]
    pub active_intent: Option<String>,
    #[serde(default)]
    pub completed_outputs: Vec<String>,
    #[serde(default)]
    pub unresolved_errors: Vec<Value>,
    #[serde(default)]
    pub open_issues: Vec<Value>,
    #[serde(default)]
    pub verifications: Vec<Value>,
    #[serde(default = "empty_object")]
    pub checkpoint: Value,
    #[serde(default)]
    pub pinned_constraints: Vec<String>,
    #[serde(default)]
    pub tool_calls_made: u32,
    #[serde(default)]
    pub model_calls_made: u32,
    #[serde(default)]
    pub last_seq: i64,
    #[serde(default = "default_version")]
    pub version: u32,
}

impl Default for TaskState {
    fn default() -> Self {
        Self {
            task_id: String::new(),
            status: TaskStatus::Created,
            active_intent: None,
            completed_outputs: Vec::new(),
            unresolved_errors: Vec::new(),
            open_issues: Vec::new(),
            verifications: Vec::new(),
            checkpoint: empty_object(),
            pinned_constraints: Vec::new(),
            tool_calls_made: 0,
            model_calls_made: 0,
            last_seq: 0,
            version: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActionIntent {
    pub kind: IntentKind,
    pub task_id: String,
    #[serde(default = "default_reason")]
    pub reason_code: String,
    #[serde(default = "empty_object")]
    pub payload: Value,
    #[serde(default)]
    pub expected_refs: Vec<String>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

fn default_reason() -> String {
    "unspecified".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    #[serde(default)]
    pub seq: i64,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    pub task_id: String,
    pub actor: String,
    pub event_type: String,
    pub timestamp: String,
    #[serde(default = "empty_object")]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct ModelUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cached_input_tokens: u64,
    #[serde(default)]
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelResponse {
    #[serde(default = "default_model_id")]
    pub model_id: String,
    pub intent: ActionIntent,
    #[serde(default)]
    pub usage: ModelUsage,
    #[serde(default)]
    pub latency_ms: f64,
}

fn default_model_id() -> String {
    "fake".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub source_id: String,
    pub task_id: String,
    pub origin: String,
    pub content_hash: String,
    #[serde(default = "default_mime")]
    pub mime_type: String,
    #[serde(default = "default_owner")]
    pub owner: String,
    #[serde(default = "default_untrusted")]
    pub trust_level: TrustLevel,
    pub storage_uri: String,
    #[serde(default = "default_version")]
    pub revision: u32,
    #[serde(default)]
    pub created_at: String,
}

fn default_mime() -> String {
    "text/plain".to_string()
}
fn default_owner() -> String {
    "system".to_string()
}
fn default_untrusted() -> TrustLevel {
    TrustLevel::Untrusted
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceRef {
    pub ref_id: String,
    pub source_id: String,
    #[serde(default = "default_version")]
    pub revision: u32,
    #[serde(default)]
    pub locator: Option<String>,
    #[serde(default)]
    pub token_estimate: u32,
    #[serde(default = "default_access_scope")]
    pub access_scope: String,
}

fn default_access_scope() -> String {
    "task".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub artifact_id: String,
    pub task_id: String,
    pub path: String,
    pub checksum: String,
    #[serde(default = "default_markdown_mime")]
    pub mime_type: String,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub provenance: Vec<String>,
    #[serde(default)]
    pub created_at: String,
}

fn default_markdown_mime() -> String {
    "text/markdown".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ToolRun {
    pub tool_run_id: String,
    pub task_id: String,
    pub operation_id: String,
    pub args_hash: String,
    pub status: ToolStatus,
    #[serde(default)]
    pub result_preview: String,
    #[serde(default)]
    pub result_refs: Vec<String>,
    #[serde(default = "default_environment")]
    pub environment: String,
    #[serde(default)]
    pub side_effects: Vec<String>,
    #[serde(default)]
    pub receipt: Option<Value>,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub created_at: String,
}

fn default_environment() -> String {
    "sandbox".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct ToolResult {
    #[serde(default)]
    pub tool_run_id: Option<String>,
    #[serde(default)]
    pub operation_id: Option<String>,
    #[serde(default = "default_tool_status")]
    pub status: ToolStatus,
    #[serde(default)]
    pub result_preview: String,
    #[serde(default)]
    pub result_refs: Vec<String>,
    #[serde(default)]
    pub side_effects: Vec<String>,
    #[serde(default)]
    pub receipt: Option<Value>,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub diagnostic: Option<Value>,
    #[serde(default)]
    pub state_events: Vec<Value>,
    #[serde(default = "empty_object")]
    pub result_data: Value,
    #[serde(default)]
    pub replayed: bool,
}

fn default_tool_status() -> ToolStatus {
    ToolStatus::Error
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityOperation {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "empty_object")]
    pub input_schema: Value,
    #[serde(default = "default_risk")]
    pub risk_class: RiskClass,
    #[serde(default)]
    pub side_effects: Vec<String>,
}

fn default_risk() -> RiskClass {
    RiskClass::ReadOnly
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityManifest {
    pub id: String,
    #[serde(default = "default_version_str")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub operations: Vec<CapabilityOperation>,
    #[serde(default)]
    pub permissions: BTreeMap<String, bool>,
    #[serde(default = "default_risk")]
    pub risk_class: RiskClass,
    #[serde(default)]
    pub output_contract: BTreeMap<String, bool>,
    #[serde(default)]
    pub healthcheck: Option<String>,
}

fn default_version_str() -> String {
    "0.1.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VerificationDiagnostic {
    pub verdict: Verdict,
    pub target_ref: String,
    pub criterion: String,
    pub error_code: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub repair_hint: String,
    #[serde(default)]
    pub repair_safe_to_auto_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VerificationResult {
    pub target_ref: String,
    pub criterion: String,
    pub verdict: Verdict,
    #[serde(default)]
    pub findings: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub diagnostics: Vec<VerificationDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct RetrievalReceipt {
    #[serde(default)]
    pub selected_refs: Vec<String>,
    #[serde(default)]
    pub omitted_candidates: usize,
    #[serde(default)]
    pub revisions: BTreeMap<String, u32>,
    #[serde(default)]
    pub estimated_tokens: usize,
    #[serde(default)]
    pub reason_codes: Vec<String>,
}

/// An activated Skill's instructions, bounded, for the working set only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActiveSkill {
    pub name: String,
    #[serde(default = "default_version_str")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub instructions: String,
}

/// Bounded model-visible context (plan §3.5).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContextEnvelope {
    pub goal: String,
    #[serde(default)]
    pub pinned_constraints: Vec<String>,
    #[serde(default = "empty_object")]
    pub task_state: Value,
    #[serde(default)]
    pub root_index: BTreeMap<String, String>,
    #[serde(default)]
    pub active_refs: Vec<String>,
    #[serde(default)]
    pub recent_results: Vec<Value>,
    /// Bounded auto-memory / topic notes relevant to this step (memory layer).
    #[serde(default)]
    pub memory: Vec<Value>,
    #[serde(default)]
    pub active_skills: Vec<ActiveSkill>,
    #[serde(default = "empty_object")]
    pub budget: Value,
    #[serde(default)]
    pub retrieval_receipt: Option<RetrievalReceipt>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_status_roundtrip() {
        for s in [
            "created",
            "active",
            "waiting_approval",
            "paused",
            "blocked",
            "completed",
            "failed",
            "cancelled",
        ] {
            let st = TaskStatus::from_str(s).unwrap();
            assert_eq!(st.as_str(), s);
            let json = serde_json::to_string(&st).unwrap();
            assert_eq!(json, format!("\"{s}\""));
        }
    }

    #[test]
    fn transitions_are_bounded() {
        assert!(status_transition_allowed(
            TaskStatus::Created,
            TaskStatus::Active
        ));
        assert!(!status_transition_allowed(
            TaskStatus::Completed,
            TaskStatus::Active
        ));
        assert!(status_transition_allowed(
            TaskStatus::Blocked,
            TaskStatus::Active
        ));
        assert!(!status_transition_allowed(
            TaskStatus::Cancelled,
            TaskStatus::Active
        ));
    }

    #[test]
    fn unknown_fields_rejected() {
        let bad = r#"{"kind":"respond","task_id":"t","extra":1}"#;
        assert!(serde_json::from_str::<ActionIntent>(bad).is_err());
    }

    #[test]
    fn tool_result_roundtrip() {
        let tr = ToolResult {
            status: ToolStatus::Ok,
            result_preview: "done".into(),
            result_refs: vec!["source://a@1".into()],
            ..Default::default()
        };
        let json = serde_json::to_string(&tr).unwrap();
        let back: ToolResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.result_preview, "done");
        assert_eq!(back.status, ToolStatus::Ok);
    }
}
