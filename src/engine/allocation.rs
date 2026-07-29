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
    /// Inputs were not usable (NaN, or a bad decay fit). Never silently
    /// treated as a good posting.
    Unusable,
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
pub fn decay(age_days: f64, half_life_days: f64) -> Option<f64> {
    // A bad fit must fail LOUDLY. An earlier version returned 1.0 here,
    // reasoning that it "refuses to trust a bad fit" - but 1.0 asserts every
    // posting is maximally fresh, which makes the allocator age-blind and
    // re-inverts invariant 1: with h <= 0 a 365-day posting ties a 1-day one
    // and the id tie-break decides. That is the exact defect this module
    // exists to prevent, reintroduced through the guard written to prevent it.
    if half_life_days.is_nan() || half_life_days <= 0.0 {
        return None;
    }
    if half_life_days.is_infinite() {
        return Some(1.0); // the correct limit: no decay at all
    }
    if age_days.is_nan() {
        return None;
    }
    Some(0.5_f64.powf(age_days / half_life_days).clamp(0.0, 1.0))
}

/// Edge cost. Linear, bounded in `[0, 1]`.
///
/// Minimising the total maximises `sum(p * decay)`, which is expected
/// callbacks. Two earlier forms were wrong: `-log p * decay` inverted the age
/// direction, and `-log(p * decay)` maximised the *product* - the probability
/// that every application succeeds, which nobody wants.
pub fn cost(p_callback: f64, decay: f64) -> Option<f64> {
    // f64::clamp PROPAGATES NaN, so clamping is not a guard. An unguarded NaN
    // is selected (NaN >= tau is false), poisons expected_callbacks, and makes
    // the sort comparator a non-total order - which on Rust >= 1.81 may panic
    // or silently randomise the slate. debug_assert does not help: the release
    // profile sets no debug-assertions.
    if p_callback.is_nan() || decay.is_nan() {
        return None;
    }
    Some(1.0 - p_callback.clamp(0.0, 1.0) * decay.clamp(0.0, 1.0))
}

/// Reservation cost: the price of leaving a unit of budget unspent.
pub fn tau(p_min: f64) -> f64 {
    1.0 - p_min.clamp(0.0, 1.0)
}

/// Smallest per-family cap that still lets the budget be spent across `n`
/// families. Without this floor the generalist end silently under-fills:
/// six families at `cap = 1` with a budget of seven strands a unit in slack
/// even when a good seventh posting exists in an already-used family.
pub fn min_feasible_cap(budget: usize, family_sizes: &[usize]) -> usize {
    // ceil(B/F) is NECESSARY BUT NOT SUFFICIENT: it assumes every family holds
    // at least that many above-floor postings. Real distributions are skewed.
    // B=7 over {f0: 6, f1: 1} gives ceil(7/2)=4, which still under-fills by 2 -
    // the exact harm the floor exists to prevent. The correct condition is
    // sum(min(cap, |Fi|)) >= B; here that needs cap = 6.
    if family_sizes.is_empty() {
        return 1;
    }
    let total: usize = family_sizes.iter().sum();
    let target = budget.min(total);
    (1..=target.max(1))
        .find(|cap| family_sizes.iter().map(|n| (*n).min(*cap)).sum::<usize>() >= target)
        .unwrap_or(target.max(1))
}

