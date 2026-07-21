//! Guide/CLI drift tripwire (pre-gate Task C).
//!
//! The tester-facing guide pair `docs/cli-guide.md` + `docs/cli-guide.html`
//! is the AUTHORITATIVE tester documentation (see CLAUDE.md Conventions). It
//! has drifted from the CLI at least four times this project (a bad-token
//! grammar claim, the cancel line, a stale reveal letter, a fuzz-time claim) --
//! every time caught only by an expensive human review pass. These tests make
//! the cheap `cargo test` suite catch the mechanical slice: the surfaces where
//! the guides quote the CLI verbatim.
//!
//! Design rules:
//! * Authority always lives in code (`HELP_TEXT`, `render::board_grid`); the
//!   guides follow it, never the reverse. Every failure message names the
//!   guide file, the surface, and where the authoritative text lives.
//! * Assertions ride on stable strings, never on layout, so a formatting-only
//!   guide edit (markup, spacing, a reworded description) does not false-
//!   positive -- only real content/behavior drift trips them.
//! * The .html quotes the CLI inside entity-escaped `<code>`/`<pre>`; compare
//!   the two formats and the CLI strings on the same footing by unescaping.

use yokohama_cli::repl::{Repl, HELP_TEXT};
use yokohama_cli::session::Session;
use yokohama_cli::{render, screen};
use yokohama_core::{GameConfig, Rules, StartingOrderChoice};

const GUIDE_MD: &str = include_str!("../../../docs/cli-guide.md");
const GUIDE_HTML: &str = include_str!("../../../docs/cli-guide.html");

/// The clear-screen + cursor-home escape every composed frame opens with
/// (`screen::wrap`); stripped before measuring a frame's visible width.
const CLEAR_HOME: &str = "\u{1b}[2J\u{1b}[H";

/// Serializes the two frame measurements below. `screen::force` is
/// thread-local only under `cfg(test)`; an INTEGRATION test links the library
/// without it, so here `force` writes a process-wide flag -- and libtest runs
/// these two tests on parallel threads. Unserialized, `frame_width`'s closing
/// `force(false)` could land inside `frame_height`'s measurement (and vice
/// versa), leaving the frame un-composed and tripping the `CLEAR_HOME`
/// assertion below. Reproduced roughly twice per 80 runs before this lock.
static FRAME_FLAG: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The guard for `FRAME_FLAG`, ignoring poisoning: if a measurement panicked
/// while holding it, the flag is restored by the next `force` call anyway and
/// a poisoned mutex must not turn one real failure into several.
fn frame_flag_guard() -> std::sync::MutexGuard<'static, ()> {
    FRAME_FLAG.lock().unwrap_or_else(|e| e.into_inner())
}

