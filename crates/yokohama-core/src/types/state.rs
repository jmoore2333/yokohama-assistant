//! Complete game state representation.

use serde::{Deserialize, Serialize};

use crate::types::{
    AchievementCard, AreaId, BoardLayout, ChurchTrack, CustomsTrack,
    LabBoard, MercenarySupply, OrderCardId, PlayerId, PlayerColor, PlayerState,
    PortBoard, TechCardId, TurnPhase,
};

/// Game configuration (immutable after setup).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameConfig {
    pub player_count: u8,
    pub player_colors: Vec<PlayerColor>,
    pub start_player: PlayerId,
    /// Whether Port B board is in use (3-4 players).
    pub has_port_b: bool,
    /// Whether Lab B board is in use (4 players only).
    pub has_lab_b: bool,
    /// Seed used by `new_game` to generate this game's board/decks (0 for legacy/manual construction).
    pub setup_seed: u64,
}

impl GameConfig {
    pub fn new(player_count: u8, player_colors: Vec<PlayerColor>, start_player: PlayerId) -> Self {
        debug_assert!((2..=4).contains(&player_count));
        debug_assert!(player_colors.len() == player_count as usize);
        Self {
            player_count,
            player_colors,
            start_player,
            has_port_b: player_count >= 3,
            has_lab_b: player_count >= 4,
            setup_seed: 0,
        }
    }

    /// Create configuration for a specific player count with default colors.
    pub fn for_player_count(player_count: u8) -> Self {
        debug_assert!((2..=4).contains(&player_count));
        let colors = match player_count {
            2 => vec![PlayerColor::Red, PlayerColor::Blue],
            3 => vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow],
            4 => vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green],
            _ => vec![PlayerColor::Red, PlayerColor::Blue],
        };
        Self::new(player_count, colors, PlayerId::new(0))
    }

    /// Track trigger threshold for Church/Customs based on player count.
    pub fn track_trigger(&self) -> u8 {
        match self.player_count {
            2 => 4,
            3 => 5,
            4 => 6,
            _ => 6,
        }
    }

    /// Areas to remove based on player count.
    /// Returns area types that should not be on the board for this player count.
    /// - 2 players: Lab B, Port B, Chinatown, Canal, 1 of each production
    /// - 3 players: Lab B, 1 of each production
    /// - 4 players: None removed
    pub fn removed_area_types(&self) -> Vec<crate::types::AreaType> {
        use crate::types::AreaType;
        match self.player_count {
            2 => vec![
                AreaType::LaboratoryB,
                AreaType::PortB,
                AreaType::Chinatown,
                AreaType::Canal,
            ],
            3 => vec![AreaType::LaboratoryB],
            _ => vec![],
        }
    }

    /// Starting money for a player based on turn order (rulebook setup step 9).
    /// Start player: 3 Yen, Others: 4 Yen.
    pub fn starting_money(&self, player: PlayerId) -> u32 {
        if player == self.start_player { 3 } else { 4 }
    }

    /// Number of order cards to remove from deck for this player count (rulebook setup step 12).
    pub fn order_cards_to_remove(&self) -> u8 {
        match self.player_count {
            2 => 16,
            3 => 5,
            4 => 0,
            _ => 0,
        }
    }

    /// Areas to mark as inactive for this player count.
    /// Returns area indices that should be removed.
    pub fn inactive_area_count(&self) -> u8 {
        match self.player_count {
            2 => 5,
            3 => 3,
            4 => 0,
            _ => 0,
        }
    }
}

/// Port boards state (A required, B only in 3-4 player).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortState {
    pub port_a: PortBoard,
    pub port_b: Option<PortBoard>,
}

impl PortState {
    pub fn new(has_port_b: bool) -> Self {
        Self {
            port_a: PortBoard::new(),
            port_b: if has_port_b { Some(PortBoard::new()) } else { None },
        }
    }
}

/// Laboratory boards state (A required, B only in 4 player).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabState {
    pub lab_a: LabBoard,
    pub lab_b: Option<LabBoard>,
}

impl LabState {
    pub fn new(has_lab_b: bool) -> Self {
        Self {
            lab_a: LabBoard::new(),
            lab_b: if has_lab_b { Some(LabBoard::new()) } else { None },
        }
    }
}

