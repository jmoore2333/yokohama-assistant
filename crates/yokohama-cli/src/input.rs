//! Input ACQUISITION: the one place the driver turns a stream of bytes into
//! the lines `wizard::run_wizard` and `repl::Repl::step` parse (Phase-3.5a T9,
//! spec §0.9).
//!
//! **The module's name is true again.** It once held the common half of three
//! card grammars: an order-token parser (`O17`/`17`), a tech-token parser
//! (`T05`/a name prefix), and a `TechEntryError` whose two variants existed so
//! the wizard and the live reveal could word the same failure for different
//! audiences. T7 moved the wizard onto [`crate::card_prompt`], the crate's ONE
//! card grammar (spec §0.2, findings F4/F5); T8 moved the live reveal onto it
//! too, which left both parsers with no caller, and they went rather than being
//! kept warm — a second parser for card tokens is exactly how the driver came to
//! have four grammars, and one that compiles but is unreachable is the version
//! of that nobody notices. What SURVIVED that move lives in `card_prompt` and
//! `reveal`: a name matching both copies of a title is ONE answer rather than an
//! ambiguity (ten titles circulate as two IDENTICAL cards — R8, spec §0.6 — so
//! asking an operator which of two indistinguishable pieces of cardboard they
//! hold is a question with no answer), and a live reveal's refusal still talks
//! about the DECK rather than the registry
//! ([`crate::reveal::check_tech_reveal`], where that wording is pinned).
//!
//! # The seam
//!
//! [`LineReader`] is an `Iterator<Item = String>` with two arms and no third:
//!
//! * **Plain** — `BufRead::lines()`, byte for byte what the driver read before
//!   T9. Every pipe, every redirect, every test, every CI run takes this arm.
//! * **Editor** — `rustyline`, giving ← → Home End Backspace editing and ↑ ↓
//!   over the lines entered this session.
//!
//! Which arm is a decision made ONCE, by `main`, from the process's own
//! streams — see [`init_from_tty`] and [`editor_gate`]. Nothing downstream of
//! the iterator can tell which arm produced a line, and that is the design:
//! what reaches a parser is exactly the bytes a plain `read_line` would have
//! delivered, so the line protocol, the REPL grammar and every recorded event
//! are untouched by this module's existence.
//!
//! # Why the editor prints no prompt of its own
//!
//! [`EDIT_PROMPT`] is `""`, which looks like an omission and is not. Every
//! surface `main` prints — the composed frame, a view, a wizard question —
//! ends its prompt with a NEWLINE (`decision_render` closes the decision zone
//! with `"> \n"`), so at the moment a read begins the cursor is already at
//! column 0 of a fresh row with nothing to its left. That is precisely the
//! geometry rustyline assumes for a zero-width prompt, so its redraw — which
//! is a carriage return, the prompt, the line, and an erase-to-end-of-display —
//! can only ever repaint the row the user is typing on. The `> ` marker sits on
//! the row ABOVE and is out of the editor's reach BY CONSTRUCTION rather than
//! by our being careful. `the_editor_prompt_is_empty_because_the_frame_ends_at_column_zero`
//! pins both halves of that sentence against a real composed frame.
//!
//! # History is in memory, and that is a leak fence
//!
//! In a live-tracked game the lines an operator types NAME HIDDEN CARDS: a
//! reveal capture is a sequence of order titles and technology names that no
//! other player at the table is entitled to see. A history FILE would leave
//! them on disk after the game, outside every guard the driver puts on
//! `PlayerView` and the privacy toggle, readable by anything that can read a
//! home directory. So the history is per-session and in memory, and it is that
//! structurally: `Cargo.toml` takes rustyline with `default-features = false`,
//! which is what makes `DefaultHistory` the in-memory `MemHistory` instead of
//! the file-backed one. Turning the feature back on does not silently start
//! writing a file — it fails to compile
//! ([`tests::history_is_in_memory_by_construction`]).
//!
//! # Ctrl-C and Ctrl-D
//!
//! Raw mode means rustyline sees both keys instead of the kernel, so both need
//! an answer ([`classify`]):
//!
//! * **Ctrl-C** discards the half-typed line and offers a fresh one. It must
//!   NOT arrive at the REPL as an empty line: bare Enter on a one-option prompt
//!   is the forced-option ceremony and APPLIES that option, so an interrupt
//!   mapped to `""` would make Ctrl-C play a move.
//! * **Ctrl-D on an empty line** ends the input stream, which is exactly what
//!   the pre-T9 binary did with it (canonical-mode EOF) and exactly what a
//!   script running out of lines does. `quit` remains the way to leave with the
//!   `bye` line.
//!
//! Under a pipe neither key reaches this module at all — SIGINT and EOF behave
//! as they always have.
//!
//! # Toggles: what shipped
//!
//! Spec §0.9 asks for single-KEYPRESS view toggles *if the editor's key
//! bindings allow it cheaply*, and names single-letter + Enter as the
//! guaranteed minimum. Single keypress is NOT cheap here, for a reason that is
//! about the editor rather than about effort: rustyline dispatches one key to
//! one `Cmd`, and there is no `Cmd` that inserts text AND accepts the line, so
//! "press `t`, get `tracks`" cannot be expressed as a binding at all — it needs
//! a bespoke read loop wrapped around the editor, i.e. the raw-mode TUI
//! navigation spec §1 puts OUT of 3.5a. Binding a bare letter would also
//! capture that letter everywhere it is typed unless the handler re-derives
//! "the line is empty" per keystroke. So the shipped tier is the stated
//! minimum, and it needs no grammar change to reach: `view` opens the menu and
//! every surface on it is its own number + Enter, while the surfaces
//! themselves keep the words `tracks`, `sites`, `hand`, `log`, `?`.

