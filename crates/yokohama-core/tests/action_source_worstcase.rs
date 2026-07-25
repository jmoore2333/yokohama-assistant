//! Phase 3.1a Task 6 — the EXECUTABLE PROOF that the six-figure retreat handover
//! (guidingpath:172b) is discharged: serving ONE retreat candidate off the 4P
//! worst case (product `3³ · 2¹⁴ = 442,368`) allocates a bounded, near-constant
//! COUNT — `O(#areas)`, never `O(product)` — so a search inner loop can page or
//! sample the family the exhaustive `get_legal_moves` would otherwise
//! materialize.
//!
//! METER (spec `docs/plans/2026-07-25-phase3.1-action-interface-spec.md` §4,
//! PINNED). The shared counting allocator (`action_cost_meter`, included below
//! and by `examples/action_cost.rs`) reports two figures per measured region:
//!   - allocation COUNT — the PRIMARY, HARD, deterministic assertion. An
//!     `O(product)` serve would allocate ~442k times; a bounded serve stays under
//!     [`BOUND`], so `< BOUND` discriminates six orders of magnitude.
//!   - peak OUTSTANDING requested bytes — the high-water mark of net-live
//!     requested bytes over the region (NOT RSS, NOT cumulative). RECORDED
//!     (printed), never gated: it is machine-variable, so a hard byte assertion
//!     would be brittle.
//!
//! TWO TIERS (CLAUDE.md two-tier policy):
//!   - PER-EDIT (not ignored): serve one candidate off the FULL 442,368 worst
//!     case (lazy ⇒ instant) and off a scaled-down smoke — the COUNT bound holds
//!     and is FLAT while the product grows 108×. Never materializes the family.
//!   - RELEASE (`#[ignore]`d): build the full 4P OnBoard `GameState` and PAY the
//!     exhaustive `get_legal_moves` ceiling ONCE (MEASURED on this fixture: 442,369
//!     moves, ~1.02M allocs) to record concretely the gap the factored serve
//!     avoids, while re-proving the serve-one bound on the same snapshot.
//!
//! MUTATION-DISCRIMINATION (recorded in guidingpath's 2026-07-25 3.1a rows):
//! because Tasks 2/5 already made retreat lazy, the COUNT bound passes on first
//! write. Its discriminating power was proven by temporarily routing the
//! measured serve through a materialize-then-index path
//! (`retreat_iter(&on_board).collect::<Vec<_>>()[k].clone()`): the count leapt
//! to 539,096 — the measured ALLOCATION count of the full collect, reproducible
//! as `examples/action_cost.rs`'s `retreat_iter().collect()` row (the MOVE
//! count is 442,368) — and the bound FAILED, confirming `< BOUND` genuinely
//! separates `O(#areas)` from `O(product)`. Reverted to the lazy path here.

#[path = "action_cost_meter/mod.rs"]
mod meter;

use std::collections::BTreeMap;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use yokohama_core::{
    families, family_len, get_legal_moves, retreat_iter, retreat_len, retreat_sample,
    AchievementCard, AchievementType, AreaConfig, AreaId, AreaType, BoardLayout, ChurchTrack,
    CustomsTrack, GameConfig, GameState, LabState, MercenarySupply, MoveFamily, PlayerColor,
    PlayerId, PlayerState, PortState, PresidentLocation, Rules, TurnPhase,
};

/// The hard allocation-COUNT ceiling for serving ONE retreat candidate — the
/// same generous `< 64` the lib-internal probe uses
/// (`search_actions::tests::serving_one_retreat_candidate_is_o_areas_not_o_product`):
/// a handful of small allocations (the ≤17-entry radix `Vec` plus one decoded
/// ≤17-entry `recover` `BTreeMap`), never scaling with the 442,368 product.
const BOUND: usize = 64;

/// The 4P retreat worst case `on_board`: three areas at count 2 (ids 0,1,2) and
/// fourteen at count 1 (ids 3..17) — product `3³ · 2¹⁴ = 442,368`, the six-figure
/// family this factored source exists to serve without materializing.
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

