# A1-Foundation: Yokohama Game Engine Core Types

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Establish Cargo workspace and define all core data structures for Yokohama game state representation.

**Architecture:** Rust library crate (`yokohama-core`) with zero game logic in this phase - just type definitions, derives, and basic constructors. All structs derive common traits (Debug, Clone, PartialEq, Eq, Hash where appropriate). Enums are exhaustive with no placeholder variants.

**Tech Stack:** Rust 2021 edition, serde for serialization, no other dependencies yet.

**Reference Docs:**
- `analysis-1/yokohama-kb/03-state-schema.md` - Draft structs (use as starting point)
- `analysis-1/yokohama-kb/01-rules-reference.md` - Rule details for enum variants
- `analysis-1/yokohama-kb/02-quick-reference.md` - Lookup tables for constants

**Rules Scope:**
- Supports base Yokohama rules for **2–4 players** only (no expansions or variants)
- Uses the OKAZU Brand English rulebook as reference
- `yokohama-core` is the headless rules and state engine used by the AR Yokohama assistant; it is UI-agnostic and optimized for fast simulation

---

## Glossary: Rulebook Terms ↔ Rust Types

| Rulebook term      | Rust type / field                           |
|--------------------|---------------------------------------------|
| President pawn     | `PresidentLocation` in `PlayerState`        |
| Assistants         | `AssistantState`                            |
| Shophouse          | `ShophouseState`                            |
| Trading House      | `TradingHouseState`                         |
| 5-Power token      | `FivePowerEffect` / per-area `Option`       |
| Station token      | `station_location: Option<AreaId>` in state |
| Foreign Mercenary  | `MercenaryState` (player) / `MercenarySupply` (global) |
| Order card         | `OrderCard` / `OrderCardId`                 |
| Technology card    | `TechCard` / `TechCardId`                   |
| Church track       | `ChurchTrack`                               |
| Customs track      | `CustomsTrack`                              |
| Start Player Card  | `start_player: PlayerId` in `GameConfig`    |

---

## Design Notes

### Information Model
The game is **nearly perfect information**. The only hidden information is each player's **starting Order card** (drawn at setup, kept secret). All other state is public:
- All resource holdings, board positions, buildings
- Order cards taken from Port (everyone sees which)
- Technology cards owned
- Track positions

For the engine, we model this as:
- `GameState` always contains the actual card identities
- `PlayerState.starting_order: Option<OrderCardId>` tracks which card is hidden
- The AI knows 2 of 3 Order cards each opponent holds (only starting card is uncertain)
- For MCTS search, we treat the game as perfect information (the AI knows all cards)
- UI layer uses `starting_order` to hide card details from other players

### Global Components (in GameState)
These are not naturally per-area or per-player:
- **VP track**: Each player's current victory points
- **Church track**: Assistant positions and majority tracking
- **Customs track**: Assistant positions and majority tracking
- **Foreign Mercenary supply**: Pool of unclaimed mercenaries by country
- **Station token**: Which area has the station (if Station tech acquired)
- **Order/Tech decks**: Remaining cards not yet revealed

### 5-Power Tokens
5-Power tokens are **shuffled and assigned to areas at setup**; the engine treats them as runtime configuration rather than hard-coded area properties:
- Per-area field: `five_power_token: Option<FivePowerEffect>`
- No draw pile after setup (unused tokens returned to box)
- Token is claimed when a player achieves 5 power in that area

### Technology Effects as Event Hooks
`TechEffect` represents hooks into specific game phases, not arbitrary strings:

