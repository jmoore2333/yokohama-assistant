//! **§5.4 PART B — the bounded compatibility solve.** Witness search, the exact
//! oracle, deletion-minimization, and the two registered scale diagnostics.
//!
//! Spec: `docs/plans/2026-08-20-phase3.5c-v4-spec-prereg.md` §5.4 (lines
//! 740–806), over the registered rows of §5.2 (484–724) under §5.1's
//! conventions (396–483). §13 names this file: "the solver example — witness +
//! exact oracle + minimization, with its registered unit tests".
//!
//! # The question this answers, and the one it does not
//!
//! Part B asks whether ONE integer weight vector per stage can satisfy every
//! registered pair row at once, inside a declared box, with `w_vp` pinned and
//! V2's hand ordering standing. A SAT is NOT a statement that the architecture
//! is right; it is the statement [`SAT_READING`] prints, and nothing more. An
//! UNSAT is a statement about the system, and §5.4 sends it through
//! deletion-minimization and two scale diagnostics BEFORE anyone is allowed to
//! read it as an architecture result.
//!
//! # Freeze-before-solve (§5.4's closing clause)
//!
//! "The step-2 package's digest precedes the first real-pair invocation;
//! development uses toy systems only. The oracle IMPLEMENTATION freezes at the
//! step-2 commit and every verdict artifact prints its digest." This file was
//! developed and proven against [`toys`] alone. The real system lives behind
//! `solve --real`, which is a separate invocation made after this
//! implementation is frozen and reviewed — see [`IDENTITY_NOTE`].
//!
//! # The trust boundary
//!
//! **Only verified certificates are trusted.** The simplex is a SEARCH; its
//! output is never the answer. An UNSAT ships a Farkas combination that
//! [`verify_farkas`] re-checks in checked i128 INTEGER arithmetic after clearing
//! denominators, reading only the integer system and the multipliers — a pass
//! that shares no code with the search. A SAT ships a point that
//! [`System::violation`] re-checks against every row. If a certificate fails
//! verification the verdict is [`Verdict::Inconclusive`], never UNSAT. That is
//! what lets §5.4 call this "the ONLY authority for infeasibility" while the
//! search itself stays ordinary code.
//!
//! # Commands
//!
//! ```text
//! solver identities                 the identity block and the realized census
//! solver toys                       list the toy systems
//! solver toy <name>                 run one toy end to end
//! solver solve --real               THE REAL RUN (step-4, post-freeze)
//! ```

use std::cmp::Ordering;
use std::process::ExitCode;

use yokohama_core::game::eval::{Feature, WeightTable};
use yokohama_core::game::pair_fixtures::{
    CandidateFeature, FeatureId, MarginClass, PairDeclaration, StageEndpoint, AXIS_WIDTH,
    REGISTERED,
};

// ---------------------------------------------------------------------------
// The registered constants of the arithmetic model (§5.4's first paragraph)
// ---------------------------------------------------------------------------

/// §5.4: "Integer weights, box |w| ≤ 64".
const BOX_LIMIT: i64 = 64;

/// §5.4's box-widening diagnostic: "re-decide at |w| ≤ 128".
const WIDENED_BOX_LIMIT: i64 = 128;

/// §5.4's witness-search step set, IN THE ORDER THE SPEC WRITES IT.
///
/// The order is not decoration: it is the last tie-break, so two steps that
/// leave the same violation at the same `|w|` are separated by it and the
/// search is total. See [`witness_search`].
const STEP_SET: [i64; 10] = [16, -16, 8, -8, 4, -4, 2, -2, 1, -1];

/// §5.4: "≤ 32 sweeps".
const MAX_SWEEPS: usize = 32;

