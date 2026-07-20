//! Terminal-state SCORING differential (Phase 2.9, Stage 2).
//!
//! The `#[ignore]`d sweep below joins the pre-merge RELEASE gates
//! (CLAUDE.md two-tier policy: release-profile, large-N, before
//! merges/phase closes — never the per-edit debug suite). Run it with:
//!
//! `cargo test --release -p yokohama-difftest --test scoring_differential -- --ignored`
//!
//! It drives 2 policies x 2p/3p/4p x 15 seeds of full seeded self-play to
//! completion and asserts the engine's `final_result` and the oracle's
//! `final_standings` agree on every player's total VP AND the full ranked
//! order (strict total order, official clockwise tie-break). Divergences
//! write `phase: "FinalScoring"` artifacts to the crate artifacts dir.
//!
//! The non-ignored tests are per-edit plumbing: one short-seed game driven
//! to its terminal state proves the drive → compare_final pipeline works
//! and that the engine's own ranking is internally consistent with its
//! totals.

use yokohama_core::{is_game_over, Rules};
use yokohama_difftest::artifact::default_artifacts_dir;
use yokohama_difftest::drive::{drive_game_to_end, new_seeded_game, Policy, StepOutcome};
use yokohama_difftest::runner::run_scoring_game;
use yokohama_difftest::scoring::{compare_final, ranking_consistent};

/// Drive ONE short-seed 2p uniform game to completion and compare at the
/// terminal state: `compare_final` must return a comparison (not `None`),
/// with one engine row per player, ranked consistently with its own totals
/// (descending — the engine-internal sanity half; oracle agreement is the
/// release sweep's job, not this test's).
#[test]
fn plumbing_one_game_drives_to_terminal_and_compares() {
    let rules = Rules::standard();
    let (stats, final_state) =
        drive_game_to_end(0, 2, Policy::Uniform, &rules, None, |_, _, _| Ok(StepOutcome::Continue))
            .expect("seed 0, 2p, uniform drives to completion");
    assert!(stats.completed, "the game must actually end, not hit a cap");
    assert!(is_game_over(&final_state), "completed:true must mean a terminal state");

    let cmp = compare_final(&final_state, &rules)
        .expect("compare_final must return Some on a terminal state");
    assert_eq!(
        cmp.engine.len(),
        final_state.players.len(),
        "one engine standing per player"
    );
    assert!(
        ranking_consistent(&cmp.engine),
        "engine ranking must be non-increasing in total VP: {:?}",
        cmp.engine
    );
}

/// Tie-break coverage census (Stage-2 gate): organic ties occur in only a
/// few percent of random games, so the clockwise tie-break would otherwise
/// ride on a handful of sweep games. This test CONSTRUCTS the tie
/// deterministically: drive a real game to terminal, then surgically add
/// money to the runner-up until both leaders' totals are equal (each 2 Yen
/// = +1 remainder VP — scoring is a pure function of the state, so state
/// surgery is sound for a scoring comparison, unlike enumeration
/// reachability), and assert both sides agree on the FULL ranked order of
/// the genuinely-tied state.
#[test]
fn constructed_tie_resolves_identically_on_both_sides() {
    let rules = Rules::standard();
    let (_stats, mut state) =
        drive_game_to_end(0, 3, Policy::Uniform, &rules, None, |_, _, _| Ok(StepOutcome::Continue))
            .expect("seed 0, 3p, uniform drives to completion");

    // Equalize the top two totals by paying the runner-up in 2-Yen steps.
    let before = compare_final(&state, &rules).expect("terminal");
    let (leader, leader_total) = before.engine[0];
    let (runner_up, runner_total) = before.engine[1];
    let deficit = leader_total - runner_total;
    let ru = state
        .players
        .iter_mut()
        .find(|p| p.id == runner_up)
        .expect("runner-up exists");
    // Round money down to an even remainder base first so +2*deficit lands
    // exactly on the leader's total.
    ru.money -= ru.money % 2;
    ru.money += 2 * deficit;

    let cmp = compare_final(&state, &rules).expect("still terminal after fungible surgery");
    assert_eq!(
        cmp.engine[0].1, cmp.engine[1].1,
        "surgery must produce a genuine top-two tie: {:?}",
        cmp.engine
    );
    assert!(
        !cmp.is_divergent(),
        "engine and oracle must rank the tied state identically:\n  engine: {:?}\n  oracle: {:?}",
        cmp.engine,
        cmp.oracle
    );
    // The tie-break is the CLOCKWISE rule, not order-of-total insertion:
    // whichever of the tied pair is closest clockwise from the start player
    // must come first on BOTH sides.
    let n = state.players.len() as u8;
    let start = state.config.start_player;
    let dist = |p: yokohama_core::PlayerId| (p.0 + n - start.0) % n;
    let expect_first =
        if dist(leader) <= dist(runner_up) { leader } else { runner_up };
    assert_eq!(
        cmp.engine[0].0, expect_first,
        "clockwise-from-start must decide the tie (start={start:?}): {:?}",
        cmp.engine
    );
}

/// `compare_final` refuses non-terminal states: scoring comparison is
/// defined ONLY at game over (a mid-game "final score" is not a thing the
/// engine or oracle claims).
#[test]
fn compare_final_is_none_on_a_non_terminal_state() {
    let rules = Rules::standard();
    let (state, _rng) = new_seeded_game(0, 2, &rules).expect("setup");
    assert!(!is_game_over(&state));
    assert!(compare_final(&state, &rules).is_none());
}

/// Release-tier scoring sweep: 2 policies x 2p/3p/4p x 15 seeds, zero
/// scoring divergences allowed. See module docs for the gate command.
#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): large-N full games; run with --release before merges/phase closes"]
fn scoring_sweep_engine_vs_oracle() {
    let rules = Rules::standard();
    let artifact_dir = default_artifacts_dir();
    let mut divergences: Vec<String> = Vec::new();

    for policy in [Policy::Uniform, Policy::RetreatAverse] {
        for players in 2..=4u8 {
            for seed in 0..15u64 {
                let outcome = run_scoring_game(seed, players, policy, &rules, &artifact_dir)
                    .unwrap_or_else(|e| {
                        panic!("seed {seed} {players}p {}: driver failure: {e}", policy.as_str())
                    });
                if let Some(path) = outcome.divergence {
                    divergences.push(format!(
                        "seed {seed} {players}p {}: engine {:?} vs oracle {:?} -> {}",
                        policy.as_str(),
                        outcome.comparison.engine,
                        outcome.comparison.oracle,
                        path.display()
                    ));
                }
            }
        }
    }

    assert!(
        divergences.is_empty(),
        "{} games with scoring divergences (artifacts written):\n{}",
        divergences.len(),
        divergences.join("\n")
    );
}
