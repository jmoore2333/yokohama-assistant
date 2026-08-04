//! The architecture gate — Phase 3.4, spec
//! `docs/plans/2026-08-04-phase3.4-search-arch-spec.md` §8.
//!
//! WHAT THIS BINARY IS FOR. Phase 3.4 adds a whole ARCHITECTURE to the arena —
//! `Contestant::MctsV1`, a UCT tree over one sampled world — plus a shared
//! scored-successor budget the capped S1 arm is sized by. Spec §8 registers
//! three release cells as roster gate #14, and this binary is those three made
//! executable:
//!
//! | cell | contestants (X vs Y) | N (units) | asserts |
//! |---|---|---|---|
//! | determinism | `mcts-v1@750` vs `lookahead-v1:b=4,cap=750` | 12, run twice | byte-identical `CellReport` + flipped-master anti-vacuity |
//! | honest counters | the same C2 shape, smaller | 8 | `x_explanations == Some(x_applied_moves)`, the x/y budget meters, the cap-counter shape |
//! | mcts null | `mcts-v1@750` vs ITSELF | 24 | nonzero SE / non-degenerate unit shares — containment PRINTED |
//!
//! **NO CELL IN THIS BINARY ASSERTS A DIRECTION** (spec §8, verbatim). That is
//! `arena_gate`'s discipline rather than `evaluator_gate`'s: 3.3's promotion cell
//! asserts a direction because the map's 3.3 gate text literally reads "beats",
//! and 3.4's gate text does not — the architecture question is settled by the
//! pre-registered experiment (spec §5, §0.1) whose cells are close-time
//! `examples/arena` commands recorded in the ADR, deliberately NOT roster tests.
//! A roster cell asserting `mcts-v1` beats the capped S1 arm would be freezing an
//! adjudication this cycle has not made, in the binary that is supposed to notice
//! it changing.
//!
//! ## Why the null cell is a GATE clause and not a nicety
//!
//! Spec §8 (lens 1 S11) states the reason and it is the one clause here that
//! would be easy to drop for cost: without it the roster gains a contestant with
//! **no detector for a common-random-numbers or keying collapse**. Such a
//! collapse reproduces byte-identically — so the determinism cell stays green —
//! and prints a healthy-looking θ̂ near the null, so a reader sees nothing. The
//! `sd > 0` and distinct-shares assertions are what make the null cell an
//! instrument rather than decoration. It is the standing self-test shape map row
//! 14 requires of every new contestant, and it is direction-free by construction
//! (a same-contestant cell has no direction to claim).
//!
//! CONTAINMENT of 0.5 is **printed and never asserted at N = 24**. `evaluator_gate`
//! asserts containment at N = 200, where a fixed master makes it a regression
//! question; at 24 units the interval is ~0.25 wide and containment carries almost
//! no information, while a fresh master would fail it at the nominal rate. The
//! shape being gated here is degeneracy, not calibration.
//!
//! ## Masters — and why this binary carries TWO
//!
//! [`GATE_MASTER`] = `0xA3E1_2026_0804_0004` is the release cells' committed
//! diagnostic master: the arena `A3E1` family, date field `2026_0804`, serial
//! `0004` — the first free serial in that family at authoring, grep-verified
//! against every `0xA3E1…` literal in the repo (`0804_0001`–`0804_0003` are
//! `tests/contestant_golden.rs`'s three T0 goldens; 3.2 and 3.3 hold the
//! `0726_*`/`0727_*` serials; `0803_0001`/`0002` are the research notes'
//! harnesses). **Chosen by serial ORDER before any cell was run** — the
//! `evaluator_gate` rule, and here it matters for a different reason than there:
//! no cell asserts a direction, but the null cell asserts non-degeneracy and the
//! determinism cell pins bytes, so a master picked after seeing which one made
//! the shares look healthiest would be master shopping at one remove.
//!
//! Every cell runs single-threaded on that one master under spec §3's gate-cell
//! exemption, inherited from 3.2/3.3 verbatim: an OS-entropy null cell fails at
//! its nominal rate BY CONSTRUCTION and the determinism cell would be unstatable.
//!
//! [`MASTER`] is the PER-EDIT smoke's own, and it predates this task (T3). It is
//! deliberately left where it is: the smoke's assertions (`binds > 0`, a whole
//! capped game completing at cap 8) are recorded against that master, and
//! re-pointing a green per-edit pin at a new seed for tidiness would churn
//! evidence to buy nothing. `arena_gate` carries two masters on the same
//! reasoning. **One observation left as an observation:** `MASTER`'s value is
//! also `mcts.rs`'s `SALT_MCTS_STREAM` literal. Two different namespaces, no
//! functional interaction (a salt is mixed into a stream key, never compared to a
//! master), but a reader grepping the constant will find both.
//!
//! ## Tier
//!
//! Release, three cells, `#[ignore]`d:
//! `cargo test --release -p yokohama-core --test architecture_gate -- --ignored --test-threads=1`.
//! The fourth test, [`the_architecture_battery_runs_at_smoke_scale`], is spec
//! §8's per-edit plumbing smoke and is **never `#[ignore]`d**, on `arena_gate`'s
//! and `evaluator_gate`'s stated grounds: a binary whose only tests are
//! release-only rots invisibly between the day it is written and the day the gate
//! first runs. It drives the two 3.4 ARMS directly (the MCTS agent through its own
//! honest chain, the capped S1 arm through the public arena driver) AND runs one
//! tiny cell through this file's own helper layer, so neither half of the binary
//! is release-only.
//!
//! ## Measured at HEAD
//!
//! Release, Apple M5, quiet machine, binaries pre-built, `--test-threads=1` (part
//! of the documented command, so the recorded figure is the one the command
//! produces). Per-cell walls come from running each cell ALONE; the total is a
//! separate whole-binary run, because a sum of per-command times is not a
//! measured total:
//!
//! ```text
//! null (mcts self-play)   48 games   18.69 s   389.4 ms/game
//! determinism             72 games   10.99 s   (2 asserted runs + 1 anti-vacuity run)
//! honest counters         16 games    2.69 s   168.1 ms/game
//! sum of the three                   32.37 s
//! WHOLE BINARY, one sequence         32.52 / 32.58 / 32.52 s across three runs, 136 games
//! ```
//!
//! Against spec §8's estimate of ~30–60 s: **~32.5 s, at the low end.** The null
//! cell is 58% of it and the reason is one number — an mcts-vs-mcts game costs
//! **389.4 ms** against the C2 shape's **168.1 ms**, because both seats run the
//! tree. Anyone sizing another mcts-vs-mcts cell should size it off 389 ms/game
//! and not off the C2 rate.
//!
//! The per-edit smoke measures **1.53 s debug** (1.55 / 1.53 / 1.52 across three
//! runs), of which arm 3's one smoke-scale cell is ~1.2 s — the price of not
//! leaving this file's helper layer release-only, paid once per edit.

