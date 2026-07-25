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
//! Task 1 (this commit) delivers [`MoveFamily`] + [`families`] — the dispatch
//! skeleton that mirrors `get_legal_moves`' queue-first + phase dispatch. The
//! per-family machinery (the retreat mixed-radix constructor/index/sample, the
//! placement combination construction, the small-family delegations) and the
//! lazy route enumerator land in later 3.1a/3.1b tasks.

use std::collections::BTreeMap;

use rand::Rng;

use crate::game::Rules;
use crate::types::*;

/// A tag naming one decision-phase move family legal at the current state.
///
/// Returned by [`families`], which mirrors [`get_legal_moves`]'s queue-first +
/// `match turn_phase` dispatch (`legal_moves.rs:9-24`) but yields these tags
/// instead of moves; later 3.1a/3.1b tasks page and sample the actual moves
/// per tag.
///
/// Only [`MoveFamily::MovementRetreat`] carries a payload: it owns a clone of
/// the current player's `on_board` assistant map so the exploding retreat
/// family can be indexed/sampled without re-reading `GameState`. Every other
/// family re-derives its moves from `&GameState` + `&Rules` at page time, so a
/// bare tag suffices — the owned snapshot is only paid where it removes a
/// borrow from the returned iterators.
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
/// `rules` is unused today — the dispatch is a pure function of the turn phase,
/// the pending-choice queue, and the President's location — but is part of the
/// shared `ActionSource` signature the later per-family functions use.
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
/// rejection-free and always yields an applicable move.
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

    /// Which `MoveFamily` a given `Move` belongs to, as a discriminant so the
    /// payload-carrying `MovementRetreat` compares by variant only. Exhaustive
    /// on purpose (no `_` arm): a new `Move` variant must be classified here,
    /// or this test stops compiling.
    fn family_kind(mv: &Move) -> std::mem::Discriminant<MoveFamily> {
        use std::mem::discriminant as d;
        match mv {
            Move::UseMercenary { .. }
            | Move::UseUniversity
            | Move::UseSpinningMill
            | Move::FulfillAchievement { .. }
            | Move::FulfillOrder { .. }
            | Move::EndAdditionalPhase => d(&MoveFamily::AdditionalAction),

            Move::PlaceThreeAreas { .. } | Move::PlaceTwoSameArea { .. } => {
                d(&MoveFamily::Placement)
            }

            Move::MovePresident { .. } | Move::PlacePresident { .. } => {
                d(&MoveFamily::MovementRoutes)
            }

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
}
