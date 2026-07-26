//! Phase 3.1b Task 6 — the single in-repo route-cost re-measure (spec §8).
//!
//! RECORDS, never asserts. Three blocks:
//!
//! **A. ORGANIC.** At every Movement decision point reached by uniform-random
//! self-play under ONE stated policy: the SHIPPED [`enumerate_routes`] (latency,
//! allocations, label count) and the frozen [`enumerate_president_routes`] on the
//! same state. This replaces the 21 / 32 / 38 organic-maxima scatter (three
//! harnesses, three trajectories, no canonical figure — spec F5) with one
//! policy-stated figure set, and replaces the 679 ns mean / 3.3 µs max that were
//! measured out of repo under `Prune::Adaptive` (spec F3) with the shipped
//! variant's own numbers. The ZERO-LABEL and low-label buckets are broken out
//! rather than averaged away: the port builds its `RouteGraph` over every board
//! row before it discovers there is nothing to walk, so it is SLOWER than the
//! frozen search exactly there, and the honest figure set has to show it.
//!
//! PRUNE RE-DERIVATION: NOT PERFORMED IN-REPO, stated rather than implied
//! (spec §8 asked for it; T6 review). The shipped port IS the no-oracle shape
//! (spec F3's `Prune::None` decision) — the saturation-oracle variants were
//! scratch-prototype-only and were never ported, so there is nothing in-repo
//! to A/B against. The F3 decision therefore still rests on the out-of-repo
//! bench3.log data. What THIS example's bucket table bears on it: 83.6% of
//! organic decision points sit at ≤5 labels — the regime where the scratch
//! data had Adaptive ~3× faster — but the absolute per-call cost there is
//! hundreds of nanoseconds paid once per decision point, so the trade is
//! immaterial at organic scale, and the dense regime (where the no-oracle
//! shape WINS) is where cost is real. Recorded in guidingpath's 3.1b ledger:
//! flip-or-confirm requires porting an oracle variant — worth doing only if
//! 3.3 profiling ever shows the label-poor regime mattering.
//!
//! **B. DENSE.** The four adversarial cells of `tests/action_source_equivalence.rs`'s
//! release battery (labels 458 / 519 / 612 / 694, each an observation under that
//! harness and never a ceiling — spec F5/L9), timed AND allocation-metered on
//! both variants. The 612 and 694 cells were untimed when the spec was written;
//! the equivalence battery has since timed them, and this block adds the
//! allocation figures and puts both variants on one bench.
//!
//! **C. K-TABLE.** The per-DECISION-POINT arithmetic ledger row L10 exists to
//! make concrete: what a K-candidate widening loop pays when it re-enumerates
//! per candidate, at the organic and dense anchors, at one and at two
//! enumerations per candidate (`family_len` + `iter_family().nth(i)` pays two).
//!
//!   cargo run --release -p yokohama-core --example route_cost
//!   cargo run --release -p yokohama-core --example route_cost -- --policy family-weighted
//!
//! **L11 (added 3.2 Task 13).** Block A's driver is selectable. The default is
//! the legacy uniform-over-`get_legal_moves` driver every figure above was
//! recorded under; `--policy <name>` drives it with a shipped [`RolloutPolicy`]
//! through a `DecisionHandle` instead. This exists because a route-label
//! distribution is a statement about a POLICY and not about the game: uniform
//! play returns the President to hand far more often than any competent policy
//! would, so its organic maxima under-describe what a real player's search
//! pays. Both arms are on the record; neither may be quoted as the other.
//!
//! Meter: the shared `action_cost_meter` (3.1a), so `allocs` (deterministic
//! event count) and `peak_bytes` (peak OUTSTANDING requested bytes,
//! machine-variable) mean here exactly what the 3.1a bound test means by them.
//! Latencies are `Instant`-timed averages with `std::hint::black_box` to defeat
//! elision — no criterion, no new dependency.
//!
//! MEASUREMENT CAVEAT, stated because it cuts AGAINST the port's own result:
//! this binary links the counting global allocator, whose disarmed fast path is
//! still a thread-local read on every allocation in the process. The frozen
//! reference allocates orders of magnitude more than the port at the dense
//! cells, so that tax lands almost entirely on the reference — every
//! shipped-vs-frozen latency RATIO below is therefore an upper bound on the
//! port's real advantage, not a neutral estimate. Allocation counts and label
//! counts are unaffected.

