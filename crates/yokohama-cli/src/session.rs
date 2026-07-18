//! `Session`: the simulation/live-tracking core the Task-11 REPL drives.
//! Wraps `{Rules, GameRecord, GameState}` with atomic apply/undo/save/load
//! (design docs/plans/2026-07-18-phase2-cli-driver-design.md).

use yokohama_core::*;

/// Whether a `Session` is playing a simulated game (all hidden information
/// engine-generated) or tracking a live physical game (opponent state
/// reconciled from table observations via `SetupObservations`/`Reveal*`
/// events).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackingMode {
    Simulation,
    Live,
}

/// The simulation/live-tracking core: `{Rules, GameRecord, GameState}` kept
/// in lockstep. Standing invariant: `state` always equals what replaying
/// `record` against `rules` produces. `apply`/`apply_standalone`/`undo`
/// maintain it by staging changes on a clone of `state` (or of `record`'s
/// events, for `undo`) and only committing after every check passes --
/// nothing is ever recorded/committed on an error path.
pub struct Session {
    rules: Rules,
    record: GameRecord,
    state: GameState,
    pub mode: TrackingMode,
    pub save_path: Option<std::path::PathBuf>,
}

impl Session {
    /// Start a new simulated game: `new_game` + `finalize_setup` under a
    /// fresh `GameRecord`. Both players' hidden information is
    /// engine-generated, so there is nothing to reconcile.
    pub fn new_sim(
        players: u8,
        seed: u64,
        choices: Vec<StartingOrderChoice>,
        rules: Rules,
    ) -> Result<Session, String> {
        let config = GameConfig::for_player_count(players);
        let pending = new_game(config.clone(), &rules, seed).map_err(|e| e.to_string())?;
        let state = finalize_setup(pending, &choices).map_err(|e| e.to_string())?;
        let record = GameRecord::new(seed, config, choices);
        Ok(Session { rules, record, state, mode: TrackingMode::Simulation, save_path: None })
    }

    /// Start a new live-tracked game: `new_game`, reconcile physical-table
    /// `obs` into the seeded `PendingSetup` (`reconcile_setup`), then
    /// `finalize_setup`. `obs` is carried in the record so `replay`
    /// reproduces the reconciled state.
    pub fn new_live(
        config: GameConfig,
        seed: u64,
        obs: SetupObservations,
        choices: Vec<StartingOrderChoice>,
        rules: Rules,
    ) -> Result<Session, String> {
        let pending = new_game(config.clone(), &rules, seed).map_err(|e| e.to_string())?;
        let pending = reconcile_setup(pending, &obs, &rules).map_err(|e| e.to_string())?;
        let state = finalize_setup(pending, &choices).map_err(|e| e.to_string())?;
        let mut record = GameRecord::new(seed, config, choices);
        record.setup_observations = Some(obs);
        Ok(Session { rules, record, state, mode: TrackingMode::Live, save_path: None })
    }

