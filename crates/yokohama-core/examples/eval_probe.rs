//! `eval_probe` — the corpus instrument evaluator v1's weights are frozen
//! against (Phase 3.3 T4; spec §2.4, §2.5, §10.4).
//!
//! It does four things, in one pass over one generated corpus so that every
//! figure below is a statement about the SAME games:
//!
//!   * **(a) generate** a self-play corpus under a NAMED contestant — Greedy-v1
//!     through the honest arena driver, or a [`RolloutPolicy`] rung through the
//!     raw-state driver (legitimate here: the bot is not the subject, which is
//!     research-notes R1's own harness disposition) — at a given player count,
//!     game count and master, snapshotting **every seat's view at every turn
//!     start**;
//!   * **(b) per-feature AUC** against the eventual winner, on R1's design:
//!     game-aggregated cells (game × seat × stage tercile), a **game-clustered**
//!     bootstrap for the interval, and EARLY / **MID** / LATE reported for every
//!     row — the shipped evaluator total rides along as a benchmark row and the
//!     T3 hand split's two halves are separate rows beside their unsplit sum;
//!   * **(c) proxy validation** — the shipped [`progress_ppm`] against the
//!     post-hoc terciles (confusion, accuracy, and the distinct-value count that
//!     says how many steps the "continuous" proxy actually has);
//!   * **(d) behavioural counters** — the §2.5 nonzero-recovery pick share, the
//!     §3.3 placement cap-bind count, the game-length distribution and the
//!     end-trigger mix.
//!
//! # Two things this instrument is careful about
//!
//! **One `ScoreStructure` per view.** Four of the five score-structure features
//! are RANKINGS over the whole table, so the convenience accessor
//! `eval::raw_value` re-derives the entire structure on every call. The inner
//! loop here therefore calls [`ScoreStructure::derive`] ONCE per view and reads
//! [`ScoreStructure::value_of`] for every (feature, player) pair off it — the
//! same discipline `eval::score` follows, and the difference at corpus scale is
//! 16 × players derivations per view against one.
//!
//! **Snapshots are not independent.** Consecutive turns of one game are heavily
//! autocorrelated and every seat of one game shares its outcome label, so a
//! per-snapshot interval would be far too narrow (R1 measured per-snapshot AUCs
//! attenuated 0.03–0.06 by the same autocorrelation). Cells aggregate a
//! (game, seat, tercile) block to one number and the bootstrap resamples GAMES,
//! not cells — the cluster is the unit of independence.
//!
//! # Determinism
//!
//! Everything is a function of `(master, contestant, weights, players, games,
//! bootstrap B)`. The corpus comes off `SeedSchedule::diagnostic(master)`; the
//! resample stream is derived from the DESIGN (master, games, players, B) and
//! printed, never from the data. Re-running a printed command reproduces its
//! output byte for byte on the same build.
//!
//! # Masters
//!
//! `--master <u64>` is a DIAGNOSTIC master by the name of its constructor
//! (3.2 spec §3): reproduction, fixtures and recorded measurement artifacts.
//! `--os-entropy` draws one and prints it — the rule for BOT-FACING EVALUATION
//! runs whose figures get quoted as strength claims. The weight freeze is a
//! measurement artifact and not a strength claim, so it runs diagnostic; the
//! header says which, always.
//!
//! ```text
//! cargo run --release -p yokohama-core --example eval_probe -- \
//!     run 2 2000 --contestant greedy-v1 --master 0xE7A1202607270001
//! cargo run --release -p yokohama-core --example eval_probe -- \
//!     run 2 200 --contestant family-weighted --blocks proxy
//! cargo run --release -p yokohama-core --example eval_probe -- table
//! ```
//!
//! Its own statistics are unit-tested: `cargo test -p yokohama-core --example
//! eval_probe`.

use std::collections::BTreeMap;
use std::time::Instant;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use yokohama_core::game::eval::{
    progress_ppm, score, Feature, ScoreStructure, WeightTable, PROGRESS_FULL_PPM,
};
use yokohama_core::game::greedy::{
    weight_table_identity, ExplanationTally, GreedyAgent, DEFAULT_RETREAT_SAMPLES,
    GREEDY_CONTROLLER,
};
use yokohama_core::{
    agent_stream_seed, finalize_setup, new_game, observe, replay_from, run_arena_game,
    run_raw_game, ArenaGameStats, GameState, PlayerId, PlayerView, QuantileSketch, RolloutPolicy,
    Rules, SeedSchedule, SeedScheduleKind, Slot, TriggerTally, WorldSeed, SEMANTICS_REV,
};

const USAGE: &str = "\
usage: eval_probe <subcommand> [options]

  run <players> <games> [--contestant <name>] [--weights v1|pre-freeze]
                        [--master <u64> | --os-entropy]
                        [--invariants | --no-invariants]
                        [--bootstrap <B>] [--blocks <list>]
      Generate a self-play corpus and report the blocks in <list> (default all):
        features  per-feature AUC per stage tercile, game-clustered bootstrap
        proxy     progress_ppm against the post-hoc terciles
        counters  recovery share, cap binds, game length, end-trigger mix

  table [--weights v1|pre-freeze]
      Print a weight table with its provenance identity. No games are played.

contestants:
  greedy-v1            the honest arena driver (observe -> sampler -> handle)
  uniform | retreat-averse | family-weighted
                       a RolloutPolicy rung through the RAW-STATE driver, which
                       is privileged play by construction and labelled as such
                       (legitimate for a corpus: the bot is not the subject)
";

/// This task's committed diagnostic master. The NAME of the constructor is the
/// waiver (`game::seeds`): the freeze is a measurement artifact that must
/// reproduce, not a quoted strength claim.
const DEFAULT_MASTER: u64 = 0xE7A1_2026_0727_0001;

/// Bootstrap replicates, default. R1's own B for the same estimator.
const DEFAULT_BOOTSTRAP: u32 = 2_000;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("run") => cmd_run(&args[1..]),
        Some("table") => cmd_table(&args[1..]),
        _ => Err(USAGE.to_string()),
    };
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(2);
    }
}

// ---------------------------------------------------------------------------
// argv
// ---------------------------------------------------------------------------

/// Split argv into positionals and `--flag [value]` pairs.
///
/// An unknown flag is an ERROR, never a silent no-op — `examples/arena`'s rule,
/// and for its reason: a mistyped `--os-entropi` that quietly left the run on a
/// diagnostic master would put a fixed master behind a figure published as
/// freshly drawn.
fn split_args(
    args: &[String],
    valued: &[&str],
    bare: &[&str],
) -> Result<(Vec<String>, BTreeMap<String, String>), String> {
    let mut positional = Vec::new();
    let mut flags: BTreeMap<String, String> = BTreeMap::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some(name) = arg.strip_prefix("--") {
            if valued.contains(&name) {
                let value =
                    args.get(i + 1).ok_or_else(|| format!("--{name} needs a value\n\n{USAGE}"))?;
                flags.insert(name.to_string(), value.clone());
                i += 2;
                continue;
            }
            if bare.contains(&name) {
                flags.insert(name.to_string(), String::new());
                i += 1;
                continue;
            }
            return Err(format!("unknown flag --{name}\n\n{USAGE}"));
        }
        positional.push(arg.clone());
        i += 1;
    }
    Ok((positional, flags))
}

fn parse_u64(raw: &str) -> Result<u64, String> {
    let cleaned = raw.replace('_', "");
    let parsed = match cleaned.strip_prefix("0x").or_else(|| cleaned.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => cleaned.parse::<u64>(),
    };
    parsed.map_err(|e| format!("{raw:?} is not a u64: {e}"))
}

