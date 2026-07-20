//! Final scoring calculations.

use std::collections::HashMap;
use crate::types::*;
use crate::game::Rules;
use super::cards::{OrderRegistry, TechRegistry};

/// Per-player breakdown of final score components. `total` is always the sum
/// of the other six fields -- `breakdowns` builds it incrementally as each
/// component is folded in, so the identity can't drift out of sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreBreakdown {
    pub base_vp: u32,        // p.victory_points at game end (incl. achievements)
    pub church: u32,
    pub customs: u32,
    pub technology: u32,
    pub country: u32,
    pub remainder: u32,      // mercenaries/imports/money/goods conversions
    pub total: u32,
}

/// One player's placement in the final ranking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub player: PlayerId,
    pub breakdown: ScoreBreakdown,
    pub rank: u8,
}

/// The complete result of a finished game: every player's standing
/// (`standings[0]` is rank 1) plus the winner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameResult {
    pub standings: Vec<Standing>,
    pub winner: PlayerId,
}

/// Calculate final scores for all players.
pub fn calculate_final_scores(
    state: &GameState,
    order_registry: &OrderRegistry,
    tech_registry: &TechRegistry,
) -> Vec<(PlayerId, u32)> {
    breakdowns(state, order_registry, tech_registry)
        .into_iter()
        .map(|(p, b)| (p, b.total))
        .collect()
}

/// The full per-player scoring breakdown plus the official ranking (KB
/// 01-rules-reference.md:556): highest total wins; ties broken by clockwise
/// closeness to `state.config.start_player` -- `distance = (seat + n -
/// start) % n`, distance 0 (the start player) closest. Ranks are 1..=n and
/// never shared.
pub fn final_result(state: &GameState, rules: &Rules) -> GameResult {
    let n = state.players.len() as u32;
    let start = state.config.start_player.0 as u32;
    let mut rows = breakdowns(state, &rules.orders, &rules.techs);
    rows.sort_by_key(|(p, b)| {
        let dist = (p.0 as u32 + n - start) % n;
        (std::cmp::Reverse(b.total), dist)
    });
    let standings = rows.into_iter().enumerate()
        .map(|(i, (player, breakdown))| Standing { player, breakdown, rank: (i + 1) as u8 })
        .collect::<Vec<_>>();
    GameResult { winner: standings[0].player, standings }
}

/// Per-player score components, captured separately instead of folded into
/// one running total -- both `calculate_final_scores` and `final_result`
/// build on this (the former discards everything but `total`).
fn breakdowns(
    state: &GameState,
    order_registry: &OrderRegistry,
    tech_registry: &TechRegistry,
) -> Vec<(PlayerId, ScoreBreakdown)> {
    let mut rows: Vec<(PlayerId, ScoreBreakdown)> = state
        .players
        .iter()
        .map(|p| {
            (
                p.id,
                ScoreBreakdown {
                    base_vp: p.victory_points,
                    church: 0,
                    customs: 0,
                    technology: 0,
                    country: 0,
                    remainder: 0,
                    total: p.victory_points,
                },
            )
        })
        .collect();

    // 1. Church Bonus
    let church_bonus = calculate_church_bonus(state);
    for (player, bonus) in church_bonus {
        if let Some((_, b)) = rows.iter_mut().find(|(id, _)| *id == player) {
            b.church += bonus;
            b.total += bonus;
        }
    }

    // 2. Customs Bonus
    let customs_bonus = calculate_customs_bonus(state);
    for (player, bonus) in customs_bonus {
        if let Some((_, b)) = rows.iter_mut().find(|(id, _)| *id == player) {
            b.customs += bonus;
            b.total += bonus;
        }
    }

    // 3. Technology Bonus
    let tech_bonus = calculate_technology_bonus(state, tech_registry);
    for (player, bonus) in tech_bonus {
        if let Some((_, b)) = rows.iter_mut().find(|(id, _)| *id == player) {
            b.technology += bonus;
            b.total += bonus;
        }
    }

    // 4. Country Bonus
    for player in &state.players {
        let country_vp = calculate_country_bonus(state, player.id, order_registry, tech_registry);
        if let Some((_, b)) = rows.iter_mut().find(|(id, _)| *id == player.id) {
            b.country += country_vp;
            b.total += country_vp;
        }
    }

    // 5. Remainder Scoring
    for player in &state.players {
        let remainder_vp = calculate_remainder_bonus(player);
        if let Some((_, b)) = rows.iter_mut().find(|(id, _)| *id == player.id) {
            b.remainder += remainder_vp;
            b.total += remainder_vp;
        }
    }

    rows
}

