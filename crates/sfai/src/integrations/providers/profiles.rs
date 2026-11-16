//! Provider profiles for OpenAI-compatible endpoints.
//!
//! First-class providers: DeepSeek (default), Qwen (DashScope compatible mode),
//! Xiaomi MiMo (endpoint via env until vendor docs are fixed), and a generic
//! `custom` profile for any OpenAI-compatible endpoint.
//!
//! Prices are **approximate/default** values and are always overridable
//! (`--price-in` / `--price-out`, or `SFAI_*` env). Costs in reports are only as
//! good as the configured price table (ADR-019).

use crate::error::{Result, SfaiError};
use serde::Serialize;

pub const DEFAULT_DEEPSEEK_BASE: &str = "https://api.deepseek.com/v1";
pub const DEFAULT_DEEPSEEK_MODEL: &str = "deepseek-chat";
pub const DEFAULT_QWEN_BASE: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";
pub const DEFAULT_QWEN_MODEL: &str = "qwen-plus";

#[derive(Debug, Clone, Serialize)]
pub struct ProviderProfile {
    pub name: String,
    pub base_url: String,
    pub model: String,
    /// Environment variables checked in order for the API key.
    pub api_key_envs: Vec<String>,
    pub price_input_per_million: f64,
    pub price_output_per_million: f64,
    pub note: String,
}

fn env_non_empty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

impl ProviderProfile {
    pub fn api_key(&self) -> Option<String> {
        for env_name in &self.api_key_envs {
            if let Some(key) = env_non_empty(env_name) {
                return Some(key);
            }
        }
        None
    }

    pub fn is_configured(&self) -> bool {
        !self.base_url.is_empty() && !self.model.is_empty() && self.api_key().is_some()
    }

    pub fn with_overrides(
        mut self,
        model: Option<&str>,
        price_in: Option<f64>,
        price_out: Option<f64>,
    ) -> Self {
        if let Some(model) = model.filter(|m| !m.trim().is_empty()) {
            self.model = model.to_string();
        }
        if let Some(price) = price_in {
            self.price_input_per_million = price;
        }
        if let Some(price) = price_out {
            self.price_output_per_million = price;
        }
        self
    }

    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "base_url": self.base_url,
            "model": self.model,
            "api_key_envs": self.api_key_envs,
            "configured": self.is_configured(),
            "price_input_per_million": self.price_input_per_million,
            "price_output_per_million": self.price_output_per_million,
            "note": self.note,
        })
    }
}

pub fn builtin_profiles() -> Vec<ProviderProfile> {
    vec![
        ProviderProfile {
            name: "deepseek".into(),
            base_url: env_non_empty("DEEPSEEK_BASE_URL").unwrap_or_else(|| DEFAULT_DEEPSEEK_BASE.into()),
            model: env_non_empty("DEEPSEEK_MODEL").unwrap_or_else(|| DEFAULT_DEEPSEEK_MODEL.into()),
            api_key_envs: vec!["DEEPSEEK_API_KEY".into(), "SFAI_MODEL_API_KEY".into()],
            // Approximate public prices; override with --price-in/--price-out.
            price_input_per_million: 0.27,
            price_output_per_million: 1.10,
            note: "DeepSeek official OpenAI-compatible API (prices approximate)".into(),
        },
        ProviderProfile {
            name: "qwen".into(),
            base_url: env_non_empty("QWEN_BASE_URL")
                .unwrap_or_else(|| DEFAULT_QWEN_BASE.into()),
            model: env_non_empty("QWEN_MODEL").unwrap_or_else(|| DEFAULT_QWEN_MODEL.into()),
            api_key_envs: vec![
                "DASHSCOPE_API_KEY".into(),
                "QWEN_API_KEY".into(),
                "SFAI_MODEL_API_KEY".into(),
            ],
            price_input_per_million: 0.40,
            price_output_per_million: 1.20,
            note: "Alibaba DashScope OpenAI-compatible mode (prices approximate)".into(),
        },
        ProviderProfile {
            name: "mimo".into(),
            base_url: env_non_empty("XIAOMI_MIMO_BASE_URL")
                .or_else(|| env_non_empty("MIMO_BASE_URL"))
                .unwrap_or_default(),
            model: env_non_empty("XIAOMI_MIMO_MODEL")
                .or_else(|| env_non_empty("MIMO_MODEL"))
                .unwrap_or_else(|| "mimo".into()),
            api_key_envs: vec![
                "XIAOMI_MIMO_API_KEY".into(),
                "MIMO_API_KEY".into(),
                "SFAI_MODEL_API_KEY".into(),
            ],
            price_input_per_million: 0.0,
            price_output_per_million: 0.0,
            note: "Xiaomi MiMo: set XIAOMI_MIMO_BASE_URL / XIAOMI_MIMO_MODEL and prices until official defaults are recorded".into(),
        },
        ProviderProfile {
            name: "custom".into(),
            base_url: env_non_empty("SFAI_MODEL_BASE_URL").unwrap_or_default(),
            model: env_non_empty("SFAI_MODEL_NAME").unwrap_or_default(),
            api_key_envs: vec!["SFAI_MODEL_API_KEY".into()],
            price_input_per_million: 1.00,
            price_output_per_million: 3.00,
            note: "Generic OpenAI-compatible endpoint from SFAI_MODEL_* env".into(),
        },
    ]
}

