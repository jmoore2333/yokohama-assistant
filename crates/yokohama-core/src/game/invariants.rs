//! State invariant checks for the Task 18 self-play fuzz gate.
//!
//! `check_invariants` is a pure, read-only auditor: it never mutates state
//! and never depends on how the state was reached. The self-play driver
//! (`examples/selfplay.rs` / `tests/common`) calls it after every applied
//! move; any `Err` is a corrupted-state bug report (conservation broken,
//! a structural double-claim, or an out-of-range field), independent of
//! whether the move sequence that produced it was itself "legal" play.
//!
//! Card-count conservation (orders = 36, techs = 30) is gated on the
//! `Rules` actually containing that many cards, so `Rules::empty_for_tests()`
//! fixtures (empty registries) are exempt rather than spuriously failing.

use std::collections::HashSet;

use crate::game::rules::Rules;
use crate::types::GameState;

/// Check every engine-maintained conservation and structural invariant
/// against a `GameState`. Returns `Err` naming the first violation found.
pub fn check_invariants(state: &GameState, rules: &Rules) -> Result<(), String> {
    for p in &state.players {
        let total = p.assistants.total();
        if total != 20 {
            return Err(format!("P{} assistants total {total} != 20", p.id.0));
        }
        if p.order_cards_in_hand.len() > 3 {
            return Err(format!("P{} order hand size {} > 3", p.id.0, p.order_cards_in_hand.len()));
        }
        // Shophouses: 2 in hand + 6 in warehouse + 0 built at game start;
        // in_hand/in_warehouse/built are the only three zones a shophouse
        // token can occupy (Employment Agency moves warehouse->hand,
        // Building moves hand->built).
        let sh_total = p.shophouses.in_hand as usize
            + p.shophouses.in_warehouse as usize
            + p.shophouses.built.len();
        if sh_total != 8 {
            return Err(format!(
                "P{} shophouse total {sh_total} != 8 (in_hand={} in_warehouse={} built={})",
                p.id.0, p.shophouses.in_hand, p.shophouses.in_warehouse, p.shophouses.built.len()
            ));
        }
        // Trading houses: three zones since the staging re-model - the
        // Employment Agency takes warehouse -> hand (paying the printed
        // 4/5/6/7 progression) and the Building step places from hand free.
        let th_total = p.trading_houses.in_hand as usize
            + p.trading_houses.in_warehouse as usize
            + p.trading_houses.built.len();
        if th_total != 4 {
            return Err(format!(
                "P{} trading house total {th_total} != 4 (in_hand={} in_warehouse={} built={})",
                p.id.0,
                p.trading_houses.in_hand,
                p.trading_houses.in_warehouse,
                p.trading_houses.built.len()
            ));
        }
    }

    // Order conservation: deck + ports (6 slots/board) + hands + completed +
    // removed = 36, with NO +1-per-hidden term -- the Unknown-starting-order
    // fiction parks BOTH dealt cards in orders_removed, so there's nothing
    // extra to count (decision log 2026-07-17). Gated on the registry
    // actually holding 36 cards so Rules::empty_for_tests() is exempt.
    let in_ports = state.port_state.port_a.slots.iter().flatten().count()
        + state.port_state.port_b.as_ref().map_or(0, |b| b.slots.iter().flatten().count());
    let in_hands: usize = state.players.iter().map(|p| p.order_cards_in_hand.len()).sum();
    let completed: usize = state.players.iter().map(|p| p.completed_orders.len()).sum();
    let total_orders =
        state.order_deck.len() + in_ports + in_hands + completed + state.orders_removed.len();
    if rules.orders.iter().count() == 36 && total_orders != 36 {
        return Err(format!(
            "order conservation broken: deck({}) + ports({in_ports}) + hands({in_hands}) + \
             completed({completed}) + removed({}) = {total_orders} != 36",
            state.order_deck.len(),
            state.orders_removed.len()
        ));
    }

    // Tech conservation: deck + labs (6 slots/board) + owned = 30 physical
    // cards (20 titles, 10 with a duplicate copy -- rules.techs.iter()
    // counts physical cards, verified against
    // `tech_registry_models_30_card_deck` in game/cards.rs). Gated the same
    // way as orders.
    let in_labs = state.lab_state.lab_a.slots.iter().flatten().count()
        + state.lab_state.lab_b.as_ref().map_or(0, |b| b.slots.iter().flatten().count());
    let owned: usize = state.players.iter().map(|p| p.technologies.len()).sum();
    let total_techs = state.tech_deck.len() + in_labs + owned;
    if rules.techs.iter().count() == 30 && total_techs != 30 {
        return Err(format!(
            "tech conservation broken: deck({}) + labs({in_labs}) + owned({owned}) = {total_techs} != 30",
            state.tech_deck.len()
        ));
    }

    // No two players may claim the same (area, slot) shophouse pairing.
    // (≤1 shophouse per player per area is already implied by each player's
    // own `HashMap<AreaId, u8>` key uniqueness; this catches cross-player
    // double-claims of one physical slot.)
    let shophouse_slots: Vec<(crate::types::AreaId, u8)> = state
        .players
        .iter()
        .flat_map(|p| p.shophouses.built.iter().map(|(&area, &slot)| (area, slot)))
        .collect();
    let unique_slots: HashSet<_> = shophouse_slots.iter().collect();
    if unique_slots.len() != shophouse_slots.len() {
        return Err(format!("duplicate shophouse (area, slot) claim across players: {shophouse_slots:?}"));
    }

    // No two players may claim a trading house in the same area.
    let th_areas: Vec<crate::types::AreaId> =
        state.players.iter().flat_map(|p| p.trading_houses.built.iter().copied()).collect();
    let unique_areas: HashSet<_> = th_areas.iter().collect();
    if unique_areas.len() != th_areas.len() {
        return Err(format!("duplicate trading house area claim across players: {th_areas:?}"));
    }

    if let Some(power) = state.current_action_power {
        if power > 5 {
            return Err(format!("current_action_power {power} > 5"));
        }
    }

    // I1(b) final-review fix (design §5.1 queue invariant): pending_choices
    // must be empty at TurnEnd -- the phase machine's one player hand-off
    // point (nothing else ever assigns TurnPhase::TurnEnd; see
    // apply_move.rs's advance_phase). A stale queue surviving to TurnEnd
    // would misgrant its reward once resolved, since by then the game has
    // ended (or, for the mid-game AdditionalActionSecond->AdditionalActionFirst
    // hand-off, current_player has already moved on). I1(a)'s apply_move
    // guard on EndAdditionalPhase makes this unreachable via ordinary play
    // (TurnEnd is only ever reached through that guarded arm) -- this is
    // the defense-in-depth audit for the fuzz gate.
    if state.turn_phase == crate::types::TurnPhase::TurnEnd && !state.pending_choices.is_empty() {
        return Err(format!("pending_choices non-empty at TurnEnd: {:?}", state.pending_choices));
    }

    // Queue-head resolvability (Phase 2.9 Stage 5, Stage-4 census finding):
    // get_legal_moves is queue-first -- a non-empty pending_choices makes it
    // return ONLY the head's resolution set. If that set is empty the game
    // livelocks: get_legal_moves yields nothing, is_game_over stays false, and
    // no move can ever apply. Organic play cannot reach it (push_choice
    // auto-resolves singletons, normalize_queue_head clears lone-declines, and
    // Church/Customs/Station heads only ever queue with >=2 live options), but
    // a tampered or deserialized head (e.g. a CustomsSpace at an impossible
    // cost, or a StationPlace with no placeable area) can -- so replay_strict
    // rejects it here loudly instead of hanging.
    if let Some(head) = state.pending_choices.first() {
        if crate::game::choices::generate_choice_resolutions(state, head, rules).is_empty() {
            return Err(format!(
                "pending choice head {head:?} has no legal resolution (empty resolution set would livelock get_legal_moves)"
            ));
        }
    }

    // End-game trigger coherence (Stage-6 audit): check_end_game_triggers
    // always sets trigger_player + trigger_reason with the flag, and
    // final_round only ever follows a trigger. A state violating either is
    // tampered/corrupt, and previously made the free-fn and method
    // is_game_over implementations disagree (now unified).
    if state.end_game_triggered && (state.trigger_player.is_none() || state.trigger_reason.is_none())
    {
        return Err(
            "end_game_triggered without trigger_player/trigger_reason (trigger coherence)"
                .to_string(),
        );
    }
    if state.final_round && !state.end_game_triggered {
        return Err("final_round without end_game_triggered (final_round coherence)".to_string());
    }

    // Board membership (design §4.1): every stored AreaId must exist on the
    // board. A record or bot that fabricates areas must fail loudly here.
    let on_board = |a: &crate::types::AreaId| state.board_layout.areas.contains_key(a);
    for p in &state.players {
        if let crate::types::PresidentLocation::OnBoard(a) = p.president_location {
            if !on_board(&a) {
                return Err(format!("player {} president on non-board area {:?}", p.id.0, a));
            }
        }
        for a in p.assistants.on_board.keys() {
            if !on_board(a) {
                return Err(format!("player {} assistants on non-board area {:?}", p.id.0, a));
            }
        }
        for a in p.shophouses.built.keys() {
            if !on_board(a) {
                return Err(format!("player {} shophouse on non-board area {:?}", p.id.0, a));
            }
        }
        for a in &p.trading_houses.built {
            if !on_board(a) {
                return Err(format!("player {} trading house on non-board area {:?}", p.id.0, a));
            }
        }
    }
    if let Some(a) = state.station_location {
        if !on_board(&a) {
            return Err(format!("station token on non-board area {:?}", a));
        }
    }
    if let Some(a) = state.president_start_area {
        if !on_board(&a) {
            return Err(format!("president_start_area is non-board area {:?}", a));
        }
    }
    // Sweep beyond the 4 fields above (types/state.rs audit, T2): the
    // mercenary sub-turn's target area, and the 5-Power token map's keys.
    // board_layout is itself part of stored/serialized GameState, so a
    // corrupted or fabricated save can carry a token entry keyed by a
    // ghost area even though normal generation never produces one
    // (board_gen.rs adds every grid cell to `areas` before optionally
    // giving that same cell a building site / token).
    if let Some(a) = state.mercenary_target_area {
        if !on_board(&a) {
            return Err(format!("mercenary_target_area is non-board area {:?}", a));
        }
    }
    for a in state.board_layout.five_power_tokens.keys() {
        if !on_board(a) {
            return Err(format!("5-power token slot on non-board area {:?}", a));
        }
    }

    // BoardLayout STRUCTURAL integrity (external review I-7, defense-in-depth
    // backlog). BoardLayoutBuilder enforces most of this at construction, but
    // `BoardLayout` is part of the serialized `GameState`: a state deserialized
    // straight into its fields, or hand-built by a bot/record, never runs the
    // builder. Every clause below holds on BOTH generated boards (board_gen:
    // add_adjacency pushes both directions, canal crossings are added to
    // adjacency too, sites/tokens key only off non-Canal cells) and live
    // re-typed boards (reconcile_board rebuilds adjacency/crossings/sites/
    // tokens the same way) -- so none of it fires on an organic state.
    let board = &state.board_layout;
    // building_sites: every key is a real, non-Canal area (five_power_tokens
    // membership is already audited above; a site on the Canal or off-board
    // would misprice or crash Building).
    for area in board.building_sites.keys() {
        if !on_board(area) {
            return Err(format!("building site on non-board area {area:?}"));
        }
        if board.areas[area].area_type == crate::types::AreaType::Canal {
            return Err(format!("building site on Canal area {area:?}"));
        }
    }
    // adjacency: every key and every named neighbor is a real area...
    for (area, neighbors) in &board.adjacency {
        if !on_board(area) {
            return Err(format!("adjacency entry for non-board area {area:?}"));
        }
        for n in neighbors {
            if !on_board(n) {
                return Err(format!("adjacency for {area:?} names non-board neighbor {n:?}"));
            }
        }
    }
    // ...and adjacency is SYMMETRIC (b in adj[a] => a in adj[b]); the movement
    // BFS and every route argument assume an undirected graph.
    for (area, neighbors) in &board.adjacency {
        for n in neighbors {
            if !board.adjacency.get(n).is_some_and(|back| back.contains(area)) {
                return Err(format!("adjacency not symmetric: {area:?} lists {n:?} but not the reverse"));
            }
        }
    }
    // canal_crossings: both endpoints are real areas AND the pair is an
    // adjacency edge -- transit edges ARE adjacency edges (design 2026-07-18;
    // Stage-3 battery), so the BFS can traverse straight across the Canal.
    for (a, b) in &board.canal_crossings {
        if !on_board(a) || !on_board(b) {
            return Err(format!("canal crossing references non-board area: {a:?} <-> {b:?}"));
        }
        let a_to_b = board.adjacency.get(a).is_some_and(|ns| ns.contains(b));
        let b_to_a = board.adjacency.get(b).is_some_and(|ns| ns.contains(a));
        if !a_to_b || !b_to_a {
            return Err(format!("canal crossing {a:?} <-> {b:?} is not an adjacency edge"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{create_test_game_state, finalize_setup, new_game, StartingOrderChoice};
    use crate::types::{AreaId, OrderCardId, PresidentLocation};

    /// Baseline fixture for the per-player / structural checks: a minimal
    /// state (empty decks/board) is fine here because these tests use
    /// `Rules::empty_for_tests()`, which gates the card-conservation checks
    /// off (see module docs) — those get their own golden fixture below.
    fn baseline() -> (GameState, Rules) {
        (create_test_game_state(2), Rules::empty_for_tests())
    }

    /// Golden fixture for the two card-conservation checks: a real,
    /// freshly-dealt 2-player game under the standard 36-order/30-tech
    /// registries, which `check_invariants` must accept as-is.
    fn golden_standard_game() -> (GameState, Rules) {
        let rules = Rules::standard();
        let pending = new_game(crate::types::GameConfig::for_player_count(2), &rules, 42).unwrap();
        let state = finalize_setup(
            pending,
            &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
        )
        .unwrap();
        (state, rules)
    }

    #[test]
    fn assistants_must_total_20_per_player() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        state.players[0].assistants.in_hand += 1; // now 21
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("assistants total"), "expected assistants-total error, got: {err}");
    }

    #[test]
    fn order_hand_limit_is_3() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        state.players[0].order_cards_in_hand =
            vec![OrderCardId(1), OrderCardId(2), OrderCardId(3), OrderCardId(4)];
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("order hand size"), "expected hand-limit error, got: {err}");
    }

    #[test]
    fn shophouse_total_is_8_per_player() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        state.players[0].shophouses.in_hand += 1; // now 9
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("shophouse total"), "expected shophouse-total error, got: {err}");
    }

    #[test]
    fn trading_house_total_is_4_per_player() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        state.players[0].trading_houses.in_warehouse += 1; // now 5
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("trading house total"), "expected TH-total error, got: {err}");
    }

    #[test]
    fn order_conservation_deck_ports_hands_completed_removed_equals_36() {
        let (state, rules) = golden_standard_game();
        assert!(check_invariants(&state, &rules).is_ok());

        let mut corrupt = state.clone();
        corrupt.orders_removed.pop(); // a card vanishes
        let err = check_invariants(&corrupt, &rules).unwrap_err();
        assert!(err.contains("order conservation"), "expected order-conservation error, got: {err}");
    }

    #[test]
    fn tech_conservation_deck_labs_owned_equals_30() {
        let (state, rules) = golden_standard_game();
        assert!(check_invariants(&state, &rules).is_ok());

        let mut corrupt = state.clone();
        corrupt.tech_deck.pop(); // a card vanishes
        let err = check_invariants(&corrupt, &rules).unwrap_err();
        assert!(err.contains("tech conservation"), "expected tech-conservation error, got: {err}");
    }

    #[test]
    fn shophouse_area_slot_pairs_are_unique_across_players() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        let area = AreaId::new(0);
        // Move one shophouse from warehouse to built for each player at the
        // SAME (area, slot) so each player's own total stays 8 (isolating
        // the cross-player uniqueness violation from the total-count check).
        state.players[0].shophouses.in_warehouse -= 1;
        state.players[0].shophouses.built.insert(area, 1);
        state.players[1].shophouses.in_warehouse -= 1;
        state.players[1].shophouses.built.insert(area, 1);

        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("duplicate shophouse"), "expected shophouse-uniqueness error, got: {err}");
    }

    #[test]
    fn trading_house_areas_are_unique_across_players() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        let area = AreaId::new(0);
        state.players[0].trading_houses.in_warehouse -= 1;
        state.players[0].trading_houses.built.push(area);
        state.players[1].trading_houses.in_warehouse -= 1;
        state.players[1].trading_houses.built.push(area);

        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("duplicate trading house"), "expected TH-uniqueness error, got: {err}");
    }

    #[test]
    fn current_action_power_must_not_exceed_5() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        state.current_action_power = Some(6);
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("current_action_power"), "expected power-bound error, got: {err}");
    }

    /// Stage-6 audit finding: `end_game::is_game_over` and
    /// `GameState::is_game_over` had drifted on tampered states where
    /// `end_game_triggered` is set without `trigger_player`/`trigger_reason`
    /// — a state `check_end_game_triggers` can never produce. The free fn
    /// now delegates to the method (single definition), and this invariant
    /// makes the incoherent state loudly invalid for `replay_strict`.
    #[test]
    fn end_game_trigger_fields_must_be_coherent() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        state.end_game_triggered = true;
        state.trigger_player = None;
        state.trigger_reason = None;
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("trigger"), "expected trigger-coherence error, got: {err}");

        // final_round without the trigger flag is equally incoherent.
        let (mut state, rules) = baseline();
        state.final_round = true;
        state.end_game_triggered = false;
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("final_round"), "expected final_round-coherence error, got: {err}");
    }

    /// I1(b) final-review fix (design §5.1 queue invariant): a queued
    /// choice surviving to TurnEnd -- the phase machine's one player
    /// hand-off point -- is a corrupted-state bug, the same class
    /// check_invariants already audits for everything else.
    #[test]
    fn pending_choices_must_be_empty_at_turn_end() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        state.turn_phase = crate::types::TurnPhase::TurnEnd;
        state.pending_choices.push(crate::types::PendingChoice::AnyGood);
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("pending_choices") && err.contains("TurnEnd"), "expected a pending-choices-at-TurnEnd error, got: {err}");
    }

    /// Board membership (design §4.1, T2): a record or bot that fabricates an
    /// `AreaId` not present in `board_layout.areas` must be rejected. Uses the
    /// golden (real-board) fixture rather than `baseline()`, whose
    /// `board_layout.areas` is empty and would make every area a "ghost" --
    /// these mutations need a real board so only the fabricated area fails.
    ///
    /// Covers all FOUR of the brief's original fields plus the two BUILT-token
    /// maps (`shophouses.built`, `trading_houses.built`) that the membership
    /// sweep also guards but that had no firing test until Phase 2.9 Stage 5 --
    /// each moves a token warehouse->built so totals stay conserved (8 / 4)
    /// and ONLY the membership check can be what catches the ghost.
    #[test]
    fn out_of_board_area_ids_violate_invariants() {
        let (state, rules) = golden_standard_game();
        let ghost = AreaId::unchecked(99);

        let mut s = state.clone();
        s.players[0].president_location = PresidentLocation::OnBoard(ghost);
        assert!(check_invariants(&s, &rules).is_err(), "ghost president area must fail");

        let mut s = state.clone();
        s.players[0].assistants.on_board.insert(ghost, 1);
        // keep totals conserved so ONLY the membership check can catch it:
        s.players[0].assistants.in_hand -= 1;
        assert!(check_invariants(&s, &rules).is_err(), "ghost assistant area must fail");

        let mut s = state.clone();
        s.station_location = Some(ghost);
        assert!(check_invariants(&s, &rules).is_err(), "ghost station area must fail");

        let mut s = state.clone();
        s.president_start_area = Some(ghost);
        assert!(check_invariants(&s, &rules).is_err(), "ghost president_start_area must fail");

        // shophouses.built (invariant previously UNCOVERED): warehouse->built
        // keeps the per-player total at 8, so the (area,slot) uniqueness and
        // total checks both pass and the membership check is the only catch.
        let mut s = state.clone();
        s.players[0].shophouses.in_warehouse -= 1;
        s.players[0].shophouses.built.insert(ghost, 0);
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("shophouse on non-board"), "ghost shophouse-built area must fail: {e}");

        // trading_houses.built (invariant previously UNCOVERED): same shape,
        // total stays 4.
        let mut s = state.clone();
        s.players[0].trading_houses.in_warehouse -= 1;
        s.players[0].trading_houses.built.push(ghost);
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("trading house on non-board"), "ghost trading-house-built area must fail: {e}");
    }

    /// T2 field sweep beyond the brief's 4 checks above: two more
    /// `AreaId`-bearing fields found while auditing `types/state.rs` --
    /// the mercenary sub-turn's target area, and the 5-Power token map
    /// nested in `board_layout` (itself part of serialized `GameState`,
    /// so a corrupted/fabricated save can carry a token keyed by a ghost
    /// area even though normal board generation never produces one).
    #[test]
    fn out_of_board_area_ids_violate_invariants_swept_fields() {
        let (state, rules) = golden_standard_game();
        let ghost = AreaId::unchecked(99);

        let mut s = state.clone();
        s.mercenary_target_area = Some(ghost);
        assert!(check_invariants(&s, &rules).is_err(), "ghost mercenary_target_area must fail");

        let mut s = state.clone();
        s.board_layout.five_power_tokens.insert(ghost, None);
        assert!(check_invariants(&s, &rules).is_err(), "ghost 5-power token area must fail");
    }

    /// Two distinct real areas that are NOT adjacent. Exists on every board
    /// bigger than a single clique -- the 2x5 golden board has many (opposite
    /// corners, any two cells >1 grid step apart).
    fn non_adjacent_pair(layout: &crate::types::BoardLayout) -> (AreaId, AreaId) {
        let areas: Vec<AreaId> = layout.areas.keys().copied().collect();
        for &a in &areas {
            for &b in &areas {
                if a != b && !layout.adjacency.get(&a).is_some_and(|ns| ns.contains(&b)) {
                    return (a, b);
                }
            }
        }
        panic!("golden board has no non-adjacent pair (complete graph?)");
    }

    /// I-7 (external Codex review of PR #2, defense-in-depth backlog):
    /// BoardLayout STRUCTURAL integrity. `BoardLayoutBuilder` enforces these
    /// at construction, but a `GameState` deserialized straight into its
    /// fields -- or hand-built by a bot/record -- never runs the builder, so
    /// `check_invariants` re-audits the board itself. Every clause holds on
    /// both generated boards (`board_gen`) and live re-typed boards
    /// (`reconcile_board`); each mutation below trips its TARGETED clause
    /// first (the ghost-neighbor case also implies a latent asymmetry, so
    /// teeth survive either clause's removal — Stage-5 review nit) and needs
    /// the golden real-board fixture (`baseline()`'s board is empty, so no
    /// clause has anything to bite).
    #[test]
    fn board_layout_structural_invariants_catch_corruption() {
        use crate::types::AreaType;
        let (state, rules) = golden_standard_game();
        assert!(check_invariants(&state, &rules).is_ok());
        let ghost = AreaId::unchecked(99);
        let real: Vec<AreaId> = state.board_layout.areas.keys().copied().collect();

        // building_sites keyed off a non-board area.
        let mut s = state.clone();
        let site = s.board_layout.building_sites.values().next().unwrap().clone();
        s.board_layout.building_sites.insert(ghost, site);
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("building site on non-board area"), "{e}");

        // building_sites keyed off the Canal (re-type a real site area to Canal).
        let mut s = state.clone();
        let site_area = *s.board_layout.building_sites.keys().next().unwrap();
        s.board_layout.areas.get_mut(&site_area).unwrap().area_type = AreaType::Canal;
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("building site on Canal area"), "{e}");

        // adjacency keyed off a non-board area.
        let mut s = state.clone();
        s.board_layout.adjacency.insert(ghost, vec![]);
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("adjacency entry for non-board area"), "{e}");

        // adjacency naming a non-board neighbor.
        let mut s = state.clone();
        s.board_layout.adjacency.get_mut(&real[0]).unwrap().push(ghost);
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("non-board neighbor"), "{e}");

        // adjacency asymmetric: drop one direction of an existing edge.
        let mut s = state.clone();
        let (a, b) = {
            let (a, ns) = s.board_layout.adjacency.iter().find(|(_, ns)| !ns.is_empty()).unwrap();
            (*a, ns[0])
        };
        s.board_layout.adjacency.get_mut(&b).unwrap().retain(|&x| x != a);
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("not symmetric"), "{e}");

        // canal_crossings with a non-board endpoint.
        let mut s = state.clone();
        s.board_layout.canal_crossings.push((real[0], ghost));
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("canal crossing references non-board area"), "{e}");

        // canal_crossings on a real pair that is NOT an adjacency edge
        // (transit edges MUST be adjacency edges -- Stage-3 battery).
        let mut s = state.clone();
        let (a, b) = non_adjacent_pair(&s.board_layout);
        s.board_layout.canal_crossings.push((a, b));
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("not an adjacency edge"), "{e}");
    }

    /// Queue-head resolvability (Phase 2.9 Stage 5, Stage-4 census finding):
    /// `get_legal_moves` is queue-first, so a non-empty `pending_choices`
    /// whose HEAD has an empty resolution set livelocks the driver --
    /// `get_legal_moves` returns nothing, `is_game_over` is false, and no move
    /// can ever apply. Organic play never reaches it (`push_choice`
    /// auto-resolves singletons, `normalize_queue_head` clears lone-declines,
    /// and Church/Customs/Station heads only queue with >=2 live options), but
    /// a tampered/deserialized head can -- so `check_invariants` (hence
    /// `replay_strict`) rejects it loudly instead of hanging.
    #[test]
    fn pending_choice_head_with_empty_resolution_set_is_rejected() {
        // A CustomsSpace head at a printed cost no space carries (tampered
        // cost): eligible_customs_spaces_for_cost returns empty, so no
        // resolution move exists.
        let (mut s, rules) = golden_standard_game();
        s.pending_choices.push(crate::types::PendingChoice::CustomsSpace { cost: 99 });
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("no legal resolution"), "{e}");

        // StationPlace on a board with no placeable area (empty baseline
        // board): the token has nowhere to land. Distinct variant (no Decline
        // at all), same livelock class.
        let (mut s, rules) = baseline();
        s.pending_choices.push(crate::types::PendingChoice::StationPlace);
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("no legal resolution"), "{e}");
    }
}
