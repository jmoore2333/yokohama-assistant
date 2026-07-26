#![cfg(debug_assertions)]
//! L10: ONE President-route enumeration per Movement decision point
//! (Phase 3.2 Task 2; spec `docs/plans/2026-07-25-phase3.2-arena-spec.md` §8,
//! §0-R2(a)).
//!
//! `DecisionHandle` promises that a decision point's routes family is
//! enumerated at most once however many of `size`/`iter`/`sample`/`volume` read
//! it — the promise that lets the policy (Task 5) and the volume metric
//! (Task 4) be two consumers of one enumeration. Call-site discipline cannot
//! hold that; `route_enum`'s DFS counter can, and this file is its pin.
//!
//! ## Why this file is DEBUG tier, not release
//!
//! `#![cfg(debug_assertions)]` at the crate root, so in a release build this
//! binary is empty. The counter lives behind `cfg(any(test, debug_assertions))`
//! — and `cfg(test)` is FALSE for the library as an integration test links it,
//! so in `cargo test --release` the accessor is the unconditional 0-returning
//! stub and every assertion here would read 0 vacuously. Debug is where the
//! instrument exists, and per-edit is where an enumerate-once guarantee wants
//! checking, so this is a per-edit debug test with no release twin.
//!
//! ## What the counter does and does NOT catch (spec §8, both caveats)
//!
//! It fires on any President-route DFS, from either entry point (the frozen
//! `movement::enumerate_president_routes` or `route_enum`'s fixed-array
//! search), so a stray `get_legal_moves` at a Movement point **with the
//! President ON BOARD** takes the count to 2. That on-board qualifier is the
//! counter's whole reach (T2 review finding 1): at an IN-HAND Movement point
//! the routes family emits `PlacePresident` and walks nothing, so a stray
//! materialization runs zero DFSes and is invisible here — even though those
//! points dispatch the same exploding `MovementRetreat` family (88.6% of
//! organic Movement points are in-hand under uniform play). The exploding
//! family's cost cover at in-hand points is the R2(b) pick allocation bound
//! (Task 5), not this counter. At every other phase the delegated families
//! call `get_legal_moves` underneath BY DESIGN and no route DFS fires there,
//! so `== 0` off-Movement is a statement about route enumeration, not about
//! list materialization.
//!
//! The assertion is **harness-scoped, never armed globally**. The CLI bot path
//! legitimately pays 2 in debug (the policy's enumeration plus `Session::apply`'s
//! materializing legality gate, which spec §9 deliberately keeps); nothing in
//! the engine asserts on this counter outside this file and
//! `search_actions`' unit pin.
//!
//! ## Mutation proof
//!
//! Both arms of the assertion were shown to discriminate, by inserting a second
//! `family_len(&state, rules, &MoveFamily::MovementRoutes)` call into
//! `drive_and_pin`'s per-decision block (debug, 2026-07-26, at this file's
//! HEAD):
//!
//! - guarded to Movement points (`if want == 1`), it took the count to 2 and
//!   the test FAILED at `seed 0 2P turn 4 phase Movement: expected 1 route DFS,
//!   counted 2` — the enumerate-once arm;
//! - unguarded, it fired even earlier, at `seed 0 2P turn 2 phase AreaAction:
//!   expected 0 route DFS, counted 1` — the off-Movement arm, which catches a
//!   route enumeration at a phase that should pay none (`movement_routes_family`
//!   reads the President's location, never `turn_phase`, so it will happily
//!   search off-phase).
//!
//! Both reverted by inverse edit (L7 — never `git checkout -- <file>`); the
//! file below is the reverted state.

mod common;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use yokohama_core::game::{
    cell_to_area_id, derive_canal_crossings, enumerate_president_routes, families, family_len,
    fits_fixed_arrays, get_legal_moves, is_game_over, route_dfs_count, route_dfs_reset,
    row_lengths, staggered_adjacency, DecisionHandle, FamilySize, MoveFamily,
};
use yokohama_core::{
    apply_move_unchecked, AreaConfig, AreaId, AreaType, BoardLayout, BoardLayoutBuilder,
    BuildingSite, FivePowerEffect, GameState, Move, PlayerId, PresidentLocation, Rules, TurnPhase,
};

