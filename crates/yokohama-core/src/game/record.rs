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

/// The `SeatProvenance::controller` value for a HUMAN seat (spec §5 names it on
/// the same footing as the policy names, under the same never-hand-typed rule).
/// T12's CLI is the consumer — the wizard, the panel, and the resume path all
/// read THIS const, never a scattered literal (T8 review finding 2).
pub const CONTROLLER_HUMAN: &str = "human";

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
    /// (`0` on a document that carried no key at all — "the writer said
    /// nothing" — never a valid written value; `RUN_PROVENANCE_SCHEMA` is 1.)
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
    /// The controller IMPLEMENTATION version under that stable name, bumped when
    /// figures recorded under the same name stop being COMPARABLE. Distinct from
    /// [`RunProvenance::schema`], which is about this struct's shape and not
    /// about behaviour.
    ///
    /// **One const per controller NAME** (spec §0.5 / L12), and comparability is
    /// the criterion rather than authorship: `game::policy::POLICY_IMPL_VERSION`
    /// for a rollout rung, `greedy::GREEDY_IMPL_VERSION`,
    /// `lookahead::LOOKAHEAD_IMPL_VERSION`, `lookahead::ATOMIC_DIAG_IMPL_VERSION`
    /// for the evaluator controllers. A change to something SHARED — the
    /// evaluator weights, say — bumps every controller const that embeds it,
    /// honestly, because every one of their recorded figures stops being
    /// comparable at once; `eval::EVAL_IMPL_VERSION` additionally rides
    /// [`params`](Self::params) for finer attribution.
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
    /// record. PROVENANCE ONLY, deliberately not a compatibility key
    /// (2026-07-27 decoupling): the workspace version moves at a release cut
    /// while executable semantics do not -- `semantics_rev` (with
    /// `rules_hash` and the terminal digest) is what gates `replay_strict`,
    /// and rev, never version, is the corpus layout's compatibility key
    /// (`arena/corpus.rs`). Lenient `replay`
    /// and the CLI's `Session::load` surface a drift as a warning so a
    /// human sees which build wrote the file; nothing refuses on it.
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
        // T8 review ruling (probe 3): the None arm stays a no-op — fabricating
        // a zero-seat RunProvenance to hold the context would be worse — but
        // it is LOUD in debug, because the only wrong path here is a caller
        // ordering this before set_run_seats (or forgetting seats entirely),
        // which would silently produce corpus records with no run-level label.
        debug_assert!(
            self.run.is_some(),
            "set_run_context before set_run_seats: the context has no provenance to land in",
        );
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
/// Warns (to stderr) on any provenance drift -- engine_version, rules_hash,
/// semantics_rev -- and replays anyway. `replay_strict` (Task 3) is the
/// hard-rejecting alternative training pipelines should use instead; it
/// rejects on the SEMANTIC keys (rules_hash, semantics_rev, terminal
/// digest) but treats `engine_version` as provenance like this function
/// does (2026-07-27 decoupling -- a release cut must not split a
/// rev-keyed corpus).
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
/// - `rules_hash` mismatch, or `semantics_rev` mismatch OR missing (`None`
///   -- unknown provenance, fail-closed) => Err at event_index 0.
///   `engine_version` is deliberately NOT a gate (2026-07-27 decoupling,
///   spec `docs/plans/2026-07-27-replay-strict-engine-version-decouple.md`):
///   the workspace version moves at a release cut while executable semantics
///   do not, and the corpus layout (`arena/corpus.rs`) promises rev-keyed
///   records strict-replay against their own `SEMANTICS_REV`; the guards
///   below protect a forgotten rev bump at per-record granularity, which a
///   release-cut version string cannot.
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
/// provenance checks and around each folded step.
pub fn replay_strict(record: &GameRecord, rules: &Rules) -> Result<GameState, ReplayError> {
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

/// The state after the FIRST `k` EVENTS of `record` — the replay-to-position
/// primitive (Phase 3.5c W5, design §3).
///
/// **The off-by-one, stated explicitly, because every caller depends on it:**
/// `k` counts events APPLIED, not an event index. So `state_at(record, 0)` is
/// the post-setup initial state with nothing folded onto it; `state_at(record,
/// record.events.len())` is the state a full [`replay`] produces; and the state
/// a mover decided IN before the move that became event `n` is
/// `state_at(record, n)` — event `n` is the first event NOT applied. (`why <n>`
/// and the deferred replay viewer both address events by that index, so a
/// silent off-by-one here would explain the wrong decision.)
///
/// `k > record.events.len()` is a typed refusal naming the bound — **never a
/// clamp**: a caller addressing events by index must learn that it asked for
/// something that does not exist rather than quietly receive the terminal
/// state. The range is checked BEFORE the setup preamble runs, so an
/// out-of-range `k` is reported as such even on a record whose setup would also
/// fail; that refusal carries `event_index: record.events.len()` (the
/// past-the-end boundary that was exceeded — `replay_strict`'s terminal-digest
/// precedent for an error about the whole record rather than one event), since
/// no event failed to apply.
///
/// **`ReplayError::event_index` is RECORD-RELATIVE here**, unlike the
/// slice-relative trap the CLI's `Session::load` documents (session.rs:296-308,
/// where a fresh 1-element slice per iteration makes the error's own index
/// structurally always 0 and the true index has to be reconstructed by the
/// caller). This function folds `&record.events[..k]`, a slice that starts at
/// the record's own event 0, so the index a failure reports IS the record's
/// event index and needs no adjustment.
///
/// Deliberately NEITHER lenient nor strict about provenance: unlike [`replay`]
/// it prints no drift warnings (a positioning primitive gets called per
/// invocation — per-call stderr would be noise), and unlike [`replay_strict`]
/// it runs no per-event invariant checks and refuses nothing on
/// `rules_hash`/`semantics_rev`. A caller that needs those verdicts asks for
/// them by calling the function that renders them.
///
/// Cost is O(k) checked applies per call — trivial at CLI scales, and no state
/// is cached anywhere (`GameState` stays history-free by design).
pub fn state_at(record: &GameRecord, k: usize, rules: &Rules) -> Result<GameState, ReplayError> {
    let available = record.events.len();
    if k > available {
        return Err(ReplayError {
            event_index: available,
            message: format!(
                "state_at({k}) is past the end of the event log: the record carries {available} events, \
                 so k must be at most {available} (state_at never clamps)"
            ),
        });
    }
    let initial = rebuild_initial_state(record, rules)?;
    replay_from(&initial, &record.events[..k], rules)
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

    /// A record of the first `moves` legal moves of a real 2P game, PLUS every
    /// state the game actually passed through: `states[k]` is the state after
    /// the first `k` events, captured AS THE GAME WAS PLAYED rather than
    /// reconstructed afterwards. That is what makes it usable as an independent
    /// witness for `state_at` — asserting against a state the record's own
    /// machinery rebuilt would only prove that machinery agrees with itself.
    /// `states.len() == moves + 1` (index 0 is the post-setup initial state).
    fn recorded_walk(seed: u64, moves: usize) -> (GameRecord, Vec<GameState>, Rules) {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let pending = new_game(config, &rules, seed).unwrap();
        let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(1)];
        // From `pending.state.config`, so `config.setup_seed == record.seed`.
        let mut record = GameRecord::new(seed, pending.state.config.clone(), choices.clone(), &rules);
        let mut state = finalize_setup(pending, &choices).unwrap();
        let mut states = vec![state.clone()];
        for _ in 0..moves {
            let mv = crate::game::legal_moves::get_legal_moves(&state, &rules)[0].clone();
            state = apply_move(&state, &mv, &rules).unwrap();
            record.push_move(mv);
            states.push(state.clone());
        }
        (record, states, rules)
    }

    /// A record with some events to hang the provenance pins off. Setup plus
    /// the first `moves` legal moves of a real 2P game, terminal digest stamped
    /// — i.e. a record `replay_strict` accepts. The construction is
    /// `recorded_walk`'s (one fixture builder, not two that can drift); this
    /// wrapper adds the digest stamp and keeps only the terminal state, which
    /// is all its callers want.
    fn strict_clean_record(seed: u64, moves: usize) -> (GameRecord, GameState, Rules) {
        let (mut record, states, rules) = recorded_walk(seed, moves);
        let terminal = states.last().expect("recorded_walk always captures the initial state").clone();
        record.stamp_terminal_digest(&terminal);
        (record, terminal, rules)
    }

    /// `state_at`'s off-by-one, stated as a test: `k` counts EVENTS APPLIED,
    /// so `state_at(r, 0)` applies none of them and is the post-setup initial
    /// state — the state the move that became event 0 was decided IN. Three
    /// readings fail here: an off-by-one (index 0 returning the state after the
    /// first event), a clamp, and a "0 means the whole log" reading.
    #[test]
    fn state_at_zero_is_the_post_setup_initial_state() {
        let (record, states, rules) = recorded_walk(3, 3);
        assert_eq!(record.events.len(), 3, "index 0 means nothing on a record with no events");

        let at_zero = state_at(&record, 0, &rules).expect("k=0 is in range for every record");
        assert_eq!(at_zero, states[0], "state_at(0) must be the state the game started from");

        // Anti-vacuity, both directions: the fixture's events genuinely move
        // the state, so `state_at(0)` agreeing with the initial state is not
        // something every implementation would satisfy — in particular an
        // implementation that folded the whole log regardless of `k` fails the
        // second assertion.
        assert_ne!(states[0], states[3], "the fixture's events must move the state");
        assert_ne!(
            at_zero,
            replay(&record, &rules).expect("the fixture replays"),
            "state_at(0) must not be the fully replayed state",
        );
    }

    /// The whole ladder, cross-checked against a DIFFERENT route to each index:
    /// the `Session::load` walk (rebuild the setup preamble, then fold one
    /// event at a time through `replay_from`) reaches state `k` in `k`
    /// one-event calls where `state_at` reaches it in one `k`-event call.
    /// Agreement at every index is what makes `state_at` a positioning
    /// primitive rather than "replay, roughly".
    #[test]
    fn state_at_matches_an_incremental_fold_at_every_index() {
        let (record, states, rules) = recorded_walk(5, 6);
        assert_eq!(record.events.len(), 6);
        // Anti-vacuity: with two adjacent states equal, an off-by-one at that
        // index would be invisible to the comparisons below.
        for k in 1..states.len() {
            assert_ne!(states[k - 1], states[k], "adjacent states must differ (index {k})");
        }

        let pending = new_game(record.config.clone(), &rules, record.seed).unwrap();
        let mut walked = finalize_setup(pending, &record.choices).unwrap();
        // `states` is indexed by events-applied, so iterating it walks exactly
        // `k in 0..=events.len()` — pinned here, since the loop only covers the
        // whole ladder (the top index included) if that relation holds.
        assert_eq!(states.len(), record.events.len() + 1);
        for (k, played) in states.iter().enumerate() {
            if k > 0 {
                walked = replay_from(&walked, std::slice::from_ref(&record.events[k - 1]), &rules)
                    .unwrap_or_else(|e| panic!("the incremental fold failed at event {}: {e}", k - 1));
            }
            let at_k = state_at(&record, k, &rules).unwrap_or_else(|e| panic!("state_at({k}) failed: {e}"));
            assert_eq!(at_k, walked, "state_at({k}) disagreed with the incremental fold");
            assert_eq!(&at_k, played, "state_at({k}) disagreed with the state the game played through");
        }

        // ...and the top of the ladder is the whole-record entry point, which
        // is the other half of the off-by-one: `k == events.len()` is IN range
        // and is the terminal state.
        assert_eq!(
            state_at(&record, record.events.len(), &rules).unwrap(),
            replay(&record, &rules).unwrap(),
            "state_at(events.len()) must equal a full replay",
        );
    }

    /// Past the end is a typed refusal naming the bound — never a clamp. A
    /// clamp is the dangerous shape here: a caller addressing events by index
    /// (`why <n>`) would silently get the terminal state for every
    /// out-of-range index and never learn it asked for something that does not
    /// exist.
    #[test]
    fn state_at_past_the_end_is_a_typed_error_not_a_clamp() {
        let (record, states, rules) = recorded_walk(7, 4);
        let n = record.events.len();
        assert_eq!(n, 4);

        // The boundary itself is IN range.
        assert_eq!(state_at(&record, n, &rules).unwrap(), states[n]);

        let err = state_at(&record, n + 1, &rules).expect_err("one past the end must not succeed");
        assert!(
            err.message.contains(&(n + 1).to_string()) && err.message.contains(&n.to_string()),
            "the refusal must name both what was asked and the bound: {}",
            err.message,
        );
        // Nothing failed to APPLY, so the index points at the past-the-end
        // boundary that was exceeded — `replay_strict`'s terminal-digest
        // precedent for an error about the whole record rather than one event.
        assert_eq!(err.event_index, n);

        // Far past the end refuses the same way rather than saturating.
        assert!(state_at(&record, usize::MAX, &rules).is_err(), "usize::MAX must refuse, not clamp");
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
