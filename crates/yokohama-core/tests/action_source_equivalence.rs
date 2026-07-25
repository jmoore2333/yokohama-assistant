//! Phase 3.1a Task 5: per-family factored↔exhaustive equivalence, TWO TIERS.
//!
//! Proves the whole point of the factored action source (`game/search_actions.rs`):
//! the UNION of the move families [`families`] lists — each served by
//! [`iter_family`] — reconstructs the frozen exhaustive [`get_legal_moves`]
//! surface EXACTLY, as a multiset, at every reachable decision point. The
//! factored source pages/samples the six-figure retreat family without
//! materializing it, but the moves it can serve must still be precisely the
//! exhaustive ones — no more (soundness), no fewer (completeness).
//!
//! This lives in `yokohama-core` and compares the factored surface against the
//! engine's own `get_legal_moves`, NOT via the oracle. It reuses the
//! difftest `compare.rs` SHAPE (a Debug-string key + a multiset diff) but with a
//! crate-local, oracle-free key: both sides here are ENGINE-emitted (either
//! `get_legal_moves` itself or `iter_family`, which for the delegated families
//! filters `get_legal_moves` and for retreat/placement reproduces the generator
//! move-for-move), and the engine already emits every order-insensitive payload
//! in a canonical order (`compare.rs`), so a plain Debug key is a sound multiset
//! key AND stricter than `compare.rs`'s canonicalizing one: an accidental
//! within-payload reorder in the factored source surfaces as a divergence here
//! rather than being normalized away.
//!
//! TWO TIERS (CLAUDE.md two-tier policy):
//! - PER-EDIT (not ignored): the driven-game union check at seeds 0..5 × 2/3/4P
//!   (capped), plus the constructed adversarial fixtures — the retreat 442,368
//!   worst case (checked by COUNT + spot-checks, never a second 442k
//!   collection), the ≥ C(17,3) placement family, and the non-sorted-hand
//!   FulfillOrder order round-trip.
//! - RELEASE (`#[ignore]`d): the same union check swept at seeds 0..20 × 2/3/4P,
//!   deeper into each game — the distributional regression net.
//!
//! `MovementRoutes` is still exhaustive-backed (a 3.1b stub that delegates to
//! `get_legal_moves`), so the union stays COMPLETE here; 3.1b replaces it with a
//! lazy route enumerator and this test keeps holding.

mod common;

use std::collections::BTreeMap;

use rand::Rng;

use yokohama_core::{
    apply_move_unchecked, families, family_len, get_legal_moves, is_game_over, iter_family,
    AchievementCard, AchievementType, AreaConfig, AreaId, AreaType, BoardLayout, ChurchTrack,
    CustomsTrack, GameConfig, GameState, LabState, MercenarySupply, Move, MoveFamily, OrderCardId,
    PlayerColor, PlayerId, PlayerState, PortState, PresidentLocation, Rules, TechEffect, TurnPhase,
};

// ---------------------------------------------------------------------------
// Crate-local, oracle-free move-key multiset compare (the `compare.rs` SHAPE).
// ---------------------------------------------------------------------------

/// Debug-string multiset of `moves`. See the module header for why a plain
/// Debug key (no canonicalization) is both sound and deliberately stricter than
/// `compare.rs`'s canonicalizing key for this engine-vs-engine comparison.
fn key_counts(moves: &[Move]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for m in moves {
        *counts.entry(format!("{m:?}")).or_insert(0usize) += 1;
    }
    counts
}

/// `None` when `a` and `b` are equal as multisets; otherwise a compact report of
/// only the keys whose counts differ (so a failure at a large state stays
/// readable rather than dumping two full lists).
fn multiset_diff(a: &[Move], b: &[Move]) -> Option<String> {
    let (ca, cb) = (key_counts(a), key_counts(b));
    if ca == cb {
        return None;
    }
    let mut keys: Vec<&String> = ca.keys().chain(cb.keys()).collect();
    keys.sort_unstable();
    keys.dedup();
    let lines: Vec<String> = keys
        .into_iter()
        .filter_map(|k| {
            let (na, nb) = (ca.get(k).copied().unwrap_or(0), cb.get(k).copied().unwrap_or(0));
            (na != nb).then(|| format!("  {k}: union={na} legal={nb}"))
        })
        .collect();
    Some(format!("multiset mismatch ({} key(s) differ):\n{}", lines.len(), lines.join("\n")))
}

