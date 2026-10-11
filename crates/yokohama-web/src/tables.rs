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
//! windows are repaired on the way past the next read (`Games::settle`)
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

// `check_shape` under an alias: this module has one of its own, for the TABLE
// document, and two rules with one name in one file is how the wrong one comes
// to be called.
use crate::chat::{
    check_shape as check_chat_shape, check_text, Author, ChatCounts, ChatDoc, ChatRefusal, Message, MessageView,
    HOUSE,
};
use crate::client::{HistoricalFrame, Viewer};
use crate::games::{bring_alive, enter, evict_down_to, mirror_of, GameError, Games};
use crate::identity::User;
use crate::store::{is_safe_id, Doc, Version, WriteError};
use crate::table::{DealView, Lock, SeatDoc, TableDoc, TableStatus, TableView, TurnMirror};
use yokohama_core::GameRecord;

/// The shape version stamped on every table document this build writes.
///
/// **2 since M6.2**, which added an open seat kind, an open status, a per-seat
/// display name and the lock. Every one of those is `#[serde(default)]`, so a
/// schema-1 document READS here unchanged; the number is what lets an OLDER
/// server refuse a document a newer one wrote, which is the direction that
/// cannot be made safe by defaults.
const TABLE_SCHEMA: u32 = 3;

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
                .map_err(|e| serde::de::Error::custom(format!("\"{t}\" is not a u64 seed: {e}", t = crate::echo::echo(&t)))),
        }
    }
}

/// One seat of a requested table: a person's key (an email, or a plain name),
/// the bot, or nobody yet.
///
/// **Exactly one of the THREE**, refused by name otherwise. Three optional
/// fields rather than a tagged enum because that is what the browser's form
/// produces and what a `curl` writes by hand; the refusal names the seat, which
/// a serde untagged-union error would not.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeatSpec {
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub bot: bool,
    /// An unfilled seat (M6.2, R-2.7): anyone signed in may take it.
    #[serde(default)]
    pub open: bool,
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
    /// A LABEL since M6.2 (R-2.4): free text, 1–64 characters, what the lobby
    /// prints. **Never the id** — it used to be both, which is why a game could
    /// not be called "Thursday's game" and why two tables could not share a
    /// name.
    #[serde(default)]
    pub name: Option<String>,
    /// The storage id, under [`is_safe_id`]; absent, the server draws one. Tests
    /// and the fixture harness set it, and since M6.5b the browser's Worker sets it
    /// on every LOCAL create (`local-<16 hex>`, R-6.5b.12); a hosted create never does.
    #[serde(default)]
    pub id: Option<String>,
    /// Written onto every seat the creator holds (R-2.3), and onto no other —
    /// a name is a claim about the person who gave it.
    #[serde(default)]
    pub display_name: Option<String>,
    /// The table password (R-2.1); stored as a [`crate::table::Lock`], never
    /// served. Trimmed here, so " sesame " and "sesame" are one password.
    #[serde(default)]
    pub password: Option<String>,
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
        match (&spec.user, spec.bot, spec.open) {
            // Normalized HERE and nowhere else, through the one function that
            // knows what a key of this API's is — so a seat bound from a
            // header reading `  Red@Wire.TEST ` is the seat the next request
            // finds. A plain name is a key here under every mode (M6.5c2); only
            // a CALLER is held to the mode's rule.
            (Some(raw), false, false) => {
                let user = User::parse(raw).map_err(|e| format!("seat {i}: {e}"))?;
                seats.push(SeatDoc::Human { user: user.as_str().to_string(), name: None, bound_at_create: true });
            }
            (None, true, false) => seats.push(SeatDoc::Bot),
            (None, false, true) => seats.push(SeatDoc::Open),
            _ => return Err(format!("seat {i}: exactly one of user, bot or open")),
        }
    }
    // The wizard's rule, kept, with its predicate WIDENED for M6.2: a table
    // with no human seat is refused. Bot-vs-bot spectating is deliberately out
    // of v1, and a table with nobody at it would sit there waiting for a move
    // that never comes. **An open seat is not a human seat** — it is nobody
    // yet — so "every seat is a bot" is no longer the whole of the rule.
    if !seats.iter().any(|s| matches!(s, SeatDoc::Human { .. })) {
        return Err("at least one seat must stay human".to_string());
    }
    if !seats.iter().any(|s| matches!(s, SeatDoc::Human { user, .. } if user == creator.as_str()))
    {
        return Err(format!("the creator must hold a seat — {creator} is not seated"));
    }
    if n > 2 && seats.iter().any(|s| matches!(s, SeatDoc::Bot)) {
        return Err("the bot is measured at two players only".to_string());
    }
    // M6.2's three, in the order §3.1 lists them. The id's own check is
    // `create`'s and comes after these — it is the only one that needs the
    // store's rule rather than this module's.
    if let Some(raw) = &req.name {
        check_name(raw).map_err(|e| format!("name: {e}"))?;
    }
    let display_name = match &req.display_name {
        Some(raw) => Some(check_name(raw).map_err(|e| format!("display name: {e}"))?),
        None => None,
    };
    if let Some(raw) = &req.password {
        let p = raw.trim();
        if p.is_empty() {
            return Err("a password is at least one character".to_string());
        }
        if p.chars().count() > 64 {
            return Err("a password is at most 64 characters".to_string());
        }
    }
    // **Onto the creator's seats and no other** (R-plan-6). A create may seat
    // other people, and writing the creator's chosen name onto their seats
    // would be this server making a claim about somebody who has not spoken.
    if let Some(name) = &display_name {
        for seat in &mut seats {
            if let SeatDoc::Human { user, name: slot, .. } = seat {
                if user == creator.as_str() {
                    *slot = Some(name.clone());
                }
            }
        }
    }

    let mut config = GameConfig::for_player_count(req.players);
    config.player_colors = req.colors.clone();
    config.start_player = PlayerId::new(req.start_seat);
    Ok((config, seats))
}

/// Is this a document every consumer may INDEX BY SEAT and BUILD A CONFIG FROM?
///
/// **The BOUNDS are the load-bearing half, and the length checks alone do not
/// give them.** Four fields carry a number this module then hands to something
/// that panics on a value it does not accept, and every one of them is reached
/// on a path that holds a TABLE LOCK across the read — where a panic is not a
/// dropped connection but a POISONED mutex, after which every later `keep`,
/// `join`, `delete` and repairing `settle` AT THAT TABLE dies with it.
///
/// **One of the four is wider than one table, so "one dead game" is not the
/// ceiling.** [`Games::list_for`] calls [`TableDoc::view`] on every row it
/// keeps, and `view` hands the turn mirror to `PlayerId::new` — so an
/// out-of-range `to_move` on ONE file drops `GET /api/games` for every player,
/// which the lobby polls every 3 s, with no `CatchPanicLayer` in the crate to
/// turn it into a 500. The per-table lock M6.2 brought narrowed the OTHER
/// three (`create`, still on the global `table_lock`, is not touched at all);
/// it did not narrow this one.
///
/// - `players`: `TableDoc::seats_of` maps each seat's position through
///   `PlayerId::new`, which `types/ids.rs` documents as PANICKING at `id >= 4`,
///   so `players: 9` with nine seats and nine keeps satisfies every length
///   comparison and then panics on its fifth seat.
/// - `start_seat`: `config_from` does `PlayerId::new(doc.start_seat)`, the
///   same assertion, and `Games::finalize` makes that call under the lock.
///   Measured on a hand-written `start_seat: 7`: `PlayerId must be 0-3, got 7`,
///   raised from inside `Games::keep`.
/// - `keeps[i]`: `choices_of` does `Keep(keep - 1)` on a `u8`, so a stored
///   `0` is `attempt to subtract with overflow` in a debug build — measured, on
///   the same call.
/// - `turn`: [`TableDoc::view`] does `PlayerId::new` on BOTH `to_move` and
///   `winner`. Measured on a `Playing` document with `to_move: 9` before the
///   arm existed: `check_shape=Ok(())` and then `PlayerId must be 0-3, got 9`.
///
/// The keep bound is `{None, Some(1), Some(2)}` and not merely non-zero,
/// because `1` and `2` are the only numbers `Games::keep` ever writes: the wire
/// numbers the two dealt cards from 1, and `cards_of`'s doc is the join. A
/// stored `3` indexes a third card that does not exist.
///
/// A file in this server's store is written only by this server, so failing on
/// one means a hand-edited or truncated file — which is exactly when a typed
/// 500 naming the file beats a dropped socket and a dead lock. A document a
/// browser SENDS is not one of those files, and failing on it refuses what was
/// sent.
///
/// Checked wherever a document ENTERS this process. From the store:
/// `Games::read_table`, which refuses, and [`Games::list_for`], which skips
/// and says so — both, because the listing deliberately does not go through
/// `read_table`, and one unreadable file must not take the other nineteen
/// tables off the lobby. From a browser: the backup module's pair check
/// ([`crate::backup::check_pair`]), over a pair the archive route receives or
/// a backup restores, and its `check_table_alone`, over a restored table still
/// at the deal.
///
/// `pub` since M6.5c (R-6.5c.8): a host refuses at the source a copy its lobby
/// would drop — the house's own check (`crates/yokohama-worker`'s
/// `house_copy_inner`) — because the listing's line arrives only once the row
/// is gone.
pub fn check_shape(doc: &TableDoc) -> Result<(), String> {
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
    // **M6.3's list, bounded on the same reasoning as the four numbers below
    // it.** Nothing here is handed to a panicking constructor, so the ceiling
    // is about SIZE rather than about a panic: `Games::unlock` refuses the
    // 257th entry, and a hand-written file carrying ten thousand is a document
    // this server would go on reading, writing and re-serializing on every
    // join at that table. Each entry must parse as a `User` — an email or a
    // plain name (M6.5c2) — for the plainer reason that `can_watch` compares
    // it against one: an entry that never normalizes is an entry no caller can
    // ever match, written as though it admitted somebody.
    if doc.unlocked.len() > UNLOCKED_CAP {
        return Err(format!("{} watchers — a table remembers {UNLOCKED_CAP}", doc.unlocked.len()));
    }
    if let Some(bad) = doc.unlocked.iter().find(|u| User::parse(u).is_err()) {
        return Err(format!("\"{bad}\" on the watcher list is neither an email nor a name", bad = crate::echo::echo(bad)));
    }
    // M6.5a's stamp, bounded like a message's `created_at` (R-6.5a.10).
    if let Some(at) = doc.archived_at {
        if at > crate::chat::MAX_CREATED_AT {
            return Err(format!("archived_at {at} is past {}", crate::chat::MAX_CREATED_AT));
        }
    }
    for (i, seat) in doc.seats.iter().enumerate() {
        if let SeatDoc::Human { name: Some(name), .. } = seat {
            if let Err(why) = check_name(name) {
                return Err(format!("seat {i}'s name: {why}"));
            }
        }
    }
    // **The table's own name, by the same rule as a seat's** (security S1b,
    // finding D25): `create` and `join` apply it on write, and a document that
    // arrives through the archive route or a restore never passed through them.
    if let Some(name) = &doc.name {
        if let Err(why) = check_name(name) {
            return Err(format!("the table's name: {why}"));
        }
    }
    // BOTH mirror fields, because both are handed to `PlayerId::new` by
    // `TableDoc::view` (`table.rs`), which asserts `id < 4` and PANICS — and
    // `view` runs under the per-table lock in `join` and `keep` (poisoning that
    // table's mutex) and inside `Games::list_for`, which builds a row for every
    // table on the volume. One hand-edited `to_move` there drops the lobby poll
    // for every player, not one game.
    if let Some(&TurnMirror { to_move, winner, .. }) = doc.turn.as_ref() {
        if usize::from(to_move) >= usize::from(doc.players) {
            return Err(format!("to_move {to_move} at a {}-seat table", doc.players));
        }
        if let Some(w) = winner {
            if usize::from(w) >= usize::from(doc.players) {
                return Err(format!("winner {w} at a {}-seat table", doc.players));
            }
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

/// Which chat document a read or a write means (M6.4, R-6.4.7) — a table's, or
/// the house's.
///
/// **The house rule lives HERE and nowhere else** (M6.5c2 cycle D, row 130):
/// the house is [`Doc::House`] at [`HOUSE`] with no seat bound, and a table is
/// [`Doc::Chat`] at its own id, bounded by its player count. That was an
/// `(id, players)` pair whose `None` meant the house — so the house branch
/// carried an `id` nothing read, and "no player count means the house" was
/// stated once in the read and again in the write.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ChatAt<'a> {
    /// The table `id`'s chat, whose authors hold seats below `players`.
    Table { id: &'a str, players: u8 },
    /// The lobby's one chat document, where nobody holds a seat.
    House,
}

impl<'a> ChatAt<'a> {
    fn doc(self) -> Doc {
        match self {
            Self::Table { .. } => Doc::Chat,
            Self::House => Doc::House,
        }
    }

    fn key(self) -> &'a str {
        match self {
            Self::Table { id, .. } => id,
            Self::House => HOUSE,
        }
    }

    fn players(self) -> Option<u8> {
        match self {
            Self::Table { players, .. } => Some(players),
            Self::House => None,
        }
    }
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
/// for — they have kept at every seat they hold and are WAITING on the others:
/// the caller's own LOWEST seat (M6.5c2 cycle D, straw poll row 6).
///
/// It was the table's lowest human seat, so blue (seat 1) waiting on red was
/// drawn red's seat 0. Nothing leaked — every human hand on this table is
/// `Unknown` until the game starts — but the waiting screen was somebody
/// else's table, one keep after it had been the caller's own.
///
/// **Never a bot seat**: [`TableDoc::seats_of`] names only the seats this user
/// holds as a person. **The empty case is unreachable** — [`Games::deal`] reads
/// the table through [`Games::table`], which refuses a caller with no seat — and
/// falls back to [`first_human_seat`] rather than panicking, for that
/// function's reason: this runs in a request handler.
pub(crate) fn seat_of_waiting(doc: &TableDoc, user: &User) -> PlayerId {
    doc.seats_of(user).first().copied().unwrap_or_else(|| first_human_seat(&doc.seats))
}

/// The LOWEST HUMAN seat — [`seat_of_waiting`]'s fallback for a caller holding
/// no seat, which no route reaches.
///
/// **Never a bot seat, and that is the load-bearing half.** Every human hand is
/// empty on the set table — no seat has chosen, so every human choice is
/// `Unknown`. A bot seat's is not empty: its keep is already applied, and a
/// frame built for it would unmask the one card on the set table that is
/// nobody's business ([`client::facts_for`] shows the VIEWER's hand and masks
/// the rest).
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
    let mut repl = Repl::new(session);
    let mut table = HistoricalFrame::at(&mut repl, 0, Viewer::Seat(viewer))
        .map_err(|e| format!("the setup position did not build: {e}"))?;
    withhold_until_the_choice(&mut table);
    Ok(table)
}

/// Which question `GET /api/games` is asking (§3.3).
///
/// Three scopes and not a filter grammar: a lobby has exactly three lists on it
/// — the tables I sit at, the tables I could sit at, and (M6.3) the games I
/// could watch — and a general predicate over table documents would be a query
/// language nobody asked for, with a cost per row that grows without anything
/// bounding it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Every table this caller holds a seat at, at every status.
    Mine,
    /// Every table still filling its seats, whoever made it (R-2.7).
    Open,
    /// Every game under way or finished, whoever made it (M6.3, R-3.9) — drawn
    /// as "Games in progress", so a watcher can find one without a link.
    ///
    /// **"Over" is in it, and that is the status rule and not a choice made
    /// here**: over is not a `TableStatus` (`table.rs`), it is a `Playing`
    /// table whose session says so, so `status == Playing` IS "playing or
    /// over". A row's `over` and `winner` come off the turn mirror, which is
    /// what lets the lobby say "over · `<name>` won" without rebuilding a
    /// session.
    ///
    /// The row does NOT promise the caller may watch: a locked table is listed
    /// (seeing that a game exists was never what the lock gated, R-3.5) and its
    /// `can_watch` reads false, which is the unlock face.
    Playing,
}

impl std::str::FromStr for Scope {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "mine" => Ok(Self::Mine),
            "open" => Ok(Self::Open),
            "playing" => Ok(Self::Playing),
            other => Err(format!(
                "no scope named \"{other}\" — the three are mine, open and playing",
                other = crate::echo::echo(other)
            )),
        }
    }
}

/// How many watchers one table remembers (M6.3, §3.4). The 257th unlock is a
/// 409 rather than a silent eviction: a list that forgot its oldest entry would
/// quietly re-lock a watcher mid-game, and a table with 256 people watching it
/// is a state whose owner should hear about it.
pub const UNLOCKED_CAP: usize = 256;

/// The sentence a seeded create is refused with at a table that seats
/// somebody else (R-SEC8(b), finding B3; [`crate::games::SeedPolicy`]).
pub const SEED_NOT_SOLO: &str =
    "a seed is honoured only at a table whose every human seat is yours — leave it blank and the server draws one";

/// The sentence a creator-chosen id with a dot is refused with (security S1c,
/// finding B9): a cache keys a path's last dot-suffix as a file type, and a
/// table named `evening.js` is a private frame the edge would keep. The
/// storage rule (`store::is_safe_id`) still admits dots, so a table stored
/// before this cycle loads; only a CREATE refuses them.
pub const DOTTED_ID: &str = "is not a usable game id: a dot names a file type to a cache, and a table's id has none";

pub const LIST_LIMIT_DEFAULT: usize = 20;
pub const LIST_LIMIT_MAX: usize = 100;

/// **Whether the creator's Abandon SAVES this table before deleting it** (the
/// M6.5a rule, R-6.5a.11, factored out in cycle E for R-E5): the host keeps an
/// archive, the table is `Playing` (dealt; over is a playing table whose
/// session says so), it carries no `archived_at` stamp (a game archived at
/// its end is not put twice), and the game has at least one event.
///
/// ONE function with two callers that hand it the event fact from different
/// places, on purpose (ruling P-3, as amended by T5-a): [`Games::delete`]
/// passes whether the RECORD it reads holds an event — the truth, because the
/// decision to file a save must never rest on a cache — and
/// [`TableDoc::view`] passes [`crate::table::TurnMirror::has_events`], the
/// cache a lobby row can read without the record. They part only where the
/// mirror is stale (`mirror_turn` swallows a conflict), and then the view is
/// the one that is wrong.
pub(crate) fn saves_on_abandon(doc: &TableDoc, has_events: bool, archive_on: bool) -> bool {
    archive_on && doc.status == TableStatus::Playing && doc.archived_at.is_none() && has_events
}

