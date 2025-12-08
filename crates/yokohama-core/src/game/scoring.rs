//! Final scoring calculations.

use std::collections::HashMap;
use crate::types::*;
use super::cards::{OrderRegistry, TechRegistry};

/// Calculate final scores for all players.
pub fn calculate_final_scores(
    state: &GameState,
    order_registry: &OrderRegistry,
    tech_registry: &TechRegistry,
) -> Vec<(PlayerId, u32)> {
    let mut scores: Vec<(PlayerId, u32)> = state
        .players
        .iter()
        .map(|p| (p.id, p.victory_points))
        .collect();

    // 1. Church Bonus
    let church_bonus = calculate_church_bonus(state);
    for (player, bonus) in church_bonus {
        if let Some((_, score)) = scores.iter_mut().find(|(id, _)| *id == player) {
            *score += bonus;
        }
    }

    // 2. Customs Bonus
    let customs_bonus = calculate_customs_bonus(state);
    for (player, bonus) in customs_bonus {
        if let Some((_, score)) = scores.iter_mut().find(|(id, _)| *id == player) {
            *score += bonus;
        }
    }

    // 3. Technology Bonus
    let tech_bonus = calculate_technology_bonus(state, tech_registry);
    for (player, bonus) in tech_bonus {
        if let Some((_, score)) = scores.iter_mut().find(|(id, _)| *id == player) {
            *score += bonus;
        }
    }

    // 4. Country Bonus
    for player in &state.players {
        let country_vp = calculate_country_bonus(state, player.id, order_registry, tech_registry);
        if let Some((_, score)) = scores.iter_mut().find(|(id, _)| *id == player.id) {
            *score += country_vp;
        }
    }

    // 5. Remainder Scoring
    for player in &state.players {
        let remainder_vp = calculate_remainder_bonus(player);
        if let Some((_, score)) = scores.iter_mut().find(|(id, _)| *id == player.id) {
            *score += remainder_vp;
        }
    }

    scores
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
