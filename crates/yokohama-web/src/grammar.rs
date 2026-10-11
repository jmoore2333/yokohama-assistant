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
//! pick an option, answer a sub-prompt, page, undo — and a word added to the
//! REPL tomorrow is refused here by default rather than exposed by default.
//!
//! **A word can also LEAVE the list, and one has** (M6.1 §5.4). `privacy`
//! toggled the hand mask when one browser served every seat; a seated table
//! resolves each caller to its own seat and masks every other hand before the
//! payload is built, so the toggle has nothing left to hide and no seat it
//! could speak for. It is refused by name rather than by default, because the
//! client that sends it was right yesterday.

/// The head words a browser may send.
///
/// Everything the client's own affordances produce, and nothing else.
/// `PromptState` tells the client which of these are live; this is the fence,
/// not the menu.
const HEADS: &[&str] = &[
    // sub-prompt control
    "confirm", "cancel", // paging over a long option list
    "more", "list", // the two spellings of the same undo
    "undo", "back", // the four move grammars: placement A/B, assistant-move, retreat
    "a", "b", "m", "r",
];

/// The most bytes a gameplay line may carry (security S1c, finding D28). The
/// web client composes nothing longer than an option number or an area token
/// (`A05`), under six bytes; the grammar also admits the CLI's four move
/// grammars for the terminal's sake, whose longest meaningful line is a 4P
/// retreat naming every area with its count — eighteen `A17:5` pairs behind
/// `r `, 99 bytes (109 zero-padded; the pre-flight's measurement) — so 256 is
/// more than twice the longest legal line and an eight-thousandth of the 2 MiB body
/// limit a hostile line could fill.
pub const LINE_MAX: usize = 256;

/// The shape of a line BEFORE its grammar is read: at most [`LINE_MAX`] bytes,
/// no control character (a newline inside a "line" once read as three tokens;
/// finding D29's note). The refusal names the LENGTH and never the text, since
/// a refusal that echoed the line was the review's 3.5× amplifier.
pub fn check_line(line: &str) -> Result<(), String> {
    if line.len() > LINE_MAX {
        return Err(format!("a line is at most {LINE_MAX} bytes — this one is {} bytes", line.len()));
    }
    if let Some(c) = line.chars().find(|c| c.is_control()) {
        return Err(format!("a line carries no control character — this one has {}", c.escape_unicode()));
    }
    Ok(())
}

