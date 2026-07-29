# Contributing to ATSassin

Thanks for taking the time to contribute. This project exists to make good job-search tooling **free and accessible to everyone**, and that only works if the community owns it. Whether you are fixing a typo, improving docs, or shipping a new feature, you are welcome here.

## Table of contents

- [What we are building](#what-we-are-building)
- [Before you start](#before-you-start)
- [Development setup](#development-setup)
- [Picking your first issue](#picking-your-first-issue)
- [Getting help](#getting-help)
- [Opening a pull request](#opening-a-pull-request)
- [Code conventions](#code-conventions)
- [Reporting bugs](#reporting-bugs)
- [Requesting features](#requesting-features)
- [Security issues](#security-issues)
- [Code of conduct](#code-of-conduct)
- [Contributor recognition](#contributor-recognition)

---

## What we are building

ATSassin is a **local-first, privacy-first, job-search assistant** that runs as a single Rust binary. It helps people discover, evaluate, and tailor applications without selling their data or requiring expensive subscriptions.

The current focus is the four-step critical chain: repair the foundation, then build the **evidence**, **calibration** and **allocation** layers that turn a ranked list into an allocated slate. See [`docs/INFLECTION_ARCHITECTURE.md`](docs/INFLECTION_ARCHITECTURE.md) for why.

A smaller, experimental **LoRA-sharing** track also exists (Stages 0–1 only: local adapter generation and a read-only HTTP registry). Its later stages were rejected — see below.

### Current documents

| Document | What it is |
|---|---|
| [`docs/INFLECTION_ARCHITECTURE.md`](docs/INFLECTION_ARCHITECTURE.md) | Why the architecture is what it is |
| [`docs/DECISIONS.md`](docs/DECISIONS.md) | ADR-001…009 and **REJ-001…009** — read the rejections before proposing |
| [`docs/ROADMAP.md`](docs/ROADMAP.md#the-critical-chain) | The build order |
| [`docs/TEST_STRATEGY.md`](docs/TEST_STRATEGY.md) | How your PR is verified |
| [`docs/design/`](docs/design/) | Per-layer implementation specs |

### ⚠️ Historical documents — plans, not current direction

These predate the 2026-07-29 architecture reset. They are kept for context and **must not be used to pick up work** — several plan things that are now rejected:

| Document | Why it is stale |
|---|---|
| `docs/CRITICAL_CHAIN_PLAN.md` | Sequences the LoRA chain **including Stage 3 DHT/P2P (#49), which is rejected** ([REJ-001](docs/DECISIONS.md)) |
| `docs/AUDIT_DESIGN_GAPS.md` | Its issue numbers are stale; crowd-sourcing (#105) is rejected |
| `docs/CATEGORY_LEADERSHIP_ROADMAP.md` | Gap numbering predates the current tracker |
| `docs/DESIGN_autonomous_loop.md` | Phase structure superseded by the critical chain |
| `docs/REDTEAM_AUDIT_COMBINED.md`, `docs/FIX_PLAN_*.md`, `docs/UAT_REPORT_*.md` | Point-in-time reports; their `#N` are internal finding numbers, not GitHub issues |

**If a historical document disagrees with `DECISIONS.md` or the tracker, they win.** When in doubt, start at [#156](https://github.com/Celerio-sg/ATSassin/issues/156).

---

## Before you start

### The four documents that govern this codebase

Read these before writing code. Where anything else in the repo disagrees with them, they win.

| Document | What it settles |
|---|---|
| [docs/INFLECTION_ARCHITECTURE.md](docs/INFLECTION_ARCHITECTURE.md) | Why the product moves from *ranking* to *allocating* |
| [docs/DECISIONS.md](docs/DECISIONS.md) | Settled decisions (ADR-001…008) and **rejected** proposals (REJ-001…008) |
| [docs/ROADMAP.md](docs/ROADMAP.md#the-critical-chain) | The build order — Step 0 blocks everything |
| [docs/TEST_STRATEGY.md](docs/TEST_STRATEGY.md) | The five test tiers your PR is reviewed against |

Per-layer specs are in [docs/design/](docs/design/). The tracking epic is [#156](https://github.com/Celerio-sg/ATSassin/issues/156) — start there.

**Before proposing P2P/DHT, LMDB, Elias-Fano, or arena/CSR work, read the Rejected section of `DECISIONS.md`.** Those were evaluated in depth and rejected on their merits, with reasoning. If you have evidence that invalidates a stated reason, open an issue citing that reason — that is a genuinely welcome contribution, and better than the proposal being re-raised from scratch.

### Five rules that get PRs rejected if broken

These are not style preferences. Each has a live defect behind it.

1. **No fabricated data** ([ADR-002](docs/DECISIONS.md)). A missing value is `None`, never a plausible substitute. A fabricated `posted_at` once made the ranking systematically prefer the sources that fabricate dates over those that report them truthfully — a fabricated default is worse than a null because it is indistinguishable from evidence downstream.
2. **Errors propagate** ([ADR-003](docs/DECISIONS.md)). No `.unwrap_or_default()` on a fallible source call. Collapsing errors into empty results once made a network outage read as "no jobs found, try a different query".
3. **No real-person PII** ([ADR-007](docs/DECISIONS.md)) in code, fixtures, docs, issues, comments, or commit messages — including in file and directory names. Test personas are synthetic; trial records describe a profile by *shape* only.
4. **No constant that varies by circumstance** ([ADR-008](docs/DECISIONS.md)). Effort budgets, seniority bands, diversification caps: derived from the user's own data, or asked. A number fitted to one profile and hardcoded is a defect, not a default.
5. **Rates carry intervals** ([ADR-005](docs/DECISIONS.md)). Any user-facing probability reports a posterior interval and a `prior_dominated` flag. Never a bare point estimate.

### The standing review question

> **Would this behave sensibly for a user unlike the person who wrote it?**

ATSassin claims to serve everyone — any seniority, industry, market, language, career stage. The realistic threat to that is not carelessness; it is that whoever is testing supplies the concrete detail that makes a design feel well-grounded, and their circumstances get written in as universals. It has already happened once in this repo, and [#158](https://github.com/Celerio-sg/ATSassin/issues/158) tracks the cleanup.

If your answer to the question above needs a caveat, the caveat is the missing parameter. **Failure to serve a profile shape is a tracked gap in the tool, never a narrowing of who the tool is for.**

### Then

1. **Check the [open issues](https://github.com/Celerio-sg/ATSassin/issues).** Almost everything real and open is tracked there. Milestones map to the four build steps.
2. **Look for the `good first issue` label** if this is your first contribution. These are scoped to be reviewable in one sitting and do not require deep familiarity with the codebase.
3. **Check for the `blocked` label** before starting — it means the issue depends on another one landing first.
4. **Open an issue first** if you want to work on something not yet filed. This avoids duplicate work and lets a maintainer flag anything that conflicts with the project's direction.

---

## Development setup

```bash
git clone https://github.com/Celerio-sg/ATSassin.git && cd ATSassin
cargo build --release
cp .env.example .env
```

No Docker, no Python, no Node required to build or test the core project. `rust-toolchain.toml` pins the exact toolchain CI uses — running `cargo build` will fetch it automatically via `rustup`.

If you hit a build issue, check:

- You have a recent stable Rust toolchain via `rustup`.
- You ran `cargo build --release` at least once to fetch dependencies.
- The issue is not already covered in [open issues](https://github.com/Celerio-sg/ATSassin/issues).

---

## Picking your first issue

We try to keep the issue tracker honest. A good issue tells you:

- What the problem or feature is
- Why it matters
- Acceptance criteria (what "done" looks like)
- Whether it is blocked by another issue

A great place to start is one of these labels:

- [`good first issue`](https://github.com/Celerio-sg/ATSassin/contribute) — small, self-contained, mentor-friendly
- [`help wanted`](https://github.com/Celerio-sg/ATSassin/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22) — larger tasks that need community help
- [`documentation`](https://github.com/Celerio-sg/ATSassin/issues?q=is%3Aissue+is%3Aopen+label%3Adocumentation) — docs improvements, examples, or clarity fixes

If an issue interests you but is unclear, **ask before coding**. A short question in the issue saves everyone time.

---

## Active parallel tracks

These workstreams can be developed in parallel — they are not blocked by each other, although some depend on shared building blocks (e.g. PII scrubbing gates any shared data). See `docs/AUDIT_DESIGN_GAPS.md` §Parallel-track notes and `docs/CRITICAL_CHAIN_PLAN.md` for dependencies and sequencing.

The four **milestones** are the unit to work against — they mirror the critical chain and each issue is assigned to one.

| Track | Milestone / label | Entry point | Blocked? |
|---|---|---|---|
| **Foundation repair** | `step-0-foundation` | [#156](https://github.com/Celerio-sg/ATSassin/issues/156) → Step 0 | **No — start here** |
| **Evidence layer** | `layer-1-evidence` | [`docs/design/EVIDENCE_LAYER.md`](docs/design/EVIDENCE_LAYER.md) | Yes — needs #142, #144, #145 |
| **Calibration layer** | `layer-2-calibration` | [`docs/design/CALIBRATION_LAYER.md`](docs/design/CALIBRATION_LAYER.md) | Yes — needs Layer 1 |
| **Allocation layer** | `layer-3-allocation` | [`docs/design/ALLOCATION_LAYER.md`](docs/design/ALLOCATION_LAYER.md) | Yes — needs Layer 2 |
| **Distillation** | `area:distillation` | #109–#114 | No — independent of the chain |
| **LoRA sharing (Stages 0–1 only)** | `design:autonomous-loop` | #46, #47 | No — but below the chain in priority |
| **Reliability / hygiene** | `area:reliability` and friends | #69–#83 | No |

**Two tracks that appeared in earlier versions of this table are gone:** `area:crowdsource` (#105 rejected — [REJ-001](docs/DECISIONS.md)) and `area:exposure` (never implemented; `src/engine/ai_exposure.rs` does not exist). Do not start work on either.

Everything in **Step 0 is unblocked and can start today.** Layer work is genuinely blocked, not just deprioritised — random job identity and fabricated posting dates make every downstream measurement untrustworthy, so building on top of them produces work that has to be redone.

### Becoming an area lead

If you make sustained, high-quality contributions to one track, a maintainer may add you as the CODEOWNER for that area. There is no formal ladder — consistent, helpful work and good judgment are enough.

Typical path to becoming an area lead:

1. Open or comment on issues in the track.
2. Land 2–3 meaningful PRs in the area with minimal review cycles.
3. Review other contributors' PRs in that area and provide useful feedback.
4. Open a discussion or mention your interest in the relevant issue; a maintainer will update `CODEOWNERS`.

Area leads are not expected to be available 24/7. They are simply the first reviewer for new PRs in their area and help keep the issue backlog triaged.

### Labels we use

| Label | Meaning |
|---|---|
| `good first issue` | Small, self-contained, mentor-friendly |
| `help wanted` | Larger tasks needing community help |
| `documentation` | Docs, examples, or clarity fixes |
| `bug` | Something is broken |
| `enhancement` | New feature or improvement |
| `design` | Needs architectural review first |
| `area:pii` | Privacy / PII scrubbing |
| `area:coach` | Career coaching / continuous polling |
| `area:crowdsource` | Crowd-sourcing / community registry |
| `area:exposure` | AI exposure / automation risk |
| `design:autonomous-loop` | LoRA sharing / autonomous loop |
| `blocked` | Depends on another issue |

When you open an issue or PR, please pick the relevant `area:*` label so the right maintainer sees it. If you do not have permission to add labels, a maintainer will triage it for you.

---

## Getting help

- **First time here?** Check out issue [#53 — Welcome, first-time contributors!](https://github.com/Celerio-sg/ATSassin/issues/53)
- **Not sure which channel to use?** See [docs/COMMUNITY.md](docs/COMMUNITY.md)
- **Casual questions / ideas:** [GitHub Discussions](https://github.com/Celerio-sg/ATSassin/discussions)
- **Real-time chat:** [Discord](https://discord.gg/PwwnemcAy) — casual chat, quick questions, and pairing
- **Bug reports / feature requests:** [Open an issue](https://github.com/Celerio-sg/ATSassin/issues/new/choose)
- **Real-time chat:** check the repository README or pinned issues for any Discord / community links
- **Mentorship:** if you are a first-time contributor and get stuck, mention it in the issue or PR. Maintainers try to pair-program or leave detailed review guidance when bandwidth allows.

---

## Opening a pull request

Run the same checks CI runs, locally, before opening a PR:

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo test --lib --test integration --test cli
```

All four must pass clean. `cargo clippy`'s `-D warnings` is strict on purpose — a real regression class (dead code hidden behind a misplaced `#[allow]`) shipped past local review once this session because it wasn't run before committing; running it locally is the whole prevention.

If you are touching anything in `.github/workflows/`, be aware YAML validity is not enough — a scheduled workflow shipped this session with correct YAML but a real runtime failure (missing `permissions: issues: write`) that only showed up when actually triggered on GitHub's infrastructure. If you can, trigger it via `workflow_dispatch` and check the real run before opening the PR.

A good PR:

- Links the issue it closes (e.g., "Closes #123")
- Explains the *why*, not just the *what*
- Includes verification steps ("I ran X and saw Y")
- Keeps the change as small as possible while still being useful

---

## Community and ecosystem

- [docs/COMMUNITY.md](docs/COMMUNITY.md) — how we communicate, where each conversation belongs, and how to avoid duplication
- [docs/AWESOME.md](docs/AWESOME.md) — plugins, integrations, and resources built by the community
- [GitHub Discussions](https://github.com/Celerio-sg/ATSassin/discussions) — questions, ideas, and anything that is not a bug or feature request
- **Real-time chat** — [Discord](https://discord.gg/PwwnemcAy) — casual chat, quick questions, and pairing
- **Submit to awesome lists** — tracked in issue [#55](https://github.com/Celerio-sg/ATSassin/issues/55)

## Code conventions

- **No comments explaining *what* code does** — names should make that obvious. Comments are for *why*: a non-obvious constraint, a workaround for a specific bug, a decision that would look wrong without context.
- **Never fabricate data or results.** This project's single biggest historical defect class was scan output that looked real but wasn't. If a data source can't be verified, degrade to an honest empty/error state — never invent a plausible-looking placeholder. See `docs/ROADMAP.md`'s ground-truth note for the standard this is held to.
- **Prefer real end-to-end verification over trusting unit tests in isolation.** Several real bugs this session passed their own unit tests but broke on the first real end-to-end run (a parser that worked in isolation but lost data on the actual file round-trip; a CI check whose regex matched nothing). If you are touching a parser, a CLI command, or a CI script, run it for real against real input before considering it done.
- Match existing patterns in the file you are editing rather than introducing a new style.

---

## Reporting bugs

Open a [bug report](https://github.com/Celerio-sg/ATSassin/issues/new?template=bug_report.yml). Include:

- What you ran
- What you expected
- What actually happened
- Version / commit hash
- Relevant logs (redact anything personal)

If it is a scraper or board issue, the exact command and raw response (anonymized) are the fastest path to a fix.

## Requesting features

Open a [feature request](https://github.com/Celerio-sg/ATSassin/issues/new?template=feature_request.yml). Before filing, check `docs/ROADMAP.md` and open issues — it might already be tracked.

---

## Security issues

Do not open a public issue for a security vulnerability — see [`SECURITY.md`](SECURITY.md).

---

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).

---

## Contributor recognition

We believe in giving credit. When your PR merges, your GitHub handle will appear in the release notes and in the project's contributor list. If you would prefer not to be named, let the maintainer know.

If you make a significant or sustained contribution, you may be invited to help triage issues or review PRs. There is no formal ladder — just consistent, helpful work and good judgment.
