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
