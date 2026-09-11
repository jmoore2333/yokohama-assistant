//! What a browser is allowed to say to `Repl::step`.
//!
//! The mutation endpoint hands its `line` straight to the REPL's step function,
//! which is the right design — `step` IS the state machine, and a second
//! dispatcher here would drift from it. But `step` is the TERMINAL's front
//! door, and it answers to more than the game: it renders full-screen surfaces,
//! opens sticky overlays, and writes files. Three of those are wrong through
//! HTTP, and one of them is a hole.
//!
//! **`save <path>` is an arbitrary filesystem write.** `Session::save`
//! (`session.rs:407`) creates `<path>.tmp`, writes, fsyncs and renames onto
//! `<path>` — anywhere the server process can write. A `line` is client-supplied
//! text. Nothing about being behind an authenticating proxy makes that
//! acceptable: an authenticated caller is still not entitled to name a path on
//! the server's disk. Bare `save` is harmless only because `save_path` is
//! `None` on every web game, which is one line of drift from being the same
//! hole.
//!
//! **The overlay words silently change what a NUMBER means.** `?`, `index`,
//! `orders`, `techs`, `goals`, `sitecards` and `view` open a sticky overlay in
//! which a digit selects a CARD, not a move — and `ClientState` has no field
//! that reports an overlay is open, because `Repl::overlay` is `pub(crate)`.
//! Measured on a fresh 2P game, each line through `step`:
//!
//! ```text
//!     step("orders") -> the card index          events 0 -> 0
//!     step("1")      -> "nothing here is selectable"   events 0 -> 0
//!     step("cancel") -> back to the decision zone
//!     step("1")      -> the move applies        events 0 -> 1
//! ```
//!
//! So a client whose next click sends `1` would apply nothing and be told
//! nothing, or — after `view` — apply a move it never showed. That is not a
//! hazard the client can be careful about; it is a hazard the door has to close.
//!
//! **The surface words return terminal art.** `board`, `players`, `score`,
//! `tracks`, `sites`, `ports`, `labs`, `areas`, `hand`, `log`, `help`,
//! `suggest`, `why` all answer with box-drawing text measured in columns. A
//! browser cannot use it, and serving it here would make the API's answer to
//! "what is on the port board" a picture of a terminal. Those surfaces belong
//! on their own read-only routes over the same `screen_model` builders.
//!
//! **So this is an ALLOWLIST**, and deliberately not a denylist. The same shape
//! and the same reasoning as [`crate::store::is_safe_id`]: a denylist over a
//! grammar is a list of the tricks someone has thought of, and `save` is only
//! the one that is obvious today. What is on the list is what a game needs —
//! pick an option, answer a sub-prompt, page, undo, toggle privacy — and a word
//! added to the REPL tomorrow is refused here by default rather than exposed by
//! default.

/// The head words a browser may send.
///
/// Everything the client's own affordances produce, and nothing else.
/// `PromptState` tells the client which of these are live; this is the fence,
/// not the menu.
const HEADS: &[&str] = &[
    // sub-prompt control
    "confirm", "cancel", // paging over a long option list
    "more", "list", // the two spellings of the same undo
    "undo", "back", // the hand-contents mask
    "privacy", // the four move grammars: placement A/B, assistant-move, retreat
    "a", "b", "m", "r",
];

/// Whether `line` is a gameplay line a browser may send.
///
/// A bare empty line IS allowed: it is the CLI's "apply the sole option", which
/// the client offers as a button whenever there is exactly one.
pub fn is_gameplay_line(line: &str) -> bool {
    let mut tokens = line.split_whitespace();
    let Some(head) = tokens.next() else {
        return true;
    };
    (HEADS.contains(&head) || is_area_or_number(head)) && tokens.all(is_token)
}

/// A head that is not a listed word may only be a NUMBER or an AREA.
///
/// Deliberately narrower than [`is_token`], and the distinction is the whole
/// fence: a head is what selects a REPL command, so anything alphabetic that is
/// not on the list must be refused — `save`, `view` and `orders` are ordinary
/// alphanumeric words and would sail through the argument rule. An argument, by
/// contrast, has already been claimed by a listed head and only has to be
/// path-shape-free.
fn is_area_or_number(head: &str) -> bool {
    let digits = head.strip_prefix(['A', 'a']).unwrap_or(head);
    !digits.is_empty() && digits.len() <= 3 && digits.chars().all(|c| c.is_ascii_digit())
}

/// One argument or one option number: ASCII alphanumerics and `:`, nothing else.
///
/// Covers every shape the grammars use — a bare option number (`7`), an area
/// with or without its prefix (`15`, `A15`, `a15`), a retreat pair (`3:2`), the
/// `t` separator in `b 5 t 6`, and `hand` as an assistant-move endpoint. It
/// admits no `/`, no `\`, no `.` and no space, so nothing that reaches `step`
/// through here can be read as a path even if a future REPL word wanted one.
fn is_token(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 16
        && token.chars().all(|c| c.is_ascii_alphanumeric() || c == ':')
}