fn master_from(flags: &BTreeMap<String, String>) -> Result<SeedScheduleKind, String> {
    match (flags.get("master"), flags.contains_key("os-entropy")) {
        (Some(_), true) => Err("--master and --os-entropy are mutually exclusive".to_string()),
        (Some(raw), false) => parse_u64(raw).map(SeedScheduleKind::Diagnostic),
        (None, true) => Ok(SeedScheduleKind::OsEntropy),
        (None, false) => Ok(SeedScheduleKind::Diagnostic(DEFAULT_MASTER)),
    }
}

/// Which weight table the corpus bot evaluates with and the benchmark row
/// reports.
fn weights_from(flags: &BTreeMap<String, String>) -> Result<(WeightTable, &'static str), String> {
    match flags.get("weights").map(String::as_str) {
        None | Some("v1") => Ok((WeightTable::V1, "v1")),
        Some("pre-freeze") => Ok((WeightTable::PRE_FREEZE_2026_07_27, "pre-freeze")),
        Some(other) => Err(format!("unknown --weights {other:?} (v1 | pre-freeze)")),
    }
}

/// Who plays the corpus.
///
/// Named `Probe…` rather than `Contestant` because `game::arena::Contestant` is
/// a public type with a DIFFERENT shape (four variants, knobs on three of them),
/// and its greedy arm is not this one: two incompatible answers to
/// `grep 'enum Contestant'` in one crate is a trap, not an abbreviation
/// (six-lens review).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ProbeContestant {
    /// Greedy-v1 at every seat, through the HONEST driver.
    Greedy,
    /// One policy rung at every seat, through the RAW-STATE driver.
    Policy(RolloutPolicy),
}

impl ProbeContestant {
    fn parse(spec: &str) -> Result<Self, String> {
        if spec == GREEDY_CONTROLLER {
            return Ok(ProbeContestant::Greedy);
        }
        RolloutPolicy::from_str_name(spec).map(ProbeContestant::Policy).ok_or_else(|| {
            format!("unknown contestant {spec:?} (greedy-v1 | uniform | retreat-averse | family-weighted)")
        })
    }

    fn echo(self, weights: &WeightTable, label: &str) -> String {
        match self {
            ProbeContestant::Greedy => format!(
                "{GREEDY_CONTROLLER}{{m={DEFAULT_RETREAT_SAMPLES},weights={label}:{:#018x}}} \
                 [honest driver: observe -> sampler K=1 -> handle]",
                weight_table_identity(weights) as u64,
            ),
            ProbeContestant::Policy(p) => {
                let (name, version, params) = p.descriptor();
                let knobs: Vec<String> =
                    params.iter().map(|(k, v)| format!("{k}={v}")).collect();
                format!(
                    "{name}@v{version}{{{}}} [RAW-STATE driver: privileged play, corpus only]",
                    knobs.join(","),
                )
            }
        }
    }
}

fn profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

fn master_echo(kind: SeedScheduleKind) -> String {
    match kind {
        SeedScheduleKind::Diagnostic(m) => format!("diagnostic:{m:#018x}"),
        SeedScheduleKind::OsEntropy => "os-entropy (drawn per run, printed below)".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Rows: what gets an AUC
// ---------------------------------------------------------------------------

/// One reported row. Sixteen are the shipped features; the last two exist to
/// answer questions the shipped list cannot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Row {
    /// A shipped feature of `Feature::ALL`, in canonical order.
    Feature(Feature),
    /// `assistants_in_hand_core + assistants_in_hand_excess` — the feature R1-4
    /// actually measured (0.6026 early), before T3 split it. Reported so the
    /// two halves have something to be compared AGAINST.
    HandUnsplit,
    /// The evaluator's own positional total: the benchmark row every feature is
    /// trying to beat, and already a relative quantity.
    EvalTotal,
}

impl Row {
    fn name(self) -> &'static str {
        match self {
            Row::Feature(f) => f.name(),
            Row::HandUnsplit => "assistants_in_hand (unsplit)",
            Row::EvalTotal => "EVAL TOTAL (benchmark)",
        }
    }

    /// The weight pair the row's measured sign is judged against, or `None` for
    /// the two synthetic rows (which ship no weight).
    fn weights(self, table: &WeightTable) -> Option<(i64, i64)> {
        match self {
            Row::Feature(f) => Some(table.get(f)),
            Row::HandUnsplit | Row::EvalTotal => None,
        }
    }

    fn all() -> Vec<Row> {
        let mut rows: Vec<Row> = Feature::ALL.into_iter().map(Row::Feature).collect();
        rows.push(Row::HandUnsplit);
        rows.push(Row::EvalTotal);
        rows
    }
}

/// The ROWS_LEN values a single view contributes, relative and absolute.
///
/// Relative = `viewer − max over opponents`, matching the shipped evaluator's
/// relative form. The ORIENTATION question `eval::opponent_extreme` answers
/// (which opponent a negative-weight term should pick) does not arise here: a
/// plain maximum is used, it is EXACT at 2P — the mandated freeze corpus, where
/// the maximum is over the one opponent — and at 3–4P it is a stated
/// convention, recorded rather than assumed away. AUC is a separation
/// statistic, so a feature the plain max orients "backwards" reads below 0.5
/// and is still measured, just from the other side.
fn row_values(
    view: &PlayerView,
    rules: &Rules,
    weights: &WeightTable,
    rows: &[Row],
    out_rel: &mut [f64],
    out_abs: &mut [f64],
) {
    // ONCE per view — see the module doc. Everything below indexes it.
    let structure = ScoreStructure::derive(view, rules);
    let Some(me) = view.players.iter().find(|p| p.id == view.viewer) else {
        // `eval::seated_viewer`'s assert, for the same reason and at the same
        // strength: the caller allocates `out_rel`/`out_abs` OUTSIDE its
        // (turn × seat) loop and folds them unconditionally, so an early return
        // that leaves them untouched double-counts the previous seat's row into
        // this one. `observe` cannot produce a viewer-less view; a hand-built
        // one can, and this is a probe, so it must fail loudly rather than
        // report a corpus statistic that is quietly one seat wrong.
        debug_assert!(
            false,
            "probe got a view whose own viewer seat {:?} is not among its players",
            view.viewer,
        );
        return;
    };
    let total = score(view, rules, weights) as f64;

    for (slot, row) in rows.iter().enumerate() {
        let (mine, theirs) = match row {
            Row::Feature(f) => {
                let mine = structure.value_of(me, *f);
                let theirs = view
                    .players
                    .iter()
                    .filter(|p| p.id != me.id && !p.id.is_dummy())
                    .map(|p| structure.value_of(p, *f))
                    .max()
                    .unwrap_or(0);
                (mine as f64, theirs as f64)
            }
            Row::HandUnsplit => {
                let sum = |p: &yokohama_core::VisiblePlayer| {
                    structure.value_of(p, Feature::AssistantsInHandCore)
                        + structure.value_of(p, Feature::AssistantsInHandExcess)
                };
                let theirs = view
                    .players
                    .iter()
                    .filter(|p| p.id != me.id && !p.id.is_dummy())
                    .map(sum)
                    .max()
                    .unwrap_or(0);
                (sum(me) as f64, theirs as f64)
            }
            // Already relative, and an "absolute" evaluator total is not a
            // defined quantity — the row reports the same number in both
            // columns rather than inventing one.
            Row::EvalTotal => (total, 0.0),
        };
        out_rel[slot] = mine - theirs;
        out_abs[slot] = mine;
    }
}

