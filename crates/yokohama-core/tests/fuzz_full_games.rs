//! Task 18 (spec F1/F2): full-game random self-play fuzz gate.
//!
//! GATE STATUS: PASSING as of Task 18.5. The full sweep (5000 seeds x each
//! of 2p/3p/4p via `cargo run --release -p yokohama-core --example selfplay
//! -- 5000 <players> 0`) completes 15,000/15,000 games with invariants
//! checked after every apply. `fuzz_500_games_per_player_count` is the
//! CI-sized slice; it stays `#[ignore]`d only because 1500 full games are
//! too slow for the routine debug-profile suite — run it explicitly with
//! `cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored`.
//!
//! End-trigger coverage strategy (orchestrator-adjudicated, Task 18.4):
//! uniform-random play organically reaches OrderDeckEmpty and
//! ChurchTrackFull reliably, and essentially nothing else — random players
//! never hold customs imports, and the deck or church ends the game long
//! before anyone accumulates 8 shophouses or 4 trading houses. The corpus
//! test therefore asserts that organic PAIR, and the three rare triggers
//! are each proven END-TO-END by a seeded near-threshold scenario below:
//! surgically construct a legal state one step from the trigger, fire it
//! through the public `apply_move_unchecked` API, then random-play the
//! final round to a completed, invariant-checked game.
//!
//! MEASURED DISTRIBUTION (Phase 3.2 Task 10, re-measured 2026-07-26 at
//! `SEMANTICS_REV` 5; uniform policy, this binary's own driver, 500 games
//! per count at seeds 0..500 — i.e. exactly the corpus
//! `fuzz_500_games_per_player_count` builds — release, via
//! `cargo run --release -p yokohama-core --example selfplay -- 500 <n> 0`):
//! 2P OrderDeckEmpty 266 / ChurchTrackFull 231 / TradingHousesComplete 3;
//! 3P OrderDeckEmpty 416 / ChurchTrackFull 84; 4P OrderDeckEmpty 499 /
//! ChurchTrackFull 1. ShophousesComplete and CustomsTrackFull were 0 at
//! every count. This CORRECTS a stale line that said the other three
//! triggers "occurred zero times" across 15,000 games: TradingHousesComplete
//! does fire organically at 2P, rarely (3 in 500). It stays out of the
//! asserted pair deliberately — 3/500 at one player count is far too thin
//! to gate on, and its end-to-end proof is the scenario test below.
//!
//! The 4P ChurchTrackFull row (1/500 = 0.20%) is the trigger collapse
//! Phase 3.2's spec §0-R1 is written about, reproduced here at this HEAD:
//! uniform play barely reaches the church at 4P. That is a fact about the
//! POLICY, not about the engine, which is why the fix was a new policy and
//! a new binary (`tests/policy_diversity.rs`) rather than a change here —
//! this binary's uniform corpus is deliberately kept, since uniform and the
//! weighted policy cover disjoint halves of the apply surface.

mod common;
use common::{run_random_game, run_random_game_from};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move_unchecked, finalize_setup, get_legal_moves, new_game, AreaAction, AreaType,
    EndGameTrigger, GameConfig, GameState, Move, PlayerId, PresidentLocation, Rules,
    StartingOrderChoice, TurnPhase,
};

#[test]
#[ignore = "1500 full games: too slow for the debug suite; run with cargo test --release -- --ignored (gate PASSES; see module docs)"]
fn fuzz_500_games_per_player_count() {
    let rules = Rules::standard();
    let mut triggers_seen = std::collections::HashSet::new();

    for players in 2..=4u8 {
        for seed in 0..500u64 {
            let stats = run_random_game(seed, players, &rules)
                .unwrap_or_else(|e| panic!("seed {seed} {players}p: {e}"));
            if let Some(t) = stats.trigger {
                triggers_seen.insert(std::mem::discriminant(&t));
            }
        }
    }

    // Organic coverage: see module docs. The three rare triggers are
    // covered end-to-end by the scenario tests below.
    for t in [
        EndGameTrigger::OrderDeckEmpty,
        EndGameTrigger::ChurchTrackFull,
    ] {
        assert!(
            triggers_seen.contains(&std::mem::discriminant(&t)),
            "organically-reachable trigger {t:?} missing from the corpus"
        );
    }
}

