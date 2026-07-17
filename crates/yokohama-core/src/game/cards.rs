//! Card registries and Order card fulfillment.

use std::collections::HashMap;
use crate::types::*;

/// Registry for looking up Order cards by ID.
/// Single source of truth for card definitions; used by engine and CV integration.
#[derive(Debug, Clone)]
pub struct OrderRegistry {
    cards: HashMap<OrderCardId, OrderCard>,
    /// Stable string codes for CV integration (e.g., "order_eng_01")
    codes: HashMap<String, OrderCardId>,
}

impl OrderRegistry {
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
            codes: HashMap::new(),
        }
    }

    /// Insert a card with its stable code for CV integration.
    pub fn insert(&mut self, card: OrderCard, code: &str) {
        self.codes.insert(code.to_string(), card.id);
        self.cards.insert(card.id, card);
    }

    /// Get card by numeric ID (for engine).
    pub fn get(&self, id: OrderCardId) -> Option<&OrderCard> {
        self.cards.get(&id)
    }

    /// Get card by stable identifier (for CV integration).
    pub fn by_code(&self, code: &str) -> Option<&OrderCard> {
        self.codes.get(code).and_then(|id| self.cards.get(id))
    }

    pub fn iter(&self) -> impl Iterator<Item = &OrderCard> {
        self.cards.values()
    }
}

impl Default for OrderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for looking up Tech cards by ID.
/// Single source of truth for card definitions; used by engine and CV integration.
#[derive(Debug, Clone)]
pub struct TechRegistry {
    cards: HashMap<TechCardId, TechCard>,
    /// Stable string codes for CV integration (e.g., "tech_tram")
    codes: HashMap<String, TechCardId>,
}

impl TechRegistry {
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
            codes: HashMap::new(),
        }
    }

    /// Insert a card with its stable code for CV integration.
    pub fn insert(&mut self, card: TechCard, code: &str) {
        self.codes.insert(code.to_string(), card.id);
        self.cards.insert(card.id, card);
    }

    /// Get card by numeric ID (for engine).
    pub fn get(&self, id: TechCardId) -> Option<&TechCard> {
        self.cards.get(&id)
    }

    /// Get card by stable identifier (for CV integration).
    pub fn by_code(&self, code: &str) -> Option<&TechCard> {
        self.codes.get(code).and_then(|id| self.cards.get(id))
    }

    pub fn iter(&self) -> impl Iterator<Item = &TechCard> {
        self.cards.values()
    }
}

impl Default for TechRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderRegistry {
    /// The standard 36-card Order deck (verified 2026-07-17 against physical components).
    pub fn standard() -> Self {
        let mut r = Self::new();
        for (card, code) in crate::game::data::standard_orders() {
            r.insert(card, code);
        }
        r
    }
}

impl TechRegistry {
    /// The standard 30-card Technology deck: 20 unique titles, 10 of which have
    /// a duplicate physical card (verified 2026-07-17 against physical components).
    pub fn standard() -> Self {
        let mut r = Self::new();
        for (card, code) in crate::game::data::standard_techs() {
            r.insert(card, code);
        }
        r
    }
}

/// The standard 24 Building Site cards (verified 2026-07-17 against physical components).
pub fn standard_building_sites() -> Vec<BuildingSite> {
    crate::game::data::standard_sites()
}

/// Check if player can fulfill an Order card.
pub fn can_fulfill_order(
    state: &GameState,
    player: PlayerId,
    order_id: OrderCardId,
    order_registry: &OrderRegistry,
) -> bool {
    let p = state.player(player);

    // Must have the order in hand
    if !p.order_cards_in_hand.contains(&order_id) {
        return false;
    }

    // Must have required resources
    if let Some(card) = order_registry.get(order_id) {
        let req = &card.requirements;
        p.copper >= req.copper as u32
            && p.silk >= req.silk as u32
            && p.tea >= req.tea as u32
            && p.seafood >= req.seafood as u32
    } else {
        false
    }
}

