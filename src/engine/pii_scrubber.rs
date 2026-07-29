//! PII scrubbing for LoRA training pairs.
//!
//! Before any training data is shared via the community LoRA registry,
//! all personally identifiable information must be removed. This module
//! provides deterministic scrubbing of common PII patterns from text.
//! It is deliberately not a universal named-entity recognizer: unsupported
//! free-text identity must remain behind the fail-closed egress boundary.
//!
//! This is a **blocking prerequisite** for any LoRA sharing feature
//! (CRITICAL_CHAIN_PLAN.md issue #46). Sharing a distilled model that
//! memorizes PII would be project-killing.

use crate::models::profile::UserProfile;
use regex::Regex;
use std::collections::HashSet;

lazy_static::lazy_static! {
    // Email pattern: matches most common email formats
    static ref EMAIL_RE: Regex = Regex::new(
        r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"
    ).expect("email regex must compile");

    // Extract broadly, then validate digit count and shape in
    // `looks_like_phone`. Horizontal whitespace is intentional: candidates
    // must never consume multiple JSONL lines.
    static ref PHONE_CANDIDATE_RE: Regex = Regex::new(
        r"(?:\+|00|\()?\d(?:[\d().\- \t]{5,}\d)"
    ).expect("phone candidate regex must compile");

    static ref DATE_LIKE_RE: Regex = Regex::new(
        r"^(?:\d{4}[-/.]\d{1,2}[-/.]\d{1,2}|\d{1,2}[-/.]\d{1,2}[-/.]\d{2,4})$"
    ).expect("date-like regex must compile");

    // Common company name patterns (capitalized words followed by Inc/Ltd/LLC/etc)
    static ref COMPANY_RE: Regex = Regex::new(
        r"[A-Z][a-zA-Z0-9]+(?:\s+[A-Z][a-zA-Z0-9]+)*(?:\s+(?:Inc\.?|Ltd\.?|LLC|Corp\.?|GmbH|Pty\.?|S\.A\.|B\.V\.))\.?"
    ).expect("company regex must compile");

    static ref SINGAPORE_ADDRESS_RE: Regex = Regex::new(
        r"(?ix)\b(?:blk\s+)?\d{1,4}[A-Z]?\s+(?:[\p{L}\d][\p{L}\d.'-]*\s+){1,8}(?:road|rd|street|st|avenue|ave|drive|dr|lane|ln|crescent|close|walk|link|view|terrace)(?:\s+\d{1,3})?(?:\s*,\s*\#\d{1,3}-\d{1,4})?\s*,\s*singapore\s+\d{6}\b"
    ).expect("Singapore address regex must compile");

    static ref UK_ADDRESS_RE: Regex = Regex::new(
        r"(?ix)\b\d{1,5}[A-Z]?\s+(?:[\p{L}\d][\p{L}\d.'-]*\s+){1,8}(?:street|road|avenue|lane|drive|close|way|place|crescent|terrace)\s*,\s*(?:[\p{L}][\p{L}.'-]*\s+){0,5}[A-Z]{1,2}\d[A-Z\d]?\s*\d[A-Z]{2}\b"
    ).expect("UK address regex must compile");

    static ref INDIA_ADDRESS_RE: Regex = Regex::new(
        r"(?ix)\b\d{1,5}[A-Z]?\s+(?:[\p{L}\d][\p{L}\d.'-]*\s+){1,8}(?:road|rd|street|st|avenue|ave|lane|drive|marg|nagar|layout)\s*,\s*(?:[\p{L}][\p{L}.'-]*\s*){1,5}\s*,\s*(?:[\p{L}][\p{L}.'-]*\s*){1,5}\s+\d{6}\b"
    ).expect("India address regex must compile");

    static ref EU_ADDRESS_RE: Regex = Regex::new(
        r"\b\p{Lu}[\p{L}.'-]*(?:\s+[\p{L}][\p{L}.'-]*){0,7}\s+\d{1,5}[A-Za-z]?,\s*\d{4,5}\s+\p{Lu}[\p{L}.'-]*(?:\s+\p{Lu}[\p{L}.'-]*){0,2}\b"
    ).expect("EU address regex must compile");

    static ref LEADING_NUMBER_ADDRESS_RE: Regex = Regex::new(
        r"(?ix)\b\d{1,6}[A-Z]?\s+(?:[\p{L}\d][\p{L}\d.'-]*\s+){1,8}?(?:street|st|avenue|ave|road|rd|boulevard|blvd|lane|ln|drive|dr|court|ct|way|place|pl|crescent|close|walk|terrace|highway|hwy|marg|nagar|layout|strasse|straße|rue|via|calle)\b"
    ).expect("leading-number address regex must compile");

    static ref NRIC_FIN_RE: Regex = Regex::new(
        r"(?i)\b[STFGM]\d{7}[A-Z]\b"
    ).expect("NRIC/FIN regex must compile");

    static ref US_SSN_RE: Regex = Regex::new(
        r"\b\d{3}-\d{2}-\d{4}\b"
    ).expect("US SSN regex must compile");

    static ref LABELLED_ID_RE: Regex = Regex::new(
        r"(?i)\b(?:nric|fin|passport|national\s+id|identity\s+card|aadhaar|aadhar|pan|social\s+security|ssn|national\s+insurance|ni\s+number|tax\s+id)\s*(?:number|no\.?|#)?\s*[:\-]?\s*[A-Z0-9][A-Z0-9 -]{3,22}[A-Z0-9]\b"
    ).expect("labelled identity regex must compile");

    static ref DOB_RE: Regex = Regex::new(
        r"(?i)\b(?:date\s+of\s+birth|dob|born\s+on)\s*[:\-]\s*(?:\d{1,4}[-/.]\d{1,2}[-/.]\d{1,4}|[A-Z][a-z]+\s+\d{1,2},?\s+\d{4})\b"
    ).expect("date-of-birth regex must compile");

    static ref SOCIAL_HANDLE_RE: Regex = Regex::new(
        r#"(?m)(?:^|[\s:,("])@[A-Za-z0-9_][A-Za-z0-9_.-]{1,29}\b"#
    ).expect("social handle regex must compile");

    static ref EXPLICIT_NAME_RE: Regex = Regex::new(
        r"(?im)^\s*name\s*[:\-]\s*(\S.*?)\s*$"
    ).expect("explicit name regex must compile");
}

