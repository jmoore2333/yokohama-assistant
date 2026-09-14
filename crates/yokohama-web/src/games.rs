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
use yokohama_core::greedy::score_starting_order;
use yokohama_core::{
    new_game, GameConfig, GameRecord, OrderCard, OrderCardId, PlayerColor, PlayerId, Rules,
    SeedSchedule, StartingOrderChoice,
};

use crate::client::HistoricalFrame;
use crate::store::{is_safe_id, Doc, GameId, Store, Version, WriteError};

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
    /// One entry per seat: `1` or `2` for a human seat, as the cards are
    /// numbered on screen; `null` for a bot seat, whose keep is the bot's
    /// (M5.5 spec §3.3.2). A value on a bot seat, or `null` on a human one,
    /// is refused by name — the client never chooses for the bot again.
    pub keeps: Vec<Option<u8>>,
    #[serde(default)]
    pub bot_seats: Vec<u8>,
    #[serde(default)]
    pub name: Option<String>,
}

/// One dealt card and what the built-in bot would score it at.
#[derive(Debug, Clone, Serialize)]
pub struct DealCard {
    pub card: OrderCard,
    /// `score_starting_order` — the printed economics, hand-set and
    /// unmeasured (its own doc); shown beside the card under the CLI's hedge.
    pub keep_score: i64,
}

/// The two cards a seat was dealt, so a client can show them before choosing.
///
/// **A seat only ever sees its OWN deal (2026-09-06, R-C1).** On a bot seat
/// `cards` is EMPTY and there is no keep field on this struct at all — the
/// client draws two card backs from `is_bot` alone. The bot still keeps by the
/// same argmax ([`bot_keep`], applied in [`choices_of`] at `create`): the
/// record knows which card it took, and the player does not.
#[derive(Debug, Clone, Serialize)]
pub struct Deal {
    pub seat: PlayerId,
    pub is_bot: bool,
    /// The seat's two dealt cards, each with its Keep score — and `[]` on a bot
    /// seat, whose deal is not the reader's to see.
    pub cards: Vec<DealCard>,
}

/// The set table (M5.5 spec §3.3.1): the position after setup, a throwaway
/// projection over a session that is never persisted, beside the deals.
#[derive(Debug, Clone, Serialize)]
pub struct Preview {
    pub seed: WireSeed,
    pub table: HistoricalFrame,
    pub deals: Vec<Deal>,
}

/// Why a preview could not be served — and, more to the point, WHOSE fault it
/// is.
///
/// [`preview`] used to return one `String` and the route mapped every one of
/// them to 400. Three of the four things that can go wrong there are not the
/// caller's: once [`config_of`] has accepted a request, a `new_game`, a
/// `Session::new_sim` or a `HistoricalFrame::at` failure is this process
/// failing to build a position from a request that was fine — and answering
/// "your request was wrong" sends the client away to fix something that is not
/// broken. Only the first is a 400.
#[derive(Debug, Clone)]
pub enum PreviewError {
    /// The request cannot start a game: [`config_of`]'s own refusals, which
    /// name the seat, the numbering or the count. A 400.
    Request(String),
    /// The setup, or its projection, failed. Nothing the caller sent is wrong.
    /// A 500.
    Server(String),
}

impl PreviewError {
    /// The sentence, whichever side it came from.
    pub fn message(&self) -> &str {
        match self {
            Self::Request(m) | Self::Server(m) => m,
        }
    }
}

