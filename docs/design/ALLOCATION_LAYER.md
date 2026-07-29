# Layer 3 — Allocation: min-cost flow slate generation

**Status:** Design accepted, unbuilt · **ADR:** [ADR-006](../DECISIONS.md#adr-006--the-output-is-an-allocated-slate-not-a-ranked-list)
**Depends on:** Step 0, Layer 1, Layer 2

## Purpose

Turn the opportunity set into a **decision**: which applications to make this week, in what order, by when — and which to skip, with a reason.

This is the inflection. Everything below it is infrastructure for this output.

## Why a solver at all — and an honest bound on the claim

The user can only make a finite number of genuinely-tailored applications per period. The question is which subset.

**Be precise about what this buys, because an earlier draft of this section overclaimed.** Under the constraint set as specified — a budget `B`, a per-family cap, and one application per posting — the feasible sets form a *truncated partition matroid* and the objective is *modular* (a sum of independent per-posting terms). Max-weight independent set in a matroid is solved **exactly by greedy**: sort by weight, take if feasible. So for the constraints listed here, **sorting by `P·decay` and taking greedily subject to the family caps — stopping once `P·decay ≤ P_min` — is optimal**, and a flow solver is machinery for a problem a sort already solves.

Three claims an earlier draft made for the solver, and what is actually true:

| Claim | Reality |
|---|---|
| "Sorting cannot express a deadline" | The model is **single-period**. Decay enters as a re-weighting of the score, so sorting by `P·decay` expresses exactly as much deadline as the flow does. A genuine "this can wait a cycle" argument needs a multi-period formulation that does not exist here |
| "Near-identical postings are redundant" | Redundancy is a **submodular** effect. This objective is modular; nothing penalises redundancy except the family cap, which is coarse |
| "Marginal value diminishes across families" | A capacity constraint is a **box constraint, not a concave utility**. The fifth application in a family has undiminished marginal value right up until the cap truncates it to zero |

**So why keep the flow formulation?** Because the constraint set is not going to stay a matroid:

- **#167's effort weighting** makes budget consumption heterogeneous, which breaks the matroid immediately.
- ~~Posting liveness (#177) and multi-period scheduling add structure greedy cannot express.~~ **This was also wrong.** Liveness *removes* postings from the ground set, and a restriction of a matroid is still a matroid. Unit-time jobs with deadlines and per-slot capacity form a scheduling matroid. Greedy stays optimal for both.

**So exactly one justification survives: effort weighting.** If #167 is descoped, there is no remaining reason to build a flow solver.

That single reason is genuine — a knapsack constraint is not a matroid, and retrofitting it onto a sort means rewriting the layer. *"Greedy is provably suboptimal"* was not, and should not be repeated. **If effort weighting is descoped, greedy is the correct implementation and this layer should be simplified accordingly.**

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

⚠️ **These two rows do not currently state different objectives, and the claim below is wrong.** "Maximise P(offer)" on a slate *is* `P(≥1 offer)` — an offer from any member is an offer. And switching from `E[#callbacks]` to `P(≥1)` is a change of **cost function** (`1−p` → `log(1−p)`), not of capacity and floor — with the added problem that `log(1−p)` is negative and the construction forbids negative costs.

What the solver genuinely handles with capacity and floor alone is **the same objective at different scales**: a larger budget and a lower `P_min` for the throughput regime. That is still a real and useful distinction, and it is what the tool must not moralise about. Whether `P(≥1 offer)` is worth supporting as a separate objective is **open**, and would require reworking the cost sign convention.

**The tool must not moralise about which regime the user is in.** Someone who needs income in three weeks is not doing it wrong; they have a different objective function, and telling them to make "one good move" would be advice masquerading as optimisation.

### Diversification cap — derived from adjacency; *no* diversification is `cap = B`

The cap must be **derived from the profile's actual adjacency structure**, not assumed. Adjacency availability varies enormously:

- **High adjacency**: generalists with transferable function across industries. Diversification is high-value.
- **Low adjacency**: licensed or credentialed specialists — clinicians, tax attorneys, airline pilots, actuaries — where "adjacent" roles are often a *downgrade* or legally unavailable.
- **Narrow-by-stage**: early-career candidates with one demonstrated skill, where breadth reads as unfocused rather than versatile.

> ### ⚠️ Read the direction carefully — this was documented backwards
>
> The cap is a **per-family maximum** on `user → archetype`. So:
>
> | Cap value | Effect |
> |---|---|
> | `cap = 1` | At most **one** application per family → a budget of 7 is forced across **7 distinct families** → **maximum diversification** |
> | `cap ≥ B` | Unbinding → all `B` applications may sit in **one** family → **no diversification** |
>
> Earlier drafts said *"a cap of 1 (no diversification)"*, which is exactly inverted, and prescribed *"if archetypes cluster tightly, the cap tightens"* — also inverted. A specialist with one viable family and `cap = 1` gets a **slate of 1** and the rest of the budget dumped into slack, which is precisely the harm the paragraph was trying to prevent.

**"No diversification" is `cap = B`, not `cap = 1`.** That value must be reachable, because pushing a specialist toward adjacent families is actively harmful and pushing an early-career candidate to spread thin works against them.

Derive the cap from the role-archetype inference already in the pipeline, in the correct direction: **tightly clustered archetypes mean low adjacency, so the cap should loosen toward `B`** (concentrate). Widely spread archetypes mean high adjacency, so the cap can tighten (explore). Do not hardcode either end.

**Both ends need a guard, not just the specialist end.** A feasibility floor is required: **`cap ≥ ⌈B / F⌉`** where `F` is the number of families holding postings above the floor. Without it the generalist end silently under-fills — 6 viable archetypes with `cap = 1` and `B = 7` caps flow at 6 and dumps a budget unit into slack even when an excellent 7th posting exists in an already-used family. That is the same harm as the specialist case, on the other boundary.

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
              [cap = 1, cost = 1 − P(callback)·decay(age) ]                              │
                                                 ▼                                       │
                                             posting                                     │
                                                 │                                       │
                                          [cap = 1, cost 0]                              │
                                                 ▼                                       ▼
                                                sink ◀────────────────────────────────────
```

### ⚠️ The cost is `1 − P·decay`, and this replaces an earlier `−log(P·decay)`

Two successive drafts got this wrong. The history is worth keeping because both errors are easy to repeat.

**Draft 1 wrote `−log P × decay`**, which inverts the model: with `decay ∈ (0,1]` shrinking as a posting ages, *multiplying* reduces the cost of stale postings and a min-cost solver therefore prefers them. At `P = 0.10`, a 34-day posting scored 0.691 against a fresh one at 2.303 — the solver would have picked the stale one.

**Draft 2 wrote `−log(P·decay)`**, which fixes the age direction but optimises the wrong quantity. Minimising `Σ −log pᵢ` maximises `Π pᵢ` — the probability that **every** application succeeds. Nobody wants that. Expected callbacks is `Σ pᵢ`, a different functional. "Log-additive" was the tell: log-additivity gives the product.

| Objective | Correct edge cost |
|---|---|
| `E[#callbacks] = Σ pᵢ` ← **what we want** | `1 − pᵢ` (or any `C − pᵢ`) |
| `P(≥1 callback)` | `log(1 − pᵢ)` — ⚠️ **negative for all `p > 0`**, so it violates the `cost ∈ [0,1]` assertion and voids the no-Bellman-Ford argument. Do not adopt without reworking both |
| `P(all succeed)` | `−log pᵢ` ← draft 2 |

**Use `cost = 1 − P(callback)·decay(age)`.** Since `P·decay ∈ [0,1]`, the cost is in `[0,1]`: non-negative (so Dijkstra needs no Bellman-Ford init), bounded (no infinities), and minimising the total maximises `Σ Pᵢ·decayᵢ`, which **is** expected callbacks and is directly readable off the solution.

Age direction still holds: fresh `1 − 0.10(1.0) = 0.90` beats stale `1 − 0.10(0.3) = 0.97`.

> **Be precise about what this change does and does not do.** Under the *current* constraint set it changes **no slate at all**. Both costs are `Σ f(p)` with `f` strictly decreasing, so the inclusion test and the greedy order are identical — and over a matroid, identical order plus identical threshold gives the identical selected set. A brute-force sweep over 4,000 random instances found **zero** differing slates.
>
> What it actually fixes: the **reported objective**, which was in nats and is now in callbacks; the `P_floor`/`τ` inversion; and the behaviour once #167's effort weighting lands, where magnitudes rather than order begin to matter. **Do not assume the slate semantics moved — they did not.**

**This also removes a hole the log form had.** With `τ = −log(P_min)` and a `P_floor = 1e-6` clamp, any `P_min < 1e-6` gave `τ > 13.8`, at which point clamped **zero-probability postings became cheaper than slack and were selected**. In the linear form `τ = 1 − P_min` and both sit in `[0,1]`, so the ordering cannot invert.

### The slack edge is load-bearing — and it must attach to `user`, not `source`

Min-cost **max**-flow maximises flow *value* first, then minimises cost among the flows achieving it. Without slack it will always spend the entire budget, even on postings not worth applying to — contradicting [ADR-008](../DECISIONS.md), which requires a slate of 1, or 0, to be reachable.

**The attachment point is not a detail.** Placing slack on `source → sink` breaks it, and the failure is silent:

| Placement | Source cut | Max flow | Behaviour |
|---|---|---|---|
| `source → sink` ✗ | `B` (to user) + `B` (to slack) = **2B** | 2B | The solver *must* achieve 2B, so it pushes B through postings **and** B through slack. The posting path saturates anyway — every posting is selected regardless of cost, and the slack accomplishes nothing |
| `user → sink` ✓ | **B** | B | The solver distributes B units between posting paths and slack, choosing whichever is cheaper per unit |

Worked example — budget `B = 5`, `P_min = 0.08` so `τ = 0.92`, with `P·decay` values `[0.30, 0.25, 0.05, 0.02, 0.01]` giving costs `[0.70, 0.75, 0.95, 0.98, 0.99]`:

- **Correct (`user → sink`):** the two costs below `τ` are selected; the remaining 3 units flow through slack. Total `0.70 + 0.75 + 3(0.92) = 4.21`, against `4.37` for forcing all five — so the solver correctly prefers to leave budget unspent. **Slate of 2**, expected callbacks `0.30 + 0.25 = 0.55`.
- **Wrong (`source → sink`):** max flow is `2B`, forcing all 5 postings plus 5 slack units. **Slate of 5, always.**

`τ` is the **reservation cost** — the price of *not* using a unit of budget. It is where the relevance floor lives, expressed inside the graph rather than as a pre-filter, and it must be user-visible and tunable: it is the knob that says "don't put things on my slate that aren't worth it."

Set **`τ = 1 − P_min`** where `P_min` is the lowest callback probability the user considers worth an application. A posting is selected exactly when `1 − P·decay < 1 − P_min`, i.e. when `P·decay > P_min` — **strictly greater**.

**Ties must break toward slack.** At `P_min = 0` a posting with `P·decay = 0` costs exactly `1`, which equals `τ`. Min-cost flow is indifferent at a tie, and the posting-id tie-break would resolve it toward the *postings* — producing a full slate of zero-probability applications. Invariant 2 as originally worded ("costs **above** `τ`") passes vacuously in that case, because `1` is not above `1`.

Implement the slack edge as **strictly preferred at equal cost**, or equivalently set `τ = 1 − P_min − ε` for a small `ε`. `P_min = 0` remains reachable, satisfying ADR-008's requirement that a fit floor of 0 be expressible — it just must not select worthless postings.

**Implementer's check:** with every posting cost above `τ`, the solver must return an **empty** slate. If it returns a full one, the slack is attached to the wrong node.

Each element earns its place:

- **Source capacity** = the honest weekly tailoring budget. Encodes the constraint mass-apply tools pretend away.
- **Role-archetype capacity** = diversification cap. This is what forces exploration of adjacent families rather than piling into one.
- **Edge cost** = `1 − P(callback)·decay` from [Layer 2](CALIBRATION_LAYER.md). Minimising the total maximises `Σ P·decay`, which is expected callbacks by linearity of expectation — no independence assumption required.
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
| **`user → sink` (slack)** | cap = effort budget; cost = `τ` = `1 − P_min`. **Must attach to `user`, not `source`** — see above |
| `user → archetype` | cap = diversification cap for that family; cost 0 |
| `archetype → posting` | cap 1; cost = `1 − P(callback)·decay(age)` |
| `posting → sink` | cap 1; cost 0 |

**Degenerate inputs.** The linear cost removes most of this class — `1 − P·decay` cannot be infinite or `NaN` for finite inputs, so there is nothing to floor from below. Two guards remain:

- **Clamp the `decay` OUTPUT, not the `age` input: `decay = min(decay, 1)`.** Clamping `age < 0` to zero is *not sufficient* — it closes only one of three paths to `decay > 1`:
  | Path | Example | Caught by an age clamp? |
  |---|---|---|
  | Future-dated `datePosted` | `age = −30, h = 7` → `decay = 19.5` | yes |
  | **Negative half-life** | `age = 8, h = −7` → `decay = 2.21` | **no** — `age` is positive |
  | **Zero half-life** | `age = 0, h = 0` → `0.5^(0/0)` = **NaN** | **no** |

  `h` is a *fitted* parameter (derived once Layer 2 has data), so a bad fit reaching the solver is a live path, not a hypothetical. **Validate `h > 0` and finite at the point it is set**, and clamp `decay` itself. Worked case: `h = 7`, `age = −30`, `P = 0.10` → `decay = 19.5`, `P·decay = 1.95`, `cost = −0.95`.
- **Clamp `P ≤ 1`.** A miscalibrated posterior should never exceed 1, but clamp rather than trust it — same negative-cost consequence.

**Assert `cost ∈ [0,1]` before it enters the solver.** That single assertion catches both, and would have caught the negative-cost path immediately.

Postings past `validThrough` are excluded rather than decayed, so the lower floor should rarely bind.

**Posting-to-archetype multiplicity.** A posting may plausibly belong to several role families. Assign each posting to **exactly one** archetype — its highest-scoring — otherwise the diversification cap is unenforceable, since flow could reach one posting through several archetype edges and evade the cap. Record the alternates for display; do not add parallel edges.

**Cost quantisation.** Successive shortest paths wants integer costs. Scale by a fixed factor (suggest 10⁴) and round: `cost_int = round((1 − P·decay) × 10_000)`, giving integers in `[0, 10_000]`. Document the factor — it bounds the precision of every allocation decision. Ties break on `(posting.id)` ascending so the solver is deterministic, which acceptance criterion 1 requires.

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
| 2 | All posting costs **at or above** `τ` → slate is **empty** | Catches slack misattachment. "At or above" matters: at `P_min = 0` a worthless posting costs exactly `τ`, and a strict-inequality test passes vacuously |
| 3 | All posting costs below `τ`, more postings than budget → slate size **equals budget** | Catches slack starving the posting path |
| 4 | Postings in one family exceeding its cap → selection **stops at the cap**, remainder goes to other families or slack | Catches unenforced diversification |
| 5 | Same input twice → **byte-identical** slate | Catches non-deterministic tie-breaking |
| 6 | `decay > 1` (future-dated posting) present → all costs remain in `[0,1]`, solver completes | Catches the negative-cost path, which breaks Dijkstra **silently** |
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

Successive shortest paths with potentials. Roughly 200 lines, or `petgraph` — note it is **not currently a dependency**, so adding it is a real decision, not a free one.

**Complexity, derived rather than asserted.** With 5,000 postings and ~30 archetypes:

```
V = 1 source + 1 user + 30 archetypes + 5000 postings + 1 sink  ≈ 5,033
E = 1 + 1 (slack) + 30 + 5000 + 5000                            ≈ 10,032
```

SSP performs one Dijkstra per unit of flow, and flow value is bounded by the budget `B` (≈7), **not** by the number of postings. So the work is `B · O(E log V)` ≈ `7 × 10,032 × 12.3` ≈ **0.9 M operations**, roughly **9 ms** in release Rust — about 11× inside the 100 ms acceptance criterion.

This is why the posting count barely matters: postings add edges, but augmentations are capped by the budget. Even the effort-quantisation resolution to the #167 conflict, which multiplies edges by the max effort units per posting (say 4), lands near 35 ms and stays polynomial.

**Why SSP:** costs are all non-negative and bounded (`1 − P·decay ∈ [0,1]`), the graph is small and layered, and SSP with potentials is the simplest correct choice at this scale. Network simplex and cost-scaling are faster asymptotically and not worth the complexity for a few thousand nodes. Since costs are non-negative, **Bellman-Ford initialisation is unnecessary** — potentials start at zero and Dijkstra suffices from the first iteration.

**Not Hopcroft-Karp, not Ford-Fulkerson.** Hopcroft-Karp solves maximum-cardinality bipartite matching, which is degenerate here (one candidate) and cannot express costs at all. Ford-Fulkerson solves max-flow without costs. Neither expresses the objective; both are recorded here so the choice is not relitigated.

Scale: a few thousand postings, tens of role archetypes, one user. **Milliseconds, single-threaded.** No arena allocation, no CSR, no lock-free structures, no epoch reclamation — see [REJ-004](../DECISIONS.md#rej-004--arena-allocation-generational-indices-csr-epoch-based-reclamation). The graph is rebuilt per solve; there is no mutation hot loop to optimise.

The hardware floor is untouched: this adds no model, no index, and no persistent structure.

## Counterfactual re-solve

Falls out for free, and closes #122 far more credibly than the LLM-prompt approach currently specced.

**Diff the expected outcome, NOT the raw objective.** The objective is

```
cost_total = B − Σ_selected P·decay − (B − |S|)·P_min
```

which contains `B`. So for a **budget** counterfactual the objective *grows* even as the outcome improves: budget 7 with `ΣP·decay = 1.2` gives `7 − 1.2 = 5.80`; budget 10 with `1.6` gives `10 − 1.6 = 8.40`. Diffing the objective reports **+2.60 — that raising the budget made things worse**, which is backwards, and budget change is a required counterfactual.

Extract **`Σ_selected P·decay`** from each solve and diff *that*. It is the expected-callback total and is comparable across solves with different budgets.

Re-solve with one constraint relaxed, then diff:

```
Current slate:                    1.2 expected callbacks
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

Expected outcome: 0.3 – 1.3 callbacks  (n=12, prior-dominated)
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
