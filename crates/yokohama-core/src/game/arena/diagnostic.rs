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
pub fn move_period(_moves: &[Move]) -> Option<usize> {
    // PLACEHOLDER (RED): the detector's tests state the contract above.
    None
}

/// The runner's `DIAGNOSTIC SUBCOMMANDS` help section.
///
/// It lives in CORE rather than in `examples/arena.rs` for
/// [`named_identities_usage`]'s reason, one level down: the two subcommands'
/// contract — above all the no-strength-figure rule — is a property of the code
/// here, and help text that restates a property from another crate is help text
/// free to go stale about it.
pub const DIAGNOSTIC_USAGE: &str = "PLACEHOLDER";

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
    // PLACEHOLDER (RED).
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::arena::NamedIdentity;

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
        let block: Vec<Move> = (0..=MAX_DETECTED_PERIOD)
            .map(|i| if i % 2 == 0 { end() } else { uni() })
            .collect();
        assert_eq!(block.len(), MAX_DETECTED_PERIOD + 1);
        let mut moves = Vec::new();
        for _ in 0..PERIOD_REPETITIONS {
            moves.extend(block.iter().cloned());
        }
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
