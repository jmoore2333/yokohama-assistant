//! Task 18 (spec F1/F2): full-game random self-play fuzz gate.
//!
//! `fuzz_500_games_per_player_count` is the CI-sized slice of the full gate
//! (500 seeds x {2,3,4} players); the full 5000-seed sweep is run via the
//! `selfplay` example (`cargo run --release -p yokohama-core --example
//! selfplay -- 5000 4 0`, Task 18 Step 5). It is `#[ignore]`d: as scaffolded
//! here (Task 18 Steps 1-3), the engine is not yet expected to pass it --
//! that's Task 18 Steps 4-5 (orchestrator-direct: fix what the fuzzer
//! finds, one commit per fix, never weaken the invariants). Un-ignore once
//! the gate is green.
//!
//! `smoke_run_random_game_never_panics` is the non-ignored counterpart: it
//! proves the harness itself (not the engine) is sound by asserting
//! `run_random_game` only ever returns `Ok`/`Err`, never panics. `Err`
//! results are expected and reported, not asserted against, at this stage.

mod common;
use common::run_random_game;
use yokohama_core::Rules;

#[test]
#[ignore = "Task 18.4-5 gate: run explicitly via cargo test -- --ignored; un-ignored when the gate passes"]
fn fuzz_500_games_per_player_count() {
    let rules = Rules::standard();
    let mut triggers_seen = std::collections::HashSet::new();

    for players in 2..=4u8 {
        for seed in 0..500u64 {
            let stats = run_random_game(seed, players, &rules)
                .unwrap_or_else(|e| panic!("seed {seed} {players}p: {e}"));
            if let Some(t) = stats.trigger {
                triggers_seen.insert(std::mem::discriminant(&t));
            }
        }
    }

    assert_eq!(triggers_seen.len(), 5, "all five end triggers must occur across the corpus");
}

#[test]
fn smoke_run_random_game_never_panics() {
    let rules = Rules::standard();
    for seed in 0..3u64 {
        match run_random_game(seed, 2, &rules) {
            Ok(stats) => println!(
                "seed {seed}: completed in {} turns ({} moves), trigger {:?}",
                stats.turns, stats.moves_applied, stats.trigger
            ),
            Err(e) => println!(
                "seed {seed}: reported Err (acceptable pre-Task-18.4/5, see task-18-report.md): {}",
                e.lines().next().unwrap_or(&e)
            ),
        }
    }
}
