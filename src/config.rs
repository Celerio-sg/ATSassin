use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub profile_path: PathBuf,
    pub database_path: PathBuf,
    pub llm: LlmConfig,
    pub tiers: TierConfig,
    pub scraping: ScrapingConfig,
    pub export: ExportConfig,
    #[serde(default)]
    pub preferences: JobPreferences,
}

/// User-set job search preferences. Every field is optional/"Any" by
/// default - filtering only ever removes jobs the user has explicitly told
/// us they don't want, never invents a preference the user didn't set.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobPreferences {
    /// Minimum acceptable compensation in USD/yr equivalent. Jobs with no
    /// parseable salary figure are never excluded by this filter - we can't
    /// honestly say they fail a check we couldn't evaluate.
    pub min_comp_usd: Option<u64>,
    #[serde(default)]
    pub employment_type: EmploymentTypePref,
    #[serde(default)]
    pub work_mode: WorkModePref,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum EmploymentTypePref {
    #[default]
    Any,
    FullTimeOnly,
    ContractOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum WorkModePref {
    #[default]
    Any,
    RemoteOnly,
    HybridOrRemote,
    OnsiteOk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    pub base_url: String,
    pub api_key: Option<String>,
    pub default_model: String,
    pub light_model: String,
    pub balanced_model: String,
    pub full_model: String,
    pub embed_model: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
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

impl LlmProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::Groq => "groq",
            Self::OpenRouter => "openrouter",
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Kimi => "kimi",
            Self::Glm => "glm",
            Self::Lightning => "lightning",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    pub light: ModelTier,
    pub balanced: ModelTier,
    pub full: ModelTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTier {
    pub model: String,
    pub quantization: String,
    pub context_tokens: u32,
    pub cpu_ok: bool,
    pub cpu_threads: Option<u8>,
    pub ram_min_gb: u8,
    pub score_threshold: f64,
    pub passes: u8,
    pub recommended_batch: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapingConfig {
    pub enabled: bool,
    pub boards: Vec<String>,
    pub max_results_per_board: usize,
    pub rate_limit_ms: u64,
    pub user_agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub pdf_template: String,
    pub cover_template: String,
    pub default_format: ExportFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Pdf,
    Docx,
    Markdown,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            profile_path: PathBuf::from("profile.md"),
            database_path: PathBuf::from("atsassin.db"),
            llm: LlmConfig::default(),
            tiers: TierConfig::default(),
            scraping: ScrapingConfig::default(),
            export: ExportConfig::default(),
            preferences: JobPreferences::default(),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::Ollama,
            base_url: "http://localhost:11434".to_string(),
            api_key: None,
            default_model: "qwen3.5:9b".to_string(),
            light_model: "qwen3.5:4b".to_string(),
            balanced_model: "qwen3.5:9b".to_string(),
            full_model: "qwen3.5:9b:q6".to_string(),
            embed_model: "nomic-embed-text".to_string(),
            timeout_seconds: 120,
            max_retries: 3,
        }
    }
}

impl Default for TierConfig {
    fn default() -> Self {
        Self {
            light: ModelTier {
                model: "qwen3.5:4b".to_string(),
                quantization: "Q4_K_M".to_string(),
                context_tokens: 4096,
                cpu_ok: true,
                cpu_threads: Some(4),
                ram_min_gb: 4,
                score_threshold: 0.6,
                passes: 1,
                recommended_batch: 64,
            },
            balanced: ModelTier {
                model: "qwen3.5:9b".to_string(),
                quantization: "Q4_K_M".to_string(),
                context_tokens: 8192,
                cpu_ok: true,
                cpu_threads: Some(6),
                ram_min_gb: 8,
                score_threshold: 0.7,
                passes: 3,
                recommended_batch: 32,
            },
            full: ModelTier {
                model: "qwen3.5:9b:q6".to_string(),
                quantization: "Q6_K".to_string(),
                context_tokens: 32768,
                cpu_ok: false,
                cpu_threads: None,
                ram_min_gb: 16,
                score_threshold: 0.75,
                passes: 5,
                recommended_batch: 16,
            },
        }
    }
}

impl Default for ScrapingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            // indeed/glassdoor are bot-protected and rarely return real results
            // (see Scraper::scrape_indeed/scrape_glassdoor); they remain
            // available via `--boards` but are no longer a default.
            boards: vec![
                "linkedin".to_string(),
                "seek".to_string(),
                "companies".to_string(),
                "social".to_string(),
            ],
            max_results_per_board: 50,
            rate_limit_ms: 1000,
            user_agent: "ATSassin/0.1 (local-first job search)".to_string(),
        }
    }
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            pdf_template: "templates/resume.html".to_string(),
            cover_template: "templates/cover_letter.html".to_string(),
            default_format: ExportFormat::Pdf,
        }
    }
}

