# Architecture Decision Record

Decisions that are **settled**. If you are about to propose something in the "Rejected" section, read the reasoning first — these were evaluated in depth and rejected on their merits, not for lack of ambition. If you have evidence that invalidates a reason given here, open an issue citing the specific reason; that is a welcome contribution.

Each decision states the date, the status, and the reasoning. Superseding a decision requires a new entry, not an edit.

---

## ADR-001 — Job identity is content-addressed, never random

**Date:** 2026-07-29 · **Status:** Accepted · **Supersedes:** the implicit v4-UUID convention

Job primary keys are derived deterministically from the posting's canonical identity:

1. Canonicalise the URL: lowercase host, strip `utm_*`/`gclid`/`ref`/`source` params, drop the fragment, remove trailing slash.
2. `id = sha256(canonical_url)` truncated to 128 bits, hex-encoded.
3. Where the URL is a search page rather than a posting (some social sources), fall back to `sha256(company + "\x00" + title + "\x00" + normalised_location)`.

The `jobs` table gains `UNIQUE(canonical_url)` and writes become `INSERT … ON CONFLICT DO UPDATE`.

**Why:** v4 UUIDs (`cli.rs:758`, `tui.rs:607`, `daemon.rs:114`) are random and unrelated to content, so the same posting scanned twice becomes two rows, the evaluation cache keyed on `job_id` can never hit, and the daemon re-evaluates every job on every tick at full LLM cost forever. This made continuous market-watch unshippable. Every downstream layer — dedup, calibration, allocation — requires stable identity.

**Consequence:** re-scanning becomes idempotent and cheap. This is a prerequisite for everything else in the current architecture.

---

## ADR-002 — Missing data is represented as missing, never as a plausible default

**Date:** 2026-07-29 · **Status:** Accepted

No field may be populated with a substitute value that asserts something the system does not know. Specifically prohibited, all of which existed and have been removed or are scheduled for removal:

| Prohibited | Was at | Correct behaviour |
|---|---|---|
| `posted_at: Some(Utc::now())` when the source gave no date | `scraper.rs:872,951,1003`; `social_scraper.rs` ×7 | `None` |
| Synthesised 0.5 evaluation on LLM parse failure | `scorer.rs:102-111` | Return `Err`; do not persist |
| Hardcoded `location: "Remote"` | `social_scraper.rs` ×8 | `None` or the observed value |
| Search-page URL stored as the posting URL | `scraper.rs:871,950,1002` | Flag as a lead, not a posting |
| Fabricated market data in `roles research` | `cli.rs:880-915` | Derive from evidence or report unavailable |

**Why:** this is the project's fourth design principle, and the violations were not inert. The fabricated `posted_at` fed `landscore`'s recency term (`landscore.rs:71`), which grants +15 to anything dated today versus +5 for an honest `None` — so **the ranking systematically promoted the sources that fabricated dates over the sources that reported them truthfully**. A fabricated default is worse than a null because it is indistinguishable from evidence downstream.

**Consequence:** ranking, calibration, and allocation may only consume fields that are either observed or explicitly absent. Scoring functions must handle `None` as a first-class case rather than treating it as zero.

---

## ADR-003 — Errors propagate; they are not collapsed into empty results

**Date:** 2026-07-29 · **Status:** Accepted

Scraper dispatch must distinguish "this source returned no matches" from "this source failed". `scraper.rs:83-159` applied `.unwrap_or_default()` to all ~20 dispatch arms, collapsing every `Err` into an empty `Vec` before returning `Ok`. The downstream consequence was that `scanner.rs:59`'s "All boards failed" branch became unreachable dead code, and a total network outage was reported to the user as *"No jobs returned… Try a different query"*.

Sources return `Result<Vec<JobSummary>>`; the aggregator collects both successes and failures and reports them separately.

**Why:** honest failure is a stated value, and a misdiagnosis is a failure of honesty. Telling a user to change their search terms when their network is down wastes their time and erodes trust in every other message the tool prints.

---

## ADR-004 — Extraction is a tiered ladder, not a browser

**Date:** 2026-07-29 · **Status:** Accepted

