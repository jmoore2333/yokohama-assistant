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
        // I11 (Phase 3.0): starting-order coherence. `starting_order` is
        // effectively write-only in the engine (types/player.rs:244 init,
        // game/setup.rs:251, types/state.rs:446 -- each pairing the record with
        // the card's arrival in hand) and no legal move ever clears it, while
        // the one hand-removal site (apply_move.rs:1335) pushes straight to
        // `completed_orders`. So no reachable state violates either clause --
        // but Phase 3 derives seat views and SAMPLES states from them, and both
        // read `starting_order` to decide what to mask. A dangling value would
        // mask the wrong card (or nothing) with no error, so the guarantee
        // moves into the mutation-proven invariant net. Same class as I9:
        // invariant/test-only, no SEMANTICS_REV implication.
        if let Some(c) = p.starting_order {
            if !p.order_cards_in_hand.contains(&c) && !p.completed_orders.contains(&c) {
                return Err(format!(
                    "P{} starting_order {} is in neither the hand nor the completed pile",
                    p.id.0, c.0
                ));
            }
        }
        // The live-tracking fiction is that a hidden deal is recorded NOWHERE
        // (both dealt cards park in orders_removed until reveal_starting_order
        // pulls one out and clears the flag in the same breath), so the flag
        // and an identity are mutually exclusive.
        if p.starting_order_hidden && p.starting_order.is_some() {
            return Err(format!(
                "P{} starting_order_hidden is set but an identity is recorded",
                p.id.0
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

    // I9 (PR #5 §2.1): the 2-player setup dummy (`PlayerId::DUMMY`) occupies
    // EXACTLY Church faith-7 and Customs space-4, and only at 2 players. Since
    // finding D1 the engine reads "is this a 2P game" off `slots[3].is_some()`
    // while the oracle reads it off `player_count == 2`; without this clause a
    // 2P state with an empty faith-7 (or a dummy loose on some other slot, or a
    // stray dummy at 3/4 players) is representable and the two crates silently
    // disagree. No production path builds one -- every setup path funnels
    // through `place_setup_dummies` and no legal move ever vacates or adds a
    // track dummy -- but Phase 3 constructs states outside `new_game`, so this
    // moves the guarantee into the per-edit, mutation-proven, fuzz-exercised net.
    {
        let church_dummy = crate::types::ChurchTrack::faith_to_index(7)
            .expect("faith 7 is on the Church track");
        let customs_dummy = crate::types::CustomsTrack::space_to_index(4)
            .expect("space 4 is on the Customs track");
        let is_two_player = state.config.player_count == 2;
        // Presence: at 2 players the two fixed spaces MUST carry the dummy.
        if is_two_player {
            if !state.church_track.slots[church_dummy].is_some_and(|s| s.player.is_dummy()) {
                return Err("2-player state missing the setup dummy on Church faith-7".to_string());
            }
            if !state.customs_track.slots[customs_dummy].is_some_and(|s| s.player.is_dummy()) {
                return Err("2-player state missing the setup dummy on Customs space-4".to_string());
            }
        }
        // Exclusivity: the dummy may appear ONLY on those two spaces at 2
        // players, and nowhere at all above two players.
        for (i, slot) in state.church_track.slots.iter().enumerate() {
            if slot.is_some_and(|s| s.player.is_dummy()) && !(is_two_player && i == church_dummy) {
                return Err(format!(
                    "setup dummy on Church slot {i} (faith {}) -- the dummy is legal only on 2-player Church faith-7",
                    crate::types::ChurchTrack::index_to_faith(i)
                ));
            }
        }
        for (i, slot) in state.customs_track.slots.iter().enumerate() {
            if slot.is_some_and(|s| s.player.is_dummy()) && !(is_two_player && i == customs_dummy) {
                return Err(format!(
                    "setup dummy on Customs slot {i} (space {}) -- the dummy is legal only on 2-player Customs space-4",
                    crate::types::CustomsTrack::index_to_space(i)
                ));
            }
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
    //
    // Head COHERENCE (FIX-B P1, Codex PR-3 review): a non-empty resolution
    // SET is necessary but not sufficient -- the Stage-4 forged-head guards
    // in resolve_choice_move / the Church-Customs placement tails reject
    // every GENERATED move for a head that is incoherent with the rest of
    // the state, which deadlocks (moves listed, none applies) while the
    // clause above stays green. The cheap field checks below mirror those
    // guards exactly: generation-nonempty PLUS these coherence clauses
    // TOGETHER approximate "some listed move actually applies". A full
    // try-apply check (fold each listed move against a clone) is
    // deliberately NOT done: check_invariants runs after EVERY move in the
    // per-move fuzz path, and try-applying whole resolution sets there
    // would multiply its cost by the branching factor.
    if let Some(head) = state.pending_choices.first() {
        if crate::game::choices::generate_choice_resolutions(state, head, rules).is_empty() {
            return Err(format!(
                "pending choice head {head:?} has no legal resolution (empty resolution set would livelock get_legal_moves)"
            ));
        }
        match head {
            // Mirror of resolve_choice_move's placed-once guard: a
            // StationPlace head generates placements for every placeable
            // area, but with the token already down every one is rejected.
            crate::types::PendingChoice::StationPlace => {
                if state.station_location.is_some() {
                    return Err(format!(
                        "pending StationPlace head with the Station token already placed at {:?} (placed once, immovable -- every resolution would be rejected)",
                        state.station_location
                    ));
                }
            }
            // Mirror of resolve_choice_move's phase guard: both
            // ChooseRecoveryLeave moves are always generated, but outside
            // the Recovery phase both are rejected.
            crate::types::PendingChoice::RecoveryLeave => {
                if state.turn_phase != crate::types::TurnPhase::Recovery {
                    return Err(format!(
                        "pending RecoveryLeave head outside the Recovery phase (turn_phase {:?} -- every resolution would be rejected)",
                        state.turn_phase
                    ));
                }
            }
            // Mirror of the Stage-4 placement-tail guards
            // (place_church_assistant / place_customs_assistant): the head
            // generates its eligible spaces, but every space resolution is
            // rejected unless the acting area resolves AND the current
            // player still has an assistant in hand or on that area (the
            // engine's own resolve_acting_area +
            // has_church_or_customs_assistant, reused verbatim).
            crate::types::PendingChoice::ChurchSpace { .. }
            | crate::types::PendingChoice::CustomsSpace { .. } => {
                let name = if matches!(head, crate::types::PendingChoice::ChurchSpace { .. }) {
                    "ChurchSpace"
                } else {
                    "CustomsSpace"
                };
                match crate::game::actions::resolve_acting_area(state, state.current_player) {
                    Err(e) => {
                        return Err(format!(
                            "pending {name} head whose acting area does not resolve ({e}) -- every space resolution would be rejected"
                        ));
                    }
                    Ok(area) => {
                        if !crate::game::actions::has_church_or_customs_assistant(
                            state.current_player(),
                            area,
                        ) {
                            return Err(format!(
                                "pending {name} head with no assistant source (hand and acting area {area:?} both empty) -- every space resolution would be rejected"
                            ));
                        }
                    }
                }
            }
            _ => {}
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
    use crate::types::{
        AreaId, ChurchSlot, ChurchTrack, CustomsSlot, CustomsTrack, GameConfig, OrderCardId,
        PlayerId, PresidentLocation,
    };

    /// Baseline fixture for the per-player / structural checks: a minimal
    /// state (empty decks/board) is fine here because these tests use
    /// `Rules::empty_for_tests()`, which gates the card-conservation checks
    /// off (see module docs) — those get their own golden fixture below.
    fn baseline() -> (GameState, Rules) {
        let mut state = create_test_game_state(2);
        // I9 (PR #5 §2.1): a valid 2-player state carries the setup dummy on
        // Church faith-7 / Customs space-4. `create_test_game_state` is a bare
        // skeleton that skips it (it is not a production setup path), so place
        // it here -- exactly as `new_game` does -- so the fixture satisfies the
        // new track-dummy invariant that every other clause's mutation test
        // builds on.
        crate::game::setup::place_setup_dummies(&mut state);
        (state, Rules::empty_for_tests())
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
    /// bigger than a single clique -- the 2p golden board (a 4/3/2/1 staggered
    /// pyramid) has many: the two ends of its 4-tile top row, or its apex and
    /// any top-row tile.
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

    /// Any non-Canal placeable area on the golden board (real fixture, so
    /// the head-coherence tests below can forge heads whose RESOLUTION SET
    /// is non-empty -- the exact gap the resolvability clause alone misses).
    fn placeable_area(state: &GameState) -> crate::types::AreaId {
        *state
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type.allows_placement())
            .map(|(a, _)| a)
            .expect("golden board has a placeable area")
    }

    /// Assert the corrupt state is a true DEADLOCK: `get_legal_moves` (queue-
    /// first) lists the head's resolutions, yet the checked `apply_move`
    /// rejects every one of them -- the game can never advance.
    fn assert_deadlocked(state: &GameState, rules: &Rules) {
        let moves = crate::game::get_legal_moves(state, rules);
        assert!(!moves.is_empty(), "resolution set must be NON-empty (the resolvability clause alone must not fire)");
        for mv in &moves {
            assert!(
                crate::game::apply_move(state, mv, rules).is_err(),
                "deadlock fixture broken: {mv:?} applied"
            );
        }
    }

    /// FIX-B P1 (Codex PR-3 review), head-coherence clause (a): a forged
    /// `StationPlace` head with the Station token ALREADY placed generates a
    /// non-empty resolution set (every placeable area), but the Stage-4
    /// placed-once guard in `resolve_choice_move` rejects every one --
    /// deadlock while the resolvability clause alone stays green.
    #[test]
    fn forged_station_place_head_with_token_already_placed_is_rejected() {
        let (mut s, rules) = golden_standard_game();
        s.station_location = Some(placeable_area(&s));
        s.pending_choices.push(crate::types::PendingChoice::StationPlace);

        assert_deadlocked(&s, &rules);
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("StationPlace") && e.contains("already placed"), "{e}");
    }

    /// FIX-B P1, head-coherence clause (b): a forged `RecoveryLeave` head
    /// outside the Recovery phase always generates both `ChooseRecoveryLeave`
    /// moves, but the Stage-4 phase guard rejects them -- same deadlock class.
    #[test]
    fn forged_recovery_leave_head_outside_recovery_phase_is_rejected() {
        let (mut s, rules) = golden_standard_game();
        assert_ne!(s.turn_phase, crate::types::TurnPhase::Recovery, "fixture must start outside Recovery");
        s.pending_choices.push(crate::types::PendingChoice::RecoveryLeave);

        assert_deadlocked(&s, &rules);
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("RecoveryLeave") && e.contains("Recovery"), "{e}");
    }

    /// FIX-B P1, head-coherence clause (c) -- the orchestrator's
    /// generalization of the Stage-4 class: a `ChurchSpace`/`CustomsSpace`
    /// head whose assistant source is GONE (hand empty AND the acting area
    /// empty) still generates its eligible spaces, but the Stage-4 tail guard
    /// (`place_church_assistant`/`place_customs_assistant`) rejects every
    /// space -- deadlock. Also covers the acting area itself failing to
    /// resolve (President in hand, no mercenary sub-turn).
    #[test]
    fn church_or_customs_head_with_assistant_source_gone_is_rejected() {
        // Hand + acting area both empty: President parked on a real area,
        // every hand assistant moved to the warehouse (conserving the 20).
        let (mut s, rules) = golden_standard_game();
        let area = placeable_area(&s);
        let cur = s.current_player;
        {
            let p = s.player_mut(cur);
            p.president_location = PresidentLocation::OnBoard(area);
            p.assistants.in_warehouse += p.assistants.in_hand;
            p.assistants.in_hand = 0;
            let on_area = p.assistants.in_area(area);
            p.assistants.in_warehouse += on_area;
            p.assistants.on_board.remove(&area);
        }
        s.pending_choices.push(crate::types::PendingChoice::ChurchSpace { total_faith: 5 });
        assert_deadlocked(&s, &rules);
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("ChurchSpace") && e.contains("assistant"), "{e}");

        // Same drained state, CustomsSpace head: same clause, same catch.
        s.pending_choices.clear();
        s.pending_choices.push(crate::types::PendingChoice::CustomsSpace { cost: 1 });
        assert_deadlocked(&s, &rules);
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("CustomsSpace") && e.contains("assistant"), "{e}");

        // Acting area unresolvable (President in hand, not a mercenary
        // sub-turn): the tail's `resolve_acting_area` rejects every space.
        let (mut s, rules) = golden_standard_game();
        let cur = s.current_player;
        s.player_mut(cur).president_location = PresidentLocation::InHand;
        s.pending_choices.push(crate::types::PendingChoice::ChurchSpace { total_faith: 5 });
        assert_deadlocked(&s, &rules);
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("acting area"), "{e}");
    }

    /// I9 (PR #5 §2.1): the 2-player setup dummy (`PlayerId::DUMMY`) occupies
    /// EXACTLY Church faith-7 and Customs space-4 — and nowhere else. Since
    /// finding D1 the engine reads "is this a 2P game" off `slots[3].is_some()`
    /// while the oracle reads it off `player_count == 2`, so a 2P state with an
    /// empty faith-7 (or a dummy loose on some other slot) is representable and
    /// the two crates silently disagree. Mutates the golden real-2P fixture
    /// (dummy present by construction) one clause at a time.
    #[test]
    fn two_player_setup_dummy_must_sit_on_faith7_and_space4_and_nowhere_else() {
        let church_dummy = ChurchTrack::faith_to_index(7).unwrap();
        let customs_dummy = CustomsTrack::space_to_index(4).unwrap();
        let (state, rules) = golden_standard_game();
        assert!(check_invariants(&state, &rules).is_ok());

        // Presence (a): 2P with an empty Church faith-7.
        let mut s = state.clone();
        s.church_track.slots[church_dummy] = None;
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("dummy") && e.contains("Church faith-7"), "{e}");

        // Presence (a): 2P with an empty Customs space-4.
        let mut s = state.clone();
        s.customs_track.slots[customs_dummy] = None;
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("dummy") && e.contains("Customs space-4"), "{e}");

        // Exclusivity (b): a dummy loose on ANOTHER Church slot (faith-4). The
        // real dummy stays on faith-7, so ONLY the "nowhere else" clause fires.
        let mut s = state.clone();
        s.church_track.slots[ChurchTrack::faith_to_index(4).unwrap()] =
            Some(ChurchSlot { player: PlayerId::DUMMY, arrival_order: 0 });
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("dummy") && e.contains("Church"), "{e}");

        // Exclusivity (b): a dummy loose on ANOTHER Customs slot (space-1).
        let mut s = state.clone();
        s.customs_track.slots[CustomsTrack::space_to_index(1).unwrap()] =
            Some(CustomsSlot { player: PlayerId::DUMMY, arrival_order: 0 });
        let e = check_invariants(&s, &rules).unwrap_err();
        assert!(e.contains("dummy") && e.contains("Customs"), "{e}");
    }

    /// I11 (Phase 3.0) clause (a): `starting_order` must name a card the
    /// player actually holds — in hand, or in the completed pile once played.
    /// No reachable state violates it (the field's only three write sites all
    /// pair it with the card's arrival, and the sole hand-removal site pushes
    /// straight to `completed_orders`), but Phase 3 derives seat views and
    /// SAMPLES states from them, and both read `starting_order` to decide what
    /// to mask — a dangling value masks the wrong card, silently.
    #[test]
    fn a_dangling_starting_order_is_caught() {
        let (state, rules) = golden_standard_game();
        assert!(check_invariants(&state, &rules).is_ok(), "control: real setup is clean");
        let card = state.players[1].starting_order.unwrap();

        // Positive control for the second zone: completing the starting order
        // moves it hand -> completed pile (apply_move.rs:1335-1336 does exactly
        // that pair of writes), and the result is still coherent. The clause
        // must accept BOTH zones or it fires on ordinary play.
        let mut s = state.clone();
        s.players[1].order_cards_in_hand.retain(|&c| c != card);
        s.players[1].completed_orders.push(card);
        assert!(check_invariants(&s, &rules).is_ok(), "a COMPLETED starting order is coherent");

        // Dangling: recorded, but held in neither zone.
        let mut s = state.clone();
        s.players[1].order_cards_in_hand.retain(|&c| c != card);
        s.orders_removed.push(card); // keep the 36-card count honest
        let err = check_invariants(&s, &rules).unwrap_err();
        assert!(
            err.contains("starting_order") && err.contains("neither the hand"),
            "got: {err}"
        );
    }

    /// I11 clause (b): `starting_order_hidden` is the live-tracking fiction
    /// that an opponent's kept card is recorded NOWHERE (both dealt cards sit
    /// in `orders_removed` until `reveal_starting_order` pulls one out and
    /// clears the flag in the same breath). Flag-set-plus-identity is the
    /// contradiction: an observation mask would treat the seat as unknown
    /// while the state names the card.
    ///
    /// The recorded card is a REAL card in P1's hand, so clause (a) is
    /// satisfied and only the contradiction itself can fire.
    #[test]
    fn a_hidden_deal_that_also_records_a_card_is_caught() {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 42).unwrap();
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown])
                .unwrap();
        assert!(check_invariants(&state, &rules).is_ok(), "control: live setup is clean");

        // removed -> hand keeps the 36-card count honest, exactly as a real
        // reveal would; then record it WITHOUT clearing the hidden flag.
        let card = state.orders_removed.pop().unwrap();
        state.players[1].order_cards_in_hand.push(card);
        state.players[1].starting_order = Some(card);
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(
            err.contains("starting_order_hidden") && err.contains("identity is recorded"),
            "got: {err}"
        );
    }

    /// I9, the symmetric 3/4-player direction: a game with no dummy is clean,
    /// and a dummy planted on ANY track slot above two players is rejected.
    #[test]
    fn no_setup_dummy_above_two_players() {
        let rules = Rules::standard();
        for players in [3u8, 4] {
            let pending = new_game(GameConfig::for_player_count(players), &rules, 3).unwrap();
            let choices: Vec<StartingOrderChoice> =
                (0..players).map(|_| StartingOrderChoice::Keep(0)).collect();
            let state = finalize_setup(pending, &choices).unwrap();
            assert!(check_invariants(&state, &rules).is_ok(), "{players}P golden must be clean");

            let mut s = state.clone();
            s.church_track.slots[0] = Some(ChurchSlot { player: PlayerId::DUMMY, arrival_order: 0 });
            let e = check_invariants(&s, &rules).unwrap_err();
            assert!(e.contains("dummy"), "{players}P church dummy: {e}");

            let mut s = state.clone();
            s.customs_track.slots[0] =
                Some(CustomsSlot { player: PlayerId::DUMMY, arrival_order: 0 });
            let e = check_invariants(&s, &rules).unwrap_err();
            assert!(e.contains("dummy"), "{players}P customs dummy: {e}");
        }
    }
}
