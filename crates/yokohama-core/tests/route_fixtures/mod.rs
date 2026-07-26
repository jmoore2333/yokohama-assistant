//! Shared President-route test fixtures (Phase 3.2 Task 3; spec
//! `docs/plans/2026-07-25-phase3.2-arena-spec.md` §10.1).
//!
//! Cargo compiles every `tests/*.rs` as its own binary, so a fixture used by
//! several of them was previously REPLICATED into each — the dense 17-area cell
//! lived three times over (`action_source_equivalence.rs`,
//! `action_source_worstcase.rs`, and, from Task 2, `l10_enumeration_pin.rs`).
//! Replication made spec §4-2's same-fixture rule a matter of review
//! discipline: each copy's anti-vacuity check computes the frozen reference on
//! ITS OWN construction, so a drifted copy stays self-consistent and passes.
//! This module makes the rule STRUCTURAL — one construction, three consumers,
//! a drift impossible to introduce in only one of them. It is shared exactly
//! the way `tests/common/mod.rs` and `tests/action_cost_meter/mod.rs` already
//! are (a subdirectory module each binary declares).
//!
//! ## What lives here, and what deliberately does not
//!
//! Only the fixtures more than one binary needs, and only their CONSTRUCTION.
//! Every claim ABOUT a fixture — label floors, allocation bounds, ordered
//! comparisons against the frozen reference — stays in the consuming binary,
//! because those claims are what each gate owns and they differ per file. The
//! consumers' own single-use helpers (`state_with_areas`, `worst_case_state`,
//! `routes_diamond`, …) stay where they are.
//!
//! The dense cell's SHAPE originates in
//! `yokohama-difftest/tests/routes3_worstcase.rs` — the harness whose battery
//! first recorded the 458/519 label observations these cells reproduce — and is
//! re-built here rather than imported: `yokohama-core` does not depend on the
//! difftest crate, and a test crate is not a fixture library. That re-build is
//! deliberate and stays; what Task 3 removed is the re-build's re-re-builds
//! WITHIN this crate.
//!
//! ## The two cells and when to reach for which
//!
//! - [`dense_17_area_cell`] — the EXPENSIVE one: the real 4P staggered pyramid
//!   with an interior Canal, 17 own-assistant areas and saturating money. It is
//!   what the release-tier route gates measure (hundreds of labels; the `plain`
//!   geometry has reproduced 458 across three independent harnesses), and it
//!   costs seconds in a debug build because the frozen reference's DFS and the
//!   enumerator's lex-drift `debug_assert` both scale with the walked paths.
//! - [`small_movement_cell`] — the CHEAP one: seven areas shaped so the label
//!   count is still a product (tens of labels, ~44 observed), sub-millisecond in
//!   debug. Reach for this whenever a test needs only a NON-DEGENERATE route
//!   family — an anti-vacuity precondition, a smoke over a bounded surface — and
//!   not the adversarial density. `action_source_worstcase.rs`'s per-edit route
//!   bound already uses it; a future per-edit consumer (e.g. a policy or arena
//!   test wanting a real routes family under the debug profile) should prefer it
//!   over paying the dense cell's seconds for a floor this cell clears.
//!
//! Label counts quoted anywhere in this module are OBSERVATIONS under the
//! harnesses that recorded them, NEVER ceilings (spec F5/L9: the route-label
//! space is unsearched).

// Each consumer uses a subset of this module; the unused remainder is not dead
// code, it is another binary's fixture (same reason `tests/common/mod.rs` does
// this).
#![allow(dead_code)]

use std::collections::BTreeMap;

use yokohama_core::{
    cell_to_area_id, derive_canal_crossings, row_lengths, staggered_adjacency, AchievementCard,
    AchievementType, AreaConfig, AreaId, AreaType, BoardLayout, BoardLayoutBuilder, BuildingSite,
    ChurchTrack, CustomsTrack, FivePowerEffect, GameConfig, GameState, LabState, MercenarySupply,
    PlayerColor, PlayerId, PlayerState, PortState, PresidentLocation, TurnPhase,
};

