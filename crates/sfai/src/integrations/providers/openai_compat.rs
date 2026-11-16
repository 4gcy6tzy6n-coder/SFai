//! OpenAI-compatible provider adapter (plan §2 models row).
//!
//! `normalize_response` maps the standard `/v1/chat/completions` tool-calling
//! shape into the single action-intent envelope and is unit-tested offline.
//! The live HTTP call is deliberately *not* implemented in v0: core
//! correctness must not depend on network access (M1 remaining item).

use crate::contracts::{ActionIntent, ContextEnvelope, ModelResponse, ModelUsage};
use crate::core::action_intents::parse_intent;
use crate::core::model_adapter::{ModelAdapter, ModelError};
use serde_json::{json, Value};

pub fn normalize_response(
    model_id: &str,
    payload: &Value,
    task_id: &str,
) -> Result<ModelResponse, ModelError> {
    let choice = payload
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| ModelError::Provider("no choices in provider response".into()))?;
    let message = choice.get("message").cloned().unwrap_or_else(|| json!({}));
    let usage_raw = payload.get("usage").cloned().unwrap_or_else(|| json!({}));
    let usage = ModelUsage {
        input_tokens: usage_raw
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        output_tokens: usage_raw
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cached_input_tokens: usage_raw
            .get("prompt_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cost_usd: 0.0,
    };
    let tool_calls = message
        .get("tool_calls")
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();
    let intent: ActionIntent = if let Some(call) = tool_calls.first() {
        let fn_call = call.get("function").cloned().unwrap_or_else(|| json!({}));
        let args_str = fn_call
            .get("arguments")
            .and_then(|a| a.as_str())
            .unwrap_or("{}");
        let mut args: Value = serde_json::from_str(args_str).map_err(|e| {
            ModelError::Provider(format!("function arguments are not valid JSON: {e}"))
        })?;
        if let Some(obj) = args.as_object_mut() {
            obj.entry("task_id")
                .or_insert_with(|| Value::String(task_id.to_string()));
        }
        parse_intent(&args)
            .map_err(|e| ModelError::Provider(format!("invalid intent from model: {e}")))?
    } else {
        ActionIntent {
            kind: crate::contracts::IntentKind::Respond,
            task_id: task_id.to_string(),
            reason_code: "direct_answer".into(),
            payload: json!({"text": message.get("content").and_then(|c| c.as_str()).unwrap_or("")}),
            expected_refs: vec![],
            idempotency_key: None,
        }
    };
    Ok(ModelResponse {
        model_id: model_id.to_string(),
        intent,
        usage,
        latency_ms: 0.0,
    })
}

pub struct OpenAICompatibleAdapter {
    pub base_url: String,
    pub api_key: String,
    pub model_name: String,
}

impl OpenAICompatibleAdapter {
    pub fn from_env() -> Self {
        Self {
            base_url: std::env::var("SFAI_MODEL_BASE_URL").unwrap_or_default(),
            api_key: std::env::var("SFAI_MODEL_API_KEY").unwrap_or_default(),
            model_name: std::env::var("SFAI_MODEL_NAME").unwrap_or_default(),
        }
    }
}

impl ModelAdapter for OpenAICompatibleAdapter {
    fn model_id(&self) -> String {
        format!(
            "openai-compat/{}",
            if self.model_name.is_empty() {
                "unset"
            } else {
                &self.model_name
            }
        )
    }

    fn decide(&mut self, _envelope: &ContextEnvelope) -> Result<ModelResponse, ModelError> {
        Err(ModelError::Provider(
            "live provider HTTP is not implemented in v0; use a fake model offline \
             (see docs/debug-reports for remaining M1 work)"
                .into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_tool_call_payload() {
        let payload = json!({
            "choices": [{"message": {"tool_calls": [{"function": {
                "name": "capabilities.invoke",
                "arguments": "{\"kind\":\"execute\",\"payload\":{\"operation_id\":\"web.search\",\"arguments\":{\"query\":\"x\"}}}"
            }}]}}],
            "usage": {"prompt_tokens": 100, "completion_tokens": 20}
        });
        let resp = normalize_response("m", &payload, "task_9").unwrap();
        assert_eq!(resp.intent.task_id, "task_9");
        assert_eq!(resp.intent.kind, crate::contracts::IntentKind::Execute);
        assert_eq!(resp.usage.input_tokens, 100);
    }

    #[test]
    fn normalizes_direct_answer() {
        let payload = json!({"choices": [{"message": {"content": "42"}}]});
        let resp = normalize_response("m", &payload, "task_9").unwrap();
        assert_eq!(resp.intent.kind, crate::contracts::IntentKind::Respond);
        assert_eq!(resp.intent.payload["text"], "42");
    }

    #[test]
    fn rejects_empty_choices() {
        assert!(normalize_response("m", &json!({}), "t").is_err());
    }
}
