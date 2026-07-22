//! Church and Customs management board tracks.

use serde::{Deserialize, Serialize};

use crate::types::PlayerId;

/// A slot on the Church Management Board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChurchSlot {
    /// Player who placed here.
    pub player: PlayerId,
    /// Arrival order, recorded at placement. NOT used by final scoring:
    /// the church/customs majority count-tie is POSITIONAL — rightmost
    /// occupied space wins (RB p.9's examples compare pawn positions;
    /// `game/scoring.rs`), differentially confirmed against the independent
    /// oracle 2026-07-20. This field's old "higher = later = wins ties"
    /// wording described a reading the engine never implemented.
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

    /// Real-player assistants on the track — the end-game trigger's currency.
    /// EXCLUDES the 2-player setup dummy ([`PlayerId::DUMMY`]): a 2P game ends
    /// on the 4th REAL pawn (the printed 5-total being 4 real + 1 dummy;
    /// finding D1), so the trigger's 4/5/6 thresholds must count real pawns
    /// only. The dummy still occupies its slot for blocking and majority
    /// scoring — it is just not a pawn the trigger tallies.
    pub fn total_assistants(&self) -> u8 {
        self.slots
            .iter()
            .flatten()
            .filter(|d| !d.player.is_dummy())
            .count() as u8
    }

    /// Movement-icon riders per Faith space (decision log 2026-07-17):
    /// Faith 4-11 -> 1,1,2,2,2,2,3,3. Zero for any other input (no printed
    /// rider off the track).
    pub fn movement_rider(space: u8) -> u8 {
        match space {
            4 | 5 => 1,
            6..=9 => 2,
            10 | 11 => 3,
            _ => 0,
        }
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
    /// Arrival order, recorded at placement. NOT used by final scoring:
    /// the church/customs majority count-tie is POSITIONAL — rightmost
    /// occupied space wins (RB p.9's examples compare pawn positions;
    /// `game/scoring.rs`), differentially confirmed against the independent
    /// oracle 2026-07-20. This field's old "higher = later = wins ties"
    /// wording described a reading the engine never implemented.
    pub arrival_order: u8,
}

/// Customs Management Board state. Board-art verified 2026-07-17: 7 printed
/// spaces numbered 1-7 left to right, each with a fixed printed cost
/// (imported goods) and VP reward. SUPERSEDES the earlier goods-keyed
/// 8-slot model (`goods_to_index` retired).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomsTrack {
    /// 7 slots for spaces 1-7. Index 0 = space 1, index 6 = space 7.
    pub slots: [Option<CustomsSlot>; 7],
    /// Next arrival order number.
    pub next_arrival_order: u8,
}

impl CustomsTrack {
    pub fn new() -> Self {
        Self {
            slots: [None; 7],
            next_arrival_order: 0,
        }
    }

    /// Convert space number (1-7) to slot index (0-6).
    pub fn space_to_index(space: u8) -> Option<usize> {
        if (1..=7).contains(&space) {
            Some((space - 1) as usize)
        } else {
            None
        }
    }

    /// Convert slot index (0-6) to space number (1-7).
    pub fn index_to_space(index: usize) -> u8 {
        (index + 1) as u8
    }

    /// Candidate spaces for a printed imported-goods cost, higher-VP
    /// (leftmost) space first: cost 1 -> {1,2}; cost 2 -> {3,4}; cost 3 ->
    /// {5,6}; cost 4 -> {7} alone. `None` for any other cost.
    pub fn spaces_for_cost(cost: u8) -> Option<&'static [u8]> {
        match cost {
            1 => Some(&[1, 2]),
            2 => Some(&[3, 4]),
            3 => Some(&[5, 6]),
            4 => Some(&[7]),
            _ => None,
        }
    }

    /// Printed imported-goods cost for a given space (1-7) -- the inverse
    /// of `spaces_for_cost`. SINGLE SOURCE for the CLI's `tracks_view`,
    /// which previously re-hardcoded this as its own array literal.
    pub fn space_cost(space: u8) -> u8 {
        match space {
            1 | 2 => 1,
            3 | 4 => 2,
            5 | 6 => 3,
            7 => 4,
            _ => 0,
        }
    }

    /// VP reward for a given space (1-7).
    pub fn vp_reward(space: u8) -> u8 {
        match space {
            1 => 5,
            2 => 4,
            3 => 10,
            4 => 9,
            5 => 16,
            6 => 15,
            7 => 22,
            _ => 0,
        }
    }

    /// Rider bonus for a given space, as (warehouse_takes, yen). Spaces 5-7
    /// carry no rider.
    pub fn rider(space: u8) -> (u8, u8) {
        match space {
            1 => (2, 0),
            2 => (0, 2),
            3 => (1, 0),
            4 => (0, 1),
            _ => (0, 0),
        }
    }

    /// Count assistants placed by a specific player.
    pub fn player_assistant_count(&self, player: PlayerId) -> u8 {
        self.slots
            .iter()
            .filter(|slot| slot.map(|s| s.player) == Some(player))
            .count() as u8
    }

    /// Real-player assistants on the track — the end-game trigger's currency.
    /// EXCLUDES the 2-player setup dummy ([`PlayerId::DUMMY`]): a 2P game ends
    /// on the 4th REAL pawn (the printed 5-total being 4 real + 1 dummy;
    /// finding D1), so the trigger's 4/5/6 thresholds must count real pawns
    /// only. The dummy still occupies its slot for blocking and majority
    /// scoring — it is just not a pawn the trigger tallies.
    pub fn total_assistants(&self) -> u8 {
        self.slots
            .iter()
            .flatten()
            .filter(|d| !d.player.is_dummy())
            .count() as u8
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
