//! Order cards and Technology cards.

use serde::{Deserialize, Serialize};

use crate::types::{Country, OrderCardId, PlayerId, TechCardId};

/// Trade goods required to complete an Order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TradeGoodRequirement {
    pub copper: u8,
    pub silk: u8,
    pub tea: u8,
    pub seafood: u8,
}

impl TradeGoodRequirement {
    pub fn new(copper: u8, silk: u8, tea: u8, seafood: u8) -> Self {
        Self { copper, silk, tea, seafood }
    }

    /// Total goods required.
    pub fn total(&self) -> u8 {
        self.copper + self.silk + self.tea + self.seafood
    }
}

/// Rewards from completing an Order card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OrderRewards {
    pub victory_points: u8,
    pub money: u8,
    pub imported_goods: u8,
    pub copper: u8,
    pub silk: u8,
    pub tea: u8,
    pub seafood: u8,
    pub movement: u8,
    pub any_trade_good: u8,
    pub warehouse_tokens: u8, // Items from warehouse to hand
}

/// An Order card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderCard {
    pub id: OrderCardId,
    pub country: Country,
    pub requirements: TradeGoodRequirement,
    pub rewards: OrderRewards,
}

/// Technology card effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TechEffect {
    // Movement
    /// Move President to any area without opponent President.
    Tram,
    /// Pass through opponent Presidents without paying.
    Stagecoach,
    /// May stop and act in area with opponent President.
    PostalSystem,
    /// Place Station token; +1 power for all; owner can teleport.
    Station,

    // Building bonuses
    /// +2 VP when building Shophouse.
    Gaslight,
    /// +3 VP when building Trading House.
    ElectricalLight,

    // Production bonuses
    /// +1 Copper when receiving Copper.
    MiningTechnology,
    /// Once per turn: discard 1 Silk -> gain 2 Yen.
    SpinningMill,

    // Order/Tech acquisition bonuses
    /// +2 VP when completing Order card.
    Ball,
    /// +1 Yen when completing Order card.
    LanguageSchool,
    /// +2 VP when acquiring Technology card.
    PatentSystem,
    /// +1 Yen when acquiring Technology card.
    LetterPressPrinting,

    // Track bonuses
    /// +3 VP when placing assistant on Church.
    Winery,
    /// +4 VP when placing assistant on Customs.
    Brickyard,

    // Economy
    /// Turn start: if Yen <= 1, refill to 2 Yen.
    SystemOfStock,
    /// Once per turn: pay 1 Yen -> take assistant from Warehouse.
    University,

    // Placement
    /// Placement Option A: may place in 4 different areas.
    Newspaper,
    /// After placing 2 in same area, may place 1 in adjacent area.
    Telegram,
    /// Recovery: may leave 1 assistant in the action area.
    Telephone,

    // Instant
    /// Immediately gain 10 VP on acquisition.
    Exposition,
}

/// A Technology card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechCard {
    pub id: TechCardId,
    pub name: String,
    pub country: Country,
    pub industry_cost: u8,
    pub effect: TechEffect,
}

/// Port board state: 6 card slots, 2 per zone, left to right as
/// Low, Low, Mid, Mid, High, High (board-art verified 2026-07-17;
/// see `.superpowers/sdd/asset-sweep-report.md` Q3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortBoard {
    /// Indices 0-1 = Low, 2-3 = Mid, 4-5 = High.
    pub slots: [Option<OrderCardId>; 6],
}

impl PortBoard {
    pub fn new() -> Self {
        Self { slots: [None; 6] }
    }

    /// Zone for a given slot index (0-5). Indices 0,1 = Low; 2,3 = Mid; 4,5 = High.
    /// The mapping is positional and fixed regardless of board contents.
    pub fn zone_of(index: usize) -> PortZone {
        match index {
            0 | 1 => PortZone::Low,
            2 | 3 => PortZone::Mid,
            _ => PortZone::High,
        }
    }

    /// Find the slot index holding the given card, if it's present on this board.
    pub fn find_card(&self, card_id: OrderCardId) -> Option<usize> {
        self.slots.iter().position(|slot| *slot == Some(card_id))
    }

    /// True if every slot is occupied.
    pub fn is_full(&self) -> bool {
        self.slots.iter().all(|s| s.is_some())
    }

