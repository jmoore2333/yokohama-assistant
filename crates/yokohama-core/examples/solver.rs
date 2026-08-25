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
//! implementation is frozen and reviewed — see [`identity_block`], which names
//! what each verdict artifact carries and why a source hash is the
//! machine-checked half of it.
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

use std::collections::HashMap;
use std::process::ExitCode;

use yokohama_core::game::eval::{Feature, ScoreStructure, WeightTable};
use yokohama_core::game::greedy::weight_table_identity;
use yokohama_core::game::pair_fixtures::{
    build_registered_pair, package_digest, sign_prior, CandidateFeature, FeatureId, MarginClass,
    PairDeclaration, PairRole, Side, StageEndpoint, AXIS_WIDTH, REGISTERED,
};
use yokohama_core::{PlayerId, PlayerView, Rules, VisiblePlayer, SEMANTICS_REV};

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
    /// A DERIVED Chvátal–Gomory cut (B2). Not deletion-eligible, and never part
    /// of a registered system: a cut exists only inside one question put to the
    /// oracle by [`decide_with_cuts`], which throws the strengthened system away
    /// as soon as it has its answer.
    Cut,
    /// A half of the `w_vp` equality pin. Not deletion-eligible.
    Pin,
    /// One of R3's projection rows — a displacement link or the displacement
    /// budget. Not deletion-eligible, and never part of a registered system:
    /// these exist only inside [`augmented`], where the question is not "is the
    /// registered system satisfiable" but "is it satisfiable within `d` of the
    /// draft".
    Repair,
}

/// The coefficient width a row set is written over.
///
/// `AXIS_WIDTH` for every system §5.4 registers, and wider for R3's augmented
/// projection systems, whose extra columns carry per-coordinate displacement.
/// Reading the width off the rows rather than off the constant is what lets the
/// ONE frozen oracle decide both: at `AXIS_WIDTH` the two are the same number,
/// so every §5.4 verdict is the verdict it always was.
fn row_width(rows: &[Row]) -> usize {
    rows.iter().map(|r| r.coef.len()).max().unwrap_or(AXIS_WIDTH)
}

/// One linear inequality `Σ_f coef[f] · w_f ≥ rhs`, in integer units.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Row {
    label: String,
    kind: RowKind,
    coef: Vec<i64>,
    rhs: i64,
    /// The registered `margin_vp` fraction, for MARGINED pair rows ONLY.
    ///
    /// `Some` exactly where §5.1's margin is a FUNCTION of the pin, so the
    /// early-rescale ladder can re-evaluate it at each rung. `None` for
    /// DIRECTION-ONLY and WEAK rows, whose 1 and 0 are not prices and do not
    /// move with the pin, and for every box, pin and hand-ordering row.
    margin_vp: Option<(i64, i64)>,
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

/// **§5.2's MARGINED pairs and the VP FRACTIONS their integers come from**,
/// transcribed from the document.
///
/// §5.1 defines a MARGINED margin as `max(1, ceil(margin_vp × w_vp(stage)))` —
/// a FUNCTION of the pin, of which the registered integer is one evaluation, at
/// the shipped `w_vp = (1, 40)`. The registry stores the evaluation; this table
/// stores the function's other argument, which is what §5.4's early-rescale
/// diagnostic needs when it MOVES the pin.
///
/// Transcribed from §5.2's own prose: 1(a) "amortized ⅓ VP/good" over one good ·
/// 2(a) the same ⅓ rate over goods +2 · 3(a) "amortized ½ VP/yen" over one yen ·
/// 3(b) the ⅓ rate over one good · 3(c) "one activation's 2¥ ≈ 1 VP" ·
/// 4(d) "margin_vp = 4".
const MARGIN_VP: [(&str, i64, i64); 6] =
    [("1(a)", 1, 3), ("2(a)", 2, 3), ("3(a)", 1, 2), ("3(b)", 1, 3), ("3(c)", 1, 1), ("4(d)", 4, 1)];

/// §5.1's MARGINED formula, `max(1, ceil(margin_vp × w_vp))`, at an ARBITRARY
/// pin.
///
/// The registered integers are this evaluated at the shipped pin. The
/// early-rescale ladder evaluates it again at each rung, because a margin left
/// at its shipped value while the pin moves is a margin that has silently
/// changed meaning.
fn margined_integer(numerator: i64, denominator: i64, w_vp: i64) -> i64 {
    let scaled = numerator * w_vp;
    let ceiling = scaled.div_euclid(denominator) + i64::from(scaled.rem_euclid(denominator) > 0);
    ceiling.max(1)
}

/// The registered VP fraction of a MARGINED pair, or `None` for the
/// DIRECTION-ONLY and WEAK classes.
///
/// Those two are deliberately absent rather than stored as 1/1 and 0/1: §5.1's
/// own words are that 1 and 0 "are not prices, they are the two shapes of 'no
/// price was registered'", so they do NOT move with the pin and a fraction for
/// them would be a fiction that re-priced them.
fn margin_fraction(id: &str) -> Option<(i64, i64)> {
    MARGIN_VP.iter().find(|(pair, _, _)| *pair == id).map(|(_, n, d)| (*n, *d))
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

/// V2's production weight vector at one stage — the source of both the `w_vp` pin
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
    /// The width is `self.lo.len()`, which is `AXIS_WIDTH` for every system
    /// §5.4 registers and wider for one of R3's augmented projection systems.
    /// At `AXIS_WIDTH` this emits exactly the rows it always emitted.
    fn inequalities(&self) -> Vec<Row> {
        let mut all = self.rows.clone();
        let vp = Feature::VictoryPoints.index();
        let axis = FeatureId::all();
        for i in 0..self.lo.len() {
            let kind = if i == vp { RowKind::Pin } else { RowKind::BoxBound };
            let name = match axis.get(i) {
                Some(id) => id.label().to_string(),
                None => format!("|w-w0| #{}", i - AXIS_WIDTH),
            };
            let mut lower = vec![0i64; self.lo.len()];
            lower[i] = 1;
            all.push(Row {
                label: format!("{name} >= {}", self.lo[i]),
                kind: kind.clone(),
                coef: lower,
                rhs: self.lo[i],
                margin_vp: None,
            });
            let mut upper = vec![0i64; self.lo.len()];
            upper[i] = -1;
            all.push(Row {
                label: format!("{name} <= {}", self.hi[i]),
                kind,
                coef: upper,
                rhs: -self.hi[i],
                margin_vp: None,
            });
        }
        all
    }

    /// `Σ_f coef[f]·w[f] − rhs` for one row, in i128.
    ///
    /// i128 because the accumulation is free there and the alternative is
    /// reasoning about a bound: coefficients are feature deltas whose size the
    /// fixtures decide, not this file.
    ///
    /// N-8, the bound that makes the unchecked arithmetic here and in
    /// [`propagate`] safe rather than lucky. A weight is always inside the box,
    /// so `|w| ≤ 128` at the widest rung; a coefficient is an `i64`. Each term
    /// is therefore under `2^70`, and at most 34 are summed, so `|Σ| < 2^76` —
    /// comfortably inside i128's `2^127`, with the same argument covering
    /// `propagate`'s `reach` and `need`. Checked arithmetic is reserved for the
    /// LP, which multiplies rationals by rationals and genuinely can escape.
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
        // `self.lo.len()`, not `AXIS_WIDTH`: an augmented projection system is
        // wider, and its extra columns start at 0 like every candidate does.
        let mut w = vec![0i64; self.lo.len()];
        for &f in &Feature::ALL[..Feature::INCUMBENT_COUNT] {
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
/// [`FeatureId::all`] index order (the sixteen-feature incumbent prefix of
/// `Feature::ALL`, then the eighteen candidates in `CandidateFeature::ALL`
/// order — the research axis §3.2 declares), and AT each coordinate applies the best strictly
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
    fn new(n: i128, d: i128) -> Option<Rat> {
        if d == 0 {
            return None;
        }
        // `i128::MIN.checked_neg()` is None, so the negation is checked rather
        // than assumed: a wrap here would silently flip a sign.
        let (n, d) = if d < 0 { (n.checked_neg()?, d.checked_neg()?) } else { (n, d) };
        let g = gcd(n.unsigned_abs(), d.unsigned_abs());
        if g == 0 {
            return Some(Rat::ZERO);
        }
        let g = i128::try_from(g).ok()?;
        Some(Rat { n: n / g, d: d / g })
    }

    /// An integer as a rational.
    fn int(n: i128) -> Rat {
        Rat { n, d: 1 }
    }

    fn add(self, o: Rat) -> Option<Rat> {
        let n = self.n.checked_mul(o.d)?.checked_add(o.n.checked_mul(self.d)?)?;
        Rat::new(n, self.d.checked_mul(o.d)?)
    }

    fn neg(self) -> Option<Rat> {
        Some(Rat { n: self.n.checked_neg()?, d: self.d })
    }

    fn sub(self, o: Rat) -> Option<Rat> {
        self.add(o.neg()?)
    }

    fn mul(self, o: Rat) -> Option<Rat> {
        Rat::new(self.n.checked_mul(o.n)?, self.d.checked_mul(o.d)?)
    }

    fn div(self, o: Rat) -> Option<Rat> {
        Rat::new(self.n.checked_mul(o.d)?, self.d.checked_mul(o.n)?)
    }

    /// `-1`, `0` or `+1`.
    fn sign(self) -> i32 {
        match self.n.cmp(&0) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    fn is_zero(self) -> bool {
        self.n == 0
    }

    /// Whether this rational IS an integer. Total, because [`Rat::new`] reduces
    /// and keeps the denominator positive.
    fn is_integer(self) -> bool {
        self.d == 1
    }

    /// `⌈n/d⌉`. The denominator is positive by construction, so this is
    /// [`ceil_div`] with no sign case.
    fn ceil(self) -> i128 {
        ceil_div(self.n, self.d)
    }

    /// Exact comparison. `None` on an overflow in the cross-multiplication.
    fn cmp_exact(self, o: Rat) -> Option<std::cmp::Ordering> {
        // Both denominators are positive by construction, so cross-multiplying
        // preserves the order without a sign case.
        Some(self.n.checked_mul(o.d)?.cmp(&o.n.checked_mul(self.d)?))
    }
}

/// Euclid's GCD over magnitudes. `gcd(0, x) == x`, so [`Rat::new`] reduces a
/// zero numerator without a special case.
fn gcd(a: u128, b: u128) -> u128 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// Least common multiple over magnitudes, checked — used to clear a
/// certificate's denominators.
fn lcm(a: u128, b: u128) -> Option<u128> {
    if a == 0 || b == 0 {
        return Some(0);
    }
    (a / gcd(a, b)).checked_mul(b)
}

// ---------------------------------------------------------------------------
// §5.4 (2) — the exact oracle
// ---------------------------------------------------------------------------

/// §5.4's registered node budget: "10⁷ nodes per stage system".
const NODE_BUDGET: u64 = 10_000_000;

/// §5.4's registered enumeration budget: a relaxation-feasible/integer-infeasible
/// verdict is admissible "only when the PRODUCT of the support's box ranges is
/// ≤ 10⁷ points".
const ENUMERATION_BUDGET: u128 = 10_000_000;

/// A cap on simplex pivots. Bland's rule already forbids cycling, so this is a
/// belt on a proof rather than the proof — and hitting it is INCONCLUSIVE.
const PIVOT_CAP: usize = 200_000;

/// **What an UNSAT SHIPS.** §5.4 admits exactly two shapes, and the verdict type
/// cannot be built without one of them.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Certificate {
    /// "a relaxation-infeasible UNSAT ships a Farkas nonnegative rational
    /// combination, independently re-verified by an integer pass after clearing
    /// denominators". `y` is that combination, denominators already cleared, one
    /// entry per row of [`System::inequalities`]; `rhs` is the positive
    /// `Σ y_i b_i` the verification computed.
    Farkas { y: Vec<i128>, rhs: i128 },
    /// "a relaxation-feasible/integer-infeasible verdict is re-derived by
    /// exhaustive enumeration over the IIS's variable support, admissible only
    /// when the PRODUCT of the support's box ranges is ≤ 10⁷ points".
    Enumeration { support: Vec<usize>, points: u128 },
    /// **B2's extension.** The system was strengthened by verified
    /// Chvátal–Gomory cuts and the STRENGTHENED system was then certified by
    /// `inner` — the frozen oracle's own verdict on a question with more rows in
    /// it. Every cut carries its parent rows and multipliers, so the whole chain
    /// re-derives from the original system and this certificate alone.
    ///
    /// **Why it certifies the ORIGINAL system.** Each cut is a nonnegative
    /// rational combination of rows already valid for every integer point, with
    /// an INTEGRAL combined coefficient vector — so at an integer point its left
    /// side is an integer, and rounding its right side UP loses no integer
    /// point. By induction over the chain every cut is valid for every integer
    /// point of the original system, so a strengthened system with no point at
    /// all is an original system with no INTEGER point at all.
    ChvatalGomory { cuts: Vec<DerivedCut>, inner: Box<Certificate> },
}

/// The oracle's three answers. There is no fourth, and INCONCLUSIVE is not a
/// weak UNSAT — §5.4: "'the oracle did not decide' is never infeasibility".
#[derive(Clone, Debug, PartialEq, Eq)]
enum Verdict {
    /// A point satisfying every inequality, re-checked before it was returned.
    Sat { w: Vec<i64> },
    /// Infeasible, with a VERIFIED certificate.
    Unsat { certificate: Certificate },
    /// The oracle did not decide. Routed to the owner.
    Inconclusive { reason: String },
}

impl Verdict {
    /// The one-word spelling an artifact prints.
    fn name(&self) -> &'static str {
        match self {
            Verdict::Sat { .. } => "SAT",
            Verdict::Unsat { .. } => "UNSAT",
            Verdict::Inconclusive { .. } => "INCONCLUSIVE",
        }
    }
}

/// **THE TRUST BOUNDARY.** Re-check a Farkas combination in checked i128 INTEGER
/// arithmetic, reading only the integer system and the multipliers.
///
/// Farkas' lemma for `{w : A w ≥ b}`: the set is empty iff there is `y ≥ 0` with
/// `yᵀA = 0` and `yᵀb > 0`. This function proves exactly that, and it shares no
/// code with the search that proposed `y` — which is what makes §5.4's
/// "independently re-verified by an integer pass after clearing denominators"
/// an independence claim rather than a restatement.
///
/// Returns the positive `yᵀb` on success. Any overflow is an `Err`, and its
/// caller turns that into INCONCLUSIVE rather than UNSAT.
fn verify_farkas(rows: &[Row], y: &[i128]) -> Result<i128, String> {
    if y.len() != rows.len() {
        return Err(format!("{} multipliers against {} inequalities", y.len(), rows.len()));
    }
    if let Some((i, m)) = y.iter().enumerate().find(|(_, m)| **m < 0) {
        return Err(format!("multiplier {i} is {m}; Farkas needs y >= 0"));
    }
    let overflow = |what: &str| format!("i128 overflow while checking {what}");

    // EVERY column the rows are written over, which is `AXIS_WIDTH` for a §5.4
    // system and wider for an augmented projection one. A column left unchecked
    // is a column a combination could hide a nonzero in, so the width comes off
    // the rows themselves rather than off a constant that would be right for
    // one caller and short for the other.
    for column in 0..row_width(rows) {
        let mut sum: i128 = 0;
        for (row, multiplier) in rows.iter().zip(y) {
            let term = i128::from(row.coef.get(column).copied().unwrap_or(0))
                .checked_mul(*multiplier)
                .ok_or_else(|| overflow("a coefficient product"))?;
            sum = sum.checked_add(term).ok_or_else(|| overflow("a coefficient column"))?;
        }
        if sum != 0 {
            return Err(format!(
                "column {column} of yA sums to {sum}, not 0; the combination is not a Farkas \
                 witness",
            ));
        }
    }

    let mut rhs: i128 = 0;
    for (row, multiplier) in rows.iter().zip(y) {
        let term = i128::from(row.rhs)
            .checked_mul(*multiplier)
            .ok_or_else(|| overflow("a right-hand-side product"))?;
        rhs = rhs.checked_add(term).ok_or_else(|| overflow("the right-hand side"))?;
    }
    if rhs <= 0 {
        return Err(format!("yb is {rhs}, and Farkas needs it strictly positive"));
    }
    Ok(rhs)
}

/// What the exact LP found about the RATIONAL relaxation.
#[derive(Clone, Debug)]
enum Relaxation {
    Feasible,
    /// Infeasible, with the Farkas multipliers the simplex's optimal duals give
    /// — a PROPOSAL, not yet a proof. [`verify_farkas`] decides.
    Infeasible(Vec<Rat>),
    /// Overflow, a pivot-cap hit, or a degenerate shape. Never infeasibility.
    Undecided(String),
}

/// **The exact rational LP** — Phase-I simplex with Bland's rule over [`Rat`],
/// deciding the relaxation of `A w ≥ b` and extracting Farkas multipliers when
/// it is empty.
///
/// # The formulation, because the sign conventions are the whole correctness
///
/// The weights are FREE, so each is split `w_j = p_j − q_j` with `p, q ≥ 0`.
/// Farkas' condition for `{x ≥ 0 : [A, −A] x ≥ b}` then reads `yᵀA ≤ 0` AND
/// `−yᵀA ≤ 0`, which is `yᵀA = 0` — the clean form, recovered rather than
/// assumed.
///
/// Each row `A_i w − s_i = b_i` (surplus `s_i ≥ 0`) is normalized by
/// `sign_i = ±1` so its right-hand side is nonnegative. Where `b_i ≤ 0` the
/// normalization leaves the surplus with coefficient `+1`, so `s_i` is a
/// feasible starting basic variable and NO ARTIFICIAL IS NEEDED — which matters
/// at scale, since the box contributes two such rows per column and would
/// otherwise double the tableau.
///
/// # Extracting the certificate
///
/// The dual multiplier on the ORIGINAL inequality `i` is `y_i = −Z[s_i]`, the
/// negated reduced cost of its surplus column. At Phase-I optimality every
/// enterable column has `Z ≤ 0`, so `y ≥ 0` falls out; `Z[p_j] ≤ 0` and
/// `Z[q_j] ≤ 0` are `yᵀA_j ≤ 0` and `yᵀA_j ≥ 0`, so `yᵀA = 0` falls out; and
/// the objective cell is `yᵀb`, which is positive exactly when Phase-I could
/// not drive the artificials to zero. None of that is trusted here — it is
/// [`verify_farkas`]'s to confirm.
fn relaxation(all: &[Row], row_index: &[usize], columns: &[usize]) -> Relaxation {
    let m = row_index.len();
    let n = columns.len();
    if m == 0 || n == 0 {
        // Nothing to contradict: an empty row set is satisfied by any point,
        // and rows over no active column were dropped precisely because they
        // are individually satisfiable.
        return Relaxation::Feasible;
    }
    let undecided = |what: &str| Relaxation::Undecided(what.to_string());

    // Row normalization: sign, and which rows need an artificial.
    let sign: Vec<i128> = row_index.iter().map(|&i| if all[i].rhs < 0 { -1 } else { 1 }).collect();
    let mut artificial_of = vec![None; m];
    let mut artificials = 0usize;
    for (i, s) in sign.iter().enumerate() {
        if *s > 0 {
            artificial_of[i] = Some(artificials);
            artificials += 1;
        }
    }

    // Columns: p_0..p_{n-1} | q_0..q_{n-1} | s_0..s_{m-1} | artificials.
    let first_surplus = 2 * n;
    let first_artificial = first_surplus + m;
    let width = first_artificial + artificials;

    let mut t = vec![vec![Rat::ZERO; width + 1]; m];
    let mut basis = vec![0usize; m];
    for (i, &ri) in row_index.iter().enumerate() {
        let row = &all[ri];
        let s = Rat::int(sign[i]);
        for (jj, &column) in columns.iter().enumerate() {
            let Some(a) = Rat::int(i128::from(row.coef[column])).mul(s) else {
                return undecided("a coefficient overflowed while building the tableau");
            };
            let Some(negated) = a.neg() else {
                return undecided("a coefficient overflowed while building the tableau");
            };
            t[i][jj] = a;
            t[i][n + jj] = negated;
        }
        t[i][first_surplus + i] = Rat::int(-sign[i]);
        match artificial_of[i] {
            Some(k) => {
                t[i][first_artificial + k] = Rat::int(1);
                basis[i] = first_artificial + k;
            }
            None => basis[i] = first_surplus + i,
        }
        let Some(rhs) = Rat::int(i128::from(row.rhs)).mul(s) else {
            return undecided("a right-hand side overflowed while building the tableau");
        };
        t[i][width] = rhs;
    }

    // Price out the artificial basis: Z_j = Σ_{artificial rows} T[i][j] − c_j.
    let mut z = vec![Rat::ZERO; width + 1];
    for i in 0..m {
        if artificial_of[i].is_some() {
            for j in 0..=width {
                let Some(sum) = z[j].add(t[i][j]) else {
                    return undecided("the objective row overflowed while pricing out");
                };
                z[j] = sum;
            }
        }
    }
    for k in 0..artificials {
        let j = first_artificial + k;
        let Some(reduced) = z[j].sub(Rat::int(1)) else {
            return undecided("the objective row overflowed while pricing out");
        };
        z[j] = reduced;
    }

    // Bland's rule: smallest enterable index with a positive reduced cost, and
    // on a ratio tie the smallest BASIC variable index leaves. Artificials are
    // never enterable, which is the standard way to keep them from returning.
    for _ in 0..PIVOT_CAP {
        let Some(enter) = (0..first_artificial).find(|&j| z[j].sign() > 0) else {
            break;
        };
        let mut leaving: Option<(usize, Rat)> = None;
        for i in 0..m {
            if t[i][enter].sign() <= 0 {
                continue;
            }
            let Some(ratio) = t[i][width].div(t[i][enter]) else {
                return undecided("a ratio test overflowed");
            };
            leaving = match leaving {
                None => Some((i, ratio)),
                Some((best_row, best_ratio)) => {
                    let Some(order) = ratio.cmp_exact(best_ratio) else {
                        return undecided("a ratio comparison overflowed");
                    };
                    let takes_it = order == std::cmp::Ordering::Less
                        || (order == std::cmp::Ordering::Equal && basis[i] < basis[best_row]);
                    if takes_it { Some((i, ratio)) } else { Some((best_row, best_ratio)) }
                }
            };
        }
        let Some((pivot_row, _)) = leaving else {
            // Phase-I's objective is bounded below by zero, so this shape is a
            // defect rather than a result -- and a defect is INCONCLUSIVE.
            return undecided("the Phase-I objective read as unbounded");
        };

        let p = t[pivot_row][enter];
        for cell in &mut t[pivot_row] {
            let Some(scaled) = cell.div(p) else {
                return undecided("a pivot normalization overflowed");
            };
            *cell = scaled;
        }
        let pivot_line = t[pivot_row].clone();
        for (i, line) in t.iter_mut().enumerate() {
            if i == pivot_row || line[enter].is_zero() {
                continue;
            }
            let factor = line[enter];
            for (cell, p) in line.iter_mut().zip(&pivot_line) {
                let Some(term) = factor.mul(*p) else {
                    return undecided("a pivot elimination overflowed");
                };
                let Some(updated) = cell.sub(term) else {
                    return undecided("a pivot elimination overflowed");
                };
                *cell = updated;
            }
        }
        if !z[enter].is_zero() {
            let factor = z[enter];
            for j in 0..=width {
                let Some(term) = factor.mul(t[pivot_row][j]) else {
                    return undecided("an objective update overflowed");
                };
                let Some(updated) = z[j].sub(term) else {
                    return undecided("an objective update overflowed");
                };
                z[j] = updated;
            }
        }
        basis[pivot_row] = enter;

        if (0..first_artificial).all(|j| z[j].sign() <= 0) {
            break;
        }
    }

    if (0..first_artificial).any(|j| z[j].sign() > 0) {
        return undecided("the pivot cap was reached before Phase-I reached an optimum");
    }
    if z[width].sign() <= 0 {
        return Relaxation::Feasible;
    }

    // Infeasible. y_i = −Z[s_i], at the width of the FULL inequality list so
    // the verification reads the whole system.
    let mut y = vec![Rat::ZERO; all.len()];
    for (i, &ri) in row_index.iter().enumerate() {
        let Some(multiplier) = z[first_surplus + i].neg() else {
            return undecided("a certificate multiplier overflowed");
        };
        y[ri] = multiplier;
    }
    Relaxation::Infeasible(y)
}

/// Clear a rational combination's denominators, so [`verify_farkas`] can read
/// it in integers.
fn clear_denominators(y: &[Rat]) -> Option<Vec<i128>> {
    let mut multiple: u128 = 1;
    for r in y {
        multiple = lcm(multiple, r.d.unsigned_abs())?;
    }
    let multiple = i128::try_from(multiple).ok()?;
    y.iter().map(|r| r.n.checked_mul(multiple / r.d)).collect()
}

/// The columns that can matter: those carrying a nonzero coefficient in some
/// row.
///
/// A variable absent from every row appears only in its own box bounds, which
/// are satisfiable on their own, so it can take no part in an infeasibility and
/// its Farkas multiplier would be zero. Dropping it shrinks the LP without
/// changing a verdict or a certificate — the certificate is expanded back to
/// full width before verification, so the trust boundary still reads the whole
/// system.
fn active_columns(rows: &[Row]) -> Vec<usize> {
    (0..row_width(rows))
        .filter(|&j| rows.iter().any(|r| r.coef.get(j).copied().unwrap_or(0) != 0))
        .collect()
}

/// **§5.4 (2) — THE EXACT ORACLE, the only authority for infeasibility.**
///
/// # The order of operations, and why it is this order
///
/// 1. The RATIONAL relaxation, decided exactly. A relaxation-infeasible system
///    is integer-infeasible, and it is the case that ships a Farkas certificate
///    — so it is asked first and, when it fires, nothing else runs.
/// 2. Otherwise a bounded depth-first branch-and-bound hunts an integer point.
///    A point found is SAT, re-checked against every row before it is returned.
/// 3. A tree that exhausts without a point is NOT yet UNSAT. §5.4 requires the
///    verdict be "re-derived by exhaustive enumeration over the IIS's variable
///    support", admissible only under [`ENUMERATION_BUDGET`]; over budget is
///    INCONCLUSIVE.
///
/// # Why the search's pruning cannot produce a wrong UNSAT
///
/// Bound propagation and the interval test shape the SEARCH; they never carry a
/// verdict. Every UNSAT rests on a certificate that was verified independently
/// (path 1) or on an exhaustive enumeration that uses no pruning at all (path
/// 3). A propagation bug therefore degrades to INCONCLUSIVE — it can lose a
/// solution the enumeration would have found, and the enumeration is what
/// decides.
fn decide(sys: &System) -> Verdict {
    // N-1: an identically-zero deletable row is a construction defect, not a
    // system. `row_from_signatures` refuses one on the real path (Part A's zero
    // blocking says no registered row is one), so reaching here with one means a
    // caller built it by hand. Debug-only because a release verdict on such a
    // system is still SOUND -- at a positive margin it is honestly UNSAT.
    debug_assert!(
        !sys.rows.iter().any(|r| r.coef.iter().all(|c| *c == 0)),
        "a deletable row is identically zero; it constrains nothing and, at a positive margin, \
         is unsatisfiable by construction rather than by contradiction",
    );
    let all = sys.inequalities();
    let columns = active_columns(&sys.rows);
    // Rows over no active column were dropped with their columns: each is a box
    // bound or a pin half on a variable nothing else reads, and `lo <= hi`
    // holds for every one of them, so none can carry an infeasibility.
    let row_index: Vec<usize> = all
        .iter()
        .enumerate()
        .filter(|(_, r)| columns.iter().any(|&j| r.coef[j] != 0))
        .map(|(i, _)| i)
        .collect();

    match relaxation(&all, &row_index, &columns) {
        Relaxation::Undecided(why) => Verdict::Inconclusive {
            reason: format!("the exact relaxation did not decide: {why}"),
        },
        Relaxation::Infeasible(y) => {
            let Some(cleared) = clear_denominators(&y) else {
                return Verdict::Inconclusive {
                    reason: "clearing the proposed certificate's denominators overflowed"
                        .to_string(),
                };
            };
            match verify_farkas(&all, &cleared) {
                Ok(rhs) => Verdict::Unsat { certificate: Certificate::Farkas { y: cleared, rhs } },
                Err(e) => Verdict::Inconclusive {
                    reason: format!(
                        "the relaxation reported infeasible but its Farkas combination did not \
                         verify, so no infeasibility is claimed: {e}",
                    ),
                },
            }
        }
        Relaxation::Feasible => integer_search(sys, &columns, NODE_BUDGET),
    }
}

/// How the bounded depth-first search ended.
enum Search {
    Found(Vec<i64>),
    /// The tree closed with no integer point. NOT yet a verdict.
    Exhausted,
    /// [`NODE_BUDGET`] ran out.
    Budget,
}

/// §5.4 (2)'s "bounded depth-first branch-and-bound over the integer box",
/// followed by the enumeration re-derivation an exhausted tree owes.
///
/// `budget` is a parameter only so the budget-exhaustion ARM can be exercised:
/// [`decide`] always passes the registered [`NODE_BUDGET`], and a test that had
/// to spend ten million nodes to prove the arm exists would not be a test.
fn integer_search(sys: &System, columns: &[usize], budget: u64) -> Verdict {
    let mut nodes = 0u64;
    match explore(sys, columns, sys.lo.clone(), sys.hi.clone(), &mut nodes, budget) {
        Search::Found(w) => {
            // Re-checked before it leaves, on the same discipline as the
            // certificate: a point is trusted because it verifies.
            if sys.violation(&w) == 0 {
                Verdict::Sat { w }
            } else {
                Verdict::Inconclusive {
                    reason: "the search returned a point that does not satisfy the rows"
                        .to_string(),
                }
            }
        }
        Search::Budget => Verdict::Inconclusive {
            reason: format!(
                "the search spent its registered budget of {budget} nodes. The oracle did not \
                 decide, which is not infeasibility.",
            ),
        },
        Search::Exhausted => rederive_by_enumeration(sys, columns),
    }
}

/// One node of the search: propagate, dive, branch.
fn explore(
    sys: &System,
    columns: &[usize],
    lo: Vec<i64>,
    hi: Vec<i64>,
    nodes: &mut u64,
    budget: u64,
) -> Search {
    *nodes += 1;
    if *nodes > budget {
        return Search::Budget;
    }
    let Some((lo, hi)) = propagate(&sys.rows, lo, hi, columns) else {
        return Search::Exhausted;
    };

    // THE DIVE: the V2-padded start, clamped into this sub-box. Cheap, and it
    // is the point a near-feasible system is most likely to be satisfied at, so
    // a satisfiable system usually decides at the root instead of walking a
    // tree.
    let start = sys.v2_padded();
    let mut candidate = start.clone();
    for (i, x) in candidate.iter_mut().enumerate() {
        *x = (*x).clamp(lo[i], hi[i]);
    }
    if sys.violation(&candidate) == 0 {
        return Search::Found(candidate);
    }

    match columns.iter().copied().find(|&c| lo[c] < hi[c]) {
        None => Search::Exhausted,
        Some(branch) => {
            // Split at the midpoint, lower half first — deterministic, and no
            // entropy anywhere in the domain.
            let mid = (lo[branch] + hi[branch]).div_euclid(2);
            let mut lower_hi = hi.clone();
            lower_hi[branch] = mid;
            match explore(sys, columns, lo.clone(), lower_hi, nodes, budget) {
                Search::Exhausted => {}
                other => return other,
            }
            let mut upper_lo = lo;
            upper_lo[branch] = mid + 1;
            explore(sys, columns, upper_lo, hi, nodes, budget)
        }
    }
}