/// Fulfill an Order card, paying costs and receiving rewards.
pub fn fulfill_order(
    state: &mut GameState,
    player: PlayerId,
    order_id: OrderCardId,
    order_registry: &OrderRegistry,
    tech_registry: &TechRegistry,
) -> Result<(), &'static str> {
    if !can_fulfill_order(state, player, order_id, order_registry) {
        return Err("Cannot fulfill order: missing requirements");
    }

    let card = order_registry.get(order_id).unwrap();
    let req = card.requirements;
    let rewards = card.rewards;
    let country = card.country;

    let p = state.player_mut(player);

    // Pay costs
    p.copper -= req.copper as u32;
    p.silk -= req.silk as u32;
    p.tea -= req.tea as u32;
    p.seafood -= req.seafood as u32;

    // Remove from hand, add to completed
    p.order_cards_in_hand.retain(|&id| id != order_id);
    p.completed_orders.push(order_id);

    // Receive rewards
    p.victory_points += rewards.victory_points as u32;
    p.money += rewards.money as u32;
    p.imported_goods += rewards.imported_goods as u32;
    p.copper += rewards.copper as u32;
    p.silk += rewards.silk as u32;
    p.tea += rewards.tea as u32;
    p.seafood += rewards.seafood as u32;

    // Check for Ball technology bonus (+2 VP)
    let has_ball = p.technologies.iter().any(|&tech_id| {
        tech_registry.get(tech_id).map(|t| t.effect == TechEffect::Ball).unwrap_or(false)
    });
    if has_ball {
        let p = state.player_mut(player);
        p.victory_points += 2;
    }

    // Check for Language School technology bonus (+1 Yen)
    let has_language_school = state.player(player).technologies.iter().any(|&tech_id| {
        tech_registry.get(tech_id).map(|t| t.effect == TechEffect::LanguageSchool).unwrap_or(false)
    });
    if has_language_school {
        state.player_mut(player).money += 1;
    }

    // Check for Foreign Mercenary eligibility
    check_mercenary_eligibility(state, player, country, order_registry, tech_registry);

    Ok(())
}

/// Check and award Foreign Mercenary if player has reached threshold.
pub(crate) fn check_mercenary_eligibility(
    state: &mut GameState,
    player: PlayerId,
    _country: Country,
    order_registry: &OrderRegistry,
    tech_registry: &TechRegistry,
) {
    // Count country icons for each country
    for country in Country::ALL {
        let p = state.player(player);
        let mut count = 0u8;

        for &order_id in &p.completed_orders {
            if let Some(card) = order_registry.get(order_id) {
                if card.country == country {
                    count += 1;
                }
            }
        }

        for &tech_id in &p.technologies {
            if let Some(card) = tech_registry.get(tech_id) {
                if card.country == country {
                    count += 1;
                }
            }
        }

        // Award mercenary at 2, 4, 6...
        let threshold = (count / 2) * 2;
        let already_has = p.mercenaries.available.get(&country).copied().unwrap_or(0)
            + p.mercenaries.used.get(&country).copied().unwrap_or(0);

        if threshold > already_has * 2 {
            // Player earned a new mercenary
            let supply = match country {
                Country::English => &mut state.mercenary_supply.english,
                Country::American => &mut state.mercenary_supply.american,
                Country::French => &mut state.mercenary_supply.french,
                Country::German => &mut state.mercenary_supply.german,
                Country::Dutch => &mut state.mercenary_supply.dutch,
            };

            if *supply > 0 {
                *supply -= 1;
                let p = state.player_mut(player);
                *p.mercenaries.available.entry(country).or_insert(0) += 1;
            }
        }
    }
}

/// Refill Port board from deck, placing new cards in rightmost empty slots
/// first. Also serves setup's initial fill: called against an empty board
/// with a sufficiently large deck, it fills all 6 slots.
pub fn refill_port(
    port: &mut PortBoard,
    deck: &mut Vec<OrderCardId>,
) {
    for i in (0..6).rev() {
        if port.slots[i].is_none() {
            port.slots[i] = deck.pop();
        }
    }
}

/// Remove a card from Lab and slide remaining cards left to fill the gap.
/// Rulebook p.13: Cards slide left, position affects cost (+0, +0, +1, +1, +2, +2).
/// Only one card leaves the Lab per action, so a single-gap shift suffices
/// (unlike Port, which can remove 2 non-contiguous cards in one action and
/// needs full compaction — see `execute_port_action`).
pub fn remove_and_slide_lab(lab: &mut LabBoard, slot_index: usize) -> Option<TechCardId> {
    if slot_index >= 6 {
        return None;
    }

    let taken = lab.slots[slot_index].take();

    // Slide cards left to fill the gap
    for i in slot_index..5 {
        if lab.slots[i].is_none() && lab.slots[i + 1].is_some() {
            lab.slots[i] = lab.slots[i + 1].take();
        }
    }
    // Note: rightmost slot is now None, will be filled by refill_lab

    taken
}

