use crate::engine::profile_parser::read_text_lossy;
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "atsassin")]
#[command(
    about = "The silent killer of bad job matches.",
    long_about = "The silent killer of bad job matches.\n\nQuick start:\n  1. atsassin profile init --resume <file>   (parse your resume/LinkedIn export)\n  2. atsassin roles infer -n 8                (infer target role archetypes)\n  3. atsassin scan --role \"<top role>\"        (find real job postings)\n  4. atsassin evaluate --job-id <id>          (score a job against your profile)\n  5. atsassin tailor --job-id <id>            (generate a tailored resume + cover letter)\n\nOr run `atsassin tui` for the interactive dashboard - it walks you through the same steps."
)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    #[arg(short, long, global = true, default_value = "config.toml")]
    pub config: PathBuf,
    #[arg(short = 'P', long, global = true, default_value = "balanced")]
    pub preset: String,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Profile(ProfileArgs),
    Roles(RolesArgs),
    Scan(ScanArgs),
    Evaluate(EvaluateArgs),
    Tailor(TailorArgs),
    Apply(ApplyArgs),
    Playbook,
    Pipeline(PipelineArgs),
    Tui(TuiArgs),
    Distill(DistillArgs),
    Feedback(FeedbackArgs),
    Market(MarketArgs),
    Preferences(PreferencesArgs),
    Recommend(RecommendArgs),
    Companies(CompaniesArgs),
    Outcomes(OutcomesArgs),
    Compute(ComputeArgs),
    Telemetry(TelemetryArgs),
    Daemon(DaemonArgs),
}

#[derive(Args, Debug)]
pub struct RecommendArgs {
    /// How many ranked jobs to show
    #[arg(short, long, default_value = "15")]
    pub limit: usize,
    /// How many persisted jobs to consider (across all past scans)
    #[arg(long, default_value = "500")]
    pub pool: usize,
}

#[derive(Args, Debug)]
pub struct CompaniesArgs {
    #[command(subcommand)]
    pub action: CompaniesAction,
}

#[derive(Subcommand, Debug)]
pub enum CompaniesAction {
    /// Discover which ATS board a company uses by scanning its public
    /// careers page (issue #1).
    Discover {
        /// Company display name, e.g. "Acme Corp"
        #[arg(short, long)]
        name: String,
        /// Company domain, e.g. "acme.com"
        #[arg(short, long)]
        domain: String,
    },
    /// List previously discovered company boards.
    List,
}

#[derive(Args, Debug)]
pub struct OutcomesArgs {
    #[command(subcommand)]
    pub action: OutcomesAction,
}

