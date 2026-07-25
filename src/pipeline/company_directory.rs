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
