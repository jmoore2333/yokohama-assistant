//! **The chat documents (M6.4, spec §2).** One per table (`<id>.chat.json`)
//! and one for the house (`_house.chat.json`): an append-only list whose
//! numbers start at 1 and are never reused, trimmed to a configured keep from
//! the OLDEST end, never in the record and never undone (R-6.4.9).
//!
//! **Every stored field is an INPUT** (the M6.1 lesson): [`check_shape`] runs on
//! read — as the table document's does — AND on write, which the table's does
//! not, because this process appends to a chat document on every message and a
//! malformed author written once would poison the next read under the table's
//! lock. A seat is bounded by the table's player count not because anything
//! here hands it to `PlayerId::new` (nothing does) but because the client
//! indexes `colors[seat]` with it, and an input is bounded where it enters. The
//! CLOCK is bounded for the same reason and the same way ([`MAX_CREATED_AT`]):
//! the client builds a `Date` from it, and a `Date` out of range throws out of
//! the panel's render rather than drawing a wrong time.
//!
//! Two things are deliberately NOT refused on read (R-6.4.8): a text longer than
//! the configured cap (served as stored) and a document longer than the
//! configured keep (trimmed on its next append), so lowering either number on
//! a running volume cannot take a table out.
//!
//! **A message can be DELETED since M6.5c2 cycle D (R-D4), and a delete leaves
//! a TOMBSTONE** — the message's number, clock and author, its text emptied
//! and `deleted` set ([`ChatDoc::delete`]) — so `last` and every cursor a
//! client holds mean what they meant, and a number is still never reused.
//! [`ChatDoc::rev`] is the document's REVISION: it moves on every append and
//! every delete, where `last` moves on an append alone, so a reader that
//! compares the two can tell that a message it already holds has changed.

use serde::{Deserialize, Serialize};

use crate::identity::User;

/// This document's own shape version, unrelated to `SEMANTICS_REV`.
pub const CHAT_SCHEMA: u32 = 1;

/// The newest clock this server will store or serve: 9999-12-31T23:59:59Z.
///
/// **A ceiling, not a calendar.** `created_at` is a stored field and therefore
/// an INPUT, and it is the one the client hands straight to
/// `new Date(created_at * 1000).toISOString()` — which THROWS above 8.64e15 ms
/// and takes the panel's whole render down with it, for every browser at that
/// table. The number here is far under that limit on purpose: a value past the
/// year 9999 is a hand-edit or a half-finished restore, never a clock, and
/// refusing it costs a message nobody typed.
pub const MAX_CREATED_AT: u64 = 253_402_300_799;

/// The one id the house document answers to.
///
/// One house, one id: a second name would be a second file nothing lists and
/// nothing deletes, so `store` refuses one on both implementations rather than
/// leaving `MemStore` — where `is_safe_id("house2")` is perfectly true — to
/// keep a document `FsStore` would never have written.
pub const HOUSE: &str = "house";

/// A tag's length in hex characters: four reads at a glance beside a name, and 65,536 values keep two posters in
/// one room apart in all but a chance collision.
pub const TAG_HEX: usize = 4;

/// **The tag a watcher's or house poster's key reads as in ONE room** (security S2c, finding D8; the owner's R-2):
/// the first four hex of SHA-256 over the room's salt, a newline and the key. Stable within the room, unlinkable
/// across rooms (another salt), and not invertible to an address: the salt is the room's and is never served, so
/// a stranger cannot hash a guessed address and confirm it (B7's fence). Under the open door it says "the same key
/// as before" and nothing more — a name is a claim there by design, the owner's accepted risk (2026-10-13).
pub fn tag_of(salt: &str, key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(salt.as_bytes());
    h.update(b"\n");
    h.update(key.as_bytes());
    let digest = h.finalize();
    digest.iter().take(TAG_HEX / 2).map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDoc {
    pub schema: u32,
    pub id: String,
    /// The newest number ever assigned — NOT `messages.len()`, and not the
    /// newest retained: a trim drops messages and leaves this alone, so a
    /// client's `after` cursor stays valid across it. A delete leaves it alone
    /// too: a tombstone keeps its number.
    pub last: u64,
    /// The document's REVISION (M6.5c2 cycle D, R-D4): +1 per append AND per
    /// delete, never by a trim. `last` counts the messages a reader has not
    /// fetched; this counts every change to what it may already hold, so
    /// `rev - last` moves exactly when a held message was deleted.
    ///
    /// `#[serde(default)]`: a document written before the delete reads as
    /// revision 0 and moves from there — it is compared for CHANGE and never
    /// for its value, so the starting point does not matter.
    #[serde(default)]
    pub rev: u64,
    /// **The room's salt** (security S2c, finding D8; decision D-5): what a watcher's or house poster's tag is
    /// derived from, with their key. Given on the document's first append after this cycle by `Games::chat_salt`;
    /// `None` on a document written before, until then. Never served: the tag is served, and it is one way.
    #[serde(default)]
    pub salt: Option<String>,
    pub messages: Vec<Message>,
}

/// The two numbers a signal carries about one chat document (M6.5c2 cycle D,
/// R-D4): [`ChatDoc::last`], a count for the text, and [`ChatDoc::rev`], a
/// revision for the shape. `Events.chat` and `Events.chat_rev` are these two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChatCounts {
    pub last: u64,
    pub rev: u64,
}

/// Why a delete was refused (M6.5c2 cycle D, R-D4), each worded by
/// [`std::fmt::Display`] for the route to carry verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRefusal {
    /// The document holds no message numbered this — one never assigned, or
    /// one the trim has already dropped. A 404: there is nothing here to
    /// delete, and a tombstone is something.
    NotFound(u64),
    /// The caller is neither the message's author nor the table's creator. A
    /// 403, and asked BEFORE whether the message is already a tombstone: the
    /// tombstone keeps its author, so the answer to a stranger is the same
    /// before and after the author deleted it.
    NotYours,
}

impl std::fmt::Display for ChatRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(seq) => write!(f, "no message {seq} in this chat"),
            Self::NotYours => f.write_str("only the message's author or the table's creator may delete it"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub seq: u64,
    /// Unix seconds. **Named `created_at` because the corpus freezer pins a
    /// clock by KEY NAME** (`tests/wire_fixtures.rs`); a field spelled `at`
    /// would churn fixtures on every regeneration.
    pub created_at: u64,
    pub author: Author,
    /// The empty string once [`Message::deleted`] is set, and never otherwise
    /// ([`check_shape`]).
    pub text: String,
    /// A TOMBSTONE (M6.5c2 cycle D, R-D4): the message was deleted by its
    /// author or the table's creator, and its number, clock and author stay.
    /// `#[serde(default)]`, so a document written before the delete reads as
    /// it did.
    #[serde(default)]
    pub deleted: bool,
}

