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
//! with the dashboard (BESIDE it while the board and the designed dashboard
//! both fit `FLOOR_W`, which is 2 players on the V3 board; BELOW it at 3 and 4
//! -- std exposes no terminal size, so what the frame itself measures is the
//! only honest signal), the board legend, a rule, then the body (the step's
//! delta/error lines plus the normal prompt block, unchanged from piped mode --
//! nothing exists only in the frame).
//!
//! **Every region of the frame is now the redesigned one** (T4 finished the
//! last of them): the V3 board, the dashboard beside or below it, and the
//! decision zone in the body. What remains transitional is the 3P/4P
//! ARRANGEMENT, which T10 re-lays — see [`FRAME_HEIGHTS`].

use std::sync::atomic::{AtomicBool, Ordering};

use crate::panel_render;
use crate::render;
use crate::repl::Repl;
use crate::screen_model::{TitleMode, TitleModel};
use crate::session::TrackingMode;
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

    /// A test's answer to "how wide is the terminal", or `None` for "this
    /// thread has not said" — see `terminal_width`. The doubled `Option` is
    /// deliberate: the outer one is whether a test has spoken, the inner one is
    /// whether the answer is a width or "there is no terminal", and collapsing
    /// them would make "no terminal" unstatable.
    static FORCED_WIDTH: std::cell::Cell<Option<Option<usize>>> =
        const { std::cell::Cell::new(None) };
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

/// The real terminal's column count, or `None` when there is no terminal to
/// ask — measured ONCE per process and remembered.
///
/// **`std` still exposes no terminal size**, which is why every width in this
/// module is a designed constant and why `Repl::board_cmd`'s note says so. The
/// card index is the one surface that cannot live with that: it lays cards out
/// in a GRID sized from [`FLOOR_W`], so below 200 columns every row it drew
/// wrapped mid-card, at the exact moment a player was trying to read cards
/// (T7 review SHOULD-6, found on a real terminal).
///
/// So it asks the terminal, through the one channel available without adding a
/// dependency or unsafe code to a crate that has neither: `stty size`, reading
/// `/dev/tty` rather than stdin so a redirected stdin cannot change the answer.
///
/// **Gated on stdout being a TTY, which is what keeps the piped protocol
/// byte-identical** (spec §0.12). `/dev/tty` answers even when stdout is a pipe
/// — verified — so without this gate a piped run inside a terminal would render
/// differently from the same run in CI, and `piped_dogfood` would be measuring
/// the developer's window. Under a pipe this is `None` and every caller falls
/// back to the designed floor, exactly as before.
///
/// Everything here fails to `None`: no `/dev/tty`, no `stty`, a non-zero exit,
/// unparseable output, or a nonsense width. A screen is no place to take the
/// driver down over a rounding question.
///
/// **It reads the `ENABLED` atomic directly rather than calling [`enabled`],
/// and that is the hermeticity fence.** `force` is a test hook for the LAYOUT
/// question; letting it also decide whether this process shells out would make
/// every test that forces frames on measure the DEVELOPER'S WINDOW, so the same
/// suite would pass at 200 columns and fail at 120. Only `init_from_tty` sets
/// the atomic, and only `main` calls that — so a test can never reach the
/// subprocess, and [`force_width`] is how a test says what the terminal is.
pub(crate) fn terminal_width() -> Option<usize> {
    static MEASURED: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    if let Some(forced) = FORCED_WIDTH.with(|f| f.get()) {
        return forced;
    }
    if !may_measure() {
        return None;
    }
    *MEASURED.get_or_init(measure_terminal_width)
}

/// Whether this process may ask a terminal how wide it is.
///
/// Its own function so the fence above can be ASSERTED rather than inferred: a
/// test observing only `terminal_width() == None` would also pass with the
/// fence removed on any machine without a `/dev/tty`, which is to say it would
/// pass in CI and stop catching the regression on a developer's laptop —
/// precisely the class of environment-dependence the fence exists to prevent.
fn may_measure() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Test hook: report `cols` (or `None`) as the terminal width without asking a
/// terminal. Per-thread, for the reason [`force`] is; production never calls it.
#[cfg(test)]
pub(crate) fn force_width(cols: Option<usize>) {
    FORCED_WIDTH.with(|f| f.set(Some(cols)));
}

/// Test hook: forget a [`force_width`], so this thread falls back to the
/// production path (which, in a test, measures nothing — see [`terminal_width`]).
#[cfg(test)]
pub(crate) fn clear_forced_width() {
    FORCED_WIDTH.with(|f| f.set(None));
}

