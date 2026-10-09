//! **The table document: who may sit at a game, and in which seat.**
//!
//! The room's second seam (`docs/plans/2026-09-14-web-frontend-m6-room-plan.md`
//! §0, R-P4): identity at the origin, SEAT BINDINGS IN A DOCUMENT BESIDE THE
//! RECORD, views scoped in the server. This module is that document.
//!
//! **It is deliberately not in the `GameRecord`.** A record is a seed plus an
//! event log and is replayed by the engine; who may open a game in a browser is
//! not a fact about the game's position and must not become one — a seat
//! binding written into a record would move `SEMANTICS_REV` and would travel
//! with every save the CLI writes. So the two live side by side under one id
//! ([`crate::store::Doc`]), each with its own version, and a seat change and a
//! move land independently.
//!
//! **Three things never leave this document.** The setup `seed` — hidden-set
//! member #1 (`game/observe.rs`), which regenerates both decks, the boxed pile,
//! the board and every deal — the `keeps`, which are what the OTHER seats
//! chose, and, since M6.2, the `lock`, whose salt and hash in a browser are a
//! password to attack offline. [`TableView`] is what a seated caller is handed
//! instead, and it carries a boolean in each one's place: `chosen` per seat
//! (whether a seat has answered is public at any table; which card it took is
//! not) and `locked` for the password.
//!
//! **And the ADDRESSES cross to their owners alone** (M6.5c2 cycle D,
//! R-6.5c2.35, R-D3). A seat's `user` and the table's `created_by` stay in this
//! document as the keys every binding is written under; the view carries a
//! [`SeatView`] per seat — a `name`, a `you`, and to the table's creator alone
//! the address they typed for a seat nobody has named yet — and the creator as
//! the flag `mine` and the name `creator`.

use serde::{Deserialize, Serialize};
use yokohama_core::{PlayerColor, PlayerId};

use crate::client::HistoricalFrame;
use crate::tables::DealCard;
use crate::identity::User;

/// Who plays one seat.
///
/// A human seat carries the NORMALIZED key `identity::User::parse` produced — an
/// email, or (M6.5c2) a plain name — the one key every binding is written
/// under, so a seat bound from a header reading `  Red@Wire.TEST ` is the same
/// seat the next request finds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeatDoc {
    Human {
        user: String,
        /// What this seat is CALLED on screen (M6.2). `None` on every seat a
        /// schema-1 document bound, and on a seat whose holder gave no name —
        /// which the view says as `you` to its holder, as the address to the
        /// creator who typed it, and as the word "invited" to everyone else
        /// ([`SeatView`]), and is why this is optional rather than defaulted to
        /// something here.
        #[serde(default)]
        name: Option<String>,
        /// Whether the CREATOR bound this seat at create — typing an address
        /// for it — rather than a joiner taking an open seat (security S1a,
        /// finding B7). Only such a seat may show its address back to the
        /// creator as `invited` ([`SeatView`]): a joiner's key is the joiner's
        /// own, and the creator has no claim on it. Serde-default `false` and
        /// skipped when false, so a document written before the field reads
        /// every seat as joined and a joined seat's bytes are what they were;
        /// `TABLE_SCHEMA` stays 3.
        #[serde(default, skip_serializing_if = "is_false")]
        bound_at_create: bool,
    },
    /// The shipped production contestant, as the CLI seats it
    /// (`bot::PROVISIONAL_DEFAULT_BOT`). WHICH contestant is not stored here:
    /// the record's own `SeatProvenance` carries it once the game exists, and a
    /// second statement of it would be a second thing to keep true.
    Bot,
    /// An unfilled seat at an open table (M6.2). Never present once the table
    /// has left `Open` — `check_shape` refuses the pair.
    Open,
}

/// Open, dealing, or playing. **"Over" is not a status** — it is read from the
/// session, because a finished game is a property of the record and storing it
/// here would be a copy that can go stale.
///
/// `Open` is M6.2's and is the one status with a SEAT invariant beside it: a
/// table is open exactly while it has an open seat, both directions refused in
/// `tables::check_shape`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TableStatus {
    Open,
    Dealing,
    Playing,
}

/// A few facts of the session, MIRRORED here so the lobby can be built from the
/// table documents alone (R-plan-3). M6.2 added the winner, for the same reason
/// the other two are here: a finished game's row says who won without rebuilding
/// the session that knows.
///
/// **The record is the truth and this is a cache off it.**
/// `Games::persist` refreshes it after every applied move and IGNORES a conflict
/// doing so: a lost mirror write costs a list row that says whose turn it was
/// one move ago, and nothing else — no play route reads this. Listing twenty
/// games by rebuilding twenty sessions is what it exists to avoid.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnMirror {
    pub to_move: u8,
    pub over: bool,
    /// The ENGINE's winner once `over` — the same value `/result` projects as
    /// `Placing.won`, never `standings[0]` (rank order; `outcome.rs:71`).
    #[serde(default)]
    pub winner: Option<u8>,
    /// Whether the RECORD holds any event (cycle E, R-E5; ruling T5-a): the one
    /// fact `tables::saves_on_abandon` needs that the document did not carry,
    /// so a lobby row can say "Abandon and save" without reading the record.
    /// A cache like the rest of this type — `persist` stamps it beside
    /// `to_move`, and a swallowed conflict can leave it stale — which is why
    /// the delete itself asks the RECORD and never this.
    ///
    /// **A bit and not a count, and that is the cost rule**: it changes ONCE
    /// per game, at the first applied move, so `Games::mirror_turn`'s
    /// equal-skip still leaves every later mid-turn move without a table
    /// write — and on the Worker without a slot or a House push. A count
    /// would change on every move.
    ///
    /// `#[serde(default)]`: a mirror written before the field reads `false`
    /// until its next write, and `TABLE_SCHEMA` stays 3. **Absent from the
    /// document while `false`**, [`TurnMirror::bot`]'s rule: a document written
    /// before the field round-trips through this type byte for byte — the
    /// Worker's house copy and its receipt fixtures are built on that round
    /// trip — and a table at its deal stores what it always stored.
    #[serde(default, skip_serializing_if = "is_false")]
    pub has_events: bool,
    /// The RECORD's bot (M6.5c2, R-6.5c2.10): the first non-human seat's
    /// `(controller, version, params)`, so a lobby row can label a dealt
    /// table's bot, and say whether this build runs it, without rebuilding the
    /// session that knows. `None` for an all-human game — and for a table whose
    /// mirror was last written before this field, which reads "the bot" with no
    /// version until its next applied move (a finished one never gains it; no
    /// persist follows game over — review m7).
    ///
    /// **Absent from the document when `None`**, so an all-human table's stored
    /// bytes are what they were; `#[serde(default)]` reads a document written
    /// before it, and neither `TableDoc` nor this type carries
    /// `deny_unknown_fields`, so an older binary reads one written after it —
    /// `TABLE_SCHEMA` stays 3. The field is also why this type is no longer
    /// `Copy`: its readers take a reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot: Option<crate::bots::BotDescriptor>,
}

