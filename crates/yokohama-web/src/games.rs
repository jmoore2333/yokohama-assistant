//! Live games, and the rule that none of them may exist only in memory.
//!
//! A handle is a `Repl` behind a mutex. It is a cache, never the record: every
//! applied move is persisted immediately, and a handle that is absent is
//! rebuilt from the stored `GameRecord` through `Session::from_record`, which
//! runs every refusal the CLI's resume path runs. That is what lets this server
//! live in a container that sleeps.
//!
//! **M6.1 put a second document in front of that one, and it lives next door**
//! ([`crate::tables`] — one type cut along a seam, the table half there and the
//! record half here). A game now begins as a TABLE ([`crate::table`]) with no
//! record at all: `create` writes the table,
//! each seated player is dealt and keeps from its own view, and the LAST keep
//! finalizes — builds the session, writes the record, and flips the table to
//! `playing`. Until then there is nothing for `Session::from_record` to rebuild
//! and every route that needs a session answers 409. The two writes are not one
//! transaction, so both crash windows are repaired on the next read
//! ([`Games::settle`]) rather than left to a human.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use yokohama_cli::repl::Repl;
use yokohama_cli::session::Session;
use yokohama_core::{GameRecord, Rules};

use crate::signal::Signals;
use crate::store::{Doc, GameId, Store, Version, WriteError};
use crate::table::{TableStatus, TurnMirror};

/// Why a table operation was refused, in the shapes an HTTP layer answers with.
///
/// **`Dealing` carries no sentence and is not a `Conflict`**, deliberately: it
/// is the one refusal a client BRANCHES on rather than displays — a game opened
/// by id that is still dealing needs the setup screen, not an error banner — so
/// it reaches the wire as a 409 carrying `"status": "dealing"` beside the
/// sentence, and a variant of its own is what keeps that from being a string
/// comparison in the handler.
#[derive(Debug)]
pub enum GameError {
    NotFound(String),
    /// Every route that needs a SESSION, while the table is still dealing.
    Dealing,
    /// A session route at a table still FILLING ITS SEATS (M6.2, spec §3.4).
    /// A sibling of `Dealing` for the same reason: the client branches on it —
    /// an open table needs the lobby's join screen, not an error banner.
    Open,
    /// A table-phase route (join, deal, keep) at a table already playing or
    /// finished (M6.2, spec §3.4). The mirror image of `Dealing`, and typed for
    /// the same reason: a client that opened a link to a game that has started
    /// wants the board, not a sentence.
    Playing,
    Forbidden(String),
    Conflict(String),
    BadRequest(String),
    /// This process failed at something the caller asked for correctly.
    Server(String),
}

impl std::fmt::Display for GameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(m)
            | Self::Forbidden(m)
            | Self::Conflict(m)
            | Self::BadRequest(m)
            | Self::Server(m) => f.write_str(m),
            Self::Dealing => f.write_str(DEALING_SENTENCE),
            Self::Open => f.write_str(OPEN_SENTENCE),
            Self::Playing => f.write_str(PLAYING_SENTENCE),
        }
    }
}

impl std::error::Error for GameError {}

/// The one sentence a dealing table answers every session route with. Beside it
/// on the wire rides `"status": "dealing"`, which is what the client branches
/// on — the words are for a human reading a `curl`.
pub const DEALING_SENTENCE: &str = "this table is still dealing";

/// The sentence a table that is still filling its seats answers with. Beside it
/// rides `"status": "open"` (M6.2, spec §3.4).
pub const OPEN_SENTENCE: &str = "this table is still open";

/// The sentence a table-phase route answers with once the game has started.
/// Beside it rides `"status": "playing"`.
pub const PLAYING_SENTENCE: &str = "this table is playing";

/// The facts the list mirrors off a live session (R-plan-3).
///
/// The winner is read from `final_result` — the engine's own ranking with the
/// KB's clockwise tie-break — and ONLY once the game is over, because
/// `final_result` scores whatever position it is handed and would otherwise
/// name a leader as a winner.
pub(crate) fn mirror_of(repl: &Repl) -> TurnMirror {
    let over = repl.session.is_over();
    TurnMirror {
        to_move: repl.session.state().current_player.0,
        over,
        winner: over.then(|| repl.session.final_result().winner.0),
    }
}

/// Every live game, plus the store behind them.
pub struct Games {
    pub(crate) rules: Arc<Rules>,
    pub(crate) store: Arc<dyn Store>,
    pub(crate) live: Mutex<HashMap<GameId, Arc<Mutex<Repl>>>>,
    /// What this process last successfully LEFT IN THE STORE, per id — the
    /// [`Version`] the store reported for the JSON written (or, on a cold
    /// rebuild, for the JSON read).
    ///
    /// **This is what makes the write a compare-and-swap rather than a
    /// monotonicity heuristic**, and the difference is a working undo. See
    /// [`Games::persist`].
    pub(crate) written: Mutex<HashMap<GameId, Version>>,
    /// Serializes `create` against every other `create` — and, since M6.2,
    /// NOTHING ELSE. Every other table writer holds that table's own lock
    /// ([`Games::lock_for`], R-2.9).
    ///
    /// `create` is check-then-write across a slow middle, so without this two
    /// requests both read "absent" and both went on to build, write and insert.
    /// A per-table lock cannot serve there: the id may be DRAWN, so two requests
    /// racing for one name are not yet two requests about one table, and the
    /// occupancy check reads two documents. Held across the whole of it, because
    /// any shorter hold leaves the same window somewhere else.
    ///
    /// A dedicated lock rather than holding `live`: `live` is taken by every
    /// `handle`, and blocking ordinary reads for the length of a game setup to
    /// serialize an operation a user performs by hand is the wrong trade.
    pub(crate) table_lock: Mutex<()>,
    /// One lock PER TABLE (M6.2, R-2.9), created on first use and kept.
    ///
    /// `table_lock` above serializes every table writer in the process against
    /// every other one, which is right for `create` — where the thing being
    /// protected is the ID SPACE and two requests for different names still
    /// race for the same one — and wrong for everything else. A finalize builds
    /// a session and writes a record while holding it, so one table's last keep
    /// held the whole lobby's write path; with the lobby, that is twenty people
    /// waiting on a game none of them is at.
    ///
    /// The outer mutex is held only long enough to hand back the `Arc`, never
    /// across the table's own lock, so the two can never be taken in two orders.
    /// The map is not pruned: an entry is a pointer and a mutex, and a delete
    /// that removed one would drop it while a blocked writer was about to take
    /// it and hand the next writer a DIFFERENT lock for the same table.
    pub(crate) locks: Mutex<HashMap<GameId, Arc<Mutex<()>>>>,
    /// Ids deleted since this process started, so a request that was already
    /// holding a handle cannot write the record back after `DELETE` removed it.
    /// Cleared by `create`, so a name can be reused.
    pub(crate) deleted: Mutex<std::collections::HashSet<GameId>>,
    /// The change signal (M6.3, spec §2): one `watch` channel per table and
    /// one for the house, bumped AFTER each of this type's writes lands.
    ///
    /// It lives here because every write site does, and only because of that:
    /// [`crate::signal`] knows nothing about a game, a table or a seat, and
    /// nothing in this module reads a bump back. It is a notification that
    /// something this process wrote has changed, for the streams in front of
    /// it to answer — so a failed or refused write bumps nothing, and every
    /// call below sits on a success path.
    pub(crate) signals: Signals,
}

