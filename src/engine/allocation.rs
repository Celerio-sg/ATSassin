//! Layer 3 reference semantics and the executable form of the solver
//! invariants in `docs/design/ALLOCATION_LAYER.md`.
//!
//! **Why this module exists.** Three sign inversions shipped in that spec
//! within a single day - the cost function multiplied decay outside the log
//! (making the solver prefer *stale* postings), the slack edge attached to
//! `source` instead of `user` (making the slate unable to under-fill), and
//! the diversification cap was documented as its own inverse. Every one was
//! syntactically plausible, none was caught by review, and each was caught in
//! milliseconds by arithmetic. Two further defects were introduced *while
//! fixing* the first three.
//!
//! Prose invariants do not catch sign errors. These do. The spec's own
//! instruction is "write these as tests FIRST", and this is that.
//!
//! **Why greedy and not min-cost flow.** Under the constraint set as
//! specified - a budget, a per-family cap, one application per posting - the
//! feasible sets form a truncated partition matroid and the objective is
//! modular, so greedy is *exactly* optimal (Rado-Edmonds). A flow solver is
//! machinery for a problem a sort already solves. Greedy is therefore the
//! honest reference, and `invariant_7_no_better_selection_exists` checks it
//! against brute force so the claim is verified rather than asserted.
//!
//! When #167's effort weighting lands, budget consumption becomes
//! heterogeneous, the matroid property is lost, and a flow (or DP) solver
//! becomes necessary. At that point `solve` is replaced and **these tests
//! stay** - they are the contract, not the implementation.

/// A posting as the allocator sees it. `p_callback` comes from Layer 2.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Stable content-addressed id (ADR-001). Also the deterministic
    /// tie-break key, so ordering never depends on scan order.
    pub id: String,
    /// The single role family this posting is assigned to. Exactly one, or
    /// the cap is unenforceable - flow could otherwise reach one posting
    /// through several family edges and evade it.
    pub archetype: String,
    /// P(callback) for this user and posting, from Layer 2.
    pub p_callback: f64,
    /// Days since the posting went up. Negative means future-dated, which is
    /// routine in real feeds (scheduled publication, timezone skew).
    pub age_days: f64,
}

