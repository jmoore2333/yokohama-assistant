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
//! Frame anatomy, and it differs by ARRANGEMENT rather than only by size.
//! BESIDE (2 players, where the board and the designed dashboard both fit
//! `FLOOR_W`): title strip, the board with the dashboard joined to its right,
//! the board legend. BELOW (3 and 4, where they do not): title strip, the
//! board, the legend directly under it, then the dashboard laid out in seat
//! COLUMNS across `below_block_width` (T10). Both then close with the
//! discoverability strip, a rule, and the body -- the step's delta/error lines
//! plus the normal prompt block, unchanged from piped mode, since nothing
//! exists only in the frame.
//!
//! Which arrangement runs is derived from the geometry, not from a player
//! count: std exposes no terminal size, so what the frame itself measures is
//! the only honest signal.
//!
//! **Every region of the frame at every player count is now the redesigned
//! one** — T4 finished the regions and T10 finished the arrangements, which is
//! why all three counts open inside the 200x50 design floor (see
//! [`FRAME_HEIGHTS`]).

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
/// **A DIFFERENT element is the widest one at each count**, which is why the
/// three numbers do not track the board: at 2 it is the board with the
/// dashboard joined beside it, at 120 + [`PANEL_GAP`] + [`DASH_W`] = 198; at 4,
/// since T10, it is the BELOW DASHBOARD, whose four seat columns of
/// [`panel_render::SEAT_COL_W`] span 200 — see [`below_block_width`] for why
/// four columns of the 180-column board would be too narrow to draw a seat in;
/// and at 3, since the T10 review, it is that same below dashboard held open to
/// [`panel_render::BELOW_BLOCK_MIN`] by the widest legal ACHIEVEMENTS row.
///
/// The history in one line, because four changes moved these: the pre-redesign
/// numbers were 114/132/108; T2's V3 board took them to 163/163/180 and then
/// 164/163/180 when [`PANEL_GAP`] widened to the mocks' four-column gutter; T3
/// replaced the 40-column compact panel with the designed 74-column dashboard,
/// which put the 2P join at 198; T10 laid the 3P and 4P dashboards out in
/// columns, which left 163 alone and took 180 to 200; and the T10 review found
/// that 163 was too narrow for the compressed rows the column layout had just
/// created, which took 3 players from 163 to 175.
///
/// **The 3-player number is DERIVED, not chosen.** It is the widest legal
/// compressed panel row plus a column, swept over every card triple and every
/// claimant state — the table is in [`panel_render::BELOW_BLOCK_MIN`], with the
/// 580-of-32,768 clip rate that made it necessary. Nothing about the site-slot
/// legend or the 3-player board sets that frame's width any more.
///
/// **198, 175 and 200 are the design working, not near misses.** Spec §0.1
/// designs the frame at 200 columns and every approved mock is drawn at exactly
/// that — the 4P mock's own closing rule measures 200. At 2 players, 120 + 4 +
/// 74 is what "the board and the dashboard side by side inside 200" costs; at
/// 4, four seat columns is what the floor is for; at 3, the design floor's 37
/// columns of headroom over the old 163 is what pays for full words in every
/// legal state rather than an abbreviation. What that costs is SLACK: the 4P
/// frame sits exactly on [`FLOOR_W`], so the floor rather than `FRAME_MARGIN`
/// is the binding bound there, which is the failure
/// `frame_fits_the_raised_floor_at_every_player_count` says is the right one to
/// get.
const FRAME_WIDTHS: [usize; 3] = [198, 175, 200];

