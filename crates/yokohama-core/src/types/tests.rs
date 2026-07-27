//! Unit tests for core types.

use crate::types::*;

#[test]
fn test_player_id_creation() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    assert_ne!(p0, p1);
    assert_eq!(p0.0, 0);
}

#[test]
fn test_player_id_try_new() {
    assert!(PlayerId::try_new(0).is_ok());
    assert!(PlayerId::try_new(3).is_ok());
    assert!(PlayerId::try_new(4).is_err());
    assert!(PlayerId::try_new(255).is_err());
}

#[test]
fn test_area_id_try_new() {
    assert!(AreaId::try_new(0).is_ok());
    assert!(AreaId::try_new(17).is_ok());
    assert!(AreaId::try_new(18).is_err());
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
fn test_area_type_allows_skip_only_at_permissive_areas() {
    // Task 16 optionality ruling: Skip is legal only at the six areas
    // with optional components; production areas and the Bank have no
    // optional component (their imperative "Gain ..." move is the only
    // legal action there).
    assert!(AreaType::PortA.allows_area_action_skip());
    assert!(AreaType::PortB.allows_area_action_skip());
    assert!(AreaType::LaboratoryA.allows_area_action_skip());
    assert!(AreaType::LaboratoryB.allows_area_action_skip());
    assert!(AreaType::EmploymentAgency.allows_area_action_skip());
    assert!(AreaType::Chinatown.allows_area_action_skip());
    assert!(AreaType::Church.allows_area_action_skip());
    assert!(AreaType::Customs.allows_area_action_skip());

    assert!(!AreaType::CopperMine.allows_area_action_skip());
    assert!(!AreaType::SilkMill.allows_area_action_skip());
    assert!(!AreaType::TeaPlantation.allows_area_action_skip());
    assert!(!AreaType::FishingGround.allows_area_action_skip());
    assert!(!AreaType::Bank.allows_area_action_skip());
    assert!(!AreaType::Canal.allows_area_action_skip());
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
fn test_trading_house_already_taken() {
    // T2-m2: single-source for the "in_hand + total_built()" formula
    // that was duplicated verbatim at 5 call sites.
    let mut th = TradingHouseState::new();
    assert_eq!(th.already_taken(), 0);

    th.in_hand = 2;
    assert_eq!(th.already_taken(), 2);

    th.built.push(AreaId::new(0));
    assert_eq!(th.already_taken(), 3);
}

#[test]
fn test_warehouse_item_flat_cost() {
    // T2-m3: single source for the 0 (assistant) / 2 (shophouse) flat
    // costs the CLI's warehouse_item_cost re-hardcoded separately.
    // TradingHouse has no flat cost -- it needs already_taken().
    assert_eq!(WarehouseItem::Assistant.flat_cost(), Some(0));
    assert_eq!(WarehouseItem::Shophouse.flat_cost(), Some(2));
    assert_eq!(WarehouseItem::TradingHouse.flat_cost(), None);
}

#[test]
fn test_church_track_rewards() {
    assert_eq!(ChurchTrack::vp_reward(4), 3);
    assert_eq!(ChurchTrack::vp_reward(11), 14);
    assert_eq!(ChurchTrack::faith_to_index(4), Some(0));
    assert_eq!(ChurchTrack::faith_to_index(11), Some(7));
    assert_eq!(ChurchTrack::faith_to_index(3), None);
}

/// Phase-2.9 Stage-6 banked-VP audit pin: the FULL Church VP table against
/// box-verified literals (owner box check 2026-07-18, guidingpath decision
/// log: "church board VP 3/4/5/6/7/8/11/14 CONFIRMED"; KB
/// 01-rules-reference.md Church table). The pre-existing test above pinned
/// only the endpoints — the six middle amounts had no literal pin anywhere
/// (tracks.rs tests compare against `vp_reward` itself, which cannot catch
/// a table edit).
#[test]
fn church_track_vp_full_table_pin() {
    let expected: [(u8, u8); 8] =
        [(4, 3), (5, 4), (6, 5), (7, 6), (8, 7), (9, 8), (10, 11), (11, 14)];
    for (faith, vp) in expected {
        assert_eq!(ChurchTrack::vp_reward(faith), vp, "faith {faith}");
    }
    // Off-track inputs credit nothing.
    assert_eq!(ChurchTrack::vp_reward(3), 0);
    assert_eq!(ChurchTrack::vp_reward(12), 0);
}

#[test]
fn test_customs_track_rewards() {
    // Board-art verified 2026-07-17: 7 spaces keyed by space number
    // (SUPERSEDES the old goods-keyed 8-slot model).
    assert_eq!(CustomsTrack::vp_reward(1), 5);
    assert_eq!(CustomsTrack::vp_reward(2), 4);
    assert_eq!(CustomsTrack::vp_reward(3), 10);
    assert_eq!(CustomsTrack::vp_reward(4), 9);
    assert_eq!(CustomsTrack::vp_reward(5), 16);
    assert_eq!(CustomsTrack::vp_reward(6), 15);
    assert_eq!(CustomsTrack::vp_reward(7), 22);
    assert_eq!(CustomsTrack::vp_reward(8), 0); // no such space
}

#[test]
fn test_customs_track_space_index_roundtrip() {
    assert_eq!(CustomsTrack::space_to_index(1), Some(0));
    assert_eq!(CustomsTrack::space_to_index(7), Some(6));
    assert_eq!(CustomsTrack::space_to_index(0), None);
    assert_eq!(CustomsTrack::space_to_index(8), None);
    assert_eq!(CustomsTrack::index_to_space(0), 1);
    assert_eq!(CustomsTrack::index_to_space(6), 7);
}

#[test]
fn test_customs_track_spaces_for_cost() {
    // Cost pairs: 1+2 cost 1, 3+4 cost 2, 5+6 cost 3, 7 alone costs 4.
    // Leftmost of each pair is listed first (higher VP / v1 default).
    assert_eq!(CustomsTrack::spaces_for_cost(1), Some(&[1, 2][..]));
    assert_eq!(CustomsTrack::spaces_for_cost(2), Some(&[3, 4][..]));
    assert_eq!(CustomsTrack::spaces_for_cost(3), Some(&[5, 6][..]));
    assert_eq!(CustomsTrack::spaces_for_cost(4), Some(&[7][..]));
    assert_eq!(CustomsTrack::spaces_for_cost(0), None);
    assert_eq!(CustomsTrack::spaces_for_cost(5), None);
}

#[test]
fn test_customs_track_space_cost() {
    // Inverse of spaces_for_cost: the printed cost for each of the 7
    // spaces, 1-7 in order. SINGLE SOURCE for render.rs's tracks_view,
    // which previously re-hardcoded this as its own array literal.
    let expected: [u8; 7] = [1, 1, 2, 2, 3, 3, 4];
    for (i, &cost) in expected.iter().enumerate() {
        let space = (i + 1) as u8;
        assert_eq!(CustomsTrack::space_cost(space), cost, "space {space}");
    }
}

#[test]
fn test_customs_track_rider() {
    // (warehouse_takes, yen) per space; spaces 5-7 carry no rider.
    assert_eq!(CustomsTrack::rider(1), (2, 0));
    assert_eq!(CustomsTrack::rider(2), (0, 2));
    assert_eq!(CustomsTrack::rider(3), (1, 0));
    assert_eq!(CustomsTrack::rider(4), (0, 1));
    assert_eq!(CustomsTrack::rider(5), (0, 0));
    assert_eq!(CustomsTrack::rider(6), (0, 0));
    assert_eq!(CustomsTrack::rider(7), (0, 0));
}

#[test]
fn test_achievement_vp() {
    assert_eq!(AchievementType::Copper5.first_place_vp(), 8);
    assert_eq!(AchievementType::Copper5.subsequent_vp(), 6);
    assert_eq!(AchievementType::Money10.first_place_vp(), 9);
    assert_eq!(AchievementType::Production4.first_place_vp(), 10);
}

/// Phase-2.9 Stage-6 banked-VP audit pin: first/subsequent VP for ALL 12
/// achievement types against KB 01-rules-reference.md's Achievement tables
/// (A: 8/6, B: 9/7, C: 10/8). The pre-existing test above pinned only 4 of
/// the 24 amounts; in particular no test pinned the subsequent VP for any
/// category-B or category-C card.
#[test]
fn achievement_vp_full_table_pin() {
    use AchievementType::*;
    let cases: [(AchievementType, u8, u8); 12] = [
        // Category A: Trade Good Holdings — 1st 8 VP, 2nd+ 6 VP
        (Copper5, 8, 6),
        (Silk6, 8, 6),
        (Tea7, 8, 6),
        (Seafood7, 8, 6),
        // Category B: Collection Goals — 1st 9 VP, 2nd+ 7 VP
        (Money10, 9, 7),
        (Tokens5, 9, 7),
        (Orders5, 9, 7),
        (Technologies4, 9, 7),
        // Category C: Building Presence — 1st 10 VP, 2nd+ 8 VP
        (Production4, 10, 8),
        (Commercial4, 10, 8),
        (Prod3Comm2, 10, 8),
        (Prod2Comm3, 10, 8),
    ];
    for (ach, first, subsequent) in cases {
        assert_eq!(ach.first_place_vp(), first, "{ach:?} first");
        assert_eq!(ach.subsequent_vp(), subsequent, "{ach:?} subsequent");
    }
}

#[test]
fn test_lab_position_modifier() {
    // Board art (asset-sweep-report.md Q4): 6 slots, surcharges +1,+1,+2,+2
    // on slots 3-6 (1-indexed) i.e. indices 2-5.
    assert_eq!(LabBoard::position_modifier(0), 0);
    assert_eq!(LabBoard::position_modifier(1), 0);
    assert_eq!(LabBoard::position_modifier(2), 1);
    assert_eq!(LabBoard::position_modifier(3), 1);
    assert_eq!(LabBoard::position_modifier(4), 2);
    assert_eq!(LabBoard::position_modifier(5), 2);
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

#[test]
fn test_achievement_card_vp() {
    let mut card = AchievementCard::new(AchievementType::Copper5);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // No one claimed yet
    assert_eq!(card.vp_for_player(p0), 0);

    // First player claims
    card.claimed_by.push(p0);
    assert_eq!(card.vp_for_player(p0), 8); // First place

    // Second player claims
    card.claimed_by.push(p1);
    assert_eq!(card.vp_for_player(p1), 6); // Subsequent
}

#[test]
fn test_mercenary_state() {
    let mut state = MercenaryState::new();
    assert_eq!(state.total(), 0);
    assert_eq!(state.unused_count(), 0);

    state.available.insert(Country::English, 2);
    state.used.insert(Country::French, 1);

    assert_eq!(state.total(), 3);
    assert_eq!(state.unused_count(), 2);
}

#[test]
fn test_trade_good_requirement() {
    let req = TradeGoodRequirement::new(2, 1, 3, 0);
    assert_eq!(req.total(), 6);
}

#[test]
fn test_game_config_removed_area_types() {
    let config_2p = GameConfig::for_player_count(2);
    let removed_2p = config_2p.removed_area_types();
    assert!(removed_2p.contains(&AreaType::LaboratoryB));
    assert!(removed_2p.contains(&AreaType::PortB));
    assert!(removed_2p.contains(&AreaType::Chinatown));
    assert!(removed_2p.contains(&AreaType::Canal));

    let config_3p = GameConfig::for_player_count(3);
    let removed_3p = config_3p.removed_area_types();
    assert!(removed_3p.contains(&AreaType::LaboratoryB));
    assert!(!removed_3p.contains(&AreaType::PortB)); // Port B is in 3p

    let config_4p = GameConfig::for_player_count(4);
    let removed_4p = config_4p.removed_area_types();
    assert!(removed_4p.is_empty()); // Nothing removed for 4p
}

#[test]
fn board_layout_builder_rejects_a_crossing_that_is_not_an_adjacency_edge() {
    // A canal crossing that is NOT also a (bidirectional) adjacency edge is a
    // topology `derive_canal_crossings` can never produce -- board_gen adds
    // every crossing to BOTH adjacency and canal_crossings. `build()` must
    // reject it, the same check `invariants.rs` runs on a finished board, so a
    // directly-constructed fixture cannot smuggle an untraversable crossing in
    // (types §3.4, third review).
    let mut builder = BoardLayoutBuilder::new();
    for id in [0u8, 1] {
        builder.add_area(AreaId::new(id), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, id) });
        builder.set_building_site(AreaId::new(id), BuildingSite::new([1, 1, 1, 1], 3));
        builder.set_five_power_token(AreaId::new(id), FivePowerEffect::Money(1));
    }
    // A crossing (0,1) with NO adjacency edge backing it.
    builder.add_canal_crossing(AreaId::new(0), AreaId::new(1));
    assert_eq!(
        builder.build(),
        Err(BoardLayoutError::CanalCrossingNotAdjacent(AreaId::new(0), AreaId::new(1))),
    );

    // The legitimate any-to-any case -- a crossing that IS an adjacency edge
    // (how board_gen layers it in) -- still builds.
    let mut ok = BoardLayoutBuilder::new();
    for id in [0u8, 1] {
        ok.add_area(AreaId::new(id), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, id) });
        ok.set_building_site(AreaId::new(id), BuildingSite::new([1, 1, 1, 1], 3));
        ok.set_five_power_token(AreaId::new(id), FivePowerEffect::Money(1));
    }
    ok.add_adjacency(AreaId::new(0), AreaId::new(1));
    ok.add_canal_crossing(AreaId::new(0), AreaId::new(1));
    assert!(ok.build().is_ok());
}