// ---------------------------------------------------------------------------
// Cells and stages
// ---------------------------------------------------------------------------

/// A stage bucket assigned POST HOC from the game's realized length — the only
/// honest way to label a stage, since nothing in-game knows how long the game
/// will turn out to be. The shipped `progress_ppm` proxy is measured AGAINST
/// these labels in block (c), which is the whole point of having both.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Early,
    Mid,
    Late,
}

const STAGES: [Stage; 3] = [Stage::Early, Stage::Mid, Stage::Late];

impl Stage {
    fn name(self) -> &'static str {
        match self {
            Stage::Early => "EARLY",
            Stage::Mid => "MID",
            Stage::Late => "LATE",
        }
    }

    fn index(self) -> usize {
        match self {
            Stage::Early => 0,
            Stage::Mid => 1,
            Stage::Late => 2,
        }
    }
}

/// Which tercile the `turn_index`-th of `turns` turn starts falls in.
///
/// Integer arithmetic, so the split is exact and machine-independent: turn
/// index `i` is EARLY while `3i < turns`, MID while `3i < 2·turns`, LATE
/// after. A game shorter than three turns simply has empty later buckets, and
/// an empty bucket contributes no cell.
fn stage_of(turn_index: usize, turns: usize) -> Stage {
    let scaled = 3 * turn_index;
    if scaled < turns {
        Stage::Early
    } else if scaled < 2 * turns {
        Stage::Mid
    } else {
        Stage::Late
    }
}

/// One (game × seat × stage) analysis cell: the MEAN of each row over the
/// snapshots in the block, plus the block's outcome label.
///
/// `game` is the CLUSTER id the bootstrap resamples on — every cell of one game
/// travels together, which is what makes the interval honest about two seats
/// sharing an outcome and a hundred turns sharing a trajectory.
struct Cell {
    game: u32,
    stage: Stage,
    label: bool,
    rel: Vec<f64>,
    abs: Vec<f64>,
}

/// A per-(game, seat, stage) accumulator, folded into a [`Cell`] at game end.
struct CellAcc {
    n: u64,
    rel: Vec<f64>,
    abs: Vec<f64>,
}

impl CellAcc {
    fn new(rows: usize) -> Self {
        CellAcc { n: 0, rel: vec![0.0; rows], abs: vec![0.0; rows] }
    }
}

// ---------------------------------------------------------------------------
// AUC and the game-clustered bootstrap
// ---------------------------------------------------------------------------

/// The rank-based AUC of one column under per-GAME multiplicities.
///
/// `sorted` is `(value, game, label)` ascending by value; `mult[game]` is how
/// many times that game appears in the current bootstrap resample (all ones for
/// the point estimate). Ties are handled by tie-GROUP, each cross-class pair
/// inside a group counting a half — the standard mid-rank convention, and not a
/// detail here: an integer feature like `five_power_tokens` is mostly ties, and
/// counting them as wins would push its AUC toward 1 for free.
///
/// `None` when one of the two classes is empty under the resample — an AUC over
/// no discordant pairs is not 0.5, it is undefined, and a resample that loses a
/// class must be reported as such rather than filled in.
fn auc_weighted(sorted: &[(f64, u32, bool)], mult: &[u32]) -> Option<f64> {
    let mut pos_total = 0.0f64;
    let mut neg_total = 0.0f64;
    let mut neg_before = 0.0f64;
    let mut concordant = 0.0f64;

    let mut i = 0;
    while i < sorted.len() {
        let value = sorted[i].0;
        let (mut pos, mut neg) = (0.0f64, 0.0f64);
        while i < sorted.len() && sorted[i].0 == value {
            let w = f64::from(mult[sorted[i].1 as usize]);
            if sorted[i].2 {
                pos += w;
            } else {
                neg += w;
            }
            i += 1;
        }
        concordant += pos * (neg_before + 0.5 * neg);
        neg_before += neg;
        pos_total += pos;
        neg_total += neg;
    }

    if pos_total == 0.0 || neg_total == 0.0 {
        return None;
    }
    Some(concordant / (pos_total * neg_total))
}

/// The nearest-rank order statistic `num/den` of `n` sorted values — the
/// crate's ONE quantile rule (`metrics::nearest_rank_index`), mirrored here
/// because it is `pub(crate)`. At B = 2,000 the 95% endpoints are the 50th and
/// 1,950th replicates.
fn nearest_rank_index(n: usize, num: u64, den: u64) -> usize {
    let n = n as u64;
    let rank = (n * num).div_ceil(den).max(1);
    (rank.min(n) - 1) as usize
}

/// A 64-bit mixer for deriving printed seeds from DESIGN parameters. Mirrors
/// `seeds::splitmix64`, which is crate-private; nothing about a corpus outcome
/// enters here, which is the property that matters — a resample stream read off
/// the data would make the interval a function of the data twice over.
fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// One row × stage column, pre-sorted once so a bootstrap replicate costs O(n)
/// instead of O(n log n).
struct Column {
    rel: Vec<(f64, u32, bool)>,
    abs: Vec<(f64, u32, bool)>,
}

/// A point estimate with its game-clustered interval and the cell counts behind
/// it.
#[derive(Clone, Copy)]
struct AucRow {
    point: Option<f64>,
    lo: f64,
    hi: f64,
    cells: usize,
    winners: usize,
    /// Every cell in the column held the SAME value, so the AUC is 0.5 by
    /// construction and every bootstrap replicate is too.
    ///
    /// Reported as its own state rather than as an interval, because a printed
    /// `[0.5000, 0.5000]` is a zero-width interval claiming a measurement it did
    /// not make — the same refusal `arena_gate`'s §0-R3 exists to enforce one
    /// estimator over. It happens for real here: nobody owns a trading house in
    /// the first tercile, so `trading_houses_built` is identically zero there.
    constant: bool,
    /// Mean of the ABSOLUTE (viewer-only) value over the column's cells, so the
    /// hand-set magnitudes can be read against the SCALE of the thing they
    /// multiply rather than against each other alone.
    mean_abs: f64,
}

impl AucRow {
    fn render(&self) -> String {
        if self.constant {
            return "  constant (no signal) ".to_string();
        }
        match self.point {
            None => "    --  (one class empty)  ".to_string(),
            Some(p) => format!("{p:.4} [{:.4},{:.4}]", self.lo, self.hi),
        }
    }
}

/// Does every cell of this column carry the same value? See
/// [`AucRow::constant`].
fn is_constant(column: &[(f64, u32, bool)]) -> bool {
    column.first().is_some_and(|first| column.iter().all(|c| c.0 == first.0))
}

// ---------------------------------------------------------------------------
// `run`
// ---------------------------------------------------------------------------

/// Which report blocks a run prints.
#[derive(Clone, Copy)]
struct Blocks {
    features: bool,
    proxy: bool,
    counters: bool,
}

fn blocks_from(flags: &BTreeMap<String, String>) -> Result<Blocks, String> {
    let Some(list) = flags.get("blocks") else {
        return Ok(Blocks { features: true, proxy: true, counters: true });
    };
    let mut blocks = Blocks { features: false, proxy: false, counters: false };
    for name in list.split(',').filter(|s| !s.is_empty()) {
        match name {
            "all" => blocks = Blocks { features: true, proxy: true, counters: true },
            "features" => blocks.features = true,
            "proxy" => blocks.proxy = true,
            "counters" => blocks.counters = true,
            other => return Err(format!("unknown block {other:?} (features | proxy | counters)")),
        }
    }
    Ok(blocks)
}

