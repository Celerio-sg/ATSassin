use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Transaction, TransactionBehavior, MAIN_DB};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use tracing::{debug, warn};

use crate::engine::compute_broker::{ComputeBroker, ProviderQuota};
use crate::models::job::{
    ActivityEvent, DimensionScore, Evaluation, Job, JobRow, PipelineEntry, PipelineStatus,
    Recommendation,
};
use crate::models::profile::UserProfile;
use crate::models::role::{
    CompensationBand, DemandLevel, MarketDemand, RoleArchetype, Seniority, TrendDirection,
};
use crate::pipeline::board_discovery::{AtsType, DiscoveredBoard};

pub const CURRENT_SCHEMA_VERSION: u32 = 2;

const INITIAL_SCHEMA_SQL: &str = r#"
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
    CREATE TABLE IF NOT EXISTS market_data (
        id TEXT PRIMARY KEY,
        role_id TEXT NOT NULL,
        posting_volume_30d INTEGER,
        trend TEXT,
        last_updated TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS company_boards (
        company TEXT PRIMARY KEY,
        ats_type TEXT NOT NULL,
        slug TEXT NOT NULL,
        source_url TEXT NOT NULL,
        discovered_at TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS provider_quota (
        provider TEXT PRIMARY KEY,
        tier_type TEXT NOT NULL,
        remaining_requests INTEGER,
        remaining_tokens INTEGER,
        resets_at TEXT,
        reliability_score REAL,
        last_observed TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_applications_job_id ON applications(job_id);
    CREATE INDEX IF NOT EXISTS idx_pipeline_job_id ON pipeline(job_id);
    CREATE INDEX IF NOT EXISTS idx_evaluations_job_id ON evaluations(job_id);
    CREATE INDEX IF NOT EXISTS idx_jobs_source ON jobs(source);
"#;

const FEEDBACK_SCHEMA_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS feedback (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        job_id TEXT NOT NULL,
        task_type TEXT NOT NULL,
        action TEXT NOT NULL,
        recommendation_text TEXT NOT NULL,
        edited_text TEXT,
        edit_distance INTEGER,
        confidence_before REAL NOT NULL,
        confidence_after REAL NOT NULL,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX IF NOT EXISTS idx_feedback_task ON feedback(task_type);
    CREATE INDEX IF NOT EXISTS idx_feedback_job ON feedback(job_id);
    CREATE INDEX IF NOT EXISTS idx_feedback_created ON feedback(created_at);
"#;

type MigrationFn = for<'connection> fn(&Transaction<'connection>) -> rusqlite::Result<()>;

#[derive(Clone, Copy)]
struct Migration {
    version: u32,
    name: &'static str,
    apply: MigrationFn,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_pipeline_schema",
        apply: migration_1_initial_pipeline_schema,
    },
    Migration {
        version: 2,
        name: "feedback_schema",
        apply: migration_2_feedback_schema,
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationOutcome {
    Current {
        version: u32,
    },
    Created {
        version: u32,
    },
    Migrated {
        from: u32,
        to: u32,
        backup_path: PathBuf,
    },
}

fn migration_1_initial_pipeline_schema(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute_batch(INITIAL_SCHEMA_SQL)
}

fn migration_2_feedback_schema(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute_batch(FEEDBACK_SCHEMA_SQL)
}

fn migrate_database(
    conn: &mut Connection,
    db_path: &Path,
    migrations: &[Migration],
) -> Result<MigrationOutcome> {
    migrate_database_to(conn, db_path, migrations, CURRENT_SCHEMA_VERSION)
}

fn migrate_database_to(
    conn: &mut Connection,
    db_path: &Path,
    migrations: &[Migration],
    target_version: u32,
) -> Result<MigrationOutcome> {
    migrate_database_to_with_lock_hook(conn, db_path, migrations, target_version, || Ok(()))
}

fn migrate_database_to_with_lock_hook<F>(
    conn: &mut Connection,
    db_path: &Path,
    migrations: &[Migration],
    target_version: u32,
    after_lock: F,
) -> Result<MigrationOutcome>
where
    F: FnOnce() -> Result<()>,
{
    validate_migration_sequence(migrations, target_version)?;
    let observed_from = read_user_version(conn)?;
    if observed_from > target_version {
        anyhow::bail!(
            "Database schema version {observed_from} is newer than this binary supports ({target_version}); refusing to open {} to prevent corruption",
            db_path.display()
        );
    }
    if observed_from == target_version {
        return Ok(MigrationOutcome::Current {
            version: observed_from,
        });
    }

    let transaction = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("Failed to start SQLite schema migration transaction")?;
    let from = read_user_version(&transaction)?;
    if from > target_version {
        anyhow::bail!(
            "Database schema version {from} became newer than this binary supports ({target_version}) while waiting for the migration lock; refusing to modify {}",
            db_path.display()
        );
    }
    if from == target_version {
        drop(transaction);
        return Ok(MigrationOutcome::Current { version: from });
    }

    after_lock()?;
    let backup_path = if database_has_pre_migration_state(&transaction, from)? {
        let path = create_pre_migration_backup(db_path, from, target_version)?;
        warn!(
            "SQLite schema migration backup created at {}",
            path.display()
        );
        Some(path)
    } else {
        None
    };

    for migration in migrations
        .iter()
        .filter(|migration| migration.version > from)
    {
        (migration.apply)(&transaction).with_context(|| {
            format!(
                "SQLite migration {} ({}) failed; all migration changes were rolled back",
                migration.version, migration.name
            )
        })?;
        transaction
            .pragma_update(None, "user_version", i64::from(migration.version))
            .with_context(|| {
                format!(
                    "Failed to record SQLite schema version {} after migration {}",
                    migration.version, migration.name
                )
            })?;
    }
    transaction.commit().context(
        "Failed to commit SQLite schema migrations; all migration changes were rolled back",
    )?;

    match backup_path {
        Some(backup_path) => Ok(MigrationOutcome::Migrated {
            from,
            to: target_version,
            backup_path,
        }),
        None => Ok(MigrationOutcome::Created {
            version: target_version,
        }),
    }
}

fn database_has_pre_migration_state(transaction: &Transaction<'_>, version: u32) -> Result<bool> {
    if version > 0 {
        return Ok(true);
    }
    let user_schema_objects: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )
        .context("Failed to determine whether the locked database needs a migration backup")?;
    Ok(user_schema_objects > 0)
}

fn validate_migration_sequence(migrations: &[Migration], target_version: u32) -> Result<()> {
    if migrations.len() != target_version as usize {
        anyhow::bail!(
            "Invalid SQLite migration registry: {} migrations declared for schema version {}",
            migrations.len(),
            target_version
        );
    }
    for (index, migration) in migrations.iter().enumerate() {
        let expected = index as u32 + 1;
        if migration.version != expected {
            anyhow::bail!(
                "Invalid SQLite migration registry: expected version {expected}, found {} ({})",
                migration.version,
                migration.name
            );
        }
    }
    Ok(())
}

fn read_user_version(conn: &Connection) -> Result<u32> {
    let raw: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .context("Failed to read SQLite PRAGMA user_version")?;
    u32::try_from(raw).context("SQLite PRAGMA user_version was negative or out of range")
}

fn create_pre_migration_backup(db_path: &Path, from: u32, to: u32) -> Result<PathBuf> {
    let file_name = db_path.file_name().and_then(|name| name.to_str()).context(
        "Cannot create a migration backup for a database path without a UTF-8 file name",
    )?;
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let unique = uuid::Uuid::new_v4().simple();
    let backup_path = db_path.with_file_name(format!(
        "{file_name}.backup-v{from}-to-v{to}-{timestamp}-{unique}.sqlite3"
    ));
    let source_permissions = std::fs::metadata(db_path)
        .with_context(|| {
            format!(
                "Refusing to migrate because source database permissions could not be read from {}",
                db_path.display()
            )
        })?
        .permissions();
    let placeholder = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup_path)
        .with_context(|| {
            format!(
                "Refusing to migrate because the backup could not be created securely at {}",
                backup_path.display()
            )
        })?;
    std::fs::set_permissions(&backup_path, source_permissions.clone()).with_context(|| {
        format!(
            "Refusing to migrate because source permissions could not be applied to {}",
            backup_path.display()
        )
    })?;
    drop(placeholder);

    let source = Connection::open(db_path).with_context(|| {
        format!(
            "Refusing to migrate because the database could not be reopened for a consistent backup at {}",
            db_path.display()
        )
    })?;
    if let Err(error) = source.backup(MAIN_DB, &backup_path, None) {
        let _ = std::fs::remove_file(&backup_path);
        return Err(error).with_context(|| {
            format!(
                "Refusing to migrate because the pre-migration backup could not be created at {}",
                backup_path.display()
            )
        });
    }
    std::fs::set_permissions(&backup_path, source_permissions).with_context(|| {
        format!(
            "Refusing to migrate because source permissions could not be restored on {}",
            backup_path.display()
        )
    })?;
    Ok(backup_path)
}

pub struct PipelineTracker {
    conn: Mutex<Connection>,
    // Kept around so the calibration hook in `log_status_to_feedback`
    // (issue #10) can hand the same db path to `FeedbackTracker::new`
    // without re-deriving it from env. Issue #10 didn't store this
    // before, which is what caused the build to fail.
    db_path: std::path::PathBuf,
    migration_outcome: MigrationOutcome,
}

struct GuardedConnection<'a> {
    conn: MutexGuard<'a, Connection>,
    finished: bool,
}

impl GuardedConnection<'_> {
    fn commit(mut self) -> Result<()> {
        self.conn
            .execute_batch("COMMIT")
            .context("Failed to commit guarded SQLite operation")?;
        self.finished = true;
        Ok(())
    }
}

