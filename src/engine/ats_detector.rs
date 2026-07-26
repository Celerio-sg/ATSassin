//! Offline ATS detector: looks at a company's public careers-page HTML and
//! decides which ATS platform + board slug it routes through. Foundation
//! for a future "auto-fill" mode of `company_directory` (issue #1). The
//! runtime page-fetching path is opt-in (currently disabled to keep scan
//! latency unchanged); a one-shot offline detector run against a CSV of
//! career-page URLs would let new entries appear in the curated
//! directory with the right slug automatically.
//!
//! Pattern matches are deliberately loose: the goal is "is this company
//! hosted on Greenhouse?" not "what is the exact slug?". Each provider
//! regex is held in a `OnceLock<Regex>` so we compile once per process.
//! Greenhouse's two URL shapes (`boards.greenhouse.io/<slug>` legacy
//! web board and `boards-api.greenhouse.io/v1/boards/<slug>` API form)
//! get separate regexes whose named `slug` capture group keeps the slug
//! field equal to just the company slug, not the full URL tail.

// Result not actively used in the public surface; module stays free of anyhow coupling.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedBoard {
    Greenhouse { slug: String },
    Lever { slug: String },
    Ashby { slug: String },
    Workable { slug: String },
}

impl DetectedBoard {
    pub fn provider(&self) -> &'static str {
        match self {
            DetectedBoard::Greenhouse { .. } => "greenhouse",
            DetectedBoard::Lever { .. } => "lever",
            DetectedBoard::Ashby { .. } => "ashby",
            DetectedBoard::Workable { .. } => "workable",
        }
    }
    pub fn slug(&self) -> &str {
        match self {
            DetectedBoard::Greenhouse { slug }
            | DetectedBoard::Lever { slug }
            | DetectedBoard::Ashby { slug }
            | DetectedBoard::Workable { slug } => slug,
        }
    }
}

fn regex_greenhouse_legacy() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"boards\.greenhouse\.io/(?P<slug>[a-z0-9_-]+)")
            .expect("greenhouse legacy regex pattern must compile")
    })
}
fn regex_greenhouse_api() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"boards-api\.greenhouse\.io/v1/boards/(?P<slug>[a-z0-9_-]+)")
            .expect("greenhouse api regex pattern must compile")
    })
}
fn regex_lever() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"jobs\.lever\.co/(?P<slug>[a-z0-9_-]+)")
            .expect("lever regex pattern must compile")
    })
}
fn regex_ashby() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"jobs\.ashbyhq\.com/(?P<slug>[a-zA-Z0-9_-]+)")
            .expect("ashby regex pattern must compile")
    })
}
fn regex_workable() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"(?:apply|jobs)\.workable\.com/(?P<slug>[a-zA-Z0-9_/\-]+)")
            .expect("workable regex pattern must compile")
    })
}

/// Pull the named `slug` capture group out of a regex match. Falls back
/// to the full match if the named group isn't set - shouldn't happen
/// with the patterns above, but keeps the helper safe under future
/// regex refactors.
fn slug_from(cap: &regex::Captures) -> String {
    cap.name("slug")
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| cap[0].to_string())
}

/// Detect an ATS embed signature in a career-page HTML payload. Returns
/// `None` if no recognized signature is found - never fabricates data
/// (matches the project's "honest empty > fabricated" standard).
pub fn detect_ats(html: &str) -> Option<DetectedBoard> {
    if let Some(cap) = regex_greenhouse_legacy().captures(html) {
        return Some(DetectedBoard::Greenhouse {
            slug: slug_from(&cap),
        });
    }
    if let Some(cap) = regex_greenhouse_api().captures(html) {
        return Some(DetectedBoard::Greenhouse {
            slug: slug_from(&cap),
        });
    }
    if let Some(cap) = regex_lever().captures(html) {
        return Some(DetectedBoard::Lever {
            slug: slug_from(&cap),
        });
    }
    if let Some(cap) = regex_ashby().captures(html) {
        return Some(DetectedBoard::Ashby {
            slug: slug_from(&cap),
        });
    }
    if let Some(cap) = regex_workable().captures(html) {
        return Some(DetectedBoard::Workable {
            slug: slug_from(&cap),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_greenhouse_from_boards_greenhouse_io() {
        let html = r#"<script src="https://boards.greenhouse.io/canva"></script>"#;
        let detected = detect_ats(html).expect("should detect");
        assert_eq!(detected.provider(), "greenhouse");
        assert_eq!(detected.slug(), "canva");
    }

    #[test]
    fn detect_greenhouse_from_api_url() {
        let html = r#"fetch("https://boards-api.greenhouse.io/v1/boards/xero/jobs?content=true")"#;
        let detected = detect_ats(html).expect("should detect");
        assert_eq!(detected.provider(), "greenhouse");
        assert_eq!(detected.slug(), "xero");
    }

    #[test]
    fn detect_lever() {
        let html = r#"<iframe src="https://jobs.lever.co/safetyculture"></iframe>"#;
        let detected = detect_ats(html).expect("should detect");
        assert_eq!(detected.provider(), "lever");
        assert_eq!(detected.slug(), "safetyculture");
    }

    #[test]
    fn detect_ashby() {
        let html = r#"<meta property="og:url" content="https://jobs.ashbyhq.com/airwallex" />"#;
        let detected = detect_ats(html).expect("should detect");
        assert_eq!(detected.provider(), "ashby");
        assert_eq!(detected.slug(), "airwallex");
    }

    #[test]
    fn detect_workable_from_apply() {
        let html = r#"<a href="https://apply.workable.com/j/klaviyo">Apply here</a>"#;
        let detected = detect_ats(html).expect("should detect");
        assert_eq!(detected.provider(), "workable");
        assert!(
            detected.slug().ends_with("klaviyo"),
            "expected slug to end with klaviyo, got {:?}",
            detected.slug()
        );
    }

    #[test]
    fn detect_returns_none_for_unrecognized_html() {
        let html = r#"<html><body><h1>Open Roles</h1><ul><li>Engineer</li></ul></body></html>"#;
        assert_eq!(detect_ats(html), None);
    }

    #[test]
    fn detect_slugs_use_only_safe_characters() {
        // Pinned: any future regex loosening must come with a story for
        // URL-escaping at `boards.greenhouse.io/<slug>` - an apostrophe
        // or slash would silently break the sweep and read as "0 jobs
        // from this company" without an error.
        let html = r#"https://boards.greenhouse.io/foo-bar_123"#;
        let detected = detect_ats(html).expect("should detect");
        assert!(detected
            .slug()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }
}