/// Everything one corpus pass collects.
struct Corpus {
    rows: Vec<Row>,
    cells: Vec<Cell>,
    games: u32,
    snapshots: u64,
    /// `(progress_ppm, post-hoc stage)` at every turn start (seat-invariant, so
    /// one observation per turn start, not per view).
    proxy: Vec<(u32, Stage)>,
    turns: QuantileSketch,
    moves: QuantileSketch,
    trigger: TriggerTally,
    applied: u64,
    retreat_picks: u64,
    nonzero_recovery: u64,
    area_actions: u64,
    builds: u64,
    /// Greedy only; zero under a policy rung, which has no cap to bind.
    cap_binds: u64,
    decisions: u64,
    candidates: u64,
    wall: f64,
}

fn generate(
    players: u8,
    games: u32,
    contestant: ProbeContestant,
    weights: &WeightTable,
    rules: &Rules,
    master: SeedScheduleKind,
    invariants: bool,
) -> Result<(Corpus, u64), String> {
    let (mut schedule, master_value) = match master {
        SeedScheduleKind::Diagnostic(m) => (SeedSchedule::diagnostic(m), m),
        SeedScheduleKind::OsEntropy => SeedSchedule::from_os_entropy(),
    };

    let rows = Row::all();
    let mut corpus = Corpus {
        rows: rows.clone(),
        cells: Vec::new(),
        games: 0,
        snapshots: 0,
        proxy: Vec::new(),
        turns: QuantileSketch::default(),
        moves: QuantileSketch::default(),
        trigger: TriggerTally::default(),
        applied: 0,
        retreat_picks: 0,
        nonzero_recovery: 0,
        area_actions: 0,
        builds: 0,
        cap_binds: 0,
        decisions: 0,
        candidates: 0,
        wall: 0.0,
    };

    let started = Instant::now();
    for game in 0..games {
        let seed = schedule.next_world();
        let (stats, tallies) =
            play_one(seed, players, contestant, weights, rules, master_value, invariants)?;

        corpus.games += 1;
        corpus.turns.observe(u64::from(stats.turns));
        corpus.moves.observe(stats.moves_applied);
        corpus.trigger.observe(stats.trigger.as_ref());
        corpus.applied += stats.coverage.applied();
        corpus.retreat_picks += stats.coverage.retreat_picks();
        corpus.nonzero_recovery += stats.coverage.nonzero_recovery_retreats();
        corpus.area_actions += stats.coverage.area_actions();
        corpus.builds += stats.coverage.builds();
        for tally in &tallies {
            corpus.cap_binds += tally.placement_cap_binds();
            corpus.decisions += tally.explained();
            corpus.candidates += tally.candidates_scored();
        }

        snapshot_game(&stats, players, weights, rules, &rows, game, &mut corpus)?;
    }
    corpus.wall = started.elapsed().as_secs_f64();

    Ok((corpus, master_value))
}

/// Play ONE game with the named contestant, returning its stats plus the
/// per-seat Greedy tallies (empty for a policy rung).
fn play_one(
    seed: WorldSeed,
    players: u8,
    contestant: ProbeContestant,
    weights: &WeightTable,
    rules: &Rules,
    master: u64,
    invariants: bool,
) -> Result<(ArenaGameStats, Vec<ExplanationTally>), String> {
    match contestant {
        ProbeContestant::Greedy => {
            // Cloned out BEFORE the agents move into the driver — the
            // `HonestPathTally` precedent, and the only way to read the
            // counters afterwards.
            let handles: std::sync::Arc<std::sync::Mutex<Vec<ExplanationTally>>> =
                std::sync::Arc::default();
            let sink = std::sync::Arc::clone(&handles);
            let table = *weights;
            let mut make = |seat: PlayerId, _n: u8| -> Box<dyn yokohama_core::ArenaAgent> {
                let agent = GreedyAgent::with_weights(
                    rules,
                    table,
                    DEFAULT_RETREAT_SAMPLES,
                    agent_stream_seed(seed, seat, Slot(seat.0)),
                );
                sink.lock().expect("tally sink").push(agent.tally());
                Box::new(agent)
            };
            let stats = run_arena_game(seed, players, rules, &mut make, invariants)
                .map_err(|e| format!("game at seed {:#018x}: {e}", seed.0))?;
            let tallies = handles.lock().expect("tally sink").clone();
            Ok((stats, tallies))
        }
        ProbeContestant::Policy(policy) => {
            let seats = vec![policy; players as usize];
            let stats = run_raw_game(seed, players, rules, &seats, master, invariants)
                .map_err(|e| format!("game at seed {:#018x}: {e}", seed.0))?;
            Ok((stats, Vec::new()))
        }
    }
}

/// Walk one finished game's record and fold every turn start into cells.
///
/// The walk is a REPLAY rather than a hook inside the driver: `run_arena_game`
/// hands back a `GameRecord`, and folding it here means the snapshots are taken
/// from exactly the states the recorded game passed through — the same fold
/// `replay`/`replay_strict` use, one event at a time, so this instrument cannot
/// have its own idea of what a game did.
fn snapshot_game(
    stats: &ArenaGameStats,
    players: u8,
    weights: &WeightTable,
    rules: &Rules,
    rows: &[Row],
    game: u32,
    corpus: &mut Corpus,
) -> Result<(), String> {
    let record = &stats.record;
    let pending = new_game(record.config.clone(), rules, record.seed)
        .map_err(|e| format!("replay setup at game {game}: {e}"))?;
    let mut state = finalize_setup(pending, &record.choices)
        .map_err(|e| format!("replay finalize at game {game}: {e}"))?;

    // Pass 1: collect the turn-start states. Cheap in memory next to the fold
    // itself (a few dozen clones per game) and it buys the POST-HOC label: the
    // tercile of a turn is not knowable until the game's turn count is.
    let mut starts: Vec<GameState> = vec![state.clone()];
    let mut last_turn = state.turn_number;
    for (i, event) in record.events.iter().enumerate() {
        state = replay_from(&state, std::slice::from_ref(event), rules)
            .map_err(|e| format!("replay game {game} event {i}: {e}"))?;
        if state.turn_number != last_turn {
            last_turn = state.turn_number;
            starts.push(state.clone());
        }
    }

    // Pass 2: one accumulator per (seat, stage), folded at the end.
    let turns = starts.len();
    let mut accs: Vec<CellAcc> = (0..usize::from(players) * STAGES.len())
        .map(|_| CellAcc::new(rows.len()))
        .collect();
    let mut rel = vec![0.0f64; rows.len()];
    let mut abs = vec![0.0f64; rows.len()];

    for (index, start) in starts.iter().enumerate() {
        let stage = stage_of(index, turns);
        for seat in 0..players {
            let view = observe(start, PlayerId(seat))
                .map_err(|e| format!("observe game {game} turn {index} seat {seat}: {e}"))?;
            // The proxy is public-board-derived and therefore seat-invariant;
            // one observation per turn start, taken at seat 0.
            if seat == 0 {
                corpus.proxy.push((progress_ppm(&view, rules), stage));
            }
            row_values(&view, rules, weights, rows, &mut rel, &mut abs);
            let acc = &mut accs[usize::from(seat) * STAGES.len() + stage.index()];
            acc.n += 1;
            for slot in 0..rows.len() {
                acc.rel[slot] += rel[slot];
                acc.abs[slot] += abs[slot];
            }
            corpus.snapshots += 1;
        }
    }

    for seat in 0..players {
        for stage in STAGES {
            let acc = &accs[usize::from(seat) * STAGES.len() + stage.index()];
            // A game too short to reach a tercile contributes no cell there.
            // Dropping it is right — a mean over zero snapshots is not a
            // measurement — and it is why the stage rides IN the cell.
            if acc.n == 0 {
                continue;
            }
            let n = acc.n as f64;
            corpus.cells.push(Cell {
                game,
                stage,
                label: stats.result.winner == PlayerId(seat),
                rel: acc.rel.iter().map(|v| v / n).collect(),
                abs: acc.abs.iter().map(|v| v / n).collect(),
            });
        }
    }
    Ok(())
}

