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

Published funnel statistics enter the system as **prior distributions**, never as figures shown to the user as guidance. Personal rates are shrunk toward the prior, and reported as intervals. **The shrinkage is *inversely* proportional to observation count** — the prior's share is `w = (α+β)/(α+β+n)`, which *decreases* as `n` grows, so more data means *less* pull toward the literature. An earlier draft of this ADR stated that relationship backwards.

The model is a **conjugate Beta-Binomial with an informative prior**. It is *not* empirical Bayes — that would estimate the hyperparameters from the data rather than take them from published research — and it is not hierarchical unless partial pooling across buckets is added later. Earlier drafts used both terms incorrectly; see [design/CALIBRATION_LAYER.md](design/CALIBRATION_LAYER.md).

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

## ADR-008 — Parameters are derived or user-set; the testing profile is not the design profile

**Date:** 2026-07-29 · **Status:** Accepted

Every feature must work for any profile, any industry, any seniority, any market. The threat to that is not carelessness — it is that **whoever is testing supplies the vivid, concrete detail that makes a design feel well-grounded**, and their circumstances then get written into the spec as universals.

This is not hypothetical. The first draft of the three layer specs, written and reviewed in one sitting against a single live profile, contained all of the following:

| Leaked assumption | Whose it was | Why it fails others |
|---|---|---|
| "~5–15 tailored applications a week" | A selective senior candidate | Wrong by an order of magnitude for early-career, high-volume markets, or anyone under time pressure |
| Diversification across role families is valuable | A 25-year generalist | Harmful for licensed specialists; counterproductive for early-career candidates where breadth reads as unfocused |
| Structural factors = name bias, gaps, self-employment, market tightness | One person's circumstances | Omitted age, caregiving, disability, work authorisation, language, credential recognition, gender — the *dominant* factor for most users |
| Tier-2 ATS = Greenhouse / Lever / Ashby / Workday | US/Western tech employment | Matches almost nothing across most of the world's labour market |
| Employment-type detection by English substring match | An anglophone user | Silently degrades on every non-English posting |

None was written in bad faith; each felt like precision at the time, because it was precise — about one person.

**The rules that follow:**

1. **No constant that varies by circumstance.** Effort budgets, diversification caps, seniority bands, tailoring thresholds: derived from the user's own data, or asked. A number fitted to one profile and hardcoded is a defect, not a default.
2. **A valid parameter range includes its degenerate case.** *No* diversification must be reachable — which in this construction is a per-family cap **equal to the budget**, not a cap of 1. (A cap of 1 is *maximum* diversification; an earlier draft of this rule had it backwards.) A slate of 1, and a slate of 0, must also be reachable.
3. **Enumerated lists of human circumstance are assumed incomplete.** Structural factors, employment types, work arrangements, document formats. Default to adding, and never let the testing profile define the list's boundaries.
4. **A single-profile result is an illustration, never a validation.** Label it as such in the text. Claims about how often a mechanism pays off require the multi-shape trial matrix in [TEST_STRATEGY.md](TEST_STRATEGY.md).
5. **Geographic and linguistic coverage is stated, not implied.** Where a mechanism serves one region well, say which regions it does not serve, and treat that as a tracked gap.
6. **The tool does not moralise about the user's objective.** Someone applying broadly under time pressure has a different objective function, not a worse strategy.

**Review question for every PR:** *would this behave sensibly for a user unlike the person who wrote it?* If the answer needs a caveat, the caveat is the missing parameter.

## ADR-009 — Show the data and the conditional result; never prescribe the action

**Date:** 2026-07-29 · **Status:** Accepted · **Settles:** Board review Open Questions 6 and 7

The product sits on the **earning-intelligence** side of a line it must not cross into **advice**. The line is not about tone or hedging language — it is about what the tool is in a position to know.

| In scope — earning intelligence | Out of scope — advice |
|---|---|
| "This posted range sits below 8 comparable postings for this role and region" | "You should ask for £X" |
| "Relaxing your location constraint adds 0.9 expected interviews" | "You should relocate" |
| "Your callback rate is below baseline; field experiments attribute much of that gap to screening effects on [factor]" | "You should remove that from your CV" |
| "Applications you tailored deeply converted at 9–13%; lightly tailored at 2–5%" | "You should apply to fewer jobs" |

**The test:** can the claim be grounded in evidence the tool actually holds, and stated as a conditional or an observation? If it requires knowing the user's risk tolerance, finances, family situation, health, or what they want from their life — the tool does not know those things, and asserting them is advice wearing a data costume.

**Specific consequences:**

