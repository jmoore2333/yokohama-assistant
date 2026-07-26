//! Where an arena game's record goes, and what is allowed to go there (Phase
//! 3.2, spec `docs/plans/2026-07-25-phase3.2-arena-spec.md` §5).
//!
//! THE CORPUS IS TRAINING DATA, which is what makes this module more than a
//! `fs::write` wrapper. Everything filed here is a candidate input to 3.3, so
//! the layout has to keep a mixed-policy game expressible, the filenames have to
//! be collision-free across a whole rotation block, and privileged play has to
//! be structurally unable to arrive wearing an ordinary label.
//!
//! LAYOUT: `<dir>/rev<semantics_rev>/<players>P-<cell>/<world_seed:016x>-<arr>.json`.
//! Each segment answers a question a reader of one file would otherwise have to
//! guess:
//!
//!   * `rev<n>` — which executable semantics produced it. A record replays
//!     strict-clean only against its own `SEMANTICS_REV` (the rev-4→5 Canal
//!     alignment is the live precedent: legality moved, no reachable state did,
//!     and rev-4 saves stopped resuming), so mixing revs in one directory would
//!     build a corpus that cannot be replayed as a whole.
//!   * `<players>P-<cell>` — the MATCHUP, not just the policy. `ra-vs-fw` and
//!     `fw-selfplay` are different cells; a layout keyed on a single policy name
//!     could not name either, which is the lens repair spec §5 records.
//!   * `<seed:016x>-<arr>` — the design unit and the arrangement within it. A 4P
//!     rotation block's four games SHARE the hex seed and differ only in `-r0`
//!     … `-r3`; a 2P pair uses `-a0`/`-a1`. Zero-padded hex so the coordinate
//!     is fixed-width and sorts.
//!
//! COMPACT JSON, DELIBERATELY UNLIKE `Session::save`'s pretty output (measured
//! 2.29× smaller). The CLI writes save files a human opens in an editor; this
//! writes machine input at ~32–281 KB per game across thousands of games, and
//! the divergence is a size decision, not an oversight.
//!
//! THE CONTAINMENT RULE, and what it is worth. A `run_raw_game` record is
//! privileged play — the policy read the true `GameState` — so [`emit_record`]
//! refuses to file one under [`default_corpus_dir`] unless the cell name says
//! `-diagnostic`. Stated precisely: this catches the ORDINARY mistake, which is
//! pointing a screening run's `--emit` at the real corpus directory. It does not
//! defeat a caller who writes elsewhere and copies the file in, and it is not
//! meant to — the durable guarantee is that the LABEL rides the artifact
//! (`SeatProvenance::observation`), so a reader can always tell, and this check
//! is what makes something actually read it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::game::record::{GameRecord, OBSERVATION_FULL_STATE_DIAGNOSTIC};
use crate::game::seeds::WorldSeed;

/// The cell-name suffix that admits full-state-diagnostic records.
pub const DIAGNOSTIC_CELL_SUFFIX: &str = "-diagnostic";

/// The crate-local gitignored corpus directory, `crates/yokohama-core/corpus/`.
///
/// Computed in code rather than configured, mirroring difftest's
/// `default_artifacts_dir`: a generated-data directory that moves with the crate
/// cannot drift from the `.gitignore` entry that keeps it out of history.
pub fn default_corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus")
}

/// The canonical path for ONE game — see the module doc for what each segment
/// carries.
///
/// `arr` is the arrangement index WITHIN the design unit, and its letter names
/// the unit shape: `a` for a 2P pair, `r` for a 3–4P rotation block. Callers
/// pass `SEMANTICS_REV` for `rev`; it is a parameter so a tool can address an
/// older rev's corpus without lying about the running engine.
pub fn corpus_path(
    dir: &Path,
    rev: u32,
    players: u8,
    cell: &str,
    seed: WorldSeed,
    arr: u8,
) -> PathBuf {
    let arrangement = if players == 2 { 'a' } else { 'r' };
    dir.join(format!("rev{rev}"))
        .join(format!("{players}P-{cell}"))
        .join(format!("{:016x}-{arrangement}{arr}.json", seed.0))
}

/// Why `path` must not receive `record`, or `None` if it may.
///
/// A pure predicate with no I/O so the rule is testable without writing into the
/// repo's own corpus — and so [`emit_record`] can refuse BEFORE it creates a
/// directory, leaving nothing behind when it says no.
///
/// A record carrying NO provenance is not refused. Absence is not a diagnostic
/// label: every pre-3.2 record has none, as does every CLI save, and treating
/// silence as an accusation would refuse the ordinary case while still not
/// catching a mislabelled one.
pub fn corpus_refusal(path: &Path, record: &GameRecord) -> Option<String> {
    let diagnostic_seats: Vec<usize> = record
        .run
        .as_ref()?
        .seats
        .iter()
        .enumerate()
        .filter(|(_, seat)| seat.observation == OBSERVATION_FULL_STATE_DIAGNOSTIC)
        .map(|(i, _)| i)
        .collect();
    // ANY privileged seat makes the whole game privileged play: the other seats
    // played against a controller that could see their hidden information.
    if diagnostic_seats.is_empty() {
        return None;
    }
    if !path.starts_with(default_corpus_dir()) {
        return None;
    }
    let cell = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if cell.ends_with(DIAGNOSTIC_CELL_SUFFIX) {
        return None;
    }
    Some(format!(
        "refusing to file a {OBSERVATION_FULL_STATE_DIAGNOSTIC} record (seats {diagnostic_seats:?}) \
         into the corpus cell {cell:?}: privileged play is not training data — name the cell \
         \"{cell}{DIAGNOSTIC_CELL_SUFFIX}\" or emit outside {}",
        default_corpus_dir().display(),
    ))
}

