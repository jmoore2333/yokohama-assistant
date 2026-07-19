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

/// Why `Session::apply`/`apply_standalone` could not commit (Task 13, design
/// §5.3-7). The REPL (`repl::Repl::finish_apply`) matches on the variant to
/// choose user-facing text and, for `Illegal`, to tell a live-mode desync
/// guard (`reveal::lock_on_order_desync`/`lock_on_tech_desync`) that nothing
/// actually applied, so it should skip re-checking a board that never
/// changed -- replacing an earlier version that sniffed formatted text for
/// the substring "Illegal:" for the same purpose.
#[derive(Debug)]
pub enum ApplyError {
    /// `is_over()` was already true when `apply`/`apply_standalone` was
    /// called -- nothing was staged, checked, or attempted.
    GameOver,
    /// The move itself was illegal: not offered by `get_legal_moves` on the
    /// post-reveal state, or rejected by `apply_move`.
    Illegal(String),
    /// `check_invariants` failed on the state produced by an otherwise-legal
    /// apply -- a driver/engine bug, not a legality problem. The caller
    /// locks the session down on this variant rather than just rejecting
    /// the one move.
    Invariant(String),
    /// A reveal event failed to fold onto the staged state (`apply`'s bound
    /// reveals, or `apply_standalone`'s `RevealStartingOrder`), or wasn't a
    /// variant valid in that position.
    Reveal(String),
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplyError::GameOver => write!(f, "the game is already over"),
            ApplyError::Illegal(msg) => write!(f, "{msg}"),
            ApplyError::Invariant(msg) => write!(f, "{msg}"),
            ApplyError::Reveal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ApplyError {}

/// The simulation/live-tracking core: `{Rules, GameRecord, GameState}` kept
/// in lockstep. Standing invariant: `state` always equals what replaying
/// `record` against `rules` produces FROM ITS BASE -- for every session
/// built by `new_sim`/`new_live`/`load` that base is `replay`'s own
/// `new_game`+`finalize_setup`; for a `for_test_from_state` session
/// (test-only) it's `checkpoint` instead, since `replay` has no
/// `(seed, config, choices)` that reconstructs a hand-built state.
/// `apply`/`apply_standalone`/`undo` maintain it by staging changes on a
/// clone of `state` (or of `record`'s events, for `undo`) and only
/// committing after every check passes -- nothing is ever
/// recorded/committed on an error path.
pub struct Session {
    rules: Rules,
    record: GameRecord,
    state: GameState,
    pub mode: TrackingMode,
    pub save_path: Option<std::path::PathBuf>,
    /// Set only by `for_test_from_state` (test-only, Task 11 fix wave):
    /// the state `undo` bottoms out at once `record.events` is fully
    /// unwound, in place of `replay`'s normal `new_game`+`finalize_setup`
    /// base. `None` for every session built by `new_sim`/`new_live`/`load`
    /// -- those always bottom out at a state `replay` DOES reconstruct
    /// correctly, so `undo` uses the real `replay` function for them
    /// unchanged.
    checkpoint: Option<GameState>,
}

impl Session {
    /// Start a new simulated game: `new_game` + `finalize_setup` under a
    /// fresh `GameRecord`. Both players' hidden information is
    /// engine-generated, so there is nothing to reconcile. `config` is taken
    /// as-built by the caller (mirrors `new_live`) rather than rederived from
    /// a bare player count, so wizard-collected seat colors/start seat are
    /// honored instead of silently falling back to `for_player_count`'s
    /// defaults.
    pub fn new_sim(
        config: GameConfig,
        seed: u64,
        choices: Vec<StartingOrderChoice>,
        rules: Rules,
    ) -> Result<Session, String> {
        let pending = new_game(config.clone(), &rules, seed).map_err(|e| e.to_string())?;
        let state = finalize_setup(pending, &choices).map_err(|e| e.to_string())?;
        let record = GameRecord::new(seed, config, choices, &rules);
        Ok(Session { rules, record, state, mode: TrackingMode::Simulation, save_path: None, checkpoint: None })
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
        let mut record = GameRecord::new(seed, config, choices, &rules);
        record.setup_observations = Some(obs);
        Ok(Session { rules, record, state, mode: TrackingMode::Live, save_path: None, checkpoint: None })
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
        Ok(Session { rules, record, state, mode, save_path: Some(path.to_path_buf()), checkpoint: None })
    }

