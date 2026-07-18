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

/// Where an Assistant pawn is moved FROM by the movement-icon reward
/// (rulebook p.10): the player's hand, or an Area currently holding one of
/// their own on-board assistants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveSource { Hand, Area(AreaId) }

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
}
