//! Live games, and the rule that none of them may exist only in memory.
//!
//! A handle is a `Repl` behind a mutex. It is a cache, never the record: every
//! applied move is persisted immediately, and a handle that is absent is
//! rebuilt from the stored `GameRecord` through `Session::from_record`, which
//! runs every refusal the CLI's resume path runs. That is what lets this server
//! live in a container that sleeps.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
use yokohama_cli::repl::Repl;
use yokohama_cli::session::Session;
use yokohama_core::{
    new_game, GameConfig, GameRecord, OrderCard, PlayerColor, PlayerId, Rules, SeedSchedule,
    StartingOrderChoice,
};

use crate::store::{is_safe_id, GameId, SessionStore};

/// A `u64` that crosses the wire as a STRING.
///
/// **Not fussiness — a correctness bug.** A JavaScript `number` is an IEEE
/// double and cannot hold a `u64`. [`draw_seed`] draws over the FULL range, so
/// a real game's seed is around 1.8e19, and `JSON.parse` rounds it silently:
/// the browser was already displaying `5856483359458926000`, whose last four
/// digits are not in the seed.
///
/// The setup flow makes that worse than a display bug, because it ROUND-TRIPS
/// the value: `/api/games/preview` draws a seed and deals from it, the client
/// shows the deal and sends the seed back to `POST /api/games`, and a rounded
/// seed starts a DIFFERENT GAME than the one the player looked at. The cards
/// they chose between are not the cards they get.
///
/// Deserialization accepts a string or a number, so a small literal from `curl`
/// or a test still works and nothing that was correct becomes an error.
/// Serialization always emits a string, so nothing a client reads can be lossy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireSeed(pub u64);

impl Serialize for WireSeed {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for WireSeed {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Either {
            Text(String),
            Number(u64),
        }
        match Either::deserialize(d)? {
            Either::Number(n) => Ok(WireSeed(n)),
            Either::Text(t) => t
                .trim()
                .parse::<u64>()
                .map(WireSeed)
                .map_err(|e| serde::de::Error::custom(format!("{t:?} is not a u64 seed: {e}"))),
        }
    }
}

/// What a client sends to start a game.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewGame {
    pub players: u8,
    pub colors: Vec<PlayerColor>,
    #[serde(default)]
    pub start_seat: u8,
    /// Omit to have the server draw one. **Omitting is the right default** —
    /// see [`draw_seed`]. A string or a number; see [`WireSeed`] for why the
    /// answer always comes back as a string.
    #[serde(default)]
    pub seed: Option<WireSeed>,
    /// Which dealt card each seat keeps, `1` or `2`, as the cards are numbered
    /// on screen.
    pub keeps: Vec<u8>,
    #[serde(default)]
    pub bot_seats: Vec<u8>,
    #[serde(default)]
    pub name: Option<String>,
}