impl AppConfig {
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let mut cfg = if path.exists() {
            let data = std::fs::read_to_string(path).context("Failed to read config")?;
            toml::from_str(&data).context("Failed to parse config")?
        } else {
            info!("Config not found at {:?}, using defaults", path);
            Self::default()
        };

        info!(
            "Loaded config: provider={:?}, base_url={}",
            cfg.llm.provider, cfg.llm.base_url
        );

        if let Ok(provider) = std::env::var("LLM_PROVIDER") {
            info!("Overriding provider from env: {}", provider);
            cfg.llm.provider = match provider.to_lowercase().as_str() {
                "ollama" => LlmProvider::Ollama,
                "groq" => {
                    cfg.llm.base_url = "https://api.groq.com".to_string();
                    cfg.llm.default_model = std::env::var("GROQ_MODEL")
                        .unwrap_or_else(|_| "llama-3.3-70b-versatile".to_string());
                    cfg.llm.light_model = std::env::var("GROQ_LIGHT_MODEL")
                        .unwrap_or_else(|_| "llama-3.1-8b-instant".to_string());
                    cfg.llm.balanced_model = std::env::var("GROQ_BALANCED_MODEL")
                        .unwrap_or_else(|_| "llama-3.3-70b-versatile".to_string());
                    cfg.llm.full_model = std::env::var("GROQ_FULL_MODEL")
                        .unwrap_or_else(|_| "llama-3.3-70b-versatile".to_string());
                    cfg.llm.api_key = std::env::var("GROQ_API_KEY").ok().filter(|v| !v.is_empty());
                    LlmProvider::Groq
                }
                "openrouter" => {
                    cfg.llm.base_url = "https://openrouter.ai".to_string();
                    cfg.llm.default_model = std::env::var("OPENROUTER_MODEL")
                        .unwrap_or_else(|_| "meta-llama/llama-3.3-70b-instruct".to_string());
                    cfg.llm.light_model = std::env::var("OPENROUTER_LIGHT_MODEL")
                        .unwrap_or_else(|_| "google/gemma-2-9b-it".to_string());
                    cfg.llm.balanced_model = std::env::var("OPENROUTER_BALANCED_MODEL")
                        .unwrap_or_else(|_| "meta-llama/llama-3.3-70b-instruct".to_string());
                    cfg.llm.full_model = std::env::var("OPENROUTER_FULL_MODEL")
                        .unwrap_or_else(|_| "meta-llama/llama-3.3-70b-instruct".to_string());
                    cfg.llm.api_key = std::env::var("OPENROUTER_API_KEY")
                        .ok()
                        .filter(|v| !v.is_empty());
                    LlmProvider::OpenRouter
                }
                "openai" => LlmProvider::OpenAI,
                "anthropic" => LlmProvider::Anthropic,
                "kimi" => {
                    cfg.llm.base_url = std::env::var("MOONSHOT_BASE_URL")
                        .unwrap_or_else(|_| "https://api.moonshot.cn".to_string());
                    cfg.llm.default_model =
                        std::env::var("KIMI_MODEL").unwrap_or_else(|_| "kimi-k2.6".to_string());
                    cfg.llm.light_model = std::env::var("KIMI_LIGHT_MODEL")
                        .unwrap_or_else(|_| "kimi-k2.5".to_string());
                    cfg.llm.balanced_model = std::env::var("KIMI_BALANCED_MODEL")
                        .unwrap_or_else(|_| "kimi-k2.6".to_string());
                    cfg.llm.full_model = std::env::var("KIMI_FULL_MODEL")
                        .unwrap_or_else(|_| "kimi-k2.6".to_string());
                    cfg.llm.api_key = std::env::var("KIMI_API_KEY").ok().filter(|v| !v.is_empty());
                    LlmProvider::Kimi
                }
                "glm" => {
                    cfg.llm.base_url = std::env::var("GLM_BASE_URL")
                        .unwrap_or_else(|_| "https://open.bigmodel.cn".to_string());
                    cfg.llm.default_model =
                        std::env::var("GLM_MODEL").unwrap_or_else(|_| "glm-5.2".to_string());
                    cfg.llm.light_model =
                        std::env::var("GLM_LIGHT_MODEL").unwrap_or_else(|_| "glm-4.5".to_string());
                    cfg.llm.balanced_model = std::env::var("GLM_BALANCED_MODEL")
                        .unwrap_or_else(|_| "glm-5.2".to_string());
                    cfg.llm.full_model =
                        std::env::var("GLM_FULL_MODEL").unwrap_or_else(|_| "glm-5.2".to_string());
                    cfg.llm.api_key = std::env::var("GLM_API_KEY").ok().filter(|v| !v.is_empty());
                    LlmProvider::Glm
                }
                "lightning" => {
                    cfg.llm.base_url = std::env::var("LIGHTNING_BASE_URL")
                        .unwrap_or_else(|_| "https://api.lightning.ai".to_string());
                    cfg.llm.default_model = std::env::var("LIGHTNING_MODEL")
                        .unwrap_or_else(|_| "meta-llama/Llama-4-Maverick-17B".to_string());
                    cfg.llm.light_model = std::env::var("LIGHTNING_LIGHT_MODEL")
                        .unwrap_or_else(|_| "meta-llama/Llama-3.1-8B-Instruct".to_string());
                    cfg.llm.balanced_model = std::env::var("LIGHTNING_BALANCED_MODEL")
                        .unwrap_or_else(|_| "meta-llama/Llama-4-Maverick-17B".to_string());
                    cfg.llm.full_model = std::env::var("LIGHTNING_FULL_MODEL")
                        .unwrap_or_else(|_| "meta-llama/Llama-4-Maverick-17B".to_string());
                    cfg.llm.api_key = std::env::var("LIGHTNING_API_KEY")
                        .ok()
                        .filter(|v| !v.is_empty());
                    LlmProvider::Lightning
                }
                "custom" => LlmProvider::Custom,
                _ => cfg.llm.provider,
            };
        }

