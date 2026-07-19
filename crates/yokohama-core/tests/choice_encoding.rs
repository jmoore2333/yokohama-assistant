//! Task 15 (design 2026-07-18 §6.1-1): choice-encoding behavioral proof +
//! organic-coverage census. Extended by engine-authority Task 5 (design
//! 2026-07-19 §4.2) to add `StationPlace`.
//!
//! Choice-type inventory (enumerated from `types/choices.rs` /
//! `game/choices.rs` / `types/moves.rs` -- the source of truth, not the
//! task brief): seven `PendingChoice` variants --
//!   `AnyGood`, `WarehouseTake`, `AssistantMove`,
//!   `ChurchSpace { total_faith }`, `CustomsSpace { cost }`, `RecoveryLeave`,
//!   `StationPlace`
//! -- resolved by nine `Move` variants (`RecoveryLeave` has two, so ten
//! resolution shapes total) --
//!   `ChooseAnyGood`, `ChooseWarehouseTake`, `DeclineWarehouseTake`,
//!   `ChooseAssistantMove`, `DeclineAssistantMove`, `ChooseChurchSpace`,
//!   `ChooseCustomsSpace`, `ChooseRecoveryLeave { leave_one: bool }`,
//!   `ChooseStationPlace`.
//!
//! Every fixture below drives through the public API only
//! (`get_legal_moves`/`apply_move`/`push_choice` from a `create_test_game_state`
//! or `new_game`-derived `GameState`) -- a deliberate choice, not a
//! visibility constraint (T15 final-review fix: the other helpers in
//! `game/choices.rs`/`game/actions/tracks.rs`, e.g. `resolve_choice_move`,
//! `place_church_assistant`, `place_customs_assistant`, are ordinary
//! `pub fn`s, not `#[cfg(test)]`-hidden, and reachable from an integration
//! test like this one via `yokohama_core::game::actions::...`). Sticking to
//! the same handful of entry points a real caller (the CLI, a future bot)
//! would use is what proves the choice-encoding behavior end-to-end through
//! the actual public surface, rather than reaching around it.

mod common;
use common::run_random_game_observed;

use std::collections::BTreeSet;

use yokohama_core::{
    apply_move, create_test_game_state, get_legal_moves, push_choice, replay_from, AreaConfig,
    AreaAction, AreaId, AreaType, Country, GameRecord, GameState, Move, MoveSource, OrderCard,
    OrderCardId, OrderRewards, PendingChoice, PresidentLocation, RecordedEvent, Rules,
    StartingOrderChoice, TechCard, TechCardId, TechEffect, TradeGood, TradeGoodRequirement,
    TurnPhase, WarehouseItem,
};

fn area0() -> AreaId {
    AreaId::new(0)
}

/// Register one custom Order card (zero requirements, so it's always
/// affordable) with the given `rewards`, in an otherwise-empty registry.
/// The standard way this module drives a specific cascade shape without
/// depending on real card data or a dealt hand.
fn order_rules_with_rewards(rewards: OrderRewards) -> (OrderCardId, Rules) {
    let order_id = OrderCardId(1);
    let mut rules = Rules::empty_for_tests();
    rules.orders.insert(
        OrderCard {
            id: order_id,
            country: Country::English,
            requirements: TradeGoodRequirement::default(),
            rewards,
        },
        "test_order",
    );
    (order_id, rules)
}

/// A 2-player state, `area0` on the board (`ty`), the custom order (from
/// `order_rules_with_rewards`) already in player 0's hand, ready for
/// `Move::FulfillOrder`.
fn order_fixture(rewards: OrderRewards, ty: AreaType) -> (GameState, OrderCardId, Rules) {
    let (order_id, rules) = order_rules_with_rewards(rewards);
    let mut state = create_test_game_state(2);
    state.turn_phase = TurnPhase::AdditionalActionFirst;
    state.board_layout.areas.insert(area0(), AreaConfig { area_type: ty, grid_position: (0, 0) });
    state.players[0].order_cards_in_hand.push(order_id);
    (state, order_id, rules)
}

// ===========================================================================
// One public-API scenario per PendingChoice variant.
// ===========================================================================

#[test]
fn any_good_choice_resolves_via_public_api() {
    let (state, order_id, rules) =
        order_fixture(OrderRewards { any_trade_good: 1, ..Default::default() }, AreaType::Bank);
    let starting_tea = state.players[0].tea;

    let mid = apply_move(&state, &Move::FulfillOrder { order_id }, &rules).unwrap();
    assert_eq!(mid.pending_choices, vec![PendingChoice::AnyGood]);
    assert_eq!(
        get_legal_moves(&mid, &rules),
        vec![
            Move::ChooseAnyGood { good: TradeGood::Copper },
            Move::ChooseAnyGood { good: TradeGood::Silk },
            Move::ChooseAnyGood { good: TradeGood::Tea },
            Move::ChooseAnyGood { good: TradeGood::Seafood },
        ]
    );

    let done = apply_move(&mid, &Move::ChooseAnyGood { good: TradeGood::Tea }, &rules).unwrap();
    assert_eq!(done.players[0].tea, starting_tea + 1);
    assert!(done.pending_choices.is_empty());
}

