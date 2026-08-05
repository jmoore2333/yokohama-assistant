//! The shared per-decision SCORED-SUCCESSOR meter (Phase 3.4 spec §3.4) — the
//! matched-budget instrument the architecture experiment adjudicates on.
//!
//! # The currency, and why it is TOTAL
//!
//! One **scored successor** = one apply + observe + evaluate of a candidate.
//! Spec §0.3 fixes it as the experiment's budget unit (measured at ≈1.359 µs
//! inside the S1 rung, ≈88% of its per-decision wall) and fixes it **root
//! included**: the research base's meter counts the root loop, so a
//! "post-root" cap would silently re-base B\* = 750 into a different unit. This
//! type therefore counts exactly what
//! [`SearchTally::candidates_scored`](crate::game::lookahead::SearchTally::candidates_scored)
//! already counted — the meter is a GATE on a number the repo was already
//! measuring, not a second currency beside it.
//!
//! # Two knobs' worth of semantics, both deliberate
//!
//! - **`None` is not "a very large cap"** — an uncapped budget makes
//!   [`may_start`](ScoredBudget::may_start) constantly true and
//!   [`exhausted`](ScoredBudget::exhausted) constantly false, so a search that
//!   threads one through takes byte-identical decisions to one that never heard
//!   of a budget. That is what makes `cap=None` a golden-pinned no-op rather
//!   than a behaviour a test has to re-verify.
//! - **The meter is per DECISION, not per search.** A caller running K sampled
//!   worlds threads ONE budget through all K, because "total scored successors
//!   per decision" is what §0.3 prices. At the shipped `K = 1` the two coincide;
//!   at `K > 1` the later worlds see a budget the earlier ones have already
//!   spent, which is the honest reading of the total and is stated at
//!   [`crate::game::lookahead::search_world_budgeted`] rather than silently
//!   reinterpreted.
//!
//! Nothing here decides anything: a budget answers "may deeper work START?" and
//! "has it DIED?", and the search owns what to do about either. The two
//! consumers (spec §3.4's capped `lookahead-v1`, and 3.4's MCTS-v1 at T3) differ
//! in exactly that policy — a capped continuation is atomic and discarded whole,
//! a started simulation completes — so the policy cannot live in the meter.

use crate::game::greedy::uniform_index;

/// The per-decision scored-successor meter: a cap that may be absent, and what
/// has been spent against it.
///
/// `Copy` because it is two words and every consumer wants to hand a snapshot
/// to a report; the LIVE meter is always threaded as `&mut`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoredBudget {
    cap: Option<u32>,
    realized: u64,
}

impl ScoredBudget {
    /// No cap — the shipped shape, and byte-identical to no budget at all.
    pub const fn unlimited() -> Self {
        Self { cap: None, realized: 0 }
    }

    /// A cap of `cap` TOTAL scored successors for this decision, root included.
    pub const fn capped(cap: u32) -> Self {
        Self { cap: Some(cap), realized: 0 }
    }

    /// The knob's shape as the agents carry it: `None` ⇒ [`unlimited`](Self::unlimited).
    pub const fn from_cap(cap: Option<u32>) -> Self {
        match cap {
            Some(c) => Self::capped(c),
            None => Self::unlimited(),
        }
    }

    /// This decision's cap, if it has one.
    pub const fn cap(&self) -> Option<u32> {
        self.cap
    }

    /// Scored successors charged so far this decision, root pass included.
    pub const fn realized(&self) -> u64 {
        self.realized
    }

    /// Charge ONE scored successor. Charging past the cap is legal and is how
    /// [`overshoot`](Self::overshoot) becomes measurable: spec §3.4 makes the
    /// root pass MANDATORY, so a decision whose root family alone is bigger than
    /// its cap overruns by construction and the overrun is a validity line
    /// rather than an error.
    pub fn charge(&mut self) {
        self.realized += 1;
    }

    /// May deeper work be STARTED? (`realized < cap`, always true uncapped.)
    ///
    /// Deliberately not "will it fit": no caller can know a continuation's or a
    /// simulation's cost before paying it, so the start gate is the only
    /// question the meter can answer honestly.
    pub fn may_start(&self) -> bool {
        self.cap.is_none_or(|c| self.realized < u64::from(c))
    }

