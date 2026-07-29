# Layer 2 — Calibration: the per-user conversion model

**Status:** Design accepted, unbuilt · **ADR:** [ADR-005](../DECISIONS.md#adr-005--conversion-rates-are-per-user-posteriors-not-displayed-benchmarks)
**Depends on:** Step 0, Layer 1 · **Blocks:** Layer 3

## Purpose

Estimate, for this user, the probability that an application converts — and report it honestly, including when there is not enough data to say.

This is the layer no competitor can build. It requires longitudinal per-user outcome data across applications, callbacks, and rejections. A cloud product asking users to upload their complete rejection history is making a trust request most users will decline. A local-first binary already holds the data and never transmits it. **The privacy architecture is what makes this feature exclusive, not a constraint on it.**

## The substrate already exists

Every input is already captured. None is aggregated. This layer is the missing analytical tier, not new instrumentation.

| Model input | Already stored at | Computed today? |
|---|---|---|
| Tailoring depth | `feedback.rs:14` `edit_distance` | No |
| Submission latency | `jobs.posted_at` → `pipeline.updated_at` at `Applied` | No |
| Callback / interview / offer | `PipelineStatus`, `outcomes.rs` IMAP ingestion | Recorded, never aggregated |
| Role-fit score | `scorer.rs` 6-dimension evaluation | Yes, never correlated with outcome |
| Segment / role family | #133 classifier output | Planned |

Submission latency is blocked on the Step 0 `posted_at` fix — with fabricated dates the feature is not merely inaccurate, it is inverted, because fabrication clusters at zero days.

## Model

A hierarchical Beta-Binomial per outcome transition. For transition *t* (applied→callback, callback→interview, interview→offer) and feature bucket *b*:

```
θ_{t,b} ~ Beta(α_{t,b}, β_{t,b})
k successes out of n trials observed locally
posterior: Beta(α + k, β + n − k)
```

Priors come from published funnel research, encoded as a static table. **This is the only legitimate use of the published benchmarks** — they are prior parameters, never figures shown to the user as guidance. That table is what remains of issue #119 after Layer 1 supersedes it as a salary source.

Prior strength is deliberately weak (suggest α+β ≈ 20 effective observations) so genuine personal signal overtakes it within a realistic search, but a user with four applications is not handed a number driven entirely by their own noise.

### Shrinkage is mandatory

A user with 12 applications and 1 callback has almost no signal. Reporting "your callback rate is 8.3%" from that is a random number in a confident font.

The model reports the **posterior interval**, and the CLI/TUI must render the interval, never a bare point estimate. Below a configurable observation floor the output says plainly that the estimate is prior-dominated and not yet personal.

```
Callback rate (deeply tailored, <7d):  4% – 19%   (n=12, prior-dominated)
Callback rate (deeply tailored, <7d):  9% – 13%   (n=140, personal)
```

This is the project's honest-failure value expressed statistically. It is also what makes the number trustworthy enough for Layer 3 to optimise against.

### Controllable / structural decomposition

**This is a safety requirement, not a modelling nicety.**

Observed outcomes are decomposed into two groups, and only one may drive recommendations:

| Controllable — drives recommendations | Structural — attributed, never actioned |
|---|---|
| Tailoring depth (`edit_distance`) | Name-based screening bias |
| Submission latency (days after `posted_at`) | Employment-gap filters |
| Role-fit score | Prior self-employment penalty |
| Role-family / segment choice | Market tightness for the segment |
| Application volume | Duration-dependence (demand-side) |

A naive model fitted to raw outcomes learns *"you are a weak candidate"* when the actual signal is documented market discrimination. Correspondence studies put minority-named callbacks at 25–50% of baseline and prior self-employment under 10% — a founder re-entering employed roles, which is precisely this project's founding persona, sits in the worst-penalised bucket in the literature.

The tool must therefore say:

> Your callback rate is below the population baseline. Field experiments attribute a large share of this gap to screening effects on prior self-employment rather than to candidate quality. Of the factors you control, submission latency is where your own data shows the largest effect.

Not:

> Your callback rate is 1.2%. Consider lowering your target seniority.

Per [REJ-008](../DECISIONS.md#rej-008--acting-on-structural-bias-data-as-advice), structural factors enter as attribution only. Any feature suggesting name anglicisation or gap concealment is permanently out of scope regardless of measured efficacy.

## Feature extraction

**Tailoring depth.** Normalised edit distance between the generated document and the base resume, already computed at `feedback.rs:14`. Bucket into generic / light / deep using quantiles of the user's own distribution rather than fixed thresholds — a fixed cut would be another asserted constant.

**Submission latency.** `pipeline.updated_at` at first `Applied` transition minus `jobs.posted_at`. Null when `posted_at` is `None` — these rows are excluded from the latency model rather than imputed.

**Role fit.** The existing `overall_score`. Bucketed by quantile.

Issue #132 proposed fixed match weights (exact 1.0 / adjacent 0.7 / transferable 0.4). Keep the **taxonomy** as a feature extractor; delete the **constants**. Those weights are exactly the class of asserted magic number ADR-002 bans — they must be fitted from outcomes, with intervals, or not claimed.

## Outputs

Consumed by Layer 3 as edge costs, and surfaced to the user directly:

```rust
pub struct ConversionEstimate {
    pub transition: Transition,
    pub posterior: BetaPosterior,
    pub n_observed: usize,
    /// True when the prior still dominates - callers MUST render this.
    pub prior_dominated: bool,
    pub controllable: Vec<FactorEffect>,
    pub structural: Vec<FactorAttribution>,
}
```

`prior_dominated` is not advisory. Any renderer that drops it is a bug.

## Relationship to existing issues

- **#115** (calibrate against outcomes) — the data flow is right; the target changes from ranking distilled models to fitting this model. The community-registry half is dropped.
- **#48** (Proof-of-Quality reputation) — its estimator inputs are literally this feature set. Reframed to a purely local estimator; anonymised vote publishing is dropped (it is an outbound data path, which Step 0 is closing).
- **#119** — survives only as the prior table.
- **#132** — taxonomy retained, constants deleted.

## Acceptance criteria

1. Posterior intervals, never bare point estimates, in every surface.
2. `prior_dominated` rendered wherever an estimate appears.
3. Controllable and structural factors reported in separate sections with distinct language; structural factors never appear in a recommendation.
4. Rows with `posted_at = None` excluded from the latency model, not imputed.
5. Model fits from data already in SQLite with no new collection and no network call.
6. A user with zero outcome history gets published priors, clearly labelled as such, and the tool says so.