#[test]
fn smoke_run_random_game_never_panics() {
    let rules = Rules::standard();
    for seed in 0..3u64 {
        let stats = run_random_game(seed, 2, &rules)
            .unwrap_or_else(|e| panic!("seed {seed} 2p must complete: {e}"));
        assert!(stats.trigger.is_some(), "completed game must record its trigger");
    }
}

/// Build a fresh seeded 2p standard game, ready for surgical adjustment.
fn fresh_2p(seed: u64) -> (GameState, ChaCha8Rng, Rules) {
    let rules = Rules::standard();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let config = GameConfig::for_player_count(2);
    let pending = new_game(config, &rules, seed).expect("new_game");
    let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)];
    let state = finalize_setup(pending, &choices).expect("finalize_setup");
    let _ = &mut rng;
    (state, rng, rules)
}

/// Put `player`'s president on `area` in the AreaAction phase at `power`.
fn stage_area_action(state: &mut GameState, player: PlayerId, area: yokohama_core::AreaId, power: u8) {
    state.current_player = player;
    state.turn_phase = TurnPhase::AreaAction;
    state.current_action_power = Some(power);
    state.players[player.0 as usize].president_location = PresidentLocation::OnBoard(area);
}

/// Find an area of the given type in the generated layout.
fn area_of_type(state: &GameState, ty: AreaType) -> yokohama_core::AreaId {
    *state
        .board_layout
        .areas
        .iter()
        .find(|(_, cfg)| cfg.area_type == ty)
        .map(|(id, _)| id)
        .expect("area type present in layout")
}

#[test]
fn scenario_customs_track_trigger_end_to_end() {
    let (mut state, mut rng, rules) = fresh_2p(101);
    let customs = area_of_type(&state, AreaType::Customs);

    // Surgery (conservation-respecting): three pawns already on customs
    // spaces 1, 2 and 3 - one more placement hits the 2p threshold of 4.
    for (i, (player, space)) in [(0u8, 1u8), (1, 2), (0, 3)].into_iter().enumerate() {
        let idx = (space - 1) as usize;
        let p = &mut state.players[player as usize];
        p.assistants.in_hand -= 1;
        p.assistants.on_customs += 1;
        state.customs_track.slots[idx] = Some(yokohama_core::CustomsSlot {
            player: PlayerId::new(player),
            arrival_order: i as u8,
        });
        state.customs_track.next_arrival_order = i as u8 + 1;
    }
    // The acting player pays 4 imports -> space 7 (the only empty cost-4
    // space) and needs an assistant in hand (guaranteed: started with 8).
    state.players[0].imported_goods = 4;
    stage_area_action(&mut state, PlayerId::new(0), customs, 5);

    let mv = Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid: 4 } };
    state = apply_move_unchecked(&state, &mv, &rules).expect("customs placement");
    assert!(state.end_game_triggered);
    assert!(matches!(state.trigger_reason, Some(EndGameTrigger::CustomsTrackFull)));

    let stats = run_random_game_from(state, &mut rng, 101, 2, &rules)
        .expect("random play finishes the triggered game");
    assert!(matches!(stats.trigger, Some(EndGameTrigger::CustomsTrackFull)));
}

#[test]
fn scenario_shophouses_complete_trigger_end_to_end() {
    let (mut state, mut rng, rules) = fresh_2p(102);

    // Surgery: player 0 has built 7 shophouses on 7 distinct site areas
    // (site cards and player state kept mutually consistent; conservation
    // 8 = 7 built + 1 in hand + 0 warehouse).
    let site_areas: Vec<yokohama_core::AreaId> =
        state.board_layout.building_sites.keys().copied().take(8).collect();
    assert!(site_areas.len() >= 8, "2p layout has at least 8 site areas");
    for &area in &site_areas[..7] {
        state.players[0].shophouses.built.insert(area, 0);
        state
            .board_layout
            .building_sites
            .get_mut(&area)
            .unwrap()
            .shophouses_built[0] = Some(PlayerId::new(0));
    }
    state.players[0].shophouses.in_hand = 1;
    state.players[0].shophouses.in_warehouse = 0;

    // Build the 8th at power 4 through the public API.
    let build_area = site_areas[7];
    stage_area_action(&mut state, PlayerId::new(0), build_area, 4);
    state.turn_phase = TurnPhase::Building;

    let slot = state.board_layout.building_sites[&build_area]
        .next_shophouse_slot(PlayerId::new(0))
        .expect("free slot") as u8;
    state = apply_move_unchecked(&state, &Move::BuildShophouse { slot }, &rules).expect("8th shophouse");
    assert!(state.end_game_triggered);
    assert!(matches!(
        state.trigger_reason,
        Some(EndGameTrigger::ShophousesComplete(p)) if p == PlayerId::new(0)
    ));

    let stats = run_random_game_from(state, &mut rng, 102, 2, &rules)
        .expect("random play finishes the triggered game");
    assert!(matches!(stats.trigger, Some(EndGameTrigger::ShophousesComplete(_))));
}

