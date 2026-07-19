//! Move representation for game actions.

use serde::{Deserialize, Serialize};

use crate::types::{
    AchievementType, AreaId, Country, MoveSource, OrderCardId, TechCardId, TradeGood, WarehouseItem,
};

/// Turn phases in order of execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TurnPhase {
    /// Phase A - Additional actions before main action.
    AdditionalActionFirst,

    /// Phase B Step 1 - Place assistants.
    Placement,
    /// Phase B Step 2 - Move President.
    Movement,
    /// Phase B Step 3 - Execute area action.
    AreaAction,
    /// Phase B Step 4 - Take 5-Power token if eligible.
    FivePowerBonus,
    /// Phase B Step 5 - Build if eligible.
    Building,
    /// Phase B Step 6 - Recover assistants.
    Recovery,

    /// Phase C - Additional actions after main action.
    AdditionalActionSecond,

    /// Turn complete, advance to next player.
    TurnEnd,
}

/// All possible moves in the game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Move {
    // Phase A/C: Additional Actions
    UseMercenary {
        country: Country,
        target_area: AreaId,
    },
    FulfillAchievement {
        achievement: AchievementType,
    },
    FulfillOrder {
        order_id: OrderCardId,
    },
    /// End additional action phase and proceed.
    EndAdditionalPhase,

    // Step 1: Placement
    /// Option A: Place 1 assistant in each of up to 3 different areas.
    PlaceThreeAreas {
        placements: Vec<AreaId>, // 0-3 areas
    },
    /// Option B: Place 2 assistants in 1 area.
    /// With Telegram tech, may also place 1 in an adjacent area.
    PlaceTwoSameArea {
        area: AreaId,
        /// Telegram bonus: place 1 assistant in adjacent area (None if not using).
        adjacent_bonus: Option<AreaId>,
    },

    // Step 2: Movement
    /// Move President along a path to destination.
    MovePresident {
        /// Intermediate areas (for payment tracking).
        path: Vec<AreaId>,
        destination: AreaId,
    },
    /// Return President to hand, optionally recovering assistants.
    ReturnPresidentToHand {
        recover_assistants_from: Vec<AreaId>,
    },
    /// Place President from hand onto board.
    PlacePresident {
        destination: AreaId,
    },

    // Step 3: Area Action
    ExecuteAreaAction {
        action: AreaAction,
    },

    // Step 4: 5-Power Bonus
    Take5PowerToken,
    Skip5PowerBonus,

    // Step 5: Building
    BuildShophouse {
        /// Slot index 0-3.
        slot: u8,
    },
    BuildTradingHouse,
    SkipBuilding,

    // Step 6: Recovery is automatic (no move needed)

    // Choice resolutions (design 2026-07-18): legal only while the matching
    // PendingChoice heads GameState::pending_choices.
    ChooseAnyGood { good: TradeGood },
    ChooseWarehouseTake { item: WarehouseItem },
    DeclineWarehouseTake,
    ChooseAssistantMove { from: MoveSource, to: AreaId },
    DeclineAssistantMove,
    ChooseChurchSpace { space: u8 },
    ChooseCustomsSpace { space: u8 },
}

/// Specific area actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AreaAction {
    // Production areas - just take based on power
    ProduceCopper,
    ProduceSilk,
    ProduceTea,
    ProduceSeafood,

    // Bank - just take money based on power
    Bank,

    // Port - choose which cards to take, named by card id (not slot: slots
    // shift under compaction/refill, so a card id is the only stable
    // reference a caller/CLI tracker can use across a take).
    Port {
        cards_taken: Vec<OrderCardId>,
        /// Payment for second card: Some(true) = 2 Yen, Some(false) = 1 Imported Good, None = only 1 card
        second_card_payment: Option<PortPayment>,
    },

    // Laboratory - choose which tech to acquire
    Laboratory {
        card_id: TechCardId,
        /// Extra Yen paid beyond power (for industry value).
        extra_yen: u8,
        /// Extra Imported Goods paid for industry value.
        extra_imported: u8,
        /// Extra payment for 5th+ tech: Some(true) = 2 Yen, Some(false) = 1 Imported Good
        fifth_tech_payment: Option<bool>,
    },

    // Employment Agency - choose which tokens to take
    EmploymentAgency {
        assistants: u8,
        shophouses: u8,
        trading_houses: u8,
    },

    // Chinatown - specify exchanges
    Chinatown {
        exchanges: Vec<ChinatownExchange>,
    },

    // Church - specify goods contribution
    Church {
        copper: bool,
        silk: bool,
        tea: bool,
        seafood: bool,
        yen: bool,
        imported_good: bool,
    },

    // Customs - specify imported goods to pay
    Customs {
        imported_goods_paid: u8,
    },

    /// Carry out the area action while declining all optional components:
    /// grants no cards, goods, or VP. Legal only at the six areas with
    /// optional components — Port A/B, Laboratory A/B, Employment Agency,
    /// Chinatown, Church, Customs (rulebook p.7/p.11-12 optionality ruling,
    /// decision log 2026-07-17). Illegal at production areas and the Bank,
    /// whose imperative "Gain ..." move is their only legal action.
    Skip,
}

/// Payment type for second Port card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortPayment {
    TwoYen,
    OneImportedGood,
}

/// A single exchange at Chinatown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChinatownExchange {
    // Sell goods for yen
    CopperToYen,  // 1 Copper -> 2 Yen
    SilkToYen,    // 1 Silk -> 2 Yen
    TeaToYen,     // 1 Tea -> 1 Yen
    SeafoodToYen, // 1 Seafood -> 1 Yen

    // Buy goods with yen
    YenToCopper,   // 2 Yen -> 1 Copper
    YenToSilk,     // 2 Yen -> 1 Silk
    YenToTea,      // 1 Yen -> 1 Tea
    YenToSeafood,  // 1 Yen -> 1 Seafood
    YenToImported, // 4 Yen -> 1 Imported Good
}
