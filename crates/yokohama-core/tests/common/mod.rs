//! Shared random self-play driver for Task 18 (spec F1/F2).
//!
//! This file is compiled from two places that cannot depend on each other
//! (Cargo `tests/` and `examples/` are separate crates that only see the
//! library, never each other): `tests/fuzz_full_games.rs` includes it the
//! normal way (`mod common;`), and `examples/selfplay.rs` includes this
//! exact same source file via `#[path = "../tests/common/mod.rs"] mod
//! common;` so both binaries compile the identical `run_random_game` --
//! zero duplication, one source of truth (documented choice; the brief's
//! other sanctioned shape was example-side duplication, which this avoids).

#![allow(dead_code)]

use std::collections::HashMap;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use yokohama_core::{
    apply_move_unchecked, check_invariants, finalize_setup, get_legal_moves, is_game_over, new_game,
    EndGameTrigger, GameConfig, GameRecord, GameState, Move, Rules, StartingOrderChoice, TurnPhase,
};

/// Turn cap: exceeding this without `is_game_over` returning true is treated
/// as a non-termination failure rather than looping forever.
///
/// 10000, not Task 18.4's 2000 (engine-authority Task 7 re-measurement):
/// enumerating Option B retreat as the full per-area cartesian product (I5
/// ruling -- any number of assistants, not all-or-nothing) makes
/// `ReturnPresidentToHand` the single largest move family whenever a
/// player's assistants are scattered across several board areas, which
/// dilutes uniform-random selection AWAY from `MovePresident`/
/// `PlacePresident`/`ExecuteAreaAction` and measurably slows convergence --
/// not a dead end (a temporarily-raised cap confirmed every one of the
/// fuzz gate's 1500 seeds, 500 per player count, still completes), just
/// slower. Re-measured worst case at that same 1500-seed sample (`cargo run
/// --release -p yokohama-core --example selfplay -- 500 <players> 0`,
/// diagnostic max-turns instrumentation, discarded after measuring): 2p
/// seed 191 -> 6036 turns (mean 1181.6), 3p seed 114 -> 4755 (mean 1625.2),
/// 4p seed 194 -> 4104 (mean 2126.6). 10000 gives ~65% headroom over the
/// observed 6036 max. Real games end far sooner; this bound exists purely
/// to catch true non-termination.
pub const MAX_TURNS: u32 = 10000;

/// Summary statistics for one completed random self-play game.
#[derive(Debug, Clone)]
pub struct GameStats {
    pub players: u8,
    pub seed: u64,
    pub turns: u32,
    pub moves_applied: u64,
    pub trigger: Option<EndGameTrigger>,
    /// Per-phase (sum of branching factor, number of decisions observed),
    /// for computing a mean branching factor per `TurnPhase`.
    pub branching_by_phase: HashMap<TurnPhase, (u64, u64)>,
}

/// Build a fresh seeded game up to the post-setup state, under the same
/// choice policy as `run_random_game`/`run_random_game_observed` (per-player
/// `StartingOrderChoice::Keep` drawn from the seeded RNG) -- reused here via
/// the identical `new_game` -> `finalize_setup` sequence rather than
/// reimplemented, so setup logic has exactly one copy. Returns the RNG in
/// its post-setup-consumption state alongside the resulting `GameState`, so
/// a caller that wants to keep driving deterministic random play from the
/// same seed can (mirrors `run_random_game`'s internal handoff to
/// `run_random_game_from`). Panics on setup failure: like `run_recorded_game`,
/// this is test-only setup infrastructure, not a driver under test.
pub fn new_seeded_game(seed: u64, players: u8, rules: &Rules) -> (GameState, ChaCha8Rng) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let config = GameConfig::for_player_count(players);
    let pending = new_game(config, rules, seed)
        .unwrap_or_else(|e| panic!("seed={seed} players={players}p: new_game failed: {e}"));

    let choices: Vec<StartingOrderChoice> =
        (0..players).map(|_| StartingOrderChoice::Keep(rng.gen_range(0..2u8))).collect();
    let state = finalize_setup(pending, &choices)
        .unwrap_or_else(|e| panic!("seed={seed} players={players}p: finalize_setup failed: {e}"));

    (state, rng)
}

