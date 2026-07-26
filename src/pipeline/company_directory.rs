//! A curated directory of companies with a known, live Greenhouse job-board
//! API - the same "zero-token, direct-API" technique used by career-ops's
//! `scan.mjs`/`portals.yml` (a competitor benchmarked against ATSassin;
//! see docs/COMPETITIVE_BENCHMARK_2026-07-25.md). Which ATS platform a
//! company publicly exposes its careers page through is a public fact, not
//! proprietary logic - this list distills that same technique down to its
//! first principle (curated company -> ATS-board-slug mapping, hit the
//! public JSON API directly, filter locally, zero LLM tokens) and re-
//! implements it natively: concurrent async fetches instead of a sequential
//! sweep, and no dependency on an external AI-CLI/WebSearch fallback for
//! any entry on this list - everything here resolves via a plain HTTP GET.
//!
//! Extend this list freely; a stale or renamed slug degrades gracefully
//! (scrape_greenhouse returns an empty result for a 404, never a crash) and
//! is reported, not hidden - see `CompanySweepReport`.
//!
//! `APAC_GREENHOUSE_COMPANIES` (issue #14) extends the US/EU-centric
//! `GREENHOUSE_COMPANIES` with SaaS firms that do expose a Greenhouse
//! board from APAC - before this was added, an SG / HK / JP / AU candidate
//! calling `atsassin scan --boards companies` got 0 results from this
//! board and the highest-signal sweep was effectively invisible. The
//! challenge with verification is that some Greenhouse boards don't
//! follow the `/<slug>` path, so a developer who slash-searches a few
//! likely candidates should add the ones that resolve. A stale slug still
//! degrades gracefully.
pub const GREENHOUSE_COMPANIES: &[(&str, &str)] = &[
    ("Anthropic", "anthropic"),
    ("PolyAI", "polyai"),
    ("Parloa", "parloa"),
    ("Intercom", "intercom"),
    ("Hume AI", "humeai"),
    ("Airtable", "airtable"),
    ("Vercel", "vercel"),
    ("Temporal", "temporal"),
    ("Arize AI", "arizeai"),
    ("RunPod", "runpod"),
    ("Weights & Biases (CoreWeave)", "coreweave"),
    ("Glean", "gleanwork"),
    ("Speechmatics", "speechmatics"),
    ("Boomi", "boomilp"),
    ("Later", "later"),
    ("Safari AI", "safariai"),
    ("Hootsuite", "hootsuite"),
    ("Black Forest Labs", "blackforestlabs"),
    ("Helsing", "helsing"),
    ("Celonis", "celonis"),
    ("Contentful", "contentful"),
    ("GetYourGuide", "getyourguide"),
    ("HelloFresh", "hellofresh"),
    ("N26", "n26"),
    ("Trade Republic", "traderepublicbank"),
    ("SumUp", "sumup"),
    ("Scandit", "scandit"),
    ("Wayve", "wayve"),
    ("Isomorphic Labs", "isomorphiclabs"),
    ("PhysicsX", "physicsx"),
    ("Stability AI", "stabilityai"),
    ("Templafy", "templafy"),
    ("Amplemarket", "amplemarket"),
    ("Runway", "runwayml"),
    ("Hightouch", "hightouch"),
    ("PlanetScale", "planetscale"),
];

/// APAC Greenhouse boards. Slugs have been verified against
/// `boards.greenhouse.io/<slug>` returning valid JSON listings as of
/// mid-2026; the list is intentionally conservative so that a sweep that
/// comes back empty for an APAC candidate is real (zero matches found),
/// not silent (slug returns 404 and the convention spread into the UI
/// unchanged). To find more slugs, redirect to a company's `/careers`
/// page and look for `boards.greenhouse.io/<slug>` in the network tab.
pub const APAC_GREENHOUSE_COMPANIES: &[(&str, &str)] = &[
    ("Canva", "canva"),
    ("Xero", "xero"),
    ("SafetyCulture", "safetyculture"),
    ("Airwallex", "airwallex"),
    ("Klaviyo", "klaviyo"),
    ("Stripe (APAC)", "stripe"), // US-headquartered but lists SG/AU jobs
];

/// Combined sweep target. The two source lists stay separate so contributors
/// can tell at a glance which geography a new entry helps; the runtime
/// uses this concatenation so concurrent sweep and rate-limit accounting
/// don't have to special-case which list a slug came from.
///
/// Built as a function rather than a `pub const` because `Vec::new` /
/// `extend_from_slice` / `Box::leak` aren't `const fn`. Callers should
/// ideally invoke it once and stash the result; in practice the only
/// caller (`scrape_companies`) does one allocation per sweep, paid in
/// well under a millisecond, so the simplicity of a function wins over
/// the cost of a `Lazy<Vec<...>>` here.
pub fn all_greenhouse_companies() -> Vec<(&'static str, &'static str)> {
    GREENHOUSE_COMPANIES
        .iter()
        .chain(APAC_GREENHOUSE_COMPANIES.iter())
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_greenhouse_companies_includes_apac_entries() {
        // Locked-in: APAC representative must stay in the combined list.
        // If a future refactor accidentally drops APAC slugs (issue #14
        // regressing), this test fails before the vacancy reaches the
        // run-time sweep, where it would silently degrade the SG/HK
        // /JP/AU candidate experience to zero `companies` results.
        let all = all_greenhouse_companies();
        let apac_slugs: std::collections::HashSet<&str> = APAC_GREENHOUSE_COMPANIES
            .iter()
            .map(|(_, slug)| *slug)
            .collect();
        let combined_slugs: std::collections::HashSet<&str> =
            all.iter().map(|(_, slug)| *slug).collect();
        for slug in &apac_slugs {
            assert!(
                combined_slugs.contains(slug),
                "APAC slug {slug:?} missing from combined list (issue #14 regression)"
            );
        }
    }

    #[test]
    fn apac_and_us_eu_lists_disjoint() {
        // If a contributor accidentally re-classifies a US/EU company as
        // APAC (or vice versa), the union still works but the distinct
        // *whose problem did this contribution fix* signal blur. Keep
        // them disjoint so lists remain meaningful for at-a-glance review.
        let mut us_eu: std::collections::HashSet<&str> =
            GREENHOUSE_COMPANIES.iter().map(|(_, s)| *s).collect();
        for (_, slug) in APAC_GREENHOUSE_COMPANIES {
            assert!(
                !us_eu.contains(slug),
                "slug {slug:?} appears in both APAC and US/EU lists; pick one"
            );
            us_eu.insert(slug);
        }
    }
}
