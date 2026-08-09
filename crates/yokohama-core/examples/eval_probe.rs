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
    achievement_progress_postclaim_sum, interpolate, progress_ppm, score, view_is_terminal,
    Feature, ScoreStructure, WeightTable, PROGRESS_FULL_PPM,
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
use yokohama_core::{
    agent_stream_seed, finalize_setup, new_game, observe, replay_from, run_arena_game,
    run_raw_game, ArenaAgent, ArenaGameStats, Contestant, GameState, PlayerId, PlayerView,
    QuantileSketch, RolloutPolicy, Rules, SeedSchedule, SeedScheduleKind, Slot, TriggerTally,
    WorldSeed, SEMANTICS_REV,
};

const USAGE: &str = "\
usage: eval_probe <subcommand> [options]

  run <players> <games> [--contestant <name>] [--weights v1|pre-freeze]
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
                             ones. <spec> is `v1` (the reproduction check),
                             `v2-draft`, or `<name>=<e0,...,e15>/<l0,...,l15>`
                             (sixteen i64 per vector, Feature::ALL order).
                             A candidate is EVALUATED here and never GENERATES
                             the corpus: `--weights` still chooses what the bot
                             plays and still takes only v1 | pre-freeze, because
                             the protocol's corpora are incumbent-play corpora.
      --candidate-columns <list>
                             Comma-separated extra ROWS in the features block,
                             measured beside the sixteen and shipped by nothing.
                             Today: `postclaim` (achievement_progress_sum with
                             the viewer's already-claimed cards dropped).
                             Feature::ALL stays sixteen everywhere. REFUSED
                             when --blocks omits `features` (an ignored column
                             would be a silent no-op).

  table [--weights v1|pre-freeze]
      Print a weight table with its provenance identity. No games are played.

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
        None | Some("v1") => Ok((WeightTable::V1, "v1")),
        Some("pre-freeze") => Ok((WeightTable::PRE_FREEZE_2026_07_27, "pre-freeze")),
        Some(other) => Err(format!("unknown --weights {other:?} (v1 | pre-freeze)")),
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
/// [`Contestant::build`]:** `build` hard-codes [`WeightTable::V1`], and this
/// instrument's entire `--weights pre-freeze` arm is the ability to run a corpus
/// under a DIFFERENT table. Delegating construction would make that flag a
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
}

impl CandidateColumn {
    /// The reported column name — deliberately the shipped feature's name plus a
    /// suffix, so a reader diffing the two rows sees they measure the same
    /// quantity under different rules.
    fn name(self) -> &'static str {
        match self {
            CandidateColumn::PostClaim => "achievement_progress_sum_postclaim",
        }
    }

    /// One seat's raw value for this column.
    fn value(self, view: &PlayerView, player: &yokohama_core::VisiblePlayer) -> i64 {
        match self {
            CandidateColumn::PostClaim => achievement_progress_postclaim_sum(view, player),
        }
    }

    /// Parse a comma-separated `--candidate-columns` list. An unknown name is an
    /// ERROR naming what was seen — `split_args`'s rule one level down, for its
    /// reason: a silently-dropped column would print a report missing the very
    /// row the run was made to look at.
    fn parse_list(raw: &str) -> Result<Vec<Self>, String> {
        raw.split(',')
            .filter(|s| !s.is_empty())
            .map(|name| match name {
                "postclaim" => Ok(CandidateColumn::PostClaim),
                other => Err(format!(
                    "unknown --candidate-columns entry {other:?} (postclaim)"
                )),
            })
            .collect()
    }
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
#[derive(Clone, Debug, PartialEq, Eq)]
struct ScoreTableSpec {
    name: String,
    table: WeightTable,
}

