# ATSassin Test Strategy

**Status:** Active · **Date:** 2026-07-29
**Applies to:** all contributions from the next release onward

## Why this exists

ATSassin makes claims that ordinary unit tests cannot verify. "Runs on 4 GB CPU-only", "never fabricates data", "the recommendations are useful" are not assertions about a function's return value — they are assertions about a system's behaviour against the live internet and against real human outcomes.

At adoption, the suite had 105 tests concentrated away from the highest-risk code: `tracker.rs` (owns all user state), the privacy-sensitive egress path, `scorer.rs`, `matcher.rs`, and the entire `tailor → llm → router` chain had little or no direct coverage. Step 0 has since added deterministic egress, prompt-boundary, context-budget, and mock-transport refusal tests; the remaining module-level gaps below still govern new work.

This strategy defines five tiers, each testing something the tier below cannot.

---

## Tier 1 — Unit (deterministic, offline, CI-gated)

Standard `#[test]` coverage. Runs on every PR via `ci.yml:39`.

**Mandatory coverage before the next release.** These are ranked by consequence of failure, not by ease:

| Module | Why it is first | Minimum |
|---|---|---|
| `pipeline/tracker.rs`, `engine/feedback.rs` | Own shared user state; corruption is unrecoverable | Round-trip every entity; transactional migration rollback; lock-before-backup and backup/live-row preservation; transaction-spanning version guards; newer-schema refusal at every entry point; status transitions; upsert idempotency under ADR-001 |
| `engine/pii_scrubber.rs` | Privacy gate | International formats (E.164, SG/UK/IN/EU), not just NANP |
| `engine/egress.rs` / `engine/distillation.rs` | Shared training and prompt egress boundary | Gate ordering; PII/injection/overflow refusal before mock transport; no unvalidated request or file escapes the gate |
| `engine/scorer.rs` | Feeds every downstream decision | Parse failure returns `Err`, never a synthesised score |
| `engine/matcher.rs` | Unverified ranking input | Scoring monotonicity |

**Banned patterns.** A test asserting `result.is_ok() || result.is_err()` is not a test. Assertions that cannot fail must be removed, not tolerated.

**Property tests** where the invariant is clearer than any example — scoring monotonicity (more overlap never lowers the score), canonical-URL idempotency (`canon(canon(u)) == canon(u)`), flow-solver feasibility (never exceeds any capacity).

---

## Tier 2 — Contract tests against recorded fixtures (deterministic, offline, CI-gated)

The extraction ladder ([Layer 1](design/EVIDENCE_LAYER.md)) parses four wire formats that change without notice. Live-testing them in CI would be flaky and rude to the upstream hosts.

Record real responses once, commit them as fixtures, parse them in CI:

```
tests/fixtures/extraction/
  greenhouse_board.json        ashby_with_compensation.json
  lever_postings.json          next_data_hydration.html
  workday_jobs.json            jsonld_jobposting.html
  jsonld_missing_salary.html   greenhouse_empty.json
```

Every fixture must have a **degraded twin** — missing salary, absent `datePosted`, malformed JSON, empty array, HTTP 429 body. The degraded twin asserts the honest-failure path: `None`, not a substitute (ADR-002), and `Err`, not an empty `Ok` (ADR-003).

Fixtures are scrubbed of personal data before commit. Where a real posting names a hiring manager, that field is replaced with a synthetic value and the file notes it.

---

## Tier 3 — Live board-health canary (non-deterministic, scheduled, non-blocking)

Already exists (`board_health.yml`, daily cron) and genuinely asserts — it greps for `Found N jobs` across five boards and files a labelled issue on zero.

**Two upgrades required:**

**Tier-fallthrough as the drift signal.** Once the extraction ladder lands, "returned jobs" is too coarse. A source that normally resolves at Tier 2 (ATS JSON, with compensation) and silently degrades to Tier 4 (JSON-LD, often without) still returns jobs and still passes the current canary — while quietly losing the fields Layers 2 and 3 depend on. Track the per-source tier distribution and alert on distribution shift.

**Distinguish zero-from-failure.** `board_health.yml:58` uses `|| true`, so a crashed binary is indistinguishable from an empty board. Under ADR-003 these are different events and must be reported differently.

*Live evidence this matters:* in the 2026-07-29 trial below, `remoteok` and `weworkremotely` returned exactly 0 jobs for **all four** queries while `companies` returned 105. Under the current error-swallowing behaviour there is no way to tell from the output whether those boards are genuinely empty for these queries or silently failing. That ambiguity is the bug ADR-003 closes.