pub fn resolve_provider(name: &str) -> Result<ProviderProfile> {
    builtin_profiles()
        .into_iter()
        .find(|p| p.name == name)
        .ok_or_else(|| {
            SfaiError::Validation(format!(
                "unknown provider {name:?}; choose from deepseek, qwen, mimo, custom"
            ))
        })
}

/// Resolve the provider for a CLI/API run:
/// - explicit `--provider` wins;
/// - otherwise a fully configured `custom` profile from `SFAI_MODEL_*`;
/// - otherwise an error asking for `--provider`.
pub fn resolve_for_cli(
    provider: Option<&str>,
    model: Option<&str>,
    price_in: Option<f64>,
    price_out: Option<f64>,
) -> Result<ProviderProfile> {
    if let Some(name) = provider {
        let profile = resolve_provider(name)?.with_overrides(model, price_in, price_out);
        if profile.base_url.is_empty() {
            return Err(SfaiError::Validation(format!(
                "provider {name:?} has no base URL configured; set its *_BASE_URL env (see docs/providers.md)"
            )));
        }
        if profile.api_key().is_none() {
            return Err(SfaiError::Validation(format!(
                "provider {name:?} is missing an API key; set {}",
                profile.api_key_envs.join(" or ")
            )));
        }
        return Ok(profile);
    }
    let custom = resolve_provider("custom")?;
    if !custom.base_url.is_empty() && !custom.model.is_empty() {
        return Ok(custom.with_overrides(model, price_in, price_out));
    }
    Err(SfaiError::Validation(
        "no provider selected: pass --provider deepseek|qwen|mimo|custom \
         (or set SFAI_MODEL_BASE_URL + SFAI_MODEL_NAME)"
            .into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_profiles_are_listed() {
        let names: Vec<String> = builtin_profiles().into_iter().map(|p| p.name).collect();
        assert_eq!(names, vec!["deepseek", "qwen", "mimo", "custom"]);
    }

    #[test]
    fn deepseek_defaults_are_sane() {
        let profile = resolve_provider("deepseek").unwrap();
        assert_eq!(profile.model, "deepseek-chat");
        assert!(profile.base_url.contains("api.deepseek.com"));
        assert!(profile
            .api_key_envs
            .contains(&"DEEPSEEK_API_KEY".to_string()));
    }

    #[test]
    fn overrides_apply() {
        let profile = resolve_provider("deepseek").unwrap().with_overrides(
            Some("deepseek-reasoner"),
            Some(0.5),
            Some(2.0),
        );
        assert_eq!(profile.model, "deepseek-reasoner");
        assert_eq!(profile.price_input_per_million, 0.5);
        assert_eq!(profile.price_output_per_million, 2.0);
    }

    #[test]
    fn unknown_provider_errors() {
        assert!(resolve_provider("nope").is_err());
    }
}
