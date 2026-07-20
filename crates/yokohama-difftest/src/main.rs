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
//! NOTE (Stage 1 shell): while `yokohama_oracle::legal_moves` is the empty
//! stub, EVERY game diverges at step 0 (all engine moves are engine-only) —
//! expected until the independence-protected oracle track lands the real
//! enumerator.

use std::process::ExitCode;

use yokohama_core::Rules;
use yokohama_difftest::artifact::default_artifacts_dir;
use yokohama_difftest::drive::Policy;
use yokohama_difftest::runner::run_differential_game;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprintln!("usage: yokohama-difftest <games> <players> <base_seed> <policy: uniform|retreat-averse>");
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