/// The sentence §5.4 requires a SAT to be read as, printed verbatim.
///
/// "a SAT is read as 'no contradiction among the K binding constraints,' never
/// unqualified compatibility (the system is underdetermined by design: 25–27
/// rows vs 33 free)."
const SAT_READING: &str = "READ THIS SAT AS: no contradiction among the K binding constraints. \
     NEVER as unqualified compatibility — the system is underdetermined by design \
     (24-27 rows against 33 free variables).";

// ---------------------------------------------------------------------------
// Exact rationals (§5.4: "i128 rationals, CHECKED operations, gcd-reduced")
// ---------------------------------------------------------------------------

/// An exact rational over `i128`, gcd-reduced with a positive denominator.
///
/// Every operation is CHECKED and returns `None` on overflow, because §5.4 says
/// "ANY overflow aborts to INCONCLUSIVE (never UNSAT)" — so an arithmetic limit
/// has to be a value the caller must handle rather than a panic or a wrap.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Rat {
    n: i128,
    d: i128,
}

impl Rat {
    /// Zero.
    const ZERO: Rat = Rat { n: 0, d: 1 };

    /// `n/d`, reduced, with the sign carried by the numerator. `None` on a zero
    /// denominator or on an overflow inside the reduction.
    fn new(_n: i128, _d: i128) -> Option<Rat> {
        unimplemented!("task A")
    }

    /// An integer as a rational.
    fn int(_n: i128) -> Rat {
        unimplemented!("task A")
    }

    fn add(self, _o: Rat) -> Option<Rat> {
        unimplemented!("task A")
    }

    fn mul(self, _o: Rat) -> Option<Rat> {
        unimplemented!("task A")
    }

    /// `-1`, `0` or `+1`.
    fn sign(self) -> i32 {
        unimplemented!("task A")
    }

    fn is_zero(self) -> bool {
        self.sign() == 0
    }

    /// Exact comparison. `None` on an overflow in the cross-multiplication.
    fn cmp_exact(self, _o: Rat) -> Option<Ordering> {
        unimplemented!("task A")
    }
}

// ---------------------------------------------------------------------------
// The row model
// ---------------------------------------------------------------------------

/// What a row IS, which decides whether deletion-minimization may remove it.
///
/// §5.4: "deletion-minimization ... over the DELETABLE tier — the registered
/// pair rows ... and, recorded separately, the hand-ordering rows; the `w_vp`
/// pins and the box are NOT deletion-eligible".
#[derive(Clone, Debug, PartialEq, Eq)]
enum RowKind {
    /// A registered pair row. `order` is its §5.2 DOCUMENT-ORDER index — see
    /// [`document_order`].
    Pair { id: String, order: usize },
    /// One of V2's two hand-ordering rows.
    HandOrdering,
    /// A box bound, `w_f ≥ -limit` or `-w_f ≥ -limit`. Not deletion-eligible.
    BoxBound,
    /// A half of the `w_vp` equality pin. Not deletion-eligible.
    Pin,
}

/// One linear inequality `Σ_f coef[f] · w_f ≥ rhs`, in integer units.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Row {
    label: String,
    kind: RowKind,
    coef: Vec<i64>,
    rhs: i64,
}

/// One stage's constraint system.
///
/// §5.4: "The system DECOMPOSES: no constraint couples the stages ... witness
/// search, oracle, and minimization run PER STAGE over 34 variables (33 free
/// after the pin)."
#[derive(Clone, Debug)]
struct System {
    stage: StageEndpoint,
    /// The DELETABLE tier in DELETION ORDER: pair rows in §5.2 document order,
    /// then the two hand-ordering rows.
    rows: Vec<Row>,
    /// Per-coordinate box. `w_vp`'s interval is degenerate — that IS the pin.
    lo: Vec<i64>,
    hi: Vec<i64>,
    /// The pinned `w_vp` value, kept beside the box so the early-rescale
    /// diagnostic can move it by name rather than by index surgery.
    pin_vp: i64,
    box_limit: i64,
}

// ---------------------------------------------------------------------------
// Margin extraction (§5.1's margin classes)
// ---------------------------------------------------------------------------