/// Play one random game to completion (or to a hard failure) from a seed.
///
/// Deterministic: the same `(seed, players)` pair always plays the same
/// game -- the starting-order keep-choice and every subsequent move pick
/// are drawn from a single `ChaCha8Rng` seeded with `seed`.
///
/// `GameState` is history-free by design (see crate docs), so this function
/// keeps its own `Vec<Move>` log for failure reporting.
///
/// Returns `Ok(GameStats)` when `is_game_over` becomes true. Returns
/// `Err(String)` -- a full dump (seed, turn, phase, move log, and the
/// complete state as JSON) -- for any of: `new_game`/`finalize_setup`
/// failing, `get_legal_moves` returning empty in a non-terminal state,
/// `apply_move_unchecked` erroring on a move `get_legal_moves` itself
/// offered, a `check_invariants` violation after an apply, or exceeding
/// `MAX_TURNS` without `is_game_over`. This function's own logic never
/// panics -- failures are always reported through the `Result`, not via
/// panic.
pub fn run_random_game(seed: u64, players: u8, rules: &Rules) -> Result<GameStats, String> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let config = GameConfig::for_player_count(players);
    let pending = new_game(config, rules, seed)
        .map_err(|e| format!("seed={seed} players={players}p: new_game failed: {e}"))?;

    let choices: Vec<StartingOrderChoice> =
        (0..players).map(|_| StartingOrderChoice::Keep(rng.gen_range(0..2u8))).collect();
    let state = finalize_setup(pending, &choices)
        .map_err(|e| format!("seed={seed} players={players}p: finalize_setup failed: {e}"))?;

    run_random_game_from(state, &mut rng, seed, players, rules)
}

/// Random-play an ALREADY-CONSTRUCTED state to completion under the same
/// contract as `run_random_game` (same failure modes, same invariant checks
/// after every apply). Used by the seeded near-threshold trigger scenarios,
/// which surgically construct a legal state, fire a specific trigger through
/// `apply_move_unchecked`, and then let random play finish the final round.
pub fn run_random_game_from(
    state: GameState,
    rng: &mut ChaCha8Rng,
    seed: u64,
    players: u8,
    rules: &Rules,
) -> Result<GameStats, String> {
    run_random_game_from_observed(state, rng, seed, players, rules, |_, _| {})
}

/// Play a FRESH seeded game to completion (like `run_random_game`), invoking
/// `observe(state, mv)` immediately before every applied move -- `state` is
/// the state `mv` is about to be applied to (so `state.pending_choices`
/// reflects what was pending when `mv` was picked). Twin of
/// `run_random_game`/`run_random_game_from`, added for Task 15's organic
/// choice-coverage census: the census needs to see every move as it's
/// chosen, not just the final outcome.
pub fn run_random_game_observed(
    seed: u64,
    players: u8,
    rules: &Rules,
    observe: impl FnMut(&GameState, &Move),
) -> Result<GameStats, String> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let config = GameConfig::for_player_count(players);
    let pending = new_game(config, rules, seed)
        .map_err(|e| format!("seed={seed} players={players}p: new_game failed: {e}"))?;

    let choices: Vec<StartingOrderChoice> =
        (0..players).map(|_| StartingOrderChoice::Keep(rng.gen_range(0..2u8))).collect();
    let state = finalize_setup(pending, &choices)
        .map_err(|e| format!("seed={seed} players={players}p: finalize_setup failed: {e}"))?;

    run_random_game_from_observed(state, &mut rng, seed, players, rules, observe)
}

/// Shared loop behind `run_random_game_from` (no-op observer) and
/// `run_random_game_observed` (fresh setup, real observer) -- one copy of
/// the play-to-completion logic so both public entry points stay exactly as
/// they were (same failure modes, same invariant checks) rather than
/// drifting apart.
fn run_random_game_from_observed(
    mut state: GameState,
    rng: &mut ChaCha8Rng,
    seed: u64,
    players: u8,
    rules: &Rules,
    mut observe: impl FnMut(&GameState, &Move),
) -> Result<GameStats, String> {
    let mut move_log: Vec<Move> = Vec::new();
    let mut branching_by_phase: HashMap<TurnPhase, (u64, u64)> = HashMap::new();
    let mut moves_applied: u64 = 0;

    // Guard the INCOMING state: corruption from setup or a scenario test's
    // surgical construction must be reported as such, not misattributed to
    // whichever move happens to be applied first (Task 18 scaffold-review
    // finding).
    if let Err(reason) = check_invariants(&state, rules) {
        return Err(dump_failure(
            seed,
            players,
            &format!("corrupt starting state (invariant violated before any move): {reason}"),
            &state,
            &move_log,
        ));
    }

    loop {
        if is_game_over(&state) {
            return Ok(GameStats {
                players,
                seed,
                turns: state.turn_number,
                moves_applied,
                trigger: state.trigger_reason,
                branching_by_phase,
            });
        }

        if state.turn_number > MAX_TURNS {
            return Err(dump_failure(
                seed,
                players,
                "non-termination: turn_number exceeded MAX_TURNS without is_game_over becoming true",
                &state,
                &move_log,
            ));
        }

        let moves = get_legal_moves(&state, rules);
        let entry = branching_by_phase.entry(state.turn_phase).or_insert((0, 0));
        entry.0 += moves.len() as u64;
        entry.1 += 1;

        if moves.is_empty() {
            return Err(dump_failure(
                seed,
                players,
                "dead end: get_legal_moves returned empty in a non-terminal state",
                &state,
                &move_log,
            ));
        }

        let choice = moves[rng.gen_range(0..moves.len())].clone();
        observe(&state, &choice);
        // apply_move_unchecked: choice is drawn from get_legal_moves by
        // construction, so the checked membership scan would roughly double
        // the fuzz gate's per-move enumeration cost for no benefit here.
        match apply_move_unchecked(&state, &choice, rules) {
            Ok(next_state) => {
                if let Err(reason) = check_invariants(&next_state, rules) {
                    move_log.push(choice);
                    return Err(dump_failure(
                        seed,
                        players,
                        &format!("invariant violated after applying move: {reason}"),
                        &next_state,
                        &move_log,
                    ));
                }
                move_log.push(choice);
                state = next_state;
                moves_applied += 1;
            }
            Err(e) => {
                return Err(dump_failure(
                    seed,
                    players,
                    &format!(
                        "apply_move_unchecked returned Err for a move get_legal_moves said was legal: {e}"
                    ),
                    &state,
                    &move_log,
                ));
            }
        }
    }
}

