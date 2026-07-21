//! Stage-3 worst-case battery: the three-way pruning-soundness check on a
//! CONSTRUCTED adversarial state on the real staggered topology, plus the
//! affordability boundary cases the monotone-cutoff argument turns on.
//!
//! TWO TIERS (Task 4, 2026-07-21 — honest, no silent cap). The BOARD is the real
//! 4P staggered pyramid (Task 1), rows 6/5/4/3, with the Canal at a fully
//! interior cell — so its six brick neighbours plus nine any-to-any crossings
//! form a COMPLETE K6 subgraph (all 15 pairs joined), the maximal staggered
//! stress a board can present to the pruning enumerator. Two batteries assert the
//! SAME three-way-equality property at different own-assistant coverage:
//!
//! - PER-EDIT SMOKE (`worst_case_smoke_battery_…`, NOT ignored): assistants cover
//!   the Canal hub's complete six-tile neighbourhood (the whole interior K6) plus
//!   the president's dense approach = 11 areas. Real staggered stress, enumerable
//!   in ~0.5s, so it runs every edit. Recorded bound 183.
//! - RELEASE GATE (`worst_case_battery_…` + `comparison_keys_…`, `#[ignore]`d):
//!   assistants cover EVERY non-Canal area (all 17) — the true maximal stressor,
//!   recorded bound 519. The pruneless reference (`yokohama-oracle`'s
//!   `route_reference.rs`) enumerates ALL simple-path orderings, bounded only by
//!   the own-assistant step rule, so an assistants-everywhere 18-node staggered
//!   graph is combinatorially explosive BY CONSTRUCTION — ~47s debug / ~3s
//!   release, hence a pre-merge gate run with `--ignored`. A queued
//!   independence-controlled follow-up will replace that DFS with an exact
//!   bitmask DP over (current, visited-set, toll-count) states, collapsing the
//!   ordering explosion; once landed the maximal state becomes per-edit-feasible.
//!
//! Recorded max label counts, all honest: 183 (per-edit smoke) · 519 (release
//! full density) · 405 (old sparse 3x6 lattice, lineage). The smoke figure is
//! below the old lattice's ONLY because its coverage is intentionally smaller for
//! speed, not because the staggered geometry is sparser — at equal coverage the
//! staggered board outbranches the lattice (e.g. 417 at 15 areas, measured), and
//! the release gate's 519 is the honest denser-graph maximum.
//!
//! This harness is openly implementation-aware (lib.rs header): states are
//! hand-built from the public type constructors exactly like the engine's
//! own fixtures — type-valid, not setup-valid (the oracle and engine are
//! total over these; the sweeps in `routes3_differential.rs` cover the
//! reachable-state distribution). The staggered board graph itself is built
//! from the engine's own `staggered_adjacency` / `derive_canal_crossings`
//! topology primitives.
//!
//! ANALYTIC-BOUND NOTE (plan mandate): guidingpath.md's Phase-3 section
//! records `enumerate_president_routes` measured max ~16 labels ORGANICALLY
//! (self-play measurement, not a bound). The constructed maxima printed by the
//! two batteries are the adversarial counterpart. Both are printed; the tests
//! assert three-way EQUALITY only — never against the organic figure, which is a
//! measurement under a policy, not a property of the rules.

//! PERF HISTORY (2026-07-20): the first battery run exposed a quadratic
//! `dedup_moves` scan in the oracle — 2^17 retreat moves at this state made
//! one `oracle_moves` call minutes long (measured 4^n scaling: 23ms/10
//! own-assistant areas, 264ms/12, 3.85s/14 debug). The orchestrator landed a
//! HashSet-keyed dedup (pinned semantically by the oracle's
//! `dedup_moves_matches_quadratic_reference`). PERF UPDATE (2026-07-21, Task 1
//! topology switch): the staggered graph's interior-Canal K6 makes the pruneless
//! reference ~47s at full 17-area density, so that maximal battery is the
//! `#[ignore]`d release gate and the per-edit tier runs the 11-area smoke (~0.5s).
//! If the smoke ever slows to MINUTES, suspect a dedup or enumeration regression
//! on the 2^17-sized retreat family before the walks.

use std::collections::BTreeMap;