/// The composed frame's HEIGHT at each player count, indexed `players - 2` --
/// the three numbers guide §2 states, measured the same way [`FRAME_WIDTHS`] is
/// and pinned by `the_opening_frame_is_the_recorded_size_at_every_count`.
///
/// **ALL THREE are inside the design floor's 50 rows since T10, and the two
/// that were not are the whole of what T10 was for.** Spec §0.1 designs the
/// frame at 200x50. Through T3-T9 the 3- and 4-player frames opened at 79 and
/// 87 rows, because the dashboard that joins BESIDE a 120-column board had
/// nowhere to go under a 150- or 180-column one but straight down, eight rows
/// per seat: on a 50-row terminal the title strip and the top board rows had
/// scrolled off before the player could read them, which is F3 exactly,
/// returning by a different door. The number was written down at each task
/// rather than a bound being quietly relaxed to cover it.
///
/// **T10's fix is a layout change, not a trim** — the approved `frame-4p.ans`
/// arrangement, made total. The players panel turns on its side (one column per
/// seat, [`panel_render::SEAT_ROWS`] rows for all of them together instead of
/// eight rows each), and the other three panels compress onto one titled row
/// apiece. That is 15 rows at both counts against 45 and 54, and it lands the
/// openings at 49 and 48.
///
/// **The three OPENING numbers no longer move independently, and that is a
/// property rather than a coincidence.** The below block is FIXED-height by
/// construction (see [`panel_render::SEAT_ROWS`]), so what separates 49 from 48
/// is the one row 3 players pays elsewhere: the standing hint WRAPS to the
/// frame's own width, and at 175 columns its clause list still needs two lines
/// where 198 and 200 take one. Everything else about the two frames is the same
/// size.
///
/// That property is about the OPENING frame and about the 3- and 4-player
/// arrangement. It does NOT extend to what a frame reaches in play at 2
/// players, where the dashboard is ragged and grows with holdings —
/// [`WALK_HEIGHTS`] is where that distinction is drawn, and T10 blurred it.
///
/// `cfg(test)`, unlike [`FRAME_WIDTHS`]: a width is a BUDGET the running driver
/// spends (`suggest` sizes its body against `frame_width`), while a height is
/// only ever a measurement and a pin. Nothing at runtime asks how tall its own
/// frame is, so a production constant here would be dead code claiming to be
/// used.
#[cfg(test)]
const FRAME_HEIGHTS: [usize; 3] = [48, 49, 48];

/// The design floor's height (spec §0.1's 200x50), which every entry of
/// [`FRAME_HEIGHTS`] meets since T10. `cfg(test)` for the same reason.
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
/// **MEASURED, then set** — the same discipline as [`FRAME_MARGIN`], and
/// **RE-MEASURED at T4, at T10, and again at the T10 review**, which is the
/// condition each revision of this doc attached to the table:
///
/// | count | opening | tallest in play | overshoot | measured on | (T10's reading) | (T4's) | (T3's) |
/// |---|---|---|---|---|---|---|---|
/// | 2 | 48 | 67 | +19 | seed 18, rot 2, 900 moves | 60, +12 | 60, +12 | 66, +20 |
/// | 3 | 49 | 60 | +11 | seed 7, rot 0, 250 moves | 60, +11 | 90, +12 | 96, +20 |
/// | 4 | 48 | 60 | +12 | seed 7, rot 0, 250 moves | 60, +12 | 99, +12 | 105, +20 |
///
/// **THE THREE ENTRIES ARE NOT THE SAME KIND OF NUMBER, and T10 claimed they
/// were.** Its commit read "all three in-play maxima are now the SAME number,
/// which is what a fixed-height dashboard buys" — true of 3 and 4 players and
/// false of 2, which shares none of that layout. The review reproduced a 2P
/// frame at 66 rows on the parent commit and this table's own deep walk finds
/// 67, against the 60 T10 recorded from a 250-move walk. Nothing regressed; the
/// number had never been looked for.
///
/// - **3 and 4 players are CEILINGS.** Their dashboard is a fixed-height block
///   (`panel_render::SEAT_ROWS`), so nothing a seat collects can add a row and
///   the only thing that grows those frames is the decision zone's own
///   twelve-row page (`decision_render::OPTION_ROWS`). Both peak at the walk's
///   move 7 — an AreaAction on the Church with 27 payment combinations — and a
///   longer walk does not find a taller frame.
/// - **2 players is NOT a ceiling.** Its dashboard joins BESIDE the board and
///   is ragged: one line per owned Technology, per hand card, per panel that
///   fills up. It therefore grows with holdings for as long as a game runs, and
///   the recorded figure is a property of the WALK rather than of the layout —
///   a 4,000-move version of the same walk reaches 68 and is still climbing
///   when it stops. What 67 means is exactly "what the walk pinned in
///   `every_frame_of_a_real_game_walk_fits_its_recorded_bounds` draws", which is
///   all a regression net needs: that walk is deterministic, so a layout change
///   that adds a row to the 2P dashboard moves this number and fails.
///
/// The 2P entry is deliberately NOT quoted as a bound anywhere a player reads
/// (guide §2 says what the shape is instead), and fixing the raggedness itself
/// is a layout question this task did not open.
///
/// **What the overshoot is, at every count, is the zone's own budget rather
/// than a defect with a fix pending.** A decision with more options than one
/// screen holds has to either page or grow, and this driver pages; twelve rows
/// is the deliberate size of that page.
///
/// A number that is recorded is not a number that is licensed: growth PAST these
/// fails, and the fix is to re-measure the table deliberately, not to raise one
/// entry until a run goes quiet.
#[cfg(test)]
const WALK_HEIGHTS: [usize; 3] = [67, 60, 60];

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
        return full_screen(body);
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