/// A scaled-DOWN retreat `on_board` for the per-edit smoke: twelve count-1 areas
/// ⇒ product `2¹² = 4096`. Small enough to serve a mid index instantly, large
/// enough that an `O(product)` serve would allocate thousands of times — so the
/// SAME `< BOUND` count here evidences the bound is FLAT in product size.
fn smoke_on_board() -> BTreeMap<AreaId, u8> {
    (0..12u8).map(|a| (AreaId::new(a), 1u8)).collect()
}

/// PER-EDIT (not ignored) — THE HARD BOUND: serving one retreat candidate off
/// the 442,368 worst case (a lazy `nth` deep into the family, and one
/// independent sample) allocates under [`BOUND`], and the same bound holds at
/// the scaled smoke — proving the count is `O(#areas)`, not `O(product)`. Peak
/// outstanding bytes are printed (recorded), never asserted.
#[test]
fn serving_one_retreat_candidate_is_bounded_allocation_count() {
    let worst = worst_case_on_board();
    // The ceiling avoided, from the closed-form product — no materialization.
    assert_eq!(retreat_len(&worst), 442_368, "worst-case retreat product");

    // Lazy deep serve: index 400k into the 442,368 family.
    let (served, m_iter) = meter::measure(|| retreat_iter(&worst).nth(400_000));
    assert!(served.is_some(), "index 400_000 is within the 442,368 family");
    assert!(
        m_iter.allocs < BOUND,
        "retreat_iter().nth(400_000) allocated {} (>= {BOUND}) — a serve that scales with the 442k product",
        m_iter.allocs,
    );

    // Independent per-area sample: also bounded.
    let mut rng = ChaCha8Rng::seed_from_u64(20_260_725);
    let (_sampled, m_sample) = meter::measure(|| retreat_sample(&worst, &mut rng));
    assert!(m_sample.allocs < BOUND, "retreat_sample allocated {} (>= {BOUND})", m_sample.allocs);

    // Scaled smoke (product 4096): the SAME bound — the count is flat in product.
    let smoke = smoke_on_board();
    let smoke_len = retreat_len(&smoke);
    assert_eq!(smoke_len, 4096);
    let (served_smoke, m_smoke) = meter::measure(|| retreat_iter(&smoke).nth(smoke_len / 2));
    assert!(served_smoke.is_some());
    assert!(
        m_smoke.allocs < BOUND,
        "smoke retreat_iter().nth allocated {} (>= {BOUND})",
        m_smoke.allocs,
    );

    // Record (never gated): the count stays bounded and near-flat while the
    // product grows 4096 → 442,368 (108×), and peak outstanding bytes stay tiny.
    println!(
        "serve-one retreat allocation COUNT (bound {BOUND}):\n  \
         product 442368: nth={} allocs, sample={} allocs\n  \
         product   4096: nth={} allocs\n  \
         peak outstanding bytes: nth={}, sample={}, smoke-nth={}",
        m_iter.allocs,
        m_sample.allocs,
        m_smoke.allocs,
        m_iter.peak_bytes,
        m_sample.peak_bytes,
        m_smoke.peak_bytes,
    );
}

/// The full 4P **OnBoard** retreat worst-case `GameState`: 17 placement-allowing
/// areas (ids 0..17), the current player's president ON BOARD at area 0, own
/// assistants at the worst-case distribution ⇒ `get_legal_moves` materializes
/// the 442,368-move retreat block (plus a handful of routes). Type-valid via the
/// public constructors exactly like `action_source_equivalence.rs`'s
/// `state_with_areas` / `routes3_worstcase.rs`'s `movement_state` (not
/// setup-valid; both `get_legal_moves` and the factored source are total over
/// it). One free edge (0~1) leaves a couple of `MovePresident` routes without an
/// own-assistant-dense DFS explosion, so the recorded ceiling is dominated by
/// the retreat product it exists to avoid.
fn worst_case_state() -> GameState {
    let colors = [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green];
    let mut areas = BTreeMap::new();
    for i in 0..17u8 {
        areas.insert(
            AreaId::new(i),
            AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, i) },
        );
    }
    let mut adjacency = BTreeMap::new();
    adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
    adjacency.insert(AreaId::new(1), vec![AreaId::new(0)]);

    let mut s = GameState {
        config: GameConfig::for_player_count(4),
        board_layout: BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        },
        players: (0..4)
            .map(|i| PlayerState::new(PlayerId::new(i), colors[i as usize], i == 0))
            .collect(),
        current_player: PlayerId::new(0),
        turn_phase: TurnPhase::Movement,
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
    };
    s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
    s.players[0].assistants.on_board = worst_case_on_board();
    s
}