/// `serde`'s `skip_serializing_if` for [`TurnMirror::has_events`].
fn is_false(b: &bool) -> bool {
    !*b
}

/// The table password, as stored: hex of 16 random bytes and hex of
/// SHA-256(salt_bytes ‖ password_bytes). Serialized for the DOCUMENT; [`TableView`]
/// has no field of this type.
///
/// **A shared game password, not an account credential** (R-2.8). It gates
/// joining a table, and from M6.3 watching one, behind a door Cloudflare Access
/// has already opened; nobody's identity rests on it. That is why there is no
/// argon2 here and no work factor to tune — the salt stops one precomputed
/// table from reading every table's password at once, and that is the whole of
/// what this is for.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lock {
    pub salt: String,
    pub hash: String,
}

impl Lock {
    pub fn new(password: &str) -> Self {
        let salt_bytes: [u8; 16] = {
            let a = crate::tables::draw_seed().to_le_bytes();
            let b = crate::tables::draw_seed().to_le_bytes();
            let mut s = [0u8; 16];
            s[..8].copy_from_slice(&a);
            s[8..].copy_from_slice(&b);
            s
        };
        let salt = hex(&salt_bytes);
        let hash = hex(&digest(&salt_bytes, password));
        Self { salt, hash }
    }

    /// Constant-time over the hash bytes: every byte is compared and the
    /// mismatches are folded with OR, so the time does not depend on WHERE the
    /// first difference is.
    pub fn matches(&self, given: &str) -> bool {
        let Some(salt_bytes) = unhex(&self.salt) else { return false };
        let Some(stored) = unhex(&self.hash) else { return false };
        let computed = digest(&salt_bytes, given);
        if stored.len() != computed.len() {
            return false;
        }
        let mut diff = 0u8;
        for (a, b) in stored.iter().zip(computed.iter()) {
            diff |= a ^ b;
        }
        diff == 0
    }
}

fn digest(salt: &[u8], password: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(salt);
    h.update(password.as_bytes());
    h.finalize().to_vec()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `None` on anything that is not an even number of ASCII hex digits.
///
/// **The `is_ascii` guard is a panic fence, not a taste**: `s[i..i + 2]` slices
/// by BYTE, and a multi-byte character straddling that boundary is a panic
/// rather than an error. `tables::check_shape` refuses a stored lock that is
/// not hex, so no document reaching [`Lock::matches`] through the API has one —
/// but `matches` is public and this makes it total on its own terms.
fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.is_ascii() || !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// The document, as it is stored (`<id>.table.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDoc {
    /// This document's own shape version, unrelated to `SEMANTICS_REV`.
    pub schema: u32,
    pub id: String,
    pub name: Option<String>,
    pub created_by: String,
    /// Unix seconds (R-plan-1). A number rather than a formatted string: the
    /// list sorts on it, and a sort over an ISO string is a sort that works
    /// until a timezone or a fractional second appears in one.
    pub created_at: u64,
    pub players: u8,
    pub colors: Vec<PlayerColor>,
    pub start_seat: u8,
    pub seats: Vec<SeatDoc>,
    /// `1` or `2` once a HUMAN seat has chosen; a bot seat's stays `None`
    /// forever — the bot's keep is `tables::bot_keep`'s, computed at finalize and
    /// living in the record, never here.
    pub keeps: Vec<Option<u8>>,
    /// **Never on the wire.** See the module doc.
    pub seed: u64,
    pub status: TableStatus,
    /// `Some` once playing; see [`TurnMirror`].
    pub turn: Option<TurnMirror>,
    /// The table password, or none (M6.2). **Never on the wire** — like `seed`
    /// and `keeps`, and for the plainer reason that a salt and a hash handed to
    /// a browser are a password to attack offline. [`TableView`] carries the
    /// boolean `locked` in its place.
    #[serde(default)]
    pub lock: Option<Lock>,
    /// Everyone who has presented this table's password to `POST …/{id}/unlock`
    /// and may therefore WATCH it (M6.3, R-3.5). Normalized keys, as every
    /// other binding here is.
    ///
    /// **Never on the wire**, like `seed`, `keeps` and `lock` — it is the list
    /// of other people at this table who are not playing, which is nobody
    /// else's business; [`TableView::can_watch`] carries the one boolean the
    /// caller needs about themselves. Additive with a default, so `TABLE_SCHEMA`
    /// stays 2 and a document written before this cycle reads with an empty
    /// list; `tables::check_shape` bounds it at
    /// [`crate::tables::UNLOCKED_CAP`] and requires each entry to parse.
    #[serde(default)]
    pub unlocked: Vec<String>,
    /// When this table was archived (M6.5a, R-6.5a.10) — unix seconds from the
    /// layer's clock — or `None`. Set at game over when the put lands and at an
    /// abandon whose put lands; **unset after a failed put at game over is what
    /// says a retry is owed** (umbrella R-6.5.4). Additive with a default, so a
    /// schema-2 document reads with `None`; `TABLE_SCHEMA` is 3 since this
    /// field. Bounded in `tables::check_shape` like every other stamp.
    #[serde(default)]
    pub archived_at: Option<u64>,
}

impl TableDoc {
    /// Every seat this user holds, lowest first. Empty for a stranger, which is
    /// the 403 on every route that needs a seat.
    pub fn seats_of(&self, user: &User) -> Vec<PlayerId> {
        self.seats
            .iter()
            .enumerate()
            .filter(|(_, s)| matches!(s, SeatDoc::Human { user: u, .. } if u == user.as_str()))
            .map(|(i, _)| PlayerId::new(i as u8))
            .collect()
    }

    /// Every seat nobody holds yet, lowest first — what a join fills and what
    /// the lobby counts.
    pub fn open_seats(&self) -> Vec<PlayerId> {
        self.seats
            .iter()
            .enumerate()
            .filter(|(_, s)| matches!(s, SeatDoc::Open))
            .map(|(i, _)| PlayerId::new(i as u8))
            .collect()
    }

    pub fn is_full(&self) -> bool {
        self.open_seats().is_empty()
    }

    /// True when the table has no lock, or the lock matches.
    pub fn check_password(&self, given: &str) -> bool {
        self.lock.as_ref().is_none_or(|l| l.matches(given))
    }

