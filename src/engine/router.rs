use super::compute_broker::ComputeBroker;
use super::cost::CostCalculator;
use super::hardware::HardwareProfile;
use super::llm::{LlmClient, LlmMessage, LlmRequest, LlmResponse};
use super::quality::QualityTracker;
use super::telemetry::{LlmCall, TelemetryLogger};
use crate::config::{LlmConfig, ModelTier};
use anyhow::Result;
use std::sync::Arc;
use tokio::time::Instant;

#[derive(Debug, Clone)]
pub struct ModelRouter {
    pub llm_client: LlmClient,
    pub light: ModelTier,
    pub balanced: ModelTier,
    pub full: ModelTier,
    pub fallback_chain: Vec<String>,
    pub telemetry: Option<Arc<TelemetryLogger>>,
    pub cost_calc: CostCalculator,
    pub quality: Arc<QualityTracker>,
    pub broker: Option<Arc<std::sync::Mutex<ComputeBroker>>>,
}

impl ModelRouter {
    pub fn from_llm_config(
        llm: &LlmConfig,
        light: ModelTier,
        balanced: ModelTier,
        full: ModelTier,
        telemetry_path: Option<std::path::PathBuf>,
    ) -> Self {
        let provider = match llm.provider {
            crate::config::LlmProvider::Ollama => super::llm::LlmProvider::Ollama,
            crate::config::LlmProvider::Groq => super::llm::LlmProvider::Groq,
            crate::config::LlmProvider::OpenRouter => super::llm::LlmProvider::OpenRouter,
            crate::config::LlmProvider::OpenAI => super::llm::LlmProvider::OpenAI,
            crate::config::LlmProvider::Anthropic => super::llm::LlmProvider::Anthropic,
            crate::config::LlmProvider::Kimi => super::llm::LlmProvider::Kimi,
            crate::config::LlmProvider::Glm => super::llm::LlmProvider::Glm,
            crate::config::LlmProvider::Lightning => super::llm::LlmProvider::Lightning,
            crate::config::LlmProvider::Custom => super::llm::LlmProvider::Custom,
        };

        let client = LlmClient::new(
            &llm.base_url,
            llm.api_key.as_deref(),
            provider,
            llm.timeout_seconds,
            llm.max_retries,
        );

        let telemetry = telemetry_path.map(|p| Arc::new(TelemetryLogger::new(p)));
        let cost_calc = CostCalculator::new().with_provider(llm.provider.as_str());
        let app_config = crate::config::AppConfig {
            llm: llm.clone(),
            ..Default::default()
        };
        let broker = ComputeBroker::from_config(&app_config);

        Self {
            llm_client: client,
            light,
            balanced,
            full,
            fallback_chain: vec![
                llm.provider.as_str().to_string(),
                "groq".to_string(),
                "openrouter".to_string(),
            ],
            telemetry,
            cost_calc,
            quality: Arc::new(QualityTracker::new()),
            broker: Some(Arc::new(std::sync::Mutex::new(broker))),
        }
    }

    pub fn tier(&self, tier_name: &str) -> &ModelTier {
        match tier_name {
            "light" => &self.light,
            "balanced" => &self.balanced,
            "full" => &self.full,
            _ => &self.balanced,
        }
    }

    pub fn model_for_task(&self, task: &str) -> &ModelTier {
        let base_tier = match task {
            "scoring" | "role_inference" => "light",
            "tailoring" | "cover_letter" | "deep_research" => "balanced",
            "review" | "final_polish" => "full",
            _ => "balanced",
        };

        if self.should_escalate(task) {
            tracing::info!("RLHF Self-Optimization: Escalating tier for task '{}' based on user feedback metrics", task);
            match base_tier {
                "light" => &self.balanced,
                "balanced" => &self.full,
                _ => &self.full,
            }
        } else {
            self.tier(base_tier)
        }
    }

    pub async fn chat(&self, messages: Vec<LlmMessage>, tier: &ModelTier) -> Result<LlmResponse> {
        self.chat_with_task(messages, tier, "unknown").await
    }

