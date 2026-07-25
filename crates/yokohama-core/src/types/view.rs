//! Seat-relative observation of a game (Phase 3.0).
//!
//! A `PlayerView` is what ONE seat may legitimately see. It is produced only
//! by `game::observe::observe`, which masks the five hidden members
//! (`order_deck` order, `tech_deck` order, `orders_removed` contents,
//! opponents' unfulfilled starting orders, and the setup seed). Bots take
//! views, never `GameState` -- that is the honesty boundary of the whole
//! computer-player ladder.
//!
//! This module is PURE DATA: no `crate::game` imports, no `&Rules` methods,
//! no hashing (the `&Rules` pool helpers live in `game::observe`, and
//! `view_digest` beside `state_digest` in `game::semantics`).

use serde::{Deserialize, Serialize};

use crate::types::{
    AchievementCard, AdditionalActionState, AreaId, AssistantState, BoardLayout, ChurchTrack,
    CustomsTrack, EndGameTrigger, LabState, MercenaryState, MercenarySupply, OrderCardId,
    PendingChoice, PlayerColor, PlayerId, PortState, PresidentLocation, ShophouseState, TechCardId,
    TradingHouseState, TurnPhase,
};

/// One entry of a seat's visible hand: either a card whose identity is public
/// (every Port take is witnessed by the whole table --
/// `analysis-1/yokohama-kb/01-rules-reference.md:703`) or the one slot holding
/// an unfulfilled starting order, whose identity is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandCard {
    Known(OrderCardId),
    HiddenStartingOrder,
}

/// The public half of `GameConfig`. Deliberately has NO seed field: a view
/// must not carry the game's generation provenance, and a zeroed `setup_seed`
/// would be indistinguishable from a genuine seed-0 game (seed 0 is the
/// repo's documented default base seed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewConfig {
    pub player_count: u8,
    pub player_colors: Vec<PlayerColor>,
    pub start_player: PlayerId,
    pub has_port_b: bool,
    pub has_lab_b: bool,
}

/// A player as seen from some seat. Identical to `PlayerState` except:
/// `order_cards_in_hand` becomes `hand` (one entry may be masked),
/// `starting_order` is `None` for every seat but the viewer, and
/// `starting_order_hidden` is dropped (the masked hand slot IS the signal).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisiblePlayer {
    pub id: PlayerId,
    pub color: PlayerColor,
    pub victory_points: u32,
    pub president_location: PresidentLocation,
    pub assistants: AssistantState,
    pub shophouses: ShophouseState,
    pub trading_houses: TradingHouseState,
    pub money: u32,
    pub copper: u32,
    pub silk: u32,
    pub tea: u32,
    pub seafood: u32,
    pub imported_goods: u32,
    pub hand: Vec<HandCard>,
    pub starting_order: Option<OrderCardId>,
    pub completed_orders: Vec<OrderCardId>,
    pub technologies: Vec<TechCardId>,
    pub five_power_tokens: u8,
    pub mercenaries: MercenaryState,
    pub church_positions: Vec<u8>,
    pub customs_positions: Vec<u8>,
}

/// Everything seat `viewer` may legitimately know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerView {
    pub viewer: PlayerId,
    pub config: ViewConfig,
    pub board_layout: BoardLayout,
    pub players: Vec<VisiblePlayer>,
    pub current_player: PlayerId,
    pub turn_phase: TurnPhase,
    pub turn_number: u32,
    pub additional_action_state: AdditionalActionState,
    pub current_action_power: Option<u8>,
    pub is_mercenary_action: bool,
    pub mercenary_target_area: Option<AreaId>,
    pub mercenary_return_phase: Option<TurnPhase>,
    pub president_start_area: Option<AreaId>,
    pub port_state: PortState,
    pub lab_state: LabState,
    /// Deck SIZE is public (a countable stack; the end trigger requires it to
    /// be observable -- `01-rules-reference.md:514`). Its ORDER is not.
    pub order_deck_len: usize,
    pub tech_deck_len: usize,
    /// The PHYSICAL box size, derived rather than copied -- see
    /// `game::observe::observe`. Under the live `Unknown` fiction the engine's
    /// `orders_removed` holds one extra card per hidden seat.
    pub orders_removed_len: usize,
    pub church_track: ChurchTrack,
    pub customs_track: CustomsTrack,
    pub achievements: [AchievementCard; 3],
    pub mercenary_supply: MercenarySupply,
    pub station_location: Option<AreaId>,
    pub end_game_triggered: bool,
    pub trigger_reason: Option<EndGameTrigger>,
    pub final_round: bool,
    pub trigger_player: Option<PlayerId>,
    pub pending_choices: Vec<PendingChoice>,
}

/// Why a seat view could not be derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewError {
    /// `PlayerId::DUMMY` is a track-slot sentinel with no `PlayerState`.
    DummySeat,
    SeatOutOfRange(PlayerId),
    /// The viewer's own starting-order deal was never recorded (a live
    /// `Unknown` on the viewer's own seat). A seat cannot be unknown to
    /// itself: such a state is a tracking artifact, not a seat view.
    ViewerDealUnrecorded(PlayerId),
    CorruptState(&'static str),
}

impl std::fmt::Display for ViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViewError::DummySeat => write!(f, "PlayerId::DUMMY is not a seat and has no view"),
            ViewError::SeatOutOfRange(p) => write!(f, "seat {} is not in this game", p.0),
            ViewError::ViewerDealUnrecorded(p) => write!(
                f,
                "seat {}'s own starting-order deal was never recorded; it cannot view itself",
                p.0
            ),
            ViewError::CorruptState(msg) => write!(f, "state is not viewable: {msg}"),
        }
    }
}

impl std::error::Error for ViewError {}
