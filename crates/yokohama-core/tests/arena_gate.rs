//! The arena gate — Phase 3.2, spec `docs/plans/2026-07-25-phase3.2-arena-spec.md`
//! §12 gate 1, governed by §0-R3.
//!
//! WHAT THIS BINARY IS FOR. The map's 3.2 gate asked that the arena
//! "reproducibly rank two different seat policies with honest intervals". Spec
//! §0-R3 showed that clause is satisfiable while exercising no interval
//! machinery at all — the two policies that existed when it was written are a
//! blowout (retreat-averse beat uniform 4000/4000 games at 2P), so θ̂ = 1.000
//! with zero per-unit variance and every interval silently collapses to ±0.000.
//! The restatement replaced one clause with five, and this binary is those five
//! made executable:
//!
//!   (i)   a non-degenerate cell reporting a real interval, plus a GUARANTEED
//!         edge manufactured from two shipped knob settings
//!         (`edge_cells_report_real_intervals`);
//!   (ii)  a same-policy null cell whose interval contains its null with a
//!         nonzero SE (`null_cell_is_honest`);
//!   (iii) validation of the interval machinery in two honest parts — synthetic
//!         units with KNOWN truth at R ≥ 2,000
//!         (`synthetic_coverage_validates_the_interval_machinery`) and one small
//!         REAL-game replicate block, which is the only half that can see
//!         clustering (`real_game_replicates_match_reported_se`);
//!   (iv)  the zero-width refusal path exercised on real games
//!         (`the_bracket_pair_refuses_zero_width`);
//!   (v)   reproducibility, byte-for-byte (`reports_are_deterministic`).
//!
//! Two more properties ride here because this is the binary that plays arena
//! games at scale: the honest-path counters spec §4 turned into gate assertions
//! (`honest_play_counters_hold`), and the corpus check that makes "emitted
//! records replay strict-clean" verified rather than asserted in prose
//! (`emitted_corpus_replays_strict_clean`).
//!
//! EVERY CELL RUNS ON A COMMITTED DIAGNOSTIC MASTER, single-threaded. That is
//! spec §3's gate-cell exemption, and it is not a convenience: a null cell run
//! on fresh OS entropy fails at the nominal 5% rate BY CONSTRUCTION, and
//! (v)'s byte-identity pin would be unstatable. `SeedSchedule::diagnostic`'s
//! NAME is the waiver — the OS-entropy rule binds published evaluation runs,
//! which is what `examples/arena.rs --os-entropy` is for.
//!
//! WHAT IS *NOT* ASSERTED, and why. No cell asserts a DIRECTION — that
//! contestant X beats contestant Y. §0-R3(i) requires a real interval, not a
//! measured winner, and E1's paired-VP data says the retreat-averse-vs-weighted
//! pair may sit near null at 2P. A gate that asserted a direction would be
//! asserting a fact about the policies (measured under a different harness),
//! not a fact about the instrument. The directions this run actually measured
//! are PRINTED on every cell instead.
//!
//! TIER: release. The per-edit slices of these same properties already live in
//! `game/arena/stats.rs`'s unit tests (a 40-unit null cell, the refusal's four
//! arms, an R=200 synthetic coverage slice, the byte-identity pin) — this
//! binary is the full-scale tier, run by
//! `cargo test --release -p yokohama-core --test arena_gate -- --ignored`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use yokohama_core::{
    agent_stream_seed, bootstrap_percentile, corpus_path, emit_record, replay_strict,
    run_arena_game, run_cell, run_cell_emitting, unit_for, ArenaError, CellReport, CellSpec,
    GameRecord, HonestPathTally, IntervalOutcome, PolicyAgent, RolloutPolicy, Rules, SeedSchedule,
    SeedScheduleKind, SEMANTICS_REV,
};

/// This binary's committed diagnostic master (plan Task 9).
const MASTER: u64 = 0xA3E1_2026_0726_0001;

/// The master the synthetic coverage cell runs on — deliberately NOT this
/// binary's. See `synthetic_coverage_validates_the_interval_machinery`.
const SYNTHETIC_MASTER: u64 = 0xA3E1_2026_0726_0007;

/// The two-sided 95% normal quantile.
///
/// A DELIBERATE independent restatement, not a visibility workaround (T9
/// review, probe-2 ruling): a test importing its theoretical reference from
/// the module under test has a weaker check, and the estimator this gate
/// validates uses no z at all (nearest-rank order statistics), so the two
/// share nothing that can drift. 1.959964 changes only if the project
/// abandons 95%, in which case this gate SHOULD disagree loudly.
const Z_95: f64 = 1.959964;

/// The `FamilyWeighted` knob setting E1 measured APART from the shipped default
/// — p = 0.05 against the default's p = 1/6, both at k = 5. §0-R3(i)'s
/// guaranteed edge is manufactured from these two, which is the point: it is
/// built out of shipped machinery rather than a bespoke test policy.
const FW_LOW_P: RolloutPolicy = RolloutPolicy::FamilyWeighted { retreat_ppm: 50_000, recovery_min_of: 5 };

// ---------------------------------------------------------------------------
// Scale
// ---------------------------------------------------------------------------