use yokohama_core::{
    cell_to_area_id, derive_canal_crossings, row_lengths, staggered_adjacency, AchievementCard,
    AchievementType, AreaConfig, AreaId, AreaType, BoardLayout, BoardLayoutBuilder, BuildingSite,
    ChurchTrack, CustomsTrack, FivePowerEffect, GameConfig, GameState, LabState, MercenarySupply,
    PlayerColor, PlayerId, PlayerState, PortState, PresidentLocation, Rules, TechEffect, TurnPhase,
};
use yokohama_difftest::routes3::three_way_route_check;

/// The Canal's grid cell for the worst-case board: `(row 1, col 3)` = AreaId(9)
/// on the staggered 4P pyramid, a fully-interior tile with all SIX brick
/// neighbors. That is the maximal-neighbor placement, so the any-to-any
/// derivation yields the largest possible crossing set (see
/// [`staggered_board_4p_with_canal`]).
const WORST_CASE_CANAL: (u8, u8) = (1, 3);

/// The real 4-player STAGGERED pyramid (rulebook p.3 rows 6/5/4/3, 18 tiles —
/// the topology the generator produces since Task 1), with the Canal at `canal`.
///
/// This harness is openly implementation-aware (lib.rs header), so the graph is
/// built from the engine's own topology primitives rather than re-derived here:
/// brick-offset adjacency from [`staggered_adjacency`], and the ANY-TO-ANY Canal
/// transit pairs from [`derive_canal_crossings`] (every canal-neighbor pair not
/// already brick-adjacent). Each crossing is added to BOTH adjacency (so a walk
/// can cross the Canal without stopping on it) and `canal_crossings` (so the
/// cross tolls 1 Yen to the bank). AreaIds are row-major, top row first.
///
/// With the Canal at a fully-interior cell it has all six neighbors, so
/// C(6,2)=15 neighbor pairs minus the 6 brick-adjacent ones = NINE tolled
/// crossings — the maximal crossing fan a real board can carry, and the point of
/// the worst case: the densest legal graph for the route-label enumerator.
fn staggered_board_4p_with_canal(canal: (u8, u8)) -> BoardLayout {
    let rows = row_lengths(4); // [6, 5, 4, 3]
    // Area types are irrelevant to movement except `allows_placement` (Tram
    // destinations); a rotating palette of ordinary areas keeps the board
    // plausible without claiming setup validity.
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
            let ty = if (r as u8, col) == canal {
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
    // Brick-offset base graph (engine primitive).
    for (a, z) in staggered_adjacency(&rows) {
        b.add_adjacency(a, z);
    }
    // Any-to-any Canal transit pairs (engine primitive): adjacency + toll.
    for (a, z) in derive_canal_crossings(canal, &rows) {
        b.add_adjacency(a, z);
        b.add_canal_crossing(a, z);
    }
    b.build().expect("staggered worst-case board validates")
}

/// The worst-case board: staggered 4P pyramid, Canal at [`WORST_CASE_CANAL`].
fn worst_case_board() -> BoardLayout {
    staggered_board_4p_with_canal(WORST_CASE_CANAL)
}

/// Fidelity pin (Task 4): the worst-case board is the real 4P STAGGERED
/// pyramid (rows 6/5/4/3) with an INTERIOR Canal carrying all nine any-to-any
/// crossings — the maximal crossing fan a real board can hold. The old 3x6
/// lattice gave rows 6/6/6 and only four colinear crossings; this pin fails
/// loudly on any regression back to lattice geometry.
#[test]
fn worst_case_board_is_the_staggered_pyramid_not_a_lattice() {
    let board = worst_case_board();
    let mut per_row: BTreeMap<u8, usize> = BTreeMap::new();
    for cfg in board.areas.values() {
        *per_row.entry(cfg.grid_position.0).or_insert(0) += 1;
    }
    assert_eq!(
        per_row.into_values().collect::<Vec<_>>(),
        vec![6, 5, 4, 3],
        "row shape must be the staggered pyramid, not a rectangular lattice",
    );
    assert_eq!(
        board.canal_crossings.len(),
        9,
        "an interior Canal carries nine any-to-any crossings (the lattice had four)",
    );
    for &(a, z) in &board.canal_crossings {
        assert!(board.adjacent_areas(a).contains(&z), "crossing {a:?}~{z:?} must be adjacency too");
        assert!(board.is_canal_crossing(a, z));
    }
    // A brick notch edge the rectangular 3x6 lattice can never carry: the tile
    // at (1,0) touches BOTH (0,0) and its notch-neighbor (0,1).
    let rows = row_lengths(4);
    let id = |rc: (u8, u8)| cell_to_area_id(rc.0, rc.1, &rows);
    assert!(
        board.adjacent_areas(id((1, 0))).contains(&id((0, 1))),
        "brick notch (1,0)~(0,1) missing — lattice geometry leaked",
    );
}

/// Hand-built `n`-player Movement state on `board`: current player 0, empty
/// queue, presidents in hand (tests place them). Same construction as the
/// engine-side fixtures — type-valid by the public constructors.
fn movement_state(players: u8, board: BoardLayout) -> GameState {
    let colors = [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green];
    GameState {
        config: GameConfig::for_player_count(players),
        board_layout: board,
        players: (0..players)
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
    }
}

fn tech_id(rules: &Rules, effect: TechEffect) -> yokohama_core::TechCardId {
    rules
        .techs
        .iter()
        .find(|t| t.effect == effect)
        .map(|t| t.id)
        .expect("standard data carries the tech")
}

/// Shared constructor: the worst-case staggered board, mover's president at the
/// corner (0,0), own assistants on `assistant_ids`, three opponent presidents on
/// `opponents` (REACHABLE tiles ⇒ crossing-adjacent pass-through fee diversity),
/// money huge so no affordability cutoff bites (the boundary tests own that).
fn worst_case_state_with(assistant_ids: &[u8], opponents: [u8; 3]) -> GameState {
    let mut s = movement_state(4, worst_case_board());
    s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0)); // corner (0,0)
    for &id in assistant_ids {
        s.players[0].assistants.on_board.insert(AreaId::new(id), 1);
    }
    for (i, &opp) in opponents.iter().enumerate() {
        s.players[i + 1].president_location = PresidentLocation::OnBoard(AreaId::new(opp));
    }
    s.players[0].money = 1_000;
    s
}