#[test]
fn warehouse_take_choice_resolves_via_public_api() {
    let (state, order_id, rules) =
        order_fixture(OrderRewards { warehouse_tokens: 1, ..Default::default() }, AreaType::Bank);
    let starting_hand = state.players[0].assistants.in_hand;
    let starting_warehouse = state.players[0].assistants.in_warehouse;

    let mid = apply_move(&state, &Move::FulfillOrder { order_id }, &rules).unwrap();
    assert_eq!(mid.pending_choices, vec![PendingChoice::WarehouseTake]);

    let done =
        apply_move(&mid, &Move::ChooseWarehouseTake { item: WarehouseItem::Assistant }, &rules).unwrap();
    assert_eq!(done.players[0].assistants.in_hand, starting_hand + 1);
    assert_eq!(done.players[0].assistants.in_warehouse, starting_warehouse - 1);
    assert!(done.pending_choices.is_empty());
}

#[test]
fn assistant_move_choice_resolves_via_public_api() {
    let (state, order_id, rules) =
        order_fixture(OrderRewards { movement: 1, ..Default::default() }, AreaType::Bank);
    let starting_hand = state.players[0].assistants.in_hand;

    let mid = apply_move(&state, &Move::FulfillOrder { order_id }, &rules).unwrap();
    assert_eq!(mid.pending_choices, vec![PendingChoice::AssistantMove]);
    assert!(get_legal_moves(&mid, &rules)
        .contains(&Move::ChooseAssistantMove { from: MoveSource::Hand, to: area0() }));

    let done = apply_move(
        &mid,
        &Move::ChooseAssistantMove { from: MoveSource::Hand, to: area0() },
        &rules,
    )
    .unwrap();
    assert_eq!(done.players[0].assistants.in_hand, starting_hand - 1);
    assert_eq!(done.players[0].assistants.on_board[&area0()], 1);
    assert!(done.pending_choices.is_empty());
}

/// Church area + president there (3P, so the 2P faith-7 dummy space never
/// interferes -- irrelevant at faith 6 but matches the pattern), goods for
/// a fixed 3-type contribution (+3 Faith) so `current_action_power = 3`
/// lands total Faith at 6 (design §5.4: faith = power + distinct payment
/// types; church_faith_from_power caps power itself at 5, unrelated to the
/// distinct-types bonus). Fresh Church track -> `eligible_church_spaces`
/// returns {4,5,6} (all <= 6, all empty) -- 3 eligible, so the choice
/// genuinely queues instead of placing inline.
fn church_move() -> Move {
    Move::ExecuteAreaAction {
        action: AreaAction::Church {
            copper: true,
            silk: true,
            tea: true,
            seafood: false,
            yen: false,
            imported_good: false,
        },
    }
}

/// Combined ChurchSpace per-type scenario AND the church->riders cascade
/// (design §6.1-1 lists them separately, but ChurchSpace can only ever be
/// observed as a genuinely queued choice in the >=2-eligible-spaces case,
/// which is exactly the case that also has movement riders -- there's no
/// separate simpler shape to demonstrate ChurchSpace alone).
#[test]
fn church_space_choice_places_and_cascades_into_movement_riders() {
    let rules = Rules::empty_for_tests();
    let mut state = create_test_game_state(3);
    state.board_layout.areas.insert(area0(), AreaConfig { area_type: AreaType::Church, grid_position: (0, 0) });
    state.players[0].president_location = PresidentLocation::OnBoard(area0());
    state.turn_phase = TurnPhase::AreaAction;
    state.current_action_power = Some(3);
    state.players[0].copper = 1;
    state.players[0].silk = 1;
    state.players[0].tea = 1;
    let starting_hand = state.players[0].assistants.in_hand;

    let mid = apply_move(&state, &church_move(), &rules).unwrap();
    assert_eq!(mid.pending_choices, vec![PendingChoice::ChurchSpace { total_faith: 6 }]);
    assert_eq!(
        get_legal_moves(&mid, &rules),
        vec![
            Move::ChooseChurchSpace { space: 4 },
            Move::ChooseChurchSpace { space: 5 },
            Move::ChooseChurchSpace { space: 6 },
        ]
    );

    let after_space = apply_move(&mid, &Move::ChooseChurchSpace { space: 6 }, &rules).unwrap();
    assert_eq!(after_space.players[0].church_positions, vec![6]);
    assert_eq!(after_space.players[0].victory_points, 5); // ChurchTrack::vp_reward(6)
    assert_eq!(after_space.players[0].assistants.in_hand, starting_hand - 1); // placement, hand-first
    // Faith 6 -> ChurchTrack::movement_rider(6) == 2 riders (the "church->riders" cascade).
    assert_eq!(after_space.pending_choices, vec![PendingChoice::AssistantMove, PendingChoice::AssistantMove]);

    // Drain both riders through the public API: one real move, one decline.
    let after_r1 = apply_move(
        &after_space,
        &Move::ChooseAssistantMove { from: MoveSource::Hand, to: area0() },
        &rules,
    )
    .unwrap();
    assert_eq!(after_r1.pending_choices, vec![PendingChoice::AssistantMove]);
    let after_r2 = apply_move(&after_r1, &Move::DeclineAssistantMove, &rules).unwrap();
    assert!(after_r2.pending_choices.is_empty());
}

fn customs_move(imported_goods_paid: u8) -> Move {
    Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid } }
}

