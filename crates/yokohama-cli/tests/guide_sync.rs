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
use yokohama_cli::style::glyph;
use yokohama_cli::suggest::{HONESTY_NOTE, NONZERO_SUMMARY_TAIL, RUNNER_UPS_LABEL, TABLE_HEADINGS, TOTAL_LABEL};
use yokohama_cli::{render, screen, suggest};
use yokohama_core::{eval, Contestant, GameConfig, NamedIdentity, Rules, StartingOrderChoice};

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
/// produces. If the grid geometry changes (e.g. `board_render::TILE_W`, or a
/// board's row lengths), the computed widths move and the guide must follow --
/// the numbers are derived here, never copied.
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
/// explanation table's headings AT THEIR COLUMN SPACING, its total row, the
/// runner-up heading, and — since branch-4 — the nonzero-product summary's
/// tail plus the marker itself in the lead column the renderer puts it in.
///
/// T10 shipped ~50 lines of captured screen into both guides without extending
/// this file (six-lens review) — exactly the verbatim-quoting surface this
/// module exists to pin, and the widest one the guides carry.
#[test]
fn the_captured_suggest_screen_matches_the_renderers_own_strings() {
    let (md, html) = captured_block_starting_with(SUGGEST_BLOCK_OPENING);

    // The identity line, DERIVED from the same values `render_suggestion` reads,
    // so a contestant/version/default change forces the capture to be retaken.
    // The NAME comes off the production contestant's own descriptor and the
    // three knobs off `NamedIdentity::KNOBS` — the 2026-08-30 ship commit moved
    // both from the bare `lookahead-v1` consts, and a needle left on the old
    // ones would have gone on passing a capture of a seat nothing plays.
    let knobs = format!(
        " · {} · beam {}, K={}, M={}, evaluator impl {} · stream ",
        Contestant::PRODUCTION.descriptor().0,
        NamedIdentity::KNOBS.beam,
        NamedIdentity::KNOBS.worlds,
        NamedIdentity::KNOBS.retreat_samples,
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
            ("the nonzero-product summary", NONZERO_SUMMARY_TAIL),
            ("the honesty note, line 1", HONESTY_NOTE[0]),
            ("the honesty note, line 2", HONESTY_NOTE[1]),
            ("the honesty note, line 3", HONESTY_NOTE[2]),
        ] {
            assert!(
                block.contains(needle),
                "{file}'s captured suggest screen no longer shows {what} as the CLI renders it \
                 ({needle:?}). Authority: crates/yokohama-cli/src/suggest.rs \
                 (render_suggestion + its pub consts). Re-capture the screen in BOTH formats."
            );
        }

        // The MARKER, pinned in the column the renderer puts it in — the lead
        // character of a row, which is the one thing about it a substring
        // needle would not see. The fixture's own table has nonzero rows (the
        // summary needle above counts them), so this is not vacuous.
        assert!(
            block.lines().any(|line| line.starts_with(glyph::NONZERO_MARK)),
            "{file}'s captured suggest screen shows no nonzero-product marker in its lead \
             column. Authority: crates/yokohama-cli/src/suggest.rs (marked_table_row). \
             Re-capture the screen in BOTH formats."
        );
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

// ---------------------------------------------------------------------------
// §5 and §8 pins (T12). The T12 audit found SIX stale claims in §8 and one
// unpinned-by-luck agreement in §5; nothing in this file read either section.
// These four close the drift sites the audit actually found, rather than §8 in
// general -- a section of running prose is not mechanically pinnable, and a
// test pretending otherwise would rot into noise.
// ---------------------------------------------------------------------------

/// Command lines on ONE footing: `HELP_TEXT` opens with a `Commands:` banner
/// and indents its lines two spaces, the guide's fenced block does neither.
/// Both are normalized here — banner and blank lines dropped, one leading two-
/// space indent removed — and nothing further, so the INTERIOR column
/// alignment every line carries is still compared.
fn section5_lines(block: &str) -> Vec<String> {
    block
        .lines()
        .map(|l| l.strip_prefix("  ").unwrap_or(l).trim_end().to_string())
        .filter(|l| !l.is_empty() && l != "Commands:")
        .collect()
}

/// (l) §5 is `HELP_TEXT` VERBATIM, description text included.
///
/// (a) above compares only the first token of each line, and (d) compares the
/// two formats against each other -- so between them a reworded `HELP_TEXT`
/// description could drift away from the guide with the suite green. The T12
/// audit found the two still agreeing on all 35 lines by luck; this makes it a
/// property. Authority is `repl::HELP_TEXT`.
#[test]
fn section5_is_help_text_verbatim() {
    let help = section5_lines(HELP_TEXT);
    let guide = section5_lines(&md_code_block_after("## 5. Commands"));
    assert!(help.len() > 15, "HELP_TEXT parse produced only {} lines", help.len());
    assert_eq!(
        guide, help,
        "cli-guide.md §5 is no longer HELP_TEXT verbatim -- the guide's command block is a \
         transcript of the in-app `help`, so a reworded description in repl.rs::HELP_TEXT is \
         carried into the guide in the same commit (authority: repl::HELP_TEXT)."
    );
}

/// The slice of a guide between one heading and the next, so a §8.4 assertion
/// cannot be satisfied by §4 prose that happens to use the same words.
fn section_slice<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let from = text
        .find(start)
        .unwrap_or_else(|| panic!("a guide is missing the heading {start:?}"));
    let rest = &text[from..];
    let to = rest[start.len()..]
        .find(end)
        .map(|i| i + start.len())
        .unwrap_or(rest.len());
    &rest[..to]
}

