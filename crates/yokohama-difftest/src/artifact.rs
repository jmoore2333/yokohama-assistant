//! Divergence artifacts: a self-contained minimal reproduction.
//!
//! The artifact carries the FULL serialized `GameState` (complete and
//! resumable by design — see yokohama-core's crate docs), so the state
//! snapshot IS the minimal repro: anyone can deserialize it and re-run both
//! enumerators without replaying a move prefix. `seed`/`step_index` are kept
//! for provenance only.
//!
//! Artifacts are written to `crates/yokohama-difftest/artifacts/` (gitignored)
//! as `<seed>-<players>-<policy>-<step>.json`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Current artifact schema version. Bump when `DivergenceArtifact`'s shape
/// (or the meaning of a field) changes IN A NON-BACKWARD-COMPATIBLE way, so a
/// future reader of an old file gets "predates schema N" instead of a raw
/// serde error. Version 1 = Stage-1 full-GameState-snapshot form (blessed at
/// the Stage-1 review: the state snapshot IS the repro; the plan's
/// move-prefix + shrinking mandate is satisfied by a 0-move prefix).
///
/// The `rules_hash` / `semantics_rev` provenance fields (review §4.6) were
/// added WITHOUT bumping this: they are purely additive and `#[serde(default)]`,
/// so every pre-provenance file still deserializes cleanly (empty hash / no
/// rev) — the committed `fixtures/ruling-a1/*.json` are exactly such files.
/// A version bump is for shapes that BREAK old readers; a defaulted addition
/// does not.
pub const ARTIFACT_FORMAT_VERSION: u32 = 1;

fn default_format_version() -> u32 {
    // Files written before the field existed are version 1 by definition.
    1
}

/// One recorded divergence between engine and oracle enumeration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DivergenceArtifact {
    /// Artifact schema version — see [`ARTIFACT_FORMAT_VERSION`].
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    /// Seed of the self-play game that reached the diverging state.
    pub seed: u64,
    /// Player count of that game.
    pub players: u8,
    /// Move-selection policy name (`"uniform"` / `"retreat-averse"`).
    pub policy: String,
    /// 0-based index of the decision point within the game (number of moves
    /// applied before this state).
    pub step_index: u64,
    /// `GameState::turn_number` at the diverging state.
    pub turn_number: u32,
    /// `GameState::turn_phase` at the diverging state (Debug rendering).
    pub phase: String,
    /// The full diverging `GameState`, serialized with serde — self-contained
    /// and resumable.
    pub state_json: serde_json::Value,
    /// Keys the engine offered that the oracle did not (soundness).
    pub engine_only: Vec<String>,
    /// Keys the oracle found that the engine missed (completeness).
    pub oracle_only: Vec<String>,
    /// Duplicate keys per side (`"<side>: <key> x<count>"`).
    pub duplicates: Vec<String>,
    /// `Rules::content_hash()` of the rules the artifact was written against
    /// (review §4.6 provenance). Empty for pre-provenance files (serde
    /// default). `repro` warns if it differs from the running rules.
    #[serde(default)]
    pub rules_hash: String,
    /// `SEMANTICS_REV` of the engine that wrote the artifact (review §4.6
    /// provenance). `None` for pre-provenance files (serde default). `repro`
    /// warns if it differs from the running engine's revision.
    #[serde(default)]
    pub semantics_rev: Option<u32>,
}

/// Provenance stamp for a FRESH artifact: the running rules fingerprint and
/// the engine's semantics revision. All three writers use this so an artifact
/// records which rules/semantics produced it (review §4.6).
pub fn current_provenance(rules: &yokohama_core::Rules) -> (String, Option<u32>) {
    (rules.content_hash(), Some(yokohama_core::SEMANTICS_REV))
}

/// The artifact KIND segment, derived from `phase`: `"scoring"` for
/// `FinalScoring`, `"routes3"` for `Routes3`, `"enum"` for a Stage-1
/// enumeration artifact (any `TurnPhase` Debug string). Part of the filename
/// so a routes3 (or scoring) divergence can never OVERWRITE a Stage-1
/// artifact at the same `<seed>-<players>-<policy>-<step>` coordinates
/// (review §4.6: `fs::write` truncates).
pub fn artifact_kind(phase: &str) -> &'static str {
    match phase {
        "FinalScoring" => "scoring",
        "Routes3" => "routes3",
        _ => "enum",
    }
}

/// The crate-local gitignored artifacts directory,
/// `crates/yokohama-difftest/artifacts/`.
pub fn default_artifacts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("artifacts")
}

