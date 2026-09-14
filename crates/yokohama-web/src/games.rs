//! Live games, and the rule that none of them may exist only in memory.
//!
//! A handle is a `Repl` behind a mutex. It is a cache, never the record: every
//! applied move is persisted immediately, and a handle that is absent is
//! rebuilt from the stored `GameRecord` through `Session::from_record`, which
//! runs every refusal the CLI's resume path runs. That is what lets this server
//! live in a container that sleeps.
//!
//! **M6.1 put a second document in front of that one.** A game now begins as a
//! TABLE ([`crate::table`]) with no record at all: `create` writes the table,
//! each seated player is dealt and keeps from its own view, and the LAST keep
//! finalizes — builds the session, writes the record, and flips the table to
//! `playing`. Until then there is nothing for `Session::from_record` to rebuild
//! and every route that needs a session answers 409. The two writes are not one
//! transaction, so both crash windows are repaired on the next read
//! ([`Games::settle`]) rather than left to a human.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
use yokohama_cli::repl::Repl;
use yokohama_cli::session::Session;
use yokohama_core::greedy::score_starting_order;
use yokohama_core::{
    new_game, Contestant, GameConfig, GameRecord, OrderCard, OrderCardId, PlayerColor, PlayerId,
    Rules, SeedSchedule, StartingOrderChoice,
};

use crate::client::HistoricalFrame;
use crate::identity::User;
use crate::store::{is_safe_id, Doc, GameId, Store, Version, WriteError};
use crate::table::{
    now_unix, DealView, SeatDoc, TableDoc, TableStatus, TableView, TurnMirror,
};

/// The shape version stamped on every table document this build writes.
const TABLE_SCHEMA: u32 = 1;

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

/// The two cards a seat was dealt, so a client can show them before choosing.
///
/// **A seat only ever sees its OWN deal (2026-09-06, R-C1).** On a bot seat
/// `cards` is EMPTY and there is no keep field on this struct at all — the
/// client draws two card backs from `is_bot` alone. The bot still keeps by the
/// same argmax ([`bot_keep`], applied at finalize): the record knows which card
/// it took, and the player does not.
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
///
/// **Retired by Task 10 of this cycle**, when the setup screen reads
/// `GET …/deal` instead and no unstarted table exists outside the store. It
/// survives here so the milestone's steps stay independently green (R-plan-4).
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
        }
    }
}

impl std::error::Error for GameError {}

