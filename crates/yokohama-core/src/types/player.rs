//! Player state and resource tracking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{AreaId, Country, OrderCardId, PlayerId, PlayerColor, TechCardId};

/// Where the President pawn is located.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresidentLocation {
    InHand,
    OnBoard(AreaId),
}

/// Tracking of assistant pawn locations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantState {
    pub in_hand: u8,
    pub in_warehouse: u8,
    /// Count per area on the main board.
    pub on_board: HashMap<AreaId, u8>,
    /// Locked on Achievement cards.
    pub on_achievements: u8,
    /// Locked on Church Management Board.
    pub on_church: u8,
    /// Locked on Customs Management Board.
    pub on_customs: u8,
}

impl AssistantState {
    /// Create initial assistant state for game start.
    pub fn new() -> Self {
        Self {
            in_hand: 8,
            in_warehouse: 12,
            on_board: HashMap::new(),
            on_achievements: 0,
            on_church: 0,
            on_customs: 0,
        }
    }

    /// Total assistants across all locations.
    pub fn total(&self) -> u8 {
        self.in_hand
            + self.in_warehouse
            + self.on_board.values().sum::<u8>()
            + self.on_achievements
            + self.on_church
            + self.on_customs
    }

    /// Count of assistants on board in a specific area.
    pub fn in_area(&self, area: AreaId) -> u8 {
        self.on_board.get(&area).copied().unwrap_or(0)
    }
}

impl Default for AssistantState {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracking of Shophouse token locations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShophouseState {
    pub in_hand: u8,
    pub in_warehouse: u8,
    /// Areas where shophouses are built (area -> slot index 0-3).
    pub built: HashMap<AreaId, u8>,
}

impl ShophouseState {
    pub fn new() -> Self {
        Self {
            in_hand: 2,
            in_warehouse: 6,
            built: HashMap::new(),
        }
    }

    pub fn total_built(&self) -> usize {
        self.built.len()
    }
}

impl Default for ShophouseState {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracking of Trading House token locations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradingHouseState {
    pub in_warehouse: u8,
    /// Areas where trading houses are built (order matters for cost).
    pub built: Vec<AreaId>,
}

impl TradingHouseState {
    pub fn new() -> Self {
        Self {
            in_warehouse: 4,
            built: Vec::new(),
        }
    }

    /// Cost to purchase next trading house from warehouse.
    pub fn next_cost(&self) -> u8 {
        match self.built.len() {
            0 => 4,
            1 => 5,
            2 => 6,
            3 => 7,
            _ => 0, // No more available
        }
    }

    pub fn total_built(&self) -> usize {
        self.built.len()
    }
}

impl Default for TradingHouseState {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracking of Foreign Mercenaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercenaryState {
    /// Face-up (usable) mercenaries by country.
    pub available: HashMap<Country, u8>,
    /// Face-down (used) mercenaries by country.
    pub used: HashMap<Country, u8>,
}

impl MercenaryState {
    pub fn new() -> Self {
        Self {
            available: HashMap::new(),
            used: HashMap::new(),
        }
    }

    /// Total mercenaries (both available and used).
    pub fn total(&self) -> u8 {
        self.available.values().sum::<u8>() + self.used.values().sum::<u8>()
    }

    /// Unused mercenaries (for end-game scoring).
    pub fn unused_count(&self) -> u8 {
        self.available.values().sum()
    }
}

impl Default for MercenaryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete state for a single player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: PlayerId,
    pub color: PlayerColor,

    // Victory tracking
    pub victory_points: u32,

    // Pawns and tokens
    pub president_location: PresidentLocation,
    pub assistants: AssistantState,
    pub shophouses: ShophouseState,
    pub trading_houses: TradingHouseState,

    // Resources
    pub money: u32,
    pub copper: u32,
    pub silk: u32,
    pub tea: u32,
    pub seafood: u32,
    pub imported_goods: u32,

    // Cards
    pub order_cards_in_hand: Vec<OrderCardId>,
    /// The starting Order card (hidden from other players).
    /// This is the only hidden information in base Yokohama.
    /// None if not yet assigned (pre-setup) or if revealed for some reason.
    pub starting_order: Option<OrderCardId>,
    /// True when this player chose to keep their starting order deal hidden (live-tracked
    /// opponent): the card's identity is not recorded anywhere in this state.
    pub starting_order_hidden: bool,
    pub completed_orders: Vec<OrderCardId>,
    pub technologies: Vec<TechCardId>,

    // Tokens collected
    pub five_power_tokens: u8, // Just count, effects already resolved
    pub mercenaries: MercenaryState,

    // Track positions (for tie-breaking: later position wins)
    pub church_positions: Vec<u8>,  // Faith values of placed assistants
    pub customs_positions: Vec<u8>, // Import values of placed assistants
}

impl PlayerState {
    /// Create a new player with starting resources.
    pub fn new(id: PlayerId, color: PlayerColor, is_start_player: bool) -> Self {
        Self {
            id,
            color,
            victory_points: 0,
            president_location: PresidentLocation::InHand,
            assistants: AssistantState::new(),
            shophouses: ShophouseState::new(),
            trading_houses: TradingHouseState::new(),
            money: if is_start_player { 3 } else { 4 },
            copper: 1,
            silk: 1,
            tea: 1,
            seafood: 1,
            imported_goods: 0,
            order_cards_in_hand: Vec::new(),
            starting_order: None,
            starting_order_hidden: false,
            completed_orders: Vec::new(),
            technologies: Vec::new(),
            five_power_tokens: 0,
            mercenaries: MercenaryState::new(),
            church_positions: Vec::new(),
            customs_positions: Vec::new(),
        }
    }

    /// Total trade goods (copper + silk + tea + seafood).
    pub fn total_trade_goods(&self) -> u32 {
        self.copper + self.silk + self.tea + self.seafood
    }
}
