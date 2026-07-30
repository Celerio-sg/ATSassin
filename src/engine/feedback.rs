use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use crate::pipeline::tracker::{open_current_database, with_current_database_transaction};

#[derive(Debug, Clone, Serialize)]
pub struct FeedbackEvent {
    pub id: i64,
    pub job_id: String,
    pub task_type: FeedbackTask,
    pub action: FeedbackAction,
    pub recommendation_text: String,
    pub edited_text: Option<String>,
    pub edit_distance: Option<usize>,
    pub confidence_before: f64,
    pub confidence_after: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackTask {
    Scoring,
    Tailoring,
    CoverLetter,
    DeepResearch,
    RoleInference,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackAction {
    Accepted,
    Edited,
    Ignored,
    Escalated,
}

impl std::fmt::Display for FeedbackTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedbackTask::Scoring => write!(f, "scoring"),
            FeedbackTask::Tailoring => write!(f, "tailoring"),
            FeedbackTask::CoverLetter => write!(f, "cover_letter"),
            FeedbackTask::DeepResearch => write!(f, "deep_research"),
            FeedbackTask::RoleInference => write!(f, "role_inference"),
        }
    }
}

impl std::fmt::Display for FeedbackAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedbackAction::Accepted => write!(f, "accepted"),
            FeedbackAction::Edited => write!(f, "edited"),
            FeedbackAction::Ignored => write!(f, "ignored"),
            FeedbackAction::Escalated => write!(f, "escalated"),
        }
    }
}

pub struct FeedbackTracker {
    db_path: std::path::PathBuf,
}

impl FeedbackTracker {
    pub fn new(db_path: &Path) -> Result<Self> {
        let tracker = Self {
            db_path: db_path.to_path_buf(),
        };
        open_current_database(&tracker.db_path)?;
        Ok(tracker)
    }

    #[allow(clippy::too_many_arguments)] // mirrors the feedback-event schema 1:1; a params struct would just move the same fields around
    pub fn record_feedback(
        &self,
        job_id: &str,
        task: FeedbackTask,
        action: FeedbackAction,
        recommendation: &str,
        edited: Option<&str>,
        confidence_before: f64,
        confidence_after: f64,
    ) -> Result<i64> {
        // rusqlite's ToSql/FromSql no longer cover usize/u64 directly (removed
        // upstream to avoid platform-width ambiguity) - store as i64, which is
        // what SQLite's INTEGER affinity actually is.
        let edit_distance = edited.map(|e| Self::edit_distance(recommendation, e) as i64);
        let task_str = task.to_string();
        let action_str = action.to_string();
        let recommendation_str = recommendation.to_string();
        let edited_str = edited.map(|s| s.to_string());
        with_current_database_transaction(&self.db_path, |conn| {
            let mut stmt = conn.prepare("INSERT INTO feedback (job_id, task_type, action, recommendation_text, edited_text, edit_distance, confidence_before, confidence_after) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")?;
            let id = stmt.insert(rusqlite::params![
                job_id,
                task_str,
                action_str,
                recommendation_str,
                edited_str,
                edit_distance,
                confidence_before,
                confidence_after,
            ])?;
            Ok(id)
        })
    }

    pub fn get_acceptance_rate(&self, task: FeedbackTask, window_days: i64) -> Result<f64> {
        with_current_database_transaction(&self.db_path, |conn| {
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM feedback WHERE task_type = ?1 AND action = 'accepted' AND created_at >= datetime('now', ?2)")?;
            let accepted: i64 = stmt.query_row(
                [&task.to_string(), &format!("-{} days", window_days)],
                |row| row.get(0),
            )?;

            let mut stmt = conn.prepare("SELECT COUNT(*) FROM feedback WHERE task_type = ?1 AND created_at >= datetime('now', ?2)")?;
            let total: i64 = stmt.query_row(
                [&task.to_string(), &format!("-{} days", window_days)],
                |row| row.get(0),
            )?;

            if total == 0 {
                return Ok(0.0);
            }
            Ok(accepted as f64 / total as f64)
        })
    }

    pub fn get_edit_distance_stats(&self, task: FeedbackTask) -> Result<Option<(f64, f64)>> {
        with_current_database_transaction(&self.db_path, |conn| {
            let mut stmt = conn.prepare("SELECT edit_distance FROM feedback WHERE task_type = ?1 AND action = 'edited' AND edit_distance IS NOT NULL")?;
            let distances: Vec<usize> = stmt
                .query_map([&task.to_string()], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|v| v as usize)
                .collect();

            if distances.is_empty() {
                return Ok(None);
            }
            let mean = distances.iter().sum::<usize>() as f64 / distances.len() as f64;
            let variance = distances
                .iter()
                .map(|d| (*d as f64 - mean).powi(2))
                .sum::<f64>()
                / distances.len() as f64;
            Ok(Some((mean, variance.sqrt())))
        })
    }

    pub fn should_escalate_automation(&self, task: FeedbackTask) -> Result<bool> {
        let acceptance_rate = self.get_acceptance_rate(task.clone(), 30)?;
        let edit_stats = self.get_edit_distance_stats(task)?;

        let high_acceptance = acceptance_rate >= 0.85;
        let low_edit_distance = edit_stats.map(|(mean, _)| mean < 50.0).unwrap_or(true);

        Ok(high_acceptance && low_edit_distance && acceptance_rate > 0.0)
    }

