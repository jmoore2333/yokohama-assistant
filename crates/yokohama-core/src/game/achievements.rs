//! Achievement card validation and fulfillment.

use crate::types::*;

/// Check if player can claim an Achievement.
pub fn can_claim_achievement(
    state: &GameState,
    player: PlayerId,
    achievement: AchievementType,
) -> bool {
    let p = state.player(player);

    // Check if already claimed by this player
    for card in &state.achievements {
        if card.achievement_type == achievement && card.claimed_by.contains(&player) {
            return false;
        }
    }

    // Check if player has assistant in hand (required to place)
    if p.assistants.in_hand < 1 {
        return false;
    }

    // Requirement met iff current progress has reached what's needed --
    // `achievement_progress` carries the exact arithmetic every arm below
    // used to have inline (Task 14, T2.3: extracted so a CLI progress
    // display and the actual claim gate can never disagree -- there's only
    // one copy of the arithmetic left).
    let (current, needed) = achievement_progress(state, player, achievement);
    current >= needed
}

/// Progress toward `achievement` for `player`: `(current, needed)`. This is
/// the SAME arithmetic `can_claim_achievement`'s match used to have inline,
/// per arm -- extracted so a CLI progress display (`render::
/// achievements_view`) and the actual claim gate can never disagree (Task
/// 14, T2.3). Does NOT account for `can_claim_achievement`'s other two
/// gates (already claimed by this player, no assistant in hand to place) --
/// those aren't "progress" in any numeric sense, so `can_claim_achievement`
/// still checks them itself before ever consulting this function.
///
/// The two Category-C AND-shaped achievements (`Prod3Comm2`/`Prod2Comm3`)
/// can't be expressed as a single linear "own N of a thing" pair the way
/// every other achievement can: they require a production-area count AND a
/// commercial-area count to EACH individually clear their own threshold.
/// They're encoded as a capped sum: `needed` is the sum of both thresholds,
/// and `current` is the sum of each count capped at its own threshold
/// (`count.min(threshold)`). Because each capped term can never exceed its
/// own threshold, their sum can only reach the combined `needed` when BOTH
/// terms are individually at their cap -- i.e. `current >= needed` holds
/// exactly when both sub-conditions hold, reproducing the AND precisely
/// (not an approximation; see `achievement_progress_and_shaped_achievement_
/// caps_each_leg_at_its_own_threshold` for a worked counterexample against
/// a naive uncapped sum).
pub fn achievement_progress(
    state: &GameState,
    player: PlayerId,
    achievement: AchievementType,
) -> (u32, u32) {
    let p = state.player(player);

    match achievement {
        // Category A: Trade good holdings (no payment, just demonstrate ownership)
        AchievementType::Copper5 => (p.copper, 5),
        AchievementType::Silk6 => (p.silk, 6),
        AchievementType::Tea7 => (p.tea, 7),
        AchievementType::Seafood7 => (p.seafood, 7),

        // Category B: Collection goals
        AchievementType::Money10 => (p.money, 10),
        AchievementType::Tokens5 => {
            (p.five_power_tokens as u32 + p.mercenaries.total() as u32, 5)
        }
        AchievementType::Orders5 => (p.completed_orders.len() as u32, 5),
        AchievementType::Technologies4 => (p.technologies.len() as u32, 4),

        // Category C: Building presence
        AchievementType::Production4 => (count_production_buildings(state, player) as u32, 4),
        AchievementType::Commercial4 => (count_commercial_buildings(state, player) as u32, 4),
        AchievementType::Prod3Comm2 => {
            let prod = (count_production_buildings(state, player) as u32).min(3);
            let comm = (count_commercial_buildings(state, player) as u32).min(2);
            (prod + comm, 5)
        }
        AchievementType::Prod2Comm3 => {
            let prod = (count_production_buildings(state, player) as u32).min(2);
            let comm = (count_commercial_buildings(state, player) as u32).min(3);
            (prod + comm, 5)
        }
    }
}

/// Count unique production areas where player has a building.
fn count_production_buildings(state: &GameState, player: PlayerId) -> usize {
    let p = state.player(player);
    let mut count = 0;

    for &area_id in p.shophouses.built.keys() {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_production() {
                count += 1;
            }
        }
    }

    for &area_id in &p.trading_houses.built {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_production()
                && !p.shophouses.built.contains_key(&area_id)
            {
                // Don't double-count if also have shophouse
                count += 1;
            }
        }
    }

    count
}