    /// Load a saved `GameRecord` from `path` and replay it to reconstruct
    /// the `Session`. Mode is inferred: `Live` iff
    /// `record.setup_observations.is_some()`. Runs `check_invariants` on
    /// the replayed state before returning, so a corrupt save file surfaces
    /// as a load error rather than propagating silently into play.
    pub fn load(path: &std::path::Path, rules: Rules) -> Result<Session, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let record: GameRecord = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let state = replay(&record, &rules).map_err(|e| e.to_string())?;
        check_invariants(&state, &rules)?;
        let mode = if record.setup_observations.is_some() { TrackingMode::Live } else { TrackingMode::Simulation };
        Ok(Session { rules, record, state, mode, save_path: Some(path.to_path_buf()) })
    }

    /// Write the record as pretty-printed JSON to `path`. `load`
    /// reconstructs `state` by replaying the record -- `state` itself is
    /// never serialized.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(&self.record).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| e.to_string())
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn rules(&self) -> &Rules {
        &self.rules
    }

    pub fn record(&self) -> &GameRecord {
        &self.record
    }

    pub fn legal(&self) -> Vec<Move> {
        get_legal_moves(&self.state, &self.rules)
    }

    pub fn is_over(&self) -> bool {
        is_game_over(&self.state)
    }

    /// Apply reveals+move as ONE atomic group: reveals fold first, then `mv`
    /// is checked against `get_legal_moves` on the post-reveal state (the
    /// CLI never applies a move `get_legal_moves` didn't offer -- this is
    /// what makes that a structural guarantee instead of caller discipline),
    /// then applied; invariants checked; on any error nothing is
    /// recorded/committed. Returns `delta_summary` lines, plus an
    /// autosave-failure warning line when `save_path` is set and the write
    /// fails.
    pub fn apply(&mut self, mv: Move, reveals: Vec<RecordedEvent>) -> Result<Vec<String>, String> {
        let mut staged = self.state.clone();
        for ev in &reveals {
            match ev {
                RecordedEvent::RevealOrders(c) => staged.arrange_orders_on_top(c).map_err(|e| e.to_string())?,
                RecordedEvent::RevealTechs(c) => staged.arrange_techs_on_top(c).map_err(|e| e.to_string())?,
                RecordedEvent::RevealFivePowerToken { area, effect } => {
                    staged.reveal_five_power_token(*area, *effect).map_err(|e| e.to_string())?
                }
                other => return Err(format!("event not allowed as a bound reveal: {other:?}")),
            }
        }
        // The engine's apply_move does not itself validate that `mv` matches
        // the current phase (MoveError::WrongPhase is dead code -- see
        // guidingpath's decision log; deliberately deferred at the engine
        // level). Session enforces the plan's global constraint here
        // instead: never apply a move get_legal_moves didn't offer. Checked
        // against `staged` (post-reveal), since the group is atomic and
        // legality is defined on the state the move will actually run
        // against.
        if !get_legal_moves(&staged, &self.rules).contains(&mv) {
            return Err(format!("move not legal in the current state: {mv:?}"));
        }
        let next = apply_move(&staged, &mv, &self.rules).map_err(|e| e.to_string())?;
        check_invariants(&next, &self.rules)
            .map_err(|e| format!("INVARIANT VIOLATION after apply (driver/engine bug): {e}"))?;
        let mut delta = crate::render::delta_summary(&self.state, &next, &self.rules);
        for ev in reveals {
            self.record.push_event(ev);
        }
        self.record.push_move(mv);
        self.state = next;
        if let Some(warning) = self.autosave() {
            delta.push(warning);
        }
        Ok(delta)
    }

    /// Apply a standalone reveal event (`RevealStartingOrder`) atomically:
    /// staged clone -> `reveal_starting_order` -> invariants -> commit ->
    /// autosave. Any other variant is rejected -- those are bound reveals,
    /// only valid as part of `apply`'s group.
    pub fn apply_standalone(&mut self, ev: RecordedEvent) -> Result<Vec<String>, String> {
        let (player, card) = match &ev {
            RecordedEvent::RevealStartingOrder { player, card } => (*player, *card),
            other => return Err(format!("event not allowed as a standalone reveal: {other:?}")),
        };
        let mut staged = self.state.clone();
        staged.reveal_starting_order(player, card).map_err(|e| e.to_string())?;
        check_invariants(&staged, &self.rules)
            .map_err(|e| format!("INVARIANT VIOLATION after apply (driver/engine bug): {e}"))?;
        let mut delta = crate::render::delta_summary(&self.state, &staged, &self.rules);
        self.record.push_event(ev);
        self.state = staged;
        if let Some(warning) = self.autosave() {
            delta.push(warning);
        }
        Ok(delta)
    }

    /// Replay-prefix undo of the last group (design §5.4): a trailing
    /// `Move` takes its contiguous immediately-preceding bound reveals
    /// (`RevealOrders`/`RevealTechs`/`RevealFivePowerToken`) with it; a
    /// trailing standalone reveal (`RevealStartingOrder`, or a bound reveal
    /// left with no following move -- possible only after a mid-group
    /// crash) pops alone. Err when the log is empty. This is the one path
    /// that can't stage-then-commit on `state` directly -- undo's job is to
    /// move `state` backward, so it stages on a clone of `record` instead
    /// and re-derives `state` via `replay`.
    pub fn undo(&mut self) -> Result<(), String> {
        let mut record = self.record.clone();
        match record.events.pop() {
            None => return Err("nothing to undo".to_string()),
            Some(RecordedEvent::Move(_)) => {
                while matches!(
                    record.events.last(),
                    Some(RecordedEvent::RevealOrders(_))
                        | Some(RecordedEvent::RevealTechs(_))
                        | Some(RecordedEvent::RevealFivePowerToken { .. })
                ) {
                    record.events.pop();
                }
            }
            Some(_) => {} // standalone reveal (or an orphaned bound reveal): pops alone
        }
        let state = replay(&record, &self.rules).map_err(|e| e.to_string())?;
        self.record = record;
        self.state = state;
        // autosave failure must stay visible, but `undo`'s signature is
        // pinned by the Task-11 REPL (which matches on `Ok(())`) -- eprintln
        // is a known last-resort wart here, not a design choice; logged for
        // the final whole-branch review rather than redesigned around.
        if let Some(warning) = self.autosave() {
            eprintln!("{warning}");
        }
        Ok(())
    }

    pub fn final_scores(&self) -> Vec<(PlayerId, u32)> {
        calculate_final_scores(&self.state, &self.rules.orders, &self.rules.techs)
    }

    /// Best-effort write to `save_path` (`None` when unset, or when the
    /// write succeeds). A failure is returned rather than swallowed so the
    /// caller can surface it -- a silent autosave failure would mean a
    /// crash loses more than the player thinks.
    fn autosave(&self) -> Option<String> {
        let path = self.save_path.as_ref()?;
        match self.save(path) {
            Ok(()) => None,
            Err(e) => Some(format!("WARNING: autosave failed: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `session`'s own top-level already glob-imports yokohama_core (needed
    // by the impl code), and `super::*` carries that into this module too --
    // this second import is the brief's Step-1 test text verbatim, kept
    // as-is and just silenced rather than diverging from the spec.
    #[allow(unused_imports)]
    use yokohama_core::*;

    fn sim_2p(seed: u64) -> Session {
        Session::new_sim(
            2, seed,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        ).unwrap()
    }

    #[test]
    fn apply_then_undo_restores_previous_state() {
        let mut s = sim_2p(4);
        let s0 = s.state().clone();
        let mv = s.legal()[0].clone();
        let delta = s.apply(mv, vec![]).unwrap();
        assert!(!delta.is_empty());
        assert_ne!(*s.state(), s0);
        s.undo().unwrap();
        assert_eq!(*s.state(), s0);
        assert!(s.record().events.is_empty());
        assert!(s.undo().is_err()); // nothing left
    }

    #[test]
    fn illegal_move_is_rejected_atomically() {
        let mut s = sim_2p(4);
        let before_state = s.state().clone();
        let before_events = s.record().events.len();
        // A Building-phase move is never legal in AdditionalActionFirst.
        assert!(s.apply(Move::SkipBuilding, vec![]).is_err());
        assert_eq!(*s.state(), before_state);
        assert_eq!(s.record().events.len(), before_events);
    }

    #[test]
    fn save_load_round_trips_state_and_mode() {
        let mut s = sim_2p(4);
        for _ in 0..5 {
            let mv = s.legal()[0].clone();
            s.apply(mv, vec![]).unwrap();
        }
        let path = std::env::temp_dir().join(format!("yokohama_test_{}.json", std::process::id()));
        s.save(&path).unwrap();
        let loaded = Session::load(&path, Rules::standard()).unwrap();
        assert_eq!(loaded.state(), s.state());
        assert!(matches!(loaded.mode, TrackingMode::Simulation));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn autosave_writes_after_each_apply() {
        let mut s = sim_2p(4);
        let path = std::env::temp_dir().join(format!("yokohama_autosave_{}.json", std::process::id()));
        s.save_path = Some(path.clone());
        let mv = s.legal()[0].clone();
        s.apply(mv, vec![]).unwrap();
        assert!(path.exists());
        let loaded = Session::load(&path, Rules::standard()).unwrap();
        assert_eq!(loaded.state(), s.state());
        std::fs::remove_file(&path).ok();
    }

    /// The CLI-layer analog of the fuzz driver: a full random game driven
    /// entirely through Session::apply completes and its record replays.
    #[test]
    fn full_random_game_through_session_completes_and_replays() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(0);
        let mut s = sim_2p(0);
        let mut applied = 0u32;
        while !s.is_over() {
            assert!(s.state().turn_number <= 2000, "non-termination");
            let legal = s.legal();
            assert!(!legal.is_empty(), "dead end at turn {}", s.state().turn_number);
            let mv = legal[rng.gen_range(0..legal.len())].clone();
            s.apply(mv, vec![]).unwrap();
            applied += 1;
        }
        assert!(applied > 100);
        assert_eq!(replay(s.record(), s.rules()).unwrap(), *s.state());
        assert!(!s.final_scores().is_empty());
    }
}
