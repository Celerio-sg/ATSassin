//! Layer 2 reference semantics and the executable form of the statistical
//! claims in `docs/design/CALIBRATION_LAYER.md`.
//!
//! **Why this module exists.** The spec's two worked interval examples were
//! wrong for a full day and survived several reviews: they were printed as
//! "the equal-tailed 90% credible interval" while actually being roughly 85%
//! and 58% intervals, and the second implied ~140 observations where the
//! stated width needs ~640. Reading cannot catch that. `interval_level_of`
//! catches it in microseconds, and `printed_examples_match_their_claimed_level`
//! is the regression.
//!
//! The model is a **conjugate Beta-Binomial with an informative prior**. It is
//! *not* empirical Bayes (that estimates hyperparameters from the data rather
//! than taking them from published research) and it is not hierarchical
//! (buckets are independent). Both terms were used incorrectly in earlier
//! drafts, and a statistically literate contributor would have built the
//! wrong thing.

/// A Beta posterior over a conversion rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beta {
    pub alpha: f64,
    pub beta: f64,
}

impl Beta {
    /// Build a prior from a published rate and a strength in effective
    /// observations, so the prior mean is exactly `p_bar`.
    /// Returns `None` unless `strength > 0` and finite. With `strength = 0`
    /// both parameters are zero, `mean()` is `0.0/0.0 = NaN`, and that NaN
    /// walks into the allocator - where it is selected, poisons the objective,
    /// and makes the sort comparator a non-total order. `strength` is a
    /// *derived* parameter (#176 owns it), so a bad derivation reaching here
    /// is a live path rather than a hypothetical.
    pub fn prior(p_bar: f64, strength: f64) -> Option<Self> {
        if !strength.is_finite() || strength <= 0.0 || p_bar.is_nan() {
            return None;
        }
        let p = p_bar.clamp(0.0, 1.0);
        Some(Beta {
            alpha: p * strength,
            beta: (1.0 - p) * strength,
        })
    }

    /// Conjugate update: `k` successes in `n` trials.
    ///
    /// Validated at the boundary rather than by `debug_assert`. The release
    /// profile sets no `debug-assertions` and no `overflow-checks`, so with
    /// `k > n` the old version wrapped the `u32` subtraction and produced
    /// `beta = 4.29e9` - destroying the posterior silently, which then reads
    /// as "nothing was worth applying to". A double-counted callback is not
    /// exotic; it is one ingestion bug away.
    pub fn update(self, k: u32, n: u32) -> Option<Self> {
        if k > n {
            return None;
        }
        Some(Beta {
            alpha: self.alpha + k as f64,
            beta: self.beta + (n - k) as f64,
        })
    }

    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    pub fn concentration(&self) -> f64 {
        self.alpha + self.beta
    }
}

/// The prior's share of the posterior mean: `w = (a+b) / (a+b+n)`.
///
/// Note this **decreases** with `n` - more data means *less* pull toward the
/// literature. ADR-005 originally stated the relationship backwards.
pub fn prior_weight(strength: f64, n: u32) -> f64 {
    strength / (strength + n as f64)
}

/// True when the prior still supplies more than half the answer, i.e.
/// `n < strength`. Derived from the model rather than a hand-picked floor,
/// which would be exactly the asserted constant ADR-008 bans.
pub fn prior_dominated(strength: f64, n: u32) -> bool {
    // With strength = 0, prior_weight is 0.0/0.0 = NaN and `NaN > 0.5` is
    // false - so a user with ZERO observations was reported as having a
    // *personal* estimate, violating acceptance criterion 6. Zero data is
    // always prior-dominated, whatever the strength.
    if n == 0 {
        return true;
    }
    let w = prior_weight(strength, n);
    if w.is_nan() {
        return true; // cannot establish a personal estimate; say so
    }
    w > 0.5
}

/// Regularised incomplete beta `I_x(a,b)`, via the continued fraction in
/// Numerical Recipes §6.4. Implemented inline rather than adding a
/// dependency - the hardware floor is 4 GB CPU-only.
fn betacf(a: f64, b: f64, x: f64) -> f64 {
    const MAXIT: usize = 300;
    const EPS: f64 = 3.0e-14;
    const FPMIN: f64 = 1.0e-300;
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < FPMIN {
        d = FPMIN;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=MAXIT {
        let m_f = m as f64;
        let m2 = 2.0 * m_f;
        let aa = m_f * (b - m_f) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m_f) * (qab + m_f) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < EPS {
            break;
        }
    }
    h
}