/// Combined CustomsSpace per-type scenario AND the customs-1->2-takes
/// cascade. Unlike Church (every space 4-11 carries a rider), Customs
/// spaces 5-7 carry NONE (`CustomsTrack::rider` is `(0, 0)` there) -- cost 1
/// specifically is chosen because its pair {1,2} is both empty on a fresh
/// track, so paying 1 import queues CustomsSpace{cost:1}, and space 1
/// specifically carries the +2-warehouse-take rider
/// (`CustomsTrack::rider(1) == (2, 0)`) that makes this double as the
/// cascade test.
#[test]
fn customs_space_choice_places_and_cascades_into_warehouse_takes() {
    let rules = Rules::empty_for_tests();
    let mut state = create_test_game_state(2);
    state.board_layout.areas.insert(area0(), AreaConfig { area_type: AreaType::Customs, grid_position: (0, 0) });
    state.players[0].president_location = PresidentLocation::OnBoard(area0());
    state.turn_phase = TurnPhase::AreaAction;
    state.current_action_power = Some(3);
    state.players[0].imported_goods = 1;
    let starting_hand = state.players[0].assistants.in_hand;
    let starting_warehouse = state.players[0].assistants.in_warehouse;
    let starting_money = state.players[0].money;

    let mid = apply_move(&state, &customs_move(1), &rules).unwrap();
    assert_eq!(mid.pending_choices, vec![PendingChoice::CustomsSpace { cost: 1 }]);
    assert_eq!(
        get_legal_moves(&mid, &rules),
        vec![Move::ChooseCustomsSpace { space: 1 }, Move::ChooseCustomsSpace { space: 2 }]
    );

    let placed = apply_move(&mid, &Move::ChooseCustomsSpace { space: 1 }, &rules).unwrap();
    assert_eq!(placed.players[0].customs_positions, vec![1]);
    assert_eq!(placed.players[0].victory_points, 5); // CustomsTrack::vp_reward(1)
    assert_eq!(placed.players[0].assistants.in_hand, starting_hand - 1); // placement, hand-first
    assert_eq!(placed.pending_choices, vec![PendingChoice::WarehouseTake, PendingChoice::WarehouseTake]);

    // Drain both rider takes: one real take, one decline.
    let after_take =
        apply_move(&placed, &Move::ChooseWarehouseTake { item: WarehouseItem::Assistant }, &rules).unwrap();
    assert_eq!(after_take.pending_choices, vec![PendingChoice::WarehouseTake]);
    let final_state = apply_move(&after_take, &Move::DeclineWarehouseTake, &rules).unwrap();
    assert!(final_state.pending_choices.is_empty());

    let p = &final_state.players[0];
    assert_eq!(p.assistants.in_hand, starting_hand - 1 + 1, "placement spent 1, the Assistant take refunded 1");
    assert_eq!(p.assistants.in_warehouse, starting_warehouse - 1);
    assert_eq!(p.money, starting_money, "space 1's rider carries no yen; Assistant take is free");
}

fn telephone_tech(id: u8) -> TechCard {
    TechCard { id: TechCardId(id), name: "Telephone".to_string(), country: Country::American, industry_cost: 6, effect: TechEffect::Telephone }
}

