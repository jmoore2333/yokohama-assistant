//! `eval_probe` — the corpus instrument evaluator v1's weights are frozen
//! against (Phase 3.3 T4; spec §2.4, §2.5, §10.4).
//!
//! It does four things, in one pass over one generated corpus so that every
//! figure below is a statement about the SAME games:
//!
//!   * **(a) generate** a self-play corpus under a NAMED contestant — Greedy-v1
//!     or Lookahead-v1 through the honest arena driver, or a [`RolloutPolicy`]
//!     rung through the raw-state driver (legitimate here: the bot is not the
//!     subject, which is research-notes R1's own harness disposition) — at a
//!     given player count, game count and master, snapshotting **every seat's
//!     view at every turn start**;
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
//!     end-trigger mix, plus (on a searching contestant) the spec §0.3(b)
//!     per-seat explained-equals-applied identity and the two search guards
//!     `ply_cap_hits` / `free_cap_hits`, which are RECORDED and never asserted:
//!     this is an instrument, and the gate that asserts them is
//!     `tests/evaluator_gate.rs`.
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
//! # The V4 candidate columns (§7 step 3)
//!
//! Beside the corpus work above, this instrument carries the eighteen §3.2
//! CANDIDATE COLUMNS in probe space — `candidate_columns` is the adapter
//! `pair_fixtures::CandidateColumns` was declared to receive, and `pairs` dumps
//! the frozen fixture package with all thirty-four delta rows real. Nothing
//! here fits or generates anything; `Feature::ALL` is unmoved at sixteen until
//! step 8. The step-2 close handover's cross-checks over that package — the
//! declared candidate levels, O1's registered pool and floors, §5.1's
//! expected-nonzero rule on the candidate axis, family 5's audit properties and
//! family 11's congruence — are the cells at the bottom of this file, and they
//! run under this example's own test command and no other.
//!
//! ```text
//! cargo run --release -p yokohama-core --example eval_probe -- \
//!     run 2 2000 --contestant greedy-v1 --master 0xE7A1202607270001
//! cargo run --release -p yokohama-core --example eval_probe -- \
//!     run 2 200 --contestant family-weighted --blocks proxy
//! cargo run --release -p yokohama-core --example eval_probe -- \
//!     run 2 400 --contestant lookahead-v1 --master 0xE7A1202608090001
//! cargo run --release -p yokohama-core --example eval_probe -- table
//! cargo run --release -p yokohama-core --example eval_probe -- pairs --pair "4(c)"
//! ```
//!
//! Its own statistics are unit-tested: `cargo test -p yokohama-core --example
//! eval_probe`.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use yokohama_core::game::eval::{
    achievement_progress, achievement_progress_postclaim_sum, interpolate, progress_ppm, score,
    view_is_terminal, Feature, ScoreStructure, WeightTable, PROGRESS_FULL_PPM,
};
// `DEFAULT_RETREAT_SAMPLES` is deliberately NOT imported here: since the parse
// delegates to `Contestant::from_str_name`, every default this file used to type
// comes off that grammar's own consts instead. The tests import it to pin them.
use yokohama_core::game::greedy::{
    weight_table_identity, ExplanationTally, GreedyAgent, GREEDY_CONTROLLER,
};
use yokohama_core::game::lookahead::{
    LookaheadAgent, SearchScheme, SearchTally, LOOKAHEAD_CONTROLLER,
};
use yokohama_core::game::driven::{
    escrow_recipe_pair, placement_shape, registry, root_table, RecipeMiss, RecipePair,
    RootPredicate, RootSource,
};
use yokohama_core::game::driven_runner::DEFAULT_SEAT;
use yokohama_core::game::pair_fixtures::{
    build_registered_pair, package_digest, registered, render_pair_report_with,
    render_registry_manifest, CandidateFeature, FeatureId, PairRole, Side, StageEndpoint,
    AXIS_WIDTH, REGISTERED,
};
use yokohama_core::{
    agent_stream_seed, apply_move, finalize_setup, get_legal_moves, new_game, observe, replay_from,
    run_arena_game, run_raw_game, state_digest, AchievementType, AreaId, AreaType, ArenaAgent,
    ArenaGameStats, Contestant, GameRecord, GameState, HandCard, Move, PlayerId, PlayerView,
    QuantileSketch, RolloutPolicy, Rules, SeedSchedule, SeedScheduleKind, Slot, TriggerTally,
    VisiblePlayer, WorldSeed, SEMANTICS_REV,
};

const USAGE: &str = "\
usage: eval_probe <subcommand> [options]

  run <players> <games> [--contestant <name>] [--weights v1|v2|pre-freeze]
                        [--master <u64> | --os-entropy]
                        [--invariants | --no-invariants]
                        [--bootstrap <B>] [--blocks <list>]
                        [--score-table <spec>]... [--candidate-columns <list>]
      Generate a self-play corpus and report the blocks in <list> (default all):
        features  per-feature AUC per stage tercile, game-clustered bootstrap
        proxy     progress_ppm against the post-hoc terciles
        counters  recovery share, cap binds, game length, end-trigger mix

      --score-table <spec>   REPEATABLE. Evaluate a CANDIDATE weight table on
                             this corpus and report its EVAL TOTAL AUC per stage,
                             split fit (EVEN game index) / held-out (ODD) by
                             generation index alone, with a game-clustered
                             bootstrap on the HELD-OUT figures — the quotable
                             ones. <spec> is a BASE, optionally named and
                             optionally EXTENDED:
                               <base>                        (bare, unnamed)
                               <name>=<base>[+<column>=<early>/<late>]...
                             where <base> is `v1` (the reproduction check for
                             every committed round command), `v2` (the table
                             that ships since the 2026-08-10 freeze), `v2-draft`
                             (the same numbers under the rounds' own spelling),
                             or `<e0,...,e15>/<l0,...,l15>` (sixteen i64 per
                             vector, Feature::ALL order).
                             Each `+` clause is the V3 spec §4.1 17th-column
                             machinery: it prices a --candidate-columns name
                             exactly as a shipped feature is priced —
                             interpolate(early,late) x (mine - the signed-weight
                             opponent extreme) — so an extension table IS the
                             arithmetic preview of admitting that column.
                             Feature::ALL does not move; a clause is not a
                             feature. Clauses come off the RIGHT greedily, so a
                             vector base writes its entries bare or `-`-signed
                             (a leading `+` entry is refused by name). An
                             extension table on a NAMED base also prints the
                             §4.2 PAIRED held-out improvement interval against
                             that base.
                             A candidate is EVALUATED here and never GENERATES
                             the corpus: `--weights` chooses what the bot plays,
                             and its DEFAULT is still v1 so every committed
                             corpus command reproduces.
      --candidate-columns <list>
                             Comma-separated extra ROWS in the features block,
                             measured beside the sixteen and shipped by nothing.
                             Feature::ALL stays sixteen everywhere. REFUSED
                             when --blocks omits `features` (an ignored column
                             would be a silent no-op). The nine names, each
                             also the row's printed name except `postclaim`:
                               postclaim
                                   achievement_progress_sum with the viewer's
                                   OWN already-claimed cards dropped (T3)
                               church_majority_margin
                                   signed margin in the 6/3 pawn ranking, 2P
                                   dummy competing (D1); > 0 means this seat
                                   holds first place as the board stands
                               customs_majority_margin
                                   the same over the 8/4 ranking
                               tech_majority_margin
                                   the 10/5 industry-value ranking, 2P dummy
                                   competing at DOUBLED value 15 (D5)
                               achievement_race_proximity
                                   minus the nearest UNCLAIMED-by-this-seat
                                   card's remaining capped gap (0 is nearest)
                               clock_control_church
                                   this seat's own pawns on the church track
                               port_empty_slots
                                   empty Port card slots across every board —
                                   BOARD-GLOBAL, so its relative form is 0 for
                                   every seat by construction and only the
                                   absolute form carries information
                               money
                                   this seat's yen
                               terminal_margin_now
                                   this seat's would-be-FINAL margin if the
                                   game were scored where it stands: its full
                                   final total minus the best other seat's.
                                   View-honest — every component of a
                                   scored-now total is public board state

  table [--weights v1|v2|pre-freeze]
      Print a weight table with its provenance identity. No games are played.

  pairs [--pair <id>] [--stage early|late] [--manifest]
      Dump the V4 §5 pair-fixture package's reports with ALL THIRTY-FOUR delta
      rows REAL: the sixteen incumbents through `eval`'s public surface as
      before, and the eighteen §3.2 candidates through this instrument's
      CandidateColumns adapter (§7's probe space for every column until
      step 8 moves them into `Feature::ALL`). No games are played and nothing
      is fitted — this is the DUMP the collision instrument and the solver read
      their inputs off, and every report prints the package digest.
        --pair <id>    one registration only, e.g. `4(c)` (default: all)
        --stage early|late
                       one endpoint only (default: every endpoint the
                       registration declares; a pair that does not declare the
                       named one is skipped, and the count says so)
        --manifest     the registry manifest alone, no reports

  collinearity <players> <games> [--contestant <name>] [--weights v1|v2|pre-freeze]
                                [--master <u64> | --os-entropy]
                                [--invariants | --no-invariants]
      §5.5's COLLINEARITY DIAGNOSTIC, before any fit and fitting nothing.
      Generates a 2P corpus and reports, over the FULL 34-column matrix in the
      RELATIVE currency: per-column ranges and scales (in BOTH currencies),
      candidate x candidate and candidate x incumbent correlations, §2's
      declared clusters realized, the E1/Money10 read POOLED and CONDITIONED on
      the drawn Category-B card, each candidate's VIF against the retained set
      per stage tercile and pooled, and §8's collinear flags. 2P only: at 3-4P
      `opponent_extreme` orients by weight, which would make a weight-free
      diagnostic a statement about the incumbent weights instead.

  collision [--fixtures-only] [--no-generated] [--no-tie]
      The §5.3 PART A collision test, weight-free. Signs every registered gate
      pair's SIDES and every non-terminal successor of the §6 root set with
      `(exact progress_ppm, Δv over the 34 features)` from the MOVER's view,
      groups by identical signature and applies §5.3's blocking rule. Zero
      blocking collisions is NECESSARY; a blocking collision exits non-zero and
      names the pair, because the step-4 fork is owner territory. Also prints
      the near-collisions (equal Δv, different ppm), the organic classes with
      their sizes, and the e113 top tie's complete-vector measurement.
        --fixtures-only  the 106 registered sides alone — no roots, no tie
        --no-generated   registry roots only (skips the seeded scans)
        --no-tie         skip the e113 root-table measurement

  rooted [--record <path>] [--no-generated]
      The §7 STEP-5 ROOTED-READ GATES, weight-free: raw direction and
      non-collision at the DEVELOPMENT roots and nothing beyond the frozen gate
      texts. Four gates and two read-and-record items —
        GATE 1  g1-e113-tiemass: the summed delta over ALL SEVEN frozen S
                definitions strictly favours the registered enabling-line
                placement over its registered same-count dead side, and the
                pair's two sides do not collide (§5.3's own classifier)
        GATE 2  each placement-fork development ordinal at M-PF: the S signature
                takes >= 2 distinct values across the fork's same-count set
        GATE 3  CONSTRUCTED strict-O3 coverage — 4(b) EXECUTABLE >= 1, with
                4(b) DISTANT and 4(a) both sides at 0 (corpus-free)
        GATE 4  the RECORDED opponent-pending development band: at each of the
                five charter development exhibits, opponent-O1 >= 1 AND
                opponent-O3 = 0, with opponent-O2 recorded
        record  the X wiring proof (satisfies NOTHING) and the fired-while-behind
                root's complete candidate columns (never a gate)
      Every root's identity is digest-verified BEFORE any column is read, and
      the EIGHT escrow-reserved exhibits refuse a column read by construction
      (the 2026-08-23 charter row extends §6's burn rule to them verbatim).
      A gate failure exits non-zero: step 5 stops before any fitting.
        --record <path>  the regenerated `bga-import/896199544` record GATE 4's
                         five development exhibits are events of. Absent, that
                         gate prints a typed SKIP and is not run
        --no-generated   skip the seeded scans (GATE 2 and the fired-while-behind
                         read)

contestants (the grammar is Contestant::from_str_name's — unknown is an error at
every level, and an unknown knob is never silently ignored):
  greedy-v1[:m=<n>]    the honest arena driver (observe -> sampler -> handle)
  lookahead-v1[:b=<n>,k=<n>,m=<n>,cap=<n>]
                       the S1 turn-boundary search rung, through the SAME honest
                       driver (never the raw-state one). b = beam, k = worlds,
                       m = retreat samples, cap = the shared scored-successor
                       budget; OMITTING cap is the shipped UNCAPPED rung, and
                       cap=0 is refused rather than read as uncapped. Caps
                       32-128 are a MEASURED non-terminating band for 2P
                       self-play (LookaheadAgent::with_cap's doc carries the
                       data) — a corpus run there grinds to the turn budget
                       and errors rather than finishing
  uniform | retreat-averse | family-weighted[:p=<ppm>,k=<n>]
                       a RolloutPolicy rung through the RAW-STATE driver, which
                       is privileged play by construction and labelled as such
                       (legitimate for a corpus: the bot is not the subject)
  mcts-v1 and atomic-diag-v1 PARSE but are refused HERE: this instrument reports
  the greedy/search counter sets and has none for them. Run those through
  `cargo run --release -p yokohama-core --example arena`.
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
        Some("pairs") => cmd_pairs(&args[1..]),
        Some("collision") => cmd_collision(&args[1..]),
        Some("collinearity") => cmd_collinearity(&args[1..]),
        Some("rooted") => cmd_rooted(&args[1..]),
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

/// [`split_args`]' three products: the positionals, the single-valued flags,
/// and the repeatable ones (each with every value it was given, in argv order).
type SplitArgs = (Vec<String>, BTreeMap<String, String>, BTreeMap<String, Vec<String>>);

/// Split argv into positionals, `--flag [value]` pairs, and REPEATABLE flags.
///
/// An unknown flag is an ERROR, never a silent no-op — `examples/arena`'s rule,
/// and for its reason: a mistyped `--os-entropi` that quietly left the run on a
/// diagnostic master would put a fixed master behind a figure published as
/// freshly drawn.
///
/// `multi` names the flags that may appear more than once; their values collect
/// into a `Vec` IN ARGV ORDER, which is the order the candidate blocks print in.
/// They are kept out of the single-valued map deliberately: a repeat landing
/// there would silently keep the last value, and losing a `--score-table` is
/// exactly the failure mode the unknown-flag rule above exists to prevent one
/// level up.
fn split_args(
    args: &[String],
    valued: &[&str],
    bare: &[&str],
    multi: &[&str],
) -> Result<SplitArgs, String> {
    let mut positional = Vec::new();
    let mut flags: BTreeMap<String, String> = BTreeMap::new();
    let mut repeated: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some(name) = arg.strip_prefix("--") {
            if multi.contains(&name) {
                let value =
                    args.get(i + 1).ok_or_else(|| format!("--{name} needs a value\n\n{USAGE}"))?;
                repeated.entry(name.to_string()).or_default().push(value.clone());
                i += 2;
                continue;
            }
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
    Ok((positional, flags, repeated))
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
        // **THE DEFAULT STAYS `v1`, and that is a decision rather than an
        // oversight.** Since the 2026-08-10 v2 freeze the SHIPPED table is `V2`,
        // so a default-weights corpus is no longer an incumbent-play corpus of
        // what ships — but every committed probe command (the R0 corpora, both
        // rounds' L2 passes, the greedy counters comparator) omits this flag,
        // and moving the default would additionally change the TABLE out from
        // under them. **What the default buys is stated exactly (freeze review
        // MUST-1): the TABLE's thirty-two numbers, so a run is weight-comparable
        // with the record — NOT the corpus itself.** T3's post-claim filter now
        // rides `ScoreStructure::derive` unconditionally, with no switch, so the
        // same command at the same master plays a DIFFERENT corpus than the one
        // the recorded figures were measured on (measured: R0-G's 1,099,281
        // applied moves read 1,101,363 at the freeze HEAD). A re-run here is a
        // different measurement, not a reproduction — `eval_screen_round`'s
        // post-freeze doc says the same of criteria 1–2 — and reproducing a
        // recorded figure means checking out its recorded commit. The `table`
        // subcommand's default print pairs V1's digits with `eval_impl_version
        // 3` for the same reason; the `[v1]` header is the self-label. `v2` is
        // spelled out below and must be asked for.
        None | Some("v1") => Ok((WeightTable::V1, "v1")),
        Some("v2") => Ok((WeightTable::V2, "v2")),
        Some("pre-freeze") => Ok((WeightTable::PRE_FREEZE_2026_07_27, "pre-freeze")),
        Some(other) => Err(format!("unknown --weights {other:?} (v1 | v2 | pre-freeze)")),
    }
}

/// Who plays the corpus.
///
/// Named `Probe…` rather than `Contestant` because [`Contestant`] is a public
/// type with a DIFFERENT shape (five variants), and two incompatible answers to
/// `grep 'enum Contestant'` in one crate is a trap, not an abbreviation
/// (six-lens review). Since Phase 3.5c W1 the NAME GRAMMAR is nonetheless
/// single-sourced — [`ProbeContestant::parse`] runs [`Contestant::from_str_name`]
/// and then narrows — so a spec typed here means what it means at
/// `examples/arena`, knobs and refusals included.
///
/// **Why the probe still constructs its own agents rather than calling
/// [`Contestant::build`]:** `build` hard-codes the SHIPPED table
/// ([`WeightTable::V2`] since the 2026-08-10 freeze, `V1` before it — a
/// contestant carries no weight knob by L12 design), and this instrument's
/// entire `--weights` flag is the ability to run a corpus under a different
/// one. Delegating construction would make that flag a
/// silent no-op — the exact failure mode `split_args` refuses unknown flags to
/// avoid, one level up.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ProbeContestant {
    /// Greedy-v1 at every seat, through the HONEST driver.
    Greedy { m: u8 },
    /// Lookahead-v1 — the S1 turn-boundary rung — at every seat, through the
    /// SAME honest driver. `cap` absent is the shipped UNCAPPED rung, which is
    /// what every recorded 3.3 figure was measured on.
    Lookahead { b: u16, k: u8, m: u8, cap: Option<u32> },
    /// One policy rung at every seat, through the RAW-STATE driver.
    Policy(RolloutPolicy),
}

impl ProbeContestant {
    /// Parse a contestant spec through the CORE grammar, then narrow it to the
    /// arms this instrument meters.
    ///
    /// The two refusals are not a to-do list. An eval_probe corpus is only worth
    /// the counters block that describes it, and neither refused arm has one
    /// here: MCTS-v1's meters are a different SET (`McTally`, with no
    /// free-extension counter at all and a `(Some, None)` cap-counter shape),
    /// and the S2 atomic arm is a labelled diagnostic no weight-freeze figure
    /// reads. A silently-accepted third arm would print the greedy block over a
    /// contestant that never bound a placement cap — a figure with the wrong
    /// provenance, which is worse than a refusal.
    fn parse(spec: &str) -> Result<Self, String> {
        // The narrowing is an exhaustive match, no `_` arm: a sixth contestant
        // upstream must be dispositioned here rather than silently refused by a
        // catch-all that names nothing.
        match Contestant::from_str_name(spec)? {
            Contestant::GreedyV1 { m } => Ok(ProbeContestant::Greedy { m }),
            Contestant::LookaheadV1 { b, k, m, cap } => {
                Ok(ProbeContestant::Lookahead { b, k, m, cap })
            }
            Contestant::Policy(policy) => Ok(ProbeContestant::Policy(policy)),
            Contestant::AtomicDiag { .. } => Err(Self::refusal(
                spec,
                "the S2 atomic-diag arm is a labelled DIAGNOSTIC (3.3 spec §4.1) and no \
                 weight-freeze figure reads its corpus",
            )),
            Contestant::MctsV1 { .. } => Err(Self::refusal(
                spec,
                "the tree's counters are a different set from the greedy/search ones this \
                 instrument reports (McTally, and no free-extension meter at all), so its \
                 corpus would arrive with a counters block describing some other contestant",
            )),
        }
    }

    fn refusal(spec: &str, why: &str) -> String {
        format!(
            "eval_probe does not drive {spec:?}: {why}. Run it through `examples/arena` instead. \
             (this probe: {GREEDY_CONTROLLER} | {LOOKAHEAD_CONTROLLER} | uniform | \
             retreat-averse | family-weighted)"
        )
    }

    fn echo(self, weights: &WeightTable, label: &str) -> String {
        let identity = weight_table_identity(weights) as u64;
        match self {
            ProbeContestant::Greedy { m } => format!(
                "{GREEDY_CONTROLLER}{{m={m},weights={label}:{identity:#018x}}} \
                 [honest driver: observe -> sampler K=1 -> handle]",
            ),
            ProbeContestant::Lookahead { b, k, m, cap } => format!(
                "{LOOKAHEAD_CONTROLLER}{{b={b},k={k},m={m},{}weights={label}:{identity:#018x}}} \
                 [honest driver: observe -> sampler K=1 -> handle]",
                // An absent cap emits NO token — `Contestant::provenance`'s rule,
                // held here for its reason: an uncapped echo has to say the
                // SHIPPED rung, not a rung wearing a budget it never had.
                cap.map_or(String::new(), |c| format!("cap={c},")),
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

/// One seat's counter read-out from a contestant that EXPLAINS.
///
/// The two underlying tallies are deliberately different types upstream (a beam
/// has continuation lengths and budget meters; a one-ply argmax has neither), so
/// this is the narrow thing the probe needs from both. It mirrors
/// [`yokohama_core::ExplainHandle`]'s `Option` rule verbatim — **`None` always
/// means "this contestant has no such quantity", never "zero"** — and lives here
/// beside it rather than reusing it because `ExplainHandle` does not expose
/// `placement_cap_binds`, which is the §3.3 counter this probe's greedy block has
/// printed since Phase 3.3 and which both tallies carry.
#[derive(Clone)]
enum ProbeTally {
    Greedy(ExplanationTally),
    Search(SearchTally),
}

impl ProbeTally {
    /// Decisions this seat explained. On a completed game this equals the moves
    /// the seat applied — no evaluator agent can return a move without
    /// explaining it — which is the identity [`identity_blocks`] checks.
    fn explained(&self) -> u64 {
        match self {
            Self::Greedy(t) => t.explained(),
            Self::Search(t) => t.explained(),
        }
    }

    fn candidates_scored(&self) -> u64 {
        match self {
            Self::Greedy(t) => t.candidates_scored(),
            Self::Search(t) => t.candidates_scored(),
        }
    }

    /// Decisions whose placement family exceeded the candidate cap. Summed over
    /// DECISIONS for the one-ply arm and over search NODES for S1, which is why
    /// the two arms print the counter with different words.
    fn placement_cap_binds(&self) -> u64 {
        match self {
            Self::Greedy(t) => t.placement_cap_binds(),
            Self::Search(t) => t.placement_cap_binds(),
        }
    }

    /// Continuations `MAX_TURN_PLIES` stopped — `None` for the one-ply argmax,
    /// which has no continuation to truncate, on `ExplainHandle::ply_cap_hits`'s
    /// own grounds: a `0` there would read as "searched and never hit the guard".
    fn ply_cap_hits(&self) -> Option<u64> {
        match self {
            Self::Greedy(_) => None,
            Self::Search(t) => Some(t.ply_cap_hits()),
        }
    }

    /// Free extensions `MAX_FREE_EXTENSION` stopped, same `None` rule. `Some(0)`
    /// on an S1 corpus is STRUCTURAL rather than evidence — the turn-boundary
    /// scheme never enters the quiescence arm at all.
    fn free_cap_hits(&self) -> Option<u64> {
        match self {
            Self::Greedy(_) => None,
            Self::Search(t) => Some(t.free_cap_hits()),
        }
    }

    /// Spec §3.4's cap-bind count — decisions where the scored-successor budget
    /// DENIED work. Same `None` rule; `Some(0)` on an uncapped corpus is the
    /// shipped shape (an absent cap threads an unlimited budget that never
    /// binds), while a capped corpus with binds at essentially every decision is
    /// a MATERIALLY different corpus — which is why the block prints these
    /// beside the search guards rather than leaving the §3.3 placement line to
    /// impersonate them (W1-C review SHOULD-1).
    fn budget_binds(&self) -> Option<u64> {
        match self {
            Self::Greedy(_) => None,
            Self::Search(t) => Some(t.budget_binds()),
        }
    }

    /// Spec §3.4's starvation count — decisions that got ONLY the root pass.
    /// A strict sub-case of [`budget_binds`](Self::budget_binds); same `None`
    /// rule.
    fn budget_starved(&self) -> Option<u64> {
        match self {
            Self::Greedy(_) => None,
            Self::Search(t) => Some(t.budget_starved_decisions()),
        }
    }
}

/// The spec §0.3(b) identity, per SEAT: an explaining contestant explained
/// exactly the moves it applied.
///
/// Returns `(blocks, mismatches)` over `max(len)` indices. A LENGTH disagreement
/// is a mismatch at every unpaired index rather than a truncation, because a zip
/// over the shorter list would report a clean identity over a table it only half
/// checked — and "no tallies came back at all" is exactly what a contestant that
/// stopped handing them out looks like.
fn identity_blocks(explained: &[u64], moves_by_seat: &[u64]) -> (u64, u64) {
    let blocks = explained.len().max(moves_by_seat.len());
    let mismatches =
        (0..blocks).filter(|i| explained.get(*i) != moves_by_seat.get(*i)).count();
    (blocks as u64, mismatches as u64)
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
    /// A row that is NOT in the shipped feature list — see [`CandidateColumn`].
    Candidate(CandidateColumn),
    /// The evaluator's own positional total: the benchmark row every feature is
    /// trying to beat, and already a relative quantity.
    EvalTotal,
}

impl Row {
    fn name(self) -> &'static str {
        match self {
            Row::Feature(f) => f.name(),
            Row::HandUnsplit => "assistants_in_hand (unsplit)",
            Row::Candidate(c) => c.name(),
            Row::EvalTotal => "EVAL TOTAL (benchmark)",
        }
    }

    /// The weight pair the row's measured sign is judged against, or `None` for
    /// the synthetic rows and the candidate columns (which ship no weight, so
    /// there is no shipped direction for a measurement to contradict).
    fn weights(self, table: &WeightTable) -> Option<(i64, i64)> {
        match self {
            Row::Feature(f) => Some(table.get(f)),
            Row::HandUnsplit | Row::Candidate(_) | Row::EvalTotal => None,
        }
    }

    /// The reported rows, in print order: the sixteen shipped features, the
    /// unsplit hand row, then any CANDIDATE COLUMNS, and the benchmark total
    /// last.
    ///
    /// Candidate columns sit before `EvalTotal` so the benchmark keeps ending
    /// the table, which is the shape every recorded artifact's appendix has.
    /// With `columns` empty the vector is byte-for-byte the one this instrument
    /// has always built — the property that keeps an argument-free run
    /// reproducing the R0 appendices.
    fn all(columns: &[CandidateColumn]) -> Vec<Row> {
        let mut rows: Vec<Row> = Feature::ALL.into_iter().map(Row::Feature).collect();
        rows.push(Row::HandUnsplit);
        rows.extend(columns.iter().copied().map(Row::Candidate));
        rows.push(Row::EvalTotal);
        rows
    }
}

/// A row this instrument can measure that the shipped feature list does NOT
/// contain — an admission candidate, reported beside the sixteen so a proposal
/// can be read against the evidence rather than argued from first principles.
///
/// **NOT a [`Feature`], and nothing here moves `Feature::ALL`.** The shipped
/// list stays sixteen entries everywhere; adding a variant here adds a REPORTED
/// COLUMN and nothing else. That separation is the point: the protocol's ladder
/// (§4.2) wants L1 evidence for an admission BEFORE the feature count moves, and
/// the feature count moving is the one step `Feature::ALL`'s own doc calls out
/// as having a silent failure mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CandidateColumn {
    /// The T3 post-claim filter's value: `achievement_progress_sum` with the
    /// viewer's already-claimed cards dropped (protocol §4.1(ii) / §5's T3 row).
    /// Computed by the additive `eval` helper, which nothing in `score` calls.
    PostClaim,
    /// **Round 3, T5.** The seat's signed margin in the CHURCH majority ranking
    /// — see [`majority_margin`] for the arithmetic and its rules citation.
    ChurchMajorityMargin,
    /// **Round 3, T5.** The same over the CUSTOMS majority ranking.
    CustomsMajorityMargin,
    /// **Round 3, T5/T9.** The seat's signed margin in the TECHNOLOGY
    /// industry-value ranking, the 2P dummy competing at doubled value 15 (D5)
    /// — see [`tech_majority_margin`].
    TechMajorityMargin,
    /// **Round 3, T5.** Nearness to the seat's next achievement claim — see
    /// [`achievement_race_proximity`].
    AchievementRaceProximity,
    /// **Round 3, T6.** The seat's OWN pawn count on the church track: how much
    /// of the ending clock this seat is itself driving.
    ///
    /// **NEAR-REDUNDANT WITH [`ChurchMajorityMargin`] in the RELATIVE form at
    /// 2P** (Task-D review SHOULD-2, measured): whenever the dummy is both
    /// seats' best rival, `church_rel = 2·(c_me − c_opp)` while
    /// `clock_rel = c_me − c_opp` — proportional, and AUC is rank-based, so the
    /// two relative columns carry IDENTICAL AUC there (sign agreement 4,446 of
    /// 4,448 organic 2P cells). An L1 report must not read them as two
    /// independent signals, and round 4 must not admit one signal twice.
    ClockControlChurch,
    /// **Round 3, T6.** Empty Port card slots across every Port board — the
    /// conjunct the `OrderDeckEmpty` trigger drops. BOARD-GLOBAL: see
    /// [`port_empty_slots`] for what that costs the relative form.
    PortEmptySlots,
    /// **Round 3, T7/T8.** The seat's yen, re-tested at rev-7 prices.
    Money,
    /// **V3, C1 — the F9 debt item, never before carried to L1.** The seat's
    /// would-be-FINAL margin if the game were scored where it stands:
    /// `ScoreStructure::final_total(me)` minus the best other seated non-dummy
    /// seat's, raw signed and unclamped, with no rank component — see
    /// [`terminal_margin_now`].
    ///
    /// **Provenance.** D7 triage item F9 and the round-3 runs report (§4.2)
    /// record the same absence twice: canon 15's inverted end-trigger brake
    /// needs the TRUE margin, and no deficit-keyed term exists — scenario
    /// registry 18 pins exactly that gap, and the Phase-C f2-control read made
    /// it mechanical (the subject fired the end-trigger 20/20 at the one
    /// admitted root where not-firing dominates under every splitting policy).
    /// F9's gap was a missing TERM, not an uncomputable one, which is what this
    /// column is here to measure.
    ///
    /// **View-honesty, stated precisely (V3 spec §2).** Every component of a
    /// scored-NOW total is public board state — running VP, the country set,
    /// the technology / church / customs majorities and the leftover
    /// conversions all read face-up fields of the view. This is
    /// `TerminalStanding`'s own observation from the other side: an opponent's
    /// masked starting order is value the viewer cannot see COMING, so it
    /// changes future `*_now` TRAJECTORIES and never the scored-now
    /// arithmetic. The column is therefore exact truth delivered view-honestly.
    ///
    /// **The selection-side fence.** `driven::true_margin_now` — full-state
    /// truth through `final_result`'s official standings — stays HARNESS-side
    /// (design §1 family 2: "nothing in the evaluator may read it"). This
    /// column is written fresh on the view side, the round-3 probe-mirror
    /// pattern, and is pinned by BEHAVIOUR against that reference: the tests
    /// may read the selection helper, the evaluator never does.
    TerminalMarginNow,
}

/// Every accepted `--candidate-columns` token, paired with the column it names.
///
/// [`CandidateColumn::parse_list`] and its refusal's "what IS accepted" list
/// both read THIS array, so a token can never drift from what the refusal
/// advertises. [`CandidateColumn::token`]'s exhaustive match is the
/// compile-time half of the guard (a new variant must be named); this array is
/// the half that makes it reachable from the command line, and
/// `every_candidate_column_is_reachable_from_the_command_line` is what pins the
/// two together.
const CANDIDATE_COLUMNS: [CandidateColumn; 9] = [
    CandidateColumn::PostClaim,
    CandidateColumn::ChurchMajorityMargin,
    CandidateColumn::CustomsMajorityMargin,
    CandidateColumn::TechMajorityMargin,
    CandidateColumn::AchievementRaceProximity,
    CandidateColumn::ClockControlChurch,
    CandidateColumn::PortEmptySlots,
    CandidateColumn::Money,
    CandidateColumn::TerminalMarginNow,
];

impl CandidateColumn {
    /// The command-line token. Equal to [`name`](Self::name) for every round-3
    /// column — a name a reader sees in the report is a name they can type —
    /// and deliberately NOT for `postclaim`, whose short token is already in
    /// recorded round-1 commands and stays byte-compatible with them.
    fn token(self) -> &'static str {
        match self {
            CandidateColumn::PostClaim => "postclaim",
            _ => self.name(),
        }
    }

    /// The reported column name. For `postclaim` this is deliberately the
    /// shipped feature's name plus a suffix, so a reader diffing the two rows
    /// sees they measure the same quantity under different rules; the round-3
    /// columns name the quantity they compute, in snake_case, and every one of
    /// them fits the table's 28-column name field.
    fn name(self) -> &'static str {
        match self {
            CandidateColumn::PostClaim => "achievement_progress_sum_postclaim",
            CandidateColumn::ChurchMajorityMargin => "church_majority_margin",
            CandidateColumn::CustomsMajorityMargin => "customs_majority_margin",
            CandidateColumn::TechMajorityMargin => "tech_majority_margin",
            CandidateColumn::AchievementRaceProximity => "achievement_race_proximity",
            CandidateColumn::ClockControlChurch => "clock_control_church",
            CandidateColumn::PortEmptySlots => "port_empty_slots",
            CandidateColumn::Money => "money",
            CandidateColumn::TerminalMarginNow => "terminal_margin_now",
        }
    }

    /// One seat's raw value for this column.
    ///
    /// `rules` is here for the technology margin and the terminal margin —
    /// industry values live in the tech registry, exactly as
    /// `eval::technology_majority` and `ScoreStructure::derive` read them.
    fn value(self, view: &PlayerView, player: &yokohama_core::VisiblePlayer, rules: &Rules) -> i64 {
        match self {
            CandidateColumn::PostClaim => achievement_progress_postclaim_sum(view, player),
            CandidateColumn::ChurchMajorityMargin => majority_margin(
                view.church_track.slots.iter().map(|slot| slot.map(|o| o.player)),
                player.id,
            ),
            CandidateColumn::CustomsMajorityMargin => majority_margin(
                view.customs_track.slots.iter().map(|slot| slot.map(|o| o.player)),
                player.id,
            ),
            CandidateColumn::TechMajorityMargin => tech_majority_margin(view, player, rules),
            CandidateColumn::AchievementRaceProximity => achievement_race_proximity(view, player),
            CandidateColumn::ClockControlChurch => view
                .church_track
                .slots
                .iter()
                .flatten()
                .filter(|occupant| occupant.player == player.id)
                .count() as i64,
            CandidateColumn::PortEmptySlots => port_empty_slots(view),
            CandidateColumn::Money => i64::from(player.money),
            CandidateColumn::TerminalMarginNow => terminal_margin_now(view, player, rules),
        }
    }

    /// Parse a comma-separated `--candidate-columns` list. An unknown name is an
    /// ERROR naming what was seen — `split_args`'s rule one level down, for its
    /// reason: a silently-dropped column would print a report missing the very
    /// row the run was made to look at.
    fn parse_list(raw: &str) -> Result<Vec<Self>, String> {
        raw.split(',')
            .filter(|s| !s.is_empty())
            .map(|name| {
                CANDIDATE_COLUMNS.iter().copied().find(|c| c.token() == name).ok_or_else(|| {
                    format!(
                        "unknown --candidate-columns entry {name:?} ({})",
                        CANDIDATE_COLUMNS
                            .iter()
                            .map(|c| c.token())
                            .collect::<Vec<_>>()
                            .join(" | "),
                    )
                })
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// The round-3 admission columns' arithmetic (probe-side mirrors)
// ---------------------------------------------------------------------------
//
// Everything below is a MIRROR of scoring arithmetic that lives in `eval.rs`
// and `game/scoring.rs`, restated here at finer grain: the shipped features
// report the VP a majority would pay, and these report the seat's standing IN
// that majority. They live in this example rather than in `eval.rs` because
// round 3 is an L1 EVIDENCE round (execution-ledger row, protocol §4.2): an
// admission moves `Feature::ALL`, which is freeze-commit blast radius, so the
// evidence is gathered before any admission is proposed. If a column is later
// admitted, its computation moves into `eval.rs` with its own tests at that
// point.
//
// A mirror with no pin is a second implementation waiting to disagree, which is
// why each margin below is pinned by BEHAVIOUR against the shipped structure —
// positive exactly when `ScoreStructure` awards this seat first place — rather
// than by inspection.
//
// **Canon conformance** (guidingpath's 2026-08-09 canon-batch row, which
// reversed two drafted scenarios): church denial and achievement denial are NOT
// canon, customs is a RACE, and the end-trigger brake INVERTS. So every shape
// here is a MARGIN, a PROXIMITY or a CONTROL SHARE read from the seat's own
// side — "how am I doing in this race, and how much of the clock am I driving"
// — and none of them is a denial-seeking construct.

/// Contender slots in a management-board majority: the four printed seats plus
/// the 2-player dummy. `eval.rs` holds the same two constants privately
/// (`MAX_CONTENDERS` / `DUMMY_SLOT`), so they are restated rather than imported
/// and the agreement is pinned behaviourally.
const PROBE_MAX_SEATS: usize = 4;
const PROBE_DUMMY_SLOT: usize = PROBE_MAX_SEATS;

/// A track owner's row in the tally, or `None` for a seat number outside the
/// printed range (only a hand-built view has one) — `eval::majority_bonus`'s own
/// slot rule.
fn contender_slot(owner: PlayerId) -> Option<usize> {
    if owner.is_dummy() {
        return Some(PROBE_DUMMY_SLOT);
    }
    let index = usize::from(owner.0);
    (index < PROBE_MAX_SEATS).then_some(index)
}

/// One seat's SIGNED MARGIN in a management-board majority ranking.
///
/// # The arithmetic, and why it is shaped this way
///
/// `eval::majority_bonus` ranks contenders by the rulebook's key — pawn COUNT
/// first, then the RIGHTMOST occupied space (rulebook p.9; the count-then-
/// position tie-break is `game/scoring.rs`'s, differentially confirmed against
/// the independent oracle 2026-07-20) — and awards `vps[0]` / `vps[1]` to the
/// first two, a place taken by the 2P dummy being CONSUMED and awarded to
/// nobody (finding D1, rulebook p.9 NOTE).
///
/// That key is lexicographic and this column is one scalar, so the count gap is
/// SCALED BY TWO to leave room for the tie-break underneath it:
///
/// ```text
/// margin = 2·(my pawns − the best rival's pawns) + tie
/// tie    = +1 / −1 by rightmost occupied space, and ONLY on a count tie
/// ```
///
/// which is strictly monotone in the rulebook's own ordering: a one-pawn lead
/// reads at least +1 whatever the tie term does, and a count tie won on position
/// reads exactly +1. One space holds one pawn, so two contenders can never share
/// a rightmost index — the tie term is never 0 between two contenders that both
/// hold a pawn.
///
/// # What the sign means, exactly
///
/// `> 0` iff this seat holds FIRST place as the board stands — iff `eval`'s
/// structure would award it `vps[0]` right now. It does NOT distinguish second
/// place from not placing at all (two seats behind one leader both read
/// negative), which is deliberate: the T5 re-aim asks for majority ARITHMETIC
/// and opponent-progress AWARENESS, and "am I winning this majority, by how
/// much" is the quantity a seat can act on. `== 0` means no contender holds a
/// pawn at all.
///
/// # Who counts as a rival
///
/// The 2P dummy does, at every count where it holds a pawn. A seat holding NO
/// pawn does not — `majority_bonus` skips a zero tally, so such a seat is not in
/// the ranking and cannot be anybody's best rival. With no rival contending the
/// margin is simply twice this seat's own pawn count.
fn majority_margin(occupants: impl Iterator<Item = Option<PlayerId>>, me: PlayerId) -> i64 {
    let mut tally = [(0u8, 0u8); PROBE_MAX_SEATS + 1];
    for (index, owner) in occupants.enumerate() {
        let Some(owner) = owner else { continue };
        let Some(slot) = contender_slot(owner) else { continue };
        tally[slot].0 = tally[slot].0.saturating_add(1);
        tally[slot].1 = tally[slot].1.max(index as u8);
    }

    let Some(my_slot) = contender_slot(me) else { return 0 };
    let mine = tally[my_slot];
    let best = tally
        .iter()
        .enumerate()
        .filter(|(slot, entry)| *slot != my_slot && entry.0 > 0)
        .map(|(_, entry)| *entry)
        .max_by_key(|entry| (entry.0, entry.1));

    let Some(best) = best else {
        return 2 * i64::from(mine.0);
    };
    let gap = i64::from(mine.0) - i64::from(best.0);
    let tie = if mine.0 == best.0 {
        (i64::from(mine.1) - i64::from(best.1)).signum()
    } else {
        0
    };
    2 * gap + tie
}

/// Seats clockwise from the start player, 0 for the start player itself — the
/// official tie-break key for the technology majority. A restatement of
/// `eval::clockwise_distance`, which is private to that module; `rem_euclid`
/// rather than `%` for the same reason it gives (a hand-built config must not
/// underflow the subtraction).
fn probe_clockwise_distance(view: &PlayerView, player: PlayerId) -> i32 {
    let seats = i32::from(view.config.player_count);
    if seats <= 0 {
        return 0;
    }
    (i32::from(player.0) + seats - i32::from(view.config.start_player.0)).rem_euclid(seats)
}

/// One seat's SIGNED MARGIN in the TECHNOLOGY industry-value ranking (10 / 5).
///
/// The same shape as [`majority_margin`] over a different key: total industry
/// value first, then the official CLOCKWISE distance from the start player
/// (nearest wins). Values are carried DOUBLED, exactly as
/// `eval::technology_majority` and `scoring::calculate_technology_bonus` carry
/// them, because that is what makes D5 representable: at two players the setup
/// dummy competes at industry value exactly **7.5** (owner ruling 2026-08-08,
/// rulebook p.9 final-scoring NOTE), i.e. doubled 15 against real doubled totals
/// that are always even — so a tie against the dummy is unrepresentable and its
/// distance key is a placeholder that never gets read.
///
/// A player owning NO technology never places (`technology_majority` skips a
/// zero industry value), so it is never a rival here either; its own margin is
/// the whole doubled gap to the leader, which is the honest reading of "not in
/// this race at all".
fn tech_majority_margin(
    view: &PlayerView,
    me: &yokohama_core::VisiblePlayer,
    rules: &Rules,
) -> i64 {
    let doubled = |player: &yokohama_core::VisiblePlayer| -> u32 {
        2 * player
            .technologies
            .iter()
            .filter_map(|id| rules.techs.get(*id))
            .map(|card| u32::from(card.industry_cost))
            .sum::<u32>()
    };

    // (doubled industry value, clockwise distance), better = higher value then
    // SMALLER distance — `technology_majority`'s `better` predicate verbatim.
    let mut best: Option<(u32, i32)> = None;
    let mut consider = |candidate: (u32, i32)| {
        let improves = match best {
            None => true,
            Some(current) => {
                candidate.0 > current.0 || (candidate.0 == current.0 && candidate.1 < current.1)
            }
        };
        if improves {
            best = Some(candidate);
        }
    };
    for other in &view.players {
        if other.id == me.id || other.id.is_dummy() {
            continue;
        }
        let value = doubled(other);
        if value == 0 {
            continue;
        }
        consider((value, probe_clockwise_distance(view, other.id)));
    }
    if view.config.player_count == 2 {
        consider((15, i32::MAX));
    }

    let mine = doubled(me);
    let Some(best) = best else {
        return 2 * i64::from(mine);
    };
    let gap = i64::from(mine) - i64::from(best.0);
    let tie = if mine == best.0 {
        // Nearer to the start player is BETTER, so the sign is reversed against
        // the raw distance comparison.
        (i64::from(best.1) - i64::from(probe_clockwise_distance(view, me.id))).signum()
    } else {
        0
    };
    2 * gap + tie
}

/// How near this seat is to its NEXT achievement claim, negated so that nearer
/// reads HIGHER.
///
/// Over the cards this seat has not already claimed, each card's remaining gap
/// is `needed − min(current, needed)` on `eval::achievement_progress`'s own
/// arithmetic (the AND-shaped cards' per-leg caps included), and the column is
/// minus the SMALLEST of them.
///
/// Three decisions, each recorded rather than implied:
///
/// * **Cards claimed by an OPPONENT stay live.** The engine's claim gate refuses
///   only a REPEAT claim by the SAME seat (`can_claim_achievement`), so a card
///   an opponent has taken is still worth its printed subsequent value to this
///   seat, and dropping it would price a reachable reward at nothing. This is
///   also why the column is SELF-guidance rather than a denial term: the owner's
///   2026-08-09 canon batch rules achievement denial explicitly NOT canon, and
///   names achievements agenda-setters instead.
/// * **The cap is inherited**, so hoarding past a requirement does not read as
///   extra nearness — the same reasoning `achievement_progress_sum` gives.
/// * **A seat that has claimed all three reads 0**, which COLLIDES with the
///   reading for a seat standing on a claimable card. Both are "nothing left to
///   chase"; the state needs three claims by one seat to occur at all; and the
///   alternative — a sentinel outside the scale — would put a discontinuity into
///   a column whose whole content is an ordering. Stated so a reader of the L1
///   table knows which zero they are looking at.
fn achievement_race_proximity(view: &PlayerView, me: &yokohama_core::VisiblePlayer) -> i64 {
    let mut nearest: Option<u32> = None;
    for card in &view.achievements {
        if card.claimed_by.contains(&me.id) {
            continue;
        }
        let (current, needed) = achievement_progress(view, me, card.achievement_type);
        let gap = needed.saturating_sub(current.min(needed));
        nearest = Some(nearest.map_or(gap, |best| best.min(gap)));
    }
    -i64::from(nearest.unwrap_or(0))
}

/// Empty Port CARD SLOTS across every Port board in play (six per board; two
/// boards from three players up).
///
/// T6's dropped conjunct: the `OrderDeckEmpty` end trigger fires on "the order
/// deck is empty AND a Port slot is empty" (`01-rules-reference.md:514`), and
/// only the deck half has ever been priced — `progress_ppm`'s deck clock reads
/// the deck, nothing reads the slots.
///
/// **BOARD-GLOBAL, and the report must not be able to hide it:** every seat sees
/// the same number, so the relative form (`viewer − best opponent`) is
/// identically 0 and its AUC column is CONSTANT by construction. The ABSOLUTE
/// form's per-stage trajectory is the whole of what this column can tell an L1
/// reading, and `port_empty_slots_counts_every_board_and_reads_the_same_from_
/// every_seat` pins both halves of that through `row_values` itself.
///
/// **A per-seat formulation is possible and is deliberately NOT what this is.**
/// The faithful seat-relative shape of the dropped conjunct is an INTERACTION —
/// the clock's nearness multiplied by whether ending now favours this seat —
/// and the second factor is terminal-margin awareness, which the protocol lists
/// as its own T6 candidate. Building the product here would measure the two
/// candidates at once and let a null in either hide behind the other, so this
/// column stays the raw conjunct and the interaction is proposed rather than
/// implemented (round-3 report).
fn port_empty_slots(view: &PlayerView) -> i64 {
    let empty = |board: &yokohama_core::PortBoard| -> i64 {
        board.slots.iter().filter(|slot| slot.is_none()).count() as i64
    };
    empty(&view.port_state.port_a) + view.port_state.port_b.as_ref().map_or(0, empty)
}

// ---------------------------------------------------------------------------
// The V3 admission column's arithmetic (C1)
// ---------------------------------------------------------------------------

/// This seat's row in a [`ScoreStructure`]'s per-seat vectors, or `None` for the
/// dummy sentinel and for a seat number outside the printed range.
///
/// `eval::seat_index`'s rule verbatim, restated here because that function is
/// private — the same treatment [`contender_slot`] gives `eval`'s own slot rule
/// one section up, and for the same reason. The two are DIFFERENT rules and are
/// deliberately not one function: a majority tally gives the 2P dummy a
/// contender row, and a score structure gives it none.
fn structure_seat(player: &yokohama_core::VisiblePlayer) -> Option<usize> {
    if player.id.is_dummy() {
        return None;
    }
    let i = usize::from(player.id.0);
    (i < PROBE_MAX_SEATS).then_some(i)
}

/// One seat's WOULD-BE-FINAL MARGIN if the game were scored where it stands:
/// its full final total minus the best other seated non-dummy seat's.
///
/// # The arithmetic
///
/// ```text
/// margin = final_total(me) − max over the other seated non-dummy seats
/// ```
///
/// with [`ScoreStructure::final_total`] supplying each total — running VP plus
/// country, technology, church, customs and the leftover conversions. Raw,
/// signed, unclamped, and with NO rank component: `TerminalStanding` shapes the
/// same two totals into a win/loss ladder because `score` needs an ORDERING,
/// and this column deliberately reports the underlying quantity instead, which
/// is what an L1 separation reading is about. **No opponent ⇒ 0**, spelled as
/// `unwrap_or(my_total)` — `terminal_standing_of`'s own convention for a
/// single-seat view, which only a fixture can produce.
///
/// # Cost
///
/// Derives a whole [`ScoreStructure`], so a snapshot pays one derivation per
/// seat on top of the one `row_values` already made. Accepted instrument cost
/// (V3 spec §4.1): threading the caller's structure would put a
/// `&ScoreStructure` parameter on [`CandidateColumn::value`] for one of nine
/// arms, and this is an example whose corpus cost is dominated by playing the
/// games. The derivation is a pure function of `(view, rules)`, so the two
/// cannot disagree.
///
/// # Honesty
///
/// Every input is a face-up field of the view — this is the interaction
/// [`port_empty_slots`]' doc names as its own candidate, computed on its own.
/// Pinned against `driven::true_margin_now` by behaviour over organic play at
/// 2/3/4P, which is the arithmetic reference and stays selection-side.
fn terminal_margin_now(
    view: &PlayerView,
    me: &yokohama_core::VisiblePlayer,
    rules: &Rules,
) -> i64 {
    let structure = ScoreStructure::derive(view, rules);
    let Some(mine) = structure_seat(me) else { return 0 };
    let my_total = structure.final_total(mine);
    let mut best_other: Option<i64> = None;
    for player in &view.players {
        let Some(i) = structure_seat(player) else { continue };
        if i == mine {
            continue;
        }
        let total = structure.final_total(i);
        best_other = Some(best_other.map_or(total, |best: i64| best.max(total)));
    }
    my_total.saturating_sub(best_other.unwrap_or(my_total))
}

// ---------------------------------------------------------------------------
// Candidate weight tables (the protocol's L2 arm)
// ---------------------------------------------------------------------------

/// One weight table named on the command line, to be EVALUATED on this corpus.
///
/// **A candidate never GENERATES a corpus** (protocol §2, verbatim: "candidate
/// weight tables are never evaluated by regenerating a corpus under candidate
/// play (circular and expensive); candidates are evaluated on the fixed corpora
/// by held-out fitting and by the arena screen"). That is why `--weights` — the
/// flag that chooses what the BOT plays — is untouched by this arm and still
/// takes only `v1 | pre-freeze`: the two flags mean different things, and
/// letting a draft table into the generator would quietly turn an incumbent-play
/// corpus into a candidate-play one wearing the same header.
/// One `+<column>=<early>/<late>` extension clause: the column it prices and
/// its early/late weight pair, in exactly the shape a shipped feature's pair
/// takes in a [`WeightTable`].
type ExtensionClause = (CandidateColumn, i64, i64);

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScoreTableSpec {
    name: String,
    table: WeightTable,
    /// The `+<column>=<early>/<late>` clauses this table carries on top of its
    /// base, in the order they were written — V3 spec §4.1's "17th column" L2
    /// machinery. Empty for every pre-V3 spelling, and an empty list is what
    /// makes an extension-free table byte-identical to what this parser built
    /// before the clause grammar existed.
    extensions: Vec<ExtensionClause>,
}

impl ScoreTableSpec {
    /// A base table with NO extension clauses — the pre-V3 shape, kept as a
    /// constructor so the three bare spellings and every fixture say what they
    /// mean rather than repeating an empty vector.
    fn named(name: &str, table: WeightTable) -> Self {
        ScoreTableSpec { name: name.to_string(), table, extensions: Vec::new() }
    }
}

/// Every accepted candidate-column token, as a refusal reads them out.
fn candidate_column_menu() -> String {
    CANDIDATE_COLUMNS.iter().map(|c| c.token()).collect::<Vec<_>>().join(" | ")
}

/// One `+<column>=<early>/<late>` clause, or `None` when `suffix` is not one.
///
/// STRICT and total: every part must be there and every part must parse, so a
/// suffix this rejects is a suffix the greedy strip leaves in the BASE. The
/// weights go through `i64::from_str`, which accepts a leading `+` — harmless
/// after the clause's own `=`, where no re-segmentation is possible, and the
/// grammar's only reserved use of `+` is the clause separator itself.
fn parse_extension_clause(suffix: &str) -> Option<ExtensionClause> {
    let (token, weights) = suffix.split_once('=')?;
    let column = CANDIDATE_COLUMNS.iter().copied().find(|c| c.token() == token)?;
    let (early, late) = weights.split_once('/')?;
    Some((column, early.trim().parse::<i64>().ok()?, late.trim().parse::<i64>().ok()?))
}

/// Strip the extension clauses off the RIGHT of a `--score-table` spec, and
/// hand back the base spelling that remains.
///
/// **The V3 spec §4.1 parse rule, verbatim:** clauses come off greedily from
/// the right while the suffix matches `+<known column token>=<i64>/<i64>`; what
/// remains is the base. Written as a search over `+` positions right to left
/// rather than a split, because a clause weight may legitimately carry its own
/// `+` sign (`money=+3/4`) and a naive "cut at the last `+`" would segment
/// inside it.
///
/// Anything left that still LOOKS like a clause — a `+` whose suffix carries an
/// `=` — is a typed refusal rather than base text: it is either an unknown
/// column or a clause whose weights did not read, and letting it fall through
/// to the vector parser would report a width error about a spec whose real
/// problem was a mistyped column name.
fn split_extension_clauses(spec: &str) -> Result<(&str, Vec<ExtensionClause>), String> {
    let mut rest = spec;
    let mut clauses: Vec<ExtensionClause> = Vec::new();
    'strip: loop {
        for (at, _) in rest.char_indices().rev().filter(|(_, c)| *c == '+') {
            if let Some(clause) = parse_extension_clause(&rest[at + 1..]) {
                clauses.push(clause);
                rest = &rest[..at];
                continue 'strip;
            }
        }
        break;
    }
    clauses.reverse();

    if let Some(at) = rest.rfind('+') {
        let suffix = &rest[at + 1..];
        if let Some((token, weights)) = suffix.split_once('=') {
            let known = CANDIDATE_COLUMNS.iter().any(|c| c.token() == token);
            return Err(if known {
                format!(
                    "--score-table {spec:?}: the extension clause {suffix:?} is malformed — a \
                     clause is `+<column>=<early>/<late>` with two i64 weights, and {weights:?} \
                     is not that"
                )
            } else {
                format!(
                    "--score-table {spec:?}: the extension clause {suffix:?} names no known \
                     candidate column ({token:?}); accepted: {}",
                    candidate_column_menu(),
                )
            });
        }
    }

    // A clause with nothing named in front of it has a base but no name to
    // report the row under, and two such rows would both print as `v2`.
    if !clauses.is_empty() && matches!(rest, "v1" | "v2" | "v2-draft") {
        return Err(format!(
            "--score-table {spec:?}: an extension table needs a NAME to be reported under — \
             write `<name>={rest}+…`, not `{rest}+…`"
        ));
    }

    for (i, (column, _, _)) in clauses.iter().enumerate() {
        if clauses[..i].iter().any(|(seen, _, _)| seen == column) {
            return Err(format!(
                "--score-table {spec:?}: the candidate column {:?} is extended TWICE; one clause \
                 per column (two would price one term at their sum and read in the report as two)",
                column.token(),
            ));
        }
    }
    Ok((rest, clauses))
}

/// Parse one `--score-table` value.
///
/// The grammar is `<base>` or `<name>=<base>[+<column>=<early>/<late>]…`, where
/// `<base>` is one of:
///   * `v1` — the pre-v2-freeze table, i.e. the REPRODUCTION CHECK for every
///     round command (run it beside a real candidate and the candidate's
///     numbers have something to be read against on the same split);
///   * `v2` — [`WeightTable::V2`], the table that SHIPS since the freeze;
///   * `v2-draft` — the same numbers under the spelling the rounds used
///     (`WeightTable::V2_DRAFT` is an alias of `V2` since the freeze promoted
///     it), kept so the committed round commands still parse and reproduce;
///   * `<e0,…,e15>/<l0,…,l15>` — sixteen comma-separated `i64` per vector, in
///     [`Feature::ALL`] order, for a grid cell that has no const yet.
///
/// The three bare spellings stay legal WITHOUT a name, which is what keeps the
/// committed round commands byte-compatible; a name is required as soon as a
/// clause is present, because the clause is what the row has to be told apart
/// by. Every extension-free spec parses to an EMPTY clause list and prices
/// exactly as it did before the clause grammar existed.
///
/// **The clause layer is V3 spec §4.1's "17th column" machinery**: each clause
/// prices a [`CandidateColumn`] with an early/late weight pair, exactly the way
/// a shipped feature is priced, so an extension table IS the arithmetic preview
/// of admitting that column. `Feature::ALL` does not move — the base vector's
/// width stays keyed to `Feature::COUNT`, and a clause is not a feature.
///
/// Every refusal names what it SAW (the importer discipline: an unrecognized
/// input is a typed error, never a silent default). The vector arm checks its
/// lengths against `Feature::COUNT` rather than against a literal 16, so a
/// feature admission cannot leave this parser accepting a stale width — and it
/// refuses a leading-`+` ENTRY by name, because `+` is the clause separator and
/// a `+5` inside a vector is an ambiguity rather than a signed five.
fn parse_score_table(spec: &str) -> Result<ScoreTableSpec, String> {
    const FORMS: &str = "(v1 | v2 | v2-draft | <name>=<base>[+<column>=<early>/<late>]... where \
                         <base> is v1 | v2 | v2-draft | <16 comma-separated i64>/<16 \
                         comma-separated i64>)";

    let (base_spec, extensions) = split_extension_clauses(spec)?;

    let named = |base: &str| -> Option<WeightTable> {
        match base {
            "v1" => Some(WeightTable::V1),
            "v2" => Some(WeightTable::V2),
            "v2-draft" => Some(WeightTable::V2_DRAFT),
            _ => None,
        }
    };
    if let Some(table) = named(base_spec) {
        if extensions.is_empty() {
            return Ok(ScoreTableSpec::named(base_spec, table));
        }
    }

    let Some((name, base)) = base_spec.split_once('=') else {
        return Err(format!("unknown --score-table {spec:?} {FORMS}"));
    };
    if name.is_empty() {
        return Err(format!(
            "--score-table {spec:?} has an empty name before the `=`; a candidate needs a name \
             to be reported under {FORMS}"
        ));
    }
    if let Some(table) = named(base) {
        return Ok(ScoreTableSpec { name: name.to_string(), table, extensions });
    }
    let Some((early, late)) = base.split_once('/') else {
        return Err(format!(
            "--score-table {spec:?} names no LATE vector: the two vectors are separated by `/` \
             {FORMS}"
        ));
    };

    // `Feature::COUNT`, never a literal 16: a future feature admission moves the
    // width, and a parser holding a stale literal would accept exactly the
    // tables that no longer fit the vectors.
    let parse_vector = |raw: &str, which: &str| -> Result<[i64; Feature::COUNT], String> {
        let entries: Vec<&str> = raw.split(',').collect();
        if entries.len() != Feature::COUNT {
            return Err(format!(
                "--score-table {spec:?}: the {which} vector has {} entries, and Feature::ALL has \
                 {} — every weight is named or none is",
                entries.len(),
                Feature::COUNT,
            ));
        }
        let mut out = [0i64; Feature::COUNT];
        for (slot, entry) in entries.iter().enumerate() {
            let entry = entry.trim();
            if entry.starts_with('+') {
                return Err(format!(
                    "--score-table {spec:?}: the {which} vector's entry {} ({entry:?}) opens with \
                     `+`, which is ambiguous here — `+` separates extension clauses, so a vector \
                     entry is written bare or `-`-signed",
                    slot + 1,
                ));
            }
            out[slot] = entry.parse::<i64>().map_err(|e| {
                format!(
                    "--score-table {spec:?}: the {which} vector's entry {} ({entry:?}) is not an \
                     i64: {e}",
                    slot + 1,
                )
            })?;
        }
        Ok(out)
    };

    Ok(ScoreTableSpec {
        name: name.to_string(),
        table: WeightTable {
            early: parse_vector(early, "EARLY")?,
            late: parse_vector(late, "LATE")?,
        },
        extensions,
    })
}

/// Which half of the L2 split a game's cells belong to.
///
/// **A function of the GENERATION INDEX ALONE** — even = fit, odd = held-out —
/// pre-registered in the round-1 ledger row before any candidate was scored.
/// Never a function of a cell's value, its outcome label, or anything the corpus
/// measured: a split that read an outcome would make the held-out figure a
/// second look at the same data, which is the whole thing a held-out figure is
/// supposed to rule out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Split {
    Fit,
    HeldOut,
}

impl Split {
    fn of(game: u32) -> Self {
        if game.is_multiple_of(2) {
            Split::Fit
        } else {
            Split::HeldOut
        }
    }

    fn name(self) -> &'static str {
        match self {
            Split::Fit => "fit",
            Split::HeldOut => "held-out",
        }
    }

    /// This game's position among the games on its own side — the CLUSTER index
    /// the bootstrap resamples over, since `auc_weighted` indexes `mult` by the
    /// cluster id it is handed.
    ///
    /// One expression serves both sides because halving maps `0,2,4,…` and
    /// `1,3,5,…` onto `0,1,2,…` alike.
    fn cluster(self, game: u32) -> u32 {
        debug_assert_eq!(Self::of(game), self, "game {game} is not on the {} side", self.name());
        game / 2
    }

    /// How many clusters this side holds out of `games` generated. An odd game
    /// count leaves the extra game on the FIT side, which is what the even-index
    /// rule implies and is stated here so the resample population is read off
    /// the design rather than counted out of the rows.
    fn clusters(self, games: u32) -> usize {
        match self {
            Split::Fit => games.div_ceil(2) as usize,
            Split::HeldOut => (games / 2) as usize,
        }
    }
}

/// One view's positional total under a candidate spec, split at the base/
/// extension seam.
///
/// Both halves ride ONE pass over the structure: the paired-difference arm
/// (V3 spec §4.2) needs the base table's own number for every cell, and
/// re-running the sixteen-term mirror loop a second time to get it would double
/// an extension table's corpus cost for a quantity the first pass already had
/// in hand. For a spec with NO extension clauses the two fields are the same
/// bits — no arithmetic happens between them — which is what makes an
/// extension-free candidate's column identical to what it was before the clause
/// grammar existed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct CandidateTotals {
    base: f64,
    total: f64,
}

/// The evaluator's own positional total under a CANDIDATE table, RE-WEIGHTED
/// from the [`ScoreStructure`] this view already derived.
///
/// A deliberate MIRROR of `eval::score`'s loop — the interpolation, the
/// signed-weight opponent selection with its first-wins tie rule, and the
/// saturating arithmetic. It is a mirror rather than a second `score` call for
/// the cost reason this file's module doc opens with: `score` derives a whole
/// `ScoreStructure` per call, and a nineteen-candidate grid over a 340,000-view
/// corpus would pay that derivation six million extra times. The drift a mirror
/// risks is paid for by
/// `the_candidate_machinery_with_v1_reproduces_the_benchmark_row`, which pins
/// this against `score` itself, cell by cell, over a real corpus.
///
/// TERMINAL views short-circuit. §2.6 ranks a finished position instead of
/// scoring it, and the ladder value is a function of the standings alone — the
/// same number under every weight table — so `shipped_total`, the value `score`
/// already returned for this view, is the answer for every candidate and cannot
/// disagree with one. **On every corpus this instrument can generate the branch
/// is DEFENSIVELY UNREACHABLE** (Task-D review NOTE-1): snapshots are taken at
/// turn STARTS only, and a finished game records no further turn start, so no
/// recorded view is terminal. It is kept for totality — a hand-built corpus
/// could contain one — and it is deliberately the trivial pass-through above
/// precisely so unreachable code cannot lie about a weight table.
#[allow(clippy::too_many_arguments)]
fn candidate_total(
    view: &PlayerView,
    me: &yokohama_core::VisiblePlayer,
    structure: &ScoreStructure,
    rules: &Rules,
    spec: &ScoreTableSpec,
    progress: u32,
    terminal: bool,
    shipped_total: f64,
) -> CandidateTotals {
    if terminal {
        return CandidateTotals { base: shipped_total, total: shipped_total };
    }
    let mut total: i64 = 0;
    for feature in Feature::ALL {
        let (early, late) = spec.table.get(feature);
        let weight = interpolate(early, late, progress);
        let mine = structure.value_of(me, feature);
        let theirs = opponent_extreme(view, me, weight, |other| structure.value_of(other, feature));
        total = total.saturating_add(weight.saturating_mul(mine.saturating_sub(theirs)));
    }
    let base = total;

    // The §4.1 extension clauses, priced EXACTLY as the sixteen above are: one
    // interpolated weight, the viewer's value minus the same signed-weight
    // opponent extreme, saturating throughout. That identity is the whole
    // instrument — an extension table is what admitting the column would
    // arithmetically do, not an approximation of it.
    for (column, early, late) in &spec.extensions {
        let weight = interpolate(*early, *late, progress);
        let mine = column.value(view, me, rules);
        let theirs = opponent_extreme(view, me, weight, |other| column.value(view, other, rules));
        total = total.saturating_add(weight.saturating_mul(mine.saturating_sub(theirs)));
    }
    CandidateTotals { base: base as f64, total: total as f64 }
}

/// `eval::opponent_extreme`'s rule, reproduced once for BOTH halves of
/// [`candidate_total`].
///
/// Maximize `weight · value` over the non-dummy opponents and return the
/// winner's RAW value, with ties — including every tie at `weight == 0` —
/// keeping the FIRST opponent in seat order, and `0` when the viewer has no
/// opponent at all. Unexercised at 2P, where the maximum is over one opponent;
/// the corpora are not the only thing this runs on, and a NEGATIVE weight picks
/// a different seat than a plain maximum would, which is the drift a shared
/// helper exists to prevent between the feature loop and the clause loop.
fn opponent_extreme(
    view: &PlayerView,
    me: &yokohama_core::VisiblePlayer,
    weight: i64,
    value_of: impl Fn(&yokohama_core::VisiblePlayer) -> i64,
) -> i64 {
    let mut theirs: Option<i64> = None;
    for other in &view.players {
        if other.id == me.id || other.id.is_dummy() {
            continue;
        }
        let value = value_of(other);
        let improves = match theirs {
            None => true,
            Some(current) => weight.saturating_mul(value) > weight.saturating_mul(current),
        };
        if improves {
            theirs = Some(value);
        }
    }
    theirs.unwrap_or(0)
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
#[allow(clippy::too_many_arguments)]
fn row_values(
    view: &PlayerView,
    rules: &Rules,
    weights: &WeightTable,
    rows: &[Row],
    candidates: &[ScoreTableSpec],
    out_rel: &mut [f64],
    out_abs: &mut [f64],
    out_cand: &mut [f64],
    out_base: &mut [f64],
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
            // Same plain-max convention as the feature rows above, and for the
            // same reason: this column is measured, not shipped, so the
            // orientation question a signed weight answers does not arise.
            Row::Candidate(c) => {
                let theirs = view
                    .players
                    .iter()
                    .filter(|p| p.id != me.id && !p.id.is_dummy())
                    .map(|p| c.value(view, p, rules))
                    .max()
                    .unwrap_or(0);
                (c.value(view, me, rules) as f64, theirs as f64)
            }
            // Already relative, and an "absolute" evaluator total is not a
            // defined quantity — the row reports the same number in both
            // columns rather than inventing one.
            Row::EvalTotal => (total, 0.0),
        };
        out_rel[slot] = mine - theirs;
        out_abs[slot] = mine;
    }

    // The candidate tables, priced on the structure derived once above. Costs
    // nothing at all when no `--score-table` was given, which is what keeps an
    // argument-free run byte-identical to every recorded appendix.
    if !candidates.is_empty() {
        let progress = progress_ppm(view, rules);
        let terminal = view_is_terminal(view);
        for (slot, candidate) in candidates.iter().enumerate() {
            let totals = candidate_total(
                view,
                me,
                &structure,
                rules,
                candidate,
                progress,
                terminal,
                total,
            );
            out_cand[slot] = totals.total;
            out_base[slot] = totals.base;
        }
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
    /// One EVAL TOTAL per `--score-table` candidate, in the order they were
    /// named. Empty — and allocation-free — when none was given.
    cand: Vec<f64>,
    /// The same cell under each candidate's BASE table alone, extension clauses
    /// unpriced — the second arm of the §4.2 paired difference. Bit-identical
    /// to [`cand`](Self::cand) for a candidate that carries no clause, which is
    /// the property the zero-extension reproduction pin reads.
    base: Vec<f64>,
}

/// A per-(game, seat, stage) accumulator, folded into a [`Cell`] at game end.
struct CellAcc {
    n: u64,
    rel: Vec<f64>,
    abs: Vec<f64>,
    cand: Vec<f64>,
    base: Vec<f64>,
}

impl CellAcc {
    fn new(rows: usize, candidates: usize) -> Self {
        CellAcc {
            n: 0,
            rel: vec![0.0; rows],
            abs: vec![0.0; rows],
            cand: vec![0.0; candidates],
            base: vec![0.0; candidates],
        }
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
    /// The V3 spec §4.2 ladder anchor's per-column input: `|relative value|` at
    /// every SNAPSHOT, one sketch per ROW slot.
    ///
    /// Indexed by row slot rather than by position among the candidate columns
    /// so it cannot drift out of step with `rows` — only `Row::Candidate` slots
    /// are ever observed, and the rest stay empty. EMPTY (allocation-free) when
    /// the run named no candidate column, which is what keeps an
    /// argument-free run byte-identical to every recorded appendix.
    ///
    /// Per-SNAPSHOT, deliberately NOT per-cell: the anchor scales a WEIGHT
    /// against the magnitudes a decision actually sees, and a cell is a mean
    /// over a whole (game × seat × stage) block, which is a smaller number for
    /// a reason that has nothing to do with the weight.
    anchor_rel: Vec<QuantileSketch>,
    /// The anchor's shared TARGET: `|interpolate(V2's VictoryPoints pair,
    /// progress) · VP relative value|` at every snapshot.
    ///
    /// `WeightTable::V2` regardless of `--weights`, because V2 is the base every
    /// §4.1 extension table rides on — the anchor calibrates against the term
    /// the candidate would sit beside in the SHIPPED table, not against
    /// whatever table generated the corpus.
    anchor_vp_product: QuantileSketch,
    turns: QuantileSketch,
    moves: QuantileSketch,
    trigger: TriggerTally,
    applied: u64,
    retreat_picks: u64,
    nonzero_recovery: u64,
    area_actions: u64,
    builds: u64,
    /// Explaining contestants only; zero under a policy rung, which has no cap
    /// to bind.
    cap_binds: u64,
    decisions: u64,
    candidates: u64,
    /// The two SEARCH guards, `None` on a contestant that has no such quantity
    /// (see [`ProbeTally::ply_cap_hits`]). RECORDED here and asserted nowhere:
    /// the gate that asserts them is `tests/evaluator_gate.rs`.
    ply_cap_hits: Option<u64>,
    free_cap_hits: Option<u64>,
    /// Spec §3.4's budget meters, same `None` rule and the same
    /// recorded-never-asserted posture (`tests/architecture_gate.rs` is where
    /// their relations are gated). On an UNCAPPED corpus both read `Some(0)` by
    /// construction; on a capped one they are the figures that say what kind of
    /// corpus this actually is.
    budget_binds: Option<u64>,
    budget_starved: Option<u64>,
    /// The §0.3(b) identity, folded over (game × seat) blocks — see
    /// [`identity_blocks`]. Zero blocks under a policy rung, which explains
    /// nothing and hands back no tallies at all.
    identity_blocks: u64,
    identity_mismatches: u64,
    wall: f64,
}

#[allow(clippy::too_many_arguments)]
fn generate(
    players: u8,
    games: u32,
    contestant: ProbeContestant,
    weights: &WeightTable,
    rules: &Rules,
    master: SeedScheduleKind,
    invariants: bool,
    columns: &[CandidateColumn],
    candidates: &[ScoreTableSpec],
) -> Result<(Corpus, u64), String> {
    let (mut schedule, master_value) = match master {
        SeedScheduleKind::Diagnostic(m) => (SeedSchedule::diagnostic(m), m),
        SeedScheduleKind::OsEntropy => SeedSchedule::from_os_entropy(),
    };

    let rows = Row::all(columns);
    let mut corpus = Corpus {
        rows: rows.clone(),
        cells: Vec::new(),
        games: 0,
        snapshots: 0,
        proxy: Vec::new(),
        anchor_rel: if columns.is_empty() {
            Vec::new()
        } else {
            vec![QuantileSketch::default(); rows.len()]
        },
        anchor_vp_product: QuantileSketch::default(),
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
        ply_cap_hits: None,
        free_cap_hits: None,
        budget_binds: None,
        budget_starved: None,
        identity_blocks: 0,
        identity_mismatches: 0,
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
            if let Some(hits) = tally.ply_cap_hits() {
                *corpus.ply_cap_hits.get_or_insert(0) += hits;
            }
            if let Some(hits) = tally.free_cap_hits() {
                *corpus.free_cap_hits.get_or_insert(0) += hits;
            }
            if let Some(binds) = tally.budget_binds() {
                *corpus.budget_binds.get_or_insert(0) += binds;
            }
            if let Some(starved) = tally.budget_starved() {
                *corpus.budget_starved.get_or_insert(0) += starved;
            }
        }
        // A rung explains nothing and hands back no tallies at all, so there is
        // no identity to check — folding one anyway would report every seat of
        // every rung game as a mismatch. Gated on the CONTESTANT rather than on
        // `tallies.is_empty()`, so an explaining arm that stopped filling the
        // sink still reads as the violation it is.
        if !matches!(contestant, ProbeContestant::Policy(_)) {
            let explained: Vec<u64> = tallies.iter().map(ProbeTally::explained).collect();
            let (blocks, mismatches) = identity_blocks(&explained, &stats.moves_by_seat);
            corpus.identity_blocks += blocks;
            corpus.identity_mismatches += mismatches;
        }

        snapshot_game(&stats, players, weights, rules, &rows, candidates, game, &mut corpus)?;
    }
    corpus.wall = started.elapsed().as_secs_f64();

    Ok((corpus, master_value))
}

/// Play ONE game with the named contestant, returning its stats plus the
/// per-seat explanation tallies IN SEAT ORDER (empty for a policy rung, which
/// explains nothing).
///
/// Seat order is what makes the §0.3(b) identity a per-seat check rather than an
/// aggregate one: [`run_arena_game`] builds its agents `(0..players)`, so the
/// `i`-th tally belongs to the `i`-th entry of `ArenaGameStats::moves_by_seat`.
/// The ONE construction site for the probe's lookahead agent, pulled out of
/// [`play_one`]'s closure so a test can pin the knob-to-parameter wiring:
/// `worlds` and `retreat_samples` are ADJACENT `u8` parameters on
/// [`LookaheadAgent::with_knobs`], so a k/m transposition compiles clean, the
/// provenance echo would not reveal it (it prints the PARSED knobs), and every
/// corpus it generated would be mislabelled (W1-C review NOTE-3). The pin is
/// `lookahead_knobs_reach_their_agent_slots_unswapped`.
fn lookahead_probe_agent(
    rules: &Rules,
    b: u16,
    k: u8,
    m: u8,
    cap: Option<u32>,
    table: WeightTable,
    agent_seed: u64,
) -> LookaheadAgent {
    LookaheadAgent::with_knobs(rules, SearchScheme::TurnBoundary { beam: b }, k, m, table, agent_seed)
        .with_cap(cap)
}

fn play_one(
    seed: WorldSeed,
    players: u8,
    contestant: ProbeContestant,
    weights: &WeightTable,
    rules: &Rules,
    master: u64,
    invariants: bool,
) -> Result<(ArenaGameStats, Vec<ProbeTally>), String> {
    let table = *weights;
    match contestant {
        ProbeContestant::Greedy { m } => {
            drive_honest(seed, players, rules, invariants, |agent_seed| {
                let agent = GreedyAgent::with_weights(rules, table, m, agent_seed);
                let tally = ProbeTally::Greedy(agent.tally());
                (Box::new(agent), tally)
            })
        }
        // The SAME honest chain as the greedy arm — `run_arena_game`, one agent
        // per seat on its own `agent_stream_seed` stream — and deliberately NOT
        // `run_raw_game`: a search corpus generated from privileged state would
        // be a different measurement wearing this one's label, and the whole
        // point of the A0 (contestant × rev) corpus is that the two contestants
        // differ in nothing but the contestant.
        ProbeContestant::Lookahead { b, k, m, cap } => {
            drive_honest(seed, players, rules, invariants, |agent_seed| {
                let agent = lookahead_probe_agent(rules, b, k, m, cap, table, agent_seed);
                let tally = ProbeTally::Search(agent.tally());
                (Box::new(agent), tally)
            })
        }
        ProbeContestant::Policy(policy) => {
            let seats = vec![policy; players as usize];
            let stats = run_raw_game(seed, players, rules, &seats, master, invariants)
                .map_err(|e| format!("game at seed {:#018x}: {e}", seed.0))?;
            Ok((stats, Vec::new()))
        }
    }
}

/// Run one game through the HONEST driver with `make` at every seat.
///
/// Each agent's tally is cloned out BEFORE the agent moves into the driver — the
/// `HonestPathTally` precedent, and the only way to read the counters afterwards,
/// since a game's agents are built by the driver's own factory and never handed
/// back.
fn drive_honest(
    seed: WorldSeed,
    players: u8,
    rules: &Rules,
    invariants: bool,
    make: impl Fn(u64) -> (Box<dyn ArenaAgent>, ProbeTally),
) -> Result<(ArenaGameStats, Vec<ProbeTally>), String> {
    let handles: std::sync::Arc<std::sync::Mutex<Vec<ProbeTally>>> = std::sync::Arc::default();
    let sink = std::sync::Arc::clone(&handles);
    let mut factory = |seat: PlayerId, _n: u8| -> Box<dyn ArenaAgent> {
        let (agent, tally) = make(agent_stream_seed(seed, seat, Slot(seat.0)));
        sink.lock().expect("tally sink").push(tally);
        agent
    };
    let stats = run_arena_game(seed, players, rules, &mut factory, invariants)
        .map_err(|e| format!("game at seed {:#018x}: {e}", seed.0))?;
    let tallies = handles.lock().expect("tally sink").clone();
    Ok((stats, tallies))
}

/// Walk one finished game's record and fold every turn start into cells.
///
/// The walk is a REPLAY rather than a hook inside the driver: `run_arena_game`
/// hands back a `GameRecord`, and folding it here means the snapshots are taken
/// from exactly the states the recorded game passed through — the same fold
/// `replay`/`replay_strict` use, one event at a time, so this instrument cannot
/// have its own idea of what a game did.
/// Pass 1 of a game's fold: replay the record and keep the TURN-START states.
///
/// Cheap in memory next to the fold itself (a few dozen clones per game) and it
/// buys the POST-HOC label: the tercile of a turn is not knowable until the
/// game's turn count is. Shared by the AUC fold and §5.5's, so the two read the
/// same population of the same games.
fn turn_start_states(
    stats: &ArenaGameStats,
    rules: &Rules,
    game: u32,
) -> Result<Vec<GameState>, String> {
    let record = &stats.record;
    let pending = new_game(record.config.clone(), rules, record.seed)
        .map_err(|e| format!("replay setup at game {game}: {e}"))?;
    let mut state = finalize_setup(pending, &record.choices)
        .map_err(|e| format!("replay finalize at game {game}: {e}"))?;

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
    Ok(starts)
}

#[allow(clippy::too_many_arguments)]
fn snapshot_game(
    stats: &ArenaGameStats,
    players: u8,
    weights: &WeightTable,
    rules: &Rules,
    rows: &[Row],
    candidates: &[ScoreTableSpec],
    game: u32,
    corpus: &mut Corpus,
) -> Result<(), String> {
    let starts = turn_start_states(stats, rules, game)?;

    // Pass 2: one accumulator per (seat, stage), folded at the end.
    let turns = starts.len();
    let mut accs: Vec<CellAcc> = (0..usize::from(players) * STAGES.len())
        .map(|_| CellAcc::new(rows.len(), candidates.len()))
        .collect();
    let mut rel = vec![0.0f64; rows.len()];
    let mut abs = vec![0.0f64; rows.len()];
    let mut cand = vec![0.0f64; candidates.len()];
    let mut base = vec![0.0f64; candidates.len()];
    // Looked up once per game rather than per snapshot; `None` only if the
    // shipped feature list ever stops carrying running VP.
    let vp_slot = rows.iter().position(|row| matches!(row, Row::Feature(Feature::VictoryPoints)));

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
            row_values(
                &view,
                rules,
                weights,
                rows,
                candidates,
                &mut rel,
                &mut abs,
                &mut cand,
                &mut base,
            );
            // Per SEAT, per turn start — the same population the cells are
            // built from, before it is averaged into one. A no-op, and free of
            // the `progress_ppm` call, when no candidate column was named.
            if !corpus.anchor_rel.is_empty() {
                let progress = progress_ppm(&view, rules);
                observe_anchor(corpus, rows, &rel, vp_slot, progress);
            }
            let acc = &mut accs[usize::from(seat) * STAGES.len() + stage.index()];
            acc.n += 1;
            for slot in 0..rows.len() {
                acc.rel[slot] += rel[slot];
                acc.abs[slot] += abs[slot];
            }
            for (target, value) in acc.cand.iter_mut().zip(cand.iter()) {
                *target += value;
            }
            for (target, value) in acc.base.iter_mut().zip(base.iter()) {
                *target += value;
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
                cand: acc.cand.iter().map(|v| v / n).collect(),
                base: acc.base.iter().map(|v| v / n).collect(),
            });
        }
    }
    Ok(())
}

fn cmd_run(args: &[String]) -> Result<(), String> {
    let (positional, flags, repeated) = split_args(
        args,
        &["contestant", "weights", "master", "bootstrap", "blocks", "candidate-columns"],
        &["os-entropy", "invariants", "no-invariants"],
        &["score-table"],
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
    let columns = match flags.get("candidate-columns") {
        None => Vec::new(),
        Some(raw) => CandidateColumn::parse_list(raw)?,
    };
    // A candidate column only ever prints inside the features block, so a
    // --blocks list without `features` would silently drop the request — and
    // this file's rule is that ignored input is an error, never a no-op.
    if !columns.is_empty() && !blocks.features {
        return Err(
            "--candidate-columns rides the features block, and this --blocks list omits \
             `features` — add it or drop the flag (a silently-ignored column would violate \
             the ignored-input rule)"
                .to_string(),
        );
    }
    let candidates: Vec<ScoreTableSpec> = repeated
        .get("score-table")
        .map(|specs| specs.iter().map(|s| parse_score_table(s)).collect::<Result<Vec<_>, _>>())
        .transpose()?
        .unwrap_or_default();

    let rules = Rules::standard();
    let (corpus, master_value) = generate(
        players,
        games,
        contestant,
        &weights,
        &rules,
        master,
        invariants,
        &columns,
        &candidates,
    )?;

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
    // AFTER the standard blocks, and gated on the flag rather than on `--blocks`
    // — the candidate arm is not one of the three named blocks, and a run that
    // asked for a candidate table wants it whichever blocks it also asked for.
    if !candidates.is_empty() {
        print!(
            "{}",
            render_candidate_tables(&corpus, &weights, &candidates, bootstrap, resample_seed)
        );
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

/// The V3 spec §4.2 ladder anchor's inputs, one line each, or NO lines at all
/// when the run named no candidate column.
///
/// # What the lines are for
///
/// §4.2 pre-commits the ladder's centre rung before any fixed-corpus run:
/// `a` is the integer minimizing `|a · p90(|relative column value|) −
/// p50(|VP term product|)|`. Both quantities are read off ONE scratch run, so
/// the derivation is auditable from a printed artifact rather than
/// re-measured — which is why they are printed here, beside the columns they
/// describe, and not left for a reader to reconstruct. The features block
/// itself prints AUC and `mean_abs` only, and nothing anywhere else prints a
/// term PRODUCT.
///
/// # What they are NOT
///
/// Order statistics over SNAPSHOTS, not over the (game × seat × stage) cells
/// the AUC table is built from, and nearest-rank rather than interpolated —
/// [`QuantileSketch`]'s own rule. Neither line is a claim about anything: no
/// interval, no direction, no disposition. They are the two magnitudes the
/// anchor rule divides.
fn render_anchor_stats(corpus: &Corpus) -> Vec<String> {
    if corpus.anchor_rel.is_empty() {
        return Vec::new();
    }
    // An empty sketch has no order statistic; `--` says so rather than printing
    // a 0 a reader would divide by. Only a corpus that recorded no snapshot at
    // all reaches it.
    let stat = |sketch: &QuantileSketch, num: u64, den: u64| -> String {
        sketch.quantile(num, den).map_or_else(|| "--".to_string(), |v| v.to_string())
    };
    let mut lines = vec![
        "  ANCHOR STATS (V3 spec 4.2) — nearest-rank over SNAPSHOTS, not the cells above; the \
         ladder anchor a minimizes |a * p90(|column|) - p50(|VP term|)|, the VP term priced \
         under WeightTable::V2 at each snapshot's own progress"
            .to_string(),
        format!(
            "  ANCHOR STATS  {:<30} p50 = {}  (n = {})",
            "vp_term_product_v2 (shared)",
            stat(&corpus.anchor_vp_product, 1, 2),
            corpus.anchor_vp_product.len(),
        ),
    ];
    for (slot, row) in corpus.rows.iter().enumerate() {
        if !matches!(row, Row::Candidate(_)) {
            continue;
        }
        lines.push(format!(
            "  ANCHOR STATS  {:<30} p90 = {}  (n = {})",
            row.name(),
            stat(&corpus.anchor_rel[slot], 9, 10),
            corpus.anchor_rel[slot].len(),
        ));
    }
    lines
}

/// Fold ONE snapshot into the §4.2 anchor sketches.
///
/// `rel` is the very vector [`row_values`] just filled and the cell fold is
/// about to consume, so a column cannot be measured here and printed there from
/// two arithmetics. `vp_slot` is looked up from `rows` rather than assumed to be
/// zero — `Feature::VictoryPoints` leads `Feature::ALL` today, and this fold
/// must not be the thing that breaks if it ever stops leading it.
///
/// Values reach the sketch as whole numbers because they ARE whole numbers:
/// every row's relative form is a difference of `i64` feature values widened to
/// `f64`, and the VP product multiplies one of those by an `i64` weight.
/// `round` states that rather than trusting a truncating cast to agree.
fn observe_anchor(
    corpus: &mut Corpus,
    rows: &[Row],
    rel: &[f64],
    vp_slot: Option<usize>,
    progress: u32,
) {
    if corpus.anchor_rel.is_empty() {
        return;
    }
    let magnitude = |value: f64| -> u64 { value.abs().round() as u64 };
    for (slot, row) in rows.iter().enumerate() {
        if matches!(row, Row::Candidate(_)) {
            corpus.anchor_rel[slot].observe(magnitude(rel[slot]));
        }
    }
    let (early, late) = WeightTable::V2.get(Feature::VictoryPoints);
    let weight = interpolate(early, late, progress) as f64;
    let vp_rel = vp_slot.map_or(0.0, |slot| rel[slot]);
    corpus.anchor_vp_product.observe(magnitude(weight * vp_rel));
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
    // Printed only when a `--candidate-columns` row is present, so an
    // argument-free run stays byte-identical to every recorded appendix. The
    // marking is load-bearing: these rows sit in the same table as the sixteen
    // and ship in nothing, and a reader who takes one for a feature would read
    // the whole report as describing an evaluator that does not exist.
    let candidate_columns: Vec<&str> = corpus
        .rows
        .iter()
        .filter_map(|r| match r {
            Row::Candidate(c) => Some(c.name()),
            _ => None,
        })
        .collect();
    if !candidate_columns.is_empty() {
        println!(
            "  CANDIDATE COLUMNS — measured here, shipped by NOTHING, no weight and so no \
             disposition; Feature::ALL is still {}: {}",
            Feature::COUNT,
            candidate_columns.join(", "),
        );
    }
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

    // The §4.2 anchor inputs, printed only when a candidate column is present —
    // same rule as the CANDIDATE COLUMNS banner above, and for the same reason.
    let anchor = render_anchor_stats(corpus);
    if !anchor.is_empty() {
        println!();
        for line in &anchor {
            println!("{line}");
        }
    }
}

// ---------------------------------------------------------------------------
// Block (e): candidate weight tables, fit vs held-out
// ---------------------------------------------------------------------------

/// One (candidate × stage × split) column, pre-sorted for [`auc_weighted`].
struct SplitColumn {
    rows: Vec<(f64, u32, bool)>,
    /// Games on this side of the split — the resample's cluster count, taken
    /// from the DESIGN rather than from the rows, so a stage that happens to be
    /// missing a short game's cell still resamples the right population.
    clusters: usize,
    winners: usize,
}

/// Which of a candidate's two columns a [`SplitColumn`] collects.
///
/// The BASE arm is the same candidate with its extension clauses unpriced — the
/// comparison the §4.2 improvement interval is a difference against. For a
/// candidate with no clauses the two arms are the same numbers, and that is a
/// property rather than a coincidence: [`candidate_total`] computes them in one
/// pass and nothing happens between them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Arm {
    Extended,
    Base,
}

/// Collect one candidate's cells for one stage and one side of the split.
fn split_column(corpus: &Corpus, candidate: usize, stage: Stage, split: Split) -> SplitColumn {
    split_column_arm(corpus, candidate, stage, split, Arm::Extended)
}

/// [`split_column`] over a named [`Arm`].
fn split_column_arm(
    corpus: &Corpus,
    candidate: usize,
    stage: Stage,
    split: Split,
    arm: Arm,
) -> SplitColumn {
    let mut rows: Vec<(f64, u32, bool)> = corpus
        .cells
        .iter()
        .filter(|c| c.stage == stage && Split::of(c.game) == split)
        .map(|c| {
            let value = match arm {
                Arm::Extended => c.cand[candidate],
                Arm::Base => c.base[candidate],
            };
            (value, split.cluster(c.game), c.label)
        })
        .collect();
    rows.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("candidate totals are finite"));
    let winners = rows.iter().filter(|r| r.2).count();
    SplitColumn { rows, clusters: split.clusters(corpus.games), winners }
}

/// This candidate's own resample stream.
///
/// Derived from the DESIGN — the run's own design-derived `resample_seed` and
/// the candidate's WEIGHT-TABLE IDENTITY — and never from an outcome. The
/// identity rather than the position deliberately: a candidate re-run ALONE, to
/// check one row of a nineteen-cell grid, then reproduces the interval it was
/// quoted with, which a position-keyed stream would not.
fn candidate_resample_seed(resample_seed: u64, table: &WeightTable) -> u64 {
    mix64(resample_seed ^ mix64(weight_table_identity(table) as u64))
}

/// The percentile interval for the three HELD-OUT stage columns of ONE
/// candidate.
///
/// ONE multiplicity draw per replicate, reused across the three stages —
/// `report_features`'s rule, for its reason: the cluster resample is a property
/// of the replicate, not of the column, so drawing per stage would make a
/// candidate's own stage rows mutually incomparable.
///
/// Across CANDIDATES the sharing follows the KEY: [`candidate_resample_seed`]
/// keys the stream on the BASE weight table's identity, so two candidates that
/// name the SAME base — an extension table and the base itself, or two
/// extension tables on one base — draw the same multiplicities and ARE paired,
/// while two candidates on DIFFERENT bases are not. That same-base pairing is
/// not a side effect: it is what [`paired_improvement_intervals`] rests on, and
/// it is why the Δ column's replicates are the very ones behind the marginal
/// intervals printed beside them. The block header states both halves rather
/// than leaving a reader to assume either way.
fn held_out_intervals(
    columns: &[SplitColumn; 3],
    bootstrap: u32,
    seed: u64,
) -> [(f64, f64); 3] {
    let clusters = columns[0].clusters;
    if clusters == 0 || bootstrap == 0 {
        return [(f64::NAN, f64::NAN); 3];
    }
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut replicates: [Vec<f64>; 3] = [
        Vec::with_capacity(bootstrap as usize),
        Vec::with_capacity(bootstrap as usize),
        Vec::with_capacity(bootstrap as usize),
    ];
    let mut mult = vec![0u32; clusters];
    for _ in 0..bootstrap {
        mult.iter_mut().for_each(|m| *m = 0);
        for _ in 0..clusters {
            mult[rng.gen_range(0..clusters)] += 1;
        }
        for (s, column) in columns.iter().enumerate() {
            if let Some(a) = auc_weighted(&column.rows, &mult) {
                replicates[s].push(a);
            }
        }
    }
    let mut out = [(f64::NAN, f64::NAN); 3];
    for (s, reps) in replicates.iter_mut().enumerate() {
        reps.sort_by(|a, b| a.partial_cmp(b).expect("AUCs are finite"));
        if reps.is_empty() {
            continue;
        }
        out[s] = (
            reps[nearest_rank_index(reps.len(), 25, 1000)],
            reps[nearest_rank_index(reps.len(), 975, 1000)],
        );
    }
    out
}

/// One stage's held-out IMPROVEMENT of an extension table over its own base.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Improvement {
    /// `AUC(extended) − AUC(base)` on the unresampled column, or `None` when
    /// either side's AUC is undefined (one class empty).
    point: Option<f64>,
    /// The percentile endpoints of the PAIRED difference. `NaN` when no
    /// replicate produced a difference at all.
    lo: f64,
    hi: f64,
}

/// The §4.2 improvement interval: a PAIRED-DIFFERENCE held-out bootstrap of an
/// extension table against its own base.
///
/// **One multiplicity draw per replicate, applied to BOTH arms, differenced
/// inside the replicate** — never two independent bootstraps subtracted. The
/// two arms are the same cells of the same games under two pricings, so almost
/// all of the sampling variation is shared; differencing inside the replicate
/// cancels it, and differencing two independently-drawn intervals would add the
/// two variances instead and report a width the design does not have. A
/// zero-width interval here is therefore a real reading — the extension re-
/// ranked nothing in any resample — and not the refused kind, which is an
/// estimator that cannot tell "no variation" from "no measurement".
///
/// The stream is the candidate's own `resample_seed`, which is keyed on the
/// BASE weight table's identity (see [`candidate_resample_seed`]) — so these
/// replicates are the very ones behind the two marginal intervals printed
/// beside them.
fn paired_improvement_intervals(
    extended: &[SplitColumn; 3],
    base: &[SplitColumn; 3],
    bootstrap: u32,
    seed: u64,
) -> [Improvement; 3] {
    let mut out = [Improvement { point: None, lo: f64::NAN, hi: f64::NAN }; 3];
    for (s, slot) in out.iter_mut().enumerate() {
        let ones = vec![1u32; base[s].clusters.max(extended[s].clusters)];
        slot.point = match (
            auc_weighted(&extended[s].rows, &ones),
            auc_weighted(&base[s].rows, &ones),
        ) {
            (Some(e), Some(b)) => Some(e - b),
            _ => None,
        };
    }

    // The cluster population is the DESIGN's, taken from the base arm: the two
    // arms are the same cells of the same games under two pricings, so they
    // resample over one population by construction.
    let clusters = base[0].clusters;
    if clusters == 0 || bootstrap == 0 {
        return out;
    }
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut replicates: [Vec<f64>; 3] = [
        Vec::with_capacity(bootstrap as usize),
        Vec::with_capacity(bootstrap as usize),
        Vec::with_capacity(bootstrap as usize),
    ];
    let mut mult = vec![0u32; clusters];
    for _ in 0..bootstrap {
        mult.iter_mut().for_each(|m| *m = 0);
        for _ in 0..clusters {
            mult[rng.gen_range(0..clusters)] += 1;
        }
        // ONE draw, BOTH arms, differenced HERE — the whole point. The draw
        // sequence is `held_out_intervals`' own at this seed, so these are the
        // very replicates behind the marginal interval printed beside the Δ.
        for (s, reps) in replicates.iter_mut().enumerate() {
            if let (Some(e), Some(b)) =
                (auc_weighted(&extended[s].rows, &mult), auc_weighted(&base[s].rows, &mult))
            {
                reps.push(e - b);
            }
        }
    }
    for (s, reps) in replicates.iter_mut().enumerate() {
        reps.sort_by(|a, b| a.partial_cmp(b).expect("AUC differences are finite"));
        if reps.is_empty() {
            continue;
        }
        out[s].lo = reps[nearest_rank_index(reps.len(), 25, 1000)];
        out[s].hi = reps[nearest_rank_index(reps.len(), 975, 1000)];
    }
    out
}

/// One improvement cell, or the honest non-answer.
fn improvement_render(imp: &Improvement) -> String {
    match imp.point {
        None => "--  (one class empty)".to_string(),
        Some(p) if imp.lo.is_nan() => format!("{p:+.4}  (no interval)"),
        Some(p) => format!("{p:+.4} [{:+.4}, {:+.4}]", imp.lo, imp.hi),
    }
}

/// The NAME of a base weight table, or `None` for an explicit vector.
///
/// Compares the NUMBERS rather than carrying a spelling: `v2-draft` and `v2`
/// are one table since the freeze promoted the draft (the grammar test asserts
/// exactly that), so both report as `v2`, and a hand-written vector that
/// happens to equal a shipped table is that table — which is true, and is what
/// the improvement interval would be quoted against either way.
fn named_base_table(table: &WeightTable) -> Option<&'static str> {
    if *table == WeightTable::V1 {
        Some("v1")
    } else if *table == WeightTable::V2 {
        Some("v2")
    } else {
        None
    }
}

/// One AUC point estimate, or the two honest non-answers.
fn point_render(column: &SplitColumn) -> String {
    if is_constant(&column.rows) {
        return "constant".to_string();
    }
    match auc_weighted(&column.rows, &vec![1u32; column.clusters]) {
        None => "--".to_string(),
        Some(p) => format!("{p:.4}"),
    }
}

/// The whole `(e)` block as one deterministic string.
///
/// Returned rather than printed so the determinism claim is testable at all: a
/// block that only ever reaches stdout cannot be compared with itself.
fn render_candidate_tables(
    corpus: &Corpus,
    corpus_weights: &WeightTable,
    candidates: &[ScoreTableSpec],
    bootstrap: u32,
    resample_seed: u64,
) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    let corpus_identity = weight_table_identity(corpus_weights) as u64;
    let fit_games = Split::Fit.clusters(corpus.games);
    let held_games = Split::HeldOut.clusters(corpus.games);

    let _ = writeln!(
        out,
        "\n--- (e) candidate weight tables: EVAL TOTAL under a CANDIDATE table, fit/held-out by \
         game ---"
    );
    let _ = writeln!(
        out,
        "  L2 split: fit = EVEN generation index (0,2,4,...) · held-out = ODD (1,3,5,...) — a \
         function of the generation index ALONE, never of a cell's value or label"
    );
    let _ = writeln!(out, "  games: fit {fit_games} · held-out {held_games}");
    let _ = writeln!(
        out,
        "  HELD-OUT is the QUOTABLE evidence figure; fit is printed for the gap. NOTHING is \
         fitted on the fit half — candidates are hand-specified tables evaluated identically \
         on both halves, so the fit/held-out gap is sampling noise, not an overfit signature. \
         The overfitting hazard that IS live here is SELECTION across a candidate grid on \
         these held-out figures; a round quoting a grid winner carries that caveat, and the \
         protocol's L4 screen plus the freeze's fresh cell are the guards"
    );
    let _ = writeln!(
        out,
        "  bootstrap B={bootstrap}, game-clustered, on the HELD-OUT columns only; one draw per \
         replicate is shared by a candidate's three stages, so those are jointly resampled — and \
         shared by any two candidates that name the SAME BASE weight table, because the stream is \
         keyed on that table's identity. That sharing is what an extension table's Δ against its \
         own base rests on; two candidates on DIFFERENT bases are still NOT paired"
    );
    let _ = writeln!(
        out,
        "  each candidate's stream = mix64(run resample_seed ^ mix64(weight identity)) — the \
         DESIGN and the candidate's own identity, never the outcomes; keyed on IDENTITY not \
         position, so re-running one candidate alone reproduces its interval"
    );
    let _ = writeln!(
        out,
        "  candidates are EVALUATED on this corpus and NEVER generate one (protocol §2: \
         incumbent-play corpora only) — the games above were played under the header's \
         contestant and weight table, identity {corpus_identity:#018x}"
    );

    for (ordinal, candidate) in candidates.iter().enumerate() {
        let identity = weight_table_identity(&candidate.table) as u64;
        let seed = candidate_resample_seed(resample_seed, &candidate.table);
        let fit: Vec<SplitColumn> =
            STAGES.iter().map(|s| split_column(corpus, ordinal, *s, Split::Fit)).collect();
        let held: [SplitColumn; 3] = [
            split_column(corpus, ordinal, Stage::Early, Split::HeldOut),
            split_column(corpus, ordinal, Stage::Mid, Split::HeldOut),
            split_column(corpus, ordinal, Stage::Late, Split::HeldOut),
        ];
        let intervals = held_out_intervals(&held, bootstrap, seed);
        // The §4.2 improvement arm: an extension table on a NAMED base is
        // quoted against that base itself, over the SAME replicates.
        let base_name =
            (!candidate.extensions.is_empty()).then(|| named_base_table(&candidate.table)).flatten();
        let improvement = base_name.map(|_| {
            let base_arm: [SplitColumn; 3] = [
                split_column_arm(corpus, ordinal, Stage::Early, Split::HeldOut, Arm::Base),
                split_column_arm(corpus, ordinal, Stage::Mid, Split::HeldOut, Arm::Base),
                split_column_arm(corpus, ordinal, Stage::Late, Split::HeldOut, Arm::Base),
            ];
            paired_improvement_intervals(&held, &base_arm, bootstrap, seed)
        });

        let _ = writeln!(
            out,
            "\n  [{}] candidate={}  {}={identity:#018x}  vs corpus table: {}  \
             resample_seed={seed:#018x}",
            ordinal + 1,
            candidate.name,
            // An extension table's weight-table identity is its BASE's — the
            // clauses are not in it, and two extension tables on one base share
            // it — so the label says which quantity is being printed.
            if candidate.extensions.is_empty() { "identity" } else { "base identity" },
            match candidate.extensions.len() {
                0 =>
                    if identity == corpus_identity {
                        "SAME".to_string()
                    } else {
                        "DIFFERENT".to_string()
                    },
                n => format!(
                    "{} base plus {n} extension clause{}",
                    if identity == corpus_identity { "SAME" } else { "DIFFERENT" },
                    if n == 1 { "" } else { "s" },
                ),
            },
        );
        let _ = writeln!(out, "      early={:?}", candidate.table.early);
        let _ = writeln!(out, "      late ={:?}", candidate.table.late);
        if !candidate.extensions.is_empty() {
            let _ = writeln!(
                out,
                "      ext  =[{}]  (each priced as a feature is: interpolate(early,late) x \
                 (mine - the signed-weight opponent extreme))",
                candidate
                    .extensions
                    .iter()
                    .map(|(column, early, late)| format!("+{}={early}/{late}", column.token()))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
        let _ = writeln!(
            out,
            "      hand-split ordering (L14 core>on_board>excess, both vectors): {}",
            if yokohama_core::game::eval_screen::hand_split_ordering_holds(&candidate.table) {
                "ok"
            } else {
                "VIOLATED — protocol §4.2 forbids relaxing it at any ladder rung"
            },
        );
        let _ = writeln!(
            out,
            "      {:<7} {:>9} {:>9}   {:<26} {:>13}{}",
            "stage",
            "fit auc",
            "held auc",
            "held-out 95% [lo, hi]",
            "cells fit/held",
            // The LAST column, so it is never padded: a trailing run of spaces
            // in a report is bytes a diff has to explain.
            base_name.map_or(String::new(), |b| format!("   Δ vs {b}, 95% paired")),
        );
        for stage in STAGES {
            let s = stage.index();
            let (lo, hi) = intervals[s];
            let _ = writeln!(
                out,
                "      {:<7} {:>9} {:>9}   {:<26} {:>13}{}",
                stage.name(),
                point_render(&fit[s]),
                point_render(&held[s]),
                // A CONSTANT held-out column refuses its interval the way block
                // (a)'s `AucRow::render` does: every resample of a constant is
                // the constant, so [0.5000, 0.5000] would be a zero-width
                // interval claiming a measurement it did not make — the same
                // refusal `arena_gate`'s §0-R3 exists to enforce.
                if is_constant(&held[s].rows) {
                    "--  (constant: no interval)".to_string()
                } else if lo.is_nan() {
                    "--  (no interval)".to_string()
                } else {
                    format!("[{lo:.4}, {hi:.4}]")
                },
                format!(
                    "{}/{} ({}w/{}w)",
                    fit[s].rows.len(),
                    held[s].rows.len(),
                    fit[s].winners,
                    held[s].winners,
                ),
                improvement
                    .map_or(String::new(), |imp| format!("   {}", improvement_render(&imp[s]))),
            );
        }
        // The Δ column's reading, and — when there is none — why not. An
        // absence a reader has to infer is the failure mode this file refuses
        // everywhere else.
        match (candidate.extensions.is_empty(), base_name) {
            (false, Some(b)) => {
                let _ = writeln!(
                    out,
                    "      Δ = the PAIRED held-out difference against {b} ITSELF: one resample \
                     draw per replicate applied to BOTH arms and differenced inside the \
                     replicate, never two independent bootstraps subtracted. An exactly \
                     zero-width Δ is a real reading — the clauses re-ranked nothing in any \
                     resample — and improvement > 0 at the pre-registered stage is the §4.2 \
                     advance condition, quoted with this interval beside it"
                );
            }
            (false, None) => {
                let _ = writeln!(
                    out,
                    "      (no improvement interval: §4.2 quotes the paired difference against a \
                     NAMED base table, and this candidate's base is an explicit vector)"
                );
            }
            (true, _) => {}
        }
    }
    out
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

/// A counter this contestant may not have at all. `n/a` rather than `0`, on
/// [`ProbeTally`]'s `Option` rule: printing a zero for an absent quantity is
/// exactly the confusion the rule exists to prevent.
fn counter(value: Option<u64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |v| v.to_string())
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
        // FROZEN BLOCK. Its bytes are the cross-rev comparator for the A0
        // (contestant × rev) obligation — a greedy corpus at rev 7 has to diff
        // line for line against the rev-5 freeze artifact — so a new line HERE
        // would show up as a difference in every row of that comparison. The
        // search block below is where this instrument's new counters print, and
        // the identity VIOLATION line after the match fires on any arm, so the
        // freeze costs no coverage.
        ProbeContestant::Greedy { .. } => {
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
        ProbeContestant::Lookahead { .. } => {
            println!(
                "  placement cap binds         {:>10}  of {} decisions  (spec §3.3, summed over \
                 search NODES: one decision can bind it several times)",
                corpus.cap_binds, corpus.decisions,
            );
            println!(
                "  candidates/decision         {:>10.2}  ({} scored / {} decisions)",
                corpus.candidates as f64 / corpus.decisions.max(1) as f64,
                corpus.candidates,
                corpus.decisions,
            );
            println!(
                "  explained / applied         {:>10} / {}  over {} (game x seat) blocks, \
                 per-seat mismatches {}   <- spec §0.3(b)",
                corpus.decisions,
                corpus.applied,
                corpus.identity_blocks,
                corpus.identity_mismatches,
            );
            println!(
                "  ply cap hits                {:>10}  free cap hits {}   [RECORDED, never \
                 asserted here: this is an instrument; evaluator_gate is the gate]",
                counter(corpus.ply_cap_hits),
                counter(corpus.free_cap_hits),
            );
            // Spec §3.4's meters, so a CAPPED corpus cannot pass for the shipped
            // uncapped one: at cap=750 the budget binds at essentially every
            // decision and candidates/decision drops ~4.6x, which is a
            // materially different corpus wearing the same contestant name. On
            // an uncapped run 0 / 0 is the truthful shipped shape.
            println!(
                "  budget binds                {:>10}  starved {}   (spec §3.4, over {} decisions; \
                 0 / 0 is the UNCAPPED shape — architecture_gate is where the relations are gated)",
                counter(corpus.budget_binds),
                counter(corpus.budget_starved),
                corpus.decisions,
            );
        }
        ProbeContestant::Policy(_) => {
            println!("  placement cap binds         (n/a: a policy rung has no candidate cap)");
        }
    }
    // Fires on ANY arm, which is why the frozen greedy block above can stay
    // frozen: a violated identity is never silent, it just does not cost a line
    // when it holds.
    if corpus.identity_mismatches > 0 {
        println!(
            "  *** {} of {} (game x seat) blocks EXPLAINED A DIFFERENT NUMBER OF MOVES THAN THEY \
             APPLIED — spec §0.3(b) is violated and every figure above is suspect",
            corpus.identity_mismatches, corpus.identity_blocks,
        );
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
    let (positional, flags, _) = split_args(args, &["weights"], &[], &[])?;
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
// `pairs` — the V4 §5 fixture package at all thirty-four columns
// ---------------------------------------------------------------------------

/// **THE STEP-3 ADAPTER.** One candidate feature, for one seat of one view.
///
/// This is the function `pair_fixtures::CandidateColumns` was declared to
/// receive, and it is deliberately nothing but a MATCH: the eighteen §3.2
/// helpers live on `eval::ScoreStructure` (spec §13 — "`eval.rs` itself carries
/// the eighteen helpers"), and a second derivation of any of them here would be
/// the one thing that could make the probe's columns and the shipped evaluator's
/// disagree. `CandidateColumns`' own doc names the shape it wants:
/// `eval::raw_value`'s, so that "the adapter is a match, not a translation
/// layer".
///
/// **The structure is derived ONCE per call**, as `raw_value` does — the four
/// track majorities and the whole §3.2 view context are properties of the table,
/// so a helper-by-helper re-derivation would pay for them eighteen times per
/// seat. A print harness can afford the per-call derive that `raw_value` also
/// pays; the inner loops that cannot (the corpus sweep above, and step 4's
/// collision instrument) derive once and call the methods directly, which is
/// exactly what the adapter's identity pin checks they may do.
///
/// **It never declines.** `CandidateColumns` returns an `Option` because the
/// print harness needs a "not built yet" path — that is what
/// `pair_fixtures::CANDIDATE_PENDING` renders, and it is reached by passing
/// `None` for the whole adapter rather than by a label refusing. Every one of
/// the eighteen helpers is total on any view `observe` can produce, so every arm
/// here returns `Some`, and the pin says so: a `None` from this function would
/// print as PENDING and read as "step 3 never built that column".
///
/// A NON-CAPTURING free fn because `CandidateColumns` is a plain `fn` pointer,
/// not a boxed closure — the seam is deliberately capture-free so a fixture
/// report cannot be rendered against columns that quietly carry state.
/// The [`CandidateColumns`](yokohama_core::game::pair_fixtures::CandidateColumns)
/// adapter: one derive per call, eighteen arms onto `eval`'s own helpers.
///
/// It never declines. The `Option` is the SEAM's shape, not this adapter's
/// opinion — `render_pair_report_with` renders the pending marker when the
/// whole adapter is absent, and a `None` from a wired adapter would print that
/// marker for a column that exists.
fn candidate_columns(
    view: &PlayerView,
    player: &VisiblePlayer,
    feature: CandidateFeature,
    rules: &Rules,
) -> Option<i64> {
    Some(candidate_value(&ScoreStructure::derive(view, rules), view, player, feature, rules))
}

/// The eighteen arms themselves, over a structure the CALLER derived.
///
/// Split out of [`candidate_columns`] for the one caller that needs many
/// columns of one view at once: [`signature`] reads all eighteen for both
/// seats, and the adapter's own contract is one derive PER CALL, which would
/// make that thirty-six derivations of the same structure. Four of the five
/// score-structure features are rankings over the whole table, so this is the
/// same discipline `eval::score` and `relative_vector` already follow — and
/// keeping ONE match here rather than restating it at the second caller is what
/// stops the two from drifting (the adapter pin covers both through this).
fn candidate_value(
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
        CandidateFeature::MatchedProductionAccess => {
            s.matched_production_access(view, player, rules)
        }
    }
}

fn cmd_pairs(args: &[String]) -> Result<(), String> {
    let (positional, flags, _) = split_args(args, &["pair", "stage"], &["manifest"], &[])?;
    if !positional.is_empty() {
        return Err(format!("pairs takes no positional arguments\n\n{USAGE}"));
    }
    let only_stage = match flags.get("stage").map(String::as_str) {
        None => None,
        Some("early") => Some(StageEndpoint::Early),
        Some("late") => Some(StageEndpoint::Late),
        Some(other) => return Err(format!("unknown stage `{other}` (early|late)")),
    };
    // An unknown id is an ERROR, never an empty dump: a mistyped `--pair 4c`
    // that printed a header and no reports reads exactly like a package that
    // lost a registration.
    let only_pair = match flags.get("pair") {
        None => None,
        Some(id) => Some(
            registered(id).ok_or_else(|| format!("`{id}` is not a registered pair"))?,
        ),
    };

    let rules = Rules::standard();
    println!("=== eval_probe pairs [semantics rev {SEMANTICS_REV}] ===");
    print!("{}", render_registry_manifest());
    if flags.contains_key("manifest") {
        return Ok(());
    }

    let mut printed = 0usize;
    let mut skipped = 0usize;
    for pair in REGISTERED.iter() {
        if only_pair.is_some_and(|p| p.id != pair.id) {
            continue;
        }
        for stage in pair.stages {
            if only_stage.is_some_and(|s| s != *stage) {
                skipped += 1;
                continue;
            }
            let fixture = build_registered_pair(pair, *stage, &rules)
                .map_err(|e| format!("{} at {}: {e:?}", pair.id, stage.name()))?;
            println!();
            print!("{}", render_pair_report_with(&fixture, &rules, Some(candidate_columns)));
            printed += 1;
        }
    }
    println!();
    println!(
        "reports             {printed} printed, {skipped} outside the named endpoint  \
         (digest {})",
        package_digest(),
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// §5.3 Part A — the collision instrument
// ---------------------------------------------------------------------------

/// **THE §5.3 SIGNATURE of ONE view**, taken from a NAMED MOVER seat:
/// `(exact progress_ppm, Δv over the 34 features)`.
///
/// # What `Δv` is here, and why the seat is part of the question
///
/// `Δv` is the MOVER-RELATIVE vector — `mine − theirs`, one entry per axis slot,
/// at 2P where "theirs" is the single non-dummy opponent. That is the quantity
/// a weight multiplies (`game/eval.rs`'s `w · (mine − theirs)`), so §5.4's solve
/// row `Σ_f w_f(stage)·Δv_f ≥ margin` is the DIFFERENCE of two signatures'
/// vectors, and two views sharing a signature score identically under EVERY
/// weight vector in the box. That is exactly what makes a collision fatal to a
/// gate pair and merely interesting anywhere else.
///
/// The seat is load-bearing twice over and neither reading is optional: the
/// VIEW is the mover's (hidden information is masked as the mover sees it) and
/// the DIFFERENCING is signed from the mover. Six of the eighteen candidate
/// columns — O1, O2, O3, O5, O6, X3 — do NOT simply negate when the other seat
/// asks, so an opponent-side signature is not recoverable from this one by
/// flipping signs (step 3's carry-7 finding 2, pinned in this file's tests).
///
/// 2P only, by §5.3: at 3–4P `opponent_extreme` orients by weight, which would
/// make the "relative" half of the signature weight-dependent and the whole
/// instrument no longer weight-free.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Signature {
    ppm: u32,
    dv: [i64; AXIS_WIDTH],
}

/// Sign one view from `mover`'s seat.
///
/// A missing mover seat is a typed refusal rather than a zero vector: a
/// signature computed against an absent seat would be a perfectly well-formed
/// number that collides with everything.
fn signature(view: &PlayerView, mover: PlayerId, rules: &Rules) -> Result<Signature, String> {
    let me = view
        .players
        .iter()
        .find(|p| p.id == mover)
        .ok_or_else(|| format!("seat {} is not a player in this view", mover.0))?;
    let opponent = view.players.iter().find(|p| p.id != mover && !p.id.is_dummy());
    let structure = ScoreStructure::derive(view, rules);
    let mut dv = [0i64; AXIS_WIDTH];
    for f in Feature::ALL {
        let mine = structure.value_of(me, f);
        let theirs = opponent.map_or(0, |o| structure.value_of(o, f));
        dv[FeatureId::Incumbent(f).index()] = mine - theirs;
    }
    for c in CandidateFeature::ALL {
        let mine = candidate_value(&structure, view, me, c, rules);
        let theirs = opponent.map_or(0, |o| candidate_value(&structure, view, o, c, rules));
        dv[FeatureId::Candidate(c).index()] = mine - theirs;
    }
    Ok(Signature { ppm: progress_ppm(view, rules), dv })
}

/// Whether a successor is one of the two sides of its root's REGISTERED
/// development pair — §6's frozen recipe, computed by
/// [`escrow_recipe_pair`] from the ENGINE-EXPORTED rules quantity and from no
/// feature column.
///
/// This is the whole difference between a stop and a diagnostic on the root
/// side of §5.3's blocking rule. Two successors of one root that share a
/// signature are an ORGANIC collision — §5.3 says so in as many words, and the
/// e113 top tie is the case it names. Two successors that share a signature AND
/// are the pair the recipe registered are a row the solve can never satisfy.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum RecipeRole {
    Unregistered,
    Productive,
    Dead,
}

/// Where a signed view came from, in the words the artifact prints.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Provenance {
    /// One SIDE of one registered pair, at one endpoint.
    ///
    /// `gate` is [`PairRole::Constraint`] and nothing else. §5.3's inputs are
    /// "every GATE pair's sides (families 1–4, 6–8, 10(a)/(b)/(d), 11)" — the
    /// parenthetical breaks family 10 open precisely because 10(c) is its
    /// DIAGNOSTIC — and the frame's own type says the same: `Constraint` is "a
    /// solve row and a blocking side", `Diagnostic` is "2(c) and 10(c):
    /// RECORDED, never a gate". A diagnostic side is still SIGNED (a collision
    /// between it and a gate side is worth reading) and never blocks.
    PairSide { pair: String, stage: &'static str, side: &'static str, gate: bool },
    /// One non-terminal successor of one root, in one named successor SET.
    ///
    /// See [`sign_root_sets`] for why a root can have two.
    Successor { root: String, set: &'static str, ordinal: usize, mv: String, role: RecipeRole },
}

impl Provenance {
    fn label(&self) -> String {
        match self {
            Self::PairSide { pair, stage, side, gate } => {
                format!("pair {pair} {stage}/{side}{}", if *gate { "" } else { " [diagnostic]" })
            }
            Self::Successor { root, set, ordinal, mv, role } => {
                let tag = match role {
                    RecipeRole::Unregistered => "",
                    RecipeRole::Productive => " [REGISTERED productive]",
                    RecipeRole::Dead => " [REGISTERED dead]",
                };
                format!("succ {root}/{set}#{ordinal}{tag} {mv}")
            }
        }
    }
}

/// One view, signed.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct SignedView {
    sig: Signature,
    from: Provenance,
}

/// Why a class BLOCKS. §5.3's rule has exactly these arms in scope before step
/// 10 (the recipe-instantiated ESCROW pair is the third, and reading the escrow
/// band at all is fenced until then).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Blocking {
    /// Two sides of ONE registered gate pair at ONE endpoint.
    GatePair { pair: String, stage: &'static str },
    /// The two sides of ONE root's registered development successor pair.
    RootPair { root: String },
}

impl Blocking {
    fn label(&self) -> String {
        match self {
            Self::GatePair { pair, stage } => format!("gate pair {pair} at {stage}"),
            Self::RootPair { root } => format!("registered development pair at {root}"),
        }
    }
}

/// Views sharing an EXACT signature — same `progress_ppm`, same 34-vector.
struct CollisionClass {
    sig: Signature,
    members: Vec<Provenance>,
    blocking: Vec<Blocking>,
}

/// Views sharing a Δv at DIFFERENT `progress_ppm` — reported, never blocking.
struct NearClass {
    dv: [i64; AXIS_WIDTH],
    ppms: Vec<u32>,
    members: Vec<Provenance>,
}

/// The Part-A result: the exact-collision classes, the near-collision classes,
/// and how many views were signed to get them.
struct PartA {
    signed: usize,
    classes: Vec<CollisionClass>,
    near: Vec<NearClass>,
}

impl PartA {
    /// Every blocking finding across every class. §5.3: zero is necessary.
    fn blocking(&self) -> Vec<Blocking> {
        let mut out: Vec<Blocking> =
            self.classes.iter().flat_map(|c| c.blocking.iter().cloned()).collect();
        out.sort();
        out.dedup();
        out
    }

    /// The class-size distribution — `size -> how many classes`.
    fn size_histogram(&self) -> BTreeMap<usize, usize> {
        let mut out = BTreeMap::new();
        for class in &self.classes {
            *out.entry(class.members.len()).or_default() += 1;
        }
        out
    }

    fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("signed views        {}\n", self.signed));
        out.push_str(&format!("collision classes   {}\n", self.classes.len()));
        out.push_str(&format!(
            "  in classes        {}\n",
            self.classes.iter().map(|c| c.members.len()).sum::<usize>(),
        ));
        let histogram = self.size_histogram();
        out.push_str(&format!(
            "  size histogram    {}\n",
            if histogram.is_empty() {
                "-".to_string()
            } else {
                histogram
                    .iter()
                    .map(|(size, count)| format!("{size}x{count}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            },
        ));
        out.push_str(&format!("near-collisions     {}\n", self.near.len()));
        out.push_str(&format!("BLOCKING            {}\n", self.blocking().len()));

        for finding in self.blocking() {
            out.push_str(&format!("\nBLOCKING            {}\n", finding.label()));
        }
        for (i, class) in self.classes.iter().enumerate() {
            out.push_str(&format!(
                "\nclass {:<3}           ppm {} · size {}{}\n",
                i,
                class.sig.ppm,
                class.members.len(),
                if class.blocking.is_empty() { "" } else { "  *** BLOCKING ***" },
            ));
            out.push_str(&format!("  dv                {}\n", render_dv(&class.sig.dv)));
            for member in &class.members {
                out.push_str(&format!("  member            {}\n", member.label()));
            }
        }
        for (i, near) in self.near.iter().enumerate() {
            out.push_str(&format!(
                "\nnear  {:<3}           ppm {:?} · size {}\n",
                i,
                near.ppms,
                near.members.len(),
            ));
            out.push_str(&format!("  dv                {}\n", render_dv(&near.dv)));
            for member in &near.members {
                out.push_str(&format!("  member            {}\n", member.label()));
            }
        }
        out
    }
}

/// A Δv in one line, nonzero rows only — a 34-wide all-zero row is the common
/// case and printing it in full would bury the entries that matter.
fn render_dv(dv: &[i64; AXIS_WIDTH]) -> String {
    let nonzero: Vec<String> = FeatureId::all()
        .iter()
        .filter(|slot| dv[slot.index()] != 0)
        .map(|slot| format!("{}={}", slot.label(), dv[slot.index()]))
        .collect();
    if nonzero.is_empty() {
        "all zero".to_string()
    } else {
        nonzero.join(" ")
    }
}

/// Group signed views into §5.3's classes and apply its blocking rule.
///
/// Deterministic in the INPUT SET rather than in the input ORDER: every list
/// this builds is sorted before it is read, so the artifact is a function of
/// what was signed and not of the order the roots happened to resolve in.
fn classify(views: Vec<SignedView>) -> PartA {
    let signed = views.len();

    let mut exact: BTreeMap<Signature, Vec<Provenance>> = BTreeMap::new();
    let mut by_dv: BTreeMap<[i64; AXIS_WIDTH], Vec<(u32, Provenance)>> = BTreeMap::new();
    for view in views {
        by_dv.entry(view.sig.dv).or_default().push((view.sig.ppm, view.from.clone()));
        exact.entry(view.sig).or_default().push(view.from);
    }

    let mut classes = Vec::new();
    for (sig, mut members) in exact {
        if members.len() < 2 {
            continue;
        }
        members.sort();
        classes.push(CollisionClass { blocking: blocking_findings(&members), sig, members });
    }

    let mut near = Vec::new();
    for (dv, mut members) in by_dv {
        let mut ppms: Vec<u32> = members.iter().map(|(ppm, _)| *ppm).collect();
        ppms.sort_unstable();
        ppms.dedup();
        if ppms.len() < 2 {
            continue;
        }
        members.sort();
        near.push(NearClass {
            dv,
            ppms,
            members: members.into_iter().map(|(_, from)| from).collect(),
        });
    }

    PartA { signed, classes, near }
}

/// §5.3's blocking rule, applied to ONE class's members.
fn blocking_findings(members: &[Provenance]) -> Vec<Blocking> {
    let mut pair_sides: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
    let mut root_roles: BTreeMap<&str, Vec<RecipeRole>> = BTreeMap::new();
    for member in members {
        match member {
            // A diagnostic registration yields no solve row, so a collision
            // between its sides cannot make one infeasible.
            Provenance::PairSide { gate: false, .. } => {}
            Provenance::PairSide { pair, stage, side, gate: true } => {
                pair_sides.entry((pair.as_str(), stage)).or_default().push(side);
            }
            Provenance::Successor { root, role, .. } => {
                root_roles.entry(root.as_str()).or_default().push(*role);
            }
        }
    }

    let mut out = Vec::new();
    for ((pair, stage), mut sides) in pair_sides {
        sides.sort_unstable();
        sides.dedup();
        if sides.len() >= 2 {
            out.push(Blocking::GatePair { pair: pair.to_string(), stage });
        }
    }
    for (root, roles) in root_roles {
        if roles.contains(&RecipeRole::Productive) && roles.contains(&RecipeRole::Dead) {
            out.push(Blocking::RootPair { root: root.to_string() });
        }
    }
    out.sort();
    out
}

/// What the frozen package contributed, counted as the artifact prints it.
///
/// The ROW counts are over GATE registrations alone, which is what §5.4's
/// census means by a "pair row": the two diagnostics price nothing and yield
/// none. The FIXTURE counts are over everything signed.
struct FixtureCensus {
    fixtures: usize,
    diagnostic_fixtures: usize,
    gate_rows_early: usize,
    gate_rows_late: usize,
}

/// Sign every side of every registered pair — §5.3's first input.
///
/// Every side, not only the gate sides: a diagnostic side colliding with a gate
/// side is a reading worth having, and excluding it would hide it. What the
/// role decides is BLOCKING, not membership.
fn sign_fixture_package(rules: &Rules) -> Result<(Vec<SignedView>, FixtureCensus), String> {
    let mut views = Vec::new();
    let mut census =
        FixtureCensus { fixtures: 0, diagnostic_fixtures: 0, gate_rows_early: 0, gate_rows_late: 0 };
    for pair in REGISTERED.iter() {
        let gate = matches!(pair.role, PairRole::Constraint);
        for stage in pair.stages {
            let fixture = build_registered_pair(pair, *stage, rules)
                .map_err(|e| format!("{} at {}: {e:?}", pair.id, stage.name()))?;
            census.fixtures += 1;
            if gate {
                match stage {
                    StageEndpoint::Early => census.gate_rows_early += 1,
                    StageEndpoint::Late => census.gate_rows_late += 1,
                }
            } else {
                census.diagnostic_fixtures += 1;
            }
            for side in [Side::Greater, Side::Lesser] {
                let view = fixture.view(side);
                if view_is_terminal(view) {
                    return Err(format!(
                        "{} {} {}: §5.3's domain is NON-TERMINAL views and this side is terminal",
                        pair.id,
                        stage.name(),
                        side.name(),
                    ));
                }
                views.push(SignedView {
                    sig: signature(view, fixture.subject(), rules)
                        .map_err(|e| format!("{} {}: {e}", pair.id, side.name()))?,
                    from: Provenance::PairSide {
                        pair: pair.id.to_string(),
                        stage: stage.name(),
                        side: side.name(),
                        gate,
                    },
                });
            }
        }
    }
    Ok((views, census))
}

/// **M-PF** — spec §6's `placement-fork-same-count` master, drawn from OS
/// entropy on 2026-08-21 and RECORDED in the execution ledger's step-2 row
/// BEFORE first use. Named here so this instrument reproduces the registered
/// development ordinals and DRAWS NOTHING; §5.4's own sentence is "the solver
/// and collision test draw nothing".
const M_PF: u64 = 0x87FE_F1D5_16F6_B4E9;

/// §6's development band. Ordinals 4..8 are the extension (undrawn — no supply
/// miss) and 8..16 are the ESCROW band, fenced until step 10.
const M_PF_DEVELOPMENT: std::ops::Range<u32> = 0..4;

/// The FIRED-WHILE-BEHIND root: `gen:end-firable-behind-in-truth:…:11`, the
/// step-2 row's READ-AND-RECORD identity (digest `392fe6571d0aa82a`).
///
/// Its predicate is a PHASE-C predicate, not one of §6's two banded ones, so no
/// ordinal band and no burn rule reach it — and its confirmatory block was
/// already read at the Phase-C close. It rides Part A as RECORDED, NEVER
/// GATING: no §5.3 blocking arm names it, because nothing registers a pair
/// there.
const M_FIRED_WHILE_BEHIND: u64 = 0xA3E1_2026_0818_0006;
const FIRED_WHILE_BEHIND_ORDINAL: u32 = 11;

/// One root whose non-terminal successors Part A signs.
struct RootInput {
    name: String,
    origin: String,
    digest: String,
    state: GameState,
    /// §6's frozen recipe at this root, or the typed reason there is none.
    /// A miss is not a failure: it says this root registers no successor pair,
    /// so nothing here can block.
    recipe: Result<RecipePair, RecipeMiss>,
    /// Whether a blocking finding at this root would be a §5.3 STOP. The
    /// fired-while-behind root is recorded and never gating.
    gating: bool,
    note: &'static str,
}

/// One named successor set of one root, signed.
struct SuccessorSet {
    name: &'static str,
    reading: &'static str,
    path: String,
    enumerated: usize,
    terminal: usize,
    views: Vec<SignedView>,
}

/// Sign the NON-TERMINAL successors of one root, from the ROOT MOVER's seat.
///
/// The mover is the seat on the move AT THE ROOT and stays the mover across
/// every set: §5.3 signs "from the MOVER's view", and a signature that switched
/// seats mid-set would compare vectors drawn in two different currencies.
///
/// # Why a root can have TWO sets, and why this carries both
///
/// §5.3's inputs name "the exhibit-class roots' non-terminal successor sets
/// (g1-e113's 139 placements; the §6 development root tables)". At the exhibit
/// roots the two phrases coincide: g1-e113's own legal list IS 139 placements,
/// which this instrument reproduces exactly. At a §6 PLACEMENT-FORK root they
/// do not, and the difference is not cosmetic — the fork roots sit at
/// `AdditionalActionFirst`, where the legal list is the additional-action
/// options (one or two moves) and the fork's placements are not legal until
/// `EndAdditionalPhase` advances the phase. That matters because the ROOT half
/// of §5.3's blocking rule names the registered development successor pair, and
/// §6's frozen recipe pair is a pair of PLACEMENTS: under the literal
/// legal-list reading that arm could never fire at any development root, and a
/// clean "zero blocking" would be reporting the enumeration rather than the
/// columns.
///
/// So both are signed and both are labelled — `root` for the literal legal
/// list, `fork` for the placement set the predicate, the step-5 gate ("the
/// fork's same-count set") and the recipe all speak about. The reading is
/// therefore RECORDED rather than chosen. Both sets go through the CHECKED
/// `apply_move` at every step; the fork set reaches its states the way play
/// reaches them, by taking `EndAdditionalPhase` first.
fn sign_root_sets(root: &RootInput, rules: &Rules) -> Result<Vec<SuccessorSet>, String> {
    let mover = root.state.current_player;
    let mut sets = Vec::new();

    let moves = get_legal_moves(&root.state, rules);
    sets.push(sign_one_set(
        root,
        "root",
        "the root's own legal list",
        format!("get_legal_moves(root) at {:?}", root.state.turn_phase),
        &root.state,
        &moves,
        mover,
        rules,
    )?);

    // The fork set exists exactly where the frozen recipe found a turn start to
    // classify: the same `EndAdditionalPhase`-then-enumerate walk, taken here
    // through the checked boundary.
    if root.recipe.is_ok() && moves.contains(&Move::EndAdditionalPhase) {
        let placing = apply_move(&root.state, &Move::EndAdditionalPhase, rules)
            .map_err(|e| format!("{}: EndAdditionalPhase did not apply: {e}", root.name))?;
        let placements = get_legal_moves(&placing, rules);
        sets.push(sign_one_set(
            root,
            "fork",
            "§6's fork placement set (the step-5 gate's \"same-count set\")",
            format!("EndAdditionalPhase then get_legal_moves at {:?}", placing.turn_phase),
            &placing,
            &placements,
            mover,
            rules,
        )?);
    }
    Ok(sets)
}

#[allow(clippy::too_many_arguments)]
fn sign_one_set(
    root: &RootInput,
    name: &'static str,
    reading: &'static str,
    path: String,
    from_state: &GameState,
    moves: &[Move],
    mover: PlayerId,
    rules: &Rules,
) -> Result<SuccessorSet, String> {
    let mut terminal = 0usize;
    let mut views = Vec::new();
    for (ordinal, mv) in moves.iter().enumerate() {
        let next = apply_move(from_state, mv, rules)
            .map_err(|e| format!("{}/{name}: successor {ordinal} did not apply: {e}", root.name))?;
        let view = observe(&next, mover)
            .map_err(|e| format!("{}/{name}: successor {ordinal} unobservable: {e}", root.name))?;
        if view_is_terminal(&view) {
            terminal += 1;
            continue;
        }
        let role = match &root.recipe {
            Ok(pair) if pair.productive == *mv => RecipeRole::Productive,
            Ok(pair) if pair.dead == *mv => RecipeRole::Dead,
            _ => RecipeRole::Unregistered,
        };
        views.push(SignedView {
            sig: signature(&view, mover, rules)
                .map_err(|e| format!("{}/{name}: successor {ordinal}: {e}", root.name))?,
            from: Provenance::Successor {
                root: root.name.clone(),
                set: name,
                ordinal,
                mv: format!("{mv:?}"),
                role,
            },
        });
    }
    Ok(SuccessorSet { name, reading, path, enumerated: moves.len(), terminal, views })
}

/// Resolve the §6 root set Part A runs over, as the flag-1 ruling left it.
///
/// **What is IN**: the two exhibit-class registry roots and the four
/// `placement-fork-same-count` development ordinals at the recorded M-PF, all
/// GATING; plus the fired-while-behind root, RECORDED and not gating.
///
/// **What is OUT and why**: `opponent-sitting-executable`'s generated bands are
/// STRUCK by the owner's flag-1 ruling (2026-08-22) and M-OSE is closed
/// permanently undrawn, so there is no OSE ordinal to resolve; its
/// re-scoped exhibit — the game-3 opponent-pending recorded root — is NOT YET
/// RESOLVED, and Part A names it pending rather than quietly running without
/// it. The escrow band {8..16} is unread by construction: this function asks
/// for `M_PF_DEVELOPMENT` and nothing else.
fn resolve_part_a_roots(generated: bool, rules: &Rules) -> Result<Vec<RootInput>, String> {
    let mut out = Vec::new();
    let mut push = |name: String,
                    origin: String,
                    source: &RootSource,
                    gating: bool,
                    note: &'static str|
     -> Result<(), String> {
        let (state, descriptor) =
            source.resolve(rules).map_err(|e| format!("{name}: {e:?}"))?;
        let recipe = escrow_recipe_pair(&state, rules);
        out.push(RootInput {
            name,
            origin,
            digest: descriptor.root_digest,
            state,
            recipe,
            gating,
            note,
        });
        Ok(())
    };

    for wanted in ["g1-e113-tiemass", "g3-e93-separated"] {
        let entry = registry()
            .into_iter()
            .find(|e| e.name == wanted)
            .ok_or_else(|| format!("{wanted} is not a registry root"))?;
        // A missing owner-game save is a TYPED MISS naming the file, never a
        // silently shorter run: the registry roots are two of §5.3's named
        // inputs and a pass computed without them is a pass about less.
        let source = entry.load(rules).map_err(|e| {
            format!("{wanted}: {e:?} (the owner-game saves are gitignored; absence is a miss)")
        })?;
        push(
            entry.name.to_string(),
            format!("{}:{}", entry.path, entry.event_index),
            &source,
            true,
            entry.note,
        )?;
    }

    if generated {
        for ordinal in M_PF_DEVELOPMENT {
            let source = RootSource::Generated {
                predicate: RootPredicate::PlacementForkSameCount,
                master: M_PF,
                ordinal,
            };
            push(
                format!("m-pf-dev-{ordinal}"),
                format!("gen:placement-fork-same-count:{M_PF:#018X}:{ordinal}"),
                &source,
                true,
                "a §6 development ordinal at the recorded M-PF",
            )?;
        }
        let source = RootSource::Generated {
            predicate: RootPredicate::EndFirableBehindInTruth,
            master: M_FIRED_WHILE_BEHIND,
            ordinal: FIRED_WHILE_BEHIND_ORDINAL,
        };
        push(
            "fired-while-behind".to_string(),
            format!(
                "gen:end-firable-behind-in-truth:{M_FIRED_WHILE_BEHIND:#018X}:\
                 {FIRED_WHILE_BEHIND_ORDINAL}"
            ),
            &source,
            false,
            "READ-AND-RECORD ONLY, never a gate (step-2 row)",
        )?;
    }
    Ok(out)
}

/// **THE e113 TOP-TIE MEASUREMENT** — §5.3's own step-4 assignment.
///
/// The D7 headline root is a 36-way tie in the SHIPPED evaluator's sixteen
/// features. This asks the question the tie poses to a WIDER value model: over
/// exactly those tied candidates, how many distinct 34-feature signatures are
/// there? One class would mean the eighteen candidate columns see nothing the
/// sixteen missed at this position; thirty-six would mean the tie is an
/// artifact of the incumbent width alone.
///
/// It is a MEASUREMENT and not a gate: the tied set is not a registered pair,
/// so a collapse here is organic by §5.3's own rule.
fn e113_top_tie(rules: &Rules) -> Result<String, String> {
    // THE SAME loader and the SAME tie the step-5 gate's dead side is drawn
    // from ([`e113_root`]) — two copies of "call `root_table`, take the top
    // value, filter" is the shape that lets a measurement and a gate quietly
    // describe two different sets.
    let root = e113_root(rules)?;
    let (state, tied, top) = (&root.state, &root.tie, root.top_value);

    let mover = state.current_player;
    let mut classes: BTreeMap<Signature, Vec<String>> = BTreeMap::new();
    for mv in tied {
        let next = apply_move(state, mv, rules).map_err(|e| format!("e113 tie: {e}"))?;
        let view = observe(&next, mover).map_err(|e| format!("e113 tie: {e}"))?;
        let sig = signature(&view, mover, rules)?;
        classes.entry(sig).or_default().push(format!("{mv:?}"));
    }

    let mut out = String::new();
    out.push_str(&format!("root digest         {}\n", root.digest));
    out.push_str(&format!("contestant          {}\n", root.echo));
    out.push_str(&format!("decision seed       {:#018x}\n", root.decision_seed));
    out.push_str(&format!("candidates          {}\n", root.candidates));
    out.push_str(&format!("top value           {top}\n"));
    out.push_str(&format!("tied at top         {}\n", tied.len()));
    out.push_str(&format!("distinct signatures {} over the tied set\n", classes.len()));
    let mut sizes: Vec<usize> = classes.values().map(Vec::len).collect();
    sizes.sort_unstable();
    out.push_str(&format!("  class sizes       {sizes:?}\n"));

    // The reading, stated in the register the result earns and NOT further:
    // the sentence is derived from the two counts, so a future run that
    // separated fewer candidates cannot keep printing the strong claim.
    out.push_str("\nREADING             ");
    if classes.len() == tied.len() {
        out.push_str(&format!(
            "the {}-way tie the SHIPPED sixteen features cannot break AT\n\
             \x20                   ALL resolves into {} DISTINCT complete-vector signatures —\n\
             \x20                   one class per tied candidate. The widened axis separates\n\
             \x20                   exactly where the incumbent evaluator is blind. This is the\n\
             \x20                   SIGHTED-EVALUATOR THESIS measured at the instrument level\n\
             \x20                   rather than argued: not that the 34 columns differ from the\n\
             \x20                   16, but that they carry information the 16 do not have.\n",
            tied.len(),
            classes.len(),
        ));
    } else {
        out.push_str(&format!(
            "the {}-way incumbent tie resolves into {} complete-vector\n\
             \x20                   signatures — a PARTIAL separation. {} tied candidates remain\n\
             \x20                   indistinguishable on the full 34-column axis; the classes\n\
             \x20                   above name them.\n",
            tied.len(),
            classes.len(),
            tied.len() - classes.len(),
        ));
    }
    out.push_str(
        "                    It is a MEASUREMENT, not a gate: the tied set is not a registered\n\
         \x20                   pair, so a collapse here would be ORGANIC by §5.3's own rule.\n",
    );
    for (i, (sig, members)) in classes.iter().enumerate() {
        out.push_str(&format!("  class {i:<3}         size {}\n", members.len()));
        out.push_str(&format!("    dv              {}\n", render_dv(&sig.dv)));
        for member in members {
            out.push_str(&format!("    member          {member}\n"));
        }
    }
    Ok(out)
}

fn cmd_collision(args: &[String]) -> Result<(), String> {
    let (positional, flags, _) =
        split_args(args, &[], &["fixtures-only", "no-generated", "no-tie"], &[])?;
    if !positional.is_empty() {
        return Err(format!("collision takes no positional arguments\n\n{USAGE}"));
    }
    let fixtures_only = flags.contains_key("fixtures-only");
    let generated = !flags.contains_key("no-generated");

    let rules = Rules::standard();
    let started = Instant::now();
    println!("=== eval_probe collision [semantics rev {SEMANTICS_REV}] ===");
    println!("{}", PART_A_HEADER.trim_end());
    println!();
    println!("package digest      {}", package_digest());
    println!("axis width          {AXIS_WIDTH}");
    println!("profile             {}", profile());

    let (mut views, census) = sign_fixture_package(&rules)?;
    println!();
    println!("--- inputs: the frozen fixture package ---");
    println!("fixtures            {} (pair, stage) realizations", census.fixtures);
    println!("sides signed        {}", census.fixtures * 2);
    println!(
        "  of which          {} diagnostic sides (2(c), 10(c): RECORDED, never a gate — signed \
         but never blocking)",
        census.diagnostic_fixtures * 2,
    );
    println!(
        "gate row census     REALIZED early {} · late {}   FROZEN §5.4 early 25 · late 27",
        census.gate_rows_early, census.gate_rows_late,
    );
    println!(
        "                    the early delta is the 4(c) LATE-only RULING's footprint (owner, \
         2026-08-22, guidingpath) — a recorded ruling, never a defect"
    );

    // --- the roots ----------------------------------------------------------
    println!();
    println!("--- inputs: the root successor sets ---");
    if fixtures_only {
        println!("roots               SKIPPED (--fixtures-only)");
    } else {
        for root in resolve_part_a_roots(generated, &rules)? {
            println!();
            println!("root                {}", root.name);
            println!("  origin            {}", root.origin);
            println!("  digest            {}", root.digest);
            println!("  mover             seat {}", root.state.current_player.0);
            println!("  phase             {:?}", root.state.turn_phase);
            match &root.recipe {
                Ok(pair) => println!(
                    "  registered pair   total {} · productive {:?} (reward {}) · dead {:?}",
                    pair.total, pair.productive_areas, pair.productive_reward, pair.dead_areas,
                ),
                Err(miss) => println!(
                    "  registered pair   none ({miss:?}) — no blocking-eligible successor pair here"
                ),
            }
            println!("  gating            {}", if root.gating { "yes" } else { "no — recorded" });
            println!("  note              {}", root.note);
            for set in sign_root_sets(&root, &rules)? {
                let operative = if set.name == "fork" {
                    "§5.3-OPERATIVE here"
                } else if root.recipe.is_ok() {
                    "completeness courtesy"
                } else {
                    "§5.3-OPERATIVE here"
                };
                println!(
                    "  set {:<5}         {} enumerated · {} signed (terminal {}) — {} \
                     [{operative}]",
                    set.name,
                    set.enumerated,
                    set.views.len(),
                    set.terminal,
                    set.reading,
                );
                println!("    path            {}", set.path);
                let registered: Vec<&str> = set
                    .views
                    .iter()
                    .filter_map(|v| match &v.from {
                        Provenance::Successor { role: RecipeRole::Productive, .. } => {
                            Some("productive")
                        }
                        Provenance::Successor { role: RecipeRole::Dead, .. } => Some("dead"),
                        _ => None,
                    })
                    .collect();
                println!(
                    "    registered      {}",
                    if registered.is_empty() {
                        "neither recipe side is in this set".to_string()
                    } else {
                        registered.join(" + ")
                    },
                );
                views.extend(set.views);
            }
        }
        if !generated {
            println!();
            println!("generated roots     SKIPPED (--no-generated)");
        }
    }
    println!();
    println!("{}", NOT_IN_SCOPE.trim_end());

    // --- the classification -------------------------------------------------
    let part_a = classify(views);
    println!();
    println!("--- §5.3 Part A ---");
    print!("{}", part_a.render());

    // --- the e113 measurement ----------------------------------------------
    if !fixtures_only && !flags.contains_key("no-tie") {
        println!();
        println!("--- the e113 top tie, complete-vector (a step-4 MEASUREMENT, not a gate) ---");
        print!("{}", e113_top_tie(&rules)?);
    }

    println!();
    println!("wall                {:.1}s", started.elapsed().as_secs_f64());
    let blocking = part_a.blocking();
    if blocking.is_empty() {
        println!("VERDICT             zero blocking collisions (§5.3's necessary condition holds)");
        Ok(())
    } else {
        Err(format!(
            "STOP — {} BLOCKING COLLISION(S): {}\n\nThe ★ fork at checkpoint step 4 is OWNER \
             territory: repair-by-scope-expansion is fenced (checkpoint §6) and this instrument \
             reports rather than repairs.",
            blocking.len(),
            blocking.iter().map(Blocking::label).collect::<Vec<_>>().join("; "),
        ))
    }
}

/// The reading this instrument implements, printed at the top of every artifact
/// so a figure quoted out of it carries its own semantics.
const PART_A_HEADER: &str = "\
§5.3 signature      (exact progress_ppm, Δv over the 34 features) of ONE view
Δv                  the MOVER-RELATIVE vector `mine − theirs`, per axis slot, 2P
seat convention     the view is the MOVER's (masked as the mover sees it) AND the
                    differencing is signed from the mover. Six candidate rows —
                    O1 O2 O3 O5 O6 X3 — do NOT negate for the other seat, so an
                    opponent-side signature is not this one with flipped signs
domain              non-terminal views only
a collision         two views with an IDENTICAL (ppm, 34-vector): they score
                    equal under EVERY weight vector, so no fit can separate them
near-collision      equal Δv at DIFFERENT ppm — reported, never blocking, because
                    the two interpolate to different weight vectors
blocking rule       a class blocks iff it contains two sides of ONE registered
                    GATE pair, or ONE root's registered development successor
                    pair (§6's frozen recipe), or — step 10 only — a
                    recipe-instantiated escrow pair. Everything else that shares
                    a signature is ORGANIC: a diagnostic with a class size

TWO ADJUDICATIONS this run rests on, both RATIFIED (team lead, 2026-08-23)

diagnostics         2(c) and 10(c) are DIAGNOSTIC registrations. They yield no
                    solve row, so a collision between their sides cannot make one
                    infeasible: they are SIGNED and REPORTED here, never blocking.
                    Three texts concur — §5.3's input list breaks family 10 open
                    to \"10(a)/(b)/(d)\" precisely to exclude 10(c);
                    `PairRole::Constraint` is documented as \"a solve row and a
                    blocking side\" against `Diagnostic`'s \"RECORDED, never a
                    gate\"; and §5.4's 34 row-yielding registrations are the
                    registry's 36 minus exactly those two. 10(c)'s sides ARE
                    signature-identical, BY REGISTERED CONSTRUCTION — step 2
                    proved its \"+3 maximum\" from the fact that an all-rows-equal
                    pair carries no more than 3 residual faith — so the class this
                    run reports there is the construction working, not a defect

successor sets      a §6 placement-fork root carries TWO, both signed and
                    labelled. `fork` is the §5.3-OPERATIVE set at those roots: the
                    registered development successor pair is a pair of PLACEMENTS,
                    which are not legal until `EndAdditionalPhase` advances the
                    phase, so the fork set is where the blocking rule has teeth.
                    `root` — the literal legal list, one or two additional-action
                    options — is a recorded completeness courtesy. At g1-e113 the
                    two COINCIDE: its own legal list IS 139 placements, matching
                    §5.3's own words, which is the confirmation the split is
                    faithful rather than inventive";

/// What §5.3 names as an input and this run does NOT carry, said out loud.
const NOT_IN_SCOPE: &str = "\
--- named, not carried ---
OSE generated bands STRUCK by the owner's flag-1 ruling (2026-08-22); M-OSE closed
                    permanently undrawn, so there is no ordinal to sign
game-3 OSE exhibit  NOT YET A READ ROOT, and deliberately so. The game-3 lane closed
                    (run_id bga-import/896199544, 234 events, terminal digest
                    3a9b2c6d7206f3e2) with THIRTEEN opponent-PENDING exhibit
                    identities recorded — events 34, 50, 58, 68, 78, 94, 111, 127,
                    144, 159, 168, 184, 198 — and with the census finding that the
                    STRICT opponent-sitting-executable clause has ZERO members in
                    792 non-terminal positions across all three complete human
                    games. WHICH pending exhibit(s) become read roots is step-5
                    charter work, batch-level pre-registered; Part A over the
                    chosen root's successor set is OWED then, and this run does not
                    presume the choice
escrow band {8..16} UNREAD by construction (§6's burn rule): this run asks for the
                    development ordinals {0..4} and nothing above";

// ---------------------------------------------------------------------------
// §5.5 — the collinearity diagnostic, before any fit
// ---------------------------------------------------------------------------

/// The sufficient statistics for one bucket of §5.5's matrix.
///
/// Sums and cross-products rather than retained rows: the corpus is ~160,000
/// (snapshot × seat) observations of 34 columns and every figure §5.5 asks for
/// — correlations, VIF, per-column ranges — is a function of `n`, `Σx`, `Σxx'`
/// and the extrema. Keeping the rows would buy nothing and cost 40MB per
/// bucket.
///
/// TWO CURRENCIES are accumulated. The Gram matrix is RELATIVE (`mine −
/// theirs`), because that is the vector a weight multiplies and therefore the
/// one whose collinearity threatens a fit. The RANGES are kept in both, because
/// the question the ranges answer — "what scale is this column on, next to the
/// box |w| ≤ 64?" — is asked of the raw column as often as of the differenced
/// one, and step 3's handover recorded the units as "wildly different" without
/// measuring them.
#[derive(Clone)]
struct ColumnStats {
    n: u64,
    /// `Σx` in the relative currency.
    sum: Vec<f64>,
    /// `Σ x_i x_j` for `i ≤ j`, relative, row-major over the upper triangle.
    gram: Vec<f64>,
    rel_min: Vec<i64>,
    rel_max: Vec<i64>,
    own_sum: Vec<f64>,
    own_sumsq: Vec<f64>,
    own_min: Vec<i64>,
    own_max: Vec<i64>,
}

/// Index into the packed upper triangle, `i ≤ j`.
fn tri(i: usize, j: usize) -> usize {
    let (i, j) = if i <= j { (i, j) } else { (j, i) };
    i * AXIS_WIDTH - i * (i + 1) / 2 + j
}

impl ColumnStats {
    fn new() -> Self {
        Self {
            n: 0,
            sum: vec![0.0; AXIS_WIDTH],
            gram: vec![0.0; AXIS_WIDTH * (AXIS_WIDTH + 1) / 2],
            rel_min: vec![i64::MAX; AXIS_WIDTH],
            rel_max: vec![i64::MIN; AXIS_WIDTH],
            own_sum: vec![0.0; AXIS_WIDTH],
            own_sumsq: vec![0.0; AXIS_WIDTH],
            own_min: vec![i64::MAX; AXIS_WIDTH],
            own_max: vec![i64::MIN; AXIS_WIDTH],
        }
    }

    fn observe(&mut self, rel: &[i64; AXIS_WIDTH], own: &[i64; AXIS_WIDTH]) {
        self.n += 1;
        for i in 0..AXIS_WIDTH {
            let xi = rel[i] as f64;
            self.sum[i] += xi;
            self.rel_min[i] = self.rel_min[i].min(rel[i]);
            self.rel_max[i] = self.rel_max[i].max(rel[i]);
            for (j, value) in rel.iter().enumerate().skip(i) {
                self.gram[tri(i, j)] += xi * *value as f64;
            }
            let oi = own[i] as f64;
            self.own_sum[i] += oi;
            self.own_sumsq[i] += oi * oi;
            self.own_min[i] = self.own_min[i].min(own[i]);
            self.own_max[i] = self.own_max[i].max(own[i]);
        }
    }

    fn merge(&mut self, other: &Self) {
        self.n += other.n;
        for i in 0..AXIS_WIDTH {
            self.sum[i] += other.sum[i];
            self.rel_min[i] = self.rel_min[i].min(other.rel_min[i]);
            self.rel_max[i] = self.rel_max[i].max(other.rel_max[i]);
            self.own_sum[i] += other.own_sum[i];
            self.own_sumsq[i] += other.own_sumsq[i];
            self.own_min[i] = self.own_min[i].min(other.own_min[i]);
            self.own_max[i] = self.own_max[i].max(other.own_max[i]);
        }
        for (target, value) in self.gram.iter_mut().zip(other.gram.iter()) {
            *target += value;
        }
    }

    fn mean(&self, i: usize) -> f64 {
        if self.n == 0 {
            return 0.0;
        }
        self.sum[i] / self.n as f64
    }

    /// Population covariance in the relative currency.
    fn cov(&self, i: usize, j: usize) -> f64 {
        if self.n == 0 {
            return 0.0;
        }
        self.gram[tri(i, j)] / self.n as f64 - self.mean(i) * self.mean(j)
    }

    fn sd(&self, i: usize) -> f64 {
        self.cov(i, i).max(0.0).sqrt()
    }

    /// `None` where either column never moves — a constant column has no
    /// correlation, and reporting one as 0 would read as "measured independent"
    /// rather than "not measurable".
    fn corr(&self, i: usize, j: usize) -> Option<f64> {
        let (si, sj) = (self.sd(i), self.sd(j));
        if si <= COLUMN_MOVES || sj <= COLUMN_MOVES {
            return None;
        }
        Some((self.cov(i, j) / (si * sj)).clamp(-1.0, 1.0))
    }

    fn own_mean(&self, i: usize) -> f64 {
        if self.n == 0 {
            return 0.0;
        }
        self.own_sum[i] / self.n as f64
    }

    fn own_sd(&self, i: usize) -> f64 {
        if self.n == 0 {
            return 0.0;
        }
        let m = self.own_mean(i);
        (self.own_sumsq[i] / self.n as f64 - m * m).max(0.0).sqrt()
    }
}

/// A column below this standard deviation is CONSTANT for §5.5's purposes.
/// The columns are integer-valued, so any real variation is ≥ 1/n away from
/// zero and this cannot swallow a moving column.
const COLUMN_MOVES: f64 = 1e-9;

/// A pivot below this in the Gauss-Jordan elimination is an EXACT dependency.
const PIVOT_FLOOR: f64 = 1e-9;

/// §5.5's per-candidate VIF, plus the two ways a column can fail to have one.
struct VifTable {
    /// Columns that never move — no correlation, no VIF, excluded from the
    /// solve rather than ridged into it.
    constant: Vec<usize>,
    /// Columns EXACTLY dependent on the columns before them. Reported by name:
    /// `1/(1 − R²)` at `R² = 1` is not a large number, it is not a number, and
    /// a ridged inverse would print a plausible VIF for a column carrying no
    /// independent information at all.
    dependent: Vec<usize>,
    /// The diagonal of the inverted correlation matrix, over the columns that
    /// have one.
    vif: BTreeMap<usize, f64>,
}

/// Each active column's VIF against every other active column.
///
/// Gauss-Jordan on the correlation matrix with partial pivoting; the inverse's
/// diagonal IS the VIF vector (`(R⁻¹)_ii = 1/(1 − R²_i)`), which is why this
/// computes one inverse rather than 34 regressions. A column whose pivot
/// collapses is dropped from the active set and re-listed as `dependent`, and
/// the remaining columns' VIFs are computed on the submatrix that is left —
/// so one exact dependency does not erase the whole reading.
fn vif_table(stats: &ColumnStats) -> VifTable {
    let mut constant = Vec::new();
    let mut active = Vec::new();
    for i in 0..AXIS_WIDTH {
        if stats.sd(i) <= COLUMN_MOVES {
            constant.push(i);
        } else {
            active.push(i);
        }
    }

    let mut dependent = Vec::new();
    loop {
        let m = active.len();
        if m == 0 {
            return VifTable { constant, dependent, vif: BTreeMap::new() };
        }
        let mut r = vec![vec![0.0f64; m]; m];
        for (a, &i) in active.iter().enumerate() {
            for (b, &j) in active.iter().enumerate() {
                r[a][b] = stats.corr(i, j).unwrap_or(0.0);
            }
        }
        match invert_symmetric(&r) {
            Ok(inverse) => {
                let vif = active
                    .iter()
                    .enumerate()
                    .map(|(a, &i)| (i, inverse[a][a]))
                    .collect::<BTreeMap<_, _>>();
                return VifTable { constant, dependent, vif };
            }
            Err(collapsed) => {
                // The elimination names the LATER member of the dependency (it
                // is the one whose pivot has nothing left to stand on), which
                // is the honest report: the earlier columns still carry the
                // information, and this one adds none.
                dependent.push(active[collapsed]);
                dependent.sort_unstable();
                active.remove(collapsed);
            }
        }
    }
}

/// Invert a correlation matrix by Gauss-Jordan with partial pivoting.
///
/// `Err(k)` names the ACTIVE-SET index whose column had no usable pivot — an
/// exact linear dependency on the columns eliminated before it.
fn invert_symmetric(r: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, usize> {
    let m = r.len();
    let mut a = r.to_vec();
    let mut inv: Vec<Vec<f64>> = (0..m)
        .map(|i| (0..m).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
        .collect();

    for col in 0..m {
        let (pivot_row, magnitude) = (col..m)
            .map(|row| (row, a[row][col].abs()))
            .max_by(|x, y| x.1.total_cmp(&y.1))
            .expect("the range is non-empty");
        if magnitude <= PIVOT_FLOOR {
            return Err(col);
        }
        a.swap(col, pivot_row);
        inv.swap(col, pivot_row);
        let pivot = a[col][col];
        for j in 0..m {
            a[col][j] /= pivot;
            inv[col][j] /= pivot;
        }
        for row in 0..m {
            if row == col {
                continue;
            }
            let factor = a[row][col];
            if factor == 0.0 {
                continue;
            }
            for j in 0..m {
                a[row][j] -= factor * a[col][j];
                inv[row][j] -= factor * inv[col][j];
            }
        }
    }
    Ok(inv)
}

/// §2's DECLARED clusters — the list §5.5 says its report is aimed at, written
/// in the axis's own labels so a spelling that stops resolving is an error
/// rather than a silently missing row.
const DECLARED_CLUSTERS: &[(&str, &[&str])] = &[
    ("E1 ↔ the Money10 leg", &["E1", "achievement_progress_sum"]),
    ("E2 ↔ O2 (aspect-split, O2 ≤ E2)", &["E2", "O2"]),
    ("O1 ↔ HandSize", &["O1", "hand_size"]),
    ("{S1, S3} (2P: four production tiles)", &["S1", "S3"]),
    ("{S1, S2, S5, S6} (shared presence index)", &["S1", "S2", "S5", "S6"]),
    ("{S4, S7}", &["S4", "S7"]),
    ("{O2, X2, E2}", &["O2", "X2", "E2"]),
    ("{X1, E1, ImportedGoods}", &["X1", "E1", "imported_goods"]),
    ("{X3, S1, O2} (matched access reduces to its parents)", &["X3", "S1", "O2"]),
    ("{E3, S6} (the retrieval chain's two stages)", &["E3", "S6"]),
    ("E1 ↓ vs TradingHousesBuilt ↑ over a game arc", &["E1", "trading_houses_built"]),
];

/// §8's rule: a `(0, 0)` column is COLLINEAR rather than vacuous at either of
/// these, and is NOT removed.
const COLLINEAR_CORR: f64 = 0.9;
const COLLINEAR_VIF: f64 = 10.0;

/// A label's axis slot, or an error naming the spelling that did not resolve.
fn slot_named(label: &str) -> Result<usize, String> {
    FeatureId::all()
        .iter()
        .find(|slot| slot.label() == label)
        .map(|slot| slot.index())
        .ok_or_else(|| format!("`{label}` is not an axis label"))
}

/// §5.5's buckets: stage tercile × the Category-B conditioning.
fn bucket_of(stage: Stage, money10: bool) -> usize {
    stage.index() * 2 + usize::from(money10)
}

fn cmd_collinearity(args: &[String]) -> Result<(), String> {
    let (positional, flags, _) = split_args(
        args,
        &["contestant", "weights", "master"],
        &["os-entropy", "invariants", "no-invariants"],
        &[],
    )?;
    if positional.len() != 2 {
        return Err(format!("collinearity needs <players> <games>\n\n{USAGE}"));
    }
    let players: u8 = positional[0].parse().map_err(|_| "players must be a number".to_string())?;
    // 2P ONLY, and for §5.3's reason: the relative currency needs exactly one
    // opponent. At 3–4P "theirs" is `opponent_extreme`, which orients BY WEIGHT
    // — a collinearity diagnostic computed on it would be a diagnostic about
    // the incumbent weights, which is not what "before any fit" means.
    if players != 2 {
        return Err(format!(
            "collinearity is 2P only (the relative currency needs one opponent), got {players}"
        ));
    }
    let games: u32 = positional[1].parse().map_err(|_| "games must be a number".to_string())?;
    let contestant = ProbeContestant::parse(
        flags.get("contestant").map(String::as_str).unwrap_or(LOOKAHEAD_CONTROLLER),
    )?;
    let (weights, weights_name) = weights_from(&flags)?;
    let master = master_from(&flags)?;
    let invariants = !flags.contains_key("no-invariants");

    let rules = Rules::standard();
    let (mut schedule, master_value) = match master {
        SeedScheduleKind::Diagnostic(m) => (SeedSchedule::diagnostic(m), m),
        SeedScheduleKind::OsEntropy => SeedSchedule::from_os_entropy(),
    };

    let started = Instant::now();
    let mut buckets: Vec<ColumnStats> = vec![ColumnStats::new(); STAGES.len() * 2];
    let mut money10_games = 0u32;
    for game in 0..games {
        let seed = schedule.next_world();
        let (stats, _) =
            play_one(seed, players, contestant, &weights, &rules, master_value, invariants)?;
        let starts = turn_start_states(&stats, &rules, game)?;
        let turns = starts.len();
        let mut counted = false;
        for (index, start) in starts.iter().enumerate() {
            let stage = stage_of(index, turns);
            for seat in 0..players {
                let view = observe(start, PlayerId(seat))
                    .map_err(|e| format!("observe game {game} turn {index} seat {seat}: {e}"))?;
                let money10 = view
                    .achievements
                    .iter()
                    .any(|c| c.achievement_type == AchievementType::Money10);
                if !counted {
                    counted = true;
                    money10_games += u32::from(money10);
                }
                let signed = signature(&view, PlayerId(seat), &rules)
                    .map_err(|e| format!("game {game} turn {index} seat {seat}: {e}"))?;
                let own = own_vector(&view, PlayerId(seat), &rules)?;
                buckets[bucket_of(stage, money10)].observe(&signed.dv, &own);
            }
        }
    }

    let mut pooled = ColumnStats::new();
    for bucket in &buckets {
        pooled.merge(bucket);
    }
    let mut by_stage: Vec<ColumnStats> = Vec::new();
    for stage in STAGES {
        let mut acc = ColumnStats::new();
        acc.merge(&buckets[bucket_of(stage, false)]);
        acc.merge(&buckets[bucket_of(stage, true)]);
        by_stage.push(acc);
    }
    let mut money10_on = ColumnStats::new();
    let mut money10_off = ColumnStats::new();
    for stage in STAGES {
        money10_on.merge(&buckets[bucket_of(stage, true)]);
        money10_off.merge(&buckets[bucket_of(stage, false)]);
    }

    println!("=== eval_probe collinearity [semantics rev {SEMANTICS_REV}] ===");
    println!("{}", COLLINEARITY_HEADER.trim_end());
    println!();
    println!("contestant          {}", contestant.echo(&weights, weights_name));
    println!("weights             {weights_name} ({:#018x})", weight_table_identity(&weights) as u64);
    println!("players             {players}");
    println!("games               {games}");
    println!("master              {}", master_echo(master));
    println!("invariants          {}", if invariants { "on" } else { "off" });
    println!("observations        {} (turn start × seat)", pooled.n);
    println!(
        "Category-B split    Money10 drawn in {money10_games} of {games} games \
         ({} obs on / {} obs off)",
        money10_on.n, money10_off.n,
    );
    println!("profile             {}", profile());

    print!("{}", render_ranges(&pooled));
    print!("{}", render_correlations(&pooled));
    print!("{}", render_clusters(&pooled, &money10_on, &money10_off)?);
    print!("{}", render_vifs(&pooled, &by_stage));
    print!("{}", render_collinear_flags(&pooled, &by_stage));

    println!();
    println!("wall                {:.1}s", started.elapsed().as_secs_f64());
    Ok(())
}

/// One view's OWN 34-vector from `subject`'s seat — the un-differenced levels,
/// which is the currency the range table's first half reports.
fn own_vector(
    view: &PlayerView,
    subject: PlayerId,
    rules: &Rules,
) -> Result<[i64; AXIS_WIDTH], String> {
    let me = view
        .players
        .iter()
        .find(|p| p.id == subject)
        .ok_or_else(|| format!("seat {} is not a player in this view", subject.0))?;
    let structure = ScoreStructure::derive(view, rules);
    let mut out = [0i64; AXIS_WIDTH];
    for f in Feature::ALL {
        out[FeatureId::Incumbent(f).index()] = structure.value_of(me, f);
    }
    for c in CandidateFeature::ALL {
        out[FeatureId::Candidate(c).index()] = candidate_value(&structure, view, me, c, rules);
    }
    Ok(out)
}

fn render_ranges(stats: &ColumnStats) -> String {
    let mut out = String::new();
    out.push_str("\n--- per-column ranges and scales (pooled) ---\n");
    out.push_str(&format!(
        "{:<4} {:<28} {:>7} {:>7} {:>9} {:>8}   {:>7} {:>7} {:>9} {:>8}\n",
        "row", "feature", "own:min", "own:max", "own:mean", "own:sd", "rel:min", "rel:max",
        "rel:mean", "rel:sd",
    ));
    for slot in FeatureId::all() {
        let i = slot.index();
        out.push_str(&format!(
            "{:<4} {:<28} {:>7} {:>7} {:>9.3} {:>8.3}   {:>7} {:>7} {:>9.3} {:>8.3}\n",
            i,
            slot.label(),
            stats.own_min[i],
            stats.own_max[i],
            stats.own_mean(i),
            stats.own_sd(i),
            stats.rel_min[i],
            stats.rel_max[i],
            stats.mean(i),
            stats.sd(i),
        ));
    }
    out
}

/// Every candidate×candidate and candidate×incumbent pair at or above the
/// reporting floor, strongest first.
fn render_correlations(stats: &ColumnStats) -> String {
    const FLOOR: f64 = 0.7;
    let mut rows: Vec<(f64, String)> = Vec::new();
    for c in CandidateFeature::ALL {
        let i = FeatureId::Candidate(c).index();
        for slot in FeatureId::all() {
            let j = slot.index();
            if j == i || (slot.is_candidate() && j < i) {
                continue;
            }
            if let Some(r) = stats.corr(i, j) {
                if r.abs() >= FLOOR {
                    rows.push((
                        -r.abs(),
                        format!(
                            "  {:>+.3}  {:<28} {:<28} {}",
                            r,
                            c.label(),
                            slot.label(),
                            if slot.is_candidate() { "cand×cand" } else { "cand×incumbent" },
                        ),
                    ));
                }
            }
        }
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut out = String::new();
    out.push_str(&format!("\n--- correlations |r| ≥ {FLOOR} (pooled, relative currency) ---\n"));
    if rows.is_empty() {
        out.push_str("  none\n");
    }
    for (_, line) in rows {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn render_clusters(
    pooled: &ColumnStats,
    money10_on: &ColumnStats,
    money10_off: &ColumnStats,
) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("\n--- §2's DECLARED clusters, realized ---\n");
    for (name, labels) in DECLARED_CLUSTERS {
        out.push_str(&format!("  {name}\n"));
        let slots: Vec<usize> =
            labels.iter().map(|l| slot_named(l)).collect::<Result<Vec<_>, _>>()?;
        for a in 0..slots.len() {
            for b in (a + 1)..slots.len() {
                out.push_str(&format!(
                    "    {:<28} {:<28} {}\n",
                    labels[a],
                    labels[b],
                    match pooled.corr(slots[a], slots[b]) {
                        Some(r) => format!("{r:>+.3}"),
                        None => "  (constant)".to_string(),
                    },
                ));
            }
        }
    }

    // §5.5's named conditioning: the E1/Money10 read POOLED and CONDITIONED.
    let e1 = slot_named("E1")?;
    let progress = slot_named("achievement_progress_sum")?;
    out.push_str("\n--- the E1 / Money10 read, pooled AND conditioned (§2, §5.5) ---\n");
    let render = |label: &str, stats: &ColumnStats| -> String {
        format!(
            "  {label:<28} n {:>8}   corr(E1, achievement_progress_sum) {}\n",
            stats.n,
            match stats.corr(e1, progress) {
                Some(r) => format!("{r:>+.3}"),
                None => "(constant)".to_string(),
            },
        )
    };
    out.push_str(&render("pooled", pooled));
    out.push_str(&render("Money10 DRAWN", money10_on));
    out.push_str(&render("Money10 NOT drawn", money10_off));
    out.push_str(
        "  READ THIS CAREFULLY: the Money10 LEG is not a column of the 34-axis — it is one term\n\
         \x20 inside the incumbent `achievement_progress_sum`, which is the carrier this read uses.\n\
         \x20 §2's exact algebraic identity (E1 IS the leg below 10¥) is therefore visible here as\n\
         \x20 a correlation against the sum that CONTAINS the leg, diluted by the sum's other two\n\
         \x20 cards. The DRAWN/NOT-drawn contrast is the measurement; the pooled figure mixes the\n\
         \x20 games where the leg is not in play at all.\n",
    );
    Ok(out)
}

fn render_vifs(pooled: &ColumnStats, by_stage: &[ColumnStats]) -> String {
    let tables: Vec<VifTable> = by_stage.iter().map(vif_table).collect();
    let pooled_table = vif_table(pooled);

    let mut out = String::new();
    out.push_str("\n--- VIF per candidate against the retained set ---\n");
    out.push_str(&format!(
        "{:<4} {:<28} {:>10} {:>10} {:>10} {:>10}\n",
        "row", "feature", "early", "mid", "late", "pooled",
    ));
    let cell = |table: &VifTable, slot: usize| -> String {
        if table.constant.contains(&slot) {
            "constant".to_string()
        } else if table.dependent.contains(&slot) {
            "DEPENDENT".to_string()
        } else {
            table.vif.get(&slot).map_or_else(|| "-".to_string(), |v| format!("{v:.2}"))
        }
    };
    for c in CandidateFeature::ALL {
        let slot = FeatureId::Candidate(c).index();
        out.push_str(&format!(
            "{:<4} {:<28} {:>10} {:>10} {:>10} {:>10}\n",
            slot,
            c.label(),
            cell(&tables[0], slot),
            cell(&tables[1], slot),
            cell(&tables[2], slot),
            cell(&pooled_table, slot),
        ));
    }
    if !pooled_table.dependent.is_empty() {
        out.push_str("\n  EXACT DEPENDENCIES (pooled) — no finite VIF exists for these:\n");
        for slot in &pooled_table.dependent {
            out.push_str(&format!("    {}\n", FeatureId::all()[*slot].label()));
        }
    }
    if !pooled_table.constant.is_empty() {
        out.push_str("\n  CONSTANT columns (pooled) — no correlation and no VIF is measurable:\n");
        for slot in &pooled_table.constant {
            out.push_str(&format!("    {}\n", FeatureId::all()[*slot].label()));
        }
    }
    out
}

/// §5.5's role in §8, computed: which candidates a `(0, 0)` reading may NOT be
/// removed for.
///
/// The thresholds are applied POOLED **and per stage tercile**, because §8's
/// rule is a fact about a column and not about an aggregate: a column that is
/// collinear in the EARLY tercile is not vacuous there, whatever the pooled
/// figure averages it down to. Both readings are printed with the stage named,
/// so a decision made on one of them says which.
fn render_collinear_flags(pooled: &ColumnStats, by_stage: &[ColumnStats]) -> String {
    let pooled_table = vif_table(pooled);
    let stage_tables: Vec<VifTable> = by_stage.iter().map(vif_table).collect();

    let mut out = String::new();
    out.push_str(&format!(
        "\n--- §8 collinearity flags (|corr| > {COLLINEAR_CORR} against ANY retained column, OR \
         VIF ≥ {COLLINEAR_VIF}) ---\n"
    ));
    let mut any = false;
    for c in CandidateFeature::ALL {
        let i = FeatureId::Candidate(c).index();
        let mut reasons: Vec<String> = Vec::new();
        let mut worst_corr: Option<(f64, &'static str, &'static str)> = None;

        for (label, stats, table) in std::iter::once(("pooled", pooled, &pooled_table))
            .chain(STAGES.iter().map(|s| (s.name(), &by_stage[s.index()], &stage_tables[s.index()])))
        {
            let worst = FeatureId::all()
                .iter()
                .filter(|slot| slot.index() != i)
                .filter_map(|slot| stats.corr(i, slot.index()).map(|r| (r.abs(), slot.label())))
                .max_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((r, against)) = worst {
                if worst_corr.is_none_or(|(best, _, _)| r > best) {
                    worst_corr = Some((r, against, label));
                }
                if r > COLLINEAR_CORR {
                    reasons.push(format!("{label}: |corr| {r:.3} against {against}"));
                }
            }
            if table.dependent.contains(&i) {
                reasons.push(format!("{label}: EXACTLY DEPENDENT"));
            } else if let Some(v) = table.vif.get(&i) {
                if *v >= COLLINEAR_VIF {
                    reasons.push(format!("{label}: VIF {v:.2}"));
                }
            }
        }

        if !reasons.is_empty() {
            any = true;
            out.push_str(&format!("  {:<6} COLLINEAR — {}\n", c.label(), reasons.join(" · ")));
        } else {
            let worst_vif = STAGES
                .iter()
                .map(|s| stage_tables[s.index()].vif.get(&i).copied().unwrap_or(0.0))
                .chain(pooled_table.vif.get(&i).copied())
                .fold(0.0f64, f64::max);
            match worst_corr {
                Some((r, against, label)) => out.push_str(&format!(
                    "  {:<6} clear — worst |corr| {r:.3} against {against} ({label}), VIF ≤ \
                     {worst_vif:.2}\n",
                    c.label(),
                )),
                // A column that never moves is neither flagged nor cleared: it
                // is NOT MEASURABLE, and printing it as clear would read as a
                // measurement that said "independent".
                None => out.push_str(&format!(
                    "  {:<6} not measurable — constant on this corpus, so neither threshold \
                     applies\n",
                    c.label(),
                )),
            }
        }
    }
    if !any {
        out.push_str("  NONE FLAGGED — every candidate clears both thresholds at every stage\n");
    }
    out.push_str(
        "  §8: a flagged column reading (0, 0) is COLLINEAR, not vacuous, and is NOT removed\n",
    );
    out
}

const COLLINEARITY_HEADER: &str = "\
§5.5 asks         candidate×candidate and candidate×incumbent correlations, each
                  candidate's VIF against the retained set, per stage tercile and
                  pooled, conditioned on the drawn Category-B card for E1/Money10
currency          correlations and VIF over the RELATIVE vector (mine − theirs) —
                  the quantity a weight multiplies. Ranges are reported in BOTH
                  currencies: the raw level and the differenced one
population        every (turn start × seat), the same population the AUC cells
                  aggregate, before any aggregation
nothing is fitted this runs BEFORE any fit, and removes nothing";

// ---------------------------------------------------------------------------
// §7 step 5 — the ROOTED-READ GATES
// ---------------------------------------------------------------------------
//
// Checkpoint §7 step 5: "Rooted-read gates at the DEVELOPMENT roots — raw
// direction and non-collision only." No weights, no fitting, nothing beyond the
// frozen gate texts. A gate failure STOPS before any fitting: this command
// reports and exits non-zero, exactly as `collision` does, because repair is
// owner territory.
//
// The gate TEXTS are frozen at spec §6 and at the 2026-08-23 charter row; both
// are quoted in the headers below rather than paraphrased, and the two places a
// frozen text needed a stated READING carry an INTERPRETATION NOTE at the site
// that implements them ([`E113_SUM_READING`] and [`FORK_DISTINCTNESS_READING`]).

/// The SEVEN **S** columns, in §3.2's own registration order.
///
/// The step-5 e113 gate is a sum over "ALL SEVEN frozen S definitions" and the
/// placement-fork gate is a statement about "the S columns", so both need the
/// group as a group. Taken from [`CandidateFeature::ALL`]'s prefix rather than
/// re-typed: `S_COLUMNS` is pinned equal to that prefix in this file's tests, so
/// a re-ordering of the canonical array cannot leave this list describing the
/// old order.
const S_COLUMNS: [CandidateFeature; 7] = [
    CandidateFeature::ProductionAccess,
    CandidateFeature::TrackAccess,
    CandidateFeature::AssistantAreasDistinct,
    CandidateFeature::PresidentLocalAccess,
    CandidateFeature::BankAccess,
    CandidateFeature::EmploymentAgencyAccess,
    CandidateFeature::PresidentReachableAreas,
];

/// The THREE **X** columns — §6's RECORDED WIRING PROOF and nothing else.
const X_COLUMNS: [CandidateFeature; 3] = [
    CandidateFeature::LabTakeable,
    CandidateFeature::PortTakeable,
    CandidateFeature::MatchedProductionAccess,
];

/// The three **O** columns the charter's recorded read names.
const OSE_COLUMNS: [CandidateFeature; 3] = [
    CandidateFeature::HeldOrderValue,
    CandidateFeature::OrderMatchedGoods,
    CandidateFeature::ExecutableOrders,
];

/// The S slots of one §5.3 Δv, in [`S_COLUMNS`] order.
fn s_slots(dv: &[i64; AXIS_WIDTH]) -> [i64; 7] {
    let mut out = [0i64; 7];
    for (i, c) in S_COLUMNS.iter().enumerate() {
        out[i] = dv[FeatureId::Candidate(*c).index()];
    }
    out
}

/// **INTERPRETATION NOTE 1 — the e113 gate's SUM is UNWEIGHTED and CROSS-UNIT,
/// because that is what the frozen text says.**
///
/// Spec §6 (the "Step-5 gates (pass rules fixed here)" paragraph): "at
/// `g1-e113-tiemass` — the summed delta over ALL SEVEN frozen S definitions
/// strictly favors the enabling-line placement over a registered same-count
/// dead-side placement".
///
/// The seven S definitions are NOT in one unit — S1 and S5 are exported reward
/// quantities, S2 is a power, S3/S7 are area counts, S4 is a ring mass, S6 is a
/// token count — so `Σ_{S1..S7} [S(enabling) − S(dead)]` adds numbers that a
/// fitted table would each price separately. Summing them at weight 1 apiece is
/// therefore a WEAKER and coarser statement than any weighted one, and it is
/// deliberately the statement the pre-registration froze: step 5 is the RAW
/// DIRECTION read, before any weight exists to price with (checkpoint §7 step 5
/// — "weighted winner-preservation controls belong to step 9, where complete
/// weighted scoring exists").
///
/// It is implemented LITERALLY. Normalising the columns, weighting them, or
/// reading the gate as "a majority of the seven columns favours the enabling
/// side" would each be a different — and un-pre-registered — gate. The
/// per-column deltas are PRINTED beside the sum so a reader can see exactly
/// which columns carry it and in what units, but the PASS RULE reads the sum.
const E113_SUM_READING: &str = "\
gate text (§6)      \"the summed delta over ALL SEVEN frozen S definitions strictly
                    favors the enabling-line placement over a registered
                    same-count dead-side placement, and no blocking collision
                    between that registered pair's sides\"
the sum             UNWEIGHTED and CROSS-UNIT, implemented literally: the seven S
                    definitions are not in one unit (rewards, a power, area
                    counts, a ring mass, a token count) and the frozen text sums
                    them at weight 1 apiece. That is the RAW-DIRECTION read step 5
                    exists to be — no weight exists yet to price with. Per-column
                    deltas print beside the sum; the PASS RULE reads the sum
the currency        each S is the §5.3 MOVER-RELATIVE value (mine − theirs, 2P),
                    the same differencing Part A registered and this file's
                    `signature` computes — so the gate quantity is
                    Σ_S [Δv_S(enabling successor) − Δv_S(dead successor)]";

/// **INTERPRETATION NOTE 2 — the placement-fork gate's "≥ 2 distinct values" is
/// read as a VECTOR, not column by column.**
///
/// Spec §6: "at each placement-fork dev ordinal — the S columns take ≥ 2 distinct
/// values across the fork's same-count set".
///
/// Read column-by-column ("every S column takes ≥ 2 distinct values") the gate
/// would be FALSE by construction at almost any fork: S5 is zero for every
/// placement that puts nobody at the Bank, S2 is zero for every placement that
/// touches neither track, and a placement fork that happens not to offer the
/// Bank makes S5 constant across every member without that saying anything about
/// blindness. What step 5 is asking is the ANTI-COLLAPSE question — does the S
/// group SEE the fork at all, or does the whole same-count block land on one
/// point of the S subspace the way it lands on one point of the incumbent
/// sixteen? So the reading is: the 7-dimensional S signature takes ≥ 2 distinct
/// values across the fork's same-count set.
///
/// The stronger per-column reading is not silently dropped — the per-column
/// distinct-value counts PRINT, so a reader sees exactly which of the seven move
/// at each root and which are flat there.
///
/// **The second registered choice: WHICH total.** §6 says "the fork's same-count
/// set", and a fork root can carry several totals that each hold ≥ 2 distinct
/// area sets, so the phrase does not by itself pick one. This probe registers the
/// SMALLEST such total. That is the instrument's own reading and is NOT inherited
/// from the frozen predicate: `driven::same_count_fork` returns a BOOL over the
/// placement list in list order and names no total, so there is no witness there
/// to adopt. The choice is made verdict-insensitive by reporting rather than by
/// argument — every other total's counts print beside it, and the verdict line
/// states whether the gate would also have held at each. It does, at all four
/// development ordinals.
const FORK_DISTINCTNESS_READING: &str = "\
gate text (§6)      \"at each placement-fork dev ordinal — the S columns take >= 2
                    distinct values across the fork's same-count set\"
the reading         the VECTOR one: the 7-dimensional S signature takes >= 2
                    distinct values across the set. A column-by-column reading
                    (\"every S column moves\") is FALSE BY CONSTRUCTION at almost
                    any fork — S5 is flat wherever no member reaches the Bank, S2
                    wherever no member touches a track — and would gate the
                    board's furniture rather than the columns. The question step 5
                    asks is ANTI-COLLAPSE: does the S group see the fork at all,
                    where the incumbent sixteen collapse it to one number?
                    Per-column distinct counts print, so the stronger reading is
                    visible even though the pass rule is the vector one
the same-count set  §6 says \"the fork's same-count set\" and a fork root can carry
                    several forking totals, so THIS PROBE REGISTERS the choice: the
                    placements sharing the SMALLEST total that carries >= 2 distinct
                    area sets. That is this instrument's reading and not the frozen
                    predicate's — `same_count_fork` returns a BOOL over placements
                    in list order and names no total at all, so there is no witness
                    to inherit from it. The choice is made verdict-insensitive by
                    reporting: every OTHER total's counts print, and the verdict
                    line says whether the gate would also have held at each";

// --- the e113 exemplar pair -------------------------------------------------

/// The registry root the exemplar pair is registered AT.
const E113_ROOT: &str = "g1-e113-tiemass";

/// `state_digest` of that root — the step-4 ledger row's recorded identity.
/// Verified BEFORE any column is read; a mismatch is a typed refusal, because a
/// gate computed at a different position is a gate about a different question.
const E113_ROOT_DIGEST: &str = "565903fe991210f0";

/// **THE e113 EXEMPLAR PAIR, FROZEN.**
///
/// Checkpoint §7 step 2 registers "the development roots — g1:113 plus
/// predeclared organic same-count/different-location roots (**the class, not one
/// exemplar**)". The class was pre-registered; step 5 registers the EXEMPLAR
/// inside it, which is a registration and not a scope change.
///
/// **The enabling side** is the D7 triage's own recorded mechanism fact
/// (`docs/plans/2026-08-17-phase-b-d7-triage.md` §1, "Two mechanism facts",
/// item 1): "the searched principal line walks the President to Fishing Ground
/// (A09) and produces seafood — yet the placement that actually includes A09
/// ('Laboratory A + Fishing Ground') sits at −1, BELOW the 36-way winning tie".
/// It is therefore the root's legal placement whose area set is EXACTLY
/// {Laboratory A, Fishing Ground}, resolved from the loaded root's own board by
/// [`AreaType`] and never by a guessed id.
///
/// **The dead side's criterion, stated so it can be re-applied rather than
/// trusted:** the LEXICOGRAPHICALLY-LEAST member — by the move's canonical
/// encoding, which is its derived `Debug` rendering, the same string every
/// artifact in this file prints a move as — of the RECOMPUTED top tie, among
/// those members that place the SAME TOTAL as the enabling side and whose placed
/// AREA SET is DISJOINT from the enabling side's. "Lexicographically least"
/// rather than "first iterated": an iteration order is a property of the
/// enumerator, and a selection that depends on it is a selection that a
/// generator change can silently re-point.
///
/// Both sides are frozen HERE as area ids plus their canonical encodings plus
/// the `state_digest` of the successor each produces, and [`e113_exemplar`]
/// RE-DERIVES the criterion at runtime and REFUSES on any disagreement. The
/// frozen constants are the record; the re-derivation is what stops them from
/// drifting away from the criterion that chose them.
struct ExemplarPair {
    enabling_areas: &'static [u8],
    enabling_encoding: &'static str,
    enabling_digest: &'static str,
    dead_areas: &'static [u8],
    dead_encoding: &'static str,
    dead_digest: &'static str,
    /// The total each side places — equal by the criterion, frozen so a
    /// re-derivation that produced a different-sized pair is caught by the
    /// constant and not only by the equality.
    total: usize,
}

/// The frozen registration. Recorded by the step-5 development selection run;
/// re-verified against its own criterion on every run since.
const E113_EXEMPLAR: ExemplarPair = ExemplarPair {
    enabling_areas: E113_ENABLING_AREAS,
    enabling_encoding: E113_ENABLING_ENCODING,
    enabling_digest: E113_ENABLING_DIGEST,
    dead_areas: E113_DEAD_AREAS,
    dead_encoding: E113_DEAD_ENCODING,
    dead_digest: E113_DEAD_DIGEST,
    total: E113_TOTAL,
};

const E113_ENABLING_AREAS: &[u8] = &[0, 9];
const E113_ENABLING_ENCODING: &str = "PlaceThreeAreas { placements: [AreaId(0), AreaId(9)] }";
const E113_ENABLING_DIGEST: &str = "1e4bd02ecce80635";
const E113_DEAD_AREAS: &[u8] = &[1, 2];
const E113_DEAD_ENCODING: &str = "PlaceThreeAreas { placements: [AreaId(1), AreaId(2)] }";
const E113_DEAD_DIGEST: &str = "4f5bf9d05388c4c6";
const E113_TOTAL: usize = 2;

/// Resolve the unique area of one [`AreaType`] on a board.
///
/// A type that appears twice is a typed refusal rather than a first-match: the
/// exemplar's enabling side is named by SITE ("Laboratory A + Fishing Ground")
/// and a board where the name does not pick out one tile is a board where the
/// registration's own words do not identify a placement.
fn sole_area_of(state: &GameState, want: AreaType) -> Result<AreaId, String> {
    let found: Vec<AreaId> = state
        .board_layout
        .areas
        .iter()
        .filter(|(_, cfg)| cfg.area_type == want)
        .map(|(id, _)| *id)
        .collect();
    match found.as_slice() {
        [one] => Ok(*one),
        [] => Err(format!("{want:?} is not on this board — the exemplar's site name resolves to nothing")),
        many => Err(format!(
            "{want:?} appears {} times on this board ({many:?}) — the exemplar's site name does \
             not identify one placement",
            many.len(),
        )),
    }
}

/// A placement move's total and area set, through the FROZEN predicate's own
/// helper (`driven::placement_shape`) so "same count" means one thing repo-wide.
fn placement_of(mv: &Move) -> Option<(usize, BTreeSet<AreaId>)> {
    placement_shape(mv)
}

/// The canonical encoding of a move: its derived `Debug` rendering.
///
/// `Move` implements no `Ord`, and the selection criterion needs a TOTAL order
/// that does not depend on enumeration. `Debug` is derived, deterministic, and
/// already the string every artifact in this file prints a move as, so ordering
/// by it makes the criterion re-checkable from the printed output alone.
fn canonical_encoding(mv: &Move) -> String {
    format!("{mv:?}")
}

/// The e113 root, its recomputed top tie, and the identity that says both are
/// the ones the registration was taken at.
struct E113Root {
    state: GameState,
    digest: String,
    tie: Vec<Move>,
    top_value: i64,
    candidates: usize,
    echo: String,
    decision_seed: u64,
    /// Every ranked candidate and its table value — kept so the registration
    /// can be checked against the D7 triage fact it is taken FROM ("the
    /// placement that actually includes A09 sits at −1, BELOW the 36-way
    /// winning tie"), rather than only against the tie.
    ranked: Vec<(Move, i64)>,
}

/// Load `g1-e113-tiemass` and recompute its top tie.
///
/// Factored out of [`e113_top_tie`] rather than restated: the step-5 gate's dead
/// side is chosen from THE SAME tie the step-4 measurement reports, and two
/// copies of "call `root_table`, take `top_value`, filter" is exactly the shape
/// that lets the two drift into describing different sets.
fn e113_root(rules: &Rules) -> Result<E113Root, String> {
    let entry = registry()
        .into_iter()
        .find(|e| e.name == E113_ROOT)
        .ok_or(format!("{E113_ROOT} is not a registry root"))?;
    let source = entry.load(rules).map_err(|e| format!("{E113_ROOT}: {e:?}"))?;
    let (state, descriptor) = source.resolve(rules).map_err(|e| format!("{e:?}"))?;
    let contestant = Contestant::from_str_name(DEFAULT_SEAT)
        .map_err(|e| format!("the driven default contestant does not parse: {e}"))?;
    let table = root_table(&state, entry.bot_seat, contestant, rules, entry.decision_seed)
        .map_err(|e| format!("root table at g1-e113: {e:?}"))?;
    let top_value = table.top_value().ok_or("the root table is empty")?;
    let tie: Vec<Move> =
        table.ranked.iter().filter(|r| r.value == top_value).map(|r| r.mv.clone()).collect();
    Ok(E113Root {
        digest: descriptor.root_digest,
        candidates: table.candidates(),
        echo: table.echo.clone(),
        decision_seed: entry.decision_seed,
        ranked: table.ranked.iter().map(|r| (r.mv.clone(), r.value)).collect(),
        state,
        tie,
        top_value,
    })
}

/// The two sides the criterion selects at a loaded root, DERIVED — never read
/// off the frozen constants.
///
/// Returned as moves plus the facts the artifact prints, so the re-verification
/// against [`E113_EXEMPLAR`] happens at ONE site and reads like the comparison
/// it is.
struct DerivedExemplar {
    lab: AreaId,
    fishing: AreaId,
    enabling: Move,
    dead: Move,
    total: usize,
    /// Every top-tie member that passed the same-count + disjointness filter the
    /// least-encoding rule then broke, in that encoding order — `eligible[0]`
    /// IS [`DerivedExemplar::dead`]. Kept whole rather than counted: a filter
    /// that admitted exactly one member would make "lexicographically least"
    /// vacuous, and the SPREAD over all of them is what says whether the gate's
    /// verdict is a property of the position or of the tie-break.
    eligible: Vec<Move>,
}

/// The dead-side criterion's FILTER and ORDER, as a pure function over a tie.
///
/// Split out of [`derive_e113_exemplar`] so the criterion is checkable without a
/// loaded root: the registration's whole claim is that its dead side is what
/// this function returns first, and a criterion that can only be exercised
/// through a gitignored save is a criterion nothing tests.
///
/// Returns the eligible members in canonical-encoding order — `[0]` is the
/// registered dead side.
fn eligible_dead_sides<'a>(
    tie: &'a [Move],
    enabling_areas: &BTreeSet<AreaId>,
    total: usize,
) -> Vec<&'a Move> {
    let mut out: Vec<&Move> = tie
        .iter()
        .filter(|mv| {
            placement_of(mv).is_some_and(|(t, areas)| t == total && areas.is_disjoint(enabling_areas))
        })
        .collect();
    out.sort_by_key(|mv| canonical_encoding(mv));
    out
}

/// Apply the registered criterion to a loaded root.
fn derive_e113_exemplar(root: &E113Root, rules: &Rules) -> Result<DerivedExemplar, String> {
    let lab = sole_area_of(&root.state, AreaType::LaboratoryA)?;
    let fishing = sole_area_of(&root.state, AreaType::FishingGround)?;
    let wanted: BTreeSet<AreaId> = [lab, fishing].into_iter().collect();

    // The ENABLING side comes off the root's own legal list — not off the tie.
    // The D7 fact is precisely that it is NOT in the tie, so looking for it
    // there would be looking for it where the triage says it is not.
    let legal = get_legal_moves(&root.state, rules);
    let enabling: Vec<&Move> = legal
        .iter()
        .filter(|mv| placement_of(mv).is_some_and(|(_, areas)| areas == wanted))
        .collect();
    let enabling = match enabling.as_slice() {
        [one] => (*one).clone(),
        [] => {
            return Err(format!(
                "no legal placement at {E113_ROOT} has area set {{Laboratory A = {}, Fishing \
                 Ground = {}}} — the D7 mechanism fact the enabling side is registered from does \
                 not hold at this root",
                lab.0, fishing.0,
            ))
        }
        many => {
            return Err(format!(
                "{} legal placements share the area set {{{}, {}}} — the enabling side is not \
                 identified by its site names alone",
                many.len(),
                lab.0,
                fishing.0,
            ))
        }
    };
    let (total, enabling_areas) =
        placement_of(&enabling).ok_or("the enabling side is not a placement")?;

    // The DEAD side: same total, DISJOINT area set, taken from the recomputed
    // top tie, broken by the least canonical encoding.
    let eligible = eligible_dead_sides(&root.tie, &enabling_areas, total);
    let dead = eligible
        .first()
        .ok_or_else(|| {
            format!(
                "no member of the {}-way top tie places {total} into an area set disjoint from \
                 {{{}, {}}} — the dead-side criterion selects nothing here",
                root.tie.len(),
                lab.0,
                fishing.0,
            )
        })?
        .to_owned()
        .clone();

    Ok(DerivedExemplar {
        lab,
        fishing,
        enabling,
        dead,
        total,
        eligible: eligible.into_iter().cloned().collect(),
    })
}

/// One NAMED SEAT's own seven S values on one view — the absolute currency
/// [`E113_ABSOLUTE_NOTE`] records beside the gate.
///
/// The seat is a parameter rather than "the mover" because both halves of that
/// note are read through this: the mover's values are the recorded diagnostic,
/// and the OPPONENT's are what
/// `a_movers_placement_cannot_move_the_opponents_s_vector` pins invariant.
fn seat_s_values(view: &PlayerView, seat: PlayerId, rules: &Rules) -> Result<[i64; 7], String> {
    let me = view
        .players
        .iter()
        .find(|p| p.id == seat)
        .ok_or_else(|| format!("seat {} is not a player in this view", seat.0))?;
    let structure = ScoreStructure::derive(view, rules);
    let mut out = [0i64; 7];
    for (i, c) in S_COLUMNS.iter().enumerate() {
        out[i] = candidate_value(&structure, view, me, *c, rules);
    }
    Ok(out)
}

/// Both S readings of one successor of the e113 root.
fn e113_successor_s(
    root: &GameState,
    mv: &Move,
    mover: PlayerId,
    rules: &Rules,
) -> Result<([i64; 7], [i64; 7]), String> {
    let next = apply_move(root, mv, rules)
        .map_err(|e| format!("{} did not apply at {E113_ROOT}: {e}", canonical_encoding(mv)))?;
    let view = observe(&next, mover).map_err(|e| format!("successor unobservable: {e}"))?;
    Ok((s_slots(&signature(&view, mover, rules)?.dv), seat_s_values(&view, mover, rules)?))
}

/// One frozen side, checked against what the criterion just derived.
fn check_frozen_side(
    label: &str,
    derived: &Move,
    successor_digest: &str,
    areas: &'static [u8],
    encoding: &'static str,
    digest: &'static str,
) -> Result<(), String> {
    let derived_encoding = canonical_encoding(derived);
    let derived_areas: Vec<u8> = placement_of(derived)
        .map(|(_, set)| set.iter().map(|a| a.0).collect())
        .unwrap_or_default();
    if derived_areas != areas {
        return Err(format!(
            "the {label} side's FROZEN areas {areas:?} are not what the registered criterion \
             derives here ({derived_areas:?}) — the registration has drifted from its own rule",
        ));
    }
    if derived_encoding != encoding {
        return Err(format!(
            "the {label} side's FROZEN encoding `{encoding}` is not the derived one \
             `{derived_encoding}`",
        ));
    }
    if successor_digest != digest {
        return Err(format!(
            "the {label} side's successor digest is {successor_digest}, and the registration \
             froze {digest} — the same move at a different position",
        ));
    }
    Ok(())
}

/// **WHY THE ABSOLUTE CURRENCY IS RECORDED BESIDE THE GATE, AND IS NOT IT.**
///
/// The gate quantity is the DIFFERENCED one — the §5.3 mover-relative
/// `mine − theirs` Part A registered, which is also the quantity a weight
/// multiplies in `game/eval.rs`. That is the convention this task was directed
/// to implement and it is the one the pass rule reads.
///
/// **The two readings differ only through the OPPONENT's S values**, and at THIS
/// root they do not differ at all. Every one of the seven S definitions reads the
/// QUERIED SEAT's own state: the presence gate is that seat's own
/// `context.occupied[seat]` bitset, and `eval::realizable_power`
/// (`game/eval.rs:2411-2430`) sums the unconditional president term, that seat's
/// `assistants.in_area(area)`, and that seat's own shophouse and trading house.
/// **A mover's placement therefore cannot move any term of the OPPONENT's S
/// vector**, so on a placement family the opponent's half is a genuine constant
/// and the two currencies COINCIDE — which is why the run prints them equal at
/// every row of the spread, and what `a_movers_placement_cannot_move_the_
/// opponents_s_vector` pins structurally rather than by observing it once.
///
/// **The one real cross-seat channel is the STATION.** `realizable_power`'s only
/// board-global term is `view.station_location == Some(area)`, worth +1 to
/// WHOEVER acts there — so a successor that RELOCATES the station changes the
/// opponent's S1, S2, S5 and S6 as well as the mover's, and there the two
/// currencies genuinely separate. No placement relocates it (the station moves
/// through its own `StationPlace` resolution), so that channel is simply not
/// open on this family; at a root whose successors are station moves it would
/// be, and the diagnostic is carried for exactly that case.
///
/// Both are therefore printed and the METHODOLOGY is what is load-bearing: the
/// gate reads the differenced sum, the absolute sum is a RECORDED diagnostic,
/// and if the two ever disagree in SIGN that disagreement is a finding for the
/// owner — never a licence to read whichever one passes.
const E113_ABSOLUTE_NOTE: &str = "\
recorded beside     the same sum in the MOVER-ABSOLUTE currency (the opponent's half
                    dropped). The two differ ONLY through the opponent's S values,
                    and a mover's PLACEMENT cannot move any of them: every S reads
                    the queried seat's own occupancy, and realizable_power
                    (eval.rs:2411-2430) sums the president term plus THAT seat's
                    own assistants, shophouse and trading house. So on this family
                    the opponent's half is a genuine constant and the two readings
                    COINCIDE — as the spread below prints at every row.
                    The one real cross-seat channel is the STATION: realizable_power
                    reads view.station_location, worth +1 to WHOEVER acts there, so
                    a successor that RELOCATES the station moves the opponent's S
                    too. No placement does; at a station-moving root it would.
                    The GATE reads the DIFFERENCED sum. A sign disagreement
                    between the two is a FINDING, never a choice of reading";

/// **THE RE-REGISTRATION**, printed under the e113 numbers so the reading that
/// governs them travels with them.
///
/// The ruling is quoted rather than paraphrased on the two points a reader would
/// otherwise have to take on trust: WHY the frozen form was ruled ill-posed (a
/// category error in the gate's own shape, not a defect in the columns), and
/// WHAT the direction claim now has to satisfy instead. The killed alternatives
/// are named because a re-registration that only records the option taken reads
/// as the only option there was.
const E113_REREGISTRATION: &str = "\
the ruling          guidingpath 2026-08-23, \"THE TWO STEP-5 FINDINGS ARE RULED\"
                    (owner, on the independently CONFIRMED lane review): the frozen
                    §6 direction gate is FAILED-AS-REGISTERED here, and the claim
                    RE-REGISTERS FORWARD to step 9(b)
why                 \"a category error frozen into the gate's form: an unweighted sum
                    across incommensurable units\" — the dead side sits on Customs,
                    so S2 swings -3 in POWER units against S1's +1 in REWARD units.
                    Robust: 3 of 28 dead candidates favoured, range [-8,+2]
what step 9(b) asks the FITTED identity's WEIGHTED preference must favour the
                    enabling side over the registered dead side, among the complete
                    weighted rooted scores §7 already requires there — unit-coherent,
                    falsifiable, and a test of the thing that ships
the pair's forward   enabling successor 1e4bd02ecce80635 · dead successor
identity            4f5bf9d05388c4c6 — the two digests step 9(b) re-reads
killed alternatives the MAJORITY reading also fails this data (the nonzero deltas
                    split 2-2); per-column-any is vacuous; pre-fit normalization
                    smuggles weights into a weight-free gate; closing at the frozen
                    letter would record representation-infeasibility for a unit
                    artifact against GATE 2's organic evidence
what STANDS here    NON-COLLISION (a step-5 gate, PASS) and the 28-candidate
                    separation spread below, recorded as WIRING evidence: the sums
                    span [-8,+2], so the S group SEPARATES. What failed was the
                    frozen DIRECTION, implemented literally per the two in-source
                    interpretation notes
the honest cost     step 5 loses its only directional cell; the milestone's first
                    directional evidence moves POST-FIT. The ruling records that as
                    the cost of the re-registration, and so does this line";

/// The e113 gate's result, as the artifact prints it.
struct E113Gate {
    root_digest: String,
    candidates: usize,
    top_value: i64,
    tie: usize,
    echo: String,
    decision_seed: u64,
    lab: AreaId,
    fishing: AreaId,
    total: usize,
    eligible: usize,
    enabling: String,
    /// The enabling side's own value in the SHIPPED evaluator's root table, and
    /// its distance below the top — the D7 triage fact the registration cites,
    /// re-derived rather than trusted.
    enabling_value: Option<i64>,
    dead: String,
    enabling_digest: String,
    dead_digest: String,
    /// Per-column `Δv_S(enabling) − Δv_S(dead)`, in [`S_COLUMNS`] order.
    per_column: [i64; 7],
    sum: i64,
    /// The SAME quantity in the MOVER-ABSOLUTE currency — `S_mine(enabling) −
    /// S_mine(dead)`, the opponent's half dropped. **RECORDED, never the pass
    /// rule** (see [`E113_ABSOLUTE_NOTE`]).
    absolute_per_column: [i64; 7],
    absolute_sum: i64,
    /// Every eligible dead-side candidate, scored the same two ways:
    /// `(canonical encoding, differenced sum, absolute sum)`. **RECORDED, never
    /// a gate** — it is what says whether a verdict is a property of the
    /// POSITION or of the criterion's own arbitrary tie-break.
    spread: Vec<(String, i64, i64)>,
    /// Whether the two sides share a §5.3 signature, through Part A's own
    /// classifier. Empty is the pass.
    blocking: Vec<Blocking>,
    /// The complete 34-slot signatures, for the artifact's record.
    enabling_dv: [i64; AXIS_WIDTH],
    dead_dv: [i64; AXIS_WIDTH],
    enabling_ppm: u32,
    dead_ppm: u32,
}

impl E113Gate {
    /// The RAW DIRECTION read — §6's frozen "strictly favors" over the summed
    /// delta. Whether it is a verdict input is [`Verdict`]'s business, not this
    /// type's; here it is simply what the sum says.
    fn direction_favours_enabling(&self) -> bool {
        self.sum > 0
    }

    /// The pair's NON-COLLISION, through Part A's own classifier.
    fn non_collision(&self) -> bool {
        self.blocking.is_empty()
    }
}

/// **GATE 1** — the e113 S-sum and the pair's non-collision.
fn e113_exemplar(rules: &Rules) -> Result<E113Gate, String> {
    let root = e113_root(rules)?;
    if root.digest != E113_ROOT_DIGEST {
        return Err(format!(
            "{E113_ROOT} resolves to state_digest {} and the ledger recorded {E113_ROOT_DIGEST} \
             — REFUSING before any column is read: a gate computed at a different position is a \
             gate about a different question",
            root.digest,
        ));
    }

    let derived = derive_e113_exemplar(&root, rules)?;
    let mover = root.state.current_player;

    let mut signed = Vec::new();
    let mut side_facts = Vec::new();
    for (label, mv, role) in [
        ("enabling", &derived.enabling, RecipeRole::Productive),
        ("dead", &derived.dead, RecipeRole::Dead),
    ] {
        let next = apply_move(&root.state, mv, rules)
            .map_err(|e| format!("the {label} side did not apply at {E113_ROOT}: {e}"))?;
        let view = observe(&next, mover)
            .map_err(|e| format!("the {label} side's successor is unobservable: {e}"))?;
        if view_is_terminal(&view) {
            return Err(format!(
                "the {label} side's successor is TERMINAL — §5.3's domain, which this gate reads \
                 in, is non-terminal views",
            ));
        }
        let sig = signature(&view, mover, rules)?;
        side_facts.push((label, canonical_encoding(mv), state_digest(&next), sig.clone()));
        signed.push(SignedView {
            sig,
            from: Provenance::Successor {
                root: E113_ROOT.to_string(),
                set: "exemplar",
                ordinal: signed.len(),
                mv: canonical_encoding(mv),
                role,
            },
        });
    }

    // The frozen registration is checked AFTER the successors exist (their
    // digests are half of it) and BEFORE the gate quantity is reported.
    check_frozen_side(
        "enabling",
        &derived.enabling,
        &side_facts[0].2,
        E113_EXEMPLAR.enabling_areas,
        E113_EXEMPLAR.enabling_encoding,
        E113_EXEMPLAR.enabling_digest,
    )?;
    check_frozen_side(
        "dead",
        &derived.dead,
        &side_facts[1].2,
        E113_EXEMPLAR.dead_areas,
        E113_EXEMPLAR.dead_encoding,
        E113_EXEMPLAR.dead_digest,
    )?;
    if derived.total != E113_EXEMPLAR.total {
        return Err(format!(
            "the criterion derives a total of {} and the registration froze {}",
            derived.total, E113_EXEMPLAR.total,
        ));
    }

    let enabling_s = s_slots(&side_facts[0].3.dv);
    let dead_s = s_slots(&side_facts[1].3.dv);
    let mut per_column = [0i64; 7];
    for i in 0..7 {
        per_column[i] = enabling_s[i] - dead_s[i];
    }

    // The RECORDED absolute currency, and the RECORDED spread over every
    // eligible dead-side candidate. Neither is a pass rule; both exist so a
    // verdict can be read as a statement about the position rather than about
    // the tie-break that picked one candidate out of many.
    let (_, enabling_abs) = e113_successor_s(&root.state, &derived.enabling, mover, rules)?;
    let mut absolute_per_column = [0i64; 7];
    let mut spread = Vec::new();
    for candidate in &derived.eligible {
        let (diff_s, abs_s) = e113_successor_s(&root.state, candidate, mover, rules)?;
        let diff_sum: i64 = (0..7).map(|i| enabling_s[i] - diff_s[i]).sum();
        let abs_sum: i64 = (0..7).map(|i| enabling_abs[i] - abs_s[i]).sum();
        if candidate == &derived.dead {
            for i in 0..7 {
                absolute_per_column[i] = enabling_abs[i] - abs_s[i];
            }
        }
        spread.push((canonical_encoding(candidate), diff_sum, abs_sum));
    }

    // NON-COLLISION through Part A's OWN classifier and blocking rule, not a
    // restatement of it: the two sides go in as one root's registered
    // development successor pair, which is the arm §5.3 names, and
    // `blocking_findings` decides.
    let part_a = classify(signed);

    Ok(E113Gate {
        root_digest: root.digest,
        candidates: root.candidates,
        top_value: root.top_value,
        tie: root.tie.len(),
        echo: root.echo,
        decision_seed: root.decision_seed,
        lab: derived.lab,
        fishing: derived.fishing,
        total: derived.total,
        eligible: derived.eligible.len(),
        enabling: side_facts[0].1.clone(),
        enabling_value: root
            .ranked
            .iter()
            .find(|(mv, _)| *mv == derived.enabling)
            .map(|(_, value)| *value),
        dead: side_facts[1].1.clone(),
        enabling_digest: side_facts[0].2.clone(),
        dead_digest: side_facts[1].2.clone(),
        per_column,
        sum: per_column.iter().sum(),
        absolute_per_column,
        absolute_sum: absolute_per_column.iter().sum(),
        spread,
        blocking: part_a.blocking(),
        enabling_dv: side_facts[0].3.dv,
        dead_dv: side_facts[1].3.dv,
        enabling_ppm: side_facts[0].3.ppm,
        dead_ppm: side_facts[1].3.ppm,
    })
}

// --- the placement-fork development ordinals --------------------------------

/// The two distinctness readings of a set of S vectors: `(distinct VECTORS,
/// distinct values of each column alone)`.
///
/// Pure, so [`FORK_DISTINCTNESS_READING`]'s claim — that the two readings differ
/// and that the vector one is the gate — is executable without playing a game.
fn distinct_s(vectors: &[[i64; 7]]) -> (usize, [usize; 7]) {
    let mut per_column = [0usize; 7];
    for (i, slot) in per_column.iter_mut().enumerate() {
        *slot = vectors.iter().map(|v| v[i]).collect::<BTreeSet<_>>().len();
    }
    (vectors.iter().collect::<BTreeSet<_>>().len(), per_column)
}

/// One fork total's anti-collapse reading.
struct TotalRead {
    total: usize,
    members: usize,
    /// Distinct 7-dimensional S signatures across those members.
    distinct_vectors: usize,
    /// Distinct values of each S column alone, in [`S_COLUMNS`] order — the
    /// stronger per-column reading, PRINTED so it is visible even though the
    /// pass rule is the vector one.
    distinct_per_column: [usize; 7],
    /// Distinct COMPLETE 34-slot Δv, for scale: the S group's separation is only
    /// interesting against what the whole axis sees.
    distinct_complete: usize,
    /// Distinct incumbent-only Δv — what the SHIPPED sixteen see here.
    distinct_incumbent: usize,
}

/// One placement-fork development ordinal, read.
struct ForkRead {
    name: String,
    origin: String,
    digest: String,
    /// The witnessing total — the smallest carrying ≥ 2 distinct area sets.
    ///
    /// THIS PROBE's registered choice, not the frozen predicate's: see
    /// [`FORK_DISTINCTNESS_READING`]'s second registered choice.
    witness: usize,
    totals: Vec<TotalRead>,
}

impl ForkRead {
    fn witness_read(&self) -> Option<&TotalRead> {
        self.totals.iter().find(|t| t.total == self.witness)
    }

    fn passed(&self) -> bool {
        self.witness_read().is_some_and(|t| t.distinct_vectors >= 2)
    }

    /// Whether the gate would ALSO have held at every other total that carries
    /// a fork — recorded beside the verdict, never the pass rule.
    fn holds_everywhere(&self) -> bool {
        self.totals.iter().all(|t| t.members < 2 || t.distinct_vectors >= 2)
    }
}

/// **GATE 2** — the S columns are not collapsed across a fork's same-count set.
///
/// The fork set is reached the way play reaches it and the way Part A's own
/// `fork` set does: `EndAdditionalPhase` through the CHECKED boundary, then the
/// placement family. Signatures are the §5.3 ones from the ROOT MOVER's seat, so
/// this gate and Part A speak one currency. (At a fork the opponent's half of
/// `mine − theirs` is the same for every member, so the differencing shifts the
/// whole set by a constant and cannot manufacture or destroy distinctness — it
/// is used for consistency, not for effect.)
fn read_placement_fork(root: &DevRoot, rules: &Rules) -> Result<ForkRead, String> {
    let moves = get_legal_moves(&root.state, rules);
    if !moves.contains(&Move::EndAdditionalPhase) {
        return Err(format!(
            "{}: the fork set is unreachable — `EndAdditionalPhase` is not legal at {:?}",
            root.name, root.state.turn_phase,
        ));
    }
    let placing = apply_move(&root.state, &Move::EndAdditionalPhase, rules)
        .map_err(|e| format!("{}: EndAdditionalPhase did not apply: {e}", root.name))?;
    let placements = get_legal_moves(&placing, rules);
    let mover = root.state.current_player;

    // Group by total, then read each group's distinctness.
    let mut by_total: BTreeMap<usize, Vec<(BTreeSet<AreaId>, Signature)>> = BTreeMap::new();
    for mv in &placements {
        let Some((total, areas)) = placement_of(mv) else {
            continue;
        };
        let next = apply_move(&placing, mv, rules)
            .map_err(|e| format!("{}: a fork member did not apply: {e}", root.name))?;
        let view = observe(&next, mover)
            .map_err(|e| format!("{}: a fork successor is unobservable: {e}", root.name))?;
        if view_is_terminal(&view) {
            continue;
        }
        by_total.entry(total).or_default().push((areas, signature(&view, mover, rules)?));
    }

    // The WITNESS is THIS PROBE's registered choice — the smallest total whose
    // area sets number two or more. `driven::same_count_fork` names no total to
    // inherit (it is a bool over the placement list), so this is a reading the
    // instrument owns; every other total's counts are reported beside it.
    let witness = by_total
        .iter()
        .find(|(_, members)| {
            members.iter().map(|(areas, _)| areas).collect::<BTreeSet<_>>().len() >= 2
        })
        .map(|(total, _)| *total)
        .ok_or_else(|| {
            format!(
                "{}: no total carries two distinct area sets — this root does not satisfy the \
                 predicate it was generated by",
                root.name,
            )
        })?;

    let mut totals = Vec::new();
    for (total, members) in &by_total {
        let sigs: Vec<&Signature> = members.iter().map(|(_, s)| s).collect();
        let vectors: Vec<[i64; 7]> = sigs.iter().map(|s| s_slots(&s.dv)).collect();
        let (distinct_vectors, per_column) = distinct_s(&vectors);
        totals.push(TotalRead {
            total: *total,
            members: members.len(),
            distinct_vectors,
            distinct_per_column: per_column,
            distinct_complete: sigs.iter().map(|s| s.dv).collect::<BTreeSet<_>>().len(),
            distinct_incumbent: sigs
                .iter()
                .map(|s| {
                    Feature::ALL.map(|f| s.dv[FeatureId::Incumbent(f).index()])
                })
                .collect::<BTreeSet<_>>()
                .len(),
        });
    }

    Ok(ForkRead {
        name: root.name.clone(),
        origin: root.origin.clone(),
        digest: root.digest.clone(),
        witness,
        totals,
    })
}

// --- the OSE reads: the 2026-08-23 charter row ------------------------------

/// The charter row, quoted rather than paraphrased so a figure taken out of this
/// artifact carries the pre-registration that authorised it.
const OSE_CHARTER: &str = "\
charter (guidingpath 2026-08-23, \"THE STEP-5 OSE READ CHARTER IS RULED\")

the split           the flag-1 re-scope realized as a registered split of the
                    thirteen recorded game-3 opponent-pending exhibits: FIVE
                    development / EIGHT escrow-reserved, by the fixed EVERY-THIRD
                    rule over event order. Development = e34 e68 e111 e159 e198;
                    escrow-reserved UNREAD = e50 e58 e78 e94 e127 e144 e168 e184
                    (exactly §6's {8..15} band size, so the >= 6-of-8 block-pass
                    rule applies unchanged at step 10)
the burn rule       \"The §6 burn rule extends to the reserved eight VERBATIM: no
                    table/adjudicate/read of any kind before step 10.\" This
                    surface REFUSES a column read at any of the eight, by
                    construction and not by convention. Digest-IDENTITY
                    verification at all thirteen is permitted (the §6
                    capacity-measurement analogue: no root table is read)
the registered read at every dev root, opponent-O1 >= 1 AND opponent-O3 = 0 —
                    the first is the pending value the blind evaluator lacks,
                    exposed at a recorded organic state; the second is STRICT
                    HONESTY, the column reproducing the census truth that strict
                    sitting-executable is ZERO at every recorded human position.
                    opponent-O2 is READ-AND-RECORDED per root, never gated, and
                    the existential \">= 1 dev root shows opponent-O2 >= 1\" is
                    recorded as a FINDING
THE BAND, RE-REGISTERED (guidingpath 2026-08-23, \"THE TWO STEP-5 FINDINGS ARE RULED\")

the dev band        {e111, e159, e198} — the three the recorded document can show a
                    MOVER's view of. The charter's other two, e34 and e68, are
                    recorded NOT-RUN-STRUCTURAL and re-registered OUT: seat 0's deal
                    is Unknown in the import fiction until its event-73 reveal (seat
                    1's is event 14) and both movers there are seat 0
the reserved eight  UNTOUCHED by this re-registration. The burn guard still refuses
                    every column read at them, and step 10's own unviewable-ordinal
                    rule (VOID != miss, registered in the same row) is step 10's

strict O3 coverage  routes to the CONSTRUCTED fixtures exactly as the flag-1
                    ruling's own text routes it: family-4(b)'s EXECUTABLE side
                    carries O3 >= 1, with 4(b)-DISTANT and 4(a) BOTH SIDES as the
                    O3 = 0 specificity controls";

/// The record the thirteen exhibits are events of.
const OSE_RUN_ID: &str = "bga-import/896199544";
const OSE_EVENTS: usize = 234;
const OSE_RULES_HASH: &str = "d85445c479cb1ead";
const OSE_TERMINAL_DIGEST: &str = "3a9b2c6d7206f3e2";

/// **THE THIRTEEN FLAG-1 EXHIBIT IDENTITIES**, in event order, transcribed from
/// the game-3 lane-close ledger row (`state_digest`, semantics.rs:207).
///
/// The list is the IDENTITY of the band and the input to the every-third split
/// both. Nothing here is read without first matching its digest.
const OSE_EXHIBITS: [(usize, &str); 13] = [
    (34, "b0378e42b483fb5d"),
    (50, "9a7bf7de4e63c6f3"),
    (58, "e41026415c46fdd2"),
    (68, "e2ddfcf9ff66d27d"),
    (78, "fb820d3a0b414792"),
    (94, "28e21cbb8e6cc585"),
    (111, "e95ce315885ffbcd"),
    (127, "96c80532fd189c9c"),
    (144, "300f68760903da34"),
    (159, "a425836e88d1d337"),
    (168, "1a014df522b362d4"),
    (184, "21665544fdd030d3"),
    (198, "30f75795b472095b"),
];

/// The charter's DEVELOPMENT five, transcribed from the ruling row.
const OSE_DEVELOPMENT: [usize; 5] = [34, 68, 111, 159, 198];

/// The charter's ESCROW-RESERVED eight, transcribed from the ruling row. **No
/// column read reaches these before step 10.**
const OSE_RESERVED: [usize; 8] = [50, 58, 78, 94, 127, 144, 168, 184];

/// What an exhibit is FOR — the burn guard's type.
///
/// The guard is structural rather than a convention: [`ose_columns`] takes a
/// `Role` and the `Reserved` arm has no column-reading branch at all, so a
/// caller cannot reach a reserved exhibit's columns by forgetting a check.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OseRole {
    Development,
    Reserved,
}

/// The charter's EVERY-THIRD rule, applied to [`OSE_EXHIBITS`]' event order.
///
/// DERIVED, then pinned against the two transcribed lists in this file's tests:
/// the ruling names both the rule and its answer, so the honest implementation
/// computes the rule and checks it reproduces the answer. A transcription slip
/// in either list is then a test failure rather than a silently different band.
fn ose_role(event_index: usize) -> Result<OseRole, String> {
    let position = OSE_EXHIBITS
        .iter()
        .position(|(e, _)| *e == event_index)
        .ok_or_else(|| {
            format!(
                "event {event_index} is not one of the thirteen recorded {OSE_RUN_ID} \
                 opponent-pending exhibits — the charter's split says nothing about it, so this \
                 surface will not read it"
            )
        })?;
    Ok(if position % 3 == 0 { OseRole::Development } else { OseRole::Reserved })
}

/// The three O columns of the OPPONENT seat, at one exhibit — **or a typed
/// refusal, because this exhibit is escrow-reserved.**
///
/// The refusal is the BURN GUARD. It names the charter row rather than saying
/// "not allowed", so a caller that hits it is told which ruling fenced the read
/// and until when.
fn ose_columns(
    role: OseRole,
    event_index: usize,
    view: &PlayerView,
    opponent: &VisiblePlayer,
    rules: &Rules,
) -> Result<[i64; 3], String> {
    match role {
        OseRole::Reserved => Err(format!(
            "REFUSED: event {event_index} of {OSE_RUN_ID} is one of the EIGHT ESCROW-RESERVED \
             exhibits (guidingpath 2026-08-23, \"THE STEP-5 OSE READ CHARTER IS RULED\"), and \
             that row extends §6's burn rule to them VERBATIM: no table / adjudicate / read of \
             any kind before step 10. Digest-identity verification is permitted here; a column \
             read is not.",
        )),
        OseRole::Development => {
            let structure = ScoreStructure::derive(view, rules);
            let mut out = [0i64; 3];
            for (i, c) in OSE_COLUMNS.iter().enumerate() {
                out[i] = candidate_value(&structure, view, opponent, *c, rules);
            }
            Ok(out)
        }
    }
}

/// One development exhibit, read — or the typed reason it could not be.
struct ExhibitRead {
    event_index: usize,
    digest: String,
    mover: PlayerId,
    outcome: ExhibitOutcome,
}

/// **WHY A REGISTERED ROOT CAN BE UNREADABLE, AND WHY THAT IS NOT A PASS.**
///
/// The charter's read is `opponent-O1` and `opponent-O3` — the OPPONENT's
/// columns AS THE MOVER SEES THEM, which needs the MOVER's `PlayerView`. In an
/// IMPORTED live-tracking record a seat whose dealt starting order was never
/// witnessed carries the `starting_order_hidden` fiction, and `observe` REFUSES
/// to build that seat's own view (`ViewError::ViewerDealUnrecorded`: "a seat
/// cannot be unknown to itself"). Where the mover is such a seat, the registered
/// read has no honest viewer.
///
/// The opponent's OWN view is not a substitute and the difference is the whole
/// point of the column: `observe(state, opponent)` shows the opponent its own
/// masked starting order, while `observe(state, mover)` masks exactly that card
/// — which is the information O1's pool floor is designed around. Reading the
/// opponent's columns off the opponent's own view would be reading privileged
/// information and calling it a view-honest measurement.
///
/// So such a root is NOT RUN, with the reason typed. It is deliberately neither
/// a pass (nothing was measured) nor a fail (nothing was measured), and the
/// verdict counts it separately and exits non-zero: a registered gate root that
/// went unread is an unresolved step-5 condition, and a run that quietly
/// reported four-of-five as green would be a pass about less.
enum ExhibitOutcome {
    Read {
        /// `[O1, O2, O3]` of the OPPONENT seat, from the MOVER's view.
        columns: [i64; 3],
        /// The mover's own three, recorded for contrast — the census's own
        /// finding is that payable visible cards live in the MOVER's hand, and
        /// printing both is what makes the opponent's zero readable rather than
        /// mysterious.
        mover_columns: [i64; 3],
        opponent: PlayerId,
        /// The opponent's hand as the MOVER sees it: `(total, of which KNOWN)`.
        ///
        /// O1 and O3 read KNOWN held orders only, so the visible count is what
        /// makes an opponent zero readable — a hand that is entirely the masked
        /// starting order reads zero on both columns for a reason that has
        /// nothing to do with the columns.
        hand: (usize, usize),
    },
    NotRun {
        reason: String,
    },
}

impl ExhibitRead {
    fn columns(&self) -> Option<[i64; 3]> {
        match &self.outcome {
            ExhibitOutcome::Read { columns, .. } => Some(*columns),
            ExhibitOutcome::NotRun { .. } => None,
        }
    }

    fn passed(&self) -> Option<bool> {
        self.columns().map(|c| c[0] >= 1 && c[2] == 0)
    }
}

/// Load and identity-check the game-3 record `--record` points at.
///
/// Every ledger-recorded key is checked, not only the one the charter names:
/// run id, event count, semantics rev, rules hash and terminal digest. The
/// charter's per-root `state_digest` check then rides on top. A record that
/// disagrees anywhere is a typed refusal — the exhibits are identified by their
/// position IN THIS DOCUMENT, so a different document makes every event index
/// mean something else.
fn load_ose_record(path: &str, rules: &Rules) -> Result<GameRecord, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("--record {path}: {e} (the g3 record regenerates by the `import` \
                              command; records live in the session scratchpad, never committed)"))?;
    let record: GameRecord = serde_json::from_str(&text)
        .map_err(|e| format!("--record {path} is not a GameRecord: {e}"))?;
    check_ose_record_identity(&record, rules).map_err(|e| format!("--record {path} {e}"))?;
    Ok(record)
}

/// The identity half of [`load_ose_record`], over a record already in hand.
///
/// Factored so the refusal is reachable without a file on disk: every shipped
/// call resolves a document the tests may not have, and a check no cell
/// exercises is a check that can be deleted with the suite still green.
fn check_ose_record_identity(record: &GameRecord, rules: &Rules) -> Result<(), String> {
    let run_id = record.run.as_ref().and_then(|r| r.run_id.clone()).unwrap_or_default();
    let mut wrong = Vec::new();
    if run_id != OSE_RUN_ID {
        wrong.push(format!("run_id {run_id:?} != {OSE_RUN_ID:?}"));
    }
    if record.events.len() != OSE_EVENTS {
        wrong.push(format!("events {} != {OSE_EVENTS}", record.events.len()));
    }
    if record.semantics_rev != Some(SEMANTICS_REV) {
        wrong.push(format!("semantics_rev {:?} != Some({SEMANTICS_REV})", record.semantics_rev));
    }
    if record.rules_hash != OSE_RULES_HASH {
        wrong.push(format!("rules_hash {:?} != {OSE_RULES_HASH:?}", record.rules_hash));
    }
    if record.terminal_digest.as_deref() != Some(OSE_TERMINAL_DIGEST) {
        wrong.push(format!(
            "terminal_digest {:?} != Some({OSE_TERMINAL_DIGEST:?})",
            record.terminal_digest,
        ));
    }
    if record.rules_hash != rules.content_hash() {
        wrong.push(format!(
            "rules_hash {:?} is not this build's {:?}",
            record.rules_hash,
            rules.content_hash(),
        ));
    }
    if wrong.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "is not the recorded game-3 document the charter's thirteen exhibit identities index \
             into: {}. REFUSING before any root resolves: every event index in the charter names \
             a position in THAT document.",
            wrong.join("; "),
        ))
    }
}

/// **GATE 4** — the charter's registered development read, at the five dev
/// exhibits.
///
/// Digest FIRST, at all thirteen: identity verification is explicitly permitted
/// at the reserved eight (no root table is read), and verifying them is what
/// makes "the band is the band the ledger recorded" a checked statement rather
/// than an assumption about five of its members. Columns are read at the five
/// only, through [`ose_columns`]' guard.
fn read_ose_band(record: &GameRecord, rules: &Rules) -> Result<BandRead, String> {
    let mut identities = Vec::new();
    let mut reads = Vec::new();
    for (event_index, expected) in OSE_EXHIBITS {
        let role = ose_role(event_index)?;
        let source =
            RootSource::RecordPrefix { record: record.clone(), event_index };
        let (state, descriptor) = source
            .resolve(rules)
            .map_err(|e| format!("{OSE_RUN_ID} event {event_index}: {e:?}"))?;
        if descriptor.root_digest != expected {
            return Err(format!(
                "{OSE_RUN_ID} event {event_index} resolves to state_digest {} and the game-3 \
                 lane-close ledger row recorded {expected} — REFUSING before any column is read",
                descriptor.root_digest,
            ));
        }
        identities.push((event_index, descriptor.root_digest.clone(), role));
        if role == OseRole::Reserved {
            continue;
        }

        let mover = state.current_player;
        let outcome = match observe(&state, mover) {
            // The mover's own seat is unknown to itself in this imported
            // document, so the registered read has no honest viewer here. See
            // [`ExhibitOutcome`] for why the opponent's own view is not a
            // substitute.
            Err(e) => ExhibitOutcome::NotRun { reason: format!("{e}") },
            Ok(view) => {
                let opponent = view
                    .players
                    .iter()
                    .find(|p| p.id != mover && !p.id.is_dummy())
                    .ok_or_else(|| {
                        format!("{OSE_RUN_ID} event {event_index}: no non-dummy opponent seat")
                    })?;
                let me = view.players.iter().find(|p| p.id == mover).ok_or_else(|| {
                    format!("{OSE_RUN_ID} event {event_index}: the mover is not in its own view")
                })?;
                let structure = ScoreStructure::derive(&view, rules);
                let mut mine = [0i64; 3];
                for (i, c) in OSE_COLUMNS.iter().enumerate() {
                    mine[i] = candidate_value(&structure, &view, me, *c, rules);
                }
                ExhibitOutcome::Read {
                    columns: ose_columns(role, event_index, &view, opponent, rules)?,
                    mover_columns: mine,
                    opponent: opponent.id,
                    hand: (
                        opponent.hand.len(),
                        opponent.hand.iter().filter(|c| matches!(c, HandCard::Known(_))).count(),
                    ),
                }
            }
        };
        reads.push(ExhibitRead {
            event_index,
            digest: descriptor.root_digest,
            mover,
            outcome,
        });
    }
    Ok(BandRead { reads, identities })
}

/// What one pass over the thirteen exhibits produced: the five development
/// reads, and the identity verification of ALL thirteen.
struct BandRead {
    reads: Vec<ExhibitRead>,
    /// `(event index, verified digest, role)` for every exhibit — the reserved
    /// eight included, because verifying an identity is not a read.
    identities: Vec<(usize, String, OseRole)>,
}

/// **GATE 3** — the CONSTRUCTED strict-O3 coverage, corpus-free.
///
/// The flag-1 ruling routes strict `opponent-sitting-executable` coverage to the
/// hand-built fixtures, because the strict clause has ZERO members in 792
/// non-terminal positions across all three complete human games — a structural
/// fact about human play, not a gap in the columns. The pair fixtures declare
/// their own O3 (`o3_greater = 1`, `o3_lesser = 0`, `o3_both_sides = 0`) and
/// this reads the COLUMN, at the SUBJECT seat, on the constructed states.
///
/// 4(b) runs at BOTH endpoints because its registration declares both; 4(a) is
/// Early-only and runs where it is declared. Nothing here is a new fixture: the
/// frozen package builds them and this reads them.
struct O3Cell {
    pair: &'static str,
    stage: &'static str,
    side: &'static str,
    expect: &'static str,
    o3: i64,
    ok: bool,
}

fn read_constructed_o3(rules: &Rules) -> Result<Vec<O3Cell>, String> {
    let mut out = Vec::new();
    for (id, sides) in [
        ("4(b)", [(Side::Greater, "EXECUTABLE", ">= 1"), (Side::Lesser, "DISTANT", "= 0")]),
        ("4(a)", [(Side::Greater, "MATCHED", "= 0"), (Side::Lesser, "UNMATCHED", "= 0")]),
    ] {
        let declaration = registered(id).ok_or(format!("`{id}` is not a registered pair"))?;
        for stage in declaration.stages {
            let fixture = build_registered_pair(declaration, *stage, rules)
                .map_err(|e| format!("{id} at {}: {e:?}", stage.name()))?;
            for (side, name, expect) in sides {
                let view = fixture.view(side);
                let subject = view
                    .players
                    .iter()
                    .find(|p| p.id == fixture.subject())
                    .ok_or_else(|| format!("{id} {}: the subject is not in its own view", side.name()))?;
                let structure = ScoreStructure::derive(view, rules);
                let o3 = candidate_value(
                    &structure,
                    view,
                    subject,
                    CandidateFeature::ExecutableOrders,
                    rules,
                );
                out.push(O3Cell {
                    pair: declaration.id,
                    stage: stage.name(),
                    side: name,
                    expect,
                    o3,
                    ok: if expect == ">= 1" { o3 >= 1 } else { o3 == 0 },
                });
            }
        }
    }
    Ok(out)
}

// --- the read-and-record items ---------------------------------------------

/// One development root, resolved and identified, for the reads that are not
/// gates.
struct DevRoot {
    name: String,
    origin: String,
    digest: String,
    state: GameState,
    note: &'static str,
    /// Whether GATE 2 applies here — a TYPED fact carried from the resolution
    /// that knows it, never a name-prefix test at the use site. Which roots a
    /// gate covers is exactly the thing a rename should not be able to change
    /// quietly.
    placement_fork: bool,
}

/// Resolve the §6 development root set for the read-and-record items.
///
/// §6's own list: "`g1-e113-tiemass`, `g3-e93-separated`, the two predicates'
/// development ordinals, and the fired-while-behind root (READ-AND-RECORD ONLY,
/// never a gate)" — with `opponent-sitting-executable`'s generated band STRUCK
/// by the flag-1 ruling and re-scoped to the recorded exhibits above, which is
/// why no OSE ordinal appears here.
fn resolve_dev_roots(generated: bool, rules: &Rules) -> Result<Vec<DevRoot>, String> {
    let mut out = Vec::new();
    let mut push = |name: String,
                    origin: String,
                    source: &RootSource,
                    note,
                    placement_fork|
     -> Result<(), String> {
        let (state, descriptor) = source.resolve(rules).map_err(|e| format!("{name}: {e:?}"))?;
        out.push(DevRoot {
            name,
            origin,
            digest: descriptor.root_digest,
            state,
            note,
            placement_fork,
        });
        Ok(())
    };

    for wanted in [E113_ROOT, "g3-e93-separated"] {
        let entry = registry()
            .into_iter()
            .find(|e| e.name == wanted)
            .ok_or_else(|| format!("{wanted} is not a registry root"))?;
        let source = entry.load(rules).map_err(|e| {
            format!("{wanted}: {e:?} (the owner-game saves are gitignored; absence is a miss)")
        })?;
        push(
            entry.name.to_string(),
            format!("{}:{}", entry.path, entry.event_index),
            &source,
            entry.note,
            false,
        )?;
    }

    if generated {
        for ordinal in M_PF_DEVELOPMENT {
            let source = RootSource::Generated {
                predicate: RootPredicate::PlacementForkSameCount,
                master: M_PF,
                ordinal,
            };
            push(
                format!("m-pf-dev-{ordinal}"),
                format!("gen:placement-fork-same-count:{M_PF:#018X}:{ordinal}"),
                &source,
                "a §6 development ordinal at the recorded M-PF",
                true,
            )?;
        }
    }
    Ok(out)
}

/// **READ-AND-RECORD** — the X wiring proof (spec §6, §16-5).
///
/// Spec §6: "**The X non-vacuity read at step 5 is a RECORDED wiring proof, NOT
/// a gate and NOT a rooted effect** — §8's removal exception (ii) is satisfied
/// for X ONLY by the 10(a)/10(b) carrier reads … a wiring-proof read never
/// satisfies exception (ii) for any feature."
///
/// So this SATISFIES NOTHING. It records that X1/X2/X3 are wired to something
/// that moves at organic positions, and the artifact says in as many words that
/// it neither gates nor counts toward any removal exception.
struct XWiringRead {
    root: String,
    digest: String,
    origin: String,
    note: &'static str,
    /// Per column, `(mover value, opponent value)` — absolute, not differenced:
    /// a wiring proof is about the helper producing a value, and a difference
    /// can be zero with both sides nonzero.
    values: [(i64, i64); 3],
}

impl XWiringRead {
    fn nonzero(&self) -> usize {
        self.values.iter().filter(|(m, o)| *m != 0 || *o != 0).count()
    }
}

/// Read named columns for BOTH seats at one root, from the mover's view.
fn both_seat_columns(
    state: &GameState,
    columns: &[CandidateFeature],
    rules: &Rules,
) -> Result<Vec<(i64, i64)>, String> {
    let mover = state.current_player;
    let view = observe(state, mover).map_err(|e| format!("unobservable: {e}"))?;
    let me = view
        .players
        .iter()
        .find(|p| p.id == mover)
        .ok_or("the mover is not a player in its own view")?;
    let opponent = view.players.iter().find(|p| p.id != mover && !p.id.is_dummy());
    let structure = ScoreStructure::derive(&view, rules);
    Ok(columns
        .iter()
        .map(|c| {
            (
                candidate_value(&structure, &view, me, *c, rules),
                opponent.map_or(0, |o| candidate_value(&structure, &view, o, *c, rules)),
            )
        })
        .collect())
}

/// **READ-AND-RECORD** — the fired-while-behind root's complete candidate
/// columns.
///
/// Checkpoint §7 step 2 registers this root as "**READ-AND-RECORD ONLY — never a
/// gate** (timing is out of scope; gating on it would smuggle the timing layer
/// back in through acceptance)". Every one of the eighteen prints; no verdict
/// line names it.
struct FiredWhileBehindRead {
    origin: String,
    digest: String,
    columns: Vec<(i64, i64)>,
}

/// The step-2 row's recorded identity for that root.
const FIRED_WHILE_BEHIND_DIGEST: &str = "392fe6571d0aa82a";

fn read_fired_while_behind(rules: &Rules) -> Result<FiredWhileBehindRead, String> {
    let source = RootSource::Generated {
        predicate: RootPredicate::EndFirableBehindInTruth,
        master: M_FIRED_WHILE_BEHIND,
        ordinal: FIRED_WHILE_BEHIND_ORDINAL,
    };
    let origin = format!(
        "gen:end-firable-behind-in-truth:{M_FIRED_WHILE_BEHIND:#018X}:{FIRED_WHILE_BEHIND_ORDINAL}"
    );
    let (state, descriptor) =
        source.resolve(rules).map_err(|e| format!("fired-while-behind: {e:?}"))?;
    if descriptor.root_digest != FIRED_WHILE_BEHIND_DIGEST {
        return Err(format!(
            "fired-while-behind resolves to state_digest {} and the step-2 ledger row recorded \
             {FIRED_WHILE_BEHIND_DIGEST} — REFUSING before any column is read",
            descriptor.root_digest,
        ));
    }
    Ok(FiredWhileBehindRead {
        origin,
        digest: descriptor.root_digest,
        columns: both_seat_columns(&state, &CandidateFeature::ALL, rules)?,
    })
}

// --- the command ------------------------------------------------------------

/// The reading this command implements, printed at the top of every artifact.
const ROOTED_HEADER: &str = "\
checkpoint §7 step 5 \"Rooted-read gates at the DEVELOPMENT roots — raw direction
                    and non-collision only. … These roots are TDD/development
                    fixtures, NOT generalization evidence — formula authors see
                    them; the escrow block carries the generalization read
                    (step 10). … A gate failure stops before any fitting.\"
what is NOT here    weights, fits, and any direction beyond the frozen gate
                    texts. Weighted winner-preservation (g3:93) is step 9's,
                    where complete weighted scoring exists
on a failure        this instrument REPORTS and exits non-zero. Repair by
                    re-tuning a gate is fenced: a step-5 failure is a finding";

/// **WHAT COUNTS AT STEP 5**, in one place so it can be asserted rather than
/// read off a hundred lines of printing.
///
/// Every gate result and every recorded read passes through a `take_*` method,
/// and [`Verdict::clears`] is the only thing the exit code reads. Before this
/// type the rule lived inline in the command, where the single cell that could
/// have pinned it would have had to run the whole corpus-gated command to do so.
///
/// # The gate set is the RE-REGISTERED one
///
/// guidingpath 2026-08-23, **"THE TWO STEP-5 FINDINGS ARE RULED"**: the e113
/// DIRECTION claim re-registers forward to step 9(b) and the OSE development
/// band re-registers as {e111, e159, e198}. So the set that gates at step 5 is
/// **GATE-1 non-collision · GATE 2 · GATE 3 · the re-registered GATE 4**, and
/// two categories of result are RECORDED without gating: the e113 direction read
/// (FAILED-AS-REGISTERED, printed in full) and the two exhibits whose mover's
/// deal the imported record never witnessed (NOT-RUN-STRUCTURAL).
///
/// **Recorded is not silent.** Everything in [`Verdict::recorded`] prints in the
/// verdict block, so a run that clears still says out loud what it did not gate
/// on — which is the whole difference between a re-registration and a gate
/// quietly dropped.
struct Verdict {
    /// Findings from gates that COUNT. Non-empty ⇒ the run stops.
    failures: Vec<String>,
    /// Results the ruling re-registered OUT of the step-5 gate set. Printed in
    /// the verdict block, never an input to [`Verdict::clears`].
    recorded: Vec<String>,
}

impl Verdict {
    fn new() -> Self {
        Self { failures: Vec::new(), recorded: Vec::new() }
    }

    /// GATE 1 — the e113 exemplar pair. NON-COLLISION gates; DIRECTION records.
    fn take_e113(&mut self, gate: &E113Gate) {
        if !gate.non_collision() {
            self.failures.push(format!(
                "GATE 1 (e113): {} blocking finding(s) between the registered pair's sides",
                gate.blocking.len(),
            ));
        }
        if !gate.direction_favours_enabling() {
            self.recorded.push(format!(
                "GATE 1 (e113) DIRECTION: FAILED-AS-REGISTERED — the summed delta over all seven \
                 frozen S definitions reads {} where §6 asks for > 0. RE-REGISTERED FORWARD to \
                 step 9(b) (guidingpath 2026-08-23): the fitted identity's WEIGHTED preference \
                 must favour the enabling side over the registered dead side, which is \
                 unit-coherent where an unweighted cross-unit sum is not",
                gate.sum,
            ));
        }
    }

    /// GATE 2 — one placement-fork development ordinal.
    fn take_fork(&mut self, read: &ForkRead) {
        if !read.passed() {
            self.failures.push(format!(
                "GATE 2 ({}): the S signature takes {} distinct value(s) across the witnessing \
                 same-count set (>= 2 required)",
                read.name,
                read.witness_read().map_or(0, |t| t.distinct_vectors),
            ));
        }
    }

    /// GATE 3 — one constructed strict-O3 cell.
    fn take_o3(&mut self, cell: &O3Cell) {
        if !cell.ok {
            self.failures.push(format!(
                "GATE 3 ({} {} {}): O3 = {}, expected {}",
                cell.pair, cell.stage, cell.side, cell.o3, cell.expect,
            ));
        }
    }

    /// GATE 4 — one recorded opponent-pending exhibit.
    ///
    /// A root that READ gates on the charter's clauses. A root the imported
    /// record cannot show a mover's view of is re-registered OUT of the band and
    /// RECORDED: the ruling calls it NOT-RUN-STRUCTURAL, flag-4-style, and it is
    /// the registered shape rather than a stop.
    fn take_exhibit(&mut self, read: &ExhibitRead) {
        match (&read.outcome, read.passed()) {
            (_, Some(true)) => {}
            (ExhibitOutcome::Read { columns, .. }, _) => self.failures.push(format!(
                "GATE 4 (e{}): opponent O1 = {}, O3 = {} (O1 >= 1 and O3 = 0 required)",
                read.event_index, columns[0], columns[2],
            )),
            (ExhibitOutcome::NotRun { reason }, _) => self.recorded.push(format!(
                "GATE 4 (e{}) NOT-RUN-STRUCTURAL: {reason}. RE-REGISTERED OUT of the development \
                 band (guidingpath 2026-08-23) — the band is {{e111, e159, e198}}",
                read.event_index,
            )),
        }
    }

    /// Whether step 5 clears. The ONLY thing the exit code reads.
    ///
    /// [`Verdict::recorded`] is deliberately absent from this expression: a
    /// re-registered result is not a quiet pass, it is a result the owner moved
    /// to a different gate, and the verdict block prints every one of them.
    fn clears(&self) -> bool {
        self.failures.is_empty()
    }
}

fn cmd_rooted(args: &[String]) -> Result<(), String> {
    let (positional, flags, _) = split_args(args, &["record"], &["no-generated"], &[])?;
    if !positional.is_empty() {
        return Err(format!("rooted takes no positional arguments\n\n{USAGE}"));
    }
    let generated = !flags.contains_key("no-generated");

    let rules = Rules::standard();
    let started = Instant::now();
    println!("=== eval_probe rooted [semantics rev {SEMANTICS_REV}] ===");
    println!("{}", ROOTED_HEADER.trim_end());
    println!();
    println!("package digest      {}", package_digest());
    println!("axis width          {AXIS_WIDTH}");
    println!("profile             {}", profile());

    // A registered gate root that could not be evaluated is tracked SEPARATELY
    // from one that evaluated the wrong way. Both stop the run; conflating them
    // would report "nothing was measured" as "the measurement disagreed".
    let mut verdict = Verdict::new();

    // --- GATE 1: the e113 exemplar pair ------------------------------------
    println!();
    println!("--- GATE 1 · g1-e113-tiemass: the S-sum and the pair's non-collision ---");
    println!("{}", E113_SUM_READING.trim_end());
    let gate1 = e113_exemplar(&rules)?;
    println!();
    println!("root digest         {} (ledger {E113_ROOT_DIGEST}) — VERIFIED", gate1.root_digest);
    println!("contestant          {}", gate1.echo);
    println!("decision seed       {:#018x}", gate1.decision_seed);
    println!("candidates          {}", gate1.candidates);
    println!("top value           {}", gate1.top_value);
    println!("tied at top         {}", gate1.tie);
    println!(
        "sites resolved      Laboratory A = AreaId({}) · Fishing Ground = AreaId({}) (from the \
         loaded root's own board)",
        gate1.lab.0, gate1.fishing.0,
    );
    println!("enabling side       {}", gate1.enabling);
    println!("  successor digest  {}", gate1.enabling_digest);
    println!(
        "  registered as     the D7 triage's mechanism fact: the placement that includes the \
         searched line's Fishing Ground, NOT in the top tie"
    );
    println!(
        "  D7 cross-check    shipped-table value {} — {} the top ({}). The triage's own \
         sentence is \"sits at −1,\n                    BELOW the 36-way winning tie\"; this line \
         re-derives it rather than citing it",
        gate1.enabling_value.map_or("ABSENT FROM THE TABLE".to_string(), |v| v.to_string()),
        gate1
            .enabling_value
            .map_or("not comparable to".to_string(), |v| format!("{} below", gate1.top_value - v)),
        gate1.top_value,
    );
    println!("dead side           {}", gate1.dead);
    println!("  successor digest  {}", gate1.dead_digest);
    println!(
        "  criterion         lexicographically-least by canonical encoding among the {} top-tie \
         members placing the same total ({}) into an area set DISJOINT from the enabling side's",
        gate1.eligible, gate1.total,
    );
    println!("  frozen            both sides re-derived from the criterion this run and matched");
    println!();
    for (i, c) in S_COLUMNS.iter().enumerate() {
        println!(
            "  {:<6} enabling {:>6}   dead {:>6}   delta {:>6}",
            c.label(),
            s_slots(&gate1.enabling_dv)[i],
            s_slots(&gate1.dead_dv)[i],
            gate1.per_column[i],
        );
    }
    println!("  {:<6} {:>32}{:>7}   <- THE GATE QUANTITY", "SUM", "", gate1.sum);
    println!();
    println!("{}", E113_ABSOLUTE_NOTE.trim_end());
    println!(
        "  absolute          {}   sum {}",
        S_COLUMNS
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{}={}", c.label(), gate1.absolute_per_column[i]))
            .collect::<Vec<_>>()
            .join(" "),
        gate1.absolute_sum,
    );
    println!(
        "  sign agreement    {}",
        if gate1.sum.signum() == gate1.absolute_sum.signum() {
            "the two currencies AGREE in sign"
        } else {
            "*** the two currencies DISAGREE in sign — a FINDING for the owner ***"
        },
    );
    println!();
    println!(
        "spread (RECORDED, never a gate) — the same sum against EVERY eligible dead-side candidate,\n\
         \x20                   so the verdict reads as a property of the position rather than of\n\
         \x20                   the tie-break that picked one of {} out of the top tie:",
        gate1.spread.len(),
    );
    let favouring = gate1.spread.iter().filter(|(_, d, _)| *d > 0).count();
    let abs_favouring = gate1.spread.iter().filter(|(_, _, a)| *a > 0).count();
    let mut sums: Vec<i64> = gate1.spread.iter().map(|(_, d, _)| *d).collect();
    sums.sort_unstable();
    println!(
        "  differenced       enabling strictly favoured over {favouring} of {} candidates · range \
         [{}, {}] · upper median {}",
        gate1.spread.len(),
        sums.first().copied().unwrap_or(0),
        sums.last().copied().unwrap_or(0),
        sums.get(sums.len() / 2).copied().unwrap_or(0),
    );
    let mut abs_sums: Vec<i64> = gate1.spread.iter().map(|(_, _, a)| *a).collect();
    abs_sums.sort_unstable();
    println!(
        "  absolute          enabling strictly favoured over {abs_favouring} of {} candidates · \
         range [{}, {}] · upper median {}",
        gate1.spread.len(),
        abs_sums.first().copied().unwrap_or(0),
        abs_sums.last().copied().unwrap_or(0),
        abs_sums.get(abs_sums.len() / 2).copied().unwrap_or(0),
    );
    println!(
        "  median convention the UPPER median — element {} of the {}-element sorted list. At an \
         EVEN length\n                    there is no single middle element and no midpoint is \
         interpolated, so the\n                    printed figure is an OBSERVED sum and not a \
         derived statistic",
        gate1.spread.len() / 2,
        gate1.spread.len(),
    );
    for (encoding, diff, abs) in &gate1.spread {
        println!(
            "    {:<52} differenced {:>4}   absolute {:>4}{}",
            encoding,
            diff,
            abs,
            if *encoding == gate1.dead { "   <- REGISTERED dead side" } else { "" },
        );
    }
    println!();
    println!("enabling ppm        {}", gate1.enabling_ppm);
    println!("dead ppm            {}", gate1.dead_ppm);
    println!("enabling dv         {}", render_dv(&gate1.enabling_dv));
    println!("dead dv             {}", render_dv(&gate1.dead_dv));
    println!(
        "non-collision       {} (through §5.3's own classifier and blocking rule — the pair goes \
         in as one root's registered development successor pair, the arm §5.3 names)",
        if gate1.blocking.is_empty() {
            "zero blocking findings".to_string()
        } else {
            gate1.blocking.iter().map(Blocking::label).collect::<Vec<_>>().join("; ")
        },
    );
    println!(
        "GATE 1 direction    {}   RECORDED, NOT A STEP-5 GATE (re-registered to step 9(b))",
        if gate1.direction_favours_enabling() {
            "favours the enabling side"
        } else {
            "*** FAILED-AS-REGISTERED ***"
        },
    );
    println!(
        "GATE 1 non-collision {}   THE step-5 gate half, and it stands",
        if gate1.non_collision() { "PASS" } else { "*** FAIL ***" },
    );
    println!("{}", E113_REREGISTRATION.trim_end());
    verdict.take_e113(&gate1);

    // --- GATE 2: the placement-fork development ordinals -------------------
    println!();
    println!("--- GATE 2 · the placement-fork development ordinals {{0,1,2,3}} at M-PF ---");
    println!("{}", FORK_DISTINCTNESS_READING.trim_end());
    println!();
    let dev_roots = resolve_dev_roots(generated, &rules)?;
    if !generated {
        println!("SKIPPED             --no-generated: the four M-PF ordinals resolve by seeded scan");
    }
    for root in dev_roots.iter().filter(|r| r.placement_fork) {
        let read = read_placement_fork(root, &rules)?;
        println!();
        println!("root                {}", read.name);
        println!("  origin            {}", read.origin);
        println!("  digest            {}", read.digest);
        println!("  witnessing total  {}", read.witness);
        for t in &read.totals {
            println!(
                "  total {:<2}          {} members · S-vectors {} · complete-Δv {} · incumbent-Δv \
                 {}{}",
                t.total,
                t.members,
                t.distinct_vectors,
                t.distinct_complete,
                t.distinct_incumbent,
                if t.total == read.witness { "   <- witness" } else { "" },
            );
            println!(
                "    per column      {}",
                S_COLUMNS
                    .iter()
                    .enumerate()
                    .map(|(i, c)| format!("{}={}", c.label(), t.distinct_per_column[i]))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
        println!(
            "  GATE 2            {}   (would also hold at every other forking total: {})",
            if read.passed() { "PASS" } else { "*** FAIL ***" },
            if read.holds_everywhere() { "yes" } else { "no" },
        );
        verdict.take_fork(&read);
    }

    // --- GATE 3: the constructed strict-O3 coverage ------------------------
    println!();
    println!("--- GATE 3 · CONSTRUCTED strict-O3 coverage (corpus-free) ---");
    println!(
        "the routing         the flag-1 ruling's own text routes strict coverage here: strict \
         opponent-sitting-\n                    executable has ZERO members in 792 non-terminal \
         positions across all three\n                    complete human games, so the constructed \
         fixtures carry it"
    );
    println!();
    for cell in read_constructed_o3(&rules)? {
        println!(
            "  {:<5} {:<5} {:<10} O3 = {:<3} expected {:<5} {}",
            cell.pair,
            cell.stage,
            cell.side,
            cell.o3,
            cell.expect,
            if cell.ok { "PASS" } else { "*** FAIL ***" },
        );
        verdict.take_o3(&cell);
    }

    // --- GATE 4: the recorded development band -----------------------------
    println!();
    println!("--- GATE 4 · the RECORDED opponent-pending development band ---");
    println!("{}", OSE_CHARTER.trim_end());
    println!();
    match flags.get("record") {
        None => println!(
            "SKIPPED             no --record <path>. The five development exhibits are events of \
             the\n                    regenerated {OSE_RUN_ID} record, which lives in the session \
             scratchpad and\n                    is never committed; without it this gate is NOT \
             RUN, and this line says\n                    so rather than a shorter run reading as \
             a pass"
        ),
        Some(path) => {
            let record = load_ose_record(path, &rules)?;
            println!("record              {path}");
            println!(
                "  identity          run_id {OSE_RUN_ID} · {OSE_EVENTS} events · rev \
                 {SEMANTICS_REV} · rules {OSE_RULES_HASH} · terminal {OSE_TERMINAL_DIGEST} — \
                 VERIFIED"
            );
            let BandRead { reads, identities } = read_ose_band(&record, &rules)?;
            println!(
                "  identities        {}/13 exhibit state_digests VERIFIED against the game-3 \
                 lane-close ledger row (permitted at all thirteen: no root table is read)",
                identities.len(),
            );
            println!(
                "  development       {} charter roots: {}   RE-REGISTERED band {{e111, e159, \
                 e198}}",
                identities.iter().filter(|(_, _, r)| *r == OseRole::Development).count(),
                OSE_DEVELOPMENT.iter().map(|e| format!("e{e}")).collect::<Vec<_>>().join(" "),
            );
            println!(
                "  reserved          {} escrow-reserved, columns REFUSED by construction: {}",
                identities.iter().filter(|(_, _, r)| *r == OseRole::Reserved).count(),
                OSE_RESERVED.iter().map(|e| format!("e{e}")).collect::<Vec<_>>().join(" "),
            );
            println!();
            for read in &reads {
                match &read.outcome {
                    ExhibitOutcome::Read { columns, mover_columns, opponent, hand } => {
                        println!(
                            "  e{:<4} digest {} · mover seat {} · opponent seat {} (hand {}, {} \
                             known)",
                            read.event_index,
                            read.digest,
                            read.mover.0,
                            opponent.0,
                            hand.0,
                            hand.1,
                        );
                        println!(
                            "        opponent  O1 = {:<4} O2 = {:<4} O3 = {:<4}   {}",
                            columns[0],
                            columns[1],
                            columns[2],
                            if read.passed() == Some(true) {
                                "PASS (O1 >= 1, O3 = 0)"
                            } else {
                                "*** FAIL ***"
                            },
                        );
                        println!(
                            "        mover     O1 = {:<4} O2 = {:<4} O3 = {:<4}   (contrast, \
                             never gated)",
                            mover_columns[0], mover_columns[1], mover_columns[2],
                        );
                    }
                    ExhibitOutcome::NotRun { reason } => {
                        println!(
                            "  e{:<4} digest {} · mover seat {}",
                            read.event_index, read.digest, read.mover.0,
                        );
                        println!("        NOT-RUN-STRUCTURAL — {reason}");
                        println!(
                            "        RE-REGISTERED OUT of the development band (guidingpath \
                             2026-08-23): the band is\n        {{e111, e159, e198}} and this root \
                             is recorded, flag-4-style, rather than gated.\n        The import \
                             fiction records seat 0's deal as Unknown until its event-73 \
                             reveal\n        (seat 1's is event 14) and both movers here are seat \
                             0, so `observe` refuses with\n        the engine's own typed \
                             `ViewerDealUnrecorded`. The registered read needs the\n        \
                             MOVER's view; the opponent's OWN view is not a substitute — it \
                             prices the exact\n        masked starting order into O1/O2/O3, which \
                             is the card the mover cannot see, and\n        the ruling RATIFIES \
                             refusing it as the honest reading.\n        THE COST, said out loud: \
                             the dev band's early-game half is structurally absent\n        and \
                             position diversity narrows to mid/late"
                        );
                    }
                }
                verdict.take_exhibit(read);
            }
            let readable: Vec<[i64; 3]> = reads.iter().filter_map(ExhibitRead::columns).collect();
            let with_o2 = readable.iter().filter(|c| c[1] >= 1).count();
            println!();
            println!(
                "FINDING             opponent-O2 >= 1 at {with_o2} of {} READ development roots \
                 ({} of {} registered\n                    roots were readable) — the charter's \
                 existential, RECORDED and never gated",
                readable.len(),
                readable.len(),
                reads.len(),
            );
        }
    }

    // --- read-and-record ---------------------------------------------------
    println!();
    println!("--- READ-AND-RECORD · the X wiring proof (spec §6 lines 853-857, §16-5) ---");
    println!(
        "it satisfies        NOTHING. §6: \"The X non-vacuity read at step 5 is a RECORDED WIRING\n\
         \x20                   PROOF, NOT a gate and NOT a rooted effect\" — §8's removal \
         exception (ii)\n\
         \x20                   is satisfied for X ONLY by the 10(a)/10(b) carrier reads. No \
         verdict\n\
         \x20                   line below counts it"
    );
    println!();
    let mut wiring = Vec::new();
    for root in &dev_roots {
        let values = both_seat_columns(&root.state, &X_COLUMNS, &rules)?;
        wiring.push(XWiringRead {
            root: root.name.clone(),
            digest: root.digest.clone(),
            origin: root.origin.clone(),
            note: root.note,
            values: [values[0], values[1], values[2]],
        });
    }
    for read in &wiring {
        println!("  {:<18} {}   {}", read.root, read.digest, read.origin);
        println!("                     {}", read.note);
        println!(
            "                     {}   ({} of 3 columns nonzero at some seat)",
            X_COLUMNS
                .iter()
                .enumerate()
                .map(|(i, c)| format!(
                    "{}=(mover {}, opponent {})",
                    c.label(),
                    read.values[i].0,
                    read.values[i].1
                ))
                .collect::<Vec<_>>()
                .join("  "),
            read.nonzero(),
        );
    }
    for (i, c) in X_COLUMNS.iter().enumerate() {
        let roots = wiring.iter().filter(|r| r.values[i].0 != 0 || r.values[i].1 != 0).count();
        println!(
            "  {:<6} nonzero at {roots} of {} development roots (mover or opponent){}",
            c.label(),
            wiring.len(),
            if roots == 0 {
                "   <- reads ZERO at EVERY development root: this wiring proof is VACUOUS for \
                 this column"
            } else {
                ""
            },
        );
    }
    println!();
    println!("--- READ-AND-RECORD · the fired-while-behind root ---");
    println!(
        "never a gate        checkpoint §7 step 2: \"the fired-while-behind root as READ-AND-RECORD\n\
         \x20                   ONLY — never a gate (timing is out of scope; gating on it would\n\
         \x20                   smuggle the timing layer back in through acceptance)\""
    );
    if generated {
        let read = read_fired_while_behind(&rules)?;
        println!();
        println!("origin              {}", read.origin);
        println!(
            "digest              {} (step-2 ledger {FIRED_WHILE_BEHIND_DIGEST}) — VERIFIED",
            read.digest,
        );
        println!("columns             (mover, opponent), all eighteen:");
        for (i, c) in CandidateFeature::ALL.iter().enumerate() {
            println!(
                "  {:<6} mover {:>6}   opponent {:>6}   mine-theirs {:>6}",
                c.label(),
                read.columns[i].0,
                read.columns[i].1,
                read.columns[i].0 - read.columns[i].1,
            );
        }
    } else {
        println!();
        println!("SKIPPED             --no-generated: this root resolves by seeded scan");
    }

    println!();
    println!("wall                {:.1}s", started.elapsed().as_secs_f64());
    // The RECORDED half prints whether or not the run clears: a re-registered
    // result that only showed up in a failure message would vanish exactly when
    // the run went green, which is when a reader most needs to see it.
    if !verdict.recorded.is_empty() {
        println!();
        println!(
            "RE-REGISTERED       {} result(s) recorded and NOT gated at step 5:",
            verdict.recorded.len(),
        );
        for entry in &verdict.recorded {
            println!("  {entry}");
        }
    }
    println!();
    if verdict.clears() {
        println!("{}", CLEARANCE_BLOCK.trim_end());
        Ok(())
    } else {
        Err(format!(
            "STOP — step 5 does not clear:\n\n{} GATE FAILURE(S) in the RE-REGISTERED gate set \
             (GATE-1 non-collision · GATE 2 ·\nGATE 3 · the re-registered GATE 4):\n  {}\n\n\
             Checkpoint §7 step 5: \"A gate failure stops before any fitting.\" This instrument \
             REPORTS\nrather than repairs. A gate re-tuned to pass is not a gate — repair is \
             OWNER territory\n(checkpoint §6 fences repair-by-scope-expansion).",
            verdict.failures.len(),
            verdict.failures.join("\n  "),
        ))
    }
}

/// The clearance block, printed when the re-registered gate set holds.
///
/// It states which gates cleared, which results were re-registered rather than
/// gated, and WHERE the superseded run lives — a clearance that did not point at
/// the STOP it replaces would read as though step 5 had always been green.
const CLEARANCE_BLOCK: &str = "\
VERDICT             STEP 5 CLEARS UNDER THE RE-REGISTERED GATES
the gate set        GATE-1 non-collision · GATE 2 · GATE 3 · the re-registered
                    GATE 4 — all PASS on the data above
the registration    guidingpath 2026-08-23, \"THE TWO STEP-5 FINDINGS ARE RULED\"
                    (owner, on the independently CONFIRMED lane review): the e113
                    DIRECTION claim re-registers forward to step 9(b), where the
                    fitted identity's WEIGHTED preference over the same registered
                    pair is unit-coherent and falsifiable; the OSE development
                    band re-registers as {e111, e159, e198}
what is NOT claimed step 5 has NO directional cell left. The milestone's first
                    directional evidence moves post-fit, and the ruling records
                    that as the honest COST of the re-registration rather than a
                    detail of it
the superseded run  the STOP-of-record under the ORIGINAL frozen gates — the
                    finding this registration answers — was banked at HEAD
                    3dc1e54 (exit 2). The step-5 close row cites BOTH artifacts";

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

    use yokohama_core::game::greedy::DEFAULT_RETREAT_SAMPLES;
    use yokohama_core::game::lookahead::{DEFAULT_BEAM, DEFAULT_WORLDS};
    use yokohama_core::game::pair_fixtures::{
        build_registered_pair, FeatureId, PairDeclaration, PairFixture, Side, CANDIDATE_PENDING,
        REGISTERED,
    };
    use yokohama_core::{
        apply_move, get_legal_moves, is_game_over, AchievementCard, AchievementType, ChurchSlot,
        check_invariants, ChurchTrack, CustomsSlot, CustomsTrack, GameConfig, HandCard,
        OrderCardId, PendingChoice,
        StartingOrderChoice, TechCardId,
    };

    /// W1-C review NOTE-3: `worlds` and `retreat_samples` are ADJACENT `u8`
    /// parameters on `LookaheadAgent::with_knobs`, so a k/m transposition in
    /// [`lookahead_probe_agent`] would compile clean, echo the PARSED knobs
    /// (the echo prints what was typed, not what the agent holds), and mislabel
    /// every corpus this instrument generates. Pinned through the agent's own
    /// accessors at ASYMMETRIC values — a swap cannot pass — plus the beam and
    /// both cap shapes.
    #[test]
    fn lookahead_knobs_reach_their_agent_slots_unswapped() {
        let rules = Rules::standard();
        let agent = lookahead_probe_agent(&rules, 3, 2, 9, Some(750), WeightTable::V1, 0xA5);
        assert!(
            matches!(agent.scheme(), SearchScheme::TurnBoundary { beam: 3 }),
            "b must land in the beam slot",
        );
        assert_eq!(agent.worlds(), 2, "k must land in the worlds slot");
        assert_eq!(agent.retreat_samples(), 9, "m must land in the retreat-samples slot");
        assert_eq!(agent.cap(), Some(750));

        // The absent-cap shape stays absent (the T0 byte-inertness fact).
        let uncapped = lookahead_probe_agent(
            &rules,
            DEFAULT_BEAM,
            DEFAULT_WORLDS,
            DEFAULT_RETREAT_SAMPLES,
            None,
            WeightTable::V1,
            0xA5,
        );
        assert_eq!(uncapped.cap(), None);
    }

    /// Task C (Phase 3.5c W1): the probe's contestant grammar IS the core one
    /// ([`Contestant::from_str_name`], promoted in W1-B), so a spec typed here
    /// means what it means at `examples/arena` — knobs and refusals included.
    #[test]
    fn the_contestant_grammar_is_the_cores_and_the_old_four_still_parse() {
        assert_eq!(
            ProbeContestant::parse(GREEDY_CONTROLLER),
            Ok(ProbeContestant::Greedy { m: DEFAULT_RETREAT_SAMPLES }),
        );
        assert_eq!(
            ProbeContestant::parse("uniform"),
            Ok(ProbeContestant::Policy(RolloutPolicy::Uniform)),
        );
        assert_eq!(
            ProbeContestant::parse("retreat-averse"),
            Ok(ProbeContestant::Policy(RolloutPolicy::RetreatAverse)),
        );
        assert_eq!(
            ProbeContestant::parse("family-weighted"),
            Ok(ProbeContestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT)),
        );

        // Knobs the probe never had before it delegated — the whole point of
        // single-sourcing rather than growing a second spelling of the grammar.
        assert_eq!(ProbeContestant::parse("greedy-v1:m=3"), Ok(ProbeContestant::Greedy { m: 3 }));
        assert_eq!(
            ProbeContestant::parse("family-weighted:p=50000,k=5"),
            Ok(ProbeContestant::Policy(RolloutPolicy::FamilyWeighted {
                retreat_ppm: 50_000,
                recovery_min_of: 5,
            })),
        );

        // …and the refusals ride along unchanged: unknown IS an error at every
        // level, and the message names what it saw.
        for (spec, token) in
            [("uniform:p=5", "uniform"), ("greedy-v1:zzz=1", "zzz"), ("nope", "nope")]
        {
            let message = ProbeContestant::parse(spec).expect_err("spec must be refused");
            assert!(message.contains(token), "refusal of {spec:?} did not name {token:?}: {message}");
        }
    }

    #[test]
    fn the_lookahead_arm_parses_bare_and_at_every_knob() {
        assert_eq!(
            ProbeContestant::parse(LOOKAHEAD_CONTROLLER),
            Ok(ProbeContestant::Lookahead {
                b: DEFAULT_BEAM,
                k: DEFAULT_WORLDS,
                m: DEFAULT_RETREAT_SAMPLES,
                // An absent cap is the SHIPPED rung, not some large number:
                // every 3.3 figure was measured uncapped.
                cap: None,
            }),
        );
        assert_eq!(
            ProbeContestant::parse("lookahead-v1:b=2,k=3,m=4,cap=750"),
            Ok(ProbeContestant::Lookahead { b: 2, k: 3, m: 4, cap: Some(750) }),
        );
        assert_eq!(
            ProbeContestant::parse("lookahead-v1:b=8"),
            Ok(ProbeContestant::Lookahead {
                b: 8,
                k: DEFAULT_WORLDS,
                m: DEFAULT_RETREAT_SAMPLES,
                cap: None,
            }),
        );
        let zero = ProbeContestant::parse("lookahead-v1:cap=0").expect_err("cap=0 must be refused");
        assert!(zero.contains("omit the knob"), "{zero}");
        assert!(ProbeContestant::parse("lookahead-v1:b=0").is_err(), "b=0 is not a beam");
        assert!(ProbeContestant::parse("lookahead-v1:zzz=1").is_err(), "unknown knob");
    }

    /// The two arms the probe REFUSES, and the reason the refusal is the
    /// probe's rather than the grammar's: the core parser accepts every one of
    /// these specs, and this instrument declines to generate a corpus it cannot
    /// print honest counters for.
    #[test]
    fn the_probe_refuses_the_contestants_it_does_not_meter() {
        for spec in
            ["mcts-v1", "mcts-v1:cap=750", "atomic-diag-v1", "lookahead-atomic-diag-v1:d=3"]
        {
            assert!(
                Contestant::from_str_name(spec).is_ok(),
                "{spec:?} must PARSE — the refusal below is the probe's, not the grammar's",
            );
            let message = ProbeContestant::parse(spec).expect_err("spec must be refused here");
            assert!(message.contains(spec), "the refusal must name what it saw: {message}");
            assert!(
                message.contains("eval_probe does not drive"),
                "the refusal must say whose it is: {message}",
            );
            assert!(
                message.contains("examples/arena"),
                "the refusal must name where the arm DOES run: {message}",
            );
        }
    }

    #[test]
    fn the_echo_names_the_honest_driver_and_omits_an_absent_cap() {
        let weights = WeightTable::V1;
        let greedy = ProbeContestant::Greedy { m: DEFAULT_RETREAT_SAMPLES }.echo(&weights, "v1");
        assert!(greedy.starts_with("greedy-v1{m=8,weights=v1:0x"), "{greedy}");
        assert!(greedy.contains("[honest driver: observe -> sampler K=1 -> handle]"), "{greedy}");

        let bare = ProbeContestant::Lookahead {
            b: DEFAULT_BEAM,
            k: DEFAULT_WORLDS,
            m: DEFAULT_RETREAT_SAMPLES,
            cap: None,
        }
        .echo(&weights, "v1");
        assert!(bare.starts_with("lookahead-v1{b=4,k=1,m=8,weights=v1:0x"), "{bare}");
        assert!(!bare.contains("cap="), "an absent cap must emit no token: {bare}");
        assert!(
            bare.contains("[honest driver: observe -> sampler K=1 -> handle]"),
            "the search arm runs the SAME honest chain the greedy arm does: {bare}",
        );

        let capped = ProbeContestant::Lookahead {
            b: DEFAULT_BEAM,
            k: DEFAULT_WORLDS,
            m: DEFAULT_RETREAT_SAMPLES,
            cap: Some(750),
        }
        .echo(&weights, "v1");
        assert!(capped.contains("cap=750,"), "{capped}");
        assert_ne!(capped, bare);

        // The rung says PRIVILEGED, and it must keep saying it.
        let rung = ProbeContestant::Policy(RolloutPolicy::Uniform).echo(&weights, "v1");
        assert!(rung.contains("RAW-STATE driver: privileged play, corpus only"), "{rung}");
    }

    #[test]
    fn the_per_seat_identity_counts_every_block_including_unpaired_ones() {
        assert_eq!(identity_blocks(&[3, 4], &[3, 4]), (2, 0));
        assert_eq!(identity_blocks(&[3, 4], &[3, 5]), (2, 1));
        assert_eq!(identity_blocks(&[], &[]), (0, 0));
        assert_eq!(identity_blocks(&[3], &[3, 4]), (2, 1));
        // A contestant that stopped handing back tallies at all is exactly the
        // case a zip over the SHORTER list would report as a clean identity
        // over zero blocks — so the unpaired indices count as mismatches.
        assert_eq!(identity_blocks(&[], &[3, 4]), (2, 2));
        assert_eq!(identity_blocks(&[3, 4], &[]), (2, 2));
    }

    /// The lookahead arm end to end: through the HONEST driver (never
    /// `run_raw_game`), with the per-seat §0.3(b) identity holding and the
    /// view-derived blocks — features cells and proxy points — populated for a
    /// search corpus exactly as for a greedy one.
    ///
    /// One game, because a debug-profile S1 game is seconds rather than
    /// milliseconds; the properties are structural, so N buys nothing here.
    #[test]
    fn a_lookahead_corpus_runs_through_the_honest_driver_and_explains_every_move() {
        let rules = Rules::standard();
        let (corpus, master) = generate(
            2,
            1,
            ProbeContestant::Lookahead {
                b: DEFAULT_BEAM,
                k: DEFAULT_WORLDS,
                m: DEFAULT_RETREAT_SAMPLES,
                cap: None,
            },
            &WeightTable::V1,
            &rules,
            SeedScheduleKind::Diagnostic(DEFAULT_MASTER),
            true,
            &[],
            &[],
        )
        .expect("a 2P lookahead game completes");
        assert_eq!(master, DEFAULT_MASTER);
        assert_eq!(corpus.games, 1);

        // (a) the identity, per seat rather than in aggregate.
        assert!(corpus.applied > 0, "a game that applied no moves proves nothing");
        assert_eq!(corpus.identity_blocks, 2, "one block per seat");
        assert_eq!(corpus.identity_mismatches, 0);
        assert_eq!(corpus.decisions, corpus.applied, "every applied move was explained");

        // (b) the search cap counters EXIST on this arm. `Some` is the whole
        // claim — the values are RECORDED, never asserted (the probe is an
        // instrument, not a gate), and `Some(0)` vs `None` is what separates
        // "searched and never hit the guard" from "has no guard to hit".
        assert!(corpus.ply_cap_hits.is_some(), "an S1 corpus has a ply guard to report");
        assert!(corpus.free_cap_hits.is_some(), "an S1 corpus has a free-extension counter");
        assert!(
            corpus.candidates > corpus.decisions,
            "a beam search scores more than one successor per decision",
        );

        // (c) the view-derived blocks work for a search corpus too — verified,
        // not assumed: they read the RECORD the driver handed back, and the
        // record does not know which agent produced it.
        assert!(!corpus.cells.is_empty(), "no analysis cells from a search corpus");
        assert!(corpus.snapshots > 0);
        assert_eq!(
            corpus.proxy.len() * 2,
            corpus.snapshots as usize,
            "one proxy point per turn start, one snapshot per (turn start x seat) at 2P",
        );

        // The discriminating half of the `Option` rule: the one-ply arm has NO
        // such quantity, and answers `None` rather than a zero that would read
        // as a measurement.
        let (greedy, _) = generate(
            2,
            1,
            ProbeContestant::Greedy { m: DEFAULT_RETREAT_SAMPLES },
            &WeightTable::V1,
            &rules,
            SeedScheduleKind::Diagnostic(DEFAULT_MASTER),
            true,
            &[],
            &[],
        )
        .expect("a 2P greedy game completes");
        assert_eq!(greedy.ply_cap_hits, None, "a one-ply argmax has no continuation to truncate");
        assert_eq!(greedy.free_cap_hits, None);
        assert_eq!(greedy.identity_mismatches, 0);
        assert_eq!(greedy.identity_blocks, 2);
    }

    // -----------------------------------------------------------------------
    // Phase 3.5c round 1 — the L2 candidate arm
    // -----------------------------------------------------------------------

    /// The four accepted spellings, and the shape of what they build.
    #[test]
    fn the_score_table_grammar_parses_its_four_forms() {
        assert_eq!(
            parse_score_table("v1"),
            Ok(ScoreTableSpec::named("v1", WeightTable::V1)),
        );
        assert_eq!(
            parse_score_table("v2"),
            Ok(ScoreTableSpec::named("v2", WeightTable::V2)),
        );
        // The rounds' own spelling, kept alive by the alias so every committed
        // command still parses — and it names the SAME numbers `v2` does, which
        // is what the promotion means. The two report under DIFFERENT names on
        // purpose: a block quoting `v2-draft` is reproducing a round, one
        // quoting `v2` is measuring what ships.
        assert_eq!(
            parse_score_table("v2-draft"),
            Ok(ScoreTableSpec::named("v2-draft", WeightTable::V2_DRAFT)),
        );
        assert_eq!(
            parse_score_table("v2-draft").unwrap().table,
            parse_score_table("v2").unwrap().table,
            "the draft spelling and the shipped spelling name one table since the freeze",
        );

        // An explicit vector: sixteen per side, Feature::ALL order, negatives
        // and zeroes included so the parser cannot be a positive-only reader.
        let early: Vec<String> = (0..16).map(|i| (i as i64 - 3).to_string()).collect();
        let late: Vec<String> = (0..16).map(|i| (40 - i as i64).to_string()).collect();
        let spec = format!("A1={}/{}", early.join(","), late.join(","));
        let parsed = parse_score_table(&spec).expect("an explicit vector must parse");
        assert_eq!(parsed.name, "A1");
        assert_eq!(parsed.table.early[0], -3);
        assert_eq!(parsed.table.early[3], 0);
        assert_eq!(parsed.table.late[0], 40);
        assert_eq!(parsed.table.late[15], 25);
        assert_eq!(
            parsed.table.get(Feature::VictoryPoints),
            (-3, 40),
            "slot 0 of each vector is Feature::ALL[0]",
        );
    }

    /// Every refusal names what it SAW. A silently-ignored candidate spec would
    /// print a report missing the row the run was made to look at, which is the
    /// same class of failure `split_args` refuses unknown flags to prevent.
    #[test]
    fn an_unknown_or_malformed_score_table_is_a_typed_error_naming_what_it_saw() {
        let sixteen = (0..16).map(|_| "1").collect::<Vec<_>>().join(",");
        for (spec, token) in [
            // `v3` stands in for the spelling `v2` used to hold here: a table
            // name the grammar does not know is an ERROR, never a silent
            // default. (`v2` itself became legal at the freeze.)
            ("v3", "v3"),
            ("pre-freeze", "pre-freeze"),
            ("", ""),
            ("A1=1,2,3/4,5,6", "A1"),
            ("=1/2", "name"),
        ] {
            let message = parse_score_table(spec).expect_err("{spec:?} must be refused");
            assert!(
                message.contains(token),
                "the refusal of {spec:?} did not name {token:?}: {message}",
            );
        }
        // A vector of the wrong WIDTH names the width it got and the width the
        // feature list has, so a future admission cannot be mistaken for a typo.
        let short = format!("A1={}/{}", sixteen, (0..15).map(|_| "1").collect::<Vec<_>>().join(","));
        let message = parse_score_table(&short).expect_err("15 late entries must be refused");
        assert!(message.contains("15") && message.contains("16"), "{message}");
        // A non-integer entry names the token rather than the whole spec.
        let bad = format!("A1={}/{}", sixteen.replacen('1', "x", 1), sixteen);
        let message = parse_score_table(&bad).expect_err("a non-integer entry must be refused");
        assert!(message.contains('x'), "{message}");
        // The missing separator, both ways round.
        assert!(parse_score_table("A1=1,2,3").is_err(), "no `/` is not a table");
        assert!(parse_score_table("A1/1,2,3").is_err(), "no `=` is not a named table");
    }

    #[test]
    fn an_unknown_candidate_column_is_a_typed_error() {
        assert_eq!(
            CandidateColumn::parse_list("postclaim"),
            Ok(vec![CandidateColumn::PostClaim]),
        );
        let message = CandidateColumn::parse_list("postclim").expect_err("a typo must be refused");
        assert!(message.contains("postclim"), "{message}");
        assert!(message.contains("postclaim"), "the refusal must name what IS accepted: {message}");
        // Feature::ALL is not touched by a candidate column, ever.
        assert_eq!(Feature::COUNT, 16);
        assert_eq!(Row::all(&[CandidateColumn::PostClaim]).len(), Row::all(&[]).len() + 1);
        // …and the benchmark row stays last.
        assert!(matches!(Row::all(&[CandidateColumn::PostClaim]).last(), Some(Row::EvalTotal)));
    }

    /// **The split is a function of the generation index ALONE** — the round-1
    /// pre-registration's words. Pinned on a hand-built corpus so the mapping is
    /// visible rather than inferred: games 0/2/4 are fit at cluster indices
    /// 0/1/2, games 1/3/5 are held-out at cluster indices 0/1/2, and no cell's
    /// VALUE or LABEL moves it.
    #[test]
    fn the_l2_split_is_a_function_of_the_generation_index_alone() {
        let cells: Vec<Cell> = (0..6u32)
            .map(|game| Cell {
                game,
                stage: Stage::Early,
                // Labels and values deliberately correlate with parity: if the
                // split ever read either, this fixture would still look tidy and
                // the assertions below would still have to hold, so they pin the
                // MAPPING rather than a summary of it.
                label: game % 2 == 1,
                rel: vec![f64::from(game)],
                abs: vec![f64::from(game)],
                cand: vec![f64::from(game) * 10.0],
                base: vec![f64::from(game) * 10.0],
            })
            .collect();
        let corpus = test_corpus(cells, 6);

        for game in 0..6u32 {
            let expected = if game % 2 == 0 { Split::Fit } else { Split::HeldOut };
            assert_eq!(Split::of(game), expected, "game {game}");
            assert_eq!(expected.cluster(game), game / 2, "game {game} cluster index");
        }
        assert_eq!(Split::Fit.clusters(6), 3);
        assert_eq!(Split::HeldOut.clusters(6), 3);
        // An ODD game count leaves the extra game on the FIT side.
        assert_eq!(Split::Fit.clusters(7), 4);
        assert_eq!(Split::HeldOut.clusters(7), 3);

        let fit = split_column(&corpus, 0, Stage::Early, Split::Fit);
        let held = split_column(&corpus, 0, Stage::Early, Split::HeldOut);
        assert_eq!(fit.rows.iter().map(|r| r.0).collect::<Vec<_>>(), vec![0.0, 20.0, 40.0]);
        assert_eq!(held.rows.iter().map(|r| r.0).collect::<Vec<_>>(), vec![10.0, 30.0, 50.0]);
        assert_eq!(fit.rows.iter().map(|r| r.1).collect::<Vec<_>>(), vec![0, 1, 2]);
        assert_eq!(held.rows.iter().map(|r| r.1).collect::<Vec<_>>(), vec![0, 1, 2]);
        assert_eq!(fit.clusters, 3);
        assert_eq!(held.clusters, 3);
        assert_eq!(fit.winners, 0, "even games lose in this fixture");
        assert_eq!(held.winners, 3);
    }

    /// **THE ANTI-DRIFT PIN.** `candidate_total` is a hand-written mirror of
    /// `eval::score`'s loop (interpolation, signed-weight opponent selection,
    /// saturating arithmetic, the terminal short-circuit), written that way to
    /// avoid re-deriving a `ScoreStructure` per candidate per view. A mirror
    /// with no pin is a second implementation waiting to disagree, so this
    /// runs the candidate machinery with V1 — the very table the corpus was
    /// scored under — and requires it to reproduce the benchmark EVAL TOTAL
    /// row's cell values EXACTLY, cell by cell, on a real corpus.
    #[test]
    fn the_candidate_machinery_with_v1_reproduces_the_benchmark_row() {
        // BOTH player counts, deliberately: at 2P the signed-weight opponent
        // selection is a no-op (one opponent), so a 2P-only pin leaves
        // `candidate_total`'s max-over-opponents loop structurally unexercised —
        // an inverted selection (max→min) passes it (Task-D review SHOULD-1,
        // mutation-demonstrated). The 4P cell is what makes that mutation fail;
        // `--score-table` itself is not gated to 2P.
        let rules = Rules::standard();
        for players in [2u8, 4] {
            let (corpus, _) = generate(
                players,
                1,
                ProbeContestant::Greedy { m: DEFAULT_RETREAT_SAMPLES },
                &WeightTable::V1,
                &rules,
                SeedScheduleKind::Diagnostic(DEFAULT_MASTER),
                true,
                &[],
                &[ScoreTableSpec::named("v1", WeightTable::V1)],
            )
            .expect("a greedy game completes");

            let slot = corpus
                .rows
                .iter()
                .position(|r| matches!(r, Row::EvalTotal))
                .expect("the benchmark row is always reported");
            assert!(!corpus.cells.is_empty(), "a corpus with no cells proves nothing");
            for cell in &corpus.cells {
                assert_eq!(cell.cand.len(), 1);
                assert_eq!(
                    cell.cand[0].to_bits(),
                    cell.rel[slot].to_bits(),
                    "{players}P game {} stage {:?}: the candidate re-weighting drifted from \
                     `eval::score`",
                    cell.game,
                    cell.stage,
                );
            }
        }
    }

    /// Same arguments, same bytes — the property every recorded figure rests on.
    #[test]
    fn the_candidate_block_is_byte_identical_on_a_second_render() {
        let rules = Rules::standard();
        let candidates = vec![
            ScoreTableSpec::named("v1", WeightTable::V1),
            ScoreTableSpec::named("v2-draft", WeightTable::V2_DRAFT),
        ];
        let (corpus, _) = generate(
            2,
            2,
            ProbeContestant::Greedy { m: DEFAULT_RETREAT_SAMPLES },
            &WeightTable::V1,
            &rules,
            SeedScheduleKind::Diagnostic(DEFAULT_MASTER),
            true,
            &[],
            &candidates,
        )
        .expect("two 2P greedy games complete");

        let first = render_candidate_tables(&corpus, &WeightTable::V1, &candidates, 64, 0xABCD);
        let second = render_candidate_tables(&corpus, &WeightTable::V1, &candidates, 64, 0xABCD);
        assert_eq!(first, second, "the candidate block must not depend on run order");
        // …and it says the things a quoted figure needs beside it.
        for needle in ["fit", "held-out", "EARLY", "MID", "LATE", "v2-draft", "resample_seed"] {
            assert!(first.contains(needle), "the block never mentions {needle:?}:\n{first}");
        }
        // A different candidate stream is a different block.
        let alone = vec![candidates[1].clone()];
        assert_ne!(
            render_candidate_tables(&corpus, &WeightTable::V1, &alone, 64, 0xABCD),
            first,
        );
    }


    // -----------------------------------------------------------------------
    // Phase 3.5c V3 T2 — extended score tables (the §4.1 "17th column")
    // -----------------------------------------------------------------------

    /// The clause grammar `<name>=<base>[+<column>=<early>/<late>]…`, and the
    /// pre-V3 spellings unchanged beside it.
    #[test]
    fn the_score_table_grammar_parses_extension_clauses() {
        // Every pre-V3 spelling still builds exactly what it built before the
        // clause grammar existed — an EMPTY clause list. That is what
        // byte-compatibility means for a committed round command.
        for spec in ["v1", "v2", "v2-draft"] {
            let parsed = parse_score_table(spec).expect("a bare form parses");
            assert!(parsed.extensions.is_empty(), "{spec} must carry no clause");
            assert_eq!(parsed.name, spec);
        }

        // A named base with no clause at all is spellable now, and names the
        // same table under a name of the caller's choosing.
        assert_eq!(
            parse_score_table("probe1=v2").expect("a named base parses"),
            ScoreTableSpec::named("probe1", WeightTable::V2),
        );

        // One clause — the data-contact command's own spec.
        let one = parse_score_table("probe1=v2+terminal_margin_now=4/4").expect("one clause");
        assert_eq!(one.name, "probe1");
        assert_eq!(one.table, WeightTable::V2);
        assert_eq!(one.extensions, vec![(CandidateColumn::TerminalMarginNow, 4, 4)]);

        // A clause on the `v1` base with a NEGATIVE late weight: the base is
        // whatever the grammar names, and a weight pair is signed like any
        // other feature's.
        let on_v1 = parse_score_table("probe2=v1+money=3/-7").expect("a v1 base takes a clause");
        assert_eq!(on_v1.table, WeightTable::V1);
        assert_eq!(on_v1.extensions, vec![(CandidateColumn::Money, 3, -7)]);

        // TWO clauses: stripped from the RIGHT, reported LEFT TO RIGHT.
        let two = parse_score_table("probe3=v2+money=1/2+postclaim=3/4").expect("two clauses");
        assert_eq!(
            two.extensions,
            vec![(CandidateColumn::Money, 1, 2), (CandidateColumn::PostClaim, 3, 4)],
        );

        // A clause on a VECTOR-form base: the greedy right-strip runs first, so
        // the sixteen-per-side base is still there to parse behind it.
        let early = (0..16).map(|i| (i as i64 - 3).to_string()).collect::<Vec<_>>().join(",");
        let late = (0..16).map(|i| (40 - i as i64).to_string()).collect::<Vec<_>>().join(",");
        let on_vector = parse_score_table(&format!("A1={early}/{late}+church_majority_margin=-2/5"))
            .expect("a vector base takes a clause");
        assert_eq!(on_vector.name, "A1");
        assert_eq!(on_vector.table.early[0], -3);
        assert_eq!(on_vector.table.late[15], 25);
        assert_eq!(on_vector.extensions, vec![(CandidateColumn::ChurchMajorityMargin, -2, 5)]);

        // A clause WEIGHT may carry an explicit `+`: after the clause's own `=`
        // the sign is unambiguous, and the right-strip finds the clause behind
        // it rather than mis-segmenting on the inner `+`.
        assert_eq!(
            parse_score_table("probe4=v2+money=+3/4").expect("a signed clause weight").extensions,
            vec![(CandidateColumn::Money, 3, 4)],
        );
    }

    /// Every clause refusal names what it SAW — the file's standing rule, and
    /// the three the V3 spec calls out by name.
    #[test]
    fn an_extension_clause_refusal_names_what_it_saw() {
        // An unknown column names the token AND what IS accepted.
        let message = parse_score_table("probe1=v2+nosuch=1/2").expect_err("an unknown column");
        assert!(message.contains("nosuch"), "{message}");
        assert!(
            message.contains("terminal_margin_now"),
            "the refusal must list what IS accepted: {message}",
        );

        // A malformed clause names the clause it could not read.
        for spec in ["probe1=v2+money=1/x", "probe1=v2+money=1", "probe1=v2+money=/2"] {
            let clause = spec.rsplit_once('+').expect("the fixture has a clause").1;
            let message = parse_score_table(spec).expect_err("a malformed clause is refused");
            assert!(message.contains(clause), "{spec}: the refusal never named {clause:?}: {message}");
        }

        // A leading-`+` VECTOR entry names the AMBIGUITY: with `+` reserved for
        // clause separation, `+1` in a vector is not a signed 1.
        let sixteen = (0..16).map(|_| "1").collect::<Vec<_>>().join(",");
        let message = parse_score_table(&format!("A1=+{sixteen}/{sixteen}"))
            .expect_err("a leading-`+` vector entry is refused");
        assert!(message.contains("+1"), "{message}");
        assert!(message.contains("ambiguous"), "the refusal must name the ambiguity: {message}");

        // An extension needs a NAME: `v2+…` has a base but nothing to report
        // the row under.
        let message = parse_score_table("v2+money=1/2").expect_err("an unnamed extension");
        assert!(message.contains("v2+money=1/2"), "{message}");
        assert!(message.contains("name"), "{message}");

        // A column named TWICE is refused rather than silently summed — two
        // clauses on one column would price it once at the sum and read in the
        // report as two independent terms.
        let message =
            parse_score_table("probe1=v2+money=1/2+money=3/4").expect_err("a duplicate column");
        assert!(message.contains("money"), "{message}");
        assert!(message.to_lowercase().contains("twice"), "{message}");
    }

    /// The three HELD-OUT stage columns of one candidate, on a named arm.
    fn held_columns(corpus: &Corpus, ordinal: usize, arm: Arm) -> [SplitColumn; 3] {
        [
            split_column_arm(corpus, ordinal, Stage::Early, Split::HeldOut, arm),
            split_column_arm(corpus, ordinal, Stage::Mid, Split::HeldOut, arm),
            split_column_arm(corpus, ordinal, Stage::Late, Split::HeldOut, arm),
        ]
    }

    /// **REPRODUCTION PIN (a).** A zero-weight extension is INERT: it must
    /// reproduce its base's row cell for cell, bit for bit — and be quoted
    /// against the same resample stream, which is what keying the stream on the
    /// BASE table buys.
    #[test]
    fn a_zero_weight_extension_reproduces_its_base_row_cell_for_cell() {
        let rules = Rules::standard();
        let candidates = vec![
            parse_score_table("v2").expect("the plain base"),
            parse_score_table("inert=v2+terminal_margin_now=0/0").expect("a zero-weight clause"),
        ];
        let (corpus, _) = generate(
            2,
            4,
            ProbeContestant::Greedy { m: DEFAULT_RETREAT_SAMPLES },
            &WeightTable::V1,
            &rules,
            SeedScheduleKind::Diagnostic(DEFAULT_MASTER),
            true,
            &[],
            &candidates,
        )
        .expect("four 2P greedy games complete");

        assert!(!corpus.cells.is_empty(), "a corpus with no cells proves nothing");
        for cell in &corpus.cells {
            assert_eq!(
                cell.cand[1].to_bits(),
                cell.cand[0].to_bits(),
                "game {} stage {:?}: a 0/0 clause moved the total",
                cell.game,
                cell.stage,
            );
            // …and each candidate's own base arm is its own column when there
            // is nothing priced between them.
            assert_eq!(cell.base[0].to_bits(), cell.cand[0].to_bits());
            assert_eq!(cell.base[1].to_bits(), cell.cand[1].to_bits());
        }

        let seed_plain = candidate_resample_seed(0xABCD, &candidates[0].table);
        let seed_ext = candidate_resample_seed(0xABCD, &candidates[1].table);
        assert_eq!(
            seed_plain, seed_ext,
            "an extension table and its base share one resample stream (review N4)",
        );
        let plain = held_out_intervals(&held_columns(&corpus, 0, Arm::Extended), 64, seed_plain);
        let inert = held_out_intervals(&held_columns(&corpus, 1, Arm::Extended), 64, seed_ext);
        for stage in 0..3 {
            assert_eq!(plain[stage].0.to_bits(), inert[stage].0.to_bits(), "stage {stage} lo");
            assert_eq!(plain[stage].1.to_bits(), inert[stage].1.to_bits(), "stage {stage} hi");
        }
    }

    /// An extension is priced EXACTLY as a feature would be: the interpolated
    /// weight times (viewer − opponent extreme). At 2P the opponent extreme is
    /// the single opponent, so the cell arithmetic is readable against the
    /// column's own relative row.
    #[test]
    fn an_extension_prices_its_column_exactly_as_a_feature_would() {
        let rules = Rules::standard();
        let candidates = vec![parse_score_table("m5=v2+money=5/5").expect("a flat clause")];
        let (corpus, _) = generate(
            2,
            3,
            ProbeContestant::Greedy { m: DEFAULT_RETREAT_SAMPLES },
            &WeightTable::V1,
            &rules,
            SeedScheduleKind::Diagnostic(DEFAULT_MASTER),
            true,
            &[CandidateColumn::Money],
            &candidates,
        )
        .expect("three 2P greedy games complete");

        let slot = corpus
            .rows
            .iter()
            .position(|r| matches!(r, Row::Candidate(CandidateColumn::Money)))
            .expect("the money column was requested");
        let mut moved = 0;
        for cell in &corpus.cells {
            let expected = cell.base[0] + 5.0 * cell.rel[slot];
            assert!(
                (cell.cand[0] - expected).abs() <= 1e-9 * (1.0 + expected.abs()),
                "game {} stage {:?}: {} is not base {} + 5 x rel {}",
                cell.game,
                cell.stage,
                cell.cand[0],
                cell.base[0],
                cell.rel[slot],
            );
            if cell.cand[0] != cell.base[0] {
                moved += 1;
            }
        }
        assert!(moved > 0, "a flat weight of 5 that never moved a cell proves nothing");
    }

    /// **The opponent rule is the MIRROR's, not a plain maximum.** A positive
    /// weight picks the same opponent a plain max does; a NEGATIVE one must
    /// sometimes pick a different seat, which is exactly what fails on an
    /// extension loop that copied `row_values`' convention by mistake.
    #[test]
    fn an_extension_takes_the_signed_weight_opponent_not_the_plain_maximum() {
        let rules = Rules::standard();
        let candidates = vec![
            parse_score_table("pos=v2+money=5/5").expect("a positive clause"),
            parse_score_table("neg=v2+money=-5/-5").expect("a negative clause"),
        ];
        let (corpus, _) = generate(
            4,
            2,
            ProbeContestant::Greedy { m: DEFAULT_RETREAT_SAMPLES },
            &WeightTable::V1,
            &rules,
            SeedScheduleKind::Diagnostic(DEFAULT_MASTER),
            true,
            &[CandidateColumn::Money],
            &candidates,
        )
        .expect("two 4P greedy games complete");

        let slot = corpus
            .rows
            .iter()
            .position(|r| matches!(r, Row::Candidate(CandidateColumn::Money)))
            .expect("the money column was requested");
        let mut disagreements = 0;
        for cell in &corpus.cells {
            let plain_max = cell.base[0] + 5.0 * cell.rel[slot];
            assert!(
                (cell.cand[0] - plain_max).abs() <= 1e-9 * (1.0 + plain_max.abs()),
                "a POSITIVE weight maximizes the raw value, exactly as a plain max does",
            );
            if (cell.cand[1] - (cell.base[1] - 5.0 * cell.rel[slot])).abs() > 1e-6 {
                disagreements += 1;
            }
        }
        assert!(
            disagreements > 0,
            "at 4P a NEGATIVE weight must sometimes select a different opponent than the plain \
             maximum — without a disagreement this pin passes on a plain-max extension loop",
        );
    }

    /// **The §4.2 improvement interval is PAIRED.** A strictly increasing
    /// transform of a column leaves every rank untouched, so the difference is
    /// exactly zero in EVERY resample while each arm's own interval is wide:
    /// two independent bootstraps, subtracted, would report a spread here.
    #[test]
    fn the_improvement_interval_is_a_paired_difference_and_not_two_bootstraps() {
        let column = |f: &dyn Fn(f64) -> f64| {
            let mut rows: Vec<(f64, u32, bool)> =
                (0..12u32).map(|g| (f(f64::from(g)), g, g.is_multiple_of(3))).collect();
            rows.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("finite"));
            let winners = rows.iter().filter(|r| r.2).count();
            SplitColumn { rows, clusters: 12, winners }
        };
        let three = |f: &dyn Fn(f64) -> f64| [column(f), column(f), column(f)];

        let base = three(&|v| v);
        let rank_preserving = three(&|v| 2.0 * v + 1.0);
        let improvement = paired_improvement_intervals(&rank_preserving, &base, 256, 0x51D5);
        for (s, imp) in improvement.iter().enumerate() {
            assert_eq!(imp.point, Some(0.0), "stage {s}: a rank-preserving transform gains nothing");
            assert_eq!(
                (imp.lo, imp.hi),
                (0.0, 0.0),
                "stage {s}: the PAIRED difference of a rank-preserving transform is exactly zero \
                 in every resample",
            );
        }

        // …and the arms are not themselves degenerate, so the zero width above
        // is a property of the PAIRING and not of the fixture.
        let marginal = held_out_intervals(&three(&|v| v), 256, 0x51D5);
        assert!(
            marginal[0].1 > marginal[0].0,
            "the base arm's own interval must have width: {marginal:?}",
        );

        // A column that flips the ranking reads as a real, signed difference
        // with a real interval — the arithmetic is a difference, not a flag.
        let flipped = three(&|v| -v);
        let worse = paired_improvement_intervals(&flipped, &base, 256, 0x51D5);
        assert!(
            worse[0].point.expect("both arms have both classes") > 0.0,
            "this fixture's winners sit LOW, so negating the column improves it: {worse:?}",
        );
        assert!(worse[0].hi > worse[0].lo, "a real difference has a real interval: {worse:?}");

        // An arm with no rows has no AUC, and the improvement says so rather
        // than filling in a zero.
        let empty = || SplitColumn { rows: Vec::new(), clusters: 12, winners: 0 };
        let none = paired_improvement_intervals(&[empty(), empty(), empty()], &base, 256, 0x51D5);
        assert_eq!(none[0].point, None, "an empty arm has no point estimate");
        assert!(none[0].lo.is_nan() && none[0].hi.is_nan(), "…and no interval: {none:?}");
    }

    /// The report path: the clause echo, the improvement column on a NAMED
    /// base, its stated absence on a vector-form base, and a zero-extension
    /// candidate whose rows keep their pre-V3 shape.
    #[test]
    fn an_extension_tables_block_echoes_its_clauses_and_prints_the_improvement_interval() {
        let rules = Rules::standard();
        let sixteen = (0..16).map(|_| "1").collect::<Vec<_>>().join(",");
        let candidates = vec![
            parse_score_table("v2").expect("the plain base"),
            parse_score_table("probe1=v2+terminal_margin_now=4/4").expect("a named base"),
            parse_score_table(&format!("A1={sixteen}/{sixteen}+money=1/2"))
                .expect("a vector base"),
        ];
        let (corpus, _) = generate(
            2,
            4,
            ProbeContestant::Greedy { m: DEFAULT_RETREAT_SAMPLES },
            &WeightTable::V1,
            &rules,
            SeedScheduleKind::Diagnostic(DEFAULT_MASTER),
            true,
            &[],
            &candidates,
        )
        .expect("four 2P greedy games complete");

        let first = render_candidate_tables(&corpus, &WeightTable::V1, &candidates, 64, 0xABCD);
        let second = render_candidate_tables(&corpus, &WeightTable::V1, &candidates, 64, 0xABCD);
        assert_eq!(first, second, "the candidate block must not depend on run order");

        // The clause echo rides beside the early/late echo.
        assert!(first.contains("terminal_margin_now=4/4"), "no clause echo:\n{first}");
        assert!(first.contains("money=1/2"), "no clause echo:\n{first}");
        // The header states what IS paired rather than the old blanket denial.
        assert!(first.contains("paired"), "the header never mentions pairing:\n{first}");

        let i1 = first.find("[1] candidate=v2").expect("the plain row");
        let i2 = first.find("[2] candidate=probe1").expect("the named-base row");
        let i3 = first.find("[3] candidate=A1").expect("the vector-base row");
        let plain = &first[i1..i2];
        let named_base = &first[i2..i3];
        let vector_base = &first[i3..];

        assert!(
            !plain.contains('\u{0394}'),
            "a table with NO clause gains no improvement column:\n{plain}",
        );
        assert!(named_base.contains("\u{0394} vs v2"), "no improvement column:\n{named_base}");
        assert!(
            !vector_base.contains("\u{0394} vs"),
            "a vector-form base has no NAMED base to improve on:\n{vector_base}",
        );
        assert!(
            vector_base.contains("no improvement interval"),
            "an absent improvement column must say why:\n{vector_base}",
        );
    }

    /// A [`Corpus`] carrying nothing but cells — everything the split and the
    /// candidate columns read, and nothing they do not.
    fn test_corpus(cells: Vec<Cell>, games: u32) -> Corpus {
        Corpus {
            rows: Row::all(&[]),
            cells,
            games,
            snapshots: 0,
            proxy: Vec::new(),
            anchor_rel: Vec::new(),
            anchor_vp_product: QuantileSketch::default(),
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
            ply_cap_hits: None,
            free_cap_hits: None,
            budget_binds: None,
            budget_starved: None,
            identity_blocks: 0,
            identity_mismatches: 0,
            wall: 0.0,
        }
    }

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

    // -----------------------------------------------------------------------
    // Phase 3.5c round 3 — the T5/T6/T7 admission columns (an L1 evidence round)
    // -----------------------------------------------------------------------

    /// A real OBSERVED view, from a real setup — the only kind of view this
    /// instrument ever sees in production. Every fixture below mutates one of
    /// these rather than hand-building a `PlayerView`, so no field a column
    /// reads can be a shape `observe` never produces.
    fn observed_view(players: u8, seed: u64, seat: u8, rules: &Rules) -> PlayerView {
        let pending =
            new_game(GameConfig::for_player_count(players), rules, seed).expect("a standard setup");
        let choices: Vec<_> = (0..players).map(|_| StartingOrderChoice::Keep(0)).collect();
        let state = finalize_setup(pending, &choices).expect("the setup finalizes");
        observe(&state, PlayerId(seat)).expect("a seated view")
    }

    /// A 2P view whose two tracks hold NOTHING but the setup dummy (church
    /// faith-7 / customs space-4, invariant I9). The D1 fixture shape
    /// `tests/eval_core.rs` uses, restated view-side.
    fn dummy_view(rules: &Rules) -> PlayerView {
        let mut view = observed_view(2, 7, 0, rules);
        assert!(
            view.church_track.slots.iter().flatten().any(|s| s.player.is_dummy())
                && view.customs_track.slots.iter().flatten().any(|s| s.player.is_dummy()),
            "a 2P setup must place both dummy pawns — finding D1",
        );
        view.church_track.slots.iter_mut().for_each(|slot| {
            *slot = slot.filter(|occupant| occupant.player.is_dummy());
        });
        view.customs_track.slots.iter_mut().for_each(|slot| {
            *slot = slot.filter(|occupant| occupant.player.is_dummy());
        });
        view
    }

    fn seat_of(view: &PlayerView, seat: u8) -> &VisiblePlayer {
        view.players.iter().find(|p| p.id == PlayerId(seat)).expect("the seat is in this view")
    }

    fn seat_mut(view: &mut PlayerView, seat: u8) -> &mut VisiblePlayer {
        view.players.iter_mut().find(|p| p.id == PlayerId(seat)).expect("the seat is in this view")
    }

    fn column_value(column: CandidateColumn, view: &PlayerView, seat: u8, rules: &Rules) -> i64 {
        column.value(view, seat_of(view, seat), rules)
    }

    /// What the SHIPPED structure awards this seat — the quantity every margin
    /// mirror is judged against.
    fn shipped_bonus(view: &PlayerView, seat: u8, feature: Feature, rules: &Rules) -> i64 {
        ScoreStructure::derive(view, rules).value_of(seat_of(view, seat), feature)
    }

    fn place_church(view: &mut PlayerView, faith: u8, seat: u8) {
        let index = ChurchTrack::faith_to_index(faith).expect("a printed faith space");
        view.church_track.slots[index] =
            Some(ChurchSlot { player: PlayerId(seat), arrival_order: faith });
    }

    fn place_customs(view: &mut PlayerView, space: u8, seat: u8) {
        let index = CustomsTrack::space_to_index(space).expect("a printed customs space");
        view.customs_track.slots[index] =
            Some(CustomsSlot { player: PlayerId(seat), arrival_order: space });
    }

    /// Distinct-NAMED technology cards with the requested industry costs. The
    /// registry holds 20 titles across 30 physical cards and a player may own
    /// only ONE card of a given name, so a fixture stacking two copies of one
    /// title would be a state no game can reach.
    fn techs_totalling(rules: &Rules, costs: &[u8]) -> Vec<TechCardId> {
        let cards = rules.techs.iter_sorted_by_id();
        let mut used: Vec<String> = Vec::new();
        let mut out = Vec::new();
        for cost in costs {
            let card = cards
                .iter()
                .find(|c| c.industry_cost == *cost && !used.contains(&c.name))
                .unwrap_or_else(|| panic!("no unused technology of industry cost {cost}"));
            used.push(card.name.clone());
            out.push(card.id);
        }
        out
    }

    /// **THE ANTI-DRIFT PIN for the church margin.** The column is a mirror of
    /// `eval::majority_bonus`'s ranking at finer grain, so the claim it makes —
    /// POSITIVE exactly when this seat holds first place as the board stands —
    /// is checked against what the shipped structure actually awards, case by
    /// case, with the 2P dummy competing throughout (finding D1, rulebook p.9
    /// NOTE; church 6/3, count then RIGHTMOST occupied space).
    ///
    /// Anti-vacuity is built in: the same seat reads 0 / 3 / 6 across the cases
    /// below, so a mirror that ignored the dummy, or one that ignored the
    /// rightmost tie-break, disagrees on a VALUE rather than on an absence.
    #[test]
    fn the_church_margin_is_positive_exactly_when_the_seat_holds_the_six() {
        let rules = Rules::standard();
        let check = |label: &str, view: &PlayerView, margins: [i64; 2], bonuses: [i64; 2]| {
            for seat in 0..2u8 {
                let margin = column_value(CandidateColumn::ChurchMajorityMargin, view, seat, &rules);
                let bonus = shipped_bonus(view, seat, Feature::ChurchBonusNow, &rules);
                assert_eq!(margin, margins[usize::from(seat)], "{label}: seat {seat} margin");
                assert_eq!(bonus, bonuses[usize::from(seat)], "{label}: seat {seat} shipped bonus");
                assert_eq!(
                    margin > 0,
                    bonus == 6,
                    "{label}: seat {seat}: a positive margin must mean first place \
                     (margin {margin}, bonus {bonus})",
                );
            }
        };

        // (i) The dummy alone on the track: it holds first place, awarded to
        //     nobody, and both real seats are one pawn behind it.
        let lone_dummy = dummy_view(&rules);
        check("dummy alone", &lone_dummy, [-2, -2], [0, 0]);

        // (ii) Both real pawns LEFT of faith-7: a three-way count tie the dummy
        //      wins on rightmost, so P1 collects only the second place.
        let mut denied = dummy_view(&rules);
        place_church(&mut denied, 4, 0);
        place_church(&mut denied, 5, 1);
        check("denied", &denied, [-1, -1], [0, 3]);

        // (iii) A count tie between the two real seats, decided by rightmost.
        let mut rightmost = dummy_view(&rules);
        place_church(&mut rightmost, 4, 0);
        place_church(&mut rightmost, 8, 0);
        place_church(&mut rightmost, 5, 1);
        place_church(&mut rightmost, 9, 1);
        check("rightmost tie", &rightmost, [-1, 1], [3, 6]);

        // (iv) A real count gap: the magnitude carries it, at 2 per pawn.
        let mut gap = dummy_view(&rules);
        for faith in [4u8, 5, 6] {
            place_church(&mut gap, faith, 0);
        }
        place_church(&mut gap, 9, 1);
        check("count gap", &gap, [4, -4], [6, 3]);

        // (v) An empty track — including no dummy — is a flat 0 for everyone,
        //     the one shape in which the margin reads exactly zero.
        let mut empty = dummy_view(&rules);
        empty.church_track.slots = [None; 8];
        check("empty", &empty, [0, 0], [0, 0]);

        // (vi) A lone real occupant with the dummy cleared: first place with no
        //      rival at all, and the seat that never placed is not in the
        //      ranking (no second place is awarded to it).
        let mut lone = dummy_view(&rules);
        lone.church_track.slots = [None; 8];
        place_church(&mut lone, 6, 0);
        check("lone occupant", &lone, [2, -2], [6, 0]);
    }

    /// SHOULD-1 (Task-D review): the two arithmetic mutations that survived the
    /// whole branch battery, each pinned by the exact shape the review's
    /// 7,933-position organic harness caught them with.
    ///
    /// (a) The best-RIVAL selection ranks count-tied rivals by RIGHTMOST: seat 0
    /// at faith 8 sits BETWEEN its two count-tied rivals (opponent at 9, dummy
    /// at 7). A selection truncated to count alone keeps whichever rival
    /// iterates last and can flip the tie term to +1 — first place by the
    /// margin's sign contract — while the shipped structure awards seat 0 the
    /// SECOND place. (b) A zero-industry seat is never a tech contender: at the
    /// 3P OPENING (no dummy, no technology anywhere) every margin reads exactly
    /// 0, and a dropped guard turns the clockwise term into a spurious ±1 at
    /// the very early cells the round's L1 reading examines first.
    #[test]
    fn tied_rivals_rank_by_rightmost_and_zero_industry_seats_never_contend() {
        let rules = Rules::standard();

        let mut between = dummy_view(&rules);
        place_church(&mut between, 8, 0);
        place_church(&mut between, 9, 1);
        let margin = column_value(CandidateColumn::ChurchMajorityMargin, &between, 0, &rules);
        let bonus = shipped_bonus(&between, 0, Feature::ChurchBonusNow, &rules);
        assert_eq!(bonus, 3, "fixture: the three-way count tie must award seat 0 second place");
        assert_eq!(
            margin, -1,
            "seat 0's best rival is the RIGHTMOST of its two count-tied rivals (the opponent \
             at faith 9, not the dummy at 7), so the tie term reads against it",
        );

        for seat in 0..3u8 {
            let view = observed_view(3, 77, seat, &rules);
            assert!(
                view.players.iter().all(|p| p.technologies.is_empty()),
                "fixture: a post-setup opening holds no technologies",
            );
            assert_eq!(
                column_value(CandidateColumn::TechMajorityMargin, &view, seat, &rules),
                0,
                "seat {seat}: with no contender anywhere the margin is exactly 0 — never a \
                 clockwise ±1",
            );
        }
    }

    /// The customs half of the same pin (8/4, seven printed spaces). Case (i) is
    /// the engine's own D1 reproducer read through the view: the dummy wins a
    /// COUNT TIE on the rightmost space and drops P0 from 8 VP to 4.
    #[test]
    fn the_customs_margin_is_positive_exactly_when_the_seat_holds_the_eight() {
        let rules = Rules::standard();
        let check = |label: &str, view: &PlayerView, margins: [i64; 2], bonuses: [i64; 2]| {
            for seat in 0..2u8 {
                let margin =
                    column_value(CandidateColumn::CustomsMajorityMargin, view, seat, &rules);
                let bonus = shipped_bonus(view, seat, Feature::CustomsBonusNow, &rules);
                assert_eq!(margin, margins[usize::from(seat)], "{label}: seat {seat} margin");
                assert_eq!(bonus, bonuses[usize::from(seat)], "{label}: seat {seat} shipped bonus");
                assert_eq!(
                    margin > 0,
                    bonus == 8,
                    "{label}: seat {seat}: a positive margin must mean first place \
                     (margin {margin}, bonus {bonus})",
                );
            }
        };

        let mut denied = dummy_view(&rules);
        place_customs(&mut denied, 3, 0);
        place_customs(&mut denied, 1, 1);
        check("dummy wins the rightmost tie", &denied, [-1, -1], [4, 0]);

        let mut beaten = dummy_view(&rules);
        place_customs(&mut beaten, 5, 0);
        place_customs(&mut beaten, 6, 0);
        place_customs(&mut beaten, 1, 1);
        place_customs(&mut beaten, 2, 1);
        check("dummy out-counted", &beaten, [1, -1], [8, 4]);

        let mut empty = dummy_view(&rules);
        empty.customs_track.slots = [None; 7];
        check("empty", &empty, [0, 0], [0, 0]);
    }

    /// **D5 made executable on the candidate side.** At two players the setup
    /// dummy competes in the TECHNOLOGY majority at industry value exactly 7.5
    /// (owner ruling 2026-08-08, rulebook p.9 final-scoring NOTE), which the
    /// engine and `eval.rs` alike represent as DOUBLED value 15 against real
    /// doubled totals that are always even. These two cells straddle that
    /// half-point — industry 7 (doubled 14) loses to it, industry 8 (doubled
    /// 16) beats it — which is the whole content of the ruling.
    #[test]
    fn the_tech_margin_prices_the_2p_dummy_at_doubled_fifteen() {
        let rules = Rules::standard();
        for (costs, industry, margin_me, bonus_me, margin_them) in [
            (vec![3u8, 4], 7u32, -2i64, 5i64, -30i64),
            (vec![3, 5], 8, 2, 10, -32),
        ] {
            let mut view = observed_view(2, 11, 0, &rules);
            let ids = techs_totalling(&rules, &costs);
            let total: u32 = ids
                .iter()
                .map(|id| u32::from(rules.techs.get(*id).expect("a registry card").industry_cost))
                .sum();
            assert_eq!(total, industry, "the fixture must own the industry it claims");
            seat_mut(&mut view, 0).technologies = ids;
            seat_mut(&mut view, 1).technologies = Vec::new();

            assert_eq!(
                column_value(CandidateColumn::TechMajorityMargin, &view, 0, &rules),
                margin_me,
                "industry {industry} against the dummy's 7.5",
            );
            assert_eq!(
                shipped_bonus(&view, 0, Feature::TechBonusNow, &rules),
                bonus_me,
                "industry {industry}: the shipped award",
            );
            // A seat owning NO technology never places — it is not a contender
            // at all — and the margin says so from the other side.
            assert_eq!(
                column_value(CandidateColumn::TechMajorityMargin, &view, 1, &rules),
                margin_them,
            );
            assert_eq!(shipped_bonus(&view, 1, Feature::TechBonusNow, &rules), 0);
            for seat in 0..2u8 {
                let margin = column_value(CandidateColumn::TechMajorityMargin, &view, seat, &rules);
                let bonus = shipped_bonus(&view, seat, Feature::TechBonusNow, &rules);
                assert_eq!(margin > 0, bonus == 10, "seat {seat}: margin {margin}, bonus {bonus}");
            }
        }
    }

    /// The technology tie-break is the official CLOCKWISE one (nearest to the
    /// start player wins), and it is the only term that can decide an
    /// equal-industry ranking. Run at 4P, where no dummy competes, and flipped
    /// so the tie flips in the mirror AND in the shipped award — a fixture whose
    /// answer never moved would pin nothing.
    #[test]
    fn the_tech_margin_breaks_an_equal_value_tie_clockwise_from_the_start_player() {
        let rules = Rules::standard();
        let mut view = observed_view(4, 13, 0, &rules);
        let pair = techs_totalling(&rules, &[5, 5]);
        for player in view.players.iter_mut() {
            player.technologies = Vec::new();
        }
        seat_mut(&mut view, 2).technologies = vec![pair[0]];
        seat_mut(&mut view, 3).technologies = vec![pair[1]];

        // Start player P1 ⇒ distances P2 = 1, P3 = 2, so P2 wins the tie.
        view.config.start_player = PlayerId(1);
        assert_eq!(column_value(CandidateColumn::TechMajorityMargin, &view, 2, &rules), 1);
        assert_eq!(column_value(CandidateColumn::TechMajorityMargin, &view, 3, &rules), -1);
        assert_eq!(shipped_bonus(&view, 2, Feature::TechBonusNow, &rules), 10);
        assert_eq!(shipped_bonus(&view, 3, Feature::TechBonusNow, &rules), 5);
        // A seat with no technology sits a whole doubled value behind.
        assert_eq!(column_value(CandidateColumn::TechMajorityMargin, &view, 0, &rules), -20);
        assert_eq!(shipped_bonus(&view, 0, Feature::TechBonusNow, &rules), 0);

        // Start player P3 ⇒ distances P3 = 0, P2 = 3: the tie flips, both sides.
        view.config.start_player = PlayerId(3);
        assert_eq!(column_value(CandidateColumn::TechMajorityMargin, &view, 3, &rules), 1);
        assert_eq!(column_value(CandidateColumn::TechMajorityMargin, &view, 2, &rules), -1);
        assert_eq!(shipped_bonus(&view, 3, Feature::TechBonusNow, &rules), 10);
        assert_eq!(shipped_bonus(&view, 2, Feature::TechBonusNow, &rules), 5);
    }

    /// The achievement column is SELF-guidance, per the owner's 2026-08-09 canon
    /// batch: achievements are agenda-setters, so what it measures is how near
    /// THIS seat is to its next claim — never how near an opponent is.
    #[test]
    fn achievement_race_proximity_is_the_nearest_unclaimed_gap_negated() {
        let rules = Rules::standard();
        let one_tech = techs_totalling(&rules, &[3]);
        let fresh = || {
            let mut view = observed_view(2, 17, 0, &rules);
            view.achievements = [
                AchievementCard::new(AchievementType::Money10),
                AchievementCard::new(AchievementType::Orders5),
                AchievementCard::new(AchievementType::Technologies4),
            ];
            let me = seat_mut(&mut view, 0);
            me.money = 8; // Money10: gap 2 — the nearest
            me.completed_orders = vec![OrderCardId(1), OrderCardId(2)]; // Orders5: gap 3
            me.technologies = one_tech.clone(); // Technologies4: gap 3
            view
        };
        let proximity = |view: &PlayerView| {
            column_value(CandidateColumn::AchievementRaceProximity, view, 0, &rules)
        };

        let base = fresh();
        assert_eq!(proximity(&base), -2, "the NEAREST unclaimed card sets the column");

        // The viewer's OWN claim drops that leg; the next-nearest takes over.
        let mut self_claimed = fresh();
        self_claimed.achievements[0].claimed_by.push(PlayerId(0));
        assert_eq!(proximity(&self_claimed), -3);

        // An OPPONENT's claim leaves this seat's leg fully LIVE: the engine's
        // own gate refuses only a REPEAT claim by the same seat, so the card is
        // still worth its subsequent value here (`can_claim_achievement`).
        let mut opponent_claimed = fresh();
        opponent_claimed.achievements[0].claimed_by.push(PlayerId(1));
        assert_eq!(proximity(&opponent_claimed), -2);

        // A card standing at full progress reads 0 — the top of the scale.
        let mut claimable = fresh();
        seat_mut(&mut claimable, 0).money = 10;
        assert_eq!(proximity(&claimable), 0);

        // …and OVER-progress does not read positive: the gap is capped, exactly
        // as `achievement_progress_sum`'s legs are.
        let mut hoarding = fresh();
        seat_mut(&mut hoarding, 0).money = 25;
        assert_eq!(proximity(&hoarding), 0);

        // All three claimed by this seat reads 0 as well — a DECIDED collision
        // with the claimable-now reading above, recorded rather than papered
        // over: both are "nothing left to chase", and the state needs three
        // claims by one seat to occur at all.
        let mut swept = fresh();
        for card in swept.achievements.iter_mut() {
            card.claimed_by.push(PlayerId(0));
        }
        assert_eq!(proximity(&swept), 0);
    }

    /// The clock column is CONTROL-shaped, not brake-shaped: the owner's canon
    /// batch inverts the end-trigger question (a behind player keeps building),
    /// so what is worth measuring is how much of the church clock this seat is
    /// itself driving.
    #[test]
    fn clock_control_church_counts_this_seats_own_pawns_and_never_the_dummys() {
        let rules = Rules::standard();
        let mut view = dummy_view(&rules);
        let control =
            |view: &PlayerView, seat: u8| column_value(CandidateColumn::ClockControlChurch, view, seat, &rules);

        // The dummy is on the track and belongs to neither seat.
        assert_eq!(view.church_track.slots.iter().flatten().count(), 1);
        assert_eq!(control(&view, 0), 0);
        assert_eq!(control(&view, 1), 0);

        place_church(&mut view, 4, 0);
        place_church(&mut view, 8, 0);
        place_church(&mut view, 5, 1);
        assert_eq!(control(&view, 0), 2);
        assert_eq!(control(&view, 1), 1);

        // The seats' shares SUM to the end-trigger's own currency — the
        // executable form of "the dummy is not a pawn the trigger tallies"
        // (`ChurchTrack::total_assistants`, finding D1).
        let total: i64 = (0..2u8).map(|seat| control(&view, seat)).sum();
        assert_eq!(total, i64::from(view.church_track.total_assistants()));
        assert_eq!(view.church_track.slots.iter().flatten().count(), 4, "the dummy is still there");
    }

    /// T6's dropped conjunct. BOARD-GLOBAL by construction, which the report
    /// must not be able to hide: the relative form is identically zero for every
    /// seat, and that is pinned here through `row_values` itself — the function
    /// the features block actually reads — rather than asserted about the helper
    /// alone.
    #[test]
    fn port_empty_slots_counts_every_board_and_reads_the_same_from_every_seat() {
        let rules = Rules::standard();
        let mut view = observed_view(2, 19, 0, &rules);
        assert!(view.port_state.port_b.is_none(), "2P plays one Port board");
        let empty =
            |view: &PlayerView, seat: u8| column_value(CandidateColumn::PortEmptySlots, view, seat, &rules);

        view.port_state.port_a.slots = [None; 6];
        assert_eq!(empty(&view, 0), 6);
        assert_eq!(empty(&view, 1), 6);
        view.port_state.port_a.slots[0] = Some(OrderCardId(1));
        view.port_state.port_a.slots[5] = Some(OrderCardId(2));
        assert_eq!(empty(&view, 0), 4);
        assert_eq!(empty(&view, 1), 4, "a board-global quantity reads the same from every seat");

        let rows = [Row::Candidate(CandidateColumn::PortEmptySlots)];
        let mut rel = [0.0f64];
        let mut abs = [0.0f64];
        let mut cand: [f64; 0] = [];
        let mut base: [f64; 0] = [];
        row_values(
            &view,
            &rules,
            &WeightTable::V1,
            &rows,
            &[],
            &mut rel,
            &mut abs,
            &mut cand,
            &mut base,
        );
        assert_eq!(rel[0], 0.0, "the relative form of a board-global column is zero by construction");
        assert_eq!(abs[0], 4.0, "the absolute form is the whole information this column carries");

        // The SECOND Port board counts too — a 2P-only implementation would
        // read 6 where the table holds twelve slots.
        let mut four = observed_view(4, 23, 0, &rules);
        assert!(four.port_state.port_b.is_some(), "4P plays two Port boards");
        four.port_state.port_a.slots = [None; 6];
        four.port_state.port_b.as_mut().expect("port B").slots = [None; 6];
        assert_eq!(empty(&four, 0), 12);
        four.port_state.port_b.as_mut().expect("port B").slots[2] = Some(OrderCardId(3));
        assert_eq!(empty(&four, 0), 11);
    }

    /// T7/T8's candidate, re-tested at rev-7 prices (the Church action now costs
    /// 2 Yen, D1 2026-08-08). Trivial arithmetic, pinned anyway: a column that
    /// silently read the wrong field would be indistinguishable from a null
    /// result at L1, which is exactly the conclusion this round might draw.
    #[test]
    fn money_is_this_seats_yen() {
        let rules = Rules::standard();
        let mut view = observed_view(2, 29, 0, &rules);
        seat_mut(&mut view, 0).money = 7;
        seat_mut(&mut view, 1).money = 13;
        assert_eq!(column_value(CandidateColumn::Money, &view, 0, &rules), 7);
        assert_eq!(column_value(CandidateColumn::Money, &view, 1, &rules), 13);
        seat_mut(&mut view, 0).money = 0;
        assert_eq!(column_value(CandidateColumn::Money, &view, 0, &rules), 0);
    }

    // -----------------------------------------------------------------------
    // V3 C1 — `terminal_margin_now`
    // -----------------------------------------------------------------------

    /// How many applied moves one organic walk below plays before it stops.
    ///
    /// A PREFIX of a real game, not a truncated measurement: every state the
    /// walk visits is reached by uniform-random legal play from a real setup,
    /// which is all an equality pin needs. The cap is what keeps a per-edit
    /// test off the long tail a uniform game has (`policy_diversity` measures
    /// ~9,900 decision points for an uncapped uniform 4P game).
    const WALK_MOVES: usize = 400;

    /// Play one uniform-random organic game prefix, calling `at_turn_start` with
    /// every state that OPENS a turn — the setup state included.
    ///
    /// Hand-rolled because the probe's tests have no state-yielding walk idiom:
    /// `snapshot_game` folds a finished `GameRecord` into cells and yields the
    /// caller no state, and `generate` hands back a `Corpus`. The turn-start
    /// rule is `snapshot_game`'s verbatim — a change of `turn_number` — so the
    /// states these pins visit are the states the instrument samples.
    fn organic_turn_starts(
        players: u8,
        seed: u64,
        rules: &Rules,
        mut at_turn_start: impl FnMut(&GameState),
    ) {
        let pending =
            new_game(GameConfig::for_player_count(players), rules, seed).expect("a standard setup");
        let choices: Vec<_> = (0..players).map(|_| StartingOrderChoice::Keep(0)).collect();
        let mut state = finalize_setup(pending, &choices).expect("the setup finalizes");
        // Its own stream, keyed on the walk's seed: the pick must not perturb
        // any seeded stream the engine itself derives from the setup seed.
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x5741_4c4b);
        let mut last_turn = state.turn_number;
        at_turn_start(&state);
        for _ in 0..WALK_MOVES {
            if is_game_over(&state) {
                break;
            }
            let moves = get_legal_moves(&state, rules);
            assert!(!moves.is_empty(), "a running {players}P game at seed {seed:#x} has no move");
            let mv = moves[rng.gen_range(0..moves.len())].clone();
            state = apply_move(&state, &mv, rules).expect("a move off the legal list applies");
            if state.turn_number != last_turn {
                last_turn = state.turn_number;
                at_turn_start(&state);
            }
        }
    }

    /// **The C1 equality pin (V3 spec §2(a)).** At the MOVER's own view the
    /// column equals `driven::true_margin_now` of the underlying state, over
    /// organic play at 2P, 3P and 4P.
    ///
    /// The two arithmetics are independent. This column maxes
    /// `ScoreStructure::final_total` over the seated non-dummy rivals of a
    /// MASKED `PlayerView`; the reference ranks `final_result`'s official
    /// standings over the FULL state and takes the best-RANKED rival. They
    /// agree for two stated reasons: every component of a scored-now total is
    /// public board state (an opponent's masked starting order scores nothing
    /// while it sits in hand), and the standings sort on total first, so the
    /// best-ranked rival's total IS the maximum total over rivals. Margin 0
    /// stays not-a-win in both and no tie-break enters the margin.
    ///
    /// A failure here is a finding about ONE of the two arithmetics, and the
    /// cycle stops to attribute it before any L1 figure is quoted (spec §2).
    #[test]
    fn the_terminal_margin_column_is_the_selection_side_truth_over_organic_play() {
        let rules = Rules::standard();
        let mut checked = 0usize;
        let mut nonzero = 0usize;
        let walks: [(u8, u64); 6] =
            [(2, 0x51), (2, 0x52), (3, 0x53), (3, 0x54), (4, 0x55), (4, 0x56)];
        for (players, seed) in walks {
            organic_turn_starts(players, seed, &rules, |state| {
                let mover = state.current_player;
                let view = observe(state, mover).expect("a seated view");
                let column =
                    column_value(CandidateColumn::TerminalMarginNow, &view, mover.0, &rules);
                let truth = yokohama_core::game::driven::true_margin_now(state, &rules);
                assert_eq!(
                    column, truth,
                    "{players}P seed {seed:#x} turn {}: the view-side margin and the \
                     selection-side truth disagree",
                    state.turn_number,
                );
                checked += 1;
                if column != 0 {
                    nonzero += 1;
                }
            });
        }
        assert!(checked >= 60, "only {checked} snapshots checked — the walks stopped early");
        assert!(nonzero > 0, "every checked margin was zero, which a stub also satisfies");
    }

    /// **The C1 antisymmetry pin (V3 spec §2(b)).** At 2P the two seats' columns
    /// negate each other exactly: each has exactly one rival, so the two values
    /// are `A − B` and `B − A` of the same two final totals.
    #[test]
    fn the_terminal_margin_column_negates_exactly_between_the_two_2p_seats() {
        let rules = Rules::standard();
        let mut checked = 0usize;
        let mut nonzero = 0usize;
        for seed in [0x61u64, 0x62] {
            organic_turn_starts(2, seed, &rules, |state| {
                let mut seats = [0i64; 2];
                for seat in 0..2u8 {
                    let view = observe(state, PlayerId(seat)).expect("a seated view");
                    seats[usize::from(seat)] =
                        column_value(CandidateColumn::TerminalMarginNow, &view, seat, &rules);
                }
                assert_eq!(
                    seats[0], -seats[1],
                    "seed {seed:#x} turn {}: the two 2P seats do not negate",
                    state.turn_number,
                );
                checked += 1;
                if seats[0] != 0 {
                    nonzero += 1;
                }
            });
        }
        assert!(checked >= 20, "only {checked} snapshots checked — the walks stopped early");
        assert!(nonzero > 0, "every 2P margin was zero, and 0 == -0 passes a stub");
    }

    /// Every variant is typeable, every token is unique, and nothing here moves
    /// `Feature::ALL` or the argument-free row vector.
    #[test]
    fn every_candidate_column_is_reachable_from_the_command_line() {
        for column in CANDIDATE_COLUMNS {
            assert_eq!(
                CandidateColumn::parse_list(column.token()),
                Ok(vec![column]),
                "{column:?} does not parse from its own token",
            );
            if column != CandidateColumn::PostClaim {
                assert_eq!(
                    column.token(),
                    column.name(),
                    "a round-3 column's token is its printed name",
                );
                // Round-1 review NOTE-5 recorded `postclaim`'s 34-character name
                // overflowing the 28-wide name field as cosmetic; the round-3
                // names are inside it, and this keeps them there.
                assert!(column.name().len() <= 28, "{} overflows the name field", column.name());
            }
        }
        let tokens: Vec<&str> = CANDIDATE_COLUMNS.iter().map(|c| c.token()).collect();
        let names: Vec<&str> = CANDIDATE_COLUMNS.iter().map(|c| c.name()).collect();
        for (label, mut list) in [("token", tokens.clone()), ("name", names.clone())] {
            let before = list.len();
            list.sort_unstable();
            list.dedup();
            assert_eq!(list.len(), before, "two candidate columns share a {label}");
        }

        // A list parses in argv order, and an empty entry is skipped.
        assert_eq!(
            CandidateColumn::parse_list("money,church_majority_margin,"),
            Ok(vec![CandidateColumn::Money, CandidateColumn::ChurchMajorityMargin]),
        );

        // An unknown name names what it SAW and everything that IS accepted.
        let message =
            CandidateColumn::parse_list("church_margin").expect_err("a near-miss must be refused");
        assert!(message.contains("church_margin"), "{message}");
        for token in &tokens {
            assert!(message.contains(token), "the refusal never mentions {token:?}: {message}");
        }

        // V3 T1: the registry is NINE, and the C1 column is in it. Spelled as a
        // literal because the loops above are all self-referential — they read
        // the array they are meant to be guarding, so a column dropped from it
        // would take its own coverage with it.
        assert_eq!(CANDIDATE_COLUMNS.len(), 9, "the candidate registry is nine columns at V3");
        assert!(
            CANDIDATE_COLUMNS.contains(&CandidateColumn::TerminalMarginNow),
            "the C1 column is missing from the registry the command line reads",
        );
        assert_eq!(CandidateColumn::TerminalMarginNow.name(), "terminal_margin_now");
        assert_eq!(CandidateColumn::TerminalMarginNow.token(), "terminal_margin_now");

        // The shipped feature list is untouched, the argument-free row vector is
        // the one every recorded appendix was rendered from, and the benchmark
        // row still ends the table with every column present.
        assert_eq!(Feature::COUNT, 16);
        assert_eq!(Row::all(&[]).len(), 18);
        assert_eq!(Row::all(&CANDIDATE_COLUMNS).len(), 18 + CANDIDATE_COLUMNS.len());
        assert!(matches!(Row::all(&CANDIDATE_COLUMNS).last(), Some(Row::EvalTotal)));
        assert_eq!(
            Row::all(&CANDIDATE_COLUMNS)[..17],
            Row::all(&[])[..17],
            "candidate columns must not disturb the sixteen features or the unsplit hand row",
        );
    }

    /// The columns are functions of the VIEW and of nothing else. Built twice
    /// from scratch rather than cloned: two `observe` calls allocate their own
    /// hash maps with their own `RandomState`, so a column that had come to
    /// depend on a map's iteration order reads differently between the two.
    #[test]
    fn the_admission_columns_are_pure_functions_of_the_view() {
        let rules = Rules::standard();
        let build = || {
            let mut view = dummy_view(&rules);
            place_church(&mut view, 4, 0);
            place_church(&mut view, 9, 1);
            place_customs(&mut view, 2, 0);
            seat_mut(&mut view, 0).technologies = techs_totalling(&rules, &[3, 5]);
            seat_mut(&mut view, 0).money = 6;
            seat_mut(&mut view, 1).money = 11;
            view
        };
        let first = build();
        let second = build();
        assert_eq!(first, second, "the two fixtures must be the same view to begin with");
        for column in CANDIDATE_COLUMNS {
            for seat in 0..2u8 {
                let a = column_value(column, &first, seat, &rules);
                let b = column_value(column, &second, seat, &rules);
                let again = column_value(column, &first, seat, &rules);
                assert_eq!(a, b, "{column:?} seat {seat} moved between two identical views");
                assert_eq!(a, again, "{column:?} seat {seat} moved on a second call");
            }
        }
    }

    /// End to end on a real corpus: the columns ride the same cell machinery the
    /// sixteen do, every value is finite, the board-global column's relative
    /// cells are identically zero, and the columns are not all silently zero
    /// (the anti-vacuity that separates a measured null from a stub).
    #[test]
    fn the_admission_columns_ride_a_real_corpus_end_to_end() {
        let rules = Rules::standard();
        let (corpus, _) = generate(
            2,
            1,
            ProbeContestant::Greedy { m: DEFAULT_RETREAT_SAMPLES },
            &WeightTable::V1,
            &rules,
            SeedScheduleKind::Diagnostic(DEFAULT_MASTER),
            true,
            &CANDIDATE_COLUMNS,
            &[],
        )
        .expect("a 2P greedy game completes");

        assert_eq!(corpus.rows.len(), 18 + CANDIDATE_COLUMNS.len());
        assert!(!corpus.cells.is_empty(), "a corpus with no cells proves nothing");
        let slot = |column: CandidateColumn| {
            corpus
                .rows
                .iter()
                .position(|row| matches!(row, Row::Candidate(c) if *c == column))
                .expect("every requested column is a row")
        };
        let port = slot(CandidateColumn::PortEmptySlots);
        for cell in &corpus.cells {
            for value in cell.rel.iter().chain(cell.abs.iter()) {
                assert!(value.is_finite(), "game {} stage {:?}: a non-finite cell", cell.game, cell.stage);
            }
            assert_eq!(cell.rel[port], 0.0, "a board-global column has no seat margin");
            assert!(cell.abs[port] >= 0.0, "an empty-slot count is never negative");
        }
        // The 2P dummy sits on both tracks from setup, so every seat starts a
        // pawn behind in the church ranking — a value no stub produces.
        let church = slot(CandidateColumn::ChurchMajorityMargin);
        assert!(
            corpus.cells.iter().any(|cell| cell.abs[church] != 0.0),
            "the church margin is identically zero across a whole game",
        );
        let money = slot(CandidateColumn::Money);
        assert!(
            corpus.cells.iter().any(|cell| cell.abs[money] > 0.0),
            "no seat held a single yen all game",
        );
    }

    /// **The §4.2 anchor lines, over a real corpus.** One p90 per candidate
    /// column plus the shared VP-term p50, each read off the sketch its line
    /// names, and one observation folded per SNAPSHOT.
    #[test]
    fn the_anchor_stats_report_a_p90_per_candidate_column_and_the_shared_vp_p50() {
        let rules = Rules::standard();
        let columns =
            [CandidateColumn::TerminalMarginNow, CandidateColumn::AchievementRaceProximity];
        let (corpus, _) = generate(
            2,
            1,
            ProbeContestant::Greedy { m: DEFAULT_RETREAT_SAMPLES },
            &WeightTable::V1,
            &rules,
            SeedScheduleKind::Diagnostic(DEFAULT_MASTER),
            true,
            &columns,
            &[],
        )
        .expect("a 2P greedy game completes");

        let lines = render_anchor_stats(&corpus);
        assert_eq!(
            lines.len(),
            2 + columns.len(),
            "header + the shared line + one per column:\n{}",
            lines.join("\n"),
        );
        assert!(lines[0].contains("ANCHOR STATS") && lines[0].contains("nearest-rank"), "{}", lines[0]);
        assert_eq!(render_anchor_stats(&corpus), lines, "the lines must not depend on render order");

        // Exactly one observation per snapshot, and only in the CANDIDATE slots.
        assert!(corpus.snapshots > 0, "a corpus with no snapshots proves nothing");
        assert_eq!(corpus.anchor_vp_product.len() as u64, corpus.snapshots);
        for (slot, row) in corpus.rows.iter().enumerate() {
            let n = corpus.anchor_rel[slot].len() as u64;
            let expected = if matches!(row, Row::Candidate(_)) { corpus.snapshots } else { 0 };
            assert_eq!(n, expected, "{} sketched {n} of {} snapshots", row.name(), corpus.snapshots);
        }

        // Each line reports THAT sketch's own nearest-rank statistic.
        for column in columns {
            let slot = corpus
                .rows
                .iter()
                .position(|r| matches!(r, Row::Candidate(c) if *c == column))
                .expect("every requested column is a row");
            let p90 = corpus.anchor_rel[slot].quantile(9, 10).expect("a sketch with observations");
            let line = lines
                .iter()
                .find(|l| l.contains(column.name()))
                .unwrap_or_else(|| panic!("no line for {}:\n{}", column.name(), lines.join("\n")));
            assert!(line.starts_with("  ANCHOR STATS"), "{line}");
            assert!(line.contains(&format!("p90 = {p90}")), "{line} does not carry p90 {p90}");
            assert!(line.contains(&format!("n = {}", corpus.snapshots)), "{line}");
        }
        let p50 = corpus.anchor_vp_product.quantile(1, 2).expect("a sketch with observations");
        let shared = lines
            .iter()
            .find(|l| l.contains("vp_term_product_v2"))
            .expect("the shared VP-term line");
        assert!(shared.contains(&format!("p50 = {p50}")), "{shared}");

        // Anti-vacuity: sketches of all zeros satisfy every equality above.
        let margin = corpus
            .rows
            .iter()
            .position(|r| matches!(r, Row::Candidate(c) if *c == CandidateColumn::TerminalMarginNow))
            .expect("the C1 column is a row");
        assert!(
            corpus.anchor_rel[margin].quantiles().expect("observations").max > 0,
            "the terminal margin was identically zero across a whole game",
        );
        assert!(p50 > 0, "the VP term product was zero at the median across a whole game");
    }

    /// The anchor lines' own arithmetic, on sketches with hand-derived order
    /// statistics: nearest rank, the right sketch behind each line, and no
    /// lines at all on a corpus that carries no candidate column.
    #[test]
    fn the_anchor_lines_are_nearest_rank_over_the_sketch_each_one_names() {
        let mut corpus = test_corpus(Vec::new(), 0);
        corpus.rows = Row::all(&[CandidateColumn::TerminalMarginNow]);
        corpus.anchor_rel = vec![QuantileSketch::default(); corpus.rows.len()];
        let slot = corpus
            .rows
            .iter()
            .position(|r| matches!(r, Row::Candidate(_)))
            .expect("one candidate row");
        // 1..=10 ascending: nearest rank puts p90 at rank ceil(10 · 9/10) = 9,
        // the 9th smallest of ten — 9 — where an INTERPOLATED p90 of 1..=10
        // would read 9.1. The sketch's rule is nearest rank, and the two
        // statistics are not the same number.
        for v in 1..=10u64 {
            corpus.anchor_rel[slot].observe(v);
        }
        // {4, 2, 6} out of order: p50 is the ceil(3 · 1/2) = 2nd smallest, 4.
        for v in [4u64, 2, 6] {
            corpus.anchor_vp_product.observe(v);
        }

        let lines = render_anchor_stats(&corpus);
        assert_eq!(lines.len(), 3, "{}", lines.join("\n"));
        // Whitespace-insensitive so the pin is on the CONTENT and its order,
        // not on a column width a later row could widen.
        assert_eq!(
            lines[1].split_whitespace().collect::<Vec<_>>(),
            ["ANCHOR", "STATS", "vp_term_product_v2", "(shared)", "p50", "=", "4", "(n", "=", "3)"],
            "{}",
            lines[1],
        );
        assert_eq!(
            lines[2].split_whitespace().collect::<Vec<_>>(),
            ["ANCHOR", "STATS", "terminal_margin_now", "p90", "=", "9", "(n", "=", "10)"],
            "{}",
            lines[2],
        );

        // An empty sketch reports no statistic rather than a zero.
        let mut unobserved = test_corpus(Vec::new(), 0);
        unobserved.rows = Row::all(&[CandidateColumn::Money]);
        unobserved.anchor_rel = vec![QuantileSketch::default(); unobserved.rows.len()];
        let empty = render_anchor_stats(&unobserved);
        assert_eq!(empty.len(), 3, "{}", empty.join("\n"));
        for line in &empty[1..] {
            assert!(line.contains("= --"), "an empty sketch must not print a number: {line}");
        }

        // No candidate column, no lines at all — an argument-free run's report
        // stays byte-identical to every recorded appendix.
        assert!(render_anchor_stats(&test_corpus(Vec::new(), 0)).is_empty());
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

    // -----------------------------------------------------------------------
    // §7 STEP 3 — the candidate-column adapter and the frozen-fixture
    // cross-checks (the step-2 close handover's carry list)
    // -----------------------------------------------------------------------

    /// Every registered (pair, stage) fixture, built once and kept with the
    /// declaration it answers to.
    ///
    /// A construction failure is a PANIC naming the pair rather than a skip:
    /// the package is frozen, so a fixture that stopped building is a finding,
    /// and a sweep that quietly ran over fewer rows would report a clean pass
    /// for the wrong reason.
    fn every_registered_fixture(rules: &Rules) -> Vec<(&'static PairDeclaration, PairFixture)> {
        let mut out = Vec::new();
        for pair in REGISTERED.iter() {
            for stage in pair.stages {
                match build_registered_pair(pair, *stage, rules) {
                    Ok(fixture) => out.push((pair, fixture)),
                    Err(e) => panic!("{} at {}: {e:?}", pair.id, stage.name()),
                }
            }
        }
        out
    }

    /// Every `(fixture, side, seat-row)` triple the registry offers — the
    /// domain every sweep below runs over.
    fn every_cell(
        fixtures: &[(&'static PairDeclaration, PairFixture)],
    ) -> Vec<(&'static str, &'static str, PlayerId)> {
        let mut out = Vec::new();
        for (decl, fixture) in fixtures {
            for side in [Side::Greater, Side::Lesser] {
                for player in &fixture.view(side).players {
                    out.push((decl.id, side.name(), player.id));
                }
            }
        }
        out
    }

    /// **THE ADAPTER PIN** — every one of the eighteen labels answers with its
    /// OWN named helper, and no two arms could be swapped without this cell
    /// noticing.
    ///
    /// The first half is the identity itself, restated arm by arm: a match that
    /// routed `S1` to `track_access` would still return `Some(i64)` for every
    /// label, so "the adapter answers" proves nothing on its own and the helper
    /// each label is supposed to reach has to be named here a second time.
    ///
    /// The second half is what makes the first half discriminating. Restating
    /// the match is only a check if the two sides could disagree, so every
    /// ORDERED PAIR of labels is required to be SEPARATED somewhere in the
    /// registry — some (fixture, side, seat) where the two helpers read
    /// differently. A pair that never separates is a pair whose arms could
    /// trade places undetected, and this cell fails naming it rather than
    /// passing over it.
    #[test]
    fn the_step_three_adapter_answers_every_label_with_its_own_named_helper() {
        let rules = Rules::standard();
        let fixtures = every_registered_fixture(&rules);
        let cells = every_cell(&fixtures);
        assert!(cells.len() >= 100, "only {} cells — the sweep has shrunk", cells.len());

        // Per label, its value at every cell, in one fixed cell order.
        let mut columns: Vec<Vec<i64>> = vec![Vec::new(); CandidateFeature::COUNT];
        for (_, fixture) in &fixtures {
            for side in [Side::Greater, Side::Lesser] {
                let view = fixture.view(side);
                let s = ScoreStructure::derive(view, &rules);
                for player in &view.players {
                    for label in CandidateFeature::ALL {
                        let named = match label {
                            CandidateFeature::ProductionAccess => s.production_access(view, player),
                            CandidateFeature::TrackAccess => s.track_access(view, player),
                            CandidateFeature::AssistantAreasDistinct => {
                                s.assistant_areas_distinct(player)
                            }
                            CandidateFeature::PresidentLocalAccess => {
                                s.president_local_access(player)
                            }
                            CandidateFeature::BankAccess => s.bank_access(view, player),
                            CandidateFeature::EmploymentAgencyAccess => {
                                s.employment_agency_access(view, player)
                            }
                            CandidateFeature::PresidentReachableAreas => {
                                s.president_reachable_areas(player)
                            }
                            CandidateFeature::MoneyStock => s.money_stock(player),
                            CandidateFeature::TradeGoodsStock => s.trade_goods_stock(player),
                            CandidateFeature::BuildingTokensReady => {
                                s.building_tokens_ready(player)
                            }
                            CandidateFeature::HeldOrderValue => s.held_order_value(player, &rules),
                            CandidateFeature::OrderMatchedGoods => {
                                s.order_matched_goods(player, &rules)
                            }
                            CandidateFeature::ExecutableOrders => {
                                s.executable_orders(player, &rules)
                            }
                            CandidateFeature::FlagIconMarginal => {
                                s.flag_icon_marginal(player, &rules)
                            }
                            CandidateFeature::HeldOrderIcons => s.held_order_icons(player, &rules),
                            CandidateFeature::LabTakeable => s.lab_takeable(view, player),
                            CandidateFeature::PortTakeable => s.port_takeable(view, player, &rules),
                            CandidateFeature::MatchedProductionAccess => {
                                s.matched_production_access(view, player, &rules)
                            }
                        };
                        assert_eq!(
                            candidate_columns(view, player, label, &rules),
                            Some(named),
                            "{} is not {}",
                            label.name(),
                            label.name(),
                        );
                        columns[label.index()].push(named);
                    }
                }
            }
        }

        // The separation matrix: no two arms may be interchangeable.
        let mut never_separated: Vec<String> = Vec::new();
        for a in CandidateFeature::ALL {
            for b in CandidateFeature::ALL {
                if a.index() >= b.index() {
                    continue;
                }
                if columns[a.index()] == columns[b.index()] {
                    never_separated.push(format!("{} == {}", a.name(), b.name()));
                }
            }
        }
        assert!(
            never_separated.is_empty(),
            "these label pairs read identically at every registered cell, so the adapter could \
             swap their arms undetected: {never_separated:?}",
        );
    }

    /// One side's candidate vector, in the SUBJECT's own value and the
    /// OPPONENT's, both as read from the subject's view.
    ///
    /// Two vectors rather than the report's single differenced row because
    /// §5.2's registrations are written in BOTH currencies and the distinction
    /// is load-bearing: a declared LEVEL ("O2 = 5 on both sides") is the
    /// subject's own value, while a declared DELTA ("the DIFFERENCED ΔO1 = 15")
    /// is the relative form the solve row reads. Checking a level against a
    /// relative row would pass wherever the opponent happens to read 0 and fail
    /// wherever it does not, for reasons that have nothing to do with the
    /// registration.
    fn candidate_pair(
        fixture: &PairFixture,
        side: Side,
        rules: &Rules,
    ) -> ([i64; CandidateFeature::COUNT], [i64; CandidateFeature::COUNT]) {
        let view = fixture.view(side);
        let subject = fixture.subject();
        let mut own = [0i64; CandidateFeature::COUNT];
        let mut theirs = [0i64; CandidateFeature::COUNT];
        for label in CandidateFeature::ALL {
            let read = |seat: Option<&VisiblePlayer>| {
                seat.map_or(0, |p| {
                    candidate_columns(view, p, label, rules).expect("the adapter never declines")
                })
            };
            own[label.index()] = read(view.players.iter().find(|p| p.id == subject));
            theirs[label.index()] =
                read(view.players.iter().find(|p| p.id != subject && !p.id.is_dummy()));
        }
        (own, theirs)
    }

    /// A registered scalar by key.
    fn param(decl: &PairDeclaration, key: &str) -> Option<i64> {
        decl.parameters.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
    }

    /// The candidate label a `<label>_<side>` parameter key names, e.g. `"o2"`.
    fn label_named(prefix: &str) -> Option<CandidateFeature> {
        CandidateFeature::ALL.into_iter().find(|c| c.label().eq_ignore_ascii_case(prefix))
    }

    /// **CARRY 6 — THE DECLARED CANDIDATE LEVELS, computed.**
    ///
    /// The step-2 close named this the highest-leverage cross-check for one
    /// reason: until this commit no code in the repo computed a candidate
    /// column, so every level a registration declares — 4(a)'s "O2 1 against
    /// 0", 4(b)'s "O2 5 both sides", 4(e)'s "O5 4 against 0" — was a REGISTERED
    /// PREDICTION, re-derived by hand from §3.2's cards and checked by nothing.
    /// 11(d)'s own note says so in as many words: "no code computes O5 until
    /// step 3 … Step 3's candidate columns VERIFY it; until then it is a
    /// registered prediction, and this note is where a mismatch would be
    /// caught."
    ///
    /// **NOTHING IS RE-TYPED HERE.** The expected numbers are read out of the
    /// registrations' own `parameters`, by a KEY GRAMMAR rather than by a table
    /// this cell carries: `<label>_greater` / `<label>_lesser` /
    /// `<label>_both_sides`, plus 4(f)'s before/after axis. A transcription
    /// table would be a third copy of the levels, free to drift from both the
    /// registration and the formula; reading the registry means a level can
    /// only change by moving the digest.
    ///
    /// **4(f)'S BEFORE/AFTER ORIENTATION IS CHECKED, NOT ASSUMED.** Its axis is
    /// the take, not the card, so "after" has to be mapped onto a side. The
    /// registration also declares `hand_before` / `hand_after`, and the mapping
    /// is accepted only when the realized hand sizes agree with it — the
    /// fixture is the witness for the orientation its own levels are read at.
    ///
    /// The sweep is the WHOLE REGISTRY, not family 4 alone: the grammar costs
    /// nothing outside it and 10(d) declares `x3_greater` / `x3_lesser` on
    /// exactly the same shape.
    #[test]
    fn every_registered_candidate_level_is_what_the_columns_compute() {
        let rules = Rules::standard();
        let fixtures = every_registered_fixture(&rules);
        let mut checked = 0usize;
        let mut family_four = 0usize;
        println!(
            "{:<8} {:<6} {:<28} {:>9} {:>9}",
            "pair", "stage", "declared level", "greater", "lesser",
        );
        for (decl, fixture) in &fixtures {
            let (g_own, _) = candidate_pair(fixture, Side::Greater, &rules);
            let (l_own, _) = candidate_pair(fixture, Side::Lesser, &rules);

            // 4(f)'s orientation witness, where the registration carries one.
            let hands_confirm_after_is_greater = match (
                param(decl, "hand_before"),
                param(decl, "hand_after"),
            ) {
                (Some(before), Some(after)) => {
                    let size = |side| fixture.view(side).players.iter()
                        .find(|p| p.id == fixture.subject())
                        .map_or(0, |p| p.hand.len() as i64);
                    assert_eq!(
                        (size(Side::Lesser), size(Side::Greater)),
                        (before, after),
                        "{}: the registered before/after hand sizes do not match the realized \
                         sides, so `_before`/`_after` cannot be oriented",
                        decl.id,
                    );
                    true
                }
                _ => false,
            };

            for (key, declared) in decl.parameters {
                let Some((prefix, suffix)) = key.split_once('_') else { continue };
                let Some(label) = label_named(prefix) else { continue };
                let sides: &[(Side, &str)] = match suffix {
                    "greater" => &[(Side::Greater, "greater")],
                    "lesser" => &[(Side::Lesser, "lesser")],
                    "both_sides" => &[(Side::Greater, "greater"), (Side::Lesser, "lesser")],
                    "after" if hands_confirm_after_is_greater => &[(Side::Greater, "greater")],
                    "before" if hands_confirm_after_is_greater => &[(Side::Lesser, "lesser")],
                    _ => panic!(
                        "{}: `{key}` names {} but no side this cell can orient — a new level \
                         spelling needs an arm here, never a silent skip",
                        decl.id,
                        label.label(),
                    ),
                };
                for (side, name) in sides {
                    let computed = match side {
                        Side::Greater => g_own[label.index()],
                        Side::Lesser => l_own[label.index()],
                    };
                    assert_eq!(
                        computed, *declared,
                        "{} at {}: the registration declares `{key}` = {declared} for the {name} \
                         side, and the computed {} column reads {computed}. THIS IS A STOP: a \
                         declared level and its formula disagree.",
                        decl.id,
                        fixture.stage().name(),
                        label.label(),
                    );
                }
                println!(
                    "{:<8} {:<6} {:<28} {:>9} {:>9}",
                    decl.id,
                    fixture.stage().name(),
                    format!("{key} = {declared}"),
                    g_own[label.index()],
                    l_own[label.index()],
                );
                checked += 1;
                if decl.family == 4 {
                    family_four += 1;
                }
            }
        }
        // ANTI-VACUITY, MEASURED THEN PINNED: the grammar matched the whole
        // registry's supply of level keys — family 4's sixteen across its
        // endpoints (4(b) declares three at each of its two), plus 10(d)'s two.
        assert_eq!(checked, 18, "the level grammar matched {checked} keys, not the registry's 18");
        assert_eq!(family_four, 16, "family 4 declares 16 of them across its endpoints");
    }

    /// **CARRY 6, the half §5.2 wrote in PROSE.** Four family-4 registrations
    /// declare candidate levels in their notes rather than in `parameters` —
    /// 4(c)'s "O2 reads 4 on both sides", 4(g)'s "O6 is therefore the SOLE
    /// mover — 1 against 0" — and a level that lives only in a sentence is
    /// exactly the level a formula can contradict without anything noticing.
    ///
    /// This table IS a transcription, which the parameter sweep above
    /// deliberately is not, so every row carries the words it transcribes. The
    /// alternative — leaving the prose levels unchecked because checking them
    /// costs a copy — would leave the carrier of 4(g) and the whole O2/O3
    /// equalization of 4(c) and 4(e) resting on nothing.
    #[test]
    fn the_family_four_levels_registered_in_prose_are_what_the_columns_compute() {
        use CandidateFeature::{
            ExecutableOrders, FlagIconMarginal, HeldOrderIcons, MatchedProductionAccess,
            OrderMatchedGoods,
        };
        // (pair, label, greater, lesser, the note this row transcribes)
        let declared: &[(&str, CandidateFeature, i64, i64, &str)] = &[
            // 4(a) note 4: "O6: both icon-free. O5: … every attainable marginal
            // is 0 on both sides"; note 5: "X3 IS 0 ON BOTH SIDES BECAUSE THE
            // SEAT STANDS NOWHERE."
            ("4(a)", HeldOrderIcons, 0, 0, "O6: both icon-free"),
            ("4(a)", FlagIconMarginal, 0, 0, "every attainable marginal is 0 on both sides"),
            ("4(a)", MatchedProductionAccess, 0, 0, "X3 IS 0 ON BOTH SIDES"),
            // 4(c) note 5: "At one of each, every leg of the shared (3, 3, 3, 2)
            // vector is stock-capped and O2 reads 4 on both sides. Nothing is
            // executable at one of each … so O3 is 0 on both sides".
            ("4(c)", OrderMatchedGoods, 4, 4, "O2 reads 4 on both sides"),
            ("4(c)", ExecutableOrders, 0, 0, "O3 is 0 on both sides"),
            // 4(e) note 1: "Zero trade-good stocks (O2 = O3 = 0) … O6 equal
            // (0 both)"; and "NO own assistant at any production area
            // (S1 = X3 = 0 …)".
            ("4(e)", OrderMatchedGoods, 0, 0, "Zero trade-good stocks (O2 = O3 = 0)"),
            ("4(e)", ExecutableOrders, 0, 0, "Zero trade-good stocks (O2 = O3 = 0)"),
            ("4(e)", HeldOrderIcons, 0, 0, "O6 equal (0 both)"),
            ("4(e)", MatchedProductionAccess, 0, 0, "S1 = X3 = 0"),
            // 4(f) note 6: "O3 AND O6 ARE HELD AT 0 BY THE REGISTERED CARDS …
            // O5 is 0 on both sides".
            ("4(f)", ExecutableOrders, 0, 0, "O3 AND O6 ARE HELD AT 0"),
            ("4(f)", HeldOrderIcons, 0, 0, "O3 AND O6 ARE HELD AT 0"),
            ("4(f)", FlagIconMarginal, 0, 0, "O5 is 0 on both sides"),
            // 4(g)'s doc: "O6 is therefore the SOLE mover — 1 against 0 — with
            // O2 and O3 held at 0 by the registered zero stocks and O5 held at
            // 0 by the empty realized set".
            ("4(g)", HeldOrderIcons, 1, 0, "O6 is therefore the SOLE mover — 1 against 0"),
            ("4(g)", OrderMatchedGoods, 0, 0, "O2 and O3 held at 0 by the registered zero stocks"),
            ("4(g)", ExecutableOrders, 0, 0, "O2 and O3 held at 0 by the registered zero stocks"),
            ("4(g)", FlagIconMarginal, 0, 0, "O5 held at 0 by the empty realized set"),
        ];

        let rules = Rules::standard();
        let fixtures = every_registered_fixture(&rules);
        let mut seen = 0usize;
        for (decl, fixture) in &fixtures {
            let (g_own, _) = candidate_pair(fixture, Side::Greater, &rules);
            let (l_own, _) = candidate_pair(fixture, Side::Lesser, &rules);
            for (id, label, g, l, note) in declared {
                if *id != decl.id {
                    continue;
                }
                assert_eq!(
                    (g_own[label.index()], l_own[label.index()]),
                    (*g, *l),
                    "{id} at {}: §5.2 registers {} as ({g}, {l}) — \"{note}\" — and the computed \
                     columns read ({}, {}). THIS IS A STOP.",
                    fixture.stage().name(),
                    label.label(),
                    g_own[label.index()],
                    l_own[label.index()],
                );
                seen += 1;
            }
        }
        // Every transcribed row found its fixture; 4(g) declares both endpoints
        // and is therefore checked twice.
        assert_eq!(seen, declared.len() + 4, "{seen} rows matched a fixture");
    }

    /// **CARRY 6's ARITHMETIC HALF — O1's unseen pool against the frozen
    /// package.**
    ///
    /// 4(c) and 4(d) are the two registrations that write O1's estimator out in
    /// numbers: a pool SIZE, an OPPONENT FLOOR per side, and the differenced
    /// row those two produce. The charter calls this the cross-validation that
    /// matters because it checks the pool against the REGISTERED package rather
    /// than against `observe` — `ViewContext`'s two accumulators are re-derived
    /// from the view, and if they counted a card the registration says is boxed
    /// (or missed one it says is face-up) the floor would move and nothing else
    /// in the suite would say so.
    ///
    /// The identity 4(c) states — `ΔO1 = Δfaces + Δfloor_own − Δfloor_opp` — is
    /// checked TERM BY TERM rather than as one subtraction that comes out
    /// right. `Δfloor_own` is 0 by construction on both pairs, and that is the
    /// term with a structural witness: the subject's hand carries no masked
    /// slot at all, so there is no own-side floor read to difference.
    #[test]
    fn the_registered_order_pools_and_floors_are_what_the_view_context_derives() {
        let rules = Rules::standard();
        let fixtures = every_registered_fixture(&rules);
        let mut checked = 0usize;
        for (decl, fixture) in &fixtures {
            let (Some(pool), Some(floor_g), Some(floor_l), Some(delta_o1)) = (
                param(decl, "pool_cards"),
                param(decl, "greater_opponent_floor"),
                param(decl, "lesser_opponent_floor"),
                param(decl, "differenced_delta_o1"),
            ) else {
                continue;
            };

            let mut faces = [0i64; 2];
            let mut opponent = [0i64; 2];
            for (i, side) in [Side::Greater, Side::Lesser].into_iter().enumerate() {
                let view = fixture.view(side);
                let context = ScoreStructure::derive(view, &rules).context;
                assert_eq!(
                    i64::from(context.unseen_orders),
                    pool,
                    "{} {}: the registration declares a {pool}-card unseen pool and the context \
                     derives {}",
                    decl.id,
                    side.name(),
                    context.unseen_orders,
                );
                let declared_floor = if i == 0 { floor_g } else { floor_l };
                assert_eq!(
                    context.unseen_order_floor(),
                    declared_floor,
                    "{} {}: the registration declares an opponent floor of {declared_floor} and \
                     `unseen_order_floor` reads {}. THIS IS A STOP.",
                    decl.id,
                    side.name(),
                    context.unseen_order_floor(),
                );
                // `Δfloor_own = 0` has a structural witness, not an arithmetic
                // one: the subject reads no floor because it holds no masked
                // slot. Where the registration states the term, it says 0.
                let subject = view
                    .players
                    .iter()
                    .find(|p| p.id == fixture.subject())
                    .expect("the subject has a row in its own view");
                assert!(
                    !subject.hand.iter().any(|c| matches!(c, HandCard::HiddenStartingOrder)),
                    "{} {}: the subject carries a masked slot, so Δfloor_own is not structurally 0",
                    decl.id,
                    side.name(),
                );
                if let Some(own_floor) = param(decl, "subject_own_floor_both_sides") {
                    assert_eq!(own_floor, 0, "{}: the registered own-side floor", decl.id);
                }
                let (own, theirs) = candidate_pair(fixture, side, &rules);
                faces[i] = own[CandidateFeature::HeldOrderValue.index()];
                opponent[i] = theirs[CandidateFeature::HeldOrderValue.index()];
            }

            // The opponent's whole O1 IS its masked slot on both pairs, which
            // is what makes `Δfloor_opp` and the opponent's own row the same
            // number — stated as an assert so a future fixture that gave the
            // opponent a Known card would fail here rather than quietly
            // redefining the identity's third term.
            assert_eq!(
                (opponent[0], opponent[1]),
                (floor_g, floor_l),
                "{}: the opponent's O1 is not its masked slot alone",
                decl.id,
            );
            let delta_faces = faces[0] - faces[1];
            let delta_floor_own = 0i64;
            let delta_floor_opp = floor_g - floor_l;
            assert_eq!(
                delta_faces + delta_floor_own - delta_floor_opp,
                delta_o1,
                "{}: ΔO1 = Δfaces + Δfloor_own − Δfloor_opp",
                decl.id,
            );
            // And the identity's value is the row the solve actually reads.
            let relative = (faces[0] - opponent[0]) - (faces[1] - opponent[1]);
            assert_eq!(relative, delta_o1, "{}: the differenced relative O1 row", decl.id);
            println!(
                "{:<8} pool {pool}  floors {floor_g}/{floor_l}  faces {}/{}  ΔO1 {delta_o1}",
                decl.id, faces[0], faces[1],
            );
            checked += 1;

            // Where the registration also names the hand's face total per side,
            // that IS the subject's own O1 — every card in the subject's hand
            // is Known to it.
            if let (Some(g), Some(l)) =
                (param(decl, "greater_hand_faces"), param(decl, "lesser_hand_faces"))
            {
                assert_eq!((faces[0], faces[1]), (g, l), "{}: the registered hand faces", decl.id);
            }
        }
        assert_eq!(checked, 2, "4(c) and 4(d) are the two registrations that write the pool out");
    }

    /// **CARRY 3 — §5.1's expected-nonzero rule, in the direction that could
    /// not be checked until now.**
    ///
    /// The rule has two halves: a LISTED row must move, and a MOVING row must
    /// be listed ("an unlisted nonzero row at run time is a fixture defect").
    /// Over the sixteen incumbents both halves have been enforced since step 2
    /// — `PairFixture::new` refuses an unlisted nonzero incumbent, and
    /// `pair_fixtures`' own `every_registered_pair_builds_at_every_endpoint_it_declares`
    /// calls `listed_incumbent_rows_are_nonzero` for the other direction, so
    /// neither is duplicated here. Over the EIGHTEEN CANDIDATES neither half
    /// could be checked at all, because nothing computed the columns. This is
    /// the whole missing half of the rule, over the whole frozen package.
    ///
    /// The delta is the DIFFERENCED RELATIVE row — the number
    /// `render_pair_report_with` prints and a Part-B solve row constrains — not
    /// the subject's own value, for the reason `PairFixture::incumbent_delta`
    /// gives: the evaluator's contribution is `w · (mine − theirs)`.
    ///
    /// Violations are COLLECTED rather than asserted one at a time. A frozen
    /// package that has drifted has probably drifted in more than one row, and
    /// a first-failure abort would hand the orchestrator one symptom at a time
    /// across as many runs as there are defects.
    #[test]
    fn every_expected_nonzero_candidate_row_moves_and_every_moving_row_is_listed() {
        let rules = Rules::standard();
        let fixtures = every_registered_fixture(&rules);
        let mut listed_but_still: Vec<String> = Vec::new();
        let mut moved_but_unlisted: Vec<String> = Vec::new();
        let mut listed_and_moving = 0usize;
        for (decl, fixture) in &fixtures {
            let (g_own, g_theirs) = candidate_pair(fixture, Side::Greater, &rules);
            let (l_own, l_theirs) = candidate_pair(fixture, Side::Lesser, &rules);
            for label in CandidateFeature::ALL {
                let i = label.index();
                let delta = (g_own[i] - g_theirs[i]) - (l_own[i] - l_theirs[i]);
                let listed = decl.expected_nonzero.contains(&FeatureId::Candidate(label));
                let where_ = format!("{} at {} / {}", decl.id, fixture.stage().name(), label.label());
                match (listed, delta) {
                    (true, 0) => listed_but_still.push(format!("{where_}: listed, delta 0")),
                    (true, _) => listed_and_moving += 1,
                    (false, 0) => {}
                    (false, d) => moved_but_unlisted.push(format!("{where_}: unlisted, delta {d}")),
                }
            }
        }
        assert!(
            listed_but_still.is_empty() && moved_but_unlisted.is_empty(),
            "§5.1's expected-nonzero rule fails on the candidate axis. THIS IS A STOP — a fixture \
             defect is fixed and re-frozen by the orchestrator, never here.\n  listed but still: \
             {listed_but_still:#?}\n  moved but unlisted: {moved_but_unlisted:#?}",
        );
        // ANTI-VACUITY, and EXACT rather than a floor: a run in which no listed
        // candidate row moved anywhere would satisfy both halves trivially, and
        // the registry is FROZEN — how many candidate rows it lists is a
        // property of the digest, so the count is a constant and not a
        // measurement that may drift. 53 (pair, stage) fixtures, 100 listed
        // candidate rows, every one of them moving.
        assert_eq!(fixtures.len(), 53, "the frozen package realizes 53 (pair, stage) fixtures");
        assert_eq!(
            listed_and_moving, 100,
            "the frozen registry lists 100 candidate rows across its fixtures; a different count \
             means a tuple moved, which would have moved the digest",
        );
        println!(
            "{} (pair, stage) fixtures x {} candidate rows: {listed_and_moving} listed and moving, \
             0 defects",
            fixtures.len(),
            CandidateFeature::COUNT,
        );
    }

    /// **CARRY 4 — no candidate column reads the choice queue (lens B NF-3).**
    ///
    /// The claim is 6(a)'s own, written into its registration: "THE QUEUED
    /// CHOICE IS CARRIED, NOT RESOLVED … no row on the thirty-four-wide axis
    /// reads `pending_choices`." It needs checking rather than trusting for a
    /// concrete reason — **`PlayerView` CARRIES `pending_choices`**
    /// (`types/view.rs:119`), so this is not a boundary the signature enforces.
    /// A helper reading it would compile, would be view-honest by every other
    /// measure, and would make a fixture's columns depend on a decision the
    /// registration deliberately did not make.
    ///
    /// The TEXT half scans `eval.rs`'s non-test region for the field name at
    /// all, which subsumes the per-helper form: if the file never spells it, no
    /// helper does, and no head list has to be kept in step here (the
    /// per-helper discipline and its `S_/E_/X_HELPER_HEADS` templates stay in
    /// `tests/eval_core.rs`, where the other §7 scans live). It is a TRIPWIRE
    /// rather than a proof, on `eval_core`'s own stated grounds.
    ///
    /// The BEHAVIOURAL half is the one that cannot be talked around: every
    /// column is recomputed over the whole frozen package with the queue
    /// EMPTIED, with a one-element queue and with a three-element one, and
    /// every one of the eighteen must read identically at every seat. Editing
    /// the VIEW rather than rebuilding the state is deliberate — it isolates
    /// the field exactly, with no second difference riding along and no
    /// question about whether the drained state is constructible.
    ///
    /// BOTH HALVES MUTATION-PROVEN, and separately, since the text half runs
    /// first and would otherwise mask the other. `production_access` was given
    /// `let mut total = view.pending_choices.len() as i64;`:
    ///   - text half:        FAILED — "eval.rs names `pending_choices` outside
    ///     its test module".
    ///   - behavioural half, with the text assert temporarily relaxed: FAILED —
    ///     "1(a) greater seat 0: S1 moves when the choice queue is replaced by
    ///     [AnyGood]".
    #[test]
    fn no_candidate_column_reads_the_choice_queue() {
        // The text half.
        const SRC: &str = include_str!("../src/game/eval.rs");
        let non_test = SRC.split("#[cfg(test)]").next().expect("eval.rs has a test module");
        assert!(
            !non_test.contains("pending_choices"),
            "eval.rs names `pending_choices` outside its test module — a candidate column that \
             read the queue would make a fixture's row depend on a decision its registration \
             deliberately left unresolved (6(a)'s note)",
        );
        // ANTI-VACUITY: the needle is a real spelling of a real field.
        const VIEW_SRC: &str = include_str!("../src/types/view.rs");
        assert!(
            VIEW_SRC.contains("pub pending_choices"),
            "the scanned needle no longer names a field of `PlayerView`",
        );

        // The behavioural half.
        let rules = Rules::standard();
        let fixtures = every_registered_fixture(&rules);
        let queues: [Vec<PendingChoice>; 3] = [
            Vec::new(),
            vec![PendingChoice::AnyGood],
            vec![
                PendingChoice::WarehouseTake,
                PendingChoice::AssistantMove,
                PendingChoice::StationPlace,
            ],
        ];
        let mut carried_a_queue: Vec<String> = Vec::new();
        for (decl, fixture) in &fixtures {
            for side in [Side::Greater, Side::Lesser] {
                let view = fixture.view(side);
                if !view.pending_choices.is_empty() {
                    carried_a_queue.push(format!(
                        "{} {} ({:?})",
                        decl.id,
                        side.name(),
                        view.pending_choices,
                    ));
                }
                for queue in &queues {
                    let mut edited = view.clone();
                    edited.pending_choices.clone_from(queue);
                    let s = ScoreStructure::derive(&edited, &rules);
                    let base = ScoreStructure::derive(view, &rules);
                    for player in &view.players {
                        let mirror = edited
                            .players
                            .iter()
                            .find(|p| p.id == player.id)
                            .expect("the edit touched only the queue");
                        for label in CandidateFeature::ALL {
                            assert_eq!(
                                candidate_columns(&edited, mirror, label, &rules),
                                candidate_columns(view, player, label, &rules),
                                "{} {} seat {}: {} moves when the choice queue is replaced by \
                                 {queue:?}",
                                decl.id,
                                side.name(),
                                player.id.0,
                                label.label(),
                            );
                        }
                    }
                    // The context the columns ride is queue-blind too, which is
                    // where a future column would inherit the leak from.
                    assert_eq!(s.context, base.context, "{} {}: the view context", decl.id, side.name());
                }
            }
        }
        // The EXECUTABLE half needs a real unresolved queue somewhere, or
        // "emptied" is the same view three times: 6(a)'s greater side is the
        // registration that carries one.
        assert_eq!(
            carried_a_queue,
            vec![
                "6(a) greater ([WarehouseTake])".to_string(),
                "6(a) greater ([WarehouseTake])".to_string(),
            ],
            "the package's unresolved queues are not where the registration says",
        );
    }

    /// One label's value for the subject seat, at one side of one fixture.
    fn subject_column(fixture: &PairFixture, side: Side, label: CandidateFeature, rules: &Rules)
        -> i64 {
        let view = fixture.view(side);
        let seat = view
            .players
            .iter()
            .find(|p| p.id == fixture.subject())
            .expect("the subject has a row in its own view");
        candidate_columns(view, seat, label, rules).expect("the adapter never declines")
    }

    /// The same, for a view built by hand off an edited state.
    fn subject_column_of(view: &PlayerView, seat: PlayerId, label: CandidateFeature, rules: &Rules)
        -> i64 {
        let row = view.players.iter().find(|p| p.id == seat).expect("the seat has a row");
        candidate_columns(view, row, label, rules).expect("the adapter never declines")
    }

    /// **CARRY 5 — 11(d)'s slot-5 inertness, at the computed X1 column.**
    ///
    /// 11(d) is family 11's X1 conservation gate, and the whole pair rests on
    /// one arithmetic claim its registration makes in prose: with money 0, no
    /// imports and realizable power 3 at the Laboratory, "the six deficits are
    /// 0 · 2 · 3 · 1 · 2 · 2, so slot 0 is the only zero and the other three
    /// cost-3 cards sit behind a surcharge they cannot pay for". Slot 5 is one
    /// of those three. Until this commit the claim was checked by the fixture's
    /// own slot arithmetic (`pair_fixtures`' `eleven_d_buys_at_zero_deficit…`
    /// re-derives the deficits); what it was NOT checked against is the X1
    /// FORMULA, which is the thing the solve will read.
    ///
    /// Inertness is proven by MUTATION rather than by re-deriving the table a
    /// third time. Slot 5 is emptied, and then filled with EVERY printed
    /// technology CARD in turn — all thirty of them, which is twenty titles
    /// with ten circulating as two physical copies, walked by
    /// `iter_sorted_by_id` so the order is the registry's rather than a
    /// `HashMap`'s — and X1 may not move for any of them. The cards rather than
    /// the titles because X1's ownership clause is EFFECT equality and the
    /// second copy of a title is the case that clause exists for.
    ///
    /// That is a stronger statement than the registration's: not
    /// merely that Patent System is unaffordable there, but that NO printed
    /// card is, because the cheapest printed industry is 3 and slot 5
    /// surcharges +2 against a power of 3 and an empty purse.
    ///
    /// Both stages, both sides. The greater side's lab is NOT the registered
    /// stock — its purchase ran the engine's own slide and refill — so running
    /// the sweep there asks the same question of a board the registration does
    /// not write out.
    #[test]
    fn eleven_d_slot_five_is_inert_at_the_computed_x1_column() {
        let rules = Rules::standard();
        let decl = registered("11(d)").expect("11(d) is registered");
        let titles = rules.techs.iter_sorted_by_id();
        assert_eq!(titles.len(), 30, "the printed technology registry: 20 titles, 30 cards");
        for stage in decl.stages {
            let fixture = build_registered_pair(decl, *stage, &rules)
                .unwrap_or_else(|e| panic!("11(d) at {}: {e:?}", stage.name()));
            let subject = fixture.subject();
            for side in [Side::Greater, Side::Lesser] {
                let base = subject_column(&fixture, side, CandidateFeature::LabTakeable, &rules);
                println!("11(d) {} {} X1 = {base}", stage.name(), side.name());
                let mut variants: Vec<(String, Option<TechCardId>)> =
                    vec![("empty".to_string(), None)];
                for card in &titles {
                    variants.push((format!("{:?}", card.effect), Some(card.id)));
                }
                for (name, slot) in variants {
                    let mut edited = fixture.state(side).clone();
                    edited.lab_state.lab_a.slots[5] = slot;
                    let view = observe(&edited, subject).expect("the subject observes its own seat");
                    assert_eq!(
                        subject_column_of(&view, subject, CandidateFeature::LabTakeable, &rules),
                        base,
                        "11(d) {} {}: X1 moved when lab slot 5 was set to {name} — slot 5 is \
                         registered inert at realizable power 3 against an empty purse",
                        stage.name(),
                        side.name(),
                    );
                }
            }
            // The registered direction, at the column rather than at the stock:
            // the purchase spends the one takeable slot.
            let takeable = (
                subject_column(&fixture, Side::Greater, CandidateFeature::LabTakeable, &rules),
                subject_column(&fixture, Side::Lesser, CandidateFeature::LabTakeable, &rules),
            );
            assert_eq!(
                takeable,
                (0, 1),
                "11(d) at {}: the registration declares X1 -1 takeable",
                stage.name(),
            );

            // ANTI-VACUITY, and it is the control the inertness sweep needs:
            // the SAME edit at SLOT 0 does move the column. Without it, "X1
            // never moved" would be satisfied just as well by an X1 that reads
            // the lab not at all.
            let mut without_slot_zero = fixture.state(Side::Lesser).clone();
            without_slot_zero.lab_state.lab_a.slots[0] = None;
            let view = observe(&without_slot_zero, subject).expect("the subject observes");
            assert_eq!(
                subject_column_of(&view, subject, CandidateFeature::LabTakeable, &rules),
                0,
                "11(d) at {}: emptying the ONE takeable slot must move X1, or the slot-5 sweep \
                 proves nothing",
                stage.name(),
            );
        }
    }

    /// **CARRY 8 — family 11's congruence, re-verified through the computed S
    /// columns.**
    ///
    /// The family's whole design rests on a geometric claim §5.2 registers as a
    /// requirement rather than a hope: the two acted areas' President
    /// neighbourhoods are congruent, "so S4 and S7 are EQUAL by construction —
    /// a registered requirement, not a disjunction". `pair_fixtures` proves the
    /// geometry (`the_family_eleven_reflection_is_a_board_automorphism`, and a
    /// per-pair ring/reach comparison) and says in as many words why it stops
    /// there: "Step 3 owns S4's and S7's implementations … and nothing here is a
    /// second derivation of them". This is the other end of that sentence — the
    /// automorphism argument read off the FORMULAS it was made about.
    ///
    /// `family_11_layout`'s doc states the realized values, not just the
    /// equality: "S4 = 1 (the acted area holds nothing after Recovery; its ring
    /// holds the one survivor) and S7 = 1 (that survivor is the whole reachable
    /// set) on both sides. S3 = 1 on both sides for the same reason." All three
    /// are pinned at their VALUES, because an equality at 0 would hold just as
    /// well for a fixture whose pawns had all vanished.
    ///
    /// The registered per-pair deltas ride the same sweep, in §5.2's own words:
    /// 11(a) "E2 +1, S1 −1", 11(b) "E1 +2, S5 −2", 11(c) "S6 −1", 11(d)
    /// "X1 −1 takeable; E1 unchanged".
    ///
    /// **OWN AND RELATIVE COINCIDE HERE, and that is asserted rather than
    /// assumed.** §5.2 writes family 11's deltas in the subject's own values
    /// while a solve row reads the relative form; the two agree only because
    /// the pair's two sides differ in the SUBJECT's activation alone and leave
    /// the opponent identical. A construction that disturbed the opponent would
    /// leave every prose delta above true and every solve row wrong, so the
    /// coincidence is checked for all eighteen labels before the deltas are.
    #[test]
    fn family_elevens_congruence_and_registered_deltas_hold_at_the_computed_columns() {
        use CandidateFeature::{
            AssistantAreasDistinct, BankAccess, EmploymentAgencyAccess, LabTakeable, MoneyStock,
            PresidentLocalAccess, PresidentReachableAreas, ProductionAccess, TradeGoodsStock,
        };
        // §5.2's registered candidate deltas, greater minus lesser.
        let deltas: &[(&str, CandidateFeature, i64)] = &[
            ("11(a)", ProductionAccess, -1),
            ("11(a)", TradeGoodsStock, 1),
            ("11(b)", BankAccess, -2),
            ("11(b)", MoneyStock, 2),
            ("11(c)", EmploymentAgencyAccess, -1),
            ("11(d)", LabTakeable, -1),
            ("11(d)", MoneyStock, 0),
        ];

        let rules = Rules::standard();
        let mut pairs = 0usize;
        let mut checked = 0usize;
        for (decl, fixture) in every_registered_fixture(&rules) {
            if decl.family != 11 {
                continue;
            }
            pairs += 1;
            let (g_own, g_theirs) = candidate_pair(&fixture, Side::Greater, &rules);
            let (l_own, l_theirs) = candidate_pair(&fixture, Side::Lesser, &rules);
            let at = format!("{} at {}", decl.id, fixture.stage().name());

            // The opponent is untouched by the activation choice, so the two
            // currencies are the same number.
            for label in CandidateFeature::ALL {
                let i = label.index();
                assert_eq!(
                    g_theirs[i], l_theirs[i],
                    "{at}: the opponent's {} differs across the sides, so §5.2's own-value \
                     deltas are not the relative rows a solve reads",
                    label.label(),
                );
            }

            // The congruence, at its registered VALUES.
            for label in [AssistantAreasDistinct, PresidentLocalAccess, PresidentReachableAreas] {
                let i = label.index();
                assert_eq!(
                    (g_own[i], l_own[i]),
                    (1, 1),
                    "{at}: `family_11_layout` registers {} = 1 on both sides — the automorphism \
                     carries one board position onto the other",
                    label.label(),
                );
            }

            for (id, label, declared) in deltas {
                if *id != decl.id {
                    continue;
                }
                let i = label.index();
                assert_eq!(
                    g_own[i] - l_own[i],
                    *declared,
                    "{at}: §5.2 registers {} = {declared} and the computed columns read {}. \
                     THIS IS A STOP.",
                    label.label(),
                    g_own[i] - l_own[i],
                );
                checked += 1;
            }
            println!(
                "{at:<20} S1 {:>3} S3 {:>3} S4 {:>3} S5 {:>3} S6 {:>3} S7 {:>3} X1 {:>3}",
                g_own[ProductionAccess.index()] - l_own[ProductionAccess.index()],
                g_own[AssistantAreasDistinct.index()] - l_own[AssistantAreasDistinct.index()],
                g_own[PresidentLocalAccess.index()] - l_own[PresidentLocalAccess.index()],
                g_own[BankAccess.index()] - l_own[BankAccess.index()],
                g_own[EmploymentAgencyAccess.index()] - l_own[EmploymentAgencyAccess.index()],
                g_own[PresidentReachableAreas.index()] - l_own[PresidentReachableAreas.index()],
                g_own[LabTakeable.index()] - l_own[LabTakeable.index()],
            );
        }
        // Nine (pair, stage) fixtures: four both-endpoint pairs and 11(e) late.
        assert_eq!(pairs, 9, "family 11 realizes 9 (pair, stage) fixtures");
        // Fourteen rather than seven: every declared delta is checked at every
        // endpoint its pair registers, and four of the five pairs register two.
        assert_eq!(checked, 14, "the registered family-11 candidate deltas, per endpoint");
    }

    /// The seat that is not `subject`, in a 2P fixture.
    fn other_seat(fixture: &PairFixture, side: Side) -> PlayerId {
        fixture
            .view(side)
            .players
            .iter()
            .map(|p| p.id)
            .find(|id| *id != fixture.subject() && !id.is_dummy())
            .expect("a 2P fixture has a rival seat")
    }

    /// Whether any seat's hand in `view` carries a masked starting order —
    /// §5.2(b)'s class test, read off the view rather than off a label.
    fn carries_a_masked_slot(view: &PlayerView) -> bool {
        view.players
            .iter()
            .any(|p| p.hand.iter().any(|c| matches!(c, HandCard::HiddenStartingOrder)))
    }

    /// One seat's relative candidate vector as read from ITS OWN view.
    fn relative_from(view: &PlayerView, me: PlayerId, rules: &Rules)
        -> [i64; CandidateFeature::COUNT] {
        let mut out = [0i64; CandidateFeature::COUNT];
        for label in CandidateFeature::ALL {
            let read = |seat: Option<&VisiblePlayer>| {
                seat.map_or(0, |p| {
                    candidate_columns(view, p, label, rules).expect("the adapter never declines")
                })
            };
            let mine = read(view.players.iter().find(|p| p.id == me));
            let theirs = read(view.players.iter().find(|p| p.id != me && !p.id.is_dummy()));
            out[label.index()] = mine - theirs;
        }
        out
    }

    /// **CARRY 7(b) — §5.2 family 5's CROSS-VIEW NEGATION, over the registry.**
    ///
    /// The property §5.2(b) registers is: at 2P, on a fixture where NEITHER
    /// view carries a masked hand slot, the relative vector read from one
    /// seat's view is the exact negation of the one read from the other's.
    ///
    /// **THE FROZEN PACKAGE REALIZES NO SUCH FIXTURE, and this cell records
    /// that as a measured fact rather than leaving it implicit.** All 106
    /// (fixture, side) pairs carry a masked slot in at least one of the two
    /// views, which is structural rather than accidental: a 2P state with no
    /// `HiddenStartingOrder` anywhere needs BOTH seats' starting orders
    /// fulfilled, and no registration asks for that. 4(g) comes closest and its
    /// note says exactly how far — "the subject's view carries no
    /// `HiddenStartingOrder` slot at all" — but the OPPONENT still sees the
    /// subject's own dealt card masked, so 4(g) is one view short of the class.
    /// §5.2(b)'s identity is therefore pinned where it can be: on states
    /// `tests/eval_core.rs` builds for it (`group_s_/e_/x_differences_negate_
    /// across_the_two_2p_views`), and NOT duplicated here.
    ///
    /// **WHAT THE REGISTRY CAN SAY, and it is not nothing:** twelve of the
    /// eighteen labels negate across the two views at every one of the 106
    /// sides, masked slot or not, because they read nothing masking can touch.
    /// The six that do not are asserted AS A SET, so a thirteenth label
    /// becoming view-sensitive fails here rather than passing quietly into the
    /// solve.
    ///
    /// **THE SET IS `{O1, O2, O3, O5, O6, X3}` — ONE MORE THAN §5.2(c)'s
    /// DECLARED-ASYMMETRY LIST**, and the extra member is O5. That is not a
    /// contradiction and it is worth stating plainly. §5.2(c) lists the
    /// features whose cross-view gap EQUALS the masked slot's own additive
    /// term, and `eval_core`'s pin for it records why O5 is left out: "O5 is a
    /// MAX over a set, not a sum over a hand, so a masked card has no additive
    /// term to be the gap." Being undecomposable is not the same as being
    /// symmetric — a seat's own Known card puts a country in its attainable set
    /// that the opponent cannot see — so O5 IS view-asymmetric, in a form no
    /// (c)-style identity captures. Step 4's collision instrument and step 5's
    /// solver read relative vectors from ONE seat's view; this is the row where
    /// "which seat's view" is a question with an answer and no closed form.
    #[test]
    fn twelve_candidate_columns_negate_across_the_two_2p_views_at_every_registered_side() {
        let rules = Rules::standard();
        let mut sides = 0usize;
        let mut in_class: Vec<String> = Vec::new();
        let mut subject_side_unmasked: Vec<String> = Vec::new();
        let mut unobservable: Vec<String> = Vec::new();
        let mut breaks: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for (decl, fixture) in every_registered_fixture(&rules) {
            let subject = fixture.subject();
            for side in [Side::Greater, Side::Lesser] {
                let at = format!("{} {} {}", decl.id, fixture.stage().name(), side.name());
                let opponent = other_seat(&fixture, side);
                let mine = fixture.view(side);
                let theirs = match observe(fixture.state(side), opponent) {
                    Ok(v) => v,
                    Err(e) => {
                        unobservable.push(format!("{at}: {e:?}"));
                        continue;
                    }
                };
                sides += 1;
                if !carries_a_masked_slot(mine) {
                    subject_side_unmasked.push(at.clone());
                    if !carries_a_masked_slot(&theirs) {
                        in_class.push(at.clone());
                    }
                }
                let a = relative_from(mine, subject, &rules);
                let b = relative_from(&theirs, opponent, &rules);
                for label in CandidateFeature::ALL {
                    let i = label.index();
                    if a[i] != -b[i] {
                        breaks.insert(label.label());
                    }
                }
            }
        }
        assert!(
            unobservable.is_empty(),
            "the package carries sides whose rival seat `observe` refuses: {unobservable:#?}",
        );
        assert_eq!(sides, 106, "53 (pair, stage) fixtures, two sides each");

        // The class is empty, and 4(g) is exactly how close the package gets.
        assert!(
            in_class.is_empty(),
            "a registered fixture now falls in §5.2(b)'s no-hidden-slot class ({in_class:?}) — \
             the identity is newly exercisable here and this cell should assert it rather than \
             record its absence",
        );
        assert_eq!(
            subject_side_unmasked,
            vec![
                "4(g) early greater".to_string(),
                "4(g) early lesser".to_string(),
                "4(g) late greater".to_string(),
                "4(g) late lesser".to_string(),
            ],
            "4(g)'s note — \"the subject's view carries no `HiddenStartingOrder` slot at all\" — \
             names the package's only masked-free subject views",
        );

        // The twelve that negate everywhere, stated as the complement of the six
        // that do not.
        let expected: std::collections::BTreeSet<&str> =
            ["O1", "O2", "O3", "O5", "O6", "X3"].into_iter().collect();
        assert_eq!(
            breaks, expected,
            "the view-asymmetric labels are not the registered set; the other {} negate at every \
             one of the {sides} sides",
            CandidateFeature::COUNT - expected.len(),
        );
        println!(
            "cross-view negation: {} of {} labels negate at all {sides} registered sides; \
             asymmetric = {breaks:?}",
            CandidateFeature::COUNT - breaks.len(),
            CandidateFeature::COUNT,
        );
    }

    /// Swap the LABELS of seats 0 and 1 — the rows trade places and EVERY
    /// `PlayerId` the rest of the state names travels with them.
    ///
    /// `tests/eval_core.rs` has a relabel of the same name for its hand-built
    /// 5(a) fixtures, and this is deliberately not that one: it is a lib-facing
    /// test helper an example cannot reach, and the frozen package needs two
    /// fields that one does not touch. `BuildingSite::shophouses_built` and
    /// `trading_house_owner` name their owners inside `board_layout`, so a
    /// relabel that skipped them would leave a built shophouse credited to a
    /// seat whose own `shophouses.built` map had moved — an INCOHERENT state,
    /// which is why the caller re-checks invariants rather than trusting this
    /// list to be complete.
    fn relabel_seats_0_and_1(state: &GameState) -> GameState {
        let swap = |p: PlayerId| match p.0 {
            0 => PlayerId(1),
            1 => PlayerId(0),
            _ => p,
        };
        let mut out = state.clone();
        out.players.swap(0, 1);
        out.players[0].id = PlayerId(0);
        out.players[1].id = PlayerId(1);
        for slot in out.church_track.slots.iter_mut().flatten() {
            slot.player = swap(slot.player);
        }
        for slot in out.customs_track.slots.iter_mut().flatten() {
            slot.player = swap(slot.player);
        }
        for card in out.achievements.iter_mut() {
            for claimant in card.claimed_by.iter_mut() {
                *claimant = swap(*claimant);
            }
        }
        for site in out.board_layout.building_sites.values_mut() {
            for owner in site.shophouses_built.iter_mut().flatten() {
                *owner = swap(*owner);
            }
            site.trading_house_owner = site.trading_house_owner.map(swap);
        }
        out.current_player = swap(out.current_player);
        out.config.start_player = swap(out.config.start_player);
        out.trigger_player = out.trigger_player.map(swap);
        out
    }

    /// Each seat's OWN eighteen values, read from that seat's OWN view.
    fn own_rows(state: &GameState, rules: &Rules) -> [[i64; CandidateFeature::COUNT]; 2] {
        let mut out = [[0i64; CandidateFeature::COUNT]; 2];
        for (seat, row) in out.iter_mut().enumerate() {
            let id = PlayerId(seat as u8);
            let view = observe(state, id).expect("a 2P fixture seat observes");
            for label in CandidateFeature::ALL {
                row[label.index()] = subject_column_of(&view, id, label, rules);
            }
        }
        out
    }

    /// **CARRY 7(a) — §5.2 family 5's INDEX-PERMUTATION INVARIANCE, over the
    /// registry.**
    ///
    /// Relabeling the seats permutes every candidate feature's per-seat values
    /// identically. §5.2 names the registered cross-seat reads this is really
    /// about — S4's in-hand branch and S7 read ALL seats' President locations,
    /// since an opponent's President bars a re-entry target and un-counts a
    /// reachable area — so the claim is not the trivial one that each feature
    /// reads one row; it is that the cross-seat reads follow the relabeling too.
    ///
    /// `tests/eval_core.rs` pins this per group on ONE state built to make the
    /// two seats differ. Running it over the frozen package asks the harder
    /// version: whether it survives every position the solve will actually read
    /// — hand-built layouts, mid-turn states, a purchased laboratory, a drained
    /// port.
    ///
    /// **THE RELABEL IS RE-CHECKED, NOT TRUSTED.** A relabel that missed a
    /// `PlayerId` would produce a state whose seats no longer own what the board
    /// says they own, and the columns would differ for that reason rather than
    /// for a feature-locality one — a FALSE failure on a frozen package. So
    /// every relabeled state goes through `check_invariants` before anything is
    /// read off it, and a refusal is reported as a relabel defect in as many
    /// words rather than as a permutation failure.
    #[test]
    fn every_candidate_column_permutes_with_a_full_seat_relabel_at_every_registered_side() {
        let rules = Rules::standard();
        let mut sides = 0usize;
        let mut discriminating = 0usize;
        for (decl, fixture) in every_registered_fixture(&rules) {
            for side in [Side::Greater, Side::Lesser] {
                let at = format!("{} {} {}", decl.id, fixture.stage().name(), side.name());
                let state = fixture.state(side);
                let relabelled = relabel_seats_0_and_1(state);
                check_invariants(&relabelled, &rules).unwrap_or_else(|e| {
                    panic!(
                        "{at}: the relabeled state is incoherent ({e}) — this is a DEFECT IN THE \
                         RELABEL HELPER, not a permutation failure: some `PlayerId` the state \
                         names did not travel with the rows"
                    )
                });
                let before = own_rows(state, &rules);
                let after = own_rows(&relabelled, &rules);
                sides += 1;
                if before[0] != before[1] {
                    discriminating += 1;
                }
                for label in CandidateFeature::ALL {
                    let i = label.index();
                    assert_eq!(
                        (after[0][i], after[1][i]),
                        (before[1][i], before[0][i]),
                        "{at}: {} does not permute with the seat labels",
                        label.label(),
                    );
                }
            }
        }
        assert_eq!(sides, 106, "53 (pair, stage) fixtures, two sides each");
        // ANTI-VACUITY: on a side whose two seats read identically the
        // permutation is invisible, so the claim rests on the sides where they
        // do not. Measured before it was set.
        assert_eq!(
            discriminating, 106,
            "only {discriminating} of {sides} sides have seats that read differently, so the \
             permutation is invisible on the rest",
        );
        println!("seat relabel: {discriminating} of {sides} sides discriminate, all permute");
    }

    /// **CARRY 1's OTHER HALF — the print seam itself.** With the adapter
    /// supplied every one of the thirty-four rows carries a number; WITHOUT it
    /// the eighteen candidate rows still render `CANDIDATE_PENDING`, in the
    /// same columns.
    ///
    /// The pending path is not dead code now that step 3 has built the columns,
    /// and its width behaviour is what this pins. `push_row`'s doc gives the
    /// reason it exists: "the candidate cell is `CANDIDATE_PENDING` rather than
    /// a number and a per-arm `format!` would silently ragged-edge the table
    /// exactly where a reader is scanning for what is not built yet". So the
    /// marker is asserted to FIT the delta column exactly, and both renderings
    /// are asserted to put their delta field at the same character offset —
    /// which is the property a reader relies on and the one a future edit to
    /// either arm could break without any test noticing.
    ///
    /// `CANDIDATE_PENDING`'s own doc explains why the value columns print `-`
    /// rather than 0: "A zero in a delta table is a MEASUREMENT". That is
    /// checked too — a pending row must carry no digit anywhere.
    #[test]
    fn the_print_seam_renders_thirty_four_real_rows_and_still_aligns_its_pending_path() {
        // The delta field is `{index:<4} {feature:<28} {greater:>16} {lesser:>16}
        // {delta:>16}` — 84 characters up to and including the delta column.
        const DELTA_FIELD: std::ops::Range<usize> = 68..84;
        assert_eq!(
            CANDIDATE_PENDING.len(),
            DELTA_FIELD.len(),
            "the pending marker no longer fits the delta column, so the table ragged-edges",
        );

        let rules = Rules::standard();
        let decl = registered("4(c)").expect("4(c) is registered");
        let fixture = build_registered_pair(decl, StageEndpoint::Late, &rules).expect("4(c) builds");
        let real = render_pair_report_with(&fixture, &rules, Some(candidate_columns));
        let pending = render_pair_report_with(&fixture, &rules, None);

        // Both reports print the digest §5.1 requires of every artifact.
        for report in [&real, &pending] {
            assert!(report.contains(&package_digest()), "every report prints the digest");
        }

        let rows = |report: &str| -> Vec<String> {
            report
                .lines()
                .filter(|l| l.len() >= DELTA_FIELD.end)
                .filter(|l| l.split_whitespace().next().is_some_and(|w| w.parse::<usize>().is_ok()))
                .map(str::to_string)
                .collect()
        };
        let real_rows = rows(&real);
        let pending_rows = rows(&pending);
        assert_eq!(real_rows.len(), 34, "the whole §3.2 axis, 16 incumbents + 18 candidates");
        assert_eq!(pending_rows.len(), 34, "the pending rendering carries the same axis");

        assert!(
            !real.contains(CANDIDATE_PENDING),
            "a candidate row still renders as pending with the adapter supplied",
        );
        for (i, (r, p)) in real_rows.iter().zip(&pending_rows).enumerate() {
            // The incumbent half is identical between the two renderings; the
            // candidate half differs only in the three value columns.
            let candidate = i >= 16;
            assert_eq!(
                r[..34],
                p[..34],
                "row {i}: the index and feature columns differ between the two renderings",
            );
            if candidate {
                assert_eq!(&p[DELTA_FIELD], CANDIDATE_PENDING, "row {i}: the pending marker");
                assert!(
                    !p[34..DELTA_FIELD.start].contains(|c: char| c.is_ascii_digit()),
                    "row {i}: a pending row printed a digit in a VALUE column — a zero in a \
                     delta table is a MEASUREMENT, which is the claim `CANDIDATE_PENDING` exists \
                     to avoid making (the marker's own `3` lives in the delta column)",
                );
                assert!(
                    r[DELTA_FIELD].trim().parse::<i64>().is_ok(),
                    "row {i}: the real rendering must carry a number where the marker was",
                );
            } else {
                assert_eq!(r, p, "row {i}: an incumbent row must render identically either way");
            }
        }
    }

    /// The `pairs` command's argv discipline: an unknown narrowing is an ERROR,
    /// never an empty dump.
    ///
    /// A mistyped `--pair 4c` that printed a header and no reports reads exactly
    /// like a package that lost a registration — the same reasoning
    /// [`split_args`] applies to unknown flags one level up.
    #[test]
    fn the_pairs_command_refuses_an_unknown_narrowing_rather_than_dumping_nothing() {
        let arg = |s: &str| s.to_string();
        assert!(cmd_pairs(&[arg("--pair"), arg("4c")])
            .is_err_and(|e| e.contains("`4c` is not a registered pair")));
        assert!(cmd_pairs(&[arg("--stage"), arg("mid")])
            .is_err_and(|e| e.contains("unknown stage `mid`")));
        assert!(cmd_pairs(&[arg("4(a)")]).is_err_and(|e| e.contains("no positional arguments")));
        assert!(cmd_pairs(&[arg("--os-entropi")]).is_err(), "an unknown flag is never ignored");
        // The narrowings that ARE spelled correctly resolve.
        assert!(cmd_pairs(&[arg("--pair"), arg("4(c)"), arg("--manifest")]).is_ok());
    }

    // -----------------------------------------------------------------------
    // §5.3 Part A — the collision instrument
    // -----------------------------------------------------------------------

    /// **THE SIGNATURE'S CURRENCY** — §5.3 signs a view with
    /// `(exact progress_ppm, Δv over the 34 features)`, and this cell says which
    /// Δv that is by deriving the pair's whole delta table from the two SIDE
    /// signatures alone.
    ///
    /// The reading the instrument implements: `Δv` is the MOVER-RELATIVE vector
    /// `mine − theirs` of ONE view, so a pair's registered delta row is the
    /// DIFFERENCE of its two sides' signatures. If the currency were the
    /// subject's own levels instead, this subtraction would reproduce the delta
    /// table only where the opponent happens to read equal on both sides — which
    /// is most rows, and exactly why the check runs over the whole registry
    /// rather than a sample.
    ///
    /// Both halves are checked against surfaces that do not go through
    /// [`signature`]: the sixteen incumbents against `PairFixture::incumbent_delta`
    /// (core's own public arithmetic) and the eighteen candidates against
    /// [`candidate_pair`], the two-currency helper carry 3 and carry 6 already
    /// read the registry with.
    #[test]
    fn the_signature_difference_across_a_pair_is_the_registered_delta_table() {
        let rules = Rules::standard();
        let fixtures = every_registered_fixture(&rules);
        assert_eq!(fixtures.len(), 53, "the frozen package realizes 53 (pair, stage) fixtures");

        let mut rows = 0usize;
        for (decl, fixture) in &fixtures {
            let subject = fixture.subject();
            let g = signature(fixture.view(Side::Greater), subject, &rules)
                .unwrap_or_else(|e| panic!("{} greater: {e}", decl.id));
            let l = signature(fixture.view(Side::Lesser), subject, &rules)
                .unwrap_or_else(|e| panic!("{} lesser: {e}", decl.id));

            let incumbent = fixture.incumbent_delta(&rules);
            for f in Feature::ALL {
                let slot = FeatureId::Incumbent(f).index();
                assert_eq!(
                    g.dv[slot] - l.dv[slot],
                    incumbent[f.index()],
                    "{} {}: incumbent {} — the signature difference is not the delta row",
                    decl.id,
                    fixture.stage().name(),
                    f.name(),
                );
                rows += 1;
            }

            let (g_own, g_theirs) = candidate_pair(fixture, Side::Greater, &rules);
            let (l_own, l_theirs) = candidate_pair(fixture, Side::Lesser, &rules);
            for c in CandidateFeature::ALL {
                let slot = FeatureId::Candidate(c).index();
                let expected = (g_own[c.index()] - g_theirs[c.index()])
                    - (l_own[c.index()] - l_theirs[c.index()]);
                assert_eq!(
                    g.dv[slot] - l.dv[slot],
                    expected,
                    "{} {}: candidate {} — the signature difference is not the delta row",
                    decl.id,
                    fixture.stage().name(),
                    c.label(),
                );
                rows += 1;
            }

            // The other half of the signature: the endpoint's exact ppm, taken
            // from the view rather than from the declared endpoint label.
            assert_eq!(g.ppm, fixture.progress_ppm(&rules), "{}: greater ppm", decl.id);
            assert_eq!(
                l.ppm,
                progress_ppm(fixture.view(Side::Lesser), &rules),
                "{}: lesser ppm",
                decl.id,
            );
        }
        assert_eq!(rows, 53 * 34, "the sweep must cover every row of every fixture");
    }

    /// **THE SEAT CONVENTION IS LOAD-BEARING** — the other seat's signature of
    /// the same position is NOT the mover's with the signs flipped.
    ///
    /// §5.3 signs "from the MOVER's view", and this cell is why that phrase
    /// cannot be dropped as a formality. Re-signing all 106 registered states
    /// from the OPPONENT's seat (its own masked view, its own differencing)
    /// leaves the exact-`ppm` half untouched — progress is public-board-derived
    /// — and moves a NAMED set of axis rows off the negation a reader would
    /// assume. The set is MEASURED here and asserted as a set: a column joining
    /// or leaving it is a finding about the columns, not about this test.
    #[test]
    fn the_other_seats_signature_is_not_the_movers_negated() {
        let rules = Rules::standard();
        let fixtures = every_registered_fixture(&rules);

        let mut asymmetric: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut sides = 0usize;
        for (decl, fixture) in &fixtures {
            for side in [Side::Greater, Side::Lesser] {
                let state = fixture.state(side);
                let mover = fixture.subject();
                let opponent = fixture
                    .view(side)
                    .players
                    .iter()
                    .find(|p| p.id != mover && !p.id.is_dummy())
                    .map(|p| p.id)
                    .expect("a 2P fixture has one non-dummy opponent");

                let mine = signature(fixture.view(side), mover, &rules)
                    .unwrap_or_else(|e| panic!("{} {}: {e}", decl.id, side.name()));
                let theirs = signature(
                    &observe(state, opponent).expect("the opponent seat observes"),
                    opponent,
                    &rules,
                )
                .unwrap_or_else(|e| panic!("{} {} opponent: {e}", decl.id, side.name()));

                assert_eq!(
                    mine.ppm,
                    theirs.ppm,
                    "{} {}: progress is public and must not depend on the seat asking",
                    decl.id,
                    side.name(),
                );
                for slot in FeatureId::all() {
                    if theirs.dv[slot.index()] != -mine.dv[slot.index()] {
                        *asymmetric.entry(slot.label()).or_default() += 1;
                    }
                }
                sides += 1;
            }
        }
        assert_eq!(sides, 106, "the frozen package realizes 106 sides");

        let labels: Vec<&str> = asymmetric.keys().copied().collect();
        assert_eq!(
            labels,
            vec!["O1", "O2", "O3", "O5", "O6", "X3"],
            "the view-asymmetric rows have changed (counts: {asymmetric:?})",
        );
    }

    /// A signature with a hand-chosen `ppm` and a hand-chosen 34-vector, for the
    /// classifier cells: the rule under test is arithmetic over signatures and
    /// provenances, and building it out of real positions would test the
    /// positions instead.
    fn test_sig(ppm: u32, fill: i64) -> Signature {
        let mut dv = [0i64; AXIS_WIDTH];
        for (i, slot) in dv.iter_mut().enumerate() {
            *slot = fill + i as i64;
        }
        Signature { ppm, dv }
    }

    fn side_of(pair: &str, stage: &'static str, side: &'static str, sig: Signature) -> SignedView {
        SignedView {
            sig,
            from: Provenance::PairSide { pair: pair.to_string(), stage, side, gate: true },
        }
    }

    /// The same, for a registration whose role prices nothing.
    fn diag_of(pair: &str, stage: &'static str, side: &'static str, sig: Signature) -> SignedView {
        SignedView {
            sig,
            from: Provenance::PairSide { pair: pair.to_string(), stage, side, gate: false },
        }
    }

    fn succ_of(root: &str, ordinal: usize, role: RecipeRole, sig: Signature) -> SignedView {
        SignedView {
            sig,
            from: Provenance::Successor {
                root: root.to_string(),
                set: "fork",
                ordinal,
                mv: format!("move-{ordinal}"),
                role,
            },
        }
    }

    /// **THE BLOCKING RULE, arm by arm** (§5.3): a class blocks iff it contains
    /// two sides of ONE registered gate pair or the two sides of ONE root's
    /// registered development successor pair. Everything else that shares a
    /// signature is ORGANIC — a diagnostic with a class size, never a stop.
    #[test]
    fn a_class_blocks_only_on_a_registered_pair() {
        let shared = test_sig(0, 7);

        // (a) two sides of ONE gate pair.
        let one_pair = classify(vec![
            side_of("4(c)", "late", "greater", shared.clone()),
            side_of("4(c)", "late", "lesser", shared.clone()),
        ]);
        assert_eq!(one_pair.blocking().len(), 1, "two sides of one pair must block");
        assert!(matches!(
            &one_pair.blocking()[0],
            Blocking::GatePair { pair, stage } if pair == "4(c)" && *stage == "late"
        ));

        // (b) two sides of DIFFERENT pairs: organic, and the class is reported
        // with its size rather than swallowed.
        let two_pairs = classify(vec![
            side_of("4(c)", "late", "greater", shared.clone()),
            side_of("1(a)", "early", "lesser", shared.clone()),
        ]);
        assert!(two_pairs.blocking().is_empty(), "different pairs never block");
        assert_eq!(two_pairs.classes.len(), 1);
        assert_eq!(two_pairs.classes[0].members.len(), 2);

        // (b2) two sides of one DIAGNOSTIC registration: signed, listed, and
        // never blocking — it yields no solve row for a collision to sink.
        let diagnostic = classify(vec![
            diag_of("10(c)", "early", "greater", shared.clone()),
            diag_of("10(c)", "early", "lesser", shared.clone()),
        ]);
        assert!(diagnostic.blocking().is_empty(), "a diagnostic pair is never a gate pair");
        assert_eq!(diagnostic.classes[0].members.len(), 2, "and it is still reported");

        // (c) the same pair at DIFFERENT endpoints is two gate pairs, not one.
        let two_stages = classify(vec![
            side_of("4(c)", "late", "greater", shared.clone()),
            side_of("4(c)", "early", "greater", shared.clone()),
        ]);
        assert!(two_stages.blocking().is_empty(), "one side at each endpoint is not a gate pair");

        // (d) a root's REGISTERED pair blocks; two unregistered successors of
        // the same root do not — §5.3's own "organic collisions are
        // diagnostics" is what separates them.
        let registered_pair = classify(vec![
            succ_of("g1-e113-tiemass", 4, RecipeRole::Productive, shared.clone()),
            succ_of("g1-e113-tiemass", 9, RecipeRole::Dead, shared.clone()),
        ]);
        assert_eq!(registered_pair.blocking().len(), 1);
        assert!(matches!(
            &registered_pair.blocking()[0],
            Blocking::RootPair { root } if root == "g1-e113-tiemass"
        ));

        let organic_successors = classify(vec![
            succ_of("g1-e113-tiemass", 4, RecipeRole::Unregistered, shared.clone()),
            succ_of("g1-e113-tiemass", 9, RecipeRole::Unregistered, shared.clone()),
        ]);
        assert!(
            organic_successors.blocking().is_empty(),
            "two same-root successors that are not the registered pair are a diagnostic",
        );
        assert_eq!(organic_successors.classes[0].members.len(), 2);

        // (e) the registered pair of DIFFERENT roots is not one root's pair.
        let across_roots = classify(vec![
            succ_of("g1-e113-tiemass", 4, RecipeRole::Productive, shared.clone()),
            succ_of("g3-e93-separated", 9, RecipeRole::Dead, shared.clone()),
        ]);
        assert!(across_roots.blocking().is_empty(), "the pair must be ONE root's");

        // A singleton is not a collision at all.
        let alone = classify(vec![side_of("4(c)", "late", "greater", shared)]);
        assert!(alone.classes.is_empty() && alone.blocking().is_empty());
    }

    /// **NEAR-COLLISIONS** (§5.3): equal Δv at different `progress_ppm` is
    /// REPORTED and never blocking — the two views interpolate to different
    /// weight vectors, so a weight that separates them exists even though their
    /// vectors are identical.
    #[test]
    fn equal_deltas_at_different_progress_are_near_collisions_and_never_block() {
        let a = test_sig(0, 7);
        let b = test_sig(1_000_000, 7);
        assert_eq!(a.dv, b.dv, "the two differ ONLY in progress");

        let out = classify(vec![
            side_of("4(c)", "early", "greater", a),
            side_of("4(c)", "late", "lesser", b),
        ]);
        assert!(out.classes.is_empty(), "different ppm is not the same signature");
        assert!(out.blocking().is_empty(), "a near-collision never blocks");
        assert_eq!(out.near.len(), 1);
        assert_eq!(out.near[0].ppms, vec![0, 1_000_000]);
        assert_eq!(out.near[0].members.len(), 2);
    }

    /// The artifact is a record, so its ordering may not depend on the order the
    /// inputs happened to be built in.
    #[test]
    fn the_classification_is_ordering_independent() {
        let mut input = vec![
            side_of("1(a)", "early", "greater", test_sig(0, 1)),
            side_of("1(a)", "early", "lesser", test_sig(0, 1)),
            succ_of("r", 2, RecipeRole::Dead, test_sig(0, 1)),
            succ_of("r", 1, RecipeRole::Productive, test_sig(500_000, 5)),
            side_of("2(b)", "early", "greater", test_sig(500_000, 5)),
        ];
        let forward = classify(input.clone());
        input.reverse();
        let backward = classify(input);
        assert_eq!(forward.render(), backward.render(), "the render must not depend on argv order");
        assert_eq!(forward.blocking().len(), 1, "1(a)'s two sides collide");
    }

    /// **A DIAGNOSTIC PAIR IS NOT A GATE PAIR** — and the frozen package makes
    /// the distinction load-bearing at the very first run.
    ///
    /// §5.3's inputs are "every GATE pair's sides (families 1–4, 6–8,
    /// 10(a)/(b)/(d), 11)", and the parenthetical breaks family 10 open for one
    /// reason: 10(c) is the family's DIAGNOSTIC. The frame says the same thing
    /// in its own type — `PairRole::Constraint` is documented as "a solve row
    /// and a blocking side", `PairRole::Diagnostic` as "2(c) and 10(c):
    /// RECORDED, never a gate" — and §5.4's row census confirms it by
    /// arithmetic: the 34 row-yielding registrations it enumerates are the
    /// registry's 36 minus exactly those two.
    ///
    /// This matters because 10(c)'s two sides ARE signature-identical, by
    /// registered construction: step 2 proved its "+3 maximum" from the fact
    /// that an ALL-ROWS-EQUAL pair can carry no more than 3 residual faith. A
    /// blocking rule that keyed on "two sides of one registration" would stop
    /// the milestone on a pair that prices nothing and constrains nothing.
    #[test]
    fn the_diagnostic_pairs_sides_collide_by_construction_and_do_not_block() {
        let rules = Rules::standard();
        let (views, census) = sign_fixture_package(&rules).expect("the frozen package signs");
        assert_eq!(views.len(), 106, "every registered side is signed, diagnostics included");
        assert_eq!(census.gate_rows_early, 24, "§5.4's EARLY census under the 4(c) ruling");
        assert_eq!(census.gate_rows_late, 27, "§5.4's LATE census");
        assert_eq!(census.diagnostic_fixtures, 2, "2(c) and 10(c)");

        let out = classify(views);
        let tenc: Vec<&CollisionClass> = out
            .classes
            .iter()
            .filter(|c| {
                c.members.iter().any(|m| matches!(m, Provenance::PairSide { pair, .. } if pair == "10(c)"))
            })
            .collect();
        assert_eq!(tenc.len(), 1, "10(c)'s two sides share a signature by construction");
        assert_eq!(tenc[0].members.len(), 2);
        assert!(tenc[0].blocking.is_empty(), "a diagnostic pair is not a gate pair");
        assert!(
            out.blocking().is_empty(),
            "the frozen package carries no blocking collision: {:?}",
            out.blocking(),
        );
    }

    // -----------------------------------------------------------------------
    // §5.5 — the collinearity / VIF instrument
    // -----------------------------------------------------------------------

    /// A row of the §5.5 matrix with the named slots filled and the rest left
    /// at zero — which also exercises the constant-column path, since 31 of the
    /// 34 never move in these cells.
    fn stats_row(values: &[(usize, i64)]) -> [i64; AXIS_WIDTH] {
        let mut row = [0i64; AXIS_WIDTH];
        for (slot, value) in values {
            row[*slot] = *value;
        }
        row
    }

    fn stats_of(rows: &[[i64; AXIS_WIDTH]]) -> ColumnStats {
        let mut stats = ColumnStats::new();
        for row in rows {
            stats.observe(row, row);
        }
        stats
    }

    /// **THE CORRELATION AND VIF ARITHMETIC, against hand-computable truth.**
    ///
    /// Four columns of four rows, chosen so every number below is derivable
    /// with a pencil: x = 1,2,3,4 and y = 1,3,2,4 have equal variance 1.25 and
    /// covariance 1.0, so `corr = 0.8` exactly and each one's VIF against the
    /// other is `1/(1 − 0.64) = 2.7`.
    #[test]
    fn the_collinearity_arithmetic_is_what_it_claims() {
        let x = [1i64, 2, 3, 4];
        let y = [1i64, 3, 2, 4];
        let rows: Vec<[i64; AXIS_WIDTH]> =
            (0..4).map(|i| stats_row(&[(0, x[i]), (1, y[i])])).collect();
        let stats = stats_of(&rows);

        assert_eq!(stats.n, 4);
        assert!((stats.mean(0) - 2.5).abs() < 1e-12);
        assert!((stats.sd(0) - 1.25f64.sqrt()).abs() < 1e-12, "population sd, not the sample one");
        let r = stats.corr(0, 1).expect("both columns move");
        assert!((r - 0.8).abs() < 1e-12, "corr was {r}");
        assert_eq!(stats.corr(0, 2), None, "a constant column has no correlation");

        let table = vif_table(&stats);
        assert_eq!(table.constant.len(), AXIS_WIDTH - 2, "the 32 unfilled slots are constant");
        assert!(table.dependent.is_empty(), "0.8 is not a dependency");
        let expected = 1.0 / (1.0 - 0.8 * 0.8);
        for slot in [0, 1] {
            let vif = table.vif.get(&slot).copied().expect("both active columns get a VIF");
            assert!((vif - expected).abs() < 1e-9, "VIF at {slot} was {vif}, want {expected}");
        }
    }

    /// An EXACT dependency is reported as one, by name, and never as a large
    /// finite number: `1/(1 − R²)` at `R² = 1` is not a measurement, and a
    /// silently ridged inverse would print a plausible VIF for a column that
    /// carries no independent information at all.
    #[test]
    fn an_exact_dependency_is_named_rather_than_ridged() {
        // Perfect pairwise collinearity: y = 2x.
        let doubled = stats_of(&[
            stats_row(&[(0, 1), (1, 2)]),
            stats_row(&[(0, 2), (1, 4)]),
            stats_row(&[(0, 3), (1, 6)]),
        ]);
        assert!((doubled.corr(0, 1).expect("both move") - 1.0).abs() < 1e-12);
        let table = vif_table(&doubled);
        assert_eq!(table.dependent, vec![1], "the later column is the dependent one");
        assert!(table.vif.contains_key(&0), "the independent column still gets a VIF");
        assert!(!table.vif.contains_key(&1), "a dependent column gets no finite VIF");

        // A three-column dependency no PAIRWISE correlation reveals: z = x + y
        // with x and y themselves uncorrelated.
        let summed = stats_of(&[
            stats_row(&[(0, 1), (1, 1), (2, 2)]),
            stats_row(&[(0, 1), (1, -1), (2, 0)]),
            stats_row(&[(0, -1), (1, 1), (2, 0)]),
            stats_row(&[(0, -1), (1, -1), (2, -2)]),
        ]);
        assert!(summed.corr(0, 1).expect("both move").abs() < 1e-12, "x and y are uncorrelated");
        let table = vif_table(&summed);
        assert_eq!(table.dependent, vec![2], "z is the dependency, and no pair shows it");
    }

    /// Every label §2's declared cluster list names must resolve to an axis
    /// slot, and the two commands' argv discipline is the same as `pairs`': an
    /// unknown flag or an unsupported player count is an ERROR, never a run
    /// that quietly measures something else.
    #[test]
    fn the_two_instruments_refuse_rather_than_measure_the_wrong_thing() {
        for (name, labels) in DECLARED_CLUSTERS {
            for label in *labels {
                slot_named(label).unwrap_or_else(|e| panic!("cluster {name:?}: {e}"));
            }
        }

        let arg = |s: &str| s.to_string();
        // 3–4P is refused BY NAME: at more than one opponent the relative
        // currency's "theirs" is `opponent_extreme`, which orients by weight,
        // and a weight-oriented column is not a weight-free diagnostic.
        assert!(cmd_collinearity(&[arg("3"), arg("10")])
            .is_err_and(|e| e.contains("2P only") && e.contains("got 3")));
        assert!(cmd_collinearity(&[arg("2")]).is_err_and(|e| e.contains("<players> <games>")));
        assert!(cmd_collinearity(&[arg("2"), arg("4"), arg("--os-entropi")]).is_err());
        assert!(cmd_collision(&[arg("4(a)")]).is_err_and(|e| e.contains("no positional")));
        assert!(cmd_collision(&[arg("--fixtures-onli")]).is_err());
        // The fixture half needs no corpus, no root and no master, so it is
        // cheap enough to run as a cell — and running it here is what keeps the
        // command's own glue from rotting between artifacts.
        assert!(cmd_collision(&[arg("--fixtures-only")]).is_ok());
        assert!(cmd_rooted(&[arg("4(a)")]).is_err_and(|e| e.contains("no positional")));
        assert!(cmd_rooted(&[arg("--no-generatd")]).is_err());
    }

    // -----------------------------------------------------------------------
    // §7 step 5 — the rooted-read gates
    //
    // CORPUS-FREE by construction (the driven convention): the gate ARITHMETIC,
    // the registered criteria, the charter's split and its burn guard are all
    // exercised here on constructed material. The gates that need the owner
    // saves or the regenerated game-3 record run in the COMMAND, and the
    // orchestrator records their output.
    // -----------------------------------------------------------------------

    /// A 2P state with a real generated board, for the cells that need one —
    /// the same shape `observed_view` above builds, kept state-side.
    fn two_player_state() -> (GameState, Rules) {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 7)
            .expect("a standard 2P setup");
        let choices: Vec<_> = (0..2).map(|_| StartingOrderChoice::Keep(0)).collect();
        let state = finalize_setup(pending, &choices).expect("the setup finalizes");
        (state, rules)
    }

    /// The three column groups are SLICES of the canonical array, not re-typed
    /// lists — so a re-ordering of `CandidateFeature::ALL` cannot leave a group
    /// describing the old order while still compiling.
    #[test]
    fn the_step_five_column_groups_are_the_canonical_arrays_own_slices() {
        assert_eq!(S_COLUMNS.as_slice(), &CandidateFeature::ALL[0..7], "S1..S7");
        assert_eq!(OSE_COLUMNS.as_slice(), &CandidateFeature::ALL[10..13], "O1, O2, O3");
        assert_eq!(X_COLUMNS.as_slice(), &CandidateFeature::ALL[15..18], "X1, X2, X3");
        // The labels are the ones the gate texts are written in.
        assert_eq!(
            S_COLUMNS.map(CandidateFeature::label),
            ["S1", "S2", "S3", "S4", "S5", "S6", "S7"],
        );
        assert_eq!(OSE_COLUMNS.map(CandidateFeature::label), ["O1", "O2", "O3"]);
        assert_eq!(X_COLUMNS.map(CandidateFeature::label), ["X1", "X2", "X3"]);
    }

    /// `s_slots` reads the S SLOTS of a 34-wide Δv and nothing else.
    ///
    /// MUTATION-RELEVANT: a slot arithmetic that read incumbent indices would
    /// still produce seven numbers and still sum, so the cell plants a distinct
    /// value in every axis slot and names the seven it expects back.
    #[test]
    fn s_slots_reads_the_seven_candidate_slots_and_no_others() {
        let mut dv = [0i64; AXIS_WIDTH];
        for slot in FeatureId::all() {
            // A value unique to each slot, so a wrong index cannot coincide.
            dv[slot.index()] = 100 + slot.index() as i64;
        }
        let expected: [i64; 7] =
            S_COLUMNS.map(|c| 100 + FeatureId::Candidate(c).index() as i64);
        assert_eq!(s_slots(&dv), expected);
        // Every incumbent slot is EXCLUDED — the strongest statement of "and no
        // others" that does not depend on the layout's numbering.
        for f in Feature::ALL {
            let value = 100 + FeatureId::Incumbent(f).index() as i64;
            assert!(!s_slots(&dv).contains(&value), "{} leaked into the S group", f.name());
        }
    }

    /// **INTERPRETATION NOTE 1, EXECUTABLE.** The e113 gate quantity is the
    /// UNWEIGHTED CROSS-UNIT SUM, and it is deliberately NOT a majority vote
    /// over the seven columns.
    ///
    /// The contrast is the point: a Δ that favours the enabling side on SIX of
    /// seven columns and loses one column by a large margin FAILS the frozen
    /// gate. A "most columns agree" reading would pass it. The frozen text says
    /// "the summed delta", so the sum is what the pass rule reads — and this
    /// cell is what stops a future reader from quietly substituting the more
    /// intuitive rule.
    #[test]
    fn the_e113_gate_sums_the_seven_columns_and_does_not_vote_on_them() {
        let plant = |values: [i64; 7]| {
            let mut dv = [0i64; AXIS_WIDTH];
            for (i, c) in S_COLUMNS.iter().enumerate() {
                dv[FeatureId::Candidate(*c).index()] = values[i];
            }
            dv
        };
        let enabling = plant([1, 1, 1, 1, 1, 1, -10]);
        let dead = plant([0, 0, 0, 0, 0, 0, 0]);
        let per_column: Vec<i64> =
            (0..7).map(|i| s_slots(&enabling)[i] - s_slots(&dead)[i]).collect();
        let sum: i64 = per_column.iter().sum();

        assert_eq!(sum, -4, "the unweighted cross-unit sum");
        assert!(sum <= 0, "the FROZEN gate does not pass on this delta");
        assert_eq!(
            per_column.iter().filter(|d| **d > 0).count(),
            6,
            "…while six of seven columns favour the enabling side, which a MAJORITY reading \
             would have passed. The two readings disagree, and §6's text picks the sum",
        );

        // THE PASS RULE ITSELF, not only the arithmetic beside it (review
        // SHOULD-2). The cells above pin what the SUM is; without this one a
        // majority vote substituted into `E113Gate::passed` would trip nothing,
        // because no cell had ever asked the gate whether it passed.
        let planted = |values: [i64; 7]| {
            let mut per_column = [0i64; 7];
            per_column.copy_from_slice(&values);
            gate_over(per_column, Vec::new())
        };
        assert!(
            !planted([1, 1, 1, 1, 1, 1, -10]).direction_favours_enabling(),
            "SIX of seven columns favour the enabling side and the SUM is -4: the frozen read \
             says NO, and a majority reading substituted into it would say yes",
        );
        assert!(
            planted([1, 1, 1, 1, 1, 1, -5]).direction_favours_enabling(),
            "sum +1 — the same six-of-seven shape the other way round, so the cell above is about \
             the SUM and not about the shape",
        );

        // And the sum is genuinely strict: an exactly-balanced delta does not
        // favour anything, because the text says "strictly favors".
        assert_eq!(planted([3, -3, 0, 0, 0, 0, 0]).sum, 0);
        assert!(
            !planted([3, -3, 0, 0, 0, 0, 0]).direction_favours_enabling(),
            "a zero sum does not STRICTLY favour either side",
        );

        // WHAT THIS CELL NO LONGER SAYS, and why. Until the 2026-08-23 ruling it
        // ended by pinning the gate as CONJUNCTIVE — sum AND non-collision — and
        // that assertion was CORRECT for the frozen gates. The ruling
        // re-registers the direction claim forward to step 9(b) and leaves
        // non-collision as the step-5 gate, so the conjunction is no longer the
        // rule and asserting it here would pin a registration that has been
        // superseded. The two halves are now pinned where they belong: the
        // direction as a RECORDED read above, and non-collision as a gate in
        // `a_blocking_collision_at_the_exemplar_pair_still_stops_step_five`.
        assert!(
            !gate_over(
                [1, 1, 1, 1, 1, 1, -5],
                vec![Blocking::RootPair { root: E113_ROOT.to_string() }],
            )
            .non_collision(),
            "a colliding pair reads as colliding whatever the sum says",
        );
    }

    /// A [`Verdict`] over the shape the instrument actually measures at HEAD —
    /// the e113 direction failing, the two structurally unviewable exhibits, and
    /// everything else passing.
    fn measured_verdict() -> Verdict {
        let mut verdict = Verdict::new();
        // GATE 1: the measured deltas [1,-3,-1,1,0,0,0], sum -2, no collision.
        verdict.take_e113(&gate_over([1, -3, -1, 1, 0, 0, 0], Vec::new()));
        // GATE 2: four ordinals, each separating.
        for name in ["m-pf-dev-0", "m-pf-dev-1", "m-pf-dev-2", "m-pf-dev-3"] {
            verdict.take_fork(&fork_over(name, 6));
        }
        // GATE 3: the six constructed cells, all as declared.
        for (pair, side, expect, o3) in [
            ("4(b)", "EXECUTABLE", ">= 1", 1),
            ("4(b)", "DISTANT", "= 0", 0),
            ("4(a)", "MATCHED", "= 0", 0),
        ] {
            verdict.take_o3(&O3Cell {
                pair,
                stage: "early",
                side,
                expect,
                o3,
                ok: true,
            });
        }
        // GATE 4: three that read and pass, two structurally unviewable.
        for (event, columns) in [(111, [32, 6, 0]), (159, [18, 4, 0]), (198, [14, 3, 0])] {
            verdict.take_exhibit(&exhibit_read(event, ExhibitOutcome::Read {
                columns,
                mover_columns: [0, 0, 0],
                opponent: PlayerId(1),
                hand: (1, 1),
            }));
        }
        for event in [34, 68] {
            verdict.take_exhibit(&exhibit_read(
                event,
                ExhibitOutcome::NotRun {
                    reason: "seat 0's own starting-order deal was never recorded; it cannot view \
                             itself"
                        .to_string(),
                },
            ));
        }
        verdict
    }

    fn exhibit_read(event_index: usize, outcome: ExhibitOutcome) -> ExhibitRead {
        ExhibitRead { event_index, digest: "0".repeat(16), mover: PlayerId(0), outcome }
    }

    fn fork_over(name: &str, distinct_vectors: usize) -> ForkRead {
        ForkRead {
            name: name.to_string(),
            origin: String::new(),
            digest: "0".repeat(16),
            witness: 1,
            totals: vec![TotalRead {
                total: 1,
                members: 10,
                distinct_vectors,
                distinct_per_column: [3, 2, 1, 1, 2, 2, 1],
                distinct_complete: 8,
                distinct_incumbent: 1,
            }],
        }
    }

    /// **THE RE-REGISTERED GATE SET CLEARS ON THE MEASURED SHAPE.**
    ///
    /// guidingpath 2026-08-23, "THE TWO STEP-5 FINDINGS ARE RULED": the e113
    /// direction claim re-registers forward to step 9(b) and the OSE development
    /// band re-registers as {e111, e159, e198} with e34/e68 recorded
    /// NOT-RUN-STRUCTURAL. What remains a step-5 gate is the exemplar pair's
    /// NON-COLLISION, GATE 2, GATE 3 and the re-registered GATE 4 — and on the
    /// measured data all of them pass, so the run clears.
    #[test]
    fn the_re_registered_gate_set_clears_on_the_measured_shape() {
        let verdict = measured_verdict();
        assert!(
            verdict.clears(),
            "step 5 clears under the re-registered gate set; failures {:?}",
            verdict.failures,
        );
    }

    /// The e113 DIRECTION read is recorded and is no longer a verdict input.
    ///
    /// Its CONTROL sits in the same cell: a blocking collision at the same pair
    /// still stops the run, so what changed is which half of GATE 1 gates — not
    /// that GATE 1 stopped mattering.
    #[test]
    fn the_e113_direction_failure_is_recorded_and_does_not_stop_step_five() {
        let mut recorded_only = Verdict::new();
        recorded_only.take_e113(&gate_over([1, -3, -1, 1, 0, 0, 0], Vec::new()));
        assert!(
            recorded_only.clears(),
            "the direction claim re-registers to step 9(b): FAILED-AS-REGISTERED at step 5 is a \
             RECORDING, not a stop. failures {:?}",
            recorded_only.failures,
        );
        assert!(
            !recorded_only.failures.iter().any(|f| f.contains("GATE 1")),
            "…and it is not filed as a GATE 1 failure either",
        );
    }

    /// A blocking collision at the exemplar pair STILL stops step 5.
    #[test]
    fn a_blocking_collision_at_the_exemplar_pair_still_stops_step_five() {
        let mut colliding = Verdict::new();
        // Direction PASSING, so only the collision can be doing the work.
        colliding.take_e113(&gate_over(
            [1, 1, 1, 1, 1, 1, 1],
            vec![Blocking::RootPair { root: E113_ROOT.to_string() }],
        ));
        assert!(
            !colliding.clears(),
            "non-collision is step-5 content and STAYS a gate — the ruling re-registered the \
             direction claim, not the whole of GATE 1",
        );
        assert!(colliding.failures.iter().any(|f| f.contains("GATE 1")));
    }

    /// A structurally unviewable exhibit is the REGISTERED SHAPE, not a stop —
    /// with the control that a root which DID read and failed still stops.
    #[test]
    fn a_structurally_unviewable_exhibit_does_not_stop_step_five() {
        let mut structural = Verdict::new();
        structural.take_exhibit(&exhibit_read(
            34,
            ExhibitOutcome::NotRun { reason: "ViewerDealUnrecorded".to_string() },
        ));
        assert!(
            structural.clears(),
            "e34/e68 are re-registered OUT of the development band and recorded \
             NOT-RUN-STRUCTURAL; they no longer stop the run",
        );

        // THE CONTROL. A root that DID read and read the wrong way is still a
        // gate failure, so the arm above is about unviewability and not about
        // GATE 4 having been switched off.
        let mut wrong = Verdict::new();
        wrong.take_exhibit(&exhibit_read(111, ExhibitOutcome::Read {
            columns: [0, 6, 0],
            mover_columns: [0, 0, 0],
            opponent: PlayerId(1),
            hand: (1, 1),
        }));
        assert!(!wrong.clears(), "opponent-O1 = 0 at a re-registered dev root still fails");
        let mut executable = Verdict::new();
        executable.take_exhibit(&exhibit_read(111, ExhibitOutcome::Read {
            columns: [32, 6, 1],
            mover_columns: [0, 0, 0],
            opponent: PlayerId(1),
            hand: (1, 1),
        }));
        assert!(!executable.clears(), "opponent-O3 = 1 still fails the strict-honesty clause");
    }

    /// An [`E113Gate`] over planted columns, for the cells that exercise the
    /// PASS RULE rather than the measurement.
    ///
    /// Everything the pass rule does not read is filled with obvious placeholder
    /// material: `passed()` reads `sum` and `blocking` and nothing else, and
    /// spelling that out here is what keeps a future field from being quietly
    /// added to the rule without a cell noticing.
    fn gate_over(per_column: [i64; 7], blocking: Vec<Blocking>) -> E113Gate {
        E113Gate {
            root_digest: String::new(),
            candidates: 0,
            top_value: 0,
            tie: 0,
            echo: String::new(),
            decision_seed: 0,
            lab: AreaId(0),
            fishing: AreaId(0),
            total: 2,
            eligible: 0,
            enabling: String::new(),
            enabling_value: None,
            dead: String::new(),
            enabling_digest: String::new(),
            dead_digest: String::new(),
            per_column,
            sum: per_column.iter().sum(),
            absolute_per_column: per_column,
            absolute_sum: per_column.iter().sum(),
            spread: Vec::new(),
            blocking,
            enabling_dv: [0; AXIS_WIDTH],
            dead_dv: [0; AXIS_WIDTH],
            enabling_ppm: 0,
            dead_ppm: 0,
        }
    }

    /// **INTERPRETATION NOTE 2, EXECUTABLE.** The fork gate's "≥ 2 distinct
    /// values" is the VECTOR reading, and the per-column reading is a different
    /// — and at real roots, false — statement.
    ///
    /// The planted set has three members whose S5 and S7 are CONSTANT (no member
    /// reaches the Bank; none changes the president's reach) and whose vectors
    /// are nonetheless distinct. The vector reading passes; a rule demanding
    /// that every column move fails. The run artifact shows the same shape at
    /// every M-PF development ordinal, which is why the note exists.
    #[test]
    fn the_fork_gate_reads_the_s_vector_and_not_each_column_alone() {
        let members: Vec<[i64; 7]> = vec![
            [1, 0, 2, 1, 0, 3, 4],
            [2, 0, 2, 1, 0, 3, 4],
            [1, 0, 3, 1, 0, 3, 4],
        ];
        let (vectors, per_column) = distinct_s(&members);

        assert_eq!(vectors, 3, "three distinct S signatures");
        assert!(vectors >= 2, "the VECTOR reading — the frozen gate — passes");
        assert_eq!(per_column, [2, 1, 2, 1, 1, 1, 1]);
        assert!(
            per_column.iter().any(|d| *d < 2),
            "…while FOUR of the seven columns are constant across the set, so a per-column \
             reading would FAIL a set the frozen gate passes",
        );

        // The anti-collapse case the gate exists to catch: one point, however
        // many members.
        let collapsed = vec![[1, 2, 3, 4, 5, 6, 7]; 9];
        assert_eq!(distinct_s(&collapsed).0, 1, "a collapsed block reads ONE vector");
        assert!(distinct_s(&collapsed).0 < 2, "and fails the gate, which is the point");
    }

    /// **[`E113_ABSOLUTE_NOTE`]'s central claim, pinned STRUCTURALLY rather than
    /// observed once**: a mover's PLACEMENT cannot move any term of the
    /// OPPONENT's S vector, which is why the two currencies coincide on the e113
    /// family and why the run prints them equal at every row of the spread.
    ///
    /// The review that produced this cell also produced the reason it exists: an
    /// earlier version of the note asserted the OPPOSITE — that
    /// `realizable_power` counts every assistant at an area including the
    /// opponent's — which is false at the bytes (`eval.rs:2411-2430` sums the
    /// president term plus the QUERIED seat's own assistants, shophouse and
    /// trading house). Prose about a value model is exactly the thing that
    /// should not be trusted unexecuted, so the corrected claim arrives with a
    /// cell attached.
    ///
    /// Corpus-free: a freshly generated 2P game supplies the fork. The
    /// anti-vacuity half matters as much as the invariance half — a position
    /// where NO placement moved the MOVER's S vector either would satisfy the
    /// invariance assertion while proving nothing.
    #[test]
    fn a_movers_placement_cannot_move_the_opponents_s_vector() {
        let (state, rules) = two_player_state();
        let mover = state.current_player;
        let placing = apply_move(&state, &Move::EndAdditionalPhase, &rules)
            .expect("a turn start advances to Placement");
        let opponent = placing
            .players
            .iter()
            .map(|p| p.id)
            .find(|id| *id != mover && !id.is_dummy())
            .expect("a 2P game has one opponent");

        let before = observe(&placing, mover).expect("the mover views the placement phase");
        let opponent_before =
            seat_s_values(&before, opponent, &rules).expect("the opponent's S vector");
        let mover_before = seat_s_values(&before, mover, &rules).expect("the mover's S vector");

        let mut placements = 0usize;
        let mut mover_moved = false;
        for mv in get_legal_moves(&placing, &rules) {
            if placement_of(&mv).is_none() {
                continue;
            }
            placements += 1;
            let next = apply_move(&placing, &mv, &rules).expect("a legal placement applies");
            let view = observe(&next, mover).expect("the successor is observable");
            assert_eq!(
                seat_s_values(&view, opponent, &rules).expect("the opponent's S vector"),
                opponent_before,
                "{mv:?} moved the OPPONENT's S vector. Every S reads the queried seat's own \
                 occupancy and `realizable_power` sums that seat's OWN assistants, shophouse and \
                 trading house — the only board-global term is the station, which no placement \
                 relocates",
            );
            if seat_s_values(&view, mover, &rules).expect("the mover's S vector") != mover_before {
                mover_moved = true;
            }
        }

        assert!(placements >= 2, "a fork needs at least two placements; got {placements}");
        assert!(
            mover_moved,
            "ANTI-VACUITY: no placement moved the MOVER's own S vector either, so the invariance \
             above would hold at a position where the columns see nothing at all",
        );
    }

    /// The dead-side criterion is a FILTER plus a TOTAL ORDER, and both halves
    /// are load-bearing.
    ///
    /// The planted tie contains: a member sharing an area with the enabling side
    /// (excluded by disjointness), a member of a different total (excluded by
    /// same-count), and three eligible ones whose canonical encodings order
    /// them. "First iterated" would have returned the tie's first element, which
    /// this cell deliberately makes the WRONG answer.
    #[test]
    fn the_dead_side_criterion_filters_then_takes_the_least_canonical_encoding() {
        let place = |areas: &[u8]| Move::PlaceThreeAreas {
            placements: areas.iter().map(|a| AreaId(*a)).collect(),
        };
        let enabling: BTreeSet<AreaId> = [AreaId(0), AreaId(9)].into_iter().collect();
        let tie = vec![
            place(&[5, 8]),    // eligible, but not the least
            place(&[0, 3]),    // SHARES area 0 with the enabling side
            place(&[1, 2, 4]), // a different total
            place(&[1, 2]),    // eligible AND the least
            Move::PlaceTwoSameArea { area: AreaId(3), adjacent_bonus: None }, // eligible, total 2
            Move::EndAdditionalPhase, // not a placement at all
        ];

        let eligible = eligible_dead_sides(&tie, &enabling, 2);
        assert_eq!(eligible.len(), 3, "two exclusions and one non-placement dropped");
        assert_eq!(
            eligible[0],
            &place(&[1, 2]),
            "the LEXICOGRAPHICALLY LEAST canonical encoding, not the first iterated",
        );
        assert_ne!(eligible[0], &tie[0], "first-iterated would have chosen differently");
        assert!(
            eligible.iter().all(|mv| placement_of(mv)
                .is_some_and(|(t, a)| t == 2 && a.is_disjoint(&enabling))),
            "every survivor is same-count AND disjoint",
        );
        // The order is the derived Debug rendering, which is what the artifact
        // prints — so the criterion is re-checkable from the printed output.
        let encodings: Vec<String> = eligible.iter().map(|mv| canonical_encoding(mv)).collect();
        let mut sorted = encodings.clone();
        sorted.sort();
        assert_eq!(encodings, sorted);
        assert_eq!(canonical_encoding(&place(&[1, 2])), format!("{:?}", place(&[1, 2])));
    }

    /// The FROZEN e113 registration is internally consistent with the criterion
    /// that chose it — checkable without the gitignored save.
    ///
    /// The command re-derives the pair from a loaded root and refuses on any
    /// disagreement; this cell checks the half that needs no root: same count,
    /// disjoint area sets, encodings that reconstruct from the frozen ids, and
    /// digests of the shape `state_digest` produces.
    #[test]
    fn the_frozen_e113_registration_agrees_with_its_own_criterion() {
        let e = E113_EXEMPLAR;
        let enabling: BTreeSet<AreaId> = e.enabling_areas.iter().map(|a| AreaId(*a)).collect();
        let dead: BTreeSet<AreaId> = e.dead_areas.iter().map(|a| AreaId(*a)).collect();

        assert_eq!(enabling.len(), e.enabling_areas.len(), "the frozen ids are distinct");
        assert_eq!(dead.len(), e.dead_areas.len());
        assert!(enabling.is_disjoint(&dead), "the criterion's DISJOINTNESS clause");
        assert_eq!(e.total, 2, "both sides place two assistants");

        for (label, areas, encoding) in [
            ("enabling", e.enabling_areas, e.enabling_encoding),
            ("dead", e.dead_areas, e.dead_encoding),
        ] {
            let mv = Move::PlaceThreeAreas {
                placements: areas.iter().map(|a| AreaId(*a)).collect(),
            };
            assert_eq!(canonical_encoding(&mv), encoding, "{label}: ids and encoding disagree");
            let (total, set) = placement_of(&mv).expect("a placement");
            assert_eq!(total, e.total, "{label}: the frozen total");
            assert_eq!(set.len(), areas.len(), "{label}");
        }
        for (label, digest) in [
            ("root", E113_ROOT_DIGEST),
            ("enabling successor", e.enabling_digest),
            ("dead successor", e.dead_digest),
        ] {
            assert_eq!(digest.len(), 16, "{label}: state_digest is 16 hex characters");
            assert!(digest.chars().all(|c| c.is_ascii_hexdigit()), "{label}");
        }
        assert_ne!(e.enabling_digest, e.dead_digest, "two sides, two successors");
    }

    /// `sole_area_of` identifies a site by TYPE and refuses an ambiguous board.
    ///
    /// The refusal is not decorative: the enabling side is registered by SITE
    /// NAME ("Laboratory A + Fishing Ground"), so a board where the name picks
    /// out two tiles is a board where the registration does not identify a
    /// placement. The duplicate is planted rather than hunted for, so the cell
    /// says the same thing at every board the generator produces.
    #[test]
    fn sole_area_of_names_one_tile_or_refuses() {
        let (state, _) = two_player_state();
        let lab = sole_area_of(&state, AreaType::LaboratoryA).expect("Laboratory A is unique");
        assert_eq!(state.board_layout.areas[&lab].area_type, AreaType::LaboratoryA);

        let mut ambiguous = state.clone();
        let other = *ambiguous
            .board_layout
            .areas
            .keys()
            .find(|id| **id != lab)
            .expect("a second area");
        ambiguous.board_layout.areas.get_mut(&other).expect("present").area_type =
            AreaType::LaboratoryA;
        assert!(sole_area_of(&ambiguous, AreaType::LaboratoryA)
            .is_err_and(|e| e.contains("appears 2 times") && e.contains("does not identify")));

        let mut absent = state.clone();
        absent.board_layout.areas.get_mut(&lab).expect("present").area_type = AreaType::Canal;
        assert!(sole_area_of(&absent, AreaType::LaboratoryA)
            .is_err_and(|e| e.contains("is not on this board")));
    }

    /// The charter's EVERY-THIRD rule REPRODUCES its own two transcribed lists.
    ///
    /// The ruling names both the rule and its answer. The honest implementation
    /// computes the rule; this cell checks it lands on the answer, so a
    /// transcription slip in either list is a failure here rather than a
    /// silently different band eight ordinals wide.
    #[test]
    fn the_ose_split_is_the_charters_every_third_rule_over_event_order() {
        let derived_dev: Vec<usize> = OSE_EXHIBITS
            .iter()
            .filter(|(e, _)| ose_role(*e) == Ok(OseRole::Development))
            .map(|(e, _)| *e)
            .collect();
        let derived_reserved: Vec<usize> = OSE_EXHIBITS
            .iter()
            .filter(|(e, _)| ose_role(*e) == Ok(OseRole::Reserved))
            .map(|(e, _)| *e)
            .collect();

        assert_eq!(derived_dev, OSE_DEVELOPMENT, "the charter's five development exhibits");
        assert_eq!(derived_reserved, OSE_RESERVED, "the charter's eight reserved exhibits");
        assert_eq!(OSE_EXHIBITS.len(), OSE_DEVELOPMENT.len() + OSE_RESERVED.len());
        assert_eq!(
            OSE_RESERVED.len(),
            8,
            "exactly §6's {{8..16}} band size, so the >= 6-of-8 block-pass rule applies unchanged",
        );

        // Event order is the rule's INPUT and the list must be in it.
        let events: Vec<usize> = OSE_EXHIBITS.iter().map(|(e, _)| *e).collect();
        let mut sorted = events.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(events, sorted, "thirteen distinct event indices in ascending order");

        // Thirteen distinct identities, each a state_digest.
        let digests: BTreeSet<&str> = OSE_EXHIBITS.iter().map(|(_, d)| *d).collect();
        assert_eq!(digests.len(), 13, "thirteen distinct positions");
        assert!(OSE_EXHIBITS
            .iter()
            .all(|(_, d)| d.len() == 16 && d.chars().all(|c| c.is_ascii_hexdigit())));

        // An event the charter never registered is not classifiable at all.
        assert!(ose_role(35).is_err_and(|e| e.contains("not one of the thirteen")));
        assert!(ose_role(0).is_err());
    }

    /// **THE BURN GUARD**, and its mutation control in the same cell.
    ///
    /// A guard that refused EVERYTHING would satisfy the refusal half and be
    /// useless, so the development arm is asserted in the same breath: the same
    /// view, the same seat, the same columns — only the role differs, and the
    /// role is what decides.
    #[test]
    fn the_burn_guard_refuses_every_reserved_exhibit_and_reads_every_development_one() {
        let rules = Rules::standard();
        let fixture = build_registered_pair(registered("4(b)").expect("4(b)"), StageEndpoint::Early, &rules)
            .expect("4(b) early");
        let view = fixture.view(Side::Greater);
        let subject = view
            .players
            .iter()
            .find(|p| p.id == fixture.subject())
            .expect("the subject is in its own view");

        for event in OSE_RESERVED {
            let refusal = ose_columns(OseRole::Reserved, event, view, subject, &rules)
                .expect_err("a reserved exhibit REFUSES a column read");
            assert!(refusal.contains("REFUSED"), "e{event}");
            assert!(refusal.contains("ESCROW-RESERVED"), "e{event}");
            assert!(refusal.contains("2026-08-23"), "e{event}: the refusal names the charter row");
            assert!(refusal.contains("before step 10"), "e{event}: …and until when");
            assert!(refusal.contains(&format!("event {event}")), "e{event}: …and which exhibit");
        }

        // THE CONTROL. Same everything, Development role: the read happens.
        for event in OSE_DEVELOPMENT {
            let columns = ose_columns(OseRole::Development, event, view, subject, &rules)
                .expect("a development exhibit READS");
            assert_eq!(columns.len(), 3);
        }

        // And the guard reads the SAME numbers the adapter does, so the guard
        // is a gate on ACCESS and not a second derivation of the columns.
        let structure = ScoreStructure::derive(view, &rules);
        let direct: Vec<i64> = OSE_COLUMNS
            .iter()
            .map(|c| candidate_value(&structure, view, subject, *c, &rules))
            .collect();
        assert_eq!(
            ose_columns(OseRole::Development, 34, view, subject, &rules).expect("reads"),
            direct.as_slice(),
        );
    }

    /// The record identity check refuses a document that is not the recorded
    /// game 3, and names every key that disagreed.
    ///
    /// A freshly generated 2P record shares NONE of the five recorded keys, so
    /// it exercises the refusal without a file on disk — which is what makes
    /// the check reachable in a corpus-free suite at all.
    #[test]
    fn the_ose_record_identity_refuses_a_document_the_charter_does_not_index() {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let pending = new_game(config.clone(), &rules, 7).expect("a standard 2P setup");
        let choices: Vec<_> = (0..2).map(|_| StartingOrderChoice::Keep(0)).collect();
        let _ = finalize_setup(pending, &choices).expect("the setup finalizes");
        let record = GameRecord::new(7, config, choices, &rules);

        let refusal = check_ose_record_identity(&record, &rules)
            .expect_err("a fresh record is not the recorded game 3");
        for key in ["run_id", "events", "terminal_digest"] {
            assert!(refusal.contains(key), "the refusal names {key}: {refusal}");
        }
        assert!(refusal.contains("REFUSING before any root resolves"));
        // The rev and the rules hash of a FRESH record are this build's, so
        // they are the two keys that legitimately AGREE — the cell says so
        // rather than asserting a refusal that names all five.
        assert!(!refusal.contains("semantics_rev"), "a fresh record carries this rev: {refusal}");
        assert!(!refusal.contains("is not this build's"), "…and this rules hash: {refusal}");
    }

    /// **GATE 3 CROSS-PINNED TO THE FROZEN PACKAGE.** The expectations the
    /// constructed O3 gate asserts are the levels the registrations DECLARE.
    ///
    /// Without this the gate is a second, independent opinion about what 4(a)
    /// and 4(b) contain, and a re-registration could move the fixture while the
    /// gate went on asserting the old numbers and passing.
    #[test]
    fn the_constructed_o3_gate_asserts_the_levels_the_registrations_declare() {
        let rules = Rules::standard();
        let cells = read_constructed_o3(&rules).expect("the constructed cells build");
        assert_eq!(cells.len(), 6, "4(b) at both endpoints, 4(a) at its one");
        assert!(cells.iter().all(|c| c.ok), "every constructed O3 cell passes");

        let declared = |id: &str, key: &str| -> i64 {
            registered(id)
                .unwrap_or_else(|| panic!("{id} is registered"))
                .parameters
                .iter()
                .find(|(name, _)| *name == key)
                .unwrap_or_else(|| panic!("{id} declares {key}"))
                .1
        };
        for cell in &cells {
            let expected = match (cell.pair, cell.side) {
                ("4(b)", "EXECUTABLE") => declared("4(b)", "o3_greater"),
                ("4(b)", "DISTANT") => declared("4(b)", "o3_lesser"),
                ("4(a)", _) => declared("4(a)", "o3_both_sides"),
                other => panic!("unregistered cell {other:?}"),
            };
            assert_eq!(
                cell.o3, expected,
                "{} {} {}: the column reads {} and the registration declares {expected}",
                cell.pair, cell.stage, cell.side, cell.o3,
            );
        }
        // The specificity the flag-1 ruling asks for: exactly ONE of the three
        // registered sides carries strict executability, and it is 4(b)'s
        // EXECUTABLE side. A column that returned 1 everywhere would pass the
        // ">= 1" half and fail here.
        assert_eq!(cells.iter().filter(|c| c.o3 >= 1).count(), 2, "4(b) EXECUTABLE at both stages");
        assert!(cells
            .iter()
            .filter(|c| c.o3 >= 1)
            .all(|c| c.pair == "4(b)" && c.side == "EXECUTABLE"));
        assert!(cells.iter().filter(|c| c.side != "EXECUTABLE").all(|c| c.o3 == 0));
    }

    /// The X wiring proof counts a column present when EITHER seat reads
    /// nonzero, and the artifact says so when a column reads zero everywhere.
    #[test]
    fn the_x_wiring_read_counts_a_column_present_at_either_seat() {
        let read = |values: [(i64, i64); 3]| XWiringRead {
            root: "t".to_string(),
            digest: "0".repeat(16),
            origin: "t".to_string(),
            note: "t",
            values,
        };
        assert_eq!(read([(0, 0), (0, 0), (0, 0)]).nonzero(), 0);
        assert_eq!(read([(1, 0), (0, 0), (0, 0)]).nonzero(), 1, "the mover alone counts");
        assert_eq!(read([(0, 0), (0, 4), (0, 0)]).nonzero(), 1, "the opponent alone counts");
        assert_eq!(read([(-1, 0), (0, 0), (2, 3)]).nonzero(), 2, "a negative is not absent");
    }

    /// An exhibit that could not be READ is neither a pass nor a fail.
    ///
    /// The three-way status is the whole reason `ExhibitOutcome` exists: a
    /// boolean would have to call an unmeasured root one of the two, and both
    /// answers are false.
    #[test]
    fn an_unreadable_exhibit_is_neither_a_pass_nor_a_fail() {
        let at = |outcome| ExhibitRead {
            event_index: 34,
            digest: "0".repeat(16),
            mover: PlayerId(0),
            outcome,
        };
        let read = |columns| ExhibitOutcome::Read {
            columns,
            mover_columns: [0, 0, 0],
            opponent: PlayerId(1),
            hand: (1, 1),
        };
        assert_eq!(at(read([32, 6, 0])).passed(), Some(true), "O1 >= 1 and O3 = 0");
        assert_eq!(at(read([0, 6, 0])).passed(), Some(false), "O1 = 0 fails the first clause");
        assert_eq!(at(read([32, 6, 1])).passed(), Some(false), "O3 = 1 fails the second");
        assert_eq!(
            at(ExhibitOutcome::NotRun { reason: "unviewable".to_string() }).passed(),
            None,
            "NOT RUN is a third answer, not a false",
        );
        assert!(at(ExhibitOutcome::NotRun { reason: "unviewable".to_string() })
            .columns()
            .is_none());
    }
}
