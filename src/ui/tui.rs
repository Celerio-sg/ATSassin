use anyhow::Result;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Gauge, List, ListItem, Paragraph, Row, Table,
        TableState, Wrap,
    },
    Terminal,
};
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::JobPreferences;
use crate::engine::hardware::HardwareProfile;
use crate::engine::role_inference::RoleInferenceEngine;
use crate::engine::router::ModelRouter;
use crate::models::job::{ActivityEvent, Evaluation, Job, JobRow, PipelineStatus};
use crate::models::profile::UserProfile;
use crate::models::role::RoleArchetype;
use crate::pipeline::scraper::Scraper;
use crate::pipeline::tracker::PipelineTracker;

/// Everything the TUI needs to do real work (scan, infer roles, evaluate,
/// tailor, show which provider/model/preset is actually configured) that
/// isn't already in the PipelineTracker it's given.
pub struct TuiConfig {
    pub db_path: PathBuf,
    pub provider_label: String,
    pub model_label: String,
    pub mode_label: String,
    pub profile: Option<UserProfile>,
    pub router: ModelRouter,
    pub rate_limit_ms: u64,
    pub user_agent: String,
    pub boards: Vec<String>,
    pub scan_limit: usize,
    /// ASCII-only rendering (no emoji/spinner glyphs) for terminals/codepages
    /// that mangle them - most legacy Windows consoles and some SSH clients.
    pub ascii: bool,
    pub preferences: JobPreferences,
}

const REGIONS: [&str; 4] = ["Global", "North America", "Europe", "APAC"];
const SPINNER_UNICODE: [&str; 10] = [
    "\u{280B}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283C}", "\u{2834}", "\u{2826}", "\u{2827}",
    "\u{2807}", "\u{280F}",
];
const SPINNER_ASCII: [&str; 4] = ["|", "/", "-", "\\"];

enum TuiEvent {
    ScanBoardStarted(String),
    ScanBoardDone(String, usize),
    ScanComplete,
    ScanError(String),
    RolesInferred(Vec<RoleArchetype>),
    RolesError(String),
    EvaluateDone(f64, String),
    EvaluateError(String),
    TailorDone(PathBuf),
    TailorError(String),
}

pub struct TuiDashboard {
    tracker: PipelineTracker,
    cfg: TuiConfig,

    jobs: Vec<JobRow>,
    /// Indices into `jobs` currently shown in the table - all of them
    /// unless `prefs_filter_on`, in which case only jobs that pass
    /// `preferences::check`. Recomputed by `recompute_view()`.
    visible_indices: Vec<usize>,
    prefs_filter_on: bool,
    roles: Vec<RoleArchetype>,
    activity: Vec<ActivityEvent>,
    pipeline_counts: Vec<(PipelineStatus, usize)>,
    job_state: TableState,
    selected_eval: Option<Evaluation>,

    region_idx: usize,
    spinner_frame: usize,

    scanning: bool,
    scan_log: Vec<String>,
    scan_boards_done: usize,
    scan_boards_total: usize,
    scan_found_total: usize,
    inferring_roles: bool,
    evaluating: bool,
    tailoring: bool,
    status: Option<String>,

    sys: sysinfo::System,

    tx: mpsc::UnboundedSender<TuiEvent>,
    rx: mpsc::UnboundedReceiver<TuiEvent>,
}

const COLOR_BORDER_ACTIVE: Color = Color::Rgb(0, 220, 180);
const COLOR_MINT: Color = Color::Rgb(80, 230, 150);
const COLOR_CYAN: Color = Color::Rgb(0, 210, 225);
const COLOR_GOLD: Color = Color::Rgb(240, 200, 80);
const COLOR_AMBER: Color = Color::Rgb(235, 160, 60);
const COLOR_RED: Color = Color::Rgb(230, 90, 90);
const COLOR_DIM: Color = Color::Rgb(120, 130, 140);
const COLOR_PANEL_BORDER: Color = Color::Rgb(45, 70, 80);

/// Semantic color for a 0.0-1.0 match score - always paired with the numeric
/// value in the UI, never color-only, so it stays meaningful in terminals
/// without color and stays grep-able in captured output.
fn score_color(score: f64) -> Color {
    if score >= 0.8 {
        COLOR_MINT
    } else if score >= 0.6 {
        COLOR_AMBER
    } else {
        COLOR_RED
    }
}

fn bar(fraction: f64, width: usize) -> String {
    let filled = ((fraction.clamp(0.0, 1.0)) * width as f64).round() as usize;
    format!(
        "{}{}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(width.saturating_sub(filled))
    )
}