/// Who said it, as STORED — the address in `user` is the key `you` is read
/// from, and never crosses the wire ([`AuthorView`] does).
///
/// **Tagged, the `SeatDoc` shape, so an arm can be added additively on both
/// sides of the wire (R-6.4.15).** `Bot` is read and written and PRODUCED BY NO
/// ROUTE in this cycle — it is here so the arm that arrives later does not
/// arrive as a shape change.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Author {
    Seat {
        seat: u8,
        user: String,
        #[serde(default)]
        name: Option<String>,
    },
    Watcher {
        user: String,
        #[serde(default)]
        name: Option<String>,
    },
    Bot {
        seat: u8,
        #[serde(default)]
        name: Option<String>,
    },
}

/// Who said it, as a READER is handed it (M6.5c2 cycle D, R-6.5c2.35, R-D3):
/// the [`Author`] arms with the address taken out and `you` put in.
///
/// **The address stays in the document and never crosses.** Every message
/// used to carry its poster's `user` to every reader — the house's to anyone
/// signed in — so a room was a roster of addresses. `you` is the one fact a
/// reader needs about an author's identity: whether it is them. A poster
/// with no name is drawn as `you`, or as its seat number in the seat's
/// colour, or as "someone" for a watcher, by the client.
///
/// Tagged with the document's arm names, so an arm added to [`Author`] is
/// added here in the same words.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthorView {
    Seat { seat: u8, name: Option<String>, you: bool },
    Watcher { name: Option<String>, you: bool, tag: Option<String> },
    /// A bot is nobody's `you`, so the arm carries no such field.
    Bot { seat: u8, name: Option<String> },
}

/// One message as a READER is handed it: [`Message`] with its author as an
/// [`AuthorView`].
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MessageView {
    pub seq: u64,
    /// Unix seconds — the same key [`Message::created_at`] is named for.
    pub created_at: u64,
    pub author: AuthorView,
    /// Empty on a tombstone.
    pub text: String,
    /// The message was deleted ([`Message::deleted`]): drawn as a removed
    /// message in its author's row, never skipped, so the numbers a reader
    /// holds stay contiguous with what the room said.
    pub deleted: bool,
}

impl Message {
    /// This message as `reader` is handed it — `you` read off the STORED
    /// address, which is normalized on read ([`normalized`]) and compared
    /// with the reader's normalized key.
    pub fn view_for(&self, reader: &User, salt: Option<&str>) -> MessageView {
        let you = |user: &String| user == reader.as_str();
        let author = match &self.author {
            Author::Seat { seat, user, name } => AuthorView::Seat { seat: *seat, name: name.clone(), you: you(user) },
            Author::Watcher { user, name } => AuthorView::Watcher { name: name.clone(), you: you(user), tag: salt.map(|s| tag_of(s, user)) },
            Author::Bot { seat, name } => AuthorView::Bot { seat: *seat, name: name.clone() },
        };
        MessageView {
            seq: self.seq,
            created_at: self.created_at,
            author,
            text: self.text.clone(),
            deleted: self.deleted,
        }
    }
}

/// The two retention numbers (R-6.4.8), from the launch variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatLimits {
    /// Messages retained per document; `None` is `all`.
    pub keep: Option<usize>,
    /// Characters per message.
    pub text: usize,
}

impl Default for ChatLimits {
    fn default() -> Self {
        Self { keep: Some(500), text: 500 }
    }
}

/// The variable that turns the house chat off (R-6.4.6).
pub const LOBBY_VAR: &str = "YOKOHAMA_WEB_LOBBY_CHAT";
/// The variable that sets how many messages a document retains (R-6.4.8).
pub const KEEP_VAR: &str = "YOKOHAMA_WEB_CHAT_KEEP";
/// The variable that sets how long one message may be (R-6.4.8).
pub const TEXT_VAR: &str = "YOKOHAMA_WEB_CHAT_TEXT";

/// The three launch variables (R-6.4.6, R-6.4.8). **Each HAS a default**, unlike
/// the identity mode: a missing switch is not a missing door.
///
/// Which is the whole of why this does not copy `IdentityMode::from_env`'s
/// refusal on absence. An identity source is a decision about who may act, so a
/// server that was not told refuses to start; a retention number is a decision
/// about how much is kept, and there is a sane answer nobody has to write down.
/// A BAD value is refused either way, and by the variable's own name — a server
/// that started with `YOKOHAMA_WEB_CHAT_KEEP=lots` silently meaning 500 would
/// be a server whose operator believes a number that is not in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatConfig {
    /// Whether the house chat is on ([`LOBBY_VAR`], default on). Off, its two
    /// routes answer 404 and `/api/health` says so, so a client draws no panel.
    pub lobby: bool,
    /// The retention numbers every chat document is kept to ([`KEEP_VAR`],
    /// [`TEXT_VAR`]).
    pub limits: ChatLimits,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self { lobby: true, limits: ChatLimits::default() }
    }
}

#[cfg(feature = "server")]
impl ChatConfig {
    /// The three reads, `server`-only since M6.5a: a host with no process
    /// environment builds a [`ChatConfig`] itself and never asks.
    pub fn from_env() -> Result<Self, String> {
        Self::from_env_values(
            Self::value_of(LOBBY_VAR, std::env::var(LOBBY_VAR))?.as_deref(),
            Self::value_of(KEEP_VAR, std::env::var(KEEP_VAR))?.as_deref(),
            Self::value_of(TEXT_VAR, std::env::var(TEXT_VAR))?.as_deref(),
        )
    }

    /// One variable's read: unset is `None`, and a value that is not UTF-8 is
    /// REFUSED by the variable's name (M6.5c2 cycle D, row 130).
    ///
    /// It was `.ok()`, which folded the second case into the first: an
    /// operator who set a stray byte ran the default and was never told —
    /// the one outcome refusing a bad value by name exists to prevent.
    fn value_of(name: &str, read: Result<String, std::env::VarError>) -> Result<Option<String>, String> {
        match read {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(raw)) => Err(format!("{name} is not UTF-8 — got {raw:?}")),
        }
    }
}
// ---- end server ----

