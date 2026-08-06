//! TTY-gated composed-frame presentation (Task 6 redesign, design note
//! docs/plans/2026-07-21-cli-frontend-redesign-note.md §1-2). This module
//! owns the ONLY non-SGR ANSI the crate emits: the clear-screen +
//! cursor-home that turns the rolling line protocol into an arranged game
//! screen on a real terminal. Like `style`, it is OFF at rest -- `wrap` is
//! the identity function until `main.rs` calls `init_from_tty()` on a real
//! terminal -- so every piped run and every test stays byte-plain without
//! opting in. `NO_COLOR` deliberately does NOT disable frames (it governs
//! color only); the in-app `screen` command does.
//!
//! Frame anatomy (top to bottom): title strip, the staggered board joined
//! with the players panel (BESIDE it while the board and the designed
//! dashboard both fit `FLOOR_W`, which is 2 players on the V3 board; BELOW it
//! at 3 and 4 -- std exposes no terminal size, so what the frame itself
//! measures is the only honest signal), the board legend, a rule, then the
//! body (the step's delta/error lines plus the normal prompt block, unchanged
//! from piped mode -- nothing exists only in the frame).
//!
//! **Transitional between Phase-3.5a T2 and T4.** The board region is the
//! redesigned V3 one; the players panel and the decision zone below it are
//! still the pre-redesign surfaces, and T3 and T4 replace them in turn.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::render;
use crate::repl::Repl;
use crate::style::strip_ansi;

static ENABLED: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// Per-thread override of `ENABLED`. Unconditionally thread-local (NOT
    /// `cfg(test)`-gated): an integration test links this library without
    /// `cfg(test)`, so gating this on it would make `force` write the
    /// process-wide `ENABLED` there and let parallel frame-toggling tests race
    /// (the old `FRAME_FLAG` mutex in `guide_sync` papered over exactly that).
    /// The real CLI is single-threaded and never calls `force`, so `FORCED`
    /// stays `None` on the main thread and `enabled` falls through to `ENABLED`
    /// exactly as before -- no behavioural change to the shipped app.
    static FORCED: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Enable composed frames iff stdout is a real terminal. `main.rs` calls
/// this once, first thing; nothing else ever should.
pub fn init_from_tty() {
    use std::io::IsTerminal;
    ENABLED.store(std::io::stdout().is_terminal(), Ordering::Relaxed);
}

/// Test hook: force frames on/off regardless of TTY state. Per-thread, so
/// parallel tests (lib unit tests AND integration tests) stay hermetic;
/// production never calls it.
pub fn force(enabled: bool) {
    FORCED.with(|f| f.set(Some(enabled)));
}

pub fn enabled() -> bool {
    if let Some(forced) = FORCED.with(|f| f.get()) {
        return forced;
    }
    ENABLED.load(Ordering::Relaxed)
}

/// Display width of `s`: visible chars, ANSI escape runs excluded.
pub(crate) fn visible_width(s: &str) -> usize {
    strip_ansi(s).chars().count()
}

/// The redesign's raised layout FLOOR (Phase-3.5a spec §0.1): the frame is
/// designed at 200x50, degrading below that rather than being built for it.
///
/// It is the arrangement decision, not a cap. std exposes no terminal size, so
/// the only honest signal is what the frame itself measures: the dashboard
/// sits BESIDE the board while the two together fit the floor, and stacks
/// BELOW it when they do not. On the V3 board that reproduces §0.1's ruling
/// from the geometry rather than from a hard-wired player count — 120 + 4 + 74
/// = 198 fits at 2 players (beside), 150 and 180 do not at 3 and 4 (below),
/// which is exactly what the mocks show. Widening [`PANEL_GAP`] to the mocks'
/// four columns spent two of the four this arrangement had spare and flipped
/// nothing: the 2P join clears the floor by 2, and the 3P board is 28 columns
/// past it with the gap set to anything at all.
/// `the_dashboard_sits_beside_the_board_only_while_both_fit_the_floor` is the
/// pin.
pub(crate) const FLOOR_W: usize = 200;

/// Columns between the board and a dashboard joined beside it — the approved
/// mocks' own gutter. `mockgen.py` pads the board block to 124 columns before
/// appending the dashboard, and the V3 board is 120 wide at 2 players, so the
/// designed gutter is four columns and this constant is that number rather than
/// a value chosen here.
const PANEL_GAP: usize = 4;