    /// Has the budget died? The negation of [`may_start`](Self::may_start),
    /// named separately because the two read at different sites: one gates
    /// STARTING a unit of work, the other aborts one already running.
    pub fn exhausted(&self) -> bool {
        !self.may_start()
    }

    /// `realized − cap`, floored at 0 — 0 whenever uncapped or inside the cap.
    pub fn overshoot(&self) -> u64 {
        match self.cap {
            Some(c) => self.realized.saturating_sub(u64::from(c)),
            None => 0,
        }
    }
}

impl Default for ScoredBudget {
    fn default() -> Self {
        Self::unlimited()
    }
}

/// The overshoot distribution over a run's capped decisions — spec §3.4's
/// validity line, which is a DISTRIBUTION rather than a total because one
/// oversized root family says something different from a thousand small ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OvershootSummary {
    /// Capped decisions observed (the denominator; an uncapped run records
    /// none, which is why the summary is an `Option` at the read site).
    pub decisions: u64,
    pub p50: u64,
    pub p90: u64,
    pub max: u64,
}

/// Summarize per-decision overshoots by NEAREST RANK (`index = ⌈q·n⌉ − 1` on
/// the sorted samples) — the repo's existing quantile convention, and the one
/// that keeps p50/p90/max exact on the small integer samples this produces.
///
/// `None` on an empty sample: a run that never capped a decision has no
/// overshoot distribution, and printing zeros for it would claim a measurement
/// that was not taken.
pub(crate) fn summarize_overshoot(samples: &[u64]) -> Option<OvershootSummary> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();
    let at = |q: f64| -> u64 {
        let rank = (q * n as f64).ceil() as usize;
        sorted[rank.clamp(1, n) - 1]
    };
    Some(OvershootSummary {
        decisions: n as u64,
        p50: at(0.5),
        p90: at(0.9),
        max: *sorted.last().expect("non-empty"),
    })
}

