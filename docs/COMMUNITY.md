# Community & communication guide

This document explains how the ATSassin community communicates and where each type of conversation belongs.

## Channels

| Channel | Purpose | Examples |
|---|---|---|
| **GitHub Issues** | Actionable bugs, features, and tasks | "Scan fails when board returns 0 jobs", "Add support for X" |
| **GitHub Discussions** | Questions, ideas, show-and-tell, community polls | "How do I set up Ollama on Windows?", "What if we added Y?" |
| **Discord / real-time chat** | Casual chat, quick questions, real-time collaboration | "Can someone pair with me on this PR?", "Is this a known issue?" |
| **docs/AWESOME.md** | Curated ecosystem list | Plugins, integrations, related tools |
| **Security reports** | Private vulnerability reports | See [`SECURITY.md`](../SECURITY.md) |

## Avoiding duplication

- **If you have a concrete bug or feature request, open an issue.** Discussions are great for brainstorming, but issues are where work gets tracked.
- **If an idea is still forming, start a Discussion.** Once it matures into a concrete proposal, a maintainer can convert it into an issue (or you can open one).
- **Do not use Discussions for security reports.** Use the private reporting path in [`SECURITY.md`](../SECURITY.md).
- **Wiki is intentionally not used.** Documentation lives in the repo (`README.md`, `docs/`, and inline docs) so it goes through the same review process as code.
- **GitHub Projects are used as a visual mirror of the roadmap.** The source of truth is still [`docs/ROADMAP.md`](ROADMAP.md) and [`docs/CRITICAL_CHAIN_PLAN.md`](CRITICAL_CHAIN_PLAN.md). The board is linked to the repo so contributors can see issue progress at a glance. To keep it from duplicating effort, board columns match issue states (To do / In progress / In review / Done) and cards are linked issues, not hand-maintained tasks.

### Setting up the project board (maintainers)

1. Go to the repository on GitHub.
2. Click **Projects** → **Link a project**.
3. Create a new **public** project named **"ATSassin Autonomous LoRA Sharing"**. Public visibility lets contributors see the board without extra permissions.
4. Add columns: **To do**, **In progress**, **In review**, **Done**.
5. Add the critical-chain issues from [`docs/CRITICAL_CHAIN_PLAN.md`](CRITICAL_CHAIN_PLAN.md) (e.g. #45–#55).
6. Add a note in the project description: *"Visual mirror of docs/ROADMAP.md and docs/CRITICAL_CHAIN_PLAN.md."*
7. Pin the project on the repository so it appears on the repo homepage.

## Real-time chat

- **Discord:** [https://discord.gg/PwwnemcAy](https://discord.gg/PwwnemcAy) — casual chat, quick questions, and pairing

If you are a maintainer updating this link in the future:

1. Choose a platform that aligns with the project's privacy values (Discord, Matrix, etc.).
2. Create a server/room.
3. Create a `#code-of-conduct` or `#rules` channel and link to [`CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md).
4. Create an `#introductions` channel.
5. Update the invite/link in `README.md`, `CONTRIBUTING.md`, and this document.
6. Open a PR with the updated links.

## Code of conduct

All community spaces follow the [Contributor Covenant](../CODE_OF_CONDUCT.md). Be kind, be patient, and assume good intent.
