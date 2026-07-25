//! Free, local, zero-LLM-call relevance ranking of scanned jobs against the
//! user's real profile. Runs before any LLM evaluation so the tool stays
//! usable at zero budget and, when a paid/hosted model IS configured, so
//! `evaluate` can be pointed at the subset actually worth spending calls on
//! (career-ops/jobsync pattern: cheap prerank, then LLM on the top-K only).
//!
//! Uses term-overlap weighted by inverse document frequency computed over
//! the batch of jobs actually being ranked (a real local corpus - the jobs
//! just scanned), not a fabricated relevance number.

use crate::models::profile::UserProfile;
use std::collections::{HashMap, HashSet};

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 2)
        .map(|s| s.to_string())
        .collect()
}

/// Weighted terms derived from the user's real profile - skills count more
/// than free-text experience titles.
fn profile_terms(profile: &UserProfile) -> HashMap<String, f64> {
    let mut terms: HashMap<String, f64> = HashMap::new();
    for skill in &profile.skills {
        for t in tokenize(&skill.name) {
            *terms.entry(t).or_insert(0.0) += 2.0;
        }
    }
    for exp in &profile.experience {
        for t in tokenize(&exp.title) {
            *terms.entry(t).or_insert(0.0) += 1.0;
        }
    }
    terms
}

/// Scores and sorts `items` by relevance to `profile`, highest first.
/// `text_of` extracts the searchable text (title + description, typically)
/// from each item. Returns `(original_index, score in [0.0, 1.0])` pairs.
pub fn rank<T>(
    profile: &UserProfile,
    items: &[T],
    text_of: impl Fn(&T) -> String,
) -> Vec<(usize, f64)> {
    let terms = profile_terms(profile);
    if terms.is_empty() || items.is_empty() {
        return items.iter().enumerate().map(|(i, _)| (i, 0.0)).collect();
    }

    let item_tokens: Vec<HashSet<String>> = items
        .iter()
        .map(|j| tokenize(&text_of(j)).into_iter().collect())
        .collect();

    let n = items.len() as f64;
    // Smoothed IDF: ln(N/(df+1)) + 1. A term in exactly one of two docs must
    // still score meaningfully higher than one in both - the naive
    // ln(N/(1+df)) form hits exactly 0 in that case (ln(2/2)=0), which wiped
    // out the very terms doing the discriminating. This form only trends
    // toward its (still positive) floor as df approaches N.
    let idf: HashMap<&str, f64> = terms
        .keys()
        .map(|term| {
            let df = item_tokens
                .iter()
                .filter(|toks| toks.contains(term))
                .count() as f64;
            (term.as_str(), (n / (df + 1.0)).ln() + 1.0)
        })
        .collect();

    let max_possible: f64 = terms
        .iter()
        .map(|(t, w)| w * idf.get(t.as_str()).copied().unwrap_or(0.0))
        .sum();

    let mut scored: Vec<(usize, f64)> = item_tokens
        .iter()
        .enumerate()
        .map(|(i, toks)| {
            let raw: f64 = terms
                .iter()
                .filter(|(t, _)| toks.contains(t.as_str()))
                .map(|(t, w)| w * idf.get(t.as_str()).copied().unwrap_or(0.0))
                .sum();
            let score = if max_possible > 0.0 {
                (raw / max_possible).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (i, score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::profile::{Skill, SkillCategory, SkillLevel};
    use chrono::Utc;

    fn profile_with_skills(skills: &[&str]) -> UserProfile {
        UserProfile {
            name: "Test".to_string(),
            email: None,
            phone: None,
            location: None,
            linkedin_url: None,
            portfolio_url: None,
            summary: None,
            skills: skills
                .iter()
                .map(|s| Skill {
                    name: s.to_string(),
                    category: SkillCategory::Technical,
                    level: SkillLevel::Advanced,
                    years: None,
                })
                .collect(),
            experience: vec![],
            education: vec![],
            certifications: vec![],
            languages: vec![],
            raw_text: String::new(),
            inferred_roles: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn ranks_matching_job_higher() {
        let profile = profile_with_skills(&["Rust", "SQLite", "Distributed Systems"]);
        let jobs = vec![
            "Senior Rust engineer building SQLite-backed distributed systems".to_string(),
            "Marketing copywriter for a fashion brand".to_string(),
        ];
        let ranked = rank(&profile, &jobs, |j: &String| j.clone());
        assert_eq!(ranked[0].0, 0, "the Rust/SQLite job should rank first");
        assert!(ranked[0].1 > ranked[1].1);
    }

    #[test]
    fn empty_profile_terms_returns_zero_scores_not_panic() {
        let profile = profile_with_skills(&[]);
        let jobs = vec!["Anything".to_string()];
        let ranked = rank(&profile, &jobs, |j: &String| j.clone());
        assert_eq!(ranked.len(), 1);
    }
}