#[path = "../tests/action_cost_meter/mod.rs"]
mod meter;

// The same seeded post-setup constructor the equivalence sweep drives from, so
// this example's "organic" means what that gate's "organic" means.
#[path = "../tests/common/mod.rs"]
mod common;

// The dense cells' shared construction (Task 3's `tests/route_fixtures`) — the
// same fixture the gated batteries pin, so the curves recorded here cannot
// drift from the gated cells (T3 review F3).
#[path = "../tests/route_fixtures/mod.rs"]
mod route_fixtures;

use std::hint::black_box;
use std::time::{Duration, Instant};

use rand::Rng;

use yokohama_core::{
    apply_move_unchecked, enumerate_president_routes, enumerate_routes, fits_fixed_arrays,
    get_legal_moves, is_game_over, AreaId, DecisionHandle, GameState, PlayerId, PresidentLocation,
    RolloutPolicy, Rules, TechCardId, TechEffect, TurnPhase,
};

// ---------------------------------------------------------------------------
// The stated policy (block A). Everything organic below is measured under THIS
// and nothing else; any figure quoted elsewhere cites it.
// ---------------------------------------------------------------------------

/// Block A's move-selection driver, chosen by `--policy` (default `uniform`).
///
/// The distinction is ledger L11's whole subject: route labels are a property
/// of the STATES a game visits, so a label distribution is a statement about a
/// POLICY, never about Yokohama. Two arms exist so both statements are on the
/// record and neither can be quoted as the other.
#[derive(Clone, Copy)]
enum SweepDriver {
    /// Uniform-random over the materialized `get_legal_moves` list — the driver
    /// every committed 3.1b organic figure was recorded under, kept as the
    /// DEFAULT so an argument-free run still reproduces those curves. It is not
    /// `RolloutPolicy::Uniform`: that rung draws families-first over the
    /// factored surface, which is the same distribution by construction but a
    /// different code path, and re-baselining the 3.1b curves onto it would buy
    /// nothing and cost their comparability.
    LegacyUniform,
    /// A shipped [`RolloutPolicy`] drawn through a `DecisionHandle`, exactly as
    /// the arena's raw-state driver does. This is L11's re-measure arm (spec
    /// §6): the shipped `FAMILY_WEIGHTED_DEFAULT` visits materially different
    /// boards from uniform play, and the organic label maxima move with it.
    Policy(RolloutPolicy),
}

impl SweepDriver {
    /// The policy label printed on every figure this driver produces.
    fn label(self) -> String {
        match self {
            Self::LegacyUniform => {
                "uniform-random self-play over `get_legal_moves` (legacy 3.1b driver)".to_string()
            }
            Self::Policy(p) => {
                let (name, version, params) = p.descriptor();
                let knobs: Vec<String> =
                    params.iter().map(|(k, v)| format!("{k}={v}")).collect();
                format!(
                    "RolloutPolicy `{name}` v{version} [{}] over a DecisionHandle \
                     (the arena's raw-state driver shape)",
                    knobs.join(", "),
                )
            }
        }
    }
}