/// RELEASE (`#[ignore]`d) — pay the exhaustive ceiling ONCE to record the gap.
/// Build the full 4P OnBoard state and materialize `get_legal_moves` (MEASURED on
/// this fixture: 442,369 moves = 442,368 retreat + 1 route, ~1.02M allocs, ~57.8
/// MB peak) — the exact cost the factored serve avoids — then re-prove the
/// serve-one COUNT bound on the same snapshot. `#[ignore]`d for its
/// recorded-curve character: the ceiling run is a regression-net MEASUREMENT,
/// not a per-edit correctness property — an equal-cost 442k materialization
/// already runs per-edit in `action_source_equivalence`'s retreat fixture, so
/// runtime is not the tiering reason.
#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): a recorded-curve measurement, not a per-edit correctness property — re-pays the 442k exhaustive ceiling (~1.02M allocs) to record the gap the factored serve avoids (an equal-cost materialization already runs per-edit in action_source_equivalence); run with cargo test --release -p yokohama-core --test action_source_worstcase -- --ignored before merges/phase closes"]
fn exhaustive_ceiling_recorded_and_serve_one_still_bounded() {
    let rules = Rules::standard();
    let state = worst_case_state();
    let on_board = state.players[0].assistants.on_board.clone();

    // The retreat family the factored source serves lazily — closed-form product.
    let retreat = families(&state, &rules)
        .into_iter()
        .find(|f| matches!(f, MoveFamily::MovementRetreat { .. }))
        .expect("Movement dispatches a retreat family");
    assert_eq!(family_len(&state, &rules, &retreat), 442_368);

    // PAY the exhaustive ceiling once, under the meter (build + drop the list
    // INSIDE the measured region so allocs/peak reflect the full materialization).
    let (legal_len, m_exhaustive) = meter::measure(|| get_legal_moves(&state, &rules).len());
    // The MEASURED total on this fixture — the 442,368 retreat product plus exactly
    // ONE MovePresident route (0→1 over the lone free edge; no return-to-start) =
    // 442,369. This is the measured value, NOT the brief-reasoned ~442,384 (which
    // assumed a denser board's ~16 routes); the minimal 0~1 adjacency deliberately
    // keeps the route term at 1 so the ceiling is dominated by the retreat product.
    assert_eq!(
        legal_len, 442_369,
        "exhaustive list must be 442,368 retreat + 1 route on this minimal-adjacency fixture",
    );

    // Re-prove the serve-one bound on the SAME snapshot.
    let (served, m_serve) = meter::measure(|| retreat_iter(&on_board).nth(400_000));
    assert!(served.is_some());
    assert!(m_serve.allocs < BOUND, "serve-one allocated {} (>= {BOUND})", m_serve.allocs);

    println!(
        "EXHAUSTIVE CEILING vs FACTORED SERVE (4P OnBoard worst case):\n  \
         exhaustive get_legal_moves: {legal_len} moves, {} allocs, {} peak outstanding bytes\n  \
         factored serve-one:              1 move, {} allocs, {} peak outstanding bytes\n  \
         allocation ratio: {}x",
        m_exhaustive.allocs,
        m_exhaustive.peak_bytes,
        m_serve.allocs,
        m_serve.peak_bytes,
        m_exhaustive.allocs / m_serve.allocs.max(1),
    );
}
