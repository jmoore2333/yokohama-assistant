//! PRUNELESS president-route REFERENCE enumerator (Phase 2.9, Stage 3).
//!
//! Purpose: the engine's route DFS and the Stage-1 oracle's route DFS
//! (`movement::route_labels`) BOTH prune on accumulated-cost-exceeds-money,
//! justified by the same monotonicity argument (spec §4.4: every route fee is
//! a nonnegative Yen amount and nothing on a route earns money, so cost is
//! nondecreasing along a path). That shared assumption has been reasoned about
//! but never tested against an implementation that does not make it. This
//! module is that implementation: it enumerates EVERY simple path over the
//! own-assistant step rule (spec §4.1) with NO affordability cutoff anywhere
//! in the walk — the walker does not even receive `money` — prices each
//! COMPLETED path after enumeration, filters `cost <= money` only at the END,
//! and dedups to the L2 `(destination, fees)` labels.
//!
//! DELIBERATELY SHARED: teleport labels (Tram / Station, including the
//! `§7.1-A1` gate, RULED Reading 2 by Jeremy 2026-07-20) are appended by REUSING the pruned
//! module's [`movement::teleport_labels`]. Teleports have no walk and no fees,
//! so sharing them makes the three-way diff surface (engine DFS vs pruned
//! oracle DFS vs this enumerator) exactly the WALKS.
//!
//! NOT wired into [`crate::oracle_moves`]: this is a reference surface for the
//! Stage-3 harness, not part of the Stage-1 contract (plan, Stage 3 section).

use std::collections::{BTreeMap, BTreeSet};

use yokohama_core::{AreaId, GameState, PlayerId, PresidentLocation, Rules, TechEffect};

use crate::common::{allows_pawn, opponent_presidents_in, owns_effect};
use crate::{movement, RouteLabel};

/// Every legal `(destination, fees)` movement label for the current player,
/// computed WITHOUT any affordability pruning during the walk (module header).
///
/// Empty unless the President is `OnBoard` (labels are an on-board family,
/// spec §1.4/§4). Semantics per spec §4.1: every stepped-into area holds ≥1
/// own assistant; destination ≠ start; destination president-free unless
/// Postal System; pass-through president fees 1 Yen to each owner (Stagecoach
/// waives all president fees, pass-through and Postal stop alike); canal
/// crossings 1 Yen to the BANK per traversed crossing edge, first edge
/// included. Affordability (§4.1-6) is applied as a FINAL filter over fully
/// priced paths, never inside the walk.
pub fn route_labels_pruneless(state: &GameState, rules: &Rules) -> Vec<RouteLabel> {
    let mover = state.current_player;
    let p = state.current_player();
    let PresidentLocation::OnBoard(start) = p.president_location else {
        return Vec::new();
    };
    let postal = owns_effect(state, rules, mover, TechEffect::PostalSystem);
    let stagecoach = owns_effect(state, rules, mover, TechEffect::Stagecoach);

    // Phase 1 — enumerate + price the COMPLETE candidate space (no money in
    // scope; see `walk_label_candidates`).
    let mut labels = walk_label_candidates(state, mover, start, postal, stagecoach);

    // Phase 2 — the ONLY affordability check in this module: filter fully
    // priced paths at the END (§4.1-6: fees are mandatory payments).
    labels.retain(|l| l.mover_cost() <= p.money);

    // Teleports: reused from the pruned module BY DESIGN (module header). No
    // affordability filter applies — teleport labels are zero-fee.
    labels.extend(movement::teleport_labels(state, rules));

    labels.sort();
    labels.dedup();
    labels
}

/// Phase 1+2 of the walk surface: enumerate the COMPLETE simple-path candidate
/// space from `start` and price each completed path. `money` is deliberately
/// absent from this signature — an affordability cutoff inside the walk is
/// structurally impossible, which is the whole point of this module.
fn walk_label_candidates(
    state: &GameState,
    mover: PlayerId,
    start: AreaId,
    postal: bool,
    stagecoach: bool,
) -> Vec<RouteLabel> {
    enumerate_simple_paths(state, mover, start, postal)
        .into_iter()
        .map(|path| RouteLabel {
            destination: *path.last().expect("recorded paths are non-empty"),
            fees: price_path(state, mover, start, &path, stagecoach),
        })
        .collect()
}

