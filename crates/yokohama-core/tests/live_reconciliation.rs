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
    let mut record = GameRecord::new(seed, config, choices.clone(), rules);
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

/// Review §3 blocker 2, the tech half: orders have a boxed-at-setup removed
/// pile (`orders_removed`, ~18 cards at 2p), which is why an order reveal can
/// name a card the seeded deck guessed out of play and `arrange_orders_on_top`
/// must exchange it in. The tech deck has NO such setup removal — every tech is
/// either in the deck or on a Lab board — so after a live setup reconciles the
/// observed Labs, the tech deck's membership is the EXACT complement of the
/// Labs (and of any owned techs). A tech reveal can therefore never name a card
/// the engine has out of the deck, so `arrange_techs_on_top` needs no exchange
/// path (a missing tech would be a genuine desync, correctly refused).
#[test]
fn tech_deck_membership_is_exact_after_live_setup_so_no_exchange_is_needed() {
    let rules = Rules::standard();
    for players in [2u8, 3, 4] {
        let config = GameConfig::for_player_count(players);
        // A different seed stands in for the physical table's Lab contents.
        let table = new_game(config.clone(), &rules, 4242).unwrap();
        let lab_a: [yokohama_core::TechCardId; 6] =
            std::array::from_fn(|i| table.state.lab_state.lab_a.slots[i].unwrap());
        let lab_b = table
            .state
            .lab_state
            .lab_b
            .as_ref()
            .map(|b| std::array::from_fn(|i| b.slots[i].unwrap()));
        let obs = yokohama_core::SetupObservations { lab_a: Some(lab_a), lab_b, ..Default::default() };
        let pending = new_game(config.clone(), &rules, 7).unwrap();
        let pending = yokohama_core::reconcile_setup(pending, &obs, &rules).unwrap();
        let choices: Vec<StartingOrderChoice> =
            (0..players).map(|_| StartingOrderChoice::Keep(0)).collect();
        let state = finalize_setup(pending, &choices).unwrap();

        // Every tech id (1..=30 across physical copies) is accounted for by
        // deck ∪ labs, with NO third pile: the deck is exactly the complement.
        let mut in_deck: std::collections::BTreeSet<u8> = state.tech_deck.iter().map(|c| c.0).collect();
        let lab_ids: std::collections::BTreeSet<u8> = state
            .lab_state
            .lab_a
            .slots
            .iter()
            .chain(state.lab_state.lab_b.iter().flat_map(|b| b.slots.iter()))
            .flatten()
            .map(|c| c.0)
            .collect();
        // Disjoint (a card can't be in the deck and a lab at once) and together
        // exactly the 30 physical tech ids.
        assert!(in_deck.is_disjoint(&lab_ids), "{players}p: a tech is both in the deck and a lab");
        in_deck.extend(&lab_ids);
        assert_eq!(in_deck.len(), 30, "{players}p: deck ∪ labs must be all 30 techs, no boxed pile");
        // And the observed labs really are what we asked for (reconcile worked).
        assert_eq!(std::array::from_fn::<_, 6, _>(|i| state.lab_state.lab_a.slots[i].unwrap()), lab_a);
    }
}

#[test]
fn revealed_hidden_starting_order_replays() {
    let rules = Rules::standard();
    let config = GameConfig::for_player_count(2);
    let pending = new_game(config.clone(), &rules, 21).unwrap();
    let hidden_deal = pending.deals[1];
    let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown];
    let mut record = GameRecord::new(21, config, choices.clone(), &rules);
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
            tokens: None,
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
    let mut record = GameRecord::new(31, config.clone(), choices.clone(), &rules);
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

