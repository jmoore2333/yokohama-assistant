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

use crate::archive::{Archive, NoArchive};
use crate::chat::{ChatConfig, ChatLimits};
use crate::signal::{NoSignal, Signal};
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
    /// A game NOT OVER whose recorded bot this build does not run (M6.5c2,
    /// R-6.5c2.10): the labels of the bot it was played against and of the one
    /// this build deals. Typed rather than a `Conflict` for `Dealing`'s reason
    /// — a client BRANCHES on it, drawing the reason rather than a banner — so
    /// the pair rides beside the sentence as `"foreign_bot"`.
    ForeignBot { played: String, runs: String },
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
            Self::ForeignBot { played, runs } => f.write_str(&crate::bots::cannot_continue(played, runs)),
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
        // The record's first bot seat (M6.5c2, R-6.5c2.10). Every bot a web
        // table deals is one contestant (`tables::contestants_of`), so the
        // first names them all.
        bot: crate::bots::recorded(repl.session.record()).into_iter().next().map(|(_, d)| d),
    }
}

/// How this crate reads a wall clock (M6.5a, R-6.5a.6).
///
/// A seam rather than a call, because `std::time::SystemTime::now` is a
/// PLATFORM call: on `wasm32-unknown-unknown` it does not fail to compile, it
/// panics at run time — so a table layer that reached for it directly would
/// build for the browser and then die stamping its first document. The host
/// supplies the reading instead — [`crate::table::now_unix`] today, whatever a
/// wasm host can read when there is one — and nothing below knows which it was
/// handed.
///
/// It is also what lets a cell pin a stamp by VALUE. Everything else this type
/// writes is a function of the seeds and the identities; a wall clock is not,
/// which is why the committed corpus has to normalise it away.
pub type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

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
    /// The change signal (M6.3, spec §2), bumped AFTER each of this type's
    /// writes lands — **a trait object since M6.5a** (R-6.5a.8).
    ///
    /// It lives here because every write site does, and only because of that:
    /// [`crate::signal`] knows nothing about a game, a table or a seat, and
    /// nothing in this module reads a bump back. It is a notification that
    /// something this process wrote has changed, for the streams in front of
    /// it to answer — so a failed or refused write bumps nothing, and every
    /// call below sits on a success path.
    ///
    /// The axum host passes [`crate::signal::Signals`] (one `watch` channel per
    /// table and one for the house) and keeps its own handle to subscribe on; a
    /// host with nothing to wake passes [`NoSignal`]. This type cannot tell
    /// them apart, and that is the point: what a write site owes is the CALL,
    /// and what becomes of it is the transport's.
    pub(crate) signal: Arc<dyn Signal>,
    /// The newest chat number per table this process has seen (M6.4, R-plan-5):
    /// seeded from the document on a miss, advanced by every append, dropped by
    /// a delete and by a create.
    ///
    /// It exists so the count a signal frame carries costs no document parse
    /// per bump per connection — a stream's per-bump closure reads it, and with
    /// twenty watchers on a table the alternative is twenty parses of the same
    /// file for one message.
    ///
    /// **Per PROCESS.** A second process's appends are invisible here until the
    /// store is a database (M6.5); the DOCUMENT is always the authority and
    /// every page is served from it.
    pub(crate) chat_last: Mutex<HashMap<GameId, u64>>,
    /// Serializes the house's appends against each other.
    ///
    /// Its own mutex because the house is not a table: it has no `lock_for`
    /// entry, and `table_lock` is the id space's, which the house does not
    /// enter. Two people talking in the lobby are otherwise a lost message —
    /// both read `last`, both write `last + 1`, and the compare-and-swap saves
    /// only whichever of them the store sees second.
    pub(crate) house_lock: Mutex<()>,
    /// The lobby switch and the retention numbers (R-6.4.6, R-6.4.8), from the
    /// launch variables — on the layer rather than on the host's state since
    /// M6.5a (R-6.5a.1), so the house switch and the two text checks read one
    /// set of facts on every host.
    pub chat: ChatConfig,
    /// The wall clock this type stamps documents from — see [`Clock`] for why
    /// it is a seam and not a call.
    ///
    /// **Two callers, both in [`crate::tables`]**: the `created_at` a `create`
    /// writes onto a table document, and the one an append writes onto a
    /// message. Nothing here reads a clock back, and nothing compares two
    /// readings, so a host is free to hand over a constant.
    pub(crate) clock: Clock,
    /// Where a finished or abandoned game's record and table document go
    /// (M6.5a, R-6.5a.9) — `NoArchive` for a host that keeps none, the
    /// directory on the NAS, M6.5c's bucket. Asked at the two sites of
    /// R-6.5a.11 and nowhere else.
    pub(crate) archive: Arc<dyn Archive>,
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
        Self::with_limits(rules, store, ChatLimits::default())
    }

    /// [`Games::new`] with the chat's retention numbers given rather than
    /// defaulted — how a test asks for a keep small enough to watch a trim
    /// happen. **Its signal is [`NoSignal`]**, so a cell that watches a bump
    /// calls [`Games::with_parts`] and keeps the handle it passed; **its clock
    /// ticks** — 1, 2, 3, one per reading, see [`Games::ticking_clock`] — so a
    /// cell that pins a stamp by VALUE reaches for [`Games::with_clock`].
    pub fn with_limits(rules: Arc<Rules>, store: Arc<dyn Store>, limits: ChatLimits) -> Self {
        Self::with_parts(
            rules,
            store,
            ChatConfig { lobby: true, limits },
            Arc::new(NoSignal),
            Self::ticking_clock(),
            Arc::new(NoArchive),
        )
    }

    /// A clock that reads 1, 2, 3, … — one tick per reading, no platform call.
    ///
    /// The default for a `Games` built without a host (R-6.5a.6): the wall clock
    /// is `server`-only and `AppState` passes it, and a constant would make
    /// every table's `created_at` a tie, turning each "newest first" cell into a
    /// claim about the id tie-break instead. Small integers, not unix seconds —
    /// a cell that wants a stamp that LOOKS like a time injects one through
    /// [`Games::with_clock`].
    pub(crate) fn ticking_clock() -> Clock {
        let ticks = Arc::new(std::sync::atomic::AtomicU64::new(0));
        Arc::new(move || ticks.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1)
    }

    /// The one constructor that builds anything: every part this type holds
    /// that a host may choose, given rather than defaulted.
    ///
    /// How `main` builds the layer (M6.5a, R-6.5a.8) — the signal is a trait
    /// object, so the host builds its own and passes a clone, keeping the
    /// other half to subscribe on, and the clock (R-6.5a.6) is the host's too.
    /// The two constructors above are this one with the chat facts, the signal
    /// or the clock chosen for them.
    pub fn with_parts(
        rules: Arc<Rules>,
        store: Arc<dyn Store>,
        chat: ChatConfig,
        signal: Arc<dyn Signal>,
        clock: Clock,
        archive: Arc<dyn Archive>,
    ) -> Self {
        Self {
            rules,
            store,
            live: Mutex::new(HashMap::new()),
            written: Mutex::new(HashMap::new()),
            table_lock: Mutex::new(()),
            locks: Mutex::new(HashMap::new()),
            deleted: Mutex::new(std::collections::HashSet::new()),
            signal,
            chat_last: Mutex::new(HashMap::new()),
            house_lock: Mutex::new(()),
            chat,
            clock,
            archive,
        }
    }

    /// This `Games` with a different [`Archive`] — the builder step a cell
    /// that watches a put reaches for, as [`Games::with_clock`] is for a stamp.
    pub fn with_archive(mut self, archive: Arc<dyn Archive>) -> Self {
        self.archive = archive;
        self
    }

    /// This `Games` with a different [`Clock`].
    ///
    /// A builder step rather than a constructor of its own: a cell that pins a
    /// stamp by VALUE wants the in-memory layer every other cell builds with
    /// one part of it changed, and spelling [`Games::with_parts`] out there
    /// would restate the store, the retention numbers and the signal in order
    /// to say something about none of them.
    pub fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    pub fn store(&self) -> &Arc<dyn Store> {
        &self.store
    }

    /// The rules this layer's games are built with — one `Rules::standard()` per
    /// process, held here since M6.5a (the host keeps no handle of its own).
    pub fn rules(&self) -> &Rules {
        &self.rules
    }

    /// A host's side channel INTO the store (M6.5b, R-6.5b.4): a document
    /// written with no version — a CREATE, refused if present — for a Worker
    /// filling a fresh store at boot or on a lock acquisition.
    ///
    /// It bypasses this type on purpose. The pair a browser holds is a state
    /// the player was already shown, so there is nothing for `Games` to decide
    /// about it; the `Repl` is rebuilt from the record on first touch through
    /// [`Games::handle`] exactly as an absent handle is. Versionless because
    /// the store it fills is fresh — the Worker `init()`s before it loads — so
    /// the `Store` trait needs no overwrite-without-a-version arm, and a load
    /// that DOES meet a present document is a bug in the caller and says so.
    pub fn load_document(&self, doc: Doc, id: &str, text: &str) -> Result<(), String> {
        self.store.write_if_version(doc, id, text, None).map(|_| ()).map_err(|e| e.to_string())
    }

    /// The side channel OUT: the store's exact bytes, for the host's own
    /// persistence and its file save. `None` is absent; an unreadable present
    /// document is the `Err` the `Store` trait distinguishes it as.
    pub fn read_document(&self, doc: Doc, id: &str) -> Result<Option<String>, String> {
        self.store.read(doc, id).map(|r| r.map(|(text, _)| text))
    }

    /// The chat's retention numbers, for the route that has to WORD the text
    /// cap in a refusal rather than restate it.
    pub fn limits(&self) -> ChatLimits {
        self.chat.limits
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
        // **The foreign-bot rule's one site** (M6.5c2, R-6.5c2.10), on every
        // host, for every rebuild. The descriptors are compared off the parsed
        // record; "not over" is a fact about the replayed state, so it is read
        // off the session — and the refusal returns BEFORE `Repl::new`, whose
        // `BotSeats::from_provenance` is what would turn the foreign seat into
        // a human one, and before `bring_alive`, which would let a foreign bot
        // this CLI can still run move again. No `Repl` is built, so a converted
        // seat never exists and nothing is persisted.
        let foreign = crate::bots::foreign(&record).into_iter().next();
        let session =
            Session::from_record(record, (*self.rules).clone()).map_err(GameError::Server)?;
        if let Some((_, played)) = &foreign {
            if !session.is_over() {
                return Err(GameError::ForeignBot {
                    played: crate::bots::label(played),
                    runs: crate::bots::label(&crate::bots::production()),
                });
            }
        }
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
        // The game-over archive site (M6.5a, R-6.5a.11), after the mirror and
        // before the bump: the bytes just written are what is archived.
        self.archive_if_over(id, repl, &json);
        // **After the mirror, not before it** (spec §2): the lobby's
        // whose-turn and over lines are read off the mirror, so a bump sent
        // between the record write and the refresh wakes a lobby that then
        // reads the previous turn and hears nothing more.
        self.signal.bump(id);
        Ok(())
    }

    /// R-6.5a.11's game-over site. **Best effort, and the move is answered 200
    /// regardless**: the record is already written, the store is the durable
    /// copy, and a failure here is logged and leaves `archived_at` UNSET —
    /// which is what says a retry is owed (the next abandon, or the operator;
    /// umbrella R-6.5.4). Runs once: a table already stamped is not put again,
    /// which is also what keeps a vs-bot game undone past its end and finished
    /// a second time at its FIRST completion in the archive (spec §10).
    ///
    /// Since M6.5c a host is a third retrier: [`Games::archive_over_retry`] runs
    /// this again once the host's deferred put has landed (R-6.5c.9).
    ///
    /// `record_json` is the text the store write above took, so the archived
    /// bytes ARE the stored bytes — never a re-serialization.
    fn archive_if_over(&self, id: &str, repl: &Repl, record_json: &str) {
        if !crate::archive::is_on(&self.archive) || !repl.session.is_over() {
            return;
        }
        let Ok(Some((mut doc, version))) = self.read_table(id) else { return };
        if doc.archived_at.is_some() {
            return;
        }
        doc.archived_at = Some((self.clock)());
        let table_json = crate::archive::strip_for_archive(&doc);
        // The server's OWN games carry the empty caller prefix (R-6.5b.15): the
        // `browser/<segment>/` tree is for games a browser sent.
        match self.archive.put("", crate::archive::Why::Over, id, record_json, &table_json) {
            // Both outcomes are a landed copy: `Identical` means an earlier put
            // of these very bytes landed and only its stamp did not (M6.5b,
            // spec §2.4), so the retry stamps rather than being refused.
            Ok(_) => {
                // The stamp is what keeps this game to ONE archive copy: the next
                // abandon puts under `abandoned/`, a DIFFERENT path, so a put that
                // landed without its stamp would be archived a second time there
                // rather than refused (`archive::paths` is per reason). Retried
                // once on a version conflict — an `unlock` or a `join` writing the
                // document in the same instant is the only other writer left at
                // game over — and every other outcome is logged, because a landed
                // copy with no stamp is the case an operator most needs to hear.
                match self.write_table(&doc, Some(version)) {
                    Ok(_) => {}
                    Err(WriteError::Conflict { .. }) => match self.read_table(id) {
                        Ok(Some((mut again, v2))) => {
                            again.archived_at = doc.archived_at;
                            if let Err(e) = self.write_table(&again, Some(v2)) {
                                eprintln!("archive of {id:?} at game over landed, but its stamp did not: {e}");
                            }
                        }
                        Ok(None) => eprintln!("archive of {id:?} at game over landed, but its table is gone"),
                        Err(e) => eprintln!("archive of {id:?} at game over landed, but its stamp did not: {e}"),
                    },
                    Err(WriteError::Io(e)) => {
                        eprintln!("archive of {id:?} at game over landed, but its stamp did not: {e}")
                    }
                }
            }
            Err(e) => eprintln!("archive of {id:?} at game over failed: {e}"),
        }
    }

    /// The game-over site's retry as a public door (M6.5c, R-6.5c.4, R-6.5c.9).
    ///
    /// A host whose archive put is ASYNCHRONOUS — a Durable Object uploads to R2
    /// after the move's reply — cannot complete it inside [`Games::persist`], so
    /// it calls this once the upload has landed. The stamp and its retry on a
    /// version conflict stay [`Games::archive_if_over`]'s own code, never a
    /// host's copy. Idempotent: a stamped, unfinished or archive-off table puts
    /// nothing. `Ok(())` means the retry RAN, not that a stamp landed: a refused
    /// put or a failed stamp is the layer's own log line, which a host that
    /// cannot hear `eprintln!` learns by reading `archived_at` afterwards.
    ///
    /// "Unfinished" means a PLAYING game that is not yet over: `Ok(())`, and no
    /// put. A table that has no game at all answers what [`Games::handle`]
    /// answers it — `Err(GameError::Open)` while its seats are filling,
    /// `Err(GameError::Dealing)` while it deals, and `Err(GameError::NotFound)`
    /// for an id with no table — and that is the right answer rather than a
    /// quiet `Ok`: a host asks this only for a game whose game-over put it
    /// deferred, and no such put can exist before there is a record.
    pub fn archive_over_retry(&self, id: &str) -> Result<(), GameError> {
        let handle = self.handle(id)?;
        let repl = handle.lock().expect("a game");
        let Some((record_json, _)) = self.store.read(Doc::Record, id).map_err(GameError::Server)? else {
            return Err(GameError::NotFound(format!("no game named {id:?}")));
        };
        self.archive_if_over(id, &repl, &record_json);
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
        if doc.turn.as_ref() == Some(&turn) {
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
    use crate::chat::Author;
    use crate::store::MemStore;
    use crate::tables::WireSeed;
    use yokohama_core::PlayerId;

    /// **Every bump site answers under [`NoSignal`]** (M6.5a, R-6.5a.8, spec
    /// §2.4).
    ///
    /// The premise, and it is a premise rather than an assertion: the layer is
    /// built by [`test_support::games`], which installs `NoSignal` because
    /// `Games::new` defaults to it. Nothing here can check that at run time — a
    /// `dyn Signal` does not say which one it is — so the dependence is written
    /// down at that helper as well, where a change to the default would be
    /// made.
    ///
    /// The walk drives one write of every kind that bumps and REQUIRES each to
    /// answer `Ok`: `create`, the finalize's `flip_to_playing` and `persist`
    /// reach `bump`, `keep` and `chat_append` reach `bump_table`,
    /// `house_append` reaches `bump_house_chat`, and `delete` reaches `end` —
    /// **all four trait methods, on the implementation that does nothing**.
    ///
    /// **What it adds over the cells beside it.** Many cells in this binary
    /// build with `games()` and so cross some of these sites under `NoSignal`
    /// on their way to asserting something else; a bump site that had kept a
    /// `Signals`-shaped assumption would break one of them and be reported as
    /// whatever that cell was about. This one exists for the crossing itself.
    /// It is the only `NoSignal` cell that drives all four methods THROUGH
    /// this type's write sites — measured, the nearest other reaches three
    /// (`a_delete_removes_the_chat_and_its_count`, which sends no house
    /// message) — so a failure here names the signal rather than a symptom.
    /// Mutation-proven: each of `NoSignal`'s four bodies was made to panic in
    /// turn, and this cell failed on all four.
    ///
    /// It is not a claim about what a bump DOES: under `NoSignal` there is
    /// nothing to observe, which is the point.
    #[test]
    fn every_bump_site_answers_under_no_signal() {
        let games = games();
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));

        // `create` — the bump at the end of its success arm.
        let id = games.create(&two_people("quiet", CORPUS_SEED), &red).expect("a table").id;

        // `keep` — `bump_table` each time, and the LAST human keep finalizes,
        // so the flip to `playing` reaches `bump` as well.
        games.keep(&id, &red, PlayerId::new(0), 1, None).expect("red's keep");
        games.keep(&id, &blue, PlayerId::new(1), 1, None).expect("blue's keep, the finalize");
        // The premise of the line above: a table still dealing has no record,
        // so a handle at all is the finalize having run.
        let handle = games.handle(&id).expect("the finalize built a session");

        // `persist` — `bump`, after an applied move.
        {
            let mut repl = handle.lock().expect("the handle");
            let before = repl.session.record().events.len();
            repl.step("1");
            assert!(repl.session.record().events.len() > before, "the step applied a move");
            games.persist(&id, &repl).expect("the move is persisted");
        }

        // `chat_append` — `bump_table`; `house_append` — `bump_house_chat`.
        let seat = Author::Seat { seat: 0, user: "red@t.test".into(), name: Some("Red".into()) };
        games.chat_append(&id, seat, "hello").expect("a table message");
        games
            .house_append(Author::Watcher { user: "red@t.test".into(), name: None }, "hi all")
            .expect("a house message");

        // `delete` — `end`, the one method the others cannot reach.
        games.delete(&id, &red).expect("the creator's delete");
        assert!(games.read_table(&id).expect("a readable store").is_none(), "the table is gone");
    }

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

    /// **The mirror carries the record's bot from the deal on, and a persist
    /// refreshes it** (M6.5c2, R-6.5c2.10) — a lobby row's only source for a
    /// dealt table's label. An all-human table's mirror has none, and its
    /// stored document does not spell the key.
    #[test]
    fn the_mirror_carries_the_records_bot_from_the_deal_and_a_persist_restores_it() {
        let games = games();
        let red = who("red@t.test");
        let handle = playing(&games, &req("vsbot"), &["red@t.test"]);
        let mirror = games.table("vsbot", &red).unwrap().turn.expect("a dealt table's mirror");
        assert_eq!(mirror.bot, Some(crate::bots::production()), "the deal wrote the record's bot");

        // A mirror written before the field, as every table dealt before this
        // deploy has one.
        let (mut doc, version) = games.read_table("vsbot").unwrap().unwrap();
        doc.turn.as_mut().expect("a mirror").bot = None;
        games.write_table(&doc, Some(version)).unwrap();
        assert_eq!(games.table("vsbot", &red).unwrap().view(&red).bot, None, "premise: no version to name");

        // Red's next move is persisted, and the persist puts the bot back.
        {
            let mut repl = handle.lock().unwrap();
            let before = repl.session.record().events.len();
            repl.step("1");
            assert!(repl.session.record().events.len() > before, "premise: the step applied a move");
            games.persist("vsbot", &repl).unwrap();
        }
        let mirror = games.table("vsbot", &red).unwrap().turn.expect("a mirror");
        assert_eq!(mirror.bot, Some(crate::bots::production()), "the persist refreshed it");

        playing(&games, &two_people("humans", CORPUS_SEED), &["red@t.test", "blue@t.test"]);
        assert_eq!(games.table("humans", &red).unwrap().turn.expect("a mirror").bot, None);
        let (text, _) = games.store().read(Doc::Table, "humans").unwrap().unwrap();
        let stored: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(stored["turn"].get("bot").is_none(), "an all-human table's document spells no `bot`: {text}");
    }

    /// **The foreign-bot rule, at its one site** (M6.5c2, R-6.5c2.10; review
    /// M2): a game NOT over whose recorded bot is not the one this build deals
    /// refuses with the two labels — and refuses BEFORE a `Repl` exists.
    ///
    /// The foreign bot is `lookahead-v1`, which this CLI CAN run, at a cut where
    /// it is to move: a rebuild that got as far as `Repl::new` would pump it and
    /// persist the move. So the untouched write count is the measurement — and
    /// the same cut under THIS build's bot, which does pump and persist, is what
    /// says the count could have moved.
    #[test]
    fn a_game_not_over_against_a_bot_this_build_does_not_run_refuses_before_any_rebuild() {
        let store = Arc::new(CountingStore::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let (record, table) = foreign_pair("lookahead-v1", Some(BOT_TO_MOVE_AT));
        games.load_document(Doc::Record, PLAYED_ID, &record).unwrap();
        games.load_document(Doc::Table, PLAYED_ID, &table).unwrap();
        let loaded = store.writes();
        match games.handle(PLAYED_ID) {
            Err(GameError::ForeignBot { played, runs }) => {
                assert_eq!(played, "V2 lookahead", "the recorded bot's label");
                assert_eq!(runs, crate::bots::label(&crate::bots::production()), "this build's");
            }
            other => panic!("expected the foreign-bot refusal, got {:?}", other.map(|_| "a handle")),
        }
        assert_eq!(store.writes(), loaded, "nothing persisted: no `Repl`, so no pump");
        assert!(games.live.lock().unwrap().is_empty(), "and nothing is resident");

        // The anti-vacuity: the SAME cut under this build's own bot rebuilds,
        // pumps the bot's owed move and persists it.
        let store = Arc::new(CountingStore::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let (own, table) = foreign_pair(&crate::bots::production().controller, Some(BOT_TO_MOVE_AT));
        games.load_document(Doc::Record, PLAYED_ID, &own).unwrap();
        games.load_document(Doc::Table, PLAYED_ID, &table).unwrap();
        let loaded = store.writes();
        let handle = games.handle(PLAYED_ID).expect("this build's own bot opens");
        assert!(handle.lock().unwrap().session.record().events.len() > BOT_TO_MOVE_AT, "the bot moved");
        assert!(store.writes() > loaded, "premise: a rebuild at this cut persists");
    }

    /// **A finished game against a foreign bot opens** (review M2): it has no
    /// bot left to move, so the rule is about resuming and not about reading.
    #[test]
    fn a_finished_game_against_a_foreign_bot_opens() {
        let games = games();
        let (record, table) = foreign_pair("v2-padded", None);
        games.load_document(Doc::Record, PLAYED_ID, &record).unwrap();
        games.load_document(Doc::Table, PLAYED_ID, &table).unwrap();
        let handle = games.handle(PLAYED_ID).expect("a finished game opens");
        assert!(handle.lock().unwrap().session.is_over());
    }

    /// **(Review Focus 4) R2-o1: an owed game-over upload that was undone and
    /// then meets a new bot is bounded, not stranded.** The Worker's retry
    /// reaches the layer as `archive_over_retry`, which answers the foreign-bot
    /// refusal (the shell's backoff retries it: `settleUpload`'s `parseStamp`
    /// throws on an error body), and the creator's Abandon — which reads the
    /// record's event count and never rebuilds — archives the game and clears
    /// it.
    #[test]
    fn an_unfinished_foreign_game_is_refused_by_the_retry_and_cleared_by_abandon() {
        let archive = RecordingArchive::recording();
        let games = games().with_archive(archive.clone());
        let (record, table) = foreign_pair("v2-padded", Some(BOT_TO_MOVE_AT));
        games.load_document(Doc::Record, PLAYED_ID, &record).unwrap();
        games.load_document(Doc::Table, PLAYED_ID, &table).unwrap();
        assert!(matches!(games.archive_over_retry(PLAYED_ID), Err(GameError::ForeignBot { .. })));
        assert!(archive.puts.lock().unwrap().is_empty(), "the retry put nothing");
        games.delete(PLAYED_ID, &who("local@browser")).expect("the creator's Abandon");
        let puts = archive.puts.lock().unwrap();
        assert_eq!(puts.len(), 1, "archived once");
        assert_eq!(puts[0].why, crate::archive::Why::Abandoned, "the mirror says not over");
        assert_eq!(puts[0].record, record, "the stored bytes");
        assert!(games.read_table(PLAYED_ID).unwrap().is_none(), "and gone");
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
        let (games, signals) = watched_games(Arc::clone(&store) as Arc<dyn Store>);
        let id = "bumped".to_string();
        let handle = playing(&games, &req("bumped"), &["red@t.test"]);

        let mut rx = signals.subscribe(&id);
        let mut house = signals.subscribe_house();
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
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        let id = "refused".to_string();
        let handle = playing(&games, &req("refused"), &["red@t.test"]);
        let red = who("red@t.test");
        let repl = handle.lock().expect("the handle");

        games.delete(&id, &red).expect("a delete");
        // `delete` itself ENDS the table's channel, so the subscription for
        // this assertion is taken after it — a fresh sender that has seen
        // nothing, exactly as `signal.rs`'s own end cell describes.
        let rx = signals.subscribe(&id);
        let house = signals.subscribe_house();

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
        let (games, signals) = watched_games(Arc::clone(&store) as Arc<dyn Store>);
        let id = "wfail".to_string();
        let handle = playing(&games, &req("wfail"), &["red@t.test"]);

        let rx = signals.subscribe(&id);
        let house = signals.subscribe_house();
        store.failing.store(true, Ordering::SeqCst);

        let mut repl = handle.lock().expect("the handle");
        repl.step("1");
        let err = games.persist(&id, &repl).expect_err("the store refused the write");
        assert!(err.contains("the disk went away"), "the store's own reason: {err}");

        assert!(!rx.has_changed().expect("the table channel is open"), "no write, no bump");
        assert!(!house.has_changed().expect("the house channel is open"));
    }

    /// **`persist`'s bump follows the MIRROR write, not merely the record
    /// write** (spec §2: "`persist`, after the record write and `mirror_turn`
    /// → `bump`").
    ///
    /// This is the one site the spec spells out in two sequenced steps, and
    /// the reason is the lobby: its whose-turn and over lines are read off the
    /// table document's [`TurnMirror`], so a bump sent between the record
    /// write and the refresh wakes a lobby that reads the PREVIOUS turn — and
    /// then hears nothing more, because the refresh that follows is announced
    /// to nobody. A stale row until the next move at that table.
    ///
    /// **The park sits strictly between the two writes**, which is what makes
    /// the cell discriminating rather than a second copy of
    /// `a_write_the_store_refuses_bumps_nothing`: that one closes the gap
    /// BEFORE the record write, and a bump hoisted one line above
    /// `mirror_turn` passes it and every other cell in this crate. The
    /// anti-vacuity assertion is the record event count read AT the gate — it
    /// proves the record write is already past, so the quiet receiver is quiet
    /// at the one instant the defect would be loud.
    ///
    /// **The stored mirror is cleared on purpose before the move.**
    /// `mirror_turn` returns early when the document already holds the turn it
    /// would write, and after a move that pumps the bot back to the same seat
    /// it does — so without this the table write never happens and the gate
    /// never fires. `None` is the shape a table carries before its first
    /// flip, so it is a document this server can read back.
    #[test]
    fn persists_bump_follows_the_mirror_write() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::mpsc::{channel, Receiver, Sender};
        use std::time::Duration;

        /// Parks the FIRST `Doc::Table` write after it is armed — inside
        /// `persist`, that is `mirror_turn`'s and there is no other.
        ///
        /// Armed by hand rather than keyed on the document, because the flip's
        /// write and this one both carry `status: playing` and nothing in the
        /// bytes tells them apart. Arming after the setup is what does.
        struct MirrorGate {
            inner: MemStore,
            armed: AtomicBool,
            at: Mutex<Sender<()>>,
            go: Mutex<Receiver<()>>,
        }
        impl Store for MirrorGate {
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
                if doc == Doc::Table && self.armed.swap(false, Ordering::SeqCst) {
                    let _ = self.at.lock().expect("the gate").send(());
                    let _ = self.go.lock().expect("the gate").recv();
                }
                self.inner.write_if_version(doc, id, json, expected)
            }
            fn delete(&self, id: &str) -> Result<(), String> {
                self.inner.delete(id)
            }
        }

        let (at_tx, at_rx) = channel();
        let (go_tx, go_rx) = channel();
        let store = Arc::new(MirrorGate {
            inner: MemStore::default(),
            armed: AtomicBool::new(false),
            at: Mutex::new(at_tx),
            go: Mutex::new(go_rx),
        });
        let (games, signals) = watched_games(Arc::clone(&store) as Arc<dyn Store>);
        let games = Arc::new(games);
        let id = "mirror".to_string();
        let handle = playing(&games, &req("mirror"), &["red@t.test"]);

        let events = || {
            let (json, _) = store
                .read(Doc::Record, &id)
                .expect("a readable store")
                .expect("a stored record");
            serde_json::from_str::<GameRecord>(&json).expect("a record").events.len()
        };
        let before = events();

        let (mut doc, version) =
            games.read_table(&id).expect("a readable store").expect("the table");
        doc.turn = None;
        games.write_table(&doc, Some(version)).expect("the mirror is cleared");

        let mut rx = signals.subscribe(&id);
        let mut house = signals.subscribe_house();
        rx.borrow_and_update();
        house.borrow_and_update();
        store.armed.store(true, Ordering::SeqCst);

        let mover = {
            let (games, handle, id) = (Arc::clone(&games), Arc::clone(&handle), id.clone());
            std::thread::spawn(move || {
                let mut repl = handle.lock().expect("the handle");
                repl.step("1");
                games.persist(&id, &repl)
            })
        };
        at_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("persist reaches mirror_turn's table write");

        assert!(events() > before, "the premise: the record write is already past the gate");
        assert!(
            !rx.has_changed().expect("the table channel is open"),
            "the bump was sent before the mirror the woken lobby reads had been refreshed",
        );
        assert!(!house.has_changed().expect("the house channel is open"));

        go_tx.send(()).expect("the parked write is still listening");
        mover.join().expect("the move thread").expect("the persist");

        assert!(rx.has_changed().expect("the table channel is open"), "the bump, once both land");
        assert!(house.has_changed().expect("the house channel is open"));
        let (doc, _) = games.read_table(&id).expect("a readable store").expect("the table");
        assert!(doc.turn.is_some(), "the refreshed mirror a woken lobby reads");
    }

    /// **The host's two side channels into and out of the store** (M6.5b,
    /// R-6.5b.4): a versionless write is a CREATE, and the read gives back the
    /// exact bytes.
    ///
    /// Both halves are load-bearing for the browser host. `load_document` is
    /// what a Worker fills a fresh `MemStore` with at boot and on every lock
    /// acquisition, and it must REFUSE a present document rather than
    /// overwrite one: a second load over a live game would silently roll the
    /// game back to whatever the caller happened to be holding. And the read
    /// must be byte-for-byte, because what the Worker writes back to IndexedDB
    /// is compared with what it last wrote — a re-serialization would differ
    /// in whitespace or key order and make every read look like a change.
    #[test]
    fn a_versionless_load_creates_once_and_reads_back_byte_for_byte() {
        let games = games();
        let text = r#"{"schema":3,"id":"local-1"}"#;
        games.load_document(Doc::Table, "local-1", text).expect("a fresh document is created");
        let again = games.load_document(Doc::Table, "local-1", r#"{"schema":3,"id":"other"}"#);
        assert!(again.is_err(), "a present document is refused, not overwritten: {again:?}");
        assert_eq!(
            games.read_document(Doc::Table, "local-1").expect("a readable store").as_deref(),
            Some(text),
            "the store's exact bytes, and the refused write left them alone",
        );
        assert_eq!(
            games.read_document(Doc::Record, "local-1").expect("a readable store"),
            None,
            "absent is None, and a table is not a record",
        );
    }

    // ---------------------------------------------------------------------
    // The game-over archive's retry as a public door (M6.5c, R-6.5c.4,
    // R-6.5c.9, spec §2.6 and §5.1).
    // ---------------------------------------------------------------------

    /// R-6.5c.9: a host whose put was DEFERRED completes the game-over archive
    /// later, and the stamp is the layer's own — one put, the stored bytes, and
    /// idempotent once stamped.
    #[test]
    fn archive_over_retry_completes_a_game_over_put_that_failed() {
        let games = games().with_archive(RecordingArchive::failing());
        let (_, repl) = play_to_the_end(&games, "late");
        assert!(repl.session.is_over(), "premise: the game is over");
        let (doc, _) = games.read_table("late").unwrap().unwrap();
        assert_eq!(doc.archived_at, None, "premise: the failed put left the stamp unset");

        let archive = RecordingArchive::recording();
        let games = games.with_archive(archive.clone());
        // `play_to_the_end` drives a SECOND, bot-seated `Repl`: the table's own
        // resident handle is still at the opening, so without this eviction the
        // retry reads a game that is not over and the cell fails even though the
        // code is right (measured by the plan's review). In production the
        // resident handle IS the game, so the function itself must not evict.
        games.evict_for_test("late");
        games.archive_over_retry("late").expect("the retry");
        let stored = games.store().read(Doc::Record, "late").unwrap().unwrap().0;
        {
            let puts = archive.puts.lock().unwrap();
            assert_eq!(puts.len(), 1, "one put");
            assert_eq!(puts[0].why, crate::archive::Why::Over);
            assert_eq!(puts[0].prefix, "", "a server's own game carries the empty prefix");
            assert_eq!(puts[0].record, stored, "the archived bytes ARE the stored bytes");
        }
        let (doc, _) = games.read_table("late").unwrap().unwrap();
        assert!(doc.archived_at.is_some(), "the stamp landed");
        games.archive_over_retry("late").expect("a second retry");
        assert_eq!(archive.puts.lock().unwrap().len(), 1, "a stamped game is not put again");
    }

    /// Nothing owed, nothing put: an unfinished game, an archive that is off.
    #[test]
    fn archive_over_retry_does_nothing_for_an_unfinished_game_or_an_archive_that_is_off() {
        let archive = RecordingArchive::recording();
        let running = games().with_archive(archive.clone());
        one_move_in(&running, "mid");
        // NO eviction here (re-review N3): `both_seats_as_bots` writes BOT
        // provenance for BOTH seats, so a rebuild's `bring_alive` would pump up
        // to 200 bot moves. Measured at this seed: `one_move_in` stops at the
        // 200-move cap, the rebuild's one pump plays the remaining 157 to game
        // over (357 events), and the rebuild's own persist archives the game.
        // Evict only a game that is ALREADY over.
        running.archive_over_retry("mid").expect("an unfinished game is not an error");
        assert!(archive.puts.lock().unwrap().is_empty(), "nothing is owed for a game still running");

        // NoArchive, at a game that IS over: the eviction makes the retry rebuild
        // from the finished record. With the archive-off guard in place,
        // `archive_if_over` returns on that guard before it reads "over" at
        // all. Without the eviction the not-over reading would stop the retry
        // even with the archive-off guard gone, so the assert on `archived_at`
        // would say nothing about the archive. What this half does NOT catch,
        // measured: a mutant that drops the `is_on` check from
        // `archive_if_over` stays green, because `NoArchive::put` refuses and no
        // stamp lands either way.
        let off = games();
        play_to_the_end(&off, "off");
        off.evict_for_test("off");
        off.archive_over_retry("off").expect("an archive that is off is not an error");
        assert_eq!(off.read_table("off").unwrap().unwrap().0.archived_at, None);
        assert!(matches!(off.archive_over_retry("nobody"), Err(GameError::NotFound(_))));
    }

    /// The stamp's version-conflict retry is the layer's code, and this is the one
    /// cell that reaches it: an archive whose put WRITES the table document — the
    /// shape of an `unlock` landing in the same instant — so `archive_if_over`'s
    /// compare-and-swap meets a version it did not read, re-reads, and stamps over
    /// the newer document without losing it.
    #[test]
    fn archive_over_retry_stamps_through_a_concurrent_table_write_and_keeps_it() {
        struct Meddling {
            store: Arc<dyn Store>,
        }
        impl crate::archive::Archive for Meddling {
            fn put(&self, _: &str, _: crate::archive::Why, id: &str, _: &str, _: &str)
                -> Result<crate::archive::PutOutcome, crate::archive::PutError> {
                let (text, version) = self.store.read(Doc::Table, id).unwrap().unwrap();
                let mut doc: crate::table::TableDoc = serde_json::from_str(&text).unwrap();
                doc.unlocked.push("meddler@t.test".to_string());
                let json = serde_json::to_string(&doc).unwrap();
                self.store.write_if_version(Doc::Table, id, &json, Some(version)).unwrap();
                Ok(crate::archive::PutOutcome::Landed)
            }
            fn kind(&self) -> &'static str { "meddling" }
        }
        let games = games().with_archive(RecordingArchive::failing());
        play_to_the_end(&games, "raced");
        let store = Arc::clone(games.store());
        let games = games.with_archive(Arc::new(Meddling { store }));
        games.evict_for_test("raced"); // the resident handle is the opening's — see the first cell
        games.archive_over_retry("raced").expect("the retry");
        let (doc, _) = games.read_table("raced").unwrap().unwrap();
        assert!(doc.archived_at.is_some(), "the stamp was retried over the concurrent write");
        assert!(doc.unlocked.iter().any(|u| u == "meddler@t.test"), "and the concurrent write was kept");
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
    use crate::signal::Signals;
    use crate::store::{Doc, MemStore, Store};
    use crate::tables::{NewGame, SeatSpec, WireSeed};

    /// The in-memory layer most cells drive.
    ///
    /// **Its signal is [`crate::signal::NoSignal`]**, because `Games::new`
    /// defaults to it (M6.5a, R-6.5a.8). That is a premise
    /// `every_bump_site_answers_under_no_signal` rests on and cannot assert —
    /// a `dyn Signal` does not say which one it is — so a change to the
    /// default belongs with a re-read of that cell. Every cell in THIS module that watches a
    /// bump uses [`watched_games`] instead (the route cells watch through
    /// `AppState`'s own `signals`).
    pub(crate) fn games() -> Games {
        Games::new(Arc::new(Rules::standard()), Arc::new(MemStore::default()))
    }

    /// A [`Games`] over `store`, **and the signal handle to subscribe on**.
    ///
    /// The signal is a trait object inside `Games` since M6.5a (R-6.5a.8) and
    /// there is no way back out of it — bumping is all a write site may do — so
    /// a cell that WATCHES a bump builds the [`Signals`] itself and keeps its
    /// half, exactly as `AppState` does. [`games`] above stays the helper for
    /// every cell that does not watch: its signal is the `NoSignal` that
    /// `Games::new` defaults to.
    pub(crate) fn watched_games(store: Arc<dyn Store>) -> (Games, Arc<Signals>) {
        let signals = Arc::new(Signals::default());
        let games = Games::with_parts(
            Arc::new(Rules::standard()),
            store,
            crate::chat::ChatConfig::default(),
            signals.clone(),
            Games::ticking_clock(),
            Arc::new(crate::archive::NoArchive),
        );
        (games, signals)
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
        User::parse(s).expect("a key")
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

    /// One remembered put, as [`RecordingArchive`] saw it.
    ///
    /// A named struct rather than the tuple it was until M6.5b: the prefix is a
    /// fifth thing to check and `puts[0].4` reads like nothing at all.
    pub(crate) struct Put {
        pub prefix: String,
        pub why: crate::archive::Why,
        pub id: String,
        pub record: String,
        pub table: String,
    }

    /// An [`Archive`](crate::archive::Archive) that REMEMBERS its puts, or
    /// refuses every one of them — the two shapes the archive-site cells need.
    pub(crate) struct RecordingArchive {
        pub puts: Mutex<Vec<Put>>,
        pub fail: bool,
    }

    impl RecordingArchive {
        pub(crate) fn recording() -> Arc<Self> {
            Arc::new(Self { puts: Mutex::new(Vec::new()), fail: false })
        }
        pub(crate) fn failing() -> Arc<Self> {
            Arc::new(Self { puts: Mutex::new(Vec::new()), fail: true })
        }
    }

    impl crate::archive::Archive for RecordingArchive {
        fn put(
            &self,
            prefix: &str,
            why: crate::archive::Why,
            id: &str,
            r: &str,
            t: &str,
        ) -> Result<crate::archive::PutOutcome, crate::archive::PutError> {
            if self.fail {
                return Err(crate::archive::PutError::Failed("disk on fire".to_string()));
            }
            self.puts.lock().unwrap().push(Put {
                prefix: prefix.to_string(),
                why,
                id: id.to_string(),
                record: r.to_string(),
                table: t.to_string(),
            });
            Ok(crate::archive::PutOutcome::Landed)
        }
        fn kind(&self) -> &'static str {
            "recording"
        }
    }

    /// The two-person table `id`, dealt from [`CORPUS_SEED`], played to the END
    /// by pumping BOTH seats as bots in process and PERSISTING after every pump
    /// — the way `input` persists after every move; a pump plays a run of up to
    /// 200 moves, so this is two or three persists — so the game-over site sees
    /// more than one persist and exactly one of them is the end. Returns how
    /// many persists it made and the finished `Repl`.
    pub(crate) fn play_to_the_end(games: &Games, id: &str) -> (usize, Repl) {
        let mut repl = both_seats_as_bots(games, id);
        let mut persists = 0;
        for _ in 0..400 {
            if repl.session.is_over() {
                break;
            }
            repl.pump_bots();
            games.persist(id, &repl).expect("a persist");
            persists += 1;
        }
        assert!(repl.session.is_over(), "the both-bot pump must reach a finished game");
        (persists, repl)
    }

    /// A two-person table with ONE bot-pumped move persisted: playing, with
    /// events, and nowhere near over.
    pub(crate) fn one_move_in(games: &Games, id: &str) -> Repl {
        let mut repl = both_seats_as_bots(games, id);
        repl.pump_bots();
        assert!(!repl.session.is_over());
        games.persist(id, &repl).expect("a persist");
        repl
    }

    /// The table `id` opened and dealt through `Games` ([`playing`]), and a
    /// SECOND `Repl` over the very game it describes — same seed, same colours,
    /// same keeps (card 1 at every seat, `Keep(0)`) — with both seats as bots,
    /// so `pump_bots` drives it. A `Repl` learns its bots at construction from
    /// the session's run provenance and `Session` is not `Clone`, which is why
    /// the table's own handle cannot simply be told to play itself; `persist`
    /// compares record identity by SEED, so this one persists in its place.
    fn both_seats_as_bots(games: &Games, id: &str) -> Repl {
        use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
        use yokohama_cli::session::Session;
        use yokohama_core::{GameConfig, StartingOrderChoice};
        let handle = playing(games, &two_people(id, CORPUS_SEED), &["red@t.test", "blue@t.test"]);
        let (colors, start) = {
            let live = handle.lock().unwrap();
            let state = live.session.state();
            (state.config.player_colors.clone(), state.config.start_player)
        };
        let mut config = GameConfig::for_player_count(2);
        config.player_colors = colors;
        config.start_player = start;
        let mut session = Session::new_sim(
            config,
            CORPUS_SEED,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        )
        .expect("the very game the table describes");
        let bots =
            BotSeats::new(vec![Some(PROVISIONAL_DEFAULT_BOT), Some(PROVISIONAL_DEFAULT_BOT)], CORPUS_SEED);
        session.set_run_seats(bots.seat_provenance());
        Repl::new(session)
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

    /// The committed browser-played pair (595 events, a person at seat 0 and
    /// the V4 bot at seat 1), as `browser_played_pin.rs` holds it.
    pub(crate) const PLAYED_RECORD: &str = include_str!("../tests/fixtures/browser-played-2p.json");
    pub(crate) const PLAYED_TABLE: &str = include_str!("../tests/fixtures/browser-played-2p.table.json");
    /// The pair's own id, under which every cell stores it.
    pub(crate) const PLAYED_ID: &str = "local-242a5429453c593c";
    /// The event count at which [`foreign_pair`]'s cut leaves the BOT (seat 1)
    /// to move in a game not yet over — measured, and asserted as a premise by
    /// the cell that reads it.
    pub(crate) const BOT_TO_MOVE_AT: usize = 300;

    /// The committed pair re-seated as a game played against ANOTHER bot
    /// (M6.5c2, R-6.5c2.10): seat 1's recorded controller renamed to
    /// `controller`, its version and knobs kept — `Contestant::from_parts`
    /// reads `lookahead-v1`'s or a named identity's three knobs from them and
    /// accepts the identity keys — with the table's mirror naming the same
    /// bot, as a persist under the build that dealt it would have written.
    /// With `cut`, only the first `cut` events, re-stamped for the state they
    /// fold to, and the mirror moved with them: not over, and that state's
    /// seat to move. Answers `(record, table)` as JSON.
    pub(crate) fn foreign_pair(controller: &str, cut: Option<usize>) -> (String, String) {
        use yokohama_core::{state_at, GameRecord};
        let mut record: GameRecord = serde_json::from_str(PLAYED_RECORD).expect("the pair's record");
        let mut table: crate::table::TableDoc = serde_json::from_str(PLAYED_TABLE).expect("the pair's table");
        let seat = &mut record.run.as_mut().expect("the pair's run provenance").seats[1];
        seat.controller = controller.to_string();
        let bot = crate::bots::BotDescriptor {
            controller: seat.controller.clone(),
            version: seat.version,
            params: seat.params.clone(),
        };
        let mirror = table.turn.as_mut().expect("the pair's mirror");
        mirror.bot = Some(bot);
        if let Some(cut) = cut {
            record.events.truncate(cut);
            let state = state_at(&record, cut, &Rules::standard()).expect("the cut replays");
            record.stamp_terminal_digest(&state);
            *mirror = crate::table::TurnMirror {
                to_move: state.current_player.0,
                over: false,
                winner: None,
                bot: mirror.bot.take(),
            };
        }
        (serde_json::to_string(&record).expect("a record"), serde_json::to_string(&table).expect("a table"))
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