/// How many `Repl`s stay resident.
///
/// **A handle is a cache, never the record** — this module's own first
/// paragraph — so dropping one costs a replay and loses nothing. The map used
/// to grow forever and shrink only on `DELETE`: measured at ~27.6 KB retained
/// per game across 200 creates, with nothing in the API able to reclaim one.
/// Each entry also owns a full `Rules` clone, which is why the figure is not
/// small.
///
/// 64 is chosen for the shape of the deployment rather than from a benchmark:
/// this is one operator's server behind Cloudflare Access, so the working set
/// is a handful of games, and a bound two orders of magnitude above that costs
/// nothing while turning unbounded growth into a fixed ceiling. An eviction is
/// invisible to a client — the next request rebuilds through
/// `Session::from_record`, which is the same path a slept container takes.
const MAX_RESIDENT: usize = 64;

impl Games {
    pub fn new(rules: Arc<Rules>, store: Arc<dyn Store>) -> Self {
        Self {
            rules,
            store,
            live: Mutex::new(HashMap::new()),
            written: Mutex::new(HashMap::new()),
            table_lock: Mutex::new(()),
            locks: Mutex::new(HashMap::new()),
            deleted: Mutex::new(std::collections::HashSet::new()),
            signals: Signals::default(),
        }
    }

    pub fn store(&self) -> &Arc<dyn Store> {
        &self.store
    }

    /// The change signal every write site bumps ([`Games::signals`] the field).
    /// The routes in front of this type subscribe through it; nothing here
    /// reads one back.
    pub fn signals(&self) -> &Signals {
        &self.signals
    }

    /// The table's OWN lock (R-2.9). `join`, `keep`, `delete` and a repairing
    /// `settle` TAKE it for the whole of what they do, and `settle_locked`,
    /// `finalize` and `flip_to_playing` are CALLED with it held — the
    /// distinction matters, because the mutex is not reentrant and a second
    /// take from inside one of those three would deadlock. `create` holds the
    /// global [`Games::table_lock`] instead, because the thing it is protecting
    /// is the id space and not one document.
    ///
    /// The entry is created on first use, so a table nobody has written costs
    /// nothing, and it is never removed — see [`Games::locks`] for why a delete
    /// that pruned the map would hand two writers two different locks for one
    /// table.
    pub(crate) fn lock_for(&self, id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().expect("games lock");
        Arc::clone(locks.entry(id.to_string()).or_insert_with(|| Arc::new(Mutex::new(()))))
    }

    /// The live handle for `id`, rebuilt from the store if it is not resident.
    ///
    /// **409 while the table is dealing**, which is what makes every session
    /// route answer a client that opened a game by id with the status it needs
    /// rather than a 404 it cannot act on.
    pub fn handle(&self, id: &str) -> Result<Arc<Mutex<Repl>>, GameError> {
        if let Some(h) = self.live.lock().expect("games lock").get(id) {
            return Ok(Arc::clone(h));
        }
        match self.settle(id)? {
            None => Err(GameError::NotFound(format!("no game named {id:?}"))),
            // **Before the dealing arm**, and the order is the claim: an open
            // table has no record either, and a client polling one must be told
            // which of the two it is looking at.
            Some((doc, _)) if doc.status == TableStatus::Open => Err(GameError::Open),
            Some((doc, _)) if doc.status == TableStatus::Dealing => Err(GameError::Dealing),
            Some(_) => self.live_handle(id),
        }
    }

    /// The handle alone: resident, or rebuilt from the RECORD.
    ///
    /// Deliberately does not consult the table document — [`Games::handle`] has
    /// already done that, and `settle`'s repair arm calls this to read the turn
    /// off a game whose table is mid-flip.
    pub(crate) fn live_handle(&self, id: &str) -> Result<Arc<Mutex<Repl>>, GameError> {
        if let Some(h) = self.live.lock().expect("games lock").get(id) {
            return Ok(Arc::clone(h));
        }
        let (json, version) = self
            .store
            .read(Doc::Record, id)
            .map_err(GameError::Server)?
            .ok_or_else(|| GameError::NotFound(format!("no game named {id:?}")))?;
        // The base for `persist`'s compare-and-swap: what we found in the store
        // is what we may overwrite until someone else changes it. Recorded
        // BEFORE `live` is taken — nothing may take `written` while holding
        // `live`.
        self.written.lock().expect("games lock").insert(id.to_string(), version);
        let record: GameRecord =
            serde_json::from_str(&json).map_err(|e| GameError::Server(e.to_string()))?;
        let session =
            Session::from_record(record, (*self.rules).clone()).map_err(GameError::Server)?;
        let mut repl = Repl::new(session);
        // A game that comes back with a bot to move is owed that move — the
        // same thing `main.rs` does through `opening()` when a save is resumed.
        // Persist only if the pump actually played, so reading a game that was
        // already waiting on a human rewrites nothing.
        let before = repl.session.record().events.len();
        bring_alive(&mut repl);
        if repl.session.record().events.len() != before {
            self.persist(id, &repl).map_err(GameError::Server)?;
        }
        let handle = Arc::new(Mutex::new(repl));
        let mut live = self.live.lock().expect("games lock");
        // Another request may have rebuilt the same game while this one was
        // replaying. Keep whichever landed first, so one game is never two
        // handles applying moves to two divergent records.
        let kept = Arc::clone(live.entry(id.to_string()).or_insert(handle));
        evict_down_to(&mut live, id);
        Ok(kept)
    }

