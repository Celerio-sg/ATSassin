//! Honest, local job-preference filtering. Every check runs against real
//! fields already on a scraped `Job` (salary_range, location, description,
//! remote flag) with plain string/number matching - zero LLM calls, zero
//! invented data. When a field can't be determined from what was actually
//! scraped, the job is never penalized for it: an unknown is not a failure.

use crate::config::{EmploymentTypePref, JobPreferences, WorkModePref};
use crate::models::job::{Job, JobRow};

/// Result of checking one job against the user's preferences.
#[derive(Debug, Clone)]
pub struct PreferenceMatch {
    pub matches: bool,
    /// Human-readable reasons for a non-match (empty if it matches, or if
    /// no preference was strict enough to reject it).
    pub reasons: Vec<String>,
}

/// The minimal set of real, already-scraped fields any preference check
/// needs. `Job` and `JobRow` both provide it, so the same filtering logic
/// runs identically whether it's called from `scan` (has a full `Job`) or
/// the TUI's job table (has a `JobRow`).
pub struct JobFacts<'a> {
    pub title: &'a str,
    pub location: &'a str,
    pub description: &'a str,
    pub remote: bool,
    pub salary_range: Option<&'a str>,
}

impl<'a> From<&'a Job> for JobFacts<'a> {
    fn from(j: &'a Job) -> Self {
        Self {
            title: &j.title,
            location: &j.location,
            description: &j.description,
            remote: j.remote,
            salary_range: j.salary_range.as_deref(),
        }
    }
}

impl<'a> From<&'a JobRow> for JobFacts<'a> {
    fn from(j: &'a JobRow) -> Self {
        Self {
            title: &j.title,
            location: &j.location,
            description: &j.description,
            remote: j.remote,
            salary_range: j.salary_range.as_deref(),
        }
    }
}

pub fn check<'a>(facts: impl Into<JobFacts<'a>>, prefs: &JobPreferences) -> PreferenceMatch {
    let job = facts.into();
    let mut reasons = Vec::new();

    if let Some(min) = prefs.min_comp_usd {
        if let Some(found) = extract_max_annual_usd(job.salary_range, job.description) {
            if found < min {
                reasons.push(format!("comp ~${found} below your ${min} floor"));
            }
        }
        // No parseable figure -> can't evaluate, so we don't reject it.
    }

    match prefs.employment_type {
        EmploymentTypePref::Any => {}
        EmploymentTypePref::FullTimeOnly => {
            if mentions(
                &job,
                &[
                    "contract",
                    "contractor",
                    "freelance",
                    "temporary",
                    "temp role",
                ],
            ) && !mentions(&job, &["full-time", "full time", "permanent", "fte"])
            {
                reasons.push("looks like contract/temporary, not full-time".to_string());
            }
        }
        EmploymentTypePref::ContractOnly => {
            if mentions(&job, &["full-time", "full time", "permanent"])
                && !mentions(&job, &["contract", "contractor", "freelance", "interim"])
            {
                reasons.push("looks like full-time/permanent, not contract".to_string());
            }
        }
    }

    match prefs.work_mode {
        WorkModePref::Any => {}
        WorkModePref::RemoteOnly => {
            if !job.remote && !mentions(&job, &["remote", "work from home", "wfh"]) {
                reasons.push("no remote signal found (location/description)".to_string());
            }
        }
        WorkModePref::HybridOrRemote => {
            if !job.remote && !mentions(&job, &["remote", "hybrid", "work from home"]) {
                reasons.push("no remote/hybrid signal found".to_string());
            }
        }
        WorkModePref::OnsiteOk => {}
    }

    PreferenceMatch {
        matches: reasons.is_empty(),
        reasons,
    }
}

fn mentions(job: &JobFacts, needles: &[&str]) -> bool {
    let haystack = format!("{} {} {}", job.title, job.location, job.description).to_lowercase();
    needles.iter().any(|n| haystack.contains(n))
}

