//! §3.3 + §4 President movement — lane B, implemented per spec §3.3/§4.
//!
//! Every rule here is authored from the rules sources only (rulebook p.6/p.14/
//! p.15–16, decision-log rows cited inline); no engine logic file was consulted.

use std::collections::{BTreeMap, BTreeSet};

use yokohama_core::{
    AreaId, GameState, Move, PlayerId, PresidentLocation, Rules, TechEffect,
};

use crate::common::{opponent_presidents_in, owns_effect};
use crate::RouteLabel;

/// §1.4 Movement dispatch: the exact part (`PlacePresident` destinations when
/// the President is in hand, plus the retreat family always) and the on-board
/// route-label part (empty when the President is in hand).
///
/// Movement never dead-ends: Option B (the retreat family, which contains at
/// least the empty-map form) is always performable, President in hand or on
/// board [RB p.6; DL 2026-07-17 "Movement Option B is legal with the President
/// already in hand"].
pub(crate) fn movement_moves(state: &GameState, rules: &Rules) -> (Vec<Move>, Vec<RouteLabel>) {
    match state.current_player().president_location {
        PresidentLocation::InHand => {
            let mut exact = place_president_moves(state, rules);
            exact.extend(retreat_family(state));
            (exact, Vec::new())
        }
        PresidentLocation::OnBoard(_) => (retreat_family(state), route_labels(state, rules)),
    }
}

/// §3.3-2 retreat family: `ReturnPresidentToHand` for every per-area recovery
/// count map (each key `1..=in_area`), the empty map included [RB p.6 "you may
/// also return any Assistant pawns from any Area to your hand"; any-number-per-
/// area encoding DL 2026-07-19 §8 veto row I5]. Management-board assistants
/// (church/customs/achievements) are structurally excluded — they are not in
/// `on_board` [RB p.6 "You may not return Assistant pawns placed on Management
/// Boards"]. No fees of any kind. Family size = Π (count + 1) over
/// own-assistant areas, materialized (brute force, §3.3-2).
pub(crate) fn retreat_family(state: &GameState) -> Vec<Move> {
    let p = state.current_player();
    let mut maps: Vec<BTreeMap<AreaId, u8>> = vec![BTreeMap::new()];
    for (&area, &count) in p.assistants.on_board.iter().filter(|(_, &c)| c >= 1) {
        let mut next = Vec::with_capacity(maps.len() * (count as usize + 1));
        for m in &maps {
            for k in 0..=count {
                let mut m2 = m.clone();
                if k > 0 {
                    // Areas with 0 recovered are omitted from the map (§3.3-2).
                    m2.insert(area, k);
                }
                next.push(m2);
            }
        }
        maps = next;
    }
    maps.into_iter().map(|recover| Move::ReturnPresidentToHand { recover }).collect()
}

/// §3.3-1 `PlacePresident` destinations (President in hand only): every
/// Possible Action Area — ≥1 own assistant AND no other player's President
/// [RB p.6]. With Postal System, additionally every own-assistant area holding
/// opponent president(s) at 1 Yen each, waived entirely by also owning
/// Stagecoach [RB p.15; from-hand placement explicitly included, DL 2026-07-17
/// I2]. The own-assistant requirement is never lifted — Postal System modifies
/// only the president clause. Affordability-gated.
///
/// From-hand Tram does NOT exist (board-only, owner-ruled [DL 2026-07-19]);
/// from-hand Station is unruled — conservative board-only reading (§7.1-A1).
pub(crate) fn place_president_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    let player = state.current_player;
    let p = state.current_player();
    if p.president_location != PresidentLocation::InHand {
        return Vec::new();
    }
    let postal = owns_effect(state, rules, player, TechEffect::PostalSystem);
    let stagecoach = owns_effect(state, rules, player, TechEffect::Stagecoach);
    let mut out = Vec::new();
    for (&area, &count) in &p.assistants.on_board {
        if count == 0 {
            continue;
        }
        let opponents = opponent_presidents_in(state, player, area).len() as u32;
        let legal = if opponents == 0 {
            true
        } else if postal {
            // Destination fee: 1 Yen per opponent president, waived by also
            // owning Stagecoach [RB p.15 Postal System].
            let fee = if stagecoach { 0 } else { opponents };
            fee <= p.money
        } else {
            false
        };
        if legal {
            out.push(Move::PlacePresident { destination: area });
        }
    }
    out
}