/// PII scrubbing result with statistics
#[derive(Debug, Clone)]
pub struct ScrubResult {
    /// The scrubbed text
    pub text: String,
    /// Number of emails removed
    pub emails_removed: usize,
    /// Number of phone numbers removed
    pub phones_removed: usize,
    /// Number of company names removed
    pub companies_removed: usize,
    /// Number of addresses removed
    pub addresses_removed: usize,
    /// Number of candidate identity terms removed
    pub identity_terms_removed: usize,
    /// Number of national IDs, passport values, or DOB fields removed
    pub identifiers_removed: usize,
    /// Number of standalone social handles removed
    pub social_handles_removed: usize,
}

/// Identity context used to make free-text redaction candidate-specific.
#[derive(Debug, Clone, Default)]
pub struct ScrubContext {
    /// Company names that should NOT be scrubbed (e.g., target companies)
    pub preserve_companies: HashSet<String>,
    identity_terms: HashSet<String>,
    identity_anchor_present: bool,
}

impl ScrubContext {
    pub fn from_profile(profile: &UserProfile) -> Self {
        let mut context = Self::default();
        if let Some(explicit_name) = EXPLICIT_NAME_RE
            .captures(&profile.raw_text)
            .and_then(|captures| captures.get(1))
            .map(|value| value.as_str().trim())
            .filter(|value| value.eq_ignore_ascii_case(profile.name.trim()))
        {
            context.identity_anchor_present = context.insert_identity_term(explicit_name);
        }
        for value in [
            profile.email.as_deref(),
            profile.phone.as_deref(),
            profile.location.as_deref(),
            profile.linkedin_url.as_deref(),
            profile.portfolio_url.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            context.insert_identity_term(value);
        }
        for experience in &profile.experience {
            context.insert_identity_term(&experience.company);
        }
        for education in &profile.education {
            context.insert_identity_term(&education.institution);
        }
        context
    }

    #[cfg(test)]
    pub(crate) fn add_identity_term(&mut self, value: impl AsRef<str>) {
        if self.insert_identity_term(value) {
            self.identity_anchor_present = true;
        }
    }

    fn insert_identity_term(&mut self, value: impl AsRef<str>) -> bool {
        let value = value.as_ref().trim();
        let lower = value.to_lowercase();
        if value.chars().count() >= 3
            && !matches!(
                lower.as_str(),
                "unknown" | "not provided" | "n/a" | "none" | "remote"
            )
        {
            self.identity_terms.insert(value.to_string());
            true
        } else {
            false
        }
    }

