# Layer 1 — Evidence: the tiered extraction ladder

**Status:** Design accepted, unbuilt · **ADR:** [ADR-004](../DECISIONS.md#adr-004--extraction-is-a-tiered-ladder-not-a-browser)
**Depends on:** Step 0 (foundation repair) · **Blocks:** Layer 2, Layer 3

## Purpose

Supply **honest structured facts** — real posting dates, real compensation, real work-mode and restriction fields — to every layer above. Today those fields are either absent, regex-guessed from prose, or fabricated.

This layer is not a performance optimisation. It is the evidence base. Layer 2 cannot fit a submission-latency model without a real `datePosted`, and Layer 3 cannot decay opportunity value without one.

## Why one mechanism closes four workstreams

| Previously filed as | Delivered by |
|---|---|
| #116 — autonomous company ATS detector | Tier 1 + Tier 2 |
| #58 / #119 — replace LLM compensation estimation with real data | Tier 2 `includeCompensation`, Tier 4 `baseSalary` |
| #117 — visa / language / experience restriction parser | Tier 4 structured fields |
| Fabricated `posted_at` (ADR-002) | Tier 2 / Tier 4 authoritative `datePosted` |

The salary point is the one worth dwelling on, because it **replaces** planned work rather than accelerating it. Issue #119 proposed building and maintaining a static JSON of role × region × seniority compensation baselines. That artifact needs perpetual curation, has no per-posting provenance, and is stale the day it ships. Tier 2/4 gives compensation **from the employer, for that exact posting, with perfect provenance and zero maintenance**. The maintained dataset survives only as a *prior* for Layer 2 (see [CALIBRATION_LAYER.md](CALIBRATION_LAYER.md)), never as a user-facing per-job number.

## The ladder

Tiers are attempted in order per target. A tier that fails falls through to the next. The tier that succeeded is **recorded on the job row** — this is both a provenance record and the drift signal that replaces the current board-health canary probe set.

### Tier 1 — CNAME enumeration (discovery, not extraction)

Resolve `careers.{domain}`, `jobs.{domain}`, `boards.{domain}` over DNS-over-HTTPS. If the CNAME target matches a known ATS host, the company's ATS and slug are identified without fetching a page.

```
boards.greenhouse.io    → Greenhouse, slug from the CNAME target or a follow-up probe
jobs.lever.co           → Lever
jobs.ashbyhq.com        → Ashby
*.myworkdayjobs.com     → Workday
```

Persist results to the existing `company_boards` table (`tracker.rs:859`), which already has the right shape (`company, ats_type, slug, source_url, discovered_at`). This turns the hand-maintained 44-entry `company_directory.rs` list into a seed set rather than the source of truth.

**Note:** DNS-over-HTTPS is a network call to a resolver, not to the employer. It carries no scraping-politeness burden and no ToS exposure.

### Tier 2 — ATS JSON APIs (primary extraction path)

Public, documented, intended-for-programmatic-consumption endpoints. `scraper.rs` already implements Greenhouse, Lever and Ashby clients; this tier generalises them behind the source trait and adds the compensation parameter that is currently not requested.

| ATS | Endpoint | Fields gained |
|---|---|---|
| Greenhouse | `GET /v1/boards/{token}/jobs?content=true` | title, location, content, `updated_at` |
| Lever | `GET /v0/postings/{slug}?mode=json` | postings, `commitment` (contract/FTE), team, salary range, `createdAt` |
| Ashby | `GET /posting-api/job-board/{name}?includeCompensation=true` | **compensation tiers**, `isRemote`, `employmentType`, `publishedAt` |
| Workday | `POST /wday/cxs/{tenant}/{site}/jobs` | title, location, `postedOn` |

`employmentType` from Lever `commitment` and Ashby is what makes an employment-type preference a **fact rather than a keyword guess**. Today employment-type detection is a substring scan over title+description (`landscore.rs:26-33`), which is English-only and therefore silently degrades outside anglophone postings.

> ### ⚠️ These four platforms serve a minority of the world's labour market
>
> Greenhouse, Lever, Ashby and Workday are concentrated in US and Western-European technology employment. A user in Japan, India, China, Indonesia, Brazil, Nigeria or most of the world will match **almost nothing** at this tier, and would experience the ladder as broken rather than as thorough.
>
> Two consequences that must shape the build order:
>
> 1. **Tier 4 (JSON-LD) is the universal path, not the fallback.** Schema.org `JobPosting` is driven by search-engine visibility incentives that apply in every market and language. For non-Western users it is the *primary* tier, which is why it is specced as the highest-leverage single tier and should ship first among the extraction tiers.
> 2. **Regional ATS are a first-class extension point, not a nice-to-have.** The tier-2 table is a starting set, and its Western skew is an artifact of where the existing code began — not a judgment about which markets matter. Adding a regional ATS must be one new file implementing `JobSource`, with no changes elsewhere. Contributors who work in under-served markets are the highest-value contributors this layer can attract, and the issue tracker should say so.
>
> A tier-coverage metric per region belongs in the drift telemetry below: if users in a region are consistently resolving at tier 4 or falling through entirely, that is a sourcing gap to be filled, not noise.

### Tier 3 — SSR hydration blobs

For Next.js/Nuxt career pages that serve an empty mount point, extract the serialised state directly from the raw HTTP text stream:

```
<script id="__NEXT_DATA__" type="application/json"> … </script>
```

Parse, then walk to `props.pageProps` and locate the posting array. **Do not instantiate a headless browser** — a Chromium instance costs ~167 MB, which is incompatible with the 4 GB floor and cannot be run concurrently across sources.

Implementation is a bounded scan of the response body for the opening tag, then a brace-matched slice to the closing tag, then `serde_json`. Cap the scanned body at a configurable size (suggest 4 MB) and fall through on miss.

### Tier 4 — Schema.org JSON-LD (the generic tail)

`<script type="application/ld+json">` blocks with `"@type": "JobPosting"`. Widely deployed because it drives search-engine job listings, so coverage on the long tail of company career pages is good.

Fields consumed, and what each unblocks:

| JSON-LD field | Maps to | Unblocks |
|---|---|---|
| `datePosted` | `Job.posted_at` | Layer 2 submission latency; Layer 3 age decay |
| `validThrough` | new `Job.expires_at` | Layer 3 expiry constraint |
| `baseSalary` | structured compensation | #58 / #119 |
| `employmentType` | contract vs FTE | preference filtering as fact |
| `jobLocationType: TELECOMMUTE` | `Job.remote` | replaces hardcoded `"Remote"` (ADR-002) |
| `applicantLocationRequirements` | work-authorisation constraint | #117 |
| `occupationalExperienceRequirements` | experience floor | #117 |
| `directApply` | native posting vs aggregator repost | dedup signal; slate quality |
| `hiringOrganization` | canonical company name | dedup key stability |

**`directApply` deserves attention.** It distinguishes a first-party posting from an aggregator repost, which is a cleaner syndication signal than text similarity and should be preferred over SimHash where present.

## Interface

Extends the `JobSource` trait already designed in #130. The trait was specified to wrap the existing ~20 board arms; it should instead express the ladder.

```rust
pub enum Tier { Cname, AtsApi, SsrHydration, JsonLd, Fallback }

pub struct Extracted {
    pub summary: JobSummary,
    /// Which tier produced this. Recorded on the job row for provenance
    /// and drift detection - never inferred after the fact.
    pub tier: Tier,
}

#[async_trait]
pub trait JobSource: Send + Sync {
    fn name(&self) -> &str;
    /// Per-host politeness is enforced by the SourceManager, not here.
    async fn fetch(&self, q: &Query, prefs: &Preferences) -> Result<Vec<Extracted>>;
}
```

Per ADR-003 this returns `Result`, and `SourceManager` reports successes and failures **separately** so a network outage is never rendered as "no matches".

## Drift detection

The current board-health canary (#68, `board_health.yml`) greps for `"Found N jobs"` across five boards and files an issue on zero. With a tiered ladder there is a strictly better signal: **tier-fallthrough rate**.

A source that normally resolves at Tier 2 and starts resolving at Tier 4 has not failed — it has degraded, and it will silently lose its compensation and `employmentType` fields while still returning jobs. The current canary cannot see this at all. Track per-source tier distribution over time; alert on a distribution shift, not just on zero.

## Compliance boundary

Unchanged from current practice and worth restating because this layer expands reach:

- Only endpoints that are public and intended for programmatic consumption. No authenticated portals, no login-walled data.
- Per-host concurrency caps and rate limits enforced centrally in `SourceManager` (`scraper.rs:1032` already has per-host semaphores for the company sweep; generalise it).
- **Rate limiting is enforced locally.** It is never delegated to a shared or peer-supplied source — see [REJ-001](../DECISIONS.md#rej-001--p2p--dht-distributed-crawling-libp2p-kademlia-skademlia-merkle-crdt).

## Acceptance criteria

1. Tier recorded on every extracted job; queryable distribution per source.
2. `posted_at` is populated from an authoritative source or is `None`. **Zero occurrences of `Utc::now()` as a posting date** anywhere in the extraction path.
3. Compensation, `employmentType`, and remote status populated from structured fields where the tier supplies them; `None` otherwise, never guessed.
4. A source failure is distinguishable from an empty result at the `SourceManager` boundary and in CLI output.
5. No headless browser on any tier-1-to-4 path.
6. Greenhouse/Lever/Ashby extraction reaches parity with today's output, plus compensation and employment type.
