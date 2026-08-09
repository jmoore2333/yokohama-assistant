//! The candidate-vs-incumbent WEIGHT SCREEN (Phase 3.5c, v2 protocol §4.2's L4).
//!
//! SKELETON — every entry point is `unimplemented!()`. The tests below are the
//! specification and they are RED by construction; the implementation lands in
//! the next commit.

use std::collections::BTreeMap;

use crate::game::arena::{ArenaError, IntervalOutcome, SeedScheduleKind};
use crate::game::eval::WeightTable;
use crate::game::rules::Rules;

/// Which evaluator rung a screen runs at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenRung {
    /// Greedy-v1's one-ply argmax at its shipped retreat-sample knob.
    Greedy,
    /// Lookahead-v1 at its shipped knobs — the S1 turn-boundary rung, UNCAPPED.
    Lookahead,
}

/// One screen: a candidate weight table against an incumbent one, at a rung.
#[derive(Debug, Clone)]
pub struct ScreenSpec {
    pub name: String,
    pub rung: ScreenRung,
    pub candidate: WeightTable,
    pub incumbent: WeightTable,
    pub players: u8,
    pub units: u32,
    pub master: SeedScheduleKind,
}

impl ScreenSpec {
    /// The provenance this spec WILL carry, when its master is already known.
    pub fn provenance(&self) -> Option<ScreenProvenance> {
        unimplemented!("ScreenSpec::provenance")
    }
}

/// The T16 composite key, made unavoidable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenProvenance {
    pub screen: String,
    pub rung: ScreenRung,
    pub controller: String,
    pub controller_version: u32,
    pub candidate_params: BTreeMap<String, i64>,
    pub incumbent_params: BTreeMap<String, i64>,
    pub candidate_identity: i64,
    pub incumbent_identity: i64,
    pub semantics_rev: u32,
    pub eval_impl_version: u32,
    pub players: u8,
    pub units: u32,
    pub games: u32,
    pub master: u64,
    pub master_kind: &'static str,
    pub resample_seed: u64,
    pub setup_identical: bool,
}

impl ScreenProvenance {
    pub fn render(&self) -> String {
        unimplemented!("ScreenProvenance::render")
    }
}

/// Where the screen's interval sits relative to the cell's null.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullPosition {
    Above,
    Below,
    Spans,
}

/// One screened cell.
#[derive(Debug, Clone)]
pub struct ScreenReport {
    pub provenance: ScreenProvenance,
    pub games: u32,
    pub null: f64,
    pub candidate_unit_shares: Vec<f64>,
    pub theta_hat: f64,
    pub interval: IntervalOutcome,
    pub position: NullPosition,
    pub unit_sd: f64,
    pub unit_se: f64,
    pub candidate_applied_moves: u64,
    pub candidate_explanations: u64,
    pub incumbent_applied_moves: u64,
    pub incumbent_explanations: u64,
    pub seat0_wins: u32,
    pub vp_margin_mean: f64,
    pub candidate_hand_split_ok: bool,
    pub incumbent_hand_split_ok: bool,
}

impl ScreenReport {
    pub fn render(&self) -> String {
        unimplemented!("ScreenReport::render")
    }

    pub fn direction_above(&self) -> bool {
        unimplemented!("ScreenReport::direction_above")
    }

    pub fn entirely_below_null(&self) -> bool {
        unimplemented!("ScreenReport::entirely_below_null")
    }
}

/// L14/T1's one-ply non-degeneracy inequality, in BOTH vectors.
pub fn hand_split_ordering_holds(_weights: &WeightTable) -> bool {
    unimplemented!("hand_split_ordering_holds")
}

/// Run one screen with per-move invariant checking ON.
pub fn run_screen(_spec: &ScreenSpec, _rules: &Rules) -> Result<ScreenReport, ArenaError> {
    unimplemented!("run_screen")
}

