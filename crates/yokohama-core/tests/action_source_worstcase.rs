//! Worst-case ALLOCATION bounds for the factored action source
//! (`game/search_actions.rs`) — THREE bounds, with DELIBERATELY DIFFERENT
//! SHAPES.
//!
//! | bound | scope | shape | why that shape |
//! |-------|-------|-------|----------------|
//! | 1 (3.1a T6) | retreat family | SERVE-ONE constant: `< `[`BOUND`] allocations to serve ONE candidate out of 442,368 | the family is HUGE, so it is paged/sampled lazily and NEVER materialized |
//! | 2 (3.1b T4) | routes family | WHOLE-FAMILY `O(labels)`: `<= 2·labels + 32` allocations for the ENTIRE enumeration ([`route_bound`]) | the family is SMALL (max OBSERVED 694 adversarial / 38 organic — spec L9: no label ceiling is known), so it is COMPUTED CHEAPLY per call, not paged |
//! | 3 (3.2 T5) | one whole DECISION POINT | ONE-PICK constant: handle construction + one `RolloutPolicy` pick, [`pick_bound_on_board`] / [`PICK_BOUND_IN_HAND`] | a bot pays per DECISION, not per family; this is the executable half of spec §0-R2(b), at BOTH President locations because the L10 counter is blind at one of them |
//!
//! The shapes differ because the costs differ, and conflating them is the
//! mis-read spec §0's honest reframing exists to prevent: **the routes family
//! never needed LAZINESS — it needed a cheap COMPUTATION.** No bound's language
//! transfers to another. A serve-one constant is not even expressible for
//! routes, whose contract is one whole enumeration per call (spec §3.1, L10 — no
//! cache in 3.1b); an `O(family)` bound would be vacuous for retreat, whose
//! family has 442,368 members and whose whole point is never to touch them all;
//! and neither of those is a statement about what ONE DECISION costs, which is
//! the figure §0-R2(b) needed and bound 3 supplies. Every doc site that lists
//! family costs states the three FAMILY shapes (retreat's closed form, routes'
//! exact-but-one-enumeration, placement's upper bound) side by side for the same
//! reason.
//!
//! METER (spec `docs/plans/2026-07-25-phase3.1-action-interface-spec.md` §4,
//! PINNED; shared by BOTH bounds). The shared counting allocator
//! (`action_cost_meter`, included below and by `examples/action_cost.rs`) reports
//! two figures per measured region:
//!   - allocation COUNT — the PRIMARY, HARD, deterministic assertion. For
//!     retreat, an `O(product)` serve would allocate ~442k times against a
//!     [`BOUND`] of 64, so the gate discriminates six orders of magnitude; for
//!     routes, the frozen reference DFS allocates per visited NODE against a
//!     bound linear in LABELS (see bound 2's mutation record below).
//!   - peak OUTSTANDING requested bytes — the high-water mark of net-live
//!     requested bytes over the region (NOT RSS, NOT cumulative). RECORDED
//!     (printed), never gated: it is machine-variable, so a hard byte assertion
//!     would be brittle. Wall clock is likewise recorded, never asserted.
//!
//! ---
//! ## BOUND 1 — retreat (Phase 3.1a Task 6)
//!
//! The EXECUTABLE PROOF that the six-figure retreat handover (guidingpath:172b)
//! is discharged: serving ONE retreat candidate off the 4P worst case (product
//! `3³ · 2¹⁴ = 442,368`) allocates a bounded, near-constant COUNT — `O(#areas)`,
//! never `O(product)` — so a search inner loop can page or sample the family the
//! exhaustive `get_legal_moves` would otherwise materialize.
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
//!
//! ---
//! ## BOUND 2 — routes (Phase 3.1b Task 4, spec §4-2)
//!
//! `route_enum::enumerate_routes` computes the WHOLE walked-label set in
//! [`route_bound`] allocations — linear in the labels it RETURNS, never in the
//! simple paths it WALKS. The frozen `enumerate_president_routes` is the
//! contrast: `RouteDfs::step` (movement.rs:475) builds a fresh `RouteVector`
//! (`BTreeMap`) for every hop it prices, plus visited-`BTreeSet` traffic per
//! recursion and a witness `Vec` clone per kept label, so its count tracks
//! PATHS. Worst-case TIME remains `O(affordable simple paths)` for both — spec
//! §0 bought the allocation reading, not a complexity result, and nothing here
//! claims otherwise.
//!
//! ANTI-VACUITY IS LOAD-BEARING HERE (spec §4-2). `<= 2·labels + 32` is
//! SELF-REFERENTIAL in `labels`: an enumerator that silently returned nothing
//! would pass it trivially. So every route cell FIRST pins its realized label
//! count to the frozen reference's count on the SAME state and to a stated
//! floor, and only THEN meters the bound. The floors are `>=` OBSERVATIONS under
//! this harness, never ceilings (spec F5/L9).
//!
//! SAME FIXTURES AS THE EQUIVALENCE GATE (spec §4-2, the dependency that makes
//! the bound mean anything): both route cells come from `tests/route_fixtures`,
//! and `action_source_equivalence.rs` pins that same dense construction's label
//! LIST — pairs and order — to the frozen enumerator. Without that, this file
//! would only know the count is large, not that it is right. Until Phase 3.2
//! Task 3 the construction was REPLICATED into each binary and the rule rested
//! on review discipline (a drifted copy stayed self-consistent and passed); the
//! shared module made it structural.
//!
//! TWO TIERS (CLAUDE.md two-tier policy):
//!   - PER-EDIT (not ignored): the ≤ 10-area smoke fixture, through both
//!     `enumerate_routes` and the public `iter_family(MovementRoutes)` surface.
//!     Small by mandate, the same one `action_source_equivalence.rs`' per-edit
//!     route cells follow — the enumerator's lex-drift `debug_assert` is
//!     `O(prefixes × labels)`, so a label-rich fixture costs multiples in the
//!     debug profile (this one is sub-millisecond).
//!   - RELEASE (`#[ignore]`d): the dense 17-area cell, which additionally PAYS
//!     the frozen reference's ceiling ONCE under the meter and RECORDS the gap.
//!
//! MUTATION-DISCRIMINATION (PERFORMED 2026-07-25, BOTH TIERS — the bound passes
//! on first write, so this is where its discriminating power comes from).
//! Reproduce by swapping the metered call for `enumerate_president_routes(&s,
//! s.current_player, from, Some(&rules.techs))`, which returns the SAME
//! `(destination, path)` pairs on the same state — so the label count, the
//! floor and the bound are all unchanged and only the measured COUNT moves:
//!   - per-edit smoke (44 labels, bound 120): the frozen DFS allocated **270**
//!     — 2.25× the bound — and the assertion FAILED.
//!   - release dense cell (458 labels, bound 948): the frozen DFS allocated
//!     **1,687,537** — 1,780× the bound — and the assertion FAILED.
//!
//! Both mutations were reverted. The release figure is not only a mutation
//! record: the same measurement runs unmutated in the release test as the
//! recorded reference ceiling, which is why that test can print a 3,613×
//! discharge ratio without ever asserting on it.