/// Parse one `--score-table` value.
///
/// Three forms, and nothing else parses:
///   * `v1` — the shipped table, i.e. the REPRODUCTION CHECK (run it beside a
///     real candidate and the candidate's numbers have something to be read
///     against on the same split);
///   * `v2-draft` — [`WeightTable::V2_DRAFT`], the round's iteration candidate;
///   * `<name>=<e0,…,e15>/<l0,…,l15>` — sixteen comma-separated `i64` per
///     vector, in [`Feature::ALL`] order, for a grid cell that has no const yet.
///
/// Every refusal names what it SAW (the importer discipline: an unrecognized
/// input is a typed error, never a silent default), and the vector arm checks
/// its lengths against `Feature::COUNT` rather than against a literal 16, so a
/// feature admission cannot leave this parser accepting a stale width.
fn parse_score_table(spec: &str) -> Result<ScoreTableSpec, String> {
    const FORMS: &str =
        "(v1 | v2-draft | <name>=<16 comma-separated i64>/<16 comma-separated i64>)";

    match spec {
        "v1" => {
            return Ok(ScoreTableSpec { name: "v1".to_string(), table: WeightTable::V1 })
        }
        "v2-draft" => {
            return Ok(ScoreTableSpec {
                name: "v2-draft".to_string(),
                table: WeightTable::V2_DRAFT,
            })
        }
        _ => {}
    }

    let Some((name, vectors)) = spec.split_once('=') else {
        return Err(format!("unknown --score-table {spec:?} {FORMS}"));
    };
    if name.is_empty() {
        return Err(format!(
            "--score-table {spec:?} has an empty name before the `=`; a candidate needs a name \
             to be reported under {FORMS}"
        ));
    }
    let Some((early, late)) = vectors.split_once('/') else {
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
            out[slot] = entry.trim().parse::<i64>().map_err(|e| {
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
fn candidate_total(
    view: &PlayerView,
    me: &yokohama_core::VisiblePlayer,
    structure: &ScoreStructure,
    table: &WeightTable,
    progress: u32,
    terminal: bool,
    shipped_total: f64,
) -> f64 {
    if terminal {
        return shipped_total;
    }
    let mut total: i64 = 0;
    for feature in Feature::ALL {
        let (early, late) = table.get(feature);
        let weight = interpolate(early, late, progress);
        let mine = structure.value_of(me, feature);
        // `eval::opponent_extreme`'s rule, reproduced: maximize `weight · value`
        // over the non-dummy opponents and subtract the winner's RAW value, with
        // ties (including every tie at `weight == 0`) keeping the FIRST opponent
        // in seat order. Unexercised at 2P, where the maximum is over one
        // opponent — but the corpora are not the only thing this runs on.
        let mut theirs: Option<i64> = None;
        for other in &view.players {
            if other.id == me.id || other.id.is_dummy() {
                continue;
            }
            let value = structure.value_of(other, feature);
            let improves = match theirs {
                None => true,
                Some(current) => weight.saturating_mul(value) > weight.saturating_mul(current),
            };
            if improves {
                theirs = Some(value);
            }
        }
        let theirs = theirs.unwrap_or(0);
        total = total.saturating_add(weight.saturating_mul(mine.saturating_sub(theirs)));
    }
    total as f64
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
                    .map(|p| c.value(view, p))
                    .max()
                    .unwrap_or(0);
                (c.value(view, me) as f64, theirs as f64)
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
            out_cand[slot] = candidate_total(
                view,
                me,
                &structure,
                &candidate.table,
                progress,
                terminal,
                total,
            );
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
}

/// A per-(game, seat, stage) accumulator, folded into a [`Cell`] at game end.
struct CellAcc {
    n: u64,
    rel: Vec<f64>,
    abs: Vec<f64>,
    cand: Vec<f64>,
}

impl CellAcc {
    fn new(rows: usize, candidates: usize) -> Self {
        CellAcc {
            n: 0,
            rel: vec![0.0; rows],
            abs: vec![0.0; rows],
            cand: vec![0.0; candidates],
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
            );
            let acc = &mut accs[usize::from(seat) * STAGES.len() + stage.index()];
            acc.n += 1;
            for slot in 0..rows.len() {
                acc.rel[slot] += rel[slot];
                acc.abs[slot] += abs[slot];
            }
            for (target, value) in acc.cand.iter_mut().zip(cand.iter()) {
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

/// Collect one candidate's cells for one stage and one side of the split.
fn split_column(corpus: &Corpus, candidate: usize, stage: Stage, split: Split) -> SplitColumn {
    let mut rows: Vec<(f64, u32, bool)> = corpus
        .cells
        .iter()
        .filter(|c| c.stage == stage && Split::of(c.game) == split)
        .map(|c| (c.cand[candidate], split.cluster(c.game), c.label))
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
/// candidate's own stage rows mutually incomparable. Across CANDIDATES the
/// streams differ by construction (see [`candidate_resample_seed`]), so two
/// candidates' intervals are not paired — stated in the block header rather than
/// left for a reader to assume either way.
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
         replicate is shared by a candidate's three stages, so those are jointly resampled and \
         two DIFFERENT candidates' intervals are not paired"
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

        let _ = writeln!(
            out,
            "\n  [{}] candidate={}  identity={identity:#018x}  vs corpus table: {}  \
             resample_seed={seed:#018x}",
            ordinal + 1,
            candidate.name,
            if identity == corpus_identity { "SAME" } else { "DIFFERENT" },
        );
        let _ = writeln!(out, "      early={:?}", candidate.table.early);
        let _ = writeln!(out, "      late ={:?}", candidate.table.late);
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
            "      {:<7} {:>9} {:>9}   {:<26} {:>13}",
            "stage", "fit auc", "held auc", "held-out 95% [lo, hi]", "cells fit/held",
        );
        for stage in STAGES {
            let s = stage.index();
            let (lo, hi) = intervals[s];
            let _ = writeln!(
                out,
                "      {:<7} {:>9} {:>9}   {:<26} {:>13}",
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
            );
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

    /// The three accepted spellings, and the shape of what they build.
    #[test]
    fn the_score_table_grammar_parses_its_three_forms() {
        assert_eq!(
            parse_score_table("v1"),
            Ok(ScoreTableSpec { name: "v1".to_string(), table: WeightTable::V1 }),
        );
        assert_eq!(
            parse_score_table("v2-draft"),
            Ok(ScoreTableSpec { name: "v2-draft".to_string(), table: WeightTable::V2_DRAFT }),
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
            ("v2", "v2"),
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
                &[ScoreTableSpec { name: "v1".to_string(), table: WeightTable::V1 }],
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
            ScoreTableSpec { name: "v1".to_string(), table: WeightTable::V1 },
            ScoreTableSpec { name: "v2-draft".to_string(), table: WeightTable::V2_DRAFT },
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

    /// A [`Corpus`] carrying nothing but cells — everything the split and the
    /// candidate columns read, and nothing they do not.
    fn test_corpus(cells: Vec<Cell>, games: u32) -> Corpus {
        Corpus {
            rows: Row::all(&[]),
            cells,
            games,
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
