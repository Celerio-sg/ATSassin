#!/usr/bin/env python3
"""Batch-create comprehensive GitHub issues for ATSassin."""
import subprocess
import shutil
import tempfile
import os

REPO = "Celerio-sg/ATSassin"

def create_issue(title, body, labels):
    """Create a GitHub issue."""
    cmd = [
        "gh", "issue", "create",
        "--repo", REPO,
        "--title", title,
        "--label", labels,
    ]
    # Write body to temp file to avoid shell escaping issues
    with tempfile.NamedTemporaryFile(mode="w", suffix=".md", delete=False) as f:
        f.write(body)
        f.flush()
        body_path = f.name

    try:
        cmd.extend(["--body-file", body_path])
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
        if result.returncode == 0:
            print(f"  OK [{labels}] {title}")
            print(f"     {result.stdout.strip()}")
        else:
            print(f"  FAIL [{labels}] {title}")
            print(f"     {result.stderr.strip()}")
    finally:
        os.unlink(body_path)


# ============================================================================
# GROUP A: DISTILLATION PIPELINE (special depth - user flagged as under-considered)
# ============================================================================

create_issue(
    "[Design] Training dataset curation pipeline — deduplication, quality thresholds & balanced task distribution",
    """## Background
The current `export_from_feedback_and_telemetry` in `src/engine/distillation.rs` reads feedback events and writes them as training pairs. This is a good start but lacks proper curation:

- No deduplication of near-identical pairs
- No minimum-confidence threshold before a pair enters the training set
- No balancing across task types (role_inference, scoring, tailoring, deep_research)
- No stratification by model tier (light/balanced/full)

This can produce a training set that over-represents easy scoring tasks and under-represents difficult tailoring tasks.

## Goals
- User/role agnostic: the curation pipeline must work identically for any profile
- Deterministic: same feedback should produce the same curated set
- Minimal: zero external ML dependencies — pure Rust with SQLite

## Acceptance Criteria
- [ ] Design a deduplication strategy for training pairs (edit-distance or embedding-similarity threshold)
- [ ] Implement configurable confidence floor (default 0.6) below which pairs are excluded
- [ ] Implement task-type balancing so no task exceeds 40% of the curated set
- [ ] Export curated manifest alongside the raw JSONL for audit
- [ ] Add unit tests with synthetic feedback data covering dedup, thresholding, and balancing
- [ ] Document the curation pipeline in CONTRIBUTING.md

## Related
- Depends on #46 (Stage 0: Local LoRA generation foundation)
- See docs/DESIGN_autonomous_loop.md Section 5.3 (Distillation cycle)""",
    "enhancement,design:autonomous-loop,area:models,area:distillation"
)

create_issue(
    "[Design] Automated student model training workflow — end-to-end local fine-tuning",
    """## Background
The current `atsassin distill` generates synthetic pairs and external conversion scripts (ONNX/GGUF/OpenVINO) but does NOT perform any actual training. The user must manually run external tools. For ATSassin to be truly autonomous, it should guide — and eventually execute — the full training cycle.

Target model sizes: 22M, 109M, 1.5B parameters (chosen to run on consumer hardware).

## Goals
- User/role agnostic: same workflow regardless of profile content
- Privacy-preserving: all training data stays local by default
- Progressive: start with script generation (current), graduate to guided execution, then to fully automated

## Acceptance Criteria
- [ ] Document the current training gap: scripts are generated but not run
- [ ] Add `atsassin distill --train` that optionally steps through the training process with user prompts
- [ ] For each training target (22M/109M/1.5B), estimate and display RAM/disk requirements before starting
- [ ] Support local training via Ollama/Llama.cpp built-in fine-tuning capabilities
- [ ] Support cloud training via Lightning AI (already integrated but untested — see #85)
- [ ] Add a training progress indicator (polling job status for cloud, parsing log output for local)
- [ ] Export the trained adapter as a GGUF or LoRA file that ATSassin can load
- [ ] Add tests for the training flow with a mocked training backend

## Related
- Blocks #46 (Stage 0) and #47 (Stage 1)
- Depends on training dataset curation pipeline
- See docs/DESIGN_autonomous_loop.md Section 5.3""",
    "enhancement,design:autonomous-loop,area:models,area:distillation"
)