Job data is extracted through four tiers, in order, falling through only on failure: CNAME enumeration → ATS JSON APIs → SSR hydration blobs (`__NEXT_DATA__`) → Schema.org JSON-LD. Headless browsing is a last resort, not a default.

See [design/EVIDENCE_LAYER.md](design/EVIDENCE_LAYER.md).

**Why:** one mechanism resolves four separately-filed workstreams (ATS detection, real salary data, restriction parsing, real posting dates) and eliminates the ~167 MB-per-instance memory cost of headless Chromium, which is incompatible with the 4 GB hardware floor.

**Notably, this replaces the planned maintained salary dataset.** Structured compensation from the employer's own API has perfect provenance, needs no curation, and cannot go stale — strictly better than a periodically-refreshed JSON baseline file that requires perpetual maintenance.

---

## ADR-005 — Conversion rates are per-user posteriors, not displayed benchmarks

**Date:** 2026-07-29 · **Status:** Accepted

Published funnel statistics enter the system as **prior distributions** for a per-user empirical-Bayes model, never as figures shown to the user as guidance. Personal rates are shrunk toward the prior in proportion to observation count, and reported as intervals.

Two constraints are mandatory, not optional:

- **Shrinkage.** A user with 12 applications and 1 callback has almost no signal. The model reports an interval and says plainly when it does not yet know.
- **Controllable/structural decomposition.** Observed outcomes are decomposed into candidate-controllable factors (tailoring depth, submission latency, role fit) and structural factors (name-based screening bias, employment-gap filters, prior self-employment penalties, market tightness). **Only controllable factors may drive recommendations.** Structural factors are attributed explicitly to the user.

See [design/CALIBRATION_LAYER.md](design/CALIBRATION_LAYER.md).

**Why:** a naive personal model fitted to raw outcomes learns "you are a weak candidate" when the real signal is documented market discrimination — correspondence studies put minority-named callbacks at 25–50% of baseline and prior self-employment under 10%. Attributing that to the candidate would be both wrong and harmful. Attributing it correctly turns the most uncomfortable finding in the research into the most humane feature in the product.

---

## ADR-006 — The output is an allocated slate, not a ranked list

**Date:** 2026-07-29 · **Status:** Accepted

The flagship output is a min-cost max-flow solution over the opportunity set — a weekly slate respecting an effort budget, role diversification caps, and posting age decay — not a sorted list.

See [design/ALLOCATION_LAYER.md](design/ALLOCATION_LAYER.md).

**Why:** ranked lists are a commodity. The user's binding constraint is a finite budget of genuinely-tailored applications against a decaying opportunity set with non-linear marginal value across role families. Greedy ranking cannot express expiry, shared budget, or diversification, and is provably suboptimal under all three.

---

## ADR-007 — No real-person PII in the repository or the issue tracker

**Date:** 2026-07-29 · **Status:** Accepted

This is a public MIT repository. No contributor's, maintainer's, or test subject's **name, employers, compensation figures, contact details, or immigration status** may appear in tracked files, issue bodies, issue comments, PR descriptions, or commit messages.

This binds equally to:

- **Test fixtures.** Personas are synthetic. Where a real profile is used for a live trial, it stays local and gitignored; the committed record describes the profile by **shape only** — seniority, function, region, years, employment status. That is all a reader needs to judge whether a finding generalises.
- **Directory and file names.** A fixture directory named after a real person leaks in the path itself, and paths appear in every stack trace, issue reference, and CI log.
- **Trial records and defect reports.** A defect can always be described without the subject's data. If a finding seems to require a real figure, state the *shape* of the error ("several times below the correct value") rather than the value.

**Why it needs to be a rule rather than a habit:** the failure mode is not carelessness about privacy, it is that concrete detail makes a bug report more persuasive. During the 2026-07-29 work, a real compensation figure was written into a public issue precisely because it made the defect vivid. It was removed, but the pull toward specificity is exactly why this is written down.

`.gitignore` enforces the local half (root CV patterns, `/profile.md`, `/apply_kit_*/`, `/assets/examples/`). The rest is review discipline — check it on every PR.

---

# Rejected

## REJ-001 — P2P / DHT distributed crawling (libp2p, Kademlia, S/Kademlia, Merkle-CRDT)