    /// Write `repl`'s record through. Called after every mutation.
    ///
    /// **THREE guards, and each closes a measured way to lose a move.**
    ///
    /// A game DELETED while this request was in flight is not resurrected. The
    /// window is the whole of `Repl::step`, bot pump included — measured at
    /// 0.16-0.30 s in a debug build — and without the check the file came back:
    /// `DELETE` answered 204, the file went, the in-flight input finished, and
    /// `FsStore::write` re-created the parent directory and the record.
    ///
    /// A record that is a DIFFERENT GAME under the same id is not overwritten —
    /// identity by seed, checked first, because a name is not a game.
    ///
    /// And a record that is NOT WHAT THIS PROCESS LAST LEFT THERE is not
    /// overwritten. That is a **compare-and-swap**, and this paragraph used to
    /// say the opposite of the body below it: *"This is a monotonicity guard,
    /// not a compare-and-swap: it refuses a write that would go backwards."*
    /// It was a monotonicity guard, the swap replaced it, and the doc was left
    /// behind — the exact defect this lane has shipped for three consecutive
    /// milestones, in the function whose fix introduced it. Recorded rather
    /// than quietly deleted, because the next reader's instinct should be to
    /// distrust a paragraph that has stopped matching its code.
    ///
    /// **The swap is now the STORE's** ([`Store::write_if_version`]): the
    /// guards below decide what version the write may land on, and the store
    /// holds its own lock across the read, the compare and the write. Nothing
    /// about what is refused moved — the three refusals are the same three
    /// sentences in the same order — but a store that can enforce the condition
    /// (a row version, a conditional PUT) is now a second implementation rather
    /// than a rewrite of this function.
    ///
    /// **A FOURTH write rides along and is not a guard**: the table document's
    /// turn mirror (R-plan-3), refreshed after the record lands and IGNORED if
    /// it conflicts. See [`Games::mirror_turn`].
    ///
    /// The arm-by-arm reasoning, and the honest limits, are inline below.
    pub fn persist(&self, id: &str, repl: &Repl) -> Result<(), String> {
        if self.deleted.lock().expect("games lock").contains(id) {
            return Err(self.conflict(id, format!("{id:?} was deleted while this move was in flight")));
        }
        // Held across the read, the compare AND the write, so the swap is atomic
        // within this process. Nothing takes this lock while holding `live`, so
        // the `conflict` helper below (which does take `live`) cannot deadlock.
        let mut written = self.written.lock().expect("games lock");

        let record = repl.session.record();
        let events = record.events.len();
        // `expected` is what the write below is CONDITIONAL on: `None` means
        // "create, and refuse a present document", `Some(v)` means "replace
        // only what I just read". The guards decide which, and the store
        // performs the swap — the comparison that used to happen here by hand
        // now happens where the read and the write are under one lock.
        //
        // **A read that FAILS is PROPAGATED, never discarded.** "Cannot be
        // read" and "is not there" are different answers — the seam exists to
        // keep them apart — and `?` returns the first one through this
        // function's ordinary `Err` path, which does NOT evict: the handle is
        // right and the store is not, exactly as `conflict`'s doc says.
        //
        // The line was `.ok().flatten()`, and discarding the error defeated
        // that rule from the inside. A failed read became `expected: None`,
        // which is a CREATE, and a create over a present document is a
        // `Conflict` — so a transient I/O failure took the live session down,
        // the one outcome the two variants exist to prevent. It is not enough
        // that `FsStore`'s own write would have failed the same way and
        // answered `Io`: the guarantee has to hold for the TRAIT, and any store
        // whose read can fail where its write's read succeeds lands on the
        // evicting arm. Pinned by
        // `a_read_failure_refuses_the_write_and_leaves_the_handle_resident`.
        //
        // The old code, before the seam, wrote straight over an unreadable
        // record. That does not survive either, and deliberately: refusing is
        // the rule `create` has enforced since the `chmod 000` measurement,
        // applied to the other writer.
        let stored = self.store.read(Doc::Record, id)?;
        let mut expected: Option<Version> = None;
        if let Some((stored, current)) = &stored {
            if let Ok(on_disk) = serde_json::from_str::<GameRecord>(stored) {
                // IDENTITY BEFORE LENGTH — a name is not a game.
                //
                // The tombstone's own bound is "`create`, which clears the id",
                // and `create` clears it on ATTEMPT, before the new record
                // exists. So delete-then-recreate inside an in-flight `step`
                // left the doomed handle facing a record it passed both other
                // guards against: no tombstone (cleared), and FEWER events than
                // it holds (a fresh game), so the deleted game overwrote the new
                // one. Comparing the seed makes the two games distinguishable,
                // which a name plus a length never was.
                //
                // **The two seeds are COMPARED here and printed nowhere**, and
                // that is not tidiness. This sentence leaves the process: on an
                // `input`, `lib.rs` pushes it into `warnings` as `APPLIED BUT
                // NOT SAVED — {e}` and serializes it on the reply. So the
                // version that named both numbers handed the OLD game's player
                // the NEW table's setup seed — hidden-set member #1, which
                // regenerates that table's decks, boxed pile, board and every
                // deal. Narrow (a delete, a re-create under the same name, and
                // an `input` inside `Repl::step` while it happens) and never in
                // the corpus, so `no_committed_payload_carries_the_setup_seed`
                // cannot see it. Pinned instead by the refusal's own cell,
                // which reads the sentence for both spellings of both seeds.
                //
                // The length sentence below names an EVENT COUNT, which is
                // public arithmetic about a game the caller is already playing.
                if on_disk.seed != record.seed {
                    return Err(self.conflict(
                        id,
                        format!(
                            "{id:?} now holds a different game — refusing to write one game's \
                             events over another's"
                        ),
                    ));
                }
                // **COMPARE-AND-SWAP, not monotonicity.** The old guard asked
                // "does the store hold MORE events than I do?", which is true
                // of two opposite situations it could not tell apart: a stale
                // handle about to erase a move another writer already
                // acknowledged (refuse), and a live handle that just shortened
                // its OWN log because the player pressed Undo (write it).
                // `Session::undo` pops events — a record is a seed plus an
                // event log — so undo and staleness look identical to a length
                // comparison, and undo lost. The client was told "Undid last
                // entry.", the write was refused into a `warnings` line, and the
                // next request rebuilt from the store with the move back.
                //
                // The right question is whether the store is still what THIS
                // process last left there. That permits an undo, and it is
                // STRICTLY STRONGER than the length test rather than merely
                // more permissive: two divergent lines that reach the SAME
                // length differ in content and not in count, so a length
                // comparison waved them through and this does not.
                //
                // It detects a conflict; it does not prevent one. Within THIS
                // process `write_if_version` holds the store's own lock across
                // the read, the compare and the write; across processes nothing
                // serializes read-then-write — `FsStore` publishes atomically
                // per file (temp + rename), and that is the whole of it. Which
                // is unchanged, and was never true of the old guard either.
                match written.get(id) {
                    // Ours to overwrite, shorter or longer.
                    Some(base) if base == current => expected = Some(*current),
                    Some(_) => {
                        return Err(self.conflict(
                            id,
                            format!(
                                "{id:?} changed under this handle since it last wrote ({} stored) — \
                                 refusing to overwrite another writer's record",
                                on_disk.events.len(),
                            ),
                        ));
                    }
                    // No base: this process has neither read nor written this
                    // id, so it cannot know whose record is on disk. Fall back
                    // to the old conservative test rather than assuming.
                    None => {
                        if on_disk.events.len() > events {
                            return Err(self.conflict(
                                id,
                                format!(
                                    "{id:?} has advanced past this handle ({} stored vs {events} here) — refusing to write backwards",
                                    on_disk.events.len(),
                                ),
                            ));
                        }
                        expected = Some(*current);
                    }
                }
            } else {
                // A record that will not PARSE is still a record, and replacing
                // it is still a replace: conditioning on what was just read
                // keeps a second writer's repair from being clobbered by this
                // one. The old code wrote over it unconditionally; this refuses
                // only if it changed between that read and the write, which is
                // the same answer in every case a test can arrange and a
                // stronger one under a concurrent repair.
                expected = Some(*current);
            }
        }
        // `Session::to_record_json`, never `to_string_pretty(record())`: the
        // terminal digest is stamped at WRITE time, so serializing the record
        // directly here wrote a record with no digest (disabling the engine's
        // provenance refusal on every game this server owns) and rewrote a
        // CLI-written save's digest-for-state-N onto a state-N+1 record, which
        // the next cold rebuild refused forever.
        let json = repl.session.to_record_json()?;
        // The swap itself. A CONFLICT here means the store moved between the
        // compare above and this line — a window the guards cannot close from
        // outside — and it is a conflict in the same sense as the three
        // refusals above, so it evicts the handle. An I/O failure does NOT:
        // there the handle is right and the disk is not, which is the
        // distinction `conflict`'s doc draws and `WriteError` now carries in
        // its type.
        //
        // **NO CELL DRIVES THE CONFLICT ARM**, and saying so is worth the line.
        // Within one process the guards above have already read the store under
        // the same `written` lock, so the only way to reach it is a SECOND
        // PROCESS writing between that read and this write — which no test here
        // can arrange. It is defence in depth against a window the old code
        // could not see at all, not a covered invariant.
        let version = match self.store.write_if_version(Doc::Record, id, &json, expected) {
            Ok(v) => v,
            Err(WriteError::Conflict { .. }) => {
                return Err(self.conflict(
                    id,
                    format!("{id:?} changed under this handle between the compare and the write"),
                ))
            }
            Err(WriteError::Io(e)) => return Err(e),
        };
        written.insert(id.to_string(), version);
        drop(written);
        self.mirror_turn(id, repl);
        // **After the mirror, not before it** (spec §2): the lobby's
        // whose-turn and over lines are read off the mirror, so a bump sent
        // between the record write and the refresh wakes a lobby that then
        // reads the previous turn and hears nothing more.
        self.signals.bump(id);
        Ok(())
    }

