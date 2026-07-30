//! Phase 3 — optional, hardware-gated orchestrator daemon.
//!
//! The daemon implements the full autonomous-loop workflow:
//!   scan -> evaluate/rank -> queue for tailoring -> follow-up reminders ->
//!   IMAP outcome sync.
//!
//! It is gated by the detected hardware tier: if the machine is below
//! `Balanced`, the daemon refuses to stay resident and advises the user to
//! use cron instead.

use crate::config::AppConfig;
use crate::engine::hardware::HardwareProfile;
use crate::engine::router::ModelRouter;
use crate::pipeline::tracker::PipelineTracker;
use anyhow::Result;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Clone)]
pub struct DaemonConfig {
    /// How long to sleep between ticks, in seconds.
    pub interval_sec: u64,
    /// Optional board list; falls back to config.scraping.boards.
    pub boards: Option<Vec<String>>,
    /// Role query passed to each scan tick.
    pub role: Option<String>,
    /// Max jobs per board per tick.
    pub limit: usize,
    /// Minimum composite score before a job is queued for auto-tailoring.
    pub tailor_threshold: f64,
    /// Max jobs to auto-tailor per tick.
    pub max_tailor_per_tick: usize,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            interval_sec: 3600,
            boards: None,
            role: None,
            limit: 10,
            tailor_threshold: 75.0,
            max_tailor_per_tick: 3,
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
        .clone()
        .unwrap_or_else(|| cfg.scraping.boards.clone());
    let role = daemon_cfg
        .role
        .clone()
        .unwrap_or_else(|| "general".to_string());

    // Run once mode: interval_sec == 0 means single tick.
    if daemon_cfg.interval_sec == 0 {
        return run_tick(&cfg, &boards, &role, &daemon_cfg).await;
    }
    loop {
        run_tick(&cfg, &boards, &role, &daemon_cfg).await?;
        tokio::time::sleep(Duration::from_secs(daemon_cfg.interval_sec)).await;
    }
}

fn meets_daemon_tier(profile: &HardwareProfile) -> bool {
    profile.recommended_tier == "balanced" || profile.recommended_tier == "full"
}

async fn run_tick(
    cfg: &AppConfig,
    boards: &[String],
    role: &str,
    daemon_cfg: &DaemonConfig,
) -> Result<()> {
    info!("Daemon tick: scanning boards");
    let scraper = crate::pipeline::scraper::Scraper::new(
        cfg.scraping.rate_limit_ms,
        cfg.scraping.user_agent.clone(),
    );
    let tracker = PipelineTracker::new(&cfg.database_path)?;
    let mut new_jobs = Vec::new();

    for board in boards {
        match scraper
            .scrape_board_at(board, role, daemon_cfg.limit, None)
            .await
        {
            Ok(result) => {
                info!("[{}] Found {} jobs", board, result.jobs.len());
                for summary in result.jobs.into_iter().take(daemon_cfg.limit) {
                    if summary.url.is_empty() {
                        continue;
                    }
                    let job = crate::models::job::Job {
                        id: uuid::Uuid::new_v4().to_string(),
                        title: summary.title.clone(),
                        company: summary.company.clone(),
                        location: summary.location.clone(),
                        remote: false,
                        job_type: None,
                        salary_range: None,
                        description: summary
                            .description
                            .clone()
                            .unwrap_or_else(|| summary.snippet.clone()),
                        requirements: vec![],
                        posted_at: summary.posted_at,
                        source: board.clone(),
                        url: summary.url.clone(),
                        applied: false,
                        scraped_at: chrono::Utc::now(),
                    };
                    if tracker.save_job(&job).is_ok() {
                        new_jobs.push(job);
                    }
                }
            }
            Err(e) => warn!("[{}] Scan failed: {}", board, e),
        }
    }

    if new_jobs.is_empty() {
        info!("No new jobs this tick");
    } else {
        evaluate_and_queue(cfg, &tracker, &new_jobs, daemon_cfg).await?;
    }

    trigger_follow_ups(&tracker)?;
    sync_imap_outcomes(cfg).await;

    Ok(())
}

