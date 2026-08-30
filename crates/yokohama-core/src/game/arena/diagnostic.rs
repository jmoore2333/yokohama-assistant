//! The V4 step-11 DIAGNOSIS WAVE's read surfaces (W-D1).
//!
//! **What this module is for, stated first because it constrains everything
//! below.** The V4 confirmatory field returned INVALID RUN: the candidate's own
//! self-null arm hit `HONEST_MAX_TURNS`, the shared non-termination guard, and
//! the arena refuses a WHOLE cell at the first capped game by design (a skipped
//! game would read as a different N). Two things follow from that design, and
//! both are what this module answers:
//!
//!   1. a capped game leaves NO evidence — `cell failed: turn cap reached at
//!      turn 10001 (world seed 0x…)` names a world and nothing about what the
//!      world was doing;
//!   2. no cell can COUNT deadlocks — it reports games-until-first-hit and
//!      stops.
//!
//! **NOTHING HERE MEASURES STRENGTH.** No function in this module computes or
//! renders a win share, a θ̂, a seat-0 win count, a VP margin or an interval of
//! any contestant's share, and [`render_cap_rate`] leads with a printed label
//! saying so. That is a DESIGN constraint of the diagnosis wave, not an
//! oversight: the field's own figures are sealed, and a diagnostic that quietly
//! produced a comparable number would put an uncounted read inside every later
//! reading. The one interval this module computes is on a CAP RATE — a health
//! measurement of the harness, not of a player.
//!
//! **Seating is the cell's, not a second transcription.** [`run_world`] resolves
//! its seating through [`unit_for`] and keys every agent through
//! [`agent_stream_seed`] exactly as `stats::run_cell_emitting_timed` does, so a
//! cell's capped game reproduces byte-for-byte from `(seed, arrangement, x, y)`
//! — which is the whole point of surface (A). [`run_cap_rate`] then plays worlds
//! through [`run_world`] rather than re-deriving the loop, so the two surfaces
//! cannot disagree about who sat where.

use super::stats::X_SLOT;
use super::{
    agent_stream_seed, require_supported_player_count, run_arena_game_diagnostic_capped, unit_for,
    ArenaAgent, ArenaError, ArenaGameStats, CellSpec, Contestant, DiagnosticGame,
    DiagnosticOutcome, NamedIdentity, SeedScheduleKind,
};
use crate::game::agent::HONEST_MAX_TURNS;
use crate::game::metrics::{trigger_name, wilson_95, TriggerTally, WilsonInterval};
use crate::game::record::{replay_from, state_at, GameRecord, RecordedEvent};
use crate::game::rules::Rules;
use crate::game::seeds::{SeedSchedule, WorldSeed};
use crate::game::semantics::state_digest;
use crate::types::{GameState, Move, PlayerId, TurnPhase};

/// The label [`render_cap_rate`] leads with — registered VERBATIM in the
/// step-11 diagnosis-wave ledger row before this code existed, and pinned here
/// so a later edit cannot soften it into something a reader might mistake for a
/// strength figure's caveat.
pub const CAP_RATE_LABEL: &str =
    "DIAGNOSTIC — cap-rate instrument; no strength figure is computed here";

/// The largest period [`move_period`] looks for (the wave's registered bound).
pub const MAX_DETECTED_PERIOD: usize = 64;

/// How many times a candidate period must repeat before it is reported.
///
/// FOUR, registered: two repetitions of a `p`-block is just "the last 2p moves
/// happen to mirror", which fires constantly on ordinary play (two consecutive
/// identical `EndAdditionalPhase`s already satisfy p = 1 at two repetitions).
/// Four is the wave's declared evidence bar for calling a tail a loop; it is a
/// DESCRIPTION of the tail, never a proof that the game cannot progress.
pub const PERIOD_REPETITIONS: usize = 4;

/// The smallest `p` in `1..=`[`MAX_DETECTED_PERIOD`] for which the last
/// `PERIOD_REPETITIONS · p` applied moves are `p`-periodic, or `None`.
///
/// SMALLEST, deliberately: a 2-periodic tail is also 4-periodic and 6-periodic,
/// and reporting the largest fit would describe a two-move loop as a six-move
/// one. `None` covers both "no period fits" and "the sequence is shorter than
/// any window" — the caller prints `period=none` for either, because a tail too
/// short to carry four repetitions is not evidence of aperiodicity.
///
/// Move EQUALITY is the whole comparison. Two `PlaceThreeAreas` into different
/// areas are different moves and break the period, which is the intended
/// reading: a loop that walks its assistants around is not the loop the wave is
/// looking for.
pub fn move_period(moves: &[Move]) -> Option<usize> {
    for p in 1..=MAX_DETECTED_PERIOD {
        let window = PERIOD_REPETITIONS * p;
        if moves.len() < window {
            // The window only grows with p, so nothing longer can fit either.
            break;
        }
        let tail = &moves[moves.len() - window..];
        if tail[p..].iter().zip(tail.iter()).all(|(later, earlier)| later == earlier) {
            return Some(p);
        }
    }
    None
}

/// The runner's `DIAGNOSTIC SUBCOMMANDS` help section.
///
/// It lives in CORE rather than in `examples/arena.rs` for
/// [`named_identities_usage`]'s reason, one level down: the two subcommands'
/// contract — above all the no-strength-figure rule — is a property of the code
/// here, and help text that restates a property from another crate is help text
/// free to go stale about it.
pub const DIAGNOSTIC_USAGE: &str =
    "  DIAGNOSTIC SUBCOMMANDS (V4 step-11 diagnosis wave) -- 2P read surfaces. NEITHER
  computes or prints a win share, a theta_hat, a seat-0 count, a VP margin or an
  interval of any contestant's share: they exist to show what a CAPPED game was
  doing and how often one happens, and a diagnostic that quietly produced a
  comparable figure would sit inside every later reading of a counted run.

  world <x> <y> <world-seed> [--arrangement 0|1|both] [--tail <K>]
        [--invariants|--no-invariants]
      Play ONE world seated EXACTLY as `cell` seats it -- same seatings table,
      same agent stream keying -- so a cell's capped game reproduces byte-for-byte
      from (seed, arrangement, x, y). <world-seed> parses like --master (0x hex or
      decimal); it is a WORLD, not a master, and no schedule is drawn from it.
      Prints, per arrangement: the seating and both echoes; the outcome (TERMINATED
      with its trigger and final standings, or CAPPED at turn N); moves applied per
      seat; the TAIL -- the last K turns' applied moves, one line each with turn,
      mover and phase; a PERIOD line (the smallest p <= 64 whose block repeats four
      times over the tail of applied moves, or none); and a compact END-STATE
      summary. Defaults: --arrangement both, --tail 40.

  caprate <x> <y> <units> [--master <u64> | --os-entropy]
          [--invariants|--no-invariants]
      Play <units> worlds from the master's schedule exactly as a cell would (both
      arrangements per world) but COUNT capped games instead of refusing the cell
      at the first one. Prints the cap count with a 95% Wilson interval, the list
      of capped (seed, arrangement) pairs so each reproduces under `world`, the
      end-trigger histogram over COMPLETED games and their mean/max turns. Any
      other driver failure -- sampler refusal, illegal pick, driver bug -- still
      REFUSES the whole run: only a turn cap is counted. Leads with the label
      `DIAGNOSTIC — cap-rate instrument; no strength figure is computed here`.