use std::sync::{Arc, Mutex};

use yokohama_core::game::greedy::DEFAULT_RETREAT_SAMPLES;
use yokohama_core::game::lookahead::{LookaheadAgent, DEFAULT_BEAM, DEFAULT_WORLDS};
use yokohama_core::game::mcts::{
    MctsAgent, LeafRule, DEFAULT_EXPLORATION_PPM, DEFAULT_SQUASH_SCALE,
};
use yokohama_core::types::{GameConfig, Move, PlayerId};
use yokohama_core::{
    agent_stream_seed, apply_move_unchecked, finalize_setup, new_game, observe, run_cell,
    run_arena_game_capped, ArenaAgent, ArenaGameStats, CellReport, CellSpec, Contestant,
    IntervalOutcome, RecordedEvent, Rules, SeedScheduleKind, Slot, StartingOrderChoice, WorldSeed,
};

/// The PER-EDIT smoke's committed diagnostic master (spec §3's gate-cell
/// exemption). See the module doc's masters paragraph for why this is not
/// [`GATE_MASTER`].
const MASTER: u64 = 0x3C15_2026_0804_0001;

/// The RELEASE cells' committed diagnostic master — arena family, first free
/// serial at authoring, chosen by serial order before any cell ran. Module doc.
const GATE_MASTER: u64 = 0xA3E1_2026_0804_0004;

/// The experiment's budget unit, B\* (spec §5): 750 TOTAL scored successors per
/// decision, the capped S1 arm's own-states realized spend. Both sides of the
/// C2 shape are sized by it, which is what makes the cell node-matched.
const B_STAR: u32 = 750;

/// The 2P null. Every cell here is 2P; the report's cross-checks print it per
/// cell rather than trusting this constant.
const NULL_2P: f64 = 0.5;

/// Decisions the smoke's MCTS arm takes. Bounded rather than a whole game: the
/// point is that the wiring works and the meters move, and a full MCTS game at
/// B\* is release-tier work.
const MCTS_DECISIONS: u32 = 20;

/// The smoke's capped-S1 arm's cap. **8, and deliberately not a bigger one**:
/// spec §5 records a MEASURED non-terminating band at caps ≈32–128 for this arm,
/// so a smoke sitting there would fail for a reason the experiment's
/// completion-failure surface already owns. At 8 the arm is starved at every
/// large decision and completes (T1: 5/5 masters, 56–260 turns).
const S1_CAP: u32 = 8;

// ---------------------------------------------------------------------------
// Scale
// ---------------------------------------------------------------------------