/// One `stty size` call. Separate from the memo so a test can exercise the
/// parse without burning the process-wide `OnceLock`.
fn measure_terminal_width() -> Option<usize> {
    let tty = std::fs::File::open("/dev/tty").ok()?;
    let out = std::process::Command::new("stty")
        .arg("size")
        .stdin(std::process::Stdio::from(tty))
        .output()
        .ok()?;
    out.status.success().then_some(())?;
    parse_stty_size(&String::from_utf8_lossy(&out.stdout))
}

/// `"40 120\n"` — rows then columns, which is the order that has bitten
/// everyone who assumed otherwise, so it is parsed by POSITION and named here.
///
/// **BOTH fields must be numbers**, even though only the second is used: the
/// shape being recognized is "two integers", and a parser that skipped the first
/// field unread would happily take the `4` out of `rows 4 columns 100`. A width
/// under [`MIN_SANE_WIDTH`] is treated as no answer at all.
fn parse_stty_size(out: &str) -> Option<usize> {
    let mut fields = out.split_whitespace();
    let _rows: usize = fields.next()?.parse().ok()?;
    let cols: usize = fields.next()?.parse().ok()?;
    (cols >= MIN_SANE_WIDTH).then_some(cols)
}

/// Below this, a reported width is not believed. Terminals do report 0 while
/// being resized, and a detached PTY can report nonsense; the designed floor is
/// a better answer than a one-column grid.
const MIN_SANE_WIDTH: usize = 20;