create_issue(
    "[Design] Distillation evaluation harness & benchmark — automated quality gate",
    """## Background
The current `DistillationPipeline::evaluate_quality_gate` checks whether quantized accuracy stays within 1pp of baseline. This is a good start but only covers quantization — it does not evaluate whether the distilled model actually performs better than the original on real tasks.

A proper evaluation harness should:
- Run representative tasks from each task type (role_inference, scoring, tailoring)
- Compare output against ground-truth or a trusted baseline model
- Measure not just accuracy but also latency, RAM usage, and output quality
- Fail the pipeline if quality regresses beyond a configurable threshold

## Goals
- User/role agnostic: evaluation tasks are synthetic/fixture-driven, not profile-dependent
- Deterministic: same model should produce the same evaluation results
- Fast: complete evaluation in under 60 seconds for 22M models

## Acceptance Criteria
- [ ] Design a fixture-based evaluation dataset with known-correct outputs for each task type
- [ ] Implement `atsassin distill evaluate` that runs the evaluation harness
- [ ] Measure and report: accuracy, latency (p50/p95), RAM usage, output length consistency
- [ ] Compare against a configurable baseline model (default: the current production model)
- [ ] Export evaluation results as JSON for CI consumption
- [ ] Add a quality gate that fails if any metric regresses beyond threshold
- [ ] Add integration tests that run the harness against a small local model
- [ ] Document the evaluation methodology in docs/EVALUATION.md

## Related
- Depends on training dataset curation pipeline (quality evaluation needs ground-truth pairs)
- See docs/DESIGN_autonomous_loop.md Section 5.3""",
    "enhancement,design:autonomous-loop,area:models,area:distillation"
)

create_issue(
    "[Design] Continuous model improvement loop — auto-retrain on high-confidence feedback",
    """## Background
ATSassin collects real feedback every time a user accepts, edits, or rejects a recommendation. Over time, this builds a high-signal dataset of what the model got right and what it got wrong. Currently, this data is collected in SQLite but never automatically triggers a retraining cycle.

Goal: when enough high-confidence, high-quality training pairs accumulate, the system should automatically flag (and optionally execute) a retraining cycle. This closes the loop from 'model made recommendation' > 'user responded' > 'model improved'.

## Goals
- User/role agnostic: trigger thresholds are relative to pair count and quality, not profile content
- Opt-in by default: retraining must be explicitly configured; the system only recommends, never executes without consent
- Incremental: don't retrain from scratch — support fine-tuning from the previous checkpoint

## Acceptance Criteria
- [ ] Define the automatic retraining trigger: N high-confidence pairs (default 100) with average confidence > 0.7
- [ ] Implement a config watcher that checks pair count after each feedback event
- [ ] When threshold is met, surface a CLI/TUI notification: 'N new training pairs available — run `atsassin distill --retrain`'
- [ ] Add `atsassin distill --retrain` that executes the full training pipeline on the accumulated dataset
- [ ] After retraining, run the evaluation harness automatically and report quality delta
- [ ] If the new model passes the quality gate, create or update a symlink/alias to activate it
- [ ] If the new model fails the quality gate, keep the previous model and report the failure
- [ ] Add tests that simulate feedback accumulation and verify the retrain trigger logic

## Related
- Depends on distillation evaluation harness
- Depends on training dataset curation pipeline
- See docs/ROADMAP.md Experimental section""",
    "enhancement,design:autonomous-loop,area:models,area:distillation"
)

