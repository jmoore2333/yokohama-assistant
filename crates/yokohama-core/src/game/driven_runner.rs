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

use crate::game::arena::{agent_stream_seed, ArenaAgent, Contestant, Slot};
use crate::game::driven::{
    self, Adjudication, DrivenOpts, DrivenOutcome, GeneratingEcology, RegistryEntry, ReplyPolicy,
    RootDescriptor, RootOrigin, RootPredicate, RootSource, RootTable, SeatSpec,
};
use crate::game::driven_exact::{self, ExactTable, WorldInvariance};
use crate::game::driven_families::{
    families, run_family_block_with, AdmissionVerdict, BlockKind, BlockOpts, ExclusionReason,
    FamilyBlockReport, FamilyDecl, FamilyGenerator, FamilyLabel, FamilyRunError, FamilyVerdict,
    PassRule, RootRow, RootVerdict, TurnPredicate, DECLARED_BY_THE_ROOT,
    FAMILY2_CONTROL_GENERATOR, FROZEN_EPSILON_VP, PROXIMITY_CORNER_GENERATOR,
    RACE_CONTROL_GENERATOR,
};
use crate::game::lookahead::SearchStats;
use crate::game::metrics::wilson_95;
use crate::game::rules::Rules;
use crate::game::seeds::{SeedSchedule, WorldSeed};
use crate::game::semantics::state_digest;
use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
use crate::types::{AreaAction, AreaId, GameConfig, Move, PlayerId};

/// Every subcommand this runner answers. The parser reads it and
/// [`USAGE`]'s completeness pin asserts every member is documented.
pub const SUBCOMMANDS: &[&str] =
    &["list", "table", "run", "adjudicate", "exact", "family", "opening", "capacity"];

/// `list`'s flags: none.
pub const LIST_FLAGS: &[&str] = &[];

/// The BURN-RULE acknowledgement, on every subcommand that READS a root.
///
/// **A valued flag rather than a bare switch, and the value is the ordinal being
/// burned.** [`split_args`] has no bare-flag arm by design, and the design is
/// right here for a second reason: a switch copied off somebody else's command
/// line unlocks whatever it is pasted onto, while a flag that must NAME the
/// ordinal can only unlock the read its typist meant. See [`deny_escrow_burn`].
pub const ESCROW_READ_FLAG: &str = "escrow-read";

/// `table`'s flags.
pub const TABLE_FLAGS: &[&str] = &["contestant", "decision-seed", ESCROW_READ_FLAG];

/// `exact`'s flags.
///
/// **`master` is deliberately ABSENT and its absence is enforced with a REASON**
/// (see the parse arm): the exact solve draws no worlds, so accepting a master
/// would advertise a dependence it does not have.
pub const EXACT_FLAGS: &[&str] =
    &["max-nodes", "max-depth", "max-memo", "eps", "rider", ESCROW_READ_FLAG];

/// `capacity`'s flags.
pub const CAPACITY_FLAGS: &[&str] = &["master", "stop-after"];

/// `family`'s flags.
pub const FAMILY_FLAGS: &[&str] = &["block", "master", "arm"];

/// `opening`'s flags.
pub const OPENING_FLAGS: &[&str] = &["master", "games", "contestant"];

/// The GENERATED root form's prefix — `gen:<predicate>:<master>:<ordinal>`.
///
/// A prefix rather than a fourth colon-splitting rule, because
/// [`parse_root_ref`] splits a `<path>:<event>` root at its LAST colon and a
/// generated spec has three: the prefix is read FIRST, so the two grammars
/// cannot shadow one another whatever a path happens to contain.
pub const GEN_PREFIX: &str = "gen:";

/// The blocks `family --block` accepts, in declaration order.
///
/// **THERE ARE TWO AND THERE IS NO THIRD.** Escrow ordinals live in the FREEZE
/// DOCUMENT and deliberately not in code (design §2's escrow hygiene), so no
/// command can burn one by accident — `--block escrow` is exactly as unknown as
/// any other typo, and this array is what makes that a property rather than an
/// omission. The `gen:` escape hatch technically REACHES an escrow ordinal;
/// that protection is procedural (the runs report's commands-as-record), and
/// saying so out loud is the design's own instruction.
pub const BLOCK_NAMES: [&str; 2] = ["pilot", "confirmatory"];

/// The arms `family --arm` accepts, in declaration order.
pub const ARM_NAMES: [&str; 2] = ["positive", "control"];

/// `run`'s flags. **`replications` is deliberately absent** — a single rooted
/// run has nothing to replicate, and accepting the flag would silently ignore
/// it.
pub const RUN_FLAGS: &[&str] = &["master", "seats", ESCROW_READ_FLAG];

/// `adjudicate`'s flags.
pub const ADJUDICATE_FLAGS: &[&str] =
    &["master", "seats", "replications", ESCROW_READ_FLAG];

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

/// The `opening` probe's SETUP PATH, printed in its own output (design NOTE-4 —
/// the ecology discipline: a population is only comparable to another when the
/// path that produced it is on the page).
///
/// **It is the PUBLIC path and not the arena's** (plan review SHOULD-5): the
/// Phase-B split seamed out only `play_from_state`, so the arena's setup half
/// is still interior to its private driver and a probe that wanted to reuse it
/// would have to widen a seam for one caller. What makes the reuse unnecessary
/// is that the two are pinned BIT IDENTICAL — see this module's
/// `the_opening_setup_path_is_bit_identical_to_the_arena_setup_half`.
pub const OPENING_SETUP_PATH: &str =
    "new_game(GameConfig::for_player_count(2), rules, world) -> each seat's \
     ArenaAgent::choose_starting_order in SEAT ORDER -> finalize_setup (the PUBLIC setup path; \
     the arena's own setup half is private to its driver, and the two are pinned bit-identical \
     against state_at(record, 0) at the same master and the same streams)";

/// What the opening probe prints when a first turn executed no area action.
///
/// A named constant because a cell asserts against it: an opening turn that
/// reached no area action is a real (if surprising) reading, and it must not be
/// spelled two ways.
pub const OPENING_NO_ACTION: &str = "(no area action)";

/// The area action whose share the opening probe reports as the LAB RUSH — the
/// C-D question's own quantity.
pub const LAB_RUSH_KIND: &str = "Laboratory";

/// The BRAND a control render carries when the report it is rendering ALSO
/// inverted admission (Phase C, T6 fix wave, review MUST-1).
///
/// **A tripwire on the page, not an assertion in a test** — deliberately, and
/// this is the second half of [`family_block_opts`]'s answer. A block run is
/// minutes to hours and lands in a T7 triage note, so the failure that matters
/// is not a red suite but a table somebody QUOTES. `report.control` is the
/// sizing the run actually used, `arm` is what the caller asked for, and the
/// pair `(control, control=true)` is the double negation — the control's own
/// claim upside down. Any edit that reaches `BlockOpts::control` around the
/// seam therefore announces itself in its own output, in the arm block, above
/// every number it would otherwise quietly invert.
pub const DOUBLE_NEGATION_BRAND: &str =
    "!! DOUBLE NEGATION — this CONTROL arm ran with admission-inverted=true, so the block asked \
     whether the control's own pass behaviour LOSES by delta: the control's claim upside down. \
     The arithmetic below is NOT this arm's result. Re-run with the inversion off (the sizing \
     seam `family_block_opts` is the one place it is decided).";

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
                             ONLY `lookahead-v1` and its knob variants have a
                             root table at all. Every other arm is a TYPED
                             REFUSAL naming why, never an empty table: a rung
                             SAMPLES its move and values no candidate,
                             `greedy-v1` is a one-ply argmax that reports no
                             per-candidate vector and no counters, and `mcts-v1`
                             / `atomic-diag-v1` are labelled diagnostics no v1
                             consumer asks a root table about.
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

  exact <root> [--max-nodes <n>] [--max-depth <n>] [--max-memo <n>] [--eps <vp>]
      Solve the remaining game EXACTLY at an IN-FINAL-ROUND root and report every
      candidate's exact value, the optimal set at eps=0, the top set at the
      declared eps, and the gap for every other row. THE FIRST LINE OF THE OUTPUT
      is the clairvoyance label and it is not optional: this is a
      PERFECT-INFORMATION solve over the TRUE state with BOTH seats played
      optimally, so a value is an UPPER BOUND on achievable quality and a pick
      outside the optimal set is NOT by itself misplay.

      NO --master, and the absence is the point: the solve draws no worlds and
      derives no seed, so its answer is a pure function of (root, rules, bounds).
      Supplying one is an error carrying that sentence rather than a bare
      `unknown flag`.

      SCOPE is enforced structurally, before anything is expanded. A MIDGAME root
      (the end game not triggered) has an unbounded remainder and refuses by
      name. A TRIGGER-ROUND root (triggered, final round not begun) carries a
      whole further round and refuses as its OWN named class -- a different fact,
      reported apart. 3-4P refuses too: best-rival-by-rank is not antisymmetric
      there, so the margin is not zero-sum and minimax is unsound.

      --max-nodes <n>        expanded decision nodes. A breach refuses the WHOLE
                             table -- never a partial one, on the adjudicator's
                             grounds -- and carries the COUNTED total as an
                             honest lower bound on the root's tree.
      --max-depth <n>        decisions from the root.
      --max-memo <n>         transposition entries; 0 DISABLES memoization. The
                             table stores TURN-ENTRY states only, a measured
                             choice: storing every node cut expanded nodes 3.2x
                             but cost 4.1x wall.
      --eps <vp>             default 2, family 1's FROZEN parameter, so the exact
                             verdict is comparable with the frozen instrument's
                             on the tolerance the family actually ran at. On an
                             exact read eps is a declared INDIFFERENCE BAND and
                             not noise cover -- there is no sampling noise here.
      --rider <master>       run the W = 5 WORLD-INVARIANCE read after the solve
                             and print its verdict. THE ONE PLACE `exact` takes a
                             master, and the carve-out is exactly the point: the
                             SOLVE draws no worlds, the RIDER does -- it re-solves
                             the root in five determinizations of the MOVER's own
                             view, drawn through the honest chain's own sampler.
                             W is pre-registered at 5 and is not adjustable; a
                             root that cannot afford five worlds refuses.
                             INVARIANT means the optimal set is the same in the
                             true world and in every world the mover could not
                             tell apart from it, so a pick outside the set is
                             UNAMBIGUOUS MISPLAY. CONFOUNDED means the optimal
                             set MOVES with the world, so the exact verdict still
                             resolves the policy disagreement but licenses NO
                             misplay claim. VACUOUS means every determinization
                             WAS the true position -- the view left nothing
                             material unseen, so no alternative world existed to
                             disagree and the read licenses NO attribution
                             either way. Record the master BEFORE the run.
      COST is the ROOT's, not a knob's: this is an exhaustive solve of two turns
      of play. Minutes at a small root; hours at a wide one; some roots refuse.
      The rider multiplies that by SIX -- the truth solve plus five worlds.
      (The truth table is solved once and reused, so it is six and not seven.)

  family <id> --block pilot --master <u64> [--arm positive|control]
  family <id> --block confirmatory [--arm positive|control]
      Run ONE declared block of ONE frozen family and print its report: the
      label FIRST, D7's step-one counter check SECOND, then the per-policy
      admission table, the per-root replication lines (each carrying its own
      decision seed, so a failing replication feeds straight back to
      `table ... --decision-seed`), the binomial line, and the exclusion /
      dissent blocks. THE COMMAND IS THE RECORD.

      <id>                   one of: final-round | end-trigger |
                             exclusive-race | achievement
      --block pilot          the declaration's PILOT ordinals. --master is
                             MANDATORY here: recorded-before-use stays the
                             caller's obligation and this instrument will not
                             invent one.
      --block confirmatory   the FROZEN block baked into the declaration at the
                             freeze commit -- PER ARM, since the two arms draw
                             their own roots off their own masters. --master is
                             REFUSED here: the declaration's master IS the
                             committed identity, and a typo must not be able to
                             run a plausible-looking non-frozen block. An arm
                             carrying no baked block refuses BY NAME.
      --arm positive         (default) the family's positive generator.
      --arm control          its declared control (or corner) arm, at the
                             control PASS RULE where the control asks a
                             different question:
                               final-round     trigger-round-not-final, same rule
                               end-trigger     the behind-in-truth complement,
                                               read as not-end-fired-by-handoff
                               exclusive-race  the no-denial builder, read as
                                               denial-not-taken-by-handoff
                               achievement     the proximity corner, same rule
      SIZING is the BLOCK's: a pilot runs BlockOpts::pilot (admission K = 4, the
      posture the pilot runs were taken at, unchanged so a pilot re-draws the
      same roots and adjudications) and a confirmatory runs BlockOpts::freeze
      (admission K = 8, doubled against group-mean noise). The ARM never moves
      the sizing. The SUBJECT is the shipped `lookahead-v1`: a family result is a
      statement about that subject, so it is deliberately not a flag here.
      COST: `roots x menu x candidates x K` whole games for admission plus
      `roots x R` subject turns for the reads -- tens of minutes per root in
      release at the declared sizing. The pilot is what priced it, and a
      confirmatory block costs about twice its pilot per root on K alone.

  opening --master <u64> --games <N> [--contestant <spec>]
      The C-D OPENING PROBE: N seeded fresh 2P games, the subject's FIRST TURN
      played move by move, and the distribution over (placement area types,
      president area type, first area action) with the lab-rush share and its
      95% Wilson interval. Cheap, recorded, no gate. The SETUP PATH is printed
      in the output, because a population is only comparable to another when the
      path that produced it is on the page.

      --games <N>            required; 0 is refused (a probe of no games
                             measures nothing).
      --contestant <spec>    default `lookahead-v1`. Any contestant the grammar
                             spells plays here -- unlike `table`, an opening turn
                             needs no ranked surface, so a rung is a legitimate
                             (and much cheaper) subject.

  capacity <predicate> --master <u64> [--stop-after <n>]
      Measure how many strided ORDINALS a predicate's scan realizes at a master,
      and which generated game each one came from. NO ROOT TABLE is built or
      read and no position appears anywhere in the output -- which is why V4's
      spec §6 calls this measurement permitted, required at step 2, and NOT a
      burn. It takes a PREDICATE and not a <root>, deliberately: capacity is a
      question about a predicate and a master, and taking a root would let the
      one command §6 exempts be pointed at an ordinal.

      --stop-after <n>       bound the walk at n ordinals. The report then says
                             TRUNCATED and its count is a LOWER BOUND on the
                             supply, never the supply. 0 is refused (a walk
                             bounded at zero ordinals measures nothing). Omit it
                             for the exact figure, at the ecology's whole price.
      COST is the ECOLOGY's, and it is a scan's: seconds for the rung-scanned
      family-1 pair, minutes to hours for the six that scan the v2 LOOKAHEAD
      ecology. That price is the reason to know the number BEFORE a master is
      drawn, which is where §6 puts the measurement.

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
  gen:<predicate>:<master>:<ordinal>
                   the ordinal-th position a seeded SCAN at <master> found
                   satisfying <predicate> -- so the separator and the
                   adjudicator are runnable on generated roots. The resolved
                   root's whole provenance prints in the report header
                   (predicate, master, ordinal, the scan's own game index, and
                   the GeneratingEcology, WITHOUT which the triple is not an
                   identity: the ordinal-th hit is a fact about the games the
                   scan played). <master> takes the same spelling --master does;
                   <ordinal> is decimal. It carries NO recorded seed and NO
                   recorded move, so `table` requires --decision-seed and prints
                   NOT CHECKED, exactly as it does for a path root.
                   COST is the ECOLOGY's: the family-1 predicates scan a rung
                   ecology (seconds); the four layer-2 predicates and the two V4
                   predicates scan the v2 LOOKAHEAD ecology, where both
                   generating seats search, and that is minutes.
                   <predicate> is one of:
                     final-round-already-triggered | trigger-round-not-final
                     end-firable-ahead-in-truth | end-firable-behind-in-truth
                     exclusive-race-one-step | achievement-claim-available
                     placement-fork-same-count | opponent-sitting-executable
                   The last two are V4's step-2 pair (spec §6). They alone carry
                   ORDINAL BANDS -- development 0..4, development extension 4..8,
                   escrow 8..16 -- and the escrow band is fenced by the BURN
                   RULE. The other six have no band: family blocks run
                   confirmatory ordinals 4..28 by declaration.

the BURN RULE (--escrow-read <ordinal>, on table / run / adjudicate / exact):
  Spec §6 fences every read at ordinal >= 8 of a BANDED predicate until step 10,
  and `table` / `run` / `adjudicate` / `exact` REFUSE such a root rather than
  taking it. The refusal names the predicate, the ordinal, the escrow floor and
  the development bands. If this IS step 10, acknowledge the burn by naming the
  ordinal: --escrow-read <that same ordinal>. It is an ACKNOWLEDGEMENT and not a
  switch -- it must MATCH, and it is REFUSED where no burn is happening, so a
  flag pasted off another command line can never unlock this one. The scope is
  the two banded predicates alone: ordinal >= 8 is entirely ordinary for the
  other six, whose frozen family blocks run confirmatory ordinals 4..28.

seats (--seats <a,b[,c,d]>, one spec per seat in seat order, default
`lookahead-v1` on EVERY seat):
  the contestant grammar   Contestant::from_str_name's, where unknown is an error
                           at every level and an unknown knob is never silently
                           ignored:
      greedy-v1[:m=<n>]
      lookahead-v1[:b=<n>,k=<n>,m=<n>,cap=<n>]
      uniform | retreat-averse | family-weighted[:p=<ppm>,k=<n>]
      atomic-diag-v1[:d=<n>,k=<n>,m=<n>] | mcts-v1[:cap=,c=,scale=,m=,leaf=]
  the declared REPLY menu  the four bare names below. The first three resolve to
                           the same contestants the grammar spells and differ
                           only in what the provenance records they were ASKED
                           FOR, which is the half that stops meaning what it
                           means at D1's identity freeze:
      family-weighted | greedy-at-final | lookahead-at-final | race-rusher
  `family-weighted` ALONE is the reply-menu slot; `family-weighted:p=...` is the
  knobbed rung through the contestant grammar. Both resolve to a rung.
  `race-rusher` is the declared ADVERSARIAL arm and names NO contestant at all:
  a rule over state that takes a race step when one is in front of it and
  delegates to an inner greedy-at-final otherwise. It is a harness instrument,
  never an entrant, and it is spellable here only as a REPLY.

masters:
  --master has NO DEFAULT on `run`, `adjudicate`, `opening`, or a `family`
  PILOT block (a `family` CONFIRMATORY block refuses it outright). A master is drawn from OS
  entropy and RECORDED BY THE CALLER BEFORE THE RUN (the standing policy), so an
  instrument that drew one here would put the draw after the decision to run, in
  a process whose output is the thing the draw is supposed to be independent of.
  Decimal, or hex with a 0x prefix; `_` separators are ignored.

Unknown subcommands, unknown flags, unknown seat names, unknown contestant knobs
and stray positionals are all ERRORS carrying this text -- never silent no-ops.
A REPEATED flag is an ERROR too, which is a deliberate departure from
eval_probe's last-wins: dropping the first value silently is the same
substitution failure one level down from a mistyped flag.
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
    /// The ordinal-th position a seeded scan at `master` found satisfying
    /// `predicate` — the `gen:` form (Phase C, T6).
    ///
    /// Nothing is scanned at parse time: like the other two arms this names a
    /// root and resolves later, which is what keeps the whole grammar testable
    /// without playing a game.
    Generated { predicate: RootPredicate, master: u64, ordinal: u32 },
}

/// Which ARM of a family's declaration a block run uses.
///
/// A family's control (or corner) arm is a declared GENERATOR and, where the
/// control asks a different question, a declared control PASS RULE — never a
/// different family, which is why this is a flag on `family <id>` rather than
/// four more ids. See [`control_arm`] for the mapping and its reasoning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyArm {
    /// The declaration as [`families`] carries it.
    Positive,
    /// Its declared control (or corner) arm.
    Control,
}

impl FamilyArm {
    /// The stable name, as the grammar spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Control => "control",
        }
    }
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
    /// One EXACT bounded-tree adjudication (`game::driven_exact`).
    ///
    /// Carries no master and no seats: the solve is a pure function of the root,
    /// the rules and its bounds, and both seats are played OPTIMALLY rather than
    /// by any policy.
    Exact {
        root: RootRef,
        bounds: driven_exact::ExactBounds,
        /// The tolerance the top set is reported at, BESIDE the ε = 0 argmax.
        eps_vp: i64,
        /// `Some(master)` runs the W = 5 world-invariance rider afterwards.
        ///
        /// **The ONE place `exact` accepts a master**, and the carve-out is the
        /// spec's own: the SOLVE draws no worlds, the RIDER does. Off unless a
        /// master is supplied, because a rider without a recorded master would
        /// put the draw after the decision to run.
        rider: Option<u64>,
    },
    /// One declared block of one frozen family.
    ///
    /// `master` stays an `Option` all the way to
    /// [`run_family_block_with`](crate::game::driven_families::run_family_block_with):
    /// the pilot-requires-it / confirmatory-refuses-it rule is THAT function's,
    /// and re-stating it here would give the harness two authorities on which
    /// master a block runs.
    Family {
        id: &'static str,
        block: BlockKind,
        master: Option<u64>,
        arm: FamilyArm,
    },
    /// The C-D opening probe.
    Opening {
        master: u64,
        games: u32,
        contestant: Contestant,
    },
    /// ONE predicate's realized ordinal SUPPLY at a master — spec §6's step-2
    /// capacity measurement.
    ///
    /// Deliberately NOT a `<root>` subcommand: capacity is a question about a
    /// predicate and a master, not about a position, and taking a root here
    /// would let the one command §6 calls "not a burn" be pointed at an ordinal.
    Capacity {
        predicate: RootPredicate,
        master: u64,
        /// `None` walks the ecology's whole game budget — the exact supply, at
        /// the ecology's whole price.
        stop_after: Option<u32>,
    },
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a command line (argv WITHOUT the program name).
pub fn parse(args: &[String]) -> Result<Command, String> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(format!("a subcommand is required\n\n{USAGE}"));
    };
    let rest = &args[1..];
    match subcommand {
        "list" => {
            let (positional, _) = split_args(rest, LIST_FLAGS)?;
            if !positional.is_empty() {
                return Err(format!(
                    "list takes no positional argument, got {positional:?}\n\n{USAGE}"
                ));
            }
            Ok(Command::List)
        }
        "table" => {
            let (positional, flags) = split_args(rest, TABLE_FLAGS)?;
            let root = one_root(&positional, "table")?;
            deny_escrow_burn(&root, flags.get(ESCROW_READ_FLAG), "table")?;
            let contestant = match flags.get("contestant") {
                Some(spec) => Contestant::from_str_name(spec)
                    .map_err(|e| format!("--contestant {spec:?}: {e}\n\n{USAGE}"))?,
                None => default_contestant()?,
            };
            let decision_seed = flags.get("decision-seed").map(|raw| parse_u64(raw)).transpose()?;
            Ok(Command::Table { root, contestant, decision_seed })
        }
        "run" => {
            let (positional, flags) = split_args(rest, RUN_FLAGS)?;
            let root = one_root(&positional, "run")?;
            deny_escrow_burn(&root, flags.get(ESCROW_READ_FLAG), "run")?;
            Ok(Command::Run {
                root,
                master: master_of(&flags, "run")?,
                seats: seats_of(&flags)?,
            })
        }
        "adjudicate" => {
            let (positional, flags) = split_args(rest, ADJUDICATE_FLAGS)?;
            let root = one_root(&positional, "adjudicate")?;
            deny_escrow_burn(&root, flags.get(ESCROW_READ_FLAG), "adjudicate")?;
            let replications = match flags.get("replications") {
                Some(raw) => u32::try_from(parse_u64(raw)?)
                    .map_err(|_| format!("--replications {raw:?} is not a u32\n\n{USAGE}"))?,
                None => DEFAULT_REPLICATIONS,
            };
            Ok(Command::Adjudicate {
                root,
                master: master_of(&flags, "adjudicate")?,
                replications,
                seats: seats_of(&flags)?,
            })
        }
        "exact" => {
            // Checked BEFORE `split_args`, so `--master` gets the sentence that
            // says why rather than a bare "unknown flag" a caller would read as
            // a typo on their own part.
            if rest.iter().any(|a| a == "--master") {
                return Err(format!(
                    "exact takes no --master: the exact solve DRAWS NO worlds and derives no \
                     seed — its answer is a pure function of (root, rules, bounds), so accepting \
                     one would advertise a dependence it does not have. `adjudicate` requires \
                     --master precisely because its playouts do need worlds.\n\n{USAGE}"
                ));
            }
            let (positional, flags) = split_args(rest, EXACT_FLAGS)?;
            let root = one_root(&positional, "exact")?;
            deny_escrow_burn(&root, flags.get(ESCROW_READ_FLAG), "exact")?;
            let defaults = driven_exact::ExactBounds::default();
            let bounds = driven_exact::ExactBounds {
                max_nodes: match flags.get("max-nodes") {
                    Some(raw) => parse_u64(raw)?,
                    None => defaults.max_nodes,
                },
                max_depth: match flags.get("max-depth") {
                    Some(raw) => u32::try_from(parse_u64(raw)?)
                        .map_err(|_| format!("--max-depth {raw:?} is not a u32\n\n{USAGE}"))?,
                    None => defaults.max_depth,
                },
                max_memo_entries: match flags.get("max-memo") {
                    Some(raw) => usize::try_from(parse_u64(raw)?)
                        .map_err(|_| format!("--max-memo {raw:?} is not a usize\n\n{USAGE}"))?,
                    None => defaults.max_memo_entries,
                },
                // Not a flag: ordering cannot change a value, so exposing it
                // would offer a caller a choice with no answer attached.
                order_moves: defaults.order_moves,
            };
            let eps_vp = match flags.get("eps") {
                Some(raw) => i64::try_from(parse_u64(raw)?)
                    .map_err(|_| format!("--eps {raw:?} is not a non-negative integer\n\n{USAGE}"))?,
                None => FROZEN_EPSILON_VP,
            };
            let rider = flags.get("rider").map(|raw| parse_u64(raw)).transpose()?;
            Ok(Command::Exact { root, bounds, eps_vp, rider })
        }
        "family" => {
            let (positional, flags) = split_args(rest, FAMILY_FLAGS)?;
            let id = one_family(&positional)?;
            let block = match flags.get("block") {
                Some(raw) => parse_block(raw)?,
                None => {
                    return Err(format!(
                        "family {id} needs --block: {}\n\n{USAGE}",
                        block_vocabulary(),
                    ))
                }
            };
            let arm = match flags.get("arm") {
                Some(raw) => parse_arm(raw)?,
                None => FamilyArm::Positive,
            };
            let master = flags.get("master").map(|raw| parse_u64(raw)).transpose()?;
            Ok(Command::Family { id, block, master, arm })
        }
        "opening" => {
            let (positional, flags) = split_args(rest, OPENING_FLAGS)?;
            if !positional.is_empty() {
                return Err(format!(
                    "opening takes no positional argument, got {positional:?}\n\n{USAGE}"
                ));
            }
            let games = match flags.get("games") {
                Some(raw) => u32::try_from(parse_u64(raw)?)
                    .map_err(|_| format!("--games {raw:?} is not a u32\n\n{USAGE}"))?,
                None => {
                    return Err(format!(
                        "opening requires --games, which has no default: a probe's N is the whole \
                         of what its distribution means\n\n{USAGE}"
                    ))
                }
            };
            if games == 0 {
                return Err(format!(
                    "opening --games 0 measures nothing: a probe of no games has no \
                     distribution and no rate to put an interval around\n\n{USAGE}"
                ));
            }
            let contestant = match flags.get("contestant") {
                Some(spec) => Contestant::from_str_name(spec)
                    .map_err(|e| format!("--contestant {spec:?}: {e}\n\n{USAGE}"))?,
                None => default_contestant()?,
            };
            Ok(Command::Opening { master: master_of(&flags, "opening")?, games, contestant })
        }
        "capacity" => {
            let (positional, flags) = split_args(rest, CAPACITY_FLAGS)?;
            let predicate = one_predicate(&positional)?;
            let stop_after = match flags.get("stop-after") {
                Some(raw) => {
                    let bound = u32::try_from(parse_u64(raw)?).map_err(|_| {
                        format!("--stop-after {raw:?} is not a u32\n\n{USAGE}")
                    })?;
                    if bound == 0 {
                        return Err(format!(
                            "capacity --stop-after 0 measures nothing: a walk bounded at zero \
                             ordinals plays no game and counts no hit, so it reports neither a \
                             supply nor a lower bound on one. Omit the flag to walk the whole \
                             ecology.\n\n{USAGE}"
                        ));
                    }
                    Some(bound)
                }
                None => None,
            };
            Ok(Command::Capacity {
                predicate,
                master: master_of(&flags, "capacity")?,
                stop_after,
            })
        }
        other => Err(format!("unknown subcommand {other:?}\n\n{USAGE}")),
    }
}

