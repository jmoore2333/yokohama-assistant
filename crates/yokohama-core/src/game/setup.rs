//! Game setup configuration and initialization.

use crate::game::board_gen::generate_board_layout;
use crate::game::cards::{refill_lab, refill_port};
use crate::game::rules::Rules;
use crate::types::*;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Create initial player states for a game configuration.
pub fn create_players(config: &GameConfig) -> Vec<PlayerState> {
    (0..config.player_count)
        .map(|i| {
            let player_id = PlayerId::new(i);
            let is_start = player_id == config.start_player;
            let mut player = PlayerState::new(player_id, config.player_colors[i as usize], is_start);
            // Set starting money based on turn order
            player.money = config.starting_money(player_id);
            player
        })
        .collect()
}

/// Create port state based on game configuration.
pub fn create_port_state(config: &GameConfig) -> PortState {
    PortState::new(config.has_port_b)
}

/// Create lab state based on game configuration.
pub fn create_lab_state(config: &GameConfig) -> LabState {
    LabState::new(config.has_lab_b)
}

/// A player's decision about their two-card starting-order deal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartingOrderChoice {
    /// Keep the card at this index (0 or 1) from the deal as the public starting order;
    /// the other card is removed from play.
    Keep(u8),
    /// Keep the deal private: neither card is recorded in hand, and the player's starting
    /// order stays hidden information (both dealt cards are removed from play fiction).
    Unknown,
}

/// Output of `new_game`: board, decks, and players are set up and starting-order cards are
/// dealt, but each player's keep/unknown choice hasn't been applied yet. Call `finalize_setup`
/// with the choices to get a playable `GameState`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingSetup {
    pub state: GameState,
    /// Two-card starting-order deal per player, indexed by `PlayerId`.
    pub deals: Vec<[OrderCardId; 2]>,
}

/// Achievement types available to be drawn for category A (trade good holdings).
const ACHIEVEMENTS_A: [AchievementType; 4] = [
    AchievementType::Copper5,
    AchievementType::Silk6,
    AchievementType::Tea7,
    AchievementType::Seafood7,
];
/// Achievement types available to be drawn for category B (collection goals).
const ACHIEVEMENTS_B: [AchievementType; 4] = [
    AchievementType::Money10,
    AchievementType::Tokens5,
    AchievementType::Orders5,
    AchievementType::Technologies4,
];
/// Achievement types available to be drawn for category C (building presence).
const ACHIEVEMENTS_C: [AchievementType; 4] = [
    AchievementType::Production4,
    AchievementType::Commercial4,
    AchievementType::Prod3Comm2,
    AchievementType::Prod2Comm3,
];

/// Deterministically generate a new game from a seed: randomized board layout, shuffled order
/// and tech decks (with player-count removal and Port/Lab initial fills applied), one random
/// Achievement per category, and a two-card starting-order deal per player. The same seed
/// always produces the same `PendingSetup`.
pub fn new_game(mut config: GameConfig, rules: &Rules, seed: u64) -> Result<PendingSetup, &'static str> {
    config.setup_seed = seed;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let board_layout = generate_board_layout(
        &config,
        &mut rng,
        rules.building_sites.clone(),
        rules.five_power_tokens.clone(),
    )
    .map_err(|_| "board generation failed")?;

    // Order deck: all 36, shuffle, remove by player count, deal 2 per player.
    let mut order_deck: Vec<OrderCardId> = rules.orders.iter().map(|c| c.id).collect();
    order_deck.sort_by_key(|id| id.0); // determinism before shuffle
    order_deck.shuffle(&mut rng);
    let orders_removed: Vec<OrderCardId> = (0..config.order_cards_to_remove())
        .filter_map(|_| order_deck.pop())
        .collect();
    let deals: Vec<[OrderCardId; 2]> = (0..config.player_count)
        .map(|_| [order_deck.pop().unwrap(), order_deck.pop().unwrap()])
        .collect();

    // Tech deck: all 30, shuffle.
    let mut tech_deck: Vec<TechCardId> = rules.techs.iter().map(|c| c.id).collect();
    tech_deck.sort_by_key(|id| id.0); // determinism before shuffle
    tech_deck.shuffle(&mut rng);

    // One random Achievement per category.
    let achievements = [
        AchievementCard::new(*ACHIEVEMENTS_A.choose(&mut rng).unwrap()),
        AchievementCard::new(*ACHIEVEMENTS_B.choose(&mut rng).unwrap()),
        AchievementCard::new(*ACHIEVEMENTS_C.choose(&mut rng).unwrap()),
    ];

    let players = create_players(&config);

    let mut state = GameState {
        config: config.clone(),
        board_layout,
        players,
        current_player: config.start_player,
        turn_phase: TurnPhase::AdditionalActionFirst,
        turn_number: 1,
        additional_action_state: AdditionalActionState::new(),
        current_action_power: None,
        is_mercenary_action: false,
        mercenary_target_area: None,
        mercenary_return_phase: None,
        president_start_area: None,
        port_state: create_port_state(&config),
        lab_state: create_lab_state(&config),
        order_deck,
        tech_deck,
        orders_removed,
        church_track: ChurchTrack::new(),
        customs_track: CustomsTrack::new(),
        achievements,
        mercenary_supply: MercenarySupply::new(),
        station_location: None,
        end_game_triggered: false,
        trigger_reason: None,
        final_round: false,
        trigger_player: None,
    };

    // Initial fills: Port A/B to 3 cards, Lab A/B to 4 cards.
    refill_port(&mut state.port_state.port_a, &mut state.order_deck);
    if let Some(port_b) = state.port_state.port_b.as_mut() {
        refill_port(port_b, &mut state.order_deck);
    }
    for _ in 0..4 {
        refill_lab(&mut state.lab_state.lab_a, &mut state.tech_deck);
    }
    if let Some(lab_b) = state.lab_state.lab_b.as_mut() {
        for _ in 0..4 {
            refill_lab(lab_b, &mut state.tech_deck);
        }
    }

    Ok(PendingSetup { state, deals })
}

