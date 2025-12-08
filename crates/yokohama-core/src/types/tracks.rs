//! Church and Customs management board tracks.

use serde::{Deserialize, Serialize};

use crate::types::PlayerId;

/// A slot on the Church Management Board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChurchSlot {
    /// Player who placed here.
    pub player: PlayerId,
    /// Arrival order for tie-breaking (higher = later = wins ties).
    pub arrival_order: u8,
}

/// Church Management Board state.
/// Slots correspond to Faith values 4-11.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChurchTrack {
    /// 8 slots for Faith values 4-11.
    /// Index 0 = Faith 4, Index 7 = Faith 11.
    pub slots: [Option<ChurchSlot>; 8],
    /// Next arrival order number.
    pub next_arrival_order: u8,
}

impl ChurchTrack {
    pub fn new() -> Self {
        Self {
            slots: [None; 8],
            next_arrival_order: 0,
        }
    }

    /// Convert Faith value (4-11) to slot index (0-7).
    pub fn faith_to_index(faith: u8) -> Option<usize> {
        if (4..=11).contains(&faith) {
            Some((faith - 4) as usize)
        } else {
            None
        }
    }

    /// Convert slot index (0-7) to Faith value (4-11).
    pub fn index_to_faith(index: usize) -> u8 {
        (index + 4) as u8
    }

    /// VP reward for a given Faith value.
    pub fn vp_reward(faith: u8) -> u8 {
        match faith {
            4 => 3,
            5 => 4,
            6 => 5,
            7 => 6,
            8 => 7,
            9 => 8,
            10 => 11,
            11 => 14,
            _ => 0,
        }
    }

    /// Count assistants placed by a specific player.
    pub fn player_assistant_count(&self, player: PlayerId) -> u8 {
        self.slots
            .iter()
            .filter(|slot| slot.map(|s| s.player) == Some(player))
            .count() as u8
    }

    /// Total assistants placed (for end-game trigger).
    pub fn total_assistants(&self) -> u8 {
        self.slots.iter().filter(|s| s.is_some()).count() as u8
    }
}

impl Default for ChurchTrack {
    fn default() -> Self {
        Self::new()
    }
}

/// A slot on the Customs Management Board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomsSlot {
    /// Player who placed here.
    pub player: PlayerId,
    /// Arrival order for tie-breaking.
    pub arrival_order: u8,
}

/// Customs Management Board state.
/// Slots correspond to Imported Goods paid 1-7+.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomsTrack {
    /// 8 slots for Imported Goods 1-7+ (index 7 = 7 or more).
    pub slots: [Option<CustomsSlot>; 8],
    /// Next arrival order number.
    pub next_arrival_order: u8,
}

impl CustomsTrack {
    pub fn new() -> Self {
        Self {
            slots: [None; 8],
            next_arrival_order: 0,
        }
    }

    /// Convert Imported Goods count to slot index.
    pub fn goods_to_index(goods: u8) -> Option<usize> {
        if (1..=7).contains(&goods) {
            Some((goods - 1) as usize)
        } else if goods > 7 {
            Some(6) // Slot 7 handles 7+
        } else {
            None
        }
    }

    /// VP reward for a given Imported Goods count.
    pub fn vp_reward(goods: u8) -> u8 {
        match goods {
            1 => 5,
            2 => 4,
            3 => 10,
            4 => 9,
            5 => 16,
            6 => 15,
            _ => 22, // 7+
        }
    }

    /// Count assistants placed by a specific player.
    pub fn player_assistant_count(&self, player: PlayerId) -> u8 {
        self.slots
            .iter()
            .filter(|slot| slot.map(|s| s.player) == Some(player))
            .count() as u8
    }

    /// Total assistants placed (for end-game trigger).
    pub fn total_assistants(&self) -> u8 {
        self.slots.iter().filter(|s| s.is_some()).count() as u8
    }
}

impl Default for CustomsTrack {
    fn default() -> Self {
        Self::new()
    }
}

/// Global mercenary supply pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercenarySupply {
    pub english: u8,
    pub american: u8,
    pub french: u8,
    pub german: u8,
    pub dutch: u8,
}

impl MercenarySupply {
    /// Create supply with initial counts.
    pub fn new() -> Self {
        Self {
            english: 3,
            american: 2,
            french: 2,
            german: 2,
            dutch: 1,
        }
    }
}

impl Default for MercenarySupply {
    fn default() -> Self {
        Self::new()
    }
}
