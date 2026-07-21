//! `repro` dispatch (Codex PR-3 review, P1): the repro core must re-run the
//! comparison THAT KIND of artifact stores — dispatched on `phase` — not the
//! Stage-1 enumeration on everything. Before the dispatch existed, a
//! perfectly fresh `FinalScoring` or `Routes3` artifact failed to reproduce
//! its own stored diff (`repro --strict` falsely exited 3), because the
//! enumeration comparison was being diffed against scoring/routes3 lines.
//!
//! These tests build REAL artifacts with the production writers on driven
//! game states and assert the dispatch reproduces each stored diff
//! byte-identically.

use yokohama_core::{is_game_over, GameState, Rules};
use yokohama_difftest::artifact::{
    default_artifacts_dir, read_artifact, write_artifact, DivergenceArtifact,
    ARTIFACT_FORMAT_VERSION,
};
use yokohama_difftest::drive::{drive_game, drive_game_to_end, DriveStats, Policy, StepOutcome};
use yokohama_difftest::repro::repro_artifact;
use yokohama_difftest::routes3::{
    format_pair_diffs, three_way_route_check_with_moves, RouteComparison,
};
use yokohama_difftest::runner::write_scoring_artifact;
use yokohama_difftest::scoring::compare_final;

/// A FRESH `FinalScoring` artifact — the real Stage-2 writer on a real
/// driven terminal state — must reproduce its own stored diff
/// byte-identically under repro, and report the CURRENT scoring divergence
/// verdict (clean here). The pre-dispatch path re-ran the Stage-1
/// enumeration instead: on a terminal state that yields empty key sets,
/// which can never match the stored `"P<id>: <total>"` ranked-totals lines,
/// so `--strict` falsely failed.
#[test]
fn fresh_scoring_artifact_reproduces_byte_identically() {
    let rules = Rules::standard();
    let (stats, final_state) =
        drive_game_to_end(0, 2, Policy::Uniform, &rules, None, |_, _, _| Ok(StepOutcome::Continue))
            .expect("seed 0, 2p, uniform drives to completion");
    assert!(is_game_over(&final_state));
    let cmp = compare_final(&final_state, &rules).expect("terminal state compares");
    assert!(!cmp.is_divergent(), "engine and oracle scoring agree on master: {cmp:?}");

    // The production writer, on a distinctive seed so no sweep artifact
    // collides; a clean comparison still writes both sides' full answers.
    let stats = DriveStats { seed: u64::MAX - 3, ..stats };
    let dir = default_artifacts_dir();
    let path = write_scoring_artifact(
        stats.seed, stats.players, stats.policy, &stats, &final_state, &cmp, &dir,
    )
    .expect("write scoring artifact");
    let artifact = read_artifact(&path).expect("read back");
    std::fs::remove_file(&path).expect("cleanup");
    assert_eq!(artifact.phase, "FinalScoring");
    assert!(!artifact.engine_only.is_empty(), "stored lines are the full ranked totals");

    let report = repro_artifact(&artifact, &rules).expect("repro runs");
    assert_eq!(
        report.engine_only, artifact.engine_only,
        "fresh scoring re-run must produce the stored engine ranked-totals lines"
    );
    assert_eq!(report.oracle_only, artifact.oracle_only);
    assert_eq!(report.duplicates, artifact.duplicates);
    assert!(
        report.reproduces(&artifact),
        "a FRESH scoring artifact must reproduce byte-identically (strict exit 3 was the bug)"
    );
    assert!(!report.divergent, "scoring is clean, so repro must not claim a current divergence");
}

/// Drive until a state qualifies for the three-way route check and hand it
/// back with its (clean, on master) comparison.
fn qualifying_routes3_state(rules: &Rules) -> (GameState, RouteComparison, u64) {
    let mut captured: Option<(GameState, RouteComparison, u64)> = None;
    drive_game(0, 2, Policy::Uniform, rules, Some(600), |state, moves, step| {
        if let Some(cmp) = three_way_route_check_with_moves(state, rules, moves) {
            captured = Some((state.clone(), cmp, step));
            return Ok(StepOutcome::Stop);
        }
        Ok(StepOutcome::Continue)
    })
    .expect("drive");
    captured.expect("600 steps of a 2p game must reach a qualifying Movement state")
}

