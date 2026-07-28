#!/usr/bin/env bash
# Helper to create the strategic remaining-work issues in GitHub.
# Usage: bash scripts/create_remaining_issues.sh [REPO]
# Default repo: Celerio-sg/ATSassin
set -euo pipefail

REPO="${1:-${REPO:-Celerio-sg/ATSassin}}"

echo "Creating issues in ${REPO}..."

issue_exists() {
    local title="$1"
    local count
    count=$(gh issue list --repo "$REPO" --state open --search "$title" --json number --jq 'length')
    [[ "$count" -gt 0 ]]
}

create_issue() {
    local title="$1"
    local body_file="$2"
    shift 2
    if issue_exists "$title"; then
        echo "Issue already exists: $title"
        return 0
    fi
    gh issue create --repo "$REPO" --title "$title" --body-file "$body_file" "$@"
}

# Issue 1: Crowd sourcing
cat > /tmp/issue_crowd.md <<'EOF'
## Background
ATSassin should automatically crawl for new job boards and social posts. Rather than each instance discovering the same posts independently, the community should pool its knowledge. The same pooling should apply to salaries, reviews of posts to avoid, etc.

## Goals
- Avoid duplicated discovery work across instances.
- Build a shared, privacy-preserving knowledge base of roles, boards and compensation signals.
- Respect PII and consent: only non-identifying, verified data should be shared.

## Acceptance Criteria
- [ ] Design a protocol for anonymously submitting role/board/salary/review signals.
- [ ] Implement a submission path that scrubs PII before any data leaves the local machine.
- [ ] Create a read-only community feed/registry that instances can query.
- [ ] Add reputation/scoring so high-quality contributors are weighted.
- [ ] Document how users opt in or out of the knowledge pool.

## Related
- #45 Autonomous community LoRA sharing, provenance & volunteer compute cooperative
- #46 Stage 0 Local LoRA generation foundation
- #49 Stage 3 DHT/P2P LoRA adapter distribution

## Priority
HIGH
EOF

create_issue "[Feature] Crowd-source role, salary and job-board knowledge across users" /tmp/issue_crowd.md --label "enhancement" --label "community" --label "privacy" --label "area:scraper"

# Issue 2: Continual polling
cat > /tmp/issue_polling.md <<'EOF'
## Background
ATSassin should continuously poll the job landscape to make sure the user is achieving the best income for their preferences. It should offer insights that help users verify their preferences and understand whether relocating, changing role focus, or switching industries would drastically improve income and prospects.

## Goals
- Keep users on the app even when they are not actively job hunting.
- Act as a continuous career coach (opposite of a headhunter: the user's continuous role hunter/optimiser).
- Surface evidence-based recommendations (e.g. relocation, skill gap, salary upside) without being intrusive.

## Acceptance Criteria
- [ ] Schedule regular landscape scans based on user preferences.
- [ ] Track market signals (salary, demand, remote policy) over time.
- [ ] Generate personalised insight cards (e.g. "Relocating to X could increase your estimated income by Y%").
- [ ] Integrate with preferences so insights refine or challenge stated goals.
- [ ] Add a command/dashboard to review historical landscape trends.

## Related
- #45 Autonomous community LoRA sharing, provenance & volunteer compute cooperative
- #63-83 Red-team audit findings

## Priority
HIGH
EOF

create_issue "[Feature] Continual job-landscape polling and career coaching insights" /tmp/issue_polling.md --label "enhancement" --label "analytics" --label "user-value" --label "area:scraper"

# Issue 3: Design completeness audit
cat > /tmp/issue_audit.md <<'EOF'
## Background
We need a ruthless, first-principles audit of the codebase and roadmap to identify any open gaps that must be plugged for completeness, consistency and thought leadership. The goal is to keep users one step ahead of AI automation replacing roles and help them optimise income and enjoyment with minimal effort — a goal admirable enough to attract contributors.

## Scope
- Review every module in src/ for dead ends, missing error handling, and unfinished workflows.
- Review docs/ROADMAP.md and docs/DESIGN_autonomous_loop.md against the code.
- Identify contradictions between documented phases and implemented phases.
- Flag any TODO/FIXME/placeholder that is not tracked by an existing issue.

## Deliverables
- [ ] A markdown audit report in docs/.
- [ ] A list of new or updated GitHub issues for each gap.
- [ ] A prioritised recommendation of permanent fixes vs workarounds.

## Acceptance Criteria
- [ ] All src/ modules inspected.
- [ ] Roadmap and design docs aligned with code.
- [ ] No untracked TODOs remain.

## Priority
HIGH
EOF

create_issue "[Audit] Comprehensive codebase and roadmap completeness review" /tmp/issue_audit.md --label "documentation" --label "good first issue" --label "audit"

# Cleanup
rm -f /tmp/issue_crowd.md /tmp/issue_polling.md /tmp/issue_audit.md

echo "Done."