/// The cursor naming this row: `<created_at>,<id>`.
///
/// Both halves, because `created_at` is unix SECONDS and twenty tables made in
/// one second share one — a cursor of the timestamp alone could not say which
/// of them the caller had already seen.
fn cursor_of(doc: &TableDoc) -> String {
    format!("{},{}", doc.created_at, doc.id)
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
        // `enter`, poisoned or not: the id space's lock guards no data.
        let _serialized = enter(&self.table_lock);
        let (config, seats) = config_of(req, creator).map_err(GameError::BadRequest)?;
        // **A creator's seed is honoured at a SOLO table only** (R-SEC8(b),
        // finding B3): every human seat the creator's and none open, else the
        // creator knows every deal and both decks. The fixtures opt into
        // `SeedPolicy::Diagnostic` by name; nothing a person reaches does.
        if req.seed.is_some() && self.seed_policy == crate::games::SeedPolicy::SoloOnly {
            let somebody_else = seats.iter().any(|s| match s {
                SeatDoc::Open => true,
                SeatDoc::Human { user, .. } => user.as_str() != creator.as_str(),
                SeatDoc::Bot => false,
            });
            if somebody_else {
                return Err(GameError::BadRequest(SEED_NOT_SOLO.to_string()));
            }
        }
        let seed = req.seed.map_or_else(draw_seed, |s| s.0);

        let id = match &req.id {
            Some(id) if !is_safe_id(id) => {
                return Err(GameError::BadRequest(format!("\"{id}\" is not a usable game id", id = crate::echo::echo(id))))
            }
            Some(id) if id.contains('.') => {
                return Err(GameError::BadRequest(format!("\"{}\" {DOTTED_ID}", crate::echo::echo(id))))
            }
            Some(id) => id.clone(),
            // An INDEPENDENT draw, never `seed`. A default id of
            // `sim-{seed:016x}` published the setup seed — hidden-set member #1
            // — in the id itself, so `GET /api/games` handed out every game's
            // hidden set to anyone who could list. Dropping the seed from the
            // payloads (see `client.rs`) is undone if the id still spells it.
            None => format!("sim-{:016x}", draw_seed()),
        };
        // **An unreadable document REFUSES the name, and not as "absent".** The
        // store's `read` carries that distinction in its type — `Ok(None)` is
        // `NotFound` and nothing else — so an `Err` is never read as a free
        // name: that is what stops a new game overwriting a real one, measured
        // with a `chmod 000` (the POST returned 201 and the original record was
        // gone). BOTH documents are asked about, because either one present
        // means the name is taken — a table whose finalize failed has no
        // record, and a record whose table was hand-deleted is R-P7's legacy
        // game, which this API will not serve but must not overwrite either.
        //
        // **And an `Err` is the STORE's failure, a 500 naming its error**
        // (M6.5c2 cycle D, straw poll row 7). It was folded into "already
        // exists", a 400: a sentence that asserts a game nobody could read, and
        // a status telling the caller to change a request that was right.
        for doc in [Doc::Table, Doc::Record] {
            match self.store.read(doc, &id) {
                Ok(None) => {}
                Ok(Some(_)) => {
                    return Err(GameError::BadRequest(format!("a game named \"{id}\" already exists", id = crate::echo::echo(&id))))
                }
                Err(e) => {
                    return Err(GameError::Server(format!("the store could not be read for \"{id}\": {e}", id = crate::echo::echo(&id))))
                }
            }
        }

        self.deleted.lock().expect("games lock").remove(&id);

        // **The status is a function of the seats and not of the request.**
        // `check_shape` refuses the pair in both directions, so a table that is
        // open exactly while it has an open seat is the only shape this server
        // can write and the only one it can read back.
        let status = if seats.iter().any(|s| matches!(s, SeatDoc::Open)) {
            TableStatus::Open
        } else {
            TableStatus::Dealing
        };
        let doc = TableDoc {
            schema: TABLE_SCHEMA,
            id,
            name: req.name.as_deref().map(str::trim).map(str::to_string),
            created_by: creator.as_str().to_string(),
            created_at: (self.clock)(),
            players: req.players,
            colors: config.player_colors.clone(),
            start_seat: config.start_player.0,
            seats,
            keeps: vec![None; usize::from(req.players)],
            seed,
            status,
            turn: None,
            // Trimmed the same way `config_of` validated it: `Lock::new` is
            // unguarded by design, so the rule that an empty password is
            // refused lives there and the trim must agree with it here.
            lock: req.password.as_deref().map(str::trim).map(Lock::new),
            unlocked: Vec::new(),
            archived_at: None,
        };
        match self.write_table(&doc, None) {
            Ok(_) => {
                // A name reopened starts its chat at nothing. `delete` drops the
                // entry too, so in that path this is belt and braces; it is the
                // one that still holds when a document was left behind by some
                // other route, which is the same reason the tombstone set is
                // cleared a few lines above.
                self.chat_counts.lock().expect("games lock").remove(&doc.id);
                self.signal.bump(&doc.id);
                Ok(doc.view(creator, self.archive_on()))
            }
            Err(WriteError::Conflict { .. }) => {
                Err(GameError::BadRequest(format!("a game named \"{}\" already exists", crate::echo::echo(&doc.id))))
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
    /// to a handler and to that table's lock. Checking at the places a document
    /// ENTERS this process makes every later index total by construction rather
    /// than making twenty call sites defensive.
    pub(crate) fn read_table(&self, id: &str) -> Result<Option<(TableDoc, Version)>, GameError> {
        let Some((json, version)) = self.store.read(Doc::Table, id).map_err(GameError::Server)?
        else {
            return Ok(None);
        };
        let doc: TableDoc = serde_json::from_str(&json)
            .map_err(|e| GameError::Server(format!("\"{id}\"'s table document does not parse: {e}", id = crate::echo::echo(id))))?;
        check_shape(&doc).map_err(|why| {
            GameError::Server(format!("\"{id}\"'s table document is malformed: {why}", id = crate::echo::echo(id)))
        })?;
        Ok(Some((doc, version)))
    }

    /// **Absent before locked** (security S1c, finding B8): the one pre-read the
    /// six writers that always lock make before [`Games::with_table_lock`] —
    /// `join`, `keep`, `unlock`, `delete` and the chat's append and delete — so
    /// a fabricated id is a 404 that never creates a lock entry. The read under
    /// the lock is the one a write is checked against; this one only says
    /// "there". `settle` reads for itself and locks only to repair.
    fn require_table(&self, id: &str) -> Result<(), GameError> {
        self.read_table(id)?.ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
        Ok(())
    }

    pub(crate) fn write_table(&self, doc: &TableDoc, expected: Option<Version>) -> Result<Version, WriteError> {
        let json = serde_json::to_string_pretty(doc)
            .map_err(|e| WriteError::Io(format!("the table document does not serialize: {e}")))?;
        self.store.write_if_version(Doc::Table, &doc.id, &json, expected)
    }

    // -----------------------------------------------------------------------
    // M6.4 — the chat (spec §2.4, R-6.4.7, R-6.4.8, R-6.4.9, R-6.4.15)
    // -----------------------------------------------------------------------

    /// The stored chat document at `at`, parsed, shape-checked and NORMALIZED.
    ///
    /// **An absent document is an EMPTY one and never a refusal**, unlike
    /// [`Games::read_table`]: a table nobody has spoken at has no chat file,
    /// and every route below would otherwise have to tell an empty chat from a
    /// missing table itself. The version comes back `None` with it, which is
    /// exactly what the first write wants to create rather than replace.
    ///
    /// **A document that will not PARSE is a `Server` failure**, for
    /// `read_table`'s reason read one document along: answering "no messages"
    /// for a file that is sitting right there is how the next append overwrites
    /// it and numbers from 1 again.
    ///
    /// The seat bound the shape rule applies is `at`'s: a table's player
    /// count, which the caller's own table document decides, or none at the
    /// house — see [`crate::chat::check_shape`].
    ///
    /// **Normalized** (M6.5c2 cycle D, row 130): the shape rule parses every
    /// author's address and name, and what it parsed is what is served — see
    /// [`crate::chat::normalized`].
    pub(crate) fn read_chat(&self, at: ChatAt<'_>) -> Result<(ChatDoc, Option<Version>), GameError> {
        let key = at.key();
        let Some((json, version)) = self.store.read(at.doc(), key).map_err(GameError::Server)? else {
            return Ok((ChatDoc::empty(key), None));
        };
        let parsed: ChatDoc = serde_json::from_str(&json)
            .map_err(|e| GameError::Server(format!("\"{key}\"'s chat document does not parse: {e}", key = crate::echo::echo(key))))?;
        check_chat_shape(&parsed, at.players())
            .map_err(|why| GameError::Server(format!("\"{key}\"'s chat document is malformed: {why}", key = crate::echo::echo(key))))?;
        Ok((crate::chat::normalized(parsed), Some(version)))
    }

    /// The compare-and-swap, with the shape rule applied ON THE WAY OUT.
    ///
    /// The table document's writer does not do that and does not need to: this
    /// one appends to what it just read, on every message, so a malformed
    /// author written once would poison every later read of that document under
    /// the table's own lock. Checking here makes a bad message a refusal to the
    /// caller who sent it instead.
    ///
    /// **The key is `at`'s and the document's own `id` is not consulted**,
    /// which is the whole of the fence: `id` is read out of a file, and the
    /// shape rule does not bound it (it has no expected key to bound it
    /// against). Keyed off the document, an append at one table that met a
    /// document claiming to be another wrote its messages — and its version —
    /// to the other table's file. Pinned by
    /// `an_append_writes_at_the_key_it_read_and_not_at_the_documents_own_id`.
    fn write_chat(&self, at: ChatAt<'_>, doc: &ChatDoc, expected: Option<Version>) -> Result<Version, GameError> {
        check_chat_shape(doc, at.players()).map_err(GameError::Server)?;
        let key = at.key();
        let json = serde_json::to_string_pretty(doc)
            .map_err(|e| GameError::Server(format!("the chat document does not serialize: {e}")))?;
        match self.store.write_if_version(at.doc(), key, &json, expected) {
            Ok(v) => Ok(v),
            Err(WriteError::Conflict { .. }) => Err(GameError::Conflict(format!(
                "\"{key}\"'s chat changed while this message was in flight",
                key = crate::echo::echo(key)
            ))),
            Err(WriteError::Io(e)) => Err(GameError::Server(e)),
        }
    }

    /// The append itself, with the caller's lock ALREADY HELD: read, number,
    /// push, trim, write.
    ///
    /// **One copy of R-6.4.9.** The numbering-and-trim invariant — a number is
    /// `last + 1`, `last` advances, the trim takes the oldest and leaves `last`
    /// alone — was written out twice, once per appender, and an invariant
    /// stated twice is an invariant that can be half-changed. The two public
    /// appenders differ in three things and only three: which lock they hold,
    /// which document they name ([`ChatAt`]), and which channel they bump
    /// afterwards.
    ///
    /// **The text has already been checked** by the caller, deliberately before
    /// the APPEND takes the table's lock, so a message that will be refused
    /// never makes anyone wait behind another message. That is why this takes a
    /// `String` rather than a `&str` and applies no rule of its own. (Not
    /// "before any lock": the caller's admission runs first, and on a dealing
    /// table where every human has kept it goes through [`Games::settle`],
    /// which takes that table's lock to finish the deal — the same correction
    /// `lib::chat_post`'s own doc carries.)
    ///
    /// The document is written under `at`'s key and its `id` is SET to that
    /// key, so a document this server writes always names the file it is in —
    /// see [`Games::write_chat`] for what keying off the stored `id` did.
    ///
    /// **An append moves the REVISION too** (M6.5c2 cycle D, R-D4): `rev` is
    /// +1 per append and per delete, so `rev - last` moves exactly when a held
    /// message was deleted. The counts come back beside the message for the
    /// caller's cache.
    fn append_locked(&self, at: ChatAt<'_>, author: Author, text: String) -> Result<(Message, ChatCounts), GameError> {
        let (mut doc, version) = self.read_chat(at)?;
        doc.id = at.key().to_string();
        let message = Message { seq: doc.last + 1, created_at: (self.clock)(), author, text, deleted: false };
        doc.last = message.seq;
        doc.rev += 1;
        doc.messages.push(message.clone());
        doc.trim(self.chat.limits.keep);
        self.write_chat(at, &doc, version)?;
        Ok((message, doc.counts()))
    }

    /// **The one writer** (R-6.4.15 (b)): the text rule, the table's own lock,
    /// the read, the append, the trim, the compare-and-swap, THEN the bump.
    ///
    /// The lock is the TABLE's and not a lock of the chat's own, because a
    /// message is a write at a table and the thing being serialized is
    /// read-then-write against one document. Two appends that both read `last`
    /// and both wrote `last + 1` would leave one message on the floor with the
    /// store's version check as the only witness.
    ///
    /// **The bump is last, after the write, and a gated cell proves it** —
    /// `a_chat_bump_precedes_no_write`. A bump ahead of its write sends every
    /// browser at the table to fetch a message that has not landed, and the
    /// message that then lands is announced to nobody.
    pub fn chat_append(
        &self,
        id: &str,
        author: Author,
        text: &str,
    ) -> Result<Message, GameError> {
        let text = check_text(text, &self.chat.limits).map_err(GameError::BadRequest)?;
        self.require_table(id)?;
        self.with_table_lock(id, || {
            let (table, _) = self
                .read_table(id)?
                .ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
            let (message, counts) = self.append_locked(ChatAt::Table { id, players: table.players }, author, text)?;
            // `message.seq` IS the document's `last`: the trim drops the oldest and
            // leaves the number alone, which is the whole of R-6.4.9.
            self.chat_counts.lock().expect("games lock").insert(id.to_string(), counts);
            // AFTER the write, never before (the gated cell).
            self.signal.bump_table(id);
            Ok(message)
        })
    }

    /// The same for the house, under the house's own lock and bumping the
    /// house's CHAT counter (R-6.4.7).
    ///
    /// The house (`ChatAt::House`) has no player count, which is what makes a
    /// seat author a refusal here: the house has no seats, and the shape rule
    /// says so rather than this function restating it.
    pub fn house_append(&self, author: Author, text: &str) -> Result<Message, GameError> {
        let text = check_text(text, &self.chat.limits).map_err(GameError::BadRequest)?;
        // `enter`, poisoned or not: the house's lock guards no data.
        let _serialized = enter(&self.house_lock);
        let (message, _) = self.append_locked(ChatAt::House, author, text)?;
        self.signal.bump_house_chat();
        Ok(message)
    }

    /// The delete itself, with the caller's lock ALREADY HELD: read, tombstone,
    /// write — [`Games::append_locked`]'s shape for [`ChatDoc::delete`]
    /// (M6.5c2 cycle D, R-D4).
    ///
    /// Answers the counts to cache when the document CHANGED, and `None` when
    /// it did not — a second delete of a tombstone — so the caller writes
    /// nothing and bumps nothing for a request that changed nothing. A refusal
    /// is [`ChatRefusal`]'s sentence, verbatim: `NotFound` a 404 and
    /// `NotYours` a 403.
    fn delete_message_locked(
        &self,
        at: ChatAt<'_>,
        seq: u64,
        user: &User,
        creator: Option<&str>,
    ) -> Result<Option<ChatCounts>, GameError> {
        let (mut doc, version) = self.read_chat(at)?;
        doc.id = at.key().to_string();
        let before = doc.rev;
        doc.delete(seq, user, creator).map_err(|refusal| match refusal {
            ChatRefusal::NotFound(_) => GameError::NotFound(refusal.to_string()),
            ChatRefusal::NotYours => GameError::Forbidden(refusal.to_string()),
        })?;
        if doc.rev == before {
            return Ok(None);
        }
        self.write_chat(at, &doc, version)?;
        Ok(Some(doc.counts()))
    }

    /// **The table's delete** (M6.5c2 cycle D, R-D4): the table's own lock,
    /// the read, the tombstone, the compare-and-swap, THEN the bump — the
    /// appender's order and for the appender's reason, pinned by
    /// `a_chat_delete_bump_precedes_no_write`. The CREATOR is the stored
    /// table's `created_by`, read under the same lock.
    ///
    /// The caller has ADMITTED the user (`api::chat_delete`, as `chat_post`
    /// admits a poster), so a stranger never reaches the document.
    pub fn delete_chat(&self, id: &str, seq: u64, user: &User) -> Result<(), GameError> {
        self.require_table(id)?;
        self.with_table_lock(id, || {
            let (table, _) = self
                .read_table(id)?
                .ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
            let at = ChatAt::Table { id, players: table.players };
            let Some(counts) = self.delete_message_locked(at, seq, user, Some(&table.created_by))? else {
                return Ok(());
            };
            self.chat_counts.lock().expect("games lock").insert(id.to_string(), counts);
            // AFTER the write, never before (the gated cell).
            self.signal.bump_table(id);
            Ok(())
        })
    }

    /// **The house's delete** (M6.5c2 cycle D, R-D4): under the house's own
    /// lock, the author alone — the house has no creator — and then the
    /// house's REVISION bump, never its chat counter: a delete adds no message.
    pub fn delete_house_chat(&self, seq: u64, user: &User) -> Result<(), GameError> {
        // `enter`, poisoned or not: the house's lock guards no data.
        let _serialized = enter(&self.house_lock);
        if self.delete_message_locked(ChatAt::House, seq, user, None)?.is_some() {
            self.signal.bump_house_chat_rev();
        }
        Ok(())
    }

    /// The retained messages numbered above `after`, and the document's `last`
    /// and `rev` ([`ChatCounts`]; the revision since M6.5c2 cycle D, R-D4).
    ///
    /// `last` is the document's and not the page's, so a client that asks from
    /// a cursor the trim has passed is told what the newest number is rather
    /// than being left to infer it from an empty page.
    ///
    /// **`table` is the caller's, read by its admission** (M6.5c2 cycle D, row
    /// 130): the chat's seat bound is the table's player count, and reading
    /// the document a second time for that number was a second read per page.
    /// The KEY is `id` and never `table.id`, for `Games::write_chat`'s reason:
    /// a document's own `id` is read out of a file.
    ///
    /// **Built for `reader`** (M6.5c2 cycle D, R-6.5c2.35, R-D3): each message
    /// crosses as [`ChatDoc::page_for`] hands it — `you` on the reader's own,
    /// and no author's address.
    pub fn chat_page(
        &self,
        id: &str,
        table: &TableDoc,
        reader: &User,
        after: u64,
    ) -> Result<(Vec<MessageView>, ChatCounts), GameError> {
        let (mut doc, _) = self.read_chat(ChatAt::Table { id, players: table.players })?;
        doc.messages.retain(|m| m.seq > after);
        Ok((doc.page_for(reader), doc.counts()))
    }

    /// The house's page, built for `reader` as [`Games::chat_page`]'s is.
    pub fn house_page(&self, reader: &User, after: u64) -> Result<(Vec<MessageView>, ChatCounts), GameError> {
        let (mut doc, _) = self.read_chat(ChatAt::House)?;
        doc.messages.retain(|m| m.seq > after);
        Ok((doc.page_for(reader), doc.counts()))
    }

    /// The house chat document's `last` and `rev` ([`ChatCounts`]), read as
    /// [`Games::house_page`] reads them — what the NAS SEEDS its house frame's
    /// counters from when it starts (M6.5c2 cycle D; the close
    /// `2026-09-29-web-frontend-m6.5c2d-close.md` §4 and §5, ruling 46;
    /// `AppState::with_store_chat_archive_and_log`, which the NAS reaches
    /// through `AppState::production`).
    ///
    /// An absent document is an empty one, `0` and `0`, as every read of it
    /// is. A document that cannot be read or will not parse is the refusal
    /// every house route answers with.
    pub fn house_counts(&self) -> Result<ChatCounts, GameError> {
        self.read_chat(ChatAt::House).map(|(doc, _)| doc.counts())
    }

    /// The count a signal frame carries (R-plan-5), from the cache or from one
    /// read of the document.
    ///
    /// **A store that cannot be read answers 0 and is NOT cached**, so the next
    /// call retries. Every route beside this one refuses on that failure; this
    /// one is called from a stream's per-bump closure, which has nowhere to put
    /// a refusal, and a wrong count sends a client to a route that WILL refuse
    /// properly.
    ///
    /// **An absent document answers 0 and IS cached** — that is not a failure,
    /// it is a table nobody has spoken at, and `create` and `delete` both drop
    /// the entry so a reopened name cannot inherit it.
    ///
    /// **The seed only ever FILLS an empty slot and never replaces one**
    /// (`or_insert`, final review minor 6). The read above is deliberately
    /// under no lock of this table's — this is called from a stream's per-bump
    /// closure and must not queue behind a message — so an append can land
    /// between it and this insert, and a plain `insert` would put the older
    /// number back.
    ///
    /// **Exactly THREE things fill the entry, and keeping what is there is
    /// never the older answer under any** (re-review of the fix wave, `docs/plans/2026-09-19-web-frontend-m6.4-close.md` §4.6;
    /// the delete since M6.5c2 cycle D): an append and a delete, each of which
    /// writes its own counts with a plain `insert` after the document is
    /// written and under the table's lock — so they land in the order the
    /// writes did, and know numbers no read of this kind can have missed; and
    /// a cold seed like this one, which reads the document — so two cold seeds
    /// racing each other carry the same numbers and it does not matter which
    /// wins. Pinned, with the window held open by a gate, by
    /// `a_cold_count_read_never_overwrites_a_newer_one_from_an_append`.
    pub fn chat_last(&self, id: &str) -> u64 {
        self.chat_counts(id).last
    }

    /// [`Games::chat_last`]'s reading with the document's revision beside it
    /// (M6.5c2 cycle D, R-D4) — the pair `Events` carries as `chat` and
    /// `chat_rev`, cached and seeded exactly as that doc says.
    pub fn chat_counts(&self, id: &str) -> ChatCounts {
        if let Some(n) = self.chat_counts.lock().expect("games lock").get(id) {
            return *n;
        }
        // The player count is not known here and the seat bound is a WRITE-side
        // and route-side check: passing `None` would refuse every seat author
        // this server itself wrote. So the document is parsed without the shape
        // rule and only its `last` and `rev` are read — and a document that
        // will not parse reads 0 here while every route that SERVES it says so
        // out loud.
        //
        // **That 0 is CACHED like any other** (re-review of the fix wave,
        // `docs/plans/2026-09-19-web-frontend-m6.4-close.md` §4.6, out-of-scope note), so an unparseable document makes this table
        // report 0 for the life of the process — until an append re-inserts a
        // real number, which it cannot do while the document will not parse.
        // Pre-existing and deliberate: the alternative is re-reading and
        // re-parsing a broken file on every bump of a table nobody can fix from
        // here, and the routes that serve the messages refuse out loud rather
        // than serving this count as if it meant something. The cell at
        // `a_chat_document_that_will_not_parse_refuses_the_page_and_reads_zero`
        // pins the 0; it does not pin the caching.
        let n = match self.store.read(Doc::Chat, id) {
            Ok(Some((json, _))) => serde_json::from_str::<ChatDoc>(&json).map(|d| d.counts()).unwrap_or_default(),
            Ok(None) => ChatCounts::default(),
            Err(_) => return ChatCounts::default(),
        };
        *self.chat_counts.lock().expect("games lock").entry(id.to_string()).or_insert(n)
    }

    /// The table as a seated caller may read it, REPAIRED first if it needs it.
    ///
    /// **This is the fence `deal`, `keep` and `delete` still read through**
    /// (R-1.5). Since M6.3 the session ROUTES read through `lib::admitted`
    /// instead, which goes via [`Games::table_for_anyone`] and applies its own,
    /// stricter status arms on top.
    pub fn table(&self, id: &str, user: &User) -> Result<TableDoc, GameError> {
        let (doc, _) = self
            .settle(id)?
            .ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
        if doc.seats_of(user).is_empty() {
            return Err(GameError::Forbidden("you are not seated at this table".to_string()));
        }
        Ok(doc)
    }

    /// [`Games::table`]'s rule with R-2.7's ONE widening: an OPEN table reads
    /// for any signed-in caller.
    ///
    /// A join link is an address and nothing else (R-2.5), so the person who
    /// follows one has to be able to see what they are joining — the seats, who
    /// is already there, and whether a password is wanted — before they hold a
    /// seat at it. What an open table discloses is exactly what the `open`
    /// listing already hands to everybody.
    ///
    /// **M6.3 widens it again, at `playing` and over** (R-3.13, superseding
    /// M6.2's "and `table` at every other status keeps R-1.5's 403"): a
    /// would-be watcher has to read `locked` and `can_watch` to know whether to
    /// ask for a password, and the view carries exactly the public facts the
    /// `playing` listing already hands to everybody. A LOCKED table is read the
    /// same way and answers `can_watch: false` — the lock gates watching, never
    /// seeing that the game exists.
    ///
    /// **`Dealing` is the one status a stranger is still refused at, and it is
    /// now the whole of the fence.** Nobody watches a deal (R-3.5), and a deal
    /// is the moment the OTHER seats' cards are in flight. So the predicate is
    /// "not dealing, or seated" rather than a list of admitted statuses: a
    /// fourth status added to `TableStatus` would be open to a stranger by
    /// default, which is why `check_shape` and this sentence are both here.
    /// Every other session route reads through the crate root's `admitted`, whose own
    /// status arms are stricter than this one.
    pub fn table_for_anyone(&self, id: &str, user: &User) -> Result<TableDoc, GameError> {
        let (doc, _) = self
            .settle(id)?
            .ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
        if doc.status != TableStatus::Dealing || !doc.seats_of(user).is_empty() {
            return Ok(doc);
        }
        Err(GameError::Forbidden("you are not seated at this table".to_string()))
    }

    /// **Present the password once, and this caller may watch from now on**
    /// (M6.3, §3.4, R-3.5).
    ///
    /// The email is written onto the table document rather than handed back as
    /// a token, for the reason every other seam here is a document: a token
    /// would be a second credential this server would have to mint, carry and
    /// expire, and the thing being gated is one table in one room. The list is
    /// the table's memory of who has knocked.
    ///
    /// **A no-op is a 200 and writes nothing** — an unlocked table, or a caller
    /// who already watches — because the client posts this before it knows
    /// which case it is in, and a write per page load would be a write per page
    /// load. The seat holder's arm is the same 200: asking a player for their
    /// own table's password is the one answer that is always wrong.
    ///
    /// Held under this table's own lock (`Games::lock_for`, R-2.9) across the
    /// read and the write, so two unlocks in flight cannot each append over the
    /// other's version — and a conflict is reported rather than retried, since
    /// the caller knows their password and re-posting is free.
    ///
    /// **It bumps no signal** (controller ruling R-pre-3): it changes nothing
    /// any other screen shows. The unlocking caller is answered with the view
    /// in this very reply.
    pub fn unlock(&self, id: &str, user: &User, password: &str) -> Result<TableView, GameError> {
        self.require_table(id)?;
        self.with_table_lock(id, || {
            let (mut doc, version) = self
                .read_table(id)?
                .ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
            // There is nothing to watch yet at either of these, and a joiner sends
            // the password to `join`. Typed, so the client branches on the status
            // rather than on the sentence.
            match doc.status {
                TableStatus::Open => return Err(GameError::Open),
                TableStatus::Dealing => return Err(GameError::Dealing),
                TableStatus::Playing => {}
            }
            if doc.lock.is_none() || doc.can_watch(user) {
                return Ok(doc.view(user, self.archive_on()));
            }
            if !doc.check_password(password.trim()) {
                return Err(GameError::Forbidden("wrong password for this table".to_string()));
            }
            if doc.unlocked.len() >= UNLOCKED_CAP {
                return Err(GameError::Conflict(
                    "this table has as many spectators as it can remember".to_string(),
                ));
            }
            doc.unlocked.push(user.as_str().to_string());
            match self.write_table(&doc, Some(version)) {
                Ok(_) => Ok(doc.view(user, self.archive_on())),
                Err(WriteError::Conflict { .. }) => {
                    Err(GameError::Conflict(format!("\"{id}\" changed while this unlock was in flight", id = crate::echo::echo(id))))
                }
                Err(WriteError::Io(e)) => Err(GameError::Server(e)),
            }
        })
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
            return self.with_table_lock(id, || self.settle_locked(id));
        }
        Ok(Some((doc, version)))
    }

    /// [`Games::settle`] with this table's own lock already held by the caller
    /// ([`Games::lock_for`] — R-2.9, no longer the global one). The
    /// document is re-read under the lock, because the writer that made the
    /// repair necessary may have finished it while this request was waiting.
    fn settle_locked(&self, id: &str) -> Result<Option<(TableDoc, Version)>, GameError> {
        let Some((doc, version)) = self.read_table(id)? else { return Ok(None) };
        if doc.status != TableStatus::Dealing || !doc.all_humans_kept() {
            return Ok(Some((doc, version)));
        }
        if self.store.read(Doc::Record, id).map_err(GameError::Server)?.is_some() {
            let handle = self.live_handle(id, &doc)?;
            let turn = mirror_of(&*self.lock_game(&handle, id)?);
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
        // Where it was played (M6.5c2, spec §2.4): AFTER the seats, whose
        // provenance the id lands in — `Session::set_run_id`'s own contract.
        session.set_run_id(self.run_host.run_id());
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
                let handle = self.live_handle(&id, &doc)?;
                let turn = mirror_of(&*self.lock_game(&handle, &id)?);
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
            .ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
        doc.status = TableStatus::Playing;
        doc.turn = Some(turn);
        match self.write_table(&doc, Some(version)) {
            Ok(version) => {
                // **This is `finalize`'s bump too**, and there is exactly one
                // because there is exactly one write: `finalize` ends by
                // calling this function, so a bump of its own would announce
                // the same flip twice. A `watch` coalesces, so the second
                // would be harmless — and it would still be a bump for a write
                // that did not happen, which is the rule this file keeps.
                self.signal.bump(id);
                Ok((doc, version))
            }
            // Another writer flipped it first. Its answer is as good as ours —
            // the record is the same record — so report what is there.
            Err(WriteError::Conflict { .. }) => self
                .read_table(id)?
                .ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id)))),
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
        // A table still filling its seats has no deal to serve: the cards a seat
        // is shown depend on the PLAYER COUNT and the seed, and a seat that is
        // still open is a seat whose neighbour has not arrived.
        if doc.status == TableStatus::Open {
            return Err(GameError::Open);
        }
        if doc.status == TableStatus::Playing {
            return Err(GameError::Playing);
        }
        let seat = doc.next_unkept_seat_of(user);
        let config = config_from(&doc);
        let pending = new_game(config.clone(), &self.rules, doc.seed)
            .map_err(|e| GameError::Server(format!("the setup did not deal: {e}")))?;
        // The set table is drawn for the seat this caller is choosing FOR, so
        // the frame beside the two cards is that seat's own; once every seat of
        // theirs has chosen there is no such seat and their own lowest seat is
        // drawn instead ([`seat_of_waiting`]). Neither shows a hand.
        let frame = set_table(
            config,
            doc.seed,
            &doc.seats,
            &pending.deals,
            &self.rules,
            seat.unwrap_or_else(|| seat_of_waiting(&doc, user)),
        )
        .map_err(GameError::Server)?;
        Ok(DealView {
            table: doc.view(user, self.archive_on()),
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

    /// Take an OPEN seat at an open table (M6.2, §3.2).
    ///
    /// **The refusals are in the order a client can act on them** (§3.2): the
    /// table exists, it is still open, the seat is one this table has, the name
    /// is a name, the seat is free, and the password is right. The PASSWORD is
    /// last of the six on purpose — a caller who sent the wrong seat number
    /// learns that before it learns anything about the lock, and a caller who
    /// sent neither right learns about the seat first, which is the thing it can
    /// fix by itself.
    ///
    /// **The seat and the flip are ONE write.** A join that filled the seat,
    /// wrote, and then flipped would leave a moment at which the table is full
    /// and still `Open` — a shape `check_shape` refuses to read back — and two
    /// crash windows where there is now none.
    ///
    /// A table WITHOUT a lock ignores any password sent: the password is the
    /// table's, not the caller's, and refusing a harmless extra field here would
    /// make a client that remembered one unable to join a table that dropped it.
    pub fn join(
        &self,
        id: &str,
        user: &User,
        seat: PlayerId,
        name: Option<&str>,
        password: Option<&str>,
    ) -> Result<TableView, GameError> {
        self.require_table(id)?;
        self.with_table_lock(id, || {
            let (mut doc, version) = self
                .read_table(id)?
                .ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
            match doc.status {
                TableStatus::Open => {}
                TableStatus::Dealing => return Err(GameError::Dealing),
                TableStatus::Playing => return Err(GameError::Playing),
            }
            // `seats.len()`, not `PlayerId`'s own range: seat 3 is a real seat at a
            // 4P table and not one here, and `read_table` has already refused a
            // document whose `seats` disagrees with its `players`.
            let n = usize::from(seat.0);
            if n >= doc.seats.len() {
                return Err(GameError::BadRequest(format!(
                    "seat {} at a {}-seat table",
                    seat.0, doc.players
                )));
            }
            let name = match name {
                Some(raw) => Some(check_name(raw).map_err(GameError::BadRequest)?),
                None => None,
            };
            // **Open, not "not mine".** A seat this caller already holds is taken
            // too — otherwise a join is a way to re-bind a seat somebody is sitting
            // at, including one's own, and a person who wants a second seat takes an
            // OPEN one (which this allows, and a cell pins).
            if !matches!(doc.seats[n], SeatDoc::Open) {
                return Err(GameError::Conflict(format!("seat {} is taken", seat.0)));
            }
            if doc.lock.is_some() {
                match password.map(str::trim) {
                    // An absent password and an empty one are the same request —
                    // "let me in without one" — and get the same sentence, which
                    // names what to send rather than saying the guess was wrong.
                    None | Some("") => {
                        return Err(GameError::Forbidden(
                            "this table is locked — send its password".to_string(),
                        ))
                    }
                    Some(p) if !doc.check_password(p) => {
                        return Err(GameError::Forbidden(
                            "wrong password for this table".to_string(),
                        ))
                    }
                    Some(_) => {}
                }
            }
            doc.seats[n] = SeatDoc::Human { user: user.as_str().to_string(), name, bound_at_create: false };
            if doc.is_full() {
                doc.status = TableStatus::Dealing;
            }
            match self.write_table(&doc, Some(version)) {
                Ok(_) => {
                    // The house too: a seat filled is a row on the open list that
                    // changed, and a join that FILLED the table is a row that left
                    // it — the same write, so the same bump.
                    self.signal.bump(id);
                    Ok(doc.view(user, self.archive_on()))
                }
                // Somebody took this seat between the read and here. The stored
                // table is the table — `create`'s rule and `finalize`'s — so the
                // answer is a conflict and not a retry.
                Err(WriteError::Conflict { .. }) => Err(GameError::Conflict(format!(
                    "\"{id}\" changed while this join was in flight",
                    id = crate::echo::echo(id)
                ))),
                Err(WriteError::Io(e)) => Err(GameError::Server(e)),
            }
        })
    }

    /// One seat chooses. The LAST one finalizes.
    ///
    /// `name` is the display name the chooser gave (M6.2, §3.5), and it lands
    /// on the seat ONLY when the seat has none yet: a seat named at create
    /// keeps that name, and a seat somebody else's create bound gets its name
    /// from its holder's first keep. A later keep never renames a seat.
    pub fn keep(
        &self,
        id: &str,
        user: &User,
        seat: PlayerId,
        card: u8,
        name: Option<&str>,
    ) -> Result<TableView, GameError> {
        self.require_table(id)?;
        self.with_table_lock(id, || {
            // Settled first, so a table left mid-finalize by a crash is repaired by
            // the same call that would otherwise refuse a keep it has already taken.
            let (mut doc, version) = self
                .settle_locked(id)?
                .ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
            let n = seat.0;
            // A seat nobody holds and a seat somebody ELSE holds are the same
            // refusal, and deliberately: a caller learning which of the two it was
            // would be learning about a table it has no seat at. The bounds check
            // rides along — `seats_of` only ever names a seat this document has.
            //
            // **ABOVE the two status arms, and that order is the claim** (spec
            // §3.4's "a STRANGER gets 403 … on every route but `table` and `join`",
            // ruling #8). Below them, a caller with no seat here was told WHICH
            // status the table was in — 409 "this table is still open" or "this
            // table is playing" — so `keep` was the one route on which somebody
            // holding only an id learned something about a table that was not
            // theirs, while every other route answered them with one sentence about
            // seats and nothing else. Measured against the whole route sweep in
            // `every_session_route_refuses_a_stranger_at_every_status_except_table_and_join_at_open`,
            // which is where the discrepancy was found.
            //
            // **No SEATED caller's answer moves**: a holder passes this check and
            // meets the same status arm it met before, so the 409 `open` a seat
            // holder gets at an open table is unchanged and is asserted in that
            // same cell.
            if !doc.seats_of(user).contains(&seat) {
                return Err(GameError::Forbidden(format!("seat {n} is not yours")));
            }
            if doc.status == TableStatus::Open {
                return Err(GameError::Open);
            }
            if doc.status == TableStatus::Playing {
                return Err(GameError::Playing);
            }
            if doc.keeps[usize::from(n)].is_some() {
                return Err(GameError::Conflict(format!("seat {n} has already chosen")));
            }
            if !(1..=2).contains(&card) {
                return Err(GameError::BadRequest(format!(
                    "keep {card} — the dealt cards are numbered 1 and 2"
                )));
            }
            // Checked BEFORE anything is mutated, so a refused name leaves the
            // document exactly as it was — the keep included.
            let name = match name {
                Some(raw) => Some(check_name(raw).map_err(GameError::BadRequest)?),
                None => None,
            };
            doc.keeps[usize::from(n)] = Some(card);
            if let (Some(new), SeatDoc::Human { name: slot @ None, .. }) =
                (name, &mut doc.seats[usize::from(n)])
            {
                *slot = Some(new);
            }
            // **Written BEFORE the finalize, not with it.** The choice is the
            // player's and is durable the moment it is made; a crash between this
            // write and the record leaves a table that `settle` finishes on the next
            // read. Writing the keep only as part of a successful finalize would
            // lose every choice at a table whose last keep failed.
            match self.write_table(&doc, Some(version)) {
                Ok(_) => {}
                Err(WriteError::Conflict { .. }) => {
                    return Err(GameError::Conflict(format!(
                        "\"{id}\" changed while this keep was in flight",
                        id = crate::echo::echo(id)
                    )))
                }
                Err(WriteError::Io(e)) => return Err(GameError::Server(e)),
            }
            // **The TABLE alone** (spec §2): the dealing screen's "waiting on"
            // line has moved and no lobby row has. If this keep is the last one,
            // the finalize below reaches the house through `flip_to_playing` —
            // which is a second write and therefore rightly a second bump.
            self.signal.bump_table(id);
            if doc.all_humans_kept() {
                let (doc, _) = self.finalize(doc)?;
                return Ok(doc.view(user, self.archive_on()));
            }
            Ok(doc.view(user, self.archive_on()))
        })
    }

    /// **One page of the lobby**, in the scope the caller asked for (§3.3).
    ///
    /// **A record with no table document is not a game this API serves** (R-P7):
    /// the listing is over `Doc::Table` ALONE — no record is read and no session
    /// is built, which `a_listing_reads_no_record_and_builds_no_session` holds
    /// with a store whose record reader panics. `to_move`, `over` and `winner`
    /// come off the table's own [`TurnMirror`], which exists for exactly this:
    /// a lobby that read them from the games would rebuild every session on the
    /// volume on every poll. So a pre-M6 record is neither listed nor openable
    /// and is left on disk untouched for the Docker-archive pull.
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
    /// layer's log (`Games::with_log`, M6.5c2): stderr beside `main.rs`'s own
    /// lines on the NAS, the shell's log on the Worker, the console in the
    /// browser — so a corrupt table is visible wherever the layer runs.
    ///
    /// **The order is `(created_at desc, id asc)` and it is TOTAL**, which is
    /// what makes the cursor exact rather than approximate: `created_at` is unix
    /// SECONDS, so a sort on it alone leaves ties whose order is whatever the
    /// store listed, and a page boundary inside such a tie would drop rows or
    /// repeat them. `before` names the last row the caller saw and the retain is
    /// STRICTLY after it in that same order — no gap, no overlap
    /// (`the_cursor_pages_without_gap_or_overlap`).
    ///
    /// `limit` is the caller's page size, already range-checked at the route;
    /// **`0` is refused here too** rather than trusted, because `rows[limit - 1]`
    /// below would otherwise index `usize::MAX` for any in-process caller that
    /// did not check. `next` is `Some` only when a row was actually left over.
    pub fn list(
        &self,
        user: &User,
        scope: Scope,
        limit: usize,
        before: Option<(u64, String)>,
    ) -> Result<(Vec<TableView>, Option<String>), String> {
        if limit == 0 {
            return Err("a page of no rows is not a page".to_string());
        }
        let mut rows: Vec<TableDoc> = Vec::new();
        for id in self.store.list(Doc::Table)? {
            let doc = match self.store.read(Doc::Table, &id) {
                Ok(Some((json, _))) => match serde_json::from_str::<TableDoc>(&json) {
                    Ok(doc) => doc,
                    Err(e) => {
                        self.log.line(
                            crate::log::LISTING_SKIPPED,
                            serde_json::json!({ "id": id.as_str(), "why": "does not parse", "error": e.to_string() }),
                        );
                        continue;
                    }
                },
                // Absent between the listing and the read — a delete that
                // landed in the window. Nothing is wrong and nothing is lost.
                Ok(None) => continue,
                Err(e) => {
                    self.log.line(
                        crate::log::LISTING_SKIPPED,
                        serde_json::json!({ "id": id.as_str(), "why": "could not be read", "error": e }),
                    );
                    continue;
                }
            };
            // **The same fence `read_table` applies**, because this path
            // deliberately does not go through it — and `seats_of` two lines
            // below is exactly the call [`check_shape`] exists to make total.
            if let Err(why) = check_shape(&doc) {
                self.log.line(
                    crate::log::LISTING_SKIPPED,
                    serde_json::json!({ "id": id.as_str(), "why": "is malformed", "error": why }),
                );
                continue;
            }
            let keep = match scope {
                Scope::Mine => !doc.seats_of(user).is_empty(),
                Scope::Open => doc.status == TableStatus::Open,
                Scope::Playing => doc.status == TableStatus::Playing,
            };
            if keep {
                rows.push(doc);
            }
        }
        // Newest first, ties broken by id so the order is total and a reload
        // does not reshuffle two tables made in the same second.
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at).then_with(|| a.id.cmp(&b.id)));
        if let Some((at, id)) = before {
            // Strictly AFTER the cursor row in this order: older created_at, or
            // an equal created_at with a greater id.
            rows.retain(|d| d.created_at < at || (d.created_at == at && d.id > id));
        }
        let next = (rows.len() > limit).then(|| cursor_of(&rows[limit - 1]));
        rows.truncate(limit);
        Ok((rows.iter().map(|doc| doc.view(user, self.archive_on())).collect(), next))
    }

    /// Every table this user sits at, newest first — [`Games::list`] at the
    /// `mine` scope with no paging.
    ///
    /// Kept as a name because the cells that predate the scopes read this
    /// question and not a page of it; it DELEGATES rather than restating the
    /// loop, so there is one listing in this file and not two that can disagree.
    pub fn list_for(&self, user: &User) -> Result<Vec<TableView>, String> {
        self.list(user, Scope::Mine, LIST_LIMIT_MAX, None).map(|(rows, _)| rows)
    }

    /// Delete, and remember that we did. **The creator's, and nobody else's.**
    ///
    /// The tombstone is what stops an in-flight `input` from writing the record
    /// back after the file is gone. It is bounded by `create`, which clears the
    /// id — so the set only ever holds ids that are deleted and not recreated,
    /// and re-creating a deleted name works exactly as before.
    ///
    /// **It is a TABLE WRITER, so it holds this table's own lock** — since
    /// M6.2 `Games::lock_for`'s and not the global `table_lock`, which now
    /// serializes `create` alone (R-2.9).
    /// Without it, a delete could read a table that was still dealing, and the
    /// last `keep` could then write the table and enter `finalize` before the
    /// unlink landed — leaving `finalize`'s `expected: None` record write to
    /// succeed over two documents that were already gone. The result is an
    /// orphan record: unlisted (R-P7 lists over tables), unopenable, and
    /// holding the name forever, since `create` checks both documents. Measured
    /// and reproduced in
    /// `a_delete_cannot_interleave_with_the_keep_that_finalizes`.
    ///
    /// The table is therefore read through `Games::read_table` and not
    /// through [`Games::table`]: `table` goes via `settle`, which takes this
    /// same non-reentrant lock when a repair is due, and calling it from here
    /// would deadlock. The two refusals it raises are restated inline so the
    /// caller reads the same sentences — and NOT settling is the right answer
    /// besides, since finalizing a table on the way to deleting it only writes
    /// the record this function then has to remove.
    ///
    /// **One window is by design** (M6.5c2 cycle D, row 128; moved by cycle F,
    /// Task 4b). The tombstone goes down FIRST, under the save lock and before
    /// the record is read (the order is argued inline below), so a `persist`
    /// that begins after it — a move at this table while its delete is in
    /// flight — is refused, "…was deleted while this move was in flight", and
    /// evicts its handle, while one that began before it has landed before the
    /// record is read. If the put or the unlink then FAILS, the tombstone is
    /// lifted and the table is still there: one move refused out loud, never
    /// one lost in silence. Its reply says `APPLIED BUT NOT SAVED`, and the next
    /// request rebuilds from the record the failed delete left in place.
    ///
    /// **The lock entry is pruned on the way out, as every writer's is**
    /// (`Games::with_table_lock`) — after this call has let go of its own
    /// clone, and only when no other writer holds one (`Games::prune_lock`). A
    /// delete alone pruned from M6.5c2 cycle D, row 128, and only when it
    /// succeeded; since security S1c, finding B8, a refused delete prunes too,
    /// and one at an absent id never makes an entry.
    pub fn delete(&self, id: &str, user: &User) -> Result<(), GameError> {
        self.require_table(id)?;
        self.with_table_lock(id, || self.delete_locked(id, user))
    }

    /// [`Games::delete`]'s body, with this table's own lock held by the caller.
    fn delete_locked(&self, id: &str, user: &User) -> Result<(), GameError> {
        let (doc, _) = self
            .read_table(id)?
            .ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
        if doc.seats_of(user).is_empty() {
            return Err(GameError::Forbidden("you are not seated at this table".to_string()));
        }
        // The sentence names the ROLE and never the person (M6.5c2 cycle D,
        // R-6.5c2.35): it named `created_by`, handing the creator's address to
        // every other seat that pressed Abandon.
        if !doc.is_creator(user) {
            return Err(GameError::Forbidden("only the table's creator can delete this table".to_string()));
        }
        // **The tombstone goes down FIRST: before the record is read, under the
        // save lock. The order is the whole of it** (cycle F, Task 4b; spec
        // §2.4). `persist` reads the set under that same lock and holds it
        // across its record read and its write, so every save either landed
        // before this insert, and so before the record is read below, or reads
        // the tombstone and is refused out loud ("…was deleted while this move
        // was in flight"). The old order read and filed first and tombstoned
        // last: a save landing in between was answered saved, then unlinked,
        // and was missing from the copy; one that read after the unlink created
        // a record with no table. Both are reproduced against the old order in
        // `a_save_landing_while_the_delete_files_its_copy_is_refused_never_lost`
        // and `a_save_past_its_tombstone_check_is_filed_with_the_game_and_leaves_no_orphan`.
        // The lock is let go before the read and the put, so no other game's
        // save waits on the archive's disk.
        //
        // It stays BEFORE the unlink for `create`'s sake too. `create` CLEARS
        // this id from the set, and since the lock split (R-2.9) `create` holds
        // a different mutex from this one, so the two interleave: with the
        // insert after the unlink, a create that slipped between found both
        // documents absent, cleared a tombstone that was not there yet, wrote a
        // brand-new table — and then THIS call tombstoned the id that now names
        // it. `persist` reads the set, so every move at that new table answered
        // `"…" was deleted while this move was in flight` and evicted its
        // handle. With the insert first, `create`'s clear is strictly later in
        // every interleaving. Arranged and measured in
        // `a_delete_cannot_tombstone_the_table_a_create_put_in_its_place`.
        {
            let _saves = self.written.lock().expect("games lock");
            self.deleted.lock().expect("games lock").insert(id.to_string());
        }
        if let Err(refused) = self.archive_on_abandon(id, &doc) {
            // Nothing was unlinked, so the table stands and must stay playable:
            // the tombstone comes back up, as after a failed unlink below, or
            // every later move at a live game is refused as deleted. Pinned by
            // `a_delete_whose_put_fails_lifts_its_tombstone_and_the_next_save_lands`.
            self.deleted.lock().expect("games lock").remove(id);
            return Err(refused);
        }
        if let Err(e) = self.store.delete(id) {
            // Nothing was unlinked, so the table is still there and must still
            // be playable: a tombstone left behind by a failed unlink refuses
            // every move at a live game, naming a delete that did not happen.
            self.deleted.lock().expect("games lock").remove(id);
            return Err(GameError::Server(e));
        }
        self.written.lock().expect("games lock").remove(id);
        self.live.lock().expect("games lock").remove(id);
        // The store's own `delete` took the chat document with the record and
        // the table; the COUNT is this process's cache and goes here, or a name
        // reopened later numbers from above messages nobody can read.
        self.chat_counts.lock().expect("games lock").remove(id);
        // **`end`, not `bump`**: there is no state left for a woken subscriber
        // to read, so every stream on this table has to close rather than
        // fetch a 404. The house is bumped inside `end`, so a lobby drops the
        // row.
        self.signal.end(id);
        Ok(())
    }

    /// The copy [`Games::delete`] files before it unlinks, with this table's
    /// own lock held and the tombstone already down; an `Err` refuses the
    /// delete, and the caller lifts the tombstone.
    fn archive_on_abandon(&self, id: &str, doc: &TableDoc) -> Result<(), GameError> {
        // **Abandon and save** (M6.5a, R-6.5a.11; umbrella R-6.5.5): the rule
        // is [`saves_on_abandon`], asked here of the RECORD — the truth, never
        // the mirror's cache (cycle E, ruling P-3 as amended by T5-a) — and
        // asked of every `TableView` over the mirror's bit, so the lobby words
        // "Abandon and save" from the rule this applies. A saved table is
        // archived BEFORE anything is unlinked — and if the put fails the
        // delete is REFUSED, its tombstone lifted by the caller, nothing
        // removed, because the save is the point of the control and the
        // creator can simply try again. A table already stamped (at game over,
        // or by an earlier failed delete whose put landed) is deleted without a
        // second put; an open or dealing table, or a playing one with no
        // events, deletes as before. Since M6.5b (R-6.5b.13) a FINISHED game
        // with no stamp lands under `over/`. The rule is asked FIRST as if the
        // game had moved, so the record is read — and must parse — only where
        // the rest of it could still say yes, exactly as before the factoring:
        // a delete at an open table, or with the archive off, reads no record.
        let archive_on = self.archive_on();
        if saves_on_abandon(doc, true, archive_on) {
            let record = self.store.read(Doc::Record, id).map_err(GameError::Server)?;
            if let Some((record_json, _)) = record {
                let events = serde_json::from_str::<GameRecord>(&record_json)
                    .map(|r| r.events.len())
                    .map_err(|e| GameError::Server(format!("\"{id}\"'s record does not parse: {e}", id = crate::echo::echo(id))))?;
                if saves_on_abandon(doc, events > 0, archive_on) {
                    // R-6.5b.13: the leaf follows the MIRROR's truth. `turn` is a
                    // cache `mirror_turn` may leave one move stale when its write
                    // FAILS OR CONFLICTS — it swallows both — so in exactly that
                    // case the leaf can still lie; `Games::handle(id).is_over`
                    // would not, at the cost of a rebuild and a replay under this
                    // lock, and was declined. A lie costs a SECOND COPY only when
                    // the game-over stamp was lost as well (M6.5c2 cycle D, row
                    // 11): a stamped game is not put here at all, and an
                    // unstamped one whose mirror says over lands under `over/`,
                    // where its landed copy answers `Identical`
                    // (`a_landed_put_whose_stamp_failed_then_abandoned_lands_once_under_over`).
                    // So the residual is a lost stamp AND a mirror that no longer
                    // says over: a stale one, or a true one after an undo past
                    // game over at a one-person table, where the CLI's undo
                    // applies — no lie there, but the first ending's copy already
                    // sits under `over/` (measured in M6.5c2 cycle D and recorded
                    // in its close,
                    // `docs/plans/2026-09-29-web-frontend-m6.5c2d-close.md`).
                    // Named, not closed.
                    let why = if doc.turn.as_ref().is_some_and(|t| t.over) {
                        crate::archive::Why::Over
                    } else {
                        crate::archive::Why::Abandoned
                    };
                    let mut stamped = doc.clone();
                    stamped.archived_at = Some((self.clock)());
                    let table_json = crate::archive::strip_for_archive(&stamped);
                    self.archive
                        .put("", why, id, &record_json, &table_json)
                        .map_err(|why| {
                            GameError::Server(format!(
                                "the archive could not be written ({why}) — the table was not deleted"
                            ))
                        })?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::SeatView;
    use crate::chat::ChatLimits;
    use crate::games::test_support::*;
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
        let started =
            HistoricalFrame::at(&mut handle.lock().unwrap(), 0, Viewer::Seat(PlayerId::new(0)))
                .unwrap();
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
        r.seats[0] = SeatSpec { user: Some("other@t.test".into()), bot: false, open: false };
        let e = games.create(&r, &red).unwrap_err().to_string();
        assert!(e.contains("creator"), "R-1.6: {e}");
        let mut r = req("order");
        r.seats[1] = SeatSpec { user: Some("x@t.test".into()), bot: true, open: false };
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("exactly one"));
        let mut r = req("order");
        r.seats = vec![
            SeatSpec { user: None, bot: true, open: false },
            SeatSpec { user: None, bot: true, open: false },
        ];
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("human"));
        let mut r = req("order");
        r.name = Some("x".repeat(65));
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("1 to 64"));
        let mut r = req("order");
        r.password = Some("   ".into());
        assert!(
            games.create(&r, &red).unwrap_err().to_string().contains("at least one character")
        );
        let mut r = req("order");
        r.id = Some("bad id".into());
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("not a usable"));
        // **Wrong in TWO ways, at the two ENDS of the chain.** The id is the
        // only check that does not live in `config_of` — it needs the store's
        // rule, not this module's — so this is what says the request is
        // validated before the id is looked at rather than beside it.
        let mut r = req("order");
        r.players = 5;
        r.id = Some("bad id".into());
        let e = games.create(&r, &red).unwrap_err().to_string();
        assert!(e.contains("2 to 4"), "the whole of config_of precedes the id: {e}");
        let v = games.create(&req("order"), &red).expect("a table");
        assert_eq!(v.status, TableStatus::Dealing);
        assert_eq!(v.my_seats, vec![PlayerId::new(0)]);
        assert_eq!(v.chosen, vec![false, true], "the bot is chosen from the start");
        assert!(
            games.store().read(Doc::Record, "order").unwrap().is_none(),
            "no record until the last keep",
        );
    }

    /// **Every stamp this type writes is the injected clock's reading**
    /// (R-6.5a.6): the table a `create` writes, and the message an append
    /// writes, which are the only two callers in the crate.
    ///
    /// One cell rather than two, because the claim is the SEAM and not either
    /// site — a `now_unix()` left behind at one of them reads the host's wall
    /// clock, which is nowhere near this reading, and fails here. That is also
    /// why the reading is a fixed past instant and not anything derived from
    /// the host's own clock: a cell comparing against `now_unix()` would pass
    /// on exactly the code this task exists to remove.
    ///
    /// **What it is not**: a fence against `SystemTime` reached for anywhere
    /// else. R-6.5a.6 gives that to a source-scanning cell, because a compile
    /// for wasm cannot see a platform call that only panics at run time. What
    /// this holds is that the two callers ask [`Games`] rather than the clock.
    ///
    /// **Mutation-proven at both ends**: `now_unix()` was put back at the
    /// create site and at the append site in turn, and the cell failed on each
    /// — on the assertion belonging to the site that moved, so a failure here
    /// names which of the two it was.
    #[test]
    fn a_table_and_a_message_are_stamped_by_the_clock_the_games_was_built_with() {
        // Built through `with_parts` (cycle D, row 40): the clock is a PART the
        // host chooses, and `with_clock` after the default constructor is a
        // second route to it that no host takes.
        let games = Games::with_parts(
            Arc::new(Rules::standard()),
            Arc::new(MemStore::default()),
            crate::chat::ChatConfig::default(),
            Arc::new(crate::signal::NoSignal),
            Arc::new(|| 1_700_000_000),
            Arc::new(crate::archive::NoArchive),
            crate::games::RunHost::Server,
        )
        .with_seed_policy(crate::games::SeedPolicy::Diagnostic);
        let red = who("red@t.test");
        let view = games.create(&req("clock"), &red).expect("a table");
        let (doc, _) = games.read_table(&view.id).expect("a readable store").expect("the table");
        assert_eq!(doc.created_at, 1_700_000_000, "the table's stamp is the injected reading");
        let m = games.chat_append(&view.id, red_seat(), "hello").expect("an append");
        assert_eq!(m.created_at, 1_700_000_000, "and so is a message's");
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
                        SeatSpec { user: None, bot: true, open: false }
                    } else {
                        SeatSpec { user: Some("red@t.test".into()), bot: false, open: false }
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
        games.keep(&id, &red, PlayerId::new(0), 1, None).unwrap();
        let d = games.deal(&id, &red).unwrap();
        assert_eq!((d.seat, d.cards.is_none()), (None, true), "all of red's seats have kept");
        let d = games.deal(&id, &blue).unwrap();
        assert_eq!(ids(d.cards.as_ref().unwrap()), pending.deals[1].to_vec());
    }

    /// **A caller waiting on the others is drawn their OWN seat's set table**
    /// (M6.5c2 cycle D, straw poll row 6). BLUE (seat 1) keeps while RED has
    /// not, so blue has no seat left to choose for — and the frame beside
    /// "waiting" is still blue's, never red's seat 0 for being the lowest human
    /// seat. No card rides either frame (every human hand is `Unknown` until the
    /// game starts, and `cards` is null), so what this pins is whose table the
    /// waiting screen is: the caller's, as it was one keep earlier.
    #[test]
    fn the_waiting_callers_set_table_is_drawn_for_their_own_seat() {
        let games = games();
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        let id = games.create(&two_people("waiting", CORPUS_SEED), &red).unwrap().id;
        let choosing = games.deal(&id, &blue).expect("blue's deal");
        assert_eq!(choosing.frame.seat, Some(PlayerId::new(1)), "premise: choosing, blue is drawn blue's seat");
        games.keep(&id, &blue, PlayerId::new(1), 1, None).expect("blue keeps");
        let waiting = games.deal(&id, &blue).expect("blue waits on red");
        assert_eq!((waiting.seat, waiting.cards.is_none()), (None, true), "premise: blue has no seat left to choose for");
        assert_eq!(waiting.frame.seat, Some(PlayerId::new(1)), "the waiting frame is blue's own seat");
        let red_deal = games.deal(&id, &red).expect("red's deal");
        assert_eq!(red_deal.frame.seat, Some(PlayerId::new(0)), "and red, still choosing, is drawn red's");
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
    /// viewer this way. (A caller WAITING on others is drawn their own seat,
    /// [`seat_of_waiting`], which names no bot by construction; this fallback
    /// sits behind it for a caller holding no seat, whom `deal` refuses first.)
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
        games.keep(&id, &red, PlayerId::new(0), 1, None).unwrap();
        // Red holds every human seat, so `next_unkept_seat_of` is now `None`.
        // The last human keep FINALIZES, so the table is playing and `deal`
        // refuses — which is itself the answer: there is no set table left to
        // draw for anyone.
        let d = games.deal(&id, &red);
        assert!(matches!(d, Err(GameError::Playing)), "the table finalized on the last keep");

        // **The fallback is asserted on the function itself, because with
        // `preview` retired no ROUTE can reach it at a table holding a bot.**
        // It used to be reached through `preview`, which always chose its viewer
        // this way. A bot is legal at two players only (R-P10), so a bot table
        // has exactly one human seat and that seat's keep finalizes the table —
        // which is the refusal just above. Since cycle D no route reaches it at
        // a table of two PEOPLE either: a waiting caller is drawn their own seat
        // (`the_waiting_callers_set_table_is_drawn_for_their_own_seat`), and
        // `first_human_seat` is only `seat_of_waiting`'s answer for a caller
        // holding no seat. So the claim that it never names a bot is asserted
        // where it lives.
        let seats = |first_is_bot: bool| {
            if first_is_bot {
                vec![SeatDoc::Bot, SeatDoc::Human { user: "red@t.test".to_string(), name: None, bound_at_create: false }]
            } else {
                vec![SeatDoc::Human { user: "red@t.test".to_string(), name: None, bound_at_create: false }, SeatDoc::Bot]
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
        assert!(matches!(games.keep(&id, &blue, PlayerId::new(0), 1, None), Err(GameError::Forbidden(_))));
        assert!(matches!(games.keep(&id, &red, PlayerId::new(0), 3, None), Err(GameError::BadRequest(_))));
        assert!(matches!(games.handle(&id), Err(GameError::Dealing)));
        games.keep(&id, &red, PlayerId::new(0), 2, None).unwrap();
        assert!(matches!(games.keep(&id, &red, PlayerId::new(0), 1, None), Err(GameError::Conflict(_))));
        let v = games.keep(&id, &blue, PlayerId::new(1), 1, None).unwrap();
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
            matches!(games.keep(&id, &red, PlayerId::new(0), 1, None), Err(GameError::Playing)),
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
        r.seats[1] = SeatSpec { user: Some("red@t.test".into()), bot: false, open: false };
        let id = games.create(&r, &red).unwrap().id;
        assert_eq!(games.deal(&id, &red).unwrap().seat, Some(PlayerId::new(0)));
        games.keep(&id, &red, PlayerId::new(0), 1, None).unwrap();
        assert_eq!(games.deal(&id, &red).unwrap().seat, Some(PlayerId::new(1)));
        let v = games.keep(&id, &red, PlayerId::new(1), 1, None).unwrap();
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
        games.keep(&id, &red, PlayerId::new(0), 1, None).unwrap();
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
        // The test clock ticks once per reading (`Games::ticking_clock`), so the
        // second create is NEWER by construction and this cell's claim is about
        // the sort's first key, not the id tie-break. (Under the wall clock it
        // slept 1.1 s here, because `created_at` is unix SECONDS.)
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
    /// holds THAT TABLE's lock across the read (`Games::lock_for`, R-2.9), so
    /// the panic POISONS that mutex and every later `keep`, `join`, `delete`
    /// and repairing `settle` AT THIS TABLE dies with it, with nothing anywhere
    /// able to clear it.
    ///
    /// **Read the closing create at its M6.2 size.** Under M6.1's single global
    /// `table_lock` it was the load-bearing line, because a poisoning here took
    /// every later `create` in the process down too; since the lock split it
    /// would survive that poisoning in any case. What it still says — and what
    /// a `catch_unwind` somewhere could not fake — is that after twelve calls
    /// against three broken tables the PROCESS is alive and serving. The
    /// `list_for` line is the other half of R-3.1 — the bad file leaves the
    /// listing, the good one stays on it, and the call still answers `Ok`.
    ///
    /// **THREE documents, because there are three unbounded fields and only one
    /// of them was bounded.** `players` was R-3.2's; the final review found
    /// `start_seat` and `keeps[i]` reaching the SAME locked path with the same
    /// panic class. `config_from` does `PlayerId::new(doc.start_seat)`, which
    /// asserts `id < 4`, and `finalize` is called with the table's lock held; a
    /// stored `keeps: [0, …]` reaches `choices_of`'s `Keep(keep - 1)`, a
    /// debug-build overflow on the same call. Each arm is refused typed at the
    /// read, which is what makes the panic unreachable rather than caught.
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
            // `PlayerId::new(7)` panics, and `finalize` makes that call with
            // the table's lock held.
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
            // The path that holds the TABLE's lock across the read, which is
            // the one whose panic would be a poisoned mutex rather than one
            // dropped request.
            assert!(
                matches!(games.keep(id, &red, PlayerId::new(0), 1, None), Err(GameError::Server(_))),
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
            .expect("the process is alive, so the server can still open a table");
    }

    /// **A table the listing skips is SAID** (M6.5c2, spec §2.6): one
    /// `listing-skipped` line per skipped document, naming it and why, where
    /// each was an `eprintln!` that no wasm host could hear. Three documents,
    /// one per site — one that does not parse, one the store cannot read, one
    /// `check_shape` refuses — in the store's own (sorted) order, and the
    /// healthy table still listed.
    #[test]
    fn a_listing_says_each_table_it_skips_and_why() {
        /// A `MemStore` whose read of one id FAILS — a present file this
        /// process may not read.
        struct OneUnreadable(MemStore);
        impl Store for OneUnreadable {
            fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
                self.0.list(doc)
            }
            fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
                if id == "unread" {
                    return Err("permission denied".to_string());
                }
                self.0.read(doc, id)
            }
            fn write_if_version(&self, doc: Doc, id: &str, json: &str, expected: Option<Version>)
                -> Result<Version, WriteError> {
                self.0.write_if_version(doc, id, json, expected)
            }
            fn delete(&self, id: &str) -> Result<(), String> {
                self.0.delete(id)
            }
        }
        let log = RecordingLog::recording();
        let games = Games::new(Arc::new(Rules::standard()), Arc::new(OneUnreadable(MemStore::default())))
            .with_log(log.clone());
        let red = who("red@t.test");
        games.create(&two_people("healthy", CORPUS_SEED), &red).expect("a neighbour table");
        let malformed = serde_json::json!({
            "schema": 3, "id": "malformed", "name": null, "created_by": "red@t.test",
            "created_at": 1_789_000_000u64, "players": 2, "colors": ["Red", "Blue"],
            // `PlayerId::new(7)` panics; `check_shape` is what refuses it first.
            "start_seat": 7,
            "seats": [{ "kind": "human", "user": "red@t.test" }, { "kind": "human", "user": "blue@t.test" }],
            "keeps": [null, null], "seed": CORPUS_SEED, "status": "dealing", "turn": null,
        })
        .to_string();
        for (id, text) in [("unparsed", r#"{"schema":3}"#.to_string()), ("unread", "{}".to_string()), ("malformed", malformed)] {
            games.store().write_if_version(Doc::Table, id, &text, None).expect("the store takes what it is given");
        }
        let listed = games.list_for(&red).expect("one bad file does not fail the listing");
        assert_eq!(listed.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), ["healthy"]);
        let said: Vec<(&str, String, String)> = log
            .lines
            .lock()
            .unwrap()
            .iter()
            .map(|(event, f)| (*event, f["id"].as_str().unwrap().to_string(), f["why"].as_str().unwrap().to_string()))
            .collect();
        let line = |id: &str, why: &str| (crate::log::LISTING_SKIPPED, id.to_string(), why.to_string());
        assert_eq!(
            said,
            [line("malformed", "is malformed"), line("unparsed", "does not parse"), line("unread", "could not be read")],
        );
        let unread = log.fields(2);
        assert_eq!(unread["error"], "permission denied", "the store's own reason rides the line");
    }

    // ---------------------------------------------------------------------
    // The two archive sites (M6.5a, R-6.5a.11).
    // ---------------------------------------------------------------------

    fn stripped(table_json: &str) -> TableDoc {
        serde_json::from_str(table_json).expect("the archived table document reads")
    }

    /// The game-over site: one put, at the end, carrying the STORED bytes and a
    /// stripped table document; the stamp lands.
    #[test]
    fn a_game_that_ends_is_archived_once_with_the_stored_bytes() {
        let archive = RecordingArchive::recording();
        let games = games().with_archive(archive.clone());
        let (persists, repl) = play_to_the_end(&games, "over");
        // A both-bot pump plays a RUN (up to 200 moves) per call, so a 2P game is
        // two or three persists — enough that "once" is a claim and not a count.
        assert!(persists >= 2, "anti-vacuity: the game took {persists} persists to end");
        let puts = archive.puts.lock().unwrap();
        assert_eq!(puts.len(), 1, "one put at game over, not one per persist ({persists})");
        assert_eq!(puts[0].why, crate::archive::Why::Over);
        assert_eq!(puts[0].prefix, "", "this server's own games carry no caller prefix");
        assert_eq!(puts[0].id, "over");
        let (stored, _) = games.store().read(Doc::Record, "over").unwrap().unwrap();
        assert_eq!(puts[0].record, stored, "the archived record is the stored bytes, not a re-serialization");
        let (doc, _) = games.read_table("over").unwrap().unwrap();
        assert!(doc.archived_at.is_some(), "the stamp landed");
        let archived = stripped(&puts[0].table);
        assert_eq!(archived.archived_at, doc.archived_at, "the archived copy carries the same stamp");
        assert_eq!(archived.lock, None);
        assert!(archived.unlocked.is_empty());
        assert_eq!(archived.status, TableStatus::Playing, "anti-vacuity: it is the table's document");
        // A persist AFTER the end adds no second put.
        drop(puts);
        games.persist("over", &repl).unwrap();
        assert_eq!(archive.puts.lock().unwrap().len(), 1, "stamped tables are not put again");
    }

    /// The first failure rule: a put that fails at game over is SAID (M6.5c2:
    /// through the log seam, where it was an `eprintln!`), the move is still
    /// answered, and the stamp stays UNSET — the retry mark.
    #[test]
    fn a_failing_archive_at_game_over_still_answers_the_move() {
        let log = RecordingLog::recording();
        let games = games().with_archive(RecordingArchive::failing()).with_log(log.clone());
        let (_, _repl) = play_to_the_end(&games, "over-fails"); // every persist returned Ok
        let (doc, _) = games.read_table("over-fails").unwrap().unwrap();
        assert_eq!(doc.archived_at, None, "unset says a retry is owed");
        assert!(games.handle("over-fails").is_ok(), "the game is still served");
        assert_eq!(log.events(), [crate::log::OVER_PUT_REFUSED], "one line, at the persist that ended the game");
        assert_eq!(log.fields(0), serde_json::json!({ "id": "over-fails", "error": "disk on fire" }));
    }

    /// The abandon site: a playing table with events is put under `abandoned/`
    /// and THEN deleted.
    #[test]
    fn abandoning_a_playing_table_with_events_archives_then_deletes() {
        let archive = RecordingArchive::recording();
        let games = games().with_archive(archive.clone());
        let _repl = one_move_in(&games, "quit");
        games.delete("quit", &who("red@t.test")).unwrap();
        let puts = archive.puts.lock().unwrap();
        assert_eq!(puts.len(), 1);
        assert_eq!(puts[0].why, crate::archive::Why::Abandoned);
        assert_eq!(puts[0].id, "quit");
        assert_eq!(puts[0].prefix, "", "this server's own games carry no caller prefix");
        let archived = stripped(&puts[0].table);
        assert!(archived.archived_at.is_some(), "the archived copy is stamped");
        let record: GameRecord = serde_json::from_str(&puts[0].record).unwrap();
        assert!(!record.events.is_empty(), "anti-vacuity: the archived record has the move");
        assert!(games.read_table("quit").unwrap().is_none(), "deleted after the put");
        assert!(games.store().read(Doc::Record, "quit").unwrap().is_none());
    }

    /// R-6.5b.13: a FINISHED game with no stamp deleted by its creator lands
    /// under `over/`, because the MIRROR says over — a game finished before the
    /// archive was switched on, or one whose game-over put failed. **The
    /// residual, named:** `turn` is a cache `mirror_turn` may leave one move
    /// stale when its write fails or conflicts, so in exactly that case the
    /// leaf can still lie; `Games::handle(id).is_over` would not, at the cost of
    /// a rebuild and a replay under the table lock, and was declined. The cell
    /// claims what is true: the leaf follows the mirror.
    #[test]
    fn a_finished_unstamped_game_deleted_lands_under_over() {
        let games = games(); // archive OFF, so no stamp lands
        let (_, repl) = play_to_the_end(&games, "late");
        assert!(repl.session.is_over(), "premise: the game is over");
        let (doc, _) = games.read_table("late").unwrap().unwrap();
        assert_eq!(doc.archived_at, None, "premise: nothing stamped it");
        assert!(doc.turn.as_ref().is_some_and(|t| t.over), "premise: the mirror says over");
        let archive = RecordingArchive::recording();
        let games = games.with_archive(archive.clone());
        games.delete("late", &who("red@t.test")).expect("the creator's delete");
        let puts = archive.puts.lock().unwrap();
        assert_eq!(puts.len(), 1);
        assert_eq!(
            puts[0].why,
            crate::archive::Why::Over,
            "a finished game is archived as OVER at its delete, not as abandoned"
        );
        assert_eq!(puts[0].id, "late");
    }

    /// **Row 11's residual, as the positive it is** (M6.5c2 cycle D;
    /// R-6.5b.13): a game-over put that LANDED and whose stamp was LOST is
    /// archived ONCE at the next abandon. The table's mirror says over, so the
    /// abandon puts under `over/`, where the landed copy answers `Identical`
    /// and the delete goes ahead. What would put a second copy under
    /// `abandoned/` is a lost stamp AND a mirror that no longer says over — a
    /// stale one, or a true one after an undo past game over at a one-person
    /// table (measured in M6.5c2 cycle D and recorded in its close,
    /// `docs/plans/2026-09-29-web-frontend-m6.5c2d-close.md`) — the residual
    /// `delete`'s doc names and does not close.
    ///
    /// Over a real [`crate::archive::DirArchive`], because `Identical` is the
    /// directory's own answer to the same bytes, and a double that recorded the
    /// second put would say nothing about whether it lands.
    #[cfg(feature = "server")]
    #[test]
    fn a_landed_put_whose_stamp_failed_then_abandoned_lands_once_under_over() {
        use std::sync::atomic::{AtomicBool, Ordering};
        /// A store whose table writes CARRYING A STAMP fail while armed — the
        /// stamp's own write, and nothing else.
        struct StampRefusing {
            inner: MemStore,
            armed: AtomicBool,
        }
        impl Store for StampRefusing {
            fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
                self.inner.list(doc)
            }
            fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, crate::store::Version)>, String> {
                self.inner.read(doc, id)
            }
            fn write_if_version(
                &self,
                doc: Doc,
                id: &str,
                json: &str,
                expected: Option<crate::store::Version>,
            ) -> Result<crate::store::Version, WriteError> {
                let stamped = doc == Doc::Table && !json.contains("\"archived_at\": null");
                if stamped && self.armed.load(Ordering::SeqCst) {
                    return Err(WriteError::Io("the stamp's write failed".to_string()));
                }
                self.inner.write_if_version(doc, id, json, expected)
            }
            fn delete(&self, id: &str) -> Result<(), String> {
                self.inner.delete(id)
            }
        }
        let dir = std::env::temp_dir().join(format!("yokohama_row11_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Arc::new(StampRefusing { inner: MemStore::default(), armed: AtomicBool::new(true) });
        let log = RecordingLog::recording();
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>)
            .with_archive(Arc::new(crate::archive::DirArchive::new(&dir)))
            .with_log(log.clone());
        play_to_the_end(&games, "late");
        let (over, table) = crate::archive::paths(yokohama_core::SEMANTICS_REV, "", crate::archive::Why::Over, "late").unwrap();
        let (abandoned, _) =
            crate::archive::paths(yokohama_core::SEMANTICS_REV, "", crate::archive::Why::Abandoned, "late").unwrap();
        let stored = games.store().read(Doc::Record, "late").unwrap().expect("the record").0;
        assert_eq!(std::fs::read_to_string(dir.join(&over)).unwrap(), stored, "premise: the game-over put landed");
        assert_eq!(log.events(), [crate::log::OVER_UNSTAMPED], "premise: its stamp was lost, and said");
        let (doc, _) = games.read_table("late").unwrap().unwrap();
        assert_eq!(doc.archived_at, None, "premise: nothing stamped it");
        assert!(doc.turn.as_ref().is_some_and(|t| t.over), "premise: the mirror says over");

        store.armed.store(false, Ordering::SeqCst);
        games.delete("late", &who("red@t.test")).expect("the creator's delete, over a copy already landed");
        assert_eq!(std::fs::read_to_string(dir.join(&over)).unwrap(), stored, "ONE copy, under over/");
        assert!(dir.join(&table).is_file(), "with its table");
        assert!(!dir.join(&abandoned).exists(), "and none under abandoned/");
        assert!(games.read_table("late").unwrap().is_none(), "the table is gone");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Nothing to save, nothing put: an open table, a dealing table, a playing
    /// table with NO events, and a table already archived at game over.
    #[test]
    fn abandoning_an_open_dealing_moveless_or_archived_table_archives_nothing() {
        let archive = RecordingArchive::recording();
        let games = games().with_archive(archive.clone());
        let red = who("red@t.test");
        // Open.
        let mut open = two_people("open", CORPUS_SEED);
        open.seats[1] = SeatSpec { user: None, bot: false, open: true };
        games.create(&open, &red).unwrap();
        games.delete("open", &red).unwrap();
        // Dealing.
        games.create(&two_people("dealing", CORPUS_SEED), &red).unwrap();
        games.delete("dealing", &red).unwrap();
        // Playing, no move yet.
        let handle = playing(&games, &two_people("fresh", CORPUS_SEED), &["red@t.test", "blue@t.test"]);
        drop(handle);
        games.delete("fresh", &red).unwrap();
        assert_eq!(archive.puts.lock().unwrap().len(), 0, "open, dealing and moveless tables save nothing");
        // Finished — archived at over (one put) — then abandoned: no second put.
        play_to_the_end(&games, "done");
        assert_eq!(archive.puts.lock().unwrap().len(), 1);
        games.delete("done", &red).unwrap();
        assert_eq!(archive.puts.lock().unwrap().len(), 1, "the already-archived table added nothing");
        assert!(games.read_table("done").unwrap().is_none(), "and it was deleted");
    }

    /// **R-E5 (cycle E): `TableView.saves` is the delete's own rule, asked of
    /// the view** (ruling P-3). Six tables, each read through the ROUTE
    /// (`api::get_table`, so the archive switch the route hands the view is
    /// what is read) and through the creator's `mine` LISTING (`Games::list`,
    /// which builds the lobby row's view in a call of its own), and then
    /// deleted by its creator: the view's `saves`, the listing row's, the
    /// rule over whether the RECORD holds an event (what `delete_locked`
    /// reads) and the delete's own put count agree on every one. The view
    /// reads the MIRROR's bit, which `persist` stamps, so `has_events` is the
    /// record's here — the mirror is fresh at every one of these tables.
    ///
    /// **The `Playing` conjunct is NOT caught here**: a dealing table reached
    /// through the route holds no event, so its `saves` reads false with that
    /// conjunct dropped. Its catcher is `table.rs`'s
    /// `a_mirror_without_has_events_reads_false_and_the_view_saves_only_where_the_rule_says`,
    /// which sets the bit on a dealing document ("only a playing table is
    /// saved"). Here the moveless playing table catches a `saves` that drops
    /// the `has_events` conjunct, and the dealing table one that drops it and
    /// the status together; the archive-off table's put count is a COUNT (an
    /// archive that records and says `off`), not a premise.
    #[test]
    fn the_view_saves_exactly_where_the_delete_files_a_save() {
        /// A recording archive that reports itself OFF.
        struct Off(Arc<RecordingArchive>);
        impl crate::archive::Archive for Off {
            fn put(
                &self,
                prefix: &str,
                why: crate::archive::Why,
                id: &str,
                r: &str,
                t: &str,
            ) -> Result<crate::archive::PutOutcome, crate::archive::PutError> {
                self.0.put(prefix, why, id, r, t)
            }
            fn kind(&self) -> &'static str {
                "off"
            }
        }
        /// A playing two-person table with exactly ONE applied move persisted.
        fn one_event_in(games: &Games, id: &str) {
            let handle = playing(games, &two_people(id, CORPUS_SEED), &["red@t.test", "blue@t.test"]);
            let mut repl = handle.lock().unwrap();
            repl.step("1");
            assert_eq!(repl.session.record().events.len(), 1, "premise: one event");
            games.persist(id, &repl).expect("a persist");
        }
        let red = who("red@t.test");
        let check = |games: &Games, archive: &Arc<RecordingArchive>, id: &str, saves: bool| {
            let view = crate::api::get_table(games, &red, id).unwrap_or_else(|_| panic!("{id}: the table"));
            let record_events = games
                .store()
                .read(Doc::Record, id)
                .unwrap()
                .map_or(0, |(json, _)| serde_json::from_str::<GameRecord>(&json).unwrap().events.len());
            assert_eq!(view.has_events, record_events > 0, "{id}: the mirror's bit is the record's");
            assert_eq!(view.saves, saves, "{id}: the view");
            // **The LOBBY's row, too** (fix round 1, Important 1): the listing
            // builds its views in its own call, and the lobby row's Abandon is
            // what ruling 4a-1 words. Every listing file in the corpus comes
            // from an `Api` that keeps no archive, so this is the one place a
            // listing that dropped the host's switch is seen.
            let (rows, _) = games.list(&red, Scope::Mine, LIST_LIMIT_MAX, None).expect("the creator's listing");
            let row = rows.iter().find(|t| t.id == id).unwrap_or_else(|| panic!("{id}: listed under mine"));
            assert_eq!((row.has_events, row.saves), (view.has_events, saves), "{id}: the lobby row");
            let (doc, _) = games.read_table(id).unwrap().unwrap();
            assert_eq!(saves_on_abandon(&doc, record_events > 0, games.archive_on()), saves, "{id}: the rule");
            let before = archive.puts.lock().unwrap().len();
            games.delete(id, &red).expect("the creator's delete");
            let filed = archive.puts.lock().unwrap().len() - before;
            assert_eq!(filed, usize::from(saves), "{id}: the delete filed what the view said");
        };
        let on = RecordingArchive::recording();
        let with_on = games().with_archive(on.clone());
        // Dealing: nothing to save.
        with_on.create(&two_people("dealing", CORPUS_SEED), &red).unwrap();
        check(&with_on, &on, "dealing", false);
        // Playing with no move yet: nothing to save.
        drop(playing(&with_on, &two_people("fresh", CORPUS_SEED), &["red@t.test", "blue@t.test"]));
        check(&with_on, &on, "fresh", false);
        // Playing with one event and the archive on: saved.
        one_event_in(&with_on, "one");
        check(&with_on, &on, "one", true);
        // Finished and archived at game over: not saved twice.
        play_to_the_end(&with_on, "done");
        assert_eq!(on.puts.lock().unwrap().len(), 2, "premise: the game-over put landed");
        check(&with_on, &on, "done", false);
        // The same one-event table with the archive off: nothing to save.
        let off = RecordingArchive::recording();
        let with_off = games().with_archive(Arc::new(Off(off.clone())));
        one_event_in(&with_off, "one-off");
        check(&with_off, &off, "one-off", false);
        // Finished while the archive was off, abandoned once it is on: saved,
        // under `over/`.
        let late = RecordingArchive::recording();
        let with_late = {
            let g = games();
            play_to_the_end(&g, "late");
            g.with_archive(late.clone())
        };
        check(&with_late, &late, "late", true);
        assert_eq!(late.puts.lock().unwrap()[0].why, crate::archive::Why::Over);
    }

    /// The second failure rule: an abandon whose put fails is REFUSED, with a
    /// sentence naming the archive, and the table stays exactly as it was.
    #[test]
    fn an_abandon_whose_put_fails_is_refused_and_the_table_stays() {
        let games = games().with_archive(RecordingArchive::failing());
        let _repl = one_move_in(&games, "stuck");
        let red = who("red@t.test");
        let err = games.delete("stuck", &red).unwrap_err();
        assert!(
            matches!(&err, GameError::Server(m) if m.starts_with("the archive could not be written (") && m.ends_with("— the table was not deleted")),
            "{err}"
        );
        assert!(err.to_string().contains("disk on fire"), "the archive's own reason is in the sentence: {err}");
        let (doc, _) = games.read_table("stuck").unwrap().expect("nothing deleted");
        assert_eq!(doc.archived_at, None, "no stamp without a put");
        assert!(games.store().read(Doc::Record, "stuck").unwrap().is_some(), "the record is intact");
        assert!(games.handle("stuck").is_ok(), "no tombstone: the game still answers");
        // And a stranger's delete is still the seat refusal, before the archive
        // is asked at all.
        assert!(matches!(games.delete("stuck", &who("green@t.test")), Err(GameError::Forbidden(_))));
    }

    /// R-6.5a.10: a schema-2 document — everything M6.2 wrote — reads with
    /// `archived_at` absent, and the new stamp is bounded on write exactly as
    /// a message's `created_at` is.
    #[test]
    fn a_schema_two_table_document_reads_with_archived_at_absent_and_a_bound_is_kept_on_write() {
        let json = r#"{"schema":2,"id":"old","name":"old","created_by":"red@t.test",
            "created_at":1789000000,"players":2,"colors":["Red","Blue"],"start_seat":0,
            "seats":[{"kind":"human","user":"red@t.test"},{"kind":"human","user":"blue@t.test"}],
            "keeps":[null,null],"seed":7,"status":"dealing","turn":null}"#;
        let doc: TableDoc = serde_json::from_str(json).expect("a schema-2 document reads");
        assert_eq!(doc.schema, 2);
        assert_eq!(doc.archived_at, None);
        assert_eq!(check_shape(&doc), Ok(()), "and it is a shape this server still accepts");
        let mut bad = doc.clone();
        bad.archived_at = Some(crate::chat::MAX_CREATED_AT + 1);
        let why = check_shape(&bad).unwrap_err();
        assert!(why.contains("archived_at"), "{why}");
        let mut fine = doc;
        fine.archived_at = Some(crate::chat::MAX_CREATED_AT);
        assert_eq!(check_shape(&fine), Ok(()), "the ceiling itself is allowed");
    }

    /// Every M6.2 shape arm, read as the SENTENCE it answers with.
    ///
    /// Built in Rust rather than as JSON, and that is the point: the arms
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
                SeatDoc::Human { user: "red@t.test".into(), name: Some("Red".into()), bound_at_create: false },
                SeatDoc::Human { user: "blue@t.test".into(), name: None, bound_at_create: false },
            ],
            keeps: vec![None, None],
            seed: CORPUS_SEED,
            status: TableStatus::Dealing,
            turn: None,
            lock: None,
            unlocked: Vec::new(),
            archived_at: None,
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
            d.seats[0] = SeatDoc::Human { user: "red@t.test".into(), name: Some("n".repeat(65)), bound_at_create: false };
            d
        };
        let control_name = || {
            let mut d = valid();
            d.seats[1] =
                SeatDoc::Human { user: "blue@t.test".into(), name: Some("bl\u{7}ue".into()), bound_at_create: false };
            d
        };
        let winner_off_the_table = || {
            let mut d = valid();
            d.status = TableStatus::Playing;
            d.turn = Some(TurnMirror { to_move: 0, over: true, winner: Some(2), has_events: false, bot: None });
            d
        };
        // The other half of the mirror, and the one with the wider blast
        // radius: `TableDoc::view` hands `to_move` to `PlayerId::new`, which
        // panics above 3, and `Games::list_for` calls `view` on every row it
        // keeps. Without this arm the probe reads `check_shape=Ok(())` and
        // `view` panics `PlayerId must be 0-3, got 9`.
        let to_move_off_the_table = || {
            let mut d = valid();
            d.status = TableStatus::Playing;
            d.turn = Some(TurnMirror { to_move: 9, over: false, winner: None, has_events: false, bot: None });
            d
        };

        // M6.3's list: too long, and an entry `can_watch` could never match.
        let too_many_watchers = || {
            let mut d = valid();
            d.unlocked = (0..=UNLOCKED_CAP).map(|i| format!("w{i}@t.test")).collect();
            d
        };
        let unparsable_watcher = || {
            let mut d = valid();
            d.unlocked = vec!["..".to_string()];
            d
        };

        let cases: [(&str, TableDoc); 10] = [
            ("newer server", newer()),
            ("open seat", open_seat_while_dealing()),
            ("no open seat", open_with_no_open_seat()),
            ("salt", short_salt()),
            ("1 to 64", long_name()),
            ("control", control_name()),
            ("winner 2", winner_off_the_table()),
            ("to_move 9", to_move_off_the_table()),
            ("257 watchers", too_many_watchers()),
            ("watcher list", unparsable_watcher()),
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

    /// **The table's own name is bounded on read as a seat's is** (security
    /// S1b, finding D25): `create` and `join` apply `check_name` on write, and a
    /// document that arrives through the archive route or a restore never
    /// passed through them — a 900 KB `name` was filed verbatim.
    #[test]
    fn check_shape_bounds_the_tables_own_name() {
        // A playing two-person table, built inline as `archive.rs`'s
        // `locked_doc` builds one, with no lock: this module's cells keep no
        // builder for a playing document.
        let mut doc = TableDoc {
            schema: 3,
            id: "sim-2".into(),
            name: None,
            created_by: "a@x.test".into(),
            created_at: 1_789_000_000,
            players: 2,
            colors: vec![PlayerColor::Red, PlayerColor::Blue],
            start_seat: 0,
            seats: vec![
                SeatDoc::Human { user: "a@x.test".into(), name: None, bound_at_create: false },
                SeatDoc::Human { user: "b@x.test".into(), name: None, bound_at_create: false },
            ],
            keeps: vec![Some(1), Some(2)],
            seed: 7,
            status: TableStatus::Playing,
            turn: None,
            lock: None,
            unlocked: vec!["watcher@x.test".to_string()],
            archived_at: Some(1_789_000_001),
        };
        doc.name = Some("x".repeat(65));
        let said = check_shape(&doc).expect_err("65 characters");
        assert_eq!(said, "the table's name: a name is 1 to 64 characters");
        doc.name = Some("a\u{0007}b".to_string());
        assert_eq!(check_shape(&doc).expect_err("a control character"), "the table's name: a name has no control characters");
        doc.name = Some("x".repeat(64));
        assert!(check_shape(&doc).is_ok(), "64 is the bound");
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

    /// **Every writer holds a table's lock through `with_table_lock`** (finding
    /// B8, decision D-3): the helper is where "drop, then prune" lives, so this
    /// file must not take a lock by hand. The one site allowed to call
    /// `lock_for` is the helper itself, in `games.rs`. The six pre-reads are
    /// `require_table`'s, counted the same way.
    ///
    /// The scan reads the CODE, cut at the file's first column-0
    /// `#[cfg(test)]` (the layer's scan convention): uncut, this cell's own
    /// literals match, so the first assertion could never pass and each count
    /// would carry one call too many.
    #[test]
    fn no_writer_in_this_file_takes_a_lock_by_hand() {
        let whole = include_str!("tables.rs");
        let source = whole.split("\n#[cfg(test)]\n").next().expect("the code above the tests");
        assert!(source.len() < whole.len(), "the cut found the test module");
        assert!(!source.contains("lock_for("), "a writer calls lock_for directly — use with_table_lock");
        assert!(source.matches("with_table_lock(").count() >= 7, "the seven writers go through the helper");
        assert!(source.matches(".require_table(").count() >= 6, "the six pre-reads go through require_table");
    }

    /// A [`MemStore`] that parks the FIRST table read after it is armed, so a
    /// cell can look at the lock map while a writer waits on that read.
    struct ReadGate {
        inner: MemStore,
        armed: std::sync::atomic::AtomicBool,
        arrived: Mutex<std::sync::mpsc::Sender<()>>,
        release: Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl Store for ReadGate {
        fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
            self.inner.list(doc)
        }
        fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
            if doc == Doc::Table && self.armed.swap(false, std::sync::atomic::Ordering::SeqCst) {
                let _ = self.arrived.lock().expect("the gate").send(());
                let _ = self.release.lock().expect("the gate").recv();
            }
            self.inner.read(doc, id)
        }
        fn write_if_version(&self, doc: Doc, id: &str, json: &str, expected: Option<Version>) -> Result<Version, WriteError> {
            self.inner.write_if_version(doc, id, json, expected)
        }
        fn delete(&self, id: &str) -> Result<(), String> {
            self.inner.delete(id)
        }
    }

    /// **A writer at an absent id never makes a lock entry, not even for the
    /// length of its own call** (finding B8: the read before the lock). The
    /// helper prunes on every exit, so a count taken AFTER a refused write
    /// cannot tell a writer that read first from one that locked first — the
    /// entry is gone either way, and dropping a pre-read passes every other
    /// cell. This one parks each writer's FIRST table read and counts the map
    /// while it waits: read first, the map is empty; locked first, the entry
    /// is there. `settle` is not a row: at an absent id it never locks, since
    /// its own first read decides whether there is a repair to lock for.
    #[test]
    fn a_writer_at_an_absent_id_reads_before_it_takes_the_lock() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let (arrived_tx, arrived_rx) = channel();
        let (release_tx, release_rx) = channel();
        let store = Arc::new(ReadGate {
            inner: MemStore::default(),
            armed: AtomicBool::new(false),
            arrived: Mutex::new(arrived_tx),
            release: Mutex::new(release_rx),
        });
        let games = Arc::new(Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>));
        type Writer = fn(&Games, &User) -> Result<(), GameError>;
        let rows: [(&str, Writer); 6] = [
            ("join", |g, u| g.join("nosuch", u, PlayerId::new(1), None, None).map(drop)),
            ("keep", |g, u| g.keep("nosuch", u, PlayerId::new(0), 1, None).map(drop)),
            ("unlock", |g, u| g.unlock("nosuch", u, "x").map(drop)),
            ("delete", |g, u| g.delete("nosuch", u)),
            ("chat_append", |g, _| g.chat_append("nosuch", red_seat(), "hi").map(drop)),
            ("delete_chat", |g, u| g.delete_chat("nosuch", 1, u)),
        ];
        for (name, writer) in rows {
            store.armed.store(true, Ordering::SeqCst);
            let running = {
                let games = Arc::clone(&games);
                std::thread::spawn(move || writer(&games, &who("red@t.test")))
            };
            arrived_rx.recv_timeout(Duration::from_secs(10)).unwrap_or_else(|_| panic!("{name} reads the table"));
            let entries = games.locks.lock().expect("the map").len();
            release_tx.send(()).expect("the parked read is listening");
            let answer = running.join().expect("the writer's thread");
            assert_eq!(entries, 0, "{name} held a lock entry during its first read: it locked before it read");
            assert!(matches!(answer, Err(GameError::NotFound(_))), "{name}: {answer:?}");
        }
        assert_eq!(games.locks.lock().expect("the map").len(), 0);
    }

    /// A [`MemStore`] with GATES in it, so a test can choose an interleaving
    /// rather than race for one.
    ///
    /// Each gate is armed independently and fires exactly once: it signals that
    /// a thread has ARRIVED at that operation and then blocks until the test
    /// releases it. `park_record` sits in front of a `Doc::Record` write —
    /// which is `finalize`'s — `park_delete` inside `Store::delete`, after
    /// `Games::delete` has read the table and decided to remove it, and
    /// `park_table` (M6.2) in front of a `Doc::Table` write, which is where a
    /// `join` sits with its seat filled and its flip not yet on disk, and
    /// `park_after_delete` (M6.2) inside `Store::delete` AFTER the unlink — the
    /// one point at which a `create` of the same id finds both documents gone
    /// while the delete has not yet finished its bookkeeping.
    ///
    /// **`park_delete` and `park_after_delete` sit on either side of the same
    /// call and are not interchangeable**: the keep/finalize race needs the
    /// unlink to be PENDING, and the delete/create race needs it to have
    /// HAPPENED.
    ///
    /// Both channel ends tolerate a dropped peer, so a test that fails and
    /// unwinds while a thread is parked here does not turn one failure into
    /// two.
    struct RaceStore {
        inner: MemStore,
        park_record: std::sync::atomic::AtomicBool,
        park_delete: std::sync::atomic::AtomicBool,
        park_table: std::sync::atomic::AtomicBool,
        park_after_delete: std::sync::atomic::AtomicBool,
        at_record: Mutex<std::sync::mpsc::Sender<()>>,
        go_record: Mutex<std::sync::mpsc::Receiver<()>>,
        at_delete: Mutex<std::sync::mpsc::Sender<()>>,
        go_delete: Mutex<std::sync::mpsc::Receiver<()>>,
        at_table: Mutex<std::sync::mpsc::Sender<()>>,
        go_table: Mutex<std::sync::mpsc::Receiver<()>>,
        at_after_delete: Mutex<std::sync::mpsc::Sender<()>>,
        go_after_delete: Mutex<std::sync::mpsc::Receiver<()>>,
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
            if doc == Doc::Table
                && self.park_table.swap(false, std::sync::atomic::Ordering::SeqCst)
            {
                Self::wait(&self.at_table, &self.go_table);
            }
            self.inner.write_if_version(doc, id, json, expected)
        }
        fn delete(&self, id: &str) -> Result<(), String> {
            if self.park_delete.swap(false, std::sync::atomic::Ordering::SeqCst) {
                Self::wait(&self.at_delete, &self.go_delete);
            }
            let answer = self.inner.delete(id);
            if self.park_after_delete.swap(false, std::sync::atomic::Ordering::SeqCst) {
                Self::wait(&self.at_after_delete, &self.go_after_delete);
            }
            answer
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
        use std::time::{Duration, Instant};

        let (at_record_tx, at_record_rx) = channel();
        let (go_record_tx, go_record_rx) = channel();
        let (at_delete_tx, at_delete_rx) = channel();
        let (go_delete_tx, go_delete_rx) = channel();
        let (at_table_tx, _at_table_rx) = channel();
        let (_go_table_tx, go_table_rx) = channel();
        let (at_after_delete_tx, _at_after_delete_rx) = channel();
        let (_go_after_delete_tx, go_after_delete_rx) = channel();
        let store = Arc::new(RaceStore {
            inner: MemStore::default(),
            park_record: AtomicBool::new(false),
            park_delete: AtomicBool::new(false),
            park_table: AtomicBool::new(false),
            park_after_delete: AtomicBool::new(false),
            at_record: Mutex::new(at_record_tx),
            go_record: Mutex::new(go_record_rx),
            at_delete: Mutex::new(at_delete_tx),
            go_delete: Mutex::new(go_delete_rx),
            at_table: Mutex::new(at_table_tx),
            go_table: Mutex::new(go_table_rx),
            at_after_delete: Mutex::new(at_after_delete_tx),
            go_after_delete: Mutex::new(go_after_delete_rx),
        });
        let games =
            Arc::new(Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>));
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        let id = games.create(&two_people("raced", CORPUS_SEED), &red).expect("a table").id;
        // Seat 0 chooses. The stored table now reads `keeps: [1, null]`:
        // DEALING and NOT all-kept, which is the state `settle` passes over
        // without taking the lock — step 1's precondition.
        games.keep(&id, &red, PlayerId::new(0), 1, None).expect("red's keep");

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
            std::thread::spawn(move || games.keep(&id, &blue, PlayerId::new(1), 1, None))
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
        // still waiting for the table's lock.
        go_delete_tx.send(()).expect("the parked delete is still listening");
        go_record_tx.send(()).expect("a buffered release");
        // Bounded, so a lock that is never let go FAILS here, naming the
        // thread, rather than hanging the joins below.
        let deadline = Instant::now() + Duration::from_secs(5);
        while !(deleter.is_finished() && keeper.is_finished()) {
            if Instant::now() >= deadline {
                let unfinished: Vec<&str> = [("the delete thread", deleter.is_finished()), ("the keep thread", keeper.is_finished())]
                    .into_iter()
                    .filter_map(|(name, done)| (!done).then_some(name))
                    .collect();
                panic!("not finished within 5 s of the release: {}", unfinished.join(" and "));
            }
            std::thread::sleep(Duration::from_millis(2));
        }
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

    /// **A seated non-creator's delete names nobody** (M6.5c2 cycle D,
    /// R-6.5c2.35, R-D3): the refusal said `only {created_by} can delete this
    /// table`, which handed the creator's address to every other seat that
    /// pressed Abandon. The sentence now says who may, without saying who that
    /// is.
    #[test]
    fn a_seated_non_creators_delete_refusal_names_no_address() {
        let games = games();
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        let id = games.create(&two_people("named", CORPUS_SEED), &red).unwrap().id;
        match games.delete(&id, &blue) {
            Err(GameError::Forbidden(m)) => assert_eq!(m, "only the table's creator can delete this table"),
            other => panic!("a seated non-creator's delete is a 403: {other:?}"),
        }
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
                                games.keep(&id, &creator, seat, 1, None).expect("a keep");
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

    /// **An existing game whose record cannot be READ is not a free name — and
    /// the refusal is the STORE's failure, a 500 naming its error** (M6.5c2
    /// cycle D, straw poll row 7), never the 400 "already exists": nobody
    /// knows whether a game is there, so the sentence that says one is would be
    /// false, and a 400 tells the caller to change a request that was right.
    /// The refusal itself is what stops a new game overwriting a real one (the
    /// `chmod 000` measurement), so the store's write PANICS here: no 201, and
    /// the record untouched.
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
        let err = games.create(&req("taken"), &who("red@t.test")).expect_err("the name is not free");
        let sentence = "the store could not be read for \"taken\": permission denied";
        assert!(matches!(&err, GameError::Server(m) if m == sentence), "{err:?}");
        let answer = crate::ApiError::from(err);
        assert_eq!((answer.status.as_u16(), answer.message.as_str()), (500, sentence));
    }

    /// **An open seat opens the table; a full one deals at once.**
    ///
    /// Both halves in one cell because the claim is the RULE and not either
    /// reading: a `create` that always wrote `Open` would pass the first
    /// assertion and break every existing vs-bot table, and one that never wrote
    /// it would pass the second. `chosen` is asserted at the open seats too — an
    /// unfilled seat has not chosen, and reading `true` there would tell the
    /// lobby a table was ready to finalize.
    #[test]
    fn create_lands_open_when_any_seat_is_open_and_dealing_otherwise() {
        let games = games();
        let red = who("red@t.test");
        let mut r = req("open3");
        r.players = 3;
        r.colors = vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Green];
        r.seats = vec![
            SeatSpec { user: Some("red@t.test".into()), bot: false, open: false },
            SeatSpec { user: None, bot: false, open: true },
            SeatSpec { user: None, bot: false, open: true },
        ];
        let v = games.create(&r, &red).expect("an open table");
        assert_eq!(v.status, TableStatus::Open);
        assert_eq!(v.seats[1], SeatView::Open);
        assert_eq!(v.chosen, vec![false, false, false]);
        let v = games.create(&req("bot2"), &red).expect("a bot table");
        assert_eq!(v.status, TableStatus::Dealing, "a 2P bot table is full at create");
    }

    /// M6.2's four new refusals, in the order §3.1 lists them, and the one
    /// widening the open seat forces: an open seat is NOT a human seat, so a
    /// table of open seats and a bot has nobody at it yet and is refused by the
    /// rule that already refused an all-bot table.
    #[test]
    fn create_refuses_the_new_arms_in_order_and_an_open_seat_is_not_human() {
        let games = games();
        let red = who("red@t.test");
        let mut r = req("o");
        r.seats[1] = SeatSpec { user: None, bot: true, open: true };
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("exactly one"));
        let mut r = req("o");
        r.seats = vec![
            SeatSpec { user: None, bot: false, open: true },
            SeatSpec { user: None, bot: true, open: false },
        ];
        let e = games.create(&r, &red).unwrap_err().to_string();
        assert!(e.contains("human"), "open seats and a bot: no human yet: {e}");
        let mut r = req("o");
        r.name = Some("x".repeat(65));
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("1 to 64"));
        let mut r = req("o");
        r.display_name = Some("\u{7}".into());
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("control"));
        let mut r = req("o");
        r.password = Some("   ".into());
        assert!(
            games.create(&r, &red).unwrap_err().to_string().contains("at least one character")
        );
        let mut r = req("o");
        r.id = Some("bad id".into());
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("not a usable"));

        // **The ORDER, and only a probe wrong in TWO ways can see it.** Each of
        // the six above carries a single defect, so it says a rule EXISTS and
        // nothing about where in the chain it sits: `name` moved above the
        // creator check would leave every one of them green. Each probe below
        // is wrong at two ADJACENT links and asserts that the EARLIER sentence
        // is the one a client is handed — which is the whole claim of §3.1's
        // list, and of this cell's name.
        let mut r = req("o");
        r.seats[0] = SeatSpec { user: Some("other@t.test".into()), bot: false, open: false };
        r.name = Some("x".repeat(65));
        let e = games.create(&r, &red).unwrap_err().to_string();
        assert!(e.contains("creator"), "the creator check precedes the name: {e}");
        let mut r = req("o");
        r.players = 3;
        r.colors = vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Green];
        r.seats = vec![
            SeatSpec { user: Some("red@t.test".into()), bot: false, open: false },
            SeatSpec { user: None, bot: true, open: false },
            SeatSpec { user: Some("red@t.test".into()), bot: false, open: false },
        ];
        r.name = Some("x".repeat(65));
        let e = games.create(&r, &red).unwrap_err().to_string();
        assert!(e.contains("two players only"), "the bot check precedes the name: {e}");
        // `starts_with`, never `contains`: both sentences come out of
        // `check_name` and differ only in their prefix, and "name:" is a
        // SUBSTRING of "display name:" — so a `contains` here would pass on
        // either answer and pin nothing.
        let mut r = req("o");
        r.name = Some("x".repeat(65));
        r.display_name = Some("\u{7}".into());
        let e = games.create(&r, &red).unwrap_err().to_string();
        assert!(e.starts_with("name:"), "the name precedes the display name: {e}");
        let mut r = req("o");
        r.display_name = Some("\u{7}".into());
        r.password = Some("   ".into());
        let e = games.create(&r, &red).unwrap_err().to_string();
        assert!(e.starts_with("display name:"), "the display name precedes the password: {e}");
        let mut r = req("o");
        r.password = Some("   ".into());
        r.id = Some("bad id".into());
        let e = games.create(&r, &red).unwrap_err().to_string();
        assert!(e.contains("at least one character"), "the password precedes the id: {e}");
    }

    /// **A creator-chosen id with a dot is refused at create** (finding B9's
    /// second half): `evening.js` named a private frame to the edge's
    /// extension rule. The storage rule is untouched — a table stored before
    /// this cycle under `old.js` still loads, plays and deletes — so the fence
    /// is the create's alone. Every id the corpus and the client use is dotless.
    #[test]
    fn a_dotted_id_is_refused_at_create_and_a_stored_one_still_reads_and_deletes() {
        let games = games();
        let red = who("red@t.test");
        for id in ["x.js", "x.css", "x.png", "x.bin", "evening.2", "a.b.c"] {
            let mut req = two_people("dotted", CORPUS_SEED);
            req.id = Some(id.to_string());
            let err = games.create(&req, &red).expect_err(id);
            assert!(matches!(&err, GameError::BadRequest(m) if m == &format!("\"{id}\" {DOTTED_ID}")), "{id}: {err:?}");
        }
        // **The ORDER, by two-defect probes** (the M6.2 lesson): an id the
        // storage rule refuses AND that carries a dot reads the storage rule's
        // sentence, so the dotted arm sits after `is_safe_id`'s and never
        // speaks for an id no table could have been stored under.
        for id in ["..", "x.table"] {
            let mut req = two_people("dotted", CORPUS_SEED);
            req.id = Some(id.to_string());
            let err = games.create(&req, &red).expect_err(id);
            assert!(matches!(&err, GameError::BadRequest(m) if m == &format!("\"{id}\" is not a usable game id")), "{id}: {err:?}");
        }
        let mut req = two_people("dash-ok", CORPUS_SEED);
        req.id = Some("x-js".to_string());
        games.create(&req, &red).expect("a dash is fine");

        // Stored before the rule: a real document — the dotless table's, re-keyed
        // through the store under `old.js` — reads and deletes; the route-level
        // drive (`lib.rs`'s `a_stored_dotted_table_still_loads_plays_and_deletes`)
        // plays and streams one.
        games.create(&two_people("oldjs", CORPUS_SEED), &red).expect("a table");
        let (json, _) = games.store().read(Doc::Table, "oldjs").unwrap().expect("the table");
        let mut doc: TableDoc = serde_json::from_str(&json).unwrap();
        doc.id = "old.js".to_string();
        games.store().write_if_version(Doc::Table, "old.js", &serde_json::to_string(&doc).unwrap(), None).unwrap();
        assert!(games.table_for_anyone("old.js", &red).is_ok(), "a stored dotted table still reads");
        games.delete("old.js", &red).expect("and deletes");
    }

    /// **A creator's seed is honoured at a SOLO table only** (R-SEC8(b),
    /// finding B3): every human seat the creator's and none open. Under the
    /// production policy a seed beside an open seat or another person is a
    /// 400 in `SEED_NOT_SOLO`'s words and the same body without a seed is a
    /// 201; a seed beside a BOT is honoured, a bot being no person (Review
    /// Focus 5); and under `Diagnostic` — the fixtures' named opt-in, which
    /// `games()` sets — every shape is honoured, which is what every other
    /// cell in this module relies on. The refusal's place in `create`'s order
    /// — after `config_of`'s last, before the id's first — is pinned by two
    /// two-defect probes.
    #[test]
    fn a_creators_seed_is_honoured_at_a_solo_table_only() {
        use crate::games::SeedPolicy;
        let production = Games::new(Arc::new(Rules::standard()), Arc::new(MemStore::default()))
            .with_seed_policy(SeedPolicy::SoloOnly);
        let creator = who("red@t.test");
        // Another person.
        let err = production.create(&two_people("other", 7), &creator).expect_err("refused");
        assert!(matches!(&err, GameError::BadRequest(m) if m == SEED_NOT_SOLO), "{err:?}");
        // An open seat.
        let mut open = req_seeded("open", 7);
        open.seats[1] = SeatSpec { user: None, bot: false, open: true };
        let err = production.create(&open, &creator).expect_err("refused");
        assert!(matches!(&err, GameError::BadRequest(m) if m == SEED_NOT_SOLO), "{err:?}");
        // **The ORDER, by two-defect probes at the ADJACENT links** (the M6.2
        // lesson: a single-defect probe pins nothing about order). The seed is
        // judged after `config_of`'s last refusal, the password, and before the
        // id's first, an unusable id: wrong at the seed AND the link after, the
        // seed's sentence is the one handed back; wrong at the seed AND the link
        // before, `config_of`'s is.
        let mut after = open.clone();
        after.id = Some("bad id".into());
        let err = production.create(&after, &creator).expect_err("refused");
        assert!(matches!(&err, GameError::BadRequest(m) if m == SEED_NOT_SOLO), "the seed precedes the id: {err:?}");
        let mut before = open.clone();
        before.password = Some("   ".into());
        let err = production.create(&before, &creator).expect_err("refused");
        assert!(
            matches!(&err, GameError::BadRequest(m) if m == "a password is at least one character"),
            "config_of's last refusal precedes the seed: {err:?}"
        );
        // The other-person shape with NO seed: drawn, 201 (the open-seat shape unseeded is route cell (b)'s).
        let mut unseeded = two_people("other2", 7);
        unseeded.seed = None;
        assert!(production.create(&unseeded, &creator).is_ok(), "a drawn seed opens the table");
        // A bot is no person; a hot seat is one person.
        assert!(production.create(&req_seeded("bot", 7), &creator).is_ok(), "the bot's seat is nobody's");
        let mut hot = req_seeded("hot", 7);
        hot.seats[1] = SeatSpec { user: Some("red@t.test".into()), bot: false, open: false };
        assert!(production.create(&hot, &creator).is_ok(), "every human seat is the creator's");
        // The fixtures' opt-in: `games()` is `Diagnostic`, and the refused shape is honoured there.
        assert_eq!(games().seed_policy(), SeedPolicy::Diagnostic);
        assert!(games().create(&two_people("diag", 7), &creator).is_ok());
        // And a table opened under the refusal is NOT there: nothing was written.
        assert!(production.read_table("other").unwrap().is_none(), "a refused create wrote a table");
    }

    /// **The name is a label, the id is its own field, and the display name
    /// lands on the CREATOR's seats and nobody else's.**
    ///
    /// The three were one string until this cycle: `name` was the storage key,
    /// so a game could not be called "Thursday's game" and a person had no name
    /// at all. The second half is the fence — red creating a table that seats
    /// blue must not write red's chosen name onto blue's seat, which is the one
    /// way a display name could become a claim about somebody else.
    #[test]
    fn the_name_is_a_label_the_id_is_drawn_and_the_display_name_lands_on_the_creators_seats() {
        let games = games();
        let red = who("red@t.test");
        let mut r = two_people("ignored", CORPUS_SEED);
        r.id = None;
        r.name = Some("Thursday's game".into());
        r.display_name = Some("Ben Castle".into());
        r.seats[1] = SeatSpec { user: Some("red@t.test".into()), bot: false, open: false };
        let v = games.create(&r, &red).expect("a table");
        assert!(v.id.starts_with("sim-"), "drawn: {}", v.id);
        assert_eq!(v.name.as_deref(), Some("Thursday's game"));
        for s in &v.seats {
            assert_eq!(s, &SeatView::Human { name: Some("Ben Castle".into()), you: true, invited: None });
        }
        let mut r = two_people("named", CORPUS_SEED);
        r.display_name = Some("Ben Castle".into());
        let v = games.create(&r, &red).expect("a table");
        // Blue's seat carries no name, so its creator — and only its creator —
        // is handed the address they typed (R-D3).
        assert_eq!(
            v.seats[1],
            SeatView::Human { name: None, you: false, invited: Some("blue@t.test".into()) },
            "not blue's",
        );
    }

    /// **The password is trimmed on the way in and the view says only that one
    /// exists.** The salt and the hash are a password to attack offline, so the
    /// document keeps them; `locked` is the whole of what a client is told.
    #[test]
    fn a_password_at_create_locks_the_table_and_the_view_says_only_that() {
        let games = games();
        let red = who("red@t.test");
        let mut r = req("locked");
        r.password = Some(" sesame ".into());
        let v = games.create(&r, &red).expect("a table");
        assert!(v.locked);
        let (doc, _) = games.read_table("locked").unwrap().unwrap();
        assert!(doc.check_password("sesame"), "trimmed on the way in");
        assert!(!doc.check_password(" sesame "));
    }

    /// A table with seat 1 OPEN, optionally locked — the shape every join cell
    /// starts from.
    fn open_two(games: &Games, id: &str, password: Option<&str>) -> TableView {
        let mut r = req(id);
        r.seats[1] = SeatSpec { user: None, bot: false, open: true };
        r.password = password.map(str::to_string);
        games.create(&r, &who("red@t.test")).expect("an open table")
    }

    /// **Every refusal a join can raise, in the order a client reads them**
    /// (§3.2), and then the join that fills the last seat and FLIPS the table
    /// to dealing.
    ///
    /// One cell rather than seven, for the reason
    /// `create_refuses_in_the_order_a_client_reads_and_binds_the_creator` is
    /// one: the ORDER is the claim. A probe wrong in one way says a rule
    /// exists and nothing about where in the chain it sits — so the name probe
    /// below carries a CORRECT password at an OPEN seat, and the "taken" probe
    /// carries the password too, each asserting that the earlier link is not
    /// the one that answers.
    #[test]
    fn join_refuses_in_the_order_a_client_reads() {
        let games = games();
        let blue = who("blue@t.test");
        let s1 = PlayerId::new(1);
        assert!(matches!(games.join("nope", &blue, s1, None, None), Err(GameError::NotFound(_))));
        open_two(&games, "locked", Some("sesame"));
        assert!(matches!(
            games.join("locked", &blue, PlayerId::new(2), None, None),
            Err(GameError::BadRequest(_))
        ));
        let e = games.join("locked", &blue, s1, Some(&"x".repeat(65)), Some("sesame")).unwrap_err();
        assert!(e.to_string().contains("1 to 64"), "{e}");
        let e = games.join("locked", &blue, PlayerId::new(0), None, Some("sesame")).unwrap_err();
        assert!(matches!(e, GameError::Conflict(_)) && e.to_string().contains("taken"), "{e}");
        let e = games.join("locked", &blue, s1, None, None).unwrap_err();
        assert!(matches!(e, GameError::Forbidden(_)) && e.to_string().contains("locked"), "{e}");
        let e = games.join("locked", &blue, s1, None, Some("open")).unwrap_err();
        assert!(
            matches!(e, GameError::Forbidden(_)) && e.to_string().contains("wrong password"),
            "{e}"
        );

        // **The two ADJACENT pairs the probes above cannot see.** Each of the
        // six is wrong in ONE way apart from the seat probe, so it says a rule
        // exists and nothing about where it sits: `check_name` moved below the
        // taken check, or the taken check moved below the lock, would leave
        // every one of them green. These two are wrong at two adjacent links
        // and assert the EARLIER sentence is the one a client is handed, which
        // is the whole claim of this cell's name.
        let e = games
            .join("locked", &blue, PlayerId::new(0), Some(&"x".repeat(65)), Some("sesame"))
            .unwrap_err();
        assert!(e.to_string().contains("1 to 64"), "the name precedes the taken seat: {e}");
        let e = games.join("locked", &blue, PlayerId::new(0), None, Some("open")).unwrap_err();
        assert!(e.to_string().contains("taken"), "the taken seat precedes the password: {e}");

        let v = games.join("locked", &blue, s1, Some(" Alex "), Some("sesame")).expect("seated");
        assert_eq!(v.seats[1], SeatView::Human { name: Some("Alex".into()), you: true, invited: None });
        assert_eq!(v.my_seats, vec![s1]);
        assert_eq!(v.status, TableStatus::Dealing, "the filling join flips");
        assert!(matches!(games.join("locked", &blue, s1, None, Some("sesame")), Err(GameError::Dealing)));
    }

    /// **The seat and the flip are ONE write**, not two.
    ///
    /// Two writes are two crash windows and a moment at which the table is full
    /// and still open — a state `check_shape` would refuse to read back. The
    /// count is the only way to see it: both orderings leave the same document.
    #[test]
    fn the_filling_join_writes_the_seat_and_the_flip_in_one_write() {
        let store = Arc::new(CountingStore::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        open_two(&games, "one", None);
        let before = store.writes();
        games.join("one", &who("blue@t.test"), PlayerId::new(1), None, None).expect("seated");
        assert_eq!(store.writes() - before, 1, "one write for the seat AND the flip");
        let (doc, _) = games.read_table("one").unwrap().unwrap();
        assert_eq!(doc.status, TableStatus::Dealing);
        assert!(doc.open_seats().is_empty());
    }

    /// **A person may hold two seats, and a seat named by email is TAKEN.**
    ///
    /// The second half is the one an implementation gets wrong by checking "is
    /// this seat mine" instead of "is this seat open": red joining the seat red
    /// already holds must be refused, or a join becomes a way to re-bind a seat
    /// somebody is sitting at.
    #[test]
    fn a_person_may_take_two_seats_and_a_named_seat_is_taken() {
        let games = games();
        let red = who("red@t.test");
        let mut r = req("three");
        r.players = 3;
        r.colors = vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Green];
        r.seats = vec![
            SeatSpec { user: Some("red@t.test".into()), bot: false, open: false },
            SeatSpec { user: Some("green@t.test".into()), bot: false, open: false },
            SeatSpec { user: None, bot: false, open: true },
        ];
        games.create(&r, &red).expect("a table");
        let e = games.join("three", &red, PlayerId::new(1), None, None).unwrap_err();
        assert!(e.to_string().contains("taken"), "a named seat is Human and therefore taken: {e}");
        let v = games.join("three", &red, PlayerId::new(2), None, None).expect("a second seat");
        assert_eq!(v.my_seats, vec![PlayerId::new(0), PlayerId::new(2)]);
    }

    /// **A keep names its seat only if the seat has no name yet** (§3.5).
    ///
    /// The name belongs to the person who gave it first — at create, for the
    /// creator's own seats — so a later keep cannot rename a seat, and a seat
    /// bound by somebody else's create gets its name from the holder's first
    /// keep.
    #[test]
    fn keep_writes_the_name_when_the_seat_has_none_and_leaves_a_set_one_alone() {
        let games = games();
        let red = who("red@t.test");
        let blue = who("blue@t.test");
        let mut r = two_people("kn", CORPUS_SEED);
        r.display_name = Some("Ben Castle".into());
        games.create(&r, &red).expect("a table");
        games.keep("kn", &blue, PlayerId::new(1), 1, Some("Alex")).expect("blue keeps with a name");

        // **The 400, and it has to be probed HERE — at a table that is still
        // DEALING.** Below the finalizing keep this call meets the `Playing`
        // guard at the top of `keep` and never reaches `check_name` at all, so
        // a `keep` with no name check in it would answer `Playing` and leave
        // the assertion green. Seat 0 is red's, has not chosen, and the card is
        // valid, so nothing but the name can refuse it.
        let e = games.keep("kn", &red, PlayerId::new(0), 1, Some("\u{7}")).unwrap_err();
        assert!(
            matches!(e, GameError::BadRequest(_)) && e.to_string().contains("control"),
            "a bad name is a 400: {e}",
        );

        // And the refused keep wrote NOTHING: seat 0 can still choose, which it
        // could not if the refusal had left `keeps[0]` behind ("already
        // chosen"). This is also the last keep, so it finalizes.
        let v = games.keep("kn", &red, PlayerId::new(0), 1, Some("Not Ben")).expect("red keeps");
        assert_eq!(v.seats[0], SeatView::Human { name: Some("Ben Castle".into()), you: true, invited: None });
        assert_eq!(v.seats[1], SeatView::Human { name: Some("Alex".into()), you: false, invited: None });
        assert_eq!(v.status, TableStatus::Playing, "the last keep finalized");
    }

    /// **The lock is PER TABLE** (R-2.9): a finalize at one table does not
    /// block a join at another.
    ///
    /// The global `table_lock` M6.1 shipped serialized every table writer in
    /// the process against every other, so one table's finalize — which builds
    /// a session and writes a record — held the lobby's whole write path. With
    /// the lock split by id the two are independent, and the only way to SEE
    /// that is to park one and answer the other while it is parked: the record
    /// write inside A's finalize is the longest hold there is, and B's join
    /// completes underneath it.
    ///
    /// **The join runs on its own thread and is WAITED FOR with a timeout**,
    /// which is the difference between a cell that fails and a cell that hangs.
    /// Called inline, the mutation below blocks the test thread forever and the
    /// suite stops rather than reporting; on a thread, the waiting ends and the
    /// assertion names what happened. The gates are released and both threads
    /// joined BEFORE the assertion, so a failure leaves nothing parked.
    ///
    /// Measured against the global lock (M6.1's `table_lock` restored in `keep`
    /// and `join`): the join at B never answers inside the timeout, because it
    /// is waiting on the one lock the parked finalize holds.
    #[test]
    fn a_finalize_at_one_table_does_not_block_a_join_at_another() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let (at_record_tx, at_record_rx) = channel();
        let (go_record_tx, go_record_rx) = channel();
        let (at_delete_tx, _at_delete_rx) = channel();
        let (_go_delete_tx, go_delete_rx) = channel();
        let (at_table_tx, _at_table_rx) = channel();
        let (_go_table_tx, go_table_rx) = channel();
        let (at_after_delete_tx, _at_after_delete_rx) = channel();
        let (_go_after_delete_tx, go_after_delete_rx) = channel();
        let store = Arc::new(RaceStore {
            inner: MemStore::default(),
            park_record: AtomicBool::new(false),
            park_delete: AtomicBool::new(false),
            park_table: AtomicBool::new(false),
            park_after_delete: AtomicBool::new(false),
            at_record: Mutex::new(at_record_tx),
            go_record: Mutex::new(go_record_rx),
            at_delete: Mutex::new(at_delete_tx),
            go_delete: Mutex::new(go_delete_rx),
            at_table: Mutex::new(at_table_tx),
            go_table: Mutex::new(go_table_rx),
            at_after_delete: Mutex::new(at_after_delete_tx),
            go_after_delete: Mutex::new(go_after_delete_rx),
        });
        let games =
            Arc::new(Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>));
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        open_two(&games, "b", None);
        games.create(&two_people("a", CORPUS_SEED), &red).expect("a table");
        games.keep("a", &red, PlayerId::new(0), 1, None).expect("red's keep");

        store.park_record.store(true, Ordering::SeqCst);
        let finalizer = {
            let (games, blue) = (Arc::clone(&games), blue.clone());
            std::thread::spawn(move || games.keep("a", &blue, PlayerId::new(1), 1, None))
        };
        at_record_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("A's finalize reaches its record write and parks there");

        let (done_tx, done_rx) = channel();
        let joiner = {
            let (games, blue) = (Arc::clone(&games), blue.clone());
            std::thread::spawn(move || {
                let answer = games.join("b", &blue, PlayerId::new(1), None, None);
                let _ = done_tx.send(answer.is_ok());
                answer
            })
        };
        let answered = done_rx.recv_timeout(Duration::from_secs(10));

        go_record_tx.send(()).expect("the parked finalize is still listening");
        finalizer.join().expect("the keep thread").expect("A finalizes after the gate opens");
        let joined = joiner.join().expect("the join thread");
        assert_eq!(
            answered,
            Ok(true),
            "the join at B did not answer while A's finalize was parked — the two tables are \
             still sharing one lock. It eventually answered {joined:?}",
        );
    }

    /// **A `DELETE` may not interleave with the join that fills the last
    /// seat** — the sibling of
    /// `a_delete_cannot_interleave_with_the_keep_that_finalizes`, and the cell
    /// that says the per-table lock did not lose what the global one bought.
    ///
    /// The join is parked INSIDE the store at its table write, which is the one
    /// point at which a delete could unlink the document this join is about to
    /// replace. With the lock held by the join, the delete cannot reach
    /// `Store::delete` at all while it is parked — the middle assertion, and the
    /// one that goes red if `join` stops taking the lock. The two outcomes are
    /// then required to be CONSISTENT: whichever landed first, no table document
    /// exists after the delete, and the name is reusable.
    #[test]
    fn a_delete_cannot_interleave_with_the_join_that_fills_the_last_seat() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::mpsc::{channel, RecvTimeoutError};
        use std::time::Duration;

        let (at_record_tx, _at_record_rx) = channel();
        let (_go_record_tx, go_record_rx) = channel();
        let (at_delete_tx, at_delete_rx) = channel();
        let (go_delete_tx, go_delete_rx) = channel();
        let (at_table_tx, at_table_rx) = channel();
        let (go_table_tx, go_table_rx) = channel();
        let (at_after_delete_tx, _at_after_delete_rx) = channel();
        let (_go_after_delete_tx, go_after_delete_rx) = channel();
        let store = Arc::new(RaceStore {
            inner: MemStore::default(),
            park_record: AtomicBool::new(false),
            park_delete: AtomicBool::new(false),
            park_table: AtomicBool::new(false),
            park_after_delete: AtomicBool::new(false),
            at_record: Mutex::new(at_record_tx),
            go_record: Mutex::new(go_record_rx),
            at_delete: Mutex::new(at_delete_tx),
            go_delete: Mutex::new(go_delete_rx),
            at_table: Mutex::new(at_table_tx),
            go_table: Mutex::new(go_table_rx),
            at_after_delete: Mutex::new(at_after_delete_tx),
            go_after_delete: Mutex::new(go_after_delete_rx),
        });
        let games =
            Arc::new(Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>));
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        open_two(&games, "raced", None);

        store.park_table.store(true, Ordering::SeqCst);
        store.park_delete.store(true, Ordering::SeqCst);

        let joiner = {
            let (games, blue) = (Arc::clone(&games), blue.clone());
            std::thread::spawn(move || games.join("raced", &blue, PlayerId::new(1), None, None))
        };
        at_table_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the join reaches its table write with the seat filled and the flip pending");

        let deleter = {
            let (games, red) = (Arc::clone(&games), red.clone());
            std::thread::spawn(move || games.delete("raced", &red))
        };
        assert!(
            matches!(
                at_delete_rx.recv_timeout(Duration::from_millis(500)),
                Err(RecvTimeoutError::Timeout),
            ),
            "the delete reached the store while a join of the same table was mid-write — which is \
             the interleaving that unlinks a document another writer is about to write back",
        );

        go_table_tx.send(()).expect("the parked join is still listening");
        go_delete_tx.send(()).expect("a buffered release");
        let joined = joiner.join().expect("the join thread");
        let deleted = deleter.join().expect("the delete thread");

        // **Consistent, in either order.** With the lock the join lands first
        // and the delete then removes a dealing table; the assertion is that a
        // table document never outlives its own delete, and that the pair of
        // answers agrees with what is on disk.
        assert!(joined.is_ok(), "the join landed first under the lock: {joined:?}");
        assert!(deleted.is_ok(), "the delete then removed a dealing table: {deleted:?}");
        assert!(store.read(Doc::Table, "raced").expect("a readable store").is_none());
        assert!(store.read(Doc::Record, "raced").expect("a readable store").is_none());
        assert!(matches!(games.table("raced", &red), Err(GameError::NotFound(_))));
        games.create(&req("raced"), &red).expect("the name is reusable");
    }

    /// **A `DELETE` may not tombstone the table a `create` put in its place.**
    ///
    /// The window the lock split opened, in the order it happens with the
    /// tombstone written last:
    ///
    /// 1. `delete` takes the TABLE's lock and unlinks both documents;
    /// 2. `create` of the same id — holding the GLOBAL `table_lock`, which is a
    ///    different mutex since R-2.9, so nothing stops it — finds both
    ///    documents absent, clears the id from `deleted` (where it is not yet),
    ///    and writes a brand-new table;
    /// 3. `delete` inserts the id into `deleted`.
    ///
    /// The new table is now tombstoned. `Games::persist` reads that set under
    /// its save lock, before it writes (cycle F, Task 4b), so EVERY move at a
    /// game the player just created answers `"…" was deleted while this move
    /// was in flight` and evicts its handle —
    /// a game that cannot be played and says something untrue about why. Under
    /// M6.1 both calls took the one global mutex and the interleaving did not
    /// exist; none of that cycle's four race cells is delete-versus-create.
    ///
    /// **The gate is AFTER the unlink and that is the whole arrangement.**
    /// `park_delete`, which sits before it, would prove nothing here: with the
    /// documents still present the create refuses the name as occupied and
    /// never reaches step 2.
    ///
    /// Measured against the pre-fix ordering: `deleted` contains `"dc"` after
    /// both calls return, and the create's own table is unplayable.
    #[test]
    fn a_delete_cannot_tombstone_the_table_a_create_put_in_its_place() {
        // The gate is armed AT CONSTRUCTION rather than by a later `store`,
        // because nothing may run between the create below and the delete.
        use std::sync::atomic::AtomicBool;
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let (at_record_tx, _at_record_rx) = channel();
        let (_go_record_tx, go_record_rx) = channel();
        let (at_delete_tx, _at_delete_rx) = channel();
        let (_go_delete_tx, go_delete_rx) = channel();
        let (at_table_tx, _at_table_rx) = channel();
        let (_go_table_tx, go_table_rx) = channel();
        let (at_after_delete_tx, at_after_delete_rx) = channel();
        let (go_after_delete_tx, go_after_delete_rx) = channel();
        let store = Arc::new(RaceStore {
            inner: MemStore::default(),
            park_record: AtomicBool::new(false),
            park_delete: AtomicBool::new(false),
            park_table: AtomicBool::new(false),
            park_after_delete: AtomicBool::new(true),
            at_record: Mutex::new(at_record_tx),
            go_record: Mutex::new(go_record_rx),
            at_delete: Mutex::new(at_delete_tx),
            go_delete: Mutex::new(go_delete_rx),
            at_table: Mutex::new(at_table_tx),
            go_table: Mutex::new(go_table_rx),
            at_after_delete: Mutex::new(at_after_delete_tx),
            go_after_delete: Mutex::new(go_after_delete_rx),
        });
        let games =
            Arc::new(Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>));
        let red = who("red@t.test");
        games.create(&req("dc"), &red).expect("the table that is about to go");

        let deleter = {
            let (games, red) = (Arc::clone(&games), red.clone());
            std::thread::spawn(move || games.delete("dc", &red))
        };
        at_after_delete_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the delete has unlinked both documents and not yet finished");

        // Step 2, on its own thread and WAITED FOR, so a create that blocked
        // fails this cell instead of hanging it.
        let (made_tx, made_rx) = channel();
        let creator = {
            let (games, red) = (Arc::clone(&games), red.clone());
            std::thread::spawn(move || {
                let answer = games.create(&req("dc"), &red);
                let _ = made_tx.send(answer.is_ok());
                answer
            })
        };
        let made = made_rx.recv_timeout(Duration::from_secs(10));

        go_after_delete_tx.send(()).expect("the parked delete is still listening");
        deleter.join().expect("the delete thread").expect("the delete");
        let created = creator.join().expect("the create thread");
        assert_eq!(made, Ok(true), "the create of the freed name did not land: {created:?}");

        // **The claim.** The id names a table the player just made, and nothing
        // in this process thinks it was deleted.
        assert!(
            !games.deleted.lock().expect("games lock").contains("dc"),
            "the delete tombstoned the table the create put in its place",
        );
        // And the consequence a player would actually meet: the new table
        // finishes dealing and its first persisted move is not refused.
        games.keep("dc", &red, PlayerId::new(0), 1, None).expect("the new table finalizes");
        let handle = games.handle("dc").expect("the new table plays");
        let repl = handle.lock().expect("a game");
        games.persist("dc", &repl).expect("a move at the new table is not refused as deleted");
    }

    // ---------------------------------------------------------------------
    // The delete-versus-save window (cycle F, Task 4b; spec §2.4).
    // ---------------------------------------------------------------------

    /// Where [`ParkedRead`] parks the one read it is armed for.
    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Park {
        /// The read waits, THEN asks the store: it returns the store as it
        /// stands at the release.
        BeforeTheRead,
        /// The read asks the store, THEN waits: what it returns is fixed before
        /// the wait, whatever lands during it.
        AfterTheRead,
    }

    /// A `MemStore` that parks ONE armed read and says each time it unlinks
    /// (cycle F, Task 4b): the double the delete-versus-save cells drive.
    ///
    /// A delete and a save meet at two kinds of store call: the record READ
    /// (the delete's fixes the bytes it files; the save's decides whether its
    /// write replaces or creates) and the UNLINK. Parking one read on a chosen
    /// side of the store, and hearing the unlink, is every interleaving these
    /// cells force, and no sleep forces any of them. The arm is taken by the
    /// first read it matches, so every other read, the other party's included,
    /// passes straight through.
    struct ParkedRead {
        inner: MemStore,
        armed: Mutex<Option<(Doc, &'static str, Park)>>,
        at: Mutex<std::sync::mpsc::Sender<()>>,
        go: Mutex<std::sync::mpsc::Receiver<()>>,
        unlinked: Mutex<std::sync::mpsc::Sender<()>>,
    }

    /// The cell's ends of a [`ParkedRead`]'s gates: the parked read has
    /// arrived, release it, and an unlink has happened.
    struct ParkedReadGates {
        at: std::sync::mpsc::Receiver<()>,
        go: std::sync::mpsc::Sender<()>,
        unlinked: std::sync::mpsc::Receiver<()>,
    }

    impl ParkedRead {
        fn new() -> (Arc<Self>, ParkedReadGates) {
            use std::sync::mpsc::channel;
            let (at_tx, at) = channel();
            let (go, go_rx) = channel();
            let (unlinked_tx, unlinked) = channel();
            let store = Arc::new(Self {
                inner: MemStore::default(),
                armed: Mutex::new(None),
                at: Mutex::new(at_tx),
                go: Mutex::new(go_rx),
                unlinked: Mutex::new(unlinked_tx),
            });
            (store, ParkedReadGates { at, go, unlinked })
        }

        fn arm(&self, doc: Doc, id: &'static str, park: Park) {
            *self.armed.lock().expect("the arm") = Some((doc, id, park));
        }

        fn park(&self) {
            let _ = self.at.lock().expect("the gate").send(());
            let _ = self.go.lock().expect("the gate").recv();
        }
    }

    impl Store for ParkedRead {
        fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
            self.inner.list(doc)
        }
        fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
            let park = {
                let mut armed = self.armed.lock().expect("the arm");
                match *armed {
                    Some((d, i, park)) if d == doc && i == id => {
                        *armed = None;
                        Some(park)
                    }
                    _ => None,
                }
            };
            if park == Some(Park::BeforeTheRead) {
                self.park();
            }
            let answer = self.inner.read(doc, id);
            if park == Some(Park::AfterTheRead) {
                self.park();
            }
            answer
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
            let answer = self.inner.delete(id);
            let _ = self.unlinked.lock().expect("the gate").send(());
            answer
        }
    }

    /// A `Games` over `store` with a recording archive, so a cell reads the
    /// copy a delete filed.
    fn archived_over(store: &Arc<ParkedRead>) -> (Arc<Games>, Arc<RecordingArchive>) {
        let archive = RecordingArchive::recording();
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(store) as Arc<dyn Store>)
            .with_archive(archive.clone());
        (Arc::new(games), archive)
    }

    /// ONE move more on `repl`, applied and not saved, and the record's length
    /// with it. The game is not over after it: a game's LAST move files under
    /// `over/`, and these cells read `abandoned/`.
    fn a_late_move(repl: &mut Repl) -> usize {
        let mv = repl.session.legal().first().cloned().expect("a legal move");
        repl.session.apply(mv, Vec::new()).expect("a legal move applies");
        assert!(!repl.session.is_over(), "premise: the late move does not end the game");
        repl.session.record().events.len()
    }

    /// Each copy the archive took: where it was filed and how many events it holds.
    fn filed(archive: &RecordingArchive) -> Vec<(crate::archive::Why, usize)> {
        archive
            .puts
            .lock()
            .unwrap()
            .iter()
            .map(|p| (p.why, serde_json::from_str::<GameRecord>(&p.record).expect("the copy reads").events.len()))
            .collect()
    }

    /// **A save that lands while the delete files its copy is REFUSED, never
    /// lost** (cycle F, Task 4b; spec §2.4): variant 1 of the window Task 4's
    /// review read in `Games::delete`.
    ///
    /// The old order: the delete read the record, put it to the archive (two
    /// files, each flushed to disk), and only THEN laid the tombstone and
    /// unlinked, while a save checked the tombstone once, at its top. So a save
    /// whose check passed before the tombstone, and whose write landed after the
    /// delete's read and before the unlink, was ANSWERED SAVED; the unlink then
    /// removed it, and the copy under `abandoned/` lacked the move.
    ///
    /// The delete is parked AFTER its record read has answered: the bytes it
    /// files are fixed, and nothing is unlinked yet. The save runs whole in that
    /// gap, on its own thread, and is waited for with a bound, so a save that
    /// blocked on the delete fails this cell instead of hanging it. The delete
    /// holds the save lock only to lay its tombstone, never across its read or
    /// its put, so no save waits on the archive's disk. With the tombstone laid
    /// first, the save is refused out loud, in the sentence a save after a
    /// finished delete has always had, and the copy is the game as the delete
    /// read it.
    ///
    /// Measured against the old order: the save answers `Ok(())`, and the copy
    /// holds one event fewer than the save wrote.
    #[test]
    fn a_save_landing_while_the_delete_files_its_copy_is_refused_never_lost() {
        use std::time::Duration;
        let (store, gates) = ParkedRead::new();
        let (games, archive) = archived_over(&store);
        let red = who("red@t.test");
        let mut repl = one_move_in(&games, "late");
        let read_by_the_delete = repl.session.record().events.len();
        let saved_by_the_move = a_late_move(&mut repl);

        store.arm(Doc::Record, "late", Park::AfterTheRead);
        let deleter = {
            let (games, red) = (Arc::clone(&games), red.clone());
            std::thread::spawn(move || games.delete("late", &red))
        };
        gates.at.recv_timeout(Duration::from_secs(10)).expect("the delete reads the record it will file");

        let (answered_tx, answered_rx) = std::sync::mpsc::channel();
        let saver = {
            let games = Arc::clone(&games);
            std::thread::spawn(move || {
                let answer = games.persist("late", &repl);
                let _ = answered_tx.send(answer.clone());
                answer
            })
        };
        let in_the_gap = answered_rx.recv_timeout(Duration::from_secs(10));
        gates.go.send(()).expect("the parked delete is still listening");
        let deleted = deleter.join().expect("the delete thread");
        let saved = saver.join().expect("the save thread");

        let copies = filed(&archive);
        assert!(
            saved.is_err(),
            "the save was ANSWERED SAVED with {saved_by_the_move} events while the delete filed its copy, \
             and the copies hold {copies:?}: the move is lost",
        );
        assert_eq!(saved, Err(format!("{:?} was deleted while this move was in flight", "late")));
        assert_eq!(
            in_the_gap,
            Ok(saved.clone()),
            "the save did not answer while the delete was parked between its read and its put: the \
             delete holds the save lock across its archive work, and every save waits on the disk",
        );
        assert!(deleted.is_ok(), "{deleted:?}");
        assert_eq!(copies, [(crate::archive::Why::Abandoned, read_by_the_delete)], "the copy is the game as the delete read it");
        assert!(store.read(Doc::Record, "late").expect("a readable store").is_none(), "no record outlives its delete");
        assert!(store.read(Doc::Table, "late").expect("a readable store").is_none());
    }

    /// **A save that passed its tombstone check is filed with the game, and
    /// leaves no orphan** (cycle F, Task 4b; spec §2.4): variant 2 of the
    /// window, the ORPHAN RECORD.
    ///
    /// The old order: a save checked the tombstone at its top and only then
    /// took the save lock to read and write the record, and the delete took no
    /// save lock before its unlink. So a save that passed its check and read the
    /// store AFTER the unlink found no record and wrote as a create
    /// (`expected: None`, which asks for exactly an absent document). A record
    /// with no table then stood: unlisted, holding the name, and invisible to
    /// the sweep, which lists tables. `mirror_turn` found no table, so nothing
    /// repaired it.
    ///
    /// The save is parked INSIDE its record read, BEFORE the store answers:
    /// past its tombstone check, holding the save lock, at the one point where
    /// its read could still follow an unlink. With the tombstone laid under
    /// that same lock, the delete cannot lay it, read or unlink until the save's
    /// write has landed. A WAIT shows only as non-arrival, so the middle
    /// assertion is the form this file's M6.2 race cells use
    /// (`a_delete_cannot_interleave_with_the_keep_that_finalizes`,
    /// `a_delete_cannot_interleave_with_the_join_that_fills_the_last_seat`):
    /// the unlink does not arrive within 500 ms while the save is parked.
    /// Correct code cannot produce it there at all, so the bound never decides
    /// a pass. The copy the delete then files HOLDS the save's move. The cell
    /// also catches a tombstone laid before the read but NOT under the save
    /// lock: the delete then reads, files and unlinks around the parked save.
    ///
    /// Measured against the old order: the unlink arrives, and once both
    /// threads finish the store holds a record and no table.
    #[test]
    fn a_save_past_its_tombstone_check_is_filed_with_the_game_and_leaves_no_orphan() {
        use std::time::Duration;
        let (store, gates) = ParkedRead::new();
        let (games, archive) = archived_over(&store);
        let red = who("red@t.test");
        let mut repl = one_move_in(&games, "orphan");
        let saved_by_the_move = a_late_move(&mut repl);

        store.arm(Doc::Record, "orphan", Park::BeforeTheRead);
        let saver = {
            let games = Arc::clone(&games);
            std::thread::spawn(move || games.persist("orphan", &repl))
        };
        gates
            .at
            .recv_timeout(Duration::from_secs(10))
            .expect("the save passes its tombstone check and reaches its record read");
        let deleter = {
            let (games, red) = (Arc::clone(&games), red.clone());
            std::thread::spawn(move || games.delete("orphan", &red))
        };
        let unlinked_while_parked = gates.unlinked.recv_timeout(Duration::from_millis(500)).is_ok();
        gates.go.send(()).expect("the parked save is still listening");
        let saved = saver.join().expect("the save thread");
        let deleted = deleter.join().expect("the delete thread");

        let record = store.read(Doc::Record, "orphan").expect("a readable store");
        let table = store.read(Doc::Table, "orphan").expect("a readable store");
        assert!(
            !(record.is_some() && table.is_none()),
            "a RECORD WITH NO TABLE: the save passed its check before the delete's tombstone, read the \
             store after the unlink and created the record (unlisted, holding the name, invisible to the sweep)",
        );
        assert!(
            !unlinked_while_parked,
            "the delete unlinked the game while a save that had passed its tombstone check was mid-write",
        );
        assert_eq!(saved, Ok(()), "a save that began before the tombstone is saved");
        assert!(deleted.is_ok(), "{deleted:?}");
        assert!(record.is_none() && table.is_none(), "the delete removed both documents");
        assert_eq!(
            filed(&archive),
            [(crate::archive::Why::Abandoned, saved_by_the_move)],
            "the copy holds the save's move",
        );
    }

    /// **A save reads the tombstone only while it holds the save lock** (cycle
    /// F, Task 4b): the catcher that no race cell can be.
    ///
    /// With the check made BEFORE the lock, a save parked inside its read still
    /// holds the lock, so
    /// `a_save_past_its_tombstone_check_is_filed_with_the_game_and_leaves_no_orphan`
    /// passes. Yet a save that passed its check and then WAITED for the lock,
    /// behind another game's save, reads after any tombstone and unlink laid in
    /// the meantime: variant 2 again, reached through a wait that no store
    /// double can park.
    ///
    /// So the order is read off the locks themselves, with no thread and no
    /// bound. The tombstone set is POISONED (a writer died holding it, as the
    /// poisoned-lock cells in `games.rs` arrange), and the save panics where it
    /// reads the set. A panic poisons only the mutexes held at that moment, so
    /// the save lock is poisoned too exactly when the check ran under it.
    /// Measured against the old order: the save lock is clean.
    #[test]
    fn a_save_reads_the_tombstone_only_while_it_holds_the_save_lock() {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let games = games();
        let repl = one_move_in(&games, "held");
        let died = catch_unwind(AssertUnwindSafe(|| {
            let _held = games.deleted.lock().unwrap();
            panic!("a writer died holding the tombstone set");
        }));
        assert!(died.is_err() && games.deleted.is_poisoned(), "premise: the tombstone set is poisoned");
        assert!(!games.written.is_poisoned(), "premise: the save lock is not");
        let saved = catch_unwind(AssertUnwindSafe(|| games.persist("held", &repl)));
        assert!(saved.is_err(), "premise: the save reads the tombstone set, and its poison panics the save");
        assert!(
            games.written.is_poisoned(),
            "the save read the tombstone WITHOUT holding the save lock: a save past its check can still \
             wait for the lock while a delete tombstones, reads and unlinks",
        );
    }

    /// **A delete whose put fails LIFTS its tombstone** (cycle F, Task 4b).
    /// With the tombstone laid before the record is read, a refused put must
    /// take it back up, or every later save at a table still standing is
    /// refused as deleted. The cell pins the pre-delete state exactly: the
    /// tombstone absent, the table listed, and the next save accepted.
    ///
    /// Against the old order this held without any lift, because the tombstone
    /// came after the put. It is red against the new order without the lift.
    #[test]
    fn a_delete_whose_put_fails_lifts_its_tombstone_and_the_next_save_lands() {
        let games = games().with_archive(RecordingArchive::failing());
        let red = who("red@t.test");
        let mut repl = one_move_in(&games, "lifted");
        let err = games.delete("lifted", &red).unwrap_err();
        assert!(
            matches!(&err, GameError::Server(m) if m.ends_with("— the table was not deleted")),
            "premise: the put failed and the delete was refused: {err}",
        );
        assert!(
            !games.deleted.lock().expect("games lock").contains("lifted"),
            "the refused delete left its tombstone down: every later save at this table is refused as deleted",
        );
        assert!(
            games.list_for(&red).expect("a listing").iter().any(|t| t.id == "lifted"),
            "the table is still listed",
        );
        let events = a_late_move(&mut repl);
        games.persist("lifted", &repl).expect("a save after the refused delete is accepted");
        let (stored, _) = games.store().read(Doc::Record, "lifted").unwrap().expect("the record");
        let stored: GameRecord = serde_json::from_str(&stored).expect("the record reads");
        assert_eq!(stored.events.len(), events, "the save landed");
    }

    /// **A refused join writes NOTHING**, read as the stored bytes and the
    /// version rather than as the absence of a visible change.
    ///
    /// A 500-character name is the probe because it is refused LATE — after the
    /// document has been read and the seat found open — so a `join` that
    /// mutated its copy and wrote before validating would pass every other cell
    /// here and be caught only by this one.
    #[test]
    fn a_five_hundred_character_name_on_join_is_refused_and_the_document_is_unchanged() {
        let games = games();
        let blue = who("blue@t.test");
        open_two(&games, "long", None);
        let before = games.store().read(Doc::Table, "long").unwrap().expect("a table document");
        let e = games.join("long", &blue, PlayerId::new(1), Some(&"x".repeat(500)), None).unwrap_err();
        assert!(matches!(e, GameError::BadRequest(_)) && e.to_string().contains("1 to 64"), "{e}");
        let after = games.store().read(Doc::Table, "long").unwrap().expect("a table document");
        assert_eq!(before.0, after.0, "the stored bytes are unchanged");
        assert_eq!(before.1, after.1, "and so is the version");
    }

    /// **The two scopes are two different questions over one set of documents**
    /// (§3.3), and the sort is `(created_at desc, id asc)` in both.
    ///
    /// `mine` is every table this caller holds a seat at WHATEVER its status —
    /// so the open one red made is on it beside the dealing one — and `open` is
    /// every open table WHOEVER made it, so blue's is on red's `open` listing
    /// while blue's dealing table is on neither of red's.
    ///
    /// **The sleep is what makes this cell about the sort's primary key.**
    /// `created_at` is unix SECONDS (R-plan-1), so three creates in one second
    /// are one instant and every assertion below would be about the id
    /// tie-break instead — the same reason
    /// `the_list_is_the_tables_i_sit_at_newest_first_with_whose_turn_it_is`
    /// pays for one. `o1` is made first and alone; `mine-only` and `o2` land
    /// after it and are never compared with each other, falling as they do in
    /// different scopes.
    ///
    /// The last line is R-2.7 read from the LISTING rather than from a route: a
    /// row on the `open` scope may be a table this caller has no seat at, so
    /// `my_seats` is empty on one of these two rows and not on the other.
    #[test]
    fn the_two_scopes_list_by_construction_newest_first() {
        let games = games();
        let red = who("red@t.test");
        let blue = who("blue@t.test");
        open_two(&games, "o1", None);
        // One tick per create under the test clock — no wall-clock sleep needed.
        games.create(&two_people("mine-only", CORPUS_SEED), &red).unwrap();
        let mut r = req("o2");
        r.seats = vec![
            SeatSpec { user: Some("blue@t.test".into()), bot: false, open: false },
            SeatSpec { user: None, bot: false, open: true },
        ];
        games.create(&r, &blue).unwrap();
        let (mine, next) = games.list(&red, Scope::Mine, 20, None).unwrap();
        assert_eq!(mine.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), vec!["mine-only", "o1"]);
        assert!(next.is_none());
        let (open, _) = games.list(&red, Scope::Open, 20, None).unwrap();
        assert_eq!(open.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), vec!["o2", "o1"]);
        assert!(open.iter().all(|t| t.status == TableStatus::Open));
        assert!(open[0].my_seats.is_empty() && open[1].my_seats == vec![PlayerId::new(0)]);
    }

    /// **The cursor pages without a gap and without an overlap** (§3.3).
    ///
    /// Five tables in one second, so every row shares a `created_at` and the
    /// whole of the ordering is the id tie-break — which is the case the cursor
    /// has to get right, since `created_at` alone cannot separate these rows and
    /// a cursor comparing it alone would either repeat the page it just served
    /// or skip past it.
    ///
    /// The walk asserts the WHOLE SET at the end rather than each page, because
    /// a duplicate and an omission are the two failures and only the set sees
    /// both: an off-by-one at the boundary (`>=` where the code says `>`) hands
    /// `p1` back on the second page, and `seen` then has six entries against
    /// five.
    #[test]
    fn the_cursor_pages_without_gap_or_overlap() {
        let games = games();
        let red = who("red@t.test");
        for i in 0..5 {
            games.create(&req(&format!("p{i}")), &red).unwrap();
        }
        let mut seen: Vec<String> = Vec::new();
        let mut before = None;
        loop {
            let (page, next) = games.list(&red, Scope::Mine, 2, before.clone()).unwrap();
            assert!(page.len() <= 2);
            seen.extend(page.iter().map(|t| t.id.clone()));
            match next {
                Some(cursor) => {
                    let (at, id) = cursor.split_once(',').expect("created_at,id");
                    before = Some((at.parse::<u64>().unwrap(), id.to_string()));
                }
                None => break,
            }
        }
        seen.sort();
        assert_eq!(seen, (0..5).map(|i| format!("p{i}")).collect::<Vec<_>>());
    }

    /// **Both scopes read TABLE documents alone** (§3.3), proven by a store
    /// whose record reader panics.
    ///
    /// The fence earns a store of its own because the cheap way to list would
    /// be the expensive one: `to_move`, `over` and `winner` all live in the
    /// session, and a listing that reached for them would rebuild every game on
    /// the volume on every poll of the lobby. The turn mirror exists so that it
    /// does not, and this is what says so.
    ///
    /// **Armed AFTER the arranging, and that is not a weakening.** `create`
    /// asks the store about both documents — an id is taken if either is
    /// present — so a store that panicked from the first call would refuse to
    /// make a table at all and this cell could never reach the calls it is
    /// about. The flag is set between the arranging and the acting, and the
    /// acting is the part under test.
    ///
    /// **Three tables, one at each status, and the PLAYING one is the point.**
    /// A record is only written at finalize, so an open table and a dealing
    /// table have none on the volume — a panicking reader armed against those
    /// two alone could never fire, whatever the listing did, and the cell would
    /// be arguing from a temptation that was not on the disk. The playing table
    /// is the one with a record to read and with `to_move`, `over` and `winner`
    /// to be tempted by; it is what makes the assertion reachable.
    #[test]
    fn a_listing_reads_no_record_and_builds_no_session() {
        struct RecordPanics {
            inner: MemStore,
            armed: std::sync::atomic::AtomicBool,
        }
        impl RecordPanics {
            fn armed(&self) -> bool {
                self.armed.load(std::sync::atomic::Ordering::SeqCst)
            }
        }
        impl Store for RecordPanics {
            fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
                self.inner.list(doc)
            }
            fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
                assert!(
                    !(doc == Doc::Record && self.armed()),
                    "the listing read a RECORD for {id}"
                );
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
        let store = Arc::new(RecordPanics {
            inner: MemStore::default(),
            armed: std::sync::atomic::AtomicBool::new(false),
        });
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let red = who("red@t.test");
        open_two(&games, "o", None);
        games.create(&two_people("d", CORPUS_SEED), &red).unwrap();
        // The third status, and the only one with a record behind it — see the
        // doc. Built before the flag goes up, since finalizing writes and then
        // reads exactly the document this store is about to refuse.
        drop(playing(&games, &req_seeded("p", CORPUS_SEED), &["red@t.test"]));
        assert!(
            games.store().read(Doc::Record, "p").unwrap().is_some(),
            "the playing table has a record on the store, which is what arms this cell",
        );
        store.armed.store(true, std::sync::atomic::Ordering::SeqCst);
        let (mine, _) = games.list(&red, Scope::Mine, 20, None).unwrap();
        assert_eq!(mine.len(), 3, "all three statuses are on the listing: {mine:?}");
        // Read off the TABLE's own turn mirror and never off the game.
        let played = mine.iter().find(|t| t.id == "p").expect("the playing table");
        assert_eq!((played.status, played.over), (TableStatus::Playing, Some(false)));
        assert!(played.to_move.is_some(), "whose turn it is, from the mirror: {played:?}");
        games.list(&red, Scope::Open, 20, None).unwrap();
    }

    /// **An open table reads for anybody signed in; every other status keeps
    /// R-1.5's 403** (R-2.7).
    ///
    /// The dealing half is the anti-vacuity partner and is the half with teeth:
    /// a widening written as "any caller may read a table" would pass the first
    /// three lines and hand a stranger the seats, the colours and the status of
    /// every game on the volume.
    #[test]
    fn an_open_table_is_readable_by_a_stranger_and_a_dealing_one_is_not() {
        let games = games();
        let stranger = who("nobody@t.test");
        open_two(&games, "o", None);
        let doc = games.table_for_anyone("o", &stranger).expect("an open table reads");
        assert!(doc.view(&stranger, false).my_seats.is_empty());
        games.create(&two_people("d", CORPUS_SEED), &who("red@t.test")).unwrap();
        assert!(matches!(games.table_for_anyone("d", &stranger), Err(GameError::Forbidden(_))));
        assert!(matches!(games.table_for_anyone("nope", &stranger), Err(GameError::NotFound(_))));
    }

    /// **A create bumps the table and the house** (spec §2).
    ///
    /// The house because a new table belongs on somebody's lobby; the table's
    /// own channel because `subscribe` makes a sender for an id nobody has
    /// written yet, so a client that opened a link before the table existed
    /// wakes on its first write rather than waiting for its second.
    #[test]
    fn a_create_bumps_the_table_and_the_house() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        let mut rx = signals.subscribe("made");
        let house = signals.subscribe_house();
        assert_eq!(rx.borrow().seq, 0, "nothing has happened at this id yet");

        games.create(&req("made"), &who("red@t.test")).expect("a table");

        assert!(rx.has_changed().expect("the table channel is open"));
        assert_eq!(rx.borrow_and_update().seq, 1);
        assert!(house.has_changed().expect("the house channel is open"));
        // The state a woken subscriber reads is the state after the write.
        assert!(games.read_table("made").expect("a readable store").is_some());
    }

    /// **A create that is REFUSED bumps nothing.** The refusal here is the
    /// occupied name, which is the one `create` raises AFTER it has drawn a
    /// seed and built a document and BEFORE it writes — the window a bump
    /// placed anywhere but the success arm falls into.
    #[test]
    fn a_refused_create_bumps_nothing() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        let red = who("red@t.test");
        games.create(&req("twice"), &red).expect("a table");

        let rx = signals.subscribe("twice");
        let house = signals.subscribe_house();
        games.create(&req("twice"), &red).expect_err("the name is taken");

        assert!(!rx.has_changed().expect("the table channel is open"), "no write, no bump");
        assert!(!house.has_changed().expect("the house channel is open"));
    }

    /// **A join bumps the table and the house**: a seat filled is something
    /// the open table's own screen shows, and a table that left the open list
    /// is something the lobby shows.
    #[test]
    fn a_join_bumps_the_table_and_the_house() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        open_two(&games, "j", None);
        let mut rx = signals.subscribe("j");
        let mut house = signals.subscribe_house();
        rx.borrow_and_update();
        house.borrow_and_update();

        games
            .join("j", &who("blue@t.test"), PlayerId::new(1), None, None)
            .expect("the open seat");

        assert!(rx.has_changed().expect("the table channel is open"));
        assert!(house.has_changed().expect("the house channel is open"));
        let (doc, _) = games.read_table("j").expect("a readable store").expect("the table");
        assert_eq!(doc.status, TableStatus::Dealing, "the seat that was filled is on the store");
    }

    /// **A join that is REFUSED bumps nothing**, driven through the wrong
    /// password — a refusal that happens after the table has been read and
    /// the seat found open, so nothing but the write itself separates it from
    /// the success arm.
    #[test]
    fn a_refused_join_bumps_nothing() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        open_two(&games, "jr", Some("hunter2"));
        let rx = signals.subscribe("jr");
        let house = signals.subscribe_house();

        games
            .join("jr", &who("blue@t.test"), PlayerId::new(1), None, Some("nope"))
            .expect_err("the wrong password");

        assert!(!rx.has_changed().expect("the table channel is open"), "no write, no bump");
        assert!(!house.has_changed().expect("the house channel is open"));
    }

    /// **A keep that does not finalize bumps the TABLE and not the house**
    /// (spec §2).
    ///
    /// Nothing a lobby row shows has changed — the table is still dealing,
    /// still at the same seats — but the dealing screen's "waiting on" line
    /// has, so the two channels answer differently and this is the one site
    /// where they do.
    ///
    /// **Not the last keep**, deliberately: the finalize that a last keep
    /// triggers DOES bump the house, so a cell written over a 1-human table
    /// would assert the opposite of what it claims to and pass.
    #[test]
    fn a_keep_that_does_not_finalize_bumps_the_table_and_not_the_house() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        games.create(&two_people("k", CORPUS_SEED), &red).expect("a table");
        let mut rx = signals.subscribe("k");
        let mut house = signals.subscribe_house();
        rx.borrow_and_update();
        house.borrow_and_update();

        games.keep("k", &red, PlayerId::new(0), 1, None).expect("red's keep");

        assert!(rx.has_changed().expect("the table channel is open"), "the dealing screen moved");
        assert!(!house.has_changed().expect("the house channel is open"), "the lobby did not");
        let (doc, _) = games.read_table("k").expect("a readable store").expect("the table");
        assert_eq!(doc.keeps, vec![Some(1), None], "the keep a woken subscriber reads");
        // The premise: this keep was NOT the last one.
        assert_eq!(doc.status, TableStatus::Dealing);

        // And the LAST keep, through `finalize` -> `flip_to_playing`, reaches
        // the house.
        games.keep("k", &blue, PlayerId::new(1), 1, None).expect("blue's keep");
        assert!(house.has_changed().expect("the house channel is open"), "the finalize did");
        let (doc, _) = games.read_table("k").expect("a readable store").expect("the table");
        assert_eq!(doc.status, TableStatus::Playing);
    }

    /// **A keep that is REFUSED bumps nothing**, driven through a second keep
    /// at a seat that has already chosen.
    #[test]
    fn a_refused_keep_bumps_nothing() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        let red = who("red@t.test");
        games.create(&two_people("kr", CORPUS_SEED), &red).expect("a table");
        games.keep("kr", &red, PlayerId::new(0), 1, None).expect("red's keep");

        let rx = signals.subscribe("kr");
        let house = signals.subscribe_house();
        games.keep("kr", &red, PlayerId::new(0), 2, None).expect_err("seat 0 has already chosen");

        assert!(!rx.has_changed().expect("the table channel is open"), "no write, no bump");
        assert!(!house.has_changed().expect("the house channel is open"));
    }

    /// **A delete ENDS the table's channel and bumps the house.**
    ///
    /// Two different things, and both are the point: every stream open on the
    /// table has to close (there is no table left to serve), and every lobby
    /// has to drop the row. The terminal bump carries `end: true`, which is
    /// the only way a subscriber can tell "go and read it again" from "there
    /// is nothing to read".
    ///
    /// **The channel then CLOSES, and the order of the two assertions below
    /// is what says so.** `end` drops the sender after modifying the value, so
    /// the terminal bump is readable — a `watch` receiver keeps the last value
    /// past its sender — while `has_changed` answers `Err`. A cell that asked
    /// `has_changed` FIRST would read that `Err` and never see the `end` flag
    /// at all, which is how this one was written and what it failed on.
    #[test]
    fn a_delete_ends_the_table_and_bumps_the_house() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        let red = who("red@t.test");
        games.create(&req("gone"), &red).expect("a table");
        let mut rx = signals.subscribe("gone");
        let mut house = signals.subscribe_house();
        rx.borrow_and_update();
        house.borrow_and_update();

        games.delete("gone", &red).expect("a delete");

        assert!(rx.borrow_and_update().end, "the end a stream closes on");
        assert!(rx.has_changed().is_err(), "and then the channel closes under every stream");
        assert!(house.has_changed().expect("the house channel is open"));
        assert!(games.read_table("gone").expect("a readable store").is_none());
    }

    /// **A delete that is REFUSED ends nothing**, driven through a caller who
    /// is seated but is not the creator: a stream on a live table must not
    /// close because somebody who could not delete it tried.
    #[test]
    fn a_refused_delete_ends_nothing() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        games.create(&two_people("dr", CORPUS_SEED), &red).expect("a table");
        let mut rx = signals.subscribe("dr");
        rx.borrow_and_update();

        games.delete("dr", &blue).expect_err("delete is the creator's");

        assert!(!rx.has_changed().expect("the table channel is open"), "no unlink, no end");
        assert!(!rx.borrow().end);
    }

    /// A [`MemStore`] that PARKS the table write which flips a table to
    /// `playing`, and nothing else.
    ///
    /// Keyed on the document's own `status` rather than on a write COUNT,
    /// because the write to park is the one write of the finalize sequence
    /// whose bump is under test: the keep's own table write and the record
    /// write both go past it untouched, and a counter would have to be
    /// re-derived every time that sequence gained a write. Parsed rather than
    /// string-matched, so a table whose free-text NAME spells the field does
    /// not park a write nobody was asking about.
    struct FlipGate {
        inner: MemStore,
        at_flip: Mutex<std::sync::mpsc::Sender<()>>,
        go_flip: Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl Store for FlipGate {
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
            let flipping = doc == Doc::Table
                && serde_json::from_str::<TableDoc>(json)
                    .is_ok_and(|d| d.status == TableStatus::Playing);
            if flipping {
                let _ = self.at_flip.lock().expect("the gate").send(());
                let _ = self.go_flip.lock().expect("the gate").recv();
            }
            self.inner.write_if_version(doc, id, json, expected)
        }
        fn delete(&self, id: &str) -> Result<(), String> {
            self.inner.delete(id)
        }
    }

    /// **A BUMP PRECEDES NO WRITE** — the property every cell above assumes
    /// and none of them can see, because in an unparked run the write and the
    /// bump are one instant apart.
    ///
    /// A bump means "the state moved, go and read it". Sent BEFORE its write,
    /// it sends every subscriber to fetch the state it already has, and the
    /// write that follows is then announced to nobody — a table that a client
    /// stops hearing about until something else happens at it. So the flip's
    /// table write is parked inside the store and the channel is read while it
    /// sits there.
    ///
    /// **The comparison is a SEQUENCE and not `has_changed`, and that is the
    /// whole design of this cell.** The keep that triggers the finalize has
    /// ALREADY bumped the table, correctly, on its own write — so a receiver
    /// asked "have you changed?" mid-park says yes and says nothing about the
    /// finalize. The assertion is that the seq is EXACTLY the keep's while the
    /// flip is parked, and higher once it lands.
    ///
    /// A bump moved in front of `flip_to_playing`'s write, or up to the top of
    /// `finalize`, fires while the gate holds and the mid-park equality fails.
    #[test]
    fn a_bump_precedes_no_write() {
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let (at_flip_tx, at_flip_rx) = channel();
        let (go_flip_tx, go_flip_rx) = channel();
        let store = Arc::new(FlipGate {
            inner: MemStore::default(),
            at_flip: Mutex::new(at_flip_tx),
            go_flip: Mutex::new(go_flip_rx),
        });
        let (games, signals) = watched_games(Arc::clone(&store) as Arc<dyn Store>);
        let games = Arc::new(games);
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        let id = games.create(&two_people("parked", CORPUS_SEED), &red).expect("a table").id;
        games.keep(&id, &red, PlayerId::new(0), 1, None).expect("red's keep");

        let mut rx = signals.subscribe(&id);
        let house = signals.subscribe_house();
        let before = rx.borrow_and_update().seq;

        let keeper = {
            let (games, id, blue) = (Arc::clone(&games), id.clone(), blue.clone());
            std::thread::spawn(move || games.keep(&id, &blue, PlayerId::new(1), 1, None))
        };
        at_flip_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the finalize reaches the flip with its table write pending");

        // The keep's OWN write has landed and is announced; the flip's has not
        // and is not.
        let parked = rx.borrow_and_update().seq;
        assert_eq!(parked, before + 1, "the keep's bump, and only the keep's");
        let (doc, _) = games.read_table(&id).expect("a readable store").expect("the table");
        assert_eq!(doc.status, TableStatus::Dealing, "the premise: the flip has NOT landed");
        assert_eq!(doc.keeps, vec![Some(1), Some(1)], "and the keep's write HAS");

        go_flip_tx.send(()).expect("the parked flip is still listening");
        keeper.join().expect("the keep thread").expect("the keep");

        assert!(rx.has_changed().expect("the table channel is open"), "the flip, once it lands");
        assert!(rx.borrow_and_update().seq > parked);
        assert!(house.has_changed().expect("the house channel is open"));
        let (doc, _) = games.read_table(&id).expect("a readable store").expect("the table");
        assert_eq!(doc.status, TableStatus::Playing, "the state the woken subscriber reads");
    }

    // ---------------------------------------------------------------------
    // M6.4 — the append (spec §2.4, §4, R-6.4.7, R-6.4.8, R-6.4.9)
    // ---------------------------------------------------------------------

    /// A table's chat page the way `api::chat_read` asks for one: the table
    /// read first, then passed in (M6.5c2 cycle D, row 130) — read by red, the
    /// seat every cell here creates its table as.
    fn page_of(games: &Games, id: &str, after: u64) -> Result<(Vec<MessageView>, u64), GameError> {
        let (table, _) = games.read_table(id)?.expect("a table to page");
        games.chat_page(id, &table, &who("red@t.test"), after).map(|(page, counts)| (page, counts.last))
    }

    fn red_seat() -> Author {
        Author::Seat { seat: 0, user: "red@t.test".into(), name: Some("Red".into()) }
    }

    /// **A stored author is served in the form the shape rule reads it in**
    /// (M6.5c2 cycle D, row 130). The rule parses the address (trimmed and
    /// lowercased, `User::parse`) and the name (trimmed, `check_name`) and
    /// threw both results away, so a document written by hand or brought back
    /// by a restore served `" RED@T.test "` beside a seat whose key is
    /// `red@t.test` — two people, to a client comparing them. The address no
    /// longer crosses (R-6.5c2.35), and the normal form is what makes the
    /// reader's `you` answerable: the padded, upper-cased address is red's.
    #[test]
    fn a_stored_author_is_served_in_its_normal_form() {
        let games = games();
        let red = who("red@t.test");
        let id = games.create(&two_people("normal", CORPUS_SEED), &red).expect("a table").id;
        let message = |author: Author| Message { seq: 1, created_at: 1, author, text: "hi".into(), deleted: false };
        let stored = ChatDoc {
            schema: crate::chat::CHAT_SCHEMA,
            id: id.clone(),
            last: 1,
            rev: 1,
            messages: vec![message(Author::Seat { seat: 0, user: " RED@T.test ".into(), name: Some("  Red ".into()) })],
        };
        let json = serde_json::to_string(&stored).expect("a chat document");
        games.store().write_if_version(Doc::Chat, &id, &json, None).expect("stored");
        let (page, _) = page_of(&games, &id, 0).expect("served");
        assert_eq!(
            page.iter().map(|m| m.author.clone()).collect::<Vec<_>>(),
            [crate::chat::AuthorView::Seat { seat: 0, name: Some("Red".into()), you: true }],
        );
    }

    /// **An append lands, numbers from 1, and bumps the TABLE after its write**
    /// (spec §2.4).
    ///
    /// And not the house: a message at one table is not a change to anybody's
    /// lobby listing, so a house bump here would send every lobby in the room
    /// to refetch two lists because two people are talking (R-6.4.7).
    ///
    /// The `after = last` tail is the cursor rule the stream depends on: a
    /// client that has read everything asks again and is told nothing, rather
    /// than being handed the last message a second time.
    #[test]
    fn a_chat_append_writes_then_bumps_the_table_and_not_the_house() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        let red = who("red@t.test");
        let id = games.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        let mut rx = signals.subscribe(&id);
        let house = signals.subscribe_house();
        rx.borrow_and_update();
        let m = games.chat_append(&id, red_seat(), "  hello ").expect("an append");
        assert_eq!((m.seq, m.text.as_str()), (1, "hello"));
        assert!(rx.has_changed().unwrap(), "the table woke");
        assert!(!house.has_changed().unwrap(), "the house did not (R-6.4.7)");
        let (page, last) = page_of(&games, &id, 0).expect("a page");
        assert_eq!((page.len(), last, page[0].seq), (1, 1, 1));
        assert_eq!(games.chat_last(&id), 1);
        assert_eq!(
            games.chat_counts(&id),
            ChatCounts { last: 1, rev: 1 },
            "an append moves the number AND the revision (R-D4)",
        );
        let (delta, _) = page_of(&games, &id, 1).expect("a page");
        assert!(delta.is_empty(), "after=last is empty");
    }

    /// **The write key is the one the read used, and NEVER the stored `id`.**
    ///
    /// `id` is a field read out of a file, which in this lane makes it an INPUT
    /// — and it is the one field [`crate::chat::check_shape`] does not bound,
    /// because bounding it there would need the expected key the rule is not
    /// given. Keyed off the document instead, an append at A that met a
    /// document claiming to be B read A's chat, wrote it to B's file, and
    /// carried A's version there as the compare-and-swap expectation: the
    /// document is planted here exactly as a hand-edit or a half-finished
    /// restore would leave it.
    ///
    /// The planted `last` of 4 is what makes the landing legible — the message
    /// numbers 5, so the page it lands on is unmistakably the planted document
    /// advanced and not a fresh one.
    ///
    /// **B gets a document of its own before the append, with the SAME BYTES,
    /// and that is what makes the second half a reading** (final review, minor
    /// 16). An earlier draft asserted B's chat was "still absent"; nothing had
    /// ever written one, so a `None` there was equally consistent with a store
    /// nobody had touched, and the assertion held whatever the append did.
    ///
    /// The bytes have to be identical because [`crate::store::version_of`] is a
    /// digest of them: planted DIFFERENTLY, B's version would not match the one
    /// A's read handed the writer and the misdirected compare-and-swap would be
    /// refused — the cell would go red one line earlier, at the append, and
    /// still say nothing about B. Identical, the swap SUCCEEDS, A's message
    /// lands in B's file, and this is the assertion that sees it.
    #[test]
    fn an_append_writes_at_the_key_it_read_and_not_at_the_documents_own_id() {
        let store = Arc::new(MemStore::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let red = who("red@t.test");
        let a = games.create(&two_people("alpha", CORPUS_SEED), &red).expect("a table").id;
        let b = games.create(&two_people("beta", CORPUS_SEED), &red).expect("a table").id;
        let lying = serde_json::json!({ "schema": 1, "id": b, "last": 4, "messages": [] });
        store
            .write_if_version(Doc::Chat, &a, &lying.to_string(), None)
            .expect("the planted document");
        // B's own chat, planted with the SAME BYTES so that a write misdirected
        // there would be ACCEPTED rather than refused on its version — see the
        // doc above.
        let bs_version = store
            .write_if_version(Doc::Chat, &b, &lying.to_string(), None)
            .expect("B's own document");

        games.chat_append(&a, red_seat(), "here").expect("an append at A");

        // **B FIRST**, so the cell reports the misdirection where it happened
        // rather than as a message missing from A — which is the same failure
        // read from the far end and one line later.
        let (bs_json, bs_now) = store.read(Doc::Chat, &b).expect("a store").expect("B's chat");
        assert_eq!(
            (bs_json.as_str(), bs_now),
            (lying.to_string().as_str(), bs_version),
            "B's chat is untouched — the same bytes at the same version",
        );
        let (page, last) = page_of(&games, &a, 0).expect("a page");
        assert_eq!(
            (page.len(), last, page[0].seq),
            (1, 5, 5),
            "and it landed at A, numbered above the planted last",
        );
        let (json, _) = store.read(Doc::Chat, &a).expect("a store").expect("A's chat");
        let written: serde_json::Value = serde_json::from_str(&json).expect("the document");
        assert_eq!(written["id"], a, "and the document this server wrote names its own file");
    }

    /// **The text rule is the appender's too** (R-plan-3).
    ///
    /// The route applies it to answer 400 with the reason; this end applies it
    /// because it is the one writer, and a rule enforced only at the door is a
    /// rule the next caller of this method does not have.
    #[test]
    fn an_append_refuses_an_empty_an_over_cap_and_a_control_text() {
        let games = games();
        let red = who("red@t.test");
        let id = games.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        for bad in ["   ", "a\tb"] {
            assert!(
                matches!(games.chat_append(&id, red_seat(), bad), Err(GameError::BadRequest(_))),
                "{bad:?}",
            );
        }
        let long = "x".repeat(501);
        assert!(matches!(
            games.chat_append(&id, red_seat(), &long),
            Err(GameError::BadRequest(_))
        ));
        assert_eq!(games.chat_last(&id), 0, "nothing landed");
    }

    /// **The keep trims the OLDEST and `last` keeps counting** (R-6.4.8,
    /// R-6.4.9).
    ///
    /// A number is never reused, so a client holding a cursor taken before the
    /// trim still means by it what it meant: it asks for everything above 5 and
    /// is told nothing, rather than being served three messages it has already
    /// seen under numbers it has already used.
    #[test]
    fn the_keep_drops_the_oldest_and_never_reuses_a_number() {
        let games = Games::with_limits(
            Arc::new(Rules::standard()),
            Arc::new(MemStore::default()),
            ChatLimits { keep: Some(3), text: 500 },
        );
        let red = who("red@t.test");
        let id = games.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        for i in 1..=5 {
            games.chat_append(&id, red_seat(), &format!("m{i}")).expect("an append");
        }
        let (page, last) = page_of(&games, &id, 0).expect("a page");
        assert_eq!(page.iter().map(|m| m.seq).collect::<Vec<_>>(), vec![3, 4, 5]);
        assert_eq!(last, 5);
        assert_eq!(games.chat_last(&id), 5);
    }

    /// **`all` keeps everything** (R-6.4.8) — the launch variable's other end,
    /// so the trim is a configured number and not a constant wearing one.
    ///
    /// **N is the DEFAULT keep plus one, and that is its only job** (M6.5c2
    /// cycle D, row 130): an `all` that fell back to the default would trim at
    /// the default, so one message past it is the first count that can tell
    /// the two apart. Derived rather than written (it was a bare 600), so the
    /// cell follows the default if the default moves.
    #[test]
    fn keep_all_trims_nothing() {
        let games = Games::with_limits(
            Arc::new(Rules::standard()),
            Arc::new(MemStore::default()),
            ChatLimits { keep: None, text: 500 },
        );
        let red = who("red@t.test");
        let id = games.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        let n = ChatLimits::default().keep.expect("the default keeps a number") + 1;
        for i in 1..=n {
            games.chat_append(&id, red_seat(), &format!("m{i}")).expect("an append");
        }
        assert_eq!(page_of(&games, &id, 0).expect("a page").0.len(), n);
    }

    /// **A document longer than the keep is read whole and trimmed on the next
    /// append, never refused** (R-6.4.8).
    ///
    /// Which is what makes lowering the number on a running volume safe: the
    /// operator restarts with a smaller keep and the tables that were over it
    /// keep serving, rather than every one of them answering 500 until somebody
    /// edits a file.
    #[test]
    fn an_over_keep_document_is_served_and_trimmed_on_the_next_append() {
        let store = Arc::new(MemStore::default());
        let wide = Games::with_limits(
            Arc::new(Rules::standard()),
            Arc::clone(&store) as Arc<dyn Store>,
            ChatLimits { keep: None, text: 500 },
        );
        let red = who("red@t.test");
        let id = wide.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        for i in 1..=10 {
            wide.chat_append(&id, red_seat(), &format!("m{i}")).expect("an append");
        }
        let narrow = Games::with_limits(
            Arc::new(Rules::standard()),
            store as Arc<dyn Store>,
            ChatLimits { keep: Some(4), text: 500 },
        );
        assert_eq!(page_of(&narrow, &id, 0).expect("served, not refused").0.len(), 10);
        narrow.chat_append(&id, red_seat(), "m11").expect("an append");
        let (page, _) = page_of(&narrow, &id, 0).expect("a page");
        assert_eq!(page.iter().map(|m| m.seq).collect::<Vec<_>>(), vec![8, 9, 10, 11]);
    }

    /// **The count is seeded from the document on a cold miss** (R-plan-5).
    ///
    /// The second `Games` is this process after a sleep: the stream's per-bump
    /// closure reads a count it has never written, and a count that started at
    /// 0 there would tell every reconnecting client that every message it holds
    /// is newer than the table's.
    #[test]
    fn chat_last_is_seeded_from_the_document_when_this_process_has_not_seen_it() {
        let store = Arc::new(MemStore::default());
        let first = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let red = who("red@t.test");
        let id = first.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        first.chat_append(&id, red_seat(), "one").expect("an append");
        first.chat_append(&id, red_seat(), "two").expect("an append");
        let second = Games::new(Arc::new(Rules::standard()), store as Arc<dyn Store>);
        assert_eq!(second.chat_last(&id), 2, "a fresh process reads the document once");
        assert_eq!(second.chat_last("nothing-here"), 0, "no document, no messages");
    }

    /// **The house append bumps the house's CHAT counter and no table**, and a
    /// refused one bumps nothing at all.
    ///
    /// The seat author is the refusal worth having here: the house has no
    /// seats, so a message claiming one would index `colors[seat]` at a table
    /// that does not exist. It is a `Server` refusal and not a `BadRequest`
    /// because no route can produce it — the house route builds a `Watcher` —
    /// so reaching it means this process built a message it should not have.
    #[test]
    fn a_house_append_bumps_the_house_chat_counter_only() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        let mut house = signals.subscribe_house();
        house.borrow_and_update();
        let m = games
            .house_append(Author::Watcher { user: "g@t.test".into(), name: Some("G".into()) }, "hi all")
            .expect("an append");
        assert_eq!(m.seq, 1);
        let h = *house.borrow_and_update();
        assert_eq!((h.seq, h.chat), (0, 1));
        assert!(
            matches!(
                games.house_append(
                    Author::Seat { seat: 0, user: "r@t.test".into(), name: None },
                    "x",
                ),
                Err(GameError::Server(_)),
            ),
            "a seat author is refused at the house by the shape rule",
        );
        assert!(!house.has_changed().unwrap(), "a refused append bumps nothing");
        assert_eq!(house.borrow().chat, 1, "and leaves the counter where it was");
        let (page, counts) = games.house_page(&who("g@t.test"), 0).expect("a page");
        assert_eq!((page.len(), counts.last), (1, 1), "and wrote nothing");
    }

    /// **A delete removes the chat document and forgets the count.**
    ///
    /// The document goes with the record and the table inside the store's own
    /// `delete`; the COUNT is this process's cache and has to be dropped here,
    /// or a name reopened later starts its numbering above messages nobody can
    /// read.
    ///
    /// The recreate tail is over-determined on purpose and says so: both
    /// `delete` and `create` drop the entry, so the tail holds if either one
    /// does. It is here for `create`'s clear, which is the one that still
    /// stands when a document was left behind by some other route.
    #[test]
    fn a_delete_removes_the_chat_and_its_count() {
        let games = games();
        let red = who("red@t.test");
        let id = games.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        games.chat_append(&id, red_seat(), "bye").expect("an append");
        games.delete(&id, &red).expect("a delete");
        assert!(games.store().read(Doc::Chat, &id).expect("a store").is_none());
        assert_eq!(games.chat_last(&id), 0);

        games.create(&two_people(&id, CORPUS_SEED), &red).expect("the name is free again");
        assert_eq!(games.chat_last(&id), 0, "a reopened name starts at nothing");
        assert!(page_of(&games, &id, 0).expect("a page").0.is_empty());
    }

    /// A [`MemStore`] that PARKS every chat write and nothing else.
    ///
    /// Keyed on the document KIND, which is all the discrimination these two
    /// cells need: the create that precedes the parked append writes a table,
    /// and no other chat write is in flight.
    struct ChatGate {
        inner: MemStore,
        at_write: Mutex<std::sync::mpsc::Sender<()>>,
        go: Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl Store for ChatGate {
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
            if doc == Doc::Chat || doc == Doc::House {
                let _ = self.at_write.lock().expect("the gate").send(());
                let _ = self.go.lock().expect("the gate").recv();
            }
            self.inner.write_if_version(doc, id, json, expected)
        }
        fn delete(&self, id: &str) -> Result<(), String> {
            self.inner.delete(id)
        }
    }

    /// **A chat bump precedes no write** (spec §2.4): the append's write is
    /// parked inside the store and the receiver is read QUIET while it sits
    /// there.
    ///
    /// The same property [`a_bump_precedes_no_write`] holds for the flip, and
    /// unprovable the same way without a gate — in an unparked run the write
    /// and the bump are one instant apart. A bump sent first sends every
    /// browser at the table to fetch a message that is not there yet, and the
    /// message that then lands is announced to nobody.
    ///
    /// `has_changed` suffices here where the flip's cell needed a sequence
    /// comparison: nothing else has bumped this table since the subscribe, so
    /// a wake mid-park can only be the append's.
    #[test]
    fn a_chat_bump_precedes_no_write() {
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let (at_tx, at_rx) = channel();
        let (go_tx, go_rx) = channel();
        let store = Arc::new(ChatGate {
            inner: MemStore::default(),
            at_write: Mutex::new(at_tx),
            go: Mutex::new(go_rx),
        });
        let (games, signals) = watched_games(Arc::clone(&store) as Arc<dyn Store>);
        let games = Arc::new(games);
        let red = who("red@t.test");
        let id = games.create(&two_people("parked", CORPUS_SEED), &red).expect("a table").id;
        let mut rx = signals.subscribe(&id);
        rx.borrow_and_update();

        let writer = {
            let (games, id) = (Arc::clone(&games), id.clone());
            std::thread::spawn(move || games.chat_append(&id, red_seat(), "parked"))
        };
        at_rx.recv_timeout(Duration::from_secs(10)).expect("the append reaches its write");
        assert!(!rx.has_changed().unwrap(), "the write is parked and the receiver is QUIET");

        go_tx.send(()).expect("the parked write is listening");
        writer.join().expect("the thread").expect("the append");
        assert!(rx.has_changed().unwrap(), "and wakes once it lands");
    }

    // ---------------------------------------------------------------------
    // M6.5c2 cycle D — the delete (R-D4, Review Focus 2)
    // ---------------------------------------------------------------------

    fn blue_seat() -> Author {
        Author::Seat { seat: 1, user: "blue@t.test".into(), name: Some("Blue".into()) }
    }

    /// The stored chat document's version, so a cell can say "nothing was
    /// written" by what the store holds rather than by a page that would read
    /// the same either way.
    fn chat_version(games: &Games, id: &str) -> Version {
        games.store().read(Doc::Chat, id).expect("a store").expect("a chat document").1
    }

    /// **A delete by the author, and by the table's creator, writes the
    /// tombstone and then bumps the TABLE** (R-D4) — and not the house, for
    /// the append's reason (R-6.4.7). The count moves not at all and the
    /// revision once per delete: the catchers for "a delete bumps `last`" and
    /// "a delete leaves `rev` unchanged" through the appender's own cache.
    #[test]
    fn a_chat_delete_by_the_author_or_the_creator_writes_then_bumps_the_table() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        let red = who("red@t.test");
        let id = games.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        games.chat_append(&id, red_seat(), "mine").expect("red's");
        games.chat_append(&id, blue_seat(), "theirs").expect("blue's");
        assert_eq!(games.chat_counts(&id), ChatCounts { last: 2, rev: 2 }, "premise");
        let mut rx = signals.subscribe(&id);
        let house = signals.subscribe_house();
        rx.borrow_and_update();

        games.delete_chat(&id, 1, &red).expect("the author deletes her own");
        assert!(rx.has_changed().unwrap(), "the table woke");
        assert!(!house.has_changed().unwrap(), "the house did not");
        assert_eq!(games.chat_counts(&id), ChatCounts { last: 2, rev: 3 });
        rx.borrow_and_update();
        games.delete_chat(&id, 2, &red).expect("the creator deletes blue's");
        assert!(rx.has_changed().unwrap());
        assert_eq!(games.chat_counts(&id), ChatCounts { last: 2, rev: 4 });

        let (page, last) = page_of(&games, &id, 0).expect("a page");
        assert_eq!(last, 2, "the cursor holds");
        assert_eq!(page.iter().map(|m| (m.seq, m.deleted, m.text.as_str())).collect::<Vec<_>>(), [(1, true, ""), (2, true, "")]);
        forget_chat_count(&games, &id);
        assert_eq!(games.chat_counts(&id), ChatCounts { last: 2, rev: 4 }, "and the DOCUMENT says so, not just the cache");
    }

    /// **A refused delete and an idempotent one write nothing and announce
    /// nothing** (R-D4, Review Focus 2) — blue holds a seat and did not create
    /// the table, so red's message is not blue's to delete; a number nobody was
    /// given is not found; and red's second delete of her own message changes
    /// nothing, so it signals nothing either.
    #[test]
    fn a_refused_or_repeated_chat_delete_writes_nothing_and_bumps_nothing() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        let red = who("red@t.test");
        let blue = who("blue@t.test");
        let id = games.create(&two_people("guarded", CORPUS_SEED), &red).expect("a table").id;
        games.chat_append(&id, red_seat(), "mine").expect("red's");
        let mut rx = signals.subscribe(&id);
        rx.borrow_and_update();
        let version = chat_version(&games, &id);

        match games.delete_chat(&id, 1, &blue) {
            Err(GameError::Forbidden(m)) => {
                assert_eq!(m, "only the message's author or the table's creator may delete it")
            }
            other => panic!("a seat that neither wrote it nor created the table: {other:?}"),
        }
        match games.delete_chat(&id, 9, &red) {
            Err(GameError::NotFound(m)) => assert_eq!(m, "no message 9 in this chat"),
            other => panic!("a number nobody was given: {other:?}"),
        }
        assert_eq!(chat_version(&games, &id), version, "neither refusal wrote");
        assert!(!rx.has_changed().unwrap(), "and neither bumped");

        games.delete_chat(&id, 1, &red).expect("the author");
        rx.borrow_and_update();
        let version = chat_version(&games, &id);
        let counts = games.chat_counts(&id);
        games.delete_chat(&id, 1, &red).expect("again, idempotent");
        assert_eq!(chat_version(&games, &id), version, "the second wrote nothing");
        assert_eq!(games.chat_counts(&id), counts, "and moved nothing");
        assert!(!rx.has_changed().unwrap(), "and announced nothing");
    }

    /// **A delete's bump precedes no write** — [`a_chat_bump_precedes_no_write`]
    /// for the tombstone (R-D4): the delete's write is parked inside the store
    /// and the receiver read QUIET while it sits there. A bump ahead of it
    /// sends every browser at the table to re-read a page that still shows the
    /// message, and the delete that then lands is announced to nobody.
    #[test]
    fn a_chat_delete_bump_precedes_no_write() {
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let (at_tx, at_rx) = channel();
        let (go_tx, go_rx) = channel();
        let store = Arc::new(ChatGate {
            inner: MemStore::default(),
            at_write: Mutex::new(at_tx),
            go: Mutex::new(go_rx),
        });
        let (games, signals) = watched_games(Arc::clone(&store) as Arc<dyn Store>);
        let games = Arc::new(games);
        let red = who("red@t.test");
        let id = games.create(&two_people("parked", CORPUS_SEED), &red).expect("a table").id;
        // The append's own write parks too: let it through.
        let appender = {
            let (games, id) = (Arc::clone(&games), id.clone());
            std::thread::spawn(move || games.chat_append(&id, red_seat(), "soon gone"))
        };
        at_rx.recv_timeout(Duration::from_secs(10)).expect("the append reaches its write");
        go_tx.send(()).expect("the parked append is listening");
        appender.join().expect("the thread").expect("the append");

        let mut rx = signals.subscribe(&id);
        rx.borrow_and_update();
        let deleter = {
            let (games, id, red) = (Arc::clone(&games), id.clone(), red.clone());
            std::thread::spawn(move || games.delete_chat(&id, 1, &red))
        };
        at_rx.recv_timeout(Duration::from_secs(10)).expect("the delete reaches its write");
        assert!(!rx.has_changed().unwrap(), "the tombstone is parked and the receiver is QUIET");
        go_tx.send(()).expect("the parked write is listening");
        deleter.join().expect("the thread").expect("the delete");
        assert!(rx.has_changed().unwrap(), "and wakes once it lands");
    }

    /// **The house: the author alone, and a delete moves the REVISION and
    /// nothing else** (R-D4). The house has no creator, so blue may not
    /// delete red's post whoever blue is; red may, and the house channel's
    /// `chat` does not move — a delete adds no message — while `rev` does, so
    /// every lobby wakes. The same ordering holds as the append's: under the
    /// house's lock, the write, then the bump.
    #[test]
    fn a_house_delete_is_the_authors_alone_and_moves_the_revision() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        let red = who("red@t.test");
        let blue = who("blue@t.test");
        let author = Author::Watcher { user: "red@t.test".into(), name: Some("Red".into()) };
        let m = games.house_append(author, "anyone?").expect("a house post");
        let mut house = signals.subscribe_house();
        let before = *house.borrow_and_update();
        assert_eq!((before.chat, before.rev), (1, 1), "premise: the post moved both");

        match games.delete_house_chat(m.seq, &blue) {
            Err(GameError::Forbidden(said)) => {
                assert_eq!(said, "only the message's author or the table's creator may delete it")
            }
            other => panic!("the house has no creator, so only the author: {other:?}"),
        }
        assert!(!house.has_changed().unwrap(), "a refusal wakes no lobby");

        games.delete_house_chat(m.seq, &red).expect("the author");
        assert!(house.has_changed().unwrap(), "every lobby wakes");
        let h = *house.borrow_and_update();
        assert_eq!((h.seq, h.chat, h.rev), (before.seq, before.chat, before.rev + 1), "the revision and nothing else");
        let (page, counts) = games.house_page(&blue, 0).expect("the house");
        assert_eq!(
            counts,
            ChatCounts { last: m.seq, rev: 2 },
            "the cursor holds, and the page's revision counts one post and one delete",
        );
        assert!(page[0].deleted && page[0].text.is_empty(), "a tombstone in red's row: {page:?}");
    }

    /// Drop this process's remembered count for one table, so the next
    /// `chat_last` takes the MISS path a fresh process would take.
    ///
    /// The cache is what the two cells below are about, and both need it empty
    /// over a table that has already been written to — which no public call
    /// arranges (`create` and `delete` clear it, and both also clear the
    /// document).
    fn forget_chat_count(games: &Games, id: &str) {
        games.chat_counts.lock().expect("games lock").remove(id);
    }

    /// A [`MemStore`] that PARKS the FIRST `Doc::Chat` read and nothing else,
    /// once armed.
    ///
    /// Once, and only the first, because the racing append reads the same
    /// document on its way to writing it: a gate that parked every chat read
    /// would park the append too and there would be no race to watch.
    struct ChatReadGate {
        inner: MemStore,
        armed: std::sync::atomic::AtomicBool,
        at_read: Mutex<std::sync::mpsc::Sender<()>>,
        go: Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl Store for ChatReadGate {
        fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
            self.inner.list(doc)
        }
        fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
            let answer = self.inner.read(doc, id);
            // AFTER the answer is computed, which is the whole of the window:
            // the caller now holds a number, and the append is free to make it
            // stale before the caller writes it down.
            if doc == Doc::Chat && self.armed.swap(false, std::sync::atomic::Ordering::SeqCst) {
                let _ = self.at_read.lock().expect("the gate").send(());
                let _ = self.go.lock().expect("the gate").recv();
            }
            answer
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

    /// **A cold `chat_last` seeds the cache and never moves it BACKWARDS**
    /// (final review, minor 6).
    ///
    /// The miss path reads the document under no lock of its own — deliberately,
    /// because it is called from a stream's per-bump closure and must not queue
    /// behind a table — so an append can land between that read and the moment
    /// the number is written down. With a plain `insert` the older number wins
    /// and every count-driven browser at the table under-reports until the NEXT
    /// message: holding N-1, it does not fetch message N.
    ///
    /// `or_insert` is the whole of the fix and it is safe in the one direction
    /// that matters: the entry only ever arrives from an append, which knows a
    /// number this read cannot have missed.
    ///
    /// The gate is what makes the window real rather than argued — in an
    /// unparked run the read and the insert are one instant apart.
    #[test]
    fn a_cold_count_read_never_overwrites_a_newer_one_from_an_append() {
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let (at_tx, at_rx) = channel();
        let (go_tx, go_rx) = channel();
        let store = Arc::new(ChatReadGate {
            inner: MemStore::default(),
            armed: std::sync::atomic::AtomicBool::new(false),
            at_read: Mutex::new(at_tx),
            go: Mutex::new(go_rx),
        });
        let games =
            Arc::new(Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>));
        let red = who("red@t.test");
        let id = games.create(&two_people("cold", CORPUS_SEED), &red).expect("a table").id;

        // A stream's closure asks for the count of a table nobody has spoken
        // at, and is parked holding "0".
        store.armed.store(true, std::sync::atomic::Ordering::SeqCst);
        let reader = {
            let (games, id) = (Arc::clone(&games), id.clone());
            std::thread::spawn(move || games.chat_last(&id))
        };
        at_rx.recv_timeout(Duration::from_secs(10)).expect("the cold read answers");

        // And a message lands while it sits there: the append writes the
        // document and records 1.
        games.chat_append(&id, red_seat(), "first").expect("an append");
        assert_eq!(games.chat_last(&id), 1, "the append's own number is what is cached");

        go_tx.send(()).expect("the parked read is listening");
        let seen = reader.join().expect("the thread");
        assert_eq!(seen, 1, "the parked read takes the newer number rather than its own stale 0");
        assert_eq!(
            games.chat_last(&id),
            1,
            "and the cache was not moved BACKWARDS, which would leave every browser \
             at this table one message behind until the next one",
        );
    }

    /// **A store that cannot be read answers 0 and CACHES NOTHING** (spec §9.1;
    /// `chat_last`'s own doc rests on it).
    ///
    /// The one route with nowhere to put a refusal: it is called from a
    /// stream's per-bump closure, and a wrong count sends a client to a route
    /// that refuses properly. What it must not do is REMEMBER the wrong number
    /// — a cached 0 would outlive the disk trouble and the table would go quiet
    /// for the life of the process.
    ///
    /// The anti-vacuity is the second half: once the failure is spent, the same
    /// call reads the document and answers its real number.
    #[test]
    fn a_failed_count_read_answers_zero_and_caches_nothing() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        /// A [`MemStore`] whose next `n` `Doc::Chat` reads fail, and only those.
        struct FlakyChat {
            inner: MemStore,
            fail_chat_reads: AtomicUsize,
        }
        impl Store for FlakyChat {
            fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
                self.inner.list(doc)
            }
            fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
                if doc == Doc::Chat
                    && self
                        .fail_chat_reads
                        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                        .is_ok()
                {
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

        let store =
            Arc::new(FlakyChat { inner: MemStore::default(), fail_chat_reads: AtomicUsize::new(0) });
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let red = who("red@t.test");
        let id = games.create(&two_people("flaky", CORPUS_SEED), &red).expect("a table").id;
        games.chat_append(&id, red_seat(), "said").expect("an append");
        // The append cached 1; the cache is what a miss has to be arranged
        // around, so it is cleared exactly as a fresh process would find it.
        forget_chat_count(&games, &id);

        store.fail_chat_reads.store(1, Ordering::SeqCst);
        assert_eq!(games.chat_last(&id), 0, "a failed read answers 0");
        assert_eq!(
            store.fail_chat_reads.load(Ordering::SeqCst),
            0,
            "the armed failure never fired, so this cell watched the happy path twice",
        );
        assert_eq!(games.chat_last(&id), 1, "and cached nothing, so the next call gets it right");
    }

    /// **A chat document that will not PARSE is a `Server` failure on the route
    /// that serves it, and a 0 on the count** (spec §9.1).
    ///
    /// The two answers differ on purpose and both are `read_chat`'s and
    /// `chat_last`'s own doc sentences: a route that answered "no messages" for
    /// a file sitting right there would let the next append overwrite it and
    /// number from 1 again, while the count has nowhere to put a refusal and a
    /// wrong number sends the client to the route that refuses properly.
    #[test]
    fn a_chat_document_that_will_not_parse_refuses_the_page_and_reads_zero() {
        let games = games();
        let red = who("red@t.test");
        let id = games.create(&two_people("broken", CORPUS_SEED), &red).expect("a table").id;
        games
            .store()
            .write_if_version(Doc::Chat, &id, "{", None)
            .expect("the planted half-written document");
        forget_chat_count(&games, &id);

        let e = page_of(&games, &id, 0).expect_err("a page over a broken document");
        assert!(
            matches!(&e, GameError::Server(why) if why.contains("does not parse")),
            "{e:?}",
        );
        assert_eq!(games.chat_last(&id), 0, "and the count says 0 rather than inventing one");
    }

    /// The same for the house, whose appender is a different function with a
    /// different lock and its own bump — so the property has to be shown twice
    /// or it is shown for one of the two writers.
    #[test]
    fn a_house_chat_bump_precedes_no_write() {
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let (at_tx, at_rx) = channel();
        let (go_tx, go_rx) = channel();
        let store = Arc::new(ChatGate {
            inner: MemStore::default(),
            at_write: Mutex::new(at_tx),
            go: Mutex::new(go_rx),
        });
        let (games, signals) = watched_games(Arc::clone(&store) as Arc<dyn Store>);
        let games = Arc::new(games);
        let mut house = signals.subscribe_house();
        house.borrow_and_update();

        let writer = {
            let games = Arc::clone(&games);
            std::thread::spawn(move || {
                games.house_append(Author::Watcher { user: "g@t.test".into(), name: None }, "parked")
            })
        };
        at_rx.recv_timeout(Duration::from_secs(10)).expect("the append reaches its write");
        assert!(!house.has_changed().unwrap(), "quiet while parked");

        go_tx.send(()).expect("listening");
        writer.join().expect("the thread").expect("the append");
        assert_eq!(house.borrow_and_update().chat, 1);
    }

    // ---------------------------------------------------------------------
    // M6.3 — the watcher's door (§3.4, R-3.5, R-3.9, R-3.13)
    // ---------------------------------------------------------------------

    /// A two-person request with a password on it.
    fn two_people_locked(id: &str, password: &str) -> NewGame {
        let mut r = two_people(id, CORPUS_SEED);
        r.password = Some(password.to_string());
        r
    }

    /// A LOCKED table played into `playing`, and the games that hold it.
    fn a_locked_game(id: &str) -> Games {
        let games = games();
        playing(&games, &two_people_locked(id, "sesame"), &["red@t.test", "blue@t.test"]);
        games
    }

    /// **The seven answers of `POST …/{id}/unlock`** (§3.4), in one cell,
    /// because what is being pinned is the LADDER: each arm on its own is
    /// satisfied by an implementation that answers it first and the rest
    /// wrongly.
    ///
    /// **A single-defect probe pins nothing about ORDER**, so every adjacent
    /// link carries a TWO-defect one — a request that would trip both rungs,
    /// asserted against the one that must speak first:
    ///
    /// - (status, password): an open table AND the wrong password answers
    ///   `Open`. There is nothing to watch yet, whatever was typed.
    /// - (can_watch, password): a SEAT HOLDER sending the empty string is a
    ///   200 no-op — asserted in the write-free cell below, which is where the
    ///   `CountingStore` that gives it its second half lives.
    /// - (password, cap): a full watcher list AND the wrong password answers
    ///   `Forbidden`. A caller who cannot open the door is never told how
    ///   crowded the room is.
    ///
    /// The two status arms are typed (`Open`, `Dealing`) rather than sentences,
    /// so the client branches on the status exactly as it does everywhere else.
    #[test]
    fn unlock_answers_a_missing_table_the_two_early_statuses_a_wrong_password_and_the_success() {
        let open = games();
        let dealing = games();
        let playing = a_locked_game("locked");
        let grey = who("grey@t.test");
        let red = who("red@t.test");

        assert!(
            matches!(playing.unlock("nosuch", &grey, "sesame"), Err(GameError::NotFound(_))),
            "a table that is not there",
        );

        // There is nothing to watch at either of the two early statuses, and a
        // joiner sends the password to `join` instead.
        let mut req = two_people_locked("stillopen", "sesame");
        req.seats[1] = SeatSpec { user: None, bot: false, open: true };
        open.create(&req, &red).expect("an open table");
        // TWO defects — still open AND the wrong password — and the STATUS is
        // what answers. A ladder that checked the password first would say
        // `Forbidden` here and every single-defect probe in this cell would
        // still pass.
        assert!(matches!(open.unlock("stillopen", &grey, "wrong"), Err(GameError::Open)));
        dealing.create(&two_people_locked("stilldealing", "sesame"), &red).expect("a table");
        assert!(matches!(dealing.unlock("stilldealing", &grey, "sesame"), Err(GameError::Dealing)));

        // The wrong password, and the RIGHT one.
        let refused = playing.unlock("locked", &grey, "open").expect_err("the wrong password");
        assert!(
            matches!(&refused, GameError::Forbidden(m) if m == "wrong password for this table"),
            "{refused:?}",
        );
        assert!(
            !playing.read_table("locked").unwrap().unwrap().0.unlocked.contains(&"grey@t.test".to_string()),
            "a refused unlock writes nobody onto the document",
        );

        let view = playing.unlock("locked", &grey, "sesame").expect("the password");
        assert!(view.can_watch, "the view this caller is handed says so");
        assert!(view.locked, "and the table is still locked to everyone else");
        let (doc, _) = playing.read_table("locked").unwrap().unwrap();
        assert_eq!(doc.unlocked, vec!["grey@t.test".to_string()], "the email is on the document");
        assert!(doc.can_watch(&grey));

        // **Padding is trimmed**, the same way `create` trimmed the password it
        // hashed — otherwise a password typed with a trailing space is a
        // password this server hashed one way and compares another.
        let green = who("green@t.test");
        assert!(playing.unlock("locked", &green, "  sesame ").expect("trimmed").can_watch);
    }

    /// **A plain name on the watcher list reads, and is one key however it is
    /// typed** (M6.5c2, spec §2.1 and §3; review H2). The key grammar is
    /// `check_shape`'s too, and it runs on every read — so a grammar the check
    /// did not share would take the whole table off the lobby (the arm
    /// `every_shape_arm_refuses_with_the_sentence_it_promises` pins). The name
    /// is written by the layer's own `unlock`, read back and listed.
    #[test]
    fn a_table_whose_watcher_list_holds_a_plain_name_reads() {
        let playing = a_locked_game("named");
        assert!(playing.unlock("named", &who("  Ben   Castle "), "sesame").expect("the password").can_watch);
        let (doc, _) = playing.read_table("named").expect("it reads").expect("it is there");
        assert_eq!(doc.unlocked, vec!["ben castle".to_string()], "the KEY, normalized once");
        assert!(doc.can_watch(&who("BEN CASTLE")), "the same key, typed otherwise");
        let lobby = playing.list_for(&who("red@t.test")).expect("the lobby");
        assert!(lobby.iter().any(|row| row.id == "named"), "listed");
    }

    /// A seat holder never needs the password, and an unlocked table never
    /// needs one either — and NEITHER writes.
    ///
    /// **The write COUNT, on both arms, and not the document's contents or its
    /// version.** `Version` is a hash of the bytes, so an idempotent re-write
    /// of the same document reports the same version and a version compare
    /// cannot see it; and reading `unlocked` back empty says the same nothing,
    /// since an unconditional `write_table` of the unchanged document satisfies
    /// it exactly. Only [`CountingStore`] can tell a no-op from a write that
    /// happened to change nothing, so BOTH arms are built over one — an earlier
    /// draft instrumented the seat holder's and left the no-lock arm reading
    /// its own document back.
    ///
    /// **The seat holder's arm is also this cycle's (can_watch, password)
    /// two-defect probe** (see the ladder cell above): a caller who may already
    /// watch AND presents nothing is answered by `can_watch`, not by the
    /// password check, which would refuse an empty string.
    #[test]
    fn an_unlock_with_nothing_to_unlock_is_a_two_hundred_that_writes_nothing() {
        let red = who("red@t.test");
        let grey = who("grey@t.test");

        let store = Arc::new(CountingStore::default());
        let locked = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        playing(&locked, &two_people_locked("locked", "sesame"), &["red@t.test", "blue@t.test"]);
        let before = store.writes();
        let mine = locked.unlock("locked", &red, "").expect("my own table");
        assert!(mine.can_watch, "a seat holder watches without presenting anything");
        assert_eq!(store.writes(), before, "and nothing was written");

        // The same claim, and the same instrument, at a table with NO lock.
        let counted = Arc::new(CountingStore::default());
        let free = Games::new(Arc::new(Rules::standard()), Arc::clone(&counted) as Arc<dyn Store>);
        playing(&free, &two_people("free", CORPUS_SEED), &["red@t.test", "blue@t.test"]);
        let before = counted.writes();
        let view = free.unlock("free", &grey, "").expect("no lock, nothing to unlock");
        assert!(view.can_watch);
        assert!(!view.locked);
        assert_eq!(counted.writes(), before, "an unlocked table is unlocked without a write");
        assert!(free.read_table("free").unwrap().unwrap().0.unlocked.is_empty());
    }

    /// **`unlock` bumps no signal** (controller ruling R-pre-3, cycle D row
    /// 39): the doc on [`Games::unlock`] said so and no cell read a receiver.
    /// Both channels are subscribed BEFORE the unlock, the unlock is one that
    /// WRITES (the roster gains the caller, so a write happened and a bump
    /// would have had something to announce), and neither has changed after.
    /// The catcher for battery row 26: a bump added after the unlock's write.
    #[test]
    fn an_unlock_that_writes_bumps_no_signal() {
        let (games, signals) = watched_games(Arc::new(MemStore::default()));
        playing(&games, &two_people_locked("locked", "sesame"), &["red@t.test", "blue@t.test"]);
        let table = signals.subscribe("locked");
        let house = signals.subscribe_house();
        let grey = who("grey@t.test");
        games.unlock("locked", &grey, "sesame").expect("the password");
        assert_eq!(
            games.read_table("locked").unwrap().unwrap().0.unlocked,
            vec!["grey@t.test".to_string()],
            "premise: the unlock wrote the roster",
        );
        assert!(!table.has_changed().expect("the table channel is open"), "no bump on the table's channel");
        assert!(!house.has_changed().expect("the house channel is open"), "and none on the house's");
    }

    /// **One unlock admits ONE caller, and a second unlock by the same caller
    /// adds no second entry.**
    ///
    /// The two halves are one cell because they are the same list read twice:
    /// a `push` with no membership test passes the two-watcher half and fails
    /// the dedup half, and a `push` guarded by "the list is empty" passes dedup
    /// and fails the other.
    #[test]
    fn one_unlock_admits_one_caller_and_unlocking_twice_leaves_one_entry() {
        let games = a_locked_game("locked");
        let grey = who("grey@t.test");
        let green = who("green@t.test");

        games.unlock("locked", &grey, "sesame").expect("the password");
        games.unlock("locked", &grey, "sesame").expect("again");
        let (doc, _) = games.read_table("locked").unwrap().unwrap();
        assert_eq!(doc.unlocked, vec!["grey@t.test".to_string()], "one entry, not two");
        assert!(!doc.can_watch(&green), "and the other watcher is still outside");
    }

    /// The 257th watcher is a typed 409 rather than a silent eviction of the
    /// oldest — which would re-lock somebody mid-game.
    ///
    /// **Both sides of the cap** (the S1 stack review's CB-F8): the list is
    /// filled to ONE under the cap by hand, the 256th unlock is ADMITTED, and
    /// only then is the 257th refused — so a roster that refuses at 256 fails
    /// the admit line, as one that admits a 257th fails the refusal; and the
    /// figure is pinned at its literal (finding D7, accepted).
    #[test]
    fn the_two_hundred_and_fifty_seventh_watcher_is_refused() {
        assert_eq!(UNLOCKED_CAP, 256, "a table remembers 256 watchers (finding D7)");
        let games = a_locked_game("locked");
        let (mut doc, version) = games.read_table("locked").unwrap().unwrap();
        doc.unlocked = (0..UNLOCKED_CAP - 1).map(|i| format!("w{i}@t.test")).collect();
        games.write_table(&doc, Some(version)).expect("a list one under the cap");

        games.unlock("locked", &who("ada@t.test"), "sesame").expect("the 256th watcher is admitted");
        assert_eq!(games.read_table("locked").unwrap().unwrap().0.unlocked.len(), UNLOCKED_CAP, "the list is full");

        let refused = games
            .unlock("locked", &who("grey@t.test"), "sesame")
            .expect_err("the list is full");
        assert!(
            matches!(&refused, GameError::Conflict(m)
                if m == "this table has as many spectators as it can remember"),
            "{refused:?}",
        );

        // TWO defects — a full list AND the wrong password — and the PASSWORD
        // is what answers. A caller who cannot open the door is never told how
        // crowded the room is, and a ladder that checked the cap first would
        // tell them; every single-defect probe in this file would still pass.
        let refused = games
            .unlock("locked", &who("green@t.test"), "wrong")
            .expect_err("the wrong password, at a full table");
        assert!(
            matches!(&refused, GameError::Forbidden(m) if m == "wrong password for this table"),
            "{refused:?}",
        );
        // And the document is exactly as full as it was — the refusal is not a
        // write that also failed.
        assert_eq!(games.read_table("locked").unwrap().unwrap().0.unlocked.len(), UNLOCKED_CAP);
    }

    /// **R-3.13: a playing table reads for a stranger, and says whether they
    /// may watch it.**
    ///
    /// The anti-vacuity half is the DEALING row: a widening written as "anybody
    /// may read any table" passes the first two reads and fails the third.
    #[test]
    fn a_playing_table_reads_for_a_stranger_and_a_dealing_one_does_not() {
        let free = games();
        let dealing = games();
        let locked = a_locked_game("locked");
        let grey = who("grey@t.test");
        let view = locked.table_for_anyone("locked", &grey).expect("a playing table reads");
        assert!(view.lock.is_some());
        assert!(!view.can_watch(&grey), "locked, and nothing has been presented");

        playing(&free, &two_people("free", CORPUS_SEED), &["red@t.test", "blue@t.test"]);
        assert!(
            free.table_for_anyone("free", &grey).expect("a playing table").can_watch(&grey),
            "no lock: a stranger may watch",
        );

        dealing.create(&two_people("dealt", CORPUS_SEED), &who("red@t.test")).expect("a table");
        assert!(
            matches!(dealing.table_for_anyone("dealt", &grey), Err(GameError::Forbidden(_))),
            "nobody watches a deal",
        );
    }

    /// **The `playing` scope lists games under way AND finished ones**, and
    /// neither of the other two statuses.
    ///
    /// "Over" is not a `TableStatus` — it is a `Playing` table whose mirror
    /// says so — so the finished row is made by writing the mirror rather than
    /// by playing a game out: what is under test is the LISTING's predicate,
    /// and a session would only make the cell slower.
    #[test]
    fn the_playing_scope_lists_games_under_way_and_finished_ones_and_nothing_else() {
        let games = games();
        let red = who("red@t.test");
        let grey = who("grey@t.test");

        let mut open = two_people("opent", CORPUS_SEED);
        open.seats[1] = SeatSpec { user: None, bot: false, open: true };
        games.create(&open, &red).expect("an open table");
        games.create(&two_people("dealt", CORPUS_SEED), &red).expect("a dealing table");
        playing(&games, &two_people("playt", CORPUS_SEED), &["red@t.test", "blue@t.test"]);
        playing(&games, &two_people("overt", CORPUS_SEED), &["red@t.test", "blue@t.test"]);
        let (mut doc, version) = games.read_table("overt").unwrap().unwrap();
        doc.turn = Some(TurnMirror { to_move: 0, over: true, winner: Some(1), has_events: false, bot: None });
        games.write_table(&doc, Some(version)).expect("a finished mirror");

        let (rows, next) = games
            .list(&grey, Scope::Playing, LIST_LIMIT_DEFAULT, None)
            .expect("the listing");
        let mut ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["overt", "playt"], "the two games, and neither of the others");
        assert_eq!(next, None);
        let over = rows.iter().find(|r| r.id == "overt").expect("the finished row");
        assert_eq!(over.over, Some(true));
        assert_eq!(over.winner, Some(PlayerId::new(1)), "the lobby says who won");
        assert!(rows.iter().all(|r| r.my_seats.is_empty()), "built for a caller with no seat");

        // And the scope parses off the wire by that name, with the refusal
        // naming all three.
        assert_eq!("playing".parse::<Scope>(), Ok(Scope::Playing));
        let why = "watching".parse::<Scope>().expect_err("not a scope");
        assert!(why.contains("mine, open and playing"), "{why}");
    }
}
