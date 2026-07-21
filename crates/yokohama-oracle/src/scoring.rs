//! Independent final-scoring + ranking oracle (Phase 2.9, Stage 2).
//!
//! Authored from the rules sources ONLY — rulebook pp.8–10 (End of Game
//! Conditions, Final Score Calculation, Winning the Game), KB
//! `01-rules-reference.md` §Final Scoring / §Winning, and `guidingpath.md`'s
//! decision log (the 2026-07-19 engine-authority row pins the engine-side
//! `final_result` winner API as "the official clockwise tie-break"; this module
//! derives the same procedure independently from the rulebook's own text). No
//! engine scoring/end-game file was read.
//!
//! A player's FINAL total VP at a terminal state comprises:
//!
//! 1. **Banked VP** — `PlayerState::victory_points`, the VP track as it stands.
//!    Every in-game score (order fulfillment, achievements, building-site
//!    rewards, tech instants, church/customs placement rewards, Port-at-5 +3,
//!    …) was banked when it happened; the rulebook's "final score calculation"
//!    [RB p.9] is a list of ADDITIONS on top of the running total.
//! 2. **Church bonus** [RB p.9 §1; KB §Final Scoring 1] — most assistants on
//!    the Church Management Board: 6 VP, second most: 3 VP. Count ties break
//!    by the RIGHTMOST assistant pawn (positional — RB p.9's example compares
//!    "the position of the pawns"); players with no pawn on the board never
//!    score ("If there is only 1 player on the board, that player earns 6
//!    points and no other player receives points"). In a 2-player game the
//!    setup dummy competes too [RB p.9 NOTE, via DL 2026-07-21 D1: "The dummy
//!    player's assistants are taken into account during final scoring to
//!    increase competition for majority bonuses"] — one pawn at faith-7,
//!    ranked with the real players but never itself awarded (see
//!    `track_awards`). Derived by rule from the 2-player count, so a "sole"
//!    real player is never truly alone at 2P: the dummy is always on the board.
//! 3. **Customs bonus** [RB p.9 §2; KB §Final Scoring 2] — same procedure,
//!    8 VP / 4 VP, on the Customs Management Board; the 2-player dummy competes
//!    here too, one pawn at space-4.
//! 4. **Technology bonus** [RB p.9–10 §3; KB §Final Scoring 3] — highest total
//!    printed Industry value on owned Technology cards: 10 VP, second: 5 VP.
//!    Only players who own at least one Technology card participate. Ties
//!    break by closeness clockwise to the Start player, "the Start player
//!    himself is considered to be the closest" [RB p.9].
//! 5. **Country bonus** [RB p.10 §4; KB §Final Scoring 4] — unfulfilled orders
//!    return to the box (score nothing); Country icons on COMPLETED Order
//!    cards and owned Technology cards divide into sets of DIFFERENT icons:
//!    5 → 12 VP, 4 → 8, 3 → 4, 2 → 2, 1 → 0; each icon used once. The oracle
//!    scores the optimal division (largest-sets-first / conjugate partition,
//!    KB: "Optimal: form largest sets first"), brute-force-verified optimal
//!    for all per-icon counts ≤ 7 in the tests; beyond that by convexity of
//!    the set-value table (increments 0,2,2,4,4 are non-decreasing, so
//!    largest-sets-first cannot lose by exchange).
//! 6. **Remainder scoring** [RB p.10 §5; KB §Final Scoring 5] — 1 VP per
//!    unused (face-up) Foreign Mercenary token, 1 VP per Imported Good token,
//!    1 VP per full 2 Yen, 1 VP per full 3 Trade Goods (any combination).
//!
//! **Ranking** [RB p.10 Winning the Game; KB §Winning; decision log 2026-07-19
//! engine-authority row]: most VP wins; ties go to the player closest to the
//! Start player clockwise (the Start player closest). Seat order is `PlayerId`
//! order — `GameState::next_player` is `(id+1) % n` (types/state.rs), so
//! clockwise distance from the Start player is `(id + n - start) % n`, which is
//! unique per player: the output is a STRICT total order with no shared ranks.
//!
//! Reading choices (documented, differential-relevant):
//! - Church/Customs counts and rightmost positions are read from the TRACK
//!   state (`ChurchTrack::slots` / `CustomsTrack::slots`, the physical board:
//!   one pawn per printed space, left-to-right index order), NOT from the
//!   per-player `church_positions`/`customs_positions` value vectors — the
//!   board is what the rulebook's "rightmost" refers to, and slot occupancy
//!   makes it strict (two pawns never share a space).
//! - "Rightmost" is POSITIONAL. The rulebook example [RB p.9] compares pawn
//!   POSITIONS, so this oracle uses the highest occupied slot index.
//!   (Probe outcome, Stage-2 sweeps 2026-07-20: the engine ALSO scores
//!   positionally — the old `arrival_order` field comment in types/tracks.rs
//!   described a reading the engine never implemented and has been corrected.
//!   Note the differential confirms the two sides AGREE; it cannot by itself
//!   adjudicate positional-vs-arrival — the rulebook's examples do.)

use serde::{Deserialize, Serialize};
use yokohama_core::{ChurchTrack, Country, CustomsTrack, GameState, PlayerId, PlayerState, Rules};