    pub fn has_identity_context(&self) -> bool {
        self.identity_anchor_present
    }
}

/// Scrub PII from text
pub fn scrub_text(text: &str, context: &ScrubContext) -> ScrubResult {
    let mut result = text.to_string();
    let mut emails_removed = 0;
    let mut phones_removed = 0;
    let mut companies_removed = 0;
    let mut addresses_removed = 0;
    let mut identity_terms_removed = 0;
    let mut identifiers_removed = 0;
    let mut social_handles_removed = 0;

    let mut identity_terms: Vec<_> = context.identity_terms.iter().collect();
    identity_terms.sort_by_key(|term| std::cmp::Reverse(term.chars().count()));
    for term in identity_terms {
        let pattern = identity_pattern(term);
        let count = pattern.find_iter(&result).count();
        if count > 0 {
            result = pattern.replace_all(&result, "[IDENTITY]").into_owned();
            identity_terms_removed += count;
        }
    }

    // Scrub emails
    let emails: Vec<String> = EMAIL_RE
        .find_iter(&result)
        .map(|m| m.as_str().to_string())
        .collect();
    for email in emails {
        if !context.preserve_companies.contains(&email) {
            result = result.replace(&email, "[EMAIL]");
            emails_removed += 1;
        }
    }

    let phones = detected_phone_candidates(&result);
    for phone in phones {
        result = result.replace(&phone, "[PHONE]");
        phones_removed += 1;
    }

    for pattern in [&*NRIC_FIN_RE, &*US_SSN_RE, &*DOB_RE] {
        let count = pattern.find_iter(&result).count();
        if count > 0 {
            result = pattern.replace_all(&result, "[IDENTIFIER]").into_owned();
            identifiers_removed += count;
        }
    }
    let labelled_ids: Vec<String> = LABELLED_ID_RE
        .find_iter(&result)
        .map(|match_| match_.as_str().to_string())
        .filter(|candidate| looks_like_labelled_id(candidate))
        .collect();
    for labelled_id in labelled_ids {
        result = result.replace(&labelled_id, "[IDENTIFIER]");
        identifiers_removed += 1;
    }

    let handles: Vec<String> = SOCIAL_HANDLE_RE
        .find_iter(&result)
        .map(|match_| match_.as_str().to_string())
        .filter(|candidate| is_social_handle(candidate))
        .collect();
    for handle in handles {
        let prefix = handle
            .chars()
            .next()
            .filter(|character| *character != '@')
            .map(|character| character.to_string())
            .unwrap_or_default();
        result = result.replace(&handle, &format!("{prefix}[SOCIAL_HANDLE]"));
        social_handles_removed += 1;
    }

    // Scrub company names (unless preserved)
    let companies: Vec<String> = COMPANY_RE
        .find_iter(&result)
        .map(|m| m.as_str().to_string())
        .collect();
    for company in companies {
        if !context.preserve_companies.contains(&company) {
            result = result.replace(&company, "[COMPANY]");
            companies_removed += 1;
        }
    }

    for pattern in address_patterns() {
        let count = pattern.find_iter(&result).count();
        if count > 0 {
            result = pattern.replace_all(&result, "[ADDRESS]").into_owned();
            addresses_removed += count;
        }
    }

    ScrubResult {
        text: result,
        emails_removed,
        phones_removed,
        companies_removed,
        addresses_removed,
        identity_terms_removed,
        identifiers_removed,
        social_handles_removed,
    }
}

/// Check if text contains any PII patterns
pub fn contains_pii(text: &str, context: &ScrubContext) -> bool {
    for term in &context.identity_terms {
        let pattern = identity_pattern(term);
        if pattern.is_match(text) {
            return true;
        }
    }
    if EMAIL_RE.is_match(text) {
        for email in EMAIL_RE.find_iter(text) {
            if !context.preserve_companies.contains(email.as_str()) {
                return true;
            }
        }
    }
    if !detected_phone_candidates(text).is_empty() {
        return true;
    }
    if NRIC_FIN_RE.is_match(text)
        || US_SSN_RE.is_match(text)
        || LABELLED_ID_RE
            .find_iter(text)
            .any(|candidate| looks_like_labelled_id(candidate.as_str()))
        || DOB_RE.is_match(text)
        || SOCIAL_HANDLE_RE
            .find_iter(text)
            .any(|candidate| is_social_handle(candidate.as_str()))
    {
        return true;
    }
    if COMPANY_RE.is_match(text) {
        for company in COMPANY_RE.find_iter(text) {
            if !context.preserve_companies.contains(company.as_str()) {
                return true;
            }
        }
    }
    if address_patterns()
        .iter()
        .any(|pattern| pattern.is_match(text))
    {
        return true;
    }
    false
}