/// Interval bound propagation to a fixpoint. `None` when a domain empties,
/// which proves the sub-box holds no point.
///
/// For a row `Σ a_g w_g ≥ rhs` and one variable `f`, the rest of the row can
/// contribute at most `maxrest`, so `a_f w_f ≥ rhs − maxrest` — a bound that
/// tightens `f`'s interval. Rounding it onto the integer lattice is sound
/// because the search walks integers; and it never carries a verdict, so even a
/// defect here degrades to INCONCLUSIVE rather than to a wrong UNSAT (see
/// [`decide`]).
fn propagate(
    rows: &[Row],
    mut lo: Vec<i64>,
    mut hi: Vec<i64>,
    columns: &[usize],
) -> Option<(Vec<i64>, Vec<i64>)> {
    // A fixpoint over integer intervals terminates; the cap only bounds how
    // long a slow one may take before the search branches instead.
    for _ in 0..8 {
        let mut changed = false;
        for row in rows {
            let mut reach: i128 = 0;
            for &g in columns {
                let a = i128::from(row.coef[g]);
                if a != 0 {
                    reach += (a * i128::from(lo[g])).max(a * i128::from(hi[g]));
                }
            }
            if reach < i128::from(row.rhs) {
                return None;
            }
            for &f in columns {
                let a = i128::from(row.coef[f]);
                if a == 0 {
                    continue;
                }
                let own = (a * i128::from(lo[f])).max(a * i128::from(hi[f]));
                let need = i128::from(row.rhs) - (reach - own);
                if a > 0 {
                    let bound = ceil_div(need, a);
                    if bound > i128::from(lo[f]) {
                        if bound > i128::from(hi[f]) {
                            return None;
                        }
                        lo[f] = bound as i64;
                        changed = true;
                    }
                } else {
                    let bound = floor_div(need, a);
                    if bound < i128::from(hi[f]) {
                        if bound < i128::from(lo[f]) {
                            return None;
                        }
                        hi[f] = bound as i64;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    Some((lo, hi))
}

fn floor_div(n: i128, d: i128) -> i128 {
    let (q, r) = (n / d, n % d);
    if r != 0 && ((r < 0) != (d < 0)) {
        q - 1
    } else {
        q
    }
}

fn ceil_div(n: i128, d: i128) -> i128 {
    let (q, r) = (n / d, n % d);
    if r != 0 && ((r < 0) == (d < 0)) {
        q + 1
    } else {
        q
    }
}

/// §5.4's re-derivation for a relaxation-feasible, integer-infeasible verdict:
/// "exhaustive enumeration over the IIS's variable support, admissible only
/// when the PRODUCT of the support's box ranges is ≤ 10⁷ points ... otherwise
/// INCONCLUSIVE".
///
/// The enumeration uses NO pruning and NO propagation, which is the point: it
/// shares nothing with the search whose result it is confirming.
fn rederive_by_enumeration(sys: &System, columns: &[usize]) -> Verdict {
    let mut points: u128 = 1;
    for &c in columns {
        let range = u128::try_from(i128::from(sys.hi[c]) - i128::from(sys.lo[c]) + 1)
            .expect("a box interval is non-empty by construction");
        match points.checked_mul(range) {
            Some(p) if p <= ENUMERATION_BUDGET => points = p,
            _ => {
                return Verdict::Inconclusive {
                    reason: format!(
                        "the search closed the tree with no integer point, but §5.4 admits that \
                         verdict only when it is re-derived by exhaustive enumeration, and this \
                         support's box ranges exceed the registered {ENUMERATION_BUDGET}-point \
                         enumeration budget. The oracle did not decide, which is not \
                         infeasibility.",
                    ),
                }
            }
        }
    }

    let mut w = sys.v2_padded();
    let mut odometer = vec![0u128; columns.len()];
    for _ in 0..points {
        for (k, &c) in columns.iter().enumerate() {
            w[c] = sys.lo[c] + odometer[k] as i64;
        }
        if sys.violation(&w) == 0 {
            // The enumeration is the more trustworthy instrument, so its point
            // stands and the tree's "exhausted" was the defect.
            return Verdict::Sat { w };
        }
        for (k, &c) in columns.iter().enumerate() {
            let span = u128::try_from(i128::from(sys.hi[c]) - i128::from(sys.lo[c]) + 1)
                .expect("non-empty");
            odometer[k] += 1;
            if odometer[k] < span {
                break;
            }
            odometer[k] = 0;
        }
    }
    Verdict::Unsat {
        certificate: Certificate::Enumeration { support: columns.to_vec(), points },
    }
}

// ---------------------------------------------------------------------------
// B2 — the CHVÁTAL–GOMORY certification extension
// ---------------------------------------------------------------------------

/// **THE AUTHORITY**, printed with every certificate the extension builds.
const CG_AUTHORITY: &str = "B2 (guidingpath decision log, 2026-08-25, \"THE T4-ENTRY BATCH IS \
     RULED\"): the certification path is extended with Chvatal-Gomory row rounding -- derived \
     cuts (nonnegative rational combinations of the current system's rows with integral combined \
     coefficient vector, right-hand side rounded up in >=-form) join the QUESTION CHAIN put to \
     the frozen oracle; `decide`'s logic is untouched; every derived cut is recorded with its \
     parent rows and multipliers; the engine-local 10^7 per-question budget stands unchanged; \
     typed INCONCLUSIVE remains the honest residual.";

/// How many ROUNDS of rounding one derivation may run.
///
/// **Not a registered figure**, and the distinction matters as much here as it
/// does for [`REPAIR_ORACLE_BUDGET`]. These three caps are ENGINE-LOCAL choices
/// about how hard ONE derivation may look, nothing downstream reads them as
/// registered quantities, and exceeding one truncates the cut set
/// deterministically — never a verdict.
///
/// **A wording disagreement, recorded rather than resolved** (the D3/D6 style).
/// B2 says "the engine-local 10⁷ per-question budget stands unchanged", while
/// [`ENUMERATION_BUDGET`]'s in-source provenance — written at T3 and quoting
/// §5.4 directly — calls the same figure registered. This wave changed NEITHER
/// comment and neither value: the two spellings disagree about the 10⁷'s
/// provenance, not about the number, and which is right is the owner's to say.
/// The three caps here are engine-local under either reading.
const CG_MAX_ROUNDS: usize = 3;

/// How many cuts one ELIMINATION round may accept.
const CG_MAX_ELIMINATION_CUTS: usize = 64;

/// How many row PAIRS one elimination round may examine.
const CG_MAX_PAIRS: u64 = 4_000_000;

/// The widest support an elimination cut may carry.
///
/// A dense combination of two dense rows is a row the exact simplex pays for on
/// every pivot and almost never learns from; the useful eliminations are the
/// ones that cancel a column and leave a short row behind. This is a stated
/// heuristic about which PROPOSALS are worth asking about, and it can only cost
/// cuts: soundness is [`verify_cut`]'s, and completeness was never claimed.
const CG_MAX_CUT_SUPPORT: usize = 8;

/// **ONE DERIVED CUT**, with everything needed to re-derive it from the system
/// it was derived over.
#[derive(Clone, Debug, PartialEq, Eq)]
struct DerivedCut {
    /// How the cut reads, naming its parents and their multipliers.
    label: String,
    /// `(row index, multiplier)` for each parent, into the derivation POOL: the
    /// original inequalities first, then the earlier cuts in derivation order.
    /// Multipliers are nonnegative rationals.
    parents: Vec<(usize, Rat)>,
    /// `Σ y_i A_i`, which C–G requires to be INTEGRAL.
    coef: Vec<i64>,
    /// `⌈Σ y_i b_i⌉`.
    rhs: i64,
}

impl DerivedCut {
    /// The cut as a row the oracle can be asked about.
    fn row(&self) -> Row {
        Row {
            label: self.label.clone(),
            kind: RowKind::Cut,
            coef: self.coef.clone(),
            rhs: self.rhs,
            margin_vp: None,
        }
    }
}

/// **THE TRUST BOUNDARY FOR A CUT**, on the same discipline [`verify_farkas`]
/// applies to a Farkas combination: recompute the claim from the parents' own
/// row data in exact rationals and share nothing with the code that proposed it.
///
/// The three conditions ARE C–G's definition, and each is checked rather than
/// assumed: the multipliers are nonnegative, the combined coefficient vector is
/// integral and equals what the cut claims, and the right-hand side is exactly
/// the CEILING of the combined one. Any overflow is an `Err`, so an arithmetic
/// limit drops the cut rather than admitting an unchecked one.
fn verify_cut(pool: &[Row], cut: &DerivedCut) -> Result<(), String> {
    if cut.parents.is_empty() {
        return Err("a cut with no parent is not a combination of anything".to_string());
    }
    if cut.coef.iter().all(|c| *c == 0) {
        return Err("the combined coefficient vector is identically zero".to_string());
    }
    for (i, y) in &cut.parents {
        if *i >= pool.len() {
            return Err(format!("parent {i} is not a row of the {}-row pool", pool.len()));
        }
        if y.sign() <= 0 {
            return Err(format!("parent {i}'s multiplier is not strictly positive"));
        }
    }
    let overflow = || "i128 overflow while re-deriving the cut".to_string();
    let width = row_width(pool).max(cut.coef.len());
    for column in 0..width {
        let mut sum = Rat::ZERO;
        for (i, y) in &cut.parents {
            let a = i128::from(pool[*i].coef.get(column).copied().unwrap_or(0));
            let term = y.mul(Rat::int(a)).ok_or_else(overflow)?;
            sum = sum.add(term).ok_or_else(overflow)?;
        }
        let claimed = Rat::int(i128::from(cut.coef.get(column).copied().unwrap_or(0)));
        if sum != claimed {
            return Err(format!(
                "column {column} of the combination is {}/{} and the cut claims {}",
                sum.n,
                sum.d,
                cut.coef.get(column).copied().unwrap_or(0),
            ));
        }
    }
    let mut rhs = Rat::ZERO;
    for (i, y) in &cut.parents {
        let term = y.mul(Rat::int(i128::from(pool[*i].rhs))).ok_or_else(overflow)?;
        rhs = rhs.add(term).ok_or_else(overflow)?;
    }
    if i128::from(cut.rhs) != rhs.ceil() {
        return Err(format!(
            "the combined right-hand side is {}/{}, whose ceiling is {}, and the cut claims {}",
            rhs.n,
            rhs.d,
            rhs.ceil(),
            cut.rhs,
        ));
    }
    Ok(())
}

/// **THE WHOLE CHAIN, re-derived from the base system and the cuts' own
/// records.**
///
/// The pool is rebuilt here rather than passed in, so a cut whose parents name
/// rows that were not there when it was derived fails: the order the cuts are
/// listed in IS the order they were derived in, and this walks it.
fn verify_cut_chain(base: &[Row], cuts: &[DerivedCut]) -> Result<(), String> {
    let mut pool = base.to_vec();
    for (k, cut) in cuts.iter().enumerate() {
        verify_cut(&pool, cut).map_err(|e| format!("derived cut {} did not re-verify: {e}", k + 1))?;
        pool.push(cut.row());
    }
    Ok(())
}

/// One cut's label: the multipliers and the parent rows it rounds.
fn cut_label(pool: &[Row], parents: &[(usize, Rat)]) -> String {
    let terms: Vec<String> = parents
        .iter()
        .map(|(i, y)| {
            let m = if y.is_integer() { y.n.to_string() } else { format!("{}/{}", y.n, y.d) };
            format!("{m} x [{}]", pool[*i].label)
        })
        .collect();
    format!("C-G cut: ceil({})", terms.join(" + "))
}

/// **PROPOSE ONE CUT** from a set of parents and multipliers.
///
/// The steps are C–G's own, and each one is a reason a proposal is dropped:
///
/// 1. the combined coefficient vector must be INTEGRAL — that is the condition
///    that makes the left side an integer at an integer point, and without it
///    there is nothing to round;
/// 2. it must be nonzero — an all-zero row constrains nothing (and at a positive
///    right-hand side it would be a Farkas certificate the LP already has);
/// 3. the coefficient gcd is folded INTO the multipliers, which is the
///    strengthening step and not a separate operation: dividing `y` by `g`
///    leaves a combination whose coefficients are `coef/g`;
/// 4. the CEILING must BITE. An already-integral right-hand side means the
///    rounded row IS the combination, which the relaxation already implies, so
///    asking about it costs a column and learns nothing.
fn propose_cut(pool: &[Row], width: usize, parents: Vec<(usize, Rat)>) -> Option<DerivedCut> {
    let mut combined = vec![Rat::ZERO; width];
    let mut rhs = Rat::ZERO;
    for (i, y) in &parents {
        let row = pool.get(*i)?;
        for (column, slot) in combined.iter_mut().enumerate() {
            let a = i128::from(row.coef.get(column).copied().unwrap_or(0));
            if a == 0 {
                continue;
            }
            *slot = slot.add(y.mul(Rat::int(a))?)?;
        }
        rhs = rhs.add(y.mul(Rat::int(i128::from(row.rhs)))?)?;
    }
    if combined.iter().any(|x| !x.is_integer()) {
        return None;
    }
    let mut coef = Vec::with_capacity(width);
    for x in &combined {
        coef.push(i64::try_from(x.n).ok()?);
    }
    let magnitude =
        coef.iter().filter(|c| **c != 0).fold(0u128, |g, c| gcd(g, u128::from(c.unsigned_abs())));
    if magnitude == 0 {
        return None;
    }
    let g = i64::try_from(magnitude).ok()?;
    let (coef, parents, rhs) = if g > 1 {
        let mut scaled = Vec::with_capacity(parents.len());
        for (i, y) in parents {
            scaled.push((i, Rat::new(y.n, y.d.checked_mul(i128::from(g))?)?));
        }
        (coef.iter().map(|c| c / g).collect(), scaled, rhs.div(Rat::int(i128::from(g)))?)
    } else {
        (coef, parents, rhs)
    };
    if rhs.is_integer() {
        return None;
    }
    let rounded = i64::try_from(rhs.ceil()).ok()?;
    Some(DerivedCut { label: cut_label(pool, &parents), parents, coef, rhs: rounded })
}

/// What the pool ALREADY says, keyed by coefficient vector: the strongest
/// right-hand side it carries there.
///
/// A proposal no stronger than one of these adds nothing — the system already
/// implies it — and admitting it would cost the exact simplex a column for no
/// information. This is what keeps the elimination stage from filling its budget
/// with rounded box bounds.
fn strongest_rows(pool: &[Row], width: usize) -> HashMap<Vec<i64>, i64> {
    let mut strongest: HashMap<Vec<i64>, i64> = HashMap::new();
    for row in pool {
        let key: Vec<i64> =
            (0..width).map(|c| row.coef.get(c).copied().unwrap_or(0)).collect();
        strongest.entry(key).and_modify(|best| *best = (*best).max(row.rhs)).or_insert(row.rhs);
    }
    strongest
}

/// **STAGE A — SINGLE-ROW ROUNDING.** `Σ a_j w_j ≥ b` with `g = gcd(a)` becomes
/// `Σ (a_j/g) w_j ≥ ⌈b/g⌉`, at multiplier `1/g`.
///
/// This is the cheap half and, on the systems this file solves, the one that
/// does the work: a registered pair row whose realized `Δv` are ±2 against an
/// odd margin — 11(b)'s `2(w_E1 − w_S5) ≥ 1` is exactly that shape — is
/// LP-satisfied at a half-unit and integer-satisfied only at a whole one, and
/// that half-unit is where a displacement level goes integer-infeasible while
/// staying relaxation-feasible.
///
/// ONE pass is enough per pool: a rounded row has coefficient gcd 1 by
/// construction, so re-rounding it derives nothing.
fn single_row_cuts(pool: &[Row], width: usize) -> Vec<DerivedCut> {
    let strongest = strongest_rows(pool, width);
    let mut out = Vec::new();
    for (i, row) in pool.iter().enumerate() {
        let magnitude =
            row.coef.iter().filter(|c| **c != 0).fold(0u128, |g, c| {
                gcd(g, u128::from(c.unsigned_abs()))
            });
        let Ok(g) = i64::try_from(magnitude) else { continue };
        if g <= 1 {
            continue;
        }
        let Some(y) = Rat::new(1, i128::from(g)) else { continue };
        let Some(cut) = propose_cut(pool, width, vec![(i, y)]) else { continue };
        if admits(&strongest, &cut) {
            out.push(cut);
        }
    }
    out
}

/// **STAGE B — PAIRWISE ELIMINATION ROUNDING.**
///
/// For a pair of rows sharing a column at opposite signs, the multipliers that
/// CANCEL that column are `λ_i = |a_jc| / G` and `λ_j = |a_ic| / G` with
/// `G = gcd(|a_ic|, |a_jc|)` — both integers, so the combination is integral by
/// construction and the only question is whether its coefficient gcd fails to
/// divide the combined right-hand side. When it does, the ceiling bites and
/// there is a cut; when it does not, the combination is LP-implied and there is
/// nothing to ask.
///
/// Eliminating against a BOX row is included and is not a special case: the box
/// is in the pool like everything else, and `x ≥ −64` is a perfectly good thing
/// to eliminate a variable against.
///
/// Pairs are walked in ascending `(i, j)` order and columns in ascending order,
/// so the caps truncate a deterministic sequence rather than an arbitrary one.
fn elimination_cuts(pool: &[Row], width: usize) -> Vec<DerivedCut> {
    let strongest = strongest_rows(pool, width);
    let mut out = Vec::new();
    let mut pairs = 0u64;
    for i in 0..pool.len() {
        for j in (i + 1)..pool.len() {
            pairs += 1;
            if pairs > CG_MAX_PAIRS || out.len() >= CG_MAX_ELIMINATION_CUTS {
                return out;
            }
            for column in 0..width {
                let a = pool[i].coef.get(column).copied().unwrap_or(0);
                let b = pool[j].coef.get(column).copied().unwrap_or(0);
                if a == 0 || b == 0 || (a > 0) == (b > 0) {
                    continue;
                }
                let (am, bm) = (i128::from(a.unsigned_abs()), i128::from(b.unsigned_abs()));
                let Ok(shared) = i128::try_from(gcd(a.unsigned_abs().into(), b.unsigned_abs().into()))
                else {
                    continue;
                };
                let (Some(yi), Some(yj)) = (Rat::new(bm, shared), Rat::new(am, shared)) else {
                    continue;
                };
                let Some(cut) = propose_cut(pool, width, vec![(i, yi), (j, yj)]) else { continue };
                if cut.coef.iter().filter(|c| **c != 0).count() > CG_MAX_CUT_SUPPORT {
                    continue;
                }
                if admits(&strongest, &cut) {
                    out.push(cut);
                    break;
                }
            }
        }
    }
    out
}

/// Whether the pool does not already say this, or more.
fn admits(strongest: &HashMap<Vec<i64>, i64>, cut: &DerivedCut) -> bool {
    strongest.get(&cut.coef).is_none_or(|best| cut.rhs > *best)
}

/// The RATIONAL relaxation of a bare row list, decided exactly — the same
/// reading [`relaxation_feasible`] takes, over rows rather than over a
/// [`System`].
fn rows_relaxation_feasible(rows: &[Row]) -> Option<bool> {
    let columns = active_columns(rows);
    let row_index: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| columns.iter().any(|&j| r.coef.get(j).copied().unwrap_or(0) != 0))
        .map(|(i, _)| i)
        .collect();
    match relaxation(rows, &row_index, &columns) {
        Relaxation::Feasible => Some(true),
        Relaxation::Infeasible(_) => Some(false),
        Relaxation::Undecided(_) => None,
    }
}

/// **THE DERIVATION.** Stage A once, then elimination rounds while the
/// relaxation is still feasible and rounds remain.
///
/// Stopping as soon as the strengthened relaxation is EMPTY is not an
/// optimization with a correctness cost: a further round could only add rows the
/// Farkas certificate does not need, and every cut already accepted has been
/// verified.
///
/// A cut is accepted only after [`verify_cut`] re-derives it against the pool as
/// it stood, so a defect in any of the proposal machinery costs cuts and cannot
/// cost soundness.
fn derive_cuts(sys: &System) -> Vec<DerivedCut> {
    let base = sys.inequalities();
    let width = row_width(&base);
    let mut pool = base;
    let mut cuts: Vec<DerivedCut> = Vec::new();

    let stage_a = single_row_cuts(&pool, width);
    if accept_cuts(&mut pool, &mut cuts, stage_a) > 0
        && rows_relaxation_feasible(&pool) == Some(false)
    {
        return cuts;
    }
    for _ in 1..CG_MAX_ROUNDS {
        let stage_b = elimination_cuts(&pool, width);
        if accept_cuts(&mut pool, &mut cuts, stage_b) == 0 {
            break;
        }
        if rows_relaxation_feasible(&pool) == Some(false) {
            break;
        }
    }
    cuts
}

/// Verify a round's proposals and take the ones that stand.
///
/// The pool grows as they are taken, which leaves every already-recorded parent
/// index pointing at the row it pointed at: the list only ever appends.
fn accept_cuts(pool: &mut Vec<Row>, cuts: &mut Vec<DerivedCut>, fresh: Vec<DerivedCut>) -> usize {
    let mut accepted = 0;
    for cut in fresh {
        if verify_cut(pool, &cut).is_err() {
            continue;
        }
        pool.push(cut.row());
        cuts.push(cut);
        accepted += 1;
    }
    accepted
}

/// The phrase [`rederive_by_enumeration`] routes its over-budget INCONCLUSIVE
/// with — the ONE seam B2 extends.
const ENUMERATION_BUDGET_PHRASE: &str = "enumeration budget";

/// Whether an INCONCLUSIVE reason is the enumeration-budget route.
///
/// **A node-budget INCONCLUSIVE is a different fact and must not trigger the
/// extension**: the search ran out of nodes and the system may well be
/// satisfiable, so strengthening it would be spending a great deal of exact
/// arithmetic on a question nobody asked. Pinned against BOTH routes by
/// [`the_extension_triggers_on_the_enumeration_budget_route_and_not_the_node_one`],
/// so the coupling to the message is checked rather than assumed.
fn is_enumeration_budget_route(reason: &str) -> bool {
    reason.contains(ENUMERATION_BUDGET_PHRASE)
}

/// **B2's QUESTION CHAIN.** [`decide`] first, unchanged; then, at the
/// enumeration-budget seam ONLY, the same oracle asked again about a
/// strengthened system.
///
/// # What this does and does not touch
///
/// `decide` is called, never edited: the frozen oracle answers both questions
/// and this function chooses which questions to put. That is what B2's "cuts
/// join the QUESTION CHAIN put to the frozen oracle; `decide`'s logic is
/// untouched" is, spelled as code.
///
/// # Why each answer is the answer
///
/// * **UNSAT on the strengthened system** certifies the original: every cut is
///   valid for every integer point of the original (see
///   [`Certificate::ChvatalGomory`]), so a strengthened system with no point at
///   all is an original system with no integer point.
/// * **SAT on the strengthened system** is a point of the original too — its
///   rows are a superset — and it is re-checked against the original rows and
///   box here before it is returned, on the same discipline the oracle applies
///   to its own points.
/// * **Anything else** leaves the typed INCONCLUSIVE standing, with the
///   extension's own outcome appended. B2: "typed INCONCLUSIVE remains the
///   honest residual if certification still cannot complete."
fn decide_with_cuts(sys: &System) -> Verdict {
    let verdict = decide(sys);
    let Verdict::Inconclusive { reason } = &verdict else { return verdict };
    if !is_enumeration_budget_route(reason) {
        return verdict;
    }
    let cuts = derive_cuts(sys);
    if cuts.is_empty() {
        return Verdict::Inconclusive {
            reason: format!(
                "{reason} A Chvatal-Gomory extension ({CG_AUTHORITY}) derived no cut over this \
                 system, so no strengthened question was asked.",
            ),
        };
    }
    // THE TRUST BOUNDARY, once more before the question is asked: the chain is
    // re-derived from the base system and the cuts' own parent records, sharing
    // nothing with the derivation that proposed them.
    let base = sys.inequalities();
    if let Err(e) = verify_cut_chain(&base, &cuts) {
        return Verdict::Inconclusive {
            reason: format!(
                "{reason} A Chvatal-Gomory extension was attempted and its derivation did not \
                 re-verify ({e}), so no cut was asked and nothing is claimed.",
            ),
        };
    }
    let mut strengthened = sys.clone();
    for cut in &cuts {
        strengthened.rows.push(cut.row());
    }
    match decide(&strengthened) {
        Verdict::Unsat { certificate } => Verdict::Unsat {
            certificate: Certificate::ChvatalGomory { cuts, inner: Box::new(certificate) },
        },
        Verdict::Sat { w } => {
            let inside = (0..sys.lo.len()).all(|f| w[f] >= sys.lo[f] && w[f] <= sys.hi[f]);
            if sys.violation(&w) == 0 && inside {
                Verdict::Sat { w }
            } else {
                Verdict::Inconclusive {
                    reason: format!(
                        "{reason} A Chvatal-Gomory extension's strengthened system returned a \
                         point that does not satisfy the ORIGINAL rows, so nothing is claimed.",
                    ),
                }
            }
        }
        Verdict::Inconclusive { reason: residual } => Verdict::Inconclusive {
            reason: format!(
                "{reason} A Chvatal-Gomory extension ({CG_AUTHORITY}) derived {} verified cut(s) \
                 and the strengthened system still did not decide: {residual}",
                cuts.len(),
            ),
        },
    }
}

// ---------------------------------------------------------------------------
// §5.4 (3) — minimization, inspection, and the two scale diagnostics
// ---------------------------------------------------------------------------

/// §5.4's box-widening diagnostic: "re-decide at |w| ≤ 128".
const WIDENED_BOX_LIMIT: i64 = 128;

/// §5.4's early-rescale ladder: "the LCD of its margin_vp values, equivalently
/// `w_vp(early) ∈ {1, 2, 3, 6}`".
const EARLY_RESCALE_LADDER: [i64; 4] = [1, 2, 3, 6];

/// The sentence §5.4 requires the minimized set to be read as, printed verbatim.
///
/// "The artifact is an **irreducible infeasible subsystem (IIS),
/// order-dependent, not necessarily minimum-cardinality** — stated in those
/// words."
const IIS_READING: &str = "THIS SET IS AN IRREDUCIBLE INFEASIBLE SUBSYSTEM (IIS), ORDER-DEPENDENT, \
     AND NOT NECESSARILY MINIMUM-CARDINALITY. A different deletion order can yield a different \
     set of a different size; this one is §5.2's literal document order, then the hand-ordering \
     rows.";

/// What deletion-minimization produced.
#[derive(Clone, Debug)]
struct Minimization {
    /// The IIS, in deletion order — the rows that survived every probe.
    kept: Vec<Row>,
    /// The rows deletion removed, in the order it removed them.
    dropped: Vec<String>,
    /// Set when an INCONCLUSIVE child HALTED the minimization. §5.4: the
    /// artifact then "reports the current (non-minimal) infeasible set with the
    /// INCONCLUSIVE named".
    halted: Option<String>,
}

/// **§5.4 (3) — deletion-minimization against the exact oracle.**
///
/// One pass in deletion order. A row whose removal leaves the system UNSAT was
/// not needed and stays out; a row whose removal makes it SAT is needed and goes
/// back. An INCONCLUSIVE child HALTS the pass — §5.4 is explicit that §5B
/// inspection then proceeds on a non-minimal set with that caveat, "never on a
/// completed-looking core".
fn minimize(sys: &System) -> Minimization {
    let mut alive = vec![true; sys.rows.len()];
    let mut dropped = Vec::new();
    let mut halted = None;

    for probe in 0..sys.rows.len() {
        alive[probe] = false;
        let trial = System {
            rows: sys
                .rows
                .iter()
                .zip(&alive)
                .filter(|(_, keep)| **keep)
                .map(|(r, _)| r.clone())
                .collect(),
            ..sys.clone()
        };
        match decide(&trial) {
            // Still infeasible without it, so it was not needed.
            Verdict::Unsat { .. } => dropped.push(sys.rows[probe].label.clone()),
            // Its removal relieves the infeasibility: it is needed.
            Verdict::Sat { .. } => alive[probe] = true,
            Verdict::Inconclusive { reason } => {
                alive[probe] = true;
                halted = Some(format!(
                    "the probe that removed `{}` came back INCONCLUSIVE ({reason}), so the pass \
                     STOPPED. What follows is the current set, which is infeasible but NOT \
                     minimal -- §5B inspection proceeds on it with that caveat, never on a \
                     completed-looking core.",
                    sys.rows[probe].label,
                ));
                break;
            }
        }
    }

    let kept = sys
        .rows
        .iter()
        .zip(&alive)
        .filter(|(_, keep)| **keep)
        .map(|(r, _)| r.clone())
        .collect();
    Minimization { kept, dropped, halted }
}

/// §5.4's two registered scale diagnostics, run on an UNSAT.
///
/// Neither deletes anything: "the `w_vp` pins and the box are NOT
/// deletion-eligible but are probed by two registered diagnostics".
#[derive(Clone, Debug)]
struct ScaleDiagnostics {
    /// Re-decided at |w| ≤ 128. SAT under widening is a SCALE finding under
    /// checkpoint §5B.
    widened: Verdict,
    /// Early systems only: re-decided along the `w_vp(early) ∈ {1, 2, 3, 6}`
    /// ladder. SAT under rescale is integer granularity, "a SCALE finding,
    /// never architecture".
    rescaled: Vec<(i64, Verdict)>,
}

/// **The system re-priced at a diagnostic pin.**
///
/// §5.1 makes a MARGINED margin `max(1, ceil(margin_vp × w_vp(stage)))`, so it
/// is a FUNCTION of the pin and moving the pin without re-evaluating it changes
/// what the row claims. Leaving the shipped integers in place UNDERSTATES every
/// margined row at rungs above 1 — 3(c) would ask for 1 where the formula asks
/// for 2 — and an understated system can come back SAT, which would label a
/// real infeasibility a SCALE finding. That is anti-conservative in exactly the
/// direction checkpoint §5B cares about, so the ladder re-prices.
///
/// Rows carrying no fraction are left alone: DIRECTION-ONLY's 1 and WEAK's 0 are
/// not prices, and the box, pin and hand-ordering rows are not margins at all.
fn repriced_at(sys: &System, w_vp: i64) -> System {
    let rows = sys
        .rows
        .iter()
        .map(|row| {
            let mut row = row.clone();
            if let Some((numerator, denominator)) = row.margin_vp {
                row.rhs = margined_integer(numerator, denominator, w_vp);
            }
            row
        })
        .collect();
    System::new(sys.stage, rows, sys.box_limit, w_vp)
}

fn scale_diagnostics(sys: &System) -> ScaleDiagnostics {
    let widened =
        decide(&System::new(sys.stage, sys.rows.clone(), WIDENED_BOX_LIMIT, sys.pin_vp));

    // §5.4 scopes the rescale to "an IIS lying wholly in the early system".
    // Since the system decomposes per stage, that is exactly a system whose
    // stage is early.
    let rescaled = match sys.stage {
        StageEndpoint::Late => Vec::new(),
        StageEndpoint::Early => EARLY_RESCALE_LADDER
            .into_iter()
            .map(|pin| (pin, decide(&repriced_at(sys, pin))))
            .collect(),
    };
    ScaleDiagnostics { widened, rescaled }
}

/// §5.4's witness reporting owes "the count of constraints implied by the rest
/// (drop-one re-test)".
///
/// Read RIGOROUSLY: row `c` is IMPLIED by the rest when the rest plus the
/// NEGATION of `c` is infeasible. Over the integers the negation of
/// `Σ a·w ≥ rhs` is `Σ a·w ≤ rhs − 1`, so the probe is an exact-oracle question
/// and its answer carries the oracle's trichotomy. The weaker reading — drop the
/// row, re-run the descent, see whether the point still satisfies it — would
/// report a property of the search, and §5.4 already has one of those.
#[derive(Clone, Debug, Default)]
struct ImplicationReport {
    implied: Vec<String>,
    independent: Vec<String>,
    undecided: Vec<String>,
}

fn implied_by_the_rest(sys: &System) -> ImplicationReport {
    let mut report = ImplicationReport::default();
    for (probe, row) in sys.rows.iter().enumerate() {
        let mut rows: Vec<Row> = sys
            .rows
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != probe)
            .map(|(_, r)| r.clone())
            .collect();
        // The negation, over the integers: `Σ a·w ≥ rhs` fails exactly when
        // `Σ a·w ≤ rhs − 1`, which is `−Σ a·w ≥ 1 − rhs`.
        rows.push(Row {
            label: format!("NOT ({})", row.label),
            kind: RowKind::Pair { id: "negation".to_string(), order: usize::MAX },
            coef: row.coef.iter().map(|c| -c).collect(),
            rhs: 1 - row.rhs,
            // A negation is not a registered margined row, so it does not
            // re-price under the rescale ladder.
            margin_vp: None,
        });
        let probed = System { rows, ..sys.clone() };
        match decide(&probed) {
            Verdict::Unsat { .. } => report.implied.push(row.label.clone()),
            Verdict::Sat { .. } => report.independent.push(row.label.clone()),
            Verdict::Inconclusive { .. } => report.undecided.push(row.label.clone()),
        }
    }
    report
}

// ---------------------------------------------------------------------------
// The real path (§7 step 4's Part-B run — built here, invoked after the freeze)
// ---------------------------------------------------------------------------

/// The eighteen candidate columns for one seat of one view.
///
/// **This is a SECOND implementation of a seam `pair_fixtures` declares and
/// deliberately does not own.** `pair_fixtures::CandidateColumns`
/// (`game/pair_fixtures.rs:5287`) is a function POINTER type — "step 3 builds
/// the eighteen helpers in `eval.rs` (spec §13) and passes an adapter of this
/// shape" — and `eval_probe`'s adapter is private to that example target, which
/// no other target can call. The helpers themselves are shared: every arm below
/// dispatches to `ScoreStructure`, and this file derives nothing of its own.
///
/// # The drift defences, and the blind spot between them
///
/// THREE defences, because the first two have a shared gap and saying so is the
/// only way to know whether the third is needed:
///
/// 1. The extraction's `expected_nonzero` check ([`row_from_signatures`]): a
///    swapped arm puts a nonzero at a row the registration does not list, and
///    the real run REFUSES naming it rather than solving.
/// 2. An arm-by-arm restatement test against the helpers each label names.
/// 3. A SOURCE-LEVEL pin that reads this function's own match arms and checks
///    each against `CandidateFeature::name()`.
///
/// **THE BLIND SPOT the first two share.** Defence 1 only fires when a swap
/// moves a value to a row the registration does not list. Two features that are
/// BOTH listed nonzero on the same pair could be swapped with each other and
/// every row would still land on a listed row — the coefficients would be wrong
/// and nothing would refuse. Defence 2 is what has to catch a swap of that
/// shape, and defence 2 is a RESTATEMENT: it compares this match against a
/// second hand-written copy of the same mapping, so a swap made identically in
/// both is invisible to it.
///
/// That is why defence 3 exists. Its reference is not a copy of anything: it is
/// `CandidateFeature::name()`, which lives in the frozen package file and whose
/// eighteen snake_case strings are exactly the helper method names. A swap
/// between any two arms — however well-matched the restatement, however both
/// rows are listed — makes some arm call a method whose name is not its
/// variant's, and the pin fails.
fn candidate_dispatch(
    s: &ScoreStructure,
    view: &PlayerView,
    player: &VisiblePlayer,
    feature: CandidateFeature,
    rules: &Rules,
) -> i64 {
    match feature {
        CandidateFeature::ProductionAccess => s.production_access(view, player),
        CandidateFeature::TrackAccess => s.track_access(view, player),
        CandidateFeature::AssistantAreasDistinct => s.assistant_areas_distinct(player),
        CandidateFeature::PresidentLocalAccess => s.president_local_access(player),
        CandidateFeature::BankAccess => s.bank_access(view, player),
        CandidateFeature::EmploymentAgencyAccess => s.employment_agency_access(view, player),
        CandidateFeature::PresidentReachableAreas => s.president_reachable_areas(player),
        CandidateFeature::MoneyStock => s.money_stock(player),
        CandidateFeature::TradeGoodsStock => s.trade_goods_stock(player),
        CandidateFeature::BuildingTokensReady => s.building_tokens_ready(player),
        CandidateFeature::HeldOrderValue => s.held_order_value(player, rules),
        CandidateFeature::OrderMatchedGoods => s.order_matched_goods(player, rules),
        CandidateFeature::ExecutableOrders => s.executable_orders(player, rules),
        CandidateFeature::FlagIconMarginal => s.flag_icon_marginal(player, rules),
        CandidateFeature::HeldOrderIcons => s.held_order_icons(player, rules),
        CandidateFeature::LabTakeable => s.lab_takeable(view, player),
        CandidateFeature::PortTakeable => s.port_takeable(view, player, rules),
        CandidateFeature::MatchedProductionAccess => s.matched_production_access(view, player, rules),
    }
}

/// One view's 34-wide MOVER-RELATIVE vector, `mine − theirs`.
///
/// The same quantity `eval_probe`'s §5.3 signature takes, and for the same
/// reason: a weight multiplies `mine − theirs` (`game/eval.rs`), so §5.4's row
/// `Σ_f w_f·Δv_f ≥ margin` is the DIFFERENCE of two of these.
fn relative_vector(view: &PlayerView, mover: PlayerId, rules: &Rules) -> Result<Vec<i64>, String> {
    let me = view
        .players
        .iter()
        .find(|p| p.id == mover)
        .ok_or_else(|| format!("seat {} is not a player in this view", mover.0))?;
    let opponent = view.players.iter().find(|p| p.id != mover && !p.id.is_dummy());
    let structure = ScoreStructure::derive(view, rules);
    let mut v = vec![0i64; AXIS_WIDTH];
    for &f in &Feature::ALL[..Feature::INCUMBENT_COUNT] {
        let theirs = opponent.map_or(0, |o| structure.value_of(o, f));
        v[FeatureId::Incumbent(f).index()] = structure.value_of(me, f) - theirs;
    }
    for c in CandidateFeature::ALL {
        let mine = candidate_dispatch(&structure, view, me, c, rules);
        let theirs =
            opponent.map_or(0, |o| candidate_dispatch(&structure, view, o, c, rules));
        v[FeatureId::Candidate(c).index()] = mine - theirs;
    }
    Ok(v)
}

/// Turn one pair's two signed sides into its registered row, or REFUSE.
///
/// The refusals are the extraction's whole safety story, and each names its
/// clause:
///
/// * a nonzero at a row the registration does not list is §5.1's "unlisted
///   nonzero row at run time is a fixture defect";
/// * an identically-zero row against a positive margin is unsatisfiable by
///   construction and would poison the system silently — Part A's zero-blocking
///   result says no registered row is one, so meeting one here is a finding;
/// * a missing margin is [`margin_of`]'s refusal.
fn row_from_signatures(
    pair: &'static PairDeclaration,
    stage: StageEndpoint,
    greater: &[i64],
    lesser: &[i64],
) -> Result<Row, String> {
    if greater.len() != AXIS_WIDTH || lesser.len() != AXIS_WIDTH {
        return Err(format!(
            "{} at {}: the axis is {AXIS_WIDTH} wide and the sides are {} and {}",
            pair.id,
            stage.name(),
            greater.len(),
            lesser.len(),
        ));
    }
    // §5.1: "a declared GREATER SIDE (named FIRST in every registration;
    // Δv = greater − lesser)".
    let coef: Vec<i64> = greater.iter().zip(lesser).map(|(g, l)| g - l).collect();

    let unlisted: Vec<&str> = FeatureId::all()
        .into_iter()
        .filter(|id| coef[id.index()] != 0 && !pair.expected_nonzero.contains(id))
        .map(|id| id.label())
        .collect();
    if !unlisted.is_empty() {
        return Err(format!(
            "{} at {}: rows {unlisted:?} are nonzero but the registration does not list them. \
             §5.1 makes an unlisted nonzero row a FIXTURE DEFECT, sanctioned to fix and re-freeze \
             before covered results -- so this run REFUSES rather than solving a system the \
             package did not register. (It is equally the extraction's own alarm: a candidate \
             column dispatched to the wrong helper lands here.)",
            pair.id,
            stage.name(),
        ));
    }

    let margin = margin_of(pair, stage)?;
    if coef.iter().all(|c| *c == 0) {
        return Err(format!(
            "{} at {}: the row is identically zero against margin {margin}. Part A returned ZERO \
             blocking collisions, which says no registered row is identically zero, so this is a \
             finding rather than a row -- and at a positive margin it would be an unsatisfiable \
             row nothing could repair.",
            pair.id,
            stage.name(),
        ));
    }

    let order = document_order(pair.id)
        .ok_or_else(|| format!("{} is not in the registry", pair.id))?;
    Ok(Row {
        label: format!("pair {} {}", pair.id, stage.name()),
        kind: RowKind::Pair { id: pair.id.to_string(), order },
        coef,
        rhs: margin,
        // MARGINED rows carry the fraction their integer was evaluated from, so
        // §5.4's rescale ladder can re-evaluate it when it moves the pin.
        margin_vp: margin_fraction(pair.id),
    })
}

/// Assemble one stage's system: extracted pair rows in DELETION ORDER, then the
/// two hand-ordering rows, over the box with `w_vp` pinned.
///
/// Separate from the extraction so the assembly can be exercised on synthetic
/// rows: §5.4's freeze-before-solve clause keeps the frozen thirty-four out of
/// this batch entirely, and a wiring test that had to build them would break it.
fn assemble(stage: StageEndpoint, pair_rows: Vec<Row>, box_limit: i64) -> System {
    let mut rows = pair_rows;
    rows.sort_by_key(|r| match &r.kind {
        RowKind::Pair { order, .. } => *order,
        _ => usize::MAX,
    });
    rows.extend(toys::hand_ordering_rows());
    System::new(stage, rows, box_limit, toys::pinned_vp(stage))
}

/// Extract one registered pair's row at one endpoint, through the frozen
/// package's own public surface.
///
/// **NOT EXERCISED IN THIS BATCH, and that is the point.** §5.4's
/// freeze-before-solve clause says "development uses toy systems only"; the
/// three functions this one composes — [`build_registered_pair`],
/// [`relative_vector`], [`row_from_signatures`] — are each tested on their own,
/// and this is the glue that would touch the frozen thirty-four.
fn extract_row(
    pair: &'static PairDeclaration,
    stage: StageEndpoint,
    rules: &Rules,
) -> Result<Row, String> {
    let fixture = build_registered_pair(pair, stage, rules)
        .map_err(|e| format!("{} at {}: {e:?}", pair.id, stage.name()))?;
    let subject = fixture.subject();
    let greater = relative_vector(fixture.view(Side::Greater), subject, rules)
        .map_err(|e| format!("{} greater: {e}", pair.id))?;
    let lesser = relative_vector(fixture.view(Side::Lesser), subject, rules)
        .map_err(|e| format!("{} lesser: {e}", pair.id))?;
    row_from_signatures(pair, stage, &greater, &lesser)
}

/// One toy row as the builder takes it: label, sparse `(index, coef)` entries,
/// and the right-hand side.
type ToyRow<'a> = (&'a str, &'a [(usize, i64)], i64);

/// The same, owning its label and entries — what a GENERATED toy builds before
/// it borrows them back into [`ToyRow`]s.
type OwnedToyRow = (String, Vec<(usize, i64)>, i64);

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
                    margin_vp: None,
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
                Row {
                    label: label.to_string(),
                    kind: RowKind::HandOrdering,
                    coef,
                    rhs: 1,
                    margin_vp: None,
                }
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
            // A three-row cycle: no PAIR of rows contradicts, so a single-row
            // interval test cannot see it and the certificate needs all three
            // (Farkas y = (1, 1, 1), which sums the coefficients to zero and
            // the margins to 3).
            "unsat-cycle" => toy(
                StageEndpoint::Early,
                &[
                    ("S1 over S2", &[(s1, 1), (col("S2"), -1)], 1),
                    ("S2 over S3", &[(col("S2"), 1), (col("S3"), -1)], 1),
                    ("S3 over S1", &[(col("S3"), 1), (s1, -1)], 1),
                ],
                BOX_LIMIT,
            ),
            // §5.4's registered "known-UNSAT (incl. a fractional-margin-derived
            // system)". Both rows come from ONE amortization claim -- E1 is
            // worth 1/2 VP per yen, or here 1/3 for the arithmetic -- read at
            // the two ends §5.1 reads it at:
            //
            //   * the MARGIN, strengthened by §5.1's early floor from 1/3 to
            //     `max(1, ceil(1/3 x 1)) = 1`, giving `w_E1 >= 1`;
            //   * the RATE itself, which says three units of E1 are worth no
            //     more than one VP: `-3 w_E1 + w_vp >= 0`.
            //
            // With `w_vp` pinned to 1 the rate caps `w_E1` at 1/3 while the
            // floored margin demands 1, so the system is infeasible AT THE
            // RATIONAL RELAXATION and ships a Farkas certificate. It is the
            // granularity §5.1 warns about made concrete: the contradiction is
            // the 3x floor, not the architecture -- which is why §5.4 keeps an
            // early-rescale diagnostic, and this toy is what fires it.
            "unsat-fractional-margin" => {
                let mut built = toy(
                    StageEndpoint::Early,
                    &[
                        ("E1 floored margin", &[(e1, 1)], 1),
                        (
                            "the 1/3 VP rate itself",
                            &[(e1, -3), (Feature::VictoryPoints.index(), 1)],
                            0,
                        ),
                    ],
                    BOX_LIMIT,
                );
                // The margin row DECLARES the rate it was floored from, so the
                // rescale ladder re-prices it exactly as it will re-price a
                // registered MARGINED row. Without this the toy would exercise
                // the diagnostic's uncalibrated half and prove nothing about
                // the calibrated one.
                built.rows[0].margin_vp = Some((1, 3));
                built
            }
            // Relaxation-FEASIBLE (w_S1 = 1/2 satisfies both rows) and
            // integer-infeasible. No Farkas certificate exists, so this is the
            // toy that exercises §5.4's other admitted shape: the exhaustive
            // enumeration, here over a single column of 129 points, well inside
            // the registered budget.
            "unsat-integer-only" => toy(
                StageEndpoint::Early,
                &[("twice S1 at least 1", &[(s1, 2)], 1), ("twice S1 at most 1", &[(s1, -2)], -1)],
                BOX_LIMIT,
            ),
            // UNSAT at |w| <= 64 (the widest reachable spread is 128) and SAT
            // at |w| <= 128 -- §5.4's box-widening diagnostic has something to
            // find.
            "unsat-box-widening" => toy(
                StageEndpoint::Early,
                &[("S1 far over S2", &[(s1, 1), (col("S2"), -1)], 2 * BOX_LIMIT + 1)],
                BOX_LIMIT,
            ),
            // **A SCALE PROBE, not a model of anything.** Twenty-six rows over
            // all thirty-four columns — the early system's shape — with
            // coefficients from a fixed arithmetic formula rather than from any
            // fixture. It answers the one question the real run needs answered
            // in advance and no small toy can: does the exact rational simplex
            // stay tractable at full width? Deterministic, like everything else
            // here: the formula takes no seed and draws no entropy.
            "scale-probe-full-width" => {
                let rows: Vec<OwnedToyRow> = (0..26)
                    .map(|r: usize| {
                        // The row multiplier is coprime to the modulus on
                        // purpose: a multiple of 7 would cancel and give every
                        // row the same coefficients, which is a wide system
                        // with one distinct row in it.
                        let entries = (0..AXIS_WIDTH)
                            .map(|c| (c, ((r * 5 + c * 13 + 5) % 7) as i64 - 3))
                            .filter(|(_, a)| *a != 0)
                            .collect();
                        (format!("scale probe row {r}"), entries, 1)
                    })
                    .collect();
                let borrowed: Vec<ToyRow> =
                    rows.iter().map(|(l, e, r)| (l.as_str(), e.as_slice(), *r)).collect();
                toy(StageEndpoint::Early, &borrowed, BOX_LIMIT)
            }
            _ => return None,
        })
    }

    /// Every registered toy's name, in the order [`named`] lists them.
    pub const NAMES: [&str; 10] = [
        "standing-only",
        "sat-reachable",
        "sat-coupled",
        "unsat-direct",
        "unsat-box",
        "unsat-cycle",
        "unsat-fractional-margin",
        "unsat-integer-only",
        "unsat-box-widening",
        "scale-probe-full-width",
    ];
}

