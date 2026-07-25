//! Allocation-free President-route enumeration (Phase 3.1b, spec
//! `docs/plans/2026-07-25-phase3.1b-routes-validation-spec.md` §3.1).
//!
//! [`enumerate_routes`] is an exact, order-identical replacement for
//! [`enumerate_president_routes`](crate::game::enumerate_president_routes) on
//! the factored side: same `(destination, path)` pairs, same emission order,
//! same lex-least witness per label. The frozen generator stays the authority
//! `apply_move` checks against and the standing oracle every equivalence test
//! compares to; this module exists so a Movement decision point can be served
//! without the frozen search's per-node `BTreeMap` traffic (spec §0: 467
//! allocations against the reference's 1,687,537 at the adversarial maximum,
//! measured out of repo).
//!
//! ## The label is three scalars, so no `RouteVector` is built during search
//!
//! `route_fee_vector` (movement.rs:330) prices a path as a closed form: 1 Yen
//! per opponent-President owner per path AREA — the "intermediate" and
//! "Postal destination" loops apply the identical charge to disjoint slices of
//! `path`, so every area is priced exactly once, position-independently, and
//! Stagecoach waives the whole term — plus 1 Yen to the bank per canal crossing
//! over the `[president_location] ++ path` EDGE sequence, which nothing waives.
//! An opponent owns one President on one area and a simple path visits each
//! area once, so each owner contributes 0 or 1: the entire fee vector is
//! `(owner bitmask, toll count)`, both incremental along a hop. The search
//! therefore carries `(u8 mask, u32 tolls)` and reconstructs the frozen
//! generator's `BTreeMap` emission order at the end via [`cmp_label`], never
//! materialising a map.
//!
//! ## Lex-least witness: two halves and a bridging lemma
//!
//! Both searches keep one witness path per `(destination, fee vector)` label,
//! and both keep the lexicographically least one — by different mechanisms, so
//! the agreement needs an argument.
//!
//! **(i) This module.** Children are tried in ascending `AreaId` order, so the
//! preorder DFS emits simple paths in strictly increasing lexicographic order:
//! a path is emitted before every extension of it (a proper prefix sorts before
//! the longer vector), and two paths that first differ at hop `k` are emitted in
//! the order of that hop, because the subtree of the smaller hop is fully
//! explored before the larger hop is tried. Hence the FIRST path recorded for a
//! label is that label's lex-least path, and first-encounter-wins is exactly the
//! tie-break — pinned live by the `debug_assert` in [`Search::step`], which
//! fires if a later encounter is ever lex-smaller than the stored witness.
//!
//! **(ii) The frozen reference.** `RouteDfs::step` (movement.rs:508) does a
//! compare-and-keep — `path < existing` replaces the stored witness — which
//! selects the lex-least member of whatever set it visits, independent of
//! traversal order (and so is insensitive to `BoardLayout`'s neighbour order).
//!
//! **Bridging lemma.** The two selections agree because they range over the
//! same candidate set. Both hop only where `can_step_onto` allows (own
//! assistant present, never back onto the start area), both keep paths simple,
//! both prune on the same monotone affordability test (`fee sum <= money`; no
//! edge has negative weight, so an unaffordable prefix has no affordable
//! extension), and both record a destination only where
//! `is_possible_action_area_with_tech` holds. Same candidate set + same
//! selection criterion (lex-least) ⇒ same witness, so the outputs are equal
//! element for element once both are ordered by `(destination, fee vector)`.
//!
//! ## Totality
//!
//! The search writes into fixed-size arrays indexed by raw `AreaId`, so
//! [`fits_fixed_arrays`] is evaluated as a PRE-FLIGHT — before any array is
//! built — and a state it rejects (only a hand-built one can be) delegates to
//! the frozen enumerator, which is output-identical by the guarantee above. No
//! input panics.

use std::cmp::Ordering;

use crate::game::{enumerate_president_routes, is_possible_action_area_with_tech, Rules, TechRegistry};
use crate::types::*;

/// Fixed-array capacity for board areas: `AreaId` is a `u8` the type asserts
/// below 18, and the largest generated board is the 4P pyramid's 18 tiles. The
/// arrays are sized off THIS, never off the tighter ≤16-steppable observation —
/// that one is a property of board *generation*, which this module does not get
/// to assume (spec F8).
pub const MAX_AREAS: usize = 18;

/// Fixed-array capacity for one area's stored neighbours: brick-offset
/// adjacency contributes at most 6 and any-to-any Canal transit the rest.
const MAX_DEG: usize = 17;

/// `u128` words per destination in the found-label bitset. A label class is
/// `(owner mask, toll count)`, at most `2^3 · (MAX_AREAS + 1) = 152` of them.
const MAX_WORDS: usize = 2;

/// Opponents whose President fees the `u8` owner mask can distinguish. Yokohama
/// seats at most 4, so at most 3 — but a hand-built state can hold more, which
/// is why [`fits_fixed_arrays`] checks it rather than assuming it.
const MAX_OPPONENTS: usize = 3;

