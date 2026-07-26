//! Phase 3.1a Task 5 (+ Phase 3.1b Task 3): per-family factored↔exhaustive
//! equivalence, TWO TIERS.
//!
//! Proves the whole point of the factored action source (`game/search_actions.rs`):
//! the UNION of the move families [`families`] lists — each served by
//! [`iter_family`] — reconstructs the frozen exhaustive [`get_legal_moves`]
//! surface EXACTLY at every reachable decision point. The factored source
//! pages/samples the six-figure retreat family without materializing it and
//! computes the routes family from a separate enumerator, but the moves it can
//! serve must still be precisely the exhaustive ones — no more (soundness), no
//! fewer (completeness), and since 3.1b in the same ORDER.
//!
//! TWO CHECKS, NEITHER IMPLYING THE OTHER'S DIAGNOSTIC (3.1b Task 3, spec §4-1a).
//! Every decision point takes both, in this order:
//!  1. MEMBERSHIP — the string-multiset diff ([`multiset_diff`]), which names the
//!     keys whose counts differ. It is order-blind by construction: a factored
//!     source that emitted the right moves in the wrong order would pass it
//!     silently, which is exactly the bug class 3.1b's `cmp_label` port can
//!     introduce.
//!  2. ORDER — elementwise [`Vec<Move>`] equality ([`first_divergence`]), for the
//!     `MovementRoutes` family against the `MovePresident`/`PlacePresident`
//!     subsequence of the legal list (the 3.1b contract) and then for the whole
//!     concatenated union, since [`families`] emits its tags in the order
//!     `get_legal_moves` emits their moves. It reports the first differing index,
//!     which a membership diff cannot, and cannot report a missing move as
//!     anything but a divergence at wherever the two lists first slip.
//!
//! Check 1 is retained deliberately: on a membership failure its report is the
//! useful one (WHICH moves appeared or vanished), while check 2 would only say
//! "index 137 differs". Neither is the other's fallback — they fail with
//! different information about different bug classes.
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
//! - PER-EDIT (not ignored): the driven-game union+order check at seeds 0..5 ×
//!   2/3/4P (capped), plus the constructed adversarial fixtures — the retreat
//!   442,368 worst case (checked by COUNT + spot-checks, never a second 442k
//!   collection), the placement family hard-pinned at its full 3,231 ceiling,
//!   the non-sorted-hand FulfillOrder order round-trip, and the 3.1b routes
//!   cells (Station suppression fires / a tolled walk and a free teleport both
//!   survive / Tram to a walk-unreachable area / from-hand `PlacePresident` /
//!   oversized-board delegation). The ROUTES cells are all ≤ 10 areas so the
//!   enumerator's lex-drift `debug_assert` stays cheap per-edit; the 17-area
//!   retreat/placement fixtures above predate that constraint and never touch
//!   the enumerator.
//! - RELEASE (`#[ignore]`d): the same sweep at seeds 0..20 × 2/3/4P deeper into
//!   each game — the distributional regression net — plus the 3.1b DENSE
//!   adversarial route cells, which are release-tier because each pays a full
//!   frozen-reference enumeration on a money-saturated 17-area staggered board.
//!
//! `MovementRoutes` is no longer the exhaustive-backed 3.1b stub: it is served
//! by `game/route_enum.rs`'s enumerator, which is why the ordered check above
//! exists and why the dense cells below stress the enumerator at hundreds of
//! labels rather than at organic play's tens.

mod common;
mod route_fixtures;

use std::collections::BTreeMap;

use rand::Rng;

use yokohama_core::{
    apply_move_unchecked, can_teleport_to_station_with_tech, enumerate_routes, families, family_len,
    fits_fixed_arrays, get_legal_moves, is_game_over, iter_family, AchievementCard, AchievementType,
    AreaConfig, AreaId, AreaType, BoardLayout, ChurchTrack, CustomsTrack, GameConfig, GameState,
    LabState, MercenarySupply, Move, MoveFamily, OrderCardId, PlayerColor, PlayerId, PlayerState,
    PortState, PresidentLocation, Rules, TechCardId, TechEffect, TurnPhase,
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

/// `None` when `got` and `want` are equal ELEMENT FOR ELEMENT; otherwise the
/// first index at which they differ, with both entries (or the length mismatch
/// when one is a prefix of the other) — so a failure at a several-hundred-move
/// Movement family stays as readable as one at a four-move fixture.
///
/// The order gate (3.1b Task 3). `multiset_diff` above cannot see a pure
/// permutation, and a permutation is precisely what a wrong `cmp_label`
/// (`route_enum.rs`'s ported comparator) or a mis-sequenced family assembly
/// produces: same moves, wrong order, silently passing every membership check.
fn first_divergence(got: &[Move], want: &[Move]) -> Option<String> {
    if got == want {
        return None;
    }
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        if g != w {
            return Some(format!(
                "order mismatch at index {i} of {}/{}:\n  factored: {g:?}\n  legal:    {w:?}",
                got.len(),
                want.len(),
            ));
        }
    }
    Some(format!(
        "one list is a prefix of the other: factored {} moves, legal {} moves",
        got.len(),
        want.len(),
    ))
}