// ---------------------------------------------------------------------------
// Anti-vacuity fixture: the dense 17-area cell
//
// Task 3 swaps this for tests/route_fixtures
// ---------------------------------------------------------------------------

/// The Canal's grid cell: `(row 1, col 3)` on the 4P staggered pyramid, the
/// fully-interior tile whose any-to-any transit makes its neighbourhood a
/// complete K6 — the densest graph a real board can present to the enumerator.
/// Same cell `action_source_equivalence.rs`'s dense battery and
/// `yokohama-difftest`'s `routes3_worstcase.rs` use.
const DENSE_CANAL: (u8, u8) = (1, 3);

/// The real 4P staggered pyramid (rows 6/5/4/3, 18 tiles) with the Canal at
/// [`DENSE_CANAL`], built from the engine's own topology primitives so the
/// fixture cannot drift from the board the generator produces.
fn dense_board() -> BoardLayout {
    let rows = row_lengths(4);
    let palette = [
        AreaType::CopperMine,
        AreaType::SilkMill,
        AreaType::TeaPlantation,
        AreaType::FishingGround,
        AreaType::Bank,
        AreaType::Chinatown,
    ];
    let mut b = BoardLayoutBuilder::new();
    let mut idx = 0u8;
    for (r, &len) in rows.iter().enumerate() {
        for col in 0..len {
            let id = AreaId::new(idx);
            let ty = if (r as u8, col) == DENSE_CANAL {
                AreaType::Canal
            } else {
                palette[(idx as usize) % palette.len()]
            };
            b.add_area(id, AreaConfig { area_type: ty, grid_position: (r as u8, col) });
            if ty != AreaType::Canal {
                b.set_building_site(id, BuildingSite::new([1, 1, 1, 1], 3));
                b.set_five_power_token(id, FivePowerEffect::Money(1));
            }
            idx += 1;
        }
    }
    for (a, z) in staggered_adjacency(&rows) {
        b.add_adjacency(a, z);
    }
    for (a, z) in derive_canal_crossings(DENSE_CANAL, &rows) {
        b.add_adjacency(a, z);
        b.add_canal_crossing(a, z);
    }
    b.build().expect("dense staggered board validates")
}

/// A dense Movement state on [`dense_board`]: the mover's President on area 0,
/// own assistants on every non-Canal area (so the whole interior is walkable),
/// the three opponents' Presidents on 3/8/17, and 1,000 Yen so no affordability
/// cutoff bites. Money is the enumerator's cost driver, and this cell exists to
/// be expensive.
fn dense_state(rules: &Rules) -> GameState {
    let rows = row_lengths(4);
    let canal = cell_to_area_id(DENSE_CANAL.0, DENSE_CANAL.1, &rows);

    let (mut s, _rng) = common::new_seeded_game(0, 4, rules);
    s.pending_choices.clear();
    s.turn_phase = TurnPhase::Movement;
    s.board_layout = dense_board();
    s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
    s.players[0].assistants.on_board.clear();
    for id in (0..18u8).filter(|&id| AreaId::new(id) != canal) {
        s.players[0].assistants.on_board.insert(AreaId::new(id), 1);
    }
    for (i, opp) in [3u8, 8, 17].iter().enumerate() {
        s.players[i + 1].president_location = PresidentLocation::OnBoard(AreaId::new(*opp));
    }
    s.players[0].money = 1_000;
    s.current_player = PlayerId::new(0);
    s
}

