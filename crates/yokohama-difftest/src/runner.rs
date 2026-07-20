//! Differential runner: drive one engine self-play game, comparing engine
//! vs. oracle enumeration at every decision point, and write a
//! [`DivergenceArtifact`](crate::artifact::DivergenceArtifact) for the first
//! divergence (the game stops there — the state snapshot in the artifact is
//! the whole repro, so playing on adds nothing).

use std::path::{Path, PathBuf};

use yokohama_core::Rules;

use crate::artifact::{write_artifact, DivergenceArtifact};
use crate::compare::compare_moves;
use crate::drive::{drive_game, DriveStats, Policy, StepOutcome};

/// Outcome of one differentially-checked game.
#[derive(Debug, Clone)]
pub struct GameDiffOutcome {
    pub stats: DriveStats,
    /// Path of the artifact written for the first divergence, if any.
    pub divergence: Option<PathBuf>,
}

/// Drive the `(seed, players, policy)` game to completion, comparing engine
/// and oracle at every decision point. On the first divergence (set
/// mismatch in either direction, or duplicate keys on either side) a
/// self-contained artifact is written to `artifact_dir` and the game stops.
/// `Err` carries hard driver failures (dead end, invariant violation, or an
/// artifact-write failure), never a divergence — divergences are data.
pub fn run_differential_game(
    seed: u64,
    players: u8,
    policy: Policy,
    rules: &Rules,
    artifact_dir: &Path,
) -> Result<GameDiffOutcome, String> {
    let mut divergence: Option<PathBuf> = None;

    let stats = drive_game(seed, players, policy, rules, None, |state, engine_moves, step| {
        let oracle = yokohama_oracle::oracle_moves(state, rules);
        let cmp = compare_moves(state, rules, engine_moves, &oracle);
        if !cmp.is_divergent() {
            return Ok(StepOutcome::Continue);
        }
        let artifact = DivergenceArtifact {
            format_version: crate::artifact::ARTIFACT_FORMAT_VERSION,
            seed,
            players,
            policy: policy.as_str().to_string(),
            step_index: step,
            turn_number: state.turn_number,
            phase: format!("{:?}", state.turn_phase),
            state_json: serde_json::to_value(state)
                .map_err(|e| format!("state serialization for artifact failed: {e}"))?,
            engine_only: cmp.engine_only,
            oracle_only: cmp.oracle_only,
            duplicates: cmp.duplicates,
        };
        let path = write_artifact(&artifact, artifact_dir)?;
        divergence = Some(path);
        Ok(StepOutcome::Stop)
    })?;

    Ok(GameDiffOutcome { stats, divergence })
}
