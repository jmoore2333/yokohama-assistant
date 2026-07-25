//! President movement and pathfinding.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use crate::types::{AreaId, GameState, PlayerId, PresidentLocation, TechEffect};
use super::cards::TechRegistry;

/// Fee vector for one Movement-step path: `None` is the bank (canal tolls),
/// `Some(owner)` is that opponent's President owner (pass-through/
/// destination fees). Task 6 (L2 route enumeration, design 2026-07-19
/// §4.3): the engine used to track only the path's TOTAL cost as a `u32` -
/// this type is the single source of WHO gets paid, so
/// `enumerate_president_routes` can tell two equal-total-cost paths apart
/// when they pay different recipients (the owner's ruling: deliberately
/// paying MORE to deny an opponent a fee is real, enumerable play).
pub type RouteVector = BTreeMap<Option<PlayerId>, u32>;

/// Check if an area is a valid destination (Possible Action Area) — a real
/// board area that is not the Canal, holds >=1 of the player's own assistants,
/// and is not blocked by an opponent President.
/// Without TechRegistry, assumes no Postal System.
pub fn is_possible_action_area(state: &GameState, player: PlayerId, area: AreaId) -> bool {
    is_possible_action_area_full(state, player, area, None, false)
}

/// Check if an area is a valid destination with technology awareness.
pub fn is_possible_action_area_with_tech(
    state: &GameState,
    player: PlayerId,
    area: AreaId,
    tech_registry: Option<&TechRegistry>,
) -> bool {
    is_possible_action_area_full(state, player, area, tech_registry, false)
}

/// Full version with all options.
/// - is_mercenary: If true, opponent presidents don't block.
///
/// THE stop-legality predicate: every consumer that decides whether a President
/// may STOP somewhere calls it rather than restating the test (the from-hand
/// `PlacePresident` arm, both route searches' destination filters,
/// `route_enum::RouteGraph`, and `move_legality`), which is why the Canal
/// exclusion below needed to be stated only here to reach all of them
/// (2026-07-25 alignment, `SEMANTICS_REV` 4).
pub fn is_possible_action_area_full(
    state: &GameState,
    player: PlayerId,
    area: AreaId,
    tech_registry: Option<&TechRegistry>,
    is_mercenary: bool,
) -> bool {
    let p = state.player(player);

    // Must be a real board area, and never the Canal: no pawn may ever be
    // placed there (rulebook p.14; guidingpath 2026-07-21 Canal re-ruling),
    // which is exactly what `require_placeable_area` enforces on the apply
    // side. Stated BEFORE the mercenary early-return below, so a mercenary
    // target inherits it too — no in-repo caller passes `is_mercenary = true`
    // today (mercenary sub-turns read `state.mercenary_target_area` instead),
    // so that leg is future-proofing an argument-reachable path rather than a
    // fix to a live one.
    if !state.board_layout.areas.get(&area).is_some_and(|c| c.area_type.allows_placement()) {
        return false;
    }

    // Must have at least 1 assistant
    if p.assistants.in_area(area) < 1 {
        return false;
    }

    // Mercenaries ignore opponent president blocking
    if is_mercenary {
        return true;
    }

    // Check for Postal System technology
    let has_postal = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::PostalSystem).unwrap_or(false)
        })
    });

    // Check for other Presidents (unless has Postal System)
    if !has_postal {
        for (i, other_player) in state.players.iter().enumerate() {
            if i != player.0 as usize
                && other_player.president_location == PresidentLocation::OnBoard(area)
            {
                return false;
            }
        }
    }

    true
}

/// Find all areas reachable from current position via assistant chain.
/// Returns list of (destination, path_to_reach). Without TechRegistry,
/// assumes no Postal System.
pub fn find_reachable_areas(
    state: &GameState,
    player: PlayerId,
    from: AreaId,
) -> Vec<(AreaId, Vec<AreaId>)> {
    find_reachable_areas_with_tech(state, player, from, None)
}

/// Find all areas reachable from current position via assistant chain, with
/// technology awareness (Postal System lets the destination check tolerate
/// an opponent President already there). Returns one (destination, path)
/// pair per reachable destination: the MINIMUM-TOTAL-COST path into it
/// (canal tolls + opponent-president pass-through/destination fees --
/// `calculate_movement_cost_full`'s exact weighting), not merely the fewest
/// hops (I3 final-review fix: a canal crossing is always hop-count-shortest,
/// so a plain BFS would toll a destination even when a longer, toll-free
/// route through the mover's own assistants also reaches it).
///
/// A Dijkstra over the assistant-presence graph: nodes are `from` plus
/// every area holding >=1 of the mover's own assistants (a path may only
/// travel through -- and may stop at -- areas holding the mover's own
/// assistant, same rule the old BFS enforced, so `from` itself never
/// re-enters as a waypoint). The weight of stepping onto area `b` from `a`
/// is the canal toll on the (a,b) crossing plus 1 Yen per opponent
/// President occupying `b` (waived by Stagecoach) -- summing these along a
/// path reproduces `calculate_movement_cost_full`'s total exactly, since
/// that function's "intermediate" and "destination" loops together price
/// every area in `path` once, and its canal-toll loop scans the same
/// `[from] + path` window sequence this Dijkstra's edges do.
///
/// Ties are broken by lexicographically smallest path (`Vec<AreaId>`'s
/// derived `Ord`), so enumeration stays fully deterministic. This is sound
/// with plain lazy-deletion Dijkstra (no separate tie-break pass needed):
/// extending a path by one hop with a non-negative-weight edge always
/// produces a STRICTLY greater `(cost, path)` key than the path it
/// extends -- if the edge costs > 0, the cost component alone decides it;
/// if it costs 0, the cost ties but a proper prefix always sorts before
/// the longer vector it's a prefix of (`vec![a] < vec![a, b]`). So keys
/// only ever increase along any walk, meaning the first successful pop for
/// any area is already that area's global-optimum (cost, path) pair.
///
/// Task 6 (L2 route enumeration, design 2026-07-19 §4.3): this is a
/// single-MINIMUM-cost view of the reachability graph. President Movement
/// legal-move generation no longer calls this directly -- it uses
/// `enumerate_president_routes`, which returns every distinct affordable
/// (destination, fee vector) label, not just the cheapest path per
/// destination. This function has had NO caller outside this module's own
/// unit tests ever since (2026-07-25 re-check: the "mercenary paths" consumer
/// this line used to name never existed -- mercenary sub-turns resolve their
/// acting area from `state.mercenary_target_area`). It is kept as the
/// cheapest-route view for any future consumer that genuinely wants one, and
/// stays honest by construction: its hop and destination gates are the same
/// two shared predicates every other search calls.
pub fn find_reachable_areas_with_tech(
    state: &GameState,
    player: PlayerId,
    from: AreaId,
    tech_registry: Option<&TechRegistry>,
) -> Vec<(AreaId, Vec<AreaId>)> {
    let p = state.player(player);

    let has_stagecoach = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Stagecoach).unwrap_or(false)
        })
    });

    // Mirrors calculate_movement_cost_full's per-area pricing for stepping
    // onto `to` from `from_area`: canal tolls are never waived; the
    // opponent-president fee is waived entirely by Stagecoach.
    let edge_cost = |from_area: AreaId, to: AreaId| -> u32 {
        let mut cost = 0;
        if state.board_layout.is_canal_crossing(from_area, to) {
            cost += 1;
        }
        if !has_stagecoach {
            cost += state.players.iter()
                .filter(|o| o.id != player && o.president_location == PresidentLocation::OnBoard(to))
                .count() as u32;
        }
        cost
    };

    let mut best: BTreeMap<AreaId, (u32, Vec<AreaId>)> = BTreeMap::new();
    let mut heap: BinaryHeap<Reverse<(u32, Vec<AreaId>, AreaId)>> = BinaryHeap::new();

    for &adj in state.board_layout.adjacent_areas(from) {
        // Can only move through areas with our assistants
        if can_step_onto(state, player, from, adj) {
            heap.push(Reverse((edge_cost(from, adj), vec![adj], adj)));
        }
    }

    while let Some(Reverse((cost, path, area))) = heap.pop() {
        if best.contains_key(&area) {
            continue; // a cheaper-or-equal entry already finalized this area (see doc comment)
        }
        best.insert(area, (cost, path.clone()));

        // Continue exploring if we have assistants here
        if p.assistants.in_area(area) > 0 {
            for &adj in state.board_layout.adjacent_areas(area) {
                if !can_step_onto(state, player, from, adj) {
                    continue;
                }
                let mut new_path = path.clone();
                new_path.push(adj);
                heap.push(Reverse((cost + edge_cost(area, adj), new_path, adj)));
            }
        }
    }

    best.into_iter()
        .filter(|&(area, _)| is_possible_action_area_with_tech(state, player, area, tech_registry))
        .map(|(area, (_, path))| (area, path))
        .collect()
}