/// Whether [`RouteGraph::build`] and the search can run over `state` without
/// exceeding a fixed array. Evaluated over `BoardLayout` + `players` BEFORE any
/// array is written, so a rejected state never touches the fast path at all.
///
/// The four bounds, each tied to the write it guards:
/// 1. every area id (and every adjacency endpoint) is below [`MAX_AREAS`] —
///    they index the per-area tables and the `u32` visited bitmask directly;
/// 2. no adjacency list is longer than `MAX_DEG` — it is copied into the
///    per-area neighbour row. The bound is on the RAW list, so a list padded
///    with duplicates is rejected even though its distinct entries would fit;
/// 3. at most [`MAX_OPPONENTS`] opponents — one owner bit each in a `u8`;
/// 4. the resulting label space fits `MAX_WORDS` `u128`s per destination.
///
/// Rejection is conservative, never wrong: the caller delegates to the frozen
/// enumerator, whose output this module reproduces exactly.
pub fn fits_fixed_arrays(state: &GameState) -> bool {
    let layout = &state.board_layout;

    if layout.areas.len() > MAX_AREAS || layout.areas.keys().any(|a| a.0 as usize >= MAX_AREAS) {
        return false;
    }
    for (area, neighbors) in &layout.adjacency {
        if area.0 as usize >= MAX_AREAS || neighbors.len() > MAX_DEG {
            return false;
        }
        if neighbors.iter().any(|n| n.0 as usize >= MAX_AREAS) {
            return false;
        }
    }
    // The President's own area is the search root, so it indexes the arrays too.
    for seat in &state.players {
        if let PresidentLocation::OnBoard(area) = seat.president_location {
            if area.0 as usize >= MAX_AREAS {
                return false;
            }
        }
    }

    let opponents = state.players.len().saturating_sub(1);
    if opponents > MAX_OPPONENTS {
        return false;
    }
    // Worst case over every player and every money value at this board size
    // (see `RouteGraph::max_tolls`), so one probe covers the whole state.
    let label_classes = (1usize << opponents) * (MAX_AREAS + 1);
    label_classes <= MAX_WORDS * 128
}

/// Whether `player` owns a technology with `effect`.
fn has_tech(state: &GameState, player: PlayerId, reg: Option<&TechRegistry>, effect: TechEffect) -> bool {
    let p = state.player(player);
    reg.is_some_and(|registry| {
        p.technologies
            .iter()
            .any(|&id| registry.get(id).is_some_and(|card| card.effect == effect))
    })
}

/// The own-assistant subgraph the search walks, precomputed once per decision
/// point into fixed-size arrays. No heap, no `GameState` reads during search.
///
/// PRECONDITION: [`fits_fixed_arrays`] holds for `state` (debug-asserted). The
/// per-area tables are computed by calling the same predicates the frozen
/// enumerator calls, over every id in `0..MAX_AREAS` rather than over
/// `BoardLayout::areas` — `can_step_onto` and `is_possible_action_area_with_tech`
/// read assistants and President locations, not board membership, so a state
/// whose assistants sit off the declared area map still enumerates identically.
///
/// `pub(crate)` deliberately: the precondition is only debug-asserted, so a
/// crate-external caller could construct one on an oversized board and hit an
/// index panic in release — [`enumerate_routes`] (which pre-flights and
/// delegates) is the total public surface; this type is plumbing for it and
/// for the in-crate 3.1b family assembly.
pub(crate) struct RouteGraph {
    /// Search root: the area the President walks from.
    from: u8,
    money: u32,
    /// `is_possible_action_area_with_tech` per area — where a walk may STOP.
    is_dest: [bool; MAX_AREAS],
    /// Owner bits of the opponent Presidents standing on each area (empty under
    /// Stagecoach, which waives the whole President-fee term).
    fee_mask: [u8; MAX_AREAS],
    /// Steppable neighbours per area as `(area, edge is a canal crossing)`,
    /// ASCENDING — the lex-least-witness proof depends on this order, so it is
    /// imposed here rather than inherited from `BoardLayout`.
    adj: [[(u8, bool); MAX_DEG]; MAX_AREAS],
    adj_len: [u8; MAX_AREAS],
    /// Whether the first hop onto each area crosses a canal. Keyed on the
    /// President's CURRENT area (what `route_fee_vector` prepends), which is
    /// `from` for every caller in this crate but is tracked separately so the
    /// two can never silently diverge.
    first_toll: [bool; MAX_AREAS],
    /// Toll-count ceiling, so `(mask, tolls)` indexes a bounded label space: a
    /// simple path takes at most one hop per area and each hop tolls at most
    /// once, while affordability caps the count at `money`. The prototype
    /// tightened this to the canal-incident node count; that only ever fed its
    /// saturation-prune oracle, which is not ported (spec F3 ships `Prune::None`),
    /// and the loose bound still lands inside the same fixed bitset.
    max_tolls: u32,
    /// Bitset words per destination.
    words: usize,
}