/// Count unique commercial areas where player has a building.
fn count_commercial_buildings(state: &GameState, player: PlayerId) -> usize {
    let p = state.player(player);
    let mut count = 0;

    for &area_id in p.shophouses.built.keys() {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_commercial() {
                count += 1;
            }
        }
    }

    for &area_id in &p.trading_houses.built {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_commercial()
                && !p.shophouses.built.contains_key(&area_id)
            {
                count += 1;
            }
        }
    }

    count
}

/// Claim an Achievement.
pub fn claim_achievement(
    state: &mut GameState,
    player: PlayerId,
    achievement: AchievementType,
) -> Result<u32, &'static str> {
    if !can_claim_achievement(state, player, achievement) {
        return Err("Cannot claim achievement: requirements not met");
    }

    // Find the achievement card and add player to claimed list
    let mut vp_awarded = 0u32;
    for card in &mut state.achievements {
        if card.achievement_type == achievement {
            let is_first = card.claimed_by.is_empty();
            card.claimed_by.push(player);
            vp_awarded = if is_first {
                achievement.first_place_vp() as u32
            } else {
                achievement.subsequent_vp() as u32
            };
            break;
        }
    }

    // Lock an assistant on the achievement
    let p = state.player_mut(player);
    p.assistants.in_hand -= 1;
    p.assistants.on_achievements += 1;
    p.victory_points += vp_awarded;

    Ok(vp_awarded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_achievement_first_vs_subsequent() {
        assert_eq!(AchievementType::Copper5.first_place_vp(), 8);
        assert_eq!(AchievementType::Copper5.subsequent_vp(), 6);
    }

    /// RULED (Jeremy, 2026-07-20 decision-log row, closing ruling A2):
    /// face-DOWN (used) mercenaries count toward the Tokens5 achievement —
    /// "own a total of 5" means all owned tokens, face-up or face-down.
    /// Pin: 3 five-power tokens + 2 USED mercenaries (zero face-up) meets
    /// the requirement; with only 1 used mercenary it does not.
    #[test]
    fn tokens5_counts_face_down_mercenaries_ruled_a2() {
        let mut state = create_test_state();
        let p = state.player_mut(PlayerId::new(0));
        p.five_power_tokens = 3;
        p.mercenaries.used.insert(Country::English, 2);
        let (current, needed) =
            achievement_progress(&state, PlayerId::new(0), AchievementType::Tokens5);
        assert!(
            current >= needed,
            "3 five-power + 2 face-down mercenaries = 5 owned tokens: {current}/{needed}"
        );
        let p = state.player_mut(PlayerId::new(0));
        p.mercenaries.used.insert(Country::English, 1);
        let (current, needed) =
            achievement_progress(&state, PlayerId::new(0), AchievementType::Tokens5);
        assert!(current < needed, "4 owned tokens must not qualify: {current}/{needed}");
    }

    fn create_test_state() -> GameState {
        let config = GameConfig::new(
            2,
            vec![PlayerColor::Red, PlayerColor::Blue],
            PlayerId::new(0),
        );

        let board_layout = BoardLayout {
            areas: BTreeMap::new(),
            adjacency: BTreeMap::new(),
            canal_crossings: vec![],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };

        let players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];

        GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::AdditionalActionFirst,
            turn_number: 1,
            additional_action_state: AdditionalActionState::new(),
            current_action_power: None,
            is_mercenary_action: false,
            mercenary_target_area: None,
            mercenary_return_phase: None,
            president_start_area: None,
            port_state: PortState::new(false),
            lab_state: LabState::new(false),
            order_deck: vec![],
            tech_deck: vec![],
            orders_removed: vec![],
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

    /// Jeremy-adjudicated 2026-07-18: Category-C achievements ("Buildings in
    /// N distinct areas") count Shophouses AND Trading Houses the same way -
    /// pins that `count_production_buildings`/`count_commercial_buildings`
    /// already tally both (they did before this pin was added; this guards
    /// against a future regression, e.g. someone "simplifying" away the
    /// trading_houses.built loop).
    #[test]
    fn test_count_production_buildings_counts_trading_houses_too() {
        let mut state = create_test_state();
        let mine_area = AreaId::new(0);
        let mill_area = AreaId::new(1);
        state.board_layout.areas.insert(mine_area, AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 0),
        });
        state.board_layout.areas.insert(mill_area, AreaConfig {
            area_type: AreaType::SilkMill,
            grid_position: (0, 1),
        });
        // A Shophouse in one production area, a Trading House (no
        // Shophouse) in another distinct production area: both must count.
        state.players[0].shophouses.built.insert(mine_area, 0);
        state.players[0].trading_houses.built.push(mill_area);

        assert_eq!(count_production_buildings(&state, PlayerId::new(0)), 2);
    }

    #[test]
    fn test_count_commercial_buildings_counts_trading_houses_too() {
        let mut state = create_test_state();
        let bank_area = AreaId::new(0);
        let port_area = AreaId::new(1);
        state.board_layout.areas.insert(bank_area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.board_layout.areas.insert(port_area, AreaConfig {
            area_type: AreaType::PortA,
            grid_position: (0, 1),
        });
        state.players[0].shophouses.built.insert(bank_area, 0);
        state.players[0].trading_houses.built.push(port_area);

        assert_eq!(count_commercial_buildings(&state, PlayerId::new(0)), 2);
    }

    #[test]
    fn test_count_production_buildings_does_not_double_count_area_with_both() {
        // A single area holding BOTH a Shophouse and a Trading House is one
        // distinct area, not two - the achievement counts distinct AREAS,
        // not distinct buildings.
        let mut state = create_test_state();
        let mine_area = AreaId::new(0);
        state.board_layout.areas.insert(mine_area, AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 0),
        });
        state.players[0].shophouses.built.insert(mine_area, 0);
        state.players[0].trading_houses.built.push(mine_area);

        assert_eq!(count_production_buildings(&state, PlayerId::new(0)), 1);
    }

    #[test]
    fn test_production4_achievement_claimable_via_trading_houses_alone() {
        // End-to-end through can_claim_achievement: 4 Trading Houses (no
        // Shophouses at all) in 4 distinct production areas must satisfy
        // Production4, exactly as 4 Shophouses would.
        let mut state = create_test_state();
        state.players[0].assistants.in_hand = 1;
        for i in 0..4u8 {
            let area = AreaId::new(i);
            state.board_layout.areas.insert(area, AreaConfig {
                area_type: AreaType::CopperMine,
                grid_position: (0, i),
            });
            state.players[0].trading_houses.built.push(area);
        }

        assert!(can_claim_achievement(&state, PlayerId::new(0), AchievementType::Production4));
    }

    /// Fixture for `progress_matches_the_claim_gate` (Task 14, T2.3): a
    /// 2-player state with deliberately varied resource/building counts --
    /// some exactly AT a threshold, some above, some below -- and lopsided
    /// production/commercial building counts (Red production-heavy and
    /// commercial-empty, Blue the reverse) specifically to exercise the
    /// AND-shaped Prod3Comm2/Prod2Comm3 arms, not just the linear ones.
    /// Both players keep their full starting `assistants.in_hand` (8) and
    /// neither has claimed anything, so `can_claim_achievement`'s two
    /// pre-arithmetic gates (already claimed, no assistant in hand) never
    /// fire here -- the property under test reduces to exactly the
    /// arithmetic both functions must agree on.
    fn progress_fixture_state() -> GameState {
        let mut state = create_test_state();

        let prod_areas = [AreaId::new(0), AreaId::new(1), AreaId::new(2)];
        let comm_areas = [AreaId::new(3), AreaId::new(4)];
        let prod_types = [AreaType::CopperMine, AreaType::SilkMill, AreaType::TeaPlantation];
        let comm_types = [AreaType::Bank, AreaType::PortA];
        for (i, &a) in prod_areas.iter().enumerate() {
            state.board_layout.areas.insert(a, AreaConfig { area_type: prod_types[i], grid_position: (0, i as u8) });
        }
        for (i, &a) in comm_areas.iter().enumerate() {
            state.board_layout.areas.insert(a, AreaConfig { area_type: comm_types[i], grid_position: (1, i as u8) });
        }

        // Red: exactly at threshold on every linear check; production-heavy
        // (3, meets Prod3Comm2's production leg), commercial-empty (fails
        // its commercial leg).
        state.players[0].copper = 5;
        state.players[0].silk = 2;
        state.players[0].tea = 9;
        state.players[0].seafood = 7;
        state.players[0].money = 10;
        state.players[0].five_power_tokens = 2;
        state.players[0].mercenaries.available.insert(Country::English, 3);
        state.players[0].completed_orders = (0..4).map(OrderCardId).collect();
        state.players[0].technologies = (0..4).map(TechCardId).collect();
        state.players[0].trading_houses.built = prod_areas.to_vec();

        // Blue: the opposite skew -- commercial-only buildings (meets
        // Prod3Comm2's commercial leg at exactly 2, fails its production
        // leg at 0; also fails Prod2Comm3's commercial leg, which needs 3).
        state.players[1].copper = 0;
        state.players[1].silk = 6;
        state.players[1].tea = 1;
        state.players[1].seafood = 0;
        state.players[1].money = 3;
        state.players[1].five_power_tokens = 0;
        state.players[1].completed_orders = (10..16).map(OrderCardId).collect();
        state.players[1].technologies = Vec::new();
        state.players[1].trading_houses.built = comm_areas.to_vec();

        state
    }

    /// Task 14 (T2.3), brief Step 1: `achievement_progress` must carry the
    /// SAME arithmetic `can_claim_achievement` uses, for every achievement
    /// type (not just the 3 physically in play this game -- the "already
    /// claimed" gate is a no-op for a type that isn't one of the 3 cards,
    /// so the arithmetic-equivalence property holds regardless) and every
    /// player in the fixture.
    #[test]
    fn progress_matches_the_claim_gate() {
        let state = progress_fixture_state();
        let all_achievements = [
            AchievementType::Copper5,
            AchievementType::Silk6,
            AchievementType::Tea7,
            AchievementType::Seafood7,
            AchievementType::Money10,
            AchievementType::Tokens5,
            AchievementType::Orders5,
            AchievementType::Technologies4,
            AchievementType::Production4,
            AchievementType::Commercial4,
            AchievementType::Prod3Comm2,
            AchievementType::Prod2Comm3,
        ];
        for &player in &[PlayerId::new(0), PlayerId::new(1)] {
            for &ach in &all_achievements {
                let (cur, need) = achievement_progress(&state, player, ach);
                assert_eq!(
                    cur >= need,
                    can_claim_achievement(&state, player, ach),
                    "player {:?} achievement {:?}: cur={} need={}",
                    player, ach, cur, need,
                );
            }
        }
    }

    /// Pins the exact capped-sum encoding for the AND-shaped achievements
    /// (doc comment on `achievement_progress`) against concrete numbers,
    /// not just the boolean equivalence `progress_matches_the_claim_gate`
    /// checks -- guards against a regression that keeps the boolean
    /// equivalence by accident while silently breaking the numbers a
    /// player actually sees in `render::achievements_view`. Deliberately
    /// uses a RAW total (6) that exceeds `need` (5) while the achievement
    /// still must NOT be claimable, because one leg (commercial, 1) is
    /// individually below ITS OWN threshold (2) -- a naive uncapped
    /// `production + commercial >= need` sum would wrongly call this
    /// claimable.
    #[test]
    fn achievement_progress_and_shaped_achievement_caps_each_leg_at_its_own_threshold() {
        let mut state = create_test_state();
        let prod_areas: Vec<AreaId> = (0..5).map(AreaId::new).collect();
        let comm_area = AreaId::new(5);
        for (i, &a) in prod_areas.iter().enumerate() {
            state.board_layout.areas.insert(a, AreaConfig {
                area_type: AreaType::CopperMine,
                grid_position: (0, i as u8),
            });
        }
        state.board_layout.areas.insert(comm_area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (1, 0),
        });
        state.players[0].trading_houses.built = prod_areas.clone();
        state.players[0].trading_houses.built.push(comm_area);

        let (cur, need) = achievement_progress(&state, PlayerId::new(0), AchievementType::Prod3Comm2);
        assert_eq!(need, 5, "need = 3 (production cap) + 2 (commercial cap)");
        assert_eq!(cur, 4, "current = min(5,3) production + min(1,2) commercial = 3 + 1, NOT the raw 6");
        assert!(cur < need);
        assert!(
            !can_claim_achievement(&state, PlayerId::new(0), AchievementType::Prod3Comm2),
            "commercial count (1) is still below its own threshold (2), even though the RAW total (6) exceeds `need` (5)"
        );
    }
}