/// Seeds swept at each player count. Small literal seeds are deliberate here
/// and permitted by spec §3: this binary RECORDS a cost curve, it is not a
/// bot-facing evaluation whose figures get quoted as strength, and a curve you
/// cannot re-run is not a curve.
const SEEDS: std::ops::Range<u64> = 0..12;
/// Per-game step cap — set to run games to their natural END, which is what a
/// distribution needs. Uniform-random Yokohama games are long: `tests/common`
/// records means of 1,181 / 1,625 / 2,126 turns at 2/3/4P and a 6,036-turn
/// maximum, and a turn spends ~5 decision points, so the equivalence sweep's
/// 8,000-step cap truncates about a quarter of them — which would bias this
/// distribution toward early-game boards, where fewer own assistants are on the
/// board and there is correspondingly less to walk. The truncation count is
/// printed either way rather than assumed to be zero.
const STEP_CAP: usize = 40_000;
/// Timing repeats per organic decision point, per variant. Both sides get the
/// same count deliberately: the interesting organic cell is the ZERO-LABEL one,
/// where the frozen search returns in tens of nanoseconds — at ten repeats that
/// is a handful of `Instant` ticks and the bucket's ratio swung by 25% between
/// two runs, so the repeats buy the low end its precision. The whole sweep still
/// costs a couple of seconds.
const ORGANIC_SHIPPED_ITERS: u32 = 100;
const ORGANIC_FROZEN_ITERS: u32 = 100;
/// Timing repeats per dense cell (the frozen side runs ~100 ms per call there).
const DENSE_SHIPPED_ITERS: u32 = 20;
const DENSE_FROZEN_ITERS: u32 = 3;

/// Average wall-clock of `f` over `iters` runs, `black_box`ing the result so the
/// optimizer cannot elide the work. (`examples/action_cost.rs`'s helper,
/// deliberately identical — the two examples' latencies are comparable only if
/// they are timed the same way.)
fn time_avg<R>(iters: u32, mut f: impl FnMut() -> R) -> Duration {
    let start = Instant::now();
    for _ in 0..iters {
        black_box(f());
    }
    start.elapsed() / iters
}

// ---------------------------------------------------------------------------
// Distribution helpers.
// ---------------------------------------------------------------------------

/// mean / median / p99 / max over a sample. Held in `u128` so nanoseconds and
/// counts share one summarizer.
struct Summary {
    mean: u128,
    median: u128,
    p99: u128,
    max: u128,
}

fn summarize(v: &[u128]) -> Summary {
    let mut s = v.to_vec();
    s.sort_unstable();
    let n = s.len();
    Summary {
        mean: if n == 0 { 0 } else { s.iter().sum::<u128>() / n as u128 },
        median: percentile(&s, 0.50),
        p99: percentile(&s, 0.99),
        max: s.last().copied().unwrap_or(0),
    }
}