/// (m) §8.4's phase table names every phase exactly as `describe::phase_name`
/// does. The table is what a tester maps the rulebook's turn structure onto, so
/// a renamed phase that lands only in code makes it a lie. Swept over the whole
/// `PHASE_CHAIN` rather than a copied list, so a tenth phase cannot slip past.
///
/// **Matched as a WHOLE table cell, inside §8.4 only.** A bare `contains(name)`
/// over the document passes on a renamed phase whenever the old name is a
/// PREFIX of the new one -- `"5-Power bonus"` is still found inside
/// `"5-Power bonuses"` -- which is this branch's recurring overlapping-text
/// defect, and it survived this test's own first mutation. The closing
/// delimiter is what makes the match exact.
#[test]
fn section8_phase_names_match_the_describer() {
    let chain = yokohama_cli::screen_model::PHASE_CHAIN;
    assert_eq!(chain.len(), 9, "PHASE_CHAIN changed length; §8.4's table is sized to it");
    let md = section_slice(GUIDE_MD, "### 8.4", "### 8.5");
    let html = section_slice(GUIDE_HTML, "<h3>8.4", "<h3>8.5");
    for phase in chain {
        let name = yokohama_cli::describe::phase_name(phase);
        for (file, text, cell) in [
            ("cli-guide.md", md, format!("`{name}`")),
            ("cli-guide.html", html, format!("<code>{name}</code>")),
        ] {
            assert!(
                text.contains(&cell),
                "{file} §8.4 no longer names the phase {name:?} as a whole table cell -- the \
                 table maps the CLI's phase names onto the rulebook's turn structure (authority: \
                 describe::phase_name)."
            );
        }
    }
}

/// (n2) §4's `why` section states BOTH grammars and the refusal that makes the
/// command trustworthy, in both formats (W5 task 5).
///
/// The mechanical half of the guide pair for a NEW command. (a)/(d)/(l) already
/// hold §5's one-line entry to `HELP_TEXT` verbatim in both formats; what they
/// cannot see is the prose that teaches the command — and `why`'s prose carries
/// two claims a tester will act on: that `why <n>` takes the LOG's own number
/// (so it has to name the log), and that a reconstruction which disagrees with
/// the record REFUSES rather than showing an account of a move nobody played.
/// Test (b)'s shape, scoped to §4 so a §5 line cannot satisfy a §4 assertion —
/// this file's own named-check lesson.
#[test]
fn section4_states_the_why_grammars_and_its_mismatch_refusal() {
    let md = section_slice(GUIDE_MD, "## 4.", "## 5.");
    let html = unescape_html(section_slice(GUIDE_HTML, "<h2>4.", "<h2>5."));
    for needle in ["why <n>", "the number `log` prints", "refuses and names both moves"] {
        // The `.md` writes code spans with backticks and the `.html` with
        // <code>; both are stripped to the same words before comparing, which
        // is the "word-identical modulo markup" rule this pair follows.
        let plain = needle.replace('`', "");
        for (file, text) in [("cli-guide.md", md.replace('`', "")), ("cli-guide.html", strip_code_tags(&html))] {
            assert!(
                text.contains(&plain),
                "{file} §4 no longer says {plain:?} about `why` — the section teaches the \
                 command's two grammars and its refuse-on-mismatch guarantee, and both formats \
                 carry it word for word (authority: crates/yokohama-cli/src/why.rs)."
            );
        }
    }
}

/// `<code>`/`</code>`/`<strong>`/`</strong>`/`<em>`/`</em>` removed, so HTML
/// prose can be compared against the markdown's backtick-stripped twin.
fn strip_code_tags(s: &str) -> String {
    let mut out = s.to_string();
    for tag in ["<code>", "</code>", "<strong>", "</strong>", "<em>", "</em>"] {
        out = out.replace(tag, "");
    }
    out
}