impl ChatConfig {
    /// The read split from the parse, the identity extractor's shape, so the
    /// unset case is testable without touching the process environment.
    ///
    /// **`0` is refused for both numbers, and that is a rule rather than
    /// tidiness**: `ChatDoc::trim(Some(0))` would drop every message the
    /// instant it was appended, and a text cap of zero would refuse every
    /// message ever typed. Both are chats that work and say nothing, which is
    /// worse than a server that will not start.
    pub fn from_env_values(
        lobby: Option<&str>,
        keep: Option<&str>,
        text: Option<&str>,
    ) -> Result<Self, String> {
        // Compared case-insensitively and quoted AS TYPED (M6.5c2 cycle D, row
        // 130), trimmed, as the two numbers below are: an operator who wrote
        // `Maybe` is shown `Maybe`, not a lowercased copy they never typed.
        let lobby = match lobby.map(str::trim) {
            None => true,
            Some(s) if s.eq_ignore_ascii_case("on") => true,
            Some(s) if s.eq_ignore_ascii_case("off") => false,
            Some(other) => return Err(format!("{LOBBY_VAR} must be on or off — got {other:?}")),
        };
        // **The absent arms take their values from `ChatLimits::default()` and
        // never restate one.** A second copy of a default is a number that can
        // move in one place, and the way it would have gone unnoticed is worth
        // stating exactly: moving `ChatLimits::default()` turns the health
        // cell's hardcoded 500 red at once, the developer fixes THAT number,
        // the suite returns to green — and this parse arm, restating the old
        // one, survives unremarked, so a server launched with no variables set
        // runs a retention nothing in the suite is looking at. The pin below
        // (`setting nothing IS Default`) is what now catches that second step.
        let defaults = ChatLimits::default();
        let keep = match keep.map(str::trim) {
            None => defaults.keep,
            // `all` is read the way `on`/`off` are — the three variables are
            // one operator's `docker run` line, and a rule that held for two of
            // them and not the third would be a rule nobody could remember.
            Some(s) if s.eq_ignore_ascii_case("all") => None,
            Some(s) => match s.parse::<usize>() {
                Ok(n) if n > 0 => Some(n),
                _ => {
                    return Err(format!(
                        "{KEEP_VAR} must be a positive number of messages or `all` — got {s:?}",
                    ))
                }
            },
        };
        let text = match text.map(str::trim) {
            None => defaults.text,
            Some(s) => match s.parse::<usize>() {
                Ok(n) if n > 0 => n,
                _ => {
                    return Err(format!(
                        "{TEXT_VAR} must be a positive number of characters — got {s:?}",
                    ))
                }
            },
        };
        Ok(Self { lobby, limits: ChatLimits { keep, text } })
    }
}

impl ChatDoc {
    pub fn empty(id: &str) -> Self {
        Self { schema: CHAT_SCHEMA, id: id.to_string(), last: 0, rev: 0, salt: None, messages: Vec::new() }
    }

    /// The two numbers a signal carries about this document — see
    /// [`ChatCounts`].
    pub fn counts(&self) -> ChatCounts {
        ChatCounts { last: self.last, rev: self.rev }
    }

    /// Delete message `seq` as `by` (M6.5c2 cycle D, R-D4): the message
    /// becomes a TOMBSTONE — its text emptied, `deleted` set, its number,
    /// clock and author kept — and [`ChatDoc::rev`] moves; `last` does not.
    ///
    /// `creator` is the table's STORED `created_by`, or `None` at the house,
    /// which nobody created: there only the author may delete.
    ///
    /// **The order is the rule.** A number the document does not hold is
    /// [`ChatRefusal::NotFound`] — never assigned, or trimmed, and a trimmed
    /// message took its author with it, so nothing could decide a 403 for it.
    /// Then the caller's standing: the author (a seat's or a watcher's stored
    /// address, normalized on read as `by` is), or the creator; a bot's
    /// message has no author who can ask. Only THEN whether it is already a
    /// tombstone — so a second delete by someone entitled is an idempotent
    /// `Ok` that moves nothing, and a stranger is refused alike before and
    /// after.
    pub fn delete(&mut self, seq: u64, by: &User, creator: Option<&str>) -> Result<(), ChatRefusal> {
        let message = self.messages.iter_mut().find(|m| m.seq == seq).ok_or(ChatRefusal::NotFound(seq))?;
        let author = match &message.author {
            Author::Seat { user, .. } | Author::Watcher { user, .. } => Some(user.as_str()),
            Author::Bot { .. } => None,
        };
        if author != Some(by.as_str()) && creator != Some(by.as_str()) {
            return Err(ChatRefusal::NotYours);
        }
        if message.deleted {
            return Ok(());
        }
        message.deleted = true;
        message.text.clear();
        self.rev += 1;
        Ok(())
    }

    /// Drop the OLDEST until `keep` remain; `last` is untouched, so a number is
    /// never reused and a cursor taken before the trim still means what it did.
    ///
    /// **Total over every `keep`, zero included** (M6.5c2 cycle D, row 130):
    /// the launch variables refuse `0`, but [`ChatLimits`] is a public type a
    /// host builds for itself, and `Some(0)` keeps nothing and panics nowhere.
    pub fn trim(&mut self, keep: Option<usize>) {
        if let Some(k) = keep {
            if self.messages.len() > k {
                let drop = self.messages.len() - k;
                self.messages.drain(..drop);
            }
        }
    }
}

impl ChatDoc {
    /// Every retained message, as `reader` may be handed it — see
    /// [`MessageView`]. A caller that wants a page from a cursor filters
    /// `messages` first.
    pub fn page_for(&self, reader: &User) -> Vec<MessageView> {
        self.messages.iter().map(|m| m.view_for(reader, self.salt.as_deref())).collect()
    }
}

/// The text rule, applied on POST by the route and again by the appender.
///
/// Trimmed and returned, so `" hi "` and `"hi"` are one message rather than two
/// that only look alike.
pub fn check_text(raw: &str, limits: &ChatLimits) -> Result<String, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("a message needs some words".to_string());
    }
    if text.chars().count() > limits.text {
        return Err(format!("a message is at most {} characters", limits.text));
    }
    if crate::text::invisible(text).is_some() {
        return Err(format!("a message is one line, with {}", crate::text::INVISIBLE_RULE));
    }
    Ok(text.to_string())
}

