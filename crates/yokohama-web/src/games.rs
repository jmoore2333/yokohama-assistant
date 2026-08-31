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

/// What a client sends to start a game.
#[derive(Debug, Clone, Deserialize)]
pub struct NewGame {
    pub players: u8,
    pub colors: Vec<PlayerColor>,
    #[serde(default)]
    pub start_seat: u8,
    /// Omit to have the server draw one. **Omitting is the right default** —
    /// see [`draw_seed`].
    #[serde(default)]
    pub seed: Option<u64>,
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
    pub seed: u64,
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
    let seed = req.seed.unwrap_or_else(draw_seed);
    let pending = new_game(config, rules, seed).map_err(|e| e.to_string())?;
    Ok(Preview {
        seed,
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
}

impl Games {
    pub fn new(rules: Arc<Rules>, store: Arc<dyn SessionStore>) -> Self {
        Self { rules, store, live: Mutex::new(HashMap::new()) }
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
        let seed = req.seed.unwrap_or_else(draw_seed);

        let id = match &req.name {
            Some(name) if !is_safe_id(name) => {
                return Err(format!("{name:?} is not a usable game name"))
            }
            Some(name) => name.clone(),
            None => format!("sim-{seed:016x}"),
        };
        if self.store.read(&id).is_ok() {
            return Err(format!("a game named {id:?} already exists"));
        }

        let mut session = Session::new_sim(config, seed, choices, (*self.rules).clone())?;
        session.set_run_seats(bots.seat_provenance());
        let repl = Repl::new(session);

        let handle = Arc::new(Mutex::new(repl));
        {
            let mut r = handle.lock().expect("fresh handle");
            bring_alive(&mut r);
            self.persist(&id, &r)?;
        }
        self.live.lock().expect("games lock").insert(id.clone(), Arc::clone(&handle));
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
        Ok(Arc::clone(live.entry(id.to_string()).or_insert(handle)))
    }

    /// Write `repl`'s record through. Called after every mutation.
    pub fn persist(&self, id: &str, repl: &Repl) -> Result<(), String> {
        let json =
            serde_json::to_string_pretty(repl.session.record()).map_err(|e| e.to_string())?;
        self.store.write(id, &json)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        self.store.delete(id)?;
        self.live.lock().expect("games lock").remove(id);
        Ok(())
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
