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

> **Executable reference: `src/engine/calibration.rs`.** The conjugate update, the shrinkage identity, the `prior_dominated` threshold and the equal-tailed interval are all implemented and tested there. `interval_level_of` computes the true credibility level of any stated interval — which is how the two wrong examples in this document were found after surviving several reviews.


> **Terminology correction (2026-07-29).** Earlier drafts, and several issue titles, call this *"hierarchical empirical-Bayes"*. **Both words were wrong** and a statistically literate contributor would have implemented the wrong thing.
>
> - **Empirical Bayes** estimates the prior hyperparameters *from the observed data*, usually by pooling across many units. Here they come from published literature and are fixed. That is **Bayesian inference with an informative prior** — a different method.
> - **Hierarchical** means buckets share a hyperprior so a sparse bucket borrows strength from related ones. As specified below they are independent, so the model is **not hierarchical** either.
>
> The pooling described in *Optional: partial pooling* below is the thing that would make it genuinely hierarchical, and it is worth doing — but it is an extension, not what the base model is. Where issue titles still say "empirical-Bayes", read "Bayesian with an informative prior".

Conjugate Beta-Binomial per outcome transition and feature bucket. For transition *t* (applied→callback, callback→interview, interview→offer) and bucket *b*:

```
prior:      θ_{t,b} ~ Beta(α_{t,b}, β_{t,b})
observed:   k successes out of n trials, locally
posterior:  θ_{t,b} | data ~ Beta(α_{t,b} + k,  β_{t,b} + n − k)
```

Priors come from published funnel research, encoded as a static table. **That table does not exist yet and is specified by #176** — it is *not* issue #119, which is a salary dataset and unrelated. Earlier drafts of this doc pointed at #119; that pointer was wrong. **This is the only legitimate use of the published benchmarks** — they are prior parameters, never figures shown to the user as guidance.

Prior strength is `α + β`, in units of effective prior observations. Set `α = p̄(α+β)` and `β = (1 − p̄)(α+β)` where `p̄` is the published rate for that bucket, so the prior mean is exactly `p̄`.

**`α+β` is a tunable parameter, not a constant** ([ADR-008](../DECISIONS.md)). A value near 20 makes personal signal overtake the prior within a realistic search while stopping four applications from driving the number; the value must be documented and adjustable, and #176 owns deriving it rather than asserting it.

### Shrinkage is mandatory

This is not an extra step — it falls out of the conjugate update. The posterior mean is a precision-weighted average of the prior mean and the observed rate:

```
E[θ | data] = (α + k) / (α + β + n)
            = w · p̄  +  (1 − w) · (k/n),     where w = (α+β) / (α+β+n)
```

So `w` is the share of the answer still coming from the prior. With `α+β = 20` and `n = 12`, `w = 0.63` — **most of that number is still the literature, not the user.**

That gives a principled definition rather than an arbitrary threshold:

> **`prior_dominated` is true when `w > 0.5`, i.e. when `n < α + β`.**

Use that, not a separately configurable floor — a hand-picked floor would be exactly the asserted constant ADR-008 bans, and this one is derived from the model.

### Interval method — specify it, do not leave it to the implementer

Report the **equal-tailed 90% credible interval**: the 5th and 95th percentiles of the posterior Beta, via the inverse regularised incomplete beta function.

Equal-tailed rather than highest-density: for small `k` the posterior is strongly skewed and the two differ materially, so leaving the choice open makes two correct implementations disagree. Equal-tailed is also cheap and monotone in the data, which the invariant tests rely on.

The model reports that interval, and the CLI/TUI must render it — never a bare point estimate.

```
Callback rate (deeply tailored, <7d):  3.2% – 20.2%   (n=12,  prior-dominated)
Callback rate (deeply tailored, <7d):  7.6% – 15.8%   (n=140, personal)
```

> **These figures are computed, not hand-written**, by `engine::calibration` at `p̄ = 0.115`, `α+β = 20`. An earlier draft printed `4%–19%` and `9%–13%` and labelled both "equal-tailed 90%". They were actually **84%** and **58%** intervals — wrong, wrong by different amounts, and undetectable by reading. `printed_examples_match_their_claimed_level` is now the regression test; **any interval added to these docs must pass it before it is printed.**
>
> Note what the honest numbers show: at n=140 the interval is still **8.2 points wide**, not the 4 the old example implied. Reaching 4 points at this rate takes roughly **600** observations. That is a real constraint on how quickly the tool can claim to know something personal, and the previous example concealed it.

