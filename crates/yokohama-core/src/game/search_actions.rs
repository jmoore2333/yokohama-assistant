//! Factored, lazy bot-facing action source over `&GameState` + `&Rules`
//! (Phase 3.1a; spec `docs/plans/2026-07-25-phase3.1-action-interface-spec.md`).
//!
//! Purely additive to [`get_legal_moves`](crate::game::get_legal_moves): that
//! exhaustive `Vec<Move>` surface is frozen and stays the authority
//! `apply_move` checks against. This module exists because the materialized
//! list has a reachable six-figure worst case — the President-recovery
//! count-vector product `3³·2¹⁴ = 442,368` moves — that allocates before
//! progressive widening can prune it. The factored source yields move
//! *families* first, then pages/samples within a family without ever building
//! the exploding `Vec`.
//!
//! The delivered 3.1a surface: [`families`] dispatch (queue-first + phase,
//! mirroring `get_legal_moves`), the bespoke lazy retreat family (mixed-radix
//! constructor/iter/sample — the six-figure family, never materialized), the
//! bespoke lazy placement family (combination construction with
//! per-combination affordability), and delegate-and-filter for the small
//! families. [`MoveFamily::MovementRoutes`] is the single exhaustive-backed
//! stub until 3.1b lands the lazy route enumerator. The plan's `ActionSource`
//! shipped as these free functions (the plan-sanctioned alternative), not a
//! struct — there is no `ActionSource` type to grep for.

use std::collections::BTreeMap;

use rand::Rng;

use crate::game::{calculate_placement_cost, get_legal_moves, Rules};
use crate::types::*;

/// A tag naming one decision-phase move family legal at the current state.
///
/// Returned by [`families`], which mirrors [`get_legal_moves`]'s queue-first +
/// `match turn_phase` dispatch (`legal_moves.rs:9-24`) but yields these tags
/// instead of moves; [`family_len`]/[`iter_family`]/[`sample_family`] page and
/// sample the actual moves per tag.
///
/// Only [`MoveFamily::MovementRetreat`] carries a payload: it owns a clone of
/// the current player's `on_board` assistant map so the exploding retreat
/// family can be indexed/sampled without re-reading `GameState`. Every other
/// family re-derives its moves from `&GameState` + `&Rules` at page time, so a
/// bare tag suffices — the owned snapshot is only paid where it removes a
/// borrow from the returned iterators.
///
/// PRECONDITION: a family is only meaningful against the `state` it was derived
/// from. The per-family functions take both, and the retreat arms
/// `debug_assert` that the snapshot still matches the current player's
/// `on_board` — serving a stale or foreign snapshot is a caller bug (the debug
/// assert makes it loud; in release the served moves would simply be rejected
/// by the checked `apply_move` boundary as a typed `MoveError`).
///
/// `MovementRoutes` covers **both** on-board President movement
/// (`Move::MovePresident`, including Station/Tram teleports) and placing the
/// President from hand (`Move::PlacePresident`) — the two ways the President
/// reaches the board. The exhaustive generator emits both from the single
/// `TurnPhase::Movement` branch and there is no separate place-President phase,
/// so they share one family tag. `MovementRetreat` is the
/// `Move::ReturnPresidentToHand` family, offered from either President location
/// (`from_hand` records which).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveFamily {
    /// Phase A/C additional actions: end-phase, mercenaries, fulfill
    /// order/achievement, University, Textile Mill.
    AdditionalAction,
    /// Placement step: Option A distinct-area k-subsets and Option B
    /// two-in-one-area (with the optional Telegram neighbour).
    Placement,
    /// Movement step, President-onto-board: `MovePresident` routes (President
    /// on board) or `PlacePresident` (President in hand). The lazy enumerator
    /// is 3.1b.
    MovementRoutes,
    /// Movement step, `ReturnPresidentToHand`: the per-area recovery-count
    /// product `∏(count + 1)` — the six-figure family. Carries an owned clone
    /// of the current player's `on_board` counts so it is indexable/sampleable
    /// without touching `GameState`; `from_hand` is true when the President is
    /// already in hand (Option B from hand) rather than on the board.
    MovementRetreat {
        on_board: BTreeMap<AreaId, u8>,
        from_hand: bool,
    },
    /// Area-action step: the six optional-component areas plus production and
    /// the Bank.
    AreaAction,
    /// 5-Power bonus step: take the token or skip.
    FivePower,
    /// Building step: build a shophouse / trading house, or skip.
    Building,
    /// A `PendingChoice` heads the queue: only its resolutions are legal
    /// (mirrors the `get_legal_moves` queue short-circuit).
    ChoiceResolution,
}

/// The move families legal at `state`, mirroring [`get_legal_moves`]'s
/// queue-first + phase dispatch (`legal_moves.rs:9-24`) but yielding family
/// tags rather than moves, in the same order `get_legal_moves` emits their
/// moves.
///
/// Structural by design: it emits the tag(s) the current phase dispatches to
/// and does **not** probe a family for emptiness. A degenerate state whose
/// phase-family happens to be empty still gets its tag — the family's own
/// paging yields the empty set there, so the per-family and union equivalences
/// (`⋃ iter_family == get_legal_moves`) still hold. `Recovery` and `TurnEnd`
/// are automatic/terminal and dispatch to no family.
///
/// `rules` is unused — the dispatch is a pure function of the turn phase, the
/// pending-choice queue, and the President's location — but is kept so every
/// entry point of this module shares the one `(&GameState, &Rules, …)` shape
/// ([`family_len`]/[`iter_family`]/[`sample_family`]).
pub fn families(state: &GameState, _rules: &Rules) -> Vec<MoveFamily> {
    // Queue short-circuit (mirrors `legal_moves.rs:9-11`): while a
    // `PendingChoice` heads the queue, the only legal moves are its
    // resolutions, whatever the turn phase.
    if !state.pending_choices.is_empty() {
        return vec![MoveFamily::ChoiceResolution];
    }

    match state.turn_phase {
        TurnPhase::AdditionalActionFirst | TurnPhase::AdditionalActionSecond => {
            vec![MoveFamily::AdditionalAction]
        }
        TurnPhase::Placement => vec![MoveFamily::Placement],
        TurnPhase::Movement => {
            // Both President locations emit a routes family (PlacePresident
            // from hand, MovePresident on board) followed by the retreat
            // family — the same order `generate_movement_moves_with_tech`
            // uses (`legal_moves.rs:254-373`).
            let p = state.current_player();
            vec![
                MoveFamily::MovementRoutes,
                MoveFamily::MovementRetreat {
                    on_board: p.assistants.on_board.clone(),
                    from_hand: matches!(p.president_location, PresidentLocation::InHand),
                },
            ]
        }
        TurnPhase::AreaAction => vec![MoveFamily::AreaAction],
        TurnPhase::FivePowerBonus => vec![MoveFamily::FivePower],
        TurnPhase::Building => vec![MoveFamily::Building],
        TurnPhase::Recovery | TurnPhase::TurnEnd => vec![],
    }
}

// ---------------------------------------------------------------------------
// Retreat family (Task 2): the `Move::ReturnPresidentToHand` recovery-count
// product `∏(count + 1)` over the president's on-board areas. The exhaustive
// generator materializes this vector (`enumerate_recovery_options`,
// `legal_moves.rs:382-400`); its 4P worst case is `3³·2¹⁴ = 442_368` moves.
// This factored source serves/pages/samples it with a direct constructor and a
// lazy mixed-radix decode, allocating only per served candidate — never the
// product. Every function filters `count > 0` first, exactly as the generator's
// `occupied` does, so a snapshot carrying a stray count-0 entry (radix 1,
// contributes nothing) stays equivalent.
// ---------------------------------------------------------------------------

/// Build one [`Move::ReturnPresidentToHand`] from explicit per-area recovery
/// counts, dropping any zero-count pair (recovering 0 from an area is the same
/// as omitting it, matching how `enumerate_recovery_options` never inserts a
/// 0). No enumeration and **no validation**: the assembled `recover` map goes
/// straight into the move, and `apply_return_to_hand` (`apply_move.rs:634`) is
/// the sole validator — an over-count pair is left for that arm to reject with
/// `RuleViolation`, not pre-checked here (the frozen apply boundary owns
/// legality, per the plan's read-only constraint).
///
/// `on_board` is unused by construction — a `recover` map is fully determined
/// by the chosen `per_area` counts — and is present only so the retreat
/// family's four entry points (`retreat_move` / [`retreat_len`] /
/// [`retreat_iter`] / [`retreat_sample`]) share one shape over the family's
/// owned `on_board` snapshot.
pub fn retreat_move(_on_board: &BTreeMap<AreaId, u8>, per_area: &[(AreaId, u8)]) -> Move {
    let recover: BTreeMap<AreaId, u8> =
        per_area.iter().copied().filter(|&(_, count)| count > 0).collect();
    Move::ReturnPresidentToHand { recover }
}

/// Size of the retreat family for `on_board`: `∏ (count + 1)` over the occupied
/// areas (count > 0). Each area independently contributes a "recover 0..=count"
/// choice — including the recover-nothing and recover-everything extremes — so
/// this equals `enumerate_recovery_options(player).len()`
/// (`legal_moves.rs:382-400`), the product this source never materializes (4P
/// worst case `3³·2¹⁴ = 442_368`). An empty `on_board` yields 1 (the lone
/// recover-nothing move).
pub fn retreat_len(on_board: &BTreeMap<AreaId, u8>) -> usize {
    on_board
        .values()
        .filter(|&&count| count > 0)
        .map(|&count| count as usize + 1)
        .product()
}

/// Lazily enumerate the retreat family in the **same order and with the same
/// `recover` maps** as
/// `enumerate_recovery_options(player).map(|recover| ReturnPresidentToHand { recover })`
/// (`legal_moves.rs:274,370,382-400`), without ever building the
/// `∏(count+1)`-element vector.
///
/// Index `i ∈ 0..retreat_len(on_board)` is decoded as a mixed-radix number over
/// the occupied areas (count > 0) in ascending `AreaId` order, where the LAST
/// such area is the least-significant digit. That reproduces the generator's
/// fold exactly: it seeds `[{}]` and, for each area in ascending order, replaces
/// the running set with `running × (0..=count)` — so the last-folded (largest)
/// area varies fastest and the first-folded (smallest) slowest. Each served
/// candidate allocates only its own `≤17`-entry `recover` map (plus the
/// one-time `≤17`-entry radix vector), never `O(product)`.
///
/// The returned iterator overrides `nth`, so paging deep into the family skips
/// in O(1) and decodes exactly one candidate — a plain `(0..len).map(decode)`
/// would not: `Map` inherits the default `Iterator::nth`, which invokes the
/// decode for every *skipped* index and reintroduces the O(product) allocation
/// this source exists to avoid.
pub fn retreat_iter(on_board: &BTreeMap<AreaId, u8>) -> impl Iterator<Item = Move> {
    // Occupied areas in ascending AreaId order, count > 0 — identical to
    // `enumerate_recovery_options`' `occupied`. Element 0 is the
    // most-significant digit; the last element the least-significant.
    let areas: Vec<(AreaId, u8)> = on_board
        .iter()
        .filter(|&(_, &count)| count > 0)
        .map(|(&area, &count)| (area, count))
        .collect();
    let len = areas.iter().map(|&(_, count)| count as usize + 1).product::<usize>();
    RetreatIter { areas, len, index: 0 }
}