/// The one sentence a dealing table answers every session route with. Beside it
/// on the wire rides `"status": "dealing"`, which is what the client branches
/// on — the words are for a human reading a `curl`.
pub const DEALING_SENTENCE: &str = "this table is still dealing";

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
                seats.push(SeatDoc::Human { user: user.as_str().to_string() });
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
    if !seats.iter().any(|s| matches!(s, SeatDoc::Human { user } if user == creator.as_str())) {
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

/// Is this a document every consumer may INDEX BY SEAT?
///
/// **The `players` bound is the load-bearing half, and the length checks alone
/// do not give it.** `TableDoc::seats_of` maps each seat's position through
/// `PlayerId::new`, which `types/ids.rs` documents as PANICKING at `id >= 4` —
/// so a document reading `players: 9` with nine seats and nine keeps satisfies
/// every length comparison and then panics on its fifth seat. Inside a handler
/// that is a dropped connection with no sentence in it; inside `Games::keep`,
/// which holds `table_lock` across the read, it POISONS that mutex, and every
/// later `create`, `keep` and repairing `settle` in the process fails with it.
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
    Ok(())
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

/// The two facts the list mirrors off a live session (R-plan-3).
fn mirror_of(repl: &Repl) -> TurnMirror {
    TurnMirror {
        to_move: repl.session.state().current_player.0,
        over: repl.session.is_over(),
    }
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
/// One function for both callers: [`preview`], which survives until this cycle's
/// last task, and [`Games::deal`], which is what replaces it. Two copies of this
/// body would be two set tables that can disagree about what a player sees
/// before choosing.
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

/// The deal a seed produces AND the table it sets, without starting anything.
///
/// **Retired by Task 10** — `GET …/deal` on a real table is what this becomes,
/// and that one serves a seat its OWN cards rather than every seat's at once.
pub fn preview(req: &NewGame, rules: &Rules, creator: &User) -> Result<Preview, PreviewError> {
    let (config, seats) = config_of(req, creator).map_err(PreviewError::Request)?;
    let seed = req.seed.map_or_else(draw_seed, |s| s.0);
    let pending = new_game(config.clone(), rules, seed)
        .map_err(|e| PreviewError::Server(format!("the setup did not deal: {e}")))?;
    let table = set_table(config, seed, &seats, &pending.deals, rules, first_human_seat(&seats))
        .map_err(PreviewError::Server)?;
    let deals = pending
        .deals
        .iter()
        .enumerate()
        .map(|(i, deal)| {
            let is_bot = matches!(seats.get(i), Some(SeatDoc::Bot));
            Deal {
                seat: PlayerId::new(i as u8),
                is_bot,
                // **A bot seat sends no cards (R-C1).** Not a filter on the
                // client's side of the wire but an absence on this one: the
                // payload cannot leak what it does not carry. The bot's own
                // keep is applied above, where the previewed table needs it,
                // and reaches the client only as a card already in a hand.
                cards: if is_bot { Vec::new() } else { cards_of(deal, rules) },
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
    /// Serializes every write to a TABLE document against every other one:
    /// `create`, each `keep`, and the self-repair in [`Games::settle`].
    ///
    /// `create` is check-then-write across a slow middle, so without this two
    /// requests both read "absent" and both went on to build, write and insert.
    /// A keep is read-modify-write over the same document, and the LAST keep
    /// finalizes — two of them racing would build two sessions and write two
    /// records for one table. Held across the whole of each, because any shorter
    /// hold leaves the same window somewhere else.
    ///
    /// A dedicated lock rather than holding `live`: `live` is taken by every
    /// `handle`, and blocking ordinary reads for the length of a game setup to
    /// serialize an operation a user performs by hand is the wrong trade.
    table_lock: Mutex<()>,
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
            table_lock: Mutex::new(()),
            deleted: Mutex::new(std::collections::HashSet::new()),
        }
    }

    pub fn store(&self) -> &Arc<dyn Store> {
        &self.store
    }

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
    fn read_table(&self, id: &str) -> Result<Option<(TableDoc, Version)>, GameError> {
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

    fn write_table(&self, doc: &TableDoc, expected: Option<Version>) -> Result<Version, WriteError> {
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
    fn settle(&self, id: &str) -> Result<Option<(TableDoc, Version)>, GameError> {
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
            Some((doc, _)) if doc.status == TableStatus::Dealing => Err(GameError::Dealing),
            Some(_) => self.live_handle(id),
        }
    }

    /// The handle alone: resident, or rebuilt from the RECORD.
    ///
    /// Deliberately does not consult the table document — [`Games::handle`] has
    /// already done that, and `settle`'s repair arm calls this to read the turn
    /// off a game whose table is mid-flip.
    fn live_handle(&self, id: &str) -> Result<Arc<Mutex<Repl>>, GameError> {
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
        drop(written);
        self.mirror_turn(id, repl);
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

    /// Delete, and remember that we did. **The creator's, and nobody else's.**
    ///
    /// The tombstone is what stops an in-flight `input` from writing the record
    /// back after the file is gone. It is bounded by `create`, which clears the
    /// id — so the set only ever holds ids that are deleted and not recreated,
    /// and re-creating a deleted name works exactly as before.
    pub fn delete(&self, id: &str, user: &User) -> Result<(), GameError> {
        let doc = self.table(id, user)?;
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

    /// A normalized identity, the way the extractor produces one.
    fn who(s: &str) -> User {
        User::parse(s).expect("an email")
    }

    /// The cycle's standing request: red at seat 0, the bot at seat 1.
    fn req_seeded(name: &str, seed: u64) -> NewGame {
        NewGame {
            players: 2,
            colors: vec![PlayerColor::Red, PlayerColor::Blue],
            start_seat: 0,
            seed: Some(WireSeed(seed)),
            seats: vec![
                SeatSpec { user: Some("red@t.test".into()), bot: false },
                SeatSpec { user: None, bot: true },
            ],
            name: Some(name.to_string()),
        }
    }

    fn req(name: &str) -> NewGame {
        req_seeded(name, CORPUS_SEED)
    }

    /// The same table with a PERSON in seat 1 — the shape every cell about
    /// per-seat scoping needs, since a bot seat never calls anything.
    fn two_people(name: &str, seed: u64) -> NewGame {
        let mut r = req_seeded(name, seed);
        r.seats[1] = SeatSpec { user: Some("blue@t.test".into()), bot: false };
        r
    }

    /// create -> keep for every human -> the playing handle, as the API does it.
    ///
    /// A game is no longer one call: the cells below that are about PERSISTENCE
    /// want the handle `create` used to hand back, and this is the sequence that
    /// now produces one. Each user keeps for every seat it holds, lowest first,
    /// which is exactly what a hot seat does through the browser.
    fn playing(games: &Games, req: &NewGame, users: &[&str]) -> Arc<Mutex<Repl>> {
        let creator = who(users[0]);
        let id = games.create(req, &creator).expect("a table").id;
        for u in users {
            let user = who(u);
            while let Some(seat) =
                games.table(&id, &user).expect("seated").next_unkept_seat_of(&user)
            {
                games.keep(&id, &user, seat, 1).expect("a keep");
            }
        }
        games.handle(&id).expect("playing")
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
        let p = preview(&req("p"), &rules, &who("red@t.test")).unwrap();
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
        assert!(g.list_for(&who("red@t.test")).unwrap().is_empty());
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
        let p = preview(&req_seeded("hidden", ARGMAX_IS_CARD_2), &rules, &who("red@t.test")).unwrap();
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
        let p = preview(&req_seeded("withheld", ARGMAX_IS_CARD_2), &rules, &who("red@t.test")).unwrap();

        // What the SAME seed puts on a real table, read from the game itself.
        let g = games();
        let handle = playing(&g, &req_seeded("withheld", ARGMAX_IS_CARD_2), &["red@t.test"]);
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
        games
            .store()
            .write_if_version(Doc::Table, "broken", &broken.to_string(), None)
            .expect("the store takes what it is given");

        assert!(matches!(games.table("broken", &red), Err(GameError::Server(_))));
        assert!(matches!(games.deal("broken", &red), Err(GameError::Server(_))));
        // The path that holds `table_lock` across the read, which is the one
        // whose panic would be a poisoned mutex rather than one dropped request.
        assert!(matches!(
            games.keep("broken", &red, PlayerId::new(0), 1),
            Err(GameError::Server(_)),
        ));

        let listed = games.list_for(&red).expect("one bad file does not fail the listing");
        assert!(listed.iter().all(|t| t.id != "broken"), "the malformed table is skipped");
        assert!(
            listed.iter().any(|t| t.id == "healthy"),
            "and the other tables are still on the screen: {:?}",
            listed.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
        );

        games
            .create(&two_people("after", CORPUS_SEED), &red)
            .expect("`table_lock` is not poisoned, so the server can still open a table");
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
}