/// Whether a president-movement step may land on `to`, given the move's
/// start area `start`: `to` must be a real board area that is not the Canal,
/// must hold >=1 of the mover's own assistants (the assistant-chain
/// requirement), and must not be `start` itself.
///
/// The Canal clause (2026-07-25 alignment, `SEMANTICS_REV` 4) is the rule that
/// no pawn may ever be placed there (rulebook p.14; guidingpath 2026-07-21
/// Canal re-ruling) — the Canal is transit-only in the sense that its
/// CROSSINGS join two other areas, never that a pawn stands on the Canal tile
/// itself. Stating it here mirrors `validate_president_path`'s per-hop
/// `require_placeable_area` on the apply side.
///
/// That start-exclusion is CANONICALIZATION, not a rules claim. The
/// 2026-07-20 effective-route ruling (guidingpath decision log) is that
/// retracing isn't real movement: the only prohibited revisit is ENDING on
/// the start area, and any declared route that re-enters `start` mid-move
/// reduces to a shorter simple effective route with the same endpoint and
/// the same fees. Excluding `start` from every hop therefore enumerates one
/// representative per effective route rather than narrowing the legal set.
///
/// This is THE hop-legality predicate — every consumer that decides whether a
/// President may step somewhere calls it rather than restating the test, so no
/// two of them can diverge on what counts as a legal hop. That currently means
/// the two frozen searches (`find_reachable_areas_with_tech`'s Dijkstra, one
/// path per destination, and `enumerate_president_routes`' DFS, every simple
/// affordable path), the 3.1b lazy enumerator (`route_enum::RouteGraph::build`,
/// which precomputes it per area), and the single-candidate predicate
/// (`move_legality`). The class is what the guarantee is stated over; a fifth
/// consumer joins it by calling this function, not by being listed here.
pub(crate) fn can_step_onto(state: &GameState, player: PlayerId, start: AreaId, to: AreaId) -> bool {
    to != start
        && state.board_layout.areas.get(&to).is_some_and(|c| c.area_type.allows_placement())
        && state.player(player).assistants.in_area(to) > 0
}

/// Check if player can teleport to Station area (Station technology).
/// Rulebook p.15: "You may move your President pawn to this area during
/// the Movement Step for free... regardless of other President pawns."
pub fn can_teleport_to_station(
    state: &GameState,
    player: PlayerId,
) -> Option<AreaId> {
    can_teleport_to_station_with_tech(state, player, None)
}

/// Check if player can teleport to Station area with tech awareness.
pub fn can_teleport_to_station_with_tech(
    state: &GameState,
    player: PlayerId,
    tech_registry: Option<&TechRegistry>,
) -> Option<AreaId> {
    let p = state.player(player);

    // Must have Station technology
    let has_station = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Station).unwrap_or(false)
        })
    });

    if !has_station {
        return None;
    }

    // Must have a station placed
    let station_area = state.station_location?;

    // Station area must have our assistant — OWNER-RULED (Jeremy,
    // 2026-07-20 decision-log row, closing ruling A1): the station token
    // adds POWER only, like a shophouse/trading house; RB p.6's
    // assistant-at-destination rule is NOT overridden. The card's explicit
    // clauses stand: the jump is pathless, free, and ignores other
    // President pawns AT the station. Was an assumed reading since
    // inception; adjudicated after the Phase-2.9 differential surfaced it.
    if p.assistants.in_area(station_area) > 0 {
        Some(station_area)
    } else {
        None
    }
}

/// Calculate movement cost (opponent Presidents and canal tolls).
/// Without TechRegistry, assumes no Stagecoach.
pub fn calculate_movement_cost(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
) -> u32 {
    calculate_movement_cost_full(state, player, path, None, false)
}

/// Calculate movement cost with technology awareness.
pub fn calculate_movement_cost_with_tech(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
    tech_registry: Option<&TechRegistry>,
) -> u32 {
    calculate_movement_cost_full(state, player, path, tech_registry, false)
}

/// Calculate movement cost for mercenary action.
/// Mercenaries don't pay for passing through opponent presidents.
pub fn calculate_movement_cost_mercenary(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
) -> u32 {
    calculate_movement_cost_full(state, player, path, None, true)
}

