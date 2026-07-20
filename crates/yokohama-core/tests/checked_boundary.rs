// crates/yokohama-core/tests/checked_boundary.rs
//! Design §4.1: the public transition is membership-checked; the raw
//! transition is explicit opt-in for prechecked hot paths.
mod common;

use yokohama_core::{
    apply_move, apply_move_unchecked, get_legal_moves, Move, MoveError, Rules,
};

/// A deterministic mid-game state: fold the first `n` random moves of a seed.
fn state_after(seed: u64, players: u8, n: usize) -> yokohama_core::GameState {
    let rules = Rules::standard();
    let (mut state, _) = common::new_seeded_game(seed, players, &rules);
    for _ in 0..n {
        let moves = get_legal_moves(&state, &rules);
        state = apply_move_unchecked(&state, &moves[0], &rules).unwrap();
    }
    state
}

#[test]
fn checked_apply_rejects_out_of_phase_move_with_not_legal() {
    let rules = Rules::standard();
    let state = state_after(4, 2, 0); // Placement phase, start of game
    // A Movement-phase move: free-path teleport the raw arm would happily apply.
    let mv = Move::MovePresident { path: vec![], destination: yokohama_core::AreaId::new(2) };
    assert!(!get_legal_moves(&state, &rules).contains(&mv));
    assert_eq!(apply_move(&state, &mv, &rules), Err(MoveError::NotLegal));
}

#[test]
fn checked_apply_rejects_in_phase_but_illegal_move_with_not_legal() {
    let rules = Rules::standard();
    let state = state_after(4, 2, 0);
    // In-phase (Placement) but not in the legal list: place into the same
    // area three times with Option A (generator never emits duplicates).
    let a = *state.board_layout.areas.keys().next().unwrap();
    let mv = Move::PlaceThreeAreas { placements: vec![a, a, a] };
    if get_legal_moves(&state, &rules).contains(&mv) {
        panic!("fixture assumption broken: pick a different illegal shape");
    }
    assert_eq!(apply_move(&state, &mv, &rules), Err(MoveError::NotLegal));
}

#[test]
fn checked_apply_accepts_every_legal_move_and_matches_unchecked() {
    let rules = Rules::standard();
    let state = state_after(9, 3, 5);
    for mv in get_legal_moves(&state, &rules) {
        let a = apply_move(&state, &mv, &rules).expect("legal move must pass the checked path");
        let b = apply_move_unchecked(&state, &mv, &rules).unwrap();
        assert_eq!(a, b, "checked and unchecked must agree on legal moves");
    }
}
