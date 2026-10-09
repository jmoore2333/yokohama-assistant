//! Single-candidate legality: `is_move_legal` / `is_president_path_legal`
//! (Phase 3.1b, spec `docs/plans/2026-07-25-phase3.1b-routes-validation-spec.md`
//! §3.2, settling the 3.1 spec's open decision #3).
//!
//! A search that PROPOSES one move — a widening step, a UCT child, a replayed
//! transposition — needs to know whether that one move is legal without
//! enumerating the family it came from. This module is that predicate.
//!
//! ## Design A, and the measurement that chose it
//!
//! Two designs were prototyped out of repo and measured at master `15dd5c5`
//! (release profile, this Mac; allocation COUNT is deterministic, wall clock is
//! recorded not asserted):
//!
//! - **A — this module:** re-state the cheap per-arm predicates over
//!   `&GameState`, delegating only where the frozen generator is the cheapest
//!   correct answer.
//! - **B — apply-and-see:** `apply_move_unchecked(state, mv, rules).is_ok()`,
//!   i.e. let the raw transition arms judge.
//!
//! | cell (R3 PROTOTYPE at master `15dd5c5`, 4P worst case — not this port) | A ns | A allocs | B ns | B allocs | B/A |
//! |---|---:|---:|---:|---:|---:|
//! | retreat ACCEPT (mid-family) | 12 | **0** | 777–856 | 43 | ~66× |
//! | retreat REJECT | 5 | **0** | 747–974 | 43 | ~158× |
//! | route ACCEPT (≥3 hops) | 305–321 | 3 | 1176–1241 | 50 | ~3.9× |
//! | route REJECT (non-adjacent hop) | 16–21 | 1 | 747–807 | 44 | ~43× |
//! | bare `GameState::clone` here | — | — | 747–826 | **43** | — |
//!
//! Across ordinary states the per-family spread is **2.5×–326×** in A's favour
//! (accept AND reject), because B's floor is a whole `GameState` clone. A is
//! allocation-free on every family except routes (`route_fee_vector` builds one
//! `BTreeMap` + one `Vec`, ≤3 events) and the two delegated families (3–5.5) —
//! so "allocation-free predicate" is the retreat claim, not a global one.
//!
//! **B is not merely slower, it is unsound alone.** The raw arms deliberately
//! defend only structure (`apply_move.rs:362-364` states the contract: semantic
//! legality is the CHECKED boundary's job), so `apply_move_unchecked` returns
//! `Ok` for moves that are not legal. Re-verified over 48,000 states (seeds
//! 0..40 × 2/3/4P): **Building without power 4+ 5,520 false accepts · on-board
//! movement reachability 3,306 · `PlacePresident` onto a non-Possible-Action
//! Area 84,579 · wrong PHASE 25,969 (100% — no raw arm reads `turn_phase` at
//! all)**. Design A rejects all four dimensions; the two Phase-2.9-hardened
//! control dimensions came back exactly 0, so that residue is the documented
//! one and nothing has drifted. B keeps its residue role — a last check on a
//! construction-correct source — and stays the ultimate authority through
//! checked `apply_move` outside the search loop.
//!
//! ## What "legal" means here: witnesses, not list membership
//!
//! [`get_legal_moves`] emits a CANONICAL list, not every legal encoding. Two
//! canonicalizations matter:
//!
//! 1. one lex-least witness PATH per `(destination, RouteVector)` label
//!    (`enumerate_president_routes`), and
//! 2. a free teleport is suppressed when a zero-cost walk already covers its
//!    destination (`legal_moves.rs:301-367`).
//!
//! [`PathMode::RulesLegal`] accepts any rules-legal WITNESS, so it is
//! deliberately WIDER than list membership: the prototype's sweep found 588
//! disagreements against `get_legal_moves` and every one was a non-canonical
//! witness for a label the generator does emit — **zero new labels**. That is
//! the contract this module promises and its tests gate: an accepted
//! `MovePresident` always shares its `(destination, RouteVector)` LABEL with
//! something in the list, never its raw path. Every other variant is a
//! canonical singleton, so for those, acceptance and membership coincide.
//! [`PathMode::Canonical`] adds back (2) only — see its docs.
//!
//! Prototype totals at `15dd5c5`: 0 false rejects on 4,582,426 legal moves; 0
//! apply-errors on 4,583,014 accepts; 0 over-rejections on 1,811,838 constructed
//! illegals.
//!
//! ## Drift guard (standing)
//!
//! `is_move_legal` matches `Move` EXHAUSTIVELY with no `_` arm, so a new variant
//! is a compile error here rather than a silent rejection — the same guard
//! `search_actions::family_kind` uses. The arms RE-STATE logic that lives in two
//! frozen files (`legal_moves.rs`, `apply_move.rs`), which cannot be imported
//! (the generators and validators are private) and must not be edited; each
//! mirror therefore carries a citation to the lines it mirrors, and the
//! agreement is pinned by tests, not by the comments: every legal move is
//! accepted (no false rejects), every acceptance applies cleanly, and every
//! acceptance's label is in the list.

use crate::game::movement::can_step_onto;
use crate::game::{
    calculate_place_president_fee, calculate_placement_cost, can_claim_achievement,
    can_fulfill_order, can_teleport_to_station_with_tech, can_use_spinning_mill, can_use_university,
    generate_choice_resolutions, get_legal_moves, has_tech_effect,
    is_possible_action_area_with_tech, route_fee_vector, Rules,
};
use crate::types::*;

/// How strictly [`is_move_legal`] treats `get_legal_moves`' canonicalization of
/// President movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathMode {
    /// RULES legality: accept ANY legal witness path. A non-canonical witness
    /// for a label the generator emits is an ACCEPT — it applies identically
    /// (same destination, same fees, same recipients), it is just not the path
    /// the generator chose to print.
    RulesLegal,
    /// Additionally require the canonicalization the generator applies where
    /// deciding it is cheap: a free teleport (`path: []`) is canonical only when
    /// no ZERO-COST WALK reaches that destination, because the generator
    /// suppresses the duplicate (`legal_moves.rs:301-367`).
    ///
    /// What this mode does NOT promise: it does not verify lex-least witness
    /// SELECTION. Deciding whether a given zero-cost walk is the one the
    /// generator would print requires enumerating that destination's routes —
    /// exactly the cost this predicate exists to avoid — so a non-canonical
    /// witness for an emitted label is still accepted in `Canonical` mode. Use
    /// `route_enum::movement_routes_family` when the canonical LIST is what you
    /// need; use this when you need a candidate judged.
    Canonical,
}

/// Whether `mv` is legal at `state` — a single-candidate answer that never
/// enumerates the family `mv` belongs to.
///
/// Dispatch mirrors [`get_legal_moves`] (`legal_moves.rs:9-24`): queue-first —
/// while a `PendingChoice` heads `pending_choices`, ONLY that choice's
/// resolutions are legal, whatever the turn phase — then per-variant, each arm
/// gated on the phase that emits it. That phase gate is what closes the Tier-C
/// phase dimension (25,969 false accepts for `apply_move_unchecked`, a 100%
/// rate: no raw arm reads `turn_phase`).
///
/// Two families DELEGATE to the frozen generator rather than re-state it, both
/// measured small (`AreaAction` mean 4.46 / max 154 moves, `ChoiceResolution`
/// max 114):
/// - `ExecuteAreaAction` — `generate_area_action_moves` is private in the frozen
///   `legal_moves.rs`, and at `TurnPhase::AreaAction` with an empty queue
///   `get_legal_moves` IS that generator (both delegations' meaning is pinned by
///   `the_delegated_families_are_phase_pure`);
/// - choice resolutions — `generate_choice_resolutions` is `pub`, so the queue
///   arm calls it directly, which is literally `get_legal_moves`' own queue
///   short-circuit.
///
/// See [`PathMode`] for what `mode` changes; see the module docs for why
/// acceptance is WIDER than membership in `get_legal_moves`.
pub fn is_move_legal(state: &GameState, mv: &Move, rules: &Rules, mode: PathMode) -> bool {
    // Queue short-circuit (`legal_moves.rs:9-11`).
    if let Some(head) = state.pending_choices.first() {
        return generate_choice_resolutions(state, head, rules).contains(mv);
    }

    let player = state.current_player;
    let p = state.player(player);
    let phase = state.turn_phase;
    let techs = Some(&rules.techs);
    // The two additional-action phases share one generator, and `Recovery` /
    // `TurnEnd` reach no arm at all — every arm below names the phase that
    // emits it, which is what closes the Tier-C phase dimension.
    let additional =
        matches!(phase, TurnPhase::AdditionalActionFirst | TurnPhase::AdditionalActionSecond);

    match mv {
        // -- additional actions (`legal_moves.rs:27-86`) ----------------------
        Move::EndAdditionalPhase => additional,
        Move::UseMercenary { country, target_area } => {
            additional
                && !state.additional_action_state.mercenary_used
                && p.mercenaries.available.get(country).copied().unwrap_or(0) > 0
                && placeable(state, *target_area)
                && p.assistants.in_area(*target_area) > 0
                && p.president_location != PresidentLocation::OnBoard(*target_area)
        }
        Move::FulfillOrder { order_id } => {
            additional
                && p.order_cards_in_hand.contains(order_id)
                && can_fulfill_order(state, player, *order_id, &rules.orders)
        }
        Move::FulfillAchievement { achievement } => {
            additional
                && state.achievements.iter().any(|a| a.achievement_type == *achievement)
                && can_claim_achievement(state, player, *achievement)
        }
        Move::UseUniversity => additional && can_use_university(state, player, &rules.techs),
        Move::UseSpinningMill => additional && can_use_spinning_mill(state, player, &rules.techs),

        // -- placement (`legal_moves.rs:115-246`) -----------------------------
        Move::PlaceThreeAreas { placements } => {
            phase == TurnPhase::Placement && distinct_area_placement_legal(state, placements, rules)
        }
        Move::PlaceTwoSameArea { area, adjacent_bonus } => {
            phase == TurnPhase::Placement
                && same_area_placement_legal(state, *area, *adjacent_bonus, rules)
        }

        // -- movement (`legal_moves.rs:248-377`) ------------------------------
        Move::MovePresident { path, destination } => {
            phase == TurnPhase::Movement
                && is_president_path_legal(state, player, path, *destination, rules, mode)
        }
        Move::PlacePresident { destination } => {
            phase == TurnPhase::Movement
                // The from-hand encoding, and a real board area: both are what
                // `apply_place_president` itself refuses (`apply_move.rs:569-574`).
                // ALIGNED 2026-07-25 (`SEMANTICS_REV` 4): this arm used to carry
                // the module's ONE narrowing — an explicit `placeable(..)`
                // conjunct siding with apply, because the generator emitted
                // `PlacePresident{Canal}` on a fixture with an own assistant on
                // the Canal while every apply path rejected it. The exclusion now
                // lives in `is_possible_action_area_full` (`movement.rs`), the
                // shared predicate the generator's arm, both route searches,
                // `route_enum::RouteGraph` and this line all call — so generator,
                // predicate and apply agree BY CONSTRUCTION and the conjunct is
                // redundant. Zero narrowings remain.
                && p.president_location == PresidentLocation::InHand
                && is_possible_action_area_with_tech(state, player, *destination, techs)
                && calculate_place_president_fee(state, player, *destination, techs) <= p.money
        }
        // Retreat legality is the per-area count bound and nothing else — the
        // complete raw arm (`apply_move.rs:634-639`). The generator's product
        // (`enumerate_recovery_options`) is that bound enumerated; a single
        // candidate needs no enumeration, which is why the six-figure family
        // costs nothing to validate.
        Move::ReturnPresidentToHand { recover } => {
            phase == TurnPhase::Movement
                && recover.iter().all(|(&area, &n)| n >= 1 && n <= p.assistants.in_area(area))
        }

        // -- area action: DELEGATED (see this function's docs) ----------------
        Move::ExecuteAreaAction { .. } => {
            phase == TurnPhase::AreaAction && get_legal_moves(state, rules).contains(mv)
        }

        // -- 5-Power bonus (`legal_moves.rs:862-886`) -------------------------
        Move::Skip5PowerBonus => phase == TurnPhase::FivePowerBonus,
        Move::Take5PowerToken => {
            phase == TurnPhase::FivePowerBonus
                && state.current_action_power == Some(5)
                && matches!(
                    acting_area(state).and_then(|a| state.board_layout.five_power_tokens.get(&a)),
                    Some(Some(_)),
                )
        }

        // -- building (`legal_moves.rs:888-942`) ------------------------------
        Move::SkipBuilding => phase == TurnPhase::Building,
        Move::BuildShophouse { slot } => {
            phase == TurnPhase::Building
                && buildable_site(state).is_some_and(|site| {
                    p.shophouses.in_hand > 0
                        && !site.shophouses_built.contains(&Some(player))
                        && site.shophouses_built.get(*slot as usize).is_some_and(Option::is_none)
                })
        }
        Move::BuildTradingHouse => {
            phase == TurnPhase::Building
                && buildable_site(state).is_some_and(|site| {
                    p.trading_houses.in_hand > 0 && site.can_build_trading_house()
                })
        }

        // -- choice resolutions -----------------------------------------------
        // Legal ONLY while their choice heads the queue, which returned above.
        // With an empty queue nothing here is legal, whatever the phase.
        Move::ChooseAnyGood { .. }
        | Move::ChooseWarehouseTake { .. }
        | Move::DeclineWarehouseTake
        | Move::ChooseAssistantMove { .. }
        | Move::DeclineAssistantMove
        | Move::ChooseChurchSpace { .. }
        | Move::ChooseCustomsSpace { .. }
        | Move::ChooseRecoveryLeave { .. }
        | Move::ChooseStationPlace { .. } => false,
    }
}

