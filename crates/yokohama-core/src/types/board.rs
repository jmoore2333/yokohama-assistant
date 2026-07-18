//! Board layout and area definitions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::types::{AreaId, PlayerId};

/// All area types in Yokohama.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AreaType {
    // Production areas (4 types, some have duplicates)
    CopperMine,
    SilkMill,
    TeaPlantation,
    FishingGround,

    // Commercial areas
    Bank,
    PortA,
    PortB,
    LaboratoryA,
    LaboratoryB,
    EmploymentAgency,
    Chinatown,
    Church,
    Customs,

    // Special
    Canal,
}

impl AreaType {
    /// Returns true if this is a production area.
    pub fn is_production(&self) -> bool {
        matches!(
            self,
            AreaType::CopperMine
                | AreaType::SilkMill
                | AreaType::TeaPlantation
                | AreaType::FishingGround
        )
    }

    /// Returns true if this is a commercial area.
    pub fn is_commercial(&self) -> bool {
        matches!(
            self,
            AreaType::Bank
                | AreaType::PortA
                | AreaType::PortB
                | AreaType::LaboratoryA
                | AreaType::LaboratoryB
                | AreaType::EmploymentAgency
                | AreaType::Chinatown
                | AreaType::Church
                | AreaType::Customs
        )
    }

    /// Returns true if assistants can be placed here.
    pub fn allows_placement(&self) -> bool {
        !matches!(self, AreaType::Canal)
    }

    /// Returns true if this area has a building site.
    pub fn has_building_site(&self) -> bool {
        !matches!(self, AreaType::Canal)
    }

    /// Returns true if this area's action may be declined via
    /// `AreaAction::Skip` (rulebook p.7/p.11-12 optionality ruling, decision
    /// log 2026-07-17): the six areas with optional components. Production
    /// areas and the Bank have no optional component — their imperative
    /// "Gain ..." move is the only legal action there.
    pub fn allows_area_action_skip(&self) -> bool {
        matches!(
            self,
            AreaType::PortA
                | AreaType::PortB
                | AreaType::LaboratoryA
                | AreaType::LaboratoryB
                | AreaType::EmploymentAgency
                | AreaType::Chinatown
                | AreaType::Church
                | AreaType::Customs
        )
    }
}

/// Configuration for a single area on the board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AreaConfig {
    pub area_type: AreaType,
    /// Grid position for layout reference (row, col).
    pub grid_position: (u8, u8),
}

/// Reward printed on a single Shophouse space of a Building Site card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SpaceReward {
    pub vp: u8,
    pub yen: u8,
    pub seafood: u8,
    pub tea: u8,
    pub silk: u8,
    pub copper: u8,
    /// Wild trade-good token (any one of the four goods).
    pub any_good: u8,
    /// Movement icon (rulebook p.10 "Description of the Icons"): move an
    /// assistant from hand or an area to another area; no President fees.
    pub movement: u8,
    /// Warehouse icon (rulebook p.10): take a token (assistant/shophouse/
    /// trading house) from the Warehouse tile to hand, at Employment Agency cost.
    pub warehouse_tokens: u8,
}

/// Building site card placed on an area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingSite {
    /// Rewards for the four Shophouse spaces, in TL, TR, ML, MR order.
    pub spaces: [SpaceReward; 4],
    /// VP reward for Trading House.
    pub trading_house_reward: u8,
    /// Which player owns shophouse in each slot (0-3), None if empty.
    pub shophouses_built: [Option<PlayerId>; 4],
    /// Which player owns the trading house, if any.
    pub trading_house_owner: Option<PlayerId>,
}