/// Fee vector for moving along `path` -- the ONE place that prices a
/// Movement-step path (Task 6, L2 route enumeration, design 2026-07-19
/// §4.3). Ports what was previously `calculate_movement_cost_full`'s
/// scalar-accumulation logic unchanged, just writing into a `RouteVector`
/// instead of a running `u32`:
/// - intermediate president pass-through fees (every area in `path` except
///   the last, 1 Yen per opponent President owner occupying it),
/// - the Postal System destination stop fee (the last area in `path`, same
///   1-Yen-per-owner pricing -- vacuous without Postal System since such a
///   destination is otherwise unreachable),
/// - canal tolls over the FULL route, including the start -> path[0] edge
///   (`p.president_location` prepended, since a toll applies to every
///   crossing in the move, not just crossings strictly within `path`),
/// - Stagecoach waives president fees entirely (both bullets above) but
///   never tolls; `is_mercenary` also waives president fees (mercenaries
///   bypass blocking and fees) but likewise never tolls.
///
/// `calculate_movement_cost_full` is now just this vector's sum, and
/// `credit_movement_president_fees` (apply_move.rs) consumes it directly
/// instead of re-deriving who's owed what -- one pricing source end to
/// end, so pricing and crediting can never drift apart.
pub fn route_fee_vector(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
    tech_registry: Option<&TechRegistry>,
    is_mercenary: bool,
) -> RouteVector {
    let mut route: RouteVector = BTreeMap::new();
    let p = state.player(player);

    // Mercenaries don't pay for opponent presidents
    let skip_president_cost = is_mercenary;

    // Check for Stagecoach technology
    let has_stagecoach = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Stagecoach).unwrap_or(false)
        })
    });

    // Check each area in path (except destination)
    let intermediate = if path.len() > 1 { &path[..path.len() - 1] } else { &[] };

    for &area in intermediate {
        // Pay for opponent Presidents (unless has Stagecoach or is mercenary)
        if !has_stagecoach && !skip_president_cost {
            for other_player in &state.players {
                if other_player.id != player
                    && other_player.president_location == PresidentLocation::OnBoard(area)
                {
                    *route.entry(Some(other_player.id)).or_insert(0) += 1;
                }
            }
        }
    }

    // Destination stop fee (Postal System, rulebook p.15: "You may end your
    // movement in an Area occupied by another President pawn... You must
    // still pay these players (unless you also own the Stagecoach)").
    // Without Postal System such a destination is never reachable in the
    // first place, so this is vacuous for everyone else. A non-empty path's
    // last area is where the President ends this move; an EMPTY path is a
    // free teleport (Station/Tram) and is exempt - Station's card text is
    // unconditionally "for free... regardless of other President pawns",
    // and Tram's destination is guaranteed president-free by construction.
    if !has_stagecoach && !skip_president_cost {
        if let Some(&destination) = path.last() {
            for other_player in &state.players {
                if other_player.id != player
                    && other_player.president_location == PresidentLocation::OnBoard(destination)
                {
                    *route.entry(Some(other_player.id)).or_insert(0) += 1;
                }
            }
        }
    }

    // Check for canal crossings (mercenaries still pay canal tolls). The
    // toll applies to EVERY crossing in the move, including the very first
    // step out of the President's current area, so the scan must start
    // from wherever the President currently is, not just within `path`
    // (regression fix, I4 final-review: `path.windows(2)` alone missed the
    // start -> path[0] edge). From hand (Movement Option B places, never
    // traverses), there is no "current area" to prepend.
    let full_route: Vec<AreaId> = match p.president_location {
        PresidentLocation::OnBoard(start) => std::iter::once(start).chain(path.iter().copied()).collect(),
        PresidentLocation::InHand => path.to_vec(),
    };
    for window in full_route.windows(2) {
        if state.board_layout.is_canal_crossing(window[0], window[1]) {
            *route.entry(None).or_insert(0) += 1;
        }
    }

    route
}

/// Full movement cost calculation with all options.
/// - is_mercenary: If true, no cost for opponent presidents.
pub fn calculate_movement_cost_full(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
    tech_registry: Option<&TechRegistry>,
    is_mercenary: bool,
) -> u32 {
    route_fee_vector(state, player, path, tech_registry, is_mercenary).values().sum()
}

/// Key for `enumerate_president_routes`'s dedup map: destination area + fee
/// vector for the path that reaches it. Deduping and sorting on this exact
/// pair is what turns "one path per destination" into "one path per
/// (destination, who's paid what)".
type RouteLabel = (AreaId, RouteVector);

/// Enumerate every DISTINCT (destination, fee vector) reachable from `from`
/// over SIMPLE paths through the mover's own assistants, affordability-
/// pruned against `state.player(player).money`. Owner's L2 ruling (design
/// 2026-07-19 §4.3): unlike `find_reachable_areas_with_tech`'s single
/// minimum-cost path per destination, a deliberately pricier route that
/// pays a DIFFERENT fee recipient -- or simply pays more -- is real,
/// enumerable play (income-denial routing), so every affordable label must
/// survive, not just the cheapest.
///
/// One entry per distinct (destination, `RouteVector`) pair: when several
/// simple paths tie on the SAME pair, the lexicographically-least path
/// (`Vec<AreaId>`'s derived `Ord`) is kept. The whole result is emitted
/// sorted by (destination, vector) -- both guaranteed by collecting into a
/// single `BTreeMap<RouteLabel, Vec<AreaId>>` keyed exactly that way (a
/// later insert only replaces the stored path when the new one sorts
/// smaller), whose in-order iteration IS that sort order.
///
/// DFS, not Dijkstra: `find_reachable_areas_with_tech` only ever needs the
/// cheapest path per destination, so lazy-deletion Dijkstra suffices --
/// here every affordable label must survive, so the search must actually
/// visit every simple path. Bounded in practice: paths only traverse the
/// mover's own-assistant areas (a small subgraph -- the whole board is at
/// most 18 areas, and any one player's own on-board assistants are a
/// fraction of that) and monotonic-cost pruning (no edge ever has
/// negative weight, so once a prefix is unaffordable every extension of it
/// is too) stops exploration the moment a prefix exceeds the player's
/// money; the release fuzz gate measures the real wall-time impact (this
/// task's report has the before/after).
pub fn enumerate_president_routes(
    state: &GameState,
    player: PlayerId,
    from: AreaId,
    tech_registry: Option<&TechRegistry>,
) -> Vec<(AreaId, Vec<AreaId>)> {
    let dfs = RouteDfs { state, player, from, tech_registry, money: state.player(player).money };

    let mut best: BTreeMap<RouteLabel, Vec<AreaId>> = BTreeMap::new();
    let mut path: Vec<AreaId> = Vec::new();
    let mut visited: BTreeSet<AreaId> = BTreeSet::new();

    dfs.step(from, &mut path, &mut visited, &mut best);

    best.into_iter().map(|((dest, _vector), path)| (dest, path)).collect()
}