/// ANTI-VACUITY, and it runs FIRST inside the pin below: the family the counter
/// meters is the FROZEN reference's own enumeration, at a label count no
/// degenerate or short-circuited search could produce.
///
/// It is a helper rather than its own `#[test]` on purpose — as a precondition
/// of the driven pin it cannot be skipped, and running it twice would double
/// the file's dominant cost (see the timing note in the pin's doc).
fn assert_dense_fixture_is_real(rules: &Rules) -> usize {
    let s = dense_state(rules);
    assert!(fits_fixed_arrays(&s), "the dense cell must take the fixed-array (non-delegating) path");

    let reference =
        enumerate_president_routes(&s, PlayerId::new(0), AreaId::new(0), Some(&rules.techs));
    let handle = DecisionHandle::new(&s, rules);
    let served = handle.size(&MoveFamily::MovementRoutes);

    // FLOOR, never a ceiling (L9: the route-label space is unsearched; this
    // harness reproduced 458 here, the same figure `action_source_equivalence`'s
    // `plain` dense cell records). The walked routes are the whole family on
    // this fixture — no Station, no Tram — so the handle's routes count IS the
    // reference's length.
    assert!(
        reference.len() >= 400,
        "anti-vacuity floor: the dense cell enumerated only {} labels",
        reference.len(),
    );
    assert_eq!(
        served,
        FamilySize::Exact(reference.len() as u64),
        "the handle's routes family must be the frozen reference's enumeration",
    );
    reference.len()
}

// ---------------------------------------------------------------------------
// The pin: one enumeration per Movement decision point, none elsewhere
// ---------------------------------------------------------------------------

/// What a decision point at `state` should cost in route DFSes, derived from
/// the STATE rather than from `families()` — so the pin is not checking the
/// dispatch against itself.
///
/// A Movement point with the President IN HAND costs ZERO: the routes family is
/// then `movement_routes_family`'s `PlacePresident` arm, which walks nothing.
/// That asymmetry is real and is asserted, not smoothed over — the coverage
/// tally below pins that both Movement arms were reached.
fn expected_route_dfs(state: &GameState) -> u64 {
    let on_board =
        matches!(state.current_player().president_location, PresidentLocation::OnBoard(_));
    u64::from(
        state.pending_choices.is_empty() && state.turn_phase == TurnPhase::Movement && on_board,
    )
}

/// Decision-point counts a driven game reached, so the pin can assert it
/// exercised both arms rather than passing on an all-zero game.
#[derive(Default, Debug)]
struct Coverage {
    movement_on_board: u64,
    movement_in_hand: u64,
    other: u64,
}

/// Drive one full game, a `DecisionHandle` per decision point, asserting the
/// route-DFS count after every decision.
///
/// The pick is size-weighted across the families and then `handle.sample`
/// within one — a DRIVER, not a policy: placement is weighted by its honest
/// UPPER bound, so the walk is not exactly uniform over the legal list. Task 5
/// owns policy semantics; all this needs is to visit decision points the way a
/// bot will.
fn drive_and_pin(seed: u64, players: u8, rules: &Rules) -> Coverage {
    let (mut state, _) = common::new_seeded_game(seed, players, rules);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut coverage = Coverage::default();

    // A DECISION cap, not a turn cap — a turn spends several decisions. Sized
    // well clear of what these two seeds need (the 4P game is the longer, at
    // ~8.9k decisions), and the post-loop assertion below is what makes hitting
    // it a failure rather than a quietly-truncated game.
    const MAX_DECISIONS: u32 = 40_000;

    for _ in 0..MAX_DECISIONS {
        if is_game_over(&state) {
            break;
        }

        let want = expected_route_dfs(&state);
        route_dfs_reset();

        let handle = DecisionHandle::new(&state, rules);
        assert_eq!(
            route_dfs_count(),
            0,
            "seed {seed} {players}P turn {}: building the handle must enumerate nothing",
            state.turn_number,
        );

        // `volume()` reads every family's size — the routes term included, so
        // the enumeration happens HERE and every later read must reuse it.
        let (_total, _has_upper) = handle.volume();
        let mut weights: Vec<u64> =
            handle.families().iter().map(|f| handle.size(f).value()).collect();
        assert!(!weights.is_empty(), "a decision point must dispatch at least one family");

        let mv = loop {
            let total: u64 = weights.iter().sum();
            assert!(total > 0, "seed {seed} {players}P: every family empty at a decision point");
            let mut pick = rng.gen_range(0..total);
            let i = weights
                .iter()
                .position(|&w| {
                    if pick < w {
                        true
                    } else {
                        pick -= w;
                        false
                    }
                })
                .expect("the weighted index is below the total");
            match handle.sample(&handle.families()[i], &mut rng) {
                Some(mv) => break mv,
                // A structurally-emptyable family that `size` said was
                // non-empty cannot happen; zeroing its weight keeps the driver
                // total rather than trusting that.
                None => weights[i] = 0,
            }
        };

        assert_eq!(
            route_dfs_count(),
            want,
            "seed {seed} {players}P turn {} phase {:?}: expected {want} route DFS, counted {} \
             (Movement points with the President on board pay exactly 1; every other decision \
             point pays 0)",
            state.turn_number,
            state.turn_phase,
            route_dfs_count(),
        );

        match (want, state.turn_phase) {
            (1, _) => coverage.movement_on_board += 1,
            (0, TurnPhase::Movement) if state.pending_choices.is_empty() => {
                coverage.movement_in_hand += 1
            }
            _ => coverage.other += 1,
        }

        state = apply_move_unchecked(&state, &mv, rules)
            .unwrap_or_else(|e| panic!("seed {seed} {players}P: sampled {mv:?} failed to apply: {e:?}"));
    }

    // These are FULL games by claim, so make the claim executable: a run that
    // exhausted the cap would still satisfy every count assertion above while
    // silently pinning only a prefix of the game.
    assert!(
        is_game_over(&state),
        "seed {seed} {players}P: the game hit the decision cap without ending ({coverage:?})",
    );
    coverage
}