fn cmd_run(args: &[String]) -> Result<(), String> {
    let (positional, flags) = split_args(
        args,
        &["contestant", "weights", "master", "bootstrap", "blocks"],
        &["os-entropy", "invariants", "no-invariants"],
    )?;
    if positional.len() != 2 {
        return Err(format!("run takes 2 positional arguments\n\n{USAGE}"));
    }
    let players: u8 = positional[0].parse().map_err(|e| format!("<players>: {e}"))?;
    let games: u32 = positional[1].parse().map_err(|e| format!("<games>: {e}"))?;
    if games == 0 {
        return Err("run needs at least one game".to_string());
    }
    let contestant = ProbeContestant::parse(
        flags.get("contestant").map(String::as_str).unwrap_or(GREEDY_CONTROLLER),
    )?;
    let (weights, weights_label) = weights_from(&flags)?;
    let master = master_from(&flags)?;
    let bootstrap: u32 = match flags.get("bootstrap") {
        None => DEFAULT_BOOTSTRAP,
        Some(raw) => raw.parse().map_err(|e| format!("--bootstrap: {e}"))?,
    };
    if flags.contains_key("invariants") && flags.contains_key("no-invariants") {
        return Err("--invariants and --no-invariants are mutually exclusive".to_string());
    }
    let invariants = !flags.contains_key("no-invariants");
    let blocks = blocks_from(&flags)?;

    let rules = Rules::standard();
    let (corpus, master_value) =
        generate(players, games, contestant, &weights, &rules, master, invariants)?;

    // Derived from the DESIGN, printed, and never from the outcomes.
    let resample_seed =
        mix64(master_value ^ mix64(u64::from(games) << 20 | u64::from(players) << 8))
            ^ mix64(u64::from(bootstrap));

    println!("=== eval_probe run ===");
    println!(
        "  build={} semantics_rev={SEMANTICS_REV} master={}",
        profile(),
        master_echo(master),
    );
    if matches!(master, SeedScheduleKind::OsEntropy) {
        println!("  drawn master = {master_value:#018x}");
    }
    println!("  contestant={}", contestant.echo(&weights, weights_label));
    println!(
        "  players={players} games={} snapshots={} cells={} per-move-invariants={invariants}",
        corpus.games,
        corpus.snapshots,
        corpus.cells.len(),
    );
    println!("  weights[{weights_label}] early={:?}", weights.early);
    println!("  weights[{weights_label}] late ={:?}", weights.late);
    println!(
        "  weight identity={:#018x}   bootstrap B={bootstrap} resample_seed={resample_seed:#018x}",
        weight_table_identity(&weights) as u64,
    );
    println!("  corpus wall {:.2}s ({:.1} ms/game, {} build)", corpus.wall,
        corpus.wall * 1000.0 / f64::from(corpus.games.max(1)), profile());
    if profile() == "debug" {
        println!(
            "  WARNING: DEBUG build. Wall figures are unoptimized; the AUCs and counters are \
             deterministic and unaffected, but re-run with --release before recording anything."
        );
    }

    if blocks.features {
        report_features(&corpus, &weights, bootstrap, resample_seed);
    }
    if blocks.proxy {
        report_proxy(&corpus, players);
    }
    if blocks.counters {
        report_counters(&corpus, contestant);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Block (b): per-feature AUC
// ---------------------------------------------------------------------------

/// Build the per-(row, stage) sorted columns.
///
/// Sorted ONCE here so a bootstrap replicate is a linear walk: at B = 2,000 over
/// 54 columns of 4,000 cells, re-sorting per replicate would be the whole cost
/// of the instrument.
fn columns(corpus: &Corpus) -> Vec<Vec<Column>> {
    let mut out: Vec<Vec<Column>> = (0..corpus.rows.len())
        .map(|_| STAGES.iter().map(|_| Column { rel: Vec::new(), abs: Vec::new() }).collect())
        .collect();
    for cell in &corpus.cells {
        for (slot, row) in out.iter_mut().enumerate() {
            let col = &mut row[cell.stage.index()];
            col.rel.push((cell.rel[slot], cell.game, cell.label));
            col.abs.push((cell.abs[slot], cell.game, cell.label));
        }
    }
    for row in out.iter_mut() {
        for col in row.iter_mut() {
            col.rel.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("feature values are finite"));
            col.abs.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("feature values are finite"));
        }
    }
    out
}

/// How a measured stage AUC sits against the weight that ships for that stage.
fn disposition(auc: &AucRow, weight: Option<i64>) -> &'static str {
    let (Some(_point), Some(w)) = (auc.point, weight) else {
        return "-";
    };
    if auc.constant {
        return "no-signal(const)";
    }
    if w == 0 {
        return "zero-weight";
    }
    let claims_positive = w > 0;
    // "Materially moved" (spec §2.4), operationalized: the measured sign's
    // game-clustered 95% interval EXCLUDES the shipped direction.
    if claims_positive && auc.hi < 0.5 {
        return "CONTRADICTED";
    }
    if !claims_positive && auc.lo > 0.5 {
        return "CONTRADICTED";
    }
    if claims_positive && auc.lo > 0.5 {
        return "confirmed";
    }
    if !claims_positive && auc.hi < 0.5 {
        return "confirmed";
    }
    "no-signal"
}

/// Whether a linear early→late shape is adequate for this row: is the MID point
/// estimate inside the interval the two endpoints span?
///
/// RECORDED, never gated (spec §2.4). A row whose mid sits outside its
/// endpoints is telling us a straight line between two weights cannot represent
/// it — useful input to a later non-linear stage shape, and nothing v1 acts on.
fn shape(early: &AucRow, mid: &AucRow, late: &AucRow) -> String {
    // A CONSTANT column's point is the 0.5 the estimator itself refuses to
    // interval — a shape verdict resting on it would apply the refusal rule
    // inconsistently two columns apart (T4 review), so the shape is
    // undetermined whenever any endpoint is constant.
    if early.constant || mid.constant || late.constant {
        return "undetermined(const)".to_string();
    }
    let (Some(e), Some(m), Some(l)) = (early.point, mid.point, late.point) else {
        return "-".to_string();
    };
    let (lo, hi) = if e <= l { (e, l) } else { (l, e) };
    if m >= lo && m <= hi {
        "monotone".to_string()
    } else {
        let excess = if m < lo { lo - m } else { m - hi };
        format!("NON-MONOTONE(+{excess:.4})")
    }
}

