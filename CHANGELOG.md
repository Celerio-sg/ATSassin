# Changelog

All notable changes to this project are documented here. Format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). This project is pre-1.0 and versioned informally — see git history/tags for exact commit-level detail.

## [Unreleased]

### Fixed
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