/// The ONE predicate `capacity` takes, named exactly as a `gen:` root names it.
fn one_predicate(positional: &[String]) -> Result<RootPredicate, String> {
    let [only] = positional else {
        return Err(format!(
            "capacity takes exactly ONE predicate, got {}: {positional:?}\n\n{USAGE}",
            positional.len(),
        ));
    };
    RootPredicate::ALL.into_iter().find(|candidate| candidate.as_str() == only).ok_or_else(|| {
        let known: Vec<&str> = RootPredicate::ALL.into_iter().map(RootPredicate::as_str).collect();
        format!(
            "unknown predicate {only:?}: the scan predicates are {}\n\n{USAGE}",
            known.join(" | "),
        )
    })
}

/// **THE BURN-RULE GUARD** (spec §6): no `table` / `adjudicate` / read of any
/// kind at ordinal ≥ 8 of a BANDED predicate before step 10.
///
/// **Why a guard at all, when the rule is written down.** The existing
/// [`BLOCK_NAMES`] note says out loud that escrow ordinals live in a freeze
/// DOCUMENT so no command can burn one by accident — and then that the `gen:`
/// escape hatch technically reaches one anyway, with the protection procedural.
/// That was the honest disposition for a family block whose escrow lived in
/// prose. V4's escrow band lives in CODE ([`driven::V4_ESCROW_ORDINALS`]), so
/// the same honesty now buys a refusal instead of a sentence.
///
/// **Why SCOPED to [`driven::V4_BANDED_PREDICATES`].** Ordinal ≥ 8 is entirely
/// ordinary for the other six: the frozen family blocks run confirmatory
/// ordinals 4..28 by declaration, and a general fence would refuse the frozen
/// blocks' own roots. Only the two V4 predicates have bands, so only they are
/// fenced.
///
/// **Why ≥ 8 rather than the band 8..16.** The rule §6 states is "no read of any
/// kind at ordinal ≥ 8", not "inside the escrow band" — an ordinal past the
/// band is not a development ordinal either, and reading one would be a read of
/// the same population the escrow block is drawn from.
///
/// **Why the acknowledgement takes the ORDINAL.** Step 10 has to be able to read
/// escrow, so a refusal with no way through would only move the problem to a
/// code edit made under time pressure. `--escrow-read <ordinal>` is a way
/// through that cannot be taken by accident: it must be typed, it must MATCH,
/// it lands verbatim in the commands-as-record, and it is REFUSED where no burn
/// is happening — so it can never be a habit already present when the burn
/// arrives.
fn deny_escrow_burn(
    root: &RootRef,
    acknowledgement: Option<&String>,
    subcommand: &str,
) -> Result<(), String> {
    let burning = match root {
        RootRef::Generated { predicate, ordinal, .. }
            if driven::V4_BANDED_PREDICATES.contains(predicate)
                && *ordinal >= driven::V4_ESCROW_ORDINALS.start =>
        {
            Some((*predicate, *ordinal))
        }
        _ => None,
    };
    match (burning, acknowledgement) {
        (None, None) => Ok(()),
        (None, Some(raw)) => Err(format!(
            "--{ESCROW_READ_FLAG} {raw:?} names a burn that is not happening: {subcommand} was \
             pointed at a root that is not an escrow ordinal of a banded predicate. The flag is an \
             ACKNOWLEDGEMENT of spec §6's burn rule, not a switch, and accepting it where there is \
             nothing to acknowledge would let it become a habit that is already typed when the \
             burn arrives.\n\n{USAGE}"
        )),
        (Some((predicate, ordinal)), None) => Err(format!(
            "BURN RULE (spec §6): ordinal {ordinal} of `{}` is at or past the escrow floor {}, and \
             no table / adjudicate / read of any kind may touch it before step 10. Development \
             ordinals are {}..{} and the development extension is {}..{}. If this IS step 10, say \
             so by naming the ordinal: --{ESCROW_READ_FLAG} {ordinal}. The step-10 ledger row \
             records exactly which ordinals were read, and this command is what it \
             records.\n\n{USAGE}",
            predicate.as_str(),
            driven::V4_ESCROW_ORDINALS.start,
            driven::V4_DEVELOPMENT_ORDINALS.start,
            driven::V4_DEVELOPMENT_ORDINALS.end,
            driven::V4_DEVELOPMENT_EXTENSION_ORDINALS.start,
            driven::V4_DEVELOPMENT_EXTENSION_ORDINALS.end,
        )),
        (Some((predicate, ordinal)), Some(raw)) => {
            let acknowledged = raw.parse::<u32>().map_err(|e| {
                format!(
                    "--{ESCROW_READ_FLAG} {raw:?} is not a decimal ordinal: {e}. It acknowledges \
                     the ordinal being burned, so it is that number and nothing else.\n\n{USAGE}"
                )
            })?;
            if acknowledged == ordinal {
                Ok(())
            } else {
                Err(format!(
                    "--{ESCROW_READ_FLAG} {acknowledged} does not acknowledge ordinal {ordinal} of \
                     `{}`, which is the escrow read this command would take. An acknowledgement \
                     that did not have to match would unlock whichever command it was pasted \
                     onto.\n\n{USAGE}",
                    predicate.as_str(),
                ))
            }
        }
    }
}

/// The four declared family ids, as one sentence.
fn family_vocabulary() -> String {
    let ids: Vec<&str> = families().iter().map(|decl| decl.id).collect();
    ids.join(" | ")
}

/// The two block names, as one sentence — see [`BLOCK_NAMES`] for why there are
/// exactly two.
fn block_vocabulary() -> String {
    format!("the blocks are {}", BLOCK_NAMES.join(" | "))
}

/// The ONE family id `family` takes, checked against the declarations so a typo
/// refuses before anything is resolved.
fn one_family(positional: &[String]) -> Result<&'static str, String> {
    let [only] = positional else {
        return Err(format!(
            "family takes exactly ONE id, got {}: {positional:?} ({})\n\n{USAGE}",
            positional.len(),
            family_vocabulary(),
        ));
    };
    families()
        .into_iter()
        .find(|decl| decl.id == only)
        .map(|decl| decl.id)
        .ok_or_else(|| {
            format!(
                "unknown family {only:?}: the declared families are {}\n\n{USAGE}",
                family_vocabulary(),
            )
        })
}

/// `--block`.
fn parse_block(raw: &str) -> Result<BlockKind, String> {
    match raw {
        "pilot" => Ok(BlockKind::Pilot),
        "confirmatory" => Ok(BlockKind::Confirmatory),
        other => Err(format!("unknown block {other:?}: {}\n\n{USAGE}", block_vocabulary())),
    }
}

/// `--arm`.
fn parse_arm(raw: &str) -> Result<FamilyArm, String> {
    match raw {
        "positive" => Ok(FamilyArm::Positive),
        "control" => Ok(FamilyArm::Control),
        other => Err(format!(
            "unknown arm {other:?}: the arms are {}\n\n{USAGE}",
            ARM_NAMES.join(" | "),
        )),
    }
}

/// argv split into positionals and `--flag value` pairs.
///
/// EVERY flag this runner has takes a value, so there is no bare-flag arm — and
/// a valueless flag is therefore an error rather than a silent `true`. An
/// unknown flag is an error too (`eval_probe`'s rule): a mistyped
/// `--decision-see` that quietly left the entry's recorded seed in place would
/// print a reproduction verdict about a decision nobody asked for. A REPEATED
/// flag is an error as well rather than last-wins, since losing the first value
/// is the same silent-substitution failure one level down.
fn split_args(
    args: &[String],
    valued: &[&str],
) -> Result<(Vec<String>, BTreeMap<String, String>), String> {
    let mut positional = Vec::new();
    let mut flags: BTreeMap<String, String> = BTreeMap::new();
    let mut i = 0;
    while i < args.len() {
        if let Some(name) = args[i].strip_prefix("--") {
            if !valued.contains(&name) {
                return Err(format!("unknown flag --{name}\n\n{USAGE}"));
            }
            let value =
                args.get(i + 1).ok_or_else(|| format!("--{name} needs a value\n\n{USAGE}"))?;
            if flags.insert(name.to_string(), value.clone()).is_some() {
                return Err(format!("--{name} was given twice\n\n{USAGE}"));
            }
            i += 2;
            continue;
        }
        positional.push(args[i].clone());
        i += 1;
    }
    Ok((positional, flags))
}

/// The ONE root every subcommand but `list` takes.
fn one_root(positional: &[String], subcommand: &str) -> Result<RootRef, String> {
    match positional {
        [only] => parse_root_ref(only),
        other => Err(format!(
            "{subcommand} takes exactly ONE root, got {}: {other:?}\n\n{USAGE}",
            other.len()
        )),
    }
}

/// `--master`, which has no default anywhere.
fn master_of(flags: &BTreeMap<String, String>, subcommand: &str) -> Result<u64, String> {
    let raw = flags.get("master").ok_or_else(|| {
        format!(
            "{subcommand} requires --master, which has no default: a master is drawn from OS \
             entropy and RECORDED BY THE CALLER BEFORE THE RUN (the standing policy), so this \
             instrument takes the recorded value rather than drawing one — a draw made here would \
             sit after the decision to run, in a process whose output is the thing the draw is \
             supposed to be independent of\n\n{USAGE}"
        )
    })?;
    parse_u64(raw)
}

/// `--seats`, left `None` when absent because the default needs the root's
/// player count and nothing here has read the root yet.
fn seats_of(flags: &BTreeMap<String, String>) -> Result<Option<Vec<SeatSpec>>, String> {
    flags.get("seats").map(|raw| parse_seats(raw)).transpose()
}

/// [`DEFAULT_SEAT`] through the grammar. Propagated rather than unwrapped: a
/// runner that panicked on its own default would be worse than one that says so.
fn default_contestant() -> Result<Contestant, String> {
    Contestant::from_str_name(DEFAULT_SEAT)
        .map_err(|e| format!("the default contestant {DEFAULT_SEAT:?} does not parse: {e}"))
}

/// `gen:<predicate>:<master>:<ordinal>`, `<path>:<event>`, or a bare registry
/// name.
///
/// **The `gen:` prefix is read FIRST**, before the last-colon split: a generated
/// spec carries three colons and a path may carry any number, so reading the
/// prefix is the only rule under which the two grammars cannot shadow one
/// another. The cost is one absurd collision, stated rather than left to be
/// discovered: a record file literally NAMED `gen` is spelled `./gen:3`, since
/// `gen:3` now reads as a malformed generated spec — and it refuses naming the
/// generated form, which is the diagnosis a reader needs to reach for the
/// `./`.
pub fn parse_root_ref(raw: &str) -> Result<RootRef, String> {
    if let Some(rest) = raw.strip_prefix(GEN_PREFIX) {
        return parse_generated_root(raw, rest);
    }
    let Some((path, event)) = raw.rsplit_once(':') else {
        if raw.is_empty() {
            return Err(format!("an empty root names nothing\n\n{USAGE}"));
        }
        return Ok(RootRef::Fixture(raw.to_string()));
    };
    // The LAST colon, so a path that contains one still reads. A path with a
    // colon and no event index is a user error rather than a fixture name,
    // because a fixture name with a colon in it does not exist.
    if path.is_empty() {
        return Err(format!("{raw:?} has no path before its last colon\n\n{USAGE}"));
    }
    let event_index = event.parse::<usize>().map_err(|e| {
        format!(
            "{raw:?} carries a colon, so it reads as <path>:<event> split at the LAST colon — but \
             {event:?} is not a decimal event index: {e}\n\n{USAGE}"
        )
    })?;
    Ok(RootRef::Path { path: PathBuf::from(path), event_index })
}

/// The `gen:` form's three fields, each refusing on its own terms.
///
/// `raw` is what the caller typed and rides every refusal, because a spec with
/// the wrong ARITY has no one field to blame and the useful thing to print is
/// the whole of it beside the shape it should have had.
fn parse_generated_root(raw: &str, rest: &str) -> Result<RootRef, String> {
    let fields: Vec<&str> = rest.split(':').collect();
    let [name, master, ordinal] = fields.as_slice() else {
        return Err(format!(
            "{raw:?} is not a generated root: the form is \
             gen:<predicate>:<master>:<ordinal> and this carries {} field(s) after the \
             prefix\n\n{USAGE}",
            fields.len(),
        ));
    };
    let predicate = RootPredicate::ALL
        .into_iter()
        .find(|candidate| candidate.as_str() == *name)
        .ok_or_else(|| {
            let known: Vec<&str> =
                RootPredicate::ALL.into_iter().map(RootPredicate::as_str).collect();
            format!(
                "unknown predicate {name:?} in {raw:?}: the scan predicates are {}\n\n{USAGE}",
                known.join(" | "),
            )
        })?;
    let master = parse_u64(master)
        .map_err(|e| format!("the master in {raw:?} does not read: {e}\n\n{USAGE}"))?;
    let ordinal = ordinal.parse::<u32>().map_err(|e| {
        format!("the ordinal {ordinal:?} in {raw:?} is not a decimal u32: {e}\n\n{USAGE}")
    })?;
    Ok(RootRef::Generated { predicate, master, ordinal })
}

/// ONE seat spec: the reply menu's three bare names, else the contestant
/// grammar.
///
/// The menu is read FIRST and only on an EXACT bare name, which is what keeps
/// `family-weighted` (the menu slot) and `family-weighted:p=…` (the knobbed rung
/// through the contestant grammar) distinct. Both resolve to the same kind of
/// seat; what differs is the `requested` label the provenance records, and that
/// label is the half whose meaning moves at D1's identity freeze.
pub fn parse_seat(spec: &str) -> Result<SeatSpec, String> {
    // HAND-MAINTAINED, and deliberately so: `ReplyPolicy` ships no `ALL`, the
    // USAGE seat list is written by hand beside it, and the pins assert every
    // name here round-trips and appears there. `race-rusher` is the fourth slot
    // (Phase C, T3) and is argv-REACHABLE because T7's pilots want to name it
    // in `--seats`; it resolves to no contestant at all, which
    // `SeatSpec::build`'s second arm answers.
    for policy in [
        ReplyPolicy::FamilyWeighted,
        ReplyPolicy::GreedyAtFinal,
        ReplyPolicy::LookaheadAtFinal,
        ReplyPolicy::RaceRusher,
    ] {
        if spec == policy.as_str() {
            return Ok(SeatSpec::Reply(policy));
        }
    }
    Contestant::from_str_name(spec).map(SeatSpec::Contestant).map_err(|e| {
        format!("seat {spec:?} is neither a reply-menu name nor a contestant: {e}\n\n{USAGE}")
    })
}

/// A comma-separated seat list, one spec per seat in seat order.
pub fn parse_seats(raw: &str) -> Result<Vec<SeatSpec>, String> {
    if raw.trim().is_empty() {
        return Err(format!("--seats needs at least one seat\n\n{USAGE}"));
    }
    raw.split(',').map(|spec| parse_seat(spec.trim())).collect()
}

/// `eval_probe`'s spelling, so a master typed for one instrument means the same
/// thing typed for the other: decimal, or `0x` hex, with `_` separators ignored.
pub fn parse_u64(raw: &str) -> Result<u64, String> {
    let cleaned = raw.replace('_', "");
    let parsed = match cleaned.strip_prefix("0x").or_else(|| cleaned.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => cleaned.parse::<u64>(),
    };
    parsed.map_err(|e| format!("{raw:?} is not a u64: {e}"))
}

// ---------------------------------------------------------------------------
// The family arms
// ---------------------------------------------------------------------------

/// A declaration RE-POINTED at its declared CONTROL (or corner) arm.
///
/// The design's registry is FOUR families, and each control is an ARM of its
/// family rather than a fifth row — so the mapping from an id to its arm lives
/// here, in the grammar that exposes it, and the ARMS themselves live in
/// `driven_families` as named handles ([`FAMILY2_CONTROL_GENERATOR`],
/// [`RACE_CONTROL_GENERATOR`], [`PROXIMITY_CORNER_GENERATOR`]) exactly so this
/// function has something to point AT rather than something to invent. Family
/// 1's control has no handle because it is a plain predicate
/// ([`RootPredicate::TriggerRoundNotFinal`], the declaration's own comment
/// names it); it is exposed the same way.
///
/// | family           | control generator                | control rule                           |
/// |------------------|----------------------------------|----------------------------------------|
/// | `final-round`    | `TriggerRoundNotFinal` predicate | unchanged (`AdjudicatedBest`)          |
/// | `end-trigger`    | `FAMILY2_CONTROL_GENERATOR`      | `Turn(NotEndFired)`                    |
/// | `exclusive-race` | `RACE_CONTROL_GENERATOR`         | `Turn(DenialNotTaken)`                 |
/// | `achievement`    | `PROXIMITY_CORNER_GENERATOR`     | unchanged (`Turn(AchievementClaimed)`) |
///
/// **The two rule swaps are NEGATIONS, which is why the admission inversion
/// stays OFF under them.** That is the double negation, and it is stated ONCE —
/// at [`family_block_opts`], the single seam that decides `BlockOpts::control`
/// — because a judgment split across two functions is a judgment nobody
/// re-checks. This function's half of it: the control's READ is the negated
/// predicate, so `end-trigger` swaps in `Turn(NotEndFired)` and
/// `exclusive-race` swaps in `Turn(DenialNotTaken)`.
///
/// **THE FROZEN BLOCK IS RE-POINTED TOO** (the freeze wave): the arm's
/// `confirmatory` becomes the declaration's
/// [`confirmatory_control`](FamilyDecl::confirmatory_control), because the two
/// arms draw their own roots off their own masters and a control run that
/// inherited the positive arm's block would quote the positive arm's identity
/// for a different set of positions. Family 1's two blocks happen to name one
/// master — its control is a different PREDICATE, so that master's ordinal space
/// is a different space — and the re-point is what makes even that explicit.
///
/// Every other field is the declaration's, untouched: an arm is a generator, a
/// rule and a block, never a different family.
pub fn control_arm(decl: &FamilyDecl) -> Result<FamilyDecl, String> {
    let (generator, rule) = match decl.id {
        "final-round" => (FamilyGenerator::Predicate(RootPredicate::TriggerRoundNotFinal), decl.rule),
        "end-trigger" => (FAMILY2_CONTROL_GENERATOR, PassRule::Turn(TurnPredicate::NotEndFired)),
        "exclusive-race" => (
            RACE_CONTROL_GENERATOR,
            PassRule::Turn(TurnPredicate::DenialNotTaken(DECLARED_BY_THE_ROOT)),
        ),
        "achievement" => (PROXIMITY_CORNER_GENERATOR, decl.rule),
        other => {
            return Err(format!(
                "no control arm for family {other:?}: the declared families are {}",
                family_vocabulary(),
            ))
        }
    };
    let mut arm = decl.clone();
    arm.generator = generator;
    arm.rule = rule;
    arm.confirmatory = decl.confirmatory_control.clone();
    Ok(arm)
}