/// The DESIGNED dashboard width — the approved mocks' `DASH_W`, which T3 cuts
/// the panels over to.
///
/// The arrangement is decided against this number rather than against whatever
/// the panel beside the board happens to measure TODAY, and that is deliberate
/// rather than premature. The frame is transitional between T2 and T4: the V3
/// board has landed and the old 40-column compact panel has not yet been
/// replaced. Measuring the live panel would put the 3-player board beside it
/// at 192 columns now and stack it below the moment T3 widens the panel to its
/// design — an arrangement that flips under a task that never touched layout.
/// Sizing on the design instead gives spec §0.1's ruling immediately (2P
/// beside, 3P and 4P below) and gives the same answer after T3.
const DASH_W: usize = 74;

/// The composed frame's width at each player count, indexed `players - 2` --
/// the three numbers guide §2 states, and **the budget a BODY has to fit.**
///
/// [`wrap`] sizes its title strip and its closing rule to the widest line it is
/// given, so a body line past this widens the whole game screen for one command
/// and snaps back on the next: the horizontal twin of the vertical jitter F3
/// fixed. Bodies that can outrun a board ask for this rather than inventing a
/// number (`suggest`, whose widest placement row renders at 135 columns).
///
/// **A mirrored table, not a live derivation, and the reason is a real
/// constraint:** a body is built BEFORE the frame that will hold it exists --
/// `Repl::step` returns text and `main.rs` wraps it afterwards -- so nothing at
/// body-build time can measure its own frame. What keeps a mirror honest is the
/// pin: `the_frame_width_table_matches_the_composed_opening_frame` measures
/// every entry off [`stage_width`], the same function `wrap` sizes itself with,
/// and `guide_sync` derives the same three numbers a second time from `wrap`
/// for guide §2 -- so a layout change that moves a width fails two tests rather
/// than drifting.
///
/// **These are OPENING widths.** The board, legend and info strip are fixed per
/// count; the players panel is the one part play changes, and it can leave the
/// frame WIDER later on. A body inside these numbers is therefore inside those
/// frames too. The residual is the other direction -- a panel that rendered
/// NARROWER than at the opening would drop a frame below this bound. Its
/// content starts at its minimum (0 VP, no goods, a President in hand rather
/// than a named area), so that is not expected, and repl.rs's
/// `the_suggest_screen_fits_the_composed_frame_at_every_player_count` checks a
/// real mid-game frame against this floor at each count rather than leaving the
/// claim to argument.
///
/// **T2 moved all three**, and the cause is the V3 board's 30-column tiles:
/// the board alone went 72/90/108 to 120/150/180 columns. A DIFFERENT element
/// is the widest one at each count now, which is why the three numbers no
/// longer track the board: at 4 players it is the 180-column board itself; at 3
/// it is the site-slot LEGEND, at 163 columns the one row that keys the V3
/// tiles' compact tokens and the same length at every count; at 2 it is the
/// board with the panel joined beside it, at 120 + [`PANEL_GAP`] + the panel's
/// own 40 = 164, one column past that legend. The pre-redesign numbers were
/// 114/132/108.
///
/// The 2P entry moved again in the T2 review fix wave, 163 to 164, when
/// [`PANEL_GAP`] took the approved mocks' four-column gutter in place of the
/// two it had been carrying: that join was 162 at a gap of 2, a column short of
/// the legend, and two more columns put it a column past.
const FRAME_WIDTHS: [usize; 3] = [164, 163, 180];

/// [`FRAME_WIDTHS`] for a `players`-player game (2..=4).
///
/// A count outside that range gets the NARROWEST entry, which is the safe
/// direction: text that fits the narrowest frame this driver composes fits all
/// of them. `GameConfig` admits only 2..=4, so the fallback is defence, not a
/// path -- and it returns rather than panicking, since a width helper is no
/// place to take a screen down over a rounding question.
///
/// The minimum is COMPUTED, not indexed. It used to read `FRAME_WIDTHS[2]`,
/// which was the narrowest entry only because the 4-player frame happened to
/// be -- and T2's board made it the widest, quietly turning "the safe
/// direction" into the opposite one. A derived minimum cannot be wrong-footed
/// by a layout change again.
pub(crate) fn frame_width(players: usize) -> usize {
    FRAME_WIDTHS
        .get(players.wrapping_sub(2))
        .copied()
        .unwrap_or_else(|| FRAME_WIDTHS.iter().copied().min().expect("three entries"))
}