use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};

use rustyline::error::ReadlineError;
use yokohama_core::*;

/// The prompt string handed to the line editor: EMPTY, deliberately. See the
/// module doc — the frame has already drawn `> ` and left the cursor at column
/// 0 of the row below it, so a zero-width prompt is the truthful description of
/// where the editor is starting from.
const EDIT_PROMPT: &str = "";

/// Set by [`init_from_tty`] and read by [`may_edit`]; `false` until then.
///
/// **The switch, and the fence.** It is the same shape `style::init_from_tty`
/// and `screen::init_from_tty` use, for the same reason and one extra: only
/// `main` calls the initializer, so no test can reach the editor arm however it
/// is run. The lesson `screen::may_measure` records applies here verbatim — a
/// test that merely observed "no editor was constructed" would also pass with
/// the gate deleted whenever the suite happens to run without a terminal, which
/// is to say it would pass in CI and stop catching the regression on a
/// developer's laptop. `cargo test -- --nocapture` on a real terminal is that
/// laptop: stdout IS a tty there, so a gate that asked the streams directly
/// would arm itself inside the suite.
static INTERACTIVE: AtomicBool = AtomicBool::new(false);

/// Decide, once, whether this process's input may be edited. Called by `main`
/// and by nothing else.
pub fn init_from_tty() {
    use std::io::IsTerminal;
    INTERACTIVE.store(
        editor_gate(
            std::io::stdin().is_terminal(),
            std::io::stdout().is_terminal(),
            std::env::var("TERM").ok().as_deref(),
        ),
        Ordering::Relaxed,
    );
}

/// Terminals rustyline will not put in raw mode — its own `UNSUPPORTED_TERM`
/// list, compared its way (ASCII case-insensitive), and an UNSET `TERM` is not
/// on it. Mirrored rather than imported because the upstream constant is
/// private; if a rustyline upgrade changes the list this copy has to follow it,
/// and the way that shows up is the third clause below quietly doing nothing.
const RAW_MODE_UNSUPPORTED_TERMS: [&str; 3] = ["dumb", "cons25", "emacs"];

/// **Three clauses, and none of them is about taste.**
///
/// 1. stdin being a terminal is what makes editing meaningful at all.
/// 2. stdout being one is what keeps the piped protocol byte-identical:
///    rustyline's unix terminal writes its redraw to `STDOUT_FILENO`, so with
///    stdin on a tty and stdout on a pipe an editor would be spraying escape
///    sequences into the redirected stream — breaking "piped output is
///    byte-plain" (spec §0.12) in exactly the configuration a developer reaches
///    for when capturing a transcript.
/// 3. `TERM` naming a terminal rustyline cannot put in raw mode is refused
///    because of what rustyline does INSTEAD, which is not "nothing": it falls
///    back to a plain reader of its OWN (`readline_direct`), and that reader is
///    not `BufRead::lines()` — it interprets `\x08` as a backspace and rubs out
///    the character before it. Taking the plain arm here keeps ONE reader
///    defining what a line is, rather than two that agree on almost every input.
fn editor_gate(stdin_is_terminal: bool, stdout_is_terminal: bool, term: Option<&str>) -> bool {
    if !(stdin_is_terminal && stdout_is_terminal) {
        return false;
    }
    !term.is_some_and(|t| RAW_MODE_UNSUPPORTED_TERMS.iter().any(|u| u.eq_ignore_ascii_case(t)))
}

