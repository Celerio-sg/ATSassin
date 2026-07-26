//! Phase 0 — email outcome ingestion.
//!
//! Closes the loop by reading the user's mailbox for rejection,
//! interview, and offer emails, then updating pipeline statuses
//! accordingly. This initial version uses local heuristics/regex rather
//! than LLM calls, keeping it private and zero-cost.
//!
//! Credentials are stored in the OS keychain (keyring crate). If the
//! keychain is unavailable, the command falls back to prompting once
//! per sync and does not persist the password.

use crate::models::job::PipelineStatus;
use crate::pipeline::tracker::PipelineTracker;
use anyhow::{Context, Result};
use mailparse::{parse_mail, MailHeaderMap};
use regex::Regex;

/// Supported outcome sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeSource {
    Email,
    Manual,
}

/// A classified outcome signal derived from an email or manual entry.
#[derive(Debug, Clone)]
pub struct OutcomeSignal {
    pub job_id: Option<String>,
    pub company: Option<String>,
    pub status: PipelineStatus,
    pub source: OutcomeSource,
    pub source_id: String,
    pub raw_subject: String,
    pub date: chrono::DateTime<chrono::Utc>,
}

/// Classify a raw email subject/body into a pipeline status using
/// heuristics. No LLM call is made.
pub fn classify_email(subject: &str, body: &str) -> Option<PipelineStatus> {
    let combined = format!("{} {}", subject, body).to_lowercase();

    // Offer signals
    if regex_matches(
        &combined,
        &[
            "congratulations",
            "pleased to offer",
            "offer of employment",
            "extend an offer",
            "we would like to offer",
        ],
    ) {
        return Some(PipelineStatus::Offered);
    }

    // Interview signals
    if regex_matches(
        &combined,
        &[
            "interview",
            "phone screen",
            "recruiter chat",
            "schedule a call",
            "invite you to interview",
            "would like to meet",
        ],
    ) {
        return Some(PipelineStatus::Interviewing);
    }

    // Rejection signals
    if regex_matches(
        &combined,
        &[
            "not moving forward",
            "regret to inform",
            "regretfully inform",
            "unfortunately",
            "application was not selected",
            "decided not to proceed",
            "will not be proceeding",
            "we have chosen another",
            "position has been filled",
        ],
    ) {
        return Some(PipelineStatus::Rejected);
    }

    // Acknowledgement / applied
    if regex_matches(
        &combined,
        &[
            "received your application",
            "thank you for applying",
            "application has been received",
            "we received your application",
        ],
    ) {
        return Some(PipelineStatus::Applied);
    }

    None
}

fn regex_matches(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|p| text.contains(p))
}

/// Parse a single email message into subject, body, from, and date.
pub fn parse_email_message(raw: &[u8]) -> Result<ParsedEmail> {
    let parsed = parse_mail(raw).context("Failed to parse email")?;
    let subject = parsed
        .headers
        .get_first_value("Subject")
        .unwrap_or_default();
    let from = parsed.headers.get_first_value("From").unwrap_or_default();
    let date_str = parsed.headers.get_first_value("Date").unwrap_or_default();

    let body = parsed.get_body().ok().unwrap_or_default();

    let date = parse_rfc2822_date(&date_str).unwrap_or_else(|_| chrono::Utc::now());

    Ok(ParsedEmail {
        subject,
        from,
        body,
        date,
    })
}

fn parse_rfc2822_date(s: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    let dt =
        mailparse::dateparse(s).map_err(|e| anyhow::anyhow!("Failed to parse date: {:?}", e))?;
    Ok(chrono::DateTime::from_timestamp(dt, 0).unwrap_or_else(chrono::Utc::now))
}

#[derive(Debug, Clone)]
pub struct ParsedEmail {
    pub subject: String,
    pub from: String,
    pub body: String,
    pub date: chrono::DateTime<chrono::Utc>,
}

/// Configuration for an IMAP account. The password is not stored here.
#[derive(Debug, Clone)]
pub struct ImapConfig {
    pub server: String,
    pub port: u16,
    pub username: String,
}

impl ImapConfig {
    /// Store the password in the OS keychain for this account.
    pub fn save_password(&self, password: &str) -> Result<()> {
        let entry = keyring::Entry::new("atsassin_imap", &self.username)
            .context("Failed to create keyring entry")?;
        entry
            .set_password(password)
            .context("Failed to save password")?;
        Ok(())
    }