/// **THE MARGIN EXTRACTION RULE**, and it is one line: a registered pair's
/// margin at an endpoint is `MarginClass::margin_at`.
///
/// §5.1 (lines 428–439) defines exactly three classes and `margin_at`
/// (`game/pair_fixtures.rs:425–434`) is their transcription:
///
/// * MARGINED — "a registered INTEGER `max(1, ceil(margin_vp × w_vp(stage)))`
///   with a NAMED CARRIER", stored per endpoint.
/// * DIRECTION-ONLY — 1.
/// * WEAK — 0 (family 7's anti-parking row, the only one).
///
/// **Totality is guaranteed, not assumed.** `PairDeclaration::check_consistency`
/// (`game/pair_fixtures.rs:843–851`) already refuses a declaration that declares
/// an endpoint without pricing it, and `PairFixture::new` runs it as clause 0.
/// So the `None` arm here is unreachable through the registry, and it is a typed
/// refusal rather than a default precisely because a defaulted margin would be a
/// silently weaker row.
fn margin_of(_d: &PairDeclaration, _stage: StageEndpoint) -> Result<i64, String> {
    unimplemented!("task A")
}

/// The §5.2 DOCUMENT-ORDER index of a registered pair — the deletion order.
///
/// §5.4 fixes the deletion order as "the order the rows appear in §5.2's printed
/// text, top to bottom, then stage — the literal document order, immune to
/// lettering quirks". [`REGISTERED`] is already in that order (its family-2 run
/// is 2(a), 2(b), 2(d), 2(c), exactly as §5.2 prints it, and family 10 keeps
/// 10(c) between (b) and (d)), so the array index IS the document order and no
/// string sort can drift from it.
fn document_order(id: &str) -> Option<usize> {
    REGISTERED.iter().position(|p| p.id == id)
}

// ---------------------------------------------------------------------------
// The census (§5.4's realized-vs-frozen cross-check)
// ---------------------------------------------------------------------------

/// §5.4's frozen count of EARLY pair rows.
const FROZEN_EARLY: usize = 25;

/// §5.4's frozen count of LATE pair rows.
const FROZEN_LATE: usize = 27;

/// The ONE registered departure from §5.4's frozen census, cited rather than
/// inferred.
const CENSUS_RULING: &str = "4(c) HIGHER FACES was ruled LATE-ONLY by the owner on 2026-08-22 \
     (guidingpath decision-log row; the declaration carries the ruling in its own notes). Its \
     early row named an unreachable state: §5.1's p = 0 caps the hand at the single dealt \
     starting order, and no ONE card can differ in face while holding the requirement vector \
     AND the icon faces equal. The frozen text's both-endpoints 18 becomes 17 and its late-only \
     9 becomes 10, so EARLY moves 25 -> 24 and LATE is UNCHANGED at 27. This is the ruling's \
     footprint and nothing else -- NOT a registry defect, and §5.4's defect clause is wired not \
     to fire on it.";

/// Realized constraint-row counts, per stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Census {
    early: usize,
    late: usize,
}

/// Count the realized `PairRole::Constraint` rows per stage, off the
/// declarations alone.
fn realized_census() -> Census {
    unimplemented!("task A")
}

// ---------------------------------------------------------------------------
// The witness search (§5.4 (1))
// ---------------------------------------------------------------------------

/// What one witness search found.
#[derive(Clone, Debug)]
struct WitnessReport {
    w: Vec<i64>,
    violation: i128,
    sweeps: usize,
    /// Rows with slack 0 at the returned point — §5.4's "binding-constraint
    /// count K". Meaningful only when `violation == 0`.
    binding: Vec<String>,
    /// Per-row `Σ coef·w − rhs`, in row order.
    slack: Vec<i128>,
}

impl System {
    /// `Σ_f coef[f]·w[f] − rhs` for one row, in i128.
    fn slack_of(_row: &Row, _w: &[i64]) -> i128 {
        unimplemented!("task A")
    }