    pub async fn chat_with_task(
        &self,
        messages: Vec<LlmMessage>,
        tier: &ModelTier,
        task: &str,
    ) -> Result<LlmResponse> {
        let optimized = HardwareProfile::global().tier_for_hardware(tier);
        let start = Instant::now();
        let result = self
            .llm_client
            .chat(LlmRequest {
                model: optimized.model.clone(),
                messages,
                temperature: 0.2,
                max_tokens: std::cmp::min(2048, optimized.context_tokens),
                stream: false,
            })
            .await;

        let latency = start.elapsed().as_millis();

        if let Some(broker) = &self.broker {
            if let Ok(mut broker) = broker.try_lock() {
                if let Ok(resp) = &result {
                    broker.observe_quota_from_headers(
                        &self.llm_client.provider.to_string(),
                        &resp.headers,
                    );
                }
            }
        }

        if let Some(ref telemetry) = self.telemetry {
            match &result {
                Ok(resp) => {
                    let cost = self.cost_calc.calculate(
                        &self.llm_client.provider.to_string(),
                        &resp.model,
                        resp.prompt_tokens,
                        resp.completion_tokens,
                    );
                    let call_id = format!("{}-{}", task, chrono::Utc::now().timestamp_millis());
                    let _ = telemetry.record_call(&LlmCall {
                        call_id: call_id.clone(),
                        ts: chrono::Utc::now().to_rfc3339(),
                        provider: self.llm_client.provider.to_string(),
                        model: resp.model.clone(),
                        task: task.to_string(),
                        prompt_tokens: resp.prompt_tokens,
                        completion_tokens: resp.completion_tokens,
                        latency_ms: latency,
                        cost_usd: cost.total_usd,
                        success: true,
                        error: None,
                        quality: None,
                        edit_distance: None,
                    });
                }
                Err(e) => {
                    let call_id = format!("{}-{}", task, chrono::Utc::now().timestamp_millis());
                    let _ = telemetry.record_call(&LlmCall {
                        call_id,
                        ts: chrono::Utc::now().to_rfc3339(),
                        provider: self.llm_client.provider.to_string(),
                        model: optimized.model.clone(),
                        task: task.to_string(),
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        latency_ms: latency,
                        cost_usd: 0.0,
                        success: false,
                        error: Some(e.to_string()),
                        quality: None,
                        edit_distance: None,
                    });
                }
            }
        }

        result
    }

    pub async fn chat_with_fallback(
        &self,
        messages: Vec<LlmMessage>,
        tier: &ModelTier,
        task: &str,
    ) -> Result<LlmResponse> {
        let primary = self.llm_client.provider.to_string();

        // Phase 1 Compute Broker: let the broker choose the best fallback
        // provider based on observed quota and local-first policy. We only
        // use a broker-recommended provider if it differs from the primary
        // and from any already-tried provider.
        let mut tried = std::collections::HashSet::new();
        tried.insert(primary.clone());

        if let Some(broker) = &self.broker {
            let profile = broker
                .lock()
                .ok()
                .and_then(|b| b.route_task(task, false).cloned());
            if let Some(profile) = profile {
                if profile.name != primary && !profile.allow_paid {
                    tracing::info!("ComputeBroker selected fallback: {}", profile.name);
                    tried.insert(profile.name.clone());
                    if let Ok(resp) = self
                        .chat_via_provider(messages.clone(), tier, task, &profile.name)
                        .await
                    {
                        return Ok(resp);
                    }
                }
            }
        }

        for fallback in &self.fallback_chain {
            if !tried.insert(fallback.clone()) {
                continue;
            }
            tracing::warn!("Provider {} failed, trying fallback: {}", primary, fallback);
            if let Ok(resp) = self
                .chat_via_provider(messages.clone(), tier, task, fallback)
                .await
            {
                return Ok(resp);
            }
        }

        Err(anyhow::anyhow!(
            "All providers failed (primary: {}, fallbacks: {:?})",
            primary,
            self.fallback_chain
        ))
    }

