# ATSassin: Adversarial Review & Architectural Inflection Point

**Date:** 2026-07-29
**Status:** Proposed — supersedes nothing; extends `VENTURE_BOARD_REVIEW.md`
**Method:** Every claim below is verified against the code at the cited `file:line`. Where the board review and the code disagree, the code wins.

---

# Phase 1 — Adversarial Review

The foundations are better than most projects at this stage: `cargo clippy -D warnings` and `cargo test` both gate PRs (`ci.yml:35,39`), the toolchain is pinned, the secrets scan over tracked files is clean, `.gitignore` is unusually well-reasoned, the board-health canary genuinely asserts, and the release profile is correctly size-tuned. The daemon really is a full orchestrator, and the six-dimension rubric really exists (`prompts.rs:89`).

But there are three defects that must be fixed before anything is built on top, and one of them breaks the product's primary promise.

## P0-1 — The PII gate does not cover the file that leaves the machine

> **Resolution (2026-07-29): fixed by #143 and #81 (closed).** `ValidatedTrainingPayload` now owns the exact checked bytes, and `LightningClient::submit_training_job` cannot accept a raw path. The egress check runs after export and before the first network request, fails closed without candidate identity context, and creates no `*.flagged.jsonl` copy. Deterministic detector fixtures now cover representative SG, UK, India, EU, and US shapes with false-positive controls. The audit below is retained as the historical finding; this is not a claim of universal NER.

This is the most serious finding. In the only code path where user data egresses:

```
cli.rs:1429   export_training_data(...)                 → runs pii_gate (distillation.rs:250)
cli.rs:1435   export_from_feedback_and_telemetry(...)   → writes training_pairs.jsonl
cli.rs:1449   training_file = output_dir/"training_pairs.jsonl"
cli.rs:1457   client.submit_training_job(..., &training_file)   → uploads to Lightning AI
```

The gate runs at step one, against a directory that does not yet contain the file written at step two. `export_from_feedback_and_telemetry` (`distillation.rs:326`) never calls `pii_gate` itself. **The one file transmitted off-device is the one file never checked.** `VENTURE_BOARD_REVIEW.md:506` and `:813` both assert "PII gate validates final output."

Two compounding defects:

- **The gate leaks on abort.** `distillation.rs:271-276` copies the PII-bearing file to `output_dir.parent()` as `*.flagged.jsonl` *before* `remove_dir_all(output_dir)`. The abort path preserves the PII one directory up, outside the directory it deletes.
- **The detectors are US-only.** `pii_scrubber.rs:21-23` is a strict 10-digit NANP pattern; `:31-33` matches US street suffixes. A Singapore `+65 9123 4567`, a UK number, or an Indian number matches nothing. The comment at `:20` claims "international and US formats" — it is false. No detection at all for person names, national ID/NRIC, passport, DOB, or social handles. `ScrubContext.preserve_names` (`:57`) is declared and never read.

Given the maintainer and first-trial persona are Singapore-based, the default configuration fails for its own founding user.

**Fix:** move `pii_gate` to a single choke point that runs after *all* writes and immediately before any upload; make it fail closed; delete the flagged-copy behaviour or write it to a path the user explicitly opts into; extend the regex set to E.164 international forms before any non-US user is onboarded.

## P0-2 — Job identity is random, so nothing downstream can be trusted

Every live scan path assigns a fresh v4 UUID as the job's primary key:

| Path | Line |
|---|---|
| CLI scan | `cli.rs:758` |
| TUI scan | `tui.rs:607` |
| Daemon tick | `daemon.rs:114` |

A v4 UUID is random and has no relationship to the posting's content or URL. The schema (`tracker.rs:76-91`) has `id TEXT PRIMARY KEY` and `url TEXT NOT NULL` with **no uniqueness constraint on `url`**, and writes go through `INSERT OR REPLACE` keyed on that random id (`tracker.rs:175`). There is no dedup anywhere in the job pool — the only `deduplicate` in the codebase (`social_scraper.rs:93`) is an in-memory pass over one social batch, keyed on `(url, title)`, discarded after the call.

