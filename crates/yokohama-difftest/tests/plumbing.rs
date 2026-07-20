//! Harness plumbing smoke tests (NOT ignored — these run in the per-edit
//! debug suite): the driver itself must work engine-only before any
//! differential claim means anything.

use yokohama_core::{check_invariants, route_fee_vector, Move, Rules};
use yokohama_difftest::compare::{engine_route_label, route_label_key};
use yokohama_difftest::drive::{drive_game, new_seeded_game, Policy, StepOutcome};

/// Drive one short game (200 steps, seed 0, 2p, uniform) engine-only.
/// `drive_game` checks `check_invariants` after every apply internally, so a
/// clean `Ok` IS the invariants-stay-clean assertion; the observer re-checks
/// per decision point anyway so a violation names the step it surfaced at.
#[test]
fn smoke_short_uniform_game_keeps_invariants_clean() {
    let rules = Rules::standard();
    let stats = drive_game(0, 2, Policy::Uniform, &rules, Some(200), |state, moves, step| {
        assert!(!moves.is_empty(), "no dead ends at step {step}");
        check_invariants(state, &rules).map_err(|e| format!("invariant dirty at step {step}: {e}"))?;
        Ok(StepOutcome::Continue)
    })
    .expect("short uniform game drives cleanly");
    assert_eq!(stats.steps, 200, "the cap, not the game end, should stop a 200-step slice");
}

/// The retreat-averse policy drives real games too (same plumbing, the
/// other selectable policy).
#[test]
fn smoke_short_retreat_averse_game_drives_cleanly() {
    let rules = Rules::standard();
    let stats = drive_game(1, 2, Policy::RetreatAverse, &rules, Some(100), |_, _, _| Ok(StepOutcome::Continue))
        .expect("short retreat-averse game drives cleanly");
    assert!(stats.steps > 0);
}

/// Engine-side route-label projection agrees with the engine's own pricing
/// on a real state: the label's parts must re-sum to `route_fee_vector`'s
/// total (the engine's claim), and an empty path (teleport encoding) must
/// price to the all-zero label.
#[test]
fn engine_route_label_matches_engine_pricing_on_a_real_state() {
    let rules = Rules::standard();
    let (state, _rng) = new_seeded_game(0, 2, &rules).expect("setup");

    // Walk the driver forward until a state actually offers MovePresident.
    let mut found = false;
    drive_game(0, 2, Policy::Uniform, &rules, Some(300), |state, moves, _| {
        for m in moves {
            if let Move::MovePresident { path, destination } = m {
                let label = engine_route_label(state, &rules, path, *destination);
                let vector =
                    route_fee_vector(state, state.current_player, path, Some(&rules.techs), false);
                let total: u32 = vector.values().sum();
                assert_eq!(label.mover_cost, total, "label cost must be the engine's own sum");
                assert_eq!(
                    label.player_credits.values().sum::<u32>() + label.bank,
                    label.mover_cost,
                    "credits + bank must re-sum to the mover's cost"
                );
                assert_eq!(label.to, *destination);
                found = true;
            }
        }
        Ok(StepOutcome::Continue)
    })
    .expect("drive");
    assert!(found, "300 steps of a 2p game must offer at least one MovePresident");

    // Teleport encoding: empty path prices to the all-zero label.
    let some_area = *state.board_layout.areas.keys().next().expect("board has areas");
    let label = engine_route_label(&state, &rules, &[], some_area);
    assert_eq!((label.mover_cost, label.bank), (0, 0));
    assert!(label.player_credits.is_empty());
    let _: String = route_label_key(&label); // key building never panics
}