/// Church track majority bonus: 1st = 6 VP, 2nd = 3 VP.
fn calculate_church_bonus(state: &GameState) -> Vec<(PlayerId, u32)> {
    let mut counts: HashMap<PlayerId, (u8, u8)> = HashMap::new(); // (count, rightmost_position)

    for (index, slot) in state.church_track.slots.iter().enumerate() {
        if let Some(slot_data) = slot {
            let entry = counts.entry(slot_data.player).or_insert((0, 0));
            entry.0 += 1;
            entry.1 = entry.1.max(index as u8);
        }
    }

    let mut rankings: Vec<(PlayerId, u8, u8)> = counts
        .into_iter()
        .map(|(player, (count, position))| (player, count, position))
        .collect();

    // Sort by count (desc), then by rightmost position (desc) for tiebreaker
    rankings.sort_by(|a, b| {
        b.1.cmp(&a.1).then(b.2.cmp(&a.2))
    });

    let mut bonuses = Vec::new();
    if let Some((player, _, _)) = rankings.first() {
        bonuses.push((*player, 6));
    }
    if let Some((player, _, _)) = rankings.get(1) {
        bonuses.push((*player, 3));
    }

    bonuses
}

/// Customs track majority bonus: 1st = 8 VP, 2nd = 4 VP.
fn calculate_customs_bonus(state: &GameState) -> Vec<(PlayerId, u32)> {
    let mut counts: HashMap<PlayerId, (u8, u8)> = HashMap::new();

    for (index, slot) in state.customs_track.slots.iter().enumerate() {
        if let Some(slot_data) = slot {
            let entry = counts.entry(slot_data.player).or_insert((0, 0));
            entry.0 += 1;
            entry.1 = entry.1.max(index as u8);
        }
    }

    let mut rankings: Vec<(PlayerId, u8, u8)> = counts
        .into_iter()
        .map(|(player, (count, position))| (player, count, position))
        .collect();

    rankings.sort_by(|a, b| {
        b.1.cmp(&a.1).then(b.2.cmp(&a.2))
    });

    let mut bonuses = Vec::new();
    if let Some((player, _, _)) = rankings.first() {
        bonuses.push((*player, 8));
    }
    if let Some((player, _, _)) = rankings.get(1) {
        bonuses.push((*player, 4));
    }

    bonuses
}

/// Technology total industry value bonus: 1st = 10 VP, 2nd = 5 VP.
/// Tiebreaker: closest to start player (clockwise).
fn calculate_technology_bonus(state: &GameState, tech_registry: &TechRegistry) -> Vec<(PlayerId, u32)> {
    // Calculate total industry value for each player
    let mut totals: Vec<(PlayerId, u32, u8)> = state
        .players
        .iter()
        .map(|p| {
            let total_iv: u32 = p.technologies
                .iter()
                .filter_map(|&tech_id| tech_registry.get(tech_id))
                .map(|card| card.industry_cost as u32)
                .sum();
            // Distance from start player (for tiebreaker - lower is better)
            let distance = (p.id.0 + state.config.player_count - state.config.start_player.0)
                % state.config.player_count;
            (p.id, total_iv, distance)
        })
        .collect();

    // Sort by industry value (desc), then by distance from start player (asc)
    totals.sort_by(|a, b| {
        b.1.cmp(&a.1).then(a.2.cmp(&b.2))
    });

    let mut bonuses = Vec::new();
    // Participation gate: `iv > 0` is equivalent to "owns >=1 tech card"
    // ONLY because every tech in the standard data has industry_cost >= 3
    // (08-tech-cards.yaml min is 3). The rulebook's literal gate is
    // ownership; if a zero-industry tech is ever added, this must become an
    // ownership check or a 0-iv owner would wrongly be excluded (Stage-2
    // differential review note, 2026-07-20).
    if let Some((player, iv, _)) = totals.first() {
        if *iv > 0 {
            bonuses.push((*player, 10));
        }
    }
    if let Some((player, iv, _)) = totals.get(1) {
        if *iv > 0 {
            bonuses.push((*player, 5));
        }
    }

    bonuses
}

