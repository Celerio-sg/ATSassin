//! PII scrubbing for LoRA training pairs.
//!
//! Before any training data is shared via the community LoRA registry,
//! all personally identifiable information must be removed. This module
//! provides deterministic scrubbing of common PII patterns from text.
//!
//! This is a **blocking prerequisite** for any LoRA sharing feature
//! (CRITICAL_CHAIN_PLAN.md issue #46). Sharing a distilled model that
//! memorizes PII would be project-killing.

use regex::Regex;
use std::collections::HashSet;

lazy_static::lazy_static! {
    // Email pattern: matches most common email formats
    static ref EMAIL_RE: Regex = Regex::new(
        r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"
    ).expect("email regex must compile");

    // Phone number patterns (international and US formats)
    static ref PHONE_RE: Regex = Regex::new(
        r"\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}"
    ).expect("phone regex must compile");

    // Common company name patterns (capitalized words followed by Inc/Ltd/LLC/etc)
    static ref COMPANY_RE: Regex = Regex::new(
        r"[A-Z][a-zA-Z0-9]+(?:\s+[A-Z][a-zA-Z0-9]+)*(?:\s+(?:Inc\.?|Ltd\.?|LLC|Corp\.?|GmbH|Pty\.?|S\.A\.|B\.V\.))\.?"
    ).expect("company regex must compile");

    // Address patterns (street addresses)
    static ref ADDRESS_RE: Regex = Regex::new(
        r"\d+\s+[A-Z][a-zA-Z0-9\s,]+?(?:Street|St|Avenue|Ave|Road|Rd|Boulevard|Blvd|Lane|Ln|Drive|Dr|Court|Ct|Way|Place|Pl)"
    ).expect("address regex must compile");
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
}

/// Additional context to preserve certain entities
#[derive(Debug, Clone, Default)]
pub struct ScrubContext {
    /// Company names that should NOT be scrubbed (e.g., target companies)
    pub preserve_companies: HashSet<String>,
    /// Names that should NOT be scrubbed (e.g., common first names in context)
    pub preserve_names: HashSet<String>,
}

/// Scrub PII from text
pub fn scrub_text(text: &str, context: &ScrubContext) -> ScrubResult {
    let mut result = text.to_string();
    let mut emails_removed = 0;
    let mut phones_removed = 0;
    let mut companies_removed = 0;
    let mut addresses_removed = 0;

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

    // Scrub phone numbers
    let phones: Vec<String> = PHONE_RE
        .find_iter(&result)
        .map(|m| m.as_str().to_string())
        .collect();
    for phone in phones {
        result = result.replace(&phone, "[PHONE]");
        phones_removed += 1;
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

    // Scrub addresses
    let addresses: Vec<String> = ADDRESS_RE
        .find_iter(&result)
        .map(|m| m.as_str().to_string())
        .collect();
    for address in addresses {
        result = result.replace(&address, "[ADDRESS]");
        addresses_removed += 1;
    }

    ScrubResult {
        text: result,
        emails_removed,
        phones_removed,
        companies_removed,
        addresses_removed,
    }
}

/// Check if text contains any PII patterns
pub fn contains_pii(text: &str, context: &ScrubContext) -> bool {
    if EMAIL_RE.is_match(text) {
        for email in EMAIL_RE.find_iter(text) {
            if !context.preserve_companies.contains(email.as_str()) {
                return true;
            }
        }
    }
    if PHONE_RE.is_match(text) {
        return true;
    }
    if COMPANY_RE.is_match(text) {
        for company in COMPANY_RE.find_iter(text) {
            if !context.preserve_companies.contains(company.as_str()) {
                return true;
            }
        }
    }
    if ADDRESS_RE.is_match(text) {
        return true;
    }
    false
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
        let context = ScrubContext::default();
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
        assert!(!contains_pii(&result.text, &context));
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