/// One row of the final ranking: a player and their final total VP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerStanding {
    pub player: PlayerId,
    pub total_vp: u32,
}

/// Final standings for a terminal `state`, ranked best-first with the official
/// tie-break fully applied (a strict total order — no ties in the output).
///
/// Total per player = banked VP track + Church bonus + Customs bonus +
/// Technology bonus + Country bonus + Remainder scoring [RB p.9-10]; ranking
/// = VP descending, ties by clockwise closeness to the Start player [RB p.10
/// Winning the Game; decision log 2026-07-19 engine-authority row].
pub fn final_standings(state: &GameState, rules: &Rules) -> Vec<PlayerStanding> {
    let n = state.players.len();

    // The 2-player setup dummy competes in BOTH majority bonuses [RB p.9 NOTE,
    // via decision log 2026-07-21 D1: "The dummy player's assistants are taken
    // into account during final scoring to increase competition for majority
    // bonuses"]. It is one pawn at church faith-7 / customs space-4 — the
    // spaces it pre-blocks. Derived BY RULE from the 2-player count (the same
    // by-rule precedent the placement-eligibility code uses), NOT read from any
    // engine dummy field: 3/4-player games have no dummy.
    let (church_dummy_slot, customs_dummy_slot) = if state.config.player_count == 2 {
        (
            Some(ChurchTrack::faith_to_index(7).expect("faith 7 always maps")),
            Some(CustomsTrack::space_to_index(4).expect("space 4 always maps")),
        )
    } else {
        (None, None)
    };

    // Comparative bonuses computed board-wide, then per-player additions.
    let church = track_awards(
        state.church_track.slots.iter().map(|s| s.map(|x| x.player)),
        n,
        6,
        3,
        church_dummy_slot,
    );
    let customs = track_awards(
        state.customs_track.slots.iter().map(|s| s.map(|x| x.player)),
        n,
        8,
        4,
        customs_dummy_slot,
    );
    let technology = technology_awards(state, rules);

    let mut totals: Vec<u32> = Vec::with_capacity(n);
    for (i, p) in state.players.iter().enumerate() {
        let banked = p.victory_points;
        let country = country_score_from_counts(&icon_counts(p, rules));
        let remainder = remainder_score(p);
        totals.push(banked + church[i] + customs[i] + technology[i] + country + remainder);
    }

    // Ranking: VP descending; ties by clockwise distance from the Start
    // player ascending (Start player distance 0 — "considered closest",
    // RB p.9 parenthetical applied to Winning per RB p.10). Distances are
    // unique per seat, so the order is strict.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        totals[b]
            .cmp(&totals[a])
            .then(clockwise_distance(state, a).cmp(&clockwise_distance(state, b)))
    });
    order
        .into_iter()
        .map(|i| PlayerStanding { player: state.players[i].id, total_vp: totals[i] })
        .collect()
}

/// Clockwise seats-from-Start-player distance. Seat order is `PlayerId` order
/// (`GameState::next_player` = `(id+1) % n`, types/state.rs); the Start player
/// himself is distance 0 [RB p.9].
fn clockwise_distance(state: &GameState, seat: usize) -> usize {
    let n = state.players.len();
    let start = state.config.start_player.0 as usize;
    (seat + n - start) % n
}

/// Church/Customs comparative bonus [RB p.9 §1-§2]: most pawns on the track
/// takes `first_vp`, second most `second_vp`; count ties break by the
/// RIGHTMOST pawn (highest occupied slot index — positional per RB p.9's
/// example); players with no pawn never score. Returns per-seat awards.
///
/// `dummy_slot` carries the 2-player setup dummy [RB p.9 NOTE; DL 2026-07-21
/// D1]: `Some(idx)` is the fixed slot of its single pawn (church faith-7 /
/// customs space-4), `None` in 3/4-player games where no dummy exists. The
/// dummy is RANKED alongside the real players so it can deny or take a place,
/// but any place it takes is awarded to nobody. The caller derives it by rule
/// from the player count, never from an engine dummy field; the dummy's fixed
/// space is one no real player can occupy in a 2P game, so the real-player scan
/// (which drops any owner id `>= n`, the `PlayerId::DUMMY` sentinel included)
/// and the by-rule dummy never double-count.
fn track_awards(
    slots: impl Iterator<Item = Option<PlayerId>>,
    n: usize,
    first_vp: u32,
    second_vp: u32,
    dummy_slot: Option<usize>,
) -> Vec<u32> {
    let mut count = vec![0u32; n];
    let mut rightmost = vec![0usize; n]; // valid only where count > 0
    for (idx, owner) in slots.enumerate() {
        if let Some(p) = owner {
            let seat = p.0 as usize;
            if seat < n {
                count[seat] += 1;
                rightmost[seat] = idx; // slots iterate left-to-right
            }
        }
    }
    // Competitors for the two places: each real player holding a pawn, plus —
    // in a 2P game — the dummy (one pawn at its fixed slot). `Some(seat)` is a
    // real player; `None` is the dummy, ranked but never awarded.
    let mut competitors: Vec<(u32, usize, Option<usize>)> =
        (0..n).filter(|&i| count[i] > 0).map(|i| (count[i], rightmost[i], Some(i))).collect();
    if let Some(slot) = dummy_slot {
        competitors.push((1, slot, None));
    }
    // (count desc, rightmost desc). One pawn per space — the dummy's fixed
    // space being one no real player can occupy — makes `rightmost` unique
    // across all competitors, so the order is strict.
    competitors.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    let mut awards = vec![0u32; n];
    // A place held by the dummy (`None`) is awarded to nobody, but it still
    // pushes the real players below it down a rank.
    if let Some(&(_, _, Some(seat))) = competitors.first() {
        awards[seat] = first_vp;
    }
    if let Some(&(_, _, Some(seat))) = competitors.get(1) {
        awards[seat] = second_vp;
    }
    awards
}