impl TuiDashboard {
    pub fn new(tracker: PipelineTracker, cfg: TuiConfig) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let jobs = tracker.list_job_rows(50).unwrap_or_default();
        let roles = tracker.list_roles(10).unwrap_or_default();
        let activity = tracker.recent_activity(10).unwrap_or_default();
        let pipeline_counts = tracker.pipeline_status_counts().unwrap_or_default();
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        let mut job_state = TableState::default();
        if !jobs.is_empty() {
            job_state.select(Some(0));
        }
        let visible_indices: Vec<usize> = (0..jobs.len()).collect();
        let selected_eval = job_state
            .selected()
            .and_then(|i| visible_indices.get(i))
            .and_then(|&idx| jobs.get(idx))
            .and_then(|j| tracker.get_latest_evaluation(&j.id).ok().flatten());

        Self {
            tracker,
            cfg,
            jobs,
            visible_indices,
            prefs_filter_on: false,
            roles,
            activity,
            pipeline_counts,
            job_state,
            selected_eval,
            region_idx: 0,
            spinner_frame: 0,
            scanning: false,
            scan_log: Vec::new(),
            scan_boards_done: 0,
            scan_boards_total: 0,
            scan_found_total: 0,
            inferring_roles: false,
            evaluating: false,
            tailoring: false,
            status: None,
            sys,
            tx,
            rx,
        }
    }

    fn refresh_from_db(&mut self) {
        self.jobs = self.tracker.list_job_rows(50).unwrap_or_default();
        self.activity = self.tracker.recent_activity(10).unwrap_or_default();
        self.pipeline_counts = self.tracker.pipeline_status_counts().unwrap_or_default();
        self.recompute_view();
        if self.job_state.selected().is_none() && !self.visible_indices.is_empty() {
            self.job_state.select(Some(0));
        }
        self.load_selected_eval();
    }

    /// Recomputes which job indices are visible given `prefs_filter_on`.
    /// Real, local, honest filtering over already-scraped fields - see
    /// `engine::preferences`. Never touches the underlying `jobs` data.
    fn recompute_view(&mut self) {
        self.visible_indices = if self.prefs_filter_on {
            self.jobs
                .iter()
                .enumerate()
                .filter(|(_, j)| {
                    crate::engine::preferences::check(*j, &self.cfg.preferences).matches
                })
                .map(|(i, _)| i)
                .collect()
        } else {
            (0..self.jobs.len()).collect()
        };
        let len = self.visible_indices.len();
        match self.job_state.selected() {
            Some(i) if i >= len && len > 0 => self.job_state.select(Some(len - 1)),
            Some(_) if len == 0 => self.job_state.select(None),
            None if len > 0 => self.job_state.select(Some(0)),
            _ => {}
        }
    }

    fn toggle_prefs_filter(&mut self) {
        self.prefs_filter_on = !self.prefs_filter_on;
        self.recompute_view();
        self.load_selected_eval();
        self.status = Some(if self.prefs_filter_on {
            format!(
                "Preference filter ON - showing {} of {} job(s).",
                self.visible_indices.len(),
                self.jobs.len()
            )
        } else {
            "Preference filter OFF - showing all jobs.".to_string()
        });
    }

    /// Free, local, zero-LLM-cost relevance sort (career-ops/jobsync
    /// "prerank" pattern) - reorders the currently visible jobs by
    /// term-overlap with the real profile so `evaluate`/`tailor` can be
    /// pointed at the strongest candidates first.
    fn sort_by_relevance(&mut self) {
        let Some(profile) = self.cfg.profile.clone() else {
            self.status = Some("No profile loaded - can't compute relevance.".to_string());
            return;
        };
        if self.visible_indices.is_empty() {
            return;
        }
        let visible = self.visible_jobs();
        let ranked = crate::engine::prerank::rank(&profile, &visible, |j: &&JobRow| {
            format!("{} {}", j.title, j.description)
        });
        self.visible_indices = ranked
            .iter()
            .map(|(i, _)| self.visible_indices[*i])
            .collect();
        self.job_state.select(Some(0));
        self.load_selected_eval();
        self.status =
            Some("Sorted by local relevance to your profile (zero LLM calls).".to_string());
    }

    /// Same relevance sort as `sort_by_relevance`, but keeps whichever job
    /// is currently selected (by id) still selected afterward, instead of
    /// jumping to index 0. Used after evaluate/scan-refresh, where the user
    /// didn't ask for a resort and shouldn't have their selection yanked
    /// out from under them - unlike the explicit `x` keybinding, where
    /// jumping to the top is exactly the point.
    fn resort_preserving_selection(&mut self) {
        let Some(profile) = self.cfg.profile.clone() else {
            return;
        };
        if self.visible_indices.is_empty() {
            return;
        }
        let selected_id = self.selected_job_id();
        let visible = self.visible_jobs();
        let ranked = crate::engine::prerank::rank(&profile, &visible, |j: &&JobRow| {
            format!("{} {}", j.title, j.description)
        });
        self.visible_indices = ranked
            .iter()
            .map(|(i, _)| self.visible_indices[*i])
            .collect();
        let restored = selected_id.and_then(|id| {
            self.visible_indices
                .iter()
                .position(|&idx| self.jobs[idx].id == id)
        });
        self.job_state.select(Some(restored.unwrap_or(0)));
        self.load_selected_eval();
    }

    fn load_selected_eval(&mut self) {
        self.selected_eval = self
            .selected_job_id()
            .and_then(|id| self.tracker.get_latest_evaluation(&id).ok().flatten());
    }

    fn selected_job_id(&self) -> Option<String> {
        self.job_state
            .selected()
            .and_then(|i| self.visible_indices.get(i))
            .and_then(|&idx| self.jobs.get(idx))
            .map(|j| j.id.clone())
    }

    fn visible_jobs(&self) -> Vec<&JobRow> {
        self.visible_indices
            .iter()
            .filter_map(|&i| self.jobs.get(i))
            .collect()
    }

    fn icon(&self, emoji: &str, ascii_fallback: &str) -> String {
        if self.cfg.ascii {
            ascii_fallback.to_string()
        } else {
            emoji.to_string()
        }
    }

    fn spinner(&self) -> &'static str {
        if self.cfg.ascii {
            SPINNER_ASCII[self.spinner_frame % SPINNER_ASCII.len()]
        } else {
            SPINNER_UNICODE[self.spinner_frame % SPINNER_UNICODE.len()]
        }
    }

    fn busy(&self) -> bool {
        self.scanning || self.inferring_roles || self.evaluating || self.tailoring
    }

    pub async fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        // Restore the terminal even on panic - without this, a crash mid-render
        // leaves the user's shell stuck in raw mode on the alternate screen.
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            default_hook(info);
        }));

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        let result = self.run_loop(&mut terminal).await;

        let _ = std::panic::take_hook();
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    async fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        let mut tick = tokio::time::interval(Duration::from_millis(500));

        loop {
            while let Ok(event) = self.rx.try_recv() {
                self.handle_tui_event(event);
            }

            if self.busy() {
                self.spinner_frame = self.spinner_frame.wrapping_add(1);
            }

            terminal.draw(|f| self.render(f))?;

            tokio::select! {
                _ = tick.tick() => {
                    self.sys.refresh_cpu_usage();
                    self.sys.refresh_memory();
                }
                _ = tokio::time::sleep(Duration::from_millis(60)) => {}
            }

            let mut quit = false;
            while crossterm::event::poll(Duration::from_millis(0))? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    if key.kind == crossterm::event::KeyEventKind::Press {
                        match key.code {
                            crossterm::event::KeyCode::Char('q') => {
                                quit = true;
                            }
                            crossterm::event::KeyCode::Char('j')
                            | crossterm::event::KeyCode::Down => self.next_row(),
                            crossterm::event::KeyCode::Char('k')
                            | crossterm::event::KeyCode::Up => self.prev_row(),
                            crossterm::event::KeyCode::Char('g') => {
                                self.region_idx = (self.region_idx + 1) % REGIONS.len();
                            }
                            crossterm::event::KeyCode::Char('p') => self.toggle_prefs_filter(),
                            crossterm::event::KeyCode::Char('x') => self.sort_by_relevance(),
                            crossterm::event::KeyCode::Char('R') => self.refresh_from_db(),
                            crossterm::event::KeyCode::Char('r') => self.start_role_inference(),
                            crossterm::event::KeyCode::Char('s') => self.start_scan(),
                            crossterm::event::KeyCode::Char('e') => self.start_evaluate(),
                            crossterm::event::KeyCode::Char('t') => self.start_tailor(),
                            _ => {}
                        }
                    }
                }
                if quit {
                    break;
                }
            }
            if quit {
                break;
            }
        }

        Ok(())
    }

    fn handle_tui_event(&mut self, event: TuiEvent) {
        match event {
            TuiEvent::ScanBoardStarted(board) => {
                self.scan_log.push(format!("Scanning {board}..."));
            }
            TuiEvent::ScanBoardDone(board, count) => {
                self.scan_boards_done += 1;
                self.scan_found_total += count;
                self.scan_log
                    .push(format!("[{board}] found {count} job(s)"));
            }
            TuiEvent::ScanComplete => {
                self.scanning = false;
                self.refresh_from_db();
                // Found via UAT: without this, the job table defaults to
                // most-recently-scraped-first, which regularly put an
                // irrelevant social-aggregator post (e.g. a HackerNews "Ask
                // HN" thread) ahead of real, well-matched postings - so the
                // first `e` a new user pressed evaluated the wrong thing.
                // Auto-sorting by real local relevance fixes the default
                // without requiring the user to know to press 'x' first.
                if self.cfg.profile.is_some() && !self.jobs.is_empty() {
                    self.sort_by_relevance();
                }
                self.status = Some(format!(
                    "Scan complete - {} job(s) found across {} board(s), sorted by relevance.",
                    self.scan_found_total, self.scan_boards_done
                ));
            }
            TuiEvent::ScanError(e) => {
                // A single board failing doesn't stop the others - only
                // ScanComplete (sent once every board has been attempted)
                // closes the modal. Closing it here previously hid an
                // in-progress scan while it kept running invisibly.
                self.scan_boards_done += 1;
                self.scan_log.push(format!("error: {e}"));
            }
            TuiEvent::RolesInferred(roles) => {
                self.inferring_roles = false;
                self.status = Some(format!("Inferred {} role(s).", roles.len()));
                for role in &roles {
                    let _ = self.tracker.save_role(role);
                }
                self.roles = roles;
            }
            TuiEvent::RolesError(e) => {
                self.inferring_roles = false;
                self.status = Some(format!("Role inference failed: {e}"));
            }
            TuiEvent::EvaluateDone(score, grade) => {
                self.evaluating = false;
                self.refresh_from_db();
                // Found via UAT: without this, the table reverted to
                // unsorted (most-recent-first) order right after the job
                // that was just evaluated had been correctly relevance-
                // sorted to the top - so a user glancing at the table
                // afterward saw irrelevant posts back above it, even though
                // the evaluation itself had targeted the right job.
                if self.cfg.profile.is_some() && !self.jobs.is_empty() {
                    self.resort_preserving_selection();
                }
                self.status = Some(format!("Evaluated: {score:.2} ({grade})"));
            }
            TuiEvent::EvaluateError(e) => {
                self.evaluating = false;
                self.status = Some(format!("Evaluation failed: {e}"));
            }
            TuiEvent::TailorDone(path) => {
                self.tailoring = false;
                self.status = Some(format!(
                    "Tailored resume + cover letter saved to {}",
                    path.display()
                ));
                self.refresh_from_db();
            }
            TuiEvent::TailorError(e) => {
                self.tailoring = false;
                self.status = Some(format!("Tailoring failed: {e}"));
            }
        }
    }

    fn start_role_inference(&mut self) {
        if self.inferring_roles {
            return;
        }
        let Some(profile) = self.cfg.profile.clone() else {
            self.status = Some(
                "No profile loaded - run `atsassin profile init --resume <file>` first."
                    .to_string(),
            );
            return;
        };
        self.inferring_roles = true;
        self.status = Some("Inferring roles from your profile...".to_string());

        let router = self.cfg.router.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let engine = RoleInferenceEngine::new(router);
            match engine.infer_roles(&profile).await {
                Ok(roles) => {
                    let _ = tx.send(TuiEvent::RolesInferred(roles));
                }
                Err(e) => {
                    let _ = tx.send(TuiEvent::RolesError(e.to_string()));
                }
            }
        });
    }

    fn start_scan(&mut self) {
        if self.scanning {
            return;
        }
        let base_query = match self.roles.first() {
            Some(r) => r.title.clone(),
            None => {
                self.status = Some("No inferred role yet - scanning with a generic query. Press 'r' first for a targeted scan.".to_string());
                "software".to_string()
            }
        };
        let region = REGIONS[self.region_idx];
        let query = if region == "Global" {
            base_query
        } else {
            format!("{base_query} {region}")
        };

        self.scanning = true;
        self.scan_log.clear();
        self.scan_boards_done = 0;
        self.scan_found_total = 0;
        self.scan_log.push(format!("Query: \"{query}\""));

        let boards = self.cfg.boards.clone();
        self.scan_boards_total = boards.len();
        let limit = self.cfg.scan_limit;
        let rate_limit_ms = self.cfg.rate_limit_ms;
        let user_agent = self.cfg.user_agent.clone();
        let db_path = self.cfg.db_path.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let scraper = Scraper::new(rate_limit_ms, user_agent);
            let tracker = match PipelineTracker::new(&db_path) {
                Ok(t) => t,
                Err(e) => {
                    let _ = tx.send(TuiEvent::ScanError(e.to_string()));
                    let _ = tx.send(TuiEvent::ScanComplete);
                    return;
                }
            };

            for board in boards {
                let _ = tx.send(TuiEvent::ScanBoardStarted(board.clone()));
                let result = match scraper.scrape_board(&board, &query, limit).await {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx.send(TuiEvent::ScanError(format!("{board}: {e}")));
                        continue;
                    }
                };
                for summary in &result.jobs {
                    if summary.url.is_empty() {
                        continue;
                    }
                    let job = Job {
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
                    let _ = tracker.save_job(&job);
                }
                let _ = tx.send(TuiEvent::ScanBoardDone(board, result.jobs.len()));
            }
            let _ = tx.send(TuiEvent::ScanComplete);
        });
    }

    fn start_evaluate(&mut self) {
        if self.evaluating {
            return;
        }
        let Some(job_id) = self.selected_job_id() else {
            self.status = Some("No job selected - scan first with 's'.".to_string());
            return;
        };
        let Some(profile) = self.cfg.profile.clone() else {
            self.status = Some(
                "No profile loaded - run `atsassin profile init --resume <file>` first."
                    .to_string(),
            );
            return;
        };
        let job = match self.tracker.get_job(&job_id) {
            Ok(Some(j)) => j,
            Ok(None) => {
                self.status = Some(format!("Job '{job_id}' not found in database."));
                return;
            }
            Err(e) => {
                self.status = Some(format!("Failed to load job: {e}"));
                return;
            }
        };

        self.evaluating = true;
        self.status = Some(format!("Evaluating \"{}\"...", job.title));

        let router = self.cfg.router.clone();
        let db_path = self.cfg.db_path.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let tracker = match PipelineTracker::new(&db_path) {
                Ok(t) => t,
                Err(e) => {
                    let _ = tx.send(TuiEvent::EvaluateError(e.to_string()));
                    return;
                }
            };
            let scorer =
                crate::engine::scorer::Scorer::new(router, crate::engine::prompts::Prompts);
            match scorer.evaluate(&job, &profile).await {
                Ok(eval) => {
                    let _ = tracker.save_evaluation(&eval);
                    let _ = tracker.add_pipeline_entry(&job.id, PipelineStatus::Evaluated);
                    let _ = tx.send(TuiEvent::EvaluateDone(
                        eval.overall_score,
                        eval.overall_grade,
                    ));
                }
                Err(e) => {
                    let _ = tx.send(TuiEvent::EvaluateError(e.to_string()));
                }
            }
        });
    }

    fn start_tailor(&mut self) {
        if self.tailoring {
            return;
        }
        let Some(job_id) = self.selected_job_id() else {
            self.status = Some("No job selected - scan first with 's'.".to_string());
            return;
        };
        let Some(profile) = self.cfg.profile.clone() else {
            self.status = Some(
                "No profile loaded - run `atsassin profile init --resume <file>` first."
                    .to_string(),
            );
            return;
        };
        let job = match self.tracker.get_job(&job_id) {
            Ok(Some(j)) => j,
            Ok(None) => {
                self.status = Some(format!("Job '{job_id}' not found in database."));
                return;
            }
            Err(e) => {
                self.status = Some(format!("Failed to load job: {e}"));
                return;
            }
        };

        self.tailoring = true;
        self.status = Some(format!(
            "Tailoring resume + cover letter for \"{}\"...",
            job.title
        ));

        let router = self.cfg.router.clone();
        let db_path = self.cfg.db_path.clone();
        let tx = self.tx.clone();
        let short_id: String = job.id.chars().take(8).collect();
        let output_path = PathBuf::from(format!("tailored_{short_id}.md"));
        tokio::spawn(async move {
            let tailor =
                crate::engine::tailor::Tailor::new(router, crate::engine::prompts::Prompts);
            let resume = match tailor.generate_resume(&job, &profile).await {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(TuiEvent::TailorError(e.to_string()));
                    return;
                }
            };
            let cover = match tailor.generate_cover_letter(&job, &profile).await {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(TuiEvent::TailorError(e.to_string()));
                    return;
                }
            };
            let combined = format!("{resume}\n\n---\n\n{cover}");
            if let Err(e) =
                crate::ui::output::OutputEngine::export_markdown(&combined, &output_path)
            {
                let _ = tx.send(TuiEvent::TailorError(e.to_string()));
                return;
            }
            if let Ok(tracker) = PipelineTracker::new(&db_path) {
                let _ = tracker.add_pipeline_entry(&job.id, PipelineStatus::Drafted);
            }
            let _ = tx.send(TuiEvent::TailorDone(output_path));
        });
    }

    /// Real, state-driven onboarding checklist - each item reflects an
    /// actual DB/config check, never a scripted assumption about where the
    /// user is. Returns `None` once every step has genuinely happened, so
    /// it gets out of the way for an established user.
    fn onboarding_line(&self) -> Option<String> {
        let has_profile = self.cfg.profile.is_some();
        let has_roles = !self.roles.is_empty();
        let has_jobs = !self.jobs.is_empty();
        let has_eval = self.jobs.iter().any(|j| j.overall_score.is_some());
        let has_draft = self
            .pipeline_counts
            .iter()
            .any(|(s, c)| matches!(s, PipelineStatus::Drafted) && *c > 0);

        if has_profile && has_roles && has_jobs && has_eval && has_draft {
            return None;
        }
        let mark = |done: bool| if done { "[x]" } else { "[ ]" };
        Some(format!(
            " Getting started: {} Profile   {} Infer roles [r]   {} Scan jobs [s]   {} Evaluate [e]   {} Tailor [t]",
            mark(has_profile), mark(has_roles), mark(has_jobs), mark(has_eval), mark(has_draft),
        ))
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let onboarding = self.onboarding_line();
        let header_height = if onboarding.is_some() { 4 } else { 3 };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(header_height),
                Constraint::Min(10),
                Constraint::Length(1),
            ])
            .split(f.area());

        self.render_header(f, chunks[0], onboarding.as_deref());

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(26),
                Constraint::Percentage(48),
                Constraint::Percentage(26),
            ])
            .split(chunks[1]);

        self.render_left(f, body[0]);
        self.render_center(f, body[1]);
        self.render_right(f, body[2]);

        self.render_footer(f, chunks[2]);

        // Scoped to the center (jobs) panel, not the whole frame - a
        // full-frame-centered modal bled into the left profile/roles panel
        // and cut text mid-line (found via PTY probe testing).
        if self.scanning {
            self.render_scan_modal(f, body[1]);
        }
    }

    fn render_header(&self, f: &mut ratatui::Frame, area: Rect, onboarding: Option<&str>) {
        let hw = HardwareProfile::global();
        let robot = self.icon("\u{1F916}", "AI");
        let disk = self.icon("\u{1F4BE}", "DB");
        let gauge_icon = self.icon("\u{1F5A5}\u{FE0F}", "HW");

        let cpu_pct = self.sys.global_cpu_usage();
        let ram_used_gb = self.sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
        let ram_total_gb = self.sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

        let text = format!(
            " {robot} {}: {} | Mode: {} | {disk} SQLite: {} | {gauge_icon} CPU {:.0}% RAM {:.1}/{:.1}GB | GPU: {}",
            self.cfg.provider_label,
            self.cfg.model_label,
            self.cfg.mode_label,
            self.cfg.db_path.display(),
            cpu_pct,
            ram_used_gb,
            ram_total_gb,
            if hw.has_gpu { format!("{}GB VRAM", hw.gpu_vram_gb.unwrap_or(0)) } else { "none".to_string() },
        );

        let block = Block::default()
            .title(" ATSassin - The Silent Killer of Bad Job Matches ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_MINT));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines = vec![Line::from(Span::styled(
            text,
            Style::default().fg(COLOR_CYAN),
        ))];
        if let Some(ob) = onboarding {
            lines.push(Line::styled(
                ob.to_string(),
                Style::default().fg(COLOR_GOLD),
            ));
        }
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn render_left(&self, f: &mut ratatui::Frame, area: Rect) {
        let block = Block::default()
            .title(" Profile & Archetypes ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_PANEL_BORDER));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();
        match &self.cfg.profile {
            Some(p) => {
                lines.push(Line::from(format!(
                    "{} {}",
                    self.icon("\u{2705}", "[x]"),
                    p.name
                )));
                lines.push(Line::from(format!(
                    "{} skills, {} roles worked",
                    p.skills.len(),
                    p.experience.len()
                )));
                if let Some(loc) = &p.location {
                    lines.push(Line::from(loc.clone()));
                }
            }
            None => {
                lines.push(Line::from("No profile loaded."));
                lines.push(Line::from("Run: atsassin profile init --resume <file>"));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::styled(
            format!("Region filter ([g] to cycle): {}", REGIONS[self.region_idx]),
            Style::default().fg(COLOR_GOLD),
        ));
        lines.push(Line::styled(
            format!(
                "Prefs filter ([p] to toggle): {}",
                if self.prefs_filter_on { "ON" } else { "off" }
            ),
            Style::default().fg(if self.prefs_filter_on {
                COLOR_MINT
            } else {
                COLOR_DIM
            }),
        ));
        lines.push(Line::styled(
            format!(
                "  min comp: {} | type: {:?} | mode: {:?}",
                self.cfg
                    .preferences
                    .min_comp_usd
                    .map(|v| format!("${v}"))
                    .unwrap_or_else(|| "any".to_string()),
                self.cfg.preferences.employment_type,
                self.cfg.preferences.work_mode,
            ),
            Style::default().fg(COLOR_DIM),
        ));

        lines.push(Line::from(""));
        lines.push(Line::styled(
            "Inferred Roles ([r] to infer):",
            Style::default().fg(COLOR_CYAN),
        ));

        if self.inferring_roles {
            lines.push(Line::from(format!("  {} inferring...", self.spinner())));
        } else if self.roles.is_empty() {
            lines.push(Line::from("  (none yet - press 'r')"));
        } else {
            for (i, r) in self.roles.iter().take(8).enumerate() {
                let flag = if r.compensation_band.source.contains("clamped") {
                    " *"
                } else {
                    ""
                };
                lines.push(Line::from(format!(
                    "  {}. {} - {:?} ({} {}k median{})",
                    i + 1,
                    r.title,
                    r.market_demand.level,
                    r.compensation_band.currency,
                    r.compensation_band.median / 1000,
                    flag,
                )));
            }
            if self
                .roles
                .iter()
                .any(|r| r.compensation_band.source.contains("clamped"))
            {
                lines.push(Line::styled(
                    "  * estimate corrected - model's raw figure was implausible",
                    Style::default().fg(COLOR_DIM),
                ));
            }
        }

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    fn render_center(&mut self, f: &mut ratatui::Frame, area: Rect) {
        let block = Block::default()
            .title(if self.prefs_filter_on {
                format!(
                    " Job Scan & Evaluation - prefs filter ON ({}/{}) ",
                    self.visible_indices.len(),
                    self.jobs.len()
                )
            } else {
                " Job Scan & Evaluation ".to_string()
            })
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER_ACTIVE));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(6), Constraint::Length(8)])
            .split(inner);

        let header = Row::new(["ID", "Title", "Company", "Location", "Match", "Status"]).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

        let visible = self.visible_jobs();
        let rows: Vec<Row> = if visible.is_empty() {
            let hint = if self.jobs.is_empty() {
                "Press [s] to scan"
            } else {
                "No jobs match your preferences - [p] to toggle filter off"
            };
            vec![Row::new(vec![
                Cell::new(""),
                Cell::new(if self.jobs.is_empty() {
                    "No jobs yet."
                } else {
                    "0 jobs shown."
                }),
                Cell::new(hint),
                Cell::new("-"),
                Cell::new("-"),
                Cell::new("-"),
            ])]
        } else {
            visible
                .iter()
                .map(|j| {
                    let short_id: String = j.id.chars().take(8).collect();
                    let score_cell = match j.overall_score {
                        Some(s) => Cell::new(format!("{:.0}%", s * 100.0))
                            .style(Style::default().fg(score_color(s))),
                        None => Cell::new("-"),
                    };
                    let status = j
                        .status
                        .as_ref()
                        .map(|s| format!("{s:?}"))
                        .unwrap_or_else(|| "-".to_string());
                    Row::new(vec![
                        Cell::new(short_id),
                        Cell::new(j.title.clone()),
                        Cell::new(j.company.clone()),
                        Cell::new(j.location.clone()),
                        score_cell,
                        Cell::new(status),
                    ])
                })
                .collect()
        };

        let table = Table::new(
            rows,
            [
                Constraint::Length(9),
                Constraint::Percentage(26),
                Constraint::Percentage(20),
                Constraint::Percentage(16),
                Constraint::Length(7),
                Constraint::Percentage(15),
            ],
        )
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(COLOR_GOLD)
                .add_modifier(Modifier::BOLD),
        );

        f.render_stateful_widget(table, chunks[0], &mut self.job_state);

        self.render_detail_panel(f, chunks[1]);
    }

    fn render_detail_panel(&self, f: &mut ratatui::Frame, area: Rect) {
        let selected = self.job_state.selected().and_then(|i| self.jobs.get(i));

        if self.evaluating {
            let text = format!("{} Evaluating...", self.spinner());
            f.render_widget(
                Paragraph::new(text).style(Style::default().fg(COLOR_GOLD)),
                area,
            );
            return;
        }

        let Some(eval) = &self.selected_eval else {
            let hint = if selected.is_some() {
                "Not yet evaluated - press [e] to evaluate this job against your profile."
            } else {
                ""
            };
            f.render_widget(
                Paragraph::new(hint).style(Style::default().fg(COLOR_DIM)),
                area,
            );
            return;
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(area);

        let pct = (eval.overall_score.clamp(0.0, 1.0) * 100.0) as u16;
        let gauge = Gauge::default()
            .block(Block::default().title(format!(
                "Match Score: {:.0}% ({})  [e]=re-evaluate  [t]=tailor",
                eval.overall_score * 100.0,
                eval.overall_grade
            )))
            .gauge_style(
                Style::default()
                    .fg(score_color(eval.overall_score))
                    .bg(Color::Rgb(25, 40, 45)),
            )
            .percent(pct);
        f.render_widget(gauge, chunks[0]);

        let mut lines: Vec<Line> = Vec::new();
        for dim in eval.dimensions.iter().take(4) {
            let frac = if dim.max > 0.0 {
                dim.score / dim.max
            } else {
                0.0
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{:<18}", dim.name), Style::default().fg(COLOR_DIM)),
                Span::styled(bar(frac, 10), Style::default().fg(score_color(frac))),
                Span::raw(format!(" {:.1}/{:.1}", dim.score, dim.max)),
            ]));
        }
        if !eval.strengths.is_empty() {
            lines.push(Line::styled(
                format!("+ {}", eval.strengths.join("; ")),
                Style::default().fg(COLOR_MINT),
            ));
        }
        if !eval.gaps.is_empty() {
            lines.push(Line::styled(
                format!("- {}", eval.gaps.join("; ")),
                Style::default().fg(COLOR_AMBER),
            ));
        }
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), chunks[1]);
    }

    fn render_right(&self, f: &mut ratatui::Frame, area: Rect) {
        let block = Block::default()
            .title(" Pipeline Summary & HW ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_PANEL_BORDER));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(4),
                Constraint::Length(2),
            ])
            .split(inner);

        self.render_pipeline_summary(f, chunks[0]);
        self.render_activity(f, chunks[1]);
        self.render_hw_bar(f, chunks[2]);
    }

    fn render_pipeline_summary(&self, f: &mut ratatui::Frame, area: Rect) {
        let count_of = |status: PipelineStatus| -> usize {
            self.pipeline_counts
                .iter()
                .find(|(s, _)| *s == status)
                .map(|(_, c)| *c)
                .unwrap_or(0)
        };
        let tracked: usize = self.pipeline_counts.iter().map(|(_, c)| c).sum();
        let interviewing = count_of(PipelineStatus::Interviewing);
        let offered = count_of(PipelineStatus::Offered);
        let rejected = count_of(PipelineStatus::Rejected);

        let lines = vec![
            Line::from(format!("Tracked in pipeline: {tracked}")),
            Line::styled(
                format!("Interviewing: {interviewing}"),
                Style::default().fg(COLOR_CYAN),
            ),
            Line::styled(
                format!("Offers: {offered}"),
                Style::default().fg(COLOR_MINT),
            ),
            Line::styled(
                format!("Rejected: {rejected}"),
                Style::default().fg(COLOR_RED),
            ),
        ];
        f.render_widget(Paragraph::new(lines), area);
    }

    fn render_activity(&self, f: &mut ratatui::Frame, area: Rect) {
        let block = Block::default()
            .title("Activity")
            .borders(Borders::TOP)
            .border_style(Style::default().fg(COLOR_PANEL_BORDER));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let items: Vec<ListItem> = if self.activity.is_empty() {
            vec![ListItem::new("No activity yet.").style(Style::default().fg(COLOR_DIM))]
        } else {
            self.activity
                .iter()
                .map(|e| {
                    ListItem::new(format!(
                        "{}\n  {}",
                        e.timestamp.format("%Y-%m-%d %H:%M"),
                        e.description
                    ))
                    .style(Style::default().fg(COLOR_DIM))
                })
                .collect()
        };
        f.render_widget(List::new(items), inner);

        if let Some(status) = &self.status {
            let status_area = Rect {
                y: inner.y + inner.height.saturating_sub(2),
                height: 2.min(inner.height),
                ..inner
            };
            f.render_widget(
                Paragraph::new(status.as_str())
                    .style(Style::default().fg(COLOR_GOLD))
                    .wrap(Wrap { trim: true }),
                status_area,
            );
        }
    }

    fn render_hw_bar(&self, f: &mut ratatui::Frame, area: Rect) {
        let cpu_pct = self.sys.global_cpu_usage();
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title("CPU")
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(COLOR_PANEL_BORDER)),
            )
            .gauge_style(
                Style::default()
                    .fg(if cpu_pct > 85.0 {
                        COLOR_RED
                    } else {
                        COLOR_CYAN
                    })
                    .bg(Color::Rgb(25, 40, 45)),
            )
            .percent(cpu_pct.clamp(0.0, 100.0) as u16);
        f.render_widget(gauge, area);
    }

    fn render_footer(&self, f: &mut ratatui::Frame, area: Rect) {
        let footer = Paragraph::new("q=quit  j/k=nav  g=region  p=prefs-filter  r=infer roles  s=scan  e=evaluate  t=tailor  R=refresh")
            .style(Style::default().fg(COLOR_DIM))
            .alignment(Alignment::Center);
        f.render_widget(footer, area);
    }

    fn render_scan_modal(&self, f: &mut ratatui::Frame, area: Rect) {
        let popup = centered_rect(92, 80, area);
        f.render_widget(Clear, popup);

        let block = Block::default()
            .title(format!(" {} Scanning ", self.spinner()))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(COLOR_MINT))
            .style(Style::default().bg(Color::Rgb(20, 30, 38)));
        let inner = block.inner(popup);
        f.render_widget(block, popup);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(3)])
            .split(inner);

        let progress = if self.scan_boards_total > 0 {
            (self.scan_boards_done as f64 / self.scan_boards_total as f64 * 100.0) as u16
        } else {
            0
        };
        let gauge = Gauge::default()
            .block(Block::default().title(format!(
                "{}/{} boards - {} match(es) found so far",
                self.scan_boards_done, self.scan_boards_total, self.scan_found_total
            )))
            .gauge_style(Style::default().fg(COLOR_MINT).bg(Color::Rgb(25, 40, 45)))
            .percent(progress.min(100));
        f.render_widget(gauge, chunks[0]);

        let lines: Vec<Line> = self
            .scan_log
            .iter()
            .rev()
            .take(chunks[1].height as usize)
            .rev()
            .map(|l| Line::from(l.clone()))
            .collect();
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), chunks[1]);
    }

    fn next_row(&mut self) {
        if self.visible_indices.is_empty() {
            return;
        }
        let i = match self.job_state.selected() {
            Some(i) if i + 1 < self.visible_indices.len() => i + 1,
            Some(i) => i,
            None => 0,
        };
        self.job_state.select(Some(i));
        self.load_selected_eval();
    }

    fn prev_row(&mut self) {
        if self.visible_indices.is_empty() {
            return;
        }
        let i = match self.job_state.selected() {
            Some(i) if i > 0 => i - 1,
            Some(i) => i,
            None => 0,
        };
        self.job_state.select(Some(i));
        self.load_selected_eval();
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
