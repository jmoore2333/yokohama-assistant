//! Task 18 (spec F1/F2): random self-play driver.
//!
//! Usage: `cargo run --release -p yokohama-core --example selfplay -- <games> <players> <base_seed>`
//!
//! Plays `<games>` random games at `<players>` player count, seeded
//! `base_seed, base_seed+1, ..`. Exits nonzero with a full failure dump
//! (seed, turn, phase, move log, state JSON) on the first game that hits a
//! dead end, an `apply_move` error, or an invariant violation -- see
//! `tests/common/mod.rs` (`run_random_game`) for exactly what is checked.
//! On a fully successful run, prints: games completed, mean turns,
//! per-phase mean branching factor, end-trigger histogram, moves/sec, and
//! (Task 16, design §6.1-2) a choice-effectiveness metrics block.

#[path = "../tests/common/mod.rs"]
mod common;
#[path = "choice_metrics/mod.rs"]
mod choice_metrics;

use std::collections::HashMap;
use std::time::Instant;

use choice_metrics::ChoiceMetrics;
use common::run_random_game_observed;
use yokohama_core::{Rules, TurnPhase};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: selfplay <games> <players> <base_seed>");
        std::process::exit(2);
    }
    let games: u64 = args[1].parse().expect("<games> must be an integer");
    let players: u8 = args[2].parse().expect("<players> must be 2, 3, or 4");
    let base_seed: u64 = args[3].parse().expect("<base_seed> must be an integer");

    let rules = Rules::standard();
    let start = Instant::now();

    let mut completed: u64 = 0;
    let mut total_turns: u64 = 0;
    let mut total_moves: u64 = 0;
    let mut branching_by_phase: HashMap<TurnPhase, (u64, u64)> = HashMap::new();
    let mut trigger_histogram: HashMap<String, u64> = HashMap::new();
    let mut metrics = ChoiceMetrics::new();

    for i in 0..games {
        let seed = base_seed + i;
        // run_random_game_observed (not the plain run_random_game): same
        // play-to-completion driver and GameStats, plus a per-move hook so
        // the choice-metrics block below can be built from this exact game
        // trajectory instead of a second, separately-driven replay.
        let result = run_random_game_observed(seed, players, &rules, |state, mv| {
            metrics.observe(state, mv, &rules);
        });
        match result {
            Ok(stats) => {
                completed += 1;
                total_turns += stats.turns as u64;
                total_moves += stats.moves_applied;
                for (phase, (sum, count)) in stats.branching_by_phase {
                    let entry = branching_by_phase.entry(phase).or_insert((0, 0));
                    entry.0 += sum;
                    entry.1 += count;
                }
                let key = stats.trigger.map_or_else(|| "None".to_string(), |t| format!("{t:?}"));
                *trigger_histogram.entry(key).or_insert(0) += 1;
                // None: run_random_game_observed only returns summary
                // GameStats, not the final GameState -- see finish_game's
                // doc comment for why that's still exact.
                metrics.finish_game(None);
            }
            Err(e) => {
                eprintln!(
                    "FAILURE after {completed}/{games} completed games (this one: seed {seed}, {players}p):\n{e}"
                );
                std::process::exit(1);
            }
        }
    }

    let elapsed = start.elapsed();
    println!("games completed: {completed}/{games}");
    println!("mean turns: {:.1}", total_turns as f64 / completed.max(1) as f64);

    println!("per-phase mean branching factor:");
    let mut phases: Vec<TurnPhase> = branching_by_phase.keys().copied().collect();
    phases.sort_by_key(|p| format!("{p:?}")); // stable, deterministic print order
    for phase in phases {
        let (sum, count) = branching_by_phase[&phase];
        println!("  {phase:?}: {:.2} (n={count})", sum as f64 / count.max(1) as f64);
    }

    println!("end-trigger histogram: {trigger_histogram:?}");
    let moves_per_sec = total_moves as f64 / elapsed.as_secs_f64().max(1e-9);
    println!("moves/sec: {moves_per_sec:.1}");

    metrics.print();
}
