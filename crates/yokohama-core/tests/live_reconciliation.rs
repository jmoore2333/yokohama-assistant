//! Phase-2 live-reconciliation property tests (design 2026-07-18 §5.6):
//! a game recorded WITH reveal events replays to the exact final state,
//! both when reveals name the seeded draws (identity) and when they name
//! different cards (permuted — the live-tracking case).

mod common;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move, check_invariants, finalize_setup, get_legal_moves, is_game_over, new_game, replay,
    GameConfig, GameRecord, RecordedEvent, Rules, StartingOrderChoice,
};

/// Play up to `max_moves` random moves; before every deck-drawing move,
/// inject RevealOrders/RevealTechs events. `permute=false` names the exact
/// seeded upcoming draws (identity); `permute=true` names the BOTTOM cards
/// of the deck instead (a genuinely different assignment) when the deck is
/// deep enough, falling back to identity when it isn't.
fn play_with_reveals(seed: u64, players: u8, max_moves: usize, permute: bool, rules: &Rules) -> (yokohama_core::GameState, GameRecord) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let config = GameConfig::for_player_count(players);
    let pending = new_game(config.clone(), rules, seed).unwrap();
    let choices: Vec<StartingOrderChoice> =
        (0..players).map(|_| StartingOrderChoice::Keep(rng.gen_range(0..2u8))).collect();
    let mut record = GameRecord::new(seed, config, choices.clone());
    let mut state = finalize_setup(pending, &choices).unwrap();

    for _ in 0..max_moves {
        if is_game_over(&state) {
            break;
        }
        let moves = get_legal_moves(&state, rules);
        assert!(!moves.is_empty(), "dead end at turn {}", state.turn_number);
        let mv = moves[rng.gen_range(0..moves.len())].clone();

        let scratch = apply_move(&state, &mv, rules).expect("legal move applies");
        let d_orders = state.order_deck.len() - scratch.order_deck.len();
        let d_techs = state.tech_deck.len() - scratch.tech_deck.len();

        if d_orders > 0 {
            let cards: Vec<_> = if permute && state.order_deck.len() >= 2 * d_orders {
                state.order_deck.iter().take(d_orders).copied().collect() // bottom cards
            } else {
                state.order_deck.iter().rev().take(d_orders).copied().collect() // seeded draws
            };
            state.arrange_orders_on_top(&cards).unwrap();
            record.push_event(RecordedEvent::RevealOrders(cards));
        }
        if d_techs > 0 {
            let cards: Vec<_> = if permute && state.tech_deck.len() >= 2 * d_techs {
                state.tech_deck.iter().take(d_techs).copied().collect()
            } else {
                state.tech_deck.iter().rev().take(d_techs).copied().collect()
            };
            state.arrange_techs_on_top(&cards).unwrap();
            record.push_event(RecordedEvent::RevealTechs(cards));
        }

        state = apply_move(&state, &mv, rules).expect("legal move applies after arrangement");
        check_invariants(&state, rules).expect("invariants hold after every applied move");
        record.push_move(mv);
    }
    (state, record)
}

#[test]
fn identity_reveals_replay_to_identical_state() {
    let rules = Rules::standard();
    for players in [2u8, 3, 4] {
        for seed in 0..3u64 {
            let (final_state, record) = play_with_reveals(seed, players, 120, false, &rules);
            assert_eq!(replay(&record, &rules).unwrap(), final_state, "seed {seed} {players}p");
        }
    }
}

#[test]
fn permuted_reveals_replay_to_identical_state_and_hold_invariants() {
    let rules = Rules::standard();
    for players in [2u8, 3, 4] {
        for seed in 0..3u64 {
            let (final_state, record) = play_with_reveals(seed, players, 120, true, &rules);
            let replayed = replay(&record, &rules).unwrap();
            assert_eq!(replayed, final_state, "seed {seed} {players}p");
            check_invariants(&replayed, &rules).unwrap();
            // JSON round-trip of a reveal-bearing record preserves replay.
            let json = serde_json::to_string(&record).unwrap();
            let back: GameRecord = serde_json::from_str(&json).unwrap();
            assert_eq!(replay(&back, &rules).unwrap(), final_state);
        }
    }
}

#[test]
fn revealed_hidden_starting_order_replays() {
    let rules = Rules::standard();
    let config = GameConfig::for_player_count(2);
    let pending = new_game(config.clone(), &rules, 21).unwrap();
    let hidden_deal = pending.deals[1];
    let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown];
    let mut record = GameRecord::new(21, config, choices.clone());
    let mut state = finalize_setup(pending, &choices).unwrap();

    state.reveal_starting_order(yokohama_core::PlayerId::new(1), hidden_deal[0]).unwrap();
    record.push_event(RecordedEvent::RevealStartingOrder {
        player: yokohama_core::PlayerId::new(1),
        card: hidden_deal[0],
    });
    check_invariants(&state, &rules).unwrap();
    assert_eq!(replay(&record, &rules).unwrap(), state);
}

