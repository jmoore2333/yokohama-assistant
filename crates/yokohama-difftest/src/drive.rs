//! Seeded engine self-play driver, mirroring the engine's own fuzz drivers
//! (`crates/yokohama-core/tests/common/mod.rs`, used by both
//! `tests/fuzz_full_games.rs` and `examples/selfplay.rs`): identical
//! `new_game` -> `finalize_setup` construction (per-player
//! `StartingOrderChoice::Keep` drawn from one seeded `ChaCha8Rng`), stepping
//! by choosing from `get_legal_moves` and applying via
//! `apply_move_unchecked` (every move is drawn from the legal list by
//! construction), `check_invariants` after every apply and on the incoming
//! state, and the same turn cap of 10000.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move_unchecked, check_invariants, finalize_setup, get_legal_moves, is_game_over,
    new_game, GameConfig, GameState, Move, Rules, StartingOrderChoice,
};

/// Turn cap (matches the engine fuzz drivers' `MAX_TURNS`): exceeding this
/// without `is_game_over` is treated as non-termination, not looped on.
pub const MAX_TURNS: u32 = 10_000;

/// Move-selection policy for self-play.
///
/// `RetreatAverse` differs from `Uniform` ONLY in never VOLUNTEERING
/// retreats, implementing the external probe's policy from guidingpath.md's
/// 2026-07-20 sizing-correction row (Phase 3 section). Exact rule: at each
/// decision point, let F be the subset of legal moves that are NOT
/// `Move::ReturnPresidentToHand` with a nonzero total recovery count
/// (`recover.values().sum() > 0`). If F is non-empty, choose uniformly from
/// F; otherwise (nothing else is legal) fall back to choosing uniformly
/// from the full legal list. A `ReturnPresidentToHand` with an empty or
/// all-zero `recover` map recovers nothing and counts as a non-retreat —
/// the excluded family is exactly the Movement-Option-B count-vector
/// product that uniform-random play over-selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Uniform-random over the full legal list (the fuzzer's policy).
    Uniform,
    /// Uniform-random over non-retreat moves when any exist (see above).
    RetreatAverse,
}

impl Policy {
    /// Stable name used in artifact filenames and CLI args.
    pub fn as_str(&self) -> &'static str {
        match self {
            Policy::Uniform => "uniform",
            Policy::RetreatAverse => "retreat-averse",
        }
    }
}

impl std::str::FromStr for Policy {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "uniform" => Ok(Policy::Uniform),
            "retreat-averse" | "retreat_averse" => Ok(Policy::RetreatAverse),
            other => Err(format!("unknown policy {other:?} (expected uniform | retreat-averse)")),
        }
    }
}

/// True for the moves `Policy::RetreatAverse` refuses to volunteer: a
/// Movement-Option-B recovery (`ReturnPresidentToHand`) that actually
/// recovers at least one assistant.
pub fn is_volunteered_retreat(m: &Move) -> bool {
    matches!(m, Move::ReturnPresidentToHand { recover } if recover.values().any(|&n| n > 0))
}

/// Pick one move from `moves` (must be non-empty) under `policy`.
pub fn pick_move<'a>(moves: &'a [Move], policy: Policy, rng: &mut ChaCha8Rng) -> &'a Move {
    match policy {
        Policy::Uniform => &moves[rng.gen_range(0..moves.len())],
        Policy::RetreatAverse => {
            let non_retreat: Vec<&Move> =
                moves.iter().filter(|m| !is_volunteered_retreat(m)).collect();
            if non_retreat.is_empty() {
                // Nothing but retreats is legal: fall back to the full list.
                &moves[rng.gen_range(0..moves.len())]
            } else {
                non_retreat[rng.gen_range(0..non_retreat.len())]
            }
        }
    }
}

/// Summary of one driven game.
#[derive(Debug, Clone)]
pub struct DriveStats {
    pub seed: u64,
    pub players: u8,
    pub policy: Policy,
    /// Decision points visited (moves applied).
    pub steps: u64,
    /// `turn_number` of the last state seen.
    pub turns: u32,
    /// True when the game reached `is_game_over` (false when stopped by the
    /// observer or by `max_steps`).
    pub completed: bool,
}

/// Observer verdict per decision point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    /// Keep playing.
    Continue,
    /// Stop the drive cleanly (e.g. a divergence artifact was written).
    Stop,
}

/// Build a fresh seeded post-setup game exactly the way the engine fuzz
/// drivers do: per-player `StartingOrderChoice::Keep(rng.gen_range(0..2))`
/// from a `ChaCha8Rng` seeded with `seed`. Returns the RNG in its
/// post-setup state so the caller keeps drawing from the same stream.
pub fn new_seeded_game(seed: u64, players: u8, rules: &Rules) -> Result<(GameState, ChaCha8Rng), String> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let config = GameConfig::for_player_count(players);
    let pending = new_game(config, rules, seed)
        .map_err(|e| format!("seed={seed} players={players}p: new_game failed: {e}"))?;
    let choices: Vec<StartingOrderChoice> =
        (0..players).map(|_| StartingOrderChoice::Keep(rng.gen_range(0..2u8))).collect();
    let state = finalize_setup(pending, &choices)
        .map_err(|e| format!("seed={seed} players={players}p: finalize_setup failed: {e}"))?;
    Ok((state, rng))
}