    /// Refresh the list's two mirrored integers (R-plan-3).
    ///
    /// **Every failure here is swallowed, and that is the design.** The record
    /// is the truth and this is a cache of `to_move` and `over` for the lobby;
    /// nothing on a play route reads it. A conflict means another writer
    /// refreshed the same document — from a keep, or from its own move — and
    /// losing that race costs a list row that says whose turn it was one move
    /// ago, until the next persist. Failing the MOVE for it would trade a
    /// durable game for a stale label.
    fn mirror_turn(&self, id: &str, repl: &Repl) {
        let Ok(Some((mut doc, version))) = self.read_table(id) else { return };
        let turn = mirror_of(repl);
        if doc.turn == Some(turn) {
            return;
        }
        doc.turn = Some(turn);
        let _ = self.write_table(&doc, Some(version));
    }

    /// Refuse a write AND drop the handle that attempted it.
    ///
    /// **A refused write means the handle is wrong, so keeping it resident is
    /// what turns one lost move into a lost session.** `live` is never
    /// invalidated against the store, so without this the same stale `Repl`
    /// answers every later request: each move applies to it, each returns 200
    /// with a `warnings` line, and each write is refused — the player goes on
    /// playing a game that ended at the last write anyone accepted. Evicting
    /// sends the next request through `Session::from_record`, which is the
    /// durable truth.
    ///
    /// Only CONFLICTS come through here — deleted, superseded, or a different
    /// game under the id. An I/O failure does NOT evict: there the handle is
    /// right and the disk is not, and `input`'s 200-plus-`warnings` answer for
    /// that case is a measured decision (see its comment) that this leaves
    /// alone.
    fn conflict(&self, id: &str, why: String) -> String {
        self.live.lock().expect("games lock").remove(id);
        why
    }

    /// Drop the resident handle for `id`, so the next request takes the COLD
    /// path through the store. Tests only: nothing in the API evicts by name.
    #[cfg(test)]
    pub fn evict_for_test(&self, id: &str) {
        self.live.lock().expect("games lock").remove(id);
    }
}

/// Drop resident handles until the map is within [`MAX_RESIDENT`], never
/// dropping `keep`.
///
/// Eviction order is arbitrary rather than least-recently-used: an LRU needs a
/// clock or a counter on every read, and the property that matters here is only
/// that the map is BOUNDED. A handle is a cache; the record is the truth, and
/// an evicted game rebuilds on its next request through the same path a slept
/// container takes.
pub(crate) fn evict_down_to(live: &mut HashMap<GameId, Arc<Mutex<Repl>>>, keep: &str) {
    while live.len() > MAX_RESIDENT {
        let victim = live.keys().find(|k| k.as_str() != keep).cloned();
        match victim {
            Some(k) => {
                live.remove(&k);
            }
            None => break,
        }
    }
}

