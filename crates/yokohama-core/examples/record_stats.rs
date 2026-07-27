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
use yokohama_core::{
    finalize_setup, new_game, reconcile_setup, replay_from, replay_strict, GameRecord, RecordedEvent, Rules,
};

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

    // Tool-level fail-closed on the digest (FIX-B P1, Codex PR-3 review):
    // `replay_strict` itself verifies the terminal digest only WHEN PRESENT
    // (in-memory test records legitimately pre-date stamping), but a record
    // submitted to THIS tool is retained-corpus input and must carry one.
    if record.terminal_digest.is_none() {
        eprintln!(
            "{path}: record carries no terminal_digest -- a record without a digest is not \
             acceptable retained-corpus input. Re-save it via the CLI Session (which stamps the \
             digest at save) or stamp it with GameRecord::stamp_terminal_digest before submitting."
        );
        std::process::exit(1);
    }

    // Strict gate (guidingpath I5): before trusting any metric off a
    // submitted save, hold it to the bar a training pipeline would. Strict
    // replay rejects a rules_hash / semantics_rev mismatch (engine_version
    // is provenance, not a gate -- 2026-07-27 decoupling), runs
    // check_invariants after every event, and (if the record was stamped)
    // verifies the terminal-state digest -- so a corpus produced
    // under different executable semantics fails loudly here instead of being
    // silently reinterpreted and folded into the stats.
    let final_state = replay_strict(&record, &rules).unwrap_or_else(|e| {
        eprintln!("strict replay rejected {path} at event {}: {}", e.event_index, e.message);
        std::process::exit(1);
    });

    // Metrics need each Move observed against the state AS OF that move, which
    // the final-state-only replay_strict has no hook for -- so re-fold
    // leniently for the metrics walk. Strict replay just proved the setup
    // rebuilds and every event applies, so these steps cannot fail now (hence
    // `expect`, not a graceful exit -- a failure here would be an engine
    // determinism bug, not bad input). Per-event replay_from keeps this fold
    // from drifting from yokohama-core's own replay logic (M4).
    let mut pending = new_game(record.config.clone(), &rules, record.seed)
        .expect("strict replay already rebuilt setup from this seed/config");
    if let Some(obs) = &record.setup_observations {
        pending =
            reconcile_setup(pending, obs, &rules).expect("strict replay already reconciled these observations");
    }
    let mut state =
        finalize_setup(pending, &record.choices).expect("strict replay already finalized setup with these choices");

    let mut metrics = ChoiceMetrics::new();
    for ev in &record.events {
        if let RecordedEvent::Move(mv) = ev {
            metrics.observe(&state, mv, &rules);
        }
        state = replay_from(&state, std::slice::from_ref(ev), &rules)
            .expect("strict replay already folded this event cleanly");
    }
    debug_assert_eq!(state, final_state, "the metrics re-fold must reproduce strict replay's terminal state");

    // final_state (== the loop's terminal state) is the real resulting
    // GameState -- whether or not the replayed game actually finished -- so
    // the trailing Church/Customs classification (if any) is exact instead of
    // relying on finish_game's game-over invariant fallback.
    metrics.finish_game(Some(&final_state));

    println!("record: {path} ({} events, seed {})", record.events.len(), record.seed);
    metrics.print();
}
