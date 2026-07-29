> ## ⚠️ HISTORICAL — do not pick up work from this document
>
> Predates the 2026-07-29 architecture reset. Kept for context only. Parts of it plan work that is now **rejected** — notably Stage 3 DHT/P2P (#49) and crowd-sourced pooling (#105), both closed under [REJ-001](DECISIONS.md).
>
> Issue numbers here are stale. For current direction see [DECISIONS.md](DECISIONS.md), [ROADMAP.md](ROADMAP.md#the-critical-chain) and the tracking epic **#156**. Where this document disagrees with those, they win.

---

# Design: Autonomous Closed-Loop Job & Income Optimization for ATSassin

**Status:** Reviewed and revised — see §0 before reading the rest of this document  
**Scope:** Architecture and roadmap for turning ATSassin from a user-triggered pipeline into a resourceful, autonomous, closed-loop system.  
**Principles:** Local-first, privacy-first, opt-in-only, lightweight by default, opportunistically scales to free cloud capacity.

---

## 0. Review notes (read this first)

The original draft was strong on breadth — every real capability gap is named — but several component designs would, if built as first described, break the property that makes ATSassin worth using: **`cargo build --release`, no Docker, no Python, no Node, runs unattended-free on a 4GB CPU laptop.** That property is untested even for *today's* simpler tool (issue #5 is still open), so nothing in this design should make it harder to eventually prove, and nothing should regress it while unproven. The edits below are not additions to the plan — they're the plan, tightened to be buildable without silently making the lightweight case a second-class citizen.

**Four changes made throughout, applied consistently:**

1. **Every new capability rides on the existing `HardwareProfile` tier system (`engine::hardware`), not a new resource concept.** If a feature needs a background process, network egress the user didn't request, or non-trivial RAM/CPU, it must check the hardware tier and degrade to "off, CLI-only, run me manually" below `balanced`. This is a few lines of code reusing what's already there, not new infrastructure — but it has to be a rule, not a hope, or "runs on any device" quietly becomes "runs on any device, except the new features."
2. **No component introduces a hand-maintained registry of external state that goes stale.** The Compute Broker's original provider table is exactly the same failure shape as the company-directory rot this project already found and fixed (issue #1) — a static list of free-tier terms that change without notice. Redesigned below to prefer self-discovery (a provider tells you its own limits) over a maintained list, with the list only as a bootstrap/fallback.
3. **Nothing that isn't already true gets a more confident story than it deserves.** Distillation stays "export data, run your own external tool" — matching what `atsassin distill` genuinely does today — rather than describing an in-process fine-tuning stack that would require pulling in a Python/PyTorch toolchain and quietly break the "no Python" claim in the README.
4. **The phased roadmap is re-cut so the first three phases need zero new daemon, zero new hard dependencies, and are individually small enough to be a good-first-issue for an external contributor** (see §12) — directly in service of keeping outside contributors like the one who opened PR #16 engaged with work they can actually pick up and ship, not a multi-week architecture project.

Where the original text is unchanged below, it was already consistent with these rules. Where changed, the reasoning is inline.

---

## 1. Problem statement

Today ATSassin is a **passive, user-triggered toolkit**: the user runs `scan`, `evaluate`, `tailor`, and updates the pipeline manually. The goal is to evolve it into a **closed-loop, autonomous engine** that:

1. Discovers job opportunities continuously.
2. Evaluates and ranks them against the user's profile and preferences.
3. Generates tailored application materials.
4. Tracks outcomes (interviews, offers, rejections).
5. Learns from outcomes to improve future recommendations and writing.
6. Does all of this while staying lightweight, private, and cheap — using local hardware first and free cloud capacity whenever available.

---

## 2. First-principles model: graph engineering

The design is expressed as a graph of **nodes**, **edges**, and **shared state**:

| Primitive | Meaning in ATSassin |
|---|---|
| **Nodes** | Specialized agents/workers: `Scraper`, `Evaluator`, `Ranker`, `Tailor`, `Actuator`, `OutcomeParser`, `Distiller`, `ComputeBroker`, `ResourceBroker`, `Orchestrator`. |
| **Edges** | Event-driven routing: a new job posting triggers evaluation; evaluation triggers ranking; ranking triggers queuing; a queued job may trigger actuation after human approval. |
| **Shared state** | SQLite (hot state), compressed cold archive, and distilled model weights. Every node reads from and writes to shared state; no node holds un-persisted truth. |

A single CLI invocation is a degenerate graph (one node). The target architecture is a persistent event graph that can run unattended.

---

## 3. Current state vs. target state

| Capability | Today | Target |
|---|---|---|
| Job discovery | User runs `scan` | Background daemon scans on schedule, reacts to triggers |
| Evaluation | User runs `evaluate` | Auto-evaluation of high-rank jobs |
| Tailoring | User runs `tailor` | Auto-tailoring for approved queue |
| Application submission | None (`automation.rs` is stubbed) | Human-in-the-loop actuation when enabled |
| Outcome tracking | Manual `pipeline update` | Email/inbox ingestion + browser extension |
| Model selection | Single configured provider | Dynamic routing across free/local/paid providers |
| Learning | LLM-quality feedback only | Outcome-based model calibration + distillation |
| Storage | Uncompressed SQLite + JSONL | Deduplicated, compressed, tiered hot/cold store |

---

## 4. Architecture overview

```
─────────────────────────────────────────────────────────────────┐
│                        User / TUI / CLI                         │
└─────────────────────────────┬───────────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────────┐
│                        Orchestrator Node                          │
│   Schedules work, routes events, enforces guardrails, requires    │
│   human approval for any actuation with real-world side effects.  │
└─────────────────────────────┬─────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
┌───────▼──────┐   ┌──────────▼─────────┐   ┌───────▼──────
│   Scraper    │   │      Evaluator     │   │   Tailor     │
│   Node       │   │      Node          │   │   Node       │
└───────┬───────┘   └────────┬─────────┘   └───────┬───────┘
        │                    │                     │
        └────────────────────┼─────────────────────┘
                             │
                ┌────────────▼────────────┐
                │   Compute Broker /      │
                │   Archive Manager       │
                │  Routes every task to   │
                │  the best available     │
                │  free/local/paid node.  │
                └────────────┬────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
┌───────▼──────┐   ┌─────────▼─────────┐   ┌──────▼───────┐
│ Outcome      │   │   Distiller /     │   │   Archive /  │
│ Parser Node  │   │   Model Shrinker  │   │   Compressor │
└──────────────┘   └─────────┬─────────┘   └──────────────┘
                             │
                ┌────────────▼────────────
                │      Shared State       │
                │  SQLite (hot) + cold    │
                │  compressed archive     │
                └─────────────────────────┘
```

**This diagram is the full target state, not the shape of what ships first.** The Orchestrator box only becomes a literal running process in Phase 3+ (§5.3, §9), and only on hardware that opts into it; through Phase 2 the same responsibilities are covered by plain CLI commands the user's own scheduler calls. Read every box below as "this responsibility exists somewhere," not "this is a new always-running component."

---

## 5. Component designs

### 5.1 Compute Broker

A new module (`src/engine/compute_broker.rs`) that routes LLM calls across whatever providers the user has actually configured (this project already supports Groq/Ollama/Kimi/GLM/Lightning/OpenRouter/OpenAI/Anthropic — the broker's job is choosing *which configured one*, not discovering new ones the user never gave a key for).

**Revised: quota comes from the provider at call time, not a maintained table.** The original design's `provider_quota` table listing per-provider free-tier terms is the same failure shape as the pre-fix company directory (issue #1): a static snapshot of external state that goes stale the moment a provider changes its terms, with no mechanism to notice. Free-tier limits change often and without notice — this table would need the same weekly-maintenance burden the company directory did, for a much larger and faster-moving set of providers. Prefer:

- **Self-reporting first**: most OpenAI-compatible providers return rate-limit state in response headers (`x-ratelimit-remaining-requests`, `retry-after`, etc.) or a 429 body. Record what the provider itself just told you, per call, in `provider_quota` — the table becomes an *observed-state cache*, not a hand-authored source of truth.
- **A small bootstrap list only for providers with no discoverable signal** (e.g. local Ollama, which has no quota concept at all — `tier_type = 'local'` always wins on cost/privacy and needs no tracking). Keep this list to the handful of providers already wired into `ModelRouter` today; resist the urge to pre-populate it with providers nobody has configured yet — that's exactly the "seed list nobody asked for" pattern that made the company directory stale.
- Directly closes issue #3 (`--preset` has no effect on hosted providers) as a side effect: once the broker knows a provider's actual capability/cost profile, `--preset` can map to "prefer cheapest/fastest provider that fits the task" instead of a fixed model name.

Data model (same shape, reframed as an observed cache rather than a maintained registry):

```sql
CREATE TABLE provider_quota (
    provider TEXT PRIMARY KEY,
    tier_type TEXT,              -- 'local', 'configured' (has a user-supplied key)
    remaining_requests INTEGER,  -- last value the provider itself reported, or NULL if unknown
    resets_at TEXT,              -- from the provider's own response, or NULL
    reliability_score REAL,      -- computed locally from telemetry.rs call history, not asserted
    last_observed TEXT
);
```

Providers: whatever's already in `config::LlmProvider` plus Ollama. No new provider gets a row until the user configures a key for it — the broker routes among what's actually usable, it doesn't advertise providers the user hasn't opted into.

**Recommend-don't-automate, for a later phase:** routing only among configured providers is right for the reasons above, but it leaves real value on the table — a user with no `GROQ_API_KEY` set is missing a genuinely free, fast provider they'd probably want. The broker should not sign up for that on the user's behalf (real ToS and credential-handling risk - see §8.3), but it could *recommend* it: "you have no free-tier cloud provider configured; Groq offers a free tier with no credit card at console.groq.com" printed once, informationally, the first time the broker notices every configured route is exhausted or absent. Purely advisory, no automation, and explicitly a Phase 1+ nice-to-have, not part of the Phase 1 MVP - noted here so a future reviewer doesn't mistake "we don't discover providers automatically" for "we never help the user find one."

**Paid-fallback policy (resolves Open Question #1 below rather than leaving it open):** no paid provider is ever used without the user explicitly setting a `allow_paid = true` flag for that specific provider in config - not a global switch, not an implicit fallback when free/local options are exhausted. Exhausting free capacity should surface as "no capacity available, configure a provider or wait for reset," never a silent switch to a provider that bills the user. This is a default worth stating plainly rather than leaving as an open question a later implementer has to guess at.

### 5.2 Archive Manager *(renamed from "Resource Broker")*

Renamed because, with cloud-storage arbitrage removed from scope (see below), "Resource Broker" over-promises relative to what this component actually does — a local compression/archival scheduler, not a broker choosing among multiple resource providers. If cloud archival is ever built (see the deferred item below), it re-earns a broker-shaped name at that point; until then, call it what it is.

**Revised: scoped down.** The original draft's cloud-storage/compute arbitrage (R2, B2, GitHub Actions minutes, HF endpoints) solves a problem this project doesn't have yet — a single user's job-search telemetry is realistically low tens of MB, not something that needs a multi-provider cloud-storage broker. Building that now is solving for scale that doesn't exist while adding real cost: it requires the user to already have accounts/credentials for services unrelated to job search (contradicting "single binary, zero setup"), and several of those free tiers explicitly restrict automated/bulk usage in their ToS — the same "never evade rate limits or ToS" principle this design already commits to in §7 cuts against automating signup-free usage of them without a human confirming each provider relationship first.

**What actually needs building, in order:**
1. **Local compression only, into a separate cold table, not an in-place blob swap.** Concretely: rows older than 30 days are removed from the hot `telemetry`/`feedback` tables, `zstd`-compressed, and written to a new `telemetry_archive` table (`id, source_table, compressed_blob, original_row_count, compressed_at`) keyed so they can be decompressed and rehydrated on demand. This is the deliberate choice, not "just gzip the column in place": hot-table queries (recent telemetry, active pipeline reads) stay on uncompressed rows and pay zero decompression cost on the common path; only the rare "look at old history" query touches the archive table and pays to decompress. An in-place blob column would make *every* read of that column pay a decompression cost, including the ones that hit it most. This distinction belongs in the Phase 2 issue (#27) directly, not just this doc, since it changes the schema, not just the compression step.
2. **Cloud archival stays fully out of scope until local compression is shipped and someone's local DB has actually grown large enough to need it.** If/when that's real, it should be a single explicit `--archive-to <provider>` opt-in per user, not a broker silently choosing among providers on their behalf — the whole point of "opt-in-only" in this document's stated principles.

This section intentionally does less than the original draft. That's the point.

### 5.3 Orchestrator

**Revised: no new daemon for the first three phases.** "Scheduled, unattended work" does not require a persistent background process — cron (Linux/macOS) and Task Scheduler (Windows) already do that, and calling an existing CLI subcommand on a timer is the actual mechanism, not a new one. A daemon means a long-lived process holding memory and, likely, a network connection, on a machine that might be the 4GB CPU laptop this project claims to support and has never verified against (issue #5). That's a real cost that should be paid only once there's a proven need for genuinely event-driven behavior (e.g. reacting to an email arriving mid-scan, not just "run every 6 hours").

**Phased:**
- **Phases 0-2 (outcome ingestion, Compute Broker, local compression): zero daemon.** `atsassin outcomes sync` and `atsassin scan --auto` (queues without prompting, still requires `pipeline update`/actuation to be manual) are plain CLI subcommands a user's own cron/Task Scheduler calls. This is also why these phases are safe to hand to an external contributor — no long-running-process design, testing, or lifecycle-management expertise required, just CLI commands like every other feature in the codebase today.
- **Phase 3+ (`atsassin daemon`, if it turns out to be needed):** gated behind `HardwareProfile::detect().tier >= Balanced` (reusing `engine::hardware`, not a new concept) — on a `light`-tier machine, `atsassin daemon` prints "this device is better suited to scheduled CLI calls than a background process; see the README for a cron example" and exits, rather than running anyway and eating the RAM budget a constrained device doesn't have.

**✅ COMPLETED:** The daemon is now fully implemented with the complete autonomous-loop workflow:
- Scan boards on schedule
- Evaluate and rank new jobs using prerank + LLM scoring
- Queue high-quality jobs for auto-tailoring (with configurable threshold)
- Trigger follow-ups based on pipeline status and elapsed time
- Sync IMAP outcomes for pipeline status updates
- Route tasks through the Compute Broker
- Hardware-gated to Balanced/Full tiers only

Responsibilities (unchanged from original draft, now explicitly scoped to whichever of the two modes above is active):
- Poll boards on a schedule.
- Evaluate and rank new jobs.
- Queue high-quality jobs for tailoring/actuation.
- Trigger follow-ups based on pipeline status and elapsed time.
- Require explicit user approval before any external actuation.
- Route tasks through the Compute Broker.

Guardrails:
- No application is submitted without user approval.
- Rate limits are enforced per board and per provider.
- Every automated action is logged.

### 5.4 Outcome ingestion

Closes the loop by reading real-world outcomes. **This is the highest-leverage phase and the best-scoped one for an external contributor to start on** — it's parsing and classification against a local mailbox, no daemon, no browser automation, no new hard dependencies beyond an IMAP client crate.

**Off by default.** Every other opt-in boundary in this design (paid providers, cloud archival, actuation) is explicit; outcome ingestion needs the same treatment and the original draft didn't say so outright. Nothing reads the user's mailbox until they explicitly run `atsassin outcomes connect` and store a credential - there is no ambient/automatic mailbox access, ever, and no other command should trigger it as a side effect.

Sources:
- IMAP email parsing for rejection/interview/offer emails (start with plain IMAP + app-password auth, which every major provider supports without OAuth complexity; add OAuth for Gmail/Outlook as a later, separate task once IMAP+app-password proves the classification logic works).
- Browser extension or MCP hook to detect when the user visits an ATS page and updates status — deferred behind IMAP ingestion; it's a second, independent source, not a blocker for the first.

**Credential handling (not in the original draft, and it needs to be explicit before any code lands):** IMAP access means holding a credential that reaches the user's entire inbox, not just job-related mail — a materially bigger trust boundary than anything else in this document, including the resume/PII data ATSassin already stores. Non-negotiable for the first implementation:
- App-password or OAuth token only, **never** the account's actual login password.
- Credential stored using the OS keychain (`keyring` crate or equivalent), not `config.toml` or SQLite in plaintext.
- IMAP fetch is read-only (`\Seen` flags untouched, nothing deleted or moved) and scoped to a search query (sender/subject heuristics for ATS-pattern senders), not a full mailbox sync.
- Classification (rejection vs. interview vs. offer) runs locally — parsed email bodies are exactly the kind of sensitive data that should never leave the device via a cloud LLM call without the same opt-in the rest of this design already requires for other sensitive routing.

Result:
- Pipeline statuses are updated automatically.
- Outcome signal feeds back into `feedback.rs` and `telemetry.rs`.
- Model calibration improves scoring/tailoring quality over time.

### 5.5 Actuation layer

Extends `src/pipeline/automation.rs` from stubs to a real browser automation node.

**Revised: item 4 (auto-submit) is downgraded from "phase 4 deliverable" to "not planned; revisit only if the first three phases prove insufficient."** This project has already stated the principle "recommends and tailors, never auto-applies" (see `ROADMAP.md`), and it's grounded in more than caution: LinkedIn and most ATS platforms' terms of service prohibit automated application submission, and an account getting flagged/banned for it is a real, user-facing cost this design would be introducing, not just a risk to caveat. "Explicitly approved jobs" doesn't remove the ToS exposure — the automation is still what's submitting, whether or not a human clicked "approve" upstream of it.

Phased approach:
1. Draft generation and human approval queue (default, and — per the point above — the durable target state, not a stepping stone to something else).
2. Assisted form filling via Chrome DevTools Protocol + vision-capable model: fills the form, stops before the submit control, same boundary already used successfully this session for a real application (Claude driving the user's own browser, human clicks send).
3. Per-site adapters for Greenhouse, Lever, Ashby, Workday — improves fill accuracy for phase 2, does not change the human-submits boundary.
4. ~~Optional auto-submit~~ — removed from this design. If a future need is found to revisit this, it deserves its own design doc and explicit user sign-off, not a bullet point inherited from an earlier draft.

Guardrails:
- A human submits every application. No exceptions in this design.
- CAPTCHA detection pauses and asks for human help.
- Rate limiting prevents account bans on the fill-assistance requests themselves (still real even without auto-submit).

### 5.6 Distillation / model shrinkage

Use the persisted telemetry stream to improve toward smaller, user-specific models.

**Revised: stays "export + external tool," matching what `atsassin distill` already does today, not a new in-process fine-tuning stack.** Steps 3-5 of the original draft (fine-tune via LoRA, evaluate against a teacher model, deploy) describe a genuine ML training pipeline — that means pulling in a Python/PyTorch-based toolchain (`unsloth`, `PEFT`, or similar; there is no mature Rust-native LoRA fine-tuning story as of writing) as a hard dependency of the project. That directly breaks the README's current, real claim — "no Docker, no Python, no Node" — for every user, not just the ones who want distillation. Training belongs *outside* the single binary:

**✅ COMPLETED:** The distillation pipeline is fully implemented:
1. Collect `(input, output, task, feedback)` tuples from `telemetry.rs` and `feedback.rs` — already what `atsassin distill` does.
2. Filter high-quality pairs: accepted outputs, low edit distance, positive outcomes (now also fed by the pipeline-status → feedback wiring shipped this session).
3. `atsassin distill` exports the filtered pairs **plus ready-to-run external training scripts** targeting well-supported external tools:
   - ONNX conversion script with dependency checking
   - GGUF quantization script with llama.cpp integration
   - OpenVINO export script for Intel hardware
   - Unsloth training script template
4. Evaluating the resulting checkpoint and deciding whether to point `ModelRouter` at it stays in-binary (that part is just config + a benchmark comparison, no training runtime needed) — this is the piece worth building in Rust.
5. Use the small model as the default when it passes the quality gate; escalate to larger/cloud models via the Compute Broker when confidence is low.

**✅ COMPLETED:** PII scrubbing is integrated before any export:
- All training pairs are scrubbed using `pii_scrubber.rs`
- A PII gate validates final output and aborts if any detectable PII remains
- Context-aware preservation for target companies

Storage optimization:
- Deduplicate prompts.
- Store tailored outputs as diffs against the base profile.
- Local `zstd` compression for cold data (see the revised §5.2) — LLM-based summarization of telemetry is deferred: it would mean spending real LLM budget to compress data whose entire purpose is to be cheap-to-keep, which is backwards.

### 5.6b Community LoRA sharing and provenance (experimental)

This is the long-term, autonomous extension of the distillation flywheel: users benefit from LoRA adapters produced by others, and better source models naturally produce higher-ranked artifacts without requiring a blockchain or whole-model P2P.

**Artifact.** A LoRA adapter is a small (10–200 MB) GGUF or Safetensors file plus a manifest:

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

**Autonomous discovery and apply.** ATSassin fetches a registry (HTTP first; DHT later) only when the user opts in via config, validates the adapter hash, downloads it via `reqwest`, and creates a local Ollama model variant (`FROM <base>\nADAPTER <path>`). The `ModelRouter` then routes the appropriate task to that variant. No Python runtime is needed in the core binary.

**Provenance without impossible cryptography.** The manifest records lineage claims, but lineage is treated as *claimed*, not *verified*. Verification is replaced by reputation:
- Content hashes guarantee artifact integrity.
- Local telemetry/feedback produce a Proof-of-Quality score: accepted outputs / total uses, weighted by outcome signal.
- Anonymized quality votes flow to a lightweight, free-cloud coordinator (reusing the same free-tier/quota-aware ethos as the Compute Broker).
- Adapters rank by empirical acceptance; a "Fable 5 distillate" only stays on top if it wins in practice.

**Lessons from ledgers, DAOs, and BOINC.**
- **Immutable hashed ledgers**: use content-addressed manifest DAGs and SHA-256 verification for artifact integrity. This provides auditability without needing a blockchain.
- **DAO governance**: reputation and ranking are client-enforced, usage-based, and opt-in. There is no token, no on-chain voting, and no delegated trust. "Governance" is simply "clients don't download low-reputation artifacts."
- **BOINC**: volunteer compute/storage is fine for non-critical, best-effort sharing, but the network must degrade gracefully when volunteers leave. BitTorrent-style swarms need active seeders, so the design starts with a small HTTP registry and only moves to DHT when there is enough participation to keep swarms alive.

**Phasing.** Start with a read-only community registry (Stage 1), add reputation-based ranking (Stage 2), and only introduce DHT/P2P transport once adapter volume justifies it (Stage 3). Because the artifact format is the same at every stage, the transport can be swapped without touching the rest of the system.

**Guardrails.** See `ROADMAP.md` under *Experimental — Autonomous community LoRA sharing* for the full table. In short: PII scrubbing before any shared artifact is created, only GGUF/Safetensors accepted, hashes verified, sharing opt-in, and bandwidth capped by the Compute Broker to protect free-tier/metered users.

### 5.7 Persistence compression

Prevent the local database from growing unbounded.

Techniques:
- Deduplication of job postings and prompts.
- Delta encoding for tailored outputs.
- Embedding + summary for long texts instead of full raw text.
- Periodic cold archival of old telemetry to free cloud storage.

---
## 6. Data model

The existing SQLite schema is extended with:

```sql
-- Provider/resource quotas
CREATE TABLE provider_quota (...);

-- Distilled model checkpoints and quality gates
CREATE TABLE model_checkpoints (
    id TEXT PRIMARY KEY,
    task TEXT,
    model_path TEXT,
    base_model TEXT,
    teacher_model TEXT,
    quality_delta_pp REAL,
    evaluated_at TEXT
);

-- Archived telemetry references (cold storage)
CREATE TABLE telemetry_archives (
    id TEXT PRIMARY KEY,
    archive_path TEXT,
    provider TEXT,
    compressed_size_bytes INTEGER,
    original_size_bytes INTEGER,
    archived_at TEXT
);
```

---

## 7. Privacy, security, and ethics

| Concern | Mitigation |
|---|---|
| Resume/PII exposure | Local-first by default; no raw data leaves device without opt-in. |
| Free-tier data training | Flag providers that train on free-tier data; route sensitive tasks to local/Ollama. |
| Cloud storage of archives | Encrypt client-side before upload. |
| Auto-submission | Explicit per-job approval; never silent. |
| Rate limits / ToS | Hard caps per provider and site; never evade. |
| Model bias | Quality gates and outcome-based calibration; human remains in control. |

---

## 8. Explicitly out of scope

The following were considered and rejected for this design:

### 8.1 Blockchain for federated learning coordination

Rejected because:
- Blockchains are designed for consensus on small state, not high-bandwidth gradient/model-update aggregation.
- On-chain storage of model updates is prohibitively expensive.
- ZK proofs of valid training are compute-prohibitive.
- Adds enormous complexity without proportional benefit.

### 8.2 Peer-to-peer federated learning

Rejected because:
- Model updates can leak PII unless strong differential privacy is applied, which degrades model quality.
- Sybil and poisoning attacks are hard to prevent without a trusted aggregator.
- Network overhead conflicts with the lightweight, runs-on-any-device goal.

### 8.3 Account automation for free credits

Rejected because:
- Automating signup for free credits violates most providers' terms of service.
- Requires email/phone/CAPTCHA handling that is brittle and ethically questionable.
- The design instead uses keys the user already possesses.

---

## 9. Phased roadmap

Re-cut so phases 0-2 need no daemon, no new hard dependencies, and no cloud accounts beyond what the user already configures for LLM providers — each is independently shippable and reviewable, which matters as much for keeping the project honest as for keeping contributors able to pick work up (see §12).

### Phase 0: Close the outcome loop
- IMAP email ingestion for rejection/interview/offer detection, app-password/OAuth only, OS-keychain credential storage, read-only mailbox access.
- Automatic pipeline status updates (already has somewhere to feed into: pipeline-status → feedback wiring shipped this session).
- Plain CLI command (`atsassin outcomes sync`), user's own cron/Task Scheduler runs it on a schedule if they want automation - no daemon.
- This alone transforms ATSassin from a manual tracker into a self-improving one, and is the best-scoped phase for an external contributor to start on.

### Phase 1: Compute Broker + dynamic routing
- `src/engine/compute_broker.rs`, routing only among providers the user has actually configured.
- Quota tracked as an *observed* cache (provider self-reports via response headers), not a hand-maintained registry - avoids repeating the company-directory rot this project already fixed once.
- Multi-provider routing in `ModelRouter`; closes issue #3 (`--preset` doing nothing on hosted providers) as a direct side effect.
- `atsassin compute status` CLI command.

### Phase 2: Local compression
- `zstd`-compress telemetry/archive rows older than 30 days, in place, in SQLite.
- Cloud archival explicitly deferred - see the revised §5.2 - until local compression ships and someone's DB has actually grown large enough to need it.

### Phase 3: Orchestrator daemon (optional, hardware-gated)
- `atsassin daemon`, gated behind `HardwareProfile::detect().tier >= Balanced` - refuses to run and points to the cron-based Phase 0-2 workflow on constrained hardware instead of consuming its RAM/CPU budget.
- Event-driven graph of nodes, if and when genuinely event-driven behavior (not just "run every N hours," which Phase 0-2's cron approach already covers) is proven necessary.
- Human-in-the-loop approval for actuation.

### Phase 4: Actuation (assistive only - see revised §5.5)
- Real browser automation with vision-capable DOM reasoning, form-filling only.
- Per-site ATS adapters for Greenhouse, Lever, Ashby, Workday.
- No auto-submit path. A human submits every application, permanently, not as an interim state.

### Phase 5: Distillation flywheel (external training, in-binary evaluation)
- `atsassin distill` exports filtered high-quality pairs plus a ready-to-run external training script (extends what the command already does today).
- Training itself runs in the user's own external Python/LoRA environment, not in the ATSassin binary.
- Quality-gate evaluation and `ModelRouter` checkpoint selection stay in-binary - no training runtime dependency added to the core project.

---

## 10. Open questions

1. ~~Should paid-fallback providers be auto-used when free tiers are exhausted, or should they require explicit per-session approval?~~ **Resolved, not left open:** no paid provider is used without an explicit per-provider `allow_paid = true` config flag - see §5.1. Exhausted free/local capacity surfaces as "no capacity available," never a silent paid fallback.
2. Which external training stack should `atsassin distill`'s generated script target first - `llama.cpp` LoRA tooling, `mlx-lm` (Apple Silicon), or `unsloth`? Probably start with whichever the existing script-generation path already targets, if any, and extend from there.
3. ~~Should cold-archive encryption keys be derived from a user password, or stored alongside the config?~~ Deferred with cloud archival itself (§5.2) - not a question worth answering before there's a cloud archive to encrypt.
4. What is the acceptable quality-drop threshold for deploying a distilled model?
5. IMAP + app-password covers most providers day one; which OAuth flow (Gmail, Outlook) is worth the extra implementation cost first, based on what contributors' and early users' actual mail providers turn out to be?
6. *(New)* At what local-DB size does Phase 2's local `zstd` compression stop being sufficient, in practice, for a typical user - i.e. when does the deferred cloud-archival question in §5.2 actually become live?

---

## 11. Summary

This design turns ATSassin into a **resourceful autonomous system** without becoming a heavier one for anyone not opting in: it uses the cheapest capable resource available (local hardware first, configured providers second, paid only when the user explicitly allows it), compresses its experience locally before ever considering the cloud, hands training off to external tools rather than absorbing a Python toolchain, and closes the loop on real-world outcomes — all while keeping the CLI-only, no-daemon, no-new-accounts path fully functional as the default, not a legacy mode being phased out.

The next recommended step is **Phase 0 (outcome ingestion)** - highest leverage, smallest footprint, and the best-scoped phase to hand to an external contributor first (see §12). The Compute Broker (Phase 1) is the right second step once Phase 0 has proven the pattern of shipping these phases as plain, reviewable CLI additions.

---

## 12. Keeping external contributors engaged

PR #16 (a real, well-scoped, correctly-targeted external contribution) is the reason this section exists: the fastest way to lose a contributor who's already shown up is to make the next thing to work on either too vague or too large to finish in an evening. Applied to this design:

- **Phase 0 decomposes into several independently-mergeable, good-first-issue-sized pieces**, each small enough to review in one sitting: (a) IMAP connection + credential storage via OS keychain, (b) rejection/interview/offer email classification (a fixture-driven parsing problem, no LLM required for a first pass - keyword/pattern matching against real example emails, similar in spirit to the header-aware CSV fix that closed issue #15), (c) wiring classified outcomes into `pipeline update`, (d) the `atsassin outcomes sync` CLI command itself. File each as its own issue, cross-linked, rather than one large "implement Phase 0" issue.
- **Every phase in §9 stays a plain CLI command through Phase 2** - no contributor needs to understand daemon lifecycle, event graphs, or background-process testing to contribute to the highest-value 60% of this roadmap.
- **The Compute Broker's provider-observation design (§5.1) is naturally forkable per-provider** - "add self-reporting quota parsing for provider X" is a contained, well-bounded unit of work once the broker's interface exists, the same shape as the company-directory good-first-issues that already worked well this session.
- When Phase 0 ships, credit contributors by name in the PR/issue and in any release notes - cheap, genuine, and the single highest-ROI thing for keeping people coming back.