| Category | Example Cards | Hook Point |
|----------|---------------|------------|
| `OnGainTech` | Patent System (+2 VP) | After acquiring any tech |
| `ModifyMovement` | Tram (teleport), Stagecoach (free passage) | Movement step |
| `ModifyPower` | Station (+1 to all in area) | Power calculation |
| `OnBuild` | Gaslight (+2 VP), Electrical Light (+3 VP) | Building step |
| `OnOrder` | Ball (+2 VP), Language School (+1 yen) | Order fulfillment |
| `OnTrack` | Winery (+3 VP Church), Brickyard (+4 VP Customs) | Track placement |
| `ModifyPlacement` | Newspaper (4 areas), Telegram (bonus adjacent) | Placement step |
| `EndGameScoring` | (none in base game) | Final scoring |
| `Instant` | Exposition (immediate +10 VP) | On acquisition |

### Player-Count Setup Profiles
Setup varies by player count. `GameConfig` captures these differences:

| Setting | 2 Players | 3 Players | 4 Players |
|---------|-----------|-----------|-----------|
| Areas removed | Lab B, Port B, Chinatown, Canal, 1 of each production | Lab B, 1 of each production | None |
| Order cards removed | 16 | 5 | 0 |
| Track trigger | 4 assistants | 5 assistants | 6 assistants |
| Starting yen | 3 (start) / 4 (other) | Same | Same |

`GameState::new(config: GameConfig)` consumes this configuration rather than hard-coding assumptions.

---

## Task 1: Initialize Cargo Workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/yokohama-core/Cargo.toml`
- Create: `crates/yokohama-core/src/lib.rs`

**Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/yokohama-core",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
authors = ["Jeremy Moore"]

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
```

**Step 2: Create crate directories**

Run:
```bash
mkdir -p crates/yokohama-core/src
```

**Step 3: Create yokohama-core Cargo.toml**

```toml
[package]
name = "yokohama-core"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
description = "Core game engine for Yokohama board game"

[dependencies]
serde.workspace = true
```

**Step 4: Create initial lib.rs**

```rust
//! Yokohama board game core engine.
//!
//! This crate models the complete game rules for Yokohama,
//! a Meiji-era merchant trading game for 2-4 players.

pub mod types;

pub use types::*;
```

**Step 5: Create types module file**

Create: `crates/yokohama-core/src/types/mod.rs`

```rust
//! Core type definitions for Yokohama game state.

mod ids;
mod player;
mod board;
mod cards;
mod tracks;
mod moves;
mod state;

pub use ids::*;
pub use player::*;
pub use board::*;
pub use cards::*;
pub use tracks::*;
pub use moves::*;
pub use state::*;
```

**Step 6: Verify workspace builds**

Run:
```bash
cargo check
```

Expected: Compilation succeeds (with warnings about empty modules)

**Step 7: Commit**

```bash
git init
git add Cargo.toml crates/
git commit -m "chore: initialize Cargo workspace with yokohama-core crate"
```

---

## Task 2: Define ID Types and Basic Enums

**Files:**
- Create: `crates/yokohama-core/src/types/ids.rs`

**Step 1: Write ID types**

```rust
//! Strongly-typed identifiers and basic enumerations.

use serde::{Deserialize, Serialize};

/// Error type for invalid ID construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidIdError {
    pub type_name: &'static str,
    pub value: u8,
    pub max: u8,
}

impl std::fmt::Display for InvalidIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} must be 0-{}, got {}", self.type_name, self.max - 1, self.value)
    }
}

impl std::error::Error for InvalidIdError {}

/// Player identifier (0-3 for up to 4 players).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct PlayerId(pub u8);

impl PlayerId {
    /// Create a new PlayerId. Panics if id >= 4.
    pub fn new(id: u8) -> Self {
        assert!(id < 4, "PlayerId must be 0-3, got {}", id);
        Self(id)
    }

    /// Try to create a PlayerId, returning error for invalid values.
    /// Use this for CV-ingested data to avoid panics.
    pub fn try_new(id: u8) -> Result<Self, InvalidIdError> {
        if id < 4 {
            Ok(Self(id))
        } else {
            Err(InvalidIdError { type_name: "PlayerId", value: id, max: 4 })
        }
    }

    /// Create a PlayerId without validation (for deserialization).
    pub const fn unchecked(id: u8) -> Self {
        Self(id)
    }
}