#[derive(Subcommand, Debug)]
pub enum OutcomesAction {
    /// Store IMAP credentials in the OS keychain (Phase 0).
    Connect {
        /// IMAP server, e.g. imap.gmail.com
        #[arg(short, long)]
        server: String,
        /// IMAP port (default 993 for TLS).
        #[arg(short, long, default_value_t = 993)]
        port: u16,
        /// IMAP username, usually the email address.
        #[arg(short, long)]
        username: String,
        /// IMAP password or app-password.
        #[arg(short, long)]
        password: String,
    },
    /// Read ATS outcome emails and update pipeline statuses (Phase 0).
    Sync {
        /// IMAP server, e.g. imap.gmail.com
        #[arg(short, long)]
        server: String,
        /// IMAP port (default 993 for TLS).
        #[arg(short, long, default_value_t = 993)]
        port: u16,
        /// IMAP username, usually the email address.
        #[arg(short, long)]
        username: String,
        /// If provided, use this password instead of the stored one.
        #[arg(short, long)]
        password: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct ComputeArgs {
    #[command(subcommand)]
    pub action: ComputeAction,
}

#[derive(Args, Debug)]
pub struct TelemetryArgs {
    #[command(subcommand)]
    pub action: TelemetryAction,
}

#[derive(Subcommand, Debug)]
pub enum TelemetryAction {
    /// Compress telemetry records older than N days into a zstd archive
    /// and remove them from the hot journal (Phase 2).
    Archive {
        /// Days of recent telemetry to keep uncompressed (default 30).
        #[arg(short, long, default_value_t = 30)]
        days: i64,
    },
}

#[derive(Args, Debug)]
pub struct DaemonArgs {
    /// Daemon tick interval in seconds (default 3600 = 1 hour).
    #[arg(short, long, default_value_t = 3600)]
    pub interval: u64,
    /// Run a single tick and exit instead of looping.
    #[arg(long, default_value_t = false)]
    pub once: bool,
}

#[derive(Subcommand, Debug)]
pub enum ComputeAction {
    /// Show the current Compute Broker provider registry and cached quota
    /// (Phase 1).
    Status,
}

#[derive(Args, Debug)]
pub struct PreferencesArgs {
    #[command(subcommand)]
    pub action: PreferencesAction,
}

#[derive(Subcommand, Debug)]
pub enum PreferencesAction {
    Show,
    Set {
        /// Minimum acceptable compensation, USD/yr equivalent
        #[arg(long)]
        min_comp: Option<u64>,
        /// any | fulltime | contract
        #[arg(long)]
        employment_type: Option<String>,
        /// any | remote | hybrid | onsite
        #[arg(long)]
        work_mode: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub action: ProfileAction,
}

#[derive(Subcommand, Debug)]
pub enum ProfileAction {
    Init {
        #[arg(short, long)]
        resume: Option<PathBuf>,
        #[arg(short, long)]
        linkedin: Option<PathBuf>,
        #[arg(short, long)]
        portfolio: Option<String>,
    },
    Show,
    Export {
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Args, Debug)]
pub struct ScanArgs {
    #[arg(short, long)]
    pub role: Option<String>,
    #[arg(short, long, default_value = "10")]
    pub limit: usize,
    #[arg(short, long, value_delimiter = ',')]
    pub boards: Option<Vec<String>>,
    /// Only show/save jobs matching your saved preferences (`atsassin preferences set`)
    #[arg(long)]
    pub prefs_only: bool,
    /// Target location (e.g. "Singapore", "United Kingdom", "Worldwide").
    /// Without this, LinkedIn's guest API silently defaults to whatever
    /// location it infers server-side (found via real-world testing:
    /// consistently US postings regardless of query text) - this was a
    /// real gap, not a query-wording problem.
    #[arg(short = 'L', long)]
    pub location: Option<String>,
}

#[derive(Args, Debug)]
pub struct EvaluateArgs {
    #[arg(short, long)]
    pub job_id: Option<String>,
    #[arg(short, long)]
    pub file: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct TailorArgs {
    #[arg(short, long)]
    pub job_id: Option<String>,
    #[arg(short, long)]
    pub file: Option<PathBuf>,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ApplyArgs {
    /// Job ID for which the saved application materials will be used.
    #[arg(short, long)]
    pub job_id: String,
    /// Directory to write apply.js and bookmarklet.txt.
    #[arg(short, long, default_value = "apply_kit")]
    pub output: PathBuf,
}

#[derive(Args, Debug)]
pub struct PipelineArgs {
    #[command(subcommand)]
    pub action: PipelineAction,
}

#[derive(Subcommand, Debug)]
pub enum PipelineAction {
    List,
    Add {
        #[arg(short, long)]
        job_id: String,
        #[arg(short, long)]
        status: Option<String>,
    },
    Update {
        #[arg(short, long)]
        job_id: String,
        #[arg(short, long)]
        status: Option<String>,
        #[arg(short, long)]
        notes: Option<String>,
        // Explicit 'C' (not the auto-derived 'c') - 'c' is already the
        // global --config short flag, visible in every subcommand. Only
        // caught by clap's debug_assert, which is compiled out of release
        // builds - found via CI's non-release `cargo test`, not locally.
        #[arg(short = 'C', long)]
        contact: Option<String>,
        /// Follow-up date as YYYY-MM-DD
        #[arg(short, long = "follow-up")]
        follow_up: Option<String>,
    },
    Export {
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Shows the job you applied to and the resume/cover letter you actually
    /// submitted for it - so you're never guessing ahead of an interview.
    Show {
        #[arg(short, long)]
        job_id: String,
    },
}

#[derive(Args, Debug)]
pub struct TuiArgs {
    #[arg(short, long, default_value_t = false)]
    pub fullscreen: bool,
    /// Render without emoji - use on terminals/codepages that mangle them
    /// (legacy Windows consoles, some SSH clients).
    #[arg(long, default_value_t = false)]
    pub ascii: bool,
}

#[derive(Args, Debug)]
pub struct DistillArgs {
    #[arg(short, long, default_value = "distillation_data")]
    pub output: PathBuf,
    #[arg(short, long, default_value_t = 5)]
    pub roles: usize,
    #[arg(short, long)]
    pub profile: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct FeedbackArgs {
    #[command(subcommand)]
    pub action: FeedbackActionCli,
}

#[derive(Subcommand, Debug)]
pub enum FeedbackActionCli {
    Record {
        #[arg(short, long)]
        job_id: String,
        #[arg(short, long)]
        task: String,
        #[arg(short, long)]
        action: String,
        #[arg(short, long)]
        recommendation: String,
        #[arg(short, long)]
        edited: Option<String>,
        // Explicit distinct short flags - both auto-derive to 'c' from
        // their field name, which clap's debug_assert rejects (compiled
        // out of release builds, so this only surfaces in debug/test runs).
        #[arg(short = 'b', long, default_value_t = 0.0)]
        confidence_before: f64,
        #[arg(short = 'A', long, default_value_t = 0.0)]
        confidence_after: f64,
    },
    Stats {
        #[arg(short, long, default_value_t = 30)]
        days: i64,
    },
    Recent {
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    ShouldEscalate {
        #[arg(short, long)]
        task: String,
    },
}

#[derive(Args, Debug)]
pub struct MarketArgs {
    #[command(subcommand)]
    pub action: MarketAction,
}

#[derive(Subcommand, Debug)]
pub enum MarketAction {
    Stats,
    Rates {
        #[arg(short, long)]
        role: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct RolesArgs {
    #[command(subcommand)]
    pub action: RolesAction,
}

#[derive(Subcommand, Debug)]
pub enum RolesAction {
    List,
    Infer {
        #[arg(short = 'n', long, default_value_t = 5)]
        count: usize,
    },
    Research {
        #[arg(short, long)]
        role: Option<String>,
    },
}

use crate::engine::profile_parser::{ProfileInput, ProfileParser};
use crate::engine::role_inference::RoleInferenceEngine;
use crate::engine::router::ModelRouter;

/// Parses a `YYYY-MM-DD` follow-up date into midnight UTC on that day.
fn parse_follow_up_date(s: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    let date = chrono::NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").with_context(|| {
        format!(
            "Invalid --follow-up date '{}'. Expected format: YYYY-MM-DD",
            s
        )
    })?;
    Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc())
}

impl Cli {
    fn load_config(&self) -> Result<crate::config::AppConfig> {
        let mut cfg = crate::config::AppConfig::load(&self.config)?;
        cfg.apply_preset(&self.preset);
        Ok(cfg)
    }

    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Commands::Profile(args) => self.handle_profile(args).await,
            Commands::Roles(args) => self.handle_roles(args).await,
            Commands::Scan(args) => self.handle_scan(args).await,
            Commands::Evaluate(args) => self.handle_evaluate(args).await,
            Commands::Tailor(args) => self.handle_tailor(args).await,
            Commands::Apply(args) => self.handle_apply(args).await,
            Commands::Playbook => self.handle_playbook().await,
            Commands::Pipeline(args) => self.handle_pipeline(args).await,
            Commands::Tui(args) => self.handle_tui(args).await,
            Commands::Distill(args) => self.handle_distill(args).await,
            Commands::Feedback(args) => self.handle_feedback(args).await,
            Commands::Market(args) => self.handle_market(args).await,
            Commands::Preferences(args) => self.handle_preferences(args).await,
            Commands::Recommend(args) => self.handle_recommend(args).await,
            Commands::Companies(args) => self.handle_companies(args).await,
            Commands::Outcomes(args) => self.handle_outcomes(args).await,
            Commands::Compute(args) => self.handle_compute(args).await,
            Commands::Telemetry(args) => self.handle_telemetry(args).await,
            Commands::Daemon(args) => self.handle_daemon(args).await,
        }
    }

    pub async fn handle_profile(&self, args: &ProfileArgs) -> Result<()> {
        match &args.action {
            ProfileAction::Init {
                resume,
                linkedin,
                portfolio,
            } => {
                let input = if let Some(path) = resume {
                    let ext = path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    match ext.as_str() {
                        "pdf" => ProfileInput::Pdf { path: path.clone() },
                        "docx" => ProfileInput::Docx { path: path.clone() },
                        "md" | "txt" | "yaml" | "yml" => {
                            ProfileInput::Markdown { path: path.clone() }
                        }
                        _ => ProfileInput::Markdown { path: path.clone() },
                    }
                } else if let Some(path) = linkedin {
                    ProfileInput::LinkedInExport { path: path.clone() }
                } else if let Some(url) = portfolio {
                    ProfileInput::PortfolioUrl { url: url.clone() }
                } else {
                    anyhow::bail!("Provide --resume, --linkedin, or --portfolio URL");
                };

                let profile = ProfileParser::parse(input)?;
                info!(
                    "Parsed profile: {} ({} skills, {} experiences)",
                    profile.name,
                    profile.skills.len(),
                    profile.experience.len()
                );
                println!("Profile parsed successfully for: {}", profile.name);
                println!("Skills found: {}", profile.skills.len());
                println!("Experience entries: {}", profile.experience.len());
                println!("Run `atsassin roles infer` to discover suitable roles.");

                let default_path = std::path::PathBuf::from("profile.md");
                std::fs::write(&default_path, &profile.raw_text)
                    .context("Failed to save profile to profile.md")?;
                println!("Profile saved to {}", default_path.display());
                Ok(())
            }
            ProfileAction::Show => {
                let cfg = self.load_config()?;
                let path = if cfg.profile_path.exists() {
                    cfg.profile_path
                } else {
                    std::path::PathBuf::from("profile.md")
                };
                if !path.exists() {
                    anyhow::bail!("No profile found. Run `atsassin profile init` first.");
                }
                let profile = ProfileParser::parse(ProfileInput::Markdown { path })?;
                println!("=== Candidate Profile: {} ===", profile.name);
                println!(
                    "Email: {}",
                    profile.email.as_deref().unwrap_or("Not specified")
                );
                println!(
                    "Location: {}",
                    profile.location.as_deref().unwrap_or("Not specified")
                );
                println!(
                    "\nSummary:\n{}",
                    profile.summary.as_deref().unwrap_or("No summary provided")
                );
                println!("\nSkills ({}):", profile.skills.len());
                for skill in profile.skills.iter().take(15) {
                    println!("  - {}", skill.name);
                }
                println!("\nExperience Entries: {}", profile.experience.len());
                Ok(())
            }
            ProfileAction::Export { output } => {
                let cfg = self.load_config()?;
                let path = if cfg.profile_path.exists() {
                    cfg.profile_path
                } else {
                    std::path::PathBuf::from("profile.md")
                };
                if !path.exists() {
                    anyhow::bail!("No profile found to export.");
                }
                std::fs::copy(&path, output)?;
                println!("Exported candidate profile to: {}", output.display());
                Ok(())
            }
        }
    }

    pub async fn handle_daemon(&self, args: &DaemonArgs) -> Result<()> {
        let cfg = self.load_config()?;
        if args.once {
            let daemon_cfg = crate::engine::daemon::DaemonConfig {
                interval_sec: 0,
                boards: Some(cfg.scraping.boards.clone()),
                role: None,
                limit: cfg.scraping.max_results_per_board,
            };
            let mut once_cfg = daemon_cfg;
            once_cfg.interval_sec = 0;
            crate::engine::daemon::run_daemon(cfg, once_cfg).await
        } else {
            let daemon_cfg = crate::engine::daemon::DaemonConfig {
                interval_sec: args.interval,
                boards: Some(cfg.scraping.boards.clone()),
                role: None,
                limit: cfg.scraping.max_results_per_board,
            };
            crate::engine::daemon::run_daemon(cfg, daemon_cfg).await
        }
    }

    pub async fn handle_telemetry(&self, args: &TelemetryArgs) -> Result<()> {
        let cfg = self.load_config()?;
        match &args.action {
            TelemetryAction::Archive { days } => {
                let logger = crate::engine::telemetry::TelemetryLogger::new(
                    cfg.database_path.with_extension("llm_telemetry.jsonl"),
                );
                let archived = logger.archive_old_records(*days)?;
                println!(
                    "Archived {} telemetry record(s) older than {} days.",
                    archived, days
                );
                Ok(())
            }
        }
    }

    pub async fn handle_compute(&self, args: &ComputeArgs) -> Result<()> {
        let cfg = self.load_config()?;
        match &args.action {
            ComputeAction::Status => {
                let broker = crate::engine::compute_broker::ComputeBroker::from_config(&cfg);
                println!("=== Compute Broker Status ===");
                if broker.providers.is_empty() {
                    println!("No providers configured.");
                } else {
                    for provider in &broker.providers {
                        println!(
                            "  {} ({}) - {} - model: {}{}",
                            provider.name,
                            provider.tier_type.as_str(),
                            provider.base_url,
                            provider.default_model,
                            if provider.allow_paid {
                                " [paid ok]"
                            } else {
                                ""
                            }
                        );
                    }
                }
                Ok(())
            }
        }
    }

    pub async fn handle_companies(&self, args: &CompaniesArgs) -> Result<()> {
        let cfg = self.load_config()?;
        let tracker = crate::pipeline::tracker::PipelineTracker::new(&cfg.database_path)?;
        match &args.action {
            CompaniesAction::Discover { name, domain } => {
                println!(
                    "Discovering ATS board for {} at https://{}...",
                    name, domain
                );
                match crate::pipeline::board_discovery::discover_domain(name, domain).await? {
                    Some(board) => {
                        println!(
                            "Discovered {} board for {}: {} (from {})",
                            board.ats_type.as_str(),
                            board.company,
                            board.slug,
                            board.source_url
                        );
                        tracker.save_company_boards(&[board])?;
                        println!("Saved to local database.");
                    }
                    None => {
                        println!(
                            "No known ATS detected on public careers pages for {}.",
                            name
                        );
                    }
                }
                Ok(())
            }
            CompaniesAction::List => {
                let boards = tracker.load_company_boards()?;
                if boards.is_empty() {
                    println!("No discovered company boards. Run `atsassin companies discover --name <name> --domain <domain>` first.");
                } else {
                    println!("Discovered company boards:");
                    for board in boards {
                        println!(
                            "  {} -> {} ({} board, source: {})",
                            board.company,
                            board.slug,
                            board.ats_type.as_str(),
                            board.source_url
                        );
                    }
                }
                Ok(())
            }
        }
    }

    pub async fn handle_outcomes(&self, args: &OutcomesArgs) -> Result<()> {
        let cfg = self.load_config()?;
        match &args.action {
            OutcomesAction::Connect {
                server,
                port,
                username,
                password,
            } => {
                let imap_cfg = crate::pipeline::outcomes::ImapConfig {
                    server: server.clone(),
                    port: *port,
                    username: username.clone(),
                };
                imap_cfg.save_password(password)?;
                println!("IMAP credentials stored in OS keychain.");
                Ok(())
            }
            OutcomesAction::Sync {
                server,
                port,
                username,
                password,
            } => {
                let imap_cfg = crate::pipeline::outcomes::ImapConfig {
                    server: server.clone(),
                    port: *port,
                    username: username.clone(),
                };
                let password = if let Some(p) = password {
                    p.clone()
                } else {
                    imap_cfg
                        .load_password()?
                        .ok_or_else(|| anyhow::anyhow!("No stored IMAP password. Run `atsassin outcomes connect` first or pass --password."))?
                };
                let db_path = cfg.database_path.clone();
                // IMAP is synchronous network I/O; keep it off the async runtime.
                let signals = tokio::task::spawn_blocking(move || {
                    let tracker = crate::pipeline::tracker::PipelineTracker::new(&db_path)?;
                    crate::pipeline::outcomes::sync_email_outcomes(&imap_cfg, &password, &tracker)
                })
                .await??;
                println!("Processed {} outcome signal(s)", signals.len());
                for signal in &signals {
                    println!(
                        "  {:?} -> {:?} ({})",
                        signal.status, signal.source_id, signal.raw_subject
                    );
                }
                Ok(())
            }
        }
    }

    pub async fn handle_scan(&self, args: &ScanArgs) -> Result<()> {
        let cfg = self.load_config()?;
        let telemetry_path = cfg.database_path.with_extension("llm_telemetry.jsonl");
        let _router = ModelRouter::from_llm_config(
            &cfg.llm,
            cfg.tiers.light,
            cfg.tiers.balanced,
            cfg.tiers.full,
            Some(telemetry_path.clone()),
        );
        let tracker = crate::pipeline::tracker::PipelineTracker::new(&cfg.database_path)?;
        // Issue #1: augment the curated company sweep with discovered
        // Greenhouse boards. Other ATS types (Lever/Ashby/Workday) are
        // detected but not yet swept by this path; they are still saved so
        // future per-ATS scrapers can use them.
        let discovered = tracker
            .load_company_boards()
            .unwrap_or_default()
            .into_iter()
            .filter(|b| {
                matches!(
                    b.ats_type,
                    crate::pipeline::board_discovery::AtsType::Greenhouse
                )
            })
            .map(|b| (b.company, b.slug))
            .collect::<Vec<_>>();
        let scraper = crate::pipeline::scraper::Scraper::new(
            cfg.scraping.rate_limit_ms,
            cfg.scraping.user_agent.clone(),
        )
        .with_extra_companies(discovered);

        let boards = args.boards.clone().unwrap_or(cfg.scraping.boards.clone());
        let query = args.role.clone().unwrap_or_else(|| "general".to_string());
        let mut scanned_jobs: Vec<crate::models::job::Job> = Vec::new();

        for board in boards {
            info!("Scanning {} for role: {}", board, query);
            let result = scraper
                .scrape_board_at(&board, &query, args.limit, args.location.as_deref())
                .await?;
            println!("[{}] Found {} jobs", board, result.jobs.len());
            for summary in result.jobs.iter().take(args.limit) {
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

                let pref_match = crate::engine::preferences::check(&job, &cfg.preferences);
                if args.prefs_only && !pref_match.matches {
                    continue;
                }

                println!(
                    "  - {} at {} ({})",
                    summary.title, summary.company, summary.location
                );
                println!("    URL: {}", summary.url);
                if !pref_match.matches {
                    println!(
                        "    prefs: does not match - {}",
                        pref_match.reasons.join("; ")
                    );
                }
                if tracker.save_job(&job).is_ok() {
                    println!("    id: {}", job.id);
                }
                scanned_jobs.push(job);
            }
        }

        // Free, local, zero-LLM-cost relevance ranking (career-ops/jobsync
        // "prerank" pattern) - only runs if a profile is available, and
        // never calls the LLM. Helps point `evaluate` at the jobs actually
        // worth spending a call on.
        if !scanned_jobs.is_empty() && cfg.profile_path.exists() {
            if let Ok(profile) = ProfileParser::parse(ProfileInput::Markdown {
                path: cfg.profile_path.clone(),
            }) {
                let ranked = crate::engine::prerank::rank(&profile, &scanned_jobs, |j| {
                    format!("{} {}", j.title, j.description)
                });
                println!("\n=== Top matches by local relevance (zero LLM calls) ===");
                for (idx, score) in ranked.iter().take(5) {
                    let job = &scanned_jobs[*idx];
                    println!(
                        "  {:.0}%  {} at {}  (id: {})",
                        score * 100.0,
                        job.title,
                        job.company,
                        job.id
                    );
                }
                println!("Run `atsassin evaluate --job-id <id>` on these first for the best return on any LLM budget you're spending.");
            }
        }

        Ok(())
    }

    pub async fn handle_roles(&self, args: &RolesArgs) -> Result<()> {
        let cfg = self.load_config()?;
        let telemetry_path = cfg.database_path.with_extension("llm_telemetry.jsonl");
        let router = ModelRouter::from_llm_config(
            &cfg.llm,
            cfg.tiers.light,
            cfg.tiers.balanced,
            cfg.tiers.full,
            Some(telemetry_path.clone()),
        );
        let role_engine = RoleInferenceEngine::new(router.clone());

        match &args.action {
            RolesAction::List => {
                let path = if cfg.profile_path.exists() {
                    cfg.profile_path
                } else {
                    std::path::PathBuf::from("profile.md")
                };
                if !path.exists() {
                    anyhow::bail!("No profile found. Run `atsassin profile init` first.");
                }
                let profile = ProfileParser::parse(ProfileInput::Markdown { path })?;
                let roles = role_engine.infer_roles(&profile).await?;
                println!("=== Target Role Archetypes ===");
                for (idx, r) in roles.iter().enumerate() {
                    println!(
                        "{}. {} ({}) - Seniority: {:?}",
                        idx + 1,
                        r.title,
                        r.industry,
                        r.seniority
                    );
                    println!(
                        "   Typical Requirements: {}",
                        r.typical_requirements.join(", ")
                    );
                }
                Ok(())
            }
            RolesAction::Infer { count } => {
                let profile = ProfileParser::parse(ProfileInput::Markdown {
                    path: cfg.profile_path.clone(),
                })?;
                let roles = role_engine.infer_roles(&profile).await?;
                println!("Inferred {} roles:", roles.len().min(*count));
                for r in roles.iter().take(*count) {
                    println!("- {} ({})", r.title, r.industry);
                }
                Ok(())
            }
            RolesAction::Research { role } => {
                let research_engine = crate::engine::deep_research::DeepResearchEngine::new(router);
                let title = role
                    .clone()
                    .unwrap_or_else(|| "Software Architect".to_string());
                let archetype = crate::models::role::RoleArchetype {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: title.clone(),
                    industry: "Technology".to_string(),
                    seniority: crate::models::role::Seniority::Senior,
                    fit_score: 0.85,
                    market_demand: crate::models::role::MarketDemand {
                        level: crate::models::role::DemandLevel::High,
                        posting_volume_30d: 1200,
                        trend: crate::models::role::TrendDirection::Growing,
                        last_updated: chrono::Utc::now(),
                    },
                    compensation_band: crate::models::role::CompensationBand {
                        currency: "USD".to_string(),
                        min: 150000,
                        max: 240000,
                        median: 195000,
                        source: "Market Intelligence".to_string(),
                    },
                    typical_requirements: vec![
                        "System Architecture".to_string(),
                        "Performance Engineering".to_string(),
                    ],
                    top_companies: vec!["BigTech".to_string(), "Scaleups".to_string()],
                    inferred_from_profile: true,
                    created_at: chrono::Utc::now(),
                };

                let updated = research_engine
                    .research_role(&archetype, "High market demand for senior engineers")
                    .await?;
                println!("=== Deep Research Report: {} ===", updated.title);
                println!("Industry: {}", updated.industry);
                println!("Market Demand: {:?}", updated.market_demand.level);
                println!(
                    "Typical Requirements: {}",
                    updated.typical_requirements.join(", ")
                );
                println!("Top Hiring Companies: {}", updated.top_companies.join(", "));
                Ok(())
            }
        }
    }

    pub async fn handle_evaluate(&self, args: &EvaluateArgs) -> Result<()> {
        let cfg = self.load_config()?;
        let telemetry_path = cfg.database_path.with_extension("llm_telemetry.jsonl");
        let router = ModelRouter::from_llm_config(
            &cfg.llm,
            cfg.tiers.light.clone(),
            cfg.tiers.balanced.clone(),
            cfg.tiers.full.clone(),
            Some(telemetry_path.clone()),
        );
        let tracker = crate::pipeline::tracker::PipelineTracker::new(&cfg.database_path)?;

        let job = if let Some(job_id) = &args.job_id {
            match tracker.get_job(job_id)? {
                Some(job) => job,
                None => anyhow::bail!("Job '{}' not found in database. Run `atsassin evaluate --file <jd.txt>` to add it first.", job_id),
            }
        } else if let Some(file) = &args.file {
            let text = read_text_lossy(file).context("Failed to read job description file")?;
            let job = crate::models::job::Job {
                id: uuid::Uuid::new_v4().to_string(),
                title: "Imported Job".to_string(),
                company: "Unknown".to_string(),
                location: "Unknown".to_string(),
                remote: false,
                job_type: None,
                salary_range: None,
                description: text,
                requirements: vec![],
                posted_at: None,
                source: "file".to_string(),
                url: String::new(),
                applied: false,
                scraped_at: chrono::Utc::now(),
            };
            let job_id = job.id.clone();
            tracker.save_job(&job)?;
            println!("Job saved as '{}'. Evaluating...", job_id);
            job
        } else {
            anyhow::bail!("Provide --job-id or --file");
        };

        let profile = crate::engine::profile_parser::ProfileParser::parse(
            crate::engine::profile_parser::ProfileInput::Markdown {
                path: cfg.profile_path.clone(),
            },
        )?;
        let scorer = crate::engine::scorer::Scorer::new(router, crate::engine::prompts::Prompts);
        let evaluation = scorer.evaluate(&job, &profile).await?;

        println!(
            "Score: {} ({})",
            evaluation.overall_score, evaluation.overall_grade
        );
        println!("Summary: {}", evaluation.match_summary);
        println!("Strengths: {}", evaluation.strengths.join(", "));
        println!("Gaps: {}", evaluation.gaps.join(", "));

        tracker.save_evaluation(&evaluation)?;
        tracker.add_pipeline_entry(&job.id, crate::models::job::PipelineStatus::Evaluated)?;

        Ok(())
    }

    pub async fn handle_tailor(&self, args: &TailorArgs) -> Result<()> {
        let cfg = self.load_config()?;
        let telemetry_path = cfg.database_path.with_extension("llm_telemetry.jsonl");
        let router = ModelRouter::from_llm_config(
            &cfg.llm,
            cfg.tiers.light.clone(),
            cfg.tiers.balanced.clone(),
            cfg.tiers.full.clone(),
            Some(telemetry_path.clone()),
        );
        let tracker = crate::pipeline::tracker::PipelineTracker::new(&cfg.database_path)?;

        let job = if let Some(job_id) = &args.job_id {
            match tracker.get_job(job_id)? {
                Some(job) => job,
                None => anyhow::bail!("Job '{}' not found in database. Run `atsassin evaluate --file <jd.txt>` to add it first.", job_id),
            }
        } else if let Some(file) = &args.file {
            let text = read_text_lossy(file).context("Failed to read job description file")?;
            let job = crate::models::job::Job {
                id: uuid::Uuid::new_v4().to_string(),
                title: "Imported Job".to_string(),
                company: "Unknown".to_string(),
                location: "Unknown".to_string(),
                remote: false,
                job_type: None,
                salary_range: None,
                description: text,
                requirements: vec![],
                posted_at: None,
                source: "file".to_string(),
                url: String::new(),
                applied: false,
                scraped_at: chrono::Utc::now(),
            };
            let job_id = job.id.clone();
            tracker.save_job(&job)?;
            println!("Job saved as '{}'.", job_id);
            job
        } else {
            anyhow::bail!("Provide --job-id or --file");
        };

        let profile = crate::engine::profile_parser::ProfileParser::parse(
            crate::engine::profile_parser::ProfileInput::Markdown {
                path: cfg.profile_path.clone(),
            },
        )?;
        let tailor = crate::engine::tailor::Tailor::new(router, crate::engine::prompts::Prompts);

        let resume = tailor.generate_resume(&job, &profile).await?;
        let cover = tailor.generate_cover_letter(&job, &profile).await?;

        println!("=== Tailored Resume ===\n{}", resume);
        println!("\n=== Cover Letter ===\n{}", cover);

        // Persisted so that months later - e.g. ahead of an interview - it's
        // possible to see exactly what was submitted and for which job, even
        // if the original posting has since been taken down or edited.
        let model_used = format!("{:?}/{}", cfg.llm.provider, self.preset);
        tracker.record_application(&job.id, &resume, &cover, &model_used)?;
        println!(
            "\nSaved to your application record for '{} at {}' (job id: {}). Run `atsassin pipeline show --job-id {}` any time to see it again.",
            job.title, job.company, job.id, job.id
        );

        if let Some(output) = &args.output {
            crate::ui::output::OutputEngine::export_markdown(
                &format!("{}\n\n{}", resume, cover),
                output,
            )?;
            println!("Exported to {}", output.display());
        }
        Ok(())
    }

    pub async fn handle_apply(&self, args: &ApplyArgs) -> Result<()> {
        let cfg = self.load_config()?;
        let tracker = crate::pipeline::tracker::PipelineTracker::new(&cfg.database_path)?;
        let application = tracker
            .get_latest_application(&args.job_id)?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No application on file for job {}. Run `atsassin tailor --job-id {}` first.",
                    args.job_id,
                    args.job_id
                )
            })?;

        let profile_text = std::fs::read_to_string(&cfg.profile_path).unwrap_or_default();
        let profile =
            crate::pipeline::actuation::profile_from_application(&profile_text, &application);
        crate::pipeline::actuation::write_apply_kit(&profile, &args.output)?;
        println!(
            "Wrote apply kit to {}. Open apply.js or use the bookmarklet to fill the form. Review before submitting.",
            args.output.display()
        );
        Ok(())
    }

    pub async fn handle_playbook(&self) -> Result<()> {
        let playbook = include_str!("../PLAYBOOK.md");
        println!("{}", playbook);
        Ok(())
    }

    pub async fn handle_pipeline(&self, args: &PipelineArgs) -> Result<()> {
        let cfg = self.load_config()?;
        let tracker = crate::pipeline::tracker::PipelineTracker::new(&cfg.database_path)?;

        match &args.action {
            PipelineAction::List => {
                let entries = tracker.list_pipeline()?;
                if entries.is_empty() {
                    println!("Pipeline is empty. Run `atsassin evaluate --file <jd.txt>` to add jobs, then `atsassin pipeline list` to track them.");
                } else {
                    for entry in entries {
                        println!(
                            "{} | {:?} | contact: {} | follow-up: {} | notes: {}",
                            entry.job_id,
                            entry.status,
                            entry.contact.as_deref().unwrap_or("-"),
                            entry
                                .follow_up_date
                                .map(|d| d.format("%Y-%m-%d").to_string())
                                .unwrap_or_else(|| "-".to_string()),
                            entry.notes.as_deref().unwrap_or("-"),
                        );
                    }
                }
            }
            PipelineAction::Add { job_id, status } => {
                let status = match status {
                    Some(s) => crate::models::job::PipelineStatus::parse(s)?,
                    None => crate::models::job::PipelineStatus::New,
                };
                if tracker.get_job(job_id)?.is_none() {
                    println!("Warning: Job '{}' not found in database. Add it first with `atsassin evaluate --file <jd.txt>`.", job_id);
                }
                let entry = tracker.add_pipeline_entry(job_id, status)?;
                println!(
                    "Added pipeline entry {} with status {:?}",
                    entry.id, entry.status
                );
            }
            PipelineAction::Update {
                job_id,
                status,
                notes,
                contact,
                follow_up,
            } => {
                let status = match status {
                    Some(s) => Some(crate::models::job::PipelineStatus::parse(s)?),
                    None => None,
                };
                let follow_up_date = match follow_up {
                    Some(s) => Some(parse_follow_up_date(s)?),
                    None => None,
                };
                if status.is_none()
                    && notes.is_none()
                    && contact.is_none()
                    && follow_up_date.is_none()
                {
                    anyhow::bail!("Provide at least one of --status, --notes, --contact, --follow-up to update.");
                }
                let rows = tracker.update_pipeline_fields(
                    job_id,
                    status,
                    notes.as_deref(),
                    contact.as_deref(),
                    follow_up_date,
                )?;
                if rows == 0 {
                    anyhow::bail!("No pipeline entry found for job '{}'. Run `atsassin pipeline add --job-id {} --status new` first.", job_id, job_id);
                }
                println!("Updated {} pipeline entry(s) for job '{}'", rows, job_id);
            }
            PipelineAction::Export { output } => {
                let entries = tracker.list_pipeline()?;
                let mut wtr = csv::Writer::from_path(output)?;
                wtr.write_record([
                    "job_id",
                    "status",
                    "title",
                    "company",
                    "url",
                    "notes",
                    "contact",
                    "created_at",
                    "updated_at",
                ])?;
                for entry in &entries {
                    let job = tracker.get_job(&entry.job_id)?;
                    let (title, company, url) = match job {
                        Some(j) => (j.title, j.company, j.url),
                        None => ("Unknown".to_string(), String::new(), String::new()),
                    };
                    wtr.write_record([
                        &entry.job_id,
                        &format!("{:?}", entry.status),
                        &title,
                        &company,
                        &url,
                        &entry.notes.clone().unwrap_or_default(),
                        &entry.contact.clone().unwrap_or_default(),
                        &entry.created_at.to_rfc3339(),
                        &entry.updated_at.to_rfc3339(),
                    ])?;
                }
                wtr.flush()?;
                println!("Exported {} entries to {}", entries.len(), output.display());
            }
            PipelineAction::Show { job_id } => {
                let job = tracker
                    .get_job(job_id)?
                    .ok_or_else(|| anyhow::anyhow!("Job '{}' not found in database.", job_id))?;
                println!("=== {} at {} ===", job.title, job.company);
                println!("Location: {}", job.location);
                println!("URL: {}", job.url);
                println!("Job ID: {}\n", job.id);

                let status = tracker
                    .list_pipeline()?
                    .into_iter()
                    .find(|e| &e.job_id == job_id)
                    .map(|e| format!("{:?}", e.status));
                println!(
                    "Pipeline status: {}",
                    status.unwrap_or_else(|| "(not in pipeline)".to_string())
                );

                println!(
                    "\n--- Job description (as scraped) ---\n{}",
                    job.description
                );

                match tracker.get_latest_application(job_id)? {
                    Some(app) => {
                        println!(
                            "\n--- What you submitted (generated {}) ---",
                            app.generated_at.to_rfc3339()
                        );
                        println!("\n=== Resume ===\n{}", app.resume_text);
                        println!("\n=== Cover Letter ===\n{}", app.cover_letter_text);
                    }
                    None => {
                        println!("\nNo tailored application on file for this job yet - run `atsassin tailor --job-id {}` to generate and save one.", job_id);
                    }
                }
            }
        }

        Ok(())
    }

    /// Ranks every job scraped so far (across all past `scan` runs, not just
    /// the last one) by how likely it is to land quickly - see
    /// `engine::landscore` for the composite formula and its rationale.
    pub async fn handle_recommend(&self, args: &RecommendArgs) -> Result<()> {
        let cfg = self.load_config()?;
        let tracker = crate::pipeline::tracker::PipelineTracker::new(&cfg.database_path)?;
        let rows = tracker.list_job_rows(args.pool)?;
        if rows.is_empty() {
            println!("No jobs scraped yet. Run `atsassin scan --role \"<role>\" --location \"<location>\"` first.");
            return Ok(());
        }

        let profile = if cfg.profile_path.exists() {
            ProfileParser::parse(ProfileInput::Markdown {
                path: cfg.profile_path.clone(),
            })
            .ok()
        } else {
            None
        };

        // One batch prerank pass over every pooled job so the lexical IDF
        // weighting is computed against a real, large corpus - the same
        // reason `scan` ranks its whole batch at once rather than job by job.
        let relevance: Vec<f64> = if let Some(profile) = &profile {
            let ranked = crate::engine::prerank::rank(profile, &rows, |r| {
                format!("{} {}", r.title, r.description)
            });
            let mut by_index = vec![0.0; rows.len()];
            for (idx, score) in ranked {
                by_index[idx] = score;
            }
            by_index
        } else {
            vec![0.0; rows.len()]
        };

        let now = chrono::Utc::now();
        let mut scored: Vec<(
            f64,
            crate::engine::landscore::LandScore,
            &crate::models::job::JobRow,
        )> = rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let pref_match = crate::engine::preferences::check(row, &cfg.preferences);
                // overall_score is stored on a 0..1 scale (see Evaluation).
                let eval_score = row.overall_score;
                let text = format!("{} {}", row.title, row.description);
                let ls = crate::engine::landscore::score(
                    relevance[i],
                    &pref_match,
                    eval_score,
                    row.posted_at,
                    now,
                    &text,
                );
                (ls.composite, ls, row)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        if profile.is_none() {
            println!("(No profile found - ranking on preferences/recency/contract-signal only, not fit. Run `atsassin profile init` for full ranking.)\n");
        }
        println!(
            "=== Top {} jobs most likely to land quickly ===\n",
            args.limit.min(scored.len())
        );
        for (composite, ls, row) in scored.iter().take(args.limit) {
            let fit_label = match ls.eval_score_pct {
                Some(pct) => format!("LLM-scored fit {pct:.0}%"),
                None => format!("lexical fit {:.0}%", ls.relevance_pct),
            };
            let recency_label = match ls.posted_days_ago {
                Some(0) => "posted today".to_string(),
                Some(d) => format!("posted {d}d ago"),
                None => "posted date unknown".to_string(),
            };
            println!(
                "  {composite:.0}  {} at {} ({})",
                row.title, row.company, row.location
            );
            println!(
                "       {fit_label} | {recency_label}{}{}",
                if ls.contract_signal {
                    " | contract/interim/fractional language"
                } else {
                    ""
                },
                if !ls.pref_match {
                    format!(" | prefs: {}", ls.pref_reasons.join("; "))
                } else {
                    String::new()
                }
            );
            println!("       id: {}", row.id);
        }
        println!("\nRun `atsassin evaluate --job-id <id>` on unscored ones to sharpen this ranking with a real LLM score.");
        Ok(())
    }

    pub async fn handle_tui(&self, args: &TuiArgs) -> Result<()> {
        let cfg = self.load_config()?;
        let telemetry_path = cfg.database_path.with_extension("llm_telemetry.jsonl");
        let router = ModelRouter::from_llm_config(
            &cfg.llm,
            cfg.tiers.light.clone(),
            cfg.tiers.balanced.clone(),
            cfg.tiers.full.clone(),
            Some(telemetry_path),
        );
        let tracker = crate::pipeline::tracker::PipelineTracker::new(&cfg.database_path)?;

        let profile_path = if cfg.profile_path.exists() {
            Some(cfg.profile_path.clone())
        } else {
            None
        };
        let profile = profile_path
            .and_then(|p| ProfileParser::parse(ProfileInput::Markdown { path: p }).ok());

        let tui_cfg = crate::ui::tui::TuiConfig {
            db_path: cfg.database_path.clone(),
            provider_label: cfg.llm.provider.as_str().to_string(),
            model_label: cfg.llm.default_model.clone(),
            mode_label: self.preset.clone(),
            profile,
            router,
            rate_limit_ms: cfg.scraping.rate_limit_ms,
            user_agent: cfg.scraping.user_agent.clone(),
            boards: cfg.scraping.boards.clone(),
            scan_limit: cfg.scraping.max_results_per_board.min(15),
            ascii: args.ascii,
            preferences: cfg.preferences.clone(),
        };

        let mut dashboard = crate::ui::tui::TuiDashboard::new(tracker, tui_cfg);
        dashboard.run().await
    }

    pub async fn handle_distill(&self, args: &DistillArgs) -> Result<()> {
        let cfg = self.load_config()?;
        let profile_text = if let Some(path) = &args.profile {
            std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read profile: {}", path.display()))?
        } else {
            let profile_path = std::path::Path::new("profile.md");
            if !profile_path.exists() {
                anyhow::bail!("No profile provided. Run `atsassin profile init` first or pass --profile <path>");
            }
            std::fs::read_to_string(profile_path)?
        };

        let profile =
            crate::engine::profile_parser::ProfileParser::profile_from_text(&profile_text)?;
        let role_archetypes = if args.roles > 0 {
            let telemetry_path = cfg.database_path.with_extension("llm_telemetry.jsonl");
            let router = crate::engine::router::ModelRouter::from_llm_config(
                &cfg.llm,
                cfg.tiers.light.clone(),
                cfg.tiers.balanced.clone(),
                cfg.tiers.full.clone(),
                Some(telemetry_path),
            );
            let engine = crate::engine::role_inference::RoleInferenceEngine::new(router);
            let mut roles = engine.infer_roles(&profile).await?;
            roles.truncate(args.roles);
            roles
        } else {
            vec![]
        };
        let roles: Vec<String> = role_archetypes.into_iter().map(|r| r.title).collect();

        let output_dir = &args.output;
        std::fs::create_dir_all(output_dir)?;

        crate::engine::distillation::DistillationPipeline::export_training_data(
            &profile_text,
            &roles,
            output_dir,
        )?;
        let journal_path = cfg.database_path.with_extension("llm_telemetry.jsonl");
        let pair_count =
            crate::engine::distillation::DistillationPipeline::export_from_feedback_and_telemetry(
                &cfg.database_path,
                &journal_path,
                output_dir,
            )?;
        println!("Exported {} high-confidence training pair(s).", pair_count);

        let manifest = output_dir.join("manifest.json");
        println!("Exported distillation data to: {}", output_dir.display());
        println!("Manifest: {}", manifest.display());
        println!("Roles inferred: {}", roles.len());

        if matches!(cfg.llm.provider, crate::config::LlmProvider::Lightning) {
            println!("Lightning AI provider detected. Submitting distillation job to Lightning AI training endpoint...");
            println!("NOTE: Lightning AI SDK integration is stubbed. In production, this would submit the JSONL to Lightning AI for training.");
        }

        Ok(())
    }

    pub async fn handle_feedback(&self, args: &FeedbackArgs) -> Result<()> {
        let cfg = self.load_config()?;
        let tracker = crate::engine::feedback::FeedbackTracker::new(&cfg.database_path)?;

        match &args.action {
            FeedbackActionCli::Record {
                job_id,
                task,
                action,
                recommendation,
                edited,
                confidence_before,
                confidence_after,
            } => {
                let task_enum = match task.to_lowercase().as_str() {
                    "scoring" => crate::engine::feedback::FeedbackTask::Scoring,
                    "tailoring" => crate::engine::feedback::FeedbackTask::Tailoring,
                    "cover_letter" | "coverletter" => {
                        crate::engine::feedback::FeedbackTask::CoverLetter
                    }
                    "deep_research" | "deepresearch" => {
                        crate::engine::feedback::FeedbackTask::DeepResearch
                    }
                    "role_inference" | "roleinference" => {
                        crate::engine::feedback::FeedbackTask::RoleInference
                    }
                    _ => crate::engine::feedback::FeedbackTask::Scoring,
                };
                let action_enum = match action.to_lowercase().as_str() {
                    "accepted" => crate::engine::feedback::FeedbackAction::Accepted,
                    "edited" => crate::engine::feedback::FeedbackAction::Edited,
                    "ignored" => crate::engine::feedback::FeedbackAction::Ignored,
                    "escalated" => crate::engine::feedback::FeedbackAction::Escalated,
                    _ => crate::engine::feedback::FeedbackAction::Accepted,
                };

                let id = tracker.record_feedback(
                    job_id,
                    task_enum,
                    action_enum,
                    recommendation,
                    edited.as_deref(),
                    *confidence_before,
                    *confidence_after,
                )?;
                println!("Recorded feedback event ID: {}", id);
                Ok(())
            }
            FeedbackActionCli::Stats { days } => {
                println!(
                    "=== Feedback & Self-Optimization Stats (Last {} days) ===",
                    days
                );
                let tasks = [
                    crate::engine::feedback::FeedbackTask::Scoring,
                    crate::engine::feedback::FeedbackTask::Tailoring,
                    crate::engine::feedback::FeedbackTask::CoverLetter,
                    crate::engine::feedback::FeedbackTask::DeepResearch,
                    crate::engine::feedback::FeedbackTask::RoleInference,
                ];

                for t in tasks {
                    let rate = tracker.get_acceptance_rate(t.clone(), *days)?;
                    let edit_stats = tracker.get_edit_distance_stats(t.clone())?;
                    let escalate = tracker.should_escalate_automation(t.clone())?;

                    println!("Task: {:<15} | Acceptance Rate: {:>5.1}% | Avg Edit Dist: {:>5.1} | Auto-Escalate: {}",
                        t.to_string(),
                        rate * 100.0,
                        edit_stats.map(|(m, _)| m).unwrap_or(0.0),
                        if escalate { "YES" } else { "NO" }
                    );
                }
                Ok(())
            }
            FeedbackActionCli::Recent { limit } => {
                let events = tracker.list_recent(*limit)?;
                println!("=== Recent {} Feedback Events ===", limit);
                for ev in events {
                    println!("[{}] Job: {} | Task: {:?} | Action: {:?} | EditDist: {:?} | Confidence: {:.2}->{:.2}",
                        ev.created_at, ev.job_id, ev.task_type, ev.action, ev.edit_distance, ev.confidence_before, ev.confidence_after
                    );
                }
                Ok(())
            }
            FeedbackActionCli::ShouldEscalate { task } => {
                let task_enum = match task.to_lowercase().as_str() {
                    "scoring" => crate::engine::feedback::FeedbackTask::Scoring,
                    "tailoring" => crate::engine::feedback::FeedbackTask::Tailoring,
                    "cover_letter" => crate::engine::feedback::FeedbackTask::CoverLetter,
                    "deep_research" => crate::engine::feedback::FeedbackTask::DeepResearch,
                    "role_inference" => crate::engine::feedback::FeedbackTask::RoleInference,
                    _ => crate::engine::feedback::FeedbackTask::Scoring,
                };
                let should = tracker.should_escalate_automation(task_enum)?;
                println!(
                    "Task '{}' automation escalation recommendation: {}",
                    task,
                    if should {
                        "ESCALATE (High precision)"
                    } else {
                        "REMAIN TIER (Human review recommended)"
                    }
                );
                Ok(())
            }
        }
    }

    /// `assets/data/market_stats_2026.json` is checked into the repo next
    /// to the source, but the installed/distributed binary is run from
    /// wherever the user put it - a bare relative path only resolves when
    /// the current directory happens to be the repo root (true for `cargo
    /// run`, false for the "single binary" usage this project advertises).
    /// Checks CWD first (keeps the dev workflow working unchanged), then
    /// falls back to the directory the executable itself lives in.
    fn find_market_stats_json() -> Option<String> {
        let rel = "assets/data/market_stats_2026.json";
        if let Ok(s) = std::fs::read_to_string(rel) {
            return Some(s);
        }
        let exe_relative = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(rel)))?;
        std::fs::read_to_string(exe_relative).ok()
    }