/// Nearest-rank percentile over an ascending sample (empty ⇒ 0).
fn percentile(sorted: &[u128], p: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

/// Nanoseconds as a `Duration` string, two decimals, so columns line up.
fn dur(ns: u128) -> String {
    format!("{:.2?}", Duration::from_nanos(ns.min(u128::from(u64::MAX)) as u64))
}

/// Arithmetic mean over a sample (empty ⇒ 0), for the columns that want only a
/// mean and not a whole [`Summary`].
fn mean_of(v: &[u128]) -> u128 {
    if v.is_empty() {
        0
    } else {
        v.iter().sum::<u128>() / v.len() as u128
    }
}

// ---------------------------------------------------------------------------
// Block A — organic.
// ---------------------------------------------------------------------------

/// One measured organic Movement decision point.
struct Point {
    labels: usize,
    shipped_ns: u128,
    frozen_ns: u128,
    shipped_allocs: usize,
    frozen_allocs: usize,
}

/// What the sweep saw besides the measured points — the denominators that make
/// the distributions readable (and that would expose a vacuous run).
struct SweepShape {
    games: usize,
    decision_points: usize,
    /// Movement points whose President was already in hand: a routes family is
    /// dispatched there too, but it is the `PlacePresident` arm, and
    /// `enumerate_routes` walks nothing. Counted, not measured — folding these
    /// into the zero-label bucket would inflate it with states that cannot walk
    /// by construction rather than states that found nothing to walk.
    from_hand_points: usize,
    truncated_games: usize,
    /// Games that ended on a structurally empty family rather than on
    /// `is_game_over` — expected to be zero, printed so it cannot hide.
    dead_end_games: usize,
    wall: Duration,
}

fn organic_sweep(rules: &Rules, driver: SweepDriver) -> (Vec<Point>, SweepShape) {
    let mut points = Vec::new();
    let mut shape = SweepShape {
        games: 0,
        decision_points: 0,
        from_hand_points: 0,
        truncated_games: 0,
        dead_end_games: 0,
        wall: Duration::ZERO,
    };
    let sweep_start = Instant::now();

    for players in [2u8, 3, 4] {
        for seed in SEEDS {
            let (mut state, mut rng) = common::new_seeded_game(seed, players, rules);
            shape.games += 1;
            let mut steps = 0usize;

            loop {
                if is_game_over(&state) {
                    break;
                }

                // The two arms differ ONLY in how the next move is drawn. The
                // legacy arm's control flow is preserved exactly — same probe,
                // same order, same rng call — because every 3.1b figure was
                // recorded under it and a default run must still reproduce them.
                let drawn = match driver {
                    SweepDriver::LegacyUniform => {
                        let legal = get_legal_moves(&state, rules);
                        if legal.is_empty() {
                            break;
                        }
                        shape.decision_points += 1;
                        measure_if_walkable(&mut points, &mut shape, &state, rules);
                        Some(legal[rng.gen_range(0..legal.len())].clone())
                    }
                    SweepDriver::Policy(policy) => {
                        let handle = DecisionHandle::new(&state, rules);
                        if handle.families().is_empty() {
                            break;
                        }
                        shape.decision_points += 1;
                        measure_if_walkable(&mut points, &mut shape, &state, rules);
                        policy.pick(&handle, &mut rng)
                    }
                };

                // `pick` returns `None` only on a structurally empty family, and
                // never at a Movement point (the retreat family always holds the
                // recover-nothing move). Counted and printed rather than
                // silently swallowed — the arena driver treats it as an error,
                // and a recording example should at least show it happened.
                let Some(mv) = drawn else {
                    shape.dead_end_games += 1;
                    break;
                };

                state = apply_move_unchecked(&state, &mv, rules).expect("apply drawn move");
                steps += 1;
                if steps >= STEP_CAP {
                    shape.truncated_games += 1;
                    break;
                }
            }
        }
    }

    shape.wall = sweep_start.elapsed();
    (points, shape)
}

/// A routes family is dispatched exactly when the queue is empty and the phase
/// is Movement (`search_actions::families`); only the President-on-board case
/// actually walks, so only it is measured.
fn measure_if_walkable(
    points: &mut Vec<Point>,
    shape: &mut SweepShape,
    state: &GameState,
    rules: &Rules,
) {
    if state.pending_choices.is_empty() && state.turn_phase == TurnPhase::Movement {
        let player = state.current_player;
        match state.player(player).president_location {
            PresidentLocation::OnBoard(from) => {
                points.push(measure_point(state, rules, player, from));
            }
            _ => shape.from_hand_points += 1,
        }
    }
}

/// Both variants on one state: label count once, then latency and allocations
/// for each. The two searches are output-identical here (that is the equivalence
/// gate's job, not this example's), so one label count describes both.
fn measure_point(state: &GameState, rules: &Rules, player: PlayerId, from: AreaId) -> Point {
    let reg = Some(&rules.techs);

    let labels = enumerate_routes(state, rules).len();
    let shipped_ns = time_avg(ORGANIC_SHIPPED_ITERS, || enumerate_routes(state, rules)).as_nanos();
    let (_, shipped_m) = meter::measure(|| enumerate_routes(state, rules));

    let frozen_ns =
        time_avg(ORGANIC_FROZEN_ITERS, || enumerate_president_routes(state, player, from, reg))
            .as_nanos();
    let (_, frozen_m) = meter::measure(|| enumerate_president_routes(state, player, from, reg));

    Point {
        labels,
        shipped_ns,
        frozen_ns,
        shipped_allocs: shipped_m.allocs,
        frozen_allocs: frozen_m.allocs,
    }
}

const BUCKETS: [(&str, usize, usize); 6] = [
    ("0 (nothing walkable)", 0, 0),
    ("1-2", 1, 2),
    ("3-5", 3, 5),
    ("6-10", 6, 10),
    ("11-20", 11, 20),
    ("21+", 21, usize::MAX),
];

fn report_organic(points: &[Point], shape: &SweepShape, driver: SweepDriver) {
    let movement_points = points.len() + shape.from_hand_points;
    let on_board_share = if movement_points == 0 {
        0.0
    } else {
        100.0 * points.len() as f64 / movement_points as f64
    };
    println!(
        "== A. ORGANIC ROUTE COST ==\n\n\
         POLICY (the one every figure in this block is stated under, and the only one):\n\
         {}, from `common::new_seeded_game`, seeds {}..{} x 2/3/4 players = {} games,\n\
         played to their natural end under a {}-step safety cap; {} games hit the cap and\n\
         {} ended on an empty family. Sweep wall clock {:.2?}. Decision points driven: {}.\n\n\
         Measured at every Movement decision point with the President ON BOARD: {} points.\n\
         A further {} Movement points had the President IN HAND: a routes family is\n\
         dispatched there too, but it is the `PlacePresident` arm and `enumerate_routes`\n\
         walks nothing, so those are counted, not measured. That {:.1}% on-board share is\n\
         itself a POLICY ARTIFACT, not a fact about Yokohama — uniform-random play draws\n\
         from a retreat family that returns the President to hand far more often than any\n\
         competent policy would, so a real player pays the walk enumeration at a much\n\
         higher rate than uniform's counts suggest. That is ledger L11, and `--policy\n\
         family-weighted` is the arm that measures it rather than asserting it.\n\n\
         This SUPERSEDES the 21/32/38 organic-label maxima (three harnesses, three\n\
         trajectories, no canonical figure) and the 679 ns mean / 3.3 us max latencies\n\
         (measured out of repo under Prune::Adaptive). Latency repeats: shipped x{},\n\
         frozen x{}.\n",
        driver.label(),
        SEEDS.start,
        SEEDS.end,
        shape.games,
        STEP_CAP,
        shape.truncated_games,
        shape.dead_end_games,
        shape.wall,
        shape.decision_points,
        points.len(),
        shape.from_hand_points,
        on_board_share,
        ORGANIC_SHIPPED_ITERS,
        ORGANIC_FROZEN_ITERS,
    );

    let labels: Vec<u128> = points.iter().map(|p| p.labels as u128).collect();
    let shipped: Vec<u128> = points.iter().map(|p| p.shipped_ns).collect();
    let frozen: Vec<u128> = points.iter().map(|p| p.frozen_ns).collect();
    let s_allocs: Vec<u128> = points.iter().map(|p| p.shipped_allocs as u128).collect();
    let f_allocs: Vec<u128> = points.iter().map(|p| p.frozen_allocs as u128).collect();

    println!("  {:<40} {:>10} {:>10} {:>10} {:>10}", "distribution (n = points)", "mean", "median", "p99", "max");
    println!("  {:-<40} {:->10} {:->10} {:->10} {:->10}", "", "", "", "", "");
    count_row("labels per point", &summarize(&labels));
    time_row("enumerate_routes (SHIPPED)", &summarize(&shipped));
    time_row("enumerate_president_routes (frozen)", &summarize(&frozen));
    count_row("allocs, shipped", &summarize(&s_allocs));
    count_row("allocs, frozen", &summarize(&f_allocs));

    println!(
        "\n  BY LABEL BUCKET (the mean above hides the shape: the port pays a full RouteGraph\n\
         build — every board row — before it can discover there is nothing to walk, so at\n\
         the zero-label end it is SLOWER than the frozen search, which returns almost\n\
         immediately. Recorded, NOT optimized: restricting the build to steppable rows is a\n\
         follow-up recorded in guidingpath's 3.1b ledger row (T7), deliberately not taken in\n\
         a measurement task. The two allocation\n\
         columns are printed so the counting-allocator caveat can be CHECKED per bucket\n\
         instead of taken on trust — where both sides allocate nothing, the meter's tax\n\
         cannot be what separates them.)\n"
    );
    println!(
        "  {:<24} {:>7} {:>11} {:>11} {:>11} {:>11} {:>8} {:>8} {:>8}",
        "labels",
        "points",
        "ship mean",
        "ship p99",
        "froz mean",
        "froz p99",
        "ship/fr",
        "s.alloc",
        "f.alloc",
    );
    println!(
        "  {:-<24} {:->7} {:->11} {:->11} {:->11} {:->11} {:->8} {:->8} {:->8}",
        "", "", "", "", "", "", "", "", "",
    );
    for &(name, lo, hi) in &BUCKETS {
        let bucket: Vec<&Point> =
            points.iter().filter(|p| p.labels >= lo && p.labels <= hi).collect();
        if bucket.is_empty() {
            println!("  {name:<24} {:>7}", 0);
            continue;
        }
        let sh: Vec<u128> = bucket.iter().map(|p| p.shipped_ns).collect();
        let fr: Vec<u128> = bucket.iter().map(|p| p.frozen_ns).collect();
        let sa: Vec<u128> = bucket.iter().map(|p| p.shipped_allocs as u128).collect();
        let fa: Vec<u128> = bucket.iter().map(|p| p.frozen_allocs as u128).collect();
        let (sh_s, fr_s) = (summarize(&sh), summarize(&fr));
        let ratio = if fr_s.mean == 0 { 0.0 } else { sh_s.mean as f64 / fr_s.mean as f64 };
        println!(
            "  {name:<24} {:>7} {:>11} {:>11} {:>11} {:>11} {ratio:>7.2}x {:>8} {:>8}",
            bucket.len(),
            dur(sh_s.mean),
            dur(sh_s.p99),
            dur(fr_s.mean),
            dur(fr_s.p99),
            mean_of(&sa),
            mean_of(&fa),
        );
    }
}

fn time_row(label: &str, s: &Summary) {
    println!(
        "  {label:<40} {:>10} {:>10} {:>10} {:>10}",
        dur(s.mean),
        dur(s.median),
        dur(s.p99),
        dur(s.max),
    );
}

fn count_row(label: &str, s: &Summary) {
    println!("  {label:<40} {:>10} {:>10} {:>10} {:>10}", s.mean, s.median, s.p99, s.max);
}

// ---------------------------------------------------------------------------
// Block B — the dense cells.
//
// The board and states below come from the SHARED construction in
// `tests/route_fixtures` — the same fixture the equivalence battery pins
// pairs-and-order for, the worstcase gate meters, and the l10 pin floors — so
// the curve figures this example records are measurements of the SAME cells
// those gates gate, structurally rather than by review discipline. (An earlier
// header here claimed an example cannot see a test binary's helpers; false —
// `#[path]` inclusion is exactly how this file already shares the meter and
// the seeded constructor, and the T3 review's F3 closed the fourth copy that
// claim had justified.)
// ---------------------------------------------------------------------------

/// The mover's President on `start`, own assistants on all 17 non-Canal areas,
/// the three opponents on `opponents`, `techs` granted, 1,000 Yen so no
/// affordability cutoff bites (money is the DFS cost driver — spec F6, and these
/// cells exist to be expensive).
/// The equivalence battery's dense cell via `tests/route_fixtures`, plus the
/// techs-and-`Rules` tail that is this example's own concern — the shared
/// constructor carries board/positions/money, and granting techs needs a
/// `Rules` lookup, so the wrapper stays here exactly as
/// `action_source_equivalence.rs`'s does.
fn dense_state(start: u8, opponents: [u8; 3], techs: &[TechEffect], rules: &Rules) -> GameState {
    let mut s = route_fixtures::dense_17_area_cell_at(start, opponents);
    s.players[0].technologies = techs.iter().map(|&e| tech_id(rules, e)).collect();
    s
}

fn tech_id(rules: &Rules, effect: TechEffect) -> TechCardId {
    rules
        .techs
        .iter()
        .find(|t| t.effect == effect)
        .map(|t| t.id)
        .expect("standard data carries the tech")
}

/// One dense cell plus the label count the equivalence battery observed for it
/// (spec F5: an OBSERVATION under that harness, never a ceiling).
struct DenseCell {
    name: &'static str,
    start: u8,
    opponents: [u8; 3],
    techs: &'static [TechEffect],
    observed: usize,
}

const DENSE_CELLS: [DenseCell; 4] = [
    DenseCell { name: "plain", start: 0, opponents: [3, 8, 17], techs: &[], observed: 458 },
    DenseCell {
        name: "postal",
        start: 0,
        opponents: [3, 8, 17],
        techs: &[TechEffect::PostalSystem],
        observed: 519,
    },
    DenseCell {
        name: "postal, start 12, opp [1,6,11]",
        start: 12,
        opponents: [1, 6, 11],
        techs: &[TechEffect::PostalSystem],
        observed: 612,
    },
    DenseCell {
        name: "postal, start 17, opp [2,7,12]",
        start: 17,
        opponents: [2, 7, 12],
        techs: &[TechEffect::PostalSystem],
        observed: 694,
    },
];

/// Returns the slowest shipped enumeration across the cells (the block-C dense
/// anchor) with the cell that produced it.
fn report_dense(rules: &Rules) -> (&'static str, u128) {
    println!(
        "\n\n== B. DENSE ADVERSARIAL CELLS ==\n\n\
         The equivalence battery's four cells: 4P staggered pyramid, interior Canal (K6,\n\
         nine tolled crossings), own assistants on all 17 non-Canal areas, three opponent\n\
         Presidents, 1,000 Yen. Label counts are OBSERVATIONS under this harness's\n\
         geometry, NEVER ceilings (spec F5/L9). Latency repeats: shipped x{}, frozen x{}.\n\
         The last two cells (612 / 694) get allocation figures here for the first time.\n",
        DENSE_SHIPPED_ITERS, DENSE_FROZEN_ITERS,
    );
    println!(
        "  {:<32} {:>7} {:>11} {:>9} {:>11} {:>11} {:>11} {:>8}",
        "cell", "labels", "shipped", "s.allocs", "s.peak B", "frozen", "f.allocs", "speedup",
    );
    println!(
        "  {:-<32} {:->7} {:->11} {:->9} {:->11} {:->11} {:->11} {:->8}",
        "", "", "", "", "", "", "", "",
    );

    let mut worst = ("", 0u128);
    for cell in &DENSE_CELLS {
        let s = dense_state(cell.start, cell.opponents, cell.techs, rules);
        assert!(fits_fixed_arrays(&s), "{}: dense cell must take the fixed-array path", cell.name);
        let player = s.current_player;
        let from = AreaId::new(cell.start);
        let reg = Some(&rules.techs);

        let labels = enumerate_routes(&s, rules).len();
        let ship_ns = time_avg(DENSE_SHIPPED_ITERS, || enumerate_routes(&s, rules)).as_nanos();
        let (_, ship_m) = meter::measure(|| enumerate_routes(&s, rules));

        let froz_ns =
            time_avg(DENSE_FROZEN_ITERS, || enumerate_president_routes(&s, player, from, reg))
                .as_nanos();
        let (_, froz_m) = meter::measure(|| enumerate_president_routes(&s, player, from, reg));

        if ship_ns > worst.1 {
            worst = (cell.name, ship_ns);
        }
        let speedup = if ship_ns == 0 { 0.0 } else { froz_ns as f64 / ship_ns as f64 };
        println!(
            "  {:<32} {labels:>7} {:>11} {:>9} {:>11} {:>11} {:>11} {speedup:>7.1}x",
            cell.name,
            dur(ship_ns),
            ship_m.allocs,
            ship_m.peak_bytes,
            dur(froz_ns),
            froz_m.allocs,
        );
        // The observed figure is context, not a gate: this example asserts
        // nothing about labels (the equivalence battery floors them).
        if labels != cell.observed {
            println!(
                "  {:<32} (note: {labels} labels here vs {} recorded by the equivalence battery)",
                "", cell.observed,
            );
        }
    }
    worst
}

// ---------------------------------------------------------------------------
// Block C — the K-candidate table (ledger row L10's sizing input).
// ---------------------------------------------------------------------------

fn report_k_table(anchors: &[(String, u128)]) {
    println!(
        "\n\n== C. PER-DECISION-POINT ARITHMETIC (ledger L10) ==\n\n\
         3.1b's routes family is COMPUTE-PER-CALL: no cache, one full enumeration per\n\
         `iter_family` / `sample_family` / `family_len` call. A 3.3 widening loop that\n\
         re-enumerates per candidate therefore pays K enumerations per DECISION POINT —\n\
         and `family_len` followed by `iter_family().nth(i)` pays TWO per candidate. The\n\
         table is the printed record of what that costs, so the sizing input is measured\n\
         rather than inferred.\n"
    );
    println!(
        "  {:<38} {:>12} {:>12} {:>12}",
        "anchor (1 enumeration / candidate)", "K=1", "K=10", "K=100",
    );
    println!("  {:-<38} {:->12} {:->12} {:->12}", "", "", "", "");
    for (name, ns) in anchors {
        println!("  {name:<38} {:>12} {:>12} {:>12}", dur(*ns), dur(ns * 10), dur(ns * 100));
    }
    println!(
        "\n  {:<38} {:>12} {:>12} {:>12}",
        "anchor (2 enumerations / candidate)", "K=1", "K=10", "K=100",
    );
    println!("  {:-<38} {:->12} {:->12} {:->12}", "", "", "", "");
    for (name, ns) in anchors {
        println!(
            "  {name:<38} {:>12} {:>12} {:>12}",
            dur(ns * 2),
            dur(ns * 20),
            dur(ns * 200),
        );
    }
    println!(
        "\n  Read: at the organic anchors a re-enumerating loop is free at any K a search\n\
         would use; at the dense anchor it is not, and ONE enumeration per Movement\n\
         decision point (an enumerate-once handle or a tree-node cache) is what 3.3 needs\n\
         to hold. That is L10's recommendation, now with this run's numbers under it."
    );
}

/// `--policy <name>` → the block-A driver. Names are `RolloutPolicy`'s own
/// (`RolloutPolicy::from_str_name`), plus `uniform` meaning the legacy driver,
/// so the default run is unchanged.
fn parse_driver(args: &[String]) -> Result<SweepDriver, String> {
    let mut driver = SweepDriver::LegacyUniform;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--policy" => {
                let name = rest.next().ok_or("--policy needs a value")?;
                driver = match name.as_str() {
                    "uniform" => SweepDriver::LegacyUniform,
                    other => SweepDriver::Policy(
                        RolloutPolicy::from_str_name(other)
                            .ok_or_else(|| format!("unknown policy `{other}`"))?,
                    ),
                };
            }
            other => return Err(format!("unexpected argument `{other}`")),
        }
    }
    Ok(driver)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let driver = match parse_driver(&args) {
        Ok(d) => d,
        Err(message) => {
            eprintln!(
                "{message}\n\nusage: route_cost [--policy uniform|retreat-averse|family-weighted]\n\
                 \n  uniform (default) is the legacy 3.1b driver — an argument-free run\n\
                 reproduces the committed curves. Any other name drives block A with that\n\
                 shipped RolloutPolicy (ledger L11's re-measure arm)."
            );
            std::process::exit(2);
        }
    };

    let rules = Rules::standard();
    println!(
        "== Phase 3.1b route-cost re-measure (release; peak = peak OUTSTANDING requested bytes) ==\n\n\
         Every figure RECORDED, none asserted. The counting global allocator is linked into\n\
         this binary, and its disarmed fast path still costs a thread-local read per\n\
         allocation — which taxes the allocation-heavy frozen reference far more than the\n\
         port, so every shipped-vs-frozen ratio below is an UPPER bound on the port's real\n\
         advantage. Label and allocation counts are unaffected.\n"
    );

    let (points, shape) = organic_sweep(&rules, driver);
    report_organic(&points, &shape, driver);
    let (dense_worst_name, dense_worst_ns) = report_dense(&rules);

    let shipped: Vec<u128> = points.iter().map(|p| p.shipped_ns).collect();
    let s = summarize(&shipped);
    report_k_table(&[
        ("organic mean".to_string(), s.mean),
        ("organic p99".to_string(), s.p99),
        ("organic max".to_string(), s.max),
        (format!("dense max ({dense_worst_name})"), dense_worst_ns),
    ]);
}
