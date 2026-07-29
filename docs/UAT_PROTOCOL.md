# ATSassin User Acceptance Testing (UAT) Protocol v1.0

**Document Version**: 1.0  
**Date**: July 24, 2026  
**Purpose**: Fully defined, repeatable, objective testing framework to validate ATSassin across diverse user scenarios, hardware profiles, and API configurations.

---

## 1. Objectives

- Verify role inference quality and relevance across diverse candidate profiles.
- Evaluate tailoring, matching, and output quality.
- Measure performance on low-spec (4GB RAM, CPU-only) vs high-end hardware and local vs hosted APIs.
- Quantify estimated job-landing probability and earning potential.
- Identify any remaining gaps before declaring v1.0 ready.

## 2. Scope

- All Tier 1 scenarios (5 personas) and optionally Tier 2 scenarios.
- Both `--preset lightweight` (CPU-only, minimal RAM) and `--preset full` + hosted APIs.
- All core CLI commands exercised per scenario.

## 3. Prerequisites

- Latest ATSassin release binary (`target/release/atsassin.exe`).
- Ollama running with required models (`qwen3.5:4b` for lightweight, `qwen3.5:9b` for balanced/full).
- Valid API keys configured in `.env` (Kimi, Lightning AI, Groq if testing upscale).
- Test data folder structure: `tests/uat/<scenario_name>/` containing `profile.md`.

## 4. Test Scenarios (Tier 1)

| # | Persona | Background | Target Role | Key Challenge |
|---|---------|-----------|-------------|---------------|
| 1 | **Maya Kestrel (synthetic)** | 25+ yrs tech sales/GTM/PM leadership (regional VP, GM, interim director, founder) | Remote contract/interim GTM leadership in APAC | Senior pivot to contract/interim |
| 2 | **Returning Housewife / Nomadic Freelancer** | 38yo, 8yr career gap, PA/TA/ESL background | Flexible remote VA/tutoring (20-30h/wk) funding travel in SEA/Europe | Career gap detection, non-traditional roles |
| 3 | **Worldschooling Parent** | 42yo former marketing coordinator, full-time worldschooling 2 kids | Part-time remote content/VA/curriculum design | Unconventional work, flexible hours |
| 4 | **Recent Graduate in Tokyo** | 24yo Japanese business graduate, 3mo internship, basic English | Entry-level tech sales/BD/international coordination | Thin resume, language barrier |
| 5 | **Retrenched "Salary Man"** | 54yo, 28yr IT sales rep at single Japanese firm, retrenched | Stable contract/part-time, pivot support | Age, single-employer bias, retrenchment |

## 5. Step-by-Step Test Procedure (Per Scenario)

1. **Profile Init**: `atsassin profile init --resume <profile.md>`
2. **Profile Show**: `atsassin profile show`
3. **Role Inference**: `atsassin roles infer -n 8`
4. **Job Scan**: `atsassin scan --role "<top_inferred_role>" --limit 15`
5. **Market Intelligence**: `atsassin market stats` and `atsassin market rates --role "<top_role>"`
6. **Feedback System**: `atsassin feedback stats` and `atsassin feedback recent`
7. **Distillation**: `atsassin distill --roles 3 --output distillation_test`
8. **Pipeline**: `atsassin pipeline list`

### LLM-Dependent Steps (require Ollama or hosted API)

9. **Evaluate**: `atsassin evaluate --file <jd.txt>` (fixture is UTF-8; `tests/fixtures/sample_jd_utf16.txt` is kept as a UTF-16LE regression fixture and must not crash either). This persists the job to the database and prints its id.
10. **Tailor**: `atsassin tailor --file <jd.txt>` (first time for a job) or `atsassin tailor --job-id <id>` (using the id printed by step 9 or a `scan` result).

## 6. Scoring Rubric (1.0 – 5.0 Scale)

| Dimension | Weight | 1.0 | 3.0 | 5.0 |
|-----------|--------|-----|-----|-----|
| **Role Inference** | 25% | Irrelevant/generic roles | Some relevant, some misses | Highly relevant, realistic, diverse |
| **Tailoring Quality** | 40% | Generic slop | Acceptable but unpolished | Personalized, accurate, ATS-optimized |
| **Overall Usability** | 20% | Crashes, errors, slow | Works but rough edges | Smooth, ultra-fast, low-spec ready |
| **Assessment Accuracy** | 15% | Way off market rates | Ballpark | Precise, data-driven |

## 7. Success Criteria

- **Average weighted score across Tier 1 ≥ 4.2 / 5.0**
- **≥ 70% of scenarios produce ready-to-submit applications** (Steps 9-10, when LLM available)
- **Low-spec mode (`--preset lightweight`) fully usable on 4GB RAM CPU-only** without crashes or hangs
- **Startup time < 100ms** (measured via `--help` invocation)
- **Binary size < 15 MB** (release build with LTO + strip)

## 8. Reporting

Final UAT Report must include:
- Per-scenario scores with weighted average
- Strengths and weaknesses per scenario
- Probability estimates and earning potential per persona
- Mode comparison (lightweight vs balanced vs full)
- Summary metrics against success criteria
- Any remaining gaps or future enhancements

## 9. Re-Run Instructions

```bash
# Run all Tier 1 scenarios
cd tests/uat
for /d %d in (*) do (
    copy "%d\profile.md" ..\..\profile.md
    ..\..\target\release\atsassin.exe profile show
    ..\..\target\release\atsassin.exe roles infer -n 8
    ..\..\target\release\atsassin.exe market stats
)
```
