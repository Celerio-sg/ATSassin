> ## ⚠️ HISTORICAL — do not pick up work from this document
>
> Predates the 2026-07-29 architecture reset. Kept for context only. Parts of it plan work that is now **rejected** — notably Stage 3 DHT/P2P (#49) and crowd-sourced pooling (#105), both closed under [REJ-001](DECISIONS.md).
>
> Issue numbers here are stale. For current direction see [DECISIONS.md](DECISIONS.md), [ROADMAP.md](ROADMAP.md#the-critical-chain) and the tracking epic **#156**. Where this document disagrees with those, they win.

---

# Critical Chain Plan — Autonomous LoRA Sharing & Ecosystem

> **Scope:** This document applies Critical Chain Project Management (CCPM) to the experimental autonomous LoRA-sharing roadmap and the broader ATSassin autonomous loop. It includes an adversarial red-team review, a dependency map, a sequenced plan, and GitHub issue tagging guidance.
>
> **Companion docs:** [ROADMAP.md](ROADMAP.md) · [DESIGN_autonomous_loop.md](DESIGN_autonomous_loop.md) · [CATEGORY_LEADERSHIP_ROADMAP.md](CATEGORY_LEADERSHIP_ROADMAP.md)
>
> **Tracking issue:** [#45](https://github.com/Celerio-sg/ATSassin/issues/45)

---

## 1. Adversarial red-team: top risks and gaps

These are the failure modes that could kill the ecosystem feature (or the project) if not addressed early. They are ranked by a combination of impact and how easy they are to ignore until it is too late.

### R1 — Provider onboarding is still a hand-off, not a hand-hold
**Risk:** The "no technical barrier" goal is not met. A user without a configured provider hits a 401 or a terse error and has no idea how to get a free Groq/Kimi/etc. key. The existing `ComputeBroker` only routes among providers the user has already configured; there is no proactive recommendation or setup wizard.
**Evidence:** `docs/DESIGN_autonomous_loop.md` §5.1 explicitly calls for a future "recommend-don't-automate" advisory but it does not exist in `src/cli.rs` or `src/engine/compute_broker.rs` today.
**Mitigation:** Implement the provider onboarding wizard (#51) *before* Stage 1 of LoRA sharing, because every later stage assumes users can obtain free API capacity without becoming ML engineers.

### R2 — Privacy/PII leakage through shared LoRA weights
**Risk:** A model distilled on a user's real resumes, cover letters, and job data can memorize names, companies, email addresses, and addresses. If that adapter is shared, PII leaks to the network. This is a legal, trust, and project-killing risk.
**Evidence:** `docs/ROADMAP.md` guardrails table lists PII scrubbing. **RESOLVED (#143 and #81 closed):** `src/engine/egress.rs` validates the exact bytes uploaded by Lightning and fails closed before networking; candidate-derived values are scrubbed and no flagged copy is retained. Representative international detector fixtures and false-positive controls are complete without claiming universal NER.
**Mitigation:** ✅ EGRESS CONTAINMENT COMPLETED - the distillation pipeline scrubs all pairs and only `ValidatedTrainingPayload` can cross the Lightning upload boundary. This is not a claim of universal entity recognition.

### R3 — Unverifiable provenance claims become marketing spin
**Risk:** A publisher claims "distilled from Fable 5" when they actually used a cheap local model. Without a trusted execution environment during training, the claim is unverifiable. If rankings rely on claimed lineage, the system becomes a reputation contest for liars.
**Evidence:** `docs/DESIGN_autonomous_loop.md` §5.6b already states lineage is *claimed*, not *verified*, and substitutes reputation. This is correct but under-specified — there is no concrete reputation algorithm, no Sybil resistance, and no fallback if the coordinator is gamed.
**Mitigation:** Rank by empirical Proof-of-Quality only (#48). Treat lineage as display metadata. Document the impossibility of cryptographic proof clearly so contributors do not try to build one.

### R4 — The DHT/P2P stage is premature and could starve users' bandwidth
**Risk:** Stage 3 proposes a DHT before there is any proof that the HTTP registry is too centralized or that seeders exist. BitTorrent-style swarms die without seeders; a DHT with no participants is worse than a static `registry.json`.
**Evidence:** `docs/ROADMAP.md` Stage 3 says "Once adapter volume justifies it." This is the right gate, but there are no metrics for when volume justifies it.
**Mitigation:** Define exit criteria for Stage 3 (e.g., ≥50 adapters, ≥20 active seeders, ≥1000 weekly downloads) and keep it gated until those metrics are met.

### R5 — Volunteer compute cooperative could silently consume battery/data on laptops
**Risk:** Stage 4 donates idle local CPU/GPU cycles. On a laptop with metered internet, a background work unit could consume data or battery without the user realizing. This contradicts the local-first, low-spec promise.
**Evidence:** The guardrails mention idle detection and hardware tiers but do not define thresholds (CPU %, battery state, network meteredness).
**Mitigation:** Add explicit guardrails before Stage 4 ships: AC power required, no metered networks, default CPU cap (e.g. 25 %), and a clear tray/status indicator.

### R6 — The Compute Broker's `route_task` is incomplete and could bill users silently
**Risk:** The current `src/engine/compute_broker.rs` `route_task` logic has hardcoded fallbacks and does not yet implement the explicit `allow_paid` per-provider semantics described in the design. A paid provider could be selected without clear user consent.
**Evidence:** `route_task` returns the first provider matching crude filters; the paid-fallback policy from `DESIGN_autonomous_loop.md` §5.1 is not implemented.
**Mitigation:** ✅ COMPLETED - `ComputeBroker` now implements explicit `allow_paid` field in `ProviderProfile` (defaults to false), routing logic respects this flag, and tests verify paid-fallback rejection behavior.

### R7 — Documentation drift will confuse contributors
**Risk:** `docs/ROADMAP.md` and `docs/DESIGN_autonomous_loop.md` are now ahead of the code. New contributors will read them, assume features exist, and either duplicate work or file confused issues.
**Evidence:** `CATEGORY_LEADERSHIP_ROADMAP.md` gap #7 documents README drift as a real, already-occurred problem.
**Mitigation:** Every open issue in the Critical Chain plan must reference the exact doc section and acceptance criteria. Do not merge doc-only changes without linking them to the tracking issue.

---

## 2. Completeness / integrity mapping

| Requirement from chat | Where it is captured | Status / gap |
|---|---|---|
| No technical barrier | #51 provider onboarding wizard | Not implemented; needed before ecosystem growth |
| Autonomous operation | `DESIGN_autonomous_loop.md` §5.3, Stage 0-4 | ✅ COMPLETED - Daemon implements full autonomous loop (scan → evaluate → queue → tailor → follow-ups → IMAP sync) |
| LoRA adapters (not whole models) | `ROADMAP.md` Experimental section | Captured |
| Immutable hashed ledgers for provenance | `ROADMAP.md` Stage 2, `DESIGN_autonomous_loop.md` §5.6b | Captured as manifest DAG / content hashes |
| DAO lessons without tokens | `ROADMAP.md` Stage 2 | Captured as client-enforced reputation |
| BOINC-style volunteer local compute | `ROADMAP.md` Stage 4, #50 | Captured; guardrails need thresholds |
| Free cloud storage / LLM quota discovery | `DESIGN_autonomous_loop.md` §5.1, §5.6b | ✅ COMPLETED - Compute Broker implements quota observation from headers; provider onboarding not started |
| DHT scaling only when justified | `ROADMAP.md` Stage 3 | Captured; needs explicit gating metrics |
| No account automation / ToS violations | `DESIGN_autonomous_loop.md` §8.3 | Captured; must remain a hard rule |
| Rank by real earnings outcome | `ROADMAP.md` Stage 2, Stage 4 | Under-specified; needs concrete metric (offer/interview conversion) |
| PII scrubbing before any shared artifact | `ROADMAP.md` guardrails | ✅ Egress containment and deterministic detector coverage completed by #143 and #81 (closed); not universal NER |

---

## 3. Critical Chain plan

### 3.1 Work breakdown and estimates (aggressive, padding-stripped)

All estimates are in focused engineering days for a single contributor. CCPM says strip padding and protect the chain with a project buffer.

| ID | Issue | Task | Aggressive estimate | Notes |
|---|---|---|---|---|
| A | #51 | Provider onboarding/recommendation wizard | 2 days | Must be trivial for first-time users; highest leverage single issue |
| B | #46 | Stage 0: Local LoRA generation foundation | 4 days | Includes PII scrubber, training-pair export, and local quality gate |
| C | #47 | Stage 1: Read-only registry + manifest validation | 3 days | Depends on manifest format agreed in B |
| D | #48 | Stage 2: Proof-of-Quality reputation ranking | 5 days | Depends on C for manifest; needs coordinator design |
| E | #50 | Stage 4: Volunteer local compute cooperative | 4 days | Depends on B for adapter/evaluation units |
| F | #49 | Stage 3: DHT/P2P distribution | 6 days | Depends on C; gated by adoption metrics |
| G | — | Harden `ComputeBroker` paid-fallback + tests | 2 days | Precedes C/D/E; not yet filed as standalone issue |

### 3.2 Dependency graph

```
A (#51)  ─────────────────────────────────────────►  enables every later stage
 │
B (#46) Stage 0 ──► C (#47) Stage 1 ──► D (#48) Stage 2
 │                     │
 └─► E (#50) Stage 4  │
                       └─► F (#49) Stage 3 (gated by metrics)
```

G (broker hardening) ✅ COMPLETED - Compute Broker now implements `allow_paid` semantics and quota observation. No longer a blocking dependency.

### 3.3 Critical Chain

The longest dependent path is:

**A → B → C → D** = 2 + 4 + 3 + 5 = **14 aggressive days**

- A and B can start in parallel, but B does not release value without A (users cannot produce adapters if they cannot set up a provider). Therefore the chain starts with A → B.
- C depends on B (manifest format).
- D depends on C (registry before ranking).
- E and F are off the critical chain but consume the same maintainer review bandwidth.

**Project buffer:** Add 50 % buffer (7 days) feeding the end of D for integration, review, and contributor onboarding friction. **Total target: 21 days to a working reputation-ranked LoRA registry.**

**Note:** G (broker hardening) is now complete, reducing the critical chain by 2 days. Updated target: **19 days**.

### 3.4 Resource/contentention notes

- **Reviewer/maintainer time is the real bottleneck**, not code writing. The plan assumes one senior reviewer can keep pace. If not, the buffer shrinks or the chain extends.
- **Stage 3 (DHT)** is intentionally deprioritized and gated. It should not be started until C and D are live and the gating metrics are met.
- **Stage 4 (cooperative)** can be prototyped in parallel with C/D because it reuses the same evaluation units, but it should not ship before the PII and trust model are proven in B.
- **Broker hardening (G)** is now complete, removing a horizontal dependency and freeing reviewer bandwidth for the critical chain.

---

## 4. GitHub issue tagging and contributor guidance

### Labels to apply (use existing labels only)

| Issue | Recommended labels | Reason |
|---|---|---|
| #45 | `design:autonomous-loop` | Tracking parent |
| #46 | `design:autonomous-loop`, `good first issue` (partial) | Local-only; PII scrubber is senior, script export is newcomer-friendly — split if needed |
| #47 | `design:autonomous-loop`, `good first issue` | Self-contained HTTP registry + hash validation |
| #48 | `design:autonomous-loop` | Algorithm design; not a first issue |
| #49 | `design:autonomous-loop`, `help wanted` | Advanced networking; requires Rust-libp2p experience |
| #50 | `design:autonomous-loop`, `help wanted` | Systems work; needs hardware/OS knowledge |
| #51 | `design:autonomous-loop`, `good first issue` | UI/copy/validation only |

### Dependencies to add in issue bodies

- #45 depends on #46, #47, #48, #49, #50, #51
- #46 (Stage 0) blocks #47, #48, #49, #50
- #47 (Stage 1) blocks #48 and #49
- #48 (Stage 2) blocks nothing but benefits from #50
- #49 (Stage 3) is gated by adoption metrics; depends on #47
- #50 (Stage 4) depends on #46
- #51 (onboarding) is parallel enabler for all

### Milestone proposal

| Milestone | Issues | Definition of done |
|---|---|---|
| M1 — No-barrier setup | #51, G (broker hardening) | New user can install, run `atsassin setup`, and make a successful LLM call within 5 minutes |
| M2 — Local LoRA foundation | #46 | User can export training pairs, run external script, and locally evaluate the adapter |
| M3 — Community registry MVP | #47, #48 | User can opt in, download an adapter, and see it ranked by quality |
| M4 — Scale-out | #49, #50 | DHT or cooperative compute ships, gated by metrics |

---

## 5. Contributor split: small, reviewable chunks

To avoid the "too big to finish in an evening" trap that loses contributors:

- **#51 provider onboarding** — split into: (a) detect-no-provider + print recommendation, (b) `atsassin setup` interactive flow, (c) validation test call.
- **#46 Stage 0** — split into: (a) PII scrubber + tests, (b) high-confidence pair export, (c) external script generation, (d) local quality-gate evaluation.
- **#47 Stage 1** — split into: (a) config block, (b) registry fetch + parsing, (c) manifest validation, (d) Ollama variant creation.
- **#48 Stage 2** — split into: (a) local quality-score computation, (b) anonymized vote publishing, (c) registry ranking, (d) Sybil-mitigation design doc.
- **#50 Stage 4** — split into: (a) config + idle detection, (b) work-unit sandbox, (c) LoRA evaluation task, (d) quality-gate validation task.

---

## 6. Red-team action items

1. ✅ **Add PII scrubbing acceptance to #46.** COMPLETED - PII scrubber implemented and integrated.
2. ✅ **File issue G** (ComputeBroker hardening + paid-fallback tests) and block #47/#48/#49/#50 on it. COMPLETED - Broker now implements `allow_paid` semantics.
3. **Define Stage 3 gating metrics in #49** before work starts.
4. **Add Stage 4 idle/battery/network guardrails to #50** acceptance criteria.
5. **Keep #51 as the top priority** because it unblocks every user-facing value of the ecosystem.

---

## 7. Summary

The critical chain for the autonomous LoRA-sharing ecosystem is **#51 → #46 → #47 → #48**, with a **19-day target** including buffer (reduced from 21 days due to completion of broker hardening). The biggest risks are not technical — they are **onboarding friction, PII leakage, and unverifiable provenance claims**. The plan addresses them by making provider onboarding trivial, requiring PII scrubbing before any sharing (✅ COMPLETED), and replacing claimed lineage with empirical Proof-of-Quality. Stages 3 and 4 remain gated until adoption and safety metrics are met.
