//! Command-line differential sweep runner.
//!
//! Usage: `cargo run -p yokohama-difftest -- <games> <players> <base_seed> <policy>`
//! with `<policy>` one of `uniform` | `retreat-averse`.
//!
//! Plays `<games>` seeded self-play games (seeds `base_seed..base_seed+games`)
//! comparing engine vs. oracle at every decision point; each game's first
//! divergence writes a self-contained artifact to
//! `crates/yokohama-difftest/artifacts/` and stops that game. Exits 0 when
//! every game completes divergence-free, 1 on any divergence or driver
//! failure, 2 on usage error.
//!
//! NOTE (Stage 1 shell): while `yokohama_oracle::oracle_moves` is an empty/partial
//! stub, EVERY game diverges at step 0 (all engine moves are engine-only) —
//! expected until the independence-protected oracle track lands the real
//! enumerator.

use std::process::ExitCode;

use yokohama_core::Rules;
use yokohama_difftest::artifact::default_artifacts_dir;
use yokohama_difftest::drive::Policy;
use yokohama_difftest::runner::run_differential_game;

/// `repro <artifact.json>`: reload a divergence artifact's state snapshot and
/// re-run both sides' enumeration on it, printing the full key lists and the
/// re-computed diff — the mechanical half of adversarial verification.
fn repro(path: &str) -> ExitCode {
    let artifact = match yokohama_difftest::artifact::read_artifact(std::path::Path::new(path)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("repro: cannot read artifact {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let state: yokohama_core::GameState = match serde_json::from_value(artifact.state_json.clone()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("repro: state snapshot does not deserialize: {e}");
            return ExitCode::from(2);
        }
    };
    let rules = Rules::standard();
    println!(
        "artifact: seed={} players={} policy={} step={} turn={} phase={}",
        artifact.seed, artifact.players, artifact.policy, artifact.step_index,
        artifact.turn_number, artifact.phase
    );
    println!(
        "state: current_player={:?} pending_choices={:?}",
        state.current_player, state.pending_choices
    );
    let engine_moves = yokohama_core::get_legal_moves(&state, &rules);
    let oracle = yokohama_oracle::oracle_moves(&state, &rules);
    let cmp = yokohama_difftest::compare::compare_moves(&state, &rules, &engine_moves, &oracle);
    println!("\n== ENGINE ({} moves) ==", engine_moves.len());
    for m in &engine_moves {
        println!("  {m:?}");
    }
    println!("\n== ORACLE ({} exact + {} labels) ==", oracle.exact.len(), oracle.movement.len());
    for m in &oracle.exact {
        println!("  {m:?}");
    }
    for l in &oracle.movement {
        println!("  {l:?}");
    }
    println!("\n== RE-COMPUTED DIFF ==");
    println!("engine_only ({}):", cmp.engine_only.len());
    for k in &cmp.engine_only {
        println!("  {k}");
    }
    println!("oracle_only ({}):", cmp.oracle_only.len());
    for k in &cmp.oracle_only {
        println!("  {k}");
    }
    println!("duplicates ({}):", cmp.duplicates.len());
    for k in &cmp.duplicates {
        println!("  {k}");
    }
    let stored = (&artifact.engine_only, &artifact.oracle_only, &artifact.duplicates);
    let fresh = (&cmp.engine_only, &cmp.oracle_only, &cmp.duplicates);
    println!(
        "\nreproduces stored diff: {}",
        if stored == fresh { "YES (byte-identical)" } else { "NO — stored diff differs from re-run (investigate)" }
    );
    if cmp.is_divergent() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "repro" {
        return repro(&args[2]);
    }
    if args.len() != 5 {
        eprintln!("usage: yokohama-difftest <games> <players> <base_seed> <policy: uniform|retreat-averse>");
        eprintln!("       yokohama-difftest repro <artifact.json>");
        return ExitCode::from(2);
    }
    let (games, players, base_seed, policy) = match (
        args[1].parse::<u64>(),
        args[2].parse::<u8>(),
        args[3].parse::<u64>(),
        args[4].parse::<Policy>(),
    ) {
        (Ok(g), Ok(p), Ok(s), Ok(pol)) if (2..=4).contains(&p) => (g, p, s, pol),
        _ => {
            eprintln!("usage: yokohama-difftest <games> <players: 2-4> <base_seed> <policy: uniform|retreat-averse>");
            return ExitCode::from(2);
        }
    };

    let rules = Rules::standard();
    let artifact_dir = default_artifacts_dir();
    let mut clean: u64 = 0;
    let mut diverged: u64 = 0;

    for i in 0..games {
        let seed = base_seed + i;
        match run_differential_game(seed, players, policy, &rules, &artifact_dir) {
            Ok(outcome) => match outcome.divergence {
                None => {
                    clean += 1;
                    println!(
                        "seed {seed}: clean ({} steps, {} turns)",
                        outcome.stats.steps, outcome.stats.turns
                    );
                }
                Some(path) => {
                    diverged += 1;
                    println!(
                        "seed {seed}: DIVERGENCE at step {} (turn {}) -> {}",
                        outcome.stats.steps,
                        outcome.stats.turns,
                        path.display()
                    );
                }
            },
            Err(e) => {
                eprintln!("seed {seed}: DRIVER FAILURE:\n{e}");
                return ExitCode::from(1);
            }
        }
    }

    println!(
        "{games} games ({players}p, policy {}): {clean} clean, {diverged} diverged",
        policy.as_str()
    );
    if diverged > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