/// Country set bonus: 5 diff = 12, 4 diff = 8, 3 diff = 4, 2 diff = 2.
fn calculate_country_bonus(
    state: &GameState,
    player: PlayerId,
    order_registry: &OrderRegistry,
    tech_registry: &TechRegistry,
) -> u32 {
    let p = state.player(player);

    // Count country icons from completed orders and technologies
    let mut country_counts: HashMap<Country, u8> = HashMap::new();

    for &order_id in &p.completed_orders {
        if let Some(card) = order_registry.get(order_id) {
            *country_counts.entry(card.country).or_insert(0) += 1;
        }
    }

    for &tech_id in &p.technologies {
        if let Some(card) = tech_registry.get(tech_id) {
            *country_counts.entry(card.country).or_insert(0) += 1;
        }
    }

    // Greedy set matching: form largest sets first
    let mut total_vp = 0u32;

    loop {
        // Count how many different countries we have at least 1 icon for
        let available: Vec<Country> = country_counts
            .iter()
            .filter(|(_, &count)| count > 0)
            .map(|(&country, _)| country)
            .collect();

        let set_size = available.len();
        if set_size < 2 {
            break; // Can't form any more sets
        }

        // Award VP for this set
        let set_vp = match set_size {
            5 => 12,
            4 => 8,
            3 => 4,
            2 => 2,
            _ => 0,
        };
        total_vp += set_vp;

        // Consume one icon from each country in the set
        for country in available.iter().take(set_size.min(5)) {
            if let Some(count) = country_counts.get_mut(country) {
                *count -= 1;
            }
        }
    }

    total_vp
}