/// The shape rule, on read AND on write.
///
/// `players` is the table's count, or `None` at the house, where no author
/// holds a seat. Length is deliberately not a refusal here — see the module
/// doc. The CLOCK is, and for the reason the seat bound is: the client renders
/// it, and an input is bounded where it enters ([`MAX_CREATED_AT`]).
pub fn check_shape(doc: &ChatDoc, players: Option<u8>) -> Result<(), String> {
    if doc.schema > CHAT_SCHEMA {
        return Err(format!(
            "schema {} — this server reads up to {CHAT_SCHEMA}; a newer server wrote it",
            doc.schema,
        ));
    }
    let mut prev = 0u64;
    for (i, m) in doc.messages.iter().enumerate() {
        if m.seq <= prev {
            return Err(format!(
                "message {i} has seq {} after {prev}: numbers must increase",
                m.seq,
            ));
        }
        prev = m.seq;
        // A TOMBSTONE carries no text and nothing else does (M6.5c2 cycle D,
        // R-D4): a deleted message that still said something would serve the
        // words its author took down.
        //
        // BLANK as well as empty (M6.5c2 cycle D, row 130): the appender
        // trims, so no text this server wrote is only whitespace, and one a
        // hand-edited or restored document carries is refused as the route
        // refuses it — by the same trim.
        if m.deleted {
            if !m.text.is_empty() {
                return Err(format!("message {i} is deleted and still carries text"));
            }
        } else if m.text.trim().is_empty() || m.text.chars().any(char::is_control) {
            return Err(format!("message {i}'s text is empty, blank or carries a control character"));
        }
        // The clock, bounded here because the client renders it: see
        // [`MAX_CREATED_AT`].
        if m.created_at > MAX_CREATED_AT {
            return Err(format!(
                "message {i}'s clock {} is past the year 9999",
                m.created_at,
            ));
        }
        let (user, name, seat) = match &m.author {
            Author::Seat { seat, user, name } => (Some(user), name.as_deref(), Some(*seat)),
            Author::Watcher { user, name } => (Some(user), name.as_deref(), None),
            Author::Bot { seat, name } => (None, name.as_deref(), Some(*seat)),
        };
        if let Some(u) = user {
            if User::parse(u).is_err() {
                return Err(format!(
                    "message {i}'s author {u:?} is neither an email nor a name",
                ));
            }
        }
        if let Some(n) = name {
            crate::tables::check_stored_name(n).map_err(|why| format!("message {i}'s author name: {why}"))?;
        }
        match (seat, players) {
            (Some(s), Some(p)) if s >= p => {
                return Err(format!("message {i} names seat {s} at a {p}-seat table"))
            }
            (Some(s), None) => {
                return Err(format!("message {i} names seat {s} in the house, which has no seats"))
            }
            _ => {}
        }
    }
    if doc.last < prev {
        return Err(format!("last {} is behind the newest message {prev}", doc.last));
    }
    Ok(())
}