// ---------------------------------------------------------------------------
// §9's registered-margin repair -- R3's certified exact simultaneous per-stage
// projection
// ---------------------------------------------------------------------------

/// **R3'S RULING SENTENCE**, which this section implements and which the module
/// doc of every function below is written against
/// (`docs/plans/2026-08-24-step9-charter.md`:117-127):
///
/// > "§9's registered-margin repair is executed, per endpoint, as ONE exact
/// > integer projection: over the full R1 system with removed features fixed at
/// > 0 and the pin fixed, find the assignment minimizing (lexicographically)
/// > total |displacement| from the draft's starting vector, then the count of
/// > changed coordinates, then ties by the canonical `Feature::ALL` ×
/// > early-then-late order — subject additionally to: no coordinate moves FROM a
/// > prior-agreeing sign to an opposed one, and no opposed coordinate moves
/// > further from agreement. The optimum is certified over the finite integer
/// > box by the frozen solver's exact-oracle machinery; a heuristic answer is
/// > insufficient; infeasibility is a typed `REPAIR-INFEASIBLE` routed to the
/// > owner, never a silent relaxation. Every changed coordinate is recorded with
/// > the binding rows that priced it."
///
/// # THE THREE LEVELS, and how each is CERTIFIED
///
/// The projection is a chain of feasibility questions, every one of them put to
/// [`decide`] — the frozen §5.4 oracle, unchanged. Nothing here decides
/// anything: this module chooses which questions to ask and in what order, and
/// the oracle answers them. That is what makes "a heuristic answer is
/// insufficient" a property of the code rather than a claim about it.
///
/// 1. **DISPLACEMENT.** "Is there a feasible point within `d` of the draft?" is
///    asked of an AUGMENTED system ([`augmented`]) carrying one extra column per
///    movable coordinate that prices `|w_f − w0_f|`, plus the budget row
///    `Σ|w_f − w0_f| ≤ d`. `D*` is the least `d` whose answer is SAT, found by
///    bisection between a level with a witness and a level with a VERIFIED
///    infeasibility certificate. Every level below `D*` is empty by the
///    certificate at `D* − 1` and the one-line monotonicity argument
///    ([`REPAIR_MONOTONICITY`]).
/// 2. **CHANGED COUNT.** At `D*`, "is there a feasible point that moves only the
///    coordinates in `S`?" is asked for every support `S` of size `k`, `k`
///    ascending, in ascending listed order. A level whose every support came
///    back UNSAT is CERTIFIED EMPTY BY EXHAUSTION — the enumeration is the
///    certificate, on the same discipline as [`rederive_by_enumeration`], and
///    over budget is INCONCLUSIVE rather than a claim.
/// 3. **THE CANONICAL REPRESENTATIVE.** With `D*` and the support fixed, each
///    coordinate is fixed in listed order to the least `(|w_f − w0_f|, w_f)` that
///    keeps the system feasible — each fixing a SAT, each rejected value an
///    UNSAT. See [`REPAIR_CANONICAL`], which states the rule the ruling's
///    "canonical `Feature::ALL` order" is realized as.
///
/// **The stages do not couple** (spec §5.4:745-747), so `early-then-late` is the
/// order the endpoints are RUN in and nothing more: each endpoint is one
/// independent projection.
const REPAIR_RULING: &str = "R3: ONE exact integer projection per endpoint -- lexicographically \
     minimal |displacement| from the draft, then changed-coordinate count, then the canonical \
     listed order; no coordinate moves from a prior-agreeing sign to an opposed one and no \
     opposed coordinate moves further from agreement; certified over the finite integer box by \
     the frozen exact oracle; REPAIR-INFEASIBLE is typed and routed, never a silent relaxation.";

/// Why ONE certificate closes every level below `D*`.
const REPAIR_MONOTONICITY: &str = "Every displacement level below the one certified empty here is \
     itself empty: a point with total |displacement| <= d also has total |displacement| <= d' for \
     every d' >= d, so the feasible sets are nested and emptiness propagates downward. One \
     verified certificate is therefore the whole tail.";

/// **THE REGISTERED CANONICAL RULE**, stated because the ruling names an order
/// and an order is not yet a selection.
const REPAIR_CANONICAL: &str = "CANONICAL SELECTION (registered): among the optima at the optimal \
     displacement and changed count, the support is the FIRST in ascending `Feature::ALL` order \
     (compare the supports' sorted coordinate indices lexicographically); within that support, \
     each coordinate is fixed in ascending `Feature::ALL` order to the least (|w_f - w0_f|, w_f) \
     that leaves the system feasible, certified at each fixing. The value key's second component \
     is there to make the rule TOTAL; over a convex projected feasible set it never fires, and \
     over the integers it can.";

/// **THE REGISTERED REMOVAL SET.** The four candidates §8's selected interaction
/// cell removed: `E3`, `O3`, `O6`, `X3`.
///
/// Provenance: the 2026-08-24 guidingpath rulings row (step 9's T0 registration,
/// `docs/plans/2026-08-24-step9-charter.md`) over the step-6 close, whose
/// artifact of record prints the same four —
/// `docs/temp/step6-artifacts/v3l-final-artifact.txt:274`.
///
/// **This is a second SPELLING of `eval_probe`'s `STEP9_REMOVED`, and it is held
/// to it by a PIN rather than by a shared constant.** The two live in different
/// example binaries, which cannot import one another. What holds them together
/// is
/// `the_projected_system_reproduces_the_banked_artifact_s_kept_and_dropped_rows`:
/// this file's projection must reproduce the banked artifact's kept/dropped row
/// names and counts and satisfy its witness, which is the same reading the other
/// encoding is pinned to. A drift in either spelling breaks that test.
const REPAIR_REMOVED: [CandidateFeature; 4] = [
    CandidateFeature::BuildingTokensReady,
    CandidateFeature::ExecutableOrders,
    CandidateFeature::HeldOrderIcons,
    CandidateFeature::MatchedProductionAccess,
];

/// The projection's own budget, in its own deterministic currency: ORACLE CALLS.
///
/// The oracle counts nodes inside each call ([`NODE_BUDGET`]); this counts the
/// calls. Both are counts of work done, not of time spent — nothing in the
/// decision path reads a clock, so the same inputs spend the same budget on
/// every machine. Exhaustion is INCONCLUSIVE, routed; it is never infeasibility.
///
/// **THIS NUMBER IS NOT REGISTERED, and the distinction is worth keeping.**
/// §5.4 registers [`NODE_BUDGET`] and [`ENUMERATION_BUDGET`] at 10⁷ each; those
/// figures come from the spec. `20_000` is an ENGINE-LOCAL cap chosen at T3 for
/// a currency §5.4 does not name — how many times the projection may ask, as
/// opposed to how hard the oracle may look at one question. Nothing downstream
/// reads it as a registered quantity, and the printed text does not call it one.
const REPAIR_ORACLE_BUDGET: u64 = 20_000;

/// The support enumeration's budget, in enumerated subsets — [`ENUMERATION_BUDGET`]'s
/// discipline for level 2's certificate. Over budget is INCONCLUSIVE: an
/// enumeration that did not finish has certified nothing.
const REPAIR_SUPPORT_BUDGET: u64 = 10_000_000;

/// The starting-vector specs `repair --start` accepts.
const REPAIR_START_SPECS: [&str; 2] = ["v2-padded", "zero"];

/// **§4'S PROJECTION, on the solver's own rows.** A row survives iff every
/// candidate column it carries a nonzero `Δv` in survives the removal set.
///
/// The same admission `eval_probe`'s `row_applies` runs, spelled over `Row`
/// instead of over `GateRow`: "a row survives a cell iff its whole candidate
/// collateral is inside that cell's set". Incumbent columns never drop a row —
/// every identity holds all sixteen.
///
/// The dropped rows are returned BY NAME because R3's report has to print them:
/// a repair that quietly solved a smaller system than the one it named would be
/// the same defect as a silent relaxation.
fn project_pair_rows(rows: Vec<Row>) -> (Vec<Row>, Vec<String>) {
    let removed: Vec<usize> =
        REPAIR_REMOVED.iter().map(|c| FeatureId::Candidate(*c).index()).collect();
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    for row in rows {
        if removed.iter().any(|&slot| row.coef[slot] != 0) {
            dropped.push(row.label.clone());
        } else {
            kept.push(row);
        }
    }
    (kept, dropped)
}

/// One endpoint's starting vector, by registered spec.
///
/// `v2-padded` is R9's ratified anchor — "incumbents at V2, candidates 0", the
/// same start §5.4's witness search takes. `zero` is everything at zero but the
/// pin, which is the vector a fit-free draft would begin from.
fn start_vector(spec: &str, stage: StageEndpoint) -> Result<Vec<i64>, String> {
    let mut w = vec![0i64; AXIS_WIDTH];
    w[Feature::VictoryPoints.index()] = toys::pinned_vp(stage);
    match spec {
        "zero" => Ok(w),
        "v2-padded" => {
            let table = v2_vector(stage);
            for &f in &Feature::ALL[..Feature::INCUMBENT_COUNT] {
                w[f.index()] = table[f.index()];
            }
            Ok(w)
        }
        other => Err(format!(
            "`{other}` is not a registered starting vector; `repair --start` takes one of \
             {REPAIR_START_SPECS:?}",
        )),
    }
}

/// One endpoint's repair problem: the PROJECTED system, the draft's starting
/// vector, and the coordinates the projection is allowed to move.
#[derive(Clone)]
struct RepairProblem {
    stage: StageEndpoint,
    /// The projected system with R3's constraints already folded into the box:
    /// removed features fixed at 0, `w_vp` fixed at the pin, and each candidate's
    /// sign constraint as a one-sided bound. **They are box tightenings and not
    /// rows because that is exactly what they are** — every one of them is a
    /// bound on a single coordinate, and the oracle's propagation reads a box
    /// bound directly.
    sys: System,
    /// The rows §4's projection dropped, by name.
    dropped: Vec<String>,
    /// How the starting vector was named.
    start: String,
    w0: Vec<i64>,
    /// The coordinates the projection may move: not fixed by the box, and
    /// carried by at least one row.
    ///
    /// **A coordinate no row reads is pinned at the draft, and that is a
    /// reduction rather than a policy.** Moving it cannot change any row value,
    /// so it can only add displacement and add a changed coordinate — both
    /// objectives strictly worsen, so no optimum moves it.
    free: Vec<usize>,
    /// The projected system as it stood BEFORE R3's sign clauses were folded
    /// into its box.
    ///
    /// B1's certified-exception class removes ONE clause at a time, and a
    /// removal has to be a re-derivation rather than an edit: `lo_at`/`hi_at`
    /// take a max and a min, so a bound that has been tightened cannot be
    /// untightened by inspection. Keeping the pre-clause system means
    /// [`RepairProblem::excepting`] rebuilds the whole box from one source with
    /// exactly one clause left out.
    raw: System,
    /// The coordinates whose sign clause is OFF at this problem under B1's
    /// class. EMPTY for every problem [`RepairProblem::new`] builds — the raw
    /// un-excepted projection is what that constructor means, and the standing
    /// S4-late regression pins it.
    exceptions: Vec<CandidateFeature>,
}

/// What one question put to the oracle asked, and what came back.
#[derive(Clone)]
struct TrailStep {
    question: String,
    outcome: TrailOutcome,
    /// The labelled rows a Farkas combination leaned on, or the enumerated
    /// support's coordinates — so the trail can be read without re-deriving it.
    leaned_on: Vec<String>,
}

/// How a level was closed.
#[derive(Clone)]
enum TrailOutcome {
    /// Certified empty by the oracle, with the certificate it shipped.
    Empty(Certificate),
    /// Certified nonempty by a point the rows re-checked.
    Witnessed(Vec<i64>),
    /// Certified empty by EXHAUSTING the supports at this changed count. The
    /// enumeration is the certificate; the count is how wide it was.
    Enumerated { supports: u64 },
    /// True by arithmetic, with no search: `Σ|w_f − w0_f| ≥ 0` over the
    /// integers, so the level below zero is empty and nothing had to look.
    Arithmetic(&'static str),
    /// The oracle did not decide. NEVER infeasibility.
    Undecided(String),
}

/// **THE REPAIR'S THREE ANSWERS.** There is no fourth, and `Inconclusive` is not
/// a weak `Infeasible`.
enum Repair {
    Repaired(Box<RepairReport>),
    /// **`REPAIR-INFEASIBLE`** — no assignment in the box satisfies the rows,
    /// the pin, the orderings and the sign constraints at once. Routed to the
    /// owner with the oracle's certificate; never a silent relaxation.
    Infeasible { certificate: Certificate, trail: Vec<TrailStep> },
    /// The oracle, or the projection's own budget, did not decide. Routed.
    Inconclusive { reason: String, trail: Vec<TrailStep> },
}

impl Repair {
    fn name(&self) -> &'static str {
        match self {
            Repair::Repaired(_) => "REPAIRED",
            Repair::Infeasible { .. } => "REPAIR-INFEASIBLE",
            Repair::Inconclusive { .. } => "INCONCLUSIVE",
        }
    }
}

/// The repaired vector and everything R3 requires recorded beside it.
struct RepairReport {
    stage: StageEndpoint,
    start: String,
    w0: Vec<i64>,
    w: Vec<i64>,
    /// `Σ_f |w_f − w0_f|` over the free coordinates. The pin is fixed, not
    /// displaced, so it never appears in this sum.
    displacement: i64,
    /// The changed coordinates, in ascending `Feature::ALL` order.
    changed: Vec<usize>,
    /// `System::violation` of the returned vector against the projected system —
    /// zero, re-checked outside the oracle.
    violation: i128,
    /// The projected rows the pricing record reads.
    projected: Vec<Row>,
    dropped: Vec<String>,
    trail: Vec<TrailStep>,
    oracle_calls: u64,
    supports_examined: u64,
}

impl RepairReport {
    /// The trail step that answered exactly this question.
    fn step(&self, question: &str) -> Option<&TrailStep> {
        self.trail.iter().find(|s| s.question == question)
    }

    /// **THE ROWS THAT PRICED ONE COORDINATE** — R3's last sentence.
    ///
    /// A row PRICED coordinate `f` if it carries `f` with a nonzero `Δv` and
    /// would be VIOLATED with `f` alone returned to the draft's value. That is
    /// the relation "priced it" names: the row that stops this coordinate going
    /// back.
    fn pricing(&self, f: usize) -> Vec<String> {
        self.rows_reading(f, |slack, back| slack - back < 0)
    }

    /// The rows TIGHT at the optimum that carry `f` — slack exactly zero.
    ///
    /// Recorded BESIDE [`RepairReport::pricing`] because the two differ and the
    /// difference is instructive rather than a defect: a row whose margin and
    /// coefficients share no common factor can be the whole reason a coordinate
    /// moved and still never sit at slack 0 (11(b)'s margin is 1 against
    /// coefficients ±2, and it is exactly that row).
    fn tight(&self, f: usize) -> Vec<String> {
        self.rows_reading(f, |slack, _| slack == 0)
    }

    fn rows_reading(&self, f: usize, keep: impl Fn(i128, i128) -> bool) -> Vec<String> {
        self.projected
            .iter()
            .filter(|row| row.coef.get(f).copied().unwrap_or(0) != 0)
            .filter(|row| {
                let slack = System::slack_of(row, &self.w);
                let back = i128::from(row.coef[f]) * i128::from(self.w[f] - self.w0[f]);
                keep(slack, back)
            })
            .map(|row| row.label.clone())
            .collect()
    }
}

/// **THE AUGMENTED SYSTEM** — the projected system plus the machinery that makes
/// "within `level` of the draft" a question the frozen oracle can be asked.
///
/// One extra column per movable coordinate holds that coordinate's displacement.
/// Two rows pin it from below (`t ≥ w − w0` and `t ≥ w0 − w`, together
/// `t ≥ |w − w0|`), and one budget row caps the sum (`−Σ t ≥ −level`). Nothing
/// pins `t` from above except its box, and nothing needs to: the budget row is
/// an upper bound and `t` only ever wants to be small, so `Σ|w − w0| ≤ level`
/// holds exactly when the augmented system is satisfiable.
///
/// **Every coordinate not in `movable` is pinned at the draft's value.** For the
/// coordinates no row reads that is the reduction [`RepairProblem::free`]
/// documents; for the coordinates a SUPPORT question excludes it is the question
/// itself.
fn augmented(
    p: &RepairProblem,
    level: i64,
    movable: &[usize],
    pins: &[(usize, i64)],
) -> System {
    let width = AXIS_WIDTH + movable.len();
    let mut lo = vec![0i64; width];
    let mut hi = vec![0i64; width];
    lo[..AXIS_WIDTH].copy_from_slice(&p.sys.lo);
    hi[..AXIS_WIDTH].copy_from_slice(&p.sys.hi);
    for f in 0..AXIS_WIDTH {
        if !movable.contains(&f) {
            lo[f] = p.w0[f];
            hi[f] = p.w0[f];
        }
    }
    for (f, value) in pins {
        lo[*f] = *value;
        hi[*f] = *value;
    }

    let mut rows: Vec<Row> = p
        .sys
        .rows
        .iter()
        .map(|r| {
            let mut coef = r.coef.clone();
            coef.resize(width, 0);
            Row { coef, ..r.clone() }
        })
        .collect();

    for (j, &f) in movable.iter().enumerate() {
        let aux = AXIS_WIDTH + j;
        // The widest displacement this coordinate can reach inside its own box,
        // capped by the level: `t <= Σt <= level` because every `t` is >= 0.
        let span = (p.w0[f] - lo[f]).max(hi[f] - p.w0[f]).clamp(0, level.max(0));
        lo[aux] = 0;
        hi[aux] = span;
        let label = FeatureId::all()[f].label();
        for (sign, rhs) in [(-1i64, -p.w0[f]), (1, p.w0[f])] {
            let mut coef = vec![0i64; width];
            coef[aux] = 1;
            coef[f] = sign;
            rows.push(Row {
                label: format!("|d| {label} >= {} (w - w0)", if sign < 0 { "+" } else { "-" }),
                kind: RowKind::Repair,
                coef,
                rhs,
                margin_vp: None,
            });
        }
    }

    if !movable.is_empty() {
        let mut coef = vec![0i64; width];
        for j in 0..movable.len() {
            coef[AXIS_WIDTH + j] = -1;
        }
        rows.push(Row {
            label: format!("displacement budget <= {level}"),
            kind: RowKind::Repair,
            coef,
            rhs: -level,
            margin_vp: None,
        });
    }

    System { stage: p.stage, rows, lo, hi, pin_vp: p.sys.pin_vp, box_limit: p.sys.box_limit }
}

/// The RATIONAL relaxation of one augmented system, decided exactly — a HINT and
/// nothing else.
///
/// It chooses where the certified bisection starts and carries no verdict: the
/// level it points at is probed through [`decide`] like every other, and the
/// level below it is probed too, so a wrong hint costs probes and cannot cost
/// correctness. `None` is "the LP did not decide", which is also only a hint.
///
/// Reading it is sound in one direction and that is the direction used: an
/// integer point is a rational point, so a level whose relaxation is empty has
/// no integer point either, and the least relaxation-feasible level is a lower
/// bound on `D*`.
fn relaxation_feasible(sys: &System) -> Option<bool> {
    let all = sys.inequalities();
    let columns = active_columns(&sys.rows);
    let row_index: Vec<usize> = all
        .iter()
        .enumerate()
        .filter(|(_, r)| columns.iter().any(|&j| r.coef[j] != 0))
        .map(|(i, _)| i)
        .collect();
    match relaxation(&all, &row_index, &columns) {
        Relaxation::Feasible => Some(true),
        Relaxation::Infeasible(_) => Some(false),
        Relaxation::Undecided(_) => None,
    }
}

impl RepairProblem {
    /// Build one endpoint's problem, REFUSING a start the registered system does
    /// not describe.
    ///
    /// Three typed refusals, on R4's malformed-identity discipline — "a typed
    /// instrument failure ... INVALIDATES the read and does not count against
    /// the table or the cap". A draft whose `w_vp` is not the pin, whose
    /// coordinates leave the box, or which prices a removed feature is not a
    /// worse draft; it is a draft the system has no variable for, and repairing
    /// it would be repairing something else.
    fn new(
        stage: StageEndpoint,
        sys: System,
        dropped: Vec<String>,
        start: &str,
        w0: Vec<i64>,
    ) -> Result<RepairProblem, String> {
        RepairProblem::with_exceptions(stage, sys, dropped, start, w0, &[])
    }

    /// The same, with `exceptions`' sign clauses LEFT OFF — B1's class, and the
    /// only way this file builds a problem that is not the raw projection.
    ///
    /// The refusals are [`RepairProblem::new`]'s, unchanged, plus one of its
    /// own: a REMOVED feature has no sign clause to except (its box is
    /// `lo = hi = 0`, which is §4's projection fixing it, not a direction), so
    /// naming one is a malformed exception rather than a wider system.
    fn with_exceptions(
        stage: StageEndpoint,
        sys: System,
        dropped: Vec<String>,
        start: &str,
        w0: Vec<i64>,
        exceptions: &[CandidateFeature],
    ) -> Result<RepairProblem, String> {
        if w0.len() != AXIS_WIDTH {
            return Err(format!("the axis is {AXIS_WIDTH} wide and the start is {}", w0.len()));
        }
        let vp = Feature::VictoryPoints.index();
        if w0[vp] != sys.pin_vp {
            return Err(format!(
                "the start prices w_vp at {} and the {} pin is {}. R1 applies the pin as a FIXED \
                 coordinate, not as a repair, so a start that breaks it is a malformed identity \
                 rather than a system to repair.",
                w0[vp],
                stage.name(),
                sys.pin_vp,
            ));
        }
        for (i, id) in FeatureId::all().into_iter().enumerate() {
            if w0[i].abs() > sys.box_limit {
                return Err(format!(
                    "the start prices {} at {} and the box is |w| <= {}. R1: a repair demanding \
                     |w| > 64 is a routed SCALE finding, never a silent clamp or widening.",
                    id.label(),
                    w0[i],
                    sys.box_limit,
                ));
            }
        }
        for candidate in REPAIR_REMOVED {
            let slot = FeatureId::Candidate(candidate).index();
            if w0[slot] != 0 {
                return Err(format!(
                    "the start prices {} at {}, and the projected system fixes the removed \
                     features at 0 — it has no variable there. A malformed identity, not a worse \
                     table.",
                    candidate.label(),
                    w0[slot],
                ));
            }
        }

        for candidate in exceptions {
            if REPAIR_REMOVED.contains(candidate) {
                return Err(format!(
                    "{} is a REMOVED feature, fixed at 0 by §4's projection. That is not a sign \
                     clause and B1's certified-exception class has nothing to relax there.",
                    candidate.label(),
                ));
            }
        }

        let raw = sys.clone();
        let mut sys = sys;
        // **THE SIGN CONSTRAINT, as one bound per coordinate.** R3: no coordinate
        // moves FROM a prior-agreeing sign to an opposed one, and no opposed
        // coordinate moves further from agreement. Both clauses, and the
        // controller's 0-start clause, are the single relation
        // `w_f·p_f >= min(0, w0_f·p_f)`: an agreeing or zero start floors the
        // coordinate at agreement-or-zero, and an opposed start floors it at
        // where it already is.
        //
        // The priors are the library's `SIGN_PRIORS`, the ONE registered spelling
        // — §8's wrong-sign removal trigger and §9(c)'s prior-agreeing carrier
        // clause read the same table. The sixteen INCUMBENTS carry no registered
        // prior and are deliberately left unconstrained beyond the system and the
        // box: inventing one here would be registering a direction nobody ruled.
        //
        // **An EXCEPTED coordinate's clause is not applied at all** (B1) — it
        // keeps the registered integer box and nothing else. The prior itself is
        // untouched: `SIGN_PRIORS` still reads `+` for that coordinate, and the
        // certificate that licensed the exception stands beside it.
        for candidate in CandidateFeature::ALL {
            if exceptions.contains(&candidate) {
                continue;
            }
            let f = FeatureId::Candidate(candidate).index();
            if sign_prior(candidate) > 0 {
                lo_at(&mut sys.lo, f, w0[f].min(0));
            } else {
                hi_at(&mut sys.hi, f, w0[f].max(0));
            }
        }
        for candidate in REPAIR_REMOVED {
            let f = FeatureId::Candidate(candidate).index();
            sys.lo[f] = 0;
            sys.hi[f] = 0;
        }

        let free: Vec<usize> = (0..AXIS_WIDTH)
            .filter(|&f| sys.lo[f] < sys.hi[f])
            .filter(|&f| sys.rows.iter().any(|r| r.coef[f] != 0))
            .collect();
        Ok(RepairProblem {
            stage,
            sys,
            dropped,
            start: start.to_string(),
            w0,
            free,
            raw,
            exceptions: exceptions.to_vec(),
        })
    }

    /// The same problem with ONE more sign clause left off, re-derived from
    /// [`RepairProblem::raw`].
    fn excepting(&self, extra: CandidateFeature) -> Result<RepairProblem, String> {
        let mut exceptions = self.exceptions.clone();
        if !exceptions.contains(&extra) {
            exceptions.push(extra);
        }
        RepairProblem::with_exceptions(
            self.stage,
            self.raw.clone(),
            self.dropped.clone(),
            &self.start,
            self.w0.clone(),
            &exceptions,
        )
    }

    /// Whether `candidate`'s sign clause is currently APPLIED **and** actually
    /// tightens the registered box.
    ///
    /// A clause that coincides with the box constrains nothing, so removing it
    /// cannot change a verdict — probing it would spend an oracle call to learn
    /// that the system is the system. That is a reduction, not a policy: the
    /// relaxed system it would ask about is literally the same system.
    fn clause_active(&self, candidate: CandidateFeature) -> bool {
        if self.exceptions.contains(&candidate) || REPAIR_REMOVED.contains(&candidate) {
            return false;
        }
        let f = FeatureId::Candidate(candidate).index();
        if sign_prior(candidate) > 0 {
            self.w0[f].min(0) > self.raw.lo[f]
        } else {
            self.w0[f].max(0) < self.raw.hi[f]
        }
    }

    /// One coordinate's sign clause, spelled as [`System::inequalities`] labels
    /// its box rows — so a clause named here and a row named in a Farkas
    /// combination are the same string.
    fn clause_text(&self, candidate: CandidateFeature) -> String {
        let f = FeatureId::Candidate(candidate).index();
        let label = FeatureId::all()[f].label();
        if sign_prior(candidate) > 0 {
            format!("{label} >= {}", self.w0[f].min(0))
        } else {
            format!("{label} <= {}", self.w0[f].max(0))
        }
    }

    fn displacement(&self, w: &[i64]) -> i64 {
        self.free.iter().map(|&f| (w[f] - self.w0[f]).abs()).sum()
    }

    fn changed(&self, w: &[i64]) -> Vec<usize> {
        self.free.iter().copied().filter(|&f| w[f] != self.w0[f]).collect()
    }
}

fn lo_at(lo: &mut [i64], f: usize, bound: i64) {
    lo[f] = lo[f].max(bound);
}

fn hi_at(hi: &mut [i64], f: usize, bound: i64) {
    hi[f] = hi[f].min(bound);
}

/// The projection in flight: the budgets it is spending and the trail it is
/// building.
struct Projection<'a> {
    p: &'a RepairProblem,
    calls: u64,
    supports: u64,
    oracle_budget: u64,
    support_budget: u64,
    trail: Vec<TrailStep>,
}

impl<'a> Projection<'a> {
    fn new(p: &'a RepairProblem, oracle_budget: u64, support_budget: u64) -> Projection<'a> {
        Projection { p, calls: 0, supports: 0, oracle_budget, support_budget, trail: Vec::new() }
    }

    fn routed(&self, reason: String) -> Repair {
        Repair::Inconclusive { reason, trail: self.trail.clone() }
    }

    /// Put one question to the frozen oracle, spending one call.
    fn ask(&mut self, sys: &System) -> Result<Verdict, Repair> {
        self.calls += 1;
        if self.calls > self.oracle_budget {
            return Err(self.routed(format!(
                "the projection spent its budget of {} oracle calls (an engine-local cap, not a \
                 §5.4-registered figure). The oracle did not decide, which is not infeasibility.",
                self.oracle_budget,
            )));
        }
        // **B2's QUESTION CHAIN.** Every question the projection puts to the
        // oracle goes through the extension: the frozen `decide` answers first
        // and unchanged, and only its enumeration-budget INCONCLUSIVE is asked
        // again with cuts.
        Ok(decide_with_cuts(sys))
    }

    /// The same, RECORDED in the certificate trail.
    fn ask_recorded(&mut self, question: String, sys: &System) -> Result<Verdict, Repair> {
        let verdict = self.ask(sys)?;
        let all = sys.inequalities();
        let (outcome, leaned_on) = match &verdict {
            Verdict::Sat { w } => (TrailOutcome::Witnessed(w[..AXIS_WIDTH].to_vec()), Vec::new()),
            Verdict::Unsat { certificate } => {
                (TrailOutcome::Empty(certificate.clone()), certificate_leans(&all, certificate))
            }
            Verdict::Inconclusive { reason } => {
                (TrailOutcome::Undecided(reason.clone()), Vec::new())
            }
        };
        self.trail.push(TrailStep { question, outcome, leaned_on });
        Ok(verdict)
    }

    fn record(&mut self, question: String, outcome: TrailOutcome, leaned_on: Vec<String>) {
        self.trail.push(TrailStep { question, outcome, leaned_on });
    }
}

/// **WHAT ONE CERTIFICATE LEANED ON**, by row label or by column.
///
/// The MULTIPLIER rides with the label. A routed REPAIR-INFEASIBLE is read by a
/// person deciding what to do about it, and the combination's shape — which
/// rows, how heavily — is the whole of what there is to read.
///
/// **ONLY THE FARKAS ARM NAMES ROWS, AND THE OTHER TWO SAY SO.**
///
/// `all` is the inequality list the certificate was issued against. A Farkas
/// combination is written over exactly that list, so its rows can be named and
/// a caller may read CLAUSE CARRIAGE off the result. The other two shapes
/// cannot be read that way and must not look as though they can:
///
/// * an ENUMERATION names the columns it swept, not rows — a list of coordinate
///   labels is not a list of rows, and a clause-matching caller finds nothing in
///   it;
/// * a CHVATAL-GOMORY certificate's inner verdict was taken over the
///   STRENGTHENED list, so its multipliers cannot be zipped against this one.
///   The derived cuts are nameable and are still named — each label carries its
///   parents and their multipliers — but they are NOT the whole combination, and
///   a page that printed "41 nonzero multipliers" beside four cut labels would
///   read as though they were.
///
/// So each non-Farkas arm leads with a line saying what shape it is and what was
/// not looked at. **Never an empty list**: an empty `leans on` on a decision
/// page reads as "nothing is standing", which is the reading of a singly-forced
/// coordinate and the opposite of what a refused probe means.
fn certificate_leans(all: &[Row], certificate: &Certificate) -> Vec<String> {
    match certificate {
        Certificate::Farkas { y, .. } => all
            .iter()
            .zip(y)
            .filter(|(_, m)| **m != 0)
            .map(|(r, m)| format!("{} x{m}  [{}]", r.label, sparse_row(r)))
            .collect(),
        Certificate::Enumeration { support, .. } => {
            let columns: Vec<String> = support
                .iter()
                .map(|&c| match FeatureId::all().get(c) {
                    Some(id) => id.label().to_string(),
                    None => format!("|w-w0| #{}", c - AXIS_WIDTH),
                })
                .collect();
            vec![format!(
                "residual certificate is {}; it names COLUMNS and not rows, so clause carriage \
                 not introspected. Swept: {columns:?}",
                CertificateShape::Enumeration.name(),
            )]
        }
        Certificate::ChvatalGomory { cuts, inner } => {
            let mut out = vec![format!(
                "residual certificate is {}; the derived cuts are named below, but the BASE rows \
                 the inner {} leans on are NOT enumerated here (it was taken over the \
                 STRENGTHENED system), so clause carriage not introspected",
                CertificateShape::ChvatalGomory.name(),
                CertificateShape::of(inner).name(),
            )];
            out.extend(
                cuts.iter().map(|cut| format!("{}  [{}]", cut.label, sparse_row(&cut.row()))),
            );
            out
        }
    }
}

/// **THE ENGINE.** R3's projection at its two shipped budgets:
/// [`REPAIR_ORACLE_BUDGET`], an engine-local cap, and [`REPAIR_SUPPORT_BUDGET`],
/// §5.4's registered 10⁷.
fn repair(p: &RepairProblem) -> Repair {
    repair_within(p, REPAIR_ORACLE_BUDGET, REPAIR_SUPPORT_BUDGET)
}

/// The same, at budgets a caller names.
///
/// The budgets are parameters only so their EXHAUSTION ARMS can be exercised —
/// [`integer_search`]'s reasoning verbatim: a test that had to spend the whole
/// budget to prove the arm exists would not be a test. [`repair`] always passes
/// the shipped pair.
fn repair_within(p: &RepairProblem, oracle_budget: u64, support_budget: u64) -> Repair {
    let mut projection = Projection::new(p, oracle_budget, support_budget);
    match projection.run() {
        Ok(report) => Repair::Repaired(Box::new(report)),
        Err(routed) => routed,
    }
}

impl Projection<'_> {
    fn run(&mut self) -> Result<RepairReport, Repair> {
        let p = self.p;
        let free = p.free.clone();

        // (0) Is the repair problem feasible AT ALL? The plain projected system,
        // sign constraints and pin already in its box. An UNSAT here is
        // REPAIR-INFEASIBLE and nothing further is asked.
        let plain = p.sys.clone();
        let w_hi = match self.ask_recorded(
            "the projected system, sign-constrained".to_string(),
            &plain,
        )? {
            Verdict::Sat { w } => w,
            Verdict::Unsat { certificate } => {
                let trail = std::mem::take(&mut self.trail);
                return Err(Repair::Infeasible { certificate, trail });
            }
            Verdict::Inconclusive { reason } => return Err(self.routed(reason)),
        };
        // **`d_hi` IS A FEASIBLE LEVEL**, which the bisection's upper bracket
        // needs and which is not quite free: `w_hi` came from the plain system,
        // where the coordinates no row reads were not pinned at the draft.
        // Resetting them to the draft changes no row value (no row reads them),
        // leaves the point in its box (the draft is), and leaves this sum alone
        // (it runs over the free coordinates only) — so a point at exactly this
        // level exists.
        let d_hi = p.displacement(&w_hi);

        // (1) THE DISPLACEMENT LEVEL. `lo` is a level certified EMPTY, `hi` a
        // level with a witness; bisect until they are adjacent.
        let hint = self.lp_hint(d_hi)?;
        let mut lo = -1i64;
        let mut hi = d_hi;
        let mut witness = w_hi;
        self.record(
            "displacement <= -1".to_string(),
            TrailOutcome::Arithmetic(
                "|w - w0| >= 0 over the integers, so no level below zero holds a point",
            ),
            Vec::new(),
        );
        while hi - lo > 1 {
            let probe = next_probe(lo, hi, hint);
            let sys = augmented(p, probe, &free, &[]);
            match self.ask_recorded(format!("displacement <= {probe}"), &sys)? {
                Verdict::Sat { w } => {
                    witness = w[..AXIS_WIDTH].to_vec();
                    hi = probe;
                }
                Verdict::Unsat { .. } => lo = probe,
                Verdict::Inconclusive { reason } => {
                    // The BRACKET rides with the routed reason. "The oracle did
                    // not decide" is a fact about one level; what a reader needs
                    // is where the optimum was known to lie when the projection
                    // stopped, which is the pair the bisection was holding.
                    return Err(self.routed(format!(
                        "at displacement level {probe}, with the optimum bracketed in \
                         ({lo}, {hi}]: {reason}",
                    )));
                }
            }
        }
        let d_star = hi;

        // (2) THE CHANGED COUNT, at `d_star`. Ascending `k`, ascending listed
        // order within `k`; the first SAT support is both the count and the
        // canonical support.
        let violated = self.violated_carriers();
        let ceiling = p.changed(&witness).len();
        let mut support = Vec::new();
        let mut found = false;
        for k in 0..=ceiling {
            let before = self.supports;
            let mut chosen = Vec::new();
            match self.search_support(d_star, k, 0, &mut chosen, &violated, &free)? {
                Some((s, w)) => {
                    self.record(
                        format!("changed count <= {k} at displacement {d_star}"),
                        TrailOutcome::Witnessed(w[..AXIS_WIDTH].to_vec()),
                        s.iter().map(|&f| FeatureId::all()[f].label().to_string()).collect(),
                    );
                    witness = w[..AXIS_WIDTH].to_vec();
                    support = s;
                    found = true;
                    break;
                }
                None => self.record(
                    format!("changed count <= {k} at displacement {d_star}"),
                    TrailOutcome::Enumerated { supports: self.supports - before },
                    Vec::new(),
                ),
            }
        }
        if !found {
            // Unreachable through the arithmetic: `witness` itself changes
            // `ceiling` coordinates and hits every violated row, so the `k =
            // ceiling` pass has at least one satisfiable support. Typed rather
            // than asserted, because a wrong answer is worse than a routed one.
            return Err(self.routed(format!(
                "no support of size <= {ceiling} was satisfiable at displacement {d_star}, yet a \
                 witness at that displacement changes exactly {ceiling} coordinates. The \
                 projection did not decide, which is not infeasibility.",
            )));
        }

        // (3) THE CANONICAL REPRESENTATIVE within that support.
        let mut pins: Vec<(usize, i64)> = Vec::new();
        for &f in &support {
            let label = FeatureId::all()[f].label();
            let mut settled = None;
            for value in value_keys(p.w0[f], p.sys.lo[f], p.sys.hi[f], d_star) {
                let mut trial = pins.clone();
                trial.push((f, value));
                let sys = augmented(p, d_star, &support, &trial);
                match self.ask(&sys)? {
                    Verdict::Sat { w } => {
                        witness = w[..AXIS_WIDTH].to_vec();
                        settled = Some(value);
                        break;
                    }
                    Verdict::Unsat { .. } => continue,
                    Verdict::Inconclusive { reason } => return Err(self.routed(reason)),
                }
            }
            let Some(value) = settled else {
                return Err(self.routed(format!(
                    "no value in {label}'s box left the system feasible at displacement \
                     {d_star}; the projection did not decide, which is not infeasibility.",
                )));
            };
            pins.push((f, value));
            self.record(
                format!("{label} = {value} (|d| = {})", (value - p.w0[f]).abs()),
                TrailOutcome::Witnessed(witness.clone()),
                Vec::new(),
            );
        }

        let w = witness;
        // The returned point is re-checked OUTSIDE the oracle, on the same
        // discipline the oracle applies to its own: against every row, and
        // against the box the sign constraints tightened. A point that fails
        // either is not a repair, and saying so is cheaper than believing it.
        let violation = p.sys.violation(&w);
        if violation != 0 {
            return Err(self.routed(format!(
                "the projection's own re-check leaves violation {violation} on the returned \
                 point. Nothing is claimed.",
            )));
        }
        if let Some((f, value)) =
            (0..AXIS_WIDTH).map(|f| (f, w[f])).find(|&(f, x)| x < p.sys.lo[f] || x > p.sys.hi[f])
        {
            return Err(self.routed(format!(
                "the returned point prices {} at {value}, outside [{}, {}] -- the box the \
                 registered limit and the sign constraint left it. Nothing is claimed.",
                FeatureId::all()[f].label(),
                p.sys.lo[f],
                p.sys.hi[f],
            )));
        }
        Ok(RepairReport {
            stage: p.stage,
            start: p.start.clone(),
            w0: p.w0.clone(),
            displacement: p.displacement(&w),
            changed: p.changed(&w),
            violation,
            projected: p.sys.rows.clone(),
            dropped: p.dropped.clone(),
            trail: std::mem::take(&mut self.trail),
            oracle_calls: self.calls,
            supports_examined: self.supports,
            w,
        })
    }