This is the project's honest-failure value expressed statistically. It is also what makes the number trustworthy enough for Layer 3 to optimise against.


### Optional: partial pooling (this is what "hierarchical" would mean)

Buckets are related — deep/light/generic tailoring are points on one axis. A genuinely hierarchical model puts a hyperprior over the bucket-level `θ` so a sparse bucket borrows strength from its neighbours, which materially helps early in a search when every bucket is thin.

Worth doing, but it is **not** the base model and must not be described as though it is. Defer until the base model has real data; revisit under #150.

### Controllable / structural decomposition

**This is a safety requirement, not a modelling nicety.**

Observed outcomes are decomposed into two groups, and only one may drive recommendations:

| Controllable — drives recommendations | Structural — attributed, never actioned |
|---|---|
| Tailoring depth (`edit_distance`) | Name / ethnicity-signalled screening bias |
| Submission latency (days after `posted_at`) | Age (both directions) |
| Role-fit score | Career gaps — caregiving, medical, military transition, incarceration |
| Role-family / segment choice | Prior self-employment or gig-economy history |
| Application volume | Work-authorisation and visa status |
| Which sources are used | Disability, and disability disclosure effects |
| | Gender, and gender-signalled role segregation |
| | Non-native language or accent signals |
| | Education-institution prestige and credential-country recognition |
| | Market tightness for the segment and region |
| | Duration dependence (demand-side) |

**The structural list must never be tuned to whoever is testing.** The five factors this table opened with were drawn from one profile's circumstances, which is precisely the failure mode this section exists to prevent. Any user's dominant structural factor is likely to be absent from a list assembled around someone else: for a returning parent it is the caregiving gap, for a candidate over 55 it is age, for a recent migrant it is credential recognition and work authorisation, for a disabled candidate it is disclosure effects. Contributors adding a factor should add it to the *structural* column by default and require evidence before treating anything as controllable.

A naive model fitted to raw outcomes learns *"you are a weak candidate"* when the actual signal is documented market discrimination. Correspondence studies put minority-named callbacks at 25–50% of baseline; comparable field-experiment penalties are documented across most rows above.

The tool must therefore say:

> Your callback rate is below the population baseline. Field experiments attribute a large share of this gap to screening effects on [factor] rather than to candidate quality. Of the factors you control, submission latency is where your own data shows the largest effect.

Not:

> Your callback rate is 1.2%. Consider lowering your target seniority.

**Attribution requires evidence, and silence is the honest default.** Do not infer a protected characteristic in order to attribute an effect to it. Attribute only from what the user has explicitly told the tool or what is unambiguously present in the profile they supplied. Where the model detects an unexplained gap it cannot attribute, it says exactly that — an unexplained shortfall is a more honest output than a guessed cause, and guessing here means inferring ethnicity or disability from a name or a gap.

Per [REJ-008](../DECISIONS.md#rej-008--acting-on-structural-bias-data-as-advice), structural factors enter as attribution only. Any feature suggesting name anglicisation or gap concealment is permanently out of scope regardless of measured efficacy.

## Feature extraction

**Tailoring depth.** Normalised edit distance between the generated document and the base resume, already computed at `feedback.rs:14`. Bucket into generic / light / deep using quantiles of the user's own distribution rather than fixed thresholds — a fixed cut would be another asserted constant.

**Submission latency.** `pipeline.updated_at` at first `Applied` transition minus `jobs.posted_at`. Null when `posted_at` is `None` — these rows are excluded from the latency model rather than imputed.

**Role fit.** The existing `overall_score`. Bucketed by quantile.

Issue #132 proposed fixed match weights (exact 1.0 / adjacent 0.7 / transferable 0.4). Keep the **taxonomy** as a feature extractor; delete the **constants**. Those weights are exactly the class of asserted magic number **ADR-008** bans (not ADR-002, which is about missing data — an earlier draft cited the wrong rule) — they must be fitted from outcomes, with intervals, or not claimed.

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
- **#119** — **not** the prior table. It is a salary dataset, superseded by Layer 1. The conversion prior table is **#176**.
- **#132** — taxonomy retained, constants deleted.

## Acceptance criteria

1. Posterior intervals, never bare point estimates, in every surface.
2. `prior_dominated` rendered wherever an estimate appears.
3. Controllable and structural factors reported in separate sections with distinct language; structural factors never appear in a recommendation.
4. Rows with `posted_at = None` excluded from the latency model, not imputed.
5. Model fits from data already in SQLite with no new collection and no network call.
6. A user with zero outcome history gets published priors, clearly labelled as such, and the tool says so.
