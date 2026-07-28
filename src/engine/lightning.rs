//! Lightning AI training integration.
//!
//! Submits distillation training data to the Lightning AI platform via their
//! HTTP API. The client is intentionally thin: it uploads the JSONL, creates
//! a fine-tuning job, and returns the job ID/status so the CLI can surface it.
//!
//! Configuration precedence:
//!   1. `config.toml` values when `provider = "lightning"` (`base_url`, `api_key`).
//!   2. Environment variables `LIGHTNING_BASE_URL` and `LIGHTNING_API_KEY`.
//!
//! Endpoint paths default to `/v1/fine-tuning/files` and `/v1/fine-tuning/jobs`
//! and can be overridden at client construction time with:
//!   - `LIGHTNING_UPLOAD_PATH` (default `/v1/fine-tuning/files`)
//!   - `LIGHTNING_JOBS_PATH`   (default `/v1/fine-tuning/jobs`)
//!   - `LIGHTNING_STATUS_PATH` (default `/v1/fine-tuning/jobs`)
//!
//! NOTE: The fine-tuning endpoints are modelled on common provider patterns.
//! They should be verified against the current Lightning AI documentation and
//! updated in this module if the provider's API surface changes.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Training job status returned by Lightning AI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LightningJobStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "unknown")]
    Unknown,
}

impl std::fmt::Display for LightningJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LightningJobStatus::Pending => write!(f, "pending"),
            LightningJobStatus::Running => write!(f, "running"),
            LightningJobStatus::Completed => write!(f, "completed"),
            LightningJobStatus::Failed => write!(f, "failed"),
            LightningJobStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Response from creating or polling a Lightning AI fine-tuning job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightningJob {
    pub id: String,
    pub status: LightningJobStatus,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub message: String,
}

/// Client for submitting distillation training jobs to Lightning AI.
#[derive(Debug, Clone)]
pub struct LightningClient {
    base_url: String,
    api_key: String,
    upload_path: String,
    jobs_path: String,
    status_path: String,
    client: reqwest::Client,
}

impl LightningClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            upload_path: std::env::var("LIGHTNING_UPLOAD_PATH")
                .unwrap_or_else(|_| "/v1/fine-tuning/files".to_string()),
            jobs_path: std::env::var("LIGHTNING_JOBS_PATH")
                .unwrap_or_else(|_| "/v1/fine-tuning/jobs".to_string()),
            status_path: std::env::var("LIGHTNING_STATUS_PATH")
                .unwrap_or_else(|_| "/v1/fine-tuning/jobs".to_string()),
            client: reqwest::Client::new(),
        }
    }

    /// Submit a fine-tuning job using the provided JSONL training file.
    ///
    /// The current Lightning AI public API surface is modelled as a
    /// multi-part upload followed by a job creation call. If the provider
    /// changes their endpoints, only this module needs to change.
    pub async fn submit_training_job(
        &self,
        model: &str,
        training_file: &Path,
    ) -> Result<LightningJob> {
        if !training_file.exists() {
            anyhow::bail!("Training file not found: {}", training_file.display());
        }

        let upload_url = format!("{}{}", self.base_url, self.upload_path);
        let create_url = format!("{}{}", self.base_url, self.jobs_path);

        let form = reqwest::multipart::Form::new()
            .file("file", training_file)
            .await?;

        let upload_resp = self
            .client
            .post(&upload_url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?;

        if !upload_resp.status().is_success() {
            let status = upload_resp.status();
            let text = upload_resp.text().await.unwrap_or_default();
            anyhow::bail!("Lightning AI upload failed ({}): {}", status, text);
        }

        #[derive(Deserialize)]
        struct UploadResult {
            id: String,
        }
        let upload: UploadResult = upload_resp.json().await?;

        let body = serde_json::json!({
            "model": model,
            "training_file": upload.id,
        });

        let create_resp = self
            .client
            .post(&create_url)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !create_resp.status().is_success() {
            let status = create_resp.status();
            let text = create_resp.text().await.unwrap_or_default();
            anyhow::bail!("Lightning AI job creation failed ({}): {}", status, text);
        }

        let job: LightningJob = create_resp.json().await?;
        Ok(job)
    }

    /// Poll the status of an existing fine-tuning job.
    pub async fn get_job_status(&self, job_id: &str) -> Result<LightningJob> {
        let url = format!("{}{}/{}", self.base_url, self.status_path, job_id);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Lightning AI job status failed ({}): {}", status, text);
        }

        let job: LightningJob = resp.json().await?;
        Ok(job)
    }

    /// Create a client from the LLM config when the provider is Lightning.
    /// Falls back to `LIGHTNING_API_KEY` / `LIGHTNING_BASE_URL` env vars.
    pub fn from_config(llm: &crate::config::LlmConfig) -> Result<Self> {
        let base_url = if !llm.base_url.is_empty() {
            llm.base_url.clone()
        } else {
            std::env::var("LIGHTNING_BASE_URL")
                .unwrap_or_else(|_| "https://api.lightning.ai".to_string())
        };
        let api_key = llm
            .api_key
            .clone()
            .filter(|k| !k.is_empty())
            .or_else(|| std::env::var("LIGHTNING_API_KEY").ok().filter(|k| !k.is_empty()))
            .context("Lightning AI api_key is required. Set it in config.toml or LIGHTNING_API_KEY env var.")?;
        Ok(Self::new(&base_url, &api_key))
    }

    /// Create a client purely from environment variables.
    /// Prefer `from_config` when an LLM config is already loaded.
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("LIGHTNING_BASE_URL")
            .unwrap_or_else(|_| "https://api.lightning.ai".to_string());
        let api_key = std::env::var("LIGHTNING_API_KEY")
            .context("LIGHTNING_API_KEY is required for Lightning AI training")?;
        if api_key.is_empty() {
            anyhow::bail!("LIGHTNING_API_KEY is set but empty");
        }
        Ok(Self::new(&base_url, &api_key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_display_works() {
        assert_eq!(LightningJobStatus::Pending.to_string(), "pending");
        assert_eq!(LightningJobStatus::Completed.to_string(), "completed");
    }

    #[test]
    fn client_stores_config() {
        let client = LightningClient::new("https://api.lightning.ai", "test-key");
        assert_eq!(client.base_url, "https://api.lightning.ai");
        assert_eq!(client.api_key, "test-key");
    }

    #[test]
    fn from_config_prefers_config_over_env() {
        let llm = crate::config::LlmConfig {
            base_url: "https://custom.lightning.ai".to_string(),
            api_key: Some("cfg-key".to_string()),
            ..Default::default()
        };
        let client = LightningClient::from_config(&llm).unwrap();
        assert_eq!(client.base_url, "https://custom.lightning.ai");
        assert_eq!(client.api_key, "cfg-key");
    }

    #[test]
    fn from_config_falls_back_to_env() {
        use std::sync::Mutex;
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Temporarily set env var; test will restore it.
        let original = std::env::var("LIGHTNING_API_KEY").ok();
        std::env::set_var("LIGHTNING_API_KEY", "env-key");
        let llm = crate::config::LlmConfig {
            base_url: "https://api.lightning.ai".to_string(),
            api_key: None,
            ..Default::default()
        };
        let client = LightningClient::from_config(&llm).unwrap();
        assert_eq!(client.api_key, "env-key");
        match original {
            Some(v) => std::env::set_var("LIGHTNING_API_KEY", v),
            None => std::env::remove_var("LIGHTNING_API_KEY"),
        }
    }
}