impl TryFrom<u8> for PlayerId {
    type Error = InvalidIdError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

/// Area tile identifier on the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct AreaId(pub u8);

impl AreaId {
    /// Create a new AreaId. Panics if id >= 18.
    pub fn new(id: u8) -> Self {
        assert!(id < 18, "AreaId must be 0-17 (max 18 areas), got {}", id);
        Self(id)
    }

    /// Try to create an AreaId, returning error for invalid values.
    /// Use this for CV-ingested data to avoid panics.
    pub fn try_new(id: u8) -> Result<Self, InvalidIdError> {
        if id < 18 {
            Ok(Self(id))
        } else {
            Err(InvalidIdError { type_name: "AreaId", value: id, max: 18 })
        }
    }

    /// Create an AreaId without validation (for deserialization).
    pub const fn unchecked(id: u8) -> Self {
        Self(id)
    }
}

impl TryFrom<u8> for AreaId {
    type Error = InvalidIdError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

/// Order card identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderCardId(pub u8);

/// Technology card identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TechCardId(pub u8);

/// Player colors matching physical game components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerColor {
    Red,
    Blue,
    Yellow,
    Green,
}

impl PlayerColor {
    /// All colors in standard play order.
    pub const ALL: [PlayerColor; 4] = [
        PlayerColor::Red,
        PlayerColor::Blue,
        PlayerColor::Yellow,
        PlayerColor::Green,
    ];
}

/// Countries for set collection and Foreign Mercenaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Country {
    English,
    American,
    French,
    German,
    Dutch,
}

impl Country {
    /// All countries.
    pub const ALL: [Country; 5] = [
        Country::English,
        Country::American,
        Country::French,
        Country::German,
        Country::Dutch,
    ];

    /// Starting supply of Foreign Mercenaries per country.
    pub fn mercenary_supply(&self) -> u8 {
        match self {
            Country::English => 3,
            Country::American => 2,
            Country::French => 2,
            Country::German => 2,
            Country::Dutch => 1,
        }
    }
}
```

**Step 2: Verify it compiles**

Run:
```bash
cargo check -p yokohama-core
```

Expected: Success

**Step 3: Commit**

```bash
git add crates/yokohama-core/src/types/ids.rs
git commit -m "feat: add ID types and basic enums (PlayerId, AreaId, Country)"
```

---

## Task 3: Define Area Types

**Files:**
- Create: `crates/yokohama-core/src/types/board.rs`

**Step 1: Write area type enum and area configuration**

Reference: `01-rules-reference.md` "Area Types and Actions" section

```rust
//! Board layout and area definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
}

/// Configuration for a single area on the board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AreaConfig {
    pub area_type: AreaType,
    /// Grid position for layout reference (row, col).
    pub grid_position: (u8, u8),
}

/// Building site card placed on an area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingSite {
    /// VP rewards for Shophouse slots 1-4.
    pub shophouse_rewards: [u8; 4],
    /// VP reward for Trading House.
    pub trading_house_reward: u8,
    /// Which player owns shophouse in each slot (0-3), None if empty.
    pub shophouses_built: [Option<PlayerId>; 4],
    /// Which player owns the trading house, if any.
    pub trading_house_owner: Option<PlayerId>,
}

impl BuildingSite {
    /// Create a new empty building site with given VP values.
    pub fn new(shophouse_rewards: [u8; 4], trading_house_reward: u8) -> Self {
        Self {
            shophouse_rewards,
            trading_house_reward,
            shophouses_built: [None; 4],
            trading_house_owner: None,
        }
    }

    /// Returns the next available shophouse slot (0-3), or None if full.
    pub fn next_shophouse_slot(&self, player: PlayerId) -> Option<usize> {
        // Player can only have one shophouse per area
        if self.shophouses_built.iter().any(|&p| p == Some(player)) {
            return None;
        }
        self.shophouses_built.iter().position(|p| p.is_none())
    }

