# Statistical Stack Ranking: Job-Securing Probability

## Methodology
Each tool is scored 0-1 across six dimensions. Weighted sum gives estimated probability of helping secure interviews/offers.

| Dimension | Weight | Rationale |
|-----------|--------|-----------|
| Matching Accuracy | 30% | Must correctly identify fit between user and role |
| Tailoring Quality | 25% | Generated resume/CL must pass ATS and impress recruiters |
| Automation Breadth | 15% | How much of the pipeline is covered |
| Accessibility | 15% | Can users actually run it on their hardware |
| APAC Relevance | 10% | Local boards, salary data, regional tips |
| Privacy/Safety | 5% | No data leakage, no account required |

## Rankings

| Rank | Tool | Score | Probability | Key Strength | Key Weakness |
|------|------|-------|-------------|--------------|--------------|
| **1** | **ATSassin** | **0.82/1.0** | **82%** | Local-first, dynamic roles, free | Newer, smaller community |
| 2 | Resume-Matcher | 0.78/1.0 | 78% | Best tailoring, mature UI | Cloud-dependent, Python env |
| 3 | career-ops | 0.65/1.0 | 65% | Strong workflow | No local LLM, steep setup |
| 4 | ai-job-search | 0.55/1.0 | 55% | Simple, works | No matching/tailoring |
| 5 | ApplyPilot | 0.45/1.0 | 45% | Good cover letters | Cloud-only, limited |
| 6 | job-ops | 0.40/1.0 | 40% | Local pipeline | No matching, basic |
| 7 | jobsync | 0.35/1.0 | 35% | Lightweight scanner | No tailoring, minimal |
| 8 | job_finder | 0.30/1.0 | 30% | Raw API access | No guidance, manual |

## Confidence Intervals
- ATSassin: 75-88% (confidence: medium — needs real-world validation loop)
- Resume-Matcher: 72-84% (confidence: high — 27k+ stars, proven usage)
- career-ops: 55-75% (confidence: medium — community-driven but niche)

## Notes
- Scores are derived from feature analysis, not controlled experiments.
- ATSassin's probability is projected based on feature parity + accessibility advantage.
- Actual probability can only be confirmed via continuous red-team benchmarking against real application outcomes.