/// MAXIMAL release-tier state (used only behind `#[ignore]`, see module SCOPE):
/// own assistants on EVERY non-Canal area (all 17), so the whole interior-Canal
/// K6 is live and the mover can walk the entire pyramid. The true maximal
/// route-label stressor — bound 519 — but the pruneless reference's all-orderings
/// walk is ~47s in debug, so it runs as a release gate, not per-edit.
fn worst_case_state() -> GameState {
    let canal = cell_to_area_id(WORST_CASE_CANAL.0, WORST_CASE_CANAL.1, &row_lengths(4)).0;
    let ids: Vec<u8> = (0..18u8).filter(|&id| id != canal).collect();
    worst_case_state_with(&ids, [3, 8, 17])
}

/// FEASIBLE per-edit SMOKE state (see module SCOPE): own assistants on the Canal
/// hub's COMPLETE six-tile neighbourhood {3,4,8,10,13,14} (the whole interior
/// K6) plus the president's dense upper-row approach {0,1,2,6,7} = 11 areas. The
/// K6 stays fully live (real staggered stress), while the walk space stays
/// enumerable in ~1s so the three-way check runs every edit. Opponents on
/// reachable Canal flanks (3, 8) + interior (7) for pass-through fee diversity.
fn smoke_state() -> GameState {
    worst_case_state_with(&[0, 1, 2, 3, 4, 6, 7, 8, 10, 13, 14], [3, 8, 7])
}

/// Run the Stagecoach/Postal 2x2 + one Tram+Station tech-matrix battery on
/// `base`, asserting FULL three-way equality (engine / pruned oracle / pruneless
/// reference) in every cell and returning the max label count. `station_at` must
/// be a covered own-assistant area (RULED §7.1-A1 Reading-2 gate): AreaId(14), a
/// far tile, is in BOTH the maximal and the smoke coverage.
fn run_three_way_battery(base: &GameState, station_at: AreaId, rules: &Rules) -> usize {
    let postal = tech_id(rules, TechEffect::PostalSystem);
    let stage = tech_id(rules, TechEffect::Stagecoach);
    let tram = tech_id(rules, TechEffect::Tram);
    let station = tech_id(rules, TechEffect::Station);
    let cells: [(&str, Vec<yokohama_core::TechCardId>, Option<AreaId>); 5] = [
        ("plain", vec![], None),
        ("postal", vec![postal], None),
        ("stagecoach", vec![stage], None),
        ("postal+stagecoach", vec![postal, stage], None),
        ("tram+station", vec![tram, station], Some(station_at)),
    ];
    let mut max_labels = 0usize;
    let mut cases = 0usize;
    for (name, techs, station_loc) in cells {
        let mut s = base.clone();
        s.players[0].technologies.extend(techs.iter().copied());
        s.station_location = station_loc;
        let cmp = three_way_route_check(&s, rules)
            .expect("constructed Movement state must qualify");
        assert!(!cmp.is_divergent(), "{name}: three-way divergence: {:#?}", cmp.diverged);
        assert!(cmp.label_count() > 0, "{name}: vacuously empty label set");
        println!("  cell {name}: {} labels", cmp.label_count());
        max_labels = max_labels.max(cmp.label_count());
        cases += 1;
    }
    assert_eq!(cases, 5);
    max_labels
}

