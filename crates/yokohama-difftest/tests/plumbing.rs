//! Harness plumbing smoke tests (NOT ignored — these run in the per-edit
//! debug suite): the driver itself must work engine-only before any
//! differential claim means anything.

use yokohama_core::{check_invariants, route_fee_vector, Move, Rules};
use yokohama_difftest::compare::{engine_route_label, route_label_key};
use yokohama_difftest::drive::{drive_game, new_seeded_game, Policy, StepOutcome};

/// Drive one short game (200 steps, seed 0, 2p, uniform) engine-only.
/// `drive_game` checks `check_invariants` after every apply internally, so a
/// clean `Ok` IS the invariants-stay-clean assertion; the observer re-checks
/// per decision point anyway so a violation names the step it surfaced at.
#[test]
fn smoke_short_uniform_game_keeps_invariants_clean() {
    let rules = Rules::standard();
    let stats = drive_game(0, 2, Policy::Uniform, &rules, Some(200), |state, moves, step| {
        assert!(!moves.is_empty(), "no dead ends at step {step}");
        check_invariants(state, &rules).map_err(|e| format!("invariant dirty at step {step}: {e}"))?;
        Ok(StepOutcome::Continue)
    })
    .expect("short uniform game drives cleanly");
    assert_eq!(stats.steps, 200, "the cap, not the game end, should stop a 200-step slice");
}

/// The retreat-averse policy drives real games too (same plumbing, the
/// other selectable policy).
#[test]
fn smoke_short_retreat_averse_game_drives_cleanly() {
    let rules = Rules::standard();
    let stats = drive_game(1, 2, Policy::RetreatAverse, &rules, Some(100), |_, _, _| Ok(StepOutcome::Continue))
        .expect("short retreat-averse game drives cleanly");
    assert!(stats.steps > 0);
}

/// Engine-side route-label projection agrees with the engine's own pricing
/// on a real state: the label's parts must re-sum to `route_fee_vector`'s
/// total (the engine's claim), and an empty path (teleport encoding) must
/// price to the all-zero label.
#[test]
fn engine_route_label_matches_engine_pricing_on_a_real_state() {
    let rules = Rules::standard();
    let (state, _rng) = new_seeded_game(0, 2, &rules).expect("setup");

    // Walk the driver forward until a state actually offers MovePresident.
    let mut found = false;
    drive_game(0, 2, Policy::Uniform, &rules, Some(300), |state, moves, _| {
        for m in moves {
            if let Move::MovePresident { path, destination } = m {
                let label = engine_route_label(state, &rules, path, *destination);
                let vector =
                    route_fee_vector(state, state.current_player, path, Some(&rules.techs), false);
                let total: u32 = vector.values().sum();
                assert_eq!(label.mover_cost(), total, "label cost must be the engine's own sum");
                assert_eq!(label.fees, vector, "the label's fee map IS the engine's fee vector");
                assert_eq!(label.destination, *destination);
                found = true;
            }
        }
        Ok(StepOutcome::Continue)
    })
    .expect("drive");
    assert!(found, "300 steps of a 2p game must offer at least one MovePresident");

    // Teleport encoding: empty path prices to the all-zero label.
    let some_area = *state.board_layout.areas.keys().next().expect("board has areas");
    let label = engine_route_label(&state, &rules, &[], some_area);
    assert_eq!(label.mover_cost(), 0);
    assert!(label.fees.is_empty());
    let _: String = route_label_key(&label); // key building never panics
}

/// A REAL GameState survives the artifact round-trip byte-faithfully — the
/// stub round-trip in artifact.rs covers the envelope; this covers the
/// payload the whole repro contract rests on (Stage-1 review finding 2).
#[test]
fn real_state_round_trips_through_an_artifact() {
    use yokohama_difftest::artifact::{
        read_artifact, write_artifact, DivergenceArtifact, ARTIFACT_FORMAT_VERSION,
    };
    let rules = Rules::standard();
    let (state, _rng) = new_seeded_game(3, 3, &rules).expect("setup");
    let artifact = DivergenceArtifact {
        format_version: ARTIFACT_FORMAT_VERSION,
        seed: u64::MAX - 1, // cannot collide with sweep artifacts
        players: 3,
        policy: "uniform".to_string(),
        step_index: 0,
        turn_number: state.turn_number,
        phase: format!("{:?}", state.turn_phase),
        state_json: serde_json::to_value(&state).expect("serialize state"),
        engine_only: vec![],
        oracle_only: vec![],
        duplicates: vec![],
        rules_hash: String::new(),
        semantics_rev: None,
    };
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("artifacts");
    let path = write_artifact(&artifact, &dir).expect("write");
    let back = read_artifact(&path).expect("read");
    let state_back: yokohama_core::GameState =
        serde_json::from_value(back.state_json.clone()).expect("state deserializes");
    assert_eq!(state, state_back, "state must survive the artifact round-trip");
    std::fs::remove_file(&path).expect("cleanup");
}

/// The committed ruling-A1 fixtures stay loadable and re-enumerable forever —
/// they are the owner-facing evidence for ruling A1 and must not rot silently
/// (Stage-1 review finding 2). Each fixture's RECORDED diff shows the
/// historical pre-ruling divergence (oracle-only Station label); the FRESH
/// comparison must be CLEAN because both sides implement Reading 2 — RULED
/// by Jeremy 2026-07-20 (decision-log row): the station token adds power
/// only, RB p.6's assistant-at-destination rule stands. These fixtures are
/// the permanent evidence of the divergence that surfaced the question.
#[test]
fn ruling_a1_fixtures_load_and_compare_clean_under_reading_2() {
    use yokohama_difftest::artifact::read_artifact;
    use yokohama_difftest::compare::compare_moves;
    let rules = Rules::standard();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/ruling-a1");
    let mut checked = 0;
    for name in [
        "6-4-retreat-averse-269.json",
        "7-4-retreat-averse-130.json",
        "7-4-uniform-30.json",
    ] {
        let artifact = read_artifact(&dir.join(name)).expect("fixture reads");
        assert_eq!(artifact.format_version, 1, "{name}: schema version");
        assert!(
            !artifact.oracle_only.is_empty(),
            "{name}: recorded diff must show the historical pre-interim divergence"
        );
        let state: yokohama_core::GameState =
            serde_json::from_value(artifact.state_json.clone())
                .unwrap_or_else(|e| panic!("{name}: state no longer deserializes — the A1 ruling evidence is rotting: {e}"));
        let engine_moves = yokohama_core::get_legal_moves(&state, &rules);
        let oracle = yokohama_oracle::oracle_moves(&state, &rules);
        let cmp = compare_moves(&state, &rules, &engine_moves, &oracle);
        assert!(
            !cmp.is_divergent(),
            "{name}: must compare CLEAN under RULED Reading 2; got engine_only={:?} oracle_only={:?} dups={:?}",
            cmp.engine_only, cmp.oracle_only, cmp.duplicates
        );
        checked += 1;
    }
    assert_eq!(checked, 3);
}