/// (n) The choice-option strings §8.6 QUOTES are strings `describe_move`
/// actually produces. Every one of these is a constant arm -- no state reaches
/// its wording -- so a fresh session is enough to render them, and an editorial
/// pass over `describe.rs` that forgets the guide trips here. The T12 audit
/// found §8.6 quoting a warehouse banner two rewrites out of date.
///
/// **Scoped to §8.6, and it has to be** (six-lens F6). This searched the WHOLE
/// document while its failure message spoke about §8.6, so any needle with a
/// twin elsewhere was unpinned: deleting `Decline the assistant move` from
/// §8.6 item 3 left the guide's own inventory wrong and the suite green,
/// because §4's cross-reference to the same option satisfied the `contains`.
/// That is the branch's NAMED CHECK -- an assertion a DIFFERENT line satisfies
/// -- landing in the test written to close it, and the slicing helper the
/// sibling test uses was already three tests up.
#[test]
fn section8_choice_options_are_what_the_describer_says() {
    use yokohama_core::Move;
    let session = Session::new_sim(
        GameConfig::for_player_count(2),
        0,
        vec![StartingOrderChoice::Keep(0); 2],
        Rules::standard(),
    )
    .expect("new_sim over a standard board");

    for mv in [
        Move::DeclineWarehouseTake,
        Move::DeclineAssistantMove,
        Move::ChooseRecoveryLeave { leave_one: true },
        Move::ChooseRecoveryLeave { leave_one: false },
        Move::UseUniversity,
        Move::UseSpinningMill,
    ] {
        let words = yokohama_cli::describe::describe_move(&mv, session.state(), session.rules());
        // §8.6 is the LAST subsection, so the delimiter is §9's heading --
        // `section_slice` falling back to end-of-document would put §9 inside
        // the slice and re-open exactly the hole this scoping closes.
        for (file, text) in [
            ("cli-guide.md", section_slice(GUIDE_MD, "### 8.6", "## 9.")),
            ("cli-guide.html", section_slice(GUIDE_HTML, "<h3>8.6", "<h2>9.")),
        ] {
            assert!(
                unescape_html(text).contains(&words),
                "{file} §8.6 no longer quotes {words:?} -- §8.6 lists what each queued choice \
                 offers, in the tool's own words (authority: describe::describe_move)."
            );
        }
    }
}

/// (o) §8.3's claim about the `players` panel's goods letters is the panel's
/// real vocabulary.
///
/// The audit found the table selling a letter (`i` for imports) that the panel
/// has never printed, alongside `imp`, a token no shipped call site emits. Both
/// survived because nothing read §8.3. Rather than re-transcribe the format
/// string, this renders the panel and asserts the shape the guide describes:
/// the four goods carry letters, and imports and yen are spelled out instead.
#[test]
fn section8_goods_letters_are_what_the_players_panel_prints() {
    let session = Session::new_sim(
        GameConfig::for_player_count(2),
        0,
        vec![StartingOrderChoice::Keep(0); 2],
        Rules::standard(),
    )
    .expect("new_sim over a standard board");
    let seats = render::SeatDisplay::open(session.state().players.len());
    let panel = render::players_panel(session.state(), session.rules(), &seats);

    for letter in ["c1", "s1", "t1", "f1"] {
        assert!(
            panel.contains(letter),
            "the players panel no longer prints {letter:?}; §8.3's letter column states the set \
             c/s/t/f (authority: render::players_panel): {panel}"
        );
    }
    assert!(
        panel.contains("imports 0") && !panel.contains("i0"),
        "the players panel spells imports out and gives it NO letter, which is exactly what \
         §8.3 says after the T12 audit found the guide claiming an `i` (authority: \
         render::players_panel): {panel}"
    );
    for (file, text) in [("cli-guide.md", GUIDE_MD), ("cli-guide.html", GUIDE_HTML)] {
        assert!(
            text.contains("goods letters are exactly `c s t f`")
                || text.contains("goods letters are exactly <code>c s t f</code>"),
            "{file} §8.3 no longer states the panel's letter set (authority: \
             render::players_panel)."
        );
    }
}

