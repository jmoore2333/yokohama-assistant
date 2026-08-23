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

use yokohama_core::game::eval::{Feature, ScoreStructure, WeightTable};
use yokohama_core::game::greedy::weight_table_identity;
use yokohama_core::game::pair_fixtures::{
    build_registered_pair, package_digest, CandidateFeature, FeatureId, MarginClass,
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

    for column in 0..AXIS_WIDTH {
        let mut sum: i128 = 0;
        for (row, multiplier) in rows.iter().zip(y) {
            let term = i128::from(row.coef[column])
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
    (0..AXIS_WIDTH).filter(|&j| rows.iter().any(|r| r.coef[j] != 0)).collect()
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
        Relaxation::Feasible => integer_search(sys, &columns),
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
fn integer_search(sys: &System, columns: &[usize]) -> Verdict {
    let mut nodes = 0u64;
    match explore(sys, columns, sys.lo.clone(), sys.hi.clone(), &mut nodes) {
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
            reason: format!("the search spent its registered budget of {NODE_BUDGET} nodes"),
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
) -> Search {
    *nodes += 1;
    if *nodes > NODE_BUDGET {
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
            match explore(sys, columns, lo.clone(), lower_hi, nodes) {
                Search::Exhausted => {}
                other => return other,
            }
            let mut upper_lo = lo;
            upper_lo[branch] = mid + 1;
            explore(sys, columns, upper_lo, hi, nodes)
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
fn minimize(_sys: &System) -> Minimization {
    unimplemented!("task C")
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

fn scale_diagnostics(_sys: &System) -> ScaleDiagnostics {
    unimplemented!("task C")
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

fn implied_by_the_rest(_sys: &System) -> ImplicationReport {
    unimplemented!("task C")
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
/// The drift defence is not this file's honour. It is the extraction's
/// `expected_nonzero` check ([`row_from_signatures`]): a swapped arm puts a
/// nonzero at a row the registration does not list, and the real run REFUSES
/// naming it rather than solving.
fn candidate_dispatch(
    _s: &ScoreStructure,
    _view: &PlayerView,
    _player: &VisiblePlayer,
    _feature: CandidateFeature,
    _rules: &Rules,
) -> i64 {
    unimplemented!("task C")
}

/// One view's 34-wide MOVER-RELATIVE vector, `mine − theirs`.
///
/// The same quantity `eval_probe`'s §5.3 signature takes, and for the same
/// reason: a weight multiplies `mine − theirs` (`game/eval.rs`), so §5.4's row
/// `Σ_f w_f·Δv_f ≥ margin` is the DIFFERENCE of two of these.
fn relative_vector(
    _view: &PlayerView,
    _mover: PlayerId,
    _rules: &Rules,
) -> Result<Vec<i64>, String> {
    unimplemented!("task C")
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
    _pair: &'static PairDeclaration,
    _stage: StageEndpoint,
    _greater: &[i64],
    _lesser: &[i64],
) -> Result<Row, String> {
    unimplemented!("task C")
}

/// Assemble one stage's system: extracted pair rows in DELETION ORDER, then the
/// two hand-ordering rows, over the box with `w_vp` pinned.
///
/// Separate from the extraction so the assembly can be exercised on synthetic
/// rows: §5.4's freeze-before-solve clause keeps the frozen thirty-four out of
/// this batch entirely, and a wiring test that had to build them would break it.
fn assemble(_stage: StageEndpoint, _pair_rows: Vec<Row>, _box_limit: i64) -> System {
    unimplemented!("task C")
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
            "unsat-fractional-margin" => toy(
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
            ),
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
            _ => return None,
        })
    }

    /// Every registered toy's name, in the order [`named`] lists them.
    pub const NAMES: [&str; 9] = [
        "standing-only",
        "sat-reachable",
        "sat-coupled",
        "unsat-direct",
        "unsat-box",
        "unsat-cycle",
        "unsat-fractional-margin",
        "unsat-integer-only",
        "unsat-box-widening",
    ];
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

/// One weight vector's entries that differ from the V2-padded start.
fn print_vector(sys: &System, w: &[i64]) {
    let start = sys.v2_padded();
    let mut moved = 0usize;
    for (i, id) in FeatureId::all().into_iter().enumerate() {
        if w[i] != start[i] {
            println!("  {:<28}  {:>5}  (from {})", id.label(), w[i], start[i]);
            moved += 1;
        }
    }
    if moved == 0 {
        println!("  none -- the V2-padded start was already the returned point");
    }
}

/// One verdict and its certificate, in the words §5.4 requires.
fn render_verdict(sys: &System, verdict: &Verdict) -> String {
    let mut out = String::new();
    match verdict {
        Verdict::Sat { w } => {
            out.push_str("                     a point satisfying every inequality, re-checked\n");
            for (i, id) in FeatureId::all().into_iter().enumerate() {
                if w[i] != sys.v2_padded()[i] {
                    out.push_str(&format!("  {:<28}  {:>5}\n", id.label(), w[i]));
                }
            }
            out.push_str(&format!("{SAT_READING}\n"));
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
    print_vector(&sys, &report.w);

    println!();
    let verdict = decide(&sys);
    println!("exact oracle         {}", verdict.name());
    print!("{}", render_verdict(&sys, &verdict));
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
                }
            }
        }
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

    /// The 18-arm dispatcher against the same helpers, restated arm by arm from
    /// the helper NAMES rather than from the dispatcher.
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
        assert_eq!(expected.len(), CandidateFeature::COUNT, "every column is restated");
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