";

/// The runner's `NAMED IDENTITIES` help section, BUILT FROM
/// [`NamedIdentity::ALL`].
///
/// The hand-written block this replaces went stale the moment a third arm
/// joined the grammar: `Contestant::from_str_name`'s refusal listed
/// `v4-step9-draft1` while the help text above it listed two names. Composing
/// the block from the same array the grammar reads makes that drift
/// unrepresentable — and the per-arm blurb below is an exhaustive match, so a
/// fourth arm is a compile error rather than a silently undocumented name.
pub fn named_identities_usage() -> String {
    // NOTE: no `\` line-continuation at the opening quote — it would eat the
    // two-space indent that puts this section at the same level as `RUNGS` and
    // `EVALUATOR CONTESTANTS` in the runner's help.
    let mut text = String::from(
        "  NAMED IDENTITIES (V4 step 8) -- the one exception to the line above, and it is
  an exception by NAME rather than by knob: a named identity is a distinct
  contestant whose table is part of what its name means, never a `weights=` knob
  on an arm above.
",
    );
    for arm in NamedIdentity::ALL {
        for (i, line) in blurb(arm).lines().enumerate() {
            if i == 0 {
                text.push_str(&format!("    {:<36} {line}\n", arm.name()));
            } else {
                text.push_str(&format!("    {:<36} {line}\n", ""));
            }
        }
    }
    text.push_str(
        "      All are KNOBLESS: they run one transcribed knob set (b=4, k=1, m=8, uncapped)
      so a cell pitting two of them against each other varies the TABLE and nothing
      else. Their echoes carry the name and the resolved `weights=` identity. They
      are ARENA arms: examples/eval_probe selects its table with --weights instead,
      the driven harness refuses them, and they are not CLI bot seats.
      Unknown contestant names, unknown knobs and unknown knob VALUES are errors.
",
    );
    text
}

/// The per-arm help line(s). An EXHAUSTIVE match, which is the second half of
/// [`named_identities_usage`]'s anti-staleness: the loop guarantees every arm is
/// listed, and this guarantees a new arm cannot be listed with no description.
fn blurb(arm: NamedIdentity) -> &'static str {
    match arm {
        NamedIdentity::V2Padded => {
            "lookahead at the shipped knobs on WeightTable::V2\n\
             (the shipped table, padded) -- the same seat\n\
             `lookahead-v1` plays, said as an identity"
        }
        NamedIdentity::V2RefitPadded => {
            "the §8 B-cell REFIT of the sixteen incumbents\n\
             (no candidate column admitted); NOT a shipped table"
        }
        NamedIdentity::V4Step9Draft1 => {
            "the V4 step-9 CANDIDATE, draft 1's table -- the first\n\
             arm whose candidate slots are nonzero. NOT a shipped\n\
             table; the wire name is proposed, not spec-registered"
        }
    }
}

// ---------------------------------------------------------------------------
// Surface (A): one world, seated as a cell seats it
// ---------------------------------------------------------------------------

/// Which world, under which two contestants, in which arrangement.
///
/// Shaped like [`CellSpec`] on purpose: a cell is this plus a unit count and a
/// schedule, and the seating rule below is the cell's own.
#[derive(Clone, Debug)]
pub struct WorldSpec {
    /// The contestant under test — [`X_SLOT`]'s occupant, whichever seat that is
    /// in this arrangement.
    pub x: Contestant,
    pub y: Contestant,
    /// A WORLD seed, not a master: no schedule is drawn from it.
    pub seed: WorldSeed,
    pub players: u8,
    /// Index into [`unit_for`]'s seatings — at 2P, 0 is `[X, Y]` and 1 is
    /// `[Y, X]`.
    pub arrangement: u8,
}

/// One world played under the honest chain, plus everything a reader needs to
/// know which game it was.
#[derive(Debug)]
pub struct WorldRun {
    pub seed: WorldSeed,
    pub players: u8,
    pub arrangement: u8,
    /// The seat contestant X occupied in this arrangement.
    pub x_seat: PlayerId,
    /// [`Contestant::echo`] for each side — the runner's own echo composer, so
    /// the header and a cell's report echo cannot spell an identity two ways.
    pub x_echo: String,
    pub y_echo: String,
    /// The turn guard this run played under. Recorded so the outcome line can
    /// say whether a cap was the SHIPPED guard or a diagnostic value.
    pub turn_cap: u32,
    pub game: DiagnosticGame,
}

/// Play ONE world under [`HONEST_MAX_TURNS`], seated exactly as a cell seats it.
pub fn run_world(
    spec: &WorldSpec,
    rules: &Rules,
    invariants: bool,
) -> Result<WorldRun, ArenaError> {
    run_world_capped(spec, rules, invariants, HONEST_MAX_TURNS)
}

/// [`run_world`] with the turn guard as a parameter — for tests, which must
/// reach a capped game in milliseconds.
pub fn run_world_capped(
    spec: &WorldSpec,
    rules: &Rules,
    invariants: bool,
    max_turns: u32,
) -> Result<WorldRun, ArenaError> {
    require_supported_player_count(spec.players)?;
    let unit = unit_for(spec.players, spec.seed);
    let seating = unit
        .seatings
        .get(spec.arrangement as usize)
        .ok_or_else(|| {
            ArenaError::Driver(format!(
                "arrangement {} does not exist at {}P (the unit has {})",
                spec.arrangement,
                spec.players,
                unit.seatings.len(),
            ))
        })?
        .clone();
    let x_seat = seating
        .iter()
        .position(|&slot| slot == X_SLOT)
        .map(|i| PlayerId(i as u8))
        .ok_or_else(|| {
            ArenaError::Driver(format!(
                "seating {seating:?} has no slot {} to seat contestant X in",
                X_SLOT.0,
            ))
        })?;

    // THE CELL'S OWN FACTORY, minus the explanation handles and the stopwatch a
    // cell folds (neither changes what a seat plays). Every contestant, rung or
    // evaluator, is built through `Contestant::build` on
    // `agent_stream_seed(seed, seat, slot)` — the single keying rule.
    let (x, y) = (spec.x, spec.y);
    let seed = spec.seed;
    let mut make_agent = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
        let slot = seating[seat.0 as usize];
        let contestant = if slot == X_SLOT { x } else { y };
        let (agent, _handle) = contestant.build(rules, agent_stream_seed(seed, seat, slot));
        agent
    };
    let game = run_arena_game_diagnostic_capped(
        seed,
        spec.players,
        rules,
        &mut make_agent,
        invariants,
        max_turns,
    )?;

    Ok(WorldRun {
        seed,
        players: spec.players,
        arrangement: spec.arrangement,
        x_seat,
        x_echo: x.echo(),
        y_echo: y.echo(),
        turn_cap: max_turns,
        game,
    })
}