---

## Tier 3b — Profile-shape matrix (the agnosticism gate)

A tool claiming to serve everyone cannot be validated on one person. The 2026-07-29 trial ran a single profile shape — senior generalist, APAC, contract-seeking, anglophone — and that shape is close to the best case for several mechanisms in the architecture, which makes it the *weakest* possible evidence that they generalise.

**Every layer must be exercised across the matrix below before its acceptance criteria are considered met.** These are synthetic profiles (ADR-007); the five existing UAT personas are the starting set and are insufficient on their own.

| Axis | Shapes that must be covered | What it stresses |
|---|---|---|
| **Career stage** | early-career (one demonstrated skill), mid, senior generalist | Diversification cap must **loosen toward the budget** for early-career and specialists (concentrate), and tighten only where adjacency is genuinely broad. A cap of 1 forces maximum spread — the opposite of what these shapes need |
| **Adjacency structure** | generalist (high), licensed specialist (near-zero) | A specialist must not be pushed toward adjacent families |
| **Urgency regime** | selective (one considered move), throughput (income needed soon) | Both regimes must produce sensible slates; neither is moralised about |
| **Market** | US/W-EU, plus at least two of India, Japan, Brazil, Indonesia, Nigeria | Tier-2 ATS coverage collapses outside the West; tier 4 must carry it |
| **Language** | anglophone, plus at least one non-English posting corpus | Employment-type and seniority detection are English substring matches today |
| **Structural factors** | career gap (caregiving), older candidate, work-authorisation constrained | The dominant structural factor differs per user and must be attributable |
| **Employment type** | permanent, contract/fractional, part-time | Preference handling must not privilege the shape that was tested first |

**Failure to serve a shape is a finding, not an exclusion.** If the tool returns nothing useful for an early-career candidate in Jakarta, that is a tracked gap in the sourcing or scoring layer — not evidence that the user is out of scope. The mission is explicit that the only thing which should change per user is *what the tool discovers*, never *how the tool operates*.

**Minimum bar before any layer ships:** the layer behaves sensibly — not necessarily well, but sensibly and honestly — on every row above, with degenerate parameter values reachable where the shape calls for them.

## Tier 4 — Live outcome trial (non-deterministic, manual, the real test)

This is the tier that tests the product rather than the code. It runs against a real profile, real postings, and real applications, and it produces the **calibration baseline** that [Layer 2](design/CALIBRATION_LAYER.md) is fitted against.

It is not a pass/fail gate on a PR. It is a longitudinal instrument.

### Protocol

**Setup.** Real profile at `PROFILE_PATH`; preferences set to the target engagement type; a provider configured. Record the binary's git SHA and the provider/model, because a trial is only comparable to another trial run on the same code.

**Discovery.** Scan across at least four role framings, including at least two *adjacent* to the user's obvious title. This is not optional: the whole "unawareness" thesis predicts the obvious framings underperform, and the trial is where that prediction is tested rather than assumed.

**Record per posting, at application time:**

| Field | Why |
|---|---|
| `posted_at`, `applied_at` → latency in days | Primary controllable leading indicator |
| Tailoring depth (`edit_distance` band) | Primary controllable leading indicator |
| Evaluation score, extraction tier, source | Fit and provenance |
| Structural flags (self-employment gap, non-local name, career gap) | Attribution only — never actioned (REJ-008) |