/// End game trigger reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EndGameTrigger {
    /// A player built all 4 Trading Houses.
    TradingHousesComplete(PlayerId),
    /// A player built all 8 Shophouses.
    ShophousesComplete(PlayerId),
    /// Order deck empty and a Port slot is empty.
    OrderDeckEmpty,
    /// Church track reached threshold.
    ChurchTrackFull,
    /// Customs track reached threshold.
    CustomsTrackFull,
}

/// State for tracking additional action phase and once-per-turn tech effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdditionalActionState {
    /// Has a mercenary been used this turn?
    pub mercenary_used: bool,
    /// Has Spinning Mill tech been used this turn? (discard 1 silk → gain 2 Yen)
    pub spinning_mill_used: bool,
    /// Has University tech been used this turn? (pay 1 Yen → take assistant from warehouse)
    pub university_used: bool,
}

impl AdditionalActionState {
    pub fn new() -> Self {
        Self {
            mercenary_used: false,
            spinning_mill_used: false,
            university_used: false,
        }
    }
}

impl Default for AdditionalActionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete game state - sufficient to resume at any point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    // Configuration (immutable after setup)
    pub config: GameConfig,
    pub board_layout: BoardLayout,

    // Dynamic state
    pub players: Vec<PlayerState>,
    pub current_player: PlayerId,
    pub turn_phase: TurnPhase,
    pub turn_number: u32,

    // Additional action tracking
    pub additional_action_state: AdditionalActionState,
    /// Power achieved in current turn's area action (for building eligibility).
    pub current_action_power: Option<u8>,

    // Mercenary sub-turn tracking
    /// True when executing a mercenary action (nested turn: AreaAction->Building->Recovery).
    /// After Recovery, returns to AdditionalAction phase instead of ending turn.
    pub is_mercenary_action: bool,
    /// The area being targeted by the current mercenary action.
    pub mercenary_target_area: Option<AreaId>,
    /// Which Additional Action phase to return to after mercenary sub-turn.
    pub mercenary_return_phase: Option<TurnPhase>,

    // Movement tracking
    /// President's starting area for this turn (cannot return here during movement).
    pub president_start_area: Option<AreaId>,

    // Shared card pools
    pub port_state: PortState,
    pub lab_state: LabState,
    /// Order card deck; last element is the top (drawn via `Vec::pop`).
    pub order_deck: Vec<OrderCardId>,
    /// Tech card deck; last element is the top (drawn via `Vec::pop`).
    pub tech_deck: Vec<TechCardId>,
    /// Order cards removed from play at setup (unused player-count reduction) or discarded
    /// as the "other" card in a revealed starting-order deal. Hidden-information bookkeeping:
    /// every card is in exactly one of `order_deck`, a player's hand, or here.
    pub orders_removed: Vec<OrderCardId>,

    // Tracks
    pub church_track: ChurchTrack,
    pub customs_track: CustomsTrack,

    // Achievements in play
    pub achievements: [AchievementCard; 3],

    // Global supply
    pub mercenary_supply: MercenarySupply,

    // Station token location (if Station tech acquired)
    pub station_location: Option<AreaId>,

    // End game tracking
    pub end_game_triggered: bool,
    pub trigger_reason: Option<EndGameTrigger>,
    /// True during final round after trigger.
    pub final_round: bool,
    /// Player who triggered end game (for round completion logic).
    pub trigger_player: Option<PlayerId>,
}

impl GameState {
    /// Get current player's state.
    pub fn current_player(&self) -> &PlayerState {
        &self.players[self.current_player.0 as usize]
    }

    /// Get current player's state mutably.
    pub fn current_player_mut(&mut self) -> &mut PlayerState {
        &mut self.players[self.current_player.0 as usize]
    }

    /// Get a player's state by ID.
    pub fn player(&self, id: PlayerId) -> &PlayerState {
        &self.players[id.0 as usize]
    }

    /// Get a player's state mutably.
    pub fn player_mut(&mut self, id: PlayerId) -> &mut PlayerState {
        &mut self.players[id.0 as usize]
    }

    /// Get the next player in turn order.
    pub fn next_player(&self) -> PlayerId {
        PlayerId::new((self.current_player.0 + 1) % self.config.player_count)
    }