/// A `Routes3` artifact must be re-run through the THREE-WAY route check —
/// the report shows all three surfaces (engine / pruned-oracle /
/// pruneless-reference label sets) and its fresh diff is the
/// `format_pair_diffs` line format, byte-identical to what the Stage-3
/// writer stores. The pre-dispatch path printed the Stage-1 enumeration
/// instead, which never mentions the oracle's two route surfaces at all.
#[test]
fn routes3_artifact_repro_reruns_the_three_way_check() {
    let rules = Rules::standard();
    let (state, cmp, step) = qualifying_routes3_state(&rules);
    assert!(!cmp.is_divergent(), "three-way check is clean on master");
    assert!(cmp.label_count() > 0, "a qualifying state offers at least one route label");

    let (engine_only, oracle_only) = format_pair_diffs(&cmp.diverged);
    let artifact = DivergenceArtifact {
        format_version: ARTIFACT_FORMAT_VERSION,
        seed: u64::MAX - 4, // distinctive: cannot collide with sweep artifacts
        players: 2,
        policy: "uniform".to_string(),
        step_index: step,
        turn_number: state.turn_number,
        phase: "Routes3".to_string(),
        state_json: serde_json::to_value(&state).expect("serialize state"),
        engine_only,
        oracle_only,
        duplicates: Vec::new(),
    };
    let dir = default_artifacts_dir();
    let path = write_artifact(&artifact, &dir).expect("write");
    let artifact = read_artifact(&path).expect("read back");
    std::fs::remove_file(&path).expect("cleanup");

    let report = repro_artifact(&artifact, &rules).expect("repro runs");
    assert!(
        report.reproduces(&artifact),
        "a FRESH routes3 artifact must reproduce byte-identically; fresh engine_only={:?} oracle_only={:?} dups={:?}",
        report.engine_only, report.oracle_only, report.duplicates
    );
    assert!(!report.divergent, "the three-way check is clean, so no current divergence");
    // The report must actually be the three-way comparison: every surface
    // named, every engine label listed.
    for surface in ["engine", "pruned-oracle", "pruneless-reference"] {
        assert!(
            report.lines.iter().any(|l| l.contains(surface)),
            "repro of a Routes3 artifact must show the {surface} label set; lines:\n{}",
            report.lines.join("\n")
        );
    }
    for key in &cmp.engine {
        assert!(
            report.lines.iter().any(|l| l.contains(key.as_str())),
            "engine label {key} missing from the routes3 repro report"
        );
    }
}

/// A `Routes3` artifact whose stored diff no longer reproduces (a historical
/// fixture after a fix) is NOT an error and NOT a current divergence: the
/// fresh three-way check is clean, so `reproduces` is false while
/// `divergent` stays false — exactly the exit-code split (non-strict 0,
/// strict 3) the bin contract promises.
#[test]
fn routes3_artifact_with_stale_stored_diff_is_clean_but_not_reproduced() {
    let rules = Rules::standard();
    let (state, cmp, step) = qualifying_routes3_state(&rules);
    assert!(!cmp.is_divergent());

    let artifact = DivergenceArtifact {
        format_version: ARTIFACT_FORMAT_VERSION,
        seed: u64::MAX - 5,
        players: 2,
        policy: "uniform".to_string(),
        step_index: step,
        turn_number: state.turn_number,
        phase: "Routes3".to_string(),
        state_json: serde_json::to_value(&state).expect("serialize state"),
        // A historical diff that no longer reproduces (same line format the
        // Stage-3 writer uses).
        engine_only: vec![
            "[engine vs pruned-oracle] engine-only: route:RouteLabel { .. }".to_string(),
        ],
        oracle_only: Vec::new(),
        duplicates: Vec::new(),
    };
    let report = repro_artifact(&artifact, &rules).expect("repro runs");
    assert!(!report.reproduces(&artifact), "the stale stored diff must not reproduce");
    assert!(!report.divergent, "fresh three-way check is clean");
}

/// A `FinalScoring` artifact whose snapshot is NOT terminal cannot be
/// re-scored — that is an `Err` (the bin's exit 2), never a silent fallback
/// to the enumeration comparison.
#[test]
fn scoring_artifact_with_non_terminal_state_is_an_error() {
    let rules = Rules::standard();
    let (state, _cmp, step) = qualifying_routes3_state(&rules); // any mid-game state
    assert!(!is_game_over(&state));
    let artifact = DivergenceArtifact {
        format_version: ARTIFACT_FORMAT_VERSION,
        seed: u64::MAX - 6,
        players: 2,
        policy: "uniform".to_string(),
        step_index: step,
        turn_number: state.turn_number,
        phase: "FinalScoring".to_string(),
        state_json: serde_json::to_value(&state).expect("serialize state"),
        engine_only: vec!["P0: 1".to_string()],
        oracle_only: vec!["P0: 1".to_string()],
        duplicates: Vec::new(),
    };
    let err = repro_artifact(&artifact, &rules)
        .expect_err("a non-terminal FinalScoring snapshot must be an error, not an enumeration re-run");
    assert!(
        err.contains("terminal"),
        "the error must say the state is not terminal: {err}"
    );
}