create_issue(
    "[Design] Cross-architecture deployment targets — ONNX Runtime, OpenVINO, CoreML, DirectML, WebGPU",
    """## Background
The current distillation pipeline exports conversion scripts for ONNX (via optimum), GGUF (via llama.cpp), and OpenVINO (via Intel model optimizer). This is a solid foundation but leaves gaps:
- No Apple Silicon / CoreML support
- No Windows DirectML support for GPU acceleration
- No WebGPU path for in-browser inference
- No automated verification that converted models produce identical outputs to the source

## Goals
- User/role agnostic: all targets export the same model architecture for the same profile
- Verified: every export format includes a validation step
- Fallback: if a target fails to export, the pipeline continues with the remaining targets

## Acceptance Criteria
- [ ] Add CoreML export script for Apple Silicon (macOS M-series)
- [ ] Add DirectML export script for Windows GPU acceleration
- [ ] Add `atsassin distill validate` that runs a small batch of inputs through each exported model
- [ ] Verify output consistency across formats: for the same input, all formats should produce the same prediction within tolerance
- [ ] Document hardware requirements for each target format in docs/HARDWARE_TARGETS.md
- [ ] Add a manifest entry per export target recording: format, file hash, model size, validation status
- [ ] Add tests that validate the conversion scripts compile/parse correctly

## Related
- Depends on student model training workflow (needs a trained model to export)
- Extends the existing ONNX/GGUF/OpenVINO script generation in src/engine/distillation.rs""",
    "enhancement,design:autonomous-loop,area:models,area:distillation,help wanted"
)

create_issue(
    "[Design] Model registry & versioning for distilled artifacts",
    """## Background
As ATSassin accumulates multiple distilled models (different sizes, quantization formats, training iterations), there is no way to:
- Track which model version produced which evaluation result
- Roll back to a previous model if a new one regresses
- Compare performance across versions
- Share model performance metadata with the community registry

A lightweight model registry in SQLite solves this without adding infrastructure.

## Goals
- User/role agnostic: versioning works identically for any profile
- Local-first: everything in SQLite, no external services
- Serializable: the registry can be exported as JSON for sharing

## Acceptance Criteria
- [ ] Design a `model_registry` SQLite table: id, version, task_type, model_size, quantization, format, file_hash, evaluation_results (JSON), created_at, active (bool)
- [ ] Update the distillation pipeline to register each trained/exported model in the registry
- [ ] Add `atsassin model list` to show registered models and their evaluation results
- [ ] Add `atsassin model activate <id>` to switch the active model for a task type
- [ ] Implement automatic rollback: if a new model fails the quality gate, keep the previous model active
- [ ] Add tests for registry operations (register, list, activate, rollback)
- [ ] Document the model registry schema in docs/ARCHITECTURE.md

## Related
- Depends on distillation evaluation harness (registry stores evaluation results)
- Enables community model sharing (Stage 1-2)
- See docs/ROADMAP.md Experimental section""",
    "enhancement,design:autonomous-loop,area:models,area:distillation"
)

create_issue(
    "[Design] Calibrate distillation against real pipeline outcomes — close the recommendation-to-offer loop",
    """## Background
The ultimate measure of distillation quality is not accuracy on a test set — it is whether the distilled model helps users get better outcomes (more interviews, more offers, higher compensation). 

Currently, the pipeline has no feedback loop connecting 'model version X produced these recommendations' to 'the user applied and got an interview/offer because of recommendation Y'.

## Goals
- User/role agnostic: outcome tracking works for any profile
- Privacy-preserving: outcome data (offers, comp) is stored locally and never shared
- Actionable: the calibration data drives model ranking in the community registry

## Acceptance Criteria
- [ ] Add model_version tracking to pipeline status transitions (which model version produced the evaluate/tailor output)
- [ ] After an outcome is classified (Interviewing, Offered, Rejected), update the model's success metrics in the registry
- [ ] Implement a per-model success dashboard: `atsassin model stats` shows offers/applications per model, avg comp uplift, etc.
- [ ] Integrate with community registry rankings: models with proven outcome improvement rank higher
- [ ] If a model shows statistically significant outcome regression, auto-flag it for rollback
- [ ] Add tests with synthetic outcome data to verify the calibration logic

## Related
- Depends on model registry & versioning
- Depends on pipeline outcome classification (Phase 0)
- Closes the loop described in docs/DESIGN_autonomous_loop.md""",
    "enhancement,design:autonomous-loop,area:models,area:distillation,analytics,user-value"
)

