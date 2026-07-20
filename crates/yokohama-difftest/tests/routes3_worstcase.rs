//! Stage-3 worst-case battery (NOT ignored — per-edit debug suite): the
//! three-way pruning-soundness check on CONSTRUCTED adversarial states, where
//! the walk space is maximal, plus the affordability boundary cases the
//! monotone-cutoff argument turns on.
//!
//! This harness is openly implementation-aware (lib.rs header): states are
//! hand-built from the public type constructors exactly like the engine's
//! own fixtures — type-valid, not setup-valid (the oracle and engine are
//! total over these; the sweeps in `routes3_differential.rs` cover the
//! reachable-state distribution).
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
//! `dedup_moves_matches_quadratic_reference`), and the full-coverage
//! three-way below runs in seconds. If this test ever slows to minutes
//! again, suspect a dedup or enumeration regression on a 2^17-sized family
//! before suspecting the walks.

use std::collections::BTreeMap;

use yokohama_core::{
    AchievementCard, AchievementType, AreaConfig, AreaId, AreaType, BoardLayout,
    BoardLayoutBuilder, BuildingSite, ChurchTrack, CustomsTrack, FivePowerEffect, GameConfig,
    GameState, LabState, MercenarySupply, PlayerColor, PlayerId, PlayerState, PortState,
    PresidentLocation, Rules, TechEffect, TurnPhase,
};
use yokohama_difftest::routes3::three_way_route_check;

/// 3x6 grid board, row-major `AreaId(r*6+c)`, mirroring `board_gen`'s real
/// construction: grid-orthogonal adjacency everywhere, and the Canal cell at
/// `canal` contributing its colinear flank crossing pairs — (L,R), (U,D) and
/// BOTH diagonals — each added to BOTH adjacency and canal_crossings
/// (crossings are transit edges, the Canal is never a node on a path).
/// With the Canal at an interior cell of the middle row, all FOUR pairs
/// exist — the maximal crossing fan a real board can carry.
fn grid_board_3x6_with_canal(canal: (u8, u8)) -> BoardLayout {
    const ROWS: u8 = 3;
    const COLS: u8 = 6;
    // Area types are irrelevant to movement except `allows_placement`
    // (Tram destinations); a rotating palette of ordinary areas keeps the
    // board plausible without claiming setup validity.
    let palette = [
        AreaType::CopperMine,
        AreaType::SilkMill,
        AreaType::TeaPlantation,
        AreaType::FishingGround,
        AreaType::Bank,
        AreaType::Chinatown,
    ];
    let mut b = BoardLayoutBuilder::new();
    for r in 0..ROWS {
        for c in 0..COLS {
            let id = AreaId::new(r * COLS + c);
            let ty = if (r, c) == canal {
                AreaType::Canal
            } else {
                palette[((r * COLS + c) as usize) % palette.len()]
            };
            b.add_area(id, AreaConfig { area_type: ty, grid_position: (r, c) });
            if ty != AreaType::Canal {
                b.set_building_site(id, BuildingSite::new([1, 1, 1, 1], 3));
                b.set_five_power_token(id, FivePowerEffect::Money(1));
            }
        }
    }
    for r in 0..ROWS {
        for c in 0..COLS {
            let id = AreaId::new(r * COLS + c);
            if c + 1 < COLS {
                b.add_adjacency(id, AreaId::new(r * COLS + c + 1));
            }
            if r + 1 < ROWS {
                b.add_adjacency(id, AreaId::new((r + 1) * COLS + c));
            }
        }
    }
    // Canal crossing pairs: colinear through-lines only, both flanks must
    // exist (box-ruled geometry, CLAUDE.md Canal row).
    let (cr, cc) = (canal.0 as i16, canal.1 as i16);
    let flank = |r: i16, c: i16| -> Option<AreaId> {
        (r >= 0 && r < ROWS as i16 && c >= 0 && c < COLS as i16)
            .then(|| AreaId::new((r as u8) * COLS + c as u8))
    };
    let pairs = [
        ((cr, cc - 1), (cr, cc + 1)),     // (L, R)
        ((cr - 1, cc), (cr + 1, cc)),     // (U, D)
        ((cr - 1, cc - 1), (cr + 1, cc + 1)), // diagonal
        ((cr - 1, cc + 1), (cr + 1, cc - 1)), // anti-diagonal
    ];
    let mut crossing_count = 0;
    for ((r1, c1), (r2, c2)) in pairs {
        if let (Some(a), Some(z)) = (flank(r1, c1), flank(r2, c2)) {
            b.add_adjacency(a, z);
            b.add_canal_crossing(a, z);
            crossing_count += 1;
        }
    }
    assert_eq!(crossing_count, 4, "the constructed Canal must carry all four crossing pairs");
    b.build().expect("worst-case board validates")
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

/// The adversarial 4p state: Canal at (1,2) with all four crossing pairs,
/// mover's president at corner (0,0), mover assistants on EVERY non-Canal
/// area, the three opponent presidents scattered (two on crossing flanks,
/// one far corner), money huge so no cutoff ever bites.
fn worst_case_state() -> GameState {
    let canal = (1u8, 2u8);
    let mut s = movement_state(4, grid_board_3x6_with_canal(canal));
    let canal_id = AreaId::new(canal.0 * 6 + canal.1);
    s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
    for id in 0..18u8 {
        let a = AreaId::new(id);
        if a != canal_id {
            s.players[0].assistants.on_board.insert(a, 1);
        }
    }
    // Opponent presidents scattered: (1,1)=7 and (0,2)=2 flank the Canal
    // (crossing-adjacent fee interactions), (2,4)=16 sits far away.
    s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(7));
    s.players[2].president_location = PresidentLocation::OnBoard(AreaId::new(2));
    s.players[3].president_location = PresidentLocation::OnBoard(AreaId::new(16));
    s.players[0].money = 1_000;
    s
}

/// (a) + (b): the battery across Stagecoach/Postal on/off (plus one
/// Tram+Station cell exercising the shared teleport surface), asserting FULL
/// three-way equality (engine / pruned oracle / pruneless reference) on the
/// mandated full-coverage 17-area state in every cell, and RECORDING the
/// constructed maximum label count next to the organic ~16 claim (a
/// measurement, not a bound — printed, never asserted against).
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
    // cell (station on a far flank; every area holds an own assistant, so
    // the PENDING-RULING interim gate passes).
    let cells: [(&str, Vec<yokohama_core::TechCardId>, Option<AreaId>); 5] = [
        ("plain", vec![], None),
        ("postal", vec![postal], None),
        ("stagecoach", vec![stage], None),
        ("postal+stagecoach", vec![postal, stage], None),
        ("tram+station", vec![tram, station], Some(AreaId::new(15))),
    ];

    for (name, techs, station_at) in cells {
        let mut s = worst_case_state();
        s.players[0].technologies.extend(techs.iter().copied());
        s.station_location = station_at;
        let cmp = three_way_route_check(&s, &rules)
            .expect("constructed Movement state must qualify");
        assert!(
            !cmp.is_divergent(),
            "{name}: three-way divergence on the full 17-area state: {:#?}",
            cmp.diverged
        );
        assert!(cmp.label_count() > 0, "{name}: vacuously empty label set");
        println!("worst-case cell {name}: {} labels (full 17-area state)", cmp.label_count());
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
    let s = movement_state(2, grid_board_3x6_with_canal((1, 2)));
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