    /// The least relaxation-feasible level, by bisection — a HINT (see
    /// [`relaxation_feasible`]).
    fn lp_hint(&mut self, d_hi: i64) -> Result<i64, Repair> {
        let free = self.p.free.clone();
        let mut lo = -1i64;
        let mut hi = d_hi;
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            self.calls += 1;
            if self.calls > self.oracle_budget {
                return Err(self.routed(format!(
                    "the projection spent its budget of {} oracle calls (an engine-local cap, not \
                     a §5.4-registered figure) while placing the first probe. The oracle did not \
                     decide, which is not infeasibility.",
                    self.oracle_budget,
                )));
            }
            match relaxation_feasible(&augmented(self.p, mid, &free, &[])) {
                Some(true) => hi = mid,
                Some(false) => lo = mid,
                None => return Ok(0),
            }
        }
        Ok(hi)
    }

    /// Each row VIOLATED at the draft, as the free coordinates it carries.
    ///
    /// **The hitting-set reduction, and why it is sound.** A row violated at the
    /// draft and none of whose carriers moved reads exactly the value it read at
    /// the draft, so it is still violated. Every feasible point's changed set
    /// therefore HITS every one of these sets, and a support that does not hit
    /// them all cannot be asked — it is not a shortcut past a question, it is a
    /// question with a proven answer.
    fn violated_carriers(&self) -> Vec<Vec<usize>> {
        self.p
            .sys
            .rows
            .iter()
            .filter(|row| System::slack_of(row, &self.p.w0) < 0)
            .map(|row| self.p.free.iter().copied().filter(|&f| row.coef[f] != 0).collect())
            .collect()
    }

    /// Every support of size `k` that hits every violated row, in ascending
    /// listed order, asked of the oracle until one answers SAT.
    #[allow(clippy::too_many_arguments)]
    fn search_support(
        &mut self,
        level: i64,
        k: usize,
        start: usize,
        chosen: &mut Vec<usize>,
        violated: &[Vec<usize>],
        free: &[usize],
    ) -> Result<Option<SupportHit>, Repair> {
        self.supports += 1;
        if self.supports > self.support_budget {
            return Err(self.routed(format!(
                "the support enumeration spent its registered budget of {} subsets. An \
                 enumeration that did not finish has certified nothing, so this is INCONCLUSIVE \
                 and not infeasibility.",
                self.support_budget,
            )));
        }
        // Prune: a violated row no remaining candidate can hit closes the whole
        // subtree, since every superset of this prefix inside it misses that row.
        let reachable = |row: &Vec<usize>| {
            row.iter().any(|c| chosen.contains(c) || free[start..].contains(c))
        };
        if !violated.iter().all(reachable) {
            return Ok(None);
        }
        if chosen.len() == k {
            if !violated.iter().all(|row| row.iter().any(|c| chosen.contains(c))) {
                return Ok(None);
            }
            let sys = augmented(self.p, level, chosen, &[]);
            return match self.ask(&sys)? {
                Verdict::Sat { w } => Ok(Some((chosen.clone(), w))),
                Verdict::Unsat { .. } => Ok(None),
                Verdict::Inconclusive { reason } => Err(self.routed(reason)),
            };
        }
        for i in start..free.len() {
            if free.len() - i < k - chosen.len() {
                break;
            }
            chosen.push(free[i]);
            let found = self.search_support(level, k, i + 1, chosen, violated, free)?;
            chosen.pop();
            if found.is_some() {
                return Ok(found);
            }
        }
        Ok(None)
    }
}

/// A support that answered SAT, with the point that answered it.
type SupportHit = (Vec<usize>, Vec<i64>);

/// Where to probe next: the hint, then the level below it, then the midpoint.
///
/// The hint is the least relaxation-feasible level, so it is `D*` whenever the
/// relaxation is tight there — and then the level below it closes the bracket in
/// one more probe, on a system whose relaxation is empty and whose certificate is
/// therefore a cheap Farkas combination. Off the hint the bisection is ordinary.
fn next_probe(lo: i64, hi: i64, hint: i64) -> i64 {
    for candidate in [hint, hint - 1] {
        if candidate > lo && candidate < hi {
            return candidate;
        }
    }
    lo + (hi - lo) / 2
}

/// One coordinate's candidate values in the registered canonical key order:
/// least `|w_f − w0_f|` first, and at equal distance the smaller value first.
fn value_keys(w0: i64, lo: i64, hi: i64, level: i64) -> Vec<i64> {
    let reach = (w0 - lo).max(hi - w0).min(level.max(0));
    let mut out = Vec::new();
    for d in 0..=reach {
        for value in [w0 - d, w0 + d] {
            if value >= lo && value <= hi && !out.contains(&value) {
                out.push(value);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// B1 — the CERTIFIED-EXCEPTION CLASS, a driver ABOVE the projection
// ---------------------------------------------------------------------------

/// **THE AUTHORITY EVERY EXCEPTION CITES**, printed wherever one is applied.
///
/// The class was ruled by the owner in the 2026-08-25 guidingpath decision-log
/// row "THE T4-ENTRY BATCH IS RULED", sentence B1, over the T3 wave's S4-late
/// REPAIR-INFEASIBLE finding. It is a registration, not an engineering choice,
/// and an artifact that applied one without naming the sentence that licensed it
/// would be a silent relaxation wearing a certificate.
const EXCEPTION_AUTHORITY: &str = "B1 (guidingpath decision log, 2026-08-25, \"THE T4-ENTRY \
     BATCH IS RULED\"): when the R3 projection returns typed REPAIR-INFEASIBLE with an \
     oracle-held certificate, a coordinate PROVEN forced strictly against its registered sign \
     prior may have its sign clause -- and ONLY it -- relaxed to the integer box at that \
     endpoint alone.";

/// **THE TWO-QUESTION FORCING TEST**, which the engine RE-PROVES before it
/// applies anything.
const EXCEPTION_FORCING_TEST: &str = "FORCING TEST (B1, re-proved here, never inherited): (i) \
     the full system stands oracle-certified infeasible; (ii) the system with EXACTLY that one \
     sign clause removed is oracle-certified FEASIBLE. Together, every solution of the relaxed \
     system VIOLATES the removed clause -- the relaxation is FORCED, not chosen. An \
     observed-negative witness never qualifies, and `SIGN_PRIORS` stays on the books unedited: \
     the certificate stands BESIDE the prior, never in place of it.";

/// **WHAT A JOINT CONFLICT IS AND IS NOT.**
const JOINT_CONFLICT_READING: &str = "REPAIR-JOINT-CONFLICT (B1): the system is oracle-certified \
     infeasible and NO single active sign clause's removal restores feasibility, so no \
     coordinate is SINGLY forced and the certified-exception class does not apply. This is typed \
     DISTINCTLY and routed to the OWNER -- it is outside the class, never an exception, and \
     never a silent relaxation. Chain-relaxing two clauses at once would be choosing, not \
     proving.";

/// **A CERTIFICATE'S SHAPE**, kept beside a probe so a renderer can say what it
/// is looking at without holding the certificate itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CertificateShape {
    Farkas,
    Enumeration,
    ChvatalGomory,
}

impl CertificateShape {
    fn of(certificate: &Certificate) -> CertificateShape {
        match certificate {
            Certificate::Farkas { .. } => CertificateShape::Farkas,
            Certificate::Enumeration { .. } => CertificateShape::Enumeration,
            Certificate::ChvatalGomory { .. } => CertificateShape::ChvatalGomory,
        }
    }

    fn name(self) -> &'static str {
        match self {
            CertificateShape::Farkas => "FARKAS",
            CertificateShape::Enumeration => "EXHAUSTIVE ENUMERATION",
            CertificateShape::ChvatalGomory => "CHVATAL-GOMORY",
        }
    }

    /// **ONLY A FARKAS CERTIFICATE NAMES ROWS**, so it is the only shape a
    /// caller may read CLAUSE CARRIAGE off.
    ///
    /// An enumeration names columns and a Chvátal–Gomory certificate names
    /// derived cuts over a system that is not the one the caller is holding.
    /// Reading either as "these clauses carry the residual" would answer the
    /// question with the wrong list — and, worse, with an EMPTY one, which on a
    /// decision page reads as "nothing is standing" and is the reading of a
    /// singly-forced coordinate.
    fn carriage_readable(self) -> bool {
        self == CertificateShape::Farkas
    }
}

/// The oracle's answer to ONE probe, typed so the fold below can be total.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeAnswer {
    /// Question (ii) came back SAT: the coordinate is proven singly forced.
    Feasible,
    StillInfeasible,
    Undecided,
}

impl ProbeAnswer {
    /// The one-word spelling a page prints.
    fn name(self) -> &'static str {
        match self {
            ProbeAnswer::Feasible => "FEASIBLE",
            ProbeAnswer::StillInfeasible => "still infeasible",
            ProbeAnswer::Undecided => "undecided",
        }
    }
}

/// One coordinate's answer to question (ii), recorded whether or not it fired.
///
/// The probes that came back "still infeasible" are the record that the
/// coordinate finally excepted was the FIRST in canonical order — a claim about
/// an order is only checkable if the order is printed.
#[derive(Clone)]
struct ExceptionProbe {
    candidate: CandidateFeature,
    clause: String,
    /// The oracle's answer to "is the system without this clause feasible?".
    answer: ProbeAnswer,
    detail: String,
    /// A REFUSED probe's residual certificate, by row.
    ///
    /// **This is the half a JOINT conflict is read by.** "No single clause's
    /// removal helps" is a verdict; WHICH rows still contradict once a given
    /// clause is off is the thing an owner needs to rule on, and it is a
    /// different row set per probe — the residual certificate names the next
    /// coordinate as surely as the standing one named this coordinate.
    leaned_on: Vec<String>,
    /// A REFUSED probe's residual certificate SHAPE, which decides whether
    /// [`ExceptionProbe::leaned_on`] can be read as clause carriage at all.
    /// `None` for a FEASIBLE or undecided probe, which has no residual.
    residual: Option<CertificateShape>,
}

/// **ONE CERTIFIED EXCEPTION**, with everything B1 requires recorded beside it.
#[derive(Clone)]
struct CertifiedException {
    candidate: CandidateFeature,
    stage: StageEndpoint,
    /// The clause that was removed, in [`System::inequalities`]' own spelling.
    clause: String,
    /// Question (i): the standing infeasibility certificate, VERIFIED by the
    /// oracle that issued it.
    infeasibility: Certificate,
    /// That certificate's rows, with per-row multipliers and sparse contents.
    leaned_on: Vec<String>,
    /// Question (ii): the point that certified the relaxed system feasible,
    /// re-checked against every row before the oracle returned it.
    witness: Vec<i64>,
    /// Every coordinate examined at this iteration, in canonical order, up to
    /// and including the one that fired.
    probes: Vec<ExceptionProbe>,
}

/// **THE DRIVER'S THREE ANSWERS.** `REPAIR-INFEASIBLE` is not among them: an
/// infeasibility either names a singly-forced coordinate (and is excepted) or
/// does not (and is a JOINT conflict). The raw projection keeps its own verdict
/// and its own regression.
enum Driven {
    Repaired { exceptions: Vec<CertifiedException>, report: Box<RepairReport> },
    /// **`REPAIR-JOINT-CONFLICT`** — see [`JOINT_CONFLICT_READING`].
    JointConflict {
        exceptions: Vec<CertifiedException>,
        certificate: Certificate,
        leaned_on: Vec<String>,
        probes: Vec<ExceptionProbe>,
        trail: Vec<TrailStep>,
    },
    /// The oracle, or a budget, did not decide. Routed — and a JOINT conflict is
    /// deliberately NOT claimed on an undecided probe, because "no coordinate is
    /// singly forced" is a statement about every coordinate.
    Inconclusive { exceptions: Vec<CertifiedException>, reason: String, trail: Vec<TrailStep> },
}

impl Driven {
    fn name(&self) -> &'static str {
        match self {
            Driven::Repaired { .. } => "REPAIRED",
            Driven::JointConflict { .. } => "REPAIR-JOINT-CONFLICT",
            Driven::Inconclusive { .. } => "INCONCLUSIVE",
        }
    }

    /// The exceptions applied on the way to this answer, in the order they were
    /// applied.
    fn exceptions(&self) -> &[CertifiedException] {
        match self {
            Driven::Repaired { exceptions, .. }
            | Driven::JointConflict { exceptions, .. }
            | Driven::Inconclusive { exceptions, .. } => exceptions,
        }
    }
}

/// What one iteration's search for a singly-forced coordinate found.
enum Forcing {
    /// The FIRST coordinate in canonical order whose relaxed system is feasible.
    Forced { candidate: CandidateFeature, witness: Vec<i64>, probes: Vec<ExceptionProbe> },
    /// Every active clause was probed and every relaxed system came back
    /// certified infeasible.
    Joint { probes: Vec<ExceptionProbe> },
    /// At least one probe did not decide, so `Joint` is not available: the
    /// coordinate that could not be decided might have been the forced one.
    Undecided { probes: Vec<ExceptionProbe>, reason: String },
}

/// **THE DRIVER.** B1's class at the shipped budgets.
fn drive_repair(p: &RepairProblem) -> Driven {
    drive_repair_within(p, REPAIR_ORACLE_BUDGET, REPAIR_SUPPORT_BUDGET)
}

/// The same, at budgets a caller names — [`repair_within`]'s reasoning verbatim.
///
/// # The loop, and why it terminates
///
/// Each iteration either answers (REPAIRED / JOINT / INCONCLUSIVE) or applies
/// exactly one exception to a coordinate that had none. There are
/// [`CandidateFeature::COUNT`] coordinates, so at most that many iterations
/// apply an exception, and when every clause is off `clause_active` is false
/// everywhere, [`forced_coordinate`] returns `Joint` with no probes, and the
/// loop ends.
fn drive_repair_within(p: &RepairProblem, oracle_budget: u64, support_budget: u64) -> Driven {
    let mut current = p.clone();
    let mut exceptions: Vec<CertifiedException> = Vec::new();
    loop {
        let trail = match repair_within(&current, oracle_budget, support_budget) {
            Repair::Repaired(report) => return Driven::Repaired { exceptions, report },
            Repair::Inconclusive { reason, trail } => {
                return Driven::Inconclusive { exceptions, reason, trail }
            }
            Repair::Infeasible { trail, .. } => trail,
        };

        // **QUESTION (i), ASKED HERE.** B1 says the engine RE-PROVES the forcing
        // before applying any exception, so the certificate an exception records
        // is one this driver holds — not one it inherited from a verdict it is
        // about to act on. The two agree by construction today (the projection
        // raises REPAIR-INFEASIBLE only from this same system), and asking makes
        // that agreement a checked fact rather than a coupling.
        let (certificate, leaned_on) = match decide_with_cuts(&current.sys) {
            Verdict::Unsat { certificate } => {
                let leans = certificate_leans(&current.sys.inequalities(), &certificate);
                (certificate, leans)
            }
            other => {
                return Driven::Inconclusive {
                    exceptions,
                    reason: format!(
                        "the projection returned REPAIR-INFEASIBLE, but B1's question (i) re-read \
                         the same system as {}. The certified-exception class needs a STANDING \
                         infeasibility certificate and this driver does not hold one, so nothing \
                         is excepted and nothing is claimed.",
                        other.name(),
                    ),
                    trail,
                }
            }
        };

        match forced_coordinate(&current) {
            Forcing::Forced { candidate, witness, probes } => {
                let clause = current.clause_text(candidate);
                let next = match current.excepting(candidate) {
                    Ok(next) => next,
                    Err(e) => {
                        return Driven::Inconclusive {
                            exceptions,
                            reason: format!(
                                "{} was proven singly forced, but the excepted problem would not \
                                 build: {e}",
                                candidate.label(),
                            ),
                            trail,
                        }
                    }
                };
                exceptions.push(CertifiedException {
                    candidate,
                    stage: current.stage,
                    clause,
                    infeasibility: certificate,
                    leaned_on,
                    witness,
                    probes,
                });
                current = next;
            }
            Forcing::Joint { probes } => {
                return Driven::JointConflict {
                    exceptions,
                    certificate,
                    leaned_on,
                    probes,
                    trail,
                }
            }
            Forcing::Undecided { probes, reason } => {
                return Driven::Inconclusive {
                    exceptions,
                    reason: format!(
                        "a coordinate's forcing probe did not decide, so \"no coordinate is \
                         singly forced\" cannot be claimed and B1's JOINT-conflict verdict is \
                         withheld: {reason} ({} probe(s) recorded)",
                        probes.len(),
                    ),
                    trail,
                }
            }
        }
    }
}

/// **QUESTION (ii), over every still-active sign clause in canonical order.**
///
/// The order is `CandidateFeature::ALL`, which IS ascending `Feature::ALL` axis
/// order over the candidate columns (the axis is the sixteen-feature incumbent
/// prefix, then the eighteen candidates in that order) — pinned by
/// [`the_canonical_examination_order_is_the_axis_order`] rather than assumed.
///
/// The FIRST feasible relaxation wins and the search stops: B1 relaxes ONE
/// clause per iteration, and a second feasible coordinate at the same iteration
/// would be a choice between two proofs rather than a proof.
fn forced_coordinate(p: &RepairProblem) -> Forcing {
    let mut probes: Vec<ExceptionProbe> = Vec::new();
    let mut witness: Option<Vec<i64>> = None;
    for candidate in CandidateFeature::ALL {
        if !p.clause_active(candidate) {
            continue;
        }
        let clause = p.clause_text(candidate);
        let relaxed = match p.excepting(candidate) {
            Ok(relaxed) => relaxed,
            Err(e) => {
                probes.push(ExceptionProbe {
                    candidate,
                    clause,
                    answer: ProbeAnswer::Undecided,
                    detail: format!("the relaxed problem would not build: {e}"),
                    leaned_on: Vec::new(),
                    residual: None,
                });
                continue;
            }
        };
        match decide_with_cuts(&relaxed.sys) {
            Verdict::Sat { w } => {
                let detail = format!(
                    "{} = {} at the witness; every solution of the relaxed system violates the \
                     removed clause, so the relaxation is FORCED",
                    candidate.label(),
                    w[FeatureId::Candidate(candidate).index()],
                );
                probes.push(ExceptionProbe {
                    candidate,
                    clause,
                    answer: ProbeAnswer::Feasible,
                    detail,
                    leaned_on: Vec::new(),
                    residual: None,
                });
                witness = Some(w);
                break;
            }
            Verdict::Unsat { certificate } => {
                let leans = certificate_leans(&relaxed.sys.inequalities(), &certificate);
                probes.push(ExceptionProbe {
                    candidate,
                    clause,
                    answer: ProbeAnswer::StillInfeasible,
                    detail: render_certificate(&certificate),
                    leaned_on: leans,
                    residual: Some(CertificateShape::of(&certificate)),
                });
            }
            Verdict::Inconclusive { reason } => {
                probes.push(ExceptionProbe {
                    candidate,
                    clause,
                    answer: ProbeAnswer::Undecided,
                    detail: reason.clone(),
                    leaned_on: Vec::new(),
                    residual: None,
                });
            }
        }
    }
    fold_forcing(probes, witness)
}

/// **THE FOLD from a canonical-order probe list to a verdict**, kept PURE.
///
/// Every route out of [`forced_coordinate`] runs through here, and none of them
/// needs an oracle that can be made to say INCONCLUSIVE to reach — which is what
/// makes the withholding arms testable at all. `witness` is `Some` exactly when
/// the last probe answered FEASIBLE; anything else is a construction defect and
/// is typed rather than trusted.
fn fold_forcing(probes: Vec<ExceptionProbe>, witness: Option<Vec<i64>>) -> Forcing {
    let undecided: Vec<String> = probes
        .iter()
        .filter(|probe| probe.answer == ProbeAnswer::Undecided)
        .map(|probe| format!("{}: {}", probe.candidate.label(), probe.detail))
        .collect();
    let feasible = probes
        .last()
        .filter(|probe| probe.answer == ProbeAnswer::Feasible)
        .map(|probe| probe.candidate);
    match (witness, feasible) {
        (Some(witness), Some(candidate)) if undecided.is_empty() => {
            Forcing::Forced { candidate, witness, probes }
        }
        // **SYMMETRIC STRICTNESS.** The FEASIBLE probe is the last one, so every
        // undecided probe is EARLIER in canonical order — and B1 takes the
        // FIRST coordinate whose relaxation is feasible, which is a claim about
        // the whole prefix. An undecided coordinate might itself have been
        // feasible, and the exception would then land on the wrong one. Being
        // strict about withholding JOINT and lenient about claiming FORCED
        // would guard the verdict nobody acts on and wave through the one that
        // changes the table.
        (Some(_), Some(candidate)) => Forcing::Undecided {
            reason: format!(
                "{}'s relaxation is FEASIBLE, but {} earlier coordinate(s) in canonical order \
                 could not be decided, so B1's \"the FIRST coordinate whose relaxed system is \
                 feasible\" is NOT established -- an undecided coordinate might itself have been \
                 feasible. Nothing is excepted: {}",
                candidate.label(),
                undecided.len(),
                undecided.join(" · "),
            ),
            probes,
        },
        (None, None) if undecided.is_empty() => Forcing::Joint { probes },
        (None, None) => {
            let reason = undecided.join(" · ");
            Forcing::Undecided { probes, reason }
        }
        (witness, feasible) => Forcing::Undecided {
            reason: format!(
                "the probe list and the witness disagree: a witness is {} and the last probe {} \
                 FEASIBLE. That is a construction defect in the probe loop, not an answer about \
                 any system.",
                if witness.is_some() { "present" } else { "absent" },
                if feasible.is_some() { "IS" } else { "is NOT" },
            ),
            probes,
        },
    }
}

/// One vector in the hand-off grammar: comma-separated `i64`, no spaces.
///
/// The same spelling `eval_probe`'s `fit --evaluate` and `step9 --table` vectors
/// use, because a second spelling is a transcription step.
fn vector_csv(w: &[i64]) -> String {
    w.iter().map(i64::to_string).collect::<Vec<_>>().join(",")
}

/// **THE REPAIRED TABLE, IN THE GRAMMAR THE NEXT COMMAND PARSES.**
///
/// A repaired vector that has to be reassembled out of the `changed
/// coordinates` block is a vector that gets reassembled WRONG: that block prints
/// the MOVES, not the table, and the coordinates it does not name are exactly
/// the ones a reader would have to remember to copy from the draft. These two
/// lines are the whole table in exactly the form
/// `eval_probe step9 structural --table <early>/<late>` takes — thirty-four
/// comma-separated `i64` per vector, in `FeatureId::all()` order — so the T4
/// hand-off is a copy rather than a reconstruction.
///
/// **A REPAIR IS PER ENDPOINT, and the other endpoint prints UNMOVED.** This run
/// solved one stage; the other half is the run's own `--start` vector spelled at
/// that endpoint, because nothing here touched it. The header says which is
/// which, so the pair is never read as two repairs. A start this file cannot
/// spell at the other endpoint — a hand-built problem, never the shipped
/// subcommands, whose `--start` is registered before it reaches here — prints
/// the repaired half alone and says why.
fn render_paste_ready(report: &RepairReport) -> String {
    let other = match report.stage {
        StageEndpoint::Early => StageEndpoint::Late,
        StageEndpoint::Late => StageEndpoint::Early,
    };
    let repaired = vector_csv(&report.w);
    let Ok(unmoved) = start_vector(&report.start, other) else {
        return format!(
            "\npaste-ready table    `{}` is not a registered --start spec, so the {} half has no \
             spelling here;\n                     the repaired {} vector alone:\n  {:<18} \
             {repaired}\n",
            report.start,
            other.name(),
            report.stage.name(),
            report.stage.name().to_uppercase(),
        );
    };
    let unmoved = vector_csv(&unmoved);
    let (early, late) = match report.stage {
        StageEndpoint::Early => (repaired, unmoved),
        StageEndpoint::Late => (unmoved, repaired),
    };
    format!(
        "\npaste-ready table    `<early>/<late>` for `eval_probe step9 structural --table`: the \
         {} vector is\n                     this endpoint's repair, the {} one the unmoved `{}` \
         start (a repair\n                     is per endpoint, and this run solved \
         one)\n  EARLY              {early}\n  LATE               {late}\n",
        report.stage.name().to_uppercase(),
        other.name().to_uppercase(),
        report.start,
    )
}

/// **THE PROJECTION'S ARTIFACT** — one endpoint, printed whole.
///
/// It runs the repair; a caller that wants the structured answer calls
/// [`repair`] instead. Byte-identical on repeat by construction: nothing in the
/// chain reads a clock or draws entropy, so the same problem prints the same
/// page.
fn render_repair(p: &RepairProblem) -> String {
    let mut out = render_repair_header(p);
    out.push_str(&render_repair_outcome(&repair(p), 0));
    out
}

/// The block naming the problem, before anything is asked of the oracle.
///
/// Split out of [`render_repair`] so the driven artifact and the raw one open
/// the same way: a reader comparing the two is comparing what happened, not two
/// spellings of what was solved.
fn render_repair_header(p: &RepairProblem) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "\nR3 projection        {} endpoint, starting vector `{}`\n",
        p.stage.name(),
        p.start,
    ));
    out.push_str(&format!("ruling               {REPAIR_RULING}\n"));
    out.push_str(&format!("canonical rule       {REPAIR_CANONICAL}\n"));
    out.push_str(&format!(
        "removed (fixed at 0) {:?}\n",
        REPAIR_REMOVED.iter().map(|c| c.label()).collect::<Vec<_>>(),
    ));
    out.push_str(&format!("kept rows            {}\n", p.sys.rows.len()));
    out.push_str(&format!("dropped rows         {}  {:?}\n", p.dropped.len(), p.dropped));
    out.push_str(&format!("free coordinates     {}\n", p.free.len()));
    out
}

/// One projection outcome, printed whole.
///
/// `exceptions` is how many B1 exceptions were standing when the projection ran
/// — `0` on the raw path, where the REPAIRED line is byte-identical to what it
/// always was.
fn render_repair_outcome(verdict: &Repair, exceptions: usize) -> String {
    let mut out = String::new();
    let headline = verdict.name();
    match verdict {
        Repair::Repaired(report) => {
            out.push_str(&format!(
                "\nVERDICT              {headline} at the {} endpoint from `{}`: displacement {} \
                 over {} changed coordinate(s){}\n",
                report.stage.name(),
                report.start,
                report.displacement,
                report.changed.len(),
                match exceptions {
                    0 => String::new(),
                    n => format!(", under {n} certified exception(s) (B1)"),
                },
            ));
            out.push_str(&format!(
                "dropped, re-stated   {}  {:?}\n",
                report.dropped.len(),
                report.dropped,
            ));
            let last_witness = report.trail.iter().rev().find_map(|step| match &step.outcome {
                TrailOutcome::Witnessed(w) => Some(w.clone()),
                _ => None,
            });
            out.push_str(&format!(
                "witness identity     the trail's last witness {} the returned vector\n",
                if last_witness.as_ref() == Some(&report.w) { "IS" } else { "IS NOT" },
            ));
            // The two levels R3's certification MINIMALLY owes, looked up by
            // name rather than by position: a trail read by eye is a trail
            // nobody checks.
            let closed = |question: &str| match report.step(question).map(|s| &s.outcome) {
                Some(TrailOutcome::Empty(certificate)) => render_certificate(certificate),
                Some(TrailOutcome::Enumerated { supports }) => {
                    format!("exhaustive support enumeration over {supports} subsets")
                }
                Some(TrailOutcome::Arithmetic(why)) => format!("by arithmetic: {why}"),
                Some(TrailOutcome::Witnessed(_)) => "NOT CLOSED -- a witness".to_string(),
                Some(TrailOutcome::Undecided(why)) => format!("NOT CLOSED -- {why}"),
                None => "NOT ASKED".to_string(),
            };
            let below = format!("displacement <= {}", report.displacement - 1);
            out.push_str(&format!("required level 1     {below}: {}\n", closed(&below)));
            match report.changed.len().checked_sub(1) {
                Some(fewer) => {
                    let q = format!(
                        "changed count <= {fewer} at displacement {}",
                        report.displacement,
                    );
                    out.push_str(&format!("required level 2     {q}: {}\n", closed(&q)));
                }
                None => out.push_str(
                    "required level 2     no coordinate changed, so no better level exists\n",
                ),
            }
            out.push_str(&format!(
                "re-check             System::violation of the returned vector = {}\n",
                report.violation,
            ));
            out.push_str(&render_paste_ready(report));
            out.push_str("\nchanged coordinates  (w0 -> w, and the rows that priced the move)\n");
            for &f in &report.changed {
                let label = FeatureId::all()[f].label();
                out.push_str(&format!(
                    "  {label:<28} {:>4} -> {:>4}   |d| {}\n",
                    report.w0[f],
                    report.w[f],
                    (report.w[f] - report.w0[f]).abs(),
                ));
                out.push_str(&format!("      priced by  {:?}\n", report.pricing(f)));
                out.push_str(&format!("      tight at   {:?}\n", report.tight(f)));
            }
            if report.changed.is_empty() {
                out.push_str("  (none -- the starting vector already satisfies the system)\n");
            }
            out.push_str(&render_trail(&report.trail));
            out.push_str(&format!(
                "\nbudget spent         {} oracle calls of {} (an engine-local cap), {} supports \
                 enumerated of {} (§5.4's registered 10^7)\n",
                report.oracle_calls,
                REPAIR_ORACLE_BUDGET,
                report.supports_examined,
                REPAIR_SUPPORT_BUDGET,
            ));
            out.push_str(&format!("coverage             {REPAIR_MONOTONICITY}\n"));
        }
        Repair::Infeasible { certificate, trail } => {
            out.push_str(
                "\nVERDICT              REPAIR-INFEASIBLE -- no assignment inside the box \
                 satisfies the rows, the pin, the hand orderings and the sign constraints at \
                 once. R3 routes this to the OWNER; it is never a silent relaxation.\n",
            );
            out.push_str(&format!("certificate          {}\n", render_certificate(certificate)));
            out.push_str(&render_trail(trail));
        }
        Repair::Inconclusive { reason, trail } => {
            out.push_str(&format!(
                "\nVERDICT              INCONCLUSIVE -- {reason}\nROUTED               \
                 §5.4: \"the oracle did not decide\" is never infeasibility, and this is not a \
                 licence to fall back on a heuristic answer.\n",
            ));
            out.push_str(&render_trail(trail));
        }
    }
    out
}

/// **THE DRIVEN ARTIFACT** — the same projection under B1's certified-exception
/// class, printed whole.
///
/// The header is [`render_repair`]'s, then the driver's own block, then the
/// outcome. A run that applied NO exception prints the same outcome bytes the
/// raw run does, with the exception block saying so in as many words: "nothing
/// was excepted" is a fact a reader should be able to see, not infer from an
/// absence.
///
/// Byte-identical on repeat by construction, exactly as [`render_repair`] is:
/// nothing in the chain reads a clock or draws entropy.
fn render_driven(p: &RepairProblem) -> String {
    let mut out = render_repair_header(p);
    let driven = drive_repair(p);
    // ONE spelling of the verdict word, read off the answer itself — the
    // REPAIRED arm gets it from `Repair::name` through the shared outcome
    // renderer, and the other two from here.
    let headline = driven.name();
    out.push_str(&render_exception_block(driven.exceptions()));
    match driven {
        Driven::Repaired { exceptions, report } => {
            let doses = render_exception_doses(&exceptions, &report);
            out.push_str(&render_repair_outcome(&Repair::Repaired(report), exceptions.len()));
            out.push_str(&doses);
        }
        Driven::JointConflict { certificate, leaned_on, probes, trail, .. } => {
            out.push_str(&format!(
                "\nVERDICT              {headline} -- {JOINT_CONFLICT_READING}\n",
            ));
            out.push_str(&format!("certificate          {}\n", render_certificate(&certificate)));
            for label in &leaned_on {
                out.push_str(&format!("      leans on   {label}\n"));
            }
            out.push_str(&render_joint_structure(p, &probes));
            out.push_str(&render_probes("clauses probed", &probes));
            out.push_str(&render_trail(&trail));
        }
        Driven::Inconclusive { reason, trail, .. } => {
            out.push_str(&format!(
                "\nVERDICT              {headline} -- {reason}\nROUTED               \
                 §5.4: \"the oracle did not decide\" is never infeasibility, and this is not a \
                 licence to fall back on a heuristic answer.\n",
            ));
            out.push_str(&render_trail(&trail));
        }
    }
    out
}

