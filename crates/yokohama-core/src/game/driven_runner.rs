//! The driven-fixture harness's RUNNER: argv in, one rendered report out.
//!
//! **Why this is a LIB module and not the example's body** (design §2). The
//! example `crates/yokohama-core/examples/driven.rs` is thin argv glue — parse,
//! call, print, exit — and everything it would otherwise contain lives here,
//! because an example's `#[cfg(test)]` module runs under NO command this repo
//! documents unless that command is written down at four doc sites (the
//! `eval_probe` precedent, and the standing rule that a suite no gate line names
//! is a suite that never runs). Putting the argv grammar and the rendering in
//! the lib means they are covered by `cargo test -p yokohama-core`, which every
//! per-edit gate already runs, and the harness ships no new test command at all.
//!
//! **What it does NOT do.** It computes nothing. Every number it prints comes
//! off [`driven`]'s own surface — [`driven::root_table`],
//! [`driven::drive_rooted`], [`driven::adjudicate_root`] — and this module only
//! decides which of them to call and how to lay the answer out. A runner that
//! re-derived a value would be a second derivation of a decision, which is the
//! thing `suggest`'s rule (spec §0.3) and this harness's MUST-1 resolution both
//! forbid.
//!
//! **Unknown is an ERROR at every level** — unknown subcommand, unknown flag,
//! unknown seat name, unknown contestant knob, a flag without its value, a
//! positional too many. `eval_probe`'s discipline, for its reason: a mistyped
//! flag that silently left a default in place puts a published figure under
//! knobs the run did not use.
//!
//! **The output is a `String`, not a stream of `println!`s.** That is what makes
//! rendering testable at all: a report is a value a cell can compare, hash, or
//! re-render for determinism, and the example's only job is to print it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::game::arena::Contestant;
use crate::game::driven::{
    self, Adjudication, DrivenOutcome, RegistryEntry, ReplyPolicy, RootDescriptor, RootTable,
    SeatSpec,
};
use crate::game::lookahead::SearchStats;
use crate::types::Move;

/// Every subcommand this runner answers. The parser reads it and
/// [`USAGE`]'s completeness pin asserts every member is documented.
pub const SUBCOMMANDS: &[&str] = &["list", "table", "run", "adjudicate"];

/// `list`'s flags: none.
pub const LIST_FLAGS: &[&str] = &[];

/// `table`'s flags.
pub const TABLE_FLAGS: &[&str] = &["contestant", "decision-seed"];

/// `run`'s flags. **`replications` is deliberately absent** — a single rooted
/// run has nothing to replicate, and accepting the flag would silently ignore
/// it.
pub const RUN_FLAGS: &[&str] = &["master", "seats"];

/// `adjudicate`'s flags.
pub const ADJUDICATE_FLAGS: &[&str] = &["master", "seats", "replications"];

/// The seat and contestant DEFAULT, spelled as the grammar spells it.
///
/// A string rather than a `Contestant` const on purpose: this is what a user
/// would type, `Contestant::from_str_name` fills every knob from its own shipped
/// defaults, and the runner therefore introduces no second opinion about what
/// "the shipped lookahead-v1" is.
pub const DEFAULT_SEAT: &str = "lookahead-v1";

/// `adjudicate`'s default K.
pub const DEFAULT_REPLICATIONS: u32 = 4;

/// How many ranked candidates `table` prints before it stops.
pub const RANKED_HEAD: usize = 10;

/// The name a `path:event` root's loader refusals are reported under. It has no
/// registry entry, so the refusal names the ARGUMENT FORM rather than an entry
/// nobody can look up.
pub const PATH_ROOT_NAME: &str = "path:event";

pub const USAGE: &str = "\
usage: driven <subcommand> [options]

  list
      Print the named-root REGISTRY: each entry's name, record path, event index,
      bot seat, recorded decision seed, whether the record file is PRESENT or
      MISSING right now, and what the root is for. An absent record is a MISS and
      never an error -- the session roots live in gitignored saves.

  table <root> [--contestant <spec>] [--decision-seed <u64>]
      Analyze ONE decision and apply nothing: the contestant echo, the sampled
      world (sampler seed, worlds, world digest), the winner, the ranking's shape
      (candidates / top value / tied-at-top / the <=1-point top band / distinct
      values), the ranked head, and ALL FIFTEEN SearchStats counters -- the
      mechanical separator's instrument, and the surface `why` cannot be (it
      prints 2 of the 15).

      --contestant <spec>    default `lookahead-v1`, the grammar's own shipped
                             default (beam 4, one world, 8 retreat samples,
                             UNCAPPED). The separator's wider beams are
                             `lookahead-v1:b=8` and `lookahead-v1:b=16`, and a
                             wider beam is EXPECTED to diverge from the recorded
                             move -- that divergence is the measurement.
      --decision-seed <u64>  For a REGISTRY root this DEFAULTS to the entry's own
                             recorded seed, and the report then prints a
                             REPRODUCED / DIVERGED line against the move the
                             record carries at that event: the b=4 reproduction
                             check, as a command. For a <path>:<event> root it is
                             REQUIRED -- no seed is recorded anywhere for such a
                             root, and inventing one would answer a different
                             question from the one the record asks.

  run <root> --master <u64> [--seats <list>]
      Play the root to terminal ONCE and report it: provenance (root descriptor,
      resolved seat echoes, observation labels, per-seat streams), the result and
      standings, decisions and turns, the per-seat counters, and the outcome
      digest. Nothing is recorded to disk and no GameRecord is emitted -- a rooted
      run is not a from-setup game.

  adjudicate <root> --master <u64> [--replications <K>] [--seats <list>]
      Force EVERY root candidate in turn, play the remainder out K times under the
      named seats, and report each candidate's margins, mean/min/max and wins,
      plus whether every candidate reads alike. THE FIRST LINE OF THE OUTPUT is
      the robustness-read label and it is not optional: this is a playout under a
      named policy and never exact ground truth about a candidate.

      --replications <K>     default 4. Replication r plays in
                             mix(SALT_DRIVEN_REPLICATION, [master, r]), with each
                             seat's stream taken off that world.
      COST: this is `candidates x K` whole games. The candidate count is the
      ENGINE's at that root -- 1 at a forced decision, a handful at a Building
      point, up to the hundreds at a money-saturated Movement point -- so the
      sizing choices are the ROOT and the reply seats, never a flag here. Rungs
      reply in milliseconds; two uncapped lookahead seats on a wide root is hours.

