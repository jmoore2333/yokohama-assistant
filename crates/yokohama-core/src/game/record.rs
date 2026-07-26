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
use crate::game::semantics::{state_digest, SEMANTICS_REV};
use crate::game::setup::{finalize_setup, new_game, reconcile_setup, SetupObservations, StartingOrderChoice};
use crate::types::{GameConfig, GameState, Move, MoveError, OrderCardId, PlayerId, TechCardId};

/// One entry in a `GameRecord`'s event log. `Move` is ordinary play; the
/// `Reveal*` variants are live-tracking reconciliation events (design
/// 2026-07-18 §5.2) — they mutate hidden-information fiction (deck order,
/// hidden starting orders) and MUST be in the record so `replay` reproduces
/// the reconciled state. (5-Power tokens are FACE-UP open information observed
/// at setup, PR #5 §1.1 — they are never revealed, so there is no token variant.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordedEvent {
    Move(Move),
    /// The next draws from the order deck are exactly these, in draw order.
    RevealOrders(Vec<OrderCardId>),
    /// The next draws from the tech deck are exactly these, in draw order.
    RevealTechs(Vec<TechCardId>),
    /// A hidden starting order became public.
    RevealStartingOrder { player: PlayerId, card: OrderCardId },
}

/// Current [`RunProvenance`] shape. Bumped ONLY when a change breaks readers —
/// a field added with `#[serde(default)]` does not, since an older document
/// still parses and a newer one still means what it said.
pub const RUN_PROVENANCE_SCHEMA: u32 = 1;

/// [`SeatProvenance::observation`] for a seat that played from a `PlayerView`
/// — the honest boundary. Every seat a human or an arena agent played is this.
pub const OBSERVATION_VIEW: &str = "view";

/// [`SeatProvenance::observation`] for a seat whose controller read the true
/// `GameState`. Privileged play by construction (`arena::run_raw_game`, the
/// fuzzer's shape): correct for screening cells, and NOT training data. Spec
/// §4 requires this label to ride the ARTIFACT rather than the prose, which is
/// what `arena::corpus`'s emitter enforces.
pub const OBSERVATION_FULL_STATE_DIAGNOSTIC: &str = "full-state-diagnostic";

/// Who played this game — the run-level provenance ONE field serves for both
/// the arena and the CLI (spec §5).
///
/// **Replay-inert by design.** `replay`/`replay_strict` read only
/// seed/config/choices/events/observations, and `state_digest` takes a
/// `GameState`, so nothing here can move a replayed state or a digest
/// (`provenance_is_replay_inert` pins it). It sits on the record for the same
/// reason `lockdown` does: it is what a reader of the FILE needs and what the
/// state deliberately does not carry.
///
/// **Why a name and a params map, and not a typed policy enum** (spec §5, M2's
/// trade stated): serde REFUSES an unknown enum variant, so the day a fourth
/// rung ships, every older binary reading the new corpus would hard-fail at
/// parse — the opposite of ledger row 7's replay-compat intent. A string name
/// plus an integer params map degrades instead: an old reader still gets the
/// seed, the events and the seat count, and can say "controller I do not know"
/// rather than refusing the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunProvenance {
    /// [`RUN_PROVENANCE_SCHEMA`] at write time.
    #[serde(default)]
    pub schema: u32,
    /// The seed schedule's master, when the run had one above it. `None` for a
    /// single game played on its own world seed — an honest absence, not a
    /// zero (`arena::run_cell` threads the cell's master; a bare
    /// `arena::run_arena_game` caller may have no schedule at all).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master_seed: Option<u64>,
    /// The run this game belonged to — the arena CELL name, which is also the
    /// corpus directory segment. `None` for an ungrouped game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// One entry per seat, in `PlayerId` order.
    pub seats: Vec<SeatProvenance>,
}

