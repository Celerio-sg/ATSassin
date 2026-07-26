//! Lightweight discovery of which ATS board a company uses.
//!
//! Issue #1 — the curated `company_directory.rs` lists rot because ATS
//! slugs change. This module fetches a company's public careers page and
//! pattern-matches for known ATS embeds (Greenhouse, Lever, Ashby,
//! Workday). Detected slugs are stored in SQLite (see
//! `PipelineTracker::save_company_boards`) so the `companies` board can
//! fall back to discovered entries when the hand-curated list does not
//! know about a company.

use anyhow::{Context, Result};
use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtsType {
    Greenhouse,
    Lever,
    Ashby,
    Workday,
}

impl AtsType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AtsType::Greenhouse => "greenhouse",
            AtsType::Lever => "lever",
            AtsType::Ashby => "ashby",
            AtsType::Workday => "workday",
        }
    }
}

impl std::str::FromStr for AtsType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "greenhouse" => Ok(AtsType::Greenhouse),
            "lever" => Ok(AtsType::Lever),
            "ashby" => Ok(AtsType::Ashby),
            "workday" => Ok(AtsType::Workday),
            _ => Err(()),
        }
    }
}

/// Result of probing a single company domain.
#[derive(Debug, Clone)]
pub struct DiscoveredBoard {
    pub company: String,
    pub ats_type: AtsType,
    pub slug: String,
    pub source_url: String,
}

/// Build a `reqwest` client with a realistic browser user-agent.
fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.0")
        .build()
        .context("Failed to build HTTP client")
}

/// Try to fetch the careers page for a domain and extract any known ATS
/// slug. We probe a few common paths because there is no single standard.
pub async fn discover_domain(company: &str, domain: &str) -> Result<Option<DiscoveredBoard>> {
    let client = http_client()?;
    let paths = ["/careers", "/jobs", "/about/careers", "/join", "/career"];

    for path in &paths {
        let url = format!("https://{}{}", domain, path);
        if let Some(board) = discover_at_url(company, &url, &client).await? {
            return Ok(Some(board));
        }
    }

    Ok(None)
}

async fn discover_at_url(
    company: &str,
    url: &str,
    client: &reqwest::Client,
) -> Result<Option<DiscoveredBoard>> {
    let response = client.get(url).send().await;
    if response.is_err() {
        return Ok(None);
    }
    let response = response.unwrap();
    if !response.status().is_success() {
        return Ok(None);
    }
    let text = response.text().await.unwrap_or_default();
    if let Some((ats_type, slug)) = detect_ats(&text) {
        return Ok(Some(DiscoveredBoard {
            company: company.to_string(),
            ats_type,
            slug,
            source_url: url.to_string(),
        }));
    }
    Ok(None)
}

/// Detect the ATS type and slug from raw HTML.
fn detect_ats(html: &str) -> Option<(AtsType, String)> {
    // Greenhouse: boards.greenhouse.io/<slug> or api.greenhouse.io/v1/boards/<slug>
    let re_greenhouse = Regex::new(r#"boards\.greenhouse\.io/(?P<slug>[^/"'\s\\]+)"#).unwrap();
    if let Some(cap) = re_greenhouse.captures(html) {
        let slug = cap["slug"].to_string();
        return Some((AtsType::Greenhouse, slug));
    }

    // Lever: jobs.lever.co/<slug>
    let re_lever = Regex::new(r#"jobs\.lever\.co/(?P<slug>[^/"'\s\\]+)"#).unwrap();
    if let Some(cap) = re_lever.captures(html) {
        let slug = cap["slug"].to_string();
        return Some((AtsType::Lever, slug));
    }

    // Ashby: jobs.ashbyhq.com/<slug>
    let re_ashby = Regex::new(r#"jobs\.ashbyhq\.com/(?P<slug>[^/"'\s\\]+)"#).unwrap();
    if let Some(cap) = re_ashby.captures(html) {
        let slug = cap["slug"].to_string();
        return Some((AtsType::Ashby, slug));
    }

    // Workday: many forms, e.g. <company>.wd101.myworkdayjobs.com
    let re_workday = Regex::new(r#"(?P<slug>[a-zA-Z0-9-]+)\.myworkdayjobs\.com"#).unwrap();
    if let Some(cap) = re_workday.captures(html) {
        let slug = cap["slug"].to_string();
        return Some((AtsType::Workday, slug));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_greenhouse_from_embed_url() {
        let html = r#"<a href=\"https://boards.greenhouse.io/acme/jobs/123\">Apply</a>"#;
        let (ats, slug) = detect_ats(html).expect("should detect greenhouse");
        assert_eq!(ats, AtsType::Greenhouse);
        assert_eq!(slug, "acme");
    }

    #[test]
    fn detect_lever_from_embed_url() {
        let html = r#"<script src=\"https://jobs.lever.co/apex/embed\" async></script>"#;
        let (ats, slug) = detect_ats(html).expect("should detect lever");
        assert_eq!(ats, AtsType::Lever);
        assert_eq!(slug, "apex");
    }

    #[test]
    fn detect_ashby_from_embed_url() {
        let html = r#"<div data-src=\"https://jobs.ashbyhq.com/widgetron\">"#;
        let (ats, slug) = detect_ats(html).expect("should detect ashby");
        assert_eq!(ats, AtsType::Ashby);
        assert_eq!(slug, "widgetron");
    }

    #[test]
    fn detect_workday_from_embed_url() {
        let html = r#"<link href=\"https://acme.myworkdayjobs.com/en-US/careers\""#;
        let (ats, slug) = detect_ats(html).expect("should detect workday");
        assert_eq!(ats, AtsType::Workday);
        assert_eq!(slug, "acme");
    }

    #[test]
    fn detect_returns_none_for_plain_html() {
        let html = r#"<html><body>We are hiring!</body></html>"#;
        assert!(detect_ats(html).is_none());
    }
}
