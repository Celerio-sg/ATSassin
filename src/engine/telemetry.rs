use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCall {
    pub call_id: String,
    pub ts: String,
    pub provider: String,
    pub model: String,
    pub task: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub latency_ms: u128,
    pub cost_usd: f64,
    pub success: bool,
    pub error: Option<String>,
    pub quality: Option<f64>,
    pub edit_distance: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QualityEvent {
    pub call_id: String,
    pub kind: String,
    pub decision_quality: Option<f64>,
    pub outcome: Option<String>,
    pub edit_distance: Option<usize>,
    pub observed_at: String,
}

#[derive(Debug)]
pub struct TelemetryLogger {
    journal_path: std::path::PathBuf,
    outcomes_path: std::path::PathBuf,
}

impl TelemetryLogger {
    pub fn new(journal_path: impl AsRef<Path>) -> Self {
        let journal = journal_path.as_ref().to_path_buf();
        let outcomes = journal.with_extension("outcomes.jsonl");
        Self {
            journal_path: journal,
            outcomes_path: outcomes,
        }
    }

    pub fn record_call(&self, call: &LlmCall) -> Result<()> {
        let line = serde_json::to_string(call)? + "\n";
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.journal_path)?
            .write_all(line.as_bytes())?;
        Ok(())
    }

    pub fn record_quality(
        &self,
        call_id: &str,
        quality: Option<f64>,
        edit_distance: Option<usize>,
    ) -> Result<()> {
        let event = QualityEvent {
            call_id: call_id.to_string(),
            kind: "quality".to_string(),
            decision_quality: quality,
            outcome: None,
            edit_distance,
            observed_at: chrono::Utc::now().to_rfc3339(),
        };
        self.write_event(&event)
    }

    pub fn record_outcome(&self, call_id: &str, outcome: &str) -> Result<()> {
        let event = QualityEvent {
            call_id: call_id.to_string(),
            kind: "outcome".to_string(),
            decision_quality: None,
            outcome: Some(outcome.to_string()),
            edit_distance: None,
            observed_at: chrono::Utc::now().to_rfc3339(),
        };
        self.write_event(&event)
    }

    fn write_event(&self, event: &QualityEvent) -> Result<()> {
        let line = serde_json::to_string(event)? + "\n";
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.outcomes_path)?
            .write_all(line.as_bytes())?;
        Ok(())
    }

    pub fn acceptance_rate(&self, task: &str, window_hours: i64) -> Result<f64> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(window_hours);
        let mut accepted = 0u64;
        let mut total = 0u64;

        if let Ok(data) = fs::read_to_string(&self.journal_path) {
            for line in data.lines().rev().take(1000) {
                if let Ok(call) = serde_json::from_str::<LlmCall>(line) {
                    if call.task != task {
                        continue;
                    }
                    if chrono::DateTime::parse_from_rfc3339(&call.ts)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc) < cutoff)
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    total += 1;
                    if call.edit_distance.map(|d| d < 50).unwrap_or(false) {
                        accepted += 1;
                    }
                }
            }
        }

        if total == 0 {
            return Ok(0.0);
        }
        Ok(accepted as f64 / total as f64)
    }

    pub fn avg_latency_ms(&self, provider: &str, window_hours: i64) -> Result<f64> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(window_hours);
        let mut total_ms = 0u128;
        let mut count = 0u64;

        if let Ok(data) = fs::read_to_string(&self.journal_path) {
            for line in data.lines().rev().take(1000) {
                if let Ok(call) = serde_json::from_str::<LlmCall>(line) {
                    if call.provider != provider {
                        continue;
                    }
                    if chrono::DateTime::parse_from_rfc3339(&call.ts)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc) < cutoff)
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    total_ms += call.latency_ms;
                    count += 1;
                }
            }
        }

        if count == 0 {
            return Ok(0.0);
        }
        Ok(total_ms as f64 / count as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_call_appends_jsonl() {
        let dir = std::env::temp_dir().join("atsassin_test_telemetry");
        let _ = fs::create_dir_all(&dir);
        let journal = dir.join("journal.jsonl");
        let logger = TelemetryLogger::new(&journal);
        let call = LlmCall {
            call_id: "test-1".into(),
            ts: chrono::Utc::now().to_rfc3339(),
            provider: "groq".into(),
            model: "llama-3.1-8b".into(),
            task: "scoring".into(),
            prompt_tokens: 10,
            completion_tokens: 20,
            latency_ms: 120,
            cost_usd: 0.0,
            success: true,
            error: None,
            quality: None,
            edit_distance: Some(5),
        };
        logger.record_call(&call).unwrap();
        let content = fs::read_to_string(&journal).unwrap();
        assert!(content.contains("groq"));
        assert!(content.contains("test-1"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_acceptance_rate_filters_by_task() {
        let dir = std::env::temp_dir().join("atsassin_test_telemetry2");
        let _ = fs::create_dir_all(&dir);
        let journal = dir.join("journal.jsonl");
        let logger = TelemetryLogger::new(&journal);
        for i in 0..3 {
            let call = LlmCall {
                call_id: format!("test-{}", i),
                ts: chrono::Utc::now().to_rfc3339(),
                provider: "groq".into(),
                model: "llama".into(),
                task: if i < 2 {
                    "scoring".into()
                } else {
                    "tailoring".into()
                },
                prompt_tokens: 10,
                completion_tokens: 10,
                latency_ms: 100,
                cost_usd: 0.0,
                success: true,
                error: None,
                quality: None,
                edit_distance: Some(10),
            };
            logger.record_call(&call).unwrap();
        }
        let rate = logger.acceptance_rate("scoring", 24).unwrap();
        assert!((rate - 1.0).abs() < 0.001);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_avg_latency_ms_empty_returns_zero() {
        let dir = std::env::temp_dir().join("atsassin_test_telemetry3");
        let _ = fs::create_dir_all(&dir);
        let journal = dir.join("journal.jsonl");
        let logger = TelemetryLogger::new(&journal);
        let avg = logger.avg_latency_ms("groq", 24).unwrap();
        assert_eq!(avg, 0.0);
        let _ = fs::remove_dir_all(&dir);
    }
}
