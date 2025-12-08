//! Complete game state representation.

use serde::{Deserialize, Serialize};

use crate::types::{
    AchievementCard, AreaId, BoardLayout, ChurchTrack, CustomsTrack,
    LabBoard, MercenarySupply, PlayerId, PlayerColor, PlayerState,
    PortBoard, TurnPhase,
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

    /// Starting money for a player based on turn order.
    /// First player: 7 Yen, Others: 8 Yen.
    pub fn starting_money(&self, player: PlayerId) -> u32 {
        if player == self.start_player { 7 } else { 8 }
    }

    /// Number of order cards to remove from deck for this player count.
    pub fn order_cards_to_remove(&self) -> u8 {
        match self.player_count {
            2 => 10,
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

/// State for tracking additional action phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdditionalActionState {
    /// Has a mercenary been used this turn?
    pub mercenary_used: bool,
}

impl AdditionalActionState {
    pub fn new() -> Self {
        Self { mercenary_used: false }
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

    // Shared card pools
    pub port_state: PortState,
    pub lab_state: LabState,
    pub order_deck_remaining: u8,
    pub tech_deck_remaining: u8,

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
}
