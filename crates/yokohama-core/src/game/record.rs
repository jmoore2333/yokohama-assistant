//! Save/replay format for recorded games (spec F3, Phase 2's persistence
//! format).
//!
//! `GameState` is deliberately history-free (see crate docs) so it stays
//! cheap to clone for future MCTS -- `GameRecord` is the external log that
//! carries history instead: the seed and `GameConfig` `new_game` used, the
//! `StartingOrderChoice`s `finalize_setup` applied, and every `Move` applied
//! after that, in order. `replay` reruns exactly that sequence and (since
//! `new_game`/`finalize_setup`/`apply_move` are all deterministic pure
//! functions with no hidden RNG or wall-clock input) reproduces the
//! original `GameState` exactly, per `GameState`'s full `PartialEq`.

use serde::{Deserialize, Serialize};

use crate::game::apply_move::apply_move;
use crate::game::rules::Rules;
use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
use crate::types::{GameConfig, GameState, Move, MoveError};

/// A complete, replayable record of one game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRecord {
    /// `env!("CARGO_PKG_VERSION")` of the engine build that produced this
    /// record. `replay` compares this to the running engine's version and
    /// warns on mismatch, but does not refuse to replay (see `replay`'s doc
    /// comment) -- that tightening is left to Phase 2.
    pub engine_version: String,
    /// Seed passed to `new_game`.
    pub seed: u64,
    /// Config passed to `new_game`.
    pub config: GameConfig,
    /// Per-player `StartingOrderChoice`s passed to `finalize_setup`, in
    /// `PlayerId` order.
    pub choices: Vec<StartingOrderChoice>,
    /// Every `Move` applied after setup, in application order.
    pub moves: Vec<Move>,
}

impl GameRecord {
    /// Start a record for a game set up via `new_game(config, rules, seed)`
    /// then `finalize_setup(pending, &choices)`. `moves` starts empty --
    /// call `push_move` after each `apply_move`.
    pub fn new(seed: u64, config: GameConfig, choices: Vec<StartingOrderChoice>) -> Self {
        Self {
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            seed,
            config,
            choices,
            moves: Vec::new(),
        }
    }

    /// Append one applied move to the record's log.
    pub fn push_move(&mut self, mv: Move) {
        self.moves.push(mv);
    }
}

/// Reconstruct the `GameState` a `GameRecord` describes: `new_game`,
/// `finalize_setup`, then fold `apply_move` over `record.moves` in order.
///
/// Does not refuse to replay a record written by a different engine
/// version -- v1 only warns (to stderr) on a mismatch. A future engine
/// version could change data layout or apply a rules correction in a way
/// that makes an old record replay to a different state than the one that
/// produced it; Phase 2 may turn this into a hard error or a migration
/// step, but for now the record replays regardless.
///
/// Returns the first error encountered: a `new_game`/`finalize_setup`
/// failure surfaces as `MoveError::RuleViolation`; a bad move partway
/// through `record.moves` surfaces as that move's own `apply_move` error
/// (via `try_fold`'s short-circuit -- moves after the bad one never run).
pub fn replay(record: &GameRecord, rules: &Rules) -> Result<GameState, MoveError> {
    let running_version = env!("CARGO_PKG_VERSION");
    if record.engine_version != running_version {
        eprintln!(
            "warning: GameRecord.engine_version=\"{}\" does not match running engine version \"{running_version}\" -- replaying anyway (v1 does not refuse on a version mismatch)",
            record.engine_version,
        );
    }

    let pending = new_game(record.config.clone(), rules, record.seed).map_err(MoveError::RuleViolation)?;
    let state = finalize_setup(pending, &record.choices).map_err(MoveError::RuleViolation)?;

    record.moves.iter().try_fold(state, |state, mv| apply_move(&state, mv, rules))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_with_empty_moves_and_current_engine_version() {
        let config = GameConfig::for_player_count(2);
        let record =
            GameRecord::new(7, config.clone(), vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(1)]);
        assert_eq!(record.engine_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(record.seed, 7);
        assert_eq!(record.config, config);
        assert!(record.moves.is_empty());
    }

    #[test]
    fn push_move_appends_in_order() {
        let mut record = GameRecord::new(0, GameConfig::for_player_count(2), vec![]);
        record.push_move(Move::EndAdditionalPhase);
        record.push_move(Move::Skip5PowerBonus);
        assert_eq!(record.moves, vec![Move::EndAdditionalPhase, Move::Skip5PowerBonus]);
    }
}