impl Deref for GuardedConnection<'_> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

impl Drop for GuardedConnection<'_> {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.conn.execute_batch("ROLLBACK");
        }
    }
}

impl PipelineTracker {
    pub fn new(db_path: &std::path::Path) -> Result<Self> {
        let (conn, migration_outcome) = open_migrated_database(db_path)?;

        Ok(Self {
            conn: Mutex::new(conn),
            db_path: db_path.to_path_buf(),
            migration_outcome,
        })
    }

    pub fn init_schema(&self) -> Result<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire database lock: {}", e))?;
        migrate_database(&mut conn, &self.db_path, MIGRATIONS)?;
        // debug, not info: this fires on every PipelineTracker::new() call,
        // including from background tasks (e.g. the TUI's async scan) where
        // an info-level println would corrupt the alternate-screen render.
        debug!("SQLite schema initialized");
        Ok(())
    }

    pub fn migration_outcome(&self) -> &MigrationOutcome {
        &self.migration_outcome
    }

    fn current_connection(&self) -> Result<GuardedConnection<'_>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire database lock: {}", e))?;
        conn.execute_batch("BEGIN DEFERRED TRANSACTION")
            .context("Failed to start guarded SQLite operation")?;
        let version = match read_user_version(&conn) {
            Ok(version) => version,
            Err(error) => {
                let _ = conn.execute_batch("ROLLBACK");
                return Err(error);
            }
        };
        if version != CURRENT_SCHEMA_VERSION {
            conn.execute_batch("ROLLBACK")
                .context("Failed to roll back stale SQLite operation")?;
            anyhow::bail!(
                "Database schema version changed to {version} while this tracker was open; expected {CURRENT_SCHEMA_VERSION}. Refusing to continue with stale schema semantics; reopen {} with a compatible binary",
                self.db_path.display()
            );
        }
        Ok(GuardedConnection {
            conn,
            finished: false,
        })
    }

    pub fn save_profile(&self, profile: &UserProfile) -> Result<()> {
        let conn = self.current_connection()?;
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
        conn.commit()?;
        Ok(())
    }

    pub fn save_job(&self, job: &Job) -> Result<()> {
        let conn = self.current_connection()?;
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
        conn.commit()?;
        Ok(())
    }

    pub fn get_job(&self, job_id: &str) -> Result<Option<Job>> {
        let conn = self.current_connection()?;
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
        let conn = self.current_connection()?;
        let rows_affected = conn.execute(
            "UPDATE pipeline SET status = ?1, updated_at = ?2 WHERE job_id = ?3",
            params![
                serde_json::to_string(&status).unwrap_or_default(),
                Utc::now().to_rfc3339(),
                job_id
            ],
        )?;
        conn.commit()?;
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
        let (rows_affected, status_to_log) = {
            let conn = self.current_connection()?;
            let status_json = status
                .as_ref()
                .map(|s| serde_json::to_string(s).unwrap_or_default());
            let follow_up_str = follow_up_date.map(|d| d.to_rfc3339());
            let rows = conn.execute(
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
            conn.commit()?;
            (rows, status)
        };
        if let Some(s) = status_to_log.as_ref() {
            // Best-effort: a feedback-write failure shouldn't block the
            // status update itself - one missing calibration data point is
            // recoverable, one blocked status update is not.
            let _ = self.log_status_to_feedback(job_id, s);
        }
        Ok(rows_affected)
    }

    /// Issue #10 — wire the existing `engine::feedback` table to real
    /// pipeline-status transitions, so the LLM scoring prediction can
    /// eventually be validated against real human outcomes instead of
    /// remaining purely formula-only. Every transition to a terminal
    /// status (Interviewing / Offered / Rejected) is recorded with
    /// task=`scoring`, action=Accepted (Interviewing/Offered) or Ignored
    /// (Rejected). Keeping the heuristic here — not in the CLI handler —
    /// means any status-change path (the CLI, programmatic callers, future
    /// automation) automatically feeds the calibration loop.
    fn log_status_to_feedback(&self, job_id: &str, new_status: &PipelineStatus) -> Result<()> {
        use crate::engine::feedback::{FeedbackAction, FeedbackTask, FeedbackTracker};
        let (action, summary) = match new_status {
            PipelineStatus::Interviewing => (
                FeedbackAction::Accepted,
                "Pipeline: Interviewing - scoring prediction progressed past initial screen",
            ),
            PipelineStatus::Offered => (
                FeedbackAction::Accepted,
                "Pipeline: Offered - scoring prediction now validated by real outcome",
            ),
            PipelineStatus::Rejected => (
                FeedbackAction::Ignored,
                "Pipeline: Rejected - scoring prediction was off",
            ),
            _ => return Ok(()),
        };
        let tracker = FeedbackTracker::new(&self.db_path)?;
        tracker.record_feedback(
            job_id,
            FeedbackTask::Scoring,
            action,
            summary,
            None,
            0.0,
            1.0,
        )?;
        Ok(())
    }

    pub fn save_evaluation(&self, eval: &Evaluation) -> Result<()> {
        let conn = self.current_connection()?;
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
        conn.commit()?;
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
        let conn = self.current_connection()?;
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
        conn.commit()?;
        Ok(())
    }

    /// The most recent tailored resume/cover letter generated for a job, if
    /// any - what you'd want to see again ahead of an interview.
    pub fn get_latest_application(
        &self,
        job_id: &str,
    ) -> Result<Option<crate::models::job::Application>> {
        let conn = self.current_connection()?;
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
        let conn = self.current_connection()?;
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
        conn.commit()?;
        Ok(entry)
    }

    pub fn update_pipeline_status(&self, entry_id: &str, status: PipelineStatus) -> Result<()> {
        let conn = self.current_connection()?;
        conn.execute(
            "UPDATE pipeline SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                serde_json::to_string(&status).unwrap_or_default(),
                Utc::now().to_rfc3339(),
                entry_id
            ],
        )?;
        conn.commit()?;
        Ok(())
    }

    pub fn list_pipeline(&self) -> Result<Vec<PipelineEntry>> {
        let conn = self.current_connection()?;
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
        let conn = self.current_connection()?;
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
        let conn = self.current_connection()?;
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

        events.sort_by_key(|e| std::cmp::Reverse(e.timestamp));
        events.truncate(limit);
        Ok(events)
    }

    /// Full evaluation (dimensions, strengths, gaps, red flags) for a job's
    /// most recent evaluation, if one exists - backs the TUI's detail panel.
    pub fn get_latest_evaluation(&self, job_id: &str) -> Result<Option<Evaluation>> {
        let conn = self.current_connection()?;
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
        let conn = self.current_connection()?;
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
        let conn = self.current_connection()?;
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
        let conn = self.current_connection()?;
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
                // rusqlite's ToSql no longer covers u64 directly (removed
                // upstream over platform-width ambiguity) - i64 is what
                // SQLite's INTEGER affinity actually stores.
                role.compensation_band.min as i64,
                role.compensation_band.max as i64,
                role.compensation_band.median as i64,
                role.compensation_band.source,
                serde_json::to_string(&role.typical_requirements).unwrap_or_default(),
                serde_json::to_string(&role.top_companies).unwrap_or_default(),
                role.inferred_from_profile,
                role.created_at.to_rfc3339(),
            ],
        )?;
        conn.commit()?;
        Ok(())
    }

    /// Persist discovered company boards to the SQLite `company_boards`
    /// table (issue #1).
    pub fn save_company_boards(&self, boards: &[DiscoveredBoard]) -> Result<()> {
        let conn = self.current_connection()?;

        let now = Utc::now().to_rfc3339();
        for board in boards {
            conn.execute(
                "INSERT OR REPLACE INTO company_boards (company, ats_type, slug, source_url, discovered_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    board.company,
                    board.ats_type.as_str(),
                    board.slug,
                    board.source_url,
                    now.clone(),
                ],
            )?;
        }
        conn.commit()?;
        Ok(())
    }

    /// Load all persisted discovered company boards.
    pub fn load_company_boards(&self) -> Result<Vec<DiscoveredBoard>> {
        let conn = self.current_connection()?;
        let mut stmt =
            conn.prepare("SELECT company, ats_type, slug, source_url FROM company_boards")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let ats_type_str: String = row.get(1)?;
            let Ok(ats_type) = ats_type_str.parse::<AtsType>() else {
                continue;
            };
            out.push(DiscoveredBoard {
                company: row.get(0)?,
                ats_type,
                slug: row.get(2)?,
                source_url: row.get(3)?,
            });
        }
        Ok(out)
    }

    /// Persist observed provider quota (Phase 1 Compute Broker).
    pub fn save_provider_quota(&self, broker: &ComputeBroker) -> Result<()> {
        let conn = self.current_connection()?;

        for (provider, quota) in &broker.quota_cache {
            let tier_type = broker
                .providers
                .iter()
                .find(|p| &p.name == provider)
                .map(|p| p.tier_type.as_str())
                .unwrap_or("configured");
            conn.execute(
                "INSERT OR REPLACE INTO provider_quota (provider, tier_type, remaining_requests, remaining_tokens, resets_at, reliability_score, last_observed)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    provider,
                    tier_type,
                    quota.remaining_requests,
                    quota.remaining_tokens,
                    quota.resets_at,
                    quota.reliability_score,
                    quota.last_observed,
                ],
            )?;
        }
        conn.commit()?;
        Ok(())
    }

    /// Load persisted provider quota.
    pub fn load_provider_quota(&self) -> Result<HashMap<String, ProviderQuota>> {
        let conn = self.current_connection()?;
        let mut stmt = conn.prepare("SELECT provider, remaining_requests, remaining_tokens, resets_at, reliability_score, last_observed FROM provider_quota")?;
        let mut rows = stmt.query([])?;
        let mut out = HashMap::new();
        while let Some(row) = rows.next()? {
            out.insert(
                row.get(0)?,
                ProviderQuota {
                    remaining_requests: row.get(1)?,
                    remaining_tokens: row.get(2)?,
                    resets_at: row.get(3)?,
                    reliability_score: row.get(4)?,
                    last_observed: row.get(5)?,
                },
            );
        }
        Ok(out)
    }
}