/// `PlayerId::DUMMY` is a reserved sentinel deliberately outside the boundary
/// constructors' valid range: it must never be validated via `try_new` (which
/// rejects it) nor passed to `state.player()` (which would index out of
/// bounds). This pins the exemption documented at its definition (types §3.4).
#[test]
fn player_dummy_is_a_reserved_sentinel_outside_try_new() {
    assert_eq!(PlayerId::DUMMY.0, u8::MAX);
    assert!(PlayerId::DUMMY.is_dummy());
    assert!(!PlayerId::new(0).is_dummy());
    // The boundary constructor rejects the sentinel value, by design.
    assert!(PlayerId::try_new(PlayerId::DUMMY.0).is_err());
}

/// The guard-rail half of the exemption: `state.player(DUMMY)` is a misuse
/// (the sentinel has no `PlayerState`), and the debug guard names it rather
/// than surfacing an opaque out-of-bounds index (types §3.4).
///
/// DEBUG-ONLY BY CONSTRUCTION, and the name always said so (Phase 3.3 §11(a)
/// ride-along). The guard it asserts is a `debug_assert!` at `state.rs:271`,
/// which release compiles out — so in release the very next line indexes
/// `players[255]` and panics with "index out of bounds: the len is 2 but the
/// index is 255", failing this test's `expected` substring. That made
/// `cargo test --release -p yokohama-core --lib` red whole-package for a reason
/// that is not a defect, which is worse than useless: a real release failure
/// would have been indistinguishable from the standing one. The `cfg` filters
/// the test out of the profile whose behavior it does not describe; the debug
/// tier is unchanged and still runs it. Same shape and same attribute order as
/// the crate's existing debug-only `should_panic` pins
/// (`policy.rs`'s `retreat_ppm_above_the_ppm_scale_is_loud_in_debug`,
/// `search_actions.rs`'s two) — this test was simply missed when they landed.
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "reserved sentinel")]
fn state_player_rejects_the_dummy_sentinel_in_debug() {
    let state = crate::game::create_test_game_state(2);
    let _ = state.player(PlayerId::DUMMY);
}