/// Whether this process may put a line editor on its input.
///
/// Its own function so the fence can be ASSERTED rather than inferred — see
/// [`INTERACTIVE`].
fn may_edit() -> bool {
    INTERACTIVE.load(Ordering::Relaxed)
}

/// The driver's line source: an `Iterator<Item = String>` over whatever the
/// user is typing into, editable or not.
///
/// `main` builds one with [`LineReader::from_stdin`] and hands it to the wizard
/// and then to the REPL loop, so a single arm decision covers setup and play.
pub struct LineReader {
    source: Source,
}

/// The two arms. There is no third and no fallback BETWEEN them: a process
/// that cannot build an editor reads plainly for the rest of its life rather
/// than retrying, because an arm that changed mid-game would change what a
/// keystroke means mid-game.
enum Source {
    /// `BufRead::lines()` — pipes, redirects, CI, and any terminal the gate
    /// declined. Byte-for-byte the pre-T9 read path.
    Plain(std::io::Lines<Box<dyn BufRead>>),
    /// rustyline. Boxed because an `Editor` is large next to a `Lines`.
    Editor(Box<rustyline::DefaultEditor>),
}

impl LineReader {
    /// The production constructor: an editor if [`may_edit`] says this process
    /// has a terminal on both ends AND one can be built, a plain reader in
    /// every other case.
    pub fn from_stdin() -> Self {
        if may_edit() {
            if let Some(editor) = new_editor() {
                return Self { source: Source::Editor(Box::new(editor)) };
            }
        }
        Self::plain(Box::new(std::io::stdin().lock()))
    }

    /// The plain arm over an arbitrary reader — the shape the tests measure
    /// against `BufRead::lines()` directly.
    fn plain(reader: Box<dyn BufRead>) -> Self {
        Self { source: Source::Plain(reader.lines()) }
    }
}

impl Iterator for LineReader {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        match &mut self.source {
            // `.ok()` ends the stream on a read error, which is what
            // `lines().map_while(Result::ok)` did before T9 — same iterator,
            // same stopping condition, including the UTF-8 decode error.
            Source::Plain(lines) => lines.next()?.ok(),
            Source::Editor(editor) => loop {
                match classify(editor.readline(EDIT_PROMPT)) {
                    ReadOutcome::Line(line) => return Some(line),
                    ReadOutcome::Retry => continue,
                    ReadOutcome::End => return None,
                }
            },
        }
    }
}

/// What one `readline` outcome means to the iterator. Split out so the
/// Ctrl-C/Ctrl-D mapping is a pure function with its own test rather than a
/// branch that only a real terminal could ever execute.
#[derive(Debug)]
enum ReadOutcome {
    /// Hand this line to the parser.
    Line(String),
    /// Read again. The user abandoned a line; nothing reaches the parser.
    Retry,
    /// The stream is over.
    End,
}

/// See the module doc's "Ctrl-C and Ctrl-D".
fn classify(result: Result<String, ReadlineError>) -> ReadOutcome {
    match result {
        Ok(line) => ReadOutcome::Line(line),
        // Ctrl-C: discard, re-prompt. NOT `Line(String::new())` — that is bare
        // Enter, and bare Enter applies a forced single option.
        Err(ReadlineError::Interrupted) => ReadOutcome::Retry,
        // Ctrl-D on an empty line: the pre-T9 binary's EOF, unchanged.
        Err(ReadlineError::Eof) => ReadOutcome::End,
        // A terminal fault (I/O, a syscall, a resize signal we do not handle)
        // ends the stream the way an I/O error ends the plain arm. Nothing here
        // is worth taking a live game's screen down over, and the session
        // autosaves after every applied move.
        Err(_) => ReadOutcome::End,
    }
}

/// Build the editor, or `None` if this terminal will not have one.
///
/// `auto_add_history` is what makes ↑ recall anything; rustyline's in-memory
/// history already drops empty lines, so the forced-option bare Enter never
/// fills it.
fn new_editor() -> Option<rustyline::DefaultEditor> {
    let config = rustyline::Config::builder().auto_add_history(true).build();
    rustyline::DefaultEditor::with_config(config).ok()
}

