//! Action-intent envelope: parsing and validation (plan §3.3).
//!
//! The model emits one intent per step. `reason_code` is an auditable short
//! label, not a demand for private chain-of-thought. Provider-native function
//! calls normalize into the same envelope.

use crate::contracts::{ActionIntent, IntentKind};
use crate::error::SfaiError;
use serde_json::{json, Value};

pub fn parse_intent(raw: &Value) -> Result<ActionIntent, SfaiError> {
    if !raw.is_object() {
        return Err(SfaiError::Intent("intent must be an object".into()));
    }
    serde_json::from_value(raw.clone()).map_err(|e| SfaiError::Intent(e.to_string()))
}

#[allow(clippy::too_many_arguments)]
pub fn intent(
    kind: IntentKind,
    task_id: &str,
    reason_code: &str,
    payload: Value,
    expected_refs: Vec<String>,
    idempotency_key: Option<String>,
) -> ActionIntent {
    ActionIntent {
        kind,
        task_id: task_id.to_string(),
        reason_code: reason_code.to_string(),
        payload,
        expected_refs,
        idempotency_key,
    }
}

/// Single tool exposed to real providers: exactly one action intent per step.
pub fn action_tool_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "submit_action",
            "description": "Submit exactly one action intent for the next step.",
            "parameters": {
                "type": "object",
                "properties": {
                    "kind": {
                        "type": "string",
                        "enum": ["discover", "retrieve", "activate", "execute", "verify", "respond", "ask_user", "pause", "finish"]
                    },
                    "task_id": {"type": "string"},
                    "reason_code": {"type": "string"},
                    "payload": {"type": "object"},
                    "expected_refs": {"type": "array", "items": {"type": "string"}},
                    "idempotency_key": {"type": "string"}
                },
                "required": ["kind", "task_id"]
            }
        }
    })
}

/// Minimal, stable meta-interface exposed to the model (plan §3.2).
pub fn minimal_tool_schema() -> Vec<Value> {
    vec![
        json!({
            "name": "capabilities.discover",
            "description": "Find capability families by query; returns id + description only.",
            "parameters": {"query": "string", "limit": "int?", "filters": "object?"},
        }),
        json!({
            "name": "capabilities.inspect",
            "description": "Load manifest metadata for specific capabilities.",
            "parameters": {"capability_ids": "array<string>"},
        }),
        json!({
            "name": "capabilities.activate",
            "description": "Load operation schemas into the working set. Confers no permission.",
            "parameters": {"capability_id": "string"},
        }),
        json!({
            "name": "capabilities.invoke",
            "description": "Execute one operation through the policy-aware broker.",
            "parameters": {"operation_id": "string", "arguments": "object", "idempotency_key": "string?"},
        }),
        json!({
            "name": "context.search",
            "description": "Search the context index. Returns hints, not truth.",
            "parameters": {"query": "string", "scope": "string?", "limit": "int?"},
        }),
        json!({
            "name": "context.read",
            "description": "Read a bounded excerpt by ref with provenance.",
            "parameters": {"ref_id": "string", "max_tokens": "int?"},
        }),
        json!({
            "name": "context.resolve",
            "description": "Resolve refs to provenance metadata.",
            "parameters": {"ref_ids": "array<string>"},
        }),
        json!({
            "name": "verify.run",
            "description": "Run verification against target refs and criteria.",
            "parameters": {"target_refs": "array<string>", "criteria_ref": "string?"},
        }),
        json!({
            "name": "state.checkpoint",
            "description": "Checkpoint task state with a note.",
            "parameters": {"note": "string?"},
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_intent() {
        let raw = json!({"kind": "respond", "task_id": "t1", "payload": {"text": "hi"}});
        let parsed = parse_intent(&raw).unwrap();
        assert_eq!(parsed.kind, IntentKind::Respond);
        assert_eq!(parsed.reason_code, "unspecified");
    }

    #[test]
    fn rejects_bad_kind_and_extra_fields() {
        assert!(parse_intent(&json!({"kind": "explode", "task_id": "t"})).is_err());
        assert!(parse_intent(&json!({"kind": "respond", "task_id": "t", "x": 1})).is_err());
        assert!(parse_intent(&json!({"task_id": "t"})).is_err());
        assert!(parse_intent(&json!("nope")).is_err());
    }

    #[test]
    fn tool_schema_names_unique() {
        let schema = minimal_tool_schema();
        let mut names: Vec<String> = schema
            .iter()
            .map(|s| s["name"].as_str().unwrap().to_string())
            .collect();
        names.sort();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
    }
}