    /// Returns true if trading house can be built.
    pub fn can_build_trading_house(&self) -> bool {
        self.trading_house_owner.is_none()
    }
}

/// Effect from a 5-Power token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FivePowerEffect {
    ImportedGoods(u8),
    Copper(u8),
    Silk(u8),
    Tea(u8),
    Seafood(u8),
    Money(u8),
    VictoryPoints(u8),
    Movement(u8),
}

/// Complete board layout (immutable after game setup).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardLayout {
    /// Area tiles and their configurations.
    pub areas: HashMap<AreaId, AreaConfig>,

    /// Adjacency graph (bidirectional).
    pub adjacency: HashMap<AreaId, Vec<AreaId>>,

    /// Canal crossings that require 1 Yen payment.
    pub canal_crossings: Vec<(AreaId, AreaId)>,

    /// Building sites per area.
    pub building_sites: HashMap<AreaId, BuildingSite>,

    /// 5-Power tokens (None if already claimed).
    pub five_power_tokens: HashMap<AreaId, Option<FivePowerEffect>>,
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
    areas: HashMap<AreaId, AreaConfig>,
    adjacency: HashMap<AreaId, Vec<AreaId>>,
    canal_crossings: Vec<(AreaId, AreaId)>,
    building_sites: HashMap<AreaId, BuildingSite>,
    five_power_tokens: HashMap<AreaId, Option<FivePowerEffect>>,
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
        // Validate: all areas have building sites
        for area in self.areas.keys() {
            if !self.building_sites.contains_key(area) {
                return Err(BoardLayoutError::MissingBuildingSite(*area));
            }
        }

        // Validate: all areas have 5-Power tokens (Canal excluded)
        for (area, config) in &self.areas {
            if config.area_type != AreaType::Canal {
                if !self.five_power_tokens.contains_key(area) {
                    return Err(BoardLayoutError::MissingFivePowerToken(*area));
                }
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
}

/// Errors from BoardLayout validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardLayoutError {
    MissingBuildingSite(AreaId),
    MissingFivePowerToken(AreaId),
    InvalidFivePowerTokenArea(AreaId),
    InvalidAreaReference(AreaId),
    InvalidCanalCrossing(AreaId, AreaId),
}

impl std::fmt::Display for BoardLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBuildingSite(a) => write!(f, "Area {:?} missing building site", a),
            Self::MissingFivePowerToken(a) => write!(f, "Area {:?} missing 5-Power token", a),
            Self::InvalidFivePowerTokenArea(a) => write!(f, "5-Power token on invalid area: {:?}", a),
            Self::InvalidAreaReference(a) => write!(f, "Invalid area reference: {:?}", a),
            Self::InvalidCanalCrossing(a, b) => write!(f, "Invalid canal crossing: {:?} <-> {:?}", a, b),
        }
    }
}

impl std::error::Error for BoardLayoutError {}
```

**Step 2: Verify it compiles**

Run:
```bash
cargo check -p yokohama-core
```

Expected: Success

**Step 3: Commit**

```bash
git add crates/yokohama-core/src/types/board.rs
git commit -m "feat: add board types (AreaType, BuildingSite, BoardLayout)"
```

---

## Task 4: Define Player State Types

**Files:**
- Create: `crates/yokohama-core/src/types/player.rs`

**Step 1: Write player state structures**

Reference: `01-rules-reference.md` "Player Starting Resources" section

```rust
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
```

**Step 2: Verify it compiles**

Run:
```bash
cargo check -p yokohama-core
```

Expected: Success

**Step 3: Commit**

```bash
git add crates/yokohama-core/src/types/player.rs
git commit -m "feat: add player state types (AssistantState, PlayerState, etc.)"
```

---

## Task 5: Define Card Types

**Files:**
- Create: `crates/yokohama-core/src/types/cards.rs`

**Step 1: Write card structures**

Reference: `01-rules-reference.md` "Technology Cards Reference" and "Order Cards" sections

```rust
//! Order cards and Technology cards.