/// THE pin. Anti-vacuity first (the dense cell's labels against the frozen
/// reference), then two full driven games — one 2P, one 4P — asserting after
/// every decision point that exactly one route enumeration happened where the
/// state says one should, and none anywhere else.
///
/// Cost (debug, measured 2026-07-26 at this HEAD): 5.3s single-threaded for the
/// whole file, and the dense anti-vacuity cell is the larger share of this
/// test — one frozen-reference enumeration at 458 labels, plus the fixed-array
/// enumerator's O(prefixes × labels) lex-drift `debug_assert`. That is why the
/// anti-vacuity runs ONCE, as this test's precondition, rather than also
/// standing as its own test: a second run would roughly double the file.
#[test]
fn every_movement_decision_point_pays_exactly_one_route_enumeration() {
    let rules = Rules::standard();
    let labels = assert_dense_fixture_is_real(&rules);
    println!("dense 17-area cell: {labels} route labels (observation, never a ceiling)");

    let mut total = Coverage::default();
    for (seed, players) in [(0u64, 2u8), (1, 4)] {
        let c = drive_and_pin(seed, players, &rules);
        println!("seed {seed} {players}P: {c:?}");
        total.movement_on_board += c.movement_on_board;
        total.movement_in_hand += c.movement_in_hand;
        total.other += c.other;
    }

    // Anti-vacuity for the assertion itself: both arms were really exercised.
    assert!(total.movement_on_board > 0, "no on-board Movement point was reached: {total:?}");
    assert!(total.movement_in_hand > 0, "no in-hand Movement point was reached: {total:?}");
    assert!(total.other > 0, "no non-Movement decision point was reached: {total:?}");
}