pub(crate) fn open_current_database(db_path: &Path) -> Result<Connection> {
    open_migrated_database(db_path).map(|(conn, _)| conn)
}

pub(crate) fn with_current_database_transaction<T, F>(db_path: &Path, operation: F) -> Result<T>
where
    F: FnOnce(&Transaction<'_>) -> Result<T>,
{
    let (mut conn, _) = open_migrated_database(db_path)?;
    let transaction = conn
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .context("Failed to start guarded SQLite operation")?;
    let version = read_user_version(&transaction)?;
    if version != CURRENT_SCHEMA_VERSION {
        anyhow::bail!(
            "Database schema version changed to {version} while opening {}; expected {CURRENT_SCHEMA_VERSION}. Refusing to continue with stale schema semantics",
            db_path.display()
        );
    }
    let output = operation(&transaction)?;
    transaction
        .commit()
        .context("Failed to commit guarded SQLite operation")?;
    Ok(output)
}

fn open_migrated_database(db_path: &Path) -> Result<(Connection, MigrationOutcome)> {
    let mut conn = Connection::open(db_path).context("Failed to open SQLite database")?;
    let migration_outcome = migrate_database(&mut conn, db_path, MIGRATIONS)?;
    Ok((conn, migration_outcome))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_version(conn: &Connection) -> i64 {
        conn.query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap()
    }

    fn row_count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    fn backup_files(db_path: &Path) -> Vec<PathBuf> {
        let parent = db_path.parent().unwrap();
        let prefix = format!("{}.backup-", db_path.file_name().unwrap().to_string_lossy());
        let mut backups = std::fs::read_dir(parent)
            .unwrap()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().starts_with(&prefix))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        backups.sort();
        backups
    }

    fn create_legacy_database(db_path: &Path, evaluations: usize, pipeline: usize) {
        let conn = Connection::open(db_path).unwrap();
        conn.execute_batch(INITIAL_SCHEMA_SQL).unwrap();
        for index in 0..evaluations {
            conn.execute(
                "INSERT INTO evaluations (
                    id, job_id, overall_score, overall_grade, dimensions,
                    match_summary, strengths, gaps, red_flags, recommendation,
                    model_used, evaluated_at
                 ) VALUES (?1, ?2, 0.8, 'B+', '[]', 'seed', '[]', '[]', '[]',
                           'apply', 'test-model', '2026-07-30T00:00:00Z')",
                params![format!("evaluation-{index}"), format!("job-{index}")],
            )
            .unwrap();
        }
        for index in 0..pipeline {
            conn.execute(
                "INSERT INTO pipeline (
                    id, job_id, status, notes, contact, follow_up_date,
                    created_at, updated_at
                 ) VALUES (?1, ?2, '\"new\"', 'seed', NULL, NULL,
                           '2026-07-30T00:00:00Z', '2026-07-30T00:00:00Z')",
                params![format!("pipeline-{index}"), format!("job-{index}")],
            )
            .unwrap();
        }
        conn.pragma_update(None, "user_version", 0).unwrap();
    }

    fn create_old_shape_database(db_path: &Path, evaluations: usize, pipeline: usize) {
        create_legacy_database(db_path, evaluations, pipeline);
        let conn = Connection::open(db_path).unwrap();
        conn.execute_batch(
            "
            DROP TABLE applications;
            DROP TABLE company_boards;
            DROP TABLE provider_quota;
            ",
        )
        .unwrap();
    }

    fn assert_database_integrity(db_path: &Path, evaluations: i64, pipeline: i64, version: i64) {
        let conn = Connection::open(db_path).unwrap();
        let integrity: String = conn
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
        assert_eq!(user_version(&conn), version);
        assert_eq!(row_count(&conn, "evaluations"), evaluations);
        assert_eq!(row_count(&conn, "pipeline"), pipeline);
    }

    #[test]
    fn fresh_database_uses_migrations_without_creating_a_backup() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("fresh.db");

        let tracker = PipelineTracker::new(&db_path).unwrap();

        assert_eq!(
            tracker.migration_outcome(),
            &MigrationOutcome::Created {
                version: CURRENT_SCHEMA_VERSION
            }
        );
        drop(tracker);
        assert!(backup_files(&db_path).is_empty());

        let conn = Connection::open(&db_path).unwrap();
        assert_eq!(user_version(&conn), i64::from(CURRENT_SCHEMA_VERSION));
        for table in [
            "profiles",
            "roles",
            "jobs",
            "evaluations",
            "pipeline",
            "applications",
            "market_data",
            "company_boards",
            "provider_quota",
            "feedback",
        ] {
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "migration did not create {table}");
        }
    }

    #[test]
    fn legacy_database_is_backed_up_adopted_and_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("legacy.db");
        create_old_shape_database(&db_path, 3, 4);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }

        let tracker = PipelineTracker::new(&db_path).unwrap();
        let backup_path = match tracker.migration_outcome() {
            MigrationOutcome::Migrated {
                from,
                to,
                backup_path,
            } => {
                assert_eq!((*from, *to), (0, CURRENT_SCHEMA_VERSION));
                backup_path.clone()
            }
            other => panic!("expected legacy migration, got {other:?}"),
        };
        drop(tracker);

        assert_eq!(backup_files(&db_path), vec![backup_path.clone()]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&backup_path)
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        assert_database_integrity(&db_path, 3, 4, i64::from(CURRENT_SCHEMA_VERSION));
        assert_database_integrity(&backup_path, 3, 4, 0);
        let live = Connection::open(&db_path).unwrap();
        for added_table in [
            "applications",
            "company_boards",
            "provider_quota",
            "feedback",
        ] {
            let exists: i64 = live
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [added_table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "legacy adoption did not add {added_table}");
        }
    }

    #[test]
    fn feedback_migration_preserves_existing_feedback_rows() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("feedback-v1.db");
        create_legacy_database(&db_path, 1, 1);
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(FEEDBACK_SCHEMA_SQL).unwrap();
            conn.execute(
                "INSERT INTO feedback (
                    job_id, task_type, action, recommendation_text,
                    confidence_before, confidence_after
                 ) VALUES ('job-1', 'scoring', 'accepted', 'keep me', 0.5, 0.8)",
                [],
            )
            .unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
        }

        let tracker = PipelineTracker::new(&db_path).unwrap();
        let backup_path = match tracker.migration_outcome() {
            MigrationOutcome::Migrated {
                from,
                to,
                backup_path,
            } => {
                assert_eq!((*from, *to), (1, CURRENT_SCHEMA_VERSION));
                backup_path.clone()
            }
            other => panic!("expected feedback migration, got {other:?}"),
        };
        drop(tracker);

        for path in [&db_path, &backup_path] {
            let conn = Connection::open(path).unwrap();
            assert_eq!(row_count(&conn, "feedback"), 1);
            let text: String = conn
                .query_row(
                    "SELECT recommendation_text FROM feedback WHERE job_id = 'job-1'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(text, "keep me");
        }
        let live = Connection::open(&db_path).unwrap();
        assert_eq!(user_version(&live), i64::from(CURRENT_SCHEMA_VERSION));
        let backup = Connection::open(&backup_path).unwrap();
        assert_eq!(user_version(&backup), 1);
    }

    #[test]
    fn repeated_open_is_a_noop_and_does_not_replace_the_backup() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("repeat.db");
        create_old_shape_database(&db_path, 1, 1);

        let first = PipelineTracker::new(&db_path).unwrap();
        assert!(matches!(
            first.migration_outcome(),
            MigrationOutcome::Migrated { .. }
        ));
        drop(first);
        let backups_after_first = backup_files(&db_path);

        let second = PipelineTracker::new(&db_path).unwrap();
        assert_eq!(
            second.migration_outcome(),
            &MigrationOutcome::Current {
                version: CURRENT_SCHEMA_VERSION
            }
        );
        drop(second);

        assert_eq!(backup_files(&db_path), backups_after_first);
        assert_database_integrity(&db_path, 1, 1, i64::from(CURRENT_SCHEMA_VERSION));
    }

    fn migration_1_succeeds(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
        transaction.execute_batch("CREATE TABLE migration_1_applied (id INTEGER);")
    }

    fn migration_2_succeeds(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
        transaction.execute_batch("CREATE TABLE migration_2_applied (id INTEGER);")
    }

    #[test]
    fn migration_lock_precedes_backup_and_blocks_concurrent_writers() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("concurrent.db");
        create_legacy_database(&db_path, 2, 3);
        let mut conn = Connection::open(&db_path).unwrap();
        let migrations = [
            Migration {
                version: 1,
                name: "first_migration",
                apply: migration_1_succeeds,
            },
            Migration {
                version: 2,
                name: "second_migration",
                apply: migration_2_succeeds,
            },
        ];

        let mut concurrent_write_blocked = false;
        let outcome =
            migrate_database_to_with_lock_hook(&mut conn, &db_path, &migrations, 2, || {
                let concurrent = Connection::open(&db_path)?;
                concurrent.busy_timeout(std::time::Duration::ZERO)?;
                let error = concurrent
                    .pragma_update(None, "user_version", 1)
                    .expect_err("the migration lock must block writers before backup");
                concurrent_write_blocked = matches!(
                    &error,
                    rusqlite::Error::SqliteFailure(failure, _)
                        if matches!(
                            failure.code,
                            rusqlite::ErrorCode::DatabaseBusy
                                | rusqlite::ErrorCode::DatabaseLocked
                        )
                );
                anyhow::ensure!(
                    concurrent_write_blocked,
                    "concurrent schema write failed for an unexpected reason: {error}"
                );
                Ok(())
            })
            .unwrap();
        drop(conn);

        assert!(concurrent_write_blocked);
        assert!(matches!(
            outcome,
            MigrationOutcome::Migrated { from: 0, to: 2, .. }
        ));
        assert_database_integrity(&db_path, 2, 3, 2);
        let live = Connection::open(&db_path).unwrap();
        assert_eq!(row_count(&live, "migration_1_applied"), 0);
        assert_eq!(row_count(&live, "migration_2_applied"), 0);

        let backups = backup_files(&db_path);
        assert_eq!(backups.len(), 1);
        assert_database_integrity(&backups[0], 2, 3, 0);
    }

    #[test]
    fn locked_backup_eligibility_protects_state_added_after_connection_open() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("racing-initializer.db");
        std::fs::File::create(&db_path).unwrap();
        let mut migrating = Connection::open(&db_path).unwrap();

        create_legacy_database(&db_path, 2, 3);
        let outcome = migrate_database(&mut migrating, &db_path, MIGRATIONS).unwrap();
        drop(migrating);

        let backup_path = match outcome {
            MigrationOutcome::Migrated {
                from,
                to,
                backup_path,
            } => {
                assert_eq!((from, to), (0, CURRENT_SCHEMA_VERSION));
                backup_path
            }
            other => panic!("expected protected migration, got {other:?}"),
        };
        assert_database_integrity(&db_path, 2, 3, i64::from(CURRENT_SCHEMA_VERSION));
        assert_database_integrity(&backup_path, 2, 3, 0);
    }

    fn migration_2_fails(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
        transaction.execute_batch("CREATE TABLE must_rollback (id INTEGER);")?;
        Err(rusqlite::Error::InvalidQuery)
    }

    #[test]
    fn failed_migration_rolls_back_schema_data_and_version() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("failed.db");
        create_legacy_database(&db_path, 2, 3);
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
        }
        let mut conn = Connection::open(&db_path).unwrap();
        let migrations = [
            MIGRATIONS[0],
            Migration {
                version: 2,
                name: "injected_failure",
                apply: migration_2_fails,
            },
        ];

        let error = migrate_database_to(&mut conn, &db_path, &migrations, 2)
            .expect_err("the injected migration must fail");
        assert!(error.to_string().contains("migration 2"));
        drop(conn);

        assert_database_integrity(&db_path, 2, 3, 1);
        let live = Connection::open(&db_path).unwrap();
        let rolled_back_table: i64 = live
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'must_rollback'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(rolled_back_table, 0);

        let backups = backup_files(&db_path);
        assert_eq!(backups.len(), 1);
        assert_database_integrity(&backups[0], 2, 3, 1);
    }

    #[test]
    fn newer_database_is_refused_without_writing_or_backing_up() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("newer.db");
        create_legacy_database(&db_path, 2, 2);
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.pragma_update(None, "user_version", i64::from(CURRENT_SCHEMA_VERSION + 1))
                .unwrap();
        }

        let error = PipelineTracker::new(&db_path)
            .err()
            .expect("a newer schema must be refused");

        assert!(error.to_string().contains("newer than this binary"));
        assert!(backup_files(&db_path).is_empty());
        assert_database_integrity(&db_path, 2, 2, i64::from(CURRENT_SCHEMA_VERSION + 1));
    }

    #[test]
    fn persistent_tracker_refuses_reads_and_writes_after_schema_advances() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("advanced-after-open.db");
        let tracker = PipelineTracker::new(&db_path).unwrap();
        let entry = tracker
            .add_pipeline_entry("job-1", PipelineStatus::New)
            .unwrap();

        let concurrent = Connection::open(&db_path).unwrap();
        concurrent
            .pragma_update(None, "user_version", i64::from(CURRENT_SCHEMA_VERSION + 1))
            .unwrap();
        drop(concurrent);

        let read_error = tracker
            .list_pipeline()
            .expect_err("a stale persistent tracker must refuse reads");
        assert!(read_error.to_string().contains("stale schema semantics"));
        let write_error = tracker
            .update_pipeline_status(&entry.id, PipelineStatus::Applied)
            .expect_err("a stale persistent tracker must refuse writes");
        assert!(write_error.to_string().contains("stale schema semantics"));

        let conn = Connection::open(&db_path).unwrap();
        let stored_status: String = conn
            .query_row(
                "SELECT status FROM pipeline WHERE id = ?1",
                [&entry.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<PipelineStatus>(&stored_status).unwrap(),
            PipelineStatus::New
        );
    }

    #[test]
    fn guarded_operation_holds_database_snapshot_through_its_write() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("guarded-operation.db");
        let tracker = PipelineTracker::new(&db_path).unwrap();
        let entry = tracker
            .add_pipeline_entry("job-1", PipelineStatus::New)
            .unwrap();

        let guarded = tracker.current_connection().unwrap();
        let concurrent = Connection::open(&db_path).unwrap();
        concurrent.busy_timeout(std::time::Duration::ZERO).unwrap();
        let error = concurrent
            .pragma_update(None, "user_version", i64::from(CURRENT_SCHEMA_VERSION + 1))
            .expect_err("a schema writer must not pass an active guarded operation");
        assert!(matches!(
            error,
            rusqlite::Error::SqliteFailure(ref failure, _)
                if matches!(
                    failure.code,
                    rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                )
        ));

        guarded
            .execute(
                "UPDATE pipeline SET status = ?1 WHERE id = ?2",
                params![
                    serde_json::to_string(&PipelineStatus::Applied).unwrap(),
                    entry.id
                ],
            )
            .unwrap();
        guarded.commit().unwrap();

        let conn = Connection::open(&db_path).unwrap();
        assert_eq!(user_version(&conn), i64::from(CURRENT_SCHEMA_VERSION));
        let stored_status: String = conn
            .query_row(
                "SELECT status FROM pipeline WHERE id = ?1",
                [&entry.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<PipelineStatus>(&stored_status).unwrap(),
            PipelineStatus::Applied
        );
    }
}
