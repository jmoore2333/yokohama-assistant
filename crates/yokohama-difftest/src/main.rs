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
//! `score <games> <players> <base_seed> <policy>` (Stage 2) instead drives
//! each game TO COMPLETION with no per-step comparison and differentially
//! compares FINAL scoring (engine `final_result` vs. oracle
//! `final_standings`: per-player totals AND the full ranked order) at the
//! terminal state; divergences write `phase: "FinalScoring"` artifacts.
//! Same exit codes. This mirrors the `scoring_differential` release-gate
//! sweep for manual runs.
//!
//! `routes3 <games> <players> <base_seed> <policy>` (Stage 3) drives the
//! same seeded games but runs the THREE-WAY pruning-soundness check (engine
//! walk vs pruned-oracle walk vs pruneless reference, as route-label key
//! sets) at every qualifying decision point (Movement phase, empty queue,
//! President on board); the first divergence per game writes a
//! `phase: "Routes3"` artifact and stops that game. Per-seed lines report
//! how many decision points qualified and the largest label set seen; the
//! summary reports the overall maximum. Same exit codes. Mirrors the
//! `routes3_differential` release-gate sweep for manual runs.
//!
//! NOTE (Stage 1 shell): while `yokohama_oracle::oracle_moves` is an empty/partial
//! stub, EVERY game diverges at step 0 (all engine moves are engine-only) —
//! expected until the independence-protected oracle track lands the real
//! enumerator.

use std::process::ExitCode;

use yokohama_core::Rules;
use yokohama_difftest::artifact::default_artifacts_dir;
use yokohama_difftest::drive::Policy;
use yokohama_difftest::routes3::run_routes3_game;
use yokohama_difftest::runner::{run_differential_game, run_scoring_game};

