# Layer 3 — Allocation: min-cost flow slate generation

**Status:** Design accepted, unbuilt · **ADR:** [ADR-006](../DECISIONS.md#adr-006--the-output-is-an-allocated-slate-not-a-ranked-list)
**Depends on:** Step 0, Layer 1, Layer 2

## Purpose

Turn the opportunity set into a **decision**: which applications to make this week, in what order, by when — and which to skip, with a reason.

This is the inflection. Everything below it is infrastructure for this output.

## Why ranking is the wrong primitive

Every competitor, and ATSassin today, sorts by score and shows the top N. That is a commodity: LinkedIn has one, every tool in the benchmark has one, and a better sort is a better commodity.

The user's actual constraint is not "which is best" but **"which subset, given a finite budget of genuinely-tailored applications per period"**. Deep tailoring is what produces the higher callback rate; undifferentiated volume does not, which is why mass-apply tools fail. Under that constraint, greedy ranking is provably suboptimal for three independent reasons:

1. **Postings expire at different rates.** A 6-day-old posting must be acted on now; a 1-day-old one can wait a cycle at no cost. Sorting by score cannot express a deadline.
2. **Effort is a shared, renewable-per-period resource.** Spending it on the #1 and #2 roles may be worse than #1 and #7 if #1 and #2 are near-identical.
3. **Marginal value is non-linear across role families.** With four VP Sales applications in flight, a fifth adds less expected value than a first Program Manager application. Ranking has no notion of what is already in the pipeline.

Point 3 is where greedy fails hardest, and it is **exactly this project's founding insight**. The trial that reframed "VP Sales" to "Program Manager" found stronger matches immediately. The board review files this under *unawareness* and treats it as a discovery problem. It is a **portfolio allocation problem**, and modelling it as one is the novel move.

A live scan on 2026-07-29 illustrated this: against the same 44-company sweep, `"Sales Director"`, `"Country Manager"` and `"Fractional Chief Revenue Officer"` each returned **0** jobs, while `"Program Manager"` returned **46** and `"Business Development"` returned **43**. The obvious titles were empty; the adjacent framing was where the roles were.

**Read that as an illustration, not as validation.** It is n=1, and that one profile — a 25-year generalist with transferable function across several industries — is close to the best possible case for adjacency. It demonstrates that the mechanism *can* pay off; it does not establish how often, for whom, or by how much. A licensed specialist run through the same test would likely show the opposite, and that is the expected and correct result rather than a failure. Establishing the real distribution requires the multi-shape trial matrix in [TEST_STRATEGY.md](../TEST_STRATEGY.md).

## Every parameter is derived or user-set. None is a constant.

This layer is where founding-persona assumptions leak in most easily, so they are called out explicitly. **A parameter fitted to one profile shape and hardcoded is a defect, not a default.**

### Effort budget

Not a constant. The number of tailored applications a user can sustain per period varies by an order of magnitude across circumstances: a selective senior candidate making one considered move, a new graduate in a high-volume entry market, someone unemployed and needing income within weeks, someone applying around a full-time job and caring responsibilities.

The budget is **user-set, with a derived starting suggestion** from their own observed throughput (`pipeline` transitions per week) once history exists. Before that, ask; do not assume.

### Two regimes, both legitimate

The literature's tailoring-depth advantage is a *rate*, not a strategy. Expected outcomes are rate × volume, and different circumstances optimise different terms:

| Regime | Objective | Slate shape |
|---|---|---|
| **Selective** | Maximise P(offer) on few, high-fit applications | Small slate, deep tailoring, high fit floor |
| **Throughput** | Maximise P(≥1 offer soon) under time pressure | Larger slate, tailoring depth traded against count |

The solver handles both — it is the *same* min-cost flow with a different source capacity and fit floor. **The tool must not moralise about which regime the user is in.** Someone who needs income in three weeks is not doing it wrong; they have a different objective function, and telling them to make "one good move" would be advice masquerading as optimisation.

### Diversification cap — derived from adjacency, and legitimately 1

The cap must be **derived from the profile's actual adjacency structure**, not assumed. Adjacency availability varies enormously:

- **High adjacency**: generalists with transferable function across industries. Diversification is high-value.
- **Low adjacency**: licensed or credentialed specialists — clinicians, tax attorneys, airline pilots, actuaries — where "adjacent" roles are often a *downgrade* or legally unavailable.
- **Narrow-by-stage**: early-career candidates with one demonstrated skill, where breadth reads as unfocused rather than versatile.

**A cap of 1 (no diversification) is a valid solution and must be reachable.** Pushing a specialist toward adjacent families would be actively harmful, and pushing an early-career candidate to spread thin works against them.

Derive adjacency from the role-archetype inference already in the pipeline: if inferred archetypes cluster tightly, adjacency is low and the cap tightens automatically. Do not hardcode a spread.

## Formulation

Min-cost max-flow. Not maximum bipartite matching — with a single candidate that is degenerate.

```
                  [cap = weekly effort budget, cost 0]
        source ───────────────────────────────▶ user
                                                 │  │
                    [cap = diversification cap]  │  └──[slack: cap = budget, cost = τ]──┐
                                                 ▼                                       │
                                          role archetype                                 │
                                                 │                                       │
              [cap = 1, cost = −log( P(callback) · decay(age) )]                         │
                                                 ▼                                       │
                                             posting                                     │
                                                 │                                       │
                                          [cap = 1, cost 0]                              │
                                                 ▼                                       ▼
                                                sink ◀────────────────────────────────────
```

### ⚠️ The cost function is `−log( P · decay )`, **not** `−log P × decay`

This is worth stating explicitly because the wrong form is the natural thing to write and it inverts the whole model.

With `decay ∈ (0,1]` shrinking as a posting ages, **multiplying** by decay *reduces* the cost of stale postings, and a min-cost solver therefore **prefers** them:

| Posting | P(callback) | decay | `−log P × decay` ✗ | `−log(P · decay)` ✓ |
|---|---|---|---|---|
| fresh, 1 day | 0.10 | 1.0 | 2.303 | 2.303 |
| stale, 34 days | 0.10 | 0.3 | **0.691 → preferred** | **3.507 → penalised** |

Only the additive form (`−log P − log decay`) makes cost *increase* with age, which is what the <7-day review-bandwidth finding requires. Implement `−log(P · decay)` or equivalently `−log P − log decay`.

### The slack edge is load-bearing — and it must attach to `user`, not `source`

Min-cost **max**-flow maximises flow *value* first, then minimises cost among the flows achieving it. Without slack it will always spend the entire budget, even on postings not worth applying to — contradicting [ADR-008](../DECISIONS.md), which requires a slate of 1, or 0, to be reachable.

**The attachment point is not a detail.** Placing slack on `source → sink` breaks it, and the failure is silent:

| Placement | Source cut | Max flow | Behaviour |
|---|---|---|---|
| `source → sink` ✗ | `B` (to user) + `B` (to slack) = **2B** | 2B | The solver *must* achieve 2B, so it pushes B through postings **and** B through slack. The posting path saturates anyway — every posting is selected regardless of cost, and the slack accomplishes nothing |
| `user → sink` ✓ | **B** | B | The solver distributes B units between posting paths and slack, choosing whichever is cheaper per unit |

Worked example — budget `B = 5`, `τ = 2.996` (`P_min = 0.05`), posting costs `[1.2, 1.8, 4.5, 5.0, 6.1]`:

- **Correct (`user → sink`):** postings at 1.2 and 1.8 are below `τ`; the remaining 3 units flow through slack. Total cost `1.2 + 1.8 + 3(2.996) = 11.99`. Forcing all five postings would cost `18.6`, so the solver correctly prefers to leave budget unspent. **Slate of 2.**
- **Wrong (`source → sink`):** max flow is 10, forcing all 5 postings including the one at 6.1, plus 5 slack units. **Slate of 5, always.**

`τ` is the **reservation cost** — the price of *not* using a unit of budget. It is where the relevance floor lives, expressed inside the graph rather than as a pre-filter, and it must be user-visible and tunable: it is the knob that says "don't put things on my slate that aren't worth it."

Set `τ = −log(P_min)` where `P_min` is the lowest callback probability the user considers worth an application.

**Implementer's check:** with every posting cost above `τ`, the solver must return an **empty** slate. If it returns a full one, the slack is attached to the wrong node.

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

## Fully specified construction

Everything a contributor needs, so the data structure and the details are not chosen by coin flip.

| Element | Value |
|---|---|
| `source → user` | cap = effort budget (derived or asked); cost 0 |
| **`user → sink` (slack)** | cap = effort budget; cost = `τ` = `−log(P_min)`. **Must attach to `user`, not `source`** — see above |
| `user → archetype` | cap = diversification cap for that family; cost 0 |
| `archetype → posting` | cap 1; cost = `−log( P(callback) · decay(age) )` |
| `posting → sink` | cap 1; cost 0 |

**Degenerate inputs.** `P = 0` or `decay = 0` gives infinite cost. Clamp both to a small floor (suggest `P_floor = 1e-6`) before taking the log, or exclude the posting from the graph entirely — do not let an infinity or a `NaN` reach the solver. Postings past `validThrough` are excluded rather than decayed to zero, so in practice the decay floor should rarely bind.

**Posting-to-archetype multiplicity.** A posting may plausibly belong to several role families. Assign each posting to **exactly one** archetype — its highest-scoring — otherwise the diversification cap is unenforceable, since flow could reach one posting through several archetype edges and evade the cap. Record the alternates for display; do not add parallel edges.

**Cost quantisation.** Successive shortest paths wants integer costs. Scale by a fixed factor (suggest 10⁴) and round: `cost_int = round(−log(P · decay) × 10_000)`. Document the factor — it bounds the precision of every allocation decision. Ties break on `(posting.id)` ascending so the solver is deterministic, which acceptance criterion 1 requires.

**Decay functional form.** Exponential with a documented half-life: `decay(age_days) = 0.5^(age_days / h)`. `h` is derived from observed data once Layer 2 has it, and is a documented parameter until then — not a hardcoded constant ([ADR-008](../DECISIONS.md)). Postings past `validThrough` (#149) are excluded from the graph entirely rather than decayed.

**Representation.** Vector-backed with `usize` indices into a `Vec<Edge>`, not `Rc<RefCell<Node>>`. This is a coding convention rather than a scale-driven optimisation — it costs nothing, avoids fighting the borrow checker in a ~200-line solver, and is the one piece of the graph-engineering research that applies at any scale. The graph is rebuilt per solve; there is no mutation hot loop, so none of the arena/CSR/generational-index machinery is warranted ([REJ-004](../DECISIONS.md)).

## Solver invariants — test these before trusting the construction

**Two flow-network errors have already shipped in this document**, both syntactically plausible and both silently inverting the model:

1. `cost = −log P × decay` instead of `−log( P · decay )` — made the solver prefer *stale* postings.
2. Slack attached to `source` instead of `user` — made the slate unable to under-fill, so every posting was selected regardless of cost.

Neither was caught by reading. Both are caught immediately by a numeric test. **Write these as unit tests first, before the solver.** If a future change to this document contradicts one of them, the test is right and the document is wrong.

| # | Invariant | Why it catches a real error |
|---|---|---|
| 1 | Two postings identical except age → the **younger** is selected | Catches the cost-function inversion |
| 2 | All posting costs above `τ` → slate is **empty** | Catches slack misattachment |
| 3 | All posting costs below `τ`, more postings than budget → slate size **equals budget** | Catches slack starving the posting path |
| 4 | Postings in one family exceeding its cap → selection **stops at the cap**, remainder goes to other families or slack | Catches unenforced diversification |
| 5 | Same input twice → **byte-identical** slate | Catches non-deterministic tie-breaking |
| 6 | `P = 0` or `decay = 0` present → solver **completes**, no `inf`/`NaN` | Catches the degenerate-input path |
| 7 | Selected total cost ≤ cost of any other feasible selection of the same size | Catches a wrong-direction objective generally |
| 8 | Budget of 1 → slate of exactly 0 or 1 | Catches off-by-one in source capacity |

Invariant 7 is the general one: if the solver is minimising the wrong thing, it fails even when 1–6 pass.

## ⚠️ Open conflict with #167 — resolve before implementing

#167 requires that budget consumption be **effort-weighted** — a one-click apply and a 25-minute Workday form should not draw equally on the budget. That is correct product reasoning, and it is **not expressible in this formulation**: a flow network consumes exactly one unit of source capacity per unit of flow. Heterogeneous per-item consumption is a knapsack constraint, which turns a polynomial problem NP-hard and invalidates the "<100 ms for 5,000 postings" criterion.

Three ways out, in preference order:

1. **Quantise effort into integer units** and require `k` units per posting by giving each posting `k` parallel unit-capacity edges. Stays a flow problem, stays polynomial. Loses granularity, which is acceptable — effort estimates are coarse anyway.
2. **Lagrangian relaxation** — fold effort into the cost term with a multiplier tuned so expected total effort meets the budget. Approximate, still polynomial.
3. **Reformulate as an LP** with explicit bound constraints. Expressible and exact; heavier dependency, and worth noting the Flux Balance Analysis material in the graph-engineering research is exactly this shape — an LP over bounded flows — which is where that analogy earns its keep rather than being decorative.

**Do not implement #152 and #167 independently and assume they compose.** They do not, as currently written.

## Implementation

Successive shortest paths with potentials (Bellman-Ford init, then Dijkstra). Roughly 200 lines, or `petgraph` — note it is **not currently a dependency**, so adding it is a real decision, not a free one.

**Why SSP:** costs are all non-negative (`−log` of a probability ≤ 1), the graph is small and layered, and SSP with potentials is the simplest correct choice at this scale. Network simplex and cost-scaling are faster asymptotically and not worth the complexity for a few thousand nodes. Since costs are non-negative, **Bellman-Ford initialisation is unnecessary** — potentials start at zero and Dijkstra suffices from the first iteration.

**Not Hopcroft-Karp, not Ford-Fulkerson.** Hopcroft-Karp solves maximum-cardinality bipartite matching, which is degenerate here (one candidate) and cannot express costs at all. Ford-Fulkerson solves max-flow without costs. Neither expresses the objective; both are recorded here so the choice is not relitigated.

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