/// Who played ONE seat.
///
/// Per SEAT rather than per run because a mixed-controller game is the
/// sub-phase's headline capability: X against Y, human against bot. A run-level
/// field could not express either.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatProvenance {
    /// The controller's stable NAME: `"human"`, or a rollout policy's
    /// `RolloutPolicy::as_str()`. Never hand-typed at a call site — the policy
    /// is the one authority for its own name, and its serde tag is pinned equal
    /// to it (`policy.rs`'s `serde_names_match_as_str_for_every_rung`).
    pub controller: String,
    /// The controller IMPLEMENTATION version under that stable name
    /// (`game::policy::POLICY_IMPL_VERSION`), bumped when a policy's sampling
    /// SEMANTICS change — i.e. when figures recorded under the same name stop
    /// being comparable. Distinct from [`RunProvenance::schema`], which is about
    /// this struct's shape and not about behaviour.
    pub version: u32,
    /// The controller's knobs in their declared integer units — fixed-point
    /// PARTS PER MILLION for a probability, a plain count for a count.
    ///
    /// Integers, not floats, because `GameRecord` derives `Eq`: a float knob
    /// would make record comparison and round-trip identity depend on decimal
    /// formatting (critic #1). A knobless controller reports an empty map.
    #[serde(default)]
    pub params: std::collections::BTreeMap<String, i64>,
    /// The seat's own RNG stream seed, when it had one — enough, with the
    /// record's seed and this map, to re-derive the seat's play.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_seed: Option<u64>,
    /// What this seat could SEE: [`OBSERVATION_VIEW`] or
    /// [`OBSERVATION_FULL_STATE_DIAGNOSTIC`]. Single-sourced from the DRIVER
    /// that built the seat, never from the controller — a controller labelling
    /// its own information access is exactly the claim that needs an
    /// independent witness.
    pub observation: String,
}

/// A complete, replayable record of one game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRecord {
    /// `env!("CARGO_PKG_VERSION")` of the engine build that produced this
    /// record. `replay` compares this to the running engine's version and
    /// warns on mismatch, but does not refuse to replay (see `replay`'s doc
    /// comment). `replay_strict` (same module) IS that tightening: it
    /// rejects a mismatch outright at event_index 0, and is the path a
    /// training pipeline should use.
    pub engine_version: String,
    /// `Rules::content_hash()` of the `Rules` the record was created against.
    /// `replay_strict` rejects a mismatch at event_index 0; lenient `replay`
    /// warns to stderr and continues (design §4.6) -- data-shape changes to
    /// the card/site/token registries or a board-derivation-version bump
    /// change this hash, catching a training pipeline replaying a record
    /// against rules data it wasn't produced against.
    pub rules_hash: String,
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
    /// Executable-semantics revision (`SEMANTICS_REV`) of the engine that
    /// produced this record. Unlike `engine_version` (the never-bumped
    /// workspace crate version) and `rules_hash` (static card/site/token/board
    /// DATA only), this identifies the behaviour of `get_legal_moves` /
    /// `apply_move` / scoring / setup -- see `crate::game::semantics` for the
    /// bump contract. `None` means the record predates the field (pre-Phase-2.9)
    /// and its semantics provenance is UNKNOWN -- fail-closed (Codex PR-3
    /// review, FIX-B P1: the previous serde-default of literal 1 made a
    /// provenance-less record indistinguishable from a genuine current rev-1
    /// record, so `replay_strict` silently accepted it). `GameRecord::new`
    /// stamps `Some(SEMANTICS_REV)`, so every new record serializes the
    /// number. `replay` warns on `None` (missing provenance) and on a `Some`
    /// mismatch, replaying anyway; `replay_strict` rejects BOTH.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantics_rev: Option<u32>,
    /// FNV-1a digest (`state_digest`) of the FINAL `GameState` this record's
    /// event log folds to, stamped by `stamp_terminal_digest` at
    /// finalization/save. `None` on a record that was never stamped (or was
    /// written before the field existed). `replay_strict` recomputes the
    /// digest from the replayed terminal state and rejects a mismatch --
    /// defense in depth on top of its per-event invariant checks (it catches
    /// an events-vs-digest divergence even when every intermediate state is
    /// individually valid).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_digest: Option<String>,
    /// CLI live-tracking session metadata (NOT part of the replayable game
    /// history — it never affects `replay`/`replay_strict`, which read only
    /// seed/config/choices/events/observations). `Some(reason)` once the
    /// interactive driver locked the session after a desync or invariant fault
    /// (`yokohama-cli`'s `Repl`); the operator's driver refuses all further play
    /// and only allows save/log/quit. Persisted here so a lockdown SURVIVES
    /// save → quit → reload instead of silently clearing (review §3): the driver
    /// restores its `locked` flag from this on load. `None` = not locked (and
    /// the serde default for records written before the field existed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lockdown: Option<String>,
    /// Who played this game (Phase 3.2, spec §5) — see [`RunProvenance`].
    /// `None` on a record whose producer said nothing about its controllers,
    /// including every record written before the field existed. Replay-inert:
    /// modelled on `lockdown`, the in-tree precedent for metadata that rides
    /// the record without entering the replayed game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<RunProvenance>,
}

