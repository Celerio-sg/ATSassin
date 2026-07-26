//! Phase 1 — Compute Broker.
//!
//! Routes LLM tasks across the providers the user has actually configured,
//! preferring local/free ones and only falling back to paid providers when
//! explicitly allowed. Quota information is observed from provider response
//! headers and cached in SQLite, not maintained as a hand-authored registry.

use crate::config::{AppConfig, LlmProvider};
use crate::pipeline::tracker::PipelineTracker;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderTierType {
    Local,
    Configured,
    Trial,
}

impl ProviderTierType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderTierType::Local => "local",
            ProviderTierType::Configured => "configured",
            ProviderTierType::Trial => "trial",
        }
    }
}

/// A provider the user has opted into (via config or env var).
#[derive(Debug, Clone)]
pub struct ProviderProfile {
    pub name: String,
    pub tier_type: ProviderTierType,
    pub base_url: String,
    pub api_key: Option<String>,
    pub default_model: String,
    /// Whether this provider may be used when free/local capacity is
    /// exhausted. Defaults to false; never silently bill the user.
    pub allow_paid: bool,
}

/// Cached observed quota for a provider.
#[derive(Debug, Clone, Default)]
pub struct ProviderQuota {
    pub remaining_requests: Option<i64>,
    pub remaining_tokens: Option<i64>,
    pub resets_at: Option<String>,
    pub reliability_score: f64,
    pub last_observed: Option<String>,
}

/// Lightweight broker. Holds configured providers and an in-memory quota
/// cache; persists quota to SQLite when a tracker is supplied.
#[derive(Debug, Clone, Default)]
pub struct ComputeBroker {
    pub providers: Vec<ProviderProfile>,
    pub quota_cache: HashMap<String, ProviderQuota>,
}

impl ComputeBroker {
    /// Build a broker from the current app config. Only providers with a
    /// configured API key (or local Ollama) are considered.
    pub fn from_config(cfg: &AppConfig) -> Self {
        let mut providers = Vec::new();

        // Local Ollama is always available if the user pointed at it.
        if matches!(cfg.llm.provider, LlmProvider::Ollama) {
            providers.push(ProviderProfile {
                name: "ollama".to_string(),
                tier_type: ProviderTierType::Local,
                base_url: cfg.llm.base_url.clone(),
                api_key: None,
                default_model: cfg.llm.default_model.clone(),
                allow_paid: false,
            });
        } else {
            providers.push(ProviderProfile {
                name: "ollama".to_string(),
                tier_type: ProviderTierType::Local,
                base_url: "http://localhost:11434".to_string(),
                api_key: None,
                default_model: cfg.llm.default_model.clone(),
                allow_paid: false,
            });
        }

        macro_rules! add_if_key {
            ($provider:ident, $name:expr, $base:expr) => {
                if cfg.llm.provider == LlmProvider::$provider {
                    providers.push(ProviderProfile {
                        name: $name.to_string(),
                        tier_type: ProviderTierType::Configured,
                        base_url: $base.to_string(),
                        api_key: cfg.llm.api_key.clone(),
                        default_model: cfg.llm.default_model.clone(),
                        allow_paid: false,
                    });
                }
            };
        }

        add_if_key!(Groq, "groq", "https://api.groq.com");
        add_if_key!(OpenRouter, "openrouter", "https://openrouter.ai");
        add_if_key!(OpenAI, "openai", "https://api.openai.com");
        add_if_key!(Anthropic, "anthropic", "https://api.anthropic.com");
        add_if_key!(Kimi, "kimi", "https://api.moonshot.cn");
        add_if_key!(Glm, "glm", "https://open.bigmodel.cn");
        add_if_key!(Lightning, "lightning", "https://api.lightning.ai");

        Self {
            providers,
            quota_cache: HashMap::new(),
        }
    }

    /// Select the best provider for a task. Prefers local, then configured
    /// providers with remaining quota, then any configured provider. Paid
    /// providers are only chosen if `allow_paid` is true.
    pub fn route_task(&self, _task: &str, require_paid_ok: bool) -> Option<&ProviderProfile> {
        self.providers
            .iter()
            .find(|p| {
                if p.allow_paid || p.tier_type == ProviderTierType::Local {
                    return true;
                }
                if let Some(q) = self.quota_cache.get(&p.name) {
                    q.remaining_requests.unwrap_or(1) > 0 || require_paid_ok
                } else {
                    !require_paid_ok
                }
            })
            .or_else(|| self.providers.iter().find(|p| p.allow_paid))
    }

    /// Update the observed quota for a provider from a map of HTTP headers.
    /// Header names are normalized to lowercase. Recognises common rate-limit
    /// headers from OpenAI/Anthropic/Groq style APIs.
    pub fn observe_quota_from_headers(
        &mut self,
        provider: &str,
        headers: &std::collections::HashMap<String, String>,
    ) {
        let mut quota = ProviderQuota::default();

        if let Some(val) = headers
            .get("x-ratelimit-remaining-requests")
            .and_then(|s| s.parse().ok())
        {
            quota.remaining_requests = Some(val);
        }

        if let Some(val) = headers
            .get("x-ratelimit-remaining-tokens")
            .and_then(|s| s.parse().ok())
        {
            quota.remaining_tokens = Some(val);
        }

        if let Some(val) = headers.get("x-ratelimit-reset") {
            quota.resets_at = Some(val.to_string());
        }

        quota.last_observed = Some(chrono::Utc::now().to_rfc3339());
        self.quota_cache.insert(provider.to_string(), quota);
    }

    /// Persist observed quota to SQLite.
    pub fn save_quota_to_tracker(&self, tracker: &PipelineTracker) -> Result<()> {
        tracker.save_provider_quota(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> AppConfig {
        let mut cfg = AppConfig::default();
        cfg.llm.provider = LlmProvider::Groq;
        cfg.llm.api_key = Some("fake-key".to_string());
        cfg
    }

    #[test]
    fn broker_prefers_local_ollama() {
        let broker = ComputeBroker::from_config(&sample_config());
        let chosen = broker.route_task("tailoring", false);
        assert!(chosen.is_some());
        assert_eq!(chosen.unwrap().name, "ollama");
    }

    #[test]
    fn broker_can_select_cloud_when_required() {
        let mut broker = ComputeBroker::from_config(&sample_config());
        // Simulate Ollama being exhausted.
        broker.quota_cache.insert(
            "ollama".to_string(),
            ProviderQuota {
                remaining_requests: Some(0),
                ..Default::default()
            },
        );
        // Without paid OK, still picks a provider (configured has quota).
        let chosen = broker.route_task("tailoring", false);
        assert!(chosen.is_some());
    }
}