impl RouteGraph {
    /// Precompute the subgraph for `player` walking from `from`.
    pub(crate) fn build(
        state: &GameState,
        player: PlayerId,
        from: AreaId,
        reg: Option<&TechRegistry>,
    ) -> RouteGraph {
        debug_assert!(
            fits_fixed_arrays(state),
            "RouteGraph::build requires a state that passed the fits_fixed_arrays pre-flight",
        );
        let p = state.player(player);
        let stagecoach = has_tech(state, player, reg, TechEffect::Stagecoach);

        // Owner bit order is ascending PlayerId, which `cmp_label` relies on to
        // compare bit indices as if they were `Some(PlayerId)` map keys. The
        // `n_owners` cap is a no-panic backstop for a caller that ignored the
        // pre-flight, NOT support for a fourth opponent: past three owners the
        // fee vector would be wrong, which is exactly what the pre-flight
        // refuses upstream.
        let mut owners = [PlayerId(0); MAX_OPPONENTS];
        let mut n_owners = 0usize;
        if !stagecoach {
            for seat in &state.players {
                if seat.id != player
                    && matches!(seat.president_location, PresidentLocation::OnBoard(_))
                    && n_owners < MAX_OPPONENTS
                {
                    owners[n_owners] = seat.id;
                    n_owners += 1;
                }
            }
            owners[..n_owners].sort_unstable();
        }

        let president_area = match p.president_location {
            PresidentLocation::OnBoard(area) => Some(area),
            PresidentLocation::InHand => None,
        };

        let mut steppable = [false; MAX_AREAS];
        let mut is_dest = [false; MAX_AREAS];
        let mut fee_mask = [0u8; MAX_AREAS];
        let mut first_toll = [false; MAX_AREAS];
        for i in 0..MAX_AREAS {
            let area = AreaId::new(i as u8);
            // Mirrors `can_step_onto` (movement.rs:209) exactly.
            steppable[i] = area != from && p.assistants.in_area(area) > 0;
            is_dest[i] = is_possible_action_area_with_tech(state, player, area, reg);
            if !stagecoach {
                let mut mask = 0u8;
                for (bit, &owner) in owners[..n_owners].iter().enumerate() {
                    if state.player(owner).president_location == PresidentLocation::OnBoard(area) {
                        mask |= 1 << bit;
                    }
                }
                fee_mask[i] = mask;
            }
            if let Some(current) = president_area {
                first_toll[i] = state.board_layout.is_canal_crossing(current, area);
            }
        }

        let mut adj = [[(0u8, false); MAX_DEG]; MAX_AREAS];
        let mut adj_len = [0u8; MAX_AREAS];
        for (i, row) in adj.iter_mut().enumerate() {
            let area = AreaId::new(i as u8);
            let mut n = 0usize;
            for &neighbor in state.board_layout.adjacent_areas(area) {
                let j = neighbor.0 as usize;
                // A duplicated neighbour would re-walk an identical subtree; the
                // frozen search tolerates that (the repeat can only re-offer a
                // path it already stored), so dropping it is output-neutral.
                if !steppable[j] || row[..n].iter().any(|e| e.0 == neighbor.0) {
                    continue;
                }
                row[n] = (neighbor.0, state.board_layout.is_canal_crossing(area, neighbor));
                n += 1;
            }
            row[..n].sort_unstable_by_key(|e| e.0);
            adj_len[i] = n as u8;
        }

        let max_tolls = (MAX_AREAS as u32).min(p.money);
        let label_classes = (1usize << n_owners) * (max_tolls as usize + 1);
        RouteGraph {
            from: from.0,
            money: p.money,
            is_dest,
            fee_mask,
            adj,
            adj_len,
            first_toll,
            max_tolls,
            words: label_classes.div_ceil(128).max(1),
        }
    }

    /// Flatten a label class `(owner mask, toll count)` to a bit index.
    #[inline]
    fn idx(&self, mask: u8, tolls: u32) -> usize {
        mask as usize * (self.max_tolls as usize + 1) + tolls as usize
    }
}

/// Order two `(destination, owner mask, toll count)` labels exactly as the
/// frozen enumerator's `BTreeMap<(AreaId, RouteVector), _>` iteration does —
/// PORTED from the validated prototype, not re-derived, because the obvious
/// `(dest, mask, tolls)` tuple compare is wrong in two places.
///
/// A `RouteVector` is a `BTreeMap<Option<PlayerId>, u32>` compared as its sorted
/// `(key, value)` sequence, so: the bank entry `None` sorts BEFORE any
/// `Some(owner)`; a vector that is a prefix of another sorts first (the empty
/// vector first of all); and the bank entry's VALUE is the toll count, which is
/// compared before any owner key is looked at. Every owner entry's value is 1
/// (a simple path visits an owner's President at most once), so the owner tail
/// compares as the ascending owner-bit list — and the bits are assigned in
/// ascending `PlayerId` order by [`RouteGraph::build`], which is what makes
/// comparing bit indices the same as comparing `Some(PlayerId)` keys.
fn cmp_label(a: (u8, u8, u32), b: (u8, u8, u32)) -> Ordering {
    let (dest_a, mask_a, tolls_a) = a;
    let (dest_b, mask_b, tolls_b) = b;

    match dest_a.cmp(&dest_b) {
        Ordering::Equal => {}
        other => return other,
    }

    match (tolls_a > 0, tolls_b > 0) {
        (true, true) => match tolls_a.cmp(&tolls_b) {
            Ordering::Equal => {}
            other => return other,
        },
        // Only `a` pays the bank: `None` beats any owner key, but loses to the
        // empty vector, which is a prefix of everything.
        (true, false) => return if mask_b == 0 { Ordering::Greater } else { Ordering::Less },
        (false, true) => return if mask_a == 0 { Ordering::Less } else { Ordering::Greater },
        (false, false) => {}
    }

    let mut x = mask_a;
    let mut y = mask_b;
    while x != 0 && y != 0 {
        match x.trailing_zeros().cmp(&y.trailing_zeros()) {
            Ordering::Equal => {}
            other => return other,
        }
        x &= x - 1;
        y &= y - 1;
    }
    // One owner list ran out: it is a prefix of the other, so it sorts first.
    x.count_ones().cmp(&y.count_ones())
}

/// One recorded label and the witness path that first reached it.
struct Witness {
    dest: u8,
    mask: u8,
    tolls: u32,
    path: [u8; MAX_AREAS],
    len: u8,
}

