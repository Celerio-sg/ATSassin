# Changelog

All notable changes to this project are documented here. Format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). This project is pre-1.0 and versioned informally — see git history/tags for exact commit-level detail.

## [Unreleased]

### Fixed
- Job identity is now deterministic and content-addressed: posting URLs are canonicalised before hashing, search-page leads use an explicit normalised tuple identity, file imports use a content hash, and SQLite schema v3 safely re-keys legacy rows while preserving and retargeting evaluation, pipeline, application, and feedback history. Canonical upserts report whether a job is new, so repeat CLI scans and daemon ticks no longer duplicate recommendations or repeat evaluation work (#142).
- SQLite schema changes now use ordered forward-only `PRAGMA user_version` migrations in one transaction, with filename-agnostic ignored backups whose eligibility is decided under the migration write lock and which preserve source permissions, explicit outcomes, legacy adoption, repeat-open idempotency, and newer-schema refusal shared by pipeline and feedback entry points; every operation holds a database transaction across its version guard and SQL, failed migrations leave user history untouched, and daemon tests no longer open the user's default database (#181).
- Every LLM route now crosses the shared egress gate: trusted instructions and labelled untrusted data are boundary-separated, high-confidence nested instructions and marker collisions fail closed, prompt size is bounded by the adjusted model context, and the HTTP client accepts only an opaque validated request (#71).
- The identity-bearing Scenario 1 UAT profile and current-tree references were replaced with a wholly synthetic senior APAC GTM persona of equivalent parser and workflow test shape; historical reports retain their observations with anonymisation annotations (#146).
- Deterministic PII coverage now includes representative SG, UK, India, EU, and US phone/address shapes, Unicode profile identity, strong or labelled national IDs/passports/DOBs, and standalone social handles, with false-positive controls; this is explicitly not universal NER (#81).
- Lightning AI training uploads now require an opaque, fail-closed payload validated from the exact in-memory JSONL bytes sent over HTTP; candidate identity values are redacted, rejected data cannot reach the network, and the unsafe `*.flagged.jsonl` copy has been removed (#143).
- Install instructions and `Cargo.toml` package metadata pointed to the wrong GitHub repository (`srbhr/ATSassin` instead of `Celerio-sg/ATSassin`) — the one-liner installer and `git clone` command in the README would not have worked for a new user.
- LinkedIn export location/email/phone were correctly parsed but silently lost on the `profile.md` round-trip.
- `scan` results defaulted to US-only postings regardless of query wording or `--location`.
- Tailored resumes dropped most work history, education, and contact info instead of the full profile.
- Several CI/tooling scripts (`check_cli_docs.sh`, `board_health.yml`) validated locally but failed on real execution — see commit history for the specifics.

### Added
- `atsassin recommend` — composite "likely to land quickly" job ranking.
- `atsassin pipeline show` — view a job's description alongside the resume/cover letter actually submitted for it.
- Multi-region board support (`--location`, `seek:sg`/`seek:nz`/`seek:hk`), APAC company directory entries, non-English remote/hybrid phrase matching.
- Board-health canary (scheduled + `workflow_dispatch`) that opens a real tracking issue if a board silently starts returning zero results.
- `docs/DESIGN_autonomous_loop.md` — design for the next phase of autonomous, closed-loop operation.

## [0.1.0] — 2026-07-25
Initial public release.
