//! Pending reward/recovery choices (design 2026-07-18 §5.1-5.2).
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeGood { Copper, Silk, Tea, Seafood }

/// Unresolved choice for the current player. Length of the queue is bounded
/// by one reward bundle's icon count plus its cascades (always small).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PendingChoice {
    /// Gain 1 trade good of choice (order any_trade_good / space any_good).
    AnyGood,
}