/// The refusal a rejected line answers with.
///
/// It names the line, says which category it fell into, and — for the words
/// that have a real home — says where that home is. A refusal that only says
/// "no" teaches the caller nothing and gets worked around.
pub fn refusal(line: &str) -> String {
    let head = line.split_whitespace().next().unwrap_or("");
    let why = match head {
        "save" => "writes a file on the server; games persist automatically after every move",
        "quit" | "q" => "ends a terminal session; a browser closes its tab",
        "?" | "index" | "orders" | "techs" | "goals" | "sitecards" | "view" | "views" => {
            "opens a sticky terminal overlay in which a number selects a card, not a move"
        }
        "board" | "players" | "score" | "tracks" | "sites" | "ports" | "labs" | "areas"
        | "hand" | "log" | "help" | "suggest" | "why" | "panel" | "screen" | "reveal" => {
            "renders a terminal surface; the API serves these as their own read-only routes"
        }
        _ => "is not a gameplay line",
    };
    format!(
        "{line:?} {why}. This endpoint takes an option number, a sub-prompt answer \
         (confirm/cancel/an area/a count), a move grammar (a/b/m/r), more/list, undo/back, \
         privacy, or an empty line to apply a sole option."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Everything the client can actually produce gets through.
    ///
    /// Enumerated from the grammars rather than sampled: an allowlist that
    /// refuses a legal move is a game that cannot be played, and the failure
    /// would show up as one unreachable phase deep in a game rather than as a
    /// test.
    #[test]
    fn every_gameplay_line_the_client_can_produce_is_allowed() {
        for line in [
            "",            // apply the sole option
            "   ",         // ...even with whitespace the client did not trim
            "1",           // an option number
            "12",          //
            "confirm",     //
            "cancel",      //
            "more",        //
            "list",        //
            "undo",        //
            "back",        //
            "privacy",     //
            "a",           // Placement Option A, picker
            "a 1 2 3",     // ...direct
            "A05",         // an area token, prefixed
            "a05",         // ...lowercase
            "5",           // ...bare
            "b 5",         // Placement Option B
            "b 5 t 6",     // ...with the Telegram adjacent bonus
            "m hand A03",  // an assistant move
            "m A03 hand",  //
            "r",           // retreat, none
            "r 1:2 3",     // ...with counts
        ] {
            assert!(is_gameplay_line(line), "{line:?} must be allowed");
        }
    }

    /// The filesystem write, in every shape a caller would try.
    ///
    /// This is the finding the allowlist exists for, so it is asserted on its
    /// own rather than inside a list of refusals: `save <path>` through this
    /// endpoint is an arbitrary write with the server's privileges.
    #[test]
    fn no_spelling_of_save_reaches_the_filesystem() {
        for line in [
            "save",
            "save /etc/passwd",
            "save ../../../../etc/crontab",
            "save ~/.ssh/authorized_keys",
            "save C:\\Windows\\System32\\drivers\\etc\\hosts",
            "  save   /tmp/x  ",
        ] {
            assert!(!is_gameplay_line(line), "{line:?} must be refused");
        }
        assert!(refusal("save /etc/passwd").contains("writes a file on the server"));
    }

    /// The overlay words, which are the quiet half of the problem.
    #[test]
    fn the_overlay_and_surface_words_are_refused() {
        for line in [
            "?", "index", "orders", "techs", "goals", "sitecards", "view", "views", "board",
            "players", "score", "tracks", "sites", "ports", "labs", "areas", "hand", "log",
            "help", "suggest", "why", "why 3", "panel", "screen", "quit", "q", "reveal",
            "reveal 0 O14",
        ] {
            assert!(!is_gameplay_line(line), "{line:?} must be refused");
        }
    }

    /// No token may carry a path separator, a dot, or a quote — whatever word
    /// it follows.
    ///
    /// Defence in depth behind the head check: the heads are what keeps `save`
    /// out today, and this is what keeps a word ADDED to the REPL tomorrow from
    /// being handed something path-shaped merely because its head happened to
    /// be one letter long.
    #[test]
    fn no_argument_can_be_path_shaped() {
        for line in [
            "a /etc/passwd",
            "b ../x",
            "m hand ./rel",
            "r 1:2 x.y",
            "a \"quoted\"",
            "b $(id)",
            "m hand A03; save /tmp/x",
            "a 111111111111111111",
        ] {
            assert!(!is_gameplay_line(line), "{line:?} must be refused");
        }
    }

    /// The refusal names the line and says what to send instead.
    #[test]
    fn a_refusal_says_what_it_refused_and_what_would_work() {
        let r = refusal("view");
        assert!(r.contains("\"view\""), "{r}");
        assert!(r.contains("sticky terminal overlay"), "{r}");
        assert!(r.contains("option number"), "{r}");
    }
}
