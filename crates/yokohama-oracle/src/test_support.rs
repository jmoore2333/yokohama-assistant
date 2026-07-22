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

/// A small STAGGERED board — a sub-pyramid of rows `[3, 2]`, the topology class
/// the generator actually produces (Task 4). Row 0 (widest, top): CopperMine(0)
/// at (0,0), SilkMill(1) at (0,1), Church(2) at (0,2). Row 1: Customs(3) at
/// (1,0), Canal(4) at (1,1). Adjacency is brick-offset — each lower tile
/// notches between the two upper tiles above it — so the free edges are
/// 0–1, 1–2 (row 0), 3–4 (row 1), and the notch edges 0–3, 1–3, 1–4, 2–4.
/// The Canal at (1,1) neighbors {1,2,3}; under the any-to-any model its only
/// non-brick-adjacent neighbor pair, Church(2)/Customs(3), is the single tolled
/// crossing (added to BOTH adjacency and canal_crossings). Area id→type is
/// unchanged from the old fixture (0 Copper, 1 Silk, 2 Church, 3 Customs, 4
/// Canal) so the type-driven scoring/track/building fixtures are unaffected;
/// only the geometry — never walked by those fixtures — is corrected. Every
/// non-Canal area carries a trivial building site + 5-Power token so `build()`
/// validates. Type-valid, not setup-valid (a 2-player board has no Canal): the
/// oracle is a total function over reachable states.
pub(crate) fn small_board() -> BoardLayout {
    let mut b = BoardLayoutBuilder::new();
    // (id, (row, col), type) — staggered `[3, 2]`, AreaIds row-major, top first.
    let areas = [
        (0u8, (0u8, 0u8), AreaType::CopperMine),
        (1, (0, 1), AreaType::SilkMill),
        (2, (0, 2), AreaType::Church),
        (3, (1, 0), AreaType::Customs),
        (4, (1, 1), AreaType::Canal),
    ];
    for (id, grid_position, ty) in areas {
        let aid = AreaId::new(id);
        b.add_area(aid, AreaConfig { area_type: ty, grid_position });
        if ty != AreaType::Canal {
            b.set_building_site(aid, BuildingSite::new([1, 1, 1, 1], 3));
            b.set_five_power_token(aid, FivePowerEffect::Money(1));
        }
    }
    // Brick-offset free edges (row-0 lateral, row-1 lateral, and the notch
    // edges that touch each lower tile to its two upper neighbors).
    for (a, z) in [(0, 1), (1, 2), (3, 4), (0, 3), (1, 3), (1, 4), (2, 4)] {
        b.add_adjacency(AreaId::new(a), AreaId::new(z));
    }
    // Any-to-any Canal transit: Church(2)↔Customs(3) is the one canal-neighbor
    // pair not otherwise adjacent, so it becomes a tolled crossing (also a real
    // adjacency edge — brick-adjacent pairs keep their free edge and no toll).
    b.add_adjacency(AreaId::new(2), AreaId::new(3));
    b.add_canal_crossing(AreaId::new(2), AreaId::new(3));
    b.build().expect("small staggered board validates")
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

#[cfg(test)]
mod staggered_pins {
    //! Task 4 fidelity pins: [`small_board`] must be STAGGERED-legal geometry
    //! (the topology class the generator produces), not the old single-row path
    //! with the corner-jumping `1–4` Canal edge. A regression to lattice/path
    //! geometry fails one of these loudly.
    use super::small_board;
    use yokohama_core::AreaId;

    /// The brick-offset neighbor relation over grid positions (board_gen's
    /// `neighbor_cells`, re-expressed): same row ⇒ lateral (|Δcol| == 1); rows
    /// one apart ⇒ the shorter lower row notches in, so colUpper − colLower ∈
    /// {0, 1}; rows ≥ 2 apart never touch. A same-row jump (the old `1–4`) and
    /// a corner-touch are exactly the non-edges this rejects.
    fn brick_adjacent(a: (u8, u8), b: (u8, u8)) -> bool {
        let (upper, lower) = if a.0 <= b.0 { (a, b) } else { (b, a) };
        match lower.0 - upper.0 {
            0 => upper.1.abs_diff(lower.1) == 1,
            1 => upper.1 == lower.1 || upper.1 == lower.1 + 1,
            _ => false,
        }
    }

    #[test]
    fn small_board_every_free_edge_is_brick_offset_no_lattice_leak() {
        let board = small_board();
        let pos = |id: AreaId| board.areas[&id].grid_position;
        for (a, neighbors) in &board.adjacency {
            for n in neighbors {
                if board.is_canal_crossing(*a, *n) {
                    continue; // transit edge, not physical tile contiguity
                }
                assert!(
                    brick_adjacent(pos(*a), pos(*n)),
                    "free edge {a:?}~{n:?} (grid {:?}~{:?}) is not brick-offset — \
                     lattice/path geometry leaked into small_board",
                    pos(*a),
                    pos(*n),
                );
            }
        }
    }

    #[test]
    fn small_board_canal_is_the_any_to_any_derivation() {
        // Canal = area 4 at (1,1); its neighbors are {1,2,3}. The only neighbor
        // pair NOT already brick-adjacent is Church(2)/Customs(3), so it is the
        // one tolled any-to-any crossing — present in BOTH adjacency (a real
        // transit edge) and canal_crossings (the toll marker).
        let board = small_board();
        let mut crossings: Vec<(u8, u8)> = board
            .canal_crossings
            .iter()
            .map(|&(a, z)| if a.0 <= z.0 { (a.0, z.0) } else { (z.0, a.0) })
            .collect();
        crossings.sort_unstable();
        assert_eq!(crossings, vec![(2, 3)], "expected exactly the (2,3) any-to-any transit");
        assert!(board.adjacent_areas(AreaId::new(2)).contains(&AreaId::new(3)));
        assert!(board.is_canal_crossing(AreaId::new(2), AreaId::new(3)));
    }
}