    /// §5.4's total violation: `Σ_c max(0, margin_c − Σ_f w_f·Δv_f(c))`, over
    /// the DELETABLE tier only — the box is enforced structurally by the search
    /// rather than priced into the objective.
    fn violation(&self, _w: &[i64]) -> i128 {
        unimplemented!("task A")
    }

    /// §5.4's "init = V2-padded (incumbents at V2, candidates 0)".
    fn v2_padded(&self) -> Vec<i64> {
        unimplemented!("task A")
    }
}

/// **§5.4 (1) — the witness search.** Deterministic coordinate descent from the
/// V2-padded start.
///
/// # The reading of §5.4's sentence, stated because two readings exist
///
/// §5.4: "coordinate order = `Feature::ALL` order (the tie-break's 'listed
/// order'); at each coordinate try ALL of {±16, ±8, ±4, ±2, ±1}, take the best
/// strict decrease, ties toward smaller |w| then listed order; a full
/// no-improvement sweep ends the search".
///
/// Implemented as SEQUENTIAL sweeps: one pass walks the coordinates in
/// [`FeatureId::all`] index order (the sixteen incumbents in `Feature::ALL`
/// order, then the eighteen candidates in `CandidateFeature::ALL` order — the
/// axis §3.2 declares), and AT each coordinate applies the best strictly
/// decreasing step before moving on. A sweep that applies nothing ends the
/// search. The alternative reading — one best-improvement move per sweep, over
/// all coordinates — is grammatical too, but "at each coordinate ... take" puts
/// the action at the coordinate, and 32 single moves could not cross a
/// 33-variable system.
///
/// **The reading cannot make this dishonest.** The search only ever PROVES
/// feasibility, and it proves it by producing a point that [`System::violation`]
/// re-checks against every row. Under either reading a zero-violation return is
/// a witness; the worst a wrong reading can do is fail to find one, and then the
/// exact oracle decides. Infeasibility is never this function's to claim.
///
/// **Tie-breaks, total by construction:** strictly lower violation wins; equal
/// violation goes to the smaller resulting `|w_f|`; still equal goes to the
/// earlier entry of [`STEP_SET`]. A step leaving the box is not a candidate, so
/// the pin holds by the box being degenerate at `w_vp`.
fn witness_search(_sys: &System) -> WitnessReport {
    unimplemented!("task A")
}

// ---------------------------------------------------------------------------
// Toy systems (§5.4's "development uses toy systems only")
// ---------------------------------------------------------------------------

/// The toy systems, over the REAL 34-wide axis so the machinery under test is
/// the machinery that will run — only the row set is small.
mod toys {
    use super::*;

    /// Build a toy over the real axis: `rows` are `(label, [(index, coef)], rhs)`.
    pub fn toy(
        _stage: StageEndpoint,
        _rows: &[(&str, &[(usize, i64)], i64)],
        _limit: i64,
    ) -> System {
        unimplemented!("task A")
    }

    /// The axis index of a candidate column, by its §5 label.
    pub fn col(label: &str) -> usize {
        FeatureId::Candidate(
            CandidateFeature::ALL
                .into_iter()
                .find(|c| c.label() == label)
                .expect("a registered candidate label"),
        )
        .index()
    }
}