/// `doc` with every author in the form [`check_shape`] parses it into — the
/// address as `User::parse` keys it (trimmed, lowercased) and the name as
/// `tables::check_stored_name` reads it (trimmed; the READ rule, never the
/// write rule's `check_name`, which would leave a pre-S2c name padded) — for a
/// document that has PASSED the shape rule (M6.5c2 cycle D, row 130).
///
/// The rule used to parse both and throw the results away, so a document
/// written by hand or brought back by a restore served `" RED@T.test "` beside
/// a seat keyed `red@t.test`. Normalizing on read changes nothing this server
/// wrote — every writer stores these forms already — and refuses nothing the
/// rule admits: a value that did not parse was refused by the rule first, and
/// is left as it is here rather than unwrapped.
pub fn normalized(mut doc: ChatDoc) -> ChatDoc {
    for m in &mut doc.messages {
        let (user, name) = match &mut m.author {
            Author::Seat { user, name, .. } | Author::Watcher { user, name } => (Some(user), name),
            Author::Bot { name, .. } => (None, name),
        };
        if let Some(user) = user {
            if let Ok(key) = User::parse(user) {
                *user = key.as_str().to_string();
            }
        }
        if let Some(n) = name {
            if let Ok(stored) = crate::tables::check_stored_name(n) {
                *n = stored;
            }
        }
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seat(seq: u64, s: u8, text: &str) -> Message {
        Message {
            seq,
            created_at: 1,
            author: Author::Seat { seat: s, user: "red@t.test".into(), name: None },
            text: text.into(),
            deleted: false,
        }
    }

    /// A document as the appender would have left it: `last` the newest
    /// number, and `rev` beside it — one revision per append.
    fn doc(msgs: Vec<Message>) -> ChatDoc {
        let last = msgs.last().map_or(0, |m| m.seq);
        ChatDoc { schema: CHAT_SCHEMA, id: "t".into(), last, rev: last, salt: None, messages: msgs }
    }

    #[test]
    fn check_text_trims_and_bounds_by_characters_not_bytes() {
        let limits = ChatLimits { keep: Some(500), text: 5 };
        assert_eq!(check_text("  héllo ", &limits).unwrap(), "héllo");
        assert!(check_text("hello!", &limits).is_err(), "six characters over a cap of five");
        assert!(check_text("   ", &limits).is_err(), "empty after trimming");
        assert!(check_text("a\nb", &limits).is_err(), "a control character");
    }

    /// **A line is visible whole, and the LENGTH rule speaks first** (security S2c, finding D8; Review Focus 3):
    /// the review's reversed line is refused with the rule's sentence; a line of exactly the cap with one zero-width
    /// space inside it is refused by the CHARACTER sentence (it is within the cap), and one character over the cap
    /// by the LENGTH sentence — so a person reads the rule they can act on. An emoji line passes; a multi-person
    /// emoji sequence (a joiner, U+200D, category Cf) does not — the accepted loss (decision D-4).
    #[test]
    fn a_line_with_an_invisible_character_is_refused_after_the_length_rule() {
        let limits = ChatLimits { text: 10, ..ChatLimits::default() };
        let rule = "a message is one line, with no control, invisible or direction-changing character".to_string();
        assert_eq!(check_text("\u{202E}emag siht", &limits), Err(rule.clone()));
        assert_eq!(check_text("gg\u{200B} resign", &limits), Err(rule.clone()));
        let at_cap = format!("{}\u{200B}", "x".repeat(9));
        assert_eq!(at_cap.chars().count(), 10, "premise: exactly the cap");
        assert_eq!(check_text(&at_cap, &limits), Err(rule.clone()), "within the cap: the character rule");
        let over = format!("{}\u{200B}", "x".repeat(10));
        assert_eq!(check_text(&over, &limits), Err("a message is at most 10 characters".to_string()), "over the cap: the length rule, first");
        assert_eq!(check_text("🙂 gg", &limits), Ok("🙂 gg".to_string()));
        assert_eq!(check_text("\u{1F468}\u{200D}\u{1F469}", &limits), Err(rule), "a joined sequence carries a format character");
    }

    #[test]
    fn the_seat_bound_is_the_table_s_player_count_at_two_three_and_four() {
        for players in [2u8, 3, 4] {
            let ok = doc(vec![seat(1, players - 1, "hi")]);
            assert!(check_shape(&ok, Some(players)).is_ok(), "{players}P: the last seat is in range");
            let bad = doc(vec![seat(1, players, "hi")]);
            assert!(
                check_shape(&bad, Some(players)).is_err(),
                "{players}P: seat {players} would reach PlayerId::new",
            );
        }
        let bot = doc(vec![Message {
            seq: 1,
            created_at: 1,
            author: Author::Bot { seat: 7, name: None },
            text: "x".into(),
            deleted: false,
        }]);
        assert!(check_shape(&bot, Some(2)).is_err(), "a bot author is bounded too");
    }

    #[test]
    fn the_house_carries_no_seat_author() {
        assert!(check_shape(&doc(vec![seat(1, 0, "hi")]), None).is_err());
        let w = Message {
            seq: 1,
            created_at: 1,
            author: Author::Watcher { user: "g@t.test".into(), name: Some("G".into()) },
            text: "hi".into(),
            deleted: false,
        };
        assert!(check_shape(&doc(vec![w]), None).is_ok());
    }

    #[test]
    fn seq_must_increase_and_last_must_cover_it() {
        let mut d = doc(vec![seat(1, 0, "a"), seat(2, 0, "b")]);
        assert!(check_shape(&d, Some(2)).is_ok());
        d.messages[1].seq = 1;
        assert!(check_shape(&d, Some(2)).is_err(), "a repeated seq");
        let mut d = doc(vec![seat(3, 0, "a")]);
        d.last = 2;
        assert!(check_shape(&d, Some(2)).is_err(), "last behind the newest");
    }

    /// **The clock is bounded too, and the reason is the client's render**
    /// (final review, Important 1).
    ///
    /// `created_at` is the last stored field the shape rule left alone, and it
    /// is handed straight to `new Date(created_at * 1000).toISOString()` in the
    /// panel — which THROWS `RangeError` above 8.64e15 ms, out of React's
    /// render, taking the whole game screen down for every browser at that
    /// table. So a hand-edited or half-restored document is refused here
    /// instead, at a ceiling far under that: the end of year 9999, which no
    /// clock this server reads will ever reach and no reader has to translate.
    ///
    /// The seconds beside it are the anti-vacuity: a rule that refused
    /// everything would pass the first half of this cell on its own.
    #[test]
    fn a_clock_after_year_9999_is_refused_and_an_ordinary_one_is_not() {
        let mut d = doc(vec![seat(1, 0, "hi")]);
        d.messages[0].created_at = MAX_CREATED_AT;
        assert!(check_shape(&d, Some(2)).is_ok(), "the last second this server will store");
        d.messages[0].created_at = MAX_CREATED_AT + 1;
        let e = check_shape(&d, Some(2)).unwrap_err();
        assert!(e.contains("clock"), "{e}");
        d.messages[0].created_at = u64::MAX;
        assert!(check_shape(&d, Some(2)).is_err(), "and the value that crashes the panel");
        d.messages[0].created_at = now_unix_ish();
        assert!(check_shape(&d, Some(2)).is_ok(), "an ordinary message still reads");
    }

    /// Roughly 2026 in Unix seconds — a value this server really could write,
    /// so the cell above is not satisfied by a rule that refuses everything.
    fn now_unix_ish() -> u64 {
        1_789_000_000
    }

    #[test]
    fn malformed_fields_are_refused_but_length_is_not() {
        let long = "x".repeat(600);
        let d = doc(vec![seat(1, 0, &long)]);
        assert!(check_shape(&d, Some(2)).is_ok(), "a stored over-cap text is SERVED (R-6.4.8)");
        let many: Vec<Message> = (1..=600).map(|i| seat(i, 0, "m")).collect();
        assert!(
            check_shape(&doc(many), Some(2)).is_ok(),
            "an over-keep document is served and trimmed later",
        );
        let mut bad_user = doc(vec![seat(1, 0, "a")]);
        bad_user.messages[0].author =
            Author::Seat { seat: 0, user: "..".into(), name: None };
        assert!(check_shape(&bad_user, Some(2)).is_err());
        // The far side (M6.5c2): a plain name is a key, so a watcher an open
        // door wrote under one is an author this document still reads.
        let mut named = doc(vec![seat(1, 0, "a")]);
        named.messages[0].author = Author::Watcher { user: "ben castle".into(), name: None };
        assert_eq!(check_shape(&named, Some(2)), Ok(()), "a plain-name watcher is an author");
        let mut bad_name = doc(vec![seat(1, 0, "a")]);
        bad_name.messages[0].author =
            Author::Seat { seat: 0, user: "r@t.test".into(), name: Some("x".repeat(65)) };
        assert!(check_shape(&bad_name, Some(2)).is_err());
        let ctl = doc(vec![seat(1, 0, "a\u{7}b")]);
        assert!(check_shape(&ctl, Some(2)).is_err(), "a control character in a stored text");
        let joined = doc(vec![seat(1, 0, "h\u{200D}i")]);
        assert_eq!(check_shape(&joined, Some(2)), Ok(()), "a stored line the old rule admitted still reads (D-4: a family emoji's joiners)");
        let mut newer = doc(vec![]);
        newer.schema = CHAT_SCHEMA + 1;
        assert!(check_shape(&newer, Some(2)).is_err());
    }

    #[test]
    fn trim_drops_the_oldest_and_keeps_last_counting() {
        let mut d = doc((1..=7).map(|i| seat(i, 0, "m")).collect());
        d.trim(Some(5));
        assert_eq!(d.messages.iter().map(|m| m.seq).collect::<Vec<_>>(), vec![3, 4, 5, 6, 7]);
        assert_eq!(d.last, 7, "the number is never reused");
        d.trim(None);
        assert_eq!(d.messages.len(), 5, "`all` trims nothing");
    }

    #[test]
    fn the_author_serializes_tagged_like_a_seat_doc() {
        let m = seat(1, 0, "hi");
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["author"]["kind"], "seat");
        assert_eq!(v["author"]["seat"], 0);
        let w: Message = serde_json::from_value(serde_json::json!({
            "seq": 2, "created_at": 1, "text": "x",
            "author": { "kind": "watcher", "user": "g@t.test", "name": null }
        }))
        .unwrap();
        assert!(matches!(w.author, Author::Watcher { .. }));
        let b: Message = serde_json::from_value(serde_json::json!({
            "seq": 3, "created_at": 1, "text": "x", "author": { "kind": "bot", "seat": 1, "name": null }
        }))
        .unwrap();
        assert!(
            matches!(b.author, Author::Bot { seat: 1, .. }),
            "the arm no route produces still reads (R-6.4.15)",
        );
    }

    /// **A page is built for its READER** (M6.5c2 cycle D, R-6.5c2.35, R-D3):
    /// `you` marks the reader's own messages and no author carries a `user` —
    /// the address every poster's message used to carry to every reader, the
    /// house's to anyone signed in. The stored document keeps the address, so
    /// `you` is still answerable.
    ///
    /// Two readers of one page, so a `you` that was constant (or keyed on the
    /// seat rather than the poster) fails one of them.
    #[test]
    fn a_page_marks_you_on_the_readers_own_messages_and_carries_no_user() {
        use serde_json::{json, Value};
        let authored =
            |seq: u64, author: Author| Message { seq, created_at: 1, author, text: "hi".into(), deleted: false };
        let d = doc(vec![
            authored(1, Author::Seat { seat: 0, user: "red@wire.test".into(), name: Some("Red".into()) }),
            authored(2, Author::Seat { seat: 1, user: "blue@wire.test".into(), name: None }),
            authored(3, Author::Watcher { user: "green@wire.test".into(), name: None }),
            authored(4, Author::Bot { seat: 1, name: Some("V4".into()) }),
        ]);
        let red = User::parse("red@wire.test").expect("an email");
        let green = User::parse("green@wire.test").expect("an email");
        let authors = |reader: &User| -> Vec<Value> {
            let page = serde_json::to_value(d.page_for(reader)).expect("a page");
            assert!(!page.to_string().contains('@'), "an address reached a reader: {page}");
            page.as_array().expect("a list").iter().map(|m| m["author"].clone()).collect()
        };
        assert_eq!(
            authors(&red),
            [
                json!({ "kind": "seat", "seat": 0, "name": "Red", "you": true }),
                json!({ "kind": "seat", "seat": 1, "name": null, "you": false }),
                json!({ "kind": "watcher", "name": null, "you": false, "tag": null }),
                json!({ "kind": "bot", "seat": 1, "name": "V4" }),
            ],
        );
        let to_green = authors(&green);
        assert_eq!(to_green[0]["you"], false, "red's message is not green's");
        assert_eq!(to_green[2], json!({ "kind": "watcher", "name": null, "you": true, "tag": null }), "green's own");
    }

    /// **Each of the three variables HAS a default, and a bad value is refused
    /// BY NAME** (R-6.4.6, R-6.4.8).
    ///
    /// The read is split from the parse, the identity extractor's shape, so the
    /// unset case is tested without mutating the process environment under a
    /// threaded runner. `0` is refused for both numbers rather than accepted:
    /// a keep of zero would discard every message the moment it was appended,
    /// and a text cap of zero would refuse every message ever typed — two ways
    /// to configure a chat that silently does nothing.
    #[test]
    fn the_three_variables_parse_with_their_defaults_and_refuse_by_name() {
        let d = ChatConfig::from_env_values(None, None, None).unwrap();
        assert_eq!((d.lobby, d.limits.keep, d.limits.text), (true, Some(500), 500));
        // The two statements of the default are BOUND, and the tuple above is
        // what keeps this from being satisfied by both drifting together.
        assert_eq!(d, ChatConfig::default(), "setting nothing IS `Default`");
        assert!(ChatConfig::from_env_values(Some("on"), None, None).unwrap().lobby);
        assert!(!ChatConfig::from_env_values(Some("off"), None, None).unwrap().lobby);
        assert!(
            !ChatConfig::from_env_values(Some(" OFF "), None, None).unwrap().lobby,
            "trimmed, case-insensitive",
        );
        let e = ChatConfig::from_env_values(Some("maybe"), None, None).unwrap_err();
        assert!(e.contains(LOBBY_VAR) && e.contains("maybe"), "{e}");
        assert_eq!(ChatConfig::from_env_values(None, Some("all"), None).unwrap().limits.keep, None);
        assert_eq!(
            ChatConfig::from_env_values(None, Some(" ALL "), None).unwrap().limits.keep,
            None,
            "`all` is read the way `on` and `off` are: trimmed, case-insensitive",
        );
        assert_eq!(
            ChatConfig::from_env_values(None, Some("20"), None).unwrap().limits.keep,
            Some(20),
        );
        for bad in ["0", "-1", "lots", ""] {
            let e = ChatConfig::from_env_values(None, Some(bad), None).unwrap_err();
            assert!(e.contains(KEEP_VAR), "{bad:?}: {e}");
        }
        assert_eq!(ChatConfig::from_env_values(None, None, Some("140")).unwrap().limits.text, 140);
        for bad in ["0", "x", "all"] {
            let e = ChatConfig::from_env_values(None, None, Some(bad)).unwrap_err();
            assert!(e.contains(TEXT_VAR), "{bad:?}: {e}");
        }
    }

    /// **Whitespace is no text — at the route, and in a stored document**
    /// (M6.5c2 cycle D, row 130) — the catcher for either rule letting a blank
    /// message through.
    ///
    /// The route's rule trims and refuses what is left empty, in its own
    /// sentence, which is unchanged (the route half is also pinned through the
    /// router by `lib.rs`'s chat-order cell). The SHAPE rule, which reads every
    /// stored document, refused an empty text and a control character but took
    /// a text of nothing but spaces: unreachable through the appender, which
    /// trims, and exactly what a hand-edited or restored document can carry.
    /// Three blanks, two of them spaces the trim knows and a byte-wise test for
    /// the ASCII space does not.
    #[test]
    fn a_whitespace_only_text_is_refused_by_the_route_and_by_the_shape_rule() {
        let limits = ChatLimits::default();
        for blank in ["   ", "\u{a0}", "\u{3000} \u{2003}"] {
            assert_eq!(check_text(blank, &limits), Err("a message needs some words".to_string()), "{blank:?}");
            let stored = doc(vec![seat(1, 0, blank)]);
            let refused = check_shape(&stored, Some(2)).expect_err("a blank stored text is refused");
            assert!(refused.starts_with("message 0's text is"), "{blank:?}: {refused}");
        }
        assert_eq!(check_shape(&doc(vec![seat(1, 0, " a ")]), Some(2)), Ok(()), "text with spaces around it is text");
    }

    /// **A keep of zero keeps nothing and panics nowhere** (M6.5c2 cycle D, row
    /// 130). The launch variables refuse `0`, but [`ChatLimits`] is a public
    /// type a host builds for itself, so the trim has to be total over it: an
    /// empty document and a full one both come out empty, and the number is
    /// still never reused.
    #[test]
    fn trim_to_zero_keeps_nothing_and_panics_nowhere() {
        let mut full = doc(vec![seat(1, 0, "a"), seat(2, 1, "b")]);
        full.trim(Some(0));
        assert!(full.messages.is_empty(), "nothing kept");
        assert_eq!(full.last, 2, "and the number is not reused");
        let mut empty = ChatDoc::empty("t");
        empty.trim(Some(0));
        assert!(empty.messages.is_empty());
        assert_eq!(check_shape(&full, Some(2)), Ok(()), "a trimmed-to-nothing document still reads");
    }

    /// **The lobby switch's refusal quotes the value AS TYPED** (M6.5c2 cycle D,
    /// row 130), as the two numbers' refusals do: an operator who wrote
    /// `Maybe` is shown `Maybe`, not a lowercased copy they never typed. The
    /// value is trimmed, as theirs is, and `ON`/`Off` still read.
    #[test]
    fn the_lobby_refusal_quotes_the_value_as_typed() {
        let e = ChatConfig::from_env_values(Some(" Maybe "), None, None).unwrap_err();
        assert_eq!(e, format!("{LOBBY_VAR} must be on or off — got \"Maybe\""));
        assert!(ChatConfig::from_env_values(Some("ON"), None, None).unwrap().lobby);
        assert!(!ChatConfig::from_env_values(Some("Off"), None, None).unwrap().lobby);
    }

    /// **A value that is not UTF-8 is refused by the variable's name** (M6.5c2
    /// cycle D, row 130), never folded into "unset". It was read with
    /// `.ok()`, which turned `YOKOHAMA_WEB_CHAT_KEEP=<a stray byte>` into the
    /// default of 500 — a server whose operator set a number and runs another,
    /// the one outcome the refuse-by-name rule exists to prevent. Unset stays
    /// the default; a UTF-8 value is handed on as it is.
    #[cfg(all(feature = "server", unix))]
    #[test]
    fn a_non_utf8_value_is_refused_by_the_variables_name() {
        use std::os::unix::ffi::OsStringExt;
        let stray = std::ffi::OsString::from_vec(vec![b'5', 0xff]);
        let e = ChatConfig::value_of(KEEP_VAR, Err(std::env::VarError::NotUnicode(stray)))
            .expect_err("a value that is not UTF-8 is refused");
        assert!(e.starts_with(KEEP_VAR), "named by the variable: {e}");
        assert_eq!(ChatConfig::value_of(KEEP_VAR, Err(std::env::VarError::NotPresent)), Ok(None), "unset");
        assert_eq!(ChatConfig::value_of(KEEP_VAR, Ok("20".into())), Ok(Some("20".to_string())));
    }

    // -----------------------------------------------------------------------
    // M6.5c2 cycle D — the delete (R-D4)
    // -----------------------------------------------------------------------

    fn red() -> User {
        User::parse("red@t.test").expect("an email")
    }

    fn blue() -> User {
        User::parse("blue@t.test").expect("an email")
    }

    /// **A delete by the author leaves a TOMBSTONE and moves `rev`, never
    /// `last`** (R-D4) — the catcher for "a delete bumps `last`" and "a delete
    /// leaves `rev` unchanged".
    ///
    /// The tombstone keeps the message's number, clock and author, so a cursor
    /// a client holds still means what it meant and the row can still say who
    /// posted; its text is the empty string, never absent. The page is read by
    /// SOMEONE ELSE, so the wire shape asserted is the one every other reader
    /// is handed.
    #[test]
    fn a_delete_by_the_author_leaves_a_tombstone_and_moves_rev_not_last() {
        let mut d = doc(vec![seat(1, 0, "a"), seat(2, 0, "b")]);
        assert_eq!(d.counts(), ChatCounts { last: 2, rev: 2 }, "premise: two appends");
        d.delete(1, &red(), None).expect("the author may delete their own message");
        assert_eq!(d.counts(), ChatCounts { last: 2, rev: 3 }, "a delete assigns no number and moves the revision");
        let m = &d.messages[0];
        assert_eq!((m.seq, m.created_at, m.text.as_str(), m.deleted), (1, 1, "", true));
        assert_eq!(m.author, seat(1, 0, "a").author, "the tombstone still names who posted");
        assert!(!d.messages[1].deleted, "and only the message asked for");
        assert_eq!(check_shape(&d, Some(2)), Ok(()), "a tombstone is a message the shape rule reads");
        let page = serde_json::to_value(d.page_for(&blue())).expect("a page");
        assert_eq!(
            page[0],
            serde_json::json!({
                "seq": 1, "created_at": 1, "text": "", "deleted": true,
                "author": { "kind": "seat", "seat": 0, "name": null, "you": false },
            }),
        );
        assert_eq!(page[1]["deleted"], false, "every message carries the field");
    }

    /// **Only the author or the table's creator may delete** (R-D4, Review
    /// Focus 2) — the catcher for "a third seat may delete", at the document.
    ///
    /// Blue is neither: refused with no creator (the house), and refused where
    /// the creator is somebody else. Blue as the table's CREATOR may. A bot's
    /// message has no author who can ask, so only the creator may take it
    /// down. Every refusal changes nothing.
    #[test]
    fn only_the_author_or_the_tables_creator_may_delete() {
        let mut d = doc(vec![seat(1, 0, "a")]);
        assert_eq!(d.delete(1, &blue(), None), Err(ChatRefusal::NotYours), "no creator: the author alone");
        assert_eq!(
            d.delete(1, &blue(), Some("green@t.test")),
            Err(ChatRefusal::NotYours),
            "a creator who is somebody else",
        );
        assert_eq!((d.rev, d.messages[0].deleted, d.messages[0].text.as_str()), (1, false, "a"), "refused, untouched");
        d.delete(1, &blue(), Some("blue@t.test")).expect("the table's creator may delete any message");
        assert!(d.messages[0].deleted);

        let bot = Message {
            seq: 1,
            created_at: 1,
            author: Author::Bot { seat: 1, name: None },
            text: "gg".into(),
            deleted: false,
        };
        let mut b = doc(vec![bot]);
        assert_eq!(b.delete(1, &red(), None), Err(ChatRefusal::NotYours), "a bot's message has no author to ask");
        b.delete(1, &red(), Some("red@t.test")).expect("the creator may take it down");
        assert_eq!(
            ChatRefusal::NotYours.to_string(),
            "only the message's author or the table's creator may delete it",
        );
    }

    /// **A second delete is idempotent, and a stranger is refused at a
    /// tombstone too** (R-D4). The author asked twice changes the document
    /// once — `rev` does not move again, so the caller writes and signals
    /// nothing the second time — and because the tombstone keeps its author,
    /// the refusal is asked FIRST: a stranger cannot learn that a message was
    /// already deleted by being answered differently.
    #[test]
    fn a_second_delete_is_idempotent_and_a_strangers_is_still_refused() {
        let mut d = doc(vec![seat(1, 0, "a")]);
        d.delete(1, &red(), None).expect("the first");
        let after_one = d.counts();
        d.delete(1, &red(), None).expect("the second is no refusal");
        assert_eq!(d.counts(), after_one, "and moves nothing");
        assert_eq!(d.delete(1, &blue(), None), Err(ChatRefusal::NotYours));
    }

    /// **A number the document does not hold is not found** (R-D4): one never
    /// assigned, one the trim has dropped, and zero. A 404 rather than a quiet
    /// 204, because nothing was there to delete — and the trimmed message's
    /// author is gone with it, so its answer could not be a 403 either.
    #[test]
    fn a_number_the_document_does_not_hold_is_not_found() {
        let mut d = doc((1..=3).map(|i| seat(i, 0, "m")).collect());
        d.trim(Some(2));
        assert_eq!(d.delete(1, &red(), None), Err(ChatRefusal::NotFound(1)), "trimmed");
        assert_eq!(d.delete(4, &red(), None), Err(ChatRefusal::NotFound(4)), "never assigned");
        assert_eq!(d.delete(0, &red(), None), Err(ChatRefusal::NotFound(0)), "no message is numbered 0");
        assert_eq!(d.counts(), ChatCounts { last: 3, rev: 3 }, "and nothing moved");
        assert_eq!(ChatRefusal::NotFound(4).to_string(), "no message 4 in this chat");
    }

    /// **The shape rule reads a tombstone and nothing that only half is one**
    /// (R-D4): a deleted message carries no text, and a live one still needs
    /// some — the whitespace rule above is unchanged for it.
    #[test]
    fn a_tombstone_carries_no_text_and_a_live_message_still_needs_some() {
        let mut d = doc(vec![seat(1, 0, "a")]);
        d.messages[0].deleted = true;
        let e = check_shape(&d, Some(2)).expect_err("a tombstone that still carries text");
        assert!(e.starts_with("message 0"), "{e}");
        d.messages[0].text = String::new();
        assert_eq!(check_shape(&d, Some(2)), Ok(()), "a tombstone");
        d.messages[0].deleted = false;
        assert!(check_shape(&d, Some(2)).is_err(), "an empty live message");
    }

    /// **A document written before the delete reads as it did** (R-D4):
    /// neither `rev` nor `deleted` is required on read, both default, and the
    /// shape rule takes the result.
    #[test]
    fn a_document_written_before_the_delete_reads_as_before() {
        let old: ChatDoc = serde_json::from_value(serde_json::json!({
            "schema": 1, "id": "t", "last": 2,
            "messages": [{
                "seq": 2, "created_at": 1, "text": "hi",
                "author": { "kind": "watcher", "user": "g@t.test", "name": null },
            }],
        }))
        .expect("an older document parses");
        assert_eq!(old.counts(), ChatCounts { last: 2, rev: 0 });
        assert!(!old.messages[0].deleted);
        assert_eq!(check_shape(&old, None), Ok(()));
    }

    /// **The tag is four hex of SHA-256 over the room's salt, a newline and the key** (security S2c, finding D8;
    /// decision D-5): deterministic; different by salt (two rooms never share a tag) and by key (two posters never
    /// share one, bar a 1-in-65,536 collision); never longer than four. The reference value is the hash computed by
    /// hand once (`printf 'salt\nred@wire.test' | sha256sum | cut -c1-4`).
    #[test]
    fn a_tag_is_four_hex_of_the_salted_key_and_differs_by_salt_and_by_key() {
        let t = tag_of("salt", "red@wire.test");
        assert_eq!(t.len(), TAG_HEX);
        assert_eq!(t, "c54e", "the hand-computed reference (`printf 'salt\\nred@wire.test' | sha256sum | cut -c1-4`): the newline is part of the derivation — dropping it passed every unit cell (Task 4's probe P-sep)");
        assert!(t.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()), "{t}");
        assert_eq!(t, tag_of("salt", "red@wire.test"), "deterministic");
        assert_ne!(t, tag_of("other", "red@wire.test"), "a different room, a different tag");
        assert_ne!(t, tag_of("salt", "blue@wire.test"), "a different key, a different tag");
        assert_eq!(TAG_HEX, 4);
    }

    /// **A watcher's view carries the tag; a seat's and a bot's never; a document with no salt carries `None`**
    /// (Review Focus 4: a document written before this cycle, read on any host). The reader's own message is
    /// tagged too — "you · a1b2" tells the reader their own tag so they can recognise it on their next message.
    #[test]
    fn view_for_tags_watcher_authors_only_and_none_without_a_salt() {
        let red = User::parse("red@wire.test").unwrap();
        let watcher = Message { seq: 1, created_at: 0, author: Author::Watcher { user: "green@wire.test".into(), name: Some("Green".into()) }, text: "hi".into(), deleted: false };
        let seat = Message { seq: 2, created_at: 0, author: Author::Seat { seat: 0, user: "red@wire.test".into(), name: Some("Red".into()) }, text: "gl".into(), deleted: false };
        let bot = Message { seq: 3, created_at: 0, author: Author::Bot { seat: 1, name: None }, text: "…".into(), deleted: false };
        match watcher.view_for(&red, Some("salt")).author {
            AuthorView::Watcher { tag, you, .. } => { assert_eq!(tag, Some(tag_of("salt", "green@wire.test"))); assert!(!you); }
            other => panic!("{other:?}"),
        }
        match watcher.view_for(&red, None).author {
            AuthorView::Watcher { tag, .. } => assert_eq!(tag, None, "no salt yet: no tag, and no panic"),
            other => panic!("{other:?}"),
        }
        assert!(matches!(seat.view_for(&red, Some("salt")).author, AuthorView::Seat { you: true, .. }), "a seat is server-bound: no tag field at all");
        assert!(matches!(bot.view_for(&red, Some("salt")).author, AuthorView::Bot { .. }));
        let own = Message { seq: 4, created_at: 0, author: Author::Watcher { user: "red@wire.test".into(), name: None }, text: "me".into(), deleted: false };
        match own.view_for(&red, Some("salt")).author {
            AuthorView::Watcher { tag, you, name } => { assert_eq!(tag, Some(tag_of("salt", "red@wire.test"))); assert!(you); assert_eq!(name, None); }
            other => panic!("{other:?}"),
        }
    }

    /// **The salt is on the wire as a field a reader never sees and an old document reads as absent.**
    #[test]
    fn a_document_without_a_salt_field_deserializes_with_none_and_a_view_carries_no_salt() {
        let old = r#"{"schema":1,"id":"t","last":0,"rev":0,"messages":[]}"#;
        let doc: ChatDoc = serde_json::from_str(old).expect("an old document");
        assert_eq!(doc.salt, None);
        let json = serde_json::to_string(&MessageView { seq: 1, created_at: 0, author: AuthorView::Watcher { name: None, you: false, tag: Some("abcd".into()) }, text: String::new(), deleted: false }).unwrap();
        assert!(json.contains(r#""tag":"abcd""#) && !json.contains("salt"), "{json}");
    }
}
