# Roadmap, Known Issues & Ethical Considerations

> **Ground truth for contributors:** the two most reliable documents in this repo are [UAT_REPORT_2026-07-24.md](UAT_REPORT_2026-07-24.md) (issues found and fixed via real end-to-end testing across 5 personas) and [COMPETITIVE_BENCHMARK_2026-07-25.md](COMPETITIVE_BENCHMARK_2026-07-25.md) (ATSassin actually run head-to-head against 7 competitor tools). Where this roadmap and older planning docs (`COMPARISON.md`, `EXECUTIVE_SUMMARY.md`, `RANKING.md`, `COMPLIANCE_MATRIX.md`, `../AUDIT.md`) disagree, trust the UAT/benchmark reports — they reflect what was actually run and observed, not what was planned or claimed. `AUDIT.md` in particular is a point-in-time red-team note from an early draft (2026-07-23) predating the UAT/benchmark work below — most of the gaps it lists have since been closed; read it as history, not a current status report.
>
> **For the structural plan behind staying ahead** (not just the list of open items below), see [CATEGORY_LEADERSHIP_ROADMAP.md](CATEGORY_LEADERSHIP_ROADMAP.md) — a red-team pass on where today's competitive position is fragile (undocumented API drift, a one-time benchmark snapshot, docs already drifting from shipped features) and what closes each gap structurally.

## What's real today (verified by actual execution, not just present in the code)

- Profile parsing (Markdown/plain-text resumes, LinkedIn export CSVs, DOCX)
- Local-relevance role inference and pipeline tracking (SQLite), evaluate and tailor via any OpenAI-compatible provider (Groq, Ollama, Kimi, GLM, Lightning, etc.)
- Job discovery: LinkedIn guest API, Seek, single-company Greenhouse/Lever/Ashby lookups, a curated **concurrent zero-token sweep across ~35 companies' public Greenhouse APIs** (`--boards companies`), plus a social/aggregator sweep (HN, Reddit, RemoteOK, WeWorkRemotely, etc.)
- A full TUI dashboard: live role inference, scan, evaluate, and tailor, all triggerable from within the TUI, with a real pipeline summary and activity log
- Verified end-to-end for 5 realistic, diverse personas (see UAT report) with zero crashes and non-fabricated output

## Known issues / open gaps

These are real, currently-open items, not resolved yet:

1. **The curated company-to-board directory (`src/pipeline/company_directory.rs`) is a maintenance liability, not just incomplete.** It's currently a flat, hand-maintained list of ~35 company/slug pairs. Even fully filled out, which company sits on which ATS (Greenhouse/Lever/Ashby/Workable) changes constantly as companies migrate providers, so a static list goes stale within months regardless of how many entries it has. **Better direction (raised in community review):** derive the directory instead of maintaining it — fetch a company's public careers page and pattern-match the embedded board URL (Greenhouse/Lever/Ashby/Workable each have a recognizable embed/redirect shape) to detect which provider and slug they use. This turns "add more companies" from a permanent, weekly-recurring chore into a one-time detector. **Good first issue** in the meantime: adding more company/slug pairs to the existing list is still useful and needs no architecture change — just lower-leverage than the detector.
2. **`--preset` (lightweight/balanced/full) has no effect on model choice when using a hosted cloud provider** — it only changes timeout/retry/scrape-limit values. The hardware-adaptive tiering system only differentiates behavior when running against local Ollama. This should either be documented more prominently or extended so cloud presets pick different model *sizes* per tier (e.g. a cheaper/faster Groq model for `lightweight`).
3. **Compensation estimates are model-generated and can still be wrong even after the seniority-aware sanity clamp** added in this pass (see `role_inference.rs`) — treat `market rates`/inferred compensation bands as a rough guide, not authoritative data. A real market-data source (even a periodically-updated static dataset) would be a meaningful accuracy improvement over pure LLM estimation.
4. **No genuine low-spec/CPU-only hardware was used to validate the "works on any hardware" claim** — testing so far has been on capable development hardware. Needs validation on an actual 4GB-RAM/CPU-only machine.
5. **Lightning AI provider integration returns 401 Unauthorized** in every test so far — unconfirmed whether this is a request/auth-format bug in `src/engine/llm.rs` or an environment credential issue. Needs investigation with known-good Lightning credentials.
6. **The concurrent company-directory sweep (`scrape_companies`) caps in-flight requests per host (fixed — see `src/pipeline/scraper.rs`), but nothing else in the scraper does.** If board coverage grows to fan out across multiple hosts at once (e.g. sweeping Lever and Ashby company lists concurrently, not just Greenhouse), each host needs its own concurrency cap, not one global one — a shared global limit still lets one slow/rate-limiting host starve the others. Worth revisiting once a second concurrent multi-company sweep is added.

## Roadmap

### Now
- Company-directory-as-detector (see gap #1 above) — replaces a permanently-stale hand-maintained list with something derived from each company's own careers page.
- Cloud-provider model tiering for `--preset` (gap #2).

### Next — Career Coaching (flagship feature request)

A built-in career coaching mode, going beyond "find and tailor applications for the job you already know you want" toward helping a candidate figure out what they actually want and whether their current position is still the best option:

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

### Stage 3 — Distributed sharing (DHT / P2P)

- Once adapter volume justifies it, layer in a Kademlia/BitTorrent-style DHT (e.g. via `rust-libp2p`) so nodes can announce, discover, and share adapters directly.
- Always fall back to the HTTP registry when the DHT is unreachable.
- The Compute Broker's existing quota/bandwidth awareness should cap P2P egress to protect metered/free-tier users.
- Keep the artifact the same (LoRA + manifest), so the distribution transport can be swapped without changing the rest of the system.

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
| Binary size growth | Low | Low | LTO + strip already in `[profile.release]`; currently ~9.5MB |
| Community trust (privacy) | Low | High | Local-first by default, no telemetry beyond opt-in local logging, MIT-licensed and open source |

## Ethical Considerations

1. **Privacy by design**: ATSassin runs locally by default. No resume data leaves the machine unless the user explicitly configures a cloud LLM provider.
2. **Transparency**: LLM calls are logged locally (`*.llm_telemetry.jsonl`); users can audit exactly what was sent and received.
3. **No automation overreach**: ATSassin recommends, evaluates, and tailors — it does not auto-apply on a user's behalf without explicit action, and any future application-automation feature (see Later) must stay opt-in and per-application confirmed.
4. **Accessibility**: free, lightweight (~9.5MB, no GPU or Python/Node runtime required), designed to run on modest hardware — not a subscription SaaS product.
5. **Honesty over impressiveness**: this project has a documented history (see UAT report) of finding and removing fabricated/placeholder output disguised as real functionality. That standard should hold for all future contributions — an empty, honestly-labeled result is always preferable to a fabricated one.
