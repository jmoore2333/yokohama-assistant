//! Differential runner: drive one engine self-play game, comparing engine
//! vs. oracle enumeration at every decision point, and write a
//! [`DivergenceArtifact`](crate::artifact::DivergenceArtifact) for the first
//! divergence (the game stops there — the state snapshot in the artifact is
//! the whole repro, so playing on adds nothing).

use std::path::{Path, PathBuf};

use yokohama_core::{GameState, Rules};

use crate::artifact::{write_artifact, DivergenceArtifact, ARTIFACT_FORMAT_VERSION};
use crate::compare::compare_moves;
use crate::drive::{drive_game, drive_game_to_end, DriveStats, Policy, StepOutcome};
use crate::scoring::{compare_final, scoring_artifact_fields, ScoringComparison};

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
        let (rules_hash, semantics_rev) = crate::artifact::current_provenance(rules);
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
            rules_hash,
            semantics_rev,
        };
        let path = write_artifact(&artifact, artifact_dir)?;
        divergence = Some(path);
        Ok(StepOutcome::Stop)
    })?;

    Ok(GameDiffOutcome { stats, divergence })
}

/// Outcome of one scoring-checked game (Stage 2): the drive stats, the full
/// two-sided comparison at the terminal state (present even when clean, so
/// sweeps can report both sides), and the artifact path if it diverged.
#[derive(Debug, Clone)]
pub struct ScoringGameOutcome {
    pub stats: DriveStats,
    pub comparison: ScoringComparison,
    pub divergence: Option<PathBuf>,
}

/// Drive the `(seed, players, policy)` game TO COMPLETION (no per-step
/// comparison — that is Stage 1's `run_differential_game`) and compare
/// engine `final_result` vs. oracle `final_standings` at the terminal
/// state. A scoring divergence writes a schema-v1 artifact with
/// `phase: "FinalScoring"` (see [`write_scoring_artifact`]) — divergences
/// are data, never `Err`. `Err` carries hard driver failures only.
pub fn run_scoring_game(
    seed: u64,
    players: u8,
    policy: Policy,
    rules: &Rules,
    artifact_dir: &Path,
) -> Result<ScoringGameOutcome, String> {
    let (stats, final_state) =
        drive_game_to_end(seed, players, policy, rules, None, |_, _, _| Ok(StepOutcome::Continue))?;
    if !stats.completed {
        // Unreachable with no observer Stop and no max_steps (non-termination
        // is already an Err inside the driver), but never score a cut-off game.
        return Err(format!("seed={seed} players={players}p: game did not complete"));
    }
    let comparison = compare_final(&final_state, rules).ok_or_else(|| {
        format!("seed={seed} players={players}p: compare_final returned None on a driver-terminal state")
    })?;
    let divergence = if comparison.is_divergent() {
        Some(write_scoring_artifact(seed, players, policy, &stats, &final_state, &comparison, rules, artifact_dir)?)
    } else {
        None
    };
    Ok(ScoringGameOutcome { stats, comparison, divergence })
}

/// Write a scoring-divergence artifact, reusing the schema-v1
/// [`DivergenceArtifact`] envelope with the scoring conventions:
/// `phase: "FinalScoring"` (a value no `TurnPhase` Debug rendering produces,
/// so readers can tell scoring artifacts from enumeration ones),
/// `engine_only`/`oracle_only` carry each side's COMPLETE ranked totals as
/// `"P<id>: <total>"` lines (both full answers, not set differences —
/// ranking defects are visible only with both whole orders), `step_index` =
/// total moves applied (the terminal state's provenance), and `duplicates`
/// carries the oracle self-consistency finding when its emitted order
/// contradicts its own totals.
#[allow(clippy::too_many_arguments)]
pub fn write_scoring_artifact(
    seed: u64,
    players: u8,
    policy: Policy,
    stats: &DriveStats,
    final_state: &GameState,
    comparison: &ScoringComparison,
    rules: &Rules,
    artifact_dir: &Path,
) -> Result<PathBuf, String> {
    let (engine_only, oracle_only, duplicates) = scoring_artifact_fields(comparison);
    let (rules_hash, semantics_rev) = crate::artifact::current_provenance(rules);
    let artifact = DivergenceArtifact {
        format_version: ARTIFACT_FORMAT_VERSION,
        seed,
        players,
        policy: policy.as_str().to_string(),
        step_index: stats.steps,
        turn_number: final_state.turn_number,
        phase: "FinalScoring".to_string(),
        state_json: serde_json::to_value(final_state)
            .map_err(|e| format!("state serialization for scoring artifact failed: {e}"))?,
        engine_only,
        oracle_only,
        duplicates,
        rules_hash,
        semantics_rev,
    };
    write_artifact(&artifact, artifact_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::read_artifact;
    use crate::drive::new_seeded_game;
    use crate::scoring::compare_rankings;
    use yokohama_core::PlayerId;

    /// The scoring-artifact path works end to end without the oracle: a
    /// hand-built divergent comparison plus a REAL state round-trips through
    /// the schema-v1 envelope with the FinalScoring conventions intact.
    #[test]
    fn scoring_artifact_uses_final_scoring_conventions_and_round_trips() {
        let rules = Rules::standard();
        let (state, _rng) = new_seeded_game(5, 2, &rules).expect("setup");
        let engine = vec![(PlayerId::new(0), 42u32), (PlayerId::new(1), 17u32)];
        let oracle = vec![(PlayerId::new(1), 17u32), (PlayerId::new(0), 42u32)];
        let cmp = compare_rankings(engine, oracle);
        assert!(cmp.is_divergent());

        let stats = DriveStats {
            seed: u64::MAX - 2, // cannot collide with sweep artifacts
            players: 2,
            policy: Policy::Uniform,
            steps: 321,
            turns: state.turn_number,
            completed: true,
        };
        let dir = crate::artifact::default_artifacts_dir();
        let path = write_scoring_artifact(
            stats.seed, stats.players, stats.policy, &stats, &state, &cmp, &rules, &dir,
        )
        .expect("write");
        let back = read_artifact(&path).expect("read");
        std::fs::remove_file(&path).expect("cleanup");

        assert_eq!(back.phase, "FinalScoring");
        assert_eq!(back.step_index, 321);
        assert_eq!(back.engine_only, vec!["P0: 42", "P1: 17"]);
        assert_eq!(back.oracle_only, vec!["P1: 17", "P0: 42"]);
        assert_eq!(
            back.duplicates,
            vec!["oracle: emitted ranking is not descending in its own totals"],
            "the worst-first oracle order must surface the self-consistency finding"
        );
        let state_back: GameState =
            serde_json::from_value(back.state_json).expect("state deserializes");
        assert_eq!(state, state_back);
    }
}
