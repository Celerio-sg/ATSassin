# ATSassin Red Team Audit — Combined Findings

This document consolidates two adversarial red-team reviews of the ATSassin codebase and roadmap.

## Methodology

- Code-level forensic review of `src/` and `scripts/`.
- Dependency and supply-chain surface analysis.
- Failure-mode and chaos-engineering walkthrough.
- Solution-design / roadmap completeness review.

## Scope

- Rust application code in `src/`
- Python distillation scripts in `scripts/`
- GitHub repository and CI/CD infrastructure
- Documentation in `docs/`

## Severity Legend

- **CRITICAL**: Breaks end-to-end functionality, security, or user trust if shipped.
- **HIGH**: Significant production risk or missing core feature.
- **MEDIUM**: Important hardening or observability gap.
- **LOW**: Nice-to-have improvement.
- **FUTURE**: Adjacent capability, not required for production.

---

## CRITICAL

### CR-1: Distillation conversion scripts are non-functional stubs

**Files:** `src/engine/distillation.rs`

**Finding:** The exported `export_onnx.py`, `quantize_gguf.sh`, and `openvino_export.py` scripts only print placeholder messages. They do not perform ONNX conversion, GGUF quantization, or OpenVINO IR export.

**Permanent fix:** Generate scripts that invoke real tooling (`transformers.onnx` / `optimum`, `llama.cpp` `convert.py` / `convert-hf-to-gguf.py`, and `openvino.convert_model`). Scripts must validate dependencies, report actionable errors, and exit non-zero on failure.

**Related Issue:** #63

**Status:** Fixed in `src/engine/distillation.rs`.

---

### CR-2: Lightning AI integration is stubbed and never trains

**Files:** `src/cli.rs`

**Finding:** When `LlmProvider::Lightning` is selected, `atsassin distill` prints a stub note and does not submit the generated training data to any training endpoint.

**Permanent fix:** Implement a Lightning AI training client that authenticates with `LIGHTNING_API_KEY`, uploads the JSONL training data, and submits a fine-tuning job via the Lightning AI API. The CLI should report job ID, status, and errors.

**Related Issue:** #64

**Status:** Fixed in `src/engine/lightning.rs` and `src/cli.rs`.

---

## HIGH

### HI-1: Daemon is scan-only and not a full orchestrator

**Files:** `src/engine/daemon.rs`

**Finding:** The daemon only scans boards. The design specifies that it should also evaluate/rank jobs, queue high-quality jobs for tailoring, trigger follow-ups, and ingest outcomes from email/IMAP.

**Permanent fix:** Extend `run_tick` with a state-machine workflow: scan → evaluate against profile → rank with `landscore` → queue top matches for tailoring → update pipeline statuses → trigger follow-up reminders → sync IMAP outcomes.

**Related Issue:** #65

**Status:** Fixed in `src/engine/daemon.rs`.

---

### HI-2: PII scrubbing is missing from the LoRA sharing pipeline

**Files:** `src/engine/distillation.rs`, future registry/DHT code

**Finding:** The autonomous LoRA sharing design plans to pool distilled adapters across users, but the export path does not yet scrub PII before any artifact leaves the local machine.

**Permanent fix:** Integrate `PiiScrubber` into every artifact export and registry upload path. Add a gate that blocks export until `contains_pii` returns false, with user-facing logs describing what was redacted.

**Related Issue:** #66

**Status:** Fixed in `src/engine/distillation.rs`.

---

### HI-3: OpenSSL dependency remains via `imap` crate

**Files:** `Cargo.toml`

**Finding:** The codebase uses `rustls` for HTTP but the `imap` crate pulls in `native-tls` and OpenSSL, increasing supply-chain and platform-build risk.

**Permanent fix:** Switch to `imap` with the `rustls` feature, or replace with a rustls-native IMAP implementation, and add `cargo-deny` / `cargo-audit` to CI to prevent regressions.

**Related Issue:** #67

---

### HI-4: Board-health canary is not yet automated

**Files:** `src/pipeline/scanner.rs`, `src/engine/board_health.rs` (future)

**Finding:** Scraper drift is only detected manually. The roadmap calls for a canary that detects zero-result boards and structural changes.

**Permanent fix:** Implement a `board_health` module with a scheduled canary that scrapes a small probe set, compares results to baselines, and opens/issues an alert when drift exceeds a threshold.

**Related Issue:** #68

---

## MEDIUM

### ME-1: No startup validation of required secrets

**Finding:** Missing API keys or IMAP credentials are only discovered at runtime. The CLI should validate that required secrets are present for the selected provider at startup.

