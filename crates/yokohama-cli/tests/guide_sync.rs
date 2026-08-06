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
use yokohama_cli::suggest::{RUNNER_UPS_LABEL, TABLE_HEADINGS, TOTAL_LABEL};
use yokohama_cli::{render, screen, suggest};
use yokohama_core::{eval, greedy, lookahead, GameConfig, Rules, StartingOrderChoice};

const GUIDE_MD: &str = include_str!("../../../docs/cli-guide.md");
const GUIDE_HTML: &str = include_str!("../../../docs/cli-guide.html");

/// The clear-screen + cursor-home escape every composed frame opens with
/// (`screen::wrap`); stripped before measuring a frame's visible width.
const CLEAR_HOME: &str = "\u{1b}[2J\u{1b}[H";

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
    // No legal list: a board's WIDTH does not depend on which tiles carry a
    // `◈`, and this measurement is about geometry alone.
    render::board_grid(session.state(), session.rules(), &[])
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
/// sizes the frame to its widest element, and WHICH element that is differs by
/// player count -- at 2 the players panel joined beside the board (the only
/// count where it is beside it at all), at 3 the site-slot legend, at 4 the
/// board itself. So this is the width a tester's terminal actually has to be --
/// the number §2 owes them. Measured off the real `screen::wrap`, so it can
/// never be a copied literal. Frames are forced on (per-thread, so parallel
/// measurements stay hermetic without any lock) and the frame comes back plain
/// apart from its leading clear-screen, since color is off at rest.
fn frame_width(players: u8) -> usize {
    let session = Session::new_sim(
        GameConfig::for_player_count(players),
        0,
        vec![StartingOrderChoice::Keep(0); players as usize],
        Rules::standard(),
    )
    .expect("new_sim over a standard board");
    let repl = Repl::new(session);
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
/// the board's own grid. Board-only guidance under-states what the terminal
/// needs and a tester on an 80-column window gets a wrapped screen with no
/// warning (final whole-branch review, M-4). The frame is wider than the board
/// at 2 and 3 players (the panel joins beside it at 2; the site-slot legend is
/// wider than the board at 3); at 4 the 180-column board IS the widest element,
/// so the two numbers are equal there and the guidance is the same number for a
/// different reason. Derived from `screen::wrap`, never copied.
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

/// The captured screen block whose FIRST line is `opening`, from each format.
///
/// Located by content rather than by a section header: §7 prints several
/// captured screens under one header and the suggest capture is not the first,
/// so `md_code_block_after` would return the wrong one. Both formats are
/// searched the same way, which is what lets the two be compared byte for byte.
fn captured_block_starting_with(opening: &str) -> (String, String) {
    let md = GUIDE_MD
        .split("```")
        .map(|part| part.trim_matches('\n'))
        .find(|part| part.starts_with(opening))
        .unwrap_or_else(|| panic!("cli-guide.md has no captured block opening with {opening:?}"))
        .to_string();

    let mut html_blocks = GUIDE_HTML.match_indices("<pre><code>").filter_map(|(at, tag)| {
        let open = at + tag.len();
        let close = GUIDE_HTML[open..].find("</code></pre>")? + open;
        let block = unescape_html(&GUIDE_HTML[open..close]).trim_matches('\n').to_string();
        block.starts_with(opening).then_some(block)
    });
    let html = html_blocks
        .next()
        .unwrap_or_else(|| panic!("cli-guide.html has no <pre><code> opening with {opening:?}"));

    (md, html)
}

/// The opening line of the captured `suggest` screen — `render::section`'s rule
/// over the screen title, which is what makes the block findable.
const SUGGEST_BLOCK_OPENING: &str = "─ Suggest ";

/// (h) The captured `suggest` screen quotes strings the renderer OWNS, so those
/// strings are held to the code rather than to a reviewer's memory: the header's
/// knob line (every value from the const the agent itself reads), the
/// explanation table's headings AT THEIR COLUMN SPACING, its total row, and the
/// runner-up heading.
///
/// T10 shipped ~50 lines of captured screen into both guides without extending
/// this file (six-lens review) — exactly the verbatim-quoting surface this
/// module exists to pin, and the widest one the guides carry.
#[test]
fn the_captured_suggest_screen_matches_the_renderers_own_strings() {
    let (md, html) = captured_block_starting_with(SUGGEST_BLOCK_OPENING);

    // The knob line, DERIVED from the same consts `render_suggestion` reads, so
    // a rung/version/default change forces the capture to be retaken.
    let knobs = format!(
        " · {} · beam {}, K={}, M={}, evaluator impl {} · stream ",
        lookahead::LOOKAHEAD_CONTROLLER,
        lookahead::DEFAULT_BEAM,
        lookahead::DEFAULT_WORLDS,
        greedy::DEFAULT_RETREAT_SAMPLES,
        eval::EVAL_IMPL_VERSION,
    );
    // The table header row at its real column widths: this is the one line in
    // the capture where a column-width edit is otherwise invisible.
    let headings = suggest::table_row(TABLE_HEADINGS);
    let total = suggest::table_row([TOTAL_LABEL, "", "", "", ""]);
    let total = total.trim_end();

    for (file, block) in [("cli-guide.md", &md), ("cli-guide.html", &html)] {
        for (what, needle) in [
            ("the search's knob line", knobs.as_str()),
            ("the explanation table's headings", headings.as_str()),
            ("the explanation table's total row", total),
            ("the runner-up heading", RUNNER_UPS_LABEL),
        ] {
            assert!(
                block.contains(needle),
                "{file}'s captured suggest screen no longer shows {what} as the CLI renders it \
                 ({needle:?}). Authority: crates/yokohama-cli/src/suggest.rs \
                 (render_suggestion + its pub consts). Re-capture the screen in BOTH formats."
            );
        }
    }
}

/// (i) The captured `suggest` screen is text-identical between the two formats —
/// §5's parity pin applied to the other block a tester reads line by line.
#[test]
fn the_suggest_block_is_identical_across_formats() {
    let (md, html) = captured_block_starting_with(SUGGEST_BLOCK_OPENING);
    assert_eq!(
        md, html,
        "cli-guide.md and cli-guide.html captured suggest screens diverged. They must stay \
         byte-identical (modulo the HTML <pre><code> wrapper + entity escaping); re-capture BOTH \
         from one run of `suggest`."
    );
}

/// The opening line of §4's two captured Movement screens — the phase chain
/// with `MOVEMENT` lit, which no other captured block in either guide carries.
const MOVEMENT_BLOCK_OPENING: &str = "TURN 1 · RED's turn   Additional ▸ Placement ▸  MOVEMENT";

/// (j) §4's captured Movement screens are text-identical between the two
/// formats — §5's and the suggest block's parity pin, applied to the third
/// block a tester reads line by line.
///
/// The CONTENT half lives elsewhere on purpose: `repl.rs`'s
/// `the_guide_movement_captures_are_what_the_fixtures_render` regenerates both
/// blocks from the fixtures that produce them, which needs a crate-private test
/// fixture this integration test cannot reach. Together they are the same
/// two-part shape the suggest capture has — one test says the words are the
/// driver's, this one says both formats carry the same words.
#[test]
fn the_movement_blocks_are_identical_across_formats() {
    let md: Vec<String> = GUIDE_MD
        .split("```")
        .map(|part| part.trim_matches('\n').to_string())
        .filter(|part| part.starts_with(MOVEMENT_BLOCK_OPENING))
        .collect();
    let html: Vec<String> = GUIDE_HTML
        .match_indices("<pre><code>")
        .filter_map(|(at, tag)| {
            let open = at + tag.len();
            let close = GUIDE_HTML[open..].find("</code></pre>")? + open;
            let block = unescape_html(&GUIDE_HTML[open..close]).trim_matches('\n').to_string();
            block.starts_with(MOVEMENT_BLOCK_OPENING).then_some(block)
        })
        .collect();
    assert_eq!(
        md.len(),
        2,
        "cli-guide.md must carry §4's TWO captured Movement screens (one per fixture)",
    );
    assert_eq!(
        md, html,
        "cli-guide.md and cli-guide.html §4 Movement captures diverged. They must stay \
         byte-identical (modulo the HTML <pre><code> wrapper + entity escaping); re-capture BOTH \
         from the fixture runs that produce them -- never by splicing."
    );
}

/// The opening line of §6's captured reveal screens — the live-capture banner,
/// which no other block in either guide carries.
const REVEAL_BLOCK_OPENING: &str = "LIVE TRACKING ";

/// The opening line of §6's THIRD captured screen, the guided reveal's seat
/// menu. It carries no `LIVE TRACKING` banner — it is the question asked before
/// there is a capture to banner — so it needs its own key, and without one it
/// sat outside the parity pin entirely (T8 review, NOTE-3).
const SEAT_MENU_BLOCK_OPENING: &str = "Whose starting order just became public?";

/// (k) §6's captured reveal screens are text-identical between the two formats
/// — §5's and the Movement captures' parity pin, applied to the screens a
/// tester meets on their first live game (plan T8).
///
/// **All THREE of them.** §6 captures the bound Port refill, the guided seat
/// menu and the guided card question; the first and third share the live-capture
/// banner, and keying on that alone left the seat menu — a screen a tester reads
/// line by line like any other — unpinned in the format they are most likely to
/// read.
///
/// The CONTENT half lives in `reveal.rs`'s
/// `the_guide_reveal_captures_are_what_the_fixtures_render`, which regenerates
/// each block from the fixture that produces it — the same two-part shape the
/// Movement captures have, and for the same reason: one test says the words are
/// the driver's, this one says both formats carry the same words.
#[test]
fn the_reveal_blocks_are_identical_across_formats() {
    let md: Vec<String> = GUIDE_MD
        .split("```")
        .map(|part| part.trim_matches('\n').to_string())
        .filter(|part| part.starts_with(REVEAL_BLOCK_OPENING))
        .collect();
    let html: Vec<String> = GUIDE_HTML
        .match_indices("<pre><code>")
        .filter_map(|(at, tag)| {
            let open = at + tag.len();
            let close = GUIDE_HTML[open..].find("</code></pre>")? + open;
            let block = unescape_html(&GUIDE_HTML[open..close]).trim_matches('\n').to_string();
            block.starts_with(REVEAL_BLOCK_OPENING).then_some(block)
        })
        .collect();
    assert_eq!(
        md.len(),
        2,
        "cli-guide.md must carry §6's TWO captured reveal screens (the bound Port refill and \
         the guided starting-order question)",
    );
    assert_eq!(
        md, html,
        "cli-guide.md and cli-guide.html §6 reveal captures diverged. They must stay \
         byte-identical (modulo the HTML <pre><code> wrapper + entity escaping); re-capture BOTH \
         from the fixture runs that produce them -- never by splicing."
    );

    // The third screen, which carries no banner to be found by.
    let (menu_md, menu_html) = captured_block_starting_with(SEAT_MENU_BLOCK_OPENING);
    assert_eq!(
        menu_md, menu_html,
        "cli-guide.md and cli-guide.html §6 seat-menu captures diverged. They must stay \
         byte-identical (modulo the HTML <pre><code> wrapper + entity escaping); re-capture BOTH \
         from the fixture run that produces them -- never by splicing."
    );
}

/// (e) The environment/persistence promises §2 makes a tester -- the `NO_COLOR`
/// switch, per-move autosave, and (T9) the two the line editor adds -- are
/// present in BOTH formats.
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
        // The T9 promises are here rather than in prose alone. Two of them are
        // guarantees a tester at a LIVE table would act on: one says their
        // opponents' revealed cards do not outlive the game on disk, the other
        // says the key they reach for to abandon a mistyped line cannot play a
        // move for them. The third is the FEATURE the guide sells hardest --
        // recall -- and it is here because it was, until the T9 review, a claim
        // no test held: flipping rustyline's `auto_add_history` off left the
        // suite green and both guides promising a key that did nothing.
        assert!(
            text.contains("walk back through the lines you have already entered this session"),
            "{file} no longer promises ↑/↓ recall over the session's own lines -- §2 sells that as \
             the reason a long area id or card name need only be typed once (authority: \
             input::next_edited hands every DELIVERED line to Editing::remember; pinned by \
             input::tests::a_delivered_line_enters_the_history_and_an_abandoned_one_never_does)."
        );
        assert!(
            text.contains("never written to a file"),
            "{file} no longer promises the input history stays off disk -- §2 makes that promise \
             to live-table testers, whose typed lines name hidden cards (authority: the CLI takes \
             rustyline with default-features off, so `DefaultHistory` IS `MemHistory`; pinned by \
             input::tests::history_is_in_memory_by_construction)."
        );
        assert!(
            text.contains("it does not quit, and it never applies anything"),
            "{file} no longer states what Ctrl-C does -- §2 promises it abandons the typed line \
             without applying anything (authority: input::classify maps Interrupted to a re-read, \
             deliberately NOT to an empty line, which would be the forced-option bare Enter)."
        );
    }
}