// ---------------------------------------------------------------------------
// Driven-game union sweep (shared by both tiers).
// ---------------------------------------------------------------------------

/// Outcome of a sweep, for the caller's anti-vacuity assertions.
struct SweepStats {
    /// Total decision points visited.
    decision_points: usize,
    /// Decision points that took the strong string-multiset union check (all of
    /// them, organically — see [`UNION_MATERIALIZE_CAP`]).
    full_checks: usize,
    /// Decision points whose exhaustive list exceeded the cap and fell back to
    /// the cheap no-materialization invariant. Expected 0 organically.
    guard_trips: usize,
}

/// Only string-key the union when the exhaustive list is small enough that a
/// second pass is cheap. Organic random play stays far below this (the Task-4
/// unit test collects `iter_family` for every family every step at these same
/// seeds without trouble); the constructed retreat worst case exercises the
/// exploding path by COUNT instead, in its own test.
const UNION_MATERIALIZE_CAP: usize = 50_000;

/// Drive uniform-random games (via the shared `common` seeded driver) and, at
/// every decision point, assert `⋃_family iter_family(state, family)` equals
/// `get_legal_moves(state)` as a MULTISET, plus the per-family `family_len`
/// contract (exact for every family except Placement, whose `family_len` is an
/// honest upper bound ≥ the realized count).
fn run_union_sweep(rules: &Rules, seeds: std::ops::Range<u64>, cap: usize) -> SweepStats {
    let mut stats = SweepStats { decision_points: 0, full_checks: 0, guard_trips: 0 };

    for players in [2u8, 3, 4] {
        for seed in seeds.clone() {
            let (mut state, mut rng) = common::new_seeded_game(seed, players, rules);

            for _ in 0..cap {
                if is_game_over(&state) {
                    break;
                }
                let legal = get_legal_moves(&state, rules);
                if legal.is_empty() {
                    break;
                }
                let fams = families(&state, rules);

                if legal.len() <= UNION_MATERIALIZE_CAP {
                    // Strong check: materialize each family, pin its `family_len`
                    // contract, and assert the concatenated union == legal.
                    let mut union: Vec<Move> = Vec::with_capacity(legal.len());
                    for family in &fams {
                        let got: Vec<Move> = iter_family(&state, rules, family).collect();
                        let len = family_len(&state, rules, family);
                        if matches!(family, MoveFamily::Placement) {
                            assert!(
                                len >= got.len(),
                                "placement family_len {len} < realized {} at {:?} ({players}p seed {seed})",
                                got.len(),
                                state.turn_phase,
                            );
                        } else {
                            assert_eq!(
                                len,
                                got.len(),
                                "family_len {len} != realized {} for {family:?} ({players}p seed {seed})",
                                got.len(),
                            );
                        }
                        union.extend(got);
                    }
                    if let Some(diff) = multiset_diff(&union, &legal) {
                        panic!(
                            "factored union != get_legal_moves at {:?} ({players}p seed {seed} turn {}):\n{diff}",
                            state.turn_phase, state.turn_number,
                        );
                    }
                    stats.full_checks += 1;
                } else {
                    // Fallback for an (organically-unreachable) exploding state:
                    // a cheap no-materialization invariant — Σ family_len is an
                    // upper bound on |legal| (exact except placement's honest
                    // over-estimate), and holds even at a 442k retreat state
                    // without collecting the family.
                    let sum_len: usize = fams.iter().map(|f| family_len(&state, rules, f)).sum();
                    assert!(
                        sum_len >= legal.len(),
                        "Σ family_len {sum_len} < |legal| {} at {:?} ({players}p seed {seed})",
                        legal.len(),
                        state.turn_phase,
                    );
                    stats.guard_trips += 1;
                }

                stats.decision_points += 1;
                let mv = legal[rng.gen_range(0..legal.len())].clone();
                state = apply_move_unchecked(&state, &mv, rules).expect("apply drawn move");
            }
        }
    }
    stats
}

