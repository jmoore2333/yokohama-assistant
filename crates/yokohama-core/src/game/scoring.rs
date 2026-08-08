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

    award_top_two(&rankings, [6, 3])
}

/// Award the top two majority places, `vps[0]` to 1st and `vps[1]` to 2nd,
/// over a ranking already sorted best-first. Finding D1: the 2P dummy
/// ([`PlayerId::DUMMY`]) is ranked as a real competitor holding its one pawn,
/// so it can TAKE a place — but, being no real player, that place is awarded
/// to nobody. Its rank is still CONSUMED (the `place` index advances), so a
/// real player only earns the place it actually achieves behind the dummy.
/// For 3+ players (no dummy in the ranking) this is the plain top-two.
fn award_top_two(rankings: &[(PlayerId, u8, u8)], vps: [u32; 2]) -> Vec<(PlayerId, u32)> {
    let mut bonuses = Vec::new();
    for (place, vp) in vps.into_iter().enumerate() {
        if let Some((player, _, _)) = rankings.get(place) {
            if !player.is_dummy() {
                bonuses.push((*player, vp));
            }
        }
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

    award_top_two(&rankings, [8, 4])
}

/// Technology total industry value bonus: 1st = 10 VP, 2nd = 5 VP.
/// Tiebreaker: closest to start player (clockwise).
///
/// D5 (owner ruling 2026-08-08, rulebook p.9 final-scoring NOTE): in
/// 2-player games the setup dummy competes in this majority too, at an
/// industry value of EXACTLY 7.5 — "the dummy player ends the game with an
/// industry value of 7.5". Observed on BGA: owner 1st (10 VP), the dummy's
/// 7.5 beat the opponent's lower total for 2nd, which was then awarded to
/// nobody. Industry values are compared as DOUBLED integers so the half is
/// exact (the dummy is 15; real totals are even, so a tie against the dummy
/// is unrepresentable and the clockwise tiebreak can never involve it — the
/// dummy's `distance` entry below is a placeholder, not a reachable key).
/// This extends the 2026-07-21 finding-D1 church/customs modeling: the dummy
/// can TAKE a place, and a taken place is awarded to nobody
/// (`award_top_two`'s semantics, restated here over the doubled-IV ranking).
fn calculate_technology_bonus(state: &GameState, tech_registry: &TechRegistry) -> Vec<(PlayerId, u32)> {
    // Total industry value per player, DOUBLED (see the doc above).
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
            (p.id, 2 * total_iv, distance)
        })
        .collect();

    // D5: the 2P dummy contender at doubled-IV 15 (= 7.5 exactly). Presence
    // is keyed on the player count, matching invariant I9 ("2P state ⟹ the
    // setup dummy is on the board") — the dummy's industry value is a rule
    // about 2-player final scoring, not a function of any track pawn.
    if state.config.player_count == 2 {
        totals.push((PlayerId::DUMMY, 15, u8::MAX));
    }

    // Sort by doubled industry value (desc), then by distance from start
    // player (asc).
    totals.sort_by(|a, b| {
        b.1.cmp(&a.1).then(a.2.cmp(&b.2))
    });

    let mut bonuses = Vec::new();
    // Participation gate: `iv > 0` is equivalent to "owns >=1 tech card"
    // ONLY because every tech in the standard data has industry_cost >= 3
    // (08-tech-cards.yaml min is 3). The rulebook's literal gate is
    // ownership; if a zero-industry tech is ever added, this must become an
    // ownership check or a 0-iv owner would wrongly be excluded (Stage-2
    // differential review note, 2026-07-20). The dummy (15 > 0) always
    // participates at 2P; like the church/customs rankings, a place it takes
    // is CONSUMED and awarded to nobody.
    for (place_vp, entry) in [(10u32, totals.first()), (5u32, totals.get(1))] {
        if let Some((player, doubled_iv, _)) = entry {
            if *doubled_iv > 0 && !player.is_dummy() {
                bonuses.push((*player, place_vp));
            }
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
    use crate::game::{finalize_setup, new_game, StartingOrderChoice};

    /// A freshly finalized 2-player game (real board + decks). After finding
    /// D1 this also carries the setup dummy on Church faith-7 (index 3) and
    /// Customs space-4 (index 3) -- the D1 scoring tests park real-player
    /// pawns at OTHER indices and read the dummy's competition back out of
    /// the majority bonuses.
    fn finalized_2p() -> (GameState, Rules) {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 7).unwrap();
        let state = finalize_setup(
            pending,
            &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
        )
        .unwrap();
        (state, rules)
    }

    /// Finding D1 (rulebook p.9 NOTE, Jeremy's edition -- not the bundled PDF; see guidingpath's 2026-07-21 DL rows) scenario (a): the 2P dummy on Church
    /// faith-7 holds a majority PLACE, denying it to a real player. Both real
    /// players hold one pawn LEFT of faith-7, so the dummy (rightmost) takes
    /// 1st -- awarded to nobody -- and P1 collects only 2nd (3 VP) instead of
    /// the 1st it would win with the dummy absent. RED before D1: setup places
    /// no dummy, so P1 would hold 1st and receive 6.
    #[test]
    fn church_majority_2p_dummy_takes_first_place() {
        let (mut state, _rules) = finalized_2p();
        state.church_track.slots[ChurchTrack::faith_to_index(4).unwrap()] =
            Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 1 });
        state.church_track.slots[ChurchTrack::faith_to_index(5).unwrap()] =
            Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 2 });

        let bonuses = calculate_church_bonus(&state);
        assert!(
            !bonuses.iter().any(|(_, vp)| *vp == 6),
            "the dummy holds 1st, so no real player receives the 6-VP first place: {bonuses:?}"
        );
        assert!(
            bonuses.contains(&(PlayerId::new(1), 3)),
            "P1 (rightmost real, faith-5) takes 2nd behind the dummy: {bonuses:?}"
        );
        assert!(
            !bonuses.iter().any(|(p, _)| *p == PlayerId::new(0)),
            "P0 (faith-4) is beaten by both the dummy and P1: {bonuses:?}"
        );
    }

    /// Finding D1 scenario (b): the 2P dummy on Customs space-4 wins a
    /// count-tie against a real player purely by the rightmost tie-break. P0
    /// holds space-3, the dummy holds space-4 (further right) -- both count 1
    /// -- so the dummy takes 1st and P0 drops to 2nd (4 VP) from the 1st (8
    /// VP) it would win with the dummy absent. RED before D1.
    #[test]
    fn customs_majority_2p_dummy_wins_tie_via_rightmost() {
        let (mut state, _rules) = finalized_2p();
        state.customs_track.slots[CustomsTrack::space_to_index(3).unwrap()] =
            Some(CustomsSlot { player: PlayerId::new(0), arrival_order: 1 });
        state.customs_track.slots[CustomsTrack::space_to_index(1).unwrap()] =
            Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 2 });

        let bonuses = calculate_customs_bonus(&state);
        assert!(
            !bonuses.iter().any(|(_, vp)| *vp == 8),
            "the dummy wins the rightmost tie for 1st, so no real player gets 8: {bonuses:?}"
        );
        assert!(
            bonuses.contains(&(PlayerId::new(0), 4)),
            "P0 (space-3) takes 2nd behind the dummy: {bonuses:?}"
        );
        assert!(
            !bonuses.iter().any(|(p, _)| *p == PlayerId::new(1)),
            "P1 (space-1) is beaten by both the dummy and P0: {bonuses:?}"
        );
    }

    /// Finding D1 scenario (c): when both real players outrank the dummy on
    /// COUNT (2 pawns each vs the dummy's lone pawn), the dummy places third
    /// and changes nothing -- 2P scores are identical to the pre-D1 behaviour.
    /// GREEN both before and after: a regression guard, not a bug reproducer.
    #[test]
    fn church_majority_2p_dummy_beaten_by_both_leaves_scores_unchanged() {
        let (mut state, _rules) = finalized_2p();
        state.church_track.slots[ChurchTrack::faith_to_index(4).unwrap()] =
            Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 1 });
        state.church_track.slots[ChurchTrack::faith_to_index(8).unwrap()] =
            Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 2 });
        state.church_track.slots[ChurchTrack::faith_to_index(5).unwrap()] =
            Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 3 });
        state.church_track.slots[ChurchTrack::faith_to_index(9).unwrap()] =
            Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 4 });

        let bonuses = calculate_church_bonus(&state);
        assert!(
            bonuses.contains(&(PlayerId::new(1), 6)),
            "P1 (rightmost of the count-2 tie) takes 1st: {bonuses:?}"
        );
        assert!(
            bonuses.contains(&(PlayerId::new(0), 3)),
            "P0 takes 2nd; the lone-pawn dummy is beaten by both: {bonuses:?}"
        );
    }

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

    /// Standard order-card countries this file's country tests rely on
    /// (data.rs): O01 American, O02 French, O03 English, O04 German,
    /// O06 Dutch; O05 French, O08 Dutch, O09 German, O10 English,
    /// O12 American, O13 English. Set completed_orders to these ids and the
    /// real engine `calculate_country_bonus` scores the set.
    fn country_bonus(state: &GameState, rules: &Rules, order_ids: &[u8]) -> u32 {
        let mut state = state.clone();
        state.players[0].completed_orders =
            order_ids.iter().map(|&id| OrderCardId(id)).collect();
        calculate_country_bonus(&state, PlayerId::new(0), &rules.orders, &rules.techs)
    }

    /// Item-5 rewrite (was zero-assertion `test_country_bonus_full_set`):
    /// FIVE distinct countries = one 5-set = 12 VP, driven through the real
    /// engine `calculate_country_bonus` against a LITERAL expected value.
    #[test]
    fn country_bonus_full_set_is_12() {
        let rules = Rules::standard();
        let state = create_test_game_state(2);
        // American, French, English, German, Dutch -> 5 distinct.
        assert_eq!(country_bonus(&state, &rules, &[1, 2, 3, 4, 6]), 12);
    }

    /// Item-5 rewrite (was `test_country_set_distinct_countries`, which never
    /// called engine code): repeated icons of one country do NOT inflate the
    /// set -- 3 English + 1 French + 1 German is 3 DISTINCT = 4 VP, not five
    /// icons' worth. Literal expected value against the engine.
    #[test]
    fn country_bonus_counts_distinct_not_icons() {
        let rules = Rules::standard();
        let state = create_test_game_state(2);
        // O03/O10/O13 English + O02 French + O04 German: distinct = 3.
        assert_eq!(country_bonus(&state, &rules, &[3, 10, 13, 2, 4]), 4);
    }

    /// Item-5 rewrite (was `test_country_set_multiple_sets`, which asserted
    /// against its own copied algorithm): 2 icons of each of the 5 countries
    /// = two 5-sets = 24 VP, through the engine, LITERAL expected value.
    #[test]
    fn country_bonus_two_full_sets_is_24() {
        let rules = Rules::standard();
        let state = create_test_game_state(2);
        // {1,12}=American {2,5}=French {3,10}=English {4,9}=German {6,8}=Dutch.
        assert_eq!(country_bonus(&state, &rules, &[1, 12, 2, 5, 3, 10, 4, 9, 6, 8]), 24);
    }

    /// Item-6: the country-bonus set table (2/3/4/5 distinct = 2/4/8/12 VP)
    /// plus a >5-ICON case that stays few-distinct, all through the engine
    /// against literal values.
    #[test]
    fn calculate_country_bonus_covers_the_table() {
        let rules = Rules::standard();
        let state = create_test_game_state(2);
        // 2 distinct = 2 VP (American + French).
        assert_eq!(country_bonus(&state, &rules, &[1, 2]), 2);
        // 3 distinct = 4 VP (American + French + English).
        assert_eq!(country_bonus(&state, &rules, &[1, 2, 3]), 4);
        // 4 distinct = 8 VP (+ German).
        assert_eq!(country_bonus(&state, &rules, &[1, 2, 3, 4]), 8);
        // 5 distinct = 12 VP (+ Dutch).
        assert_eq!(country_bonus(&state, &rules, &[1, 2, 3, 4, 6]), 12);
        // >5 ICONS, only 3 distinct: 4 English (3,10,13,15) + 1 French + 1
        // German = 6 icons. Greedy: one 3-set (4 VP), then leftover English
        // is a lone icon (< 2 distinct) = nothing more. Icons do not inflate.
        assert_eq!(country_bonus(&state, &rules, &[3, 10, 13, 15, 2, 4]), 4);
    }

    /// Item-6: `calculate_church_bonus` directly (6 VP 1st / 3 VP 2nd),
    /// including the POSITIONAL tie-break -- equal pawn counts resolve by
    /// rightmost (higher-index) slot.
    #[test]
    fn calculate_church_bonus_awards_6_and_3_with_positional_tiebreak() {
        let mut state = create_test_game_state(2);
        // Both players have exactly 1 pawn; p0's is further right (index 5 >
        // index 3), so p0 takes 1st (6 VP), p1 2nd (3 VP).
        state.church_track.slots[5] =
            Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 0 });
        state.church_track.slots[3] =
            Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 1 });

        let bonuses = calculate_church_bonus(&state);
        assert_eq!(bonuses.len(), 2);
        assert_eq!(bonuses[0], (PlayerId::new(0), 6), "rightmost slot wins the tie -> 1st = 6");
        assert_eq!(bonuses[1], (PlayerId::new(1), 3), "2nd place = 3");
    }

    /// Item-6: `calculate_customs_bonus` directly (8 VP 1st / 4 VP 2nd) on a
    /// clear count majority.
    #[test]
    fn calculate_customs_bonus_awards_8_and_4() {
        let mut state = create_test_game_state(2);
        // p0 has 2 pawns, p1 has 1 -> p0 1st (8), p1 2nd (4).
        state.customs_track.slots[0] =
            Some(CustomsSlot { player: PlayerId::new(0), arrival_order: 0 });
        state.customs_track.slots[1] =
            Some(CustomsSlot { player: PlayerId::new(0), arrival_order: 1 });
        state.customs_track.slots[6] =
            Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 2 });

        let bonuses = calculate_customs_bonus(&state);
        assert_eq!(bonuses.len(), 2);
        assert_eq!(bonuses[0], (PlayerId::new(0), 8));
        assert_eq!(bonuses[1], (PlayerId::new(1), 4));
    }

    /// Item-6: `calculate_technology_bonus` directly (10 VP 1st / 5 VP 2nd)
    /// with the CLOCKWISE tie-break on equal industry value. p1 and p2 both
    /// own one industry-value-3 tech; start player is 0, so p1 (distance 1)
    /// outranks tied p2 (distance 2).
    #[test]
    fn calculate_technology_bonus_awards_10_and_5_with_clockwise_tiebreak() {
        let rules = Rules::standard();
        let mut state = create_test_game_state(3);
        state.config.start_player = PlayerId::new(0);
        // Language School copies (id 1 and 2), each industry_cost 3.
        state.players[1].technologies = vec![TechCardId(1)];
        state.players[2].technologies = vec![TechCardId(2)];
        // p0 owns no tech -> industry value 0, excluded from the top two.

        let bonuses = calculate_technology_bonus(&state, &rules.techs);
        assert_eq!(bonuses.len(), 2);
        assert_eq!(bonuses[0], (PlayerId::new(1), 10), "clockwise-nearest of the tie -> 1st = 10");
        assert_eq!(bonuses[1], (PlayerId::new(2), 5), "2nd = 5");
    }

    // --- D5 (owner ruling 2026-08-08): the 2P dummy in the TECHNOLOGY majority ---

    /// Registry of test techs with chosen industry costs, ids 1..=n.
    fn tech_registry_with_costs(costs: &[u8]) -> TechRegistry {
        let mut reg = TechRegistry::new();
        for (i, &cost) in costs.iter().enumerate() {
            let id = (i + 1) as u8;
            reg.insert(
                TechCard {
                    id: TechCardId(id),
                    name: format!("Test Tech {id}"),
                    country: Country::English,
                    industry_cost: cost,
                    effect: TechEffect::Tram,
                },
                "tech_test",
            );
        }
        reg
    }

    /// D5 (rulebook p.9 final-scoring NOTE, owner ruling 2026-08-08): the 2P
    /// dummy competes in the Technology majority at industry value EXACTLY
    /// 7.5. The BGA-observed shape: owner 1st (10 VP), the dummy's 7.5 beats
    /// the opponent for 2nd — awarded to nobody — so the opponent gets
    /// NOTHING. RED before D5: the opponent's 5 collected the 5-VP second
    /// place.
    #[test]
    fn tech_majority_2p_dummy_takes_second_at_seven_and_a_half() {
        let reg = tech_registry_with_costs(&[10, 5]);
        let mut state = create_test_game_state(2);
        state.config.start_player = PlayerId::new(0);
        state.players[0].technologies = vec![TechCardId(1)]; // IV 10
        state.players[1].technologies = vec![TechCardId(2)]; // IV 5 < 7.5

        let bonuses = calculate_technology_bonus(&state, &reg);
        assert_eq!(
            bonuses,
            vec![(PlayerId::new(0), 10)],
            "dummy at 7.5 takes 2nd (awarded to nobody); IV-5 opponent gets nothing"
        );
    }

    /// D5 exactness: the half is load-bearing. IV 8 BEATS the dummy's 7.5
    /// (2nd place, 5 VP); IV 7 LOSES to it (nothing). A model that rounded
    /// 7.5 to 7 or 8 fails one of these two halves.
    #[test]
    fn tech_majority_2p_dummy_industry_is_exactly_seven_and_a_half() {
        let reg = tech_registry_with_costs(&[10, 8]);
        let mut state = create_test_game_state(2);
        state.config.start_player = PlayerId::new(0);
        state.players[0].technologies = vec![TechCardId(1)]; // IV 10
        state.players[1].technologies = vec![TechCardId(2)]; // IV 8 > 7.5

        let bonuses = calculate_technology_bonus(&state, &reg);
        assert_eq!(
            bonuses,
            vec![(PlayerId::new(0), 10), (PlayerId::new(1), 5)],
            "IV 8 beats 7.5 — the dummy places third and changes nothing"
        );

        let reg = tech_registry_with_costs(&[10, 7]);
        state.players[1].technologies = vec![TechCardId(2)]; // IV 7 < 7.5
        let bonuses = calculate_technology_bonus(&state, &reg);
        assert_eq!(
            bonuses,
            vec![(PlayerId::new(0), 10)],
            "IV 7 loses to 7.5 — the dummy consumes 2nd"
        );
    }

    /// D5: the dummy can take FIRST — both real players below 7.5 leaves the
    /// 10-VP place awarded to nobody and the better real player on 2nd.
    #[test]
    fn tech_majority_2p_dummy_takes_first_when_both_players_are_below_it() {
        let reg = tech_registry_with_costs(&[5, 3]);
        let mut state = create_test_game_state(2);
        state.config.start_player = PlayerId::new(0);
        state.players[0].technologies = vec![TechCardId(1)]; // IV 5
        state.players[1].technologies = vec![TechCardId(2)]; // IV 3

        let bonuses = calculate_technology_bonus(&state, &reg);
        assert_eq!(
            bonuses,
            vec![(PlayerId::new(0), 5)],
            "dummy 1st (awarded to nobody), P0 2nd for 5 VP, P1 nothing"
        );
    }

    /// D5 scope guard: the dummy exists at 2 players only — a 3P ranking is
    /// exactly what it was (this fixture is the shipped clockwise-tiebreak
    /// test's shape, re-asserted post-D5).
    #[test]
    fn tech_majority_3p_has_no_dummy_contender() {
        let reg = tech_registry_with_costs(&[3, 3]);
        let mut state = create_test_game_state(3);
        state.config.start_player = PlayerId::new(0);
        state.players[1].technologies = vec![TechCardId(1)];
        state.players[2].technologies = vec![TechCardId(2)];

        let bonuses = calculate_technology_bonus(&state, &reg);
        assert_eq!(bonuses, vec![(PlayerId::new(1), 10), (PlayerId::new(2), 5)]);
    }

    /// Item-6: a remainder test whose MERCENARY leg is nonzero (both shipped
    /// remainder tests use a fresh player with zero mercenaries, so that leg
    /// contributed 0 in every existing case). 3 unused (face-up) mercenaries
    /// = 3 VP, on top of the other conversions.
    #[test]
    fn remainder_bonus_counts_unused_mercenaries() {
        let mut player = PlayerState::new(PlayerId::new(0), PlayerColor::Red, true);
        // Zero the starting 1/1/1/1 trade goods so only the legs under test count.
        player.copper = 0;
        player.silk = 0;
        player.tea = 0;
        player.seafood = 0;
        player.mercenaries.available.insert(Country::English, 2);
        player.mercenaries.available.insert(Country::French, 1);
        player.mercenaries.used.insert(Country::German, 4); // face-down: NOT scored
        player.imported_goods = 1; // +1
        player.money = 4; // +2 (4/2)
        // 3 unused mercenaries + 1 imported + 2 money = 6 VP.
        assert_eq!(calculate_remainder_bonus(&player), 6);
    }
}