/// The allocation-free DFS. The only heap traffic in a whole enumeration is the
/// witness buffer and the returned `Vec`s — one push per LABEL, never per path.
struct Search<'g> {
    g: &'g RouteGraph,
    /// Recorded label classes, `words` per destination.
    found: [u128; MAX_AREAS * MAX_WORDS],
    path: [u8; MAX_AREAS],
    plen: usize,
    witness: Vec<Witness>,
}

impl Search<'_> {
    #[inline]
    fn has(&self, dest: usize, idx: usize) -> bool {
        self.found[dest * self.g.words + idx / 128] & (1u128 << (idx % 128)) != 0
    }

    /// The stored witness for an already-recorded label. Debug-only support for
    /// the lex-order drift guard, so the linear scan never runs in release.
    fn stored_witness(&self, dest: u8, mask: u8, tolls: u32) -> Option<&[u8]> {
        self.witness
            .iter()
            .find(|w| w.dest == dest && w.mask == mask && w.tolls == tolls)
            .map(|w| &w.path[..w.len as usize])
    }

    /// Extend the current path by one hop from `at` in every affordable way,
    /// recording each label the first time it is reached.
    ///
    /// `visited` carries `from`'s bit from the start, so the root is never
    /// re-entered — which is also why `at == from` identifies the root hop, the
    /// one priced against the President's own area rather than against `at`.
    fn step(&mut self, at: u8, visited: u32, mask: u8, tolls: u32) {
        for k in 0..self.g.adj_len[at as usize] as usize {
            let (next, edge_toll) = self.g.adj[at as usize][k];
            let bit = 1u32 << next;
            if visited & bit != 0 {
                continue;
            }
            let toll = if at == self.g.from { self.g.first_toll[next as usize] } else { edge_toll };
            let next_mask = mask | self.g.fee_mask[next as usize];
            let next_tolls = tolls + u32::from(toll);
            // Monotone affordability prune: no edge is free of charge in the
            // wrong direction, so an unaffordable prefix has no affordable
            // extension.
            if next_mask.count_ones() + next_tolls > self.g.money {
                continue;
            }

            debug_assert!(self.plen < MAX_AREAS, "a simple path cannot exceed the area count");
            self.path[self.plen] = next;
            self.plen += 1;

            if self.g.is_dest[next as usize] {
                debug_assert!(next_tolls <= self.g.max_tolls, "toll count escaped the label space");
                let idx = self.g.idx(next_mask, next_tolls);
                if self.has(next as usize, idx) {
                    // Ascending-child preorder emits paths in strictly
                    // increasing lex order, so a re-encounter is always worse
                    // than the stored witness. This is that proof, executable.
                    debug_assert!(
                        self.stored_witness(next, next_mask, next_tolls)
                            .is_some_and(|stored| self.path[..self.plen] > *stored),
                        "lex order drift: a later witness was not lex-greater than the stored one",
                    );
                } else {
                    self.found[next as usize * self.g.words + idx / 128] |= 1u128 << (idx % 128);
                    self.witness.push(Witness {
                        dest: next,
                        mask: next_mask,
                        tolls: next_tolls,
                        path: self.path,
                        len: self.plen as u8,
                    });
                }
            }

            self.step(next, visited | bit, next_mask, next_tolls);
            self.plen -= 1;
        }
    }
}

/// Every distinct affordable `(destination, fee vector)` label the current
/// player's President can WALK to, as `(destination, witness path)` pairs
/// ordered by `(destination, fee vector)` with the lex-least path per label —
/// byte for byte what
/// [`enumerate_president_routes`](crate::game::enumerate_president_routes)
/// returns for the same state.
///
/// The walking block only: Station/Tram teleports and the from-hand
/// `PlacePresident` arm are the Movement family's other pieces (3.1b T2), so a
/// President in hand enumerates nothing here. Nothing about `turn_phase` is
/// consulted — the family dispatch owns that.
pub fn enumerate_routes(state: &GameState, rules: &Rules) -> Vec<(AreaId, Vec<AreaId>)> {
    let player = state.current_player;
    let PresidentLocation::OnBoard(from) = state.player(player).president_location else {
        return Vec::new();
    };
    walk_routes(state, player, from, Some(&rules.techs))
}

