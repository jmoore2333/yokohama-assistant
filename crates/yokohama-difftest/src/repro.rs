//! Repro core: reload a divergence artifact's state snapshot and re-run the
//! comparison THAT KIND of artifact stores — the mechanical half of
//! adversarial verification (the `repro` subcommand of the bin is a thin
//! wrapper over [`repro_artifact`]).
//!
//! Artifacts share one schema-v1 envelope but store three different
//! comparisons, distinguished by `phase` (each a value the writers pin):
//! - `"FinalScoring"` (Stage 2, `runner::write_scoring_artifact`):
//!   `engine_only`/`oracle_only` hold both sides' COMPLETE ranked totals as
//!   `"P<id>: <total>"` lines; re-running means re-comparing terminal
//!   scoring ([`crate::scoring::compare_final`]).
//! - `"Routes3"` (Stage 3, `routes3::run_routes3_game`): the fields hold
//!   [`crate::routes3::format_pair_diffs`] lines of the three-way route
//!   comparison; re-running means [`crate::routes3::three_way_route_check`].
//! - anything else is a `TurnPhase` Debug rendering — a Stage-1 enumeration
//!   artifact; re-running means `get_legal_moves` vs `oracle_moves` through
//!   [`crate::compare::compare_moves`].

use yokohama_core::{GameState, Rules};

use crate::artifact::DivergenceArtifact;

/// Result of re-running an artifact's stored comparison on its state
/// snapshot: the fresh re-computation in the artifact's own stored-line
/// format (so byte-identity against the stored fields is meaningful for
/// every artifact kind) plus the human-readable detail lines the bin prints.
#[derive(Debug, Clone)]
pub struct ReproReport {
    /// Human-readable re-run details, printed verbatim by the bin.
    pub lines: Vec<String>,
    /// Fresh re-computation of the artifact's `engine_only` field, in the
    /// same line format the artifact writer used.
    pub engine_only: Vec<String>,
    /// Fresh `oracle_only`, same format as stored.
    pub oracle_only: Vec<String>,
    /// Fresh `duplicates`, same format as stored.
    pub duplicates: Vec<String>,
    /// True when the state is CURRENTLY divergent under the comparison this
    /// artifact kind stores.
    pub divergent: bool,
}

impl ReproReport {
    /// Does the fresh re-computation reproduce the stored diff
    /// byte-identically?
    pub fn reproduces(&self, artifact: &DivergenceArtifact) -> bool {
        artifact.engine_only == self.engine_only
            && artifact.oracle_only == self.oracle_only
            && artifact.duplicates == self.duplicates
    }
}

/// Re-run the comparison `artifact` stores on its own state snapshot.
///
/// `Err` means the artifact cannot be re-run at all (state snapshot does not
/// deserialize, or the state no longer supports the stored comparison — e.g.
/// a `FinalScoring` snapshot that is not terminal); the bin maps `Err` to
/// exit 2 alongside unreadable files. A successful report may still be
/// divergent and/or fail to reproduce the stored diff — those are verdicts,
/// not errors.
pub fn repro_artifact(
    artifact: &DivergenceArtifact,
    rules: &Rules,
) -> Result<ReproReport, String> {
    let state: GameState = serde_json::from_value(artifact.state_json.clone())
        .map_err(|e| format!("state snapshot does not deserialize: {e}"))?;
    match artifact.phase.as_str() {
        "FinalScoring" => repro_scoring(&state, rules),
        "Routes3" => repro_routes3(&state, rules),
        phase if is_enumeration_phase(phase) => repro_enumeration(&state, rules),
        other => Err(format!(
            "unknown artifact phase {other:?}: not the \"FinalScoring\"/\"Routes3\" tag and \
             not a TurnPhase enumeration phase — refusing to guess a comparator. A mis-tagged \
             artifact must NOT fall through to the enumeration comparison, which is \
             unconditionally clean on a terminal state and would report a silent false pass \
             (review §4.2)."
        )),
    }
}

/// The `phase` strings a legitimate Stage-1 enumeration artifact carries:
/// the `Debug` rendering of each [`yokohama_core::TurnPhase`]. Built from the
/// real enum values so it can never disagree with what the writers stamp
/// (`format!("{:?}", state.turn_phase)`). Any string that is neither this set
/// nor the two scoring/routes3 tags is a mis-tag and is rejected upstream.
fn is_enumeration_phase(phase: &str) -> bool {
    use yokohama_core::TurnPhase::*;
    [
        AdditionalActionFirst,
        Placement,
        Movement,
        AreaAction,
        FivePowerBonus,
        Building,
        Recovery,
        AdditionalActionSecond,
        TurnEnd,
    ]
    .iter()
    .any(|p| format!("{p:?}") == phase)
}