/// Refill Lab board from deck, placing new card in rightmost empty slot.
pub fn refill_lab(
    lab: &mut LabBoard,
    deck: &mut Vec<TechCardId>,
) {
    // Find rightmost empty slot and fill from deck
    for i in (0..6).rev() {
        if lab.slots[i].is_none() && !deck.is_empty() {
            lab.slots[i] = deck.pop();
            break; // Only fill one slot per refill
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lab_slide() {
        let mut lab = LabBoard {
            slots: [
                Some(TechCardId(1)),
                Some(TechCardId(2)),
                Some(TechCardId(3)),
                Some(TechCardId(4)),
                Some(TechCardId(5)),
                Some(TechCardId(6)),
            ],
        };

        // Take slot 1 (second from left)
        let taken = remove_and_slide_lab(&mut lab, 1);
        assert_eq!(taken, Some(TechCardId(2)));

        // Cards should slide left, rightmost slot now empty
        assert_eq!(lab.slots[0], Some(TechCardId(1)));
        assert_eq!(lab.slots[1], Some(TechCardId(3)));
        assert_eq!(lab.slots[2], Some(TechCardId(4)));
        assert_eq!(lab.slots[3], Some(TechCardId(5)));
        assert_eq!(lab.slots[4], Some(TechCardId(6)));
        assert_eq!(lab.slots[5], None);
    }

    #[test]
    fn test_port_refill_fills_rightmost_empty_slots_first() {
        let mut port = PortBoard {
            slots: [Some(OrderCardId(1)), None, None, None, None, None],
        };
        let mut deck = vec![OrderCardId(10), OrderCardId(11), OrderCardId(12)]; // top = 12

        refill_port(&mut port, &mut deck);

        // Rightmost 3 empty slots filled first (indices 5, 4, 3); the
        // 2 leftmost gaps (indices 1, 2) stay empty since the deck ran out.
        assert_eq!(port.slots, [
            Some(OrderCardId(1)),
            None,
            None,
            Some(OrderCardId(10)),
            Some(OrderCardId(11)),
            Some(OrderCardId(12)),
        ]);
        assert!(deck.is_empty());
    }

    #[test]
    fn test_port_refill_fills_all_six_from_empty() {
        // Also exercises setup's initial-fill use of refill_port: an empty
        // board with a sufficiently large deck fills every slot.
        let mut port = PortBoard::new();
        let mut deck: Vec<OrderCardId> = (1..=10).map(OrderCardId).collect();

        refill_port(&mut port, &mut deck);

        assert!(port.is_full());
        assert_eq!(deck.len(), 4);
    }

    #[test]
    fn test_lab_refill() {
        let mut lab = LabBoard {
            slots: [
                Some(TechCardId(1)),
                Some(TechCardId(2)),
                None,
                None,
                None,
                None,
            ],
        };
        let mut deck = vec![TechCardId(10)];

        refill_lab(&mut lab, &mut deck);

        // Rightmost empty slot filled
        assert_eq!(lab.slots[5], Some(TechCardId(10)));
        assert_eq!(lab.slots[4], None); // Only one card added per call
        assert_eq!(lab.slots[2], None);
        assert_eq!(lab.slots[3], None);
    }

    #[test]
    fn standard_registries_have_full_counts() {
        assert_eq!(OrderRegistry::standard().iter().count(), 36);
        assert_eq!(TechRegistry::standard().iter().count(), 30); // 30 physical cards
        assert_eq!(standard_building_sites().len(), 24);
    }

    #[test]
    fn order_ids_are_printed_corner_indices() {
        let orders = OrderRegistry::standard();
        let mut ids: Vec<u8> = orders.iter().map(|c| c.id.0).collect();
        ids.sort_unstable();
        assert_eq!(ids, (1..=36).collect::<Vec<u8>>());
        // Documented non-uniqueness of country+requirements: the GB twins.
        let (a, b) = (orders.get(OrderCardId(10)).unwrap(), orders.get(OrderCardId(32)).unwrap());
        assert_eq!(a.country, Country::English);
        assert_eq!(a.country, b.country);
        assert_eq!(a.requirements, b.requirements);
        assert_ne!(a.rewards, b.rewards);
    }

    #[test]
    fn tech_registry_models_30_card_deck() {
        let techs = TechRegistry::standard();
        let mut ids: Vec<u8> = techs.iter().map(|c| c.id.0).collect();
        ids.sort_unstable();
        assert_eq!(ids, (1..=30).collect::<Vec<u8>>());
        let mut by_name = std::collections::HashMap::new();
        for c in techs.iter() { *by_name.entry(c.name.clone()).or_insert(0u8) += 1; }
        assert_eq!(by_name.len(), 20);
        assert_eq!(by_name.values().filter(|&&n| n == 2).count(), 10);
    }

    #[test]
    fn building_sites_match_adjudicated_anatomy() {
        let sites = standard_building_sites();
        assert!(sites.iter().all(|s| s.spaces[0] == SpaceReward { vp: 1, ..Default::default() }));
        assert!(sites.iter().all(|s| s.spaces[2] == SpaceReward::default())); // ML blank
        // site_003 regression: TH 7 with TR vp4, MR tea2
        assert!(sites.iter().any(|s| s.trading_house_reward == 7
            && s.spaces[1] == SpaceReward { vp: 4, ..Default::default() }
            && s.spaces[3] == SpaceReward { tea: 2, ..Default::default() }));
        let mut th: Vec<u8> = sites.iter().map(|s| s.trading_house_reward).collect();
        th.sort_unstable();
        let dist: Vec<(u8, usize)> = [5u8, 6, 7, 8, 9].iter().map(|&v| (v, th.iter().filter(|&&x| x == v).count())).collect();
        assert_eq!(dist, vec![(5, 4), (6, 5), (7, 5), (8, 5), (9, 5)]);
    }
}