/// Decode index `i` (in `0..family_size`) into one `recover` map under the
/// last-area-LSB mixed-radix convention. `areas` is the occupied set in
/// ascending `AreaId` order (element 0 = most-significant digit), so walking it
/// back-to-front takes the least-significant digit first — reproducing the
/// generator's ascending-order fold (`legal_moves.rs:382-400`).
fn decode_retreat(areas: &[(AreaId, u8)], mut index: usize) -> BTreeMap<AreaId, u8> {
    let mut recover = BTreeMap::new();
    for &(area, count) in areas.iter().rev() {
        let radix = count as usize + 1;
        let digit = (index % radix) as u8;
        index /= radix;
        if digit > 0 {
            recover.insert(area, digit);
        }
    }
    recover
}

/// Lazy retreat enumerator returned by [`retreat_iter`]. Holds only the
/// `≤17`-entry radix vector and a cursor; each yielded item decodes one index.
struct RetreatIter {
    areas: Vec<(AreaId, u8)>,
    len: usize,
    index: usize,
}

impl Iterator for RetreatIter {
    type Item = Move;

    fn next(&mut self) -> Option<Move> {
        if self.index >= self.len {
            return None;
        }
        let recover = decode_retreat(&self.areas, self.index);
        self.index += 1;
        Some(Move::ReturnPresidentToHand { recover })
    }

    /// O(1) skip: advance the cursor by `n` and decode the single element
    /// landed on. This is the crux of the no-materialization guarantee — the
    /// default `nth` would decode all `n` skipped indices.
    fn nth(&mut self, n: usize) -> Option<Move> {
        self.index = self.index.saturating_add(n);
        self.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len - self.index.min(self.len);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for RetreatIter {}

/// Draw one retreat move by an independent per-area draw: each occupied area
/// recovers a count drawn uniform in `0..=count`. Because the family is the
/// exact Cartesian product of those per-area ranges, independent uniform digits
/// make this **uniform over the whole `∏(count+1)` family** (each vector has
/// probability `∏ 1/(count+1) = 1/retreat_len`) — while doing only `O(#areas)`
/// work and allocation, never touching the product. Every draw is legal by
/// construction (0 = recover nothing there, `count` = recover all), so it is
/// rejection-free and always yields a move applicable to the state the
/// `on_board` snapshot was taken from (a stale snapshot's draw is caught by the
/// checked `apply_move` boundary, per the [`MoveFamily`] precondition).
pub fn retreat_sample(on_board: &BTreeMap<AreaId, u8>, rng: &mut impl Rng) -> Move {
    let mut recover = BTreeMap::new();
    for (&area, &count) in on_board.iter().filter(|&(_, &count)| count > 0) {
        let drawn = rng.gen_range(0..=count);
        if drawn > 0 {
            recover.insert(area, drawn);
        }
    }
    Move::ReturnPresidentToHand { recover }
}

// ---------------------------------------------------------------------------
// Placement family (Task 3): Option A distinct-area k-subsets and Option B
// two-in-one-area (plus the optional Telegram neighbour). The exhaustive
// generator materializes both blocks (`generate_placement_moves_with_tech`,
// `legal_moves.rs:115-246`); this factored source reproduces them lazily and in
// the same order. The one structural difference from the clean-product retreat
// family above is the affordability gate: `calculate_placement_cost`
// (`legal_moves.rs:99`) sums 1 Yen per (placed-area, opponent-president) pair
// over ALL placed areas, so a k-subset of individually-affordable areas can
// still exceed money and must be filtered as a WHOLE, per combination -- not per
// area. Two consequences, both honored below: [`placement_upper_len`] is an
// UPPER bound (affordable subsets are not contiguous in the enumeration order,
// so no cheap closed form gives the exact post-filter count), and
// [`placement_sample`] is NOT rejection-free.
// ---------------------------------------------------------------------------

/// The static inputs both placement paging and counting read off `state`+`rules`
/// once. `placeable` is the placement-allowing areas in ascending `AreaId` order
/// -- the combination index space, matching the generator's BTreeMap-ordered
/// `placeable_areas`. `k_max = min(n, max_areas, in_hand)` bounds the Option-A
/// subset size (`max_areas` = 4 with Newspaper else 3). `option_b`
/// (`in_hand >= 2`) and `telegram_bonus` (Telegram owned AND `in_hand >= 3`)
/// gate the Option-B block and its neighbour bonus.
struct PlacementParams {
    placeable: Vec<AreaId>,
    player: PlayerId,
    money: u32,
    k_max: usize,
    option_b: bool,
    telegram_bonus: bool,
}

fn placement_params(state: &GameState, rules: &Rules) -> PlacementParams {
    let player = state.current_player;
    let p = state.player(player);
    let in_hand = p.assistants.in_hand as usize;

    let has_effect = |effect: TechEffect| {
        p.technologies
            .iter()
            .any(|&tech_id| rules.techs.get(tech_id).is_some_and(|t| t.effect == effect))
    };
    let max_areas = if has_effect(TechEffect::Newspaper) { 4 } else { 3 };

    let placeable: Vec<AreaId> = state
        .board_layout
        .areas
        .iter()
        .filter(|(_, config)| config.area_type.allows_placement())
        .map(|(&id, _)| id)
        .collect();

    let k_max = placeable.len().min(max_areas).min(in_hand);

    PlacementParams {
        placeable,
        player,
        money: p.money,
        k_max,
        option_b: in_hand >= 2,
        telegram_bonus: has_effect(TechEffect::Telegram) && in_hand >= 3,
    }
}

/// Build one [`Move::PlaceThreeAreas`] (Option A) from a combination expressed
/// as ascending indices into `areas_sorted` (the placeable list). No
/// validation: distinctness/cap/affordability are the `apply_placement` arm's
/// job (`apply_move.rs:428`), exactly as the generator emits the vector then
/// leaves the arm to re-check. An empty `selection` is the always-legal empty
/// placement.
pub fn place_combo(areas_sorted: &[AreaId], selection: &[usize]) -> Move {
    Move::PlaceThreeAreas { placements: selection.iter().map(|&i| areas_sorted[i]).collect() }
}

/// Upper bound on the placement family size: `Σ_{k=0}^{k_max} C(n, k)` for
/// Option A plus Option B's structural count, WITHOUT subtracting the
/// per-combination affordability holes (an over-budget subset is still counted).
/// Exact when every combination is affordable -- no opponent president on a
/// placed area, or enough money to pay every fee -- and a strict over-estimate
/// once a fee gate punches a hole. This is the honest `family_len` the spec
/// (§3.4) calls for; [`placement_iter`] is the authority on the realized moves.
///
/// The Option-B term counts one basic `PlaceTwoSameArea` per placeable area plus
/// (Telegram owned, `in_hand >= 3`) each non-crossing placeable neighbour -- the
/// fee-independent shape of the generator's Option-B block, so it too is
/// exact-when-affordable and an upper bound otherwise.
pub fn placement_upper_len(state: &GameState, rules: &Rules) -> usize {
    let params = placement_params(state, rules);
    let n = params.placeable.len();

    // Option A: every k-subset for k in 0..=k_max, fees ignored.
    let mut upper: usize = (0..=params.k_max).map(|k| binomial(n, k)).sum();

    // Option B: one basic per area + each Telegram non-crossing placeable
    // neighbour (structural, fee-independent).
    if params.option_b {
        upper += n;
        if params.telegram_bonus {
            for &area in &params.placeable {
                for &adj in state.board_layout.adjacent_areas(area) {
                    let placeable_adj = state
                        .board_layout
                        .areas
                        .get(&adj)
                        .is_some_and(|c| c.area_type.allows_placement());
                    if placeable_adj && !state.board_layout.is_canal_crossing(area, adj) {
                        upper += 1;
                    }
                }
            }
        }
    }

    upper
}

/// `C(n, k)` by the multiplicative recurrence (stays integral each step, no
/// factorial overflow). Here `n <= 17`, `k <= 4`, far from `usize` limits.
fn binomial(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result = 1usize;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}

/// Lazily enumerate the placement family in the **same order and with the same
/// moves** as `get_legal_moves` at a `Placement` state, filtered to
/// `PlaceThreeAreas`/`PlaceTwoSameArea`: Option A first (all k-subsets, `k`
/// ascending `0..=k_max`, each `k`'s subsets in the generator's ascending-index
/// `j < k < l` loop order, `legal_moves.rs:147-195`), then Option B per
/// placeable area (basic then Telegram neighbours, `legal_moves.rs:207-243`).
///
/// **Generate-and-filter, never materialized.** Each candidate combination is
/// built one at a time and dropped when `calculate_placement_cost` exceeds money
/// (the per-COMBINATION gate -- see the module note), exactly as the generator's
/// inline `if player_money >= cost` does, so the emitted sequence equals the
/// generator's post-filter output move-for-move without ever collecting the
/// `Σ C(n,k)` combinations. The empty (`k = 0`) placement is always emitted
/// first (cost 0), which also covers the generator's `in_hand == 0` degenerate
/// case: `k_max` collapses to 0 and Option B is gated off, leaving exactly the
/// lone empty placement.
pub fn placement_iter<'a>(state: &'a GameState, rules: &Rules) -> impl Iterator<Item = Move> + 'a {
    let params = placement_params(state, rules);
    PlacementIter { state, params, stage: PlacementStage::OptionA { k: 0, combo: None } }
}

/// Lazy placement enumerator returned by [`placement_iter`]. Holds the
/// `&GameState` (for `calculate_placement_cost` and adjacency at page time), the
/// once-computed [`PlacementParams`], and a cursor through Option A's per-`k`
/// combinations then Option B's per-area basic/neighbour moves.
struct PlacementIter<'a> {
    state: &'a GameState,
    params: PlacementParams,
    stage: PlacementStage,
}