/// Stage-2 re-run: the terminal scoring comparison
/// ([`crate::scoring::compare_final`]), with both ranked orders printed and
/// the fresh fields built by the same
/// [`crate::scoring::scoring_artifact_fields`] projection the writer used.
fn repro_scoring(state: &GameState, rules: &Rules) -> Result<ReproReport, String> {
    let cmp = crate::scoring::compare_final(state, rules).ok_or_else(|| {
        "FinalScoring artifact's state snapshot is not terminal (is_game_over is false) — \
         cannot re-run the scoring comparison"
            .to_string()
    })?;
    let (engine_only, oracle_only, duplicates) = crate::scoring::scoring_artifact_fields(&cmp);

    let mut lines = vec![format!(
        "state: terminal, turn={} players={}",
        state.turn_number,
        state.players.len()
    )];
    lines.push(String::new());
    lines.push("== ENGINE final_result (ranked best-first) ==".to_string());
    for l in &engine_only {
        lines.push(format!("  {l}"));
    }
    lines.push(String::new());
    lines.push("== ORACLE final_standings (ranked best-first) ==".to_string());
    for l in &oracle_only {
        lines.push(format!("  {l}"));
    }
    lines.push(String::new());
    lines.push("== RE-COMPUTED SCORING COMPARISON ==".to_string());
    lines.push(format!(
        "totals_diverge: {}  order_diverges: {}  oracle_ranking_inconsistent: {}",
        cmp.totals_diverge, cmp.order_diverges, cmp.oracle_ranking_inconsistent
    ));

    let divergent = cmp.is_divergent();
    Ok(ReproReport { lines, engine_only, oracle_only, duplicates, divergent })
}

/// Stage-3 re-run: the three-way route check
/// ([`crate::routes3::three_way_route_check`]), with all three label sets
/// printed and the fresh diff in the writer's
/// [`crate::routes3::format_pair_diffs`] line format (`duplicates` is always
/// empty for `Routes3` artifacts, matching the writer).
fn repro_routes3(state: &GameState, rules: &Rules) -> Result<ReproReport, String> {
    let cmp = crate::routes3::three_way_route_check(state, rules).ok_or_else(|| {
        "Routes3 artifact's state snapshot does not qualify (needs Movement phase, empty \
         pending-choice queue, President on board) — cannot re-run the three-way check"
            .to_string()
    })?;

    let mut lines = vec![format!(
        "state: current_player={:?} pending_choices={:?}",
        state.current_player, state.pending_choices
    )];
    for (name, keys) in [
        ("engine", &cmp.engine),
        ("pruned-oracle", &cmp.pruned),
        ("pruneless-reference", &cmp.pruneless),
    ] {
        lines.push(String::new());
        lines.push(format!("== {name} labels ({}) ==", keys.len()));
        for k in keys {
            lines.push(format!("  {k}"));
        }
    }
    lines.push(String::new());
    lines.push("== RE-COMPUTED THREE-WAY DIFF ==".to_string());
    let (engine_only, oracle_only) = crate::routes3::format_pair_diffs(&cmp.diverged);
    push_keyed(&mut lines, "engine_only", &engine_only);
    push_keyed(&mut lines, "oracle_only", &oracle_only);

    let divergent = cmp.is_divergent();
    Ok(ReproReport { lines, engine_only, oracle_only, duplicates: Vec::new(), divergent })
}

/// Stage-1 enumeration re-run: `get_legal_moves` vs `oracle_moves` through
/// [`crate::compare::compare_moves`], with both sides' full lists printed.
fn repro_enumeration(state: &GameState, rules: &Rules) -> Result<ReproReport, String> {
    let engine_moves = yokohama_core::get_legal_moves(state, rules);
    let oracle = yokohama_oracle::oracle_moves(state, rules);
    let cmp = crate::compare::compare_moves(state, rules, &engine_moves, &oracle);

    let mut lines = vec![format!(
        "state: current_player={:?} pending_choices={:?}",
        state.current_player, state.pending_choices
    )];
    lines.push(String::new());
    lines.push(format!("== ENGINE ({} moves) ==", engine_moves.len()));
    for m in &engine_moves {
        lines.push(format!("  {m:?}"));
    }
    lines.push(String::new());
    lines.push(format!(
        "== ORACLE ({} exact + {} labels) ==",
        oracle.exact.len(),
        oracle.movement.len()
    ));
    for m in &oracle.exact {
        lines.push(format!("  {m:?}"));
    }
    for l in &oracle.movement {
        lines.push(format!("  {l:?}"));
    }
    lines.push(String::new());
    lines.push("== RE-COMPUTED DIFF ==".to_string());
    push_keyed(&mut lines, "engine_only", &cmp.engine_only);
    push_keyed(&mut lines, "oracle_only", &cmp.oracle_only);
    push_keyed(&mut lines, "duplicates", &cmp.duplicates);

    let divergent = cmp.is_divergent();
    Ok(ReproReport {
        lines,
        engine_only: cmp.engine_only,
        oracle_only: cmp.oracle_only,
        duplicates: cmp.duplicates,
        divergent,
    })
}

/// `"<name> (<n>):"` header plus one indented line per key.
fn push_keyed(lines: &mut Vec<String>, name: &str, keys: &[String]) {
    lines.push(format!("{name} ({}):", keys.len()));
    for k in keys {
        lines.push(format!("  {k}"));
    }
}