roots (<root> above):
  <fixture-name>   a name from `list`. Its record resolves against the WORKSPACE
                   ROOT, so one name means one file wherever the binary is
                   launched from.
  <path>:<event>   an explicit record path and the event index the mover decided
                   in -- state_at's `k`, and the number `why <n>` prints. Split at
                   the LAST colon; the index is decimal; a relative path resolves
                   against the working directory. It is read through the SAME
                   loader a registry entry takes, so a missing / unreadable /
                   non-record / STALE file refuses identically. Only the entry's
                   own transcription checks (recorded move, recorded seat) are
                   entry-only, which is why `table` prints NOT CHECKED in place of
                   a REPRODUCED / DIVERGED line for a path root.

seats (--seats <a,b[,c,d]>, one spec per seat in seat order, default
`lookahead-v1` on EVERY seat):
  the contestant grammar   Contestant::from_str_name's, where unknown is an error
                           at every level and an unknown knob is never silently
                           ignored:
      greedy-v1[:m=<n>]
      lookahead-v1[:b=<n>,k=<n>,m=<n>,cap=<n>]
      uniform | retreat-averse | family-weighted[:p=<ppm>,k=<n>]
      atomic-diag-v1[:d=<n>,k=<n>,m=<n>] | mcts-v1[:cap=,c=,scale=,m=,leaf=]
  the declared REPLY menu  the three bare names below. They resolve to the same
                           contestants the grammar spells and differ only in what
                           the provenance records they were ASKED FOR, which is
                           the half that stops meaning what it means at D1's
                           identity freeze:
      family-weighted | greedy-at-final | lookahead-at-final
  `family-weighted` ALONE is the reply-menu slot; `family-weighted:p=...` is the
  knobbed rung through the contestant grammar. Both resolve to a rung.

masters:
  --master has NO DEFAULT on `run` and `adjudicate`. A master is drawn from OS
  entropy and RECORDED BY THE CALLER BEFORE THE RUN (the standing policy), so an
  instrument that drew one here would put the draw after the decision to run, in
  a process whose output is the thing the draw is supposed to be independent of.
  Decimal, or hex with a 0x prefix; `_` separators are ignored.

Unknown subcommands, unknown flags, unknown seat names, unknown contestant knobs
and stray positionals are all ERRORS carrying this text -- never silent no-ops.
";

// ---------------------------------------------------------------------------
// The parsed command
// ---------------------------------------------------------------------------

/// WHICH root a subcommand was pointed at, before anything is loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootRef {
    /// A [`driven::registry`] entry, by name.
    Fixture(String),
    /// An explicit record file and the event index the mover decided in.
    Path { path: PathBuf, event_index: usize },
}

/// One parsed command line. Pure data — nothing here has read a file or played a
/// move, which is what lets the whole grammar be tested without a corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    List,
    Table {
        root: RootRef,
        contestant: Contestant,
        /// `None` means "take the registry entry's recorded seed", which is only
        /// available for a [`RootRef::Fixture`] — a path root with no seed is
        /// refused at execution with the sentence that says why.
        decision_seed: Option<u64>,
    },
    Run {
        root: RootRef,
        master: u64,
        /// `None` means "the default seat on every seat", which needs the root's
        /// player count and so cannot be resolved at parse time.
        seats: Option<Vec<SeatSpec>>,
    },
    Adjudicate {
        root: RootRef,
        master: u64,
        replications: u32,
        seats: Option<Vec<SeatSpec>>,
    },
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a command line (argv WITHOUT the program name).
pub fn parse(args: &[String]) -> Result<Command, String> {
    let _ = args;
    Err("driven_runner::parse is not implemented".to_string())
}

/// `<path>:<event>` or a bare registry name.
pub fn parse_root_ref(raw: &str) -> Result<RootRef, String> {
    let _ = raw;
    Err("driven_runner::parse_root_ref is not implemented".to_string())
}

/// ONE seat spec: the reply menu's three bare names, else the contestant
/// grammar.
pub fn parse_seat(spec: &str) -> Result<SeatSpec, String> {
    let _ = spec;
    Err("driven_runner::parse_seat is not implemented".to_string())
}

/// A comma-separated seat list, one spec per seat in seat order.
pub fn parse_seats(raw: &str) -> Result<Vec<SeatSpec>, String> {
    let _ = raw;
    Err("driven_runner::parse_seats is not implemented".to_string())
}