/// RecoveryLeave is only queued when the current player owns Telephone AND
/// has >=1 own assistant in the Recovery acting area -- reached here via a
/// seeded near-state scenario (fuzz gate's rare-trigger precedent: surgically
/// construct the one precondition random play might not stumble on, then
/// drive the rest through the public API) rather than organic play, so this
/// scenario stays deterministic and fast regardless of what the census (see
/// bottom of file) happens to find. Measurement note: the census DOES also
/// reach this variant organically (first at seed 3 -- see CENSUS_SEED_CAP's
/// doc comment) -- Telephone turned out not to be the rare case the task
/// brief anticipated -- but a census is a regression net, not the primary
/// proof for any one variant, so this dedicated scenario stays regardless.
#[test]
fn recovery_leave_choice_resolves_via_public_api() {
    let mut state = create_test_game_state(2);
    state.board_layout.areas.insert(area0(), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
    state.turn_phase = TurnPhase::Building;
    state.players[0].president_location = PresidentLocation::OnBoard(area0());
    state.players[0].assistants.on_board.insert(area0(), 2);

    let mut rules = Rules::empty_for_tests();
    rules.techs.insert(telephone_tech(1), "tech_telephone");
    state.players[0].technologies.push(TechCardId(1));

    let mid = apply_move(&state, &Move::SkipBuilding, &rules).unwrap();
    assert_eq!(mid.turn_phase, TurnPhase::Recovery); // paused, not auto-completed
    assert_eq!(mid.pending_choices, vec![PendingChoice::RecoveryLeave]);
    assert_eq!(
        get_legal_moves(&mid, &rules),
        vec![
            Move::ChooseRecoveryLeave { leave_one: false },
            Move::ChooseRecoveryLeave { leave_one: true },
        ]
    );

    let done = apply_move(&mid, &Move::ChooseRecoveryLeave { leave_one: true }, &rules).unwrap();
    assert_eq!(done.players[0].assistants.on_board[&area0()], 1); // 1 of 2 recovered, 1 left
    assert_eq!(done.turn_phase, TurnPhase::AdditionalActionSecond);
    assert!(done.pending_choices.is_empty());
}

/// StationPlace (design §4.2, Task 5): queued when the Station tech is
/// acquired, one `ChooseStationPlace` per placeable (non-Canal) area, no
/// Decline. `create_test_game_state`'s board starts empty (its own doc
/// comment: "minimal test game state"), so this test populates 4 placeable
/// areas + 1 Canal directly, the same way `order_fixture` does elsewhere in
/// this file -- exercising resolution in isolation from the Laboratory
/// acquisition hook (that end-to-end wiring is
/// `acquiring_station_queues_a_station_place_choice` in
/// `game/actions/cards.rs`).
#[test]
fn station_place_resolutions_cover_all_placeable_areas_and_resolution_sets_the_token() {
    let rules = Rules::standard();
    let mut state = create_test_game_state(2);
    for i in 0..4u8 {
        state.board_layout.areas.insert(
            AreaId::new(i),
            AreaConfig { area_type: AreaType::Bank, grid_position: (0, i) },
        );
    }
    state.board_layout.areas.insert(
        AreaId::new(4),
        AreaConfig { area_type: AreaType::Canal, grid_position: (0, 4) },
    );
    state.pending_choices.push(PendingChoice::StationPlace);
    let legal = get_legal_moves(&state, &rules);
    let placeable: Vec<AreaId> = state.board_layout.areas.iter()
        .filter(|(_, c)| c.area_type.allows_placement())
        .map(|(&a, _)| a).collect();
    assert_eq!(legal.len(), placeable.len(), "one ChooseStationPlace per placeable area, no Decline");
    let target = placeable[3];
    let next = apply_move(&state, &Move::ChooseStationPlace { area: target }, &rules).unwrap();
    assert_eq!(next.station_location, Some(target));
    assert!(next.pending_choices.is_empty());
}

// ===========================================================================
// Movement(2) -> 2 moves cascade (design §6.1-1's third named cascade;
// AssistantMove's own per-type scenario above uses a single icon, so this
// is the genuinely distinct "one trigger, N choices of the SAME type" shape).
// ===========================================================================

#[test]
fn order_movement_reward_of_two_queues_two_assistant_move_choices() {
    let (state, order_id, rules) =
        order_fixture(OrderRewards { movement: 2, ..Default::default() }, AreaType::Bank);
    let starting_hand = state.players[0].assistants.in_hand;

    let mid = apply_move(&state, &Move::FulfillOrder { order_id }, &rules).unwrap();
    assert_eq!(mid.pending_choices, vec![PendingChoice::AssistantMove, PendingChoice::AssistantMove]);

    let after_first = apply_move(
        &mid,
        &Move::ChooseAssistantMove { from: MoveSource::Hand, to: area0() },
        &rules,
    )
    .unwrap();
    assert_eq!(after_first.pending_choices, vec![PendingChoice::AssistantMove]);
    assert_eq!(after_first.players[0].assistants.on_board[&area0()], 1);

    let after_second = apply_move(&after_first, &Move::DeclineAssistantMove, &rules).unwrap();
    assert!(after_second.pending_choices.is_empty());
    assert_eq!(after_second.players[0].assistants.in_hand, starting_hand - 1); // only the first move consumed a hand assistant
}

// ===========================================================================
// Controller-assigned pin 1 (T3-m2): cross-type push order is
// warehouse -> any_good -> movement, fixed by source sequence
// (`apply_fulfill_order` / `apply_space_reward` both push in that exact,
// unconditional order regardless of icon counts).
// ===========================================================================

#[test]
fn cross_type_reward_push_order_is_warehouse_then_any_good_then_movement() {
    let (state, order_id, rules) = order_fixture(
        OrderRewards { warehouse_tokens: 1, any_trade_good: 1, movement: 1, ..Default::default() },
        AreaType::Bank,
    );

    let mid = apply_move(&state, &Move::FulfillOrder { order_id }, &rules).unwrap();

    // All three types come from ONE triggering move; the queue order must
    // match apply_fulfill_order's fixed source sequence exactly, not e.g.
    // alphabetical or field-declaration order (OrderRewards declares
    // movement before any_trade_good before warehouse_tokens -- the PUSH
    // order is independent of the struct's field order).
    assert_eq!(
        mid.pending_choices,
        vec![PendingChoice::WarehouseTake, PendingChoice::AnyGood, PendingChoice::AssistantMove]
    );
}

// ===========================================================================
// Controller-assigned pin 2 (T12 review rider): "generate_choice_resolutions_for
// unconditionally co-lists Decline -> push_choice singletons are always
// no-op declines" (progress.md, Task 12 review, assigned to T15).
//
// The exact, code-verified shape of that claim, variant by variant --
// narrower than "every variant always offers a Decline move" (which is
// FALSE for AnyGood/ChurchSpace/CustomsSpace: their `generate_choice_
// resolutions_for` arms have no Decline-named move at all, by design, and
// this is correct -- see the per-arm comments below). The real, load-bearing
// property is: no variant EVER silently auto-resolves to an active effect.
// Exhaustive match (no `_` arm) over PendingChoice, so adding a 7th variant
// forces a conscious decision here instead of silently passing.
// ===========================================================================

/// Dispatches to the correct per-variant assertion against `resolutions`
/// (the legal moves for a state where `head` is the pending-choice head).
fn assert_decline_or_never_singleton(head: &PendingChoice, resolutions: &[Move]) {
    match head {
        PendingChoice::WarehouseTake => {
            assert!(resolutions.contains(&Move::DeclineWarehouseTake), "WarehouseTake must always offer Decline");
        }
        PendingChoice::AssistantMove => {
            assert!(resolutions.contains(&Move::DeclineAssistantMove), "AssistantMove must always offer Decline");
        }
        PendingChoice::AnyGood => {
            // Mandatory ("gain 1 trade good of choice" -- not optional per
            // the rulebook): fixed at exactly 4 options always, so it can
            // never reach push_choice's singleton path. No Decline exists
            // or is needed (types/choices.rs's own doc comment on AnyGood).
            assert_eq!(resolutions.len(), 4, "AnyGood must always offer all 4 goods, never fewer");
        }
        PendingChoice::ChurchSpace { .. } | PendingChoice::CustomsSpace { .. } => {
            // Never goes through push_choice at all: tracks.rs pre-filters
            // 0/1/2+ eligible spaces itself and only ever calls
            // `pending_choices.push` on the 2+ branch -- a true singleton is
            // placed INLINE by the executor, never surfacing as a "choice"
            // that could silently auto-resolve. No Decline exists in this
            // resolution set (goods are already spent; declining after
            // paying isn't a rules option) -- verified here as "never fewer
            // than 2 real options when actually queued."
            assert!(resolutions.len() >= 2, "{head:?} is only ever queued with >=2 eligible spaces");
        }
        PendingChoice::RecoveryLeave => {
            // Gated by an explicit pre-push condition (Telephone owned AND
            // >=1 own assistant in the acting area), not push_choice's
            // generic singleton check -- always exactly 2 options, never a
            // singleton. leave_one:false (recover everyone -- the same
            // outcome as not using Telephone at all) is its inert
            // resolution, always present.
            assert!(
                resolutions.contains(&Move::ChooseRecoveryLeave { leave_one: false }),
                "RecoveryLeave must always offer its non-cascading leave_one:false resolution"
            );
        }
        PendingChoice::StationPlace => {
            // Unlike WarehouseTake/AssistantMove, a true singleton here would
            // NOT be an inert Decline -- Station has no Decline at all (design
            // §4.2: "the token must land"), so push_choice's generic singleton
            // path would silently PLACE the token when exactly 1 area is
            // placeable. Design §4.2 explicitly sanctions this ("if exactly
            // one eligible area exists, place inline") and the case is
            // real-board-unreachable (board_gen.rs's smallest grid is 10
            // tiles, minus at most 1 Canal). This assertion only pins ">=2
            // whenever actually queued" -- same scope as ChurchSpace/
            // CustomsSpace's arm above. The degenerate singleton-placement
            // case itself is NOT left untested: it has a direct precedent
            // test, same as ChurchSpace/CustomsSpace do (tracks.rs's
            // `church_with_single_eligible_space_places_inline_no_choice_event`
            // / `customs_with_one_pair_space_open_places_inline`) --
            // `game/choices.rs`'s
            // `station_place_with_single_placeable_area_places_inline_no_choice_event`
            // calls `push_choice` directly and asserts the token lands
            // unqueued.
            assert!(resolutions.len() >= 2, "StationPlace is only ever queued with >=2 placeable areas (real board)");
        }
    }
}

/// Task 9 becomes-singleton probe (design §4.4): `normalize_queue_head`
/// (called at the end of every `apply_move_unchecked`, right after the
/// `match mv` block resolves the head) must never leave a queue head whose
/// ONLY resolution is a Decline -- that is a dead question, and the whole
/// point of Task 9 is that it auto-resolves silently instead of prompting.
/// `get_legal_moves` is queue-first (returns exactly the head's resolution
/// set whenever a choice is pending) -- the same head-only view
/// `assert_decline_or_never_singleton` above relies on, so this is a no-op
/// (vacuously true) whenever the queue is empty.
fn assert_head_never_lone_decline(state: &GameState, rules: &Rules) {
    let Some(head) = state.pending_choices.first() else { return };
    let resolutions = get_legal_moves(state, rules);
    let lone_decline = resolutions.len() == 1
        && matches!(resolutions[0], Move::DeclineWarehouseTake | Move::DeclineAssistantMove);
    assert!(
        !lone_decline,
        "{head:?} head degraded to lone Decline without normalizing: {resolutions:?}"
    );
}

#[test]
fn push_choice_singletons_are_never_non_decline_effects() {
    let rules = Rules::empty_for_tests();

    // WarehouseTake and AssistantMove, queued normally (default player
    // stock is non-empty, so these are NOT singletons here) -- Decline is
    // present regardless.
    for head in [PendingChoice::WarehouseTake, PendingChoice::AssistantMove] {
        let mut state = create_test_game_state(2);
        state.pending_choices.push(head.clone());
        assert_decline_or_never_singleton(&head, &get_legal_moves(&state, &rules));
    }

    // AnyGood: always exactly 4, regardless of state.
    {
        let mut state = create_test_game_state(2);
        state.pending_choices.push(PendingChoice::AnyGood);
        assert_decline_or_never_singleton(&PendingChoice::AnyGood, &get_legal_moves(&state, &rules));
    }

    // ChurchSpace / CustomsSpace on a fresh track: always >=2 when queued.
    {
        let mut state = create_test_game_state(3);
        let head = PendingChoice::ChurchSpace { total_faith: 6 };
        state.pending_choices.push(head.clone());
        assert_decline_or_never_singleton(&head, &get_legal_moves(&state, &rules));
    }
    {
        let mut state = create_test_game_state(2);
        let head = PendingChoice::CustomsSpace { cost: 1 };
        state.pending_choices.push(head.clone());
        assert_decline_or_never_singleton(&head, &get_legal_moves(&state, &rules));
    }

    // RecoveryLeave: always exactly the 2 fixed options.
    {
        let mut state = create_test_game_state(2);
        state.pending_choices.push(PendingChoice::RecoveryLeave);
        assert_decline_or_never_singleton(&PendingChoice::RecoveryLeave, &get_legal_moves(&state, &rules));
    }

    // StationPlace on a 2-area board: always >=2 when queued.
    {
        let mut state = create_test_game_state(2);
        state.board_layout.areas.insert(AreaId::new(0), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        state.board_layout.areas.insert(AreaId::new(1), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 1) });
        let head = PendingChoice::StationPlace;
        state.pending_choices.push(head.clone());
        assert_decline_or_never_singleton(&head, &get_legal_moves(&state, &rules));
    }

    // The direct, stronger proof for the two variants that CAN actually
    // reach push_choice's singleton path: force zero real options and
    // confirm the auto-resolution is silently inert (queue drains, nothing
    // charged/moved) rather than a hidden active effect.
    {
        let mut state = create_test_game_state(2);
        state.players[0].assistants.in_warehouse = 0;
        state.players[0].shophouses.in_warehouse = 0;
        state.players[0].trading_houses.in_warehouse = 0;
        let before = state.players[0].clone();
        push_choice(&mut state, PendingChoice::WarehouseTake, &rules);
        assert!(state.pending_choices.is_empty(), "empty warehouse must auto-decline, not queue");
        assert_eq!(state.players[0], before, "auto-declined singleton must be a true no-op");
    }
    {
        let mut state = create_test_game_state(2);
        state.players[0].assistants.in_hand = 0; // on_board is already empty; board itself is empty too
        let before = state.players[0].clone();
        push_choice(&mut state, PendingChoice::AssistantMove, &rules);
        assert!(state.pending_choices.is_empty(), "no movable assistant must auto-decline, not queue");
        assert_eq!(state.players[0], before, "auto-declined singleton must be a true no-op");
    }

    // Task 9 becomes-singleton probe (design §4.4): two WarehouseTakes
    // queued back to back (direct `pending_choices.push`, bypassing
    // push_choice's own at-push-time singleton probe so BOTH start queued --
    // the shape `apply_space_reward`/`execute_customs_action` produce for a
    // 2-icon reward against unconsumed stock), warehouse stocked with
    // exactly ONE assistant (0 shophouses, 0 trading houses) so the first
    // resolution exhausts the only item either head could ever take -- the
    // SECOND, still-queued WarehouseTake's resolution set degrades from
    // [Assistant, Decline] to [Decline] alone. Resolving the first must not
    // leave that dead question sitting as the new head.
    {
        let mut state = create_test_game_state(2);
        state.players[0].assistants.in_warehouse = 1;
        state.players[0].shophouses.in_warehouse = 0; // else the SECOND head still offers Shophouse
        state.players[0].trading_houses.in_warehouse = 0; // ...and TradingHouse -- neither degenerate case fires
        state.pending_choices.push(PendingChoice::WarehouseTake);
        state.pending_choices.push(PendingChoice::WarehouseTake);
        let after = apply_move(
            &state,
            &Move::ChooseWarehouseTake { item: WarehouseItem::Assistant },
            &rules,
        )
        .unwrap();
        assert_head_never_lone_decline(&after, &rules);
    }
}