/// RELEASE-GATE battery (maximal, `#[ignore]`d — the two-tier policy): FULL
/// three-way equality on the full-density 17-area state in every tech cell,
/// RECORDING the true maximal label bound (519). ~47s in debug (interior-Canal
/// K6 × the all-orderings pruneless reference), so it runs with `-- --ignored`
/// before merges/phase closes; the per-edit smoke below covers the same property
/// fast. No silent cap — the three honest bounds are full-density 519 (here) ·
/// the feasible smoke (below) · the old 3x6 lattice 405 (lineage). A queued
/// bitmask-DP follow-up will make full density per-edit-feasible.
#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): full-density 17-area interior-Canal K6 (~47s pruneless walk, bound 519); run with --release before merges/phase closes"]
fn worst_case_battery_three_way_equal_and_bound_recorded() {
    let rules = Rules::standard();
    let max_labels = run_three_way_battery(&worst_case_state(), AreaId::new(14), &rules);
    println!(
        "CONSTRUCTED full-density worst-case max label count: {max_labels} \
         (release gate; the per-edit smoke records its own bound; old 3x6 lattice was 405; \
         organic self-play max ~16 — guidingpath Phase 3)"
    );
}

/// PER-EDIT SMOKE battery (the debug tier of the worst case): the SAME FULL
/// three-way equality property as the release gate above, on the feasible
/// [`smoke_state`] (the complete interior-Canal K6 + president approach, 11
/// areas), so it runs EVERY edit in ~1s. RECORDS its own bound. The maximal 519
/// battery lives behind `#[ignore]` as a release gate; no silent cap — the three
/// honest bounds are this smoke's max (printed) · full-density 519 · old lattice
/// 405.
#[test]
fn worst_case_smoke_battery_three_way_equal_and_bound_recorded() {
    let rules = Rules::standard();
    let max_labels = run_three_way_battery(&smoke_state(), AreaId::new(14), &rules);
    println!(
        "CONSTRUCTED smoke worst-case max label count: {max_labels} \
         (per-edit tier; release-gate full density = 519; old 3x6 lattice = 405; \
         organic self-play max ~16 — guidingpath Phase 3)"
    );
}

/// (c) affordability boundaries, president-fee flavor: spur board where the
/// cheapest fee-bearing route costs exactly 1 Yen. Money 0 keeps only the
/// free label; money exactly 1 adds the fee-bearing labels; three-way equal
/// at both boundaries (this is where a cutoff off-by-one would live).
#[test]
fn boundary_president_fee_money_zero_and_exactly_one() {
    let rules = Rules::standard();
    // Line 0—1—2 with a free spur 0—3; Blue's president on 1.
    let mut b = BoardLayoutBuilder::new();
    for (id, ty) in [
        (0u8, AreaType::CopperMine),
        (1, AreaType::SilkMill),
        (2, AreaType::TeaPlantation),
        (3, AreaType::FishingGround),
    ] {
        let a = AreaId::new(id);
        b.add_area(a, AreaConfig { area_type: ty, grid_position: (0, id) });
        b.set_building_site(a, BuildingSite::new([1, 1, 1, 1], 3));
        b.set_five_power_token(a, FivePowerEffect::Money(1));
    }
    b.add_adjacency(AreaId::new(0), AreaId::new(1));
    b.add_adjacency(AreaId::new(1), AreaId::new(2));
    b.add_adjacency(AreaId::new(0), AreaId::new(3));
    let board = b.build().expect("boundary board validates");

    let mut s = movement_state(2, board);
    s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
    for a in [1u8, 2, 3] {
        s.players[0].assistants.on_board.insert(AreaId::new(a), 1);
    }
    s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));

    // Money 0: the pass-through fee to reach 2 is unaffordable — only the
    // free spur label to 3 survives, on all three sides.
    s.players[0].money = 0;
    let cmp = three_way_route_check(&s, &rules).expect("qualifies");
    assert!(!cmp.is_divergent(), "money=0 diverged: {:#?}", cmp.diverged);
    assert_eq!(cmp.label_count(), 1, "money=0 keeps exactly the free label: {:?}", cmp.engine);

    // Money exactly 1: the cheapest fee-bearing route (through Blue to 2)
    // becomes affordable at exact equality.
    s.players[0].money = 1;
    let cmp = three_way_route_check(&s, &rules).expect("qualifies");
    assert!(!cmp.is_divergent(), "money=1 diverged: {:#?}", cmp.diverged);
    assert_eq!(
        cmp.label_count(),
        2,
        "money=1 adds exactly the 1-Yen route: {:?}",
        cmp.engine
    );
}