/// Whether `mv` is IN `get_legal_moves(state, rules)` — a single-candidate
/// answer that never pays the Movement phase's retreat product (the family
/// that reaches 442,368 moves in legal play; the two families `is_move_legal`
/// DELEGATES, area actions and choice resolutions, still materialize their
/// own measured-small lists — max 154 and 114) and, unlike [`is_move_legal`],
/// EXACT on the one family where acceptance is wider than membership.
///
/// `is_move_legal` accepts any legal WITNESS path for a `MovePresident` label
/// the generator emits (the module doc's "witnesses, not list membership"),
/// while the list prints one lex-least witness per label. So for the Movement
/// family — `MovePresident` and `PlacePresident` — the only exact check is the
/// family's own list, `crate::game::route_enum::movement_routes_family`:
/// byte for byte the list's Movement subsequence (`action_source_equivalence`
/// pins it), small (max observed 694 adversarial / 66 organic), compute-per-
/// call. That family consults nothing about `turn_phase` or the choice queue by
/// contract, so `is_move_legal`'s phase and queue gates are taken FIRST. For
/// every other variant acceptance and membership coincide (the module doc), so
/// `is_move_legal` IS the answer.
///
/// The proof: `is_listed_move_is_exactly_membership_in_the_legal_list` and
/// `is_listed_move_reads_the_phase_the_list_reads` below, and
/// `tests/replay_check_equivalence.rs` for the replay built on this (the
/// 2026-10-08 ruling R-SEC2). What this function is FOR: a replay of an
/// untrusted record whose cost must not follow the legal families' sizes.
pub fn is_listed_move(state: &GameState, mv: &Move, rules: &Rules) -> bool {
    match mv {
        Move::MovePresident { .. } | Move::PlacePresident { .. } => {
            is_move_legal(state, mv, rules, PathMode::Canonical)
                && crate::game::route_enum::movement_routes_family(state, rules)
                    .iter()
                    .any(|listed| listed == mv)
        }
        _ => is_move_legal(state, mv, rules, PathMode::Canonical),
    }
}

/// Option A: one assistant into each of 1..=N DISTINCT areas, or none at all
/// (`legal_moves.rs:115-196`).
fn distinct_area_placement_legal(state: &GameState, placements: &[AreaId], rules: &Rules) -> bool {
    let player = state.current_player;
    let p = state.player(player);

    if p.assistants.in_hand == 0 {
        // Degenerate case: the empty placement is the ONLY move offered
        // (`legal_moves.rs:121-125`).
        return placements.is_empty();
    }
    if placements.is_empty() {
        return true;
    }

    let max_areas =
        if has_tech_effect(state, player, TechEffect::Newspaper, &rules.techs) { 4 } else { 3 };
    if placements.len() > max_areas || placements.len() > p.assistants.in_hand as usize {
        return false;
    }
    // STRICTLY ASCENDING, which carries Option A's distinctness AND the
    // generator's canonical order (`legal_moves.rs:159-195` builds j<k<l over
    // `BTreeMap` key order; `search_actions::place_combo` matches it). A
    // permuted tuple is rules-legal and applies fine, but it is not the move
    // the list carries — accepting it would let a search count one combination
    // twice.
    if placements.windows(2).any(|pair| pair[0] >= pair[1]) {
        return false;
    }
    placements.iter().all(|&area| placeable(state, area))
        // Priced PER COMBINATION: affordability is not a per-area property once
        // two occupied areas are in the same tuple.
        && calculate_placement_cost(state, player, placements) <= p.money
}

/// Option B: two assistants into one area, optionally with Telegram's adjacent
/// bonus (`legal_moves.rs:198-243`, fees per assistant placed).
fn same_area_placement_legal(
    state: &GameState,
    area: AreaId,
    adjacent_bonus: Option<AreaId>,
    rules: &Rules,
) -> bool {
    let player = state.current_player;
    let p = state.player(player);
    if p.assistants.in_hand < 2 || !placeable(state, area) {
        return false;
    }
    // Two assistants land here, so the fee is priced off a 2-element slice.
    let cost = calculate_placement_cost(state, player, &[area, area]);
    if p.money < cost {
        return false;
    }
    match adjacent_bonus {
        None => true,
        Some(adj) => {
            has_tech_effect(state, player, TechEffect::Telegram, &rules.techs)
                && p.assistants.in_hand >= 3
                && placeable(state, adj)
                && state.board_layout.adjacent_areas(area).contains(&adj)
                // Canal crossings are movement-transit edges ONLY (design
                // 2026-07-18 decision log, Task 9) — the same exclusion the
                // generator and `validate_telegram_bonus` both apply.
                && !state.board_layout.is_canal_crossing(area, adj)
                && p.money >= cost + calculate_placement_cost(state, player, &[adj])
        }
    }
}

/// The area the current action happens in: the mercenary target during a
/// mercenary sub-turn, else the President's own area (`legal_moves.rs:869-876`
/// and `:899-909`, which resolve it identically).
fn acting_area(state: &GameState) -> Option<AreaId> {
    if state.is_mercenary_action {
        return state.mercenary_target_area;
    }
    match state.player(state.current_player).president_location {
        PresidentLocation::OnBoard(area) => Some(area),
        PresidentLocation::InHand => None,
    }
}

/// The building site a build move would land on, or `None` when nothing can be
/// built here — including the POWER 4+ gate, the dimension the raw arms never
/// check (5,520 measured false accepts).
fn buildable_site(state: &GameState) -> Option<&BuildingSite> {
    if state.current_action_power.unwrap_or(0) < 4 {
        return None;
    }
    state.board_layout.building_sites.get(&acting_area(state)?)
}

/// Whether `area` exists on this board and is not the Canal — the structural
/// check every `AreaId`-carrying arm makes (`require_placeable_area`,
/// `apply_move.rs:300-310`).
fn placeable(state: &GameState, area: AreaId) -> bool {
    state.board_layout.areas.get(&area).is_some_and(|c| c.area_type.allows_placement())
}

/// Whether an opponent's President stands on `area`.
fn opponent_president_on(state: &GameState, player: PlayerId, area: AreaId) -> bool {
    state
        .players
        .iter()
        .any(|o| o.id != player && o.president_location == PresidentLocation::OnBoard(area))
}

