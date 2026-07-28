## What does this change?

<!-- One or two sentences. Link the issue it addresses, e.g. "Closes #19". -->

## Why?

<!-- The "why" belongs here, not as inline code comments — see CONTRIBUTING.md's code-comment convention. -->

## Verification

<!-- How did you confirm this actually works? "It compiles" is not verification — several real bugs this
     project shipped past unit tests but broke on the first real end-to-end run. Prefer showing real
     output over describing what you expect to happen. -->

- [ ] `cargo fmt -- --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo build --release` passes
- [ ] `cargo test --lib --test integration --test cli` passes
- [ ] Ran the actual command/feature end-to-end (not just unit tests) and confirmed the real output — paste it below if practical (redact anything personal)
- [ ] If this touches `.github/workflows/`: triggered it via `workflow_dispatch` (or otherwise) and checked the real run, not just YAML validity

## Guardrails (check all that apply)

- [ ] **Privacy gate:** This PR does not export, pool, or share user data. If it does, `src/engine/pii_scrubber.rs` (or a successor) is applied and tested.
- [ ] **Opt-in / autonomy gate:** If this adds background polling, autonomous tasks, or anything with real-world side effects, it is strictly opt-in and respects the `HardwareProfile` tiers.
- [ ] **No fabricated data:** Any output that looks like real market/salary/job data is either real, clearly labeled as an estimate, or degraded to an honest empty state.

## Anything reviewers should know?

<!-- Tradeoffs, things you're unsure about, follow-up work you intentionally left out of scope. -->