/// Play any bot seats that are owed a move.
///
/// `Repl::step` pumps for itself after a successful apply, so this is only
/// needed where a game BEGINS to exist — a fresh game whose start seat is a
/// bot, and a stored game rebuilt with a bot to move. Without it, either would
/// sit forever waiting for a move nothing was going to make: the CLI has the
/// same two sites, and calls the same pump at both.
pub fn bring_alive(repl: &mut Repl) -> String {
    repl.pump_bots()
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use crate::store::MemStore;
    use crate::tables::WireSeed;
    use yokohama_core::PlayerId;

    /// **The mirror follows the game, because a list that lies about whose turn
    /// it is is worse than one that says nothing.**
    ///
    /// `persist` refreshes it after every applied move. The cell walks red's
    /// opening turn to its end — answering the LAST option each time, which is
    /// the one that ends a phase — and requires the mirror to have moved with
    /// it, so a `persist` that wrote only the record fails here rather than at a
    /// lobby nobody tests.
    #[test]
    fn the_turn_mirror_follows_every_persisted_move() {
        let games = games();
        let red = who("red@t.test");
        let handle = playing(&games, &two_people("mirror", CORPUS_SEED), &["red@t.test", "blue@t.test"]);
        let before = games.table("mirror", &red).unwrap().turn.unwrap();
        {
            let mut r = handle.lock().unwrap();
            // Walk red's turn to its end with the sole options the opening offers.
            for _ in 0..40 {
                if r.session.state().current_player != PlayerId::new(0) {
                    break;
                }
                let rows = r.option_rows();
                let n = yokohama_cli::screen_model::menu_number(rows.len() - 1);
                r.step(&n.to_string());
                games.persist("mirror", &r).unwrap();
            }
        }
        let after = games.table("mirror", &red).unwrap().turn.unwrap();
        assert_ne!(before, after, "the mirror moved with the turn");
        assert_eq!(after.to_move, 1);
    }

    /// A record this server writes carries the terminal digest, so the
    /// engine's own provenance refusal is ARMED on a web game exactly as it is
    /// on a CLI save.
    ///
    /// `Session::from_record` guards on `if let Some(expected_digest)`
    /// (`session.rs`), so a record written without one disables the check
    /// silently rather than failing it. Its doc states what is lost: editing
    /// `seed` alone in an otherwise clean save "replays with zero errors,
    /// passes `check_invariants` ... and resumes the game on tiles that are not
    /// the ones on the table. The digest is the only field that can see that."
    ///
    /// Asserting the FIELD would be a weaker test than it looks — a digest
    /// stamped from the wrong state is still `Some`. So this tampers with the
    /// seed and requires the refusal to fire.
    #[test]
    fn a_persisted_record_arms_the_provenance_refusal_against_an_edited_seed() {
        let store = Arc::new(MemStore::default());
        let games =
            Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let id = "digest".to_string();
        let _handle = playing(&games, &req("digest"), &["red@t.test"]);

        let stored = stored_json(&store, &id);
        let record: GameRecord = serde_json::from_str(&stored).expect("a record");
        assert!(
            record.terminal_digest.is_some(),
            "a web-written record carries a digest, or the refusal below can never run",
        );

        // Edit the seed the way the refusal exists to catch, and re-read it
        // through the same path a slept container takes.
        let mut tampered: serde_json::Value =
            serde_json::from_str(&stored).expect("the record as json");
        let seed = tampered["seed"].as_u64().expect("a seed");
        tampered["seed"] = serde_json::Value::from(seed ^ 1);
        overwrite_record(&store, &id, &tampered.to_string());

        let fresh =
            Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        // `Repl` is not `Debug`, so match rather than `expect_err`.
        let err = match fresh.handle(&id) {
            Ok(_) => panic!("an edited record must be refused, not loaded"),
            Err(e) => e.to_string(),
        };
        assert!(
            err.contains("digest"),
            "the refusal names the digest rather than some other failure: {err}",
        );
    }

    /// A save that CARRIES a digest survives a move made through this server.
    ///
    /// The destructive half of the same defect, and the one that needs a COLD
    /// rebuild to reproduce: `from_record` keeps the parsed record
    /// (`session.rs`), so a game loaded from a stamped save holds
    /// `terminal_digest: Some(digest of state N)` in memory. A `persist` that
    /// re-serialized that record wrote N+1 events still claiming state N, and
    /// the next cold rebuild refused it — a permanent 404 on an intact record
    /// the server itself had just written.
    ///
    /// Every CLI writer stamps (interactive `save`, autosave, `import --out`,
    /// `--follow --out`), so "a save that carries a digest" is every save a
    /// tester would drop into this store.
    #[test]
    fn a_save_that_carries_a_digest_survives_a_move_made_through_this_server() {
        let store = Arc::new(MemStore::default());
        let rules = Arc::new(Rules::standard());
        let games = Games::new(Arc::clone(&rules), Arc::clone(&store) as Arc<dyn Store>);
        let id = "fromcli".to_string();
        let _handle = playing(&games, &req("fromcli"), &["red@t.test"]);

        // COLD rebuild, so the resident session's record is the STAMPED one
        // off the store rather than the `None`-digest record `create` built.
        let cold = Games::new(Arc::clone(&rules), Arc::clone(&store) as Arc<dyn Store>);
        let handle = cold.handle(&id).expect("the stored game loads");
        {
            let mut repl = handle.lock().expect("the handle");
            assert!(
                repl.session.record().terminal_digest.is_some(),
                "the premise: a rebuilt session carries the digest it was loaded with",
            );
            let before = repl.session.record().events.len();
            let _ = repl.step("");
            assert!(
                repl.session.record().events.len() > before,
                "the premise: the step applied a move, so the digest is now stale",
            );
            cold.persist(&id, &repl).expect("the write");
        }

        let fresh = Games::new(Arc::clone(&rules), Arc::clone(&store) as Arc<dyn Store>);
        assert!(
            fresh.handle(&id).is_ok(),
            "the record this server just wrote must still load",
        );
    }

    /// A name is not a game: an in-flight write cannot land on a record that
    /// is a DIFFERENT game under the same id.
    ///
    /// The delete tombstone defeats itself, and its own doc names the bound
    /// that breaks: "It is bounded by `create`, which clears the id." `create`
    /// clears it on ATTEMPT, before the new record exists. So:
    /// (1) an `input` takes the lock and runs `Repl::step` (0.16-0.30 s);
    /// (2) `DELETE` removes the file and tombstones the id;
    /// (3) `create` reuses the name, CLEARING the tombstone, and persists a new
    ///     game with fewer events;
    /// (4) the in-flight write finds no tombstone and passes the monotonicity
    ///     check, so the DELETED game overwrites the new one.
    ///
    /// Two guards, each with its own test, and their interaction is the hole.
    /// The fix is to compare identity rather than a name plus a length.
    #[test]
    fn an_in_flight_write_cannot_land_on_a_different_game_that_took_the_name() {
        let store = Arc::new(MemStore::default());
        let rules = Arc::new(Rules::standard());
        let games = Games::new(Arc::clone(&rules), Arc::clone(&store) as Arc<dyn Store>);

        let id = "reused".to_string();
        let handle = playing(&games, &req("reused"), &["red@t.test"]);
        let repl = handle.lock().expect("the handle");
        let doomed_seed = repl.session.record().seed;

        games.delete(&id, &who("red@t.test")).expect("a delete");

        // The name comes back as a DIFFERENT game — a different seed is what
        // makes it a different board, deal and box.
        let mut second = req("reused");
        second.seed = Some(WireSeed(20_260_901));
        playing(&games, &second, &["red@t.test"]);
        let after_create = stored_json(&store, &id);

        let err = match games.persist(&id, &repl) {
            Ok(()) => panic!("the in-flight write landed on someone else's game"),
            Err(e) => e,
        };
        assert!(
            err.contains("different game"),
            "the refusal says the id holds another game: {err}",
        );
        // **AND IT SAYS IT WITHOUT SPELLING EITHER SEED.**
        //
        // This sentence is not confined to a log: `input` pushes it into
        // `warnings` as `APPLIED BUT NOT SAVED — {e}` (`lib.rs`) and serializes
        // it on the reply, so a sentence carrying a seed is a seed on the wire —
        // and it would be the NEW table's seed handed to the OLD game's player,
        // which regenerates that table's whole hidden set. The corpus-wide value
        // pin cannot see this one: no committed payload reaches this arm.
        //
        // Both spellings, and both games' seeds, because the fence is about the
        // number and not about which side it came from.
        for seed in [doomed_seed, 20_260_901u64] {
            for spelling in [seed.to_string(), format!("{seed:016x}"), format!("{seed:x}")] {
                assert!(
                    !err.contains(&spelling),
                    "the refusal spells a setup seed ({spelling}) on a sentence that reaches a \
                     client's `warnings`: {err}",
                );
            }
        }

        let now: GameRecord =
            serde_json::from_str(&stored_json(&store, &id)).expect("a record");
        assert_ne!(now.seed, doomed_seed, "the deleted game did not come back");
        assert_eq!(
            stored_json(&store, &id),
            after_create,
            "the stored record is byte-identical to what `create` wrote",
        );
    }

    /// A handle whose write was REFUSED is dropped, so the next request
    /// rebuilds from the store instead of playing on a game nobody can save.
    ///
    /// Without this, a refusal is permanent and silent: `live` caches the
    /// handle forever, every later move applies to it, each answers 200 with a
    /// `warnings` string, and `persist` refuses every one of them. The player
    /// keeps playing a game that ended at the last successful write. Known
    /// defect (a) — the undo hole — loses ONE move this way; this loses the
    /// rest of the session.
    ///
    /// A refusal is a CONFLICT (deleted, superseded, or a different game),
    /// never an I/O failure — an unwritable store means the handle is right and
    /// the disk is not, and `input`'s documented 200-plus-warning behaviour for
    /// that case is deliberate and unchanged.
    #[test]
    fn a_handle_whose_write_was_refused_is_not_left_resident() {
        let store = Arc::new(MemStore::default());
        let rules = Arc::new(Rules::standard());
        let games = Games::new(Arc::clone(&rules), Arc::clone(&store) as Arc<dyn Store>);
        let id = "stale".to_string();
        let handle = playing(&games, &req("stale"), &["red@t.test"]);

        // Someone else moves the stored game past this handle.
        {
            let other = Games::new(Arc::clone(&rules), Arc::clone(&store) as Arc<dyn Store>);
            let h = other.handle(&id).expect("the same game");
            let mut r = h.lock().expect("the handle");
            let _ = r.step("");
            other.persist(&id, &r).expect("the other writer's move");
        }

        let repl = handle.lock().expect("the stale handle");
        assert!(games.persist(&id, &repl).is_err(), "the backwards write is refused");
        drop(repl);

        // The refused handle must be gone, so the next request replays the
        // store rather than continuing the divergent line.
        let rebuilt = games.handle(&id).expect("a rebuild");
        assert!(
            !Arc::ptr_eq(&rebuilt, &handle),
            "the refused handle was evicted, so this is a fresh one off the store",
        );
    }

    /// A game DELETED while a move was in flight does not come back.
    ///
    /// The window is the whole of `Repl::step`, bot pump included — measured at
    /// 0.16-0.30 s in a debug build, so this is an ordinary click-then-delete,
    /// not a microsecond. Without the tombstone the in-flight `persist`
    /// re-created the record (and `FsStore::write` re-creates the parent
    /// directory too), so `DELETE` answered 204 and the game reappeared in
    /// `list`.
    #[test]
    fn a_delete_is_not_undone_by_a_move_already_in_flight() {
        let games = games();
        let id = "drace".to_string();
        let handle = playing(&games, &req("drace"), &["red@t.test"]);
        let repl = handle.lock().expect("the handle");
        let red = who("red@t.test");

        games.delete(&id, &red).expect("a delete");
        assert!(games.list_for(&red).expect("a list").is_empty(), "the delete took");

        let err = games.persist(&id, &repl).expect_err("the in-flight write is refused");
        assert!(err.contains("deleted"), "the refusal says why: {err}");
        assert!(games.list_for(&red).expect("a list").is_empty(), "and the game stays gone");

        // The name is reusable: the tombstone holds ids that are deleted and
        // NOT recreated, so this must succeed rather than trip the guard.
        assert!(games.create(&req("drace"), &red).is_ok(), "the name can be reused");
        assert_eq!(
            games.list_for(&red).expect("a list").iter().map(|t| t.id.clone()).collect::<Vec<_>>(),
            vec!["drace".to_string()],
        );
    }

    /// Replace `id`'s record the way a SECOND WRITER does: read what is there,
    /// then write against that version.
    ///
    /// Not an unconditional write, because there is no longer such a thing —
    /// and this is the shape the cells below want anyway, since every one of
    /// them is staging another writer who has seen the current record.
    fn overwrite_record(store: &MemStore, id: &str, json: &str) {
        let version = store.read(Doc::Record, id).expect("a readable store").map(|(_, v)| v);
        store.write_if_version(Doc::Record, id, json, version).expect("another writer's write");
    }

    /// How many events the STORE holds for `id`.
    fn stored_events(store: &MemStore, id: &str) -> usize {
        let (json, _) =
            store.read(Doc::Record, id).expect("a stored record").expect("a stored record");
        serde_json::from_str::<GameRecord>(&json).expect("a record").events.len()
    }

    /// **An undo reaches the store.**
    ///
    /// `Session::undo` POPS events — a record is a seed plus an event log, so
    /// undoing shortens the log. `persist`'s guard used to ask only "does the
    /// stored record hold more events than mine?", which is true of BOTH a
    /// stale handle about to erase another writer's move (the thing to refuse)
    /// and a live handle that just shortened its own log (an undo). It could
    /// not tell them apart, so it refused both: the client was told
    /// "Undid last entry.", the write was refused as a `warnings` line, and the
    /// next request rebuilt from the store with the undone move back.
    ///
    /// The guard now asks the right question — **is the store still what THIS
    /// handle last left there?** — so an undo is ours to write and a stale
    /// handle's write is not.
    #[test]
    fn an_undo_reaches_the_store() {
        let store = Arc::new(MemStore::default());
        let games =
            Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let id = "undo".to_string();
        let handle = playing(&games, &req("undo"), &["red@t.test"]);

        {
            let mut repl = handle.lock().expect("the handle");
            // Drive the way the client does: a numbered option, then `confirm`
            // when that option opened a picker. `step("1")` alone stalls inside
            // a placement sub-prompt, and `step("")` only applies a FORCED sole
            // option — both leave the record at one event, which is what the
            // premise assertion below exists to catch.
            for _ in 0..40 {
                let before = repl.session.record().events.len();
                repl.step("1");
                if repl.session.record().events.len() == before {
                    repl.step("confirm");
                }
                games.persist(&id, &repl).expect("a persist");
                if repl.session.record().events.len() >= 4 {
                    break;
                }
            }
        }
        let before = stored_events(&store, &id);
        assert!(before >= 4, "the premise: there is something to undo ({before} events)");

        {
            let mut repl = handle.lock().expect("the handle");
            repl.step("undo");
            assert!(
                repl.session.record().events.len() < before,
                "the premise: `undo` shortened the handle's own log",
            );
            games.persist(&id, &repl).expect("the undo is written through");
        }

        let after = stored_events(&store, &id);
        assert!(after < before, "the store shortened too: {before} -> {after}");

        // And it SURVIVES a cold rebuild, which is the whole point — the record
        // is the truth, and a rebuild replays it.
        let fresh =
            Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let rebuilt = fresh.handle(&id).expect("the stored game loads");
        assert_eq!(
            rebuilt.lock().expect("the handle").session.record().events.len(),
            after,
            "the rebuilt game is the undone one, not the one before the undo",
        );
    }

    /// **A divergent write of the SAME LENGTH is refused — which the length
    /// guard could not see at all.**
    ///
    /// This is what makes the new question strictly stronger rather than merely
    /// more permissive. `on_disk.events.len() > events` is false when the counts
    /// match, however different the contents, so a length comparison waved this
    /// straight through and one writer silently overwrote the other. Comparing
    /// against what this process last LEFT there catches it, because someone
    /// else's record of the same length is still not our record.
    #[test]
    fn a_divergent_write_of_the_same_length_is_refused() {
        let store = Arc::new(MemStore::default());
        let games =
            Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let id = "fork".to_string();
        let handle = playing(&games, &req("fork"), &["red@t.test"]);
        {
            let mut repl = handle.lock().expect("the handle");
            repl.step("1");
            games.persist(&id, &repl).expect("a persist");
        }
        let mine = stored_events(&store, &id);

        // Another writer replaces the record with a DIFFERENT one of the same
        // length — the shape a length comparison is blind to. Built by hand for
        // the same reason the stale-handle cell builds its ahead-record: the
        // divergence has to be exact, not incidental.
        let mut theirs: GameRecord =
            serde_json::from_str(&stored_json(&store, &id)).expect("a record");
        let last = theirs.events.pop().expect("at least one event");
        theirs.events.push(last.clone());
        theirs.events.push(last);
        theirs.events.pop();
        theirs.lockdown = Some("a different writer was here".to_string());
        let theirs_json = serde_json::to_string(&theirs).expect("json");
        overwrite_record(&store, &id, &theirs_json);
        assert_eq!(stored_events(&store, &id), mine, "the premise: the SAME length");

        let repl = handle.lock().expect("the handle");
        assert_eq!(
            repl.session.record().events.len(),
            mine,
            "the premise: this handle is not behind by any COUNT",
        );
        let err = match games.persist(&id, &repl) {
            Ok(()) => panic!("a divergent write of equal length overwrote another writer"),
            Err(e) => e,
        };
        assert!(err.contains("changed"), "the refusal names the conflict: {err}");
        assert_eq!(
            stored_json(&store, &id),
            theirs_json,
            "their record survived byte for byte",
        );
    }

    /// A handle that has fallen behind the store cannot write the store
    /// backwards.
    ///
    /// `live` caches a handle forever and nothing invalidates it against the
    /// store, so with two processes over one store — the arrangement
    /// [`Store`] exists for — a stale handle would publish a record with
    /// FEWER events, erasing a move another writer had already applied and
    /// acknowledged.
    #[test]
    fn a_stale_handle_cannot_erase_a_move_another_writer_already_stored() {
        let store = Arc::new(MemStore::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let id = "stale".to_string();
        let handle = playing(&games, &req("stale"), &["red@t.test"]);

        // One real move, so the handle and the store agree on a non-empty
        // record before anything diverges.
        {
            let mut repl = handle.lock().expect("the handle");
            repl.step("1");
            games.persist(&id, &repl).expect("a persist");
        }

        // Now the store advances behind this handle's back — which is exactly
        // what a second process doing one move looks like from here.
        let mut record: GameRecord =
            serde_json::from_str(&stored_json(&store, &id)).expect("a record");
        let ahead = record.events.last().cloned().expect("the move is recorded");
        record.events.push(ahead);
        overwrite_record(&store, &id, &serde_json::to_string(&record).expect("json"));
        let ahead_len = record.events.len();

        let repl = handle.lock().expect("the handle");
        assert!(repl.session.record().events.len() < ahead_len, "the premise: this handle is behind");

        let err = games.persist(&id, &repl).expect_err("a backwards write is refused");
        // The refusal's WORDING moved when the guard became a compare-and-swap
        // — this handle is now refused for having been overtaken rather than
        // for being shorter — so assert the property and accept either reason.
        assert!(
            err.contains("advanced past") || err.contains("changed under"),
            "the refusal says why: {err}",
        );

        let after: GameRecord =
            serde_json::from_str(&stored_json(&store, &id)).expect("a record");
        assert_eq!(after.events.len(), ahead_len, "the newer record survived");
    }

    /// **A store that cannot be READ refuses the write and KEEPS the handle.**
    ///
    /// The two ways a write can fail are not one failure, and `persist` acts on
    /// the difference: a CONFLICT means this handle is wrong about the world,
    /// so it is dropped and the next request rebuilds from the record; an I/O
    /// failure means the handle is right and the disk is not, so it stays. That
    /// rule is stated in `conflict`'s own doc and carried by `WriteError`.
    ///
    /// `persist` used to DISCARD its read error (`.ok().flatten()`), and the
    /// discard defeated the rule from the inside: an unreadable store became
    /// `expected: None`, which is a CREATE, and a create over a present
    /// document is a `Conflict` — so a transient read failure evicted the live
    /// session, which is precisely what the split exists to prevent.
    ///
    /// Residency is asserted THROUGH `handle`, which is the only honest way to
    /// ask: it returns the resident `Arc` if there is one and otherwise rebuilds
    /// off the store — and the store is still failing, so a rebuild cannot
    /// succeed. `Arc::ptr_eq` against the handle `create` returned is therefore
    /// both halves of the claim at once.
    #[test]
    fn a_read_failure_refuses_the_write_and_leaves_the_handle_resident() {
        use std::sync::atomic::{AtomicBool, Ordering};

        /// A `MemStore` whose reads can be switched off under a live handle.
        #[derive(Default)]
        struct FlakyReads {
            inner: MemStore,
            failing: AtomicBool,
        }
        impl Store for FlakyReads {
            fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
                self.inner.list(doc)
            }
            fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
                if self.failing.load(Ordering::Relaxed) {
                    return Err("the disk went away".to_string());
                }
                self.inner.read(doc, id)
            }
            fn write_if_version(
                &self,
                doc: Doc,
                id: &str,
                json: &str,
                expected: Option<Version>,
            ) -> Result<Version, WriteError> {
                self.inner.write_if_version(doc, id, json, expected)
            }
            fn delete(&self, id: &str) -> Result<(), String> {
                self.inner.delete(id)
            }
        }

        let store = Arc::new(FlakyReads::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let id = "ioflake".to_string();
        let handle = playing(&games, &req("ioflake"), &["red@t.test"]);

        store.failing.store(true, Ordering::Relaxed);
        let err = {
            let repl = handle.lock().expect("the handle");
            games.persist(&id, &repl).expect_err("a store that cannot be read refuses the write")
        };
        assert!(
            err.contains("the disk went away"),
            "the refusal carries the store's own reason rather than inventing a conflict: {err}",
        );

        let again = games.handle(&id).expect("the handle is still resident");
        assert!(
            Arc::ptr_eq(&again, &handle),
            "an I/O failure must not evict — and a rebuild is impossible here, because the \
             store is still refusing to be read",
        );
    }

    /// **A move that lands bumps the table AND the house** (spec §2).
    ///
    /// The house too, and not only the table, because the lobby's whose-turn
    /// and over lines are read off the table document's turn mirror — which is
    /// exactly what `persist` refreshes on its way out, so a move is a change
    /// the house has to hear about.
    ///
    /// The receiver is taken BEFORE the move, which is the whole point of a
    /// `watch` sender that exists before anyone has written to it: a subscriber
    /// of a table that has never been bumped must still wake on its first bump.
    /// And the wake is not asserted on its own — reading the store after it
    /// shows the move, which is what "the bump follows the write" means for a
    /// caller.
    #[test]
    fn a_persisted_move_bumps_the_table_and_the_house() {
        let store = Arc::new(MemStore::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let id = "bumped".to_string();
        let handle = playing(&games, &req("bumped"), &["red@t.test"]);

        let mut rx = games.signals().subscribe(&id);
        let mut house = games.signals().subscribe_house();
        rx.borrow_and_update();
        house.borrow_and_update();
        let before = stored_events(&store, &id);

        {
            let mut repl = handle.lock().expect("the handle");
            repl.step("1");
            games.persist(&id, &repl).expect("a persist");
        }

        assert!(rx.has_changed().expect("the table channel is open"), "the move woke the table");
        assert!(!rx.borrow_and_update().end, "a move is not the end of the table");
        assert!(house.has_changed().expect("the house channel is open"), "and the house");
        assert!(
            stored_events(&store, &id) > before,
            "the state a woken subscriber reads is the state AFTER the move",
        );
    }

    /// **A write that is REFUSED bumps nothing.** A bump means "go and read
    /// the new state", so sending one for a write that never landed sends
    /// every subscriber to fetch the state it already has — and, worse, says a
    /// move happened at a table where the store rejected it.
    ///
    /// Driven through the tombstone, because it is the one refusal a cell can
    /// arrange without a second process: `persist` returns before it reaches
    /// the store at all, which is the earliest of the three refusal arms and
    /// the one a bump placed at the TOP of the function would sail past.
    #[test]
    fn a_refused_write_bumps_nothing() {
        let games = games();
        let id = "refused".to_string();
        let handle = playing(&games, &req("refused"), &["red@t.test"]);
        let red = who("red@t.test");
        let repl = handle.lock().expect("the handle");

        games.delete(&id, &red).expect("a delete");
        // `delete` itself ENDS the table's channel, so the subscription for
        // this assertion is taken after it — a fresh sender that has seen
        // nothing, exactly as `signal.rs`'s own end cell describes.
        let rx = games.signals().subscribe(&id);
        let house = games.signals().subscribe_house();

        games.persist(&id, &repl).expect_err("the in-flight write is refused");

        assert!(!rx.has_changed().expect("the table channel is open"), "no write, no bump");
        assert!(!house.has_changed().expect("the house channel is open"));
    }

    /// **A write the STORE refuses bumps nothing either** — the last inch, and
    /// the one the tombstone cell above cannot reach.
    ///
    /// `a_refused_write_bumps_nothing` drives the FIRST of `persist`'s refusal
    /// arms, which returns before the store is touched at all; this drives the
    /// store itself, so the only thing between the bump and the write is the
    /// write. Measured: a bump moved to the line immediately above
    /// `write_if_version` passes every other cell in this crate, this one
    /// included until it existed.
    ///
    /// An I/O failure and not a conflict, because a conflict EVICTS the handle
    /// and this cell wants the ordinary refusal path — and because `persist`'s
    /// own doc says no cell here can drive the conflict arm.
    #[test]
    fn a_write_the_store_refuses_bumps_nothing() {
        use std::sync::atomic::{AtomicBool, Ordering};

        #[derive(Default)]
        struct FailingWrites {
            inner: MemStore,
            failing: AtomicBool,
        }
        impl Store for FailingWrites {
            fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
                self.inner.list(doc)
            }
            fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
                self.inner.read(doc, id)
            }
            fn write_if_version(
                &self,
                doc: Doc,
                id: &str,
                json: &str,
                expected: Option<Version>,
            ) -> Result<Version, WriteError> {
                if doc == Doc::Record && self.failing.load(Ordering::SeqCst) {
                    return Err(WriteError::Io("the disk went away".to_string()));
                }
                self.inner.write_if_version(doc, id, json, expected)
            }
            fn delete(&self, id: &str) -> Result<(), String> {
                self.inner.delete(id)
            }
        }

        let store = Arc::new(FailingWrites::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let id = "wfail".to_string();
        let handle = playing(&games, &req("wfail"), &["red@t.test"]);

        let rx = games.signals().subscribe(&id);
        let house = games.signals().subscribe_house();
        store.failing.store(true, Ordering::SeqCst);

        let mut repl = handle.lock().expect("the handle");
        repl.step("1");
        let err = games.persist(&id, &repl).expect_err("the store refused the write");
        assert!(err.contains("the disk went away"), "the store's own reason: {err}");

        assert!(!rx.has_changed().expect("the table channel is open"), "no write, no bump");
        assert!(!house.has_changed().expect("the house channel is open"));
    }
}