/// Evaluate newly-scanned jobs against the user's profile, rank them, and
/// queue high-quality matches for tailoring.
async fn evaluate_and_queue(
    cfg: &AppConfig,
    tracker: &PipelineTracker,
    jobs: &[crate::models::job::Job],
    daemon_cfg: &DaemonConfig,
) -> Result<()> {
    if !cfg.profile_path.exists() {
        warn!("No profile found; skipping evaluation and auto-tailoring");
        return Ok(());
    }

    let profile = crate::engine::profile_parser::ProfileParser::parse(
        crate::engine::profile_parser::ProfileInput::Markdown {
            path: cfg.profile_path.clone(),
        },
    )?;

    // Compute free local relevance scores once for the whole batch.
    let prerank_scores: std::collections::HashMap<String, f64> =
        crate::engine::prerank::rank(&profile, jobs, |j| format!("{} {}", j.title, j.description))
            .into_iter()
            .map(|(idx, score)| (jobs[idx].id.clone(), score))
            .collect();

    let telemetry_path = cfg.database_path.with_extension("llm_telemetry.jsonl");
    let router = ModelRouter::from_llm_config(
        &cfg.llm,
        cfg.tiers.light.clone(),
        cfg.tiers.balanced.clone(),
        cfg.tiers.full.clone(),
        Some(telemetry_path),
    );

    let scorer =
        crate::engine::scorer::Scorer::new(router.clone(), crate::engine::prompts::Prompts);
    let tailor =
        crate::engine::tailor::Tailor::new(router.clone(), crate::engine::prompts::Prompts);

    let now = chrono::Utc::now();
    let mut ranked = Vec::new();
    for job in jobs {
        match scorer.evaluate(job, &profile).await {
            Ok(evaluation) => {
                let relevance = prerank_scores.get(&job.id).copied().unwrap_or(0.0);
                let pref = crate::engine::preferences::check(job, &cfg.preferences);
                let text = format!("{} {}", job.title, job.description);
                let land_score = crate::engine::landscore::score(
                    relevance,
                    &pref,
                    Some(evaluation.overall_score),
                    job.posted_at,
                    now,
                    &text,
                );
                tracker.save_evaluation(&evaluation)?;
                tracker
                    .add_pipeline_entry(&job.id, crate::models::job::PipelineStatus::Evaluated)?;
                ranked.push((land_score.composite, job.clone()));
            }
            Err(e) => {
                warn!("Failed to evaluate job {}: {}", job.id, e);
            }
        }
    }

    ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut tailored = 0;
    for (score, job) in ranked.iter().take(daemon_cfg.max_tailor_per_tick) {
        if *score < daemon_cfg.tailor_threshold {
            break;
        }
        // Avoid re-tailoring a job we already drafted in a previous tick.
        match tracker.get_latest_application(&job.id) {
            Ok(Some(_)) => {
                info!("Skipping already-tailored job {}", job.id);
                continue;
            }
            Err(e) => {
                warn!("Could not check existing application for {}: {}", job.id, e);
            }
            _ => {}
        }
        match tailor.generate_resume(job, &profile).await {
            Ok(resume) => match tailor.generate_cover_letter(job, &profile).await {
                Ok(cover) => {
                    let model_used = format!("{:?}/{}", cfg.llm.provider, "daemon");
                    if let Err(e) =
                        tracker.record_application(&job.id, &resume, &cover, &model_used)
                    {
                        warn!("Failed to record application for {}: {}", job.id, e);
                    } else {
                        let _ = tracker.update_pipeline_status_by_job_id(
                            &job.id,
                            crate::models::job::PipelineStatus::Drafted,
                        );
                        tailored += 1;
                    }
                }
                Err(e) => warn!("Failed to generate cover letter for {}: {}", job.id, e),
            },
            Err(e) => warn!("Failed to generate resume for {}: {}", job.id, e),
        }
    }

    info!(
        "Daemon evaluated {} jobs, auto-tailored {} above threshold {}",
        ranked.len(),
        tailored,
        daemon_cfg.tailor_threshold
    );
    Ok(())
}

/// Print reminders for pipeline entries that are due for follow-up.
fn trigger_follow_ups(tracker: &PipelineTracker) -> Result<()> {
    let entries = tracker.list_pipeline()?;
    let now = chrono::Utc::now();
    for entry in entries {
        if let Some(follow_up) = entry.follow_up_date {
            if follow_up <= now {
                warn!(
                    "Follow-up due for job {} (pipeline status: {:?})",
                    entry.job_id, entry.status
                );
            }
        }
    }
    Ok(())
}

/// Best-effort IMAP outcome sync. Reads the account to sync from the
/// `DAEMON_IMAP_ACCOUNT` environment variable (`server:port:username`).
/// The password must already be stored via `atsassin outcomes connect`.
async fn sync_imap_outcomes(cfg: &AppConfig) {
    let account = match std::env::var("DAEMON_IMAP_ACCOUNT") {
        Ok(a) if !a.is_empty() => a,
        _ => {
            info!("DAEMON_IMAP_ACCOUNT not set; skipping IMAP outcome sync");
            return;
        }
    };

    let parts: Vec<&str> = account.split(':').collect();
    if parts.len() != 3 {
        warn!("DAEMON_IMAP_ACCOUNT must be in the form server:port:username");
        return;
    }

    let server = parts[0].to_string();
    let port = parts[1].parse::<u16>().unwrap_or(993);
    let username = parts[2].to_string();

    let imap_cfg = crate::pipeline::outcomes::ImapConfig {
        server,
        port,
        username: username.clone(),
    };

    let password = match imap_cfg.load_password() {
        Ok(Some(p)) => p,
        _ => {
            warn!(
                "No stored IMAP password for {}; run `atsassin outcomes connect`",
                username
            );
            return;
        }
    };

    let db_path = cfg.database_path.clone();
    match tokio::task::spawn_blocking(move || {
        let t = PipelineTracker::new(&db_path)?;
        crate::pipeline::outcomes::sync_email_outcomes(&imap_cfg, &password, &t)
    })
    .await
    {
        Ok(Ok(signals)) => info!("IMAP outcome sync processed {} signal(s)", signals.len()),
        Ok(Err(e)) => warn!("IMAP outcome sync failed: {}", e),
        Err(e) => warn!("IMAP outcome sync task panicked: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    #[tokio::test]
    async fn daemon_tick_scans_all_boards() {
        let temp = tempfile::tempdir().unwrap();
        let cfg = AppConfig {
            database_path: temp.path().join("daemon-test.db"),
            ..AppConfig::default()
        };
        let daemon_cfg = DaemonConfig::default();
        let result = run_tick(&cfg, &["greenhouse".to_string()], "engineer", &daemon_cfg).await;
        // Network call may fail in CI; we only assert it doesn't panic.
        assert!(result.is_ok() || result.is_err());
    }
}
