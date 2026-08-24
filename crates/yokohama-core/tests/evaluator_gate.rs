//! The evaluator gate — Phase 3.3, spec
//! `docs/plans/2026-07-26-phase3.3-evaluator-search-spec.md` §10.2, governed by
//! §0.1 (the promotion gate) and §0.3(b) (every played move explained).
//!
//! WHAT THIS BINARY IS FOR. Map §5-3.3's gate opens "beats family-weighted
//! random with honest intervals (paired, seat-rotated)". Spec §0.1 pre-committed
//! the executable reading of that clause — contestant under test **Lookahead-v1**
//! as X against `RolloutPolicy::FAMILY_WEIGHTED_DEFAULT` as Y, 2P, unit = one
//! world seed played at both seatings, **N = 200 units / 400 games**, percentile
//! bootstrap over the unit shares at B = 10,000 — and this binary is the five
//! cells of §10.2's table made executable, at exactly the Ns that table states:
//!
//! | cell | contestants | N (units) | asserts |
//! |---|---|---|---|
//! | promotion (arm 1) | Lookahead-v1 vs family-weighted | 200 | **DIRECTION** + §0.3(b) identity |
//! | lattice A | Greedy-v1 vs family-weighted | 200 | plumbing only — figures PRINTED |
//! | lattice B | Lookahead-v1 vs Greedy-v1 | **100** | plumbing only — figures PRINTED |
//! | null self-test | Greedy-v1 vs Greedy-v1 | 200 | interval contains 0.5, SE > 0 |
//! | determinism | the promotion cell at N = 12, run twice | 12×2 | byte-identical `CellReport` |
//!
//! **Lattice B's N is HALVED on purpose, and that is a stated choice rather than
//! a budget accident** (spec §10.2): the cell is recorded evidence about where the
//! two evaluator rungs sit relative to each other, not a gate clause, so it buys
//! its ~41 s at half the doctrine N. Nothing in the gate reads its direction.
//!
//! ## The two arms, and why only one of them lives here
//!
//! Spec §0.1 splits the promotion gate in two, and **the gate is DECLARED met
//! only if BOTH pass**:
//!
//! - **Arm 1 — the roster regression cell, which is what this file asserts
//!   forever.** Committed diagnostic master, single-threaded, byte-identical
//!   reproduction pinned. Its job is to notice a regression, so it must be
//!   deterministic; a fresh-entropy assertion in the roster would flake at the
//!   nominal rate BY CONSTRUCTION and the determinism cell would be unstatable
//!   (3.2 §3's gate-cell exemption, inherited verbatim).
//! - **Arm 2 — the fresh-entropy confirmation, which is NOT in this binary.** At
//!   the 3.3 close the same cell at the same N runs once on an OS-entropy master,
//!   drawn and committed with the run, as an `examples/arena` command whose full
//!   output lands in the close-out artifact. That is T12's business. **The
//!   close-out quotes ARM 2's figure as the strength claim** (3.2 §3: quoted
//!   figures come from entropy runs); arm 1's figure below is a regression
//!   fixture, and citing it as "how strong the bot is" would be quoting a frozen
//!   200-seed sample.
//!
//! ## THE MISS PATH — read this before touching anything
//!
//! If the promotion cell misses its direction, **STOP**. Spec §0.1's miss path is
//! explicit and it is orchestrator territory: the bot iterates (weights within
//! §2, knobs within §3/§4) and re-attempts as a NEW pre-declared attempt, with a
//! FRESH OS-entropy master for arm 2 each time (a frozen master plus k
//! re-attempts compounds a ~2.5% one-sided pass rate — ledger row 16's type-I
//! hazard). Nothing in this file may be tuned to make a miss green: not the
//! master, not N, not the direction predicate. Report the full figures and let
//! the attempt be counted.
//!
//! ## Direction IS asserted here — and ONLY in the promotion cell
//!
//! This is the deliberate contrast with `tests/arena_gate.rs`, whose module doc
//! forbids direction assertions across the board. The scoping is that file's own:
//! its cells' subject is the INSTRUMENT, so asserting who wins would assert a
//! fact about the policies inside a test about the estimator. This cell's subject
//! is the BOT, and the map's gate text literally reads "beats". The instrument's
//! own validation (null cell, synthetic coverage, refusal path, CRN keying) stays
//! in `arena_gate` and is not duplicated.
//!
//! Two shapes count as "beats", both pre-committed as passing before the run:
//!
//!   1. the bootstrap 95% interval sits **entirely above 0.5** (`lo > 0.5`); or
//!   2. the **all-won zero-width refusal**, whose rule-of-three one-sided bound
//!      `1 − 3/200 = 0.985` exceeds 0.5 — it exists because the estimator HAS
//!      that corner and a gate must be total over it.
//!
//! An all-LOST or interior-constant refusal is a MISS, not a pass, and the arm
//! below says so by requiring the unit vector to be all-won before it accepts a
//! refusal at all.
//!
//! **WHICH ARM FIRED AT HEAD, AND WHY THAT IS NOT WHAT THE SPEC EXPECTED — read
//! this before quoting the cell.** Spec §0.1 calls shape 1 "the operative arm"
//! and shape 2 "a degeneracy safety valve, not an expected path", putting
//! P(all-won at N=200) ≈ 8×10⁻¹⁴. **Shape 2 is what fired: 200 of 200 units and
//! 400 of 400 games to Lookahead-v1, θ̂ 1.000000, refusal at the 0.985 bound.**
//! That is NOT an 8×10⁻¹⁴ event, and reading it as one would be the wrong
//! inference. The spec's probability was computed at the RESEARCH BASE's per-unit
//! sweep rate of 0.86 — measured for an eval-**v0** *greedy* agent on a different
//! corpus and regime (research notes R2's 516/83/1 unit split). The shipped
//! contestant is a turn-boundary SEARCH rung over the FROZEN weight table
//! (`WeightTable::V1` when this was written, `V2` since the 2026-08-10 freeze),
//! and against family-weighted random at 2P its realized per-unit sweep rate is
//! 1.00. The honest statement is that **the estimate was for a weaker bot**, so
//! the anticipated-rare corner is the LIKELY outcome here rather than a freak
//! sample — which is also why lattice B (Lookahead vs *Greedy*, the tougher
//! opponent) refuses too. Consequences worth carrying:
//!
//! - The gate's direction rests on the **rule-of-three one-sided bound**, not on
//!   a two-sided interval. That is a pre-committed pass, and it is also a weaker
//!   quantitative statement than a two-sided interval would have been: it says
//!   θ ≥ 0.985 with 95% confidence and says nothing sharper.
//! - This binary still produces REAL two-sided intervals — lattice A
//!   (`[0.577500, 0.680000]`) and the null cell (`[0.500000, 0.597500]`) — so a
//!   reader must not conclude the estimator's two-sided arm went unexercised.
//! - Sizing follow-up (recorded, NOT acted on here): at a 1.00 sweep rate, N is
//!   no longer the binding constraint on this cell's precision and a future
//!   strength claim wanting a two-sided interval needs a HARDER field, not more
//!   units. That is a spec/owner question, not a knob this file may turn.
//!
//! The other four cells assert NO direction. The two lattice cells print theirs;
//! the null cell asserts CONTAINMENT of 0.5, which is the opposite of a direction
//! claim.
//!
//! ## The master
//!
//! One committed diagnostic master for every cell: [`MASTER`] =
//! `0xA3E1_2026_0727_0001` — the arena `A3E1` family, date field `2026_0727`,
//! serial `0001`, the first free serial in that family at authoring (grep-verified
//! against every `0xA3E1…` literal in the crate: 3.2 holds `0726_0001`,
//! `0726_0005`–`0726_0008`, `0726_0009`, `0726_0033`, `0726_0036`, `0726_0A11`;
//! 3.3 holds `0727_0090`–`0727_0092` in the scenario suite and `0727_0DEC` in the
//! depth experiment). **Chosen by serial order, before any cell was run** — a
//! master picked because it produced a passing direction would be master
//! shopping, which is exactly what spec §0.1's fresh-per-attempt arm-2 rule
//! exists to prevent.
//!
//! ## Measured at HEAD
//!
//! Release, Apple M5 10-core, quiet machine, binaries pre-built,
//! `--test-threads=1` (part of the documented command, so the recorded figure is
//! the one the command produces). Per-cell walls come from running each cell
//! ALONE; the total is a separate whole-binary run, because a sum of per-command
//! times is not a measured total (the repo's standing rule for the gate roster,
//! applied to this binary's own cells):
//!
//! ```text
//! promotion       400 games    77.13 s    192.8 ms/game
//! lattice A       400 games     5.35 s     13.4 ms/game
//! lattice B       200 games    40.79 s    204.0 ms/game
//! null self-test  400 games     8.24 s     20.6 ms/game
//! determinism      72 games    13.51 s    (2 asserted runs + 1 anti-vacuity run)
//! sum of the five             145.02 s
//! WHOLE BINARY, one sequence  145.84 s / 144.60 s across two runs, 1,472 games
//! ```
//!
//! **Those per-cell walls are a `SEMANTICS_REV` 5 / `WeightTable::V1` reading and
//! are NOT re-measured here.** The whole binary at rev 7 read 157.83 / 160.64 s
//! at the two rules-parity merge closes and 158.29 s at the 3.5c day-0 baseline;
//! at the v2 freeze it reads **184.17 s** (whole binary, one sequence, same
//! command, same 1,472 games — 3:08 total wall including the build check). The
//! rev-7 band and this figure are both above the rev-5 one because the games
//! themselves got longer, and V2's games are longer again (see the promotion
//! cell's own doc). Per-cell walls at v2 were NOT re-taken; quote the whole-binary
//! figure, which is the one the documented command produces.
//!
//! Against spec §10.2's estimate of ~150–170 s: **~145 s, just BELOW the low
//! end** — under target, so the over-target disposition does not fire. Row by row
//! against the table's per-cell estimates (84 / 4 / 45 / 5 / 11 s): promotion
//! −6.9 s, lattice A +1.4, lattice B −4.2, null +3.2, determinism +2.5. Two of
//! those have named causes rather than noise. The promotion cell is cheaper
//! because a Lookahead-vs-family-weighted game costs **192.8 ms** against the
//! spec's 209 ms figure; lattice B is cheaper only because it plays HALF the
//! games, since its per-game cost is **204.0 ms** — Lookahead-vs-Greedy games are
//! DEARER per game than Lookahead-vs-random ones (both seats evaluate), which is
//! worth knowing before anyone sizes another evaluator-vs-evaluator cell off the
//! promotion cell's rate. The determinism row is +2.5 s partly by construction:
//! it runs a third N = 12 cell as the anti-vacuity control (see that test's doc).
//! The per-edit smoke measures **7.14 s debug**.
//!
//! ## Tier
//!
//! Release, five cells, `#[ignore]`d:
//! `cargo test --release -p yokohama-core --test evaluator_gate -- --ignored --test-threads=1`.
//! The sixth test, [`the_evaluator_battery_runs_at_smoke_scale`], is spec §10.1's
//! plumbing smoke and is **never `#[ignore]`d** — a release-only binary rots
//! invisibly (`arena_gate`'s precedent, same reasoning).