fn main() -> ExitCode {
    eprintln!("solver: under construction (task A)");
    ExitCode::FAILURE
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::toys::{col, toy};
    use super::*;
    use yokohama_core::game::pair_fixtures::PairRole;

    // -- exact rationals ---------------------------------------------------

    #[test]
    fn rationals_reduce_and_carry_the_sign_in_the_numerator() {
        assert_eq!(Rat::new(4, -6), Rat::new(-2, 3));
        assert_eq!(Rat::new(0, -5), Some(Rat::ZERO));
        assert_eq!(Rat::new(1, 0), None, "a zero denominator is not a rational");
        let r = Rat::new(-8, 12).expect("reduces");
        assert!(r.d > 0, "the denominator is normalized positive, got {}", r.d);
        assert_eq!((r.n, r.d), (-2, 3));
    }

    #[test]
    fn rational_arithmetic_is_checked_and_returns_none_on_overflow() {
        let big = Rat::new(i128::MAX, 1).expect("representable");
        assert_eq!(big.add(big), None, "an overflowing add is None, never a wrap");
        assert_eq!(big.mul(big), None, "an overflowing mul is None, never a wrap");
        let third = Rat::new(1, 3).expect("representable");
        assert_eq!(third.add(third).and_then(|r| r.add(third)), Some(Rat::int(1)));
    }

    #[test]
    fn rational_comparison_is_exact_where_floating_point_would_tie() {
        let a = Rat::new(1, 3).expect("representable");
        let b = Rat::new(100_000_000_000_000_000, 300_000_000_000_000_001).expect("representable");
        assert_eq!(a.cmp_exact(b), Some(Ordering::Greater));
        assert_eq!(b.cmp_exact(a), Some(Ordering::Less));
        assert_eq!(a.cmp_exact(a), Some(Ordering::Equal));
    }

    // -- the margin rule ---------------------------------------------------

    #[test]
    fn the_margin_rule_is_margin_at_and_nothing_else() {
        for pair in REGISTERED.iter() {
            for stage in pair.stages {
                let got = margin_of(pair, *stage).expect("a declared endpoint prices its margin");
                let want = pair
                    .margin
                    .margin_at(*stage)
                    .expect("check_consistency guarantees a declared endpoint is priced");
                assert_eq!(got, want, "{} at {}", pair.id, stage.name());
            }
        }
    }

    #[test]
    fn direction_only_is_one_and_weak_is_zero_at_every_declared_endpoint() {
        for pair in REGISTERED.iter() {
            for stage in pair.stages {
                let m = margin_of(pair, *stage).expect("priced");
                match pair.margin {
                    MarginClass::DirectionOnly => {
                        assert_eq!(m, 1, "{} DIRECTION-ONLY is 1 (§5.1:435)", pair.id)
                    }
                    MarginClass::Weak => {
                        assert_eq!(m, 0, "{} WEAK is 0 (§5.1:435)", pair.id)
                    }
                    MarginClass::Margined { .. } => assert!(m >= 1, "{}", pair.id),
                }
            }
        }
    }

    /// §5.1's MARGINED formula, re-derived from §5.2's declared VP fractions
    /// against the registry's integers.
    ///
    /// This is the pin that makes [`margin_of`] a RULE rather than a lookup: the
    /// six fractions are transcribed from §5.2's own prose, run through
    /// `max(1, ceil(margin_vp × w_vp(stage)))` with the pinned `w_vp = (1, 40)`,
    /// and compared with what the registry registered. A mis-registered integer
    /// fails here rather than becoming a quietly wrong row.
    #[test]
    fn every_margined_integer_reconciles_with_the_five_one_formula() {
        // (pair, margin_vp as num/den, §5.2's own words for the rate)
        let registered_rates: [(&str, i64, i64); 6] = [
            ("1(a)", 1, 3),  // "MARGINED (amortized 1/3 VP/good)", one good
            ("2(a)", 2, 3),  // "carrier E2 via the amortized 1/3 VP/good", goods +2
            ("3(a)", 1, 2),  // "MARGINED (amortized 1/2 VP/yen)", one yen
            ("3(b)", 1, 3),  // "MARGINED", one good at the 1/3 rate
            ("3(c)", 1, 1),  // "amortized, one activation's 2 yen ~ 1 VP"
            ("4(d)", 4, 1),  // "margin_vp = 4"
        ];
        let w_vp = |stage: StageEndpoint| match stage {
            StageEndpoint::Early => 1i64,
            StageEndpoint::Late => 40i64,
        };
        for (id, num, den) in registered_rates {
            let pair = REGISTERED.iter().find(|p| p.id == id).expect("registered");
            assert!(
                matches!(pair.margin, MarginClass::Margined { .. }),
                "{id} is registered MARGINED in §5.2",
            );
            for stage in pair.stages {
                let scaled_num = num * w_vp(*stage);
                let ceiling = scaled_num.div_euclid(den) + i64::from(scaled_num.rem_euclid(den) > 0);
                let want = ceiling.max(1);
                assert_eq!(
                    margin_of(pair, *stage).expect("priced"),
                    want,
                    "{id} at {}: §5.1's max(1, ceil({num}/{den} x {})) is {want}",
                    stage.name(),
                    w_vp(*stage),
                );
            }
        }
    }

    #[test]
    fn the_early_margined_floor_collapses_onto_direction_only() {
        // §5.1: "The early floor is a STRENGTHENING (1/3-VP -> 1 is 3x; 1/2-VP
        // -> 1 is 2x): early MARGINED == DIRECTION-ONLY in force". That is a
        // claim about this registry, so it is checked against it.
        for pair in REGISTERED.iter() {
            if matches!(pair.margin, MarginClass::Margined { .. })
                && pair.stages.contains(&StageEndpoint::Early)
            {
                assert_eq!(
                    margin_of(pair, StageEndpoint::Early).expect("priced"),
                    1,
                    "{} early",
                    pair.id,
                );
            }
        }
    }

    // -- the census --------------------------------------------------------

    #[test]
    fn the_realized_census_is_early_twenty_four_late_twenty_seven() {
        assert_eq!(realized_census(), Census { early: 24, late: 27 });
    }

    #[test]
    fn the_census_gap_against_the_frozen_text_is_exactly_four_c() {
        let realized = realized_census();
        assert_eq!(
            realized.late, FROZEN_LATE,
            "the 4(c) ruling moves no LATE row: both-endpoints 18->17 and late-only 9->10",
        );
        assert_eq!(FROZEN_EARLY - realized.early, 1, "one early row, not a class of them");
        let four_c = REGISTERED.iter().find(|p| p.id == "4(c)").expect("registered");
        assert_eq!(
            four_c.stages,
            &[StageEndpoint::Late],
            "the missing early row is 4(c)'s, per the 2026-08-22 ruling",
        );
        // And no OTHER pair the frozen text lists at both endpoints has lost one.
        let both: usize = REGISTERED
            .iter()
            .filter(|p| matches!(p.role, PairRole::Constraint) && p.stages.len() == 2)
            .count();
        assert_eq!(both, 17, "the frozen text's 18 both-endpoint pairs, less 4(c)");
    }

    // -- the witness search ------------------------------------------------

    fn sat_toy() -> System {
        toy(
            StageEndpoint::Early,
            &[
                ("S1 productive", &[(col("S1"), 1)], 1),
                ("E1 money", &[(col("E1"), 1)], 3),
                ("E2 over S1", &[(col("E2"), 1), (col("S1"), -1)], 1),
            ],
            BOX_LIMIT,
        )
    }

    #[test]
    fn the_witness_search_starts_v2_padded() {
        let sys = sat_toy();
        let start = sys.v2_padded();
        assert_eq!(start.len(), AXIS_WIDTH);
        for f in Feature::ALL {
            assert_eq!(
                start[f.index()],
                WeightTable::V2.early[f.index()],
                "incumbent {} starts at V2",
                f.name(),
            );
        }
        for c in CandidateFeature::ALL {
            assert_eq!(
                start[FeatureId::Candidate(c).index()],
                0,
                "candidate {} starts at 0",
                c.label(),
            );
        }
    }

    #[test]
    fn the_witness_search_finds_a_reachable_witness_and_verifies_it() {
        let sys = sat_toy();
        let report = witness_search(&sys);
        assert_eq!(report.violation, 0, "the toy is satisfiable from the V2-padded start");
        assert_eq!(
            sys.violation(&report.w),
            0,
            "the returned point is re-checked against every row, not trusted",
        );
        assert!(report.sweeps <= MAX_SWEEPS);
    }

    #[test]
    fn the_witness_search_is_deterministic() {
        let sys = sat_toy();
        let a = witness_search(&sys);
        let b = witness_search(&sys);
        assert_eq!(a.w, b.w);
        assert_eq!(a.violation, b.violation);
        assert_eq!(a.sweeps, b.sweeps);
        assert_eq!(a.binding, b.binding);
    }

    #[test]
    fn the_witness_search_never_leaves_the_box_and_never_moves_the_pin() {
        let sys = sat_toy();
        let report = witness_search(&sys);
        for (i, &w) in report.w.iter().enumerate() {
            assert!(
                w >= sys.lo[i] && w <= sys.hi[i],
                "coordinate {i} left the box: {w} outside [{}, {}]",
                sys.lo[i],
                sys.hi[i],
            );
        }
        assert_eq!(
            report.w[Feature::VictoryPoints.index()],
            sys.pin_vp,
            "the w_vp pin is the box being degenerate, so no step can move it",
        );
    }

    #[test]
    fn the_witness_search_reports_binding_rows_and_a_slack_profile() {
        let sys = sat_toy();
        let report = witness_search(&sys);
        assert_eq!(report.slack.len(), sys.rows.len(), "one slack per deletable row");
        for (row, slack) in sys.rows.iter().zip(&report.slack) {
            assert_eq!(*slack, System::slack_of(row, &report.w));
            assert!(*slack >= 0, "a witness leaves no row violated");
        }
        let expected: Vec<String> = sys
            .rows
            .iter()
            .zip(&report.slack)
            .filter(|(_, s)| **s == 0)
            .map(|(r, _)| r.label.clone())
            .collect();
        assert_eq!(report.binding, expected, "K is the slack-0 count, computed not asserted");
    }

    #[test]
    fn an_unsatisfiable_toy_ends_the_search_without_claiming_anything() {
        // The witness search never reports infeasibility -- it reports a
        // positive residual and stops. Only the exact oracle may say UNSAT.
        let sys = toy(
            StageEndpoint::Early,
            &[
                ("S1 up", &[(col("S1"), 1)], 1),
                ("S1 down", &[(col("S1"), -1)], 1),
            ],
            BOX_LIMIT,
        );
        let report = witness_search(&sys);
        assert!(report.violation > 0, "no point satisfies both rows");
        assert!(report.sweeps <= MAX_SWEEPS, "the sweep cap is honoured");
    }

    #[test]
    fn the_sat_reading_sentence_is_the_one_the_spec_registers() {
        assert!(SAT_READING.contains("no contradiction among the K binding constraints"));
        assert!(SAT_READING.contains("NEVER as unqualified compatibility"));
    }

    // -- document order ----------------------------------------------------

    #[test]
    fn document_order_is_the_registry_order_and_matches_five_two_s_printed_text() {
        // §5.2 prints family 2 as (a), (b), (d), then the (c) diagnostic, and
        // family 10 as (a), (b), (c), (d). The array order carries both, which
        // is why the deletion order reads the array rather than sorting ids.
        assert!(document_order("2(b)") < document_order("2(d)"));
        assert!(document_order("2(d)") < document_order("2(c)"));
        assert!(document_order("10(b)") < document_order("10(c)"));
        assert!(document_order("10(c)") < document_order("10(d)"));
        assert_eq!(document_order("1(a)"), Some(0));
        assert_eq!(document_order("nope"), None);
        let mut seen: Vec<usize> =
            REGISTERED.iter().map(|p| document_order(p.id).expect("registered")).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), REGISTERED.len(), "the order is a bijection onto the registry");
    }
}