        match cfg.llm.provider {
            LlmProvider::Ollama => {
                if let Ok(val) = std::env::var("OLLAMA_BASE_URL") {
                    cfg.llm.base_url = val;
                }
                if let Ok(val) = std::env::var("OLLAMA_MODEL") {
                    cfg.llm.default_model = val.trim().to_string();
                    cfg.llm.light_model = cfg.llm.default_model.clone();
                    cfg.llm.balanced_model = cfg.llm.default_model.clone();
                    cfg.llm.full_model = cfg.llm.default_model.clone();
                }
            }
            LlmProvider::Kimi => {
                if let Ok(val) = std::env::var("KIMI_MODEL") {
                    cfg.llm.default_model = val;
                }
                if let Ok(val) = std::env::var("KIMI_API_KEY") {
                    cfg.llm.api_key = if val.is_empty() { None } else { Some(val) };
                }
            }
            LlmProvider::Glm => {
                if let Ok(val) = std::env::var("GLM_MODEL") {
                    cfg.llm.default_model = val;
                }
                if let Ok(val) = std::env::var("GLM_API_KEY") {
                    cfg.llm.api_key = if val.is_empty() { None } else { Some(val) };
                }
            }
            LlmProvider::OpenAI => {
                if let Ok(val) = std::env::var("OPENAI_API_KEY") {
                    cfg.llm.api_key = if val.is_empty() { None } else { Some(val) };
                }
                if cfg.llm.default_model == LlmConfig::default().default_model {
                    cfg.llm.default_model = "gpt-4.1-mini".to_string();
                    cfg.llm.light_model = "gpt-4.1-nano".to_string();
                    cfg.llm.balanced_model = "gpt-4.1-mini".to_string();
                    cfg.llm.full_model = "gpt-4.1".to_string();
                }
            }
            LlmProvider::Anthropic => {
                if let Ok(val) = std::env::var("ANTHROPIC_API_KEY") {
                    cfg.llm.api_key = if val.is_empty() { None } else { Some(val) };
                }
                if cfg.llm.default_model == LlmConfig::default().default_model {
                    cfg.llm.default_model = "claude-3-5-haiku-20241022".to_string();
                    cfg.llm.light_model = "claude-3-5-haiku-20241022".to_string();
                    cfg.llm.balanced_model = "claude-3-5-sonnet-20241022".to_string();
                    cfg.llm.full_model = "claude-3-5-sonnet-20241022".to_string();
                }
            }
            LlmProvider::Groq => {
                if let Ok(val) = std::env::var("GROQ_MODEL") {
                    cfg.llm.default_model = val;
                }
                if let Ok(val) = std::env::var("GROQ_API_KEY") {
                    cfg.llm.api_key = if val.is_empty() { None } else { Some(val) };
                }
            }
            LlmProvider::OpenRouter => {
                if let Ok(val) = std::env::var("OPENROUTER_MODEL") {
                    cfg.llm.default_model = val;
                }
                if let Ok(val) = std::env::var("OPENROUTER_API_KEY") {
                    cfg.llm.api_key = if val.is_empty() { None } else { Some(val) };
                }
            }
            LlmProvider::Custom => {}
            LlmProvider::Lightning => {
                if let Ok(val) = std::env::var("LIGHTNING_MODEL") {
                    cfg.llm.default_model = val;
                }
                if let Ok(val) = std::env::var("LIGHTNING_API_KEY") {
                    cfg.llm.api_key = if val.is_empty() { None } else { Some(val) };
                }
            }
        }
        if let Ok(val) = std::env::var("DATABASE_PATH") {
            cfg.database_path = std::path::PathBuf::from(val);
        }
        if let Ok(val) = std::env::var("PROFILE_PATH") {
            cfg.profile_path = std::path::PathBuf::from(val);
        }