The consequences compound:

1. **Re-scanning duplicates the pool.** The same posting scanned twice is two rows.
2. **The evaluation cache can never hit.** `get_evaluation` looks up by `job_id` (`tracker.rs:655`), which is a new random value each scan, so `scorer.evaluate` (`scorer.rs:17`) calls the LLM again for content it has already paid to evaluate.
3. **The daemon re-evaluates everything, forever.** `daemon.rs:132` saves each job (always succeeding — no key collision is possible), pushes it to `new_jobs`, and `daemon.rs:141` only skips evaluation when that vector is empty. It never is. With the default `interval_sec: 3600` (`daemon.rs:38`), the daemon re-evaluates every job on every board **every hour, indefinitely**, at full LLM cost, for postings whose text has not changed.
4. **`recommend` ranks duplicates against each other**, and the feedback/calibration data in `feedback.rs` is fragmented across the duplicate ids.

This is not merely a bug. **It makes the continuous market-watch daemon (issue #121) — the centerpiece of the career-coach roadmap — impossible to ship**, because shipping it means shipping unbounded, silent, recurring spend on redundant work. Everything in Phase 2 below depends on this being fixed first.

**Fix:** derive the job id from a canonicalised identity — normalised URL (strip UTM and tracking params, lowercase host, drop fragments) hashed, falling back to `hash(company + title + normalised_location)` where the URL is a search page. Add `UNIQUE(url_canonical)`. Then `INSERT ON CONFLICT DO UPDATE` becomes a real upsert, the evaluation cache starts hitting, and the daemon becomes cheap.

## P0-3 — Real-person PII in and beside the repository

> **Resolved 2026-07-30 (#146):** Scenario 1 is now a wholly synthetic senior APAC GTM persona with equivalent test shape. Current-tree test, example, and report references are anonymised; historical observations remain annotated.

- At the time of this review, the `tests/uat/scenario_1_*` fixture was **tracked** in a public MIT repo and contained a real individual's employment history, employers, awards, and immigration details — including in the directory name itself. `VENTURE_BOARD_REVIEW.md` re-identified the same person in prose (now removed).
- At review time, `tail-notion-country-manager.md` at the repo root was a second CV export for the same person and was not ignored. It is now covered by the root `/tail-*.md` rule, so `git add -A` cannot publish it.
- At review time, `.freebuff/` was an 8.26 MB SQLite database from an unrelated desktop application, actively writing WAL files inside the working tree, with zero references anywhere in the repo. The directory is now covered by `/.freebuff/`.

The `.gitignore` is otherwise scrupulous about exactly this class of file (`:16`, `:36-37`, `:56-60`), which makes these the gap rather than the pattern.

**Resolution:** Scenario 1 is synthetic; `.freebuff/` and the root CV export pattern are ignored. Local ignored material remains outside repository history and must not be force-added.

## Correction — this review's stub hunt had a blind spot

The Phase 1 sweep searched for `todo!()`, `unimplemented!()`, `TODO`, `FIXME`, `"stub"`, `"placeholder"` and similar markers, and reported the production paths clean. A follow-up audit on the same day found that conclusion was **too narrow to be useful**.

The dangerous case carries no marker. It compiles, returns plausibly-shaped data, and is indistinguishable from a working implementation at every call site:

```rust
// profile_parser.rs:631 — the parameter is discarded
fn extract_education(_text: &str) -> Vec<Education> {
    vec![Education { institution: "Unknown".into(), degree: "N/A".into(), … }]
}
```

That fabricates an education entry for **100% of markdown/DOCX/portfolio users**, and it reaches their generated resume (#162). A marker search will never find it.

Two more of the same class, both missed for the same reason:

- `matcher.rs:183-190` — `semantic_score` returns the **L2 norm of a single embedding** of the concatenated job and resume. It is not a similarity: there is no second vector and no cosine. It carries **0.40, the largest weight** in the composite (#163).
- `prompts.rs:89` — the six rubric dimensions are undefined noun phrases, "North-star alignment" is scored against a target never supplied, and the candidate's **name** sits in the same prompt that scores "Cultural signals" (#164).

**The lesson for future audits:** search for *semantic* stubs, not lexical ones. Concretely — functions whose parameters are `_`-prefixed but which return constructed data; functions returning a constant where a computation is implied; and any scoring component that can be checked against its own definition (a similarity that never compares two things is the tell). A grep-based CI check for the first pattern is part of #162.

## P1 — Honest-failure violations

The project's fourth design principle is "honest failure over fabricated plausibility." Four places break it, and the first one matters most for Phase 2.

**Fabricated posting dates.** `scraper.rs:872,951,1003` and seven sites in `social_scraper.rs` (`:275,320,368,420,543,589,641`) set `posted_at: Some(Utc::now())` — asserting scrape time as post time. This is not inert: it flows into `landscore::score` (`daemon.rs:201-207`) where `landscore.rs:71` grants a full +15 recency bonus to anything dated today, versus +5 for an honest `None`. **The ranking therefore systematically promotes the sources that fabricate their dates over the sources that report them truthfully.** An honest `None` is strictly better than a false `now()`.

**Fabricated evaluations.** `scorer.rs:102-111` synthesises a 0.5 score with `match_summary: "Auto-generated fallback evaluation."` when the LLM returns unparseable JSON. That fabricated score is persisted (`daemon.rs:209`), graded, converted to `Recommendation::Maybe`, and fed into ranking. Nothing downstream checks the sentinel string.

**Fabricated fields.** Job `url` is set to the *search page* URL rather than the posting (`scraper.rs:871,950,1002`); `location` is hardcoded `"Remote"` at eight sites in `social_scraper.rs`; `company` becomes `"{board} (via MCP Browser)"` at `scraper.rs:948`.

**Fabricated market data.** `cli.rs:880-915` — the `roles research` command constructs a `RoleArchetype` with hardcoded `fit_score: 0.85`, `posting_volume_30d: 1200`, `DemandLevel::High`, `TrendDirection::Growing`, and a USD 150–240k compensation band labelled `source: "Market Intelligence"`. This is invented data presented to the user under a research heading.

**Error swallowing hides real failures.** `scraper.rs:83-159` applies `.unwrap_or_default()` to all ~20 dispatch arms, collapsing every `Err` into an empty `Vec` before returning `Ok`. As a result `scanner.rs:52-54` can never populate `errors`, making the `"All boards failed"` branch at `:59` unreachable dead code. A total network outage is reported to the user as *"No jobs returned… Try a different query"* — a misdiagnosis that will send users chasing their search terms instead of their connection.

## P1 — Supply chain and coverage

**OpenSSL is still in the tree** via three independent paths: `Cargo.toml:78` (`imap 2.4`, which has no rustls feature), `Cargo.toml:81` (explicit `native-tls`), and `Cargo.toml:33` where `reqwest` lists `rustls-tls` but omits `default-features = false`, leaving `default-tls` enabled alongside it. `Cargo.lock` confirms both stacks are linked. Adding `rustls-tls` was a no-op for removing OpenSSL. This also undercuts the static-single-binary claim, since `openssl-sys` introduces a native system-library dependency. Separately, `imap-proto v0.10.2` emits a future-incompatibility warning on the current toolchain — the `imap` dependency is now a liability on two axes.

**Zero-test modules that own critical state:** `tracker.rs` (980 lines — all user state, the one thing whose corruption is unrecoverable), `distillation.rs` (566 lines — a CODEOWNERS-designated privacy area), `scorer.rs`, `matcher.rs`, and the entire `tailor → llm → router` value chain. Total coverage is 105 tests, but concentrated away from the highest-risk code. `daemon.rs:349` contains `assert!(result.is_ok() || result.is_err())` — a tautology that always passes.

## P2 — Claim drift

The binary is **10.96 MB** as measured on a clean `--release` build. `VENTURE_BOARD_REVIEW.md` claims 8.14 MB at `:43` and `:68`, and ~9.5 MB at `:811` — it contradicts itself, and both figures are wrong. Other drift: Greenhouse companies are 44 in `company_directory.rs` against ~35/~36 claimed; CLI commands are 45 against "~50"; "11+ scraping surfaces" reconciles to no enumeration in `scraper.rs`. `CODEOWNERS` resolves every rule to a single owner with three of five areas commented out, while `VENTURE_BOARD_REVIEW.md:663-664` advertises "area leads" reviewing PRs — `CONTRIBUTING.md:90-93` is honest about this; the venture doc is not.

Also: the Lightning 401 (now tracked as **#154**; #6 closed) is a **code defect, not a credential issue** — `.env.example:18` declares `LIGHTNING_USER_ID`, which is never read anywhere in `src/`, and auth is bearer-only (`lightning.rs:113,138,160`). The Unsloth training script is a placeholder comment (`distillation.rs:419-421`) with an unused import, and the GGUF script probes a filename llama.cpp renamed and passes a `--outtype` value that only accepts `f32/f16/bf16/q8_0` — the advertised Q4_K_M path cannot execute. `config.rs:422` calls `sync_tier_models()` unconditionally, collapsing light/balanced/full to a single model on every load.

## Phase 1 verdict

The foundation is sound in engineering discipline and unsound in three specific places: **data egress (P0-1), job identity (P0-2), and PII in the tree (P0-3)**. P0-2 in particular is load-bearing for everything below — the architecture in Phase 2 cannot be built on randomly-keyed jobs. Fix those three, plus the fabricated `posted_at`, and the foundation is clean.

---

# Phase 2 — The Inflection Point

## The thesis

**ATSassin ranks. Category-defining is to allocate.**

Every competitor — and ATSassin today — answers the same question: *"which jobs are the best match?"* The output is a sorted list. Lists are a commodity; LinkedIn has one, every tool in the benchmark has one, and a better list is a better commodity, not a different category.

The user's actual problem is not ranking. It is **allocation under constraint**. They have a finite budget of genuinely-tailored applications per week (deep tailoring is what produces the 8–15% callback rate; volume destroys it — which is precisely why the mass-apply competitors fail). Each posting decays in value with age. Each application is one-shot and non-repeatable. And each additional application to the same role family adds less expected value than the first to a new one.

That is not a ranking problem. It is a **capacitated allocation problem over a decaying opportunity set with per-opportunity conversion probabilities.**

Answering it produces a fundamentally different artifact:

> *"Here is your slate for this week: these 7 applications, in this order, by these deadlines. Not the other 40, and here is why. Expected outcome: 1.2 interviews. If you relaxed your location constraint to include remote-EU, it becomes 2.1."*

That is a **decision**, and it is counterfactual, budget-aware, and time-aware. No competitor can produce it, because producing it requires longitudinal per-user outcome data that only a local-first tool can ethically hold. **ATSassin's privacy architecture is not a constraint on this feature — it is the moat that makes it exclusive.**

The three research documents map onto exactly the three layers this requires. Individually each is a shopping list; composed, they are one system, and each layer is useless without the one beneath it.

| Layer | Source document | Supplies |
|---|---|---|
| **Evidence** | Lightweight Job Indexing | Honest structured facts: real dates, real compensation, real work-mode |
| **Calibration** | Conversion Metrics | The objective function, fitted to *this user's* measured outcomes |
| **Allocation** | Rust Graph Engineering | The optimiser that turns facts + probabilities + constraints into a slate |

## Layer 1 — Evidence: the tiered extraction ladder

**Accept, and prioritise. This is the highest-value idea across all three documents.**

Strip the indexing document of its P2P religion and what remains is a four-tier extraction ladder that eliminates the headless browser:

1. **CNAME enumeration** — resolve `careers.company.com`; if it aliases to `boards.greenhouse.io`, `jobs.lever.co`, etc., the ATS is identified without fetching a page.
2. **Direct ATS JSON APIs** — Greenhouse `/v1/boards/{token}/jobs?content=true`, Lever `/v0/postings/{slug}?mode=json`, Ashby `/posting-api/job-board/{name}?includeCompensation=true`.
3. **`__NEXT_DATA__` / SSR hydration blobs** — parse the serialised state out of the raw HTML text stream for Next.js/Nuxt career pages.
4. **Schema.org JSON-LD `JobPosting`** — the generic tail, giving `datePosted`, `baseSalary`, `employmentType`, `jobLocationType`, `directApply`.

What makes this the inflection-grade move is not that it is faster. It is that **one mechanism collapses four separate roadmap workstreams and repairs a correctness defect**:

| Currently filed as | Resolved by |
|---|---|
| #116 — autonomous ATS detector | Tiers 1–2 (CNAME + endpoint probe) |
| #119 / #58 — real salary data replacing LLM estimation | Tier 2 `includeCompensation`, tier 4 `baseSalary` |
| #117 — visa/experience restriction parser | Tier 4 structured fields, not regex over prose |
| P1 fabricated `posted_at` (above) | Tier 2/4 authoritative `datePosted` |

The salary point deserves emphasis. Issue #119 proposes building and maintaining "a periodically-updated lightweight JSON file" of role × region × seniority baselines. That artifact would go stale, has no provenance, and requires perpetual curation. **Tier 2/4 gives structured compensation directly from the employer, per posting, with perfect provenance and zero maintenance.** It is strictly better than the thing it replaces, and it eliminates a permanent chore rather than creating one.

The existing `scrape_board` dispatch (`scraper.rs:79-161`) is already a string-keyed match with per-board arms and real Greenhouse/Lever/Ashby clients. The ladder slots in behind the `JobSource` trait already designed in issue #130 — this is an extension of planned work, not a detour.

**Sequencing note:** honest `datePosted` from this layer is what makes the recency signal real. Until then, `landscore`'s recency term is being fed fabricated timestamps and is actively harmful. Delete the fabrication first (P1); this layer then restores the signal legitimately.

## Layer 2 — Calibration: the personal conversion model

**Accept the metrics as priors. Reject them as targets. Reject the behavioural coaching entirely.**

The conversion research is a set of population base rates. The naive integration — displaying "tailored applications convert at 8–15%!" in the UI — is worthless; it is a statistic the user can read anywhere and cannot act on.

The correct use is structural: **those tables are prior distributions for a per-user Bayesian model.** ATSassin already captures every input this model needs, and computes none of them:

| Model input | Already in the code | Currently computed? |
|---|---|---|
| Tailoring depth | `feedback.rs:14` `edit_distance` | No |
| Submission latency | `jobs.posted_at` → `pipeline.updated_at` at `Applied` | No |
| Callback / interview / offer events | `PipelineStatus`, `outcomes.rs` IMAP ingestion | Recorded, never aggregated |
| Role-fit score | `scorer.rs` 6-dimension evaluation | Yes, but never correlated with outcome |

The substrate is built. The analytical layer on top of it does not exist. That gap is the opportunity: this is the thing issue #115 ("calibrate against real outcomes") is gesturing at, and it is the single most defensible feature in the product.

Two design constraints fall directly out of the research, and both are non-obvious:

**Constraint 1 — Shrinkage is mandatory, not optional.** Most users will have fewer than 20 applications. Estimating a personal callback rate from 20 Bernoulli trials is statistically hopeless done naively — a user with 1 callback in 12 would be told their rate is 8.3% with no error bar. The correct estimator is Bayesian (Beta-Binomial): shrink the personal rate toward the published prior, weight by observation count, and **report the interval, not the point estimate**. At low `n` the tool should say plainly that it does not yet know. This is the project's "honest failure" value expressed statistically, and it is what makes the feature trustworthy rather than a random-number generator with a confident font.

**Constraint 2 — Controllable and structural factors must be modelled separately, and only controllable ones may drive recommendations.** The research documents severe structural penalties: minority- or foreign-named candidates receive 25–50% of baseline callbacks; prior self-employment yields under 10%; employment gaps over six months trigger automatic filters. A naive calibration model fed these outcomes learns *"you are a weak candidate"* and down-ranks the user's ambitions — when the actual signal is market discrimination.

This is a genuine safety requirement, not a nicety. The model must decompose the observed rate into a controllable component (tailoring depth, submission latency, role-fit) and a structural component (name, gap history, self-employment, market tightness), and:

- Recommend only on the controllable component.
- **Attribute the structural component explicitly to the user** — "your callback rate is below the population baseline; correspondence studies attribute a large share of this gap to name-based screening bias rather than candidate quality" — so a user does not conclude from a low number that their skills are worthless.

Framed correctly, this turns the most uncomfortable finding in the research into the most humane feature in the product, and it lands squarely on the "earning intelligence, not career advice" side of the line the board review draws at Open Question #7.

**Explicitly rejected from this document:**

- **Ingratiation coaching.** The research shows ingratiation tactics drive perceived P-O fit which drives offers. Building a feature that coaches users to flatter interviewers is unmeasurable locally, squarely inside the liability zone the board review already identified, and corrosive to the product's character. Reject.
- **Bias "optimisation."** Any feature that acts on the callback-penalty data by suggesting name anglicisation or gap concealment is repugnant and plausibly unlawful to recommend. The data enters the model as *attribution*, never as *advice*. Reject.
- **Interview-stage behavioural scoring.** 32% of pipeline drop-off occurs at the interview stage, but ATSassin observes nothing inside the interview. Modelling it would be fabrication. Reject.

## Layer 3 — Allocation: min-cost flow over the opportunity set

**Accept the flow formulation. Reject essentially everything else in the graph document.**

The graph engineering brief is high-quality general Rust systems knowledge that is almost entirely inapplicable here. Arena allocation, generational indices, ECS archetype graphs, CSR layouts, and `crossbeam-epoch` reclamation all matter when a large mutating in-memory graph is traversed in hot loops. ATSassin's data lives in SQLite, its graph is a few thousand nodes, and its wall-clock is dominated by network and LLM latency measured in *seconds*. Introducing CSR or epoch-based reclamation would shave nanoseconds off a workload bounded by round-trips. **Reject — this is premature optimisation wearing rigorous clothing, and the document's own framing invites it.**

But one idea in it is transformative and the document under-argues it: **the bipartite flow formulation of the application funnel.**

The document presents it lazily — "candidates and jobs form a bipartite graph," Hopcroft–Karp for maximum matching. For a single user that is degenerate: one candidate, so maximum-cardinality matching is trivial. Taken at face value it is numerology.

The real problem underneath is not maximum matching. It is **min-cost flow under capacity constraints**, and that formulation is exactly right:

```
source ──[capacity = weekly effort budget]──▶ user
user ──[capacity = diversification cap]──▶ role archetype
role archetype ──[cost = 1 − P(callback)·decay(age)]──▶ posting
posting ──[capacity = 1]──▶ sink
```

Solving for minimum cost yields the optimal *slate*, and each structural element earns its place:

- **Source capacity** encodes the honest truth that deep tailoring does not scale — the constraint the mass-apply competitors pretend away.
- **Diversification caps on role-archetype edges** are what force exploration of adjacent roles. This is the mathematical form of the founding trial's central insight — that "try Program Manager instead of VP Sales" found stronger matches immediately. The board review files this under "unawareness" and treats it as a discovery problem; **it is a portfolio allocation problem**, and modelling it as one is the genuinely novel move.
- **Edge costs from Layer 2** mean the objective is the user's own measured conversion probabilities, not industry benchmarks.
- **Age decay** operationalises the <7-day review-bandwidth window as a real term in the objective rather than a tip in the docs.

**An earlier draft claimed greedy ranking is provably suboptimal here. That was wrong** — the specified constraint set is a truncated partition matroid with a modular objective, which greedy solves exactly, and the "a 1-day-old posting can wait a cycle" argument needs a multi-period model that does not exist. The flow formulation earns its place because **effort weighting** breaks the matroid — that is the only surviving justification. Posting liveness was also claimed and is wrong: removing postings is a matroid restriction, which is still a matroid.

**On implementation cost:** min-cost max-flow over a few thousand nodes is *milliseconds*, single-threaded, in stock Rust. Successive shortest paths is ~200 lines, or `petgraph` — already a candidate dependency — provides it. No arena, no CSR, no lock-free structures, no new heavy dependency. The hardware floor is untouched.

**The counterfactual capability falls out for free.** Re-solve with one constraint modified, diff the objective value, and report the delta: *"relaxing your location constraint adds 0.9 expected interviews."* That is issue #122 (preference-challenge insights), currently specced as an LLM prompt — and a solved counterfactual is far more credible than a model's opinion. Another roadmap item collapsed into the same mechanism.

## What to reject from the indexing document, and why it matters

The decentralised P2P architecture — libp2p, Kademlia/S-Kademlia with PoW node IDs, DCUtR NAT traversal, Merkle-CRDT state sync — should be **rejected and recorded as rejected**, so it is not re-proposed. The reasons are not "too complex"; they are disqualifying on their own terms:

1. **The DHT-coordinated rate limiter is a DDoS vector.** The design stores each domain's Theoretical Arrival Time in the DHT and has peers consult it before fetching. The resource being protected — the target employer's web server — is *outside* the trust boundary. Any malicious peer reporting a TAT in the past induces every honest node to hammer that domain simultaneously, turning the user base into a botnet; reporting a TAT far in the future silently suppresses discovery of specific employers. No honest-majority assumption fixes this, because the victim is not a network participant. This alone disqualifies the design.
2. **It inverts the privacy architecture.** "Personal data never leaves the machine" is the product's first value and its core differentiator. In a P2P index, every user becomes a publisher of scraped third-party personal data — hiring manager names and emails. The document's own GDPR reasoning (Art. 14(5)(b) disproportionate effort, Art. 6(1)(f) legitimate interest) is argued for *a single controller operating a public archive*; it does not transfer to a design that makes every end user a controller.
3. **PoW node-ID generation contradicts the 4 GB CPU-only hardware floor** — the floor that is itself still unvalidated (live issue #73; #5 and #57 closed as duplicates).
4. **The bootstrap economics are inverted.** A DHT with three users indexes nothing. Value is zero until adoption is large; complexity is paid on day one.

The board review already rejected blockchain coordination and P2P federated learning (§9) for structurally identical reasons. This belongs in the same section.

## Scale-calibrated verdicts on the remaining techniques

Ruthless first principles cuts both ways: these are good techniques, and they are good at scales ATSassin does not operate at.

| Technique | Verdict | Reasoning |
|---|---|---|
| **Canonical content-addressed job identity** | **Adopt now — P0** | Fixes P0-2. Trivial, urgent, unblocks everything. |
| **SimHash near-duplicate detection** | **Adopt, scaled down** | Cross-board syndication is rampant and real. But full MinHash+LSH with b=9/r=13 banding is tuned for billions of documents. At a few thousand, a 64-bit SimHash with Hamming-distance bucketing is ~100 lines and sufficient. Adopt the idea, reject the implementation scale. |
| **FastText / ONNX classification** | **Adopt — substitute for the current plan** | Character n-gram OOV handling is genuinely right for ATS jargon and misspellings. A quantised FastText model is ~1–15 MB against ~90 MB for the all-MiniLM-L6-v2 currently proposed in #118. Better fit for the hardware floor. Recommend substituting into #118/#133. |
| **zstd dictionary compression** | **Adopt — small, real** | The project already does zstd cold archival. Dictionary mode is well-matched to 1–5 KB JSON payloads with heavy boilerplate overlap (EEO statements, benefits blurbs), roughly tripling the ratio. ~30 lines against a dictionary trained on the user's own corpus. |
| **Cuckoo filters** | **Reject — premature** | The deletion argument is correct in principle. But the visited-URL set is thousands of entries — a `HashSet<u64>` costs ~100 KB. Both Bloom and Cuckoo solve a problem the product does not have. Record as a scaling trigger, not a build item. |
| **LMDB replacing SQLite** | **Reject** | Zero-copy reads are irrelevant when the working set is thousands of rows and the bottleneck is network and LLM latency. SQLite already holds relational pipeline/evaluation/telemetry data with joins the key-value model would force into application code. A multi-week migration for no user-visible gain. |
| **Partitioned Elias-Fano + Block-Max WAND** | **Reject decisively** | Techniques for inverted indexes over hundreds of millions of documents. `prerank.rs` scores a few thousand jobs with smoothed TF-IDF in memory in microseconds, and is correct as written. PEF would add hundreds of lines of bit manipulation to optimise something unmeasurable. |
| **Arena / generational indices / CSR / EBR** | **Reject** | See above — optimising nanoseconds in a workload bounded by seconds. |

## Sequencing

The dependency order is forced — each step is worthless without its predecessor, and the first two are the P0 bug fixes.

**Step 0 — Repair the foundation.** Canonical job identity (P0-2); PII gate at a single pre-upload choke point plus international detectors (P0-1); PII files out of the tree (P0-3); delete fabricated `posted_at`, fabricated fallback evaluations, and the fabricated `roles research` archetype; stop swallowing scraper errors. *Nothing below is trustworthy until this lands.*

**Step 1 — Evidence layer.** Tiered extraction ladder behind the `JobSource` trait (#130). Delivers real `datePosted` and real compensation. Closes #116, #119, #58, #117.

**Step 2 — Calibration layer.** Compute submission latency and tailoring depth from data already captured. Bayesian (Beta-Binomial) conversion model with mandatory shrinkage, interval reporting, and controllable/structural decomposition. Closes #115.

**Step 3 — Allocation layer.** Min-cost flow slate generation with effort budget, diversification caps, and age decay. Counterfactual re-solve for preference challenges. Closes #122, reframes #121.

Steps 0 and 1 are bug fixes and planned work that happen to be the foundation. Steps 2 and 3 are the inflection. Nothing in the sequence requires a new heavy dependency, a daemon on light hardware, a network service, or any relaxation of the privacy architecture.

## Why this is the inflection and not a feature

A feature makes the list better. This changes what the product outputs — from a ranked list, which is a commodity, to an allocated slate with an expected outcome and a counterfactual, which nobody else can produce.

It is defensible for a structural reason rather than a technical one: the calibration layer requires longitudinal, per-user outcome data. A cloud competitor could build the optimiser, but asking users to hand over their complete application and rejection history is a trust ask that a SaaS product cannot make and a local-first binary does not have to. **The privacy architecture stops being a constraint the product works around and becomes the reason the flagship feature is exclusive to it.**

And it discharges the mission as stated: *"Given what I actually know and can do, which adjacent role values it most, and how do I credibly walk into that room?"* The diversification constraint is what surfaces the adjacent role. The calibrated conversion model is what makes "credibly" a number rather than an adjective.