/// The columns a full-screen card index may use: the real terminal where that
/// is knowable, the designed floor otherwise.
///
/// The floor is the FALLBACK rather than the target — see [`terminal_width`].
/// Above 200 the index simply gets more cards to a row, which is the same
/// "adaptive beyond the floor" §0.1 already asks the composed frame for.
pub(crate) fn index_width() -> usize {
    terminal_width().unwrap_or(FLOOR_W)
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

/// The DESIGNED dashboard width — the approved mocks' `DASH_W`, now the width
/// the panels actually render at ([`panel_render::DASH_W`], where the number
/// lives beside the panels that fill it).
///
/// The arrangement is decided against the DESIGN rather than against whatever
/// the panel happens to measure today, and T3 is why that mattered: a dashboard
/// is ragged-right, so a turn-1 frame whose longest panel line is 68 columns
/// would arrange one way and the same game five turns later, at 74, another.
/// Sizing on the design gives spec §0.1's ruling at every position in the game
/// (2P beside, 3P and 4P below) instead of only at the ones measured.
const DASH_W: usize = panel_render::DASH_W;

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
/// **These are OPENING widths, and since T3 they are also CEILINGS.** The
/// board, legend and info strip are fixed per count; the dashboard is the one
/// part play changes. It cannot widen a frame past these numbers, because every
/// panel line is clipped at [`DASH_W`] ([`panel_render`]'s own rule) and the 2P
/// entry already budgets the full [`DASH_W`] beside the board — which is the
/// property T3 bought and the 40-column panel did not have. What play can do is
/// leave a frame NARROWER: a dashboard is ragged-right, so a turn-1 position
/// whose longest panel line falls short of 74 composes a slightly narrower
/// frame. That direction is safe for a body sized against these numbers, and
/// repl.rs's `the_suggest_screen_fits_the_composed_frame_at_every_player_count`
/// checks a real mid-game frame against the bound at each count rather than
/// leaving the claim to argument.
///
/// **T2 moved all three and T3 moved the first**, and a DIFFERENT element is
/// the widest one at each count, which is why the three numbers do not track
/// the board: at 4 players it is the 180-column board itself; at 3 it is the
/// site-slot LEGEND, at 163 columns the one row that keys the V3 tiles'
/// compact tokens and the same length at every count; at 2 it is the board with
/// the dashboard joined beside it, at 120 + [`PANEL_GAP`] + [`DASH_W`] = 198.
///
/// The history in one line, because two tasks moved these in three weeks: the
/// pre-redesign numbers were 114/132/108; T2's V3 board took them to
/// 163/163/180 and then 164/163/180 when [`PANEL_GAP`] widened to the mocks'
/// four-column gutter; T3 replaced the 40-column compact panel with the
/// designed 74-column dashboard, which put the 2P join at 198.
///
/// **198 is 2 columns under [`FLOOR_W`], and that is the design working, not a
/// near miss.** Spec §0.1 designs the 2P frame at 200 and the mocks are drawn
/// at exactly that; 120 + 4 + 74 is what "the board and the dashboard side by
/// side inside 200" costs.
const FRAME_WIDTHS: [usize; 3] = [198, 163, 180];

/// The composed frame's HEIGHT at each player count, indexed `players - 2` --
/// the three numbers guide §2 states, measured the same way [`FRAME_WIDTHS`] is
/// and pinned by `the_opening_frame_is_the_recorded_size_at_every_count`.
///
/// **Two of these three are past the design floor's 50 rows, and it is
/// deliberate, transitional and scheduled.** Spec §0.1 designs the frame at
/// 200x50. At 2 players the dashboard joins BESIDE the board and the frame
/// opens at 48 rows, inside it. At 3 and 4 the board is too wide to leave room
/// beside it, so the same panel block stacks BELOW, and a block that costs 8
/// rows per seat plus its three other panels runs the frame to 78 and 87 rows:
/// on a 50-row terminal the title strip and the top board rows have scrolled
/// off before the player can read them, which is F3 exactly, returning by a
/// different door.
///
/// **T4 added two rows to all three**, and they are the decision zone's own
/// shape rather than growth: the redesigned zone leads with a phase chain and a
/// blank line above its "YOU ARE DECIDING" sentence (spec §0.8, the approved
/// mocks' own spacing) where the pre-redesign block opened straight onto a
/// one-line state header. The opening frame shows ONE option either way, so the
/// two rows are the whole difference at turn 1; what T4 bought back is at the
/// other end of the range, in [`WALK_HEIGHTS`].
///
/// **T5 added one row at 3 players ONLY (78 → 79), and where it lands says
/// what it is.** The standing hint gained the approved mock's `'?' the card
/// index` clause — true from T5, when `?` stopped aliasing the view menu — and
/// the hint WRAPS to the frame's own width. At 198 and 180 columns the longer
/// clause list still fits one line; at 163 it does not, so the 3-player frame
/// pays a second hint row and the other two pay nothing. That is the wrap
/// working as `decision_render::wrap_hint` designed it, and it is also a
/// reminder that these three numbers do not move together.
///
/// **T10 is the fix and it is a layout change, not a trim.** The approved 4P
/// mock (`frame-4p.ans`) draws the players panel in FOUR COLUMNS across the
/// board's own 180-column footprint and folds tracks and achievements onto one
/// line each — 43 rows all in. This task was scoped to cut the panels over at
/// every count and leave the higher-count ARRANGEMENT to T10, so what ships
/// here is the honest intermediate: the right content, in the wrong shape, with
/// the number written down instead of a bound quietly relaxed to cover it.
/// `panel` is the control that buys the rows back in the meantime.
///
/// `cfg(test)`, unlike [`FRAME_WIDTHS`]: a width is a BUDGET the running driver
/// spends (`suggest` sizes its body against `frame_width`), while a height is
/// only ever a measurement and a pin. Nothing at runtime asks how tall its own
/// frame is, so a production constant here would be dead code claiming to be
/// used.
#[cfg(test)]
const FRAME_HEIGHTS: [usize; 3] = [48, 79, 87];

/// The design floor's height (spec §0.1's 200x50), which [`FRAME_HEIGHTS`]
/// meets at 2 players and not at 3 or 4. `cfg(test)` for the same reason.
#[cfg(test)]
const FLOOR_H: usize = 50;

/// The tallest frame a real game walk DRAWS at each player count, indexed
/// `players - 2` — the IN-PLAY height bound, and the height twin of what
/// [`FRAME_MARGIN`] is for width.
///
/// **Added at the T3 review, which found the asymmetry that made it necessary.**
/// Width had two bounds over a whole battery of positions — every constructed
/// worst case plus a 250-move walk at each count — while HEIGHT had exactly one
/// measurement, of the OPENING frame, and nothing at all past turn 1. So a frame
/// that opens at 46 rows and reaches 66 in play broke no bound and printed no
/// number: the 2P in-play maximum was 20 rows past its opening for the whole of
/// T3 and nobody could have known, because nothing measured it.
///
/// **MEASURED, then set** — the same discipline as [`FRAME_MARGIN`], run over
/// the same 250-move walk, and **RE-MEASURED at T4**, which is the condition
/// the previous revision of this doc attached to the table:
///
/// | count | opening | tallest in play | overshoot | (T3's reading) |
/// |---|---|---|---|---|
/// | 2 | 48 | 60 | +12 | 66, +20 |
/// | 3 | 78 | 90 | +12 | 96, +20 |
/// | 4 | 87 | 99 | +12 | 105, +20 |
///
/// **The peak is the same frame it always was and it now costs eight rows
/// less.** All three maxima are the walk's move 7, an AreaAction on the Church
/// with 27 payment combinations — the position that set the old +20. The
/// pre-redesign zone drew twenty of those on twenty lines; the T4 zone budgets
/// ROWS instead of options (`decision_render::OPTION_ROWS`, twelve) and packs
/// short options into columns, so the same decision fits twelve rows at every
/// count. The overshoot is +12 at all three for the same reason it was +20 at
/// all three: one option list under three different boards.
///
/// **What is left is the zone's own budget, not a defect with a fix pending.**
/// A decision with more options than one screen holds has to either page or
/// grow, and this driver pages; twelve rows is the deliberate size of that
/// page. The 2P frame therefore reaches 60 rows against a 48-row opening, which
/// is 10 past the design floor at the tallest position a real game walk finds.
///
/// A number that is recorded is not a number that is licensed: growth PAST these
/// fails, and the fix is to re-measure the table deliberately, not to raise one
/// entry until a run goes quiet.
#[cfg(test)]
const WALK_HEIGHTS: [usize; 3] = [60, 90, 99];

/// Allowance over [`WALK_HEIGHTS`], and one row is all it is.
///
/// The walk is deterministic — fixed seed, and a move rotation keyed on the step
/// index — so there is no run-to-run noise for a margin to absorb: the three
/// figures above reproduced exactly across three consecutive runs. One row is
/// therefore "just above the observed variation" rather than an allowance for
/// anything in particular, and it keeps this a bound a growing frame can
/// actually break: at 1 the first added panel row still passes and the second
/// fails, which is the same grip [`FRAME_MARGIN`] has on width.
#[cfg(test)]
const WALK_HEIGHT_MARGIN: usize = 1;

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
    // A FULL-SCREEN surface replaces the frame rather than sitting under it
    // (spec §0.5's view pattern; T5's card index is the first of them). The
    // approved `order-index` mock draws no board, no dashboard and no title
    // strip -- 45 rows of cards on an otherwise empty screen -- and it has to:
    // the index alone is 45 rows against the 2-player frame's 48, so composing
    // the two would scroll both off a 50-row terminal. The clear-and-home
    // prefix still leads, so the surface arrives on a clean screen exactly as
    // a framed body does.
    if repl.full_screen_surface() {
        return format!("\u{1b}[2J\u{1b}[H{body}");
    }
    let stage = stage(repl);
    let width = stage_width(&stage);
    let strip = panel_render::title_strip(&title_model(repl), width);

    // The blank line under the strip is the approved mocks' own spacing: it is
    // what stops the title reading as a caption on the top row of tiles.
    format!(
        "\u{1b}[2J\u{1b}[H{strip}\n\n{stage}{}\n{body}",
        String::from(crate::style::glyph::RULE).repeat(width),
    )
}

