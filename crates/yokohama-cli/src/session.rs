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
    /// One cached plain-words description per `record.events` entry, in the
    /// same order -- ALWAYS in lockstep with `record.events` (every mutator
    /// asserts this in debug: `apply`/`apply_standalone` push alongside
    /// `push_event`/`push_move`, `undo` truncates to match, `load` rebuilds
    /// the whole vector by folding). Each entry is computed against the
    /// state AS OF WHEN THAT EVENT HAPPENED, not whatever state happens to
    /// be current when it's later read -- `event_description` (and
    /// `Repl::log_tail`, its one caller) is how `log` avoids re-describing
    /// an old event against live state and showing e.g. a trading-house
    /// take's LATEST printed price or a long-rotated movement fee's CURRENT
    /// recipient instead of the historical ones (T2.4; the two staleness
    /// cases Task 13's review flagged). Not part of `GameRecord`, so never
    /// serialized -- `load` rebuilds it from scratch by folding.
    event_descriptions: Vec<String>,
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
        Ok(Session {
            rules,
            record,
            state,
            mode: TrackingMode::Simulation,
            save_path: None,
            checkpoint: None,
            event_descriptions: Vec::new(),
        })
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
        Ok(Session {
            rules,
            record,
            state,
            mode: TrackingMode::Live,
            save_path: None,
            checkpoint: None,
            event_descriptions: Vec::new(),
        })
    }

    /// Load a saved `GameRecord` from `path` and replay it to reconstruct
    /// the `Session`. Mode is inferred: `Live` iff
    /// `record.setup_observations.is_some()`. Runs `check_invariants` on
    /// the replayed state before returning, so a corrupt save file surfaces
    /// as a load error rather than propagating silently into play.
    pub fn load(path: &std::path::Path, rules: Rules) -> Result<Session, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let record: GameRecord = serde_json::from_str(&text).map_err(|e| e.to_string())?;

        // Mirrors `replay`'s own lenient mismatch diagnostics (core's
        // record.rs) verbatim: v1 warns but does not refuse to replay a
        // record produced by a different engine build or against different
        // rules data. `load` can no longer just call `replay` for this --
        // see below -- so these two checks are duplicated here rather than
        // silently dropped (Task 15 review Finding 2: `check_invariants`
        // below checks only structural conservation, NOT engine_version/
        // rules_hash, so it does NOT backstop a rules-data drift that
        // happens to leave conservation intact -- an earlier version of
        // this comment claimed otherwise; that was wrong). Correct only
        // because yokohama-cli and yokohama-core share one workspace
        // version (`version.workspace = true` in both Cargo.tomls) --
        // `env!("CARGO_PKG_VERSION")` is resolved per-crate at compile
        // time, so this comparison would silently stop meaning anything if
        // the two crates' versions were ever allowed to diverge.
        let running_version = env!("CARGO_PKG_VERSION");
        if record.engine_version != running_version {
            eprintln!(
                "warning: GameRecord.engine_version=\"{}\" does not match running engine version \"{running_version}\" -- replaying anyway (v1 does not refuse on a version mismatch)",
                record.engine_version,
            );
        }
        let running_rules_hash = rules.content_hash();
        if record.rules_hash != running_rules_hash {
            eprintln!(
                "warning: GameRecord.rules_hash=\"{}\" does not match running rules content hash \"{running_rules_hash}\" -- replaying anyway (v1 does not refuse on a rules_hash mismatch)",
                record.rules_hash,
            );
        }

        // Rebuilds the base state the same way `new_sim`/`new_live` do
        // (new_game -> optional reconcile_setup -> finalize_setup) rather
        // than calling `replay` for the whole record in one shot: each
        // event needs to be described against the state AS OF THAT EVENT
        // before it folds, so the loop below interleaves
        // `describe_recorded_event` with `replay_from` one event at a time
        // -- the same per-event pattern `examples/record_stats.rs` uses to
        // interleave its own `metrics.observe` call.
        let mut pending = new_game(record.config.clone(), &rules, record.seed).map_err(|e| e.to_string())?;
        if let Some(obs) = &record.setup_observations {
            pending = reconcile_setup(pending, obs, &rules).map_err(|e| e.to_string())?;
        }
        let mut state = finalize_setup(pending, &record.choices).map_err(|e| e.to_string())?;

        let mut event_descriptions = Vec::with_capacity(record.events.len());
        for (i, ev) in record.events.iter().enumerate() {
            event_descriptions.push(crate::describe::describe_recorded_event(ev, &state, &rules));
            // `replay_from` is fed a FRESH 1-element slice per iteration
            // (so `state` can advance one event at a time between
            // description calls), so its own `ReplayError::event_index` --
            // derived from `.enumerate()` over THAT slice -- is always 0,
            // no matter which real event failed (Task 15 review Finding 1).
            // Reconstruct the message with the TRUE index `i` into
            // `record.events` instead of trusting the error's own
            // (structurally always-0) index.
            state = replay_from(&state, std::slice::from_ref(ev), &rules)
                .map_err(|e| format!("replay failed at event {i}: {}", e.message))?;
        }
        debug_assert_eq!(event_descriptions.len(), record.events.len());

        check_invariants(&state, &rules)?;
        let mode = if record.setup_observations.is_some() { TrackingMode::Live } else { TrackingMode::Simulation };
        Ok(Session {
            rules,
            record,
            state,
            mode,
            save_path: Some(path.to_path_buf()),
            checkpoint: None,
            event_descriptions,
        })
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

    /// The cached plain-words description of `record().events[i]`, computed
    /// against the state as of when that event happened (see the
    /// `event_descriptions` field doc comment) -- `Repl::log_tail` is the
    /// one caller today.
    pub fn event_description(&self, i: usize) -> &str {
        &self.event_descriptions[i]
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
        let mut reveal_descriptions = Vec::with_capacity(reveals.len());
        for ev in &reveals {
            // Described against `staged` as it stands right before THIS
            // event folds -- the historical state at the moment it
            // happened. Computed unconditionally before the apply arm below
            // (including for a variant the `other` arm is about to reject);
            // that's a harmless wasted allocation on the error path, traded
            // for one line per event instead of two parallel matches.
            let desc = crate::describe::describe_recorded_event(ev, &staged, &self.rules);
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
            reveal_descriptions.push(desc);
        }
        // Since the checked-boundary task, the engine's PUBLIC apply_move
        // itself gates on get_legal_moves membership -- the phase check it
        // used to lean on instead, MoveError::WrongPhase, was deleted as
        // dead code at Task 13 (design §5.3-7, guidingpath's decision log).
        // This call site deliberately uses apply_move_unchecked below
        // rather than that public apply_move: Session runs its own
        // identical legal-list gate right here, against `staged`
        // (post-reveal, since the group is atomic and legality is defined
        // on the state the move will actually run against), so it can skip
        // a second, redundant get_legal_moves enumeration (see the comment
        // on the apply_move_unchecked call itself, below).
        if !get_legal_moves(&staged, &self.rules).contains(&mv) {
            return Err(ApplyError::Illegal(format!("move not legal in the current state: {mv:?}")));
        }
        // Described against `staged` (post-reveal, pre-move) -- the
        // HISTORICAL state the move actually ran against -- and cached
        // below, so `log` renders this event's text as it was true when it
        // happened rather than re-evaluating it against whatever state is
        // current when `log` is later invoked (T2.4: a trading-house take's
        // printed price, a movement fee's recipient, and any other
        // state-dependent wording would otherwise go stale the moment state
        // moves on).
        let move_description = crate::describe::describe_move(&mv, &staged, &self.rules);
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
        self.event_descriptions.extend(reveal_descriptions);
        self.event_descriptions.push(move_description);
        debug_assert_eq!(self.event_descriptions.len(), self.record.events.len());
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
        // Described against the PRE-commit `self.state` -- the historical
        // state at the moment this reveal happened -- same reasoning as
        // `apply`'s per-event descriptions (T2.4). This variant's wording
        // doesn't actually depend on state, but computing it uniformly here
        // keeps `event_descriptions` populated by every path that can push
        // into `record.events`, which is what the lockstep invariant needs.
        let description = crate::describe::describe_recorded_event(&ev, &self.state, &self.rules);
        let mut staged = self.state.clone();
        staged.reveal_starting_order(player, card).map_err(|e| ApplyError::Reveal(e.to_string()))?;
        check_invariants(&staged, &self.rules)
            .map_err(|e| ApplyError::Invariant(format!("INVARIANT VIOLATION after apply (driver/engine bug): {e}")))?;
        let mut delta = crate::render::delta_summary(&self.state, &staged, &self.rules);
        self.record.push_event(ev);
        self.event_descriptions.push(description);
        debug_assert_eq!(self.event_descriptions.len(), self.record.events.len());
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
        // The cache only ever grows/shrinks in lockstep with `record.events`
        // by appending/truncating at the tail -- `event_descriptions[i]` was
        // computed for `record.events[i]` when it was first appended and
        // never touched again, so truncating to the new (shorter) length is
        // exactly right: it drops the popped entries' descriptions and
        // leaves every surviving one untouched.
        self.event_descriptions.truncate(record.events.len());
        debug_assert_eq!(self.event_descriptions.len(), record.events.len());
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

    /// The full official result -- per-player `ScoreBreakdown` plus rank,
    /// winner decided by the KB's clockwise tie-break (`final_result`,
    /// scoring.rs). What the game-over screen (`Repl::score_text`) renders;
    /// `final_scores` above stays as the flat `(player, total)` view for
    /// callers that don't need the breakdown.
    pub fn final_result(&self) -> GameResult {
        final_result(&self.state, &self.rules)
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
            event_descriptions: Vec::new(),
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
    /// would bypass that gate: `Session::load` (since the T2.4/Task-15
    /// fold-cache rewrite, its OWN inline `new_game`/`reconcile_setup`/
    /// `finalize_setup` preamble plus a per-event loop over `replay_from`,
    /// not a single `replay` call the way it used to be) applies every
    /// event via the ONE checked `apply_move` (core's apply_move.rs), WITH
    /// legal-move pre-filtering. That rejects the
    /// inserted `ReturnPresidentToHand` immediately with
    /// `MoveError::NotLegal` -- it isn't one of `ChurchSpace`'s own
    /// resolutions, so it's absent from `get_legal_moves` while that choice
    /// is queued -- and the following `ChooseChurchSpace { space: 6 }` is
    /// never reached. (The engine-layer counterpart,
    /// `tracks.rs`'s `tampered_return_to_hand_mid_church_space_queue_errors_not_panics`,
    /// exercises the SAME tamper via raw `apply_move_unchecked` instead,
    /// where the second move IS reached and would otherwise panic -- fixed
    /// there at the engine layer; here it's moot, since checked `apply_move`
    /// never gets that far.)
    #[test]
    fn load_of_tampered_record_returns_typed_error_not_panic() {
        let s = session_with_church_space_choice_queued();
        let mut record = s.record().clone();
        record.push_move(Move::ReturnPresidentToHand { recover: std::collections::BTreeMap::new() });
        record.push_move(Move::ChooseChurchSpace { space: 6 });

        let path = std::env::temp_dir().join(format!("yokohama_tampered_{}.json", std::process::id()));
        std::fs::write(&path, serde_json::to_string_pretty(&record).unwrap()).unwrap();

        let result = Session::load(&path, Rules::standard());
        std::fs::remove_file(&path).ok();

        assert!(result.is_err(), "a tampered record must surface as a typed load error, never a panic");
    }

    /// C1 regression (external six-agent review, 2026-07-20). The guard test
    /// directly above has NO teeth against this: its `ReturnPresidentToHand`
    /// tamper happens to *describe* safely, so it passes even when `load`
    /// panics on a differently-shaped tamper.
    ///
    /// T2.4's per-event fold calls `describe_recorded_event` BEFORE
    /// `replay_from` validates the event, so a tampered out-of-range
    /// `BuildShophouse` slot reached `slot_reward_text`'s raw
    /// `site.spaces[slot]` index and aborted the process -- against design
    /// §7 ("tampered move -> typed rejection, not warn, not panic"), and a
    /// regression from the pre-T2.4 `load`, which replayed in one shot and
    /// never described an unvalidated event.
    #[test]
    fn load_of_tampered_record_with_out_of_range_slot_is_typed_error_not_panic() {
        let s = session_with_church_space_choice_queued();

        // Fixture precondition: the tampered event must actually reach
        // `slot_reward_text` when described, else this test proves nothing.
        // That needs the President standing on an area that HAS a building
        // site (`describe_move`'s BuildShophouse arm resolves the site from
        // `acting_area`).
        let area = match s.state().player(s.state().current_player).president_location {
            PresidentLocation::OnBoard(a) => a,
            PresidentLocation::InHand => panic!("fixture must leave the President on board"),
        };
        assert!(
            s.state().board_layout.building_sites.contains_key(&area),
            "fixture area {area:?} must have a building site, else the tamper describes safely \
             and this test degenerates into the toothless one above"
        );

        let mut record = s.record().clone();
        record.push_move(Move::BuildShophouse { slot: 250 });

        let path = std::env::temp_dir().join(format!("yokohama_tampered_slot_{}.json", std::process::id()));
        std::fs::write(&path, serde_json::to_string_pretty(&record).unwrap()).unwrap();

        let result = Session::load(&path, Rules::standard());
        std::fs::remove_file(&path).ok();

        assert!(
            result.is_err(),
            "an out-of-range slot in a tampered record must surface as a typed load error, never a panic"
        );
    }

    /// Fix-wave regression (Task 15 review, Finding 1): `load`'s per-event
    /// fold loop must name the TRUE failing event's index, not always 0.
    /// The old (pre-T2.4) `load` called `replay` once over the whole events
    /// slice, so a `ReplayError` correctly named which event broke; the
    /// per-event `replay_from(&state, std::slice::from_ref(ev), &rules)`
    /// rewrite feeds each call a FRESH 1-element slice, so
    /// `ReplayError::event_index` -- derived from `.enumerate()` over THAT
    /// slice -- was structurally always 0 regardless of which real event
    /// failed.
    ///
    /// Tampers a record with `ChurchSpace` queued
    /// (`session_with_church_space_choice_queued`) by appending ONE
    /// `ReturnPresidentToHand` -- not one of the queued choice's own
    /// resolutions, so `apply_move`'s blanket `get_legal_moves` membership
    /// check (`MoveError::NotLegal`, core's apply_move.rs) rejects it
    /// immediately at THIS event, the first (and only) tampered one -- the
    /// same mechanism the sibling test above now documents. The expected
    /// index is computed from the fixture (`base`) rather than hardcoded,
    /// so it can't silently drift if the fixture's own move count ever
    /// changes.
    #[test]
    fn load_of_tampered_record_names_the_true_failing_event_index() {
        let s = session_with_church_space_choice_queued();
        let base = s.record().events.len();
        let mut record = s.record().clone();
        record.push_move(Move::ReturnPresidentToHand { recover: std::collections::BTreeMap::new() });
        let expected_failing_index = base; // the tampered ReturnPresidentToHand itself

        let path = std::env::temp_dir().join(format!("yokohama_tampered_index_{}.json", std::process::id()));
        std::fs::write(&path, serde_json::to_string_pretty(&record).unwrap()).unwrap();

        // `Session::load`'s Ok variant is `Session`, which (deliberately)
        // doesn't derive `Debug` -- `unwrap_err()` needs `T: Debug` for its
        // OWN panic message on the never-taken Ok arm, so a plain `match`
        // sidesteps that instead of adding a derive nothing else needs.
        let err = match Session::load(&path, Rules::standard()) {
            Err(e) => e,
            Ok(_) => panic!("expected a load error naming the tampered event, got Ok"),
        };
        std::fs::remove_file(&path).ok();

        assert!(
            err.contains(&format!("event {expected_failing_index}")),
            "error must name the TRUE failing event index ({expected_failing_index}), not always 0: {err}"
        );
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

    /// T2.4 (Task 15): `event_descriptions` must stay in exact lockstep with
    /// `record().events` across every operation that can grow or shrink the
    /// log -- `apply` (repeated), `undo`, and `load` (a fresh Session built
    /// from a saved file).
    #[test]
    fn descriptions_stay_in_lockstep_across_apply_undo_load() {
        let mut s = sim_2p(4);
        for _ in 0..5 {
            let mv = s.legal()[0].clone();
            s.apply(mv, vec![]).unwrap();
            assert_eq!(s.event_descriptions.len(), s.record().events.len());
        }

        s.undo().unwrap();
        assert_eq!(s.event_descriptions.len(), s.record().events.len());

        let path = std::env::temp_dir().join(format!("yokohama_test_lockstep_{}.json", std::process::id()));
        s.save(&path).unwrap();
        let loaded = Session::load(&path, Rules::standard()).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(loaded.event_descriptions.len(), loaded.record().events.len());
    }

    /// T2.4 (Task 15) Step-1 fixture: a trading-house warehouse take is
    /// described with the printed cost AT THE TIME IT WAS TAKEN (4 yen,
    /// `TradingHouseState::take_cost(0)`) -- `describe_move`'s cost text
    /// comes from `next_cost()`, which advances with the take progression,
    /// so re-describing the SAME historical event against a LATER state
    /// (progression advanced past 4) must not change the cached text.
    #[test]
    fn log_descriptions_are_historical_not_current() {
        let mut state = raw_state_2p(4);
        let taker = PlayerId::new(0);
        state.players[0].money = 10; // affordable at the printed 4-yen first take
        state.pending_choices.push(PendingChoice::WarehouseTake);

        let mut s = Session::for_test_from_state(state, Rules::standard());
        s.apply(Move::ChooseWarehouseTake { item: WarehouseItem::TradingHouse }, vec![]).unwrap();
        let idx = s.record().events.len() - 1;
        assert!(s.event_description(idx).contains('4'), "{}", s.event_description(idx));

        // Advance the printed take progression past the first take's price
        // -- simulates play continuing after the historical event (direct
        // field mutation: this test module is nested inside session.rs, so
        // it shares Session's private-field visibility).
        s.state.players[taker.0 as usize].trading_houses.in_hand += 1;
        assert_ne!(
            s.state().player(taker).trading_houses.next_cost(),
            4,
            "fixture must have advanced the progression, else this test proves nothing"
        );

        assert!(
            s.event_description(idx).contains('4'),
            "cached description must still show the OLD printed cost: {}",
            s.event_description(idx)
        );
    }

    /// T2.4 (Task 15) context assertion: after `load`, the fold-rebuilt
    /// `event_descriptions` must equal the pre-save vector element-by-
    /// element (not just lockstep length) -- proves the load fold
    /// reproduces history exactly, movement events (state-dependent text)
    /// included, not just reveal events (state-independent text).
    #[test]
    fn load_of_a_saved_session_reproduces_event_descriptions_exactly() {
        let mut s = sim_2p(4);
        let mut saw_move_president = false;
        for _ in 0..25 {
            if s.is_over() {
                break;
            }
            let legal = s.legal();
            // Picking legal()[0] every turn never diversifies WHERE
            // assistants land (it happens to always re-pick the same
            // single area), so the President never has anywhere else to
            // move to and MovePresident never appears -- verified via an
            // offline probe. Instead: take a MovePresident the moment one
            // is offered; otherwise prefer the Option-A placement touching
            // the MOST distinct areas (spreads assistants so a real
            // destination eventually exists), then getting the President
            // onto the board at all, else fall back to legal()[0].
            let mv = legal
                .iter()
                .find(|m| matches!(m, Move::MovePresident { .. }))
                .or_else(|| {
                    legal
                        .iter()
                        .filter(|m| matches!(m, Move::PlaceThreeAreas { .. }))
                        .max_by_key(|m| match m {
                            Move::PlaceThreeAreas { placements } => placements.len(),
                            _ => 0,
                        })
                })
                .or_else(|| legal.iter().find(|m| matches!(m, Move::PlacePresident { .. })))
                .cloned()
                .unwrap_or_else(|| legal[0].clone());
            if matches!(mv, Move::MovePresident { .. }) {
                saw_move_president = true;
            }
            s.apply(mv, vec![]).unwrap();
        }
        assert!(
            saw_move_president,
            "fixture must exercise at least one MovePresident event -- re-derive the step count/seed if the engine's move ordering drifted"
        );

        let path = std::env::temp_dir().join(format!("yokohama_test_descs_{}.json", std::process::id()));
        s.save(&path).unwrap();
        let loaded = Session::load(&path, Rules::standard()).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(loaded.event_descriptions.len(), s.event_descriptions.len());
        assert_eq!(
            loaded.event_descriptions, s.event_descriptions,
            "load's fold-rebuilt descriptions must equal the pre-save vector element-by-element"
        );
    }

    /// Minimal 3-area chain (`start` - `blocked` - `final_dest`) for the
    /// movement fee-recipient staleness test below: Red's President starts
    /// at `start` and moves to `final_dest`, PASSING THROUGH `blocked` --
    /// where Blue's President sits -- along the way. Landing directly on an
    /// opponent's area is never legal (`is_possible_action_area_full`
    /// rejects it, Postal System aside), so `final_dest` is deliberately a
    /// separate, unblocked area: this is what makes the pass-through fee
    /// (credited to Blue, 1 yen) the only reason "Blue" appears in the
    /// description at all. Mirrors `describe.rs`'s own `movement_cost_fixture`
    /// (private to that file's test module, so rebuilt minimally here
    /// rather than exported crate-wide for one caller) with the
    /// canal/free-route cases dropped -- this test only needs the
    /// pass-through fee case.
    fn movement_fee_fixture() -> (GameState, Rules) {
        let start = AreaId::new(0);
        let blocked = AreaId::new(1);
        let final_dest = AreaId::new(2);

        let mut areas = std::collections::BTreeMap::new();
        areas.insert(start, AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(blocked, AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });
        areas.insert(final_dest, AreaConfig { area_type: AreaType::SilkMill, grid_position: (0, 2) });

        let mut adjacency = std::collections::BTreeMap::new();
        adjacency.insert(start, vec![blocked]);
        adjacency.insert(blocked, vec![start, final_dest]);
        adjacency.insert(final_dest, vec![blocked]);

        let board_layout = BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![],
            building_sites: std::collections::BTreeMap::new(),
            five_power_tokens: std::collections::BTreeMap::new(),
        };

        let config = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));
        let mut players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];
        players[0].president_location = PresidentLocation::OnBoard(start);
        players[0].money = 10;
        // Ordinary (non-Tram/Station) movement only steps onto an area
        // holding the MOVER's own assistant (`can_step_onto`, movement.rs)
        // -- an opponent-occupied area is still traversable that way (fee
        // charged), just never itself a legal LANDING spot
        // (`is_possible_action_area_full` rejects a destination with
        // another player's President on it, Postal System aside). So Red
        // needs an assistant at BOTH the pass-through area `blocked` (to
        // traverse it) and the actual landing area `final_dest` (to land
        // there) -- Blue's President sits only at `blocked`, charging the
        // pass-through fee without blocking the eventual landing.
        players[0].assistants.on_board.insert(blocked, 1);
        players[0].assistants.on_board.insert(final_dest, 1);
        players[0].assistants.in_hand -= 2;
        players[1].president_location = PresidentLocation::OnBoard(blocked); // Blue charges the pass-through fee
        players[1].money = 10;

        let state = GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::Movement,
            turn_number: 1,
            additional_action_state: AdditionalActionState::new(),
            current_action_power: None,
            is_mercenary_action: false,
            mercenary_target_area: None,
            mercenary_return_phase: None,
            president_start_area: None,
            port_state: PortState::new(false),
            lab_state: LabState::new(false),
            order_deck: vec![],
            tech_deck: vec![],
            orders_removed: vec![],
            church_track: ChurchTrack::new(),
            customs_track: CustomsTrack::new(),
            achievements: [
                AchievementCard::new(AchievementType::Copper5),
                AchievementCard::new(AchievementType::Money10),
                AchievementCard::new(AchievementType::Production4),
            ],
            mercenary_supply: MercenarySupply::new(),
            station_location: None,
            end_game_triggered: false,
            trigger_reason: None,
            final_round: false,
            trigger_player: None,
            pending_choices: Vec::new(),
        };

        (state, Rules::empty_for_tests())
    }

    /// T2.4 (Task 15) context assertion (closes review obligation #1, T13's
    /// flagged MovePresident staleness case): a MovePresident event's
    /// fee/recipient text depends on WHO occupies the fee-charging area at
    /// the moment it applies -- once that player's President moves away,
    /// re-describing the SAME historical move against the state as it
    /// stands NOW would show a different (or missing) recipient. The cache
    /// must keep the ORIGINAL recipient regardless of what happens to the
    /// board afterward.
    #[test]
    fn move_president_description_keeps_historical_fee_recipient_after_state_changes() {
        let (state, rules) = movement_fee_fixture();
        let start = AreaId::new(0);
        let final_dest = AreaId::new(2);
        let mut s = Session::for_test_from_state(state, rules);

        let mv = s
            .legal()
            .into_iter()
            .find(|m| matches!(m, Move::MovePresident { destination, .. } if *destination == final_dest))
            .expect("reaching final_dest via the blocked pass-through must be a legal Movement-phase option in this fixture");
        s.apply(mv.clone(), vec![]).unwrap();
        let idx = s.record().events.len() - 1;
        let cached = s.event_description(idx).to_string();
        assert!(cached.contains("Blue"), "fixture must actually charge the pass-through fee to Blue: {cached}");

        // Blue's President relocates off the historical path entirely --
        // simulates state moving on past this historical event (direct
        // field mutation: this test module is nested inside session.rs, so
        // it shares Session's private-field visibility, same access
        // `for_test_from_state` itself relies on).
        s.state.players[1].president_location = PresidentLocation::OnBoard(start);

        // Re-describing the SAME historical move against the state as it
        // stands now would no longer credit Blue -- confirms the contrast
        // this test pins is real, not vacuous.
        let restated = crate::describe::describe_move(&mv, s.state(), s.rules());
        assert!(!restated.contains("Blue"), "fixture's contrast must actually differ once state changes: {restated}");

        assert!(
            s.event_description(idx).contains("Blue"),
            "cached description must keep the HISTORICAL fee recipient: {}",
            s.event_description(idx)
        );
    }
}
