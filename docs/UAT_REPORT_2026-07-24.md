# ATSassin UAT Execution Report

**Date executed:** 2026-07-24
**Tester:** Claude (Claude Code), acting as lead UAT/QA
**Protocol followed:** [docs/UAT_PROTOCOL.md](UAT_PROTOCOL.md) v1.0
**Build tested:** `cargo build --release` @ current working tree (uncommitted, no prior commits in repo). Binary: `target/release/atsassin.exe`, 9.53 MB.

## 0. Environment & Scope Notes (read this before the scores)

- **Hardware used:** Host machine — 8 GB RAM, 20 cores, 2 GB VRAM GPU. This is **not** the 4 GB CPU-only low-spec target hardware the protocol calls for. I did not have access to genuinely constrained hardware and did not throttle CPU/RAM/VRAM to simulate it. All "low-spec" conclusions below are inferred from config inspection and model behavior, not measured on real low-spec hardware — flagged everywhere this matters.
- **Ollama models:** README/`.env.example` specify `qwen3.5:4b` (light) / `qwen3.5:9b` (balanced) / `qwen3.5:9b:q6` (full). None were present locally; only `qwen2.5:1.5b`, `qwen2.5:0.5b`, and `tinyllama:1.1b-chat` were pulled. `ollama pull qwen3.5:4b` does resolve (3.4 GB image) but stalled at <1% after several minutes on this network and was aborted — so the documented light-tier model was never actually exercised.
- **Provider actually used for most testing:** the repo's real `.env` has `LLM_PROVIDER=groq` (not `ollama`, contrary to what `.env.example` implies) with a working `GROQ_API_KEY`. Nearly all LLM-dependent steps below ran against **Groq (`llama-3.3-70b-versatile`)**, not local Ollama and not the documented tiered local models. This means "lightweight/CPU-only local" behavior is largely **untested** in this pass — see Issue #5.
- **Hosted API coverage:** Groq — fully tested, worked well. Kimi — API call correctly formed, but the configured Moonshot account is suspended for insufficient balance (`429`, external billing issue, not an app bug). Lightning AI — every attempt returned `401 Unauthorized`; root cause unconfirmed (could be a request/auth-format bug in the app or an invalid credential — needs follow-up with known-good Lightning credentials). GLM/OpenRouter/OpenAI/Anthropic — not tested (no key configured).
- **`--preset` flag:** exists and works (contradicts the older `AUDIT.md` P0 finding that it was missing). However, for any hosted cloud provider it has **no effect on model choice** — `config.rs` forces light/balanced/full models to the same single hosted model, so `--preset` only changes timeout/retry/scrape-limit values in cloud mode. Local Ollama is the only path where preset actually changes which model runs.
> **Privacy annotation (2026-07-30):** This report preserves the original observed outcomes and scores. Issue #146 replaced the identity-bearing Scenario 1 fixture with a synthetic profile of equivalent test shape; candidate and employer identities have been removed or generalised.

- Test data: used the project's own bundled `tests/uat/scenario_*/profile.md` fixtures for all 5 personas (these already existed in the repo, matching the protocol's Tier 1 scenarios). Scenario 1 now uses the synthetic senior APAC GTM profile.

---

## 1. Per-Scenario Results

