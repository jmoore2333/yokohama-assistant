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
//! with the compact players panel (side-by-side while the board is <= 100
//! columns wide, stacked below it on the 108-column 4P board -- std exposes
//! no terminal size, so the board's own width is the only honest signal),
//! the board legend, a rule, then the body (the step's delta/error lines
//! plus the normal prompt block, unchanged from piped mode -- nothing
//! exists only in the frame).

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
    let state = repl.session.state();
    let board = render::board_grid(state);
    let board_width = board.lines().next().map(visible_width).unwrap_or(0);

    let seats = repl.seat_display();
    let mut stage = if repl.show_panel {
        if board_width <= 100 {
            join_side_by_side(&board, &render::compact_panel(state, repl.session.rules(), &seats), 2)
        } else {
            // Stacked below the board, VERTICAL space is the scarce
            // resource, not horizontal: the frame is already >= 108 columns
            // here, so the panel folds to one line per player (F3, Codex
            // review of PR #4 -- the 5-line block cost 16 rows and pushed
            // the 4P opening frame off a 140x50 terminal). The side-by-side
            // branch above is untouched, so the 2P/3P screens are
            // byte-identical to before.
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
    stage.push_str(&render::board_legend(state));
    stage.push('\n');
    // Discoverability strip (owner scope item 10): every frame names the
    // game-info surfaces and the command that opens them, so a player who
    // never read the guide finds them on screen.
    stage.push_str(
        "info: type 'view' — board · players · ports · labs · tracks · sites · goals · hand · score · log\n",
    );

    let width = stage.lines().map(visible_width).max().unwrap_or(0).max(60);
    let title = " Y O K O H A M A ";
    let pad = width.saturating_sub(title.chars().count());
    let strip = format!("{}{}{}", "═".repeat(pad / 2), title, "═".repeat(pad - pad / 2));

    format!("\u{1b}[2J\u{1b}[H{strip}\n{stage}{}\n{body}", "─".repeat(width))
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
        assert!(f.contains("5-Power tokens (face-up)"), "legend expected: {f}");
        assert!(
            f.contains("info: type 'view'"),
            "the discoverability strip must be in every frame (owner scope item 10): {f}"
        );
        assert!(f.ends_with("BODY-MARKER\n> hint\n"), "body must close the frame: {f}");
    }

    /// 2P (72-col board): the panel joins SIDE BY SIDE -- some line carries
    /// both a board border and panel text. 4P (108-col board): the panel
    /// stacks BELOW -- no line carries both.
    #[test]
    fn panel_joins_beside_narrow_boards_and_below_wide_ones() {
        force(true);
        // "President:" is panel-only text ("yen" also occurs INSIDE the 4P
        // board's canal tile, which would fool this detector).
        let r2 = repl_np(2);
        let f2 = wrap(&r2, "");
        let side = f2.lines().any(|l| l.contains('│') && l.contains("President:"));
        assert!(side, "2P panel must sit beside the board:\n{f2}");

        let r4 = repl_np(4);
        let f4 = wrap(&r4, "");
        let side4 = f4.lines().any(|l| l.contains('│') && l.contains("President:"));
        assert!(!side4, "4P panel must stack below the 108-col board:\n{f4}");
        assert!(f4.contains("Players"), "4P panel still present, just below:\n{f4}");
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

    /// I6 (third review): the frame had a `<= 50` HEIGHT pin and nothing at
    /// all on WIDTH, so two runtime lines had quietly grown past the 140
    /// columns guide §2 promises clears every layout -- the Movement prompt
    /// hint at 142, and the pending-choice status line at 150, which split
    /// the goods block mid-token (`i0` rendering as `i` with a stray
    /// `0 | hand 1` below it, reading as corrupted state). Measured on the
    /// same composed frame the height pin measures, over the widest
    /// positions the driver can actually show.
    #[test]
    fn frame_fits_one_hundred_forty_columns_at_every_player_count() {
        for n in [2u8, 3, 4] {
            for (label, frame) in worst_case_frames(n) {
                let over: Vec<(usize, &str)> = frame
                    .lines()
                    .map(|l| (visible_width(l), l))
                    .filter(|(w, _)| *w > 140)
                    .collect();
                assert!(
                    over.is_empty(),
                    "the {n}P '{label}' frame has {} line(s) past the documented 140 columns: {over:#?}",
                    over.len()
                );
            }
        }
    }

    /// Organic companion to the constructed worst cases above: a constructed
    /// state is only ever as wide as its author imagined, so walk a real
    /// 4-player game and hold the same bound on every frame it draws. This is
    /// what covers the OPTION lines real play produces -- movement routes
    /// naming three fee recipients, build options carrying a site's printed
    /// reward, the longest area labels -- none of which the fixtures force.
    #[test]
    fn every_frame_of_a_real_game_walk_fits_one_hundred_forty_columns() {
        let mut r = repl_np(4);
        force(true);
        let mut steps = 0usize;
        while !r.session.is_over() && steps < 250 {
            let frame = wrap(&r, &r.prompt());
            let over: Vec<(usize, &str)> =
                frame.lines().map(|l| (visible_width(l), l)).filter(|(w, _)| *w > 140).collect();
            assert!(over.is_empty(), "at move {steps} the frame ran past 140 columns: {over:#?}");
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
        assert!(steps > 100, "the walk must actually reach mid-game positions, only got {steps} moves");
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
}