**Related Issue:** #69

---

### ME-2: No health-check command

**Finding:** There is no single command that validates LLM, database, network, and IMAP connectivity. Add `atsassin health`.

**Related Issue:** #70

---

### ME-3: User-provided prompt input is not sanitized

**Files:** `src/engine/prompts.rs`, `src/engine/tailor.rs`, `src/engine/scorer.rs`

**Finding:** Profile and job-description text is inserted into prompts without length limits or delimiting checks, risking token overflow and injection-style misuse.

**Permanent fix:** Add length caps, boundary markers, and a small `InputSanitizer` that rejects or truncates suspiciously nested instructions.

**Related Issue:** #71

---

### ME-4: README can drift out of sync with CLI

**Finding:** New CLI commands are not automatically checked against README documentation.

**Permanent fix:** Add a CI step that extracts commands from README and verifies they exist in `atsassin --help` output.

**Related Issue:** #72

---

### ME-5: Low-spec hardware claim is not validated

**Finding:** The documented 4GB CPU-only target has not been validated on real hardware.

**Permanent fix:** Add a CI job or documented manual protocol that runs the full CLI on a 4GB CPU-only machine and asserts acceptable latency/memory.

**Related Issue:** #73

---

## LOW

### LO-1: Circuit breaker parameters are hard-coded

**Files:** `src/engine/llm.rs`

**Finding:** Threshold (5 failures) and cooldown (60s) are constants. Make them configurable per provider.

**Related Issue:** #74

---

### LO-2: No circuit breaker metrics in telemetry

**Finding:** State transitions are not emitted to telemetry, limiting operational visibility.

**Related Issue:** #75

---

### LO-3: SQLite encryption-at-rest is not offered

**Finding:** Local SQLite database is unencrypted. Add an opt-in encryption-at-rest feature for users syncing to cloud storage.

**Related Issue:** #76

---

### LO-4: Database connection uses `Mutex<Connection>`

**Files:** `src/pipeline/tracker.rs`

**Finding:** Read-heavy workloads may contend on the single mutex. Evaluate `RwLock` or a small connection pool.

**Related Issue:** #77

---

### LO-5: Disk space is not checked before large operations

**Finding:** Archival and distillation operations can exhaust disk. Add pre-flight checks.

**Related Issue:** #78

---

### LO-6: `.expect()` calls in HTTP client construction

**Files:** `src/engine/llm.rs`

**Finding:** Some HTTP client construction uses `.expect()` or `.unwrap()`. Replace with proper error propagation.

**Related Issue:** #79

---

### LO-7: Structured error codes are missing

**Finding:** Errors are stringly typed. Add machine-readable error codes for common failures.

**Related Issue:** #80

---

### LO-8: PII scrubber edge cases

**Finding:** International phone formats and unusual address formats may not be caught. Expand regex coverage and tests.

**Related Issue:** #81

---

### LO-9: Environment variables are not documented in one place

**Finding:** Secrets and overrides are scattered across source. Centralize documentation.

**Related Issue:** #82

---

### LO-10: Configuration backup/rollback is missing

**Finding:** Successful config modifications are not backed up. Add a rollback mechanism.

**Related Issue:** #83

---

## New Strategic Work Items

Raised from the latest design discussion:

| ID | Priority | Title | Related Issue |
| --- | -------- | ----- | ------------- |
| ST-1 | HIGH | Crowd-source role, salary and job-board knowledge | #105 |
| ST-2 | HIGH | Continual job-landscape polling and career coaching | #106 |
| ST-3 | HIGH | Comprehensive codebase and roadmap completeness review | #107 |

---

## FUTURE / ADJACENT CAPABILITIES

These are not blockers but represent high-value extensions:

1. Skill gap analysis against scraped JDs
2. Salary negotiation assistant
3. Company culture analysis (Glassdoor, etc.)
4. Interview prep generator
5. Resume A/B testing and callback tracking
6. LinkedIn mutual-connection mapping
7. Application timeline analytics

---

## Combined Summary Table

**Cleanup note (2026-07-29):** Duplicate issues from parallel audit passes have been consolidated. Issues #84-#104 are closed as duplicates of #63-#83. Issues #63-#66, #108 are closed as fixed (code applied in commit 071071f). The canonical open items are listed below — see the GitHub tracker for latest status.