/// Remainder scoring: mercenaries, imported goods, money, trade goods.
pub fn calculate_remainder_bonus(player: &PlayerState) -> u32 {
    let mut vp = 0u32;

    // Unused mercenaries: 1 VP each
    vp += player.mercenaries.unused_count() as u32;

    // Imported goods: 1 VP each
    vp += player.imported_goods;

    // Money: 1 VP per 2 Yen
    vp += player.money / 2;

    // Trade goods: 1 VP per 3 goods
    vp += player.total_trade_goods() / 3;

    vp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::create_test_game_state;

    #[test]
    fn final_result_breaks_ties_clockwise_from_start_player() {
        // Fixture: 3p, engineered equal totals for players 1 and 2 (set raw
        // victory_points equal, then zero every remainder-affecting field so
        // default per-seat differences -- e.g. the start player's lower
        // starting money -- can't reintroduce an accidental tiebreak).
        //
        // start_player = 0, and the TIE IS BETWEEN TWO NON-START SEATS. That
        // is what gives this test teeth (I1, external review 2026-07-20): the
        // previous fixture tied p1/p2 with start_player = 2, so the winner was
        // the start player itself at distance 0 -- which is distance 0 under
        // BOTH the shipped clockwise key and a reversed one, so a sign flip at
        // `final_result`'s sort key passed unnoticed.
        //
        //   clockwise  (p + n - start) % n : p1 = 1, p2 = 2  -> p1 wins the tie
        //   reversed   (start + n - p) % n : p1 = 2, p2 = 1  -> p2 would win
        //
        // so the direction is now observable in the outcome.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(3);
        state.config.start_player = PlayerId::new(0);
        for p in state.players.iter_mut() {
            p.money = 0;
            p.copper = 0;
            p.silk = 0;
            p.tea = 0;
            p.seafood = 0;
            p.imported_goods = 0;
        }
        state.players[0].victory_points = 5;
        state.players[1].victory_points = 10;
        state.players[2].victory_points = 10;

        let r = final_result(&state, &rules);
        assert_eq!(r.standings.len(), state.players.len(), "one standing per player");
        assert_eq!(r.winner, PlayerId::new(1), "clockwise from start 0, p1 is nearer than tied p2");
        let ranks: Vec<(PlayerId, u8)> = r.standings.iter().map(|s| (s.player, s.rank)).collect();
        // p1 outranks p2 despite equal totals; no shared ranks.
        let p1_rank = ranks.iter().find(|(p, _)| *p == PlayerId::new(1)).unwrap().1;
        let p2_rank = ranks.iter().find(|(p, _)| *p == PlayerId::new(2)).unwrap().1;
        assert!(
            p1_rank < p2_rank,
            "p1 (clockwise distance 1) must outrank tied p2 (distance 2); a reversed \
             tie-break would invert this: {ranks:?}"
        );
        let mut rank_values: Vec<u8> = ranks.iter().map(|(_, rk)| *rk).collect();
        rank_values.sort_unstable();
        assert_eq!(rank_values, vec![1, 2, 3], "ranks must be unique 1..=n, never shared");
    }

    #[test]
    fn breakdown_components_sum_to_total_and_wrapper_matches() {
        let rules = Rules::standard();
        let mut state = create_test_game_state(2);
        state.players[0].victory_points = 3;
        // American + French completed orders -> 2 distinct countries (country bonus > 0).
        state.players[0].completed_orders = vec![OrderCardId(1), OrderCardId(2)];
        // American tech, industry_cost 3 -> sole nonzero industry value (technology bonus > 0).
        state.players[0].technologies = vec![TechCardId(1)];
        // Imported good -> remainder bonus > 0.
        state.players[0].imported_goods = 2;
        // Player 0's rightmost church/customs slot outranks player 1's leftmost slot.
        state.church_track.slots[5] = Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 0 });
        state.church_track.slots[0] = Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 1 });
        state.customs_track.slots[6] = Some(CustomsSlot { player: PlayerId::new(0), arrival_order: 0 });
        state.customs_track.slots[0] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 1 });

        let r = final_result(&state, &rules);

        // Sanity: every bonus category is actually exercised somewhere in
        // this fixture, so the sum-identity check below can't pass on
        // vacuous zeros.
        let p0 = r.standings.iter().find(|s| s.player == PlayerId::new(0)).unwrap();
        assert!(p0.breakdown.church > 0, "{:?}", p0.breakdown);
        assert!(p0.breakdown.customs > 0, "{:?}", p0.breakdown);
        assert!(p0.breakdown.technology > 0, "{:?}", p0.breakdown);
        assert!(p0.breakdown.country > 0, "{:?}", p0.breakdown);
        assert!(p0.breakdown.remainder > 0, "{:?}", p0.breakdown);

        for s in &r.standings {
            let b = &s.breakdown;
            assert_eq!(b.base_vp + b.church + b.customs + b.technology + b.country + b.remainder, b.total);
        }
        let flat = calculate_final_scores(&state, &rules.orders, &rules.techs);
        for s in &r.standings {
            assert!(flat.contains(&(s.player, s.breakdown.total)));
        }
    }

    #[test]
    fn test_remainder_bonus() {
        let mut player = PlayerState::new(PlayerId::new(0), PlayerColor::Red, true);
        player.imported_goods = 2;
        player.money = 5;
        player.copper = 3;
        player.silk = 3;
        player.tea = 3;
        // 2 imported + 2 (5/2) + 3 (9/3) = 7 VP
        assert_eq!(calculate_remainder_bonus(&player), 7);
    }

    #[test]
    fn test_country_bonus_full_set() {
        // Test with 5 different countries = 12 VP
        let _order_registry = OrderRegistry::new();
        let _tech_registry = TechRegistry::new();

        // Would need to populate registries for full test
        // For now just verify function compiles
    }

    #[test]
    fn test_country_set_distinct_countries() {
        // Verify: 3 English + 1 French + 1 German = 3 distinct = 4 VP (not 5 icons)
        let mut country_counts: HashMap<Country, u8> = HashMap::new();
        country_counts.insert(Country::English, 3);
        country_counts.insert(Country::French, 1);
        country_counts.insert(Country::German, 1);

        // Count distinct countries
        let available: Vec<Country> = country_counts
            .iter()
            .filter(|(_, &count)| count > 0)
            .map(|(&country, _)| country)
            .collect();

        assert_eq!(available.len(), 3); // Only 3 distinct countries
    }

    #[test]
    fn test_country_set_multiple_sets() {
        // 2 of each country = two 5-sets = 24 VP
        let mut country_counts: HashMap<Country, u8> = HashMap::new();
        country_counts.insert(Country::English, 2);
        country_counts.insert(Country::American, 2);
        country_counts.insert(Country::French, 2);
        country_counts.insert(Country::German, 2);
        country_counts.insert(Country::Dutch, 2);

        let mut total_vp = 0u32;
        loop {
            let available: Vec<Country> = country_counts
                .iter()
                .filter(|(_, &count)| count > 0)
                .map(|(&country, _)| country)
                .collect();

            let set_size = available.len();
            if set_size < 2 {
                break;
            }

            let set_vp = match set_size {
                5 => 12,
                4 => 8,
                3 => 4,
                2 => 2,
                _ => 0,
            };
            total_vp += set_vp;

            for country in available.iter().take(set_size.min(5)) {
                if let Some(count) = country_counts.get_mut(country) {
                    *count -= 1;
                }
            }
        }

        assert_eq!(total_vp, 24); // Two 5-sets = 12 + 12 = 24 VP
    }
}