/// One applied move with the context it was played in.
///
/// The MOVE comes from the driver's own record; `turn`, `seat` and `phase` are
/// derived by FOLDING that record (see [`annotate`]) — an exact re-derivation of
/// context, never of the moves. The record carries moves, not movers or phases,
/// and the arena's history-free `GameState` keeps no log, so there is nowhere
/// else for the annotation to come from.
#[derive(Clone, Debug)]
pub struct AppliedMove {
    pub turn: u32,
    pub seat: PlayerId,
    pub phase: TurnPhase,
    pub mv: Move,
}

/// Fold a record one event at a time, recording the context BEFORE each event,
/// and hand back the state it folds to.
///
/// O(events): one `apply` per event, the same work the driver already did once.
/// A capped 10,001-turn game is tens of thousands of events, which is a second
/// of a one-world diagnostic and is why this is not done per event index.
fn annotate(
    record: &GameRecord,
    rules: &Rules,
) -> Result<(Vec<AppliedMove>, GameState), ArenaError> {
    let replay_error =
        |e: crate::game::record::ReplayError| ArenaError::Driver(format!("replay: {}", e.message));
    let mut state = state_at(record, 0, rules).map_err(replay_error)?;
    let mut applied = Vec::with_capacity(record.events.len());
    for event in &record.events {
        if let RecordedEvent::Move(mv) = event {
            applied.push(AppliedMove {
                turn: state.turn_number,
                seat: state.current_player,
                phase: state.turn_phase,
                mv: mv.clone(),
            });
        }
        state = replay_from(&state, std::slice::from_ref(event), rules).map_err(replay_error)?;
    }
    Ok((applied, state))
}

/// Render one [`WorldRun`] as the lines the runner prints.
///
/// `tail_turns` is a TURN window, not an event count: the wave reads a loop, and
/// a loop is described by turns.
pub fn render_world(
    run: &WorldRun,
    tail_turns: u32,
    rules: &Rules,
) -> Result<Vec<String>, ArenaError> {
    let (record, capped_state, turns, moves_by_seat) = match &run.game.outcome {
        DiagnosticOutcome::Terminated(stats) => {
            (&stats.record, None, stats.turns, &stats.moves_by_seat)
        }
        DiagnosticOutcome::Capped { turns, record, state, moves_by_seat } => {
            (&**record, Some(&**state), *turns, moves_by_seat)
        }
    };
    let (applied, folded) = annotate(record, rules)?;
    // THE CROSS-CHECK, at render time and not by reference to a test (review
    // S2). Every tail line's turn/seat/phase comes from `folded`'s trajectory
    // while the end-state block comes from the driver on a cap; if the two ever
    // disagreed this report would annotate one game and summarize another,
    // silently. Both digests are in hand here, so it costs one comparison.
    let folded_digest = state_digest(&folded);
    let end_state = match capped_state {
        // On a cap the DRIVER's state is authoritative — it is the state the
        // guard tripped in — and the fold must reproduce it.
        Some(state) => {
            let driver_digest = state_digest(state);
            if folded_digest != driver_digest {
                return Err(ArenaError::Driver(format!(
                    "the record does not fold to the state the driver capped in: fold digest \
                     {folded_digest} vs driver digest {driver_digest} (world {:#018x}, \
                     arrangement {}, {} applied moves) — the tail and the end state would \
                     describe different games",
                    run.seed.0,
                    run.arrangement,
                    applied.len(),
                )));
            }
            state
        }
        // On a completed game the fold IS the end state (`ArenaGameStats` does
        // not carry one), and the record already carries the digest the driver
        // stamped for it.
        None => {
            if let Some(stamped) = &record.terminal_digest {
                if &folded_digest != stamped {
                    return Err(ArenaError::Driver(format!(
                        "the record does not fold to its own stamped terminal digest: fold \
                         digest {folded_digest} vs recorded digest {stamped} (world {:#018x}, \
                         arrangement {}, {} applied moves)",
                        run.seed.0,
                        run.arrangement,
                        applied.len(),
                    )));
                }
            }
            &folded
        }
    };

    let mut lines = Vec::new();
    lines.push(format!(
        "arrangement={} world={:#018x} players={} X=seat{} Y={}",
        run.arrangement,
        run.seed.0,
        run.players,
        run.x_seat.0,
        (0..run.players)
            .filter(|&s| s != run.x_seat.0)
            .map(|s| format!("seat{s}"))
            .collect::<Vec<_>>()
            .join(","),
    ));
    lines.push(format!("  X = {}", run.x_echo));
    lines.push(format!("  Y = {}", run.y_echo));

    match &run.game.outcome {
        DiagnosticOutcome::Terminated(stats) => {
            lines.push(format!(
                "OUTCOME: TERMINATED trigger={} turns={turns}",
                stats.trigger.as_ref().map_or("none", trigger_name),
            ));
            for standing in &stats.result.standings {
                lines.push(format!(
                    "  rank {} seat{} vp={}{}",
                    standing.rank,
                    standing.player.0,
                    standing.breakdown.total,
                    if standing.player == stats.result.winner { "  (winner)" } else { "" },
                ));
            }
        }
        DiagnosticOutcome::Capped { .. } => lines.push(format!(
            "OUTCOME: CAPPED at turn {turns} (turn cap {}{})",
            run.turn_cap,
            if run.turn_cap == HONEST_MAX_TURNS {
                format!(" = HONEST_MAX_TURNS={HONEST_MAX_TURNS}")
            } else {
                format!(", a DIAGNOSTIC value; HONEST_MAX_TURNS={HONEST_MAX_TURNS}")
            },
        )),
    }

    lines.push(format!(
        "moves applied: total={} · {}",
        moves_by_seat.iter().sum::<u64>(),
        moves_by_seat
            .iter()
            .enumerate()
            .map(|(seat, n)| format!("seat{seat}={n}"))
            .collect::<Vec<_>>()
            .join(" · "),
    ));

    // The window ENDS at the last APPLIED move's turn, never at `turns` (review
    // S1). On a cap those differ: `turns` is the turn the guard READ, one past
    // the limit, in which no move was ever applied — so `..={turns}` promised a
    // window one turn wider than the moves it contained. On a completed game the
    // two coincide, and the fallback for a record with no moves at all is
    // `turns` because there is no move to take a turn from.
    // Both ends come from the moves: a window wider than the game would
    // otherwise print `turns 0..=30`, and turn 0 is not a turn.
    let last_move_turn = applied.last().map_or(turns, |step| step.turn);
    let first_move_turn = applied.first().map_or(turns, |step| step.turn);
    let first_tail_turn =
        last_move_turn.saturating_sub(tail_turns.saturating_sub(1)).max(first_move_turn);
    let tail: Vec<&AppliedMove> = applied.iter().filter(|a| a.turn >= first_tail_turn).collect();
    lines.push(format!(
        "tail (turns {first_tail_turn}..={last_move_turn}, {} of {} applied moves):",
        tail.len(),
        applied.len(),
    ));
    for step in &tail {
        lines.push(format!(
            "  turn={} seat={} phase={:?} move={:?}",
            step.turn, step.seat.0, step.phase, step.mv,
        ));
    }

    let sequence: Vec<Move> = applied.iter().map(|a| a.mv.clone()).collect();
    lines.push(match move_period(&sequence) {
        Some(p) => format!(
            "period={p} (the last {} applied moves are {p}-periodic)",
            PERIOD_REPETITIONS * p,
        ),
        None => format!(
            "period=none (no p in 1..={MAX_DETECTED_PERIOD} repeats {PERIOD_REPETITIONS}x)",
        ),
    });

    lines.extend(render_end_state(end_state));
    Ok(lines)
}