/// Drive one seeded self-play game under `policy`, invoking
/// `observer(state, legal_moves, step_index)` at every decision point
/// BEFORE a move is picked (this is where the differential comparison
/// plugs in). Stops on: game over, observer `Stop`, `max_steps` decision
/// points, or a hard failure (`Err`): empty legal list in a non-terminal
/// state, `apply_move_unchecked` rejecting a generated move, an invariant
/// violation (checked on the starting state and after every apply, like the
/// engine fuzz drivers), observer error, or exceeding [`MAX_TURNS`].
pub fn drive_game(
    seed: u64,
    players: u8,
    policy: Policy,
    rules: &Rules,
    max_steps: Option<u64>,
    mut observer: impl FnMut(&GameState, &[Move], u64) -> Result<StepOutcome, String>,
) -> Result<DriveStats, String> {
    let (mut state, mut rng) = new_seeded_game(seed, players, rules)?;

    if let Err(reason) = check_invariants(&state, rules) {
        return Err(fail(seed, players, &state, &format!("corrupt starting state: {reason}")));
    }

    let mut steps: u64 = 0;
    loop {
        if is_game_over(&state) {
            return Ok(DriveStats { seed, players, policy, steps, turns: state.turn_number, completed: true });
        }
        if state.turn_number > MAX_TURNS {
            return Err(fail(
                seed,
                players,
                &state,
                "non-termination: turn_number exceeded MAX_TURNS without is_game_over",
            ));
        }
        if max_steps.is_some_and(|cap| steps >= cap) {
            return Ok(DriveStats { seed, players, policy, steps, turns: state.turn_number, completed: false });
        }

        let moves = get_legal_moves(&state, rules);
        if moves.is_empty() {
            return Err(fail(
                seed,
                players,
                &state,
                "dead end: get_legal_moves returned empty in a non-terminal state",
            ));
        }

        match observer(&state, &moves, steps)? {
            StepOutcome::Stop => {
                return Ok(DriveStats { seed, players, policy, steps, turns: state.turn_number, completed: false });
            }
            StepOutcome::Continue => {}
        }

        let mv = pick_move(&moves, policy, &mut rng).clone();
        // apply_move_unchecked: mv is drawn from get_legal_moves by
        // construction (same rationale as the engine fuzz drivers).
        match apply_move_unchecked(&state, &mv, rules) {
            Ok(next) => {
                if let Err(reason) = check_invariants(&next, rules) {
                    return Err(fail(
                        seed,
                        players,
                        &next,
                        &format!("invariant violated after applying {mv:?}: {reason}"),
                    ));
                }
                state = next;
                steps += 1;
            }
            Err(e) => {
                return Err(fail(
                    seed,
                    players,
                    &state,
                    &format!("apply_move_unchecked rejected a generated move {mv:?}: {e}"),
                ));
            }
        }
    }
}

/// Failure report: header plus the full state JSON (the state is
/// self-contained and resumable, so this is the whole repro).
fn fail(seed: u64, players: u8, state: &GameState, reason: &str) -> String {
    let state_json = serde_json::to_string_pretty(state)
        .unwrap_or_else(|e| format!("<state JSON serialization failed: {e}>"));
    format!(
        "seed={seed} players={players}p turn={} phase={:?}\nreason: {reason}\nstate JSON:\n{state_json}",
        state.turn_number, state.turn_phase,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use yokohama_core::AreaId;

    fn retreat(counts: &[(u8, u8)]) -> Move {
        let recover: BTreeMap<AreaId, u8> =
            counts.iter().map(|&(a, n)| (AreaId::new(a), n)).collect();
        Move::ReturnPresidentToHand { recover }
    }

    #[test]
    fn retreat_classification_matches_the_documented_rule() {
        assert!(is_volunteered_retreat(&retreat(&[(1, 2), (3, 0)])));
        // Recovering nothing is NOT a volunteered retreat...
        assert!(!is_volunteered_retreat(&retreat(&[])));
        assert!(!is_volunteered_retreat(&retreat(&[(1, 0)])));
        // ...and no other family ever is.
        assert!(!is_volunteered_retreat(&Move::PlaceThreeAreas { placements: vec![] }));
    }

    #[test]
    fn retreat_averse_never_picks_a_nonzero_retreat_when_alternatives_exist() {
        let moves = vec![retreat(&[(1, 1)]), Move::PlaceThreeAreas { placements: vec![] }, retreat(&[(2, 2)])];
        let mut rng = ChaCha8Rng::seed_from_u64(0);
        for _ in 0..100 {
            let picked = pick_move(&moves, Policy::RetreatAverse, &mut rng);
            assert!(!is_volunteered_retreat(picked), "picked a volunteered retreat: {picked:?}");
        }
    }

    #[test]
    fn retreat_averse_falls_back_to_full_list_when_only_retreats_are_legal() {
        let moves = vec![retreat(&[(1, 1)]), retreat(&[(2, 2)])];
        let mut rng = ChaCha8Rng::seed_from_u64(0);
        let picked = pick_move(&moves, Policy::RetreatAverse, &mut rng);
        assert!(moves.contains(picked));
    }

    #[test]
    fn policy_parses_from_cli_names() {
        assert_eq!("uniform".parse::<Policy>().unwrap(), Policy::Uniform);
        assert_eq!("retreat-averse".parse::<Policy>().unwrap(), Policy::RetreatAverse);
        assert_eq!("retreat_averse".parse::<Policy>().unwrap(), Policy::RetreatAverse);
        assert!("greedy".parse::<Policy>().is_err());
    }
}