/// The one-time startup banner `main.rs` prints on a real terminal before
/// the wizard (piped runs never see it -- the wizard transcript is a test
/// surface). Plain ASCII art, painted bold by the caller-side `paint` calls
/// being unnecessary: bold-on-TTY comes free from the strip in `wrap`.
pub fn banner() -> &'static str {
    r#"
 __   __   ___    _  __   ___    _   _      _      __  __     _
 \ \ / /  / _ \  | |/ /  / _ \  | | | |    / \    |  \/  |   / \
  \ V /  | | | | | ' /  | | | | | |_| |   / _ \   | |\/| |  / _ \
   | |   | |_| | | . \  | |_| | |  _  |  / ___ \  | |  | | / ___ \
   |_|    \___/  |_|\_\  \___/  |_| |_| /_/   \_\ |_|  |_|/_/   \_\
"#
}

/// Whether `wrap` will actually compose a frame around `repl`'s next body --
/// i.e. whether the board and legend are already on screen above it. The
/// single source of truth for that question: `wrap` short-circuits on exactly
/// this, and `Repl::board_cmd` suppresses its own board on exactly this (I5),
/// so the two can never disagree about whether a board is already visible.
pub(crate) fn frame_active(repl: &Repl) -> bool {
    enabled() && repl.composed
}

/// Compose the full TTY frame around `body` (a step's delta/error lines +
/// prompt block), or return `body` unchanged when frames are off (piped
/// mode, or toggled off via the `screen` command). The board and panel are
/// re-rendered from CURRENT session state on every wrap -- the frame is
/// always a truthful picture of the state the prompt below it offers moves
/// against.
pub fn wrap(repl: &Repl, body: &str) -> String {
    if !frame_active(repl) {
        return body.to_string();
    }
    let stage = stage(repl);
    let width = stage_width(&stage);
    let title = " Y O K O H A M A ";
    let pad = width.saturating_sub(title.chars().count());
    let strip = format!("{}{}{}", "═".repeat(pad / 2), title, "═".repeat(pad - pad / 2));

    format!("\u{1b}[2J\u{1b}[H{strip}\n{stage}{}\n{body}", "─".repeat(width))
}

/// Everything [`wrap`] composes ABOVE the body: the board (with the players
/// panel beside or below it), the board legend, the discoverability strip.
///
/// Split out of `wrap` so [`stage_width`] can be measured on its own -- the
/// [`FRAME_WIDTHS`] pin needs the width `wrap` sizes itself to, not a width
/// re-derived by parsing a finished frame back apart.
fn stage(repl: &Repl) -> String {
    let state = repl.session.state();
    let rules = repl.session.rules();
    let board = render::board_grid(state, rules);
    let board_width = board.lines().next().map(visible_width).unwrap_or(0);

    let seats = repl.seat_display();
    let mut stage = if repl.show_panel {
        if board_width + PANEL_GAP + DASH_W <= FLOOR_W {
            join_side_by_side(&board, &render::compact_panel(state, rules, &seats), PANEL_GAP)
        } else {
            // Stacked below the board, VERTICAL space is the scarce
            // resource, not horizontal: the panel folds to one line per
            // player (F3, Codex review of PR #4 -- the 5-line block cost 16
            // rows and pushed the 4P opening frame off the screen).
            let mut s = board.clone();
            if !s.is_empty() && !s.ends_with('\n') {
                s.push('\n');
            }
            s.push_str(&render::compact_panel_oneline(state, &seats));
            s
        }
    } else {
        board.clone()
    };
    if !stage.is_empty() && !stage.ends_with('\n') {
        stage.push('\n');
    }
    stage.push_str(&render::board_legend(state, rules));
    stage.push('\n');
    // Discoverability strip (owner scope item 10): every frame names the
    // game-info surfaces and the command that opens them, so a player who
    // never read the guide finds them on screen.
    stage.push_str(
        "info: type 'view' — board · players · ports · labs · tracks · sites · goals · hand · score · log\n",
    );
    stage
}

/// The width [`wrap`] sizes a frame to: its stage's widest line, floored at 60
/// so a degenerate board still draws a strip worth looking at.
fn stage_width(stage: &str) -> usize {
    stage.lines().map(visible_width).max().unwrap_or(0).max(60)
}

