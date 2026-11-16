//! Real read-only filesystem tool (plan §15: one read-only search/file tool).
//!
//! Reads are confined to a sandbox root; traversal outside it is rejected.

use crate::capabilities::registry::{
    CapabilityRegistry, Handler, ToolExecError, ToolExecutionResult,
};
use crate::contracts::{CapabilityManifest, CapabilityOperation, RiskClass};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

pub fn register_readonly_fs(registry: &mut CapabilityRegistry, root: PathBuf) {
    let handler: Handler = Box::new(move |_ctx, args| {
        let rel = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
        let target = root.join(rel);
        let resolved = target.canonicalize().map_err(|_| ToolExecError {
            message: format!("no such file: {rel}"),
            error_code: "FILE_NOT_FOUND".into(),
        })?;
        let root_resolved = root.canonicalize().unwrap_or_else(|_| root.clone());
        if !resolved.starts_with(&root_resolved) {
            return Err(ToolExecError {
                message: format!("path escapes sandbox root: {rel}"),
                error_code: "SANDBOX_VIOLATION".into(),
            });
        }
        let text = std::fs::read_to_string(&resolved).map_err(|e| ToolExecError {
            message: e.to_string(),
            error_code: "FILE_NOT_FOUND".into(),
        })?;
        Ok(ToolExecutionResult {
            preview: text.chars().take(400).collect(),
            result_data: json!({"text": text, "path": rel}),
            ..Default::default()
        })
    });

    let manifest = CapabilityManifest {
        id: "filesystem.readonly".into(),
        version: "0.1.0".into(),
        description: "Read files inside the sandbox root".into(),
        operations: vec![CapabilityOperation {
            id: "fs.read".into(),
            description: "Read a text file relative to the sandbox root.".into(),
            input_schema: json!({"path": "string"}),
            risk_class: RiskClass::ReadOnly,
            side_effects: vec![],
        }],
        permissions: BTreeMap::from([("filesystem".to_string(), true)]),
        risk_class: RiskClass::ReadOnly,
        output_contract: BTreeMap::new(),
        healthcheck: None,
    };
    registry.register(manifest, HashMap::from([("fs.read".to_string(), handler)]));
}
