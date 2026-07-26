use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;
use tracing::{info, warn};

static OLLAMA_AVAILABLE_CACHE: AtomicU64 = AtomicU64::new(0);
const CACHE_TTL_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<LlmMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    /// HTTP response headers captured from the provider, when available.
    /// Used by the compute broker to observe rate-limit quotas.
    /// Not serialised to JSON.
    #[serde(skip)]
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct LlmClient {
    pub base_url: String,
    pub api_key: Option<String>,
    pub provider: LlmProvider,
    pub timeout: Duration,
    pub max_retries: u32,
    pub circuit_breaker_failures: Arc<std::sync::atomic::AtomicU32>,
    pub circuit_breaker_open_until: Arc<std::sync::atomic::AtomicU64>,
}

impl LlmClient {
    pub fn new(
        base_url: &str,
        api_key: Option<&str>,
        provider: LlmProvider,
        timeout_secs: u64,
        max_retries: u32,
    ) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.map(|s| s.to_string()),
            provider,
            timeout: Duration::from_secs(timeout_secs),
            max_retries,
            circuit_breaker_failures: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            circuit_breaker_open_until: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub async fn chat(&self, request: LlmRequest) -> Result<LlmResponse> {
        if self.is_circuit_open() {
            warn!("LLM circuit breaker open - provider unavailable");
            anyhow::bail!(
                "LLM circuit breaker open - provider unavailable. Wait or switch provider."
            );
        }

        let mut last_error = None;
        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let backoff = Duration::from_secs(2u64.pow(attempt.min(6)));
                warn!(
                    "LLM request failed (attempt {}), backing off {:?}",
                    attempt, backoff
                );
                sleep(backoff).await;
            }