/// Resolve each player's starting-order choice against a `PendingSetup`, producing a playable
/// `GameState`. `choices` must have one entry per player, in `PlayerId` order.
pub fn finalize_setup(
    pending: PendingSetup,
    choices: &[StartingOrderChoice],
) -> Result<GameState, &'static str> {
    let PendingSetup { mut state, deals } = pending;
    if choices.len() != deals.len() {
        return Err("choices length must match player count");
    }

    for (i, choice) in choices.iter().enumerate() {
        let deal = deals[i];
        match choice {
            StartingOrderChoice::Keep(kept_index) => {
                let kept_index = *kept_index as usize;
                if kept_index > 1 {
                    return Err("Keep index must be 0 or 1");
                }
                let kept = deal[kept_index];
                let other = deal[1 - kept_index];
                state.players[i].order_cards_in_hand.push(kept);
                state.players[i].starting_order = Some(kept);
                state.orders_removed.push(other);
            }
            StartingOrderChoice::Unknown => {
                state.players[i].starting_order_hidden = true;
                state.orders_removed.push(deal[0]);
                state.orders_removed.push(deal[1]);
            }
        }
    }

    Ok(state)
}

/// Validate that a GameState is consistent with its GameConfig.
/// Returns Ok(()) if valid, or an error message describing the inconsistency.
pub fn validate_game_state_consistency(state: &GameState) -> Result<(), &'static str> {
    let config = &state.config;

    // Validate player count
    if state.players.len() != config.player_count as usize {
        return Err("Player count mismatch");
    }

    // Validate Port B presence
    let has_port_b = state.port_state.port_b.is_some();
    if has_port_b != config.has_port_b {
        return Err("Port B presence mismatch");
    }

    // Validate Lab B presence
    let has_lab_b = state.lab_state.lab_b.is_some();
    if has_lab_b != config.has_lab_b {
        return Err("Lab B presence mismatch");
    }

    // Validate board layout doesn't have removed areas
    let removed_types = config.removed_area_types();
    for area_config in state.board_layout.areas.values() {
        if removed_types.contains(&area_config.area_type) {
            return Err("Board contains area that should be removed for player count");
        }
    }

    Ok(())
}