// ===========================================================================
// Task 9 (design §4.4): head-of-queue normalization -- a queued choice whose
// option set degrades to exactly [Decline] after an EARLIER resolution
// consumed the shared resource must auto-resolve instead of prompting a dead
// question.
// ===========================================================================

#[test]
fn a_queued_take_that_becomes_decline_only_auto_resolves_without_prompting() {
    // Fixture: warehouse has EXACTLY 1 assistant, 0 shophouses, 0 trading
    // houses -- so after the first take, nothing is left for the second.
    // Two WarehouseTakes queued back to back (the direct-push pattern from
    // `game::choices::tests::double_warehouse_choice_fixture`).
    let mut state = create_test_game_state(2);
    state.players[0].assistants.in_warehouse = 1;
    state.players[0].shophouses.in_warehouse = 0;
    state.players[0].trading_houses.in_warehouse = 0;
    state.pending_choices.push(PendingChoice::WarehouseTake);
    state.pending_choices.push(PendingChoice::WarehouseTake);
    let rules = Rules::empty_for_tests();
    let before = state.players[0].clone();

    // Resolve the first take (ChooseWarehouseTake for the assistant) -- this
    // empties the warehouse, so the second queued WarehouseTake's resolution
    // set collapses to [DeclineWarehouseTake] alone.
    let after = apply_move(
        &state,
        &Move::ChooseWarehouseTake { item: WarehouseItem::Assistant },
        &rules,
    )
    .unwrap();

    // EXPECTED post-normalization: the queue is EMPTY (the second take
    // became {Decline} and auto-resolved) and the player state shows
    // exactly one take's effect -- a true no-op for the second.
    assert!(
        after.pending_choices.is_empty(),
        "becomes-singleton head must auto-resolve, not prompt"
    );
    assert_eq!(after.players[0].assistants.in_hand, before.assistants.in_hand + 1);
    assert_eq!(after.players[0].assistants.in_warehouse, 0);
    // no second charge, no second token:
    assert_eq!(after.players[0].money, before.money);
}