/// DFS context for `enumerate_president_routes`, bundled into a struct so
/// the recursive walk below doesn't thread half a dozen loose parameters
/// through every call.
struct RouteDfs<'a> {
    state: &'a GameState,
    player: PlayerId,
    from: AreaId,
    tech_registry: Option<&'a TechRegistry>,
    money: u32,
}

impl RouteDfs<'_> {
    /// Extend `path` by one legal hop from `at` in every possible way,
    /// recording each newly-reached area as a destination (when it's a
    /// legal Possible Action Area) and recursing -- backtracking
    /// `path`/`visited` before trying the next hop, so every SIMPLE path
    /// through the graph gets its own turn. Continues exploring PAST an
    /// area even when that area isn't itself a legal destination (an
    /// opponent-blocked area can still be passed through, same as
    /// ordinary movement) as long as the path remains affordable.
    fn step(
        &self,
        at: AreaId,
        path: &mut Vec<AreaId>,
        visited: &mut BTreeSet<AreaId>,
        best: &mut BTreeMap<RouteLabel, Vec<AreaId>>,
    ) {
        for &adj in self.state.board_layout.adjacent_areas(at) {
            if visited.contains(&adj) || !can_step_onto(self.state, self.player, self.from, adj) {
                continue;
            }
            path.push(adj);
            let vector = route_fee_vector(self.state, self.player, path, self.tech_registry, false);
            let sum: u32 = vector.values().sum();
            if sum <= self.money {
                if is_possible_action_area_with_tech(self.state, self.player, adj, self.tech_registry) {
                    let key = (adj, vector);
                    let keep_new =
                        best.get(&key).map(|existing| path.as_slice() < existing.as_slice()).unwrap_or(true);
                    if keep_new {
                        best.insert(key, path.clone());
                    }
                }
                visited.insert(adj);
                self.step(adj, path, visited, best);
                visited.remove(&adj);
            }
            path.pop();
        }
    }
}