/// Tech card's display name, or a `T<id>` fallback for an id absent from
/// the registry (keeps display code panic-free against corrupt/foreign
/// records) -- the one-liner that used to be triplicated across
/// render.rs and describe.rs.
pub(crate) fn tech_name(id: TechCardId, techs: &TechRegistry) -> String {
    techs.get(id).map(|c| c.name.clone()).unwrap_or_else(|| format!("T{:02}", id.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn plain_over(bytes: &'static [u8]) -> Vec<String> {
        LineReader::plain(Box::new(Cursor::new(bytes))).collect()
    }

    fn bufread_lines(bytes: &'static [u8]) -> Vec<String> {
        Cursor::new(bytes).lines().map_while(Result::ok).collect()
    }

    /// The load-bearing property of the whole task: on the plain arm, what
    /// reaches a parser is EXACTLY what `BufRead::lines()` delivered before T9.
    ///
    /// Measured against a live `lines()` call rather than against expected
    /// literals, so the two cannot drift apart even in the corners nobody
    /// thought to write down.
    #[test]
    fn the_plain_arm_delivers_exactly_what_bufread_lines_delivers() {
        for case in [
            &b"one\ntwo\n"[..],
            b"no trailing newline",
            b"",
            b"\n\n\n",
            b"crlf\r\nsecond\r\n",
            b"  leading and trailing  \n",
            b"tabs\tinside\n",
            b"back\x08space\n",
            b"utf8 \xe2\x97\x88 mark\n",
            b"bad \xff utf8\nnever reached\n",
        ] {
            assert_eq!(
                plain_over(case),
                bufread_lines(case),
                "the plain arm must be `BufRead::lines()`, on {case:?}"
            );
        }
    }

    /// Anti-vacuity for the case above that can actually tell the two readers
    /// apart, stated positively so it cannot pass by both sides being wrong.
    ///
    /// rustyline has a plain-read fallback of its own (`readline_direct`, taken
    /// when stdin is not a tty) and it is NOT `BufRead::lines()`: it interprets
    /// `\x08` as a backspace and rubs out the character before it, so the same
    /// bytes would arrive as `"bacspace"`. A mis-gated build that let the editor
    /// see piped input would therefore be reading a slightly different language
    /// than the one the REPL's grammar is specified in — a difference no
    /// ordinary transcript would reveal, which is why it is pinned here rather
    /// than left to `piped_dogfood` (whose input contains no such byte, and
    /// which would pass either way).
    #[test]
    fn the_plain_arm_hands_over_control_bytes_untouched() {
        assert_eq!(plain_over(b"back\x08space\n"), vec!["back\u{8}space".to_string()]);
    }

    /// Every clause of the gate is load-bearing — see [`editor_gate`]. A pure
    /// function over explicit arguments, so this test says nothing at all about
    /// the machine it runs on (it does not read the real `TERM` either).
    #[test]
    fn the_editor_gate_needs_both_streams_and_a_raw_capable_term() {
        let xterm = Some("xterm-256color");
        assert!(editor_gate(true, true, xterm), "a real terminal gets the editor");
        assert!(editor_gate(true, true, None), "an UNSET TERM is not on rustyline's refusal list");
        assert!(
            !editor_gate(true, false, xterm),
            "stdout redirected: the redraw's escapes would land in the file"
        );
        assert!(!editor_gate(false, true, xterm), "stdin piped: there is nothing to edit");
        assert!(!editor_gate(false, false, xterm), "no terminal at all");
        for unsupported in ["dumb", "DUMB", "cons25", "emacs"] {
            assert!(
                !editor_gate(true, true, Some(unsupported)),
                "TERM={unsupported} makes rustyline substitute its OWN plain reader, which is not \
                 `BufRead::lines()`; the plain arm has to win instead"
            );
        }
    }

    /// The fence: the editor arm is unreachable until `main` arms it, so no
    /// test can construct one however the suite is invoked.
    ///
    /// Both directions are checked in one test because the positive half
    /// touches process-global state — arming the switch while another test
    /// asked [`LineReader::from_stdin`] for a reader would be the very race
    /// this is protecting against, and this is the only test that calls it.
    #[test]
    fn only_main_can_arm_the_editor() {
        assert!(!may_edit(), "no test calls init_from_tty, so the switch must still be off");
        assert!(
            matches!(LineReader::from_stdin().source, Source::Plain(_)),
            "with the switch off the production constructor must take the plain arm"
        );
        // …and the switch is what `may_edit` reads, not the ambient streams:
        // deleting the atomic in favour of a direct `is_terminal()` pair would
        // leave this half passing and the half above failing under
        // `cargo test -- --nocapture` on a terminal.
        INTERACTIVE.store(true, Ordering::Relaxed);
        assert!(may_edit(), "the atomic is the switch");
        INTERACTIVE.store(false, Ordering::Relaxed);
        assert!(!may_edit(), "and it is restored");
    }

    /// Ctrl-C must not become bare Enter. See [`classify`]: on a one-option
    /// prompt bare Enter APPLIES that option, so the difference between `Retry`
    /// and `Line("")` here is the difference between abandoning a typed line
    /// and playing a move by accident.
    #[test]
    fn an_interrupt_discards_the_line_and_an_eof_ends_the_stream() {
        assert!(
            matches!(classify(Ok("m a1 a2".to_string())), ReadOutcome::Line(l) if l == "m a1 a2"),
            "an accepted line passes through verbatim"
        );
        assert!(
            matches!(classify(Err(ReadlineError::Interrupted)), ReadOutcome::Retry),
            "Ctrl-C must re-read, never arrive as an empty line"
        );
        assert!(
            matches!(classify(Err(ReadlineError::Eof)), ReadOutcome::End),
            "Ctrl-D on an empty line is the pre-T9 EOF"
        );
        assert!(
            matches!(classify(Err(ReadlineError::Io(std::io::Error::other("tty gone")))), ReadOutcome::End),
            "a terminal fault ends the stream like an I/O error on the plain arm"
        );
    }

    /// The leak fence, made a compile-time fact. See the module doc: a live
    /// game's typed lines name hidden cards, so the history must never reach
    /// disk.
    ///
    /// The ASSERT below is trivia; the load-bearing part is the type ascription
    /// above it, which only compiles while `DefaultHistory` IS `MemHistory` —
    /// i.e. while `Cargo.toml` keeps `default-features = false`. Restoring
    /// `with-file-history` swaps in the file-backed history and this file stops
    /// building, which is the point: the rule is enforced by the dependency
    /// declaration rather than by remembering not to call `save`.
    #[test]
    fn history_is_in_memory_by_construction() {
        let proof: std::marker::PhantomData<rustyline::history::DefaultHistory> =
            std::marker::PhantomData::<rustyline::history::MemHistory>;
        assert_eq!(std::mem::size_of_val(&proof), 0);
    }

    /// [`EDIT_PROMPT`] is empty BECAUSE the frame ends at column 0 — the two
    /// facts are one decision, so they are pinned in one test against a REAL
    /// composed frame (`screen::wrap(repl, repl.prompt())`, which is literally
    /// what `main` prints before the first read).
    ///
    /// Make the frame stop ending in a newline and this fails, which is the
    /// warning a future layout change needs: with the cursor left mid-row, a
    /// zero-width prompt would let rustyline's carriage-return redraw repaint
    /// over whatever the frame had put to its left.
    #[test]
    fn the_editor_prompt_is_empty_because_the_frame_ends_at_column_zero() {
        use crate::repl::Repl;
        use crate::session::Session;

        let session = Session::new_sim(
            GameConfig::for_player_count(2),
            0,
            vec![StartingOrderChoice::Keep(0); 2],
            Rules::standard(),
        )
        .expect("new_sim over a standard board");
        let repl = Repl::new(session);
        crate::screen::force(true);
        let frame = crate::screen::wrap(&repl, &repl.prompt());
        crate::screen::force(false);

        assert!(
            frame.ends_with('\n'),
            "the composed frame must leave the cursor at column 0 of a fresh row; \
             it ends {:?}",
            &frame[frame.len().saturating_sub(8)..]
        );
        assert!(
            EDIT_PROMPT.is_empty(),
            "the editor draws no prompt of its own: the frame above already did"
        );
    }

    #[test]
    fn tech_name_looks_up_the_registry_and_falls_back_for_unknown_ids() {
        let rules = Rules::standard();
        assert_eq!(tech_name(TechCardId(11), &rules.techs), "Ball");
        assert_eq!(tech_name(TechCardId(255), &rules.techs), "T255");
    }
}