fn report_features(
    corpus: &Corpus,
    weights: &WeightTable,
    bootstrap: u32,
    resample_seed: u64,
) {
    let cols = columns(corpus);
    let games = corpus.games as usize;

    // One multiplicity draw per replicate, REUSED across every (row, stage)
    // column: the cluster resample is a property of the replicate, not of the
    // column, and drawing it per column would make the rows incomparable.
    let mut rng = ChaCha8Rng::seed_from_u64(resample_seed);
    let mut replicates: Vec<Vec<Vec<f64>>> = (0..corpus.rows.len())
        .map(|_| STAGES.iter().map(|_| Vec::with_capacity(bootstrap as usize)).collect())
        .collect();
    let mut mult = vec![0u32; games];
    for _ in 0..bootstrap {
        mult.iter_mut().for_each(|m| *m = 0);
        for _ in 0..games {
            mult[rng.gen_range(0..games)] += 1;
        }
        for (slot, row) in cols.iter().enumerate() {
            for (s, col) in row.iter().enumerate() {
                if let Some(a) = auc_weighted(&col.rel, &mult) {
                    replicates[slot][s].push(a);
                }
            }
        }
    }

    let ones = vec![1u32; games];
    let blank = AucRow {
        point: None,
        lo: f64::NAN,
        hi: f64::NAN,
        cells: 0,
        winners: 0,
        constant: false,
        mean_abs: f64::NAN,
    };
    let mut table: Vec<[AucRow; 3]> = Vec::with_capacity(corpus.rows.len());
    for (slot, row) in cols.iter().enumerate() {
        let mut stage_rows = [blank; 3];
        for (s, col) in row.iter().enumerate() {
            let reps = &mut replicates[slot][s];
            reps.sort_by(|a, b| a.partial_cmp(b).expect("AUCs are finite"));
            let (lo, hi) = if reps.is_empty() {
                (f64::NAN, f64::NAN)
            } else {
                (
                    reps[nearest_rank_index(reps.len(), 25, 1000)],
                    reps[nearest_rank_index(reps.len(), 975, 1000)],
                )
            };
            let n = col.abs.len().max(1) as f64;
            stage_rows[s] = AucRow {
                point: auc_weighted(&col.rel, &ones),
                lo,
                hi,
                cells: col.rel.len(),
                winners: col.rel.iter().filter(|c| c.2).count(),
                constant: is_constant(&col.rel),
                mean_abs: col.abs.iter().map(|c| c.0).sum::<f64>() / n,
            };
        }
        table.push(stage_rows);
    }

    println!();
    println!(
        "--- (b) per-feature AUC vs eventual winner | relative form | cells = game x seat x stage \
         | game-clustered bootstrap 95% (B={bootstrap}) ---"
    );
    println!(
        "  cells per stage: EARLY {} (winners {}) · MID {} ({}) · LATE {} ({})",
        table[0][0].cells, table[0][0].winners,
        table[0][1].cells, table[0][1].winners,
        table[0][2].cells, table[0][2].winners,
    );
    println!(
        "  {:<28} {:>4} {:<22} {:<16} {:<22} {:>4} {:<22} {:<16} shape",
        "row", "w_e", "EARLY auc [95%]", "early-disp", "MID auc [95%]", "w_l",
        "LATE auc [95%]", "late-disp",
    );
    for (slot, row) in corpus.rows.iter().enumerate() {
        let [early, mid, late] = &table[slot];
        let w = row.weights(weights);
        println!(
            "  {:<28} {:>4} {:<22} {:<16} {:<22} {:>4} {:<22} {:<16} {}",
            row.name(),
            w.map_or("-".to_string(), |(e, _)| e.to_string()),
            early.render(),
            disposition(early, w.map(|(e, _)| e)),
            mid.render(),
            w.map_or("-".to_string(), |(_, l)| l.to_string()),
            late.render(),
            disposition(late, w.map(|(_, l)| l)),
            shape(early, mid, late),
        );
    }

    // The absolute form, point estimates only: an interval per row here would
    // double the bootstrap for a column the evaluator does not ship. R1-9
    // measured relative beating absolute ~1.5x in separation; this is the row
    // that lets that be re-checked rather than assumed. The MEAN columns are
    // the SCALE of each raw value — the thing a hand-set magnitude multiplies,
    // and the only way to read two weights against each other honestly.
    println!();
    println!("  absolute (viewer-only) form: AUC point estimates, and the raw value's own scale");
    println!(
        "  {:<28} {:>8} {:>8} {:>8}   {:>9} {:>9} {:>9}",
        "row", "aucEARLY", "aucMID", "aucLATE", "meanEARLY", "meanMID", "meanLATE",
    );
    for (slot, row) in corpus.rows.iter().enumerate() {
        if matches!(row, Row::EvalTotal) {
            continue;
        }
        let cells = &cols[slot];
        let render = |c: &Column| {
            auc_weighted(&c.abs, &ones).map_or("--".to_string(), |a| format!("{a:.4}"))
        };
        println!(
            "  {:<28} {:>8} {:>8} {:>8}   {:>9.3} {:>9.3} {:>9.3}",
            row.name(),
            render(&cells[0]),
            render(&cells[1]),
            render(&cells[2]),
            table[slot][0].mean_abs,
            table[slot][1].mean_abs,
            table[slot][2].mean_abs,
        );
    }
}

// ---------------------------------------------------------------------------
// Block (c): proxy validation
// ---------------------------------------------------------------------------

/// The shipped proxy's predicted tercile under fixed one-third cuts.
fn predicted_stage(ppm: u32, t1: u32, t2: u32) -> Stage {
    if ppm < t1 {
        Stage::Early
    } else if ppm < t2 {
        Stage::Mid
    } else {
        Stage::Late
    }
}

/// `[truth][predicted]` counts and the accuracy they imply.
fn confusion(points: &[(u32, Stage)], t1: u32, t2: u32) -> ([[u64; 3]; 3], f64) {
    let mut matrix = [[0u64; 3]; 3];
    for (ppm, truth) in points {
        matrix[truth.index()][predicted_stage(*ppm, t1, t2).index()] += 1;
    }
    let correct: u64 = (0..3).map(|i| matrix[i][i]).sum();
    let total: u64 = matrix.iter().flatten().sum();
    (matrix, if total == 0 { f64::NAN } else { correct as f64 / total as f64 })
}

/// The best `(t1, t2)` over the distinct proxy values, and its accuracy.
///
/// IN-SAMPLE and therefore OPTIMISTIC — the same criticism R1's critic made of
/// its own 0.6419/0.6466/0.6667 figures, restated here rather than quietly
/// inherited. It is reported as the CEILING a hard three-way switch could
/// reach on this corpus even with perfect hindsight, which is what makes the
/// fixed-cut number below it interpretable.
fn best_thresholds(points: &[(u32, Stage)]) -> (u32, u32, f64) {
    let mut values: Vec<u32> = points.iter().map(|(p, _)| *p).collect();
    values.sort_unstable();
    values.dedup();
    let mut best = (0u32, 0u32, f64::NAN);
    for (i, t1) in values.iter().enumerate() {
        for t2 in values.iter().skip(i) {
            let (_, accuracy) = confusion(points, *t1, *t2);
            if best.2.is_nan() || accuracy > best.2 {
                best = (*t1, *t2, accuracy);
            }
        }
    }
    best
}