/// PER-EDIT union equivalence: at every organically-reached decision point
/// across seeds 0..5 × 2/3/4P (capped), the union of the factored families
/// reproduces `get_legal_moves` exactly as a multiset, and every `family_len`
/// holds. Organic play never explodes a family past the cap, so every decision
/// point takes the strong string-multiset check.
#[test]
fn factored_union_equals_get_legal_moves_across_driven_games() {
    let rules = Rules::standard();
    // Cap tuned for a snappy per-edit gate: 800 steps × 5 seeds × 3 player
    // counts still reaches every phase and thousands of decision points; the
    // release sweep below is the deep distributional net.
    let stats = run_union_sweep(&rules, 0..5, 800);

    assert!(stats.decision_points > 500, "expected many decision points, saw {}", stats.decision_points);
    assert_eq!(
        stats.guard_trips, 0,
        "no organic decision point should exceed the materialize cap ({} did — raise the cap or investigate)",
        stats.guard_trips,
    );
    assert_eq!(
        stats.full_checks, stats.decision_points,
        "every organic decision point should take the strong multiset check ({} of {})",
        stats.full_checks, stats.decision_points,
    );
}

// ---------------------------------------------------------------------------
// Constructed adversarial fixtures (per-edit anti-vacuity).
// ---------------------------------------------------------------------------

