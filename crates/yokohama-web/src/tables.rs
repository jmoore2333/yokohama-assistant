//! The TABLE lifecycle: create, deal, keep, finalize, list, delete.
//!
//! A game begins here and not in [`crate::games`]. `create` writes a TABLE
//! document ([`crate::table`]) with no record and no session behind it: the
//! seats are bound, the seed is drawn, and every human seat is owed a deal.
//! Each seat reads its OWN two cards from `deal` and chooses through `keep`,
//! and the LAST human keep FINALIZES — builds the session, writes the record,
//! and flips the table to `playing`. Only then is there a `GameRecord` for
//! [`crate::games`] to rebuild a handle from, which is why every session route
//! answers 409 until that happens.
//!
//! The two writes a finalize makes are not one transaction, so both crash
//! windows are repaired on the way past the next read ([`Games::settle`])
//! rather than left to a human.
//!
//! **This module and [`crate::games`] are one type cut along a seam**, not two
//! layers: both add methods to [`Games`], the table half here and the record
//! half there. The cut is behaviour-preserving — nothing below moved anything
//! but its address.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
use yokohama_cli::repl::Repl;
use yokohama_cli::session::Session;
use yokohama_core::greedy::score_starting_order;
use yokohama_core::{
    new_game, Contestant, GameConfig, OrderCard, OrderCardId, PlayerColor, PlayerId, Rules,
    SeedSchedule, StartingOrderChoice,
};

use crate::client::HistoricalFrame;
use crate::games::{bring_alive, evict_down_to, mirror_of, GameError, Games};
use crate::identity::User;
use crate::store::{is_safe_id, Doc, Version, WriteError};
use crate::table::{
    now_unix, DealView, SeatDoc, TableDoc, TableStatus, TableView, TurnMirror,
};

/// The shape version stamped on every table document this build writes.
///
/// **2 since M6.2**, which added an open seat kind, an open status, a per-seat
/// display name and the lock. Every one of those is `#[serde(default)]`, so a
/// schema-1 document READS here unchanged; the number is what lets an OLDER
/// server refuse a document a newer one wrote, which is the direction that
/// cannot be made safe by defaults.
const TABLE_SCHEMA: u32 = 2;

/// The bot stream seed a THROWAWAY table is built with.
///
/// Fixed rather than drawn, and nothing observable turns on it: the only thing
/// a set table reads off `BotSeats` is `is_bot`, for the panel's badge, and no
/// bot has moved at event 0. Drawing OS entropy on every `GET …/deal` would
/// spend it on a number that reaches no payload and no record.
const THROWAWAY_BOT_SEED: u64 = 0;

/// A `u64` that crosses the wire as a STRING.
///
/// **Not fussiness — a correctness bug.** A JavaScript `number` is an IEEE
/// double and cannot hold a `u64`. [`draw_seed`] draws over the FULL range, so
/// a real game's seed is around 1.8e19, and `JSON.parse` rounds it silently:
/// the browser was already displaying `5856483359458926000`, whose last four
/// digits are not in the seed.
///
/// **Nothing round-trips a seed any more, and this type is now one-way.** It
/// used to: `/api/games/preview` drew a seed and dealt from it, the client
/// showed the deal and sent the value back to `POST /api/games`, and a rounded
/// seed started a DIFFERENT GAME from the one the player looked at — the cards
/// they chose between were not the cards they got. `preview` is retired (M6.1
/// §8) and no payload carries a seat's seed at all
/// (`no_committed_payload_carries_the_setup_seed`), so what is left is the
/// INBOUND half: `NewGame.seed` on a create, typed by a person, a fixture or a
/// `curl`.
///
/// Deserialization accepts a string or a number, so a small literal from `curl`
/// or a test still works and nothing that was correct becomes an error. The
/// string form is what a browser must send: the setup screen's seed box is a
/// text field for exactly this reason, and a client that sent its own `Number`
/// would open a different game from the one it asked for. Serialization stays a
/// string so the type cannot become lossy if something ever serves one again.
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

/// One seat of a requested table: a person's email, or the bot.
///
/// **Exactly one of the two**, refused by name otherwise. Two optional fields
/// rather than a tagged enum because that is what the browser's form produces
/// and what a `curl` writes by hand; the refusal names the seat, which a serde
/// untagged-union error would not.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeatSpec {
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub bot: bool,
}

/// What a client sends to open a table.
///
/// **`keeps` and `bot_seats` are gone** (M6.1 §4.2). A keep is now made by the
/// seat that holds it, through `POST …/keep`, over a deal that seat was served
/// on its own — so a create can no longer choose a card for anybody, and a
/// client that sends the old keys gets a 400 naming them from the existing
/// `deny_unknown_fields` fence.
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
    /// One entry per seat, in seat order.
    pub seats: Vec<SeatSpec>,
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
///
/// **The order of the checks is the order of the sentences a client reads**, and
/// the last two are M6.1's: the creator must hold a seat (R-1.6, so a table
/// cannot be opened for other people to sit at while its author watches), and a
/// bot is refused at three and four players with the evaluator's own sentence
/// (R-P10 — "Two-player only. Every corpus, fit, scenario and screen in the
/// trail is 2P", `eval.rs:1044`).
fn config_of(req: &NewGame, creator: &User) -> Result<(GameConfig, Vec<SeatDoc>), String> {
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

    if req.seats.len() != n {
        return Err(format!("{} seat entries for {n} seats", req.seats.len()));
    }
    let mut seats: Vec<SeatDoc> = Vec::with_capacity(n);
    for (i, spec) in req.seats.iter().enumerate() {
        match (&spec.user, spec.bot) {
            // Normalized HERE and nowhere else, through the one function that
            // knows what an email of this API's is — so a seat bound from a
            // header reading `  Red@Wire.TEST ` is the seat the next request
            // finds.
            (Some(raw), false) => {
                let user = User::parse(raw).map_err(|e| format!("seat {i}: {e}"))?;
                seats.push(SeatDoc::Human { user: user.as_str().to_string(), name: None });
            }
            (None, true) => seats.push(SeatDoc::Bot),
            _ => return Err(format!("seat {i}: exactly one of user or bot")),
        }
    }
    // The wizard's rule, kept: an all-bot table is refused. Bot-vs-bot
    // spectating is deliberately out of v1, and a table with nobody at it
    // would sit there waiting for a move that never comes.
    if seats.iter().all(|s| matches!(s, SeatDoc::Bot)) {
        return Err("at least one seat must stay human".to_string());
    }
    if !seats.iter().any(|s| matches!(s, SeatDoc::Human { user, .. } if user == creator.as_str()))
    {
        return Err(format!("the creator must hold a seat — {creator} is not seated"));
    }
    if n > 2 && seats.iter().any(|s| matches!(s, SeatDoc::Bot)) {
        return Err("the bot is measured at two players only".to_string());
    }

    let mut config = GameConfig::for_player_count(req.players);
    config.player_colors = req.colors.clone();
    config.start_player = PlayerId::new(req.start_seat);
    Ok((config, seats))
}

/// Is this a document every consumer may INDEX BY SEAT and BUILD A CONFIG FROM?
///
/// **The BOUNDS are the load-bearing half, and the length checks alone do not
/// give them.** Three fields carry a number this module then hands to something
/// that panics on a value it does not accept, and every one of them is reached
/// on the path that holds `table_lock` across the read — where a panic is not a
/// dropped connection but a POISONED mutex, after which every later `create`,
/// `keep` and repairing `settle` in the process dies with it.
///
/// - `players`: `TableDoc::seats_of` maps each seat's position through
///   `PlayerId::new`, which `types/ids.rs` documents as PANICKING at `id >= 4`,
///   so `players: 9` with nine seats and nine keeps satisfies every length
///   comparison and then panics on its fifth seat.
/// - `start_seat`: [`config_from`] does `PlayerId::new(doc.start_seat)`, the
///   same assertion, and [`Games::finalize`] makes that call under the lock.
///   Measured on a hand-written `start_seat: 7`: `PlayerId must be 0-3, got 7`,
///   raised from inside `Games::keep`.
/// - `keeps[i]`: [`choices_of`] does `Keep(keep - 1)` on a `u8`, so a stored
///   `0` is `attempt to subtract with overflow` in a debug build — measured, on
///   the same call.
///
/// The keep bound is `{None, Some(1), Some(2)}` and not merely non-zero,
/// because `1` and `2` are the only numbers `Games::keep` ever writes: the wire
/// numbers the two dealt cards from 1, and `cards_of`'s doc is the join. A
/// stored `3` indexes a third card that does not exist.
///
/// Only this server writes these files, so failing here means a hand-edited or
/// truncated one — which is exactly when a typed 500 naming the file beats a
/// dropped socket and a dead lock.
///
/// Checked at the two places a document ENTERS this process: [`Games::read_table`],
/// which refuses, and [`Games::list_for`], which skips and says so. Both, because
/// the listing deliberately does not go through `read_table` — one unreadable
/// file must not take the other nineteen tables off the lobby.
fn check_shape(doc: &TableDoc) -> Result<(), String> {
    if doc.schema > TABLE_SCHEMA {
        return Err(format!(
            "schema {} — this server reads up to {TABLE_SCHEMA}; a newer server wrote it",
            doc.schema
        ));
    }
    let open = doc.seats.iter().any(|s| matches!(s, SeatDoc::Open));
    if open && doc.status != TableStatus::Open {
        return Err(format!("an open seat at a table whose status is {:?}", doc.status));
    }
    if doc.status == TableStatus::Open && !open {
        return Err("status open with no open seat".to_string());
    }
    if let Some(lock) = &doc.lock {
        if lock.salt.len() != 32 || !lock.salt.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("the lock's salt is not 32 hex characters".to_string());
        }
        if lock.hash.len() != 64 || !lock.hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("the lock's hash is not 64 hex characters".to_string());
        }
    }
    for (i, seat) in doc.seats.iter().enumerate() {
        if let SeatDoc::Human { name: Some(name), .. } = seat {
            if let Err(why) = check_name(name) {
                return Err(format!("seat {i}'s name: {why}"));
            }
        }
    }
    if let Some(TurnMirror { winner: Some(w), .. }) = doc.turn {
        if usize::from(w) >= usize::from(doc.players) {
            return Err(format!("winner {w} at a {}-seat table", doc.players));
        }
    }
    if !(2..=4).contains(&doc.players) {
        return Err(format!("{} players — a table is 2 to 4", doc.players));
    }
    let n = usize::from(doc.players);
    if doc.seats.len() != n || doc.keeps.len() != n || doc.colors.len() != n {
        return Err(format!(
            "{} players, but {} seats, {} keeps and {} colours",
            doc.players,
            doc.seats.len(),
            doc.keeps.len(),
            doc.colors.len(),
        ));
    }
    if usize::from(doc.start_seat) >= n {
        return Err(format!("seat {} cannot start a {n}-seat game", doc.start_seat));
    }
    if let Some(i) = doc.keeps.iter().position(|k| !matches!(k, None | Some(1 | 2))) {
        return Err(format!(
            "seat {i} kept card {} — the dealt cards are numbered 1 and 2",
            doc.keeps[i].expect("the arm above matched a present value"),
        ));
    }
    Ok(())
}