/// Technology bonus [RB p.9-10 §3]: highest total printed Industry value on
/// owned Technology cards takes 10 VP, second 5 VP; only owners of at least
/// one Technology card participate; ties break by clockwise closeness to the
/// Start player (Start player closest). Returns per-seat awards.
fn technology_awards(state: &GameState, rules: &Rules) -> Vec<u32> {
    let n = state.players.len();
    let industry: Vec<u32> = state
        .players
        .iter()
        .map(|p| {
            p.technologies
                .iter()
                .filter_map(|id| rules.techs.get(*id))
                .map(|t| t.industry_cost as u32)
                .sum()
        })
        .collect();
    let mut owners: Vec<usize> =
        (0..n).filter(|&i| !state.players[i].technologies.is_empty()).collect();
    owners.sort_by(|&a, &b| {
        industry[b]
            .cmp(&industry[a])
            .then(clockwise_distance(state, a).cmp(&clockwise_distance(state, b)))
    });
    let mut awards = vec![0u32; n];
    if let Some(&first) = owners.first() {
        awards[first] = 10;
    }
    if let Some(&second) = owners.get(1) {
        awards[second] = 5;
    }
    awards
}

/// Country-bonus score for icon counts (indexed by `Country::ALL` order):
/// divide the icons into sets of DIFFERENT icons, 5→12 / 4→8 / 3→4 / 2→2 /
/// 1→0 VP, each icon used once [RB p.10 §4]. Scores the OPTIMAL division via
/// the conjugate partition — the j-th set takes every icon type with count
/// ≥ j (largest sets first, KB §Final Scoring 4 "Optimal: form largest sets
/// first"); brute-force-verified optimal for all per-icon counts ≤ 7 in this
/// module's tests, and by set-value convexity beyond.
fn country_score_from_counts(counts: &[u32; 5]) -> u32 {
    fn set_value(size: usize) -> u32 {
        match size {
            5 => 12,
            4 => 8,
            3 => 4,
            2 => 2,
            _ => 0,
        }
    }
    let max = counts.iter().copied().max().unwrap_or(0);
    (1..=max).map(|j| set_value(counts.iter().filter(|&&c| c >= j).count())).sum()
}

/// Icon counts (by `Country::ALL` index) from a player's COMPLETED Order
/// cards and owned Technology cards [RB p.10 §4]. Unfulfilled orders (hand /
/// hidden starting order) return to the box and contribute nothing. Ids
/// unknown to `rules` contribute nothing (differential runs use standard
/// rules, where every id resolves).
fn icon_counts(player: &PlayerState, rules: &Rules) -> [u32; 5] {
    let mut counts = [0u32; 5];
    for id in &player.completed_orders {
        if let Some(card) = rules.orders.get(*id) {
            counts[country_index(card.country)] += 1;
        }
    }
    for id in &player.technologies {
        if let Some(card) = rules.techs.get(*id) {
            counts[country_index(card.country)] += 1;
        }
    }
    counts
}

fn country_index(c: Country) -> usize {
    Country::ALL.iter().position(|&x| x == c).expect("Country::ALL is total")
}