use serde::{Deserialize, Serialize};

use crate::types::{Country, OrderCardId, TechCardId};

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

impl TechEffect {
    /// Returns the base industry cost for this technology.
    /// Note: Some costs are uncertain in the reference; using best estimates.
    pub fn base_industry_cost(&self) -> u8 {
        match self {
            TechEffect::Tram => 5,
            TechEffect::Stagecoach => 3,
            TechEffect::PostalSystem => 4,
            TechEffect::Station => 4,
            TechEffect::Gaslight => 3,
            TechEffect::ElectricalLight => 4,
            TechEffect::MiningTechnology => 3,
            TechEffect::SpinningMill => 4,
            TechEffect::Ball => 5,
            TechEffect::LanguageSchool => 3,
            TechEffect::PatentSystem => 3,
            TechEffect::LetterPressPrinting => 3,
            TechEffect::Winery => 4,
            TechEffect::Brickyard => 4,
            TechEffect::SystemOfStock => 3,
            TechEffect::University => 4,
            TechEffect::Newspaper => 4,
            TechEffect::Telegram => 3,
            TechEffect::Telephone => 6,
            TechEffect::Exposition => 5,
        }
    }
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

/// Port board state (3 card slots).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortBoard {
    pub low: Option<OrderCardId>,
    pub mid: Option<OrderCardId>,
    pub high: Option<OrderCardId>,
}

impl PortBoard {
    pub fn new() -> Self {
        Self {
            low: None,
            mid: None,
            high: None,
        }
    }
}

impl Default for PortBoard {
    fn default() -> Self {
        Self::new()
    }
}

/// Port slot identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortSlot {
    Low,
    Mid,
    High,
}

/// Laboratory board state (4 card slots).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabBoard {
    /// 4 slots, index 0 is leftmost (no position modifier).
    /// Position modifiers: slot 2 = +1, slot 3 = +2.
    pub slots: [Option<TechCardId>; 4],
}

impl LabBoard {
    pub fn new() -> Self {
        Self { slots: [None; 4] }
    }

