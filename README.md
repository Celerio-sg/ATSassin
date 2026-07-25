# ATSassin

>The silent killer of bad job matches.

ATSassin is a lightweight, privacy-first, local-first job search assassin. It parses your CV, LinkedIn export, and portfolio to dynamically infer suitable roles, deep-research markets, score jobs with ATS accuracy, tailor resumes and cover letters, and track your pipeline — all on your machine with Ollama or your favorite local LLM.

## Job-Securing Playbook

See [PLAYBOOK.md](PLAYBOOK.md) for the integrated APAC-focused playbook covering recruiter outreach, referrals, contract platforms, personal branding, and interview prep.

## Why ATSassin?

- **Dynamic role inference**: one input (resume/LinkedIn/portfolio) → 5-10 inferred role archetypes with market demand and compensation bands
- **Local-first**: 100% local inference via Ollama; optional cloud swap (Groq, OpenRouter, OpenAI, Anthropic)
- **Hardware-adaptive**: runs on a 4GB laptop CPU, 8GB integrated graphics, or 32GB workstation
- **Single binary**: `cargo build --release` and you're done — no Docker, no Python, no Node
- **Privacy**: SQLite on disk, zero telemetry, zero SaaS, zero third-party account required

## Quick Start

```bash
# One-liner
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/srbhr/ATSassin/main/scripts/install.sh | bash

# Or manual
git clone https://github.com/srbhr/ATSassin.git && cd ATSassin
cargo build --release
cp .env.example .env
./target/release/atsassin profile init --resume profile.md
./target/release/atsassin roles infer
```

## Commands

| Command | Description |
|---------|-------------|
| `atsassin profile init --resume <file>` | Parse resume, LinkedIn export, or portfolio URL |
| `atsassin roles infer` | Dynamically infer suitable role archetypes |
| `atsassin scan --role <query>` | Scan job boards for matching roles. Default boards: `linkedin`, `seek`, `companies` (a curated, concurrently-swept directory of ~35 real companies' public Greenhouse job-board APIs - zero LLM tokens, no external agent, see `src/pipeline/company_directory.rs`), `social`. Also available via `--boards`: `indeed`/`glassdoor` (best-effort, usually blocked) and `greenhouse:<company>` / `lever:<company>` / `ashby:<company>` for a single ATS-hosted company |
| `atsassin scan --role <query> --prefs-only` | Same, but only show/save jobs matching your saved preferences |
| `atsassin preferences show` / `set` | View or set comp floor, employment type, and work-mode preferences used to filter scans and the TUI job table |
| `atsassin evaluate --job-id <id>` / `--file <jd.txt>` | ATS-score a job against your profile |
| `atsassin tailor --job-id <id>` / `--file <jd.txt>` | Generate tailored resume + cover letter |
| `atsassin pipeline list` | Track applications in SQLite |
| `atsassin pipeline update --job-id <id> --status <status> [--notes] [--contact] [--follow-up YYYY-MM-DD]` | Update tracking fields for a pipeline entry |
| `atsassin tui` | Terminal dashboard — infer roles, scan, evaluate, and tailor without leaving it (`e`/`t`/`s`/`r`, `p` to toggle preference filter, `x` to sort by local relevance) |

## Hardware Modes

| Mode | Model | Context | CPU OK | Min RAM |
|------|-------|---------|--------|---------|
| `light` | `qwen3.5:4b` | 4096 | ✅ | 4 GB |
| `balanced` | `qwen3.5:9b` | 8192 | ✅ | 8 GB |
| `full` | `qwen3.5:9b:q6` | 32768 | ❌ | 16 GB |

> `--preset` only changes which local Ollama model tier is used. On a hosted cloud provider (Groq, Kimi, GLM, Lightning, OpenRouter, OpenAI, Anthropic) it changes only request timeout/retries/scrape-limits — the model itself is fixed by your provider config, not by preset.

## Setup

1. Install [Ollama](https://ollama.com).
2. Pull models for your tier: `bash scripts/setup_ollama.sh`
3. Configure `.env` with your Ollama endpoint.
4. Run `atsassin profile init --resume resume.md`.

## License

MIT
