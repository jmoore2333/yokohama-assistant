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
/// Ord: Task 6 (L2 route enumeration) keys `RouteVector` -
/// `BTreeMap<Option<PlayerId>, u32>` - by fee recipient, which needs a
/// total order over PlayerId itself (the numeric id order is as good as
/// any; nothing depends on the specific ordering, only that it's total and
/// deterministic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
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

    /// Sentinel owner for the 2-player setup "dummy" assistant (rulebook p.9
    /// NOTE of **Jeremy's edition** — the English PDF bundled in `analysis-1/`
    /// describes no 2P dummy at all, so the citable source is guidingpath's
    /// 2026-07-21 decision-log rows, finding D1): a placed pawn of an unused
    /// colour that occupies Church faith-7 and Customs space-4. It is never a real player — no `PlayerState`
    /// carries this id, and `state.player(DUMMY)` would panic — so every
    /// consumer that enumerates real players (per-player invariants, the
    /// end-game trigger's pawn count, score breakdown rows) skips it, while
    /// the ONE representation that carries it (a track slot owned by DUMMY)
    /// still blocks placement and competes in the majority bonuses. The value
    /// sits far outside the real 0..=3 range so it can never collide.
    pub const DUMMY: PlayerId = PlayerId(u8::MAX);

    /// True for the 2-player majority "dummy" sentinel owner ([`PlayerId::DUMMY`]).
    pub fn is_dummy(self) -> bool {
        self == Self::DUMMY
    }
}

impl TryFrom<u8> for PlayerId {
    type Error = InvalidIdError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

/// Area tile identifier on the board.
/// Ord: BTreeMap keys in game state (deterministic enumeration order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
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
/// Ord: BTreeMap keys in game state (deterministic enumeration order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