/// DFS context for the §4.1 walking-route enumeration.
struct Walk<'a> {
    state: &'a GameState,
    mover: PlayerId,
    money: u32,
    postal: bool,
    stagecoach: bool,
    labels: Vec<RouteLabel>,
}

impl Walk<'_> {
    /// Extend the simple path ending at `current` by every legal step
    /// (§4.1). `visited` holds every area on the path INCLUDING the start —
    /// so the destination can never be the start [RB p.6 "may not return to
    /// the Area that it started in"] and the effective route never
    /// self-intersects [DL 2026-07-20 effective-route row]. `fees` and `spent`
    /// are the accumulated fee vector / total for the path so far.
    fn dfs(
        &mut self,
        current: AreaId,
        visited: &mut BTreeSet<AreaId>,
        fees: &mut BTreeMap<Option<PlayerId>, u32>,
        spent: u32,
    ) {
        let adjacent = self.state.board_layout.adjacent_areas(current).to_vec();
        for next in adjacent {
            if visited.contains(&next) {
                continue;
            }
            // Every stepped-into area (intermediates AND destination) must
            // hold ≥1 of the mover's own assistants [RB p.6, applied per step;
            // the p.6 example rejects the Laboratory-A route for this reason].
            // Shophouses/Trading Houses never count [RB p.6]. This also
            // excludes the Canal structurally (no pawns may ever be there).
            if self.state.player(self.mover).assistants.in_area(next) == 0 {
                continue;
            }
            // Fees charged on ENTERING `next`, stop or pass alike:
            let mut delta: Vec<(Option<PlayerId>, u32)> = Vec::new();
            // Canal crossing: 1 Yen to the BANK per traversed crossing edge,
            // including the first edge [RB p.14; DL 2026-07-18 BOX CHECK (c) +
            // I4 start-edge fix].
            if self.state.board_layout.is_canal_crossing(current, next) {
                delta.push((None, 1));
            }
            // Opponent presidents: 1 Yen to each owner — pass-through [RB p.6
            // "must immediately pay each other President pawn in the Area
            // 1 Yen and continue"] and Postal-System stops [RB p.15] price
            // identically; Stagecoach waives both [RB p.15 Stagecoach; Postal
            // "unless you also own the Stagecoach"].
            let opponents = opponent_presidents_in(self.state, self.mover, next);
            if !self.stagecoach {
                for o in &opponents {
                    delta.push((Some(*o), 1));
                }
            }
            let delta_sum: u32 = delta.iter().map(|(_, n)| n).sum();
            // Monotone affordability cutoff (§4.4): every fee is nonnegative
            // and nothing on a route earns money, so an unaffordable prefix
            // can never extend to an affordable route.
            if spent + delta_sum > self.money {
                continue;
            }
            for (k, n) in &delta {
                *fees.entry(*k).or_insert(0) += n;
            }
            visited.insert(next);
            // May stop here? No opponent president (Possible Action Area
            // [RB p.6]) — or the mover owns Postal System [RB p.15].
            if opponents.is_empty() || self.postal {
                self.labels.push(RouteLabel { destination: next, fees: fees.clone() });
            }
            // Passing an opponent president is allowed either way [RB p.6].
            self.dfs(next, visited, fees, spent + delta_sum);
            visited.remove(&next);
            for (k, n) in &delta {
                let e = fees.get_mut(k).expect("fee key just inserted");
                *e -= n;
                if *e == 0 {
                    fees.remove(k); // absent key = 0 (§1.1)
                }
            }
        }
    }
}