/// A two-area Movement state with one walkable edge: President on 0, an own
/// assistant on `far`, 5 Yen. `far` is `AreaId(18)` for the delegating cell —
/// an id past `route_enum::MAX_AREAS`, which no generated board reaches
/// (`route_enum`'s `every_generated_board_fits_the_fixed_arrays` pins that), so
/// the tuple constructor is used deliberately where `AreaId::new` would refuse.
///
/// Small on purpose: the property under test is that ONE search bumps ONCE, and
/// density buys it nothing while costing tens of seconds in debug through the
/// frozen path.
fn two_area_movement_state(far: AreaId, rules: &Rules) -> GameState {
    let (mut s, _) = common::new_seeded_game(0, 2, rules);
    s.pending_choices.clear();
    s.turn_phase = TurnPhase::Movement;
    s.board_layout = BoardLayout {
        areas: [
            (AreaId::new(0), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 0) }),
            (far, AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) }),
        ]
        .into_iter()
        .collect(),
        adjacency: [(AreaId::new(0), vec![far]), (far, vec![AreaId::new(0)])].into_iter().collect(),
        canal_crossings: vec![],
        building_sites: Default::default(),
        five_power_tokens: Default::default(),
    };
    s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
    s.players[0].assistants.on_board.clear();
    s.players[0].assistants.on_board.insert(far, 1);
    s.players[0].money = 5;
    s.players[1].president_location = PresidentLocation::InHand;
    s.current_player = PlayerId::new(0);
    s
}

/// The counter is provenance-blind: whichever entry point runs the search, ONE
/// search counts ONCE.
///
/// The load-bearing arm is the third. `walk_routes_costed`'s delegating arm
/// forwards INTO the frozen `enumerate_president_routes` rather than searching
/// itself, so it must not bump on its own account; if it did, a delegated
/// decision point would read 2 and the pin above would be asserting the wrong
/// constant. The bump therefore sits after that early return, and this test is
/// what says so executably.
#[test]
fn one_dfs_counts_once_from_either_entry_point() {
    let rules = Rules::standard();
    let fixed = two_area_movement_state(AreaId::new(1), &rules);
    assert!(fits_fixed_arrays(&fixed), "the small board must take the fixed-array path");

    route_dfs_reset();
    let frozen =
        enumerate_president_routes(&fixed, PlayerId::new(0), AreaId::new(0), Some(&rules.techs));
    assert_eq!(route_dfs_count(), 1, "the frozen search bumps exactly once");
    assert!(!frozen.is_empty(), "anti-vacuity: the frozen search must have enumerated something");

    route_dfs_reset();
    let served = family_len(&fixed, &rules, &MoveFamily::MovementRoutes);
    assert_eq!(route_dfs_count(), 1, "the fixed-array search bumps exactly once");
    assert_eq!(served, frozen.len(), "and it is the same enumeration");

    let oversized = two_area_movement_state(AreaId(18), &rules);
    assert!(!fits_fixed_arrays(&oversized), "an id past MAX_AREAS must fail the pre-flight");

    route_dfs_reset();
    let delegated = family_len(&oversized, &rules, &MoveFamily::MovementRoutes);
    assert_eq!(
        route_dfs_count(),
        1,
        "the delegating arm must not bump on top of the frozen search's own bump",
    );
    assert_eq!(delegated, 1, "anti-vacuity: the delegated walk must serve its one route");

    // §0-R2(a)'s detection mechanism, pinned rather than inferred (T2 review
    // finding 3): the exhaustive generator itself pays exactly one route DFS at
    // an on-board Movement point, which is what makes "a stray get_legal_moves
    // takes a policy-harness count to 2" true rather than assumed.
    route_dfs_reset();
    let exhaustive = get_legal_moves(&fixed, &rules);
    assert_eq!(
        route_dfs_count(),
        1,
        "get_legal_moves at an on-board Movement point costs exactly one route DFS",
    );
    assert!(!exhaustive.is_empty(), "anti-vacuity: the exhaustive list must be non-empty");
}

/// `families()` dispatch is the only tag producer this pin trusts, so the
/// handle's families must BE `families()`' — otherwise the walk above could be
/// visiting decision points through a shorter tag list and still counting 1.
#[test]
fn the_driver_walks_real_decision_points() {
    let rules = Rules::standard();
    let (state, _) = common::new_seeded_game(0, 2, &rules);
    let handle = DecisionHandle::new(&state, &rules);
    let tags = families(&state, &rules);
    assert_eq!(handle.families(), tags.as_slice(), "the handle's families are families()'");
    let served: Vec<Move> = tags.iter().flat_map(|f| handle.iter(f)).collect();
    assert!(!served.is_empty(), "the opening decision point must serve moves");
}