/// Every size this binary runs at, in one place.
///
/// Two instances. [`Scale::GATE`] is the shipped release tier — the sizes spec
/// §0-R3 and the plan name, with the replicate block sized from
/// `examples/arena.rs throughput`'s measured per-game cost (see
/// `real_game_replicates_match_reported_se`).
///
/// [`Scale::SMOKE`] is development size, and it serves two purposes that are
/// worth keeping distinct. Every cell function takes a `Scale`, so a developer
/// can point ANY of them at `SMOKE` locally and get a run in seconds instead of
/// waiting on 400 games — that is why `SMOKE` carries a value for every field,
/// including fields no shipped test reads at that size. What SHIPS at smoke
/// size is the cheap subset (`the_gate_battery_runs_at_smoke_scale`); the four
/// statistical cells stay gate-only because their assertions are not
/// informative at three units and their games are what the cost is.
#[derive(Clone, Copy, Debug)]
struct Scale {
    /// Design units in the same-policy null cell (2P pairs).
    null_units: u32,
    /// Design units in each of the two edge cells (2P pairs).
    edge_units: u32,
    /// Design units in the bracket cell (2P pairs) — uniform seats, so these
    /// are the most expensive games in the binary.
    bracket_units: u32,
    /// Design units in the 2P and 4P determinism cells. `None` skips a cell —
    /// how the smoke tier drops its 4P games, which are ~4× a 2P game's cost
    /// in debug and whose block-specific paths (rotation seatings, `-r{arr}`
    /// filenames) already have per-edit coverage in `arena.rs`/`corpus.rs`.
    determinism_units_2p: u32,
    determinism_units_4p: Option<u32>,
    /// The honest-path counter check's grid: one game per (rung × count).
    counter_rungs: &'static [RolloutPolicy],
    counter_players: &'static [u8],
    /// Corpus cell sizes: 2P pairs and 4P rotation blocks (`None` skips the
    /// block cell, as above), and the floor the tier must clear — spec §12
    /// gate 1 asks the GATE tier for ≥ 12 records including one whole block.
    corpus_pairs: u32,
    corpus_blocks: Option<u32>,
    corpus_min_records: u32,
    /// Synthetic coverage: replicates per θ, units per replicate, bootstrap
    /// replicates.
    synthetic_r: u32,
    synthetic_n: usize,
    synthetic_b: u32,
    /// Real-game replicate block: how many replicate cells, and units in each.
    replicates: u32,
    replicate_units: u32,
}

impl Scale {
    /// The shipped release tier.
    const GATE: Self = Self {
        null_units: 200,
        edge_units: 200,
        bracket_units: 60,
        determinism_units_2p: 12,
        determinism_units_4p: Some(3),
        counter_rungs: &[
            RolloutPolicy::Uniform,
            RolloutPolicy::RetreatAverse,
            RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
        ],
        counter_players: &[2, 4],
        corpus_pairs: 5,
        corpus_blocks: Some(1),
        corpus_min_records: 12,
        synthetic_r: 2_000,
        synthetic_n: 200,
        synthetic_b: 400,
        // Sized from `examples/arena.rs throughput`'s own measurement, release,
        // at this HEAD: the `edge-ra-vs-fw` 2P cell shape — the exact shape a
        // replicate is — costs 3.1 ms/game, so 100 × 20 pairs = 4,000 games
        // ≈ 12.4s, which is the bulk of this binary's budget and leaves it
        // inside spec §12's ~40s target. R is what buys the band: the ratio of
        // two sds estimated from R replicates has sampling sd ≈ 1/√(2(R−1)),
        // so R = 100 gives ±0.213 at 3σ where R = 24 would give ±0.442. Spending
        // the budget on R rather than on units is deliberate — units per
        // replicate only need to be enough for a non-degenerate interval.
        replicates: 100,
        replicate_units: 20,
    };

    /// Development / per-edit plumbing size.
    const SMOKE: Self = Self {
        null_units: 3,
        edge_units: 2,
        bracket_units: 2,
        determinism_units_2p: 1,
        determinism_units_4p: None,
        // The weighted rung at 2P only: the smoke's subject is the plumbing,
        // and a uniform game is an order of magnitude more moves (measured at
        // this master: 2P uniform 5,625 applied moves against the weighted
        // rung's 253), which would put the per-edit tier's cost entirely in a
        // cell whose statistics it is not checking.
        counter_rungs: &[RolloutPolicy::FAMILY_WEIGHTED_DEFAULT],
        counter_players: &[2],
        corpus_pairs: 1,
        corpus_blocks: None,
        corpus_min_records: 2,
        synthetic_r: 40,
        synthetic_n: 40,
        synthetic_b: 40,
        replicates: 3,
        replicate_units: 2,
    };
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn diagnostic(units: u32, name: &str, players: u8, x: RolloutPolicy, y: RolloutPolicy) -> CellSpec {
    CellSpec {
        name: name.to_string(),
        players,
        x,
        y,
        units,
        master: SeedScheduleKind::Diagnostic(MASTER),
    }
}

fn run(spec: &CellSpec, rules: &Rules) -> CellReport {
    run_cell(spec, rules).unwrap_or_else(|e| panic!("cell {:?} failed: {e}", spec.name))
}

/// Sample standard deviation (n − 1 denominator).
fn sample_sd(values: &[f64]) -> f64 {
    assert!(values.len() >= 2, "sd over fewer than two values");
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let ss: f64 = values.iter().map(|v| (v - mean) * (v - mean)).sum();
    (ss / (values.len() - 1) as f64).sqrt()
}

/// Print a cell exactly as an evaluation run would read it — spec echo first,
/// so a figure scraped out of this binary's output carries its measurement
/// policy with it.
fn print_report(report: &CellReport) {
    println!("  {}", report.spec_echo);
    println!(
        "  theta_hat={:.6}  interval={}  seat0_wins={}/{}  ties_to_seat0={}  vp_margin_mean={:+.2}",
        report.theta_hat,
        describe_interval(&report.interval),
        report.seat0_wins,
        report.games,
        report.ties_to_seat0,
        report.vp_margin_mean,
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

/// Every `.json` under `dir`, recursively.
fn collect_json(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}")) {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            collect_json(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
            out.push(path);
        }
    }
}

// ---------------------------------------------------------------------------
// (ii) The null cell
// ---------------------------------------------------------------------------

