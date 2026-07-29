# Design-Gap Audit — Crowdsourcing, Career Coach & Thought Leadership

**Scope:** Red-team the current ATSassin design against three new requirements:

1. Crowd-sourcing of roles, salaries, and post reviews.
2. Continual polling / career-coach mode that keeps users ahead of the market.
3. A completeness/integrity/thought-leadership audit that keeps users one step ahead of AI automation.

**Companion docs:** [ROADMAP.md](ROADMAP.md) · [DESIGN_autonomous_loop.md](DESIGN_autonomous_loop.md) · [CRITICAL_CHAIN_PLAN.md](CRITICAL_CHAIN_PLAN.md) · [CATEGORY_LEADERSHIP_ROADMAP.md](CATEGORY_LEADERSHIP_ROADMAP.md)

---

## 1. Completeness gaps

### 1.1 No PII scrubber is implemented for shared artifacts

**Requirement affected:** #1 (crowd-sourcing). If salary/review/role data is pooled, a user's current employer, location, and salary can be inferred from raw submissions.

**Current state:** `src/engine/pii_scrubber.rs` exists as an untracked stub but is not wired into any export path.

**Gap severity:** High. Without a proven, test-covered scrubber, the community feed cannot ship.

**Fix:** Make the PII scrubber a blocking acceptance criterion for any crowd-sourcing issue. Add fixtures that include synthetic resumes and job descriptions; assert names, emails, company names, and addresses are removed or tokenized.

### 1.2 No continuous market-watch daemon exists

**Requirement affected:** #2 (career coach). The coaching loop is described in `ROADMAP.md` but no daemon or scheduler currently runs scans in the background.

**Current state:** `atsassin daemon` is documented in `DESIGN_autonomous_loop.md` as a Phase 3, hardware-gated optional component. `atsassin scan` is still purely on-demand.

**Gap severity:** Medium. The feature can be prototyped with cron/Task Scheduler, but the project needs an explicit path from "scheduled CLI" to "lightweight daemon."

**Fix:** Add `atsassin daemon --once` and document cron-based scheduling first. Only add a resident daemon after the one-shot path proves itself on real user hardware.

### 1.3 No market-data store for preference-challenge insights

**Requirement affected:** #2. To say "relocating yields +40%," ATSassin needs a durable store of market-wide compensation/role/demand data, not just per-job LLM estimates.

**Current state:** `market rates` and `roles research` are LLM-derived and static/illustrative per `ROADMAP.md` gap #3.

**Gap severity:** High. Career-coach insights will be hallucinations until a real, periodically-updated market dataset exists.

**Fix:** Build a `market_data` table (or external data feed) sourced from aggregated, anonymized public postings and optionally from the crowd-sourcing layer. Flag derived values as estimates until corroborated.

### 1.4 No AI-replacement risk analyzer

**Requirement affected:** #3 (thought leadership). Keeping users ahead of AI automation requires an explicit model of which skills/roles are most exposed.

**Current state:** Not mentioned in any design doc.

**Gap severity:** Medium. This can be a derived analysis over market data and public research (e.g., papers/O*NET automation scores), not a new ML model.

**Fix:** Add a new `AIExposureAnalyzer` that scores user skills against automation-risk indices and surfaces adjacent, lower-risk roles.

### 1.5 No corroboration/reputation layer for crowd-sourced data

**Requirement affected:** #1. A single user can submit a fake salary or a misleading "avoid this post" flag.

**Current state:** The LoRA-sharing design includes Proof-of-Quality but does not generalize it to arbitrary crowd-sourced data.

**Gap severity:** High. Without corroboration, the salary/review feed is easy to poison.

**Fix:** Extend the Proof-of-Quality reputation layer to crowd-sourced data points. Require N independent corroborations or a public-source match before a data point is used in coaching insights.

---

## 2. Consistency gaps

### 2.1 "Opt-in-only" is repeated but not enforced uniformly

**Observation:** Every new feature claims opt-in, but there is no single checklist or test gate ensuring a new feature is actually opt-in, can be disabled, and degrades gracefully.

**Fix:** Add a design-review checklist to `CONTRIBUTING.md`:

- [ ] Feature is off by default.
- [ ] There is a config flag to disable it entirely.
- [ ] Disabling it does not break core CLI/TUI workflows.
- [ ] It respects `HardwareProfile` tiers where applicable.

### 2.2 Career-coach and crowd-sourcing overlap on salary data