/// Canonical artifact filename:
/// `<seed>-<players>-<policy>-<kind>-<step>.json`. The `<kind>` segment
/// (review §4.6) keeps a routes3/scoring divergence from truncating a
/// Stage-1 enumeration artifact at the same coordinates.
pub fn artifact_filename(artifact: &DivergenceArtifact) -> String {
    format!(
        "{}-{}-{}-{}-{}.json",
        artifact.seed,
        artifact.players,
        artifact.policy,
        artifact_kind(&artifact.phase),
        artifact.step_index
    )
}

/// Serialize `artifact` to pretty JSON at `dir/<canonical filename>`,
/// creating `dir` if needed. Returns the written path.
pub fn write_artifact(artifact: &DivergenceArtifact, dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(dir)
        .map_err(|e| format!("artifact dir {} could not be created: {e}", dir.display()))?;
    let path = dir.join(artifact_filename(artifact));
    let json = serde_json::to_string_pretty(artifact)
        .map_err(|e| format!("artifact serialization failed: {e}"))?;
    fs::write(&path, json)
        .map_err(|e| format!("artifact write to {} failed: {e}", path.display()))?;
    Ok(path)
}

/// Read an artifact back from `path`.
pub fn read_artifact(path: &Path) -> Result<DivergenceArtifact, String> {
    let json = fs::read_to_string(path)
        .map_err(|e| format!("artifact read from {} failed: {e}", path.display()))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("artifact parse of {} failed: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DivergenceArtifact {
        DivergenceArtifact {
            format_version: ARTIFACT_FORMAT_VERSION,
            seed: u64::MAX, // distinctive: cannot collide with sweep seeds
            players: 2,
            policy: "uniform".to_string(),
            step_index: 17,
            turn_number: 4,
            phase: "Movement".to_string(),
            // A placeholder Value: round-trip fidelity of the artifact
            // envelope is what's under test here; a REAL GameState flows
            // through this field in the runner path (tests/differential.rs).
            state_json: serde_json::json!({"stub": true, "nested": [1, 2, 3]}),
            engine_only: vec!["move:A".to_string()],
            oracle_only: vec![],
            duplicates: vec!["oracle: route:X x2".to_string()],
            rules_hash: "deadbeef".to_string(),
            semantics_rev: Some(1),
        }
    }

    #[test]
    fn filename_encodes_provenance() {
        // phase "Movement" -> kind "enum".
        assert_eq!(
            artifact_filename(&sample()),
            format!("{}-2-uniform-enum-17.json", u64::MAX),
        );
    }

    #[test]
    fn filename_kind_separates_scoring_and_routes3_from_enumeration() {
        let mut a = sample();
        a.phase = "FinalScoring".to_string();
        assert!(artifact_filename(&a).contains("-scoring-"), "{}", artifact_filename(&a));
        a.phase = "Routes3".to_string();
        assert!(artifact_filename(&a).contains("-routes3-"), "{}", artifact_filename(&a));
        // A routes3 and a Stage-1 artifact at identical coordinates get
        // DISTINCT filenames (the review's overwrite hazard).
        let mut enum_art = sample();
        enum_art.phase = "Movement".to_string();
        let mut routes_art = sample();
        routes_art.phase = "Routes3".to_string();
        assert_ne!(artifact_filename(&enum_art), artifact_filename(&routes_art));
    }

    #[test]
    fn pre_provenance_artifact_loads_with_defaulted_fields() {
        // A file written before the provenance fields existed (no rules_hash,
        // no semantics_rev) must still deserialize — the committed
        // fixtures/ruling-a1 files are exactly this shape.
        let json = serde_json::json!({
            "seed": 7, "players": 2, "policy": "uniform", "step_index": 3,
            "turn_number": 4, "phase": "Movement",
            "state_json": {"stub": true},
            "engine_only": [], "oracle_only": [], "duplicates": []
        });
        let art: DivergenceArtifact = serde_json::from_value(json).expect("old-format loads");
        assert_eq!(art.format_version, 1, "missing version defaults to 1");
        assert_eq!(art.rules_hash, "", "missing rules_hash defaults to empty");
        assert_eq!(art.semantics_rev, None, "missing semantics_rev defaults to None");
    }

    #[test]
    fn artifact_round_trips_through_file() {
        let artifact = sample();
        let dir = default_artifacts_dir();
        let path = write_artifact(&artifact, &dir).expect("write");
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), artifact_filename(&artifact));
        let back = read_artifact(&path).expect("read");
        assert_eq!(artifact, back);
        std::fs::remove_file(&path).expect("cleanup");
    }

    #[test]
    fn read_reports_missing_file_as_error_not_panic() {
        let err = read_artifact(Path::new("definitely/not/a/real/artifact.json"))
            .expect_err("missing file must be an Err");
        assert!(err.contains("artifact"), "error should say what failed: {err}");
    }
}