/// Every simple path `start → … → dest` (recorded WITHOUT `start`; non-empty)
/// over `board_layout` adjacency such that (spec §4.1):
///
/// 1. every stepped-into area — intermediates AND destination — holds ≥1 of
///    the mover's own assistants [RB p.6, applied per step; Shophouses/Trading
///    Houses never count] AND is not the Canal, which is unplaceable and
///    unstoppable [RB p.14; KB §Canal] — a separate rule, checked separately;
/// 2. the destination holds no opponent president, OR the mover owns Postal
///    System [RB p.6 / p.15] — a prefix ending on an opponent president is
///    still EXTENDED (passing is allowed [RB p.6]), just not recorded;
/// 3. the destination is never `start` and the path never self-intersects —
///    `visited` seeds with `start` [RB p.6; DL 2026-07-20 effective-route row].
///
/// No fees are computed here and no cutoff of ANY kind is applied: this is
/// pure graph reachability over the own-assistant step rule.
fn enumerate_simple_paths(
    state: &GameState,
    mover: PlayerId,
    start: AreaId,
    postal: bool,
) -> Vec<Vec<AreaId>> {
    let mut out = Vec::new();
    let mut path = Vec::new();
    let mut visited = BTreeSet::from([start]);
    extend_paths(state, mover, postal, start, &mut visited, &mut path, &mut out);
    out
}

/// Recursive step of [`enumerate_simple_paths`].
fn extend_paths(
    state: &GameState,
    mover: PlayerId,
    postal: bool,
    current: AreaId,
    visited: &mut BTreeSet<AreaId>,
    path: &mut Vec<AreaId>,
    out: &mut Vec<Vec<AreaId>>,
) {
    for next in state.board_layout.adjacent_areas(current).to_vec() {
        if visited.contains(&next) {
            continue;
        }
        if state.player(mover).assistants.in_area(next) == 0 {
            continue;
        }
        // The Canal is never a tile a pawn stops on or walks over [RB p.14 "no
        // pawns or tokens may be placed"; KB §Canal "unplaceable and
        // unstoppable (transit-only)"] — a rule about the tile, so it holds in
        // the pruneless reference exactly as in the pruned walk.
        if !allows_pawn(state, next) {
            continue;
        }
        visited.insert(next);
        path.push(next);
        if postal || opponent_presidents_in(state, mover, next).is_empty() {
            out.push(path.clone());
        }
        extend_paths(state, mover, postal, next, visited, path, out);
        path.pop();
        visited.remove(&next);
    }
}