#[test]
fn reconcile_moves_the_canal_and_adjacency_matches_direct_generation() {
    // Re-type a 3p board so the Canal MOVES: seeded seed-1 layout has the
    // Canal at AreaId(3), staggered grid (0,3), a top-row cell whose any-to-any
    // crossings are [(2,4),(2,8),(4,7)]; observe a tile permutation swapping it
    // to AreaId(9), grid (2,0), whose crossings differ (any-to-any transit,
    // decision log 2026-07-21).
    let rules = Rules::standard();
    let pending = new_game(GameConfig::for_player_count(3), &rules, 1).unwrap();
    let rows = yokohama_core::row_lengths(3);
    let n = yokohama_core::expected_tile_count(3);

    let mut tiles: Vec<yokohama_core::AreaType> = (0..n)
        .map(|i| pending.state.board_layout.areas[&yokohama_core::AreaId::new(i as u8)].area_type)
        .collect();
    assert_eq!(
        tiles[3],
        yokohama_core::AreaType::Canal,
        "fixture assumption: seed 1's 3p Canal must be at id 3"
    );
    assert_eq!(
        pending.state.board_layout.canal_crossings,
        vec![
            (yokohama_core::AreaId::new(2), yokohama_core::AreaId::new(4)),
            (yokohama_core::AreaId::new(2), yokohama_core::AreaId::new(8)),
            (yokohama_core::AreaId::new(4), yokohama_core::AreaId::new(7)),
        ],
        "fixture assumption: seed 1's 3p Canal crossings must be the top-row any-to-any set"
    );

    // Swap the Canal (id 3) with id 9: a valid multiset-preserving
    // permutation that moves the Canal to grid (2,0).
    tiles.swap(3, 9);
    let sites: Vec<Option<u8>> = {
        let mut next = 0u8;
        tiles
            .iter()
            .map(|t| {
                if *t == yokohama_core::AreaType::Canal {
                    None
                } else {
                    let s = next;
                    next += 1;
                    Some(s)
                }
            })
            .collect()
    };
    let obs = yokohama_core::SetupObservations {
        board: Some(yokohama_core::ObservedBoard { tiles, sites, tokens: None }),
        ..Default::default()
    };
    let reconciled = yokohama_core::reconcile_setup(pending, &obs, &rules).unwrap();
    let layout = &reconciled.state.board_layout;

    // Crossings follow the Canal's NEW cell exactly — reconcile reuses the same
    // `derive_canal_crossings` generation does — for AreaId(9) = grid (2,0).
    let canal_cell = yokohama_core::area_id_to_cell(yokohama_core::AreaId::new(9), &rows);
    let expected_new = yokohama_core::derive_canal_crossings(canal_cell, &rows);
    assert!(!expected_new.is_empty(), "moved Canal must still have transit crossings");
    assert_eq!(layout.canal_crossings, expected_new);
    for &(a, b) in &expected_new {
        assert!(layout.adjacent_areas(a).contains(&b));
        assert!(layout.adjacent_areas(b).contains(&a));
        assert!(layout.is_canal_crossing(a, b));
    }

    // The STALE seeded pair (2,4) is gone: those cells are not brick-offset
    // neighbors (2 apart on the same row), and with the Canal moved away they
    // are no longer a crossing either, so they must not be adjacent.
    assert!(!layout.adjacent_areas(yokohama_core::AreaId::new(2)).contains(&yokohama_core::AreaId::new(4)));
    assert!(!layout.adjacent_areas(yokohama_core::AreaId::new(4)).contains(&yokohama_core::AreaId::new(2)));

    // Adjacency otherwise equals the staggered brick edges plus the new
    // crossing pairs - identical to generating this exact tile arrangement directly.
    let mut expected_adjacency = std::collections::BTreeMap::<yokohama_core::AreaId, Vec<yokohama_core::AreaId>>::new();
    for (a, b) in yokohama_core::staggered_adjacency(&rows) {
        expected_adjacency.entry(a).or_default().push(b);
        expected_adjacency.entry(b).or_default().push(a);
    }
    for &(a, b) in &expected_new {
        expected_adjacency.entry(a).or_default().push(b);
        expected_adjacency.entry(b).or_default().push(a);
    }
    for v in expected_adjacency.values_mut() {
        v.sort_by_key(|id| id.0);
    }
    let mut got_adjacency = layout.adjacency.clone();
    for v in got_adjacency.values_mut() {
        v.sort_by_key(|id| id.0);
    }
    assert_eq!(got_adjacency, expected_adjacency);
}