    /// Returns the position cost modifier for a slot (0, 0, 1, or 2).
    pub fn position_modifier(slot: usize) -> u8 {
        match slot {
            0 | 1 => 0,
            2 => 1,
            3 => 2,
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
```

**Step 2: Verify it compiles**

Run:
```bash
cargo check -p yokohama-core
```

Expected: Success

**Step 3: Commit**

```bash
git add crates/yokohama-core/src/types/cards.rs
git commit -m "feat: add card types (OrderCard, TechCard, TechEffect, Achievements)"
```

---

## Task 6: Define Track Types

**Files:**
- Create: `crates/yokohama-core/src/types/tracks.rs`

**Step 1: Write Church and Customs track structures**

Reference: `02-quick-reference.md` "CHURCH" and "CUSTOMS" sections

```rust
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
        if faith >= 4 && faith <= 11 {
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
        if goods >= 1 && goods <= 7 {
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
```

**Step 2: Verify it compiles**

Run:
```bash
cargo check -p yokohama-core
```

Expected: Success

**Step 3: Commit**

```bash
git add crates/yokohama-core/src/types/tracks.rs
git commit -m "feat: add track types (ChurchTrack, CustomsTrack, MercenarySupply)"
```

---

## Task 7: Define Move Types

**Files:**
- Create: `crates/yokohama-core/src/types/moves.rs`

**Step 1: Write move representation**

Reference: `03-state-schema.md` "Move Representation" section

```rust
//! Move representation for game actions.

use serde::{Deserialize, Serialize};

use crate::types::{
    AchievementType, AreaId, Country, OrderCardId, PortSlot, TechCardId,
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
    PlaceTwoSameArea {
        area: AreaId,
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

    // Port - choose which cards to take
    Port {
        cards_taken: Vec<PortSlot>,
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
```

**Step 2: Verify it compiles**

Run:
```bash
cargo check -p yokohama-core
```

Expected: Success

**Step 3: Commit**

```bash
git add crates/yokohama-core/src/types/moves.rs
git commit -m "feat: add move types (TurnPhase, Move, AreaAction)"
```

---

## Task 8: Define Game State

**Files:**
- Create: `crates/yokohama-core/src/types/state.rs`

**Step 1: Write complete game state**

```rust
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
}

impl GameConfig {
    pub fn new(player_count: u8, player_colors: Vec<PlayerColor>, start_player: PlayerId) -> Self {
        debug_assert!(player_count >= 2 && player_count <= 4);
        debug_assert!(player_colors.len() == player_count as usize);
        Self {
            player_count,
            player_colors,
            start_player,
        }
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
        if let Some(trigger_player) = self.trigger_player {
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
```

**Step 2: Verify it compiles**

Run:
```bash
cargo check -p yokohama-core
```

Expected: Success

**Step 3: Commit**

```bash
git add crates/yokohama-core/src/types/state.rs
git commit -m "feat: add complete GameState type"
```

---

## Task 9: Add Basic Tests

**Files:**
- Create: `crates/yokohama-core/src/types/tests.rs`
- Modify: `crates/yokohama-core/src/types/mod.rs`

**Step 1: Write basic unit tests**

```rust
//! Unit tests for core types.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_id_creation() {
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);
        assert_ne!(p0, p1);
        assert_eq!(p0.0, 0);
    }

    #[test]
    fn test_country_mercenary_supply() {
        assert_eq!(Country::English.mercenary_supply(), 3);
        assert_eq!(Country::Dutch.mercenary_supply(), 1);
    }

    #[test]
    fn test_area_type_categories() {
        assert!(AreaType::CopperMine.is_production());
        assert!(!AreaType::CopperMine.is_commercial());
        assert!(AreaType::Bank.is_commercial());
        assert!(!AreaType::Canal.allows_placement());
    }

    #[test]
    fn test_building_site_slot_allocation() {
        let mut site = BuildingSite::new([3, 1, 8, 4], 9);
        let player = PlayerId::new(0);

        // First slot available
        assert_eq!(site.next_shophouse_slot(player), Some(0));

        // Build in slot 0
        site.shophouses_built[0] = Some(player);

        // Player can't build again (one per area)
        assert_eq!(site.next_shophouse_slot(player), None);

        // Different player can build
        let player2 = PlayerId::new(1);
        assert_eq!(site.next_shophouse_slot(player2), Some(1));
    }

    #[test]
    fn test_assistant_state_totals() {
        let mut state = AssistantState::new();
        assert_eq!(state.total(), 20); // 8 hand + 12 warehouse

        state.on_board.insert(AreaId::new(0), 3);
        state.in_hand -= 3;
        assert_eq!(state.total(), 20); // Still 20 total
        assert_eq!(state.in_area(AreaId::new(0)), 3);
        assert_eq!(state.in_area(AreaId::new(1)), 0);
    }

    #[test]
    fn test_trading_house_costs() {
        let mut th = TradingHouseState::new();
        assert_eq!(th.next_cost(), 4);

        th.built.push(AreaId::new(0));
        assert_eq!(th.next_cost(), 5);

        th.built.push(AreaId::new(1));
        assert_eq!(th.next_cost(), 6);

        th.built.push(AreaId::new(2));
        assert_eq!(th.next_cost(), 7);
    }

    #[test]
    fn test_church_track_rewards() {
        assert_eq!(ChurchTrack::vp_reward(4), 3);
        assert_eq!(ChurchTrack::vp_reward(11), 14);
        assert_eq!(ChurchTrack::faith_to_index(4), Some(0));
        assert_eq!(ChurchTrack::faith_to_index(11), Some(7));
        assert_eq!(ChurchTrack::faith_to_index(3), None);
    }

    #[test]
    fn test_customs_track_rewards() {
        assert_eq!(CustomsTrack::vp_reward(1), 5);
        assert_eq!(CustomsTrack::vp_reward(7), 22);
        assert_eq!(CustomsTrack::vp_reward(10), 22); // 7+ all same
    }

    #[test]
    fn test_achievement_vp() {
        assert_eq!(AchievementType::Copper5.first_place_vp(), 8);
        assert_eq!(AchievementType::Copper5.subsequent_vp(), 6);
        assert_eq!(AchievementType::Money10.first_place_vp(), 9);
        assert_eq!(AchievementType::Production4.first_place_vp(), 10);
    }

    #[test]
    fn test_lab_position_modifier() {
        assert_eq!(LabBoard::position_modifier(0), 0);
        assert_eq!(LabBoard::position_modifier(1), 0);
        assert_eq!(LabBoard::position_modifier(2), 1);
        assert_eq!(LabBoard::position_modifier(3), 2);
    }

    #[test]
    fn test_game_config_track_trigger() {
        let config_2p = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));
        assert_eq!(config_2p.track_trigger(), 4);

        let config_4p = GameConfig::new(
            4,
            vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green],
            PlayerId::new(0),
        );
        assert_eq!(config_4p.track_trigger(), 6);
    }

    #[test]
    fn test_player_state_initial_resources() {
        let start_player = PlayerState::new(PlayerId::new(0), PlayerColor::Red, true);
        assert_eq!(start_player.money, 3);

        let other_player = PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false);
        assert_eq!(other_player.money, 4);

        // Both have same goods
        assert_eq!(start_player.copper, 1);
        assert_eq!(other_player.copper, 1);
    }
}
```

**Step 2: Add tests module to mod.rs**

Add at end of `crates/yokohama-core/src/types/mod.rs`:

```rust
#[cfg(test)]
mod tests;
```

**Step 3: Run tests**

Run:
```bash
cargo test -p yokohama-core
```

Expected: All tests pass

**Step 4: Commit**

```bash
git add crates/yokohama-core/src/types/
git commit -m "test: add unit tests for core types"
```

---

## Task 10: Verify Complete Build and Document

**Step 1: Run full check**

Run:
```bash
cargo check -p yokohama-core
cargo test -p yokohama-core
cargo doc -p yokohama-core --no-deps
```

Expected: All pass, documentation generates

**Step 2: Final commit with summary**

```bash
git add .
git commit -m "feat(A1-foundation): complete core type definitions

- Initialize Cargo workspace with yokohama-core crate
- Define ID types: PlayerId, AreaId, OrderCardId, TechCardId
- Define enums: PlayerColor, Country, AreaType, TurnPhase
- Define player state: AssistantState, ShophouseState, TradingHouseState
- Define board: BoardLayout, BuildingSite, FivePowerEffect
- Define cards: OrderCard, TechCard, TechEffect, Achievements
- Define tracks: ChurchTrack, CustomsTrack, MercenarySupply
- Define moves: Move, AreaAction, ChinatownExchange
- Define complete GameState
- Add unit tests for all major types"
```

---

## Summary

This plan creates the foundational layer for the Yokohama game engine:

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace root |
| `crates/yokohama-core/Cargo.toml` | Core crate config |
| `crates/yokohama-core/src/lib.rs` | Crate entry point |
| `crates/yokohama-core/src/types/mod.rs` | Types module |
| `crates/yokohama-core/src/types/ids.rs` | ID newtypes and basic enums |
| `crates/yokohama-core/src/types/board.rs` | Board layout types |
| `crates/yokohama-core/src/types/player.rs` | Player state types |
| `crates/yokohama-core/src/types/cards.rs` | Card types |
| `crates/yokohama-core/src/types/tracks.rs` | Management board tracks |
| `crates/yokohama-core/src/types/moves.rs` | Move representation |
| `crates/yokohama-core/src/types/state.rs` | Complete game state |
| `crates/yokohama-core/src/types/tests.rs` | Unit tests |

**Next:** Part 2 (A1-Implementation) will add game logic on top of these types.