/// §0-R3(ii): a cell where BOTH contestants run the same rung must read near
/// its null with a nonzero SE.
///
/// This is the instrument's self-test, and it is the test E2's common-random-
/// numbers trap fails loudly. Keying the play stream by seat alone makes the two
/// games of a pair the same game replayed: every pair reads exactly 0.5, the
/// unit vector is constant, and the interval collapses to a REFUSAL — an
/// instrument reporting "perfectly calibrated" while measuring nothing
/// (measured ρ = −1.0000, SE 0.000). The assertion that the interval is a real
/// `Bootstrap` with `hi > lo` is what catches that, and the sd assertion below
/// states the same thing about the input rather than the output.
///
/// THE 0.5-CONTAINMENT ASSERTION IS A FIXED-MASTER CHECK, deliberately. At a
/// fresh master it would fail at the nominal 5% rate, which is precisely spec
/// §3's argument for the gate-cell exemption: this cell asks "did the
/// instrument stay calibrated since the last commit", and the answer has to be
/// deterministic to mean anything.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test arena_gate -- --ignored"]
fn null_cell_is_honest() {
    null_cell_at(Scale::GATE);
}

fn null_cell_at(scale: Scale) {
    let rules = Rules::standard();
    let spec = diagnostic(
        scale.null_units,
        "null-ra-vs-ra",
        2,
        RolloutPolicy::RetreatAverse,
        RolloutPolicy::RetreatAverse,
    );
    println!("[null cell] same rung both seats — theta_hat must sit near 0.5 with real width");
    let report = run(&spec, &rules);
    print_report(&report);

    assert_eq!(report.games, 2 * scale.null_units, "a 2P unit is two games");
    assert_eq!(report.x_unit_shares.len(), scale.null_units as usize);
    assert_eq!(report.master, MASTER, "the gate ran on an unexpected master");

    let sd = sample_sd(&report.x_unit_shares);
    assert!(
        sd > 0.0,
        "the unit vector carried NO variation (sd={sd}) — the CRN trap, not a calibrated null",
    );

    match report.interval {
        IntervalOutcome::Bootstrap { lo, hi, .. } => {
            assert!(hi > lo, "a null cell must report a NONZERO width, got [{lo}, {hi}]");
            assert!(
                lo <= 0.5 && 0.5 <= hi,
                "the same-rung null 0.5 fell OUTSIDE the cell's own 95% interval [{lo}, {hi}] \
                 (theta_hat {:.6}) — at a committed master this is a regression, not a flake",
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
// (i) The edge cells
// ---------------------------------------------------------------------------

/// §0-R3(i): at least one non-degenerate cell reporting a REAL interval.
///
/// TWO cells, for two different reasons.
///
/// **Cell A — retreat-averse vs the shipped family-weighted default.** The
/// first candidate for a real ranking, and the one whose answer nobody knows:
/// E1's paired-VP data (dVP −0.50 ± 1.73 at 2P) suggests this pair may measure
/// near-null. §0-R3(i) ANTICIPATES that as a valid outcome, so this cell asserts
/// only that the instrument produced a real two-sided interval on it. The
/// measured direction is printed, never asserted — asserting one would be
/// asserting a fact about the policies, measured elsewhere, in a test whose
/// subject is the instrument.
///
/// **Cell B — the guaranteed edge, labelled diagnostic.** Two shipped
/// `FamilyWeighted` knob settings E1 measured apart (p = 0.05 vs p = 1/6, both
/// at k = 5). It exists so clause (i) has an outcome that does not depend on an
/// unknown, and it is manufactured entirely from shipped machinery — no bespoke
/// test policy. Its cell name carries `-diagnostic`, and that label is
/// ASSERTED: a diagnostic cell whose label got dropped would read like a
/// measured rung comparison.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test arena_gate -- --ignored"]
fn edge_cells_report_real_intervals() {
    edge_cells_at(Scale::GATE);
}

fn edge_cells_at(scale: Scale) {
    let rules = Rules::standard();

    println!("[edge cell A] retreat-averse vs family-weighted default (near-0.5 is a VALID outcome)");
    let a = run(
        &diagnostic(
            scale.edge_units,
            "edge-ra-vs-fw",
            2,
            RolloutPolicy::RetreatAverse,
            RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
        ),
        &rules,
    );
    print_report(&a);
    assert_non_degenerate(&a, "edge cell A");

    println!(
        "[edge cell B] family-weighted p=50000ppm vs p=166667ppm, both k=5 — the GUARANTEED edge \
         (diagnostic)"
    );
    let b_spec = diagnostic(
        scale.edge_units,
        "edge-fw-p50000-vs-fw-p166667-diagnostic",
        2,
        FW_LOW_P,
        RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
    );
    assert!(
        b_spec.name.contains("diagnostic"),
        "the manufactured edge cell must carry its diagnostic label (spec §0-R3(i))",
    );
    let b = run(&b_spec, &rules);
    print_report(&b);
    assert_non_degenerate(&b, "edge cell B");

    // RECORDED, never asserted: whether either cell's interval excludes the 2P
    // null. This is the figure a strength claim would quote, and printing it
    // beside the assertion is how the gate stays about the instrument.
    for (label, report) in [("A", &a), ("B", &b)] {
        if let IntervalOutcome::Bootstrap { lo, hi, .. } = report.interval {
            println!(
                "  edge cell {label}: interval {} 0.5 — theta_hat {:.6}",
                if hi < 0.5 || lo > 0.5 { "EXCLUDES" } else { "contains" },
                report.theta_hat,
            );
        }
    }
}

fn assert_non_degenerate(report: &CellReport, label: &str) {
    let sd = sample_sd(&report.x_unit_shares);
    assert!(sd > 0.0, "{label}: the unit vector was constant (sd={sd}) — nothing to resample");
    match report.interval {
        IntervalOutcome::Bootstrap { lo, hi, .. } => {
            assert!(hi > lo, "{label}: a non-degenerate cell must report a positive width");
            assert!(
                (0.0..=1.0).contains(&lo) && (0.0..=1.0).contains(&hi),
                "{label}: a win-share interval left [0, 1]: [{lo}, {hi}]",
            );
            assert!(
                lo <= report.theta_hat && report.theta_hat <= hi,
                "{label}: theta_hat {:.6} fell outside its own interval [{lo}, {hi}]",
                report.theta_hat,
            );
        }
        ref other => panic!("{label}: expected a real two-sided interval, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// (iv) The zero-width refusal, on real games
// ---------------------------------------------------------------------------

/// §0-R3(iv): the degenerate cell must produce a TYPED refusal with a one-sided
/// bound, never a printed ±0.000.
///
/// The bracket pair is spec §4's standing test vector because it is the
/// degenerate case that actually happened: retreat-averse beat uniform
/// 4000/4000 games at 2P (E2). X is seated as the retreat-averse contestant so
/// the degenerate vector is all-WON and the refusal carries the INFORMATIVE
/// rule-of-three bound `1 − 3/n`; the all-lost mirror is pinned synthetically in
/// `game/arena/stats.rs`, where it costs no games at all.
///
/// THE ASSERTION IS DISJUNCTIVE, ON PURPOSE — but not against flake: at the
/// committed diagnostic master with a fixed engine this outcome is
/// DETERMINISTIC (clause (v) asserts exactly that), so this test cannot flake.
/// What the disjunction buys is tolerance to an INTENTIONAL upstream change —
/// a new master, a policy tweak, an engine edit — without a brittle re-pin
/// (T9 review, probe-1 ruling; the per-edit synthetic refusal pin in
/// `stats.rs` covers all four refusal arms at zero game cost either way).
/// A single lost pair (59/60) leaves a unit vector with two distinct values,
/// which is no longer a refusal — so the near-boundary arm accepts it and
/// requires what SHOULD be true there instead: a saturated/near-constant read
/// carrying the report's own NEAR-BOUNDARY caveat row, which exists because
/// percentile-bootstrap coverage degrades toward the boundary (measured
/// 0.39–0.87 at θ = 0.99 for N = 50–200). Either arm proves the property the
/// clause is about — the instrument refuses to print false precision.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test arena_gate -- --ignored"]
fn the_bracket_pair_refuses_zero_width() {
    bracket_at(Scale::GATE);
}

fn bracket_at(scale: Scale) {
    let rules = Rules::standard();
    let spec = diagnostic(
        scale.bracket_units,
        "bracket-ra-vs-uniform",
        2,
        RolloutPolicy::RetreatAverse,
        RolloutPolicy::Uniform,
    );
    println!("[bracket cell] the preserved blowout — the standing zero-width test vector");
    let report = run(&spec, &rules);
    print_report(&report);

    let n = report.x_unit_shares.len();
    assert_eq!(n, scale.bracket_units as usize);

    match report.interval {
        IntervalOutcome::RefusedZeroWidth { one_sided_lower, method } => {
            assert!(
                report.x_unit_shares.iter().all(|&u| u == 1.0),
                "the refusal fired on a vector that was not all-won: {:?}",
                report.x_unit_shares,
            );
            let expected = 1.0 - 3.0 / n as f64;
            assert!(
                (one_sided_lower - expected).abs() < 1e-12,
                "expected the rule-of-three bound 1 - 3/{n} = {expected}, got {one_sided_lower}",
            );
            assert!(
                method.contains("rule-of-three"),
                "the all-won refusal must name its method, got {method:?}",
            );
            assert!(
                report.cross_checks.iter().any(|row| row.contains("zero-width refusal")),
                "a refusal must print its complementary bound row",
            );
        }
        IntervalOutcome::Bootstrap { lo, hi, .. } => {
            // The knife-edge arm. Accept it only where it is honest: the read
            // must actually BE near the boundary, and the report must have said
            // so itself.
            assert!(
                report.theta_hat >= 1.0 - 2.0 / n as f64,
                "the bracket cell read theta_hat {:.6} — that is not a near-boundary blowout, \
                 and a real two-sided interval there means the bracket stopped being the \
                 degenerate test vector this clause needs (re-derive the standing vector)",
                report.theta_hat,
            );
            assert!(
                report.cross_checks.iter().any(|row| row.contains("NEAR-BOUNDARY CAVEAT")),
                "a saturated interval [{lo}, {hi}] printed no near-boundary caveat — the \
                 instrument reported false precision, which is what §0-R3(iv) forbids",
            );
            println!(
                "  NOTE: the bracket landed off the exact blowout (theta_hat {:.6} over {n} units), \
                 so the refusal path was not exercised by THIS run; the near-boundary caveat arm \
                 held instead. The refusal itself stays pinned synthetically in stats.rs.",
                report.theta_hat,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// (v) Reproducibility
// ---------------------------------------------------------------------------

/// §0-R3(v): re-running with the same recorded master and spec reproduces the
/// tabulation BYTE-IDENTICALLY.
///
/// Both unit shapes, because they are different code paths: the 2P PAIR and the
/// 3–4P ROTATION BLOCK. The 4P cell also exercises the block seating loop, whose
/// per-arrangement `x_seat` lookup is the piece that would silently produce a
/// stable-but-wrong report if a rotation were ever dropped.
///
/// ANTI-VACUITY is load-bearing here: byte-identity is trivially true for a
/// report that does not depend on the master at all, so a different master must
/// move the bytes.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test arena_gate -- --ignored"]
fn reports_are_deterministic() {
    determinism_at(Scale::GATE);
}

fn determinism_at(scale: Scale) {
    let rules = Rules::standard();

    let cells = [Some((2u8, scale.determinism_units_2p)), scale.determinism_units_4p.map(|u| (4u8, u))];
    for (players, units) in cells.into_iter().flatten() {
        let spec = diagnostic(
            units,
            &format!("determinism-{players}p"),
            players,
            RolloutPolicy::RetreatAverse,
            RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
        );
        let first = serde_json::to_string(&run(&spec, &rules)).expect("a report serializes");
        let second = serde_json::to_string(&run(&spec, &rules)).expect("a report serializes");
        assert_eq!(
            first, second,
            "the same master produced two different {players}P reports",
        );
        println!("[determinism {players}P] {} bytes, identical across runs", first.len());

        let mut other = spec.clone();
        other.master = SeedScheduleKind::Diagnostic(MASTER ^ 0xFFFF_FFFF);
        let third = serde_json::to_string(&run(&other, &rules)).expect("a report serializes");
        assert_ne!(
            first, third,
            "the {players}P report did not depend on the master at all",
        );
    }
}

// ---------------------------------------------------------------------------
// Honest-path counters
// ---------------------------------------------------------------------------

/// Spec §4: over this gate's games, the sampled-path count equals the decision
/// count.
///
/// The property is anti-vacuity for the honest chain. A completed game proves
/// nothing on its own — the silent fallback to the TRUE legal list that
/// `TrivialHonestAgent` has, and that spec §4 forbids the arena from
/// inheriting, would also have completed it. What this asserts is that every
/// move that reached the board came out of a sampled world.
///
/// THERE IS NO FALLBACK COUNTER TO ASSERT AGAINST (T6 review): the arena's
/// honest chain either produces a move or the run dies with a typed
/// `SamplerFailed`, so "fallback == 0" is a structural fact rather than a
/// measurement, and counting it would suggest the arm exists.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test arena_gate -- --ignored"]
fn honest_play_counters_hold() {
    counters_at(Scale::GATE);
}

fn counters_at(scale: Scale) {
    let rules = Rules::standard();
    let mut schedule = SeedSchedule::diagnostic(MASTER);

    for &players in scale.counter_players {
        for &rung in scale.counter_rungs {
            let seed = schedule.next_world();
            let seating = unit_for(players, seed).seatings[0].clone();
            let tallies: Arc<Mutex<Vec<HonestPathTally>>> = Arc::new(Mutex::new(Vec::new()));
            let collected = Arc::clone(&tallies);

            let stats = run_arena_game(
                seed,
                players,
                &rules,
                &mut |seat, _n| {
                    let agent =
                        PolicyAgent::new(rung, agent_stream_seed(seed, seat, seating[seat.0 as usize]));
                    collected.lock().expect("tally lock").push(agent.tally());
                    Box::new(agent)
                },
                true,
            )
            .unwrap_or_else(|e| panic!("{players}P {} game failed: {e}", rung.as_str()));

            let per_seat: Vec<u64> =
                tallies.lock().expect("tally lock").iter().map(|t| t.sampled_decisions()).collect();
            assert_eq!(per_seat.len(), players as usize);
            assert!(per_seat.iter().all(|&n| n > 0), "a seat never played: {per_seat:?}");
            assert_eq!(
                per_seat.iter().sum::<u64>(),
                stats.moves_applied,
                "{players}P {}: a move reached the board without a sampled decision behind it",
                rung.as_str(),
            );
            assert_eq!(
                stats.volumes.len() as u64,
                stats.moves_applied,
                "one volume observation per applied move",
            );
            println!(
                "[counters] {players}P {:16} seed={:#018x} moves={} per-seat={per_seat:?}",
                rung.as_str(),
                seed.0,
                stats.moves_applied,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// (iii-a) Synthetic coverage — the interval machinery against KNOWN truth
// ---------------------------------------------------------------------------

/// §0-R3(iii)'s synthetic half, at FULL SCALE: does the percentile bootstrap
/// actually cover the truth it claims to?
///
/// Units are Bernoulli(θ) draws, so the true unit mean is θ EXACTLY and coverage
/// is countable. No games are played, which is what makes R ≥ 2,000 affordable
/// — and affordable at that R is what makes the coverage band narrow enough to
/// mean something.
///
/// WHY THIS CELL IS HERE AND NOT PER-EDIT. The same check runs per-edit in
/// `game/arena/stats.rs` at R = 200 (measured 6.7s debug). Full scale is a
/// release-tier figure because the estimator is ~57× slower unoptimized —
/// R = 2,000 × 2 θ costs ~133s in debug against ~2.3s in release — so the
/// choice is between a wide band per-edit and a narrow band at merge, and this
/// binary takes the narrow one.
///
/// WHY A DIFFERENT MASTER. This cell runs on `SYNTHETIC_MASTER`, the master the
/// per-edit slice uses, deliberately: it makes this the SAME cell at full scale
/// rather than a second, differently-seeded one, so the release figures that
/// slice records in its own doc are reproduced here rather than replaced.
///
/// THE COVERAGE BAND: the R = 2,000 binomial band around the nominal 0.95 is
/// 0.95 ± 3.29·√(0.95·0.05/2000) = [0.934, 0.966] at 99.9%, rounded OUT to
/// [0.93, 0.97]. The rounding is not slack for its own sake: percentile-bootstrap
/// coverage for a proportion is not exactly nominal even in principle, so a band
/// pinned to the binomial limits alone would be asserting the estimator is
/// perfect rather than calibrated.
///
/// ANTI-VACUITY IS LOAD-BEARING. A coverage band open at the top passes
/// trivially for an estimator that returns [0, 1] every time, so the mean
/// interval WIDTH is pinned to the theoretical `2·z·√(θ(1−θ)/N)` within ±8% as
/// well. Coverage alone would not catch a bootstrap that resampled with too
/// much spread; width alone would not catch one centred wrong. Both together
/// are what "the arithmetic is validated" means.
///
/// MUTATION-PROVEN (T9, reverted by inverse edit): replacing the estimator's
/// return with the trivially-wide `(0.0, 1.0)` drives coverage to 1.0000 and
/// the assertion FAILS on the band's upper edge — which is the direction a
/// one-sided "≥ 0.93" band would have missed entirely.
///
/// MEASURED AT HEAD (release, `SYNTHETIC_MASTER`, R=2,000 · N=200 · B=400):
/// coverage **0.9540 at θ=0.5** and **0.9600 at θ=0.6**, width ratios 0.9955
/// and 0.9957. Those coverage figures reproduce the per-edit slice's recorded
/// release numbers EXACTLY, which is what running the same master buys.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test arena_gate -- --ignored"]
fn synthetic_coverage_validates_the_interval_machinery() {
    synthetic_coverage_at(Scale::GATE);
}

fn synthetic_coverage_at(scale: Scale) {
    println!(
        "[synthetic coverage] R={} replicates/theta, N={} units, B={}, master={SYNTHETIC_MASTER:#018x}, \
         no games played",
        scale.synthetic_r, scale.synthetic_n, scale.synthetic_b,
    );

    for (t_index, theta) in [0.5f64, 0.6].into_iter().enumerate() {
        let mut rng = ChaCha8Rng::seed_from_u64(SYNTHETIC_MASTER ^ (t_index as u64 + 1));
        let mut covered = 0u32;
        let mut width_sum = 0.0f64;

        for _ in 0..scale.synthetic_r {
            let units: Vec<f64> = (0..scale.synthetic_n)
                .map(|_| if rng.gen_bool(theta) { 1.0 } else { 0.0 })
                .collect();
            let (lo, hi) = bootstrap_percentile(&units, scale.synthetic_b, rng.gen());
            if lo <= theta && theta <= hi {
                covered += 1;
            }
            width_sum += hi - lo;
        }

        let coverage = f64::from(covered) / f64::from(scale.synthetic_r);
        let nominal = 2.0 * Z_95 * (theta * (1.0 - theta) / scale.synthetic_n as f64).sqrt();
        let ratio = (width_sum / f64::from(scale.synthetic_r)) / nominal;
        println!(
            "  theta={theta}: coverage={coverage:.4} ({covered}/{}), mean width ratio={ratio:.4} \
             (nominal {nominal:.6})",
            scale.synthetic_r,
        );

        assert!(
            (0.93..=0.97).contains(&coverage),
            "theta {theta}: coverage {coverage:.4} outside the R={} band [0.93, 0.97]",
            scale.synthetic_r,
        );
        assert!(
            (0.92..=1.08).contains(&ratio),
            "theta {theta}: mean interval width is {ratio:.4}x the theoretical \
             2z*sqrt(theta(1-theta)/N) = {nominal:.6} — coverage alone cannot catch this",
        );
    }
}

// ---------------------------------------------------------------------------
// (iii-b) Real-game replicates — the clustering check only games can buy
// ---------------------------------------------------------------------------

/// §0-R3(iii)'s real half: does the SE the instrument reports match the spread
/// the instrument actually has, when the outcomes come from real, clustered
/// games?
///
/// The synthetic half above validates the ARITHMETIC on independent Bernoulli
/// units. It cannot see the one thing that makes this design non-trivial: the
/// games inside a unit share a world seed and a bit-identical starting state, so
/// they are correlated, and a reported SE computed as if they were not would be
/// too small. Only real games can test that, which is why this block exists and
/// why it is small — every replicate is a whole cell.
///
/// SIZING, from `examples/arena.rs throughput`'s own measurement (see the report
/// beside this task): the block is `REPLICATES × REPLICATE_UNITS` pairs of
/// retreat-averse-vs-family-weighted 2P games, chosen so the whole binary stays
/// inside spec §12's ~40s release target.
///
/// THE BAND IS χ²-DERIVED, NOT BINOMIAL, and the difference is not pedantry.
/// The synthetic half counts coverage successes, which is a binomial quantity.
/// This half compares two standard deviations, whose ratio has sampling sd
/// ≈ 1/√(2(R−1)) — so the band below is 1 ± 3/√(2(R−1)), a 3σ band, stated with
/// R in it. A binomial band here would be the wrong distribution for the
/// statistic being tested.
///
/// WHAT IS RECORDED RATHER THAN ASSERTED: the pooled-GAME SE, which is what a
/// naive analysis that ignored clustering would have reported. The gap between
/// it and the empirical sd is the clustering penalty in this cell. It is
/// printed, not gated, because its magnitude is a property of the policies and
/// player count (E2 measured ρ ≈ 0 at 2P, +0.051 at 4P), not of the instrument
/// — and at 2P, where ρ ≈ 0, the honest expectation is that it barely differs.
///
/// MUTATION-PROVEN (T9, reverted by inverse edit): halving the reported SE
/// drives the ratio to 2.0414 against a band of [0.7868, 1.2132] and the
/// assertion FAILS. An SE that is too small by a factor of two is exactly the
/// failure mode ignoring the design unit would produce.
///
/// MEASURED AT HEAD (release, R=100 × 20 pairs): empirical sd(θ̂) 0.075577,
/// mean reported SE 0.074045 (ratio 1.0207), unit-level cross-check 0.076682,
/// pooled-game 0.078081.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test arena_gate -- --ignored"]
fn real_game_replicates_match_reported_se() {
    replicates_at(Scale::GATE);
}

fn replicates_at(scale: Scale) {
    let rules = Rules::standard();
    let mut masters = SeedSchedule::diagnostic(MASTER);
    println!(
        "[replicate block] R={} replicate cells x {} pairs, retreat-averse vs family-weighted, 2P",
        scale.replicates, scale.replicate_units,
    );

    let mut theta_hats: Vec<f64> = Vec::with_capacity(scale.replicates as usize);
    let mut bootstrap_ses: Vec<f64> = Vec::with_capacity(scale.replicates as usize);
    let mut unit_ses: Vec<f64> = Vec::with_capacity(scale.replicates as usize);
    let mut pooled_ses: Vec<f64> = Vec::with_capacity(scale.replicates as usize);

    for r in 0..scale.replicates {
        // A DIFFERENT master per replicate — replicates must be independent
        // draws of the whole cell, not the same cell re-tabulated. They descend
        // from this binary's one committed master, so the block reproduces.
        let master = masters.next_world().0;
        let spec = CellSpec {
            name: format!("replicate-{r:02}"),
            players: 2,
            x: RolloutPolicy::RetreatAverse,
            y: RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
            units: scale.replicate_units,
            master: SeedScheduleKind::Diagnostic(master),
        };
        let report = run(&spec, &rules);

        theta_hats.push(report.theta_hat);
        unit_ses.push(sample_sd(&report.x_unit_shares) / (report.x_unit_shares.len() as f64).sqrt());
        match report.interval {
            IntervalOutcome::Bootstrap { lo, hi, .. } => {
                bootstrap_ses.push((hi - lo) / (2.0 * Z_95));
            }
            ref other => panic!(
                "replicate {r} was degenerate ({other:?}) — an SE comparison needs a real \
                 interval in every replicate; re-pick the policy pair or raise the units",
            ),
        }
        // The naive alternative, for the recorded comparison: the cell's games
        // treated as independent Bernoulli trials, which is what an analysis
        // that ignored the design unit would have reported.
        let games = f64::from(report.games);
        pooled_ses.push((report.theta_hat * (1.0 - report.theta_hat) / games).sqrt());
    }

    let empirical_sd = sample_sd(&theta_hats);
    let mean_bootstrap_se = bootstrap_ses.iter().sum::<f64>() / bootstrap_ses.len() as f64;
    let mean_unit_se = unit_ses.iter().sum::<f64>() / unit_ses.len() as f64;
    let mean_pooled_se = pooled_ses.iter().sum::<f64>() / pooled_ses.len() as f64;
    let ratio = empirical_sd / mean_bootstrap_se;

    // 3σ on the ratio of two sds estimated from R replicates.
    let tolerance = 3.0 / (2.0 * f64::from(scale.replicates - 1)).sqrt();
    let (lo, hi) = (1.0 - tolerance, 1.0 + tolerance);

    println!(
        "  empirical sd(theta_hat) over R={} replicates = {empirical_sd:.6}",
        scale.replicates,
    );
    println!("  mean reported SE (bootstrap half-width / 2z) = {mean_bootstrap_se:.6}");
    println!("  mean unit-level SE (sd(units)/sqrt(n))       = {mean_unit_se:.6}  [cross-check]");
    println!(
        "  mean pooled-GAME SE (ignores clustering)     = {mean_pooled_se:.6}  [RECORDED: a naive \
         analysis would report this; the gap to the empirical sd is this cell's clustering penalty]"
    );
    println!("  ratio = {ratio:.4}, band [{lo:.4}, {hi:.4}] = 1 +/- 3/sqrt(2(R-1))");

    assert!(
        (lo..=hi).contains(&ratio),
        "the reported SE does not match the spread the instrument actually has: empirical sd \
         {empirical_sd:.6} vs mean reported SE {mean_bootstrap_se:.6} (ratio {ratio:.4}, band \
         [{lo:.4}, {hi:.4}] at R={})",
        scale.replicates,
    );
    // The two SE estimators must agree with EACH OTHER far more tightly than
    // either agrees with the empirical sd — they are two readings of the same
    // unit vector, so a gap here is an arithmetic bug, not sampling noise.
    let cross = mean_bootstrap_se / mean_unit_se;
    assert!(
        (0.85..=1.15).contains(&cross),
        "the bootstrap half-width SE and the unit-level SE disagree by {cross:.4}x — these read \
         the same unit vector and should agree closely",
    );
}

// ---------------------------------------------------------------------------
// The corpus check
// ---------------------------------------------------------------------------

/// The gate clause spec §0 kept VERBATIM: "emitted records replay strict-clean".
///
/// This is what makes it verified rather than prose. A fresh small corpus is
/// emitted through the real path — `run_cell_emitting` → `corpus_path` →
/// `emit_record`, the invocation `.gitignore`'s corpus comment points at — into
/// a TEMPORARY directory (never the repo's own corpus dir), and then EVERY file
/// is read back off disk, PARSED, and replayed through `replay_strict`. Parsed,
/// not counted: a truncated file from a crashed sweep is exactly what this
/// catches, and a count would not.
///
/// `replay_strict` is the closing authority spec §4 names, because it pushes
/// every recorded move through the CHECKED `apply_move` boundary, runs
/// `check_invariants` after every event, and verifies the terminal digest — so
/// a non-canonical witness that `PathMode::Canonical` accepted would die here
/// even though the debug-tier membership assert is compiled out in release.
///
/// FILENAME UNIQUENESS is asserted twice over, and the second one is the real
/// check: the set of paths the emitter was ASKED for, and the number of files
/// actually on disk. `fs::write` truncates, so two games filed at one path lose
/// one silently — a corpus that is short by a game with no error anywhere.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test arena_gate -- --ignored"]
fn emitted_corpus_replays_strict_clean() {
    corpus_at(Scale::GATE, "gate");
}

fn corpus_at(scale: Scale, label: &str) {
    let rules = Rules::standard();
    // The LABEL is part of the directory name (T9 review F1): keyed by pid
    // alone, the GATE corpus test and the SMOKE plumbing test shared one
    // directory and clobbered each other under `--include-ignored`'s
    // concurrent harness — neither documented command hits that, but the
    // "run everything" invocation should not fail pointing at the emitter.
    let dir = std::env::temp_dir()
        .join(format!("yokohama_arena_gate_corpus_{label}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);

    let cells = [
        Some(("gate-ra-vs-fw", 2u8, scale.corpus_pairs)),
        scale.corpus_blocks.map(|units| ("gate-fw-selfplay", 4u8, units)),
    ];
    let mut asked: BTreeSet<PathBuf> = BTreeSet::new();
    let mut emitted = 0u32;

    for (name, players, units) in cells.into_iter().flatten() {
        let spec = CellSpec {
            name: name.to_string(),
            players,
            x: RolloutPolicy::RetreatAverse,
            y: RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
            units,
            master: SeedScheduleKind::Diagnostic(MASTER),
        };
        run_cell_emitting(&spec, &rules, true, &mut |stats, arr| {
            let path = corpus_path(&dir, SEMANTICS_REV, players, name, stats.seed, arr);
            asked.insert(path.clone());
            emitted += 1;
            emit_record(&path, &stats.record)
                .map_err(|e| ArenaError::Driver(format!("emit {path:?}: {e}")))
        })
        .unwrap_or_else(|e| panic!("emitting cell {name} failed: {e}"));
    }

    assert_eq!(
        asked.len() as u32,
        emitted,
        "two games were filed at one corpus path — fs::write truncates, so one was lost",
    );
    assert!(
        emitted >= scale.corpus_min_records,
        "the corpus check must exercise at least {} records at this tier (spec §12 gate 1 asks \
         the gate tier for 12, including one whole rotation block), got {emitted}",
        scale.corpus_min_records,
    );

    let mut files = Vec::new();
    collect_json(&dir, &mut files);
    assert_eq!(
        files.len() as u32,
        emitted,
        "{} files on disk for {emitted} emitted records",
        files.len(),
    );
    files.sort();

    for path in &files {
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let record: GameRecord = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("parse {path:?}: {e} (a truncated file reads like this)"));

        // Provenance survived the round trip. The run half is the one this
        // task wires up (`run_cell_emitting` sets it), so a record that
        // replays but carries no cell or master would be an unlabelled game in
        // training data — worse than no game, because it is indistinguishable
        // from a labelled one.
        let run = record.run.as_ref().unwrap_or_else(|| panic!("{path:?} carries no provenance"));
        assert_eq!(run.master_seed, Some(MASTER), "{path:?}: wrong master in provenance");
        assert!(run.run_id.is_some(), "{path:?}: no cell named in provenance");
        assert!(!run.seats.is_empty(), "{path:?}: no seat provenance");
        for seat in &run.seats {
            assert_eq!(
                seat.observation, "view",
                "{path:?}: an honest-driver seat must be labelled \"view\", got {:?}",
                seat.observation,
            );
        }

        replay_strict(&record, &rules)
            .unwrap_or_else(|e| panic!("{path:?} failed strict replay at event {}: {}", e.event_index, e.message));
    }

    // ANTI-VACUITY, and the reason this check PARSES rather than counts: a file
    // that a crashed sweep left half-written must not read as a corpus entry.
    // Proven rather than asserted in prose — take a good record, cut it in half,
    // and require the reader to refuse it.
    let good = std::fs::read_to_string(&files[0]).expect("re-read an emitted record");
    let truncated = &good[..good.len() / 2];
    assert!(
        serde_json::from_str::<GameRecord>(truncated).is_err(),
        "a half-written record parsed as a whole one — the corpus check's parse step is doing \
         nothing, and a crashed sweep would file torn games as training data",
    );

    println!(
        "[corpus] {emitted} records emitted, parsed and strict-replayed from {:?}",
        dir.file_name().unwrap_or_default(),
    );
    std::fs::remove_dir_all(&dir).expect("the temporary corpus must be removable");
}

// ---------------------------------------------------------------------------
// Per-edit plumbing smoke
// ---------------------------------------------------------------------------

/// The cheap half of the battery at [`Scale::SMOKE`] — the one test in this
/// binary that is NOT `#[ignore]`d.
///
/// It asserts nothing the release tier does not; its subject is this harness's
/// own plumbing. A release-only binary rots invisibly: a panic in a helper, an
/// emitter path that moved, a temp directory that stopped being removable,
/// would all be found at merge time rather than at edit time. Measured 0.8s
/// debug, which is the cheapest available insurance against a gate that has
/// quietly stopped being runnable.
///
/// WHAT IT RUNS, and what it deliberately does not: the estimator's totality,
/// the honest-path counters, the whole emit → parse → `replay_strict` corpus
/// path, and the determinism pin at 2P. It skips the null, edge, bracket, and
/// replicate cells, because those are *statistical* checks whose assertions
/// carry no information at three units — running them tiny would buy a slower
/// per-edit suite and a false sense that the gate had been exercised. The 4P
/// cells are skipped for cost alone (a 4P game is ~4× a 2P game in debug); the
/// block-specific paths they would cover — rotation seatings, `-r{arr}`
/// filenames — have their own per-edit unit tests in `arena.rs` and
/// `corpus.rs`.
#[test]
fn the_gate_battery_runs_at_smoke_scale() {
    let scale = Scale::SMOKE;
    synthetic_coverage_smoke(scale);
    counters_at(scale);
    corpus_at(scale, "smoke");
    determinism_at(scale);
}

/// The synthetic cell at smoke scale asserts only that the estimator returns
/// finite, ordered endpoints — the R = 40 coverage band would be [0.6, 1.0],
/// which is not a check, and pretending otherwise is exactly the vacuous
/// assertion this branch keeps deleting.
fn synthetic_coverage_smoke(scale: Scale) {
    let mut rng = ChaCha8Rng::seed_from_u64(SYNTHETIC_MASTER);
    for _ in 0..scale.synthetic_r {
        let units: Vec<f64> =
            (0..scale.synthetic_n).map(|_| if rng.gen_bool(0.5) { 1.0 } else { 0.0 }).collect();
        let (lo, hi) = bootstrap_percentile(&units, scale.synthetic_b, rng.gen());
        assert!(lo.is_finite() && hi.is_finite() && lo <= hi, "bad interval [{lo}, {hi}]");
    }
}
