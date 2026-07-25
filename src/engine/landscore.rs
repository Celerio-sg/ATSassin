//! "Likely to land quickly" composite scoring - answers a different question
//! than `prerank`/`evaluate` do. Those ask "is this a good match?"; this asks
//! "of the jobs that already look like a good match, which is worth acting
//! on first?" Built from signals that correlate with speed-to-offer, not
//! just fit:
//!
//! - **Fit** (60%): the real LLM `evaluate` score when one exists, else the
//!   free lexical `prerank` score as a proxy (never fabricated - a job with
//!   neither is scored on relevance alone, not silently boosted).
//! - **Lexical corroboration** (15%): the `prerank` relevance score again,
//!   even when an LLM score exists - rewards jobs that are unambiguously
//!   on-topic, not just ones the LLM liked for a tenuous reason.
//! - **Recency** (up to 15pts): a posting that went up in the last few days
//!   has had fewer applicants reach it yet than one that's been live a
//!   month. This is a real, if imperfect, competition proxy - no click-through
//!   applicant-count data exists to measure competition directly.
//! - **Explicit preference-fit signal** (+10 contract/interim/fractional
//!   keyword bonus, -15 preference-mismatch penalty): a job that matches the
//!   user's stated employment-type/comp/work-mode preferences is one they'll
//!   actually pursue and be a credible candidate for, which matters more to
//!   "lands quickly" than raw seniority overlap does.

use crate::engine::preferences::PreferenceMatch;
use chrono::{DateTime, Utc};

const CONTRACT_KEYWORDS: [&str; 6] = [
    "contract",
    "contractor",
    "interim",
    "fractional",
    "freelance",
    "day rate",
];

#[derive(Debug, Clone)]
pub struct LandScore {
    /// Final 0-100 ranking score - higher sorts first.
    pub composite: f64,
    /// The free lexical prerank score (0-100) this job was given within its
    /// scan batch.
    pub relevance_pct: f64,
    /// The real LLM evaluation score (0-100), if `evaluate` has run for this
    /// job yet.
    pub eval_score_pct: Option<f64>,
    pub pref_match: bool,
    pub pref_reasons: Vec<String>,
    pub contract_signal: bool,
    pub posted_days_ago: Option<i64>,
}

/// `relevance` is the 0..1 prerank score for this job within its batch.
/// `eval_score` is the 0..1 LLM evaluation score, if one exists.
/// `text` is title+description, used only to detect contract/interim/
/// fractional language - a real, already-scraped signal, not an inference.
pub fn score(
    relevance: f64,
    pref: &PreferenceMatch,
    eval_score: Option<f64>,
    posted_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    text: &str,
) -> LandScore {
    let fit = eval_score.unwrap_or(relevance);
    let text_lower = text.to_lowercase();
    let contract_signal = CONTRACT_KEYWORDS.iter().any(|kw| text_lower.contains(kw));
    let posted_days_ago = posted_at.map(|p| (now - p).num_days().max(0));

    // Linear decay from +15 at 0 days old to 0 at 30+ days old. Unknown
    // posting date gets a small neutral credit rather than a penalty - we
    // genuinely don't know, and most boards don't expose it reliably.
    let recency_bonus = match posted_days_ago {
        Some(days) => (15.0 - (days as f64 / 30.0) * 15.0).clamp(0.0, 15.0),
        None => 5.0,
    };

    let contract_bonus = if contract_signal { 10.0 } else { 0.0 };
    let pref_penalty = if pref.matches { 0.0 } else { 15.0 };

    let composite =
        (fit * 100.0 * 0.60 + relevance * 100.0 * 0.15 + recency_bonus + contract_bonus
            - pref_penalty)
            .clamp(0.0, 100.0);

    LandScore {
        composite,
        relevance_pct: relevance * 100.0,
        eval_score_pct: eval_score.map(|s| s * 100.0),
        pref_match: pref.matches,
        pref_reasons: pref.reasons.clone(),
        contract_signal,
        posted_days_ago,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pref(matches: bool) -> PreferenceMatch {
        PreferenceMatch {
            matches,
            reasons: if matches {
                vec![]
            } else {
                vec!["test reason".to_string()]
            },
        }
    }

    #[test]
    fn fresh_contract_posting_with_high_eval_scores_highest() {
        let now = Utc::now();
        let strong = score(
            0.9,
            &pref(true),
            Some(0.9),
            Some(now),
            now,
            "Interim CRO contract role",
        );
        let weak = score(
            0.9,
            &pref(true),
            Some(0.9),
            Some(now - chrono::Duration::days(60)),
            now,
            "Permanent CRO role",
        );
        assert!(strong.composite > weak.composite);
    }

    #[test]
    fn preference_mismatch_is_penalized_not_excluded() {
        let now = Utc::now();
        let matched = score(0.5, &pref(true), None, None, now, "");
        let mismatched = score(0.5, &pref(false), None, None, now, "");
        assert!(matched.composite > mismatched.composite);
        assert!(
            mismatched.composite > 0.0,
            "a mismatch should be deprioritized, not zeroed out"
        );
    }

    #[test]
    fn missing_eval_falls_back_to_relevance_not_zero() {
        let now = Utc::now();
        let s = score(0.8, &pref(true), None, None, now, "");
        assert!(s.eval_score_pct.is_none());
        assert!(s.composite > 0.0);
    }
}