/// Cursor position for [`PlacementIter`]. `OptionA.combo == None` means "emit the
/// first combination of size `k` next"; `Some(v)` is the last-yielded
/// combination whose successor comes next. `OptionB.sub` tracks basic-vs-
/// neighbour within the current area.
enum PlacementStage {
    OptionA { k: usize, combo: Option<Vec<usize>> },
    OptionB { area_idx: usize, sub: OptionBSub },
    Done,
}

enum OptionBSub {
    Basic,
    Neighbor(usize),
}

impl Iterator for PlacementIter<'_> {
    type Item = Move;

    fn next(&mut self) -> Option<Move> {
        loop {
            // Take the stage out so each arm can freely read `self.state` /
            // `self.params` (disjoint fields) and reassign `self.stage` without
            // a borrow conflict; every arm restores it before returning/looping.
            match std::mem::replace(&mut self.stage, PlacementStage::Done) {
                PlacementStage::OptionA { mut k, combo } => {
                    let n = self.params.placeable.len();
                    let next = match combo {
                        None => first_combination(n, k),
                        Some(mut v) => advance_combination(&mut v, n).then_some(v),
                    };
                    match next {
                        Some(indices) => {
                            let placements: Vec<AreaId> =
                                indices.iter().map(|&i| self.params.placeable[i]).collect();
                            self.stage = PlacementStage::OptionA { k, combo: Some(indices) };
                            let cost =
                                calculate_placement_cost(self.state, self.params.player, &placements);
                            if self.params.money >= cost {
                                return Some(Move::PlaceThreeAreas { placements });
                            }
                            // Over budget: cursor kept, try the next combination.
                        }
                        None => {
                            // This k is exhausted: advance k, else fall through to
                            // Option B, else finish.
                            if k < self.params.k_max {
                                k += 1;
                                self.stage = PlacementStage::OptionA { k, combo: None };
                            } else if self.params.option_b {
                                self.stage =
                                    PlacementStage::OptionB { area_idx: 0, sub: OptionBSub::Basic };
                            } else {
                                self.stage = PlacementStage::Done;
                            }
                        }
                    }
                }
                PlacementStage::OptionB { area_idx, sub } => {
                    if area_idx >= self.params.placeable.len() {
                        self.stage = PlacementStage::Done;
                        continue;
                    }
                    let area = self.params.placeable[area_idx];
                    // Two assistants land in `area`: price the 2-element slice,
                    // matching the generator (`legal_moves.rs:212`).
                    let basic_cost =
                        calculate_placement_cost(self.state, self.params.player, &[area, area]);
                    if self.params.money < basic_cost {
                        // Basic unaffordable => the whole area (basic AND its
                        // Telegram bonuses) is skipped, exactly as the generator's
                        // per-area `if player_money >= cost` block gate does.
                        self.stage = PlacementStage::OptionB {
                            area_idx: area_idx + 1,
                            sub: OptionBSub::Basic,
                        };
                        continue;
                    }
                    match sub {
                        OptionBSub::Basic => {
                            self.stage = PlacementStage::OptionB {
                                area_idx,
                                sub: OptionBSub::Neighbor(0),
                            };
                            return Some(Move::PlaceTwoSameArea { area, adjacent_bonus: None });
                        }
                        OptionBSub::Neighbor(j) => {
                            if !self.params.telegram_bonus {
                                self.stage = PlacementStage::OptionB {
                                    area_idx: area_idx + 1,
                                    sub: OptionBSub::Basic,
                                };
                                continue;
                            }
                            let neighbors = self.state.board_layout.adjacent_areas(area);
                            if j >= neighbors.len() {
                                self.stage = PlacementStage::OptionB {
                                    area_idx: area_idx + 1,
                                    sub: OptionBSub::Basic,
                                };
                                continue;
                            }
                            let adj = neighbors[j];
                            self.stage = PlacementStage::OptionB {
                                area_idx,
                                sub: OptionBSub::Neighbor(j + 1),
                            };
                            // Canal crossings are movement-transit edges only,
                            // never a Telegram bonus target (`legal_moves.rs:223`).
                            if self.state.board_layout.is_canal_crossing(area, adj) {
                                continue;
                            }
                            let adj_placeable = self
                                .state
                                .board_layout
                                .areas
                                .get(&adj)
                                .is_some_and(|c| c.area_type.allows_placement());
                            if !adj_placeable {
                                continue;
                            }
                            let adj_cost =
                                calculate_placement_cost(self.state, self.params.player, &[adj]);
                            if self.params.money >= basic_cost + adj_cost {
                                return Some(Move::PlaceTwoSameArea {
                                    area,
                                    adjacent_bonus: Some(adj),
                                });
                            }
                            // Bonus over budget: cursor already advanced, retry.
                        }
                    }
                }
                PlacementStage::Done => return None,
            }
        }
    }
}

/// The first (lex-least) size-`k` combination of `0..n`: `[0, 1, …, k-1]`, or
/// `None` if `k > n`. `k = 0` yields the empty combination (the always-legal
/// empty placement).
fn first_combination(n: usize, k: usize) -> Option<Vec<usize>> {
    (k <= n).then(|| (0..k).collect())
}

/// Advance `combo` (a strictly-ascending size-`k` combination of `0..n`) to its
/// lex successor in place, returning `false` when it is already the last
/// (`[n-k, …, n-1]`, or the empty combination). Reproduces the generator's
/// nested ascending-index loops (`legal_moves.rs:159-195`): bump the rightmost
/// index that can still grow, then reset every index after it to the tightest
/// ascending run.
fn advance_combination(combo: &mut [usize], n: usize) -> bool {
    let k = combo.len();
    if k == 0 {
        return false;
    }
    let mut i = k - 1;
    loop {
        if combo[i] < n - k + i {
            combo[i] += 1;
            for j in (i + 1)..k {
                combo[j] = combo[j - 1] + 1;
            }
            return true;
        }
        if i == 0 {
            return false;
        }
        i -= 1;
    }
}

/// Draw one uniformly-random legal placement move. Unlike [`retreat_sample`],
/// placement is NOT a clean product space: affordability is a per-COMBINATION
/// gate (the summed fee over all placed areas), so an O(1) draw of a
/// per-area-affordable subset can land over budget. Rather than reject-and-
/// redraw (unbounded worst case, though the always-free empty placement
/// guarantees eventual termination), this reservoir-samples the affordability-
/// filtered [`placement_iter`] in a single lazy pass -- keeping the i-th yielded
/// move with probability 1/i. That is uniform over the whole realized family,
/// always returns a legal move (every candidate already passed the cost filter),
/// and runs in O(family size) time / O(1) space without materializing the
/// family. (An affordability-aware O(1) unrank that skips holes is a 3.3
/// widening concern, per spec §3.4.)
pub fn placement_sample(state: &GameState, rules: &Rules, rng: &mut impl Rng) -> Move {
    let mut chosen: Option<Move> = None;
    for (i, mv) in placement_iter(state, rules).enumerate() {
        // Reservoir of size 1: the (i+1)-th candidate replaces the running pick
        // with probability 1/(i+1), leaving each candidate equally likely.
        if rng.gen_range(0..=i) == 0 {
            chosen = Some(mv);
        }
    }
    // placement_iter always yields at least the empty (k = 0) placement.
    chosen.expect("placement family is never empty (the empty placement is always legal)")
}

// ---------------------------------------------------------------------------
// Unified dispatch (Task 4): family_len / iter_family / sample_family over a
// `MoveFamily`. The two exploding families are bespoke -- retreat via the
// mixed-radix constructor (Task 2), placement via lazy combination
// construction with per-combination affordability (Task 3). EVERY other family
// DELEGATES to the frozen `get_legal_moves`, filtered to the family by
// `family_kind`: that is equivalence by construction -- the yielded moves ARE a
// contiguous filter of the generator's own output, so they inherit its order
// and multiset exactly, including FulfillOrder's `order_cards_in_hand`
// insertion order (`legal_moves.rs:59`), never a sorted key. Those families are
// bounded-small, so materializing the filtered slice is fine (the laziness
// guarantee only binds retreat/placement). `MovementRoutes` is a 3.1b STUB that
// delegates the same way (filter to MovePresident/PlacePresident) so the union
// of families stays complete for the equivalence tests; 3.1b replaces it with a
// lazy route enumerator. The stub's cost, stated honestly: the delegating call
// re-runs the WHOLE `get_legal_moves` before filtering, so at a Movement
// decision point the routes stub re-pays the exhaustive ceiling — retreat
// product included. The serve-one bound therefore holds PER FAMILY; the
// per-Movement-decision-point ceiling is unchanged until 3.1b.
// ---------------------------------------------------------------------------

/// The [`MoveFamily`] a given [`Move`] belongs to, as a discriminant so the
/// payload-carrying `MovementRetreat` compares by variant only. Exhaustive on
/// purpose (no `_` arm): a new `Move` variant must be classified here or this
/// stops compiling. Used by the delegate-and-filter dispatch to slice
/// `get_legal_moves` down to one family, and by the tests to check membership.
fn family_kind(mv: &Move) -> std::mem::Discriminant<MoveFamily> {
    use std::mem::discriminant as d;
    match mv {
        Move::UseMercenary { .. }
        | Move::UseUniversity
        | Move::UseSpinningMill
        | Move::FulfillAchievement { .. }
        | Move::FulfillOrder { .. }
        | Move::EndAdditionalPhase => d(&MoveFamily::AdditionalAction),

        Move::PlaceThreeAreas { .. } | Move::PlaceTwoSameArea { .. } => d(&MoveFamily::Placement),

        Move::MovePresident { .. } | Move::PlacePresident { .. } => d(&MoveFamily::MovementRoutes),

        Move::ReturnPresidentToHand { .. } => {
            d(&MoveFamily::MovementRetreat { on_board: BTreeMap::new(), from_hand: false })
        }

        Move::ExecuteAreaAction { .. } => d(&MoveFamily::AreaAction),

        Move::Take5PowerToken | Move::Skip5PowerBonus => d(&MoveFamily::FivePower),

        Move::BuildShophouse { .. } | Move::BuildTradingHouse | Move::SkipBuilding => {
            d(&MoveFamily::Building)
        }

        Move::ChooseAnyGood { .. }
        | Move::ChooseWarehouseTake { .. }
        | Move::DeclineWarehouseTake
        | Move::ChooseAssistantMove { .. }
        | Move::DeclineAssistantMove
        | Move::ChooseChurchSpace { .. }
        | Move::ChooseCustomsSpace { .. }
        | Move::ChooseRecoveryLeave { .. }
        | Move::ChooseStationPlace { .. } => d(&MoveFamily::ChoiceResolution),
    }
}