fn address_patterns() -> [&'static Regex; 5] {
    [
        &SINGAPORE_ADDRESS_RE,
        &UK_ADDRESS_RE,
        &INDIA_ADDRESS_RE,
        &EU_ADDRESS_RE,
        &LEADING_NUMBER_ADDRESS_RE,
    ]
}

fn looks_like_phone(candidate: &str) -> bool {
    if DATE_LIKE_RE.is_match(candidate) || US_SSN_RE.is_match(candidate) {
        return false;
    }
    let digit_count = candidate
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count();
    if !(8..=15).contains(&digit_count) {
        return false;
    }
    let punctuation_count = candidate
        .chars()
        .filter(|character| !character.is_ascii_digit() && !character.is_ascii_whitespace())
        .count();
    candidate.starts_with('+')
        || candidate.starts_with("00")
        || candidate.starts_with('(')
        || digit_count == 10
        || punctuation_count >= 2
}

fn detected_phone_candidates(text: &str) -> Vec<String> {
    PHONE_CANDIDATE_RE
        .find_iter(text)
        .flat_map(|candidate| split_phone_candidate(candidate.as_str()))
        .filter(|candidate| looks_like_phone(candidate))
        .collect()
}

fn split_phone_candidate(candidate: &str) -> Vec<String> {
    if candidate
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count()
        <= 15
    {
        return vec![candidate.to_string()];
    }

    for (index, character) in candidate.char_indices() {
        if !character.is_ascii_whitespace() {
            continue;
        }
        let right_index = candidate[index..]
            .find(|next: char| !next.is_ascii_whitespace())
            .map(|offset| index + offset)
            .unwrap_or(candidate.len());
        if right_index >= candidate.len() {
            continue;
        }
        let left = candidate[..index].trim();
        let right = candidate[right_index..].trim();
        if !looks_like_phone(left) {
            continue;
        }
        let right_segments = split_phone_candidate(right);
        if right_segments
            .iter()
            .all(|segment| looks_like_phone(segment))
        {
            let mut segments = vec![left.to_string()];
            segments.extend(right_segments);
            return segments;
        }
    }

    vec![candidate.to_string()]
}

fn is_social_handle(candidate: &str) -> bool {
    let handle = candidate
        .trim_start_matches(|character: char| character != '@')
        .trim_start_matches('@');
    !matches!(
        handle.to_ascii_lowercase().as_str(),
        "media"
            | "font-face"
            | "keyframes"
            | "supports"
            | "import"
            | "page"
            | "charset"
            | "namespace"
    )
}

fn looks_like_labelled_id(candidate: &str) -> bool {
    candidate
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count()
        >= 3
}