    pub fn list_recent(&self, limit: usize) -> Result<Vec<FeedbackEvent>> {
        with_current_database_transaction(&self.db_path, |conn| {
            let mut stmt = conn.prepare("SELECT id, job_id, task_type, action, recommendation_text, edited_text, edit_distance, confidence_before, confidence_after, created_at FROM feedback ORDER BY id DESC LIMIT ?1")?;
            let events = stmt
                .query_map([limit as i64], |row| {
                    Ok(FeedbackEvent {
                        id: row.get(0)?,
                        job_id: row.get(1)?,
                        task_type: match row.get::<_, String>(2)?.as_str() {
                            "scoring" => FeedbackTask::Scoring,
                            "tailoring" => FeedbackTask::Tailoring,
                            "cover_letter" => FeedbackTask::CoverLetter,
                            "deep_research" => FeedbackTask::DeepResearch,
                            "role_inference" => FeedbackTask::RoleInference,
                            _ => FeedbackTask::Scoring,
                        },
                        action: match row.get::<_, String>(3)?.as_str() {
                            "accepted" => FeedbackAction::Accepted,
                            "edited" => FeedbackAction::Edited,
                            "ignored" => FeedbackAction::Ignored,
                            "escalated" => FeedbackAction::Escalated,
                            _ => FeedbackAction::Ignored,
                        },
                        recommendation_text: row.get(4)?,
                        edited_text: row.get(5)?,
                        edit_distance: row.get::<_, Option<i64>>(6)?.map(|v| v as usize),
                        confidence_before: row.get(7)?,
                        confidence_after: row.get(8)?,
                        created_at: row.get(9)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(events)
        })
    }

    fn edit_distance(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let mut dp = vec![vec![0; b_chars.len() + 1]; a_chars.len() + 1];

        for (i, row) in dp.iter_mut().enumerate() {
            row[0] = i;
        }
        for (j, val) in dp[0].iter_mut().enumerate() {
            *val = j;
        }

        for i in 1..=a_chars.len() {
            for j in 1..=b_chars.len() {
                if a_chars[i - 1] == b_chars[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1];
                } else {
                    dp[i][j] = 1 + dp[i - 1][j - 1].min(dp[i - 1][j].min(dp[i][j - 1]));
                }
            }
        }

        dp[a_chars.len()][b_chars.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::tracker::{with_current_database_transaction, CURRENT_SCHEMA_VERSION};
    use rusqlite::Connection;

    #[test]
    fn feedback_open_refuses_a_newer_database_without_creating_schema() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("newer.sqlite3");
        let conn = Connection::open(&db_path).unwrap();
        conn.pragma_update(None, "user_version", i64::from(CURRENT_SCHEMA_VERSION + 1))
            .unwrap();
        drop(conn);

        let error = FeedbackTracker::new(&db_path)
            .err()
            .expect("feedback must refuse a newer shared schema");
        assert!(error.to_string().contains("newer than this binary"));

        let conn = Connection::open(&db_path).unwrap();
        let feedback_tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'feedback'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(feedback_tables, 0);
    }

    #[test]
    fn feedback_write_rechecks_the_shared_schema_version() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("advanced-after-open.sqlite3");
        let tracker = FeedbackTracker::new(&db_path).unwrap();
        let conn = Connection::open(&db_path).unwrap();
        conn.pragma_update(None, "user_version", i64::from(CURRENT_SCHEMA_VERSION + 1))
            .unwrap();
        drop(conn);

        let error = tracker
            .record_feedback(
                "job-1",
                FeedbackTask::Scoring,
                FeedbackAction::Accepted,
                "recommendation",
                None,
                0.5,
                0.5,
            )
            .expect_err("each feedback operation must re-check the schema guard");
        assert!(error.to_string().contains("newer than this binary"));

        let conn = Connection::open(&db_path).unwrap();
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM feedback", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 0);
    }

    #[test]
    fn feedback_transaction_blocks_a_concurrent_schema_writer() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("feedback-transaction.sqlite3");
        FeedbackTracker::new(&db_path).unwrap();

        with_current_database_transaction(&db_path, |conn| {
            let concurrent = Connection::open(&db_path)?;
            concurrent.busy_timeout(std::time::Duration::ZERO)?;
            let error = concurrent
                .pragma_update(None, "user_version", i64::from(CURRENT_SCHEMA_VERSION + 1))
                .expect_err("the feedback transaction must block a schema writer");
            assert!(matches!(
                error,
                rusqlite::Error::SqliteFailure(ref failure, _)
                    if matches!(
                        failure.code,
                        rusqlite::ErrorCode::DatabaseBusy
                            | rusqlite::ErrorCode::DatabaseLocked
                    )
            ));
            conn.execute(
                "INSERT INTO feedback (
                    job_id, task_type, action, recommendation_text,
                    confidence_before, confidence_after
                 ) VALUES ('job-1', 'scoring', 'accepted', 'guarded', 0.5, 0.8)",
                [],
            )?;
            Ok(())
        })
        .unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM feedback", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 1);
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, i64::from(CURRENT_SCHEMA_VERSION));
    }
}