#[test]
fn revealed_five_power_token_replays_through_fold() {
    let rules = Rules::standard();
    let config = GameConfig::for_player_count(2);
    let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)];
    let mut record = GameRecord::new(33, config.clone(), choices.clone());
    let pending = new_game(config, &rules, 33).unwrap();
    let mut state = finalize_setup(pending, &choices).unwrap();
    // Pick any tokened area and reveal a DIFFERENT canonical effect there.
    let (&area, current) = state.board_layout.five_power_tokens.iter()
        .find(|(_, t)| t.is_some()).expect("setup places tokens");
    let observed = if *current == Some(yokohama_core::FivePowerEffect::Money(3)) {
        yokohama_core::FivePowerEffect::ImportedGoods(1)
    } else {
        yokohama_core::FivePowerEffect::Money(3)
    };
    state.reveal_five_power_token(area, observed).unwrap();
    record.push_event(RecordedEvent::RevealFivePowerToken { area, effect: observed });
    check_invariants(&state, &rules).unwrap();
    assert_eq!(replay(&record, &rules).unwrap(), state);
    assert_eq!(state.board_layout.five_power_tokens[&area], Some(observed));
}

#[test]
fn setup_observations_from_another_seed_replay_and_play_on() {
    // Full live-setup shape: board + achievements + ports + labs + own deal
    // observed from a DIFFERENT seed's game (standing in for the physical
    // table), then random play proceeds and the record replays.
    let rules = Rules::standard();
    let config = GameConfig::for_player_count(3);
    let table = new_game(config.clone(), &rules, 777).unwrap();
    let n = table.state.board_layout.areas.len();
    let obs = yokohama_core::SetupObservations {
        board: Some(yokohama_core::ObservedBoard {
            tiles: (0..n)
                .map(|i| table.state.board_layout.areas[&yokohama_core::AreaId::new(i as u8)].area_type)
                .collect(),
            sites: {
                // Any valid distinct assignment works as an observation;
                // reuse indices 0.. in cell order, skipping the Canal cell.
                let mut next = 0u8;
                (0..n)
                    .map(|i| {
                        let t = table.state.board_layout.areas[&yokohama_core::AreaId::new(i as u8)].area_type;
                        if t == yokohama_core::AreaType::Canal { None } else { let s = next; next += 1; Some(s) }
                    })
                    .collect()
            },
        }),
        achievements: Some([
            table.state.achievements[0].achievement_type,
            table.state.achievements[1].achievement_type,
            table.state.achievements[2].achievement_type,
        ]),
        port_a: Some(std::array::from_fn(|i| table.state.port_state.port_a.slots[i].unwrap())),
        port_b: Some(std::array::from_fn(|i| {
            table.state.port_state.port_b.as_ref().unwrap().slots[i].unwrap()
        })),
        lab_a: Some(std::array::from_fn(|i| table.state.lab_state.lab_a.slots[i].unwrap())),
        lab_b: None,
        deals: vec![(yokohama_core::PlayerId::new(0), table.deals[0])],
    };
    let choices = vec![
        StartingOrderChoice::Keep(1),
        StartingOrderChoice::Unknown,
        StartingOrderChoice::Unknown,
    ];
    let mut record = GameRecord::new(31, config.clone(), choices.clone());
    record.setup_observations = Some(obs.clone());

    let pending = new_game(config, &rules, 31).unwrap();
    let pending = yokohama_core::reconcile_setup(pending, &obs, &rules).unwrap();
    let mut state = finalize_setup(pending, &choices).unwrap();
    check_invariants(&state, &rules).unwrap();

    let mut rng = ChaCha8Rng::seed_from_u64(31);
    for _ in 0..60 {
        if is_game_over(&state) {
            break;
        }
        let moves = get_legal_moves(&state, &rules);
        let mv = moves[rng.gen_range(0..moves.len())].clone();
        // Live twin: identity reveals for any draws this move makes.
        let scratch = apply_move(&state, &mv, &rules).unwrap();
        let d_o = state.order_deck.len() - scratch.order_deck.len();
        let d_t = state.tech_deck.len() - scratch.tech_deck.len();
        if d_o > 0 {
            let cards: Vec<_> = state.order_deck.iter().rev().take(d_o).copied().collect();
            record.push_event(RecordedEvent::RevealOrders(cards));
        }
        if d_t > 0 {
            let cards: Vec<_> = state.tech_deck.iter().rev().take(d_t).copied().collect();
            record.push_event(RecordedEvent::RevealTechs(cards));
        }
        state = apply_move(&state, &mv, &rules).unwrap();
        check_invariants(&state, &rules).unwrap();
        record.push_move(mv);
    }
    assert_eq!(replay(&record, &rules).unwrap(), state);
}