/// The compact end-state summary — the state the game stopped in, terminal or
/// not.
fn render_end_state(state: &GameState) -> Vec<String> {
    let mut lines = vec![
        "end state:".to_string(),
        format!(
            "  current_player=seat{} turn_phase={:?} turn={}",
            state.current_player.0, state.turn_phase, state.turn_number,
        ),
    ];
    for player in &state.players {
        let a = &player.assistants;
        lines.push(format!(
            "  seat{}: assistants hand={} warehouse={} board={} (ach {}, church {}, customs {}) \
             · money={} · orders held={} · vp={}",
            player.id.0,
            a.in_hand,
            a.in_warehouse,
            a.on_board.values().map(|&n| u32::from(n)).sum::<u32>(),
            a.on_achievements,
            a.on_church,
            a.on_customs,
            player.money,
            player.order_cards_in_hand.len(),
            player.victory_points,
        ));
    }
    lines.push(format!(
        "  order deck={} · tech deck={} · orders removed={}",
        state.order_deck.len(),
        state.tech_deck.len(),
        state.orders_removed.len(),
    ));
    lines.push(format!("  church: {}", render_track(state.church_track.slots.iter().enumerate().map(
        |(i, slot)| (crate::types::ChurchTrack::index_to_faith(i), slot.map(|s| s.player)),
    ))));
    lines.push(format!("  customs: {}", render_track(state.customs_track.slots.iter().enumerate().map(
        |(i, slot)| (crate::types::CustomsTrack::index_to_space(i), slot.map(|s| s.player)),
    ))));
    lines
}

/// One track's occupancy: every occupied space and who holds it. The 2P dummy is
/// NAMED as the dummy — it is a real scoring occupant, and reading it as a seat
/// index would misread the board.
fn render_track(spaces: impl Iterator<Item = (u8, Option<PlayerId>)>) -> String {
    let occupied: Vec<String> = spaces
        .filter_map(|(label, occupant)| {
            occupant.map(|p| {
                if p == PlayerId::DUMMY {
                    format!("{label}=DUMMY")
                } else {
                    format!("{label}=seat{}", p.0)
                }
            })
        })
        .collect();
    if occupied.is_empty() {
        "empty".to_string()
    } else {
        occupied.join(" · ")
    }
}

// ---------------------------------------------------------------------------
// Surface (B): the cap-rate instrument
// ---------------------------------------------------------------------------

/// One capped game, named so it reproduces under [`run_world`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CappedWorld {
    pub seed: WorldSeed,
    pub arrangement: u8,
    pub turns: u32,
}

/// What a cap-rate cell measured.
///
/// **There is deliberately no field here from which a strength figure could be
/// read** — no win count, no unit share, no margin. The type is the first place
/// the wave's labelling rule is enforced; [`render_cap_rate`] is the second.
#[derive(Clone, Debug)]
pub struct CapRateReport {
    pub name: String,
    pub players: u8,
    pub units: u32,
    /// The master the schedule descended from — recorded so the run reproduces.
    pub master: u64,
    /// Games PLAYED: `units × players` at every supported count.
    pub games: u64,
    /// Every capped game, in schedule order.
    pub capped: Vec<CappedWorld>,
    /// End triggers over the COMPLETED games alone — `TriggerTally::observe`'s
    /// own rule (a game that did not complete is not an observation of the
    /// trigger distribution).
    pub completed: TriggerTally,
    /// Mean turns over completed games; `NaN` when none completed, which is the
    /// honest answer and not a zero.
    pub completed_turns_mean: f64,
    pub completed_turns_max: u32,
}

impl CapRateReport {
    pub fn cap_count(&self) -> u64 {
        self.capped.len() as u64
    }

    /// The realized cap rate over the games PLAYED (capped games included in the
    /// denominator — they are the event being counted).
    pub fn rate(&self) -> f64 {
        if self.games == 0 {
            f64::NAN
        } else {
            self.cap_count() as f64 / self.games as f64
        }
    }

    /// The 95% Wilson interval on that rate, through the crate's ONE Wilson
    /// implementation (`metrics::wilson_95`) — never a second one.
    pub fn interval(&self) -> WilsonInterval {
        wilson_95(self.cap_count(), self.games)
    }
}

/// Play `spec.units` worlds as a cell would and COUNT the capped games.
pub fn run_cap_rate(
    spec: &CellSpec,
    rules: &Rules,
    invariants: bool,
) -> Result<CapRateReport, ArenaError> {
    run_cap_rate_capped(spec, rules, invariants, HONEST_MAX_TURNS)
}

/// [`run_cap_rate`] with the turn guard as a parameter — a test forces caps in
/// milliseconds rather than playing 10,000 turns six times.
pub fn run_cap_rate_capped(
    spec: &CellSpec,
    rules: &Rules,
    invariants: bool,
    max_turns: u32,
) -> Result<CapRateReport, ArenaError> {
    require_supported_player_count(spec.players)?;
    if spec.units == 0 {
        return Err(ArenaError::Driver(format!(
            "cap-rate cell {:?} asked for 0 units: N is fixed before the run, and 0 is not a design",
            spec.name,
        )));
    }
    let (mut schedule, master) = match spec.master {
        SeedScheduleKind::Diagnostic(master) => (SeedSchedule::diagnostic(master), master),
        SeedScheduleKind::OsEntropy => SeedSchedule::from_os_entropy(),
    };

    let mut games = 0u64;
    let mut capped: Vec<CappedWorld> = Vec::new();
    let mut completed = TriggerTally::default();
    let mut completed_turns_sum = 0u64;
    let mut completed_turns_max = 0u32;
    let mut completed_games = 0u64;

    for _ in 0..spec.units {
        let seed = schedule.next_world();
        for arrangement in 0..spec.players {
            // Through `run_world_capped`, not a second loop: the two surfaces
            // must not disagree about who sat where, and a capped world named
            // here has to reproduce there.
            let run = run_world_capped(
                &WorldSpec { x: spec.x, y: spec.y, seed, players: spec.players, arrangement },
                rules,
                invariants,
                max_turns,
            )?;
            games += 1;
            match run.game.outcome {
                DiagnosticOutcome::Terminated(stats) => {
                    let stats: ArenaGameStats = *stats;
                    completed.observe(stats.trigger.as_ref());
                    completed_games += 1;
                    completed_turns_sum += u64::from(stats.turns);
                    completed_turns_max = completed_turns_max.max(stats.turns);
                }
                DiagnosticOutcome::Capped { turns, .. } => {
                    capped.push(CappedWorld { seed, arrangement, turns })
                }
            }
        }
    }

    Ok(CapRateReport {
        name: spec.name.clone(),
        players: spec.players,
        units: spec.units,
        master,
        games,
        capped,
        completed,
        completed_turns_mean: if completed_games == 0 {
            f64::NAN
        } else {
            completed_turns_sum as f64 / completed_games as f64
        },
        completed_turns_max,
    })
}

