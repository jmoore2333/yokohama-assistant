//! Task 6 (L2 route enumeration, design 2026-07-19 §4.3): organic-coverage
//! census proving multi-label Movement states -- a destination reachable
//! via >=2 distinct (path, fee-vector) labels -- actually occur in random
//! self-play on the REAL board (`Rules::standard()`), not just in the
//! hand-built diamond fixtures in `movement.rs`'s unit test module. Mirrors
//! the choice_encoding census (`tests/choice_encoding.rs`) precedent: same
//! `run_random_game_observed` driver, same seed-scan-with-early-return
//! shape, same "record the first-hit seed in a comment" convention.

mod common;
use common::run_random_game_observed;

use std::collections::HashMap;

use yokohama_core::{get_legal_moves, AreaId, Move, Rules, TurnPhase};

/// Seed range this census will scan before treating "no multi-label state
/// found" as a real regression rather than bad luck.
/// Empirically the first hit is immediate (see the test's doc comment
/// below) -- the real board is a grid with plenty of redundant adjacency,
/// so branching Movement subgraphs are the common case, not the rare one.
/// 300 matches the choice_encoding census's cap for consistency, with
/// enormous headroom over the observed worst case.
const CENSUS_SEED_CAP: u64 = 300;

#[test]
fn multi_label_states_occur_organically_in_random_play() {
    // First-hit seed (throwaway probe -- a temporary eprintln! in this
    // loop, run with `--nocapture`, then reverted before commit -- same
    // method as choice_encoding.rs's census comment): seed=0, players=2p,
    // turn=28, THREE destinations simultaneously multi-label
    // (`{AreaId(8): 2, AreaId(1): 2, AreaId(2): 2}`). Not a rare edge case:
    // the very first game scanned already hits it by turn 28 out of a
    // typical several-hundred-turn game -- the real board's grid adjacency
    // means a president with assistants spread across a couple of areas
    // near a canal or an opponent President routinely has both a cheap and
    // a deliberately-pricier route to some shared destination.
    let rules = Rules::standard();

    for seed in 0..CENSUS_SEED_CAP {
        let players = 2u8 + (seed % 3) as u8; // cycle 2p/3p/4p for full board-rule coverage
        let mut found = false;

        run_random_game_observed(seed, players, &rules, |state, _mv| {
            if found || state.turn_phase != TurnPhase::Movement {
                return;
            }
            // Queue-first gate: if a choice is pending, get_legal_moves
            // returns choice resolutions only, so no MovePresident entries
            // would match below regardless -- no separate guard needed.
            let moves = get_legal_moves(state, &rules);
            let mut by_destination: HashMap<AreaId, u32> = HashMap::new();
            for mv in &moves {
                if let Move::MovePresident { destination, .. } = mv {
                    *by_destination.entry(*destination).or_insert(0) += 1;
                }
            }
            if by_destination.values().any(|&count| count >= 2) {
                found = true;
            }
        })
        .unwrap_or_else(|e| panic!("seed={seed} players={players}p: {e}"));

        if found {
            return;
        }
    }

    panic!(
        "no Movement state with >=2 labels to the same destination found in seeds 0..{CENSUS_SEED_CAP} \
         -- L2 route enumeration should fire organically on the real board, not just in fixtures"
    );
}