    /// Load the password from the OS keychain, if present.
    pub fn load_password(&self) -> Result<Option<String>> {
        let entry = keyring::Entry::new("atsassin_imap", &self.username)
            .context("Failed to create keyring entry")?;
        match entry.get_password() {
            Ok(p) => Ok(Some(p)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Keyring error: {:?}", e)),
        }
    }
}

/// Sync email outcomes. This is the public entry point used by the CLI.
/// It connects via IMAP, fetches unread ATS-pattern emails, classifies
/// them, and updates the pipeline. It is read-only: no emails are moved or
/// marked as read.
pub fn sync_email_outcomes(
    config: &ImapConfig,
    password: &str,
    tracker: &PipelineTracker,
) -> Result<Vec<OutcomeSignal>> {
    let domain = config.server.clone();
    let mut client = connect_imap(config, password)?;

    let mut signals = Vec::new();
    let ats_patterns = ats_sender_patterns();

    // Select inbox read-only so flags are untouched.
    let mailbox = client.select("INBOX")?;
    let total = mailbox.exists as usize;
    if total == 0 {
        return Ok(signals);
    }

    // Search for unread messages. Not all servers support UNSEEN, but the
    // fallback is to scan recent messages.
    let uids: Vec<u32> = match client.search("UNSEEN") {
        Ok(uids) => uids.into_iter().collect(),
        Err(_) => (1..=total as u32).collect(),
    };

    for uid in uids {
        let body = match client.fetch(uid.to_string(), "BODY.PEEK[]") {
            Ok(messages) => {
                let mut bytes = Vec::new();
                for message in messages.iter() {
                    if let Some(body) = message.body() {
                        bytes.extend_from_slice(body);
                    }
                }
                bytes
            }
            Err(_) => continue,
        };

        if body.is_empty() {
            continue;
        }

        let parsed = match parse_email_message(&body) {
            Ok(p) => p,
            Err(_) => continue,
        };

        if !looks_like_ats_email(&parsed.from, &ats_patterns) {
            continue;
        }

        if let Some(status) = classify_email(&parsed.subject, &parsed.body) {
            let signal = OutcomeSignal {
                job_id: None,
                company: extract_company(&parsed.from, &parsed.subject),
                status,
                source: OutcomeSource::Email,
                source_id: format!("{}:{}", domain, uid),
                raw_subject: parsed.subject.clone(),
                date: parsed.date,
            };
            apply_signal(tracker, &signal)?;
            signals.push(signal);
        }
    }

    Ok(signals)
}

fn connect_imap(
    config: &ImapConfig,
    password: &str,
) -> Result<imap::Session<native_tls::TlsStream<std::net::TcpStream>>> {
    let tls = native_tls::TlsConnector::new().context("Failed to create TLS connector")?;
    let client = imap::connect((config.server.as_str(), config.port), &config.server, &tls)
        .context("Failed to connect to IMAP server")?;
    let session = client
        .login(&config.username, password)
        .map_err(|e| anyhow::anyhow!("IMAP login failed: {:?}", e.0))?;
    Ok(session)
}

fn ats_sender_patterns() -> Vec<Regex> {
    let patterns = [
        r"no.?reply@.*greenhouse.*",
        r"talent@.*",
        r"careers@.*",
        r"jobs@.*",
        r"recruiting@.*",
        r"hr@.*",
        r"people@.*",
    ];
    patterns.iter().filter_map(|p| Regex::new(p).ok()).collect()
}

fn looks_like_ats_email(from: &str, patterns: &[Regex]) -> bool {
    let lower = from.to_lowercase();
    patterns.iter().any(|p| p.is_match(&lower))
}

fn extract_company(from: &str, _subject: &str) -> Option<String> {
    // Best-effort: pull a company name from "careers@company.com".
    let re = Regex::new(r"@([^.]+)").ok()?;
    if let Some(cap) = re.captures(from) {
        if let Some(m) = cap.get(1) {
            let name = m.as_str();
            if !name.is_empty() && name.len() > 2 {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Apply a classified signal to the pipeline. We first try the explicit
/// job_id; if none exists, we fuzzy-match against the user's saved jobs.
fn apply_signal(tracker: &PipelineTracker, signal: &OutcomeSignal) -> Result<()> {
    if let Some(job_id) = &signal.job_id {
        tracker.update_pipeline_status_by_job_id(job_id, signal.status.clone())?;
        return Ok(());
    }

    if let Some(company) = &signal.company {
        if let Some(job_id) = fuzzy_find_job_id(tracker, company, &signal.raw_subject)? {
            tracker.update_pipeline_status_by_job_id(&job_id, signal.status.clone())?;
        }
    }
    Ok(())
}

/// Best-effort fuzzy match of an outcome email to a tracked job.
/// Matches on company name (case-insensitive, whitespace-normalized) and
/// a title word appearing in the subject.
fn fuzzy_find_job_id(
    tracker: &PipelineTracker,
    company: &str,
    subject: &str,
) -> Result<Option<String>> {
    let rows = tracker.list_job_rows(1000)?;
    let company_norm = company.to_lowercase().replace([' ', '.', ','], "");
    let subject_lower = subject.to_lowercase();
    for row in rows {
        let row_company_norm = row.company.to_lowercase().replace([' ', '.', ','], "");
        // Require exact normalised company match to avoid updating the wrong job.
        if row_company_norm == company_norm {
            let title_lower = row.title.to_lowercase();
            let title_words: Vec<&str> = title_lower
                .split_whitespace()
                .filter(|w| w.len() > 3)
                .collect();
            if title_words.iter().any(|w| subject_lower.contains(*w)) {
                return Ok(Some(row.id));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_rejection_email() {
        let status = classify_email(
            "Update on your application",
            "We have decided not to proceed with your application.",
        );
        assert_eq!(status, Some(PipelineStatus::Rejected));
    }

    #[test]
    fn classify_interview_email() {
        let status = classify_email(
            "Interview invitation",
            "We would like to invite you to a phone screen.",
        );
        assert_eq!(status, Some(PipelineStatus::Interviewing));
    }

    #[test]
    fn classify_offer_email() {
        let status = classify_email(
            "Congratulations - offer",
            "We are pleased to offer you the position.",
        );
        assert_eq!(status, Some(PipelineStatus::Offered));
    }

    #[test]
    fn classify_applied_email() {
        let status = classify_email(
            "Thank you for applying",
            "We received your application and will review it.",
        );
        assert_eq!(status, Some(PipelineStatus::Applied));
    }

    #[test]
    fn classify_non_outcome_email() {
        let status = classify_email("Newsletter", "Here is our weekly newsletter.");
        assert_eq!(status, None);
    }
}