/// Remainder scoring [RB p.10 §5]: 1 VP per unused (face-up) Foreign
/// Mercenary token, 1 VP per Imported Good token, 1 VP per full 2 Yen, 1 VP
/// per full 3 Trade Goods in any combination. Used (face-down) mercenaries
/// and kept 5-Power tokens score nothing.
fn remainder_score(player: &PlayerState) -> u32 {
    player.mercenaries.unused_count() as u32
        + player.imported_goods
        + player.money / 2
        + player.total_trade_goods() / 3
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{four_player_state, three_player_state, two_player_state};
    use std::collections::HashMap;
    use yokohama_core::{ChurchSlot, CustomsSlot, PlayerId, Rules};

    /// Zero every remainder-scoring resource so component fixtures are exact.
    fn zero_resources(s: &mut yokohama_core::GameState) {
        for p in &mut s.players {
            p.money = 0;
            p.copper = 0;
            p.silk = 0;
            p.tea = 0;
            p.seafood = 0;
            p.imported_goods = 0;
        }
    }

    fn total_of(standings: &[PlayerStanding], id: u8) -> u32 {
        standings
            .iter()
            .find(|st| st.player == PlayerId::new(id))
            .expect("every player appears in the standings")
            .total_vp
    }

    fn rank_order(standings: &[PlayerStanding]) -> Vec<u8> {
        standings.iter().map(|st| st.player.0).collect()
    }

    /// Place one church pawn for `player` at Faith `faith` (rules-space terms,
    /// so fixtures read in the units the rulebook uses).
    fn church_pawn(s: &mut GameState, faith: u8, player: u8) {
        let idx = yokohama_core::ChurchTrack::faith_to_index(faith).expect("faith 4-11 maps");
        s.church_track.slots[idx] =
            Some(ChurchSlot { player: PlayerId::new(player), arrival_order: 0 });
    }

    /// Place one customs pawn for `player` at printed space `space` (1-7).
    fn customs_pawn(s: &mut GameState, space: u8, player: u8) {
        let idx = yokohama_core::CustomsTrack::space_to_index(space).expect("spaces 1-7 map");
        s.customs_track.slots[idx] =
            Some(CustomsSlot { player: PlayerId::new(player), arrival_order: 0 });
    }

    // --- 1. Banked VP only (no-op state) ---

    #[test]
    fn no_op_state_total_equals_vp_track() {
        // Nothing on any track, no cards, no resources: the final total is
        // exactly the banked VP track [RB p.9: the calculation only ADDS].
        let mut s = two_player_state();
        zero_resources(&mut s);
        s.players[0].victory_points = 7;
        s.players[1].victory_points = 3;
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(st.len(), 2);
        assert_eq!(total_of(&st, 0), 7);
        assert_eq!(total_of(&st, 1), 3);
        assert_eq!(rank_order(&st), vec![0, 1], "higher VP ranks first");
    }

    // --- 2. Church bonus [RB p.9 §1] ---

    #[test]
    fn church_bonus_first_6_second_3() {
        // 3-player (no dummy) isolates the generic first/second mechanic; the
        // 2-player dummy-competition case is covered by the D1 tests below.
        let mut s = three_player_state();
        zero_resources(&mut s);
        // P0: faith 4+5 (slots 0,1); P1: faith 6 (slot 2).
        s.church_track.slots[0] = Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 0 });
        s.church_track.slots[1] = Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 1 });
        s.church_track.slots[2] = Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 2 });
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 0), 6);
        assert_eq!(total_of(&st, 1), 3);
    }

    #[test]
    fn church_bonus_count_tie_rightmost_pawn_wins() {
        // RB p.9 example shape (Red 2, Blue 2, Yellow 1): tied players compare
        // pawn POSITIONS; the rightmost pawn's owner takes 6, the tie loser is
        // second with 3, the count-1 player gets nothing.
        let mut s = three_player_state();
        zero_resources(&mut s);
        let slot = |p: u8, o: u8| Some(ChurchSlot { player: PlayerId::new(p), arrival_order: o });
        s.church_track.slots[0] = slot(2, 0); // Yellow: 1 pawn, leftmost
        s.church_track.slots[1] = slot(0, 1); // Red: slots 1,2
        s.church_track.slots[2] = slot(0, 2);
        s.church_track.slots[3] = slot(1, 3); // Blue: slots 3,4 — rightmost
        s.church_track.slots[4] = slot(1, 4);
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 1), 6, "rightmost pawn among the tied pair");
        assert_eq!(total_of(&st, 0), 3, "tie loser takes second place");
        assert_eq!(total_of(&st, 2), 0, "count-1 player is third: nothing");
    }

    #[test]
    fn church_bonus_positional_not_arrival_order() {
        // Independence probe: a pawn placed LATER on a LEFT space must not
        // beat an earlier pawn on a RIGHT space — RB p.9 compares positions.
        // 3-player (no dummy) keeps this a clean two-real-competitor race.
        let mut s = three_player_state();
        zero_resources(&mut s);
        // P0 placed first at faith 9 (slot 5); P1 placed later at faith 4 (slot 0).
        s.church_track.slots[5] = Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 0 });
        s.church_track.slots[0] = Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 1 });
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 0), 6, "positional rightmost wins despite earlier arrival");
        assert_eq!(total_of(&st, 1), 3);
    }

    #[test]
    fn church_bonus_sole_player_no_second_place() {
        // "If there is only 1 player on the board, that player earns 6 points
        // and no other player receives points" [RB p.9]. Tested at 3 players so
        // the player is GENUINELY sole — at 2P the setup dummy always competes
        // (a lone real player then only takes second; see the D1 tests).
        let mut s = three_player_state();
        zero_resources(&mut s);
        s.church_track.slots[3] = Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 0 });
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 0), 6);
        assert_eq!(total_of(&st, 1), 0, "zero-count player never takes second");
    }

    #[test]
    fn church_bonus_empty_board_scores_nothing() {
        // Genuinely empty: 3 players (no dummy) and no pawns ⇒ nobody scores.
        let mut s = three_player_state();
        zero_resources(&mut s);
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 0), 0);
        assert_eq!(total_of(&st, 1), 0);
    }

    // --- 3. Customs bonus [RB p.9 §2] ---

    #[test]
    fn customs_bonus_first_8_second_4_same_printed_cost_resolves_by_slot() {
        // Spaces 1 and 2 share printed cost 1 but are DISTINCT physical
        // spaces; rightmost is slot position, so the space-2 pawn wins the
        // count tie [RB p.9 §2 + board: 7 spaces left-to-right]. 3-player (no
        // dummy) keeps the comparison between the two real pawns only.
        let mut s = three_player_state();
        zero_resources(&mut s);
        s.customs_track.slots[0] = Some(CustomsSlot { player: PlayerId::new(0), arrival_order: 0 });
        s.customs_track.slots[1] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 1 });
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 1), 8);
        assert_eq!(total_of(&st, 0), 4);
    }

    #[test]
    fn customs_bonus_counts_beat_positions() {
        let mut s = two_player_state();
        zero_resources(&mut s);
        // P0 has 2 pawns (leftmost spaces), P1 has 1 pawn on the rightmost
        // space: count decides FIRST, positions only break count ties. This is
        // a 2P board, so the D1 dummy also competes (one pawn at space-4) — but
        // P0's count-2 wins first outright and P1's space-7 pawn is rightmost of
        // the dummy's space-4, so the dummy is cleanly beaten and the real
        // awards stand (8 / 4).
        s.customs_track.slots[0] = Some(CustomsSlot { player: PlayerId::new(0), arrival_order: 0 });
        s.customs_track.slots[1] = Some(CustomsSlot { player: PlayerId::new(0), arrival_order: 1 });
        s.customs_track.slots[6] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 2 });
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 0), 8);
        assert_eq!(total_of(&st, 1), 4);
    }

    // --- 2P setup dummy competes in the majority bonuses [RB p.9 NOTE, via
    //     guidingpath.md decision log 2026-07-21 finding D1] ---
    //
    // "The dummy player's assistants are taken into account during final
    // scoring to increase competition for majority bonuses" [RB p.9 NOTE]. The
    // dummy holds ONE pawn at church faith-7 and ONE at customs space-4 (the
    // spaces it pre-blocks — KB §Customs, DL 2026-07-17), modeled BY RULE from
    // the 2-player game — never read from a slot. A place it takes is awarded
    // to nobody; real players score the rank they reach with the dummy in the
    // race. These fixtures leave the dummy's slot EMPTY on purpose: the oracle
    // competes the dummy because `player_count == 2`, not because a pawn sits
    // there.

    #[test]
    fn church_2p_dummy_takes_second_and_denies_a_real_player() {
        // P0 has the most pawns (first, 6). The count-1 race for second is the
        // dummy (faith-7) vs P1 (faith-4): the dummy's rightmost pawn wins, so
        // it TAKES second (nobody scores it) and P1 drops to nothing.
        let mut s = two_player_state();
        zero_resources(&mut s);
        church_pawn(&mut s, 8, 0);
        church_pawn(&mut s, 9, 0); // P0: two pawns, most
        church_pawn(&mut s, 4, 1); // P1: one pawn, left of the dummy
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 0), 6, "P0 has the most pawns: first place");
        assert_eq!(total_of(&st, 1), 0, "the dummy at faith-7 takes second, denying P1");
    }

    #[test]
    fn church_2p_dummy_wins_a_rightmost_tie_for_first() {
        // All competitors hold one pawn; the dummy's faith-7 is rightmost of P0
        // (faith-4) and P1 (faith-5). The dummy TAKES first (nobody scores 6);
        // P1 is second (3), P0 third (0).
        let mut s = two_player_state();
        zero_resources(&mut s);
        church_pawn(&mut s, 4, 0);
        church_pawn(&mut s, 5, 1);
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 1), 3, "P1 (faith-5) is second behind the dummy");
        assert_eq!(total_of(&st, 0), 0, "P0 (faith-4) is third: nothing");
    }

    #[test]
    fn church_2p_dummy_beaten_cleanly_leaves_real_awards_intact() {
        // P0 has the most pawns (first, 6); P1's single pawn at faith-8 is to
        // the RIGHT of the dummy's faith-7, so P1 beats the dummy for second
        // (3) and the dummy scores nobody — the real awards are unchanged.
        let mut s = two_player_state();
        zero_resources(&mut s);
        church_pawn(&mut s, 10, 0);
        church_pawn(&mut s, 11, 0); // P0: two pawns, most
        church_pawn(&mut s, 8, 1); // P1: one pawn, right of the dummy
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 0), 6);
        assert_eq!(total_of(&st, 1), 3, "faith-8 beats the dummy's faith-7 for second");
    }

    #[test]
    fn church_2p_lone_real_player_only_takes_second_behind_the_dummy() {
        // A single real player with a pawn LEFT of faith-7 is NOT sole at 2P —
        // the dummy competes and, being rightmost, takes first. The lone real
        // player gets only second (3), not the 6 a truly-sole player would.
        let mut s = two_player_state();
        zero_resources(&mut s);
        church_pawn(&mut s, 4, 0);
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 0), 3, "the dummy denies the lone player first place");
        assert_eq!(total_of(&st, 1), 0);
    }

    #[test]
    fn church_2p_dummy_alone_awards_nobody() {
        // No real pawns: the dummy is the sole competitor, takes first, and —
        // being the dummy — that place is awarded to nobody. No real player
        // scores from the church board.
        let mut s = two_player_state();
        zero_resources(&mut s);
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 0), 0);
        assert_eq!(total_of(&st, 1), 0);
    }

    #[test]
    fn church_2p_dummy_is_rule_derived_not_read_from_the_slot() {
        // Independence: the oracle competes the dummy because the game is
        // 2-player, NOT by reading a sentinel from the faith-7 slot. Whether
        // that slot is empty or carries the engine's `PlayerId::DUMMY`, the
        // awards are identical — the sentinel is filtered by the real-player
        // `seat < n` scan and the by-rule dummy competes either way.
        let awards = |fill_sentinel: bool| {
            let mut s = two_player_state();
            zero_resources(&mut s);
            church_pawn(&mut s, 4, 1); // P1 alone at faith-4
            if fill_sentinel {
                let idx = yokohama_core::ChurchTrack::faith_to_index(7).unwrap();
                s.church_track.slots[idx] =
                    Some(ChurchSlot { player: PlayerId::DUMMY, arrival_order: 9 });
            }
            let st = final_standings(&s, &Rules::empty_for_tests());
            (total_of(&st, 0), total_of(&st, 1))
        };
        // Dummy (faith-7) first → nobody; P1 (faith-4) second → 3.
        assert_eq!(awards(false), (0, 3));
        assert_eq!(awards(true), (0, 3), "same result whether or not the sentinel is present");
    }

    #[test]
    fn customs_2p_dummy_takes_a_place() {
        // Customs mirror (8/4). P0 has the most pawns (first, 8); the count-1
        // race for second is the dummy (space-4) vs P1 (space-1) — the dummy's
        // rightmost pawn wins and takes second, denying P1.
        let mut s = two_player_state();
        zero_resources(&mut s);
        customs_pawn(&mut s, 5, 0);
        customs_pawn(&mut s, 6, 0); // P0: two pawns, most
        customs_pawn(&mut s, 1, 1); // P1: one pawn, left of the dummy
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 0), 8);
        assert_eq!(total_of(&st, 1), 0, "the dummy at space-4 takes second, denying P1");
    }

    #[test]
    fn customs_2p_dummy_wins_a_rightmost_tie() {
        // Count-1 all round: the dummy's space-4 is rightmost of P0 (space-1)
        // and P1 (space-3). Dummy first (nobody), P1 second (4), P0 nothing.
        let mut s = two_player_state();
        zero_resources(&mut s);
        customs_pawn(&mut s, 1, 0);
        customs_pawn(&mut s, 3, 1);
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 1), 4, "P1 (space-3) is second behind the dummy");
        assert_eq!(total_of(&st, 0), 0);
    }

    #[test]
    fn three_player_church_and_customs_have_no_dummy() {
        // [DL 2026-07-21 D1] the dummy exists ONLY at 2 players. At 3P the
        // faith-7 and space-4 spaces are ordinary board spaces — a pawn there
        // scores for its owner, and no phantom competitor exists.
        let mut s = three_player_state();
        zero_resources(&mut s);
        // Church: P2 occupies faith-7 (legal at 3P) and, as rightmost, takes
        // first — proving faith-7 is a real space, not a dummy block.
        church_pawn(&mut s, 4, 0);
        church_pawn(&mut s, 5, 1);
        church_pawn(&mut s, 7, 2);
        // Customs: P2 occupies space-4 (legal at 3P) and takes first likewise.
        customs_pawn(&mut s, 1, 0);
        customs_pawn(&mut s, 3, 1);
        customs_pawn(&mut s, 4, 2);
        let st = final_standings(&s, &Rules::empty_for_tests());
        // P2 first on both boards (6 + 8), P1 second (3 + 4), P0 nothing.
        assert_eq!(total_of(&st, 2), 6 + 8);
        assert_eq!(total_of(&st, 1), 3 + 4);
        assert_eq!(total_of(&st, 0), 0);
    }

    #[test]
    fn four_player_customs_has_no_dummy() {
        // The no-dummy branch is `player_count != 2`; 4P takes it too, so
        // space-4 is an ordinary space with no phantom competitor.
        let mut s = four_player_state();
        zero_resources(&mut s);
        customs_pawn(&mut s, 1, 0);
        customs_pawn(&mut s, 4, 1); // space-4 is ordinary at 4P
        let st = final_standings(&s, &Rules::empty_for_tests());
        // Both count-1: P1 (space-4, rightmost) first (8), P0 (space-1) second (4).
        assert_eq!(total_of(&st, 1), 8);
        assert_eq!(total_of(&st, 0), 4);
    }

    // --- 4. Technology bonus [RB p.9-10 §3] ---

    /// Find a tech title that exists as two physical copies (equal printed
    /// industry, different ids) — 10 of the 20 titles do (KB ownership rule).
    fn twin_techs(rules: &Rules) -> (yokohama_core::TechCardId, yokohama_core::TechCardId) {
        let all: Vec<_> = rules.techs.iter().collect();
        for a in &all {
            for b in &all {
                if a.id != b.id && a.name == b.name {
                    assert_eq!(a.industry_cost, b.industry_cost);
                    return (a.id, b.id);
                }
            }
        }
        panic!("standard data carries twin tech copies");
    }

    #[test]
    fn technology_bonus_highest_10_second_5_others_0() {
        let rules = Rules::standard();
        let mut techs: Vec<_> = rules.techs.iter().collect();
        techs.sort_by_key(|t| t.id.0);
        // Give P0 two cards and P1 one low card such that P0's total is higher.
        let hi = techs.iter().max_by_key(|t| t.industry_cost).unwrap();
        let lo = techs.iter().min_by_key(|t| t.industry_cost).unwrap();
        assert!(hi.industry_cost > lo.industry_cost, "data has distinct industry values");
        let mut s = three_player_state();
        zero_resources(&mut s);
        s.players[0].technologies = vec![hi.id];
        s.players[1].technologies = vec![lo.id];
        // P2 owns none: excluded entirely.
        let st = final_standings(&s, &rules);
        assert_eq!(total_of(&st, 0), 10);
        assert_eq!(total_of(&st, 1), 5);
        assert_eq!(total_of(&st, 2), 0);
    }

    #[test]
    fn technology_bonus_tie_breaks_clockwise_from_start_player() {
        // RB p.9: "the player who is the closest clockwise to the Start
        // player breaks ties (The Start player himself is considered to be
        // the closest)". Twin copies give an exact industry tie.
        let rules = Rules::standard();
        let (a, b) = twin_techs(&rules);
        let mut s = two_player_state();
        zero_resources(&mut s);
        s.players[0].technologies = vec![a];
        s.players[1].technologies = vec![b];
        s.config.start_player = PlayerId::new(1);
        let st = final_standings(&s, &rules);
        assert_eq!(total_of(&st, 1), 10, "start player is closest to himself");
        assert_eq!(total_of(&st, 0), 5);
    }

    #[test]
    fn technology_bonus_sole_owner_no_second_place() {
        // "If there is only 1 player with Technology cards, that player earns
        // 10 points and no other player receives points" [RB p.9-10].
        let rules = Rules::standard();
        let any = rules.techs.iter().next().expect("standard data has techs");
        let mut s = two_player_state();
        zero_resources(&mut s);
        s.players[0].technologies = vec![any.id];
        let st = final_standings(&s, &rules);
        assert_eq!(total_of(&st, 0), 10);
        assert_eq!(total_of(&st, 1), 0, "tech-less player never takes second");
    }

    // --- 5. Country bonus [RB p.10 §4] ---

    #[test]
    fn country_score_matches_rulebook_example() {
        // RB p.10 example: 3 English, 2 American, 2 French, 1 German, 1 Dutch
        // → {E,A,F,G,D}=12, {E,A,F}=4, {E}=0 → 16.
        assert_eq!(country_score_from_counts(&[3, 2, 2, 1, 1]), 16);
    }

    #[test]
    fn country_score_table_values() {
        assert_eq!(country_score_from_counts(&[0, 0, 0, 0, 0]), 0);
        assert_eq!(country_score_from_counts(&[1, 0, 0, 0, 0]), 0, "a single icon scores 0");
        assert_eq!(country_score_from_counts(&[1, 1, 0, 0, 0]), 2);
        assert_eq!(country_score_from_counts(&[1, 1, 1, 0, 0]), 4);
        assert_eq!(country_score_from_counts(&[1, 1, 1, 1, 0]), 8);
        assert_eq!(country_score_from_counts(&[1, 1, 1, 1, 1]), 12);
        // Two same icons cannot share a set: 2 of one country alone = 0.
        assert_eq!(country_score_from_counts(&[2, 0, 0, 0, 0]), 0);
        // 4 distinct beats 2+2: [2,2,1,1,0] → {4}=8 + {2}=2 = 10, not 2+2+2.
        assert_eq!(country_score_from_counts(&[2, 2, 1, 1, 0]), 10);
    }

    #[test]
    fn country_score_greedy_is_optimal_brute_force() {
        // Adversarial check of the largest-sets-first (conjugate) division:
        // brute-force the best partition for every count vector ≤ 7 per icon
        // (Stage-2 review widening — counts > 3 are reachable at game end,
        // e.g. GB order cards + 2-copy GB tech titles; ≤ 7 covers every
        // plausible table and 8^5 memoized vectors stay fast; beyond 7 the
        // set-value convexity argument in the module docs applies).
        fn set_value(size: u32) -> u32 {
            match size {
                5 => 12,
                4 => 8,
                3 => 4,
                2 => 2,
                _ => 0,
            }
        }
        fn brute(counts: [u32; 5], memo: &mut HashMap<[u32; 5], u32>) -> u32 {
            if let Some(&v) = memo.get(&counts) {
                return v;
            }
            let mut best = 0;
            for mask in 1u32..32 {
                let mut c = counts;
                let mut size = 0;
                let mut ok = true;
                for (i, ci) in c.iter_mut().enumerate() {
                    if mask & (1 << i) != 0 {
                        if *ci == 0 {
                            ok = false;
                            break;
                        }
                        *ci -= 1;
                        size += 1;
                    }
                }
                if ok {
                    best = best.max(set_value(size) + brute(c, memo));
                }
            }
            memo.insert(counts, best);
            best
        }
        let mut memo = HashMap::new();
        for a in 0..8u32 {
            for b in 0..8u32 {
                for c in 0..8u32 {
                    for d in 0..8u32 {
                        for e in 0..8u32 {
                            let counts = [a, b, c, d, e];
                            assert_eq!(
                                country_score_from_counts(&counts),
                                brute(counts, &mut memo),
                                "counts {counts:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn country_bonus_combines_completed_orders_and_techs_only() {
        // One completed Order + one owned Tech of DIFFERENT countries form a
        // set of 2 (+2 VP); orders still in hand return to the box and score
        // nothing [RB p.10 §4: "return all unfulfilled Order cards"].
        let rules = Rules::standard();
        let tech = rules.techs.iter().next().expect("standard data has techs");
        let order = rules
            .orders
            .iter()
            .find(|o| o.country != tech.country)
            .expect("an order of a different country exists");
        let unfulfilled = rules
            .orders
            .iter()
            .find(|o| o.id != order.id)
            .expect("a second order exists");
        let mut s = two_player_state();
        zero_resources(&mut s);
        s.players[0].technologies = vec![tech.id];
        s.players[0].completed_orders = vec![order.id];
        s.players[0].order_cards_in_hand = vec![unfulfilled.id];
        let st = final_standings(&s, &rules);
        // P0: sole tech owner (+10) + country set of 2 (+2) = 12.
        assert_eq!(total_of(&st, 0), 12);
        assert_eq!(total_of(&st, 1), 0);
    }

    // --- 6. Remainder scoring [RB p.10 §5] ---

    #[test]
    fn remainder_scoring_each_conversion() {
        let mut s = two_player_state();
        zero_resources(&mut s);
        let p = &mut s.players[0];
        p.money = 5; // every 2 Yen → 2 VP
        p.copper = 2;
        p.silk = 2;
        p.tea = 2;
        p.seafood = 1; // 7 trade goods, any combination → 2 VP
        p.imported_goods = 3; // 1 VP each → 3 VP
        p.mercenaries.available.insert(yokohama_core::Country::English, 2); // face-up → 2 VP
        p.mercenaries.used.insert(yokohama_core::Country::French, 1); // used: nothing
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(total_of(&st, 0), 2 + 2 + 3 + 2);
        assert_eq!(total_of(&st, 1), 0);
    }

    // --- Combined case ---

    #[test]
    fn combined_components_sum() {
        let rules = Rules::standard();
        let tech = rules.techs.iter().next().expect("standard data has techs");
        let order = rules
            .orders
            .iter()
            .find(|o| o.country != tech.country)
            .expect("an order of a different country exists");
        // 3-player (no dummy) so the church/customs components are the clean
        // sole-competitor bonuses this "all components sum" case intends.
        let mut s = three_player_state();
        zero_resources(&mut s);
        s.players[0].victory_points = 5; // banked
        s.church_track.slots[2] = Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 0 }); // +6 sole
        s.customs_track.slots[4] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 }); // P1 +8 sole
        s.players[0].technologies = vec![tech.id]; // sole tech owner +10
        s.players[0].completed_orders = vec![order.id]; // country set of 2 → +2
        s.players[0].money = 4; // +2
        s.players[0].imported_goods = 1; // +1
        let st = final_standings(&s, &rules);
        assert_eq!(total_of(&st, 0), 5 + 6 + 10 + 2 + 2 + 1);
        assert_eq!(total_of(&st, 1), 8);
    }

    // --- Ranking + official tie-break [RB p.10 Winning; DL 2026-07-19] ---

    #[test]
    fn final_tie_two_way_resolves_clockwise_from_start_player() {
        let mut s = two_player_state();
        zero_resources(&mut s);
        // Equal totals (all zero). Start player 1 → clockwise order 1, 0.
        s.config.start_player = PlayerId::new(1);
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(rank_order(&st), vec![1, 0]);
        // Start player 0 → order 0, 1.
        s.config.start_player = PlayerId::new(0);
        let st = final_standings(&s, &Rules::empty_for_tests());
        assert_eq!(rank_order(&st), vec![0, 1]);
    }

    #[test]
    fn final_tie_three_way_resolves_clockwise_from_start_player() {
        let mut s = three_player_state();
        zero_resources(&mut s);
        s.config.start_player = PlayerId::new(2);
        let st = final_standings(&s, &Rules::empty_for_tests());
        // Clockwise from seat 2: 2, 0, 1 — a strict total order despite the
        // three-way VP tie (every player ranked, no shared ranks).
        assert_eq!(rank_order(&st), vec![2, 0, 1]);
        assert_eq!(st.len(), 3);
    }

    #[test]
    fn standings_are_a_strict_total_order_and_deterministic() {
        // Mixed case: P0 and P2 tied on VP, P1 higher; start player 1.
        let mut s = three_player_state();
        zero_resources(&mut s);
        s.players[0].victory_points = 4;
        s.players[1].victory_points = 9;
        s.players[2].victory_points = 4;
        s.config.start_player = PlayerId::new(1);
        let st = final_standings(&s, &Rules::empty_for_tests());
        // P1 first on VP; P2 beats P0 (clockwise from 1: 1, 2, 0).
        assert_eq!(rank_order(&st), vec![1, 2, 0]);
        // Every player appears exactly once (strict total order).
        let mut ids = rank_order(&st);
        ids.sort_unstable();
        assert_eq!(ids, vec![0, 1, 2]);
        // Deterministic: a second evaluation is identical.
        assert_eq!(st, final_standings(&s, &Rules::empty_for_tests()));
    }
}