// ===========================================================================
// Undo-equivalence: record a cascade, truncate the last event, replay --
// must equal the state manually stopped right before that event.
// ===========================================================================

#[test]
fn undo_truncating_last_event_replays_to_the_state_before_that_choice() {
    let (initial, order_id, rules) = order_fixture(
        OrderRewards { warehouse_tokens: 1, any_trade_good: 1, ..Default::default() },
        AreaType::Bank,
    );

    let events = vec![
        RecordedEvent::Move(Move::FulfillOrder { order_id }),
        RecordedEvent::Move(Move::ChooseWarehouseTake { item: WarehouseItem::Assistant }),
        RecordedEvent::Move(Move::ChooseAnyGood { good: TradeGood::Tea }),
    ];

    let full = replay_from(&initial, &events, &rules).unwrap();
    assert!(full.pending_choices.is_empty());

    // "Undo" the last event (drop it, replay the rest).
    let undone = replay_from(&initial, &events[..events.len() - 1], &rules).unwrap();
    assert_eq!(undone.pending_choices, vec![PendingChoice::AnyGood]);

    // Must equal manually stopping right there via direct apply_move calls.
    let step1 = apply_move(&initial, &Move::FulfillOrder { order_id }, &rules).unwrap();
    let step2 =
        apply_move(&step1, &Move::ChooseWarehouseTake { item: WarehouseItem::Assistant }, &rules).unwrap();
    assert_eq!(undone, step2);
}