/// Every size this binary runs at, in one place — `arena_gate`'s and
/// `evaluator_gate`'s pattern, and for the same two reasons.
///
/// [`Scale::GATE`] is the shipped release tier: spec §8's three cells at the Ns
/// it names.
///
/// [`Scale::SMOKE`] is development size. Every cell function takes a `Scale`, so
/// a developer can point ANY of them at `SMOKE` and get a run in seconds instead
/// of waiting on 48 MCTS games — which is why `SMOKE` carries a value for every
/// field. What SHIPS at smoke size is the counters cell at one unit: it is the
/// cell whose assertions still MEAN something at N = 1 (an identity and a set of
/// per-side meter relations are not statistical), and running it is what keeps
/// this file's helper layer from being release-only.
///
/// **The SMOKE caps are tiny and that is the whole reason the smoke is
/// affordable.** A cap of 750 buys ~145 ms/game in release and debug runs the
/// search an order of magnitude slower; caps of 24/8 put the same code paths
/// under a budget that binds immediately. Y's smoke cap is 8 rather than
/// anything in 32–128 for [`S1_CAP`]'s stated reason.
#[derive(Clone, Copy, Debug)]
struct Scale {
    /// Design units (2P pairs) in each run of the determinism cell.
    determinism_units: u32,
    /// Design units in the honest-counters cell.
    counter_units: u32,
    /// Design units in the MCTS self-null.
    null_units: u32,
    /// The MCTS arm's scored-successor cap.
    x_cap: u32,
    /// The capped-S1 arm's scored-successor cap.
    y_cap: u32,
}

impl Scale {
    /// Spec §8's cells.
    const GATE: Self = Self {
        determinism_units: 12,
        counter_units: 8,
        null_units: 24,
        x_cap: B_STAR,
        y_cap: B_STAR,
    };