/// The two cards a seat was dealt, so a client can show them before choosing.
#[derive(Debug, Clone, Serialize)]
pub struct Deal {
    pub seat: PlayerId,
    pub cards: Vec<OrderCard>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Preview {
    pub seed: WireSeed,
    pub deals: Vec<Deal>,
}

/// A full-width seed from OS entropy.
///
/// **Not a convenience.** `yokohama_core`'s `agent.rs` records that the board is
/// generated from the setup seed BEFORE the deck is shuffled, so the public
/// board is an injective fingerprint of the seed: small sequential seeds are
/// brute-forceable in milliseconds, recovering the entire hidden set. A
/// timestamp is no better. `SeedSchedule::from_os_entropy` is the engine's own
/// answer and is used here rather than a local `rand` call so there is one
/// derivation to audit.
pub fn draw_seed() -> u64 {
    SeedSchedule::from_os_entropy().1
}

/// Validate a request the way the wizard validates its answers — same rules,
/// same refusals, so a game started here is a game the CLI could have started.
fn config_of(req: &NewGame) -> Result<(GameConfig, Vec<StartingOrderChoice>, BotSeats), String> {
    if !(2..=4).contains(&req.players) {
        return Err(format!("{} players — the game is 2 to 4", req.players));
    }
    let n = usize::from(req.players);
    if req.colors.len() != n {
        return Err(format!("{} colours for {n} seats", req.colors.len()));
    }
    let mut seen = req.colors.clone();
    seen.sort_by_key(|c| format!("{c:?}"));
    seen.dedup();
    if seen.len() != n {
        return Err("two seats cannot share a colour".to_string());
    }
    if usize::from(req.start_seat) >= n {
        return Err(format!("seat {} cannot start a {n}-seat game", req.start_seat));
    }
    if req.keeps.len() != n {
        return Err(format!("{} keep choices for {n} seats", req.keeps.len()));
    }
    let choices = req
        .keeps
        .iter()
        .map(|&k| match k {
            1 | 2 => Ok(StartingOrderChoice::Keep(k - 1)),
            other => Err(format!("keep {other} — the dealt cards are numbered 1 and 2")),
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut seats: Vec<Option<yokohama_core::Contestant>> = vec![None; n];
    for &s in &req.bot_seats {
        let idx = usize::from(s);
        if idx >= n {
            return Err(format!("no seat {s} in a {n}-seat game"));
        }
        seats[idx] = Some(PROVISIONAL_DEFAULT_BOT);
    }
    // The wizard's rule, kept: an all-bot table is refused. Bot-vs-bot
    // spectating is deliberately out of v1, and a table with nobody at it
    // would sit there waiting for a move that never comes.
    if seats.iter().all(Option::is_some) {
        return Err("at least one seat must stay human".to_string());
    }
    let bots = if seats.iter().any(Option::is_some) {
        BotSeats::new(seats, draw_seed())
    } else {
        BotSeats::none()
    };

    let mut config = GameConfig::for_player_count(req.players);
    config.player_colors = req.colors.clone();
    config.start_player = PlayerId::new(req.start_seat);
    Ok((config, choices, bots))
}

/// The deal a seed produces, without starting anything.
pub fn preview(req: &NewGame, rules: &Rules) -> Result<Preview, String> {
    let (config, _, _) = config_of(req)?;
    let seed = req.seed.map_or_else(draw_seed, |s| s.0);
    let pending = new_game(config, rules, seed).map_err(|e| e.to_string())?;
    Ok(Preview {
        seed: WireSeed(seed),
        deals: pending
            .deals
            .iter()
            .enumerate()
            .map(|(i, cards)| Deal {
                seat: PlayerId::new(i as u8),
                cards: cards.iter().filter_map(|id| rules.orders.get(*id).cloned()).collect(),
            })
            .collect(),
    })
}

/// Every live game, plus the store behind them.
pub struct Games {
    rules: Arc<Rules>,
    store: Arc<dyn SessionStore>,
    live: Mutex<HashMap<GameId, Arc<Mutex<Repl>>>>,
    /// Serializes `create` against itself, and nothing else.
    ///
    /// `create` is check-then-write across a slow middle (`Session::new_sim`
    /// plus the opening bot pump), so without this two requests both read
    /// "absent" and both went on to build, write and insert. Held across the
    /// WHOLE of `create` — the `exists` check, the build, the write and the
    /// insert — because any shorter hold leaves the same window somewhere else.
    ///
    /// A dedicated lock rather than holding `live`: `live` is taken by every
    /// `handle`, and blocking ordinary reads for the length of a game setup to
    /// serialize an operation a user performs by hand is the wrong trade. Two
    /// creates now queue; a create and a read do not.
    create_lock: Mutex<()>,
    /// Ids deleted since this process started, so a request that was already
    /// holding a handle cannot write the record back after `DELETE` removed it.
    /// Cleared by `create`, so a name can be reused.
    deleted: Mutex<std::collections::HashSet<GameId>>,
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
    pub fn new(rules: Arc<Rules>, store: Arc<dyn SessionStore>) -> Self {
        Self {
            rules,
            store,
            live: Mutex::new(HashMap::new()),
            create_lock: Mutex::new(()),
            deleted: Mutex::new(std::collections::HashSet::new()),
        }
    }

    pub fn store(&self) -> &Arc<dyn SessionStore> {
        &self.store
    }

    pub fn list(&self) -> Result<Vec<GameId>, String> {
        self.store.list()
    }

    /// Start a game and persist it before returning, so a client that is told
    /// "created" can always find it again.
    pub fn create(&self, req: &NewGame) -> Result<(GameId, Arc<Mutex<Repl>>), String> {
        // Held for the whole function. `exists` -> build -> `persist` ->
        // `insert` is check-then-write across a slow middle, and two requests
        // that both read "absent" both went on to write a DIFFERENT game
        // (`config_of` draws a fresh bot seed per request) — the loser's record
        // landed on disk, the winner's insert was clobbered, and the client
        // told 201 was silently switched onto the other game.
        let _serialized = self.create_lock.lock().expect("games lock");
        let (config, choices, bots) = config_of(req)?;
        let seed = req.seed.map_or_else(draw_seed, |s| s.0);

        let id = match &req.name {
            Some(name) if !is_safe_id(name) => {
                return Err(format!("{name:?} is not a usable game name"))
            }
            Some(name) => name.clone(),
            None => format!("sim-{seed:016x}"),
        };
        // `exists`, not `read(...).is_ok()`. The latter reads EVERY failure as
        // absence, so a stored game whose file was momentarily unreadable was
        // answered 201 and overwritten — measured with a `chmod 000`: the
        // create succeeded and the original record was gone.
        if self.store.exists(&id)? {
            return Err(format!("a game named {id:?} already exists"));
        }

        self.deleted.lock().expect("games lock").remove(&id);

        let mut session = Session::new_sim(config, seed, choices, (*self.rules).clone())?;
        session.set_run_seats(bots.seat_provenance());
        let repl = Repl::new(session);

        let handle = Arc::new(Mutex::new(repl));
        {
            let mut r = handle.lock().expect("fresh handle");
            bring_alive(&mut r);
            self.persist(&id, &r)?;
        }
        // `entry().or_insert()`, never `insert()` — the same rule `handle`
        // follows forty lines below, and for the same reason: one game must
        // never be two handles applying moves to two divergent records. The
        // create lock makes a colliding CREATE impossible, but a concurrent
        // `handle(&id)` can still have rebuilt this id off the store between
        // the `persist` above and this line, so keep whichever landed first
        // and hand the caller THAT one.
        //
        // **NO TEST COVERS THIS LINE**, and the distinction is worth the
        // sentence: `two_creates_of_one_name_cannot_both_succeed` is
        // mutation-proven against removing the create lock, and passes with
        // this reverted to `insert()`. It is defence in depth against the
        // create-versus-rebuild window, which no cell drives — reaching it
        // needs a store that can be parked mid-`persist`. Written down rather
        // than implied, because a comment that reads like a covered invariant
        // is how this lane has three times shipped a fence nothing held.
        let kept = {
            let mut live = self.live.lock().expect("games lock");
            let kept = Arc::clone(live.entry(id.clone()).or_insert(handle));
            evict_down_to(&mut live, &id);
            kept
        };
        Ok((id, kept))
    }

    /// The live handle for `id`, rebuilt from the store if it is not resident.
    pub fn handle(&self, id: &str) -> Result<Arc<Mutex<Repl>>, String> {
        if let Some(h) = self.live.lock().expect("games lock").get(id) {
            return Ok(Arc::clone(h));
        }
        let json = self.store.read(id)?;
        let record: GameRecord = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let session = Session::from_record(record, (*self.rules).clone())?;
        let mut repl = Repl::new(session);
        // A game that comes back with a bot to move is owed that move — the
        // same thing `main.rs` does through `opening()` when a save is resumed.
        // Persist only if the pump actually played, so reading a game that was
        // already waiting on a human rewrites nothing.
        let before = repl.session.record().events.len();
        bring_alive(&mut repl);
        if repl.session.record().events.len() != before {
            self.persist(id, &repl)?;
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
    /// **Two guards, and each closes a measured way to lose a move.**
    ///
    /// A game DELETED while this request was in flight is not resurrected. The
    /// window is the whole of `Repl::step`, bot pump included — measured at
    /// 0.16-0.30 s in a debug build — and without the check the file came back:
    /// `DELETE` answered 204, the file went, the in-flight input finished, and
    /// `FsStore::write` re-created the parent directory and the record.
    ///
    /// A record that has ALREADY MOVED PAST this handle is not overwritten. The
    /// `live` map caches a handle forever and nothing invalidates it against
    /// the store, so with two processes over one store — the arrangement
    /// `SessionStore` exists for — a stale handle could publish a record with
    /// FEWER events than the one on disk, erasing a move another writer had
    /// already applied and acknowledged. This is a monotonicity guard, not a
    /// compare-and-swap: it refuses a write that would go backwards, which is
    /// the shape of the loss, and does not pretend to serialize two writers
    /// that are both moving forwards.
    pub fn persist(&self, id: &str, repl: &Repl) -> Result<(), String> {
        if self.deleted.lock().expect("games lock").contains(id) {
            return Err(self.conflict(id, format!("{id:?} was deleted while this move was in flight")));
        }
        let record = repl.session.record();
        let events = record.events.len();
        if let Ok(stored) = self.store.read(id) {
            if let Ok(on_disk) = serde_json::from_str::<GameRecord>(&stored) {
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
                if on_disk.seed != record.seed {
                    return Err(self.conflict(
                        id,
                        format!(
                            "{id:?} now holds a different game (stored seed {}, this handle's {}) — \
                             refusing to write one game's events over another's",
                            on_disk.seed, record.seed,
                        ),
                    ));
                }
                if on_disk.events.len() > events {
                    return Err(self.conflict(
                        id,
                        format!(
                            "{id:?} has advanced past this handle ({} stored vs {events} here) — refusing to write backwards",
                            on_disk.events.len(),
                        ),
                    ));
                }
            }
        }
        // `Session::to_record_json`, never `to_string_pretty(record())`: the
        // terminal digest is stamped at WRITE time, so serializing the record
        // directly here wrote a record with no digest (disabling the engine's
        // provenance refusal on every game this server owns) and rewrote a
        // CLI-written save's digest-for-state-N onto a state-N+1 record, which
        // the next cold rebuild refused forever.
        let json = repl.session.to_record_json()?;
        self.store.write(id, &json)
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

    /// Delete, and remember that we did.
    ///
    /// The tombstone is what stops an in-flight `input` from writing the record
    /// back after the file is gone. It is bounded by `create`, which clears the
    /// id — so the set only ever holds ids that are deleted and not recreated,
    /// and re-creating a deleted name works exactly as before.
    pub fn delete(&self, id: &str) -> Result<(), String> {
        self.store.delete(id)?;
        self.live.lock().expect("games lock").remove(id);
        self.deleted.lock().expect("games lock").insert(id.to_string());
        Ok(())
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
fn evict_down_to(live: &mut HashMap<GameId, Arc<Mutex<Repl>>>, keep: &str) {
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
    use super::*;
    use crate::store::MemStore;

    fn games() -> Games {
        Games::new(Arc::new(Rules::standard()), Arc::new(MemStore::default()))
    }

    fn req(name: &str) -> NewGame {
        NewGame {
            players: 2,
            colors: vec![PlayerColor::Red, PlayerColor::Blue],
            start_seat: 0,
            seed: Some(WireSeed(20_260_831)),
            keeps: vec![1, 1],
            bot_seats: vec![1],
            name: Some(name.to_string()),
        }
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
            Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn SessionStore>);
        let (id, _handle) = games.create(&req("digest")).expect("a game");

        let stored = store.read(&id).expect("the record");
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
        store.write(&id, &tampered.to_string()).expect("the tampered write");

        let fresh =
            Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn SessionStore>);
        // `Repl` is not `Debug`, so match rather than `expect_err`.
        let err = match fresh.handle(&id) {
            Ok(_) => panic!("an edited record must be refused, not loaded"),
            Err(e) => e,
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
        let games = Games::new(Arc::clone(&rules), Arc::clone(&store) as Arc<dyn SessionStore>);
        let (id, _handle) = games.create(&req("fromcli")).expect("a game");

        // COLD rebuild, so the resident session's record is the STAMPED one
        // off the store rather than the `None`-digest record `create` built.
        let cold = Games::new(Arc::clone(&rules), Arc::clone(&store) as Arc<dyn SessionStore>);
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

        let fresh = Games::new(Arc::clone(&rules), Arc::clone(&store) as Arc<dyn SessionStore>);
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
        let games = Games::new(Arc::clone(&rules), Arc::clone(&store) as Arc<dyn SessionStore>);

        let (id, handle) = games.create(&req("reused")).expect("a game");
        let repl = handle.lock().expect("the handle");
        let doomed_seed = repl.session.record().seed;

        games.delete(&id).expect("a delete");

        // The name comes back as a DIFFERENT game — a different seed is what
        // makes it a different board, deal and box.
        let mut second = req("reused");
        second.seed = Some(WireSeed(20_260_901));
        games.create(&second).expect("the name is reusable");
        let after_create = store.read(&id).expect("the new record");

        let err = match games.persist(&id, &repl) {
            Ok(()) => panic!("the in-flight write landed on someone else's game"),
            Err(e) => e,
        };
        assert!(
            err.contains("different game"),
            "the refusal says the id holds another game: {err}",
        );

        let now: GameRecord =
            serde_json::from_str(&store.read(&id).expect("the record")).expect("a record");
        assert_ne!(now.seed, doomed_seed, "the deleted game did not come back");
        assert_eq!(
            store.read(&id).expect("the record"),
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
        let games = Games::new(Arc::clone(&rules), Arc::clone(&store) as Arc<dyn SessionStore>);
        let (id, handle) = games.create(&req("stale")).expect("a game");

        // Someone else moves the stored game past this handle.
        {
            let other = Games::new(Arc::clone(&rules), Arc::clone(&store) as Arc<dyn SessionStore>);
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

    /// Two creates of one name cannot both succeed.
    ///
    /// `create` was check-then-write with nothing held across it: `exists` ->
    /// (slow) `Session::new_sim` -> `persist` -> `live.insert`. Two requests
    /// both read "absent", both built, and both wrote — and the two games are
    /// genuinely DIFFERENT, because `config_of` draws a fresh bot seed per
    /// request, so with a bot at the start seat `bring_alive` plays a different
    /// opening. `persist`'s guard was read-then-write and length-only, so
    /// A.read(absent) -> B.read(absent) -> A.write(12) -> B.write(9) left B on
    /// disk and A's twelve-event game nowhere. The final `live.insert` then
    /// CLOBBERED, so the client holding A was silently switched onto B.
    ///
    /// **The realistic reproduction is one user and one browser**: `NewGame`'s
    /// submit guard read a `busy` STATE rather than a ref, which `App`'s own
    /// comment already records as insufficient — "React batches, so two clicks
    /// in the same tick both read `busy === false` and both post."
    ///
    /// Rounds, because the window is real but small; the mutation proof for
    /// this cell is to remove the create lock and watch it fail.
    #[test]
    fn two_creates_of_one_name_cannot_both_succeed() {
        for round in 0..40 {
            let store = Arc::new(MemStore::default());
            let games = Arc::new(Games::new(
                Arc::new(Rules::standard()),
                Arc::clone(&store) as Arc<dyn SessionStore>,
            ));
            let name = format!("dup{round}");
            let barrier = Arc::new(std::sync::Barrier::new(2));

            let threads: Vec<_> = (0..2)
                .map(|_| {
                    let games = Arc::clone(&games);
                    let barrier = Arc::clone(&barrier);
                    let name = name.clone();
                    std::thread::spawn(move || {
                        barrier.wait();
                        // The seed is left to the server on both, exactly as a
                        // double-clicked Start sends it, so a winner and a
                        // loser are distinguishable by what landed.
                        let mut r = req(&name);
                        r.seed = None;
                        games.create(&r).map(|(id, handle)| {
                            (id, handle.lock().expect("a handle").session.record().seed)
                        })
                    })
                })
                .collect();

            let outcomes: Vec<_> = threads.into_iter().map(|t| t.join().expect("no panic")).collect();
            let winners: Vec<_> = outcomes.iter().filter_map(|o| o.as_ref().ok()).collect();
            assert_eq!(
                winners.len(),
                1,
                "round {round}: exactly one create may take the name, got {}",
                winners.len(),
            );

            // The record on disk is the winner's game, and the resident handle
            // is the same one the winner was handed.
            let stored: GameRecord =
                serde_json::from_str(&store.read(&name).expect("the record")).expect("a record");
            assert_eq!(stored.seed, winners[0].1, "round {round}: the stored game is the winner's");
            let resident = games.handle(&name).expect("the resident handle");
            assert_eq!(
                resident.lock().expect("a handle").session.record().seed,
                winners[0].1,
                "round {round}: the client that was told 201 is not switched onto another game",
            );
        }
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
        let (id, handle) = games.create(&req("drace")).expect("a game");
        let repl = handle.lock().expect("the handle");

        games.delete(&id).expect("a delete");
        assert!(games.list().expect("a list").is_empty(), "the delete took");

        let err = games.persist(&id, &repl).expect_err("the in-flight write is refused");
        assert!(err.contains("deleted"), "the refusal says why: {err}");
        assert!(games.list().expect("a list").is_empty(), "and the game stays gone");

        // The name is reusable: the tombstone holds ids that are deleted and
        // NOT recreated, so this must succeed rather than trip the guard.
        assert!(games.create(&req("drace")).is_ok(), "the name can be reused");
        assert_eq!(games.list().expect("a list"), vec!["drace".to_string()]);
    }

    /// A handle that has fallen behind the store cannot write the store
    /// backwards.
    ///
    /// `live` caches a handle forever and nothing invalidates it against the
    /// store, so with two processes over one store — the arrangement
    /// `SessionStore` exists for — a stale handle would publish a record with
    /// FEWER events, erasing a move another writer had already applied and
    /// acknowledged.
    #[test]
    fn a_stale_handle_cannot_erase_a_move_another_writer_already_stored() {
        let store = Arc::new(MemStore::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn SessionStore>);
        let (id, handle) = games.create(&req("stale")).expect("a game");

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
            serde_json::from_str(&store.read(&id).expect("stored")).expect("a record");
        let ahead = record.events.last().cloned().expect("the move is recorded");
        record.events.push(ahead);
        store.write(&id, &serde_json::to_string(&record).expect("json")).expect("a write");
        let ahead_len = record.events.len();

        let repl = handle.lock().expect("the handle");
        assert!(repl.session.record().events.len() < ahead_len, "the premise: this handle is behind");

        let err = games.persist(&id, &repl).expect_err("a backwards write is refused");
        assert!(err.contains("advanced past"), "the refusal says why: {err}");

        let after: GameRecord =
            serde_json::from_str(&store.read(&id).expect("stored")).expect("a record");
        assert_eq!(after.events.len(), ahead_len, "the newer record survived");
    }

    /// An existing game whose record cannot be READ is still occupied.
    #[test]
    fn create_refuses_a_name_whose_record_is_unreadable() {
        struct Unreadable;
        impl SessionStore for Unreadable {
            fn list(&self) -> Result<Vec<GameId>, String> {
                Ok(vec!["taken".to_string()])
            }
            fn read(&self, _: &str) -> Result<String, String> {
                Err("permission denied".to_string())
            }
            fn exists(&self, id: &str) -> Result<bool, String> {
                Ok(id == "taken")
            }
            fn write(&self, _: &str, _: &str) -> Result<(), String> {
                panic!("a refused create must not reach the store");
            }
            fn delete(&self, _: &str) -> Result<(), String> {
                Ok(())
            }
        }
        let games = Games::new(Arc::new(Rules::standard()), Arc::new(Unreadable));
        let err = games.create(&req("taken")).err().expect("the name is occupied");
        assert!(err.contains("already exists"), "{err}");
    }
}