        cfg.sync_tier_models();

        info!(
            "Final config: provider={:?}, base_url={}, model={}",
            cfg.llm.provider, cfg.llm.base_url, cfg.llm.default_model
        );

        Ok(cfg)
    }

    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let data = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(path, data).context("Failed to write config")?;
        Ok(())
    }

    #[cfg(test)]
    pub fn test_default() -> Self {
        Self::default()
    }

    pub fn apply_preset(&mut self, preset: &str) {
        match preset {
            "lightweight" | "light" => {
                self.llm.default_model = self.llm.light_model.clone();
                self.llm.timeout_seconds = 60;
                self.llm.max_retries = 1;
                self.scraping.max_results_per_board = 10;
            }
            "balanced" => {
                self.llm.default_model = self.llm.balanced_model.clone();
                self.llm.timeout_seconds = 120;
                self.llm.max_retries = 2;
                self.scraping.max_results_per_board = 25;
            }
            "full" => {
                self.llm.default_model = self.llm.full_model.clone();
                self.llm.timeout_seconds = 180;
                self.llm.max_retries = 3;
                self.scraping.max_results_per_board = 50;
            }
            _ => {}
        }
        // Keep tier model names distinct so that hosted/cloud providers
        // actually get a different model per tier (issue #3). Without this,
        // sync_tier_models() collapses every tier to default_model, making
        // the preset effectively a no-op on Groq/OpenRouter/Kimi/etc.
        self.tiers.light.model = self.llm.light_model.clone();
        self.tiers.balanced.model = self.llm.balanced_model.clone();
        self.tiers.full.model = self.llm.full_model.clone();
    }

    pub fn sync_tier_models(&mut self) {
        let model = self.llm.default_model.clone();
        self.tiers.light.model = model.clone();
        self.tiers.balanced.model = model.clone();
        self.tiers.full.model = model;
        info!("Synced tier models to default: {}", self.llm.default_model);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_preset_keeps_distinct_tier_models_for_hosted_providers() {
        // Simulate a Groq-style config where the env-var bootstrap set
        // distinct light/balanced/full model names (issue #3).
        let mut cfg = AppConfig::default();
        cfg.llm.provider = LlmProvider::Groq;
        cfg.llm.light_model = "llama-3.1-8b-instant".to_string();
        cfg.llm.balanced_model = "llama-3.3-70b-versatile".to_string();
        cfg.llm.full_model = "llama-3.3-70b-versatile".to_string();

        cfg.apply_preset("balanced");

        assert_eq!(cfg.llm.default_model, "llama-3.3-70b-versatile");
        assert_eq!(cfg.tiers.light.model, "llama-3.1-8b-instant");
        assert_eq!(cfg.tiers.balanced.model, "llama-3.3-70b-versatile");
        assert_eq!(cfg.tiers.full.model, "llama-3.3-70b-versatile");

        cfg.apply_preset("light");

        assert_eq!(cfg.llm.default_model, "llama-3.1-8b-instant");
        assert_eq!(cfg.tiers.light.model, "llama-3.1-8b-instant");
        assert_eq!(cfg.tiers.balanced.model, "llama-3.3-70b-versatile");
        assert_eq!(cfg.tiers.full.model, "llama-3.3-70b-versatile");
    }

    #[test]
    fn apply_preset_resets_tier_models_after_sync_tier_models() {
        // sync_tier_models() collapses everything to default_model; a
        // subsequent apply_preset() must re-expand them (issue #3).
        let mut cfg = AppConfig::default();
        cfg.llm.provider = LlmProvider::OpenRouter;
        cfg.llm.default_model = "meta-llama/llama-3.3-70b-instruct".to_string();
        cfg.llm.light_model = "google/gemma-2-9b-it".to_string();
        cfg.llm.balanced_model = "meta-llama/llama-3.3-70b-instruct".to_string();
        cfg.llm.full_model = "meta-llama/llama-3.3-70b-instruct".to_string();
        cfg.sync_tier_models();

        cfg.apply_preset("light");

        assert_eq!(cfg.tiers.light.model, "google/gemma-2-9b-it");
        assert_eq!(
            cfg.tiers.balanced.model,
            "meta-llama/llama-3.3-70b-instruct"
        );
        assert_eq!(cfg.tiers.full.model, "meta-llama/llama-3.3-70b-instruct");
    }
}
