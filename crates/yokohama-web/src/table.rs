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

use serde::{Deserialize, Serialize};
use yokohama_core::{PlayerColor, PlayerId};

use crate::client::HistoricalFrame;
use crate::tables::DealCard;
use crate::identity::User;

/// Who plays one seat.
///
/// A human seat carries the NORMALIZED email `identity::User::parse` produced —
/// the one key every binding is written under, so a seat bound from a header
/// reading `  Red@Wire.TEST ` is the same seat the next request finds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeatDoc {
    Human {
        user: String,
        /// What this seat is CALLED on screen (M6.2). `None` on every seat a
        /// schema-1 document bound, and on a seat whose holder gave no name —
        /// the client falls back to the email, which is why this is optional
        /// rather than defaulted to something here.
        #[serde(default)]
        name: Option<String>,
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnMirror {
    pub to_move: u8,
    pub over: bool,
    /// The ENGINE's winner once `over` — the same value `/result` projects as
    /// `Placing.won`, never `standings[0]` (rank order; `outcome.rs:71`).
    #[serde(default)]
    pub winner: Option<u8>,
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
    /// and may therefore WATCH it (M6.3, R-3.5). Normalized emails, as every
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

    pub fn is_creator(&self, user: &User) -> bool {
        self.created_by == user.as_str()
    }

    /// What a SEATED caller is handed. See the module doc for the two fields
    /// that do not cross.
    pub fn view(&self, user: &User) -> TableView {
        TableView {
            id: self.id.clone(),
            name: self.name.clone(),
            created_by: self.created_by.clone(),
            created_at: self.created_at,
            players: self.players,
            colors: self.colors.clone(),
            start_seat: self.start_seat,
            seats: self.seats.clone(),
            chosen: self
                .seats
                .iter()
                .zip(&self.keeps)
                .map(|(s, k)| matches!(s, SeatDoc::Bot) || k.is_some())
                .collect(),
            my_seats: self.seats_of(user),
            status: self.status,
            to_move: self.turn.map(|t| PlayerId::new(t.to_move)),
            over: self.turn.map(|t| t.over),
            locked: self.lock.is_some(),
            winner: self.turn.and_then(|t| t.winner).map(PlayerId::new),
            can_watch: self.can_watch(user),
        }
    }
}

/// The table on the wire.
///
/// **Other seats' EMAILS are here, and they are readable by ANY signed-in
/// caller at an OPEN table since M6.2 (R-2.7) and at a PLAYING or finished one
/// since M6.3 (R-3.13).** They reach the view because the seats were invited by
/// email and a table whose other seats are anonymous cannot say whose turn it
/// is; since M6.2 a seat may also carry the display NAME its holder gave, which
/// is what the screen prefers, and the email stays because a name is chosen,
/// not unique, and two people at one table may pick the same one.
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
/// three-and-now-four: the `seed`, the `keeps`, the `lock`, and M6.3's
/// `unlocked` list.
#[derive(Debug, Clone, Serialize)]
pub struct TableView {
    pub id: String,
    pub name: Option<String>,
    pub created_by: String,
    pub created_at: u64,
    pub players: u8,
    pub colors: Vec<PlayerColor>,
    pub start_seat: u8,
    pub seats: Vec<SeatDoc>,
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
                SeatDoc::Human { user: "a@t.test".into(), name: None },
                SeatDoc::Human { user: "b@t.test".into(), name: None },
                SeatDoc::Human { user: "a@t.test".into(), name: None },
            ],
            keeps: vec![None, None, None],
            seed: 7,
            status: TableStatus::Dealing,
            turn: None,
            lock: None,
            unlocked: Vec::new(),
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
        let v = serde_json::to_value(d.view(&User::parse("a@t.test").unwrap())).unwrap();
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
                SeatDoc::Human { user: "red@t.test".into(), name: Some("Red".into()) },
                SeatDoc::Open,
                SeatDoc::Open,
            ],
            keeps: vec![None, None, None],
            seed: 1,
            status: TableStatus::Open,
            turn: None,
            lock: None,
            unlocked: Vec::new(),
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
        assert_eq!(doc.turn.expect("a mirror").winner, None);
        assert!(!doc.is_full() || doc.open_seats().is_empty());
    }

    #[test]
    fn open_seats_and_is_full_read_the_seats() {
        let d = open_doc();
        assert_eq!(d.open_seats(), vec![PlayerId::new(1), PlayerId::new(2)]);
        assert!(!d.is_full());
        let mut full = d.clone();
        full.seats[1] = SeatDoc::Bot;
        full.seats[2] = SeatDoc::Human { user: "b@t.test".into(), name: None };
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
        d.turn = Some(TurnMirror { to_move: 1, over: true, winner: Some(2) });
        let v = serde_json::to_value(d.view(&User::parse("red@t.test").unwrap())).unwrap();
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

    /// The view carries the BOOLEAN and never the list: who else is watching is
    /// nobody's business, and the answer is per reader.
    #[test]
    fn the_view_carries_can_watch_per_reader_and_never_the_unlocked_list() {
        let mut d = open_doc();
        d.lock = Some(Lock::new("sesame"));
        d.unlocked = vec!["grey@t.test".to_string()];
        let green = User::parse("green@t.test").expect("an email");
        let grey = User::parse("grey@t.test").expect("an email");

        let refused = serde_json::to_value(d.view(&green)).expect("a view");
        assert_eq!(refused["can_watch"], false);
        let admitted = serde_json::to_value(d.view(&grey)).expect("a view");
        assert_eq!(admitted["can_watch"], true, "two readers, two answers, one table");

        // By KEY and by VALUE, the pair the M6.2 password cell above uses: a
        // list re-served under another name passes the key search alone.
        let text = refused.to_string();
        assert!(!text.contains("unlocked"), "the list must not reach the view: {text}");
        assert!(!text.contains("grey@t.test"), "nor a watcher's email: {text}");
    }
}
