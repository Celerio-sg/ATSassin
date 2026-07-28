# Path to Category Leadership: a Red-Team Pass

## Framing, honestly

"Definitively guarantee unequivocal leadership" isn't an achievable engineering target — competitors ship, the market shifts, and a point-in-time benchmark win (see `COMPETITIVE_BENCHMARK_2026-07-25.md`) decays the moment a competitor closes the gap it measured. What *is* achievable is designing so that:

1. **Every past win is structurally hard to regress** (tests, not just fixes).
2. **Decay is detected automatically**, not discovered by a user hitting a broken feature.
3. **The tool degrades gracefully at the low end and scales meaningfully at the high end** — one binary, two very different operating envelopes.

This document red-teams the current state against those three goals and lays out what closes each gap. It complements `ROADMAP.md` (which tracks discrete known issues, now mirrored as GitHub Issues #1-7) — this document is about the *system* around those fixes, not any single one of them.

## Red-team: where today's "leadership" is fragile

### 1. Every real-world scraper is an undocumented API, and nothing watches for drift
`scrape_linkedin`, `scrape_seek`, and the Greenhouse/Lever/Ashby integrations all depend on API shapes that aren't publicly documented and can change without notice — already true of the company directory (Issue #1) but actually true of *every* scraper in `src/pipeline/scraper.rs`. Right now, breakage is discovered the same way the US-only-results bug and the HN-search bug were discovered this session: a human runs a real search and notices the output looks wrong. That's a fragile discovery mechanism for a tool whose core value proposition is "we actually search."

**Gap:** no automated, scheduled check that these endpoints still return sane, non-empty, correctly-shaped data.

### 2. Competitive leadership was measured once, not maintained
The 7-competitor benchmark (career-ops, job-ops, jobsync, job_finder, ApplyPilot, Resume-Matcher, ai-job-search) is a snapshot from 2026-07-25. Every one of those repos can ship a feature next month that erases a gap this benchmark found. There's currently no cadence for re-running it.

**Gap:** no scheduled or triggered re-benchmark; "we're ahead" is a claim with a timestamp, not a standing guarantee.

### 3. The prompt-completeness bug class has no regression test
This session found and fixed a real defect where `tailor`'s resume prompt silently dropped 13 of 16 real experience entries and omitted education/contact fields the data model already supported. The fix was a better prompt — but nothing in the test suite would catch this regressing if the prompt is touched again, because the failure mode is "the LLM quietly did less than asked," which a normal unit test can't observe without either mocking the LLM or hitting the real API in CI.

**Gap:** no test asserts "tailor's prompt instructs inclusion of every experience entry" (a testable property of the *prompt*, independent of what the LLM does with it) or, better, an opt-in integration test that hits a real provider and asserts entry-count parity against a fixture profile.

### 4. "Scales up to consume full resource of powerful APIs" is partially true, not fully
Local Ollama mode genuinely tiers by hardware (`src/engine/hardware.rs`, light/balanced/full quantization). Hosted-provider mode does **not** — `--preset` only changes timeout/retry/scrape-limit, never model choice (Issue #3). A user with a Groq Enterprise budget and a user on the free tier get the identical model today. That's the single biggest gap between the current implementation and the "scale up to consume full resource... when available" framing in the request.

**Gap:** no cost/quality tier differentiation on hosted providers — tracked as Issue #3, elevated here because it's the literal thing being asked for.

### 5. "Runs on any hardware" has never been tested on any hardware that would actually stress it
Issue #5 (already filed) — every test this session ran on capable development hardware. The floor of the claimed operating envelope is unverified.

### 6. Compensation and "likely to land" signals are both LLM/heuristic-only, with no ground truth
`market rates` is pure LLM estimation (Issue #4, with an honesty disclaimer already added — good, but a disclaimer isn't a fix). The new `recommend` composite score (`engine::landscore`) is a well-reasoned heuristic, but it's untested against actual outcomes — nothing closes the loop between "ATSassin said this was a 78" and "did that job actually respond/interview/offer." Without that loop, the ranking's *credibility* rests entirely on the formula being intuitively reasonable, not on evidence it's right.

**Gap:** ✅ RESOLVED - Daemon now wires pipeline status changes to feedback tracking. `engine::feedback` is integrated with pipeline status transitions (Applied → Interviewing → Offered/Rejected), providing the ground truth loop.

### 7. Docs already drifted from shipped reality mid-session
`README.md` has zero mentions of `recommend`, `pipeline show`, or `--location` — three features shipped in the last few hours. This isn't hypothetical decay, it already happened once in this session. If it happens this fast under active development, it will happen faster once development slows down.

**Gap:** no CI check that new subcommands are documented (even a weak one — e.g. asserting every `Commands::` variant name appears somewhere in README.md — would have caught this).

### 8. No security/privacy hardening for what's now a growing personal data store
The `applications` table (added this session) now holds full resume/cover-letter text and complete job descriptions in plaintext SQLite, alongside the profile (name, email, phone, full career history). For a single-user local tool this is a reasonable default, but "leadership" claims around a tool that increasingly resembles a personal CRM should include at minimum a documented stance on this, and ideally an opt-in encryption-at-rest path for users who sync this file to cloud storage (common for a `.db` sitting in a home directory).

## Roadmap: three horizons, each closing a specific gap above

### Now — structural, not feature work
- **Board-health canary** (closes gap #1): a scheduled GitHub Action (e.g. daily) that runs a small, fixed set of real scans against LinkedIn/Seek/Greenhouse/Lever/Ashby with a known-should-return-results query, and opens/updates a tracking issue automatically if any board returns suspiciously empty (mirrors the "empty 200 is suspicious" fix from Issue #7, applied at the monitoring layer, not just the request layer).
- **Cloud-provider model tiering** (closes gap #4, = Issue #3): make `--preset` pick different model *sizes* per hosted provider tier, not just timeouts. This is the most requested-feels-obvious gap and the most literal reading of "scale up to consume full resource... when available."
- **README sync check** (closes gap #7): minimal CI assertion that every `Commands::` variant has a corresponding README section; fails the build otherwise. Cheap, mechanical, prevents exactly the drift that already happened this session.
- **Resume-completeness regression test** (closes gap #3): a prompt-content test (assert the system prompt contains the completeness instruction) as a fast unit test, plus an `#[ignore]`-by-default integration test that hits a real configured provider and checks entry-count parity — run manually or in a separate opt-in CI job, not blocking every PR (keeps CI fast and free-tier-friendly).

### Next — closing the credibility loop
- ✅ **Wire `engine::feedback` to pipeline status transitions** (closes gap #6): COMPLETED - Daemon now wires pipeline status changes to feedback tracking. Every `pipeline update --status` change (especially → Interviewing/Offered/Rejected) is a free, real outcome signal already flowing through the tool.
- **Quarterly competitive re-benchmark** (closes gap #2): not full manual re-runs each time — a lighter automated check (does each competitor repo still build/run at all, has its `package.json`/`Cargo.toml`/`requirements.txt` changed meaningfully since last benchmark) that flags "worth a manual re-benchmark" rather than claiming to fully automate competitive analysis.
- **Real market-data source** (closes gap #6, = Issue #4): replace pure LLM compensation estimation with a periodically-updated static dataset as a floor, LLM estimation only as a gap-filler/adjustment on top of real numbers.

### Later — the operating-envelope extremes
- Validate on genuine low-spec hardware (Issue #5) and publish the result (a `HARDWARE_VALIDATION.md` with real numbers, not just a claim).
- Optional encryption-at-rest for the SQLite store (gap #8) — opt-in, since it adds friction (a passphrase) most single-user local setups don't need by default.
- Per-host concurrency generalization once a second concurrent multi-company sweep exists (Issue #7).

## What "unequivocal leadership" means under this plan

Not a permanent state — a standing, verifiable claim with three properties this roadmap is built to guarantee instead:
1. Every fixed defect has a test (or an explicit, documented reason it can't) so it can't silently regress.
2. Drift in dependencies ATSassin doesn't control (job-board APIs, competitor feature sets) is detected automatically within days, not discovered by a user.
3. The gap between "runs on a 4GB CPU laptop" and "burns through a hosted-API budget for maximum quality" is a real, tested, tiered continuum — not a claim that's only true at one end today.