/// [`run_screen`] with per-move `check_invariants` as a parameter.
pub fn run_screen_with(
    _spec: &ScreenSpec,
    _rules: &Rules,
    _invariants: bool,
) -> Result<ScreenReport, ArenaError> {
    unimplemented!("run_screen_with")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::eval::Feature;
    use crate::game::greedy::weight_table_identity;
    use crate::game::semantics::SEMANTICS_REV;

    /// This module's committed diagnostic master: the arena `A3E1` family, date
    /// field `2026_0809`, serial `0001` — the first free serial in that date
    /// field at authoring (grep-verified: no `0xA3E1_2026_0809_*` literal
    /// existed in the crate).
    const MASTER: u64 = 0xA3E1_2026_0809_0001;

    /// The deliberately WEAKENED candidate: [`WeightTable::V1`] with the EARLY
    /// vector halved.
    ///
    /// Chosen against map §8 row 18's warning that a constant-valued table
    /// degenerates play (the all-empty-placement game): halving preserves every
    /// sign and every ordering — no entry reaches 0 — so the evaluator still
    /// discriminates. It is also the protocol's own primary re-pricing target
    /// (§5 T4, early-vector mixing), so the mutation is the shape a real round
    /// would propose rather than an arbitrary perturbation.
    fn halved_early(base: WeightTable) -> WeightTable {
        let mut out = base;
        for w in out.early.iter_mut() {
            *w /= 2;
        }
        out
    }

    fn spec(name: &str, rung: ScreenRung, candidate: WeightTable, units: u32) -> ScreenSpec {
        ScreenSpec {
            name: name.to_string(),
            rung,
            candidate,
            incumbent: WeightTable::V1,
            players: 2,
            units,
            master: SeedScheduleKind::Diagnostic(MASTER),
        }
    }

    // -----------------------------------------------------------------------
    // (a) the NULL cell
    // -----------------------------------------------------------------------

    #[test]
    fn a_null_screen_reports_equal_identities_and_a_usable_estimate() {
        let rules = Rules::standard();
        let s = spec("null-greedy", ScreenRung::Greedy, WeightTable::V1, 2);
        let report = run_screen(&s, &rules).expect("null screen runs");

        assert_eq!(
            report.provenance.candidate_identity, report.provenance.incumbent_identity,
            "V1 vs V1 must report the SAME weight identity on both sides",
        );
        assert_eq!(report.games, 4, "2 units at 2P is 4 games");
        assert_eq!(report.candidate_unit_shares.len(), 2, "per-unit values must exist");

        // The CRN-collapse detector shape: either a real spread, or the typed
        // refusal — never a crash and never a printed +-0.000.
        match report.interval {
            IntervalOutcome::Bootstrap { lo, hi, .. } => {
                assert!(report.unit_se > 0.0, "a bootstrap interval needs a nonzero SE");
                assert!(lo <= hi, "interval endpoints ordered");
            }
            IntervalOutcome::RefusedZeroWidth { .. } => {
                assert_eq!(report.unit_sd, 0.0, "a refusal means the unit vector is constant");
            }
        }
        println!("{}", report.render());
    }

    #[test]
    fn the_lookahead_rung_runs_the_same_null_at_smoke_scale() {
        let rules = Rules::standard();
        let s = spec("null-lookahead", ScreenRung::Lookahead, WeightTable::V1, 1);
        let report = run_screen(&s, &rules).expect("lookahead null screen runs");

        assert_eq!(
            report.provenance.candidate_identity, report.provenance.incumbent_identity,
            "V1 vs V1 must report the SAME weight identity on both sides",
        );
        assert_eq!(report.games, 2);
        // Spec §0.3(b)'s identity, on BOTH sides — both agents explain here.
        assert_eq!(report.candidate_explanations, report.candidate_applied_moves);
        assert_eq!(report.incumbent_explanations, report.incumbent_applied_moves);
        println!("{}", report.render());
    }

    // -----------------------------------------------------------------------
    // (b) the DISCRIMINATING smoke
    // -----------------------------------------------------------------------

    #[test]
    fn b_a_weakened_candidate_screens_against_v1_with_distinct_identities() {
        let rules = Rules::standard();
        let weakened = halved_early(WeightTable::V1);

        // Non-degeneracy, asserted rather than asserted-in-prose.
        assert!(hand_split_ordering_holds(&weakened));
        assert!(hand_split_ordering_holds(&WeightTable::V1));
        for f in Feature::ALL {
            let (ce, _) = weakened.get(f);
            let (ie, _) = WeightTable::V1.get(f);
            assert_eq!(ce.signum(), ie.signum(), "halving must preserve {f:?}'s sign");
            assert_ne!(ce, 0, "halving must not zero {f:?}");
        }
        assert_ne!(weight_table_identity(&weakened), weight_table_identity(&WeightTable::V1));

        let s = spec("weakened-early-greedy", ScreenRung::Greedy, weakened, 2);
        let report = run_screen(&s, &rules).expect("weakened screen runs");

        assert_ne!(
            report.provenance.candidate_identity, report.provenance.incumbent_identity,
            "a weakened candidate must report a DIFFERENT identity from the incumbent",
        );
        assert!(report.candidate_hand_split_ok && report.incumbent_hand_split_ok);
        // NO DIRECTION is asserted at smoke N — the value here is that the
        // instrument runs and separates the two tables.
        println!("{}", report.render());
    }

    // -----------------------------------------------------------------------
    // (c) determinism
    // -----------------------------------------------------------------------

    #[test]
    fn c_the_same_master_reproduces_the_report_byte_identically() {
        let rules = Rules::standard();
        let s = spec("determinism", ScreenRung::Greedy, halved_early(WeightTable::V1), 1);
        let first = run_screen(&s, &rules).expect("first run").render();
        let second = run_screen(&s, &rules).expect("second run").render();
        assert_eq!(first, second, "same master, same report");

        // Anti-vacuity: a different master must move the rendering, or the pin
        // above would hold over a report that says nothing about the games.
        let mut flipped = s.clone();
        flipped.master = SeedScheduleKind::Diagnostic(MASTER ^ 0x11);
        let other = run_screen(&flipped, &rules).expect("flipped run").render();
        assert_ne!(first, other, "a flipped master must change the report");
    }

    // -----------------------------------------------------------------------
    // (d) provenance
    // -----------------------------------------------------------------------

    #[test]
    fn d_provenance_names_both_identities_the_rev_and_the_rung_knobs() {
        for rung in [ScreenRung::Greedy, ScreenRung::Lookahead] {
            let s = spec("prov", rung, halved_early(WeightTable::V1), 4);
            let p = s.provenance().expect("a diagnostic spec knows its master");
            let rendered = p.render();

            assert_eq!(p.semantics_rev, SEMANTICS_REV);
            assert_eq!(p.candidate_identity, weight_table_identity(&halved_early(WeightTable::V1)));
            assert_eq!(p.incumbent_identity, weight_table_identity(&WeightTable::V1));
            assert_ne!(p.candidate_identity, p.incumbent_identity);
            assert!(rendered.contains(&format!("semantics_rev={SEMANTICS_REV}")));
            assert!(rendered.contains(&format!("weights={}", p.candidate_identity)));
            assert!(rendered.contains(&format!("weights={}", p.incumbent_identity)));
            assert!(rendered.contains("units=4"));
            assert!(rendered.contains(&format!("master=diagnostic:{MASTER:#018x}")));
            assert!(rendered.contains("setup-identical=yes"));

            match rung {
                ScreenRung::Greedy => {
                    assert_eq!(p.controller, "greedy-v1");
                    assert!(rendered.contains("m=8"));
                }
                ScreenRung::Lookahead => {
                    assert_eq!(p.controller, "lookahead-v1");
                    assert!(rendered.contains("b=4"));
                    assert!(rendered.contains("k=1"));
                    assert!(rendered.contains("m=8"));
                }
            }
        }
    }
}