**Preference provenance is recorded before the trial starts.** For each preference value — compensation floor, employment type, work mode, location — record whether it was user-supplied, extracted from the profile, or defaulted. A hard constraint holding a defaulted value invalidates the run (see #155 and the trial record below).

**Then wait.** Outcomes arrive over weeks via `atsassin outcomes sync` (IMAP). The trial is not complete when the applications are sent; it is complete when the transitions are observed.

### What counts as success

Success is **not** "the tool found jobs". It is calibrated against the published funnel priors:

| Metric | Prior baseline | Trial target |
|---|---|---|
| Application → callback (deeply tailored, <7d) | 8–15% | Within or above interval |
| Application → callback (generic, >14d) | 1–3% | Control arm |
| Median submission latency | — | <7 days on selected slate |
| Duplicate rate in top-20 slate | — | **0%** (ADR-001) |
| Fabricated-field rate | — | **0%** (ADR-002) |

The generic/late arm is deliberate. Without a control, a good callback rate is unattributable — it could be the tool, the market, or the candidate. The prior research gives a 5× expected separation between deep-tailored-early and generic-late; reproducing that separation locally is what validates the leading-indicator model that Layer 3 optimises against.

**Sample-size honesty.** A trial of 12 applications cannot distinguish 8% from 15%. Trial reports state `n` and the posterior interval, and say when the result is prior-dominated. A trial that reports a bare percentage has failed its own standard (ADR-005).

---

## Tier 5 — Hardware-floor validation (manual, release-gating)

The "4 GB, CPU-only" claim has never been tested on 4 GB CPU-only hardware (live issue **#73**; #5 and #57 closed as duplicates). It is currently the largest unbacked claim in the project, and it is load-bearing for the mission.

Until validated, documentation must say **target**, not **verified** — the current `README`/`ROADMAP` phrasing and `PROMOTION_DRAFTS.md` overstate it.

Protocol: a real or VM-limited 4 GB CPU-only box; `cargo build --release`; `profile init` → `roles infer` → `scan` → `evaluate` → `tailor`; record peak RSS and wall-clock per stage; pass requires completion without OOM and without swap thrash.

---

## Trial record — 2026-07-29, contract roles

**Configuration.** Binary `target/release/atsassin.exe` at 10.96 MB; provider Groq `llama-3.3-70b-versatile`; preferences `ContractOnly` / `RemoteOnly`; profile shape: senior go-to-market leader, APAC, ~25 years, currently self-employed.

> **PII rule for all trial records.** Trials run against a real profile; the record does not carry it. Never commit a subject's name, employers, compensation figures, or contact details to this repo or to the issue tracker — describe the profile by *shape* only (seniority, function, region, years, employment status), because that is all the reader needs to judge whether a finding generalises. This applies to issue bodies and comments as much as to docs.

**⚠️ The trial began with a defaulted compensation floor, which produced defect #155.** The floor in force was USD 150,000, several times below the correct figure for the subject. The source was `cli.rs:1715`, which prints `--min-comp 150000` as a help-text example. **A placeholder in a help string had become a load-bearing filter.** The profile contained no compensation data, `profile_parser.rs` has no compensation extraction at all, and onboarding never asks — so no path existed by which the tool could have known.

Re-running the identical pool at a corrected floor moved three of the top eight, and two roles previously presented as strong matches fell below it.

The general lesson, which is the reason this is in the protocol rather than just in the defect: **the compensation floor is a hard constraint in the allocation layer**, so a wrong floor silently distorts the entire slate while presenting itself as the user's considered preference. Every trial begins by verifying that preferences reflect the subject's real situation, and records the provenance of each preference value. **A defaulted value sitting in a hard constraint is a finding, not a setting.**

Figures below are from the corrected run unless stated.

### Discovery — the adjacent-framing effect reproduced

Identical 44-company sweep, six role framings:

| Query | Jobs found |
|---|---|
| Fractional Chief Revenue Officer | **0** |
| Sales Director | **0** |
| Country Manager | **0** |
| Go To Market | **0** |
| Revenue Operations | 1 |
| Partnerships | 21 |
| **Program Manager** | **46** |
| Business Development | 43 |

The four "obvious" senior-GTM titles returned nothing. The adjacent framings returned 110. This is the founding-trial insight reproduced under controlled conditions, and it is the empirical basis for the diversification constraint in [Layer 3](design/ALLOCATION_LAYER.md). **A tool that ranks what the user searched for would have returned an empty result here.**

### Pool

105 jobs persisted across `companies`, `remoteok`, `weworkremotely`. `remoteok` and `weworkremotely` returned 0 on every query — see Tier 3 note above.

### Contract-suitable roles surfaced

| Score | Role | Fit | Age |
|---|---|---|---|
| 76 | Fractional Head of Business Development · PHARMExcel · UK | 85% LLM | 8d |
| 61 | Business Development Manager · PolyAI · US | 85% LLM | unknown |
| 60 | Program Manager, Professional Services · Airtable · Remote-US | 70% LLM | unknown |
| 55 | Country Manager, Singapore · Notion · SG | 85% LLM | 4d |
| 54 | Program Manager, Community (Contract) · Airtable · Remote-US | 60% LLM | unknown |
| 43 | VP, Revenue Operations · Later · Remote | 37% lexical | unknown |

The top result is a genuine fractional engagement matching the stated preference, found via an adjacent query, 8 days old and still inside the review-bandwidth window.

### Evaluation output — and a structural-bias finding

Three distinct roles evaluated end-to-end with real LLM calls. All returned grounded, non-fabricated assessments:

| Role | Score | Grade |
|---|---|---|
| Sales Director · Automation Anywhere · Singapore | 0.85 | B+ |
| Enterprise Sales Director · Kinaxis · Singapore | 0.85 | B+ |
| VP, Revenue Operations · Later · Remote | 0.80 | B+ |

**Two of the three independently penalised the candidate for being a founder**, listing under *gaps*:

> "his frequent job changes and short tenure in some roles may require further discussion"
> "Current role as a founder of a startup may indicate a lack of long-term commitment"

This is the prior-self-employment screening penalty from the correspondence-study literature — where candidates transitioning from self-employment receive callbacks in under 1% of applications against 6% for identically-qualified employed candidates — **reproduced inside ATSassin's own evaluation output**.

The finding matters because of where the tool currently files it. It is reported as a **candidate gap**, alongside genuine skill gaps like "limited RPA experience". Under [ADR-005](DECISIONS.md#adr-005--conversion-rates-are-per-user-posteriors-not-displayed-benchmarks) that classification is wrong and harmful: founder status is a *structural* factor, not a controllable one. Presenting it as a gap implies the candidate should fix it, when the only "fixes" are concealment — permanently out of scope under [REJ-008](DECISIONS.md#rej-008--acting-on-structural-bias-data-as-advice).

The correct behaviour, once Layer 2 lands, is to move it out of `gaps` and into a structural attribution: *this is a documented screening effect, it depresses your callback rate, and it is not a deficiency in your candidacy.*

This is now a required test case: **an evaluation must never list a structural factor under `gaps`.**

### Defects reproduced live

**P0-2 duplication, in the user-facing slate.** Of the top 20 recommendations, **8 slots were duplicates**: Airtable "Professional Services – West" appeared 3×, "– East" 3×, amplemarket "Senior Revenue Operations Manager" 2×, Airtable "Community (Contract)" 2×. Each duplicate carries a distinct random UUID, so the tool cannot tell them apart. This is ADR-001 observed end-to-end: 40% of the user's decision surface was noise.

**Tier collapse.** Startup logged `Synced tier models to default: llama-3.3-70b-versatile`, confirming `config.rs:422` collapses light/balanced/full to one model regardless of `--preset`.

**Honest-failure held where it was implemented.** Zero-result queries reported honestly with no fabricated postings, and `posted date unknown` was rendered truthfully rather than defaulted — the Greenhouse path does not fabricate dates. The fabrication is confined to the paths ADR-002 names.

### Baseline established — and what this trial does NOT establish

This trial is the **calibration baseline**: prior-dominated, `n` small, no outcome data yet. Applications submitted from this slate and tracked through `outcomes sync` become the first real observations for [Layer 2](design/CALIBRATION_LAYER.md). Until those outcomes arrive, every conversion figure the tool reports must be labelled prior-dominated.

Stated plainly, because the trial is persuasive in ways it has not earned:

- **It is one profile shape.** Senior generalist, APAC, contract-seeking, anglophone. See the Tier 3b matrix — the mechanisms this trial appears to support are the ones this shape is best-positioned to benefit from.
- **The adjacent-framing result is an illustration, not a validation.** A 25-year generalist is close to the best possible case for adjacency. A licensed specialist would likely show the opposite, correctly.
- **It produced zero outcome data.** The trial is *initiated*, not complete. Tier 4 completes when transitions are observed over the following weeks, not when the applications are sent. Any claim that the tool "works" on the strength of this run is unsupported.
- **Two boards returned zero on every query** and, under current error-swallowing (#145), cannot be distinguished from silent failure. Part of the pool may be missing.
- **The compensation floor was defaulted, not chosen**, until corrected mid-trial (#155).

What the trial *does* establish, because these were observed directly rather than inferred: the duplication defect (#142), the tier-collapse defect, the structural-factor misclassification (#151), and that the end-to-end pipeline runs without crashing against live sources.

---

## Contributor checklist

Before opening a PR:

- [ ] Tier 1 tests for new logic; no tautological assertions
- [ ] Tier 2 fixture **and degraded twin** for any new extraction path
- [ ] No new `Utc::now()` as a data value (ADR-002)
- [ ] No new `.unwrap_or_default()` on a fallible source call (ADR-003)
- [ ] Any user-facing rate carries an interval and `prior_dominated` (ADR-005)
- [ ] `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass
