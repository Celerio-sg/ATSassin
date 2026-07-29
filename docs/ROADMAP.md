# Roadmap, Known Issues & Ethical Considerations

> **The mission:** ATSassin helps anyone, regardless of background, location, finances, or compute power, unlock their full earning potential. The three enemies are **AI automation** (silently closing doors), **complacency** (staying in a role that undervalues you), and **unawareness** (not knowing what opportunities even exist). Everything below — the roadmap, the open gaps, the experimental features — serves that mission.
>
> **Ground truth for contributors:** the two most reliable documents in this repo are [UAT_REPORT_2026-07-24.md](UAT_REPORT_2026-07-24.md) (issues found and fixed via real end-to-end testing across 5 personas) and [COMPETITIVE_BENCHMARK_2026-07-25.md](COMPETITIVE_BENCHMARK_2026-07-25.md) (ATSassin actually run head-to-head against 7 competitor tools). Where this roadmap and older planning docs (`COMPARISON.md`, `EXECUTIVE_SUMMARY.md`, `RANKING.md`, `COMPLIANCE_MATRIX.md`, `../AUDIT.md`) disagree, trust the UAT/benchmark reports — they reflect what was actually run and observed, not what was planned or claimed. `AUDIT.md` in particular is a point-in-time red-team note from an early draft (2026-07-23) predating the UAT/benchmark work below — most of the gaps it lists have since been closed; read it as history, not a current status report.
>
> **For the structural plan behind staying ahead** (not just the list of open items below), see [CATEGORY_LEADERSHIP_ROADMAP.md](CATEGORY_LEADERSHIP_ROADMAP.md) — a red-team pass on where today's competitive position is fragile (undocumented API drift, a one-time benchmark snapshot, docs already drifting from shipped features) and what closes each gap structurally.

## What's real today (verified by actual execution, not just present in the code)

- Profile parsing (Markdown/plain-text resumes, LinkedIn export CSVs, DOCX)
- Local-relevance role inference and pipeline tracking (SQLite), evaluate and tailor via any OpenAI-compatible provider (Groq, Ollama, Kimi, GLM, Lightning, etc.)
- Job discovery: LinkedIn guest API, Seek, single-company Greenhouse/Lever/Ashby lookups, a curated **concurrent zero-token sweep across 44 companies' public Greenhouse APIs** (`--boards companies`), plus a social/aggregator sweep (HN, Reddit, RemoteOK, WeWorkRemotely, etc.)
- A full TUI dashboard: live role inference, scan, evaluate, and tailor, all triggerable from within the TUI, with a real pipeline summary and activity log
- Verified end-to-end for 5 realistic, diverse personas (see UAT report) with zero crashes and non-fabricated output

## Known issues / open gaps

These are real, currently-open items, not resolved yet:

1. **The curated company-to-board directory (`src/pipeline/company_directory.rs`) is a maintenance liability, not just incomplete.** It's currently a flat, hand-maintained list of 44 company/slug pairs. Even fully filled out, which company sits on which ATS (Greenhouse/Lever/Ashby/Workable) changes constantly as companies migrate providers, so a static list goes stale within months regardless of how many entries it has. **Better direction (raised in community review):** derive the directory instead of maintaining it — fetch a company's public careers page and pattern-match the embedded board URL (Greenhouse/Lever/Ashby/Workable each have a recognizable embed/redirect shape) to detect which provider and slug they use. This turns "add more companies" from a permanent, weekly-recurring chore into a one-time detector. **Good first issue** in the meantime: adding more company/slug pairs to the existing list is still useful and needs no architecture change — just lower-leverage than the detector.
2. **`--preset` (lightweight/balanced/full) has no effect on model choice when using a hosted cloud provider** — it only changes timeout/retry/scrape-limit values. The hardware-adaptive tiering system only differentiates behavior when running against local Ollama. This should either be documented more prominently or extended so cloud presets pick different model *sizes* per tier (e.g. a cheaper/faster Groq model for `lightweight`).
3. **Compensation estimates are model-generated and can still be wrong even after the seniority-aware sanity clamp** added in this pass (see `role_inference.rs`) — treat `market rates`/inferred compensation bands as a rough guide, not authoritative data. A real market-data source (even a periodically-updated static dataset) would be a meaningful accuracy improvement over pure LLM estimation.
4. **No genuine low-spec/CPU-only hardware was used to validate the "works on any hardware" claim** — testing so far has been on capable development hardware. Needs validation on an actual 4GB-RAM/CPU-only machine.
5. **Lightning AI provider integration returns 401 Unauthorized** in every test so far — unconfirmed whether this is a request/auth-format bug in `src/engine/llm.rs` or an environment credential issue. Needs investigation with known-good Lightning credentials.
6. **The concurrent company-directory sweep (`scrape_companies`) caps in-flight requests per host (fixed — see `src/pipeline/scraper.rs`), but nothing else in the scraper does.** If board coverage grows to fan out across multiple hosts at once (e.g. sweeping Lever and Ashby company lists concurrently, not just Greenhouse), each host needs its own concurrency cap, not one global one — a shared global limit still lets one slow/rate-limiting host starve the others. Worth revisiting once a second concurrent multi-company sweep is added.

## Roadmap

> **Read this first.** The roadmap below is superseded in ordering by the architecture agreed on 2026-07-29. See [INFLECTION_ARCHITECTURE.md](INFLECTION_ARCHITECTURE.md) for the reasoning, [DECISIONS.md](DECISIONS.md) for what is settled and what is rejected, and the three layer specs in [design/](design/). The critical chain immediately below is the authoritative build order. Where anything further down this file disagrees with it, the critical chain wins.

### The critical chain

Each step is worthless without its predecessor. This ordering is forced, not preferential.

#### Step 0 — Foundation repair (blocking; nothing else is trustworthy until this lands)

- **Canonical content-addressed job IDs** replacing random v4 UUIDs ([ADR-001](DECISIONS.md#adr-001--job-identity-is-content-addressed-never-random)). Today the same posting scanned twice becomes two rows, the evaluation cache can never hit, and the daemon re-evaluates every job every hour forever. A live trial on 2026-07-29 found **8 of the top 20 recommendations were duplicates**.
- **PII gate at a single pre-upload choke point**, plus international detectors ([ADR-002](DECISIONS.md#adr-002--missing-data-is-represented-as-missing-never-as-a-plausible-default)). The file currently uploaded to Lightning AI is written *after* the only gate runs and is never checked.
- **Delete fabricated data**: `posted_at = Utc::now()`, fallback 0.5 evaluations, the hardcoded `roles research` archetype.
- **Stop swallowing scraper errors** ([ADR-003](DECISIONS.md#adr-003--errors-propagate-they-are-not-collapsed-into-empty-results)) — a network outage is currently reported as "no jobs found, try a different query".
- **Remove OpenSSL/native-tls** (#67) — still present via three paths.

#### Step 1 — Evidence layer → [design/EVIDENCE_LAYER.md](design/EVIDENCE_LAYER.md)

Tiered extraction ladder: CNAME enumeration → ATS JSON APIs → `__NEXT_DATA__` SSR hydration → Schema.org JSON-LD. Supplies real posting dates, real compensation, and structured restriction fields.

Closes #116, #58, #117, and **replaces** the maintained salary dataset in #119 — employer-supplied compensation has perfect provenance and needs no curation.

#### Step 2 — Calibration layer → [design/CALIBRATION_LAYER.md](design/CALIBRATION_LAYER.md)

Per-user empirical-Bayes conversion model fitted from data already captured (`edit_distance`, submission latency, pipeline/IMAP outcomes). Mandatory shrinkage toward published priors, interval reporting, and controllable-vs-structural factor decomposition.

Reframes #115, #48, #132; #119 survives only as the prior table.

#### Step 3 — Allocation layer → [design/ALLOCATION_LAYER.md](design/ALLOCATION_LAYER.md)

Min-cost max-flow slate generation: weekly effort budget, role-archetype diversification caps, `−log P(callback)` costs with age decay. Counterfactual re-solve replaces the heuristic preference-challenge engine.

Reframes #122, #133, #121, #106.

### Also open (independent of the chain)
- Cloud-provider model tiering for `--preset` (gap #2), plus the `config.rs` tier-collapse bug that currently forces all three tiers to one model.
- Low-spec hardware validation (#5/#57/#73) — the largest unbacked claim in the project.

### Career Coaching — the destination (delivered by Steps 2 and 3)

A built-in career coaching mode that goes beyond "find and tailor applications for the job you already know you want." This is the feature that makes ATSassin an earning coach rather than just an application optimizer, attacking all three enemies at once:

- **Continuous market watch**: a scheduled background scan (or lightweight daemon on `balanced`+ hardware) keeps an up-to-date view of open roles that match the user's profile and preferences. It does not wait for the user to run a command.
- **Preference-challenge insights**: when the data shows that a small change — relocating, switching to contract, picking up a named adjacent skill, or targeting a different industry — could materially improve income or prospects, ATSassin surfaces the finding as a question, not a prescription. Example: "Senior Rust engineers in Berlin report median compensation ~40% higher than your current market; would you consider relocation or remote hiring in that region?"
- **Earning-potential analysis**: given a candidate's real skills/experience, surface adjacent roles, industries, or arrangements (contract vs. FTE, remote vs. local market) they may not have considered that pay meaningfully more for skills they already have — grounded in real market data (see gap #3; this feature depends on solving that first), not hand-wavy LLM guessing.
- **Interests-outside-work discovery**: a structured, low-pressure conversation (not a generic "what are your hobbies" prompt) that surfaces genuine interests and turns them into a short list of *real, currently-open roles* that connect to those interests — using the existing role-inference and scan infrastructure, but seeded from interests rather than only from past job titles. The goal is surfacing options the candidate wouldn't have searched for themselves, not replacing their judgment.
- **Anti-atrophy retention**: the coaching loop is designed to keep users engaged even when they are not actively job-hunting — by helping them re-evaluate and re-validate their current position against the market, not by spamming them.
- Should reuse the existing `profile`/`roles`/`scan` pipeline rather than becoming a separate subsystem — coaching output should be able to flow directly into `evaluate`/`tailor` like any other discovered role.
- Explicitly **not** meant to auto-decide anything for the user — this is a discovery/expansion tool, consistent with the "recommends and tailors, never auto-applies" principle below.

### Later
- OpenVINO INT8 runtime for Intel Arc/Iris Xe
- Real model distillation pipeline (train a small 22M-109M local classifier from usage data)
- Browser-automation-based application submission (opt-in, explicit per-application confirmation — never silent)
- Multi-user / team collaboration
- WASM plugin system for custom matching logic
- Autonomous community LoRA sharing and provenance — see the Experimental section below

## Experimental / Long-term — Autonomous community LoRA sharing

> **Why this matters:** the goal is a free, autonomous earning optimizer for everyone. That only works if better source models (e.g. a Fable 5 distillate) naturally produce higher-ranked, more useful shared artifacts than weaker source models (e.g. a Llama 30b distillate), and if users never have to become ML engineers to benefit.

**Core trade-off: start light, prove the concept, then scale.**

Whole-model P2P/torrent distribution is too heavy for ATSassin's local-first, low-spec promise today — multi-gigabyte files, seeder economics, monolithic updates, and unverifiable provenance claims. The lighter path is to share **LoRA adapters** (10–200 MB) that apply to a base model the user already has locally. This keeps bandwidth, seeding, and incremental-update costs low, keeps the core Rust binary Python-free, and lets the project learn about provenance and trustless reputation before committing to a heavier distribution layer.

### Stage 0 — Local LoRA generation (foundations, no distribution)

Extends the existing `atsassin distill` command:

- Export high-confidence training pairs from local feedback and telemetry.
- Generate a ready-to-run external LoRA training script (e.g. Unsloth / llama.cpp / MLX), building on the current `train_unsloth.py` generation path.
- The user trains in their own Python environment; the resulting adapter stays local.
- This stage proves the artifact format and quality gate without any network sharing.

### Stage 1 — Read-only community registry (proof of concept)

- Host a static `registry.json` on a free, user-accessible layer (e.g. GitHub Pages, Cloudflare R2, or Hugging Face) listing community LoRA adapters.
- Each adapter ships with a signed/verifiable manifest:
  - `adapter_hash` (SHA-256 of the GGUF/Safetensors file)
  - `parent_model` / `parent_model_hash` (claimed base model; treated as a claim unless the publisher independently publishes a hash)
  - `teacher_lineage` (claimed teacher model, e.g. "Fable 5" or "Llama 30b")
  - `task_type` (e.g. `scoring`, `tailoring`, `cover_letter`)
  - optional `author_pubkey`
- ATSassin fetches the registry only when the user opts in (e.g. `[lora_registry] enabled = true` in config), validates hashes, downloads the best-matching adapter via `reqwest`, and applies it to the local Ollama base model (creating a new model variant with `FROM <base>\nADAPTER <path>`).
- Ranking in this stage is manual/curated.

### Stage 2 — Reputation-based ranking (immutable-ledger + DAO lessons)

- Use local telemetry and feedback to compute a **Proof-of-Quality** score: low edit distance, accepted outputs, and positive pipeline outcomes.
- Publish anonymized quality votes to a lightweight coordinator (reusing the same free-cloud ethos as the Compute Broker — e.g. Cloudflare Workers + D1, or similar free tier). No blockchain or token required.
- Rank adapters by empirical acceptance rate, not by claimed teacher. A "Fable 5 distillate" only stays on top if it actually wins in practice.
- Provenance is content-addressed: a DAG of manifest hashes. Learn from immutable ledgers and DAO governance models: reputation is social/usage-based, not token-based, and governance is client-enforced rather than on-chain.

### Stage 3 — Distributed sharing (DHT / P2P) — ❌ REJECTED 2026-07-29

**Do not implement. See [REJ-001](DECISIONS.md#rej-001--p2p--dht-distributed-crawling-libp2p-kademlia-skademlia-merkle-crdt) before proposing any variant of this.**

Rejected on four grounds, the first of which is disqualifying on its own: a DHT-coordinated politeness/rate-limit mechanism is a DDoS vector, because the protected resource (the target web server) sits outside the trust boundary and no honest-majority assumption can protect a non-participant. It also inverts the privacy architecture by making every user a publisher of scraped third-party PII, requires PoW node-ID generation that contradicts the 4 GB hardware floor, and has inverted bootstrap economics.

Adapter distribution stays on the HTTP registry (Stage 1) indefinitely.

### Stage 1b — Community job/salary/review pooling (optional, opt-in)

> **Why this matters:** today every ATSassin instance independently discovers the same job boards, social posts, and salary signals. Crowd-sourcing this layer lets the community pool discoveries while keeping each user's profile and application data local. It is a natural extension of the community registry used for LoRA adapters.

- **Shared signals, not shared PII:** only anonymized, non-attributable data about job postings, compensation ranges, company reviews, and board-detection patterns are pooled. Names, emails, resumes, and application materials never leave the local machine.
- **Board-discovery feed:** when a user opts in, their instance can publish newly-discovered board URLs, ATS detector patterns, and social-post sources to a shared registry. Other instances consume the feed to avoid re-discovering the same starting points.
- **Salary/review aggregation:** anonymized salary data and "post to avoid" flags can be pooled using the same content-addressed manifest + reputation mechanism as LoRA adapters. Each submission is signed by an author key and ranked by observed quality, not by popularity.
- **Reuse the same transport:** the registry/DHT stages below already provide an HTTP registry (Stage 1) and an optional DHT (Stage 3). The job-data payload is just another manifest type in the same registry.
- **Anti-spam / poisoning:** salary/review data is high-value and high-manipulation-risk. Before any pooled value is trusted, it must be corroborated across multiple independent instances or matched against a public source. Claims are treated as claims until corroborated.
- **Strictly opt-in and revocable:** users enable the feed in config; they can stop contributing and/or stop consuming at any time without affecting local functionality.

### Stage 4 — Volunteer local compute cooperative (BOINC-style, optional)

- Users may opt in to donate idle local CPU/GPU cycles for community tasks such as LoRA evaluation, quality-gate validation, and adapter seeding.
- This is strictly local compute only — it never routes another user's inference through a user's configured cloud API keys or free-tier quotas, and it never pools provider credentials.
- Cooperative work units operate on anonymized or public validation sets only; they never process a user's own profile, application materials, or scraped job data.
- The reward is not tokens or quota; it is a better set of community adapters whose quality is measured by the tool's ability to improve real-world outcomes (e.g. offer and interview conversion per application).
- Runs only when the machine is idle, respects hardware tiers (`light`/`balanced`/`full`), and can be disabled entirely in config.

### Guardrails for community sharing

| Concern | Mitigation |
|---|---|
| **PII leakage** | Scrub training pairs of names, emails, companies, and addresses before any shared adapter is created. Shared artifacts must never contain user-specific resume data. |
| **Malicious weights / RCE** | Accept only GGUF and Safetensors formats; reject pickled `.pt`/`.bin` files. Verify SHA-256 hashes. Consider a local sandboxed evaluation before applying an unknown adapter. |
| **Unverifiable provenance claims** | Treat "teacher model" as a claim, not truth. Rank by observed quality. Hashes guarantee integrity of the artifact, not honesty of the claim. |
| **Free-tier / ToS violations** | No automated sign-up for free credits. Use only user-configured providers and storage. Enforce rate/bandwidth caps in the Compute Broker. Respect storage provider ToS. |
| **Autonomy without consent** | Sharing is opt-in. Local-first is the default; nothing is uploaded, downloaded, or applied without explicit user configuration. |
| **Regulatory / privacy** | Any telemetry coordinator receives only anonymized quality signals, not resume text or personal data. GDPR/CCPA-style deletion is handled by not collecting PII in the first place. |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| LLM provider rate limits / outages | Medium | High | Multi-provider fallback, already implemented; offline/local-model mode via Ollama |
| Scraper/ATS-API changes breaking scan sources | Medium | Medium | Every scraper degrades to an honest empty result on failure, never fabricates (verified in UAT) |
| Compensation-estimate inaccuracy | Medium | Medium | Seniority-aware sanity clamp in place (this pass); real market-data source is the durable fix (gap #3) |
| Binary size growth | Low | Low | LTO + strip already in `[profile.release]`; currently 10.96MB (measured 2026-07-29) |
| Community trust (privacy) | Low | High | Local-first by default, no telemetry beyond opt-in local logging, MIT-licensed and open source |

## Ethical Considerations

1. **Privacy by design**: ATSassin runs locally by default. No resume data leaves the machine unless the user explicitly configures a cloud LLM provider.
2. **Transparency**: LLM calls are logged locally (`*.llm_telemetry.jsonl`); users can audit exactly what was sent and received.
3. **No automation overreach**: ATSassin recommends, evaluates, and tailors — it does not auto-apply on a user's behalf without explicit action, and any future application-automation feature (see Later) must stay opt-in and per-application confirmed.
4. **Accessibility**: free, lightweight (10.96MB, no GPU or Python/Node runtime required), designed to run on modest hardware — not a subscription SaaS product.
5. **Honesty over impressiveness**: this project has a documented history (see UAT report) of finding and removing fabricated/placeholder output disguised as real functionality. That standard should hold for all future contributions — an empty, honestly-labeled result is always preferable to a fabricated one.