# ============================================================================
# GROUP B: SOURCING ARCHITECTURE
# ============================================================================

create_issue(
    "[Design] Modular source architecture — trait-based pluggable job sources",
    """## Background
The current `scrape_board_at` in `src/pipeline/scraper.rs` is a single large match statement covering ~20 board types. Adding a new board means modifying this function and recompiling the whole CLI. This does not scale.

The design calls for a `JobSource` trait where each source is one file in `src/sources/`:

```rust
pub trait JobSource {
    fn name(&self) -> &'static str;
    async fn fetch(&self, query: &str, limit: usize) -> Result<Vec<JobSummary>>;
}
```

## Goals
- User/role agnostic: sources are independent of profile content
- Minimal: adding a new source is one file + one line in a registry
- Self-contained: each source defines its own rate limiting and error handling

## Acceptance Criteria
- [ ] Define the `JobSource` trait in a new `src/sources/mod.rs`
- [ ] Define `SourceConfig` struct with per-source rate_limit, timeout, user_agent
- [ ] Define `SourceManager` that loads sources from config and runs them concurrently
- [ ] Port existing LinkedIn scraper as the first `JobSource` implementation
- [ ] Port existing Greenhouse/Lever/Ashby scrapers as additional implementations
- [ ] Port existing Seek scraper as an implementation
- [ ] Port existing social aggregators (HN, Reddit, RemoteOK, etc.) as a combined `SocialSource`
- [ ] Add a source registry (either config-driven or auto-discovered via module listing)
- [ ] Implement per-host concurrency capping in SourceManager (not per-source)
- [ ] Add tests for SourceManager (concurrent execution, error isolation, empty results)
- [ ] Document how to add a new source in CONTRIBUTING.md

## Related
- Extends the matching design per-host rate limiting
- Prerequisite for community board-discovery feed
- See docs/AUDIT_DESIGN_GAPS.md Section 1.1""",
    "enhancement,design:autonomous-loop,area:sourcing,help wanted"
)

create_issue(
    "[Feature] Autonomous company ATS detector — replace hand-maintained board list",
    """## Background
Issue #1 established the principle: derive the company-to-board directory instead of maintaining a static list. The `atsassin companies discover` command and `board_discovery.rs` module already exist as a foundation, but the detector only covers Greenhouse.

The full vision: given any company domain, fetch their public careers page, pattern-match the embedded ATS URLs (Greenhouse, Lever, Ashby, Workable, Workday, JazzHR, Breezy, etc.), extract the company slug, and automatically add them to the sweep.

This turns 'add more companies' from a permanent chore into a one-time detector.

## Goals
- User/role agnostic: detection works for any company in any geography
- Deterministic: same domain always produces the same detection result
- Polite: respects robots.txt and rate limits

## Acceptance Criteria
- [ ] Research and document the URL patterns for each major ATS provider (Greenhouse, Lever, Ashby, etc.)
- [ ] Extend `board_discovery.rs` to detect Lever, Ashby, Workable, Workday, JazzHR, Breezy, SmartRecruiters, BambooHR
- [ ] Implement fallback: HTML pattern-matching for ATS embed URLs on careers pages
- [ ] Implement caching of discovery results in SQLite
- [ ] Automatically sweep newly discovered companies during `atsassin scan --boards companies`
- [ ] Add `atsassin companies discover-bulk` from a curated list of company domains
- [ ] Add tests with mock HTML pages for each ATS provider
- [ ] Document the ATS detection patterns in docs/ATS_DETECTION.md

## Related
- Extends Issue #1 (closed) with real implementation
- Related to modular source architecture
- See docs/ROADMAP.md Known issues #1""",
    "enhancement,design:autonomous-loop,area:sourcing,good first issue"
)

# ============================================================================
# GROUP C: PRAGMATIC MATCHING
# ============================================================================