| ID  | Priority | Title | Related Issue | Status |
| --- | -------- | ----- | ------------- | ------ |
| CR-1 | CRITICAL | Distillation conversion scripts are stubs | #63 | ✅ Fixed (commit 071071f) |
| CR-2 | CRITICAL | Lightning AI integration is stubbed | #64 | ✅ Fixed (commit 071071f) |
| HI-1 | HIGH | Daemon is scan-only | #65 | ✅ Fixed (commit 071071f) |
| HI-2 | HIGH | PII scrubbing missing from LoRA sharing | #66 | ✅ Fixed (commit 071071f) |
| HI-3 | HIGH | OpenSSL via `imap` crate | #67 | Open |
| HI-4 | HIGH | Board-health canary not automated | #68 | Open |
| ME-1 | MEDIUM | Startup secret validation | #69 | Open |
| ME-2 | MEDIUM | Health check command | #70 | Open |
| ME-3 | MEDIUM | Prompt input sanitization | #71 | Open |
| ME-4 | MEDIUM | README sync check in CI | #72 | Open |
| ME-5 | MEDIUM | Validate low-spec hardware claim | #73 | Open |
| LO-1 | LOW | Configurable circuit breaker | #74 | Open |
| LO-2 | LOW | Circuit breaker telemetry | #75 | Open |
| LO-3 | LOW | SQLite encryption-at-rest | #76 | Open |
| LO-4 | LOW | DB connection pool contention | #77 | Open |
| LO-5 | LOW | Disk space monitoring | #78 | Open |
| LO-6 | LOW | Remove `.expect()` in HTTP clients | #79 | Open |
| LO-7 | LOW | Structured error codes | #80 | Open |
| LO-8 | P0 | PII scrubber edge cases | #81 closed | ✅ Fixed with deterministic regional and false-positive fixtures |
| LO-9 | LOW | Document env vars | #82 | Open |
| LO-10 | LOW | Config backup/rollback | #83 | Open |
| ST-1 | HIGH | Crowd-source role/salary/board knowledge | #105 | Open |
| ST-2 | HIGH | Continual landscape polling / career coach | #106 | Open |
| ST-3 | HIGH | Comprehensive design completeness audit | #107 | Open |

---

## Deep Design Issues (raised during precision-engineering design pass)

### Design Group A: Distillation Pipeline (depth expansion)

| ID | Priority | Title | Issue |
| -- | -------- | ----- | ----- |
| DP-1 | HIGH | Training dataset curation pipeline (dedup, thresholds, balancing) | #109 |
| DP-2 | HIGH | Automated student model training workflow (end-to-end local FT) | #110 |
| DP-3 | HIGH | Distillation evaluation harness & benchmark (automated quality gate) | #111 |
| DP-4 | MEDIUM | Continuous model improvement loop (auto-retrain on feedback) | #112 |
| DP-5 | MEDIUM | Cross-architecture deployment targets (CoreML, DirectML, WebGPU) | #113 |
| DP-6 | MEDIUM | Model registry & versioning for distilled artifacts | #114 |
| DP-7 | MEDIUM | Calibrate distillation against real pipeline outcomes | #115 |

### Design Group B: Sourcing Architecture

| ID | Priority | Title | Issue |
| -- | -------- | ----- | ----- |
| SA-1 | HIGH | Modular source architecture (trait-based pluggable sources) | #130 |
| SA-2 | HIGH | Autonomous company ATS detector (replace static board list) | #116 |

### Design Group C: Pragmatic Matching

| ID | Priority | Title | Issue |
| -- | -------- | ----- | ----- |
| PM-1 | HIGH | Pragmatic requirement scoring (adjacent/transferable/weighted) | #132 |
| PM-2 | MEDIUM | Job segment classifier (tag roles by industry at scrape time) | #133 |
| PM-3 | MEDIUM | Visa/language/experience restriction parser | #117 |
| PM-4 | MEDIUM | Embedding-based proximity matching (cosine similarity) | #118 |

### Design Group D: Salary Inference

| ID | Priority | Title | Issue |
| -- | -------- | ----- | ----- |
| SI-1 | HIGH | Market baseline salary dataset (lightweight JSON) | #119 |
| SI-2 | MEDIUM | Cross-corpus salary corroboration (average across sources) | #120 |

### Design Group E: Career Coach

| ID | Priority | Title | Issue |
| -- | -------- | ----- | ----- |
| CC-1 | MEDIUM | Continuous market-watch daemon (one-shot scheduled) | #121 |
| CC-2 | MEDIUM | Preference-challenge insights engine | #122 |

### Design Group F: Housekeeping

| ID | Priority | Title | Issue |
| -- | -------- | ----- | ----- |
| HK-1 | LOW | Consolidate duplicate issues and close orphaned ones | #140 |