use yokohama_core::{
    run_cell, CellReport, CellSpec, Contestant, IntervalOutcome, RolloutPolicy, Rules,
    SeedScheduleKind,
};

/// This binary's committed diagnostic master. See the module doc's master
/// paragraph — serial order, chosen before any cell ran.
const MASTER: u64 = 0xA3E1_2026_0727_0001;

/// The 2P null. Every cell here is 2P, so `1/players` is 0.5 throughout; the
/// report's cross-checks print it per cell rather than trusting this constant.
const NULL_2P: f64 = 0.5;

// ---------------------------------------------------------------------------
// Scale
// ---------------------------------------------------------------------------

/// Every size this binary runs at, in one place — `arena_gate`'s pattern, and for
/// the same two reasons.
///
/// [`Scale::GATE`] is the shipped release tier: spec §10.2's table verbatim,
/// including lattice B's deliberately halved N.
///
/// [`Scale::SMOKE`] is development size. Every cell function takes a `Scale`, so
/// a developer can point ANY of them at `SMOKE` and get a run in seconds instead
/// of waiting on 400 Lookahead games — which is why `SMOKE` carries a value for
/// every field, including fields no shipped test reads at that size. What SHIPS
/// at smoke size is spec §10.1's named pair (`the_evaluator_battery_runs_at_smoke_scale`):
/// Greedy-v1 at 2 units plus ONE Lookahead unit. The statistical cells stay
/// gate-only because their assertions carry no information at one or two units —
/// the promotion cell's own refusal bound at N = 1 is `1 − 3/1 = −2`, which is
/// not a claim about anything.
#[derive(Clone, Copy, Debug)]
struct Scale {
    /// Design units (2P pairs) in the promotion cell — spec §0.1's N.
    promotion_units: u32,
    /// Design units in lattice A (Greedy-v1 vs family-weighted).
    lattice_a_units: u32,
    /// Design units in lattice B (Lookahead-v1 vs Greedy-v1) — HALVED at the
    /// gate tier by the spec's stated choice, not by rounding.
    lattice_b_units: u32,
    /// Design units in the Greedy-vs-Greedy null self-test.
    null_units: u32,
    /// Design units in each run of the determinism cell.
    determinism_units: u32,
}

