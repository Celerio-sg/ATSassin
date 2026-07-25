# Executive Summary: Why ATSassin Wins

## The Problem
Job seekers waste hundreds of hours on applications that never get seen. Existing tools are either:
- **Cloud-only SaaS** that harvest resume data and require paid subscriptions
- **Python scripts** that need complex environments and GPUs
- **Generic LLM chat** that lacks job-search-specific reasoning

## Top 3 Competitors
1. **Resume-Matcher** (srbhr): Best UI/UX, strong tailoring, but Python-only and cloud-dependent. GitHub 27k+ stars.
2. **career-ops** (santifer): Best workflow automation, but no local LLM support. GitHub community-driven.
3. **ai-job-search** (MadsLorentzen): Simple and effective, but limited to job discovery — no matching or tailoring.

## Why ATSassin Wins
| Criterion | ATSassin | Resume-Matcher | career-ops |
|-----------|----------|----------------|------------|
| **Accessibility** | Single 8MB binary, no runtime | Python env + Node.js | Python env |
| **Local-first** | ✅ Ollama/SQLite, zero telemetry | ❌ Cloud LLM preferred | ❌ Cloud-dependent |
| **Lightweight** | Runs on 4GB RAM, CPU-only | Needs GPU for speed | Needs cloud API |
| **Privacy** | Zero egress by default | Sends data to APIs | Sends data to AWS |
| **Free forever** | ✅ Ollama + Groq free tier | ❌ Paid API costs | ❌ AWS costs |
| **Role inference** | ✅ Dynamic CV parsing → 5-10 roles | ❌ Single manual role | ❌ None |
| **APAC focus** | ✅ SEEK, LinkedIn, Glassdoor APAC | ❌ Global only | ❌ US-centric |

## Mission Alignment
ATSassin is built for people who need work. It is:
- **Truly free**: local inference with optional free cloud fallbacks
- **Accessible**: runs on old laptops with 4GB RAM
- **Private**: no account, no analytics, no data leaves the machine unless the user chooses a cloud provider
- **Open**: MIT licensed, single Rust binary

## Bottom Line
On accessibility and privacy, ATSassin is uncontested. On feature depth, it is closing the gap with Resume-Matcher's best patterns while maintaining its lightweight, local-first architecture.