#[test]
fn test_board_layout_builder_rejects_invalid_areas_for_player_count() {
    let config_2p = GameConfig::for_player_count(2);
    let config_4p = GameConfig::for_player_count(4);

    // Create a builder with a LaboratoryB area (invalid for 2p)
    let mut builder = BoardLayoutBuilder::new();
    builder.add_area(
        AreaId::new(0),
        AreaConfig {
            area_type: AreaType::LaboratoryB,
            grid_position: (0, 0),
        },
    );
    builder.set_building_site(AreaId::new(0), BuildingSite::new([2, 3, 4, 5], 8));
    builder.set_five_power_token(AreaId::new(0), FivePowerEffect::Copper(2));

    // build_for_config() should fail for 2p (LaboratoryB is removed)
    let mut builder_copy = BoardLayoutBuilder::new();
    builder_copy.add_area(
        AreaId::new(0),
        AreaConfig {
            area_type: AreaType::LaboratoryB,
            grid_position: (0, 0),
        },
    );
    builder_copy.set_building_site(AreaId::new(0), BuildingSite::new([2, 3, 4, 5], 8));
    builder_copy.set_five_power_token(AreaId::new(0), FivePowerEffect::Copper(2));

    let result = builder_copy.build_for_config(&config_2p);
    assert!(matches!(
        result,
        Err(BoardLayoutError::InvalidAreaForPlayerCount(_, AreaType::LaboratoryB, 2))
    ));

    // build_for_config() should succeed for 4p (all areas allowed)
    let result = builder.build_for_config(&config_4p);
    assert!(result.is_ok());
}