**Status:** Rejected · **Do not re-propose without addressing reason 1.**

1. **The DHT-coordinated rate limiter is a DDoS vector.** The proposed design stores each domain's Theoretical Arrival Time in the DHT and has peers consult it before fetching. The protected resource — the employer's web server — is *outside* the trust boundary. A malicious peer reporting a TAT in the past induces every honest node to fetch simultaneously, turning the user base into a botnet; a TAT far in the future silently suppresses discovery of chosen employers. No honest-majority assumption repairs this, because the victim is not a network participant.
2. **It inverts the privacy architecture.** "Personal data never leaves the machine" is the product's first value. In a P2P index every user becomes a publisher of scraped third-party PII (hiring manager names, emails). The GDPR reasoning offered for the design — Art. 14(5)(b) disproportionate effort, Art. 6(1)(f) legitimate interest — is argued for a single controller running a public archive and does not transfer to a design that makes every end user a controller.
3. **PoW node-ID generation contradicts the 4 GB CPU-only floor**, which is itself still unvalidated.
4. **Bootstrap economics are inverted.** A DHT with three users indexes nothing; value is zero until adoption is large, while complexity is paid on day one.

Consistent with the existing rejections of blockchain coordination and P2P federated learning.

## REJ-002 — LMDB replacing SQLite

**Status:** Rejected

Zero-copy `mmap` reads are irrelevant when the working set is a few thousand rows and the bottleneck is network and LLM latency measured in seconds. SQLite already stores relational pipeline/evaluation/telemetry data with joins that a key-value store would push into application code. A multi-week migration for no user-observable gain.

*Revisit if:* the local corpus exceeds ~10M rows **and** profiling attributes >5% of wall-clock to storage reads.

## REJ-003 — Partitioned Elias-Fano indexes and Block-Max WAND

**Status:** Rejected

Techniques for inverted indexes over hundreds of millions of documents. `prerank.rs` scores a few thousand jobs with smoothed TF-IDF in memory in microseconds and is correct as written. PEF would add hundreds of lines of bit-manipulation to optimise something unmeasurable at this scale.

*Revisit if:* a single user's local corpus exceeds ~1M postings.

## REJ-004 — Arena allocation, generational indices, CSR, epoch-based reclamation

**Status:** Rejected

These matter for large mutating in-memory graphs traversed in hot loops. ATSassin's graph is a few thousand nodes, lives in SQLite, and is rebuilt per solve. The allocation solver runs in milliseconds single-threaded. Introducing `crossbeam-epoch` or CSR would optimise nanoseconds inside a workload bounded by seconds of network I/O.

## REJ-005 — Cuckoo and Bloom filters for URL state

**Status:** Rejected as premature

The deletion argument for Cuckoo over Bloom is correct in principle. But the visited-URL set is thousands of entries — a `HashSet<u64>` costs roughly 100 KB. Both structures solve a problem the product does not have.

*Revisit if:* the visited set exceeds ~10M URLs.

## REJ-006 — Full MinHash + LSH banding

**Status:** Rejected at current scale; replaced by SimHash

Cross-board syndication is real and near-duplicate detection is genuinely needed. But LSH banding tuned at b=9/r=13 targets billions of documents. At a few thousand, a 64-bit SimHash with Hamming-distance bucketing is sufficient and roughly 100 lines. The idea is adopted; the implementation scale is not.

## REJ-007 — Ingratiation and interview behavioural coaching

**Status:** Rejected on scope and ethics

The research shows ingratiation tactics drive perceived Person-Organization fit, which drives offers. Building a feature that coaches users to flatter interviewers is unmeasurable locally (the tool observes nothing inside an interview), sits inside the liability zone the project has explicitly chosen to avoid, and is corrosive to the product's character. Interview-stage drop-off is real (32% of pipeline loss) but is not observable by this tool; modelling it would be fabrication.

## REJ-008 — Acting on structural bias data as advice

**Status:** Rejected permanently

Callback-penalty data for minority names, employment gaps, and prior self-employment enters the calibration model **only as attribution**, never as advice. Any feature suggesting name anglicisation, gap concealment, or similar is out of scope permanently, regardless of measured efficacy.