create_issue(
    "[Design] Pragmatic requirement scoring — adjacent, transferable & weighted matching",
    """## Background
Current scoring (both `prerank` and `evaluate`) treats requirements as binary: the profile either has a skill or doesn't. Real-world hiring is more nuanced:
- A Python requirement is perfectly met by a candidate with Python, Django, Flask
- A healthcare requirement is adjacent for a candidate with life sciences experience
- A team management requirement is transferable from program management
- A Google Cloud certification requirement is genuinely absent

The design calls for a pragmatic scoring system that weights requirements by match type:
- Exact match: 1.0 (skill literally in profile)
- Adjacent match: 0.7 (related skill, same domain)
- Transferable: 0.4 (different domain but applicable)
- Missing: 0.0 (genuinely absent)

## Goals
- User/role agnostic: adjacency mapping is universal, not profile- or industry-specific
- Deterministic: same requirements + same profile = same pragmatic score
- Lightweight: runs in under 1ms per job with zero LLM calls — uses a static adjacency map

## Acceptance Criteria
- [ ] Implement `PragmaticScorer` struct in `src/engine/pragmatic_scorer.rs`
- [ ] Define a static adjacency map: Python - Rust, healthcare - life sciences, management - leadership, etc.
- [ ] Implement `score_requirements(profile, requirements) -> PragmaticScore`
- [ ] Integrate into the prerank pipeline as a modifier on the lexical base
- [ ] Unknown requirements default to 'unknown' (never boost or penalize)
- [ ] Add tests covering: exact, adjacent, transferable, missing, and unknown matches
- [ ] Document the scoring formula and adjacency map in docs/MATCHING.md

## Related
- Complements the segment-aware prerank
- See live trial design synthesis""",
    "enhancement,design:autonomous-loop,area:matching"
)

create_issue(
    "[Feature] Job segment classifier — tag roles by industry/focus at scrape time",
    """## Background
During the live trial, roles in pharma and data security were found but the scoring treated them identically — no segment-weighted matching. A job in pharmaceutical contract sales should be scored with pharma-relevant weights, not generic sales weights.

The fix: tag every job with industry/segment tags at scrape time using a cheap keyword classifier (not LLM). Segment-matching adjusts the prerank score.

## Goals
- User/role agnostic: segments are profile-independent, derived from job content
- Cheap: zero LLM calls — pure regex keyword matching
- Composable: a job can belong to multiple segments (pharma + SaaS + APAC)

## Acceptance Criteria
- [ ] Define initial ~20 industry segments with keyword signatures
- [ ] Implement `SegmentClassifier` in `src/engine/segment_classifier.rs`
- [ ] Tag each job at scrape/persist time with its segments
- [ ] Add segment weights to user preferences
- [ ] Integrate segment matching into prerank as a score modifier
- [ ] Surface segment tags in the TUI and CLI output
- [ ] Add tests with synthetic job descriptions for each segment
- [ ] Document the segment taxonomy in CONTRIBUTING.md

## Related
- Complements pragmatic requirement scoring
- See live trial findings""",
    "enhancement,design:autonomous-loop,area:matching,good first issue"
)

create_issue(
    "[Design] Visa, language & experience restriction parser — extract constraints from job text",
    """## Background
During the live trial, roles outside the user's region were surfaced without any visa/work-authorization signal. The tool needs to extract and flag:
- Citizenship/visa requirements
- Language requirements
- Experience floors (10+ years)
- Experience ceilings (entry-level, junior)

Missing or unclear restrictions should be explicitly flagged as 'unknown'.

## Goals
- User/role agnostic: parsers work from posting text alone
- Deterministic: same text produces the same parsed restrictions
- Honest: unparseable restrictions are flagged as unknown, never inferred

## Acceptance Criteria
- [ ] Implement `RestrictionParser` in `src/engine/restriction_parser.rs`
- [ ] Extract visa/work-authorization via regex: work authorization, visa, citizen, sponsorship, right to work
- [ ] Extract language requirements via keyword + ISO 639-1 codes
- [ ] Extract experience floors and ceilings
- [ ] Store parsed restrictions alongside the job in SQLite
- [ ] Surface in TUI and CLI output
- [ ] Null/unparseable restrictions pass the filter with an info flag
- [ ] Add tests for each restriction type
- [ ] Document patterns in docs/RESTRICTION_PARSING.md

## Related
- Complements pragmatic requirement scoring
- See live trial findings""",
    "enhancement,design:autonomous-loop,area:matching,good first issue"
)

