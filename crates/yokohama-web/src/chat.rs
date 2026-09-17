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
//! indexes `colors[seat]` with it, and an input is bounded where it enters.
//!
//! Two things are deliberately NOT refused on read (R-6.4.8): a text longer than
//! the configured cap (served as stored) and a document longer than the
//! configured keep (trimmed on its next append), so lowering either number on
//! a running volume cannot take a table out.

use serde::{Deserialize, Serialize};

use crate::identity::User;
use crate::tables::check_name;

/// This document's own shape version, unrelated to `SEMANTICS_REV`.
pub const CHAT_SCHEMA: u32 = 1;

/// The one id the house document answers to.
///
/// One house, one id: a second name would be a second file nothing lists and
/// nothing deletes, so `store` refuses one on both implementations rather than
/// leaving `MemStore` — where `is_safe_id("house2")` is perfectly true — to
/// keep a document `FsStore` would never have written.
pub const HOUSE: &str = "house";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDoc {
    pub schema: u32,
    pub id: String,
    /// The newest number ever assigned — NOT `messages.len()`, and not the
    /// newest retained: a trim drops messages and leaves this alone, so a
    /// client's `after` cursor stays valid across it.
    pub last: u64,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub seq: u64,
    /// Unix seconds. **Named `created_at` because the corpus freezer pins a
    /// clock by KEY NAME** (`tests/wire_fixtures.rs`); a field spelled `at`
    /// would churn fixtures on every regeneration.
    pub created_at: u64,
    pub author: Author,
    pub text: String,
}

/// Who said it.
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
    pub lobby: bool,
    pub limits: ChatLimits,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self { lobby: true, limits: ChatLimits::default() }
    }
}

impl ChatConfig {
    pub fn from_env() -> Result<Self, String> {
        Self::from_env_values(
            std::env::var(LOBBY_VAR).ok().as_deref(),
            std::env::var(KEEP_VAR).ok().as_deref(),
            std::env::var(TEXT_VAR).ok().as_deref(),
        )
    }

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
        let lobby = match lobby.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
            None | Some("on") => true,
            Some("off") => false,
            Some(other) => return Err(format!("{LOBBY_VAR} must be on or off — got {other:?}")),
        };
        let keep = match keep.map(str::trim) {
            None => Some(500),
            Some("all") => None,
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
            None => 500,
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
        Self { schema: CHAT_SCHEMA, id: id.to_string(), last: 0, messages: Vec::new() }
    }

    /// Drop the OLDEST until `keep` remain; `last` is untouched, so a number is
    /// never reused and a cursor taken before the trim still means what it did.
    pub fn trim(&mut self, keep: Option<usize>) {
        if let Some(k) = keep {
            if self.messages.len() > k {
                let drop = self.messages.len() - k;
                self.messages.drain(..drop);
            }
        }
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
    if text.chars().any(char::is_control) {
        return Err("a message is one line, with no control characters".to_string());
    }
    Ok(text.to_string())
}

/// The shape rule, on read AND on write.
///
/// `players` is the table's count, or `None` at the house, where no author
/// holds a seat. Length is deliberately not a refusal here — see the module
/// doc.
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
        if m.text.is_empty() || m.text.chars().any(char::is_control) {
            return Err(format!("message {i}'s text is empty or carries a control character"));
        }
        let (user, name, seat) = match &m.author {
            Author::Seat { seat, user, name } => (Some(user), name.as_deref(), Some(*seat)),
            Author::Watcher { user, name } => (Some(user), name.as_deref(), None),
            Author::Bot { seat, name } => (None, name.as_deref(), Some(*seat)),
        };
        if let Some(u) = user {
            if User::parse(u).is_err() {
                return Err(format!(
                    "message {i}'s author {u:?} is not an address this server writes",
                ));
            }
        }
        if let Some(n) = name {
            check_name(n).map_err(|why| format!("message {i}'s author name: {why}"))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn seat(seq: u64, s: u8, text: &str) -> Message {
        Message {
            seq,
            created_at: 1,
            author: Author::Seat { seat: s, user: "red@t.test".into(), name: None },
            text: text.into(),
        }
    }

    fn doc(msgs: Vec<Message>) -> ChatDoc {
        let last = msgs.last().map_or(0, |m| m.seq);
        ChatDoc { schema: CHAT_SCHEMA, id: "t".into(), last, messages: msgs }
    }

    #[test]
    fn check_text_trims_and_bounds_by_characters_not_bytes() {
        let limits = ChatLimits { keep: Some(500), text: 5 };
        assert_eq!(check_text("  héllo ", &limits).unwrap(), "héllo");
        assert!(check_text("hello!", &limits).is_err(), "six characters over a cap of five");
        assert!(check_text("   ", &limits).is_err(), "empty after trimming");
        assert!(check_text("a\nb", &limits).is_err(), "a control character");
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
            Author::Seat { seat: 0, user: "not an email".into(), name: None };
        assert!(check_shape(&bad_user, Some(2)).is_err());
        let mut bad_name = doc(vec![seat(1, 0, "a")]);
        bad_name.messages[0].author =
            Author::Seat { seat: 0, user: "r@t.test".into(), name: Some("x".repeat(65)) };
        assert!(check_shape(&bad_name, Some(2)).is_err());
        let ctl = doc(vec![seat(1, 0, "a\u{7}b")]);
        assert!(check_shape(&ctl, Some(2)).is_err(), "a control character in a stored text");
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
}