/// The driver's own block: the authority, the forcing test, and every exception
/// it applied with BOTH certificates.
fn render_exception_block(exceptions: &[CertifiedException]) -> String {
    let mut out = String::new();
    out.push_str(&format!("\nexception driver     {EXCEPTION_AUTHORITY}\n"));
    out.push_str(&format!("forcing test         {EXCEPTION_FORCING_TEST}\n"));
    out.push_str(&format!("exceptions applied   {}\n", exceptions.len()));
    if exceptions.is_empty() {
        out.push_str(
            "                     (none -- the projection answered without one, and the raw \
             un-excepted\n                     system is exactly what was solved)\n",
        );
        return out;
    }
    for (k, e) in exceptions.iter().enumerate() {
        out.push_str(&format!(
            "\n  certified exception {} of {}: {} ({}) at the {} endpoint\n",
            k + 1,
            exceptions.len(),
            e.candidate.label(),
            candidate_long_name(e.candidate),
            e.stage.name(),
        ));
        out.push_str(&format!("      authority    {EXCEPTION_AUTHORITY}\n"));
        out.push_str(&format!(
            "      clause off   `{}`  (w_f*p_f >= min(0, w0_f*p_f); registered prior {})\n",
            e.clause,
            if sign_prior(e.candidate) > 0 { "+" } else { "-" },
        ));
        out.push_str(&format!(
            "      prior        UNEDITED -- `SIGN_PRIORS` still reads {} for {}; the certificate \
             stands beside it\n",
            sign_prior(e.candidate),
            e.candidate.label(),
        ));
        out.push_str(&format!(
            "      (i) infeasible  {}\n",
            render_certificate(&e.infeasibility),
        ));
        for label in &e.leaned_on {
            out.push_str(&format!("          leans on {label}\n"));
        }
        out.push_str(&format!(
            "      (ii) feasible   witness re-checked against every row; {} = {} there\n",
            e.candidate.label(),
            e.witness[FeatureId::Candidate(e.candidate).index()],
        ));
        out.push_str(&render_probes("      clauses probed", &e.probes));
    }
    out
}

/// **THE SHAPE OF A JOINT CONFLICT, in one block.**
///
/// For each clause whose removal did not help, the OTHER sign clauses its
/// residual certificate still leans on. That is the joint structure said
/// plainly: "remove this one and the contradiction moves to those" is what an
/// owner is being asked to rule on, and reading it out of fourteen multi-row
/// certificates by eye is exactly the kind of derivation an artifact should do
/// once rather than ask its reader to do.
///
/// A clause is recognised by STRING EQUALITY against
/// [`RepairProblem::clause_text`], which is the same spelling
/// [`System::inequalities`] labels the box row with — one spelling, so this
/// cannot drift into naming rows that are not clauses.
fn render_joint_structure(p: &RepairProblem, probes: &[ExceptionProbe]) -> String {
    let clauses: Vec<(CandidateFeature, String)> = CandidateFeature::ALL
        .into_iter()
        .filter(|c| p.clause_active(*c))
        .map(|c| (c, p.clause_text(c)))
        .collect();
    let mut out = String::from(
        "\njoint structure      (remove ONE clause and these sign clauses still carry the \
         residual certificate)\n",
    );
    for probe in probes.iter().filter(|probe| probe.answer == ProbeAnswer::StillInfeasible) {
        // **A SHAPE THIS CANNOT READ SAYS SO, RATHER THAN ANSWERING [].**
        // Only a FARKAS residual names rows; for any other shape the clause
        // list would be empty not because nothing carries the residual but
        // because nothing was looked at, and those two read identically on the
        // page while meaning opposite things.
        let readable = probe.residual.is_some_and(CertificateShape::carriage_readable);
        if !readable {
            out.push_str(&format!(
                "  without `{}`          residual certificate is {}; clause carriage not \
                 introspected\n",
                probe.clause,
                probe.residual.map_or("of an unrecorded shape", CertificateShape::name),
            ));
            continue;
        }
        let still: Vec<&str> = clauses
            .iter()
            .filter(|(c, text)| {
                *c != probe.candidate
                    && probe.leaned_on.iter().any(|label| label.starts_with(&format!("{text} x")))
            })
            .map(|(c, _)| c.label())
            .collect();
        out.push_str(&format!(
            "  without `{}`          still leans on {:?}\n",
            probe.clause, still,
        ));
    }
    out
}

/// Every clause examined at one iteration, in canonical order, with its answer
/// and — for a refusal — the rows its RESIDUAL certificate leaned on.
fn render_probes(heading: &str, probes: &[ExceptionProbe]) -> String {
    let mut out = format!("{heading} (canonical `Feature::ALL` order, first FEASIBLE wins)\n");
    for probe in probes {
        out.push_str(&format!(
            "          {:<4} `{}`  {}  -- {}\n",
            probe.candidate.label(),
            probe.clause,
            probe.answer.name(),
            probe.detail,
        ));
        for label in &probe.leaned_on {
            out.push_str(&format!("               residual leans on   {label}\n"));
        }
    }
    out
}

/// **THE DOSE, AND THE ROWS THAT PRICED IT** — B1's "in the final answer" half.
///
/// The exception says a coordinate MAY leave its prior; the dose says how far it
/// actually went, and the binding rows say what stopped it going back. Read off
/// the same [`RepairReport::pricing`] the changed-coordinate block uses, so an
/// excepted coordinate is priced by the same relation every other one is.
fn render_exception_doses(exceptions: &[CertifiedException], report: &RepairReport) -> String {
    if exceptions.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "\nexception doses      (the least certified anti-prior magnitude, and the rows that \
         priced it)\n",
    );
    for e in exceptions {
        let f = FeatureId::Candidate(e.candidate).index();
        out.push_str(&format!(
            "  {:<28} {:>4} -> {:>4}   |d| {}\n",
            e.candidate.label(),
            report.w0[f],
            report.w[f],
            (report.w[f] - report.w0[f]).abs(),
        ));
        out.push_str(&format!("      priced by  {:?}\n", report.pricing(f)));
        out.push_str(&format!("      tight at   {:?}\n", report.tight(f)));
    }
    out
}

/// A candidate's registered long name, for the exception block's header line.
fn candidate_long_name(candidate: CandidateFeature) -> String {
    format!("{candidate:?}")
}

/// One row's nonzero coefficients, by coordinate label.
///
/// A routed REPAIR-INFEASIBLE is read by a person deciding what to do about it,
/// and "which rows" is only half of that: the other half is what those rows SAY,
/// which is a handful of signed integers and belongs beside the multiplier.
fn sparse_row(row: &Row) -> String {
    let axis = FeatureId::all();
    let terms: Vec<String> = row
        .coef
        .iter()
        .enumerate()
        .filter(|(_, c)| **c != 0)
        .map(|(i, c)| {
            let name = match axis.get(i) {
                Some(id) => id.label().to_string(),
                None => format!("|d|#{}", i - AXIS_WIDTH),
            };
            format!("{name}:{c:+}")
        })
        .collect();
    format!("{} >= {}", terms.join(" "), row.rhs)
}

/// One certificate, named rather than dumped.
fn render_certificate(certificate: &Certificate) -> String {
    match certificate {
        Certificate::Farkas { y, rhs } => format!(
            "FARKAS, re-verified in checked i128 integers: {} nonzero multipliers, yb = {rhs}",
            y.iter().filter(|m| **m != 0).count(),
        ),
        Certificate::Enumeration { support, points } => {
            format!("EXHAUSTIVE ENUMERATION over {} columns, {points} points", support.len())
        }
        Certificate::ChvatalGomory { cuts, inner } => format!(
            "CHVATAL-GOMORY, {} verified derived cut(s) joining the question chain, then {}",
            cuts.len(),
            render_certificate(inner),
        ),
    }
}

/// The certificate trail: every question the projection asked, in order.
fn render_trail(trail: &[TrailStep]) -> String {
    let mut out = String::from("\ncertificate trail    (every lexicographically better level, closed)\n");
    for step in trail {
        let answer = match &step.outcome {
            TrailOutcome::Empty(certificate) => {
                format!("EMPTY   {}", render_certificate(certificate))
            }
            TrailOutcome::Witnessed(_) => "WITNESS point re-checked against every row".to_string(),
            TrailOutcome::Enumerated { supports } => format!(
                "EMPTY   exhaustive support enumeration, {supports} subsets, none satisfiable",
            ),
            TrailOutcome::Arithmetic(why) => format!("EMPTY   by arithmetic: {why}"),
            TrailOutcome::Undecided(why) => format!("UNDECIDED  {why}"),
        };
        out.push_str(&format!("  {:<44} {answer}\n", step.question));
        if !step.leaned_on.is_empty() {
            out.push_str(&format!("      leans on   {:?}\n", step.leaned_on));
        }
    }
    out
}

/// Render one problem the way the caller asked for it: through B1's driver (the
/// default) or as the RAW un-excepted projection.
fn render_problem(problem: &RepairProblem, driver: bool) -> String {
    if driver {
        render_driven(problem)
    } else {
        render_repair(problem)
    }
}

/// Run R3's projection over one registered toy.
fn run_repair_toy(name: &str, start: &str, driver: bool) -> Result<(), String> {
    let sys = toys::named(name)
        .ok_or_else(|| format!("`{name}` is not a registered toy; see `solver toys`"))?;
    println!("=== solver repair --toy {name} ===");
    print!("{}", identity_block(None));
    let stage = sys.stage;
    let (kept, dropped) = project_pair_rows(sys.rows.clone());
    let projected = System::new(stage, kept, sys.box_limit, sys.pin_vp);
    let w0 = start_vector(start, stage)?;
    let problem = RepairProblem::new(stage, projected, dropped, start, w0)?;
    print!("{}", render_problem(&problem, driver));
    Ok(())
}