/// Whether `line` is a gameplay line a browser may send.
///
/// A bare empty line IS allowed: it is the CLI's "apply the sole option", which
/// the client offers as a button whenever there is exactly one.
pub fn is_gameplay_line(line: &str) -> bool {
    if line.chars().any(char::is_control) {
        return false;
    }
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
/// `t` separator in `b 5 t 6`, and `hand` as an assistant-move endpoint.
///
/// **What this fences is the SHAPE of an argument, not what a word means**:
/// it admits no `/`, `\`, `.` or space, so no token names a path's components
/// — but any alphanumeric token IS a relative path, and `save`, `undo` and
/// `a` are tokens too. What keeps a REPL word from reaching `step` as a
/// command is [`HEADS`] and [`is_area_or_number`]: a head that is not listed
/// is refused there, and the REPL matches whole token slices, so an admitted
/// `undo save` is `["undo", "save"]` — no command of that shape exists. A
/// line with a newline inside it is refused before this by [`check_line`].
fn is_token(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 16
        && token.chars().all(|c| c.is_ascii_alphanumeric() || c == ':')
}

/// Whether `line` is the undo, in either of its two spellings.
///
/// **The undo RULE is not here** — R-P6 needs the table and the record, and
/// lives in `api.rs`'s `undo_allowed`. This is only the grammar's answer to
/// "was that an undo", kept beside `HEADS` because that is where the two
/// words are admitted and where a third would be added.
///
/// Whole-line equality after a trim, deliberately, and NOT a head test: `undo
/// 3` passes [`is_gameplay_line`] as an allowlisted head with a numeric
/// argument, and a head test would report it as an undo. The REPL dispatches on
/// the whole token slice (`["undo"] | ["back"]`, `repl.rs:951`), so this
/// predicate and the thing it predicts agree on exactly one set of lines.
pub fn is_undo(line: &str) -> bool {
    matches!(line.trim(), "undo" | "back")
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
        "privacy" => {
            "masked every hand at one shared terminal — a seated table has no privacy toggle: \
             every seat sees its own hand and no other"
        }
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
        "\"{}\" {why}. This endpoint takes an option number, a sub-prompt answer \
         (confirm/cancel/an area/a count), a move grammar (a/b/m/r), more/list, undo/back, \
         or an empty line to apply a sole option.",
        crate::echo::echo(line)
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

    /// **The line bound** (finding D28): the longest line the client's own
    /// grammar composes passes with room, and a line over the bound or
    /// carrying any control character is refused by a sentence that names the
    /// LENGTH and never the text — a 940 KB line of soft hyphens once came
    /// back as a 3.3 MB body. `1\nsave x` is refused here too (finding D29's
    /// note: `split_whitespace` read it as three tokens), by the control rule.
    /// **The bound is pinned at both of its edges and at its figure** (the S1
    /// stack review's CB-F4): a line of exactly `LINE_MAX` bytes passes and
    /// one more byte is refused, and `LINE_MAX` is 256 — so `>=` at the check
    /// fails the exact-length line, and a moved literal fails the figure.
    #[test]
    fn the_line_bound_admits_the_longest_legal_line_and_refuses_a_long_or_control_one() {
        // The longest line the GRAMMAR admits with meaning — the CLI's retreat
        // at a 4P table, an `A`-prefixed area id and a count for every one of
        // the eighteen areas behind `r `, 99 bytes; the web client itself
        // composes nothing longer than an option number or an area token.
        let longest = format!("r {}", (0..18).map(|a| format!("A{a}:5")).collect::<Vec<_>>().join(" "));
        assert!(longest.len() <= LINE_MAX / 2, "the longest legal line is {} bytes, the bound {LINE_MAX}", longest.len());
        assert!(check_line(&longest).is_ok() && is_gameplay_line(&longest));
        assert!(check_line("").is_ok(), "the empty line applies the sole option");

        assert_eq!(LINE_MAX, 256, "the line bound is 256 bytes (finding D28)");
        assert!(check_line(&"1".repeat(LINE_MAX)).is_ok(), "a line of exactly LINE_MAX bytes passes");
        let long = "1".repeat(LINE_MAX + 1);
        let err = check_line(&long).expect_err("over the bound");
        assert!(err.contains(&format!("{} bytes", LINE_MAX + 1)), "{err}");
        assert!(!err.contains("111111"), "the sentence names the length, never the text: {err}");
        let huge = "\u{ad}".repeat(940 * 1024 / 2);
        let err = check_line(&huge).expect_err("over the bound");
        assert!(err.len() < 200, "the refusal of a {} byte line is {} bytes", huge.len(), err.len());

        for line in ["1\nsave x", "1\rsave", "a\tb", "1\u{0}"] {
            let err = check_line(line).expect_err("a control character");
            assert!(err.contains("control character"), "{line:?}: {err}");
            assert!(!is_gameplay_line(line), "{line:?} is refused by the grammar as well");
        }
    }

    /// `refusal` repeats the line through the echo rule: sixty-four characters,
    /// escaped — the mutant that keeps `{line:?}` fails the `…` line first,
    /// then the length line.
    #[test]
    fn a_refusal_echoes_at_most_sixty_four_characters() {
        let r = refusal(&"save ".repeat(100));
        assert!(r.starts_with("\"save save"), "{r}");
        assert!(r.contains('…'), "{r}");
        assert!(r.len() < 400, "{}", r.len());
        assert_eq!(refusal("save x"), format!("{:?} writes a file on the server; games persist automatically after every move. This endpoint takes an option number, a sub-prompt answer (confirm/cancel/an area/a count), a move grammar (a/b/m/r), more/list, undo/back, or an empty line to apply a sole option.", "save x"));
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

    /// **The two spellings of one undo, named here because the RULE is
    /// elsewhere.**
    ///
    /// `lib.rs` has to know whether a line is an undo before `Repl::step` sees
    /// it (R-P6), and which words those are is the grammar's fact, not the
    /// handler's — a second list there goes stale the day a third spelling is
    /// admitted here.
    ///
    /// **The trailing-token cases are the load-bearing half.** `undo 3` passes
    /// [`is_gameplay_line`] (an allowlisted head with a numeric argument), so a
    /// rule keyed on the HEAD alone would fence `undo` and wave `undo 3`
    /// through. It is safe because the REPL dispatches undo on the whole token
    /// slice — `["undo"] | ["back"]`, `repl.rs:951` — so `undo 3` undoes
    /// nothing there either, and `trim()`-equality is exactly that set: the
    /// REPL splits on whitespace, so surrounding space undoes in both and
    /// interior space undoes in neither.
    #[test]
    fn undo_and_back_are_the_two_spellings_of_the_same_undo() {
        assert!(is_undo("undo"));
        assert!(is_undo("back"));
        assert!(is_undo("  undo  "), "the client does not always trim");
        assert!(!is_undo("1"), "an option number is not an undo");
        assert!(!is_undo(""), "the empty line applies the sole option");
        assert!(!is_undo("undo 3"), "undo takes no argument, and the REPL agrees");
        assert!(!is_undo("undone"));
    }

    /// **`privacy` is not a word a seated table has** (M6.1 §5.4).
    ///
    /// The toggle hid every hand at ONE terminal, which is what a hot seat
    /// needed when one browser served every seat. A seated table is bound: the
    /// server resolves each caller to its own seat and masks every other hand
    /// before the payload is built, so there is nothing left for a toggle to
    /// hide and no seat it could speak for. The refusal says THAT rather than
    /// "is not a gameplay line", because a client written before this cycle
    /// sent the word and its author needs to know the fence moved, not that
    /// they mistyped.
    #[test]
    fn privacy_left_the_grammar_when_every_seat_got_its_own_view() {
        assert!(!is_gameplay_line("privacy"), "privacy must be refused");
        let r = refusal("privacy");
        assert!(
            r.contains("a seated table has no privacy toggle: every seat sees its own hand \
                        and no other"),
            "{r}",
        );
        // And the menu at the end of EVERY refusal stops offering it. Read on a
        // refusal that does not mention privacy for its own reason, so this
        // cannot be satisfied by the sentence above.
        assert!(!refusal("save /tmp/x").contains("privacy"), "the menu still offers it");
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
