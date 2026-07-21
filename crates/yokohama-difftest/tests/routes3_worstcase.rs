//! Stage-3 worst-case battery (per-edit debug suite): the three-way
//! pruning-soundness check on a CONSTRUCTED adversarial state on the real
//! staggered topology, plus the affordability boundary cases the monotone-cutoff
//! argument turns on.
//!
//! SCOPE / BOUND (Task 4, 2026-07-21 — honest, no silent cap). The BOARD is the
//! real 4P staggered pyramid (Task 1), rows 6/5/4/3, with the Canal at a fully
//! interior cell — so its six brick neighbours plus nine any-to-any crossings
//! form a COMPLETE K6 subgraph (all 15 pairs joined), the maximal staggered
//! stress a board can present to the pruning enumerator. The adversarial STATE
//! bounds the mover's own-assistant coverage to a dense-but-feasible induced
//! subgraph: the TOP THREE pyramid rows (ids 0..=14 less the Canal = 14 areas),
//! which contains the whole Canal K6 neighbourhood plus the dense upper core. It
//! deliberately does NOT cover the full-density board (all 17 non-Canal areas
//! steppable): the pruneless reference (`yokohama-oracle`'s `route_reference.rs`)
//! enumerates ALL simple-path orderings, bounded only by the own-assistant step
//! rule, so an assistants-everywhere 18-node staggered graph is combinatorially
//! explosive BY CONSTRUCTION — measured ~47s in debug at full density vs ~4s on
//! this bounded state. A queued independence-controlled follow-up will replace
//! that DFS with an exact bitmask DP over (current, visited-set, toll-count)
//! states, collapsing the ordering explosion; once landed the bound can be
//! lifted. Recorded max label counts, all honest: 387 on this bounded state ·
//! 405 on the old sparse 3x6 lattice · 519 on the infeasible full-density
//! staggered board (printed by the battery). The bounded figure sits just below
//! the old lattice's ONLY because the coverage is intentionally smaller, not
//! because the geometry is sparser — at equal 15-area coverage the staggered
//! board already outbranches it (417, measured).
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
//! (self-play measurement, not a bound). The constructed maximum printed by
//! `worst_case_battery_three_way_equal_and_bound_recorded` is the adversarial
//! counterpart. Both are printed; the test asserts three-way EQUALITY only —
//! never against the organic figure, which is a measurement under a policy,
//! not a property of the rules.

//! PERF HISTORY (2026-07-20): the first battery run exposed a quadratic
//! `dedup_moves` scan in the oracle — 2^17 retreat moves at this state made
//! one `oracle_moves` call minutes long (measured 4^n scaling: 23ms/10
//! own-assistant areas, 264ms/12, 3.85s/14 debug). The orchestrator landed a
//! HashSet-keyed dedup (pinned semantically by the oracle's
//! `dedup_moves_matches_quadratic_reference`). PERF UPDATE (2026-07-21, Task 1
//! topology switch): the staggered graph's interior-Canal K6 makes the pruneless
//! reference ~47s at full 17-area density, so the battery runs on the bounded
//! 14-area state above (~4s). If it ever slows to MINUTES, suspect a dedup or
//! enumeration regression on the 2^17-sized retreat family before the walks.

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

/// The adversarial 4p state (BOUNDED per the module SCOPE note): Canal at the
/// interior cell (1,3)=AreaId(9) with all NINE any-to-any crossings, mover's
/// president at corner (0,0). Mover assistants cover the TOP THREE pyramid rows
/// — ids 0..=14 less the Canal = 14 own-assistant areas — which is the full
/// Canal K6 neighbourhood ({3,4,8,10,13,14}) plus the dense upper core; the
/// narrow bottom row (15/16/17) is deliberately left off so the pruneless
/// reference's all-orderings walk stays feasible per-edit (see module SCOPE).
/// Three opponent presidents sit on REACHABLE tiles for crossing-adjacent
/// pass-through fee diversity: Canal flanks (0,3)=3 and (1,2)=8, plus the far
/// reachable corner (2,0)=11. Money huge so no affordability cutoff ever bites
/// (the boundary tests own the cutoff).
fn worst_case_state() -> GameState {
    let rows = row_lengths(4);
    let canal_id = cell_to_area_id(WORST_CASE_CANAL.0, WORST_CASE_CANAL.1, &rows); // AreaId(9)
    let mut s = movement_state(4, worst_case_board());
    s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0)); // corner (0,0)
    // Top three rows (rows 0-2 of the 6/5/4/3 pyramid) = ids 0..=14, less Canal.
    for id in 0..15u8 {
        let a = AreaId::new(id);
        if a != canal_id {
            s.players[0].assistants.on_board.insert(a, 1);
        }
    }
    s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(3));
    s.players[2].president_location = PresidentLocation::OnBoard(AreaId::new(8));
    s.players[3].president_location = PresidentLocation::OnBoard(AreaId::new(11));
    s.players[0].money = 1_000;
    s
}

/// (a) + (b): the battery across Stagecoach/Postal on/off (plus one
/// Tram+Station cell exercising the shared teleport surface), asserting FULL
/// three-way equality (engine / pruned oracle / pruneless reference) on the
/// bounded 14-area induced subgraph in every cell (module SCOPE note), and
/// RECORDING the constructed maximum label count next to the organic ~16 claim
/// (a measurement, not a bound — printed, never asserted against).
#[test]
fn worst_case_battery_three_way_equal_and_bound_recorded() {
    let rules = Rules::standard();
    let postal = tech_id(&rules, TechEffect::PostalSystem);
    let stage = tech_id(&rules, TechEffect::Stagecoach);
    let tram = tech_id(&rules, TechEffect::Tram);
    let station = tech_id(&rules, TechEffect::Station);
    let mut max_labels = 0usize;
    let mut cases = 0usize;

    // Cell tech loadouts: the Stagecoach/Postal 2x2 plus one Tram+Station
    // cell. The Station sits at (2,3)=AreaId(14), a far tile INSIDE the
    // own-assistant subgraph (so the RULED §7.1-A1 Reading-2 assistant gate
    // passes — it must be a covered area, not one of the bottom-row tips).
    let cells: [(&str, Vec<yokohama_core::TechCardId>, Option<AreaId>); 5] = [
        ("plain", vec![], None),
        ("postal", vec![postal], None),
        ("stagecoach", vec![stage], None),
        ("postal+stagecoach", vec![postal, stage], None),
        ("tram+station", vec![tram, station], Some(AreaId::new(14))),
    ];

    for (name, techs, station_at) in cells {
        let mut s = worst_case_state();
        s.players[0].technologies.extend(techs.iter().copied());
        s.station_location = station_at;
        let cmp = three_way_route_check(&s, &rules)
            .expect("constructed Movement state must qualify");
        assert!(
            !cmp.is_divergent(),
            "{name}: three-way divergence on the bounded 14-area state: {:#?}",
            cmp.diverged
        );
        assert!(cmp.label_count() > 0, "{name}: vacuously empty label set");
        println!("worst-case cell {name}: {} labels (bounded 14-area induced subgraph)", cmp.label_count());
        max_labels = max_labels.max(cmp.label_count());
        cases += 1;
    }

    assert_eq!(cases, 5);
    println!(
        "CONSTRUCTED worst-case max label count: {max_labels} \
         (organic claim, guidingpath Phase 3: enumerate_president_routes measured max ~16 labels)"
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