/// A body that REPLACES the frame instead of sitting under it: the clear-and-
/// home prefix, and nothing else.
///
/// Extracted from [`wrap`]'s overlay branch when the load-time bot-turn review
/// became a second caller. The review is not an [`crate::repl::Overlay`] — it is
/// put up before the REPL loop starts and taken down by one keypress, so it
/// never passes through `wrap` at all — but it has to arrive on a cleared screen
/// exactly as an overlay does, and one function knowing the prefix is what keeps
/// the two from drifting into different kinds of "full screen".
pub(crate) fn full_screen(body: &str) -> String {
    format!("\u{1b}[2J\u{1b}[H{body}")
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
/// **The two arrangements differ in their ORDER as well as their shape.**
/// Beside the board the dashboard is joined to it and the site-slot legend
/// follows both. Below it, the legend comes FIRST — directly under the board it
/// explains — and the dashboard block follows. That was not a free choice: in
/// the transitional stacked form the legend sat 45 rows under its board, which
/// is a caption nobody reads.
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
    let legend = render::board_legend(state, rules);

    let mut stage = String::new();
    let seats = repl.seat_display();
    let beside = board_width + PANEL_GAP + DASH_W <= FLOOR_W;
    if repl.show_panel && beside {
        let dashboard = render::dashboard(state, rules, &seats, state.current_player);
        stage.push_str(&join_side_by_side(&board, &dashboard, PANEL_GAP));
        push_block(&mut stage, &legend);
    } else {
        push_block(&mut stage, &board);
        push_block(&mut stage, &legend);
        if repl.show_panel {
            let width = below_block_width(state.players.len(), &stage);
            push_block(
                &mut stage,
                &render::dashboard_below(state, rules, &seats, state.current_player, width),
            );
        }
    }
    // Discoverability strip (owner scope item 10): every frame names the
    // game-info surfaces and the command that opens them, so a player who
    // never read the guide finds them on screen.
    stage.push_str(
        "info: type 'view' — board · players · ports · labs · tracks · sites · goals · hand · score · log   ·   '?' — the card index\n",
    );
    stage
}

/// Append `block` to `stage`, newline-terminated exactly once.
fn push_block(stage: &mut String, block: &str) {
    if block.is_empty() {
        return;
    }
    stage.push_str(block);
    if !stage.ends_with('\n') {
        stage.push('\n');
    }
}

