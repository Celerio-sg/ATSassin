use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

const TRACKING_QUERY_KEYS: &[&str] = &["gclid", "ref", "source"];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobUrlKind {
    Posting,
    SearchPage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobIdentity {
    pub id: String,
    pub canonical_url: Option<String>,
}

impl JobIdentity {
    pub fn posting(url: &str) -> anyhow::Result<Self> {
        let canonical_url = canonicalize_job_url(url)?;
        Ok(Self {
            id: content_id(canonical_url.as_bytes()),
            canonical_url: Some(canonical_url),
        })
    }

    pub fn search_lead(company: &str, title: &str, location: &str) -> Self {
        let key = [
            normalize_identity_text(company),
            normalize_identity_text(title),
            normalize_identity_text(location),
        ]
        .join("\0");
        Self {
            id: content_id(key.as_bytes()),
            canonical_url: None,
        }
    }

    pub fn imported(description: &str) -> Self {
        let mut key = b"import\0".to_vec();
        key.extend_from_slice(description.as_bytes());
        Self {
            id: content_id(&key),
            canonical_url: None,
        }
    }
}

pub fn canonicalize_job_url(raw_url: &str) -> anyhow::Result<String> {
    let mut url =
        Url::parse(raw_url.trim()).map_err(|error| anyhow::anyhow!("Invalid job URL: {error}"))?;
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Job URL has no host"))?
        .to_lowercase();
    url.set_host(Some(&host))
        .map_err(|_| anyhow::anyhow!("Job URL host cannot be canonicalised"))?;
    url.set_fragment(None);

    if url.path().len() > 1 {
        let trimmed = url.path().trim_end_matches('/').to_string();
        url.set_path(&trimmed);
    }

    let mut retained = url
        .query_pairs()
        .filter(|(key, _)| {
            let key = key.to_ascii_lowercase();
            !key.starts_with("utm_") && !TRACKING_QUERY_KEYS.contains(&key.as_str())
        })
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    retained.sort();
    url.set_query(None);
    if !retained.is_empty() {
        url.query_pairs_mut().extend_pairs(retained);
    }

    Ok(url.to_string())
}

pub fn normalize_identity_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn content_id(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub canonical_url: Option<String>,
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

impl Job {
    pub fn imported(description: String, scraped_at: DateTime<Utc>) -> Self {
        let identity = JobIdentity::imported(&description);
        Self {
            id: identity.id,
            canonical_url: identity.canonical_url,
            title: "Imported Job".to_string(),
            company: "Unknown".to_string(),
            location: "Unknown".to_string(),
            remote: false,
            job_type: None,
            salary_range: None,
            description,
            requirements: vec![],
            posted_at: None,
            source: "file".to_string(),
            url: String::new(),
            applied: false,
            scraped_at,
        }
    }
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

/// A snapshot of exactly what was submitted for a job, recorded whenever
/// `tailor` generates a resume/cover letter - so that months later, if a
/// job's own posting has been taken down or edited, there's still a record
/// of what was actually sent and for which role. The job's own row (title,
/// company, description) is the other half of that record - joined by
/// `job_id`, never duplicated here.
#[derive(Debug, Clone)]
pub struct Application {
    pub id: String,
    pub job_id: String,
    pub resume_text: String,
    pub cover_letter_text: String,
    pub model_used: String,
    pub generated_at: DateTime<Utc>,
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

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn canonical_url_is_idempotent_and_removes_tracking() {
        let raw = "HTTPS://Example.COM/jobs/42/?utm_source=x&b=2&A=1&gclid=y#apply";
        let once = canonicalize_job_url(raw).unwrap();
        let twice = canonicalize_job_url(&once).unwrap();
        assert_eq!(once, "https://example.com/jobs/42?A=1&b=2");
        assert_eq!(twice, once);
    }

    #[test]
    fn canonical_url_sorts_query_pairs_and_preserves_root_slash() {
        assert_eq!(
            canonicalize_job_url("https://EXAMPLE.com/?z=2&a=3&a=1").unwrap(),
            "https://example.com/?a=1&a=3&z=2"
        );
    }

    #[test]
    fn posting_identity_is_stable_and_128_bits() {
        let first = JobIdentity::posting("https://example.com/job/1?utm_campaign=x").unwrap();
        let second = JobIdentity::posting("https://EXAMPLE.com/job/1/").unwrap();
        assert_eq!(first, second);
        assert_eq!(first.id.len(), 32);
        assert!(first
            .id
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
    }

    #[test]
    fn search_identity_normalizes_unicode_case_and_whitespace() {
        let first = JobIdentity::search_lead("  ACME  APAC ", "DÉVELOPPEUR", "São  Paulo");
        let second = JobIdentity::search_lead("acme apac", "développeur", "são paulo");
        assert_eq!(first, second);
        assert_ne!(
            first,
            JobIdentity::search_lead("acme apac", "développeur", "Singapore")
        );
    }

    #[test]
    fn imported_identity_is_repeatable_but_content_sensitive() {
        assert_eq!(
            JobIdentity::imported("role description"),
            JobIdentity::imported("role description")
        );
        assert_ne!(
            JobIdentity::imported("role description"),
            JobIdentity::imported("different description")
        );
        assert_ne!(
            JobIdentity::imported("https://example.com/jobs/42"),
            JobIdentity::posting("https://example.com/jobs/42").unwrap()
        );
    }
}
