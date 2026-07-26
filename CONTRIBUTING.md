# Contributing to ATSassin

Thanks for considering a contribution. This project's whole point is to make good job-search tooling free and accessible to anyone, so outside contributions are genuinely wanted, not just tolerated.

## Before you start

- **Read [`docs/ROADMAP.md`](docs/ROADMAP.md)** for known issues, and browse [open issues](https://github.com/Celerio-sg/ATSassin/issues) — everything real and open is tracked there, labeled by phase/area.
- **Look for `good first issue`** if this is your first contribution here. These are scoped to be reviewable in one sitting and don't require deep familiarity with the codebase.
- **Check for a `blocked` label** before starting — it means the issue depends on another one landing first; the issue body says which.
- If you want to work on something not yet filed as an issue, open one first and describe what you're planning before writing code — this avoids duplicate work and lets a maintainer flag anything that conflicts with the project's direction (see [`docs/DESIGN_autonomous_loop.md`](docs/DESIGN_autonomous_loop.md) §0 for the core design principles new features are expected to respect: lightweight by default, no new hard dependencies without a strong reason, opt-in for anything with real-world side effects).

## Development setup

```bash
git clone https://github.com/Celerio-sg/ATSassin.git && cd ATSassin
cargo build --release
cp .env.example .env
```

No Docker, no Python, no Node required to build or test the core project. `rust-toolchain.toml` pins the exact toolchain CI uses — running `cargo build` will fetch it automatically via `rustup`.

## Before opening a PR

Run the same checks CI runs, locally, first:

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo test --lib --test integration --test cli
```

All four must pass clean. `cargo clippy`'s `-D warnings` is strict on purpose — a real regression class (dead code hidden behind a misplaced `#[allow]`) shipped past local review once this session because it wasn't run before committing; running it locally is the whole prevention.

If you're touching anything in `.github/workflows/`, be aware YAML validity isn't enough — a scheduled workflow shipped this session with correct YAML but a real runtime failure (missing `permissions: issues: write`) that only showed up when actually triggered on GitHub's infrastructure. If you can, trigger it via `workflow_dispatch` and check the real run before opening the PR.

## Code conventions

- **No comments explaining *what* code does** — names should make that obvious. Comments are for *why*: a non-obvious constraint, a workaround for a specific bug, a decision that would look wrong without context.
- **Never fabricate data or results.** This project's single biggest historical defect class was scan output that looked real but wasn't. If a data source can't be verified, degrade to an honest empty/error state — never invent a plausible-looking placeholder. See `docs/ROADMAP.md`'s ground-truth note for the standard this is held to.
- **Prefer real end-to-end verification over trusting unit tests in isolation.** Several real bugs this session passed their own unit tests but broke on the first real end-to-end run (a parser that worked in isolation but lost data on the actual file round-trip; a CI check whose regex matched nothing). If you're touching a parser, a CLI command, or a CI script, run it for real against real input before considering it done.
- Match existing patterns in the file you're editing rather than introducing a new style.

## Reporting bugs / requesting features

Open a GitHub issue. Include what you ran, what you expected, what actually happened. If it's a scraper/board issue, include the exact command and, if possible, the raw response (redact anything personal).

## Security issues

Do not open a public issue for a security vulnerability — see [`SECURITY.md`](SECURITY.md).

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
