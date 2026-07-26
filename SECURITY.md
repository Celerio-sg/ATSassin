# Security Policy

## Reporting a vulnerability

**Do not open a public GitHub issue for a security vulnerability.**

Report it privately via [GitHub's private vulnerability reporting](https://github.com/Celerio-sg/ATSassin/security/advisories/new) for this repository. This opens a draft security advisory only maintainers can see until it's ready to be disclosed.

If that's not available to you, open a regular issue with the words "Security: " in the title and no exploit details in the body, and a maintainer will follow up privately to get the details through a safer channel.

## What counts as a security issue here

ATSassin is local-first and stores real personal data on the user's own machine: resumes, LinkedIn exports, scraped job descriptions, generated cover letters, and (once outcome ingestion ships) mailbox access. Treat as a security issue anything that could:

- Exfiltrate a user's profile/PII/credentials to a third party without their knowledge or consent.
- Read, write, or delete data outside the user's own local database/config without authorization.
- Execute arbitrary code from an untrusted source (a malicious job posting, a malicious LinkedIn export, a malicious API response).
- Bypass the "a human submits every application" boundary (see `docs/DESIGN_autonomous_loop.md` §5.5) - any path that could cause an automated submission without explicit human action is a security-severity bug, not just a functional one.
- Store a credential (API key, future IMAP credential) somewhere other than its intended secure location (`.env`, OS keychain) - e.g. logging it, committing it, or writing it to a world-readable path.

## Supported versions

This project is pre-1.0 and moves fast; only the latest commit on `main` is supported. There are no LTS branches.

## Response

This is a small open-source project maintained on a best-effort basis - there's no SLA, but security reports get priority triage over feature work.
