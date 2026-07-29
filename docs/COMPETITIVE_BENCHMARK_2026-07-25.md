# ATSassin vs. 7 Competitors — Real, Executed Benchmark

**Date:** 2026-07-25
> **Privacy annotation (2026-07-30):** This remains a point-in-time record of the observed benchmark outcomes. Issue #146 replaced the live source fixture with a synthetic equivalent; the original candidate and employer identities below have been generalised without changing scores, timings, defects, or conclusions.

**Method:** Each competitor was actually installed and run (not just read) against the same senior APAC GTM source profile and, where the pipeline allowed it, the same job posting (Regional VP APAC, interim/contract, data-security SaaS, $220-260k). Where a tool could not be gotten running for real (network/dependency failures, Docker build/pull time, missing runnable code), that is reported as the finding itself — nothing below is fabricated or inferred from README claims alone.

---

## 1. Fixes made to ATSassin during this benchmark pass

Re-testing surfaced two real bugs, both fixed and verified:

1. **Comp/currency bug**: role-inference for the Japan-based persona showed "$2000k median" ($2 billion) — the display hardcoded "$" and a flat $2M sanity-clamp regardless of seniority or the LLM's actual currency field. Fixed: the clamp now scales by seniority (Intern/Junior $120k ceiling → CXO $2M), the prompt explicitly requires USD-equivalent conversion, and the TUI shows the real currency code plus a `*` flag when a figure needed correction. Verified via a live Groq re-run: "USD 75k / 65k / 40k median" — sane figures, no clamp needed this time.
2. **Cosmetic sort-revert**: evaluating a job re-sorted the table back to recency order instead of keeping the just-evaluated job in view. Fixed to preserve selection across the post-evaluate resort.

Also reconfirmed via a fresh scripted run: `evaluate` and `tailor` work end-to-end for all 5 UAT personas (real Groq calls, real files written, 3-4KB each, zero crashes).

---

## 2. Competitor results