    pub async fn handle_market(&self, args: &MarketArgs) -> Result<()> {
        match &args.action {
            MarketAction::Stats => {
                // Issue #4 — reads assets/data/market_stats_2026.json,
                // which structures the same numbers as before into a
                // separate file instead of inline string literals. NOTE:
                // this file has no source/citation attached anywhere in
                // the repo (unlike assets/data/llm_providers_2026.json,
                // which cites a URL per entry) - moving numbers into JSON
                // does not make them sourced. Keep the "illustrative"
                // framing honest until this file has real citations or a
                // documented refresh pipeline; see issue #4.
                let stats = Self::find_market_stats_json()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
                println!("NOTE: Illustrative estimates - not sourced/verified live market data.");
                println!("=== 2026 Tech Market Intelligence & Rate Benchmarks ===");
                if let Some(s) = stats.as_ref() {
                    let apac = &s["apac_tech_hiring"];
                    let pm = &s["pm_gtm_trends"];
                    let time = &s["time_to_hire"];
                    let ai = &s["ai_impact"];
                    println!(
                        "APAC Q3 2026 hiring intention: {}% (SG {}% / HK {}% / JP {}% / IN {}%)",
                        apac["q3_2026_intention_pct"].as_i64().unwrap_or(0),
                        apac["singapore_pct"].as_i64().unwrap_or(0),
                        apac["hong_kong_pct"].as_i64().unwrap_or(0),
                        apac["japan_pct"].as_i64().unwrap_or(0),
                        apac["india_pct"].as_i64().unwrap_or(0),
                    );
                    println!(
                        "PM/GTM signal: {} open PM roles worldwide (+{}x since 2023); {}% of senior PM roles now require AI experience",
                        pm["open_pm_positions_worldwide_mar_2026"].as_i64().unwrap_or(0),
                        pm["increase_since_2023_factor"].as_f64().unwrap_or(0.0),
                        pm["pm_requiring_ai_experience_pct"].as_i64().unwrap_or(0),
                    );
                    println!(
                        "Time to hire: Global {} days | Tech Eng {} days | AU Senior IC {} days",
                        time["global_median_days"]
                            .as_array()
                            .and_then(|a| a.first())
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0),
                        time["tech_engineering_days"]
                            .as_array()
                            .and_then(|a| a.first())
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0),
                        time["australia_senior_ic_median_days"]
                            .as_i64()
                            .unwrap_or(0),
                    );
                    println!(
                        "AI impact: ~{}% auto-rejected before human review; ATS \u{2265}75 callback factor {:.1}x",
                        ai["auto_rejection_before_human_review_pct"].as_i64().unwrap_or(0),
                        ai["ats_score_75_plus_callback_improvement_factor"].as_f64().unwrap_or(0.0),
                    );
                } else {
                    println!("High Demand Roles: AI System Engineer, Lead Rust Architect, Senior Product Manager (APAC)");
                }
                Ok(())
            }
            MarketAction::Rates { role } => {
                let query = role.as_deref().unwrap_or("Software Architect");
                println!("NOTE: Illustrative estimates - not sourced/verified live market data.");
                println!("=== Compensation Benchmarks for: {} ===", query);
                let stats = Self::find_market_stats_json()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
                if let Some(s) = stats.as_ref() {
                    let rates = &s["contract_rates"];
                    println!(
                        "ANZ/SG contract: AUD ${}-${}/day; contractor FTE premium gross {}-{}% (net {}-{}%)",
                        rates["australia_day_rate_range_aud"].as_array().and_then(|a| a.first()).and_then(|v| v.as_i64()).unwrap_or(0),
                        rates["australia_day_rate_range_aud"].as_array().and_then(|a| a.get(1)).and_then(|v| v.as_i64()).unwrap_or(0),
                        rates["contractor_fte_premium_gross_pct"].as_array().and_then(|a| a.first()).and_then(|v| v.as_i64()).unwrap_or(0),
                        rates["contractor_fte_premium_gross_pct"].as_array().and_then(|a| a.get(1)).and_then(|v| v.as_i64()).unwrap_or(0),
                        rates["contractor_fte_premium_net_pct"].as_array().and_then(|a| a.first()).and_then(|v| v.as_i64()).unwrap_or(0),
                        rates["contractor_fte_premium_net_pct"].as_array().and_then(|a| a.get(1)).and_then(|v| v.as_i64()).unwrap_or(0),
                    );
                } else {
                    println!("Full-Time Equivalent (USD/yr): $160,000 - $240,000");
                    println!("Contract Rate (USD/hr): $95 - $165 / hr");
                    println!("Day Rate (AUD/day, ANZ market): $1,200 - $1,800 / day");
                }
                Ok(())
            }
        }
    }

    pub async fn handle_preferences(&self, args: &PreferencesArgs) -> Result<()> {
        let mut cfg = self.load_config()?;
        match &args.action {
            PreferencesAction::Show => {
                println!("=== Job Search Preferences ===");
                println!(
                    "Min compensation (USD/yr): {}",
                    cfg.preferences
                        .min_comp_usd
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "Any".to_string())
                );
                println!("Employment type: {:?}", cfg.preferences.employment_type);
                println!("Work mode: {:?}", cfg.preferences.work_mode);
                println!("\nSet with: atsassin preferences set --min-comp 150000 --employment-type contract --work-mode remote");
            }
            PreferencesAction::Set {
                min_comp,
                employment_type,
                work_mode,
            } => {
                if let Some(v) = min_comp {
                    cfg.preferences.min_comp_usd = Some(*v);
                }
                if let Some(s) = employment_type {
                    cfg.preferences.employment_type = match s.to_lowercase().as_str() {
                        "any" => crate::config::EmploymentTypePref::Any,
                        "fulltime" | "full-time" | "full_time" => {
                            crate::config::EmploymentTypePref::FullTimeOnly
                        }
                        "contract" => crate::config::EmploymentTypePref::ContractOnly,
                        other => anyhow::bail!(
                            "Invalid --employment-type '{}'. Valid: any, fulltime, contract",
                            other
                        ),
                    };
                }
                if let Some(s) = work_mode {
                    cfg.preferences.work_mode = match s.to_lowercase().as_str() {
                        "any" => crate::config::WorkModePref::Any,
                        "remote" => crate::config::WorkModePref::RemoteOnly,
                        "hybrid" => crate::config::WorkModePref::HybridOrRemote,
                        "onsite" => crate::config::WorkModePref::OnsiteOk,
                        other => anyhow::bail!(
                            "Invalid --work-mode '{}'. Valid: any, remote, hybrid, onsite",
                            other
                        ),
                    };
                }
                cfg.save(&self.config)?;
                println!("Preferences saved to {}", self.config.display());
                println!("They'll now be applied by `atsassin scan --prefs-only` and shown in `atsassin tui`.");
            }
        }
        Ok(())
    }
}