/// (p) **§2's quick-start sequence actually starts a game** (six-lens F4).
///
/// §2 advertises a fastest path and gives the answers in order. It had been
/// missing one — the sim tail's `Bot seats?` question, which the wizard asks
/// after the keeps — so following the guide literally fed `confirm` to that
/// question, got `! 'confirm' is not a seat number`, ran out of input and
/// printed `setup aborted: input ended mid-wizard`. No game, on the guide's own
/// advertised fastest path, in a sentence this branch had already edited once.
///
/// Two halves, and neither alone is the property. The guide is checked to
/// QUOTE these answers in this ORDER, so a rewritten sentence that drops one
/// trips here rather than in a tester's terminal; and the sequence is then
/// PIPED INTO THE SHIPPED BINARY, so a wizard that grows, drops or reorders a
/// question trips here too. The blanks are the three Enters the sentence
/// describes in prose (seed, save path, bot seats).
#[test]
fn the_quick_start_sequence_in_section2_starts_a_game() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    /// Exactly what §2 tells a tester to type, blanks included.
    const ANSWERS: &[&str] = &["sim", "2", "red", "blue", "0", "", "", "1", "1", "", "confirm"];

    // (1) The guide quotes the typed answers, in order, in its own §2 sentence.
    let sentence = section_slice(GUIDE_MD, "**First run.**", "\n\n");
    let mut at = 0usize;
    for answer in ANSWERS.iter().filter(|a| !a.is_empty()) {
        let quoted = format!("`{answer}`");
        let found = sentence[at..].find(&quoted).unwrap_or_else(|| {
            panic!(
                "cli-guide.md §2's first-run sentence no longer quotes {quoted} in the order a \
                 tester types it (authority: the wizard's question order in wizard.rs):\n{sentence}"
            )
        });
        at += found + quoted.len();
    }
    assert!(
        sentence.contains("bot seats"),
        "cli-guide.md §2 must account for the sim tail's `Bot seats?` question — leaving it out \
         is what made this sequence abort the wizard:\n{sentence}"
    );

    // (2) And the sequence starts a game in the shipped binary. Run in a temp
    // directory, because a blank save-path answer writes the default save file
    // into the current one.
    let dir = std::env::temp_dir().join(format!("yokohama_quickstart_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let mut child = Command::new(env!("CARGO_BIN_EXE_yokohama-cli"))
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the shipped binary runs");
    let mut script: String = ANSWERS.join("\n");
    script.push_str("\nquit\n");
    child.stdin.take().expect("piped stdin").write_all(script.as_bytes()).expect("write");
    let out = child.wait_with_output().expect("the child finishes");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    std::fs::remove_dir_all(&dir).ok();

    assert!(
        !text.contains("setup aborted"),
        "§2's quick-start sequence aborts the wizard — the guide's fastest path must reach a \
         game:\n{text}"
    );
    assert!(
        text.contains("--- Setup review ---") && text.contains("TURN 1"),
        "§2's quick-start sequence must reach the setup review and then the first prompt:\n{text}"
    );
}

/// Every `--flag` token in a guide body — `--` followed by an ASCII letter,
/// then flag characters. A bare `--` (cargo's argument separator) does not
/// match, and the em-dash `—` the guides use as prose punctuation is multi-byte
/// and never `0x2d`, so it is not mistaken for a flag.
fn flag_tokens(s: &str) -> Vec<String> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 < b.len() {
        if b[i] == b'-' && b[i + 1] == b'-' && b[i + 2].is_ascii_alphabetic() {
            let start = i;
            i += 2;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'-') {
                i += 1;
            }
            out.push(s[start..i].to_string());
        } else {
            i += 1;
        }
    }
    out
}

/// F7 (lens-4): the bridge `import` argv surface is INSIDE guide_sync's slice.
/// The guide pair is authoritative and guide_sync enforces the mechanical
/// slice, but that slice covered REPL commands and never the argv flags — so
/// the guide could document a `--flag` the parser lacks, or omit one it has,
/// with the whole suite green. Both directions are pinned from the single
/// exported `import::FLAGS`. `import` is the CLI's ONLY `--flag` surface (the
/// REPL takes bare words), so every `--word` token in the guide BODY must be a
/// real import flag.
#[test]
fn the_import_argv_flags_match_the_guides() {
    use yokohama_cli::bga::import::FLAGS;

    // (a) every real flag is documented in BOTH formats.
    for flag in FLAGS {
        assert!(GUIDE_MD.contains(flag), "cli-guide.md omits the import flag {flag}");
        assert!(GUIDE_HTML.contains(flag), "cli-guide.html omits the import flag {flag}");
    }

    // (b) every `--flag` token the guides carry is a real import flag. Strip the
    // HTML `<style>` block first — its CSS custom properties (`--accent`, …) are
    // `--word` tokens that are not CLI flags and live only there.
    let html_body = GUIDE_HTML
        .find("</style>")
        .map(|i| &GUIDE_HTML[i + "</style>".len()..])
        .unwrap_or(GUIDE_HTML);
    for (name, body) in [("cli-guide.md", GUIDE_MD), ("cli-guide.html", html_body)] {
        for tok in flag_tokens(body) {
            assert!(
                FLAGS.contains(&tok.as_str()),
                "{name} documents `{tok}`, which import::parse_args does not accept as a flag"
            );
        }
    }
}