impl GameRecord {
    /// Start a record for a game set up via `new_game(config, rules, seed)`
    /// then `finalize_setup(pending, &choices)`. `events` starts empty --
    /// call `push_move` after each `apply_move` (or `push_event` for a
    /// live-tracking reveal reconciliation). `rules` stamps `rules_hash`
    /// (`Rules::content_hash()`) so replay can later detect the record was
    /// produced against different rules data.
    pub fn new(seed: u64, config: GameConfig, choices: Vec<StartingOrderChoice>, rules: &Rules) -> Self {
        Self {
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            rules_hash: rules.content_hash(),
            semantics_rev: Some(SEMANTICS_REV),
            seed,
            config,
            choices,
            events: Vec::new(),
            setup_observations: None,
            terminal_digest: None,
            lockdown: None,
            run: None,
        }
    }

    /// Stamp WHO played, per seat (spec §5). Callers: the arena driver, which
    /// knows each seat's controller, and the CLI, which knows which seats are
    /// human.
    ///
    /// `master_seed`/`run_id` start empty — a driver playing one game has no
    /// schedule or cell above it to name. The harness that does fills them in
    /// with [`set_run_context`](Self::set_run_context).
    pub fn set_run_seats(&mut self, seats: Vec<SeatProvenance>) {
        self.run = Some(RunProvenance {
            schema: RUN_PROVENANCE_SCHEMA,
            master_seed: None,
            run_id: None,
            seats,
        });
    }

    /// Name the RUN this game belonged to: its seed schedule's master and the
    /// cell that ran it. A no-op on a record with no seat provenance — there is
    /// no run to contextualize, and inventing one would claim a controller
    /// nobody recorded.
    pub fn set_run_context(&mut self, master_seed: u64, run_id: &str) {
        if let Some(run) = &mut self.run {
            run.master_seed = Some(master_seed);
            run.run_id = Some(run_id.to_string());
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

    /// Stamp `terminal_digest` from `final_state` -- the `GameState` this
    /// record's event log folds to. Call at finalization/save (the record is
    /// built incrementally, so there is no single "complete" moment; callers
    /// hold the resulting state already -- e.g. the CLI `Session` its live
    /// `state`, which `replay` reproduces exactly). Lets `replay_strict` later
    /// verify the replayed terminal state matches. Idempotent; re-stamping
    /// after appending more events simply refreshes it, so it must be called
    /// AFTER the last event that belongs to the saved run.
    pub fn stamp_terminal_digest(&mut self, final_state: &GameState) {
        self.terminal_digest = Some(state_digest(final_state));
    }
}

/// Why `replay`/`replay_from` could not reconstruct a `GameState` (Task 13,
/// design §5.3-7): `event_index` is the 0-based index into the events slice
/// being folded (`record.events` for `replay`, the caller's own slice for
/// `replay_from`) of the event whose application failed, so a caller can
/// point a human at exactly which logged event is bad instead of just
/// "replay failed somewhere." A failure in `replay`'s own setup preamble
/// (`new_game`/`reconcile_setup`/`finalize_setup`, before any event has
/// folded) reports `event_index: 0` with `message` prefixed to say which
/// setup step failed, since no event index is meaningful yet.
#[derive(Debug)]
pub struct ReplayError {
    pub event_index: usize,
    pub message: String,
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "replay failed at event {}: {}", self.event_index, self.message)
    }
}

impl std::error::Error for ReplayError {}

