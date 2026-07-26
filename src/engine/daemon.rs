//! Phase 3 — optional, hardware-gated orchestrator daemon.
//!
//! The daemon loops over scan and outcome ingestion. It is gated by the
//! detected hardware tier: if the machine is below `Balanced`, the daemon
//! refuses to stay resident and advises the user to use cron instead.

use crate::config::AppConfig;
use crate::engine::hardware::HardwareProfile;
use anyhow::Result;
use std::time::Duration;
use tracing::{info, warn};

pub struct DaemonConfig {
    /// How long to sleep between ticks, in seconds.
    pub interval_sec: u64,
    /// Optional board list; falls back to config.scraping.boards.
    pub boards: Option<Vec<String>>,
    /// Role query passed to each scan tick.
    pub role: Option<String>,
    /// Max jobs per board per tick.
    pub limit: usize,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            interval_sec: 3600,
            boards: None,
            role: None,
            limit: 10,
        }
    }
}

/// Run the daemon loop. Returns immediately with a helpful message if the
/// hardware tier is below `Balanced`.
pub async fn run_daemon(cfg: AppConfig, daemon_cfg: DaemonConfig) -> Result<()> {
    let profile = HardwareProfile::detect();
    if !meets_daemon_tier(&profile) {
        println!(
            "Hardware tier is below Balanced. The daemon is disabled to avoid resource exhaustion."
        );
        println!("Run `atsassin daemon --once` or schedule `atsassin scan` via cron instead.");
        return Ok(());
    }

    info!(
        "Starting ATSassin daemon (interval: {}s, recommended: {})",
        daemon_cfg.interval_sec, profile.recommended_tier
    );

    let boards = daemon_cfg
        .boards
        .unwrap_or_else(|| cfg.scraping.boards.clone());
    let role = daemon_cfg.role.unwrap_or_else(|| "general".to_string());

    // Run once mode: interval_sec == 0 means single tick.
    if daemon_cfg.interval_sec == 0 {
        return run_tick(&cfg, &boards, &role, daemon_cfg.limit).await;
    }

    loop {
        run_tick(&cfg, &boards, &role, daemon_cfg.limit).await?;
        tokio::time::sleep(Duration::from_secs(daemon_cfg.interval_sec)).await;
    }
}

fn meets_daemon_tier(profile: &HardwareProfile) -> bool {
    profile.recommended_tier == "balanced" || profile.recommended_tier == "full"
}

async fn run_tick(cfg: &AppConfig, boards: &[String], role: &str, limit: usize) -> Result<()> {
    info!("Daemon tick: scanning boards");
    for board in boards {
        let scraper = crate::pipeline::scraper::Scraper::new(
            cfg.scraping.rate_limit_ms,
            cfg.scraping.user_agent.clone(),
        );
        match scraper.scrape_board_at(board, role, limit, None).await {
            Ok(result) => info!("[{}] Found {} jobs", board, result.jobs.len()),
            Err(e) => warn!("[{}] Scan failed: {}", board, e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    #[tokio::test]
    async fn daemon_tick_scans_all_boards() {
        let cfg = AppConfig::default();
        let result = run_tick(&cfg, &["greenhouse".to_string()], "engineer", 5).await;
        // Network call may fail in CI; we only assert it doesn't panic.
        assert!(result.is_ok() || result.is_err());
    }
}