impl BuildingSite {
    /// Create a new empty building site from VP-only rewards: a convenience
    /// constructor (each value becomes a SpaceReward with only `vp` set) for
    /// callers that don't care about the non-VP parts of a space's reward.
    pub fn new(vps: [u8; 4], trading_house_reward: u8) -> Self {
        Self {
            spaces: [
                SpaceReward { vp: vps[0], ..Default::default() },
                SpaceReward { vp: vps[1], ..Default::default() },
                SpaceReward { vp: vps[2], ..Default::default() },
                SpaceReward { vp: vps[3], ..Default::default() },
            ],
            trading_house_reward,
            shophouses_built: [None; 4],
            trading_house_owner: None,
        }
    }

    /// Returns the next available shophouse slot (0-3), or None if full.
    pub fn next_shophouse_slot(&self, player: PlayerId) -> Option<usize> {
        // Player can only have one shophouse per area
        if self.shophouses_built.contains(&Some(player)) {
            return None;
        }
        self.shophouses_built.iter().position(|p| p.is_none())
    }

    /// Returns true if trading house can be built.
    pub fn can_build_trading_house(&self) -> bool {
        self.trading_house_owner.is_none()
    }
}

/// Effect from a 5-Power token. Exactly these six faces exist in the
/// physical 20-token pool (box-verified 2026-07-18) -- earlier `VictoryPoints`
/// and `Movement` variants were test fiction and have been removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FivePowerEffect {
    ImportedGoods(u8),
    Copper(u8),
    Silk(u8),
    Tea(u8),
    Seafood(u8),
    Money(u8),
}

/// Complete board layout (immutable after game setup).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardLayout {
    /// Area tiles and their configurations.
    pub areas: BTreeMap<AreaId, AreaConfig>,

    /// Adjacency graph (bidirectional).
    pub adjacency: BTreeMap<AreaId, Vec<AreaId>>,

    /// Canal crossings that require 1 Yen payment.
    pub canal_crossings: Vec<(AreaId, AreaId)>,

    /// Building sites per area.
    pub building_sites: BTreeMap<AreaId, BuildingSite>,

    /// 5-Power tokens (None if already claimed).
    pub five_power_tokens: BTreeMap<AreaId, Option<FivePowerEffect>>,
}

impl BoardLayout {
    /// Get areas adjacent to the given area.
    pub fn adjacent_areas(&self, area: AreaId) -> &[AreaId] {
        self.adjacency.get(&area).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Check if crossing between two areas requires canal toll.
    pub fn is_canal_crossing(&self, from: AreaId, to: AreaId) -> bool {
        self.canal_crossings.contains(&(from, to)) || self.canal_crossings.contains(&(to, from))
    }
}

/// Builder for validated BoardLayout construction.
/// Ensures adjacency is bidirectional, sites exist for all areas, etc.
#[derive(Debug, Default)]
pub struct BoardLayoutBuilder {
    pub(crate) areas: BTreeMap<AreaId, AreaConfig>,
    pub(crate) adjacency: BTreeMap<AreaId, Vec<AreaId>>,
    pub(crate) canal_crossings: Vec<(AreaId, AreaId)>,
    pub(crate) building_sites: BTreeMap<AreaId, BuildingSite>,
    pub(crate) five_power_tokens: BTreeMap<AreaId, Option<FivePowerEffect>>,
}

impl BoardLayoutBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an area to the board.
    pub fn add_area(&mut self, id: AreaId, config: AreaConfig) -> &mut Self {
        self.areas.insert(id, config);
        self
    }

    /// Add bidirectional adjacency between two areas.
    pub fn add_adjacency(&mut self, a: AreaId, b: AreaId) -> &mut Self {
        self.adjacency.entry(a).or_default().push(b);
        self.adjacency.entry(b).or_default().push(a);
        self
    }

    /// Mark a canal crossing between two areas (1 Yen toll).
    pub fn add_canal_crossing(&mut self, a: AreaId, b: AreaId) -> &mut Self {
        self.canal_crossings.push((a, b));
        self
    }

    /// Set building site for an area.
    pub fn set_building_site(&mut self, area: AreaId, site: BuildingSite) -> &mut Self {
        self.building_sites.insert(area, site);
        self
    }

