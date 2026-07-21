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

    /// Item-5 rewrite (was `progress_matches_the_claim_gate`, a tautology:
    /// it asserted `achievement_progress(..).0 >= .1 == can_claim_achievement`,
    /// but `can_claim_achievement` computes its verdict from that SAME call,
    /// so the identity held under ANY threshold mutation). Pins each
    /// achievement's `needed` value to its LITERAL rulebook threshold, so a
    /// mutation of any threshold constant is caught per-edit.
    #[test]
    fn achievement_thresholds_are_the_literal_rulebook_values() {
        let state = create_test_state();
        let p = PlayerId::new(0);
        let need = |ach| achievement_progress(&state, p, ach).1;
        assert_eq!(need(AchievementType::Copper5), 5);
        assert_eq!(need(AchievementType::Silk6), 6);
        assert_eq!(need(AchievementType::Tea7), 7);
        assert_eq!(need(AchievementType::Seafood7), 7);
        assert_eq!(need(AchievementType::Money10), 10);
        assert_eq!(need(AchievementType::Tokens5), 5);
        assert_eq!(need(AchievementType::Orders5), 5);
        assert_eq!(need(AchievementType::Technologies4), 4);
        assert_eq!(need(AchievementType::Production4), 4);
        assert_eq!(need(AchievementType::Commercial4), 4);
        assert_eq!(need(AchievementType::Prod3Comm2), 5); // production cap 3 + commercial cap 2
        assert_eq!(need(AchievementType::Prod2Comm3), 5); // production cap 2 + commercial cap 3
    }

    /// Two-sided boundary for the two Category-B thresholds the review found
    /// unpinned per-edit (`achievements.rs:71` Money10, `:76` Technologies4 --
    /// only the release differential caught mutating them). Just-below must
    /// NOT be claimable; exactly-at must be.
    #[test]
    fn money10_and_technologies4_thresholds_are_two_sided() {
        let mut state = create_test_state();
        let p = PlayerId::new(0);
        state.players[0].assistants.in_hand = 1;

        state.players[0].money = 9;
        assert!(!can_claim_achievement(&state, p, AchievementType::Money10), "9 Yen must not claim Money10");
        state.players[0].money = 10;
        assert!(can_claim_achievement(&state, p, AchievementType::Money10), "10 Yen must claim Money10");

        state.players[0].technologies = (0..3).map(TechCardId).collect();
        assert!(
            !can_claim_achievement(&state, p, AchievementType::Technologies4),
            "3 techs must not claim Technologies4"
        );
        state.players[0].technologies = (0..4).map(TechCardId).collect();
        assert!(
            can_claim_achievement(&state, p, AchievementType::Technologies4),
            "4 techs must claim Technologies4"
        );
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
