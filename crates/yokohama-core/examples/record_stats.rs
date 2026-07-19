//! Task 16 (design 2026-07-18 §6.1-3): read a saved `GameRecord` JSON and
//! print the same choice-effectiveness metrics block `selfplay.rs` prints,
//! for that one game.
//!
//! Usage: `cargo run --release -p yokohama-core --example record_stats -- <save.json>`
//!
//! Works on both tester-submitted saves and the eventual live gate game's
//! save: `yokohama-cli`'s `Session::save` (crates/yokohama-cli/src/session.rs)
//! serializes `&self.record` -- a plain `GameRecord`, with no wrapping
//! struct -- directly to disk, and `Session::load` deserializes straight
//! back into `GameRecord`. So this example loads the exact same on-disk
//! shape via `yokohama_core`'s own public `GameRecord` type; no CLI
//! dependency and no serde shim were needed (see the Task 16 report for the
//! save-shape finding this confirms).

#[path = "choice_metrics/mod.rs"]
mod choice_metrics;

use choice_metrics::ChoiceMetrics;
use yokohama_core::{apply_move, finalize_setup, new_game, reconcile_setup, GameRecord, RecordedEvent, Rules};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: record_stats <save.json>");
        std::process::exit(2);
    }
    let path = &args[1];

    let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("failed to read {path}: {e}");
        std::process::exit(1);
    });
    let record: GameRecord = serde_json::from_str(&text).unwrap_or_else(|e| {
        eprintln!("failed to parse {path} as a GameRecord: {e}");
        std::process::exit(1);
    });

    // Same standard registries the CLI itself plays with (main.rs) --
    // GameRecord carries no custom Rules, only a seed/config/choices/events.
    let rules = Rules::standard();
    let mut pending = new_game(record.config.clone(), &rules, record.seed).unwrap_or_else(|e| {
        eprintln!("new_game failed: {e}");
        std::process::exit(1);
    });
    if let Some(obs) = &record.setup_observations {
        pending = reconcile_setup(pending, obs, &rules).unwrap_or_else(|e| {
            eprintln!("reconcile_setup failed: {e}");
            std::process::exit(1);
        });
    }
    let mut state = finalize_setup(pending, &record.choices).unwrap_or_else(|e| {
        eprintln!("finalize_setup failed: {e}");
        std::process::exit(1);
    });

    // Mirrors yokohama-core's own replay_from fold (game/record.rs) event by
    // event, but calls metrics.observe on each Move BEFORE applying it --
    // replay_from has no hook for that, and adding one would touch src/,
    // out of scope for this task (disclosed in the Task 16 report).
    let mut metrics = ChoiceMetrics::new();
    for (i, ev) in record.events.iter().enumerate() {
        match ev {
            RecordedEvent::Move(mv) => {
                metrics.observe(&state, mv, &rules);
                state = apply_move(&state, mv, &rules).unwrap_or_else(|e| {
                    eprintln!("event {i}: apply_move failed: {e}");
                    std::process::exit(1);
                });
            }
            RecordedEvent::RevealOrders(cards) => {
                state.arrange_orders_on_top(cards).unwrap_or_else(|e| {
                    eprintln!("event {i}: arrange_orders_on_top failed: {e}");
                    std::process::exit(1);
                });
            }
            RecordedEvent::RevealTechs(cards) => {
                state.arrange_techs_on_top(cards).unwrap_or_else(|e| {
                    eprintln!("event {i}: arrange_techs_on_top failed: {e}");
                    std::process::exit(1);
                });
            }
            RecordedEvent::RevealStartingOrder { player, card } => {
                state.reveal_starting_order(*player, *card).unwrap_or_else(|e| {
                    eprintln!("event {i}: reveal_starting_order failed: {e}");
                    std::process::exit(1);
                });
            }
            RecordedEvent::RevealFivePowerToken { area, effect } => {
                state.reveal_five_power_token(*area, *effect).unwrap_or_else(|e| {
                    eprintln!("event {i}: reveal_five_power_token failed: {e}");
                    std::process::exit(1);
                });
            }
        }
    }
    // Some(&state): unlike selfplay's driver, this loop always ends holding
    // the real resulting GameState (whether or not the replayed game
    // actually finished), so the trailing Church/Customs classification (if
    // any) can be exact instead of relying on finish_game's game-over
    // invariant fallback.
    metrics.finish_game(Some(&state));

    println!("record: {path} ({} events, seed {})", record.events.len(), record.seed);
    metrics.print();
}