            match self.try_chat(&request).await {
                Ok(resp) => {
                    if attempt > 0 {
                        info!("LLM request succeeded on attempt {}", attempt + 1);
                    }
                    self.record_success();
                    return Ok(resp);
                }
                Err(e) => {
                    warn!("LLM attempt {} failed: {}", attempt + 1, e);
                    self.record_failure();
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("LLM request failed after retries")))
    }

    fn is_circuit_open(&self) -> bool {
        let until = self.circuit_breaker_open_until.load(Ordering::Relaxed);
        if until == 0 {
            return false;
        }
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if now >= until {
            self.circuit_breaker_failures.store(0, Ordering::Relaxed);
            self.circuit_breaker_open_until.store(0, Ordering::Relaxed);
            return false;
        }
        true
    }

    fn record_failure(&self) {
        let failures = self
            .circuit_breaker_failures
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        if failures >= 5 {
            let until = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 60;
            self.circuit_breaker_open_until
                .store(until, Ordering::Relaxed);
            warn!(
                "LLM circuit breaker opened for 60s after {} failures",
                failures
            );
        }
    }

    fn record_success(&self) {
        self.circuit_breaker_failures.store(0, Ordering::Relaxed);
        self.circuit_breaker_open_until.store(0, Ordering::Relaxed);
    }

    pub async fn is_available(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cached = OLLAMA_AVAILABLE_CACHE.load(Ordering::Relaxed);
        if cached != 0 && now - cached < CACHE_TTL_SECS {
            return true;
        }

        let result = match self.provider {
            LlmProvider::Ollama => self.check_ollama_available().await,
            _ => true,
        };

        if result {
            OLLAMA_AVAILABLE_CACHE.store(now, Ordering::Relaxed);
        } else {
            OLLAMA_AVAILABLE_CACHE.store(0, Ordering::Relaxed);
        }
        result
    }

    async fn check_ollama_available(&self) -> bool {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        let url = format!("{}/api/tags", self.base_url);
        let result = client.get(&url).send().await;
        result.map(|r| r.status().is_success()).unwrap_or(false)
    }

    async fn try_chat(&self, request: &LlmRequest) -> Result<LlmResponse> {
        match self.provider {
            LlmProvider::Ollama => self.chat_ollama(request).await,
            LlmProvider::Kimi => {
                self.chat_openai_compat(request, "/v1/chat/completions")
                    .await
            }
            LlmProvider::Glm => {
                self.chat_openai_compat(request, "/api/paas/v4/chat/completions")
                    .await
            }
            LlmProvider::Groq => {
                self.chat_openai_compat(request, "/openai/v1/chat/completions")
                    .await
            }
            LlmProvider::Lightning => {
                self.chat_openai_compat(request, "/v1/chat/completions")
                    .await
            }
            LlmProvider::OpenRouter
            | LlmProvider::OpenAI
            | LlmProvider::Anthropic
            | LlmProvider::Custom => {
                self.chat_openai_compat(request, "/v1/chat/completions")
                    .await
            }
        }
    }

    async fn chat_ollama(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let url = format!("{}/api/chat", self.base_url);
        let client = reqwest::Client::builder().timeout(self.timeout).build()?;

        #[derive(Serialize)]
        struct OllamaRequest<'a> {
            model: &'a str,
            messages: Vec<LlmMessage>,
            stream: bool,
            options: OllamaOptions,
        }

        #[derive(Serialize, Default)]
        struct OllamaOptions {
            num_ctx: u32,
            temperature: f32,
        }

        let body = OllamaRequest {
            model: &request.model,
            messages: request.messages.clone(),
            stream: false,
            options: OllamaOptions {
                num_ctx: 4096,
                temperature: request.temperature,
            },
        };

        let resp = client.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await?;
            anyhow::bail!("Ollama error {}: {}", status, text);
        }

        #[derive(Debug, Deserialize)]
        struct OllamaResponse {
            message: OllamaMessage,
            #[allow(dead_code)]
            done: bool,
        }

        #[derive(Debug, Deserialize)]
        struct OllamaMessage {
            content: String,
        }

        let parsed: OllamaResponse = resp.json().await?;
        Ok(LlmResponse {
            content: parsed.message.content,
            model: request.model.clone(),
            prompt_tokens: 0,
            completion_tokens: 0,
            headers: HashMap::new(),
        })
    }

    async fn chat_openai_compat(&self, request: &LlmRequest, path: &str) -> Result<LlmResponse> {
        let url = format!("{}{}", self.base_url, path);
        let client = reqwest::Client::builder().timeout(self.timeout).build()?;

        #[derive(Serialize)]
        struct OpenAIRequest<'a> {
            model: &'a str,
            messages: Vec<LlmMessage>,
            temperature: f32,
            max_tokens: u32,
        }

        let body = OpenAIRequest {
            model: &request.model,
            messages: request.messages.clone(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let mut req = client.post(&url).json(&body);
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await?;
            // Issue #6 — 401 from a hosted provider almost always means
            // (a) wrong key, (b) wrong env-var name, or (c) the account
            // is configured for a different-prefix key. Surface the most
            // actionable hint inline next to the raw body so the user
            // sees the fix path immediately.
            if status.as_u16() == 401 {
                let hint = match self.provider {
                    LlmProvider::Lightning => "\n   Hint: confirm LIGHTNING_API_KEY in .env (issue #6: Lighting previously emitted only the raw 401 body, see studio.lighter.ai for the key)",
                    LlmProvider::Groq => "\n   Hint: confirm GROQ_API_KEY in .env (console.groq.com)",
                    LlmProvider::Kimi => "\n   Hint: confirm KIMI_API_KEY in .env (platform.moonshot.cn)",
                    LlmProvider::Glm => "\n   Hint: confirm GLM_API_KEY in .env (bigmodel.cn / Zhipu)",
                    LlmProvider::OpenAI => "\n   Hint: confirm OPENAI_API_KEY in .env",
                    LlmProvider::Anthropic => "\n   Hint: confirm ANTHROPIC_API_KEY in .env",
                    LlmProvider::OpenRouter => "\n   Hint: confirm OPENROUTER_API_KEY in .env",
                    LlmProvider::Ollama => "\n   Hint: Ollama 401 - check OLLAMA_BASE_URL allows anonymous access or that you've configured remote-ollama auth",
                    LlmProvider::Custom => "\n   Hint: check the api_key env var configured in .env for the Custom provider",
                };
                anyhow::bail!(
                    "OpenAI-compatible error {}\n   Body: {}{}",
                    status,
                    text,
                    hint
                );
            }
            anyhow::bail!("OpenAI-compatible error {}: {}", status, text);
        }

        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<OpenAIChoice>,
            usage: Option<OpenAIUsage>,
        }

        #[derive(Debug, Deserialize)]
        struct OpenAIChoice {
            message: RemoteMessage,
        }

        #[derive(Debug, Deserialize)]
        struct RemoteMessage {
            content: String,
        }

        #[derive(Deserialize)]
        struct OpenAIUsage {
            prompt_tokens: u32,
            completion_tokens: u32,
        }

        let headers = resp.headers().clone();
        let parsed: OpenAIResponse = resp.json().await?;
        let choice = parsed.choices.first().context("Empty choices from LLM")?;

        Ok(LlmResponse {
            content: choice.message.content.clone(),
            model: request.model.clone(),
            prompt_tokens: parsed.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0),
            completion_tokens: parsed
                .usage
                .as_ref()
                .map(|u| u.completion_tokens)
                .unwrap_or(0),
            headers: header_map_to_hash_map(&headers),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LlmProvider {
    Ollama,
    Groq,
    OpenRouter,
    OpenAI,
    Anthropic,
    Kimi,
    Glm,
    Lightning,
    Custom,
}

fn header_map_to_hash_map(headers: &reqwest::header::HeaderMap) -> HashMap<String, String> {
    headers
        .iter()
        .filter_map(|(k, v)| {
            let key = k.as_str().to_lowercase();
            let value = v.to_str().ok()?.to_string();
            Some((key, value))
        })
        .collect()
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmProvider::Ollama => write!(f, "ollama"),
            LlmProvider::Groq => write!(f, "groq"),
            LlmProvider::OpenRouter => write!(f, "openrouter"),
            LlmProvider::OpenAI => write!(f, "openai"),
            LlmProvider::Anthropic => write!(f, "anthropic"),
            LlmProvider::Kimi => write!(f, "kimi"),
            LlmProvider::Glm => write!(f, "glm"),
            LlmProvider::Lightning => write!(f, "lightning"),
            LlmProvider::Custom => write!(f, "custom"),
        }
    }
}