fn ln_gamma(x: f64) -> f64 {
    // Lanczos approximation, g=7, n=9.
    #[allow(clippy::excessive_precision, clippy::inconsistent_digit_grouping)]
    const C: [f64; 9] = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    ];
    if x < 0.5 {
        return (std::f64::consts::PI / (std::f64::consts::PI * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = C[0];
    let t = x + 7.5;
    for (i, &c) in C.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// CDF of Beta(a,b) at x.
pub fn beta_cdf(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let bt =
        (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * betacf(a, b, x) / a
    } else {
        1.0 - bt * betacf(b, a, 1.0 - x) / b
    }
}

/// Inverse CDF by bisection. Sufficient here - this runs once per reported
/// figure, not in a hot loop.
pub fn beta_quantile(a: f64, b: f64, p: f64) -> f64 {
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if beta_cdf(a, b, mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Equal-tailed credible interval at `level` (e.g. 0.90).
///
/// Equal-tailed rather than highest-density: at small `k` the posterior is
/// strongly skewed and the two differ materially, so leaving the choice open
/// makes two correct implementations disagree.
pub fn credible_interval(post: &Beta, level: f64) -> (f64, f64) {
    let tail = (1.0 - level) / 2.0;
    (
        beta_quantile(post.alpha, post.beta, tail),
        beta_quantile(post.alpha, post.beta, 1.0 - tail),
    )
}

/// The posterior mass actually contained in `[lo, hi]`.
///
/// This is the function that catches a printed interval whose stated level is
/// not its real level - the defect that survived several human reviews.
pub fn interval_level_of(post: &Beta, lo: f64, hi: f64) -> f64 {
    beta_cdf(post.alpha, post.beta, hi) - beta_cdf(post.alpha, post.beta, lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STRENGTH: f64 = 20.0;

    #[test]
    fn prior_mean_equals_the_published_rate() {
        let p = Beta::prior(0.115, STRENGTH).unwrap();
        assert!((p.mean() - 0.115).abs() < 1e-12);
        assert!((p.concentration() - STRENGTH).abs() < 1e-12);
    }

    /// The posterior mean is a precision-weighted average of prior mean and
    /// observed rate. Both forms must agree.
    #[test]
    fn shrinkage_identity_holds() {
        let p_bar = 0.10;
        let (k, n) = (1u32, 12u32);
        let post = Beta::prior(p_bar, STRENGTH).unwrap().update(k, n).unwrap();
        let w = prior_weight(STRENGTH, n);
        let weighted = w * p_bar + (1.0 - w) * (k as f64 / n as f64);
        assert!(
            (post.mean() - weighted).abs() < 1e-12,
            "conjugate mean {} != weighted form {}",
            post.mean(),
            weighted
        );
    }

    /// More data must mean LESS pull toward the prior. ADR-005 stated this
    /// backwards.
    #[test]
    fn shrinkage_decreases_with_more_data() {
        let ws: Vec<f64> = [4u32, 12, 140, 1000]
            .iter()
            .map(|&n| prior_weight(STRENGTH, n))
            .collect();
        for pair in ws.windows(2) {
            assert!(
                pair[1] < pair[0],
                "prior weight did not decrease: {:?}",
                ws
            );
        }
        assert!((ws[1] - 0.625).abs() < 1e-12);
    }

    #[test]
    fn prior_dominated_is_n_below_strength() {
        assert!(prior_dominated(STRENGTH, 12));
        assert!(prior_dominated(STRENGTH, 19));
        assert!(!prior_dominated(STRENGTH, 20));
        assert!(!prior_dominated(STRENGTH, 140));
    }

    /// Ground truth against analytic closed forms.
    ///
    /// **These assertions must be ASYMMETRIC in `(a, b)`.** An earlier version
    /// used only Beta(1,1) twice and Beta(2,2) - all symmetric - so an
    /// `alpha <-> beta` transposition, the single most likely bug in a
    /// hand-rolled incomplete beta, passed **every** test in this module while
    /// reporting a callback rate of 80-97% instead of 3-20%. Interval *width*
    /// is also invariant under transposition, so no width test catches it
    /// either.
    #[test]
    fn beta_cdf_matches_known_values() {
        // Symmetric cases: necessary but not sufficient.
        assert!((beta_cdf(1.0, 1.0, 0.25) - 0.25).abs() < 1e-9);
        assert!((beta_cdf(1.0, 1.0, 0.80) - 0.80).abs() < 1e-9);
        assert!((beta_cdf(2.0, 2.0, 0.5) - 0.5).abs() < 1e-9);

        // ASYMMETRIC cases. I_x(2,1) = x^2 and I_x(1,2) = 1-(1-x)^2, which
        // swap under transposition - so these two kill it outright.
        assert!(
            (beta_cdf(2.0, 1.0, 0.5) - 0.25).abs() < 1e-9,
            "I_0.5(2,1) must be 0.25; a transposition gives 0.75"
        );
        assert!(
            (beta_cdf(1.0, 2.0, 0.5) - 0.75).abs() < 1e-9,
            "I_0.5(1,2) must be 0.75; a transposition gives 0.25"
        );
        assert!((beta_cdf(3.0, 1.0, 0.5) - 0.125).abs() < 1e-9);
        assert!((beta_cdf(1.0, 3.0, 0.2) - 0.488).abs() < 1e-9);
        // Skewed shapes in the range the model actually uses.
        assert!((beta_cdf(2.0, 8.0, 0.2) - 0.56379238).abs() < 1e-6);
    }

    /// The transposition guard, stated as its own property so it cannot be
    /// weakened by editing the table above: `I_x(a,b) = 1 - I_{1-x}(b,a)`.
    /// A transposed implementation violates this for every asymmetric pair.
    #[test]
    fn beta_cdf_satisfies_the_reflection_identity() {
        for &(a, b) in &[(2.0, 1.0), (2.3, 17.7), (3.3, 28.7), (17.6, 142.4)] {
            for &x in &[0.05, 0.2, 0.5, 0.8, 0.95] {
                let lhs = beta_cdf(a, b, x);
                let rhs = 1.0 - beta_cdf(b, a, 1.0 - x);
                assert!(
                    (lhs - rhs).abs() < 1e-9,
                    "reflection identity failed at Beta({a},{b}) x={x}: {lhs} vs {rhs}"
                );
            }
        }
    }

    #[test]
    fn credible_interval_contains_the_level_it_claims() {
        let post = Beta::prior(0.115, STRENGTH).unwrap().update(1, 12).unwrap();
        let (lo, hi) = credible_interval(&post, 0.90);
        let actual = interval_level_of(&post, lo, hi);
        assert!(
            (actual - 0.90).abs() < 1e-6,
            "asked for 90%, interval [{lo:.4}, {hi:.4}] contains {actual:.4}"
        );
    }

    /// THE REGRESSION. The spec printed `4%-19%` at n=12 and `9%-13%` at
    /// n=140, both labelled "equal-tailed 90%". Neither was. Any example
    /// added to the docs must satisfy this test before it is printed.
    #[test]
    fn printed_examples_match_their_claimed_level() {
        let cases = [
            ("n=12 prior-dominated", 1u32, 12u32),
            ("n=140 personal", 16u32, 140u32),
        ];
        for (label, k, n) in cases {
            let post = Beta::prior(0.115, STRENGTH).unwrap().update(k, n).unwrap();
            let (lo, hi) = credible_interval(&post, 0.90);
            let actual = interval_level_of(&post, lo, hi);
            assert!(
                (actual - 0.90).abs() < 1e-6,
                "{label}: computed interval does not hold 90%"
            );
            // The old hand-written pair fails this, which is the point.
            let stale = if n == 12 { (0.04, 0.19) } else { (0.09, 0.13) };
            let stale_level = interval_level_of(&post, stale.0, stale.1);
            assert!(
                (stale_level - 0.90).abs() > 0.02,
                "{label}: the previously-printed interval {stale:?} would now \
                 pass, so this regression no longer guards anything"
            );
        }
    }

    /// Narrower intervals need far more data than intuition suggests. The
    /// spec claimed a 4-point-wide interval at n=140; it needs several
    /// hundred.
    #[test]
    fn four_point_interval_needs_hundreds_of_observations() {
        let width_at = |n: u32| {
            let k = (0.11 * n as f64).round() as u32;
            let post = Beta::prior(0.115, STRENGTH).unwrap().update(k, n).unwrap();
            let (lo, hi) = credible_interval(&post, 0.90);
            hi - lo
        };
        assert!(
            width_at(140) > 0.04,
            "a 90% interval at n=140 is wider than 4 points"
        );
        assert!(
            width_at(700) < 0.05,
            "several hundred observations should get close to 4 points"
        );
    }

    /// A user with no history gets the prior, and the tool must say so.
    #[test]
    fn zero_history_is_prior_dominated() {
        let post = Beta::prior(0.115, STRENGTH).unwrap().update(0, 0).unwrap();
        assert!(prior_dominated(STRENGTH, 0));
        assert!((post.mean() - 0.115).abs() < 1e-12);
    }
}