#[path = "action_cost_meter/mod.rs"]
mod meter;
mod route_fixtures;

use std::collections::BTreeMap;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use yokohama_core::{
    enumerate_president_routes, enumerate_routes, families, family_len, fits_fixed_arrays,
    get_legal_moves, iter_family, retreat_iter, retreat_len, retreat_sample, AreaId, DecisionHandle,
    GameState, Move, MoveFamily, PresidentLocation, RolloutPolicy, Rules,
};

use route_fixtures::{
    dense_17_area_cell, retreat_worst_case_cell, retreat_worst_case_on_board, small_movement_cell,
};

/// The hard allocation-COUNT ceiling for serving ONE retreat candidate — the
/// same generous `< 64` the lib-internal probe uses
/// (`search_actions::tests::serving_one_retreat_candidate_is_o_areas_not_o_product`):
/// a handful of small allocations (the ≤17-entry radix `Vec` plus one decoded
/// ≤17-entry `recover` `BTreeMap`), never scaling with the 442,368 product.
const BOUND: usize = 64;

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
    let worst = retreat_worst_case_on_board();
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

/// RELEASE (`#[ignore]`d) — the constructor builds an over-count VERBATIM;
/// `apply_return_to_hand` is the sole validator (T3 review F1: the pin restored
/// at the only tier where the property is OBSERVABLE). In debug the Task-3
/// over-count `debug_assert` intercepts before a return value exists, so no
/// debug test can watch the constructor decline to validate; this release-tier
/// test can, and it runs under the documented worstcase gate command (`cargo
/// test --release -p yokohama-core --test action_source_worstcase -- --ignored`),
/// which is what keeps it from being a test no gate ever runs. Mutation lineage:
/// a clamping mutation (`count.min(on_board[area])`) was caught by the
/// pre-Task-3 debug pin and is caught again by this assertion.
#[cfg(not(debug_assertions))]
#[test]
#[ignore = "release tier: the debug over-count tripwire makes the verbatim-build property observable only in release (T3 review F1); runs under the documented worstcase gate command"]
fn retreat_move_builds_an_over_count_verbatim_in_release() {
    // Imported here, not at the top: in a debug build this whole test is
    // compiled out and a top-level import would be unused. (`Move` IS at the
    // top level — bound 3 below uses it in both profiles.)
    use yokohama_core::retreat_move;

    let on_board: BTreeMap<AreaId, u8> = [(AreaId::new(0), 2u8)].into_iter().collect();
    let over = retreat_move(&on_board, &[(AreaId::new(0), 3)]);
    match over {
        Move::ReturnPresidentToHand { recover } => assert_eq!(
            recover.get(&AreaId::new(0)),
            Some(&3),
            "release retreat_move must build the over-count verbatim, leaving \
             apply_return_to_hand as the sole validator",
        ),
        other => panic!("retreat_move built {other:?}, not a ReturnPresidentToHand"),
    }
}