fn report_proxy(corpus: &Corpus, players: u8) {
    let points = &corpus.proxy;
    let mut distinct: Vec<u32> = points.iter().map(|(p, _)| *p).collect();
    distinct.sort_unstable();
    distinct.dedup();

    let t1 = PROGRESS_FULL_PPM / 3;
    let t2 = 2 * PROGRESS_FULL_PPM / 3;
    let (matrix, accuracy) = confusion(points, t1, t2);
    let mut class_totals = [0u64; 3];
    for (i, total) in class_totals.iter_mut().enumerate() {
        *total = matrix[i].iter().sum();
    }
    let n: u64 = class_totals.iter().sum();
    let base = class_totals.iter().max().copied().unwrap_or(0) as f64 / n.max(1) as f64;
    let (b1, b2, best) = best_thresholds(points);

    println!();
    println!(
        "--- (c) proxy validation: progress_ppm vs the POST-HOC tercile, {players}P, \
         {n} turn starts ---"
    );
    println!(
        "  distinct progress_ppm values: {} {:?}",
        distinct.len(),
        distinct.iter().take(24).collect::<Vec<_>>(),
    );
    println!("  fixed cuts ({t1} / {t2}):  accuracy {accuracy:.4}   base rate (largest truth class) {base:.4}");
    println!("    confusion, rows = post-hoc truth, cols = proxy prediction:");
    println!("      {:<8}{:>10}{:>10}{:>10}{:>10}", "", "pEARLY", "pMID", "pLATE", "total");
    for stage in STAGES {
        let r = matrix[stage.index()];
        println!(
            "      {:<8}{:>10}{:>10}{:>10}{:>10}",
            stage.name(),
            r[0],
            r[1],
            r[2],
            class_totals[stage.index()],
        );
    }
    println!(
        "  best IN-SAMPLE cuts ({b1} / {b2}): accuracy {best:.4}  [OPTIMISTIC — fitted on this \
         corpus; the honest reading is a ceiling for a hard 3-way switch]"
    );
    let saturated = points.iter().filter(|(p, _)| *p >= PROGRESS_FULL_PPM).count();
    println!(
        "  saturated at full ({PROGRESS_FULL_PPM} ppm): {saturated}/{n} = {:.4} of turn starts",
        saturated as f64 / n.max(1) as f64,
    );
}

// ---------------------------------------------------------------------------
// Block (d): behavioural counters
// ---------------------------------------------------------------------------

fn share(num: u64, den: u64) -> String {
    if den == 0 {
        return "--".to_string();
    }
    format!("{:.6}", num as f64 / den as f64)
}

fn report_counters(corpus: &Corpus, contestant: ProbeContestant) {
    println!();
    println!("--- (d) behavioural counters over the corpus ---");
    println!("  applied moves               {}", corpus.applied);
    println!(
        "  nonzero-recovery picks      {:>10}  share {}   <- spec §2.5's falsifier quantity",
        corpus.nonzero_recovery,
        share(corpus.nonzero_recovery, corpus.applied),
    );
    println!(
        "  retreat picks (any)         {:>10}  share {}",
        corpus.retreat_picks,
        share(corpus.retreat_picks, corpus.applied),
    );
    println!(
        "  area actions                {:>10}  share {}",
        corpus.area_actions,
        share(corpus.area_actions, corpus.applied),
    );
    println!(
        "  builds                      {:>10}  share {}",
        corpus.builds,
        share(corpus.builds, corpus.applied),
    );
    match contestant {
        ProbeContestant::Greedy => {
            println!(
                "  placement cap binds         {:>10}  of {} decisions  (spec §3.3)",
                corpus.cap_binds, corpus.decisions,
            );
            println!(
                "  candidates/decision         {:>10.2}  ({} scored / {} decisions)",
                corpus.candidates as f64 / corpus.decisions.max(1) as f64,
                corpus.candidates,
                corpus.decisions,
            );
        }
        ProbeContestant::Policy(_) => {
            println!("  placement cap binds         (n/a: a policy rung has no candidate cap)");
        }
    }
    if let Some(q) = corpus.turns.quantiles() {
        println!(
            "  game length, turns          median {} · p90 {} · p99 {} · max {} (n={})",
            q.median, q.p90, q.p99, q.max, q.n,
        );
    }
    if let Some(q) = corpus.moves.quantiles() {
        println!(
            "  game length, moves          median {} · p90 {} · p99 {} · max {} (n={})",
            q.median, q.p90, q.p99, q.max, q.n,
        );
    }
    let games = corpus.trigger.games();
    println!("  end-trigger mix over {games} games (95% Wilson):");
    if corpus.trigger.counts().is_empty() {
        println!("    (no trigger recorded on any game)");
    }
    for (name, count) in corpus.trigger.counts() {
        let (seen, interval) = corpus.trigger.rate(name);
        debug_assert_eq!(seen, *count);
        println!(
            "    {name:<22} {seen:>5}/{games:<5} = {:.4}  [{:.4}, {:.4}]",
            seen as f64 / games as f64,
            interval.lo,
            interval.hi,
        );
    }
}

// ---------------------------------------------------------------------------
// `table`
// ---------------------------------------------------------------------------