/// The display-name rule, applied on WRITE (create, join, keep) and on READ
/// ([`check_shape`]), so a document this server wrote is never one it refuses.
///
/// Trimmed and returned rather than merely checked: the caller stores what
/// comes back, so " Red " and "Red" are one name and not two.
pub(crate) fn check_name(raw: &str) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return Err("a name is 1 to 64 characters".to_string());
    }
    if name.chars().any(char::is_control) {
        return Err("a name has no control characters".to_string());
    }
    Ok(name.to_string())
}

/// The engine config a stored table describes. The document is the authority on
/// every field of it, so a resumed finalize deals the game the table promised.
fn config_from(doc: &TableDoc) -> GameConfig {
    let mut config = GameConfig::for_player_count(doc.players);
    config.player_colors = doc.colors.clone();
    config.start_player = PlayerId::new(doc.start_seat);
    config
}

/// One contestant entry per seat, for `BotSeats`.
fn contestants_of(seats: &[SeatDoc]) -> Vec<Option<Contestant>> {
    seats.iter().map(|s| matches!(s, SeatDoc::Bot).then_some(PROVISIONAL_DEFAULT_BOT)).collect()
}

/// Every seat's starting-order choice at finalize: the human's as the seat
/// chose it, the bot's by [`bot_keep`] over the deal the seed produced.
///
/// A missing keep on a human seat is a SERVER failure and not a refusal: the
/// only caller checks `all_humans_kept` first, so reaching this arm means the
/// document and the check disagree.
fn choices_of(
    seats: &[SeatDoc],
    keeps: &[Option<u8>],
    deals: &[[OrderCardId; 2]],
    rules: &Rules,
) -> Result<Vec<StartingOrderChoice>, String> {
    deals
        .iter()
        .enumerate()
        .map(|(i, deal)| {
            let keep = match seats.get(i) {
                Some(SeatDoc::Bot) => bot_keep(deal, rules),
                _ => keeps
                    .get(i)
                    .copied()
                    .flatten()
                    .ok_or_else(|| format!("seat {i} has not chosen a starting order"))?,
            };
            Ok(StartingOrderChoice::Keep(keep - 1))
        })
        .collect()
}