impl Scale {
    /// Spec §10.2's table.
    const GATE: Self = Self {
        promotion_units: 200,
        lattice_a_units: 200,
        lattice_b_units: 100,
        null_units: 200,
        determinism_units: 12,
    };

    /// Development / per-edit plumbing size. The two fields the shipped smoke
    /// reads are `promotion_units: 1` (spec §10.1's ONE Lookahead unit) and
    /// `lattice_a_units: 2` (its Greedy-v1 at N = 2).
    const SMOKE: Self = Self {
        promotion_units: 1,
        lattice_a_units: 2,
        lattice_b_units: 1,
        null_units: 2,
        determinism_units: 1,
    };
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Every cell in this binary is 2P on the committed master, so the spec is
/// assembled at one site and the contestants are the only thing a cell chooses.
fn cell(units: u32, name: &str, x: Contestant, y: Contestant) -> CellSpec {
    CellSpec {
        name: name.to_string(),
        players: 2,
        x,
        y,
        units,
        master: SeedScheduleKind::Diagnostic(MASTER),
    }
}

fn run(spec: &CellSpec) -> CellReport {
    let rules = Rules::standard();
    run_cell(spec, &rules).unwrap_or_else(|e| panic!("cell {:?} failed: {e}", spec.name))
}

/// Sample standard deviation (n − 1 denominator).
fn sample_sd(values: &[f64]) -> f64 {
    assert!(values.len() >= 2, "sd over fewer than two values");
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let ss: f64 = values.iter().map(|v| (v - mean) * (v - mean)).sum();
    (ss / (values.len() - 1) as f64).sqrt()
}

/// Print a cell exactly as an evaluation run would read it — spec echo FIRST, so
/// a figure scraped out of this binary's output carries its measurement policy
/// with it, then the estimate, then the counters §0.3(b) is written on.
fn print_report(label: &str, report: &CellReport) {
    println!("[{label}] {}", report.spec_echo);
    println!(
        "  theta_hat={:.6}  interval={}  seat0_wins={}/{}  ties_to_seat0={}  vp_margin_mean={:+.2}",
        report.theta_hat,
        describe_interval(&report.interval),
        report.seat0_wins,
        report.games,
        report.ties_to_seat0,
        report.vp_margin_mean,
    );
    // The unit vector's own spread, printed beside the interval it fed: the null
    // cell asserts this is nonzero, and every other cell benefits from a reader
    // being able to see whether a narrow interval came from agreement or from
    // degeneracy.
    if report.x_unit_shares.len() >= 2 {
        let sd = sample_sd(&report.x_unit_shares);
        println!(
            "  unit sd={sd:.6}  SE(theta_hat)={:.6}  n={} units",
            sd / (report.x_unit_shares.len() as f64).sqrt(),
            report.x_unit_shares.len(),
        );
    }
    println!(
        "  applied_by_seat={:?}  x_applied_moves={}  x_explanations={:?}",
        report.applied_by_seat, report.x_applied_moves, report.x_explanations,
    );
    // The search guards, printed beside the counters they qualify: a cell that
    // hit either one valued some line where the guard stopped it rather than at
    // a quiet point, so the figures above would be about a slightly different
    // quantity than they claim. `None` on both is a non-searching X.
    println!(
        "  x_ply_cap_hits={:?}  x_free_cap_hits={:?}",
        report.x_ply_cap_hits, report.x_free_cap_hits,
    );
    for row in &report.cross_checks {
        println!("    - {row}");
    }
    let mut triggers: Vec<String> = report
        .trigger
        .counts()
        .iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect();
    if triggers.is_empty() {
        triggers.push("none recorded".to_string());
    }
    println!("    - triggers over {} games: {}", report.trigger.games(), triggers.join(" "));
}

fn describe_interval(interval: &IntervalOutcome) -> String {
    match interval {
        IntervalOutcome::Bootstrap { lo, hi, b, resample_seed } => {
            format!("bootstrap[{lo:.6}, {hi:.6}] B={b} resample_seed={resample_seed:#018x}")
        }
        IntervalOutcome::RefusedZeroWidth { one_sided_lower, method } => {
            format!("REFUSED one_sided_lower={one_sided_lower:.6} ({method})")
        }
    }
}

/// The checks every cell in this binary owes regardless of what it measures:
/// the unit shape the report claims, the master it claims, and — for an X that
/// HAS an explanation path — spec §0.3(b)'s identity.
///
/// §0.3(b) is asserted here rather than only in the promotion cell because it is
/// free once a cell has run and because `Some(n)` vs `None` is the load-bearing
/// distinction: a `Contestant::Policy` X reports `None` (no explanation path at
/// all), and an evaluator X that silently stopped explaining would report
/// `Some(0)`. Both are caught by the whole equality.
///
/// The SEARCH GUARDS ride here for the same reason (six-lens review): the
/// counter was cloned out per X seat per game and then dropped, so the gate that
/// decides 3.3's promotion checked nothing about `MAX_TURN_PLIES` over its 1,472
/// games. A hit is not a wrong answer, but a capped line is valued mid-turn, so
/// a cell that starts hitting the guard is no longer measuring quite what the
/// published figures measured.
fn assert_cell_plumbing(report: &CellReport, spec: &CellSpec, x_explains: bool) {
    assert_eq!(report.games, 2 * spec.units, "a 2P unit is two games");
    assert_eq!(
        report.x_unit_shares.len(),
        spec.units as usize,
        "one unit share per design unit",
    );
    assert_eq!(report.master, MASTER, "the cell ran on an unexpected master");
    assert!(
        report.x_applied_moves > 0,
        "contestant X applied no moves at all — the cell did not play",
    );
    assert!(
        report.x_applied_moves < report.applied_by_seat.iter().sum::<u64>(),
        "x_applied_moves {} is not a strict part of the cell's total — it stopped following the \
         contestant through the rotation",
        report.x_applied_moves,
    );
    if x_explains {
        assert_eq!(
            report.x_explanations,
            Some(report.x_applied_moves),
            "spec §0.3(b): X played a move it did not explain (or explained one it did not play)",
        );
    } else {
        assert_eq!(
            report.x_explanations, None,
            "a rollout rung has no explanation path, which is not the same fact as explaining zero",
        );
    }

    if x_searches(&spec.x) {
        assert_eq!(
            report.x_ply_cap_hits,
            Some(0),
            "MAX_TURN_PLIES stopped {:?} continuation(s) in cell {:?}: some move X played was \
             explained at the cap instead of at the hand-off it was searched to",
            report.x_ply_cap_hits,
            spec.name,
        );
        // Structural at every cell in this binary (all of them run S1, which
        // never enters the quiescence arm), so this pins the plumbing, not the
        // S2 guard — an S2 cell would give the figure content.
        assert_eq!(
            report.x_free_cap_hits,
            Some(0),
            "MAX_FREE_EXTENSION recorded a hit under a scheme that cannot reach it",
        );
    } else {
        assert_eq!(
            (report.x_ply_cap_hits, report.x_free_cap_hits),
            (None, None),
            "X does not search, so a cap counter would be reporting on a search that never ran",
        );
    }
}

/// Does this contestant run a SEARCH, and therefore carry cap counters?
///
/// An exhaustive match rather than `matches!` so a contestant added later has to
/// answer the question instead of defaulting to "no search" and silently
/// dropping out of the guard assertion above.
fn x_searches(x: &Contestant) -> bool {
    match x {
        Contestant::Policy(_) | Contestant::GreedyV1 { .. } => false,
        // A named identity IS a `LookaheadAgent` at the shipped knobs
        // (`Contestant::build` routes it through the same `build_search`), so
        // it searches and carries the Phase-3.3 `(Some, Some)` cap-counter
        // shape this binary asserts. It answers with `LookaheadV1` rather than
        // needing an arm of its own.
        Contestant::LookaheadV1 { .. }
        | Contestant::AtomicDiag { .. }
        | Contestant::NamedTable(_) => true,
        // Phase 3.4's contestant is not a Phase-3.3 one, and NEITHER branch
        // describes it: it searches, but its FREE counter is `None` by
        // construction (a tree has no quiescence arm to enter). Answering
        // `true` here would make the guard above assert `Some(0)` on a
        // counter that does not exist; answering `false` would drop it out of
        // the guard silently. An mcts cell belongs to `tests/architecture_gate.rs`
        // (spec §8), so reaching this arm is a mis-routed cell.
        Contestant::MctsV1 { .. } => panic!(
            "an mcts-v1 X reached the Phase-3.3 evaluator gate; its cap-counter shape is \
             (Some, None) and this binary asserts (Some, Some) — route it to architecture_gate",
        ),
    }
}

// ---------------------------------------------------------------------------
// The promotion cell (spec §0.1 arm 1, §0.3(b))
// ---------------------------------------------------------------------------

fn promotion_spec(units: u32, name: &str) -> CellSpec {
    cell(
        units,
        name,
        Contestant::LOOKAHEAD_V1_DEFAULT,
        Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
    )
}

/// Run the promotion cell and check everything about it EXCEPT the direction.
/// The direction assertion is the gate test's, so the smoke can reuse this whole
/// body at one unit without pretending a one-unit cell says anything about
/// strength.
fn promotion_cell_at(scale: Scale) -> CellReport {
    let spec = promotion_spec(scale.promotion_units, "promotion-lookahead-v1-vs-fw");
    println!(
        "[promotion] Lookahead-v1 vs FAMILY_WEIGHTED_DEFAULT, 2P, N={} units — spec §0.1 arm 1",
        scale.promotion_units,
    );
    let report = run(&spec);
    print_report("promotion", &report);
    assert_cell_plumbing(&report, &spec, true);

    // §5.5's pairing consequence, printed by the echo and asserted here: an
    // evaluator-vs-rung cell is seed-paired and swap-balanced but NOT
    // setup-identical, because the two contestants resolve the same dealt Order
    // pair by different Keep functions. A reader of these figures needs that.
    assert!(
        report.spec_echo.contains("setup-identical=no"),
        "the promotion cell mixes Keep functions and must say so: {}",
        report.spec_echo,
    );
    assert!(
        report.spec_echo.contains("x=lookahead-v1@v2{"),
        "the contestant under test must be the search rung's own descriptor: {}",
        report.spec_echo,
    );
    report
}

/// **Spec §0.1's promotion gate, arm 1.** Lookahead-v1 vs
/// `RolloutPolicy::FAMILY_WEIGHTED_DEFAULT`, 2P, N = 200 pair units / 400 games,
/// percentile bootstrap at B = 10,000 over the unit shares, committed diagnostic
/// master, single-threaded.
///
/// **This is the ONE cell in this binary that asserts a direction**, and the two
/// accepted shapes were pre-committed before the run (module doc): the interval
/// entirely above 0.5, or the all-won refusal whose rule-of-three bound
/// `1 − 3/200 = 0.985` exceeds 0.5. Spec §0.3(b)'s explanation identity rides
/// this cell via [`assert_cell_plumbing`].
///
/// ON A MISS: do not tune. Spec §0.1's miss path is a re-attempt counted at the
/// orchestrator, with a fresh arm-2 master; see the module doc.
///
/// MUTATION-PROVEN (run at authoring, reverted by inverse edit — L7): SWAPPING the
/// contestants, so family-weighted is X and Lookahead-v1 is Y, reads θ̂ 0.000000
/// with an all-LOST refusal (`one_sided_lower` 0.000000, mean VP margin −69.71)
/// and the direction assert rejects it as a MISS. That is the mutation worth
/// running rather than bumping the 0.5: the refusal arm's hazard is accepting a
/// refusal of the WRONG SIGN, and the all-won check in front of it is what
/// forecloses that.
///
/// MEASURED AT HEAD — RE-RECORDED AT THE v2 FREEZE (2026-08-10; release, master
/// `0xA3E1_2026_0727_0001`): **θ̂ 1.000000, the estimator REFUSED a zero-width
/// interval and returned the all-won rule-of-three one-sided 95% lower bound
/// 0.985000 > 0.5** — the SECOND accepted shape, i.e. the valve rather than the
/// operative arm (the module doc's "which arm fired" paragraph is the honest
/// reading, and it is not the 8×10⁻¹⁴ event the spec's arithmetic describes).
/// 200 of 200 units swept; 400 of 400 games won; seat-0 wins 200/400, so the
/// sweep is not a seat artifact; mean VP margin **+74.15**; 400/400 games
/// completed (ChurchTrackFull 382 / OrderDeckEmpty 18) with no cap hit; applied
/// per seat [68,846 · 69,329] of which X applied **74,331 and explained 74,331**
/// (the §0.3(b) identity, exact); Wilson-on-pooled-games cross-check
/// [0.990488, 1.000000].
///
/// **The freeze did not move the SHAPE, and that was the pre-committed
/// expectation rather than a hope**: the arm was already sweeping at V1 (θ̂
/// 1.000000, the same refusal bound), so V2 could only fail to sweep, and the
/// task's instruction was to STOP and report if it did. What moved is the size
/// of the win — mean VP margin +67.42 → **+74.15** — and the amount of play
/// behind it (X applied 71,707 → 74,331). Neither is asserted. The rev-5 figures
/// this replaces were: θ̂ 1.000000, margin +67.42, triggers 385/15, applied per
/// seat [66,511 · 66,397], X 71,707.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test evaluator_gate -- --ignored --test-threads=1"]
fn lookahead_v1_beats_family_weighted_with_an_honest_interval() {
    let report = promotion_cell_at(Scale::GATE);
    let n = report.x_unit_shares.len();
    // The spec's N is a DESIGN INPUT (§0.1: 200 units, the 3.2 promotion-cell
    // policy), pinned here as a LITERAL beside the direction assert (T11
    // review): the refusal arm's surviving guards are self-referential in n
    // (`1 − 3/n`) and satisfied for any n ≥ 7, so a silent edit of
    // `Scale::GATE.promotion_units` would otherwise move the cell's N with the
    // bound and nothing would say so.
    assert_eq!(n, 200, "the promotion gate runs at the spec's pre-sized N, exactly");

    match report.interval {
        IntervalOutcome::Bootstrap { lo, hi, .. } => {
            assert!(
                lo > NULL_2P,
                "PROMOTION GATE MISS (spec §0.1): the bootstrap 95% interval [{lo:.6}, {hi:.6}] \
                 does not sit entirely above the 2P null {NULL_2P} (theta_hat {:.6} over {n} \
                 units, mean VP margin {:+.2}). This is the spec's MISS PATH — STOP, report the \
                 figures, and let the orchestrator count the attempt. Do NOT tune the master, N, \
                 or this predicate.",
                report.theta_hat,
                report.vp_margin_mean,
            );
            println!(
                "  DIRECTION ASSERTED: interval [{lo:.6}, {hi:.6}] is entirely above {NULL_2P} \
                 (margin at the near end {:+.6})",
                lo - NULL_2P,
            );
        }
        IntervalOutcome::RefusedZeroWidth { one_sided_lower, method } => {
            // The degeneracy valve. It is only a PASS in the all-won direction:
            // an all-lost or interior-constant refusal carries a one-sided lower
            // bound of 0 and is a miss, so the unit vector is checked FIRST.
            assert!(
                report.x_unit_shares.iter().all(|&u| u == 1.0),
                "PROMOTION GATE MISS (spec §0.1): the estimator refused a zero-width interval on \
                 a unit vector that was NOT all-won (theta_hat {:.6}, {method:?}) — an all-lost or \
                 interior-constant refusal is a miss, not the safety valve. Report and stop.",
                report.theta_hat,
            );
            let expected = 1.0 - 3.0 / n as f64;
            assert!(
                (one_sided_lower - expected).abs() < 1e-12,
                "the all-won refusal must carry the rule-of-three bound 1 - 3/{n} = {expected}, \
                 got {one_sided_lower}",
            );
            assert!(
                method.contains("rule-of-three"),
                "the all-won refusal must name its method, got {method:?}",
            );
            assert!(
                one_sided_lower > NULL_2P,
                "PROMOTION GATE MISS: the refusal's one-sided 95% lower bound {one_sided_lower} \
                 does not exceed the null {NULL_2P} — at N = 200 the bound is 0.985, so seeing \
                 this means the cell ran at a smaller N than the gate's",
            );
            println!(
                "  DIRECTION ASSERTED via the all-won arm: every one of {n} units swept, \
                 rule-of-three one-sided 95% lower bound {one_sided_lower:.6} > {NULL_2P}. NOTE \
                 spec §0.1 calls this the degeneracy VALVE and puts P(all-won at N=200) around \
                 8e-14 — that arithmetic used the research base's per-unit sweep rate of 0.86, \
                 measured for an eval-v0 GREEDY agent, so this outcome is the shipped search \
                 rung being stronger than the estimate rather than a freak sample. The gate \
                 passes on a pre-committed shape; the close-out must say WHICH arm fired, and \
                 that the direction rests on a one-sided bound rather than a two-sided interval."
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Lattice A — Greedy-v1 vs family-weighted (recorded)
// ---------------------------------------------------------------------------

fn lattice_a_at(scale: Scale) -> CellReport {
    let spec = cell(
        scale.lattice_a_units,
        "lattice-a-greedy-v1-vs-fw",
        Contestant::GREEDY_V1_DEFAULT,
        Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
    );
    println!(
        "[lattice A] Greedy-v1 vs FAMILY_WEIGHTED_DEFAULT, 2P, N={} units — RECORDED, no direction \
         asserted",
        scale.lattice_a_units,
    );
    let report = run(&spec);
    print_report("lattice A", &report);
    assert_cell_plumbing(&report, &spec, true);
    print_direction("lattice A", &report);
    report
}

/// Lattice A: the ONE-PLY rung against the same field the promotion cell faces.
///
/// **Recorded evidence, not a gate clause** — the map's gate names the search
/// rung, and this cell exists so the close-out can say where the ladder's lower
/// step sits against the same opponent. No direction is asserted; the figures are
/// printed. What IS asserted is plumbing (unit shape, master, and §0.3(b)'s
/// explanation identity on the greedy arm, which is a different tally type from
/// the search arm's and so needs its own exercise at scale).
///
/// MEASURED AT HEAD — RE-RECORDED AT THE v2 FREEZE (2026-08-10, release):
/// **θ̂ 0.630000, bootstrap 95% [0.577500, 0.680000]** — a REAL two-sided
/// interval that excludes the 2P null, which is worth noting because the
/// promotion cell's did not exist to. Unit sd 0.365744, SE(θ̂) 0.025862 over 200
/// units; 252/400 games; mean VP margin +6.36; 2 top-VP ties, both decided for
/// seat 0; ChurchTrackFull 388 / OrderDeckEmpty 10 / ShophousesComplete 1 /
/// TradingHousesComplete 1; X applied **126,423 moves and explained 126,423**.
/// (Pre-freeze, at V1: θ̂ 0.642500, [0.595000, 0.690000], sd 0.337659, 257/400
/// games, margin +7.40, 6 ties, triggers 389/11, X 119,822. The two θ̂s are well
/// inside each other's intervals; nothing here says the freeze moved this cell,
/// and the cell asserts no direction to begin with. The BUILDING triggers are
/// new — this is the first recorded run of this cell where a game ended by
/// completing a building set.)
///
/// Read beside lattice B and the promotion cell, this is the ladder: the one-ply
/// rung beats the same field by a MEASURABLE margin (θ̂ 0.63), the search rung
/// sweeps it (θ̂ 1.00), and the search rung also sweeps the one-ply rung. Recorded,
/// not asserted.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test evaluator_gate -- --ignored --test-threads=1"]
fn lattice_a_greedy_vs_family_weighted_is_recorded() {
    lattice_a_at(Scale::GATE);
}

// ---------------------------------------------------------------------------
// Lattice B — Lookahead-v1 vs Greedy-v1 (recorded, at HALF N)
// ---------------------------------------------------------------------------

fn lattice_b_at(scale: Scale) -> CellReport {
    let spec = cell(
        scale.lattice_b_units,
        "lattice-b-lookahead-v1-vs-greedy-v1",
        Contestant::LOOKAHEAD_V1_DEFAULT,
        Contestant::GREEDY_V1_DEFAULT,
    );
    println!(
        "[lattice B] Lookahead-v1 vs Greedy-v1, 2P, N={} units (HALF the doctrine N — a stated \
         choice, spec §10.2) — RECORDED, no direction asserted",
        scale.lattice_b_units,
    );
    let report = run(&spec);
    print_report("lattice B", &report);
    assert_cell_plumbing(&report, &spec, true);
    // Both contestants are evaluator-scored, so this cell IS setup-identical —
    // the other half of §5.5's scoping, exercised here rather than only claimed.
    assert!(
        report.spec_echo.contains("setup-identical=yes"),
        "an evaluator-vs-evaluator cell shares one Keep function and must say so: {}",
        report.spec_echo,
    );
    print_direction("lattice B", &report);
    report
}

/// Lattice B: the two evaluator rungs against each other — the rung-ladder
/// reading the close-out wants, at the HALVED N spec §10.2 states.
///
/// **Recorded evidence, not a gate clause**, and the halved N is why: 100 units
/// of Lookahead-vs-Greedy is ~30 s, and doubling it would buy precision on a
/// figure nothing asserts. Direction printed, never asserted.
///
/// MEASURED AT HEAD — RE-RECORDED AT THE v2 FREEZE (2026-08-10, release, 200
/// games): **θ̂ 1.000000, the estimator REFUSED a zero-width interval** and
/// returned the all-won rule-of-three one-sided 95% lower bound **0.970000**
/// (`1 − 3/100`). 100 of 100 units and 200 of 200 games to Lookahead-v1; seat-0
/// wins 100/200; mean VP margin +72.81; ChurchTrackFull 200/200;
/// `setup-identical=yes`; X applied **37,411 moves and explained 37,411**.
/// (Before the freeze: margin +65.59, triggers 199/1, X 37,081. Both arms moved
/// to V2 together here — this is S1 against a one-ply rung reading the same
/// table — so the sweep surviving says the search advantage is not an artifact
/// of the old weights.)
///
/// The refusal here is the same phenomenon as the promotion cell's and it is
/// informative in a way that cell is not: the search rung sweeps the STRONGER of
/// the two available fields, so the missing two-sided interval is not an artifact
/// of a weak opponent. It also means this cell cannot say HOW much stronger S1 is
/// than Greedy-v1 — only that it won every unit. Recorded, never asserted.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test evaluator_gate -- --ignored --test-threads=1"]
fn lattice_b_lookahead_vs_greedy_is_recorded() {
    lattice_b_at(Scale::GATE);
}

/// Print whether a cell's interval excludes the null — the figure a strength
/// claim would quote, printed beside cells that assert nothing about it so the
/// distinction between "measured" and "gated" survives a copy-paste.
fn print_direction(label: &str, report: &CellReport) {
    match report.interval {
        IntervalOutcome::Bootstrap { lo, hi, .. } => println!(
            "  RECORDED (not asserted) — {label}: interval {} the 2P null {NULL_2P}; theta_hat \
             {:.6}",
            if hi < NULL_2P || lo > NULL_2P { "EXCLUDES" } else { "contains" },
            report.theta_hat,
        ),
        IntervalOutcome::RefusedZeroWidth { one_sided_lower, method } => println!(
            "  RECORDED (not asserted) — {label}: the estimator REFUSED a zero-width interval \
             (theta_hat {:.6}, one-sided lower {one_sided_lower:.6}, {method:?})",
            report.theta_hat,
        ),
    }
}

// ---------------------------------------------------------------------------
// The null self-test — Greedy-v1 vs Greedy-v1
// ---------------------------------------------------------------------------

/// Spec §10.2's null self-test: the same contestant at both seats must read near
/// its null with a NONZERO SE.
///
/// Greedy-v1 rather than Lookahead-v1, and the spec says why: it exercises the
/// same evaluator-agent path — honest chain, tie-break stream, Keep function,
/// explanation tally — for a fifth of the cost, where a Lookahead null at the
/// doctrine N would spend ~166 s re-checking the keying property the greedy cell
/// already pins.
///
/// **The cell is Keep-HOMOGENEOUS by construction** (§5.5), which is what keeps
/// the self-test at full strength: both seatings of a unit start from a
/// bit-identical state, so a nonzero unit spread cannot be an artifact of the two
/// seatings having resolved the deal differently. The echo's
/// `setup-identical=yes` is asserted, not assumed.
///
/// WHAT THIS CATCHES, and it is not hypothetical: keying the play stream by seat
/// alone makes both games of a pair the same game replayed — every unit reads
/// exactly 0.5, the unit vector goes constant, and the interval collapses to a
/// REFUSAL, i.e. an instrument reporting "perfectly calibrated" while measuring
/// nothing (`arena_gate`'s null cell documents the measured ρ = −1.0000). The
/// `sd > 0` assertion states that about the INPUT and the `Bootstrap`-with-width
/// assertion states it about the OUTPUT.
///
/// THE CONTAINMENT ASSERTION IS A FIXED-MASTER CHECK, deliberately: at a fresh
/// master it would fail at the nominal 5% rate, which is 3.2 §3's whole argument
/// for the gate-cell exemption. It asks "is the instrument still calibrated
/// against the evaluator agent since the last commit", and that answer has to be
/// deterministic to mean anything.
///
/// MUTATION-PROVEN (run at authoring, reverted by inverse edit — L7): substituting
/// a Keep-homogeneous BLOWOUT pair (Lookahead-v1 vs Greedy-v1) for the self-play
/// pair reproduces exactly the degenerate shape the CRN trap produces — θ̂
/// 1.000000, a constant unit vector, a refusal instead of an interval — and the
/// `sd > 0` assertion FAILS with its CRN-trap message. Substituting a mixed-Keep
/// pair (Greedy-v1 vs family-weighted) fails the `setup-identical=yes` assertion
/// first, which is the other half of the cell's shape.
///
/// MEASURED AT HEAD — RE-RECORDED AT THE v2 FREEZE (2026-08-10, release, 400
/// games): **θ̂ 0.547500, bootstrap 95% [0.500000, 0.597500]**, containing 0.5
/// with real width; unit sd 0.353011, SE(θ̂) 0.024962 over 200 units; 219/400
/// games to X; seat-0 wins 207/400; mean VP margin +1.76; 8 top-VP ties;
/// ChurchTrackFull 400/400; X applied **119,260 moves and explained 119,260**.
/// (Before the freeze: θ̂ 0.515000, [0.465000, 0.562500], sd 0.354121, SE
/// 0.025040, 206/400, seat-0 212/400, margin +2.87, 6 ties, X 106,848.)
///
/// **READ THE CONTAINMENT MARGIN HONESTLY: the interval's LOWER BOUND is
/// 0.500000, i.e. the containment assertion passes at its boundary.** The
/// assertion is `lo <= 0.5 <= hi` and it holds, but a null cell whose interval
/// starts exactly at the null is one resample away from failing, and a future
/// run of this cell may well trip it. That would NOT be evidence of a CRN
/// collapse — the collapse this cell exists to catch produces `sd == 0` and a
/// refusal, both of which are separately asserted and both healthy here (sd
/// 0.353011, real bootstrap width 0.0975). It would be the ordinary luck of a
/// two-sided interval at N = 200 on a self-play null whose true θ is 0.5 by
/// construction. If it fires, the honest disposition is to record the run and
/// re-read the shape assertions, not to widen the band.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test evaluator_gate -- --ignored --test-threads=1"]
fn the_greedy_null_self_test_is_honest() {
    null_cell_at(Scale::GATE);
}

fn null_cell_at(scale: Scale) {
    let spec = cell(
        scale.null_units,
        "null-greedy-v1-selfplay",
        Contestant::GREEDY_V1_DEFAULT,
        Contestant::GREEDY_V1_DEFAULT,
    );
    println!(
        "[null] Greedy-v1 at BOTH seats, 2P, N={} units — theta_hat must sit near {NULL_2P} with \
         real width",
        scale.null_units,
    );
    let report = run(&spec);
    print_report("null", &report);
    assert_cell_plumbing(&report, &spec, true);
    assert!(
        report.spec_echo.contains("setup-identical=yes"),
        "the null cell must be Keep-homogeneous (spec §5.5) or its self-test is weaker than it \
         claims: {}",
        report.spec_echo,
    );

    let sd = sample_sd(&report.x_unit_shares);
    assert!(
        sd > 0.0,
        "the unit vector carried NO variation (sd={sd}) — the CRN trap, not a calibrated null",
    );

    match report.interval {
        IntervalOutcome::Bootstrap { lo, hi, .. } => {
            assert!(hi > lo, "a null cell must report a NONZERO width, got [{lo}, {hi}]");
            assert!(
                lo <= NULL_2P && NULL_2P <= hi,
                "the same-contestant null {NULL_2P} fell OUTSIDE the cell's own 95% interval \
                 [{lo}, {hi}] (theta_hat {:.6}) — at a committed master this is a regression, not \
                 a flake",
                report.theta_hat,
            );
            assert!(
                lo <= report.theta_hat && report.theta_hat <= hi,
                "the point estimate {:.6} fell outside its own interval [{lo}, {hi}]",
                report.theta_hat,
            );
        }
        ref other => panic!("the null cell must report a real interval, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Determinism — the promotion cell at N = 12, twice
// ---------------------------------------------------------------------------

/// Spec §10.2's determinism cell: the promotion cell at N = 12, run twice,
/// byte-identical `CellReport` serialization.
///
/// The promotion cell's SHAPE rather than a cheap one, because the property under
/// test is that the ASSERTING cell reproduces — every stream the search rung
/// touches (world seeds, the belief sampler, the tie-break draws at root and beam
/// frontier, the bootstrap resample) has to be a deterministic function of the
/// master, and a Greedy or Policy cell would exercise fewer of them.
///
/// ANTI-VACUITY, and it costs a third run: byte-identity is trivially true for a
/// report that does not depend on the master at all, so a flipped master must
/// move the bytes. The whole-string `assert_ne!` alone is weak — `spec_echo`
/// carries the master, so it would pass on the echo bytes with the play frozen —
/// so the stronger check is on a PLAY-DERIVED figure (`vp_margin_mean`, a mean
/// over the cell's games). Spec §10.2's "12×2" sizes the ASSERTED pair; this
/// third run is the anti-vacuity control and is declared as an addition to it.
///
/// MUTATION-PROVEN (run at authoring, reverted by inverse edit — L7): pointing the
/// control run at `MASTER` itself instead of the flipped value FAILS the
/// anti-vacuity check ("the report did not depend on the master at all"), so the
/// control is genuinely a control and not decoration.
///
/// MEASURED AT HEAD — RE-RECORDED AT THE v2 FREEZE (2026-08-10, release, 72
/// games): the two N = 12 runs serialize to **2,129 identical bytes**; the
/// flipped-master control moves `vp_margin_mean` from **+75.54 to +84.58**.
/// (Before the freeze: 1,870 bytes, +68.50 → +78.54.)
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test evaluator_gate -- --ignored --test-threads=1"]
fn the_promotion_cell_reproduces_byte_identically() {
    determinism_at(Scale::GATE);
}

fn determinism_at(scale: Scale) {
    let spec = promotion_spec(scale.determinism_units, "determinism-promotion-2p");
    println!(
        "[determinism] the promotion cell at N={} units, run twice",
        scale.determinism_units,
    );

    let first_report = run(&spec);
    let first = serde_json::to_string(&first_report).expect("a report serializes");
    let second = serde_json::to_string(&run(&spec)).expect("a report serializes");
    assert_eq!(first, second, "the same master produced two different reports");
    println!("  {} bytes, identical across runs", first.len());

    let mut other = spec.clone();
    other.master = SeedScheduleKind::Diagnostic(MASTER ^ 0xFFFF_FFFF);
    let control = run(&other);
    let third = serde_json::to_string(&control).expect("a report serializes");
    assert_ne!(first, third, "the report did not depend on the master at all");
    assert!(
        (first_report.vp_margin_mean - control.vp_margin_mean).abs() > f64::EPSILON,
        "a flipped master left the PLAY-derived figures identical (vp_margin_mean {:+.6} both \
         times) — the byte inequality above would then be satisfied by the master printed in \
         spec_echo and would prove nothing",
        first_report.vp_margin_mean,
    );
    println!(
        "  anti-vacuity: flipping the master moved vp_margin_mean {:+.2} -> {:+.2}",
        first_report.vp_margin_mean, control.vp_margin_mean,
    );
}

// ---------------------------------------------------------------------------
// Per-edit plumbing smoke (spec §10.1)
// ---------------------------------------------------------------------------

/// Spec §10.1's plumbing smoke — the one test in this binary that is NOT
/// `#[ignore]`d.
///
/// It asserts nothing the release tier does not; its subject is this harness's
/// own plumbing. A release-only binary rots invisibly: a helper that started
/// panicking, a contestant const that moved, an echo token that got renamed
/// would all be found at merge time rather than at edit time.
///
/// WHAT IT RUNS is exactly what §10.1 names: **Greedy-v1 at N = 2 units** (fast
/// by construction) **plus ONE Lookahead unit** at the SHIPPED knobs. The
/// Lookahead unit is what makes the smoke worth its cost — the search rung is the
/// contestant under test and is the expensive path, so a smoke that only ran the
/// greedy cell would leave the gate's own subject unexercised per-edit. It is
/// held to ONE unit for the reason §10.1 names: a B = 4 search game is ~230 ms in
/// release and debug runs it ~14×, so units are the only knob that keeps this
/// affordable.
///
/// WHAT IT DELIBERATELY DOES NOT RUN: the direction assertion, the null
/// containment assertion, lattice B, and the determinism cell. The first two
/// carry no information at one or two units (the promotion cell's refusal bound
/// at N = 1 is `1 − 3/1 = −2`); the last two are cost with no new plumbing —
/// determinism at one unit re-runs the same `run_cell` path the promotion call
/// already exercised, and lattice B's only unshared assertion
/// (`setup-identical=yes`) is pinned per-edit in `stats.rs` and by the null cell
/// at the gate tier.
///
/// MEASURED **7.14 s debug** at HEAD (whole binary, debug, the five gate cells
/// `#[ignore]`d out) — the Lookahead unit is essentially all of it.
#[test]
fn the_evaluator_battery_runs_at_smoke_scale() {
    let scale = Scale::SMOKE;
    lattice_a_at(scale);
    promotion_cell_at(scale);
}