/// [`enumerate_routes`] with the walk's start area given explicitly, matching
/// `enumerate_president_routes`' signature — including its allowance that
/// `from` need not be the President's own area (the start area governs the
/// simple-path start-exclusion, while `route_fee_vector` prices the first hop
/// from `president_location`). Nothing in the engine separates the two, so
/// [`enumerate_routes`] never does either; the parameter exists so the port
/// mirrors the frozen contract rather than a coincidence of its callers.
fn walk_routes(
    state: &GameState,
    player: PlayerId,
    from: AreaId,
    reg: Option<&TechRegistry>,
) -> Vec<(AreaId, Vec<AreaId>)> {
    if !fits_fixed_arrays(state) {
        return enumerate_president_routes(state, player, from, reg);
    }

    let graph = RouteGraph::build(state, player, from, reg);
    let mut search = Search {
        found: [0u128; MAX_AREAS * MAX_WORDS],
        path: [0u8; MAX_AREAS],
        plen: 0,
        witness: Vec::new(),
        g: &graph,
    };
    search.step(graph.from, 1u32 << graph.from, 0, 0);

    let mut witnesses = search.witness;
    // Labels are unique by construction, so the comparator never reports Equal
    // between two entries and the sort needs no stability.
    witnesses.sort_unstable_by(|a, b| cmp_label((a.dest, a.mask, a.tolls), (b.dest, b.mask, b.tolls)));
    witnesses
        .into_iter()
        .map(|w| {
            (
                AreaId::new(w.dest),
                w.path[..w.len as usize].iter().map(|&a| AreaId::new(a)).collect(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::RouteVector;
    use std::collections::BTreeMap;

    // -- fixture construction -------------------------------------------------

    fn colors(n: u8) -> Vec<PlayerColor> {
        [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green]
            [..n as usize]
            .to_vec()
    }

    /// A Movement-phase `GameState` over `board_layout` with `players` seats,
    /// every seat holding 5 Yen and its President in hand. Mirrors this repo's
    /// established by-hand construction (`movement.rs`'s `l2_base_state`),
    /// widened to 2-4 seats so the multi-opponent fee fixtures below can exist.
    fn base_state(players: u8, board_layout: BoardLayout) -> GameState {
        let config = GameConfig::new(players, colors(players), PlayerId::new(0));
        let mut seats = Vec::new();
        for i in 0..players {
            let mut p = PlayerState::new(PlayerId::new(i), colors(players)[i as usize], i == 0);
            p.money = 5;
            seats.push(p);
        }

        GameState {
            config,
            board_layout,
            players: seats,
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

    /// Build a `BoardLayout` from area ids and undirected edges. Area types are
    /// irrelevant to route enumeration (only assistants, presidents, adjacency
    /// and canal crossings are read), so every tile is a Bank.
    fn board(areas: &[u8], edges: &[(u8, u8)], canals: &[(u8, u8)]) -> BoardLayout {
        let mut area_map = BTreeMap::new();
        for (n, &a) in areas.iter().enumerate() {
            area_map.insert(
                AreaId(a),
                AreaConfig { area_type: AreaType::Bank, grid_position: (0, n as u8) },
            );
        }
        let mut adjacency: BTreeMap<AreaId, Vec<AreaId>> = BTreeMap::new();
        for &(a, b) in edges {
            adjacency.entry(AreaId(a)).or_default().push(AreaId(b));
            adjacency.entry(AreaId(b)).or_default().push(AreaId(a));
        }
        BoardLayout {
            areas: area_map,
            adjacency,
            canal_crossings: canals.iter().map(|&(a, b)| (AreaId(a), AreaId(b))).collect(),
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        }
    }

    /// The four-area diamond every walking fixture below is built on:
    /// `s`(0) branches to `x`(1) and `y`(2), which rejoin at `d`(3). The mover's
    /// President starts on `s` with own assistants on x, y and d (a hop may only
    /// land where the mover already has an assistant).
    fn diamond(players: u8, canals: &[(u8, u8)]) -> GameState {
        let mut s = base_state(players, board(&[0, 1, 2, 3], &[(0, 1), (0, 2), (1, 3), (2, 3)], canals));
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId(0));
        for a in [1u8, 2, 3] {
            s.players[0].assistants.on_board.insert(AreaId(a), 1);
        }
        s
    }

    /// Give player 0 a technology with `effect`. Ids are allocated by effect so
    /// two calls can stack (Postal + Stagecoach) without colliding.
    fn grant_tech(state: &mut GameState, rules: &mut Rules, effect: TechEffect) {
        let (id, name) = match effect {
            TechEffect::PostalSystem => (TechCardId(101), "Postal System"),
            TechEffect::Stagecoach => (TechCardId(102), "Stagecoach"),
            _ => panic!("fixture only grants Postal/Stagecoach"),
        };
        rules.techs.insert(
            TechCard {
                id,
                name: name.to_string(),
                country: Country::Dutch,
                industry_cost: 2,
                effect,
            },
            name,
        );
        state.players[0].technologies.push(id);
    }

    /// THE equivalence assertion: `enumerate_routes` reproduces the frozen
    /// `enumerate_president_routes` EXACTLY — same pairs, same order, same
    /// lex-least witness per label — while genuinely taking the fixed-array path
    /// (a fixture that silently delegated would make the comparison vacuous),
    /// and the fixture is rich enough to discriminate (`min_routes`).
    fn assert_matches_frozen(state: &GameState, rules: &Rules, min_routes: usize, label: &str) {
        assert!(
            fits_fixed_arrays(state),
            "{label}: fixture must exercise the fixed-array search, not the delegation fallback",
        );
        let player = state.current_player;
        let PresidentLocation::OnBoard(from) = state.player(player).president_location else {
            panic!("{label}: walking-route fixtures need the President on the board");
        };
        let want = enumerate_president_routes(state, player, from, Some(&rules.techs));
        assert!(
            want.len() >= min_routes,
            "{label}: anti-vacuity — frozen enumerator produced {} routes, expected >= {min_routes}",
            want.len(),
        );
        assert_eq!(enumerate_routes(state, rules), want, "{label}: pairs and order must match");
    }

    // -- (a) exact equality against the frozen enumerator ---------------------

    #[test]
    fn plain_walk_matches_frozen_enumerator() {
        // No techs, no canals, no opponent President on the board: three free
        // destinations, one of which (d) is reached by two tying paths.
        let mut s = diamond(2, &[]);
        s.players[1].president_location = PresidentLocation::InHand;
        assert_matches_frozen(&s, &Rules::empty_for_tests(), 3, "plain");
    }

    #[test]
    fn postal_destination_fee_matches_frozen_enumerator() {
        // Blue's President sits on d(3). Without Postal d is unreachable but
        // still passable; WITH Postal it is a legal 1-Yen destination, and the
        // through-routes to x/y pick up the same fee as a second label each.
        let mut rules = Rules::empty_for_tests();
        let mut s = diamond(2, &[]);
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId(3));
        assert_matches_frozen(&s, &rules, 4, "blocked destination, no Postal");

        grant_tech(&mut s, &mut rules, TechEffect::PostalSystem);
        assert_matches_frozen(&s, &rules, 5, "Postal destination fee");
    }

    #[test]
    fn stagecoach_waives_fees_but_never_tolls() {
        // Blue blocks x(1); the s-y edge is a canal. Stagecoach waives the
        // pass-through fee through x but the y toll survives, so the two routes
        // into d carry DIFFERENT labels ({} and {None:1}) and both must live.
        let mut rules = Rules::empty_for_tests();
        let mut s = diamond(2, &[(0, 2)]);
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId(1));
        grant_tech(&mut s, &mut rules, TechEffect::Stagecoach);
        assert_matches_frozen(&s, &rules, 3, "Stagecoach");
    }

    #[test]
    fn postal_and_stagecoach_together_match_frozen_enumerator() {
        let mut rules = Rules::empty_for_tests();
        let mut s = diamond(2, &[(0, 2)]);
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId(1));
        grant_tech(&mut s, &mut rules, TechEffect::PostalSystem);
        grant_tech(&mut s, &mut rules, TechEffect::Stagecoach);
        assert_matches_frozen(&s, &rules, 6, "Postal + Stagecoach");
    }

    #[test]
    fn distinct_toll_counts_to_one_destination_are_distinct_labels() {
        // Arm x costs one toll (s-x), arm y costs two (s-y and y-d), so each
        // destination carries a {None:1} and a {None:2} label.
        let s = diamond(2, &[(0, 1), (0, 2), (2, 3)]);
        let mut s = s;
        s.players[1].president_location = PresidentLocation::InHand;
        assert_matches_frozen(&s, &Rules::empty_for_tests(), 6, "multi-toll");
    }

    #[test]
    fn two_opponents_on_distinct_areas_order_by_owner_bit() {
        // Blue on x(1), Yellow on y(2), Postal: d(3) is reached by a
        // {Some(Blue):1} route and a {Some(Yellow):1} route, which the frozen
        // enumerator emits in ascending-PlayerId order.
        let mut rules = Rules::empty_for_tests();
        let mut s = diamond(3, &[]);
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId(1));
        s.players[2].president_location = PresidentLocation::OnBoard(AreaId(2));
        grant_tech(&mut s, &mut rules, TechEffect::PostalSystem);
        assert_matches_frozen(&s, &rules, 4, "two owners, distinct areas");
    }

    #[test]
    fn two_opponents_sharing_one_area_charge_two_fees() {
        // Blue AND Yellow both on x(1) with Postal: entering x costs 2 Yen and
        // sets two owner bits in one hop.
        let mut rules = Rules::empty_for_tests();
        let mut s = diamond(3, &[]);
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId(1));
        s.players[2].president_location = PresidentLocation::OnBoard(AreaId(1));
        grant_tech(&mut s, &mut rules, TechEffect::PostalSystem);
        assert_matches_frozen(&s, &rules, 4, "two owners, one area");
    }

    #[test]
    fn affordability_prunes_labels_the_mover_cannot_pay() {
        // The two-toll labels of `distinct_toll_counts...` fall away at 1 Yen,
        // and everything falls away at 0.
        let mut s = diamond(2, &[(0, 1), (0, 2), (2, 3)]);
        s.players[1].president_location = PresidentLocation::InHand;

        s.players[0].money = 1;
        assert_matches_frozen(&s, &Rules::empty_for_tests(), 3, "money 1");

        s.players[0].money = 0;
        let rules = Rules::empty_for_tests();
        assert!(fits_fixed_arrays(&s));
        assert_eq!(
            enumerate_routes(&s, &rules),
            enumerate_president_routes(&s, PlayerId::new(0), AreaId(0), Some(&rules.techs)),
            "money 0: every first hop is tolled, so no route is affordable",
        );
    }

    // -- (b) the ported comparator against real BTreeMap semantics ------------

    /// Every `RouteVector` shape a two-opponent graph can produce, paired with
    /// its `(owner-mask, toll-count)` encoding. Owner bit 0 is PlayerId(1) and
    /// bit 1 is PlayerId(2) — the ascending-PlayerId assignment the enumerator
    /// makes — so the two representations denote the same label.
    fn comparator_cases() -> Vec<(RouteVector, (u8, u32))> {
        let p1 = Some(PlayerId::new(1));
        let p2 = Some(PlayerId::new(2));
        vec![
            (BTreeMap::new(), (0b00, 0)),
            (BTreeMap::from([(None, 1)]), (0b00, 1)),
            (BTreeMap::from([(None, 2)]), (0b00, 2)),
            (BTreeMap::from([(p1, 1)]), (0b01, 0)),
            (BTreeMap::from([(p2, 1)]), (0b10, 0)),
            (BTreeMap::from([(p1, 1), (p2, 1)]), (0b11, 0)),
            (BTreeMap::from([(None, 1), (p1, 1)]), (0b01, 1)),
            (BTreeMap::from([(None, 1), (p2, 1)]), (0b10, 1)),
            (BTreeMap::from([(None, 2), (p1, 1), (p2, 1)]), (0b11, 2)),
        ]
    }

    /// The PORTED comparator must agree with the real
    /// `BTreeMap<Option<PlayerId>, u32>` ordering the frozen enumerator's
    /// `BTreeMap<(AreaId, RouteVector), _>` iteration realises — checked against
    /// the maps themselves, not against a reading of their semantics. The named
    /// `{None:2}` vs `{None:1, Some(p):1}` pair (spec F2) is in the table: a
    /// naive `(dest, mask, tolls)` tuple compare gets it backwards.
    #[test]
    fn cmp_label_reproduces_route_vector_ordering() {
        let cases = comparator_cases();
        for (dest_a, dest_b) in [(3u8, 3u8), (3, 4), (4, 3)] {
            for (vec_a, (mask_a, tolls_a)) in &cases {
                for (vec_b, (mask_b, tolls_b)) in &cases {
                    let want = (AreaId(dest_a), vec_a.clone()).cmp(&(AreaId(dest_b), vec_b.clone()));
                    let got = cmp_label((dest_a, *mask_a, *tolls_a), (dest_b, *mask_b, *tolls_b));
                    assert_eq!(got, want, "{vec_a:?} vs {vec_b:?} at dests {dest_a}/{dest_b}");
                }
            }
        }
    }

    /// The specific pair the spec names, spelled out on its own so a regression
    /// reads as itself rather than as one row of the table above.
    #[test]
    fn cmp_label_orders_bank_toll_before_owner_fee() {
        let heavier_toll: RouteVector = BTreeMap::from([(None, 2)]);
        let toll_plus_fee: RouteVector = BTreeMap::from([(None, 1), (Some(PlayerId::new(1)), 1)]);
        assert_eq!(heavier_toll.cmp(&toll_plus_fee), Ordering::Greater);
        assert_eq!(cmp_label((3, 0b00, 2), (3, 0b01, 1)), Ordering::Greater);
        assert_eq!(cmp_label((3, 0b01, 1), (3, 0b00, 2)), Ordering::Less);
    }

    // -- (c) lex-least witness -----------------------------------------------

    #[test]
    fn tied_labels_keep_the_lexicographically_least_path() {
        // Both diamond arms are free, so [1,3] and [2,3] collide on the empty
        // vector: the lex-least witness ([1,3]) is the one kept.
        let mut s = diamond(2, &[]);
        s.players[1].president_location = PresidentLocation::InHand;
        let rules = Rules::empty_for_tests();
        assert_matches_frozen(&s, &rules, 3, "tie-break");

        let routes = enumerate_routes(&s, &rules);
        assert!(
            routes.contains(&(AreaId(3), vec![AreaId(1), AreaId(3)])),
            "lex-least tied path must be the witness: {routes:?}",
        );
        assert!(
            !routes.iter().any(|(d, p)| *d == AreaId(3) && p == &vec![AreaId(2), AreaId(3)]),
            "lex-greater tied path must be dropped: {routes:?}",
        );
    }

    #[test]
    fn lex_least_witness_can_be_the_longer_path() {
        // d(3) is reachable free by the two-hop [1,2,3] and the one-hop [4,3].
        // Lex order compares first hops, so the LONGER path wins — a witness
        // rule keyed on path length (or on discovery in board-file order) fails
        // here while passing every equal-length tie-break.
        let mut s = base_state(
            2,
            board(&[0, 1, 2, 3, 4], &[(0, 1), (1, 2), (2, 3), (0, 4), (4, 3)], &[]),
        );
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId(0));
        s.players[1].president_location = PresidentLocation::InHand;
        for a in [1u8, 2, 3, 4] {
            s.players[0].assistants.on_board.insert(AreaId(a), 1);
        }
        let rules = Rules::empty_for_tests();
        assert_matches_frozen(&s, &rules, 4, "long lex-least");

        let routes = enumerate_routes(&s, &rules);
        assert!(
            routes.contains(&(AreaId(3), vec![AreaId(1), AreaId(2), AreaId(3)])),
            "the longer lex-least path must win: {routes:?}",
        );
    }

    #[test]
    fn the_first_hop_is_priced_from_the_presidents_own_area() {
        // `route_fee_vector` tolls the `[president_location] ++ path` edge
        // sequence, so the first hop is priced from where the President STANDS,
        // not from the walk's start area. Nothing in the engine drives those
        // apart — `enumerate_routes` derives one from the other — but the frozen
        // enumerator's signature allows it, so the port must agree there too:
        // here the President sits on 0 with a canal on 0-1, while the walk
        // starts from 4, whose 4-1 edge is toll-free. That toll is what splits
        // the two arms into d(3) into DIFFERENT labels: price the first hop off
        // the start area instead and the arms collapse onto one free label,
        // dropping half the routes.
        let mut s = base_state(2, board(&[0, 1, 2, 3, 4], &[(4, 1), (4, 2), (1, 3), (2, 3)], &[(0, 1)]));
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId(0));
        s.players[1].president_location = PresidentLocation::InHand;
        for a in [1u8, 2, 3] {
            s.players[0].assistants.on_board.insert(AreaId(a), 1);
        }
        let rules = Rules::empty_for_tests();
        let from = AreaId(4);
        let want = enumerate_president_routes(&s, PlayerId::new(0), from, Some(&rules.techs));
        for arm in [vec![AreaId(1), AreaId(3)], vec![AreaId(2), AreaId(3)]] {
            assert!(
                want.contains(&(AreaId(3), arm.clone())),
                "both arms into d must survive as distinct labels: {want:?}",
            );
        }
        assert_eq!(walk_routes(&s, PlayerId::new(0), from, Some(&rules.techs)), want);
    }

    // -- (d) pre-flight and delegation ---------------------------------------

    /// A board whose area ids run past `MAX_AREAS`. `AreaId::new` refuses these
    /// (it asserts `id < 18`), so the tuple constructor is used deliberately:
    /// no generated board can reach this shape, and the fixture exists only to
    /// drive the pre-flight's delegation arm.
    fn oversized_board_state() -> GameState {
        let mut s = base_state(2, board(&[0, 18], &[(0, 18)], &[]));
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId(0));
        s.players[1].president_location = PresidentLocation::InHand;
        s.players[0].assistants.on_board.insert(AreaId(18), 1);
        s
    }

    #[test]
    fn oversized_board_delegates_to_the_frozen_enumerator() {
        let s = oversized_board_state();
        let rules = Rules::empty_for_tests();
        assert!(!fits_fixed_arrays(&s), "an AreaId past MAX_AREAS must fail the pre-flight");

        let want = enumerate_president_routes(&s, PlayerId::new(0), AreaId(0), Some(&rules.techs));
        assert_eq!(want, vec![(AreaId(18), vec![AreaId(18)])], "fixture must have a route to compare");
        assert_eq!(enumerate_routes(&s, &rules), want, "delegation must be output-identical");
    }

    #[test]
    fn pre_flight_rejects_each_unsupported_shape() {
        let base = || {
            let mut s = base_state(2, board(&[0, 1], &[(0, 1)], &[]));
            s.players[0].president_location = PresidentLocation::OnBoard(AreaId(0));
            s
        };
        assert!(fits_fixed_arrays(&base()), "the supported baseline must pass");

        // Out-degree past MAX_DEG (duplicated neighbours are enough — the bound
        // is on the raw adjacency list, which is what the search walks).
        let mut wide = base();
        wide.board_layout.adjacency.insert(AreaId(0), vec![AreaId(1); 18]);
        assert!(!fits_fixed_arrays(&wide), "an 18-entry adjacency list must fail");

        // More seats than the owner bitmask has bits.
        let mut crowded = base();
        for i in 2..5u8 {
            crowded.players.push(PlayerState::new(PlayerId(i), PlayerColor::Green, false));
        }
        assert!(!fits_fixed_arrays(&crowded), "five seats must fail");

        // A President parked past MAX_AREAS.
        let mut off_board = base();
        off_board.players[1].president_location = PresidentLocation::OnBoard(AreaId(20));
        assert!(!fits_fixed_arrays(&off_board), "a President past MAX_AREAS must fail");
    }

    #[test]
    fn every_generated_board_fits_the_fixed_arrays() {
        // Anti-vacuity for the delegation fallback: real play never takes it.
        let rules = Rules::standard();
        for players in [2u8, 3, 4] {
            for seed in 0..4u64 {
                let config = GameConfig::for_player_count(players);
                let pending = crate::new_game(config, &rules, seed).expect("new_game");
                let choices: Vec<crate::StartingOrderChoice> =
                    (0..players).map(|_| crate::StartingOrderChoice::Keep(0)).collect();
                let state = crate::finalize_setup(pending, &choices).expect("finalize_setup");
                assert!(
                    fits_fixed_arrays(&state),
                    "generated {players}p board (seed {seed}) must take the fixed-array path",
                );
            }
        }
    }

    // -- (e) driven games -----------------------------------------------------

    /// THE organic pin: at every OnBoard Movement decision point a real game
    /// reaches, `enumerate_routes` reproduces the frozen enumerator exactly.
    #[test]
    fn enumerate_routes_matches_frozen_across_driven_games() {
        use rand::{Rng, SeedableRng};
        let rules = Rules::standard();
        let mut checked = 0usize;
        let mut total_routes = 0usize;

        for players in [2u8, 3, 4] {
            for seed in 0..3u64 {
                let config = GameConfig::for_player_count(players);
                let pending = crate::new_game(config, &rules, seed).expect("new_game");
                let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
                let choices: Vec<crate::StartingOrderChoice> = (0..players)
                    .map(|_| crate::StartingOrderChoice::Keep(rng.gen_range(0..2u8)))
                    .collect();
                let mut state = crate::finalize_setup(pending, &choices).expect("finalize_setup");

                for _ in 0..1200 {
                    if crate::is_game_over(&state) {
                        break;
                    }
                    let player = state.current_player;
                    if state.pending_choices.is_empty()
                        && state.turn_phase == TurnPhase::Movement
                    {
                        if let PresidentLocation::OnBoard(from) =
                            state.player(player).president_location
                        {
                            assert!(
                                fits_fixed_arrays(&state),
                                "organic state must take the fixed-array path ({players}p seed {seed})",
                            );
                            let want = enumerate_president_routes(
                                &state,
                                player,
                                from,
                                Some(&rules.techs),
                            );
                            assert_eq!(
                                enumerate_routes(&state, &rules),
                                want,
                                "route mismatch at {players}p seed {seed}",
                            );
                            checked += 1;
                            total_routes += want.len();
                        }
                    }

                    let legal = crate::game::get_legal_moves(&state, &rules);
                    if legal.is_empty() {
                        break;
                    }
                    let mv = legal[rng.gen_range(0..legal.len())].clone();
                    state = crate::apply_move_unchecked(&state, &mv, &rules).expect("apply");
                }
            }
        }

        assert!(checked >= 50, "expected many OnBoard Movement points, saw {checked}");
        assert!(total_routes > 0, "every checked point enumerated zero routes — vacuous");
    }
}