    /// May this caller WATCH? A seat holder always; anyone at a table with no
    /// lock; a caller on the `unlocked` list (M6.3, R-3.5).
    ///
    /// **It says nothing about the STATUS**, and that is deliberate: watching a
    /// deal is refused by the crate root's `admitted`, not here, so this one
    /// predicate answers the same question for the view (which the lobby reads
    /// at every status) and for the admission (which reads it at `playing`
    /// alone). A seat holder is included because `can_watch` is what the client
    /// draws the unlock face from, and asking a player for their own table's
    /// password is the one answer that is always wrong.
    pub fn can_watch(&self, user: &User) -> bool {
        !self.seats_of(user).is_empty()
            || self.lock.is_none()
            || self.unlocked.iter().any(|u| u == user.as_str())
    }

    /// How many DIFFERENT people hold a seat here.
    ///
    /// One is the vs-bot and hot-seat case, which R-P6 lets keep today's undo;
    /// two or more is a table where undo has to be fenced. A count of human
    /// SEATS would answer 2 for one person holding both, which is exactly the
    /// case that must not be fenced.
    pub fn distinct_humans(&self) -> usize {
        self.seats
            .iter()
            .filter_map(|s| match s {
                SeatDoc::Human { user, .. } => Some(user.as_str()),
                SeatDoc::Bot | SeatDoc::Open => None,
            })
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    }

    /// Has every human seat chosen? A bot seat is never waited for.
    pub fn all_humans_kept(&self) -> bool {
        self.seats.iter().zip(&self.keeps).all(|(s, k)| matches!(s, SeatDoc::Bot) || k.is_some())
    }

    /// The LOWEST seat of this user's that has not yet chosen — what
    /// `GET …/deal` serves, and what makes a hot seat a sequence with no "next
    /// seat" button anywhere in it.
    ///
    /// `get` rather than an index: the seat came out of `seats`, and a document
    /// whose `keeps` is shorter than that would panic here. `Games::read_table`
    /// refuses such a document before it reaches this type, so the arm is
    /// unreachable through the API — it is written this way so the method is
    /// total on its own terms rather than on its caller's.
    pub fn next_unkept_seat_of(&self, user: &User) -> Option<PlayerId> {
        self.seats_of(user)
            .into_iter()
            .find(|p| self.keeps.get(usize::from(p.0)).is_none_or(Option::is_none))
    }

