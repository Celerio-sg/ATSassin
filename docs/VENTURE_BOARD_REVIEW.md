# ATSassin: Venture & Architectural Board Review

**Document Type:** Comprehensive business, product, and technical review  
**Date:** 2026-07-29  
**Prepared for:** Venture investment review & architectural board discussion  
**Status:** Living document — reflects current implementation state and future roadmap

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Mission & Positioning](#2-mission--positioning)
3. [Product Overview & Current State](#3-product-overview--current-state)
4. [Core Architecture](#4-core-architecture)
5. [Competitive Landscape](#5-competitive-landscape)
6. [Design Dimensions](#6-design-dimensions)
7. [Implementation & Governance](#7-implementation--governance)
8. [Milestone Roadmap](#8-milestone-roadmap)
9. [Open Questions & Risks](#9-open-questions--risks)
10. [Appendix: GitHub Issues Map](#10-appendix-github-issues-map)

---

## 1. Executive Summary

ATSassin is a **local-first, privacy-first, autonomous earning optimizer** — a single Rust binary that helps anyone, regardless of background, circumstance, location, finances, or compute power, discover their full earning potential and build a career that fulfils it.

### The Problem

Job-search tooling today falls into three categories, none of which serve the working professional:

| Category | Example | Fatal flaw |
|----------|---------|------------|
| **SaaS ATS/Copilot** | LazyApply, Simplify, Huntr | Sells your data, subscription-gated, cloud-only |
| **Python/ML repo** | Resume-Matcher, career-ops | Requires Docker, Python env, GPU — fragile install, won't run on a laptop |
| **Generic LLM chat** | ChatGPT, Claude | Lacks job-search-specific reasoning, no scanner, no pipeline, no outcome feedback |

None of them help a professional answer the question: *"Given what I actually know and can do, which adjacent role values it most, and how do I credibly walk into that room?"*

### The Solution

ATSassin is a single 10.96 MB Rust binary that:

1. **Infers** 5-10 suitable role archetypes from a single input (resume, LinkedIn export, or portfolio URL)
2. **Scans** 11+ job surfaces concurrently (LinkedIn, Seek, Greenhouse, Lever, Ashby, HN, Reddit, RemoteOK, Wellfound, WeWorkRemotely, Indeed, social aggregators)
3. **Evaluates** each role against the user's profile with a 6-dimension ATS-aware rubric using any configured LLM provider
4. **Tailors** resumes and cover letters to each posting
5. **Tracks** outcomes through the full pipeline (Applied → Interviewing → Offered → Rejected)
6. **Learns** from real outcomes to improve future recommendations

All of this runs **on-device, with zero telemetry, zero data egress by default, and no account required**.

### Why Now

Most people are leaving earning potential on the table. The reasons fall into three categories that ATSassin addresses together:

1. **AI automation** — roles are being consolidated and redefined across knowledge-worker segments, but the right response isn't panic — it's evidence-based awareness of which adjacent paths are viable.
2. **Complacency** — it is easy to stay in a role that is comfortable but undervalues your skills. The market shifts constantly; the value of what you know today may not be what it was last year. Without continuous re-evaluation, earning potential silently erodes.
3. **Unawareness** — opportunities exist in roles, industries, locations, and arrangements (contract, fractional, remote) that people never search for because they do not know to look. The single biggest deficit in career earnings is not lack of skill — it is lack of awareness of what exists.

The need is not for "better ATS keyword optimization." It is for **continuous earning intelligence**: proactive discovery of opportunities, evidence-based challenges to assumptions about what you can do, and grounded data about what your skills are worth — in any market, for any profile, on any hardware.

### Key Metrics

| Dimension | Current state |
|-----------|--------------|
| Binary size | 10.96 MB (measured on a clean `--release` build, 2026-07-29) |
| Hardware floor | 4 GB RAM, CPU-only (**target, never validated** — live issue #73; #5/#57 closed as duplicates) |
| Scraping surfaces | 11+ (LinkedIn, Seek, Greenhouse × 44 companies, HN, Reddit, RemoteOK, Wellfound, WeWorkRemotely, Indeed, Ashby, Lever) |
| Evaluation rigor | 6-dimension rubric (role match, north-star alignment, compensation, cultural signals, red flags, global fit) |
| TUI capability | Full terminal dashboard: infer, scan, evaluate, tailor, pipeline tracking |
| Verified personas | 5 distinct profiles tested end-to-end with real, non-fabricated LLM output |
| Community | MIT licensed, Discord, GitHub Sponsors, 140+ issues, good-first-issue program |
| Competitive benchmark | 7 competitors actually installed and run — ATSassin is the only one that ran end-to-end without a multi-minute setup tax |

---

## 2. Mission & Positioning

### The Honest Positioning

ATSassin began as "a free autonomous earning optimizer for everyone." That is still the mission — but the product today is a credible **application optimizer**: it parses profiles, discovers roles, evaluates fit, and tailors materials. That is a usable wedge in a crowded, low-trust category, but the wedge is not the mission.

The mission is broader:

> **ATSassin is a private, autonomous earning coach that helps anyone unlock their full earning potential.**
>
> It does this by proactively scouring the opportunity landscape for everything available to you, testing your preferences by recommending adjacent roles or locations that offer increased earning potential, and coaching you through the career moves that close the gap between where you are and where you could be.
>
> The enemies are threefold: AI automation that silently closes doors, complacency that keeps you in a role that undervalues you, and simple unawareness of the opportunities that already exist. ATSassin attacks all three — continuously, privately, and on any hardware.

This positioning changes everything about prioritization:

| Current framing | Mission-aligned framing |
|---|---|
| "Improve prerank matching" | "Tell someone which adjacent role they can credibly walk into" |
| "Salary inference dataset" | "Ground the 'will I earn as much?' anxiety in real data" |
| "Preference-challenge insights" | "Surface the move they haven't considered" |
| "Continuous market watch" | "Give them peace of mind that their value isn't silently dropping" |
| "Calibrate against outcomes" | "Prove this actually helps people land" |
| "Skill gap analysis" | "Close the gap between current and potential with targeted learning" |
| "Crowd-sourced signals" | "Make every instance smarter by pooling what each one discovers" |
| "LoRA ecosystem" | "Make the tool better for everyone by sharing quality-enhancing adapters" |

### Who it serves

**Everyone.** The tool is profile-agnostic by design — the same workflow that serves a senior enterprise sales leader in Singapore also serves a junior program manager in Berlin, a freelance designer in São Paulo, or a software engineer in Bangalore who has never considered contracting. The only thing that changes is what the tool discovers for you, not how the tool operates.

But every tool needs a first believer. The founding persona that shaped the design is:

**An experienced knowledge worker (sales, BD, program management — 10-20 year range) who has built real skills across multiple career phases, senses that their market value is shifting but cannot see where it is going, and needs to make one good move — not fifty applications.**

This persona proves the workflow end-to-end. The architecture generalizes from there.

> **⚠️ This persona is the single largest threat to the agnosticism claim above, and it has already caused a real defect.** "One good move, not fifty applications" is a *regime*, not a universal truth — it describes a selective senior candidate with time. It is wrong for an early-career candidate in a high-volume market, and wrong for anyone who needs income within weeks. A 2026-07-29 self-review found this framing and four others written into the layer specs as universals (#158, [ADR-008](DECISIONS.md)).
>
> The failure mode is structural rather than careless: **whoever is testing supplies the vivid, concrete detail that makes a design feel well-grounded, and their circumstances get written in as universals.** Read this persona as *the first test case*, never as the design target. The standing review question is: *would this behave sensibly for a user unlike the person who wrote it?*

### Core Values

1. **Zero-barrier autonomy.** It must run without API keys, vector-DB setup, or configuration. Anything heavier is opt-in.
2. **Privacy by physics.** Personal data never leaves the machine. Federated signals are only about jobs, never about candidates.
3. **Pragmatic over permissive.** A 60 % match on a high-leverage role beats a 100 % match on an irrelevant one.
4. **Honest failure.** Missing salary, missing remote signal, or missing visa data is surfaced explicitly.
5. **Lightweight first.** Rust-native, SQLite-based, quantized local models before cloud LLMs.

### The Founding Trial

A live trial against the founder's own profile (a senior sales/BD/PM leader with APAC experience) ran the full workflow end-to-end: profile init → role inference → job scanning across 11 surfaces → LLM evaluation → tailoring → apply kit generation. The tool uncovered 4 contract roles (Airtable, PolyAI, PHARMExcel, Later) and produced 3 strong matches with scores ≥0.70, generating tailored resumes and cover letters for each.

The trial revealed two things that shaped the design:

1. **The tool works end-to-end.** The pipeline produced real, grounded, non-fabricated output across multiple personas.
2. **The tool was finding too few senior roles because its sourcing was too narrow** — not because those roles do not exist. This directly informed the sourcing architecture redesign (issues #116, #130) and the broader insight that *lack of awareness* is the single biggest earnings limiter.

The trial also surfaced the key challenge for the career coaching dimension: when the user suggested "try Program Manager instead of VP Sales" — a role category they had not considered — the tool found stronger, more viable contract matches immediately. The insight was not that one role type is "better" than the other. It was that **most people are not searching for the right things**, and a tool that helps them discover what they should be searching for is more valuable than one that optimizes what they already search for.

---

## 3. Product Overview & Current State

### What's Real Today (Verified by Execution)

The following capabilities have been verified through end-to-end testing against 5 realistic personas with real LLM calls:

| Capability | Verification |
|-----------|-------------|
| Profile parsing (Markdown, plain-text resumes, LinkedIn export CSVs, DOCX) | All formats tested |
| Role inference from parsed profiles | Dynamic CV → 5-10 archetypes with market data |
| Job discovery across 11+ surfaces | Real-time concurrent scraping |
| ATS-aware evaluation (6-dimension scoring) | Tested against 8+ job postings, 5 personas |
| Resume & cover letter tailoring | Real files generated, 3-4 KB each, zero crashes |
| Pipeline tracking (SQLite) | Full CRUD, status transitions, CSV export |
| TUI dashboard | Live role inference, scan, evaluate, tailor from terminal |
| Outcome ingestion (IMAP) | Read-only, OS-keychain credential storage |
| Compute Broker routing | Multi-provider, observed-quota caching |
| Feedback calibration | Pipeline status → feedback → telemetry loop |
| Distillation export | PII-scrubbed training pairs with external training scripts |
| Application kit generation | Bookmarklet + JS snippets (human submits, never auto-submit) |

### Verified Competitive Benchmark

On 2026-07-25, 7 competitor tools were actually installed and run (not just read) against the same real candidate profile. Results:

| Competitor | Verdict |
|-----------|---------|
| **career-ops** | Strongest competitor — real working scanner. Its zero-token company scan was the one gap, which was closed with a native, concurrent reimplementation. |
| **ai-job-search** | Only ~2 of 13 advertised capabilities usable without external AI agent |
| **job-ops** | Docker build never finished (killed at 25+ min) |
| **jobsync** | Running but poor output — hallucinated fake education, scrambled data, no Groq support |
| **ApplyPilot** | Install blocked by dependency timeout |
| **Resume-Matcher** | Install blocked by dependency timeout |
| **job_finder** | Install blocked by torch wheel download timeout |

**Bottom line:** ATSassin's core loop (discover → evaluate → tailor → track) is the only one that ran reliably end-to-end without a multi-minute-to-multi-hour setup tax, with verified non-fabricated output across all 5 personas.

### Hardware Modes

| Mode | Model | Context | CPU OK | Min RAM |
|------|-------|---------|--------|---------|
| `light` | `qwen3.5:4b` | 4096 | yes | 4 GB |
| `balanced` | `qwen3.5:9b` | 8192 | yes | 8 GB |
| `full` | `qwen3.5:9b:q6` | 32768 | no | 16 GB |

### Critical Defects Found in Adversarial Review (2026-07-29)

A line-by-line adversarial review ([INFLECTION_ARCHITECTURE.md](INFLECTION_ARCHITECTURE.md)) found three P0 defects. All are scheduled as Step 0 of the critical chain, and **the architecture above should be read as aspirational until they are fixed**.

1. **Historical finding — fixed by #143 and #81 (closed): the PII gate did not cover the file that left the machine.** The audit found that `training_pairs.jsonl` was written after the old directory scan and uploaded unchecked, while the abort path retained a flagged copy. The fixed boundary now validates and uploads the same owned bytes and makes no flagged copy; deterministic regional fixtures and false-positive controls are complete without claiming universal NER.

2. **Job identity is random.** All three scan paths assign v4 UUIDs, and the schema has no uniqueness constraint on `url`. The same posting scanned twice becomes two rows; the evaluation cache can never hit; the daemon re-evaluates every job every hour indefinitely at full LLM cost. A live trial on 2026-07-29 found **8 of the top 20 recommendations were duplicates of other entries**. This made continuous market-watch (#121) unshippable as designed.

3. **Historical finding — fixed by #146 (closed): real-person PII was in the tree.** The identity-bearing Scenario 1 fixture and directory were replaced with a wholly synthetic senior APAC GTM persona of equivalent test shape. Current-tree tests, examples, and reports are anonymised, while point-in-time observations remain explicitly annotated. **Standing rule: no contributor's name, employers, compensation, or contact details belong in this repo or its issue tracker.** Test personas are described by shape only.

Additionally, several honest-failure violations were found and are scheduled for removal: fabricated `posted_at` values that systematically promoted the sources that fabricate dates over those that report them truthfully; synthesised 0.5 evaluations persisted as real on LLM parse failure; and error-swallowing that renders a network outage as "no jobs returned, try a different query."

### Known Verification Gaps

> **Issue references corrected 2026-07-29.** All four gaps below previously cited issues that have since been closed (#5, #6, #1, #3). Two of those closures were duplicate cleanup; two closed issues whose underlying defect is **still live**. The live issue numbers are given below — verify against the tracker before starting work, not against this table.

1. **Low-spec hardware claim unvalidated.** The documented 4 GB CPU-only target has never been tested on actual low-spec hardware. **This remains the largest unbacked claim in the project** — the entire "runs on any hardware" value proposition rests on it. Live issue: **#73** (#5 and #57 were closed as duplicates).
2. **Lightning AI 401 is a code defect, not a credential issue.** `.env.example:18` declares `LIGHTNING_USER_ID`, which is never read anywhere in `src/`; authentication is bearer-only. Live issue: **#154** (#6 closed).
3. **Company directory hand-maintained.** Static 44-company list that will go stale. Superseded mechanism: CNAME enumeration plus ATS API probe rather than careers-page HTML matching. Live issues: **#147**, **#116** (#1 closed).
4. **`--preset` does not differentiate hosted providers — and the cause is now known.** `config.rs:422` calls `sync_tier_models()` unconditionally, overwriting the light/balanced/full model names with `default_model` on every load. Confirmed live on 2026-07-29 (`Synced tier models to default: llama-3.3-70b-versatile`). All three `ModelRouter` tiers resolve to one model, so the documented hardware-mode table above does not describe hosted-provider behaviour. Live issue: **#171** (#3 closed prematurely).

---

## 4. Core Architecture

### Design Principles

Every architectural decision follows these rules, applied consistently:

1. **Lightweight by default.** The binary must build and run on a 4 GB CPU-only machine with `cargo build --release`, no Docker, no Python, no Node.
2. **Privacy as physics.** PII never leaves the machine. Federated signals are only about jobs, never about candidates.
3. **Cheap first, expensive second.** Jobs pass through (1) cheap deterministic filters → (2) local embeddings → (3) LLM evaluation → (4) tailoring. Each stage gates the next.
4. **Honest failure over fabricated plausibility.** Scrapers that return empty results, parsers that can't extract data, and models that can't score — all report the truth, never a plausible-looking placeholder.
5. **Opt-in for side effects.** Paid providers, email access, cloud archival, community sharing — all require explicit user opt-in, one at a time.

### System Architecture (Target State)

```
┌─────────────────────────────────────────────────────────────┐
│                     User / TUI / CLI                         │
└─────────────────────────────┬───────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│                     Orchestrator Node                          │
│   Schedules work, routes events, enforces guardrails.          │
│   Through Phase 2: CLI commands scheduled by user's own cron.  │
│   Phase 3+: optional daemon on Balanced/Full hardware only.    │
└─────────────────────────────┬─────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
┌───────▼──────┐   ┌──────────▼─────────┐   ┌───────▼──────┐
│   Scraper    │   │    Evaluator       │   │   Tailor     │
│   (modular   │   │    (6-dimension)   │   │   (resume +  │
│    sources)  │   │                    │   │   cover let) │
└───────┬──────┘   └────────┬─────────┘   └───────┬──────┘
        │                    │                     │
        └────────────────────┼─────────────────────┘
                             │
                ┌────────────▼────────────┐
                │    Compute Broker /      │
                │    Archive Manager       │
                │   Routes every task to   │
                │   the best available     │
                │   free/local/paid node.  │
                │   (paid requires         │
                │   `allow_paid=true`)      │
                └────────────┬────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
┌───────▼──────┐   ┌─────────▼─────────┐   ┌──────▼───────┐
│ Outcome      │   │   Distiller /     │   │   Archive /  │
│ Parser       │   │   Model Shrinker  │   │   Compressor │
│ (IMAP, read- │   │   (export PII-    │   │   (zstd cold)│
│  only, opt-  │   │   scrubbed pairs, │   │              │
│  in, OS      │   │   external train) │   │              │
│  keychain)   │   │                   │   │              │
└──────────────┘   └─────────┬─────────┘   └──────────────┘
                             │
                ┌────────────▼────────────┐
                │     Shared State          │
                │  SQLite (hot) + zstd      │
                │  compressed cold archive  │
                └─────────────────────────┘
```

### Module Structure

```
src/
├── main.rs                    # Entry point
├── cli.rs                     # CLI command definitions (45 subcommands)
├── config.rs                  # Configuration + .env management
├── lib.rs                     # Library root
├── models/
│   ├── mod.rs                 # Shared model types
│   ├── config.rs              # Configuration types
│   ├── job.rs                 # Job posting data model
│   ├── profile.rs             # User profile data model
│   └── role.rs                # Role archetype data model
├── engine/
│   ├── mod.rs                 # Engine module root
│   ├── profile_parser.rs      # Resume/LinkedIn/DOCX parsing
│   ├── role_inference.rs      # Dynamic role inference from profile
│   ├── matcher.rs             # Profile-to-job matching (keyword + section + semantic)
│   ├── scorer.rs              # 6-dimension ATS-aware scoring
│   ├── tailor.rs              # Resume + cover letter tailoring
│   ├── prerank.rs             # Fast lexical pre-filtering
│   ├── landscore.rs           # Composite "likely to land" ranking
│   ├── router.rs              # Model routing across providers
│   ├── compute_broker.rs      # Multi-provider compute routing
│   ├── cost.rs                # Cost tracking per provider
│   ├── hardware.rs            # Hardware tier detection
│   ├── prompts.rs             # LLM prompt templates
│   ├── llm.rs                 # LLM client (circuit breaker, retry)
│   ├── deep_research.rs       # Market research via web search
│   ├── distillation.rs        # Training pair export + PII scrubbing
│   ├── pii_scrubber.rs        # PII detection and redaction
│   ├── lightning.rs           # Lightning AI training client
│   ├── telemetry.rs           # LLM call logging
│   ├── feedback.rs            # Outcome-based feedback calibration
│   ├── preferences.rs         # User preference management
│   ├── quality.rs             # Quality metrics and escalation
│   ├── daemon.rs              # Background orchestrator (hardware-gated)
│   ├── ats_detector.rs        # Autonomous ATS platform detection
│   ├── benchmark.rs           # Benchmark framework
│   └── distillation.rs        # Distillation pipeline (export + training)
├── pipeline/
│   ├── mod.rs                 # Pipeline module root
│   ├── scanner.rs             # Job scanning orchestrator
│   ├── scraper.rs             # Multi-board scraping (LinkedIn, Seek, companies, social)
│   ├── social_scraper.rs      # Social platform scraping (HN, Reddit, etc.)
│   ├── board_discovery.rs     # Autonomous board discovery
│   ├── company_directory.rs   # Curated company-ATS directory
│   ├── tracker.rs             # SQLite pipeline tracker
│   ├── outcomes.rs            # Outcome ingestion (IMAP)
│   ├── actuation.rs           # Browser automation (assistive only)
│   ├── automation.rs          # Form-filling assistance
│   └── scanner.rs             # Multi-source job scanning
└── ui/
    ├── mod.rs                 # UI module root
    ├── tui.rs                 # Terminal UI (ratatui dashboard)
    └── output.rs              # Output formatting
```

### Data Model

The SQLite database stores:

- **Profiles**: parsed user data (name, email, phone, career history, skills)
- **Jobs**: discovered postings (title, company, description, URL, source board)
- **Preferences**: compensation floor, employment type, work mode, location
- **Pipeline entries**: status, notes, contact, follow-up dates, applied materials
- **Telemetry**: per-LLM-call logs (cost, latency, model, prompt, response)
- **Feedback**: outcome signals, edit distances, acceptance rates
- **Provider quotas**: observed rate-limit state per provider

---

## 5. Competitive Landscape

### Direct Competitors (Installed and Benchmarked)

| Competitor | Language | Strengths | Fatal Flaws |
|-----------|----------|-----------|-------------|
| **career-ops** | Python | Zero-token scanner (87 companies via ATS API), clean architecture | Docker + AWS dependency, no local LLM, no evaluation/tailoring |
| **Resume-Matcher** | Python | Best tailoring UI/UX, strong matching algorithm | Python env + GPU needed, cloud-dependent, no role inference |
| **jobsync** | Python | Running, functional UI | Small local model only (no Groq), hallucinated output, no evaluation |
| **ai-job-search** | Python/Node | Working LinkedIn scanner | 11 of 13 features are markdown stubs, not runnable code |
| **job-ops** | Python | Pipeline tracking | Docker build takes 25+ minutes |
| **ApplyPilot** | TypeScript | Cover letter templates | Cloud-only, install blocked by dependency timeout |
| **job_finder** | Python | Raw API access | Heavy ML dependency chain, no executive-level vocabulary |

### Competitive Advantages

1. **Single static binary.** No Docker, no Python, no Node, no GPU. Install time: seconds.
2. **Local-first by default.** Zero data egress, zero account required, zero telemetry.
3. **Hardware-adaptive.** Tiers from 4 GB CPU-only up to workstation + cloud.
4. **End-to-end workflow.** Discover → evaluate → tailor → track → learn — all in one binary.
5. **Outcome-calibrated.** Real interview/offer/rejection signals feed back into recommendations.
6. **Privacy as architecture.** Not a feature flag — personal data physically never leaves the device.

### Competitive Vulnerabilities

1. **Company directory is hand-maintained.** The zero-token company sweep is the competitive moat, but it rests on a static list subject to rot. The permanent fix (autonomous ATS detector, issue #116) is not yet implemented.
2. **Salary inference is LLM-only.** No real market dataset grounds the estimates. Career coaching depends on fixing this first.
3. **Low-spec claim unverified.** The entire "runs on old hardware" value prop has never been stress-tested on actual low-spec hardware.
4. **No continuous polling.** Today the tool waits for the user to run a command. The career coaching vision requires autonomous market watch.
5. **No community effects.** Every instance independently discovers the same boards and salaries. Crowd-sourcing would compound the advantage.

---

## 6. Design Dimensions

This section covers every design dimension discussed and their current status. Each dimension is its own workstream with clear prioritization.

---

### 6.1 Sourcing Architecture

**Goal:** Discover every relevant role across the entire public internet, not just a handful of boards.

> **Now Layer 1 — the evidence layer.** Delivered by the tiered extraction ladder: CNAME enumeration (#147) → ATS JSON APIs → `__NEXT_DATA__` SSR hydration (#148) → Schema.org JSON-LD (#149), behind the source trait (#130). Spec: [design/EVIDENCE_LAYER.md](design/EVIDENCE_LAYER.md).
>
> **Two additions from the 2026-07-29 review that are not in the original design direction below:** cross-board syndication dedup (#166) — the same requisition pushed to four boards produces four different URLs, which content-addressed identity (#142) does *not* collapse; and non-Western regional sources (#161), because the four ATS platforms named below serve US and Western-European tech and match almost nothing across most of the world's labour market.

**Current state:**
- 11+ scraping surfaces integrated
- Concurrent company sweep (44 companies in ~16-26 seconds, measured 2026-07-29)
- Social platform scraping (HN, Reddit, RemoteOK, Wellfound, etc.)
- Per-board rate limiting with honest empty results on failure

**The gap:** The scanner is broad but shallow. It hits a fixed set of known boards. Niche boards, company career pages on unknown ATS platforms, Slack/Discord job channels, recruiter newsletters, and industry-specific aggregators are all missed.

**Design direction — Modular source architecture (issue #130):**

```rust
trait JobSource {
    fn name(&self) -> &str;
    async fn fetch(&self, query: &str, prefs: &Preferences) -> Vec<JobSummary>;
}
```

Each source is one file in `src/sources/`, registered in a `SourceManager` that runs all sources concurrently with per-host rate limiting. New boards are a single file + one line in a registry — no changes to existing code.

**Design direction — Autonomous company ATS detector (issue #116):**

Replace the hand-maintained 44-company list with runtime detection: fetch a company's public careers page URL, pattern-match against Greenhouse/Lever/Ashby/Workday embed shapes, and persist the detected provider. This turns "add more companies" from a permanent chore into a one-time detector.

**Issue map:** #130 (source architecture), #116 (ATS detector)

---

### 6.2 Pragmatic Matching

**Goal:** Match candidates to roles based on transferable skills and adjacent experience, not just keyword overlap.

> **Substantially reframed, and three defects found underneath it.** The matching *stack itself* was found broken on 2026-07-29:
>
> - **#163** — `semantic_score` returns the L2 norm of a single embedding of the job and resume concatenated. It is not a similarity; there is no second vector. It carries **0.40, the largest weight** in the composite at `matcher.rs:60`. The weights below were therefore tuned while the largest term computed nothing, and must be re-derived rather than adjusted.
> - **#168** — the **keyword paradox**: keyword matching has AUC 0.558 and keyword density correlates *negatively* with post-hire output, while remaining the operational gatekeeper. Tailoring to keywords is correct; treating a keyword score as evidence of *fit* is not. These must be separated and labelled differently.
> - **#160** — the lexical path is English/Latin-script-only. CJK text collapses to one token under `prerank.rs:14-20`, and the byte-length filter silently drops "AI", "HR", "PM".
>
> The weighted taxonomy in #132 survives as a **feature extractor**; its asserted constants (1.0 / 0.7 / 0.4) are removed and fitted per-user by Layer 2 (#150) instead. Segment tags (#133) become the diversification dimension in Layer 3 rather than a prerank weight.

**Current state:**
- TF-IDF+bM25 prerank for fast lexical filtering (already good)
- 6-dimension LLM evaluation for high-precision scoring
- Preference filtering (comp floor, employment type, work mode with APAC-friendly matching)

**The gap:** A candidate with pharma sales experience who is a 90 % fit for a data-security sales role gets ranked below someone with 30 % fit but identical keywords. There is no model of industry segments, no transferable-skill mapping, and no "close enough" scoring.

**Design direction — Pragmatic requirement scoring (issue #132):**

Replace binary keyword matching with weighted categorical scoring:
- **Core requirements** (must-have): weighted high
- **Adjacent experience** (different industry, same function): weighted medium
- **Aspirational requirements** (nice-to-have listed as required): weighted low
- **Transferable skills**: recognized across role families (e.g., "GTM strategy" → applies to both "VP Sales" and "Head of Partnerships")

**Design direction — Job segment classifier (issue #133):**

At scrape time, tag each job posting with an industry segment (pharma, data security, fintech, etc.) and role family (sales, engineering, marketing, ops). This enables:
- Cross-segment matching ("you've done pharma; data security values similar compliance knowledge")
- Segment-specific compensation baselines
- Preference filtering at segment level ("don't show me roles in industries that conflict with my values")

**Design direction — Embedding-based proximity (issue #118):**

Use lightweight local embeddings (all-MiniLM-L6-v2 via ONNX) to compute cosine similarity between profile and job description. This captures semantic proximity beyond keywords — essential for the "adjacent role transition" use case.

**Design direction — Restriction parser (issue #117):**

Extract visa, language, and experience-level constraints from job text using regex and small-NLP patterns. Surface restrictions transparently rather than silently filtering.

**Issue map:** #132 (pragmatic scoring), #133 (segment classifier), #118 (embeddings), #117 (restriction parser)

---

### 6.3 Salary Inference

**Goal:** Ground earnings potential in real data, not LLM guesses.

**Current state:**
- LLM-based compensation estimation with seniority-aware sanity clamping
- Market rates CLI command (directional only, clearly disclaimed)
- Honest failure when no figure is present

**The gap:** Salary inference today is pure LLM estimation with a hardcoded sanity ceiling. There is no real market dataset, no cross-source corroboration, and no methodology for inferring unstated ranges.

> **Superseded 2026-07-29.** The maintained dataset below is no longer the primary plan. Layer 1's extraction ladder supplies compensation **directly from the employer, per posting, with perfect provenance and zero curation** — strictly better than a baseline file that needs perpetual updating and is stale the day it ships. Issue #119 is **closed out of Layer 2 entirely** — it is a salary dataset. The Layer 2 conversion **prior table is #176**, a separate artifact that does not exist yet. Earlier text here claimed #119 "survives as the prior table"; that was wrong and sent readers to the wrong issue. See [design/EVIDENCE_LAYER.md](design/EVIDENCE_LAYER.md) and [design/CALIBRATION_LAYER.md](design/CALIBRATION_LAYER.md).
>
> Related live finding: the compensation *floor* had the same class of problem from the opposite direction — it was sourced from a help-text placeholder, with no extraction and no onboarding prompt (#155).

**Superseded design direction — Market baseline salary dataset (issue #119):**

A periodically-updated, lightweight JSON file (not a database, not an API) mapping role × region × seniority to baseline ranges. Sourced from:
- Publicly available salary surveys
- Government labor data
- Aggregated crowd-sourced signals (opt-in, anonymized)
- Cross-referenced with actual job posting figures

**Design direction — Cross-corpus corroboration (issue #120):**

When multiple sources claim a salary for the same role/region, average across them and report the spread. A single data point is flagged as "uncorroborated." No figure is displayed without transparency about its confidence level.

**Issue map:** #119 (dataset), #120 (corroboration)

---

### 6.4 Career Coach

**Goal:** Become the user's continuous career optimizer — not just a tool used during job search.

> **Now Layers 2 and 3, and this is the flagship.** The vision below is intact but was under-specified — "insight cards" and "preference challenges" had no mechanism behind them, so any confidence label they carried would itself have been fabricated. They now have one:
>
> - **Layer 2** (#150, #151): per-user Bayesian conversion estimates with mandatory shrinkage and intervals, plus controllable-vs-structural attribution
> - **Layer 3** (#152, #153): the weekly allocated slate, and preference challenges as *solved counterfactuals* rather than heuristics
>
> **Blocking dependency:** the market-watch daemon (#121) cannot ship until #142 lands. Job IDs are currently random UUIDs, so the daemon re-evaluates every job on every tick at full LLM cost, indefinitely. Shipping continuous polling on that foundation ships unbounded silent spend. Specs: [design/CALIBRATION_LAYER.md](design/CALIBRATION_LAYER.md), [design/ALLOCATION_LAYER.md](design/ALLOCATION_LAYER.md).

**Current state:**
- Profile and preferences are saved and reusable
- `recommend` command ranks all pooled jobs by composite score
- Pipeline tracking enables outcome calibration
- Daemon (hardware-gated) provides automated scanning

**The gap:** The tool is passive. It waits for the user to run a command. The career coaching vision requires:
- Continuous market awareness without user initiation
- Proactive insights about unconsidered opportunities
- Re-validation of current position against market shifts
- Anti-atrophy engagement — keeping users on the app even when not job-hunting

**Design direction — Continuous market-watch daemon (issue #121):**

A scheduled scan that maintains an up-to-date view of open roles matching the user's profile and preferences. Does not wait for the user to run `scan`. Implemented as a lightweight one-shot CLI (for cron/Task Scheduler) on light hardware, and as a persistent daemon on balanced/full hardware.

**Design direction — Preference-challenge insights (issue #122):**

When the data shows that a small change — relocating, switching to contract, picking up a named adjacent skill, or targeting a different industry — could materially improve income or prospects, ATSassin surfaces the finding as a question, not a prescription. Example: "Senior Rust engineers in Berlin report median compensation ~40% higher than your current market; would you consider relocation or remote hiring in that region?"

This feature should:
- Ground insights in real market data (depends on salary inference dataset, issue #119)
- Be profile-agnostic — works the same way for any user
- Flow into the existing evaluate/tailor pipeline

**Design direction — Anti-atrophy retention:**

The coaching loop keeps users engaged even when not actively job-hunting by helping them re-evaluate and re-validate their current position against the market. This is not "spam the user with irrelevant job alerts"; it is periodic, structured, valuable check-ins about market position.

**Issue map:** #121 (market-watch), #122 (preference-challenge)

---

### 6.5 Distillation Pipeline

**Goal:** Turn usage data into smaller, faster, user-specific models that improve over time.

> **Independent of the critical chain — proceed in parallel.** #109–#114 are orthogonal to Layers 1–3 and are not blocked by Step 0. One dependency to note: #109 reads the same `edit_distance` / feedback table that Layer 2 (#150) consumes, so coordinate on schema changes.
>
> **Two corrections to the "current state" claimed below:** the Unsloth training script is a placeholder comment with an unused import (`distillation.rs:419-421`), and the GGUF script probes a filename llama.cpp has renamed while passing a `--outtype` value that only accepts `f32/f16/bf16/q8_0` — so the advertised Q4_K_M path cannot execute. The ONNX and OpenVINO scripts are real. Separately, **#115 is reframed to Layer 2**: outcome calibration now fits the per-user conversion model rather than ranking distilled artifacts.

**Current state (implemented):**
- `atsassin distill` exports filtered high-confidence training pairs
- PII scrubbing integrated before any export (PII gate validates final output)
- External training scripts generated for ONNX, GGUF, OpenVINO, and Unsloth
- Local quality-gate evaluation selects whether to use the checkpoint

**The gap:** The pipeline is functional but not end-to-end automated. Seven specific gaps have been identified for the production-grade pipeline:

**Design direction — Training dataset curation (issue #109):**

Deduplicate prompts, apply quality thresholds (filter pairs with high edit distance or negative outcomes), and balance task distribution to prevent mode collapse. Store curated pairs in a dedicated SQLite table with audit metadata.

**Design direction — Automated student model training (issue #110):**

End-to-end local fine-tuning workflow that:
- Accepts curated training pairs
- Launches the appropriate external training script (llama.cpp LoRA, Unsloth, or MLX)
- Monitors training progress
- Reports success/failure with logs

**Design direction — Distillation evaluation harness (issue #111):**

Automated quality gate that:
- Runs the student model against a held-out evaluation set
- Computes quality metrics (perplexity delta, output acceptance rate)
- Compares against teacher model baseline
- Blocks deployment if quality drops below threshold

**Design direction — Continuous improvement loop (issue #112):**

Auto-retrain on high-confidence feedback: when new outcome signals arrive (interview → offered → accepted), add the associated input/output pair to the training set and trigger a re-evaluation. Human-reviewed, not auto-deployed.

**Design direction — Cross-architecture deployment (issue #113):**

Export to CoreML (Apple Silicon), DirectML (Windows GPU), and WebGPU (browser) — not just ONNX/GGUF/OpenVINO. Each export path verifies the output before accepting.

**Design direction — Model registry & versioning (issue #114):**

Track each distilled artifact:
- Base model, teacher model, training parameters
- Quality metrics at deployment time
- Performance over time (did quality degrade?)
- Rollback capability if quality regression is detected

**Design direction — Calibrate against real outcomes (issue #115):**

Close the recommendation-to-offer loop: correlate model quality scores with actual pipeline outcomes (evaluate score → interview probability, tailor acceptance rate → offer probability). This is the metric that matters — not perplexity or loss.

**Issue map:** #109-#115

---

### 6.6 Community LoRA Sharing & Provenance (Experimental)

**Goal:** A community ecosystem where better source models naturally produce higher-ranked, more useful shared artifacts — without blockchain, without tokens, without centralized trust.

> **Chain truncated 2026-07-29 — two of five stages are gone.** Stage 3 (DHT/P2P, #49) is **rejected**: see [REJ-001](DECISIONS.md#rej-001--p2p--dht-distributed-crawling-libp2p-kademlia-skademlia-merkle-crdt). Stage 4 (#50) is **deferred**, not rejected on its own merits, but its transport dependency is gone. **Stage 2 (#48) is reframed to Layer 2** — its estimator inputs are the calibration feature set, so the estimator becomes purely local and the anonymised vote-publishing is dropped.
>
> **Still live and independent of the chain:** Stage 0 (#46) and Stage 1 (#47). Adapter distribution stays on the HTTP registry indefinitely rather than graduating to P2P. Priority is below the critical chain.
>
> **Guardrail status corrected.** The historical unchecked-upload defect and deterministic detector gap are fixed (#143 and #81 closed): exact owned bytes are checked immediately before egress, with representative regional fixtures and false-positive controls. This is still deterministic containment rather than universal NER.

**Design philosophy:**
- Share LoRA adapters (10-200 MB), not whole models. Adapters apply to a base model the user already has locally.
- Provenance is treated as *claimed*, not *verified*. Verification is replaced by empirical reputation.
- Content hashing guarantees artifact integrity. Reputation guarantees quality.
- Start with an HTTP registry, move to DHT/P2P only when adoption volume justifies it.

**Artifact manifest:**

```json
{
  "adapter_hash": "sha256:...",
  "parent_model": "qwen3.5:9b",
  "parent_model_hash": "sha256:...",
  "teacher_lineage": ["Fable 5"],
  "task_type": "tailoring",
  "author_pubkey": "...",
  "evaluated_quality": 0.87
}
```

**Stages:**

| Stage | What | Depends on | Gate |
|-------|------|------------|------|
| **0** | Local LoRA generation foundations | PII scrubber, training pair export | `atsassin distill` already functional |
| **1** | Read-only community registry + manifest validation | Stage 0 (manifest format) | HTTP registry on free tier |
| **2** | Proof-of-Quality reputation ranking | Stage 1 (registry) | Quality votes from telemetry |
| ~~**3**~~ | ~~DHT/P2P distribution~~ | — | ❌ **REJECTED** (#49 closed) — see [REJ-001](DECISIONS.md#rej-001--p2p--dht-distributed-crawling-libp2p-kademlia-skademlia-merkle-crdt). Adapter distribution stays on the HTTP registry indefinitely. |
| **4** | Volunteer local compute cooperative (BOINC-style) | Stage 0 (evaluation units) | ⏸ **Deferred** (#50) — not rejected on its own merits, but its transport dependency is, and it sits far behind the critical chain |

**Guardrails:**

| Concern | Mitigation |
|---------|------------|
| PII leakage | ✅ Exact-byte egress containment and deterministic regional coverage fixed (#143 and #81 closed). Candidate-derived identity values are scrubbed and validation fails before networking; universal NER is not claimed. |
| Malicious weights | Accept only GGUF/Safetensors. Reject pickled `.pt`/`.bin`. Verify SHA-256. |
| Unverifiable provenance | Treat lineage as claim. Rank by observed quality. |
| ToS violations | No automated signup for free credits. Bandwidth caps through Compute Broker. |

**Issue map:** #45 (tracking), #46-#50 (stages 0-4), #51 (onboarding wizard)

---

### 6.7 Crowd-Sourcing Layer — ❌ REJECTED 2026-07-29

> **Rejected. Do not implement.** Issue #105 is closed; see [REJ-001](DECISIONS.md#rej-001--p2p--dht-distributed-crawling-libp2p-kademlia-skademlia-merkle-crdt).
>
> Two grounds. First, it creates **new outbound data paths**. The historical upload defect is fixed (#143 closed), but broadening egress remains the wrong architectural direction and detector coverage is still deliberately bounded. Second, the architecture is **per-user by construction**: Layer 2 fits a conversion model from the user's own local outcome data, and the privacy architecture is precisely what makes that layer defensible against cloud competitors. A pooled cross-user corpus has no role in it.
>
> The underlying goal — better board coverage without every instance rediscovering the same sources — is met instead by the extraction ladder (#147, #148, #149), which **derives** sources rather than pooling them.

**Original goal (retained as history):** Pool board discoveries, salary signals, and review data across users while keeping each user's profile and application data local.

**Current state:** Every ATSassin instance independently discovers the same boards, salaries, and signals.

**Design direction:**
- Shared signals, not shared PII: only anonymized, non-attributable data about job postings, compensation ranges, company reviews, and board-detection patterns.
- Board-discovery feed: opt-in publishing of newly discovered board URLs and ATS detector patterns.
- Salary/review aggregation: anonymized salary data and "post to avoid" flags, signed by author key, ranked by observed quality.
- Anti-spam: claims treated as claims until corroborated across multiple independent instances.

**Issue map:** #105 — **CLOSED as rejected.** Superseded by the extraction ladder (#147, #173, #148, #149, #161), which derives sources rather than pooling them.

---

### 6.8 Career Awareness & Fulfillment

**Goal:** Help users understand the full landscape of opportunities available to them — not just the job titles they already know — and identify paths that increase both earnings and fulfilment.

> **Now delivered by Layer 3.** This dimension was previously "identified as a gap, not yet filed". It is the **allocation layer**: the diversification constraint in the min-cost flow (#152) is what mechanically surfaces adjacent opportunities, and the counterfactual re-solve (#153) is what quantifies them. See [design/ALLOCATION_LAYER.md](design/ALLOCATION_LAYER.md).
>
> **One correction carried over from the agnosticism review (#158):** adjacency is a property of the profile, not a universal good. Generalists have high adjacency and benefit from breadth; licensed specialists (clinicians, tax attorneys, pilots) have near-zero legitimate adjacency, and early-career candidates are often harmed by breadth that reads as unfocused. The diversification cap is **derived from the profile's own archetype clustering**, and *no* diversification must be reachable — which is a per-family cap **equal to the budget**, not a cap of 1. (A cap of 1 permits one application per family and therefore forces *maximum* spread; an earlier draft had this backwards.) "Awareness expansion" that pushes a specialist sideways is a defect, not a feature.

**Current state:** Specified, not yet built. Tracked as #152 and #153.

**Design direction:**
- **Opportunity landscape expansion**: proactively surface role archetypes, industries, locations, and work arrangements the user has never searched for but that match their actual skills. This is distinct from "match me to jobs I applied for" — it is about discovering what you *could* do, not just scoring what you *did* apply for.
- **Role vulnerability assessment**: analyze which responsibilities in a user's current role are most automatable (AI threat), most undervalued (complacency risk), or most undersupplied in their market (unawareness gap). Each factor points to a different kind of intervention.
- **Adjacent opportunity paths**: given the user's skill profile, identify roles and arrangements (contract, fractional, remote, relocation) that leverage their existing capabilities differently — not just "safer from AI" but "better paid" or "more aligned with interests."
- **Skill gap analysis**: surface specific, learnable skills that bridge from the current role to a higher-opportunity adjacent role. This is the coaching dimension: not just "you could do this" but "here is how to get there."
- **Interests-outside-work discovery**: a structured exploration that surfaces genuine interests and connects them to real open roles the user would never have searched for — using the existing role-inference and scan infrastructure, seeded from interests rather than only from past job titles.

**Issue map:** Not yet filed — identified as a gap in the design review. The core challenge is integrating these discovery modes into the existing `role_inference`, `preferences`, and `recommend` pipeline without adding complexity that works against the hardware-floor principle.

---

## 7. Implementation & Governance

### Design Principles for All Development

1. **User/role agnostic.** Every feature works for any profile, any industry, any seniority level. No hardcoded assumptions about the user's background.
2. **Profile-agnostic architecture.** The same workflow operates regardless of whether the profile is a sales leader, a software engineer, or a program manager.
3. **Segment-aware but segment-neutral.** The system tags jobs by segment but uses segment information only for calibration, never for exclusion.
4. **Lightweight first.** Every component must have a path that works on `light` hardware tier (4 GB, CPU-only) without degradation.
5. **Privacy by default.** No data leaves the machine without explicit, user-initiated opt-in.
6. **Honest over impressive.** Empty results are better than fabricated data. Unknown is better than guessed.

### Security & Privacy

| Concern | Mitigation | Status |
|---------|------------|--------|
| Resume/PII exposure | Local-first by default. No raw data leaves device without opt-in. | ✅ Enforced at architecture level |
| Free-tier data training | Flag providers that train on free-tier data; route sensitive tasks to local/Ollama. | ⚠️ Implemented for known providers; needs continuous monitoring |
| Credential storage | API keys in `.env`, IMAP credentials in OS keychain via `keyring` crate. | ✅ Implemented |
| Mailbox access | Read-only IMAP with app-password/OAuth. Never stores account password. | ✅ Implemented |
| SQLite encryption | No encryption-at-rest currently. Opt-in considered for future (issue #76). | ❌ Not implemented |
| Auto-submission | No application submitted without human action. Permanently architected boundary. | ✅ Design-level guarantee |

### Governance

| Element | Approach |
|---------|---------|
| **License** | MIT — permissive, corporate-friendly, no restrictions on use or fork |
| **Code ownership** | `CODEOWNERS` exists but currently resolves every rule to the single maintainer; three of five area entries are commented out and two reference files that do not exist. **There are no area leads yet.** `CONTRIBUTING.md` documents the path to becoming one. |
| **Contribution model** | Issues with good-first-issue labels and detailed acceptance criteria; PRs reviewed by the maintainer until area leads emerge |
| **Decision making** | Technical decisions by area lead + maintainer consensus. Strategic decisions by project founder with community input. |
| **Funding** | GitHub Sponsors (4 tiers from $5/mo to $500/mo). No corporate funding or VC dependency during early stage. |
| **Community** | Discord for real-time chat, GitHub Discussions for ideas, Issues for tracked work. |

### Ethical Boundaries

1. **No automated application submission.** This is a permanent architectural boundary, not an interim state. A human submits every application.
2. **No account automation.** The system never signs up for free credits or creates accounts on the user's behalf.
3. **No fabricated data.** Every scraper degrades to an honest empty result on failure. No placeholder output.
4. **No surveillance.** Outcome ingestion is opt-in, read-only, and scoped to job-related email patterns.
5. **No vendor lock-in.** The tool works with any OpenAI-compatible provider. Switching providers is a config change, not a migration.

---

## 8. Milestone Roadmap

### Critical Chain (shortest path to highest value)

> **Superseded as of 2026-07-29.** Phases 0–5 below describe work that has largely shipped and remain accurate as history. The **current** build order is the four-step critical chain in [ROADMAP.md](ROADMAP.md#the-critical-chain): Step 0 foundation repair → Step 1 evidence layer → Step 2 calibration layer → Step 3 allocation layer. Rationale in [INFLECTION_ARCHITECTURE.md](INFLECTION_ARCHITECTURE.md); settled and rejected decisions in [DECISIONS.md](DECISIONS.md); per-layer specs in [design/](design/); verification approach in [TEST_STRATEGY.md](TEST_STRATEGY.md).
>
> The strategic work-item table further below is likewise reordered by that chain — several items in it are subsumed or reframed, and three (#49, #50, #105) are now rejected. The tracker is the authority on per-issue status.

Each phase is independently shippable. Phases 0-2 need no daemon, no new hard dependencies, and no cloud accounts beyond what the user already configures.

#### Phase 0 — Outcome Loop (🎯 Highest leverage, best for external contributors)
- IMAP email ingestion for rejection/interview/offer detection ✅
- OS-keychain credential storage ✅
- Automatic pipeline status updates via outcome ingestion ✅
- Read-only mailbox access (no `\Seen` flag modification) ✅
- Plain CLI command (`atsassin outcomes sync`) ✅
- User's own cron/Task Scheduler runs it on schedule — no daemon needed

#### Phase 1 — Compute Broker + Dynamic Routing
- Multi-provider routing in `ModelRouter` ✅
- Quota as observed cache (provider self-reports via headers) ✅
- `atsassin compute status` CLI command ✅
- Explicit `allow_paid` semantics (defaults false) ✅
- ❌ **This claim was false.** Issue #3 was closed, but `--preset` still has no effect on model choice for hosted providers — `config.rs:422` collapses all three tiers to `default_model` on every load. Confirmed live 2026-07-29 and re-filed as **#171**.

#### Phase 2 — Local Compression
- zstd-compress telemetry/archive rows older than 30 days ✅
- Cold archive table for compressed data (separate from hot tables) ✅
- Cloud archival explicitly deferred until local DB growth justifies it ✅

#### Phase 3 — Optional Daemon (Hardware-Gated)
- Background orchestrator on Balanced/Full hardware only ✅
- Event-driven evaluation, ranking, and tailoring queue ✅
- Human-in-the-loop approval for actuation ✅
- On `light`-tier hardware, prints cron recommendation and exits ✅

#### Phase 4 — Assistive Actuation (Not Auto-Submit)
- Browser automation via Chrome DevTools Protocol + vision-capable model ✅ (stubbed)
- Form-filling automation (stops before submit control) ⏳
- Per-site ATS adapters (Greenhouse, Lever, Ashby, Workday) ⏳
- No auto-submit path. Permanent boundary. ✅ (architecture commitment)

#### Phase 5 — Distillation Flywheel
- Training-pair export (already what `atsassin distill` does) ✅
- PII scrubbing before any export ✅
- External training scripts (ONNX, GGUF, OpenVINO, Unsloth) ✅
- In-binary quality-gate evaluation and model selection ✅
- Training itself: external only (Python/LoRA environment, not in binary) ✅

### Community Ecosystem Track (Experimental)

| Stage | Target | Dependencies | Status |
|-------|--------|-------------|--------|
| 0 — Local LoRA generation | Foundations for adapter creation | PII scrubber ✅, training pair export ✅ | Partially complete (needs automated evaluation) |
| 1 — Read-only registry | HTTP-based adapter sharing | Stage 0 | Design complete, not yet implemented |
| 2 — Reputation ranking | Empirical quality scores | Stage 1 | Design complete, not yet implemented |
| 3 — DHT/P2P distribution | Peer-to-peer sharing | Stage 2 | Gated by adoption metrics |
| 4 — Volunteer compute cooperative | BOINC-style idle compute pool | Stage 0 | Gated by safety guardrails |

### Strategic Work Items — allocation table

Ordered by the critical chain. **Everything in Step 0 is unblocked and can start immediately.** Layer work is blocked on Step 0 landing, because random job identity and fabricated dates make every downstream measurement untrustworthy.

Milestones in GitHub mirror these four groups. Tracking epic: **#156**.

#### Step 0 — Foundation repair · *all unblocked, start now*

| Item | Issue | Size | Blocks |
|---|---|---|---|
| Forward-only transactional SQLite migrations | #181 closed — fixed | M | #142, #182, future schema work |
| Canonical content-addressed job IDs | **#142** | M | Everything |
| PII gate at single pre-upload choke point | **#143 closed — fixed** | M | Any egress work |
| International PII detectors | #81 closed — fixed | S | #143 closed; containment prerequisite fixed |
| Remove fabricated data (dates, 0.5 evals, roles research) | **#144** | S | Layers 2, 3 |
| Stop swallowing scraper errors | **#145** | S | Layer 1 |
| Replace real-person UAT fixture | #146 closed — fixed | S | — |
| Compensation floor: extract, prompt, stop defaulting | #155 | M | Layer 3 |
| Remove OpenSSL/native-tls (3 paths) | #67 | S | — |
| Merge prompt sanitisation into the single gate | #71 closed — fixed | S | #143 closed; shared egress boundary complete |
| Profile-agnosticism corrections | #158 | L | Layers 1-3 |

#### Step 1 — Evidence layer · *blocked on #142, #144, #145*

| Item | Issue | Size |
|---|---|---|
| Tier 4: Schema.org JSON-LD — **the universal path, ship first** | **#149** | M |
| Tier 1: CNAME enumeration | #147 | M |
| Tier 3: `__NEXT_DATA__` SSR hydration | #148 | M |
| Source trait expressing the ladder | #130 | L |
| Reframed: ATS detector #116, restriction parser #117, salary corroboration #120, comp grounding #58, drift canary #68 | | |

#### Step 2 — Calibration layer · *blocked on Step 1*

| Item | Issue | Size |
|---|---|---|
| Per-user Bayesian (Beta-Binomial) conversion model | **#150** | L |
| Controllable vs structural decomposition (safety requirement) | **#151** | M |
| Reframed: outcome calibration #115, local estimator #48, match taxonomy #132, conversion prior table **#176** (not #119, which is a salary dataset) | | |

#### Step 3 — Allocation layer · *blocked on Step 2*

| Item | Issue | Size |
|---|---|---|
| Min-cost max-flow slate generation | **#152** | L |
| Counterfactual re-solve | **#153** | M |
| Reframed: preference challenges #122, segment tags as diversification dimension #133, daemon as trigger #121, tracker #106 |  | |

#### Independent of the chain

Distillation cluster (#109–#114), LoRA Stages 0–1 (#45, #46, #47), onboarding wizard (#51), reliability and hygiene (#69–#83), low-spec validation (#73), Lightning auth (#154).

#### Rejected — do not implement

| Item | Issue | Reason |
|---|---|---|
| DHT/P2P adapter distribution | #49 closed | [REJ-001](DECISIONS.md#rej-001--p2p--dht-distributed-crawling-libp2p-kademlia-skademlia-merkle-crdt) — DHT rate limiter is a DDoS vector |
| Crowd-sourced role/salary/board pooling | #105 closed | [REJ-001](DECISIONS.md#rej-001--p2p--dht-distributed-crawling-libp2p-kademlia-skademlia-merkle-crdt) — new egress path; architecture is per-user |
| Volunteer compute cooperative | #50 deferred | Transport dependency rejected |

### Recent Fixes (Applied in Current Session)

**Status corrected 2026-07-29.** All issues referenced here are now **closed**; the consolidation tracked by #140 is complete. Three of the four "fixed" claims below were also found to be **overstated** by the adversarial review — the corrections are in the table.

| Item | Status | Issue(s) |
|------|--------|----------|
| Distillation conversion scripts | ⚠️ **Partly fixed.** ONNX and OpenVINO are real. **Unsloth is a placeholder comment** with an unused import (`distillation.rs:419-421`), and the GGUF script probes a filename llama.cpp renamed while passing an `--outtype` value that only accepts `f32/f16/bf16/q8_0` — the advertised Q4_K_M path cannot execute | #63, #84 (both closed) |
| Lightning AI client | ⚠️ **Transport real, target unverified.** Endpoints are self-documented as guesses and use the OpenAI path shape. The 401 is a code defect — the user-id env var is never read | #64, #85 (closed); live: **#154** |
| Daemon as full orchestrator | ✅ **Verified** — scan, prerank, evaluate, rank, tailor, follow-ups and IMAP sync all present. But it re-evaluates every job every tick because job identity is random (**#142**) | #65, #86 (both closed) |
| PII scrubber in the distillation export | ✅ **Containment and deterministic coverage fixed.** #143 closed the unchecked upload and flagged-copy defects; #81 closed with regional and false-positive fixtures | #66, #87, #143, #81 (closed) |
| Compute Broker — explicit `allow_paid` semantics, quota observation | Fixed | — |
| Board-health canary — scheduled detection of scraper drift | Fixed | #68 (CI workflow) |
| OpenSSL dependency — still present via **three** paths, not just `imap` | Open | **#67** (#88 closed as duplicate) |
| Low-spec hardware validation | Open | **#73** (#5, #57, #94 closed as duplicates) |
| Lightning AI 401 — confirmed a **code defect**, not a credential issue | Open | **#154** (#6 closed) |

---

## 9. Open Questions & Risks

### Open Questions

> **Status reviewed 2026-07-29.** Four of the eight below are now answered or moot. Their dispositions are recorded inline so contributors do not re-open settled ground; the genuinely open ones are marked as such.

1. **Which external training stack should `atsassin distill` target first?** llama.cpp LoRA, MLX (Apple Silicon), or Unsloth? Current answer: whichever the existing script-generation path already targets; extend from there.
   > **Still open**, but note the Unsloth path is currently a placeholder comment (`distillation.rs:419-421`) with an unused import, and the GGUF script probes a filename llama.cpp has renamed while passing a `--outtype` value that only accepts `f32/f16/bf16/q8_0`. Answer this by fixing what exists before adding a fourth target. Tracked under the distillation cluster (#109–#114).

2. **What is the acceptable quality-drop threshold for deploying a distilled model?** No answer yet — needs empirical data from the evaluation harness (issue #111).

3. **Which OAuth flow (Gmail, Outlook) is worth the implementation cost first for outcome ingestion?** Depends on what contributors' and early users' actual mail providers turn out to be. IMAP + app-password covers most providers day one.
   > **Still open, and deliberately unfiled.** This is the correct answer for now: IMAP plus app-password works today, and choosing an OAuth provider before there are users to observe would be guessing. Revisit once outcome ingestion has real usage — the decision needs data the project does not yet have. Note the dependency: Layer 2 (#150) needs outcome volume, so if IMAP friction is suppressing ingestion this becomes urgent rather than deferred.

4. **At what local-DB size does Phase 2's zstd compression stop being sufficient?** The threshold for cloud archival being worth building is unknown — needs to be measured in the field.

5. **How should the Proof-of-Quality reputation algorithm work in practice?** The design says "empirical acceptance rate," but Sybil resistance, coordinator failure, and vote weighting are all underspecified.
   > **Moot as posed.** #48 has been reframed to Layer 2: its estimator inputs (accepted outputs, edit distance, pipeline outcomes) are the calibration feature set, so the estimator moves to #150 as a purely *local* model. The anonymised vote-publishing half is dropped, being an outbound data path — the same ground on which #105 was rejected. With no cross-user voting there is no Sybil surface and no coordinator to fail.

6. **Should per-posting compensation negotiation advice be a feature?** The tool can infer whether a posted range is below market, but advice about how to negotiate is a high-liability area.
   > **Answered: no — see [ADR-009](DECISIONS.md).** Showing a user that a posted range sits below comparable postings is *earning intelligence* and is in scope, delivered by Layer 1's employer-supplied compensation (#149) plus corroboration (#120). Telling them what to say, what number to counter with, or when to walk is *advice*, is unmeasurable locally, and sits on the wrong side of the line drawn in Q7.

7. **Where is the line between "earning intelligence" and "career advice"?** Earning intelligence is defensible — market data, skill gap analysis, preference challenges backed by real numbers. Career advice carries liability — telling someone what they *should* do rather than showing them what the data says they *could* do. The product should stay firmly on the earning-intelligence side.
   > **Answered, and now enforced in three places.** The line is: *show the data and the conditional result; never prescribe the action.* Enforced by [REJ-007](DECISIONS.md) (no interview/ingratiation coaching — unobservable locally), [REJ-008](DECISIONS.md) (structural bias data is attribution only, never advice), [ADR-005](DECISIONS.md) (rates carry intervals, and structural factors never drive recommendations), and #153, where a preference challenge is a *solved counterfactual* — "relaxing location adds 0.9 expected interviews" — rather than "you should relocate".

8. **What is the right retention mechanism for non-job-hunting users?** Market-watch updates, preference challenges, and anti-atrophy insights are proposed but have not been validated with real users.
   > **Mechanism now specified; validation still open.** Layer 3 supplies it: the slate regenerates when the opportunity set changes materially (#121 as trigger rather than scanner), and the counterfactual re-solve (#153) gives a periodic, concrete read on market position without requiring an active search. Whether that actually retains anyone is unvalidated and needs the Tier 4 longitudinal trial (#159 for shape coverage) — **do not treat the mechanism existing as the question being answered.**

### Top Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **LLM provider rate limits / outages** | Medium | High | Multi-provider fallback implemented. Offline/local mode via Ollama. |
| **Scraper API changes breaking scan sources** | Medium | Medium | Every scraper degrades to honest empty result on failure (verified in UAT). Board-health canary detects drift. |
| **Compensation-estimate inaccuracy** | Medium | Medium | Seniority-aware sanity clamp in place. Real market-data source is the durable fix (issue #119). |
| **Community trust erosion (privacy)** | Low | High | Local-first by default. Zero telemetry. MIT-licensed and open source. |
| **Binary size growth over time** | Low | Low | LTO + strip in release profile. Currently 10.96 MB (measured 2026-07-29). |
| **Competitor closes gap on zero-token scanning** | Medium | Medium | Autonomous ATS detector (issue #116) is the durable moat — makes the directory self-maintaining. |
| **PII leakage through shared adapters** | Low | Critical | Exact-byte upload containment and deterministic fixtures fixed (#143 and #81 closed); retain fail-closed handling for unsupported identity. Accept only safe formats (GGUF/Safetensors). |
| **Low-spec hardware claim false** | Medium | High | Issue **#73** tracks validation (#5/#57 closed as duplicates). Must test on real 4 GB CPU-only machine before declaring it proven. |
| **Onboarding friction kills adoption** | Medium | High | Provider onboarding wizard (issue #51) is top priority. Must be trivial for first-time users. |

### Explicitly Out of Scope

The following were considered and rejected for the current design:

1. **Blockchain for federated learning coordination.** Rejected: adds enormous complexity without proportional benefit. On-chain storage of model updates is prohibitively expensive.
2. **Peer-to-peer federated learning.** Rejected: model updates can leak PII without strong differential privacy. Network overhead conflicts with the lightweight goal.
3. **Account automation for free credits.** Rejected: violates most providers' ToS. The system uses only keys the user already possesses.
4. **Full auto-submit on any job.** Rejected: a permanent architectural boundary. LinkedIn and most ATS platforms' ToS prohibit automated submission, and the human-review boundary is a trust guarantee worth keeping.
5. **Cloud-native deployment.** Rejected: the entire value prop is local-first. Cloud features are strictly opt-in supplements.

---

## 10. Appendix: GitHub Issues Map

### Current State

- **Open:** 63 · **Closed:** 81 (as of 2026-07-29, post-realignment)
- **Milestones:** four, mirroring the critical chain — Step 0 (10 open), Step 1 (9), Step 2 (6), Step 3 (6)
- **Tracking epic:** #156 — the single entry point for the next release
- **Realignment applied 2026-07-29:** 15 issues created (#142–#156, #158), 17 reframed with explicit comments, 3 closed as rejected (#49, #105, and #50 deferred), 5 closed as duplicates (#5, #57, #59, #60, #61), 1 closed as delivered (#107)

### Issue Map (allocation-ready)

Everything in Step 0 is **unblocked**. Layer work is blocked until Step 0 lands, because random job identity and fabricated dates make every downstream measurement untrustworthy.

```
#156 — TRACKING EPIC: next release (start here)

Step 0 — Foundation repair  [MILESTONE, all unblocked]
├── #181  Transactional SQLite migration framework      closed — fixed
├── #142  Canonical content-addressed job IDs           P0, M
├── #143  PII gate at single pre-upload choke point     closed — fixed
├── #144  Remove fabricated data                        S
├── #145  Stop swallowing scraper errors                S
├── #146  Replace real-person UAT fixture               closed — fixed
├── #155  Compensation floor: extract/prompt/no default M
├── #158  Profile-agnosticism corrections               L
├── #81   International PII detectors                   closed — fixed
├── #67   Remove OpenSSL/native-tls (3 paths)           S
└── #71   Merge prompt sanitisation into the gate       closed — fixed

Step 1 — Evidence layer  [MILESTONE, blocked on #142/#144/#145]
├── #149  Tier 4: Schema.org JSON-LD  ← ship first, universal path
├── #147  Tier 1: CNAME enumeration
├── #148  Tier 3: __NEXT_DATA__ SSR hydration
├── #130  Source trait expressing the ladder
└── reframed: #116 ATS detector · #117 restriction parser
             #120 salary corroboration · #58 comp grounding
             #68 drift canary (tier-fallthrough)

Step 2 — Calibration layer  [MILESTONE, blocked on Step 1]
├── #150  Per-user Bayesian (Beta-Binomial) conversion model
├── #151  Controllable vs structural decomposition (safety requirement)
└── reframed: #115 outcome calibration · #48 local estimator
             #132 match taxonomy · #176 conversion prior table (#119 superseded)

Step 3 — Allocation layer  [MILESTONE, blocked on Step 2]
├── #152  Min-cost max-flow slate generation
├── #153  Counterfactual re-solve
└── reframed: #122 preference challenges · #133 segment tags
             #121 daemon as trigger · #106 tracker

Independent of the chain
├── Distillation: #109 #110 #111 #112 #113 #114
├── LoRA Stages 0-1: #45 (tracker) #46 #47
├── Matching: #118 embedding cosine fix
├── Onboarding: #51 provider wizard · #52 broker hardening
├── Reliability/hygiene: #69 #70 #72 #74-#83
├── #73  Low-spec hardware validation (largest unbacked claim)
├── #154 Lightning auth defect (user-id env var never read)
└── #55 awesome lists · #53 first-timer welcome

Rejected — do not implement (see docs/DECISIONS.md)
├── #49  DHT/P2P adapter distribution     REJ-001, closed
├── #105 Crowd-sourced pooling            REJ-001, closed
└── #50  Volunteer compute cooperative    deferred (transport rejected)
```

### Label Taxonomy

| Label Category | Labels |
|---------------|--------|
| **Architecture (added 2026-07-29)** | `step-0-foundation`, `layer-1-evidence`, `layer-2-calibration`, `layer-3-allocation`, `rejected`, `tracking` |
| **Severity** | CRITICAL, HIGH, MEDIUM, LOW (in issue titles, not labels) |
| **Area** | area:matching, area:sourcing, area:models, area:privacy, area:security, area:llm, area:daemon, area:scraper, area:cli, area:config, area:ci, area:hardware, area:telemetry, area:performance, area:errors, area:reliability, area:schema, area:distillation |
| **Design** | design:autonomous-loop, design:career-coach |
| **Work type** | good first issue, help wanted, enhancement, bug, documentation, blocked |
| **Value** | analytics, user-value, community, privacy, audit |

The four architecture labels also exist as **GitHub milestones**, which are the unit to allocate against.

---

## Document Version History

| Version | Date | Author | Notes |
|---------|------|--------|-------|
| 1.0 | 2026-07-29 | ATSassin (Buffy) | Initial comprehensive review for venture/board discussion |
| 1.1 | 2026-07-29 | ATSassin (Buffy) | Corrected positioning: broadened from 'career transition engine for AI-displaced workers' to 'universal earning optimizer for everyone.' Three-factor problem framing (AI, complacency, unawareness) throughout. Reframed 6.8 to Career Awareness & Fulfillment. Updated persona, founding trial, and open questions to match. |
| 1.2 | 2026-07-29 | ATSassin (Buffy) | Updated README.md and ROADMAP.md to match new positioning. README now opens with the three-enemy framing. ROADMAP mission block added. |
| 2.0 | 2026-07-29 | Adversarial review | **Architecture reset: ranking → allocating.** Three P0 defects recorded in §3 (PII gate misses the uploaded file; random job identity; real-person PII in the tree). §6.3 salary dataset superseded by employer-supplied extraction. §6.7 crowd-sourcing and Stage 3 DHT rejected with reasoning. §6.8 now delivered by Layer 3. §8 strategic table replaced with an allocation table ordered by the critical chain. §10 issue map rebuilt. Numeric corrections: binary 10.96 MB (was 8.14 / ~9.5), 44 companies (was ~35/36), 45 CLI commands (was ~50). CODEOWNERS governance claim corrected — there are no area leads. New: [INFLECTION_ARCHITECTURE.md](INFLECTION_ARCHITECTURE.md), [DECISIONS.md](DECISIONS.md), [TEST_STRATEGY.md](TEST_STRATEGY.md), [design/](design/). |
| 2.1 | 2026-07-29 | Agnosticism self-review | Found founding-persona assumptions written into the layer specs as universals (#158, ADR-008): effort budget, diversification value, structural-factor list, Western-only ATS coverage, English-only detection, and a single-profile result presented as validation. Corrected in the specs; warning added to the founding-persona block in §2; Tier 3b profile-shape matrix added to the test strategy as an agnosticism gate. |

---

*This document is a living artifact. All assertions about implementation status are current as of the date above. Competitive benchmarks were conducted on 2026-07-25 and are snapshots in time. Design issues reference GitHub issues in the [Celerio-sg/ATSassin](https://github.com/Celerio-sg/ATSassin) repository — see each issue for the latest acceptance criteria and status.*