/// Join two blocks line-by-line, `left` padded to its own max visible width
/// plus `gap` spaces. Padding is computed on VISIBLE width so painted lines
/// never shear the seam (style.rs's alignment rule).
fn join_side_by_side(left: &str, right: &str, gap: usize) -> String {
    let l: Vec<&str> = left.lines().collect();
    let r: Vec<&str> = right.lines().collect();
    let lw = l.iter().map(|s| visible_width(s)).max().unwrap_or(0);
    let mut out = String::new();
    for i in 0..l.len().max(r.len()) {
        let lline = l.get(i).copied().unwrap_or("");
        let rline = r.get(i).copied().unwrap_or("");
        out.push_str(lline);
        out.push_str(&" ".repeat(lw - visible_width(lline) + gap));
        out.push_str(rline);
        while out.ends_with(' ') {
            out.pop();
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
pub(crate) mod test_support {
    /// Panic unless EVERY line of `lines` renders at exactly `width` visible
    /// columns (Phase-3.5a spec §0.4: "every tile builder line is exactly
    /// `TILE_W` — asserted in tests").
    ///
    /// Width is measured through [`super::visible_width`], the crate's one
    /// escape-stripping measurement, so a painted line and its plain twin
    /// measure the same — which is precisely the property a hand-rolled
    /// `.len()` check would miss. BOTH directions fail: a short line shears a
    /// side-by-side join exactly as an over-long one does.
    ///
    /// The message names the four facts that turn a width failure into a
    /// one-look fix: which line, what was expected, what was measured, and the
    /// offending text with its escapes stripped (a raw painted string in a
    /// failure message is unreadable).
    ///
    /// KNOWN RESIDUAL (T0-review SF3), now discharged where it can be: the
    /// measurement is a `char` COUNT, not a terminal COLUMN count. The frames
    /// draw no East-Asian-Wide glyph at all -- T0's note named `★` as one and
    /// it is not (U+2605 is Ambiguous) -- but eighteen of the vocabulary's
    /// twenty-one NAMED glyphs are Ambiguous-width (`▲ ● ◈ ▶ ★ ■ ↑ · …`, the
    /// two dashes, the six box characters; `·` is two of the eighteen, being
    /// both SEPARATOR and EMPTY_SLOT), counting 1 here while rendering 2 on a
    /// CJK-configured terminal or font. [`crate::style::glyph`] is the
    /// vocabulary, the per-glyph width pins, and the recorded decision to
    /// accept that class for the raised-floor design; read it for why a line
    /// carrying the intended number of CHARACTERS carries the intended number
    /// of COLUMNS. This guard alone does not prove the second thing.
    ///
    /// Test-support for now because T0 ships no emitter that calls it; the
    /// renderer tasks that build fixed-width tiles and cards are its callers.
    pub(crate) fn assert_exact_width<S: AsRef<str>>(lines: &[S], width: usize) {
        for (i, line) in lines.iter().enumerate() {
            let text = line.as_ref();
            let measured = super::visible_width(text);
            assert!(
                measured == width,
                "line {i} is {measured} columns wide, expected {width}: {:?}",
                crate::style::strip_ansi(text),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repl::Repl;
    use crate::session::Session;
    use yokohama_core::*;

    fn repl_np(n: u8) -> Repl {
        Repl::new(
            Session::new_sim(
                GameConfig::for_player_count(n),
                7,
                vec![StartingOrderChoice::Keep(0); n as usize],
                Rules::standard(),
            )
            .unwrap(),
        )
    }

    /// Frames are the identity at rest: a piped run (frames off) sees the
    /// body byte-for-byte, no clear-screen, no chrome.
    #[test]
    fn wrap_is_identity_when_frames_are_off() {
        let r = repl_np(2);
        force(false);
        assert_eq!(wrap(&r, "hello\n> prompt\n"), "hello\n> prompt\n");
    }

    /// The composed frame leads with clear-screen + home, carries the title
    /// strip, the board, the panel, the legend, and ends with the body.
    #[test]
    fn wrap_composes_clear_title_board_panel_legend_body() {
        let r = repl_np(2);
        force(true);
        let f = wrap(&r, "BODY-MARKER\n> hint\n");
        force(false);
        assert!(f.starts_with("\u{1b}[2J\u{1b}[H"), "clear+home must lead the frame");
        assert!(f.contains("Y O K O H A M A"), "{f}");
        assert!(f.contains("┌"), "board expected in frame: {f}");
        assert!(f.contains("Players"), "panel expected in frame: {f}");
        assert!(f.contains("site slots"), "legend expected: {f}");
        assert!(
            f.contains("info: type 'view'"),
            "the discoverability strip must be in every frame (owner scope item 10): {f}"
        );
        assert!(f.ends_with("BODY-MARKER\n> hint\n"), "body must close the frame: {f}");
    }

    /// Spec §0.1's arrangement ruling, reproduced from the geometry rather
    /// than hard-wired: at 2 players the 120-column V3 board plus the panel
    /// fits [`FLOOR_W`], so the dashboard joins SIDE BY SIDE -- some line
    /// carries both a board border and panel text. At 3 and 4 players the
    /// board alone is 150 and 180 columns, so it stacks BELOW and no line
    /// carries both.
    #[test]
    fn the_dashboard_sits_beside_the_board_only_while_both_fit_the_floor() {
        force(true);
        // A line carrying BOTH a tile edge and panel-only text is the join.
        // Any of the four box characters counts, not `│` alone: V3 tiles are
        // five lines, so a panel row lands on a tile's top or bottom BORDER as
        // readily as on its interior.
        let joined = |frame: &str| {
            frame.lines().any(|l| {
                l.contains("President:") && ['┌', '┐', '└', '┘', '│'].iter().any(|c| l.contains(*c))
            })
        };

        let r2 = repl_np(2);
        let f2 = wrap(&r2, "");
        assert!(joined(&f2), "2P panel must sit beside the board:\n{f2}");

        for n in [3u8, 4] {
            let r = repl_np(n);
            let f = wrap(&r, "");
            assert!(
                !joined(&f),
                "the {n}P panel must stack below a board wider than the floor allows beside it:\n{f}"
            );
            assert!(f.contains("Players"), "{n}P panel still present, just below:\n{f}");
        }

        let r4 = repl_np(4);
        let f4 = wrap(&r4, "");
        // ...and stacked, it is the ONE-LINE-per-player fold (F3), so the
        // panel costs 4 rows here, not 20: every player's President line is
        // the same line as their name.
        assert_eq!(
            f4.lines().filter(|l| l.contains("President:")).count(),
            4,
            "the stacked panel must be one line per player:\n{f4}"
        );
        assert!(
            f4.lines().any(|l| l.contains("Red") && l.contains("President:")),
            "name and President must share one line in the stacked form:\n{f4}"
        );
        force(false);
    }

    /// [`FRAME_WIDTHS`] is a mirror, so it gets a pin: every entry is the width
    /// `wrap` actually sizes an opening frame to at that player count, measured
    /// through the very function `wrap` uses. A layout change that moves a
    /// frame's width fails HERE, next to the table, rather than silently
    /// licensing a `suggest` body that is now too wide for its screen.
    ///
    /// The out-of-range fallback is pinned in the same place as the values it
    /// falls back to: it must be the NARROWEST entry, because the whole point of
    /// the fallback is that text fitting it fits every frame.
    #[test]
    fn the_frame_width_table_matches_the_composed_opening_frame() {
        for n in [2u8, 3, 4] {
            let r = repl_np(n);
            force(true);
            let measured = stage_width(&stage(&r));
            force(false);
            assert_eq!(
                frame_width(n as usize),
                measured,
                "the {n}P frame composes at {measured} columns, so FRAME_WIDTHS must say so -- \
                 and guide §2 (checked independently by guide_sync) must state the same number"
            );
        }
        let narrowest = FRAME_WIDTHS.iter().copied().min().expect("three entries");
        assert_eq!(frame_width(9), narrowest, "an out-of-range count must fall back to the narrowest");
        assert_eq!(frame_width(0), narrowest, "...including a count below the range");
    }

    /// F3 (Codex adversarial review of PR #4): the composed screen has to
    /// FIT the terminal the guide tells a tester to open. The opening frame
    /// is exactly `wrap(repl, repl.prompt())` (`main.rs`), so measure that.
    /// At 4 players the board is 108 columns and the panel stacks BELOW it;
    /// with the 5-line-per-player block that stack ran the frame past 50
    /// rows, and in a real 140x50 PTY the title strip and the whole top
    /// board row had already scrolled off before the player could read
    /// them. Every player count must open inside 50 rows.
    #[test]
    fn opening_frame_fits_fifty_rows_at_every_player_count() {
        for n in [2u8, 3, 4] {
            let r = repl_np(n);
            force(true);
            let frame = wrap(&r, &r.prompt());
            force(false);
            let rows = frame.lines().count();
            assert!(rows <= 50, "the {n}P opening frame is {rows} rows; 50 is the documented height");
        }
    }

    /// A frame over a deliberately WIDE game position: turn 99, 3-digit
    /// money and VP, 2-digit goods on every seat, and whatever `mutate` adds
    /// (a pending-choice head, a phase, a lockdown). Built through
    /// `for_test_from_state` because no short legal move sequence reaches a
    /// 199-VP mid-game position, and the width bound has to hold there.
    fn loaded_repl(n: u8, mutate: impl FnOnce(&mut GameState)) -> Repl {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(n), &rules, 7).unwrap();
        let mut state =
            finalize_setup(pending, &vec![StartingOrderChoice::Keep(0); n as usize]).unwrap();
        state.turn_number = 99;
        for p in &mut state.players {
            p.money = 199;
            p.victory_points = 199;
            p.copper = 15;
            p.silk = 15;
            p.tea = 15;
            p.seafood = 15;
            p.imported_goods = 15;
        }
        mutate(&mut state);
        Repl::new(Session::for_test_from_state(state, rules))
    }

    /// Every frame the width pin below has to hold for, labelled so a
    /// failure names WHICH screen blew the bound.
    fn worst_case_frames(n: u8) -> Vec<(String, String)> {
        let mut out = Vec::new();
        force(true);
        let opening = repl_np(n);
        out.push(("opening".to_string(), wrap(&opening, &opening.prompt())));

        for phase in [TurnPhase::Placement, TurnPhase::Movement, TurnPhase::Building] {
            let r = loaded_repl(n, |s| s.turn_phase = phase);
            out.push((format!("loaded {phase:?}"), wrap(&r, &r.prompt())));
        }

        // Every pending-choice head: the banner text is the longest thing
        // that ever lands in the header's phase slot.
        for choice in [
            PendingChoice::AnyGood,
            PendingChoice::WarehouseTake,
            PendingChoice::AssistantMove,
            PendingChoice::ChurchSpace { total_faith: 11 },
            PendingChoice::CustomsSpace { cost: 4 },
            PendingChoice::RecoveryLeave,
            PendingChoice::StationPlace,
        ] {
            let label = format!("loaded choice {choice:?}");
            let r = loaded_repl(n, |s| {
                s.turn_phase = TurnPhase::AreaAction;
                // A full queue, so the "N choices pending" tail is at its
                // widest too.
                s.pending_choices = vec![choice; 4];
            });
            out.push((label, wrap(&r, &r.prompt())));
        }

        // Lockdown: the status line replaces the whole prompt, and it
        // carries a recorded desync reason.
        let mut locked = loaded_repl(n, |_| {});
        locked.engage_lockdown(
            "desync: revealed order card(s) did not match the board after applying".to_string(),
        );
        // `prompt()` IS the lockdown status line once `locked` is set -- pass
        // it, or this case measures an empty body and proves nothing.
        out.push(("lockdown".to_string(), wrap(&locked, &locked.prompt())));

        force(false);
        out
    }

    /// How far past its OPENING width ([`frame_width`]) a composed frame may
    /// grow in play before the two width pins below call it a defect.
    ///
    /// **MEASURED, then set — and it came back SMALLER than the review that
    /// ordered it expected, so this is one column rather than the double
    /// figures a growing panel would have wanted.** [`FRAME_WIDTHS`] warns that
    /// a mid-game players panel can leave a frame wider than its opening width,
    /// and covering that is what this margin is for. Instrumented over both
    /// pins below — every constructed worst case (turn 99, 199 money and VP, 15
    /// of every good on every seat, all seven pending-choice banners, a
    /// lockdown) and a real 250-move game walk, at each count — the overshoot
    /// is nowhere above zero:
    ///
    /// | count | opening | widest measured | overshoot |
    /// |---|---|---|---|
    /// | 2 | 164 | 164 | 0 |
    /// | 3 | 163 | 163 | 0 |
    /// | 4 | 180 | 180 | 0 |
    ///
    /// A frame reaches its opening width and stops, because at no count is the
    /// widest element one that play can grow: at 4 the 180-column board is
    /// wider than anything stacked under it, at 3 the fixed site-slot legend
    /// is, and at 2 the panel joined beside the board does set the width — but
    /// the panel's own widest line is a fixed-width goods row that three-digit
    /// money and VP still fit inside. Hence one column, just above the worst
    /// measured overshoot.
    ///
    /// It is a MARGIN, not a second floor: what it has to stay is a bound a
    /// growing line can actually break. [`FLOOR_W`] alone leaves 20 to 37
    /// columns of slack depending on the count, which is more room than the two
    /// lines the original pin caught (the 142-column Movement hint and the
    /// 150-column choice status line) ever needed to hide in; at one column of
    /// allowance that class of growth fails again. If T3's dashboard makes the
    /// panel the widest element at a count play can then widen, this constant
    /// gets RE-MEASURED — not raised to whatever silences it.
    const FRAME_MARGIN: usize = 1;

    /// Both width bounds on one composed frame, reported rather than asserted.
    ///
    /// Reported, because a run that stops at the first failing count cannot
    /// tell you whether the other two also moved -- and when the bound is a
    /// measured constant, "what did every count actually reach" is the fact
    /// that sets it. Returns the frame's widest visible line and pushes one
    /// message per broken bound onto `report`.
    ///
    /// The two bounds are checked SEPARATELY on purpose. [`FLOOR_W`] is the
    /// design's hard cap and stays stated even while the per-count bound is the
    /// tighter of the two at every count today; a MARGIN raised past the slack
    /// a count has under the floor would start failing on the floor instead,
    /// which is the right failure to get.
    fn measure_frame(n: u8, label: &str, frame: &str, report: &mut Vec<String>) -> usize {
        let opening = frame_width(usize::from(n));
        let lines_past = |limit: usize| -> Vec<(usize, &str)> {
            frame.lines().map(|l| (visible_width(l), l)).filter(|(w, _)| *w > limit).collect()
        };

        let over_floor = lines_past(FLOOR_W);
        if !over_floor.is_empty() {
            report.push(format!(
                "the {n}P '{label}' frame has {} line(s) past the documented floor of \
                 {FLOOR_W} columns: {over_floor:#?}",
                over_floor.len(),
            ));
        }

        let over_count = lines_past(opening + FRAME_MARGIN);
        if !over_count.is_empty() {
            report.push(format!(
                "the {n}P '{label}' frame has {} line(s) past this count's opening width \
                 {opening} + FRAME_MARGIN {FRAME_MARGIN} = {}: {over_count:#?}",
                over_count.len(),
                opening + FRAME_MARGIN,
            ));
        }

        frame.lines().map(visible_width).max().unwrap_or(0)
    }

    /// One line per count naming what it actually reached, so a failure (and a
    /// `--nocapture` run) hands over the numbers [`FRAME_MARGIN`]'s table is
    /// set from instead of only the verdict.
    fn widest_report(widest: &[(u8, usize, String)]) -> String {
        widest
            .iter()
            .map(|(n, w, label)| {
                let opening = frame_width(usize::from(*n));
                format!(
                    "  {n}P: widest {w} against an opening {opening} \
                     (overshoot {}), at '{label}'",
                    w.saturating_sub(opening),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// I6 (third review): the frame had a `<= 50` HEIGHT pin and nothing at
    /// all on WIDTH, so two runtime lines had quietly grown past the width
    /// guide §2 promises clears every layout -- the Movement prompt hint at
    /// 142, and the pending-choice status line at 150, which split the goods
    /// block mid-token (`i0` rendering as `i` with a stray `0 | hand 1` below
    /// it, reading as corrupted state). Measured on the same composed frame
    /// the height pin measures, over the widest positions the driver can
    /// actually show.
    ///
    /// **TWO bounds since the T2 review.** The absolute one is [`FLOOR_W`],
    /// which replaced a flat 140 at T2 -- not a relaxation to accommodate a
    /// wider board, but the redesign's own raised floor (spec §0.1 designs the
    /// frame at 200x50), since the 4P board at 180 columns leaves 140
    /// unreachable at any tile size the mocks approved. What that move cost was
    /// the pin's GRIP: at 3 players it left 37 columns of slack where the
    /// pre-T2 bound left 8, which is room enough for exactly the class of line
    /// this test was written to catch. So each count now also holds
    /// `frame_width(n) + FRAME_MARGIN` -- the width the layout says that count
    /// composes at, plus a measured allowance for the panel growing in play.
    ///
    /// **The new bound binds where the floor does not, and that was checked
    /// rather than argued.** Padding the discoverability strip to 190 columns
    /// -- a line 26 columns past the widest frame the driver composes, and
    /// still 10 clear of [`FLOOR_W`] -- fails the per-count bound at all three
    /// counts and the floor at none of them. Reverted by inverse edit.
    #[test]
    fn frame_fits_the_raised_floor_at_every_player_count() {
        let mut report = Vec::new();
        let mut widest = Vec::new();
        for n in [2u8, 3, 4] {
            let mut worst = (0usize, String::new());
            for (label, frame) in worst_case_frames(n) {
                let w = measure_frame(n, &label, &frame, &mut report);
                if w > worst.0 {
                    worst = (w, label);
                }
            }
            widest.push((n, worst.0, worst.1));
        }
        assert!(
            report.is_empty(),
            "{}\nwidest line per count:\n{}",
            report.join("\n"),
            widest_report(&widest),
        );
    }

    /// Organic companion to the constructed worst cases above: a constructed
    /// state is only ever as wide as its author imagined, so walk a real game
    /// and hold the same two bounds on every frame it draws. This is what
    /// covers the OPTION lines real play produces -- movement routes naming
    /// three fee recipients, build options carrying a site's printed reward,
    /// the longest area labels -- none of which the fixtures force.
    ///
    /// Walked at EVERY count since the T2 review, where it was 4P only. The
    /// per-count bound is a different number at each count, so a 4P-only walk
    /// would leave the other two bounds resting entirely on constructed
    /// fixtures -- and it is this walk, not the fixtures, that reaches the 3P
    /// mid-game panel [`FRAME_MARGIN`] is sized off.
    #[test]
    fn every_frame_of_a_real_game_walk_fits_the_raised_floor() {
        let mut report = Vec::new();
        let mut widest = Vec::new();
        for n in [2u8, 3, 4] {
            let mut r = repl_np(n);
            force(true);
            let mut steps = 0usize;
            let mut worst = (0usize, String::new());
            while !r.session.is_over() && steps < 250 {
                let frame = wrap(&r, &r.prompt());
                let label = format!("move {steps}");
                let w = measure_frame(n, &label, &frame, &mut report);
                if w > worst.0 {
                    worst = (w, label);
                }
                let legal = r.session.legal();
                if legal.is_empty() {
                    break;
                }
                // Rotate through the list rather than always taking [0], so the
                // walk reaches positions with buildings, techs and orders in
                // them instead of replaying one degenerate line.
                let mv = legal[steps % legal.len()].clone();
                if r.session.apply(mv, Vec::new()).is_err() {
                    break;
                }
                steps += 1;
            }
            force(false);
            assert!(
                steps > 100,
                "the {n}P walk must actually reach mid-game positions, only got {steps} moves",
            );
            widest.push((n, worst.0, worst.1));
        }
        assert!(
            report.is_empty(),
            "{}\nwidest line per count:\n{}",
            report.join("\n"),
            widest_report(&widest),
        );
    }

    /// `panel` hides/shows the players panel inside the frame; `screen`
    /// turns the whole composed mode off (body passes through untouched).
    #[test]
    fn panel_and_screen_commands_toggle_the_frame() {
        let mut r = repl_np(2);
        force(true);
        let out = r.step("panel");
        assert!(out.text.contains("panel"), "{}", out.text);
        let f = wrap(&r, "");
        assert!(!f.contains("Players"), "panel must be hidden after toggle:\n{f}");
        r.step("panel");
        assert!(wrap(&r, "").contains("Players"), "panel must return on second toggle");

        let out = r.step("screen");
        assert!(out.text.contains("screen"), "{}", out.text);
        assert_eq!(wrap(&r, "BODY"), "BODY", "composed off must pass the body through");
        r.step("screen");
        assert!(wrap(&r, "BODY").contains("\u{1b}[2J"), "composed back on");
        force(false);
    }

    /// Painted frame stays aligned: stripping the ANSI back out reproduces
    /// the plain frame exactly (the crate-wide alignment invariant).
    #[test]
    fn painted_frame_strips_back_to_plain() {
        let r = repl_np(2);
        force(true);
        crate::style::force(true);
        let colored = wrap(&r, "");
        crate::style::force(false);
        let plain = wrap(&r, "");
        force(false);
        assert_ne!(colored, plain, "forced color must actually paint");
        assert_eq!(strip_ansi(&colored), strip_ansi(&plain), "paint must never move a visible char");
    }

    #[test]
    fn banner_is_multiline_ascii() {
        let b = banner();
        assert!(b.lines().count() >= 4, "{b}");
        assert!(b.is_ascii(), "banner must be plain ASCII: {b}");
    }

    /// The guard's happy path: a block whose every line already measures the
    /// declared width passes silently.
    #[test]
    fn assert_exact_width_accepts_a_uniform_block() {
        test_support::assert_exact_width(&["┌──┐", "│ab│", "└──┘"], 4);
        test_support::assert_exact_width::<&str>(&[], 30);
    }

    /// One column over is a failure, and the message names WHICH line, what
    /// was expected, what was measured, and the offending text -- the four
    /// facts that turn a width failure into a one-look fix.
    #[test]
    #[should_panic(expected = "line 1 is 5 columns wide, expected 4")]
    fn assert_exact_width_rejects_a_line_that_is_one_column_over() {
        test_support::assert_exact_width(&["│ab│", "│abc│"], 4);
    }

    /// ...and one column short fails just as loudly: a short line shears a
    /// side-by-side join exactly as an over-long one does.
    #[test]
    #[should_panic(expected = "line 0 is 3 columns wide, expected 4")]
    fn assert_exact_width_rejects_a_line_that_is_one_column_short() {
        test_support::assert_exact_width(&["│a│"], 4);
    }

    /// Escapes carry zero display width, so a painted line of the same
    /// visible text passes the same bound its plain twin does -- the crate's
    /// standing alignment rule, made checkable.
    #[test]
    fn assert_exact_width_ignores_ansi_escapes() {
        crate::style::force(true);
        let painted = crate::style::paint(crate::style::Tone::Vp, "ab");
        crate::style::force(false);
        assert_ne!(painted, "ab", "forced color must actually paint");
        test_support::assert_exact_width(&[painted], 2);
    }
}