// ===========================================================================
// Mid-cascade save/load: serialize a GameRecord whose event log stops
// mid-cascade, round-trip through JSON, replay, and confirm the resumed
// state still has the pending choice queued with its prompts intact.
//
// `initial` here is a crafted fixture, not a `new_game`+`finalize_setup`
// output, so reconstruction uses `replay_from(&initial, &events, &rules)`
// rather than the top-level `replay(&record, &rules)` (which always starts
// from `new_game(record.seed)` -- record.rs's own doc comment calls out
// exactly this non-from-scratch case, with the CLI's `Session::undo`
// checkpoint path as the motivating precedent; this test's "save point" is
// the same shape, just for save/load instead of undo).
// ===========================================================================

#[test]
fn mid_cascade_game_record_round_trips_through_json_and_resumes() {
    let (initial, order_id, rules) = order_fixture(
        OrderRewards { warehouse_tokens: 1, any_trade_good: 1, ..Default::default() },
        AreaType::Bank,
    );

    let mut record = GameRecord::new(
        0,
        initial.config.clone(),
        vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
        &rules,
    );
    record.push_move(Move::FulfillOrder { order_id });
    record.push_move(Move::ChooseWarehouseTake { item: WarehouseItem::Assistant });
    // AnyGood is still pending here -- this is the mid-cascade save point.

    let json = serde_json::to_string(&record).expect("GameRecord must serialize");
    let reloaded: GameRecord = serde_json::from_str(&json).expect("GameRecord must deserialize");
    assert_eq!(reloaded, record, "JSON round-trip must be lossless");

    let resumed = replay_from(&initial, &reloaded.events, &rules).expect("mid-cascade replay");
    assert_eq!(resumed.pending_choices, vec![PendingChoice::AnyGood], "pending choice must survive save/load");
    assert_eq!(
        get_legal_moves(&resumed, &rules),
        vec![
            Move::ChooseAnyGood { good: TradeGood::Copper },
            Move::ChooseAnyGood { good: TradeGood::Silk },
            Move::ChooseAnyGood { good: TradeGood::Tea },
            Move::ChooseAnyGood { good: TradeGood::Seafood },
        ],
        "prompts must resume exactly where they left off"
    );
}

// ===========================================================================
// Organic-coverage census (design §6.1-1's second deliverable): proves the
// random self-play driver itself -- not just these hand-crafted fixtures --
// actually exercises every PendingChoice variant and every choice-resolution
// Move at least once.
// ===========================================================================

/// Total games this census will play before giving up. Empirically measured
/// (Task 15 investigation: a throwaway `cargo run --example probe_census`,
/// deleted before commit, using the exact `run_random_game_observed`
/// function below with `players` cycling 2p/3p/4p by `seed % 3` and
/// `Rules::standard()`): EVERY PendingChoice variant and EVERY choice Move
/// (including both `ChooseRecoveryLeave` states) fires organically by seed
/// 222. AnyGood is the slow one -- first at seed 222, matching the T11
/// review's independent finding ("AnyGood reachability... all 36 orders
/// have any_trade_good=0; Shophouse-build reward is the practical source;
/// ~2/400 random seeds reach a cascade", progress.md) -- NOT RecoveryLeave
/// as this task's brief anticipated (Telephone turned out easy: first seen
/// at seed 3, since games run 500+ turns with ample Laboratory visits).
/// 300 gives ~35% headroom over the observed worst case; the early return
/// below keeps the actual run far shorter in practice (measured wall time
/// is in this task's report).
///
/// Engine-authority Task 5 (design 2026-07-19 §4.2) added `StationPlace`
/// using the same throwaway-probe method: first seen at seed 3 (2p) --
/// another easy one, like Telephone, since a Laboratory acquisition is a
/// common random-play move and Station is just 1 of 20 tech titles in
/// circulation. 300 stayed unchanged there; AnyGood (seed 222) remained the
/// binding constraint.
///
/// Engine-authority Task 7 (Option B recovery counts encoding) re-measured
/// this with the same throwaway-probe method after enumerating Option B
/// retreat as the full per-area cartesian product (I5 ruling): AnyGood is
/// STILL the binding constraint and every other variant's first-seen seed
/// is unchanged or earlier, but the flat move-list random walk now spends
/// much more of its probability mass on `ReturnPresidentToHand` whenever a
/// player's assistants are scattered across several areas (see
/// `tests/common/mod.rs`'s `MAX_TURNS` doc comment for the same root
/// cause), pushing AnyGood's organic first-seed from 222 to 317. 430 gives
/// ~35% headroom over that new observed worst case, matching this
/// constant's original headroom policy.
const CENSUS_SEED_CAP: u64 = 430;

