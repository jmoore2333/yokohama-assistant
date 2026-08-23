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
//! ```text
//! cargo run --release -p yokohama-core --example eval_probe -- \
//!     run 2 2000 --contestant greedy-v1 --master 0xE7A1202607270001
//! cargo run --release -p yokohama-core --example eval_probe -- \
//!     run 2 200 --contestant family-weighted --blocks proxy
//! cargo run --release -p yokohama-core --example eval_probe -- \
//!     run 2 400 --contestant lookahead-v1 --master 0xE7A1202608090001
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
use yokohama_core::game::pair_fixtures::{
    build_registered_pair, package_digest, registered, render_pair_report_with,
    render_registry_manifest, CandidateFeature, StageEndpoint, REGISTERED,
};
use yokohama_core::{
    agent_stream_seed, finalize_setup, new_game, observe, replay_from, run_arena_game,
    run_raw_game, ArenaAgent, ArenaGameStats, Contestant, GameState, PlayerId, PlayerView,
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
fn candidate_columns(
    view: &PlayerView,
    player: &VisiblePlayer,
    feature: CandidateFeature,
    rules: &Rules,
) -> Option<i64> {
    let s = ScoreStructure::derive(view, rules);
    Some(match feature {
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
    })
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
        build_registered_pair, FeatureId, PairDeclaration, PairFixture, Side, REGISTERED,
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
}