/// The SIZING one `family` block runs under — the ARM never moves it (which is
/// the double negation made STRUCTURAL instead of remembered) and the BLOCK
/// always does: [`BlockOpts::pilot`] for a pilot, [`BlockOpts::freeze`] for a
/// confirmatory.
///
/// **Why this is a function and not a literal at the call site** (Phase C, T6
/// fix wave, review MUST-1). The judgment below is correct and it is also
/// invisible: `BlockOpts { control: arm == FamilyArm::Control, ..pilot() }` is
/// the edit a reader reaches for, it compiles, it reads *more* careful than
/// what is here, and NO CELL HAS EVER EXECUTED THE LINE it would change — every
/// family block refused before the freeze, and since the freeze a real
/// confirmatory block is hours of games that no suite can afford. A named seam
/// gives that edit somewhere to fail: the pin below asserts BOTH arms come back
/// with the inversion off at BOTH blocks, and `render_family_block` brands a
/// control render whose report says otherwise.
///
/// **The double negation, stated once because getting it backwards is silent.**
/// Design §1 says a family-2 control root passes on `NotEndFiredByHandoff` and
/// a family-3 control root passes on the denial NOT being taken, so the
/// control's READ is the negated predicate ([`control_arm`] swaps it in); the
/// read and the admission hook are ONE rule in `run_one_root`, so the admission
/// question becomes "does NOT firing dominate?", which is already the right
/// direction for a behind mover. `BlockOpts::control` inverts that comparison a
/// SECOND time — it is the other spelling of the same test, for a caller that
/// kept the positive rule — so setting both would ask whether not-firing LOSES,
/// which is the control's own claim upside down. The arithmetic is identical
/// either way (`mean(¬fire) − mean(fire) ≥ δ` is `mean(fire) − mean(¬fire) ≤
/// −δ`); what is not identical is the READ, and the negated rule is the one
/// that reads right. The report prints the arm and this sentence so nothing
/// rests on remembering it.
///
/// **THE BLOCK IS the sizing choice** (the freeze wave): a PILOT runs the
/// posture the pilots that already ran used — so their readings reproduce — and
/// a CONFIRMATORY runs the frozen one, whose single moved field is the admission
/// K (4 → 8, priced from pilot-measured group-mean noise). The two questions
/// meet in one function on purpose: the arm must not reach the sizing and the
/// block must, and both facts are checked by one cell.
///
/// Both `match`es are EXHAUSTIVE rather than `_ => BlockOpts::pilot()`: a third
/// arm — or a third block — has to answer this question rather than inherit an
/// answer written for two.
pub(crate) fn family_block_opts(arm: FamilyArm, block: BlockKind) -> BlockOpts {
    match arm {
        FamilyArm::Positive | FamilyArm::Control => match block {
            BlockKind::Pilot => BlockOpts::pilot(),
            BlockKind::Confirmatory => BlockOpts::freeze(),
        },
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// The fifteen [`SearchStats`] counters as `(name, value)`, in declaration
/// order.
///
/// **The body DESTRUCTURES `SearchStats` with no `..`**, which is the whole
/// point: a sixteenth counter added to that type is a COMPILE ERROR here, to be
/// answered with a row. A `stats.field` list would silently stay fifteen, and
/// `why` printing 2 of 15 is exactly the gap this table exists to close.
pub fn counter_rows(stats: &SearchStats) -> [(&'static str, u64); 15] {
    let SearchStats {
        root_candidates,
        nodes,
        handles,
        route_dfs_nodes,
        forced_nodes,
        leaves,
        scored,
        root_scored,
        budget_binds,
        budget_starved,
        placement_cap_binds,
        max_line,
        ply_cap_hits,
        free_extension_steps,
        free_cap_hits,
    } = *stats;
    [
        ("root_candidates", root_candidates),
        ("nodes", nodes),
        ("handles", handles),
        ("route_dfs_nodes", route_dfs_nodes),
        ("forced_nodes", forced_nodes),
        ("leaves", leaves),
        ("scored", scored),
        ("root_scored", root_scored),
        ("budget_binds", budget_binds),
        ("budget_starved", budget_starved),
        ("placement_cap_binds", placement_cap_binds),
        ("max_line", u64::from(max_line)),
        ("ply_cap_hits", ply_cap_hits),
        ("free_extension_steps", free_extension_steps),
        ("free_cap_hits", free_cap_hits),
    ]
}

/// `Some(n)` as `n`, `None` as `-` — the counter-absence distinction, rendered.
/// A `None` counter means "this contestant has no such quantity" and must never
/// print as a zero.
fn opt(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_string(), |v| v.to_string())
}

/// A RENDER-ONLY 16-hex fold of an already-computed reproduction string.
///
/// **Why this exists.** [`Adjudication::digest`] is `candidates × K` long — it
/// folds in every playout's world, margin, decision count and terminal digest —
/// so a real 139-candidate × 2-replication table digests to ~15 KB. Printed
/// raw, that is one "line" nobody can paste into a triage note, and a
/// reproduction check nobody performs is not a check. This is `state_digest`'s
/// own recipe (FNV-1a 64, 16 hex) over the digest STRING, so the report carries
/// a value a reader can compare at a glance.
///
/// **It is a DISPLAY shortening and NEVER an identity.** The digest function is
/// untouched and stays the authority: reproduction compares
/// `Adjudication::digest()` itself. Nothing writes this fold to disk, nothing
/// parses it back, and no record stamps it — which is exactly why it is
/// hand-rolled here rather than reaching for `game::semantics`'s (private) fold,
/// whose recipe is a *stored* identity this module must not start depending on.
fn digest_fold(digest: &str) -> String {
    // FNV-1a 64: the standard offset basis and prime, wrapping because a hash
    // is not a count.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in digest.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The root descriptor as one line: origin kind, root digest, event index.
fn root_line(root: &RootDescriptor) -> String {
    format!(
        "{}@{}  event={}",
        root.origin.kind(),
        root.root_digest,
        root.event_index.map_or_else(|| "-".to_string(), |n| n.to_string()),
    )
}

/// A GENERATED root's own provenance, as report lines — EMPTY for every other
/// origin.
///
/// This is what "the resolved root's identity prints in the header" means for
/// the `gen:` form. A registry root's identity is its name and the file behind
/// it; a generated root's is the seeded scan that FOUND it, and
/// `(predicate, master, ordinal)` is famously not the whole of that: the
/// ordinal-th hit is a fact about the games the scan played, so the
/// [`GeneratingEcology`] rides every line of it — id, driver, seat count,
/// resolved seat echoes, seed derivation, scan bound and stride. Two runs whose
/// ecologies differ are visibly two different questions, which is the entire
/// reason that type exists.
fn generated_lines(root: &RootDescriptor) -> Vec<String> {
    let RootOrigin::Generated { predicate, master, ordinal, game_index, ecology } = &root.origin
    else {
        return Vec::new();
    };
    let GeneratingEcology { id, driver, players, seats, seed_derivation, games, per_game_cap } =
        ecology;
    vec![
        format!(
            "  generated      predicate={}  master={master:#018x}  ordinal={ordinal}  \
             game={game_index}",
            predicate.as_str(),
        ),
        format!(
            "  ecology        id={id}  players={players}  games={games}  \
             per_game_cap={per_game_cap}"
        ),
        format!("  ecology driver {driver}"),
        format!("  ecology seats  {}", seats.join("  |  ")),
        format!("  ecology seeds  {seed_derivation}"),
    ]
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
///
/// **Absence is reported, never refused.** The session roots live in gitignored
/// saves, so a checkout without them must still be able to ask what the registry
/// names — and a runner that errored on the first miss would be unusable on
/// exactly the machine where the list matters most.
pub fn render_list(entries: &[RegistryEntry], anchor: &Path) -> String {
    let mut lines = vec![
        "=== driven registry ===".to_string(),
        format!("  {} entries, resolved against {}", entries.len(), anchor.display()),
        "  a MISSING record is a MISS and never a failure: the session roots live in gitignored \
         saves"
            .to_string(),
    ];
    for entry in entries {
        let presence = if anchor.join(entry.path).exists() { "PRESENT" } else { "MISSING" };
        lines.push(String::new());
        lines.push(format!("  {}  [{presence}]", entry.name));
        lines.push(format!("      path    {}", entry.path));
        lines.push(format!(
            "      event   {}   seat p{}   decision seed {:#018x}",
            entry.event_index, entry.bot_seat.0, entry.decision_seed,
        ));
        lines.push(format!(
            "      move    {}",
            entry
                .recorded_move
                .as_ref()
                .map_or_else(|| "(none recorded)".to_string(), |mv| format!("{mv:?}")),
        ));
        lines.push(format!("      note    {}", entry.note));
    }
    lines.join("\n")
}

/// ONE decision, analyzed — the mechanical separator's report.
pub fn render_root_table(table: &RootTable, ctx: &TableContext<'_>) -> String {
    let mut lines = vec![
        "=== driven table ===".to_string(),
        format!("  root           {}   {}", ctx.asked_for, root_line(ctx.root)),
        format!("  seat           p{}", table.seat.0),
        format!("  contestant     {}", table.echo),
        format!("  decision seed  {:#018x}", table.decision_seed),
        format!(
            "  world          sampler_seed={:#018x}  worlds={}  digest={}",
            table.world.sampler_seed, table.world.worlds, table.world.digest,
        ),
        format!("  winner         {:?}", table.winner),
        format!("  winner is a maximiser: {}", table.winner_is_a_maximiser()),
    ];
    lines.extend(generated_lines(ctx.root));
    lines.extend(reproduction_lines(table, ctx));
    lines.push(String::new());
    lines.push(format!(
        "  shape          candidates={}  top={}  tied-at-top={}  top-band(<={} pt)={}  \
         distinct={}",
        table.candidates(),
        table.top_value().map_or_else(|| "-".to_string(), |v| format!("{v:+}")),
        table.tied_at_top(),
        driven::TOP_BAND_POINTS,
        table.top_band(),
        table.distinct_values(),
    ));
    lines.push(String::new());
    lines.push(format!(
        "  ranked head ({} of {}, best first; margin is value - top):",
        table.ranked.len().min(RANKED_HEAD),
        table.ranked.len(),
    ));
    let top = table.top_value().unwrap_or(0);
    for (index, score) in table.ranked.iter().take(RANKED_HEAD).enumerate() {
        lines.push(format!(
            "    {:>3}.  {:>8}  {:>8}   {:?}",
            index + 1,
            format!("{:+}", score.value),
            format!("{:+}", score.value - top),
            score.mv,
        ));
    }
    lines.push(String::new());
    lines.push("  search counters (all fifteen):".to_string());
    for (name, value) in counter_rows(&table.stats) {
        lines.push(format!("    {name:<22} {value}"));
    }
    lines.join("\n")
}

/// The REPRODUCED / DIVERGED / NOT CHECKED verdict, and its override qualifier.
///
/// The check is the mechanical separator's step one, as a command: at the
/// entry's own recorded seed the shipped b=4 arm should pick the move the record
/// carries, and a wider beam is EXPECTED to diverge — that divergence is the
/// measurement rather than a fault. What the qualifier exists for is the case in
/// between: a seed the caller supplied is no longer the entry's own question, so
/// the verdict is still printed and is labelled as not being the entry's check.
fn reproduction_lines(table: &RootTable, ctx: &TableContext<'_>) -> Vec<String> {
    let event = ctx.root.event_index.map_or_else(|| "-".to_string(), |n| n.to_string());
    let Some(recorded) = ctx.recorded_move else {
        return vec![format!(
            "  reproduction   NOT CHECKED: this root carries no recorded move to compare the \
             winner against (only a registry entry does)",
        )];
    };
    let mut lines = vec![if *recorded == table.winner {
        format!("  REPRODUCED     the winner IS the move the record carries at event {event}")
    } else {
        format!(
            "  DIVERGED       the winner is {:?}; the record carries {recorded:?} at event {event}",
            table.winner,
        )
    }];
    if let Some(recorded_seed) = ctx.recorded_seed {
        if recorded_seed != table.decision_seed {
            lines.push(format!(
                "                 (the decision seed was OVERRIDDEN — {:#018x} against the \
                 entry's recorded {recorded_seed:#018x} — so this is not the entry's own \
                 reproduction check)",
                table.decision_seed,
            ));
        }
    }
    lines
}

/// One rooted run.
pub fn render_outcome(outcome: &DrivenOutcome, asked_for: &str) -> String {
    let provenance = &outcome.provenance;
    let mut lines = vec![
        "=== driven run ===".to_string(),
        format!("  root           {asked_for}   {}", root_line(&provenance.root)),
        format!(
            "  semantics      rev={}  rules={}",
            provenance.semantics_rev, provenance.rules_hash,
        ),
        format!("  master         {:#018x}", outcome.master.0),
    ];
    lines.extend(generated_lines(&provenance.root));
    lines.extend([
        String::new(),
        "  seats (as asked for, as resolved, and as the driver's own bench witnessed them):"
            .to_string(),
    ]);
    for seat in &provenance.seats {
        lines.push(format!(
            "    p{}  requested={}  echo={}  observation={}  stream={:#018x}",
            seat.seat, seat.requested, seat.echo, seat.observation, seat.agent_seed,
        ));
    }
    lines.push(String::new());
    lines.push(format!(
        "  result         winner=p{}  decisions={}  turns={}",
        outcome.result.winner.0, outcome.decisions, outcome.final_state.turn_number,
    ));
    let standings: Vec<String> = outcome
        .result
        .standings
        .iter()
        .map(|s| format!("#{} p{}={}", s.rank, s.player.0, s.breakdown.total))
        .collect();
    lines.push(format!("  standings      {}", standings.join("  ")));
    lines.push(format!(
        "  explanations   {}  (identity holds: {})",
        opt(outcome.explanations),
        outcome.explanation_identity_holds(),
    ));
    lines.push(String::new());
    lines.push("  per-seat counters (`-` means this contestant has no such quantity):".to_string());
    for seat in &outcome.seats {
        let c = &seat.counters;
        lines.push(format!(
            "    p{}  moves={}  explained={}  scored={}  root_scored={}  binds={}  starved={}  \
             ply_cap={}  free_cap={}",
            seat.seat.0,
            seat.applied_moves,
            opt(c.explained),
            opt(c.candidates_scored),
            opt(c.root_scored),
            opt(c.budget_binds),
            opt(c.budget_starved_decisions),
            opt(c.ply_cap_hits),
            opt(c.free_cap_hits),
        ));
    }
    lines.push(String::new());
    lines.push(format!("  digest         {}", outcome.digest()));
    lines.join("\n")
}

/// The world-invariance rider, rendered to append BENEATH an exact table.
///
/// **The reading is PRINTED, not left to be inferred.** The whole reason this
/// read exists is that a clairvoyant table cannot by itself distinguish misplay
/// from unavoidable uncertainty, and a report that showed five world rows
/// without saying which conclusion they license would leave exactly the gap the
/// rider was built to close.
pub fn render_world_invariance(read: &WorldInvariance) -> String {
    let mut lines = vec![
        String::new(),
        "  --- world-invariance rider (W = 5, pre-registered) ---".to_string(),
        format!(
            "  master         {:#018x}   worlds={}   (drawn from the MOVER's own view via the \
             honest chain's sampler)",
            read.master,
            read.worlds.len(),
        ),
        format!("  truth top set  {} candidate(s)", read.truth_top_set.len()),
        format!(
            "  distinct       {} of {} worlds are a DIFFERENT position from the truth",
            read.distinct_worlds,
            read.worlds.len(),
        ),
    ];
    for (index, world) in read.worlds.iter().enumerate() {
        lines.push(format!(
            "    w{:<2} {:#018x}  {}  {:>3} in top set   {}",
            index,
            world.world_seed,
            if world.differs_from_truth { "distinct" } else { "IS-TRUTH" },
            world.top_set.len(),
            if world.matches_truth { "top set matches truth" } else { "TOP SET DIFFERS" },
        ));
    }
    lines.push(String::new());
    if read.distinct_worlds == 0 {
        // The reading that is true and says nothing, called out rather than
        // dressed up as the meaningful one.
        lines.push(
            "  verdict        VACUOUS — every determinization WAS the true position, so the \
             mover's view already pinned the state and no alternative world existed to \
             disagree. This licenses no attribution either way."
                .to_string(),
        );
    } else if read.invariant {
        lines.push(
            "  verdict        INVARIANT — the optimal set is the same in the true world and \
             in every world consistent with the mover's own view, so hidden \
             information does not bear on this decision and a pick outside the set \
             is UNAMBIGUOUS MISPLAY."
                .to_string(),
        );
    } else {
        lines.push(
            "  verdict        CONFOUNDED — the optimal set MOVES with the world, so the \
             exact verdict still resolves the policy disagreement but licenses NO \
             misplay claim: a seated player could not have known which world it \
             was in."
                .to_string(),
        );
    }
    lines.join("\n")
}

/// One exact table. **Its FIRST LINE is [`driven_exact::EXACT_CLAIRVOYANT_READ`]
/// verbatim**, on [`driven::ROBUSTNESS_READ`]'s rule and for a sharper version of
/// its reason: this instrument reads PRIVILEGED information, and a table quoted
/// without its label would read as a misplay verdict it cannot support.
///
/// Both tolerances are printed. ε = 0 is the true argmax; the ε the caller asked
/// for defaults to family 1's FROZEN parameter so the two instruments' verdicts
/// are comparable on the tolerance the family actually ran at — on an exact read
/// that ε is a declared INDIFFERENCE BAND rather than noise cover.
pub fn render_exact_table(table: &ExactTable, asked_for: &str, eps_vp: i64) -> String {
    let provenance = &table.provenance;
    let mut lines = vec![
        driven_exact::EXACT_CLAIRVOYANT_READ.to_string(),
        String::new(),
        "=== driven exact ===".to_string(),
        format!("  root           {asked_for}   {}", root_line(&provenance.root)),
        format!(
            "  semantics      rev={}  rules={}",
            provenance.semantics_rev, provenance.rules_hash,
        ),
        format!("  mover          p{}", table.mover.0),
        // N1: these are what the solve REALIZED, not the bounds it ran under —
        // labelling them "bounds" invited a reader to take a node count for a
        // node budget, which are opposite kinds of number.
        format!(
            "  realized       nodes={}  depth={}  memo={}",
            table.stats.nodes, table.stats.max_depth, table.stats.memo_entries,
        ),
    ];
    lines.extend(generated_lines(&provenance.root));
    lines.push(String::new());

    let best = table.best();
    let argmax = table.argmax_set().len();
    let top = table.top_set(eps_vp).len();
    lines.push(format!(
        "  candidates     {}   optimal set (eps=0) {} of {}   top set (eps={} VP) {} of {}",
        table.rows.len(),
        argmax,
        table.rows.len(),
        eps_vp,
        top,
        table.rows.len(),
    ));
    lines.push(String::new());
    lines.push(format!(
        "  {:>3}  {:>8}  {:>6}  {:>5}  {:>12}   candidate",
        "#", "margin", "gap", "opt", "nodes",
    ));
    for (index, row) in table.rows.iter().enumerate() {
        let gap = best.map_or(0, |b| b.margin - row.value.margin);
        // Two marks, not one: `*` is the true optimum and `~` is inside the
        // declared band but not optimal. Collapsing them would hide exactly the
        // distinction the two printed set sizes exist to expose.
        let mark = if best == Some(row.value) {
            "*"
        } else if gap <= eps_vp {
            "~"
        } else {
            ""
        };
        lines.push(format!(
            "  {:>3}  {:>8}  {:>6}  {:>5}  {:>12}   {:?}",
            index + 1,
            format!("{:+}", row.value.margin),
            gap,
            mark,
            row.nodes,
            row.candidate,
        ));
    }
    lines.push(String::new());
    lines.push(format!(
        "  won at best    {}   (margin 0 is NOT a win: the official tie-break decides it)",
        best.map_or_else(|| "-".to_string(), |b| b.won.to_string()),
    ));
    lines.push(format!(
        "  memo           {} entries / {} hits   (TURN-ENTRY states only)",
        table.stats.memo_entries, table.stats.memo_hits,
    ));
    lines.join("\n")
}

/// One adjudication. **Its FIRST LINE is [`driven::ROBUSTNESS_READ`] verbatim**
/// (design §3, review N-7): the label rides the OUTPUT and not only the docs, so
/// a table pasted into a triage note carries the claim it is entitled to make.
///
/// **Its LAST line is a FOLD of the reproduction digest, not the digest** — see
/// [`digest_fold`]. `Adjudication::digest()` stays the authority a reproduction
/// compares; what this report prints is a 16-hex hash of it, because the string
/// itself is `candidates × K` long and a report has to be quotable.
pub fn render_adjudication(adjudication: &Adjudication, asked_for: &str) -> String {
    let provenance = &adjudication.provenance;
    let mut lines = vec![
        driven::ROBUSTNESS_READ.to_string(),
        String::new(),
        "=== driven adjudicate ===".to_string(),
        format!("  root           {asked_for}   {}", root_line(&provenance.root)),
        format!(
            "  semantics      rev={}  rules={}",
            provenance.semantics_rev, provenance.rules_hash,
        ),
        format!("  mover          p{}   root turn={}", adjudication.mover.0, adjudication.root_turn),
        format!(
            "  master         {:#018x}   replications={}",
            adjudication.master.0, adjudication.replications,
        ),
    ];
    lines.extend(generated_lines(&provenance.root));
    lines.push(String::new());
    lines.push("  seats:".to_string());
    for seat in &provenance.seats {
        lines.push(format!(
            "    p{}  requested={}  echo={}",
            seat.seat, seat.requested, seat.echo,
        ));
    }
    lines.push(String::new());
    lines.push(format!(
        "  candidates     {}   all candidates equal: {}   (elementwise over the {} \
         replications; vacuously true below 2 candidates, which is a FORCED position or a \
         FINISHED game rather than a dead one)",
        adjudication.candidates(),
        adjudication.all_candidates_equal(),
        adjudication.replications,
    ));
    lines.push(String::new());
    lines.push(format!(
        "  {:>3}  {:>10}  {:>8}  {:>8}  {:>8}  {:>6}   candidate / margins",
        "#", "mean", "min", "max", "wins", "k",
    ));
    for (index, row) in adjudication.rows.iter().enumerate() {
        let margins: Vec<String> = row.margins().iter().map(|m| format!("{m:+}")).collect();
        lines.push(format!(
            "  {:>3}  {:>10.2}  {:>8}  {:>8}  {:>8}  {:>6}   {:?}",
            index + 1,
            row.mean_margin(),
            row.min_margin().map_or_else(|| "-".to_string(), |m| format!("{m:+}")),
            row.max_margin().map_or_else(|| "-".to_string(), |m| format!("{m:+}")),
            format!("{}/{}", row.wins(), row.playouts.len()),
            row.playouts.len(),
            row.candidate,
        ));
        lines.push(format!(
            "       margins [{}]  continuation={}",
            margins.join(", "),
            row.continuation_digest,
        ));
    }
    lines.push(String::new());
    // The witness is the FIRST PLAYOUT's provenance, so its root descriptor
    // names that playout's own starting position — root + the first candidate —
    // and NOT the adjudicated root. Said out loud because the two are one field
    // name apart and a reader who assumed otherwise would mis-read every stream.
    match &adjudication.witness {
        Some(witness) => lines.push(format!(
            "  witness        the FIRST PLAYOUT's stamped provenance: {} (root + candidate 1, \
             not the adjudicated root; world {:#018x})",
            root_line(&witness.root),
            witness.master,
        )),
        None => lines.push(
            "  witness        none: nothing was played (a terminal root adjudicates to an empty \
             table)"
                .to_string(),
        ),
    }
    // The FOLD, not the string: see `digest_fold` for why a raw
    // `Adjudication::digest()` is unquotable on a real table.
    lines.push(format!(
        "  digest         fnv1a64:{}   (of the adjudication-v1 reproduction string)",
        digest_fold(&adjudication.digest()),
    ));
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Rendering a family block
// ---------------------------------------------------------------------------

/// What a family's LABEL claims — the sentence that has to be read before any
/// number below it is quoted.
fn label_line(label: FamilyLabel) -> String {
    match label {
        FamilyLabel::Confirmatory => "  label          CONFIRMATORY — a pass is evidence FOR the \
                                      subject and a failure routes through D7; the block \
                                      confirms a behaviour the canon (or the rules) already call \
                                      right"
            .to_string(),
        FamilyLabel::ExistenceDemonstration => {
            "  label          EXISTENCE_DEMONSTRATION — no pass bar is declared; the output is \
             the demonstrated per-root GAP, which supports a pivot or a routing and never a GO"
                .to_string()
        }
    }
}

/// A [`PassRule`] as one token, grain included.
fn rule_text(rule: &PassRule) -> String {
    match rule {
        PassRule::Turn(predicate) => format!("turn:{}", predicate.as_str()),
        PassRule::AdjudicatedBest { epsilon } => format!("adjudicated-best(epsilon={epsilon} VP)"),
    }
}

/// One root's VERDICT as a loud token plus its detail.
fn root_verdict_text(verdict: &RootVerdict) -> String {
    match verdict {
        RootVerdict::Excluded(ExclusionReason::Dead) => {
            "EXCLUDED / DEAD (no candidate won any replication and the field's mean-margin spread \
             is inside the declared bound — honest indifference, which confirms nothing and fails \
             nothing)"
                .to_string()
        }
        RootVerdict::Excluded(ExclusionReason::Forced { candidates }) => format!(
            "EXCLUDED / FORCED ({candidates} candidate(s)): a one-row table is a FORCED position — \
             it confirms nothing and fails nothing, because the mover had nothing to prefer and \
             its pick is inside every adjudicated top set by construction"
        ),
        RootVerdict::Excluded(ExclusionReason::NotDemonstrated) => {
            "EXCLUDED / NOT DEMONSTRATED (the pass behaviour does not dominate here under every \
             SPLITTING policy — a real measurement, and the rate is on the report)"
                .to_string()
        }
        RootVerdict::Excluded(ExclusionReason::TooFewSplitting { splitting, minimum }) => format!(
            "EXCLUDED / TOO FEW SPLITTING (splitting={splitting}, minimum={minimum}): fewer than \
             {minimum} of the declared policies split their playouts into two groups of two, so \
             there is no measurement here to fail — a different finding from NOT DEMONSTRATED"
        ),
        RootVerdict::CounterRefused(detail) => {
            format!("COUNTER REFUSED — a D7 branch-0 ROUTING and never a failure: {detail}")
        }
        RootVerdict::Pass => "PASS".to_string(),
        RootVerdict::Fail { misses, allowance } => {
            format!("FAIL ({misses} misses against an allowance of {allowance})")
        }
        RootVerdict::UnresolvedPendingExact { replications } => format!(
            "UNRESOLVED-pending-exact ({replications} replications landed in SOME declared \
             policy's top set and not all — the policies disagree about this position, which is \
             the exact-adjudicator trigger and never a silent fail)"
        ),
        RootVerdict::GapRecorded { misses } => {
            format!("GAP RECORDED ({misses} misses; the existence arm declares no bar)")
        }
    }
}

/// A family's VERDICT, with the VOID and EXISTENCE arms said out loud.
fn family_verdict_lines(verdict: &FamilyVerdict) -> Vec<String> {
    match verdict {
        FamilyVerdict::Pass => vec!["  verdict        PASS".to_string()],
        FamilyVerdict::Fail { failed_roots, allowance } => vec![format!(
            "  verdict        FAIL — {failed_roots} admitted roots failed against an allowance of \
             {allowance}"
        )],
        FamilyVerdict::UnresolvedPendingExact { roots } => vec![format!(
            "  verdict        UNRESOLVED-pending-exact — {roots} root(s) whose declared policies \
             disagree; the exact bounded-tree adjudicator is the named next instrument"
        )],
        FamilyVerdict::VoidAsConfirmation { admitted, drawn, minimum, disposition } => vec![
            format!(
                "  verdict        VOID AS CONFIRMATION — {admitted} admitted of {drawn} drawn, \
                 below the declared minimum of {minimum}"
            ),
            "                 THIS BLOCK CONFIRMS NOTHING, whatever its roots said: a \
             confirmatory result quoting too few admitted roots is not a confirmation"
                .to_string(),
            format!("                 disposition: {disposition}"),
        ],
        FamilyVerdict::NothingAdmitted { drawn } => vec![
            "  verdict        NOTHING ADMITTED — no reading".to_string(),
            format!(
                "                 0 of {drawn} drawn root(s) entered the denominator, so this \
                 block read NOTHING: a PASS over an empty denominator is not a pass, and the \
                 admitted-count is what a reader checks before quoting anything below"
            ),
        ],
        FamilyVerdict::GapDemonstrated { roots } => {
            let mut lines = vec![format!(
                "  verdict        GAP DEMONSTRATED over {roots} admitted root(s) — the EXISTENCE \
                 arm: no bar was declared, the failure allowance does not apply, and this supports \
                 a pivot or a routing, never a GO"
            )];
            if *roots == 0 {
                lines.push(
                    "                 over 0 admitted roots this demonstrates nothing — an \
                     existence arm with an empty denominator has no gap to show"
                        .to_string(),
                );
            }
            lines
        }
    }
}

/// One admission verdict as a loud token.
fn admission_verdict_text(verdict: AdmissionVerdict) -> String {
    match verdict {
        AdmissionVerdict::Dominates => "DOMINATES".to_string(),
        AdmissionVerdict::Fails => "FAILS".to_string(),
        AdmissionVerdict::Cannot(reason) => format!("CANNOT ({})", reason.as_str()),
    }
}

/// The per-root DETAIL block: the step-one counter table, the annotations, the
/// binomial line and every replication's own seed.
fn root_detail_lines(row: &RootRow) -> Vec<String> {
    let mut lines = vec![
        format!("    ordinal {}   {}", row.ordinal, root_line(&row.descriptor)),
        format!("        verdict        {}", root_verdict_text(&row.verdict)),
        format!(
            "        draw seed      {}",
            row.draw_seed.map_or_else(|| "-".to_string(), |seed| format!("{seed:#018x}")),
        ),
        format!(
            "        step           {}",
            row.step.map_or_else(|| "-".to_string(), |step| format!("{step:?}")),
        ),
        format!("        rule as run    {}", rule_text(&row.rule)),
        format!("        misses {}   unresolved {}", row.misses, row.unresolved),
    ];
    if let Some(rider) = row.rider {
        lines.push(format!(
            "        rider          progress_sum={}  postclaim_sum={}  race_proximity={}  \
             (RECORDED, never gated)",
            rider.progress_sum, rider.postclaim_sum, rider.race_proximity,
        ));
    }
    for (policy, dead) in &row.dead_by_policy {
        lines.push(format!(
            "        dead? {:<20} {}",
            policy.as_str(),
            if *dead { "DEAD" } else { "live" },
        ));
    }
    for (policy, set) in &row.top_sets {
        lines.push(format!("        top set {:<18} {} candidate(s)", policy.as_str(), set.len()));
    }
    let Some(read) = &row.read else {
        lines.push(
            "        read           none — this root was excluded or counter-refused before any \
             replication ran"
                .to_string(),
        );
        return lines;
    };
    let check = &read.counter_check;
    lines.push(format!(
        "        counter check  candidates={}  top={}  tied-at-top={}  top-band(<={} pt)={}  \
         distinct={}",
        check.table.candidates(),
        check.table.top_value().map_or_else(|| "-".to_string(), |v| format!("{v:+}")),
        check.tied_at_top,
        driven::TOP_BAND_POINTS,
        check.top_band,
        check.distinct_values,
    ));
    for (name, value) in counter_rows(&check.table.stats) {
        lines.push(format!("          {name:<22} {value}"));
    }
    for annotation in &read.annotations {
        lines.push(format!("        annotation     {annotation:?}"));
    }
    if read.separator_recommended() {
        lines.push(
            "        annotation     SEPARATOR RECOMMENDED — the evaluator separated little here, \
             so this root goes through b=4 / b=8 / b=16 before an argument rides it"
                .to_string(),
        );
    }
    if let Some(binomial) = read.binomial_line() {
        lines.push(format!("        binomial       {binomial}"));
    } else {
        lines.push(
            "        binomial       -  (decision grain: a miss is a fact about a TOP SET, which \
             the block's own judgment resolves)"
                .to_string(),
        );
    }
    lines.push(format!(
        "        replications   R={}  master={:#018x}  root_word={:#018x}",
        read.replication_count(),
        read.master,
        read.root_word,
    ));
    for replication in &read.replications {
        let verdict = match replication.verdict {
            Some(true) => "HELD".to_string(),
            Some(false) => "MISSED".to_string(),
            None => "-".to_string(),
        };
        lines.push(format!(
            "          r{:<3} seed={:#018x}  root_word={:#018x}  {verdict}  moves={}  \
             game_over={}  handoff={}",
            replication.replication,
            replication.decision_seed,
            replication.root_word,
            replication.turn_moves,
            replication.game_over.map_or_else(|| "-".to_string(), |over| over.to_string()),
            replication.handoff_digest.as_deref().unwrap_or("-"),
        ));
        if let Some(pick) = &replication.pick {
            lines.push(format!("               pick {pick:?}"));
        }
    }
    lines
}

/// ONE family block, rendered (design §2: label first, counter check second,
/// per-policy table, per-root replication lines, binomial line, exclusion and
/// dissent blocks).
///
/// **The ORDER is the contract.** A reader meets the LABEL before any number —
/// what a pass would even claim — and then D7's STEP-ONE counter check, because
/// a counter refusal says the preference framing does not apply to a root at
/// all. Only after both does the per-policy arithmetic appear.
///
/// `arm` is passed rather than read off the report because a
/// [`FamilyBlockReport`] records the generator it RAN and not which arm of the
/// declaration the caller asked for; the runner passes what it applied, and the
/// header prints it beside the control arm's direction sentence so a control
/// block cannot be read backwards.
pub fn render_family_block(report: &FamilyBlockReport, arm: FamilyArm) -> String {
    let annotated = report
        .roots
        .iter()
        .filter(|row| row.read.as_ref().is_some_and(|read| read.separator_recommended()))
        .count();
    let menu: Vec<&str> = report.menu.iter().map(|policy| policy.as_str()).collect();

    let mut lines = vec![
        "=== driven family ===".to_string(),
        label_line(report.label),
        format!(
            "  counter check  D7 step one — {} root(s) read, {} REFUSED, {annotated} annotated \
             (a refusal is a ROUTING and never a failure: a truncated search's pick is not a \
             preference)",
            report.admitted, report.counter_refused,
        ),
        format!(
            "  family         {}   class={}   block={}   arm={}",
            report.id,
            report.class.as_str(),
            report.block.as_str(),
            arm.as_str(),
        ),
    ];
    if arm == FamilyArm::Control {
        lines.push(
            "                 the CONTROL arm reads the family's control PASS RULE, which is the \
             NEGATED predicate where the control asks a different question — so its dominance \
             direction is the POSITIVE one and the admission inversion stays OFF: inverting both \
             would be a double negation, and the control's own claim upside down"
                .to_string(),
        );
        // The TRIPWIRE: the sentence above describes what the run SHOULD have
        // been sized as; this line fires when the report says it was not.
        if report.control {
            lines.push(format!("  {DOUBLE_NEGATION_BRAND}"));
        }
    }
    lines.extend([
        format!("  generator      {}", report.generator),
        format!("  master         {:#018x}", report.master),
        format!("  subject        {}", report.subject_echo),
        format!("  rule           {}   grain={}", rule_text(&report.rule), report.rule.grain().as_str()),
        format!(
            "  sizing         R={}  per-root F={}  admission K={}  delta={} VP  \
             admission-inverted={}",
            report.replications, report.per_root_f, report.admission_k, report.delta,
            report.control,
        ),
        format!("  menu           {}", menu.join(", ")),
        format!(
            "  counts         drawn={}  admitted={}  counter-refused={}",
            report.drawn, report.admitted, report.counter_refused,
        ),
    ]);
    lines.extend(family_verdict_lines(&report.verdict));
    lines.push(format!("  robustness     {}", report.robustness_read));

    // ---- the per-policy admission table -----------------------------------
    lines.push(String::new());
    lines.push(format!(
        "  per-policy admission (delta={} VP, inverted={}) — the TURN-grain admission arithmetic, \
         and at the DECISION grain the LIVE/DEAD split printed per root above:",
        report.delta, report.control,
    ));
    lines.push(
        "    the conjunction runs over the SPLITTING policies (both groups >= 2 playouts): a \
         CANNOT row is recorded and blocks nothing, a FAILS row blocks, and a root needs 2 \
         splitting policies before its admission is a demonstration at all"
            .to_string(),
    );
    lines.push(format!(
        "    {:>7}  {:<18}  {:>6}  {:>7}  {:>10}  {:>11}  {:>8}   verdict",
        "ordinal", "policy", "true_n", "false_n", "true_mean", "false_mean", "diff",
    ));
    let mut any = false;
    for row in &report.roots {
        let Some(admission) = &row.admission else { continue };
        for policy in &admission.policies {
            any = true;
            lines.push(format!(
                "    {:>7}  {:<18}  {:>6}  {:>7}  {:>10.2}  {:>11.2}  {:>8.2}   {}",
                row.ordinal,
                policy.policy.as_str(),
                policy.true_n,
                policy.false_n,
                policy.true_mean,
                policy.false_mean,
                policy.difference,
                admission_verdict_text(policy.verdict),
            ));
        }
    }
    if !any {
        lines.push(
            "    (none: this block reads at the DECISION grain, where admission is the LIVE/DEAD \
             split rather than a dominance demonstration)"
                .to_string(),
        );
    }

    // ---- the roots --------------------------------------------------------
    lines.push(String::new());
    lines.push("  roots:".to_string());
    for row in &report.roots {
        lines.push(String::new());
        lines.extend(root_detail_lines(row));
    }

    // ---- exclusions and dissent -------------------------------------------
    lines.push(String::new());
    lines.push("  exclusions (roots that never entered the denominator):".to_string());
    let mut excluded = false;
    for row in &report.roots {
        if let RootVerdict::Excluded(_) = row.verdict {
            excluded = true;
            lines.push(format!("    ordinal {}   {}", row.ordinal, root_verdict_text(&row.verdict)));
            if let Some(admission) = &row.admission {
                lines.push(format!(
                    "                 splitting={} of {} declared policies",
                    admission.splitting_count(),
                    admission.policies.len(),
                ));
                let dissenters: Vec<&str> =
                    admission.dissenters().iter().map(|p| p.as_str()).collect();
                if !dissenters.is_empty() {
                    lines.push(format!("                 dissenters: {}", dissenters.join(", ")));
                }
            }
        }
    }
    if !excluded {
        lines.push("    (none)".to_string());
    }

    lines.push(String::new());
    if report.sole_dissenters.is_empty() {
        lines.push(
            "  dissent        no exclusion had lookahead-at-final as its SOLE DISSENTER"
                .to_string(),
        );
    } else {
        lines.push(format!(
            "  dissent        SOLE DISSENTER at ordinals {:?} — dominance held under every \
             declared policy EXCEPT lookahead-at-final",
            report.sole_dissenters,
        ));
        lines.push(
            "                 the subject IS (provisionally) lookahead@final, so a subject defect \
             that degrades its POST-candidate play deflates measured dominance under exactly that \
             arm and would exclude exactly the roots where the defect matters — a broken subject \
             passing by shrinking its own denominator. Collected as potential subject SUPPRESSION \
             evidence and routed to D7 as a finding, never silently dropped."
                .to_string(),
        );
    }
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// The opening probe (design §2's C-D probe)
// ---------------------------------------------------------------------------

/// ONE game's opening, as the probe reads it.
///
/// The three read-out fields are AREA TYPE names and not area ids, deliberately:
/// the board's area types are dealt to positions per setup seed, so an id means
/// a different place in every game and a distribution over ids would be a
/// distribution over nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpeningGame {
    pub game_index: u32,
    /// The world this game's setup and streams descend from.
    pub world: u64,
    /// The seat that moved first.
    pub mover: PlayerId,
    /// The seed the subject's turn was taken at — the ARENA's own keying,
    /// `agent_stream_seed(world, mover, Slot(mover))`, so the probed turn is the
    /// turn that seat would have played.
    pub decision_seed: u64,
    /// `state_digest` of the opening state, which is what the setup-equivalence
    /// pin compares.
    pub opening_digest: String,
    /// The area TYPES the mover placed assistants in, sorted.
    pub placements: Vec<String>,
    /// The area TYPE the president ended the movement step on.
    pub president: Option<String>,
    /// The kind of area action executed — [`OPENING_NO_ACTION`] when the turn
    /// reached none.
    pub purchase: String,
    /// Decisions in the turn.
    pub moves: usize,
    /// Whether the first turn ENDED the game, which no opening does — recorded
    /// rather than asserted, because the probe reports what it saw.
    pub game_over: bool,
}

/// The whole probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpeningProbe {
    pub master: u64,
    pub games: u32,
    /// What the subject RESOLVED to — never the spec that was typed.
    pub subject_echo: String,
    pub rows: Vec<OpeningGame>,
}

/// One [`AreaAction`]'s KIND, as a stable name.
///
/// **Exhaustive with no `_` arm**, on [`counter_rows`]'s discipline: a new area
/// action is a compile error here, to be answered with a name, rather than a
/// silent extra row in somebody's distribution.
fn area_action_kind(action: &AreaAction) -> &'static str {
    match action {
        AreaAction::ProduceCopper => "ProduceCopper",
        AreaAction::ProduceSilk => "ProduceSilk",
        AreaAction::ProduceTea => "ProduceTea",
        AreaAction::ProduceSeafood => "ProduceSeafood",
        AreaAction::Bank => "Bank",
        AreaAction::Port { .. } => "Port",
        AreaAction::Laboratory { .. } => LAB_RUSH_KIND,
        AreaAction::EmploymentAgency { .. } => "EmploymentAgency",
        AreaAction::Chinatown { .. } => "Chinatown",
        AreaAction::Church { .. } => "Church",
        AreaAction::Customs { .. } => "Customs",
        AreaAction::Skip => "Skip",
    }
}

/// One area's TYPE name, off the state's own layout.
fn area_type_name(state: &crate::types::GameState, area: AreaId) -> String {
    state
        .board_layout
        .areas
        .get(&area)
        .map_or_else(|| format!("(unknown area {})", area.0), |config| format!("{:?}", config.area_type))
}

/// Run the C-D opening probe: `games` seeded fresh 2P games, the subject's
/// FIRST TURN played in each.
///
/// **The setup path is the PUBLIC one** ([`OPENING_SETUP_PATH`]) and it is
/// printed in the output — the Phase-B split seamed out only `play_from_state`,
/// so the arena's setup half is still interior to its private driver, and what
/// makes reusing it unnecessary is the bit-identity pin rather than a promise.
/// The worlds are `SeedSchedule::diagnostic(master)`'s, one per game, which is
/// the same convention `driven::generate_roots` draws its scan games from.
///
/// **This function COMPUTES nothing about a decision.** Every move it reports
/// came out of [`driven::drive_turn`] over the subject's own honest chain; what
/// is done here is the tabulation.
pub fn run_opening(
    master: u64,
    games: u32,
    contestant: Contestant,
    rules: &Rules,
) -> Result<OpeningProbe, String> {
    let subject = SeatSpec::Contestant(contestant);
    let mut schedule = SeedSchedule::diagnostic(master);
    let mut rows = Vec::with_capacity(games as usize);
    for game_index in 0..games {
        let world = schedule.next_world();
        let opening = build_opening(world, contestant, rules)
            .map_err(|e| format!("game {game_index}: no opening: {e}"))?;
        let mover = opening.current_player;
        let decision_seed = agent_stream_seed(world, mover, Slot(mover.0));
        let outcome = driven::drive_turn(&opening, subject, rules, decision_seed, &DrivenOpts::default())
            .map_err(|e| format!("game {game_index}: the opening turn did not drive: {e}"))?;

        let mut placements = Vec::new();
        let mut president = None;
        let mut purchase = OPENING_NO_ACTION.to_string();
        for applied in &outcome.moves {
            match &applied.mv {
                Move::PlaceThreeAreas { placements: areas } => {
                    placements.extend(areas.iter().map(|area| area_type_name(&opening, *area)));
                }
                Move::PlaceTwoSameArea { area, adjacent_bonus } => {
                    placements.push(area_type_name(&opening, *area));
                    placements.push(area_type_name(&opening, *area));
                    if let Some(bonus) = adjacent_bonus {
                        placements.push(area_type_name(&opening, *bonus));
                    }
                }
                Move::PlacePresident { destination } | Move::MovePresident { destination, .. } => {
                    president = Some(area_type_name(&opening, *destination));
                }
                Move::ExecuteAreaAction { action } => {
                    purchase = area_action_kind(action).to_string();
                }
                _ => {}
            }
        }
        placements.sort();
        rows.push(OpeningGame {
            game_index,
            world: world.0,
            mover,
            decision_seed,
            opening_digest: state_digest(&opening),
            placements,
            president,
            purchase,
            moves: outcome.moves.len(),
            game_over: outcome.game_over,
        });
    }
    Ok(OpeningProbe { master, games, subject_echo: subject.echo(), rows })
}

/// The PUBLIC setup half: `new_game`, each seat's own
/// [`ArenaAgent::choose_starting_order`] in SEAT ORDER, `finalize_setup`.
///
/// **2P, and not a parameter.** Every question this phase asks is a 2P question
/// — the subject is the retained 2P turn-boundary contestant, the v2 generating
/// ecology is 2P, and the C-D opening question is about that seat's opening. A
/// player-count flag would offer a population nothing downstream compares
/// against.
///
/// The agents are built for ALL seats before any Keep is asked, exactly as
/// `run_arena_game` builds them before its bench answers — that ordering is
/// half of what makes the two bit-identical, and the streams
/// (`agent_stream_seed(world, seat, Slot(seat))`) are the other half.
fn build_opening(
    world: WorldSeed,
    contestant: Contestant,
    rules: &Rules,
) -> Result<crate::types::GameState, String> {
    let pending = new_game(GameConfig::for_player_count(2), rules, world.0)
        .map_err(|e| format!("new_game failed: {e}"))?;
    let mut agents: Vec<Box<dyn ArenaAgent>> = (0..2u8)
        .map(|seat| {
            contestant.build(rules, agent_stream_seed(world, PlayerId(seat), Slot(seat))).0
        })
        .collect();
    let choices: Vec<StartingOrderChoice> = (0..2usize)
        .map(|seat| {
            let keep =
                agents[seat].choose_starting_order(PlayerId(seat as u8), &pending.deals[seat]);
            StartingOrderChoice::Keep(keep)
        })
        .collect();
    finalize_setup(pending, &choices).map_err(|e| format!("finalize_setup failed: {e}"))
}

/// The probe, rendered: the distribution, the lab-rush share with its Wilson
/// interval, and the setup path.
pub fn render_opening(probe: &OpeningProbe) -> String {
    let n = probe.rows.len();
    let labs = probe.rows.iter().filter(|row| row.purchase == LAB_RUSH_KIND).count();
    let wilson = wilson_95(labs as u64, n as u64);
    let share = if n == 0 { 0.0 } else { labs as f64 / n as f64 };

    let mut tally: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for row in &probe.rows {
        let key = (
            row.placements.join("+"),
            row.president.clone().unwrap_or_else(|| "-".to_string()),
            row.purchase.clone(),
        );
        *tally.entry(key).or_default() += 1;
    }

    let mut lines = vec![
        "=== driven opening ===".to_string(),
        format!("  master         {:#018x}", probe.master),
        format!("  games          {} asked for, {n} played", probe.games),
        format!("  subject        {}", probe.subject_echo),
        format!("  setup path     {OPENING_SETUP_PATH}"),
        "  turn driver    drive_turn at agent_stream_seed(world, mover, Slot(mover)) — the \
         ARENA's own keying, so the probed turn is the turn that seat would have played"
            .to_string(),
        format!(
            "  lab rush       {labs}/{n} = {share:.4}   Wilson95 [{:.4}, {:.4}]   (the first \
             turn's area action is a {LAB_RUSH_KIND})",
            wilson.lo, wilson.hi,
        ),
        String::new(),
        format!(
            "  distribution over (placement area types + president area type + first area \
             action), {} distinct of {n}:",
            tally.len(),
        ),
        format!(
            "    {:>4}  {:>7}   {:<44}  {:<18}  {}",
            "n", "share", "placements", "president", "purchase",
        ),
    ];
    for ((placements, president, purchase), count) in &tally {
        lines.push(format!(
            "    {:>4}  {:>7.4}   {placements:<44}  {president:<18}  {purchase}",
            count,
            if n == 0 { 0.0 } else { *count as f64 / n as f64 },
        ));
    }
    lines.push(String::new());
    lines.push("  games:".to_string());
    for row in &probe.rows {
        lines.push(format!(
            "    #{}  world {:#018x}  mover p{}  decision seed {:#018x}  moves {}  \
             game_over={}  opening {}",
            row.game_index,
            row.world,
            row.mover.0,
            row.decision_seed,
            row.moves,
            row.game_over,
            row.opening_digest,
        ));
        lines.push(format!(
            "        placements {}   president {}   purchase {}",
            if row.placements.is_empty() { "(none)".to_string() } else { row.placements.join("+") },
            row.president.as_deref().unwrap_or("-"),
            row.purchase,
        ));
    }
    lines.join("\n")
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
    let command = parse(args)?;
    let rules = Rules::standard();
    match command {
        Command::List => Ok(render_list(entries, anchor)),

        Command::Table { root, contestant, decision_seed } => {
            let asked = resolve(&root, entries, anchor, &rules)?;
            let seed = match decision_seed {
                Some(seed) => seed,
                None => asked.recorded_seed.ok_or_else(|| {
                    format!(
                        "{}: --decision-seed is REQUIRED for a root that records none — a \
                         <path>:<event> root, or a gen:<predicate>:<master>:<ordinal> root. No \
                         decision seed is recorded anywhere for either: the CLI's own \
                         splitmix64(bot_seed ^ event_count) derivation lives in yokohama-cli and \
                         core does not re-implement it, and a SCANNED position was never decided \
                         by anybody at all — so inventing a seed here would answer a different \
                         question from the one the root asks. A REGISTRY root takes its entry's \
                         own recorded seed by default.\n\n{USAGE}",
                        asked.asked_for,
                    )
                })?,
            };
            let (state, descriptor) = asked
                .source
                .resolve(&rules)
                .map_err(|e| format!("{}: no root: {e}", asked.asked_for))?;
            // The decision analyzed is the MOVER's. An entry that records
            // another seat as the bot there is pointed at a different decision
            // — and the loader cannot catch it, since the record does carry the
            // recorded move at that index whoever played it.
            if let Some(seat) = asked.bot_seat {
                if seat != state.current_player {
                    return Err(format!(
                        "{}: the entry records seat p{} at event {}, but p{} is on the move there \
                         — the entry points at another decision",
                        asked.asked_for,
                        seat.0,
                        asked.event_index.map_or_else(|| "-".to_string(), |k| k.to_string()),
                        state.current_player.0,
                    ));
                }
            }
            let table = driven::root_table(&state, state.current_player, contestant, &rules, seed)
                .map_err(|e| format!("{}: no root table: {e}", asked.asked_for))?;
            Ok(render_root_table(
                &table,
                &TableContext {
                    asked_for: &asked.asked_for,
                    root: &descriptor,
                    recorded_move: asked.recorded_move.as_ref(),
                    recorded_seed: asked.recorded_seed,
                },
            ))
        }

        Command::Run { root, master, seats } => {
            let asked = resolve(&root, entries, anchor, &rules)?;
            let (source, seats) = resolve_once(&asked, seats, &rules)?;
            let outcome = driven::drive_rooted(
                &source,
                &seats,
                &rules,
                WorldSeed(master),
                DrivenOpts::default(),
            )
            .map_err(|e| format!("{}: the run did not complete: {e}", asked.asked_for))?;
            Ok(render_outcome(&outcome, &asked.asked_for))
        }

        Command::Adjudicate { root, master, replications, seats } => {
            let asked = resolve(&root, entries, anchor, &rules)?;
            let (source, seats) = resolve_once(&asked, seats, &rules)?;
            let adjudication = driven::adjudicate_root(
                &source,
                &seats,
                &rules,
                WorldSeed(master),
                replications,
                DrivenOpts::default(),
            )
            .map_err(|e| format!("{}: the adjudication did not complete: {e}", asked.asked_for))?;
            Ok(render_adjudication(&adjudication, &asked.asked_for))
        }

        Command::Exact { root, bounds, eps_vp, rider } => {
            let asked = resolve(&root, entries, anchor, &rules)?;
            // The root goes in as the descriptor the resolver already produced,
            // exactly as `adjudicate` does, so the table names its origin once
            // rather than rebuilding a claim about where it came from.
            let (source, _) = resolve_once(&asked, None, &rules)?;
            let table = driven_exact::exact_table(&source, &rules, bounds)
                .map_err(|e| format!("{}: the exact solve did not complete: {e}", asked.asked_for))?;
            let mut report = render_exact_table(&table, &asked.asked_for, eps_vp);
            if let Some(master) = rider {
                let read =
                    driven_exact::world_invariance(&source, &rules, bounds, eps_vp, master)
                        .map_err(|e| {
                            format!("{}: the rider did not complete: {e}", asked.asked_for)
                        })?;
                report.push_str(&render_world_invariance(&read));
            }
            Ok(report)
        }

        Command::Family { id, block, master, arm } => {
            // `one_family` already checked the id against the declarations, so
            // this lookup cannot miss — and it is written as a lookup rather
            // than carried through the command because a declaration is a
            // VALUE and the parse's job was to name one.
            let declared = families()
                .into_iter()
                .find(|decl| decl.id == id)
                .ok_or_else(|| format!("family {id:?} is no longer declared"))?;
            let decl = match arm {
                FamilyArm::Positive => declared,
                FamilyArm::Control => control_arm(&declared)?,
            };
            // The SUBJECT is the shipped default and deliberately not a flag: a
            // family result is a statement about that subject. The sizing goes
            // through `family_block_opts` rather than through the two-argument
            // entry point so that BOTH the arm and the block reach the one seam
            // that decides them — the arm never moves the sizing (see its doc
            // for why the admission inversion stays OFF in both arms) and the
            // block always does (pilot posture vs the frozen one).
            let subject = SeatSpec::Contestant(default_contestant()?);
            match run_family_block_with(
                &decl,
                block,
                master,
                subject,
                &rules,
                family_block_opts(arm, block),
            ) {
                Ok(report) => Ok(render_family_block(&report, arm)),
                // The four COMMAND-shaped refusals carry USAGE, because each of
                // them is answered by typing something different; the RUN-shaped
                // ones do not, on `run`/`adjudicate`'s own convention.
                Err(
                    error @ (FamilyRunError::NoFrozenBlock { .. }
                    | FamilyRunError::MasterRefused { .. }
                    | FamilyRunError::MasterRequired { .. }
                    | FamilyRunError::EmptyBlock { .. }),
                ) => Err(format!("family {id} --block {}: {error}\n\n{USAGE}", block.as_str())),
                Err(error) => {
                    Err(format!("family {id} --block {}: {error}", block.as_str()))
                }
            }
        }

        Command::Opening { master, games, contestant } => {
            let probe = run_opening(master, games, contestant, &rules)?;
            Ok(render_opening(&probe))
        }

        Command::Capacity { predicate, master, stop_after } => {
            let capacity = driven::scan_capacity(predicate, master, stop_after, &rules)
                .map_err(|e| format!("capacity {}: {e}", predicate.as_str()))?;
            Ok(render_capacity(&capacity))
        }
    }
}

/// What the capacity report says it did NOT do — spec §6's "not a burn" clause,
/// on the page rather than only in the type.
pub const CAPACITY_NO_TABLE: &str =
    "NO ROOT TABLE was built or read: this walk counts strided hits and records which generated \
     game each came from. Spec §6 permits and requires capacity measurement at step 2 for exactly \
     that reason -- there is no position anywhere in this report.";

/// ONE predicate's realized ordinal supply at a master.
pub fn render_capacity(capacity: &driven::ScanCapacity) -> String {
    let ecology = &capacity.ecology;
    let bound = if capacity.truncated { "TRUNCATED at the caller's bound" } else { "EXHAUSTIVE" };
    let mut lines = vec![
        "=== driven capacity ===".to_string(),
        format!("  {CAPACITY_NO_TABLE}"),
        format!("  predicate      {}", capacity.predicate.as_str()),
        format!("  master         {:#018x}", capacity.master),
        format!(
            "  ecology        {}   games={}   per_game_cap={}",
            ecology.id, ecology.games, ecology.per_game_cap,
        ),
        format!(
            "  found          {} strided ordinal(s) in {} game(s)   [{bound}]",
            capacity.found(),
            capacity.games_walked,
        ),
    ];
    if capacity.truncated {
        lines.push(
            "  a TRUNCATED walk is a LOWER BOUND on the supply and never the supply itself"
                .to_string(),
        );
    }
    lines.push(String::new());
    lines.push("  ordinal -> generated game".to_string());
    for (ordinal, game) in capacity.game_of_ordinal.iter().enumerate() {
        lines.push(format!("      ordinal {ordinal:>3}   game {game}"));
    }
    if capacity.game_of_ordinal.is_empty() {
        lines.push("      (none)".to_string());
    }

    // The BAND verdicts, for the two banded predicates' own question. An
    // unmeasured band reads UNMEASURED and never "disjoint" -- an ordinal the
    // walk never reached is not one it found in another game.
    lines.push(String::new());
    lines.push("  band disjointness (measured, not the stride lemma)".to_string());
    for (name, band) in [
        ("development   vs escrow", driven::V4_DEVELOPMENT_ORDINALS),
        ("dev extension vs escrow", driven::V4_DEVELOPMENT_EXTENSION_ORDINALS),
    ] {
        let verdict = match capacity.bands_are_disjoint(band, driven::V4_ESCROW_ORDINALS) {
            Some(true) => "DISJOINT".to_string(),
            Some(false) => "SHARES A GAME".to_string(),
            None => "UNMEASURED (the walk did not reach every ordinal of both bands)".to_string(),
        };
        lines.push(format!("      {name}   {verdict}"));
    }
    lines.push(format!(
        "      the STRIDE alone separates development {}..{} from escrow {}..{} (gap {} >= cap {}); \
         it does NOT separate the extension's top ordinal {} from escrow's floor {}, which is what \
         the measured line above is for",
        driven::V4_DEVELOPMENT_ORDINALS.start,
        driven::V4_DEVELOPMENT_ORDINALS.end,
        driven::V4_ESCROW_ORDINALS.start,
        driven::V4_ESCROW_ORDINALS.end,
        driven::V4_ESCROW_ORDINALS.start - (driven::V4_DEVELOPMENT_ORDINALS.end - 1),
        ecology.per_game_cap,
        driven::V4_DEVELOPMENT_EXTENSION_ORDINALS.end - 1,
        driven::V4_ESCROW_ORDINALS.start,
    ));
    lines.join("\n")
}

/// A root REFERENCE turned into the material a subcommand needs: the source
/// itself, plus whatever the naming vocabulary happened to record about it.
///
/// The four `Option`s are all `Some` for a registry entry and all `None` for a
/// `<path>:<event>` or `gen:` root, and that asymmetry is the point rather than
/// an oversight — those two forms name a POSITION and nothing else, so there is
/// no recorded seed to default to, no recorded move to reproduce against, no
/// event index to quote back, and no recorded seat to check the mover against.
struct Asked {
    /// What the caller typed, for report headers and error text.
    asked_for: String,
    source: RootSource,
    /// The entry's own event index — `None` for a form that records none, and
    /// read ONLY inside the recorded-seat check, which such a form cannot
    /// reach.
    event_index: Option<usize>,
    recorded_seed: Option<u64>,
    recorded_move: Option<Move>,
    bot_seat: Option<PlayerId>,
}

fn resolve(
    root: &RootRef,
    entries: &[RegistryEntry],
    anchor: &Path,
    rules: &Rules,
) -> Result<Asked, String> {
    match root {
        RootRef::Fixture(name) => {
            let entry = entries.iter().find(|entry| entry.name == name).ok_or_else(|| {
                let known: Vec<&str> = entries.iter().map(|entry| entry.name).collect();
                format!(
                    "unknown root {name:?}: no registry entry by that name, and no ':' in it to \
                     read as <path>:<event>. Known entries: {}",
                    if known.is_empty() { "(none)".to_string() } else { known.join(", ") },
                )
            })?;
            let source = entry.load_from(anchor, rules).map_err(|e| e.to_string())?;
            Ok(Asked {
                asked_for: entry.name.to_string(),
                source,
                event_index: Some(entry.event_index),
                recorded_seed: Some(entry.decision_seed),
                recorded_move: entry.recorded_move.clone(),
                bot_seat: Some(entry.bot_seat),
            })
        }
        RootRef::Path { path, event_index } => {
            // THE SAME loader a registry entry takes, so Missing / Read / Parse
            // / Stale fire identically however the root was named. Only the
            // entry's own transcription checks stay entry-side; a path has
            // nothing to transcribe against.
            let record = driven::load_record(PATH_ROOT_NAME, path, rules)
                .map_err(|e| e.to_string())?;
            Ok(Asked {
                asked_for: format!("{}:{event_index}", path.display()),
                source: RootSource::RecordPrefix { record, event_index: *event_index },
                event_index: Some(*event_index),
                recorded_seed: None,
                recorded_move: None,
                bot_seat: None,
            })
        }
        RootRef::Generated { predicate, master, ordinal } => {
            // NOTHING is scanned here. Resolving a generated root PLAYS GAMES —
            // seconds at the family-1 rung ecology, minutes at the v2 lookahead
            // one — so it happens where the other two arms' resolution happens,
            // at `RootSource::resolve`, and the cost lands once rather than once
            // per caller of this function.
            Ok(Asked {
                asked_for: format!(
                    "{GEN_PREFIX}{}:{master:#018x}:{ordinal}",
                    predicate.as_str(),
                ),
                source: RootSource::Generated {
                    predicate: *predicate,
                    master: *master,
                    ordinal: *ordinal,
                },
                event_index: None,
                recorded_seed: None,
                recorded_move: None,
                bot_seat: None,
            })
        }
    }
}

/// The root RESOLVED ONCE, paired with the seat list the call runs under —
/// either the caller's `--seats` or [`DEFAULT_SEAT`] at every seat.
///
/// **Why the source comes back as [`RootSource::Resolved`]** (Phase C, T6 fix
/// wave, review SHOULD-1). The default seat list needs the root's PLAYER COUNT,
/// so the runner must resolve before it can build one; handing the ORIGINAL
/// source on to `drive_rooted`/`adjudicate_root` then resolves it a SECOND
/// time. For a registry or path root that is a wasted replay. For a `gen:` root
/// it is a wasted SCAN, and a scan at the v2 lookahead ecology is minutes — a
/// `run` under default seats paid it twice, once for a number it used only to
/// size a `Vec`. The T5 seam is the fix: resolve here, hand the execution call
/// the ANSWER, and [`RootSource::resolve`] returns the pair verbatim (having
/// integrity-checked that its two halves describe the same position).
///
/// **The downgrade keeps the IDENTITY.** `Constructed` would have been shorter
/// to write and would have lied: the report header names the scan that found
/// the root, so a run whose header said "constructed" would describe a root
/// nobody could reproduce. That is the whole reason `Resolved` exists rather
/// than being spelled as a state.
///
/// It runs even when `--seats` WAS typed, so "resolved once" is unconditional
/// rather than a property of the default path alone — and the seat-count
/// disagreement stays the driver's own typed `SeatMismatch`, so one authority
/// answers it.
fn resolve_once(
    asked: &Asked,
    seats: Option<Vec<SeatSpec>>,
    rules: &Rules,
) -> Result<(RootSource, Vec<SeatSpec>), String> {
    let (state, descriptor) = asked
        .source
        .resolve(rules)
        .map_err(|e| format!("{}: no root: {e}", asked.asked_for))?;
    let seats = match seats {
        Some(seats) => seats,
        None => vec![SeatSpec::Contestant(default_contestant()?); state.players.len()],
    };
    Ok((RootSource::Resolved { state, descriptor }, seats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::arena::{agent_stream_seed, run_arena_game, ArenaAgent, Slot};
    use crate::game::driven::{
        AdjudicationProvenance, AdjudicationSeat, CandidatePlayout, CandidateRow, DrivenOpts,
        RootOrigin, RootSource, SampledWorld, SeatCounters, GENERATED_ECOLOGY_V1,
        SESSION_BOT_SEAT, V4_BANDED_PREDICATES,
    };
    use crate::game::driven_families::{
        AdmissionRead, AdmissionVerdict, CounterAnnotation, CounterCheck, ExclusionReason,
        FamilyBlockReport, FamilyLabel, FamilyReadOutcome, FamilyVerdict, PolicyAdmission,
        ReadGrain, ReplicationRead, RootRow, RootVerdict, StrategicClass, FROZEN_ADMISSION_K,
        MIN_ADMITTED_ROOTS, PILOT_ADMISSION_K, VOID_DISPOSITION,
    };
    use crate::game::legal_moves::get_legal_moves;
    use crate::game::lookahead::RootScore;
    use crate::game::metrics::wilson_95;
    use crate::game::policy::RolloutPolicy;
    use crate::game::record::{state_at, GameRecord, RecordedEvent};
    use crate::game::rules::Rules;
    use crate::game::seeds::{SeedSchedule, WorldSeed};
    use crate::game::semantics::{state_digest, SEMANTICS_REV};
    use crate::types::{
        BuildingSite, EndGameTrigger, GameConfig, GameState, MercenaryState, PlayerId,
        PresidentLocation, TradingHouseState, TurnPhase,
    };

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
                    // The runner renders tables; no turn predicate rides one, so
                    // the counter-absence reading is the honest fixture value.
                    turn_verdict: None,
                })
                .collect(),
        }
    }

    // =======================================================================
    // USAGE and the grammar
    // =======================================================================

    // =======================================================================
    // `exact` — the exact bounded-tree adjudicator's runner half
    // =======================================================================

    /// `exact` parses a root and its four bounds flags.
    #[test]
    fn exact_parses_a_root_and_its_bounds() {
        let args = args(&[
            "exact",
            "gen:final-round-already-triggered:0xA3E1202608180004:9",
            "--max-nodes",
            "1234",
            "--eps",
            "2",
        ]);
        let Command::Exact { root, bounds, eps_vp, .. } = parse(&args).expect("parses") else {
            panic!("wrong command")
        };
        assert!(matches!(root, RootRef::Generated { .. }));
        assert_eq!(bounds.max_nodes, 1234);
        assert_eq!(eps_vp, 2);
    }

    /// The default ε is the FROZEN family parameter, 2 VP.
    ///
    /// Not a fresh choice: the exact read inherits family 1's declared tolerance
    /// so the six recorded FAILs are re-read on the tolerance they failed under
    /// (spec §7.3). On an exact read it is an INDIFFERENCE BAND, not noise cover.
    #[test]
    fn exact_defaults_epsilon_to_the_frozen_family_parameter() {
        let bare = args(&["exact", "gen:final-round-already-triggered:0xA3E1202608180004:9"]);
        let Command::Exact { eps_vp, .. } = parse(&bare).expect("parses") else {
            panic!("wrong command")
        };
        assert_eq!(eps_vp, FROZEN_EPSILON_VP);
    }

    /// `--master` is REFUSED with the reason, not with "unknown flag".
    ///
    /// The exact solve draws no worlds, so accepting a master would advertise a
    /// dependence it does not have — and a bare "unknown flag" would leave a
    /// caller thinking they had merely mistyped it.
    #[test]
    fn exact_refuses_a_master_and_says_why() {
        let args = args(&[
            "exact",
            "gen:final-round-already-triggered:0xA3E1202608180004:9",
            "--master",
            "0x1",
        ]);
        let err = parse(&args).expect_err("--master must be refused");
        assert!(err.contains("draws no"), "the refusal must say why: {err}");
        assert!(err.contains("--master"), "the refusal must name the flag: {err}");
    }

    /// The FIRST line of an exact report is the epistemic label, verbatim.
    ///
    /// `driven::ROBUSTNESS_READ`'s rule, one instrument over: the label rides the
    /// OUTPUT and not only the docs, so a table pasted into a triage note carries
    /// the claim it is entitled to make — and, here, the claim it is NOT.
    #[test]
    fn the_first_line_of_an_exact_report_is_the_clairvoyance_label() {
        let rules = Rules::standard();
        let table = exact_fixture_table(&rules);
        let report = render_exact_table(&table, "constructed", FROZEN_EPSILON_VP);
        let lines: Vec<&str> = report.lines().collect();
        assert_eq!(lines[0], driven_exact::EXACT_CLAIRVOYANT_READ, "the first line is not the label");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "=== driven exact ===");
    }

    /// The report names the optimal set at BOTH tolerances and marks the optimal
    /// rows, so a reader never has to recompute the argmax from the values.
    #[test]
    fn the_exact_report_prints_both_top_sets_and_marks_the_optimum() {
        let rules = Rules::standard();
        let table = exact_fixture_table(&rules);
        let report = render_exact_table(&table, "constructed", FROZEN_EPSILON_VP);
        assert!(report.contains("optimal set (eps=0)"), "{report}");
        assert!(report.contains(&format!("top set (eps={FROZEN_EPSILON_VP} VP)")), "{report}");
        assert!(report.contains("  *  "), "the optimal rows are not marked:\n{report}");
        assert!(report.contains("nodes"), "{report}");
    }

    /// A refusal is rendered as a refusal — never as an empty or partial table.
    #[test]
    fn an_out_of_scope_root_renders_a_named_refusal() {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 0x91)
            .expect("new_game builds a 2P setup");
        let midgame = finalize_setup(
            pending,
            &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
        )
        .expect("the setup finalizes");
        let err = driven_exact::exact_table(
            &RootSource::Constructed(midgame),
            &rules,
            driven_exact::ExactBounds::default(),
        )
        .expect_err("a midgame root refuses");
        assert!(err.to_string().contains("midgame root"), "{err}");
    }

    /// A tiny in-final-round table, for the render cells.
    fn exact_fixture_table(rules: &Rules) -> driven_exact::ExactTable {
        driven_exact::exact_table(
            &exact_fixture_source(rules),
            rules,
            driven_exact::ExactBounds::default(),
        )
        .expect("the fixture solves")
    }

    /// The same fixture as a source, so the rider and the table read the SAME
    /// position rather than two independently doctored ones.
    fn exact_fixture_source(rules: &Rules) -> RootSource {
        let pending = new_game(GameConfig::for_player_count(2), rules, 0x92)
            .expect("new_game builds a 2P setup");
        let mut state = finalize_setup(
            pending,
            &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
        )
        .expect("the setup finalizes");
        let n = state.config.player_count;
        let start = state.config.start_player.0;
        let mover = PlayerId::new((start + n - 1) % n);
        for p in state.players.iter_mut() {
            p.money = 0;
            p.copper = 0;
            p.silk = 0;
            p.tea = 0;
            p.seafood = 0;
            p.imported_goods = 0;
            p.mercenaries = MercenaryState::new();
            p.victory_points = if p.id == mover { 20 } else { 18 };
        }
        let area = *state
            .board_layout
            .building_sites
            .keys()
            .next()
            .expect("the 2P board carries a building site");
        state.board_layout.building_sites.insert(area, BuildingSite::new([1, 5, 2, 3], 0));
        {
            let m = state.player_mut(mover);
            m.president_location = PresidentLocation::OnBoard(area);
            m.trading_houses = TradingHouseState::new();
        }
        state.current_player = mover;
        state.turn_phase = TurnPhase::Building;
        state.current_action_power = Some(4);
        state.is_mercenary_action = false;
        state.mercenary_target_area = None;
        state.president_start_area = None;
        state.pending_choices.clear();
        state.end_game_triggered = true;
        state.final_round = true;
        state.trigger_reason = Some(EndGameTrigger::ChurchTrackFull);
        state.trigger_player = Some(state.config.start_player);
        RootSource::Constructed(state)
    }

    /// `--rider <master>` turns the world-invariance read on and carries its
    /// recorded master.
    ///
    /// A master IS accepted here and nowhere else in `exact`, which is exactly
    /// the spec's single carve-out: the SOLVE draws no worlds, the RIDER does.
    #[test]
    fn exact_takes_a_rider_master_even_though_the_solve_takes_none() {
        let with_rider = args(&[
            "exact",
            "gen:final-round-already-triggered:0xA3E1202608180004:9",
            "--rider",
            "0x83937F67C176F538",
        ]);
        let Command::Exact { rider, .. } = parse(&with_rider).expect("parses") else {
            panic!("wrong command")
        };
        assert_eq!(rider, Some(0x8393_7F67_C176_F538));

        let bare = args(&["exact", "gen:final-round-already-triggered:0xA3E1202608180004:9"]);
        let Command::Exact { rider, .. } = parse(&bare).expect("parses") else {
            panic!("wrong command")
        };
        assert_eq!(rider, None, "the rider is OFF unless a master is supplied");
    }

    /// The rider's report names its master, its five worlds, and the verdict the
    /// attribution rests on.
    #[test]
    fn the_rider_report_names_its_master_and_its_verdict() {
        let rules = Rules::standard();
        let table = exact_fixture_table(&rules);
        let read = driven_exact::world_invariance(
            &exact_fixture_source(&rules),
            &rules,
            driven_exact::ExactBounds::default(),
            FROZEN_EPSILON_VP,
            0x8393_7F67_C176_F538,
        )
        .expect("the rider runs");
        let report = render_exact_table(&table, "constructed", FROZEN_EPSILON_VP)
            + &render_world_invariance(&read);
        assert!(report.contains("0x83937f67c176f538"), "the master must be printed: {report}");
        assert!(report.contains("world-invariance"), "{report}");
        assert!(report.contains("INVARIANT") || report.contains("CONFOUNDED"), "{report}");
        assert!(report.contains("misplay"), "the reading must be printed, not implied: {report}");
    }

    /// The rider's VERDICT sentences read as sentences.
    ///
    /// A formatting guard with a real scar behind it: those sentences were first
    /// written as multi-line literals whose continuations collapsed into runs of
    /// fourteen spaces, and the defect reached a real run's output before anyone
    /// read it. Scoped to the verdict lines rather than the whole report,
    /// because the tables' column gutters are deliberate and a blanket rule
    /// would fail on them — which is itself the reason to scope rather than to
    /// loosen the threshold until the bug slips back through.
    #[test]
    fn the_rider_verdict_reads_as_a_sentence() {
        let rules = Rules::standard();
        let read = driven_exact::world_invariance(
            &exact_fixture_source(&rules),
            &rules,
            driven_exact::ExactBounds::default(),
            FROZEN_EPSILON_VP,
            0x8393_7F67_C176_F538,
        )
        .expect("the rider runs");
        let report = render_world_invariance(&read);
        let verdict = report
            .lines()
            .find(|l| l.starts_with("  verdict "))
            .expect("the rider always prints a verdict");
        let prose = verdict.trim_start_matches("  verdict ").trim_start();
        assert!(
            !prose.contains("  "),
            "the verdict sentence carries a run of 2+ spaces mid-prose (a collapsed \
             line-continuation, the defect this cell exists for):\n{verdict}"
        );
        assert!(
            prose.to_lowercase().contains("misplay"),
            "the verdict must state the reading it licenses, not leave it implied: {verdict}"
        );
    }

    /// USAGE names EVERY subcommand and EVERY flag the parser accepts — read off
    /// the parser's own whitelists, so a flag added without a line here fails
    /// rather than shipping undocumented.
    #[test]
    fn the_usage_names_every_subcommand_and_every_flag() {
        for subcommand in SUBCOMMANDS {
            assert!(USAGE.contains(subcommand), "USAGE does not name `{subcommand}`");
        }
        for flags in
        [
            LIST_FLAGS,
            TABLE_FLAGS,
            RUN_FLAGS,
            ADJUDICATE_FLAGS,
            EXACT_FLAGS,
            FAMILY_FLAGS,
            OPENING_FLAGS,
            CAPACITY_FLAGS,
        ]
        {
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
        // The two rules a caller cannot infer from a flag list: WHICH arms have
        // a root table at all (every other one is a typed refusal, pinned by
        // `mirror_build`'s own arms), and what a repeated flag does here —
        // which is NOT what `eval_probe` does.
        assert!(USAGE.contains("root table"), "USAGE does not say which arms have a root table");
        assert!(
            USAGE.contains("REPEATED flag"),
            "USAGE does not state the repeated-flag-is-an-error rule",
        );
        assert!(
            USAGE.contains("last-wins"),
            "USAGE does not name the eval_probe rule it departs from",
        );
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

    /// A flag without its value refuses — and the honest statement of WHICH
    /// refusal, since the parser does NOT special-case a flag-shaped value.
    ///
    /// Two different failures, both refusals, and the sentences differ because
    /// the failures do: a flag at the END of the line has no next token at all
    /// and refuses for want of a value, while a flag followed by another flag
    /// CONSUMES that next token whatever it looks like and refuses on the VALUE.
    /// What neither one is, is a silent default — which is the property that
    /// matters and the only one this cell is entitled to claim.
    #[test]
    fn a_flag_without_its_value_refuses() {
        let err = parse(&args(&["run", "g", "--master"]))
            .expect_err("a valueless --master must refuse");
        assert!(err.contains("master"), "the refusal hides the flag: {err}");
        assert!(err.contains("needs a value"), "the refusal is not about the absence: {err}");

        // The next token is consumed flag-shaped or not, so THIS one refuses on
        // the value it read rather than on a missing one.
        let err = parse(&args(&["run", "g", "--master", "--seats"]))
            .expect_err("a flag-shaped value must still refuse");
        assert!(err.contains("--seats"), "the refusal hides the value it read: {err}");
        assert!(err.contains("not a u64"), "the refusal is not about the VALUE: {err}");
    }

    /// A REPEATED flag is an ERROR rather than last-wins — the departure from
    /// `eval_probe` that USAGE states, pinned so the statement stays true.
    #[test]
    fn a_repeated_flag_refuses_rather_than_taking_the_last_value() {
        let err = parse(&args(&["run", "g", "--master", "1", "--master", "2"]))
            .expect_err("a repeated --master must refuse");
        assert!(err.contains("master"), "the refusal hides the flag: {err}");
        assert!(err.contains("twice"), "the refusal does not say it was repeated: {err}");
        // ANTI-VACUITY: the same line with ONE --master parses, so the refusal
        // above is about the repetition and not about the line.
        parse(&args(&["run", "g", "--master", "1"])).expect("one --master parses");
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

    /// The ranked-head BLOCK of a rendered table: the lines between the
    /// `ranked head` header and the blank line that closes it.
    fn ranked_head_rows(rendered: &str) -> Vec<&str> {
        rendered
            .lines()
            .skip_while(|line| !line.contains("ranked head"))
            .skip(1)
            .take_while(|line| !line.trim().is_empty())
            .collect()
    }

    /// The ranked head is BEST FIRST and its first row is THE WINNER.
    ///
    /// The two facts a reader takes off that block without being told, and
    /// neither is checked by a `contains` of a move name: a head rendered in
    /// reverse still contains every move, every value and every margin, and
    /// still prints a winner line above it — it just answers the separator's
    /// question upside down, which is the whole reading the block exists for.
    #[test]
    fn the_ranked_head_leads_with_the_winner_and_its_margins_never_rise() {
        let winner = Move::BuildShophouse { slot: 0 };
        let table = table_with(
            winner.clone(),
            &[
                (winner.clone(), 116),
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
        let head = ranked_head_rows(&rendered);
        assert_eq!(head.len(), table.ranked.len(), "the head is not the ranking:\n{rendered}");

        // ROW 1 IS THE WINNER — the move the table's own `winner` field names,
        // not merely a move that appears somewhere in the block.
        assert!(
            head[0].ends_with(&format!("{:?}", table.winner)),
            "row 1 is not the winner {:?}: {}",
            table.winner,
            head[0],
        );

        // …and the printed MARGINS never rise going down (best first).
        let margins: Vec<i64> = head
            .iter()
            .map(|row| {
                let token = row.split_whitespace().nth(2).unwrap_or_else(|| {
                    panic!("a head row has no margin column: {row}")
                });
                token.parse().unwrap_or_else(|e| panic!("margin {token:?} does not parse: {e}"))
            })
            .collect();
        assert!(
            margins.windows(2).all(|pair| pair[0] >= pair[1]),
            "the head is not best-first: {margins:?}",
        );
        // ANTI-VACUITY: a CONSTANT margin vector is non-increasing too, so the
        // fixture has to spread — and the spread is what a reversal inverts.
        assert!(
            margins.first() > margins.last(),
            "the fixture does not discriminate direction: {margins:?}",
        );
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
        // The reproduction digest rides the report as a FOLD of itself (N-5) —
        // the string is `candidates × K` long and the report has to be
        // quotable. `digest_fold`'s own cell pins the fold's properties.
        assert!(
            rendered.contains(&digest_fold(&equal.digest())),
            "no reproduction digest: {rendered}",
        );
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

    /// The report's `digest` line, which there is exactly one of.
    fn digest_line(rendered: &str) -> &str {
        let mut found = rendered.lines().filter(|line| line.trim_start().starts_with("digest"));
        let line = found.next().unwrap_or_else(|| panic!("no digest line:\n{rendered}"));
        assert_eq!(found.next(), None, "more than one digest line:\n{rendered}");
        line
    }

    /// The adjudication's digest line is a SHORT, STABLE, LABELLED fold that
    /// still moves with the table (N-5).
    ///
    /// The line used to print `Adjudication::digest()` raw, which is
    /// `candidates × K` long — about 15 KB on a real 139-candidate
    /// two-replication run, i.e. a reproduction check nobody can quote and
    /// therefore nobody performs. Four properties make the fold a replacement
    /// rather than a deletion: it is ONE line, it is short enough to paste, it
    /// reproduces, and it CHANGES when the table does.
    #[test]
    fn the_adjudication_digest_line_is_a_short_stable_fold_that_moves_with_a_margin() {
        let adjudication = adjudication_with(vec![
            row(Move::BuildShophouse { slot: 0 }, &[3, 3]),
            row(Move::BuildShophouse { slot: 1 }, &[1, 5]),
        ]);
        let rendered = render_adjudication(&adjudication, "fixture");
        let line = digest_line(&rendered);

        assert!(line.len() < 100, "the digest line is {} chars, not quotable: {line}", line.len());
        // It SAYS what it is: a hash, and of what.
        assert!(line.contains("fnv1a64:"), "the line does not label its fold: {line}");
        assert!(
            line.contains("reproduction string"),
            "the line does not say what was folded: {line}",
        );
        assert!(
            line.contains(&digest_fold(&adjudication.digest())),
            "the line is not the fold of THIS table's digest: {line}",
        );
        // The raw string is GONE, and it is genuinely the longer thing — so the
        // shortening is a shortening rather than a coincidence of this fixture.
        let full = adjudication.digest();
        assert!(!line.contains(&full), "the full digest is still printed: {line}");
        assert!(full.len() > line.len(), "anti-vacuity: the raw digest is not the longer form");

        // It reproduces across two renders…
        assert_eq!(line, digest_line(&render_adjudication(&adjudication, "fixture")));

        // …and it MOVES when one replication's margin moves. A digest line that
        // did not would be a decoration rather than a reproduction check.
        let moved = adjudication_with(vec![
            row(Move::BuildShophouse { slot: 0 }, &[3, 3]),
            row(Move::BuildShophouse { slot: 1 }, &[1, 6]),
        ]);
        assert_ne!(
            line,
            digest_line(&render_adjudication(&moved, "fixture")),
            "one flipped margin left the digest line unmoved",
        );
    }

    /// The adjudication table's rows as `(row line, margins line)` pairs —
    /// which also pins that a row IS two lines, since the second one carries
    /// the margin vector the first one summarizes.
    fn adjudication_rows(rendered: &str) -> Vec<(&str, &str)> {
        let mut lines = rendered
            .lines()
            .skip_while(|line| !line.contains("candidate / margins"))
            .skip(1);
        let mut rows = Vec::new();
        while let Some(row_line) = lines.next() {
            if row_line.trim().is_empty() {
                break;
            }
            let margins_line = lines
                .next()
                .unwrap_or_else(|| panic!("a row line has no margins line: {row_line}"));
            assert!(
                margins_line.trim_start().starts_with("margins ["),
                "a row's second line is not its margins: {margins_line}",
            );
            rows.push((row_line, margins_line));
        }
        rows
    }

    /// Every printed `min` sits at or below its own row's `max`, and BOTH are
    /// margins that row actually recorded.
    ///
    /// The two columns are adjacent, same width, same `{:+}` formatting and one
    /// field name apart — a transposition compiles clean and reads plausible,
    /// and every mean/wins figure around it stays right. What catches it is the
    /// ORDER relation plus membership in the row's own margin list; a
    /// same-margin row cannot catch it at all, which is why the fixture below
    /// spreads and why the cell asserts that it does.
    #[test]
    fn every_adjudication_row_prints_a_min_at_or_below_its_max_and_both_are_margins() {
        let adjudication = adjudication_with(vec![
            row(Move::BuildShophouse { slot: 0 }, &[3, 3]),
            row(Move::BuildShophouse { slot: 1 }, &[1, 5]),
            row(Move::BuildShophouse { slot: 2 }, &[-4, 2]),
        ]);
        let rendered = render_adjudication(&adjudication, "fixture");
        let rows = adjudication_rows(&rendered);
        assert_eq!(rows.len(), adjudication.rows.len(), "not one row per candidate:\n{rendered}");

        let mut spread = 0;
        for (index, (row_line, margins_line)) in rows.iter().enumerate() {
            let cell = |position: usize| -> i64 {
                let token = row_line.split_whitespace().nth(position).unwrap_or_else(|| {
                    panic!("row {index} has no column {position}: {row_line}")
                });
                token
                    .parse()
                    .unwrap_or_else(|e| panic!("row {index} column {position} {token:?}: {e}"))
            };
            // Columns: #, mean, MIN, MAX, wins, k.
            let (min, max) = (cell(2), cell(3));
            let printed: Vec<i64> = margins_line
                .split_once('[')
                .and_then(|(_, tail)| tail.split_once(']'))
                .unwrap_or_else(|| panic!("row {index} has no margin list: {margins_line}"))
                .0
                .split(',')
                .map(|m| {
                    m.trim().parse().unwrap_or_else(|e| panic!("margin {m:?} in row {index}: {e}"))
                })
                .collect();

            assert_eq!(printed, adjudication.rows[index].margins(), "row {index}'s margins");
            assert!(min <= max, "row {index} prints min {min} above max {max}: {row_line}");
            assert!(printed.contains(&min), "row {index}'s min {min} is not one of {printed:?}");
            assert!(printed.contains(&max), "row {index}'s max {max} is not one of {printed:?}");
            if min != max {
                spread += 1;
            }
        }
        // ANTI-VACUITY: a row whose margins are all equal satisfies min <= max
        // and both-are-margins under a TRANSPOSED render too.
        assert!(spread > 0, "no row spreads, so the fixture cannot see a transposition");
    }

    /// The `list` line that NAMES one entry — the line its presence marker is
    /// printed on.
    ///
    /// A per-entry assertion has to FIND this line rather than search the
    /// document, and the reason is concrete: the report's own header sentence
    /// carries the word MISSING ("a MISSING record is a MISS and never a
    /// failure"), so `rendered.contains("MISSING")` is satisfied by a render
    /// that marked every entry PRESENT — including one that inverted the test
    /// outright.
    fn entry_line<'a>(rendered: &'a str, name: &str) -> &'a str {
        let mut named =
            rendered.lines().filter(|line| line.trim_start().starts_with(name));
        let line = named
            .next()
            .unwrap_or_else(|| panic!("no line names `{name}`:\n{rendered}"));
        assert_eq!(named.next(), None, "more than one line names `{name}`:\n{rendered}");
        line
    }

    /// The `list` report marks EACH record PRESENT or MISSING on that entry's
    /// own line against the anchor it was given, and NEVER errors on absence.
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
        // PER ENTRY, on the bracketed marker: the present record's own line says
        // PRESENT and only PRESENT, the absent one's says MISSING and only
        // MISSING. An inverted marker passes every document-level `contains`
        // and fails here.
        let present = entry_line(&rendered, "present-one");
        assert!(present.contains("[PRESENT]"), "the present record is not marked: {present}");
        assert!(!present.contains("[MISSING]"), "the present record is also marked absent: {present}");
        let absent = entry_line(&rendered, "absent-one");
        assert!(absent.contains("[MISSING]"), "the absent record is not marked: {absent}");
        assert!(!absent.contains("[PRESENT]"), "the absent record is also marked present: {absent}");

        assert!(rendered.contains("a cell-owned fixture"), "the note is dropped: {rendered}");
        assert!(rendered.contains("here/present.json"), "the path is dropped: {rendered}");
        assert!(
            rendered.contains(&format!("{FIXTURE_SEED:#018x}")),
            "the recorded seed is dropped: {rendered}",
        );

        // Through the command, with an anchor that holds NOTHING: still Ok, and
        // NO line claims a present record. Stated as "no line" rather than as
        // "the report says MISSING" for the same vacuity reason.
        let empty = TempRoot::make("list-empty");
        let out = run_with(&args(&["list"]), &entries, &empty.0).expect("list never errors");
        assert!(
            !out.lines().any(|line| line.contains("[PRESENT]")),
            "an anchor holding nothing marked a record PRESENT: {out}",
        );
        for name in ["present-one", "absent-one"] {
            let line = entry_line(&out, name);
            assert!(line.contains("[MISSING]"), "`{name}` is not marked absent: {line}");
        }
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

    /// One `key=value` line, split into its fields.
    ///
    /// Exact KEYS rather than substrings, because the counter line carries both
    /// `scored=` and `root_scored=`: a `contains("scored=3")` check is satisfied
    /// by the wrong field, and a render that swapped the two would pass it.
    fn fields(line: &str) -> BTreeMap<&str, &str> {
        line.split_whitespace().filter_map(|token| token.split_once('=')).collect()
    }

    /// The run report's per-seat counter line says WHAT THE OUTCOME SAYS, field
    /// by field — and for a LOOKAHEAD seat the explained and scored fields are
    /// NUMERIC, never the `-` the legend reserves for "this contestant has no
    /// such quantity".
    ///
    /// Seated with the shipped lookahead-v1 deliberately: every other run cell
    /// in this file is rung-seated (design §6's wall discipline), and a rung
    /// prints `-` at all seven of these legitimately — so a render that hard-
    /// coded the dash, or read a different seat's counters, would pass every one
    /// of them. The root is near the end of a played game so a debug cell can
    /// afford two uncapped searches.
    #[test]
    fn the_run_report_counter_line_agrees_with_a_lookahead_seats_counters() {
        let rules = Rules::standard();
        let record = rung_record(&rules, world(9));
        let event_index = record.events.len() - 4;
        let source = RootSource::RecordPrefix { record, event_index };
        let (state, _) = source.resolve(&rules).expect("the prefix resolves");
        let seats =
            vec![SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT); state.players.len()];
        let outcome = driven::drive_rooted(
            &source,
            &seats,
            &rules,
            world(10),
            DrivenOpts::default(),
        )
        .expect("the tail run completes");
        let rendered = render_outcome(&outcome, "constructed");

        let mut scored_anywhere = 0;
        for seat in &outcome.seats {
            let prefix = format!("p{}  moves=", seat.seat.0);
            let line = rendered
                .lines()
                .find(|line| line.trim_start().starts_with(&prefix))
                .unwrap_or_else(|| panic!("no counter line for p{}:\n{rendered}", seat.seat.0));
            let printed = fields(line);
            let field = |name: &str| -> &str {
                printed
                    .get(name)
                    .unwrap_or_else(|| panic!("the counter line has no `{name}`: {line}"))
            };
            let counters = &seat.counters;

            // A lookahead seat HAS an explanation path and a scored budget. Said
            // first, because "never `-`" is only a claim about the RENDER once
            // the seat is known to have the quantity.
            assert!(
                counters.explained.is_some() && counters.candidates_scored.is_some(),
                "p{} is not an explaining, scoring seat — this cell is not seated as it claims",
                seat.seat.0,
            );

            // Every counter, as the legend says it renders: the value where the
            // seat has the quantity, `-` where it does not.
            for (name, counter) in [
                ("explained", counters.explained),
                ("scored", counters.candidates_scored),
                ("root_scored", counters.root_scored),
                ("binds", counters.budget_binds),
                ("starved", counters.budget_starved_decisions),
                ("ply_cap", counters.ply_cap_hits),
                ("free_cap", counters.free_cap_hits),
            ] {
                assert_eq!(
                    field(name),
                    counter.map_or_else(|| "-".to_string(), |value| value.to_string()),
                    "p{}'s `{name}` field disagrees with its counters: {line}",
                    seat.seat.0,
                );
            }
            assert_eq!(
                field("moves"),
                seat.applied_moves.to_string(),
                "p{}'s applied moves disagree: {line}",
                seat.seat.0,
            );
            assert_ne!(field("explained"), "-", "a lookahead seat's explained read absent: {line}");
            assert_ne!(field("scored"), "-", "a lookahead seat's scored read absent: {line}");

            scored_anywhere += counters.candidates_scored.unwrap_or(0);
        }
        // ANTI-VACUITY: a run where nobody searched would satisfy every equality
        // above with a line of zeros.
        assert!(scored_anywhere > 0, "no seat scored a candidate: {rendered}");
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

    // =======================================================================
    // Phase C, T6 — the GENERATED root spec (`gen:<predicate>:<master>:<ordinal>`)
    // =======================================================================

    /// The master the two SCANNING cells below run at. Diagnostic by the
    /// constructor's own name, per `game::seeds` — a fixture cell wants
    /// reproduction, which is the case spec §3 carves out.
    const GEN_MASTER: u64 = 0x0C06_2026_0818_9994;

    /// The master the OPENING cells run at, same waiver.
    const OPENING_MASTER: u64 = 0x0C06_2026_0818_9993;

    /// Every `RootPredicate` name parses as a `gen:` root, at either master
    /// spelling, and the parse is a ROUND TRIP: what came back names the
    /// predicate that was typed.
    #[test]
    fn a_gen_root_parses_every_predicate_name_at_either_master_spelling() {
        for predicate in RootPredicate::ALL {
            let spec = format!("gen:{}:0x0A3E_0000_0000_0001:7", predicate.as_str());
            assert_eq!(
                parse_root_ref(&spec).unwrap_or_else(|e| panic!("{spec} must parse: {e}")),
                RootRef::Generated { predicate, master: 0x0A3E_0000_0000_0001, ordinal: 7 },
            );
        }
        // A decimal master and ordinal 0 — the other end of both fields.
        assert_eq!(
            parse_root_ref("gen:final-round-already-triggered:12345:0").expect("decimal parses"),
            RootRef::Generated {
                predicate: RootPredicate::FinalRoundAlreadyTriggered,
                master: 12_345,
                ordinal: 0,
            },
        );
    }

    /// An unknown predicate refuses naming the WHOLE valid set — a caller who
    /// mistyped one gets the vocabulary, not a shrug.
    #[test]
    fn a_gen_root_with_an_unknown_predicate_names_every_valid_name() {
        let err = parse_root_ref("gen:final-round:0x1:0").expect_err("an unknown predicate refuses");
        assert!(err.contains("final-round"), "the refusal hides what was typed: {err}");
        for predicate in RootPredicate::ALL {
            assert!(
                err.contains(predicate.as_str()),
                "the refusal does not name `{}`: {err}",
                predicate.as_str(),
            );
        }
        assert!(err.contains(USAGE), "the refusal drops USAGE: {err}");
    }

    /// Each MALFORMED field refuses on its own terms — arity, master, ordinal,
    /// empty name — and every refusal carries USAGE.
    #[test]
    fn a_malformed_gen_root_refuses_naming_the_field_that_is_wrong() {
        let cases: [(&str, &str); 5] = [
            // Too few fields.
            ("gen:final-round-already-triggered:0x1", "gen:<predicate>"),
            // Too many.
            ("gen:final-round-already-triggered:0x1:0:9", "gen:<predicate>"),
            // A master that is not a number.
            ("gen:final-round-already-triggered:0xzz:0", "0xzz"),
            // An ordinal that is not a decimal u32.
            ("gen:final-round-already-triggered:0x1:-1", "-1"),
            // An empty predicate name.
            ("gen::0x1:0", "\"\""),
        ];
        for (spec, needle) in cases {
            let err = parse_root_ref(spec).unwrap_err();
            assert!(err.contains(needle), "{spec}: the refusal hides {needle}: {err}");
            assert!(err.contains(USAGE), "{spec}: the refusal drops USAGE: {err}");
        }
    }

    /// The `gen:` form reaches EVERY subcommand that takes a root — the
    /// separator and the adjudicator become runnable on generated roots, which
    /// is what the grammar is for.
    #[test]
    fn every_root_taking_subcommand_accepts_a_gen_root() {
        let spec = "gen:achievement-claim-available:0x0A3E_0000_0000_0002:3";
        let expected = RootRef::Generated {
            predicate: RootPredicate::AchievementClaimAvailable,
            master: 0x0A3E_0000_0000_0002,
            ordinal: 3,
        };
        match parse(&args(&["table", spec, "--decision-seed", "9"])).expect("table parses") {
            Command::Table { root, .. } => assert_eq!(root, expected),
            other => panic!("table parsed as {other:?}"),
        }
        match parse(&args(&["run", spec, "--master", "0x1"])).expect("run parses") {
            Command::Run { root, .. } => assert_eq!(root, expected),
            other => panic!("run parsed as {other:?}"),
        }
        match parse(&args(&["adjudicate", spec, "--master", "0x1"])).expect("adjudicate parses") {
            Command::Adjudicate { root, .. } => assert_eq!(root, expected),
            other => panic!("adjudicate parsed as {other:?}"),
        }
    }

    /// A `gen:` root carries no recorded seed, so `table` REFUSES without
    /// `--decision-seed` — the `path:event` sentence, widened to name both
    /// seedless forms rather than leaving one of them unexplained.
    #[test]
    fn a_gen_root_requires_a_decision_seed_at_table() {
        let err = run_with(
            &args(&["table", "gen:final-round-already-triggered:0x1:0"]),
            &[],
            Path::new("."),
        )
        .expect_err("a gen root without a seed must refuse");
        assert!(err.contains("--decision-seed"), "the refusal hides the flag: {err}");
        assert!(err.contains("gen:"), "the refusal does not name the gen form: {err}");
    }

    /// END TO END on the CHEAP family-1 ecology: a `gen:` root RESOLVES, and the
    /// report's header carries the generated provenance the way a registry root
    /// carries its identity — predicate, master, ordinal, the scan's game index,
    /// and the whole `GeneratingEcology`, which is the half that makes a
    /// `(predicate, master, ordinal)` triple an identity at all.
    ///
    /// The rung ecology is deliberate (design §6's wall discipline): the layer-2
    /// predicates scan the v2 LOOKAHEAD ecology, which is minutes.
    #[test]
    fn a_generated_root_resolves_and_prints_its_provenance_in_the_header() {
        let spec = format!("gen:final-round-already-triggered:{GEN_MASTER:#x}:0");
        let out = run_with(
            &args(&["table", &spec, "--decision-seed", "0x0DEC_0000_0000_0001"]),
            &[],
            Path::new("."),
        )
        .expect("the generated root resolves and tables");

        let ecology = driven::GeneratingEcology::v1();
        assert!(out.contains(RootPredicate::FinalRoundAlreadyTriggered.as_str()));
        for needle in [
            format!("{GEN_MASTER:#018x}"),
            "ordinal=0".to_string(),
            "game=".to_string(),
            ecology.id.clone(),
            ecology.driver.clone(),
            ecology.seed_derivation.clone(),
            format!("per_game_cap={}", ecology.per_game_cap),
            format!("games={}", ecology.games),
        ] {
            assert!(out.contains(&needle), "the header drops `{needle}`:\n{out}");
        }
        // A generated root has no recorded move, so the reproduction line is the
        // NOT CHECKED arm rather than a verdict about a move nobody recorded.
        assert!(out.contains("NOT CHECKED"), "a generated root claims a reproduction:\n{out}");
    }

    /// SHOULD-1 (T6 fix wave): a root reaching `run`/`adjudicate` is resolved
    /// EXACTLY ONCE, and the execution call is handed the ANSWER rather than the
    /// question.
    ///
    /// **The cost this is about is the `gen:` form's.** Resolving a generated
    /// root walks a seeded SCAN, and the default-seat path used to walk it
    /// twice — once for the player count `--seats` was not given, once again
    /// inside `drive_rooted`. On family 1's rung ecology (what this cell runs,
    /// per design §6's wall discipline) that is a second; on the v2 lookahead
    /// ecology it is minutes, paid for a number used only to size a `Vec`.
    ///
    /// **Measured, not argued**: `driven::scan_probe` counts the walks this
    /// thread took, so "once" is a 1 rather than a code reading. The other half
    /// is structural — the execution call receives a `RootSource::Resolved`,
    /// whose `resolve` arm has no scan in it at all — and the last block spends
    /// the identity end to end: the run's HEADER still names the scan, which is
    /// why the hand-off is `Resolved` and not `Constructed`.
    #[test]
    fn a_generated_root_is_resolved_once_and_the_execution_call_gets_the_answer() {
        let rules = Rules::standard();
        let spec = format!("gen:final-round-already-triggered:{GEN_MASTER:#x}:0");
        let asked =
            resolve(&parse_root_ref(&spec).expect("the spec parses"), &[], Path::new("."), &rules)
                .expect("the reference names a root");

        // DEFAULT seats — the path that used to resolve twice.
        let (resolved, scans) =
            driven::scan_probe::count_scans(|| resolve_once(&asked, None, &rules));
        let (source, seats) = resolved.expect("the generated root resolves");
        assert_eq!(scans, 1, "the default-seat path walked the scan {scans} times, not once");

        let RootSource::Resolved { state, descriptor } = &source else {
            panic!("the execution call is handed the question, not the answer: {source:?}");
        };
        assert!(
            matches!(descriptor.origin, RootOrigin::Generated { ordinal: 0, .. }),
            "the hand-off lost the scan that found the root: {:?}",
            descriptor.origin,
        );
        assert_eq!(
            seats,
            vec![
                SeatSpec::Contestant(default_contestant().expect("the default parses"));
                state.players.len()
            ],
            "the default seat list is not one default contestant per seat",
        );

        // The execution call's OWN resolve is then free and VERBATIM, which is
        // what makes "resolved once" structural rather than a promise about one
        // call site: this arm cannot scan.
        let (again, extra) = driven::scan_probe::count_scans(|| source.resolve(&rules));
        let (again_state, again_descriptor) = again.expect("a pre-resolved root re-resolves");
        assert_eq!(extra, 0, "the execution call re-walked the scan");
        assert_eq!(&again_descriptor, descriptor, "the execution call sees another descriptor");
        assert_eq!(
            state_digest(&again_state),
            state_digest(state),
            "the execution call sees another position",
        );

        // END TO END on the cheap seats (the default is `lookahead-v1`, and
        // this cell is about plumbing rather than play): ONE walk for the whole
        // command, and the header the run prints still carries the generated
        // identity the header of a `table` on the same root carries.
        let (rendered, run_scans) = driven::scan_probe::count_scans(|| {
            run_with(
                &args(&[
                    "run",
                    &spec,
                    "--master",
                    "0x0C06_2026_0818_9992",
                    "--seats",
                    "family-weighted, family-weighted",
                ]),
                &[],
                Path::new("."),
            )
        });
        let rendered = rendered.expect("the rooted run completes");
        assert_eq!(run_scans, 1, "one `run` walked the scan {run_scans} times");
        let ecology = driven::GeneratingEcology::v1();
        for needle in
            ["generated@", "ordinal=0", ecology.id.as_str(), ecology.seed_derivation.as_str()]
        {
            assert!(rendered.contains(needle), "the run's header drops `{needle}`:\n{rendered}");
        }
    }

    // =======================================================================
    // Phase C, T6 — the seat grammar's fourth menu slot
    // =======================================================================

    /// `race-rusher` parses as the fourth REPLY-MENU slot — argv-REACHABLE, so
    /// T7's pilots can name it in `--seats` — and every menu name round-trips
    /// through its own `as_str`.
    #[test]
    fn the_seat_grammar_reads_race_rusher_as_the_fourth_menu_slot() {
        assert_eq!(
            parse_seat("race-rusher").expect("race-rusher is a seat"),
            SeatSpec::Reply(ReplyPolicy::RaceRusher),
        );
        // HAND-MAINTAINED on both sides (the literal array in `parse_seat` and
        // the USAGE list), so the pin names all four rather than sweeping a
        // const that does not exist.
        for policy in [
            ReplyPolicy::FamilyWeighted,
            ReplyPolicy::GreedyAtFinal,
            ReplyPolicy::LookaheadAtFinal,
            ReplyPolicy::RaceRusher,
        ] {
            assert_eq!(
                parse_seat(policy.as_str()).unwrap_or_else(|e| panic!("{policy:?}: {e}")),
                SeatSpec::Reply(policy),
                "{policy:?} does not round-trip through the seat grammar",
            );
            assert!(
                USAGE.contains(policy.as_str()),
                "USAGE does not name the seat `{}`",
                policy.as_str(),
            );
        }
        // In a LIST, beside a contestant — the shape `--seats` actually takes.
        assert_eq!(
            parse_seats("race-rusher, greedy-v1").expect("a mixed seat list parses"),
            vec![
                SeatSpec::Reply(ReplyPolicy::RaceRusher),
                SeatSpec::Contestant(Contestant::GREEDY_V1_DEFAULT),
            ],
        );
    }

    // =======================================================================
    // Phase C, T6 — `family <id> --block <kind> [--arm <arm>] [--master <u64>]`
    // =======================================================================

    /// The verb parses: every declared family id, both blocks, both arms, and
    /// the master when one is typed.
    #[test]
    fn the_family_command_parses_every_id_block_and_arm() {
        for decl in families() {
            match parse(&args(&["family", decl.id, "--block", "pilot", "--master", "0x11"]))
                .unwrap_or_else(|e| panic!("{}: {e}", decl.id))
            {
                Command::Family { id, block, master, arm } => {
                    assert_eq!(id, decl.id);
                    assert_eq!(block, BlockKind::Pilot);
                    assert_eq!(master, Some(0x11));
                    assert_eq!(arm, FamilyArm::Positive, "the default arm is not positive");
                }
                other => panic!("{}: parsed as {other:?}", decl.id),
            }
        }
        match parse(&args(&[
            "family",
            "exclusive-race",
            "--block",
            "confirmatory",
            "--arm",
            "control",
        ]))
        .expect("a confirmatory control arm parses")
        {
            Command::Family { id, block, master, arm } => {
                assert_eq!(id, "exclusive-race");
                assert_eq!(block, BlockKind::Confirmatory);
                assert_eq!(master, None);
                assert_eq!(arm, FamilyArm::Control);
            }
            other => panic!("parsed as {other:?}"),
        }
    }

    /// An unknown family id refuses naming ALL FOUR declared ids.
    #[test]
    fn an_unknown_family_id_names_the_four_declared_ids() {
        let err = parse(&args(&["family", "final-rond", "--block", "pilot", "--master", "1"]))
            .expect_err("an unknown family must refuse");
        assert!(err.contains("final-rond"), "the refusal hides what was typed: {err}");
        for decl in families() {
            assert!(err.contains(decl.id), "the refusal does not name `{}`: {err}", decl.id);
        }
        assert!(err.contains(USAGE), "the refusal drops USAGE: {err}");
    }

    /// An unknown block name refuses naming THE TWO valid blocks — and the
    /// ESCROW-ABSENT property rides this arm: no grammar names an escrow block,
    /// so `--block escrow` is exactly as unknown as `--block wibble` and the
    /// refusal does not offer it as an alternative (design §2's escrow hygiene:
    /// escrow ordinals live in the freeze DOCUMENT so no command can burn one).
    #[test]
    fn an_unknown_block_name_refuses_naming_the_two_valid_blocks_and_no_third() {
        for typed in ["escrow", "wibble"] {
            let err = parse(&args(&["family", "final-round", "--block", typed])).unwrap_err();
            assert!(err.contains(typed), "the refusal hides what was typed: {err}");
            assert!(err.contains("pilot"), "the refusal does not name `pilot`: {err}");
            assert!(
                err.contains("confirmatory"),
                "the refusal does not name `confirmatory`: {err}",
            );
        }
        assert_eq!(BLOCK_NAMES, ["pilot", "confirmatory"], "the block vocabulary moved");
        assert!(!USAGE.contains("escrow block"), "USAGE names an escrow block");
    }

    /// `--block` is REQUIRED and an unknown `--arm` refuses naming both arms.
    #[test]
    fn the_family_command_requires_a_block_and_names_both_arms() {
        let no_block = parse(&args(&["family", "final-round", "--master", "1"]))
            .expect_err("a family run with no block must refuse");
        assert!(no_block.contains("--block"), "the refusal hides the flag: {no_block}");

        let bad_arm =
            parse(&args(&["family", "final-round", "--block", "pilot", "--arm", "sideways"]))
                .expect_err("an unknown arm must refuse");
        assert!(bad_arm.contains("sideways"), "the refusal hides what was typed: {bad_arm}");
        for arm in ARM_NAMES {
            assert!(bad_arm.contains(arm), "the refusal does not name `{arm}`: {bad_arm}");
        }
    }

    /// The two MASTER refusals, end to end and COSTING NOTHING — both fire in
    /// `resolve_block` before a single draw is resolved.
    ///
    /// Pilot without `--master`: recorded-before-use stays the caller's
    /// obligation. Confirmatory WITH one: the declaration's master is the
    /// committed identity, and a typo must not be able to run a
    /// plausible-looking non-frozen block.
    #[test]
    fn a_pilot_needs_a_master_and_a_confirmatory_refuses_one() {
        let missing =
            run_with(&args(&["family", "final-round", "--block", "pilot"]), &[], Path::new("."))
                .expect_err("a pilot with no master must refuse");
        assert!(missing.contains("recorded-before-use"), "the refusal loses its reason: {missing}");
        assert!(missing.contains("final-round"), "the refusal does not name the family: {missing}");
        assert!(missing.contains(USAGE), "the command-shaped refusal drops USAGE: {missing}");

        let supplied = run_with(
            &args(&["family", "final-round", "--block", "confirmatory", "--master", "0x1"]),
            &[],
            Path::new("."),
        )
        .expect_err("a confirmatory with a caller's master must refuse");
        assert!(
            supplied.contains("caller-supplied master is refused"),
            "the refusal loses its sentence: {supplied}",
        );
        assert!(supplied.contains(USAGE), "the command-shaped refusal drops USAGE: {supplied}");
    }

    /// **AFTER THE FREEZE** every family's confirmatory block RESOLVES its baked
    /// identity, in BOTH arms — and each arm resolves the block its own
    /// declaration carries.
    ///
    /// **This pin replaced a `run_with` sweep, and the reason is cost.** Before
    /// the freeze the whole argv path could be exercised end to end because it
    /// refused; a confirmatory run now executes `roots × menu × candidates × K`
    /// whole games at the shipped subject. So the two halves are pinned
    /// separately: the argv parse in
    /// `the_family_command_parses_every_id_block_and_arm`, and the
    /// arm-plus-block resolution here, which is everything between the parse and
    /// the first drawn root.
    #[test]
    fn every_confirmatory_block_resolves_its_baked_identity_in_both_arms() {
        for decl in families() {
            let control = control_arm(&decl).expect("every family declares a control arm");
            for (arm, resolved, expected) in [
                (FamilyArm::Positive, &decl, decl.confirmatory.clone()),
                (FamilyArm::Control, &control, decl.confirmatory_control.clone()),
            ] {
                let expected = expected.unwrap_or_else(|| {
                    panic!("{} --arm {}: no frozen block is baked", decl.id, arm.as_str())
                });
                let (master, ordinals) =
                    crate::game::driven_families::resolve_block(resolved, BlockKind::Confirmatory, None)
                        .unwrap_or_else(|e| {
                            panic!("{} --arm {}: {e}", decl.id, arm.as_str())
                        });
                assert_eq!(
                    master,
                    expected.master,
                    "{} --arm {}: the wrong master",
                    decl.id,
                    arm.as_str(),
                );
                assert_eq!(
                    ordinals,
                    expected.ordinals.clone().collect::<Vec<u32>>(),
                    "{} --arm {}: the wrong ordinals",
                    decl.id,
                    arm.as_str(),
                );
            }
        }
    }

    /// A declaration whose ARM carries no baked block still refuses BY NAME —
    /// the refusal the freeze wave re-pointed rather than deleted, reached the
    /// way a runner reaches it (through `control_arm`).
    #[test]
    fn a_control_arm_with_no_baked_block_still_refuses_by_name() {
        let mut decl = families().remove(0);
        decl.confirmatory_control = None;
        let control = control_arm(&decl).expect("family 1 has a control");
        assert!(
            control.confirmatory.is_none(),
            "the control arm did not take its own block: {:?}",
            control.confirmatory,
        );
        let err = crate::game::driven_families::resolve_block(&control, BlockKind::Confirmatory, None)
            .expect_err("an unbaked arm must refuse");
        assert!(
            err.to_string().contains(decl.id) && err.to_string().contains("FREEZE"),
            "the refusal does not name the family and the freeze: {err}",
        );
    }

    /// **THE SUPPLY ARITHMETIC** (the freeze wave, item 6): every frozen block's
    /// highest ordinal is inside the supply its generator's ECOLOGY declares.
    ///
    /// A scan's ordinal space is `games × per_game_cap` strided hits, and a
    /// frozen block that asks past it is a block that dies on `Exhausted` in the
    /// middle of a confirmatory run. This is that check as a test rather than as
    /// arithmetic in a commit message — and it is why the freeze needed NO
    /// games-bound change: v1.1 supplies 32 against family 1's 20 (pilot 4 +
    /// confirmatory 16), and the v2.3 pinned-lookahead ecology supplies 480 against the
    /// layer-2 families' 28 (pilot 4 + confirmatory 24).
    ///
    /// **WHAT THIS CHECK DOES NOT PROVE, learned the hard way TWICE
    /// (2026-08-18):** `games × per_game_cap` is the CAPACITY of the ordinal
    /// space, not the supply a predicate REALIZES inside it. Family 2's blocks
    /// passed this check at the freeze's capacity of 48 and still exhausted
    /// mid-protocol at 10 and 7 realized strided hits — which took the v2 bound
    /// 24 → 120 — and its control arm then passed the check at capacity 240 and
    /// exhausted AGAIN at 26 realized hits, which took the bound 120 → 240 (both
    /// prefix-stable extensions, so no block, master or cap moved at either).
    /// A block that FAILS here is certainly unrunnable; a block that passes is
    /// merely not yet ruled out, and only a scan settles it.
    ///
    /// BUILDER arms are exempt and say so: their ordinals index
    /// `SeedSchedule::diagnostic`, which has no bound.
    #[test]
    fn every_frozen_block_fits_the_supply_its_generator_declares() {
        let mut checked_predicates = 0;
        let mut checked_builders = 0;
        for decl in families() {
            let control = control_arm(&decl).expect("every family declares a control arm");
            for (arm, generator, block) in [
                ("positive", decl.generator, decl.confirmatory.clone()),
                ("control", control.generator, decl.confirmatory_control.clone()),
            ] {
                let block = block.expect("baked at the freeze");
                let highest = block.ordinals.end - 1;
                match generator {
                    FamilyGenerator::Predicate(predicate) => {
                        let ecology = predicate.ecology();
                        let supply = ecology.games * ecology.per_game_cap;
                        assert!(
                            highest < supply,
                            "{} --arm {arm}: ordinal {highest} is past the {} supply of {supply} \
                             ({} games × cap {})",
                            decl.id,
                            ecology.id,
                            ecology.games,
                            ecology.per_game_cap,
                        );
                        checked_predicates += 1;
                    }
                    FamilyGenerator::Builder { .. } => {
                        // A builder draws seeds off `SeedSchedule::diagnostic`,
                        // which is unbounded — there is no supply to fit.
                        checked_builders += 1;
                    }
                }
            }
        }
        // ANTI-VACUITY: both kinds were actually met.
        assert_eq!((checked_predicates, checked_builders), (5, 3));
    }

    // =======================================================================
    // Phase C, T6 — the `--arm control` mapping
    // =======================================================================

    /// `--arm control` re-points each family at its DECLARED control (or corner)
    /// generator, and — where the control asks a different question — at the
    /// control PASS RULE the declaration's own docs name.
    ///
    /// The mapping, as landed:
    ///
    /// | family           | control generator                | control rule                           |
    /// |------------------|----------------------------------|----------------------------------------|
    /// | `final-round`    | `TriggerRoundNotFinal` predicate | unchanged (`AdjudicatedBest`)          |
    /// | `end-trigger`    | `FAMILY2_CONTROL_GENERATOR`      | `Turn(NotEndFired)`                    |
    /// | `exclusive-race` | `RACE_CONTROL_GENERATOR`         | `Turn(DenialNotTaken)`                 |
    /// | `achievement`    | `PROXIMITY_CORNER_GENERATOR`     | unchanged (`Turn(AchievementClaimed)`) |
    #[test]
    fn the_control_arm_repoints_each_family_at_its_declared_arm_and_rule() {
        let by_id = |id: &str| {
            families().into_iter().find(|d| d.id == id).unwrap_or_else(|| panic!("{id} declared"))
        };

        let family1 = control_arm(&by_id("final-round")).expect("family 1 has a control");
        assert_eq!(
            family1.generator,
            FamilyGenerator::Predicate(RootPredicate::TriggerRoundNotFinal),
        );
        assert_eq!(family1.rule, by_id("final-round").rule, "family 1's rule moved");

        let family2 = control_arm(&by_id("end-trigger")).expect("family 2 has a control");
        assert_eq!(family2.generator, FAMILY2_CONTROL_GENERATOR);
        assert_eq!(family2.rule, PassRule::Turn(TurnPredicate::NotEndFired));

        let family3 = control_arm(&by_id("exclusive-race")).expect("family 3 has a control");
        assert_eq!(family3.generator, RACE_CONTROL_GENERATOR);
        assert_eq!(
            family3.rule,
            PassRule::Turn(TurnPredicate::DenialNotTaken(DECLARED_BY_THE_ROOT)),
            "family 3's control does not read the DENIAL rule",
        );
        assert!(family3.rule.names_a_step(), "the control rule must still be re-pointed per root");

        let family4 = control_arm(&by_id("achievement")).expect("family 4 has a corner");
        assert_eq!(family4.generator, PROXIMITY_CORNER_GENERATOR);
        assert_eq!(family4.rule, by_id("achievement").rule, "family 4's rule moved");

        // Every other field is the declaration's, untouched: an arm is a
        // GENERATOR and a RULE, never a different family.
        for (control, id) in [
            (&family1, "final-round"),
            (&family2, "end-trigger"),
            (&family3, "exclusive-race"),
            (&family4, "achievement"),
        ] {
            let positive = by_id(id);
            assert_eq!(control.id, positive.id);
            assert_eq!(control.label, positive.label);
            assert_eq!(control.class, positive.class);
            assert_eq!(control.replications, positive.replications);
            assert_eq!(control.per_root_f, positive.per_root_f);
            assert_eq!(control.menu, positive.menu);
            assert_eq!(control.pilot, positive.pilot);
            assert_ne!(
                control.generator, positive.generator,
                "{id}: a control equal to its positive",
            );
            // THE FROZEN BLOCK IS RE-POINTED TOO (the freeze wave): the arms
            // draw different roots, so a control run that inherited the positive
            // arm's block would quote the positive arm's identity.
            assert_eq!(
                control.confirmatory, positive.confirmatory_control,
                "{id}: the control arm did not take its own frozen block",
            );
            assert_eq!(
                control.confirmatory_control, positive.confirmatory_control,
                "{id}: an arm is a GENERATOR, a RULE and a BLOCK — never a different declaration",
            );
        }
    }

    /// THE SEAM (T6 fix wave, review MUST-1; extended by the freeze wave): the
    /// sizing hands BOTH arms the same posture with the admission inversion OFF,
    /// and hands each BLOCK its own posture.
    ///
    /// **The mutation this exists for is the natural one** — `BlockOpts {
    /// control: arm == FamilyArm::Control, ..BlockOpts::pilot() }`, which reads
    /// MORE careful than what ships and is the control's own claim upside down
    /// (see [`family_block_opts`] for the double negation). Written at the call
    /// site it survived the entire suite, because `run_with`'s `Command::Family`
    /// Ok path is unreachable in test: it refused before the freeze, and since
    /// the freeze it is hours of real games. Named, it has somewhere to fail —
    /// and the freeze wave's own edit (the block choosing the posture) rides the
    /// same seam for the same reason.
    #[test]
    fn the_family_block_sizing_keeps_the_admission_inversion_off_in_both_arms() {
        for block in [BlockKind::Pilot, BlockKind::Confirmatory] {
            let positive = family_block_opts(FamilyArm::Positive, block);
            let control = family_block_opts(FamilyArm::Control, block);
            assert!(
                !control.control,
                "the CONTROL arm inverts admission on top of its already-negated rule: a double \
                 negation, and the control's own claim upside down",
            );
            assert!(!positive.control, "the POSITIVE arm inverts admission");
            // WHOLE-VALUE equality through `Debug`, because `BlockOpts` carries
            // no `PartialEq` and a field-by-field compare would silently skip a
            // field added later: an arm is a GENERATOR and a RULE, never a
            // sizing.
            assert_eq!(
                format!("{control:?}"),
                format!("{positive:?}"),
                "the two arms run at different sizings at block {}",
                block.as_str(),
            );
        }

        // THE BLOCK, on the other hand, IS a sizing (the freeze wave): a pilot
        // runs the pre-freeze posture and a confirmatory runs the frozen one.
        assert_eq!(
            format!("{:?}", family_block_opts(FamilyArm::Positive, BlockKind::Pilot)),
            format!("{:?}", BlockOpts::pilot()),
            "a pilot block no longer runs the posture the pilots that already ran used",
        );
        assert_eq!(
            format!("{:?}", family_block_opts(FamilyArm::Control, BlockKind::Confirmatory)),
            format!("{:?}", BlockOpts::freeze()),
            "a confirmatory block does not run the FROZEN posture",
        );
        assert_ne!(
            format!("{:?}", BlockOpts::freeze()),
            format!("{:?}", BlockOpts::pilot()),
            "the two postures are one sizing, so the block cannot be choosing between them",
        );
    }

    /// An id no declaration carries refuses rather than silently returning the
    /// positive arm — the defensive half of an exhaustive mapping.
    #[test]
    fn the_control_arm_refuses_a_family_it_does_not_know() {
        let mut invented = families().remove(0);
        invented.id = "not-a-family";
        let err = control_arm(&invented).expect_err("an unknown id must refuse");
        assert!(err.contains("not-a-family"), "the refusal hides the id: {err}");
        for decl in families() {
            assert!(err.contains(decl.id), "the refusal does not name `{}`: {err}", decl.id);
        }
    }

    // =======================================================================
    // Phase C, T6 — rendering a FamilyBlockReport (CONSTRUCTED outcomes only)
    // =======================================================================

    fn counter_check(tied: usize, band: usize, distinct: usize) -> CounterCheck {
        CounterCheck {
            table: table_with(
                Move::SkipBuilding,
                &[(Move::SkipBuilding, 12), (Move::BuildTradingHouse, 11)],
            ),
            tied_at_top: tied,
            top_band: band,
            distinct_values: distinct,
        }
    }

    fn turn_replication(r: u32, verdict: bool) -> ReplicationRead {
        ReplicationRead {
            replication: r,
            decision_seed: 0x0D00_0000_0000_0000 + u64::from(r),
            root_word: 0x00FF_0000_0000_00AB,
            verdict: Some(verdict),
            pick: None,
            turn_moves: 4,
            handoff_digest: Some(format!("handoff-{r}")),
            game_over: Some(false),
            counters: SeatCounters::NONE,
        }
    }

    fn turn_read(misses: u32, r: u32) -> FamilyReadOutcome {
        FamilyReadOutcome {
            rule: PassRule::Turn(TurnPredicate::EndFired),
            grain: ReadGrain::Turn,
            master: 0x0A3E_0000_0000_0001,
            root_word: 0x00FF_0000_0000_00AB,
            root_digest: "00ff0000000000ab".to_string(),
            subject_echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
            replications: (0..r).map(|i| turn_replication(i, i >= misses)).collect(),
            counter_check: counter_check(2, 2, 2),
            annotations: vec![CounterAnnotation::TiedAtTop { candidates: 2 }],
        }
    }

    fn policy_admission(policy: ReplyPolicy, verdict: AdmissionVerdict) -> PolicyAdmission {
        PolicyAdmission {
            policy,
            true_n: 4,
            false_n: 4,
            true_mean: 12.0,
            false_mean: 1.0,
            difference: 11.0,
            verdict,
        }
    }

    fn family_row(ordinal: u32, verdict: RootVerdict, read: Option<FamilyReadOutcome>) -> RootRow {
        RootRow {
            ordinal,
            descriptor: descriptor(),
            draw_seed: Some(0x5EED_0000_0000_0000 + u64::from(ordinal)),
            step: None,
            rule: PassRule::Turn(TurnPredicate::EndFired),
            admission: Some(AdmissionRead {
                delta: 5,
                control: false,
                policies: vec![
                    policy_admission(ReplyPolicy::FamilyWeighted, AdmissionVerdict::Dominates),
                    policy_admission(ReplyPolicy::GreedyAtFinal, AdmissionVerdict::Dominates),
                ],
            }),
            dead_by_policy: Vec::new(),
            dead: false,
            top_sets: Vec::new(),
            read,
            misses: 0,
            unresolved: 0,
            verdict,
            lookahead_sole_dissenter: false,
            rider: None,
        }
    }

    fn family_report(verdict: FamilyVerdict, roots: Vec<RootRow>) -> FamilyBlockReport {
        let drawn = roots.len();
        let admitted = roots.iter().filter(|r| r.read.is_some()).count();
        let counter_refused =
            roots.iter().filter(|r| matches!(r.verdict, RootVerdict::CounterRefused(_))).count();
        let sole_dissenters =
            roots.iter().filter(|r| r.lookahead_sole_dissenter).map(|r| r.ordinal).collect();
        FamilyBlockReport {
            id: "end-trigger",
            label: FamilyLabel::Confirmatory,
            class: StrategicClass::EndTimingProximity,
            block: BlockKind::Pilot,
            generator: "end-firable-ahead-in-truth",
            master: 0x0A3E_0000_0000_0001,
            subject_echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
            rule: PassRule::Turn(TurnPredicate::EndFired),
            replications: 20,
            per_root_f: 1,
            menu: vec![ReplyPolicy::FamilyWeighted, ReplyPolicy::GreedyAtFinal],
            admission_k: 4,
            delta: 5,
            control: false,
            roots,
            drawn,
            admitted,
            counter_refused,
            sole_dissenters,
            verdict,
            robustness_read: driven::ROBUSTNESS_READ,
        }
    }

    fn line_index(rendered: &str, needle: &str) -> usize {
        rendered
            .lines()
            .position(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("the report has no line containing {needle:?}:\n{rendered}"))
    }

    /// **THE SECTION ORDER IS THE PIN** (design §2): the family report leads
    /// with its LABEL — what a pass would even claim — and D7's STEP-ONE
    /// counter check comes second, before any per-policy arithmetic. A report
    /// that put the numbers first would invite a reader to quote them before
    /// learning whether the instrument refused the framing.
    #[test]
    fn the_family_report_leads_with_the_label_then_the_counter_check() {
        let report = family_report(
            FamilyVerdict::Pass,
            vec![family_row(0, RootVerdict::Pass, Some(turn_read(0, 4)))],
        );
        let rendered = render_family_block(&report, FamilyArm::Positive);

        let label = line_index(&rendered, "CONFIRMATORY");
        let counter = line_index(&rendered, "counter check");
        let admission = line_index(&rendered, "per-policy admission");
        assert!(rendered.starts_with("=== driven family ===\n"), "no banner:\n{rendered}");
        assert_eq!(label, 1, "the LABEL is not the first content line:\n{rendered}");
        assert!(label < counter, "the counter check precedes the label:\n{rendered}");
        assert!(counter < admission, "the admission table precedes the counter check:\n{rendered}");
        // The rest of the sequence, so a section reorder fails here rather than
        // in a T7 triage note: roots, then exclusions, then the dissent block.
        let roots = line_index(&rendered, "  roots:");
        let exclusions = line_index(&rendered, "  exclusions (");
        let dissent = line_index(&rendered, "  dissent ");
        assert!(
            admission < roots && roots < exclusions && exclusions < dissent,
            "the section sequence moved:\n{rendered}",
        );
        // The identity a reader needs beside the label.
        for needle in ["end-trigger", "pilot", "arm=positive", "end-firable-ahead-in-truth"] {
            assert!(rendered.contains(needle), "the header drops `{needle}`:\n{rendered}");
        }
        assert!(
            rendered.contains(driven::ROBUSTNESS_READ),
            "the family report drops the robustness read:\n{rendered}",
        );
    }

    /// EVERY replication prints its own DECISION SEED and the root word
    /// verbatim — T7's separator feeds a failing replication's seed straight to
    /// `table … --decision-seed`, and a seed a reader must re-derive from a mix
    /// formula is a seed nobody feeds anywhere.
    #[test]
    fn the_family_report_prints_every_replication_seed_and_root_word() {
        let read = turn_read(1, 3);
        let report = family_report(
            FamilyVerdict::Pass,
            vec![family_row(0, RootVerdict::Pass, Some(read.clone()))],
        );
        let rendered = render_family_block(&report, FamilyArm::Positive);
        for replication in &read.replications {
            assert!(
                rendered.contains(&format!("{:#018x}", replication.decision_seed)),
                "replication {}'s decision seed is missing:\n{rendered}",
                replication.replication,
            );
        }
        assert!(
            rendered.contains(&format!("{:#018x}", read.root_word)),
            "the root word is missing:\n{rendered}",
        );
        // The DRAW SEED where one is recorded — a builder draw's only
        // reproduction handle.
        assert!(
            rendered.contains(&format!("{:#018x}", 0x5EED_0000_0000_0000_u64)),
            "the draw seed is missing:\n{rendered}",
        );
        // The exact binomial line, verbatim from the outcome that computed it.
        assert!(
            rendered.contains(&read.binomial_line().expect("a turn-grain read has one")),
            "the binomial line is missing:\n{rendered}",
        );
    }

    /// A PREDICATE draw records no draw seed, and the render says so rather
    /// than inventing one — `None` is the honest shape for a scanned root
    /// (`RootRow::draw_seed`'s own rule).
    #[test]
    fn a_predicate_draws_absent_seed_renders_as_absent() {
        let mut row = family_row(0, RootVerdict::Pass, Some(turn_read(0, 2)));
        row.draw_seed = None;
        let rendered = render_family_block(
            &family_report(FamilyVerdict::Pass, vec![row]),
            FamilyArm::Positive,
        );
        assert!(
            rendered.contains("draw seed      -"),
            "an absent draw seed does not render as absent:\n{rendered}",
        );
    }

    /// The per-policy ADMISSION table, the EXCLUSION block and the DISSENT
    /// block — the three things a reader needs to tell "the subject failed"
    /// from "the root was never in the denominator".
    #[test]
    fn the_family_report_renders_admission_exclusions_and_the_dissent_block() {
        let mut excluded =
            family_row(1, RootVerdict::Excluded(ExclusionReason::NotDemonstrated), None);
        excluded.admission = Some(AdmissionRead {
            delta: 5,
            control: false,
            policies: vec![
                policy_admission(ReplyPolicy::FamilyWeighted, AdmissionVerdict::Dominates),
                policy_admission(ReplyPolicy::LookaheadAtFinal, AdmissionVerdict::Fails),
            ],
        });
        excluded.lookahead_sole_dissenter = true;
        let dead = family_row(2, RootVerdict::Excluded(ExclusionReason::Dead), None);

        let report = family_report(
            FamilyVerdict::Pass,
            vec![family_row(0, RootVerdict::Pass, Some(turn_read(0, 2))), excluded, dead],
        );
        let rendered = render_family_block(&report, FamilyArm::Positive);

        // The table: one row per (root, policy), with the arithmetic δ is
        // compared to.
        assert!(rendered.contains("per-policy admission"), "no admission table:\n{rendered}");
        assert!(rendered.contains("DOMINATES"), "no dominance verdict:\n{rendered}");
        assert!(rendered.contains("FAILS"), "no failing verdict:\n{rendered}");
        assert!(rendered.contains("11.00"), "the difference is missing:\n{rendered}");

        // The exclusions, with their typed reasons.
        assert!(rendered.contains("exclusions"), "no exclusion block:\n{rendered}");
        assert!(rendered.contains("NOT DEMONSTRATED"), "no not-demonstrated reason:\n{rendered}");
        assert!(rendered.contains("DEAD"), "no dead reason:\n{rendered}");

        // The dissent block, and the circularity sentence it exists for.
        assert!(
            rendered.contains("SOLE DISSENTER"),
            "the sole-dissenter routing is missing:\n{rendered}",
        );
        assert!(
            rendered.to_lowercase().contains("suppression"),
            "the sole-dissenter block loses its reason:\n{rendered}",
        );
    }

    /// A COUNTER-REFUSED root renders as a ROUTING and never as a failure —
    /// D7's branch 0, which is the one reading a family report must not
    /// mis-summarize.
    #[test]
    fn a_counter_refused_root_renders_as_a_routing_not_a_failure() {
        let refused = family_row(
            0,
            RootVerdict::CounterRefused("ply_cap_hits=3 — a truncated search's pick".to_string()),
            None,
        );
        let rendered = render_family_block(
            &family_report(FamilyVerdict::Pass, vec![refused]),
            FamilyArm::Positive,
        );
        assert!(rendered.contains("COUNTER REFUSED"), "no counter-refusal row:\n{rendered}");
        assert!(rendered.contains("ply_cap_hits=3"), "the evidence is dropped:\n{rendered}");
        assert!(
            rendered.to_lowercase().contains("routing"),
            "a refusal reads as a failure:\n{rendered}",
        );
        assert!(rendered.contains("counter-refused=1"), "the count is missing:\n{rendered}");
    }

    /// The VOID and EXISTENCE verdicts render DISTINCTLY — a block that
    /// confirms nothing must not be readable as a quiet pass, and an existence
    /// demonstration must not be readable as a GO.
    #[test]
    fn the_void_and_existence_verdicts_render_distinctly() {
        let void = family_report(
            FamilyVerdict::VoidAsConfirmation {
                admitted: 2,
                drawn: 6,
                minimum: MIN_ADMITTED_ROOTS,
                disposition: VOID_DISPOSITION,
            },
            vec![family_row(0, RootVerdict::Pass, Some(turn_read(0, 2)))],
        );
        let rendered = render_family_block(&void, FamilyArm::Positive);
        assert!(rendered.contains("VOID AS CONFIRMATION"), "the void verdict is quiet:\n{rendered}");
        assert!(
            rendered.contains("CONFIRMS NOTHING"),
            "the void verdict does not say what it means:\n{rendered}",
        );
        assert!(rendered.contains(VOID_DISPOSITION), "the disposition is dropped:\n{rendered}");

        let mut existence = family_report(
            FamilyVerdict::GapDemonstrated { roots: 3 },
            vec![family_row(0, RootVerdict::GapRecorded { misses: 2 }, Some(turn_read(2, 4)))],
        );
        existence.label = FamilyLabel::ExistenceDemonstration;
        existence.id = "exclusive-race";
        let rendered = render_family_block(&existence, FamilyArm::Positive);
        assert!(
            rendered.contains("EXISTENCE_DEMONSTRATION"),
            "the existence label is missing:\n{rendered}",
        );
        assert!(rendered.contains("GAP DEMONSTRATED"), "the gap verdict is missing:\n{rendered}");
        assert!(
            rendered.contains("never a GO"),
            "the existence arm does not say what it cannot support:\n{rendered}",
        );
        assert!(rendered.contains("GAP RECORDED"), "the per-root gap row is missing:\n{rendered}");
    }

    /// **NOTHING ADMITTED renders as no reading at all** (the freeze wave, pilot
    /// finding 2), and the EXISTENCE arm says so too when its gap block is empty.
    ///
    /// The pilot printed `verdict PASS` over an empty denominator. What a reader
    /// must meet instead is the sentence that block earned: nothing entered the
    /// denominator, so there is nothing to read.
    #[test]
    fn a_block_that_admitted_nothing_renders_no_reading_and_the_empty_gap_says_so() {
        let nothing = family_report(FamilyVerdict::NothingAdmitted { drawn: 4 }, vec![]);
        let rendered = render_family_block(&nothing, FamilyArm::Positive);
        assert!(
            rendered.contains("NOTHING ADMITTED"),
            "an empty denominator does not announce itself:\n{rendered}",
        );
        assert!(
            rendered.contains("no reading"),
            "the verdict does not say what it means:\n{rendered}",
        );
        assert!(
            !rendered.contains("verdict        PASS"),
            "an empty denominator still renders as a pass:\n{rendered}",
        );

        // The EXISTENCE arm keeps its gap wording and GAINS the empty clause.
        let mut empty_gap = family_report(FamilyVerdict::GapDemonstrated { roots: 0 }, vec![]);
        empty_gap.label = FamilyLabel::ExistenceDemonstration;
        empty_gap.id = "exclusive-race";
        let rendered = render_family_block(&empty_gap, FamilyArm::Positive);
        assert!(rendered.contains("GAP DEMONSTRATED"), "the gap verdict moved:\n{rendered}");
        assert!(
            rendered.contains("over 0 admitted roots this demonstrates nothing"),
            "an empty gap block reads as a demonstration:\n{rendered}",
        );
        // ANTI-VACUITY: a non-empty gap block does NOT carry the clause.
        let three = family_report(FamilyVerdict::GapDemonstrated { roots: 3 }, vec![]);
        assert!(
            !render_family_block(&three, FamilyArm::Positive).contains("demonstrates nothing"),
            "the empty clause rides a block that demonstrated something",
        );
    }

    /// The two freeze-wave EXCLUSION reasons render as themselves: a FORCED root
    /// says what a one-row table is, and a too-few-splitting root NAMES its
    /// splitting count rather than reading as "the behaviour failed here".
    #[test]
    fn the_forced_and_too_few_splitting_exclusions_render_their_own_reasons() {
        let forced =
            family_row(0, RootVerdict::Excluded(ExclusionReason::Forced { candidates: 1 }), None);
        let thin = family_row(
            1,
            RootVerdict::Excluded(ExclusionReason::TooFewSplitting { splitting: 1, minimum: 2 }),
            None,
        );
        let rendered = render_family_block(
            &family_report(FamilyVerdict::NothingAdmitted { drawn: 2 }, vec![forced, thin]),
            FamilyArm::Positive,
        );

        assert!(rendered.contains("FORCED"), "the forced exclusion is unnamed:\n{rendered}");
        assert!(
            rendered.contains("a one-row table is a FORCED position — it confirms nothing and \
                               fails nothing"),
            "the forced row loses the sentence it exists to print:\n{rendered}",
        );
        assert!(
            rendered.contains("TOO FEW SPLITTING"),
            "the splitting exclusion is unnamed:\n{rendered}",
        );
        assert!(
            rendered.contains("1 of the declared policies split") || rendered.contains("splitting=1"),
            "the splitting exclusion does not name its count:\n{rendered}",
        );
    }

    /// The DECISION-grain shape (family 1's), which the turn-grain cells above
    /// never reach: no admission table at all, the LIVE/DEAD split and the
    /// per-policy top sets printed per root instead, the subject's PICKS on the
    /// replication lines, and a binomial line that says why there is none.
    #[test]
    fn the_decision_grain_report_prints_the_dead_split_the_top_sets_and_the_picks() {
        let read = FamilyReadOutcome {
            rule: PassRule::AdjudicatedBest { epsilon: 1 },
            grain: ReadGrain::Decision,
            master: 0x0A3E_0000_0000_0001,
            root_word: 0x00FF_0000_0000_00AB,
            root_digest: "00ff0000000000ab".to_string(),
            subject_echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
            replications: vec![ReplicationRead {
                replication: 0,
                decision_seed: 0x0DEC_0000_0000_0007,
                root_word: 0x00FF_0000_0000_00AB,
                verdict: None,
                pick: Some(Move::SkipBuilding),
                turn_moves: 0,
                handoff_digest: None,
                game_over: None,
                counters: SeatCounters::NONE,
            }],
            counter_check: counter_check(1, 1, 3),
            annotations: Vec::new(),
        };
        let mut row = family_row(0, RootVerdict::Pass, Some(read));
        row.admission = None;
        row.rule = PassRule::AdjudicatedBest { epsilon: 1 };
        row.dead_by_policy =
            vec![(ReplyPolicy::FamilyWeighted, false), (ReplyPolicy::GreedyAtFinal, true)];
        row.top_sets = vec![
            (ReplyPolicy::FamilyWeighted, vec![Move::SkipBuilding]),
            (ReplyPolicy::GreedyAtFinal, vec![Move::SkipBuilding, Move::BuildTradingHouse]),
        ];
        let mut report = family_report(FamilyVerdict::Pass, vec![row]);
        report.id = "final-round";
        report.rule = PassRule::AdjudicatedBest { epsilon: 1 };
        let rendered = render_family_block(&report, FamilyArm::Positive);

        assert!(
            rendered.contains("(none: this block reads at the DECISION grain"),
            "the empty admission table does not say why it is empty:\n{rendered}",
        );
        assert!(rendered.contains("dead? family-weighted"), "no per-policy dead line:\n{rendered}");
        assert!(rendered.contains("DEAD"), "no dead classification:\n{rendered}");
        assert!(rendered.contains("live"), "no live classification:\n{rendered}");
        assert!(rendered.contains("top set greedy-at-final"), "no top-set line:\n{rendered}");
        assert!(rendered.contains("2 candidate(s)"), "the top-set size is missing:\n{rendered}");
        assert!(rendered.contains("pick SkipBuilding"), "the pick is missing:\n{rendered}");
        assert!(
            rendered.contains("decision grain: a miss is a fact about a TOP SET"),
            "the decision-grain binomial line does not say why there is none:\n{rendered}",
        );
        assert!(rendered.contains("grain=decision"), "the grain is not stated:\n{rendered}");
    }

    /// Family 4's RIDER prints beside its root, labelled RECORDED and never
    /// gated — the STOP fence's own sentence, on the page rather than in a doc
    /// nobody reads next to the number.
    #[test]
    fn a_family_four_row_prints_its_recorded_rider() {
        let mut row = family_row(0, RootVerdict::Pass, Some(turn_read(0, 2)));
        row.rule = PassRule::Turn(TurnPredicate::AchievementClaimed);
        row.rider = Some(crate::game::driven_families::AchievementRider {
            progress_sum: 17,
            postclaim_sum: 11,
            race_proximity: -3,
        });
        let mut report = family_report(FamilyVerdict::Pass, vec![row]);
        report.id = "achievement";
        report.rule = PassRule::Turn(TurnPredicate::AchievementClaimed);
        let rendered = render_family_block(&report, FamilyArm::Positive);
        assert!(
            rendered.contains("progress_sum=17")
                && rendered.contains("postclaim_sum=11")
                && rendered.contains("race_proximity=-3"),
            "the rider's three readings are not all printed:\n{rendered}",
        );
        assert!(
            rendered.contains("RECORDED, never gated"),
            "the rider does not say what it may not do:\n{rendered}",
        );
    }

    /// Rendering is a pure function of the report plus the arm: the same pair
    /// renders byte-identically twice, and the ARM reaches the page — including
    /// the DOUBLE-NEGATION sentence, which is the one thing a reader of a
    /// control block could otherwise get backwards.
    #[test]
    fn the_family_render_is_deterministic_and_the_control_arm_states_its_direction() {
        let report = family_report(
            FamilyVerdict::Pass,
            vec![family_row(0, RootVerdict::Pass, Some(turn_read(0, 3)))],
        );
        let control = render_family_block(&report, FamilyArm::Control);
        assert_eq!(control, render_family_block(&report, FamilyArm::Control));
        assert_ne!(
            control,
            render_family_block(&report, FamilyArm::Positive),
            "the arm does not reach the report",
        );
        assert!(control.contains("arm=control"), "the arm is not named:\n{control}");
        assert!(
            control.contains("double negation"),
            "the control arm does not state its admission direction:\n{control}",
        );
    }

    /// THE TRIPWIRE (T6 fix wave, review MUST-1): a CONTROL render whose report
    /// says the run INVERTED admission is branded, loudly, above every number
    /// the inversion would otherwise quietly turn around.
    ///
    /// **Why a brand on the page and not only a cell.** A block run is minutes
    /// to hours and lands in a T7 triage note, so the failure that matters is
    /// not a red suite but a table somebody QUOTES. `report.control` is the
    /// sizing the run actually used and `arm` is what the caller asked for, so
    /// any edit reaching `BlockOpts::control` around [`family_block_opts`]
    /// surfaces in its own output rather than in a reviewer's memory.
    #[test]
    fn a_control_render_of_an_inverted_report_is_branded_a_double_negation() {
        let mut report = family_report(
            FamilyVerdict::Pass,
            vec![family_row(0, RootVerdict::Pass, Some(turn_read(0, 3)))],
        );

        // The HONEST pair — what `family_block_opts` produces — is UNBRANDED,
        // so the brand is a signal and not decoration.
        assert!(!report.control, "the constructed report is inverted before the cell starts");
        let honest = render_family_block(&report, FamilyArm::Control);
        assert!(
            !honest.contains("DOUBLE NEGATION"),
            "a correctly-sized control block is branded:\n{honest}",
        );

        // THE DEFECT: control arm, inverted sizing.
        report.control = true;
        let branded = render_family_block(&report, FamilyArm::Control);
        assert!(
            branded.contains(DOUBLE_NEGATION_BRAND),
            "the inverted control block carries no brand:\n{branded}",
        );
        assert!(
            line_index(&branded, "DOUBLE NEGATION") < line_index(&branded, "  sizing "),
            "the brand sits below the arithmetic it is about:\n{branded}",
        );

        // The POSITIVE arm inverted is NOT this defect: inverting admission
        // while keeping the POSITIVE rule is the other spelling of the same
        // test ([`family_block_opts`]'s doc), so branding it would cry wolf at
        // a legitimate caller — and the sizing line already declares it.
        let positive = render_family_block(&report, FamilyArm::Positive);
        assert!(
            !positive.contains("DOUBLE NEGATION"),
            "the positive arm's legitimate inversion is branded a defect:\n{positive}",
        );
        assert!(
            positive.contains("admission-inverted=true"),
            "the positive arm hides the inversion it ran under:\n{positive}",
        );
    }

    // =======================================================================
    // Phase C, T6 — the `opening` probe
    // =======================================================================

    /// The verb parses, and its two required flags refuse when absent.
    #[test]
    fn the_opening_command_parses_and_requires_its_master_and_games() {
        match parse(&args(&["opening", "--master", "0x2A", "--games", "3"])).expect("parses") {
            Command::Opening { master, games, contestant } => {
                assert_eq!(master, 0x2A);
                assert_eq!(games, 3);
                assert_eq!(contestant, Contestant::LOOKAHEAD_V1_DEFAULT);
            }
            other => panic!("parsed as {other:?}"),
        }
        match parse(&args(&[
            "opening",
            "--master",
            "1",
            "--games",
            "2",
            "--contestant",
            "greedy-v1",
        ]))
        .expect("a subject spec parses")
        {
            Command::Opening { contestant, .. } => {
                assert_eq!(contestant, Contestant::GREEDY_V1_DEFAULT);
            }
            other => panic!("parsed as {other:?}"),
        }

        let no_master = parse(&args(&["opening", "--games", "2"])).expect_err("no master refuses");
        assert!(no_master.contains("--master"), "{no_master}");
        let no_games = parse(&args(&["opening", "--master", "1"])).expect_err("no games refuses");
        assert!(no_games.contains("--games"), "{no_games}");
        let zero = parse(&args(&["opening", "--master", "1", "--games", "0"]))
            .expect_err("zero games refuses");
        assert!(zero.contains("--games"), "{zero}");
        let positional = parse(&args(&["opening", "3", "--master", "1", "--games", "2"]))
            .expect_err("a stray positional refuses");
        assert!(positional.contains("positional"), "{positional}");
    }

    /// **THE SETUP-EQUIVALENCE PIN** (plan review SHOULD-5). The probe builds
    /// its opening through the PUBLIC path — `new_game`, each seat's own
    /// `ArenaAgent::choose_starting_order` in seat order, `finalize_setup` —
    /// because the Phase-B split seamed out only `play_from_state` and the
    /// arena's setup half is still interior to its private driver. The pin is
    /// that the two agree BIT FOR BIT at the same master and the same streams:
    /// the built opening equals `state_at(record, 0)` of a `run_arena_game`
    /// there.
    ///
    /// Rung seats, one cell — the property is STRUCTURAL (which stream feeds
    /// which Keep), never a fact about the subject.
    #[test]
    fn the_opening_setup_path_is_bit_identical_to_the_arena_setup_half() {
        let rules = Rules::standard();
        let subject = Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT);
        let probe = run_opening(OPENING_MASTER, 2, subject, &rules).expect("the probe runs");
        assert_eq!(probe.rows.len(), 2, "the probe played the wrong number of games");

        for row in &probe.rows {
            let world = WorldSeed(row.world);
            let mut make = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
                subject.build(&rules, agent_stream_seed(world, seat, Slot(seat.0))).0
            };
            let stats = run_arena_game(world, 2, &rules, &mut make, true)
                .expect("the comparison game completes");
            let arena_opening =
                state_at(&stats.record, 0, &rules).expect("the record's position 0 replays");
            assert_eq!(
                row.opening_digest,
                state_digest(&arena_opening),
                "game {}: the probe's opening is not the arena's",
                row.game_index,
            );
        }
    }

    /// The probe plays the FIRST TURN of the first-moving seat and reads the
    /// three things the C-D question is about off it.
    #[test]
    fn the_opening_probe_reads_placements_president_and_purchase_off_the_first_turn() {
        let rules = Rules::standard();
        let probe = run_opening(
            OPENING_MASTER,
            3,
            Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
            &rules,
        )
        .expect("the probe runs");
        assert_eq!(probe.rows.len(), 3);
        for row in &probe.rows {
            assert!(row.moves > 0, "game {}: an empty first turn", row.game_index);
            assert!(
                !row.game_over,
                "game {}: the first turn ended the game, which no opening does",
                row.game_index,
            );
            assert_eq!(
                row.decision_seed,
                agent_stream_seed(WorldSeed(row.world), row.mover, Slot(row.mover.0)),
                "the turn is not taken on the arena's own keying",
            );
            // THE ONE STRUCTURAL RELATION between the two read-outs: a
            // president that never left the hand cannot execute an area
            // action. It is not vacuous — a rung really does take Option B's
            // return-to-hand on turn one sometimes (game 1 at this master
            // does), which is exactly why the probe REPORTS `-` there instead
            // of asserting a president it has no right to expect.
            if row.president.is_none() {
                assert_eq!(
                    row.purchase, OPENING_NO_ACTION,
                    "game {}: an area action with the president still in hand",
                    row.game_index,
                );
            }
        }
        // ANTI-VACUITY: the reader is not a function that always says "nothing".
        assert!(
            probe.rows.iter().any(|row| {
                !row.placements.is_empty()
                    && row.president.is_some()
                    && row.purchase != OPENING_NO_ACTION
            }),
            "no game read a placement, a president AND an area action: {:?}",
            probe.rows,
        );
        // ANTI-VACUITY on the OTHER arm (T6 fix wave, review NOTE-2): the
        // structural relation above is an implication, so it holds trivially at
        // any master where no president ever stays in hand — and it would then
        // pin nothing while still reading green. The comment says game 1 at
        // THIS master takes Option B's return-to-hand; this is that claim, made
        // executable, so a future master that loses the case fails here instead
        // of quietly emptying the cell.
        assert!(
            probe.rows.iter().any(|row| row.president.is_none()),
            "no game left the president in hand, so the relation above is vacuous at this \
             master — re-point it or re-choose the master: {:?}",
            probe.rows,
        );
        // Distinct worlds per game — the schedule advances.
        let worlds: std::collections::BTreeSet<u64> = probe.rows.iter().map(|r| r.world).collect();
        assert_eq!(worlds.len(), 3, "two games share a world");
    }

    /// The RENDER, on a CONSTRUCTED tally: the distribution, the lab-rush share
    /// with its Wilson interval, and the SETUP-PATH line the ecology discipline
    /// asks for (design NOTE-4).
    #[test]
    fn the_opening_render_carries_the_distribution_the_wilson_interval_and_the_setup_path() {
        let row = |index: u32, president: &str, purchase: &str| OpeningGame {
            game_index: index,
            world: 0x0A00 + u64::from(index),
            mover: PlayerId(0),
            decision_seed: 0x0D00 + u64::from(index),
            opening_digest: format!("opening-{index}"),
            placements: vec!["Bank".to_string(), "CopperMine".to_string()],
            president: Some(president.to_string()),
            purchase: purchase.to_string(),
            moves: 5,
            game_over: false,
        };
        let probe = OpeningProbe {
            master: 0x0A3E_0000_0000_0003,
            games: 4,
            subject_echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
            rows: vec![
                row(0, "LaboratoryA", "Laboratory"),
                row(1, "LaboratoryA", "Laboratory"),
                row(2, "Bank", "Bank"),
                row(3, "PortA", "Port"),
            ],
        };
        let rendered = render_opening(&probe);

        // The distribution, tallied and shared.
        assert!(rendered.contains("Bank+CopperMine"), "no placement key:\n{rendered}");
        assert!(rendered.contains("LaboratoryA"), "no president area:\n{rendered}");
        assert!(rendered.contains("0.5000"), "no share for the 2-of-4 cell:\n{rendered}");

        // The lab-rush share and its interval, through the crate's OWN Wilson
        // helper so this figure means what every other rate in the repo means.
        let wilson = wilson_95(2, 4);
        assert!(rendered.contains("lab rush"), "no lab-rush line:\n{rendered}");
        assert!(
            rendered.contains(&format!("{:.4}", wilson.lo))
                && rendered.contains(&format!("{:.4}", wilson.hi)),
            "the Wilson interval is missing:\n{rendered}",
        );

        // The setup path, printed rather than assumed.
        assert!(
            rendered.contains(OPENING_SETUP_PATH),
            "the setup-path line is missing:\n{rendered}",
        );
        assert!(rendered.contains(&probe.subject_echo), "the subject echo is missing:\n{rendered}");
    }

    /// ONE cell with the SHIPPED subject — the probe's real question is about
    /// `lookahead-v1`, and a machinery-only pin at a rung would leave the
    /// expensive path unexercised. One game, one turn (~3s debug budget).
    #[test]
    fn one_opening_game_runs_the_shipped_lookahead_subject() {
        let rules = Rules::standard();
        let probe = run_opening(OPENING_MASTER, 1, Contestant::LOOKAHEAD_V1_DEFAULT, &rules)
            .expect("the shipped subject plays an opening");
        assert_eq!(probe.rows.len(), 1);
        assert_eq!(probe.subject_echo, Contestant::LOOKAHEAD_V1_DEFAULT.echo());
        assert!(probe.rows[0].moves > 0, "the shipped subject played no move");
        let rendered = render_opening(&probe);
        assert!(rendered.contains(&Contestant::LOOKAHEAD_V1_DEFAULT.echo()), "{rendered}");
    }

    // =======================================================================
    // Phase C, T6 — USAGE and the unchanged surfaces
    // =======================================================================

    /// USAGE documents every NEW verb, flag and vocabulary: the `gen:` root form
    /// and its predicate names, the four family ids, the two blocks, the two
    /// arms, and the two master rules a caller cannot infer from a flag list.
    #[test]
    fn the_usage_documents_the_generated_roots_the_families_and_the_opening_probe() {
        assert!(USAGE.contains("gen:<predicate>:<master>:<ordinal>"), "USAGE has no gen: form");
        for predicate in RootPredicate::ALL {
            assert!(
                USAGE.contains(predicate.as_str()),
                "USAGE does not name the predicate `{}`",
                predicate.as_str(),
            );
        }
        for decl in families() {
            assert!(USAGE.contains(decl.id), "USAGE does not name family `{}`", decl.id);
        }
        for block in BLOCK_NAMES {
            assert!(USAGE.contains(block), "USAGE does not name block `{block}`");
        }
        for arm in ARM_NAMES {
            assert!(USAGE.contains(arm), "USAGE does not name arm `{arm}`");
        }
        assert!(
            USAGE.contains("recorded-before-use"),
            "USAGE does not state the pilot master rule",
        );
        assert!(
            USAGE.contains("REFUSED"),
            "USAGE does not state that a confirmatory block refuses a caller's master",
        );
        // THE TWO SIZINGS (the freeze wave): the page a caller reads before
        // typing the command names the K each block runs at.
        assert!(
            USAGE.contains(&format!("K = {PILOT_ADMISSION_K}"))
                && USAGE.contains(&format!("K = {FROZEN_ADMISSION_K}")),
            "USAGE does not name both admission K values:\n{USAGE}",
        );
    }

    /// `list` is UNCHANGED — byte for byte, on a cell-owned registry. The
    /// grammar grew two verbs and a root form; the report a reader already knows
    /// did not move.
    #[test]
    fn the_list_report_is_byte_identical_to_its_golden() {
        let anchor = TempRoot::make("list-golden");
        let entries = [entry("golden", "nothing.json", 3, PlayerId(1), None)];
        let rendered = run_with(&args(&["list"]), &entries, &anchor.0).expect("list always answers");
        let expected = format!(
            "=== driven registry ===\n  \
             1 entries, resolved against {}\n  \
             a MISSING record is a MISS and never a failure: the session roots live in gitignored \
             saves\n\n  \
             golden  [MISSING]\n      \
             path    nothing.json\n      \
             event   3   seat p1   decision seed 0x0dec151000000001\n      \
             move    (none recorded)\n      \
             note    a cell-owned fixture",
            anchor.0.display(),
        );
        assert_eq!(rendered, expected);
    }

    // =======================================================================
    // V4 step 2 — the capacity read and the BURN-RULE guard (spec §6)
    // =======================================================================

    /// The V4 step-2 DEV-SMOKE master, the declared 9999-convention throwaway.
    /// Spec §6 is FREEZE-THEN-DRAW, so no cell here may carry a recorded one.
    const V4_MASTER: u64 = 0x0401_2026_0821_9999;

    /// `capacity <predicate>` parses, takes the master everything else does, and
    /// bounds its walk.
    #[test]
    fn the_capacity_subcommand_parses_a_predicate_a_master_and_a_bound() {
        assert!(SUBCOMMANDS.contains(&"capacity"), "capacity is not in the subcommand list");

        for predicate in RootPredicate::ALL {
            let parsed = parse(&args(&["capacity", predicate.as_str(), "--master", "0x1"]))
                .unwrap_or_else(|e| panic!("capacity {} must parse: {e}", predicate.as_str()));
            match parsed {
                Command::Capacity { predicate: got, master, stop_after } => {
                    assert_eq!(got, predicate);
                    assert_eq!(master, 1);
                    assert_eq!(stop_after, None, "an absent bound is the whole ecology");
                }
                other => panic!("capacity parsed as {other:?}"),
            }
        }

        match parse(&args(&["capacity", "placement-fork-same-count", "--master", "0x2", "--stop-after", "16"]))
            .expect("a bounded capacity parses")
        {
            Command::Capacity { stop_after, .. } => assert_eq!(stop_after, Some(16)),
            other => panic!("capacity parsed as {other:?}"),
        }
    }

    /// Every way a `capacity` command can be wrong refuses on its own terms,
    /// carrying USAGE.
    #[test]
    fn a_malformed_capacity_command_refuses_naming_what_is_wrong() {
        let cases: [(&[&str], &str); 5] = [
            // No predicate.
            (&["capacity", "--master", "0x1"], "exactly ONE"),
            // Two predicates.
            (&["capacity", "trigger-round-not-final", "achievement-claim-available", "--master", "0x1"], "exactly ONE"),
            // An unknown predicate — and the refusal names the vocabulary.
            (&["capacity", "final-round", "--master", "0x1"], "final-round-already-triggered"),
            // No master.
            (&["capacity", "trigger-round-not-final"], "NO DEFAULT"),
            // A zero bound measures nothing.
            (&["capacity", "trigger-round-not-final", "--master", "0x1", "--stop-after", "0"], "measures nothing"),
        ];
        for (line, needle) in cases {
            let err = parse(&args(line)).unwrap_err();
            assert!(err.contains(needle), "{line:?}: the refusal hides {needle:?}: {err}");
            assert!(err.contains(USAGE), "{line:?}: the refusal drops USAGE: {err}");
        }
    }

    /// A `capacity` run reports the count, the ecology, the ordinal-to-game map
    /// and the two band verdicts — and reads NO root table on the way.
    ///
    /// Run on the CHEAP rung ecology deliberately: the report's SHAPE is what a
    /// cell can check, and a V4 predicate would buy the same shape for minutes
    /// of lookahead scan.
    #[test]
    fn a_capacity_run_reports_the_map_and_never_reads_a_root_table() {
        let anchor = TempRoot::make("capacity-run");
        let rendered = run_with(
            &args(&[
                "capacity",
                "final-round-already-triggered",
                "--master",
                &format!("{V4_MASTER:#018x}"),
                "--stop-after",
                "4",
            ]),
            &[],
            &anchor.0,
        )
        .expect("a bounded rung capacity walk completes");

        assert!(rendered.contains("=== driven capacity ==="), "no header: {rendered}");
        assert!(rendered.contains("final-round-already-triggered"), "the predicate is missing: {rendered}");
        assert!(rendered.contains(&format!("{V4_MASTER:#018x}")), "the master is missing: {rendered}");
        assert!(rendered.contains(GENERATED_ECOLOGY_V1), "the ecology is missing: {rendered}");
        assert!(rendered.contains("TRUNCATED"), "a bounded walk must say so: {rendered}");
        assert!(rendered.contains("ordinal -> generated game"), "no map section: {rendered}");
        let mapped: Vec<&str> = rendered
            .lines()
            .filter(|line| line.trim_start().starts_with("ordinal ") && line.contains("game "))
            .collect();
        assert_eq!(
            mapped.len(),
            4,
            "the map does not carry one line per measured ordinal: {rendered}",
        );
        assert!(
            rendered.contains("development   vs escrow"),
            "the band verdicts are missing: {rendered}",
        );
        assert!(
            rendered.contains("NO ROOT TABLE"),
            "the report does not say what it did NOT do: {rendered}",
        );
    }

    /// **THE BURN-RULE GUARD.** Spec §6 fences every read at ordinal >= 8 of the
    /// two banded predicates until step 10, and the fence is a REFUSAL rather
    /// than a comment: the `gen:` grammar reaches an escrow ordinal, so nothing
    /// but a guard stops a copy-pasted command from burning one.
    #[test]
    fn an_escrow_ordinal_of_a_banded_predicate_refuses_on_every_root_taking_subcommand() {
        for predicate in V4_BANDED_PREDICATES {
            let spec = format!("gen:{}:0x1:8", predicate.as_str());
            for line in [
                vec!["table", &spec, "--decision-seed", "9"],
                vec!["run", &spec, "--master", "0x1"],
                vec!["adjudicate", &spec, "--master", "0x1"],
                vec!["exact", &spec],
            ] {
                let err = parse(&args(&line)).unwrap_err();
                assert!(err.contains("BURN RULE"), "{line:?}: the refusal does not name the rule: {err}");
                assert!(err.contains(predicate.as_str()), "{line:?}: the refusal hides the predicate: {err}");
                assert!(err.contains("--escrow-read"), "{line:?}: the refusal hides the way through: {err}");
            }

            // The band's FLOOR is 8 and ordinal 7 is the development extension:
            // the guard must not fence a development read.
            let development = format!("gen:{}:0x1:7", predicate.as_str());
            parse(&args(&["table", &development, "--decision-seed", "9"]))
                .unwrap_or_else(|e| panic!("a development ordinal must pass the guard: {e}"));

            // Past the declared band is still >= 8, and the burn rule says >= 8.
            let past = format!("gen:{}:0x1:99", predicate.as_str());
            assert!(
                parse(&args(&["table", &past, "--decision-seed", "9"])).is_err(),
                "an ordinal past the escrow band escaped the >= 8 fence",
            );
        }
    }

    /// The guard is SCOPED to the two banded predicates: family blocks run
    /// confirmatory ordinals 4..28, so fencing ordinal >= 8 in general would
    /// refuse the frozen blocks' own roots.
    #[test]
    fn the_burn_guard_leaves_the_unbanded_predicates_alone() {
        for predicate in RootPredicate::ALL {
            if V4_BANDED_PREDICATES.contains(&predicate) {
                continue;
            }
            let spec = format!("gen:{}:0x1:27", predicate.as_str());
            parse(&args(&["table", &spec, "--decision-seed", "9"])).unwrap_or_else(|e| {
                panic!("{} is unbanded and must not be fenced: {e}", predicate.as_str())
            });
        }
    }

    /// `--escrow-read` is an ACKNOWLEDGEMENT and not a switch: it takes the
    /// ordinal being burned, must match it, and is refused where no burn is
    /// happening.
    #[test]
    fn the_escrow_acknowledgement_must_name_the_ordinal_it_is_burning() {
        let spec = format!("gen:{}:0x1:11", RootPredicate::PlacementForkSameCount.as_str());

        // The MATCHING acknowledgement is the way through.
        match parse(&args(&["table", &spec, "--decision-seed", "9", "--escrow-read", "11"]))
            .expect("a matching acknowledgement passes the guard")
        {
            Command::Table { root, .. } => assert_eq!(
                root,
                RootRef::Generated {
                    predicate: RootPredicate::PlacementForkSameCount,
                    master: 1,
                    ordinal: 11,
                },
            ),
            other => panic!("table parsed as {other:?}"),
        }

        // A MISMATCH is a refusal — a flag copied off another command cannot
        // unlock this one.
        let err = parse(&args(&["table", &spec, "--decision-seed", "9", "--escrow-read", "8"]))
            .expect_err("a mismatched acknowledgement must refuse");
        assert!(err.contains("11"), "the refusal hides the ordinal asked for: {err}");
        assert!(err.contains("8"), "the refusal hides the ordinal acknowledged: {err}");

        // A non-numeric acknowledgement refuses as one.
        assert!(parse(&args(&["table", &spec, "--decision-seed", "9", "--escrow-read", "yes"]))
            .is_err());

        // And the flag is refused where NO burn is happening, so it cannot
        // become a habit that is already typed when the burn arrives.
        let development =
            format!("gen:{}:0x1:2", RootPredicate::PlacementForkSameCount.as_str());
        let err = parse(&args(&["table", &development, "--decision-seed", "9", "--escrow-read", "2"]))
            .expect_err("the flag must refuse where there is nothing to acknowledge");
        assert!(err.contains("not happening"), "the refusal does not say why: {err}");

        let fixture = parse(&args(&["run", "g1-e113-tiemass", "--master", "0x1", "--escrow-read", "8"]));
        assert!(fixture.is_err(), "the flag was accepted on a registry root");
    }
}