/// The moves `get_legal_moves` emits that belong to `family`, in the generator's
/// order. This is the delegate-and-filter path shared by the small families and
/// the `MovementRoutes` 3.1b stub: it re-reads the frozen generator and keeps
/// the slice whose [`family_kind`] matches `family`, so the result is the
/// generator's own output move-for-move (equivalence by construction). Retreat
/// and placement never reach here -- they are served by their bespoke lazy
/// generators.
fn delegated_family_moves(state: &GameState, rules: &Rules, family: &MoveFamily) -> Vec<Move> {
    let want = std::mem::discriminant(family);
    get_legal_moves(state, rules).into_iter().filter(|mv| family_kind(mv) == want).collect()
}

/// Size of `family` at `state`: the bespoke closed form for retreat
/// (`∏(count+1)`, [`retreat_len`]) and the honest upper bound for placement
/// ([`placement_upper_len`], a strict over-estimate once a fee gate punches a
/// hole); the exact realized count for every delegated family. Equals the
/// number of moves [`iter_family`] yields for every family EXCEPT placement,
/// where it is an upper bound (`iter_family` is the authority on the realized
/// placement moves).
pub fn family_len(state: &GameState, rules: &Rules, family: &MoveFamily) -> usize {
    match family {
        MoveFamily::MovementRetreat { on_board, .. } => {
            debug_assert_eq!(
                on_board,
                &state.current_player().assistants.on_board,
                "MoveFamily::MovementRetreat used against a state it was not derived from",
            );
            retreat_len(on_board)
        }
        MoveFamily::Placement => placement_upper_len(state, rules),
        MoveFamily::MovementRoutes
        | MoveFamily::AdditionalAction
        | MoveFamily::AreaAction
        | MoveFamily::FivePower
        | MoveFamily::Building
        | MoveFamily::ChoiceResolution => delegated_family_moves(state, rules, family).len(),
    }
}

/// Lazily yield `family`'s moves at `state`, in the same order and with the same
/// moves as `get_legal_moves` filtered to that family. Retreat and placement
/// page/serve lazily (Tasks 2/3) without materializing the family; every other
/// family delegates to the frozen generator (a materialized but bounded-small
/// slice — though the delegating call itself re-runs the WHOLE generator, so
/// the routes stub re-pays the exhaustive ceiling until 3.1b; see the module
/// note above). Boxed because the arms have heterogeneous iterator types (the
/// lazy `RetreatIter`/`PlacementIter` and the delegated `vec::IntoIter`).
pub fn iter_family<'a>(
    state: &'a GameState,
    rules: &Rules,
    family: &MoveFamily,
) -> Box<dyn Iterator<Item = Move> + 'a> {
    match family {
        MoveFamily::MovementRetreat { on_board, .. } => {
            debug_assert_eq!(
                on_board,
                &state.current_player().assistants.on_board,
                "MoveFamily::MovementRetreat used against a state it was not derived from",
            );
            Box::new(retreat_iter(on_board))
        }
        MoveFamily::Placement => Box::new(placement_iter(state, rules)),
        MoveFamily::MovementRoutes
        | MoveFamily::AdditionalAction
        | MoveFamily::AreaAction
        | MoveFamily::FivePower
        | MoveFamily::Building
        | MoveFamily::ChoiceResolution => {
            Box::new(delegated_family_moves(state, rules, family).into_iter())
        }
    }
}

/// Draw one uniformly-random legal move from `family` at `state`, or `None`
/// when the family is empty at this decision point. Retreat and placement use
/// their bespoke samplers (Task 2's rejection-free per-area draw; Task 3's
/// affordability-aware reservoir) and are never empty (the recover-nothing and
/// empty-placement moves are always legal), so those arms always return `Some`;
/// a delegated family draws a uniform index into its materialized legal slice
/// and yields `None` when the slice is empty — [`families`] deliberately tags
/// structurally-emptyable families (e.g. `MovementRoutes` with no Possible
/// Action Area to enter), and `Option` puts that case in the signature instead
/// of a panic or a redundant [`family_len`] pre-check (which would materialize
/// a delegated family twice).
///
/// (Deviation from spec §3.2's literal `-> Move` signature, adopted at the PR
/// #8 review: the spec predates the observation that `families()`'s structural
/// dispatch emits organically-reachable empty route families.)
pub fn sample_family(
    state: &GameState,
    rules: &Rules,
    family: &MoveFamily,
    rng: &mut impl Rng,
) -> Option<Move> {
    match family {
        MoveFamily::MovementRetreat { on_board, .. } => {
            debug_assert_eq!(
                on_board,
                &state.current_player().assistants.on_board,
                "MoveFamily::MovementRetreat used against a state it was not derived from",
            );
            Some(retreat_sample(on_board, rng))
        }
        MoveFamily::Placement => Some(placement_sample(state, rules, rng)),
        MoveFamily::MovementRoutes
        | MoveFamily::AdditionalAction
        | MoveFamily::AreaAction
        | MoveFamily::FivePower
        | MoveFamily::Building
        | MoveFamily::ChoiceResolution => {
            let mut moves = delegated_family_moves(state, rules, family);
            if moves.is_empty() {
                return None;
            }
            let i = rng.gen_range(0..moves.len());
            Some(moves.swap_remove(i))
        }
    }
}