    /// Check if the game has ended.
    pub fn is_game_over(&self) -> bool {
        // Game is over after final round completes
        if !self.end_game_triggered {
            return false;
        }

        if !self.final_round {
            return false;
        }

        // Final round: all players have taken one turn after trigger
        // This requires tracking - simplified: check if we've looped back to trigger player
        if let Some(_trigger_player) = self.trigger_player {
            // If current player would be the one BEFORE start player, round is complete
            let player_before_start = PlayerId::new(
                (self.config.start_player.0 + self.config.player_count - 1)
                    % self.config.player_count,
            );
            return self.current_player == player_before_start
                && self.turn_phase == TurnPhase::TurnEnd;
        }

        false
    }

    /// Promote an order card to the top of the deck by swapping it with the current top card.
    /// The named card's position and the deck's top position are exchanged; deck length unchanged.
    pub fn promote_order_to_top(&mut self, card: OrderCardId) -> Result<(), crate::types::MoveError> {
        let position = self.order_deck.iter().position(|&id| id == card)
            .ok_or(crate::types::MoveError::CardNotFound)?;
        let last_index = self.order_deck.len() - 1;
        self.order_deck.swap(position, last_index);
        Ok(())
    }

    /// Promote a tech card to the top of the deck by swapping it with the current top card.
    /// The named card's position and the deck's top position are exchanged; deck length unchanged.
    pub fn promote_tech_to_top(&mut self, card: TechCardId) -> Result<(), crate::types::MoveError> {
        let position = self.tech_deck.iter().position(|&id| id == card)
            .ok_or(crate::types::MoveError::CardNotFound)?;
        let last_index = self.tech_deck.len() - 1;
        self.tech_deck.swap(position, last_index);
        Ok(())
    }

    /// Arrange the order deck so the next `cards.len()` draws (`Vec::pop`) yield
    /// `cards` in order. Every named card must currently be in `order_deck`
    /// (the removed pile does not count — arrangement only serves future draws);
    /// the list must be duplicate-free. No mutation on error. Relative order of
    /// all unnamed cards is preserved.
    pub fn arrange_orders_on_top(&mut self, cards: &[OrderCardId]) -> Result<(), crate::types::MoveError> {
        for (i, c) in cards.iter().enumerate() {
            if cards[..i].contains(c) {
                return Err(crate::types::MoveError::RuleViolation("duplicate card in deck arrangement"));
            }
        }
        if !cards.iter().all(|c| self.order_deck.contains(c)) {
            return Err(crate::types::MoveError::CardNotFound);
        }
        self.order_deck.retain(|id| !cards.contains(id));
        self.order_deck.extend(cards.iter().rev());
        Ok(())
    }

    /// Tech-deck analog of `arrange_orders_on_top`, same contract.
    pub fn arrange_techs_on_top(&mut self, cards: &[TechCardId]) -> Result<(), crate::types::MoveError> {
        for (i, c) in cards.iter().enumerate() {
            if cards[..i].contains(c) {
                return Err(crate::types::MoveError::RuleViolation("duplicate card in deck arrangement"));
            }
        }
        if !cards.iter().all(|c| self.tech_deck.contains(c)) {
            return Err(crate::types::MoveError::CardNotFound);
        }
        self.tech_deck.retain(|id| !cards.contains(id));
        self.tech_deck.extend(cards.iter().rev());
        Ok(())
    }