/// A minimal, type-valid state at `phase` with `n_areas` placement-allowing
/// (`CopperMine`) areas ids `0..n_areas`, `players` players (current = 0), and
/// empty adjacency/decks/queue — the same hand-built shape the `search_actions`
/// unit tests and `routes3_worstcase.rs` use (type-valid via public
/// constructors, not setup-valid; both `get_legal_moves` and the factored source
/// are total over it). Callers set the per-fixture knobs afterwards.
fn state_with_areas(players: u8, phase: TurnPhase, n_areas: u8) -> GameState {
    let colors = [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green];
    let mut areas = BTreeMap::new();
    for i in 0..n_areas {
        areas.insert(AreaId::new(i), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, i) });
    }
    GameState {
        config: GameConfig::for_player_count(players),
        board_layout: BoardLayout {
            areas,
            adjacency: BTreeMap::new(),
            canal_crossings: vec![],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        },
        players: (0..players)
            .map(|i| PlayerState::new(PlayerId::new(i), colors[i as usize], i == 0))
            .collect(),
        current_player: PlayerId::new(0),
        turn_phase: phase,
        turn_number: 1,
        additional_action_state: Default::default(),
        current_action_power: None,
        is_mercenary_action: false,
        mercenary_target_area: None,
        mercenary_return_phase: None,
        president_start_area: None,
        port_state: PortState::new(false),
        lab_state: LabState::new(false),
        order_deck: Vec::new(),
        tech_deck: Vec::new(),
        orders_removed: Vec::new(),
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

/// C(17,3) = 17·16·15 / 6 = 680. The placement anti-vacuity floor.
const C_17_3: usize = 680;

/// ANTI-VACUITY (a), retreat: the exploding retreat family exercised at
/// tens-of-thousands scale — the 4P worst case (three count-2 + fourteen count-1
/// over 17 areas, president ON BOARD) at exactly 442,368 members. The union is
/// checked by COUNT + spot-checks, NEVER by collecting the 442k factored side a
/// second time:
///  - `families()` emits the `MovementRetreat` tag carrying the exact 17-area
///    `on_board` snapshot; its `family_len` is the closed-form product 442,368
///    (computed without materializing anything);
///  - the exhaustive reference is consulted ONCE (it materializes the 442k
///    ceiling internally, ~46ms release / ~0.4s debug — the ceiling the factored
///    source exists to avoid): its `ReturnPresidentToHand` submultiset count ==
///    `family_len`, and the whole legal list partitions into exactly the two
///    Movement families, so the union is complete by count;
///  - landmark `iter_family().nth()` members (served lazily, no materialization)
///    are present in the exhaustive list.
#[test]
fn retreat_worst_case_442368_union_matches_by_count() {
    let rules = Rules::standard();

    let mut on_board: BTreeMap<AreaId, u8> = BTreeMap::new();
    for a in 0..3u8 {
        on_board.insert(AreaId::new(a), 2);
    }
    for a in 3..17u8 {
        on_board.insert(AreaId::new(a), 1);
    }

    let mut state = state_with_areas(4, TurnPhase::Movement, 17);
    state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
    state.players[0].assistants.on_board = on_board.clone();

    // families() emits the retreat tag with the exact snapshot (president on
    // board ⇒ from_hand = false), alongside the routes family.
    let fams = families(&state, &rules);
    let retreat = fams
        .iter()
        .find(|f| matches!(f, MoveFamily::MovementRetreat { .. }))
        .expect("Movement dispatches a retreat family");
    assert_eq!(
        *retreat,
        MoveFamily::MovementRetreat { on_board: on_board.clone(), from_hand: false },
        "retreat tag must carry the exact 17-area on_board snapshot",
    );

    // Closed-form product len — the six-figure ceiling, without materializing.
    let n = family_len(&state, &rules, retreat);
    assert_eq!(n, 442_368, "retreat family_len must be the 3^3 · 2^14 product");
    assert!(n >= 10_000, "retreat family must be exercised at tens-of-thousands scale");

    // Bounded union check: consult the exhaustive reference ONCE, compare by
    // COUNT + partition — never a second collection of the factored side.
    let legal = get_legal_moves(&state, &rules);
    let retreat_in_legal =
        legal.iter().filter(|m| matches!(m, Move::ReturnPresidentToHand { .. })).count();
    assert_eq!(
        retreat_in_legal, n,
        "get_legal_moves' ReturnPresidentToHand submultiset count must equal family_len",
    );
    let routes_in_legal = legal
        .iter()
        .filter(|m| matches!(m, Move::MovePresident { .. } | Move::PlacePresident { .. }))
        .count();
    assert_eq!(
        retreat_in_legal + routes_in_legal,
        legal.len(),
        "the whole legal list must partition into exactly the two Movement families",
    );

    // Spot-checks, served lazily (O(1) `nth`, no materialization): first =
    // recover-nothing, last = recover-everything (== the on_board map), a deep
    // interior index — and each is present in the exhaustive list.
    let first = iter_family(&state, &rules, retreat).next().expect("retreat family is non-empty");
    assert_eq!(first, Move::ReturnPresidentToHand { recover: BTreeMap::new() });
    let last = iter_family(&state, &rules, retreat).nth(n - 1).expect("last index in range");
    assert_eq!(last, Move::ReturnPresidentToHand { recover: on_board.clone() });
    let mid = iter_family(&state, &rules, retreat).nth(400_000).expect("interior index in range");
    for probe in [&first, &mid, &last] {
        assert!(legal.contains(probe), "spot-checked retreat move missing from the exhaustive list: {probe:?}");
    }
}

/// ANTI-VACUITY (a), placement: a placement family FAR above C(17,3) — 17
/// placement-allowing areas, Newspaper (k up to 4), in_hand 4, no fees ⇒
/// `family_len` = Σ_{k=0}^{4} C(17,k) + 17 Option-B basics = 3231 (the plan's
/// Test-sizing fixture), well past C(17,3)=680 and exercising the k=4
/// combination construction at integration scale. The union matches
/// `get_legal_moves` exactly (fully affordable ⇒ the honest upper bound is exact
/// and the realized set materializes cheaply).
#[test]
fn placement_family_at_scale_matches_and_exceeds_c_17_3() {
    let rules = Rules::standard();
    let newspaper = rules
        .techs
        .iter()
        .find(|t| t.effect == TechEffect::Newspaper)
        .expect("standard data carries Newspaper")
        .id;
    let mut state = state_with_areas(4, TurnPhase::Placement, 17);
    state.players[0].assistants.in_hand = 4;
    state.players[0].money = 1_000;
    state.players[0].technologies = vec![newspaper];

    assert_eq!(
        families(&state, &rules),
        vec![MoveFamily::Placement],
        "a placement state dispatches to exactly [Placement]",
    );

    let placement = MoveFamily::Placement;
    let len = family_len(&state, &rules, &placement);
    assert!(len >= C_17_3, "placement family_len {len} must exceed C(17,3)={C_17_3} (exercised at scale)");

    let got: Vec<Move> = iter_family(&state, &rules, &placement).collect();
    let legal = get_legal_moves(&state, &rules);
    if let Some(diff) = multiset_diff(&got, &legal) {
        panic!("placement union != get_legal_moves:\n{diff}");
    }
    assert_eq!(len, got.len(), "no fees ⇒ the placement upper bound is exact here");
    assert!(
        got.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements } if placements.len() == 4)),
        "a 4-area placement must be present at in_hand 4 with Newspaper (k reaches 4)",
    );
}

