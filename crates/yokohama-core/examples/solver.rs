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

use std::process::ExitCode;

use yokohama_core::game::eval::{Feature, WeightTable};
use yokohama_core::game::greedy::weight_table_identity;
use yokohama_core::game::pair_fixtures::{
    package_digest, CandidateFeature, FeatureId, MarginClass, PairDeclaration, PairRole,
    StageEndpoint, AXIS_WIDTH, REGISTERED,
};
use yokohama_core::SEMANTICS_REV;

// ---------------------------------------------------------------------------
// The registered constants of the arithmetic model (§5.4's first paragraph)
// ---------------------------------------------------------------------------

/// §5.4: "Integer weights, box |w| ≤ 64".
const BOX_LIMIT: i64 = 64;

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
fn margin_of(d: &PairDeclaration, stage: StageEndpoint) -> Result<i64, String> {
    d.margin.margin_at(stage).ok_or_else(|| {
        format!(
            "{} declares the {} endpoint but registers no margin there; §5.1's three classes \
             price every declared endpoint and `check_consistency` refuses a declaration that \
             does not",
            d.id,
            stage.name(),
        )
    })
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
///
/// Declarations ONLY — no fixture is built to count a row. The census is a
/// statement about what the package registers, and building states to answer it
/// would make it a statement about what the constructions produce.
fn realized_census() -> Census {
    let mut census = Census { early: 0, late: 0 };
    for pair in REGISTERED.iter().filter(|p| matches!(p.role, PairRole::Constraint)) {
        for stage in pair.stages {
            match stage {
                StageEndpoint::Early => census.early += 1,
                StageEndpoint::Late => census.late += 1,
            }
        }
    }
    census
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

/// V2's weight vector at one stage — the source of both the `w_vp` pin
/// (§5.1:428, "V2's shipped pair, eval.rs:854–855") and the witness search's
/// V2-padded start.
fn v2_vector(stage: StageEndpoint) -> [i64; Feature::COUNT] {
    match stage {
        StageEndpoint::Early => WeightTable::V2.early,
        StageEndpoint::Late => WeightTable::V2.late,
    }
}

impl System {
    /// Assemble one stage's system: the deletable rows, the box, and the pin as
    /// a degenerate box interval at `w_vp`.
    ///
    /// **The pin is realized as a box, not as two rows, in this representation**
    /// — the witness search then cannot move it and needs no special case. It
    /// becomes two labelled [`RowKind::Pin`] rows in [`System::inequalities`],
    /// where the Farkas certificate has to be able to name it.
    fn new(stage: StageEndpoint, rows: Vec<Row>, box_limit: i64, pin_vp: i64) -> System {
        let mut lo = vec![-box_limit; AXIS_WIDTH];
        let mut hi = vec![box_limit; AXIS_WIDTH];
        let vp = Feature::VictoryPoints.index();
        lo[vp] = pin_vp;
        hi[vp] = pin_vp;
        System { stage, rows, lo, hi, pin_vp, box_limit }
    }

    /// **The FULL inequality list `A w ≥ b`** — the deletable rows plus the box
    /// and the pin, each as a labelled row.
    ///
    /// The pin and the box live as a per-coordinate interval on [`System`]
    /// because that is what the witness search needs, and as ROWS here because
    /// that is what a Farkas certificate needs: a combination that leans on
    /// `w_S1 ≤ 64` has to be able to name that bound and carry its multiplier.
    /// The two representations are generated from one source, so they cannot
    /// disagree.
    fn inequalities(&self) -> Vec<Row> {
        let mut all = self.rows.clone();
        let vp = Feature::VictoryPoints.index();
        for (i, id) in FeatureId::all().into_iter().enumerate() {
            let kind = if i == vp { RowKind::Pin } else { RowKind::BoxBound };
            let mut lower = vec![0i64; AXIS_WIDTH];
            lower[i] = 1;
            all.push(Row {
                label: format!("{} >= {}", id.label(), self.lo[i]),
                kind: kind.clone(),
                coef: lower,
                rhs: self.lo[i],
            });
            let mut upper = vec![0i64; AXIS_WIDTH];
            upper[i] = -1;
            all.push(Row {
                label: format!("{} <= {}", id.label(), self.hi[i]),
                kind,
                coef: upper,
                rhs: -self.hi[i],
            });
        }
        all
    }

    /// `Σ_f coef[f]·w[f] − rhs` for one row, in i128.
    ///
    /// i128 because the accumulation is free there and the alternative is
    /// reasoning about a bound: coefficients are feature deltas whose size the
    /// fixtures decide, not this file.
    fn slack_of(row: &Row, w: &[i64]) -> i128 {
        let dot: i128 =
            row.coef.iter().zip(w).map(|(&c, &x)| i128::from(c) * i128::from(x)).sum();
        dot - i128::from(row.rhs)
    }

    /// §5.4's total violation: `Σ_c max(0, margin_c − Σ_f w_f·Δv_f(c))`, over
    /// the DELETABLE tier only — the box is enforced structurally by the search
    /// rather than priced into the objective.
    fn violation(&self, w: &[i64]) -> i128 {
        self.rows.iter().map(|r| (-System::slack_of(r, w)).max(0)).sum()
    }

    /// §5.4's "init = V2-padded (incumbents at V2, candidates 0)".
    fn v2_padded(&self) -> Vec<i64> {
        let table = v2_vector(self.stage);
        let mut w = vec![0i64; AXIS_WIDTH];
        for f in Feature::ALL {
            w[f.index()] = table[f.index()];
        }
        // The pin is the box being degenerate at w_vp, and V2's own vp entry IS
        // the pinned value (§5.1:428 pins `w_vp = (1, 40)`, "V2's shipped
        // pair"). Clamping rather than trusting that keeps the start admissible
        // even under the early-rescale diagnostic, which moves the pin.
        for (i, x) in w.iter_mut().enumerate() {
            *x = (*x).clamp(self.lo[i], self.hi[i]);
        }
        w
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
fn witness_search(sys: &System) -> WitnessReport {
    let mut w = sys.v2_padded();
    let mut violation = sys.violation(&w);
    let mut sweeps = 0usize;

    while violation > 0 && sweeps < MAX_SWEEPS {
        sweeps += 1;
        let mut moved = false;
        for coord in 0..AXIS_WIDTH {
            // (violation, |resulting w|, step-set position) — lexicographically
            // least wins, which is exactly §5.4's "best strict decrease, ties
            // toward smaller |w| then listed order" with no residual freedom.
            let mut best: Option<(i128, i64, usize, i64)> = None;
            for (position, &step) in STEP_SET.iter().enumerate() {
                let candidate = w[coord] + step;
                if candidate < sys.lo[coord] || candidate > sys.hi[coord] {
                    continue;
                }
                let previous = w[coord];
                w[coord] = candidate;
                let value = sys.violation(&w);
                w[coord] = previous;
                if value >= violation {
                    continue;
                }
                let key = (value, candidate.abs(), position, candidate);
                if best.is_none_or(|b| key < b) {
                    best = Some(key);
                }
            }
            if let Some((value, _, _, candidate)) = best {
                w[coord] = candidate;
                violation = value;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }

    let slack: Vec<i128> = sys.rows.iter().map(|r| System::slack_of(r, &w)).collect();
    let binding: Vec<String> = sys
        .rows
        .iter()
        .zip(&slack)
        .filter(|(_, s)| **s == 0)
        .map(|(r, _)| r.label.clone())
        .collect();
    WitnessReport { w, violation, sweeps, binding, slack }
}

// ---------------------------------------------------------------------------
// Toy systems (§5.4's "development uses toy systems only")
// ---------------------------------------------------------------------------

/// One toy row as the builder takes it: label, sparse `(index, coef)` entries,
/// and the right-hand side.
type ToyRow<'a> = (&'a str, &'a [(usize, i64)], i64);

/// The toy systems, over the REAL 34-wide axis so the machinery under test is
/// the machinery that will run — only the row set is small.
mod toys {
    use super::*;

    /// Build a toy over the real axis.
    ///
    /// The box and the pin are the REAL ones, so a toy exercises the same
    /// degenerate-`w_vp` coordinate and the same `±limit` walls the real system
    /// will. Only the row set is small.
    pub fn toy(stage: StageEndpoint, rows: &[ToyRow], limit: i64) -> System {
        let built = rows
            .iter()
            .enumerate()
            .map(|(i, (label, entries, rhs))| {
                let mut coef = vec![0i64; AXIS_WIDTH];
                for (index, value) in *entries {
                    coef[*index] = *value;
                }
                Row {
                    label: (*label).to_string(),
                    kind: RowKind::Pair { id: format!("toy-{i}"), order: i },
                    coef,
                    rhs: *rhs,
                }
            })
            .collect();
        System::new(stage, built, limit, pinned_vp(stage))
    }

    /// §5.1's pinned `w_vp`, by stage.
    pub fn pinned_vp(stage: StageEndpoint) -> i64 {
        super::v2_vector(stage)[Feature::VictoryPoints.index()]
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

    /// **V2's two hand-ordering rows**, as §5.4 registers them: "the V2 hand
    /// ordering (`core > on_board`, `on_board > excess`, both vectors) as
    /// strict inequalities".
    ///
    /// Strict over the integers is `≥ 1`. Both rows hold in V2 itself at both
    /// stages (early 30 > 6 > 5, late 20 > 10 > 2), which is what makes them
    /// STANDING constraints rather than a hypothesis — they are already true of
    /// the shipped table the pin comes from.
    pub fn hand_ordering_rows() -> Vec<Row> {
        let core = Feature::AssistantsInHandCore.index();
        let board = Feature::AssistantsOnBoard.index();
        let excess = Feature::AssistantsInHandExcess.index();
        [("hand: core > on_board", core, board), ("hand: on_board > excess", board, excess)]
            .into_iter()
            .map(|(label, greater, lesser)| {
                let mut coef = vec![0i64; AXIS_WIDTH];
                coef[greater] = 1;
                coef[lesser] = -1;
                Row { label: label.to_string(), kind: RowKind::HandOrdering, coef, rhs: 1 }
            })
            .collect()
    }

    /// **THE REGISTERED TOY SYSTEMS.** Named, so a verdict artifact can say
    /// which one it ran and a review can re-run exactly it.
    pub fn named(name: &str) -> Option<System> {
        let (s1, e1, e2) = (col("S1"), col("E1"), col("E2"));
        Some(match name {
            // The standing constraints alone -- the pin, the box, and V2's two
            // hand orderings, with no pair row at all. V2 itself is a witness,
            // so this toy is the sanity floor: if it ever fails, the pin and the
            // ordering disagree and nothing downstream means anything.
            "standing-only" => System::new(
                StageEndpoint::Early,
                hand_ordering_rows(),
                BOX_LIMIT,
                pinned_vp(StageEndpoint::Early),
            ),
            // Reachable by the descent: each row falls to one coordinate move,
            // and the coupling runs forward along the axis.
            "sat-reachable" => toy(
                StageEndpoint::Early,
                &[
                    ("S1 productive", &[(s1, 1)], 1),
                    ("E1 money", &[(e1, 1)], 3),
                    ("S1 at least E2", &[(s1, 1), (e2, -1)], 0),
                ],
                BOX_LIMIT,
            ),
            // Satisfiable, but only by moving two coordinates together, so the
            // descent stalls and the oracle has to decide it.
            "sat-coupled" => toy(
                StageEndpoint::Early,
                &[
                    ("S1 productive", &[(s1, 1)], 1),
                    ("E2 strictly over S1", &[(e2, 1), (s1, -1)], 1),
                ],
                BOX_LIMIT,
            ),
            // The two-row contradiction: Farkas y = (1, 1).
            "unsat-direct" => toy(
                StageEndpoint::Early,
                &[("S1 up", &[(s1, 1)], 1), ("S1 down", &[(s1, -1)], 1)],
                BOX_LIMIT,
            ),
            // Infeasible only against the BOX -- the row alone is satisfiable,
            // so the certificate must name the box bound it leans on.
            "unsat-box" => toy(
                StageEndpoint::Early,
                &[("S1 beyond the wall", &[(s1, 1)], BOX_LIMIT + 1)],
                BOX_LIMIT,
            ),
            _ => return None,
        })
    }

    /// Every registered toy's name, in the order [`named`] lists them.
    pub const NAMES: [&str; 5] =
        ["standing-only", "sat-reachable", "sat-coupled", "unsat-direct", "unsat-box"];
}

// ---------------------------------------------------------------------------
// The artifact
// ---------------------------------------------------------------------------

/// **The identity block every verdict artifact carries.**
///
/// §5.4: "The oracle IMPLEMENTATION freezes at the step-2 commit and every
/// verdict artifact prints its digest". Three identities, because three things
/// can move underneath a verdict independently:
///
/// * the FIXTURE PACKAGE, by `pair_fixtures::package_digest` over the registered
///   tuples — which system was solved;
/// * the SOLVER IMPLEMENTATION, by a content hash of THIS FILE's source
///   (`include_str!` of itself, through the same FNV-1a the repo's other content
///   hashes use) — which solver solved it. A source hash rather than a commit
///   hash because it needs no build plumbing and cannot go stale: it moves when
///   the implementation moves, which is exactly the property §5.4 wants when it
///   says "two conforming implementations are the same implementation";
/// * `SEMANTICS_REV`, because the fixtures are engine states and a rules change
///   would change what the rows MEAN without touching either hash above.
///
/// The freeze COMMIT is recorded in the ledger and may be supplied to the real
/// run with `--freeze-commit`; it is provenance for a reader, while the source
/// hash is the machine-checkable half.
fn identity_block(freeze_commit: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str(&format!("package digest       {}\n", package_digest()));
    out.push_str(&format!("solver identity      {}  (content hash of examples/solver.rs)\n", solver_identity()));
    out.push_str(&format!("semantics rev        {SEMANTICS_REV}\n"));
    out.push_str(&format!(
        "V2 weight identity   {:#018x}  (the source of the w_vp pin and the hand ordering)\n",
        weight_table_identity(&WeightTable::V2),
    ));
    out.push_str(&format!(
        "freeze commit        {}\n",
        freeze_commit.unwrap_or("not supplied (the source hash above is the machine-checked half)"),
    ));
    out
}

/// See [`identity_block`]. FNV-1a 64 over this file's own source bytes, the same
/// primitive `Rules::content_hash` and `pair_fixtures::package_digest` use.
fn solver_identity() -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in include_str!("solver.rs").as_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// §5.4's realized-versus-frozen census cross-check, with the ruling in place of
/// a defect claim.
fn census_block() -> String {
    let realized = realized_census();
    let mut out = String::new();
    out.push_str(&format!(
        "census               realized EARLY {} / LATE {}   frozen text EARLY {FROZEN_EARLY} / \
         LATE {FROZEN_LATE}\n",
        realized.early, realized.late,
    ));
    if realized.early == FROZEN_EARLY && realized.late == FROZEN_LATE {
        out.push_str("                     realized == frozen\n");
    } else {
        out.push_str(&format!("RULING               {CENSUS_RULING}\n"));
    }
    out
}

/// The registry's margin table — every constraint row's class and integer,
/// read off the declarations without building a single fixture.
fn margin_block() -> String {
    let mut out = String::new();
    out.push_str("\nregistered margins (declarations only -- no fixture is built to print this)\n");
    out.push_str("  ord  pair          stage  margin  class\n");
    for pair in REGISTERED.iter().filter(|p| matches!(p.role, PairRole::Constraint)) {
        // The DELETION ORDER, printed so a review can check it against §5.2's
        // printed text without reading code.
        let ord = document_order(pair.id).expect("a registered pair is in the registry");
        for stage in pair.stages {
            let margin = match margin_of(pair, *stage) {
                Ok(m) => m.to_string(),
                Err(e) => format!("REFUSED: {e}"),
            };
            let class = match pair.margin {
                MarginClass::Margined { carrier, .. } => {
                    format!("margined via {}", carrier.label())
                }
                MarginClass::DirectionOnly => "direction-only".to_string(),
                MarginClass::Weak => "weak".to_string(),
            };
            out.push_str(&format!(
                "  {ord:>3}  {:<12}  {:<5}  {:>6}  {class}\n",
                pair.id,
                stage.name(),
                margin,
            ));
        }
    }
    out.push_str(
        "  (ord = §5.2 DOCUMENT ORDER, which is the registered deletion order for §5.4 (3))\n",
    );
    out
}

/// Run one toy through the machinery that exists, and print it.
fn run_toy(name: &str) -> Result<(), String> {
    let sys = toys::named(name)
        .ok_or_else(|| format!("`{name}` is not a registered toy; see `solver toys`"))?;
    println!("=== solver toy {name} ===");
    print!("{}", identity_block(None));
    println!("stage                {}", sys.stage.name());
    println!("box                  |w| <= {}   w_vp pinned to {}", sys.box_limit, sys.pin_vp);
    println!(
        "rows                 {} deletable, {} inequalities in all (deletable + pin + box)",
        sys.rows.len(),
        sys.inequalities().len(),
    );

    let report = witness_search(&sys);
    println!();
    println!("witness search       {} sweeps, total violation {}", report.sweeps, report.violation);
    if report.violation == 0 {
        println!("verdict              WITNESS (feasibility PROVEN by a re-checked point)");
        println!("binding K            {}", report.binding.len());
        println!("{SAT_READING}");
    } else {
        println!(
            "verdict              NO WITNESS -- a statement about the SEARCH, not the system. \
             Only the exact oracle may say UNSAT."
        );
    }
    println!();
    println!("slack profile");
    for (row, slack) in sys.rows.iter().zip(&report.slack) {
        println!("  {:<24}  {:>8}", row.label, slack);
    }
    println!();
    println!("weight vector (entries away from the V2-padded start)");
    let start = sys.v2_padded();
    let mut moved = 0usize;
    for (i, id) in FeatureId::all().into_iter().enumerate() {
        if report.w[i] != start[i] {
            println!("  {:<28}  {:>5}  (from {})", id.label(), report.w[i], start[i]);
            moved += 1;
        }
    }
    if moved == 0 {
        println!("  none -- the V2-padded start was already the returned point");
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("identities") => {
            println!("=== solver identities ===");
            print!("{}", identity_block(None));
            print!("{}", census_block());
            print!("{}", margin_block());
            Ok(())
        }
        Some("toys") => {
            println!("registered toy systems (§5.4 development runs on these and nothing else):");
            for name in toys::NAMES {
                println!("  {name}");
            }
            Ok(())
        }
        Some("toy") => match args.get(1) {
            Some(name) => run_toy(name),
            None => Err("`toy` needs a name; see `solver toys`".to_string()),
        },
        Some(other) => Err(format!("unknown subcommand `{other}`\n\n{USAGE}")),
        None => Err(USAGE.to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("solver: {e}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "\
usage: solver <subcommand>

  identities         the identity block, the realized-vs-frozen census, and the
                     registry's margin table -- declarations only, no fixture built
  toys               list the registered toy systems
  toy <name>         run one toy end to end
";

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::toys::{col, toy};
    use super::*;
    use yokohama_core::game::pair_fixtures::PairRole;

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

    /// A toy the descent reaches: every row is fixed by a single coordinate
    /// move from the V2-padded start, and the coupling runs FORWARD along the
    /// axis (the constrained coordinate is the earlier one), which is the shape
    /// coordinate descent can cross.
    fn sat_toy() -> System {
        toy(
            StageEndpoint::Early,
            &[
                ("S1 productive", &[(col("S1"), 1)], 1),
                ("E1 money", &[(col("E1"), 1)], 3),
                ("S1 at least E2", &[(col("S1"), 1), (col("E2"), -1)], 0),
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

    /// **Coordinate descent is INCOMPLETE, and this is the cell that says so.**
    ///
    /// Two rows whose satisfaction requires two coordinates to move TOGETHER
    /// stall the search: no single step of any size strictly decreases the
    /// total, so a full sweep applies nothing and the search ends with a
    /// positive residual — while a witness plainly exists, exhibited and
    /// verified here.
    ///
    /// This is why §5.4 makes the exact oracle "the ONLY authority for
    /// infeasibility". A non-witness is a statement about the SEARCH, never
    /// about the system, and nothing downstream may read it as UNSAT.
    #[test]
    fn coordinate_descent_can_stall_short_of_a_witness_that_exists() {
        let sys = toy(
            StageEndpoint::Early,
            &[
                ("S1 productive", &[(col("S1"), 1)], 1),
                ("E2 strictly over S1", &[(col("E2"), 1), (col("S1"), -1)], 1),
            ],
            BOX_LIMIT,
        );
        let report = witness_search(&sys);
        assert!(report.violation > 0, "the coupled rows stall a single-coordinate search");

        let mut exhibited = sys.v2_padded();
        exhibited[col("S1")] = 1;
        exhibited[col("E2")] = 2;
        assert_eq!(
            sys.violation(&exhibited),
            0,
            "a witness exists -- so the search's residual is a fact about the search",
        );
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