/// Thread-local counting allocator for the no-materialization bound
/// ([`tests::serving_one_retreat_candidate_is_o_areas_not_o_product`]). It
/// forwards every (de)allocation to the process `System` allocator unchanged
/// and, only while the *current thread* has armed it via [`count_allocs`],
/// increments a thread-local counter — so the count is immune to the
/// allocations other `cargo test` threads make in parallel. Const-initialized
/// thread-locals never allocate on access (no lazy init, no destructor), so the
/// counter can be read from inside `alloc` without re-entering it. This lives
/// only in the `--lib` unit-test binary (`#[cfg(test)]`); integration tests
/// link the crate without it. Task 6 adds the release-tier byte/latency meter;
/// this is the per-edit COUNT bound.
#[cfg(test)]
mod alloc_probe {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static ARMED: Cell<bool> = const { Cell::new(false) };
        static ALLOCS: Cell<usize> = const { Cell::new(0) };
    }

    struct Counting;

    // SAFETY: every method forwards to the process `System` allocator with the
    // same pointer/layout, so the `GlobalAlloc` contract is upheld unchanged;
    // the only added work is a non-allocating thread-local counter bump.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            bump();
            System.alloc(layout)
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout)
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            bump();
            System.alloc_zeroed(layout)
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            bump();
            System.realloc(ptr, layout, new_size)
        }
    }

    /// Count this allocation if the current thread armed the probe. `try_with`
    /// tolerates access during thread teardown (returns without counting).
    fn bump() {
        if ARMED.try_with(Cell::get).unwrap_or(false) {
            let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
        }
    }

    #[global_allocator]
    static GLOBAL: Counting = Counting;

    /// Run `f` with allocation counting armed on THIS thread; return its value
    /// and the number of allocations (alloc + zeroed + realloc) it made.
    pub fn count_allocs<R>(f: impl FnOnce() -> R) -> (R, usize) {
        ALLOCS.with(|n| n.set(0));
        ARMED.with(|a| a.set(true));
        let r = f();
        ARMED.with(|a| a.set(false));
        (r, ALLOCS.with(Cell::get))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply_move_unchecked;
    use crate::game::get_legal_moves;

    /// Minimal 2-player state (mirrors `legal_moves::tests::create_test_state`)
    /// with a two-area board — enough for `get_legal_moves` to produce real
    /// moves to cross-check the family tags against. Defaults to `Placement`.
    fn base_state() -> GameState {
        let config = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));

        let mut areas = BTreeMap::new();
        areas.insert(AreaId::new(0), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(AreaId::new(1), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });

        let board_layout = BoardLayout {
            areas,
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
            turn_phase: TurnPhase::Placement,
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

    fn at_phase(phase: TurnPhase) -> GameState {
        let mut s = base_state();
        s.turn_phase = phase;
        s
    }

    #[test]
    fn families_match_phase_dispatch() {
        let rules = Rules::empty_for_tests();

        // Single-family phases.
        assert_eq!(families(&at_phase(TurnPhase::Placement), &rules), vec![MoveFamily::Placement]);
        assert_eq!(
            families(&at_phase(TurnPhase::AdditionalActionFirst), &rules),
            vec![MoveFamily::AdditionalAction],
        );
        assert_eq!(
            families(&at_phase(TurnPhase::AdditionalActionSecond), &rules),
            vec![MoveFamily::AdditionalAction],
        );
        assert_eq!(families(&at_phase(TurnPhase::AreaAction), &rules), vec![MoveFamily::AreaAction]);
        assert_eq!(
            families(&at_phase(TurnPhase::FivePowerBonus), &rules),
            vec![MoveFamily::FivePower],
        );
        assert_eq!(families(&at_phase(TurnPhase::Building), &rules), vec![MoveFamily::Building]);

        // Automatic / terminal phases dispatch to no family.
        assert_eq!(families(&at_phase(TurnPhase::Recovery), &rules), vec![]);
        assert_eq!(families(&at_phase(TurnPhase::TurnEnd), &rules), vec![]);
    }

    #[test]
    fn movement_yields_routes_then_retreat_with_snapshot() {
        let rules = Rules::empty_for_tests();

        let mut on_board = BTreeMap::new();
        on_board.insert(AreaId::new(0), 2);
        on_board.insert(AreaId::new(1), 1);

        // President in hand: routes family (PlacePresident) then retreat with
        // from_hand=true, carrying an exact clone of on_board.
        let mut in_hand = at_phase(TurnPhase::Movement);
        in_hand.players[0].president_location = PresidentLocation::InHand;
        in_hand.players[0].assistants.on_board = on_board.clone();
        assert_eq!(
            families(&in_hand, &rules),
            vec![
                MoveFamily::MovementRoutes,
                MoveFamily::MovementRetreat { on_board: on_board.clone(), from_hand: true },
            ],
        );

        // President on board: same routes-then-retreat order and snapshot, but
        // from_hand=false.
        let mut on_board_state = in_hand.clone();
        on_board_state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        assert_eq!(
            families(&on_board_state, &rules),
            vec![
                MoveFamily::MovementRoutes,
                MoveFamily::MovementRetreat { on_board, from_hand: false },
            ],
        );
    }

    #[test]
    fn pending_choice_short_circuits_dispatch() {
        let rules = Rules::empty_for_tests();

        // A pending choice overrides the turn phase entirely (mirrors
        // legal_moves.rs:9-11): at Placement, only ChoiceResolution.
        let mut at_placement = at_phase(TurnPhase::Placement);
        at_placement.pending_choices.push(PendingChoice::AnyGood);
        assert_eq!(families(&at_placement, &rules), vec![MoveFamily::ChoiceResolution]);

        // And it wins over Movement too (which would otherwise be two families).
        let mut at_movement = at_phase(TurnPhase::Movement);
        at_movement.pending_choices.push(PendingChoice::AnyGood);
        assert_eq!(families(&at_movement, &rules), vec![MoveFamily::ChoiceResolution]);
    }

    #[test]
    fn families_cover_every_legal_move() {
        let rules = Rules::empty_for_tests();

        // Each move get_legal_moves emits must belong to a family families()
        // lists — the union of the factored families reproduces the whole
        // legal list. States chosen so each is non-empty and panic-free
        // (Movement OnBoard is pinned structurally in the test above, so it is
        // not re-driven through the route generator here).
        let mut cases: Vec<GameState> = Vec::new();

        // Placement: real placement moves.
        cases.push(at_phase(TurnPhase::Placement));

        // Additional action: EndAdditionalPhase is always offered.
        cases.push(at_phase(TurnPhase::AdditionalActionFirst));

        // Movement, President in hand with an own assistant on area 0 -> both a
        // PlacePresident (MovementRoutes) and ReturnPresidentToHand
        // (MovementRetreat) are emitted.
        let mut movement = at_phase(TurnPhase::Movement);
        movement.players[0].president_location = PresidentLocation::InHand;
        movement.players[0].assistants.on_board.insert(AreaId::new(0), 1);
        cases.push(movement);

        // Pending choice -> ChooseAnyGood resolutions (ChoiceResolution).
        let mut choice = at_phase(TurnPhase::Placement);
        choice.pending_choices.push(PendingChoice::AnyGood);
        cases.push(choice);

        for state in &cases {
            let legal = get_legal_moves(state, &rules);
            assert!(
                !legal.is_empty(),
                "case at {:?} must have legal moves to cross-check",
                state.turn_phase,
            );
            let fams = families(state, &rules);
            let fam_kinds: Vec<_> = fams.iter().map(std::mem::discriminant).collect();
            for mv in &legal {
                assert!(
                    fam_kinds.contains(&family_kind(mv)),
                    "legal move {mv:?} at {:?} maps to no family in {fams:?}",
                    state.turn_phase,
                );
            }
        }
    }

    // -- Retreat family (Task 2) ------------------------------------------

    /// Build `on_board` from `(area_index, count)` pairs.
    fn on_board_of(pairs: &[(u8, u8)]) -> BTreeMap<AreaId, u8> {
        pairs.iter().map(|&(a, c)| (AreaId::new(a), c)).collect()
    }

    /// A Movement-phase state, president in hand, with the current player's
    /// on-board assistants set to `on_board`. `get_legal_moves` on this emits
    /// the retreat block (`enumerate_recovery_options`) as its
    /// `ReturnPresidentToHand` moves; `enumerate_recovery_options` and
    /// `apply_return_to_hand` read only `on_board`, never board membership, so
    /// the two-area base board suffices even for 17-area retreat snapshots.
    fn movement_in_hand(on_board: &BTreeMap<AreaId, u8>) -> GameState {
        let mut s = at_phase(TurnPhase::Movement);
        s.players[0].president_location = PresidentLocation::InHand;
        s.players[0].assistants.on_board = on_board.clone();
        s
    }

    /// The generator's retreat sequence at `on_board`, via the public surface:
    /// `get_legal_moves` filtered to `ReturnPresidentToHand` (pushed contiguous
    /// and last, in `enumerate_recovery_options` order — `legal_moves.rs:274`).
    /// This is the authority `retreat_iter` is pinned against (the private
    /// `enumerate_recovery_options` is unreachable and `legal_moves.rs` is
    /// read-only).
    fn generator_retreats(on_board: &BTreeMap<AreaId, u8>) -> Vec<Move> {
        let rules = Rules::empty_for_tests();
        get_legal_moves(&movement_in_hand(on_board), &rules)
            .into_iter()
            .filter(|m| matches!(m, Move::ReturnPresidentToHand { .. }))
            .collect()
    }

    /// The 4P retreat worst case: three areas at count 2, fourteen at count 1
    /// over 17 areas — product `3³ · 2¹⁴ = 442_368`, the six-figure family this
    /// factored source exists to serve without materializing. Largest AreaId
    /// (16, count 1) is the least-significant mixed-radix digit; AreaId 0
    /// (count 2) the most-significant.
    fn worst_case_on_board() -> BTreeMap<AreaId, u8> {
        let mut m = BTreeMap::new();
        for a in 0..3u8 {
            m.insert(AreaId::new(a), 2);
        }
        for a in 3..17u8 {
            m.insert(AreaId::new(a), 1);
        }
        m
    }

    /// THE load-bearing pin: `retreat_iter` reproduces the generator's retreat
    /// sequence exactly — same ORDER and same `recover` maps — across shapes
    /// small enough to materialize the generator's block. This is what fixes
    /// the mixed-radix convention (last occupied area = least-significant
    /// digit) against the real `enumerate_recovery_options` fold; a
    /// first-area-LSB decode fails it.
    #[test]
    fn retreat_iter_matches_generator_order_and_multiset() {
        let shapes: &[&[(u8, u8)]] = &[
            &[],                                     // empty -> lone recover-nothing
            &[(0, 1)],                               // radix 2
            &[(0, 2)],                               // radix 3
            &[(0, 2), (1, 1)],                       // 3·2 = 6 (worked example)
            &[(0, 1), (1, 2), (2, 3)],               // 2·3·4 = 24, mixed radices
            &[(3, 2), (7, 2), (9, 1), (12, 1)],      // sparse AreaIds, 3·3·2·2 = 36
            &[(0, 2), (1, 2), (2, 2), (3, 1), (4, 1)], // 3·3·3·2·2 = 108
        ];
        for shape in shapes {
            let on_board = on_board_of(shape);
            let got: Vec<Move> = retreat_iter(&on_board).collect();
            let want = generator_retreats(&on_board);
            assert_eq!(got, want, "order+multiset mismatch at shape {shape:?}");
        }
    }

    /// `retreat_len` equals the generator's retreat count for materializable
    /// shapes, and the closed-form product at the six-figure worst case (no
    /// generator call — that would build 442k moves).
    #[test]
    fn retreat_len_equals_generator_len() {
        let shapes: &[&[(u8, u8)]] = &[
            &[],
            &[(0, 2)],
            &[(0, 2), (1, 1)],
            &[(0, 1), (1, 2), (2, 3)],
            &[(3, 2), (7, 2), (9, 1), (12, 1)],
            &[(0, 2), (1, 2), (2, 2), (3, 1), (4, 1)],
        ];
        for shape in shapes {
            let on_board = on_board_of(shape);
            assert_eq!(
                retreat_len(&on_board),
                generator_retreats(&on_board).len(),
                "len mismatch at shape {shape:?}",
            );
        }
        assert_eq!(retreat_len(&worst_case_on_board()), 442_368);
    }

    /// Landmark indices pin the mixed-radix convention AT SCALE (17 digits)
    /// without materializing the 442k family: index 0 = recover nothing,
    /// index 1 = the least-significant digit (largest AreaId) ticks, the
    /// most-significant place value ticks AreaId 0, and the last index recovers
    /// everything (the generator's all-max terminal element,
    /// `legal_moves.rs:381`).
    #[test]
    fn retreat_iter_landmark_indices_pin_mixed_radix_convention() {
        let on_board = worst_case_on_board();
        let len = retreat_len(&on_board);
        assert_eq!(len, 442_368);

        let ret = |recover: BTreeMap<AreaId, u8>| Move::ReturnPresidentToHand { recover };

        // index 0 (a fresh iterator's first element) -> recover nothing.
        assert_eq!(retreat_iter(&on_board).next().unwrap(), ret(BTreeMap::new()));

        // index 1 -> least-significant digit: largest AreaId (16, count 1).
        assert_eq!(retreat_iter(&on_board).nth(1).unwrap(), ret(on_board_of(&[(16, 1)])));

        // Most-significant place value = 3·3·2¹⁴ = 147_456 -> AreaId 0 ticks.
        assert_eq!(
            retreat_iter(&on_board).nth(147_456).unwrap(),
            ret(on_board_of(&[(0, 1)])),
        );

        // Last index -> recover everything (all digits maxed == on_board map).
        assert_eq!(retreat_iter(&on_board).nth(len - 1).unwrap(), ret(on_board.clone()));
    }

    /// The constructor assembles the move (dropping zero counts) and does NOT
    /// pre-validate: an in-range move applies `Ok`, an over-count move is
    /// rejected by `apply_return_to_hand`, not the constructor.
    #[test]
    fn retreat_move_builds_move_and_arm_validates() {
        let rules = Rules::empty_for_tests();
        let (a0, a3) = (AreaId::new(0), AreaId::new(3));
        let on_board = on_board_of(&[(0, 2), (3, 1)]);
        let mut state = movement_in_hand(&on_board);
        state.players[0].president_location = PresidentLocation::OnBoard(a0);

        // In-range: recover 2 from a0, 1 from a3.
        let mv = retreat_move(&on_board, &[(a0, 2), (a3, 1)]);
        assert_eq!(mv, Move::ReturnPresidentToHand { recover: on_board_of(&[(0, 2), (3, 1)]) });
        let applied = apply_move_unchecked(&state, &mv, &rules).expect("in-range recovery applies");
        assert_eq!(applied.players[0].president_location, PresidentLocation::InHand);
        assert_eq!(applied.players[0].assistants.on_board.get(&a0), None, "all of a0 recovered");

        // Zero counts dropped -> recover-nothing move.
        assert_eq!(
            retreat_move(&on_board, &[(a0, 0), (a3, 0)]),
            Move::ReturnPresidentToHand { recover: BTreeMap::new() },
        );

        // Over-count: constructor builds it verbatim; the arm rejects it.
        let over = retreat_move(&on_board, &[(a0, 3)]);
        assert_eq!(over, Move::ReturnPresidentToHand { recover: on_board_of(&[(0, 3)]) });
        assert!(matches!(
            apply_move_unchecked(&state, &over, &rules),
            Err(MoveError::RuleViolation(_)),
        ));
    }

    /// Every `retreat_sample` draw is a legal recovery (rejection-free): 10k
    /// draws all apply `Ok`, and none recovers more than is on board.
    #[test]
    fn retreat_sample_draws_always_apply() {
        use rand::SeedableRng;
        let rules = Rules::empty_for_tests();
        let on_board = on_board_of(&[(0, 2), (1, 3), (2, 1), (5, 2)]);
        let mut state = movement_in_hand(&on_board);
        state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));

        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(20_260_725);
        for _ in 0..10_000 {
            let mv = retreat_sample(&on_board, &mut rng);
            let applied =
                apply_move_unchecked(&state, &mv, &rules).expect("per-area draw is always in range");
            assert_eq!(applied.players[0].president_location, PresidentLocation::InHand);
            if let Move::ReturnPresidentToHand { recover } = &mv {
                for (area, &c) in recover {
                    assert!(c >= 1 && c <= on_board[area], "drew {c} at {area:?}");
                }
            }
        }
    }

    /// Distributional pin for `retreat_sample`'s documented uniformity over the
    /// whole product family (review S2): on a 6-member family ({a0: 2, a1: 1} ⇒
    /// 3·2), 60k seeded draws must hit EVERY member, each within a generous
    /// band around len/6. Kills the support-shrinking mutation
    /// (`gen_range(0..count)`, which silently drops every recover-max vector
    /// while every legality test stays green) and any gross per-digit skew.
    #[test]
    fn retreat_sample_is_uniform_over_the_product_family() {
        use rand::SeedableRng;
        let on_board = on_board_of(&[(0, 2), (1, 1)]);
        let len = retreat_len(&on_board);
        assert_eq!(len, 6);

        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(20_260_725);
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        const DRAWS: usize = 60_000;
        for _ in 0..DRAWS {
            let mv = retreat_sample(&on_board, &mut rng);
            *counts.entry(format!("{mv:?}")).or_insert(0) += 1;
        }

        assert_eq!(counts.len(), len, "every member of the product family must be drawn");
        let expected = DRAWS / len;
        for (key, n) in &counts {
            assert!(
                (expected * 9 / 10..=expected * 11 / 10).contains(n),
                "draw frequency for {key} is {n}, outside ±10% of {expected} — not uniform",
            );
        }
    }

    /// No-materialization: serving ONE candidate off the 442k worst case
    /// allocates `O(#areas)`, not `O(product)`. The thread-local counting
    /// allocator gives the hard, parallel-safe COUNT bound; laziness is also
    /// evidenced by `nth(400_000)` returning instantly rather than building the
    /// family.
    #[test]
    fn serving_one_retreat_candidate_is_o_areas_not_o_product() {
        let on_board = worst_case_on_board();
        assert_eq!(retreat_len(&on_board), 442_368);

        // Laziness: a deep index resolves without building the whole family.
        let deep = retreat_iter(&on_board).nth(400_000).expect("index in range");
        match &deep {
            Move::ReturnPresidentToHand { recover } => {
                for (area, &c) in recover {
                    assert!(c >= 1 && c <= on_board[area]);
                }
            }
            other => panic!("expected a retreat move, got {other:?}"),
        }

        // Hard bound: 17 areas -> a handful of small allocations. The generous
        // < 64 bound is what must never scale with the 442k product.
        let (served, iter_allocs) =
            alloc_probe::count_allocs(|| retreat_iter(&on_board).nth(400_000));
        assert!(served.is_some());
        assert!(iter_allocs < 64, "retreat_iter().nth allocated {iter_allocs} (>= 64)");

        use rand::SeedableRng;
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(1);
        let (_s, sample_allocs) =
            alloc_probe::count_allocs(|| retreat_sample(&on_board, &mut rng));
        assert!(sample_allocs < 64, "retreat_sample allocated {sample_allocs} (>= 64)");
    }

    // -- Placement family (Task 3) ----------------------------------------

    /// The generator's placement sequence at `state`, via the public surface:
    /// `get_legal_moves` filtered to the two placement variants. At a
    /// `Placement` state with no pending choice this filter keeps the whole
    /// list; the filter guards against a caller passing a non-placement state.
    /// This is the authority `placement_iter` is pinned against.
    fn generator_placements(state: &GameState, rules: &Rules) -> Vec<Move> {
        get_legal_moves(state, rules)
            .into_iter()
            .filter(|m| matches!(m, Move::PlaceThreeAreas { .. } | Move::PlaceTwoSameArea { .. }))
            .collect()
    }

    /// A `Placement`-phase state with `players` players (current = player 0) and
    /// `n_areas` placement-allowing areas (ids `0..n_areas`). Reuses
    /// `base_state`'s known-good field layout, overriding only the config,
    /// player roster, and board so the generator and `placement_iter` treat it
    /// as a real placement decision point. Callers set the per-test knobs
    /// afterwards (in_hand, money, technologies, opponent president locations,
    /// adjacency, canal crossings).
    fn placement_state(players: u8, n_areas: u8) -> GameState {
        let colors = [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green];
        let color_vec: Vec<PlayerColor> = colors[..players as usize].to_vec();

        let mut s = base_state();
        s.config = GameConfig::new(players, color_vec.clone(), PlayerId::new(0));
        s.players = (0..players)
            .map(|i| PlayerState::new(PlayerId::new(i), color_vec[i as usize], i == 0))
            .collect();

        let mut areas = BTreeMap::new();
        for i in 0..n_areas {
            areas.insert(
                AreaId::new(i),
                AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, i) },
            );
        }
        s.board_layout = BoardLayout {
            areas,
            adjacency: BTreeMap::new(),
            canal_crossings: vec![],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };
        s
    }

    /// Drive a seeded real game (standard rules) and collect up to `want`
    /// distinct `Placement`-phase states (no pending choice), for cross-checking
    /// `placement_iter` against organically-reached decision points.
    fn driven_placement_states(rules: &Rules, seed: u64, players: u8, want: usize) -> Vec<GameState> {
        use rand::SeedableRng;
        let config = GameConfig::for_player_count(players);
        let pending = crate::new_game(config, rules, seed).expect("new_game");
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
        let choices: Vec<crate::StartingOrderChoice> =
            (0..players).map(|_| crate::StartingOrderChoice::Keep(rng.gen_range(0..2u8))).collect();
        let mut state = crate::finalize_setup(pending, &choices).expect("finalize_setup");

        let mut out = Vec::new();
        for _ in 0..4000 {
            if out.len() >= want || crate::is_game_over(&state) {
                break;
            }
            if state.pending_choices.is_empty() && state.turn_phase == TurnPhase::Placement {
                out.push(state.clone());
            }
            let moves = get_legal_moves(&state, rules);
            if moves.is_empty() {
                break;
            }
            let mv = moves[rng.gen_range(0..moves.len())].clone();
            state = apply_move_unchecked(&state, &mv, rules).expect("apply drawn move");
        }
        out
    }

    /// `place_combo` maps selected indices through the sorted-areas list into a
    /// `PlaceThreeAreas` move; an empty selection is the always-legal empty
    /// placement.
    #[test]
    fn place_combo_builds_place_three_areas_from_indices() {
        let areas = [AreaId::new(3), AreaId::new(7), AreaId::new(9), AreaId::new(12)];
        assert_eq!(
            place_combo(&areas, &[0, 2, 3]),
            Move::PlaceThreeAreas { placements: vec![AreaId::new(3), AreaId::new(9), AreaId::new(12)] },
        );
        assert_eq!(place_combo(&areas, &[]), Move::PlaceThreeAreas { placements: vec![] });
    }

    /// THE load-bearing pin: `placement_iter` reproduces the generator's
    /// placement sequence exactly -- same ORDER and same moves -- across
    /// organically-driven states at 2/3/4P, and `placement_upper_len` never
    /// undercounts.
    #[test]
    fn placement_iter_matches_generator_on_driven_states() {
        let rules = Rules::standard();
        let mut checked = 0usize;
        for players in [2u8, 3, 4] {
            for seed in 0..3u64 {
                for state in driven_placement_states(&rules, seed, players, 6) {
                    let got: Vec<Move> = placement_iter(&state, &rules).collect();
                    let want = generator_placements(&state, &rules);
                    assert_eq!(got, want, "order+multiset mismatch, seed {seed} {players}p");
                    assert!(
                        placement_upper_len(&state, &rules) >= got.len(),
                        "upper bound below actual, seed {seed} {players}p",
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked >= 3, "expected several driven placement states, saw {checked}");
    }

    /// Per-COMBINATION affordability pin: with three opponent presidents on
    /// three distinct areas, each of those areas is affordable ALONE (fee 1) but
    /// the 3-subset spanning all three costs 3 > money 2. The generator excludes
    /// exactly that one subset while keeping every single; `placement_iter` must
    /// match, and `placement_upper_len` (which ignores the hole) must strictly
    /// exceed the realized count. A per-AREA filter would wrongly keep {0,1,2}.
    #[test]
    fn placement_per_combination_affordability_excludes_costly_subset() {
        let rules = Rules::empty_for_tests();
        let a = AreaId::new;
        let mut s = placement_state(4, 4);
        s.players[0].assistants.in_hand = 3;
        s.players[0].money = 2;
        s.players[1].president_location = PresidentLocation::OnBoard(a(0));
        s.players[2].president_location = PresidentLocation::OnBoard(a(1));
        s.players[3].president_location = PresidentLocation::OnBoard(a(2));

        // Each area is affordable alone (fee 1 <= 2) ...
        for area in [a(0), a(1), a(2)] {
            assert_eq!(calculate_placement_cost(&s, PlayerId::new(0), &[area]), 1);
        }
        // ... but the combination of all three is not (fee 3 > 2).
        assert_eq!(calculate_placement_cost(&s, PlayerId::new(0), &[a(0), a(1), a(2)]), 3);

        let got: Vec<Move> = placement_iter(&s, &rules).collect();
        assert_eq!(got, generator_placements(&s, &rules), "order+multiset must match generator");

        let costly = Move::PlaceThreeAreas { placements: vec![a(0), a(1), a(2)] };
        assert!(!got.contains(&costly), "over-budget 3-subset must be filtered as a whole");
        for area in [a(0), a(1), a(2)] {
            assert!(
                got.contains(&Move::PlaceThreeAreas { placements: vec![area] }),
                "each individually-affordable single must survive",
            );
        }

        // The single punched hole makes the upper bound a STRICT over-estimate.
        let upper = placement_upper_len(&s, &rules);
        assert!(upper > got.len(), "affordability hole must make upper {upper} > {}", got.len());
    }

    /// Newspaper raises the Option-A ceiling to 4 areas: with in_hand 4 and no
    /// fees, `placement_iter` reaches k=4 combinations, matches the generator,
    /// and (fully affordable) `placement_upper_len` is EXACT.
    #[test]
    fn placement_iter_reaches_k4_with_newspaper() {
        let rules = Rules::standard();
        let newspaper =
            rules.techs.iter().find(|c| c.effect == TechEffect::Newspaper).expect("has Newspaper").id;
        let mut s = placement_state(4, 5);
        s.players[0].assistants.in_hand = 4;
        s.players[0].money = 100;
        s.players[0].technologies = vec![newspaper];

        let got: Vec<Move> = placement_iter(&s, &rules).collect();
        assert_eq!(got, generator_placements(&s, &rules), "order+multiset must match generator");
        assert!(
            got.iter().any(
                |m| matches!(m, Move::PlaceThreeAreas { placements } if placements.len() == 4),
            ),
            "Newspaper + in_hand 4 must reach k=4",
        );
        assert_eq!(
            placement_upper_len(&s, &rules),
            got.len(),
            "fully affordable => upper bound is exact",
        );
    }

    /// Option B with Telegram: the bonus offers the non-crossing placeable
    /// neighbour and EXCLUDES the canal-crossing one (transit-only adjacency).
    /// `placement_iter` matches the generator, including that exclusion.
    #[test]
    fn placement_iter_matches_generator_with_telegram() {
        let rules = Rules::standard();
        let telegram =
            rules.techs.iter().find(|c| c.effect == TechEffect::Telegram).expect("has Telegram").id;
        let a = AreaId::new;
        let mut s = placement_state(2, 4);
        s.players[0].assistants.in_hand = 3;
        s.players[0].money = 100;
        s.players[0].technologies = vec![telegram];
        // area 0 ~ {1 (free edge), 2 (canal crossing)}.
        s.board_layout.adjacency.insert(a(0), vec![a(1), a(2)]);
        s.board_layout.adjacency.insert(a(1), vec![a(0)]);
        s.board_layout.adjacency.insert(a(2), vec![a(0)]);
        s.board_layout.canal_crossings = vec![(a(0), a(2))];

        let got: Vec<Move> = placement_iter(&s, &rules).collect();
        assert_eq!(got, generator_placements(&s, &rules), "order+multiset must match generator");

        assert!(
            got.contains(&Move::PlaceTwoSameArea { area: a(0), adjacent_bonus: Some(a(1)) }),
            "free neighbour must be offered as a Telegram bonus",
        );
        assert!(
            !got.iter().any(|m| matches!(
                m,
                Move::PlaceTwoSameArea { area, adjacent_bonus: Some(adj) }
                    if *area == a(0) && *adj == a(2)
            )),
            "canal-crossing neighbour must be excluded from the Telegram bonus",
        );

        // Fully affordable (no fees): the upper bound -- including its Option-B
        // Telegram non-crossing-neighbour term -- is EXACT here.
        assert_eq!(placement_upper_len(&s, &rules), got.len(), "upper bound must be exact");
    }

    /// The `in_hand == 0` degenerate case: the generator returns only the empty
    /// placement (`legal_moves.rs:121-125`); `placement_iter` reproduces it via
    /// the general path (`k_max` collapses to 0, Option B gated off).
    #[test]
    fn placement_iter_handles_in_hand_zero_degenerate() {
        let rules = Rules::empty_for_tests();
        let mut s = placement_state(3, 5);
        s.players[0].assistants.in_hand = 0;

        let got: Vec<Move> = placement_iter(&s, &rules).collect();
        assert_eq!(got, vec![Move::PlaceThreeAreas { placements: vec![] }]);
        assert_eq!(got, generator_placements(&s, &rules));
        assert_eq!(placement_upper_len(&s, &rules), 1);
    }

    /// 1000 `placement_sample` draws off a rich (4P + Newspaper, in_hand 4)
    /// state all apply `Ok`, and the sampler yields variety (not a degenerate
    /// always-empty draw).
    #[test]
    fn placement_sample_draws_always_apply() {
        use rand::SeedableRng;
        let rules = Rules::standard();
        let newspaper =
            rules.techs.iter().find(|c| c.effect == TechEffect::Newspaper).expect("has Newspaper").id;
        let mut s = placement_state(4, 5);
        s.players[0].assistants.in_hand = 4;
        s.players[0].money = 100;
        s.players[0].technologies = vec![newspaper];

        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(20_260_725);
        let mut distinct = std::collections::HashSet::new();
        for _ in 0..1000 {
            let mv = placement_sample(&s, &rules, &mut rng);
            apply_move_unchecked(&s, &mv, &rules).expect("sampled placement applies");
            distinct.insert(format!("{mv:?}"));
        }
        assert!(distinct.len() > 1, "sampler must draw more than one distinct move");
    }

    /// Distributional pin for `placement_sample`'s documented uniformity over
    /// the realized family (review S2): 3 areas, in_hand 3, no fees ⇒ 11
    /// members (Σ C(3,k), k=0..=3, = 8 Option-A subsets + 3 Option-B basics).
    /// 22k seeded draws hit every member within a generous band of len/11 —
    /// killing reservoir-skew mutations (e.g. always keeping the first
    /// candidate) that legality tests cannot see.
    #[test]
    fn placement_sample_is_uniform_over_the_realized_family() {
        use rand::SeedableRng;
        let rules = Rules::empty_for_tests();
        let mut s = placement_state(2, 3);
        s.players[0].assistants.in_hand = 3;
        s.players[0].money = 100;

        let realized: Vec<Move> = placement_iter(&s, &rules).collect();
        assert_eq!(realized.len(), 11, "Σ C(3,k) (k=0..=3) + 3 Option-B basics");

        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(20_260_726);
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        const DRAWS: usize = 22_000;
        for _ in 0..DRAWS {
            let mv = placement_sample(&s, &rules, &mut rng);
            *counts.entry(format!("{mv:?}")).or_insert(0) += 1;
        }

        assert_eq!(counts.len(), realized.len(), "every realized member must be drawn");
        let expected = DRAWS / realized.len();
        for (key, n) in &counts {
            assert!(
                (expected * 85 / 100..=expected * 115 / 100).contains(n),
                "draw frequency for {key} is {n}, outside ±15% of {expected} — not uniform",
            );
        }
    }

    /// Telegram's Option-B neighbour bonus requires `in_hand >= 3` (2 for the
    /// same-area pair + 1 for the neighbour). At exactly `in_hand == 2` with
    /// Telegram owned the bonus must NOT appear: `placement_iter` matches the
    /// generator, no `adjacent_bonus` move exists, basic Option B IS still
    /// offered, and `placement_upper_len` omits the neighbour term (review S11
    /// — the `>= 3` gate previously had no boundary fixture, so an off-by-one
    /// to `>= 2` was organically invisible).
    #[test]
    fn placement_telegram_bonus_gated_off_at_in_hand_two() {
        let rules = Rules::standard();
        let telegram =
            rules.techs.iter().find(|c| c.effect == TechEffect::Telegram).expect("has Telegram").id;
        let a = AreaId::new;
        let mut s = placement_state(2, 4);
        s.players[0].assistants.in_hand = 2;
        s.players[0].money = 100;
        s.players[0].technologies = vec![telegram];
        s.board_layout.adjacency.insert(a(0), vec![a(1)]);
        s.board_layout.adjacency.insert(a(1), vec![a(0)]);

        let got: Vec<Move> = placement_iter(&s, &rules).collect();
        assert_eq!(got, generator_placements(&s, &rules), "order+multiset must match generator");
        assert!(
            !got.iter()
                .any(|m| matches!(m, Move::PlaceTwoSameArea { adjacent_bonus: Some(_), .. })),
            "in_hand 2 must not offer the Telegram neighbour bonus",
        );
        assert!(
            got.iter().any(|m| matches!(m, Move::PlaceTwoSameArea { adjacent_bonus: None, .. })),
            "basic Option B is still legal at in_hand 2",
        );
        // Upper bound omits the neighbour term: Σ C(4,k) (k=0..=2) + 4 basics.
        assert_eq!(placement_upper_len(&s, &rules), 1 + 4 + 6 + 4);
    }

    // -- Unified dispatch (Task 4) ----------------------------------------

    /// A short name for `family`, for the driven-game anti-vacuity coverage set.
    fn family_name(f: &MoveFamily) -> &'static str {
        match f {
            MoveFamily::AdditionalAction => "AdditionalAction",
            MoveFamily::Placement => "Placement",
            MoveFamily::MovementRoutes => "MovementRoutes",
            MoveFamily::MovementRetreat { .. } => "MovementRetreat",
            MoveFamily::AreaAction => "AreaAction",
            MoveFamily::FivePower => "FivePower",
            MoveFamily::Building => "Building",
            MoveFamily::ChoiceResolution => "ChoiceResolution",
        }
    }

    /// THE Task-4 equivalence pin: at every organically-reached decision point
    /// across 2/3/4P games, `iter_family(state, family)` reproduces
    /// `get_legal_moves` filtered to that family EXACTLY -- same order and
    /// multiset -- for every family `families()` lists (retreat/placement via
    /// their bespoke lazy generators, the rest via delegate-and-filter). And
    /// `family_len` equals that realized count for every family EXCEPT
    /// Placement, where it is the honest upper bound (>= realized). The coverage
    /// set pins that the per-turn-guaranteed families were actually exercised.
    #[test]
    fn iter_family_and_len_match_generator_across_driven_games() {
        use rand::SeedableRng;
        let rules = Rules::standard();
        let mut seen: std::collections::HashSet<&'static str> = std::collections::HashSet::new();

        for players in [2u8, 3, 4] {
            for seed in 0..5u64 {
                let config = GameConfig::for_player_count(players);
                let pending = crate::new_game(config, &rules, seed).expect("new_game");
                let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
                let choices: Vec<crate::StartingOrderChoice> = (0..players)
                    .map(|_| crate::StartingOrderChoice::Keep(rng.gen_range(0..2u8)))
                    .collect();
                let mut state = crate::finalize_setup(pending, &choices).expect("finalize_setup");

                for _ in 0..2000 {
                    if crate::is_game_over(&state) {
                        break;
                    }
                    let legal = get_legal_moves(&state, &rules);
                    if legal.is_empty() {
                        break;
                    }
                    for family in families(&state, &rules) {
                        let want = std::mem::discriminant(&family);
                        let expected: Vec<Move> =
                            legal.iter().filter(|m| family_kind(m) == want).cloned().collect();
                        let got: Vec<Move> = iter_family(&state, &rules, &family).collect();
                        assert_eq!(
                            got, expected,
                            "iter_family != generator-filtered for {family:?} ({players}p seed {seed})",
                        );

                        let len = family_len(&state, &rules, &family);
                        if matches!(family, MoveFamily::Placement) {
                            assert!(
                                len >= got.len(),
                                "placement upper {len} < realized {} ({players}p seed {seed})",
                                got.len(),
                            );
                        } else {
                            assert_eq!(
                                len,
                                got.len(),
                                "family_len != realized for {family:?} ({players}p seed {seed})",
                            );
                        }
                        seen.insert(family_name(&family));
                    }

                    let mv = legal[rng.gen_range(0..legal.len())].clone();
                    state = apply_move_unchecked(&state, &mv, &rules).expect("apply drawn move");
                }
            }
        }

        // Anti-vacuity: every family structurally guaranteed each turn must have
        // been exercised on a real state (ChoiceResolution is covered by its own
        // targeted test, as it is not guaranteed every game).
        for name in [
            "AdditionalAction",
            "Placement",
            "MovementRoutes",
            "MovementRetreat",
            "AreaAction",
            "FivePower",
            "Building",
        ] {
            assert!(seen.contains(name), "family {name} never exercised across driven games");
        }
    }

    /// THE FulfillOrder anti-vacuity pin: `iter_family(AdditionalAction)`
    /// preserves the generator's `order_cards_in_hand` INSERTION order for
    /// FulfillOrder moves (`legal_moves.rs:59`). A sorted-by-id iterator would
    /// reorder this deliberately-descending hand ([id 2, id 1]) and FAIL. The
    /// delegate-and-filter dispatch inherits the generator's order for free.
    #[test]
    fn additional_action_iter_preserves_nonsorted_fulfill_order_hand_order() {
        let rules = Rules::standard();
        let mut s = at_phase(TurnPhase::AdditionalActionFirst);
        // No assistant in hand -> achievements are unclaimable, keeping the
        // family focused on the two FulfillOrder moves + EndAdditionalPhase.
        s.players[0].assistants.in_hand = 0;
        // Abundant goods so both orders are fulfillable regardless of their
        // printed copper/silk/tea/seafood requirements.
        s.players[0].copper = 99;
        s.players[0].silk = 99;
        s.players[0].tea = 99;
        s.players[0].seafood = 99;
        // Non-sorted hand: id 2 BEFORE id 1 (a sorted key would emit 1 then 2).
        s.players[0].order_cards_in_hand = vec![OrderCardId(2), OrderCardId(1)];

        let family = MoveFamily::AdditionalAction;
        let got: Vec<Move> = iter_family(&s, &rules, &family).collect();

        // Equivalence by construction with the whole generator list (every
        // additional-action move is in this one family here).
        assert_eq!(got, get_legal_moves(&s, &rules), "iter_family must equal the generator list");

        // The load-bearing order pin: FulfillOrder ids in HAND order [2, 1].
        let order_ids: Vec<OrderCardId> = got
            .iter()
            .filter_map(|m| match m {
                Move::FulfillOrder { order_id } => Some(*order_id),
                _ => None,
            })
            .collect();
        assert_eq!(
            order_ids,
            vec![OrderCardId(2), OrderCardId(1)],
            "FulfillOrder must follow order_cards_in_hand insertion order, not a sorted key",
        );

        assert_eq!(family_len(&s, &rules, &family), got.len(), "family_len == realized count");
    }

    /// Every `sample_family` draw is a legal move OF THE SAMPLED FAMILY that
    /// applies `Ok`, across organically-reached states. Retreat/placement route
    /// to their bespoke samplers; the delegated families draw a uniform index
    /// into the legal slice. A structurally-empty family (e.g. no legal route)
    /// yields `None` and is skipped — the `Option` contract, pinned separately
    /// by `sample_family_returns_none_on_structurally_empty_family`.
    #[test]
    fn sample_family_draws_apply_across_driven_games() {
        use rand::SeedableRng;
        let rules = Rules::standard();
        let mut drawn = 0usize;

        for players in [2u8, 3, 4] {
            for seed in 0..3u64 {
                let config = GameConfig::for_player_count(players);
                let pending = crate::new_game(config, &rules, seed).expect("new_game");
                let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed + 4242);
                let choices: Vec<crate::StartingOrderChoice> = (0..players)
                    .map(|_| crate::StartingOrderChoice::Keep(rng.gen_range(0..2u8)))
                    .collect();
                let mut state = crate::finalize_setup(pending, &choices).expect("finalize_setup");

                for _ in 0..1500 {
                    if crate::is_game_over(&state) {
                        break;
                    }
                    let legal = get_legal_moves(&state, &rules);
                    if legal.is_empty() {
                        break;
                    }
                    for family in families(&state, &rules) {
                        for _ in 0..3 {
                            let Some(mv) = sample_family(&state, &rules, &family, &mut rng)
                            else {
                                continue;
                            };
                            assert_eq!(
                                family_kind(&mv),
                                std::mem::discriminant(&family),
                                "sample_family drew {mv:?} outside {family:?}",
                            );
                            apply_move_unchecked(&state, &mv, &rules).unwrap_or_else(|e| {
                                panic!("sampled {mv:?} from {family:?} failed to apply: {e:?}")
                            });
                            drawn += 1;
                        }
                    }
                    let mv = legal[rng.gen_range(0..legal.len())].clone();
                    state = apply_move_unchecked(&state, &mv, &rules).expect("apply drawn move");
                }
            }
        }
        assert!(drawn > 200, "expected many sampled draws, saw {drawn}");
    }

    /// A structurally-empty family yields `None` from `sample_family` rather
    /// than panicking. The shape is organically reachable: President in hand,
    /// the only own-assistant area blocked by an opponent President ⇒
    /// `families()` still tags `MovementRoutes` (structural dispatch, no
    /// emptiness probe) but no `PlacePresident` destination is a Possible
    /// Action Area, so the family is empty — while the retreat family keeps
    /// the Movement step alive and still samples `Some`.
    #[test]
    fn sample_family_returns_none_on_structurally_empty_family() {
        use rand::SeedableRng;
        let rules = Rules::empty_for_tests();
        let mut s = at_phase(TurnPhase::Movement);
        s.players[0].president_location = PresidentLocation::InHand;
        s.players[0].assistants.on_board.insert(AreaId::new(0), 1);
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(0));

        let fams = families(&s, &rules);
        let routes = fams
            .iter()
            .find(|f| matches!(f, MoveFamily::MovementRoutes))
            .expect("Movement dispatches a routes family");
        let retreat = fams
            .iter()
            .find(|f| matches!(f, MoveFamily::MovementRetreat { .. }))
            .expect("Movement dispatches a retreat family");

        assert_eq!(family_len(&s, &rules, routes), 0, "no Possible Action Area ⇒ empty routes");
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(7);
        assert!(
            sample_family(&s, &rules, routes, &mut rng).is_none(),
            "an empty family must yield None, not panic",
        );
        assert!(
            sample_family(&s, &rules, retreat, &mut rng).is_some(),
            "the never-empty retreat family must still yield Some",
        );
    }

    /// Dispatch smoke: `iter_family`/`family_len`/`sample_family` on the bespoke
    /// families forward to the Task-2/3 functions (deeply tested there), so a
    /// mis-routed arm (e.g. retreat sent through the delegate path) is caught.
    #[test]
    fn retreat_and_placement_dispatch_to_bespoke_generators() {
        use rand::SeedableRng;
        let rules = Rules::standard();

        // Retreat: the payload's on_board drives the serve; the state mirrors
        // it, per the MoveFamily precondition (debug-asserted in the arms).
        let on_board = on_board_of(&[(0, 2), (1, 1), (5, 2)]);
        let retreat = MoveFamily::MovementRetreat { on_board: on_board.clone(), from_hand: true };
        let mut s = base_state();
        s.players[0].assistants.on_board = on_board.clone();
        assert_eq!(
            iter_family(&s, &rules, &retreat).collect::<Vec<_>>(),
            retreat_iter(&on_board).collect::<Vec<_>>(),
            "retreat dispatch must equal retreat_iter",
        );
        assert_eq!(family_len(&s, &rules, &retreat), retreat_len(&on_board));

        // Placement: iter_family == placement_iter, family_len ==
        // placement_upper_len.
        let mut ps = placement_state(2, 3);
        ps.players[0].assistants.in_hand = 3;
        ps.players[0].money = 100;
        assert_eq!(
            iter_family(&ps, &rules, &MoveFamily::Placement).collect::<Vec<_>>(),
            placement_iter(&ps, &rules).collect::<Vec<_>>(),
            "placement dispatch must equal placement_iter",
        );
        assert_eq!(
            family_len(&ps, &rules, &MoveFamily::Placement),
            placement_upper_len(&ps, &rules),
        );

        // Samples land in the right family (both families are never empty).
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(4);
        assert_eq!(
            family_kind(&sample_family(&s, &rules, &retreat, &mut rng).expect("retreat non-empty")),
            std::mem::discriminant(&retreat),
        );
        assert_eq!(
            family_kind(
                &sample_family(&ps, &rules, &MoveFamily::Placement, &mut rng)
                    .expect("placement non-empty"),
            ),
            std::mem::discriminant(&MoveFamily::Placement),
        );
    }

    /// The ChoiceResolution family delegates to the generator's choice
    /// resolutions: at a pending `AnyGood`, `iter_family` yields exactly the four
    /// `ChooseAnyGood` moves in generator order, `family_len` is 4, and a sample
    /// applies `Ok`.
    #[test]
    fn choice_resolution_family_delegates_to_generator() {
        use rand::SeedableRng;
        let rules = Rules::standard();
        let mut s = at_phase(TurnPhase::Placement);
        s.pending_choices.push(PendingChoice::AnyGood);

        let family = MoveFamily::ChoiceResolution;
        let got: Vec<Move> = iter_family(&s, &rules, &family).collect();
        assert_eq!(got, get_legal_moves(&s, &rules), "delegates to the whole choice list");
        assert_eq!(family_len(&s, &rules, &family), 4);
        assert!(got.iter().all(|m| matches!(m, Move::ChooseAnyGood { .. })));

        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(9);
        let mv = sample_family(&s, &rules, &family, &mut rng).expect("choice family non-empty");
        apply_move_unchecked(&s, &mv, &rules).expect("sampled choice resolution applies");
    }
}