    /// Extract an order card from the deck or removed pile.
    /// Searches `self.order_deck` first, then `self.orders_removed` if not found.
    /// Removes the card from whichever pile contains it.
    /// Returns `Err(MoveError::CardNotFound)` if the card is in neither pile.
    ///
    /// Rationale: when a live-tracked opponent kept their starting order hidden, `finalize_setup`
    /// parked both dealt cards in `orders_removed` (the kept card's identity was unknowable).
    /// A later reveal of that card must be able to pull it from `orders_removed`.
    pub fn extract_order_from_deck(&mut self, card: OrderCardId) -> Result<(), crate::types::MoveError> {
        // Try to find and remove from order_deck first
        if let Some(position) = self.order_deck.iter().position(|&id| id == card) {
            self.order_deck.remove(position);
            return Ok(());
        }

        // If not in order_deck, try orders_removed
        if let Some(position) = self.orders_removed.iter().position(|&id| id == card) {
            self.orders_removed.remove(position);
            return Ok(());
        }

        Err(crate::types::MoveError::CardNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::create_test_game_state;

    #[test]
    fn promote_order_to_top_swaps_within_deck() {
        let mut state = create_test_game_state(2);
        state.order_deck = vec![OrderCardId(1), OrderCardId(2), OrderCardId(3)]; // 3 = top
        state.promote_order_to_top(OrderCardId(1)).unwrap();
        assert_eq!(state.order_deck.last(), Some(&OrderCardId(1)));
        assert_eq!(state.order_deck.len(), 3);
        assert!(state.promote_order_to_top(OrderCardId(9)).is_err());
    }

    #[test]
    fn promote_tech_to_top_swaps_within_deck() {
        let mut state = create_test_game_state(2);
        state.tech_deck = vec![TechCardId(1), TechCardId(2), TechCardId(3)]; // 3 = top
        state.promote_tech_to_top(TechCardId(2)).unwrap();
        assert_eq!(state.tech_deck.last(), Some(&TechCardId(2)));
        assert_eq!(state.tech_deck.len(), 3);
        assert!(state.promote_tech_to_top(TechCardId(9)).is_err());
    }

    #[test]
    fn extract_order_from_deck_removes_from_deck() {
        let mut state = create_test_game_state(2);
        state.order_deck = vec![OrderCardId(1), OrderCardId(2), OrderCardId(3)];
        state.extract_order_from_deck(OrderCardId(2)).unwrap();
        assert_eq!(state.order_deck, vec![OrderCardId(1), OrderCardId(3)]);
        assert!(state.extract_order_from_deck(OrderCardId(9)).is_err());
    }

    #[test]
    fn extract_order_from_deck_searches_removed_pile() {
        let mut state = create_test_game_state(2);
        state.order_deck = vec![OrderCardId(1), OrderCardId(2)];
        state.orders_removed = vec![OrderCardId(10), OrderCardId(11)];

        // Extract from removed pile
        state.extract_order_from_deck(OrderCardId(10)).unwrap();
        assert_eq!(state.orders_removed, vec![OrderCardId(11)]);

        // Extract from deck if not in removed
        state.extract_order_from_deck(OrderCardId(1)).unwrap();
        assert_eq!(state.order_deck, vec![OrderCardId(2)]);

        // Error if in neither
        assert!(state.extract_order_from_deck(OrderCardId(99)).is_err());
    }

    #[test]
    fn arrange_orders_on_top_sets_pop_order_and_preserves_rest() {
        let mut state = create_test_game_state(2);
        state.order_deck = vec![OrderCardId(1), OrderCardId(2), OrderCardId(3), OrderCardId(4), OrderCardId(5)];
        state.arrange_orders_on_top(&[OrderCardId(2), OrderCardId(4)]).unwrap();
        // Next pops must yield 2 then 4; untouched cards keep relative order 1,3,5.
        assert_eq!(state.order_deck, vec![OrderCardId(1), OrderCardId(3), OrderCardId(5), OrderCardId(4), OrderCardId(2)]);
        assert_eq!(state.order_deck.pop(), Some(OrderCardId(2)));
        assert_eq!(state.order_deck.pop(), Some(OrderCardId(4)));
    }

    #[test]
    fn arrange_orders_on_top_rejects_missing_and_duplicate_without_mutation() {
        let mut state = create_test_game_state(2);
        state.order_deck = vec![OrderCardId(1), OrderCardId(2)];
        state.orders_removed = vec![OrderCardId(9)];
        let before = state.order_deck.clone();
        // 9 is in orders_removed, not the deck: arrangement only serves future DRAWS.
        assert_eq!(state.arrange_orders_on_top(&[OrderCardId(9)]), Err(crate::types::MoveError::CardNotFound));
        assert_eq!(state.order_deck, before);
        assert!(matches!(
            state.arrange_orders_on_top(&[OrderCardId(1), OrderCardId(1)]),
            Err(crate::types::MoveError::RuleViolation(_))
        ));
        assert_eq!(state.order_deck, before);
        assert!(state.arrange_orders_on_top(&[]).is_ok());
        assert_eq!(state.order_deck, before);
    }

    #[test]
    fn arrange_techs_on_top_sets_pop_order() {
        let mut state = create_test_game_state(2);
        state.tech_deck = vec![TechCardId(1), TechCardId(2), TechCardId(3)];
        state.arrange_techs_on_top(&[TechCardId(1)]).unwrap();
        assert_eq!(state.tech_deck.pop(), Some(TechCardId(1)));
        assert_eq!(state.tech_deck, vec![TechCardId(2), TechCardId(3)]);
        assert_eq!(state.arrange_techs_on_top(&[TechCardId(9)]), Err(crate::types::MoveError::CardNotFound));
    }
}
