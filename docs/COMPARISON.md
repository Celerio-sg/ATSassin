# Competitor Comparison: Champion-Challenger Analysis

## Scoring Rubric (0-1 per category)
- **Feature Completeness**: matching, scoring, tailoring, cover letter, pipeline, role inference, deep research
- **Accessibility**: hardware requirements, install complexity, binary vs script
- **Privacy**: local-first, telemetry, data egress
- **Lightweight**: memory footprint, CPU fallback, binary/runtime size
- **LLM Flexibility**: provider switching, local support, fallback chains
- **APAC Market Fit**: board coverage, regional strategies, salary data

## Comparison Table

| Repo | Language | LLM Support | Matching Algo | Scoring Rubric | Tailoring | Pipeline | Role Inference | Privacy | Lightweight | ATSassin Equiv |
|------|----------|-------------|---------------|----------------|-----------|----------|----------------|---------|-------------|----------------|
| **ATSassin** | Rust | Ollama, Groq, Kimi, OpenRouter, OpenAI, Anthropic, GLM, Lightning | Keyword + Section + Semantic (Ollama) | 6-dimension | Resume + Cover Letter | SQLite CRUD + Ghost detection | Dynamic CV → 5-10 roles | Local-first, zero telemetry | 8MB binary, 4GB RAM, CPU fallback | — |
| **Resume-Matcher** | Python | Claude, ChatGPT, DeepSeek, Kimi, GLM, local | Sentence-transformers similarity | Yes (internal) | Resume + Cover Letter + Interview prep | None | Single manual role | Cloud-dependent | Python env, GPU preferred | Tailoring quality, interview prep |
| **career-ops** | Python | LLM optional | None | None | Basic co-pilot | GitHub-based pipeline | None | AWS/SaaS | Docker + AWS | Workflow automation, community |
| **ai-job-search** | Python/Node | OpenAI | URL-based matching | None | None | CSV export | None | Cloud-only | Medium | Simple job discovery |
| **ApplyPilot** | TypeScript | OpenAI | Keyword | None | Cover letter only | None | None | Cloud-only | Medium | Cover letter templates |
| **job-ops** | Python | OpenAI | Keyword | None | None | Local JSON | None | API keys | Medium | Pipeline JSON tracking |
| **jobsync** | Python | OpenAI | Semantic | None | None | Local JSON | None | API keys | Medium | Minimalist scanner |
| **job_finder** | Python | None | Regex/API | None | None | None | None | Open | Low | Raw API access |

## Champion Best Approaches by Category
- **Matching**: Resume-Matcher's sentence-transformer hybrid → ATSassin adopted (semantic + keyword)
- **Scoring**: Resume-Matcher's 3-subscore weighted ATS breakdown (keyword 0.55, skills 0.25, sections 0.20) → ATSassin adopted with 6-dimension rubric
- **Tailoring**: Resume-Matcher's diff-based path system with allowed/blocked whitelisting → ATSassin adopting
- **Pipeline**: career-ops atomic TSV tracker + canonical states → ATSassin upgraded to SQLite
- **Role Inference**: ATSassin unique — dynamic CV parsing to infer 5-10 roles
- **Ghost Detection**: career-ops liveness checking + repost clustering → ATSassin adopting
- **Scanner**: career-ops zero-token API scanner (Greenhouse/Ashby/Lever) → ATSassin adopting to fix HTML scraping

## Code-Level Patterns Extracted

### From Resume-Matcher
1. **ATS scoring weights** (`apps/backend/app/services/ats.py:18-22`): keyword_match 0.55, skills_coverage 0.25, section_completeness 0.20
2. **Whole-word keyword matching** (`ats.py:51-61`): negative lookbehind/lookahead regex
3. **Date restoration** (`parser.py:40-116`): patch LLM-parsed year-only dates from raw markdown
4. **Diff-based tailoring** (`improver.py:226-410`): path-based changes with allowed/blocked whitelisting
5. **Prompt injection sanitization** (`improver.py:28-56`): regex-based input sanitization
6. **Skill target verification** (`improver.py:754-836`): verify proposed skills against JD + resume before applying

### From career-ops
1. **Repost detection** (`detect-reposts.mjs`): fuzzy title matching + union-find clustering within 90-day window
2. **Zero-token scanner** (`scan.mjs`): direct ATS API calls (Greenhouse/Ashby/Lever), no HTML scraping
3. **Liveness checking** (`liveness-core.mjs`): expired posting detection via footer/Apply-button analysis
4. **Atomic tracker writes** (`tracker.mjs`): file locking, canonical status validation, merge-then-commit

### Adoption Status in ATSassin
| Pattern | Status |
|---------|--------|
| ATS scoring weights | ✅ Implemented in `src/engine/scorer.rs` |
| Whole-word keyword matching | ✅ Implemented in `src/engine/matcher.rs` |
| Date restoration | ⏭️ Planned |
| Diff-based tailoring | ⏭️ Planned |
| Prompt injection sanitization | ⏭️ Planned |
| Skill target verification | ⏭️ Planned |
| Repost detection | ⏭️ Planned |
| Zero-token scanner | ⏭️ Planned |
| Liveness checking | ⏭️ Planned |
| Atomic tracker writes | ✅ SQLite provides ACID guarantees |
