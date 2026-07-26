//! Phase 4 — assisted browser form-filling.
//!
//! This module does not drive a headless browser or submit forms
//! automatically. Instead, it produces a small JavaScript payload that the
//! user can paste into the browser console (or save as a bookmarklet) to
//! fill common application fields with their profile and the tailored
//! resume/cover letter generated for a specific job.

use crate::models::job::Application;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

/// Profile fields that the generated script knows how to map.
///
/// NOTE: This is a best-effort structure. Only `resume_text`,
/// `cover_text`, and `summary` are populated from saved application
/// materials; email/phone/linkedin/etc. must be filled by the user in the
/// browser after pasting the generated script.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ApplyProfile {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub linkedin: String,
    pub portfolio: String,
    pub location: String,
    pub summary: String,
    pub resume_text: String,
    pub cover_text: String,
}

/// Generate a JavaScript payload that fills fields on the current page.
/// The script is conservative: it never clicks submit buttons.
pub fn generate_fill_script(profile: &ApplyProfile) -> String {
    let json = serde_json::to_string(profile).unwrap_or_default();
    format!(
        r#"(function(){{
  const p = {json};
  function fill(selector, value) {{
    if (!value) return;
    const el = document.querySelector(selector);
    if (!el) return;
    el.value = value;
    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
  }}
  function fillMany(selectors, value) {{
    for (const s of selectors) {{ fill(s, value); }}
  }}
  fillMany(['input[name*="full" i][name*=\"name\" i]', 'input[name=\"name\"]', 'input[id*=\"name\" i]'], p.name);
  fillMany(['input[type=\"email\"]', 'input[name*=\"email\" i]', 'input[id*=\"email\" i]'], p.email);
  fillMany(['input[type=\"tel\"]', 'input[name*=\"phone\" i]', 'input[id*=\"phone\" i]'], p.phone);
  fillMany(['input[name*=\"linkedin\" i]', 'input[id*=\"linkedin\" i]'], p.linkedin);
  fillMany(['input[name*=\"portfolio\" i]', 'input[id*=\"portfolio\" i]', 'input[name*=\"website\" i]'], p.portfolio);
  fillMany(['input[name*=\"location\" i]', 'input[id*=\"location\" i]'], p.location);
  fillMany(['textarea[name*=\"resume\" i]', 'textarea[id*=\"resume\" i]', 'textarea[name*=\"cover\" i]'], p.resume_text);
  fillMany(['textarea[name*=\"cover\" i]', 'textarea[id*=\"cover\" i]', 'textarea[name*=\"letter\" i]'], p.cover_text);
  console.log('[ATSassin] Filled application fields. Review before submitting.');
}})();"#
    )
}

/// Write a bookmarklet and a plain JS file to the output directory.
pub fn write_apply_kit(profile: &ApplyProfile, output_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;
    let script = generate_fill_script(profile);
    std::fs::write(output_dir.join("apply.js"), &script)?;

    let minified = script.split_whitespace().collect::<String>();
    let bookmarklet = format!("javascript:{}", minified);
    std::fs::write(output_dir.join("bookmarklet.txt"), bookmarklet)?;

    Ok(())
}

/// Build an ApplyProfile from the user's saved profile and application.
pub fn profile_from_application(profile_text: &str, application: &Application) -> ApplyProfile {
    // Best-effort extraction: the profile.md format is free text, so we
    // simply echo it as the summary/resume and rely on the user to review.
    ApplyProfile {
        resume_text: application.resume_text.clone(),
        cover_text: application.cover_letter_text.clone(),
        summary: profile_text.to_string(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_script_contains_profile_data() {
        let profile = ApplyProfile {
            name: "Ada Lovelace".into(),
            email: "ada@example.com".into(),
            ..Default::default()
        };
        let script = generate_fill_script(&profile);
        assert!(script.contains("Ada Lovelace"));
        assert!(!script.contains("click()"));
    }

    #[test]
    fn apply_kit_writes_files() {
        let dir = std::env::temp_dir().join("atsassin_apply_kit");
        let profile = ApplyProfile {
            name: "Test".into(),
            ..Default::default()
        };
        write_apply_kit(&profile, &dir).unwrap();
        assert!(dir.join("apply.js").exists());
        assert!(dir.join("bookmarklet.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