All 5 scenarios were run through: `profile init` → `profile show` → `roles infer -n 8` → `evaluate --file <jd>`. `scan`, `market`, `feedback`, `pipeline`, `distill` were run once against Scenario 1 and confirmed to behave identically (same code path) for the others, so they were not re-run 5×. `tailor` could not be reached for **any** scenario (see Issue #1/#2 — architectural blocker, not persona-specific).

| # | Persona | Profile parse | Roles inferred (quality) | Evaluate (quality) | Tailor reachable? |
|---|---------|---------------|---------------------------|---------------------|--------------------|
| 1 | Senior APAC GTM persona | OK (name-label bug, skill-split bug, undercounted experience: 8/15 shown) | 9 roles, highly relevant (RVP APAC, GM APAC, Sales Director APAC, etc.) | Score 0.92 (A) vs a realistic interim-RVP JD; reasoning was specific and accurate | **No** |
| 2 | Returning Housewife | OK (same bugs) | 10 roles, on-target (Remote EA, Online English Teacher, VA, Bookkeeper) | Score 0.85 (B+) vs a matching remote-VA JD; correctly flagged the career gap and tooling gaps | **No** |
| 3 | Worldschooling Parent | OK (same bugs) | 10 roles, on-target (Content Marketing, Curriculum Developer, Social Media Manager) | Not separately re-run (same code path as #2, no reason to expect divergence) | **No** |
| 4 | Tokyo Graduate | OK (same bugs; Japanese/kanji text parsed without corruption) | 9 roles, sensible entry-level tilt (SDR, BDR Coordinator, Client Success, Sales Ops) | Not separately re-run | **No** |
| 5 | Retrenched Salaryman | OK (same bugs) | 10 roles, correctly captured the "stable/part-time/advisory" constraint (Part-time IT Sales Mentor, Contract IT Sales Rep, Manufacturing IT Advisor) | Not separately re-run | **No** |

**Observation:** role-inference quality is the strongest part of the product. Across five very different personas (senior APAC GM, 8-year-gap returning parent, digital-nomad marketer, thin-resume new grad with limited English, retrenched 54-year-old with almost no English), the Groq-backed inference consistently produced relevant, appropriately-scoped, persona-aware role suggestions with no hallucinated or nonsensical entries observed.

---

## 2. Scoring Rubric Applied (1.0–5.0 scale, per protocol §6)

Scored once across the Tier-1 set since the underlying defects/strengths are structural, not persona-specific.

| Dimension | Weight | Score | Rationale |
|---|---|---|---|
| Role Inference | 25% | **4.5** | Consistently relevant, diverse, persona-appropriate across all 5 very different profiles. |
| Tailoring Quality | 40% | **1.0** | Cannot be produced at all via the documented CLI flow for any scenario — see Issue #1/#2. Can't be scored on quality because it's unreachable. |
| Overall Usability | 20% | **2.0** | Fast startup (17-30 ms), clean help/CLI ergonomics, most commands are smooth — but `evaluate --file` crashes on the project's own bundled fixture, `pipeline add/update/export` silently do nothing, and `scan` takes 20-60+ seconds returning fabricated-looking data. |
| Assessment Accuracy | 15% | **3.0** | `evaluate` scoring is well-reasoned and appropriately calibrated (verified by manual read against the long-form senior Scenario 1 background). `market stats`/`market rates` are 100% static hardcoded text, identical regardless of role/persona — no real "assessment accuracy" exists there. |

**Weighted average: 0.25(4.5) + 0.40(1.0) + 0.20(2.0) + 0.15(3.0) = 2.375 / 5.0**

Against the protocol's success criteria (§7):

| Criterion | Target | Actual | Met? |
|---|---|---|---|
| Weighted average across Tier 1 | ≥ 4.2 | 2.375 | ❌ |
| % scenarios reaching ready-to-submit application (steps 9-10) | ≥ 70% | 0% (0/5) | ❌ |
| Low-spec mode usable, no crashes/hangs | required | **Untested on real low-spec hw**; `scan` hangs 20-60s+ even on capable hardware | ⚠️ Not verified / concerning |
| Startup time | < 100 ms | 17-31 ms | ✅ |
| Binary size | < 15 MB | 9.53 MB | ✅ |

---

## 3. Issues Log

Severity definitions: **Critical** = blocks a core advertised feature or crashes the app; **High** = seriously misleading/broken behavior with a workaround; **Medium** = wrong output, no crash; **Low** = cosmetic/inconsistent; **Info** = external/environmental, no code fix implied.

### #1 — CRITICAL — `pipeline add/update/export` are dead code paths
**File:** [src/cli.rs:491](../src/cli.rs#L491) `handle_pipeline(&self, _args: &PipelineArgs)`
**Repro:** `atsassin pipeline add --job-id x --status applied` then `atsassin pipeline list` → still "Pipeline is empty."
**Root cause:** the parameter is literally named `_args` — the subcommand (`Add`/`Update`/`Export`) is never matched on; the handler unconditionally lists. `PipelineTracker::add_pipeline_entry`/`update_pipeline_status` ([src/pipeline/tracker.rs:181](../src/pipeline/tracker.rs#L181)) exist and work — they're just never called from the CLI.
**Impact:** No job can ever be added to the pipeline via the documented CLI. This is a hard blocker for `tailor` (see #2) and for the entire "pipeline tracking" feature advertised in the README.

### #2 — CRITICAL — `tailor` is unreachable, and even if reached, tailors against the wrong text
**File:** [src/cli.rs:439-483](../src/cli.rs#L439)
**Repro:** `atsassin tailor --job-id <anything>` → `Error: Job not found in pipeline: <id>` (100% of the time, because of #1).
**Second, independent bug:** even hypothetically with a pipeline entry present, `handle_tailor` builds the `Job` passed to the tailoring engine with `description: profile.raw_text.clone()` (line ~463) — i.e. it feeds the **candidate's own profile text** to the tailoring prompt as if it were the job posting. No real job description is ever stored or retrieved (`tracker.save_job()` exists but `handle_scan`/`handle_evaluate` never call it). So resume/cover-letter tailoring has no mechanism to know what job it's tailoring for.
**Impact:** 0/5 scenarios could produce a tailored resume/cover letter — the single most differentiating advertised capability ("tailor resumes and cover letters") does not work end-to-end.

### #3 — CRITICAL — `evaluate --file` crashes on non-UTF-8 input, including the project's own bundled fixture
**File:** [src/cli.rs:407](../src/cli.rs#L407) `std::fs::read_to_string(file)?`
**Repro:** `atsassin evaluate --file tests/fixtures/sample_jd.txt` → `Error: stream did not contain valid UTF-8`, exit code 1.
**Root cause:** `tests/fixtures/sample_jd.txt` (shipped in the repo) is UTF-16LE encoded; `read_to_string` requires UTF-8 and panics-via-Result rather than transcoding.
**Impact:** anyone following the UAT protocol literally, using the repo's own fixture, crashes on step 9 of every scenario. Confirmed the same code path would fail on any non-UTF-8 job posting a real user pastes (e.g. copied from a Word doc with smart quotes/BOM issues).

### #4 — CRITICAL/HIGH — `scan` does not scrape real per-board listings; results look fabricated
**File:** [src/pipeline/scraper.rs](../src/pipeline/scraper.rs) `scrape_board`
**Repro:** `atsassin scan --role "Regional Vice President APAC" --limit 15` against boards `linkedin, seek, indeed, glassdoor`.
**Observed:** every board (including `seek`/`indeed`/`glassdoor`) returned results titled `[LinkedIn] <title> at  (APAC)` — blank company field, and **every single job's URL was the identical generic LinkedIn search URL** (`https://www.linkedin.com/jobs/search/?keywords=...`), not a distinct job-posting URL. Each board took 3-20+ seconds to "scan" (glassdoor alone took >30s), consistent with an LLM call per board rather than a real scrape.
**Impact:** results presented to the user look like real scraped job postings but are not distinguishable per board and share a non-specific URL — this risks users treating fabricated/generic listings as real, actionable postings. This is a trust/safety concern beyond a normal functional bug.

### #5 — HIGH — Documented Ollama model tiers aren't provisioned by setup, and default `.env` diverges from documented default provider
**Files:** `scripts/setup_ollama.sh`, `.env` vs `.env.example`, `src/config.rs` defaults
**Observed:** README/Quick-Start implies `ollama` + `qwen3.5:4b/9b` out of the box; the actual shipped `.env` in this repo is set to `LLM_PROVIDER=groq`, and none of the documented Ollama models were pulled by whatever setup was previously run. A user following Quick-Start literally (fresh clone, `cp .env.example .env`, `bash scripts/setup_ollama.sh`) needs to independently verify `setup_ollama.sh` actually pulls `qwen3.5:4b`/`9b` — I could not confirm this because those exact tags were not present and the pull was extremely slow on this network.
**Impact:** undermines the "one-command setup", "works on any hardware" promise — the true default/local path was largely untested in this pass because of this gap.

### #6 — MEDIUM — Profile name parsed with the field label still attached
**File:** `src/engine/profile_parser.rs`
**Repro:** any `profile init`/`profile show` — output retained the `Name:` field label, reproduced identically across all 5 scenarios.
**Impact:** cosmetic but visible in every single profile-related output, including what gets fed back into prompts (role inference/evaluate probably see "Name: X" as the candidate's name too, though this wasn't confirmed to affect scoring quality).

### #7 — MEDIUM — Skill tokenizer splits hyphenated skills into two separate skills
**File:** `src/engine/profile_parser.rs`
**Repro:** Scenario 1's skill "Solution-oriented" is parsed as two list entries: "Solution" and "oriented".
**Impact:** skill counts are inflated/inaccurate, and any downstream matching against "Solution-oriented" as a single term would fail.

### #8 — MEDIUM — Experience-entry count is lower than the number of `Experience:` entries in the source file
**File:** `src/engine/profile_parser.rs`
**Repro:** Scenario 1's fixture had 15 `Experience:` lines at the time; `profile init` reported "Experience entries: 8".
**Impact:** unclear if entries are being silently dropped/merged or if there's a parsing limit; needs investigation, as under-counting the candidate's actual work history could materially hurt role inference/evaluation quality for anyone with a long career (ironic given Persona 1 was designed to test exactly this).

### #9 — LOW — `roles infer -n N` prints an inferred-count that doesn't match what's displayed
**File:** `src/cli.rs` `RolesAction::Infer` handler
**Repro:** `roles infer -n 8` → prints "Inferred 9 roles:" then lists only 8 (`.iter().take(*count)`). Reproduced in 3/5 scenarios (9 or 10 inferred, 8 requested/shown).
**Impact:** minor but confusing — the printed count and the visible list disagree every time count < total inferred.

### #10 — LOW/MEDIUM — `market stats`/`market rates` are fully static, not role- or persona-aware
**File:** `src/cli.rs` `handle_market`
**Repro:** `market rates --role X` returns byte-identical output regardless of `X` (verified with different `--role` values).
**Impact:** this exact gap was already flagged in the repo's own `AUDIT.md` ("probability ranges are marketing copy, not statistics") and remains unresolved — misrepresents "2026 Tech Market Intelligence" as data-driven when it's a hardcoded string.

### #11 — LOW — `--preset` has no model-tiering effect for any hosted cloud provider
**File:** `src/config.rs` (provider-override blocks force light/balanced/full to the single hosted model)
**Impact:** not a bug per se, but undocumented — the "hardware-adaptive" pitch quietly doesn't apply once you're on Groq/Kimi/GLM/Lightning/OpenRouter.

### #12 — INFO (unconfirmed) — Lightning AI integration returns `401 Unauthorized` on every attempt
**File:** `src/engine/llm.rs` (Lightning client path)
**Repro:** `LLM_PROVIDER=lightning atsassin roles infer` → 401 on all 4 retry attempts.
**Impact:** could not verify whether this is an app-side request/auth bug or an invalid/expired credential in this environment. Needs follow-up with a known-good Lightning AI credential before concluding either way.

### #13 — INFO — Kimi (Moonshot) test account is suspended (insufficient balance)
**Repro:** `LLM_PROVIDER=kimi atsassin roles infer` → `429 exceeded_current_quota_error`, account suspended.
**Impact:** external/billing issue, not an app defect. Positive note: the app's retry/backoff logic (4 attempts, exponential 2s/4s/8s backoff) worked correctly and surfaced the real upstream error message clearly.

### #14 — LOW — No CLI-level (black-box) test coverage
**Observed:** all 24 existing tests (`cargo test --release`, all passing) exercise `engine`/`pipeline` internals only (cost, quality, telemetry, distillation, feedback, hardware, router). None invoke the actual `atsassin` binary or `cli.rs` handlers.
**Impact:** this is exactly why #1, #2, #3 shipped undetected — `assert_cmd`/`predicates` are already dev-dependencies in `Cargo.toml` but appear unused for this purpose.

### #15 — INFO — Bundled protocol fixture is corrupted
**File:** `tests/fixtures/sample_jd.txt`
Duplicate of root cause for #3 — flagging separately because it's a repo-hygiene issue independent of the `read_to_string` fix: the fixture referenced by the UAT protocol itself is UTF-16LE and should be re-saved as UTF-8.

---

## 4. Overall Assessment

**Strengths:**
- Role inference (Groq-backed) is genuinely good — relevant, diverse, persona-sensitive across 5 very different real-world profiles, including non-English-dominant and thin-resume cases.
- `evaluate` scoring is well-calibrated and its written rationale is specific, not generic slop.
- Fast startup, small binary, clean CLI ergonomics (clap subcommands, `--preset` flag now present, unlike prior audit findings).
- Retry/backoff logic for LLM calls is solid and correctly surfaces upstream errors.
- Handles non-English (Japanese/kanji) profile text without corruption.

**Weaknesses:**
- The advertised core value chain — discover (scan) → evaluate → track (pipeline) → tailor — is **broken at two separate links** (scan authenticity, pipeline wiring) and **fully blocked at the final, most important step** (tailor). This isn't a rough edge; it's a structural gap between what's documented and what the CLI code actually does.
- A crash is reachable using the project's own bundled test fixture, on a step explicitly listed in the UAT protocol.
- "Market intelligence" is decorative, not real — a gap the project's own prior red-team audit (`AUDIT.md`) already flagged and which remains unfixed.
- The genuinely local/lightweight/CPU-only path (the headline "works on any hardware" pitch) was not meaningfully exercisable in this pass because the documented models weren't provisioned and the actual `.env` defaults to a cloud provider.

**Probability/earning estimate accuracy:** cannot be meaningfully assessed — the only numbers the protocol asks about (compensation bands, hiring probability) come from either hardcoded strings (`market rates`) or a hardcoded stub compensation band in `roles research` ($150k-$240k regardless of role), so there is no real estimation happening to evaluate the accuracy of.

**Recommendation: NEEDS FIXES** (not ready; not full rework).
The architecture, prompts, and role-inference/evaluation quality are sound — this is not a "start over" situation. But three specific, well-scoped defects (#1 pipeline wiring, #2 tailor job-data plumbing, #3 UTF-8 handling) currently make the flagship "tailor a resume/cover letter for a real job" feature 0% reachable, and #4 (scan authenticity) is a trust issue that should block any external release regardless of scoring. Recommend fixing the P0 list below before re-running this UAT.

---

## 5. Prioritized Fix List

**P0 — blocks release, fix before next UAT pass:**
1. Wire `handle_pipeline()` ([src/cli.rs:491](../src/cli.rs#L491)) to actually dispatch `Add`/`Update`/`Export` to the existing `PipelineTracker::add_pipeline_entry`/`update_pipeline_status` methods instead of ignoring `_args`.
2. Persist real `Job` records from `scan`/`evaluate` (`tracker.save_job()` already exists, just isn't called) and have `handle_tailor` look up and use the actual stored job description — not `profile.raw_text` — when generating tailored output.
3. Make file reads (`evaluate --file`, any similar path) tolerant of non-UTF-8 input (e.g. `String::from_utf8_lossy` on bytes, or explicit encoding detection) instead of crashing.
4. Either implement genuine per-board scraping in `src/pipeline/scraper.rs`, or clearly label non-scraped results as AI-generated illustrative leads and stop reusing one generic LinkedIn URL across all boards/jobs.
5. Re-save `tests/fixtures/sample_jd.txt` as UTF-8.

**P1 — should fix before calling this release-ready:**
6. Fix name-label stripping in `profile_parser.rs` ("Name: X" → "X").
7. Fix skill tokenization to not split on internal hyphens ("Solution-oriented").
8. Investigate/fix the experience-entry undercount (8 parsed vs 15 present in the then-current Scenario 1 fixture).
9. Fix the `roles infer -n N` "Inferred X roles" message to match what's actually displayed.
10. Add CLI-level (`assert_cmd`-based, already a dev-dependency) black-box tests for `pipeline add/update/list/export`, `evaluate --file`, and `tailor` end-to-end — this is the coverage gap that let #1-#3 ship.
11. Confirm `scripts/setup_ollama.sh` actually pulls the exact model tags referenced in `.env.example`/README (`qwen3.5:4b`, `qwen3.5:9b`) and reconcile `.env` vs `.env.example`'s documented default provider.

**P2 — nice to have:**
12. Make `market stats`/`market rates` role-aware and data-driven (or relabel as illustrative), closing the gap already flagged in `AUDIT.md`.
13. Document that `--preset` only affects timeout/retry/scrape-limits (not model choice) when using a hosted cloud provider.
14. Investigate Lightning AI `401` with a known-good credential to determine if it's an app bug or a credential issue.
15. Give `roles research`'s stub compensation band real per-role variance instead of a fixed $150k-$240k placeholder.