/// Create a minimal test game state for the given player count.
/// This is primarily for testing purposes.
pub fn create_test_game_state(player_count: u8) -> GameState {
    let config = GameConfig::for_player_count(player_count);
    let players = create_players(&config);

    GameState {
        config: config.clone(),
        board_layout: BoardLayout {
            areas: HashMap::new(),
            adjacency: HashMap::new(),
            canal_crossings: vec![],
            building_sites: HashMap::new(),
            five_power_tokens: HashMap::new(),
        },
        players,
        current_player: config.start_player,
        turn_phase: TurnPhase::AdditionalActionFirst,
        turn_number: 1,
        additional_action_state: AdditionalActionState::new(),
        current_action_power: None,
        is_mercenary_action: false,
        mercenary_target_area: None,
        mercenary_return_phase: None,
        president_start_area: None,
        port_state: create_port_state(&config),
        lab_state: create_lab_state(&config),
        order_deck: vec![],
        tech_deck: vec![],
        orders_removed: vec![],
        church_track: ChurchTrack::new(),
        customs_track: CustomsTrack::new(),
        achievements: [
            AchievementCard::new(AchievementType::Copper5),
            AchievementCard::new(AchievementType::Money10),
            AchievementCard::new(AchievementType::Production4),
        ],
        mercenary_supply: MercenarySupply::new(),
        station_location: None,
        end_game_triggered: false,
        trigger_reason: None,
        final_round: false,
        trigger_player: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_2_player_config() {
        let config = GameConfig::for_player_count(2);

        assert_eq!(config.player_count, 2);
        assert!(!config.has_port_b);
        assert!(!config.has_lab_b);
        assert_eq!(config.track_trigger(), 4);
        assert_eq!(config.order_cards_to_remove(), 16);
        assert_eq!(config.inactive_area_count(), 5);

        // Starting money
        assert_eq!(config.starting_money(PlayerId::new(0)), 3); // Start player
        assert_eq!(config.starting_money(PlayerId::new(1)), 4); // Other player
    }

    #[test]
    fn test_3_player_config() {
        let config = GameConfig::for_player_count(3);

        assert_eq!(config.player_count, 3);
        assert!(config.has_port_b);
        assert!(!config.has_lab_b);
        assert_eq!(config.track_trigger(), 5);
        assert_eq!(config.order_cards_to_remove(), 5);
        assert_eq!(config.inactive_area_count(), 3);
    }

    #[test]
    fn test_4_player_config() {
        let config = GameConfig::for_player_count(4);

        assert_eq!(config.player_count, 4);
        assert!(config.has_port_b);
        assert!(config.has_lab_b);
        assert_eq!(config.track_trigger(), 6);
        assert_eq!(config.order_cards_to_remove(), 0);
        assert_eq!(config.inactive_area_count(), 0);
    }

    #[test]
    fn test_create_players() {
        let config = GameConfig::for_player_count(3);
        let players = create_players(&config);

        assert_eq!(players.len(), 3);

        // Start player (player 0) gets 3 Yen
        assert_eq!(players[0].money, 3);
        assert_eq!(players[0].color, PlayerColor::Red);

        // Other players get 4 Yen
        assert_eq!(players[1].money, 4);
        assert_eq!(players[1].color, PlayerColor::Blue);
        assert_eq!(players[2].money, 4);
        assert_eq!(players[2].color, PlayerColor::Yellow);
    }

    #[test]
    fn test_port_state_by_player_count() {
        // 2 players: no Port B
        let config_2p = GameConfig::for_player_count(2);
        let port_2p = create_port_state(&config_2p);
        assert!(port_2p.port_b.is_none());

        // 3 players: has Port B
        let config_3p = GameConfig::for_player_count(3);
        let port_3p = create_port_state(&config_3p);
        assert!(port_3p.port_b.is_some());

        // 4 players: has Port B
        let config_4p = GameConfig::for_player_count(4);
        let port_4p = create_port_state(&config_4p);
        assert!(port_4p.port_b.is_some());
    }

    #[test]
    fn test_lab_state_by_player_count() {
        // 2 players: no Lab B
        let config_2p = GameConfig::for_player_count(2);
        let lab_2p = create_lab_state(&config_2p);
        assert!(lab_2p.lab_b.is_none());

        // 3 players: no Lab B
        let config_3p = GameConfig::for_player_count(3);
        let lab_3p = create_lab_state(&config_3p);
        assert!(lab_3p.lab_b.is_none());

        // 4 players: has Lab B
        let config_4p = GameConfig::for_player_count(4);
        let lab_4p = create_lab_state(&config_4p);
        assert!(lab_4p.lab_b.is_some());
    }

    #[test]
    fn test_create_test_game_state() {
        // 2-player game
        let state_2p = create_test_game_state(2);
        assert_eq!(state_2p.players.len(), 2);
        assert!(state_2p.port_state.port_b.is_none());
        assert!(state_2p.lab_state.lab_b.is_none());
        assert!(state_2p.order_deck.is_empty());

        // 4-player game
        let state_4p = create_test_game_state(4);
        assert_eq!(state_4p.players.len(), 4);
        assert!(state_4p.port_state.port_b.is_some());
        assert!(state_4p.lab_state.lab_b.is_some());
        assert!(state_4p.order_deck.is_empty());
    }

    #[test]
    fn test_start_player_rotation() {
        // Test that non-zero start player gets correct money
        let config = GameConfig::new(
            3,
            vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow],
            PlayerId::new(1), // Player 1 starts
        );

        let players = create_players(&config);

        assert_eq!(players[0].money, 4); // Not start player
        assert_eq!(players[1].money, 3); // Start player
        assert_eq!(players[2].money, 4); // Not start player
    }

    #[test]
    fn test_validate_game_state_consistency() {
        // Valid states should pass
        let state_2p = create_test_game_state(2);
        assert!(validate_game_state_consistency(&state_2p).is_ok());

        let state_3p = create_test_game_state(3);
        assert!(validate_game_state_consistency(&state_3p).is_ok());

        let state_4p = create_test_game_state(4);
        assert!(validate_game_state_consistency(&state_4p).is_ok());
    }

    #[test]
    fn test_validate_detects_player_count_mismatch() {
        let mut state = create_test_game_state(2);
        // Add an extra player - should now be invalid
        state.players.push(PlayerState::new(PlayerId::new(2), PlayerColor::Yellow, false));

        let result = validate_game_state_consistency(&state);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Player count mismatch");
    }

    #[test]
    fn test_validate_detects_port_b_mismatch() {
        let mut state = create_test_game_state(2);
        // 2p shouldn't have Port B, but let's add it
        state.port_state.port_b = Some(PortBoard::new());

        let result = validate_game_state_consistency(&state);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Port B presence mismatch");
    }

    #[test]
    fn test_validate_detects_lab_b_mismatch() {
        let mut state = create_test_game_state(3);
        // 3p shouldn't have Lab B, but let's add it
        state.lab_state.lab_b = Some(LabBoard::new());

        let result = validate_game_state_consistency(&state);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Lab B presence mismatch");
    }

    #[test]
    fn test_validate_detects_invalid_area_for_player_count() {
        let mut state = create_test_game_state(2);
        // 2p shouldn't have LaboratoryB
        state.board_layout.areas.insert(
            AreaId::new(17), // Valid area ID (0-17 range)
            AreaConfig {
                area_type: AreaType::LaboratoryB,
                grid_position: (0, 0),
            },
        );

        let result = validate_game_state_consistency(&state);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Board contains area that should be removed for player count");
    }

    #[test]
    fn new_game_is_seed_deterministic_and_decks_are_correct() {
        let rules = Rules::standard();
        let a = new_game(GameConfig::for_player_count(4), &rules, 42).unwrap();
        let b = new_game(GameConfig::for_player_count(4), &rules, 42).unwrap();
        assert_eq!(a.state, b.state);
        assert_eq!(a.deals, b.deals);
        // 4p: 36 orders - 0 removed - 8 dealt (2x4) - 6 port fill (A+B) = 22 in deck
        assert_eq!(a.state.order_deck.len(), 22);
        // 30 techs - 8 lab fill (A+B, 4 each) = 22
        assert_eq!(a.state.tech_deck.len(), 22);
        assert!(a.state.port_state.port_a.low.is_some());
        assert!(a.state.lab_state.lab_b.as_ref().unwrap().slots.iter().all(|s| s.is_some()));
    }

    #[test]
    fn finalize_setup_applies_keep_and_unknown_choices() {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 7).unwrap();
        let deals = pending.deals.clone();
        let state = finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown]).unwrap();
        // P0 kept deal[0]: it's in hand and recorded as starting order; deal[1] removed from play
        assert_eq!(state.players[0].order_cards_in_hand, vec![deals[0][0]]);
        assert_eq!(state.players[0].starting_order, Some(deals[0][0]));
        assert!(state.orders_removed.contains(&deals[0][1]));
        // P1 unknown: nothing in hand, flag set, both dealt cards out of the deck fiction
        assert!(state.players[1].order_cards_in_hand.is_empty());
        assert!(state.players[1].starting_order_hidden);
        assert!(state.orders_removed.contains(&deals[1][0]) && state.orders_removed.contains(&deals[1][1]));
    }
}