    async fn chat_via_provider(
        &self,
        messages: Vec<LlmMessage>,
        tier: &ModelTier,
        task: &str,
        provider: &str,
    ) -> Result<LlmResponse> {
        let client = LlmClient::new(
            &self.llm_client.base_url,
            self.llm_client.api_key.as_deref(),
            match provider {
                "groq" => super::llm::LlmProvider::Groq,
                "openrouter" => super::llm::LlmProvider::OpenRouter,
                "ollama" => super::llm::LlmProvider::Ollama,
                "kimi" => super::llm::LlmProvider::Kimi,
                "glm" => super::llm::LlmProvider::Glm,
                "lightning" => super::llm::LlmProvider::Lightning,
                "openai" => super::llm::LlmProvider::OpenAI,
                "anthropic" => super::llm::LlmProvider::Anthropic,
                _ => super::llm::LlmProvider::Custom,
            },
            self.llm_client.timeout.as_secs(),
            self.llm_client.max_retries,
        );

        let optimized = HardwareProfile::global().tier_for_hardware(tier);
        let mut cost_calc = self.cost_calc.clone();
        cost_calc.set_provider(provider);

        let start = std::time::Instant::now();
        let result = client
            .chat(LlmRequest {
                model: optimized.model.clone(),
                messages,
                temperature: 0.2,
                max_tokens: std::cmp::min(2048, optimized.context_tokens),
                stream: false,
            })
            .await;

        let latency = start.elapsed().as_millis();

        if let Ok(ref resp) = result {
            if let Some(ref telemetry) = self.telemetry {
                let cost = cost_calc.calculate(
                    provider,
                    &resp.model,
                    resp.prompt_tokens,
                    resp.completion_tokens,
                );
                let call = LlmCall {
                    call_id: format!(
                        "{}-{}-fallback",
                        task,
                        chrono::Utc::now().timestamp_millis()
                    ),
                    ts: chrono::Utc::now().to_rfc3339(),
                    provider: provider.to_string(),
                    model: resp.model.clone(),
                    task: format!("{} (fallback)", task),
                    prompt_tokens: resp.prompt_tokens,
                    completion_tokens: resp.completion_tokens,
                    latency_ms: latency,
                    cost_usd: cost.total_usd,
                    success: true,
                    error: None,
                    quality: None,
                    edit_distance: None,
                };
                let _ = telemetry.record_call(&call);
            }
        }

        if let Some(broker) = &self.broker {
            if let Ok(mut broker) = broker.try_lock() {
                if let Ok(resp) = &result {
                    broker.observe_quota_from_headers(provider, &resp.headers);
                }
            }
        }

        result
    }

    pub async fn chat_custom(
        &self,
        model: &str,
        messages: Vec<LlmMessage>,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<LlmResponse> {
        let request = LlmRequest {
            model: model.to_string(),
            messages,
            temperature,
            max_tokens,
            stream: false,
        };
        self.llm_client.chat(request).await
    }

    pub fn optimized_tier(&self, tier_name: &str) -> ModelTier {
        let profile = HardwareProfile::global();
        let tier = self.tier(tier_name);
        profile.tier_for_hardware(tier)
    }

    pub fn recommended_batch_size(&self, tier_name: &str) -> usize {
        let profile = HardwareProfile::global();
        let tier = self.tier(tier_name);
        if !profile.has_gpu && tier.cpu_ok {
            tier.recommended_batch / 2
        } else {
            tier.recommended_batch
        }
    }

    pub fn print_hardware_hints(&self) {
        let profile = HardwareProfile::global();
        for hint in profile.cpu_optimization_hints() {
            tracing::info!("HW: {}", hint);
        }
    }

    pub fn record_quality(
        &self,
        call_id: &str,
        quality: Option<f64>,
        edit_distance: Option<usize>,
    ) {
        if let Some(ref telemetry) = self.telemetry {
            let _ = telemetry.record_quality(call_id, quality, edit_distance);
        }
        self.quality.record(crate::engine::quality::QualityRecord {
            call_id: call_id.to_string(),
            task: "unknown".to_string(),
            provider: self.llm_client.provider.to_string(),
            model: "unknown".to_string(),
            edit_distance,
            accepted: edit_distance.map(|d| d < 50).unwrap_or(true),
            confidence_before: quality.unwrap_or(0.0),
            confidence_after: quality.unwrap_or(0.0),
        });
    }

    pub fn quality_stats(&self, task: &str) -> crate::engine::quality::QualityStats {
        self.quality.stats_for(task)
    }

    pub fn should_escalate(&self, task: &str) -> bool {
        self.quality.should_escalate(task)
    }

    pub fn avg_latency_ms(&self, provider: &str, window_hours: i64) -> Result<f64> {
        if let Some(ref telemetry) = self.telemetry {
            telemetry.avg_latency_ms(provider, window_hours)
        } else {
            Ok(0.0)
        }
    }
}