/// A minimal, type-valid `players`-seat **Movement** state with `n_areas`
/// placement-allowing (`CopperMine`) areas ids `0..n_areas` and EMPTY adjacency;
/// the cells below set their own board and per-fixture knobs afterwards.
///
/// Type-valid via the public constructors, NOT setup-valid — both
/// `get_legal_moves` and the factored source are total over it. This is the
/// construction `action_source_equivalence.rs`'s `state_with_areas(players,
/// TurnPhase::Movement, n_areas)` and `action_source_worstcase.rs`'s former
/// `route_base_state(players, n_areas)` each built for themselves; it is private
/// here because a base state is not a fixture — only the two cells below are.
fn base_movement_state(players: u8, n_areas: u8) -> GameState {
    let colors = [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green];
    let mut areas = BTreeMap::new();
    for i in 0..n_areas {
        areas.insert(
            AreaId::new(i),
            AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, i) },
        );
    }
    GameState {
        config: GameConfig::for_player_count(players),
        board_layout: BoardLayout {
            areas,
            adjacency: BTreeMap::new(),
            canal_crossings: vec![],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        },
        players: (0..players)
            .map(|i| PlayerState::new(PlayerId::new(i), colors[i as usize], i == 0))
            .collect(),
        current_player: PlayerId::new(0),
        turn_phase: TurnPhase::Movement,
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

/// Add the undirected edge `a`~`b`, tolled 1 Yen to the bank when `tolled`.
fn connect(state: &mut GameState, a: u8, b: u8, tolled: bool) {
    let (a, b) = (AreaId::new(a), AreaId::new(b));
    state.board_layout.adjacency.entry(a).or_default().push(b);
    state.board_layout.adjacency.entry(b).or_default().push(a);
    if tolled {
        state.board_layout.canal_crossings.push((a, b));
    }
}

// ---------------------------------------------------------------------------
// The dense 17-area cell
// ---------------------------------------------------------------------------

/// The Canal's grid cell for [`dense_board`]: `(row 1, col 3)` = `AreaId(9)` on
/// the staggered 4P pyramid, a fully-interior tile with all SIX brick
/// neighbours. That is the maximal-neighbour placement, so the any-to-any
/// derivation yields NINE tolled crossings and the Canal's neighbourhood is a
/// complete K6 — the densest graph a real board can present to the enumerator.
///
/// The same cell `yokohama-difftest`'s `routes3_worstcase.rs` uses; that harness
/// is where the 458/519 label observations these cells reproduce were first
/// recorded.
pub const DENSE_CANAL: (u8, u8) = (1, 3);

/// The real 4P STAGGERED pyramid (rows 6/5/4/3, 18 tiles) with the Canal at
/// [`DENSE_CANAL`], built from the engine's own topology primitives
/// (`staggered_adjacency` + `derive_canal_crossings`) so the fixture cannot
/// drift from the board the generator produces.
///
/// Area types are irrelevant to route enumeration except `allows_placement`
/// (which gates Tram destinations and the Canal); a rotating palette keeps the
/// board plausible without claiming setup validity.
pub fn dense_board() -> BoardLayout {
    let rows = row_lengths(4);
    let palette = [
        AreaType::CopperMine,
        AreaType::SilkMill,
        AreaType::TeaPlantation,
        AreaType::FishingGround,
        AreaType::Bank,
        AreaType::Chinatown,
    ];
    let mut b = BoardLayoutBuilder::new();
    let mut idx = 0u8;
    for (r, &len) in rows.iter().enumerate() {
        for col in 0..len {
            let id = AreaId::new(idx);
            let ty = if (r as u8, col) == DENSE_CANAL {
                AreaType::Canal
            } else {
                palette[(idx as usize) % palette.len()]
            };
            b.add_area(id, AreaConfig { area_type: ty, grid_position: (r as u8, col) });
            if ty != AreaType::Canal {
                b.set_building_site(id, BuildingSite::new([1, 1, 1, 1], 3));
                b.set_five_power_token(id, FivePowerEffect::Money(1));
            }
            idx += 1;
        }
    }
    for (a, z) in staggered_adjacency(&rows) {
        b.add_adjacency(a, z);
    }
    // Any-to-any Canal transit: adjacency (a walk may cross without stopping)
    // AND a crossing (the cross tolls 1 Yen to the bank).
    for (a, z) in derive_canal_crossings(DENSE_CANAL, &rows) {
        b.add_adjacency(a, z);
        b.add_canal_crossing(a, z);
    }
    b.build().expect("dense staggered board validates")
}

/// A dense Movement state on [`dense_board`]: the mover's President on `start`,
/// own assistants on EVERY non-Canal area (all 17, so the whole interior K6 is
/// live and the mover can walk the entire pyramid), the three opponents'
/// Presidents on `opponents`, and 1,000 Yen so no affordability cutoff bites —
/// money is the enumerator's cost driver (spec F6), and these cells exist to be
/// expensive.
///
/// No technologies: a caller wanting Postal (or any other route-affecting tech)
/// grants them on the returned state, which keeps `Rules` out of this module's
/// signature and leaves the tech-id lookup with the binary that owns the
/// `Rules` it is looking up in.
pub fn dense_17_area_cell_at(start: u8, opponents: [u8; 3]) -> GameState {
    let canal = cell_to_area_id(DENSE_CANAL.0, DENSE_CANAL.1, &row_lengths(4));

    // base_movement_state already builds the 4P roster; only the board is
    // replaced wholesale (its n_areas argument is irrelevant here).
    let mut s = base_movement_state(4, 1);
    s.board_layout = dense_board();
    s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(start));
    s.players[0].assistants.on_board.clear();
    for id in (0..18u8).filter(|&id| AreaId::new(id) != canal) {
        s.players[0].assistants.on_board.insert(AreaId::new(id), 1);
    }
    for (i, &opp) in opponents.iter().enumerate() {
        s.players[i + 1].president_location = PresidentLocation::OnBoard(AreaId::new(opp));
    }
    s.players[0].money = 1_000;
    s
}