/// Number of moves to record for `run_recorded_game`'s "short" game --
/// enough to cross several turns and phases (placement, movement, area
/// action, building, additional actions) but far short of a full game
/// (which averages hundreds of turns, per Task 18's fuzz gate), so Task
/// 19's determinism tests stay fast.
pub const RECORDED_GAME_MOVES: u32 = 40;

/// Like `run_random_game`, but captures every applied move into a
/// `GameRecord` alongside the resulting state (Task 19, spec F3). Same
/// seeded `ChaCha8Rng` and choice-drawing order as `run_random_game`
/// (starting-order keep choice per player, then one move choice per
/// decision), so a given seed plays the identical game -- but stops after
/// `RECORDED_GAME_MOVES` applied moves (or sooner, if the game legitimately
/// ends first) rather than running to completion.
///
/// Panics rather than returning `Result`: this is test-only setup
/// infrastructure for the replay-determinism tests, not the fuzz driver
/// under test, so a hard failure here should surface immediately.
pub fn run_recorded_game(seed: u64, players: u8, rules: &Rules) -> (GameState, GameRecord) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let config = GameConfig::for_player_count(players);
    let pending = match new_game(config, rules, seed) {
        Ok(p) => p,
        Err(e) => panic!("seed={seed} players={players}p: new_game failed: {e}"),
    };
    let record_config = pending.state.config.clone();

    let choices: Vec<StartingOrderChoice> =
        (0..players).map(|_| StartingOrderChoice::Keep(rng.gen_range(0..2u8))).collect();
    let mut record = GameRecord::new(seed, record_config, choices.clone(), rules);

    let mut state = match finalize_setup(pending, &choices) {
        Ok(s) => s,
        Err(e) => panic!("seed={seed} players={players}p: finalize_setup failed: {e}"),
    };

    for _ in 0..RECORDED_GAME_MOVES {
        if is_game_over(&state) {
            break;
        }
        let moves = get_legal_moves(&state, rules);
        if moves.is_empty() {
            panic!(
                "seed={seed} players={players}p turn={}: dead end, get_legal_moves returned empty in a non-terminal state",
                state.turn_number,
            );
        }
        let mv = moves[rng.gen_range(0..moves.len())].clone();
        // apply_move_unchecked: mv is drawn from get_legal_moves by
        // construction (same rationale as run_random_game_from_observed).
        match apply_move_unchecked(&state, &mv, rules) {
            Ok(next_state) => {
                state = next_state;
                record.push_move(mv);
            }
            Err(e) => panic!(
                "seed={seed} players={players}p turn={}: apply_move_unchecked failed for a move get_legal_moves offered: {e}",
                state.turn_number,
            ),
        }
    }

    (state, record)
}

/// Build a full failure report: seed/turn/phase header, the driver-side
/// move log, and the complete state serialized as JSON (falling back to
/// `Debug` if serialization itself somehow fails -- reporting a failure
/// must never introduce a new one).
fn dump_failure(seed: u64, players: u8, reason: &str, state: &GameState, move_log: &[Move]) -> String {
    let state_json = serde_json::to_string_pretty(state)
        .unwrap_or_else(|e| format!("<state JSON serialization failed: {e}; Debug: {state:#?}>"));
    format!(
        "seed={seed} players={players}p turn={turn} phase={phase:?}\nreason: {reason}\n\
         move_log ({n} moves): {move_log:?}\n\nstate JSON:\n{state_json}",
        turn = state.turn_number,
        phase = state.turn_phase,
        n = move_log.len(),
    )
}
