use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::sync::Mutex;
use tracing::debug;

use crate::models::job::{
    ActivityEvent, DimensionScore, Evaluation, Job, JobRow, PipelineEntry, PipelineStatus,
    Recommendation,
};
use crate::models::profile::UserProfile;
use crate::models::role::{
    CompensationBand, DemandLevel, MarketDemand, RoleArchetype, Seniority, TrendDirection,
};

pub struct PipelineTracker {
    conn: Mutex<Connection>,
}

impl PipelineTracker {
    pub fn new(db_path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(db_path).context("Failed to open SQLite database")?;
        let tracker = Self {
            conn: Mutex::new(conn),
        };
        tracker.init_schema()?;
        Ok(tracker)
    }

    pub fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT,
                phone TEXT,
                location TEXT,
                linkedin_url TEXT,
                portfolio_url TEXT,
                summary TEXT,
                raw_text TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS roles (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                industry TEXT NOT NULL,
                seniority TEXT NOT NULL,
                fit_score REAL,
                market_demand TEXT,
                compensation_currency TEXT,
                compensation_min INTEGER,
                compensation_max INTEGER,
                compensation_median INTEGER,
                compensation_source TEXT,
                typical_requirements TEXT,
                top_companies TEXT,
                inferred_from_profile INTEGER,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                company TEXT NOT NULL,
                location TEXT NOT NULL,
                remote INTEGER,
                job_type TEXT,
                salary_range TEXT,
                description TEXT NOT NULL,
                requirements TEXT,
                posted_at TEXT,
                source TEXT NOT NULL,
                url TEXT NOT NULL,
                applied INTEGER DEFAULT 0,
                scraped_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS evaluations (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL,
                overall_score REAL NOT NULL,
                overall_grade TEXT NOT NULL,
                dimensions TEXT,
                match_summary TEXT,
                strengths TEXT,
                gaps TEXT,
                red_flags TEXT,
                recommendation TEXT NOT NULL,
                model_used TEXT NOT NULL,
                evaluated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pipeline (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL,
                status TEXT NOT NULL,
                notes TEXT,
                contact TEXT,
                follow_up_date TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS applications (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL,
                resume_text TEXT NOT NULL,
                cover_letter_text TEXT NOT NULL,
                model_used TEXT NOT NULL,
                generated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_applications_job_id ON applications(job_id);
            CREATE TABLE IF NOT EXISTS market_data (
                id TEXT PRIMARY KEY,
                role_id TEXT NOT NULL,
                posting_volume_30d INTEGER,
                trend TEXT,
                last_updated TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_pipeline_job_id ON pipeline(job_id);
            CREATE INDEX IF NOT EXISTS idx_evaluations_job_id ON evaluations(job_id);
            CREATE INDEX IF NOT EXISTS idx_jobs_source ON jobs(source);
        ",
        )?;
        // debug, not info: this fires on every PipelineTracker::new() call,
        // including from background tasks (e.g. the TUI's async scan) where
        // an info-level println would corrupt the alternate-screen render.
        debug!("SQLite schema initialized");
        Ok(())
    }

    pub fn save_profile(&self, profile: &UserProfile) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO profiles (id, name, email, phone, location, linkedin_url, portfolio_url, summary, raw_text, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                uuid::Uuid::new_v4().to_string(),
                profile.name,
                profile.email,
                profile.phone,
                profile.location,
                profile.linkedin_url,
                profile.portfolio_url,
                profile.summary,
                profile.raw_text,
                profile.created_at.to_rfc3339(),
                profile.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn save_job(&self, job: &Job) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO jobs (id, title, company, location, remote, job_type, salary_range, description, requirements, posted_at, source, url, applied, scraped_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                job.id,
                job.title,
                job.company,
                job.location,
                job.remote,
                job.job_type,
                job.salary_range,
                job.description,
                serde_json::to_string(&job.requirements).unwrap_or_default(),
                job.posted_at.map(|d| d.to_rfc3339()),
                job.source,
                job.url,
                job.applied,
                job.scraped_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_job(&self, job_id: &str) -> Result<Option<Job>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, company, location, remote, job_type, salary_range, description, requirements, posted_at, source, url, applied, scraped_at FROM jobs WHERE id = ?1",
        )?;
        let mut rows = stmt.query([job_id])?;
        if let Some(row) = rows.next()? {
            let requirements_str: String = row.get(8)?;
            let requirements: Vec<String> =
                serde_json::from_str(&requirements_str).unwrap_or_default();
            let posted_at = row.get::<_, String>(9).ok().and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });
            Ok(Some(Job {
                id: row.get(0)?,
                title: row.get(1)?,
                company: row.get(2)?,
                location: row.get(3)?,
                remote: row.get(4)?,
                job_type: row.get(5)?,
                salary_range: row.get(6)?,
                description: row.get(7)?,
                requirements,
                posted_at,
                source: row.get(10)?,
                url: row.get(11)?,
                applied: row.get(12)?,
                scraped_at: row
                    .get::<_, String>(13)
                    .unwrap_or_else(|_| Utc::now().to_rfc3339())
                    .parse::<DateTime<Utc>>()
                    .unwrap_or_else(|_| Utc::now()),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_pipeline_status_by_job_id(
        &self,
        job_id: &str,
        status: PipelineStatus,
    ) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let rows_affected = conn.execute(
            "UPDATE pipeline SET status = ?1, updated_at = ?2 WHERE job_id = ?3",
            params![
                serde_json::to_string(&status).unwrap_or_default(),
                Utc::now().to_rfc3339(),
                job_id
            ],
        )?;
        Ok(rows_affected)
    }

    /// Partial update of a pipeline entry's tracking fields - any argument
    /// left `None` keeps its current DB value (via SQL COALESCE), so callers
    /// only need to pass what actually changed. These columns
    /// (notes/contact/follow_up_date) existed in the schema from day one but
    /// were never settable from any CLI or TUI surface.
    pub fn update_pipeline_fields(
        &self,
        job_id: &str,
        status: Option<PipelineStatus>,
        notes: Option<&str>,
        contact: Option<&str>,
        follow_up_date: Option<DateTime<Utc>>,
    ) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let status_json = status.map(|s| serde_json::to_string(&s).unwrap_or_default());
        let follow_up_str = follow_up_date.map(|d| d.to_rfc3339());
        let rows_affected = conn.execute(
            "UPDATE pipeline SET
                status = COALESCE(?1, status),
                notes = COALESCE(?2, notes),
                contact = COALESCE(?3, contact),
                follow_up_date = COALESCE(?4, follow_up_date),
                updated_at = ?5
             WHERE job_id = ?6",
            params![
                status_json,
                notes,
                contact,
                follow_up_str,
                Utc::now().to_rfc3339(),
                job_id
            ],
        )?;
        Ok(rows_affected)
    }

    pub fn save_evaluation(&self, eval: &Evaluation) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO evaluations (id, job_id, overall_score, overall_grade, dimensions, match_summary, strengths, gaps, red_flags, recommendation, model_used, evaluated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                eval.id,
                eval.job_id,
                eval.overall_score,
                eval.overall_grade,
                serde_json::to_string(&eval.dimensions).unwrap_or_default(),
                eval.match_summary,
                serde_json::to_string(&eval.strengths).unwrap_or_default(),
                serde_json::to_string(&eval.gaps).unwrap_or_default(),
                serde_json::to_string(&eval.red_flags).unwrap_or_default(),
                serde_json::to_string(&eval.recommendation).unwrap_or_default(),
                eval.model_used,
                eval.evaluated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Records a generated resume/cover letter pair against a job. Called
    /// automatically by `tailor` - every tailoring run is kept, not just the
    /// latest, so a resubmission after tweaking the profile doesn't erase
    /// the record of what was actually sent the first time.
    pub fn record_application(
        &self,
        job_id: &str,
        resume_text: &str,
        cover_letter_text: &str,
        model_used: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO applications (id, job_id, resume_text, cover_letter_text, model_used, generated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                uuid::Uuid::new_v4().to_string(),
                job_id,
                resume_text,
                cover_letter_text,
                model_used,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// The most recent tailored resume/cover letter generated for a job, if
    /// any - what you'd want to see again ahead of an interview.
    pub fn get_latest_application(
        &self,
        job_id: &str,
    ) -> Result<Option<crate::models::job::Application>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, job_id, resume_text, cover_letter_text, model_used, generated_at
             FROM applications WHERE job_id = ?1 ORDER BY generated_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query([job_id])?;
        if let Some(row) = rows.next()? {
            let generated_at = row
                .get::<_, String>(5)?
                .parse::<DateTime<Utc>>()
                .unwrap_or_else(|_| Utc::now());
            Ok(Some(crate::models::job::Application {
                id: row.get(0)?,
                job_id: row.get(1)?,
                resume_text: row.get(2)?,
                cover_letter_text: row.get(3)?,
                model_used: row.get(4)?,
                generated_at,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn add_pipeline_entry(
        &self,
        job_id: &str,
        status: PipelineStatus,
    ) -> Result<PipelineEntry> {
        let now = Utc::now();
        let entry = PipelineEntry {
            id: uuid::Uuid::new_v4().to_string(),
            job_id: job_id.to_string(),
            status,
            notes: None,
            contact: None,
            follow_up_date: None,
            created_at: now,
            updated_at: now,
        };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO pipeline (id, job_id, status, notes, contact, follow_up_date, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.id,
                entry.job_id,
                serde_json::to_string(&entry.status).unwrap_or_default(),
                entry.notes,
                entry.contact,
                entry.follow_up_date.map(|d| d.to_rfc3339()),
                entry.created_at.to_rfc3339(),
                entry.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(entry)
    }

    pub fn update_pipeline_status(&self, entry_id: &str, status: PipelineStatus) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE pipeline SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                serde_json::to_string(&status).unwrap_or_default(),
                Utc::now().to_rfc3339(),
                entry_id
            ],
        )?;
        Ok(())
    }

    pub fn list_pipeline(&self) -> Result<Vec<PipelineEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, job_id, status, notes, contact, follow_up_date, created_at, updated_at FROM pipeline ORDER BY updated_at DESC")?;
        let mut rows = stmt.query([])?;

        let mut entries = Vec::new();
        while let Some(row) = rows.next()? {
            let status_str: String = row.get(2)?;
            let status = serde_json::from_str(&status_str).unwrap_or(PipelineStatus::New);
            entries.push(PipelineEntry {
                id: row.get(0)?,
                job_id: row.get(1)?,
                status,
                notes: row.get(3)?,
                contact: row.get(4)?,
                follow_up_date: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            });
        }
        Ok(entries)
    }

    /// Jobs joined with their latest evaluation and latest pipeline status.
    /// Never fabricates a score or status for a job that doesn't have one -
    /// both columns are `None` until real data exists.
    pub fn list_job_rows(&self, limit: usize) -> Result<Vec<JobRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT j.id, j.title, j.company, j.location, j.url, j.description, j.salary_range, j.remote,
                    e.overall_score, e.overall_grade, p.status, j.posted_at, j.scraped_at
             FROM jobs j
             LEFT JOIN evaluations e ON e.id = (
                 SELECT id FROM evaluations WHERE job_id = j.id ORDER BY evaluated_at DESC LIMIT 1
             )
             LEFT JOIN pipeline p ON p.id = (
                 SELECT id FROM pipeline WHERE job_id = j.id ORDER BY updated_at DESC LIMIT 1
             )
             ORDER BY j.scraped_at DESC
             LIMIT ?1",
        )?;
        let mut rows = stmt.query(params![limit as i64])?;

        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let status_str: Option<String> = row.get(10)?;
            let status = status_str.and_then(|s| serde_json::from_str(&s).ok());
            let posted_at = row.get::<_, Option<String>>(11)?.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });
            let scraped_at = row
                .get::<_, String>(12)
                .ok()
                .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                .unwrap_or_else(Utc::now);
            out.push(JobRow {
                id: row.get(0)?,
                title: row.get(1)?,
                company: row.get(2)?,
                location: row.get(3)?,
                url: row.get(4)?,
                description: row.get(5)?,
                salary_range: row.get(6)?,
                remote: row.get(7)?,
                overall_score: row.get(8)?,
                overall_grade: row.get(9)?,
                status,
                posted_at,
                scraped_at,
            });
        }
        Ok(out)
    }

    /// Merges evaluation and pipeline-status-change timestamps into one
    /// real, sorted activity feed. Nothing here is invented - an empty
    /// database produces an empty feed.
    pub fn recent_activity(&self, limit: usize) -> Result<Vec<ActivityEvent>> {
        let conn = self.conn.lock().unwrap();
        let mut events = Vec::new();

        let mut stmt = conn.prepare(
            "SELECT e.evaluated_at, e.overall_score, e.overall_grade, j.title
             FROM evaluations e
             LEFT JOIN jobs j ON j.id = e.job_id
             ORDER BY e.evaluated_at DESC LIMIT ?1",
        )?;
        let mut rows = stmt.query(params![limit as i64])?;
        while let Some(row) = rows.next()? {
            let ts_str: String = row.get(0)?;
            let Some(ts) = DateTime::parse_from_rfc3339(&ts_str)
                .ok()
                .map(|d| d.with_timezone(&Utc))
            else {
                continue;
            };
            let score: f64 = row.get(1)?;
            let grade: String = row.get(2)?;
            let title: Option<String> = row.get(3)?;
            events.push(ActivityEvent {
                timestamp: ts,
                description: format!(
                    "Evaluated {} - {} ({})",
                    title.as_deref().unwrap_or("job"),
                    score,
                    grade
                ),
            });
        }

        let mut stmt = conn.prepare(
            "SELECT p.updated_at, p.status, j.title
             FROM pipeline p
             LEFT JOIN jobs j ON j.id = p.job_id
             ORDER BY p.updated_at DESC LIMIT ?1",
        )?;
        let mut rows = stmt.query(params![limit as i64])?;
        while let Some(row) = rows.next()? {
            let ts_str: String = row.get(0)?;
            let Some(ts) = DateTime::parse_from_rfc3339(&ts_str)
                .ok()
                .map(|d| d.with_timezone(&Utc))
            else {
                continue;
            };
            let status_str: String = row.get(1)?;
            let status: PipelineStatus =
                serde_json::from_str(&status_str).unwrap_or(PipelineStatus::New);
            let title: Option<String> = row.get(2)?;
            events.push(ActivityEvent {
                timestamp: ts,
                description: format!(
                    "Pipeline: {} -> {:?}",
                    title.as_deref().unwrap_or("job"),
                    status
                ),
            });
        }

        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        events.truncate(limit);
        Ok(events)
    }

    /// Full evaluation (dimensions, strengths, gaps, red flags) for a job's
    /// most recent evaluation, if one exists - backs the TUI's detail panel.
    pub fn get_latest_evaluation(&self, job_id: &str) -> Result<Option<Evaluation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, job_id, overall_score, overall_grade, dimensions, match_summary, strengths, gaps, red_flags, recommendation, model_used, evaluated_at
             FROM evaluations WHERE job_id = ?1 ORDER BY evaluated_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query(params![job_id])?;
        if let Some(row) = rows.next()? {
            let dimensions: Vec<DimensionScore> = row
                .get::<_, String>(4)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let strengths: Vec<String> = row
                .get::<_, String>(6)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let gaps: Vec<String> = row
                .get::<_, String>(7)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let red_flags: Vec<String> = row
                .get::<_, String>(8)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let recommendation: Recommendation = row
                .get::<_, String>(9)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(Recommendation::Maybe);
            let evaluated_at = row
                .get::<_, String>(11)
                .ok()
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            Ok(Some(Evaluation {
                id: row.get(0)?,
                job_id: row.get(1)?,
                overall_score: row.get(2)?,
                overall_grade: row.get(3)?,
                dimensions,
                match_summary: row.get(5)?,
                strengths,
                gaps,
                red_flags,
                recommendation,
                model_used: row.get(10)?,
                evaluated_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Real counts of pipeline entries by status - backs the TUI's pipeline
    /// summary panel. Empty vec if the pipeline is empty; never invented.
    pub fn pipeline_status_counts(&self) -> Result<Vec<(PipelineStatus, usize)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT status, COUNT(*) FROM pipeline GROUP BY status")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let status_str: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            let status: PipelineStatus =
                serde_json::from_str(&status_str).unwrap_or(PipelineStatus::New);
            out.push((status, count as usize));
        }
        Ok(out)
    }

    /// Previously-inferred roles, newest first. Without this, role inference
    /// was entirely ephemeral - `save_role` was written but never called
    /// from anywhere, so every session paid for a fresh LLM call just to
    /// re-show roles the user had already seen.
    pub fn list_roles(&self, limit: usize) -> Result<Vec<RoleArchetype>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, industry, seniority, fit_score, market_demand, compensation_currency, compensation_min, compensation_max, compensation_median, compensation_source, typical_requirements, top_companies, inferred_from_profile, created_at
             FROM roles ORDER BY created_at DESC LIMIT ?1",
        )?;
        let mut rows = stmt.query(params![limit as i64])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let seniority: Seniority = row
                .get::<_, String>(3)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(Seniority::Mid);
            let market_demand: MarketDemand = row
                .get::<_, String>(5)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| MarketDemand {
                    level: DemandLevel::Medium,
                    posting_volume_30d: 0,
                    trend: TrendDirection::Stable,
                    last_updated: Utc::now(),
                });
            let typical_requirements: Vec<String> = row
                .get::<_, String>(11)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let top_companies: Vec<String> = row
                .get::<_, String>(12)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let created_at = row
                .get::<_, String>(14)
                .ok()
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            out.push(RoleArchetype {
                id: row.get(0)?,
                title: row.get(1)?,
                industry: row.get(2)?,
                seniority,
                fit_score: row.get(4)?,
                market_demand,
                compensation_band: CompensationBand {
                    currency: row.get(6)?,
                    min: row.get::<_, i64>(7)? as u64,
                    max: row.get::<_, i64>(8)? as u64,
                    median: row.get::<_, i64>(9)? as u64,
                    source: row.get(10)?,
                },
                typical_requirements,
                top_companies,
                inferred_from_profile: row.get(13)?,
                created_at,
            });
        }
        // The query above is ORDER BY created_at DESC so LIMIT grabs the
        // most recent batch - but that leaves the batch itself in reverse
        // order. fit_score is always 0.0 (never populated by the inference
        // engine - see role_inference.rs), so the only real rank signal is
        // insertion order within the batch (each role gets its own
        // created_at, monotonically increasing). Reversing restores it.
        out.reverse();
        Ok(out)
    }

    pub fn save_role(&self, role: &RoleArchetype) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO roles (id, title, industry, seniority, fit_score, market_demand, compensation_currency, compensation_min, compensation_max, compensation_median, compensation_source, typical_requirements, top_companies, inferred_from_profile, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                role.id,
                role.title,
                role.industry,
                serde_json::to_string(&role.seniority).unwrap_or_default(),
                role.fit_score,
                serde_json::to_string(&role.market_demand).unwrap_or_default(),
                role.compensation_band.currency,
                role.compensation_band.min,
                role.compensation_band.max,
                role.compensation_band.median,
                role.compensation_band.source,
                serde_json::to_string(&role.typical_requirements).unwrap_or_default(),
                serde_json::to_string(&role.top_companies).unwrap_or_default(),
                role.inferred_from_profile,
                role.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }
}
