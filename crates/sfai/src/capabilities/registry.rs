//! Capability registry: discovery, inspection, activation (plan §6).
//!
//! Discovery loads only name/description/eligibility metadata. Activation
//! returns operation schemas for the working set; it never grants permission —
//! every execution is re-checked by the policy-aware broker.

use crate::contracts::{CapabilityManifest, CapabilityOperation, TaskState};
use crate::error::{Result, SfaiError};
use crate::storage::objects::ObjectStore;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

#[derive(Debug, Clone, Default)]
pub struct ToolExecutionResult {
    pub preview: String,
    pub result_refs: Vec<String>,
    pub side_effects: Vec<String>,
    pub receipt: Option<Value>,
    pub state_events: Vec<Value>,
    pub result_data: Value,
}

#[derive(Debug, Clone)]
pub struct ToolExecError {
    pub message: String,
    pub error_code: String,
}

pub struct RunContext {
    pub task_id: String,
    pub operation_id: String,
    pub objects: ObjectStore,
    pub task_state: TaskState,
    /// Per-task workspace directory where artifacts are materialized for
    /// deterministic code checks (`code_tests_pass`).
    pub workspace_root: std::path::PathBuf,
}

pub type Handler = Box<
    dyn Fn(&mut RunContext, &Value) -> std::result::Result<ToolExecutionResult, ToolExecError>
        + Send
        + Sync,
>;

pub struct RegisteredCapability {
    pub manifest: CapabilityManifest,
    pub handlers: HashMap<String, Handler>,
}

#[derive(Default)]
pub struct CapabilityRegistry {
    caps: BTreeMap<String, RegisteredCapability>,
    op_index: HashMap<String, (String, CapabilityOperation)>,
    pub activations: Mutex<Vec<String>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, manifest: CapabilityManifest, handlers: HashMap<String, Handler>) {
        for op in &manifest.operations {
            self.op_index
                .insert(op.id.clone(), (manifest.id.clone(), op.clone()));
        }
        self.caps.insert(
            manifest.id.clone(),
            RegisteredCapability { manifest, handlers },
        );
    }

    pub fn discover(
        &self,
        query: &str,
        limit: usize,
        filters: Option<&Value>,
    ) -> Vec<CapabilityManifest> {
        let terms: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|t| t.to_string())
            .collect();
        let mut results = Vec::new();
        for cap in self.caps.values() {
            let haystack =
                format!("{} {}", cap.manifest.id, cap.manifest.description).to_lowercase();
            if !terms.is_empty() && !terms.iter().all(|t| haystack.contains(t)) {
                continue;
            }
            if let Some(f) = filters {
                if let Some(risk) = f.get("risk_class").and_then(|r| r.as_str()) {
                    if serde_json::to_string(&cap.manifest.risk_class)
                        .unwrap_or_default()
                        .trim_matches('"')
                        != risk
                    {
                        continue;
                    }
                }
            }
            results.push(cap.manifest.clone());
        }
        results.truncate(limit);
        results
    }

    pub fn inspect(&self, capability_ids: &[String]) -> Result<Vec<CapabilityManifest>> {
        let mut out = Vec::new();
        for cid in capability_ids {
            let cap = self
                .caps
                .get(cid)
                .ok_or_else(|| SfaiError::NotFound(format!("unknown capability: {cid}")))?;
            out.push(cap.manifest.clone());
        }
        Ok(out)
    }

    /// Metadata-only: loading operation schemas confers no permission.
    pub fn activate(&self, capability_id: &str) -> Result<Vec<CapabilityOperation>> {
        let cap = self
            .caps
            .get(capability_id)
            .ok_or_else(|| SfaiError::NotFound(format!("unknown capability: {capability_id}")))?;
        self.activations
            .lock()
            .expect("registry mutex poisoned")
            .push(capability_id.to_string());
        Ok(cap.manifest.operations.clone())
    }

    pub fn resolve_operation(
        &self,
        operation_id: &str,
    ) -> Result<(CapabilityManifest, CapabilityOperation)> {
        let (cap_id, op) = self
            .op_index
            .get(operation_id)
            .ok_or_else(|| SfaiError::NotFound(format!("unknown operation: {operation_id}")))?;
        let cap = self
            .caps
            .get(cap_id)
            .ok_or_else(|| SfaiError::NotFound(format!("unknown capability: {cap_id}")))?;
        Ok((cap.manifest.clone(), op.clone()))
    }

    pub fn handler_for(&self, operation_id: &str) -> Result<&Handler> {
        for cap in self.caps.values() {
            if let Some(handler) = cap.handlers.get(operation_id) {
                return Ok(handler);
            }
        }
        Err(SfaiError::NotFound(format!(
            "no handler registered for operation: {operation_id}"
        )))
    }

    pub fn all_operations(&self) -> Vec<CapabilityOperation> {
        self.op_index.values().map(|(_, op)| op.clone()).collect()
    }
}