/// (c) affordability boundaries, bank-toll flavor: both edges of a 0—1—2
/// line are Canal crossings, so labels cost 1 and 2 Yen to the BANK. Money
/// 0 → empty on all three sides; money exactly 1 → only the one-toll label.
#[test]
fn boundary_canal_toll_money_zero_and_exactly_one() {
    let rules = Rules::standard();
    let mut b = BoardLayoutBuilder::new();
    for (id, ty) in [
        (0u8, AreaType::CopperMine),
        (1, AreaType::SilkMill),
        (2, AreaType::TeaPlantation),
    ] {
        let a = AreaId::new(id);
        b.add_area(a, AreaConfig { area_type: ty, grid_position: (0, id) });
        b.set_building_site(a, BuildingSite::new([1, 1, 1, 1], 3));
        b.set_five_power_token(a, FivePowerEffect::Money(1));
    }
    b.add_adjacency(AreaId::new(0), AreaId::new(1));
    b.add_adjacency(AreaId::new(1), AreaId::new(2));
    b.add_canal_crossing(AreaId::new(0), AreaId::new(1));
    b.add_canal_crossing(AreaId::new(1), AreaId::new(2));
    let board = b.build().expect("toll board validates");

    let mut s = movement_state(2, board);
    s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
    s.players[0].assistants.on_board.insert(AreaId::new(1), 1);
    s.players[0].assistants.on_board.insert(AreaId::new(2), 1);

    s.players[0].money = 0;
    let cmp = three_way_route_check(&s, &rules).expect("qualifies");
    assert!(!cmp.is_divergent(), "money=0 diverged: {:#?}", cmp.diverged);
    assert_eq!(cmp.label_count(), 0, "money=0: every route tolls, so no labels: {:?}", cmp.engine);

    s.players[0].money = 1;
    let cmp = three_way_route_check(&s, &rules).expect("qualifies");
    assert!(!cmp.is_divergent(), "money=1 diverged: {:#?}", cmp.diverged);
    assert_eq!(
        cmp.label_count(),
        1,
        "money=1 affords exactly the one-toll label: {:?}",
        cmp.engine
    );
}

/// Non-qualifying states return `None`: wrong phase, queued choice, or the
/// President in hand.
#[test]
fn non_qualifying_states_return_none() {
    let rules = Rules::standard();

    // President in hand.
    let s = movement_state(2, worst_case_board());
    assert!(three_way_route_check(&s, &rules).is_none(), "in-hand president must not qualify");

    // Wrong phase.
    let mut s = worst_case_state();
    s.turn_phase = TurnPhase::AreaAction;
    assert!(three_way_route_check(&s, &rules).is_none(), "non-Movement phase must not qualify");

    // Queued choice.
    let mut s = worst_case_state();
    s.pending_choices = vec![yokohama_core::PendingChoice::AnyGood];
    assert!(three_way_route_check(&s, &rules).is_none(), "queued choice must not qualify");
}

/// The retreat-family and other non-`MovePresident` Movement moves never leak
/// into the comparison keys (the check compares the LABEL family only) — a
/// clean adversarial state's key sets are pure `route:` keys.
#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): runs on the full-density 17-area state (~10s pruneless walk); run with --release before merges/phase closes"]
fn comparison_keys_are_route_labels_only() {
    let rules = Rules::standard();
    let cmp = three_way_route_check(&worst_case_state(), &rules).expect("qualifies");
    let all = BTreeMap::from([
        ("engine", &cmp.engine),
        ("pruned", &cmp.pruned),
        ("pruneless", &cmp.pruneless),
    ]);
    for (side, keys) in all {
        assert!(
            keys.iter().all(|k| k.starts_with("route:")),
            "{side} side leaked a non-label key: {:?}",
            keys.iter().find(|k| !k.starts_with("route:"))
        );
    }
}
