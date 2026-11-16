//! OpenAI-compatible provider adapter (plan §2 models row).
//!
//! `normalize_response` maps the standard `/v1/chat/completions` tool-calling
//! shape into the single action-intent envelope. The live client speaks plain
//! HTTP(S) via `ureq`; unit tests exercise it against a local mock server so
//! core correctness still never depends on external network access.

use crate::contracts::{ActionIntent, ContextEnvelope, IntentKind, ModelResponse, ModelUsage};
use crate::core::action_intents::{minimal_tool_schema, parse_intent};
use crate::core::model_adapter::{ModelAdapter, ModelError};
use crate::telemetry::cost::{PRICE_PER_MILLION_INPUT, PRICE_PER_MILLION_OUTPUT};
use serde_json::{json, Value};
use std::time::Duration;

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
        cost_usd: 0.0, // priced below from reported token counts
    };
    let usage = ModelUsage {
        cost_usd: (usage.input_tokens as f64 * PRICE_PER_MILLION_INPUT
            + usage.output_tokens as f64 * PRICE_PER_MILLION_OUTPUT)
            / 1_000_000.0,
        ..usage
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
            kind: IntentKind::Respond,
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

    pub fn new(base_url: &str, api_key: &str, model_name: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            model_name: model_name.to_string(),
        }
    }

    fn task_id_from_envelope(envelope: &ContextEnvelope) -> String {
        envelope
            .root_index
            .get("task")
            .and_then(|r| r.strip_prefix("task://"))
            .and_then(|r| r.split('/').next())
            .unwrap_or("")
            .to_string()
    }

    fn request_body(&self, envelope: &ContextEnvelope) -> Value {
        json!({
            "model": self.model_name,
            "messages": [
                {"role": "system", "content": "Emit exactly one action-intent JSON object."},
                {"role": "user", "content": serde_json::to_string(envelope).unwrap_or_default()},
            ],
            "tools": minimal_tool_schema().iter().map(|t| json!({
                "type": "function",
                "function": {
                    "name": t["name"],
                    "description": t.get("description").cloned().unwrap_or(json!("")),
                    "parameters": {
                        "type": "object",
                        "properties": t.get("parameters").cloned().unwrap_or(json!({})),
                    },
                },
            })).collect::<Vec<_>>(),
        })
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

    fn decide(&mut self, envelope: &ContextEnvelope) -> Result<ModelResponse, ModelError> {
        if self.base_url.is_empty() || self.model_name.is_empty() {
            return Err(ModelError::Provider(
                "SFAI_MODEL_BASE_URL / SFAI_MODEL_NAME not configured; \
                 configure a provider or use a fake model offline"
                    .into(),
            ));
        }
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = self.request_body(envelope);
        let response = ureq::post(&url)
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .timeout(Duration::from_secs(120))
            .send_json(body)
            .map_err(|e| ModelError::Provider(format!("provider request failed: {e}")))?;
        let payload: Value = response
            .into_json()
            .map_err(|e| ModelError::Provider(format!("provider response is not JSON: {e}")))?;
        normalize_response(
            &self.model_id(),
            &payload,
            &Self::task_id_from_envelope(envelope),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn envelope() -> ContextEnvelope {
        ContextEnvelope {
            goal: "test goal".into(),
            pinned_constraints: vec![],
            task_state: json!({}),
            root_index: BTreeMap::new(),
            active_refs: vec![],
            recent_results: vec![],
            active_skills: vec![],
            budget: json!({}),
            retrieval_receipt: None,
        }
    }

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
        assert!(
            resp.usage.cost_usd > 0.0,
            "provider usage must be priced, not silently free"
        );
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

    #[test]
    fn unconfigured_adapter_fails_loudly() {
        let mut adapter = OpenAICompatibleAdapter::new("", "", "");
        assert!(adapter.decide(&envelope()).is_err());
    }

    /// Real HTTP path: the adapter POSTs a chat-completions request and the
    /// response is normalized — exercised against a local mock server.
    #[test]
    fn http_adapter_talks_to_mock_server() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            // Read the full request: headers may arrive in one packet but the
            // body can follow separately (the client streams it).
            let mut buf: Vec<u8> = Vec::new();
            let mut tmp = [0u8; 4096];
            loop {
                let n = stream.read(&mut tmp).expect("read request");
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
                let text = String::from_utf8_lossy(&buf).to_string();
                if let Some(hdr_end) = text.find("\r\n\r\n") {
                    let content_length = text[..hdr_end]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                        })
                        .unwrap_or(0);
                    if buf.len() >= hdr_end + 4 + content_length {
                        break;
                    }
                }
            }
            let request = String::from_utf8_lossy(&buf).to_string();
            assert!(request.starts_with("POST /chat/completions"));
            assert!(request.contains("\"model\":\"mock-model\""));
            let body = json!({
                "choices": [{"message": {"tool_calls": [{"function": {
                    "name": "capabilities.invoke",
                    "arguments": "{\"kind\":\"execute\",\"payload\":{\"operation_id\":\"web.search\",\"arguments\":{\"query\":\"x\"}}}"
                }}]}}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 5}
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });

        let mut adapter =
            OpenAICompatibleAdapter::new(&format!("http://{addr}"), "test-key", "mock-model");
        let response = adapter.decide(&envelope()).expect("decide");
        server.join().expect("server join");

        assert_eq!(response.intent.kind, crate::contracts::IntentKind::Execute);
        assert_eq!(response.intent.payload["operation_id"], "web.search");
        assert_eq!(response.usage.input_tokens, 10);
        assert_eq!(response.usage.output_tokens, 5);
    }
}