create_issue(
    "[Enhancement] Embedding-based proximity matching — cosine similarity over job embeddings",
    """## Background
The codebase already has a `semantic_score` in `src/engine/matcher.rs` that calls Ollama embedding API. However, it returns the embedding vector magnitude instead of proper cosine similarity against a job embedding.

Fix: precompute job embedding at scrape time, store in SQLite, compute cosine similarity against profile embedding.

## Goals
- User/role agnostic: embeddings are deterministic per text
- Local: uses local Ollama, no cloud API calls
- Incremental: store embeddings in existing SQLite jobs table

## Acceptance Criteria
- [ ] Add embedding BLOB column to jobs table
- [ ] Compute and store job embedding at scrape/persist time
- [ ] Compute and cache profile embedding once at startup
- [ ] Replace magnitude-based semantic_score with true cosine similarity
- [ ] Integrate into prerank pipeline as an additional signal
- [ ] Implement HNSW only when job count exceeds 10K
- [ ] Graceful fallback if Ollama is unavailable
- [ ] Add tests with known similar/dissimilar pairs
- [ ] Document in docs/MATCHING.md

## Related
- Fixes existing matcher.rs implementation
- Complements pragmatic scoring and segment classification""",
    "enhancement,design:autonomous-loop,area:matching"
)

# ============================================================================
# GROUP D: SALARY INFERENCE
# ============================================================================

create_issue(
    "[Feature] Market baseline salary dataset — lightweight JSON per role/region/seniority",
    """## Background
The current `extract_max_annual_usd` regex is excellent at parsing explicit salary figures. But most contract roles do not state salary. The design adds a market baseline dataset: lightweight JSON mapping (role, region, seniority) to compensation percentiles.

## Goals
- User/role agnostic: baselines aggregate across all users
- Lightweight: single JSON file, periodically updated
- Honest: all baseline values explicitly labeled as estimated

## Acceptance Criteria
- [ ] Design the market baseline JSON schema
- [ ] Create initial baseline from public sources (BLS, Levels.fyi, Glassdoor)
- [ ] Implement `MarketBaseline::lookup(role, region, seniority) -> Option<SalaryEstimate>`
- [ ] Integrate into salary pipeline: explicit > baseline > unknown
- [ ] Label baseline values as 'Estimated from market data'
- [ ] Add `atsassin market update` to refresh from optional remote URL
- [ ] Allow configurable confidence floor for estimates
- [ ] Add tests for exact match, partial match, no match, confidence labeling

## Related
- Addresses Issue #58
- Complements cross-corpus salary corroboration
- See docs/ROADMAP.md Known issues #3""",
    "enhancement,design:autonomous-loop,area:schema,good first issue"
)

create_issue(
    "[Design] Cross-corpus salary corroboration — average across sources for reliability",
    """## Background
A single job posting may not state salary, but the same role at the same company or across multiple postings often provides enough data points to infer a reliable range. The design adds cross-corpus corroboration: when the same (role, region, seniority) combination appears across 3+ postings with explicit salary data, compute a weighted average.

## Goals
- User/role agnostic: corroboration works from posting data alone
- Automatic: no manual configuration required
- Confident: minimum N sources before a corroborated value is trusted

## Acceptance Criteria
- [ ] Implement `SalaryCorroborator` that queries the jobs table for similar roles
- [ ] Define similarity: same role_title normalized + same region + seniority within 1 level
- [ ] Require minimum 3 corroborating sources before generating an estimate
- [ ] Weight more recent postings higher (exponential decay over time)
- [ ] Surface corroborated estimates alongside the confidence level and source count
- [ ] Integrate into the salary pipeline as a middle tier between explicit salary and market baseline
- [ ] Add tests with synthetic posting data

## Related
- Complements market baseline salary dataset
- Addresses Issue #58""",
    "enhancement,design:autonomous-loop,area:schema"
)