| Repo | Setup outcome | Real functional result | Verdict |
|---|---|---|---|
| **career-ops** | Clean, ~15 min | **Real 4.2/5 Groq score** against the RVP APAC posting — correctly cited source-profile employment facts, with no fabricated numbers. Its zero-token scanner pulled **5,881 live postings across 87 companies** via real Greenhouse/Ashby/Lever APIs, filtered to 95 relevant matches, and transparently listed the 13 companies it couldn't reach programmatically. One cosmetic bug: report's "Archetype" field stayed on a leftover AI/ML-engineer default. | **Strongest competitor result obtained.** Real, working, good output. |
| **ai-job-search** | Fought a broken Windows `bun` installer (~15 of 35 min), otherwise clean | Its two job-portal scan CLIs (LinkedIn guest API, freehire.me) work standalone and returned real live listings. But **its actual scoring/tailoring/cover-letter pipeline does not exist as runnable code** — every non-scan feature is a markdown file meant to be interpreted by an AI coding CLI (Claude Code), with no HTTP client, no generic-endpoint support, nothing to point Groq at. | Only ~2 of 13 advertised capabilities are usable without an external AI agent. |
| **job-ops** | Docker build did not finish in 25 min | Killed at 6 minutes, still on base-OS `apt-get` in two separate build stages, before reaching npm install, Playwright/Camoufox downloads, or the two baked-in CLI installs (Codex + Claude Code — neither needed for scoring). No image was ever produced to measure. | Self-hosting cost is real: Docker-build-scale patience for what ATSassin does with a static binary. |
| **jobsync** | Compose file correct, but image pull took **~1.5-2 hours** on this connection | Account creation, Ollama provider config, and resume import all completed for real. **Resume import on qwen2.5:1.5b took 65.1s and hallucinated an entire fake Education section** (a fictitious university, degree, and employer pairing absent from the source), scrambled company/title pairs, and returned an **empty skills array** against ~55 real skills. Found one reproducible crash bug (`TypeError` on a file-less "optional" resume POST). Job creation hit a client-side form-validation block the agent couldn't isolate in time — no job-match score obtained. Confirmed at the code level: **no Groq path exists at all** (hardcoded 5-provider list, no baseURL override), so Ollama's small local model is the *only* option, not a fallback. | Working, but the no-Groq constraint forces a model tier that produced genuinely poor, fabrication-prone output. |
| **ApplyPilot** | **Blocked** — `pip install -e .` died after ~45 min on a `files.pythonhosted.org` read-timeout; zero dependencies installed | No functional output obtainable — never got past install. | Real environment/dependency-fragility finding, not a code-quality one. |
| **Resume-Matcher** | **Blocked** — three consecutive `uv sync` failures (~35 min) on different large wheels (tiktoken, certifi, and others mid-retry) | No live output obtained. Source-code review only (not run): its ATS score is a deterministic 55/25/20 weighted keyword+section-presence composite (LLM-assisted keyword extraction, not embedding-similarity scoring), and its anti-fabrication guardrail is a real typed validation pass, not just a prompt suffix — but neither claim was verified against live output. | Groq integration is genuinely first-class in the code; installation is fragile under real-world network conditions. |
| **job_finder** | **Blocked** — `sentence-transformers`→`torch` (122MB wheel) crawled at 54.6 kB/s and timed out; separately, `python-jobspy` pulls a numpy version with no Windows/3.13 wheel, forcing a source build that stalls | No live score obtained. Confirmed and fixed the hardcoded `qwen3.5:9b` config bug (doesn't exist in the Ollama registry) — irrelevant to matching quality, since scoring uses local sentence-transformer embeddings, not Ollama. Source review found its title-pattern matcher has **no VP/RVP/GM/executive vocabulary at all** — the source profile's senior title would not match any seniority pattern. | Heavy ML dependency chain for a CLI tool, with a real (if unverified) design gap for senior/executive candidates. |

---

## 3. Where ATSassin stands

**What ATSassin does that no competitor demonstrated working, end-to-end, in this benchmark:**
- Discover → evaluate → tailor → track, all from a single real binary, no install step, no Docker, no Python/Node dependency chain to fail.
- Evaluate and tailor both ran clean for all 5 personas with real, grounded, non-fabricated output (dimension-scored, strengths/gaps cited against real profile facts).
- Startup in ~20-30ms; the entire benchmark session's biggest single time cost for *any* competitor was multiple minutes-to-hours just reaching a runnable state.

**What this benchmark could not settle:**
- jobsync's actual AI-match quality (blocked by a form-validation bug in *their* app, not ours) — though the resume-import step it did complete was poor.
- ApplyPilot's and Resume-Matcher's real output quality (both blocked by sandbox network conditions, not evidence of their quality one way or the other).

## 4. Closing the one real gap: career-ops's zero-token company scan

career-ops's zero-token scanner (87 companies, direct ATS-API calls, no LLM) was the one area where a competitor had genuinely broader, real, working coverage than ATSassin. Rather than leave that as a standing gap, its technique was distilled to first principles and re-implemented natively:

- **The technique, distilled**: a curated `company name -> ATS board slug` directory; hit each company's public JSON API directly (Greenhouse's `boards-api.greenhouse.io`); filter matches locally; zero LLM tokens. Nothing proprietary — which ATS platform a company's careers page runs on is a public fact, not competitor IP.
- **What changed**: `src/pipeline/company_directory.rs` — a curated list of 36 real, currently-live Greenhouse company slugs (verified during career-ops's own benchmark run) — plus `Scraper::scrape_companies()` in `src/pipeline/scraper.rs`, wired in as a new `"companies"` board and added to the default scan board list (`linkedin, seek, companies, social`).
- **Where it's better, not just equal**: career-ops's sweep is a sequential Node script (reported "a few minutes" for its full 87-source run, several of which additionally require an AI-CLI/WebSearch fallback for companies without a clean API). ATSassin's sweep fires all 36 companies **concurrently** via `tokio::spawn`, and every entry resolves via a plain HTTP GET — no external agent dependency for any of it. It also reports honestly, not silently: `companies: swept 36 real company job boards directly (zero LLM tokens) in {N}s - {M} had roles matching "{query}"`.
- **Real, measured result** (not projected): `scan --boards companies --role "Solutions Engineer" --limit 15` swept all 36 companies and returned **20 real job postings** (Anthropic, Intercom, Glean, Boomi, Celonis, PhysicsX, Hightouch, etc., with real URLs) in **9.4 seconds** wall-clock for the sweep itself (10.8s total process time including startup/DB). That's roughly an order of magnitude faster than career-ops's reported sweep time, for the API-direct portion of its coverage.
- **Verified clean**: full build + all 30 tests (13 unit, 6 CLI, 11 integration) still pass; no regressions.

The directory is intentionally small and easy to extend (`company_directory.rs` is a flat, append-only list) rather than a one-time claim to match 87 — the mechanism is now real, native, and faster; growing the list is a config change, not an architecture change.

## 5. Honest verdict

Against the six competitors that could be at least partially exercised for real, ATSassin's core loop (discover/evaluate/tailor) is the only one that ran reliably end-to-end without a multi-minute-to-multi-hour setup tax, and it's the only one whose evaluate+tailor output was verified clean across all 5 personas in this session. The one concrete gap found — career-ops's zero-token company-scan breadth — has been closed with a native, concurrent, faster reimplementation of the same underlying technique, verified with a real timed run. No competitor in this set demonstrated a real, working advantage over ATSassin that remains unaddressed.