/// ANTI-VACUITY (b): the FulfillOrder subsequence preserves `order_cards_in_hand`
/// INSERTION order, not a sorted key. Hand [id 2, id 1] with abundant goods
/// yields FulfillOrder(2) then FulfillOrder(1); a sorted-by-id factored impl
/// would emit [1, 2] and FAIL this. The delegate-and-filter dispatch inherits
/// the generator's Vec order for free — this pins that it stays that way.
#[test]
fn fulfill_order_subsequence_preserves_nonsorted_hand_order() {
    let rules = Rules::standard();
    let mut state = state_with_areas(2, TurnPhase::AdditionalActionFirst, 2);
    // No assistant in hand ⇒ no achievement moves interleaved; abundant goods so
    // both orders are fulfillable regardless of their printed requirements.
    state.players[0].assistants.in_hand = 0;
    state.players[0].copper = 99;
    state.players[0].silk = 99;
    state.players[0].tea = 99;
    state.players[0].seafood = 99;
    // NON-sorted hand: id 2 before id 1 (a sorted key would emit 1 then 2).
    state.players[0].order_cards_in_hand = vec![OrderCardId(2), OrderCardId(1)];

    let family = MoveFamily::AdditionalAction;
    let got: Vec<Move> = iter_family(&state, &rules, &family).collect();

    // At this state the whole legal list is this one family.
    if let Some(diff) = multiset_diff(&got, &get_legal_moves(&state, &rules)) {
        panic!("additional-action union != get_legal_moves:\n{diff}");
    }

    let order_ids: Vec<u8> = got
        .iter()
        .filter_map(|m| match m {
            Move::FulfillOrder { order_id } => Some(order_id.0),
            _ => None,
        })
        .collect();
    assert_eq!(
        order_ids,
        vec![2, 1],
        "FulfillOrder must follow order_cards_in_hand insertion order, not a sorted key",
    );

    assert_eq!(
        family_len(&state, &rules, &family),
        got.len(),
        "family_len == realized additional-action count",
    );
}

// ---------------------------------------------------------------------------
// Release tier.
// ---------------------------------------------------------------------------

/// RELEASE-GATE union sweep (CLAUDE.md two-tier policy): the same per-decision
/// union-equivalence property as the per-edit driven test, swept broader (seeds
/// 0..20 × 2/3/4P) and deeper into each game — the distributional regression net
/// that need not re-run on every edit.
#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): union-equivalence sweep at seeds 0..20 x 2/3/4P deep into each game; run with cargo test --release -p yokohama-core --test action_source_equivalence -- --ignored before merges/phase closes"]
fn release_union_sweep_seeds_0_20() {
    let rules = Rules::standard();
    let stats = run_union_sweep(&rules, 0..20, 8000);

    assert!(stats.decision_points > 5000, "expected a large sweep, saw {}", stats.decision_points);
    assert_eq!(stats.guard_trips, 0, "no organic state should exceed the materialize cap ({} did)", stats.guard_trips);
    assert_eq!(
        stats.full_checks, stats.decision_points,
        "every decision point should take the strong multiset check ({} of {})",
        stats.full_checks, stats.decision_points,
    );
    println!(
        "release union sweep: {} decision points, all full-multiset-checked against get_legal_moves",
        stats.decision_points,
    );
}