/// **THE REAL PROJECTION**, argument-gated exactly as the real solve is.
fn run_repair_real(start: &str, freeze_commit: Option<&str>, driver: bool) -> Result<(), String> {
    let rules = Rules::standard();
    println!("=== solver repair --real  [§7 step 9, R3] ===");
    print!("{}", identity_block(freeze_commit));
    print!("{}", census_block());
    for stage in StageEndpoint::ALL {
        println!("\n{}", "=".repeat(72));
        let mut rows = Vec::new();
        for pair in REGISTERED
            .iter()
            .filter(|p| matches!(p.role, PairRole::Constraint) && p.stages.contains(&stage))
        {
            rows.push(extract_row(pair, stage, &rules)?);
        }
        let (kept, dropped) = project_pair_rows(rows);
        let projected = assemble(stage, kept, BOX_LIMIT);
        let w0 = start_vector(start, stage)?;
        let problem = RepairProblem::new(stage, projected, dropped, start, w0)?;
        print!("{}", render_problem(&problem, driver));
    }
    Ok(())
}

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
    // N-3: the ruling explains ONE shape of gap and only that shape. Printing
    // it for any mismatch would let a registry that had drifted somewhere else
    // borrow the ruling's authority, which is exactly §5.4's "mismatch = registry
    // defect" clause being talked out of firing.
    match (realized.early, realized.late) {
        (FROZEN_EARLY, FROZEN_LATE) => {
            out.push_str("                     realized == frozen\n");
        }
        (24, 27) => out.push_str(&format!("RULING               {CENSUS_RULING}\n")),
        (early, late) => out.push_str(&format!(
            "REGISTRY DEFECT      realized EARLY {early} / LATE {late} matches neither the frozen \
             text ({FROZEN_EARLY}/{FROZEN_LATE}) nor the 4(c) ruling's 24/27. §5.4: a mismatch is \
             a registry defect, not a result. NOTHING here may be read as a Part-B answer.\n",
        )),
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

/// One weight vector, printed as its entries away from the V2-padded start.
///
/// The delta rather than all thirty-four, because the start is itself a
/// published number: what a reader needs is which coordinates the system MOVED.
fn render_moved(sys: &System, w: &[i64]) -> String {
    let start = sys.v2_padded();
    let mut out = String::new();
    let mut moved = 0usize;
    for (i, id) in FeatureId::all().into_iter().enumerate() {
        if w[i] != start[i] {
            out.push_str(&format!("  {:<28}  {:>5}  (from {})\n", id.label(), w[i], start[i]));
            moved += 1;
        }
    }
    if moved == 0 {
        out.push_str("  none -- the V2-padded start already satisfies the system\n");
    }
    out
}

/// One verdict and its certificate, in the words §5.4 requires.
fn render_verdict(sys: &System, verdict: &Verdict) -> String {
    let mut out = String::new();
    match verdict {
        Verdict::Sat { w } => {
            out.push_str(
                "                     a point satisfying every inequality, re-checked\n",
            );
            out.push_str(&render_moved(sys, w));
        }
        Verdict::Unsat { certificate } => {
            let rows = sys.inequalities();
            match certificate {
                Certificate::Farkas { y, rhs } => {
                    out.push_str(&format!(
                        "certificate          FARKAS, re-verified in checked i128 integers \
                         (yb = {rhs} > 0, yA = 0)\n",
                    ));
                    for (row, multiplier) in rows.iter().zip(y) {
                        if *multiplier != 0 {
                            out.push_str(&format!("  x{multiplier:<4}  {}\n", row.label));
                        }
                    }
                }
                Certificate::Enumeration { support, points } => {
                    out.push_str(&format!(
                        "certificate          EXHAUSTIVE ENUMERATION over {} column(s), {points} \
                         points, all refuted\n",
                        support.len(),
                    ));
                    for &c in support {
                        out.push_str(&format!(
                            "  {:<28}  [{}, {}]\n",
                            FeatureId::all()[c].label(),
                            sys.lo[c],
                            sys.hi[c],
                        ));
                    }
                }
                // §5.4's own solve path never builds one — the extension is
                // [`decide_with_cuts`]', and `solve` asks [`decide`]. Rendered
                // rather than refused because a renderer that panicked on a
                // reachable-by-refactor arm is a worse instrument than one that
                // prints the certificate it was handed.
                Certificate::ChvatalGomory { cuts, inner } => {
                    out.push_str(&format!(
                        "certificate          {}\n",
                        render_certificate(&Certificate::ChvatalGomory {
                            cuts: cuts.clone(),
                            inner: inner.clone(),
                        }),
                    ));
                    for cut in cuts {
                        out.push_str(&format!("  {}\n", cut.label));
                    }
                }
            }
        }
        Verdict::Inconclusive { reason } => {
            out.push_str(&format!("                     {reason}\n"));
            out.push_str(
                "                     INCONCLUSIVE is routed to the owner. It is not \
                 infeasibility.\n",
            );
        }
    }
    out
}

/// Everything §5.4 owes about ONE stage's system, once it has been built.
///
/// Shared by the toy runner and the real run, so the artifact a review reads
/// after the freeze is rendered by the same code the toys proved.
fn render_system(sys: &System) -> String {
    let mut out = String::new();
    out.push_str(&format!("stage                {}\n", sys.stage.name()));
    out.push_str(&format!(
        "box                  |w| <= {}   w_vp pinned to {}\n",
        sys.box_limit, sys.pin_vp,
    ));
    let pairs = sys.rows.iter().filter(|r| matches!(r.kind, RowKind::Pair { .. })).count();
    let orderings = sys.rows.iter().filter(|r| r.kind == RowKind::HandOrdering).count();
    out.push_str(&format!(
        "rows                 {pairs} pair + {orderings} hand-ordering deletable, {} \
         inequalities in all (deletable + pin + box)\n",
        sys.inequalities().len(),
    ));

    let report = witness_search(sys);
    out.push('\n');
    out.push_str(&format!(
        "witness search       {} sweeps, total violation {}\n",
        report.sweeps, report.violation,
    ));
    out.push_str("\nslack profile\n");
    for (row, slack) in sys.rows.iter().zip(&report.slack) {
        out.push_str(&format!("  {:<34}  {:>8}\n", row.label, slack));
    }

    if report.violation == 0 {
        out.push_str(&format!(
            "\nwitness              FEASIBILITY PROVEN by a re-checked point; binding K = {}\n",
            report.binding.len(),
        ));
        for label in &report.binding {
            out.push_str(&format!("  binding  {label}\n"));
        }
        out.push_str("the witness, as entries away from the V2-padded start\n");
        out.push_str(&render_moved(sys, &report.w));
        let implication = implied_by_the_rest(sys);
        out.push_str(&format!(
            "drop-one re-test     {} implied by the rest, {} independent, {} undecided\n",
            implication.implied.len(),
            implication.independent.len(),
            implication.undecided.len(),
        ));
        for label in &implication.implied {
            out.push_str(&format!("  implied  {label}\n"));
        }
        for label in &implication.undecided {
            out.push_str(&format!("  UNDECIDED  {label}\n"));
        }
        out.push_str(&format!("{SAT_READING}\n"));
    } else {
        out.push_str(
            "\nwitness              NONE. This is a statement about the SEARCH, not the system \
             -- only the exact oracle may say UNSAT.\n",
        );
    }

    let verdict = decide(sys);
    out.push_str(&format!("\nexact oracle         {}\n", verdict.name()));
    out.push_str(&render_verdict(sys, &verdict));

    if matches!(verdict, Verdict::Unsat { .. }) {
        let result = minimize(sys);
        out.push_str(&render_minimization(&result));
        // §5.4 probes the IIS, not the whole system: "an IIS lying wholly in
        // the early system re-decides at ... `w_vp(early)` ∈ {1, 2, 3, 6}".
        // Probing the whole system instead would MISS a scale finding whenever
        // some row outside the IIS contradicts for an unrelated reason.
        out.push_str(&render_diagnostics(&System { rows: result.kept, ..sys.clone() }));
    }
    out
}

/// §5.4 (3)'s minimized set, with the pair rows and the hand-ordering rows
/// recorded separately as the spec asks.
fn render_minimization(result: &Minimization) -> String {
    let mut out = String::new();
    out.push_str("\ndeletion-minimization (§5.2 document order, then the hand-ordering rows)\n");
    for row in &result.kept {
        let tier = if row.kind == RowKind::HandOrdering { "IIS/ordering" } else { "IIS/pair" };
        out.push_str(&format!("  {tier:<13}  {}\n", row.label));
    }
    for label in &result.dropped {
        out.push_str(&format!("  {:<13}  {label}\n", "dropped"));
    }
    if let Some(halt) = &result.halted {
        out.push_str(&format!("HALTED               {halt}\n"));
    }
    out.push_str(&format!("{IIS_READING}\n"));
    out
}

/// §5.4 (3)'s two registered scale diagnostics, over the IIS.
fn render_diagnostics(sys: &System) -> String {
    let diagnostics = scale_diagnostics(sys);
    let mut out = String::new();
    out.push_str(
        "\nscale diagnostics over the IIS (the pin and the box are probed, never deleted)\n",
    );
    out.push_str(&format!(
        "  box-widening at |w| <= {WIDENED_BOX_LIMIT}: {}{}\n",
        diagnostics.widened.name(),
        match diagnostics.widened {
            Verdict::Sat { .. } =>
                "  <- A SCALE FINDING under checkpoint §5B, not an architecture result",
            _ => "  (the contradiction does not dissolve when the box widens)",
        },
    ));
    if diagnostics.rescaled.is_empty() {
        out.push_str(
            "  early-rescale: not run -- §5.4 scopes the ladder to an IIS lying wholly in the \
             EARLY system\n",
        );
    } else {
        for (pin, verdict) in &diagnostics.rescaled {
            out.push_str(&format!(
                "  early-rescale at w_vp = {pin}: {}{}\n",
                verdict.name(),
                match verdict {
                    Verdict::Sat { .. } =>
                        "  <- INTEGER GRANULARITY, a SCALE finding, never architecture",
                    _ => "",
                },
            ));
        }
    }
    out
}

/// Run one toy through the whole instrument.
fn run_toy(name: &str) -> Result<(), String> {
    let sys = toys::named(name)
        .ok_or_else(|| format!("`{name}` is not a registered toy; see `solver toys`"))?;
    println!("=== solver toy {name} ===");
    print!("{}", identity_block(None));
    print!("{}", render_system(&sys));
    Ok(())
}

/// **THE REAL PART-B RUN.** Argument-gated behind `--real`, and invoked only
/// after this implementation is frozen and reviewed (§5.4's freeze-before-solve
/// clause).
fn run_real(freeze_commit: Option<&str>) -> Result<(), String> {
    let rules = Rules::standard();
    println!("=== solver solve --real  [§7 step 4, Part B] ===");
    print!("{}", identity_block(freeze_commit));
    print!("{}", census_block());
    println!(
        "\nfreeze-before-solve  the package digest above precedes this invocation, and the solver \
         identity is the frozen implementation's. Development ran on toy systems only."
    );

    for stage in StageEndpoint::ALL {
        println!("\n{}", "=".repeat(72));
        let mut rows = Vec::new();
        for pair in REGISTERED
            .iter()
            .filter(|p| matches!(p.role, PairRole::Constraint) && p.stages.contains(&stage))
        {
            rows.push(extract_row(pair, stage, &rules)?);
        }
        println!(
            "extraction           {} rows from the frozen registry at the {} endpoint\n",
            rows.len(),
            stage.name(),
        );
        print!("{}", render_system(&assemble(stage, rows, BOX_LIMIT)));
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().any(|a| a == name);
    let value = |name: &str| {
        args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
    };
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
        Some("solve") => match (flag("--real"), value("--toy")) {
            (true, Some(_)) => {
                Err("`--real` and `--toy` are different runs; name one".to_string())
            }
            (true, None) => run_real(value("--freeze-commit")),
            (false, Some(name)) => run_toy(name),
            (false, None) => Err(REAL_RUN_GATE.to_string()),
        },
        Some("repair") => {
            let start = value("--start").unwrap_or("v2-padded");
            // B1's driver is the DEFAULT and the raw projection is the opt-out,
            // because the driver is what the owner ruled the projection should
            // be run under. `--no-exceptions` keeps the un-excepted system
            // reachable from the command line, which is what makes "the raw
            // projection stays reachable" a thing a reader can check rather than
            // a claim about the code.
            let driver = !flag("--no-exceptions");
            match (flag("--real"), value("--toy")) {
                (true, Some(_)) => {
                    Err("`--real` and `--toy` are different runs; name one".to_string())
                }
                (true, None) => run_repair_real(start, value("--freeze-commit"), driver),
                (false, Some(name)) => run_repair_toy(name, start, driver),
                (false, None) => Err(REPAIR_RUN_GATE.to_string()),
            }
        }
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

/// What a bare `solve` says instead of solving.
const REAL_RUN_GATE: &str = "`solve` needs `--real` or `--toy <name>`. The REAL run over the \
     frozen registered pairs is gated behind `--real` on purpose: §5.4's freeze-before-solve \
     clause says the solver implementation freezes BEFORE the first real-pair invocation, so a \
     bare `solve` must not be able to reach the thirty-four by accident.";

/// What a bare `repair` says instead of projecting.
const REPAIR_RUN_GATE: &str = "`repair` needs `--real` or `--toy <name>`. R3's projection over the \
     frozen registered pairs is gated on the same clause `solve --real` is: the real thirty-four \
     are reachable deliberately or not at all.";

const USAGE: &str = "\
usage: solver <subcommand>

  identities         the identity block, the realized-vs-frozen census, and the
                     registry's margin table -- declarations only, no fixture built
  toys               list the registered toy systems
  toy <name>         run one toy end to end
  solve --toy <name> the same, spelled as a solve
  solve --real [--freeze-commit <hash>]
                     THE REAL PART-B RUN over the frozen registered pairs, per
                     stage. Gated: see §5.4's freeze-before-solve clause.
  repair --toy <name> [--start v2-padded|zero] [--no-exceptions]
                     R3's certified exact projection over one toy system
  repair --real [--start v2-padded|zero] [--freeze-commit <hash>]
                [--no-exceptions]
                     THE REAL §9 REGISTERED-MARGIN REPAIR, per endpoint: the
                     lexicographically nearest system-satisfying integer table
                     to the draft's starting vector, with its certificate trail.
                     --start defaults to v2-padded (R9's ratified anchor).
      B1'S CERTIFIED-EXCEPTION DRIVER runs ABOVE the projection by default
      (guidingpath 2026-08-25, \"THE T4-ENTRY BATCH IS RULED\"): on a typed
      REPAIR-INFEASIBLE it examines the still-active sign clauses in canonical
      `Feature::ALL` order and, for the FIRST coordinate whose system-without-
      that-one-clause is oracle-certified FEASIBLE, relaxes that clause alone --
      proven forced, because the full system is certified infeasible -- records
      both certificates, and re-runs. `--no-exceptions` runs the RAW
      un-excepted projection instead, which is what the standing S4-late
      regression pins.
      EXIT: 0 = the projection RAN. Every registered outcome is an ANSWER of a
      reporting instrument and every one of them exits 0 -- REPAIRED (with or
      without certified exceptions), REPAIR-JOINT-CONFLICT (certified infeasible
      with NO singly-forcing coordinate: outside B1's class, routed to the
      owner), REPAIR-INFEASIBLE (the raw projection's own verdict, reached under
      --no-exceptions) and INCONCLUSIVE (the oracle or a budget did not decide)
      alike. The answer is the VERDICT line, never the exit status: this
      subcommand has no GATE to read and say no, which is the whole difference
      from the probe's step-9 sections. Nonzero = the instrument could not RUN
      -- an unknown toy, an unregistered --start, a stray argument -- and says
      nothing about any system.
";

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::toys::{col, toy};
    use super::*;
    use yokohama_core::game::pair_fixtures::{PairRole, PAIR_11B, PAIR_3C};

/// The banked §8:975 witness at EARLY, transcribed from
/// `docs/temp/step6-artifacts/v3l-final-artifact.txt:275`.
const ARTIFACT_WITNESS_EARLY: [i64; AXIS_WIDTH] = [
    1, -64, 26, -4, 3, -60, -62, -64, -61, 12, 15, 8, 5, -64, 3, -64, 1, 1, -64, -31, 1, 1, 63, 2,
    2, 2, -30, 1, 1, 0, 1, 1, 1, 26,
];

/// The banked §8:975 witness at LATE, transcribed from
/// `docs/temp/step6-artifacts/v3l-final-artifact.txt:276`.
const ARTIFACT_WITNESS_LATE: [i64; AXIS_WIDTH] = [
    40, 29, -24, -64, -64, -62, -64, -64, -63, 20, 25, 14, -64, -64, 4, -64, 14, 1, -64, -31, 20,
    -64, 63, 21, 15, 0, 44, -61, 1, 1, 1, -63, 0, 0,
];

/// The rows the banked artifact's §4 projection DROPPED at EARLY, by pair id
/// (artifact:277 — "early kept 17 · dropped 7").
const ARTIFACT_DROPPED_EARLY: [&str; 7] = ["2(b)", "2(d)", "4(b)", "4(g)", "6(a)", "6(c)", "10(d)"];

/// The rows the banked artifact's §4 projection DROPPED at LATE, by pair id
/// (artifact:278 — "late kept 23 · dropped 4").
const ARTIFACT_DROPPED_LATE: [&str; 4] = ["4(b)", "4(g)", "6(a)", "6(c)"];

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

    /// The census reconciles with the count `eval_probe`'s own frozen-package
    /// cell asserts, which was arrived at independently.
    ///
    /// `every_expected_nonzero_candidate_row_moves_and_every_moving_row_is_listed`
    /// pins "the frozen package realizes 53 (pair, stage) fixtures". Those 53
    /// are this file's 51 constraint rows plus the two DIAGNOSTIC fixtures,
    /// 2(c) and 10(c), which are registered and signed but never solved. Two
    /// counts derived by different routes agreeing is worth more than either
    /// alone.
    #[test]
    fn the_census_reconciles_with_the_frozen_package_s_fixture_count() {
        let census = realized_census();
        let diagnostics: usize = REGISTERED
            .iter()
            .filter(|p| !matches!(p.role, PairRole::Constraint))
            .map(|p| p.stages.len())
            .sum();
        assert_eq!(diagnostics, 2, "2(c) and 10(c), one endpoint each");
        assert_eq!(
            census.early + census.late + diagnostics,
            53,
            "eval_probe pins 53 (pair, stage) fixtures over the same frozen package",
        );
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
        for &f in &Feature::ALL[..Feature::INCUMBENT_COUNT] {
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
        assert_eq!(Rat::int(1).div(Rat::ZERO), None, "division by zero is None");
        let third = Rat::new(1, 3).expect("representable");
        assert_eq!(third.add(third).and_then(|r| r.add(third)), Some(Rat::int(1)));
        assert_eq!(Rat::int(7).sub(Rat::int(9)), Some(Rat::int(-2)));
    }

    #[test]
    fn rational_comparison_is_exact_where_floating_point_would_tie() {
        let a = Rat::new(1, 3).expect("representable");
        let b = Rat::new(100_000_000_000_000_000, 300_000_000_000_000_001).expect("representable");
        assert_eq!(a.cmp_exact(b), Some(std::cmp::Ordering::Greater));
        assert_eq!(b.cmp_exact(a), Some(std::cmp::Ordering::Less));
        assert_eq!(a.cmp_exact(a), Some(std::cmp::Ordering::Equal));
        assert_eq!(a.sign(), 1);
        assert!(Rat::ZERO.is_zero());
    }

    // -- the exact oracle: the registered unit tests ------------------------

    fn verdict(name: &str) -> Verdict {
        decide(&toys::named(name).expect("a registered toy"))
    }

    /// §5.4's registered "known-SAT".
    #[test]
    fn the_oracle_decides_the_known_sat_toys_and_the_points_re_check() {
        for name in ["standing-only", "sat-reachable", "sat-coupled"] {
            let sys = toys::named(name).expect("registered");
            match decide(&sys) {
                Verdict::Sat { w } => {
                    assert_eq!(sys.violation(&w), 0, "{name}: the returned point is re-checked");
                    for (i, &x) in w.iter().enumerate() {
                        assert!(x >= sys.lo[i] && x <= sys.hi[i], "{name}: coordinate {i} in box");
                    }
                }
                other => panic!("{name} is satisfiable, got {}", other.name()),
            }
        }
    }

    /// The oracle decides what the descent could not — the whole reason §5.4
    /// keeps two instruments.
    #[test]
    fn the_oracle_decides_the_coupled_toy_the_witness_search_stalls_on() {
        let sys = toys::named("sat-coupled").expect("registered");
        assert!(witness_search(&sys).violation > 0, "the descent stalls here");
        assert!(matches!(decide(&sys), Verdict::Sat { .. }), "the oracle does not");
    }

    /// §5.4's registered "known-UNSAT", including the fractional-margin-derived
    /// system it names by name.
    #[test]
    fn the_oracle_decides_the_known_unsat_toys() {
        for name in [
            "unsat-direct",
            "unsat-box",
            "unsat-cycle",
            "unsat-fractional-margin",
            "unsat-integer-only",
            "unsat-box-widening",
        ] {
            assert!(
                matches!(verdict(name), Verdict::Unsat { .. }),
                "{name} is infeasible, got {}",
                verdict(name).name(),
            );
        }
    }

    /// The OTHER admitted certificate shape. `unsat-integer-only` is feasible
    /// over the rationals (`w_S1 = 1/2`), so no Farkas combination exists and
    /// §5.4's enumeration re-derivation is the only route to the verdict.
    #[test]
    fn an_integer_only_infeasibility_inside_the_budget_ships_an_enumeration() {
        let sys = toys::named("unsat-integer-only").expect("registered");
        let Verdict::Unsat { certificate: Certificate::Enumeration { support, points } } =
            decide(&sys)
        else {
            panic!("this system has no Farkas certificate; the enumeration is the only route");
        };
        assert_eq!(support, vec![col("S1")], "the support is the one column the rows read");
        assert_eq!(points, 2 * BOX_LIMIT as u128 + 1, "every integer in the box was tried");
        assert!(points <= ENUMERATION_BUDGET);
    }

    /// The three-row cycle no pair of rows contradicts — the cell that proves
    /// the oracle is a real LP and not an interval test wearing its name.
    #[test]
    fn the_cycle_needs_all_three_rows_and_the_certificate_says_so() {
        let sys = toys::named("unsat-cycle").expect("registered");
        let Verdict::Unsat { certificate: Certificate::Farkas { y, .. } } = decide(&sys) else {
            panic!("the cycle is relaxation-infeasible and ships Farkas");
        };
        let rows = sys.inequalities();
        let used: Vec<&str> = rows
            .iter()
            .zip(&y)
            .filter(|(_, m)| **m != 0)
            .map(|(r, _)| r.label.as_str())
            .collect();
        for label in ["S1 over S2", "S2 over S3", "S3 over S1"] {
            assert!(used.contains(&label), "the combination leans on {label}; it used {used:?}");
        }
    }

    /// §5.4's registered "Farkas-check round-trip" — and its teeth are the
    /// second half: a certificate that has been TAMPERED WITH is rejected.
    #[test]
    fn the_farkas_round_trip_accepts_the_real_certificate_and_rejects_a_tampered_one() {
        let sys = toys::named("unsat-direct").expect("registered");
        let Verdict::Unsat { certificate: Certificate::Farkas { y, rhs } } = decide(&sys) else {
            panic!("unsat-direct is relaxation-infeasible");
        };
        let rows = sys.inequalities();
        assert_eq!(verify_farkas(&rows, &y), Ok(rhs), "the shipped certificate re-verifies");
        assert!(rhs > 0, "the combination's right-hand side is positive");
        assert!(y.iter().all(|m| *m >= 0), "the multipliers are nonnegative");

        let first = y.iter().position(|m| *m != 0).expect("a nonzero multiplier");
        let mut bumped = y.clone();
        bumped[first] += 1;
        assert!(
            verify_farkas(&rows, &bumped).is_err(),
            "a perturbed combination no longer sums the coefficients to zero",
        );
        let mut negated = y.clone();
        negated[first] = -negated[first] - 1;
        assert!(verify_farkas(&rows, &negated).is_err(), "a negative multiplier is refused");
    }

    /// §5.4's registered "determinism".
    #[test]
    fn the_oracle_is_deterministic_across_repeats() {
        for name in toys::NAMES {
            let sys = toys::named(name).expect("registered");
            assert_eq!(decide(&sys), decide(&sys), "{name} decides identically on repeat");
        }
    }

    /// The overflow arm: "ANY overflow aborts to INCONCLUSIVE (never UNSAT)".
    #[test]
    fn an_overflowing_system_is_inconclusive_and_never_unsat() {
        let huge = i64::MAX;
        let sys = toy(
            StageEndpoint::Early,
            &[
                ("enormous up", &[(col("S1"), huge), (col("S2"), huge)], huge),
                ("enormous down", &[(col("S1"), -huge), (col("S2"), -huge)], huge),
            ],
            BOX_LIMIT,
        );
        match decide(&sys) {
            // If it DID decide, the certificate must still verify -- the point
            // of the arm is that an unverifiable one never passes.
            Verdict::Unsat { certificate: Certificate::Farkas { y, .. } } => {
                assert!(verify_farkas(&sys.inequalities(), &y).is_ok());
            }
            Verdict::Unsat { certificate: Certificate::Enumeration { points, .. } } => {
                assert!(points <= ENUMERATION_BUDGET);
            }
            Verdict::Unsat { certificate: Certificate::ChvatalGomory { .. } } => {
                panic!("`decide` does not build a Chvátal–Gomory certificate; only the extension \
                        does")
            }
            Verdict::Inconclusive { .. } => {}
            Verdict::Sat { .. } => panic!("these rows contradict; SAT is wrong either way"),
        }
    }

    /// The enumeration-admissibility bound: a support too wide to enumerate is
    /// INCONCLUSIVE, not UNSAT.
    #[test]
    fn an_integer_only_infeasibility_over_a_wide_support_is_inconclusive() {
        // Relaxation-FEASIBLE (w_S1 = 1/2 satisfies both) but integer-infeasible,
        // and padded with free columns so the support's product blows §5.4's
        // 10^7 budget.
        let up = [(col("S1"), 2)];
        let down = [(col("S1"), -2)];
        let wide: Vec<Vec<(usize, i64)>> =
            ["S2", "S3", "S4", "S5"].iter().map(|l| vec![(col(l), 1)]).collect();
        let mut rows: Vec<ToyRow> =
            vec![("2 S1 at least 1", &up, 1), ("2 S1 at most 1", &down, -1)];
        for entry in &wide {
            rows.push(("free column", entry.as_slice(), -BOX_LIMIT));
        }
        let sys = toy(StageEndpoint::Early, &rows, BOX_LIMIT);
        match decide(&sys) {
            Verdict::Inconclusive { reason } => {
                assert!(
                    reason.contains("enumeration") || reason.contains("budget"),
                    "the reason names what ran out: {reason}",
                );
            }
            Verdict::Unsat { certificate } => panic!(
                "an integer-only infeasibility over a wide support may not be claimed: {certificate:?}",
            ),
            Verdict::Sat { w } => panic!("2 w_S1 cannot be both >= 1 and <= 1 at an integer: {w:?}"),
        }
    }

    /// An UNSAT is never returned without a certificate that verifies — the
    /// structural statement of the trust boundary.
    #[test]
    fn every_unsat_the_oracle_returns_carries_a_certificate_that_verifies() {
        for name in toys::NAMES {
            let sys = toys::named(name).expect("registered");
            if let Verdict::Unsat { certificate } = decide(&sys) {
                match certificate {
                    Certificate::Farkas { y, rhs } => {
                        assert_eq!(verify_farkas(&sys.inequalities(), &y), Ok(rhs), "{name}");
                    }
                    Certificate::Enumeration { points, .. } => {
                        assert!(points <= ENUMERATION_BUDGET, "{name} stayed inside the budget");
                    }
                    // B2 puts the cuts in the QUESTION CHAIN, not in `decide`:
                    // the frozen oracle knows nothing about them and cannot ship
                    // one. A day when it does is a day `decide` grew a new
                    // answer, which is a fence, not a feature.
                    Certificate::ChvatalGomory { .. } => panic!(
                        "{name}: `decide` shipped a Chvátal–Gomory certificate, which only \
                         `decide_with_cuts` may build",
                    ),
                }
            }
        }
    }

    // -- B2: the Chvátal–Gomory certification extension ----------------------

    /// A relaxation-FEASIBLE, integer-infeasible system whose support is too
    /// wide to enumerate — the seam B2 extends.
    ///
    /// `2·w_S1 ≥ 1` and `−2·w_S1 ≥ −1` force `2·w_S1 = 1`, which no integer
    /// satisfies; `w_S1 = 1/2` satisfies both over the rationals, so there is no
    /// Farkas combination and the enumeration is the only §5.4 route. Four free
    /// columns push the support's box product past the registered 10⁷.
    fn wide_integer_only_system() -> System {
        let up = [(col("S1"), 2)];
        let down = [(col("S1"), -2)];
        let wide: Vec<Vec<(usize, i64)>> =
            ["S2", "S3", "S4", "S5"].iter().map(|l| vec![(col(l), 1)]).collect();
        let mut rows: Vec<ToyRow> =
            vec![("2 S1 at least 1", &up, 1), ("2 S1 at most 1", &down, -1)];
        for entry in &wide {
            rows.push(("free column", entry.as_slice(), -BOX_LIMIT));
        }
        // The borrow has to outlive the builder call, so the system is built
        // inside the function that owns `wide`.
        toy(StageEndpoint::Early, &rows, BOX_LIMIT)
    }

    /// A system the search must BRANCH on: `2·w_S1 + 2·w_S2 = 1` is comfortable
    /// at `(1/4, 1/4)` over the rationals and unreachable over the integers, and
    /// propagation shaves the box without emptying it — so a starved node budget
    /// has something to run out of.
    fn starved_search_system() -> System {
        toy(
            StageEndpoint::Early,
            &[
                ("twice S1 plus twice S2, at least 1", &[(col("S1"), 2), (col("S2"), 2)], 1),
                ("twice S1 plus twice S2, at most 1", &[(col("S1"), -2), (col("S2"), -2)], -1),
            ],
            BOX_LIMIT,
        )
    }

    /// **THE TRIGGER IS THE ENUMERATION-BUDGET ROUTE AND NOTHING ELSE.**
    ///
    /// B2 extends ONE seam. A node-budget INCONCLUSIVE is a different fact — the
    /// search ran out of nodes, and the system may well be satisfiable — so the
    /// predicate is checked against BOTH routes rather than against a phrase.
    #[test]
    fn the_extension_triggers_on_the_enumeration_budget_route_and_not_the_node_one() {
        let Verdict::Inconclusive { reason } = decide(&wide_integer_only_system()) else {
            panic!("the wide system routes over the enumeration budget");
        };
        assert!(is_enumeration_budget_route(&reason), "the trigger reads this route: {reason}");

        let starved = starved_search_system();
        let columns = active_columns(&starved.rows);
        let Verdict::Inconclusive { reason } = integer_search(&starved, &columns, 1) else {
            panic!("a one-node budget cannot decide a system the search has to branch on");
        };
        assert!(reason.contains("budget of 1 nodes"), "this IS the node route: {reason}");
        assert!(!is_enumeration_budget_route(&reason), "and not the enumeration one: {reason}");
    }

    /// **BORN-VALIDATION (d): ONE CUT CERTIFIES IT, and the cut is derivable by
    /// hand.**
    ///
    /// `2·w_S1 ≥ 1` has coefficient gcd 2 and an odd right-hand side, so
    /// multiplier `1/2` gives the integral vector `w_S1` with right-hand side
    /// `1/2`, and C–G rounds it up: `w_S1 ≥ 1`. The mirror row gives
    /// `−w_S1 ≥ 0`. Those two contradict at the RELAXATION, so the strengthened
    /// system ships a Farkas certificate and the answer is UNSAT rather than the
    /// honest INCONCLUSIVE `decide` alone can reach.
    #[test]
    fn a_wide_support_integer_infeasibility_is_certified_by_chvatal_gomory_rounding() {
        let sys = wide_integer_only_system();
        assert!(
            matches!(decide(&sys), Verdict::Inconclusive { .. }),
            "the frozen oracle's own answer is unchanged, which is the fence",
        );

        let Verdict::Unsat { certificate: Certificate::ChvatalGomory { cuts, inner } } =
            decide_with_cuts(&sys)
        else {
            panic!("the extension certifies this system");
        };
        assert!(
            matches!(*inner, Certificate::Farkas { .. }),
            "the strengthened system is relaxation-infeasible and ships FARKAS",
        );
        // The two cuts, by hand: `w_S1 >= 1` and `-w_S1 >= 0`.
        let s1 = col("S1");
        let mut found = Vec::new();
        for cut in &cuts {
            let nonzero: Vec<(usize, i64)> =
                cut.coef.iter().enumerate().filter(|(_, c)| **c != 0).map(|(i, c)| (i, *c)).collect();
            if nonzero == vec![(s1, 1)] && cut.rhs == 1 {
                found.push("w_S1 >= 1");
            }
            if nonzero == vec![(s1, -1)] && cut.rhs == 0 {
                found.push("-w_S1 >= 0");
            }
        }
        found.sort_unstable();
        assert_eq!(found, ["-w_S1 >= 0", "w_S1 >= 1"], "both halves of 2 w_S1 = 1, rounded");
        // Every parent is a REAL parent, at a nonnegative rational multiplier.
        for cut in &cuts {
            assert!(!cut.parents.is_empty());
            assert!(cut.parents.iter().all(|(_, y)| y.sign() > 0), "C–G needs y >= 0");
        }
        assert_eq!(verify_cut_chain(&sys.inequalities(), &cuts), Ok(()), "the chain re-verifies");
    }

    /// **THE TRUST BOUNDARY: a TAMPERED cut is rejected.**
    ///
    /// The verifier shares no code with the derivation — it recomputes
    /// `Σ y_i A_i` and `⌈Σ y_i b_i⌉` from the parents' own row data — so a
    /// derivation defect costs cuts and cannot cost soundness. Both halves are
    /// exercised: a strengthened right-hand side and a strengthened
    /// coefficient.
    #[test]
    fn a_tampered_chvatal_gomory_cut_does_not_re_verify() {
        let sys = wide_integer_only_system();
        let base = sys.inequalities();
        let cuts = derive_cuts(&sys);
        assert!(!cuts.is_empty(), "there is something to tamper with");
        assert_eq!(verify_cut_chain(&base, &cuts), Ok(()));

        let mut stronger_rhs = cuts.clone();
        stronger_rhs[0].rhs += 1;
        assert!(verify_cut_chain(&base, &stronger_rhs).is_err(), "a lifted rhs is not the ceiling");

        let mut stronger_coef = cuts.clone();
        let slot = stronger_coef[0].coef.iter().position(|c| *c != 0).expect("a nonzero column");
        stronger_coef[0].coef[slot] *= 2;
        assert!(verify_cut_chain(&base, &stronger_coef).is_err(), "the combination says otherwise");

        let mut absent_parent = cuts.clone();
        absent_parent[0].parents[0].0 = base.len() + 99;
        assert!(verify_cut_chain(&base, &absent_parent).is_err(), "a parent that is not there");
    }

    /// The extension NEVER turns "the oracle did not decide" into infeasibility.
    ///
    /// A system whose integer infeasibility no derived cut reaches keeps the
    /// typed INCONCLUSIVE, and the reason says a C–G extension was tried — the
    /// honest residual B2 names.
    #[test]
    fn an_extension_that_cannot_certify_leaves_the_answer_inconclusive() {
        // `w_S1 - w_S2 >= 1` over four extra free columns: relaxation-feasible
        // AND integer-feasible, so the search finds a point and no route is
        // taken at all. The point is that the extension cannot manufacture an
        // infeasibility where there is none.
        let feasible = toys::named("sat-coupled").expect("registered");
        assert!(matches!(decide_with_cuts(&feasible), Verdict::Sat { .. }));

        // And a genuinely undecided one: at a one-node search budget the answer
        // is the node-budget route, which the extension does not touch.
        let starved = starved_search_system();
        let columns = active_columns(&starved.rows);
        let Verdict::Inconclusive { reason } = integer_search(&starved, &columns, 1) else {
            panic!("a one-node budget cannot decide");
        };
        assert!(reason.contains("not infeasibility"), "{reason}");
        assert!(!is_enumeration_budget_route(&reason), "{reason}");
    }

    /// A SAT answer through the extension is re-checked against the ORIGINAL
    /// rows before it is returned — the cuts are valid for integer points, so a
    /// point of the strengthened system is a point of the original, and the
    /// re-check is what makes that a property of the code.
    #[test]
    fn a_sat_reached_through_the_extension_satisfies_the_unstrengthened_system() {
        for name in toys::NAMES {
            let sys = toys::named(name).expect("registered");
            if let Verdict::Sat { w } = decide_with_cuts(&sys) {
                assert_eq!(sys.violation(&w), 0, "{name}: the point satisfies the original rows");
            }
        }
    }

    /// The extension is DETERMINISTIC: same system, same cuts, same certificate.
    #[test]
    fn the_chvatal_gomory_derivation_is_deterministic() {
        let sys = wide_integer_only_system();
        let first = derive_cuts(&sys);
        let second = derive_cuts(&sys);
        assert_eq!(first, second, "the derivation reads no clock and draws no entropy");
        assert_eq!(decide_with_cuts(&sys), decide_with_cuts(&sys));
    }

    /// A derived cut carries its parents and multipliers, and the rendered
    /// certificate names them — B2's "the certificate stays independently
    /// checkable" is a property of the page, not only of the type.
    #[test]
    fn a_rendered_chvatal_gomory_certificate_names_its_cuts_and_their_parents() {
        let sys = wide_integer_only_system();
        let Verdict::Unsat { certificate } = decide_with_cuts(&sys) else {
            panic!("the extension certifies this system");
        };
        let rendered = render_certificate(&certificate);
        assert!(rendered.contains("CHVATAL-GOMORY"), "{rendered}");
        assert!(rendered.contains("FARKAS"), "the inner certificate rides along: {rendered}");
        let leans = certificate_leans(&sys.inequalities(), &certificate);
        assert!(
            leans.iter().any(|l| l.contains("2 S1 at least 1")),
            "the parent row is named: {leans:?}",
        );
        assert!(leans.iter().any(|l| l.contains("1/2")), "with its multiplier: {leans:?}");
    }

    /// **THE ENGINE-LOCAL BUDGET KEEPS ITS PROVENANCE.** §5.4 registers 10⁷ for
    /// the enumeration; B2 leaves that figure alone and says so.
    #[test]
    fn the_extension_leaves_the_registered_enumeration_budget_where_it_was() {
        assert_eq!(ENUMERATION_BUDGET, 10_000_000);
        assert_eq!(NODE_BUDGET, 10_000_000);
    }

    /// **THE REAL EARLY ENDPOINT CERTIFIES UNDER B2 AND REPAIRS.**
    ///
    /// T3 left this endpoint routed INCONCLUSIVE: the first probed displacement
    /// level was relaxation-feasible and integer-infeasible, and a 38-column
    /// support could not be re-derived inside the registered enumeration budget.
    /// The extension closes exactly that level, and the cuts that close it are
    /// the shape [`single_row_cuts`]' doc predicts — registered pair rows whose
    /// realized `Δv` are ±2 against an odd margin, rounded onto the integers:
    ///
    /// * `1(d) early` `2·w_S2 ≥ 1` → `w_S2 ≥ 1`
    /// * `3(c) early` `2·w_S5 ≥ 1` → `w_S5 ≥ 1`
    /// * `10(a) early` `2·w_X2 ≥ 1` → `w_X2 ≥ 1`
    /// * `11(b) early` `2(w_E1 − w_S5) ≥ 1` → `w_E1 − w_S5 ≥ 1`
    ///
    /// The certificate the level closes with is re-verified here against the
    /// system it was derived over, so the level's emptiness is checkable from
    /// this cell without re-running the projection.
    #[test]
    fn the_real_early_endpoint_certifies_under_the_extension_and_repairs() {
        let rules = Rules::standard();
        let mut rows = Vec::new();
        for pair in REGISTERED.iter().filter(|p| {
            matches!(p.role, PairRole::Constraint) && p.stages.contains(&StageEndpoint::Early)
        }) {
            rows.push(
                extract_row(pair, StageEndpoint::Early, &rules).expect("the registry extracts"),
            );
        }
        let (kept, dropped) = project_pair_rows(rows);
        let sys = assemble(StageEndpoint::Early, kept, BOX_LIMIT);
        let w0 = start_vector("v2-padded", StageEndpoint::Early).expect("a registered start");
        let problem = RepairProblem::new(StageEndpoint::Early, sys, dropped, "v2-padded", w0)
            .expect("the anchor is admissible");

        let Repair::Repaired(report) = repair(&problem) else {
            panic!(
                "the early endpoint certifies under B2's extension; a routed answer here is a \
                 finding to re-derive, not a flake",
            );
        };
        assert_eq!(report.displacement, 14, "the certified optimum's displacement");
        assert_eq!(report.changed.len(), 11, "over eleven changed coordinates");
        for (label, value) in
            [("S1", 1), ("S2", 1), ("S5", 1), ("S6", 1), ("S7", 1), ("E1", 2), ("E2", 2)]
        {
            assert_eq!(report.w[col(label)], value, "{label} at the early optimum");
        }
        assert_eq!(
            report.w[Feature::VictoryPoints.index()],
            1,
            "and the pin is where R1 fixed it",
        );

        // THE LEVEL BELOW, and what closed it.
        let below = format!("displacement <= {}", report.displacement - 1);
        let Some(TrailOutcome::Empty(Certificate::ChvatalGomory { cuts, inner })) =
            report.step(&below).map(|s| &s.outcome)
        else {
            panic!("level {below} is closed by a Chvátal–Gomory certificate: {below}");
        };
        assert!(matches!(**inner, Certificate::Farkas { .. }), "the strengthened system is empty");
        let rendered: Vec<String> =
            cuts.iter().map(|c| format!("{}  [{}]", c.label, sparse_row(&c.row()))).collect();
        for want in [
            "C-G cut: ceil(1/2 x [pair 1(d) early])  [S2:+1 >= 1]",
            "C-G cut: ceil(1/2 x [pair 3(c) early])  [S5:+1 >= 1]",
            "C-G cut: ceil(1/2 x [pair 10(a) early])  [X2:+1 >= 1]",
            "C-G cut: ceil(1/2 x [pair 11(b) early])  [S5:-1 E1:+1 >= 1]",
        ] {
            assert!(rendered.iter().any(|r| r == want), "the derived cuts are {rendered:?}");
        }
        // The chain re-verifies against the system the level was asked about —
        // the augmented one, rebuilt here rather than trusted.
        let level = augmented(&problem, report.displacement - 1, &problem.free, &[]);
        assert_eq!(verify_cut_chain(&level.inequalities(), cuts), Ok(()));
    }

    #[test]
    fn active_columns_are_the_ones_a_row_actually_reads() {
        let sys = toys::named("unsat-direct").expect("registered");
        assert_eq!(active_columns(&sys.rows), vec![col("S1")]);
        let widened = toys::named("unsat-fractional-margin").expect("registered");
        let mut want = vec![Feature::VictoryPoints.index(), col("E1")];
        want.sort_unstable();
        assert_eq!(active_columns(&widened.rows), want, "the pinned w_vp is read by the rate row");
    }

    /// **The scale probe decides at the early system's full width.**
    ///
    /// Twenty-six rows over thirty-four columns is the shape the real early
    /// system will present, and the question this answers is whether the exact
    /// rational simplex survives it — a question no small toy can, and one the
    /// real run should not be the first to ask. The coefficients are synthetic,
    /// so the VERDICT means nothing; that it is a verdict at all is the point.
    #[test]
    fn the_exact_oracle_decides_at_the_early_system_s_full_width() {
        let sys = toys::named("scale-probe-full-width").expect("registered");
        assert_eq!(sys.rows.len(), 26);
        assert_eq!(active_columns(&sys.rows).len(), AXIS_WIDTH, "every column is read");
        assert_eq!(sys.inequalities().len(), 26 + 2 * AXIS_WIDTH);
        let verdict = decide(&sys);
        assert!(
            !matches!(verdict, Verdict::Inconclusive { .. }),
            "the oracle must reach a verdict at this width: {verdict:?}",
        );
        assert_eq!(decide(&sys), verdict, "and reach the same one on repeat");
    }

    #[test]
    fn the_node_and_enumeration_budgets_are_the_registered_ones() {
        assert_eq!(NODE_BUDGET, 10_000_000, "§5.4: 10^7 nodes per stage system");
        assert_eq!(ENUMERATION_BUDGET, 10_000_000, "§5.4: 10^7 points");
    }

    /// A plain seeded 2P game — deliberately NOT a registered fixture, so the
    /// real-path unit tests exercise the machinery without touching the frozen
    /// thirty-four (§5.4's freeze-before-solve clause).
    fn plain_two_player_state(rules: &Rules) -> yokohama_core::GameState {
        let pending =
            yokohama_core::new_game(yokohama_core::GameConfig::for_player_count(2), rules, 0x50_1E)
                .expect("a 2P setup");
        yokohama_core::finalize_setup(
            pending,
            &[
                yokohama_core::StartingOrderChoice::Keep(0),
                yokohama_core::StartingOrderChoice::Keep(0),
            ],
        )
        .expect("finalized")
    }

    // -- minimization ------------------------------------------------------

    /// Deletion drops what the contradiction does not need and keeps what it
    /// does.
    #[test]
    fn minimization_keeps_the_rows_the_contradiction_needs() {
        // Two contradicting rows plus a bystander that no infeasibility uses.
        let sys = toy(
            StageEndpoint::Early,
            &[
                ("S1 up", &[(col("S1"), 1)], 1),
                ("a bystander", &[(col("E1"), 1)], -60),
                ("S1 down", &[(col("S1"), -1)], 1),
            ],
            BOX_LIMIT,
        );
        assert!(matches!(decide(&sys), Verdict::Unsat { .. }), "the system starts infeasible");
        let result = minimize(&sys);
        assert!(result.halted.is_none(), "nothing was inconclusive here");
        let kept: Vec<&str> = result.kept.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(kept, vec!["S1 up", "S1 down"]);
        assert_eq!(result.dropped, vec!["a bystander".to_string()]);
        let reduced = System { rows: result.kept.clone(), ..sys.clone() };
        assert!(matches!(decide(&reduced), Verdict::Unsat { .. }), "the IIS is still infeasible");
        // Irreducible: removing any one of them makes it satisfiable.
        for i in 0..result.kept.len() {
            let mut rows = result.kept.clone();
            rows.remove(i);
            let smaller = System { rows, ..sys.clone() };
            assert!(
                matches!(decide(&smaller), Verdict::Sat { .. }),
                "dropping row {i} of the IIS must relieve the infeasibility",
            );
        }
    }

    /// §5.4's registered "deletion-order reproducibility pin".
    ///
    /// Two rows are interchangeable — either one alone contradicts the first —
    /// so the IIS is NOT unique and the ORDER decides which survives. The pin
    /// names the survivor, which is what makes "order-dependent, not
    /// necessarily minimum-cardinality" a checked property rather than a
    /// disclaimer.
    #[test]
    fn the_deletion_order_decides_which_of_two_interchangeable_rows_survives() {
        let sys = toy(
            StageEndpoint::Early,
            &[
                ("A: S1 up", &[(col("S1"), 1)], 1),
                ("B: S1 down, first copy", &[(col("S1"), -1)], 1),
                ("C: S1 down, second copy", &[(col("S1"), -1)], 1),
            ],
            BOX_LIMIT,
        );
        let result = minimize(&sys);
        let kept: Vec<&str> = result.kept.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(
            kept,
            vec!["A: S1 up", "C: S1 down, second copy"],
            "B is probed first and its removal leaves {{A, C}} infeasible, so B goes and C stays",
        );
        assert_eq!(result.dropped, vec!["B: S1 down, first copy".to_string()]);
        // Reproducible: the same order gives the same set, every time.
        for _ in 0..3 {
            let again = minimize(&sys);
            assert_eq!(
                again.kept.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
                kept,
            );
            assert_eq!(again.dropped, result.dropped);
        }
    }

    /// **S-2 — the HALT arm.** §5.4: "an INCONCLUSIVE child during
    /// deletion-minimization HALTS the minimization, which reports the current
    /// (non-minimal) infeasible set with the INCONCLUSIVE named".
    ///
    /// The construction has to make the FIRST probe inconclusive, which takes
    /// two ingredients: a direct contradiction placed first (so the whole system
    /// is UNSAT and minimization runs at all), and a relaxation-feasible,
    /// integer-infeasible pair over a support too wide to enumerate (so that
    /// removing the first row leaves a system the oracle cannot decide).
    #[test]
    fn an_inconclusive_probe_halts_the_minimization_and_is_named_in_the_report() {
        let up = [(col("S6"), 1)];
        let down = [(col("S6"), -1)];
        let twice_up = [(col("S1"), 2)];
        let twice_down = [(col("S1"), -2)];
        // Padding columns to push the enumeration support over §5.4's budget:
        // five columns of 129 values is 129^5, far past 10^7.
        let wide: Vec<Vec<(usize, i64)>> =
            ["S2", "S3", "S4"].iter().map(|l| vec![(col(l), 1)]).collect();
        let mut rows: Vec<ToyRow> = vec![
            ("S6 up", &up, 1),
            ("S6 down", &down, 1),
            ("twice S1 at least 1", &twice_up, 1),
            ("twice S1 at most 1", &twice_down, -1),
        ];
        for entry in &wide {
            rows.push(("a free column", entry.as_slice(), -BOX_LIMIT));
        }
        let sys = toy(StageEndpoint::Early, &rows, BOX_LIMIT);
        assert!(
            matches!(decide(&sys), Verdict::Unsat { .. }),
            "the S6 pair contradicts at the relaxation, so the whole system is infeasible",
        );

        let result = minimize(&sys);
        let halt = result.halted.clone().expect("probe 0 leaves an undecidable system");
        assert!(halt.contains("S6 up"), "the halt names the probe that caused it: {halt}");
        assert!(halt.contains("INCONCLUSIVE"), "{halt}");
        assert!(halt.contains("NOT \nminimal") || halt.contains("NOT minimal"), "{halt}");
        assert_eq!(
            result.kept.len(),
            sys.rows.len(),
            "the pass stopped at the first probe, so every row is still in the reported set",
        );
        assert!(result.dropped.is_empty(), "nothing was established as droppable before the halt");
        assert!(
            render_minimization(&result).contains("HALTED"),
            "and the artifact says so rather than presenting a completed-looking core",
        );
    }

    // -- S-1: the rescale ladder re-prices what the pin prices ---------------

    /// §5.1's MARGINED formula, evaluated at the SHIPPED pin, reproduces the
    /// registered integers — the tie between the transcribed fractions and the
    /// registry.
    #[test]
    fn the_transcribed_fractions_reproduce_the_registered_integers() {
        for (id, numerator, denominator) in MARGIN_VP {
            let pair = REGISTERED.iter().find(|p| p.id == id).expect("registered");
            assert!(matches!(pair.margin, MarginClass::Margined { .. }), "{id} is MARGINED");
            assert_eq!(margin_fraction(id), Some((numerator, denominator)));
            for stage in pair.stages {
                let shipped = toys::pinned_vp(*stage);
                assert_eq!(
                    margined_integer(numerator, denominator, shipped),
                    margin_of(pair, *stage).expect("priced"),
                    "{id} at {}: the formula at the shipped pin IS the registered integer",
                    stage.name(),
                );
            }
        }
        // And the classes that are NOT prices carry no fraction, so nothing
        // re-prices them.
        for pair in REGISTERED.iter() {
            if !matches!(pair.margin, MarginClass::Margined { .. }) {
                assert_eq!(margin_fraction(pair.id), None, "{} is not MARGINED", pair.id);
            }
        }
    }

    /// **S-1's defect, as a cell.** A MARGINED margin is a FUNCTION of the pin,
    /// so moving the pin without re-evaluating it UNDERSTATES the row — which
    /// would let a real infeasibility come back SAT under rescale and be
    /// labelled a SCALE finding. Anti-conservative in exactly the §5B
    /// direction.
    #[test]
    fn a_margined_row_re_prices_at_each_rung_of_the_rescale_ladder() {
        // The reviewer's example: 3(c)'s margin_vp is 1, so the shipped early
        // integer is 1 and the rung-2 integer must be 2.
        assert_eq!(margined_integer(1, 1, 1), 1, "3(c) at the shipped early pin");
        assert_eq!(margined_integer(1, 1, 2), 2, "3(c) at rung 2 -- NOT the registered 1");
        assert_eq!(margined_integer(1, 1, 6), 6, "3(c) at rung 6");
        // And the fractional rows, whose early floor is the strengthening.
        assert_eq!(margined_integer(1, 3, 1), 1, "1(a)'s third of a VP floors to 1");
        assert_eq!(margined_integer(1, 3, 3), 1, "at rung 3 a third of 3 VP is exactly 1");
        assert_eq!(margined_integer(1, 3, 6), 2, "at rung 6 it is 2");

        // A REAL registered row, built from synthetic signatures so no fixture
        // is touched, must carry its fraction and re-price along the ladder.
        let pair = REGISTERED.iter().find(|p| p.id == "3(c)").expect("registered");
        let mut greater = vec![0i64; AXIS_WIDTH];
        greater[col("S5")] = 2;
        let row = row_from_signatures(pair, StageEndpoint::Early, &greater, &vec![0; AXIS_WIDTH])
            .expect("a listed row");
        assert_eq!(row.margin_vp, Some((1, 1)), "the row carries 3(c)'s registered fraction");
        assert_eq!(row.rhs, 1, "at the shipped pin it is the registered integer");

        let sys = assemble(StageEndpoint::Early, vec![row], BOX_LIMIT);
        let rungs = scale_diagnostics(&sys).rescaled;
        assert_eq!(rungs.len(), EARLY_RESCALE_LADDER.len());
        for (pin, _) in &rungs {
            let repriced = repriced_at(&sys, *pin);
            let margined = repriced
                .rows
                .iter()
                .find(|r| r.margin_vp.is_some())
                .expect("the 3(c) row is still there");
            assert_eq!(
                margined.rhs,
                margined_integer(1, 1, *pin),
                "at rung {pin} the margined row is re-priced, not left at its shipped integer",
            );
            assert_eq!(margined.rhs, *pin, "3(c)'s margin_vp is 1, so its margin IS the pin");
            assert_eq!(repriced.pin_vp, *pin);
        }
        // The rows that are not prices do NOT move.
        let orderings = repriced_at(&sys, 6);
        for row in orderings.rows.iter().filter(|r| r.kind == RowKind::HandOrdering) {
            assert_eq!(row.rhs, 1, "a hand-ordering row is not a price and does not re-price");
        }
    }

    /// **The cell that proves the LADDER re-prices, not merely that
    /// [`repriced_at`] can.**
    ///
    /// My first pass at S-1 checked `repriced_at` directly and never asserted
    /// that `scale_diagnostics` calls it — a mutation reverting the ladder to
    /// the uncalibrated form passed every test. This cell closes that: it is
    /// built so the two forms DISAGREE, and it asserts which one ships.
    ///
    /// A margined row at `margin_vp = 1` (3(c)'s rate) against a plain cap of 1.
    /// At the shipped pin the margin is 1 and the cap admits it. At rung 2 the
    /// formula makes the margin 2 while the cap still says 1 — so the CALIBRATED
    /// ladder reports UNSAT and the UNCALIBRATED one reports SAT. Since a
    /// rescale-SAT is printed as a SCALE finding, the defect's answer would have
    /// filed a live contradiction as integer granularity, which is the
    /// anti-conservative failure §5B must not make.
    #[test]
    fn the_rescale_ladder_reports_the_calibrated_verdict_where_the_two_forms_disagree() {
        let mut sys = toy(
            StageEndpoint::Early,
            &[
                ("a margined row at rate 1/1", &[(col("S5"), 1)], 1),
                ("a cap of 1", &[(col("S5"), -1)], -1),
            ],
            BOX_LIMIT,
        );
        sys.rows[0].margin_vp = Some((1, 1));

        let uncalibrated =
            decide(&System::new(sys.stage, sys.rows.clone(), sys.box_limit, 2));
        assert!(
            matches!(uncalibrated, Verdict::Sat { .. }),
            "the DEFECT's answer at rung 2: the pin moved, the margin did not, and the cap of 1 \
             still admits the shipped margin of 1",
        );

        let rungs = scale_diagnostics(&sys).rescaled;
        let at = |pin: i64| {
            rungs.iter().find(|(p, _)| *p == pin).map(|(_, v)| v).expect("a registered rung")
        };
        assert!(
            matches!(at(1), Verdict::Sat { .. }),
            "at the shipped pin the margin is 1 and the cap admits it",
        );
        assert!(
            matches!(at(2), Verdict::Unsat { .. }),
            "THE CALIBRATED ANSWER: at rung 2 the formula prices the margin at 2 and the cap of \
             1 cannot meet it. Reporting SAT here would file a live contradiction as a SCALE \
             finding.",
        );
        for pin in [3, 6] {
            assert!(matches!(at(pin), Verdict::Unsat { .. }), "and above it, rung {pin}");
        }
    }

    /// The fractional-margin toy's rescale verdict, RE-DERIVED under the
    /// calibrated form.
    ///
    /// Its margin row declares the ⅓ rate the toy is built around, so the
    /// ladder now re-prices it: 1, 1, 1, 2 at rungs 1, 2, 3, 6. The verdicts are
    /// UNCHANGED — the rung-3 SCALE finding survives calibration, because the
    /// rate row relaxes exactly as fast as the margin does. Recorded rather than
    /// assumed, per the wave's instruction that a changed verdict would be the
    /// finding.
    #[test]
    fn the_fractional_margin_toy_keeps_its_finding_under_the_calibrated_ladder() {
        let sys = toys::named("unsat-fractional-margin").expect("registered");
        let margin_row = sys.rows.iter().find(|r| r.margin_vp.is_some()).expect("declares a rate");
        assert_eq!(margin_row.margin_vp, Some((1, 3)), "the toy's own 1/3 VP rate");

        let expected: [(i64, i64, bool); 4] =
            [(1, 1, false), (2, 1, false), (3, 1, true), (6, 2, true)];
        let rungs = scale_diagnostics(&sys).rescaled;
        for (pin, want_rhs, want_sat) in expected {
            let repriced = repriced_at(&sys, pin);
            let row = repriced.rows.iter().find(|r| r.margin_vp.is_some()).expect("present");
            assert_eq!(row.rhs, want_rhs, "the re-priced margin at rung {pin}");
            let verdict = rungs.iter().find(|(p, _)| *p == pin).map(|(_, v)| v).expect("a rung");
            assert_eq!(
                matches!(verdict, Verdict::Sat { .. }),
                want_sat,
                "rung {pin} under the CALIBRATED ladder: the finding is unchanged, because the \
                 rate row relaxes exactly as fast as the margin it prices",
            );
        }
    }

    #[test]
    fn the_iis_reading_sentence_is_the_one_the_spec_registers() {
        assert!(IIS_READING.contains("IRREDUCIBLE INFEASIBLE SUBSYSTEM"));
        assert!(IIS_READING.contains("ORDER-DEPENDENT"));
        assert!(IIS_READING.contains("NOT NECESSARILY MINIMUM-CARDINALITY"));
    }

    // -- the two scale diagnostics -----------------------------------------

    /// §5.4's box-widening diagnostic, on the toy built to make it fire.
    #[test]
    fn box_widening_finds_the_scale_case_and_leaves_the_others_alone() {
        let scaled = toys::named("unsat-box-widening").expect("registered");
        assert!(matches!(decide(&scaled), Verdict::Unsat { .. }));
        assert!(
            matches!(scale_diagnostics(&scaled).widened, Verdict::Sat { .. }),
            "the spread 129 needs |w| <= 128 and gets it -- a SCALE finding",
        );

        let genuine = toys::named("unsat-direct").expect("registered");
        assert!(
            matches!(scale_diagnostics(&genuine).widened, Verdict::Unsat { .. }),
            "a real contradiction does not dissolve when the box widens",
        );
    }

    /// §5.4's early-rescale diagnostic, on the fractional-margin toy built to
    /// make it fire.
    #[test]
    fn the_early_rescale_ladder_resolves_the_fractional_margin_system() {
        let sys = toys::named("unsat-fractional-margin").expect("registered");
        let rungs = scale_diagnostics(&sys).rescaled;
        assert_eq!(
            rungs.iter().map(|(v, _)| *v).collect::<Vec<_>>(),
            EARLY_RESCALE_LADDER.to_vec(),
            "the registered ladder, in order",
        );
        let at = |pin: i64| {
            rungs.iter().find(|(v, _)| *v == pin).map(|(_, verdict)| verdict).expect("a rung")
        };
        assert!(matches!(at(1), Verdict::Unsat { .. }), "the shipped pin is where it contradicts");
        assert!(
            matches!(at(3), Verdict::Sat { .. }),
            "at w_vp = 3 the 1/3-VP rate admits w_E1 = 1 and the floored margin is satisfiable: \
             the contradiction was the 3x early floor, not the architecture",
        );
    }

    #[test]
    fn a_late_system_runs_no_early_rescale() {
        let sys = toy(
            StageEndpoint::Late,
            &[("S1 up", &[(col("S1"), 1)], 1), ("S1 down", &[(col("S1"), -1)], 1)],
            BOX_LIMIT,
        );
        assert!(
            scale_diagnostics(&sys).rescaled.is_empty(),
            "§5.4 scopes the rescale ladder to an IIS lying wholly in the EARLY system",
        );
    }

    /// **The diagnostics probe the IIS, not the whole system** — and this cell
    /// is built so the two answers DIFFER.
    ///
    /// The fractional-margin pair is deletion-probed last, so it survives as the
    /// IIS while an unrelated contradiction on another column is dropped. Under
    /// the rescale ladder the IIS resolves at `w_vp = 3` and the WHOLE system
    /// never does — so a diagnostic run over the whole system would report
    /// nothing and MISS the scale finding.
    #[test]
    fn the_scale_diagnostics_read_the_iis_and_not_the_whole_system() {
        let sys = toy(
            StageEndpoint::Early,
            &[
                ("an unrelated contradiction, up", &[(col("S1"), 1)], 1),
                ("an unrelated contradiction, down", &[(col("S1"), -1)], 1),
                ("E1 floored margin", &[(col("E1"), 1)], 1),
                (
                    "the 1/3 VP rate itself",
                    &[(col("E1"), -3), (Feature::VictoryPoints.index(), 1)],
                    0,
                ),
            ],
            BOX_LIMIT,
        );
        let iis = minimize(&sys);
        assert_eq!(
            iis.kept.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
            vec!["E1 floored margin", "the 1/3 VP rate itself"],
            "the S1 pair is probed first, and each of its rows is droppable while the \
             fractional pair still contradicts",
        );

        let whole = scale_diagnostics(&sys);
        assert!(
            whole.rescaled.iter().all(|(_, v)| matches!(v, Verdict::Unsat { .. })),
            "the WHOLE system never resolves -- the S1 pair contradicts at every pin",
        );
        let over_iis = scale_diagnostics(&System { rows: iis.kept, ..sys.clone() });
        assert!(
            over_iis.rescaled.iter().any(|(pin, v)| *pin == 3 && matches!(v, Verdict::Sat { .. })),
            "the IIS resolves at w_vp = 3, which is the finding the whole-system read loses",
        );
        assert!(
            render_system(&sys).contains("early-rescale at w_vp = 3: SAT"),
            "and the artifact prints the IIS's answer, not the whole system's",
        );
    }

    /// The node-budget arm: an exhausted budget is INCONCLUSIVE, never a
    /// verdict.
    #[test]
    fn a_spent_node_budget_is_inconclusive_and_never_unsat() {
        // `2 S1 + 2 S2` must equal 1, which no integer pair reaches while the
        // RELAXATION is comfortable at (1/4, 1/4). Bound propagation shaves the
        // box without emptying it, so the search has to branch — which is what
        // gives a starved budget something to run out of.
        let sys = toy(
            StageEndpoint::Early,
            &[
                ("twice S1 plus twice S2, at least 1", &[(col("S1"), 2), (col("S2"), 2)], 1),
                ("twice S1 plus twice S2, at most 1", &[(col("S1"), -2), (col("S2"), -2)], -1),
            ],
            BOX_LIMIT,
        );
        let columns = active_columns(&sys.rows);
        match integer_search(&sys, &columns, 1) {
            Verdict::Inconclusive { reason } => {
                assert!(reason.contains("budget of 1 nodes"), "{reason}");
                assert!(reason.contains("not infeasibility"), "{reason}");
            }
            other => panic!("a one-node budget cannot decide this; got {}", other.name()),
        }
        // With the registered budget it reaches a real answer, so the arm above
        // is a starved search rather than an undecidable system.
        let Verdict::Unsat { certificate: Certificate::Enumeration { points, .. } } = decide(&sys)
        else {
            panic!("relaxation-feasible and integer-infeasible over 2 columns: the enumeration \
                    re-derivation decides it");
        };
        assert_eq!(points, (2 * BOX_LIMIT as u128 + 1).pow(2));
    }

    // -- the drop-one implication read --------------------------------------

    #[test]
    fn a_row_the_others_already_force_is_reported_as_implied() {
        let sys = toy(
            StageEndpoint::Early,
            &[
                ("S1 at least 5", &[(col("S1"), 1)], 5),
                ("S1 at least 1", &[(col("S1"), 1)], 1),
                ("E1 at least 1", &[(col("E1"), 1)], 1),
            ],
            BOX_LIMIT,
        );
        let report = implied_by_the_rest(&sys);
        assert_eq!(
            report.implied,
            vec!["S1 at least 1".to_string()],
            "`>= 5` forces `>= 1`, and nothing forces the other two",
        );
        assert_eq!(
            report.independent,
            vec!["S1 at least 5".to_string(), "E1 at least 1".to_string()],
        );
        assert!(report.undecided.is_empty());
    }

    // -- the real path, exercised without the frozen thirty-four -------------

    /// **DEFENCE 3 — the dispatcher's arms checked against a reference that is
    /// not a copy of them.**
    ///
    /// [`candidate_dispatch`]'s doc sets out why the other two defences are not
    /// enough on their own: the `expected_nonzero` refusal cannot see a swap
    /// between two features both listed on the same pair, and a restatement test
    /// cannot see a swap made identically in the restatement. This cell has
    /// neither weakness, because its reference is
    /// `CandidateFeature::name()` — eighteen snake_case strings that live in the
    /// FROZEN package file, feed the digest and the report columns, and are
    /// exactly the helper method names.
    ///
    /// It reads this file's own source (the same `include_str!` the identity
    /// hash uses), extracts `candidate_dispatch`'s match arms, and checks that
    /// the arm for each variant calls the method that variant is named after. A
    /// swap between any two arms makes at least one of them call a method whose
    /// name is not its variant's, and no amount of matching mistakes elsewhere
    /// can hide that.
    #[test]
    fn every_dispatcher_arm_calls_the_helper_its_variant_is_named_after() {
        let source = include_str!("solver.rs");
        let start = source
            .find("\nfn candidate_dispatch(")
            .expect("the dispatcher is defined in this file");
        let body = &source[start..];
        let end = body.find("\n}\n").expect("the dispatcher's body closes");
        let body = &body[..end];

        let mut checked = 0usize;
        for feature in CandidateFeature::ALL {
            let arm = format!("CandidateFeature::{feature:?} =>");
            let at = body
                .find(&arm)
                .unwrap_or_else(|| panic!("{feature:?} has no arm in the dispatcher"));
            // Bound the scan to this arm: the next variant's pattern, or the
            // end of the body. Without the bound a MISSING call would silently
            // borrow the next arm's.
            let rest = &body[at + arm.len()..];
            let limit = rest.find("CandidateFeature::").unwrap_or(rest.len());
            let arm_text = &rest[..limit];
            let call = arm_text
                .find("s.")
                .map(|i| &arm_text[i + 2..])
                .and_then(|t| t.split(['(', ' ', '\n']).next())
                .unwrap_or_else(|| panic!("{feature:?}'s arm calls no ScoreStructure helper"));
            assert_eq!(
                call,
                feature.name(),
                "{feature:?} dispatches to `{call}` but is named `{}`. Either this is a SWAPPED \
                 ARM -- the defect this cell exists for, which neither the expected-nonzero \
                 refusal nor the restatement test is guaranteed to see -- or eval.rs renamed a \
                 helper away from its CandidateFeature name, in which case the two have to move \
                 together.",
                feature.name(),
            );
            checked += 1;
        }
        assert_eq!(checked, CandidateFeature::COUNT, "every arm was reached");
    }

    /// **DEFENCE 2** — the 18 arms against the same helpers, restated from the
    /// helper NAMES rather than from the dispatcher.
    ///
    /// This is the value-level companion to the source-level pin above: that one
    /// proves each arm NAMES the right helper, this one proves the call actually
    /// returns what that helper returns for a real seat of a real view. Its own
    /// limit is stated in [`candidate_dispatch`]'s doc — a restatement cannot
    /// catch a swap made identically in the restatement, which is why the
    /// source-level pin exists beside it.
    ///
    /// Coverage is exhaustive as a SET, not merely eighteen entries long: a
    /// table that repeated one variant and omitted another would be the right
    /// length and would leave a column unchecked.
    ///
    /// A plain seeded game, NOT a registered fixture: §5.4's freeze-before-solve
    /// clause keeps the frozen package out of this batch.
    #[test]
    fn the_candidate_dispatcher_reaches_the_helper_each_label_names() {
        let rules = Rules::standard();
        let state = plain_two_player_state(&rules);
        let view = yokohama_core::observe(&state, state.current_player).expect("observable");
        let me = view
            .players
            .iter()
            .find(|p| p.id == state.current_player)
            .expect("the mover has a row");
        let s = ScoreStructure::derive(&view, &rules);
        let expected: [(CandidateFeature, i64); 18] = [
            (CandidateFeature::ProductionAccess, s.production_access(&view, me)),
            (CandidateFeature::TrackAccess, s.track_access(&view, me)),
            (CandidateFeature::AssistantAreasDistinct, s.assistant_areas_distinct(me)),
            (CandidateFeature::PresidentLocalAccess, s.president_local_access(me)),
            (CandidateFeature::BankAccess, s.bank_access(&view, me)),
            (CandidateFeature::EmploymentAgencyAccess, s.employment_agency_access(&view, me)),
            (CandidateFeature::PresidentReachableAreas, s.president_reachable_areas(me)),
            (CandidateFeature::MoneyStock, s.money_stock(me)),
            (CandidateFeature::TradeGoodsStock, s.trade_goods_stock(me)),
            (CandidateFeature::BuildingTokensReady, s.building_tokens_ready(me)),
            (CandidateFeature::HeldOrderValue, s.held_order_value(me, &rules)),
            (CandidateFeature::OrderMatchedGoods, s.order_matched_goods(me, &rules)),
            (CandidateFeature::ExecutableOrders, s.executable_orders(me, &rules)),
            (CandidateFeature::FlagIconMarginal, s.flag_icon_marginal(me, &rules)),
            (CandidateFeature::HeldOrderIcons, s.held_order_icons(me, &rules)),
            (CandidateFeature::LabTakeable, s.lab_takeable(&view, me)),
            (CandidateFeature::PortTakeable, s.port_takeable(&view, me, &rules)),
            (CandidateFeature::MatchedProductionAccess, s.matched_production_access(&view, me, &rules)),
        ];
        // EXHAUSTIVE AS A SET, not merely the right length: a table repeating
        // one variant and omitting another would be eighteen entries long and
        // would leave a column unchecked.
        let mut restated: Vec<CandidateFeature> = expected.iter().map(|(f, _)| *f).collect();
        restated.sort_unstable();
        let mut all = CandidateFeature::ALL.to_vec();
        all.sort_unstable();
        assert_eq!(restated, all, "every column is restated exactly once");

        for (feature, want) in expected {
            assert_eq!(
                candidate_dispatch(&s, &view, me, feature, &rules),
                want,
                "{} dispatches to the helper its label names",
                feature.label(),
            );
        }
    }

    #[test]
    fn the_relative_vector_is_mover_minus_opponent_and_refuses_an_absent_seat() {
        let rules = Rules::standard();
        let state = plain_two_player_state(&rules);
        let view = yokohama_core::observe(&state, state.current_player).expect("observable");
        let v = relative_vector(&view, state.current_player, &rules).expect("the mover is seated");
        assert_eq!(v.len(), AXIS_WIDTH);
        assert!(
            relative_vector(&view, PlayerId(7), &rules).is_err(),
            "an absent seat is a refusal, never a zero vector that collides with everything",
        );
    }

    /// The extraction's refusals, on SYNTHETIC signatures — no fixture built.
    #[test]
    fn the_extraction_refuses_an_unlisted_nonzero_row() {
        let pair = REGISTERED.iter().find(|p| p.id == "3(c)").expect("registered");
        let carrier = col("S5"); // 3(c)'s registered expected-nonzero row.
        let stage = StageEndpoint::Late;

        let mut greater = vec![0i64; AXIS_WIDTH];
        greater[carrier] = 2;
        let lesser = vec![0i64; AXIS_WIDTH];
        let row = row_from_signatures(pair, stage, &greater, &lesser).expect("a listed row only");
        assert_eq!(row.coef[carrier], 2);
        assert_eq!(row.rhs, 40, "3(c)'s registered late margin");
        assert_eq!(row.kind, RowKind::Pair { id: "3(c)".to_string(), order: 11 });

        let mut stray = greater.clone();
        stray[col("X1")] = 1;
        let refusal = row_from_signatures(pair, stage, &stray, &lesser)
            .expect_err("an unlisted nonzero row is a fixture defect");
        assert!(refusal.contains("X1"), "the refusal names the row it saw: {refusal}");
    }

    #[test]
    fn the_extraction_refuses_an_identically_zero_row() {
        let pair = REGISTERED.iter().find(|p| p.id == "3(c)").expect("registered");
        let zero = vec![0i64; AXIS_WIDTH];
        let refusal = row_from_signatures(pair, StageEndpoint::Late, &zero, &zero)
            .expect_err("a zero row against a positive margin is unsatisfiable by construction");
        assert!(refusal.contains("identically zero"), "{refusal}");
    }

    /// The assembly, on synthetic rows: deletion order, the standing rows, the
    /// pin and the box.
    #[test]
    fn assembly_puts_the_pair_rows_in_document_order_then_the_hand_orderings() {
        let mut coef = vec![0i64; AXIS_WIDTH];
        coef[col("S1")] = 1;
        let synthetic: Vec<Row> = [("4(a)", 13), ("1(a)", 0), ("3(c)", 11)]
            .into_iter()
            .map(|(id, order)| Row {
                label: format!("pair {id} early"),
                kind: RowKind::Pair { id: id.to_string(), order },
                coef: coef.clone(),
                rhs: 1,
                margin_vp: None,
            })
            .collect();
        let sys = assemble(StageEndpoint::Early, synthetic, BOX_LIMIT);
        let labels: Vec<&str> = sys.rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(
            labels,
            vec![
                "pair 1(a) early",
                "pair 3(c) early",
                "pair 4(a) early",
                "hand: core > on_board",
                "hand: on_board > excess",
            ],
            "§5.2 document order, then the hand-ordering rows",
        );
        assert_eq!(sys.pin_vp, 1, "w_vp is pinned to V2's early entry");
        assert_eq!(sys.lo[Feature::VictoryPoints.index()], 1);
        assert_eq!(sys.hi[Feature::VictoryPoints.index()], 1);
        assert_eq!(sys.box_limit, BOX_LIMIT);
    }

    /// V2 itself satisfies the two standing hand-ordering rows at BOTH stages —
    /// the claim §5.4 makes when it calls them standing constraints.
    #[test]
    fn v2_satisfies_its_own_hand_ordering_at_both_stages() {
        for stage in StageEndpoint::ALL {
            let sys = assemble(stage, Vec::new(), BOX_LIMIT);
            assert_eq!(sys.violation(&sys.v2_padded()), 0, "{} stage", stage.name());
            assert!(matches!(decide(&sys), Verdict::Sat { .. }), "{} stage", stage.name());
        }
    }

    /// The real run is behind a flag, and the refusal says why.
    #[test]
    fn the_real_run_is_argument_gated_and_the_gate_names_the_clause() {
        assert!(REAL_RUN_GATE.contains("--real"));
        assert!(REAL_RUN_GATE.contains("freeze-before-solve"));
        assert!(
            USAGE.contains("solve --real"),
            "the gated command is documented rather than hidden",
        );
    }

    /// The whole artifact renders from a system — the toys prove the same code
    /// path the real run will print through.
    #[test]
    fn the_artifact_renders_every_section_the_spec_requires() {
        let sat = render_system(&toys::named("sat-reachable").expect("registered"));
        assert!(sat.contains("witness search"));
        assert!(sat.contains("slack profile"));
        assert!(sat.contains("binding K"));
        assert!(sat.contains("drop-one re-test"));
        assert!(sat.contains(SAT_READING));
        assert!(!sat.contains("deletion-minimization"), "a SAT has nothing to minimize");

        let unsat = render_system(&toys::named("unsat-fractional-margin").expect("registered"));
        assert!(unsat.contains("exact oracle         UNSAT"));
        assert!(unsat.contains("FARKAS, re-verified"));
        assert!(unsat.contains("deletion-minimization"));
        assert!(unsat.contains(IIS_READING));
        assert!(unsat.contains("box-widening at |w| <= 128"));
        assert!(unsat.contains("early-rescale at w_vp = 3: SAT"));
        assert!(unsat.contains("INTEGER GRANULARITY"));
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

    // -- R3: the certified exact simultaneous per-stage projection -----------

    /// Build a repair problem over a toy system at LATE, with an explicit start.
    fn toy_problem(rows: &[ToyRow], edits: &[(usize, i64)]) -> RepairProblem {
        let sys = toy(StageEndpoint::Late, rows, BOX_LIMIT);
        let mut w0 = sys.v2_padded();
        for (slot, value) in edits {
            w0[*slot] = *value;
        }
        RepairProblem::new(StageEndpoint::Late, sys, Vec::new(), "toy-start", w0)
            .expect("the toy start is admissible")
    }

    fn repaired(rows: &[ToyRow], edits: &[(usize, i64)]) -> Box<RepairReport> {
        match repair(&toy_problem(rows, edits)) {
            Repair::Repaired(report) => report,
            Repair::Infeasible { .. } => panic!("this system is repairable"),
            Repair::Inconclusive { reason, .. } => panic!("the oracle did not decide: {reason}"),
        }
    }

    /// **THE WITNESSED 3(c)/11(b) COUPLING — the cycle that killed row-local
    /// repair, as the engine's own regression test** (charter:129–141, §7's T3
    /// bullet).
    ///
    /// The two live rows at LATE, derived rather than trusted:
    ///
    /// * **3(c) BANK ACCESS ≻ POWER-DEAD** is MARGINED `late: Some(40)` with
    ///   `carrier` and `expected_nonzero` both the single entry `S5` — "the pair
    ///   moves S5 and NOTHING else on the axis" — so the row is `2·w_S5 ≥ 40`.
    ///   The `Δv = 2` is read off the banked artifact
    ///   (`docs/temp/step6-artifacts/v3l-final-artifact.txt:304`, "kept row late
    ///   3(c) … value 40 vs margin 40" at that artifact's `w_S5 = 20`).
    /// * **11(b) ACTED-AT-BANK ≻ ACTED-AT-COPPER** declares `MarginClass::
    ///   DirectionOnly`, so §5.1 prices it **1, not 2** — the T3 brief's stated
    ///   margin was 2 and the bytes say otherwise; the optimum is unchanged
    ///   because `2(w_E1 − w_S5) ≥ 1` and `≥ 2` have the same integer solutions.
    ///   Its own note registers the carriers: "Expected nonzero: E1 +2, S5 −2",
    ///   confirmed by the artifact's `value 2` at `w_E1 = 21, w_S5 = 20`
    ///   (artifact:315).
    ///
    /// **The optimum is unique and hand-derivable.** `2·w_S5 ≥ 40` forces
    /// `w_S5 ≥ 20`; `2(w_E1 − w_S5) ≥ 1` over the integers forces
    /// `w_E1 ≥ w_S5 + 1 ≥ 21`; both coordinates are therefore positive, so the
    /// displacement from `w0 = 0` is `w_S5 + w_E1 ≥ 41` with equality only at
    /// `(20, 21)`.
    #[test]
    fn the_three_c_eleven_b_coupling_repairs_to_the_unique_lexicographic_optimum() {
        assert_eq!(margin_of(&PAIR_3C, StageEndpoint::Late), Ok(40), "3(c) is MARGINED late 40");
        assert_eq!(margin_of(&PAIR_11B, StageEndpoint::Late), Ok(1), "11(b) is DIRECTION-ONLY");

        let (s5, e1) = (col("S5"), col("E1"));
        let report = repaired(
            &[
                ("pair 3(c) late", &[(s5, 2)], 40),
                ("pair 11(b) late", &[(e1, 2), (s5, -2)], 1),
            ],
            &[],
        );

        assert_eq!(report.w0[s5], 0, "V2-padded leaves both candidates at zero");
        assert_eq!(report.w0[e1], 0);
        assert_eq!((report.w[s5], report.w[e1]), (20, 21), "the unique optimum");
        assert_eq!(report.displacement, 41);
        assert_eq!(report.changed, vec![s5, e1], "two coordinates, in listed order");
    }

    /// **THE PROCEDURE THE CHARTER REJECTED, EXECUTED.** Sequential row-local
    /// carrier repair on the same two rows does not converge — it oscillates.
    ///
    /// Both rows register the SAME carrier (`PAIR_3C.carrier` and
    /// `PAIR_11B.carrier` are each `S5`), and they want it in opposite
    /// directions: 3(c) prices S5 up to 20, and 11(b) — whose S5 coefficient is
    /// −2 — then prices the same coordinate down to −1. The state repeats, and
    /// neither state in the cycle satisfies both rows. Executed rather than
    /// argued: the broken procedure is fifteen lines, so there is no reason to
    /// leave the charter's grounds as prose.
    #[test]
    fn sequential_row_local_carrier_repair_oscillates_on_the_witnessed_input() {
        let bank = FeatureId::Candidate(CandidateFeature::BankAccess);
        assert_eq!(PAIR_3C.carrier, Some(bank), "3(c)'s registered carrier");
        assert_eq!(PAIR_11B.carrier, Some(bank), "and 11(b)'s is the same coordinate");

        let (s5, e1) = (col("S5"), col("E1"));
        let sys = toy(
            StageEndpoint::Late,
            &[
                ("pair 3(c) late", &[(s5, 2)], 40),
                ("pair 11(b) late", &[(e1, 2), (s5, -2)], 1),
            ],
            BOX_LIMIT,
        );
        let mut w = sys.v2_padded();
        let mut seen = vec![w[s5]];
        for _ in 0..8 {
            for row in &sys.rows {
                let slack = System::slack_of(row, &w);
                if slack >= 0 {
                    continue;
                }
                // The row-local move: shift the CARRIER by the least integer
                // that satisfies THIS row alone, reading no other row. The
                // rounding follows the coefficient's sign, so each move is the
                // smallest one that works and the procedure is being given its
                // best case rather than a strawman.
                let a = i128::from(row.coef[s5]);
                let step = if a > 0 { ceil_div(-slack, a) } else { floor_div(-slack, a) };
                w[s5] += step as i64;
                assert!(System::slack_of(row, &w) >= 0, "the row-local move satisfies its own row");
                seen.push(w[s5]);
            }
        }
        assert_eq!(&seen[1..5], &[20, -1, 20, -1], "it oscillates: {seen:?}");
        assert_eq!(seen.last(), Some(&-1), "and it is still oscillating when the loop gives up");
        for value in [20i64, -1] {
            let mut probe = sys.v2_padded();
            probe[s5] = value;
            assert!(sys.violation(&probe) > 0, "neither state in the cycle satisfies both rows");
        }
        // The engine, on the same two rows, converges — to a point the row-local
        // procedure never reaches because it never moves E1 at all.
        assert_eq!(repaired(
            &[
                ("pair 3(c) late", &[(s5, 2)], 40),
                ("pair 11(b) late", &[(e1, 2), (s5, -2)], 1),
            ],
            &[],
        ).w[e1], 21);
    }

    /// **THE 0-START CLAUSE, at the shape 11(e) exhibits.** A repair never
    /// MANUFACTURES an anti-prior sign.
    ///
    /// The live exhibit is 11(e) ACTED-AT-PORT, whose carrier X2 moves
    /// `Δv = −4` there while X2's registered prior is `+`; the row is violated
    /// at the V2-padded start, so an unconstrained projection repairs it by
    /// driving X2 late NEGATIVE. This toy is that shape with the arithmetic made
    /// decisive: `w_S1 − 4·w_X2 ≥ 4` from `w0 = 0` is repaired at displacement 1
    /// by `w_X2 = −1` and at displacement 4 by `w_S1 = 4`.
    ///
    /// The test proves the constraint is what chose: the cheaper point is
    /// re-checked against the rows and the box HERE, satisfied, and still not
    /// taken.
    #[test]
    fn a_repair_never_manufactures_an_anti_prior_sign_from_a_zero_start() {
        let (s1, x2) = (col("S1"), col("X2"));
        let rows: &[ToyRow] = &[("11(e)-shaped", &[(s1, 1), (x2, -4)], 4)];
        let problem = toy_problem(rows, &[]);

        let mut cheaper = problem.w0.clone();
        cheaper[x2] = -1;
        assert_eq!(problem.sys.violation(&cheaper), 0, "the rows admit the anti-prior point");

        let report = repaired(rows, &[]);
        assert_eq!(report.w[x2], 0, "X2 stays at its zero start; the prior is +");
        assert_eq!(report.w[s1], 4, "the repair routes through the row's other mover");
        assert_eq!(report.displacement, 4, "and pays 4 rather than the unconstrained 1");
    }

    /// An OPPOSED start may move TOWARD agreement, and may not move further
    /// away.
    #[test]
    fn an_opposed_coordinate_moves_toward_agreement_and_never_further_from_it() {
        let s1 = col("S1");
        let toward = repaired(&[("S1 at least -2", &[(s1, 1)], -2)], &[(s1, -5)]);
        assert_eq!(toward.w[s1], -2, "−5 → −2 is a move toward the + prior");
        assert_eq!(toward.displacement, 3);

        let away = repair(&toy_problem(&[("S1 at most -10", &[(s1, -1)], 10)], &[(s1, -5)]));
        assert!(
            matches!(away, Repair::Infeasible { .. }),
            "moving an opposed coordinate further from agreement is REPAIR-INFEASIBLE",
        );
    }

    /// An AGREEING start never crosses to the opposed sign.
    #[test]
    fn an_agreeing_coordinate_never_crosses_to_the_opposed_sign() {
        let s1 = col("S1");
        let rows: &[ToyRow] = &[("S1 at most -3", &[(s1, -1)], 3)];
        assert!(
            matches!(repair(&toy_problem(rows, &[(s1, 5)])), Repair::Infeasible { .. }),
            "w0 agrees at +5, so w_S1 >= 0 stands and the row cannot be met",
        );
        // The same rows from an ABSENT prior repair without complaint, which is
        // what says the refusal above is the sign constraint and not the row.
        let hand = Feature::HandSize.index();
        let free = repaired(&[("hand at most -3", &[(hand, -1)], 3)], &[]);
        assert_eq!(free.w[hand], -3, "an incumbent carries no registered prior");
    }

    /// **`REPAIR-INFEASIBLE` IS TYPED AND CARRIES THE ORACLE'S CERTIFICATE** —
    /// R3: "never a silent relaxation".
    #[test]
    fn an_unrepairable_system_is_typed_infeasible_and_ships_a_verified_certificate() {
        let s1 = col("S1");
        let problem = toy_problem(&[("S1 beyond the wall", &[(s1, 1)], BOX_LIMIT + 1)], &[]);
        let Repair::Infeasible { certificate, .. } = repair(&problem) else {
            panic!("no assignment in the box satisfies this row");
        };
        match certificate {
            Certificate::Farkas { y, rhs } => {
                assert_eq!(
                    verify_farkas(&problem.sys.inequalities(), &y),
                    Ok(rhs),
                    "the shipped combination re-verifies at the trust boundary",
                );
            }
            Certificate::Enumeration { points, .. } => assert!(points <= ENUMERATION_BUDGET),
            // Reachable in principle now that B2's extension rides the
            // projection's question chain: the chain re-verifies from the base
            // system and the cuts' own parent records.
            Certificate::ChvatalGomory { cuts, .. } => {
                assert_eq!(verify_cut_chain(&problem.sys.inequalities(), &cuts), Ok(()));
            }
        }
    }

    /// **THE CERTIFICATE TRAIL**: a witness at the optimum, and an
    /// oracle-certified empty level for each lexicographically better level the
    /// projection passed over.
    #[test]
    fn the_trail_certifies_the_displacement_and_changed_count_levels_below_the_optimum() {
        let (s5, e1) = (col("S5"), col("E1"));
        let report = repaired(
            &[
                ("pair 3(c) late", &[(s5, 2)], 40),
                ("pair 11(b) late", &[(e1, 2), (s5, -2)], 1),
            ],
            &[],
        );

        let below = report.step(&format!("displacement <= {}", report.displacement - 1));
        assert!(
            matches!(below.map(|s| &s.outcome), Some(TrailOutcome::Empty(_))),
            "displacement {} is certified empty; the trail reads {:?}",
            report.displacement - 1,
            report.trail.iter().map(|s| s.question.as_str()).collect::<Vec<_>>(),
        );

        let fewer = report.step(&format!(
            "changed count <= {} at displacement {}",
            report.changed.len() - 1,
            report.displacement,
        ));
        assert!(
            matches!(fewer.map(|s| &s.outcome), Some(TrailOutcome::Empty(_) | TrailOutcome::Enumerated { .. })),
            "one fewer changed coordinate is certified empty at the optimal displacement",
        );

        let witness = report
            .trail
            .iter()
            .rev()
            .find_map(|step| match &step.outcome {
                TrailOutcome::Witnessed(w) => Some(w.clone()),
                _ => None,
            })
            .expect("the trail carries a feasibility witness");
        assert_eq!(witness, report.w, "and the witness IS the returned vector");
        assert_eq!(report.violation, 0, "re-checked against every row");
    }

    /// **EVERY CHANGED COORDINATE IS RECORDED WITH THE ROWS THAT PRICED IT** —
    /// R3's last sentence.
    ///
    /// Two readings are recorded because they differ here and only one of them
    /// is informative. `tight` is slack-0 at the optimum, the T3 brief's own
    /// gloss; `pricing` is the row that would be VIOLATED if this coordinate
    /// alone returned to its start, which is what "priced it" means. 11(b) shows
    /// the gap: its margin is 1 against even coefficients, so it can never sit
    /// at slack 0 — yet it is the whole reason E1 moved.
    #[test]
    fn every_changed_coordinate_is_recorded_with_the_rows_that_priced_it() {
        let (s5, e1) = (col("S5"), col("E1"));
        let report = repaired(
            &[
                ("pair 3(c) late", &[(s5, 2)], 40),
                ("pair 11(b) late", &[(e1, 2), (s5, -2)], 1),
            ],
            &[],
        );
        assert_eq!(report.pricing(s5), vec!["pair 3(c) late".to_string()]);
        assert_eq!(report.pricing(e1), vec!["pair 11(b) late".to_string()]);
        assert_eq!(report.tight(s5), vec!["pair 3(c) late".to_string()]);
        assert!(report.tight(e1).is_empty(), "11(b) cannot reach slack 0 at margin 1");
    }

    /// A coordinate NO ROW READS is pinned at the draft — the reduction
    /// [`RepairProblem::free`] documents, exercised.
    ///
    /// Moving it could only add displacement and add a changed coordinate, so no
    /// optimum moves it; the projection does not need to ask, and the point is
    /// that it does not ask WRONGLY either — the coordinate keeps the draft's
    /// value rather than collapsing to zero.
    #[test]
    fn a_coordinate_no_row_reads_stays_at_the_draft() {
        let (s1, s2) = (col("S1"), col("S2"));
        let report = repaired(&[("S1 up", &[(s1, 1)], 3)], &[(s2, 5)]);
        assert_eq!(report.w[s2], 5, "no row reads S2, so the projection leaves it where it was");
        assert_eq!(report.w[s1], 3);
        assert_eq!(report.displacement, 3, "and S2 contributes nothing to the displacement");
        assert_eq!(report.changed, vec![s1]);
    }

    /// **THE THIRD TIE-BREAK, REGISTERED AND EXERCISED.** `w_S1 + w_S2 ≥ 2` from
    /// zero has optima `(2, 0)`, `(1, 1)` and `(0, 2)` at displacement 2; the
    /// changed-count level cuts `(1, 1)`; the canonical order takes the
    /// listed-earlier support.
    #[test]
    fn the_canonical_tie_break_takes_the_listed_earlier_support() {
        let (s1, s2) = (col("S1"), col("S2"));
        let report = repaired(&[("either mover", &[(s1, 1), (s2, 1)], 2)], &[]);
        assert_eq!(report.displacement, 2);
        assert_eq!(report.changed, vec![s1], "one coordinate, and it is the earlier one");
        assert_eq!((report.w[s1], report.w[s2]), (2, 0));
    }

    /// A start the registered system does not describe is a TYPED refusal, not a
    /// repair — R4's malformed-identity discipline, at the projection's door.
    #[test]
    fn a_malformed_start_vector_is_refused_by_name() {
        let make = |edit: (usize, i64)| {
            let sys = toy(StageEndpoint::Late, &[("S1 up", &[(col("S1"), 1)], 1)], BOX_LIMIT);
            let mut w0 = sys.v2_padded();
            w0[edit.0] = edit.1;
            RepairProblem::new(StageEndpoint::Late, sys, Vec::new(), "malformed", w0)
        };
        let refusal = |edit: (usize, i64)| match make(edit) {
            Ok(_) => panic!("the registered system does not describe this start"),
            Err(e) => e,
        };
        let broken_pin = refusal((Feature::VictoryPoints.index(), 7));
        assert!(broken_pin.contains("w_vp"), "{broken_pin}");
        let outside = refusal((col("S1"), BOX_LIMIT + 1));
        assert!(outside.contains("box"), "{outside}");
        let removed = refusal((col("E3"), 3));
        assert!(removed.contains("E3"), "{removed}");
    }

    /// The projection keeps a row iff every candidate column it carries survives
    /// the registered removal set, and NAMES what it dropped.
    #[test]
    fn the_projection_drops_exactly_the_rows_a_removed_feature_carries() {
        let mut carries_o3 = vec![0i64; AXIS_WIDTH];
        carries_o3[col("S1")] = 1;
        carries_o3[col("O3")] = -1;
        let mut survives = vec![0i64; AXIS_WIDTH];
        survives[col("S1")] = 1;
        let rows: Vec<Row> = [("keeps", survives), ("drops", carries_o3)]
            .into_iter()
            .map(|(label, coef)| Row {
                label: label.to_string(),
                kind: RowKind::Pair { id: label.to_string(), order: 0 },
                coef,
                rhs: 1,
                margin_vp: None,
            })
            .collect();
        let (kept, dropped) = project_pair_rows(rows);
        assert_eq!(kept.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(), vec!["keeps"]);
        assert_eq!(dropped, vec!["drops".to_string()]);
    }

    /// **THE CROSS-ENCODING PIN.** The solver-side projected system IS the
    /// system `eval_probe`'s step-9 structural checker reads and the step-6
    /// record froze.
    ///
    /// The banked artifact `docs/temp/step6-artifacts/v3l-final-artifact.txt`
    /// prints, at lines 274–278, the surviving set, both witness vectors, and
    /// the projection's kept/dropped counts and dropped names. This test rebuilds
    /// the system from the frozen registry through the SOLVER's own extraction
    /// and requires all of it back — same dropped row names, same counts, and the
    /// artifact's own witness satisfying every kept row plus both hand orderings
    /// inside the box, which is the §5.4 SAT.
    #[test]
    fn the_projected_system_reproduces_the_banked_artifact_s_kept_and_dropped_rows() {
        let rules = Rules::standard();
        for (stage, kept_count, dropped_names, witness) in [
            (
                StageEndpoint::Early,
                17usize,
                &ARTIFACT_DROPPED_EARLY[..],
                &ARTIFACT_WITNESS_EARLY,
            ),
            (StageEndpoint::Late, 23, &ARTIFACT_DROPPED_LATE[..], &ARTIFACT_WITNESS_LATE),
        ] {
            let mut rows = Vec::new();
            for pair in REGISTERED
                .iter()
                .filter(|p| matches!(p.role, PairRole::Constraint) && p.stages.contains(&stage))
            {
                rows.push(extract_row(pair, stage, &rules).expect("the registry extracts"));
            }
            let (kept, dropped) = project_pair_rows(rows);
            assert_eq!(kept.len(), kept_count, "{} kept rows", stage.name());
            // The solver labels a pair row `pair <id> <stage>`; the artifact
            // names it `<id> <NAME>`. The ID is the half both spell.
            let dropped_ids: Vec<&str> =
                dropped.iter().filter_map(|d| d.split_whitespace().nth(1)).collect();
            assert_eq!(dropped_ids, dropped_names.to_vec(), "{} dropped rows", stage.name());

            let sys = assemble(stage, kept, BOX_LIMIT);
            let mut w = witness.to_vec();
            for candidate in REPAIR_REMOVED {
                w[FeatureId::Candidate(candidate).index()] = 0;
            }
            assert_eq!(
                sys.violation(&w),
                0,
                "{}: the artifact's witness satisfies every kept row and both orderings",
                stage.name(),
            );
            for (i, &x) in w.iter().enumerate() {
                assert!(x >= sys.lo[i] && x <= sys.hi[i], "{} coordinate {i} in box", stage.name());
            }
        }
    }

    /// **THE REAL LATE ENDPOINT IS `REPAIR-INFEASIBLE`, AND R3's OWN SIGN
    /// CONSTRAINT IS WHY** — a routed finding, made executable so it cannot
    /// quietly stop being true.
    ///
    /// The registered late rows force S4 `PresidentLocalAccess` strictly
    /// NEGATIVE, and S4's registered §3.2 prior is `+`, so the 0-start clause
    /// (`w_f·p_f ≥ min(0, w0_f·p_f)`, which floors a zero start at agreement)
    /// leaves the system empty. The combination is four rows and checks by hand:
    ///
    /// ```text
    ///   2 × 1(b) late          2·S3 − 2·S4 + 2·S7            >= 2
    ///   1 × 8 late             2·excess − 2·board − 2·S3 − 2·S4 − 2·S7 >= 1
    ///   2 × on_board > excess  2·board − 2·excess            >= 2
    ///   4 × S4 >= 0            4·S4                          >= 0
    ///   ------------------------------------------------------------
    ///                          0                             >= 5
    /// ```
    ///
    /// **The shipped certificate prints SIX nonzero multipliers, not four, and
    /// the two extra ones cancel.** They are the `w_vp` pin, which the box
    /// carries as a two-sided pair of rows: `victory_points >= 40` and
    /// `victory_points <= 40`, each at multiplier 2. Their coefficients sum to
    /// `+2·vp − 2·vp = 0` and their right-hand sides to `2·40 − 2·40 = 0`, so
    /// they contribute nothing to either column and the hand table above is the
    /// whole derivation. The count is asserted nowhere — `y`'s SUPPORT is what
    /// this test reads — but a reader checking the four rows against the real
    /// output would otherwise be two rows short and think the table wrong.
    ///
    /// Equivalently the first three rows alone say `−4·S4 ≥ 5`, so `S4 ≤ −2`
    /// over the integers. The banked artifact's own SAT witness agrees: it
    /// prices S4 at −31 at LATE (artifact:276), which is exactly the sight the
    /// 0-start clause forbids a repair from manufacturing.
    ///
    /// **The rows are not the problem and the test says so**: the same system
    /// with the sign bound off its box is satisfied by the artifact's witness,
    /// which is what makes this a finding about the CONSTRAINT rather than a
    /// broken registry.
    #[test]
    fn the_real_late_projection_is_repair_infeasible_against_s4_s_sign_bound() {
        let rules = Rules::standard();
        let mut rows = Vec::new();
        for pair in REGISTERED.iter().filter(|p| {
            matches!(p.role, PairRole::Constraint) && p.stages.contains(&StageEndpoint::Late)
        }) {
            rows.push(extract_row(pair, StageEndpoint::Late, &rules).expect("the registry extracts"));
        }
        let (kept, dropped) = project_pair_rows(rows);
        let sys = assemble(StageEndpoint::Late, kept, BOX_LIMIT);

        // Without the sign constraints the system is SATISFIABLE, and the banked
        // witness is the proof.
        let mut unconstrained = ARTIFACT_WITNESS_LATE.to_vec();
        for candidate in REPAIR_REMOVED {
            unconstrained[FeatureId::Candidate(candidate).index()] = 0;
        }
        assert_eq!(sys.violation(&unconstrained), 0, "the ROWS admit a point");
        assert_eq!(
            unconstrained[col("S4")],
            -31,
            "and that point prices S4 against its + prior",
        );

        let w0 = start_vector("v2-padded", StageEndpoint::Late).expect("a registered start");
        let problem = RepairProblem::new(StageEndpoint::Late, sys, dropped, "v2-padded", w0)
            .expect("the anchor is admissible");
        let Repair::Infeasible { certificate, .. } = repair(&problem) else {
            panic!("the 0-start clause leaves the registered late system empty");
        };
        let Certificate::Farkas { y, rhs } = &certificate else {
            panic!("this infeasibility is relaxation-level and ships FARKAS");
        };
        assert_eq!(*rhs, 5, "the combination's yb, as the hand derivation gives it");
        let all = problem.sys.inequalities();
        let leaned: Vec<&str> = all
            .iter()
            .zip(y)
            .filter(|(_, m)| **m != 0)
            .map(|(r, _)| r.label.as_str())
            .collect();
        assert!(leaned.contains(&"S4 >= 0"), "the sign bound is IN the combination: {leaned:?}");
        for label in ["pair 1(b) late", "pair 8 late", "hand: on_board > excess"] {
            assert!(leaned.contains(&label), "{label} carries it: {leaned:?}");
        }
        // The two rows the hand table omits BECAUSE they cancel — asserted so
        // the doc's "six, of which four derive it" is checkable and not prose.
        for label in ["victory_points >= 40", "victory_points <= 40"] {
            assert!(leaned.contains(&label), "the pin's {label} rides and cancels: {leaned:?}");
        }
        assert_eq!(leaned.len(), 6, "four deriving rows plus the pin's two: {leaned:?}");
    }

    /// Same inputs, byte-identical output — twice. No wall clock, no entropy;
    /// node counts and inputs only.
    #[test]
    fn the_repair_is_byte_identical_on_repeat() {
        let (s5, e1) = (col("S5"), col("E1"));
        let rows: &[ToyRow] = &[
            ("pair 3(c) late", &[(s5, 2)], 40),
            ("pair 11(b) late", &[(e1, 2), (s5, -2)], 1),
        ];
        let first = render_repair(&toy_problem(rows, &[]));
        let second = render_repair(&toy_problem(rows, &[]));
        assert_eq!(first, second, "the projection reproduces from its inputs");
        assert!(first.contains("R3 projection"), "{first}");
    }

    /// **THE HAND-OFF IS A COPY, NOT A RECONSTRUCTION.**
    ///
    /// A REPAIRED arm's next reader types the table into
    /// `eval_probe step9 structural --table <early>/<late>`, and the only other
    /// place the repaired numbers appear is the `changed coordinates` block —
    /// which prints the MOVES. Reassembling a table from moves plus a
    /// remembered draft is the T4 transcription hazard in one sentence, so the
    /// arm prints the whole table in the grammar that command parses.
    ///
    /// **The two example binaries cannot import one another**, so this cell
    /// applies `parse_step9_table`'s rules rather than calling it: thirty-four
    /// comma-separated `i64` per vector, in `FeatureId::all()` order, joined by
    /// `/`. Both orientations run — an EARLY repair and a LATE one — because
    /// which half is the repair and which is the unmoved start swaps with the
    /// endpoint, and a renderer that printed the repair twice would pass a
    /// one-orientation test.
    #[test]
    fn a_repaired_arm_prints_both_vectors_in_the_step_nine_table_grammar() {
        let vector_line = |out: &str, label: &str| -> Vec<i64> {
            let prefix = format!("  {label:<18} ");
            let line = out
                .lines()
                .find_map(|l| l.strip_prefix(prefix.as_str()))
                .unwrap_or_else(|| panic!("no {label} vector line in:\n{out}"));
            let values: Vec<i64> = line
                .split(',')
                .map(|x| x.parse::<i64>().unwrap_or_else(|e| panic!("{label} `{x}`: {e}")))
                .collect();
            assert_eq!(values.len(), AXIS_WIDTH, "{label} is not thirty-four wide: {line}");
            values
        };

        // The EARLY orientation, through the SHIPPED toy path: a registered
        // system and a registered `--start`, exactly as `run_repair_toy` builds
        // them.
        let sys = toys::named("sat-reachable").expect("a registered toy");
        let stage = sys.stage;
        assert_eq!(stage, StageEndpoint::Early, "this toy is the EARLY orientation");
        let (kept, dropped) = project_pair_rows(sys.rows.clone());
        let projected = System::new(stage, kept, sys.box_limit, sys.pin_vp);
        let w0 = start_vector("v2-padded", stage).expect("a registered start");
        let problem = RepairProblem::new(stage, projected, dropped, "v2-padded", w0)
            .expect("the anchor is admissible");
        let out = render_repair(&problem);
        let Repair::Repaired(report) = repair(&problem) else {
            panic!("`sat-reachable` is the repairable toy");
        };
        assert_eq!(vector_line(&out, "EARLY"), report.w, "the repaired endpoint is the repair");
        assert_eq!(
            vector_line(&out, "LATE"),
            start_vector("v2-padded", StageEndpoint::Late).expect("a registered start"),
            "and the other endpoint is the unmoved start, since a repair is per endpoint",
        );

        // The LATE orientation, over the witnessed 3(c)/11(b) coupling.
        let (s5, e1) = (col("S5"), col("E1"));
        let late_sys = toy(
            StageEndpoint::Late,
            &[("pair 3(c) late", &[(s5, 2)], 40), ("pair 11(b) late", &[(e1, 2), (s5, -2)], 1)],
            BOX_LIMIT,
        );
        let w0 = start_vector("v2-padded", StageEndpoint::Late).expect("a registered start");
        let late = RepairProblem::new(StageEndpoint::Late, late_sys, Vec::new(), "v2-padded", w0)
            .expect("the anchor is admissible");
        let late_out = render_repair(&late);
        let Repair::Repaired(late_report) = repair(&late) else {
            panic!("the coupling repairs");
        };
        assert_eq!(vector_line(&late_out, "LATE"), late_report.w, "the halves swap with the stage");
        assert_eq!(
            vector_line(&late_out, "EARLY"),
            start_vector("v2-padded", StageEndpoint::Early).expect("a registered start"),
        );

        // THE PASTE ITSELF. `<early>/<late>` is what the reader types, and the
        // two printed lines are its two halves in order.
        let pasted = format!(
            "{}/{}",
            vector_csv(&vector_line(&late_out, "EARLY")),
            vector_csv(&vector_line(&late_out, "LATE")),
        );
        let (early_half, late_half) = pasted.split_once('/').expect("one separator");
        assert_eq!(early_half.split(',').count(), AXIS_WIDTH);
        assert_eq!(late_half.split(',').count(), AXIS_WIDTH);
        assert!(late_half.starts_with("40,"), "the pinned w_vp rides the LATE half: {late_half}");

        // A start this file cannot spell at the OTHER endpoint prints the
        // repaired half alone and says why, rather than inventing a vector.
        let hand_built = render_repair(&toy_problem(
            &[("pair 3(c) late", &[(s5, 2)], 40)],
            &[],
        ));
        assert!(
            hand_built.contains("`toy-start` is not a registered --start spec"),
            "{hand_built}",
        );
        assert!(!hand_built.contains("  EARLY  "), "no invented EARLY vector: {hand_built}");
    }

    /// A SPENT BUDGET IS INCONCLUSIVE, never infeasibility — §5.4's own
    /// discipline, in the projection's own currency.
    #[test]
    fn a_spent_oracle_budget_is_inconclusive_and_never_infeasible() {
        let (s5, e1) = (col("S5"), col("E1"));
        let problem = toy_problem(
            &[
                ("pair 3(c) late", &[(s5, 2)], 40),
                ("pair 11(b) late", &[(e1, 2), (s5, -2)], 1),
            ],
            &[],
        );
        match repair_within(&problem, 1, REPAIR_SUPPORT_BUDGET) {
            Repair::Inconclusive { reason, .. } => {
                assert!(reason.contains("budget"), "{reason}");
                assert!(reason.contains("not infeasibility"), "{reason}");
            }
            other => panic!("a one-call budget cannot decide this, got {}", other.name()),
        }
    }

    /// The subcommand is documented rather than hidden, and the real projection
    /// is gated exactly as the real solve is.
    ///
    /// **THE EXIT CONTRACT IS DOCUMENTED AND EXERCISED.** `repair` is a
    /// REPORTING instrument with no gate in it: all three registered outcomes
    /// are answers, so all three RUN and return `Ok`, and the verdict is read
    /// off the `VERDICT` line rather than off `$?`. That is structural — the
    /// outcome is rendered into the page and never reaches the `Result` — and
    /// two registered toys with DIFFERENT outcomes walk it here, so a future
    /// arm that routed an answer through `Err` would fail this cell rather than
    /// quietly turning a routed finding into a nonzero exit. The third outcome,
    /// INCONCLUSIVE, has no toy at the shipped budgets by design; its
    /// construction is
    /// [`a_spent_oracle_budget_is_inconclusive_and_never_infeasible`] and it
    /// leaves [`render_repair`] by the same path the other two do.
    #[test]
    fn the_repair_subcommand_is_documented_and_the_real_run_is_gated() {
        assert!(USAGE.contains("repair"), "{USAGE}");
        assert!(USAGE.contains("--start"), "{USAGE}");
        assert!(REPAIR_START_SPECS.contains(&"v2-padded"));
        assert!(start_vector("nonsense", StageEndpoint::Late).is_err());

        assert!(USAGE.contains("EXIT: 0 = the projection RAN"), "{USAGE}");
        for outcome in ["REPAIRED", "REPAIR-INFEASIBLE", "INCONCLUSIVE"] {
            assert!(USAGE.contains(outcome), "the EXIT block names {outcome}: {USAGE}");
        }

        let outcome_of = |name: &str| -> &'static str {
            let sys = toys::named(name).expect("a registered toy");
            let (kept, dropped) = project_pair_rows(sys.rows.clone());
            let projected = System::new(sys.stage, kept, sys.box_limit, sys.pin_vp);
            let w0 = start_vector("v2-padded", sys.stage).expect("a registered start");
            let problem = RepairProblem::new(sys.stage, projected, dropped, "v2-padded", w0)
                .expect("the anchor is admissible");
            repair(&problem).name()
        };
        for (toy, want) in [("sat-reachable", "REPAIRED"), ("unsat-direct", "REPAIR-INFEASIBLE")] {
            assert_eq!(outcome_of(toy), want, "{toy}");
            for driver in [true, false] {
                assert!(
                    run_repair_toy(toy, "v2-padded", driver).is_ok(),
                    "{toy}: an ANSWER is a clean run (driver = {driver})",
                );
            }
        }
        // What DOES fail is the instrument being unable to run at all.
        assert!(run_repair_toy("nosuch", "v2-padded", true).is_err(), "an unknown toy cannot run");
        assert!(
            run_repair_toy("sat-reachable", "bogus", true).is_err(),
            "nor can an unknown --start",
        );
    }

    // -- B1: the certified-exception class, a driver ABOVE the projection ----

    /// **THE PRIORS ARE UNEDITED**, which is a clause of B1 itself: "`SIGN_PRIORS`
    /// stays on the books unedited — the certificate stands beside the prior,
    /// never in place of it (the D3/D6 record-the-disagreement style)".
    ///
    /// The whole point of the exception class is that it does NOT re-rule a
    /// direction, so a wave that quietly flipped one would have implemented a
    /// different ruling with the same name.
    #[test]
    fn the_registered_sign_priors_the_exception_class_reads_are_still_unedited() {
        for candidate in CandidateFeature::ALL {
            assert_eq!(
                sign_prior(candidate),
                1,
                "{} is registered `+`; B1 relaxes a CLAUSE, never a prior",
                candidate.label(),
            );
        }
    }

    /// **THE CANONICAL EXAMINATION ORDER IS THE AXIS ORDER**, so "canonical
    /// `Feature::ALL` order within that endpoint" and the loop over
    /// `CandidateFeature::ALL` are the same order rather than two orders that
    /// happen to agree today.
    #[test]
    fn the_canonical_examination_order_is_the_axis_order() {
        let indices: Vec<usize> =
            CandidateFeature::ALL.iter().map(|c| FeatureId::Candidate(*c).index()).collect();
        let mut ascending = indices.clone();
        ascending.sort_unstable();
        assert_eq!(indices, ascending, "the candidate columns walk the axis upward");
        assert_eq!(
            indices[0],
            Feature::INCUMBENT_COUNT,
            "and they start where the sixteen-feature incumbent prefix ends",
        );
    }

    /// A LATE toy problem over rows the caller names, with the V2-padded start
    /// the real run uses — so every candidate begins at 0 and every sign clause
    /// is `w_c >= 0`, exactly the shape the S4-late finding has.
    fn anchored_problem(rows: &[ToyRow]) -> RepairProblem {
        let sys = toy(StageEndpoint::Late, rows, BOX_LIMIT);
        let w0 = start_vector("v2-padded", StageEndpoint::Late).expect("a registered start");
        RepairProblem::new(StageEndpoint::Late, sys, Vec::new(), "v2-padded", w0)
            .expect("the anchor is admissible")
    }

    /// **BORN-VALIDATION (a): A SINGLY-FORCED SYSTEM, hand-built, with a
    /// derivable exception coordinate AND a derivable dose.**
    ///
    /// One row, `−w_S4 ≥ 2`, i.e. `w_S4 ≤ −2`. S4's registered prior is `+` and
    /// the anchor starts it at 0, so the sign clause floors it at `w_S4 ≥ 0` and
    /// the two cannot both hold: the RAW projection is REPAIR-INFEASIBLE. That is
    /// the S4-late shape with the arithmetic made decisive.
    ///
    /// Every claim below is hand-derived before the engine runs:
    ///
    /// * the forced coordinate is **S4** — removing S1's, S2's or S3's clause
    ///   changes nothing (no row reads them), so those probes come back still
    ///   infeasible, and S4 is the first whose relaxation is feasible;
    /// * the dose is **−2** — the least `|w − 0|` satisfying `w_S4 ≤ −2`, which
    ///   is what minimal displacement takes and is NOT the widest the box allows;
    /// * exactly **one** exception is applied, and the repair then answers.
    #[test]
    fn a_singly_forced_coordinate_is_excepted_at_the_least_certified_anti_prior_dose() {
        let s4 = col("S4");
        let rows: &[ToyRow] = &[("S4 at most -2", &[(s4, -1)], 2)];
        let problem = anchored_problem(rows);

        // The RAW projection's own answer, first: the driver exists because of
        // this verdict, so the test states it rather than assuming it.
        assert!(
            matches!(repair(&problem), Repair::Infeasible { .. }),
            "the un-excepted system is REPAIR-INFEASIBLE, which is what the driver acts on",
        );

        let driven = drive_repair(&problem);
        assert_eq!(driven.name(), "REPAIRED", "the exception restores a repairable system");
        let Driven::Repaired { exceptions, report } = driven else { unreachable!() };
        assert_eq!(exceptions.len(), 1, "ONE clause per iteration, and one iteration sufficed");

        let e = &exceptions[0];
        assert_eq!(e.candidate, CandidateFeature::PresidentLocalAccess, "S4 is the forced one");
        assert_eq!(e.clause, "S4 >= 0", "the clause removed, in the box row's own spelling");
        assert_eq!(e.stage, StageEndpoint::Late, "endpoint-local, per B1");
        assert!(
            matches!(e.infeasibility, Certificate::Farkas { .. }),
            "question (i) holds a VERIFIED certificate, not a report of one",
        );
        assert!(e.witness[s4] <= -2, "question (ii)'s witness violates the removed clause");

        // The probes prove the ORDER: every earlier candidate was asked and said
        // no, so S4 is the FIRST feasible relaxation and not merely one of them.
        let earlier: Vec<&str> = e
            .probes
            .iter()
            .take_while(|p| p.candidate != CandidateFeature::PresidentLocalAccess)
            .map(|p| p.candidate.label())
            .collect();
        assert_eq!(earlier, ["S1", "S2", "S3"], "the canonical prefix, all probed: {earlier:?}");
        assert!(
            e.probes.iter().take_while(|p| p.candidate != CandidateFeature::PresidentLocalAccess)
                .all(|p| p.answer == ProbeAnswer::StillInfeasible),
            "and every one of them came back certified infeasible",
        );
        assert_eq!(e.probes.last().map(|p| p.answer), Some(ProbeAnswer::Feasible));

        // THE DOSE, hand-derived: -2, the least certified anti-prior magnitude.
        assert_eq!(report.w[s4], -2, "the least |w - w0| that satisfies the row");
        assert_eq!(report.displacement, 2);
        assert_eq!(report.changed, vec![s4], "and nothing else moved");
    }

    /// **BORN-VALIDATION (b): A JOINT CONFLICT, hand-built.**
    ///
    /// `w_S4 ≤ −2` AND `w_S5 ≤ −2`, both against `+` priors from a 0 start.
    /// Removing S4's clause alone leaves `w_S5 ≥ 0` against `w_S5 ≤ −2`, still
    /// infeasible; removing S5's alone leaves S4's contradiction standing. No
    /// SINGLE clause's removal restores feasibility, so nothing is proven singly
    /// forced and the class does not apply.
    ///
    /// **That is the registered answer and not a shortfall.** B1 relaxes one
    /// clause per iteration against a proof; relaxing two together would be
    /// choosing rather than proving, which is exactly what the JOINT verdict
    /// routes to the owner instead of doing.
    #[test]
    fn an_infeasibility_no_single_clause_removal_cures_is_a_typed_joint_conflict() {
        let (s4, s5) = (col("S4"), col("S5"));
        let rows: &[ToyRow] =
            &[("S4 at most -2", &[(s4, -1)], 2), ("S5 at most -2", &[(s5, -1)], 2)];
        let problem = anchored_problem(rows);

        let driven = drive_repair(&problem);
        assert_eq!(driven.name(), "REPAIR-JOINT-CONFLICT", "a distinct verdict, outside the class");
        let Driven::JointConflict { exceptions, probes, .. } = driven else { unreachable!() };
        assert!(exceptions.is_empty(), "nothing was excepted on the way there");
        for candidate in [CandidateFeature::PresidentLocalAccess, CandidateFeature::BankAccess] {
            let probe = probes
                .iter()
                .find(|p| p.candidate == candidate)
                .unwrap_or_else(|| panic!("{} was probed", candidate.label()));
            assert_eq!(probe.answer, ProbeAnswer::StillInfeasible, "{}", candidate.label());
        }
        // Each half IS singly forced on its own — the conflict is that they are
        // not, together. Stated executably so the verdict reads as "no SINGLE
        // clause", never as "these rows are unrepairable in principle".
        for (row, coordinate) in
            [(("S4 at most -2", &[(s4, -1)][..], 2), s4), (("S5 at most -2", &[(s5, -1)][..], 2), s5)]
        {
            let alone = drive_repair(&anchored_problem(&[row]));
            assert_eq!(alone.name(), "REPAIRED", "each row alone IS singly forced");
            assert_eq!(alone.exceptions().len(), 1);
            let Driven::Repaired { report, .. } = alone else { unreachable!() };
            assert_eq!(report.w[coordinate], -2);
        }
    }

    /// **BORN-VALIDATION (c): THE STANDING REGRESSION FIXTURE, through the
    /// driver, unchanged.**
    ///
    /// The witnessed 3(c)/11(b) coupling repairs to the same unique optimum
    /// `(S5, E1) = (20, 21)` at displacement 41 — with ZERO exceptions, because
    /// the projection never returns REPAIR-INFEASIBLE on it. The driver is a
    /// layer above, and a layer above must be invisible where it has nothing to
    /// do.
    #[test]
    fn the_three_c_eleven_b_fixture_repairs_through_the_driver_with_zero_exceptions() {
        let (s5, e1) = (col("S5"), col("E1"));
        let rows: &[ToyRow] = &[
            ("pair 3(c) late", &[(s5, 2)], 40),
            ("pair 11(b) late", &[(e1, 2), (s5, -2)], 1),
        ];
        let driven = drive_repair(&anchored_problem(rows));
        assert_eq!(driven.name(), "REPAIRED");
        assert!(driven.exceptions().is_empty(), "nothing to except: the system is satisfiable");
        let Driven::Repaired { report, .. } = driven else { unreachable!() };
        assert_eq!((report.w[s5], report.w[e1]), (20, 21), "the unique optimum, unchanged");
        assert_eq!(report.displacement, 41);
        assert_eq!(report.changed, vec![s5, e1]);
    }

    /// **THE RAW UN-EXCEPTED PROJECTION STAYS REACHABLE**, from the code and from
    /// the command line, and still says REPAIR-INFEASIBLE where it did.
    #[test]
    fn the_raw_un_excepted_projection_stays_reachable_beside_the_driver() {
        let s4 = col("S4");
        let rows: &[ToyRow] = &[("S4 at most -2", &[(s4, -1)], 2)];
        let problem = anchored_problem(rows);
        assert_eq!(repair(&problem).name(), "REPAIR-INFEASIBLE", "`repair` is untouched");
        assert_eq!(drive_repair(&problem).name(), "REPAIRED", "the driver is the layer above");

        let raw = render_repair(&problem);
        assert!(raw.contains("VERDICT              REPAIR-INFEASIBLE"), "{raw}");
        assert!(!raw.contains("exception driver"), "the raw page names no driver: {raw}");
        assert!(USAGE.contains("--no-exceptions"), "and the opt-out is documented: {USAGE}");
        assert!(
            render_problem(&problem, false) == raw,
            "`--no-exceptions` is the raw page, byte for byte",
        );
    }

    /// **EVERY PRINTED EXCEPTION CITES B1 AND CARRIES BOTH CERTIFICATES**, plus
    /// the dose and the rows that priced it — the whole of what B1 requires
    /// recorded.
    #[test]
    fn a_printed_exception_cites_the_batch_row_and_carries_both_certificates() {
        let s4 = col("S4");
        let rows: &[ToyRow] = &[("S4 at most -2", &[(s4, -1)], 2)];
        let out = render_driven(&anchored_problem(rows));

        assert!(out.contains("2026-08-25"), "the batch row is dated in the page: {out}");
        assert!(out.contains("THE T4-ENTRY BATCH IS RULED"), "{out}");
        assert!(out.contains("certified exception 1 of 1: S4"), "{out}");
        assert!(out.contains("clause off   `S4 >= 0`"), "{out}");
        assert!(out.contains("(i) infeasible  FARKAS"), "{out}");
        assert!(out.contains("(ii) feasible   witness re-checked"), "{out}");
        assert!(out.contains("UNEDITED"), "the prior is stated to be untouched: {out}");
        assert!(out.contains("exception doses"), "{out}");
        assert!(out.contains("priced by  [\"S4 at most -2\"]"), "the binding row is named: {out}");
        assert!(
            out.contains("under 1 certified exception(s) (B1)"),
            "the VERDICT line says how many: {out}",
        );
        // The probes that said no are printed too: a claim about an ORDER is
        // only checkable when the order is on the page.
        for label in ["S1", "S2", "S3"] {
            assert!(
                out.contains(&format!("`{label} >= 0`  still infeasible")),
                "{label} was probed and refused: {out}",
            );
        }
    }

    /// A JOINT conflict's page names the verdict, the certificate and every
    /// clause it probed — and never prints an exception it did not apply.
    #[test]
    fn a_joint_conflict_page_names_the_verdict_and_every_clause_it_probed() {
        let (s4, s5) = (col("S4"), col("S5"));
        let out = render_driven(&anchored_problem(&[
            ("S4 at most -2", &[(s4, -1)], 2),
            ("S5 at most -2", &[(s5, -1)], 2),
        ]));
        assert!(out.contains("VERDICT              REPAIR-JOINT-CONFLICT"), "{out}");
        assert!(out.contains("routed to the OWNER"), "{out}");
        assert!(out.contains("exceptions applied   0"), "{out}");
        assert!(!out.contains("certified exception 1"), "nothing was applied: {out}");

        // **THE JOINT STRUCTURE, said plainly.** Removing S4's clause leaves the
        // contradiction standing on S5's and the reverse; every other clause is
        // irrelevant to both, so its removal leans on BOTH survivors. Derived
        // from the two rows before the engine ran.
        assert!(out.contains("joint structure"), "{out}");
        assert!(out.contains("without `S4 >= 0`          still leans on [\"S5\"]"), "{out}");
        assert!(out.contains("without `S5 >= 0`          still leans on [\"S4\"]"), "{out}");
        assert!(
            out.contains("without `S1 >= 0`          still leans on [\"S4\", \"S5\"]")
                || out.contains("without `S1 >= 0`          still leans on [\"S4\"]"),
            "an irrelevant clause's residual names whichever contradiction the LP certified: \
             {out}",
        );
    }

    /// Same inputs, byte-identical page — the driver draws no entropy and reads
    /// no clock either.
    #[test]
    fn the_driven_repair_is_byte_identical_on_repeat() {
        let s4 = col("S4");
        let rows: &[ToyRow] = &[("S4 at most -2", &[(s4, -1)], 2)];
        let first = render_driven(&anchored_problem(rows));
        let second = render_driven(&anchored_problem(rows));
        assert_eq!(first, second, "the driver reproduces from its inputs");
    }

    /// The real registered LATE problem at R9's anchor, built the way
    /// `run_repair_real` builds it.
    fn real_late_problem() -> RepairProblem {
        let rules = Rules::standard();
        let mut rows = Vec::new();
        for pair in REGISTERED.iter().filter(|p| {
            matches!(p.role, PairRole::Constraint) && p.stages.contains(&StageEndpoint::Late)
        }) {
            rows.push(extract_row(pair, StageEndpoint::Late, &rules).expect("the registry extracts"));
        }
        let (kept, dropped) = project_pair_rows(rows);
        let sys = assemble(StageEndpoint::Late, kept, BOX_LIMIT);
        let w0 = start_vector("v2-padded", StageEndpoint::Late).expect("a registered start");
        RepairProblem::new(StageEndpoint::Late, sys, dropped, "v2-padded", w0)
            .expect("the anchor is admissible")
    }

    /// **THE REAL LATE ENDPOINT IS A JOINT CONFLICT — the engine's answer, and
    /// it is NOT the S4 exception the batch row expected.**
    ///
    /// The batch's B1 grounds anticipated `S4` at dose `−2`, with the next
    /// forced coordinate reachable by ITERATION. The engine's typed answer is
    /// that no coordinate is singly forced at all, and it holds a verified
    /// certificate for every one of the fourteen refusals. Two contradictions
    /// stand INDEPENDENTLY in the registered late rows, and each survives the
    /// other's clause being removed:
    ///
    /// * **(A) `w_S4 ≤ −2`** — T3's finding, unchanged: `2×1(b) + 1×8 + 2×(hand:
    ///   on_board > excess)` sums every column to zero except `S4` at `−4`, with
    ///   `y·b = 5`, so `−4·w_S4 ≥ 5` and `w_S4 ≤ −5/4 → ≤ −2` over the integers.
    /// * **(B) `w_S3 ≤ −4`** — NEW here, and PROVEN rather than observed:
    ///   `2×1(b) + 4×1(c) + 3×8 + 6×(hand: on_board > excess)` sums every column
    ///   to zero except `S3` at `−4`, with `y·b = 15`, so `−4·w_S3 ≥ 15` and
    ///   `w_S3 ≤ −15/4 → ≤ −4`. The ledger had S3-late as OBSERVED negative at
    ///   the §8:975 witness; this is a certificate.
    ///
    /// (B) uses no S4 clause and (A) uses no S3 clause, so removing either alone
    /// leaves the other standing — which is exactly B1's JOINT conflict, and
    /// exactly why the class declines to act. The arithmetic is re-derived here
    /// from the certificate's own multipliers so the claim is checkable without
    /// re-running anything.
    #[test]
    fn the_real_late_endpoint_is_a_certified_joint_conflict_over_s4_and_s3() {
        let problem = real_late_problem();
        let driven = drive_repair(&problem);
        let Driven::JointConflict { exceptions, probes, .. } = driven else {
            panic!(
                "the engine's answer is the record: the real late endpoint drives to \
                 REPAIR-JOINT-CONFLICT, and a change to that must be re-derived here",
            );
        };
        assert!(exceptions.is_empty(), "no coordinate was proven singly forced");
        assert_eq!(probes.len(), 14, "every surviving candidate's clause was probed");
        assert!(
            probes.iter().all(|p| p.answer == ProbeAnswer::StillInfeasible),
            "and every one came back CERTIFIED infeasible, never undecided",
        );

        // (A) and (B), re-derived from the shipped multipliers rather than
        // quoted: the two `y·b` values are the two contradictions.
        let s4 = probes
            .iter()
            .find(|p| p.candidate == CandidateFeature::PresidentLocalAccess)
            .expect("S4 was probed");
        assert!(
            s4.leaned_on.iter().any(|l| l.starts_with("S3 >= 0 x")),
            "with S4 free the contradiction moves onto S3's clause: {:?}",
            s4.leaned_on,
        );
        for label in ["pair 1(b) late x2", "pair 1(c) late x4", "pair 8 late x3"] {
            assert!(
                s4.leaned_on.iter().any(|l| l.starts_with(label)),
                "{label} carries the S3 residual: {:?}",
                s4.leaned_on,
            );
        }
        let s3 = probes
            .iter()
            .find(|p| p.candidate == CandidateFeature::AssistantAreasDistinct)
            .expect("S3 was probed");
        assert!(
            s3.leaned_on.iter().any(|l| l.starts_with("S4 >= 0 x")),
            "and with S3 free it stands on S4's: {:?}",
            s3.leaned_on,
        );
    }

    /// **THE ENGINE'S TWO LATE CONTRADICTIONS, DERIVED FROM THE ROW DATA** —
    /// arithmetic, not a re-run.
    ///
    /// Both combinations are checked column by column against the registered
    /// rows through the same [`verify_farkas`] trust boundary the oracle uses,
    /// and the implied integer bounds are computed from `y·b` here. A row
    /// change that broke either derivation breaks this cell rather than quietly
    /// re-pointing the finding.
    #[test]
    fn the_two_late_contradictions_are_arithmetic_over_the_registered_rows() {
        let problem = real_late_problem();
        let all = problem.sys.inequalities();
        let index = |label: &str| {
            all.iter()
                .position(|r| r.label == label)
                .unwrap_or_else(|| panic!("{label} is a row of the late system"))
        };
        // (A) S4 <= -2 and (B) S3 <= -4: the multiplier sets, the coordinate
        // each one isolates, and the |coefficient| it isolates it at.
        for (name, multipliers, isolated, coefficient, yb, bound) in [
            (
                "(A) w_S4 <= -2",
                vec![("pair 1(b) late", 2i128), ("pair 8 late", 1), ("hand: on_board > excess", 2)],
                col("S4"),
                -4i128,
                5i128,
                -2i64,
            ),
            (
                "(B) w_S3 <= -4",
                vec![
                    ("pair 1(b) late", 2),
                    ("pair 1(c) late", 4),
                    ("pair 8 late", 3),
                    ("hand: on_board > excess", 6),
                ],
                col("S3"),
                -4,
                15,
                -4,
            ),
        ] {
            let mut y = vec![0i128; all.len()];
            for (label, m) in &multipliers {
                y[index(label)] = *m;
            }
            // Every column but the isolated one sums to zero, and that one sums
            // to `coefficient` — the Farkas shape with a single survivor.
            for column in 0..row_width(&all) {
                let sum: i128 =
                    all.iter().zip(&y).map(|(r, m)| i128::from(r.coef[column]) * m).sum();
                let want = if column == isolated { coefficient } else { 0 };
                assert_eq!(sum, want, "{name}: column {column}");
            }
            let rhs: i128 = all.iter().zip(&y).map(|(r, m)| i128::from(r.rhs) * m).sum();
            assert_eq!(rhs, yb, "{name}: y·b");
            // `coefficient · w >= yb` with a negative coefficient is
            // `w <= yb / coefficient`, floored onto the integers.
            assert_eq!(floor_div(rhs, coefficient) as i64, bound, "{name}: the integer bound");

            // And the clause the bound contradicts is the registered `>= 0`.
            let candidate = match FeatureId::all()[isolated] {
                FeatureId::Candidate(c) => c,
                FeatureId::Incumbent(_) => panic!("{name} isolates a candidate column"),
            };
            assert_eq!(sign_prior(candidate), 1, "{name}: the prior it opposes");
            assert_eq!(problem.clause_text(candidate), format!("{} >= 0", candidate.label()));
        }
    }

    // -- the fix wave: what a NON-FARKAS residual may not be allowed to look
    //    like, and when a FEASIBLE probe is not enough ----------------------

    /// A problem over the wide `2·w_S1 = 1` system: relaxation-feasible,
    /// integer-infeasible, and with a support too wide to enumerate — so every
    /// question about it is answered by the C–G extension and every residual
    /// certificate has the CHVATAL-GOMORY shape rather than the FARKAS one.
    ///
    /// The infeasibility has nothing to do with any sign clause, so no single
    /// removal cures it and the driver reaches a JOINT conflict whose fourteen
    /// residuals are all non-Farkas. That is the shape the fix wave is about.
    fn wide_integer_only_problem() -> RepairProblem {
        let sys = wide_integer_only_system();
        let w0 = start_vector("v2-padded", StageEndpoint::Early).expect("a registered start");
        RepairProblem::new(StageEndpoint::Early, sys, Vec::new(), "v2-padded", w0)
            .expect("the anchor is admissible")
    }

    /// **A NON-FARKAS RESIDUAL NEVER RENDERS AS AN EMPTY CLAUSE LIST.**
    ///
    /// `still leans on []` on the owner's decision page reads as "nothing is
    /// standing behind this clause" — which is the reading of a SINGLY FORCED
    /// coordinate, the exact opposite of what a refused probe means. Only a
    /// FARKAS certificate names rows, so only a Farkas residual can have its
    /// clause carriage read off it; every other shape must SAY that it was not
    /// introspected rather than answer the question with silence.
    #[test]
    fn a_non_farkas_residual_says_carriage_was_not_introspected_and_never_reads_as_empty() {
        let problem = wide_integer_only_problem();
        let out = render_driven(&problem);
        assert!(out.contains("VERDICT              REPAIR-JOINT-CONFLICT"), "{out}");
        assert!(
            !out.contains("still leans on []"),
            "an empty clause list on a non-Farkas residual reads as SINGLY FORCED: {out}",
        );
        assert!(
            out.contains("clause carriage not introspected"),
            "the page says what it did not look at: {out}",
        );
        assert!(out.contains("CHVATAL-GOMORY"), "and names the shape it is looking at: {out}");
    }

    /// The same claim at the RENDERING level, over a hand-built probe — so the
    /// cell fails on a renderer that special-cases the real system's shapes and
    /// leaves the general one wrong.
    #[test]
    fn the_joint_structure_block_names_every_non_farkas_residual_shape() {
        let problem = anchored_problem(&[("S4 at most -2", &[(col("S4"), -1)], 2)]);
        let probes: Vec<ExceptionProbe> = [
            (CandidateFeature::ProductionAccess, CertificateShape::ChvatalGomory),
            (CandidateFeature::TrackAccess, CertificateShape::Enumeration),
        ]
        .into_iter()
        .map(|(candidate, shape)| ExceptionProbe {
            candidate,
            clause: problem.clause_text(candidate),
            answer: ProbeAnswer::StillInfeasible,
            detail: "a hand-built residual".to_string(),
            leaned_on: Vec::new(),
            residual: Some(shape),
        })
        .collect();
        let rendered = render_joint_structure(&problem, &probes);
        assert!(!rendered.contains("still leans on []"), "{rendered}");
        for shape in ["CHVATAL-GOMORY", "EXHAUSTIVE ENUMERATION"] {
            assert!(rendered.contains(shape), "{shape} is named: {rendered}");
        }
        // A FARKAS residual is still read normally, so the fix narrows nothing
        // that was working.
        let farkas = vec![ExceptionProbe {
            candidate: CandidateFeature::BankAccess,
            clause: problem.clause_text(CandidateFeature::BankAccess),
            answer: ProbeAnswer::StillInfeasible,
            detail: "a hand-built Farkas residual".to_string(),
            leaned_on: vec![format!("{} x3  [S4:+1 >= 0]", problem.clause_text(
                CandidateFeature::PresidentLocalAccess,
            ))],
            residual: Some(CertificateShape::Farkas),
        }];
        assert!(
            render_joint_structure(&problem, &farkas).contains("still leans on [\"S4\"]"),
            "{}",
            render_joint_structure(&problem, &farkas),
        );
    }

    /// **A C–G CERTIFICATE'S `leans on` DOES NOT APPEAR COMPLETE.**
    ///
    /// The trail prints the inner Farkas combination's multiplier COUNT and then
    /// names only the derived cuts — which is a list that looks like the whole
    /// combination and is not. The rendering has to say the base rows are not
    /// enumerated for that shape.
    #[test]
    fn a_chvatal_gomory_certificates_leans_on_says_the_base_rows_are_not_enumerated() {
        let sys = wide_integer_only_system();
        let Verdict::Unsat { certificate } = decide_with_cuts(&sys) else {
            panic!("the extension certifies this system");
        };
        let leans = certificate_leans(&sys.inequalities(), &certificate);
        assert!(!leans.is_empty(), "never an empty list");
        assert!(
            leans.iter().any(|l| l.contains("NOT enumerated")),
            "the list says it is not the whole combination: {leans:?}",
        );
        assert!(
            leans.iter().any(|l| l.contains("clause carriage not introspected")),
            "and says what it did not look at: {leans:?}",
        );
        // The cuts themselves are still named — the fix ADDS a statement, it
        // does not take the useful half away.
        assert!(leans.iter().any(|l| l.contains("C-G cut")), "{leans:?}");
    }

    /// **A FEASIBLE PROBE AFTER AN UNDECIDED ONE IS NOT A FORCING PROOF.**
    ///
    /// B1 examines coordinates in canonical order and takes the FIRST whose
    /// relaxation is feasible. If an earlier coordinate could not be decided,
    /// the claim "this is the first" is not established — the undecided one
    /// might have been feasible too, and the exception would then be applied to
    /// the wrong coordinate. The driver already withholds the JOINT verdict over
    /// an undecided probe; it must be equally strict about claiming FORCED.
    #[test]
    fn an_undecided_probe_before_a_feasible_one_withholds_rather_than_claiming_forced() {
        let s4 = col("S4");
        let problem = anchored_problem(&[("S4 at most -2", &[(s4, -1)], 2)]);
        let clause = |c: CandidateFeature| problem.clause_text(c);
        let probe = |candidate, answer, detail: &str| ExceptionProbe {
            candidate,
            clause: clause(candidate),
            answer,
            detail: detail.to_string(),
            leaned_on: Vec::new(),
            residual: None,
        };

        // S2 undecided, then S4 feasible: the canonical-order claim cannot be
        // established, so nothing is excepted.
        let withheld = fold_forcing(
            vec![
                probe(CandidateFeature::ProductionAccess, ProbeAnswer::StillInfeasible, "certified"),
                probe(CandidateFeature::TrackAccess, ProbeAnswer::Undecided, "the oracle stopped"),
                probe(CandidateFeature::PresidentLocalAccess, ProbeAnswer::Feasible, "SAT"),
            ],
            Some(vec![0; AXIS_WIDTH]),
        );
        let Forcing::Undecided { reason, .. } = withheld else {
            panic!("an earlier undecided probe withholds the FORCED claim");
        };
        assert!(reason.contains("S2"), "the undecided coordinate is named: {reason}");
        assert!(reason.contains("canonical"), "and why the claim fails: {reason}");
    }

    /// The other three arms of the same fold, so every route out of it is
    /// pinned and the reviewer's "replace the whole thing with `Joint`" mutation
    /// fails a named cell.
    #[test]
    fn the_forcing_fold_answers_each_of_its_four_arms() {
        let problem = anchored_problem(&[("S4 at most -2", &[(col("S4"), -1)], 2)]);
        let probe = |candidate, answer| ExceptionProbe {
            candidate,
            clause: problem.clause_text(candidate),
            answer,
            detail: String::new(),
            leaned_on: Vec::new(),
            residual: None,
        };
        let (s1, s2, s4) = (
            CandidateFeature::ProductionAccess,
            CandidateFeature::TrackAccess,
            CandidateFeature::PresidentLocalAccess,
        );

        // (1) every clause probed and certified infeasible -> JOINT.
        let joint = fold_forcing(
            vec![probe(s1, ProbeAnswer::StillInfeasible), probe(s2, ProbeAnswer::StillInfeasible)],
            None,
        );
        assert!(matches!(joint, Forcing::Joint { .. }), "all decided, none feasible");

        // (2) feasible with every EARLIER probe decided -> FORCED.
        let forced = fold_forcing(
            vec![probe(s1, ProbeAnswer::StillInfeasible), probe(s4, ProbeAnswer::Feasible)],
            Some(vec![7; AXIS_WIDTH]),
        );
        let Forcing::Forced { candidate, witness, .. } = forced else {
            panic!("a clean canonical prefix proves the forcing");
        };
        assert_eq!(candidate, s4);
        assert_eq!(witness[0], 7, "the witness rides through the fold untouched");

        // (3) undecided, no feasible -> WITHHELD, never JOINT: "no coordinate is
        // singly forced" is a statement about EVERY coordinate.
        let withheld = fold_forcing(
            vec![probe(s1, ProbeAnswer::StillInfeasible), probe(s2, ProbeAnswer::Undecided)],
            None,
        );
        let Forcing::Undecided { reason, .. } = withheld else {
            panic!("an undecided probe withholds the JOINT verdict");
        };
        assert!(reason.contains("S2"), "{reason}");

        // (4) a witness with no feasible probe is a construction defect, and it
        // is typed rather than trusted.
        let malformed = fold_forcing(
            vec![probe(s1, ProbeAnswer::StillInfeasible)],
            Some(vec![0; AXIS_WIDTH]),
        );
        assert!(matches!(malformed, Forcing::Undecided { .. }), "a witness needs a FEASIBLE probe");
    }

    /// A REMOVED feature has no sign clause, so naming one is a typed refusal
    /// rather than a wider system — and the driver never names one, because
    /// `clause_active` excludes them.
    #[test]
    fn a_removed_feature_has_no_sign_clause_to_except() {
        let s4 = col("S4");
        let problem = anchored_problem(&[("S4 at most -2", &[(s4, -1)], 2)]);
        for candidate in REPAIR_REMOVED {
            assert!(!problem.clause_active(candidate), "{}", candidate.label());
            let refused = problem.excepting(candidate);
            assert!(refused.is_err(), "{} cannot be excepted", candidate.label());
        }
    }
}
