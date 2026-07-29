# Layer 3 — Allocation: min-cost flow slate generation

**Status:** Design accepted, unbuilt · **ADR:** [ADR-006](../DECISIONS.md#adr-006--the-output-is-an-allocated-slate-not-a-ranked-list)
**Depends on:** Step 0, Layer 1, Layer 2

## Purpose

Turn the opportunity set into a **decision**: which applications to make this week, in what order, by when — and which to skip, with a reason.

This is the inflection. Everything below it is infrastructure for this output.

## Why ranking is the wrong primitive

Every competitor, and ATSassin today, sorts by score and shows the top N. That is a commodity: LinkedIn has one, every tool in the benchmark has one, and a better sort is a better commodity.

The user's actual constraint is not "which is best" but **"which subset, given that I can only do ~5–15 genuinely-tailored applications a week"**. Deep tailoring is what produces the 8–15% callback rate; volume destroys it, which is why mass-apply tools fail. Under that constraint, greedy ranking is provably suboptimal for three independent reasons:

1. **Postings expire at different rates.** A 6-day-old posting must be acted on now; a 1-day-old one can wait a cycle at no cost. Sorting by score cannot express a deadline.
2. **Effort is a shared, renewable-per-period resource.** Spending it on the #1 and #2 roles may be worse than #1 and #7 if #1 and #2 are near-identical.
3. **Marginal value is non-linear across role families.** With four VP Sales applications in flight, a fifth adds less expected value than a first Program Manager application. Ranking has no notion of what is already in the pipeline.

Point 3 is where greedy fails hardest, and it is **exactly this project's founding insight**. The trial that reframed "VP Sales" to "Program Manager" found stronger matches immediately. The board review files this under *unawareness* and treats it as a discovery problem. It is a **portfolio allocation problem**, and modelling it as one is the novel move.

A live scan on 2026-07-29 reproduced this precisely: against the same 44-company sweep, `"Sales Director"`, `"Country Manager"` and `"Fractional Chief Revenue Officer"` each returned **0** jobs, while `"Program Manager"` returned **46** and `"Business Development"` returned **43**. The obvious senior titles were empty; the adjacent framing was where the roles were. Diversification is not a hedge here — it is where the opportunities actually are.

## Formulation

Min-cost max-flow. Not maximum bipartite matching — with a single candidate that is degenerate.

```
                  [cap = weekly effort budget]
        source ───────────────────────────────▶ user
                                                 │
                        [cap = diversification cap per family]
                                                 ▼
                                          role archetype
                                                 │
                     [cap = 1, cost = −log P(callback) × decay(age)]
                                                 ▼
                                             posting
                                                 │
                                          [cap = 1]
                                                 ▼
                                               sink
```

Each element earns its place:

- **Source capacity** = the honest weekly tailoring budget. Encodes the constraint mass-apply tools pretend away.
- **Role-archetype capacity** = diversification cap. This is what forces exploration of adjacent families rather than piling into one.
- **Edge cost** = `−log P(callback)` from [Layer 2](CALIBRATION_LAYER.md), so minimising total cost maximises expected callbacks (log-additive over independent applications).
- **Age decay** operationalises the <7-day review-bandwidth window as a term in the objective, not a tip in the docs. Requires real `datePosted` from [Layer 1](EVIDENCE_LAYER.md).
- **Posting capacity 1** — an application is one-shot and non-repeatable.

### Uncertainty

Layer 2 returns intervals, not point estimates. Two options, in order of preference:

1. **Solve on the posterior mean, report the objective interval.** Cheap, and honest about what is known.
2. **Sample-and-aggregate.** Draw *k* cost matrices from the posteriors, solve each, report selection frequency per posting. A posting selected in 95% of draws is a confident recommendation; one selected in 40% is genuinely marginal, and saying so is more useful than a false ordering.

Option 2 is preferred once Layer 2 has enough data to make the posteriors distinguishable. Below that, option 1 with a prominent `prior_dominated` flag.

## Implementation

Successive shortest paths with potentials (Bellman-Ford init, then Dijkstra). Roughly 200 lines, or `petgraph`, which is already a plausible dependency.

Scale: a few thousand postings, tens of role archetypes, one user. **Milliseconds, single-threaded.** No arena allocation, no CSR, no lock-free structures, no epoch reclamation — see [REJ-004](../DECISIONS.md#rej-004--arena-allocation-generational-indices-csr-epoch-based-reclamation). The graph is rebuilt per solve; there is no mutation hot loop to optimise.

The hardware floor is untouched: this adds no model, no index, and no persistent structure.

## Counterfactual re-solve

Falls out for free, and closes #122 far more credibly than the LLM-prompt approach currently specced.

Re-solve with one constraint relaxed, diff the objective:

```
Current slate:                    1.2 expected interviews
  + include remote-EU postings:   2.1  (+0.9)
  + raise budget 7 → 10/week:     1.6  (+0.4)
  + accept 6-month contracts:     1.4  (+0.2)
```

A *solved counterfactual* is a materially stronger claim than a model's opinion about relocation. It is also honestly bounded — it says what the data implies under the stated assumptions, not what the user should do.

This is the "preference challenge" feature, and it discharges the mission sentence — *"which adjacent role values it most, and how do I credibly walk into that room?"* The diversification constraint surfaces the adjacent role; the calibrated probability makes "credibly" a number.

## Output

```
Slate — week of 2026-07-29        budget 7/7 used

1. Fractional Head of Business Development · PHARMExcel · UK
   fit 85% · posted 8d ago · contract · apply by Thu (day 11)
2. …

Not selected, and why:
  Senior PM · Acme        — family cap reached (3 program-management already selected)
  VP Sales · Globex       — posted 34d ago, decayed below threshold
  Head of BD · Initech    — below comp floor

Expected outcome: 0.9 – 1.6 interviews  (n=12, prior-dominated)
```

The "not selected, and why" section is not a courtesy. It is the difference between a decision and an oracle, and it is what lets a user disagree with the tool on a specific ground rather than distrust it wholesale.

## Relationship to existing issues

- **#122** — mechanism replaced by counterfactual re-solve; the four bespoke heuristic signals are dropped.
- **#133** — segment tags become the diversification dimension rather than a prerank weight.
- **#121** — the market-watch daemon becomes the trigger that regenerates the slate rather than a scanner that appends rows.
- **#106** — reframed as the tracker over Layers 2 and 3.

## Acceptance criteria

1. Solver returns a slate respecting budget, diversification caps, and expiry; deterministic for a fixed input.
2. Every non-selected posting above the relevance floor carries a machine-generated reason.
3. Expected-outcome reporting is an interval and carries `prior_dominated` when Layer 2 says so.
4. Counterfactual re-solve supports at minimum: location relaxation, budget change, employment-type widening, comp-floor change.
5. Solve completes in <100 ms for 5,000 postings on the `light` hardware tier.
6. No new heavy dependency; no persistent index.
