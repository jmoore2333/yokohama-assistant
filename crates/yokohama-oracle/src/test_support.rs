//! Hand-built `GameState` fixtures for oracle unit tests (`cfg(test)` only).
//!
//! The engine's `create_test_game_state` / `new_game` are OFF the oracle's
//! allowlist (they are logic), so fixtures are assembled from the public type
//! constructors — exactly the "construct by hand" path the spec §6 preamble
//! anticipates. Fixtures are type-valid, not necessarily setup-valid (e.g. the
//! small board keeps a Canal even in a 2-player config): the oracle is a total
//! function over reachable states and the unit tests exercise its arithmetic,
//! not setup legality.

use yokohama_core::{
    AchievementCard, AchievementType, AreaConfig, AreaId, AreaType, BoardLayout,
    BoardLayoutBuilder, BuildingSite, ChurchTrack, CustomsTrack, FivePowerEffect, GameConfig,
    GameState, LabState, MercenarySupply, PlayerColor, PlayerId, PlayerState, PortState, TurnPhase,
};

/// A small board: areas 0..=3 are CopperMine, SilkMill, Church, Customs; area 4
/// is a Canal. Adjacency is the path 0–1–2–3 plus 1–4. Every non-Canal area
/// carries a trivial building site + 5-Power token so `build()` validates.
pub(crate) fn small_board() -> BoardLayout {
    let mut b = BoardLayoutBuilder::new();
    let areas = [
        (0u8, AreaType::CopperMine),
        (1, AreaType::SilkMill),
        (2, AreaType::Church),
        (3, AreaType::Customs),
        (4, AreaType::Canal),
    ];
    for (id, ty) in areas {
        let aid = AreaId::new(id);
        b.add_area(aid, AreaConfig { area_type: ty, grid_position: (0, id) });
        if ty != AreaType::Canal {
            b.set_building_site(aid, BuildingSite::new([1, 1, 1, 1], 3));
            b.set_five_power_token(aid, FivePowerEffect::Money(1));
        }
    }
    b.add_adjacency(AreaId::new(0), AreaId::new(1));
    b.add_adjacency(AreaId::new(1), AreaId::new(2));
    b.add_adjacency(AreaId::new(2), AreaId::new(3));
    b.add_adjacency(AreaId::new(1), AreaId::new(4));
    b.build().expect("small board validates")
}

/// [`two_player_state`] extended additively to 3 players (P2 Yellow): same
/// small board, `config` swapped to the 3-player one. Type-valid, not
/// setup-valid (e.g. `port_b` stays `None`) — Stage-2 scoring fixtures only
/// need seats and per-player state, never port boards.
pub(crate) fn three_player_state() -> GameState {
    let mut s = two_player_state();
    s.config = GameConfig::for_player_count(3);
    s.players.push(PlayerState::new(PlayerId::new(2), PlayerColor::Yellow, false));
    s
}

/// [`three_player_state`] extended additively to 4 players (P3 Green): same
/// small board, `config` swapped to the 4-player one. Type-valid, not
/// setup-valid — Stage-2 scoring fixtures need only seats and per-player state.
pub(crate) fn four_player_state() -> GameState {
    let mut s = three_player_state();
    s.config = GameConfig::for_player_count(4);
    s.players.push(PlayerState::new(PlayerId::new(3), PlayerColor::Green, false));
    s
}

/// A minimal 2-player state on [`small_board`]: both presidents in hand,
/// default resources, `AdditionalActionFirst`, empty queue. Tests mutate the
/// fields they care about.
pub(crate) fn two_player_state() -> GameState {
    let config = GameConfig::for_player_count(2);
    let players = vec![
        PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
        PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
    ];
    GameState {
        config,
        board_layout: small_board(),
        players,
        current_player: PlayerId::new(0),
        turn_phase: TurnPhase::AdditionalActionFirst,
        turn_number: 1,
        additional_action_state: Default::default(),
        current_action_power: None,
        is_mercenary_action: false,
        mercenary_target_area: None,
        mercenary_return_phase: None,
        president_start_area: None,
        port_state: PortState::new(false),
        lab_state: LabState::new(false),
        order_deck: Vec::new(),
        tech_deck: Vec::new(),
        orders_removed: Vec::new(),
        church_track: ChurchTrack::new(),
        customs_track: CustomsTrack::new(),
        achievements: [
            AchievementCard::new(AchievementType::Copper5),
            AchievementCard::new(AchievementType::Money10),
            AchievementCard::new(AchievementType::Production4),
        ],
        mercenary_supply: MercenarySupply::new(),
        station_location: None,
        end_game_triggered: false,
        trigger_reason: None,
        final_round: false,
        trigger_player: None,
        pending_choices: Vec::new(),
    }
}