pub fn solve(candidates: &[Candidate], params: &Params) -> Slate {
    let mut skipped: Vec<(String, Skipped)> = Vec::new();
    let mut scored: Vec<(&Candidate, f64)> = Vec::new();

    for c in candidates {
        // Anything unusable is reported, never silently scored. A NaN that
        // reaches the sort makes the comparator a non-total order, which on
        // Rust >= 1.81 may panic or silently randomise the slate.
        let Some(d) = decay(c.age_days, params.half_life_days) else {
            skipped.push((c.id.clone(), Skipped::Unusable));
            continue;
        };
        let Some(cst) = cost(c.p_callback, d) else {
            skipped.push((c.id.clone(), Skipped::Unusable));
            continue;
        };
        debug_assert!(
            (0.0..=1.0).contains(&cst),
            "cost {cst} outside [0,1] - a negative or unbounded cost breaks \
             the solver silently"
        );
        scored.push((c, c.p_callback.clamp(0.0, 1.0) * d));
    }

    // Descending value; NaN is already excluded above, so the comparator is a
    // total order. Ties break on id, making the slate byte-identical for
    // identical input regardless of scan order.
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.id.cmp(&b.0.id))
    });

    let mut selected = Vec::new();
    let mut per_family: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut expected = 0.0;

    for (c, value) in scored {
        // Tested on the VALUE scale, matching the spec's `P*decay > P_min`.
        // Testing `1 - p*d >= 1 - p_min` instead loses low bits to cancellation
        // and disagrees with the spec at the margin.
        if value <= params.p_min {
            skipped.push((c.id.clone(), Skipped::BelowFloor));
            continue;
        }
        // Family cap is checked BEFORE budget: when both bind, the cap is the
        // constraint that survives budget relaxation, so it is the honest
        // reason to show. Reporting BudgetExhausted would imply a counterfactual
        // ("raise your budget") that a re-solve will not deliver.
        let used = *per_family.get(c.archetype.as_str()).unwrap_or(&0);
        if used >= params.family_cap {
            skipped.push((c.id.clone(), Skipped::FamilyCapReached));
            continue;
        }
        if selected.len() >= params.budget {
            skipped.push((c.id.clone(), Skipped::BudgetExhausted));
            continue;
        }
        *per_family.entry(c.archetype.as_str()).or_insert(0) += 1;
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
        let d = decay(-30.0, 7.0).expect("a future-dated posting is usable");
        assert!(d <= 1.0, "decay {d} exceeded 1 for a future-dated posting");
        let k = cost(0.10, d).expect("cost of a usable posting");
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
                let v = x.p_callback * decay(x.age_days, p.half_life_days).unwrap();
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

    /// Invariant 7, strengthened. The fixed 5-candidate case above can only
    /// catch gross errors. This randomises candidates, families, budgets,
    /// caps and floors, and brute-forces the exact optimum each time - so the
    /// "greedy is optimal on this matroid" claim is verified rather than
    /// asserted, across cases nobody hand-picked.
    #[test]
    fn invariant_7_randomised_greedy_matches_brute_force() {
        // Deterministic LCG: a fixed seed keeps CI reproducible.
        let mut seed = 0x2545F4914F6CDD1Du64;
        let mut rnd = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 33) as f64) / ((1u64 << 31) as f64)
        };

        for _ in 0..400 {
            let n = 3 + (rnd() * 7.0) as usize;
            let fams = 1 + (rnd() * 4.0) as usize;
            let budget = 1 + (rnd() * 5.0) as usize;
            let cap = 1 + (rnd() * 3.0) as usize;
            let p_min = rnd() * 0.3;
            let cands: Vec<Candidate> = (0..n)
                .map(|i| Candidate {
                    id: format!("p{i:02}"),
                    archetype: format!("f{}", i % fams),
                    p_callback: rnd(),
                    age_days: rnd() * 60.0,
                })
                .collect();
            let p = Params {
                budget,
                p_min,
                half_life_days: 7.0,
                family_cap: cap,
            };
            let ours = solve(&cands, &p).expected_callbacks;

            let mut best = 0.0f64;
            for mask in 0u32..(1u32 << n) {
                let mut fam: std::collections::HashMap<&str, usize> = Default::default();
                let (mut cnt, mut tot, mut ok) = (0usize, 0.0f64, true);
                for (i, c) in cands.iter().enumerate() {
                    if mask & (1 << i) == 0 {
                        continue;
                    }
                    let v = c.p_callback * decay(c.age_days, p.half_life_days).unwrap();
                    if 1.0 - v >= tau(p.p_min) {
                        ok = false;
                        break;
                    }
                    cnt += 1;
                    if cnt > budget {
                        ok = false;
                        break;
                    }
                    let e = fam.entry(c.archetype.as_str()).or_insert(0);
                    *e += 1;
                    if *e > cap {
                        ok = false;
                        break;
                    }
                    tot += v;
                }
                if ok && tot > best {
                    best = tot;
                }
            }
            assert!(
                best - ours < 1e-9,
                "greedy got {ours}, optimum {best} (n={n} fams={fams}                  budget={budget} cap={cap} p_min={p_min:.3})"
            );
        }
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

    /// A bad fit must be REJECTED, not silently treated as maximally fresh.
    ///
    /// An earlier version returned `1.0` here on the reasoning that it
    /// "refuses to trust a bad fit". It did the opposite: `decay = 1.0` for
    /// every posting makes the allocator age-blind, so a 365-day posting ties
    /// a 1-day one and the id tie-break decides - **re-inverting invariant 1**,
    /// the exact defect this module exists to prevent. It survived all 15
    /// tests, because none of them pinned the bad-fit value.
    #[test]
    fn bad_decay_fit_is_rejected_not_treated_as_fresh() {
        assert_eq!(decay(8.0, -7.0), None, "negative half-life must be rejected");
        assert_eq!(decay(0.0, 0.0), None, "zero half-life yields NaN; reject it");
        assert_eq!(decay(1.0, f64::NAN), None, "NaN half-life must be rejected");
        // +inf is different: no decay at all is the correct limit.
        assert_eq!(decay(50.0, f64::INFINITY), Some(1.0));
    }

    /// The whole point of rejecting rather than defaulting: with a bad fit,
    /// an ancient posting must not be selected over a fresh one.
    #[test]
    fn bad_fit_does_not_let_an_ancient_posting_win() {
        let cands = vec![c("ancient", "f", 0.10, 365.0), c("new", "f", 0.10, 1.0)];
        let good = Params { budget: 1, p_min: 0.01, half_life_days: 7.0, family_cap: 9 };
        assert_eq!(solve(&cands, &good).selected, vec!["new"]);

        let bad = Params { half_life_days: -7.0, ..good.clone() };
        let s = solve(&cands, &bad);
        assert!(
            s.selected.is_empty(),
            "a bad fit must yield no slate, not an age-blind one: {:?}",
            s.selected
        );
        assert!(s.skipped.iter().all(|(_, r)| *r == Skipped::Unusable));
    }

    /// NaN must never reach the sort: it makes the comparator a non-total
    /// order, which can panic or silently randomise the slate on Rust >= 1.81.
    /// `f64::clamp` PROPAGATES NaN, so clamping is not a guard.
    #[test]
    fn nan_inputs_are_rejected_not_selected() {
        assert_eq!(cost(f64::NAN, 1.0), None);
        assert_eq!(cost(0.5, f64::NAN), None);
        let cands = vec![c("nan_p", "f", f64::NAN, 1.0), c("ok", "f", 0.5, 1.0)];
        let s = solve(&cands, &params(2, 0.05, 9));
        assert_eq!(s.selected, vec!["ok"]);
        assert!(s
            .skipped
            .iter()
            .any(|(id, r)| id == "nan_p" && *r == Skipped::Unusable));
        assert!(s.expected_callbacks.is_finite(), "a NaN poisoned the objective");
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
        // Six families of one posting and a budget of seven: only SIX
        // applications are achievable, so cap 1 genuinely suffices. The
        // function reasons about achievable flow, not nominal budget.
        assert_eq!(min_feasible_cap(7, &[1, 1, 1, 1, 1, 1]), 1);
        // Balanced and genuinely binding: four families of two, budget seven.
        assert_eq!(min_feasible_cap(7, &[2, 2, 2, 2]), 2);
        // SKEWED: ceil(B/F) = ceil(7/2) = 4 is NOT enough here. With sizes
        // {6, 1} a cap of 4 admits only 4+1 = 5 of the 7 budget units. The
        // correct condition is sum(min(cap, |Fi|)) >= B, giving 6.
        assert_eq!(min_feasible_cap(7, &[6, 1]), 6);
        let cands: Vec<_> = (0..6)
            .map(|i| c(&format!("p{i}"), &format!("f{i}"), 0.5, 0.0))
            .collect();

        let starved = solve(&cands, &params(7, 0.05, 1));
        assert_eq!(starved.selected.len(), 6, "6 families at cap 1 strand a unit");

        let cap = min_feasible_cap(7, &[2, 1, 1, 1, 1, 1]);
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