/// RELEASE (`#[ignore]`d) — pay the exhaustive ceiling ONCE to record the gap.
/// Build the full 4P OnBoard state (`route_fixtures::retreat_worst_case_cell`)
/// and materialize `get_legal_moves` (MEASURED on this fixture: 442,369 moves =
/// 442,368 retreat + 1 route, ~1.02M allocs, ~57.8 MB peak) — the exact cost the
/// factored serve avoids — then re-prove the serve-one COUNT bound on the same
/// snapshot. `#[ignore]`d for its recorded-curve character: the ceiling run is a
/// regression-net MEASUREMENT, not a per-edit correctness property — an
/// equal-cost 442k materialization already runs per-edit in
/// `action_source_equivalence`'s retreat fixture, so runtime is not the tiering
/// reason.
///
/// (The two doc blocks above were spliced into each other before Phase 3.2
/// Task 5 — this test's tail sentence had attached itself to the over-count test
/// and vice versa. Rejoined here; no assertion changed.)
#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): a recorded-curve measurement, not a per-edit correctness property — re-pays the 442k exhaustive ceiling (~1.02M allocs) to record the gap the factored serve avoids (an equal-cost materialization already runs per-edit in action_source_equivalence); run with cargo test --release -p yokohama-core --test action_source_worstcase -- --ignored before merges/phase closes"]
fn exhaustive_ceiling_recorded_and_serve_one_still_bounded() {
    let rules = Rules::standard();
    let state = retreat_worst_case_cell(false);
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

// ---------------------------------------------------------------------------
// BOUND 2 — the ROUTE family (Phase 3.1b Task 4, spec §4-2). A DIFFERENT SHAPE
// from bound 1: whole-family `O(labels)`, not serve-one constant. The module
// header states why, and why the two must never be quoted as one bound.
//
// The 3.1a retreat fixture above is route-starved BY DESIGN (one free edge, so
// exactly one walked route) and is not touched: these cells are route-rich and
// retreat-irrelevant, which is why they are a second fixture rather than a knob
// on the first.
// ---------------------------------------------------------------------------

/// The route family's hard allocation-COUNT ceiling at `labels` realized labels
/// (spec §4-2): `2·labels + 32`.
///
/// The LINEAR term is the enumerator's own output — one witness `Vec` per label,
/// plus the pair `Vec`'s geometric growth — doubled for headroom. The CONSTANT
/// absorbs the fixed per-call overhead that does not scale with labels: the
/// `RouteGraph` build, the growth steps of a small result, and (on the
/// family-level smoke) the boxed `iter_family` iterator. The 3.1b research held
/// this bound at every measured cell (467/529/85/17/62/146/237 allocations) with
/// roughly 2× margin, which is the headroom the doubling buys.
fn route_bound(labels: usize) -> usize {
    2 * labels + 32
}

/// The frozen reference's label count for `state` — the standing 3.1b oracle
/// (`enumerate_president_routes`), which every route cell's realized count is
/// pinned to before any bound is claimed.
///
/// Split out as its own call because the release cell pays it under the meter:
/// that ONE payment is both the anti-vacuity oracle and the recorded discharge
/// ceiling, and calling it twice would buy a second 182 ms for no information.
fn reference_label_count(state: &GameState, rules: &Rules, label: &str) -> usize {
    let PresidentLocation::OnBoard(from) = state.player(state.current_player).president_location
    else {
        panic!("{label}: a route cell's President stands on the board");
    };
    enumerate_president_routes(state, state.current_player, from, Some(&rules.techs)).len()
}

/// THE ANTI-VACUITY PIN, asserted BEFORE any bound is (spec §4-2): the bound
/// `<= 2·labels + 32` is SELF-REFERENTIAL in `labels`, so a silently-empty
/// enumerator would satisfy it. This pins the realized count to
///  - `reference_len`, the frozen [`reference_label_count`] on the SAME state
///    (the standing oracle every 3.1b equivalence test compares against), and
///  - `floor`, a count OBSERVED under this harness — a `>=` floor, never a
///    ceiling (spec F5/L9: the label space is unsearched).
///
/// COUNT, not list: the ordered pair-for-pair equality against the same frozen
/// reference is `action_source_equivalence.rs`' gate, on these same fixture
/// constructions. That division is deliberate — this file owns the cost claim
/// and borrows the correctness claim; duplicating the ordered compare here would
/// blur which gate fails when the enumerator regresses.
///
/// Takes both counts rather than computing them, so each tier can obtain them
/// the way its budget allows (the release cell meters them; the smoke does not)
/// while a single helper owns what the two counts must satisfy.
fn assert_route_labels(labels: usize, reference_len: usize, floor: usize, label: &str) {
    assert_eq!(
        labels, reference_len,
        "{label}: enumerate_routes returned {labels} labels against the frozen reference's \
         {reference_len}",
    );
    assert!(
        labels >= floor,
        "{label}: {labels} labels is below the {floor} this harness observed — the bound below \
         would be vacuous",
    );
}

/// The smoke fixture's label floor: 44 OBSERVED under this harness, asserted at
/// 40 so a benign label-space change is not a false failure while an emptied
/// enumerator still trips it. Seven times the plan's "≥ 6 labels so the bound
/// isn't trivial" mandate — the three opponent Presidents multiply each
/// destination's label set. A floor, never a ceiling (spec F5/L9).
const SMOKE_FLOOR: usize = 40;

/// PER-EDIT (not ignored) — THE ROUTE BOUND at a small route-rich fixture:
/// [`enumerate_routes`] computes the WHOLE label set in at most
/// [`route_bound`]`(labels)` allocations, and so does the same family served
/// through the public `iter_family(MovementRoutes)` surface (which additionally
/// assembles a `Move` per label and boxes its iterator — absorbed by the `+32`).
///
/// The fixture is `route_fixtures::small_movement_cell` — the CHEAP shared cell,
/// and the one a future per-edit consumer wanting a non-degenerate routes family
/// should reach for rather than paying the dense cell's seconds in debug.
///
/// MEASURED 2026-07-25 (debug profile, this Mac): 44 labels, bound 120;
/// `enumerate_routes` 50 allocations, `iter_family(MovementRoutes)` 56 — both
/// under half the bound, against the frozen DFS's 270 on the same state (the
/// module header's mutation record). The whole test is sub-millisecond. All
/// three figures re-measured unchanged 2026-07-26 after Task 3 moved the
/// construction into `tests/route_fixtures`.
#[test]
fn enumerating_a_small_route_family_is_bounded_allocation_count() {
    let rules = Rules::standard();
    let s = small_movement_cell();
    assert!(fits_fixed_arrays(&s), "the smoke fixture must take the fixed-array path");

    // Anti-vacuity FIRST: the bound is self-referential in `labels`.
    let labels = enumerate_routes(&s, &rules).len();
    assert_route_labels(labels, reference_label_count(&s, &rules, "smoke"), SMOKE_FLOOR, "smoke");
    let bound = route_bound(labels);

    // The enumerator alone — the walk-only surface the allocation claim is about.
    let (got, m_enum) = meter::measure(|| enumerate_routes(&s, &rules).len());
    assert_eq!(got, labels, "smoke: enumerate_routes is deterministic");
    assert!(
        m_enum.allocs <= bound,
        "smoke: enumerate_routes allocated {} for {labels} labels (> {bound} = 2·labels + 32) — \
         a count that tracks PATHS, not labels",
        m_enum.allocs,
    );

    // The same family through the public factored surface a bot actually calls.
    let family = MoveFamily::MovementRoutes;
    let (served, m_family) = meter::measure(|| iter_family(&s, &rules, &family).count());
    assert_eq!(
        served, labels,
        "smoke: without Station/Tram the family IS the walked label set, so the bound below is \
         stated against the same `labels`",
    );
    assert!(
        m_family.allocs <= bound,
        "smoke: iter_family(MovementRoutes) allocated {} for {labels} labels (> {bound})",
        m_family.allocs,
    );

    println!(
        "ROUTE BOUND, per-edit smoke (7 areas, 3 opponent Presidents, 2 canal edges):\n  \
         {labels} labels, bound {bound} = 2·labels + 32\n  \
         enumerate_routes:            {} allocs, {} peak outstanding bytes\n  \
         iter_family(MovementRoutes): {} allocs, {} peak outstanding bytes",
        m_enum.allocs, m_enum.peak_bytes, m_family.allocs, m_family.peak_bytes,
    );
}

/// The dense `plain` cell's label count as OBSERVED by
/// `action_source_equivalence.rs`'s release battery (whose ordered gate pins
/// this same construction's label LIST) — asserted here only as a `>=` FLOOR,
/// set below the observation so a benign label-space change is not a false
/// failure while an emptied enumerator still trips it. 458 is an observation
/// under that harness, NEVER a ceiling (spec F5/L9).
const DENSE_FLOOR: usize = 400;

/// RELEASE (`#[ignore]`d) — the dense route cell: hold the bound at hundreds of
/// labels, then PAY the frozen reference's ceiling ONCE under the same meter and
/// RECORD the gap the enumerator buys. Both sides return the same
/// `(destination, witness path)` pairs, so the recorded figures differ in COST
/// alone — that is the discharge figure.
///
/// `#[ignore]`d for the reference payment: the frozen DFS on a money-saturated
/// 17-area board is the expensive half, and its cost is a recorded MEASUREMENT,
/// not a per-edit correctness property (the per-edit smoke above holds the same
/// bound at a fixture the debug-profile lex-drift `debug_assert` can afford).
///
/// MEASURED 2026-07-25, release profile, this Mac — 458 labels, bound 948:
///
/// |                                       | allocations | peak outstanding bytes | time |
/// |---------------------------------------|-------------|------------------------|------|
/// | `enumerate_routes` (factored)         |         467 |                 34,196 | 3.5–4.9 ms |
/// | `enumerate_president_routes` (frozen) |   1,687,537 |                 92,924 | 182–194 ms |
///
/// **3,613× fewer allocations for the identical label list** — the recorded
/// discharge figure. The allocation and byte columns were bit-identical across
/// runs (the meter's count is deterministic by construction); the time column is
/// the observed run-to-run spread, which is why no timing here is a threshold.
///
/// All three stable figures reproduce the spec §0 measurements taken OUT OF REPO
/// (458 labels, 467 vs 1,687,537 allocations, 3,613×) to the digit, and survived
/// unchanged when 3.2 Task 3 moved the construction into `tests/route_fixtures`
/// (re-measured there and equal). That is evidence the fixture is the
/// equivalence binary's — NOT evidence that 458 is any kind of maximum (spec
/// F5/L9: the label space is unsearched).
///
/// Whole release tier of this binary: 0.20 s for both `#[ignore]`d tests
/// (0.05–0.07 s before this one was added).
#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): the dense route cell (17-area staggered board, interior Canal, money-saturated) pays the frozen reference DFS's ceiling ONCE to record the gap the O(labels) enumerator buys — a recorded-curve measurement, not a per-edit correctness property (the per-edit smoke above holds the same bound at a debug-affordable fixture); run with cargo test --release -p yokohama-core --test action_source_worstcase -- --ignored before merges/phase closes"]
fn route_reference_ceiling_recorded_and_enumeration_bounded() {
    let rules = Rules::standard();
    // The equivalence battery's `plain` cell: start 0, opponents [3,8,17], no
    // techs — literally the same construction whose label list that gate pins,
    // now that both binaries read it from `tests/route_fixtures` (spec §4-2).
    let s = dense_17_area_cell();
    assert!(fits_fixed_arrays(&s), "the dense cell must take the fixed-array path");

    // The enumerator, metered — the walk-only surface the allocation claim is
    // about, on the same state the equivalence gate pins the label LIST of.
    let t = std::time::Instant::now();
    let (labels, m_enum) = meter::measure(|| enumerate_routes(&s, &rules).len());
    let enum_time = t.elapsed();

    // PAY the frozen reference's ceiling ONCE, under the same meter. This single
    // payment does double duty — it is the anti-vacuity oracle for `labels`
    // (asserted next) AND the recorded discharge ceiling (printed below, never
    // asserted: it is the cost this module exists to avoid, not a property, and
    // a threshold on it would be a machine-speed gate).
    let t = std::time::Instant::now();
    let (reference_len, m_reference) = meter::measure(|| reference_label_count(&s, &rules, "dense plain"));
    let reference_time = t.elapsed();

    // ANTI-VACUITY BEFORE THE BOUND (spec §4-2): no allocation claim is made
    // until the label count is pinned to the frozen reference and the floor.
    assert_route_labels(labels, reference_len, DENSE_FLOOR, "dense plain");
    let bound = route_bound(labels);
    assert!(
        m_enum.allocs <= bound,
        "dense plain: enumerate_routes allocated {} for {labels} labels (> {bound} = 2·labels + \
         32) — a count that tracks PATHS, not labels",
        m_enum.allocs,
    );

    // The FAMILY surface at the same cell (T4 review's strengthening): assembly
    // adds only a handful of allocations over the enumerator (479 observed at
    // 458 labels), so the whole-family claim is gated at BOTH tiers, not
    // smoke-only. No teleport tech on this cell ⇒ the family IS the walked set.
    let (family_moves, m_family) =
        meter::measure(|| iter_family(&s, &rules, &MoveFamily::MovementRoutes).count());
    assert_eq!(family_moves, labels, "dense plain: the family serves exactly the label set");
    assert!(
        m_family.allocs <= bound,
        "dense plain: iter_family(MovementRoutes) allocated {} for {labels} labels (> {bound})",
        m_family.allocs,
    );

    println!(
        "ROUTE CEILING vs O(labels) ENUMERATION (release; dense `plain` cell — 4P staggered \
         pyramid, interior Canal, 17 own-assistant areas, money 1000).\n  \
         {labels} labels (floor {DENSE_FLOOR}; an observation under this harness, never a \
         ceiling), bound {bound} = 2·labels + 32\n  \
         enumerate_routes (factored):           {} allocs, {} peak outstanding bytes, {enum_time:.2?}\n  \
         iter_family(MovementRoutes):           {} allocs, {} peak outstanding bytes\n  \
         enumerate_president_routes (frozen):   {} allocs, {} peak outstanding bytes, {reference_time:.2?}\n  \
         allocation ratio: {}x — the recorded discharge figure. Times and bytes are RECORDED, \
         never asserted.",
        m_enum.allocs,
        m_enum.peak_bytes,
        m_family.allocs,
        m_family.peak_bytes,
        m_reference.allocs,
        m_reference.peak_bytes,
        m_reference.allocs / m_enum.allocs.max(1),
    );
}

// ---------------------------------------------------------------------------
// BOUND 3 — ONE POLICY PICK at the retreat ceiling (Phase 3.2 Task 5; spec
// §0-R2(b), §8). A THIRD shape again, and it is the one that makes §0-R2's
// honest reframing executable.
//
// R2 dropped "option-count tails bounded" — no measured shape keeps the tails
// below six figures and the mechanism explains why — and replaced it with a
// RECORDED tail plus a BOUNDED per-decision COST. Bounds 1 and 2 above bound
// the two families in isolation; this one bounds what a bot actually does at a
// whole decision point: build a handle and make ONE pick, at the largest retreat
// family a real 4P game can present. If that is bounded, the six-figure tail is
// a counting observation rather than a cost, which is exactly what R2 claims.
//
// TWO CELLS, and the second is not a duplicate (T2 review finding 1 → spec §8):
//
//   (a) President ON BOARD — the L10 DFS counter covers this shape, so the pick
//       bound is a second, independent reading of it.
//   (b) President IN HAND — the routes family emits `PlacePresident` and walks
//       NOTHING, so the counter is structurally BLIND here while the SAME
//       442,368-member retreat family is dispatched. Under uniform play 88.6%
//       of organic Movement points have this shape, so this cell is not a
//       corner: it is the common case, and this bound is its only cost cover.
//
// The two cells' bound SHAPES differ for the same reason the file's other two
// do. Cell (a) pays a route walk, whose allocations are linear in the labels it
// returns, so its bound carries a `2·routes_len` term. Cell (b) has no walk at
// all, so a constant is the honest statement and a labels term would be
// theatre.
// ---------------------------------------------------------------------------

/// Cell (a)'s bound: the route walk's `O(labels)` term (bound 2's shape) plus a
/// constant for the handle, the family `Vec`, and the drawn move's own map.
fn pick_bound_on_board(routes_len: usize) -> usize {
    2 * routes_len + 128
}

/// Cell (b)'s bound: a CONSTANT. No route walk exists at an in-hand Movement
/// point — `movement_routes_family`'s from-hand arm emits one `PlacePresident`
/// per eligible area and never searches — so nothing here scales with anything,
/// and 128 is the same headroom class as cell (a)'s constant term rather than a
/// separately-tuned number.
const PICK_BOUND_IN_HAND: usize = 128;

/// The in-hand cell's routes family, stated from the FIXTURE's construction
/// rather than read back from the code under test: `retreat_worst_case_cell`
/// puts an own assistant on every one of the 17 areas, no opponent President is
/// on the board (they are all in hand, `PlayerState::new`'s default) so no area
/// is blocked and every fee is 0. The from-hand arm therefore emits exactly one
/// `PlacePresident` per area, in ascending id order.
fn expected_in_hand_routes() -> Vec<Move> {
    (0..17u8).map(|i| Move::PlacePresident { destination: AreaId::new(i) }).collect()
}

/// The retreat family at `state`, with its closed-form size — the anti-vacuity
/// half both cells share: a bound on a pick that had no exploding family to
/// avoid would say nothing at all.
fn assert_retreat_ceiling(state: &GameState, rules: &Rules, label: &str) -> MoveFamily {
    let retreat = families(state, rules)
        .into_iter()
        .find(|f| matches!(f, MoveFamily::MovementRetreat { .. }))
        .unwrap_or_else(|| panic!("{label}: Movement must dispatch a retreat family"));
    assert_eq!(
        family_len(state, rules, &retreat),
        442_368,
        "{label}: the pick must be metered at the 3³·2¹⁴ ceiling, not a smaller family",
    );
    retreat
}

/// PER-EDIT (not ignored) — THE PICK BOUND: constructing a `DecisionHandle` and
/// making ONE `RolloutPolicy::FamilyWeighted` pick at a decision point whose
/// retreat family has 442,368 members costs a bounded allocation COUNT, at both
/// President locations.
///
/// Anti-vacuity FIRST at both cells (spec §8's standing discipline): the retreat
/// family really is 442,368 (closed form, no materialization), and the routes
/// family really is non-empty — pinned on cell (a) against the FROZEN
/// `enumerate_president_routes` on the same state, and on cell (b) against the
/// move list the fixture's own construction dictates.
///
/// TWO POLICY SETTINGS per cell, so the exploding branch is metered by
/// construction rather than by luck: `retreat_ppm = 1_000_000` takes the retreat
/// family with probability 1, and the shipped
/// [`RolloutPolicy::FAMILY_WEIGHTED_DEFAULT`] is metered beside it. Both are
/// bounded; the forced one is the reading the claim rests on.
///
/// Bytes and wall clock are RECORDED (printed), never asserted — the standing
/// meter discipline: allocation COUNT is deterministic, the other two are
/// machine-variable.
///
/// MEASURED at HEAD (debug, this Mac):
///
/// | cell | routes family | forced-retreat pick | shipped-default pick | bound |
/// |------|---------------|---------------------|----------------------|-------|
/// | (a) President on board | 1 walked label | 16 allocs, 601 B peak | 11 allocs, 545 B | 130 |
/// | (b) President in hand | 17 `PlacePresident` | 15 allocs, 1,720 B peak | 9 allocs, 1,608 B | 128 |
///
/// Both cells sit an order of magnitude under their bounds, which is the
/// headroom the constants buy — they are not tuned to the measurement.
///
/// MUTATION-PROVEN 2026-07-26 by routing the metered pick through the exhaustive
/// generator instead of the handle — `get_legal_moves(&s, &rules)` then taking
/// its last element, which is a `ReturnPresidentToHand` on both cells, so the
/// mutation returns the same KIND of move on the same state and only the
/// measured COUNT moves:
///
/// - cell (a): **1,017,597** allocations against a bound of 130 — **7,827×** —
///   and the assertion FAILED;
/// - cell (b): **1,017,589** against 128 — **7,949×** — likewise FAILED (measured
///   with cell (a) reverted, since (a)'s assertion fires first).
///
/// Both reverted by inverse edit (L7). That gap is the whole of §0-R2(b) in one
/// number: the tail is six figures and the cost of one decision is not.
#[test]
fn family_weighted_pick_at_the_retreat_ceiling_is_bounded_allocation_count() {
    let rules = Rules::standard();
    let forced = RolloutPolicy::FamilyWeighted { retreat_ppm: 1_000_000, recovery_min_of: 5 };

    // ---- cell (a): President ON BOARD ----
    let on_board = retreat_worst_case_cell(false);
    assert_retreat_ceiling(&on_board, &rules, "cell (a) on board");
    let routes_len = family_len(&on_board, &rules, &MoveFamily::MovementRoutes);
    assert_eq!(
        routes_len,
        reference_label_count(&on_board, &rules, "cell (a) on board"),
        "cell (a): the routes family must be the frozen reference's enumeration",
    );
    assert!(routes_len >= 1, "cell (a): anti-vacuity — the routes family must be non-empty");
    let bound_a = pick_bound_on_board(routes_len);

    let mut rng = ChaCha8Rng::seed_from_u64(20_260_726);
    let (forced_a, m_forced_a) = meter::measure(|| {
        let handle = DecisionHandle::new(&on_board, &rules);
        forced.pick(&handle, &mut rng)
    });
    assert!(
        matches!(forced_a, Some(Move::ReturnPresidentToHand { .. })),
        "cell (a): retreat_ppm = 1_000_000 must take the retreat family, got {forced_a:?}",
    );
    assert!(
        m_forced_a.allocs <= bound_a,
        "cell (a): a forced-retreat pick allocated {} (> {bound_a} = 2·{routes_len} + 128) — a pick \
         that scales with the 442,368 product",
        m_forced_a.allocs,
    );

    let (default_a, m_default_a) = meter::measure(|| {
        let handle = DecisionHandle::new(&on_board, &rules);
        RolloutPolicy::FAMILY_WEIGHTED_DEFAULT.pick(&handle, &mut rng)
    });
    assert!(default_a.is_some(), "cell (a): a Movement point always yields a move");
    assert!(
        m_default_a.allocs <= bound_a,
        "cell (a): a shipped-default pick allocated {} (> {bound_a})",
        m_default_a.allocs,
    );

    // ---- cell (b): President IN HAND (the counter-blind shape) ----
    let in_hand = retreat_worst_case_cell(true);
    assert_retreat_ceiling(&in_hand, &rules, "cell (b) in hand");
    let in_hand_routes: Vec<Move> =
        iter_family(&in_hand, &rules, &MoveFamily::MovementRoutes).collect();
    assert_eq!(
        in_hand_routes,
        expected_in_hand_routes(),
        "cell (b): the from-hand arm must emit one PlacePresident per own-assistant area",
    );

    let (forced_b, m_forced_b) = meter::measure(|| {
        let handle = DecisionHandle::new(&in_hand, &rules);
        forced.pick(&handle, &mut rng)
    });
    assert!(
        matches!(forced_b, Some(Move::ReturnPresidentToHand { .. })),
        "cell (b): retreat_ppm = 1_000_000 must take the retreat family, got {forced_b:?}",
    );
    assert!(
        m_forced_b.allocs <= PICK_BOUND_IN_HAND,
        "cell (b): a forced-retreat pick allocated {} (> {PICK_BOUND_IN_HAND}) at an in-hand \
         Movement point, where the L10 counter is blind and this bound is the only cost cover",
        m_forced_b.allocs,
    );

    let (default_b, m_default_b) = meter::measure(|| {
        let handle = DecisionHandle::new(&in_hand, &rules);
        RolloutPolicy::FAMILY_WEIGHTED_DEFAULT.pick(&handle, &mut rng)
    });
    assert!(default_b.is_some(), "cell (b): a Movement point always yields a move");
    assert!(
        m_default_b.allocs <= PICK_BOUND_IN_HAND,
        "cell (b): a shipped-default pick allocated {} (> {PICK_BOUND_IN_HAND})",
        m_default_b.allocs,
    );

    println!(
        "ONE-PICK BOUND at the 442,368-member retreat family (handle construction + one pick):\n  \
         cell (a) President ON BOARD, {routes_len} route label(s), bound {bound_a} = 2·labels + 128\n    \
           forced-retreat:  {} allocs, {} peak outstanding bytes\n    \
           shipped default: {} allocs, {} peak outstanding bytes\n  \
         cell (b) President IN HAND, {} PlacePresident moves, no route walk, bound {PICK_BOUND_IN_HAND}\n    \
           forced-retreat:  {} allocs, {} peak outstanding bytes\n    \
           shipped default: {} allocs, {} peak outstanding bytes\n  \
         Bytes are RECORDED, never asserted.",
        m_forced_a.allocs,
        m_forced_a.peak_bytes,
        m_default_a.allocs,
        m_default_a.peak_bytes,
        in_hand_routes.len(),
        m_forced_b.allocs,
        m_forced_b.peak_bytes,
        m_default_b.allocs,
        m_default_b.peak_bytes,
    );
}