/// Price one COMPLETED path (spec §4.1-3/4/5): walking its edges from `start`,
/// entering each area charges — stop or pass alike —
///
/// - 1 Yen to the BANK per traversed canal-crossing edge, the FIRST edge
///   included [RB p.14; DL 2026-07-18 BOX CHECK (c) + I4 start-edge fix];
/// - 1 Yen to EACH opponent-president owner in the entered area (pass-through
///   [RB p.6] and Postal-System destination stops [RB p.15] price
///   identically), all waived by Stagecoach [RB p.15].
fn price_path(
    state: &GameState,
    mover: PlayerId,
    start: AreaId,
    path: &[AreaId],
    stagecoach: bool,
) -> BTreeMap<Option<PlayerId>, u32> {
    let mut fees: BTreeMap<Option<PlayerId>, u32> = BTreeMap::new();
    let mut prev = start;
    for &step in path {
        if state.board_layout.is_canal_crossing(prev, step) {
            *fees.entry(None).or_insert(0) += 1;
        }
        if !stagecoach {
            for owner in opponent_presidents_in(state, mover, step) {
                *fees.entry(Some(owner)).or_insert(0) += 1;
            }
        }
        prev = step;
    }
    fees
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::two_player_state;
    use yokohama_core::{
        AreaConfig, AreaType, BoardLayout, BoardLayoutBuilder, BuildingSite, FivePowerEffect,
        TurnPhase,
    };

    /// Lane-private board helper (spec §5 lets a lane keep helpers private to
    /// its module). Crossing edges are ALSO adjacency edges — the Canal is not
    /// a node [DL 2026-07-18/19 canal-geometry rows].
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

    /// Diamond 0–1–3 / 0–2–3 with Blue's president on 1: the same geometry
    /// yields both zero-cost labels (via 2) and fee-bearing labels (via 1).
    fn diamond_blue_at_1() -> GameState {
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
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));
        s
    }

    /// 0—1—2 where BOTH edges are canal crossings: every walk label carries a
    /// bank toll, so the fee-bearing candidate space is nonempty while the
    /// zero-cost label space is empty.
    fn canal_only() -> GameState {
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

    /// Canal + opponent president in one geometry: 0–1–2–3 (Blue's president
    /// on 1) plus the alternative 0–4–3 whose first edge (0,4) is a canal
    /// crossing. Own assistants on 1, 2, 3, 4.
    fn canal_president() -> GameState {
        let mut s = two_player_state();
        s.board_layout = board(
            &[
                (0, AreaType::CopperMine),
                (1, AreaType::SilkMill),
                (2, AreaType::TeaPlantation),
                (3, AreaType::FishingGround),
                (4, AreaType::Bank),
            ],
            &[(0, 1), (1, 2), (2, 3), (0, 4), (4, 3)],
            &[(0, 4)],
        );
        s.turn_phase = TurnPhase::Movement;
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        for a in [1u8, 2, 3, 4] {
            s.players[0].assistants.on_board.insert(AreaId::new(a), 1);
        }
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));
        s
    }

    // --- Structural-difference guard (task 2) ---

    #[test]
    fn walker_enumerates_fee_bearing_candidates_and_filters_only_at_the_end() {
        // The walker's signature carries NO money — the candidate space is the
        // same whatever the mover's purse. Candidates include every
        // fee-bearing path; affordability bites only in the final filter.
        let rules = Rules::standard();
        let mut s = canal_only();
        let cands =
            walk_label_candidates(&s, PlayerId::new(0), AreaId::new(0), false, false);
        assert!(cands.contains(&label(1, &[(None, 1)])), "first-edge toll candidate enumerated");
        assert!(cands.contains(&label(2, &[(None, 2)])), "two-toll candidate enumerated");
        // End filter: 0 Yen keeps nothing, 1 Yen keeps the 1-toll label only,
        // 2 Yen keeps both.
        s.players[0].money = 0;
        assert!(route_labels_pruneless(&s, &rules).is_empty());
        s.players[0].money = 1;
        assert_eq!(route_labels_pruneless(&s, &rules), vec![label(1, &[(None, 1)])]);
        s.players[0].money = 2;
        let both = route_labels_pruneless(&s, &rules);
        assert!(both.contains(&label(1, &[(None, 1)])));
        assert!(both.contains(&label(2, &[(None, 2)])));
    }

    #[test]
    fn zero_money_yields_exactly_the_zero_cost_labels_rich_strictly_more() {
        // Same geometry, purse-only difference: money = 0 keeps exactly the
        // zero-cost labels; a rich purse keeps a strict superset.
        let rules = Rules::standard();
        let mut s = diamond_blue_at_1();
        s.players[0].money = 0;
        let zero = route_labels_pruneless(&s, &rules);
        assert_eq!(zero, vec![label(2, &[]), label(3, &[])]);
        s.players[0].money = 10;
        let rich = route_labels_pruneless(&s, &rules);
        assert!(zero.iter().all(|l| rich.contains(l)), "rich set contains the zero-cost labels");
        assert!(rich.len() > zero.len(), "rich set is strictly larger");
        assert!(rich.contains(&label(2, &[(Some(1), 1)])), "income-denial route via Blue");
        assert!(rich.contains(&label(3, &[(Some(1), 1)])));
        // Never a stop on Blue's president (no Postal), never the start.
        assert!(rich.iter().all(|l| l.destination != AreaId::new(1)));
        assert!(rich.iter().all(|l| l.destination != AreaId::new(0)));
    }

    // --- In-crate self-differential vs the PRUNED oracle (task 2) ---

    #[test]
    fn agrees_with_pruned_oracle_on_canal_president_stagecoach_postal_2x2() {
        let rules = Rules::standard();
        let postal = tech_id(&rules, TechEffect::PostalSystem);
        let stage = tech_id(&rules, TechEffect::Stagecoach);
        for use_postal in [false, true] {
            for use_stage in [false, true] {
                for money in [0u32, 1, 2, 3, 10] {
                    let mut s = canal_president();
                    if use_postal {
                        s.players[0].technologies.push(postal);
                    }
                    if use_stage {
                        s.players[0].technologies.push(stage);
                    }
                    s.players[0].money = money;
                    let pruned = movement::route_labels(&s, &rules);
                    let pruneless = route_labels_pruneless(&s, &rules);
                    assert_eq!(
                        pruneless, pruned,
                        "postal={use_postal} stagecoach={use_stage} money={money}"
                    );
                    if money == 10 {
                        assert!(!pruned.is_empty(), "rich cell must not be vacuously equal");
                    }
                }
            }
        }
    }

    #[test]
    fn shared_teleports_flow_through_identically() {
        // Teleports are REUSED from the pruned module (module header): both
        // surfaces must agree with Tram + Station live, and the Station
        // §7.1-A1 assistant gate (RULED Reading 2, 2026-07-20) is shared by construction.
        let rules = Rules::standard();
        let mut s = canal_president();
        s.players[0].money = 3;
        s.players[0].technologies.push(tech_id(&rules, TechEffect::Tram));
        s.players[0].technologies.push(tech_id(&rules, TechEffect::Station));
        s.station_location = Some(AreaId::new(2)); // own assistant there — gate passes
        let pruned = movement::route_labels(&s, &rules);
        let pruneless = route_labels_pruneless(&s, &rules);
        assert_eq!(pruneless, pruned);
        assert!(pruneless.contains(&label(2, &[])), "zero-fee teleport label present");

        // Station-only, 0 Yen: the teleport to Blue-occupied area 1 (own
        // assistant present) is the ONLY way to reach 1 — presidents neither
        // block nor price a Station move [RB p.15].
        let mut s = canal_president();
        s.players[0].money = 0;
        s.players[0].technologies.push(tech_id(&rules, TechEffect::Station));
        s.station_location = Some(AreaId::new(1));
        let pruned = movement::route_labels(&s, &rules);
        let pruneless = route_labels_pruneless(&s, &rules);
        assert_eq!(pruneless, pruned);
        assert!(pruneless.contains(&label(1, &[])));
    }

    #[test]
    fn pruneless_walk_also_refuses_a_canal_step_and_still_agrees_with_the_pruned_oracle() {
        // The Canal is unplaceable AND unstoppable — transit-only [RB p.14 "no
        // pawns or tokens may be placed"; KB 01-rules-reference §Canal; DL
        // 2026-07-21 any-to-any re-ruling, which re-states this]. The rule is
        // about the TILE, not about affordability or pruning, so the pruneless
        // reference must reach the same verdict as the pruned walk — otherwise
        // the three-way check compares two enumerators that disagree on a rule.
        // Fixture: 0 — 1(Canal) — 2 with the mover's own assistants (illegally,
        // for the sake of the test) on both 1 and 2, so the own-assistant step
        // rule [RB p.6] cannot be what suppresses the routes.
        let rules = Rules::standard();
        let mut s = canal_only();
        s.board_layout = board(
            &[(0, AreaType::CopperMine), (1, AreaType::Canal), (2, AreaType::SilkMill)],
            &[(0, 1), (1, 2)],
            &[],
        );
        s.players[0].money = 10;
        let pruneless = route_labels_pruneless(&s, &rules);
        assert!(pruneless.is_empty(), "no route runs onto or through the Canal [RB p.14]");
        assert_eq!(pruneless, movement::route_labels(&s, &rules));

        // Anti-vacuity: the identical geometry with a normal middle tile yields
        // both labels, in BOTH enumerators.
        let mut control = s.clone();
        control.board_layout = board(
            &[(0, AreaType::CopperMine), (1, AreaType::TeaPlantation), (2, AreaType::SilkMill)],
            &[(0, 1), (1, 2)],
            &[],
        );
        let pruneless = route_labels_pruneless(&control, &rules);
        assert!(pruneless.contains(&label(1, &[])));
        assert!(pruneless.contains(&label(2, &[])));
        assert_eq!(pruneless, movement::route_labels(&control, &rules));
    }

    #[test]
    fn in_hand_president_yields_no_labels() {
        let mut s = canal_president();
        s.players[0].president_location = PresidentLocation::InHand;
        assert!(route_labels_pruneless(&s, &Rules::standard()).is_empty());
    }
}