/// Apply one `RecordedEvent` to `state`: `RecordedEvent::Move` applies via
/// `apply_move`; each `Reveal*` variant applies via its matching `GameState`
/// reconciliation method (`arrange_orders_on_top`, `arrange_techs_on_top`,
/// `reveal_starting_order`). Returns the
/// underlying `MoveError`'s message on failure.
///
/// This is the ONE fold implementation (Task 3 -- the a6faa87 lesson was
/// that a second hand-kept copy of this match drifts from the first):
/// `replay_from` and `replay_strict` both loop this per event, differing
/// only in what they do around each step (plain short-circuit vs. also
/// running `check_invariants`).
fn fold_one_event(state: &GameState, ev: &RecordedEvent, rules: &Rules) -> Result<GameState, String> {
    let step: Result<GameState, MoveError> = (|| {
        Ok(match ev {
            RecordedEvent::Move(mv) => apply_move(state, mv, rules)?,
            RecordedEvent::RevealOrders(cards) => {
                let mut s = state.clone();
                s.arrange_orders_on_top(cards)?;
                s
            }
            RecordedEvent::RevealTechs(cards) => {
                let mut s = state.clone();
                s.arrange_techs_on_top(cards)?;
                s
            }
            RecordedEvent::RevealStartingOrder { player, card } => {
                let mut s = state.clone();
                s.reveal_starting_order(*player, *card)?;
                s
            }
        })
    })();
    step.map_err(|e| e.to_string())
}

/// Fold `events` over `initial`, in order, via `fold_one_event`. Returns the
/// first error encountered, tagged with its 0-based index into `events` --
/// short-circuits the loop, so events after the bad one never run.
///
/// This is `replay`'s event-folding loop, factored out (Task 13) so a
/// caller with some OTHER already-known-good starting `GameState` --
/// not necessarily one `new_game`+`finalize_setup` can reconstruct from a
/// `(seed, config, choices)` triple -- can reuse the exact same fold. The
/// yokohama-cli `Session::undo` checkpoint path is the motivating case: a
/// `for_test_from_state` (test-only) session's base state is hand-built,
/// so `replay` itself has nothing to replay FROM; before this it hand-kept
/// a duplicate of this loop client-side (`replay_from_checkpoint`) instead.
pub fn replay_from(initial: &GameState, events: &[RecordedEvent], rules: &Rules) -> Result<GameState, ReplayError> {
    let mut state = initial.clone();
    for (i, ev) in events.iter().enumerate() {
        state = fold_one_event(&state, ev, rules).map_err(|message| ReplayError { event_index: i, message })?;
    }
    Ok(state)
}

/// Reconstruct the pre-event-log `GameState` a `GameRecord` describes:
/// `new_game`, optional `reconcile_setup` (live-tracked games carrying
/// `setup_observations`), then `finalize_setup`. Factored out of `replay`
/// (Task 3) so `replay_strict` shares the exact same setup preamble instead
/// of a second hand-kept copy. A failure at any of the three steps reports
/// `event_index: 0` (see `ReplayError`'s doc comment), since no event index
/// is meaningful before the log starts folding.
fn rebuild_initial_state(record: &GameRecord, rules: &Rules) -> Result<GameState, ReplayError> {
    let mut pending = new_game(record.config.clone(), rules, record.seed)
        .map_err(|e| ReplayError { event_index: 0, message: format!("new_game: {e}") })?;
    if let Some(obs) = &record.setup_observations {
        pending = reconcile_setup(pending, obs, rules)
            .map_err(|e| ReplayError { event_index: 0, message: format!("reconcile_setup: {e}") })?;
    }
    finalize_setup(pending, &record.choices)
        .map_err(|e| ReplayError { event_index: 0, message: format!("finalize_setup: {e}") })
}