/// Write `record` to `path` as compact JSON, creating the parent directories.
///
/// Refuses a full-state-diagnostic record filed as ordinary corpus data (see
/// [`corpus_refusal`]) with `ErrorKind::InvalidInput`, before touching the
/// filesystem.
///
/// Compact, not pretty — the deliberate divergence from `Session::save`, per the
/// module doc.
pub fn emit_record(path: &Path, record: &GameRecord) -> io::Result<()> {
    if let Some(reason) = corpus_refusal(path, record) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, reason));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string(record)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::record::GameRecord;
    use crate::game::rules::Rules;
    use crate::game::seeds::WorldSeed;
    use crate::game::semantics::SEMANTICS_REV;
    use crate::game::setup::StartingOrderChoice;
    use crate::types::GameConfig;
    use std::collections::HashSet;

    fn a_record() -> GameRecord {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let pending = crate::game::setup::new_game(config, &rules, 5).unwrap();
        GameRecord::new(
            5,
            pending.state.config.clone(),
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(1)],
            &rules,
        )
    }

    #[test]
    fn corpus_paths_are_unique_across_a_block() {
        let dir = default_corpus_dir();
        let seed = WorldSeed(0x0123_4567_89AB_CDEF);

        let block: Vec<_> =
            (0..4).map(|arr| corpus_path(&dir, SEMANTICS_REV, 4, "ra-vs-fw", seed, arr)).collect();
        let distinct: HashSet<_> = block.iter().collect();
        assert_eq!(distinct.len(), 4, "a rotation block's four games collided: {block:?}");

        for (arr, path) in block.iter().enumerate() {
            assert_eq!(
                path.file_name().unwrap().to_str().unwrap(),
                format!("0123456789abcdef-r{arr}.json"),
                "a block game is `<seed:016x>-r<arr>.json`",
            );
            assert_eq!(
                path.parent().unwrap(),
                dir.join(format!("rev{SEMANTICS_REV}")).join("4P-ra-vs-fw"),
                "every game of a block files under one cell directory",
            );
        }

        // 2P is the PAIR unit, and its arrangement letter says so.
        let pair: Vec<_> =
            (0..2).map(|arr| corpus_path(&dir, SEMANTICS_REV, 2, "ra-vs-fw", seed, arr)).collect();
        assert_eq!(pair[0].file_name().unwrap(), "0123456789abcdef-a0.json");
        assert_eq!(pair[1].file_name().unwrap(), "0123456789abcdef-a1.json");

        // The remaining coordinates separate too — a corpus that files two
        // different games at one path silently loses one of them (`fs::write`
        // truncates; the same hazard difftest's `artifact_kind` segment exists
        // for).
        let everything: HashSet<_> = [
            corpus_path(&dir, SEMANTICS_REV, 4, "ra-vs-fw", seed, 0),
            corpus_path(&dir, SEMANTICS_REV, 4, "fw-selfplay", seed, 0),
            corpus_path(&dir, SEMANTICS_REV + 1, 4, "ra-vs-fw", seed, 0),
            corpus_path(&dir, SEMANTICS_REV, 3, "ra-vs-fw", seed, 0),
            corpus_path(&dir, SEMANTICS_REV, 4, "ra-vs-fw", WorldSeed(seed.0 + 1), 0),
        ]
        .into_iter()
        .collect();
        assert_eq!(everything.len(), 5, "two corpus coordinates share a path");
    }

    #[test]
    fn emitted_files_are_compact_json() {
        let dir = std::env::temp_dir().join(format!("yokohama_corpus_compact_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let record = a_record();
        let path = corpus_path(&dir, SEMANTICS_REV, 2, "fw-selfplay", WorldSeed(5), 0);

        emit_record(&path, &record).expect("emission into a fresh directory must create it");
        let text = std::fs::read_to_string(&path).unwrap();

        assert!(!text.contains('\n'), "compact JSON is one line; got:\n{text}");
        assert!(!text.contains("  "), "compact JSON carries no indentation");
        assert!(
            text.len() < serde_json::to_string_pretty(&record).unwrap().len(),
            "the compact form must actually be smaller than Session::save's pretty form",
        );
        let round_tripped: GameRecord = serde_json::from_str(&text).unwrap();
        assert_eq!(round_tripped, record, "a compact file must read back identical");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
