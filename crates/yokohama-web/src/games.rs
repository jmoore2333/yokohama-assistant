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
        {
            let mut live = self.live.lock().expect("games lock");
            live.insert(id.clone(), Arc::clone(&handle));
            evict_down_to(&mut live, &id);
        }
        Ok((id, handle))
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
            return Err(format!("{id:?} was deleted while this move was in flight"));
        }
        let events = repl.session.record().events.len();
        if let Ok(stored) = self.store.read(id) {
            if let Ok(on_disk) = serde_json::from_str::<GameRecord>(&stored) {
                if on_disk.events.len() > events {
                    return Err(format!(
                        "{id:?} has advanced past this handle ({} stored vs {events} here) — refusing to write backwards",
                        on_disk.events.len(),
                    ));
                }
            }
        }
        let json =
            serde_json::to_string_pretty(repl.session.record()).map_err(|e| e.to_string())?;
        self.store.write(id, &json)
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
