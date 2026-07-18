//! Save/replay format for recorded games (spec F3, Phase 2's persistence
//! format).
//!
//! `GameState` is deliberately history-free (see crate docs) so it stays
//! cheap to clone for future MCTS -- `GameRecord` is the external log that
//! carries history instead: the seed and `GameConfig` `new_game` used, the
//! `StartingOrderChoice`s `finalize_setup` applied, and every event applied
//! after that, in order (ordinary `Move`s plus live-tracking reveal
//! reconciliations -- `RecordedEvent`). `replay` reruns exactly that
//! sequence and (since `new_game`/`finalize_setup`/`apply_move` and the
//! `GameState` reconciliation methods are all deterministic with no hidden
//! RNG or wall-clock input) reproduces the original `GameState` exactly,
//! per `GameState`'s full `PartialEq`.

use serde::{Deserialize, Serialize};

use crate::game::apply_move::apply_move;
use crate::game::rules::Rules;
use crate::game::setup::{finalize_setup, new_game, reconcile_setup, SetupObservations, StartingOrderChoice};
use crate::types::{AreaId, FivePowerEffect, GameConfig, GameState, Move, MoveError, OrderCardId, PlayerId, TechCardId};

/// One entry in a `GameRecord`'s event log. `Move` is ordinary play; the
/// `Reveal*` variants are live-tracking reconciliation events (design
/// 2026-07-18 §5.2) — they mutate hidden-information fiction (deck order,
/// hidden starting orders, face-down 5-Power tokens) and MUST be in the
/// record so `replay` reproduces the reconciled state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordedEvent {
    Move(Move),
    /// The next draws from the order deck are exactly these, in draw order.
    RevealOrders(Vec<OrderCardId>),
    /// The next draws from the tech deck are exactly these, in draw order.
    RevealTechs(Vec<TechCardId>),
    /// A hidden starting order became public.
    RevealStartingOrder { player: PlayerId, card: OrderCardId },
    /// A face-down 5-Power token was revealed at `area`.
    RevealFivePowerToken { area: AreaId, effect: FivePowerEffect },
}

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
    /// Every event applied after setup, in order — moves plus live-reveal reconciliations
    pub events: Vec<RecordedEvent>,
    /// Live-tracked games: physical-table observations reconciled into the
    /// seeded PendingSetup before finalize_setup. None = pure simulation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup_observations: Option<SetupObservations>,
}

impl GameRecord {
    /// Start a record for a game set up via `new_game(config, rules, seed)`
    /// then `finalize_setup(pending, &choices)`. `events` starts empty --
    /// call `push_move` after each `apply_move` (or `push_event` for a
    /// live-tracking reveal reconciliation).
    pub fn new(seed: u64, config: GameConfig, choices: Vec<StartingOrderChoice>) -> Self {
        Self {
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            seed,
            config,
            choices,
            events: Vec::new(),
            setup_observations: None,
        }
    }

    /// Append one applied move to the record's log.
    pub fn push_move(&mut self, mv: Move) {
        self.events.push(RecordedEvent::Move(mv));
    }

    /// Append one reconciliation/reveal event to the log.
    pub fn push_event(&mut self, ev: RecordedEvent) {
        self.events.push(ev);
    }
}

/// Reconstruct the `GameState` a `GameRecord` describes: `new_game`,
/// `finalize_setup`, then fold `record.events` over the resulting state in
/// order -- `RecordedEvent::Move` applies via `apply_move`; each `Reveal*`
/// variant applies via its matching `GameState` reconciliation method
/// (`arrange_orders_on_top`, `arrange_techs_on_top`, `reveal_starting_order`,
/// `reveal_five_power_token`).
///
/// Does not refuse to replay a record written by a different engine
/// version -- v1 only warns (to stderr) on a mismatch. A future engine
/// version could change data layout or apply a rules correction in a way
/// that makes an old record replay to a different state than the one that
/// produced it; Phase 2 may turn this into a hard error or a migration
/// step, but for now the record replays regardless.
///
/// Returns the first error encountered: a `new_game`/`finalize_setup`
/// failure surfaces as `MoveError::RuleViolation`; a bad event partway
/// through `record.events` surfaces as that event's own error and
/// short-circuits the loop -- events after the bad one never run.
pub fn replay(record: &GameRecord, rules: &Rules) -> Result<GameState, MoveError> {
    let running_version = env!("CARGO_PKG_VERSION");
    if record.engine_version != running_version {
        eprintln!(
            "warning: GameRecord.engine_version=\"{}\" does not match running engine version \"{running_version}\" -- replaying anyway (v1 does not refuse on a version mismatch)",
            record.engine_version,
        );
    }

    let mut pending = new_game(record.config.clone(), rules, record.seed).map_err(MoveError::RuleViolation)?;
    if let Some(obs) = &record.setup_observations {
        pending = reconcile_setup(pending, obs, rules).map_err(MoveError::RuleViolation)?;
    }
    let mut state = finalize_setup(pending, &record.choices).map_err(MoveError::RuleViolation)?;
    for ev in &record.events {
        state = match ev {
            RecordedEvent::Move(mv) => apply_move(&state, mv, rules)?,
            RecordedEvent::RevealOrders(cards) => {
                let mut s = state;
                s.arrange_orders_on_top(cards)?;
                s
            }
            RecordedEvent::RevealTechs(cards) => {
                let mut s = state;
                s.arrange_techs_on_top(cards)?;
                s
            }
            RecordedEvent::RevealStartingOrder { player, card } => {
                let mut s = state;
                s.reveal_starting_order(*player, *card)?;
                s
            }
            RecordedEvent::RevealFivePowerToken { area, effect } => {
                let mut s = state;
                s.reveal_five_power_token(*area, *effect)?;
                s
            }
        };
    }
    Ok(state)
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
        assert!(record.events.is_empty());
    }

    #[test]
    fn push_move_appends_in_order() {
        let mut record = GameRecord::new(0, GameConfig::for_player_count(2), vec![]);
        record.push_move(Move::EndAdditionalPhase);
        record.push_event(RecordedEvent::RevealOrders(vec![]));
        assert_eq!(
            record.events,
            vec![RecordedEvent::Move(Move::EndAdditionalPhase), RecordedEvent::RevealOrders(vec![])]
        );
    }

    #[test]
    fn replay_applies_reveal_events() {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let pending = crate::game::setup::new_game(config.clone(), &rules, 3).unwrap();
        let expected_top = pending.state.order_deck[0]; // bottom card, definitely not next draw
        let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)];
        let mut record = GameRecord::new(3, config, choices);
        record.push_event(RecordedEvent::RevealOrders(vec![expected_top]));
        let state = replay(&record, &rules).unwrap();
        assert_eq!(state.order_deck.last(), Some(&expected_top));
    }

    #[test]
    fn replay_applies_setup_observations() {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let table = crate::game::setup::new_game(config.clone(), &rules, 99).unwrap();
        let obs = crate::game::setup::SetupObservations {
            port_a: Some(std::array::from_fn(|i| table.state.port_state.port_a.slots[i].unwrap())),
            ..Default::default()
        };
        let mut record = GameRecord::new(11, config, vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)]);
        record.setup_observations = Some(obs.clone());
        let state = replay(&record, &rules).unwrap();
        for i in 0..6 {
            assert_eq!(state.port_state.port_a.slots[i], Some(obs.port_a.unwrap()[i]));
        }
    }
}