- **Compensation negotiation advice is out of scope** (Q6). Showing that a range is below comparable postings is in scope and is delivered by #149 and #120. Telling the user what to counter with, when to walk, or how to frame it is not — it is unmeasurable locally, and the tool has no visibility into the leverage that determines the answer.
- **Interview coaching is out of scope** ([REJ-007](#rej-007--ingratiation-and-interview-behavioural-coaching)). The tool observes nothing inside an interview; modelling it would be fabrication.
- **Structural-factor data is attribution, never instruction** ([REJ-008](#rej-008--acting-on-structural-bias-data-as-advice)).
- **Preference challenges are solved counterfactuals** (#153), reporting an objective delta under a stated assumption — not a recommendation to act.

**Why this is a hard boundary rather than a guideline:** advice is what the user most wants and what the tool is least equipped to give. Every feature in this product will, at some point, have an obvious-seeming extension that crosses this line and would demo extremely well. The reason to write it down is that the pull is toward crossing it.

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

## REJ-009 — Learned spatial indices (GLIN, Z-order curve, learned CDF)

**Status:** Rejected as premature

The proposal replaces tree-based spatial indexes with a small neural model that learns the cumulative distribution mapping Z-order addresses to record positions, claiming large speedups over cache-optimised B-trees and 40–70× less storage overhead.

The claims are plausible for the workload they target: hundreds of millions of geo-indexed records under high-frequency insertion. ATSassin does location matching over a few thousand postings held in SQLite, where location filtering is a string comparison against user preferences and takes microseconds. There is no spatial index to replace, and adding a learned model would mean introducing an inference dependency and a training step to optimise something currently unmeasurable — against a stated 4 GB CPU-only hardware floor.

*Revisit if:* location matching becomes genuinely geometric (commute-radius or isochrone queries over a large local corpus) **and** profiling attributes meaningful wall-clock to spatial lookup.

## REJ-007 — Ingratiation and interview behavioural coaching

**Status:** Rejected on scope and ethics

The research shows ingratiation tactics drive perceived Person-Organization fit, which drives offers. Building a feature that coaches users to flatter interviewers is unmeasurable locally (the tool observes nothing inside an interview), sits inside the liability zone the project has explicitly chosen to avoid, and is corrosive to the product's character. Interview-stage drop-off is real (32% of pipeline loss) but is not observable by this tool; modelling it would be fabrication.

## REJ-008 — Acting on structural bias data as advice

**Status:** Rejected permanently

Callback-penalty data for minority names, employment gaps, and prior self-employment enters the calibration model **only as attribution**, never as advice. Any feature suggesting name anglicisation, gap concealment, or similar is out of scope permanently, regardless of measured efficacy.

---

# Concept traceability

**This table is a navigation aid, not a completeness proof.** An independent three-agent audit on 2026-07-29 tested it against the source documents and found it claimed more than it delivered: roughly 30 rows against ~120 discrete concepts across the three papers, **four factually wrong rows**, and several dispositions that pointed at issues whose content did not match the claim.

The wrong rows are corrected below and the gaps they hid are now filed (#173–#179). The lesson is recorded rather than quietly fixed, because the failure mode generalises: **a traceability table is written by the same person who did the analysis, so it inherits their blind spots and then lends them false authority.** If you are checking whether something was considered, read the issue, not this row.

Corrections applied 2026-07-29:

| Row that was wrong | What was actually true |
|---|---|
| "ATS JSON API endpoints → #130, #149 (tier 2)" | #149 is **Tier 4**; #130 predates the ladder and names no endpoint. **Tier 2 had no issue at all** — the primary extraction path, and the mechanism used to justify superseding #119/#58. Now **#173** |
| "Legal / GDPR / ToS boundary → EVIDENCE_LAYER compliance section" | That section is three operational bullets and contains **no mention of GDPR, personal data, or takedown**. The local-controller case was never resolved. Now **#175** |
| "Local per-host rate limiting → #130, enforced locally" | #130 specifies per-host **concurrency**, a different axis. The rejected distributed GCRA has **no specified local successor**. Now **#174** |
| "FastText / ONNX → #133 (segments)" | #133 explicitly excludes ML ("pure regex keyword matching"). FastText appears only as a trailing aside on #163 |
| "Funnel baseline conversion rates → priors for the model, #150" / "#119 survives only as the prior table" | **Neither issue specifies the prior table.** #119 is entirely a salary dataset. The load-bearing input to the flagship feature was unspecified. Now **#176** |
| "Diversification as capacity constraint → derived from adjacency (#158)" | That derivation is an **unchecked TODO** in #158, presented here as settled — and it is the exact parameter ADR-008 flags as the leaked founding-persona assumption |
| "Arena, generational indices, slotmap/ECS, CSR, EBR → REJ-004" | REJ-004 does not mention **slotmap**, **ECS**, or **hazard pointers**. ECS is rejected only in prose elsewhere |

Also corrected outside this table: the min-cost flow cost function was written `−log P × decay`, which **inverts the model** — it makes a min-cost solver prefer stale postings. Correct form is `−log( P · decay )`. It was wrong in four places and is now fixed in all of them (#152).

Every substantive concept below, and where it landed.

## Job Application Conversion Metrics

| Concept | Disposition |
|---|---|
| Funnel baseline conversion rates | Priors for the model — #150 |
| Tailoring-depth lift (1–3% → 8–15%) | Fitted feature — #150 |
| Early submission velocity (<7 days) | Latency feature #150; age decay #152; real dates #149 |
| **Keyword paradox** (AUC 0.558; density negatively correlated with output) | **#168** — separate filter-pass likelihood from fit |
| **Application friction** (12.47% → 3.61% by form length) | **#167** — effort-weighted budget |
| **Negative duration dependence** (half supply-, half demand-side) | **#167** — time-varying baseline with controllable/structural split |
| Structural biases (audit-study callbacks) | #151 — attribution only |
| Interview→offer conversion (30–50%) | Transition in the model — #150 |
| Ingratiation / impression management | ❌ REJ-007 |
| **Conversion prior table (α, β, provenance)** | **#176** — was unspecified; both prior pointers led to a salary dataset |
| **Sector dispersion (&lt;2% tech vs 4–10% broader corporate)** | **#179** — a single global prior is wrong by up to 5× |
| **Scheduling latency (20% of leakage), posting liveness (16%), interview-process burden** | **#177** — REJ-007 covered only the 32% row |
| **Degree-credential filter; hard gates need a different model shape; 60–75% pre-review filter rate** | **#178** |
| Onboarding leakage (18%) | Post-offer — out of scope; recorded in #177 rather than left silent |
| 32% interview-stage pipeline leakage | ❌ REJ-007 — not observable by this tool |

## Lightweight Job Indexing Mechanism

| Concept | Disposition |
|---|---|
| CNAME enumeration | #147 |
| ATS JSON API endpoints | #130, #149 (tier 2) |
| `__NEXT_DATA__` SSR hydration | #148 |
| Schema.org JSON-LD `JobPosting` | #149 — the universal tier |
| **MinHash/LSH near-duplicate detection** | **#166** — adopted as SimHash at this scale (REJ-006) |
| FastText / ONNX classification | #163 (embedding choice), #133 (segments) |
| **zstd dictionary compression** | **#169** |
| Local per-host rate limiting | #130 — enforced locally, never delegated |
| **ATS JSON endpoints (`content=true`, `mode=json`, `includeCompensation=true`, Workday)** | **#173** — Tier 2. Note #149 is Tier **4** (JSON-LD), not Tier 2 |
| **Local per-host rate limiting (GCRA, local only)** | **#174** — the rejected distributed version had no successor |
| **GDPR local case, retention, erasure, scraping legal test, anti-bot position** | **#175** |
| Ryanair v. PR Aviation; EU Database Directive; UK CMA 1990 s3A | **#175** — reasoning restored, not just the operational rule |
| Naive Bayes / cONNXr embedded inference | Not adopted — FastText is the chosen direction (#163); recorded so it is not re-proposed |
| Kademlia DHT, S/Kademlia PoW, DCUtR, Merkle-CRDT, GCRA-in-DHT | ❌ REJ-001 |
| Cuckoo / Bloom filters | ❌ REJ-005 |
| LMDB | ❌ REJ-002 |
| Partitioned Elias-Fano, Block-Max WAND, MaxScore | ❌ REJ-003 |

## Rust Graph Engineering

| Concept | Disposition |
|---|---|
| Bipartite matching → **min-cost max-flow** | #152 — reformulated; plain max-cardinality matching is degenerate for one candidate |
| Counterfactual re-solve | #153 |
| Diversification as capacity constraint | #152, derived from adjacency (#158) |
| Arena allocation, generational indices, CSR, epoch-based reclamation | ❌ REJ-004 |
| Slot maps, ECS archetype graphs, hazard pointers | ❌ Same reasoning as REJ-004 (wrong scale) — named here because REJ-004's text omits them |
| **Vector-backed index graphs (`usize` into `Vec`, not `Rc<RefCell<_>>`)** | **Adopted** as a coding convention — `design/ALLOCATION_LAYER.md` §Representation. Orthogonal to scale; costs nothing |
| **Full flow construction: slack edge, τ reservation cost, quantisation, decay form, archetype multiplicity** | **#152** — specified after the audit found it unimplementable as written |
| Ford-Fulkerson, Hopcroft-Karp vs successive shortest paths | Recorded in `design/ALLOCATION_LAYER.md` §Implementation with reasoning, so the choice is not relitigated |
| LP / Flux Balance Analysis formulation | **Live option** — one of three candidate resolutions to the #152/#167 knapsack conflict |
| Belief propagation / message passing | Unadopted alternative to the sample-and-aggregate uncertainty method; noted in #152 discussion |
| Stackless async / tokio concurrency model, epoll/io_uring | Already the codebase; per-host fan-out control is #174 |
| Learned spatial indices (GLIN, Z-order) | ❌ REJ-009 |
| Kademlia DHT | ❌ REJ-001 |
| Flux Balance Analysis, Geneva-drive scheduling | Conceptual framing only — no implementation implied |