/// `eval_probe`'s spelling, so a master typed for one instrument means the same
/// thing typed for the other: decimal, or `0x` hex, with `_` separators ignored.
pub fn parse_u64(raw: &str) -> Result<u64, String> {
    let _ = raw;
    Err("driven_runner::parse_u64 is not implemented".to_string())
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// The fifteen [`SearchStats`] counters as `(name, value)`, in declaration
/// order.
pub fn counter_rows(stats: &SearchStats) -> [(&'static str, u64); 15] {
    let _ = stats;
    [("", 0); 15]
}

/// What a `table` report knows about its root BEYOND the table itself.
#[derive(Debug, Clone, Copy)]
pub struct TableContext<'a> {
    /// What the caller typed — a fixture name or a `path:event` spec.
    pub asked_for: &'a str,
    /// The resolved root's identity.
    pub root: &'a RootDescriptor,
    /// The move the registry entry records at this event, when there is one.
    pub recorded_move: Option<&'a Move>,
    /// The seed the registry entry records for this decision, when there is one.
    pub recorded_seed: Option<u64>,
}

/// The registry, with each record's presence resolved against `anchor`.
pub fn render_list(entries: &[RegistryEntry], anchor: &Path) -> String {
    let _ = (entries, anchor);
    String::new()
}

/// ONE decision, analyzed.
pub fn render_root_table(table: &RootTable, ctx: &TableContext<'_>) -> String {
    let _ = (table, ctx);
    String::new()
}

/// One rooted run.
pub fn render_outcome(outcome: &DrivenOutcome, asked_for: &str) -> String {
    let _ = (outcome, asked_for);
    String::new()
}

/// One adjudication. **Its FIRST LINE is [`driven::ROBUSTNESS_READ`] verbatim.**
pub fn render_adjudication(adjudication: &Adjudication, asked_for: &str) -> String {
    let _ = (adjudication, asked_for);
    String::new()
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// Parse `args`, do what they say, and return the report to print.
pub fn run(args: &[String]) -> Result<String, String> {
    run_with(args, &driven::registry(), &driven::workspace_root())
}

/// [`run`] against an EXPLICIT registry and anchor.
///
/// The test affordance `RegistryEntry::load_from` documents, one level up: every
/// shipped entry points at a gitignored save, so a cell that could only ask the
/// real registry could only ever exercise the MISS. A cell supplies its own
/// entries and a temp-dir anchor and owns every file under it.
///
/// `pub(crate)` deliberately — a runner resolving registry paths against
/// anything but [`driven::workspace_root`] would name a different file for the
/// same entry name depending on where it was launched.
pub(crate) fn run_with(
    args: &[String],
    entries: &[RegistryEntry],
    anchor: &Path,
) -> Result<String, String> {
    let _ = (args, entries, anchor);
    Err("driven_runner::run_with is not implemented".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::arena::{agent_stream_seed, run_arena_game, ArenaAgent, Slot};
    use crate::game::driven::{
        AdjudicationProvenance, AdjudicationSeat, CandidatePlayout, CandidateRow, DrivenOpts,
        RootOrigin, RootSource, SampledWorld, SESSION_BOT_SEAT,
    };
    use crate::game::legal_moves::get_legal_moves;
    use crate::game::lookahead::RootScore;
    use crate::game::policy::RolloutPolicy;
    use crate::game::record::{state_at, GameRecord, RecordedEvent};
    use crate::game::rules::Rules;
    use crate::game::seeds::{SeedSchedule, WorldSeed};
    use crate::game::semantics::SEMANTICS_REV;
    use crate::types::{GameState, PlayerId};

    /// This task's committed diagnostic master (Phase B, T5). The NAME of the
    /// constructor is the waiver, per `game::seeds`: a fixture cell wants
    /// reproduction, which is exactly the diagnostic case spec §3 carves out.
    const MASTER: u64 = 0x0B01_2026_0818_9995;

    /// The seed a cell-owned registry entry records. Arbitrary and fixed — a
    /// fixture record is not a CLI save, so no derivation produced it.
    const FIXTURE_SEED: u64 = 0x0DEC_1510_0000_0001;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    fn world(index: u64) -> WorldSeed {
        let mut schedule = SeedSchedule::diagnostic(MASTER);
        (0..=index).map(|_| schedule.next_world()).last().unwrap()
    }

    /// Removes the whole temp anchor when the cell leaves, pass or panic.
    struct TempRoot(PathBuf);

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl TempRoot {
        fn make(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let path = std::env::temp_dir()
                .join(format!("yokohama-driven-runner-{tag}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&path).expect("the temp anchor is creatable");
            Self(path)
        }
    }

    /// One 2P rung game's record — the cheapest honest record there is.
    fn rung_record(rules: &Rules, seed: WorldSeed) -> GameRecord {
        let mut make = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
            Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT)
                .build(rules, agent_stream_seed(seed, seat, Slot(seat.0)))
                .0
        };
        run_arena_game(seed, 2, rules, &mut make, true)
            .unwrap_or_else(|e| panic!("the fixture game must complete: {e}"))
            .record
    }

    fn entry(
        name: &'static str,
        path: &'static str,
        event_index: usize,
        bot_seat: PlayerId,
        recorded_move: Option<Move>,
    ) -> RegistryEntry {
        RegistryEntry {
            name,
            path,
            event_index,
            bot_seat,
            decision_seed: FIXTURE_SEED,
            recorded_move,
            note: "a cell-owned fixture",
        }
    }

    fn descriptor() -> RootDescriptor {
        RootDescriptor {
            origin: RootOrigin::RecordPrefix { record_digest: "record-digest-abc".to_string() },
            event_index: Some(113),
            root_digest: "root-digest-xyz".to_string(),
        }
    }

    fn table_with(winner: Move, ranked: &[(Move, i64)]) -> RootTable {
        RootTable {
            seat: PlayerId(1),
            contestant: Contestant::LOOKAHEAD_V1_DEFAULT,
            echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
            decision_seed: FIXTURE_SEED,
            winner,
            ranked: ranked
                .iter()
                .map(|(mv, value)| RootScore { mv: mv.clone(), value: *value })
                .collect(),
            stats: SearchStats {
                root_candidates: 3,
                nodes: 11,
                handles: 11,
                route_dfs_nodes: 2,
                forced_nodes: 5,
                leaves: 7,
                scored: 29,
                root_scored: 3,
                budget_binds: 0,
                budget_starved: 0,
                placement_cap_binds: 0,
                max_line: 4,
                ply_cap_hits: 0,
                free_extension_steps: 0,
                free_cap_hits: 0,
            },
            world: SampledWorld {
                sampler_seed: 0x0011_2233_4455_6677,
                worlds: 1,
                digest: "world-digest-777".to_string(),
            },
        }
    }

    fn adjudication_with(rows: Vec<CandidateRow>) -> Adjudication {
        Adjudication {
            mover: PlayerId(1),
            root_turn: 17,
            master: WorldSeed(0xABCD_0000_0000_0001),
            replications: 2,
            rows,
            witness: None,
            provenance: AdjudicationProvenance {
                root: descriptor(),
                semantics_rev: SEMANTICS_REV,
                rules_hash: "rules-hash-000".to_string(),
                seats: vec![
                    AdjudicationSeat {
                        seat: 0,
                        requested: "reply:family-weighted".to_string(),
                        echo: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT).echo(),
                    },
                    AdjudicationSeat {
                        seat: 1,
                        requested: "contestant".to_string(),
                        echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
                    },
                ],
                master: 0xABCD_0000_0000_0001,
                replications: 2,
            },
        }
    }

    fn row(candidate: Move, margins: &[i64]) -> CandidateRow {
        CandidateRow {
            candidate,
            continuation_digest: "continuation-digest".to_string(),
            playouts: margins
                .iter()
                .enumerate()
                .map(|(replication, margin)| CandidatePlayout {
                    replication: replication as u32,
                    world: WorldSeed(0x1000 + replication as u64),
                    agent_seeds: vec![1, 2],
                    margin: *margin,
                    mover_total: 100,
                    best_rival: PlayerId(0),
                    best_rival_total: (100 - margin) as u32,
                    rank: if *margin > 0 { 1 } else { 2 },
                    won: *margin > 0,
                    decisions: 42,
                    turns: 18,
                    final_digest: format!("final-{replication}"),
                })
                .collect(),
        }
    }

    // =======================================================================
    // USAGE and the grammar
    // =======================================================================

    /// USAGE names EVERY subcommand and EVERY flag the parser accepts — read off
    /// the parser's own whitelists, so a flag added without a line here fails
    /// rather than shipping undocumented.
    #[test]
    fn the_usage_names_every_subcommand_and_every_flag() {
        for subcommand in SUBCOMMANDS {
            assert!(USAGE.contains(subcommand), "USAGE does not name `{subcommand}`");
        }
        for flags in [LIST_FLAGS, TABLE_FLAGS, RUN_FLAGS, ADJUDICATE_FLAGS] {
            for flag in flags {
                assert!(USAGE.contains(&format!("--{flag}")), "USAGE does not name `--{flag}`");
            }
        }
        // The vocabularies a caller has to type, and the defaults it has to know.
        for token in [
            DEFAULT_SEAT,
            ReplyPolicy::FamilyWeighted.as_str(),
            ReplyPolicy::GreedyAtFinal.as_str(),
            ReplyPolicy::LookaheadAtFinal.as_str(),
            "<path>:<event>",
            "REPRODUCED",
            "NOT CHECKED",
        ] {
            assert!(USAGE.contains(token), "USAGE does not name `{token}`");
        }
        assert!(
            USAGE.contains(&DEFAULT_REPLICATIONS.to_string()),
            "USAGE does not state the default K",
        );
        assert!(USAGE.contains("NO DEFAULT"), "USAGE does not say --master has no default");
    }

    /// No subcommand, and an unknown one, are both errors carrying USAGE.
    #[test]
    fn an_absent_or_unknown_subcommand_is_an_error_carrying_the_usage() {
        let empty = parse(&[]).expect_err("no subcommand must refuse");
        assert!(empty.contains(USAGE), "the refusal drops USAGE: {empty}");

        let unknown = parse(&args(&["tabel"])).expect_err("an unknown subcommand must refuse");
        assert!(unknown.contains("tabel"), "the refusal hides what was typed: {unknown}");
        assert!(unknown.contains(USAGE), "the refusal drops USAGE: {unknown}");
    }

    /// An unknown flag refuses at EVERY subcommand — never a silent no-op.
    #[test]
    fn an_unknown_flag_refuses_at_every_subcommand() {
        let lines: [Vec<String>; 4] = [
            args(&["list", "--nope", "1"]),
            args(&["table", "g", "--nope", "1"]),
            args(&["run", "g", "--master", "1", "--nope", "1"]),
            args(&["adjudicate", "g", "--master", "1", "--nope", "1"]),
        ];
        for line in lines {
            let err = match parse(&line) {
                Err(e) => e,
                Ok(command) => panic!("{line:?} must refuse, got {command:?}"),
            };
            assert!(err.contains("nope"), "{line:?}: the refusal hides the flag: {err}");
            assert!(err.contains(USAGE), "{line:?}: the refusal drops USAGE: {err}");
        }
    }

    /// `--replications` is NOT a `run` flag: a single run has nothing to
    /// replicate, and accepting it would silently ignore it.
    #[test]
    fn replications_is_not_a_run_flag() {
        assert!(!RUN_FLAGS.contains(&"replications"), "the whitelist admits it");
        let err = parse(&args(&["run", "g", "--master", "1", "--replications", "4"]))
            .expect_err("--replications on run must refuse");
        assert!(err.contains("replications"), "the refusal hides the flag: {err}");
        // …and it IS an adjudicate flag, so the refusal above is about the
        // subcommand rather than about the spelling.
        assert!(ADJUDICATE_FLAGS.contains(&"replications"));
        parse(&args(&["adjudicate", "g", "--master", "1", "--replications", "4"]))
            .expect("--replications is an adjudicate flag");
    }

    /// A flag without its value refuses rather than consuming the next flag.
    #[test]
    fn a_flag_without_its_value_refuses() {
        let err = parse(&args(&["run", "g", "--master"]))
            .expect_err("a valueless --master must refuse");
        assert!(err.contains("master"), "the refusal hides the flag: {err}");
    }

    /// Positional arity: `list` takes none, the other three take exactly one.
    #[test]
    fn the_positional_arity_is_checked_per_subcommand() {
        // ANTI-VACUITY first: a parser that refused everything would satisfy
        // every refusal below.
        assert_eq!(parse(&args(&["list"])), Ok(Command::List), "the well-formed line must parse");
        assert!(parse(&args(&["list", "extra"])).is_err(), "list takes no positional");
        assert!(parse(&args(&["table"])).is_err(), "table needs a root");
        assert!(parse(&args(&["table", "a", "b"])).is_err(), "table takes ONE root");
        assert!(parse(&args(&["run", "--master", "1"])).is_err(), "run needs a root");
        assert!(parse(&args(&["adjudicate", "--master", "1"])).is_err(), "adjudicate needs a root");
    }

    /// `run` and `adjudicate` refuse without a master, and the sentence says WHY
    /// (recorded before use by the caller) rather than merely that it is missing.
    #[test]
    fn run_and_adjudicate_refuse_without_a_master() {
        for line in [args(&["run", "g"]), args(&["adjudicate", "g"])] {
            let err = match parse(&line) {
                Err(e) => e,
                Ok(command) => panic!("{line:?} must refuse, got {command:?}"),
            };
            assert!(err.contains("--master"), "{line:?}: {err}");
            assert!(
                err.to_lowercase().contains("record"),
                "{line:?}: the refusal does not say masters are recorded before use: {err}",
            );
        }
    }

    /// The root grammar: a bare name is a fixture, anything with a colon is
    /// `<path>:<event>` split at the LAST colon, and a non-numeric tail refuses.
    #[test]
    fn the_root_grammar_splits_a_path_at_the_last_colon() {
        assert_eq!(
            parse_root_ref("g1-e113-tiemass").expect("a bare name is a fixture"),
            RootRef::Fixture("g1-e113-tiemass".to_string()),
        );
        assert_eq!(
            parse_root_ref("/tmp/a:b/game.json:113").expect("a path:event parses"),
            RootRef::Path { path: PathBuf::from("/tmp/a:b/game.json"), event_index: 113 },
        );
        let err = parse_root_ref("game.json:notanumber").expect_err("a non-numeric tail refuses");
        assert!(err.contains("notanumber"), "the refusal hides the tail: {err}");
    }

    /// Seat specs: the reply menu's three bare names, else the contestant
    /// grammar, and unknown is an error.
    #[test]
    fn seat_specs_read_the_reply_menu_then_the_contestant_grammar() {
        assert_eq!(
            parse_seat("family-weighted").expect("the menu slot parses"),
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
        );
        assert_eq!(
            parse_seat("greedy-at-final").expect("the menu slot parses"),
            SeatSpec::Reply(ReplyPolicy::GreedyAtFinal),
        );
        assert_eq!(
            parse_seat("lookahead-at-final").expect("the menu slot parses"),
            SeatSpec::Reply(ReplyPolicy::LookaheadAtFinal),
        );
        // A KNOBBED rung is the contestant grammar, not the menu slot — and both
        // resolve to a rung, so only the recorded `requested` label differs.
        let knobbed = parse_seat("family-weighted:p=50000,k=5").expect("the knobbed rung parses");
        assert!(matches!(knobbed, SeatSpec::Contestant(Contestant::Policy(_))));
        assert_eq!(
            parse_seat("lookahead-v1:b=8").expect("the contestant grammar parses"),
            SeatSpec::Contestant(
                Contestant::from_str_name("lookahead-v1:b=8").expect("the grammar accepts it")
            ),
        );
        let err = parse_seat("nosuchseat").expect_err("an unknown seat name refuses");
        assert!(err.contains("nosuchseat"), "the refusal hides the name: {err}");

        assert_eq!(
            parse_seats("family-weighted,lookahead-v1").expect("a two-seat list parses").len(),
            2,
        );
        assert!(parse_seats("").is_err(), "an empty seat list is not a seat list");
        assert!(parse_seats("family-weighted,nope").is_err(), "one bad seat refuses the list");
    }

    /// The default seat and the default contestant are THE SHIPPED
    /// lookahead-v1, spelled through the grammar rather than through a second
    /// const this module would own.
    #[test]
    fn the_default_seat_is_the_shipped_lookahead_v1_through_the_grammar() {
        assert_eq!(
            Contestant::from_str_name(DEFAULT_SEAT).expect("the default spells a contestant"),
            Contestant::LOOKAHEAD_V1_DEFAULT,
        );
        match parse(&args(&["table", "g"])).expect("table parses without --contestant") {
            Command::Table { contestant, decision_seed, .. } => {
                assert_eq!(contestant, Contestant::LOOKAHEAD_V1_DEFAULT);
                assert_eq!(decision_seed, None, "the seed defaults at EXECUTION, not at parse");
            }
            other => panic!("expected a Table command, got {other:?}"),
        }
        match parse(&args(&["adjudicate", "g", "--master", "1"]))
            .expect("adjudicate parses without --replications")
        {
            Command::Adjudicate { replications, seats, .. } => {
                assert_eq!(replications, DEFAULT_REPLICATIONS);
                assert_eq!(seats, None, "the seat list defaults at EXECUTION (it needs a count)");
            }
            other => panic!("expected an Adjudicate command, got {other:?}"),
        }
    }

    /// Masters and seeds parse as `eval_probe` parses them.
    #[test]
    fn numbers_parse_as_hex_or_decimal_with_underscores() {
        assert_eq!(parse_u64("0x15e8_0120_dfcb_f8c3"), Ok(0x15e8_0120_dfcb_f8c3));
        assert_eq!(parse_u64("0X10"), Ok(16));
        assert_eq!(parse_u64("1_000"), Ok(1000));
        assert!(parse_u64("zzz").is_err());
    }

    // =======================================================================
    // Rendering
    // =======================================================================

    /// The counter block names ALL FIFTEEN `SearchStats` fields.
    ///
    /// The names come off [`counter_rows`], whose body destructures
    /// `SearchStats` exhaustively — so a sixteenth counter is a COMPILE error
    /// there rather than a silently-unprinted field here.
    #[test]
    fn the_table_render_names_all_fifteen_search_counters() {
        let table = table_with(
            Move::BuildShophouse { slot: 0 },
            &[(Move::BuildShophouse { slot: 0 }, 116), (Move::BuildShophouse { slot: 1 }, 115)],
        );
        let root = descriptor();
        let rendered = render_root_table(
            &table,
            &TableContext {
                asked_for: "g1-e113-tiemass",
                root: &root,
                recorded_move: None,
                recorded_seed: None,
            },
        );
        let rows = counter_rows(&table.stats);
        assert_eq!(rows.len(), 15, "the counter set is not fifteen");
        for (name, value) in rows {
            assert!(!name.is_empty(), "an unnamed counter row");
            assert!(rendered.contains(name), "the render omits counter `{name}`: {rendered}");
            let _ = value;
        }
        // The values, not merely the names: a block of fifteen zeros would pass
        // a names-only check on any stats at all.
        assert!(rendered.contains("29"), "the render omits `scored`'s value: {rendered}");
    }

    /// The shape fields the separator reads, and the winner.
    #[test]
    fn the_table_render_carries_the_ranking_shape_and_the_world() {
        let table = table_with(
            Move::BuildShophouse { slot: 0 },
            &[
                (Move::BuildShophouse { slot: 0 }, 116),
                (Move::BuildShophouse { slot: 1 }, 116),
                (Move::BuildShophouse { slot: 2 }, 115),
                (Move::BuildShophouse { slot: 3 }, 12),
            ],
        );
        let root = descriptor();
        let rendered = render_root_table(
            &table,
            &TableContext {
                asked_for: "fixture",
                root: &root,
                recorded_move: None,
                recorded_seed: None,
            },
        );
        for token in [
            "fixture",
            &root.root_digest,
            &table.echo,
            &format!("{:#018x}", table.decision_seed),
            &format!("{:#018x}", table.world.sampler_seed),
            &table.world.digest,
        ] {
            assert!(rendered.contains(token), "the render omits `{token}`: {rendered}");
        }
        assert!(rendered.contains("113"), "the render omits the event index: {rendered}");
        // candidates 4, tied-at-top 2, top band (<=1 point) 3, distinct 3.
        assert_eq!((table.candidates(), table.tied_at_top()), (4, 2));
        assert_eq!((table.top_band(), table.distinct_values()), (3, 3));
        for token in ["4", "2", "3"] {
            assert!(rendered.contains(token), "the render omits shape figure {token}: {rendered}");
        }
    }

    /// The reproduction line: REPRODUCED when the winner is the recorded move,
    /// DIVERGED when it is not, NOT CHECKED when there is nothing to compare —
    /// plus the OVERRIDDEN qualifier when the seed is not the entry's own.
    #[test]
    fn the_reproduction_line_reads_reproduced_diverged_or_not_checked() {
        let winner = Move::BuildShophouse { slot: 0 };
        let other = Move::BuildShophouse { slot: 7 };
        let table = table_with(winner.clone(), &[(winner.clone(), 5), (other.clone(), 4)]);
        let root = descriptor();
        let ctx = |recorded_move: Option<&Move>, recorded_seed: Option<u64>| {
            render_root_table(
                &table,
                &TableContext {
                    asked_for: "fixture",
                    root: &root,
                    recorded_move,
                    recorded_seed,
                },
            )
        };

        let reproduced = ctx(Some(&winner), Some(FIXTURE_SEED));
        assert!(reproduced.contains("REPRODUCED"), "no reproduced verdict: {reproduced}");
        assert!(!reproduced.contains("DIVERGED"), "both verdicts printed: {reproduced}");
        assert!(!reproduced.contains("OVERRIDDEN"), "a matching seat is not an override");

        let diverged = ctx(Some(&other), Some(FIXTURE_SEED));
        assert!(diverged.contains("DIVERGED"), "no diverged verdict: {diverged}");
        assert!(!diverged.contains("REPRODUCED"), "both verdicts printed: {diverged}");

        let unchecked = ctx(None, None);
        assert!(unchecked.contains("NOT CHECKED"), "no not-checked verdict: {unchecked}");
        assert!(!unchecked.contains("REPRODUCED"), "a path root claims a reproduction");

        // An OVERRIDDEN seed is still compared, and the qualifier says the
        // comparison is no longer the entry's own b=4 reproduction check.
        let overridden = ctx(Some(&winner), Some(FIXTURE_SEED ^ 0xFF));
        assert!(overridden.contains("OVERRIDDEN"), "no override qualifier: {overridden}");
    }

    /// Rendering is a function of its inputs — the property that lets a report
    /// be quoted.
    #[test]
    fn rendering_is_deterministic_at_fixed_inputs() {
        let table = table_with(
            Move::BuildShophouse { slot: 0 },
            &[(Move::BuildShophouse { slot: 0 }, 3), (Move::BuildShophouse { slot: 1 }, 1)],
        );
        let root = descriptor();
        let ctx = TableContext {
            asked_for: "fixture",
            root: &root,
            recorded_move: None,
            recorded_seed: None,
        };
        let once = render_root_table(&table, &ctx);
        // ANTI-VACUITY: two empty strings are equal, so an unimplemented
        // renderer would satisfy the identity below without rendering anything.
        assert!(once.lines().count() > 5, "the table render is a stub: {once:?}");
        assert_eq!(once, render_root_table(&table, &ctx));

        let adjudication = adjudication_with(vec![
            row(Move::BuildShophouse { slot: 0 }, &[3, 3]),
            row(Move::BuildShophouse { slot: 1 }, &[1, 5]),
        ]);
        let once = render_adjudication(&adjudication, "fixture");
        assert!(once.lines().count() > 5, "the adjudication render is a stub: {once:?}");
        assert_eq!(once, render_adjudication(&adjudication, "fixture"));
    }

    /// The adjudication report OPENS with the robustness-read label, verbatim
    /// (T4 NOTE-2: the label rides stdout, not only the docs).
    #[test]
    fn the_adjudication_render_opens_with_the_robustness_read_verbatim() {
        let adjudication = adjudication_with(vec![row(Move::BuildShophouse { slot: 0 }, &[3, 3])]);
        let rendered = render_adjudication(&adjudication, "fixture");
        assert_eq!(
            rendered.lines().next(),
            Some(driven::ROBUSTNESS_READ),
            "the first line is not the robustness read",
        );
        assert!(
            rendered.contains(driven::ROBUSTNESS_READ),
            "the label is not present verbatim: {rendered}",
        );
    }

    /// One row per candidate, with margins, mean/min/max, wins — and the
    /// all-candidates-equal fact stated rather than left to the reader.
    #[test]
    fn the_adjudication_render_states_every_candidate_and_the_equality_fact() {
        let equal = adjudication_with(vec![
            row(Move::BuildShophouse { slot: 0 }, &[3, 3]),
            row(Move::BuildShophouse { slot: 1 }, &[3, 3]),
        ]);
        assert!(equal.all_candidates_equal());
        let rendered = render_adjudication(&equal, "fixture");
        assert!(rendered.contains("all candidates equal"), "no equality line: {rendered}");
        assert!(rendered.contains("true"), "the equality fact is not stated: {rendered}");
        assert!(rendered.contains("BuildShophouse"), "no candidate rows: {rendered}");
        assert!(rendered.contains(&equal.digest()), "no reproduction digest: {rendered}");
        assert!(rendered.contains("p1"), "the mover is not named: {rendered}");

        let unequal = adjudication_with(vec![
            row(Move::BuildShophouse { slot: 0 }, &[3, 3]),
            row(Move::BuildShophouse { slot: 1 }, &[1, 5]),
        ]);
        assert!(!unequal.all_candidates_equal());
        let rendered = render_adjudication(&unequal, "fixture");
        assert!(rendered.contains("false"), "the equality fact is not stated: {rendered}");
        // mean of [1, 5] is 3.00 while the margins differ — the elementwise
        // reading is the one that matters, and both must be visible.
        assert!(rendered.contains("+5"), "the max margin is not printed: {rendered}");
    }

    /// The `list` report marks each record PRESENT or MISSING against the anchor
    /// it was given, and NEVER errors on absence.
    #[test]
    fn the_list_report_marks_presence_and_never_errors_on_absence() {
        let anchor = TempRoot::make("list");
        std::fs::create_dir_all(anchor.0.join("here")).expect("the subdir is creatable");
        std::fs::write(anchor.0.join("here/present.json"), "{}").expect("the file is writable");

        let entries = [
            entry("present-one", "here/present.json", 3, PlayerId(1), None),
            entry("absent-one", "here/absent.json", 4, PlayerId(0), None),
        ];
        let rendered = render_list(&entries, &anchor.0);
        assert!(rendered.contains("present-one"), "the entry is missing: {rendered}");
        assert!(rendered.contains("absent-one"), "the entry is missing: {rendered}");
        assert!(rendered.contains("PRESENT"), "no presence marker: {rendered}");
        assert!(rendered.contains("MISSING"), "no absence marker: {rendered}");
        assert!(rendered.contains("a cell-owned fixture"), "the note is dropped: {rendered}");
        assert!(rendered.contains("here/present.json"), "the path is dropped: {rendered}");
        assert!(
            rendered.contains(&format!("{FIXTURE_SEED:#018x}")),
            "the recorded seed is dropped: {rendered}",
        );

        // Through the command, with an anchor that holds NOTHING: still Ok.
        let empty = TempRoot::make("list-empty");
        let out = run_with(&args(&["list"]), &entries, &empty.0).expect("list never errors");
        assert!(out.contains("MISSING"), "an empty anchor must read MISSING: {out}");
    }

    /// The SHIPPED registry renders — every entry named, nothing panicking, and
    /// no dependence on whether this checkout carries the gitignored saves.
    #[test]
    fn the_shipped_registry_lists_every_entry() {
        let rendered = run(&args(&["list"])).expect("list never errors");
        for entry in driven::registry() {
            assert!(rendered.contains(entry.name), "the list omits {}: {rendered}", entry.name);
            assert!(rendered.contains(entry.path), "the list omits {}'s path", entry.name);
        }
    }

    // =======================================================================
    // Execution, on cell-owned records
    // =======================================================================

    /// Walk `record`'s positions and return the first `(event_index, state)`
    /// whose decision satisfies `wanted`.
    fn find_position(
        record: &GameRecord,
        rules: &Rules,
        mut wanted: impl FnMut(&GameState, &Move) -> bool,
    ) -> Option<(usize, GameState)> {
        let mut state = state_at(record, 0, rules).expect("position 0 replays");
        for (position, event) in record.events.iter().enumerate() {
            if let RecordedEvent::Move(mv) = event {
                if wanted(&state, mv) {
                    return Some((position, state));
                }
            }
            state = crate::game::record::replay_from(&state, std::slice::from_ref(event), rules)
                .expect("the fixture record replays");
        }
        None
    }

    /// A `path:event` root REFUSES without `--decision-seed`, and the sentence
    /// says why (no seed is recorded for such a root).
    #[test]
    fn a_path_root_requires_a_decision_seed() {
        let rules = Rules::standard();
        let anchor = TempRoot::make("path-seed");
        let record = rung_record(&rules, world(1));
        let file = anchor.0.join("fixture.json");
        std::fs::write(&file, serde_json::to_string(&record).expect("the record serializes"))
            .expect("the fixture is writable");

        let spec = format!("{}:3", file.display());
        let err = run_with(&args(&["table", &spec]), &[], &anchor.0)
            .expect_err("a path root without a seed must refuse");
        assert!(err.contains("--decision-seed"), "the refusal hides the flag: {err}");
        assert!(
            err.to_lowercase().contains("recorded"),
            "the refusal does not say no seed is recorded: {err}",
        );
    }

    /// The whole `table` path on a CELL-OWNED record: the registry default seed
    /// is the entry's own, and the reproduction line reads REPRODUCED at a
    /// FORCED decision (where every honest contestant plays the one legal move)
    /// and DIVERGED where the rung and the search disagree.
    #[test]
    fn the_table_command_reproduces_a_forced_decision_and_reports_a_divergence() {
        let rules = Rules::standard();
        let anchor = TempRoot::make("table");
        let record = rung_record(&rules, world(2));
        std::fs::write(
            anchor.0.join("fixture.json"),
            serde_json::to_string(&record).expect("the record serializes"),
        )
        .expect("the fixture is writable");

        // A FORCED decision: one legal move, so the recorded rung move and the
        // search's pick are the same move by construction.
        let (forced_index, forced_state) =
            find_position(&record, &rules, |state, mv| {
                let legal = get_legal_moves(state, &rules);
                legal.len() == 1 && legal[0] == *mv
            })
            .expect("a 2P game has a forced decision");
        let forced_move = match &record.events[forced_index] {
            RecordedEvent::Move(mv) => mv.clone(),
            other => panic!("event {forced_index} is {other:?}"),
        };
        let entries = [entry(
            "forced",
            "fixture.json",
            forced_index,
            forced_state.current_player,
            Some(forced_move),
        )];
        let out = run_with(&args(&["table", "forced"]), &entries, &anchor.0)
            .expect("the forced decision tabulates");
        assert!(out.contains("REPRODUCED"), "a forced decision must reproduce: {out}");
        assert!(
            out.contains(&format!("{FIXTURE_SEED:#018x}")),
            "the entry's recorded seed is not the default: {out}",
        );
        assert!(!out.contains("OVERRIDDEN"), "the default seed is not an override: {out}");

        // An explicit seed that is NOT the entry's is still compared, under the
        // override qualifier.
        let out = run_with(
            &args(&["table", "forced", "--decision-seed", "0x1"]),
            &entries,
            &anchor.0,
        )
        .expect("an explicit seed tabulates");
        assert!(out.contains("OVERRIDDEN"), "an explicit seed is an override: {out}");

        // A BRANCHING decision where the rung's recorded move is not the
        // search's argmax. Bounded so a pathological record cannot hang the
        // suite; a scan that finds none is a finding, not a pass.
        let mut tried = 0;
        let mut diverged = None;
        let mut state = state_at(&record, 0, &rules).expect("position 0 replays");
        for (position, event) in record.events.iter().enumerate() {
            if let RecordedEvent::Move(mv) = event {
                if tried < 12 && get_legal_moves(&state, &rules).len() > 1 {
                    tried += 1;
                    let entries = [entry(
                        "branching",
                        "fixture.json",
                        position,
                        state.current_player,
                        Some(mv.clone()),
                    )];
                    let out = run_with(&args(&["table", "branching"]), &entries, &anchor.0)
                        .expect("a branching decision tabulates");
                    if out.contains("DIVERGED") {
                        diverged = Some(out);
                        break;
                    }
                }
            }
            state = crate::game::record::replay_from(&state, std::slice::from_ref(event), &rules)
                .expect("the fixture record replays");
        }
        let out = diverged.expect("no rung move differed from the search's pick in 12 tries");
        assert!(!out.contains("REPRODUCED"), "both verdicts printed: {out}");
    }

    /// An unknown fixture name refuses and NAMES the entries that exist.
    #[test]
    fn an_unknown_fixture_name_refuses_naming_what_exists() {
        let anchor = TempRoot::make("unknown");
        let entries = [entry("known-one", "nothing.json", 1, PlayerId(0), None)];
        let err = run_with(&args(&["table", "nope"]), &entries, &anchor.0)
            .expect_err("an unknown fixture must refuse");
        assert!(err.contains("nope"), "the refusal hides what was asked for: {err}");
        assert!(err.contains("known-one"), "the refusal does not name what exists: {err}");
    }

    /// A `table` on an entry whose recorded seat is NOT the mover refuses: the
    /// decision analyzed is the mover's, and an entry claiming otherwise is
    /// pointed at the wrong event.
    #[test]
    fn a_table_refuses_an_entry_whose_recorded_seat_is_not_the_mover() {
        let rules = Rules::standard();
        let anchor = TempRoot::make("seat");
        let record = rung_record(&rules, world(3));
        std::fs::write(
            anchor.0.join("fixture.json"),
            serde_json::to_string(&record).expect("the record serializes"),
        )
        .expect("the fixture is writable");
        let (index, state) =
            find_position(&record, &rules, |_, _| true).expect("the record carries a move");
        let wrong = PlayerId(1 - state.current_player.0);
        let entries = [entry("wrong-seat", "fixture.json", index, wrong, None)];
        let err = run_with(&args(&["table", "wrong-seat"]), &entries, &anchor.0)
            .expect_err("a mis-seated entry must refuse");
        assert!(err.contains("wrong-seat"), "the refusal hides the entry: {err}");
    }

    /// A MISSING record is a typed miss the runner reports rather than a panic.
    #[test]
    fn a_missing_record_is_reported_not_panicked() {
        let anchor = TempRoot::make("missing");
        let entries = [entry("absent", "nothing.json", 1, PlayerId(0), None)];
        let err = run_with(&args(&["table", "absent", "--decision-seed", "1"]), &entries, &anchor.0)
            .expect_err("an absent record must refuse");
        assert!(err.contains("nothing.json"), "the refusal hides the path: {err}");
    }

    /// The `run` report carries provenance, seats, the result and the digest.
    ///
    /// Rung seats deliberately (design §6's wall discipline): the property under
    /// test is the REPORT, and a debug lookahead game costs seconds.
    #[test]
    fn the_run_report_carries_provenance_seats_result_and_digest() {
        let rules = Rules::standard();
        let anchor = TempRoot::make("run");
        let record = rung_record(&rules, world(4));
        std::fs::write(
            anchor.0.join("fixture.json"),
            serde_json::to_string(&record).expect("the record serializes"),
        )
        .expect("the fixture is writable");
        // Near the END of the game, so the remainder is short.
        let index = record.events.len() - 3;
        let entries = [entry("tail", "fixture.json", index, SESSION_BOT_SEAT, None)];

        let out = run_with(
            &args(&[
                "run",
                "tail",
                "--master",
                "0x0B01_2026_0818_0001",
                "--seats",
                "family-weighted,family-weighted",
            ]),
            &entries,
            &anchor.0,
        )
        .expect("the tail run completes");

        for token in ["record-prefix", "winner", "decisions", "driven-v1", "family-weighted"] {
            assert!(out.contains(token), "the run report omits `{token}`: {out}");
        }
        assert!(out.contains(&format!("rev={SEMANTICS_REV}")), "no semantics stamp: {out}");
        assert!(out.contains(&rules.content_hash()), "no rules stamp: {out}");
        assert!(out.contains(&format!("{:#018x}", 0x0B01_2026_0818_0001_u64)), "no master: {out}");

        // The same command twice is the same report — determinism through the
        // whole execution path, not only through the renderer.
        let again = run_with(
            &args(&[
                "run",
                "tail",
                "--master",
                "0x0B01_2026_0818_0001",
                "--seats",
                "family-weighted,family-weighted",
            ]),
            &entries,
            &anchor.0,
        )
        .expect("the tail run completes");
        assert_eq!(out, again, "two identical run commands produced different reports");
    }

    /// The `adjudicate` report on a real near-terminal root: the label first,
    /// one row per candidate, K replications.
    #[test]
    fn the_adjudicate_command_reports_every_candidate_under_the_label() {
        let rules = Rules::standard();
        let anchor = TempRoot::make("adjudicate");
        let record = rung_record(&rules, world(5));
        std::fs::write(
            anchor.0.join("fixture.json"),
            serde_json::to_string(&record).expect("the record serializes"),
        )
        .expect("the fixture is writable");
        let index = record.events.len() - 2;
        let entries = [entry("tail", "fixture.json", index, SESSION_BOT_SEAT, None)];

        let out = run_with(
            &args(&[
                "adjudicate",
                "tail",
                "--master",
                "0x0B01_2026_0818_0002",
                "--replications",
                "2",
                "--seats",
                "family-weighted,family-weighted",
            ]),
            &entries,
            &anchor.0,
        )
        .expect("the tail adjudication completes");

        assert_eq!(
            out.lines().next(),
            Some(driven::ROBUSTNESS_READ),
            "the first line is not the robustness read",
        );
        assert!(out.contains("all candidates equal"), "no equality line: {out}");
        assert!(out.contains("adjudication-v1"), "no reproduction digest: {out}");
        assert!(out.contains("replications=2"), "K is not echoed: {out}");
    }

    /// The DEFAULTS are the shipped seats, and a driven run through them is a
    /// real run — pinned at a root ONE decision from the end, so the default
    /// (uncapped lookahead on every seat) is affordable in a debug cell.
    #[test]
    fn the_default_seats_drive_a_real_run() {
        let rules = Rules::standard();
        let anchor = TempRoot::make("default-seats");
        let record = rung_record(&rules, world(6));
        std::fs::write(
            anchor.0.join("fixture.json"),
            serde_json::to_string(&record).expect("the record serializes"),
        )
        .expect("the fixture is writable");
        let index = record.events.len() - 1;
        let entries = [entry("last", "fixture.json", index, SESSION_BOT_SEAT, None)];

        let out = run_with(
            &args(&["run", "last", "--master", "0x0B01_2026_0818_0003"]),
            &entries,
            &anchor.0,
        )
        .expect("the last-decision run completes");
        assert!(
            out.contains(&Contestant::LOOKAHEAD_V1_DEFAULT.echo()),
            "the default seat is not the shipped lookahead-v1: {out}",
        );
    }

    /// A CONSTRUCTED source drives too — the runner is not the only entry point
    /// and the module's own surface stays usable without argv.
    #[test]
    fn the_library_surface_stays_usable_without_argv() {
        let rules = Rules::standard();
        let record = rung_record(&rules, world(7));
        let source = RootSource::RecordPrefix { record, event_index: 2 };
        let (state, _) = source.resolve(&rules).expect("the prefix resolves");
        let seats = vec![SeatSpec::Reply(ReplyPolicy::FamilyWeighted); state.players.len()];
        let outcome = driven::drive_rooted(
            &source,
            &seats,
            &rules,
            world(8),
            DrivenOpts::default(),
        )
        .expect("the rooted run completes");
        let rendered = render_outcome(&outcome, "constructed");
        assert!(rendered.contains(&outcome.digest()), "the render drops the digest: {rendered}");
    }
}