    /// Write the record as pretty-printed JSON to `path`. `load`
    /// reconstructs `state` by replaying the record -- `state` itself is
    /// never serialized.
    ///
    /// Atomic write-then-rename: the JSON is written to a sibling `.tmp`
    /// file first, `fsync`ed, then renamed onto `path`. A crash mid-write
    /// leaves only the `.tmp` file corrupt (or absent) and `path` untouched
    /// -- `path` itself is never opened for writing, so live-mode autosave
    /// (which rewrites the ONLY copy of a tracked game after every apply)
    /// can't have its sole save corrupted by a crash mid-write. The `fsync`
    /// (Task 13) closes the remaining gap: without it, the `.tmp` file's
    /// bytes can still be sitting in the OS page cache (not yet on disk)
    /// when `rename` runs, so a crash/power-loss right after a "successful"
    /// save could still rename a not-yet-durable file onto `path`.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        use std::io::Write;
        let text = serde_json::to_string_pretty(&self.record).map_err(|e| e.to_string())?;
        let mut tmp = path.as_os_str().to_os_string();
        tmp.push(".tmp");
        let tmp = std::path::PathBuf::from(tmp);
        let mut file =
            std::fs::File::create(&tmp).map_err(|e| format!("create {} failed: {e}", tmp.display()))?;
        file.write_all(text.as_bytes()).map_err(|e| format!("write to {} failed: {e}", tmp.display()))?;
        file.sync_all().map_err(|e| format!("fsync {} failed: {e}", tmp.display()))?;
        drop(file);
        std::fs::rename(&tmp, path)
            .map_err(|e| format!("rename {} -> {} failed: {e}", tmp.display(), path.display()))
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
    /// fails. Gated by an explicit `is_over()` check FIRST (Task 13) --
    /// once the game has ended there is nothing left for `get_legal_moves`/
    /// `apply_move` to legitimately do, so this reports the unambiguous
    /// `ApplyError::GameOver` rather than whatever incidental error those
    /// would happen to produce post-game.
    pub fn apply(&mut self, mv: Move, reveals: Vec<RecordedEvent>) -> Result<Vec<String>, ApplyError> {
        if self.is_over() {
            return Err(ApplyError::GameOver);
        }
        let mut staged = self.state.clone();
        for ev in &reveals {
            match ev {
                RecordedEvent::RevealOrders(c) => {
                    staged.arrange_orders_on_top(c).map_err(|e| ApplyError::Reveal(e.to_string()))?
                }
                RecordedEvent::RevealTechs(c) => {
                    staged.arrange_techs_on_top(c).map_err(|e| ApplyError::Reveal(e.to_string()))?
                }
                RecordedEvent::RevealFivePowerToken { area, effect } => staged
                    .reveal_five_power_token(*area, *effect)
                    .map_err(|e| ApplyError::Reveal(e.to_string()))?,
                other => return Err(ApplyError::Reveal(format!("event not allowed as a bound reveal: {other:?}"))),
            }
        }
        // The engine's apply_move does not itself validate that `mv` matches
        // the current phase (that check was MoveError::WrongPhase, deleted
        // as dead code at Task 13 -- design §5.3-7, guidingpath's decision
        // log). Session enforces the plan's global constraint here instead:
        // never apply a move get_legal_moves didn't offer. Checked against
        // `staged` (post-reveal), since the group is atomic and legality is
        // defined on the state the move will actually run against.
        if !get_legal_moves(&staged, &self.rules).contains(&mv) {
            return Err(ApplyError::Illegal(format!("move not legal in the current state: {mv:?}")));
        }
        // apply_move_unchecked: the gate just above already proved
        // membership in get_legal_moves on this same staged state, so
        // routing through checked apply_move would double-enumerate
        // get_legal_moves on every interactive apply.
        let next =
            apply_move_unchecked(&staged, &mv, &self.rules).map_err(|e| ApplyError::Illegal(e.to_string()))?;
        check_invariants(&next, &self.rules)
            .map_err(|e| ApplyError::Invariant(format!("INVARIANT VIOLATION after apply (driver/engine bug): {e}")))?;
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
    /// only valid as part of `apply`'s group. Same `is_over()` gate as
    /// `apply` (Task 13), for the same reason: a hidden order revealed after
    /// the game has ended is not a real driver action.
    pub fn apply_standalone(&mut self, ev: RecordedEvent) -> Result<Vec<String>, ApplyError> {
        if self.is_over() {
            return Err(ApplyError::GameOver);
        }
        let (player, card) = match &ev {
            RecordedEvent::RevealStartingOrder { player, card } => (*player, *card),
            other => return Err(ApplyError::Reveal(format!("event not allowed as a standalone reveal: {other:?}"))),
        };
        let mut staged = self.state.clone();
        staged.reveal_starting_order(player, card).map_err(|e| ApplyError::Reveal(e.to_string()))?;
        check_invariants(&staged, &self.rules)
            .map_err(|e| ApplyError::Invariant(format!("INVARIANT VIOLATION after apply (driver/engine bug): {e}")))?;
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
    /// and re-derives `state`: via `replay` (from `new_game`+`finalize_setup`)
    /// for a normal session, or via core's `replay_from` folding the
    /// truncated `record.events` over `checkpoint` for a
    /// `for_test_from_state` session (Task 13: previously a hand-kept
    /// CLI-side mirror of `replay`'s own fold loop, `replay_from_checkpoint`;
    /// now both paths share the one fold core exposes).
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
        let state = match &self.checkpoint {
            Some(base) => replay_from(base, &record.events, &self.rules).map_err(|e| e.to_string())?,
            None => replay(&record, &self.rules).map_err(|e| e.to_string())?,
        };
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
impl Session {
    /// Test-only (Task 11 fix wave): build a `Session` whose state is
    /// `state` verbatim -- not derived via `new_game`/`finalize_setup` --
    /// for scripted tests that need to exercise a rare engine state (e.g.
    /// a multi-deep `PendingChoice` cascade) with no short, real move
    /// sequence available to reach it and record instead (see the Task 11
    /// fix-wave report for the specific case: `PendingChoice::AnyGood`
    /// needs a Power-4+ Shophouse build, itself needing Telegram tech or
    /// an already-built Shophouse/Trading House, both multi-turn).
    ///
    /// `apply` needs no special-casing -- it only ever transforms
    /// `self.state` forward via `apply_move`, which doesn't care whether
    /// `state` came from `new_game` or was hand-built. `undo` does need
    /// `checkpoint` (see its doc comment): `replay` has no
    /// `(seed, config, choices)` that would reconstruct a hand-built
    /// state, so `record`'s seed/config/choices here are placeholders,
    /// never consulted for a checkpoint session.
    ///
    /// Does NOT round-trip through `save`/`load`: `checkpoint` isn't part
    /// of the serialized `GameRecord`, so a saved-then-reloaded checkpoint
    /// session's `undo` would silently fall back to normal
    /// (`new_game`-based) `replay` and produce the wrong state. Fine for
    /// this constructor's actual use -- short-lived in-test sessions that
    /// are never saved -- but not a general-purpose checkpoint/resume
    /// feature as-is.
    pub(crate) fn for_test_from_state(state: GameState, rules: Rules) -> Session {
        let record = GameRecord::new(0, state.config.clone(), Vec::new(), &rules);
        Session {
            rules,
            record,
            checkpoint: Some(state.clone()),
            state,
            mode: TrackingMode::Simulation,
            save_path: None,
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
            GameConfig::for_player_count(2), seed,
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

    /// Task 13 test (c): the illegal-move outcome is the typed `Illegal`
    /// variant, not just "some String error" -- `lock_on_order_desync`
    /// (reveal.rs) keys its skip-check on this variant instead of sniffing
    /// formatted text for the substring "Illegal:".
    #[test]
    fn illegal_move_outcome_is_typed_illegal() {
        let mut s = sim_2p(4);
        let err = s.apply(Move::SkipBuilding, vec![]).unwrap_err();
        assert!(matches!(err, ApplyError::Illegal(_)), "{err:?}");
    }

    /// Task 13 test (b): a finished game's `apply` returns the typed
    /// `GameOver` outcome via an explicit `is_over()` gate, rather than
    /// falling through to `get_legal_moves`/`apply_move` and surfacing
    /// whatever incidental error those produce post-game. Built via
    /// `for_test_from_state` (see its doc comment) with the exact fields
    /// `is_game_over` checks flipped directly -- cheaper and more targeted
    /// than driving a real game to completion.
    #[test]
    fn apply_on_finished_game_returns_typed_game_over() {
        let mut state = create_test_game_state(2);
        state.end_game_triggered = true;
        state.final_round = true;
        state.turn_phase = TurnPhase::TurnEnd;
        // is_game_over requires current_player == the seat before start_player
        // (2p, start_player 0 -> that's seat 1) at TurnEnd in the final round.
        state.current_player = PlayerId::new(1);
        assert!(is_game_over(&state), "fixture must actually satisfy is_game_over");

        let mut s = Session::for_test_from_state(state, Rules::empty_for_tests());
        let err = s.apply(Move::EndAdditionalPhase, vec![]).unwrap_err();
        assert!(matches!(err, ApplyError::GameOver), "{err:?}");
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
        // Atomic write-then-rename (final-review Important finding): the
        // intermediate ".tmp" file must never survive a successful save --
        // only the renamed-into-place target should remain.
        let mut tmp = path.clone().into_os_string();
        tmp.push(".tmp");
        assert!(
            !std::path::Path::new(&tmp).exists(),
            "a successful save must not leave a .tmp file behind"
        );
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

    /// Drives seed 4's 2p game (this file's `sim_2p` fixture seed) up
    /// through a Church action that pays copper+silk+yen (3 distinct
    /// payment types) at turn-1 power 3 -> Faith 6, which lands on 3
    /// eligible spaces (4/5/6) on the fresh track and so
    /// queues `PendingChoice::ChurchSpace { total_faith: 6 }` rather than
    /// placing inline -- mirrors `repl.rs`'s
    /// `drive_to_double_assistant_move_cascade` recipe verbatim but stops
    /// one step earlier (before resolving the ChurchSpace choice, which
    /// that fixture goes on to do). Verified deterministic for seed 4, same
    /// precedent as that fixture's own doc comment.
    fn session_with_church_space_choice_queued() -> Session {
        let mut s = sim_2p(4);
        let church_area = *s.state().board_layout.areas.iter()
            .find(|(_, c)| c.area_type == AreaType::Church)
            .expect("this board has a Church area").0;

        if !s.legal().iter().any(|m| matches!(m, Move::PlaceTwoSameArea { .. })) {
            let end = s.legal().into_iter().find(|m| matches!(m, Move::EndAdditionalPhase))
                .expect("EndAdditionalPhase reaches Placement");
            s.apply(end, vec![]).unwrap();
        }
        let place = s.legal().into_iter().find(|m| matches!(m,
            Move::PlaceTwoSameArea { area, adjacent_bonus: None } if *area == church_area
        )).expect("Option B at Church is legal turn 1");
        s.apply(place, vec![]).unwrap();

        let pres = s.legal().into_iter().find(|m| {
            matches!(m, Move::PlacePresident { destination } if *destination == church_area)
                || matches!(m, Move::MovePresident { destination, .. } if *destination == church_area)
        }).expect("President can reach Church turn 1");
        s.apply(pres, vec![]).unwrap();

        let church_mv = s.legal().into_iter().find(|m| matches!(m, Move::ExecuteAreaAction {
            action: AreaAction::Church {
                copper: true, silk: true, tea: false, seafood: false, yen: true, imported_good: false,
            }
        })).expect("paying copper+silk+yen is affordable turn 1 (starting resources include 1 of each)");
        s.apply(church_mv, vec![]).unwrap();

        assert_eq!(
            s.state().pending_choices, vec![PendingChoice::ChurchSpace { total_faith: 6 }],
            "setup recipe drifted -- re-derive against the current engine before trusting this fixture"
        );
        s
    }

    /// I2 final-review fix, CLI-side confirmation: `Session::load` promises
    /// typed, event-indexed errors (design comment on `ApplyError`/
    /// `ReplayError`) -- a tampered save file must never panic the whole
    /// process. `Session::apply`'s own legal-move gate would reject the
    /// tampering move outright (with `ChurchSpace` queued, `get_legal_moves`
    /// offers only its resolutions), so the tampered `GameRecord` is built
    /// directly instead -- exactly how a hand-edited or corrupted save file
    /// would bypass that gate: `Session::load` -> `replay` -> `replay_from`
    /// applies every event via bare `apply_move`, with no legal-move
    /// pre-filtering. The inserted `ReturnPresidentToHand` validates
    /// nothing and moves the President to hand; the following
    /// `ChooseChurchSpace { space: 6 }` still passes `resolve_choice_move`'s
    /// membership check (`eligible_church_spaces` depends only on the
    /// track/faith), so it reaches `place_church_assistant` with no acting
    /// area to resolve -- the exact panic `tampered_return_to_hand_mid_church_space_queue_errors_not_panics`
    /// (yokohama-core's `tracks.rs`) fixed at the engine layer.
    #[test]
    fn load_of_tampered_record_returns_typed_error_not_panic() {
        let s = session_with_church_space_choice_queued();
        let mut record = s.record().clone();
        record.push_move(Move::ReturnPresidentToHand { recover_assistants_from: vec![] });
        record.push_move(Move::ChooseChurchSpace { space: 6 });

        let path = std::env::temp_dir().join(format!("yokohama_tampered_{}.json", std::process::id()));
        std::fs::write(&path, serde_json::to_string_pretty(&record).unwrap()).unwrap();

        let result = Session::load(&path, Rules::standard());
        std::fs::remove_file(&path).ok();

        assert!(result.is_err(), "a tampered record must surface as a typed load error, never a panic");
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

    /// Exercises `apply`'s reveal-fold arms and `undo`'s contiguous-bound-
    /// reveal-popping loop for a group of size > 1 -- every test above this
    /// one passes `vec![]` reveals, so that path was previously untested.
    /// An identity order-deck arrangement (name the deck's current top 2
    /// cards, in draw order -- `order_deck`'s LAST element is the next
    /// `Vec::pop` draw, per `arrange_orders_on_top`'s doc comment in
    /// yokohama-core's `types/state.rs`) is a true no-op on deck contents,
    /// so this isolates "does Session fold/group/undo reveals correctly"
    /// from "does the reveal itself reorder the deck right" (already
    /// covered engine-side). A plain first move with no reveals precedes
    /// the group so a buggy undo that over-pops into the prior group is
    /// also caught by the event-count and state assertions.
    #[test]
    fn apply_with_reveals_undoes_the_whole_group() {
        let mut s = sim_2p(4);
        let mv1 = s.legal()[0].clone();
        s.apply(mv1, vec![]).unwrap();
        let s1 = s.state().clone();
        let events_after_first = s.record().events.len();

        let deck = s1.order_deck.clone();
        let n = deck.len();
        assert!(n >= 2, "need at least 2 deck cards to name an identity pair");
        let identity_reveal = vec![deck[n - 1], deck[n - 2]]; // next 2 draws, in draw order
        let mv2 = s.legal()[0].clone();
        let delta = s.apply(mv2, vec![RecordedEvent::RevealOrders(identity_reveal.clone())]).unwrap();
        assert!(!delta.is_empty());
        assert_ne!(*s.state(), s1);
        assert_eq!(s.record().events.len(), events_after_first + 2); // reveal + move

        s.undo().unwrap();
        assert_eq!(*s.state(), s1, "undo must remove the reveal AND the move together");
        assert_eq!(
            s.record().events.len(),
            events_after_first,
            "undo must not over- or under-pop the group"
        );

        // A second identical apply still works after the undo -- the
        // identity arrangement round-tripped the deck cleanly.
        let mv3 = s.legal()[0].clone();
        assert!(s.apply(mv3, vec![RecordedEvent::RevealOrders(identity_reveal)]).is_ok());
    }

    /// Error-path atomicity for `apply_standalone`: in a sim game (both
    /// players' starting orders are known via `Keep`, never hidden),
    /// `reveal_starting_order` itself rejects the call -- nothing should be
    /// recorded or mutated.
    #[test]
    fn apply_standalone_reveal_errs_atomically_when_not_hidden() {
        let mut s = sim_2p(4);
        let before_state = s.state().clone();
        let before_events = s.record().events.len();
        let card = s.state().players[0].starting_order.expect("Keep(0) always sets a starting order");
        let ev = RecordedEvent::RevealStartingOrder { player: PlayerId::new(0), card };
        assert!(s.apply_standalone(ev).is_err());
        assert_eq!(*s.state(), before_state);
        assert_eq!(s.record().events.len(), before_events);
    }

    /// The `other =>` reject arm: only `RevealStartingOrder` is a valid
    /// standalone event -- an ordinary `Move` and a bound reveal (only
    /// legal inside `apply`'s group) are both rejected, atomically.
    #[test]
    fn apply_standalone_rejects_non_standalone_variants() {
        let mut s = sim_2p(4);
        let before_state = s.state().clone();
        let before_events = s.record().events.len();
        assert!(s.apply_standalone(RecordedEvent::Move(Move::EndAdditionalPhase)).is_err());
        assert!(s.apply_standalone(RecordedEvent::RevealOrders(vec![])).is_err());
        assert_eq!(*s.state(), before_state);
        assert_eq!(s.record().events.len(), before_events);
    }

    /// Success path for `apply_standalone`, which also doubles as
    /// `new_live`'s first exercise: a live session where player 1's
    /// starting order was dealt but kept hidden (`Unknown`), reconciled
    /// with an all-`None` `SetupObservations` (a true no-op in
    /// `reconcile_setup` -- see `revealed_hidden_starting_order_replays` in
    /// yokohama-core's `tests/live_reconciliation.rs`, whose minimal setup
    /// this mirrors; the observation-reconciliation machinery itself is
    /// already covered exhaustively there). Revealing player 1's hidden
    /// card records the event and changes state; `undo` then pops it ALONE
    /// (a group of one, since it isn't a bound reveal), restoring the exact
    /// prior state.
    #[test]
    fn apply_standalone_reveals_hidden_starting_order_in_a_live_session() {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let seed = 21;
        let pending = new_game(config.clone(), &rules, seed).unwrap();
        let hidden_deal = pending.deals[1]; // player 1 will be dealt this pair
        let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown];
        let mut s = Session::new_live(config, seed, SetupObservations::default(), choices, rules).unwrap();

        let before_state = s.state().clone();
        let before_events = s.record().events.len();
        let ev = RecordedEvent::RevealStartingOrder { player: PlayerId::new(1), card: hidden_deal[0] };
        let delta = s.apply_standalone(ev.clone()).unwrap();
        assert!(!delta.is_empty());
        assert_ne!(*s.state(), before_state);
        assert_eq!(s.record().events.last(), Some(&ev));
        assert_eq!(s.state().players[1].starting_order, Some(hidden_deal[0]));

        s.undo().unwrap();
        assert_eq!(*s.state(), before_state);
        assert_eq!(s.record().events.len(), before_events);
    }

    /// Task 12: a raw `new_game`+`finalize_setup` state (not `sim_2p`, since
    /// the choice-move tests below need direct field access to inject a
    /// `PendingChoice` and its supporting player state before wrapping in a
    /// `for_test_from_state` session).
    fn raw_state_2p(seed: u64) -> GameState {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, seed).unwrap();
        finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)]).unwrap()
    }

    /// A board-to-board `ChooseAssistantMove` resolution is unambiguous in
    /// the state diff (only this reward ever moves an assistant area-to-
    /// area) -- state injection gives it a queued `PendingChoice::
    /// AssistantMove` head directly, mirroring the `assistant_move_fixture`
    /// pattern core's own `choices.rs` tests use, instead of driving a real
    /// reward-granting sequence.
    #[test]
    fn choose_assistant_move_resolution_produces_a_moved_delta_line() {
        let mut state = raw_state_2p(4);
        let mut non_canal = state
            .board_layout
            .areas
            .iter()
            .filter(|(_, cfg)| cfg.area_type != AreaType::Canal)
            .map(|(&id, _)| id);
        let from_area = non_canal.next().expect("at least 2 non-Canal areas on a 2p board");
        let to_area = non_canal.next().expect("at least 2 non-Canal areas on a 2p board");

        state.players[0].assistants.in_hand -= 1;
        *state.players[0].assistants.on_board.entry(from_area).or_insert(0) += 1;
        state.pending_choices.push(PendingChoice::AssistantMove);

        let mut s = Session::for_test_from_state(state, Rules::standard());
        let delta = s
            .apply(Move::ChooseAssistantMove { from: MoveSource::Area(from_area), to: to_area }, vec![])
            .unwrap();
        assert!(delta.iter().any(|l| l.contains("assistant moved")), "{delta:?}");
    }

    #[test]
    fn choose_warehouse_take_shophouse_produces_a_took_delta_line() {
        let mut state = raw_state_2p(4);
        state.players[0].money = 10; // affordable: assistant/shophouse/TH all offered
        state.pending_choices.push(PendingChoice::WarehouseTake);

        let mut s = Session::for_test_from_state(state, Rules::standard());
        let delta = s.apply(Move::ChooseWarehouseTake { item: WarehouseItem::Shophouse }, vec![]).unwrap();
        assert!(delta.iter().any(|l| l.contains("took shophouse (2 yen)")), "{delta:?}");
    }

    #[test]
    fn choose_church_space_produces_a_placed_delta_line() {
        let mut state = raw_state_2p(4);
        let some_area = *state.board_layout.areas.keys().next().expect("at least one area");
        // President on board (any area -- place_church_assistant doesn't
        // check area type, only that resolve_acting_area succeeds) so the
        // resolution has an acting area to place at.
        state.players[0].president_location = PresidentLocation::OnBoard(some_area);
        state.pending_choices.push(PendingChoice::ChurchSpace { total_faith: 6 });

        let mut s = Session::for_test_from_state(state, Rules::standard());
        let delta = s.apply(Move::ChooseChurchSpace { space: 6 }, vec![]).unwrap();
        let expected = format!("placed on Church faith-6 (+{} VP)", ChurchTrack::vp_reward(6));
        assert!(delta.iter().any(|l| l.contains(&expected)), "{delta:?}");
    }
}