    /// Development / per-edit plumbing size.
    const SMOKE: Self =
        Self { determinism_units: 1, counter_units: 1, null_units: 2, x_cap: 24, y_cap: 8 };
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// MCTS-v1 at the shipped knobs, sized by `cap` — spec §5's `mcts-v1@<cap>`.
///
/// The three screening knobs (`c`, `scale`, `leaf`) are their shipped defaults
/// here and NOT the screening grid's selection: this binary regresses the
/// architecture's plumbing, and pinning it to whatever §4's screening selects
/// would make a roster gate move whenever a pre-registration is amended.
fn mcts_at(cap: u32) -> Contestant {
    Contestant::MctsV1 {
        cap: Some(cap),
        c_ppm: DEFAULT_EXPLORATION_PPM,
        scale: DEFAULT_SQUASH_SCALE,
        m: DEFAULT_RETREAT_SAMPLES,
        leaf: LeafRule::HandOff,
    }
}

/// The capped S1 arm at the experiment's operating point — `lookahead-v1:b=4`
/// under the SAME scored-successor budget, which is what node-matching means.
fn capped_s1_at(cap: u32) -> Contestant {
    Contestant::LookaheadV1 {
        b: DEFAULT_BEAM,
        k: DEFAULT_WORLDS,
        m: DEFAULT_RETREAT_SAMPLES,
        cap: Some(cap),
    }
}

/// Every cell in this binary is 2P on [`GATE_MASTER`], so the spec is assembled
/// at one site and the contestants are the only thing a cell chooses.
fn cell(units: u32, name: &str, x: Contestant, y: Contestant) -> CellSpec {
    CellSpec {
        name: name.to_string(),
        players: 2,
        x,
        y,
        units,
        master: SeedScheduleKind::Diagnostic(GATE_MASTER),
    }
}

/// The C2 SHAPE (spec §5's primary cell): MCTS-v1 as X against the capped S1 arm
/// as Y, both sized by the same budget. Two cells here run it — determinism and
/// counters — at their own Ns and under their own names.
fn c2_shaped(units: u32, name: &str, scale: Scale) -> CellSpec {
    cell(units, name, mcts_at(scale.x_cap), capped_s1_at(scale.y_cap))
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

/// Print a cell exactly as an evaluation run would read it — spec echo FIRST, so
/// a figure scraped out of this binary's output carries its measurement policy
/// with it, then the estimate, then spec §0.3's TWO-AXIS budget block (node axis
/// only; the WALL axis is not on `CellReport` at all and is `examples/arena`'s
/// business, per `CellWall`'s doc).
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
    println!(
        "  x_ply_cap_hits={:?}  x_free_cap_hits={:?}",
        report.x_ply_cap_hits, report.x_free_cap_hits,
    );
    println!(
        "  X budget: scored={:?} root={:?} binds={:?} starved={:?}",
        report.x_candidates_scored,
        report.x_root_scored,
        report.x_budget_binds,
        report.x_budget_starved_decisions,
    );
    println!(
        "  Y budget: scored={:?} root={:?} binds={:?} starved={:?}",
        report.y_candidates_scored,
        report.y_root_scored,
        report.y_budget_binds,
        report.y_budget_starved_decisions,
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

/// The checks every cell in this binary owes regardless of what it measures.
///
/// Spec §0.3(b)'s explanation identity rides here rather than in one cell for
/// `evaluator_gate`'s reason: it is free once a cell has run, and `Some(n)` vs
/// `None` is the load-bearing distinction — an MCTS X that silently stopped
/// explaining would report `Some(0)`, and the whole equality catches it without a
/// second guard.
///
/// **The cap-counter SHAPE is `(Some, None)` for MCTS-v1 and that is not the
/// shape `evaluator_gate` asserts.** The tree HAS a ply guard — its hand-off
/// completions', the same `MAX_TURN_PLIES` constant, with the same meaning (a
/// line stopped there is valued mid-turn) — and it has NO quiescence arm to
/// enter at all, so a free-cap figure would be reporting on machinery that does
/// not exist. `evaluator_gate::x_searches` panics on an MCTS X precisely to route
/// such a cell here; this is the arm it routes to.
fn assert_cell_plumbing(report: &CellReport, spec: &CellSpec) {
    assert_eq!(report.games, 2 * spec.units, "a 2P unit is two games");
    assert_eq!(
        report.x_unit_shares.len(),
        spec.units as usize,
        "one unit share per design unit",
    );
    assert_eq!(report.master, GATE_MASTER, "the cell ran on an unexpected master");
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
    assert_eq!(
        report.x_explanations,
        Some(report.x_applied_moves),
        "spec §0.3(b): X played a move it did not explain (or explained one it did not play)",
    );
    assert_eq!(
        (report.x_ply_cap_hits, report.x_free_cap_hits),
        (Some(0), None),
        "MCTS-v1's cap-counter shape (T5) is (Some, None): a completion the ply guard stopped is \
         valued MID-TURN, and a tree has no quiescence arm whose guard could report anything",
    );
    // Every cell here is Keep-homogeneous: MCTS-v1 joins `KeepKind::EvaluatorScored`
    // (spec §3.6), so a unit's two seatings start bit-identical against BOTH the
    // capped S1 arm and itself. The null cell's self-test rests on it.
    assert!(
        report.spec_echo.contains("setup-identical=yes"),
        "every cell in this binary is Keep-homogeneous and must say so: {}",
        report.spec_echo,
    );
    assert!(
        report.spec_echo.contains("x=mcts-v1@v1{"),
        "the contestant under test must be the tree's own descriptor: {}",
        report.spec_echo,
    );
}

/// One side's declared budget-meter expectation for a CAPPED contestant, in the
/// relations the T5 per-edit seam test pins (`stats.rs`:
/// `an_mcts_x_cell_counts_every_move_it_explained_and_meters_both_sides`).
///
/// All four counters are `Some` because both arms of the C2 shape have a budget;
/// the relations are the ones that carry content:
///
/// - `0 < root < scored` — the MANDATORY root pass is a real fraction of a real
///   total. `root == 0` means no root pass ran; `root == scored` means nothing
///   deeper ever started, which is the starved degenerate the cap band exists to
///   keep out of gate cells.
/// - `binds > 0` — ANTI-VACUITY. A cap that never binds is not a matched budget
///   (spec §0.3), so a cell whose meters all read "no denial" is not measuring
///   the thing this gate is about.
/// - `starved <= binds` — starvation is a SUB-CASE of binding (spec §0.1's S2
///   surface routes on it), so the containment is a shape claim about the two
///   counters and holds whatever the rates are.
fn assert_capped_side(
    side: &str,
    scored: Option<u64>,
    root: Option<u64>,
    binds: Option<u64>,
    starved: Option<u64>,
) -> (u64, u64) {
    let scored = scored.unwrap_or_else(|| panic!("{side} is capped and must meter its spend"));
    let root = root.unwrap_or_else(|| panic!("{side} must report its root share"));
    let binds = binds.unwrap_or_else(|| panic!("{side} is capped and must report bind decisions"));
    let starved =
        starved.unwrap_or_else(|| panic!("{side} is capped and must report starved decisions"));
    assert!(scored > 0, "{side}: the currency meter counted nothing");
    assert!(
        root > 0 && root < scored,
        "{side}: the mandatory root pass must be a real fraction of a real total ({root}/{scored})",
    );
    assert!(binds > 0, "{side}: anti-vacuity — a cap that never binds is not a matched budget");
    assert!(starved <= binds, "{side}: starvation is a sub-case of binding ({starved} > {binds})");
    (scored, root)
}

// ---------------------------------------------------------------------------
// (i) Determinism — the C2 shape at N = 12, twice
// ---------------------------------------------------------------------------

/// **Spec §8 cell (i).** The C2 shape at N = 12, run twice, byte-identical
/// `CellReport` serialization, with the flipped-master anti-vacuity control.
///
/// The C2 SHAPE rather than a cheap one, because the property under test is that
/// the cell the EXPERIMENT will run reproduces. MCTS-v1 adds two whole randomness
/// surfaces the 3.3 pins never saw — a per-decision keyed tie stream and a
/// per-node candidate seed — and the capped S1 arm adds a third (T1's
/// continuation-order tie site). A cheaper opponent would leave the third
/// unexercised at gate scale.
///
/// ANTI-VACUITY, and it costs a third run: byte-identity is trivially true for a
/// report that does not depend on the master at all. The whole-string
/// `assert_ne!` alone is WEAK — `spec_echo` carries the master, so it would pass
/// on the echo bytes with the play frozen — so the stronger check is on a
/// PLAY-DERIVED figure (`vp_margin_mean`, a mean over the cell's games). The
/// pattern and the reasoning are `evaluator_gate`'s, verbatim. Spec §8's "run
/// twice" sizes the ASSERTED pair; this third run is declared as an addition.
///
/// MUTATION-PROVEN (run at authoring, reverted by inverse edit — L7): pointing the
/// control run at [`GATE_MASTER`] itself instead of the flipped value FAILS the
/// anti-vacuity check ("the report did not depend on the master at all").
///
/// **THE MEASURED OUTCOME OF THIS CELL IS NOT THE EXPERIMENT'S ANSWER, and the
/// distinction matters enough to state here rather than only in the ADR.** At
/// HEAD the C2 shape's unit vector is ALL-LOST (θ̂ 0.000000, 24/24 games to the
/// capped S1 arm, mean VP margin −92.08, typed zero-width refusal). This cell
/// asserts NO direction and that figure is a fact about two contestants at
/// SHIPPED knobs, at N = 12, on THIS binary's master — spec §5's C2 runs at
/// N = 200 on the pre-registered adjudication family with a §4-screened config,
/// and §0.1 reads the decision off that. Quoting this line as the architecture
/// result would be quoting a determinism fixture.
///
/// ONE CONSEQUENCE OF THAT DEGENERACY, recorded because it scopes the pin: with
/// the interval REFUSED, the bootstrap RESAMPLE draw is not inside this cell's
/// byte-identity claim (the resample SEED still is — `spec_echo` carries it). The
/// resampler's own determinism is pinned where it produces real intervals:
/// `arena_gate::reports_are_deterministic` at both unit shapes, and the null cell
/// below, which does report a `Bootstrap`.
///
/// MEASURED AT HEAD: 24 games per run, **2,336 bytes** identical across the two
/// asserted runs; the flipped-master control moves `vp_margin_mean`
/// **−92.08 → −75.46**. See the module doc's walls block for timings.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test architecture_gate -- --ignored --test-threads=1"]
fn the_mcts_c2_cell_reproduces_byte_identically() {
    determinism_at(Scale::GATE);
}

fn determinism_at(scale: Scale) {
    let spec = c2_shaped(scale.determinism_units, "determinism-c2-mcts-vs-capped-s1", scale);
    println!("[determinism] the C2 shape at N={} units, run twice", scale.determinism_units);

    let first_report = run(&spec);
    print_report("determinism", &first_report);
    assert_cell_plumbing(&first_report, &spec);
    let first = serde_json::to_string(&first_report).expect("a report serializes");
    let second = serde_json::to_string(&run(&spec)).expect("a report serializes");
    assert_eq!(first, second, "the same master produced two different reports");
    // The new meters are inside the bytes the identity covers — without this the
    // claim could be satisfied by a report that dropped them.
    assert!(
        first.contains("\"x_candidates_scored\":") && first.contains("\"y_candidates_scored\":"),
        "the byte-identity claim must cover spec §6's budget meters on BOTH sides: {first}",
    );
    println!("  {} bytes, identical across runs", first.len());

    let mut other = spec.clone();
    other.master = SeedScheduleKind::Diagnostic(GATE_MASTER ^ 0xFFFF_FFFF);
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
// (ii) Honest counters — the C2 shape, both sides metered
// ---------------------------------------------------------------------------

/// **Spec §8 cell (ii).** Spec §0.3(b)'s explanation identity for an MCTS X, and
/// spec §6's `x_*`/`y_*` budget meters on BOTH sides of one node-matched cell,
/// at gate scale.
///
/// A DEDICATED cell rather than a reading of the determinism cell's report, and
/// smaller than it: the two claims are independent (one is about reproduction,
/// one about accounting), and running them separately means a change that breaks
/// the meters cannot be masked by a determinism cell that happens to be skipped.
/// N = 8 because none of these assertions is statistical — an identity and a set
/// of per-side relations are as true at 8 units as at 200, and the games are what
/// the cell costs.
///
/// The per-edit tier already pins this at ONE unit with tiny caps
/// (`stats.rs::an_mcts_x_cell_counts_every_move_it_explained_and_meters_both_sides`).
/// What the release tier buys is the same relations at **B\*, the experiment's
/// operating point**: a cap of 24 makes the root pass a huge share and binds on
/// almost every decision, so the per-edit cell cannot tell whether the relations
/// survive a budget that is large enough for real tree growth. They do, and the
/// shape MOVES with the budget in a way the per-edit cell could not have shown:
/// at B\* the root share is 9.3% (X) / 12.4% (Y) with `starved` 0 on both sides,
/// against 50.2% / 93.6% and 107 / 207 starved decisions at the smoke's caps of
/// 24 / 8.
///
/// MEASURED AT HEAD (16 games): X applied **3,108 moves and explained 3,108**;
/// X scored 1,080,946 (100,024 root · 1,346 binds · 0 starved), Y scored 823,801
/// (102,025 root · 686 binds · 0 starved). No direction is asserted — see the
/// determinism cell's doc for why this shape's θ̂ is not the experiment's answer.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test architecture_gate -- --ignored --test-threads=1"]
fn the_c2_cell_explains_every_move_and_meters_both_sides() {
    counters_at(Scale::GATE);
}

fn counters_at(scale: Scale) -> CellReport {
    let spec = c2_shaped(scale.counter_units, "counters-c2-mcts-vs-capped-s1", scale);
    println!(
        "[counters] the C2 shape at N={} units — X cap {} / Y cap {}",
        scale.counter_units, scale.x_cap, scale.y_cap,
    );
    let report = run(&spec);
    print_report("counters", &report);
    assert_cell_plumbing(&report, &spec);

    let (x_scored, x_root) = assert_capped_side(
        "X (mcts-v1)",
        report.x_candidates_scored,
        report.x_root_scored,
        report.x_budget_binds,
        report.x_budget_starved_decisions,
    );
    let (y_scored, y_root) = assert_capped_side(
        "Y (capped lookahead-v1)",
        report.y_candidates_scored,
        report.y_root_scored,
        report.y_budget_binds,
        report.y_budget_starved_decisions,
    );
    // The Y side must have played at all — otherwise its meters would be a sum
    // over an empty field and every relation above would be about nothing.
    let y_decisions = report.applied_by_seat.iter().sum::<u64>() - report.x_applied_moves;
    assert!(y_decisions > 0, "the field must have played too");
    // The two sides are INDEPENDENT quantities, not one number written twice —
    // the exact defect a mirrored-field seam can ship silently (lens 1 M11).
    assert_ne!((x_scored, x_root), (y_scored, y_root), "the two sides read identically");

    println!(
        "  RECORDED (not asserted): X {x_scored} scored ({x_root} root) over {} decisions · \
         Y {y_scored} ({y_root} root) over {y_decisions} decisions",
        report.x_applied_moves,
    );
    report
}

// ---------------------------------------------------------------------------
// (iii) The MCTS null — mcts-v1@750 against itself
// ---------------------------------------------------------------------------

/// **Spec §8 cell (iii): the standing self-test the roster owes every new
/// contestant** (map row 14's shape; lens 1 S11's grounds). MCTS-v1 at both
/// seats, N = 24 units, asserting a NONZERO SE and non-degenerate unit shares —
/// direction-free by construction, since a same-contestant cell has no direction
/// to claim.
///
/// WHAT IT CATCHES, and it is not hypothetical: keying the play stream by seat
/// alone makes both games of a pair the same game replayed. Every unit then reads
/// exactly 0.5, the unit vector goes constant, and the interval collapses to a
/// typed REFUSAL — an instrument reporting "perfectly calibrated" while measuring
/// nothing (`arena_gate`'s null cell records the measured ρ = −1.0000 for that
/// trap). Such a collapse REPRODUCES byte-identically, so the determinism cell
/// stays green through it; this cell is the only detector the roster would have.
///
/// **The cell is Keep-HOMOGENEOUS by construction** — X and Y are the same
/// contestant, so both seatings of a unit start bit-identical and a nonzero unit
/// spread cannot be an artifact of two Keep rules. `setup-identical=yes` is
/// asserted (in [`assert_cell_plumbing`]), not assumed.
///
/// CONTAINMENT OF 0.5 IS PRINTED AND NEVER ASSERTED, and that is a deliberate
/// difference from `evaluator_gate`'s null cell rather than an omission. At
/// N = 24 the interval is ~0.25 wide, so containment is nearly uninformative
/// about calibration while still being a coin-flip a future master could lose.
/// The degeneracy shape is what carries information at this N, so the degeneracy
/// shape is what is asserted.
///
/// MUTATION-PROVEN (run at authoring, reverted by inverse edit — L7): replacing
/// the self-play pair with a Keep-homogeneous BLOWOUT (`mcts-v1@750` vs the
/// capped S1 arm, i.e. the determinism cell's own shape) reproduces exactly the
/// degenerate shape the CRN trap produces — a constant unit vector, sd 0.000000,
/// a typed refusal instead of an interval — and the `sd > 0` assertion FAILS with
/// its CRN message. That is the mutation worth running rather than bumping a
/// threshold: the hazard is a degenerate vector passing as a calibrated null.
///
/// MEASURED AT HEAD (48 games): **θ̂ 0.479167, bootstrap 95% [0.354167, 0.604167]**
/// — which CONTAINS 0.5, printed rather than asserted. Unit sd 0.345127,
/// SE(θ̂) 0.070449 over 24 units; unit values `{0.0, 0.5, 1.0}` (6 · 13 · 5);
/// seat-0 wins 25/48; mean VP margin +2.79; ChurchTrackFull 40 / OrderDeckEmpty 8;
/// X applied **19,892 moves and explained 19,892**.
#[test]
#[ignore = "release-tier gate: cargo test --release -p yokohama-core --test architecture_gate -- --ignored --test-threads=1"]
fn the_mcts_self_null_is_non_degenerate() {
    null_at(Scale::GATE);
}

fn null_at(scale: Scale) {
    let x = mcts_at(scale.x_cap);
    let spec = cell(scale.null_units, "null-mcts-v1-selfplay", x, x);
    println!(
        "[null] MCTS-v1 at BOTH seats, 2P, N={} units — degeneracy is what is asserted; \
         containment of {NULL_2P} is PRINTED",
        scale.null_units,
    );
    let report = run(&spec);
    print_report("null", &report);
    assert_cell_plumbing(&report, &spec);

    // NOTHING is asserted about WHERE theta_hat sits. A tight point claim is
    // exactly what N = 24 cannot support — the measured 0.479167 would fail a
    // ±0.02 band while being a perfectly healthy null (SE 0.070449), which is
    // the concrete reason this cell gates SHAPE and prints location.
    let sd = sample_sd(&report.x_unit_shares);
    assert!(
        sd > 0.0,
        "the unit vector carried NO variation (sd={sd}) — the CRN/keying trap, not a null cell",
    );
    let mut distinct: Vec<f64> = report.x_unit_shares.clone();
    distinct.sort_by(|a, b| a.partial_cmp(b).expect("unit shares are finite"));
    distinct.dedup();
    assert!(
        distinct.len() >= 2,
        "the {} units took only ONE value ({distinct:?}) — a constant unit vector is the collapse \
         this cell exists to detect, and sd alone would not name it",
        report.x_unit_shares.len(),
    );

    match report.interval {
        IntervalOutcome::Bootstrap { lo, hi, .. } => {
            assert!(hi > lo, "a null cell must report a NONZERO width, got [{lo}, {hi}]");
            assert!(
                lo <= report.theta_hat && report.theta_hat <= hi,
                "the point estimate {:.6} fell outside its own interval [{lo}, {hi}]",
                report.theta_hat,
            );
            println!(
                "  RECORDED (not asserted): the interval [{lo:.6}, {hi:.6}] {} the 2P null \
                 {NULL_2P}; distinct unit values {distinct:?}",
                if lo <= NULL_2P && NULL_2P <= hi { "CONTAINS" } else { "excludes" },
            );
        }
        ref other => panic!(
            "the null cell must report a real interval, got {other:?} — a refusal at this shape IS \
             the degeneracy the cell detects",
        ),
    }
}

// ---------------------------------------------------------------------------
// Per-edit plumbing smoke (spec §8)
// ---------------------------------------------------------------------------

/// Assistants a game actually put on the board, counted off the record's applied
/// moves — `lookahead`'s helper verbatim, and its reason: a final state may have
/// recovered every assistant it ever placed.
fn assistants_placed(stats: &ArenaGameStats) -> usize {
    stats
        .record
        .events
        .iter()
        .map(|e| match e {
            RecordedEvent::Move(Move::PlaceThreeAreas { placements }) => placements.len(),
            RecordedEvent::Move(Move::PlaceTwoSameArea { adjacent_bonus, .. }) => {
                2 + usize::from(adjacent_bonus.is_some())
            }
            _ => 0,
        })
        .sum()
}

/// **THE PER-EDIT SMOKE**, the one test in this binary that is NOT `#[ignore]`d.
///
/// Three arms, and each covers a layer the other two do not. The two 3.4 ARMS run
/// end to end at development scale — MCTS-v1 through its honest chain under the
/// shared budget cap, and the capped S1 arm through the public arena driver — and
/// then ONE tiny cell runs through this file's own helper layer, so a release-only
/// helper that started panicking is found at edit time rather than at merge time
/// (`arena_gate`'s and `evaluator_gate`'s stated grounds, applied to the helpers
/// as well as to the cells).
///
/// Nothing statistical is asserted anywhere in it: a smoke that asserted a
/// direction at one unit would be asserting noise.
#[test]
fn the_architecture_battery_runs_at_smoke_scale() {
    let rules = Rules::standard();

    // -- arm 1: MCTS-v1, both seats, under B* ------------------------------
    // `simulations` at a CEILING with the cap doing the bounding — the
    // CONTESTANT shape (spec §5), and deliberately not a simulation count with a
    // cap beside it: the sterile-simulation hang lived exactly in this shape, and
    // a smoke that never ran it would never smoke-test the guard. It stays cheap
    // because the cap is what bounds the work.
    let mut agent = MctsAgent::new(&rules, agent_stream_seed(WorldSeed(MASTER), PlayerId(0), Slot(0)))
        .with_simulations(u32::MAX)
        .with_cap(Some(B_STAR));
    let tally = agent.tally();
    let pending = new_game(GameConfig::for_player_count(2), &rules, MASTER).expect("2P setup");
    let mut state =
        finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
            .expect("finalize");
    let mut placed = 0usize;
    for _ in 0..MCTS_DECISIONS {
        assert!(!state.is_game_over(), "the smoke must not run out of game");
        let seat = state.current_player;
        let view = observe(&state, seat).expect("observe");
        let report = agent.choose(&view, &rules).expect("the honest chain produces a move");
        if let Move::PlaceThreeAreas { placements } = &report.mv {
            placed += placements.len();
        }
        state = apply_move_unchecked(&state, &report.mv, &rules).expect("a chosen move applies");
    }
    println!(
        "mcts-v1 @cap {B_STAR}: {} decisions · {} scored ({} root) · binds {} · starved {} · \
         overshoot {:?} · {placed} assistants placed",
        tally.decisions(),
        tally.candidates_scored(),
        tally.root_scored(),
        tally.budget_binds(),
        tally.budget_starved_decisions(),
        tally.overshoot(),
    );
    assert_eq!(tally.decisions(), u64::from(MCTS_DECISIONS), "one decision per applied move");
    assert!(tally.candidates_scored() > 0, "the meter must have counted the search's spend");
    assert!(
        tally.root_scored() > 0 && tally.root_scored() < tally.candidates_scored(),
        "the mandatory root pass must be a real fraction of a real total",
    );
    assert!(
        tally.budget_binds() > 0,
        "anti-vacuity: at B* = {B_STAR} the cap must actually bind somewhere in {MCTS_DECISIONS} \
         decisions",
    );
    assert_eq!(
        tally.overshoot().map(|o| o.decisions),
        Some(u64::from(MCTS_DECISIONS)),
        "every capped decision contributes an overshoot sample, zero or not",
    );

    // -- arm 2: the capped S1 arm, one whole game through the arena driver --
    let world = WorldSeed(MASTER);
    let tallies: Arc<Mutex<Vec<_>>> = Arc::default();
    let sink = Arc::clone(&tallies);
    let mut make = |seat: PlayerId, _n: u8| -> Box<dyn ArenaAgent> {
        let agent = LookaheadAgent::new(&rules, agent_stream_seed(world, seat, Slot(seat.0)))
            .with_cap(Some(S1_CAP));
        sink.lock().expect("tallies").push(agent.tally());
        Box::new(agent)
    };
    let stats = run_arena_game_capped(world, 2, &rules, &mut make, true, 1_500)
        .expect("a capped S1 game must complete");
    let handles = tallies.lock().expect("tallies").clone();
    let scored: u64 = handles.iter().map(|t| t.candidates_scored()).sum();
    let binds: u64 = handles.iter().map(|t| t.budget_binds()).sum();
    println!(
        "lookahead-v1 @cap {S1_CAP}: turns {} · moves {} · scored {scored} · binds {binds} · \
         {} assistants placed",
        stats.turns,
        stats.moves_applied,
        assistants_placed(&stats),
    );
    assert!(stats.turns < 1_500, "the capped game must END inside the turn budget");
    assert!(assistants_placed(&stats) > 0, "and still place assistants");
    assert!(binds > 0, "anti-vacuity: at {S1_CAP} the cap must have denied work");

    // -- arm 3: the CELL layer, one unit at smoke caps ----------------------
    // The counters cell is the one whose assertions still mean something at
    // N = 1: an identity and a set of per-side meter relations are not
    // statistical. The determinism and null cells are deliberately excluded —
    // determinism at one unit re-runs the `run_cell` path this call already
    // exercised, and a null cell's degeneracy assertions carry no information
    // over two units.
    counters_at(Scale::SMOKE);
}