/// The canonical dense cell — [`dense_17_area_cell_at`]`(0, [3, 8, 17])`, the
/// `plain` geometry every metered route gate anchors on. Its realized label
/// count has come back at 458 from three independent harnesses (the difftest
/// battery, this crate's equivalence gate, this crate's cost gate); that is an
/// OBSERVATION under those harnesses, never a ceiling.
pub fn dense_17_area_cell() -> GameState {
    dense_17_area_cell_at(0, [3, 8, 17])
}

// ---------------------------------------------------------------------------
// The small movement cell
// ---------------------------------------------------------------------------

/// The CHEAP route fixture: seven areas, deliberately shaped so the label count
/// is a PRODUCT rather than a destination count, without a board big enough to
/// make the enumerator's debug-only lex-drift `debug_assert` (which is
/// `O(prefixes × labels)`) expensive.
///
/// ```text
///        1                            0     = the mover's President
///      / | \                          1,2,3 = one opponent President each,
///   0 - 2 -+- 4 - 5                           mutually adjacent (a triangle)
///      \ | /       \ |                4,5,6 = the destinations, likewise a
///        3           6                        triangle
/// ```
/// Edges: 0~{1,2,3}, the 1-2-3 triangle, {1,2,3}~4, and the 4-5-6 triangle.
/// `0~1` and `4~5` are CANAL crossings (1 Yen to the bank each); every other
/// edge is free. The mover's own assistants sit on 1..=6 — a hop may only land
/// where the mover already has one.
///
/// Two structures do the work, and the fixture is worthless without both:
///  - the three opponents on 1/2/3 make those areas legal to WALK THROUGH (1 Yen
///    to that owner) but illegal to STOP on (`is_possible_action_area_with_tech`
///    blocks a destination holding an opponent's President, absent Postal), so
///    they are pure label MULTIPLIERS — each destination is reached under
///    several distinct owner masks, which the two canal edges then split again
///    by toll count;
///  - the two triangles make many distinct simple PATHS collapse onto ONE label,
///    which is exactly the gap a labels-not-paths cost claim is about.
///
/// 10 Yen affords every label (the maximum cost here is 3 owners + 2 tolls), so
/// affordability pruning never truncates the set. 44 labels OBSERVED under
/// `action_source_worstcase.rs`'s meter — an observation, never a ceiling.
pub fn small_movement_cell() -> GameState {
    let mut s = base_movement_state(4, 7);
    for (a, b, tolled) in [
        (0, 1, true),
        (0, 2, false),
        (0, 3, false),
        (1, 2, false),
        (1, 3, false),
        (2, 3, false),
        (1, 4, false),
        (2, 4, false),
        (3, 4, false),
        (4, 5, true),
        (4, 6, false),
        (5, 6, false),
    ] {
        connect(&mut s, a, b, tolled);
    }
    s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
    for a in 1..=6u8 {
        s.players[0].assistants.on_board.insert(AreaId::new(a), 1);
    }
    for (i, opp) in [1u8, 2, 3].into_iter().enumerate() {
        s.players[i + 1].president_location = PresidentLocation::OnBoard(AreaId::new(opp));
    }
    s.players[0].money = 10;
    s
}
