use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub title: String,
    pub company: String,
    pub location: String,
    pub remote: bool,
    pub job_type: Option<String>,
    pub salary_range: Option<String>,
    pub description: String,
    pub requirements: Vec<String>,
    pub posted_at: Option<DateTime<Utc>>,
    pub source: String,
    pub url: String,
    pub applied: bool,
    pub scraped_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    pub id: String,
    pub job_id: String,
    pub overall_score: f64,
    pub overall_grade: String,
    pub dimensions: Vec<DimensionScore>,
    pub match_summary: String,
    pub strengths: Vec<String>,
    pub gaps: Vec<String>,
    pub red_flags: Vec<String>,
    pub recommendation: Recommendation,
    pub model_used: String,
    pub evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub name: String,
    pub score: f64,
    pub max: f64,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Recommendation {
    Apply,
    Maybe,
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEntry {
    pub id: String,
    pub job_id: String,
    pub status: PipelineStatus,
    pub notes: Option<String>,
    pub contact: Option<String>,
    pub follow_up_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStatus {
    New,
    Evaluated,
    Drafted,
    Exported,
    Applied,
    Interviewing,
    Offered,
    Rejected,
    Archived,
}

/// A denormalized view row joining a job with its latest evaluation (if any)
/// and its latest pipeline status (if any) - used by the TUI job table so it
/// never has to fabricate scores or statuses for jobs that haven't been
/// evaluated or added to the pipeline yet.
#[derive(Debug, Clone)]
pub struct JobRow {
    pub id: String,
    pub title: String,
    pub company: String,
    pub location: String,
    pub url: String,
    pub description: String,
    pub salary_range: Option<String>,
    pub remote: bool,
    pub overall_score: Option<f64>,
    pub overall_grade: Option<String>,
    pub status: Option<PipelineStatus>,
    pub posted_at: Option<DateTime<Utc>>,
    pub scraped_at: DateTime<Utc>,
}

/// A real, derived timeline event - never fabricated. Built by merging
/// evaluation and pipeline-status timestamps that already exist in the
/// database, sorted newest-first.
#[derive(Debug, Clone)]
pub struct ActivityEvent {
    pub timestamp: DateTime<Utc>,
    pub description: String,
}

impl PipelineStatus {
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "new" => Ok(Self::New),
            "evaluated" => Ok(Self::Evaluated),
            "drafted" => Ok(Self::Drafted),
            "exported" => Ok(Self::Exported),
            "applied" => Ok(Self::Applied),
            "interviewing" => Ok(Self::Interviewing),
            "offered" => Ok(Self::Offered),
            "rejected" => Ok(Self::Rejected),
            "archived" => Ok(Self::Archived),
            other => anyhow::bail!("Invalid status '{}'. Valid statuses: new, evaluated, drafted, exported, applied, interviewing, offered, rejected, archived", other),
        }
    }
}