# ============================================================================
# GROUP E: CAREER COACH
# ============================================================================

create_issue(
    "[Feature] Continuous market-watch daemon — one-shot scheduled scanning",
    """## Background
Issue #106 captures the career-coach vision. The first concrete step is a scan-once-daemon that runs on a schedule (cron/Task Scheduler) rather than a resident process. This keeps the design lightweight while proving the concept.

## Goals
- User/role agnostic: scheduling works identically for any profile
- Zero infrastructure: uses OS-native scheduling
- Honest: if no new roles, reports that instead of fabricating

## Acceptance Criteria
- [ ] Document one-shot approach: `atsassin daemon --once` + cron/Task Scheduler
- [ ] Extend daemon to detect last scan time and only surface new roles
- [ ] Add `--notify` flag for notification-compatible output
- [ ] Add sample cron entries and Task Scheduler XML to scripts/
- [ ] Add `atsassin daemon status` command
- [ ] Implement idle detection: skip scanning if user inactive > N days
- [ ] Add tests for one-shot scan logic
- [ ] Document in README.md Daemon mode section

## Related
- First step toward Issue #106
- Depends on modular source architecture""",
    "enhancement,design:career-coach,area:daemon"
)

create_issue(
    "[Design] Preference-challenge insights engine — surface opportunities not yet considered",
    """## Background
A core career-coach function: when market data shows that a small change (relocating, switching industry, picking up a skill) could materially improve income, surface the finding as a question, not a prescription.

Each insight must be grounded in market data, not LLM hallucination.

## Goals
- User/role agnostic: signals from market baselines and profile attributes
- Honest: every insight includes confidence label and data source
- Non-prescriptive: surfaced as optional reading, never as directives

## Acceptance Criteria
- [ ] Implement `InsightEngine` in `src/engine/insight_engine.rs`
- [ ] Relocation signal: compare user location vs. top-N markets
- [ ] Segment-switch signal: compare current industry vs. adjacent comp
- [ ] Skill-gap signal: flag high-demand skills in target roles user lacks
- [ ] Undersell signal: profile seniority higher than target role titles
- [ ] Rank insights by impact (comp delta) and confidence
- [ ] Surface in `atsassin recommend --insights` and TUI
- [ ] Add tests for each signal type with synthetic data
- [ ] Document in docs/INSIGHTS.md

## Related
- Depends on market baseline salary dataset
- Part of Issue #106""",
    "enhancement,design:career-coach,analytics,user-value"
)

# ============================================================================
# GROUP F: INFRASTRUCTURE
# ============================================================================

create_issue(
    "[Cleanup] Consolidate duplicate issues and close orphaned ones",
    """## Background
Over multiple red-team audit passes, several issues were created that overlap with existing issues:
- #62 / #103: Environment variable documentation
- #83 / #104: Configuration backup/rollback
- #57 / #73 / #94: Low-spec hardware validation
- #66 / #87: PII scrubbing for LoRA sharing
- #65 / #86: Daemon is scan-only
- #64 / #85: Lightning AI stub training

Each duplicate set should be consolidated into the surviving issue.

## Acceptance Criteria
- [ ] Identify all duplicate sets
- [ ] For each set, pick the survivor (older issue or more context)
- [ ] Update the survivor with additional context from the duplicate
- [ ] Close the duplicate with a comment linking to the survivor
- [ ] Verify no open PR references a closed duplicate

## Related
- Prerequisite for clean contributor onboarding""",
    "documentation,good first issue,audit"
)

print("\n=== ALL ISSUES CREATED ===")
print("Review any failures above and retry individually if needed.")