/// Pulls the largest plausible annual USD figure out of free-text
/// salary_range/description (e.g. "$220,000-$260,000 USD" -> 260000,
/// "$150k-180k" -> 180000). Ignores small numbers that look like hourly
/// rates rather than annual comp. Returns None if nothing plausible is
/// found - callers must treat that as "unknown", not "fails the filter".
pub fn extract_max_annual_usd(salary_range: Option<&str>, description: &str) -> Option<u64> {
    let text = salary_range.unwrap_or(description);
    // The second half of a range often drops the leading '$' entirely
    // ("$150k-180k", "$220,000-260,000") - the optional trailing group
    // below captures that continuation without requiring its own '$'.
    let re = regex::Regex::new(r"\$\s?([\d]{1,3}(?:,\d{3})*|\d+)(k|K)?(?:\s*[-\x{2013}]\s*\$?\s*([\d]{1,3}(?:,\d{3})*|\d+)(k|K)?)?").ok()?;
    let mut max_found: Option<u64> = None;
    let consider = |raw: &str, has_k: bool, max_found: &mut Option<u64>| {
        let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
        let Ok(mut value) = digits.parse::<u64>() else {
            return;
        };
        if has_k {
            value *= 1000;
        }
        // Hourly/day rates read as small numbers ($95-165/hr) - not a
        // meaningful comparison against an annual floor, so skip them.
        if value < 1000 {
            return;
        }
        *max_found = Some(max_found.map_or(value, |m: u64| m.max(value)));
    };
    for cap in re.captures_iter(text) {
        consider(&cap[1], cap.get(2).is_some(), &mut max_found);
        if let Some(second) = cap.get(3) {
            consider(second.as_str(), cap.get(4).is_some(), &mut max_found);
        }
    }
    max_found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::JobPreferences;
    use chrono::Utc;

    fn job(
        salary: Option<&str>,
        description: &str,
        remote: bool,
        title: &str,
        location: &str,
    ) -> Job {
        Job {
            id: "1".to_string(),
            title: title.to_string(),
            company: "Acme".to_string(),
            location: location.to_string(),
            remote,
            job_type: None,
            salary_range: salary.map(|s| s.to_string()),
            description: description.to_string(),
            requirements: vec![],
            posted_at: None,
            source: "test".to_string(),
            url: String::new(),
            applied: false,
            scraped_at: Utc::now(),
        }
    }

    #[test]
    fn extracts_max_from_range() {
        let j = job(
            Some("$220,000-$260,000 USD FTE equivalent"),
            "",
            false,
            "",
            "",
        );
        assert_eq!(
            extract_max_annual_usd(j.salary_range.as_deref(), &j.description),
            Some(260000)
        );
    }

    #[test]
    fn extracts_k_suffix() {
        let j = job(Some("$150k-180k"), "", false, "", "");
        assert_eq!(
            extract_max_annual_usd(j.salary_range.as_deref(), &j.description),
            Some(180000)
        );
    }

    #[test]
    fn ignores_hourly_rate() {
        let j = job(Some("$95-165/hr contract rate"), "", false, "", "");
        assert_eq!(
            extract_max_annual_usd(j.salary_range.as_deref(), &j.description),
            None
        );
    }

    #[test]
    fn unknown_comp_never_rejects() {
        let j = job(None, "no numbers here", false, "Engineer", "Remote");
        let prefs = JobPreferences {
            min_comp_usd: Some(200_000),
            ..Default::default()
        };
        assert!(check(&j, &prefs).matches);
    }

    #[test]
    fn below_floor_rejects() {
        let j = job(Some("$120,000"), "", false, "", "");
        let prefs = JobPreferences {
            min_comp_usd: Some(200_000),
            ..Default::default()
        };
        let result = check(&j, &prefs);
        assert!(!result.matches);
        assert!(!result.reasons.is_empty());
    }

    #[test]
    fn remote_only_accepts_remote_flag() {
        let j = job(None, "", true, "Engineer", "Anywhere");
        let prefs = JobPreferences {
            work_mode: WorkModePref::RemoteOnly,
            ..Default::default()
        };
        assert!(check(&j, &prefs).matches);
    }

    #[test]
    fn remote_only_rejects_onsite() {
        let j = job(
            None,
            "Must work on-site 5 days a week",
            false,
            "Engineer",
            "New York, NY",
        );
        let prefs = JobPreferences {
            work_mode: WorkModePref::RemoteOnly,
            ..Default::default()
        };
        assert!(!check(&j, &prefs).matches);
    }
}