/// §4.1–4.2: simple-path DFS with a monotone cost cutoff, plus Tram/Station
/// teleport labels, deduplicated by the L2 `(destination, fees)` identity
/// [DL 2026-07-19 §8 veto row]. Empty unless the President is on the board.
pub(crate) fn route_labels(state: &GameState, rules: &Rules) -> Vec<RouteLabel> {
    let mover = state.current_player;
    let p = state.current_player();
    let PresidentLocation::OnBoard(start) = p.president_location else {
        return Vec::new();
    };

    let mut walk = Walk {
        state,
        mover,
        money: p.money,
        postal: owns_effect(state, rules, mover, TechEffect::PostalSystem),
        stagecoach: owns_effect(state, rules, mover, TechEffect::Stagecoach),
        labels: Vec::new(),
    };
    let mut visited = BTreeSet::from([start]);
    let mut fees = BTreeMap::new();
    walk.dfs(start, &mut visited, &mut fees, 0);
    let mut labels = walk.labels;

    // Tram (owner only): a zero-fee label for every non-Canal area containing
    // no other player's President, other than the current area; NO assistant
    // needed at the destination [RB p.16; DL 2026-07-17 I1 — Tram does NOT
    // bypass opponent-president blocking]. Board-only [DL 2026-07-19].
    if owns_effect(state, rules, mover, TechEffect::Tram) {
        for (&area, cfg) in &state.board_layout.areas {
            if area == start || !cfg.area_type.allows_placement() {
                continue;
            }
            if !opponent_presidents_in(state, mover, area).is_empty() {
                continue;
            }
            labels.push(RouteLabel { destination: area, fees: BTreeMap::new() });
        }
    }

    // Station (owner only): a zero-fee label to the Station area — free,
    // regardless of other President pawns there [RB p.15].
    if owns_effect(state, rules, mover, TechEffect::Station) {
        if let Some(st) = state.station_location {
            // PENDING-RULING §7.1-A1 (triage 2026-07-20): temporarily mirrors the engine's Reading 2 (own assistant required at the Station area) so the differential gate binds. Jeremy's ruling flips exactly this condition — if Reading 1 is ruled, delete it (and the engine changes instead).
            let own_assistant_at_station = p.assistants.on_board.get(&st).copied().unwrap_or(0) >= 1;
            if st != start && own_assistant_at_station {
                labels.push(RouteLabel { destination: st, fees: BTreeMap::new() });
            }
        }
    }

    // L2 dedup: structural equality on (destination, fees) — a free walking
    // route and a teleport to the same destination are ONE label (§4.2).
    labels.sort();
    labels.dedup();
    labels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::two_player_state;
    use std::collections::BTreeMap;
    use yokohama_core::{
        AreaConfig, AreaId, AreaType, BoardLayout, BoardLayoutBuilder, BuildingSite,
        FivePowerEffect, GameState, PlayerId, PresidentLocation, TechEffect, TurnPhase,
    };

    /// Private board helper (lane B owns no shared fixture; spec §5 lets a lane
    /// keep helpers private to its module). Crossing edges are ALSO adjacency
    /// edges — the Canal is not a node, crossings are transit-only edges
    /// between flank tiles [DL 2026-07-18/19 canal-geometry rows].
    fn board(areas: &[(u8, AreaType)], adj: &[(u8, u8)], crossings: &[(u8, u8)]) -> BoardLayout {
        let mut b = BoardLayoutBuilder::new();
        for &(id, ty) in areas {
            let aid = AreaId::new(id);
            b.add_area(aid, AreaConfig { area_type: ty, grid_position: (0, id) });
            if ty != AreaType::Canal {
                b.set_building_site(aid, BuildingSite::new([1, 1, 1, 1], 3));
                b.set_five_power_token(aid, FivePowerEffect::Money(1));
            }
        }
        for &(x, y) in adj {
            b.add_adjacency(AreaId::new(x), AreaId::new(y));
        }
        for &(x, y) in crossings {
            b.add_canal_crossing(AreaId::new(x), AreaId::new(y));
        }
        b.build().expect("test board validates")
    }

    /// F2 [RB p.6] shape: TeaPlantation(0) — EmploymentAgency(1) — SilkMill(2),
    /// with LaboratoryA(3) offering the alternative 0–3–2 path. Mover (Red, 0)
    /// has her president on 0 and assistants on 1 and 2 ONLY; Blue's president
    /// sits on the Employment Agency (1).
    fn f2_state() -> GameState {
        let mut s = two_player_state();
        s.board_layout = board(
            &[
                (0, AreaType::TeaPlantation),
                (1, AreaType::EmploymentAgency),
                (2, AreaType::SilkMill),
                (3, AreaType::LaboratoryA),
            ],
            &[(0, 1), (1, 2), (0, 3), (3, 2)],
            &[],
        );
        s.turn_phase = TurnPhase::Movement;
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        s.players[0].assistants.on_board.insert(AreaId::new(1), 1);
        s.players[0].assistants.on_board.insert(AreaId::new(2), 1);
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));
        s
    }

    fn label(dest: u8, fees: &[(Option<u8>, u32)]) -> RouteLabel {
        RouteLabel {
            destination: AreaId::new(dest),
            fees: fees.iter().map(|(p, n)| (p.map(PlayerId::new), *n)).collect(),
        }
    }

    fn tech_id(rules: &Rules, effect: TechEffect) -> yokohama_core::TechCardId {
        rules
            .techs
            .iter()
            .find(|t| t.effect == effect)
            .map(|t| t.id)
            .expect("standard data carries the tech")
    }

    // --- F2 [RB p.6] Hisashi movement example ---

    #[test]
    fn f2_pass_through_fee_blocked_stop_and_no_route_without_assistants() {
        let s = f2_state();
        let labels = route_labels(&s, &Rules::standard());
        // Pass through Blue's president on the way to Silk Mill: 1 Yen to Blue.
        assert!(labels.contains(&label(2, &[(Some(1), 1)])));
        // Without Postal System the mover may not STOP on Blue's president.
        assert!(labels.iter().all(|l| l.destination != AreaId::new(1)));
        // The p.6 example rejects the Laboratory-A route (no own assistant
        // there), so the fee-bearing route is the ONLY label to Silk Mill.
        assert_eq!(labels.iter().filter(|l| l.destination == AreaId::new(2)).count(), 1);
        // The President may never return to its starting area [RB p.6].
        assert!(labels.iter().all(|l| l.destination != AreaId::new(0)));
    }

    #[test]
    fn f2_postal_stagecoach_two_by_two_fee_matrix() {
        // [RB p.15] Postal System permits the stop (fee stays); Stagecoach
        // waives pass-through fees; owning both waives the stop fee too.
        let rules = Rules::standard();
        let postal = tech_id(&rules, TechEffect::PostalSystem);
        let stage = tech_id(&rules, TechEffect::Stagecoach);

        // Stagecoach only: pass-through free, stop still blocked.
        let mut s = f2_state();
        s.players[0].technologies.push(stage);
        let labels = route_labels(&s, &rules);
        assert!(labels.contains(&label(2, &[])));
        assert!(labels.iter().all(|l| l.destination != AreaId::new(1)));

        // Postal only: stop legal at 1 Yen to Blue; pass-through still 1 Yen.
        let mut s = f2_state();
        s.players[0].technologies.push(postal);
        let labels = route_labels(&s, &rules);
        assert!(labels.contains(&label(1, &[(Some(1), 1)])));
        assert!(labels.contains(&label(2, &[(Some(1), 1)])));

        // Both: stop AND pass-through free.
        let mut s = f2_state();
        s.players[0].technologies.extend([postal, stage]);
        let labels = route_labels(&s, &rules);
        assert!(labels.contains(&label(1, &[])));
        assert!(labels.contains(&label(2, &[])));
    }

    #[test]
    fn f2_affordability_prunes_unpayable_pass_through() {
        // Fees are mandatory payments; insufficient money forbids the route
        // (§4.1-6, monotone cutoff §4.4).
        let mut s = f2_state();
        s.players[0].money = 0;
        assert!(route_labels(&s, &Rules::standard()).is_empty());
    }

    // --- §6.2 movement rows: retreat family ---

    #[test]
    fn retreat_family_is_exact_product_set_including_empty() {
        // [RB p.6 "return any Assistant pawns"; DL 2026-07-19 §8 veto I5:
        // any-number-per-area count maps, empty map included].
        let mut s = f2_state();
        s.players[0].assistants.on_board.insert(AreaId::new(1), 2);
        // Areas 1 (count 2) and 2 (count 1) ⇒ (2+1)×(1+1) = 6 maps.
        let fam = retreat_family(&s);
        assert_eq!(fam.len(), 6);
        assert!(fam.contains(&Move::ReturnPresidentToHand { recover: BTreeMap::new() }));
        assert!(fam.contains(&Move::ReturnPresidentToHand {
            recover: BTreeMap::from([(AreaId::new(1), 2), (AreaId::new(2), 1)]),
        }));
        assert!(fam.contains(&Move::ReturnPresidentToHand {
            recover: BTreeMap::from([(AreaId::new(1), 1)]),
        }));
        // Zero-count keys are omitted, never present.
        assert!(fam.iter().all(|m| match m {
            Move::ReturnPresidentToHand { recover } => recover.values().all(|&v| v >= 1),
            _ => false,
        }));
    }

    #[test]
    fn movement_in_hand_offers_retreat_and_place_president_no_labels() {
        // Option B is legal with the President already in hand
        // [DL 2026-07-17 Option-B-from-hand ruling]; labels need OnBoard (§1.4).
        let mut s = f2_state();
        s.players[0].president_location = PresidentLocation::InHand;
        let (exact, labels) = movement_moves(&s, &Rules::standard());
        assert!(labels.is_empty());
        assert!(exact.contains(&Move::ReturnPresidentToHand { recover: BTreeMap::new() }));
        // Possible Action Area [RB p.6]: own assistant, no opponent president.
        assert!(exact.contains(&Move::PlacePresident { destination: AreaId::new(2) }));
        // Blue's president blocks 1 without Postal System.
        assert!(!exact.contains(&Move::PlacePresident { destination: AreaId::new(1) }));
        // No own assistant at 3 — never lifted, even with Postal (§3.3-1).
        assert!(!exact.contains(&Move::PlacePresident { destination: AreaId::new(3) }));
    }

    #[test]
    fn movement_on_board_exact_is_retreat_family_plus_labels() {
        let s = f2_state();
        let (exact, labels) = movement_moves(&s, &Rules::standard());
        assert!(exact.iter().all(|m| matches!(m, Move::ReturnPresidentToHand { .. })));
        assert!(exact.contains(&Move::ReturnPresidentToHand { recover: BTreeMap::new() }));
        assert!(!labels.is_empty());
    }

    #[test]
    fn place_president_postal_fee_and_stagecoach_waiver() {
        // [RB p.15 Postal System; DL 2026-07-17 I2: from-hand placement
        // explicitly included; fee waived entirely by also owning Stagecoach].
        let rules = Rules::standard();
        let postal = tech_id(&rules, TechEffect::PostalSystem);
        let stage = tech_id(&rules, TechEffect::Stagecoach);
        let mut s = f2_state();
        s.players[0].president_location = PresidentLocation::InHand;
        s.players[0].technologies.push(postal);
        s.players[0].money = 0;
        assert!(!place_president_moves(&s, &rules)
            .contains(&Move::PlacePresident { destination: AreaId::new(1) }));
        s.players[0].money = 1;
        assert!(place_president_moves(&s, &rules)
            .contains(&Move::PlacePresident { destination: AreaId::new(1) }));
        s.players[0].money = 0;
        s.players[0].technologies.push(stage);
        assert!(place_president_moves(&s, &rules)
            .contains(&Move::PlacePresident { destination: AreaId::new(1) }));
    }

    // --- F11 [RB p.14 + DL 2026-07-18 BOX CHECK (c)] Canal tolls ---

    /// Canal shape: 0—1—2 where BOTH edges are canal crossings (flank pairs).
    /// Mover's president on 0; own assistants on 1 and 2.
    fn canal_state() -> GameState {
        let mut s = two_player_state();
        s.board_layout = board(
            &[(0, AreaType::CopperMine), (1, AreaType::SilkMill), (2, AreaType::TeaPlantation)],
            &[(0, 1), (1, 2)],
            &[(0, 1), (1, 2)],
        );
        s.turn_phase = TurnPhase::Movement;
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        s.players[0].assistants.on_board.insert(AreaId::new(1), 1);
        s.players[0].assistants.on_board.insert(AreaId::new(2), 1);
        s
    }

    #[test]
    fn f11_canal_tolls_to_bank_including_first_edge_with_cutoff() {
        let rules = Rules::standard();
        let mut s = canal_state();
        s.players[0].money = 2;
        let labels = route_labels(&s, &rules);
        // 1 Yen to the BANK per crossing, counted on the FIRST edge too
        // [RB p.14; DL 2026-07-18 I4].
        assert!(labels.contains(&label(1, &[(None, 1)])));
        assert!(labels.contains(&label(2, &[(None, 2)])));
        // Monotone affordability cutoff (§4.4).
        s.players[0].money = 1;
        let labels = route_labels(&s, &rules);
        assert!(labels.contains(&label(1, &[(None, 1)])));
        assert!(!labels.iter().any(|l| l.destination == AreaId::new(2)));
        s.players[0].money = 0;
        assert!(route_labels(&s, &rules).is_empty());
    }

    // --- F12 [RB p.15] Station teleport ---

    #[test]
    fn f12_station_teleport_presidents_ignored_but_assistant_required_pending_ruling() {
        // PENDING-RULING §7.1-A1 (triage 2026-07-20): interim mirrors the
        // engine's Reading 2 — the teleport label is emitted ONLY with >= 1
        // own assistant at the Station area. All other facets unchanged:
        // owner-only, zero fees, opponent presidents there neither block nor
        // price it, st ≠ current area.
        let rules = Rules::standard();
        let mut s = f2_state();
        s.station_location = Some(AreaId::new(3)); // no own assistant at 3
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(3));
        // Non-owner: no label to 3 (walking needs an own assistant there).
        let labels = route_labels(&s, &rules);
        assert!(labels.iter().all(|l| l.destination != AreaId::new(3)));
        // Owner, ZERO own assistants at the Station area: label ABSENT.
        s.players[0].technologies.push(tech_id(&rules, TechEffect::Station));
        let labels = route_labels(&s, &rules);
        assert!(
            labels.iter().all(|l| l.destination != AreaId::new(3)),
            "interim Reading 2: no own assistant at the Station area suppresses the label"
        );
        // Owner, ONE own assistant there: the free teleport label appears —
        // the opponent President at 3 neither blocks nor prices it (zero
        // fees; the walking route to 3 would carry a president fee).
        s.players[0].assistants.on_board.insert(AreaId::new(3), 1);
        let labels = route_labels(&s, &rules);
        assert!(labels.contains(&label(3, &[])));
        // Station at the mover's current area yields no label (st ≠ current),
        // even with an own assistant there.
        s.players[0].assistants.on_board.insert(AreaId::new(0), 1);
        s.station_location = Some(AreaId::new(0));
        let labels = route_labels(&s, &rules);
        assert!(labels.iter().all(|l| l.destination != AreaId::new(0)));
    }

    // --- §6.2 movement rows: Tram ---

    #[test]
    fn tram_labels_president_free_non_canal_areas_without_assistants() {
        // [RB p.16; DL 2026-07-17 I1: every president-free, non-Canal area
        // other than the current one; NO destination assistant needed].
        let rules = Rules::standard();
        let mut s = two_player_state();
        s.board_layout = board(
            &[
                (0, AreaType::CopperMine),
                (1, AreaType::SilkMill),
                (2, AreaType::TeaPlantation),
                (4, AreaType::Canal),
            ],
            &[(0, 1), (1, 2), (1, 4)],
            &[],
        );
        s.turn_phase = TurnPhase::Movement;
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(2));
        s.players[0].technologies.push(tech_id(&rules, TechEffect::Tram));
        let labels = route_labels(&s, &rules);
        assert!(labels.contains(&label(1, &[])), "no assistant needed at a Tram destination");
        assert!(
            labels.iter().all(|l| l.destination != AreaId::new(2)),
            "Tram does not bypass opponent-president blocking"
        );
        assert!(labels.iter().all(|l| l.destination != AreaId::new(4)), "never the Canal");
        assert!(labels.iter().all(|l| l.destination != AreaId::new(0)), "never the current area");
    }

    // --- §6.2 movement rows: label dedup + income denial ---

    /// Diamond: 0–1–3 and 0–2–3; president on 0, own assistants on 1, 2, 3.
    fn diamond_state() -> GameState {
        let mut s = two_player_state();
        s.board_layout = board(
            &[
                (0, AreaType::CopperMine),
                (1, AreaType::SilkMill),
                (2, AreaType::TeaPlantation),
                (3, AreaType::FishingGround),
            ],
            &[(0, 1), (0, 2), (1, 3), (2, 3)],
            &[],
        );
        s.turn_phase = TurnPhase::Movement;
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        for a in [1u8, 2, 3] {
            s.players[0].assistants.on_board.insert(AreaId::new(a), 1);
        }
        s
    }

    #[test]
    fn same_fee_routes_dedup_to_one_label() {
        // Two free routes to 3 (via 1 and via 2) are ONE label
        // [DL 2026-07-19 L2 row: identity is (destination, fee-vector)].
        let s = diamond_state();
        let labels = route_labels(&s, &Rules::standard());
        assert_eq!(labels.iter().filter(|l| l.destination == AreaId::new(3)).count(), 1);
        assert!(labels.contains(&label(3, &[])));
    }

    #[test]
    fn income_denial_routes_stay_distinct_labels() {
        // [DL 2026-07-19 §8 veto row]: routing to deny an opponent income
        // changes the credit vector, hence the label.
        let mut s = diamond_state();
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));
        let labels = route_labels(&s, &Rules::standard());
        let to3: Vec<_> = labels.iter().filter(|l| l.destination == AreaId::new(3)).collect();
        assert_eq!(to3.len(), 2);
        assert!(labels.contains(&label(3, &[])));
        assert!(labels.contains(&label(3, &[(Some(1), 1)])));
        assert!(labels.iter().all(|l| l.destination != AreaId::new(1)));
        // At 0 Yen only the free route survives (affordability §4.1-6).
        s.players[0].money = 0;
        let labels = route_labels(&s, &Rules::standard());
        assert_eq!(labels.iter().filter(|l| l.destination == AreaId::new(3)).count(), 1);
        assert!(labels.contains(&label(3, &[])));
    }
}