/// The helpers both halves of this crate's game tests share.
///
/// One copy rather than two, because a fixture that drifts between the table
/// tests and the record tests is a fixture that stops comparing them.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{Arc, Mutex};

    use yokohama_cli::repl::Repl;
    use yokohama_core::{PlayerColor, Rules};

    use super::Games;
    use crate::identity::User;
    use crate::store::{Doc, MemStore, Store};
    use crate::tables::{NewGame, SeatSpec, WireSeed};

    pub(crate) fn games() -> Games {
        Games::new(Arc::new(Rules::standard()), Arc::new(MemStore::default()))
    }

    /// The corpus seed. **At this seed seat 1's argmax is card 1** (57 against
    /// 31), which is the same card the old `keeps: [1, 1]` wire sent — so a
    /// cell that only runs here cannot tell the bot's own Keep from a constant.
    /// [`ARGMAX_IS_CARD_2`] is what discriminates; this seed stays in use for
    /// the cells whose subject is not WHICH card.
    pub(crate) const CORPUS_SEED: u64 = 20_260_831;

    /// A seed where seat 1's argmax is **card 2** (9 against 36), so the bot's
    /// Keep differs from every constant the old wire could have sent. Each cell
    /// using it asserts that as a PREMISE rather than trusting this comment: a
    /// deal that changed would otherwise turn these cells vacuous silently.
    pub(crate) const ARGMAX_IS_CARD_2: u64 = 20_260_830;

    /// A normalized identity, the way the extractor produces one.
    pub(crate) fn who(s: &str) -> User {
        User::parse(s).expect("an email")
    }

    /// The cycle's standing request: red at seat 0, the bot at seat 1.
    ///
    /// **The argument is the ID since M6.2**, not the name: `name` became a free
    /// -text LABEL (R-2.4) and the storage key moved to its own field, so a cell
    /// that opens a table and then reads it back by the string it passed keeps
    /// working and says which of the two it means.
    pub(crate) fn req_seeded(id: &str, seed: u64) -> NewGame {
        NewGame {
            players: 2,
            colors: vec![PlayerColor::Red, PlayerColor::Blue],
            start_seat: 0,
            seed: Some(WireSeed(seed)),
            seats: vec![
                SeatSpec { user: Some("red@t.test".into()), bot: false, open: false },
                SeatSpec { user: None, bot: true, open: false },
            ],
            name: None,
            id: Some(id.to_string()),
            display_name: None,
            password: None,
        }
    }

    pub(crate) fn req(id: &str) -> NewGame {
        req_seeded(id, CORPUS_SEED)
    }

    /// The same table with a PERSON in seat 1 — the shape every cell about
    /// per-seat scoping needs, since a bot seat never calls anything.
    pub(crate) fn two_people(id: &str, seed: u64) -> NewGame {
        let mut r = req_seeded(id, seed);
        r.seats[1] = SeatSpec { user: Some("blue@t.test".into()), bot: false, open: false };
        r
    }

    /// create -> keep for every human -> the playing handle, as the API does it.
    ///
    /// A game is no longer one call: the cells below that are about PERSISTENCE
    /// want the handle `create` used to hand back, and this is the sequence that
    /// now produces one. Each user keeps for every seat it holds, lowest first,
    /// which is exactly what a hot seat does through the browser.
    pub(crate) fn playing(games: &Games, req: &NewGame, users: &[&str]) -> Arc<Mutex<Repl>> {
        let creator = who(users[0]);
        let id = games.create(req, &creator).expect("a table").id;
        for u in users {
            let user = who(u);
            while let Some(seat) =
                games.table(&id, &user).expect("seated").next_unkept_seat_of(&user)
            {
                games.keep(&id, &user, seat, 1, None).expect("a keep");
            }
        }
        games.handle(&id).expect("playing")
    }

    /// A [`MemStore`] that COUNTS its writes.
    ///
    /// Delegation rather than a wrapper type with a `Deref`, because the thing
    /// being counted is one trait method and the others must stay exactly what
    /// `MemStore` does. A cell that asks "how many writes did this take" cannot
    /// read the answer off the stored document — both a one-write and a
    /// two-write implementation leave the same bytes.
    #[derive(Default)]
    pub(crate) struct CountingStore {
        inner: MemStore,
        writes: std::sync::atomic::AtomicUsize,
    }

    impl CountingStore {
        pub(crate) fn writes(&self) -> usize {
            self.writes.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    impl Store for CountingStore {
        fn list(&self, doc: Doc) -> Result<Vec<crate::store::GameId>, String> {
            self.inner.list(doc)
        }
        fn read(
            &self,
            doc: Doc,
            id: &str,
        ) -> Result<Option<(String, crate::store::Version)>, String> {
            self.inner.read(doc, id)
        }
        fn write_if_version(
            &self,
            doc: Doc,
            id: &str,
            json: &str,
            expected: Option<crate::store::Version>,
        ) -> Result<crate::store::Version, crate::store::WriteError> {
            self.writes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.inner.write_if_version(doc, id, json, expected)
        }
        fn delete(&self, id: &str) -> Result<(), String> {
            self.inner.delete(id)
        }
    }

    /// The record `id` is stored as, as bytes.
    ///
    /// `expect` twice, and the two are different questions: the store failed,
    /// or the store answered "not there". A cell calling this wants a record
    /// that is present, so either is a failed premise.
    pub(crate) fn stored_json(store: &MemStore, id: &str) -> String {
        store.read(Doc::Record, id).expect("a readable store").expect("a stored record").0
    }
}