/// Spec §3.4's fallback CHAIN, made total: pick a root candidate from the
/// BEST-AVAILABLE values, and if nothing at all was evaluated, draw uniformly
/// over the whole family — **never first-listed**.
///
/// `values[i]` is candidate `i`'s best available value: its continued value
/// where a continuation completed inside the budget, else the mandatory root
/// pass's one-ply value, else `None` for a candidate the budget never reached
/// at all. The rungs, in order:
///
/// 1. the argmax over the present values, ties drawn uniformly from `word`;
/// 2. — which, because rung 1 ranks whatever IS present, already IS
///    "best-evaluated-so-far" when the budget denied some candidates their
///    continuations;
/// 3. an all-`None` slice draws uniformly over the whole family.
///
/// **Rung 3 is unreachable from the capped `lookahead-v1` arm and that is a
/// property of the arm, not of this function**: spec §3.4 makes its root pass
/// mandatory, so every candidate carries at least a one-ply value. It is
/// implemented and pinned here because the chain is specified as total, because
/// the meter's other consumer can deny a root pass, and because "never
/// first-listed" is exactly the ledger-row-18 degeneracy that a `values[0]`
/// fallback would reintroduce through the budget.
///
/// `word` is a KEYED draw (the caller's tie stream), never a stream
/// consumption — constant-consumption at this selecting point like every other.
/// `values` must be non-empty; a decision with no candidates is a caller bug and
/// is refused where the candidates are enumerated.
pub(crate) fn best_available_pick(values: &[Option<i64>], word: u64) -> usize {
    // Release-checked, not debug-only (six-lens N4): both shipped consumers are
    // RELEASE-ONLY paths (`architecture_gate`, `examples/arena`), where a
    // stripped debug_assert would let an empty slice return index 0 and panic
    // out-of-range two frames later at the caller. A named assert here beats
    // that; callers structurally guarantee non-empty (expand_point refuses an
    // empty candidate list), so this costs one branch on a cold path.
    assert!(!values.is_empty(), "a root decision over an empty family is undefined");
    let best = values.iter().flatten().copied().max();
    let Some(best) = best else {
        // Rung 3: nothing evaluated. The draw is over the WHOLE family.
        return uniform_index(word, values.len().max(1));
    };
    let tied: Vec<usize> = values
        .iter()
        .enumerate()
        .filter_map(|(i, v)| (*v == Some(best)).then_some(i))
        .collect();
    tied[uniform_index(word, tied.len())]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_uncapped_budget_never_starts_nothing_and_never_overshoots() {
        let mut b = ScoredBudget::unlimited();
        assert_eq!(b.cap(), None);
        for _ in 0..10_000 {
            assert!(b.may_start(), "an uncapped budget must always admit deeper work");
            assert!(!b.exhausted());
            b.charge();
        }
        assert_eq!(b.realized(), 10_000);
        assert_eq!(b.overshoot(), 0, "an absent cap cannot be overshot");
    }

    /// The cap gates STARTING at `realized < cap`, and charging past it is legal
    /// — which is what makes the mandatory root pass's overrun measurable
    /// instead of a panic.
    #[test]
    fn a_capped_budget_gates_starts_and_measures_its_overrun() {
        let mut b = ScoredBudget::capped(3);
        assert!(b.may_start());
        b.charge();
        b.charge();
        assert!(b.may_start(), "two spent of three still admits work");
        b.charge();
        assert!(b.exhausted(), "the third charge closes the gate");
        assert_eq!(b.overshoot(), 0, "reaching the cap is not overshooting it");
        b.charge();
        b.charge();
        assert_eq!(b.realized(), 5);
        assert_eq!(b.overshoot(), 2);
        assert_eq!(ScoredBudget::from_cap(Some(3)).cap(), Some(3));
        assert_eq!(ScoredBudget::from_cap(None), ScoredBudget::default());
    }

    #[test]
    fn overshoot_is_summarized_by_nearest_rank_and_an_empty_sample_has_no_summary() {
        assert_eq!(summarize_overshoot(&[]), None, "no capped decision, no distribution");
        let s = summarize_overshoot(&[0, 0, 5, 7, 100]).expect("five samples");
        assert_eq!(s.decisions, 5);
        assert_eq!(s.p50, 5, "nearest rank: index ceil(0.5*5)-1 = 2");
        assert_eq!(s.p90, 100, "nearest rank: index ceil(0.9*5)-1 = 4");
        assert_eq!(s.max, 100);
        let one = summarize_overshoot(&[42]).expect("one sample");
        assert_eq!((one.p50, one.p90, one.max), (42, 42, 42));
    }

    /// Rung 1: the argmax over present values, ignoring the unreached ones.
    #[test]
    fn the_chain_ranks_the_values_that_are_present() {
        let values = [Some(3), None, Some(9), Some(-4), None];
        for word in 0..64u64 {
            assert_eq!(
                best_available_pick(&values, word.wrapping_mul(0x9E37_79B9_7F4A_7C15)),
                2,
                "a lone maximiser is picked whatever the keyed word",
            );
        }
    }

    /// Rung 1's tie draw reaches the FULL tied set — a first-listed fallback
    /// would return index 0 for every word.
    #[test]
    fn the_chain_draws_over_the_whole_tied_set() {
        let values = [Some(7), Some(7), Some(2), Some(7), Some(7), Some(7)];
        let mut seen = std::collections::BTreeSet::new();
        for k in 0..200u64 {
            let i = best_available_pick(&values, k.wrapping_mul(0x2545_F491_4F6C_DD1D));
            assert_ne!(i, 2, "a non-maximiser must never be picked");
            seen.insert(i);
        }
        assert_eq!(seen.len(), 5, "all five maximisers must be reachable, saw {seen:?}");
    }

    /// Rung 3: nothing evaluated at all still draws over the WHOLE family, never
    /// index 0. Unreachable from the capped `lookahead-v1` arm by its mandatory
    /// root pass — see the function doc — and pinned here because the chain is
    /// specified total and because a `values[0]` fallback is ledger row 18's
    /// null-placement degeneracy wearing a budget.
    #[test]
    fn the_chain_falls_through_to_a_uniform_draw_over_the_whole_family() {
        let values = [None; 8];
        let mut seen = std::collections::BTreeSet::new();
        for k in 0..400u64 {
            seen.insert(best_available_pick(&values, k.wrapping_mul(0x9E37_79B9_7F4A_7C15)));
        }
        assert_eq!(seen.len(), 8, "every candidate must be reachable, saw {seen:?}");
    }
}