/// Reconstruct the `GameState` a `GameRecord` describes: `rebuild_initial_state`
/// (`new_game` -> optional `reconcile_setup` -> `finalize_setup`), then
/// `replay_from` folds `record.events` over the resulting state in order.
///
/// Does not refuse to replay a record written by a different engine
/// version -- v1 only warns (to stderr) on a mismatch. A future engine
/// version could change data layout or apply a rules correction in a way
/// that makes an old record replay to a different state than the one that
/// produced it; `replay_strict` (Task 3) is the hard-rejecting alternative
/// training pipelines should use instead.
///
/// Returns the first error encountered as a `ReplayError` naming the failed
/// event's index (see its doc comment) -- a `new_game`/`reconcile_setup`/
/// `finalize_setup` failure (before any event has folded) reports index 0.
pub fn replay(record: &GameRecord, rules: &Rules) -> Result<GameState, ReplayError> {
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
    match record.semantics_rev {
        None => eprintln!(
            "warning: GameRecord carries no semantics_rev (missing semantics provenance; pre-Phase-2.9 record) -- replaying anyway (v1 does not refuse on missing provenance)",
        ),
        Some(rev) if rev != SEMANTICS_REV => eprintln!(
            "warning: GameRecord.semantics_rev={rev} does not match running engine SEMANTICS_REV={SEMANTICS_REV} -- replaying anyway (v1 does not refuse on a semantics_rev mismatch)",
        ),
        Some(_) => {}
    }

    // Lenient replay deliberately does NOT verify `terminal_digest`: that is a
    // deep post-fold property, in the same strict-only bucket as the per-event
    // invariant checks (see `replay_strict`).
    let state = rebuild_initial_state(record, rules)?;
    replay_from(&state, &record.events, rules)
}