    /// R-6.5b.6 check (4): this document describes `record`'s game.
    ///
    /// Compared: the player count, the colours, the start seat, the seed, and
    /// every keep the document CLAIMS (`Some(n)`, 1-based) against
    /// `choices[i] == Keep(n - 1)` — the `- 1` of `tables::choices_of`, whose
    /// operand type settles this one.
    ///
    /// **A `None` keep is NOT compared, and that is the rule rather than a
    /// leniency**: a bot seat's keep lives in the record and never here —
    /// `create` fills `keeps` with `None`, only a seat holder may `keep`, and
    /// `choices_of` recomputes the bot's from `bot_keep` at finalize — so every
    /// real local pair reads `[Some(n), None]` against two `Keep`s, and a rule
    /// that compared the `None` would refuse every genuine send.
    ///
    /// Nothing here reads a field the record does not have: the stored seed, the
    /// config and the choices are exactly what `new_game` and `finalize_setup`
    /// were given, so agreement means the two documents describe one game.
    pub fn agrees_with(&self, record: &yokohama_core::GameRecord) -> Result<(), String> {
        if usize::from(self.players) != usize::from(record.config.player_count) {
            return Err(format!(
                "players: the table says {}, the record {}",
                self.players, record.config.player_count
            ));
        }
        if self.colors != record.config.player_colors {
            return Err("colors: the table and the record disagree".to_string());
        }
        if self.start_seat != record.config.start_player.0 {
            return Err(format!(
                "start_seat: the table says {}, the record {}",
                self.start_seat, record.config.start_player.0
            ));
        }
        if self.seed != record.seed {
            return Err("seed: the table and the record disagree".to_string());
        }
        for (i, keep) in self.keeps.iter().enumerate() {
            if let Some(n) = keep {
                // **`checked_sub`, because this is `pub` and the numbering is
                // 1-based.** `Some(0)` names no card; through the route it
                // cannot arrive — `check_shape` bounds every keep to 1 or 2
                // before anything reads it — but a caller of this function has
                // made no such promise, and the bare `- 1` underflowed rather
                // than refusing.
                let Some(index) = n.checked_sub(1) else {
                    return Err(format!("keep: seat {i} claims card 0, which is not a card"));
                };
                let expected = yokohama_core::StartingOrderChoice::Keep(index);
                if record.choices.get(i) != Some(&expected) {
                    return Err(format!(
                        "keep: seat {i} kept card {n} by the table and not by the record"
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn is_creator(&self, user: &User) -> bool {
        self.created_by == user.as_str()
    }

    /// The creator's seat NAME: the first seat they hold that carries one, or
    /// `None` — a creator who holds no seat (the Worker's seatless table),
    /// or whose seats are all unnamed. Never the address.
    fn creator_name(&self) -> Option<String> {
        self.seats.iter().find_map(|s| match s {
            SeatDoc::Human { user, name: Some(name), .. } if *user == self.created_by => Some(name.clone()),
            _ => None,
        })
    }

    /// What a caller is handed — built FOR `user`: every per-reader field
    /// (`mine`, each seat's `you` and `invited`, `my_seats`, `can_watch`) is
    /// this caller's answer. See the module doc for what never crosses, and
    /// for the addresses, which cross to their owners alone.
    ///
    /// `archive_on` is the HOST's fact, handed in by the route
    /// ([`crate::games::Games::archive_on`]): `saves` is the abandon rule's
    /// answer over it and the mirror's `has_events` (cycle E, R-E5), and a
    /// view built with `false` saves nothing.
    pub fn view(&self, user: &User, archive_on: bool) -> TableView {
        let mine = self.is_creator(user);
        let has_events = self.turn.as_ref().is_some_and(|t| t.has_events);
        TableView {
            id: self.id.clone(),
            name: self.name.clone(),
            mine,
            creator: self.creator_name(),
            created_at: self.created_at,
            players: self.players,
            colors: self.colors.clone(),
            start_seat: self.start_seat,
            seats: self.seats.iter().map(|s| SeatView::of(s, user, mine)).collect(),
            chosen: self
                .seats
                .iter()
                .zip(&self.keeps)
                .map(|(s, k)| matches!(s, SeatDoc::Bot) || k.is_some())
                .collect(),
            my_seats: self.seats_of(user),
            status: self.status,
            to_move: self.turn.as_ref().map(|t| PlayerId::new(t.to_move)),
            over: self.turn.as_ref().map(|t| t.over),
            locked: self.lock.is_some(),
            winner: self.turn.as_ref().and_then(|t| t.winner).map(PlayerId::new),
            can_watch: self.can_watch(user),
            bot: crate::bots::table_bot(self),
            has_events,
            saves: crate::tables::saves_on_abandon(self, has_events, archive_on),
        }
    }
}

/// One seat as a reader is handed it (M6.5c2 cycle D, R-6.5c2.35, R-D3):
/// tagged like [`SeatDoc`], with the same three arms, and NO address but the
/// creator's own typing.
///
/// - `name` is what the seat's holder gave, or `null`.
/// - `you` is whether the READER holds this seat.
/// - `invited` is the address the creator typed at create — a seat the
///   creator BOUND (`bound_at_create`), never a joiner's — served to the
///   creator alone and only while the seat has no name — the one way the
///   person who invited somebody can still tell their seats apart before that
///   person has said what to call them. Every other reader of an unnamed seat
///   is handed the word "invited" by the client.
///
/// **An address is not a display.** It was, until this cycle: every reader of
/// a table was handed every seat's email, the lobby's to anyone signed in.
/// Two people who pick one name are told apart by their colour, as they are on
/// the board.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeatView {
    Human { name: Option<String>, you: bool, invited: Option<String> },
    Bot,
    Open,
}

impl SeatView {
    /// `seat` as `reader` is handed it, where `mine` is whether the reader
    /// created the table.
    pub fn of(seat: &SeatDoc, reader: &User, mine: bool) -> Self {
        match seat {
            SeatDoc::Human { user, name, bound_at_create } => SeatView::Human {
                name: name.clone(),
                you: user == reader.as_str(),
                invited: (mine && name.is_none() && *bound_at_create).then(|| user.clone()),
            },
            SeatDoc::Bot => SeatView::Bot,
            SeatDoc::Open => SeatView::Open,
        }
    }
}

/// The table on the wire.
///
/// **No address but the reader's own crosses here** (M6.5c2 cycle D,
/// R-6.5c2.35, R-D3). Until this cycle every seat's EMAIL and the creator's
/// were here, readable by ANY signed-in caller at an OPEN table (R-2.7) and at
/// a PLAYING or finished one (R-3.13). A seat is now a [`SeatView`] — a name,
/// a `you`, and to the creator alone the address they typed for a seat nobody
/// has named — and the creator is `mine` and `creator`.
///
/// **The one caller this type is withheld from is a caller with NO SEAT at a
/// table that is DEALING** — a seat holder reads the roster at every status,
/// so "dealing withholds it" would be the wrong fence by half. It is narrower
/// than M6.1's and the doc used to say otherwise. R-2.7 opened
/// `GET …/{id}/table` at `open` so a person following a join link can see what
/// they are joining; R-3.13 opened it at `playing` and over so a would-be
/// watcher can read `locked` and `can_watch` before asking for a password; and
/// R-3.9's `?scope=playing` lists every game under way to anybody, each row a
/// full `TableView`. A LOCKED table is no exception — the lock gates joining
/// and watching, never seeing that the game exists — so a stranger reads a
/// locked table's seats with `my_seats: []` and `can_watch: false`.
///
/// What is still withheld from this type at every status is the module doc's
/// list: the `seed`, the `keeps`, the `lock`, M6.3's `unlocked` list, and
/// every address but the reader's own and the creator's `invited`.
#[derive(Debug, Clone, Serialize)]
pub struct TableView {
    pub id: String,
    pub name: Option<String>,
    /// Whether the READER created this table — what draws the creator's own
    /// controls (Abandon, the invite's password hint). A fact about the
    /// reader, like `can_watch`.
    pub mine: bool,
    /// The creator's seat NAME, or `null` when they hold no seat or gave no
    /// name — never their address.
    pub creator: Option<String>,
    pub created_at: u64,
    pub players: u8,
    pub colors: Vec<PlayerColor>,
    pub start_seat: u8,
    pub seats: Vec<SeatView>,
    /// True for a bot seat from the start, and for a human seat once it has
    /// kept. The keeps themselves never cross.
    pub chosen: Vec<bool>,
    pub my_seats: Vec<PlayerId>,
    pub status: TableStatus,
    /// Present once playing, `null` while dealing — both from [`TurnMirror`].
    pub to_move: Option<PlayerId>,
    pub over: Option<bool>,
    /// Whether a password is needed to join (and, from M6.3, to watch). The
    /// salt and the hash stay in the document; see [`Lock`].
    pub locked: bool,
    /// Who won, once `over`. `null` at every other moment, including a table
    /// that is still dealing.
    pub winner: Option<PlayerId>,
    /// Whether THIS caller may watch: a seat holder, anyone at an unlocked
    /// table, or someone who has presented the password (M6.3, R-3.5). The
    /// client draws the game or the unlock face from it, so it is a fact about
    /// the reader and not about the table — two people reading one table get
    /// two different answers, which is why the `unlocked` list itself never
    /// crosses.
    pub can_watch: bool,
    /// The table's bot, or `null` (M6.5c2, R-6.5c2.10): its label and whether
    /// this build runs it — `crate::bots::table_bot`'s answer. An undealt
    /// table's is the production bot, which `finalize` will deal; a dealt one's
    /// is the record's, read off [`TurnMirror::bot`]; `null` at a table with no
    /// bot seat, and at a dealt one whose mirror predates the field.
    pub bot: Option<crate::bots::TableBot>,
    /// Whether the game's record holds any event, off
    /// [`TurnMirror::has_events`]: `false` before the deal, before the first
    /// move, and at a mirror written before the field (cycle E, R-E5; ruling
    /// T5-a). What a browser's own rule for a LOCAL game reads. A bit, never
    /// a count: see the mirror's field for why.
    pub has_events: bool,
    /// Whether the creator's Abandon would SAVE this table before deleting it —
    /// `tables::saves_on_abandon` over the mirror's `has_events` and the
    /// host's archive switch (cycle E, R-E5): what the client words "Abandon
    /// and save" from. The delete asks the RECORD instead, so the two part
    /// only where the mirror is stale.
    pub saves: bool,
}

/// What `GET /api/games/{id}/deal` serves: the set table, and the caller's OWN
/// two cards.
///
/// **`seat` and `cards` are null together**, and that is the "waiting for the
/// others" screen rather than an error: every seat this caller holds has
/// chosen, and the table is not finished dealing.
#[derive(Debug, Clone, Serialize)]
pub struct DealView {
    pub table: TableView,
    /// The caller's lowest seat that has not yet kept.
    pub seat: Option<PlayerId>,
    pub cards: Option<Vec<DealCard>>,
    /// `wizard::KEEP_SCORE_NOTE`, SERVED rather than copied into the browser:
    /// the number is the built-in bot's preference from the card's printed
    /// economics, and saying so beside it is the difference between a factual
    /// number and an implied recommendation.
    pub keep_note: &'static str,
    /// The position after setup, with the technologies and the achievements
    /// withheld until the choice (2026-09-06, R-C3).
    pub frame: HistoricalFrame,
}

/// Now, in unix seconds.
///
/// A clock that reads before the epoch gives 0 rather than failing: the only
/// consumer is the list's sort order, and a table sorting last is a better
/// answer than a create that refuses because the host's clock is wrong.
#[cfg(feature = "server")]
pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---- end server ----

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::User;
    use yokohama_core::{PlayerColor, PlayerId};

    fn doc() -> TableDoc {
        TableDoc {
            schema: 1,
            id: "t".into(),
            name: None,
            created_by: "a@t.test".into(),
            created_at: 0,
            players: 3,
            colors: vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow],
            start_seat: 0,
            seats: vec![
                SeatDoc::Human { user: "a@t.test".into(), name: None, bound_at_create: false },
                SeatDoc::Human { user: "b@t.test".into(), name: None, bound_at_create: false },
                SeatDoc::Human { user: "a@t.test".into(), name: None, bound_at_create: false },
            ],
            keeps: vec![None, None, None],
            seed: 7,
            status: TableStatus::Dealing,
            turn: None,
            lock: None,
            unlocked: Vec::new(),
            archived_at: None,
        }
    }

    #[test]
    fn seats_of_a_user_are_theirs_and_in_order_and_a_stranger_has_none() {
        let a = User::parse("a@t.test").unwrap();
        assert_eq!(doc().seats_of(&a), vec![PlayerId::new(0), PlayerId::new(2)]);
        assert!(doc().seats_of(&User::parse("nobody@t.test").unwrap()).is_empty());
        assert_eq!(doc().distinct_humans(), 2);
    }

    #[test]
    fn the_next_unkept_seat_is_the_lowest_of_mine_and_none_once_all_mine_have_kept() {
        let a = User::parse("a@t.test").unwrap();
        let mut d = doc();
        assert_eq!(d.next_unkept_seat_of(&a), Some(PlayerId::new(0)));
        d.keeps[0] = Some(1);
        assert_eq!(d.next_unkept_seat_of(&a), Some(PlayerId::new(2)), "hot-seat: the next of mine");
        d.keeps[2] = Some(2);
        assert_eq!(d.next_unkept_seat_of(&a), None);
        assert!(!d.all_humans_kept(), "b has not kept");
        d.keeps[1] = Some(1);
        assert!(d.all_humans_kept());
    }

    #[test]
    fn the_view_carries_no_seed_and_no_keeps_and_marks_a_bot_chosen_from_the_start() {
        let mut d = doc();
        d.seats[1] = SeatDoc::Bot;
        d.keeps[0] = Some(2);
        let v = serde_json::to_value(d.view(&User::parse("a@t.test").unwrap(), false)).unwrap();
        assert_eq!(v["chosen"], serde_json::json!([true, true, false]));
        assert_eq!(v["my_seats"], serde_json::json!([0, 2]));
        assert_eq!(v["status"], "dealing");
        assert_eq!(v["to_move"], serde_json::Value::Null);
        let text = v.to_string();
        assert!(!text.contains("seed") && !text.contains("keeps"), "{text}");
        assert_eq!(v["seats"][1], serde_json::json!({ "kind": "bot" }));
    }

    #[test]
    fn the_document_round_trips_through_json() {
        let d = doc();
        let back: TableDoc = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), serde_json::to_string(&d).unwrap());
    }

    /// **The real local pair's shape** (M6.5b, R-6.5b.6): a 2P PLAYING table
    /// with one person and one bot, so `keeps` reads `[Some(n), None]`.
    ///
    /// A third builder rather than a widened [`doc`], for the same reason
    /// [`open_doc`] is one: `check_shape` refuses each other's status paired
    /// with each other's seats. This one is the shape a browser game really
    /// sends, and the cell below would pass on a synthetic all-human pair while
    /// refusing every real input.
    fn playing_doc() -> TableDoc {
        TableDoc {
            schema: 3,
            id: "local-1".into(),
            name: None,
            created_by: "a@t.test".into(),
            created_at: 1_789_000_000,
            players: 2,
            colors: vec![PlayerColor::Red, PlayerColor::Blue],
            start_seat: 0,
            seats: vec![SeatDoc::Human { user: "a@t.test".into(), name: None, bound_at_create: false }, SeatDoc::Bot],
            keeps: vec![Some(1), None],
            seed: 7,
            status: TableStatus::Playing,
            turn: None,
            lock: None,
            unlocked: Vec::new(),
            archived_at: None,
        }
    }

    /// The record the document above describes, built the way `record.rs`'s own
    /// tests build one.
    ///
    /// **A human keep `Some(n)` is `Keep(n - 1)`; a bot's `None` is whatever the
    /// bot kept** — `Keep(1)` here, deliberately NOT the card the table claims
    /// for seat 0 — and is not compared.
    fn record_for(doc: &TableDoc) -> yokohama_core::GameRecord {
        use yokohama_core::{GameConfig, PlayerId, Rules, StartingOrderChoice};
        let mut config = GameConfig::for_player_count(doc.players);
        config.player_colors = doc.colors.clone();
        config.start_player = PlayerId::new(doc.start_seat);
        let choices = doc
            .keeps
            .iter()
            .map(|k| StartingOrderChoice::Keep(k.map(|n| n - 1).unwrap_or(1)))
            .collect();
        yokohama_core::GameRecord::new(doc.seed, config, choices, &Rules::standard())
    }

    /// R-6.5b.6 check (4): the five compared fields each refused BY NAME, and
    /// the real browser-played pair ACCEPTED.
    ///
    /// The accepting arm is the one that matters: five refusals over a
    /// synthetic all-human pair would pass while the only input this route ever
    /// really sees — one human keep beside a bot's `None` — was refused.
    #[test]
    fn a_table_agrees_with_its_record_and_a_bot_keep_is_not_compared() {
        let mut doc = playing_doc();
        doc.keeps = vec![Some(2), None];
        assert!(
            doc.agrees_with(&record_for(&doc)).is_ok(),
            "the real local pair: one human keep, one bot None"
        );
        let rec = record_for(&doc);
        let mut d = doc.clone();
        d.players = 3;
        assert!(d.agrees_with(&rec).unwrap_err().contains("players"));
        let mut d = doc.clone();
        d.colors.reverse();
        assert!(d.agrees_with(&rec).unwrap_err().contains("colors"));
        let mut d = doc.clone();
        d.start_seat = 1;
        assert!(d.agrees_with(&rec).unwrap_err().contains("start_seat"));
        let mut d = doc.clone();
        d.seed += 1;
        assert!(d.agrees_with(&rec).unwrap_err().contains("seed"));
        let mut d = doc.clone();
        d.keeps[0] = Some(1);
        assert!(
            d.agrees_with(&rec).unwrap_err().contains("keep"),
            "a CLAIMED keep that disagrees is refused"
        );
        // A keep of `0`: the numbering here is 1-based, so there is no such
        // card. Unreachable through the route — `check_shape` bounds a keep to
        // 1 or 2 before `agrees_with` is ever called — but this is `pub`, and
        // the arithmetic used to underflow rather than refuse.
        let mut d = doc.clone();
        d.keeps[0] = Some(0);
        assert!(
            d.agrees_with(&rec).unwrap_err().contains("card 0"),
            "a keep of card 0 is refused by name, not by an underflow"
        );
    }

    /// M6.2's shape: an OPEN table with two unfilled seats, a display name on
    /// the one seat that is taken, and no lock.
    ///
    /// A second helper rather than a widened [`doc`], because the cells above
    /// are about a DEALING table and the ones below are about an open one —
    /// and `check_shape` refuses each other's status paired with each other's
    /// seats, so one fixture cannot serve both.
    fn open_doc() -> TableDoc {
        TableDoc {
            schema: 2,
            id: "t".into(),
            name: None,
            created_by: "red@t.test".into(),
            created_at: 1_789_000_000,
            players: 3,
            colors: vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Green],
            start_seat: 0,
            seats: vec![
                SeatDoc::Human { user: "red@t.test".into(), name: Some("Red".into()), bound_at_create: false },
                SeatDoc::Open,
                SeatDoc::Open,
            ],
            keeps: vec![None, None, None],
            seed: 1,
            status: TableStatus::Open,
            turn: None,
            lock: None,
            unlocked: Vec::new(),
            archived_at: None,
        }
    }