**Observation:** Both requirements produce/consume salary/review data. If not unified, the project will end up with two inconsistent data models.

**Fix:** Define a single `MarketSignal` schema used by the coaching loop, the crowd-sourcing feed, and the market-data module.

### 2.3 "Local-first" vs. community pooling tension is unresolved

**Observation:** Crowd-sourcing implies sharing some data externally, which can conflict with the local-first brand if not messaged carefully.

**Fix:** Document precisely what is pooled (anonymized board URLs, salary ranges, post reviews) and what is not (resumes, application materials, pipeline status). Make this a top-level FAQ.

---

## 3. Thought-leadership / positioning gaps

### 3.1 No public stance on AI automation

**Observation:** The project is positioned as a tool for job search, not as a strategic career-defense tool. Requirement #3 calls for thought leadership on staying ahead of AI.

**Fix:** Add a `docs/AI_DEFENSE.md` or section in `README.md` explaining how ATSassin helps users identify automation-resistant skills and pivot early.

### 3.2 "Greater good" mission is present but not concrete for contributors

**Observation:** The mission is aspirational. Potential contributors may not know what "greater good" means technically.

**Fix:** Add a "Why contribute?" section with concrete, measurable goals: keeping tooling free, preventing PII lock-in, and democratizing access to market intelligence.

---

## 4. Ethical / guardrail gaps

| Concern | Current state | Needed before shipping |
|---|---|---|
| PII in pooled data | Stub scrubber | Tested scrubber + review of every exported field |
| Poisoned salary/review feed | None | Corroboration + reputation thresholds |
| Resource drain from continuous polling | Daemon design exists but not implemented | Rate limiting, idle detection, hardware-tier gating |
| False coaching insights | No market-data source | Corroborated market dataset + confidence labels |
| Auto-decision overreach | Principle exists | UI/CLI design review for every coaching feature |

---

## 5. Recommended issue split

These issue numbers are proposed; they should be filed (or renumbered) in GitHub as work begins.

### Parallel-track notes

Most of the gaps below can be worked on in parallel. The only hard dependency is **PII scrubbing (#55)**: any issue that exports or pools data must wait for #55 to be implemented and tested. The other tracks are independent:

- **Market signal schema (#56)** and **AI exposure analyzer (#59)** can proceed in parallel.
- **Continuous market-watch daemon (#57)** and **corroborated market-data store (#58)** can be prototyped together, but a cron-based one-shot implementation can ship before the resident daemon.
- **Proof-of-Quality extension (#60)** depends on the LoRA registry work (#47 from `CRITICAL_CHAIN_PLAN.md`) and the market-signal schema (#56), but not on the daemon or the AI exposure analyzer.
- **Opt-in / crowd-sourcing FAQ (#61)** can be drafted immediately and refined as the other tracks land.

See `CONTRIBUTING.md` §Active parallel tracks for the label-to-track mapping and `CODEOWNERS` for area ownership.

| Proposed issue | Title | Labels | Depends on |
|---|---|---|---|
| #55 | Implement and test PII scrubber for all shared/crowd-sourced exports | `design:privacy`, `good first issue` | — |
| #56 | Define `MarketSignal` schema for salary/review/role pooling | `design:data-model` | — |
| #57 | Add continuous market-watch daemon (one-shot first, resident later) | `feature`, `help wanted` | #56 |
| #58 | Build corroborated market-data store for coaching insights | `feature` | #56 |
| #59 | AI exposure analyzer — flag automation-vulnerable skills | `feature`, `good first issue` | #58 |
| #60 | Extend Proof-of-Quality reputation to crowd-sourced data | `design:autonomous-loop` | #47, #55, #56 |
| #61 | Document opt-in checklist and crowd-sourcing FAQ | `documentation`, `good first issue` | — |

See also the red-team risks in [CRITICAL_CHAIN_PLAN.md](CRITICAL_CHAIN_PLAN.md), which overlap with several gaps above (e.g. PII leakage, unverifiable provenance, resource drain).

---

## 6. Summary

The three new requirements fit cleanly into the existing autonomous-loop design, but several prerequisites must close before they are shippable: a tested PII scrubber, a unified market-signal schema, a corroborated/reputation-backed data layer, and a one-shot daemon path. Treating crowd-sourced data with the same skepticism as LoRA provenance claims will keep the community features trustworthy and consistent with ATSassin's local-first, opt-in ethos.