/// The `MovePresident`/`PlacePresident` subsequence of `moves`, in order — the
/// `MoveFamily::MovementRoutes` partition of a Movement decision point, as seen
/// from OUTSIDE the crate. `search_actions`' own `family_kind` is private, so the
/// integration tier names the same partition by move constructor; the two agree
/// because the retreat family is `ReturnPresidentToHand` and nothing else, which
/// the sweep's union check independently pins.
fn routes_partition(moves: &[Move]) -> Vec<Move> {
    moves
        .iter()
        .filter(|m| matches!(m, Move::MovePresident { .. } | Move::PlacePresident { .. }))
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// Driven-game union sweep (shared by both tiers).
// ---------------------------------------------------------------------------

/// Outcome of a sweep, for the caller's anti-vacuity assertions.
struct SweepStats {
    /// Total decision points visited — every one takes the strong
    /// string-multiset union check AND the elementwise order check (the cap
    /// below is a tripwire assert, not a branch to a weaker check).
    decision_points: usize,
    /// Decision points that dispatched a `MovementRoutes` family, i.e. that took
    /// the per-family ordered compare against the routes partition. Without
    /// this, a sweep that never reached Movement would report a green order gate
    /// having never exercised it.
    routes_points: usize,
    /// Total `MovePresident`/`PlacePresident` moves the routes families served
    /// across the sweep — a routes family is legitimately empty at some decision
    /// points, so `routes_points > 0` alone would not prove the compare saw any
    /// move at all.
    routes_moves: usize,
}

/// Tripwire ceiling on materializing the union for the strong string-multiset
/// check. Organic random play stays far below this (the Task-4 unit test
/// collects `iter_family` for every family every step at these same seeds
/// without trouble); the constructed retreat worst case exercises the exploding
/// path by COUNT instead, in its own test. The sweep hard-asserts the cap
/// rather than branching to a weaker check — a state that trips it demands a
/// deliberate decision, not a silent downgrade.
const UNION_MATERIALIZE_CAP: usize = 50_000;

/// Drive uniform-random games (via the shared `common` seeded driver) and, at
/// every decision point, assert `⋃_family iter_family(state, family)` equals
/// `get_legal_moves(state)` as a MULTISET **and** element for element, plus the
/// per-family `family_len` contract (exact for every family except Placement,
/// whose `family_len` is an honest upper bound ≥ the realized count).
///
/// The order half is 3.1b Task 3's new capability, in two steps: the
/// `MovementRoutes` family against the legal list's routes partition (the
/// contract — it is the family 3.1b re-implemented), then the whole concatenated
/// union against the whole legal list, which extends the same guarantee to every
/// other family for free because [`families`] emits its tags in
/// `get_legal_moves`' own emission order. The second subsumes the first, but the
/// first is what names the routes family when it breaks, and it is the only one
/// whose anti-vacuity counters prove Movement was reached at all.
fn run_union_sweep(rules: &Rules, seeds: std::ops::Range<u64>, cap: usize) -> SweepStats {
    let mut stats = SweepStats { decision_points: 0, routes_points: 0, routes_moves: 0 };

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

                // Tripwire, not a branch: an organic state past the cap fails
                // loudly instead of silently downgrading to a weaker check.
                assert!(
                    legal.len() <= UNION_MATERIALIZE_CAP,
                    "exhaustive list {} exceeds the materialize cap at {:?} ({players}p seed {seed}) — investigate before raising the cap",
                    legal.len(),
                    state.turn_phase,
                );

                // Strong check: materialize each family, pin its `family_len`
                // contract and (for routes) its order, then assert the
                // concatenated union == legal as a multiset AND in order.
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

                    // ORDER, per family (3.1b): the routes family must be the
                    // legal list's routes partition element for element.
                    if matches!(family, MoveFamily::MovementRoutes) {
                        if let Some(diff) = first_divergence(&got, &routes_partition(&legal)) {
                            panic!(
                                "MovementRoutes family is not the legal list's routes partition at {:?} ({players}p seed {seed} turn {}):\n{diff}",
                                state.turn_phase, state.turn_number,
                            );
                        }
                        stats.routes_points += 1;
                        stats.routes_moves += got.len();
                    }

                    union.extend(got);
                }
                // MEMBERSHIP first: on a membership failure this report names the
                // moves that appeared or vanished, which the order check cannot.
                if let Some(diff) = multiset_diff(&union, &legal) {
                    panic!(
                        "factored union != get_legal_moves at {:?} ({players}p seed {seed} turn {}):\n{diff}",
                        state.turn_phase, state.turn_number,
                    );
                }
                // ORDER, whole union: same members (just proven) in the same
                // sequence — the check a permutation bug cannot survive.
                if let Some(diff) = first_divergence(&union, &legal) {
                    panic!(
                        "factored union is a permutation of get_legal_moves, not a reproduction, at {:?} ({players}p seed {seed} turn {}):\n{diff}",
                        state.turn_phase, state.turn_number,
                    );
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
/// reproduces `get_legal_moves` exactly as a multiset AND in order, and every
/// `family_len` holds. Organic play never explodes a family past the cap, so
/// every decision point takes both strong checks.
#[test]
fn factored_union_equals_get_legal_moves_across_driven_games() {
    let rules = Rules::standard();
    // Cap tuned for a snappy per-edit gate: 800 steps × 5 seeds × 3 player
    // counts still reaches every phase and thousands of decision points; the
    // release sweep below is the deep distributional net.
    let stats = run_union_sweep(&rules, 0..5, 800);

    assert!(stats.decision_points > 500, "expected many decision points, saw {}", stats.decision_points);
    // Anti-vacuity for the ordered compare: it must have RUN on the routes
    // family, and that family must have served real moves.
    assert!(stats.routes_points > 50, "expected many Movement points, saw {}", stats.routes_points);
    assert!(stats.routes_moves > 100, "routes families served {} moves — too few to order-check", stats.routes_moves);
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

    // families() emits the retreat tag with the exact snapshot, alongside the
    // routes family.
    let fams = families(&state, &rules);
    let retreat = fams
        .iter()
        .find(|f| matches!(f, MoveFamily::MovementRetreat { .. }))
        .expect("Movement dispatches a retreat family");
    assert_eq!(
        *retreat,
        MoveFamily::MovementRetreat { on_board: on_board.clone() },
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
    // The President stands ON the board here, and that fact is readable in the
    // MOVES: the routes family walks and never places from hand. (This is what
    // the retreat tag's `from_hand: false` used to assert; the tag no longer
    // carries the President's location, because the retreat family does not
    // depend on it — see `MoveFamily::MovementRetreat`'s doc.)
    assert!(
        !legal.iter().any(|m| matches!(m, Move::PlacePresident { .. })),
        "a President already on the board is never placed from hand",
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
    assert_eq!(
        len, 3_231,
        "placement family_len must be the published ceiling Σ C(17,k) (k=0..=4) + 17 Option-B \
         basics = 3231, far above the C(17,3)={C_17_3} anti-vacuity floor",
    );

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
// Phase 3.1b: the MovementRoutes family through the PUBLIC factored surface.
//
// `game/route_enum.rs`'s unit tests pin the same behaviours against the
// crate-private assembly (`movement_routes_family`) and against the frozen
// `enumerate_president_routes`. These cells are the INTEGRATION tier: the same
// shapes reached the way a bot reaches them — `families()` dispatch, then
// `iter_family`/`family_len` — where the family tag, the boxed iterator and the
// public partition are all in the loop. A regression that only broke the public
// wiring (a mis-dispatched tag, a stale `family_len` arm) would leave every unit
// test green.
//
// All ≤ 10 areas by mandate: the enumerator's lex-drift `debug_assert` is
// O(prefixes × labels), so a money-rich dense board costs seconds in the debug
// profile. The dense cells are release-tier, at the bottom of this file.
// ---------------------------------------------------------------------------

/// Add the undirected edge `a`~`b` to `state`'s adjacency, tolled 1 Yen to the
/// bank when `tolled`. [`state_with_areas`] builds an edgeless board (the
/// retreat and placement fixtures above need no graph); route cells need one.
fn connect(state: &mut GameState, a: AreaId, b: AreaId, tolled: bool) {
    state.board_layout.adjacency.entry(a).or_default().push(b);
    state.board_layout.adjacency.entry(b).or_default().push(a);
    if tolled {
        state.board_layout.canal_crossings.push((a, b));
    }
}

/// The id of the standard-rules technology with `effect`.
fn tech_id(rules: &Rules, effect: TechEffect) -> TechCardId {
    rules
        .techs
        .iter()
        .find(|t| t.effect == effect)
        .map(|t| t.id)
        .expect("standard data carries the tech")
}

/// The four-area diamond `route_enum`'s unit fixtures are built on, rebuilt here
/// over [`state_with_areas`]: 0 branches to 1 and 2, which rejoin at 3; the
/// mover's President stands on 0 with own assistants on 1, 2 and 3 (a hop may
/// only land where the mover already has one); `canals` names the edges that toll
/// 1 Yen. 5 Yen affords every label these cells build.
fn routes_diamond(canals: &[(u8, u8)]) -> GameState {
    let mut s = state_with_areas(2, TurnPhase::Movement, 4);
    let edges = [(0u8, 1u8), (0, 2), (1, 3), (2, 3)];
    for (a, b) in edges {
        connect(&mut s, AreaId::new(a), AreaId::new(b), canals.contains(&(a, b)));
    }
    // A canal naming an edge that does not exist (or naming it in the other
    // orientation) would silently produce an UNTOLLED diamond, which is exactly
    // how the tolled-walk cell below would go vacuous without saying so.
    for c in canals {
        assert!(edges.contains(c), "fixture names a canal {c:?} that is not one of {edges:?}");
    }
    s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
    for a in [1u8, 2, 3] {
        s.players[0].assistants.on_board.insert(AreaId::new(a), 1);
    }
    s.players[0].money = 5;
    s
}

/// THE 3.1b integration assertion. At a Movement `state`:
///  - [`families`] dispatches a `MovementRoutes` tag (the public entry point);
///  - [`iter_family`] on it equals the legal list's routes partition ELEMENT FOR
///    ELEMENT, at a fixture rich enough to discriminate (`min_moves`);
///  - [`family_len`] is that exact realized count (routes is the exact-but-
///    costs-one-enumeration arm, unlike placement's upper bound);
///  - the routes and retreat partitions together account for the WHOLE legal
///    list, so a move the routes family dropped cannot be hiding in retreat.
///
/// The last point is checked by COUNT off the already-materialized legal list,
/// never by collecting the retreat family — the dense cells below sit on a
/// 2¹⁷-member retreat product, and re-materializing it would buy no information
/// the driven sweep does not already provide.
fn assert_routes_family_ordered(
    state: &GameState,
    rules: &Rules,
    min_moves: usize,
    label: &str,
) -> Vec<Move> {
    assert_eq!(state.turn_phase, TurnPhase::Movement, "{label}: fixture must be at Movement");
    let fams = families(state, rules);
    let routes = fams
        .iter()
        .find(|f| matches!(f, MoveFamily::MovementRoutes))
        .unwrap_or_else(|| panic!("{label}: Movement must dispatch a routes family, got {fams:?}"));

    let legal = get_legal_moves(state, rules);
    let want = routes_partition(&legal);
    assert!(
        want.len() >= min_moves,
        "{label}: anti-vacuity — the generator emitted {} routes-family moves, expected >= {min_moves}",
        want.len(),
    );

    let got: Vec<Move> = iter_family(state, rules, routes).collect();
    if let Some(diff) = first_divergence(&got, &want) {
        panic!("{label}: routes family != the legal list's routes partition:\n{diff}");
    }
    assert_eq!(
        family_len(state, rules, routes),
        got.len(),
        "{label}: routes family_len must be the exact realized count",
    );

    let retreat = legal.iter().filter(|m| matches!(m, Move::ReturnPresidentToHand { .. })).count();
    assert_eq!(
        got.len() + retreat,
        legal.len(),
        "{label}: routes + retreat must partition the whole Movement legal list",
    );
    got
}

/// The `(destination, path)` pairs of `moves`' `MovePresident` entries that end
/// on `dest`, in order. A teleport is the empty path.
fn moves_to(moves: &[Move], dest: u8) -> Vec<(AreaId, Vec<AreaId>)> {
    moves
        .iter()
        .filter_map(|m| match m {
            Move::MovePresident { path, destination } if *destination == AreaId::new(dest) => {
                Some((*destination, path.clone()))
            }
            _ => None,
        })
        .collect()
}

/// The destinations of `moves`' `PlacePresident` entries, in order.
fn place_destinations(moves: &[Move]) -> Vec<AreaId> {
    moves
        .iter()
        .filter_map(|m| match m {
            Move::PlacePresident { destination } => Some(*destination),
            _ => None,
        })
        .collect()
}

/// ANTI-VACUITY (c), suppression FIRES: a FREE walk to the station area already
/// carries that area's `(destination, {})` label, so the Station teleport would
/// duplicate it and is suppressed — EXACTLY ONE survivor to that area, and it is
/// the walk. The control matters as much as the count: the teleport is
/// independently confirmed available (`can_teleport_to_station_with_tech` says
/// `Some`), so its absence is suppression rather than an unmet precondition.
#[test]
fn a_free_walk_suppresses_the_station_teleport_through_the_public_surface() {
    let rules = Rules::standard();
    let mut s = routes_diamond(&[]);
    s.players[0].technologies = vec![tech_id(&rules, TechEffect::Station)];
    s.station_location = Some(AreaId::new(1));

    assert_eq!(
        can_teleport_to_station_with_tech(&s, PlayerId::new(0), Some(&rules.techs)),
        Some(AreaId::new(1)),
        "control: every teleport precondition other than suppression must hold",
    );

    let moves = assert_routes_family_ordered(&s, &rules, 3, "free walk + Station");
    assert_eq!(
        moves_to(&moves, 1),
        vec![(AreaId::new(1), vec![AreaId::new(1)])],
        "exactly one survivor to the station area, and it is the walk: {moves:?}",
    );
}

/// ANTI-VACUITY (c), suppression does NOT fire — the I-1 regression. Both edges
/// into area 1 are canal crossings, so every WALK there costs 1 Yen while the
/// Station teleport is free: different `(destination, fee vector)` labels, so
/// BOTH must survive. An implementation that suppressed on destination rather
/// than on zero cost silently drops the teleport here.
#[test]
fn a_tolled_walk_and_a_free_teleport_both_survive_through_the_public_surface() {
    let rules = Rules::standard();
    let mut s = routes_diamond(&[(0, 1), (1, 3)]);
    s.players[0].technologies = vec![tech_id(&rules, TechEffect::Station)];
    s.station_location = Some(AreaId::new(1));

    let moves = assert_routes_family_ordered(&s, &rules, 4, "tolled walk + Station");
    assert_eq!(
        moves_to(&moves, 1),
        vec![(AreaId::new(1), vec![AreaId::new(1)]), (AreaId::new(1), vec![])],
        "the tolled walk and the free teleport are distinct labels — both survive: {moves:?}",
    );
}

/// Tram at the PER-EDIT tier (T3 review): organic play at these seeds never
/// emits a teleport, so without this cell Tram's public-surface path would be
/// exercised only by the release sweep. Area 3's assistant is removed, making
/// it walk-unreachable; Tram (and only Tram) offers it as the free teleport,
/// while the free-walk destinations stay suppressed.
#[test]
fn tram_offers_the_walk_unreachable_area_through_the_public_surface() {
    let rules = Rules::standard();
    let mut s = routes_diamond(&[]);
    s.players[0].technologies = vec![tech_id(&rules, TechEffect::Tram)];
    s.players[0].assistants.on_board.remove(&AreaId::new(3));

    let moves = assert_routes_family_ordered(&s, &rules, 3, "Tram to the uncovered area");
    assert_eq!(
        moves_to(&moves, 3),
        vec![(AreaId::new(3), vec![])],
        "area 3 has no own assistant, so only Tram's free teleport reaches it: {moves:?}",
    );
    for dest in [1u8, 2] {
        assert_eq!(
            moves_to(&moves, dest).len(),
            1,
            "the free walk to area {dest} suppresses Tram's duplicate teleport",
        );
    }
}

/// From hand: the routes family is `PlacePresident` over ASCENDING area ids,
/// gated on the possible-action-area test and on affording the destination fee —
/// and `families()` still dispatches BOTH Movement families there, the
/// public-surface fact the crate-private assembly cannot show.
///
/// Through 3.1a the tag check here read a `from_hand: true` flag on the retreat
/// family. 3.2 removed that flag (spec §8 parked item 3) and this test keeps the
/// same guarantee structurally instead: at an in-hand Movement point the
/// `MovementRetreat` tag is still emitted, still carries the seat's `on_board`
/// snapshot, and the union still serves the `PlacePresident` arm — the state
/// drives `families()`, so nothing needed a flag to record it.
#[test]
fn place_president_from_hand_through_the_public_surface() {
    let rules = Rules::standard();
    let mut s = routes_diamond(&[]);
    s.players[0].president_location = PresidentLocation::InHand;
    s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(2));

    // Anti-vacuity first: an empty snapshot would make the tag pin below hold
    // for a fixture with nothing to retreat.
    assert!(
        !s.players[0].assistants.on_board.is_empty(),
        "the snapshot pin needs assistants on the board to mean anything",
    );
    assert_eq!(
        families(&s, &rules),
        vec![
            MoveFamily::MovementRoutes,
            MoveFamily::MovementRetreat { on_board: s.players[0].assistants.on_board.clone() },
        ],
        "an in-hand Movement point dispatches both families, retreat carrying the snapshot",
    );

    // No Postal: the opponent's area is not a Possible Action Area.
    let moves = assert_routes_family_ordered(&s, &rules, 2, "from hand, no Postal");
    assert_eq!(place_destinations(&moves), vec![AreaId::new(1), AreaId::new(3)]);
    assert!(
        !moves.iter().any(|m| matches!(m, Move::MovePresident { .. })),
        "a President in hand walks nowhere: {moves:?}",
    );

    // Postal opens area 2 at a 1-Yen destination fee, which 5 Yen affords.
    s.players[0].technologies = vec![tech_id(&rules, TechEffect::PostalSystem)];
    let moves = assert_routes_family_ordered(&s, &rules, 3, "from hand, Postal");
    assert_eq!(
        place_destinations(&moves),
        vec![AreaId::new(1), AreaId::new(2), AreaId::new(3)],
        "ascending, including the fee-paying destination",
    );

    // Broke: the fee gates area 2 back out; the free destinations remain.
    s.players[0].money = 0;
    let moves = assert_routes_family_ordered(&s, &rules, 2, "from hand, Postal, broke");
    assert_eq!(
        place_destinations(&moves),
        vec![AreaId::new(1), AreaId::new(3)],
        "an unaffordable destination fee excludes the destination",
    );
}

/// The pre-flight's delegation arm, driven through the PUBLIC surface. A board
/// carrying `AreaId(18)` exceeds `route_enum::MAX_AREAS`, so the enumerator
/// hands the whole walk back to the frozen `enumerate_president_routes` — and
/// `iter_family` must still be the legal list's routes partition, in order.
/// `AreaId::new` refuses ids ≥ 18, so the tuple constructor is used deliberately:
/// no generated board reaches this shape (`route_enum`'s
/// `every_generated_board_fits_the_fixed_arrays` pins that), and the fixture
/// exists only to drive the fallback.
#[test]
fn an_oversized_board_delegates_and_still_serves_the_partition() {
    let rules = Rules::standard();
    let mut s = state_with_areas(2, TurnPhase::Movement, 1);
    s.board_layout.areas.insert(
        AreaId(18),
        AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) },
    );
    connect(&mut s, AreaId::new(0), AreaId(18), false);
    s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
    s.players[0].assistants.on_board.insert(AreaId(18), 1);
    s.players[0].money = 5;

    assert!(!fits_fixed_arrays(&s), "an AreaId past MAX_AREAS must fail the pre-flight");
    let moves = assert_routes_family_ordered(&s, &rules, 1, "oversized board");
    assert_eq!(
        moves,
        vec![Move::MovePresident { path: vec![AreaId(18)], destination: AreaId(18) }],
        "the delegated walk must still be served: {moves:?}",
    );
}

// ---------------------------------------------------------------------------
// Release tier.
// ---------------------------------------------------------------------------

/// RELEASE-GATE union sweep (CLAUDE.md two-tier policy): the same per-decision
/// union-and-order equivalence property as the per-edit driven test, swept
/// broader (seeds 0..20 × 2/3/4P) and deeper into each game — the distributional
/// regression net that need not re-run on every edit.
#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): union+order equivalence sweep at seeds 0..20 x 2/3/4P deep into each game; run with cargo test --release -p yokohama-core --test action_source_equivalence -- --ignored before merges/phase closes"]
fn release_union_sweep_seeds_0_20() {
    let rules = Rules::standard();
    let stats = run_union_sweep(&rules, 0..20, 8000);

    assert!(stats.decision_points > 5000, "expected a large sweep, saw {}", stats.decision_points);
    assert!(stats.routes_points > 500, "expected many Movement points, saw {}", stats.routes_points);
    assert!(stats.routes_moves > 1000, "routes families served only {} moves", stats.routes_moves);
    println!(
        "release union sweep: {} decision points, all multiset- AND order-checked against \
         get_legal_moves; {} of them dispatched a MovementRoutes family, serving {} route moves",
        stats.decision_points, stats.routes_points, stats.routes_moves,
    );
}

// ---------------------------------------------------------------------------
// Phase 3.1b: DENSE adversarial route cells (release tier only).
//
// Organic play offers the enumerator tens of labels; these cells offer hundreds,
// which is where a comparator or witness bug that the sparse cells tolerate
// actually surfaces. They are release-tier for cost, not for character: each one
// pays a full frozen-reference enumeration inside `get_legal_moves` on a
// money-saturated board, plus that board's 2¹⁷-member retreat product, and the
// enumerator's lex-drift `debug_assert` is O(prefixes × labels) in the debug
// profile — a per-edit dense cell would cost seconds.
//
// EVERY LABEL FIGURE HERE IS AN OBSERVATION, NEVER A CEILING (spec F5/L9). The
// label space is unsearched; the floors below are `>=` against counts THIS
// harness reproduced on this machine, and the recorded maxima are printed, never
// asserted. Nothing in this file may be read as a bound on route-label count.
// ---------------------------------------------------------------------------

/// One dense cell of the battery: `route_fixtures`' shared dense 17-area
/// construction (the real staggered pyramid with an interior Canal, own
/// assistants on all 17 non-Canal areas, the three opponents on `opponents`,
/// money saturated) with `techs` granted on top.
///
/// The tech grant is the only part that stays here, because it is the only part
/// that needs `Rules`: [`tech_id`] resolves an effect against the very
/// `Rules` this binary is testing with, which keeps the shared fixture module
/// free of a `Rules` parameter it would otherwise carry for one caller. The
/// GEOMETRY is shared, and that is what spec §4-2's same-fixture rule is about —
/// `action_source_worstcase.rs` meters the identical construction.
fn dense_state(start: u8, opponents: [u8; 3], techs: &[TechEffect], rules: &Rules) -> GameState {
    let mut s = route_fixtures::dense_17_area_cell_at(start, opponents);
    s.players[0].technologies = techs.iter().map(|&e| tech_id(rules, e)).collect();
    s
}

/// One dense cell: a geometry plus the label count THIS harness reproduced for
/// it, as a FLOOR (`>=`), never as a ceiling.
struct DenseCell {
    name: &'static str,
    /// The mover's President's area.
    start: u8,
    /// The three opponents' President areas.
    opponents: [u8; 3],
    techs: &'static [TechEffect],
    /// Max label count OBSERVED under THIS harness (recorded in the test's doc
    /// comment with the reproduction it came from). Asserted only as a floor.
    floor: usize,
}

/// The dense battery. `plain` and `postal` reproduce
/// `routes3_worstcase.rs`'s maximal state (start 0, opponents [3,8,17]); the last
/// two reproduce the two label-richest geometries the 3.1b research recorded
/// (spec F5: R1's start-12 cell and the critic probe's start-17 cell, both
/// Postal, both previously UNTIMED — this battery times them).
///
/// **The four floors are this harness's own measurements, and they came back
/// EQUAL to the four figures the spec quotes from three different harnesses**
/// (458 / 519 from the difftest battery, 612 from R1's sweep, 694 from the critic
/// probe). That agreement is evidence the geometries were reproduced faithfully;
/// it is NOT evidence that any of them is a maximum. Every one stays a `>=` floor
/// (spec F5/L9: the label space is unsearched, so no figure is a ceiling).
const DENSE_CELLS: [DenseCell; 4] = [
    DenseCell { name: "plain", start: 0, opponents: [3, 8, 17], techs: &[], floor: 458 },
    DenseCell {
        name: "postal",
        start: 0,
        opponents: [3, 8, 17],
        techs: &[TechEffect::PostalSystem],
        floor: 519,
    },
    DenseCell {
        name: "postal, start 12, opponents [1,6,11]",
        start: 12,
        opponents: [1, 6, 11],
        techs: &[TechEffect::PostalSystem],
        floor: 612,
    },
    DenseCell {
        name: "postal, start 17, opponents [2,7,12]",
        start: 17,
        opponents: [2, 7, 12],
        techs: &[TechEffect::PostalSystem],
        floor: 694,
    },
];

/// RELEASE GATE — the dense adversarial route cells. At each cell the
/// `MovementRoutes` family served through the public factored surface equals the
/// legal list's routes partition ELEMENT FOR ELEMENT at hundreds of labels, meets
/// its observed floor, and has its realized label count and wall time RECORDED.
///
/// MEASURED 2026-07-25, release profile, this Mac, whole test 0.75 s. The
/// binary's release gate was 20.77 s before this battery and measured 21.24 s
/// then 20.90 s after: the two release tests run on separate threads, so the
/// dense cells overlap the longer sweep and the wall-clock delta lands inside
/// run-to-run noise rather than adding 0.75 s.
///
/// | cell                                | labels | `enumerate_routes` | ordered compare |
/// |-------------------------------------|--------|--------------------|-----------------|
/// | plain                               |    458 |            4.56 ms |       224.34 ms |
/// | postal                              |    519 |            3.81 ms |       214.64 ms |
/// | postal, start 12, opponents [1,6,11]|    612 |            1.87 ms |       113.23 ms |
/// | postal, start 17, opponents [2,7,12]|    694 |            2.69 ms |       162.55 ms |
///
/// The right column is dominated by the frozen reference inside `get_legal_moves`
/// (its route DFS plus the 2¹⁷ retreat product) — i.e. by the cost the factored
/// serve exists to avoid, not by the factored serve. Both columns are RECORDED,
/// never asserted, and the run-to-run spread is real: the `plain` cell runs
/// first and enumerated in 9.27 ms on a cold binary against 4.56 ms on a repeat,
/// which is why no timing here is a threshold. The label counts, by contrast,
/// were bit-identical across both runs.
///
/// DISCRIMINATION (mutation-probed 2026-07-25, on the driven sweep this file's
/// order gate shares): reversing the `MovementRoutes` `Vec` before the compare
/// fails at 2P seed 0 turn 1, index 0 — while the retained `multiset_diff` check
/// PASSES the same reversal unchanged. The order gate is therefore a genuinely
/// new capability over the landed 3.1a sweep, not a restatement of it. Both
/// mutations were reverted.
#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): dense adversarial route cells (17-area staggered board, money-saturated) — each pays a full frozen-reference enumeration plus a 2^17 retreat product; run with cargo test --release -p yokohama-core --test action_source_equivalence -- --ignored before merges/phase closes"]
fn release_dense_route_cells_ordered_and_floored() {
    let rules = Rules::standard();
    let mut lines = Vec::new();

    for cell in &DENSE_CELLS {
        let s = dense_state(cell.start, cell.opponents, cell.techs, &rules);
        assert!(fits_fixed_arrays(&s), "{}: dense cell must take the fixed-array path", cell.name);

        // The factored enumeration alone: the walked-label set and its cost.
        let t = std::time::Instant::now();
        let labels = enumerate_routes(&s, &rules).len();
        let enum_time = t.elapsed();

        // The ordered compare, whose cost is dominated by the frozen reference
        // inside `get_legal_moves` (the reference route DFS plus this board's
        // 2^17-member retreat product) — the ceiling the factored serve avoids.
        let t = std::time::Instant::now();
        let moves = assert_routes_family_ordered(&s, &rules, cell.floor, cell.name);
        let compare_time = t.elapsed();

        // No teleport tech in any cell, so the family IS the walked-label set —
        // which is what makes `labels` the figure the floor is stated against.
        assert_eq!(
            moves.len(),
            labels,
            "{}: without Station/Tram the family must be exactly the walked labels",
            cell.name,
        );
        // The floor is asserted on BOTH sides independently — on the generator's
        // partition inside the helper above, and here on the enumerator's own
        // output. The ordered compare forces the two to agree, so either alone
        // would do; stating both means a regression that silently emptied one
        // side names the side it emptied.
        assert!(
            labels >= cell.floor,
            "{}: enumerate_routes returned {labels} labels, below the {} this harness observed",
            cell.name,
            cell.floor,
        );

        lines.push(format!(
            "  {:38} {labels:>4} labels (floor {:>4})  enumerate_routes {:>8.2?}  \
             ordered-compare-incl-reference {:>9.2?}",
            cell.name, cell.floor, enum_time, compare_time,
        ));
    }

    println!(
        "DENSE ROUTE CELLS (release; 4P staggered pyramid, interior Canal, 17 own-assistant \
         areas, money 1000). Label counts are OBSERVATIONS under this harness, never ceilings \
         (spec F5/L9); times are recorded, never asserted:\n{}",
        lines.join("\n"),
    );
}