fn identity_pattern(term: &str) -> Regex {
    let leading_boundary = term
        .chars()
        .next()
        .is_some_and(|character| character.is_alphanumeric() || character == '_');
    let trailing_boundary = term
        .chars()
        .next_back()
        .is_some_and(|character| character.is_alphanumeric() || character == '_');
    Regex::new(&format!(
        "(?iu){}{}{}",
        if leading_boundary { r"\b" } else { "" },
        regex::escape(term),
        if trailing_boundary { r"\b" } else { "" }
    ))
    .expect("escaped identity term must compile")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrub_email() {
        let context = ScrubContext::default();
        let result = scrub_text(
            "Contact me at john@example.com or jane@company.org",
            &context,
        );
        assert_eq!(result.text, "Contact me at [EMAIL] or [EMAIL]");
        assert_eq!(result.emails_removed, 2);
    }

    #[test]
    fn scrub_phone_numbers() {
        let context = ScrubContext::default();
        let result = scrub_text("Call me at 555-123-4567 or (555) 987-6543", &context);
        assert_eq!(result.text, "Call me at [PHONE] or [PHONE]");
        assert_eq!(result.phones_removed, 2);
    }

    #[test]
    fn scrub_regional_phone_formats() {
        let context = ScrubContext::default();
        let fixtures = [
            ("SG", "+65 9123 4567"),
            ("UK", "+44 20 7946 0958"),
            ("India", "+91 98765 43210"),
            ("EU", "+49 30 901820"),
            ("US", "+1 (415) 555-2671"),
        ];

        for (region, phone) in fixtures {
            let result = scrub_text(&format!("Contact: {phone}"), &context);
            assert_eq!(
                result.text, "Contact: [PHONE]",
                "{region} fixture was not fully scrubbed"
            );
            assert_eq!(result.phones_removed, 1);
            assert!(!contains_pii(&result.text, &context));
        }
    }

    #[test]
    fn adjacent_phone_numbers_are_split_and_scrubbed() {
        let context = ScrubContext::default();
        let result = scrub_text("Phones: 555-123-4567 (212) 555-1212", &context);

        assert_eq!(result.text, "Phones: [PHONE] [PHONE]");
        assert_eq!(result.phones_removed, 2);
        assert!(!contains_pii(&result.text, &context));
    }

    #[test]
    fn scrub_company_names() {
        let context = ScrubContext::default();
        let result = scrub_text(
            "I worked at Acme Inc. and then at Global Corp LLC",
            &context,
        );
        assert_eq!(result.text, "I worked at [COMPANY] and then at [COMPANY]");
        assert_eq!(result.companies_removed, 2);
    }

    #[test]
    fn preserve_company_names() {
        let mut context = ScrubContext::default();
        context.preserve_companies.insert("Acme Inc.".to_string());
        let result = scrub_text(
            "I worked at Acme Inc. and then at Global Corp LLC",
            &context,
        );
        assert_eq!(result.text, "I worked at Acme Inc. and then at [COMPANY]");
        assert_eq!(result.companies_removed, 1);
    }

    #[test]
    fn scrub_addresses() {
        let context = ScrubContext::default();
        let result = scrub_text("I lived at 123 Main Street and 456 Oak Avenue", &context);
        assert_eq!(result.text, "I lived at [ADDRESS] and [ADDRESS]");
        assert_eq!(result.addresses_removed, 2);
    }

    #[test]
    fn scrub_regional_address_formats() {
        let context = ScrubContext::default();
        let fixtures = [
            ("SG", "Blk 123 Ang Mo Kio Ave 3, #12-01, Singapore 560123"),
            ("UK", "10 Downing Street, London SW1A 2AA"),
            ("India", "12 MG Road, Bengaluru, Karnataka 560001"),
            ("EU", "Unter den Linden 77, 10117 Berlin"),
            ("US", "1600 Pennsylvania Avenue"),
        ];

        for (region, address) in fixtures {
            let result = scrub_text(&format!("Address: {address}"), &context);
            assert_eq!(
                result.text, "Address: [ADDRESS]",
                "{region} fixture was not fully scrubbed"
            );
            assert_eq!(result.addresses_removed, 1);
            assert!(!contains_pii(&result.text, &context));
        }
    }

    #[test]
    fn scrub_identity_documents_dob_and_social_handles() {
        let context = ScrubContext::default();
        for fixture in [
            "NRIC S1234567D",
            "Passport: K1234567",
            "Aadhaar: 1234 5678 9012",
            "SSN: 123-45-6789",
            "DOB: 31/12/1980",
        ] {
            let result = scrub_text(fixture, &context);
            assert!(
                result.text.contains("[IDENTIFIER]"),
                "{fixture} was not scrubbed"
            );
            assert!(!contains_pii(&result.text, &context));
        }

        let result = scrub_text(
            "Social: @safe_fixture; email: fixture@example.com",
            &context,
        );
        assert_eq!(result.social_handles_removed, 1);
        assert_eq!(result.emails_removed, 1);
        assert!(!result.text.contains("@safe_fixture"));
        assert!(result.text.contains("[EMAIL]"));
        assert!(!contains_pii(&result.text, &context));
        assert!(contains_pii(r#"{"input":"@safe_fixture"}"#, &context));
    }

    #[test]
    fn deterministic_detectors_preserve_false_positive_fixtures() {
        let context = ScrubContext::default();
        let text = "Dates 2020-01-01 and 31/12/2024; build 12345678; version 1.2.3; Passport required for travel; led 12 roadmap initiatives; CSS @media query.";

        let result = scrub_text(text, &context);

        assert_eq!(result.text, text);
        assert_eq!(result.phones_removed, 0);
        assert_eq!(result.addresses_removed, 0);
        assert_eq!(result.identifiers_removed, 0);
        assert_eq!(result.social_handles_removed, 0);
        assert!(!contains_pii(text, &context));
    }

    #[test]
    fn contains_pii_detection() {
        let context = ScrubContext::default();
        assert!(contains_pii("john@example.com", &context));
        assert!(contains_pii("555-123-4567", &context));
        assert!(contains_pii("Acme Inc.", &context));
        assert!(contains_pii("123 Main Street", &context));
        assert!(!contains_pii("No PII here", &context));
    }

    #[test]
    fn scrubbed_text_passes_pii_check() {
        let context = ScrubContext::default();
        let result = scrub_text("Contact john@example.com at 555-123-4567", &context);
        assert!(!contains_pii(&result.text, &context));
    }

    #[test]
    fn complex_profile_scrubbing() {
        let mut context = ScrubContext::default();
        context.add_identity_term("John Doe");
        let profile = r#"
        John Doe
        Email: john.doe@example.com
        Phone: (555) 123-4567
        Previous: Acme Inc. (2018-2020), Global Corp LLC (2020-2023)
        Address: 123 Innovation Drive, Tech City
        "#;
        let result = scrub_text(profile, &context);
        assert!(result.emails_removed > 0);
        assert!(result.phones_removed > 0);
        assert!(result.companies_removed > 0);
        assert!(result.addresses_removed > 0);
        assert!(result.identity_terms_removed > 0);
        assert!(!contains_pii(&result.text, &context));
    }

    #[test]
    fn scrub_unicode_identity_terms_case_insensitively() {
        let mut context = ScrubContext::default();
        context.add_identity_term("Renée Chén");

        let result = scrub_text("Candidate: RENÉE CHÉN", &context);

        assert_eq!(result.text, "Candidate: [IDENTITY]");
        assert_eq!(result.identity_terms_removed, 1);
        assert!(!contains_pii(&result.text, &context));
    }

    #[test]
    fn identity_terms_do_not_match_inside_unrelated_words() {
        let mut context = ScrubContext::default();
        context.add_identity_term("Ann");

        let result = scrub_text("Planning applications", &context);

        assert_eq!(result.text, "Planning applications");
        assert_eq!(result.identity_terms_removed, 0);
    }

    #[test]
    fn profile_context_redacts_candidate_derived_values() -> anyhow::Result<()> {
        let profile = crate::engine::profile_parser::ProfileParser::profile_from_text(
            "Name: Renée Chén\nLocation: Singapore",
        )?;
        let context = ScrubContext::from_profile(&profile);

        let result = scrub_text("RENÉE CHÉN is based in singapore", &context);

        assert!(!result.text.contains("RENÉE CHÉN"));
        assert!(!result.text.contains("singapore"));
        assert!(context.has_identity_context());
        Ok(())
    }

    #[test]
    fn profile_context_redacts_non_latin_unicode_name() -> anyhow::Result<()> {
        let profile = crate::engine::profile_parser::ProfileParser::profile_from_text(
            "Name: 李小龙\nLocation: 新加坡",
        )?;
        let context = ScrubContext::from_profile(&profile);

        let result = scrub_text("Candidate 李小龙 is based in 新加坡", &context);

        assert_eq!(result.text, "Candidate [IDENTITY] is based in [IDENTITY]");
        assert!(!contains_pii(&result.text, &context));
        Ok(())
    }

    #[test]
    fn generic_markdown_heading_is_not_an_identity_anchor() -> anyhow::Result<()> {
        let profile = crate::engine::profile_parser::ProfileParser::profile_from_text(
            "# Resume\nJane Example\nLocation: Singapore",
        )?;
        let context = ScrubContext::from_profile(&profile);

        assert_eq!(profile.name, "# Resume");
        assert!(!context.has_identity_context());
        Ok(())
    }

    #[test]
    fn preserve_target_companies() {
        let mut context = ScrubContext::default();
        context.preserve_companies.insert("Anthropic".to_string());
        context.preserve_companies.insert("OpenAI".to_string());
        let text = "I'm applying to Anthropic and OpenAI, but previously worked at Other Corp";
        let result = scrub_text(text, &context);
        assert!(result.text.contains("Anthropic"));
        assert!(result.text.contains("OpenAI"));
        assert!(result.text.contains("[COMPANY]"));
    }
}