#[derive(Debug, Clone)]
pub struct Params {
    /// Tailored applications the user can sustain this period. Derived from
    /// their own throughput or asked - never a constant (ADR-008).
    pub budget: usize,
    /// Lowest callback probability worth an application. The relevance floor,
    /// expressed as a probability rather than as a cost.
    pub p_min: f64,
    /// Decay half-life in days. Fitted once Layer 2 has data.
    pub half_life_days: f64,
    /// Per-family maximum. `cap = 1` forces maximum spread; `cap >= budget`
    /// permits full concentration. Documented backwards in three artifacts
    /// before this was written down.
    pub family_cap: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Skipped {
    /// Cost at or above tau - not worth an application at this user's floor.
    BelowFloor,
    /// Its family was already at capacity.
    FamilyCapReached,
    /// Budget exhausted by higher-value postings.
    BudgetExhausted,
}

#[derive(Debug)]
pub struct Slate {
    pub selected: Vec<String>,
    /// Every non-selected candidate with a machine-generated reason. A slate
    /// without reasons is an oracle, not a decision.
    pub skipped: Vec<(String, Skipped)>,
    /// Sum of `p_callback * decay` over the selection. Expected callbacks by
    /// linearity of expectation - no independence assumption. This is the
    /// quantity a counterfactual must diff, NOT the raw objective, which
    /// contains `budget` and therefore *grows* when the budget is raised.
    pub expected_callbacks: f64,
}

/// Age weighting. Clamped to `(0, 1]`.
///
/// The clamp is on the **output**, not on `age_days`. Clamping the input
/// closes only one of three routes to `decay > 1`:
///
/// | Route | Example | Caught by an age clamp? |
/// |---|---|---|
/// | future-dated posting | `age = -30, h = 7` | yes |
/// | negative half-life   | `age = 8,  h = -7` | **no** - age is positive |
/// | zero half-life       | `age = 0,  h = 0`  | **no** - yields NaN |
///
/// `half_life_days` is fitted, so a bad fit reaching here is a live path.
/// A `decay > 1` gives `p * decay > 1`, a negative cost, and Dijkstra with
/// zero-initialised potentials then returns a wrong path *silently*.
pub fn decay(age_days: f64, half_life_days: f64) -> f64 {
    if !half_life_days.is_finite() || half_life_days <= 0.0 {
        return 1.0; // refuse to trust a bad fit rather than propagate NaN
    }
    let d = 0.5_f64.powf(age_days / half_life_days);
    d.clamp(0.0, 1.0)
}

/// Edge cost. Linear, bounded in `[0, 1]`.
///
/// Minimising the total maximises `sum(p * decay)`, which is expected
/// callbacks. Two earlier forms were wrong: `-log p * decay` inverted the age
/// direction, and `-log(p * decay)` maximised the *product* - the probability
/// that every application succeeds, which nobody wants.
pub fn cost(p_callback: f64, decay: f64) -> f64 {
    let p = p_callback.clamp(0.0, 1.0);
    let d = decay.clamp(0.0, 1.0);
    1.0 - p * d
}

/// Reservation cost: the price of leaving a unit of budget unspent.
pub fn tau(p_min: f64) -> f64 {
    1.0 - p_min.clamp(0.0, 1.0)
}

/// Smallest per-family cap that still lets the budget be spent across `n`
/// families. Without this floor the generalist end silently under-fills:
/// six families at `cap = 1` with a budget of seven strands a unit in slack
/// even when a good seventh posting exists in an already-used family.
pub fn min_feasible_cap(budget: usize, families: usize) -> usize {
    if families == 0 {
        return 1;
    }
    budget.div_ceil(families).max(1)
}

pub fn solve(candidates: &[Candidate], params: &Params) -> Slate {
    let t = tau(params.p_min);

    let mut scored: Vec<(&Candidate, f64, f64)> = candidates
        .iter()
        .map(|c| {
            let d = decay(c.age_days, params.half_life_days);
            let value = c.p_callback.clamp(0.0, 1.0) * d;
            let cst = cost(c.p_callback, d);
            debug_assert!(
                (0.0..=1.0).contains(&cst),
                "cost {cst} outside [0,1] - a negative or unbounded cost breaks \
                 the solver silently"
            );
            (c, value, cst)
        })
        .collect();

    // Descending value; ties broken on id so the slate is byte-identical for
    // identical input regardless of scan order.
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.id.cmp(&b.0.id))
    });

    let mut selected = Vec::new();
    let mut skipped = Vec::new();
    let mut per_family: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut expected = 0.0;

    for (c, value, cst) in scored {
        // Strictly cheaper than slack. Equality goes to slack: at p_min = 0
        // a worthless posting costs exactly tau, and `<=` would fill the
        // slate with zero-probability applications.
        if cst >= t {
            skipped.push((c.id.clone(), Skipped::BelowFloor));
            continue;
        }
        if selected.len() >= params.budget {
            skipped.push((c.id.clone(), Skipped::BudgetExhausted));
            continue;
        }
        let used = per_family.entry(c.archetype.as_str()).or_insert(0);
        if *used >= params.family_cap {
            skipped.push((c.id.clone(), Skipped::FamilyCapReached));
            continue;
        }
        *used += 1;
        expected += value;
        selected.push(c.id.clone());
    }

    Slate {
        selected,
        skipped,
        expected_callbacks: expected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(id: &str, fam: &str, p: f64, age: f64) -> Candidate {
        Candidate {
            id: id.into(),
            archetype: fam.into(),
            p_callback: p,
            age_days: age,
        }
    }

    fn params(budget: usize, p_min: f64, cap: usize) -> Params {
        Params {
            budget,
            p_min,
            half_life_days: 7.0,
            family_cap: cap,
        }
    }

    // ---- The eight invariants from docs/design/ALLOCATION_LAYER.md ----

    /// Invariant 1. Catches the original `-log P * decay` inversion, which
    /// made the solver prefer stale postings.
    #[test]
    fn invariant_1_younger_posting_preferred() {
        let cands = vec![c("old", "f", 0.10, 34.0), c("new", "f", 0.10, 1.0)];
        let s = solve(&cands, &params(1, 0.01, 9));
        assert_eq!(s.selected, vec!["new"]);
    }

    /// Invariant 2. Catches slack attached to `source` instead of `user`,
    /// which made the slate unable to under-fill.
    #[test]
    fn invariant_2_all_below_floor_gives_empty_slate() {
        let cands = vec![c("a", "f", 0.01, 0.0), c("b", "g", 0.02, 0.0)];
        let s = solve(&cands, &params(5, 0.5, 9));
        assert!(s.selected.is_empty());
        assert!(s.skipped.iter().all(|(_, r)| *r == Skipped::BelowFloor));
    }

    /// Invariant 3, with the cap proviso. Without "no family cap binding"
    /// this contradicts invariant 4.
    #[test]
    fn invariant_3_fills_budget_when_no_cap_binds() {
        let cands: Vec<_> = (0..9)
            .map(|i| c(&format!("p{i}"), &format!("f{i}"), 0.5, 0.0))
            .collect();
        let s = solve(&cands, &params(5, 0.05, 9));
        assert_eq!(s.selected.len(), 5);
    }

    /// Invariant 4. Two families at cap 1 with a budget of 3 must yield 2,
    /// not 3 - which is exactly why invariant 3 needs its proviso.
    #[test]
    fn invariant_4_family_cap_stops_selection() {
        let cands = vec![
            c("a1", "f1", 0.9, 0.0),
            c("a2", "f1", 0.8, 0.0),
            c("a3", "f1", 0.7, 0.0),
            c("b1", "f2", 0.6, 0.0),
            c("b2", "f2", 0.5, 0.0),
        ];
        let s = solve(&cands, &params(3, 0.05, 1));
        assert_eq!(s.selected.len(), 2, "cap 1 over 2 families caps flow at 2");
        assert!(s
            .skipped
            .iter()
            .any(|(_, r)| *r == Skipped::FamilyCapReached));
    }

    /// Invariant 5. Determinism must not depend on input order.
    #[test]
    fn invariant_5_deterministic_regardless_of_input_order() {
        let mut cands = vec![
            c("a", "f1", 0.5, 1.0),
            c("b", "f2", 0.5, 1.0),
            c("d", "f3", 0.5, 1.0),
        ];
        let first = solve(&cands, &params(2, 0.05, 9)).selected;
        cands.reverse();
        let second = solve(&cands, &params(2, 0.05, 9)).selected;
        assert_eq!(first, second, "equal-value postings must tie-break on id");
    }

    /// Invariant 6. A future-dated posting must not produce a negative cost.
    #[test]
    fn invariant_6_future_dated_posting_stays_in_range() {
        let d = decay(-30.0, 7.0);
        assert!(d <= 1.0, "decay {d} exceeded 1 for a future-dated posting");
        let k = cost(0.10, d);
        assert!((0.0..=1.0).contains(&k), "cost {k} outside [0,1]");
    }

    /// Invariant 7. The general one: if the objective is pointed the wrong
    /// way, this fails even when 1-6 pass. Brute-forces every feasible
    /// subset and confirms greedy is optimal.
    #[test]
    fn invariant_7_no_better_selection_exists() {
        let cands = vec![
            c("a", "f1", 0.40, 0.0),
            c("b", "f1", 0.35, 0.0),
            c("d", "f2", 0.30, 0.0),
            c("e", "f2", 0.25, 0.0),
            c("g", "f3", 0.20, 0.0),
        ];
        let p = params(3, 0.05, 2);
        let ours = solve(&cands, &p).expected_callbacks;

        let mut best = 0.0_f64;
        for mask in 0u32..(1 << cands.len()) {
            let chosen: Vec<_> = (0..cands.len())
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| &cands[i])
                .collect();
            if chosen.len() > p.budget {
                continue;
            }
            let mut fam: std::collections::HashMap<&str, usize> = Default::default();
            let mut ok = true;
            let mut total = 0.0;
            for x in &chosen {
                let e = fam.entry(x.archetype.as_str()).or_insert(0);
                *e += 1;
                if *e > p.family_cap {
                    ok = false;
                    break;
                }
                let v = x.p_callback * decay(x.age_days, p.half_life_days);
                if 1.0 - v >= tau(p.p_min) {
                    ok = false;
                    break;
                }
                total += v;
            }
            if ok && total > best {
                best = total;
            }
        }
        assert!(
            (ours - best).abs() < 1e-9,
            "greedy got {ours}, brute force found {best} - the objective is \
             pointed the wrong way or the tie-break is wrong"
        );
    }

    /// Invariant 8. Off-by-one in the budget.
    #[test]
    fn invariant_8_budget_of_one() {
        let cands: Vec<_> = (0..5)
            .map(|i| c(&format!("p{i}"), "f", 0.5, 0.0))
            .collect();
        assert_eq!(solve(&cands, &params(1, 0.05, 9)).selected.len(), 1);
    }

    // ---- Regressions for defects that actually shipped ----

    /// `p_min = 0` makes tau exactly 1, and a worthless posting also costs
    /// exactly 1. A `<=` comparison filled the slate with zero-probability
    /// applications; equality must go to slack.
    #[test]
    fn zero_floor_does_not_select_worthless_postings() {
        let cands: Vec<_> = (0..7)
            .map(|i| c(&format!("z{i}"), "f", 0.0, 0.0))
            .collect();
        let s = solve(&cands, &params(7, 0.0, 9));
        assert!(
            s.selected.is_empty(),
            "p_min=0 ties must resolve toward slack, not toward postings"
        );
    }

    /// A negative half-life has a *positive* age, so an age clamp misses it.
    #[test]
    fn negative_half_life_cannot_produce_negative_cost() {
        let d = decay(8.0, -7.0);
        assert!(d <= 1.0, "decay {d} exceeded 1 with a negative half-life");
        assert!((0.0..=1.0).contains(&cost(0.9, d)));
    }

    /// `0.5^(0/0)` is NaN, and NaN propagates silently through comparisons.
    #[test]
    fn zero_half_life_does_not_produce_nan() {
        let d = decay(0.0, 0.0);
        assert!(d.is_finite(), "decay was NaN for half_life = 0");
        assert!(cost(0.5, d).is_finite());
    }

    /// The cap is a per-family *maximum*: 1 forces maximum spread,
    /// `>= budget` permits full concentration. Documented as its own
    /// inverse in three artifacts.
    #[test]
    fn cap_direction_cap_of_one_forces_spread() {
        let cands: Vec<_> = (0..6)
            .map(|i| c(&format!("p{i}"), "one_family", 0.5, 0.0))
            .collect();

        let spread = solve(&cands, &params(6, 0.05, 1));
        assert_eq!(spread.selected.len(), 1, "cap=1 is MAXIMUM diversification");

        let concentrated = solve(&cands, &params(6, 0.05, 6));
        assert_eq!(
            concentrated.selected.len(),
            6,
            "cap>=budget is NO diversification - a specialist must be able to \
             concentrate"
        );
    }

    /// The generalist end of the same knob: without a feasibility floor the
    /// budget silently under-fills.
    #[test]
    fn feasibility_floor_prevents_generalist_underfill() {
        assert_eq!(min_feasible_cap(7, 6), 2);
        let cands: Vec<_> = (0..6)
            .map(|i| c(&format!("p{i}"), &format!("f{i}"), 0.5, 0.0))
            .collect();

        let starved = solve(&cands, &params(7, 0.05, 1));
        assert_eq!(starved.selected.len(), 6, "6 families at cap 1 strand a unit");

        let cap = min_feasible_cap(7, 6);
        let extra = c("p6", "f0", 0.5, 0.0);
        let mut with_extra = cands.clone();
        with_extra.push(extra);
        let ok = solve(&with_extra, &params(7, 0.05, cap));
        assert_eq!(ok.selected.len(), 7, "the floor must let the budget be spent");
    }

    /// Every non-selected candidate carries a reason. A slate without
    /// reasons is an oracle, not a decision.
    #[test]
    fn every_skipped_candidate_has_a_reason() {
        let cands = vec![
            c("good", "f1", 0.90, 0.0),
            c("capped", "f1", 0.80, 0.0),
            c("poor", "f2", 0.001, 0.0),
        ];
        let s = solve(&cands, &params(5, 0.05, 1));
        assert_eq!(s.selected.len() + s.skipped.len(), cands.len());
        assert!(s.skipped.iter().any(|(id, r)| id == "capped"
            && *r == Skipped::FamilyCapReached));
        assert!(s
            .skipped
            .iter()
            .any(|(id, r)| id == "poor" && *r == Skipped::BelowFloor));
    }

    /// The counterfactual must diff expected callbacks, not the raw
    /// objective. The objective contains `budget`, so diffing it reports
    /// that raising the budget made things *worse*.
    #[test]
    fn raising_budget_never_lowers_expected_callbacks() {
        let cands: Vec<_> = (0..12)
            .map(|i| {
                c(
                    &format!("p{i}"),
                    &format!("f{}", i % 4),
                    0.3 - (i as f64 * 0.01),
                    0.0,
                )
            })
            .collect();
        let small = solve(&cands, &params(3, 0.05, 9)).expected_callbacks;
        let large = solve(&cands, &params(6, 0.05, 9)).expected_callbacks;
        assert!(
            large >= small,
            "expected callbacks fell from {small} to {large} when the budget rose"
        );
    }
}