fn cmd_table(args: &[String]) -> Result<(), String> {
    let (positional, flags) = split_args(args, &["weights"], &[])?;
    if !positional.is_empty() {
        return Err(format!("table takes no positional arguments\n\n{USAGE}"));
    }
    let (weights, label) = weights_from(&flags)?;
    println!("=== eval_probe table [{label}] ===");
    println!(
        "  identity={:#018x}  eval_impl_version={}",
        weight_table_identity(&weights) as u64,
        yokohama_core::game::eval::EVAL_IMPL_VERSION,
    );
    println!("  {:<28} {:>8} {:>8}", "feature", "early", "late");
    for feature in Feature::ALL {
        let (early, late) = weights.get(feature);
        println!("  {:<28} {early:>8} {late:>8}", feature.name());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests: the statistics, against hand-computable truth.
//
// Run with `cargo test -p yokohama-core --example eval_probe`. They are here
// rather than in an integration binary because the estimator is here: an AUC
// implementation nobody checks is the shortest path to a freeze decision made
// on a mis-ranked column.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn column(points: &[(f64, bool)]) -> Vec<(f64, u32, bool)> {
        let mut rows: Vec<(f64, u32, bool)> =
            points.iter().enumerate().map(|(i, (v, l))| (*v, i as u32, *l)).collect();
        rows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        rows
    }

    fn auc(points: &[(f64, bool)]) -> Option<f64> {
        let col = column(points);
        auc_weighted(&col, &vec![1u32; points.len()])
    }

    #[test]
    fn auc_is_one_on_perfect_separation_and_zero_when_reversed() {
        assert_eq!(auc(&[(3.0, true), (1.0, false), (2.0, false)]), Some(1.0));
        assert_eq!(auc(&[(0.0, true), (1.0, false), (2.0, false)]), Some(0.0));
    }

    #[test]
    fn auc_is_a_half_when_the_classes_interleave_or_all_tie() {
        // neg 1 < pos 2 < pos 3 < neg 4: two of the four cross pairs concord.
        assert_eq!(auc(&[(2.0, true), (3.0, true), (1.0, false), (4.0, false)]), Some(0.5));
        assert_eq!(auc(&[(7.0, true), (7.0, true), (7.0, false), (7.0, false)]), Some(0.5));
        // And the OTHER interleaving is not 0.5 — a check that the case above
        // is a real property of the data and not an artifact of the estimator
        // rounding everything to a half.
        assert_eq!(auc(&[(1.0, true), (3.0, true), (0.0, false), (2.0, false)]), Some(0.75));
    }

    #[test]
    fn a_single_tie_between_the_classes_counts_half() {
        // pos {1,2} vs neg {2,3}: only the (2,2) pair is a tie, so 0.5/4.
        assert_eq!(auc(&[(1.0, true), (2.0, true), (2.0, false), (3.0, false)]), Some(0.125));
        // The middle positive beats one and loses to one: 1/2.
        assert_eq!(auc(&[(2.0, true), (1.0, false), (3.0, false)]), Some(0.5));
    }

    #[test]
    fn an_auc_over_one_class_is_undefined_rather_than_a_half() {
        assert_eq!(auc(&[(1.0, true), (2.0, true)]), None);
        assert_eq!(auc(&[(1.0, false)]), None);
        assert_eq!(auc(&[]), None);
    }

    /// The whole reason the estimator is weighted: a bootstrap resample that
    /// draws game 0 twice must read exactly as a column with game 0's rows
    /// duplicated. Checked against a case where duplication genuinely moves the
    /// answer, so an implementation that ignored `mult` could not pass.
    #[test]
    fn multiplicity_equals_duplication() {
        // game 0: (1.0, win). game 1: (2.0, loss). game 2: (3.0, win).
        let col = vec![(1.0, 0u32, true), (2.0, 1u32, false), (3.0, 2u32, true)];
        assert_eq!(auc_weighted(&col, &[1, 1, 1]), Some(0.5));
        // Doubling the LOSING game changes nothing (both positives keep the
        // same relation to it), doubling the low winner does not either — but
        // dropping the high winner does, and that is the discriminating case.
        assert_eq!(auc_weighted(&col, &[1, 1, 0]), Some(0.0));
        assert_eq!(auc_weighted(&col, &[0, 1, 1]), Some(1.0));
        // And a resample that loses a whole class is undefined, not 0.5.
        assert_eq!(auc_weighted(&col, &[0, 0, 1]), None);
        // Explicit duplication cross-check.
        let doubled = vec![
            (1.0, 0u32, true),
            (1.0, 3u32, true),
            (2.0, 1u32, false),
            (3.0, 2u32, true),
        ];
        assert_eq!(
            auc_weighted(&col, &[2, 1, 1]),
            auc_weighted(&doubled, &[1, 1, 1, 1]),
        );
    }

    #[test]
    fn stage_terciles_split_a_games_turns_into_three() {
        assert_eq!(stage_of(0, 9), Stage::Early);
        assert_eq!(stage_of(2, 9), Stage::Early);
        assert_eq!(stage_of(3, 9), Stage::Mid);
        assert_eq!(stage_of(5, 9), Stage::Mid);
        assert_eq!(stage_of(6, 9), Stage::Late);
        assert_eq!(stage_of(8, 9), Stage::Late);
        // Degenerate lengths stay total: the first turn is always EARLY.
        assert_eq!(stage_of(0, 1), Stage::Early);
        assert_eq!(stage_of(0, 2), Stage::Early);
        assert_eq!(stage_of(1, 2), Stage::Mid);
        // Every turn of every length lands in exactly one bucket.
        for turns in 1..40usize {
            let mut counts = [0usize; 3];
            for i in 0..turns {
                counts[stage_of(i, turns).index()] += 1;
            }
            assert_eq!(counts.iter().sum::<usize>(), turns, "turns={turns}");
        }
    }

    #[test]
    fn the_nearest_rank_rule_matches_the_crates_own() {
        // B = 2,000 ⇒ the 50th and 1,950th order statistics (1-based).
        assert_eq!(nearest_rank_index(2_000, 25, 1000), 49);
        assert_eq!(nearest_rank_index(2_000, 975, 1000), 1_949);
        // B = 10,000 ⇒ the 250th and 9,750th, which is what
        // `stats::bootstrap_percentile`'s doc states.
        assert_eq!(nearest_rank_index(10_000, 25, 1000), 249);
        assert_eq!(nearest_rank_index(10_000, 975, 1000), 9_749);
    }

    /// A column whose every cell ties is reported as CONSTANT, not as a
    /// zero-width interval around 0.5 — the house's refusal rule, applied to
    /// this estimator. It fires on real data: no seat owns a trading house in
    /// the first tercile.
    #[test]
    fn an_all_ties_column_is_constant_and_never_a_zero_width_interval() {
        assert!(is_constant(&[(0.0, 0, true), (0.0, 1, false), (0.0, 2, true)]));
        assert!(!is_constant(&[(0.0, 0, true), (1.0, 1, false)]));
        assert!(!is_constant(&[]), "an empty column has no value to be constant at");

        let constant = AucRow {
            point: Some(0.5),
            lo: 0.5,
            hi: 0.5,
            cells: 3,
            winners: 2,
            constant: true,
            mean_abs: 0.0,
        };
        assert_eq!(disposition(&constant, Some(15)), "no-signal(const)");
        assert!(!constant.render().contains("0.5000"), "a zero-width interval must not print");
    }

    /// The materiality rule of spec §2.4, in the three states it can be in.
    #[test]
    fn the_materiality_rule_only_fires_when_the_interval_excludes_the_sign() {
        let row = |lo: f64, hi: f64| AucRow {
            point: Some((lo + hi) / 2.0),
            lo,
            hi,
            cells: 10,
            winners: 5,
            constant: false,
            mean_abs: 1.0,
        };
        // A positive weight against an interval wholly below 0.5.
        assert_eq!(disposition(&row(0.40, 0.48), Some(7)), "CONTRADICTED");
        // The same interval with a negative weight is a CONFIRMATION.
        assert_eq!(disposition(&row(0.40, 0.48), Some(-7)), "confirmed");
        // Straddling 0.5 contradicts nothing, whichever way the weight points.
        assert_eq!(disposition(&row(0.48, 0.53), Some(7)), "no-signal");
        assert_eq!(disposition(&row(0.48, 0.53), Some(-7)), "no-signal");
        // And a wholly-above interval flips the pair round.
        assert_eq!(disposition(&row(0.52, 0.60), Some(7)), "confirmed");
        assert_eq!(disposition(&row(0.52, 0.60), Some(-7)), "CONTRADICTED");
    }

    #[test]
    fn linear_shape_adequacy_reports_a_mid_outside_its_endpoints() {
        let row = |p: f64| AucRow {
            point: Some(p),
            lo: p,
            hi: p,
            cells: 1,
            winners: 1,
            constant: false,
            mean_abs: 1.0,
        };
        assert_eq!(shape(&row(0.40), &row(0.50), &row(0.60)), "monotone");
        assert_eq!(shape(&row(0.60), &row(0.50), &row(0.40)), "monotone");
        assert_eq!(shape(&row(0.40), &row(0.70), &row(0.60)), "NON-MONOTONE(+0.1000)");
        assert_eq!(shape(&row(0.40), &row(0.30), &row(0.60)), "NON-MONOTONE(+0.1000)");
    }

    #[test]
    fn the_proxy_confusion_counts_truth_by_row_and_prediction_by_column() {
        let points = vec![
            (0u32, Stage::Early),
            (500_000, Stage::Mid),
            (1_000_000, Stage::Late),
            (1_000_000, Stage::Early), // a saturated proxy on an early turn
        ];
        let (matrix, accuracy) = confusion(&points, 333_333, 666_666);
        assert_eq!(matrix[Stage::Early.index()][Stage::Early.index()], 1);
        assert_eq!(matrix[Stage::Early.index()][Stage::Late.index()], 1);
        assert_eq!(matrix[Stage::Mid.index()][Stage::Mid.index()], 1);
        assert_eq!(matrix[Stage::Late.index()][Stage::Late.index()], 1);
        assert!((accuracy - 0.75).abs() < 1e-12);
        // Hindsight cuts can only do at least as well as any fixed pair.
        let (_, _, best) = best_thresholds(&points);
        assert!(best >= accuracy, "in-sample best {best} below fixed {accuracy}");
    }
}