/// The columns the BELOW dashboard is laid out across (T10).
///
/// THREE numbers, whichever is largest, and each one is a different thing the
/// block has to hold:
///
/// 1. what `seats` columns of [`panel_render::SEAT_COL_W`] cost — the block the
///    approved 4P mock draws, four columns of 50 being 200, the design floor
///    and that mock's own frame width;
/// 2. what the stage ALREADY occupies, so a dashboard laid under a wide board
///    spans it rather than sitting short of it;
/// 3. [`panel_render::BELOW_BLOCK_MIN`], the widest LEGAL compressed panel row
///    plus a column — added at the T10 review, which measured the achievements
///    row at 171 columns against a 3-player block of 163.
///
/// **The third is what sets the 3-player frame**, and it is why that frame is
/// 175 columns rather than the 163 it had been since T2. The 3-player board is
/// 150 columns and the site-slot legend 163, so before the review neither (1)
/// nor (2) reached what the compressed rows need and the achievements row
/// clipped its claim state in 580 of its 32,768 legal states. At 4 players (1)
/// gives 200 and dominates both others, so nothing there moved.
///
/// (2) is measured off the stage TEXT rather than off a table, so a legend or a
/// board that changes width moves this with it and cannot leave the block
/// short.
///
/// **KNOWN-EQUIVALENT MUTANT (T10 review N3, recorded so it is not re-derived):
/// (2) is inert at both shipped geometries** and has been since (3) was added —
/// the 3-player legend is 163 and (3) gives 175, the 4-player board is 180 and
/// (1) gives 200 — so deleting it fails no test today. It stays because it is
/// the only term that tracks the stage: a board or a legend that grew past 200
/// columns would leave the dashboard short of it, and no other term would
/// notice.
fn below_block_width(seats: usize, stage_so_far: &str) -> usize {
    let occupied = stage_so_far.lines().map(visible_width).max().unwrap_or(0);
    (seats * panel_render::SEAT_COL_W).max(occupied).max(panel_render::BELOW_BLOCK_MIN)
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

        // Whichever way it is arranged, the dashboard carries every seat --
        // but SIDEWAYS since T10. The stacked branch once folded to one line
        // per player and dropped fields to do it; T3 gave it the same vertical
        // block the 2-player frame has; T10 turns that block on its side, so
        // all four seats now share each of ten rows rather than owning eight
        // rows each. The pin follows the arrangement: one `Mercenaries ` ROW
        // carrying four of them.
        let r4 = repl_np(4);
        let f4 = wrap(&r4, "");
        let merc: Vec<&str> = f4.lines().filter(|l| l.contains("Mercenaries ")).collect();
        assert_eq!(merc.len(), 1, "the columns put every seat on one row:\n{f4}");
        assert_eq!(
            merc[0].matches("Mercenaries ").count(),
            4,
            "...and that row must carry all four seats: {:?}",
            strip_ansi(merc[0]),
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
                "the {n}P frame composes at {measured} columns, so FRAME_WIDTHS must say so — \
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
    /// **This was a flat `rows <= 50` until T3, became a RECORDED table plus
    /// that bound at the two counts that still held it, and since T10 is the
    /// table AND the bound at all three.** The history is worth keeping because
    /// it is the shape of an honest regression: T3 replaced the stacked
    /// one-line-per-player fold -- which existed precisely to hold this bound --
    /// with the full designed dashboard at every count (the task's own scope),
    /// and at 3 and 4 players that block ran the frame to 79 and 87 rows. The
    /// number was written down at each task rather than the bound being
    /// relaxed, and T10 re-laid the block into seat COLUMNS, which brought both
    /// counts back inside the floor. See [`FRAME_HEIGHTS`] for the arithmetic.
    ///
    /// An EQUALITY table is a stronger pin than the inequality it replaces, not
    /// a weaker one: it fails on a frame that grows by one row and on a frame
    /// that shrinks by one, so the numbers cannot drift while nobody is
    /// looking, and `guide_sync` independently requires guide §2 to state these
    /// same three. The FLOOR is asserted separately and at EVERY count, because
    /// the two say different things -- the table says what the frame is, the
    /// floor says what a tester's 200x50 window can hold.
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
                "the {n}P frame opens at {rows} rows, so FRAME_HEIGHTS must say so — \
                 and guide §2 (checked independently by guide_sync) must state the same number"
            );
        }
        for (i, rows) in FRAME_HEIGHTS.iter().enumerate() {
            assert!(
                *rows <= FLOOR_H,
                "the {}P frame must open inside the {FLOOR_H}-row design floor; it is {rows} rows",
                i + 2,
            );
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
    /// **MEASURED, then set — and RE-MEASURED at T3, at T10 and at the T10
    /// review**, which is the condition each revision of this doc attached to
    /// the constant. The instrumentation is re-run over both pins below — every
    /// constructed worst case (turn 99, 199 money and VP, 15 of every good on
    /// every seat, all seven pending-choice banners, a lockdown) and the game
    /// walks, at each count — and the overshoot has come back at zero every
    /// time, so the constant stays at one and the table states the current
    /// openings:
    ///
    /// | count | opening | widest measured | overshoot | (T10's reading) | (T3's) |
    /// |---|---|---|---|---|---|
    /// | 2 | 198 | 198 | 0 | 198 | 198 |
    /// | 3 | 175 | 175 | 0 | 163 | 163 |
    /// | 4 | 200 | 200 | 0 | 200 | 180 |
    ///
    /// A frame reaches its opening width and stops, and at every count the
    /// reason is now STRUCTURAL rather than incidental. At 2 the dashboard
    /// joined beside the board sets it, and every one of its lines is clipped at
    /// [`DASH_W`] by [`panel_render`], with the section rules sitting at exactly
    /// that width from turn 1. At 3 and 4, since T10, the below dashboard sets
    /// it: its PLAYERS rule spans the block exactly and every other line in it
    /// is clipped there, so the widest line is at its ceiling before a move is
    /// applied. The old panel had no such rule and reached 164 by measuring its
    /// own content.
    ///
    /// It is a MARGIN, not a second floor: what it has to stay is a bound a
    /// growing line can actually break. [`FLOOR_W`] alone leaves 0 to 37
    /// columns of slack depending on the count — nothing at all at 4 players
    /// now, which is why the floor is the binding bound there and this margin
    /// is the binding one at 2 and 3. At one column of allowance the class of
    /// growth the original pin caught (the 142-column Movement hint, the
    /// 150-column choice status line) fails again.
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
    ///
    /// **FOUR walks since the T10 review, not three, and the fourth is a 2P
    /// walk that runs much deeper.** The 3- and 4-player dashboards are
    /// fixed-height blocks, so 250 moves finds their ceiling; the 2-player one
    /// is ragged and grows with what its two seats own, so how tall a 2P frame
    /// gets is a question about how long you walk, not about the layout. The
    /// review found the shipped table's 2P entry understated by six rows for
    /// exactly that reason. The deep arm is the walk [`WALK_HEIGHTS`]'s 2P entry
    /// is measured on, stated here so the number and the thing that produces it
    /// cannot drift apart.
    #[test]
    fn every_frame_of_a_real_game_walk_fits_its_recorded_bounds() {
        let mut report = Vec::new();
        let mut widest: Vec<(u8, usize, String)> = Vec::new();
        let mut tallest: Vec<(u8, usize, String)> = Vec::new();
        // (count, seed, rotation offset, step cap). The first three are the
        // walk this test has always run; the fourth is the T10-review deep 2P
        // arm -- see the doc above and [`WALK_HEIGHTS`].
        for (n, seed, rot, cap) in [(2u8, 7u64, 0usize, 250usize), (3, 7, 0, 250), (4, 7, 0, 250), (2, 18, 2, 900)] {
            let mut r = Repl::new(
                Session::new_sim(
                    GameConfig::for_player_count(n),
                    seed,
                    vec![StartingOrderChoice::Keep(0); usize::from(n)],
                    Rules::standard(),
                )
                .expect("a session"),
            );
            force(true);
            let mut steps = 0usize;
            let mut worst = (0usize, String::new());
            let mut worst_h = (0usize, String::new());
            while !r.session.is_over() && steps < cap {
                let frame = wrap(&r, &r.prompt());
                let label = format!("seed {seed} rot {rot} move {steps}");
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
                let mv = legal[(steps + rot) % legal.len()].clone();
                if r.session.apply(mv, Vec::new()).is_err() {
                    break;
                }
                steps += 1;
            }
            force(false);
            assert!(
                steps > 100,
                "the {n}P walk (seed {seed}, rot {rot}) must actually reach mid-game positions, \
                 only got {steps} moves",
            );
            // One row per COUNT, keeping whichever arm reached furthest, so the
            // reports below still read as a per-count table.
            let keep_max = |acc: &mut Vec<(u8, usize, String)>, got: (usize, String)| {
                match acc.iter_mut().find(|(c, _, _)| *c == n) {
                    Some(slot) if got.0 > slot.1 => {
                        slot.1 = got.0;
                        slot.2 = got.1;
                    }
                    Some(_) => {}
                    None => acc.push((n, got.0, got.1)),
                }
            };
            keep_max(&mut widest, worst);
            keep_max(&mut tallest, worst_h);
        }
        widest.sort_by_key(|(n, _, _)| *n);
        tallest.sort_by_key(|(n, _, _)| *n);
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

    /// The BELOW arrangement's own sweep (T10), at the two counts that use it
    /// and over every position the constructed worst cases reach.
    ///
    /// Three properties the per-count width table cannot see, because it
    /// measures one number off one frame:
    ///
    /// 1. **The columns line up, on the stride the block designs.** Seat
    ///    columns are padded to a stride, and a stride that stopped being
    ///    uniform would still pass every width bound while drawing a ragged
    ///    dashboard. Derived from the rendered frame — the offsets of the
    ///    repeated `Assistants  ` label — rather than from the constant that
    ///    produced them, so the pin fails on a padding bug and not only on a
    ///    changed constant.
    ///
    ///    **An EQUALITY since the T10 review**, which found the old
    ///    `stride >= SEAT_COL_W` was a floor rather than the property: padding
    ///    the columns with the constant instead of the block-derived `col_w`
    ///    left every property test green and was caught only by the committed
    ///    golden. At 4 players the two numbers coincide (200 / 4 == 50), so the
    ///    3-player cell is the one with the grip — 175 / 3 == 58 against the
    ///    constant's 50.
    /// 2. **The block is fixed-height.** Every worst case must draw the same
    ///    number of dashboard rows as the opening frame; that is what keeps the
    ///    3P and 4P frames inside the design floor for a whole game rather than
    ///    only at turn 1.
    /// 3. **The legend follows its board.** The below arrangement puts it
    ///    directly under the board it explains; in the transitional stacked
    ///    form it sat 45 rows below, which is a caption nobody reads.
    #[test]
    fn the_below_arrangement_is_aligned_fixed_height_and_legend_first() {
        // The dashboard rows of a composed frame: from the PLAYERS rule to the
        // discoverability strip that closes every stage.
        let dashboard_rows = |frame: &str| -> Vec<String> {
            let lines: Vec<String> = frame.lines().map(strip_ansi).collect();
            let start = lines.iter().position(|l| l.starts_with("─ PLAYERS")).expect("a PLAYERS rule");
            let end = lines.iter().position(|l| l.starts_with("info: type 'view'")).expect("the info strip");
            lines[start..end].to_vec()
        };

        for n in [3u8, 4] {
            let opening = repl_np(n);
            force(true);
            let first = wrap(&opening, &opening.prompt());
            force(false);

            // (3) legend under its board, both above the dashboard.
            let plain: Vec<String> = first.lines().map(strip_ansi).collect();
            let last_tile = plain.iter().rposition(|l| l.contains('└')).expect("a board");
            let legend = plain.iter().position(|l| l.contains("site slots")).expect("a legend");
            let players = plain.iter().position(|l| l.starts_with("─ PLAYERS")).expect("a dashboard");
            assert!(
                last_tile < legend && legend < players,
                "{n}P: the legend must sit between the board and the dashboard \
                 (board ends {last_tile}, legend {legend}, dashboard {players})",
            );

            let opening_rows = dashboard_rows(&first).len();
            for (label, frame) in worst_case_frames(n) {
                let rows = dashboard_rows(&frame);

                // (2) fixed height, at every position.
                assert_eq!(
                    rows.len(),
                    opening_rows,
                    "{n}P '{label}': the below dashboard must be a fixed-height block, \
                     and this one is {} rows against the opening's {opening_rows}",
                    rows.len(),
                );

                // (1) uniform stride, measured off the rendered row.
                let zones = rows
                    .iter()
                    .find(|l| l.matches("Assistants  ").count() > 1)
                    .unwrap_or_else(|| panic!("{n}P '{label}': one row must carry every seat's zones:\n{rows:#?}"));
                let offsets: Vec<usize> = zones
                    .char_indices()
                    .filter(|(i, _)| zones[*i..].starts_with("Assistants  "))
                    .map(|(i, _)| zones[..i].chars().count())
                    .collect();
                assert_eq!(offsets.len(), usize::from(n), "{n}P '{label}': one column per seat");
                let stride = offsets[1] - offsets[0];
                for (i, off) in offsets.iter().enumerate() {
                    assert_eq!(
                        *off,
                        offsets[0] + i * stride,
                        "{n}P '{label}': seat columns must sit on a uniform {stride}-column \
                         stride, and these are at {offsets:?}",
                    );
                }
                // ...and it is the DESIGNED stride, not merely a wide enough
                // one. Both halves derived from this very frame: the PLAYERS
                // rule spans the block exactly (`panel_render::header_at`), so
                // the block is its width, and the stride the layout owes is
                // that block shared between the seats, floored at the designed
                // minimum.
                let block = rows[0].chars().count();
                let designed = (block / usize::from(n)).max(panel_render::SEAT_COL_W);
                assert_eq!(
                    stride, designed,
                    "{n}P '{label}': the seat columns sit on a {stride}-column stride, but a \
                     {block}-column block shared between {n} seats designs {designed} -- a \
                     stride that is merely WIDE ENOUGH leaves the block's right-hand columns \
                     unused and every seat's content shifted left of where it belongs",
                );
            }
        }
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
        crate::view_render::tests::assert_capture("t3-frame-2p.txt", &rendered);
    }

    /// The BELOW arrangement's committed captures (T10), one per count that
    /// uses it — the artifact the orchestrator and the owner `cat` against the
    /// approved `frame-4p.ans`, held as CHECKED goldens for the reason the
    /// 2-player one is: a capture dumped once stops being what the code
    /// renders and nobody finds out.
    ///
    /// **Both counts, because 3 players is not a narrow 4.** Its block is set
    /// by [`panel_render::BELOW_BLOCK_MIN`] rather than by four seat columns
    /// (175 against 200), its columns are 58 wide rather than 50, and its
    /// standing hint wraps where the others' does not — three differences that
    /// a 4-player capture alone would show none of.
    ///
    /// Plain text, and regenerated with `UPDATE_T10_CAPTURE=1 cargo test -p
    /// yokohama-cli --lib the_committed_below_frame_captures` after a
    /// deliberate change. LOOK AT THE DIFF before committing it. The variable
    /// is checked for PRESENCE, not for a value (see the 2-player capture).
    #[test]
    fn the_committed_below_frame_captures_are_what_the_screen_renders() {
        for n in [3u8, 4] {
            let repl = Repl::new(
                Session::new_sim(
                    GameConfig::for_player_count(n),
                    7,
                    vec![StartingOrderChoice::Keep(0); usize::from(n)],
                    Rules::standard(),
                )
                .expect("a session"),
            );
            force(true);
            let frame = wrap(&repl, &repl.prompt());
            force(false);
            // The clear-screen + cursor-home prefix is a control sequence, not
            // content: it would render as mojibake in a file meant to be read.
            let rendered = format!("{}\n", frame.trim_start_matches("\u{1b}[2J\u{1b}[H"));
            crate::view_render::tests::assert_capture(&format!("t10-frame-{n}p.txt"), &rendered);
        }
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