/// Every `PendingChoice` variant's census tag. Exhaustive match (no `_`
/// arm): a newly-added variant fails to compile here instead of silently
/// never being required by the census.
fn variant_name(choice: &PendingChoice) -> &'static str {
    match choice {
        PendingChoice::AnyGood => "AnyGood",
        PendingChoice::WarehouseTake => "WarehouseTake",
        PendingChoice::AssistantMove => "AssistantMove",
        PendingChoice::ChurchSpace { .. } => "ChurchSpace",
        PendingChoice::CustomsSpace { .. } => "CustomsSpace",
        PendingChoice::RecoveryLeave => "RecoveryLeave",
        PendingChoice::StationPlace => "StationPlace",
    }
}

/// Every choice-resolution `Move`'s census tag; `None` for the rest of the
/// (much larger) `Move` enum, i.e. ordinary play. `ChooseRecoveryLeave`'s
/// two booleans are split into distinct tags since the task brief
/// specifically wants leave_one:true tracked separately from leave_one:false
/// (the former is the rarer, cascading-adjacent resolution). Exhaustive
/// match over the WHOLE `Move` enum (no `_` arm): a newly added Move variant
/// forces a conscious choice-or-not decision here.
fn choice_move_name(mv: &Move) -> Option<&'static str> {
    match mv {
        Move::ChooseAnyGood { .. } => Some("ChooseAnyGood"),
        Move::ChooseWarehouseTake { .. } => Some("ChooseWarehouseTake"),
        Move::DeclineWarehouseTake => Some("DeclineWarehouseTake"),
        Move::ChooseAssistantMove { .. } => Some("ChooseAssistantMove"),
        Move::DeclineAssistantMove => Some("DeclineAssistantMove"),
        Move::ChooseChurchSpace { .. } => Some("ChooseChurchSpace"),
        Move::ChooseCustomsSpace { .. } => Some("ChooseCustomsSpace"),
        Move::ChooseRecoveryLeave { leave_one: false } => Some("ChooseRecoveryLeave(false)"),
        Move::ChooseRecoveryLeave { leave_one: true } => Some("ChooseRecoveryLeave(true)"),
        Move::ChooseStationPlace { .. } => Some("ChooseStationPlace"),
        Move::UseMercenary { .. }
        | Move::UseUniversity
        | Move::UseSpinningMill
        | Move::FulfillAchievement { .. }
        | Move::FulfillOrder { .. }
        | Move::EndAdditionalPhase
        | Move::PlaceThreeAreas { .. }
        | Move::PlaceTwoSameArea { .. }
        | Move::MovePresident { .. }
        | Move::ReturnPresidentToHand { .. }
        | Move::PlacePresident { .. }
        | Move::ExecuteAreaAction { .. }
        | Move::Take5PowerToken
        | Move::Skip5PowerBonus
        | Move::BuildShophouse { .. }
        | Move::BuildTradingHouse
        | Move::SkipBuilding => None,
    }
}

const ALL_PENDING: [&str; 7] =
    ["AnyGood", "WarehouseTake", "AssistantMove", "ChurchSpace", "CustomsSpace", "RecoveryLeave", "StationPlace"];
const ALL_CHOICE_MOVES: [&str; 10] = [
    "ChooseAnyGood",
    "ChooseWarehouseTake",
    "DeclineWarehouseTake",
    "ChooseAssistantMove",
    "DeclineAssistantMove",
    "ChooseChurchSpace",
    "ChooseCustomsSpace",
    "ChooseRecoveryLeave(false)",
    "ChooseRecoveryLeave(true)",
    "ChooseStationPlace",
];

fn all_variants_seen(seen_pending: &BTreeSet<&'static str>, seen_moves: &BTreeSet<&'static str>) -> bool {
    ALL_PENDING.iter().all(|v| seen_pending.contains(v))
        && ALL_CHOICE_MOVES.iter().all(|v| seen_moves.contains(v))
}

/// The variant/move names NOT yet seen, for the panic message.
fn missing(
    seen_pending: &BTreeSet<&'static str>,
    seen_moves: &BTreeSet<&'static str>,
) -> (Vec<&'static str>, Vec<&'static str>) {
    (
        ALL_PENDING.iter().copied().filter(|v| !seen_pending.contains(v)).collect(),
        ALL_CHOICE_MOVES.iter().copied().filter(|v| !seen_moves.contains(v)).collect(),
    )
}

#[test]
fn every_choice_variant_fires_organically_in_random_play() {
    let rules = Rules::standard();
    let mut seen_pending: BTreeSet<&'static str> = BTreeSet::new();
    let mut seen_moves: BTreeSet<&'static str> = BTreeSet::new();

    for seed in 0..CENSUS_SEED_CAP {
        let players = 2u8 + (seed % 3) as u8; // cycle 2p/3p/4p for full board-rule coverage
        run_random_game_observed(seed, players, &rules, |state, mv| {
            if let Some(head) = state.pending_choices.first() {
                seen_pending.insert(variant_name(head));
            }
            if let Some(name) = choice_move_name(mv) {
                seen_moves.insert(name);
            }
        })
        .unwrap_or_else(|e| panic!("seed={seed} players={players}p: {e}"));

        if all_variants_seen(&seen_pending, &seen_moves) {
            return;
        }
    }

    let (missing_pending, missing_moves) = missing(&seen_pending, &seen_moves);
    panic!(
        "unreached after {CENSUS_SEED_CAP} games: missing pending {missing_pending:?}, missing moves {missing_moves:?}"
    );
}