/// Strict replay for training pipelines (design §4.1): reject, don't warn.
/// - engine_version / rules_hash mismatch, or `semantics_rev` mismatch OR
///   missing (`None` -- unknown provenance, fail-closed) => Err at event_index 0
/// - every Move event goes through the checked `apply_move` (membership)
/// - `check_invariants` runs after EVERY event
/// - if the record carries a `terminal_digest`, the replayed terminal state
///   must reproduce it (Err at the past-the-end event index otherwise) --
///   defense in depth that catches an events-vs-digest divergence even when
///   every intermediate state passed its invariant check
///
/// Shares `rebuild_initial_state`/`fold_one_event` with `replay`/`replay_from`
/// (Task 3) -- there is exactly one setup preamble and one event fold in
/// this module; strict vs. lenient is purely what each caller does with the
/// version check and around each folded step.
pub fn replay_strict(record: &GameRecord, rules: &Rules) -> Result<GameState, ReplayError> {
    let running = env!("CARGO_PKG_VERSION");
    if record.engine_version != running {
        return Err(ReplayError {
            event_index: 0,
            message: format!(
                "engine_version mismatch: record \"{}\" vs running \"{running}\" (strict mode rejects)",
                record.engine_version
            ),
        });
    }
    let expected = rules.content_hash();
    if record.rules_hash != expected {
        return Err(ReplayError { event_index: 0, message: format!(
            "rules_hash mismatch: record \"{}\" vs running \"{expected}\" (strict mode rejects)", record.rules_hash) });
    }
    match record.semantics_rev {
        // Fail-closed on UNKNOWN provenance (FIX-B P1): a record with no
        // semantics_rev key predates the stamp -- strict mode must not
        // silently treat it as a current record.
        None => {
            return Err(ReplayError { event_index: 0, message: format!(
                "missing semantics provenance: record carries no semantics_rev (pre-Phase-2.9 record; running SEMANTICS_REV={SEMANTICS_REV}; strict mode rejects)") });
        }
        Some(rev) if rev != SEMANTICS_REV => {
            return Err(ReplayError { event_index: 0, message: format!(
                "semantics_rev mismatch: record {rev} vs running {SEMANTICS_REV} (strict mode rejects)") });
        }
        Some(_) => {}
    }
    let initial = rebuild_initial_state(record, rules)?;
    let mut state = initial;
    for (i, ev) in record.events.iter().enumerate() {
        state = fold_one_event(&state, ev, rules).map_err(|message| ReplayError { event_index: i, message })?;
        crate::game::invariants::check_invariants(&state, rules)
            .map_err(|m| ReplayError { event_index: i, message: format!("invariant violated: {m}") })?;
    }
    // Defense in depth: if the record carries a terminal-state digest, the
    // state the events folded to must reproduce it. `event_index` is the
    // past-the-end index -- the digest covers the whole run, not one event.
    if let Some(expected_digest) = &record.terminal_digest {
        let actual_digest = state_digest(&state);
        if &actual_digest != expected_digest {
            return Err(ReplayError {
                event_index: record.events.len(),
                message: format!(
                    "terminal-state digest mismatch: record {expected_digest} vs replayed {actual_digest} (strict mode rejects)"
                ),
            });
        }
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_with_empty_moves_and_current_engine_version() {
        let rules = Rules::empty_for_tests();
        let config = GameConfig::for_player_count(2);
        let record = GameRecord::new(
            7,
            config.clone(),
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(1)],
            &rules,
        );
        assert_eq!(record.engine_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(record.seed, 7);
        assert_eq!(record.config, config);
        assert!(record.events.is_empty());
        // New records carry the running semantics revision and no digest yet
        // (stamped only at finalization/save via `stamp_terminal_digest`).
        assert_eq!(record.semantics_rev, Some(SEMANTICS_REV));
        assert_eq!(record.terminal_digest, None);
    }

    #[test]
    fn push_move_appends_in_order() {
        let rules = Rules::empty_for_tests();
        let mut record = GameRecord::new(0, GameConfig::for_player_count(2), vec![], &rules);
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
        let mut record = GameRecord::new(3, config, choices, &rules);
        record.push_event(RecordedEvent::RevealOrders(vec![expected_top]));
        let state = replay(&record, &rules).unwrap();
        assert_eq!(state.order_deck.last(), Some(&expected_top));
    }

    /// Task 13 test (a): a bad event partway through the log must name its
    /// own 0-based index into `record.events`, not just "some event failed"
    /// -- a 3-event record with event 1 corrupted must report index 1
    /// exactly, regardless of event 2 (never reached) being present at all.
    #[test]
    fn replay_of_illegal_event_names_its_index() {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)];
        let mut record = GameRecord::new(4, config, choices, &rules);
        record.push_move(Move::EndAdditionalPhase); // event 0: legal
        // OrderCardId(255) was never dealt (real ids are 1-36) -- can never
        // be in any player's hand, so this is illegal regardless of phase.
        record.push_move(Move::FulfillOrder { order_id: OrderCardId(255) }); // event 1: illegal
        record.push_move(Move::EndAdditionalPhase); // event 2: never reached
        assert_eq!(record.events.len(), 3);

        let err = replay(&record, &rules).unwrap_err();
        assert_eq!(err.event_index, 1);
    }

    /// A record with some events to hang the provenance pins off. Setup plus
    /// the first `moves` legal moves of a real 2P game, terminal digest stamped
    /// — i.e. a record `replay_strict` accepts.
    fn strict_clean_record(seed: u64, moves: usize) -> (GameRecord, GameState, Rules) {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let pending = new_game(config, &rules, seed).unwrap();
        let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(1)];
        // From `pending.state.config`, so `config.setup_seed == record.seed`.
        let mut record = GameRecord::new(seed, pending.state.config.clone(), choices.clone(), &rules);
        let mut state = finalize_setup(pending, &choices).unwrap();
        for _ in 0..moves {
            let mv = crate::game::legal_moves::get_legal_moves(&state, &rules)[0].clone();
            state = apply_move(&state, &mv, &rules).unwrap();
            record.push_move(mv);
        }
        record.stamp_terminal_digest(&state);
        (record, state, rules)
    }

    fn sample_provenance() -> RunProvenance {
        RunProvenance {
            schema: RUN_PROVENANCE_SCHEMA,
            master_seed: Some(0xA3E1_2026_0726_0008),
            run_id: Some("ra-vs-fw".to_string()),
            seats: vec![
                SeatProvenance {
                    controller: "uniform".to_string(),
                    version: 1,
                    params: std::collections::BTreeMap::new(),
                    agent_seed: Some(11),
                    observation: OBSERVATION_VIEW.to_string(),
                },
                SeatProvenance {
                    controller: "family-weighted".to_string(),
                    version: 1,
                    params: [("retreat_ppm".to_string(), 166_667i64)].into_iter().collect(),
                    agent_seed: Some(22),
                    observation: OBSERVATION_VIEW.to_string(),
                },
            ],
        }
    }

    /// Provenance is METADATA: it says who played, and nothing about the game
    /// changes when it changes. Pinned rather than argued because the whole
    /// design (`GameRecord.run` serving arena AND CLI, spec §5) rests on it —
    /// if provenance could move a replayed state, a corpus record's label and
    /// its content would be entangled and neither could be trusted alone.
    #[test]
    fn provenance_is_replay_inert() {
        let (mut record, terminal, rules) = strict_clean_record(41, 6);
        assert!(record.run.is_none(), "a fresh record carries no provenance");
        let baseline = replay_strict(&record, &rules).expect("the fixture must be strict-clean");
        assert_eq!(baseline, terminal);
        let baseline_digest = record.terminal_digest.clone().expect("stamped above");
        let baseline_rules_hash = record.rules_hash.clone();

        // None -> Some.
        record.run = Some(sample_provenance());
        let with_provenance = replay_strict(&record, &rules)
            .expect("adding provenance must not change what strict replay accepts");
        assert_eq!(with_provenance, baseline, "provenance moved the replayed state");

        // Some -> a DIFFERENT Some: every field mutated, including the seat
        // vector's length (a record claiming three seats for a 2P game).
        let mut lying = sample_provenance();
        lying.schema = 99;
        lying.master_seed = Some(0);
        lying.run_id = Some("not-the-cell-that-ran".to_string());
        lying.seats.push(SeatProvenance {
            controller: "a controller that never existed".to_string(),
            version: 7,
            params: [("nonsense".to_string(), -1i64)].into_iter().collect(),
            agent_seed: None,
            observation: "not an observation mode".to_string(),
        });
        record.run = Some(lying);
        let mutated = replay_strict(&record, &rules)
            .expect("mutating provenance must not change what strict replay accepts");
        assert_eq!(mutated, baseline, "provenance moved the replayed state");

        // ...and it entered NEITHER digest. `state_digest` takes a `GameState`,
        // so it cannot see provenance by construction; `rules_hash` covers the
        // static registries. Asserted anyway — this is the property spec §5
        // states as a non-goal, and a non-goal nobody checks is a hope.
        assert_eq!(record.terminal_digest.as_ref(), Some(&baseline_digest));
        assert_eq!(state_digest(&mutated), baseline_digest);
        assert_eq!(record.rules_hash, baseline_rules_hash);
    }

    /// Every record written before this field existed has no `run` key at all.
    /// Ledger row 7's replay-compat intent says those records keep replaying —
    /// which is exactly why provenance is a `#[serde(default)]` Option and why
    /// the policy identity inside it is a NAME plus a params map rather than a
    /// typed enum (serde refuses an unknown enum variant, so a future rung
    /// would turn every older binary's read of a new corpus into a hard error).
    #[test]
    fn an_old_shape_record_without_run_still_replays_strict_clean() {
        let (mut record, terminal, rules) = strict_clean_record(42, 6);
        record.run = Some(sample_provenance());

        // Delete the key from the serialized document, which is what an
        // old-shape file IS — not a `None` in memory, which would test the
        // writer instead of the reader.
        let mut document: serde_json::Value = serde_json::to_value(&record).unwrap();
        assert!(document.get("run").is_some(), "the writer must emit a Some provenance");
        document.as_object_mut().unwrap().remove("run");
        let text = serde_json::to_string(&document).unwrap();
        assert!(!text.contains("\"run\""), "the run key survived deletion");

        let old_shape: GameRecord = serde_json::from_str(&text).expect("an old-shape record must parse");
        assert_eq!(old_shape.run, None, "a missing key must read as no provenance");
        assert_eq!(
            replay_strict(&old_shape, &rules).expect("an old-shape record must replay strict-clean"),
            terminal,
        );

        // The complementary direction: a record whose provenance IS present
        // round-trips bit-exactly, which is what `Eq`-safe params buy (the
        // fixed-point ppm integers of spec §5 — a float would make this
        // comparison depend on formatting).
        let round_tripped: GameRecord = serde_json::from_str(&serde_json::to_string(&record).unwrap()).unwrap();
        assert_eq!(round_tripped, record);
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
        let mut record =
            GameRecord::new(11, config, vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)], &rules);
        record.setup_observations = Some(obs.clone());
        let state = replay(&record, &rules).unwrap();
        for i in 0..6 {
            assert_eq!(state.port_state.port_a.slots[i], Some(obs.port_a.unwrap()[i]));
        }
    }
}
