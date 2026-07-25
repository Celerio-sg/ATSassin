# Roadmap, Known Issues & Ethical Considerations

> **Ground truth for contributors:** the two most reliable documents in this repo are [UAT_REPORT_2026-07-24.md](UAT_REPORT_2026-07-24.md) (issues found and fixed via real end-to-end testing across 5 personas) and [COMPETITIVE_BENCHMARK_2026-07-25.md](COMPETITIVE_BENCHMARK_2026-07-25.md) (ATSassin actually run head-to-head against 7 competitor tools). Where this roadmap and older planning docs (`COMPARISON.md`, `EXECUTIVE_SUMMARY.md`, `RANKING.md`, `COMPLIANCE_MATRIX.md`) disagree, trust the UAT/benchmark reports — they reflect what was actually run and observed, not what was planned or claimed.

## What's real today (verified by actual execution, not just present in the code)

- Profile parsing (Markdown/plain-text resumes, LinkedIn export CSVs, DOCX)
- Local-relevance role inference and pipeline tracking (SQLite), evaluate and tailor via any OpenAI-compatible provider (Groq, Ollama, Kimi, GLM, Lightning, etc.)
- Job discovery: LinkedIn guest API, Seek, single-company Greenhouse/Lever/Ashby lookups, a curated **concurrent zero-token sweep across ~35 companies' public Greenhouse APIs** (`--boards companies`), plus a social/aggregator sweep (HN, Reddit, RemoteOK, WeWorkRemotely, etc.)
- A full TUI dashboard: live role inference, scan, evaluate, and tailor, all triggerable from within the TUI, with a real pipeline summary and activity log
- Verified end-to-end for 5 realistic, diverse personas (see UAT report) with zero crashes and non-fabricated output

## Known issues / open gaps

These are real, currently-open items, not resolved yet:

1. **Scan board coverage is still narrow relative to the job market as a whole.** The `companies` directory currently lists ~35 companies (skewed toward AI-native firms, inherited from the initial seed list). Indeed and Glassdoor remain bot-protected and unreliable. **Good first issue**: extend `src/pipeline/company_directory.rs` with more companies across more industries (it's a flat, append-only list — no architecture change needed).
2. **`--preset` (lightweight/balanced/full) has no effect on model choice when using a hosted cloud provider** — it only changes timeout/retry/scrape-limit values. The hardware-adaptive tiering system only differentiates behavior when running against local Ollama. This should either be documented more prominently or extended so cloud presets pick different model *sizes* per tier (e.g. a cheaper/faster Groq model for `lightweight`).
3. **Compensation estimates are model-generated and can still be wrong even after the seniority-aware sanity clamp** added in this pass (see `role_inference.rs`) — treat `market rates`/inferred compensation bands as a rough guide, not authoritative data. A real market-data source (even a periodically-updated static dataset) would be a meaningful accuracy improvement over pure LLM estimation.
4. **No genuine low-spec/CPU-only hardware was used to validate the "works on any hardware" claim** — testing so far has been on capable development hardware. Needs validation on an actual 4GB-RAM/CPU-only machine.
5. **Region filter in the TUI (`g` to cycle) is a simple query-suffix, not a real geographic filter** — it appends "North America"/"Europe"/"APAC" to the search query rather than filtering by parsed job location. A proper location-aware filter (parsing each posting's location field, not just hoping the search API respects a region keyword) would be more reliable.
6. **Lightning AI provider integration returns 401 Unauthorized** in every test so far — unconfirmed whether this is a request/auth-format bug in `src/engine/llm.rs` or an environment credential issue. Needs investigation with known-good Lightning credentials.

## Roadmap

### Now
- Expand the zero-token company directory (see gap #1 above) — this is the highest-leverage, lowest-risk contribution available right now.
- Location-aware region filtering (gap #5).
- Cloud-provider model tiering for `--preset` (gap #2).

### Next — Career Coaching (flagship feature request)

A built-in career coaching mode, going beyond "find and tailor applications for the job you already know you want" toward helping a candidate figure out what they actually want:

- **Earning-potential analysis**: given a candidate's real skills/experience, surface adjacent roles, industries, or arrangements (contract vs. FTE, remote vs. local market) they may not have considered that pay meaningfully more for skills they already have — grounded in real market data (see gap #3; this feature depends on solving that first), not hand-wavy LLM guessing.
- **Interests-outside-work discovery**: a structured, low-pressure conversation (not a generic "what are your hobbies" prompt) that surfaces genuine interests and turns them into a short list of *real, currently-open roles* that connect to those interests — using the existing role-inference and scan infrastructure, but seeded from interests rather than only from past job titles. The goal is surfacing options the candidate wouldn't have searched for themselves, not replacing their judgment.
- Should reuse the existing `profile`/`roles`/`scan` pipeline rather than becoming a separate subsystem — coaching output should be able to flow directly into `evaluate`/`tailor` like any other discovered role.
- Explicitly **not** meant to auto-decide anything for the user — this is a discovery/expansion tool, consistent with the "recommends and tailors, never auto-applies" principle below.

### Later
- OpenVINO INT8 runtime for Intel Arc/Iris Xe
- Real model distillation pipeline (train a small 22M-109M local classifier from usage data)
- Browser-automation-based application submission (opt-in, explicit per-application confirmation — never silent)
- Multi-user / team collaboration
- WASM plugin system for custom matching logic

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| LLM provider rate limits / outages | Medium | High | Multi-provider fallback, already implemented; offline/local-model mode via Ollama |
| Scraper/ATS-API changes breaking scan sources | Medium | Medium | Every scraper degrades to an honest empty result on failure, never fabricates (verified in UAT) |
| Compensation-estimate inaccuracy | Medium | Medium | Seniority-aware sanity clamp in place (this pass); real market-data source is the durable fix (gap #3) |
| Binary size growth | Low | Low | LTO + strip already in `[profile.release]`; currently ~9.5MB |
| Community trust (privacy) | Low | High | Local-first by default, no telemetry beyond opt-in local logging, MIT-licensed and open source |

## Ethical Considerations

1. **Privacy by design**: ATSassin runs locally by default. No resume data leaves the machine unless the user explicitly configures a cloud LLM provider.
2. **Transparency**: LLM calls are logged locally (`*.llm_telemetry.jsonl`); users can audit exactly what was sent and received.
3. **No automation overreach**: ATSassin recommends, evaluates, and tailors — it does not auto-apply on a user's behalf without explicit action, and any future application-automation feature (see Later) must stay opt-in and per-application confirmed.
4. **Accessibility**: free, lightweight (~9.5MB, no GPU or Python/Node runtime required), designed to run on modest hardware — not a subscription SaaS product.
5. **Honesty over impressiveness**: this project has a documented history (see UAT report) of finding and removing fabricated/placeholder output disguised as real functionality. That standard should hold for all future contributions — an empty, honestly-labeled result is always preferable to a fabricated one.