/// The session facts the title strip states, mapped out of the `Session` HERE
/// rather than inside the model.
///
/// [`TitleMode`] is a local mirror of [`TrackingMode`] precisely so
/// `screen_model`'s dependency list stays `yokohama_core` alone (T0-review SF2),
/// which makes this three-line map the CALLER's job by design — this is that
/// caller.
fn title_model(repl: &Repl) -> TitleModel {
    TitleModel {
        mode: match repl.session.mode {
            TrackingMode::Simulation => TitleMode::Sim,
            TrackingMode::Live => TitleMode::Live,
        },
        seed: repl.session.state().config.setup_seed,
        autosaved: repl.session.save_path.is_some(),
    }
}

/// Everything [`wrap`] composes ABOVE the body: the board (with the dashboard
/// beside or below it), the board legend, the discoverability strip.
///
/// Split out of `wrap` so [`stage_width`] can be measured on its own -- the
/// [`FRAME_WIDTHS`] pin needs the width `wrap` sizes itself to, not a width
/// re-derived by parsing a finished frame back apart.
///
/// **The stacked arrangement is TRANSITIONAL and it costs rows.** At 2 players
/// the dashboard joins beside the board and the frame opens inside the design
/// floor. At 3 and 4 the same panel block stacks BELOW a board too wide to sit
/// beside, and the frame runs well past 50 rows — see [`FRAME_HEIGHTS`], which
/// records what each count actually opens at. T10 is the task that fixes it,
/// by re-laying the players panel into the four columns the 4P mock draws;
/// until then `panel` is the control that buys those rows back.
fn stage(repl: &Repl) -> String {
    let state = repl.session.state();
    let rules = repl.session.rules();
    // The legal list is what tints the `◈` destination marks (spec §0.7), so
    // the board the frame draws is asked for with it. Fetched here rather than
    // threaded down from `prompt()`: `wrap` composes a frame around a body that
    // was built earlier and may not be a prompt at all (a view, a delta, an
    // error), and a frame that showed marks only when the body happened to be a
    // prompt would flicker them on and off as the player looked around.
    let legal = repl.session.legal();
    let board = render::board_grid(state, rules, &legal);
    let board_width = board.lines().next().map(visible_width).unwrap_or(0);

    let seats = repl.seat_display();
    let mut stage = if repl.show_panel {
        let dashboard = render::dashboard(state, rules, &seats, state.current_player);
        if board_width + PANEL_GAP + DASH_W <= FLOOR_W {
            join_side_by_side(&board, &dashboard, PANEL_GAP)
        } else {
            let mut s = board.clone();
            if !s.is_empty() && !s.ends_with('\n') {
                s.push('\n');
            }
            s.push_str(&dashboard);
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
        "info: type 'view' — board · players · ports · labs · tracks · sites · goals · hand · score · log   ·   '?' — the card index\n",
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
        assert!(f.contains("PLAYERS"), "players panel expected in frame: {f}");
        assert!(f.contains("TRACKS"), "tracks panel expected in frame: {f}");
        assert!(f.contains("ACHIEVEMENTS"), "achievements panel expected in frame: {f}");
        assert!(f.contains("YOUR HAND"), "hand panel expected in frame: {f}");
        assert!(f.contains("site slots"), "legend expected: {f}");
        assert!(
            f.contains("info: type 'view'"),
            "the discoverability strip must be in every frame (owner scope item 10): {f}"
        );
        assert!(f.ends_with("BODY-MARKER\n> hint\n"), "body must close the frame: {f}");
    }

    /// Spec §0.1's arrangement ruling, reproduced from the geometry rather
    /// than hard-wired: at 2 players the 120-column V3 board plus the designed
    /// dashboard fits [`FLOOR_W`], so the dashboard joins SIDE BY SIDE -- some
    /// line carries both a board border and panel text. At 3 and 4 players the
    /// board alone is 150 and 180 columns, so it stacks BELOW and no line
    /// carries both.
    ///
    /// The join is detected on `Mercenaries `, which the T3 dashboard emits
    /// once per seat and no tile ever does. It used to key on `President:` --
    /// a string the redesigned panel no longer writes (it says `President ` and
    /// then either `in hand` or an area name) and one that would have gone on
    /// matching the panel it was written for right up until T2 removed the
    /// tile line that shared it.
    #[test]
    fn the_dashboard_sits_beside_the_board_only_while_both_fit_the_floor() {
        force(true);
        // A line carrying BOTH a tile edge and panel-only text is the join.
        // Any of the four box characters counts, not `│` alone: V3 tiles are
        // five lines, so a panel row lands on a tile's top or bottom BORDER as
        // readily as on its interior.
        let joined = |frame: &str| {
            frame.lines().any(|l| {
                l.contains("Mercenaries ")
                    && ['┌', '┐', '└', '┘', '│'].iter().any(|c| l.contains(*c))
            })
        };

        let r2 = repl_np(2);
        let f2 = wrap(&r2, "");
        assert!(joined(&f2), "2P dashboard must sit beside the board:\n{f2}");

        for n in [3u8, 4] {
            let r = repl_np(n);
            let f = wrap(&r, "");
            assert!(
                !joined(&f),
                "the {n}P dashboard must stack below a board wider than the floor allows \
                 beside it:\n{f}"
            );
            assert!(f.contains("PLAYERS"), "{n}P dashboard still present, just below:\n{f}");
        }

        // Whichever way it is arranged, the dashboard carries every seat: the
        // stacked branch used to fold to one line per player and drop fields to
        // do it, and T3's cutover renders the SAME block at every count.
        let r4 = repl_np(4);
        let f4 = wrap(&r4, "");
        assert_eq!(
            f4.lines().filter(|l| l.contains("Mercenaries ")).count(),
            4,
            "the stacked dashboard must carry all four seats:\n{f4}"
        );
        for name in ["RED", "BLUE", "YELLOW", "GREEN"] {
            assert!(f4.contains(name), "the stacked dashboard must name {name}:\n{f4}");
        }
        force(false);
    }

    /// The title strip states the session facts [`TitleModel`] carries, and it
    /// carries no rule character at all -- the approved mocks' own shape, and
    /// the discharge of style.rs's DEFERRED CONVERSIONS row for `═`.
    #[test]
    fn the_title_strip_states_the_session_facts_and_draws_no_double_rule() {
        let r = repl_np(2);
        force(true);
        let f = wrap(&r, "");
        force(false);
        let strip = f
            .lines()
            .next()
            .expect("a first line")
            .trim_start_matches(['\u{1b}', '[', '2', 'J', 'H']);
        assert!(strip.contains("Y O K O H A M A"), "{strip}");
        assert!(strip.contains("simulation"), "a sim session says so: {strip}");
        assert!(strip.contains("seed 7"), "the game's identity seed: {strip}");
        // Scoped to what this test EMITTED, since the T3 review: `f` is one
        // composed frame, so a pass says the frame in front of us carries no
        // `═` -- not that the crate spells U+2550 nowhere, which is a claim
        // about source text that no rendering assert can make. style.rs's
        // DEFERRED CONVERSIONS row states the source-level half separately.
        assert!(
            !f.contains('\u{2550}'),
            "the redesigned strip draws no ═ anywhere in this composed frame:\n{f}",
        );
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

    /// F3 (Codex adversarial review of PR #4): the composed screen has to FIT
    /// the terminal the guide tells a tester to open. The opening frame is
    /// exactly `wrap(repl, repl.prompt())` (`main.rs`), so measure that.
    ///
    /// **This was a flat `rows <= 50` until T3, and it is now a RECORDED table
    /// plus that bound where the bound still holds.** Weakening an F3 pin needs
    /// a reason on the record rather than a bigger constant, so here it is: T3
    /// replaced the stacked one-line-per-player fold -- which existed precisely
    /// to hold this bound -- with the full designed dashboard at every count
    /// (the task's own scope), and at 3 and 4 players that block stacks below
    /// the board and runs the frame to 76 and 85 rows. T10 re-lays it into the
    /// 4P mock's four columns; see [`FRAME_HEIGHTS`] for the whole disposition.
    ///
    /// An EQUALITY table is a stronger pin than the inequality it replaces, not
    /// a weaker one: it fails on a frame that grows by one row and on a frame
    /// that shrinks by one, so the transitional numbers cannot drift while
    /// nobody is looking, and `guide_sync` independently requires guide §2 to
    /// state these same three. What is genuinely lost is the FLOOR at 3 and 4,
    /// which is why it is still asserted at 2 rather than dropped for all three.
    #[test]
    fn the_opening_frame_is_the_recorded_size_at_every_count() {
        for n in [2u8, 3, 4] {
            let r = repl_np(n);
            force(true);
            let frame = wrap(&r, &r.prompt());
            force(false);
            let rows = frame.lines().count();
            assert_eq!(
                FRAME_HEIGHTS[usize::from(n) - 2],
                rows,
                "the {n}P frame opens at {rows} rows, so FRAME_HEIGHTS must say so -- \
                 and guide §2 (checked independently by guide_sync) must state the same number"
            );
        }
        assert!(
            FRAME_HEIGHTS[0] <= FLOOR_H,
            "the 2P frame joins the dashboard beside the board and must stay inside the \
             {FLOOR_H}-row design floor; it is {} rows",
            FRAME_HEIGHTS[0],
        );
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
    /// **MEASURED, then set — and RE-MEASURED at T3, which is the condition the
    /// previous revision of this doc attached to the constant.** That revision
    /// said: "if T3's dashboard makes the panel the widest element at a count
    /// play can then widen, this constant gets RE-MEASURED — not raised to
    /// whatever silences it." T3 did make the dashboard the widest element at 2
    /// players. The instrumentation was re-run over both pins below — every
    /// constructed worst case (turn 99, 199 money and VP, 15 of every good on
    /// every seat, all seven pending-choice banners, a lockdown) and a real
    /// 250-move game walk, at each count — and the overshoot came back at zero
    /// again, so the constant stays at one and the table states the new
    /// openings:
    ///
    /// | count | opening | widest measured | overshoot |
    /// |---|---|---|---|
    /// | 2 | 198 | 198 | 0 |
    /// | 3 | 163 | 163 | 0 |
    /// | 4 | 180 | 180 | 0 |
    ///
    /// A frame reaches its opening width and stops, and after T3 the reason at
    /// 2 players is STRUCTURAL rather than incidental. At 4 the 180-column
    /// board is wider than anything stacked under it and at 3 the fixed
    /// site-slot legend is — both unchanged. At 2 the dashboard joined beside
    /// the board sets the width, and every one of its lines is clipped at
    /// [`DASH_W`] by [`panel_render`], with the section rules sitting at exactly
    /// that width from turn 1: the widest line is already at its ceiling before
    /// a single move is applied, so there is nothing left for play to grow. The
    /// old panel had no such rule and reached 164 by measuring its own content.
    ///
    /// It is a MARGIN, not a second floor: what it has to stay is a bound a
    /// growing line can actually break. [`FLOOR_W`] alone leaves 2 to 37
    /// columns of slack depending on the count, which is more room than the two
    /// lines the original pin caught (the 142-column Movement hint and the
    /// 150-column choice status line) ever needed to hide in; at one column of
    /// allowance that class of growth fails again.
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
    ///
    /// **It measures HEIGHT too since the T3 review**, against
    /// [`WALK_HEIGHTS`], which is why the name no longer says "the raised
    /// floor". The review's finding was an asymmetry rather than a broken
    /// bound: widths were held over this walk AND every constructed worst case,
    /// while the only height ever measured was the OPENING frame's, so a 2P
    /// frame reaching 66 rows in play against a 46-row opening was invisible by
    /// construction. The same walk answers both questions and costs one
    /// traversal, so the height bound rides here rather than in a second test
    /// that would replay the identical 250 moves.
    #[test]
    fn every_frame_of_a_real_game_walk_fits_its_recorded_bounds() {
        let mut report = Vec::new();
        let mut widest = Vec::new();
        let mut tallest = Vec::new();
        for n in [2u8, 3, 4] {
            let mut r = repl_np(n);
            force(true);
            let mut steps = 0usize;
            let mut worst = (0usize, String::new());
            let mut worst_h = (0usize, String::new());
            while !r.session.is_over() && steps < 250 {
                let frame = wrap(&r, &r.prompt());
                let label = format!("move {steps}");
                let w = measure_frame(n, &label, &frame, &mut report);
                if w > worst.0 {
                    worst = (w, label.clone());
                }
                let h = frame.lines().count();
                if h > worst_h.0 {
                    worst_h = (h, label);
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
            tallest.push((n, worst_h.0, worst_h.1));
        }
        assert!(
            report.is_empty(),
            "{}\nwidest line per count:\n{}",
            report.join("\n"),
            widest_report(&widest),
        );

        // Reported before asserted, for the reason `measure_frame` is: a run
        // that stops at the first over-tall count cannot say whether the other
        // two moved, and it is exactly "what did every count reach" that sets
        // the table these bounds are read off.
        let mut too_tall = Vec::new();
        for (n, rows, label) in &tallest {
            let recorded = WALK_HEIGHTS[usize::from(*n) - 2];
            if *rows > recorded + WALK_HEIGHT_MARGIN {
                too_tall.push(format!(
                    "  {n}P reached {rows} rows at '{label}', past its recorded in-play maximum \
                     {recorded} + WALK_HEIGHT_MARGIN {WALK_HEIGHT_MARGIN}",
                ));
            }
        }
        assert!(
            too_tall.is_empty(),
            "{}\ntallest frame per count:\n{}\nIf the growth is deliberate, RE-MEASURE the whole \
             WALK_HEIGHTS table and state the new numbers there -- do not raise one constant to \
             whatever silences this run",
            too_tall.join("\n"),
            tallest
                .iter()
                .map(|(n, rows, label)| format!(
                    "  {n}P: tallest {rows} rows at '{label}', against a recorded {} (opening {})",
                    WALK_HEIGHTS[usize::from(*n) - 2],
                    FRAME_HEIGHTS[usize::from(*n) - 2],
                ))
                .collect::<Vec<_>>()
                .join("\n"),
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
        assert!(!f.contains("PLAYERS"), "dashboard must be hidden after toggle:\n{f}");
        assert!(!f.contains("YOUR HAND"), "...the whole dashboard, not only its first panel:\n{f}");
        r.step("panel");
        assert!(wrap(&r, "").contains("PLAYERS"), "dashboard must return on second toggle");

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

#[cfg(test)]
mod capture {
    use super::*;
    use crate::repl::Repl;
    use crate::session::Session;
    use yokohama_core::*;

    /// The committed FULL-FRAME capture the orchestrator and the owner `cat`
    /// against the approved mock — held as a CHECKED golden rather than dumped
    /// once, so it cannot quietly stop being what the code renders.
    ///
    /// **Whole frame, not a region.** T2's `t2-board-2p.txt` captured the board
    /// alone because that was all T2 had cut over; T3 added the top half. T4
    /// finishes it, so the golden is now the whole surface the mock is: title
    /// strip, board, dashboard beside it, legend, info strip, rule, and the
    /// decision zone — phase chain, deciding line, numbered option, standing
    /// hint, caret. Compare its tail against `main-frame.ans` directly; the two
    /// deliberate omissions from the mock's hint (T9's recall key, T5's card
    /// index) are recorded in `decision_render::hint_line`.
    ///
    /// Plain text, because a capture full of escapes is a capture nobody can
    /// read in a diff. Regenerate with `UPDATE_T3_CAPTURE=1 cargo test -p
    /// yokohama-cli --lib the_committed_frame_capture` after a deliberate
    /// change, and LOOK AT THE DIFF before committing it.
    ///
    /// The variable is checked for PRESENCE, not for a value: `UPDATE_T3_CAPTURE=0`
    /// and `UPDATE_T3_CAPTURE=` bless the capture exactly as `=1` does. That is
    /// the usual shape for a one-shot regeneration switch and it is stated here
    /// because `=0` looks like it would decline.
    #[test]
    fn the_committed_frame_capture_is_what_the_screen_renders() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/plans/2026-08-05-phase3.5a-mocks/captures/t3-frame-2p.txt",
        );
        let repl = Repl::new(
            Session::new_sim(
                GameConfig::for_player_count(2),
                7,
                vec![StartingOrderChoice::Keep(0); 2],
                Rules::standard(),
            )
            .expect("a 2P session"),
        );
        force(true);
        let frame = wrap(&repl, &repl.prompt());
        force(false);
        // The clear-screen + cursor-home prefix is a control sequence, not
        // content: it would render as mojibake in a file meant to be read.
        let rendered = format!("{}\n", frame.trim_start_matches("\u{1b}[2J\u{1b}[H"));

        if std::env::var_os("UPDATE_T3_CAPTURE").is_some() {
            std::fs::create_dir_all(std::path::Path::new(path).parent().expect("a parent"))
                .expect("the captures directory");
            std::fs::write(path, &rendered).expect("write the capture");
        }
        let committed = std::fs::read_to_string(path).expect("the committed capture");
        assert_eq!(
            committed, rendered,
            "the committed 2P frame capture is stale; regenerate it with UPDATE_T3_CAPTURE=1",
        );
    }

    /// **`stty size` reports ROWS then COLUMNS**, which is the order everyone
    /// who has assumed otherwise got wrong, so the parse is pinned by position
    /// rather than trusted to a comment (T7 review SHOULD-6).
    ///
    /// Every malformed shape falls to `None` and the caller then uses the
    /// designed floor: a terminal reporting nonsense mid-resize must not be
    /// able to lay the card index out one column wide.
    #[test]
    fn a_terminal_size_report_is_read_rows_then_columns_and_nonsense_is_refused() {
        assert_eq!(parse_stty_size("40 120\n"), Some(120), "the SECOND field is the width");
        assert_eq!(parse_stty_size("50 200"), Some(200));
        assert_eq!(parse_stty_size("  24   80  \n"), Some(80), "whitespace is not significant");
        for junk in ["", "40", "40 x", "rows 40 cols 120", "\n", "40 -5"] {
            assert_eq!(parse_stty_size(junk), None, "{junk:?} is not a size report");
        }
        // A width below the sanity floor is no answer: terminals really do
        // report 0 while being resized.
        assert_eq!(parse_stty_size(&format!("40 {}", MIN_SANE_WIDTH - 1)), None);
        assert_eq!(parse_stty_size(&format!("40 {MIN_SANE_WIDTH}")), Some(MIN_SANE_WIDTH));
    }

    /// **`force` cannot make this process shell out** (T7 review SHOULD-6).
    ///
    /// `terminal_width` reads the `ENABLED` atomic rather than `enabled()`
    /// precisely so a test forcing FRAMES on does not also start measuring the
    /// developer's window — which would make the suite pass at 200 columns and
    /// fail at 120, for reasons no failure message would mention. Only
    /// `init_from_tty` sets that atomic, and only `main` calls it.
    #[test]
    fn forcing_frames_on_does_not_make_a_test_measure_the_real_terminal() {
        clear_forced_width();
        force(true);
        // The fence itself, asserted rather than inferred from a `None` that a
        // machine with no `/dev/tty` would produce anyway.
        assert!(!may_measure(), "forcing frames on must not license a measurement");
        assert_eq!(terminal_width(), None, "a forced frame is not a terminal");
        assert_eq!(index_width(), FLOOR_W, "so the index falls back to the design floor");
        force(false);
        assert_eq!(terminal_width(), None);
        assert_eq!(index_width(), FLOOR_W);

        // ...and a test that DOES want a width says so, without a terminal.
        force_width(Some(120));
        assert_eq!(index_width(), 120);
        force_width(None);
        assert_eq!(index_width(), FLOOR_W, "'no terminal' is sayable and is not 'unset'");
        clear_forced_width();
        assert_eq!(index_width(), FLOOR_W);
    }
}
