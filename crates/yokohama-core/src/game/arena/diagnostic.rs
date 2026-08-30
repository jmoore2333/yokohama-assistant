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

use super::NamedIdentity;
use crate::types::Move;

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