/// `repro [--strict] <artifact.json>`: reload a divergence artifact's state
/// snapshot and re-run both sides' enumeration on it, printing the full key
/// lists and the re-computed diff — the mechanical half of adversarial
/// verification.
///
/// Exit codes: 0 = no CURRENT divergence on the state; 1 = currently
/// divergent; 2 = unreadable artifact; with `--strict`, 3 = the stored diff
/// does not reproduce byte-identically. A stored-vs-fresh mismatch is
/// EXPECTED for historical fixtures re-examined after a fix landed on either
/// side (the stored diff documents the pre-fix divergence) — use `--strict`
/// only when verifying a FRESH artifact.
fn repro(path: &str, strict: bool) -> ExitCode {
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
    if artifact.format_version > yokohama_difftest::artifact::ARTIFACT_FORMAT_VERSION {
        eprintln!(
            "repro: artifact schema v{} is newer than this binary's v{} — rebuild before trusting output",
            artifact.format_version,
            yokohama_difftest::artifact::ARTIFACT_FORMAT_VERSION
        );
    }
    println!(
        "artifact: schema=v{} seed={} players={} policy={} step={} turn={} phase={}",
        artifact.format_version, artifact.seed, artifact.players, artifact.policy,
        artifact.step_index, artifact.turn_number, artifact.phase
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
    let reproduces = stored == fresh;
    println!(
        "\nreproduces stored diff: {}",
        if reproduces {
            "YES (byte-identical)"
        } else {
            "NO — stored diff differs from re-run (EXPECTED for a historical fixture after a fix on either side; unexpected for a fresh artifact — re-run with --strict to fail on this)"
        }
    );
    if strict && !reproduces {
        return ExitCode::from(3);
    }
    if cmp.is_divergent() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// `score <games> <players> <base_seed> <policy>`: the Stage-2 terminal
/// scoring sweep (see module docs). Mirrors the enumeration sweep loop:
/// exit 0 all clean, 1 on any divergence or driver failure.
fn score_sweep(games: u64, players: u8, base_seed: u64, policy: Policy) -> ExitCode {
    let rules = Rules::standard();
    let artifact_dir = default_artifacts_dir();
    let mut clean: u64 = 0;
    let mut diverged: u64 = 0;

    for i in 0..games {
        let seed = base_seed + i;
        match run_scoring_game(seed, players, policy, &rules, &artifact_dir) {
            Ok(outcome) => match outcome.divergence {
                None => {
                    clean += 1;
                    println!(
                        "seed {seed}: scoring clean ({} steps, {} turns) engine={:?}",
                        outcome.stats.steps, outcome.stats.turns, outcome.comparison.engine
                    );
                }
                Some(path) => {
                    diverged += 1;
                    println!(
                        "seed {seed}: SCORING DIVERGENCE after {} steps (turn {})\n  engine: {:?}\n  oracle: {:?}\n  -> {}",
                        outcome.stats.steps,
                        outcome.stats.turns,
                        outcome.comparison.engine,
                        outcome.comparison.oracle,
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
        "{games} games ({players}p, policy {}): {clean} scoring-clean, {diverged} diverged",
        policy.as_str()
    );
    if diverged > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// `routes3 <games> <players> <base_seed> <policy>`: the Stage-3 three-way
/// pruning-soundness sweep — engine walk vs pruned-oracle walk vs pruneless
/// reference at every qualifying decision point (Movement, empty queue,
/// on-board president). Prints per-seed qualifying counts and the maximum
/// label count observed; the first divergence per game writes a
/// `phase: "Routes3"` artifact. Exit 0 all clean, 1 on any divergence or
/// driver failure.
fn routes3_sweep(games: u64, players: u8, base_seed: u64, policy: Policy) -> ExitCode {
    let rules = Rules::standard();
    let artifact_dir = default_artifacts_dir();
    let mut clean: u64 = 0;
    let mut diverged: u64 = 0;
    let mut qualifying_total: u64 = 0;
    let mut max_labels: usize = 0;

    for i in 0..games {
        let seed = base_seed + i;
        match run_routes3_game(seed, players, policy, &rules, &artifact_dir) {
            Ok(outcome) => {
                qualifying_total += outcome.qualifying_steps;
                max_labels = max_labels.max(outcome.max_labels);
                match outcome.divergence {
                    None => {
                        clean += 1;
                        println!(
                            "seed {seed}: routes3 clean ({} steps, {} qualifying, max {} labels)",
                            outcome.stats.steps, outcome.qualifying_steps, outcome.max_labels
                        );
                    }
                    Some(path) => {
                        diverged += 1;
                        println!(
                            "seed {seed}: ROUTES3 DIVERGENCE at step {} (turn {}) -> {}",
                            outcome.stats.steps,
                            outcome.stats.turns,
                            path.display()
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("seed {seed}: DRIVER FAILURE:\n{e}");
                return ExitCode::from(1);
            }
        }
    }

    println!(
        "{games} games ({players}p, policy {}): {clean} routes3-clean, {diverged} diverged; \
         {qualifying_total} qualifying decision points, max {max_labels} labels",
        policy.as_str()
    );
    if diverged > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if (args.len() == 3 && args[1] == "repro")
        || (args.len() == 4 && args[1] == "repro" && args[2] == "--strict")
    {
        let strict = args.len() == 4;
        return repro(if strict { &args[3] } else { &args[2] }, strict);
    }
    if args.len() == 6 && (args[1] == "score" || args[1] == "routes3") {
        return match (
            args[2].parse::<u64>(),
            args[3].parse::<u8>(),
            args[4].parse::<u64>(),
            args[5].parse::<Policy>(),
        ) {
            (Ok(g), Ok(p), Ok(s), Ok(pol)) if (2..=4).contains(&p) => {
                if args[1] == "score" {
                    score_sweep(g, p, s, pol)
                } else {
                    routes3_sweep(g, p, s, pol)
                }
            }
            _ => {
                eprintln!(
                    "usage: yokohama-difftest {} <games> <players: 2-4> <base_seed> <policy: uniform|retreat-averse>",
                    args[1]
                );
                ExitCode::from(2)
            }
        };
    }
    if args.len() != 5 {
        eprintln!("usage: yokohama-difftest <games> <players> <base_seed> <policy: uniform|retreat-averse>");
        eprintln!("       yokohama-difftest score <games> <players> <base_seed> <policy>");
        eprintln!("       yokohama-difftest routes3 <games> <players> <base_seed> <policy>");
        eprintln!("       yokohama-difftest repro [--strict] <artifact.json>");
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