/// Undo the only three HTML entity escapes the guide contains, so escaped
/// `.html` source can be matched against the plain-text `.md` and the CLI's
/// own strings.
fn unescape_html(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// The fenced code block immediately following a markdown `## N.` header.
fn md_code_block_after(header: &str) -> String {
    let start = GUIDE_MD
        .find(header)
        .unwrap_or_else(|| panic!("cli-guide.md is missing the header {header:?}"));
    let mut parts = GUIDE_MD[start..].split("```");
    parts.next(); // the header and any prose before the fence
    parts
        .next()
        .unwrap_or_else(|| panic!("cli-guide.md has no code block after {header:?}"))
        .trim_matches('\n')
        .to_string()
}

/// The `<pre><code>…</code></pre>` block following an HTML `<h2>` header, with
/// the wrapper stripped and entities unescaped.
fn html_pre_block_after(header: &str) -> String {
    let start = GUIDE_HTML
        .find(header)
        .unwrap_or_else(|| panic!("cli-guide.html is missing the header {header:?}"));
    let rest = &GUIDE_HTML[start..];
    let open = rest
        .find("<pre><code>")
        .unwrap_or_else(|| panic!("cli-guide.html has no <pre><code> after {header:?}"))
        + "<pre><code>".len();
    let close = rest[open..]
        .find("</code></pre>")
        .unwrap_or_else(|| panic!("cli-guide.html has an unclosed <pre><code> after {header:?}"))
        + open;
    unescape_html(&rest[open..close])
}

/// First whitespace-delimited token of each non-empty command line, skipping a
/// leading `Commands:` banner. Run over both `HELP_TEXT` and a guide §5 block
/// so the two are compared on identical footing.
fn command_tokens(block: &str) -> Vec<String> {
    block
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && *l != "Commands:")
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// A 2..=4-player board's rendered width == its first grid line's column count,
/// exactly how `Repl::board_cmd` derives the one-time width note. Read off the
/// real `render::board_grid`, so the number can never be a copied literal.
fn board_width(players: u8) -> usize {
    let session = Session::new_sim(
        GameConfig::for_player_count(players),
        0,
        vec![StartingOrderChoice::Keep(0); players as usize],
        Rules::standard(),
    )
    .expect("new_sim over a standard board");
    render::board_grid(session.state())
        .lines()
        .next()
        .map(|line| line.chars().count())
        .unwrap_or(0)
}

/// (a) Every command token the in-app `help` lists (`HELP_TEXT`, the authority
/// in `repl.rs`) is documented in BOTH guides' §5 command block. A command
/// added to the CLI but not carried into the guide trips this.
#[test]
fn help_text_commands_are_documented_in_both_guides() {
    let help = command_tokens(HELP_TEXT);
    assert!(
        help.len() > 15,
        "HELP_TEXT parse produced only {} tokens -- parser or HELP_TEXT changed shape: {help:?}",
        help.len()
    );

    for (file, block) in [
        ("cli-guide.md", md_code_block_after("## 5. Commands")),
        ("cli-guide.html", html_pre_block_after("<h2>5. Commands</h2>")),
    ] {
        let documented = command_tokens(&block);
        for cmd in &help {
            assert!(
                documented.contains(cmd),
                "{file} §5 command block does not document `{cmd}`, which HELP_TEXT lists \
                 (authority: crates/yokohama-cli/src/repl.rs). Add it to the §5 block in BOTH \
                 guide formats."
            );
        }
    }
}

/// (b) The move-grammar usage strings and Task A's Movement sub-prompt entry
/// texts appear verbatim in BOTH formats (HTML unescaped). These are strings a
/// tester copies or matches against the screen; drift in either format misleads
/// them. Authority: the `a`/`b`/`m`/`r` parsers and the route/retreat
/// sub-prompt renderers in `repl.rs` (plus `describe.rs`).
#[test]
fn move_grammar_and_subprompt_strings_appear_in_both_guides() {
    let required = [
        "a [id id id]",
        "b <id>",
        "b <id> t <adj>",
        "m <from> <to>",
        "r [id[:n]]...",
        "route options",
        "choose recovery",
        "'r' grammar works too",
        "pick a number, or 'cancel'",
        "recover how many from",
        "confirm / cancel",
    ];
    let html = unescape_html(GUIDE_HTML);
    for needle in required {
        assert!(
            GUIDE_MD.contains(needle),
            "cli-guide.md is missing the move-grammar/sub-prompt string {needle:?} \
             (authority: the a/b/m/r parsers and Movement sub-prompt render in repl.rs)."
        );
        assert!(
            html.contains(needle),
            "cli-guide.html is missing the move-grammar/sub-prompt string {needle:?} \
             (authority: the a/b/m/r parsers and Movement sub-prompt render in repl.rs)."
        );
    }
}

/// (c) The §2 board-width sentence states the widths the real renderer
/// produces. If the grid geometry changes (e.g. `CELL_WIDTH`, or a board's
/// column count), the computed widths move and the guide must follow -- the
/// numbers are derived here, never copied.
#[test]
fn section2_board_widths_match_the_renderer() {
    let (w2, w3, w4) = (board_width(2), board_width(3), board_width(4));
    let phrase = format!("{w2} columns at 2 players, {w3} at 3, and {w4} at 4");
    for (file, text) in [("cli-guide.md", GUIDE_MD), ("cli-guide.html", GUIDE_HTML)] {
        assert!(
            text.contains(&phrase),
            "{file} §2 does not state the renderer's actual board widths. render::board_grid \
             produces {w2}/{w3}/{w4} columns at 2/3/4 players, so §2 must read {phrase:?}. Update \
             the guide (both formats) if the grid geometry changed."
        );
    }
}

/// The composed frame's rendered width at the start of a game: `screen::wrap`
/// sizes the frame to its widest element, which is never the board alone (the
/// players panel joins it side by side at 2-3 players, and the game-info strip
/// is 96 columns on its own), so this is the width a tester's terminal actually
/// has to be -- the number §2 owes them. Measured off the real `screen::wrap`,
/// so it can never be a copied literal. Frames are forced on (an integration
/// test links the lib without `cfg(test)`, so `force` sets the process-wide
/// flag) and the frame comes back plain apart from its leading clear-screen,
/// since color is off at rest.
fn frame_width(players: u8) -> usize {
    let session = Session::new_sim(
        GameConfig::for_player_count(players),
        0,
        vec![StartingOrderChoice::Keep(0); players as usize],
        Rules::standard(),
    )
    .expect("new_sim over a standard board");
    let repl = Repl::new(session);
    let _frames = frame_flag_guard();
    screen::force(true);
    let frame = screen::wrap(&repl, "");
    screen::force(false);
    assert!(frame.starts_with(CLEAR_HOME), "frames must be on for this measurement");
    frame[CLEAR_HOME.len()..]
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
}

/// The composed frame's rendered HEIGHT when a game opens: exactly what
/// `main.rs` prints first, `wrap(repl, repl.prompt())`. Measured off the real
/// `screen::wrap`, so it can never be a copied literal.
fn frame_height(players: u8) -> usize {
    let session = Session::new_sim(
        GameConfig::for_player_count(players),
        0,
        vec![StartingOrderChoice::Keep(0); players as usize],
        Rules::standard(),
    )
    .expect("new_sim over a standard board");
    let repl = Repl::new(session);
    let _frames = frame_flag_guard();
    screen::force(true);
    let frame = screen::wrap(&repl, &repl.prompt());
    screen::force(false);
    assert!(frame.starts_with(CLEAR_HOME), "frames must be on for this measurement");
    frame.lines().count()
}

/// (g) §2 states the composed screen's HEIGHT as well as its width (F3, Codex
/// adversarial review of PR #4). The guide told testers how wide to size a
/// window and nothing at all about how tall, while the 4-player screen -- the
/// tallest, since its 108-column board stacks the players panel underneath
/// rather than beside -- opened past 50 rows, scrolling the title and the top
/// board row away before the game began. Derived from `screen::wrap`, never
/// copied, so a layout change that costs rows must restate the number.
#[test]
fn section2_frame_heights_match_the_composed_screen() {
    let (h2, h3, h4) = (frame_height(2), frame_height(3), frame_height(4));
    let phrase = format!("{h2} rows at 2 players, {h3} at 3, and {h4} at 4");
    for (file, text) in [("cli-guide.md", GUIDE_MD), ("cli-guide.html", GUIDE_HTML)] {
        assert!(
            text.contains(&phrase),
            "{file} §2 does not state the composed screen's actual opening height. screen::wrap \
             produces {h2}/{h3}/{h4} rows at 2/3/4 players, so §2 must read {phrase:?}. Update the \
             guide (both formats) if the frame layout changed."
        );
    }
}

/// (f) The §2 width guidance states the width of the COMPOSED FRAME, not just
/// the board's own grid. The frame is wider than the board at every player
/// count (panel beside it, 96-column info strip below it), so board-only
/// guidance under-states what the terminal needs and a tester on an 80-column
/// window gets a wrapped screen with no warning (final whole-branch review,
/// M-4). Derived from `screen::wrap`, never copied.
#[test]
fn section2_frame_widths_match_the_composed_screen() {
    let (f2, f3, f4) = (frame_width(2), frame_width(3), frame_width(4));
    let phrase = format!("{f2} columns at 2 players, {f3} at 3, and {f4} at 4");
    for (file, text) in [("cli-guide.md", GUIDE_MD), ("cli-guide.html", GUIDE_HTML)] {
        assert!(
            text.contains(&phrase),
            "{file} §2 does not state the composed screen's actual widths. screen::wrap produces \
             {f2}/{f3}/{f4} columns at 2/3/4 players, so §2 must read {phrase:?}. Update the guide \
             (both formats) if the frame layout changed."
        );
    }
}

/// (d) The §5 command block is text-identical between the two formats -- the
/// strongest parity pin, on the most drift-prone section. Only the HTML
/// `<pre><code>` wrapper and entity escaping are stripped; everything inside
/// must match byte-for-byte.
#[test]
fn section5_command_block_is_identical_across_formats() {
    let md = md_code_block_after("## 5. Commands");
    let html = html_pre_block_after("<h2>5. Commands</h2>");
    assert_eq!(
        md, html,
        "cli-guide.md and cli-guide.html §5 command blocks diverged. They must stay byte-identical \
         (modulo the HTML <pre><code> wrapper + entity escaping); reconcile BOTH against HELP_TEXT \
         (crates/yokohama-cli/src/repl.rs)."
    );
}

/// (e) The environment/persistence promises §2 makes a tester -- the `NO_COLOR`
/// switch and per-move autosave -- are present in BOTH formats.
#[test]
fn section2_environment_promises_are_in_both_guides() {
    for (file, text) in [("cli-guide.md", GUIDE_MD), ("cli-guide.html", GUIDE_HTML)] {
        assert!(
            text.contains("NO_COLOR"),
            "{file} no longer mentions NO_COLOR -- §2 documents it as the color-off switch \
             (authority: style::init_from_tty's NO_COLOR check)."
        );
        assert!(
            text.contains("autosaves after"),
            "{file} no longer states the game autosaves after every applied move -- §2 promises it \
             (authority: the REPL's post-apply autosave)."
        );
    }
}