#[test]
fn scenario_trading_houses_complete_trigger_end_to_end() {
    let (mut state, mut rng, rules) = fresh_2p(103);

    // Surgery: player 0 has 3 trading houses built and 1 in hand
    // (conservation 4 = 3 built + 1 hand + 0 warehouse); site cards mark
    // the built ones.
    let site_areas: Vec<yokohama_core::AreaId> =
        state.board_layout.building_sites.keys().copied().take(4).collect();
    for &area in &site_areas[..3] {
        state.players[0].trading_houses.built.push(area);
        state.board_layout.building_sites.get_mut(&area).unwrap().trading_house_owner =
            Some(PlayerId::new(0));
    }
    state.players[0].trading_houses.in_hand = 1;
    state.players[0].trading_houses.in_warehouse = 0;

    // Build the 4th at power 5 through the public API.
    let build_area = site_areas[3];
    stage_area_action(&mut state, PlayerId::new(0), build_area, 5);
    state.turn_phase = TurnPhase::Building;

    state = apply_move_unchecked(&state, &Move::BuildTradingHouse, &rules).expect("4th trading house");
    assert!(state.end_game_triggered);
    assert!(matches!(
        state.trigger_reason,
        Some(EndGameTrigger::TradingHousesComplete(p)) if p == PlayerId::new(0)
    ));

    let stats = run_random_game_from(state, &mut rng, 103, 2, &rules)
        .expect("random play finishes the triggered game");
    assert!(matches!(stats.trigger, Some(EndGameTrigger::TradingHousesComplete(_))));
}

#[test]
fn corrupt_starting_state_is_reported_as_such_not_blamed_on_a_move() {
    // A state that is corrupt BEFORE any move (here: one assistant simply
    // vanished) must be reported as a corrupt starting state, not
    // misattributed to whichever move gets applied first (Task 18 scaffold
    // review finding: setup/surgery corruption would otherwise send
    // debugging into move-application logic).
    let (mut state, mut rng, rules) = fresh_2p(104);
    state.players[0].assistants.in_hand -= 1; // total 19 != 20

    let err = run_random_game_from(state, &mut rng, 104, 2, &rules)
        .expect_err("corrupt state must not play");
    assert!(
        err.contains("corrupt starting state"),
        "misattributed corruption: {}",
        err.lines().take(2).collect::<Vec<_>>().join(" | ")
    );
}

/// The gate's generated-move soundness property on a small corpus slice,
/// kept non-ignored: every move `get_legal_moves` offers must apply cleanly
/// (this is implicit in every completed game above, but a dedicated name
/// makes regressions here readable).
#[test]
fn legal_moves_all_apply_over_short_random_walk() {
    let rules = Rules::standard();
    let mut rng = ChaCha8Rng::seed_from_u64(7);
    let config = GameConfig::for_player_count(3);
    let pending = new_game(config, &rules, 7).expect("new_game");
    let choices = vec![
        StartingOrderChoice::Keep(0),
        StartingOrderChoice::Keep(1),
        StartingOrderChoice::Keep(0),
    ];
    let mut state = finalize_setup(pending, &choices).expect("finalize_setup");

    use rand::Rng;
    for _ in 0..300 {
        if yokohama_core::is_game_over(&state) {
            break;
        }
        let moves = get_legal_moves(&state, &rules);
        assert!(!moves.is_empty(), "no dead ends in a non-terminal state");
        // apply_move_unchecked: every move here is drawn from
        // get_legal_moves by construction, and this loop applies ALL of
        // them per state, not just the one picked -- checked apply_move
        // would double-enumerate get_legal_moves per move and go quadratic.
        for m in &moves {
            assert!(
                apply_move_unchecked(&state, m, &rules).is_ok(),
                "generated move must apply: {m:?}"
            );
        }
        let pick = moves[rng.gen_range(0..moves.len())].clone();
        state = apply_move_unchecked(&state, &pick, &rules).expect("picked move applies");
    }
}