/// One seat's two dealt cards, each with its Keep score.
///
/// **`cards[0]` IS card 1 and `cards[1]` is card 2.** The wire numbers the dealt
/// cards from 1 because that is how they are numbered on screen, and
/// `PendingSetup.deals` indexes them from 0 — so the join between the two is
/// this order, and it is what every `- 1` conversion in this module relies on
/// ([`bot_keep`]'s result, [`choices_of`]'s `Keep(keep - 1)`, and the human's
/// stored keep). A `filter_map` that dropped an unknown id would slide the
/// second card into the first one's place; it cannot fire, because every id here
/// came out of `new_game` against these same `rules`.
fn cards_of(deal: &[OrderCardId; 2], rules: &Rules) -> Vec<DealCard> {
    deal.iter()
        .filter_map(|id| rules.orders.get(*id).cloned())
        .map(|card| DealCard { keep_score: score_starting_order(&card, rules), card })
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
/// order, and the game this table starts shows them all. What changes is only
/// when the person looking at the screen is told.
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

/// Whose panel the set table draws when the caller has no seat left to choose
/// for: the LOWEST HUMAN seat.
///
/// **Never a bot seat, and that is the load-bearing half.** Every human hand is
/// empty on this table — no seat has chosen, so every human choice is
/// `Unknown` — which makes the answer cosmetic among humans. A bot seat's is
/// not empty: its keep is already applied, and a frame built for it would
/// unmask the one card on the set table that is nobody's business
/// ([`client::facts_for`] shows the VIEWER's hand and masks the rest).
///
/// **The `unwrap_or` is the one branch that could name a bot, and it is
/// unreachable rather than a fallback with meaning**: [`config_of`] refuses a
/// bot-only table, so `seats` always holds a human. It is written as a fallback
/// and not a panic because this runs inside a request handler under the
/// per-game mutex, where a panic poisons the game rather than refusing the call;
/// seat 0 is the safe direction if it ever is reached, because seat 0 exists at
/// every player count. Pinned from the other side by
/// `the_set_table_is_drawn_for_a_human_seat_and_never_the_bots`.
fn first_human_seat(seats: &[SeatDoc]) -> PlayerId {
    PlayerId::new(
        seats.iter().position(|s| matches!(s, SeatDoc::Human { .. })).unwrap_or(0) as u8
    )
}

/// The position after setup, as a THROWAWAY projection: the set table a player
/// chooses a starting order over.
///
/// The `Session` built here is dropped when this function returns — nothing is
/// persisted, nothing joins `Games::live`, and no id is claimed. What the caller
/// gets is built through the SAME [`HistoricalFrame`] the replay route serves,
/// so the client draws the set table with the components it already has and no
/// second frame type exists.
///
/// **One caller now: [`Games::deal`].** It had two — `preview` drew a seed,
/// dealt from it and served every seat's cards at once, and this body is what
/// they shared so the two could not disagree about what a player sees before
/// choosing. `preview` is retired (M6.1 §8): a set table exists only for a
/// table that has been opened, and a seat is served its own cards over it.
fn set_table(
    config: GameConfig,
    seed: u64,
    seats: &[SeatDoc],
    deals: &[[OrderCardId; 2]],
    rules: &Rules,
    viewer: PlayerId,
) -> Result<HistoricalFrame, String> {
    // A human seat previews as `Unknown` — no card in a hand nobody has chosen
    // from — and a bot seat at its argmax, the card the game will deal it.
    let choices: Vec<StartingOrderChoice> = deals
        .iter()
        .enumerate()
        .map(|(i, deal)| match seats.get(i) {
            Some(SeatDoc::Bot) => StartingOrderChoice::Keep(bot_keep(deal, rules) - 1),
            _ => StartingOrderChoice::Unknown,
        })
        .collect();
    let mut session = Session::new_sim(config, seed, choices, rules.clone())?;
    // `Repl::new` reads WHO plays each seat off the record's run seats, so this
    // is what makes the previewed table's `is_bot` true — the same one line
    // finalize runs, and the reason no second attachment step is needed here.
    session.set_run_seats(BotSeats::new(contestants_of(seats), THROWAWAY_BOT_SEED).seat_provenance());
    let repl = Repl::new(session);
    let mut table = HistoricalFrame::at(&repl, 0, viewer)
        .map_err(|e| format!("the setup position did not build: {}", e.message))?;
    withhold_until_the_choice(&mut table);
    Ok(table)
}

impl Games {
    /// Open a TABLE. No record, no session, nothing to play yet: the seats are
    /// bound, the seed is drawn, and every human seat is owed a deal.
    pub fn create(&self, req: &NewGame, creator: &User) -> Result<TableView, GameError> {
        // Held for the whole function. `exists` -> build -> `write` is
        // check-then-write, and two requests that both read "absent" both went
        // on to write a DIFFERENT game (the seed is drawn per request), so the
        // loser's document landed on disk and the client told 201 was silently
        // switched onto the other table.
        let _serialized = self.table_lock.lock().expect("games lock");
        let (config, seats) = config_of(req, creator).map_err(GameError::BadRequest)?;
        let seed = req.seed.map_or_else(draw_seed, |s| s.0);

        let id = match &req.name {
            Some(name) if !is_safe_id(name) => {
                return Err(GameError::BadRequest(format!("{name:?} is not a usable game name")))
            }
            Some(name) => name.clone(),
            // An INDEPENDENT draw, never `seed`. A default id of
            // `sim-{seed:016x}` published the setup seed — hidden-set member #1
            // — in the id itself, so `GET /api/games` handed out every game's
            // hidden set to anyone who could list. Dropping the seed from the
            // payloads (see `client.rs`) is undone if the id still spells it.
            None => format!("sim-{:016x}", draw_seed()),
        };
        // **An unreadable document is OCCUPIED, not absent.** The store's `read`
        // carries that distinction in its type — `Ok(None)` is `NotFound` and
        // nothing else — so reading an `Err` as "a file that exists and cannot
        // be spoken for" is what stops a new game overwriting a real one:
        // measured with a `chmod 000`, the POST returned 201 and the original
        // record was gone. BOTH documents are asked about, because either one
        // present means the name is taken — a table whose finalize failed has no
        // record, and a record whose table was hand-deleted is R-P7's legacy
        // game, which this API will not serve but must not overwrite either.
        let occupied = [Doc::Table, Doc::Record].iter().any(|doc| {
            match self.store.read(*doc, &id) {
                Ok(found) => found.is_some(),
                Err(_) => true,
            }
        });
        if occupied {
            return Err(GameError::BadRequest(format!("a game named {id:?} already exists")));
        }

        self.deleted.lock().expect("games lock").remove(&id);

        let doc = TableDoc {
            schema: TABLE_SCHEMA,
            id,
            name: req.name.clone(),
            created_by: creator.as_str().to_string(),
            created_at: now_unix(),
            players: req.players,
            colors: config.player_colors.clone(),
            start_seat: config.start_player.0,
            seats,
            keeps: vec![None; usize::from(req.players)],
            seed,
            status: TableStatus::Dealing,
            turn: None,
            // Task 3 is where a create can open a table and set a password;
            // until then every table this server opens is full and unlocked.
            lock: None,
        };
        match self.write_table(&doc, None) {
            Ok(_) => Ok(doc.view(creator)),
            Err(WriteError::Conflict { .. }) => {
                Err(GameError::BadRequest(format!("a game named {:?} already exists", doc.id)))
            }
            Err(WriteError::Io(e)) => Err(GameError::Server(e)),
        }
    }

    /// The stored table document, parsed AND shape-checked.
    ///
    /// A document that will not PARSE is a `Server` failure and never `None`:
    /// the caller's question is "does this game exist", and answering "no" for a
    /// file that is sitting right there is how a create comes to overwrite it.
    ///
    /// **The shape is checked HERE, once, because every consumer INDEXES by
    /// seat** — see [`check_shape`] for what a document that failed it would do
    /// to a handler and to `table_lock`. Checking at the places a document
    /// ENTERS this process makes every later index total by construction rather
    /// than making twenty call sites defensive.
    pub(crate) fn read_table(&self, id: &str) -> Result<Option<(TableDoc, Version)>, GameError> {
        let Some((json, version)) = self.store.read(Doc::Table, id).map_err(GameError::Server)?
        else {
            return Ok(None);
        };
        let doc: TableDoc = serde_json::from_str(&json)
            .map_err(|e| GameError::Server(format!("{id:?}'s table document does not parse: {e}")))?;
        check_shape(&doc).map_err(|why| {
            GameError::Server(format!("{id:?}'s table document is malformed: {why}"))
        })?;
        Ok(Some((doc, version)))
    }

    pub(crate) fn write_table(&self, doc: &TableDoc, expected: Option<Version>) -> Result<Version, WriteError> {
        let json = serde_json::to_string_pretty(doc)
            .map_err(|e| WriteError::Io(format!("the table document does not serialize: {e}")))?;
        self.store.write_if_version(Doc::Table, &doc.id, &json, expected)
    }

    /// The table as a seated caller may read it, REPAIRED first if it needs it.
    pub fn table(&self, id: &str, user: &User) -> Result<TableDoc, GameError> {
        let (doc, _) = self
            .settle(id)?
            .ok_or_else(|| GameError::NotFound(format!("no game named {id:?}")))?;
        if doc.seats_of(user).is_empty() {
            return Err(GameError::Forbidden("you are not seated at this table".to_string()));
        }
        Ok(doc)
    }

    /// **Self-repair, on the way past.** A table stuck at `dealing` with every
    /// human keep present is a crash between the two writes the last keep makes,
    /// and there are exactly two of them: the record exists and the status was
    /// never flipped (flip it), or the record does not exist at all (finalize).
    ///
    /// The lock is taken ONLY when there is repairing to do, so an ordinary read
    /// of a playing table serializes against nothing.
    pub(crate) fn settle(&self, id: &str) -> Result<Option<(TableDoc, Version)>, GameError> {
        let Some((doc, version)) = self.read_table(id)? else { return Ok(None) };
        if doc.status == TableStatus::Dealing && doc.all_humans_kept() {
            let _serialized = self.table_lock.lock().expect("games lock");
            return self.settle_locked(id);
        }
        Ok(Some((doc, version)))
    }

    /// [`Games::settle`] with `table_lock` already held by the caller. The
    /// document is re-read under the lock, because the writer that made the
    /// repair necessary may have finished it while this request was waiting.
    fn settle_locked(&self, id: &str) -> Result<Option<(TableDoc, Version)>, GameError> {
        let Some((doc, version)) = self.read_table(id)? else { return Ok(None) };
        if doc.status != TableStatus::Dealing || !doc.all_humans_kept() {
            return Ok(Some((doc, version)));
        }
        if self.store.read(Doc::Record, id).map_err(GameError::Server)?.is_some() {
            let handle = self.live_handle(id)?;
            let turn = mirror_of(&handle.lock().expect("a game"));
            return self.flip_to_playing(id, turn).map(Some);
        }
        self.finalize(doc).map(Some)
    }

    /// Build the game the table describes, write the record, and flip the table.
    ///
    /// **The record is written FIRST and unconditionally-absent** (`expected:
    /// None`), so two writers cannot both finalize one table: the loser's write
    /// is a `Conflict`, and it then adopts the game that landed rather than
    /// racing to overwrite it.
    fn finalize(&self, doc: TableDoc) -> Result<(TableDoc, Version), GameError> {
        let id = doc.id.clone();
        let config = config_from(&doc);
        let bots = BotSeats::new(contestants_of(&doc.seats), draw_seed());
        let pending = new_game(config.clone(), &self.rules, doc.seed)
            .map_err(|e| GameError::Server(e.to_string()))?;
        let choices = choices_of(&doc.seats, &doc.keeps, &pending.deals, &self.rules)
            .map_err(GameError::Server)?;
        let mut session = Session::new_sim(config, doc.seed, choices, (*self.rules).clone())
            .map_err(GameError::Server)?;
        session.set_run_seats(bots.seat_provenance());
        let mut repl = Repl::new(session);
        bring_alive(&mut repl);
        // `Session::to_record_json`, never `to_string_pretty(record())`: the
        // terminal digest is stamped at WRITE time, and a record written without
        // one disables the engine's provenance refusal on every game this server
        // owns.
        let json = repl.session.to_record_json().map_err(GameError::Server)?;

        let turn = match self.store.write_if_version(Doc::Record, &id, &json, None) {
            Ok(version) => {
                self.written.lock().expect("games lock").insert(id.clone(), version);
                let turn = mirror_of(&repl);
                let handle = Arc::new(Mutex::new(repl));
                // `entry().or_insert()`, never `insert()`: one game must never
                // be two handles applying moves to two divergent records.
                let mut live = self.live.lock().expect("games lock");
                live.entry(id.clone()).or_insert(handle);
                evict_down_to(&mut live, &id);
                turn
            }
            // Someone else finalized this table between the read and here. The
            // session built above is dropped unstarted and the STORED game is
            // the game — the same rule `create` follows when a name is taken.
            Err(WriteError::Conflict { .. }) => {
                let handle = self.live_handle(&id)?;
                let turn = mirror_of(&handle.lock().expect("a game"));
                turn
            }
            Err(WriteError::Io(e)) => return Err(GameError::Server(e)),
        };
        self.flip_to_playing(&id, turn)
    }

    /// Mark a table playing. The document is RE-READ rather than carried in:
    /// between the caller's read and here, the record write above may have run
    /// `persist`, which refreshes the same document's turn mirror — so the
    /// version the caller held can already be stale on a write that is not in
    /// conflict with anything.
    fn flip_to_playing(&self, id: &str, turn: TurnMirror) -> Result<(TableDoc, Version), GameError> {
        let (mut doc, version) = self
            .read_table(id)?
            .ok_or_else(|| GameError::NotFound(format!("no game named {id:?}")))?;
        doc.status = TableStatus::Playing;
        doc.turn = Some(turn);
        match self.write_table(&doc, Some(version)) {
            Ok(version) => Ok((doc, version)),
            // Another writer flipped it first. Its answer is as good as ours —
            // the record is the same record — so report what is there.
            Err(WriteError::Conflict { .. }) => self
                .read_table(id)?
                .ok_or_else(|| GameError::NotFound(format!("no game named {id:?}"))),
            Err(WriteError::Io(e)) => Err(GameError::Server(e)),
        }
    }

    /// The caller's own two cards, over the set table.
    ///
    /// The deal is REGENERATED from the config and the seed on every call rather
    /// than stored: `new_game` is a pure function of the two, so the cards a seat
    /// is shown at its tenth request are the cards it was shown at its first, and
    /// there is no second copy of the deal to keep true.
    pub fn deal(&self, id: &str, user: &User) -> Result<DealView, GameError> {
        let doc = self.table(id, user)?;
        if doc.status == TableStatus::Playing {
            return Err(GameError::Conflict("this table is playing".to_string()));
        }
        let seat = doc.next_unkept_seat_of(user);
        let config = config_from(&doc);
        let pending = new_game(config.clone(), &self.rules, doc.seed)
            .map_err(|e| GameError::Server(format!("the setup did not deal: {e}")))?;
        // The set table is drawn for the seat this caller is choosing FOR, so
        // the frame beside the two cards is that seat's own; once every seat of
        // theirs has chosen there is no such seat and the lowest human's empty
        // panel is drawn instead. Neither shows a hand — see [`first_human_seat`].
        let frame = set_table(
            config,
            doc.seed,
            &doc.seats,
            &pending.deals,
            &self.rules,
            seat.unwrap_or_else(|| first_human_seat(&doc.seats)),
        )
        .map_err(GameError::Server)?;
        Ok(DealView {
            table: doc.view(user),
            seat,
            // **The caller's seat, and no other index into `deals`.** This is
            // the whole of R-C1 on this route: a payload that carried every
            // seat's cards and let the client pick would hand the hot seat's
            // neighbour their opponent's hand in a response body.
            //
            // The index is total because `read_table` refused a document whose
            // `seats` disagrees with its `players`, and `deals` is one entry per
            // player by `new_game`'s construction.
            cards: seat.map(|s| cards_of(&pending.deals[usize::from(s.0)], &self.rules)),
            keep_note: yokohama_cli::wizard::KEEP_SCORE_NOTE,
            frame,
        })
    }

    /// One seat chooses. The LAST one finalizes.
    pub fn keep(
        &self,
        id: &str,
        user: &User,
        seat: PlayerId,
        card: u8,
    ) -> Result<TableView, GameError> {
        let _serialized = self.table_lock.lock().expect("games lock");
        // Settled first, so a table left mid-finalize by a crash is repaired by
        // the same call that would otherwise refuse a keep it has already taken.
        let (mut doc, version) = self
            .settle_locked(id)?
            .ok_or_else(|| GameError::NotFound(format!("no game named {id:?}")))?;
        if doc.status == TableStatus::Playing {
            return Err(GameError::Conflict("this table is playing".to_string()));
        }
        let n = seat.0;
        // A seat nobody holds and a seat somebody ELSE holds are the same
        // refusal, and deliberately: a caller learning which of the two it was
        // would be learning about a table it has no seat at. The bounds check
        // rides along — `seats_of` only ever names a seat this document has.
        if !doc.seats_of(user).contains(&seat) {
            return Err(GameError::Forbidden(format!("seat {n} is not yours")));
        }
        if doc.keeps[usize::from(n)].is_some() {
            return Err(GameError::Conflict(format!("seat {n} has already chosen")));
        }
        if !(1..=2).contains(&card) {
            return Err(GameError::BadRequest(format!(
                "keep {card} — the dealt cards are numbered 1 and 2"
            )));
        }
        doc.keeps[usize::from(n)] = Some(card);
        // **Written BEFORE the finalize, not with it.** The choice is the
        // player's and is durable the moment it is made; a crash between this
        // write and the record leaves a table that `settle` finishes on the next
        // read. Writing the keep only as part of a successful finalize would
        // lose every choice at a table whose last keep failed.
        match self.write_table(&doc, Some(version)) {
            Ok(_) => {}
            Err(WriteError::Conflict { .. }) => {
                return Err(GameError::Conflict(format!(
                    "{id:?} changed while this keep was in flight"
                )))
            }
            Err(WriteError::Io(e)) => return Err(GameError::Server(e)),
        }
        if doc.all_humans_kept() {
            let (doc, _) = self.finalize(doc)?;
            return Ok(doc.view(user));
        }
        Ok(doc.view(user))
    }

    /// The tables this user sits at, newest first.
    ///
    /// **A record with no table document is not a game this API serves** (R-P7):
    /// the listing is over `Doc::Table`, so a pre-M6 record is neither listed
    /// nor openable and is left on disk untouched for the Docker-archive pull.
    ///
    /// A document that cannot be read, parsed or trusted to be indexed is
    /// SKIPPED rather than failing the whole list: the lobby's job is to show
    /// the games it can, and one unreadable file must not take the other
    /// nineteen off the screen. The game itself still refuses loudly when
    /// OPENED, where the caller can act on it.
    ///
    /// **Skipped, but never silently.** A table that vanishes from its own
    /// player's lobby with nothing said anywhere is worse than one that is
    /// missing loudly: `create` will then report the name occupied, so the
    /// player can neither see the game nor reuse the name and no surface says
    /// why. Each skip writes one line naming the id and the failure, through the
    /// same `eprintln!` channel `main.rs` reports its own failures on, so a
    /// corrupt table is visible in the container log.
    pub fn list_for(&self, user: &User) -> Result<Vec<TableView>, String> {
        let mut rows: Vec<TableDoc> = Vec::new();
        for id in self.store.list(Doc::Table)? {
            let doc = match self.store.read(Doc::Table, &id) {
                Ok(Some((json, _))) => match serde_json::from_str::<TableDoc>(&json) {
                    Ok(doc) => doc,
                    Err(e) => {
                        eprintln!(
                            "{id:?}'s table document does not parse, so it is not in this \
                             listing: {e}"
                        );
                        continue;
                    }
                },
                // Absent between the listing and the read — a delete that
                // landed in the window. Nothing is wrong and nothing is lost.
                Ok(None) => continue,
                Err(e) => {
                    eprintln!(
                        "{id:?}'s table document could not be read, so it is not in this \
                         listing: {e}"
                    );
                    continue;
                }
            };
            // **The same fence `read_table` applies**, because this path
            // deliberately does not go through it — and `seats_of` two lines
            // below is exactly the call [`check_shape`] exists to make total.
            if let Err(why) = check_shape(&doc) {
                eprintln!("{id:?}'s table document is malformed, so it is not in this listing: {why}");
                continue;
            }
            if doc.seats_of(user).is_empty() {
                continue;
            }
            rows.push(doc);
        }
        // Newest first, ties broken by id so the order is total and a reload
        // does not reshuffle two tables made in the same second.
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at).then_with(|| a.id.cmp(&b.id)));
        Ok(rows.iter().map(|doc| doc.view(user)).collect())
    }

    /// Delete, and remember that we did. **The creator's, and nobody else's.**
    ///
    /// The tombstone is what stops an in-flight `input` from writing the record
    /// back after the file is gone. It is bounded by `create`, which clears the
    /// id — so the set only ever holds ids that are deleted and not recreated,
    /// and re-creating a deleted name works exactly as before.
    ///
    /// **It is a TABLE WRITER, so it holds `table_lock` like the other two.**
    /// Without it, a delete could read a table that was still dealing, and the
    /// last `keep` could then write the table and enter `finalize` before the
    /// unlink landed — leaving `finalize`'s `expected: None` record write to
    /// succeed over two documents that were already gone. The result is an
    /// orphan record: unlisted (R-P7 lists over tables), unopenable, and
    /// holding the name forever, since `create` checks both documents. Measured
    /// and reproduced in
    /// `a_delete_cannot_interleave_with_the_keep_that_finalizes`.
    ///
    /// The table is therefore read through [`Games::read_table`] and not
    /// through [`Games::table`]: `table` goes via `settle`, which takes this
    /// same non-reentrant lock when a repair is due, and calling it from here
    /// would deadlock. The two refusals it raises are restated inline so the
    /// caller reads the same sentences — and NOT settling is the right answer
    /// besides, since finalizing a table on the way to deleting it only writes
    /// the record this function then has to remove.
    pub fn delete(&self, id: &str, user: &User) -> Result<(), GameError> {
        let _serialized = self.table_lock.lock().expect("games lock");
        let (doc, _) = self
            .read_table(id)?
            .ok_or_else(|| GameError::NotFound(format!("no game named {id:?}")))?;
        if doc.seats_of(user).is_empty() {
            return Err(GameError::Forbidden("you are not seated at this table".to_string()));
        }
        if !doc.is_creator(user) {
            return Err(GameError::Forbidden(format!(
                "only {} can delete this table",
                doc.created_by
            )));
        }
        self.store.delete(id).map_err(GameError::Server)?;
        self.written.lock().expect("games lock").remove(id);
        self.live.lock().expect("games lock").remove(id);
        self.deleted.lock().expect("games lock").insert(id.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::test_support::*;
    use crate::table::Lock;
    use crate::store::{GameId, MemStore, Store};
    use yokohama_core::GameRecord;

    /// **The deal's frame is the position after setup** — a throwaway
    /// projection, never a game — with every human hand empty (nothing has been
    /// chosen) and the bot's kept card in the bot's hand; each dealt card
    /// carries its Keep score.
    ///
    /// Re-pointed from `preview`, which is retired (M6.1 §8). The claim is the
    /// route's now: a set table exists only for a table that has been OPENED,
    /// and `index`/`total` at zero is what says the position is the one before
    /// any move rather than a frame from somewhere in the game.
    ///
    /// **At the corpus seed on purpose**, so the projection is asserted against
    /// the same position the committed fixtures hold. **Which card the bot kept
    /// is not on this payload and is not asserted here (R-C1)** — a caller is
    /// served its OWN seat's cards and no others; what the projection still says
    /// out loud is that the bot HAS chosen, which is the hand count below.
    #[test]
    fn the_deal_is_the_position_after_setup() {
        let rules = Rules::standard();
        let games = games();
        let red = who("red@t.test");
        let id = games.create(&req("p"), &red).expect("a table").id;
        let d = games.deal(&id, &red).expect("red's deal");
        assert_eq!((d.frame.index, d.frame.total), (0, 0), "the position before any move");
        assert!(
            d.frame.ports.boards[0].slots.iter().all(|s| s.card.is_some()),
            "the port is full",
        );
        let players = d
            .frame
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
        let cards = d.cards.as_ref().expect("the caller's own two cards");
        assert_eq!(cards.len(), 2);
        for c in cards {
            assert_eq!(c.keep_score, yokohama_core::greedy::score_starting_order(&c.card, &rules));
        }
        // And nothing has been STARTED: the table is still dealing and there is
        // no record behind it. The projection is a throwaway.
        assert_eq!(games.table(&id, &red).unwrap().status, TableStatus::Dealing);
        assert!(games.store().read(Doc::Record, &id).unwrap().is_none(), "no record until the last keep");
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
    /// (2026-09-06, R-C3).** The set table beside a seat's two cards carries the laboratory with
    /// EMPTY slots and no `Achievements` panel at all, so the player choosing a
    /// starting order reads the same table the owner's edition has in front of
    /// them at its step 9 — before the technologies go out at step 10 and the
    /// achievements at step 11.
    ///
    /// **Read against the game the same seed CREATES**, not against a list
    /// written here: the six technology titles and the three achievement types
    /// are learned from the created game's own frame and then required to be
    /// absent from the deal's JSON. A withholding checked against a
    /// hard-coded list stops discriminating the moment the deal changes.
    ///
    /// The anti-vacuity is the port: its six order ids ARE in the deal's
    /// JSON, so this is a targeted withholding and not a blank table. And
    /// `deck_remaining` is kept — the count is public arithmetic; it is the
    /// identities that are not on the table yet.
    #[test]
    fn the_deal_withholds_the_laboratory_and_the_achievements() {
        let g = games();
        let red = who("red@t.test");
        let id = g.create(&req_seeded("withheld", ARGMAX_IS_CARD_2), &red).unwrap().id;
        let p = g.deal(&id, &red).unwrap();

        // What the SAME seed puts on a real table, read from the game itself.
        let handle = playing(&g, &req_seeded("withheld-started", ARGMAX_IS_CARD_2), &["red@t.test"]);
        let started = HistoricalFrame::at(&handle.lock().unwrap(), 0, PlayerId::new(0)).unwrap();
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
            !p.frame.labs.boards.is_empty(),
            "the laboratory board is still drawn on the set table",
        );
        assert!(
            p.frame.labs.boards.iter().all(|b| b.slots.is_empty()),
            "every laboratory slot is empty on the set table",
        );
        assert_eq!(
            p.frame.labs.deck_remaining, started.labs.deck_remaining,
            "the technology deck's COUNT is public and is kept",
        );
        // The PANEL's absence, not merely an empty card list: `achievement_names`
        // returns nothing for either, and "no panel" is the claim.
        assert!(
            !p.frame
                .frame
                .dashboard
                .iter()
                .any(|panel| matches!(panel, yokohama_cli::screen_model::Panel::Achievements(_))),
            "the set table carries no Achievements panel",
        );

        let json = serde_json::to_string(&p).unwrap();
        for title in &titles {
            assert!(!json.contains(title.as_str()), "the deal names the technology {title}");
        }
        for name in &goals {
            assert!(!json.contains(name.as_str()), "the deal names the achievement {name}");
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
                p.frame.ports.boards.iter().flat_map(|b| b.slots.iter()).any(|s| s
                    .card
                    .is_some_and(|c| c.0 == id)),
                "the port's order {id} is missing from the set table",
            );
        }
    }

    /// **Every refusal a create can raise, in the order a client reads them**,
    /// and the two that are M6.1's: the creator must hold a seat (R-1.6), and
    /// nothing is written but a DEALING table.
    ///
    /// One cell rather than five, because the ORDER is the claim: a request that
    /// is wrong in two ways must name the first thing a client can fix, and a
    /// checklist reordered by an edit would otherwise pass five cells apiece.
    #[test]
    fn create_refuses_in_the_order_a_client_reads_and_binds_the_creator() {
        let games = games();
        let red = who("red@t.test");
        let mut r = req("order");
        r.players = 5;
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("2 to 4"));
        let mut r = req("order");
        r.seats[0] = SeatSpec { user: Some("other@t.test".into()), bot: false };
        let e = games.create(&r, &red).unwrap_err().to_string();
        assert!(e.contains("creator"), "R-1.6: {e}");
        let mut r = req("order");
        r.seats[1] = SeatSpec { user: Some("x@t.test".into()), bot: true };
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("exactly one"));
        let mut r = req("order");
        r.seats = vec![SeatSpec { user: None, bot: true }, SeatSpec { user: None, bot: true }];
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("human"));
        let v = games.create(&req("order"), &red).expect("a table");
        assert_eq!(v.status, TableStatus::Dealing);
        assert_eq!(v.my_seats, vec![PlayerId::new(0)]);
        assert_eq!(v.chosen, vec![false, true], "the bot is chosen from the start");
        assert!(
            games.store().read(Doc::Record, "order").unwrap().is_none(),
            "no record until the last keep",
        );
    }

    /// **R-P10, in the evaluator's own words.** The weights were fitted at two
    /// players and nothing has measured what they do at three or four
    /// (`eval.rs:1044`), so the room refuses the seat rather than seating a bot
    /// nobody can speak for. Asserted as an EQUALITY at both counts: a refusal
    /// that merely mentioned the bot would let the sentence drift away from the
    /// ruling it carries.
    #[test]
    fn a_bot_is_refused_at_three_and_four_players_with_the_measured_sentence() {
        let games = games();
        for n in [3u8, 4] {
            let mut r = req("bot3");
            r.players = n;
            r.colors = vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green]
                [..n as usize]
                .to_vec();
            r.seats = (0..n)
                .map(|i| {
                    if i == 1 {
                        SeatSpec { user: None, bot: true }
                    } else {
                        SeatSpec { user: Some("red@t.test".into()), bot: false }
                    }
                })
                .collect();
            let e = games.create(&r, &who("red@t.test")).unwrap_err().to_string();
            assert_eq!(e, "the bot is measured at two players only", "{n}P");
        }
    }

    /// **The deal is per seat, and the other seat's cards are not in it.**
    ///
    /// The whole of R-C1 moved to the server by this cycle: the payload carries
    /// the CALLER's two cards and there is no index into anyone else's, so a hot
    /// seat's neighbour cannot read an opponent's hand out of a response body.
    /// Asserted against `new_game`'s own deal at the same seed rather than
    /// against a list written here — a deal that moved says so instead of
    /// turning this vacuous.
    #[test]
    fn the_deal_serves_the_callers_own_cards_and_never_the_others() {
        let games = games();
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        let id = games.create(&two_people("deal", CORPUS_SEED), &red).unwrap().id;
        let pending = new_game(GameConfig::for_player_count(2), &Rules::standard(), CORPUS_SEED)
            .unwrap();
        let ids = |cards: &Vec<DealCard>| cards.iter().map(|c| c.card.id).collect::<Vec<_>>();
        let d = games.deal(&id, &red).expect("red's deal");
        assert_eq!(d.seat, Some(PlayerId::new(0)));
        assert_eq!(ids(d.cards.as_ref().unwrap()), pending.deals[0].to_vec());
        assert!(
            !pending.deals[1].iter().any(|b| ids(d.cards.as_ref().unwrap()).contains(b)),
            "disjoint by construction, asserted anyway",
        );
        assert_eq!(d.keep_note, yokohama_cli::wizard::KEEP_SCORE_NOTE);
        let text = serde_json::to_string(&d).unwrap();
        assert!(!text.contains(&CORPUS_SEED.to_string()), "no seed on the wire");
        assert!(
            d.frame.labs.boards.iter().all(|b| b.slots.is_empty()),
            "withheld until the choice",
        );
        games.keep(&id, &red, PlayerId::new(0), 1).unwrap();
        let d = games.deal(&id, &red).unwrap();
        assert_eq!((d.seat, d.cards.is_none()), (None, true), "all of red's seats have kept");
        let d = games.deal(&id, &blue).unwrap();
        assert_eq!(ids(d.cards.as_ref().unwrap()), pending.deals[1].to_vec());
    }

    /// **The set table is never drawn for a BOT seat, before or after the
    /// caller has chosen** (review M1).
    ///
    /// The one hand that is not empty on this screen is the bot's: every human
    /// choice is `Unknown` at that point, while a bot seat's keep is already
    /// applied so the position can be drawn. `client::facts_for` unmasks the
    /// VIEWER's hand, so a viewer of the bot's seat would put the card the bot
    /// kept on screen — the one thing the 2026-09-06 corrections took off this
    /// wire (R-C1).
    ///
    /// **Two claims, and with `preview` retired they are reached differently
    /// (M6.1 §8).** The ROUTE's is asserted through `deal`: before red keeps,
    /// the viewer is `next_unkept_seat_of` — red's own seat, a human's. The
    /// fallback's is asserted on [`first_human_seat`] itself, because no route
    /// can reach it at a table holding a bot any more: a bot is legal at two
    /// players only (R-P10), so such a table has one human seat and that seat's
    /// keep finalizes it. `preview` used to be that reach, and always chose its
    /// viewer this way.
    ///
    /// The bot's non-empty hand is asserted as a PREMISE: without it a frame
    /// built for seat 1 would show an empty panel and this cell would pass while
    /// proving nothing.
    #[test]
    fn the_set_table_is_drawn_for_a_human_seat_and_never_the_bots() {
        use yokohama_cli::screen_model::Panel;
        let games = games();
        let red = who("red@t.test");
        let id = games.create(&req_seeded("botdeal", ARGMAX_IS_CARD_2), &red).unwrap().id;

        let check = |frame: &HistoricalFrame, when: &str| {
            let hands: Vec<(u8, bool)> = frame
                .frame
                .dashboard
                .iter()
                .filter_map(|p| match p {
                    Panel::Hand(h) => Some((h.seat.0, h.hidden)),
                    _ => None,
                })
                .collect();
            assert_eq!(hands, vec![(0, false)], "{when}: the set table is seat 0's");
            let rows: Vec<(u8, bool, usize)> = frame
                .frame
                .dashboard
                .iter()
                .filter_map(|p| match p {
                    Panel::Players(pp) => Some(pp),
                    _ => None,
                })
                .flat_map(|pp| {
                    pp.players.iter().map(|p| (p.seat.0, p.hand_hidden, p.orders_in_hand))
                })
                .collect();
            assert_eq!(rows, vec![(0, false, 0), (1, true, 1)], "{when}: {rows:?}");
        };

        check(&games.deal(&id, &red).unwrap().frame, "before the caller chooses");
        games.keep(&id, &red, PlayerId::new(0), 1).unwrap();
        // Red holds every human seat, so `next_unkept_seat_of` is now `None`.
        // The last human keep FINALIZES, so the table is playing and `deal`
        // refuses — which is itself the answer: there is no set table left to
        // draw for anyone.
        let d = games.deal(&id, &red);
        assert!(matches!(d, Err(GameError::Conflict(_))), "the table finalized on the last keep");

        // **The fallback is asserted on the function itself, because with
        // `preview` retired no ROUTE can reach it at a table holding a bot.**
        // It used to be reached through `preview`, which always chose its viewer
        // this way. A bot is legal at two players only (R-P10), so a bot table
        // has exactly one human seat and that seat's keep finalizes the table —
        // which is the refusal just above. `first_human_seat` is still what
        // `deal` falls back to at a table of two PEOPLE (the branch
        // `the_deal_serves_the_callers_own_cards_and_never_the_others` walks),
        // so the claim that it never names a bot is asserted where it lives.
        let seats = |first_is_bot: bool| {
            if first_is_bot {
                vec![SeatDoc::Bot, SeatDoc::Human { user: "red@t.test".to_string(), name: None }]
            } else {
                vec![SeatDoc::Human { user: "red@t.test".to_string(), name: None }, SeatDoc::Bot]
            }
        };
        assert_eq!(first_human_seat(&seats(true)), PlayerId::new(1), "the bot is skipped");
        assert_eq!(first_human_seat(&seats(false)), PlayerId::new(0), "and the human is taken");
    }

    /// **A keep is fenced by WHO holds the seat and by having chosen once**, and
    /// the last one starts the game.
    ///
    /// Four refusals and the finalize in one cell, because they are one
    /// sequence: every refusal is stated against a table that a moment later
    /// takes the same call from the right caller, so a fence that refused
    /// everything could not pass. The card the human chose is read out of the
    /// STARTED game's hand — the only place a keep is observable once the table
    /// has stopped carrying it.
    #[test]
    fn keep_is_fenced_by_holder_and_by_once_and_the_last_keep_finalizes() {
        let games = games();
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        let id = games.create(&two_people("keep", ARGMAX_IS_CARD_2), &red).unwrap().id;
        assert!(matches!(games.keep(&id, &blue, PlayerId::new(0), 1), Err(GameError::Forbidden(_))));
        assert!(matches!(games.keep(&id, &red, PlayerId::new(0), 3), Err(GameError::BadRequest(_))));
        assert!(matches!(games.handle(&id), Err(GameError::Dealing)));
        games.keep(&id, &red, PlayerId::new(0), 2).unwrap();
        assert!(matches!(games.keep(&id, &red, PlayerId::new(0), 1), Err(GameError::Conflict(_))));
        let v = games.keep(&id, &blue, PlayerId::new(1), 1).unwrap();
        assert_eq!(v.status, TableStatus::Playing);
        assert_eq!(v.to_move, Some(PlayerId::new(0)));
        let (json, _) = games.store().read(Doc::Record, &id).unwrap().expect("the record exists now");
        let record: GameRecord = serde_json::from_str(&json).unwrap();
        assert!(record.terminal_digest.is_some());
        let handle = games.handle(&id).unwrap();
        let pending =
            new_game(GameConfig::for_player_count(2), &Rules::standard(), ARGMAX_IS_CARD_2)
                .unwrap();
        assert_eq!(
            handle.lock().unwrap().session.state().players[0].order_cards_in_hand,
            vec![pending.deals[0][1]],
            "red kept card 2",
        );
        assert!(
            matches!(games.keep(&id, &red, PlayerId::new(0), 1), Err(GameError::Conflict(_))),
            "a playing table takes no keep",
        );
    }

    /// One dealt card's Keep score, through core's own function.
    ///
    /// **The INDEPENDENT derivation**, and the reason it is here rather than
    /// reusing [`bot_keep`]: a premise asserted from the function under test is
    /// not a premise. `bot_keep` reduced to a constant would agree with itself.
    ///
    /// The module is NOT globbed to the crate root (`game/mod.rs` lists every
    /// `pub use x::*` and `greedy` is deliberately namespaced), so the path is
    /// `yokohama_core::greedy::` and not `yokohama_core::`.
    fn keep_score(rules: &Rules, id: yokohama_core::OrderCardId) -> i64 {
        let card = rules.orders.get(id).expect("a dealt card");
        yokohama_core::greedy::score_starting_order(card, rules)
    }

    /// The bot's own Keep is what lands in the started game's hand — read at
    /// BOTH seeds, where the two answers differ.
    ///
    /// **Neither half of this may come from `bot_keep`.** One seed cannot
    /// discriminate: at [`CORPUS_SEED`] the argmax is card 1, which is the card
    /// every human seat in these helpers sends, and at [`ARGMAX_IS_CARD_2`] it
    /// is card 2 — so a `bot_keep` reduced to EITHER constant fails one of the
    /// two halves, and only both halves together say the bot chose. And each
    /// seed's premise is derived from `score_starting_order` through
    /// [`keep_score`] rather than from `bot_keep`, so a `bot_keep` that agreed
    /// with itself cannot make the premise true; a deal that MOVED says so out
    /// loud instead of quietly making the cell vacuous.
    ///
    /// The review that found the one-sided version put it exactly right: the
    /// premise `assert_eq!(bot_keep(…), 2)` reads `2 == 2` under a constant `2`,
    /// and `Keep(2-1)` then puts the expected card in the hand.
    #[test]
    fn the_bot_keeps_its_own_argmax_at_finalize() {
        let games = games();
        let rules = Rules::standard();
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
            let name = format!("argmax-{seed}");
            let handle = playing(&games, &req_seeded(&name, seed), &["red@t.test"]);
            assert_eq!(
                handle.lock().unwrap().session.state().players[1].order_cards_in_hand,
                vec![deal[expected_card - 1]],
                "seed {seed}: the bot holds the card it scored highest",
            );
        }
    }

    /// **A hot seat is a SEQUENCE and needs no button.** One person holding two
    /// seats is served the lower one, and the next only once that one has
    /// chosen — which is what retires M5.5's "Next seat" pass and its rule about
    /// clearing the DOM.
    #[test]
    fn a_hot_seat_creator_is_served_seat_zero_then_seat_one() {
        let games = games();
        let red = who("red@t.test");
        let mut r = two_people("hot", CORPUS_SEED);
        r.seats[1] = SeatSpec { user: Some("red@t.test".into()), bot: false };
        let id = games.create(&r, &red).unwrap().id;
        assert_eq!(games.deal(&id, &red).unwrap().seat, Some(PlayerId::new(0)));
        games.keep(&id, &red, PlayerId::new(0), 1).unwrap();
        assert_eq!(games.deal(&id, &red).unwrap().seat, Some(PlayerId::new(1)));
        let v = games.keep(&id, &red, PlayerId::new(1), 1).unwrap();
        assert_eq!(v.status, TableStatus::Playing);
    }

    /// **Both crash windows of the last keep, repaired on the next read.**
    ///
    /// The keep and the record are two writes and cannot be one, so a process
    /// that dies between them leaves a table at `dealing` with every human keep
    /// present — in one of exactly two states. Each arm stages the state a crash
    /// would leave and requires the next ordinary read to finish the job, which
    /// is the difference between a self-repairing table and one a person has to
    /// go and fix on the volume.
    ///
    /// Arm 1 evicts the live handle first, so the repair runs the COLD path
    /// through the store rather than reading a session this process happens to
    /// still hold.
    #[test]
    fn a_table_left_dealing_with_every_keep_present_settles_on_the_next_read() {
        // Arm 1: the record exists and the status was never flipped.
        let games = games();
        let red = who("red@t.test");
        let id = games.create(&req_seeded("settle1", CORPUS_SEED), &red).unwrap().id;
        games.keep(&id, &red, PlayerId::new(0), 1).unwrap();
        let (json, v) = games.store().read(Doc::Table, &id).unwrap().unwrap();
        let mut doc: TableDoc = serde_json::from_str(&json).unwrap();
        doc.status = TableStatus::Dealing;
        doc.turn = None;
        games
            .store()
            .write_if_version(Doc::Table, &id, &serde_json::to_string(&doc).unwrap(), Some(v))
            .unwrap();
        games.evict_for_test(&id);
        assert_eq!(games.table(&id, &red).unwrap().status, TableStatus::Playing, "settled on read");

        // Arm 2: every keep present and NO record (a crash between the two writes).
        let games = Games::new(Arc::new(Rules::standard()), Arc::new(MemStore::default()));
        let id = games.create(&req_seeded("settle2", CORPUS_SEED), &red).unwrap().id;
        let (json, v) = games.store().read(Doc::Table, &id).unwrap().unwrap();
        let mut doc: TableDoc = serde_json::from_str(&json).unwrap();
        doc.keeps[0] = Some(1);
        games
            .store()
            .write_if_version(Doc::Table, &id, &serde_json::to_string(&doc).unwrap(), Some(v))
            .unwrap();
        let t = games.table(&id, &red).unwrap();
        assert_eq!(t.status, TableStatus::Playing);
        assert!(games.store().read(Doc::Record, &id).unwrap().is_some(), "finalized on read");
    }

    /// **The list is the tables you sit at**, newest first, each saying whose
    /// turn it is — which is what lets the lobby say "your turn" without
    /// rebuilding twenty sessions (R-P3).
    ///
    /// The last two lines are R-P7: a RECORD with no table document is not a
    /// game this API serves. It is neither listed nor openable, and it is left
    /// on disk — the pre-M6 games on the volume are review material for the AI
    /// track, pulled as CLI saves, and this API must not serve them or quietly
    /// adopt them.
    #[test]
    fn the_list_is_the_tables_i_sit_at_newest_first_with_whose_turn_it_is() {
        let games = games();
        let (red, blue, green) = (who("red@t.test"), who("blue@t.test"), who("green@t.test"));
        games.create(&two_people("first", CORPUS_SEED), &red).unwrap();
        // `created_at` is unix SECONDS (R-plan-1), so two creates in one second
        // are one instant and the order would be the id tie-break rather than
        // the claim this cell makes.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let handle = playing(&games, &req_seeded("second", CORPUS_SEED), &["red@t.test"]);
        drop(handle);
        let mine = games.list_for(&red).unwrap();
        assert_eq!(
            mine.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            vec!["second", "first"],
            "newest first",
        );
        assert_eq!(
            (mine[0].status, mine[0].to_move, mine[0].over),
            (TableStatus::Playing, Some(PlayerId::new(0)), Some(false)),
        );
        assert_eq!(games.list_for(&blue).unwrap().len(), 1);
        assert!(games.list_for(&green).unwrap().is_empty());
        // A record with no table document is not a game this API serves (R-P7).
        games.store().write_if_version(Doc::Record, "legacy", "{}", None).unwrap();
        assert!(games.list_for(&red).unwrap().iter().all(|t| t.id != "legacy"));
        assert!(matches!(games.handle("legacy"), Err(GameError::NotFound(_))));
        // **And it is LEFT ON DISK**, which is the other half of the ruling and
        // the half with teeth: those records are the AI track's review material,
        // pulled as CLI saves. `delete` is the only destructive path, and it
        // refuses this id before reaching the store rather than removing a file
        // it declines to serve.
        assert!(matches!(games.delete("legacy", &red), Err(GameError::NotFound(_))));
        assert!(
            games.store().read(Doc::Record, "legacy").unwrap().is_some(),
            "a pre-M6 record is left untouched for the Docker-archive pull",
        );
    }

    /// **A document nobody could have created is a typed refusal, and it does
    /// not take the server down with it.**
    ///
    /// `players: 9` with NINE seats, nine keeps and nine colours: every length
    /// agrees with every other, so the length comparisons alone wave it through
    /// and only the 2..=4 bound catches it. Without that bound `seats_of` maps
    /// the fifth seat through `PlayerId::new(4)`, which panics — and `keep`
    /// holds `table_lock` across the read, so the panic POISONS that mutex and
    /// every later `create`, `keep` and repairing `settle` in the process dies
    /// with it.
    ///
    /// **The last line is the load-bearing one**, and it is the one a
    /// `catch_unwind` somewhere could not fake: after four calls against the
    /// broken table, a create of a DIFFERENT name still works. The `list_for`
    /// line is the other half of R-3.1 — the bad file leaves the listing, the
    /// good one stays on it, and the call still answers `Ok`.
    ///
    /// **THREE documents, because there are three unbounded fields and only one
    /// of them was bounded.** `players` was R-3.2's; the final review found
    /// `start_seat` and `keeps[i]` reaching the SAME locked path with the same
    /// panic class. `config_from` does `PlayerId::new(doc.start_seat)`, which
    /// asserts `id < 4`, and `finalize` calls it under `table_lock`; a stored
    /// `keeps: [0, …]` reaches `choices_of`'s `Keep(keep - 1)`, a debug-build
    /// overflow on the same call. Each arm is refused typed at the read, which
    /// is what makes the panic unreachable rather than caught.
    #[test]
    fn a_malformed_table_document_refuses_typed_and_leaves_the_server_alive() {
        let games = games();
        let red = who("red@t.test");
        games.create(&two_people("healthy", CORPUS_SEED), &red).expect("a neighbour table");

        let broken = serde_json::json!({
            "schema": 1,
            "id": "broken",
            "name": "broken",
            "created_by": "red@t.test",
            "created_at": 1_789_000_000u64,
            "players": 9,
            "colors": ["Red", "Blue", "Yellow", "Green", "Red", "Blue", "Yellow", "Green", "Red"],
            "start_seat": 0,
            "seats": (0..9).map(|_| serde_json::json!({ "kind": "human", "user": "red@t.test" }))
                .collect::<Vec<_>>(),
            "keeps": [null, null, null, null, null, null, null, null, null],
            "seed": CORPUS_SEED,
            "status": "dealing",
            "turn": null,
        });
        // A well-formed 2P document, one field at a time made impossible. Every
        // LENGTH agrees with `players` on both of these, exactly as it does on
        // `broken` above — so the length comparisons wave all three through and
        // only the bounds catch them.
        let two_seat = |id: &str, start_seat: u8, keeps: serde_json::Value| {
            serde_json::json!({
                "schema": 1,
                "id": id,
                "name": id,
                "created_by": "red@t.test",
                "created_at": 1_789_000_000u64,
                "players": 2,
                "colors": ["Red", "Blue"],
                "start_seat": start_seat,
                "seats": [
                    { "kind": "human", "user": "red@t.test" },
                    { "kind": "human", "user": "blue@t.test" },
                ],
                "keeps": keeps,
                "seed": CORPUS_SEED,
                "status": "dealing",
                "turn": null,
            })
        };
        let documents = [
            ("broken", broken),
            // `PlayerId::new(7)` panics, and `finalize` makes that call under
            // `table_lock`.
            ("bad-start", two_seat("bad-start", 7, serde_json::json!([null, null]))),
            // `Keep(0 - 1)` on a `u8`. Seat 0 has "chosen" card 0, which is not
            // one of the two dealt cards and was never a number `keep` accepts.
            ("bad-keep", two_seat("bad-keep", 0, serde_json::json!([0, null]))),
        ];
        for (id, doc) in &documents {
            games
                .store()
                .write_if_version(Doc::Table, id, &doc.to_string(), None)
                .expect("the store takes what it is given");

            assert!(matches!(games.table(id, &red), Err(GameError::Server(_))), "{id}: table");
            assert!(matches!(games.deal(id, &red), Err(GameError::Server(_))), "{id}: deal");
            // The path that holds `table_lock` across the read, which is the one
            // whose panic would be a poisoned mutex rather than one dropped
            // request.
            assert!(
                matches!(games.keep(id, &red, PlayerId::new(0), 1), Err(GameError::Server(_))),
                "{id}: keep",
            );
        }

        let listed = games.list_for(&red).expect("one bad file does not fail the listing");
        for (id, _) in &documents {
            assert!(listed.iter().all(|t| &t.id != id), "the malformed table {id} is skipped");
        }
        assert!(
            listed.iter().any(|t| t.id == "healthy"),
            "and the other tables are still on the screen: {:?}",
            listed.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
        );

        games
            .create(&two_people("after", CORPUS_SEED), &red)
            .expect("`table_lock` is not poisoned, so the server can still open a table");
    }

    /// Every M6.2 shape arm, read as the SENTENCE it answers with.
    ///
    /// Built in Rust rather than as JSON, and that is the point: the six arms
    /// below guard against a document some future write path produces, and a
    /// JSON literal would only prove that serde can be handed a bad string. A
    /// `TableDoc` that type-checks and is still refused is the case that
    /// matters.
    ///
    /// The fixture is valid on its own — asserted first, so a cell whose arms
    /// all fire for the wrong reason cannot pass.
    #[test]
    fn every_shape_arm_refuses_with_the_sentence_it_promises() {
        let valid = || TableDoc {
            schema: TABLE_SCHEMA,
            id: "shape".into(),
            name: Some("shape".into()),
            created_by: "red@t.test".into(),
            created_at: 1_789_000_000,
            players: 2,
            colors: vec![PlayerColor::Red, PlayerColor::Blue],
            start_seat: 0,
            seats: vec![
                SeatDoc::Human { user: "red@t.test".into(), name: Some("Red".into()) },
                SeatDoc::Human { user: "blue@t.test".into(), name: None },
            ],
            keeps: vec![None, None],
            seed: CORPUS_SEED,
            status: TableStatus::Dealing,
            turn: None,
            lock: None,
        };
        assert_eq!(check_shape(&valid()), Ok(()), "the fixture itself is well formed");

        let newer = || {
            let mut d = valid();
            d.schema = TABLE_SCHEMA + 1;
            d
        };
        let open_seat_while_dealing = || {
            let mut d = valid();
            d.seats[1] = SeatDoc::Open;
            d
        };
        let open_with_no_open_seat = || {
            let mut d = valid();
            d.status = TableStatus::Open;
            d
        };
        let short_salt = || {
            let mut d = valid();
            let mut lock = Lock::new("sesame");
            lock.salt.pop();
            d.lock = Some(lock);
            d
        };
        let long_name = || {
            let mut d = valid();
            d.seats[0] = SeatDoc::Human { user: "red@t.test".into(), name: Some("n".repeat(65)) };
            d
        };
        let control_name = || {
            let mut d = valid();
            d.seats[1] =
                SeatDoc::Human { user: "blue@t.test".into(), name: Some("bl\u{7}ue".into()) };
            d
        };
        let winner_off_the_table = || {
            let mut d = valid();
            d.status = TableStatus::Playing;
            d.turn = Some(TurnMirror { to_move: 0, over: true, winner: Some(2) });
            d
        };

        let cases: [(&str, TableDoc); 7] = [
            ("newer server", newer()),
            ("open seat", open_seat_while_dealing()),
            ("no open seat", open_with_no_open_seat()),
            ("salt", short_salt()),
            ("1 to 64", long_name()),
            ("control", control_name()),
            ("winner 2", winner_off_the_table()),
        ];
        for (fragment, doc) in cases {
            let why = check_shape(&doc).expect_err(&format!("{fragment}: must refuse"));
            assert!(why.contains(fragment), "expected {fragment:?} in the refusal, read {why:?}");
        }

        // And the far side of the open-seat pair: a table that IS open with an
        // open seat is well formed, so the two arms above fence a mismatch
        // rather than the variant.
        let mut open = valid();
        open.status = TableStatus::Open;
        open.seats[1] = SeatDoc::Open;
        assert_eq!(check_shape(&open), Ok(()), "an open table with an open seat is fine");
    }

    /// **The mirror names the winner once the game is over, and nobody before
    /// then.**
    ///
    /// `final_result` scores whatever position it is handed, so a mirror that
    /// filled `winner` unconditionally would name the current LEADER as the
    /// winner of every game in the lobby. The cell reads the stored document
    /// before the end (`None`) and after it, against the engine's own answer.
    ///
    /// The finish is played by a both-bot session built over the SAME config,
    /// seed and keeps the table describes — `Games::persist` compares record
    /// identity by seed, so this is that table's game, played out — which is
    /// `lib.rs`'s `a_finished_two_person_game` mechanism, routed through
    /// `persist` because `persist` is the subject here.
    #[test]
    fn the_mirror_names_the_winner_once_over() {
        use yokohama_cli::repl::Repl;
        use yokohama_core::{GameConfig, StartingOrderChoice};

        let games = games();
        let red = who("red@t.test");
        let _handle =
            playing(&games, &two_people("winner", CORPUS_SEED), &["red@t.test", "blue@t.test"]);
        let before = games.table("winner", &red).unwrap().turn.expect("a mirror");
        assert!(!before.over);
        assert_eq!(before.winner, None, "a game in progress has no winner");

        // The very game the table describes: `playing` keeps card 1 at every
        // human seat, which is `Keep(0)`.
        let mut config = GameConfig::for_player_count(2);
        config.player_colors = vec![PlayerColor::Red, PlayerColor::Blue];
        config.start_player = PlayerId::new(0);
        let mut session = Session::new_sim(
            config,
            CORPUS_SEED,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        )
        .expect("a game");
        let bots =
            BotSeats::new(vec![Some(PROVISIONAL_DEFAULT_BOT); 2], CORPUS_SEED);
        session.set_run_seats(bots.seat_provenance());
        let mut repl = Repl::new(session);
        for _ in 0..400 {
            if repl.session.is_over() {
                break;
            }
            repl.pump_bots();
        }
        assert!(repl.session.is_over(), "the both-bot pump must reach a finished game");

        games.persist("winner", &repl).expect("the finished record");
        let after = games.table("winner", &red).unwrap().turn.expect("a mirror");
        assert!(after.over);
        assert_eq!(
            after.winner,
            Some(repl.session.final_result().winner.0),
            "the mirror carries the engine's winner",
        );
    }

    /// A [`MemStore`] with two GATES in it, so a test can choose an
    /// interleaving rather than race for one.
    ///
    /// Each gate is armed independently and fires exactly once: it signals that
    /// a thread has ARRIVED at that operation and then blocks until the test
    /// releases it. `park_record` sits in front of a `Doc::Record` write —
    /// which is `finalize`'s — and `park_delete` inside `Store::delete`, after
    /// `Games::delete` has read the table and decided to remove it.
    ///
    /// Both channel ends tolerate a dropped peer, so a test that fails and
    /// unwinds while a thread is parked here does not turn one failure into
    /// two.
    struct RaceStore {
        inner: MemStore,
        park_record: std::sync::atomic::AtomicBool,
        park_delete: std::sync::atomic::AtomicBool,
        at_record: Mutex<std::sync::mpsc::Sender<()>>,
        go_record: Mutex<std::sync::mpsc::Receiver<()>>,
        at_delete: Mutex<std::sync::mpsc::Sender<()>>,
        go_delete: Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl RaceStore {
        fn wait(
            arrived: &Mutex<std::sync::mpsc::Sender<()>>,
            release: &Mutex<std::sync::mpsc::Receiver<()>>,
        ) {
            let _ = arrived.lock().expect("the gate").send(());
            let _ = release.lock().expect("the gate").recv();
        }
    }

    impl Store for RaceStore {
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
            if doc == Doc::Record
                && self.park_record.swap(false, std::sync::atomic::Ordering::SeqCst)
            {
                Self::wait(&self.at_record, &self.go_record);
            }
            self.inner.write_if_version(doc, id, json, expected)
        }
        fn delete(&self, id: &str) -> Result<(), String> {
            if self.park_delete.swap(false, std::sync::atomic::Ordering::SeqCst) {
                Self::wait(&self.at_delete, &self.go_delete);
            }
            self.inner.delete(id)
        }
    }

    /// **A `DELETE` may not interleave with the keep that finalizes**, because
    /// the two together leave a RECORD WITH NO TABLE DOCUMENT — a game this API
    /// will never serve (R-P7), never list, and whose NAME `create` refuses
    /// forever, since it checks both documents. Only a shell can clear it.
    ///
    /// The window, in the order it happens without the lock:
    ///
    /// 1. `delete` reads the table while it is still DEALING and NOT all-kept —
    ///    the one state in which `settle` takes no lock — and is on its way to
    ///    `store.delete`;
    /// 2. the last `keep` writes its table (now all-kept) and enters
    ///    `finalize`;
    /// 3. `delete` unlinks BOTH documents;
    /// 4. `finalize`'s record write lands. It is `expected: None`, so an absent
    ///    document is exactly what it asks for, and it SUCCEEDS.
    ///    `flip_to_playing` then 404s and the orphan stays.
    ///
    /// **Both gates are needed, and where each one sits is the proof.** The
    /// delete is parked INSIDE the store, after its read and before the unlink,
    /// which is the only point at which step 2 can slide in front of step 3;
    /// the finalize is parked before its record write, which is the only point
    /// at which step 3 can slide in front of step 4. A single gate one step
    /// earlier — on the keep's own TABLE write — proves nothing about this
    /// defect: the delete then unlinks first and the keep's CONDITIONAL table
    /// write conflicts harmlessly. A single gate one step later, on the record
    /// write alone, proves nothing either: by then the stored table reads
    /// all-kept, so even the unfixed `settle` takes the lock and blocks.
    ///
    /// **The delete is started FIRST and waited for**, so the lock is
    /// unambiguously its. With the lock taken, the keep cannot reach its record
    /// write at all while the delete is parked, which is the middle assertion;
    /// without it, the keep sails past and the orphan is the one at the end.
    /// Measured on the code before the fix: the record write arrives, and
    /// `Doc::Record` is still in the store after both threads have finished.
    #[test]
    fn a_delete_cannot_interleave_with_the_keep_that_finalizes() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::mpsc::{channel, RecvTimeoutError};
        use std::time::Duration;

        let (at_record_tx, at_record_rx) = channel();
        let (go_record_tx, go_record_rx) = channel();
        let (at_delete_tx, at_delete_rx) = channel();
        let (go_delete_tx, go_delete_rx) = channel();
        let store = Arc::new(RaceStore {
            inner: MemStore::default(),
            park_record: AtomicBool::new(false),
            park_delete: AtomicBool::new(false),
            at_record: Mutex::new(at_record_tx),
            go_record: Mutex::new(go_record_rx),
            at_delete: Mutex::new(at_delete_tx),
            go_delete: Mutex::new(go_delete_rx),
        });
        let games =
            Arc::new(Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>));
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        let id = games.create(&two_people("raced", CORPUS_SEED), &red).expect("a table").id;
        // Seat 0 chooses. The stored table now reads `keeps: [1, null]`:
        // DEALING and NOT all-kept, which is the state `settle` passes over
        // without taking the lock — step 1's precondition.
        games.keep(&id, &red, PlayerId::new(0), 1).expect("red's keep");

        store.park_delete.store(true, Ordering::SeqCst);
        store.park_record.store(true, Ordering::SeqCst);

        let deleter = {
            let (games, id, red) = (Arc::clone(&games), id.clone(), red.clone());
            std::thread::spawn(move || games.delete(&id, &red))
        };
        at_delete_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the delete reaches the store with the table read and the unlink pending");

        let keeper = {
            let (games, id, blue) = (Arc::clone(&games), id.clone(), blue.clone());
            std::thread::spawn(move || games.keep(&id, &blue, PlayerId::new(1), 1))
        };

        assert!(
            matches!(
                at_record_rx.recv_timeout(Duration::from_millis(500)),
                Err(RecvTimeoutError::Timeout),
            ),
            "the finalizing keep reached its record write while a delete of the same table was \
             mid-flight — which is the interleaving that writes a record over two documents that \
             are about to be unlinked",
        );

        // Released in the order the threads can take them. The record gate's
        // send is buffered, so it works whether the keep is parked on it or
        // still waiting for `table_lock`.
        go_delete_tx.send(()).expect("the parked delete is still listening");
        go_record_tx.send(()).expect("a buffered release");
        deleter.join().expect("the delete thread").expect("the delete");
        // The keep's own outcome is NOT asserted: with the lock taken it finds
        // the table gone and refuses in conflict, which is the honest answer to
        // "the game you were choosing at has been deleted". What matters is
        // what it left behind.
        let _ = keeper.join().expect("the keep thread");

        assert!(
            store.read(Doc::Record, &id).expect("a readable store").is_none(),
            "an orphan record: unlisted, unopenable, and holding the name",
        );
        assert!(store.read(Doc::Table, &id).expect("a readable store").is_none());
        assert!(matches!(games.table(&id, &red), Err(GameError::NotFound(_))));
        assert!(matches!(games.handle(&id), Err(GameError::NotFound(_))));
        // The consequence a player would actually meet. `create` checks BOTH
        // documents, so an orphan record makes the name permanently occupied.
        games.create(&two_people("raced", CORPUS_SEED), &red).expect("the name is reusable");
    }

    /// **Delete is the CREATOR's**, and a seated player is not a creator.
    ///
    /// Asserted through a seated caller rather than a stranger, because a
    /// stranger is refused one step earlier (not seated) and would pass a
    /// `delete` with no creator check in it at all.
    #[test]
    fn delete_is_the_creators() {
        let games = games();
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        let id = games.create(&two_people("del", CORPUS_SEED), &red).unwrap().id;
        assert!(matches!(games.delete(&id, &blue), Err(GameError::Forbidden(_))));
        games.delete(&id, &red).unwrap();
        assert!(matches!(games.table(&id, &red), Err(GameError::NotFound(_))));
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
                        let creator = who("red@t.test");
                        // The winner finishes its own deal, because the RECORD
                        // is what the assertions below read and a table has none
                        // until its last keep. The loser never gets here.
                        games.create(&r, &creator).map(|view| {
                            let id = view.id;
                            while let Some(seat) = games
                                .table(&id, &creator)
                                .expect("seated")
                                .next_unkept_seat_of(&creator)
                            {
                                games.keep(&id, &creator, seat, 1).expect("a keep");
                            }
                            let handle = games.handle(&id).expect("playing");
                            let seed = handle.lock().expect("a handle").session.record().seed;
                            (id, seed)
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
        let err = games
            .create(&req("taken"), &who("red@t.test"))
            .expect_err("the name is occupied")
            .to_string();
        assert!(err.contains("already exists"), "{err}");
    }
}