/// Render a [`CapRateReport`] as the lines the runner prints, LABEL FIRST.
///
/// The end-trigger histogram is deliberately NOT here: `examples/arena` already
/// owns one printer for it (`print_triggers`), and a second spelling of a
/// histogram format is a second thing to drift.
pub fn render_cap_rate(report: &CapRateReport) -> Vec<String> {
    let interval = report.interval();
    let mut lines = vec![
        CAP_RATE_LABEL.to_string(),
        // THE RESOLVED master, always — not the `SeedScheduleKind` the caller
        // typed. An `--os-entropy` run's header says the drawn master is
        // "printed below", and this is below; without it that run is
        // unreproducible and the header is false (review M1). CLAUDE.md's arena
        // bullet: "masters are recorded before use".
        format!(
            "master={:#018x} players={} units={} cell={:?}",
            report.master, report.players, report.units, report.name,
        ),
        format!(
            "games={} capped={} rate={:.6} wilson95=[{:.6}, {:.6}]",
            report.games,
            report.cap_count(),
            report.rate(),
            interval.lo,
            interval.hi,
        ),
    ];
    if report.capped.is_empty() {
        lines.push("capped worlds: none".to_string());
    } else {
        lines.push(
            "capped worlds (each reproduces: arena world <x> <y> <seed> --arrangement <arr>):"
                .to_string(),
        );
        for capped in &report.capped {
            lines.push(format!(
                "  seed={:#018x} arrangement={} turns={}",
                capped.seed.0, capped.arrangement, capped.turns,
            ));
        }
    }
    lines.push(format!(
        "completed games={} · turns mean={:.1} max={}",
        report.completed.games(),
        report.completed_turns_mean,
        report.completed_turns_max,
    ));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::arena::{
        run_cell_emitting, CellSpec, Contestant, DiagnosticOutcome, SeedScheduleKind,
    };
    use crate::game::record::GameRecord;
    use crate::game::rules::Rules;
    use crate::game::seeds::SeedSchedule;

    /// A THROWAWAY master on the wave's 9999 convention. It is deliberately NOT
    /// one of the registered diagnostic masters `0x0B0120260831990{1..5}` and
    /// not either scratch master the freeze doc already spent: a test that
    /// played a registered schedule would consume a read the orchestrator has
    /// not taken yet.
    const SCRATCH_MASTER: u64 = 0x0B01_2026_0831_9999;

    fn rungs() -> (Contestant, Contestant) {
        // Rungs, not evaluator arms: these cells play whole 2P games in the
        // per-edit tier, and the seating/keying property under test is the same
        // for every contestant family (`Contestant::build` is the one door).
        (
            Contestant::from_str_name("family-weighted").expect("a shipped rung name"),
            Contestant::from_str_name("retreat-averse").expect("a shipped rung name"),
        )
    }

    fn scratch_spec(name: &str, units: u32) -> CellSpec {
        let (x, y) = rungs();
        CellSpec {
            name: name.to_string(),
            players: 2,
            x,
            y,
            units,
            master: SeedScheduleKind::Diagnostic(SCRATCH_MASTER),
        }
    }

    fn first_world() -> crate::game::seeds::WorldSeed {
        SeedSchedule::diagnostic(SCRATCH_MASTER).next_world()
    }

    /// **Surface (A)'s whole claim.** A cell's game must reproduce byte-for-byte
    /// from `(seed, arrangement, x, y)` — otherwise the tail printed for the
    /// capped world is a tail of some OTHER game. Both arrangements, because the
    /// slot component of `agent_stream_seed` is exactly what makes them differ.
    #[test]
    fn a_world_run_reproduces_the_cell_that_produced_it() {
        let rules = Rules::standard();
        let spec = scratch_spec("wd1-repro", 1);
        let mut from_cell: Vec<(u8, GameRecord)> = Vec::new();
        run_cell_emitting(&spec, &rules, false, &mut |stats, arr| {
            from_cell.push((arr, stats.record.clone()));
            Ok(())
        })
        .expect("the scratch world terminates under both rungs");
        assert_eq!(from_cell.len(), 2, "a 2P unit is the swapped pair");

        let seed = first_world();
        for (arr, cell_record) in &from_cell {
            let run = run_world(
                &WorldSpec { x: spec.x, y: spec.y, seed, players: 2, arrangement: *arr },
                &rules,
                false,
            )
            .expect("the same world, seated the same way");
            assert_eq!(run.x_seat.0, if *arr == 0 { 0 } else { 1 }, "arrangement {arr}'s X seat");
            let DiagnosticOutcome::Terminated(stats) = &run.game.outcome else {
                panic!("the cell completed this game, so the world run must too");
            };
            assert_eq!(
                stats.record.events, cell_record.events,
                "arrangement {arr}: the applied move sequence must be the cell's own",
            );
            assert!(cell_record.terminal_digest.is_some());
            assert_eq!(
                stats.record.terminal_digest, cell_record.terminal_digest,
                "arrangement {arr}: the terminal-state digest",
            );
        }
    }

    /// **Surface (B)'s whole claim.** A cell refuses at the FIRST capped game;
    /// this instrument counts them and keeps going, and names each one so it
    /// reproduces under surface (A).
    #[test]
    fn cap_rate_counts_capped_games_instead_of_refusing_the_run() {
        let rules = Rules::standard();
        let spec = scratch_spec("wd1-caprate-forced", 3);
        // A five-turn cap makes EVERY game cap. That is the point: `run_cell`
        // would have refused after one, and this must report six.
        let report = run_cap_rate_capped(&spec, &rules, false, 5)
            .expect("a capped game is COUNTED here, never a refusal");
        assert_eq!(report.games, 6, "three units, both arrangements each");
        assert_eq!(report.cap_count(), 6);
        assert_eq!(report.capped.len(), 6);
        assert_eq!(report.completed.games(), 0, "no game completed, so no trigger is observed");
        assert_eq!(report.completed_turns_max, 0);
        assert!(report.completed_turns_mean.is_nan(), "a mean over zero games is not a number");
        assert_eq!(report.rate(), 1.0);

        let arrangements: Vec<u8> = report.capped.iter().map(|c| c.arrangement).collect();
        assert_eq!(arrangements, vec![0, 1, 0, 1, 0, 1], "both arrangements of every world");
        let mut schedule = SeedSchedule::diagnostic(SCRATCH_MASTER);
        for pair in report.capped.chunks(2) {
            let seed = schedule.next_world();
            assert!(
                pair.iter().all(|c| c.seed == seed),
                "the capped worlds must be the master's schedule, in order, so each reproduces",
            );
        }
        assert!(report.capped.iter().all(|c| c.turns > 5), "the cap fires past the limit");

        let interval = report.interval();
        assert!(interval.lo > 0.0, "6 of 6 has a lower bound above zero, got {}", interval.lo);
        assert_eq!(interval.hi, 1.0, "an all-capped run's Wilson upper bound is 1");
    }

    /// The other end of the same instrument: healthy worlds count ZERO, and the
    /// completed games are what the trigger histogram and the turn figures are
    /// taken over.
    #[test]
    fn cap_rate_over_healthy_worlds_counts_zero_and_tallies_only_completed_games() {
        let rules = Rules::standard();
        let spec = scratch_spec("wd1-caprate-healthy", 1);
        let report = run_cap_rate(&spec, &rules, false).expect("the rungs finish these worlds");
        assert_eq!(report.games, 2);
        assert_eq!(report.cap_count(), 0);
        assert!(report.capped.is_empty());
        assert_eq!(report.completed.games(), 2, "every completed game is observed");
        assert_eq!(report.rate(), 0.0);
        assert!(report.completed_turns_max > 0);
        assert!(report.completed_turns_mean > 0.0);
        let interval = report.interval();
        assert_eq!(interval.lo, 0.0, "zero successes pins the lower bound at zero");
        assert!(interval.hi > 0.0 && interval.hi < 1.0, "and leaves a real upper bound");
    }

    /// **The no-strength-figure pin.** The diagnosis wave's labelling rule, made
    /// executable: this instrument may not produce anything a reader could quote
    /// as a comparison between the two contestants.
    #[test]
    fn the_cap_rate_render_carries_no_strength_figure_of_any_kind() {
        let rules = Rules::standard();
        let spec = scratch_spec("wd1-caprate-render", 1);
        let report = run_cap_rate_capped(&spec, &rules, false, 5).expect("counted, not refused");
        let lines = render_cap_rate(&report);
        assert_eq!(lines.first().map(String::as_str), Some(CAP_RATE_LABEL), "the label leads");
        let text = lines.join("\n");
        for banned in [
            "theta_hat", "θ̂", "unit shares", "unit variance", "seat0 wins", "VP margin",
            "win share", "Var_pair", "bootstrap",
        ] {
            assert!(
                !text.contains(banned),
                "the cap-rate instrument printed {banned:?} — it computes no strength figure \
                 by design, and a reader must not be able to quote one out of it",
            );
        }
        assert!(text.contains("games="), "the denominator");
        assert!(text.contains("capped="), "the numerator");
        assert!(text.contains("wilson95="), "the interval on the CAP RATE");
        for capped in &report.capped {
            assert!(
                text.contains(&format!("{:#018x}", capped.seed.0)),
                "every capped world is named so it reproduces under `world`",
            );
        }
    }

    /// **W-D1 review M1.** A run whose master is not printed is a run nobody can
    /// repeat. CLAUDE.md's arena bullet is binding — *"masters are recorded
    /// before use"* — and `cell` honours it through its report echo.
    #[test]
    fn the_cap_rate_render_names_the_master_its_worlds_came_from() {
        let rules = Rules::standard();
        let spec = scratch_spec("wd1-caprate-master", 1);
        let report = run_cap_rate_capped(&spec, &rules, false, 5).expect("counted, not refused");
        assert_eq!(report.master, SCRATCH_MASTER);
        let text = render_cap_rate(&report).join("\n");
        assert!(
            text.contains(&format!("master={SCRATCH_MASTER:#018x}")),
            "the resolved master must be IN the render, not only in a header the library \
             does not control:\n{text}",
        );
    }

    /// The case M1 was actually about: `--os-entropy` prints
    /// `"os-entropy (drawn per run, printed below)"` in the header, and nothing
    /// below printed it. The second assertion is the one that matters — the
    /// master rendered must be the one the schedule descended from, so the run
    /// reproduces as a diagnostic.
    #[test]
    fn an_os_entropy_cap_rate_render_names_the_master_it_drew() {
        let rules = Rules::standard();
        let (x, y) = rungs();
        let spec = CellSpec {
            name: "wd1-caprate-os-entropy".to_string(),
            players: 2,
            x,
            y,
            units: 1,
            master: SeedScheduleKind::OsEntropy,
        };
        // A five-turn cap keeps this cheap AND makes the outcome deterministic
        // (every game caps) even though the master is drawn.
        let report = run_cap_rate_capped(&spec, &rules, false, 5).expect("counted, not refused");
        let text = render_cap_rate(&report).join("\n");
        assert!(
            text.contains(&format!("master={:#018x}", report.master)),
            "an os-entropy run must print the master it drew:\n{text}",
        );
        let mut schedule = SeedSchedule::diagnostic(report.master);
        assert_eq!(
            report.capped[0].seed,
            schedule.next_world(),
            "the printed master must be the one the worlds came from — otherwise it is a \
             number, not a reproduction key",
        );
    }

    /// **What surface (A) exists to print.** A CAPPED game is the case the arena
    /// could not show anyone: the tail with its turn/mover/phase annotation, the
    /// period line, and the end-state summary all have to render off a partial
    /// record and a non-terminal state.
    #[test]
    fn a_capped_world_renders_its_outcome_tail_period_and_end_state() {
        let rules = Rules::standard();
        let (x, y) = rungs();
        let run = run_world_capped(
            &WorldSpec { x, y, seed: first_world(), players: 2, arrangement: 0 },
            &rules,
            false,
            30,
        )
        .expect("a cap is an outcome here");
        let lines = render_world(&run, 5, &rules).expect("a partial record renders");
        let text = lines.join("\n");
        assert!(text.contains("arrangement=0"), "the header names the arrangement");
        assert!(text.contains("CAPPED at turn "), "the outcome line");
        assert!(
            text.contains("HONEST_MAX_TURNS"),
            "the outcome line must say what the cap in force was relative to the shipped guard",
        );
        assert!(text.contains("moves applied:"), "moves per seat");
        assert!(text.contains("tail ("), "the tail block");
        assert!(
            lines.iter().any(|l| l.contains("turn=") && l.contains("seat=") && l.contains("phase=")
                && l.contains("move=")),
            "every tail line carries turn, mover and phase beside the move",
        );
        assert!(lines.iter().any(|l| l.starts_with("period=")), "the period line, always present");
        assert!(text.contains("current_player="), "the end-state summary");
        assert!(text.contains("turn_phase="));
        assert!(text.contains("order deck="));
        assert!(text.contains("church:"));
        assert!(text.contains("customs:"));
        assert!(text.contains("money="));
    }

    /// **W-D1 review S3.** The rung cell above proves the SEATING and the
    /// KEYING; it does not prove the wrapper asymmetry is inert. The cell path
    /// wraps every seat in `TimedAgent` and clones an `ExplainHandle` out before
    /// boxing, `run_world` does neither, and whether a handle comes back is
    /// contestant-family-dependent — so the arms the D1/D2 reads actually use
    /// need their own pin rather than an argument by analogy.
    ///
    /// ONE unit (four games: two in the cell, two re-run here). The named arms
    /// are lookahead searches, so this is the most expensive cell in the module
    /// and is deliberately sized at the minimum that covers both arrangements.
    #[test]
    fn a_world_run_reproduces_a_named_identity_cell() {
        /// A THROWAWAY master on the wave's 9999-family convention, distinct
        /// from `SCRATCH_MASTER` and from every registered D2 master.
        const MASTER: u64 = 0x0B01_2026_0831_9997;
        let rules = Rules::standard();
        let x = Contestant::from_str_name("v4-step9-draft1").expect("the V4 candidate arm");
        let y = Contestant::from_str_name("v2-padded").expect("the shipped table, named");
        let spec = CellSpec {
            name: "wd1-repro-identities".to_string(),
            players: 2,
            x,
            y,
            units: 1,
            master: SeedScheduleKind::Diagnostic(MASTER),
        };
        let mut from_cell: Vec<(u8, GameRecord)> = Vec::new();
        run_cell_emitting(&spec, &rules, false, &mut |stats, arr| {
            from_cell.push((arr, stats.record.clone()));
            Ok(())
        })
        .expect("the named arms finish this world");
        assert_eq!(from_cell.len(), 2, "a 2P unit is the swapped pair");

        let seed = SeedSchedule::diagnostic(MASTER).next_world();
        for (arr, cell_record) in &from_cell {
            let run = run_world(
                &WorldSpec { x, y, seed, players: 2, arrangement: *arr },
                &rules,
                false,
            )
            .expect("the same world, seated the same way");
            assert_eq!(run.x_seat.0, if *arr == 0 { 0 } else { 1 }, "arrangement {arr}'s X seat");
            let DiagnosticOutcome::Terminated(stats) = &run.game.outcome else {
                panic!("the cell completed this game, so the world run must too");
            };
            assert_eq!(
                stats.record.events, cell_record.events,
                "arrangement {arr}: the applied move sequence must be the cell's own",
            );
            assert!(cell_record.terminal_digest.is_some());
            assert_eq!(
                stats.record.terminal_digest, cell_record.terminal_digest,
                "arrangement {arr}: the terminal-state digest",
            );
        }
    }

    /// Every `  turn=N …` tail line's turn number, in print order.
    fn tail_turns_printed(lines: &[String]) -> Vec<u32> {
        lines
            .iter()
            .filter_map(|line| line.strip_prefix("  turn="))
            .map(|rest| {
                rest.split(' ').next().expect("a tail line's turn field").parse::<u32>().expect(
                    "a tail line's turn field is a number",
                )
            })
            .collect()
    }

    /// **W-D1 review S1.** On a cap, `turns` is the turn the GUARD READ — one
    /// past the limit, and a turn in which no move was ever applied. Labelling
    /// the window with it promised the D1 read forty turns of moves and
    /// delivered thirty-nine.
    #[test]
    fn a_capped_tails_turn_window_ends_at_the_last_applied_move() {
        const K: u32 = 5;
        let rules = Rules::standard();
        let (x, y) = rungs();
        let run = run_world_capped(
            &WorldSpec { x, y, seed: first_world(), players: 2, arrangement: 0 },
            &rules,
            false,
            30,
        )
        .expect("a cap is an outcome here");
        let DiagnosticOutcome::Capped { turns: guard_turn, .. } = &run.game.outcome else {
            panic!("a 30-turn cap cannot contain a full 2P game");
        };
        let guard_turn = *guard_turn;

        let lines = render_world(&run, K, &rules).expect("a partial record renders");
        let printed = tail_turns_printed(&lines);
        let last_move_turn = *printed.iter().max().expect("a capped game applied moves");
        assert!(
            last_move_turn < guard_turn,
            "precondition of this cell: the guard READ turn {guard_turn}, in which no move was \
             applied — the last applied move was in turn {last_move_turn}",
        );

        let label = lines
            .iter()
            .find(|line| line.starts_with("tail (turns "))
            .expect("the tail label");
        assert!(
            label.contains(&format!("turns {}..={last_move_turn}", last_move_turn - (K - 1))),
            "the window must END at the last APPLIED move's turn, not at the guard's read \
             turn: {label}",
        );

        let mut distinct = printed.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            K as usize,
            "--tail {K} must CONTAIN {K} turns of moves, got {distinct:?}",
        );
        assert_eq!(*distinct.first().expect("non-empty"), last_move_turn - (K - 1));

        // "or fewer only when the game is shorter": a window wider than the game
        // clamps to the whole game rather than inventing turns.
        let whole = render_world(&run, 10_000, &rules).expect("a partial record renders");
        let all_turns = tail_turns_printed(&whole);
        assert_eq!(all_turns.len(), record_move_count(&run), "every applied move is in the tail");
        let whole_label = whole
            .iter()
            .find(|line| line.starts_with("tail (turns "))
            .expect("the tail label");
        assert!(
            whole_label.contains(&format!(
                "turns {}..={last_move_turn}",
                all_turns.iter().min().expect("non-empty"),
            )),
            "a window wider than the game clamps to it: {whole_label}",
        );
    }

    /// Rebuild a [`WorldRun`] around a replacement outcome — the seam the S2
    /// cells feed a deliberately mismatched record/state pair through.
    fn with_outcome(run: WorldRun, outcome: DiagnosticOutcome) -> WorldRun {
        let WorldRun { seed, players, arrangement, x_seat, x_echo, y_echo, turn_cap, .. } = run;
        WorldRun {
            seed,
            players,
            arrangement,
            x_seat,
            x_echo,
            y_echo,
            turn_cap,
            game: DiagnosticGame { seed, players, outcome },
        }
    }

    /// **W-D1 review S2, the cap arm.** On a cap the end-state block comes from
    /// the DRIVER's state while every tail line's turn/seat/phase comes from
    /// `annotate`'s independent fold of the record. A divergence would annotate
    /// the tail from one trajectory and the end state from another, silently.
    /// Both digests are in hand at render time, so the renderer must check.
    #[test]
    fn render_world_refuses_when_the_fold_disagrees_with_the_drivers_state() {
        let rules = Rules::standard();
        let (x, y) = rungs();
        let run = run_world_capped(
            &WorldSpec { x, y, seed: first_world(), players: 2, arrangement: 0 },
            &rules,
            false,
            30,
        )
        .expect("a cap is an outcome here");
        assert!(render_world(&run, 5, &rules).is_ok(), "the honest run renders");

        let DiagnosticGame { outcome, .. } = run.game;
        let DiagnosticOutcome::Capped { turns, mut record, state, moves_by_seat } = outcome else {
            panic!("a 30-turn cap cannot contain a full 2P game");
        };
        // Drop the last applied move: the record now folds to the state BEFORE
        // it while the driver's state is the state AFTER it — exactly the shape
        // of the divergence this check exists to catch.
        record.events.pop().expect("a capped game applied moves");
        let tampered = with_outcome(
            run_world_capped(
                &WorldSpec { x, y, seed: first_world(), players: 2, arrangement: 0 },
                &rules,
                false,
                30,
            )
            .expect("a second identical run supplies the surrounding fields"),
            DiagnosticOutcome::Capped { turns, record, state, moves_by_seat },
        );

        let err = render_world(&tampered, 5, &rules)
            .expect_err("a record that folds to a different state must be refused, not rendered");
        let ArenaError::Driver(message) = err else {
            panic!("expected a Driver refusal, got {err:?}");
        };
        assert!(
            message.contains("fold") && message.contains("digest"),
            "the refusal must name what disagreed: {message}",
        );
    }

    /// **W-D1 review S2, the terminated arm.** On a completed game the fold IS
    /// the end state, and the record already carries the digest the driver
    /// stamped — so the check costs nothing and closes the same gap.
    #[test]
    fn render_world_refuses_when_the_fold_disagrees_with_the_recorded_terminal_digest() {
        let rules = Rules::standard();
        let (x, y) = rungs();
        let spec = WorldSpec { x, y, seed: first_world(), players: 2, arrangement: 1 };
        let run = run_world(&spec, &rules, false).expect("the rungs finish this world");
        assert!(render_world(&run, 5, &rules).is_ok(), "the honest run renders");

        let DiagnosticGame { outcome, .. } = run.game;
        let DiagnosticOutcome::Terminated(mut stats) = outcome else {
            panic!("this world terminates");
        };
        assert!(stats.record.terminal_digest.is_some(), "the driver stamped a digest");
        stats.record.terminal_digest = Some("not-the-digest-this-record-folds-to".to_string());
        let tampered = with_outcome(
            run_world(&spec, &rules, false).expect("a second identical run supplies the fields"),
            DiagnosticOutcome::Terminated(stats),
        );

        let err = render_world(&tampered, 5, &rules)
            .expect_err("a terminal digest the record does not fold to must be refused");
        let ArenaError::Driver(message) = err else {
            panic!("expected a Driver refusal, got {err:?}");
        };
        assert!(
            message.contains("fold") && message.contains("digest"),
            "the refusal must name what disagreed: {message}",
        );
    }

    fn record_move_count(run: &WorldRun) -> usize {
        match &run.game.outcome {
            DiagnosticOutcome::Terminated(stats) => stats.record.events.len(),
            DiagnosticOutcome::Capped { record, .. } => record.events.len(),
        }
    }

    /// The same renderer over a COMPLETED game: the outcome line names the
    /// trigger and the standings instead of a cap, and the end-state summary is
    /// taken at the terminal state.
    #[test]
    fn a_terminating_world_renders_its_trigger_and_standings() {
        let rules = Rules::standard();
        let (x, y) = rungs();
        let run = run_world(
            &WorldSpec { x, y, seed: first_world(), players: 2, arrangement: 1 },
            &rules,
            false,
        )
        .expect("the rungs finish this world");
        let lines = render_world(&run, 5, &rules).expect("a terminal record renders");
        let text = lines.join("\n");
        assert!(text.contains("arrangement=1"));
        assert!(text.contains("TERMINATED"), "the outcome line");
        assert!(text.contains("trigger="), "with its end trigger");
        assert!(text.contains("rank"), "and the standings final_result reports");
        assert!(!text.contains("CAPPED"), "a completed game is never labelled capped");
    }

    fn end() -> Move {
        Move::EndAdditionalPhase
    }
    fn uni() -> Move {
        Move::UseUniversity
    }
    fn mill() -> Move {
        Move::UseSpinningMill
    }

    #[test]
    fn a_one_periodic_tail_reads_as_period_one() {
        let moves = vec![uni(), mill(), end(), end(), end(), end()];
        assert_eq!(move_period(&moves), Some(1));
    }

    #[test]
    fn a_three_periodic_tail_reads_as_period_three() {
        let mut moves = vec![uni()];
        for _ in 0..PERIOD_REPETITIONS {
            moves.extend([end(), uni(), mill()]);
        }
        assert_eq!(move_period(&moves), Some(3));
    }

    #[test]
    fn the_smallest_period_wins_when_several_fit() {
        // A 1-periodic tail is also 2- and 4-periodic. Reporting the largest fit
        // would describe a one-move loop as a four-move one.
        let moves = vec![end(); 4 * PERIOD_REPETITIONS];
        assert_eq!(move_period(&moves), Some(1));
    }

    #[test]
    fn an_aperiodic_tail_reads_as_none() {
        // Every window's last block differs from the one before it.
        let mut moves = Vec::new();
        for i in 0..40 {
            moves.push(if i % 7 == 0 { uni() } else if i % 3 == 0 { mill() } else { end() });
        }
        moves.push(uni());
        moves.push(uni());
        moves.push(mill());
        assert_eq!(move_period(&moves), None);
    }

    #[test]
    fn a_tail_too_short_for_four_repetitions_reads_as_none() {
        // Three repetitions of a 2-block is 6 moves; the bar is four.
        let moves = vec![end(), uni(), end(), uni(), end(), uni()];
        assert_eq!(move_period(&moves), None);
        assert_eq!(move_period(&[]), None);
        assert_eq!(move_period(&[end(), end(), end()]), None);
    }

    #[test]
    fn no_period_beyond_the_registered_bound_is_reported() {
        // A period of MAX_DETECTED_PERIOD + 1 is real but out of the wave's
        // declared range: the detector says `none` rather than growing its own
        // bound silently.
        //
        // THE BLOCK'S SHAPE IS THE TEST. `[end ×64, uni]` has no period below
        // 65 in ANY tail window: every window ends on the lone `uni`, and
        // `tail[4p-1-p]` is an `end` for every p in 1..=64 (the `uni`s sit 65
        // apart, so a p-shift lands one on an `end` unless p is a multiple of
        // 65). An alternating block would NOT do — its own tail is 2-periodic,
        // and the cell would pass by measuring the wrong thing.
        let mut block = vec![end(); MAX_DETECTED_PERIOD];
        block.push(uni());
        assert_eq!(block.len(), MAX_DETECTED_PERIOD + 1);
        let mut moves = Vec::new();
        for _ in 0..PERIOD_REPETITIONS {
            moves.extend(block.iter().cloned());
        }
        assert!(
            moves.len() >= PERIOD_REPETITIONS * MAX_DETECTED_PERIOD,
            "the sequence must be long enough for the largest window the detector tries, or \
             the `none` is a length answer rather than a bound answer",
        );
        assert_eq!(move_period(&moves), None);
    }

    #[test]
    fn the_cap_rate_label_is_the_registered_string() {
        assert_eq!(
            CAP_RATE_LABEL,
            "DIAGNOSTIC — cap-rate instrument; no strength figure is computed here",
            "the label is registered verbatim in the step-11 diagnosis-wave ledger row",
        );
    }

    #[test]
    fn the_usage_section_names_both_diagnostic_subcommands_and_carries_the_label() {
        assert!(DIAGNOSTIC_USAGE.contains("world "), "the `world` subcommand is undocumented");
        assert!(DIAGNOSTIC_USAGE.contains("caprate "), "the `caprate` subcommand is undocumented");
        assert!(
            DIAGNOSTIC_USAGE.contains(CAP_RATE_LABEL),
            "the help text must carry the cap-rate label a reader will see printed",
        );
        assert!(
            DIAGNOSTIC_USAGE.contains("DIAGNOSTIC SUBCOMMANDS"),
            "the section heading is what makes these two readable as diagnostics",
        );
    }

    #[test]
    fn the_named_identities_section_lists_every_arm_the_grammar_accepts() {
        let text = named_identities_usage();
        for arm in NamedIdentity::ALL {
            assert!(
                text.contains(arm.name()),
                "the help text omits {:?}, which the grammar accepts — the exact drift this \
                 section exists to make unrepresentable",
                arm.name(),
            );
        }
        // Named explicitly as well as by the loop: this is the arm whose absence
        // was the stale block's actual defect at the step-11 open.
        assert!(text.contains("v4-step9-draft1"));
        assert!(text.contains("NAMED IDENTITIES"), "the section heading");
    }
}