/// Fee for placing the President from hand directly into `destination`
/// (Movement Option A) - 1 Yen per opponent President already there,
/// waived by Stagecoach (I2 final-review fix). Such a destination is only
/// reachable at all with Postal System, whose card text is explicit: "You
/// must still pay these players (unless you also own the Stagecoach)."
/// Shared by the legal-move generator (pricing gate) and
/// `apply_place_president` (the charge), so they can never drift apart.
pub fn calculate_place_president_fee(
    state: &GameState,
    player: PlayerId,
    destination: AreaId,
    tech_registry: Option<&TechRegistry>,
) -> u32 {
    let p = state.player(player);
    let has_stagecoach = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Stagecoach).unwrap_or(false)
        })
    });
    if has_stagecoach {
        return 0;
    }
    state.players.iter()
        .filter(|o| o.id != player && o.president_location == PresidentLocation::OnBoard(destination))
        .count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use crate::game::apply_move::apply_move;
    use crate::game::legal_moves::get_legal_moves;
    use crate::game::rules::Rules;
    use std::collections::BTreeMap;

    fn create_test_state_with_board() -> GameState {
        let config = GameConfig::new(
            2,
            vec![PlayerColor::Red, PlayerColor::Blue],
            PlayerId::new(0),
        );

        // Create a simple 3-area board: 0 -- 1 -- 2
        let mut areas = BTreeMap::new();
        areas.insert(AreaId::new(0), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(AreaId::new(1), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });
        areas.insert(AreaId::new(2), AreaConfig { area_type: AreaType::SilkMill, grid_position: (0, 2) });

        let mut adjacency = BTreeMap::new();
        adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        adjacency.insert(AreaId::new(1), vec![AreaId::new(0), AreaId::new(2)]);
        adjacency.insert(AreaId::new(2), vec![AreaId::new(1)]);

        let board_layout = BoardLayout {
            areas,
            adjacency,
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
            turn_phase: TurnPhase::Movement,
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

    #[test]
    fn test_is_possible_action_area() {
        let mut state = create_test_state_with_board();
        let player = PlayerId::new(0);
        let area = AreaId::new(1);

        // No assistant -> not valid
        assert!(!is_possible_action_area(&state, player, area));

        // With assistant -> valid
        state.players[0].assistants.on_board.insert(area, 1);
        assert!(is_possible_action_area(&state, player, area));

        // With opponent president -> not valid (no Postal System)
        state.players[1].president_location = PresidentLocation::OnBoard(area);
        assert!(!is_possible_action_area(&state, player, area));
    }

    /// The Canal is never placeable and never stoppable (rulebook p.14;
    /// guidingpath 2026-07-21 Canal re-ruling). Both SHARED predicates carry
    /// that exclusion, so every consumer of theirs — the from-hand
    /// `PlacePresident` arm, the DFS/Dijkstra hop gates, `RouteGraph`, and
    /// `move_legality` — inherits it by construction rather than by restating
    /// it. The mercenary leg is the one that needs pinning on purpose: the
    /// `is_mercenary` early-return short-circuits everything after it, so the
    /// area-type gate has to sit BEFORE it or mercenary targets would keep the
    /// hole. (No in-repo caller passes `is_mercenary = true` today — mercenary
    /// sub-turns resolve their acting area from `state.mercenary_target_area` —
    /// so this leg is future-proofing an argument-reachable path, not a fix to
    /// a live one.)
    #[test]
    fn canal_typed_areas_are_never_action_areas_nor_hops() {
        let mut state = create_test_state_with_board();
        let player = PlayerId::new(0);
        let area = AreaId::new(1);
        state.players[0].assistants.on_board.insert(area, 1);

        assert!(is_possible_action_area(&state, player, area), "control: a normal area");
        assert!(can_step_onto(&state, player, AreaId::new(0), area), "control: a normal hop");
        assert!(
            is_possible_action_area_full(&state, player, area, None, true),
            "control: a normal mercenary target",
        );

        state.board_layout.areas.get_mut(&area).expect("area 1").area_type = AreaType::Canal;

        assert!(!is_possible_action_area(&state, player, area), "the Canal is never stoppable");
        assert!(
            !can_step_onto(&state, player, AreaId::new(0), area),
            "the Canal is never a hop — no pawn may ever stand there",
        );
        assert!(
            !is_possible_action_area_full(&state, player, area, None, true),
            "the is_mercenary early-return must not skip the Canal exclusion",
        );
    }

    #[test]
    fn test_find_reachable_areas() {
        let mut state = create_test_state_with_board();
        let player = PlayerId::new(0);

        // Place assistants on areas 1 and 2
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);
        state.players[0].assistants.on_board.insert(AreaId::new(2), 1);

        let reachable = find_reachable_areas(&state, player, AreaId::new(0));

        // Should be able to reach area 1 and 2
        assert_eq!(reachable.len(), 2);

        let destinations: Vec<AreaId> = reachable.iter().map(|(d, _)| *d).collect();
        assert!(destinations.contains(&AreaId::new(1)));
        assert!(destinations.contains(&AreaId::new(2)));
    }

    #[test]
    fn test_movement_cost_canal() {
        let mut state = create_test_state_with_board();
        state.board_layout.canal_crossings.push((AreaId::new(0), AreaId::new(1)));

        let cost = calculate_movement_cost(&state, PlayerId::new(0), &[AreaId::new(0), AreaId::new(1)]);
        assert_eq!(cost, 1); // 1 Yen for canal
    }

    #[test]
    fn test_mercenary_no_cost_for_opponent_presidents() {
        let mut state = create_test_state_with_board();
        let player = PlayerId::new(0);

        // Place opponent president on intermediate area (area 1)
        state.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));

        // Path through area 1 to area 2
        let path = vec![AreaId::new(1), AreaId::new(2)];

        // Normal movement: costs 1 Yen for opponent president
        let normal_cost = calculate_movement_cost(&state, player, &path);
        assert_eq!(normal_cost, 1);

        // Mercenary movement: no cost for opponent president
        let merc_cost = calculate_movement_cost_mercenary(&state, player, &path);
        assert_eq!(merc_cost, 0);
    }

    #[test]
    fn test_mercenary_still_pays_canal_toll() {
        let mut state = create_test_state_with_board();
        let player = PlayerId::new(0);

        // Add a canal crossing
        state.board_layout.canal_crossings.push((AreaId::new(1), AreaId::new(2)));

        // Path through canal
        let path = vec![AreaId::new(1), AreaId::new(2)];

        // Normal movement: costs 1 Yen for canal
        let normal_cost = calculate_movement_cost(&state, player, &path);
        assert_eq!(normal_cost, 1);

        // Mercenary movement: still costs 1 Yen for canal (mercenaries pay canal tolls)
        let merc_cost = calculate_movement_cost_mercenary(&state, player, &path);
        assert_eq!(merc_cost, 1);
    }

    #[test]
    fn test_mercenary_with_opponent_and_canal() {
        let mut state = create_test_state_with_board();
        let player = PlayerId::new(0);

        // Place opponent president on intermediate area
        state.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));

        // Add a canal crossing
        state.board_layout.canal_crossings.push((AreaId::new(1), AreaId::new(2)));

        // Path through both opponent and canal
        let path = vec![AreaId::new(1), AreaId::new(2)];

        // Normal movement: 1 for opponent + 1 for canal = 2
        let normal_cost = calculate_movement_cost(&state, player, &path);
        assert_eq!(normal_cost, 2);

        // Mercenary movement: 0 for opponent + 1 for canal = 1
        let merc_cost = calculate_movement_cost_mercenary(&state, player, &path);
        assert_eq!(merc_cost, 1);
    }

    // --- Task 9: Canal crossing is a live, tolled move ------------------

    fn mover() -> PlayerId {
        PlayerId::new(0)
    }

    /// Minimal 2-area board: the mover's President sits at `start` (area 0),
    /// with an own assistant on `far` (area 1). `start` and `far` flank a
    /// Canal (no Canal-typed cell needs to exist in the fixture - only
    /// `adjacency`/`canal_crossings`, which the movement code actually
    /// consults, matter), so crossing between them is a single-hop path
    /// that tolls 1 Yen to the bank.
    fn canal_crossing_fixture() -> (GameState, Rules) {
        let config = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));

        let mut areas = BTreeMap::new();
        areas.insert(AreaId::new(0), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(AreaId::new(1), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 2) });

        let mut adjacency = BTreeMap::new();
        adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        adjacency.insert(AreaId::new(1), vec![AreaId::new(0)]);

        let board_layout = BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![(AreaId::new(0), AreaId::new(1))],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };

        let mut players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];
        players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        players[0].assistants.on_board.insert(AreaId::new(1), 1);
        players[0].money = 5;
        players[1].money = 5;

        let state = GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::Movement,
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
        };
        (state, Rules::empty_for_tests())
    }

    fn cross_move() -> Move {
        Move::MovePresident { path: vec![AreaId::new(1)], destination: AreaId::new(1) }
    }

    #[test]
    fn president_can_cross_canal_paying_one_yen_to_bank() {
        // Own assistant on the far side; path = [far]; start flanks the canal opposite it.
        let (state, rules) = canal_crossing_fixture();
        let legal = get_legal_moves(&state, &rules);
        assert!(legal.contains(&cross_move()));
        let s = apply_move(&state, &cross_move(), &rules).unwrap();
        assert_eq!(s.player(mover()).money, state.player(mover()).money - 1); // toll
        // Bank toll: NOT credited to any player.
        let others_total: u32 = s.players.iter().filter(|p| p.id != mover()).map(|p| p.money).sum();
        let others_before: u32 = state.players.iter().filter(|p| p.id != mover()).map(|p| p.money).sum();
        assert_eq!(others_total, others_before);
    }

    /// `start`(0) --canal--> `mid`(1) --ordinary adjacency--> `far`(2), with
    /// own assistants on both `mid` and `far` so the President's move is a
    /// two-step path [mid, far]. The canal crossing is on start -> path[0]
    /// (the FIRST step), a regression for the `path.windows(2)` gap that
    /// misses that edge (only pairs WITHIN `path` were ever scanned).
    fn canal_first_step_fixture() -> (GameState, Rules) {
        let (mut state, rules) = canal_crossing_fixture();
        state.board_layout.areas.insert(AreaId::new(2), AreaConfig { area_type: AreaType::SilkMill, grid_position: (0, 3) });
        state.board_layout.adjacency.entry(AreaId::new(1)).or_default().push(AreaId::new(2));
        state.board_layout.adjacency.insert(AreaId::new(2), vec![AreaId::new(1)]);
        state.players[0].assistants.on_board.insert(AreaId::new(2), 1);
        (state, rules)
    }

    fn first_step_cross_move() -> Move {
        Move::MovePresident { path: vec![AreaId::new(1), AreaId::new(2)], destination: AreaId::new(2) }
    }

    #[test]
    fn start_edge_crossing_is_tolled() {
        // The FIRST step of the path crosses the canal (regression for the
        // path.windows(2) gap that missed start -> path[0]).
        let (state, rules) = canal_first_step_fixture();
        let s = apply_move(&state, &first_step_cross_move(), &rules).unwrap();
        assert_eq!(s.player(mover()).money, state.player(mover()).money - 1);
    }

    /// `canal_crossing_fixture` plus the mover holding Stagecoach - which
    /// waives opponent-President pass-through/destination fees but must NOT
    /// waive the canal toll (a separate, unconditional charge).
    fn canal_crossing_with_stagecoach_fixture() -> (GameState, Rules) {
        let (mut state, mut rules) = canal_crossing_fixture();
        let stagecoach_id = TechCardId(1);
        let stagecoach = TechCard {
            id: stagecoach_id,
            name: "Stagecoach".to_string(),
            country: Country::Dutch,
            industry_cost: 2,
            effect: TechEffect::Stagecoach,
        };
        rules.techs.insert(stagecoach, "tech_stagecoach");
        state.players[0].technologies.push(stagecoach_id);
        (state, rules)
    }

    #[test]
    fn stagecoach_does_not_waive_canal_toll() {
        let (state, rules) = canal_crossing_with_stagecoach_fixture();
        let s = apply_move(&state, &cross_move(), &rules).unwrap();
        assert_eq!(s.player(mover()).money, state.player(mover()).money - 1);
    }

    // --- I3 final-review fix: minimum-TOTAL-cost path per destination ----
    // (pricing fix retained; the two tests below were rewritten for Task 6
    // -- L2 route enumeration now offers EVERY affordable label per
    // destination, not just the cheapest, so their old "exactly one path"
    // assertions no longer hold. See the updated tests past the fixtures.)

    /// `start`(0) --canal--> `direct_dest`(1), PLUS a longer, toll-free
    /// around-route 0-2-3-1 (ordinary adjacency, no canal on 0-2 or 2-3)
    /// reaching the SAME destination. The mover's own assistants sit on 1,
    /// 2, AND 3 -- a path must hold the mover's assistant at every hop,
    /// including the destination (`find_reachable_areas_with_tech`'s doc
    /// comment). BFS-shortest-HOP-COUNT picks the 1-hop canal path and
    /// tolls it even though the 3-hop around-route reaches area 1 for
    /// free -- the bug this fixture reproduces.
    fn canal_vs_around_fixture() -> (GameState, Rules) {
        let config = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));

        let mut areas = BTreeMap::new();
        areas.insert(AreaId::new(0), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(AreaId::new(1), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 3) });
        areas.insert(AreaId::new(2), AreaConfig { area_type: AreaType::SilkMill, grid_position: (0, 1) });
        areas.insert(AreaId::new(3), AreaConfig { area_type: AreaType::TeaPlantation, grid_position: (0, 2) });

        let mut adjacency = BTreeMap::new();
        adjacency.insert(AreaId::new(0), vec![AreaId::new(1), AreaId::new(2)]);
        adjacency.insert(AreaId::new(1), vec![AreaId::new(0), AreaId::new(3)]);
        adjacency.insert(AreaId::new(2), vec![AreaId::new(0), AreaId::new(3)]);
        adjacency.insert(AreaId::new(3), vec![AreaId::new(1), AreaId::new(2)]);

        let board_layout = BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![(AreaId::new(0), AreaId::new(1))],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };

        let mut players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];
        players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        players[0].assistants.on_board.insert(AreaId::new(1), 1);
        players[0].assistants.on_board.insert(AreaId::new(2), 1);
        players[0].assistants.on_board.insert(AreaId::new(3), 1);
        players[0].money = 5;
        players[1].money = 5;

        let state = GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::Movement,
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
        };
        (state, Rules::empty_for_tests())
    }

    #[test]
    fn movement_offers_both_the_free_around_path_and_the_tolled_canal_shortcut() {
        // Was `movement_prefers_a_free_around_path_over_a_tolled_canal_shortcut`
        // (I3 final-review, pre-L2): back when only the cheapest path per
        // destination survived, this pinned "prefer the free around-route
        // over the tolled shortcut". Task 6 (L2 route enumeration)
        // supersedes the exclusivity claim: the free around-path
        // (vector {}) and the tolled direct shortcut (vector {None: 1})
        // are DIFFERENT labels to the SAME destination, both affordable
        // (money 5), so post-L2 BOTH must be offered -- the player
        // chooses, the enumerator doesn't pick a favorite. The underlying
        // I3 pricing fix (a canal crossing is always hop-count-shortest,
        // so naive BFS would toll a destination even when a longer,
        // toll-free route also reaches it) is still exactly what's pinned
        // here: both routes' costs must be computed correctly.
        let (state, rules) = canal_vs_around_fixture();
        let legal = get_legal_moves(&state, &rules);
        let dest = AreaId::new(1);
        let free_path = Move::MovePresident {
            path: vec![AreaId::new(2), AreaId::new(3), AreaId::new(1)],
            destination: dest,
        };
        let tolled_path = Move::MovePresident { path: vec![AreaId::new(1)], destination: dest };
        assert!(legal.contains(&free_path), "free around-path must still be offered: {legal:?}");
        assert!(legal.contains(&tolled_path), "tolled direct shortcut must ALSO be offered (L2): {legal:?}");

        let s_free = apply_move(&state, &free_path, &rules).unwrap();
        assert_eq!(s_free.player(mover()).money, state.player(mover()).money, "no toll paid via the around-path");

        let s_tolled = apply_move(&state, &tolled_path, &rules).unwrap();
        assert_eq!(s_tolled.player(mover()).money, state.player(mover()).money - 1, "toll paid via the direct shortcut");
    }

    /// Reverse pin: same direct-vs-around topology as `canal_vs_around_fixture`,
    /// but the around-route now ALSO crosses a canal (3 -> 1) and passes an
    /// opponent President at area 2 -- pricing it at 2 Yen, strictly worse
    /// than the direct route's 1 Yen toll. Proves the fix genuinely COMPARES
    /// total cost rather than unconditionally preferring the longer/
    /// non-direct route (which would be an overcorrection): the
    /// canal-crossing path must still be the one offered, and still tolled.
    fn canal_still_cheapest_fixture() -> (GameState, Rules) {
        let (mut state, rules) = canal_vs_around_fixture();
        state.board_layout.canal_crossings.push((AreaId::new(3), AreaId::new(1)));
        state.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(2));
        (state, rules)
    }

    #[test]
    fn movement_offers_both_routes_even_when_the_around_path_is_pricier() {
        // Was `movement_still_tolls_the_canal_when_it_is_genuinely_cheapest`
        // (I3 final-review reverse pin, pre-L2): `canal_still_cheapest_fixture`
        // makes the around-route STRICTLY pricier (2 Yen: a Blue pass-through
        // fee plus a second canal toll) than the direct 1-Yen crossing, to
        // prove the old fix genuinely COMPARED total cost rather than
        // unconditionally preferring the longer/non-canal route. Under L2
        // both routes are still independently affordable (money 5) and
        // both are now offered -- proving the enumerator isn't "always
        // prefer non-canal" OR "always cheapest", it's "offer every
        // affordable label" (income-denial routing doesn't require the
        // pricier route to be a GOOD idea, only that it's legal and
        // affordable).
        let (state, rules) = canal_still_cheapest_fixture();
        let legal = get_legal_moves(&state, &rules);
        let dest = AreaId::new(1);
        let direct = Move::MovePresident { path: vec![AreaId::new(1)], destination: dest };
        let around = Move::MovePresident {
            path: vec![AreaId::new(2), AreaId::new(3), AreaId::new(1)],
            destination: dest,
        };
        assert!(legal.contains(&direct), "direct canal crossing must be offered: {legal:?}");
        assert!(legal.contains(&around), "pricier around-route must ALSO be offered (L2): {legal:?}");

        let s_direct = apply_move(&state, &direct, &rules).unwrap();
        assert_eq!(s_direct.player(mover()).money, state.player(mover()).money - 1, "canal toll charged on the direct route");

        let blue = PlayerId::new(1);
        let s_around = apply_move(&state, &around, &rules).unwrap();
        assert_eq!(
            s_around.player(mover()).money,
            state.player(mover()).money - 2,
            "fee + toll charged on the pricier around-route"
        );
        assert_eq!(
            s_around.player(blue).money,
            state.player(blue).money + 1,
            "Blue collects the pass-through fee"
        );
    }

    // === Task 6: L2 route enumeration =====================================
    //
    // Owner's L2 ruling (design 2026-07-19 §4.3): every distinct
    // (destination, RouteVector) over simple affordable paths is its own
    // legal move -- deliberately paying MORE to deny an opponent a fee (or
    // simply to pay the bank instead of an opponent) is real, enumerable
    // play. These fixtures are small diamond graphs -- `s`(0) branches into
    // two arms that rejoin at `dest`(3) -- built with this module's usual
    // by-hand `GameState` construction (via the shared `l2_base_state`
    // skeleton below, which only factors out the boilerplate field list
    // common to ALL of them; each fixture still owns its own board/
    // assistant/president layout so a reviewer can check the fee/toll
    // arithmetic directly against the fixture that produced it).

    /// Shared `GameState` skeleton for the L2 fixtures below: 2 players,
    /// Movement phase, empty tracks/decks/hands -- mirrors
    /// `canal_crossing_fixture`'s field list exactly (this file's
    /// established construction pattern), factored out only because THIS
    /// task adds several more small board fixtures than any single prior
    /// task did. Callers set `board_layout`, president locations,
    /// assistants, and money.
    fn l2_base_state(board_layout: BoardLayout) -> GameState {
        let config = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));
        let mut players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];
        players[0].money = 5;
        players[1].money = 5;

        GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::Movement,
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

    /// Diamond: `s`(0) branches to `x`(1) and `y`(2), both adjacent to
    /// `dest`(3). Own assistants sit on x, y, AND dest (a path must hold
    /// the mover's assistant at every hop). Route via x: Blue's President
    /// occupies x (no Postal System, so x itself is never a legal stop, but
    /// PASSING THROUGH it still costs 1 Yen to Blue; no canal on s-x or
    /// x-dest). Route via y: canal toll on s-y (1 Yen to the bank; no
    /// opponent at y, no canal on y-dest). Both routes total 1 Yen but pay
    /// a DIFFERENT recipient.
    fn l2_diamond_fixture() -> (GameState, Rules) {
        let s = AreaId::new(0);
        let x = AreaId::new(1);
        let y = AreaId::new(2);
        let dest = AreaId::new(3);

        let mut areas = BTreeMap::new();
        areas.insert(s, AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(x, AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });
        areas.insert(y, AreaConfig { area_type: AreaType::SilkMill, grid_position: (1, 0) });
        areas.insert(dest, AreaConfig { area_type: AreaType::TeaPlantation, grid_position: (1, 1) });

        let mut adjacency = BTreeMap::new();
        adjacency.insert(s, vec![x, y]);
        adjacency.insert(x, vec![s, dest]);
        adjacency.insert(y, vec![s, dest]);
        adjacency.insert(dest, vec![x, y]);

        let board_layout = BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![(s, y)],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };

        let mut state = l2_base_state(board_layout);
        state.players[0].president_location = PresidentLocation::OnBoard(s);
        state.players[0].assistants.on_board.insert(x, 1);
        state.players[0].assistants.on_board.insert(y, 1);
        state.players[0].assistants.on_board.insert(dest, 1);
        state.players[1].president_location = PresidentLocation::OnBoard(x); // Blue blocks x

        (state, Rules::empty_for_tests())
    }

    #[test]
    fn equal_cost_routes_with_different_fee_recipients_are_both_enumerated() {
        let (state, rules) = l2_diamond_fixture();
        let p0 = PlayerId::new(0);
        let blue = PlayerId::new(1);
        let s = AreaId::new(0);
        let dest = AreaId::new(3);

        let routes = enumerate_president_routes(&state, p0, s, Some(&rules.techs));
        let to_d: Vec<_> = routes.iter().filter(|(d, _)| *d == dest).collect();
        assert_eq!(to_d.len(), 2, "two distinct fee vectors must both be offered: {to_d:?}");
        let vectors: Vec<RouteVector> = to_d
            .iter()
            .map(|(_, path)| route_fee_vector(&state, p0, path, Some(&rules.techs), false))
            .collect();
        assert!(vectors.contains(&BTreeMap::from([(None, 1)])), "bank-toll route offered: {vectors:?}");
        assert!(vectors.contains(&BTreeMap::from([(Some(blue), 1)])), "fee-to-Blue route offered: {vectors:?}");
    }

    /// Second diamond: `s`(0) branches to `z`(1) [free: no canal, no
    /// opponent] and `w`(2) [BOTH edges into and out of w cross the canal:
    /// 2 Yen to the bank], rejoining at `dest`(3). No opponent Presidents
    /// anywhere -- the only cost differential is the canal.
    fn l2_free_vs_detour_fixture() -> (GameState, Rules) {
        let s = AreaId::new(0);
        let z = AreaId::new(1);
        let w = AreaId::new(2);
        let dest = AreaId::new(3);

        let mut areas = BTreeMap::new();
        areas.insert(s, AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(z, AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });
        areas.insert(w, AreaConfig { area_type: AreaType::SilkMill, grid_position: (1, 0) });
        areas.insert(dest, AreaConfig { area_type: AreaType::TeaPlantation, grid_position: (1, 1) });

        let mut adjacency = BTreeMap::new();
        adjacency.insert(s, vec![z, w]);
        adjacency.insert(z, vec![s, dest]);
        adjacency.insert(w, vec![s, dest]);
        adjacency.insert(dest, vec![z, w]);

        let board_layout = BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![(s, w), (w, dest)],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };

        let mut state = l2_base_state(board_layout);
        state.players[0].president_location = PresidentLocation::OnBoard(s);
        state.players[0].assistants.on_board.insert(z, 1);
        state.players[0].assistants.on_board.insert(w, 1);
        state.players[0].assistants.on_board.insert(dest, 1);

        (state, Rules::empty_for_tests())
    }

    #[test]
    fn deliberately_pricier_routes_are_enumerated_when_affordable_and_pruned_when_not() {
        let (mut state, rules) = l2_free_vs_detour_fixture();
        let p0 = PlayerId::new(0);
        let s = AreaId::new(0);
        let dest = AreaId::new(3);

        state.players[0].money = 2;
        let routes = enumerate_president_routes(&state, p0, s, Some(&rules.techs));
        let sums: Vec<u32> = routes
            .iter()
            .filter(|(d, _)| *d == dest)
            .map(|(_, path)| route_fee_vector(&state, p0, path, Some(&rules.techs), false).values().sum())
            .collect();
        assert!(sums.contains(&0) && sums.contains(&2), "free AND pricier labels offered: {sums:?}");

        state.players[0].money = 0;
        let routes = enumerate_president_routes(&state, p0, s, Some(&rules.techs));
        let sums: Vec<u32> = routes
            .iter()
            .filter(|(d, _)| *d == dest)
            .map(|(_, path)| route_fee_vector(&state, p0, path, Some(&rules.techs), false).values().sum())
            .collect();
        assert_eq!(sums, vec![0], "unaffordable labels pruned: {sums:?}");
    }

    /// Diamond with BOTH branches entirely free (no canal, no opponent
    /// President anywhere): `s`(0) -> `a`(1) -> `dest`(3) and `s`(0) ->
    /// `b`(2) -> `dest`(3) are both 0-cost, so they collide on the SAME
    /// (destination, RouteVector) key ({}) -- the tie-break must keep the
    /// lexicographically-least path, i.e. through `a` (AreaId(1) <
    /// AreaId(2)).
    fn l2_tied_free_routes_fixture() -> (GameState, Rules) {
        let s = AreaId::new(0);
        let a = AreaId::new(1);
        let b = AreaId::new(2);
        let dest = AreaId::new(3);

        let mut areas = BTreeMap::new();
        areas.insert(s, AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(a, AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });
        areas.insert(b, AreaConfig { area_type: AreaType::SilkMill, grid_position: (1, 0) });
        areas.insert(dest, AreaConfig { area_type: AreaType::TeaPlantation, grid_position: (1, 1) });

        let mut adjacency = BTreeMap::new();
        adjacency.insert(s, vec![a, b]);
        adjacency.insert(a, vec![s, dest]);
        adjacency.insert(b, vec![s, dest]);
        adjacency.insert(dest, vec![a, b]);

        let board_layout = BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };

        let mut state = l2_base_state(board_layout);
        state.players[0].president_location = PresidentLocation::OnBoard(s);
        state.players[0].assistants.on_board.insert(a, 1);
        state.players[0].assistants.on_board.insert(b, 1);
        state.players[0].assistants.on_board.insert(dest, 1);

        (state, Rules::empty_for_tests())
    }

    #[test]
    fn route_enumeration_is_deterministic_and_canonical() {
        let (state, rules) = l2_tied_free_routes_fixture();
        let p0 = PlayerId::new(0);
        let s = AreaId::new(0);
        let a = AreaId::new(1);
        let b = AreaId::new(2);
        let dest = AreaId::new(3);

        let routes_a = enumerate_president_routes(&state, p0, s, Some(&rules.techs));
        let routes_b = enumerate_president_routes(&state, p0, s, Some(&rules.techs));
        assert_eq!(routes_a, routes_b, "enumeration must be deterministic across calls");

        // Canonical representative: s->a->dest and s->b->dest tie on the
        // empty RouteVector, so the lexicographically-least path (through
        // a, AreaId(1) < AreaId(2)) must be the one kept.
        assert!(
            routes_a.contains(&(dest, vec![a, dest])),
            "lexicographically-least tied path must be kept: {routes_a:?}"
        );
        assert!(
            !routes_a.iter().any(|(d, path)| *d == dest && path == &vec![b, dest]),
            "the lexicographically-greater tied path must be dropped: {routes_a:?}"
        );
    }

    #[test]
    fn cost_full_equals_vector_sum_on_random_walks() {
        // Definitional identity, not an independent safety net:
        // calculate_movement_cost_full IS
        // route_fee_vector(...).values().sum() by construction (see its
        // definition above), so the two sides here can never disagree --
        // this cannot catch a pricing regression. What it actually pins is
        // that both functions run to completion without panicking across a
        // real range of paths: (a) the full enumerated set from two
        // fixtures that between them exercise canal tolls, president
        // pass-through fees, AND destination fees; (b) a few seeded random
        // walks over a third fixture's graph, NOT restricted to simple
        // paths or to enumerate_president_routes's affordability pruning --
        // just "any sequence a naive path-builder might produce", including
        // paths that revisit an area.
        let p0 = PlayerId::new(0);

        let (state1, rules1) = canal_vs_around_fixture();
        for (_, path) in enumerate_president_routes(&state1, p0, AreaId::new(0), Some(&rules1.techs)) {
            let full = calculate_movement_cost_full(&state1, p0, &path, Some(&rules1.techs), false);
            let vector_sum: u32 =
                route_fee_vector(&state1, p0, &path, Some(&rules1.techs), false).values().sum();
            assert_eq!(full, vector_sum, "path {path:?} on canal_vs_around_fixture");
        }

        let (state2, rules2) = l2_diamond_fixture();
        for (_, path) in enumerate_president_routes(&state2, p0, AreaId::new(0), Some(&rules2.techs)) {
            let full = calculate_movement_cost_full(&state2, p0, &path, Some(&rules2.techs), false);
            let vector_sum: u32 =
                route_fee_vector(&state2, p0, &path, Some(&rules2.techs), false).values().sum();
            assert_eq!(full, vector_sum, "path {path:?} on l2_diamond_fixture");
        }

        use rand::{Rng, SeedableRng};
        let candidates = [AreaId::new(1), AreaId::new(2), AreaId::new(3)];
        for seed in 0u64..5 {
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
            let len = 1 + rng.gen_range(0..3);
            let path: Vec<AreaId> = (0..len).map(|_| candidates[rng.gen_range(0..candidates.len())]).collect();
            let full = calculate_movement_cost_full(&state1, p0, &path, Some(&rules1.techs), false);
            let vector_sum: u32 =
                route_fee_vector(&state1, p0, &path, Some(&rules1.techs), false).values().sum();
            assert_eq!(full, vector_sum, "seed {seed}: random walk path {path:?}");
        }
    }
}