    #[test]
    fn a_schema_one_document_reads_with_every_new_field_at_its_default() {
        let json = r#"{"schema":1,"id":"old","name":"old","created_by":"red@t.test",
            "created_at":1789000000,"players":2,"colors":["Red","Blue"],"start_seat":0,
            "seats":[{"kind":"human","user":"red@t.test"},{"kind":"human","user":"blue@t.test"}],
            "keeps":[1,2],"seed":7,"status":"playing","turn":{"to_move":0,"over":false}}"#;
        let doc: TableDoc = serde_json::from_str(json).expect("a schema-1 document parses");
        assert!(doc.lock.is_none());
        assert!(doc.unlocked.is_empty(), "M6.3's list defaults empty on a document written before it");
        assert!(doc.seats.iter().all(|s| matches!(s, SeatDoc::Human { name: None, .. })));
        assert_eq!(doc.turn.as_ref().expect("a mirror").winner, None);
        assert_eq!(doc.turn.as_ref().expect("a mirror").bot, None, "a mirror written before the field reads none");
        assert!(!doc.is_full() || doc.open_seats().is_empty());
    }

    #[test]
    fn open_seats_and_is_full_read_the_seats() {
        let d = open_doc();
        assert_eq!(d.open_seats(), vec![PlayerId::new(1), PlayerId::new(2)]);
        assert!(!d.is_full());
        let mut full = d.clone();
        full.seats[1] = SeatDoc::Bot;
        full.seats[2] = SeatDoc::Human { user: "b@t.test".into(), name: None, bound_at_create: false };
        assert!(full.is_full());
        assert!(full.open_seats().is_empty());
    }

    #[test]
    fn a_lock_matches_its_password_and_refuses_a_one_byte_variant_and_the_empty_string() {
        let lock = Lock::new("sesame");
        assert_eq!(lock.salt.len(), 32, "16 bytes as hex");
        assert_eq!(lock.hash.len(), 64, "SHA-256 as hex");
        assert!(lock.matches("sesame"));
        assert!(!lock.matches("sesamf"));
        assert!(!lock.matches("sesam"));
        assert!(!lock.matches(""));
        assert_ne!(Lock::new("sesame").salt, lock.salt, "two locks for one password differ");
        let mut d = open_doc();
        assert!(d.check_password("anything"), "no lock: any password passes");
        d.lock = Some(lock);
        assert!(d.check_password("sesame"));
        assert!(!d.check_password("open"));
    }

    #[test]
    fn the_view_carries_locked_and_winner_and_never_the_salt_or_the_hash() {
        let mut d = open_doc();
        let lock = Lock::new("sesame");
        let (salt, hash) = (lock.salt.clone(), lock.hash.clone());
        d.lock = Some(lock);
        d.turn = Some(TurnMirror { to_move: 1, over: true, winner: Some(2), has_events: false, bot: None });
        let v = serde_json::to_value(d.view(&User::parse("red@t.test").unwrap(), false)).unwrap();
        assert_eq!(v["locked"], true);
        assert_eq!(v["winner"], 2);
        let text = v.to_string();
        for key in ["salt", "hash", "\"lock\""] {
            assert!(!text.contains(key), "{key} must not reach the view: {text}");
        }
        // And the VALUES, under whatever key: the key search above is blind to
        // a salt re-served as `nonce` — measured, that mutation left this cell
        // green and only `wire_fixtures`' run-wide value search saw it. With
        // these two lines the `--lib` binary carries a value catcher too, which
        // is what spec §8.4's mutation 4 claims of this cell.
        assert!(!text.contains(&salt), "the salt reached the view under some key: {text}");
        assert!(!text.contains(&hash), "the hash reached the view under some key: {text}");
        assert_eq!(v["seats"][1]["kind"], "open");
        assert_eq!(v["seats"][0]["name"], "Red");
    }

    /// **R-E5 (cycle E; ruling T5-a): the view's `has_events` is the mirror's
    /// bit and its `saves` the abandon rule's**, asked over the mirror and the
    /// archive switch the caller hands in. A mirror stored before the field
    /// reads `false` (`#[serde(default)]`, so `TABLE_SCHEMA` stays 3) and is
    /// written back as it was stored, and a view built with the archive off
    /// saves nothing whatever the table holds.
    #[test]
    fn a_mirror_without_has_events_reads_false_and_the_view_saves_only_where_the_rule_says() {
        let json = r#"{"schema":3,"id":"old","name":null,"created_by":"red@t.test",
            "created_at":1789000000,"players":2,"colors":["Red","Blue"],"start_seat":0,
            "seats":[{"kind":"human","user":"red@t.test"},{"kind":"human","user":"blue@t.test"}],
            "keeps":[1,2],"seed":7,"status":"playing","turn":{"to_move":0,"over":false}}"#;
        let doc: TableDoc = serde_json::from_str(json).expect("a mirror written before the field parses");
        assert!(!doc.turn.as_ref().expect("a mirror").has_events, "an older mirror reads no events");
        let stored = |d: &TableDoc| serde_json::to_value(d).unwrap()["turn"].clone();
        assert_eq!(
            stored(&doc),
            serde_json::json!({ "to_move": 0, "over": false, "winner": null }),
            "absent while false: an older mirror is written back as it was stored"
        );
        let red = User::parse("red@t.test").unwrap();
        let v = doc.view(&red, true);
        assert_eq!((v.has_events, v.saves), (false, false), "no event, nothing to save");
        let mut moved = doc.clone();
        moved.turn.as_mut().expect("a mirror").has_events = true;
        assert_eq!(stored(&moved)["has_events"], true, "and present once there is one");
        let v = serde_json::to_value(moved.view(&red, true)).unwrap();
        assert_eq!((v["has_events"].clone(), v["saves"].clone()), (serde_json::json!(true), serde_json::json!(true)));
        assert!(v.get("events").is_none(), "a bit on the wire, never a count: {v}");
        assert!(!moved.view(&red, false).saves, "the archive off saves nothing");
        let mut stamped = moved.clone();
        stamped.archived_at = Some(1_789_000_001);
        assert!(!stamped.view(&red, true).saves, "an archived table is not saved twice");
        let mut dealing = moved;
        dealing.status = TableStatus::Dealing;
        assert!(!dealing.view(&red, true).saves, "only a playing table is saved");
    }

    /// [`TableDoc::can_watch`]'s three arms and its refusal, in one cell —
    /// because the claim is the DISJUNCTION, and each arm alone is satisfied by
    /// an implementation that ignores the other two.
    #[test]
    fn a_seat_holder_watches_always_a_stranger_only_unlocked_or_listed() {
        let red = User::parse("red@t.test").expect("an email");
        let green = User::parse("green@t.test").expect("an email");
        let grey = User::parse("grey@t.test").expect("an email");

        let mut d = open_doc();
        assert!(d.can_watch(&red), "a seat holder, at a table with no lock");
        assert!(d.can_watch(&green), "no lock: anyone watches");

        d.lock = Some(Lock::new("sesame"));
        assert!(d.can_watch(&red), "a seat holder needs no password at their own table");
        assert!(!d.can_watch(&green), "locked, and this caller has presented nothing");

        d.unlocked.push("grey@t.test".to_string());
        assert!(d.can_watch(&grey), "on the list");
        assert!(!d.can_watch(&green), "and one unlock admits ONE caller");
    }

    /// **The view names the table's bot** (M6.5c2, R-6.5c2.10): an undealt
    /// table's is the one this build deals, a dealt one's is its mirror's — the
    /// record's — and `null` where there is no bot seat, or where a dealt
    /// table's mirror predates the field (review m7). `runs_here` asks the
    /// CLI's resume of the whole TRIPLE, not the name (review m12; cycle F,
    /// spec §2.3): a bot that resume seats exactly runs here whichever bot this
    /// build deals, and a drift in the version or any knob does not.
    #[test]
    fn the_view_names_the_bot_undealt_dealt_foreign_and_before_the_field() {
        use serde_json::{json, Value};
        let reader = User::parse("a@t.test").expect("an email");
        let bot_of = |d: &TableDoc| serde_json::to_value(d.view(&reader, false)).expect("a view")["bot"].clone();
        let production = crate::bots::production();
        let runs_here = json!({ "label": crate::bots::label(&production), "runs_here": true });

        let mut undealt = playing_doc();
        undealt.status = TableStatus::Dealing;
        undealt.keeps = vec![None, None];
        assert_eq!(bot_of(&undealt), runs_here, "undealt: the bot `finalize` will deal");

        let mut dealt = playing_doc();
        dealt.turn = Some(TurnMirror { to_move: 0, over: false, winner: None, has_events: false, bot: Some(production) });
        assert_eq!(bot_of(&dealt), runs_here, "dealt: the record's, from the mirror");

        let mut foreign = dealt.clone();
        foreign.turn.as_mut().and_then(|t| t.bot.as_mut()).expect("a bot").controller = "v2-padded".into();
        assert_eq!(bot_of(&foreign), json!({ "label": "V2", "runs_here": false }), "another bot: named, not run here");

        // A bot this build's CLI resumes exactly — `lookahead-v1`'s own
        // descriptor, all three halves — runs here, though it is not the one
        // this build deals (cycle F, R-F4).
        let mut genuine = dealt.clone();
        let (controller, version, params) = yokohama_core::Contestant::LOOKAHEAD_V1_DEFAULT.descriptor();
        genuine.turn.as_mut().expect("a mirror").bot = Some(crate::bots::BotDescriptor { controller, version, params });
        assert_eq!(bot_of(&genuine), json!({ "label": "V2 lookahead", "runs_here": true }), "a grandfathered bot runs here");

        // Production's NAME at another version, or with other weights, still
        // labels "V4" (`from_parts` reads neither), and does not run here: a
        // name-only compare would answer `true` for both.
        let mut other_version = dealt.clone();
        other_version.turn.as_mut().and_then(|t| t.bot.as_mut()).expect("a bot").version += 1;
        assert_eq!(bot_of(&other_version), json!({ "label": "V4", "runs_here": false }), "same name, another version");
        let mut other_weights = dealt.clone();
        *other_weights.turn.as_mut().and_then(|t| t.bot.as_mut()).expect("a bot").params.get_mut("weights").expect("weights") += 1;
        assert_eq!(bot_of(&other_weights), json!({ "label": "V4", "runs_here": false }), "same name, other weights");

        let mut older = dealt.clone();
        older.turn.as_mut().expect("a mirror").bot = None;
        assert_eq!(bot_of(&older), Value::Null, "a mirror written before the field names no version");

        let mut humans = dealt;
        humans.seats[1] = SeatDoc::Human { user: "b@t.test".into(), name: None, bound_at_create: false };
        assert_eq!(bot_of(&humans), Value::Null, "no bot seat, whatever the mirror says");
    }

    /// Every path in `v` at which a string VALUE contains `@`, and every key that
    /// does — the tree walk `wire_fixtures`' address pin makes, in small.
    fn at_signs(v: &serde_json::Value, path: &mut Vec<String>, hits: &mut Vec<String>) {
        match v {
            serde_json::Value::String(s) if s.contains('@') => hits.push(path.join(".")),
            serde_json::Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    path.push(i.to_string());
                    at_signs(item, path, hits);
                    path.pop();
                }
            }
            serde_json::Value::Object(map) => {
                for (k, item) in map {
                    path.push(k.clone());
                    if k.contains('@') {
                        hits.push(path.join("."));
                    }
                    at_signs(item, path, hits);
                    path.pop();
                }
            }
            _ => {}
        }
    }

    /// **The view names no address but the reader's own** (M6.5c2 cycle D,
    /// R-6.5c2.35, R-D3): a seat is a `name`, a `you` and — to the table's
    /// CREATOR alone — the address they typed for a seat whose holder has not
    /// yet given a name; the creator is a flag and a name.
    ///
    /// Three readers of one table, because the rule is per reader: the
    /// creator sees the address they typed, the invited seat sees itself as
    /// `you` and no address at all, and a stranger sees no address either.
    /// A human seat's KEY SET is pinned too, so a seat that re-gains `user`
    /// fails here by name and not only by the `@` search.
    #[test]
    fn the_view_names_no_address_but_the_readers_own() {
        use serde_json::{json, Value};
        let mut d = open_doc();
        d.created_by = "red@wire.test".into();
        d.seats = vec![
            SeatDoc::Human { user: "red@wire.test".into(), name: None, bound_at_create: true },
            SeatDoc::Human { user: "blue@wire.test".into(), name: None, bound_at_create: true },
            SeatDoc::Open,
        ];
        let red = User::parse("red@wire.test").expect("an email");
        let blue = User::parse("blue@wire.test").expect("an email");
        let grey = User::parse("grey@wire.test").expect("an email");
        let view = |d: &TableDoc, u: &User| serde_json::to_value(d.view(u, false)).expect("a view");
        let hits = |v: &Value| {
            let mut hits = Vec::new();
            at_signs(v, &mut Vec::new(), &mut hits);
            hits
        };

        let theirs = view(&d, &red);
        assert_eq!(theirs["mine"], true, "the creator's view: {theirs}");
        assert_eq!(theirs["seats"][1]["invited"], "blue@wire.test", "the address the creator typed");
        assert_eq!(theirs["seats"][1]["you"], false);
        assert_eq!(theirs["seats"][0]["you"], true, "the creator's own seat");
        assert_eq!(theirs["creator"], Value::Null, "the creator has no name yet");
        assert_eq!(theirs.get("created_by"), None, "the creator crosses as a flag and a name: {theirs}");
        for path in hits(&theirs) {
            assert!(
                path.starts_with("seats.") && path.ends_with(".invited"),
                "the creator is handed an address outside an `invited` seat, at {path}: {theirs}",
            );
        }
        let keys = |seat: &Value| {
            let mut keys: Vec<String> = seat.as_object().expect("a seat").keys().cloned().collect();
            keys.sort();
            keys
        };
        assert_eq!(keys(&theirs["seats"][1]), ["invited", "kind", "name", "you"], "a human seat's keys");

        let invited = view(&d, &blue);
        assert_eq!(invited["seats"][1]["you"], true, "the invited seat is `you` to its holder");
        assert_eq!(invited["mine"], false);
        for i in 0..2 {
            assert_eq!(invited["seats"][i]["invited"], Value::Null, "seat {i}: `invited` is the creator's alone");
        }
        assert_eq!(hits(&invited), Vec::<String>::new(), "no address reaches the invited seat: {invited}");

        let stranger = view(&d, &grey);
        assert_eq!(stranger["mine"], false);
        assert_eq!(stranger["seats"][0], json!({ "kind": "human", "name": null, "you": false, "invited": null }));
        assert_eq!(hits(&stranger), Vec::<String>::new(), "no address reaches a stranger: {stranger}");

        d.seats[0] = SeatDoc::Human { user: "red@wire.test".into(), name: Some("Red".into()), bound_at_create: false };
        for u in [&red, &blue, &grey] {
            assert_eq!(view(&d, u)["creator"], "Red", "the creator's seat name, to every reader");
        }
        assert_eq!(view(&d, &red)["seats"][0]["invited"], Value::Null, "a named seat is its name, even to the creator");
    }

    /// **`invited` is the address the creator TYPED, and only that** (finding
    /// B7): a joiner's seat is stored under the joiner's own key the same way,
    /// and until this cycle a joiner who gave no name was shown to the creator
    /// by address — in `…/table`, the lobby row, the deal and every keep
    /// reply. Three seats, one reader (the creator): the seat they typed for
    /// somebody else shows its address; the seat they typed for THEMSELVES
    /// shows their own (Review Focus 6); the joined seat shows `null`.
    #[test]
    fn invited_names_a_creator_bound_seat_and_never_a_joiner() {
        let creator = User::parse("alice@t.test").unwrap();
        let mut doc = open_doc();
        doc.created_by = "alice@t.test".into();
        doc.seats = vec![
            SeatDoc::Human { user: "alice@t.test".into(), name: None, bound_at_create: true },
            SeatDoc::Human { user: "carol.secret@t.test".into(), name: None, bound_at_create: false },
            SeatDoc::Human { user: "typed@t.test".into(), name: None, bound_at_create: true },
        ];
        let view = doc.view(&creator, false);
        let invited: Vec<Option<String>> = view
            .seats
            .iter()
            .map(|s| match s {
                SeatView::Human { invited, .. } => invited.clone(),
                _ => None,
            })
            .collect();
        assert_eq!(
            invited,
            vec![Some("alice@t.test".into()), None, Some("typed@t.test".into())],
            "the creator's own typed seat and the seat they typed for another; never the joiner's",
        );
        // The document's bytes: a joined seat serializes as it always did —
        // the field is skipped when false — and a creator-bound seat names it.
        let json = serde_json::to_string(&doc.seats[1]).unwrap();
        assert!(!json.contains("bound_at_create"), "a joined seat's bytes are unchanged: {json}");
        let json = serde_json::to_string(&doc.seats[0]).unwrap();
        assert!(json.contains("\"bound_at_create\":true"), "{json}");
        // And a document written before the field reads every seat as joined.
        let old: SeatDoc = serde_json::from_str(r#"{"kind":"human","user":"x@t.test"}"#).unwrap();
        assert!(matches!(old, SeatDoc::Human { bound_at_create: false, .. }));
    }

    /// The view carries the BOOLEAN and never the list: who else is watching is
    /// nobody's business, and the answer is per reader.
    #[test]
    fn the_view_carries_can_watch_per_reader_and_never_the_unlocked_list() {
        let mut d = open_doc();
        d.lock = Some(Lock::new("sesame"));
        d.unlocked = vec!["grey@t.test".to_string()];
        let green = User::parse("green@t.test").expect("an email");
        let grey = User::parse("grey@t.test").expect("an email");

        let refused = serde_json::to_value(d.view(&green, false)).expect("a view");
        assert_eq!(refused["can_watch"], false);
        let admitted = serde_json::to_value(d.view(&grey, false)).expect("a view");
        assert_eq!(admitted["can_watch"], true, "two readers, two answers, one table");

        // By KEY and by VALUE, the pair the M6.2 password cell above uses: a
        // list re-served under another name passes the key search alone.
        let text = refused.to_string();
        assert!(!text.contains("unlocked"), "the list must not reach the view: {text}");
        assert!(!text.contains("grey@t.test"), "nor a watcher's email: {text}");
    }
}