    /// True if every slot is empty.
    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|s| s.is_none())
    }

    /// True if at least one slot is empty (drives the Order-deck-empty end game trigger).
    pub fn has_empty_slot(&self) -> bool {
        self.slots.iter().any(|s| s.is_none())
    }
}

impl Default for PortBoard {
    fn default() -> Self {
        Self::new()
    }
}

/// Zone on a Port board. Each zone spans 2 physical slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortZone {
    Low,
    Mid,
    High,
}

/// Laboratory board state: 6 tech slots, with Industry surcharges +1,+1,+2,+2
/// on slots 3-6 / 1-indexed (board-art verified 2026-07-17; see
/// `.superpowers/sdd/asset-sweep-report.md` Q4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabBoard {
    /// 6 slots, index 0 is leftmost (no position modifier).
    /// Position modifiers: slots 0-1 = +0, slots 2-3 = +1, slots 4-5 = +2.
    pub slots: [Option<TechCardId>; 6],
}

impl LabBoard {
    pub fn new() -> Self {
        Self { slots: [None; 6] }
    }

    /// Returns the position cost modifier for a slot (0, 0, 1, 1, 2, or 2).
    pub fn position_modifier(slot: usize) -> u8 {
        match slot {
            0 | 1 => 0,
            2 | 3 => 1,
            4 | 5 => 2,
            _ => 0,
        }
    }
}

impl Default for LabBoard {
    fn default() -> Self {
        Self::new()
    }
}

/// Achievement card categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AchievementCategory {
    A, // Trade good holdings
    B, // Collection goals
    C, // Building presence
}

/// Specific achievement types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AchievementType {
    // Category A: Trade goods (no payment, just demonstrate ownership)
    Copper5,   // Own 5 Copper
    Silk6,     // Own 6 Silk
    Tea7,      // Own 7 Tea
    Seafood7,  // Own 7 Seafood

    // Category B: Collection goals
    Money10,       // Own 10 Yen
    Tokens5,       // Own 5 total (5-Power + Mercenaries)
    Orders5,       // Complete 5 Orders
    Technologies4, // Own 4 Technologies

    // Category C: Building presence
    Production4, // Buildings in 4 Production areas
    Commercial4, // Buildings in 4 Commercial areas
    Prod3Comm2,  // 3 Production + 2 Commercial
    Prod2Comm3,  // 2 Production + 3 Commercial
}

impl AchievementType {
    pub fn category(&self) -> AchievementCategory {
        match self {
            AchievementType::Copper5
            | AchievementType::Silk6
            | AchievementType::Tea7
            | AchievementType::Seafood7 => AchievementCategory::A,
            AchievementType::Money10
            | AchievementType::Tokens5
            | AchievementType::Orders5
            | AchievementType::Technologies4 => AchievementCategory::B,
            AchievementType::Production4
            | AchievementType::Commercial4
            | AchievementType::Prod3Comm2
            | AchievementType::Prod2Comm3 => AchievementCategory::C,
        }
    }

    /// VP for first player to achieve.
    pub fn first_place_vp(&self) -> u8 {
        match self.category() {
            AchievementCategory::A => 8,
            AchievementCategory::B => 9,
            AchievementCategory::C => 10,
        }
    }

    /// VP for subsequent players.
    pub fn subsequent_vp(&self) -> u8 {
        match self.category() {
            AchievementCategory::A => 6,
            AchievementCategory::B => 7,
            AchievementCategory::C => 8,
        }
    }
}

/// State of an Achievement card in play.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AchievementCard {
    pub achievement_type: AchievementType,
    /// Players who have claimed this achievement (in order).
    pub claimed_by: Vec<PlayerId>,
}

impl AchievementCard {
    pub fn new(achievement_type: AchievementType) -> Self {
        Self {
            achievement_type,
            claimed_by: Vec::new(),
        }
    }

    pub fn vp_for_player(&self, player: PlayerId) -> u8 {
        if let Some(pos) = self.claimed_by.iter().position(|&p| p == player) {
            if pos == 0 {
                self.achievement_type.first_place_vp()
            } else {
                self.achievement_type.subsequent_vp()
            }
        } else {
            0
        }
    }
}