    /// Place a 5-power token on an area.
    pub fn set_five_power_token(&mut self, area: AreaId, effect: FivePowerEffect) -> &mut Self {
        self.five_power_tokens.insert(area, Some(effect));
        self
    }

    /// Build and validate the BoardLayout.
    pub fn build(self) -> Result<BoardLayout, BoardLayoutError> {
        // Validate: all areas have building sites (except Canal)
        for (area, config) in &self.areas {
            if config.area_type != AreaType::Canal && !self.building_sites.contains_key(area) {
                return Err(BoardLayoutError::MissingBuildingSite(*area));
            }
        }

        // Validate: all areas have 5-Power tokens (Canal excluded)
        for (area, config) in &self.areas {
            if config.area_type != AreaType::Canal
                && !self.five_power_tokens.contains_key(area)
            {
                return Err(BoardLayoutError::MissingFivePowerToken(*area));
            }
        }

        // Validate: 5-Power tokens only placed on valid areas
        for area in self.five_power_tokens.keys() {
            if !self.areas.contains_key(area) {
                return Err(BoardLayoutError::InvalidFivePowerTokenArea(*area));
            }
        }

        // Validate: all adjacency references valid areas
        for (area, neighbors) in &self.adjacency {
            if !self.areas.contains_key(area) {
                return Err(BoardLayoutError::InvalidAreaReference(*area));
            }
            for neighbor in neighbors {
                if !self.areas.contains_key(neighbor) {
                    return Err(BoardLayoutError::InvalidAreaReference(*neighbor));
                }
            }
        }

        // Validate: canal crossings reference valid areas
        for (a, b) in &self.canal_crossings {
            if !self.areas.contains_key(a) || !self.areas.contains_key(b) {
                return Err(BoardLayoutError::InvalidCanalCrossing(*a, *b));
            }
        }

        Ok(BoardLayout {
            areas: self.areas,
            adjacency: self.adjacency,
            canal_crossings: self.canal_crossings,
            building_sites: self.building_sites,
            five_power_tokens: self.five_power_tokens,
        })
    }

    /// Build and validate the BoardLayout for a specific game configuration.
    /// Checks that the board doesn't contain areas that should be removed for the player count.
    pub fn build_for_config(self, config: &super::GameConfig) -> Result<BoardLayout, BoardLayoutError> {
        let removed_types = config.removed_area_types();

        // Validate: no areas that should be removed for this player count
        for (area, area_config) in &self.areas {
            if removed_types.contains(&area_config.area_type) {
                return Err(BoardLayoutError::InvalidAreaForPlayerCount(
                    *area,
                    area_config.area_type,
                    config.player_count,
                ));
            }
        }

        // Then run standard validation
        self.build()
    }
}

/// Errors from BoardLayout validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardLayoutError {
    MissingBuildingSite(AreaId),
    MissingFivePowerToken(AreaId),
    InvalidFivePowerTokenArea(AreaId),
    InvalidAreaReference(AreaId),
    InvalidCanalCrossing(AreaId, AreaId),
    /// Area type should be removed for this player count.
    InvalidAreaForPlayerCount(AreaId, AreaType, u8),
}

impl std::fmt::Display for BoardLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBuildingSite(a) => write!(f, "Area {:?} missing building site", a),
            Self::MissingFivePowerToken(a) => write!(f, "Area {:?} missing 5-Power token", a),
            Self::InvalidFivePowerTokenArea(a) => write!(f, "5-Power token on invalid area: {:?}", a),
            Self::InvalidAreaReference(a) => write!(f, "Invalid area reference: {:?}", a),
            Self::InvalidCanalCrossing(a, b) => write!(f, "Invalid canal crossing: {:?} <-> {:?}", a, b),
            Self::InvalidAreaForPlayerCount(a, t, p) => {
                write!(f, "Area {:?} ({:?}) should be removed for {} players", a, t, p)
            }
        }
    }
}

impl std::error::Error for BoardLayoutError {}
