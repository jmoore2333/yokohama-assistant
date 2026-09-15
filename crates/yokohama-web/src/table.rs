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
//! **Two things never leave this document.** The setup `seed` — hidden-set
//! member #1 (`game/observe.rs`), which regenerates both decks, the boxed pile,
//! the board and every deal — and the `keeps`, which are what the OTHER seats
//! chose. [`TableView`] is what a seated caller is handed instead, and it
//! carries `chosen` (a boolean per seat) in the keeps' place: whether a seat has
//! answered is public at any table; which card it took is not.

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
    Human { user: String },
    /// The shipped production contestant, as the CLI seats it
    /// (`bot::PROVISIONAL_DEFAULT_BOT`). WHICH contestant is not stored here:
    /// the record's own `SeatProvenance` carries it once the game exists, and a
    /// second statement of it would be a second thing to keep true.
    Bot,
}

/// Dealing, or playing. **"Over" is not a status** — it is read from the
/// session, because a finished game is a property of the record and storing it
/// here would be a copy that can go stale.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TableStatus {
    Dealing,
    Playing,
}

/// Two facts of the session, MIRRORED here so the lobby can be built from the
/// table documents alone (R-plan-3).
///
/// **The record is the truth and this is a cache of two integers off it.**
/// `Games::persist` refreshes it after every applied move and IGNORES a conflict
/// doing so: a lost mirror write costs a list row that says whose turn it was
/// one move ago, and nothing else — no play route reads this. Listing twenty
/// games by rebuilding twenty sessions is what it exists to avoid.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnMirror {
    pub to_move: u8,
    pub over: bool,
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
}

impl TableDoc {
    /// Every seat this user holds, lowest first. Empty for a stranger, which is
    /// the 403 on every route that needs a seat.
    pub fn seats_of(&self, user: &User) -> Vec<PlayerId> {
        self.seats
            .iter()
            .enumerate()
            .filter(|(_, s)| matches!(s, SeatDoc::Human { user: u } if u == user.as_str()))
            .map(|(i, _)| PlayerId::new(i as u8))
            .collect()
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
                SeatDoc::Human { user } => Some(user.as_str()),
                SeatDoc::Bot => None,
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
        }
    }
}

/// The table on the wire.
///
/// Other seats' EMAILS are here, to seated players only: they were invited by
/// email, and a table whose other seats are anonymous cannot say whose turn it
/// is. A display name is M6.2's.
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
pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

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
                SeatDoc::Human { user: "a@t.test".into() },
                SeatDoc::Human { user: "b@t.test".into() },
                SeatDoc::Human { user: "a@t.test".into() },
            ],
            keeps: vec![None, None, None],
            seed: 7,
            status: TableStatus::Dealing,
            turn: None,
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
}
