//! Pending reward/recovery choices (design 2026-07-18 §5.1-5.3).
use serde::{Deserialize, Serialize};

use crate::types::AreaId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeGood { Copper, Silk, Tea, Seafood }

/// A takeable warehouse item (rulebook p.10 Warehouse icon / Employment
/// Agency take): which token type, not how many -- `PendingChoice::WarehouseTake`
/// is queued once per icon/rider take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarehouseItem { Assistant, Shophouse, TradingHouse }

impl WarehouseItem {
    /// Flat Yen cost for the item types whose price doesn't depend on
    /// player state: assistants are free, shophouses cost 2. `None` for
    /// TradingHouse, whose cost is the printed 4/5/6/7 progression indexed
    /// by `TradingHouseState::already_taken` (see `TradingHouseState::
    /// take_cost`). SINGLE SOURCE (T2-m3): both `warehouse_take_options`
    /// and the CLI's warehouse-item cost display previously re-hardcoded
    /// these two constants separately.
    pub fn flat_cost(self) -> Option<u32> {
        match self {
            WarehouseItem::Assistant => Some(0),
            WarehouseItem::Shophouse => Some(2),
            WarehouseItem::TradingHouse => None,
        }
    }
}

/// Where an Assistant pawn is moved FROM by the movement-icon reward
/// (rulebook p.10): the player's hand, or an Area currently holding one of
/// their own on-board assistants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveSource { Hand, Area(AreaId) }

/// Where the movement-icon reward moves the Assistant pawn TO: a (non-Canal)
/// Area, or — since the D3 owner ruling of 2026-08-08 — back to the owner's
/// HAND, where it simply stays. The rulebook p.10 text ("from your hand or
/// from an Area to another Area") is ambiguous on the to-hand return;
/// BGA's engine allows it, and the owner ruled for BGA parity while noting
/// the disagreement (guidingpath decision log, 2026-08-08). `Hand` is only
/// ever offered for an on-board source: hand-to-hand would be a structural
/// no-op indistinguishable from Decline, so the generator never emits it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveTarget { Hand, Area(AreaId) }

/// Unresolved choice for the current player. Length of the queue is bounded
/// by one reward bundle's icon count plus its cascades (always small).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PendingChoice {
    /// Gain 1 trade good of choice (order any_trade_good / space any_good).
    AnyGood,
    /// Take 1 item from the warehouse at Employment-Agency cost, or decline
    /// (order/space Warehouse icon; Customs rider).
    WarehouseTake,
    /// Move an Assistant pawn from hand or an Area to another Area, or
    /// decline (order/space Movement icon, rulebook p.10). No President
    /// fees apply -- this is not President movement.
    AssistantMove,
    /// Choose which eligible Church space (Faith 4-11) to place on, after
    /// goods have been paid and >=2 spaces are reachable at `total_faith`
    /// (design 2026-07-18 §5.2). No decline -- the action already validated
    /// that at least 1 eligible space exists; a singleton resolves inline in the
    /// executor instead of ever reaching this queue.
    ChurchSpace { total_faith: u8 },
    /// Choose which eligible Customs space to place on for a given exact
    /// printed `cost`, after imported goods have been paid and >=2 spaces
    /// are eligible at that cost (design 2026-07-18 §5.2, Task 5). No
    /// decline -- the action already validated >=1 eligible space exists; a
    /// singleton resolves inline in the executor instead of ever reaching
    /// this queue.
    CustomsSpace { cost: u8 },
    /// Telephone: leave 1 assistant in the Recovery acting area
    /// (`leave_one: true`) instead of recovering all of them
    /// (`leave_one: false`) (design 2026-07-18 §5.2, Task 6). Queued only
    /// when the current player owns Telephone AND has >=1 own assistant in
    /// the acting area; otherwise Recovery stays fully automatic.
    RecoveryLeave,
    /// Place the Station token — queued when the Station tech is acquired
    /// (design §4.2). No Decline: the token must land.
    StationPlace,
}
