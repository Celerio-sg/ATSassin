use crate::pipeline::scraper::Scraper;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    pub enabled: bool,
    pub boards: Vec<String>,
    pub max_results_per_board: usize,
    pub rate_limit_ms: u64,
}

pub struct Scanner {
    config: ScannerConfig,
}

impl Scanner {
    pub fn new(config: ScannerConfig) -> Self {
        Self { config }
    }

    pub async fn scan_role(&self, role: &str, limit: usize) -> Result<Vec<ScrapedJob>> {
        if !self.config.enabled {
            return Ok(vec![]);
        }
        let scraper = Scraper::new(self.config.rate_limit_ms, "ATSassin/1.0".to_string());
        let per_board = limit.max(1).min(self.config.max_results_per_board);
        let mut jobs = Vec::new();
        let mut errors = Vec::new();

        for board in &self.config.boards {
            match scraper.scrape_board(board, role, per_board).await {
                Ok(result) => {
                    for summary in result.jobs {
                        jobs.push(ScrapedJob {
                            id: format!(
                                "{}-{}-{}",
                                board,
                                summary.title.replace(' ', "-").to_lowercase(),
                                chrono::Utc::now().timestamp()
                            ),
                            title: summary.title,
                            company: summary.company,
                            location: summary.location,
                            remote: false,
                            url: summary.url,
                            source: board.clone(),
                            posted_at: summary.posted_at,
                        });
                    }
                }
                Err(e) => {
                    errors.push(format!("{}: {}", board, e));
                }
            }
        }

        if jobs.is_empty() && !errors.is_empty() {
            anyhow::bail!("All boards failed: {}", errors.join("; "));
        }

        if jobs.is_empty() {
            anyhow::bail!("No jobs returned for query '{}'. Tried boards: {}. Try a different query or check network connectivity.", role, self.config.boards.join(", "));
        }

        Ok(jobs)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedJob {
    pub id: String,
    pub title: String,
    pub company: String,
    pub location: String,
    pub remote: bool,
    pub url: String,
    pub source: String,
    pub posted_at: Option<chrono::DateTime<chrono::Utc>>,
}