/// Whether moving the President along `path` to `destination` is legal for
/// `player` at `state` — the §3.6 contract, and the one genuinely new piece of
/// logic in this module.
///
/// An EMPTY `path` is the free-teleport encoding (Station/Tram), not a walk.
///
/// The contract, in the order it is checked:
/// 1. the President is ON BOARD (`PlacePresident` is the from-hand encoding)
///    and `destination` is not this turn's start area;
/// 2. teleports: `Station` (`can_teleport_to_station_with_tech == Some(dest)`)
///    or `Tram` (owned, `dest` placeable, `dest` free of opponent Presidents),
///    `dest != current`, plus [`PathMode::Canonical`]'s suppression rule;
/// 3. walks: `path` ends at `destination`, each hop exists and is placeable,
///    consecutive hops are adjacent, no area repeats and the start is never
///    re-entered — MIRRORING the frozen `validate_president_path`
///    (`apply_move.rs:375-421`), which cannot be called (private, frozen file);
/// 4. each hop passes `can_step_onto` — the real predicate
///    (`movement.rs:214`, `pub(crate)` for this), never a copy, so the
///    assistant-chain rule cannot drift between the searches and this;
/// 5. `destination` is a Possible Action Area (`is_possible_action_area_full`
///    via `is_possible_action_area_with_tech`). WITHOUT this the predicate
///    false-accepts the whole opponent-blocked-destination class — 3,306 of the
///    measured Tier-C movement false accepts;
/// 6. affordability: `route_fee_vector(path).values().sum() <= money`, the same
///    single pricing source `apply_movement` charges from.
///
/// DRIFT GUARD: step 3 re-states ~35 lines of a frozen file. The mirror is
/// pinned by tests, not by this comment — `accepted_candidates_apply_and_add_no_new_labels`
/// fails the moment an acceptance stops applying cleanly.
///
/// `player` is a parameter for signature symmetry, but the no-return-to-start
/// rule reads `state.president_start_area` — a TURN-GLOBAL latched for the
/// CURRENT player's Movement step — so passing another seat applies the wrong
/// start restriction. Call it for `state.current_player` (as `is_move_legal`
/// does) unless you know the field's provenance.
pub fn is_president_path_legal(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
    destination: AreaId,
    rules: &Rules,
    mode: PathMode,
) -> bool {
    let techs = Some(&rules.techs);
    let PresidentLocation::OnBoard(current) = state.player(player).president_location else {
        return false;
    };

    // (1) "The President pawn may not return to the Area that it started in"
    // — mirrors `validate_movement_destination` (`apply_move.rs:243-250`),
    // which runs for teleports and walks alike.
    if state.president_start_area == Some(destination) {
        return false;
    }

    if path.is_empty() {
        // (2) The free-teleport encoding. Entitlement is exactly what the
        // generator checks (`legal_moves.rs:316-367`); the raw arm deliberately
        // does not check it at all (Tier C, `apply_move.rs:362-364`).
        if destination == current {
            return false;
        }
        let station = can_teleport_to_station_with_tech(state, player, techs) == Some(destination);
        let tram = has_tech_effect(state, player, TechEffect::Tram, &rules.techs)
            && placeable(state, destination)
            && !opponent_president_on(state, player, destination);
        if !(station || tram) {
            return false;
        }
        return match mode {
            PathMode::RulesLegal => true,
            PathMode::Canonical => {
                !zero_cost_walk_exists(state, player, current, destination, rules)
            }
        };
    }

    // (3) STRUCTURE — the mirror of the frozen `validate_president_path`
    // (`apply_move.rs:375-421`): the path ends where it claims, every hop is a
    // real placeable area, consecutive hops are adjacent (board adjacency, so
    // canal-crossing edges count), and no area repeats, the start included.
    // DRIFT GUARD: these are the ~35 mirrored lines; they cannot be imported
    // (private fn in a frozen file) and are pinned by the apply-Ok property
    // instead.
    if *path.last().expect("non-empty") != destination {
        return false;
    }
    let mut seen: Vec<AreaId> = Vec::with_capacity(path.len() + 1);
    seen.push(current);
    let mut prev = current;
    for &hop in path {
        if !placeable(state, hop)
            || seen.contains(&hop)
            || !state.board_layout.adjacent_areas(prev).contains(&hop)
            // (4) The assistant chain, via the REAL predicate — the same
            // function both frozen searches hop with, never a transcription.
            || !can_step_onto(state, player, current, hop)
        {
            return false;
        }
        seen.push(hop);
        prev = hop;
    }

    // (5) The destination must be a Possible Action Area: own assistant AND no
    // opponent President unless Postal System. Structure alone would accept the
    // whole opponent-blocked class.
    if !is_possible_action_area_with_tech(state, player, destination, techs) {
        return false;
    }

    // (6) Affordability, off the one pricing source `apply_movement` charges
    // from — so a route this accepts is a route the mover can actually pay.
    let cost: u32 = route_fee_vector(state, player, path, techs, false).values().sum();
    cost <= state.player(player).money
}

