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

ATSassin is a single 8.14 MB Rust binary that:

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
| Binary size | 8.14 MB (single static binary, no runtime) |
| Hardware floor | 4 GB RAM, CPU-only (target — see open issue #5) |
| Scraping surfaces | 11+ (LinkedIn, Seek, Greenhouse × ~36 companies, HN, Reddit, RemoteOK, Wellfound, WeWorkRemotely, Indeed, Ashby, Lever) |
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

### Core Values

1. **Zero-barrier autonomy.** It must run without API keys, vector-DB setup, or configuration. Anything heavier is opt-in.
2. **Privacy by physics.** Personal data never leaves the machine. Federated signals are only about jobs, never about candidates.
3. **Pragmatic over permissive.** A 60 % match on a high-leverage role beats a 100 % match on an irrelevant one.
4. **Honest failure.** Missing salary, missing remote signal, or missing visa data is surfaced explicitly.
5. **Lightweight first.** Rust-native, SQLite-based, quantized local models before cloud LLMs.

### The Founding Trial

A live trial against the founder's own profile (Simon Brender — senior sales/BD/PM leader with APAC experience) ran the full workflow end-to-end: profile init → role inference → job scanning across 11 surfaces → LLM evaluation → tailoring → apply kit generation. The tool uncovered 4 contract roles (Airtable, PolyAI, PHARMExcel, Later) and produced 3 strong matches with scores ≥0.70, generating tailored resumes and cover letters for each.

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

### Known Verification Gaps

1. **Low-spec hardware claim unvalidated.** The documented 4 GB CPU-only target has not been tested on actual low-spec hardware (issue #5).
2. **Lightning AI provider returns 401 Unauthorized.** Unconfirmed whether bug or credential issue (issue #6).
3. **Company directory hand-maintained.** Static ~35-company list that will go stale; the permanent fix is an autonomous ATS detector (issue #1, design issue #116).
4. **--preset does not differentiate hosted providers.** On cloud providers, `--preset` only changes timeout/retry values, not model choice (issue #3).

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
├── cli.rs                     # CLI command definitions (~50 commands)
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

**Current state:**
- 11+ scraping surfaces integrated
- Concurrent company sweep (36 companies in ~9 seconds)
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

Replace the hand-maintained ~35-company list with runtime detection: fetch a company's public careers page URL, pattern-match against Greenhouse/Lever/Ashby/Workday embed shapes, and persist the detected provider. This turns "add more companies" from a permanent chore into a one-time detector.

**Issue map:** #130 (source architecture), #116 (ATS detector)

---

### 6.2 Pragmatic Matching

**Goal:** Match candidates to roles based on transferable skills and adjacent experience, not just keyword overlap.

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

**Design direction — Market baseline salary dataset (issue #119):**

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
| **3** | DHT/P2P distribution | Stage 2 | ≥50 adapters, ≥20 seeders, ≥1000 weekly downloads |
| **4** | Volunteer local compute cooperative (BOINC-style) | Stage 0 (evaluation units) | AC power, idle, no metered network |

**Guardrails:**

| Concern | Mitigation |
|---------|------------|
| PII leakage | ✅ PII scrubber implemented. Scrubs training pairs before any sharing. PII gate blocks export if detectable PII remains. |
| Malicious weights | Accept only GGUF/Safetensors. Reject pickled `.pt`/`.bin`. Verify SHA-256. |
| Unverifiable provenance | Treat lineage as claim. Rank by observed quality. |
| ToS violations | No automated signup for free credits. Bandwidth caps through Compute Broker. |

**Issue map:** #45 (tracking), #46-#50 (stages 0-4), #51 (onboarding wizard)

---

### 6.7 Crowd-Sourcing Layer

**Goal:** Pool board discoveries, salary signals, and review data across users while keeping each user's profile and application data local.

**Current state:** Every ATSassin instance independently discovers the same boards, salaries, and signals.

**Design direction:**
- Shared signals, not shared PII: only anonymized, non-attributable data about job postings, compensation ranges, company reviews, and board-detection patterns.
- Board-discovery feed: opt-in publishing of newly discovered board URLs and ATS detector patterns.
- Salary/review aggregation: anonymized salary data and "post to avoid" flags, signed by author key, ranked by observed quality.
- Anti-spam: claims treated as claims until corroborated across multiple independent instances.

**Issue map:** #105

---

### 6.8 Career Awareness & Fulfillment

**Goal:** Help users understand the full landscape of opportunities available to them — not just the job titles they already know — and identify paths that increase both earnings and fulfilment.

**Current state:** Not implemented as a dedicated dimension. The existing role inference, preference-challenge, and market-rate features touch on this but are not yet coordinated into a coherent "awareness expansion" capability.

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
| **Code ownership** | `CODEOWNERS` file with area leads for major tracks (PII, career coach, crowdsourcing, AI exposure, autonomous loop) |
| **Contribution model** | Issues with good-first-issue labels, detailed acceptance criteria, PRs reviewed by area leads |
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
- Closes issue #3 (`--preset` having no effect on hosted providers) as side effect ✅

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

### Strategic Work Items (Queued)

Each is captured as a GitHub issue with acceptance criteria:

| Priority | Item | Issue | Dependencies |
|----------|------|-------|-------------|
| HIGH | Modular source architecture — trait-based pluggable sources | #130 | None |
| HIGH | Autonomous company ATS detector — replace static board list | #116 | None |
| HIGH | Pragmatic requirement scoring — adjacent/transferable/weighted | #132 | None |
| HIGH | Market baseline salary dataset — lightweight JSON | #119 | None |
| HIGH | Training dataset curation pipeline | #109 | None |
| HIGH | Automated student model training | #110 | None |
| HIGH | Distillation evaluation harness | #111 | None |
| HIGH | Crowd-source role/salary/board knowledge | #105 | None |
| HIGH | Continual landscape polling/career coaching | #106 | None |
| MEDIUM | Job segment classifier — tag roles by industry | #133 | #130 |
| MEDIUM | Embedding-based proximity matching | #118 | None |
| MEDIUM | Visa/language/experience restriction parser | #117 | None |
| MEDIUM | Cross-corpus salary corroboration | #120 | #119 |
| MEDIUM | Continuous market-watch daemon | #121 | #119 |
| MEDIUM | Preference-challenge insights engine | #122 | #119, #121 |
| MEDIUM | Continuous model improvement loop | #112 | #110, #111 |
| MEDIUM | Cross-architecture deployment targets | #113 | #110 |
| MEDIUM | Model registry & versioning | #114 | #110 |
| MEDIUM | Calibrate distillation against outcomes | #115 | #110, #111 |
| LOW | Consolidate duplicate issues | #140 | None |
| LOW | Provide onboarding wizards | #51 | None |
| LOW | SQLite encryption-at-rest | #76 | None |
| LOW | Configurable circuit breaker | #74 | None |
| LOW | Structured error codes | #80 | None |

### Recent Fixes (Applied in Current Session)

**Note:** These code fixes have been applied to the binary and are working, but the corresponding GitHub issues have not yet been closed (issues #63, #64, #65, #66 and duplicates #84, #85, #86 remain open in the tracker awaiting cleanup). See issue #140 for the consolidation tracking.

| Item | Status | Issue(s) |
|------|--------|----------|
| Distillation conversion scripts — real ONNX, GGUF, OpenVINO scripts, not stubs | Code fixed, issue pending closure | #63, #84 |
| Lightning AI client — real training client, not stub | Code fixed, issue pending closure | #64, #85 |
| Daemon as full orchestrator (scan/evaluate/rank/tailor/follow-ups/IMAP) | Code fixed, issue pending closure | #65, #86 |
| PII scrubber — integrated into distillation export pipeline | Code fixed, issue pending closure | #66, #87 |
| Compute Broker — explicit `allow_paid` semantics, quota observation | Fixed | — |
| Board-health canary — scheduled detection of scraper drift | Fixed | #68 (CI workflow) |
| OpenSSL dependency — remove via `imap` crate `rustls` feature | Open | #67, #88 |
| Low-spec hardware validation | Open | #5, #73, #94 |
| Lightning AI 401 | Open | #6 |

---

## 9. Open Questions & Risks

### Open Questions

1. **Which external training stack should `atsassin distill` target first?** llama.cpp LoRA, MLX (Apple Silicon), or Unsloth? Current answer: whichever the existing script-generation path already targets; extend from there.

2. **What is the acceptable quality-drop threshold for deploying a distilled model?** No answer yet — needs empirical data from the evaluation harness (issue #111).

3. **Which OAuth flow (Gmail, Outlook) is worth the implementation cost first for outcome ingestion?** Depends on what contributors' and early users' actual mail providers turn out to be. IMAP + app-password covers most providers day one.

4. **At what local-DB size does Phase 2's zstd compression stop being sufficient?** The threshold for cloud archival being worth building is unknown — needs to be measured in the field.

5. **How should the Proof-of-Quality reputation algorithm work in practice?** The design says "empirical acceptance rate," but Sybil resistance, coordinator failure, and vote weighting are all underspecified.

6. **Should per-posting compensation negotiation advice be a feature?** The tool can infer whether a posted range is below market, but advice about how to negotiate is a high-liability area.

7. **Where is the line between "earning intelligence" and "career advice"?** Earning intelligence is defensible — market data, skill gap analysis, preference challenges backed by real numbers. Career advice carries liability — telling someone what they *should* do rather than showing them what the data says they *could* do. The product should stay firmly on the earning-intelligence side.

8. **What is the right retention mechanism for non-job-hunting users?** Market-watch updates, preference challenges, and anti-atrophy insights are proposed but have not been validated with real users.

### Top Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **LLM provider rate limits / outages** | Medium | High | Multi-provider fallback implemented. Offline/local mode via Ollama. |
| **Scraper API changes breaking scan sources** | Medium | Medium | Every scraper degrades to honest empty result on failure (verified in UAT). Board-health canary detects drift. |
| **Compensation-estimate inaccuracy** | Medium | Medium | Seniority-aware sanity clamp in place. Real market-data source is the durable fix (issue #119). |
| **Community trust erosion (privacy)** | Low | High | Local-first by default. Zero telemetry. MIT-licensed and open source. |
| **Binary size growth over time** | Low | Low | LTO + strip in release profile. Currently ~9.5 MB. |
| **Competitor closes gap on zero-token scanning** | Medium | Medium | Autonomous ATS detector (issue #116) is the durable moat — makes the directory self-maintaining. |
| **PII leakage through shared adapters** | Low | Critical | PII scrubber implemented. Gate validates final output. Accept only safe formats (GGUF/Safetensors). |
| **Low-spec hardware claim false** | Medium | High | Issue #5 tracks validation. Must test on real 4 GB CPU-only machine before declaring it proven. |
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

- **Total issues:** 128 (as of 2026-07-29, post-cleanup)
- **Open:** 55
- **Closed:** 73
- **Cleanup actions applied:** 37 duplicate/stale issues closed (#43, #44, #62-#66, #84-#104, #108). Strategic items #105-#107 re-opened. Old board-health canaries consolidated to #141.
- **Labels:** 20+ covering severity, area, and work type

### Issue Hierarchy

```
Tracking Issues (epics / containers)
├── #18 — Autonomous closed-loop job & income optimization
├── #45 — Autonomous community LoRA sharing, provenance & volunteer compute cooperative
├── #107 — Comprehensive codebase and roadmap completeness review

Phase Implementations (shipped phases 0-2)
├── #19 — Phase 0: IMAP connection + OS-keychain credential storage ✅
├── #20 — Phase 0: Rejection/interview/offer email classification ✅
├── #21 — Phase 0: Wire outcomes into pipeline status + feedback ✅
├── #22 — Phase 0: `atsassin outcomes sync` CLI command ✅
├── #23 — Phase 1: Compute Broker core ✅
├── #24 — Phase 1: Provider self-reported quota parsing ✅
├── #25 — Phase 1: `atsassin compute status` CLI command ✅
├── #26 — Phase 1: Wire Compute Broker into --preset ✅
├── #27 — Phase 2: Local zstd compression for old telemetry rows ✅
├── #28 — Phase 3: `atsassin daemon` background orchestrator ✅
├── #29 — Phase 4: Assisted browser form-filling (no auto-submit) ✅
├── #30 — Phase 5: Distillation: export training pairs + external script ✅

Critical Red-Team Fixes
├── #63 — CRITICAL: Distillation conversion scripts are stubs 🔧 Code fixed, see note below
├── #64 — CRITICAL: Lightning AI integration is stubbed 🔧 Code fixed, see note below
├── #65 — HIGH: Daemon is scan-only and not full orchestrator 🔧 Code fixed, see note below
├── #66 — HIGH: PII scrubbing missing from LoRA sharing pipeline 🔧 Code fixed, see note below
├── #67 — HIGH: OpenSSL dependency via `imap` crate
├── #68 — HIGH: Board-health canary for scraper drift detection
├── #69 — MEDIUM: Startup validation of required secrets
├── #70 — MEDIUM: Health check command
├── #71 — MEDIUM: Prompt input sanitization
├── #72 — MEDIUM: README sync check in CI
├── #73 — MEDIUM: Validate low-spec/CPU-only hardware claim
├── #74-#83 — LOW: Various hardening (circuit breaker, telemetry, encryption, etc.)

Performance & Community Fixes
├── #89 — Board-health canary
├── #90 — Startup validation of required secrets
├── #91 — `atsassin health` command
├── #92 — Sanitize user-provided text before LLM prompts
├── #93 — README sync check to CI
├── #94 — Validate low-spec hardware claim
├── #95 — Configurable circuit breaker parameters
├── #96 — Circuit breaker metrics in telemetry
├── #97 — SQLite encryption-at-rest
├── #98 — Database connection pool evaluation
├── #99 — Disk space monitoring
├── #100 — Remove `.expect()` in HTTP clients
├── #101 — Structured error codes
├── #102 — Expand PII scrubber for international formats
├── #103 — Document all environment variables
├── #104 — Configuration backup and rollback

Strategic Work Items
├── #105 — Crowd-source role, salary and job-board knowledge
├── #106 — Continual job-landscape polling and career coaching insights

Deep Design Issues
├── Distillation Pipeline
│   ├── #109 — Training dataset curation pipeline
│   ├── #110 — Automated student model training workflow
│   ├── #111 — Distillation evaluation harness & benchmark
│   ├── #112 — Continuous model improvement loop
│   ├── #113 — Cross-architecture deployment targets
│   ├── #114 — Model registry & versioning
│   ├── #115 — Calibrate distillation against real outcomes
│   └── (closed duplicates: #123-#129)
├── Sourcing Architecture
│   ├── #130 — Modular source architecture (trait-based sources)
│   ├── #116 — Autonomous company ATS detector
│   └── (closed duplicate: #131)
├── Pragmatic Matching
│   ├── #132 — Pragmatic requirement scoring (adjacent/transferable)
│   ├── #133 — Job segment classifier
│   ├── #117 — Visa/language/experience restriction parser
│   ├── #118 — Embedding-based proximity matching
│   └── (closed duplicate: #134)
├── Salary Inference
│   ├── #119 — Market baseline salary dataset
│   ├── #120 — Cross-corpus salary corroboration
│   └── (closed duplicates: #135-#136)
├── Career Coach
│   ├── #121 — Continuous market-watch daemon
│   ├── #122 — Preference-challenge insights engine
│   └── (closed duplicates: #137-#139)
└── Housekeeping
    ├── #140 — Consolidate duplicate issues
    └── #141 — Board-health canary

Experimental / Long-term
├── #46 — Stage 0: Local LoRA generation foundation
├── #47 — Stage 1: Read-only community registry + manifest validation
├── #48 — Stage 2: Proof-of-Quality reputation ranking
├── #49 — Stage 3: DHT/P2P LoRA adapter distribution
├── #50 — Stage 4: Volunteer local compute cooperative (BOINC-style)
├── #51 — Provider onboarding/recommendation wizard
├── #52 — Harden ComputeBroker paid-fallback and route_task policy ✅

Early Issues
├── #1-#17 — Mostly closed (scraper fixes, APAC support, CI, company directory). 
│                 #5 (low-spec validation) and #6 (Lightning AI 401) remain OPEN.
├── #55 — Submit ATSassin to awesome lists [OPEN]
├── #56 — Launch ATSassin publicly [CLOSED]
├── #57 — Validate low-spec hardware claim [OPEN]
├── #58 — Replace LLM compensation estimation with real market data [OPEN]
├── #59 — Implement provider onboarding wizard [OPEN]
├── #60 — Implement local LoRA generation foundation [OPEN]
├── #61 — Build read-only registry with manifest validation [OPEN]
```

### Label Taxonomy

| Label Category | Labels |
|---------------|--------|
| **Severity** | CRITICAL, HIGH, MEDIUM, LOW |
| **Area** | area:matching, area:sourcing, area:ai, area:models, area:coach, area:crowdsource, area:exposure, area:pii, area:schema |
| **Design** | design:autonomous-loop, design:career-coach |
| **Work type** | good first issue, help wanted, enhancement, bug, documentation, design, blocked |
| **Tracking** | canary, tracking |

---

## Document Version History

| Version | Date | Author | Notes |
|---------|------|--------|-------|
| 1.0 | 2026-07-29 | ATSassin (Buffy) | Initial comprehensive review for venture/board discussion |
| 1.1 | 2026-07-29 | ATSassin (Buffy) | Corrected positioning: broadened from 'career transition engine for AI-displaced workers' to 'universal earning optimizer for everyone.' Three-factor problem framing (AI, complacency, unawareness) throughout. Reframed 6.8 to Career Awareness & Fulfillment. Updated persona, founding trial, and open questions to match. |
| 1.2 | 2026-07-29 | ATSassin (Buffy) | Updated README.md and ROADMAP.md to match new positioning. README now opens with the three-enemy framing. ROADMAP mission block added. |

---

*This document is a living artifact. All assertions about implementation status are current as of the date above. Competitive benchmarks were conducted on 2026-07-25 and are snapshots in time. Design issues reference GitHub issues in the [Celerio-sg/ATSassin](https://github.com/Celerio-sg/ATSassin) repository — see each issue for the latest acceptance criteria and status.*