/// The bot's Keep: the argmax of [`score_starting_order`] over its two dealt
/// cards, ties to the first — the rule every evaluator contestant applies in
/// the arena (`GreedyAgent::choose_starting_order` at `greedy.rs:752`, and
/// `LookaheadAgent`'s identical arm at `lookahead.rs:2167`), so the web bot
/// opens exactly as the bot that produced the V4 figures. 1-based, as the wire
/// numbers the cards.
///
/// Those two agents read a PRECOMPUTED `keep_scores` table built over
/// `rules.orders` in full, so calling the function directly is the same
/// arithmetic and not a second derivation. `map_or(i64::MIN, …)` mirrors their
/// missing-card arm: an id the registry does not know loses, and cannot be
/// dealt in the first place.
pub fn bot_keep(deal: &[OrderCardId; 2], rules: &Rules) -> u8 {
    let score =
        |id: OrderCardId| rules.orders.get(id).map_or(i64::MIN, |c| score_starting_order(c, rules));
    if score(deal[1]) > score(deal[0]) {
        2
    } else {
        1
    }
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
fn config_of(req: &NewGame) -> Result<(GameConfig, BotSeats), String> {
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

    // The seats are settled BEFORE the keeps, because which seats are bots is
    // what makes a keep entry right or wrong. The order of the checks is the
    // order of the sentences a client reads.
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

    if req.keeps.len() != n {
        return Err(format!("{} keep entries for {n} seats", req.keeps.len()));
    }
    for (i, keep) in req.keeps.iter().enumerate() {
        match (bots.is_bot(PlayerId::new(i as u8)), keep) {
            (true, Some(_)) => {
                return Err(format!("seat {i} is a bot; its keep is the bot's — send null"))
            }
            (false, None) => {
                return Err(format!("seat {i} is human and needs a keep of 1 or 2"))
            }
            (false, Some(k)) if !(1..=2).contains(k) => {
                return Err(format!("keep {k} — the dealt cards are numbered 1 and 2"))
            }
            _ => {}
        }
    }

    let mut config = GameConfig::for_player_count(req.players);
    config.player_colors = req.colors.clone();
    config.start_player = PlayerId::new(req.start_seat);
    Ok((config, bots))
}

/// Every seat's starting-order choice: the human's as sent, the bot's by
/// [`bot_keep`] over the deal the seed produced.
fn choices_of(
    req: &NewGame,
    bots: &BotSeats,
    deals: &[[OrderCardId; 2]],
    rules: &Rules,
) -> Vec<StartingOrderChoice> {
    deals
        .iter()
        .enumerate()
        .map(|(i, deal)| {
            let seat = PlayerId::new(i as u8);
            let keep = if bots.is_bot(seat) {
                bot_keep(deal, rules)
            } else {
                req.keeps[i].expect("config_of checked it")
            };
            StartingOrderChoice::Keep(keep - 1)
        })
        .collect()
}

/// Take the technologies and the achievements back off the previewed table.
///
/// **The choice comes first in the owner's edition**, which is the Tabletop
/// Simulator upload in this repository — `tts-assets/pdf/` +
/// `httpssteamusercontentaakamaihdnetugc753716318425766140` +
/// `37EE3A37AB39CBA5E73B74B7F97C54556D7C7719.PDF`, its setup steps on **p.5**
/// (three lines because the filename is one 88-character token). At **step 9**
/// the order deck is built, the port filled, and each player dealt two orders
/// to choose between IN SECRET; the technology
/// cards reach the Laboratory only at **step 10** and the achievements the
/// score track at **step 11** — so a player choosing a starting order has
/// neither in front of them, and neither does this screen (2026-09-06, R-C3;
/// `analysis-1/00-yokohama-rulebook.pdf` orders those three differently and is
/// the outlier of the two editions in this repository).
///
/// **This is a VIEW, not a rule.** Nothing here touches the engine: the seed,
/// the draw order and the position `new_game` built are exactly what they were,
/// and `SEMANTICS_REV` does not move — the same cards are dealt, in the same
/// order, and the game `create` starts from this seed shows them all. What
/// changes is only when the person looking at the screen is told.
///
/// `deck_remaining` is deliberately KEPT on both panels: how many cards are
/// left is public arithmetic at any table, and it is the identities that have
/// not come out yet.
fn withhold_until_the_choice(table: &mut HistoricalFrame) {
    for board in &mut table.labs.boards {
        board.slots.clear();
    }
    table
        .frame
        .dashboard
        .retain(|panel| !matches!(panel, yokohama_cli::screen_model::Panel::Achievements(_)));
}

/// The deal a seed produces AND the table it sets, without starting anything.
///
/// **A throwaway, never a game.** The `Session` built here is dropped when this
/// function returns: nothing is persisted, nothing joins `Games::live`, and the
/// id a create would claim is never touched. What the caller gets is a
/// projection of the position after setup, built through the SAME
/// [`HistoricalFrame`] the replay route serves, so the client draws the set
/// table with the components it already has and no second frame type exists.
pub fn preview(req: &NewGame, rules: &Rules) -> Result<Preview, PreviewError> {
    let (config, bots) = config_of(req).map_err(PreviewError::Request)?;
    let seed = req.seed.map_or_else(draw_seed, |s| s.0);
    let pending = new_game(config.clone(), rules, seed)
        .map_err(|e| PreviewError::Server(format!("the setup did not deal: {e}")))?;
    // A human seat previews as `Unknown` — no card in a hand nobody has chosen
    // from — and a bot seat at its argmax, the card the game will deal it.
    let choices: Vec<StartingOrderChoice> = pending
        .deals
        .iter()
        .enumerate()
        .map(|(i, deal)| {
            if bots.is_bot(PlayerId::new(i as u8)) {
                StartingOrderChoice::Keep(bot_keep(deal, rules) - 1)
            } else {
                StartingOrderChoice::Unknown
            }
        })
        .collect();
    let mut session =
        Session::new_sim(config, seed, choices, rules.clone()).map_err(PreviewError::Server)?;
    // `Repl::new` reads WHO plays each seat off the record's run seats, so this
    // is what makes the previewed table's `is_bot` true — the same one line
    // `create` runs, and the reason no second attachment step is needed here.
    session.set_run_seats(bots.seat_provenance());
    let repl = Repl::new(session);
    let mut table = HistoricalFrame::at(&repl, 0).map_err(|e| {
        PreviewError::Server(format!("the setup position did not build: {}", e.message))
    })?;
    withhold_until_the_choice(&mut table);
    let deals = pending
        .deals
        .iter()
        .enumerate()
        .map(|(i, deal)| {
            let seat = PlayerId::new(i as u8);
            let is_bot = bots.is_bot(seat);
            Deal {
                seat,
                is_bot,
                // **A bot seat sends no cards (R-C1).** Not a filter on the
                // client's side of the wire but an absence on this one: the
                // payload cannot leak what it does not carry. The bot's own
                // keep is computed above, where the previewed table needs it,
                // and reaches the client only as a card already in a hand.
                //
                // **`cards[0]` IS card 1 and `cards[1]` is card 2.** The wire
                // numbers the dealt cards from 1 because that is how they are
                // numbered on screen, and `PendingSetup.deals` indexes them
                // from 0 — so the join between the two is this order, and it is
                // what the three `- 1` conversions in this module rely on
                // ([`bot_keep`]'s result, [`choices_of`]'s `Keep(keep - 1)`,
                // and the human's `req.keeps[i]`). A `filter_map` that dropped
                // an unknown id would slide the second card into the first
                // one's place; it cannot fire, because every id here came out
                // of `new_game` against these same `rules`.
                cards: if is_bot {
                    Vec::new()
                } else {
                    deal.iter()
                        .filter_map(|id| rules.orders.get(*id).cloned())
                        .map(|card| DealCard {
                            keep_score: score_starting_order(&card, rules),
                            card,
                        })
                        .collect()
                },
            }
        })
        .collect();
    Ok(Preview { seed: WireSeed(seed), table, deals })
}

/// Every live game, plus the store behind them.
pub struct Games {
    rules: Arc<Rules>,
    store: Arc<dyn Store>,
    live: Mutex<HashMap<GameId, Arc<Mutex<Repl>>>>,
    /// What this process last successfully LEFT IN THE STORE, per id — the
    /// [`Version`] the store reported for the JSON written (or, on a cold
    /// rebuild, for the JSON read).
    ///
    /// **This is what makes the write a compare-and-swap rather than a
    /// monotonicity heuristic**, and the difference is a working undo. See
    /// [`Games::persist`].
    written: Mutex<HashMap<GameId, Version>>,
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
    pub fn new(rules: Arc<Rules>, store: Arc<dyn Store>) -> Self {
        Self {
            rules,
            store,
            live: Mutex::new(HashMap::new()),
            written: Mutex::new(HashMap::new()),
            create_lock: Mutex::new(()),
            deleted: Mutex::new(std::collections::HashSet::new()),
        }
    }

    pub fn store(&self) -> &Arc<dyn Store> {
        &self.store
    }

    /// Every game with a stored record.
    pub fn list(&self) -> Result<Vec<GameId>, String> {
        self.store.list(Doc::Record)
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
        let (config, bots) = config_of(req)?;
        let seed = req.seed.map_or_else(draw_seed, |s| s.0);

        let id = match &req.name {
            Some(name) if !is_safe_id(name) => {
                return Err(format!("{name:?} is not a usable game name"))
            }
            Some(name) => name.clone(),
            // An INDEPENDENT draw, never `seed`. A default id of
            // `sim-{seed:016x}` published the setup seed — hidden-set member #1
            // — in the id itself, so `GET /api/games` handed out every game's
            // hidden set to anyone who could list. Dropping the seed from the
            // payloads (see `client.rs`) is undone if the id still spells it.
            // A second draw from the same OS entropy is independent of the
            // first and identifies the game without describing it.
            None => format!("sim-{:016x}", draw_seed()),
        };
        // **An unreadable record is OCCUPIED, not absent.** The store's `read`
        // now carries that distinction in its type — `Ok(None)` is `NotFound`
        // and nothing else, every other failure is an `Err` — so the question
        // the retired `exists` method existed to answer is answered here, by
        // reading an `Err` as "a file that exists and cannot be spoken for".
        // Reading it as absence is what let a new game overwrite a real one:
        // measured with a `chmod 000`, the POST returned 201 and the original
        // record was gone. The id reaching this line is already known safe (the
        // `match` above refuses an unsafe name and the default is generated),
        // so the only `Err` left here is I/O.
        let occupied = match self.store.read(Doc::Record, &id) {
            Ok(found) => found.is_some(),
            Err(_) => true,
        };
        if occupied {
            return Err(format!("a game named {id:?} already exists"));
        }

        self.deleted.lock().expect("games lock").remove(&id);

        // The deal is drawn HERE rather than trusted from a preview: a bot seat's
        // keep is a function of the seed and the rules, so the card this deals
        // it is the card the preview showed, and a client cannot send a
        // different one.
        let pending = new_game(config.clone(), &self.rules, seed).map_err(|e| e.to_string())?;
        let choices = choices_of(req, &bots, &pending.deals, &self.rules);
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
        let (json, version) =
            self.store.read(Doc::Record, id)?.ok_or_else(|| format!("no game named {id:?}"))?;
        // The base for `persist`'s compare-and-swap: what we found in the store
        // is what we may overwrite until someone else changes it. Recorded
        // BEFORE `live` is taken — nothing may take `written` while holding
        // `live`.
        self.written.lock().expect("games lock").insert(id.to_string(), version);
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
        // A read that FAILS (not "is absent" — the store distinguishes them)
        // leaves `expected` at `None`, and the write then refuses rather than
        // landing: `write_if_version` reads the same file, fails the same way,
        // and answers `Io`. The old code wrote straight over an unreadable
        // record, and that is the one behaviour here that deliberately did not
        // survive the move — `read` is how `create` already learns that a file
        // it cannot read is a file it must not overwrite.
        let stored = self.store.read(Doc::Record, id).ok().flatten();
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
        Ok(())
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
        self.written.lock().expect("games lock").remove(id);
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

    /// The corpus seed. **At this seed seat 1's argmax is card 1** (57 against
    /// 31), which is the same card the old `keeps: [1, 1]` wire sent — so a
    /// cell that only runs here cannot tell the bot's own Keep from a constant.
    /// [`ARGMAX_IS_CARD_2`] is what discriminates; this seed stays in use for
    /// the cells whose subject is not WHICH card.
    const CORPUS_SEED: u64 = 20_260_831;

    /// A seed where seat 1's argmax is **card 2** (9 against 36), so the bot's
    /// Keep differs from every constant the old wire could have sent. Each cell
    /// using it asserts that as a PREMISE rather than trusting this comment: a
    /// deal that changed would otherwise turn these cells vacuous silently.
    const ARGMAX_IS_CARD_2: u64 = 20_260_830;

    fn req_seeded(name: &str, seed: u64) -> NewGame {
        NewGame {
            players: 2,
            colors: vec![PlayerColor::Red, PlayerColor::Blue],
            start_seat: 0,
            seed: Some(WireSeed(seed)),
            keeps: vec![Some(1), None],
            bot_seats: vec![1],
            name: Some(name.to_string()),
        }
    }

    fn req(name: &str) -> NewGame {
        req_seeded(name, CORPUS_SEED)
    }

    /// The same request, named for the cells whose subject is the BOT's seat.
    fn req_with_bot(name: &str) -> NewGame {
        req(name)
    }

    /// One dealt card's Keep score, through core's own function.
    ///
    /// The module is NOT globbed to the crate root (`game/mod.rs` lists every
    /// `pub use x::*` and `greedy` is deliberately namespaced), so the path is
    /// `yokohama_core::greedy::` and not `yokohama_core::`.
    fn keep_score(rules: &Rules, id: yokohama_core::OrderCardId) -> i64 {
        let card = rules.orders.get(id).expect("a dealt card");
        yokohama_core::greedy::score_starting_order(card, rules)
    }

    /// The bot keeps the higher-scored of its two cards \u2014 the argmax every
    /// evaluator contestant applies in the arena
    /// (`GreedyAgent::choose_starting_order`, `greedy.rs:752`, and
    /// `LookaheadAgent`'s identical arm), reached through core. The human's keep
    /// is honoured beside it, and the cell is mutation-proven by the human's
    /// choice: swapping 1 for 2 moves the human's card and not the bot's.
    ///
    /// **At [`ARGMAX_IS_CARD_2`], deliberately.** At the corpus seed the bot's
    /// argmax IS card 1, which is the card the old `keeps: [1, 1]` wire already
    /// sent — so a `bot_keep` reduced to the constant `1` would pass there and
    /// this cell would prove nothing about the feature. The premise is asserted
    /// below rather than trusted, so a deal that moved says so out loud instead
    /// of quietly making the cell vacuous again.
    #[test]
    fn the_bot_keeps_the_higher_scored_card_and_the_human_keeps_theirs() {
        let rules = Rules::standard();
        let pending =
            new_game(GameConfig::for_player_count(2), &rules, ARGMAX_IS_CARD_2).unwrap();
        let deal = pending.deals[1];
        assert!(
            keep_score(&rules, deal[1]) > keep_score(&rules, deal[0]),
            "the premise: at this seed seat 1's argmax must be card 2 ({} then {}), or a \
             constant `bot_keep = 1` passes this cell",
            keep_score(&rules, deal[0]),
            keep_score(&rules, deal[1]),
        );
        let expected = deal[1];

        let g = games();
        let (_, handle) = g.create(&req_seeded("keep", ARGMAX_IS_CARD_2)).unwrap();
        let r = handle.lock().unwrap();
        assert_eq!(r.session.state().players[1].order_cards_in_hand, vec![expected]);
        assert_eq!(r.session.state().players[0].order_cards_in_hand, vec![pending.deals[0][0]]);
        drop(r);

        let mut two = req_seeded("keep2", ARGMAX_IS_CARD_2);
        two.keeps = vec![Some(2), None];
        let (_, handle) = g.create(&two).unwrap();
        let r = handle.lock().unwrap();
        assert_eq!(r.session.state().players[0].order_cards_in_hand, vec![pending.deals[0][1]]);
        assert_eq!(
            r.session.state().players[1].order_cards_in_hand,
            vec![expected],
            "the bot's card does not follow the human's choice",
        );
    }

    /// The refusal a `create` gives. `unwrap_err` cannot be used: the Ok half
    /// carries a `Repl`, which is not `Debug` and must not become `Debug` to
    /// satisfy a test.
    fn refused(g: &Games, req: &NewGame) -> String {
        match g.create(req) {
            Ok((id, _)) => panic!("expected a refusal; the request created {id:?}"),
            Err(e) => e,
        }
    }

    /// Every way a `keeps` list can disagree with the seats, refused BY NAME.
    #[test]
    fn a_keep_on_a_bot_seat_and_a_missing_keep_on_a_human_seat_are_refused_by_name() {
        let g = games();
        let mut on_bot = req_with_bot("b");
        on_bot.keeps = vec![Some(1), Some(1)];
        let e = refused(&g, &on_bot);
        assert!(e.contains("seat 1") && e.contains("bot"), "{e}");
        let mut on_human = req_with_bot("h");
        on_human.keeps = vec![None, None];
        let e = refused(&g, &on_human);
        assert!(e.contains("seat 0"), "{e}");
        let mut bad = req_with_bot("n");
        bad.keeps = vec![Some(3), None];
        let e = refused(&g, &bad);
        assert!(e.contains("1 and 2"), "{e}");
        let mut short = req_with_bot("s");
        short.keeps = vec![Some(1)];
        let e = refused(&g, &short);
        assert!(e.contains("2 seats"), "{e}");
    }

    /// The preview is the position after setup \u2014 a throwaway, never a game \u2014
    /// with every human hand empty (nothing has been chosen) and the bot's
    /// kept card in the bot's hand; each dealt card carries its Keep score.
    ///
    /// **At the corpus seed on purpose**, so the projection is asserted against
    /// the same position the committed fixtures hold. **Which card the bot kept
    /// is not on this payload and is not asserted here (R-C1)** \u2014 the deal is
    /// empty on a bot seat; what the projection still says out loud is that the
    /// bot HAS chosen, which is the hand count below.
    #[test]
    fn the_preview_is_the_position_after_setup() {
        let rules = Rules::standard();
        let p = preview(&req_with_bot("p"), &rules).unwrap();
        assert_eq!(p.table.index, 0);
        assert_eq!(p.table.total, 0);
        assert!(
            p.table.ports.boards[0].slots.iter().all(|s| s.card.is_some()),
            "the port is full",
        );
        let players = p
            .table
            .frame
            .dashboard
            .iter()
            .find_map(|panel| match panel {
                yokohama_cli::screen_model::Panel::Players(pp) => Some(pp),
                _ => None,
            })
            .expect("a Players panel");
        assert!(players.players[1].is_bot);
        assert_eq!(players.players[1].orders_in_hand, 1, "the bot's kept card is in its hand");
        assert_eq!(players.players[0].orders_in_hand, 0, "nothing chosen yet");
        assert_eq!(p.deals.len(), 2);
        assert!(!p.deals[0].is_bot);
        assert_eq!(p.deals[0].cards.len(), 2);
        assert!(p.deals[1].is_bot);
        assert!(p.deals[1].cards.is_empty(), "a bot seat's deal is not on the wire");
        for d in &p.deals {
            for c in &d.cards {
                assert_eq!(
                    c.keep_score,
                    yokohama_core::greedy::score_starting_order(&c.card, &rules),
                );
            }
        }
        // Nothing was persisted or made resident.
        let g = games();
        assert!(g.list().unwrap().is_empty());
    }

    /// **Another seat's deal never reaches the wire (2026-09-06, R-C1).** A bot
    /// seat's `cards` is empty and `bot_keep` is not a field at all, so a
    /// player reading the payload cannot learn what the bot was dealt or which
    /// of the two it kept. The record still knows: the kept card is in the
    /// bot's hand in the game `create` builds, which
    /// [`the_created_game_holds_the_bots_own_keep_at_both_seeds`] reads.
    ///
    /// **At [`ARGMAX_IS_CARD_2`]** so the seat this cell empties is the seat
    /// whose Keep is not the constant `1`, and with the human's half asserted
    /// beside it: a `preview` that emptied EVERY deal would hide the bot's
    /// cards and take the player's own with them.
    #[test]
    fn a_bot_seats_deal_never_reaches_the_wire() {
        let rules = Rules::standard();
        let p = preview(&req_seeded("hidden", ARGMAX_IS_CARD_2), &rules).unwrap();
        assert!(p.deals[1].is_bot, "the premise: seat 1 is the bot");
        assert!(p.deals[1].cards.is_empty(), "a bot seat's cards are not on the wire");
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("bot_keep"), "the payload names the bot's keep");
        // **The KEY SET, not the old field's name (R-C10).** The line above
        // searches for one literal, so a keep re-added under any other name —
        // `kept`, `keep_index`, `argmax` — passed every cell in this crate and
        // was caught only by the committed corpus, which CI does not run. This
        // reads what the bot's deal actually serializes to and requires it to
        // be these three keys and nothing else, which is the claim this cell's
        // own doc makes. The client half of the same fence is
        // `contract.test.ts`'s `preview_bot.deal(bot)` row.
        let bot = serde_json::to_value(&p.deals[1]).unwrap();
        let mut keys: Vec<&str> =
            bot.as_object().expect("a deal is an object").keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["cards", "is_bot", "seat"], "the bot's deal carries {bot}");
        // Anti-vacuity: the human's own deal is untouched, with both Keep
        // scores, so the withholding is targeted rather than a blank preview.
        assert!(!p.deals[0].is_bot);
        assert_eq!(p.deals[0].cards.len(), 2);
        for c in &p.deals[0].cards {
            assert_eq!(c.keep_score, yokohama_core::greedy::score_starting_order(&c.card, &rules));
        }
    }

    /// The achievement TYPE names on a frame's dashboard, or empty where the
    /// frame carries no `Achievements` panel at all. Reading them through one
    /// function keeps "the panel is absent" and "the panel is present with
    /// these three cards" from being two different shapes of assertion.
    fn achievement_names(frame: &HistoricalFrame) -> Vec<String> {
        frame
            .frame
            .dashboard
            .iter()
            .find_map(|panel| match panel {
                yokohama_cli::screen_model::Panel::Achievements(a) => Some(a),
                _ => None,
            })
            .map(|a| a.cards.iter().map(|c| format!("{:?}", c.achievement_type)).collect())
            .unwrap_or_default()
    }

    /// **The technologies and the goals are withheld until the choice
    /// (2026-09-06, R-C3).** The previewed table carries the laboratory with
    /// EMPTY slots and no `Achievements` panel at all, so the player choosing a
    /// starting order reads the same table the owner's edition has in front of
    /// them at its step 9 — before the technologies go out at step 10 and the
    /// achievements at step 11.
    ///
    /// **Read against the game the same seed CREATES**, not against a list
    /// written here: the six technology titles and the three achievement types
    /// are learned from the created game's own frame and then required to be
    /// absent from the preview's JSON. A withholding checked against a
    /// hard-coded list stops discriminating the moment the deal changes.
    ///
    /// The anti-vacuity is the port: its six order ids ARE in the preview's
    /// JSON, so this is a targeted withholding and not a blank table. And
    /// `deck_remaining` is kept — the count is public arithmetic; it is the
    /// identities that are not on the table yet.
    #[test]
    fn the_preview_withholds_the_laboratory_and_the_achievements() {
        let rules = Rules::standard();
        let p = preview(&req_seeded("withheld", ARGMAX_IS_CARD_2), &rules).unwrap();

        // What the SAME seed puts on a real table, read from the game itself.
        let g = games();
        let (_, handle) = g.create(&req_seeded("withheld", ARGMAX_IS_CARD_2)).unwrap();
        let started = HistoricalFrame::at(&handle.lock().unwrap(), 0).unwrap();
        let titles: Vec<String> = started
            .labs
            .boards
            .iter()
            .flat_map(|b| b.slots.iter())
            .filter_map(|s| s.printed.as_ref().map(|c| c.name.clone()))
            .collect();
        assert_eq!(titles.len(), 6, "the premise: a 2P laboratory offers six technologies");
        let goals = achievement_names(&started);
        assert_eq!(goals.len(), 3, "the premise: three achievement cards are in play");

        // `all` over an empty vector is TRUE, so the board itself is asserted
        // first: a withholding that cleared the BOARDS rather than their slots
        // would otherwise pass this cell while deleting the laboratory.
        assert!(
            !p.table.labs.boards.is_empty(),
            "the laboratory board is still drawn on the set table",
        );
        assert!(
            p.table.labs.boards.iter().all(|b| b.slots.is_empty()),
            "every laboratory slot is empty on the set table",
        );
        assert_eq!(
            p.table.labs.deck_remaining, started.labs.deck_remaining,
            "the technology deck's COUNT is public and is kept",
        );
        // The PANEL's absence, not merely an empty card list: `achievement_names`
        // returns nothing for either, and "no panel" is the claim.
        assert!(
            !p.table
                .frame
                .dashboard
                .iter()
                .any(|panel| matches!(panel, yokohama_cli::screen_model::Panel::Achievements(_))),
            "the set table carries no Achievements panel",
        );

        let json = serde_json::to_string(&p).unwrap();
        for title in &titles {
            assert!(!json.contains(title.as_str()), "the preview names the technology {title}");
        }
        for name in &goals {
            assert!(!json.contains(name.as_str()), "the preview names the achievement {name}");
        }
        // Anti-vacuity: the port is still on the table, by id.
        let port_ids: Vec<u8> = started
            .ports
            .boards
            .iter()
            .flat_map(|b| b.slots.iter())
            .filter_map(|s| s.card.map(|id| id.0))
            .collect();
        assert_eq!(port_ids.len(), 6, "the premise: a 2P port offers six orders");
        for id in port_ids {
            assert!(
                p.table.ports.boards.iter().flat_map(|b| b.slots.iter()).any(|s| s
                    .card
                    .is_some_and(|c| c.0 == id)),
                "the port's order {id} is missing from the set table",
            );
        }
    }

    /// The bot's own Keep is what lands in the created game's hand \u2014 read at
    /// BOTH seeds, where the two answers differ.
    ///
    /// **This cell used to read the preview's answer** (`bot_keep`), and the
    /// deal left the wire in the 2026-09-06 corrections (R-C1). The claim it
    /// made is still true and is still worth a cell, so it is made where it
    /// remains observable: the state `create` builds. It runs at BOTH seeds
    /// because one seed cannot discriminate \u2014 at [`CORPUS_SEED`] the argmax is
    /// card 1 and at [`ARGMAX_IS_CARD_2`] it is card 2, so a keep reduced to
    /// either constant fails one of the two halves. Each seed's premise is
    /// asserted from `score_starting_order` rather than trusted, so a deal that
    /// moved says so instead of turning the cell vacuous.
    #[test]
    fn the_created_game_holds_the_bots_own_keep_at_both_seeds() {
        let rules = Rules::standard();
        let g = games();
        for (seed, expected_card) in [(CORPUS_SEED, 1usize), (ARGMAX_IS_CARD_2, 2usize)] {
            let pending = new_game(GameConfig::for_player_count(2), &rules, seed).unwrap();
            let deal = pending.deals[1];
            let (a, b) = (keep_score(&rules, deal[0]), keep_score(&rules, deal[1]));
            let argmax = if b > a { 2 } else { 1 };
            assert_eq!(
                argmax, expected_card,
                "the premise at seed {seed}: the bot's argmax must be card {expected_card} \
                 ({a} then {b}), or the pair of seeds no longer discriminates",
            );
            let name = format!("agree-{seed}");
            let (_, handle) = g.create(&req_seeded(&name, seed)).unwrap();
            assert_eq!(
                handle.lock().unwrap().session.state().players[1].order_cards_in_hand,
                vec![deal[expected_card - 1]],
                "seed {seed}: the bot holds the card it scored highest",
            );
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
            Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let (id, _handle) = games.create(&req("digest")).expect("a game");

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
        let games = Games::new(Arc::clone(&rules), Arc::clone(&store) as Arc<dyn Store>);
        let (id, _handle) = games.create(&req("fromcli")).expect("a game");

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

        let (id, handle) = games.create(&req("reused")).expect("a game");
        let repl = handle.lock().expect("the handle");
        let doomed_seed = repl.session.record().seed;

        games.delete(&id).expect("a delete");

        // The name comes back as a DIFFERENT game — a different seed is what
        // makes it a different board, deal and box.
        let mut second = req("reused");
        second.seed = Some(WireSeed(20_260_901));
        games.create(&second).expect("the name is reusable");
        let after_create = stored_json(&store, &id);

        let err = match games.persist(&id, &repl) {
            Ok(()) => panic!("the in-flight write landed on someone else's game"),
            Err(e) => e,
        };
        assert!(
            err.contains("different game"),
            "the refusal says the id holds another game: {err}",
        );

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
        let (id, handle) = games.create(&req("stale")).expect("a game");

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
                Arc::clone(&store) as Arc<dyn Store>,
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
                serde_json::from_str(&stored_json(&store, &name)).expect("a record");
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

    /// The record `id` is stored as, as bytes.
    ///
    /// `expect` twice, and the two are different questions: the store failed,
    /// or the store answered "not there". A cell calling this wants a record
    /// that is present, so either is a failed premise.
    fn stored_json(store: &MemStore, id: &str) -> String {
        store.read(Doc::Record, id).expect("a readable store").expect("a stored record").0
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
        let (id, handle) = games.create(&req("undo")).expect("a game");

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
        let (id, handle) = games.create(&req("fork")).expect("a game");
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

    /// An existing game whose record cannot be READ is still occupied.
    #[test]
    fn create_refuses_a_name_whose_record_is_unreadable() {
        struct Unreadable;
        impl Store for Unreadable {
            fn list(&self, _: Doc) -> Result<Vec<GameId>, String> {
                Ok(vec!["taken".to_string()])
            }
            /// An `Err`, not `Ok(None)` — the shape a present file that cannot
            /// be read has, and the one `create` must not mistake for absence.
            fn read(&self, _: Doc, _: &str) -> Result<Option<(String, Version)>, String> {
                Err("permission denied".to_string())
            }
            fn write_if_version(
                &self,
                _: Doc,
                _: &str,
                _: &str,
                _: Option<Version>,
            ) -> Result<Version, WriteError> {
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