/// Whether a ZERO-COST walk reaches `dest` — the generator's teleport
/// suppression predicate (`legal_moves.rs:301-327`), and the only extra work
/// [`PathMode::Canonical`] does.
///
/// A flood fill over the mover's own-assistant areas across zero-cost edges
/// only: polynomial in the board, never the simple-path DFS. Sound because a
/// zero-cost route is exactly a walk in that subgraph — every fee
/// `route_fee_vector` charges is attached to an area entered (opponent
/// President, waived by Stagecoach) or an edge crossed (canal toll, never
/// waived), so a walk avoiding all of them costs 0, and any walk contains a
/// simple path with the same endpoints and no extra fees.
fn zero_cost_walk_exists(
    state: &GameState,
    player: PlayerId,
    current: AreaId,
    dest: AreaId,
    rules: &Rules,
) -> bool {
    if !is_possible_action_area_with_tech(state, player, dest, Some(&rules.techs)) {
        return false;
    }
    let stagecoach = has_tech_effect(state, player, TechEffect::Stagecoach, &rules.techs);
    let free_edge = |from: AreaId, to: AreaId| {
        !state.board_layout.is_canal_crossing(from, to)
            && (stagecoach || !opponent_president_on(state, player, to))
    };

    let mut seen = vec![current];
    let mut frontier = vec![current];
    while let Some(at) = frontier.pop() {
        for &adj in state.board_layout.adjacent_areas(at) {
            if seen.contains(&adj)
                || !can_step_onto(state, player, current, adj)
                || !free_edge(at, adj)
            {
                continue;
            }
            if adj == dest {
                return true;
            }
            seen.push(adj);
            frontier.push(adj);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{apply_move_unchecked, RouteVector};
    use std::collections::BTreeMap;

    // -- fixtures -------------------------------------------------------------
    //
    // Hand-built like `route_enum`'s and `movement.rs`'s, and deliberately NOT
    // shared with them: `#[cfg(test)]` fixtures are per-module in this crate
    // today, and a shared `tests/route_fixtures` module is a named follow-up
    // (T4 review), not a mid-wave reshape.

    fn colors(n: u8) -> Vec<PlayerColor> {
        [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green][..n as usize]
            .to_vec()
    }

    /// A `Movement`-phase state over `board_layout`: every seat holds 5 Yen,
    /// 3 assistants in hand and its President in hand.
    fn base_state(players: u8, board_layout: BoardLayout) -> GameState {
        let config = GameConfig::new(players, colors(players), PlayerId::new(0));
        let mut seats = Vec::new();
        for i in 0..players {
            let mut p = PlayerState::new(PlayerId::new(i), colors(players)[i as usize], i == 0);
            p.money = 5;
            p.assistants.in_hand = 3;
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

    /// A `BoardLayout` from ids + undirected edges; `canals` marks which of
    /// those edges toll. Area types are irrelevant to legality here (only
    /// assistants, Presidents, adjacency and crossings are read), so every tile
    /// is a Bank.
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

    /// The diamond every movement fixture is built on: `0` branches to `1` and
    /// `2`, which rejoin at `3`. The mover's President stands on `0` with own
    /// assistants on 1, 2 and 3; opponents' Presidents start in hand.
    fn diamond(players: u8, canals: &[(u8, u8)]) -> GameState {
        let mut s =
            base_state(players, board(&[0, 1, 2, 3], &[(0, 1), (0, 2), (1, 3), (2, 3)], canals));
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId(0));
        for a in [1u8, 2, 3] {
            s.players[0].assistants.on_board.insert(AreaId(a), 1);
        }
        s
    }

    /// Give player 0 a technology with `effect`; ids are allocated per effect so
    /// several calls stack without colliding.
    fn grant_tech(state: &mut GameState, rules: &mut Rules, effect: TechEffect) {
        let (id, name) = match effect {
            TechEffect::PostalSystem => (TechCardId(101), "Postal System"),
            TechEffect::Stagecoach => (TechCardId(102), "Stagecoach"),
            TechEffect::Station => (TechCardId(103), "Station"),
            TechEffect::Tram => (TechCardId(104), "Tram"),
            TechEffect::Newspaper => (TechCardId(105), "Newspaper"),
            TechEffect::Telegram => (TechCardId(106), "Telegram"),
            _ => panic!("fixture grants Postal/Stagecoach/Station/Tram/Newspaper/Telegram only"),
        };
        rules.techs.insert(
            TechCard { id, name: name.to_string(), country: Country::Dutch, industry_cost: 2, effect },
            name,
        );
        state.players[0].technologies.push(id);
    }

    /// The Station diamond: Station tech owned, token on `station`.
    fn station_diamond(canals: &[(u8, u8)], station: u8) -> (GameState, Rules) {
        let mut rules = Rules::empty_for_tests();
        let mut s = diamond(2, canals);
        grant_tech(&mut s, &mut rules, TechEffect::Station);
        s.station_location = Some(AreaId(station));
        (s, rules)
    }

    fn legal(state: &GameState, rules: &Rules, mv: &Move) -> bool {
        is_move_legal(state, mv, rules, PathMode::RulesLegal)
    }

    fn canonical(state: &GameState, rules: &Rules, mv: &Move) -> bool {
        is_move_legal(state, mv, rules, PathMode::Canonical)
    }

    // -- the label a candidate is compared by ---------------------------------

    /// What an acceptance is allowed to be. `MovePresident` compares by
    /// `(destination, RouteVector)` LABEL and never by raw path — the generator
    /// prints one lex-least witness per label, so a legal non-canonical witness
    /// must not read as a new move (module docs: 588 such witnesses measured,
    /// ZERO new labels). Every other variant is a canonical singleton and
    /// compares by equality.
    #[derive(Debug, PartialEq, Eq)]
    enum Label {
        Route(AreaId, RouteVector),
        Exact(Move),
    }

    fn label(state: &GameState, rules: &Rules, mv: &Move) -> Label {
        match mv {
            Move::MovePresident { path, destination } => Label::Route(
                *destination,
                route_fee_vector(state, state.current_player, path, Some(&rules.techs), false),
            ),
            other => Label::Exact(other.clone()),
        }
    }

    // -- driven sweep ---------------------------------------------------------

    /// Drive uniform-random games (2/3/4P × `seeds`, capped at `steps` applied
    /// moves) and hand every decision point to `visit` with its legal list.
    fn drive(seeds: u64, steps: usize, mut visit: impl FnMut(&GameState, &Rules, &[Move])) {
        use rand::{Rng, SeedableRng};
        let rules = Rules::standard();

        for players in [2u8, 3, 4] {
            for seed in 0..seeds {
                let config = GameConfig::for_player_count(players);
                let pending = crate::new_game(config, &rules, seed).expect("new_game");
                let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
                let choices: Vec<crate::StartingOrderChoice> = (0..players)
                    .map(|_| crate::StartingOrderChoice::Keep(rng.gen_range(0..2u8)))
                    .collect();
                let mut state = crate::finalize_setup(pending, &choices).expect("finalize_setup");

                for _ in 0..steps {
                    if crate::is_game_over(&state) {
                        break;
                    }
                    let legal_moves = get_legal_moves(&state, &rules);
                    if legal_moves.is_empty() {
                        break;
                    }
                    visit(&state, &rules, &legal_moves);
                    let mv = legal_moves[rng.gen_range(0..legal_moves.len())].clone();
                    state = apply_move_unchecked(&state, &mv, &rules).expect("apply");
                }
            }
        }
    }

    /// Candidates to judge at `state`: the legal list plus constructed
    /// mutations of it, plus a fixed cross-phase battery. Deliberately a mix of
    /// legal and illegal — the sweep's properties are about what the predicate
    /// ACCEPTS, so the pool must contain things it should not.
    fn candidates(state: &GameState, legal_moves: &[Move]) -> Vec<Move> {
        let mut out: Vec<Move> = legal_moves.to_vec();
        let areas: Vec<AreaId> = state.board_layout.areas.keys().copied().collect();
        let first = areas.first().copied().unwrap_or(AreaId::new(0));
        let last = areas.last().copied().unwrap_or(AreaId::new(0));

        for mv in legal_moves {
            match mv {
                Move::MovePresident { path, destination } => {
                    // A witness that does not end where it claims.
                    out.push(Move::MovePresident { path: path.clone(), destination: last });
                    // The bare teleport encoding for a walked destination.
                    out.push(Move::MovePresident { path: vec![], destination: *destination });
                    if let Some(&head) = path.first() {
                        // A repeated hop (fees would be paid twice).
                        let mut repeated = path.clone();
                        repeated.insert(0, head);
                        out.push(Move::MovePresident {
                            path: repeated,
                            destination: *destination,
                        });
                    }
                    if path.len() >= 2 {
                        let mut reversed = path.clone();
                        reversed.reverse();
                        let dest = *reversed.last().expect("non-empty");
                        out.push(Move::MovePresident { path: reversed, destination: dest });
                    }
                }
                Move::PlaceThreeAreas { placements } if placements.len() >= 2 => {
                    let mut permuted = placements.clone();
                    permuted.reverse();
                    out.push(Move::PlaceThreeAreas { placements: permuted });
                    let mut duplicated = placements.clone();
                    duplicated[1] = duplicated[0];
                    out.push(Move::PlaceThreeAreas { placements: duplicated });
                }
                Move::ReturnPresidentToHand { recover } if !recover.is_empty() => {
                    let mut over = recover.clone();
                    for count in over.values_mut() {
                        *count += 1;
                    }
                    out.push(Move::ReturnPresidentToHand { recover: over });
                    let mut zeroed = recover.clone();
                    for count in zeroed.values_mut() {
                        *count = 0;
                    }
                    out.push(Move::ReturnPresidentToHand { recover: zeroed });
                }
                _ => {}
            }
        }

        // Cross-phase battery: at any given decision point most of these belong
        // to another phase, which is the point (the Tier-C phase dimension).
        out.extend([
            Move::EndAdditionalPhase,
            Move::UseUniversity,
            Move::UseSpinningMill,
            Move::PlaceThreeAreas { placements: vec![] },
            Move::PlaceThreeAreas { placements: vec![first] },
            Move::PlaceTwoSameArea { area: first, adjacent_bonus: None },
            Move::PlaceTwoSameArea { area: first, adjacent_bonus: Some(last) },
            Move::PlacePresident { destination: first },
            Move::MovePresident { path: vec![last], destination: last },
            Move::MovePresident { path: vec![], destination: last },
            Move::ReturnPresidentToHand { recover: BTreeMap::new() },
            Move::ExecuteAreaAction { action: AreaAction::Skip },
            Move::ExecuteAreaAction { action: AreaAction::Bank },
            Move::Take5PowerToken,
            Move::Skip5PowerBonus,
            Move::BuildShophouse { slot: 0 },
            Move::BuildShophouse { slot: 9 },
            Move::BuildTradingHouse,
            Move::SkipBuilding,
            Move::ChooseAnyGood { good: TradeGood::Copper },
            Move::DeclineWarehouseTake,
            Move::DeclineAssistantMove,
            Move::ChooseRecoveryLeave { leave_one: true },
        ]);

        out
    }

    // -- (a) + (b) the two acceptance properties ------------------------------

    /// THE soundness pair, swept over constructed candidates at every decision
    /// point of 9 driven games:
    ///
    /// (a) every acceptance APPLIES — `apply_move_unchecked` returns `Ok`. Its
    ///     coverage limit, stated honestly: the raw arms do not check the
    ///     Tier-C dimensions (phase, Building power, movement reachability,
    ///     `PlacePresident` possible-area), so this property alone cannot catch
    ///     over-acceptance THERE — which is exactly why (b) and the constructed
    ///     Tier-C rejections below exist.
    ///
    /// (b) every acceptance is an already-known LABEL: for `MovePresident` the
    ///     `(destination, RouteVector)` label must be in the legal list's label
    ///     set (never the raw path — a non-canonical witness is a legitimate
    ///     accept), for every other variant the move itself must be. ZERO NEW
    ///     LABELS is the contract.
    ///
    /// Also pins `Canonical ⊆ RulesLegal`, the mode ordering.
    #[test]
    fn accepted_candidates_apply_and_add_no_new_labels() {
        let mut accepted = 0usize;
        let mut rejected = 0usize;
        let mut non_canonical_witnesses = 0usize;

        drive(3, 400, |state, rules, legal_moves| {
            let known: Vec<Label> =
                legal_moves.iter().map(|mv| label(state, rules, mv)).collect();

            for mv in candidates(state, legal_moves) {
                if !legal(state, rules, &mv) {
                    rejected += 1;
                    assert!(
                        !canonical(state, rules, &mv),
                        "Canonical must be no wider than RulesLegal: {mv:?}",
                    );
                    continue;
                }
                accepted += 1;

                assert!(
                    apply_move_unchecked(state, &mv, rules).is_ok(),
                    "(a) accepted move failed to apply at {:?}: {mv:?}",
                    state.turn_phase,
                );

                let got = label(state, rules, &mv);
                assert!(
                    known.contains(&got),
                    "(b) accepted move introduced a NEW label at {:?}: {mv:?} -> {got:?}",
                    state.turn_phase,
                );
                // An acceptance that is NOT in the list can only be a
                // non-canonical `MovePresident` witness: every other variant
                // compares by equality, so (b) above already forced it into the
                // list. Counting them is how this sweep proves it exercised the
                // `RulesLegal` contract at all.
                if !legal_moves.contains(&mv) {
                    assert!(
                        matches!(mv, Move::MovePresident { .. }),
                        "only movement has non-canonical encodings: {mv:?}",
                    );
                    non_canonical_witnesses += 1;
                }
            }
        });

        assert!(accepted > 1_000, "vacuous sweep: only {accepted} acceptances");
        assert!(rejected > 1_000, "vacuous sweep: only {rejected} rejections");
        assert!(
            non_canonical_witnesses > 0,
            "no non-canonical witness was ever accepted — the RulesLegal contract is untested",
        );
    }

    // -- is_listed_move: exact membership (R-SEC2) -----------------------------

    /// `is_listed_move` is EXACTLY membership in the legal list (the 2026-10-08
    /// ruling R-SEC2's move-level half): at every decision point of random
    /// games at 2/3/4P, every listed move passes, and every constructed
    /// candidate reads as the list reads it. Three counters are the
    /// anti-vacuity, each naming the mechanism it proves was exercised:
    /// `conjunct_cases` — candidates `PathMode::Canonical` ACCEPTS and the
    /// list LACKS (non-lex-least witnesses; `witness_variants` builds them
    /// from every walked move), where only the family conjunct can side with
    /// the list; `queued_family_refusals` — REAL members of the phase-blind
    /// Movement family judged while a choice heads the queue, where only the
    /// `is_move_legal` conjunct — its queue short-circuit — refuses them,
    /// since the family lists them (Review Focus 3); `out_of_phase_family_refusals`
    /// — the same outside the Movement phase, refused by its phase gate alone.
    /// The cells pin that both gates are PRESENT in the conjunction (the
    /// family-alone mutant turns both red); taking them first is the cost
    /// order, not what these counters prove.
    #[test]
    fn is_listed_move_is_exactly_membership_in_the_legal_list() {
        let mut listed = 0usize;
        let mut candidates_seen = 0usize;
        let mut conjunct_cases = 0usize;
        let mut queued_family_refusals = 0usize;
        let mut out_of_phase_family_refusals = 0usize;
        drive(3, 400, |state, rules, legal_moves| {
            for mv in legal_moves {
                assert!(
                    is_listed_move(state, mv, rules),
                    "a listed move must pass at {:?}: {mv:?}",
                    state.turn_phase,
                );
                listed += 1;
            }
            let mut pool = candidates(state, legal_moves);
            pool.extend(witness_variants(state, legal_moves));
            for mv in pool {
                candidates_seen += 1;
                let in_list = legal_moves.contains(&mv);
                assert_eq!(
                    is_listed_move(state, &mv, rules),
                    in_list,
                    "disagrees with the list at {:?}: {mv:?}",
                    state.turn_phase,
                );
                if !in_list && canonical(state, rules, &mv) {
                    conjunct_cases += 1;
                }
            }
            // The phase-blind family's REAL members, judged where the list
            // cannot hold them: under a queued choice, or in any phase but
            // Movement. The family is computed here exactly as the conjunct
            // computes it, so under the family-alone mutant every member
            // reads as listed.
            let queued = !state.pending_choices.is_empty();
            if queued || state.turn_phase != TurnPhase::Movement {
                for mv in crate::game::route_enum::movement_routes_family(state, rules) {
                    assert!(!legal_moves.contains(&mv), "premise: no movement move is listed here: {mv:?}");
                    assert!(
                        !is_listed_move(state, &mv, rules),
                        "a family member must be refused at {:?} (queued: {queued}): {mv:?}",
                        state.turn_phase,
                    );
                    if queued {
                        queued_family_refusals += 1;
                    } else {
                        out_of_phase_family_refusals += 1;
                    }
                }
            }
        });
        println!(
            "is_listed_move sweep: listed {listed}, candidates {candidates_seen}, conjunct cases {conjunct_cases}, \
             queued family refusals {queued_family_refusals}, out-of-phase family refusals {out_of_phase_family_refusals}"
        );
        assert!(listed > 1_000, "vacuous sweep: only {listed} listed moves");
        assert!(candidates_seen > 1_000, "vacuous sweep: only {candidates_seen} candidates");
        assert!(
            conjunct_cases > 0,
            "no Canonical-accepted non-member was judged — the family conjunct is untested",
        );
        assert!(
            queued_family_refusals > 0,
            "no family member was judged under a queued choice — the queue gate is untested",
        );
        assert!(
            out_of_phase_family_refusals > 0,
            "no family member was judged outside Movement — the phase gate is untested",
        );
    }

    /// Non-canonical WITNESSES for labels the list prints: for every walked
    /// `MovePresident`, the same route with a detour prepended through a
    /// neighbour adjacent to both the start and the first hop, and the direct
    /// hop where a longer walk's destination is adjacent to the start. Which
    /// variants are rules-legal, which share a listed move's label and which
    /// the generator happens to print is the LIST's business: every one is
    /// judged against it.
    fn witness_variants(state: &GameState, legal_moves: &[Move]) -> Vec<Move> {
        let PresidentLocation::OnBoard(current) = state.player(state.current_player).president_location
        else {
            return vec![];
        };
        let mut out = Vec::new();
        for mv in legal_moves {
            let Move::MovePresident { path, destination } = mv else { continue };
            let Some(&first) = path.first() else { continue };
            for &via in state.board_layout.adjacent_areas(current) {
                if via == first
                    || path.contains(&via)
                    || !state.board_layout.adjacent_areas(via).contains(&first)
                {
                    continue;
                }
                let mut detour = Vec::with_capacity(path.len() + 1);
                detour.push(via);
                detour.extend_from_slice(path);
                out.push(Move::MovePresident { path: detour, destination: *destination });
            }
            if path.len() >= 2 && state.board_layout.adjacent_areas(current).contains(destination) {
                out.push(Move::MovePresident { path: vec![*destination], destination: *destination });
            }
        }
        out
    }

    /// The phase dimension: a move the list held one state ago is judged at
    /// the NEXT state exactly as the next state's list judges it. The Movement
    /// family's list is phase-blind by contract (`route_enum.rs`), so this is
    /// what proves the conjunction keeps `is_move_legal`'s PHASE gate (the
    /// queue gate's half is the membership cell's `queued_family_refusals` —
    /// a movement move's successor never queues a choice, so this cell cannot
    /// meet it); the counter proves a movement move was judged outside its
    /// phase at least once.
    #[test]
    fn is_listed_move_reads_the_phase_the_list_reads() {
        let mut checked = 0usize;
        let mut movement_refused_later = 0usize;
        drive(2, 300, |state, rules, legal_moves| {
            let Some(first) = legal_moves.first() else { return };
            let next = apply_move_unchecked(state, first, rules).expect("a listed move applies");
            let next_list = get_legal_moves(&next, rules);
            for mv in legal_moves {
                let in_next = next_list.contains(mv);
                assert_eq!(
                    is_listed_move(&next, mv, rules),
                    in_next,
                    "disagrees one state later at {:?}: {mv:?}",
                    next.turn_phase,
                );
                checked += 1;
                if !in_next && matches!(mv, Move::MovePresident { .. } | Move::PlacePresident { .. }) {
                    movement_refused_later += 1;
                }
            }
        });
        println!("phase sweep: checked {checked}, movement moves refused one state later {movement_refused_later}");
        assert!(checked > 1_000, "vacuous sweep: only {checked} checks");
        assert!(
            movement_refused_later > 0,
            "no movement move was ever judged outside its phase — the gate is untested",
        );
    }

    // -- (c) zero false rejects -----------------------------------------------

    /// Every move the frozen generator emits is accepted, in BOTH modes, at
    /// every decision point of the driven sweep. A predicate that rejects a
    /// legal move would make a search silently blind to it.
    #[test]
    fn every_legal_move_is_accepted_in_both_modes() {
        let mut checked = 0usize;
        let mut phases: BTreeMap<String, usize> = BTreeMap::new();

        drive(3, 400, |state, rules, legal_moves| {
            let key = if state.pending_choices.is_empty() {
                format!("{:?}", state.turn_phase)
            } else {
                "ChoiceQueue".to_string()
            };
            *phases.entry(key).or_default() += 1;

            for mv in legal_moves {
                assert!(
                    legal(state, rules, mv),
                    "false reject (RulesLegal) at {:?}: {mv:?}",
                    state.turn_phase,
                );
                assert!(
                    canonical(state, rules, mv),
                    "false reject (Canonical) at {:?}: {mv:?}",
                    state.turn_phase,
                );
                checked += 1;
            }
        });

        assert!(checked > 10_000, "vacuous sweep: only {checked} legal moves checked");
        for want in [
            "AdditionalActionFirst",
            "AdditionalActionSecond",
            "Placement",
            "Movement",
            "AreaAction",
            "FivePowerBonus",
            "Building",
            "ChoiceQueue",
        ] {
            assert!(phases.contains_key(want), "no {want} decision point in the sweep: {phases:?}");
        }
    }

    // -- (d) constructed Tier-C illegals, one per dimension --------------------

    /// Tier C, Building power: the raw arm never checks it (5,520 measured
    /// false accepts), so the predicate must.
    #[test]
    fn building_below_power_four_is_rejected() {
        let mut s = diamond(2, &[]);
        s.turn_phase = TurnPhase::Building;
        s.board_layout.building_sites.insert(AreaId(0), BuildingSite::new([1, 1, 1, 1], 3));
        s.players[0].shophouses.in_hand = 1;
        s.players[0].trading_houses.in_hand = 1;
        let rules = Rules::empty_for_tests();

        s.current_action_power = Some(3);
        assert!(!legal(&s, &rules, &Move::BuildShophouse { slot: 0 }), "power 3 must not build");
        assert!(!legal(&s, &rules, &Move::BuildTradingHouse), "power 3 must not build");
        assert!(legal(&s, &rules, &Move::SkipBuilding), "skipping is always legal");
        assert!(
            apply_move_unchecked(&s, &Move::BuildShophouse { slot: 0 }, &rules).is_ok(),
            "the fixture must be one the RAW arm accepts, or it proves nothing about Tier C",
        );

        s.current_action_power = Some(4);
        assert!(legal(&s, &rules, &Move::BuildShophouse { slot: 0 }), "power 4 builds");
        assert!(legal(&s, &rules, &Move::BuildTradingHouse), "power 4 builds");
        assert!(!legal(&s, &rules, &Move::BuildShophouse { slot: 9 }), "slot 9 does not exist");
        assert_eq!(
            get_legal_moves(&s, &rules).len(),
            6,
            "meaning-pin: skip + one move per EMPTY shophouse slot (4) + trading house",
        );
    }

    /// Tier C, movement reachability: every structural defect a proposer can
    /// produce, on one fixture.
    #[test]
    fn defective_president_paths_are_rejected() {
        let s = diamond(2, &[]);
        let rules = Rules::empty_for_tests();
        let path = |hops: &[u8]| hops.iter().map(|&a| AreaId(a)).collect::<Vec<_>>();

        // The control: a legal two-hop walk 0 -> 1 -> 3.
        let ok = Move::MovePresident { path: path(&[1, 3]), destination: AreaId(3) };
        assert!(legal(&s, &rules, &ok), "control must be legal");

        for (why, mv) in [
            (
                "non-adjacent first hop",
                Move::MovePresident { path: path(&[3]), destination: AreaId(3) },
            ),
            (
                "repeated area",
                Move::MovePresident { path: path(&[1, 3, 1]), destination: AreaId(1) },
            ),
            (
                "path does not end at the declared destination",
                Move::MovePresident { path: path(&[1]), destination: AreaId(3) },
            ),
            (
                "bare teleport without Station or Tram",
                Move::MovePresident { path: vec![], destination: AreaId(3) },
            ),
        ] {
            assert!(!legal(&s, &rules, &mv), "{why} must be rejected: {mv:?}");
        }

        // The assistant chain: the same control walk, once area 1 no longer
        // holds one of the mover's assistants.
        let mut no_assistant = s.clone();
        no_assistant.players[0].assistants.on_board.remove(&AreaId(1));
        assert!(!legal(&no_assistant, &rules, &ok), "a hop needs the mover's own assistant");

        // The destination must be a Possible Action Area: an opponent President
        // standing on it blocks the walk without Postal System, even though
        // every hop is otherwise fine.
        let mut blocked = diamond(3, &[]);
        blocked.players[1].president_location = PresidentLocation::OnBoard(AreaId(3));
        assert!(!legal(&blocked, &rules, &ok), "an opponent President blocks the destination");
        let mut postal = rules.clone();
        grant_tech(&mut blocked, &mut postal, TechEffect::PostalSystem);
        assert!(legal(&blocked, &postal, &ok), "Postal System unblocks it (and 1 Yen is affordable)");

        // Re-entering the start area: legal above, illegal once area 3 IS the
        // area this turn's Movement step began in.
        let mut started_at_3 = s.clone();
        started_at_3.president_start_area = Some(AreaId(3));
        assert!(!legal(&started_at_3, &rules, &ok), "may not return to the starting area");

        // An unaffordable route: two opponent Presidents on the way, no money.
        let mut broke = diamond(3, &[]);
        broke.players[0].money = 0;
        broke.players[1].president_location = PresidentLocation::OnBoard(AreaId(1));
        assert!(
            !legal(&broke, &rules, &ok),
            "a route the mover cannot pay for is not legal",
        );
        broke.players[0].money = 1;
        assert!(legal(&broke, &rules, &ok), "the same route IS legal once affordable");
    }

    /// Tier C, `PlacePresident` possible-area: the largest measured false-accept
    /// dimension (84,579) — the raw arm checks placeability, never whether the
    /// mover has an assistant there or an opponent President blocks it.
    #[test]
    fn place_president_onto_a_non_possible_area_is_rejected() {
        let mut s = diamond(2, &[]);
        s.players[0].president_location = PresidentLocation::InHand;
        let rules = Rules::empty_for_tests();

        assert!(
            legal(&s, &rules, &Move::PlacePresident { destination: AreaId(1) }),
            "an own-assistant area is a Possible Action Area",
        );
        assert!(
            !legal(&s, &rules, &Move::PlacePresident { destination: AreaId(0) }),
            "no own assistant on area 0",
        );
        assert!(
            apply_move_unchecked(&s, &Move::PlacePresident { destination: AreaId(0) }, &rules)
                .is_ok(),
            "the raw arm accepts it — this dimension is Tier C, not structural",
        );

        s.players[1].president_location = PresidentLocation::OnBoard(AreaId(1));
        assert!(
            !legal(&s, &rules, &Move::PlacePresident { destination: AreaId(1) }),
            "an opponent President blocks the area without Postal System",
        );

        let mut postal_rules = rules.clone();
        grant_tech(&mut s, &mut postal_rules, TechEffect::PostalSystem);
        assert!(
            legal(&s, &postal_rules, &Move::PlacePresident { destination: AreaId(1) }),
            "Postal System unblocks it — and the 1 Yen fee is affordable",
        );
        s.players[0].money = 0;
        assert!(
            !legal(&s, &postal_rules, &Move::PlacePresident { destination: AreaId(1) }),
            "with no money the destination stop fee is unaffordable",
        );
    }

    /// Tier C, PHASE: a well-formed move of the wrong phase (a 100% false-accept
    /// rate for the raw arms — none of them reads `turn_phase`).
    #[test]
    fn a_right_move_in_the_wrong_phase_is_rejected() {
        let mut s = diamond(2, &[]);
        let rules = Rules::empty_for_tests();
        let placement = Move::PlaceThreeAreas { placements: vec![AreaId(1)] };
        let movement = Move::MovePresident { path: vec![AreaId(1)], destination: AreaId(1) };

        assert!(legal(&s, &rules, &movement), "Movement phase: the movement move is legal");
        assert!(!legal(&s, &rules, &placement), "Movement phase: a placement move is not");
        assert!(
            apply_move_unchecked(&s, &placement, &rules).is_ok(),
            "the raw arm applies it regardless of phase — that is the dimension",
        );

        s.turn_phase = TurnPhase::Placement;
        assert!(legal(&s, &rules, &placement), "Placement phase: the placement move is legal");
        assert!(!legal(&s, &rules, &movement), "Placement phase: a movement move is not");
        assert!(!legal(&s, &rules, &Move::EndAdditionalPhase), "nor an additional-action move");

        s.turn_phase = TurnPhase::Recovery;
        for mv in [&placement, &movement, &Move::SkipBuilding, &Move::Skip5PowerBonus] {
            assert!(!legal(&s, &rules, mv), "Recovery is automatic: nothing is legal ({mv:?})");
        }
    }

    /// The generator emits STRICTLY ASCENDING placement tuples
    /// (`legal_moves.rs:159-195`, matching `search_actions::place_combo`), so a
    /// permuted tuple is rules-legal but is not the move the list carries —
    /// accepting it would double-count the combination.
    #[test]
    fn permuted_and_duplicated_placements_are_rejected() {
        let mut s = diamond(2, &[]);
        s.turn_phase = TurnPhase::Placement;
        let rules = Rules::empty_for_tests();

        let ascending = Move::PlaceThreeAreas { placements: vec![AreaId(1), AreaId(2)] };
        let permuted = Move::PlaceThreeAreas { placements: vec![AreaId(2), AreaId(1)] };
        let duplicated = Move::PlaceThreeAreas { placements: vec![AreaId(1), AreaId(1)] };

        assert!(legal(&s, &rules, &ascending), "the ascending tuple is what the list carries");
        assert!(get_legal_moves(&s, &rules).contains(&ascending), "meaning-pin");
        assert!(!legal(&s, &rules, &permuted), "a permuted tuple is not the canonical move");
        assert!(!legal(&s, &rules, &duplicated), "Option A places in DISTINCT areas");
        assert!(
            apply_move_unchecked(&s, &permuted, &rules).is_ok(),
            "the raw arm applies the permutation — the ordering is canonicalization",
        );

        // Over-cap: 4 areas needs Newspaper (and a 4th assistant).
        s.players[0].assistants.in_hand = 4;
        s.players[0].assistants.on_board.insert(AreaId(0), 1);
        let four = Move::PlaceThreeAreas {
            placements: vec![AreaId(0), AreaId(1), AreaId(2), AreaId(3)],
        };
        assert!(!legal(&s, &rules, &four), "3 areas is the cap without Newspaper");
        let mut news = rules.clone();
        grant_tech(&mut s, &mut news, TechEffect::Newspaper);
        assert!(legal(&s, &news, &four), "Newspaper raises the cap to 4");
    }

    /// Placement is priced PER COMBINATION (1 Yen per assistant placed to each
    /// opponent President owner in the target area), so affordability is not a
    /// property of any single area.
    #[test]
    fn an_unaffordable_placement_combination_is_rejected() {
        let mut s = diamond(3, &[]);
        s.turn_phase = TurnPhase::Placement;
        s.players[0].money = 1;
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId(1));
        s.players[2].president_location = PresidentLocation::OnBoard(AreaId(2));
        let rules = Rules::empty_for_tests();

        let one = Move::PlaceThreeAreas { placements: vec![AreaId(1)] };
        let both = Move::PlaceThreeAreas { placements: vec![AreaId(1), AreaId(2)] };
        assert!(legal(&s, &rules, &one), "1 Yen covers one occupied area");
        assert!(!legal(&s, &rules, &both), "2 Yen is needed for both, and the mover has 1");
        assert!(!get_legal_moves(&s, &rules).contains(&both), "meaning-pin");

        // Option B pays PER ASSISTANT PLACED: two into one occupied area costs 2.
        let two_same = Move::PlaceTwoSameArea { area: AreaId(1), adjacent_bonus: None };
        assert!(!legal(&s, &rules, &two_same), "2 assistants into an occupied area cost 2 Yen");
        s.players[0].money = 2;
        assert!(legal(&s, &rules, &two_same), "affordable at 2 Yen");
    }

    /// Telegram's adjacent bonus: owned tech, a third assistant, real adjacency,
    /// and never across a canal crossing (transit-only edges).
    #[test]
    fn telegram_bonus_placements_follow_the_neighbour_rules() {
        let mut s = diamond(2, &[(0, 2)]);
        s.turn_phase = TurnPhase::Placement;
        let mut rules = Rules::empty_for_tests();
        let bonus = |adj: u8| Move::PlaceTwoSameArea {
            area: AreaId(0),
            adjacent_bonus: Some(AreaId(adj)),
        };

        assert!(!legal(&s, &rules, &bonus(1)), "no Telegram, no bonus");
        grant_tech(&mut s, &mut rules, TechEffect::Telegram);
        assert!(legal(&s, &rules, &bonus(1)), "Telegram + 3 assistants + adjacency");
        assert!(get_legal_moves(&s, &rules).contains(&bonus(1)), "meaning-pin");
        assert!(!legal(&s, &rules, &bonus(3)), "area 3 is not adjacent to area 0");
        assert!(!legal(&s, &rules, &bonus(2)), "the 0-2 edge is a canal crossing");

        s.players[0].assistants.in_hand = 2;
        assert!(!legal(&s, &rules, &bonus(1)), "the bonus needs a THIRD assistant");
    }

    /// Retreat legality is the per-area count bound `1..=have` — the complete
    /// raw arm (`apply_move.rs:634-639`), no enumeration of the product.
    #[test]
    fn out_of_range_retreat_counts_are_rejected() {
        let mut s = diamond(2, &[]);
        s.players[0].assistants.on_board.insert(AreaId(1), 2);
        let rules = Rules::empty_for_tests();
        let retreat = |entries: &[(u8, u8)]| Move::ReturnPresidentToHand {
            recover: entries.iter().map(|&(a, n)| (AreaId(a), n)).collect(),
        };

        assert!(legal(&s, &rules, &retreat(&[])), "recovering nothing is legal");
        assert!(legal(&s, &rules, &retreat(&[(1, 2)])), "recovering all of them is legal");
        assert!(legal(&s, &rules, &retreat(&[(1, 1)])), "recovering some of them is legal");
        assert!(!legal(&s, &rules, &retreat(&[(1, 3)])), "3 > the 2 assistants in area 1");
        assert!(!legal(&s, &rules, &retreat(&[(1, 0)])), "a zero count is not an entry");
        assert!(!legal(&s, &rules, &retreat(&[(0, 1)])), "no assistants in area 0");
    }

    /// At a queue head only THAT choice's resolutions are legal — including the
    /// declines of other choices, which are well-formed moves of the wrong
    /// choice.
    #[test]
    fn at_a_queue_head_only_that_choices_resolutions_are_legal() {
        let mut s = diamond(2, &[]);
        s.pending_choices.push(PendingChoice::AnyGood);
        let rules = Rules::empty_for_tests();

        assert!(
            legal(&s, &rules, &Move::ChooseAnyGood { good: TradeGood::Copper }),
            "the queue head's own resolution",
        );
        for wrong in [
            Move::DeclineWarehouseTake,
            Move::DeclineAssistantMove,
            Move::ChooseRecoveryLeave { leave_one: true },
            Move::ChooseChurchSpace { space: 4 },
            // ...and the phase move that WOULD be legal with an empty queue.
            Move::MovePresident { path: vec![AreaId(1)], destination: AreaId(1) },
            Move::ReturnPresidentToHand { recover: BTreeMap::new() },
        ] {
            assert!(!legal(&s, &rules, &wrong), "not a resolution of AnyGood: {wrong:?}");
        }
    }

    // -- (e) the two delegation purity pins ------------------------------------

    /// What the `ExecuteAreaAction` delegation MEANS: at an `AreaAction` state
    /// with an empty queue, `get_legal_moves` is `generate_area_action_moves`
    /// and nothing else, so membership in it is exactly the area-action family.
    /// The same shape for the queue: with a `PendingChoice` at the head, the
    /// list is `generate_choice_resolutions` and nothing else.
    #[test]
    fn the_delegated_families_are_phase_pure() {
        let mut area_action_states = 0usize;
        let mut queue_states = 0usize;

        drive(3, 400, |state, _rules, legal_moves| {
            if !state.pending_choices.is_empty() {
                queue_states += 1;
                assert!(
                    legal_moves.iter().all(|mv| {
                        matches!(
                            mv,
                            Move::ChooseAnyGood { .. }
                                | Move::ChooseWarehouseTake { .. }
                                | Move::DeclineWarehouseTake
                                | Move::ChooseAssistantMove { .. }
                                | Move::DeclineAssistantMove
                                | Move::ChooseChurchSpace { .. }
                                | Move::ChooseCustomsSpace { .. }
                                | Move::ChooseRecoveryLeave { .. }
                                | Move::ChooseStationPlace { .. }
                        )
                    }),
                    "a queued state offered a non-resolution: {legal_moves:?}",
                );
            } else if state.turn_phase == TurnPhase::AreaAction {
                area_action_states += 1;
                assert!(
                    legal_moves.iter().all(|mv| matches!(mv, Move::ExecuteAreaAction { .. })),
                    "an AreaAction state offered something else: {legal_moves:?}",
                );
            }
        });

        assert!(area_action_states > 100, "only {area_action_states} AreaAction states");
        assert!(queue_states > 10, "only {queue_states} queued states");
    }

    // -- (f) mercenary unreachability ------------------------------------------

    /// The Movement predicate is non-mercenary BY CONSTRUCTION: a mercenary
    /// sub-turn runs AreaAction → FivePower → Building → Recovery and never
    /// enters `TurnPhase::Movement`, so no `is_mercenary` flag has to be
    /// threaded through `is_president_path_legal` (`route_fee_vector` and
    /// `is_possible_action_area_full` both take one, and this module always
    /// passes the non-mercenary reading). Pinned by a test rather than by
    /// silence.
    ///
    /// CONSTRUCTED rather than driven, deliberately: mercenaries arrive only on
    /// order cards and no uniform-random game in this module's sweeps ever used
    /// one, so a driven version of this pin would assert nothing. That fact is
    /// WIDTH-DEPENDENT, not seed-dependent (its falsifier, measured at review:
    /// `drive(3, 400)` = 0 mercenary uses in 3,600 decision points, but the
    /// same seeds at a 2,000-step cap fire 6 times) — if this module's sweep
    /// width ever grows, re-check the story rather than inheriting it. Here the
    /// sub-turn is entered from the move that starts it and driven to its end,
    /// at eight seeds so different branches through it are covered.
    #[test]
    fn the_movement_phase_never_occurs_inside_a_mercenary_sub_turn() {
        use rand::{Rng, SeedableRng};
        let rules = Rules::empty_for_tests();

        let mut s = diamond(2, &[]);
        s.turn_phase = TurnPhase::AdditionalActionFirst;
        s.board_layout.building_sites.insert(AreaId(1), BuildingSite::new([1, 1, 1, 1], 3));
        s.board_layout.five_power_tokens.insert(AreaId(1), Some(FivePowerEffect::Copper(2)));
        s.players[0].mercenaries.available.insert(Country::English, 1);
        s.players[0].shophouses.in_hand = 1;
        s.players[0].trading_houses.in_hand = 1;
        s.players[0].assistants.on_board.insert(AreaId(1), 5);

        let start = Move::UseMercenary { country: Country::English, target_area: AreaId(1) };
        assert!(legal(&s, &rules, &start), "the fixture must actually offer the mercenary");

        for seed in 0..8u64 {
            let mut state =
                apply_move_unchecked(&s, &start, &rules).expect("the sub-turn starts");
            assert!(state.is_mercenary_action, "seed {seed}: the sub-turn is running");

            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
            let mut phases = Vec::new();
            for step in 0.. {
                assert!(step < 50, "seed {seed}: the sub-turn never ended: {phases:?}");
                if !state.is_mercenary_action {
                    break;
                }
                assert_ne!(
                    state.turn_phase,
                    TurnPhase::Movement,
                    "seed {seed}: a mercenary sub-turn reached the Movement phase",
                );
                phases.push(state.turn_phase);
                let legal_moves = get_legal_moves(&state, &rules);
                assert!(!legal_moves.is_empty(), "seed {seed}: dead end at {:?}", state.turn_phase);
                let mv = legal_moves[rng.gen_range(0..legal_moves.len())].clone();
                state = apply_move_unchecked(&state, &mv, &rules).expect("apply");
            }

            assert!(
                phases.contains(&TurnPhase::AreaAction)
                    && phases.contains(&TurnPhase::FivePowerBonus)
                    && phases.contains(&TurnPhase::Building),
                "seed {seed}: the sub-turn must run its documented chain: {phases:?}",
            );
        }
    }

    // -- (g) Canonical mode ----------------------------------------------------

    /// Suppression FIRES: a free walk to the station area already carries that
    /// destination's `{}` label, so the generator does not print the teleport.
    /// `Canonical` must reject it; `RulesLegal` must accept it (it is a legal
    /// encoding of a legal move — the Station card's text is unconditional).
    #[test]
    fn canonical_mode_rejects_a_teleport_a_free_walk_suppresses() {
        let (s, rules) = station_diamond(&[], 1);
        let teleport = Move::MovePresident { path: vec![], destination: AreaId(1) };

        assert!(
            !get_legal_moves(&s, &rules).contains(&teleport),
            "meaning-pin: the generator suppresses this teleport",
        );
        assert!(legal(&s, &rules, &teleport), "RulesLegal accepts the legal encoding");
        assert!(!canonical(&s, &rules, &teleport), "Canonical reproduces the suppression");
        assert!(
            canonical(
                &s,
                &rules,
                &Move::MovePresident { path: vec![AreaId(1)], destination: AreaId(1) },
            ),
            "the free WALK is what the list carries, and it stays canonical",
        );
    }

    /// Suppression does NOT fire: both edges into the station area toll, so
    /// every walk there is `{None:1}` and the teleport's `{}` is a DIFFERENT
    /// label. Both survive — the I-1 regression class, from the predicate side.
    #[test]
    fn canonical_mode_accepts_a_teleport_when_every_walk_is_tolled() {
        let (s, rules) = station_diamond(&[(0, 1), (1, 3)], 1);
        let teleport = Move::MovePresident { path: vec![], destination: AreaId(1) };
        let tolled_walk = Move::MovePresident { path: vec![AreaId(1)], destination: AreaId(1) };

        assert!(get_legal_moves(&s, &rules).contains(&teleport), "meaning-pin: both are emitted");
        assert!(get_legal_moves(&s, &rules).contains(&tolled_walk), "meaning-pin");
        assert!(canonical(&s, &rules, &teleport), "no zero-cost walk covers the destination");
        assert!(canonical(&s, &rules, &tolled_walk), "and the tolled walk stays legal");
    }

    /// The Tram arm of the teleport encoding: any placeable area with no
    /// opponent President, never the current area, and suppressed exactly like
    /// Station's when a free walk already covers it.
    #[test]
    fn tram_teleports_follow_the_destination_rules() {
        let mut rules = Rules::empty_for_tests();
        let mut s = diamond(2, &[(0, 1), (1, 3), (0, 2), (2, 3)]);
        grant_tech(&mut s, &mut rules, TechEffect::Tram);
        // Every edge tolls, so no zero-cost walk suppresses anything.
        let tram = |dest: u8| Move::MovePresident { path: vec![], destination: AreaId(dest) };

        assert!(canonical(&s, &rules, &tram(3)), "Tram reaches a tolled-walk destination");
        assert!(!legal(&s, &rules, &tram(0)), "never onto the current area");

        s.board_layout.areas.get_mut(&AreaId(3)).expect("area 3").area_type = AreaType::Canal;
        assert!(!legal(&s, &rules, &tram(3)), "never onto the Canal");
        s.board_layout.areas.get_mut(&AreaId(3)).expect("area 3").area_type = AreaType::Bank;

        s.players[1].president_location = PresidentLocation::OnBoard(AreaId(3));
        assert!(!legal(&s, &rules, &tram(3)), "never onto an opponent President's area");
    }

    /// THE ALIGNED WORLD, PINNED EXECUTABLE (2026-07-25, `SEMANTICS_REV` 4).
    /// This test used to pin the OPPOSITE: on a fixture-only state — own
    /// assistant on a Canal-TYPED area — the generator EMITTED
    /// `PlacePresident{canal}` while every apply path rejected it, and this
    /// module narrowed to side with apply. Its own doc said "if this leg fails,
    /// legal_moves.rs was fixed — retire the narrowing comment". It was fixed,
    /// so it is retired: the Canal exclusion now lives in the two SHARED
    /// movement predicates (`is_possible_action_area_full` and `can_step_onto`),
    /// which is why `legal_moves.rs` needed no edit at all — every arm reaches
    /// the rule through them.
    ///
    /// Four legs, over the from-hand arm and the walk arm (the old test only
    /// ever exercised the former, though the same mechanism reached hops and
    /// destinations too):
    /// (1) the generator does NOT emit `PlacePresident{canal}`;
    /// (2) the predicate still rejects a hand-built candidate in BOTH modes;
    /// (3) apply still rejects it — all three surfaces agree, which is the
    ///     property the narrowing used to buy at the cost of a divergence;
    /// (4) no emitted `MovePresident` ends on, or steps through, the Canal, and
    ///     hand-built walks that do are rejected by predicate and apply alike.
    #[test]
    fn the_canal_is_excluded_by_construction_across_generator_predicate_and_apply() {
        let rules = Rules::empty_for_tests();

        // -- from-hand arm ----------------------------------------------------
        let mut s = diamond(2, &[]);
        s.players[0].president_location = PresidentLocation::InHand;
        s.board_layout.areas.get_mut(&AreaId(1)).expect("area 1").area_type = AreaType::Canal;

        let mv = Move::PlacePresident { destination: AreaId(1) };

        assert!(
            !get_legal_moves(&s, &rules).contains(&mv),
            "meaning-pin: the generator no longer emits PlacePresident onto a Canal-typed area — \
             the exclusion lives in is_possible_action_area_full",
        );
        assert!(
            get_legal_moves(&s, &rules).contains(&Move::PlacePresident { destination: AreaId(2) }),
            "anti-vacuity: the non-Canal areas are still offered",
        );
        assert!(!legal(&s, &rules, &mv), "the predicate rejects it (RulesLegal)");
        assert!(!canonical(&s, &rules, &mv), "the predicate rejects it (Canonical)");
        assert!(apply_move_unchecked(&s, &mv, &rules).is_err(), "and apply rejects it");

        // -- walk arm: hops AND destinations ----------------------------------
        // The diamond is 0-1-3 / 0-2-3 with the President on 0 and own
        // assistants on 1, 2 and 3. With 1 typed Canal, area 3 stays reachable
        // via 2 — so the fixture proves exclusion, not mere emptiness.
        let mut w = diamond(2, &[]);
        w.board_layout.areas.get_mut(&AreaId(1)).expect("area 1").area_type = AreaType::Canal;

        let emitted = get_legal_moves(&w, &rules);
        for m in &emitted {
            if let Move::MovePresident { path, destination } = m {
                assert_ne!(*destination, AreaId(1), "no walk may END on the Canal: {m:?}");
                assert!(!path.contains(&AreaId(1)), "no walk may STEP THROUGH the Canal: {m:?}");
            }
        }
        assert!(
            emitted.contains(&Move::MovePresident {
                path: vec![AreaId(2), AreaId(3)],
                destination: AreaId(3),
            }),
            "anti-vacuity: the route that detours around the Canal is still emitted",
        );

        let onto_canal = Move::MovePresident { path: vec![AreaId(1)], destination: AreaId(1) };
        let through_canal =
            Move::MovePresident { path: vec![AreaId(1), AreaId(3)], destination: AreaId(3) };
        for m in [&onto_canal, &through_canal] {
            assert!(!legal(&w, &rules, m), "the predicate rejects it (RulesLegal): {m:?}");
            assert!(!canonical(&w, &rules, m), "the predicate rejects it (Canonical): {m:?}");
            assert!(apply_move_unchecked(&w, m, &rules).is_err(), "and apply rejects it: {m:?}");
        }
    }

    /// The two NAMED Canal residuals from the rev-4 landing, ABSORBED
    /// (owner ruling, Jeremy 2026-07-25; `SEMANTICS_REV` 4 -> 5). Both were
    /// generator/apply-CONSISTENT permissive holes reachable only on
    /// constructed fixtures — the Station teleport, whose entitlement function
    /// never inspected `area_type`, and mercenary TARGET SELECTION, which
    /// iterated `assistants.on_board` bespoke with no area-type filter and had
    /// no `require_placeable_area` on the apply side either.
    ///
    /// This pins the ABSORBED world across all three surfaces the module cares
    /// about — generator, predicate, apply — with anti-vacuity controls on the
    /// non-Canal twin of each leg so a fixture that stopped offering the move
    /// for some unrelated reason cannot pass.
    #[test]
    fn the_two_canal_residuals_are_absorbed_station_teleport_and_mercenary_target() {
        // -- Station teleport --------------------------------------------------
        // The token sits on area 1, with an own assistant on it: the whole
        // entitlement (tech owned, token placed, own assistant present). The
        // tolled crossings are what keep the FREE teleport a distinct label —
        // the generator suppresses it when a zero-cost walk already reaches the
        // station area.
        let (mut s, rules) = station_diamond(&[(0, 1), (1, 3)], 1);
        let teleport = Move::MovePresident { path: vec![], destination: AreaId(1) };
        assert!(
            get_legal_moves(&s, &rules).contains(&teleport),
            "anti-vacuity: the free teleport onto a normal station area is emitted",
        );
        assert!(legal(&s, &rules, &teleport), "anti-vacuity: and the predicate accepts it");

        s.board_layout.areas.get_mut(&AreaId(1)).expect("area 1").area_type = AreaType::Canal;
        assert!(
            !get_legal_moves(&s, &rules)
                .iter()
                .any(|m| matches!(m, Move::MovePresident { destination, .. } if *destination == AreaId(1))),
            "meaning-pin: no move of any kind may land the President on a Canal-typed \
             station area — the gate lives in can_teleport_to_station_with_tech",
        );
        assert!(!legal(&s, &rules, &teleport), "the predicate rejects it (RulesLegal)");
        assert!(!canonical(&s, &rules, &teleport), "the predicate rejects it (Canonical)");
        assert!(
            get_legal_moves(&s, &rules).iter().any(
                |m| matches!(m, Move::MovePresident { destination, .. } if *destination == AreaId(2))
            ),
            "anti-vacuity: the rest of the Movement phase is untouched",
        );

        // -- mercenary target selection ---------------------------------------
        let rules = Rules::empty_for_tests();
        let mut m = diamond(2, &[]);
        m.turn_phase = TurnPhase::AdditionalActionFirst;
        m.players[0].mercenaries.available.insert(Country::English, 1);

        let onto_1 = Move::UseMercenary { country: Country::English, target_area: AreaId(1) };
        let onto_2 = Move::UseMercenary { country: Country::English, target_area: AreaId(2) };
        assert!(
            get_legal_moves(&m, &rules).contains(&onto_1),
            "anti-vacuity: a normal target area is offered",
        );
        assert!(legal(&m, &rules, &onto_1), "anti-vacuity: and the predicate accepts it");
        assert!(apply_move_unchecked(&m, &onto_1, &rules).is_ok(), "anti-vacuity: and apply runs");

        m.board_layout.areas.get_mut(&AreaId(1)).expect("area 1").area_type = AreaType::Canal;
        assert!(
            !get_legal_moves(&m, &rules).contains(&onto_1),
            "meaning-pin: the generator no longer targets a mercenary at a Canal-typed area — \
             a mercenary is an ACTION at an area a pawn stands on, and no pawn stands on the Canal",
        );
        assert!(
            get_legal_moves(&m, &rules).contains(&onto_2),
            "anti-vacuity: the other target areas are still offered",
        );
        assert!(!legal(&m, &rules, &onto_1), "the predicate rejects it (RulesLegal)");
        assert!(!canonical(&m, &rules, &onto_1), "the predicate rejects it (Canonical)");
        assert_eq!(
            apply_move_unchecked(&m, &onto_1, &rules),
            Err(MoveError::RuleViolation("The Canal is never a placeable area")),
            "and the raw arm self-defends with the Canal rule by name",
        );

        // A ghost target id gets the MEMBERSHIP message, not the Canal one, and
        // not the assistant-count message — the placeability gate sits ahead of
        // the count check (the `choices.rs` two-message precedent). The fixture
        // SEEDS an assistant on the ghost area (an invariant-violating state,
        // deliberately: without it, generator and predicate would refuse for
        // the assistant-count reason and these assertions would pass vacuously
        // — PR #11 review) so the membership branch is the deciding one at ALL
        // THREE surfaces, making "refused at generator, predicate and apply"
        // test-pinned rather than code-read-true.
        m.players[0].assistants.on_board.insert(AreaId(99), 1);
        let ghost = Move::UseMercenary { country: Country::English, target_area: AreaId(99) };
        assert!(
            !get_legal_moves(&m, &rules).iter().any(|mv| matches!(
                mv,
                Move::UseMercenary { target_area, .. } if *target_area == AreaId(99)
            )),
            "the generator omits the off-board target even with an assistant seeded there",
        );
        assert!(!legal(&m, &rules, &ghost), "the predicate rejects the off-board target");
        assert_eq!(
            apply_move_unchecked(&m, &ghost, &rules),
            Err(MoveError::RuleViolation("Area does not exist on this board")),
        );
    }

    /// Station's own preconditions, from the predicate side: the token must be
    /// placed, the tech owned, and an own assistant must stand on the station
    /// area (owner ruling A1, 2026-07-20).
    #[test]
    fn station_teleports_require_the_whole_entitlement() {
        let (mut s, rules) = station_diamond(&[(0, 1), (1, 3)], 1);
        let teleport = Move::MovePresident { path: vec![], destination: AreaId(1) };
        assert!(legal(&s, &rules, &teleport), "control: the full entitlement");

        let no_tech = Rules::empty_for_tests();
        assert!(!legal(&s, &no_tech, &teleport), "without the Station tech there is no teleport");

        s.players[0].assistants.on_board.remove(&AreaId(1));
        assert!(!legal(&s, &rules, &teleport), "no own assistant on the station area");
    }

    /// `is_president_path_legal` is the movement predicate on its own terms:
    /// from hand there is no walk to judge (that is `PlacePresident`).
    #[test]
    fn a_president_in_hand_has_no_legal_path() {
        let mut s = diamond(2, &[]);
        s.players[0].president_location = PresidentLocation::InHand;
        let rules = Rules::empty_for_tests();

        assert!(
            !is_president_path_legal(
                &s,
                PlayerId::new(0),
                &[AreaId(1)],
                AreaId(1),
                &rules,
                PathMode::RulesLegal,
            ),
            "from hand, PlacePresident is the encoding",
        );
        assert!(legal(&s, &rules, &Move::PlacePresident { destination: AreaId(1) }));
    }
}
