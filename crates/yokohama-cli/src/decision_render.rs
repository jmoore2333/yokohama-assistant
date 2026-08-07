//! The decision zone: the phase chain, the "YOU ARE DECIDING" line, the
//! numbered options, the dimmed near-miss rows and the standing hint — drawn
//! from the screen model alone (Phase-3.5a spec §0.7/§0.8, plan T4; design
//! authority is the approved mock pair
//! `2026-08-05-phase3.5a-mocks/main-frame.ans` + `main-frame-mid.ans`).
//!
//! **And, since T8, the REVEAL zone** ([`reveal_zone`]) — the prompt that
//! replaces the decision zone while a live capture is waiting for a card. It is
//! here rather than in a module of its own because it is the same zone of the
//! screen answering a different question, and because it has to be *drawn* the
//! same way: same banner-then-blank-then-sentence rhythm, same [`wrap_hint`], a
//! caret on its own last line. Two modules would be two rhythms inside a week.
//!
//! **The third module on the R9 seam, held the same way its two siblings hold
//! it, and the import list is the proof.** This module renders; it never reads
//! game state. Its inputs are [`crate::screen_model`]'s pure data plus a seat
//! color table and already-worded option text, and the only `yokohama_core`
//! names it touches are the TYPES a row matches on. `GameState`, `Rules` and
//! `Session` are absent by construction — the adapter that reaches for them is
//! [`crate::render::decision_zone`], one function, on the other side of this
//! seam. Glyphs come only from [`crate::style::glyph`] and color only from
//! [`crate::style::Tone`].
//!
//! **Who words what, because the split is not obvious.** Three of the zone's
//! four lines are worded HERE, from typed model values: the phase chain
//! ([`crate::describe::phase_chain_name`] over a `TurnPhase`), the deciding
//! line (an exhaustive match on [`Deciding`], every `PendingChoice` variant
//! named, no `_` arm) and the near-miss rows (an exhaustive match on
//! [`NotOfferedReason`]). The option ROWS are the exception, and deliberately:
//! this module re-words the five move kinds the mocks actually draw — the
//! President's two moves, the two placements, the retreat — from the model, so
//! that the `◈` mark, the step count and the priced [`FeeModel`] all come from
//! typed data; every OTHER move prints the words `describe::describe_move`
//! already gives it, carried in on [`Row::Move::words`]. That is not a
//! shortcut. `describe_move` holds forty arms of engine-pinned preview text —
//! the Church's faith arithmetic, a production area's real yield at the current
//! power, the Customs spaces a payment reaches, a fifth technology's surcharge
//! — and re-authoring those here would be a rewrite of tested wording that no
//! mock asks for. Spec §0.2's editorial pass over that vocabulary is T11's.
//!
//! **Nothing here is ever the reason a move is or is not offered.** Rows come
//! from the caller's list, which comes from the engine's; the `◈` marks read
//! [`crate::screen_model::president_destinations`]; the near-miss rows state a
//! fact about the state and never a verdict about the rules.

use yokohama_core::{AreaId, Move, PendingChoice, PlayerColor, TurnPhase};

use crate::board_render::{color_of, Twin};
use crate::describe;
use crate::screen_model::{
    menu_number, BoardModel, Deciding, DecisionModel, FeeModel, NotOffered, NotOfferedReason,
};
use crate::style::{glyph, paint, Tone};

/// Columns a numbered option row is indented under the deciding line — the
/// mocks' own three-space step, which is what makes the rows read as a list
/// under a sentence rather than as more sentences.
const OPTION_INDENT: &str = "   ";

/// The most option ROWS one page of the zone draws.
///
/// **A row budget, not an option count, and that is the point.** The
/// pre-redesign zone paged at twenty OPTIONS and drew each on its own line, so
/// a Church payment decision — twenty-seven combinations, the tallest position
/// a real 2P game walk reaches — cost twenty rows of frame every time, and the
/// composed frame grew twenty rows past its opening height for one prompt
/// (`screen::WALK_HEIGHTS` recorded exactly that, +20 at all three counts).
/// Budgeting ROWS instead bounds the zone whatever the options do: a page of
/// short options packs into columns and fits more of them into the same twelve
/// rows, and a page of long ones falls back to a single column and still costs
/// twelve.
///
/// Twelve rather than twenty because the frame is the thing being fitted, and
/// rather than something smaller because a page that shows four options makes
/// the player hunt through `more` for a decision they can see all of today.
const OPTION_ROWS: usize = 12;

/// Columns between two option columns. Two spaces reads as a gutter; one reads
/// as a wrapped line.
const COLUMN_GAP: usize = 2;

/// The most columns the option list will pack into, however narrow its rows
/// are. Three columns of a short option still leave the eye a left-to-right
/// scan across ~60 columns per row; four turns the list into a table nobody
/// reads in order.
const MAX_COLUMNS: usize = 3;

/// One numbered row of the zone, as the caller's display list holds it.
///
/// The two variants are the two things a numbered entry can BE in this driver,
/// and they are not interchangeable: picking a [`Row::Move`] applies a move,
/// while picking a [`Row::Family`] opens a guided picker over a collapsed
/// family (the Movement route list, the retreat combinations, Placement A/B,
/// an assistant-move source). The bare-Enter ceremony's wording turns on
/// exactly that distinction, which is why the zone is told which one a row is
/// rather than inferring it from the text.
pub enum Row<'a> {
    /// One legal move, with its fee as the model priced it and its fallback
    /// wording as `describe` worded it.
    Move {
        mv: &'a Move,
        fee: Option<&'a FeeModel>,
        /// `describe::describe_move`'s line. Printed verbatim for every move
        /// kind this module does not re-word — see the module header on why
        /// that is most of them.
        words: &'a str,
    },
    /// A collapsed family, already worded by the caller (it knows the family
    /// SIZE, which is a fact about the legal list rather than about any one
    /// move).
    Family { words: &'a str },
}

/// Everything the zone needs beyond the model: which page to draw, and how the
/// hint line should read.
pub struct ZoneContext<'a> {
    /// 0-based page of the option list.
    pub page: usize,
    /// The expert grammar usable at THIS decision (`'a'/'b'`, `'r'`,
    /// `'m <from> <to>'`), or empty. Named on the hint because §0.2 keeps the
    /// grammars as aliases and an alias nobody can discover is a dead letter.
    pub expert: &'a str,
    /// The game is over: no options, no hint, just the notice.
    pub game_over: bool,
    /// The line editor is live, so ↑ really does recall what was typed.
    ///
    /// **A CALLER-SET flag, not a query.** The fact lives in `input`
    /// (`may_edit`), and a renderer that reached for it would be a renderer
    /// reading the environment — the seam R9 exists to keep clean. So the REPL
    /// asks and this carries the answer, exactly as `expert` carries which
    /// grammar the current decision accepts.
    pub recall: bool,
}

// ---------------------------------------------------------------------------
// The zone
// ---------------------------------------------------------------------------

/// The whole zone: chain, blank, deciding line, options, near-miss rows, hint.
///
/// The trailing hint line ends with a newline and the caller appends nothing —
/// `> ` is the zone's last line and the input caret sits on it, exactly as the
/// mocks draw it.
pub fn decision_zone(
    d: &DecisionModel,
    rows: &[Row<'_>],
    board: &BoardModel,
    colors: &[PlayerColor],
    ctx: &ZoneContext<'_>,
) -> String {
    let mut out = String::new();
    out.push_str(&phase_chain(d, colors));
    out.push_str("\n\n");
    out.push_str(&deciding_line(&d.deciding));
    out.push('\n');

    if ctx.game_over {
        out.push_str(&paint(Tone::Chrome, "(the game is over — commands only)"));
        out.push('\n');
    } else if rows.is_empty() {
        out.push_str(&paint(Tone::Chrome, "(no legal options right now — commands only)"));
        out.push('\n');
    } else {
        let (texts, widths) = option_texts(rows, board, colors);
        let budget = column_budget(board);
        let page = Page::of(&widths, budget, ctx.page);
        out.push_str(&option_block(&texts, &widths, &page, budget));
        for row in &d.not_offered {
            out.push_str(OPTION_INDENT);
            out.push_str(&paint(Tone::Dim, &not_offered_row(row, board)));
            out.push('\n');
        }
        // The near-miss list is CAPPED (`screen_model::NOT_OFFERED_ROWS`), and a
        // cap that fires silently reads as "those were all of them" — the exact
        // opposite of what a truncation means. Say so.
        if d.not_offered_suppressed > 0 {
            out.push_str(OPTION_INDENT);
            out.push_str(&paint(
                Tone::Dim,
                &format!("(…and {} more)", d.not_offered_suppressed),
            ));
            out.push('\n');
        }
        if page.total > 1 {
            out.push_str(OPTION_INDENT);
            out.push_str(&paint(
                Tone::Chrome,
                &pager_footer(rows.len() - page.end, page.number, page.total),
            ));
            out.push('\n');
        }
    }
    out.push_str(&hint_line(rows, ctx, board.player_count));
    out.push_str(&paint(Tone::Prompt, "> "));
    out.push('\n');
    out
}

// ---------------------------------------------------------------------------
// The phase chain
// ---------------------------------------------------------------------------

/// `TURN 9 · RED's turn   Additional ▸ Placement ▸  MOVEMENT  ▸ …` — the whole
/// chain, current step lit.
///
/// The lit step is drawn in reverse video and spaced out (` MOVEMENT `), which
/// is how the mocks make it findable at a glance without color: a player on a
/// `NO_COLOR` terminal still sees a block where the turn currently is. Every
/// other step is dimmed, so the line reads as one lit item in a run of
/// alternatives rather than as nine equal words.
fn phase_chain(d: &DecisionModel, colors: &[PlayerColor]) -> String {
    let mut t = Twin::default();
    t.tinted(Tone::Strong, &format!("TURN {} ", d.turn_number));
    t.tinted(Tone::Chrome, glyph::SEPARATOR.to_string().as_str());
    t.raw(" ");
    match color_of(colors, d.seat) {
        Some(c) => t.tinted(Tone::Player(c), &color_word(c).to_uppercase()),
        None => t.raw(&format!("SEAT {}", d.seat.0)),
    };
    t.tinted(Tone::Strong, "'s turn");
    t.raw("   ");

    for (i, &phase) in d.phase_chain.iter().enumerate() {
        if i > 0 {
            t.tinted(Tone::Chrome, &format!(" {} ", glyph::CHAIN_STEP));
        }
        let name = describe::phase_chain_name(phase);
        if i == d.current_phase {
            t.tinted(Tone::Highlight, &format!(" {} ", name.to_uppercase()));
        } else {
            t.tinted(Tone::Dim, name);
        }
    }
    t.painted
}

/// A seat color's display word. The zone spells colors itself rather than
/// borrowing the dashboard's helper: this is the only fact about a seat the
/// chain states, and both surfaces read the same [`PlayerColor`].
fn color_word(c: PlayerColor) -> &'static str {
    match c {
        PlayerColor::Red => "Red",
        PlayerColor::Blue => "Blue",
        PlayerColor::Yellow => "Yellow",
        PlayerColor::Green => "Green",
    }
}

// ---------------------------------------------------------------------------
// The deciding line
// ---------------------------------------------------------------------------

/// `YOU ARE DECIDING · <what, in words>` (spec §0.8).
///
/// **Exhaustive over both halves of [`Deciding`], with no `_` arm anywhere**,
/// which is the point of the model carrying a `PendingChoice` rather than a
/// pre-worded string: a new choice variant in the engine is a compile error
/// here, at the one line of the frame whose whole job is to say what the
/// engine is asking.
///
/// Each line says what is being DECIDED, not what the options cost — the
/// options say that themselves, in their own words, priced by the engine. That
/// division keeps this function free of state: it needs the choice's variant
/// and nothing else.
pub fn deciding_line(deciding: &Deciding) -> String {
    let what = match deciding {
        Deciding::GameOver => "nothing more — the game is over.".to_string(),
        Deciding::Choice { head, queued } => {
            let tail = if *queued > 1 {
                format!(" ({} more to resolve after this one)", queued - 1)
            } else {
                String::new()
            };
            format!("{}{tail}", choice_words(head))
        }
        Deciding::Phase(phase) => phase_words(*phase).to_string(),
    };
    format!("{} {} {what}", paint(Tone::Strong, "YOU ARE DECIDING"), glyph::SEPARATOR)
}

/// What a queued choice is asking. One arm per `PendingChoice` variant.
fn choice_words(head: &PendingChoice) -> String {
    match head {
        PendingChoice::AnyGood => {
            "which good to take — the reward is yours whichever you pick.".to_string()
        }
        PendingChoice::WarehouseTake => {
            "what to take out of your warehouse, or to decline — each option shows its price."
                .to_string()
        }
        PendingChoice::AssistantMove => {
            "which assistant to move, and where to — or to decline the move.".to_string()
        }
        PendingChoice::ChurchSpace { total_faith } => format!(
            "which Church space to take at faith {total_faith} — each shows its points and its rider.",
        ),
        PendingChoice::CustomsSpace { cost } => format!(
            "which Customs space to take for {cost} imported good{} — each shows its points and its rider.",
            if *cost == 1 { "" } else { "s" },
        ),
        PendingChoice::RecoveryLeave => {
            "whether to leave one assistant behind (Telephone) or recover them all.".to_string()
        }
        PendingChoice::StationPlace => {
            "where the Station token goes — every player reads +1 Power in the area you pick."
                .to_string()
        }
    }
}

/// What a phase is asking when no choice is queued. One arm per `TurnPhase`.
///
/// The Movement line names the `◈` mark, which is the one place the board and
/// the zone have to be read together (spec §0.7): the marks say WHICH tiles,
/// and this says why they are marked.
fn phase_words(phase: TurnPhase) -> &'static str {
    match phase {
        TurnPhase::AdditionalActionFirst => {
            "whether to use an additional action before Placement, or to move on."
        }
        TurnPhase::Placement => "where your assistants go — Option A spreads them, Option B doubles up.",
        TurnPhase::Movement => {
            "where your President moves — destinations need one of your assistants to stop on (◈ on the board)."
        }
        TurnPhase::AreaAction => "which action to take in the area your President stands in.",
        TurnPhase::FivePowerBonus => "whether to take this area's 5-Power token.",
        TurnPhase::Building => "whether to build here, and on which space.",
        TurnPhase::Recovery => "how to recover your assistants from the action area.",
        TurnPhase::AdditionalActionSecond => {
            "whether to use an additional action before your turn ends, or to end it."
        }
        TurnPhase::TurnEnd => "how this turn closes out before play passes on.",
    }
}

// ---------------------------------------------------------------------------
// The near-miss rows
// ---------------------------------------------------------------------------

/// `(not offered: TEA PLANTATION — you have no assistant there to stop on)`.
///
/// Exhaustive over [`NotOfferedReason`], no `_` arm: a reason the model can
/// state is a reason this line must be able to word.
fn not_offered_row(row: &NotOffered, board: &BoardModel) -> String {
    let reason = match row.reason {
        NotOfferedReason::StartedThereThisTurn => {
            "your President started the turn there and may not return this turn"
        }
        NotOfferedReason::CanalIsNeverAStop => "the Canal is a crossing, never a stop",
        NotOfferedReason::NoAssistantThere => "you have no assistant there to stop on",
    };
    format!("(not offered: {} {} {reason})", area_name(board, row.area), glyph::SECTION_DASH)
}

/// A board tile's display NAME, or its id when the board model does not hold
/// it — the same resolution [`crate::panel_render`] makes, for the same reason:
/// an [`AreaId`] is all a model carries and this module may not open a
/// `GameState` to name one.
fn area_name(board: &BoardModel, area: AreaId) -> String {
    board
        .tiles
        .iter()
        .find(|t| t.area == area)
        .map(|t| describe::area_type_name(t.area_type).to_uppercase())
        .unwrap_or_else(|| format!("A{:02}", area.0))
}

/// [`area_name`] plus the printed id, for a row naming an area the player may
/// have to TYPE.
///
/// **The split is the mock's, read as a rule rather than copied.** The mock's
/// Movement rows say `Move to CUSTOMS ◈` and no more, because a destination is
/// picked by its option number and the id would be noise. But the surviving
/// expert grammars address areas by id — `a 4 9 12`, `b 7`, `r 4:1 9`,
/// `m 8 9` — so a row whose areas are those grammars' arguments has to print
/// them, or §0.2's "aliases survive" turns into "aliases survive if you can
/// find the id on the board yourself". Placements and retreats name ids;
/// destinations and near-miss rows do not.
fn area_name_with_id(board: &BoardModel, area: AreaId) -> String {
    format!("{} (A{:02})", area_name(board, area), area.0)
}

// ---------------------------------------------------------------------------
// Option rows
// ---------------------------------------------------------------------------

/// Which slice of the option list one page shows, and how many pages there are.
///
/// **The page is a budget of ROWS ([`OPTION_ROWS`]), not of options, and that
/// is what bounds the zone's height.** How many options fit in those rows
/// depends on how many columns they pack into, which depends on how wide they
/// are — so the page size is not a constant and the total is not a division.
/// [`page_bounds`] walks the list instead, growing a page while it still fits
/// the budget. A screen of one-line takes packs three columns deep; a screen of
/// long Church payment lines packs two, or one, and still costs the same twelve
/// rows.
#[derive(Debug, PartialEq, Eq)]
struct Page {
    start: usize,
    end: usize,
    number: usize,
    total: usize,
}

impl Page {
    /// The `page`-th page of an already-rendered option list.
    ///
    /// A `page` past the end clamps to the last one — `more` wraps to 0 in the
    /// caller, and a clamped page is a screen the player can act on where an
    /// empty one is not.
    fn of(widths: &[usize], budget: usize, page: usize) -> Page {
        let bounds = page_bounds(widths, budget);
        let total = bounds.len();
        let number = page.min(total - 1);
        let (start, end) = bounds[number];
        Page { start, end, number, total }
    }
}

/// The pager footer, for every paged surface in the driver.
///
/// **One formatter rather than two literals kept in step** (T5 review S4). The
/// decision zone's option list and `repl`'s route sub-prompt page identically,
/// so a player who learns one screen's words meets the same ones on the next —
/// and a wording change here cannot land on one surface only. The two literal
/// copies this replaced had already been reconciled once by hand.
///
/// `remaining` is how many rows the current page does NOT show. Zero means the
/// last page, and its footer words the WRAP rather than counting a remainder it
/// does not have: `more` still works there, and `(0 more — …)` reads as a bug on
/// the one page every list ends on.
///
/// `page_number` is 0-based and `total_pages` is the count callers already hold.
/// Callers print a footer only when a list actually pages (`total_pages > 1`),
/// which is why the wrap arithmetic can assume a nonzero total; `max(1)` keeps
/// it total anyway rather than dividing by zero on a caller that forgets.
pub fn pager_footer(remaining: usize, page_number: usize, total_pages: usize) -> String {
    if remaining == 0 {
        return format!(
            "(last page — 'more' wraps to page 1 of {total_pages}, 'list' back to the first)"
        );
    }
    let next = (page_number + 1) % total_pages.max(1) + 1;
    format!("({remaining} more — 'more' for page {next} of {total_pages}, 'list' back to the first)")
}

/// Page boundaries over the option widths, each page as full as
/// [`OPTION_ROWS`] allows.
///
/// The walk is a plain forward scan because the cost of a prefix is monotone:
/// adding an option can only make the page's widest option wider (fewer
/// columns) and its count larger (more rows), so once a prefix stops fitting,
/// no longer one will. Every page takes at least ONE option however wide it is
/// — a single option that cannot fit the budget still has to be shown, and it
/// costs one row.
fn page_bounds(widths: &[usize], budget: usize) -> Vec<(usize, usize)> {
    let mut bounds = Vec::new();
    let mut start = 0usize;
    while start < widths.len() {
        let mut end = start + 1;
        while end < widths.len() {
            let count = end + 1 - start;
            let widest = widths[start..=end].iter().copied().max().unwrap_or(0);
            if count.div_ceil(columns_for(count, widest, budget)) > OPTION_ROWS {
                break;
            }
            end += 1;
        }
        bounds.push((start, end));
        start = end;
    }
    if bounds.is_empty() {
        bounds.push((0, 0));
    }
    bounds
}

/// Every option's rendered row and its PLAIN width, in list order.
///
/// Rendered once and measured once: the pager needs the widths to decide where
/// a page ends, and the renderer needs the same rows to draw it, so computing
/// them apart would be two chances to disagree about where page two starts.
fn option_texts(
    rows: &[Row<'_>],
    board: &BoardModel,
    colors: &[PlayerColor],
) -> (Vec<OptionRow>, Vec<usize>) {
    let texts: Vec<OptionRow> =
        rows.iter().enumerate().map(|(i, row)| option_text(i, row, board, colors)).collect();
    let widths = texts.iter().map(OptionRow::width).collect();
    (texts, widths)
}

/// How many pages a row list makes — the pager's wraparound, walked by exactly
/// the same function the footer's "page N of M" reads.
pub fn page_count(rows: &[Row<'_>], board: &BoardModel, colors: &[PlayerColor]) -> usize {
    // The colour table is a WIDTH input, not decoration: a fee recipient reads
    // `2 to Yellow` where it used to read `2 to seat 3`, so a pager measuring
    // rows without it would be measuring a different list from the one the
    // renderer draws — which is exactly the disagreement about where page two
    // starts that this entry point exists to prevent.
    page_bounds(&option_texts(rows, board, colors).1, column_budget(board)).len()
}

/// The numbered options of one page, packed into as many columns as fit — or,
/// when they will not pack at all, one per line and WRAPPED to the budget.
fn option_block(all: &[OptionRow], plain_all: &[usize], page: &Page, budget: usize) -> String {
    let texts = &all[page.start..page.end];
    let plain = &plain_all[page.start..page.end];
    let widest = plain.iter().copied().max().unwrap_or(0);
    let columns = columns_for(texts.len(), widest, budget);

    // ONE option per line is the only shape that can overrun the frame:
    // [`columns_for`] approves a multi-column fit against the budget BEFORE it
    // returns one, and per-column sizing below can only narrow that. So this is
    // the only branch that wraps, and the packed branch needs no width guard.
    if columns == 1 {
        return texts
            .iter()
            .flat_map(|row| row.lines(budget))
            .map(|line| format!("{line}\n"))
            .collect();
    }

    // Column-MAJOR: the numbers run down the first column and continue at the
    // top of the second, so a page reads in one downward sweep. Reading order
    // matters more than packing here — a row-major fill puts `[1]` to the right
    // of `[0]` and turns "find option 7" into arithmetic.
    let per_column = texts.len().div_ceil(columns);
    // Each column is padded to ITS OWN widest option, not to the page's. One
    // long entry among short ones — a retreat family's line beside four
    // movement rows — otherwise pushes the second column most of the way across
    // the screen and leaves the list reading as two unrelated blocks. Sizing
    // per column can only make the row narrower than the fit
    // [`columns_for`] approved on the page's widest, so it cannot overrun.
    let column_width = |c: usize| -> usize {
        (0..per_column)
            .filter_map(|r| index_at(r, c, per_column, texts.len()))
            .map(|i| plain[i])
            .max()
            .unwrap_or(0)
    };
    let widths: Vec<usize> = (0..columns).map(column_width).collect();

    let mut out = String::new();
    for r in 0..per_column {
        let mut line = String::new();
        for (c, column) in widths.iter().enumerate() {
            let Some(i) = index_at(r, c, per_column, texts.len()) else {
                continue;
            };
            if c > 0 {
                line.push_str(&" ".repeat(COLUMN_GAP));
            }
            line.push_str(&texts[i].painted());
            // Pad every cell but the last on its line, so the next column
            // starts where the others do. Measured on the PLAIN text: escapes
            // carry bytes and no display width (style.rs's alignment rule).
            let is_last = index_at(r, c + 1, per_column, texts.len()).is_none() || c + 1 == columns;
            if !is_last {
                line.push_str(&" ".repeat(column.saturating_sub(plain[i])));
            }
        }
        out.push_str(OPTION_INDENT);
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// The option index at column-major position `(r, c)`, or `None` past the end.
fn index_at(r: usize, c: usize, per_column: usize, len: usize) -> Option<usize> {
    let i = c * per_column + r;
    (i < len).then_some(i)
}

/// How many columns `count` options of `widest` columns each pack into,
/// inside a budget of `budget` columns.
///
/// A column count that would put a row past the budget is not offered, so the
/// packing can only ever leave the zone NARROWER than the frame above it —
/// which is what makes this safe to do without measuring a terminal (std
/// exposes none; see `screen`'s own note).
fn columns_for(count: usize, widest: usize, budget: usize) -> usize {
    // A list that already fits the budget stays ONE per line, whatever its
    // options measure. Columns are relief from a list too tall for the frame,
    // and applying them to a three-option Placement decision would put "Option
    // A" and "Option B" side by side — which is not what the mocks draw, and
    // makes a short list read as a table for no gain.
    if count <= OPTION_ROWS {
        return 1;
    }
    let mut columns = 1;
    while columns < MAX_COLUMNS {
        let next = columns + 1;
        // One more column only helps if there are options left to put in it.
        if count.div_ceil(next) == count.div_ceil(columns) {
            break;
        }
        let width = OPTION_INDENT.chars().count() + next * widest + (next - 1) * COLUMN_GAP;
        if width > budget {
            break;
        }
        columns = next;
    }
    columns
}

/// The column budget a zone under THIS board has: the width its composed frame
/// opens at (`screen::frame_width`), not the design floor.
///
/// The distinction is not cosmetic. A 3-player frame is 175 columns wide — the
/// board is narrower than the floor and the dashboard stacks below it — so a
/// zone packed to 200 would print option rows 25 columns past the widest thing
/// above them, and the frame's own width pin would (correctly) call that a
/// defect. The player count is the one layout fact the board model carries, and
/// this is the one place the zone needs it.
fn column_budget(board: &BoardModel) -> usize {
    crate::screen::frame_width(usize::from(board.player_count))
}

/// One character of an option row with the tone it is painted in — `None` for
/// text that takes no tint.
///
/// **Per CHARACTER, because the row WRAPS.** A row too wide for the frame is
/// split at whatever column the budget falls on, and splitting an already-
/// painted string there can cut an escape run in half: the first line then
/// carries an opening SGR with no reset and bleeds its tone across everything
/// drawn after it. Carrying the tone alongside each character instead means
/// every wrapped line paints its own runs from scratch, which makes that
/// unrepresentable rather than merely avoided.
type Cell = (Option<Tone>, char);

/// One numbered option: `[3] Move to CUSTOMS ◈ — 1 step · free`, held as the
/// cells it is painted from so it can be drawn on one line or several.
struct OptionRow {
    /// The `[3]` number cell, PLAIN. Painted [`Tone::Strong`] at emission, and
    /// its width plus the following space is the hanging indent a wrapped row's
    /// continuation lines sit at — so a tail reads as more of the option above
    /// it rather than as an unnumbered new one.
    number: String,
    /// Everything after the number cell and its space.
    body: Vec<Cell>,
}

impl OptionRow {
    /// The row's PLAIN width, [`OPTION_INDENT`] excluded (every row pays that
    /// equally, and [`columns_for`] adds it once).
    fn width(&self) -> usize {
        self.number.chars().count() + 1 + self.body.len()
    }

    /// Every cell of the row, number included — the one form both the one-line
    /// and the wrapped renderings are painted from.
    fn cells(&self) -> Vec<Cell> {
        let mut cells: Vec<Cell> =
            self.number.chars().map(|c| (Some(Tone::Strong), c)).collect();
        cells.push((None, ' '));
        cells.extend_from_slice(&self.body);
        cells
    }

    /// The row on ONE line, painted and un-indented: the packed-columns cell.
    fn painted(&self) -> String {
        paint_cells(&self.cells())
    }

    /// The row as one or more painted, INDENTED lines, none wider than
    /// `budget`.
    fn lines(&self, budget: usize) -> Vec<String> {
        let indent = OPTION_INDENT.chars().count();
        let hang = indent + self.number.chars().count() + 1;
        let cells = self.cells();
        wrap_cells(&cells, budget.saturating_sub(indent), budget.saturating_sub(hang))
            .into_iter()
            .enumerate()
            .map(|(i, piece)| {
                let lead = if i == 0 { OPTION_INDENT.to_string() } else { " ".repeat(hang) };
                format!("{lead}{}", paint_cells(piece))
            })
            .collect()
    }
}

/// Paint a run of cells, one `paint` call per maximal same-tone run.
///
/// Runs rather than characters because `paint` wraps whatever it is given in a
/// full SGR pair: per character that would be an escape pair per letter, and
/// the frame composer measures those bytes on every line it joins.
fn paint_cells(cells: &[Cell]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < cells.len() {
        let tone = cells[i].0;
        let mut run = String::new();
        while i < cells.len() && cells[i].0 == tone {
            run.push(cells[i].1);
            i += 1;
        }
        match tone {
            Some(t) => out.push_str(&paint(t, &run)),
            None => out.push_str(&run),
        }
    }
    out
}

/// Split `cells` into lines of at most `first_room` then `rest_room` columns,
/// breaking at spaces where there is one and mid-token where there is not.
///
/// **The mid-token break is not a fallback nobody hits.** An option row states
/// its own fee recipients and its own recovered areas, and a 22-pawn retreat
/// measures over 300 columns as one comma-run with no space wide enough to
/// break at inside the budget. Clipping it is the wrong trade — the tail of an
/// option row carries the price, which is exactly what the player is choosing
/// between — so the line breaks wherever it must and the words all survive.
fn wrap_cells(cells: &[Cell], first_room: usize, rest_room: usize) -> Vec<&[Cell]> {
    let mut lines: Vec<&[Cell]> = Vec::new();
    let mut pos = 0usize;
    while pos < cells.len() {
        // At least one column, always: a budget narrower than the indent would
        // otherwise loop forever rather than draw something ugly.
        let room = if lines.is_empty() { first_room } else { rest_room }.max(1);
        if cells.len() - pos <= room {
            lines.push(&cells[pos..]);
            break;
        }
        let limit = pos + room;
        match cells[pos..=limit].iter().rposition(|(_, c)| *c == ' ') {
            Some(at) if at > 0 => {
                lines.push(&cells[pos..pos + at]);
                pos += at + 1;
                // A break eats its own space, and any run of them: a
                // continuation line that opened on whitespace would sit one
                // column right of the hanging indent for no reason.
                while pos < cells.len() && cells[pos].1 == ' ' {
                    pos += 1;
                }
            }
            _ => {
                lines.push(&cells[pos..limit]);
                pos = limit;
            }
        }
    }
    lines
}

/// One numbered option, worded from the model where this module re-words and
/// from `describe`'s own line where it does not (module header).
fn option_text(
    index: usize,
    row: &Row<'_>,
    board: &BoardModel,
    colors: &[PlayerColor],
) -> OptionRow {
    let mut body: Vec<Cell> = Vec::new();
    let mut plain = |s: &str| body.extend(s.chars().map(|c| (None, c)));
    match row {
        Row::Family { words } => plain(words),
        Row::Move { mv, fee, words } => match move_row(mv, *fee, board, colors) {
            Some(worded) => {
                for (tone, text) in &worded.segments {
                    body.extend(text.chars().map(|c| (*tone, c)));
                }
            }
            None => plain(words),
        },
    };
    OptionRow { number: format!("[{}]", menu_number(index)), body }
}

/// A re-worded move row as TINTED SEGMENTS, in reading order.
///
/// **Segments rather than the old prefix/marked pair, because T11 is the
/// deliberate palette pass the T4 review deferred** ([`WordedMove::with_tail`]
/// records the deferral). Two fields could express "everything before the mark
/// is plain, everything from the mark on is green", which is not what the
/// approved `main-frame-mid.ans` draws: it BOLDS the area name, tints the `◈`
/// alone, and leaves the ` — 1 step · free` tail plain. Three treatments need
/// three segments.
struct WordedMove {
    segments: Vec<(Option<Tone>, String)>,
}

impl WordedMove {
    /// A row with no destination mark — everything it says takes no tint.
    ///
    /// Most re-worded rows are this shape (the retreat and the two placements),
    /// which is why it is a constructor.
    fn plain(text: String) -> WordedMove {
        WordedMove { segments: vec![(None, text)] }
    }
}

/// The five move kinds the mocks draw, worded from the MODEL — destination
/// name from the board, `◈` from its reachable mark, step count from the
/// path, fee from the priced [`FeeModel`]. `None` for every other kind, which
/// leaves [`option_text`] printing `describe`'s own words (module header).
fn move_row(
    mv: &Move,
    fee: Option<&FeeModel>,
    board: &BoardModel,
    colors: &[PlayerColor],
) -> Option<WordedMove> {
    match mv {
        Move::MovePresident { path, destination } => {
            // A TELEPORT states its price in the word and stops: an empty path
            // means `route_fee_vector` had nothing to charge for, so the fee
            // clause could only ever have said "free" a second time
            // (`— free teleport · free`). A priced one — which the model above
            // cannot produce, and which this does not assume away — still says
            // what it costs, once.
            let tail = match (path.is_empty(), fee) {
                (true, Some(f)) if f.total > 0 => {
                    format!("teleport {} {}", glyph::SEPARATOR, fee_words(fee, colors))
                }
                (true, _) => "free teleport".to_string(),
                (false, _) => format!(
                    "{} step{} {} {}",
                    path.len(),
                    if path.len() == 1 { "" } else { "s" },
                    glyph::SEPARATOR,
                    fee_words(fee, colors),
                ),
            };
            Some(destination_row("Move to", *destination, board, &tail))
        }
        Move::PlacePresident { destination } => Some(destination_row(
            "Place your President in",
            *destination,
            board,
            &fee_words(fee, colors),
        )),
        Move::ReturnPresidentToHand { recover } => {
            let recovered: u32 = recover.values().map(|&c| u32::from(c)).sum();
            let tail = if recovered == 0 {
                "recovering no assistants".to_string()
            } else {
                let where_from: Vec<String> = recover
                    .iter()
                    .map(|(area, n)| format!("{n} from {}", area_name_with_id(board, *area)))
                    .collect();
                format!("recovering {recovered} ({})", where_from.join(", "))
            };
            Some(WordedMove::plain(format!(
                "Return the President to your hand {} {tail}",
                glyph::SECTION_DASH
            )))
        }
        Move::PlaceThreeAreas { placements } if !placements.is_empty() => {
            let areas: Vec<String> =
                placements.iter().map(|a| area_name_with_id(board, *a)).collect();
            Some(WordedMove::plain(format!(
                "Place 1 assistant in {}{}",
                areas.join(", "),
                fee_tail(fee, colors),
            )))
        }
        Move::PlaceTwoSameArea { area, adjacent_bonus } => {
            let bonus = match adjacent_bonus {
                Some(adj) => format!(" and 1 in {} (Telegram)", area_name_with_id(board, *adj)),
                None => String::new(),
            };
            Some(WordedMove::plain(format!(
                "Place 2 assistants in {}{bonus}{}",
                area_name_with_id(board, *area),
                fee_tail(fee, colors),
            )))
        }
        _ => None,
    }
}

/// `Move to CUSTOMS ◈ — 1 step · free`: a destination row, with the `◈` drawn
/// iff the BOARD's model marks that tile reachable.
///
/// The mark is read off the board rather than assumed from the move being a
/// President move, so the row and the tile can never disagree about what the
/// engine offered: one boolean, one source.
fn destination_row(verb: &str, area: AreaId, board: &BoardModel, tail: &str) -> WordedMove {
    let marked = board.tiles.iter().any(|t| t.area == area && t.reachable);
    let mut segments = vec![
        (None, format!("{verb} ")),
        // The area NAME is what a player scans the list for, so it is the one
        // thing on the row that is bold — the approved `main-frame-mid.ans`'s
        // own treatment, adopted at T11's palette pass.
        (Some(Tone::Strong), area_name(board, area).to_string()),
        (None, " ".to_string()),
    ];
    if marked {
        segments.push((Some(Tone::Reachable), format!("{} ", glyph::REACHABLE)));
    }
    segments.push((None, format!("{} {tail}", glyph::SECTION_DASH)));
    WordedMove { segments }
}

/// `free` / `pay ¥2 (2 to Blue)` / `pay ¥3 (2 to Blue, 1 canal toll)`.
///
/// Player recipients first in seat order, the bank's canal toll last — the
/// order `describe` prints and the order the money actually moves in.
///
/// **Recipients are named by COLOUR since T11, and they used to be named by
/// seat.** `describe::placement_fee_note` — the other surface that prints this
/// exact fact — has always read `2 to Blue`, so a player comparing a Movement
/// row against a Placement preview was reading one fee in two vocabularies. The
/// rule the driver now holds everywhere: a PLAYER is named by their colour (it
/// is what is on the table); a SEAT number is used only where the thing being
/// named is a seat rather than a player — the `reveal <seat>` grammar, its
/// menu, the panel annotation that is that grammar's key, and the wizard's
/// per-seat setup questions. A fee recipient is a player.
///
/// [`FeeModel`] still carries no colour table, deliberately — it is a MODEL —
/// so the table arrives as a parameter and a seat the caller cannot colour
/// falls back to its number rather than losing its identity.
fn fee_words(fee: Option<&FeeModel>, colors: &[PlayerColor]) -> String {
    let Some(fee) = fee else { return "free".to_string() };
    if fee.total == 0 {
        return "free".to_string();
    }
    let mut parts: Vec<String> = fee
        .to_players
        .iter()
        .map(|(id, amount)| match color_of(colors, *id) {
            Some(c) => format!("{amount} to {}", color_word(c)),
            None => format!("{amount} to seat {}", id.0),
        })
        .collect();
    if fee.canal_tolls > 0 {
        parts.push(format!(
            "{} canal toll{}",
            fee.canal_tolls,
            if fee.canal_tolls == 1 { "" } else { "s" },
        ));
    }
    format!("pay {}{} ({})", glyph::YEN, fee.total, parts.join(", "))
}

/// The ` — pay ¥2 (…)` clause a placement row carries, or nothing at all when
/// the placement is free. Unlike a movement row, which always states its price
/// (`free` included, because a player choosing between routes is comparing
/// prices), a placement into empty areas has no price to compare and the words
/// would be noise on every row of the commonest decision in the game.
fn fee_tail(fee: Option<&FeeModel>, colors: &[PlayerColor]) -> String {
    match fee {
        Some(fee) if fee.total > 0 => {
            format!(" {} {}", glyph::SECTION_DASH, fee_words(Some(fee), colors))
        }
        _ => String::new(),
    }
}

// ---------------------------------------------------------------------------
// The standing hint
// ---------------------------------------------------------------------------

/// The mocks' standing hint.
///
/// **The recall clause is CONDITIONAL, and that is the whole of what is left of
/// the old deviation.** The mock's hint names `↑ recalls what you typed`; until
/// T9 shipped the line editor an arrow key produced an escape sequence, so the
/// clause was suppressed rather than printed and false. T9 made it true on a
/// terminal and left it false everywhere else — a piped run has no editor and
/// no ↑ — so it is printed exactly where it holds ([`ZoneContext::recall`]),
/// which keeps the piped protocol byte-identical to what it was and stops a
/// real terminal under-promising a key the guide sells.
///
/// **The mock's OTHER clause is now true and printed.** T4 shipped this line
/// reading `'view' or '?' any surface`, because `?` was then an alias for the
/// view menu; T5 gave `?` the card index the mock always meant it to open, so
/// the hint splits into the mock's own two clauses — `'view' any surface` and
/// `'?' the card index` — which are two different screens and now say so.
///
/// `back` IS true: it landed with this zone as an alias of `undo`, precisely so
/// the standing hint the mocks draw could be printed without qualification.
fn hint_line(rows: &[Row<'_>], ctx: &ZoneContext<'_>, players: u8) -> String {
    // The forced-single-option ceremony, preserved exactly: what bare Enter
    // DOES depends on the entry — a move commits, a collapsed family opens its
    // picker — and saying "apply" for the second over-promised (F4).
    let lead = if rows.len() == 1 {
        let verb = match rows[0] {
            Row::Move { .. } => "apply",
            Row::Family { .. } => "open",
        };
        vec![
            format!("only option — press Enter to {verb}"),
            "nothing is final until you apply it".to_string(),
        ]
    } else {
        vec!["pick a number".to_string()]
    };
    hint_with_lead(lead, ctx, players)
}

/// The standing hint on its own, for a screen that shows no options: a
/// read-only view, which reprints the hint but not the list above it.
///
/// The SAME function the zone's own hint goes through, so the two can never
/// teach two vocabularies — which is the whole reason it is exposed rather
/// than copied.
pub fn standing_hint(ctx: &ZoneContext<'_>, players: u8) -> String {
    hint_with_lead(vec!["pick a number".to_string()], ctx, players)
}

fn hint_with_lead(lead: Vec<String>, ctx: &ZoneContext<'_>, players: u8) -> String {
    if ctx.game_over {
        return format!(
            "{}\n",
            paint(
                Tone::Chrome,
                "('score' for the final breakdown · 'log' for the game · 'quit' to leave)",
            ),
        );
    }
    let mut parts = lead;
    if !ctx.expert.is_empty() {
        parts.push(ctx.expert.to_string());
    }
    // The three standing clauses spec §0.8 requires on every screen: what takes
    // an entry back, that nothing commits until it is applied (carried by the
    // lead when there is a single option, and by `back`'s wording otherwise),
    // and how to reach the other surfaces.
    if ctx.recall {
        parts.push(format!("{} recalls what you typed", glyph::HISTORY_KEY));
    }
    parts.push("'back' undoes the last applied entry".to_string());
    parts.push("'view' any surface".to_string());
    parts.push("'?' the card index".to_string());
    parts.push("'help' for commands".to_string());
    wrap_hint(&parts, crate::screen::frame_width(usize::from(players)))
}

// ---------------------------------------------------------------------------
// The reveal zone (plan T8, spec §0.2/§0.6)
// ---------------------------------------------------------------------------

/// The prompt a live capture shows instead of the decision zone: what refilled,
/// which card is being asked for, what this group already holds, and the ways
/// out.
///
/// **The same four-part rhythm as [`decision_zone`]**, deliberately — a banner
/// line, a blank, the one-sentence statement of what is being asked, and the
/// standing hint above the caret. A capture is the one moment the player is
/// looking away from the screen and at the table, so the shape they read has to
/// be the shape they already know; the pre-redesign prompt was a single wrapped
/// sentence with the slot number, the run position, the grammar and the escape
/// all packed into it (*"a lot of abbreviations"*, the brief's first finding).
///
/// It carries no options and no phase chain: the frame underneath it is
/// unchanged and still on screen, and nothing here is a decision — the engine
/// has already ruled the parked move legal.
pub fn reveal_zone(model: &crate::screen_model::RevealModel, players: u8) -> String {
    let mut out = String::new();
    out.push_str(&paint(Tone::Strong, "LIVE TRACKING"));
    out.push_str(&paint(Tone::Chrome, &format!(" {} ", glyph::SEPARATOR)));
    out.push_str(&paint(Tone::Chrome, &describe::reveal_event(model.target)));
    out.push_str("\n\n");
    out.push_str(&paint(Tone::Strong, "YOU ARE RECORDING"));
    // The target is a NOUN PHRASE ("Port A slot 6 (reveal 1 of 1)", "the card
    // that refilled Laboratory A"), which is what the index header consumes it
    // as — so the question hangs off it rather than trying to read through it.
    // The alternative frame, `which <registry> is <target>?`, produces "which
    // technology is the card that refilled Laboratory A?".
    out.push_str(&format!(
        " {} {} {} which {} is it?\n",
        glyph::SEPARATOR,
        describe::recording_target(model.target),
        glyph::SECTION_DASH,
        describe::registry_name_singular(model.kind()),
    ));
    if !model.collected.is_empty() {
        let tags: Vec<String> =
            model.collected.iter().copied().map(crate::card_prompt::card_tag).collect();
        out.push_str(OPTION_INDENT);
        out.push_str(&paint(
            Tone::Chrome,
            &format!(
                "already recorded in this reveal {} {}",
                glyph::SECTION_DASH,
                tags.join(&format!(" {} ", glyph::SEPARATOR)),
            ),
        ));
        out.push('\n');
    }
    out.push_str(&reveal_hint(model, players));
    out.push_str(&paint(Tone::Prompt, "> "));
    out.push('\n');
    out
}

/// The reveal prompt's standing hint.
///
/// **Its own clause set rather than [`standing_hint`]'s, because that one would
/// be false here.** `back` undoes the last APPLIED entry and nothing has been
/// applied; `view` opens the game-info menu, which the capture branch of
/// `Repl::step` never reaches. What IS reachable mid-capture is exactly what
/// this names: the card grammar, the index, the two full-screen views the
/// overlay interception lets through, and the two escapes.
fn reveal_hint(model: &crate::screen_model::RevealModel, players: u8) -> String {
    use crate::screen_model::RecordingTarget;
    // What `cancel` costs, which is not the same fact in the two shapes: a bound
    // reveal has a MOVE parked behind it and dropping the reveal drops the move,
    // while a standalone starting-order reveal has nothing parked at all. Said
    // exhaustively, never with a `_` arm, on this module's standing rule.
    let cancel = match model.target {
        RecordingTarget::StartingOrder { .. } => "'cancel' backs out — nothing is recorded",
        RecordingTarget::PortRefill { .. }
        | RecordingTarget::TechRefill { .. }
        | RecordingTarget::PortSetup { .. }
        | RecordingTarget::DealSetup { .. }
        | RecordingTarget::LabSetup { .. }
        | RecordingTarget::SiteSetup { .. }
        | RecordingTarget::AchievementSetup { .. } => {
            "'cancel' drops the move — nothing is applied or recorded"
        }
    };
    let parts = vec![
        crate::card_prompt::entry_hint(model.kind(), false).to_string(),
        cancel.to_string(),
        // **The whole roster, not two of it** (T11). This clause named
        // `'sites' or 'tracks'` because those were the only two full-screen
        // views when it was written; `ports`, `labs`, `hand` and `log` became
        // views too and are reachable from here by exactly the same mechanism —
        // a whole-line match on a surface's own name, intercepted ahead of the
        // capture — so the hint was under-selling four screens.
        //
        // **It names the WORDS and not `view`, deliberately.** The six surface
        // names are intercepted by `Repl::step` AHEAD of the reveal collector,
        // exactly as `?` is; the `view` menu is dispatched in the command match
        // BELOW it, so the collector reaches every `view` line first and reads
        // it as a bad card. That is a fact about dispatch ORDER — the menu is a
        // full-screen overlay like the surfaces it lists, not a sub-prompt, and
        // calling it one was the stale half of this note. Advertising it would
        // have been the shorter clause and a false one, which
        // `the_reveal_hint_never_offers_the_one_word_that_does_not_work_here`
        // is the pin for.
        "'tracks', 'sites', 'ports', 'labs', 'hand' or 'log' to look around".to_string(),
    ];
    wrap_hint(&parts, crate::screen::frame_width(usize::from(players)))
}

/// The hint's clauses packed into as few lines as fit `budget` columns, each
/// continued line signalled by the trailing separator on the one before it.
///
/// **It wraps rather than being written short enough to fit**, because "short
/// enough" is not one length: the frame is 198 columns at 2 players and 175 at
/// 3, and the longest hint — a forced single option, in a phase with an expert
/// grammar — is 187. Trimming the clause set until the widest case cleared the
/// NARROWEST frame would have cost the spec-required wording everywhere to fix
/// a case that only arises at two of the three counts. Wrapping costs one row,
/// in that case only, and cannot be outgrown by a future clause.
fn wrap_hint(parts: &[String], budget: usize) -> String {
    let sep = format!(" {} ", glyph::SEPARATOR);
    // What a line still has to spend after its last clause: one column to
    // close the parenthesis on the final line, two for the hanging " ·" that
    // marks a continuation on any other. Reserved BEFORE the fit test, not
    // added after it — the version that added it after emitted lines exactly
    // two columns over the budget, which is the whole class of defect this
    // wrap exists to prevent.
    let tail_cost = |is_last_part: bool| if is_last_part { 1 } else { 2 };
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::from("(");
    let mut on_line = 0usize;
    for (i, part) in parts.iter().enumerate() {
        let is_last = i + 1 == parts.len();
        let candidate = current.chars().count()
            + if on_line == 0 { 0 } else { sep.chars().count() }
            + part.chars().count()
            + tail_cost(is_last);
        if on_line > 0 && candidate > budget {
            // Close the line with a hanging separator, so a reader sees the
            // clause list continues rather than ending mid-thought.
            current.push_str(sep.trim_end());
            lines.push(current);
            current = String::from(" ");
            on_line = 0;
        } else if on_line > 0 {
            current.push_str(&sep);
        }
        current.push_str(part);
        on_line += 1;
    }
    current.push(')');
    lines.push(current);
    lines.iter().map(|l| format!("{}\n", paint(Tone::Chrome, l))).collect()
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::visible_width;
    use crate::screen_model;
    use yokohama_core::*;

    fn state_np(n: u8) -> (GameState, Rules) {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(n), &rules, 1).unwrap();
        let state =
            finalize_setup(pending, &vec![StartingOrderChoice::Keep(0); n as usize]).unwrap();
        (state, rules)
    }

    fn colors_np(state: &GameState) -> Vec<PlayerColor> {
        state.players.iter().map(|p| p.color).collect()
    }

    /// **The pager measures the rows WITH their colour table** — the claim
    /// `page_count`'s own comment makes, checked rather than stated (T11 review
    /// F7).
    ///
    /// The table is a width input: a fee recipient reads `1 to Blue` where a
    /// caller without it renders `1 to seat 1`, two columns wider per
    /// recipient. Two columns is enough to cost an option row a packing column,
    /// and a packing column is worth twelve options a page — so a `page_count`
    /// called with `&[]` while `decision_zone` renders with the real table is
    /// not an approximation, it is a different list.
    ///
    /// Held as a PARTITION over rendered output rather than as an arithmetic
    /// identity: walking `0..page_count` and reading the option numbers off the
    /// screens must yield every row exactly once. That catches a gap and a
    /// repeat alike, which matters because the two failures look nothing like
    /// each other — an under-count leaves rows unreachable, an over-count
    /// clamps and redraws the last page under a footer promising a new one.
    ///
    /// The fixture is MEASURED, not guessed: at 3 players (frame 175) a row
    /// paying both opponents plus one canal toll renders 83 columns with the
    /// table and 85 without, which straddles the two-column boundary
    /// (`3 + 2·85 + 2 > 175`), so 37 rows page 2 ways with it and 4 ways
    /// without. Both figures are asserted, so a layout change that dissolves
    /// the discrimination fails here instead of quietly making the test vacuous.
    #[test]
    fn the_pager_measures_its_rows_with_the_colour_table_it_is_handed() {
        let (state, rules) = state_np(3);
        let board = screen_model::build_board(&state, &rules, &[]);
        let d = screen_model::build_decision(&state, &rules, &[]);
        let colors = colors_np(&state);
        let fee = FeeModel {
            total: 3,
            to_players: vec![(PlayerId::new(1), 1), (PlayerId::new(2), 1)],
            canal_tolls: 1,
        };
        let dest = AreaId::new(0);
        let walk = Move::MovePresident { path: vec![dest], destination: dest };
        let rows: Vec<Row> =
            (0..37).map(|_| Row::Move { mv: &walk, fee: Some(&fee), words: "fallback" }).collect();

        // The measurement the fixture rests on, pinned so it cannot drift into
        // a pair of widths that no longer straddles anything.
        assert_eq!(option_text(0, &rows[0], &board, &colors).width(), 83);
        assert_eq!(option_text(0, &rows[0], &board, &[]).width(), 85);

        let with_table = page_count(&rows, &board, &colors);
        let without_table = page_count(&rows, &board, &[]);
        assert_eq!((with_table, without_table), (2, 4), "the fixture must discriminate");

        // (a) The pages the renderer draws, under the count the renderer's own
        // colour table produces, partition the list. SORTED, because options
        // fill down their columns rather than across, so the reading order of a
        // packed page is column-major and says nothing about coverage.
        let mut drawn = numbers_over_pages(&d, &rows, &board, &colors, with_table);
        assert_eq!(drawn.len(), 37, "every row drawn exactly once: {drawn:?}");
        drawn.sort_unstable();
        assert_eq!(drawn, (1..=37).collect::<Vec<_>>(), "and every row drawn");

        // (b) The same walk under the count a colourless caller would compute
        // is NOT a partition — `Page::of` clamps past the end, so pages 3 and 4
        // redraw page 2 and the player is told there are four.
        let mis_paged = numbers_over_pages(&d, &rows, &board, &colors, without_table);
        assert!(
            mis_paged.len() > 37,
            "the over-count repeats the last page rather than dropping rows: {} numbers",
            mis_paged.len(),
        );
    }

    /// Every option number the zone draws over pages `0..pages`, in order —
    /// the readback the partition check above is built on. `[n]` is the option
    /// number cell and nothing else in the zone writes a bracketed integer,
    /// which the round-trip against a known list is itself evidence for.
    fn numbers_over_pages(
        d: &screen_model::DecisionModel,
        rows: &[Row<'_>],
        board: &BoardModel,
        colors: &[PlayerColor],
        pages: usize,
    ) -> Vec<usize> {
        let mut seen = Vec::new();
        for page in 0..pages {
            let zone = crate::style::strip_ansi(&decision_zone(
                d,
                rows,
                board,
                colors,
                &ZoneContext { page, ..ctx() },
            ));
            let mut rest = zone.as_str();
            while let Some(open) = rest.find('[') {
                rest = &rest[open + 1..];
                let Some(close) = rest.find(']') else { break };
                if let Ok(n) = rest[..close].trim().parse::<usize>() {
                    seen.push(n);
                }
            }
        }
        seen
    }

    fn ctx() -> ZoneContext<'static> {
        ZoneContext { page: 0, expert: "", game_over: false, recall: false }
    }

    /// **The reveal zone, at every target a live capture can reach** (T8): the
    /// banner sentence, the noun-phrase question, and a hint that fits the frame.
    ///
    /// The three targets are swept rather than spot-checked because the zone
    /// words itself from the target alone — a variant that reached it with an
    /// empty sentence or a hint whose clauses were false would render a screen
    /// that looks finished and says nothing.
    ///
    /// **At every player count, not just 2** (T8 review, NOTE-4): the hint WRAPS
    /// to `frame_width(count)` and the 3P frame is the narrowest of the three, so
    /// a 2P-only sweep measures the zone against the widest budget it will ever
    /// be given and would miss exactly the overrun the wrap exists to prevent.
    #[test]
    fn the_reveal_zone_words_every_live_target_and_fits_the_frame() {
        use crate::screen_model::{CardRef, RecordingTarget, RevealModel};
        for players in [2u8, 3, 4] {
            for target in [
                RecordingTarget::PortRefill { board_b: false, slot: 5, nth: 0, of: 2 },
                RecordingTarget::TechRefill { board_b: true },
                RecordingTarget::StartingOrder { seat: PlayerId::new(1) },
            ] {
                let model = RevealModel { target, collected: Vec::new() };
                let text = crate::style::strip_ansi(&reveal_zone(&model, players));
                assert!(text.starts_with("LIVE TRACKING"), "{players}P: {text}");
                assert!(
                    text.contains(&describe::reveal_event(target)),
                    "{players}P: the event sentence: {text}",
                );
                assert!(text.contains("YOU ARE RECORDING"), "{players}P: {text}");
                assert!(
                    text.contains(&describe::recording_target(target)),
                    "{players}P: the target: {text}",
                );
                assert!(
                    text.contains(describe::registry_name_singular(target.kind())),
                    "{players}P: and the registry the answer comes from: {text}",
                );
                // The decision zone's `back` clause would be FALSE here:
                // nothing has been applied to take back. (`view` is the other
                // false clause, and it has its own line-specific pin below —
                // this whole-text form could not catch it.)
                assert!(!text.contains("'back' undoes"), "{players}P: {text}");
                assert!(text.contains("'?' for the card index"), "{players}P: {text}");
                // Every line inside THIS count's frame, hint wrap included.
                for line in text.lines() {
                    assert!(
                        visible_width(line) <= crate::screen::frame_width(usize::from(players)),
                        "{players}P: {:?} is {} columns against a {}-column frame: {text}",
                        line,
                        visible_width(line),
                        crate::screen::frame_width(usize::from(players)),
                    );
                }
                assert!(
                    text.ends_with("> \n"),
                    "{players}P: the caret is the zone's last line: {text:?}",
                );
            }
        }

        // A bound reveal says what `cancel` costs; a standalone one says the
        // other true thing. Two shapes, and they are not interchangeable.
        let bound = RevealModel {
            target: RecordingTarget::PortRefill { board_b: false, slot: 0, nth: 1, of: 2 },
            collected: vec![CardRef::Order(OrderCardId(4))],
        };
        let text = crate::style::strip_ansi(&reveal_zone(&bound, 2));
        assert!(text.contains("'cancel' drops the move"), "{text}");
        assert!(text.contains("already recorded in this reveal"), "{text}");
        assert!(text.contains("O04"), "{text}");
        let standalone = RevealModel {
            target: RecordingTarget::StartingOrder { seat: PlayerId::new(1) },
            collected: Vec::new(),
        };
        let text = crate::style::strip_ansi(&reveal_zone(&standalone, 2));
        assert!(text.contains("'cancel' backs out"), "{text}");
        assert!(!text.contains("drops the move"), "nothing is parked behind this one: {text}");
    }

    /// **The reveal hint never offers `view`** — the one navigation word that
    /// genuinely does not work mid-capture, and the pin is on the HINT LINE
    /// rather than on the zone's whole text (T11 review F8).
    ///
    /// **The overlapping-text lesson, FOURTH recurrence** (T7 first named it;
    /// T8's F2 refusal pin and T10's per-seat `(hidden)` sweep were the second
    /// and third). The old guard read `!text.contains("'view' any surface")` —
    /// the DECISION zone's phrasing, which this zone was never going to write
    /// anyway. It was satisfied by a string from a different surface, so it was
    /// green before the clause existed and would have stayed green if the hint
    /// started offering `view` in its own words. What has to be false is the
    /// OFFER, and the offer lives on one line, so that line is what is read.
    ///
    /// Mutation: append `"'view' any surface"` to `reveal_hint`'s clause list
    /// and this must fail; under the old guard it passed.
    #[test]
    fn the_reveal_hint_never_offers_the_one_word_that_does_not_work_here() {
        use crate::screen_model::{RecordingTarget, RevealModel};
        for players in [2u8, 3, 4] {
            for target in [
                RecordingTarget::PortRefill { board_b: false, slot: 5, nth: 0, of: 2 },
                RecordingTarget::TechRefill { board_b: true },
                RecordingTarget::StartingOrder { seat: PlayerId::new(1) },
            ] {
                let model = RevealModel { target, collected: Vec::new() };
                let hint = crate::style::strip_ansi(&reveal_hint(&model, players));
                // Anti-vacuity: this IS the hint, and it does advertise the
                // surfaces that work — so a `view`-free result means the clause
                // set omits it, not that the fixture read the wrong line.
                assert!(hint.starts_with('('), "the hint block: {hint:?}");
                assert!(hint.contains("'sites'"), "it names the surfaces that DO work: {hint}");
                assert!(
                    !hint.contains("view"),
                    "{players}P: the hint offers `view`, which the reveal collector swallows \
                     before the command match ever sees it: {hint}",
                );
            }
        }
    }

    /// Every `PendingChoice` variant words a deciding line, and every one of
    /// them actually says something: the exhaustive match is a compile-time
    /// guarantee, and this is the run-time half — a variant wired to an empty
    /// or duplicated line would compile fine.
    #[test]
    fn every_pending_choice_words_its_own_deciding_line() {
        let heads = [
            PendingChoice::AnyGood,
            PendingChoice::WarehouseTake,
            PendingChoice::AssistantMove,
            PendingChoice::ChurchSpace { total_faith: 7 },
            PendingChoice::CustomsSpace { cost: 2 },
            PendingChoice::RecoveryLeave,
            PendingChoice::StationPlace,
        ];
        let mut seen: Vec<String> = Vec::new();
        for head in &heads {
            let line = deciding_line(&Deciding::Choice { head: head.clone(), queued: 1 });
            assert!(line.starts_with("YOU ARE DECIDING"), "{line}");
            let what = line.split_once(glyph::SEPARATOR).expect("a separator").1.trim();
            assert!(what.len() > 20, "{head:?} words its line too thinly: {line}");
            assert!(!seen.contains(&what.to_string()), "{head:?} reuses another line: {line}");
            seen.push(what.to_string());
        }
        assert_eq!(seen.len(), heads.len());
    }

    /// ...and every `TurnPhase` does too.
    #[test]
    fn every_phase_words_its_own_deciding_line() {
        let mut seen: Vec<String> = Vec::new();
        for phase in screen_model::PHASE_CHAIN {
            let line = deciding_line(&Deciding::Phase(phase));
            let what = line.split_once(glyph::SEPARATOR).expect("a separator").1.trim();
            assert!(what.len() > 20, "{phase:?} words its line too thinly: {line}");
            assert!(!seen.contains(&what.to_string()), "{phase:?} reuses another line: {line}");
            seen.push(what.to_string());
        }
        assert_eq!(seen.len(), screen_model::PHASE_CHAIN.len());
    }

    /// A queued choice says how many more are behind it — the fact the
    /// pre-redesign banner carried as "N choices pending" and the one thing a
    /// player mid-cascade needs to know.
    #[test]
    fn a_queued_cascade_says_how_many_follow() {
        let one = deciding_line(&Deciding::Choice { head: PendingChoice::AnyGood, queued: 1 });
        assert!(!one.contains("more to resolve"), "a lone choice has no tail: {one}");
        let three = deciding_line(&Deciding::Choice { head: PendingChoice::AnyGood, queued: 3 });
        assert!(three.contains("2 more to resolve"), "{three}");
    }

    /// The chain lights exactly one phase, names all nine, and leads with the
    /// turn and the seat.
    #[test]
    fn the_phase_chain_lights_the_current_phase_only() {
        let (mut state, rules) = state_np(2);
        state.turn_phase = TurnPhase::Movement;
        state.turn_number = 9;
        let d = screen_model::build_decision(&state, &rules, &[]);
        let line = phase_chain(&d, &colors_np(&state));
        assert!(line.starts_with("TURN 9 "), "{line}");
        assert!(line.contains("RED's turn"), "{line}");
        assert!(line.contains(" MOVEMENT "), "the current phase is lit and spaced: {line}");
        for phase in screen_model::PHASE_CHAIN {
            let name = describe::phase_chain_name(phase);
            assert!(line.contains(name) || line.contains(&name.to_uppercase()), "{name}: {line}");
        }
        assert_eq!(
            line.matches(glyph::CHAIN_STEP).count(),
            screen_model::PHASE_CHAIN.len() - 1,
            "one separator between each pair: {line}",
        );
    }

    /// The `◈` on an option row comes from the BOARD's mark, not from the move
    /// being a President move: mark the tile and the row shows it, clear the
    /// mark and the same move renders without one. That is what makes the row
    /// and the tile one claim rather than two.
    #[test]
    fn the_destination_mark_follows_the_boards_own_reachable_flag() {
        let (state, rules) = state_np(2);
        let dest = AreaId::new(3);
        let mv = Move::MovePresident { path: vec![dest], destination: dest };
        let mut board = screen_model::build_board(&state, &rules, &[]);
        let unmarked =
            option_text(0, &Row::Move { mv: &mv, fee: None, words: "fallback" }, &board, &colors_np(&state)).painted();
        assert!(!unmarked.contains(glyph::REACHABLE), "{unmarked}");

        for tile in &mut board.tiles {
            tile.reachable = tile.area == dest;
        }
        let marked =
            option_text(0, &Row::Move { mv: &mv, fee: None, words: "fallback" }, &board, &colors_np(&state)).painted();
        assert!(marked.contains(glyph::REACHABLE), "{marked}");
        assert!(marked.contains("1 step"), "the step count comes off the path: {marked}");
        assert!(marked.contains("free"), "an unpriced move is free: {marked}");
    }

    /// A priced move states the price in the mocks' own shape, and a free one
    /// says "free" rather than "pay ¥0".
    #[test]
    fn a_priced_move_states_its_fee_and_a_zero_fee_reads_as_free() {
        let paid = FeeModel {
            total: 3,
            to_players: vec![(PlayerId::new(1), 2)],
            canal_tolls: 1,
        };
        let words = fee_words(Some(&paid), &[]);
        assert_eq!(words, "pay ¥3 (2 to seat 1, 1 canal toll)", "{words}");
        assert_eq!(fee_words(None, &[]), "free");
        assert_eq!(
            fee_words(Some(&FeeModel { total: 0, to_players: Vec::new(), canal_tolls: 0 }), &[]),
            "free",
        );
    }

    /// A move kind the mocks do not draw keeps `describe`'s own words —
    /// including the engine-pinned previews this module deliberately does not
    /// re-author.
    #[test]
    fn an_unworded_move_kind_prints_describes_line_verbatim() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let mv = Move::ExecuteAreaAction { action: AreaAction::ProduceCopper };
        let words = describe::describe_move(&mv, &state, &rules);
        let row = option_text(4, &Row::Move { mv: &mv, fee: None, words: &words }, &board, &colors_np(&state)).painted();
        assert_eq!(row, format!("[5] {words}"), "{row}");
    }

    /// Every `NotOfferedReason` words a distinct row, and each names its area.
    #[test]
    fn every_not_offered_reason_words_its_own_row() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let area = AreaId::new(1);
        let named = area_name(&board, area);
        let mut seen: Vec<String> = Vec::new();
        for reason in [
            NotOfferedReason::StartedThereThisTurn,
            NotOfferedReason::CanalIsNeverAStop,
            NotOfferedReason::NoAssistantThere,
        ] {
            let row = not_offered_row(&NotOffered { area, reason }, &board);
            assert!(row.starts_with("(not offered: "), "{row}");
            assert!(row.contains(&named), "the row names its area ({named}): {row}");
            assert!(!seen.contains(&row), "{reason:?} reuses another row: {row}");
            seen.push(row);
        }
        // An area the board model does not hold still names something a player
        // can act on, rather than rendering as a blank.
        let orphan =
            not_offered_row(&NotOffered { area: AreaId::new(17), reason: NotOfferedReason::NoAssistantThere }, &board);
        assert!(orphan.contains("A17"), "{orphan}");
    }

    /// The forced-single-option ceremony is preserved EXACTLY, including the
    /// apply/open split a grouped family needs (F4).
    #[test]
    fn the_single_option_ceremony_keeps_its_apply_open_split() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let d = screen_model::build_decision(&state, &rules, &[]);
        let mv = Move::EndAdditionalPhase;

        let one_move = decision_zone(
            &d,
            &[Row::Move { mv: &mv, fee: None, words: "End the phase" }],
            &board,
            &colors_np(&state),
            &ctx(),
        );
        assert!(one_move.contains("only option — press Enter to apply"), "{one_move}");

        let one_family =
            decision_zone(&d, &[Row::Family { words: "Option A" }], &board, &colors_np(&state), &ctx());
        assert!(one_family.contains("only option — press Enter to open"), "{one_family}");

        let two = decision_zone(
            &d,
            &[Row::Family { words: "a" }, Row::Family { words: "b" }],
            &board,
            &colors_np(&state),
            &ctx(),
        );
        assert!(two.contains("pick a number"), "{two}");
        assert!(!two.contains("only option"), "{two}");
    }

    /// The hint makes no claim this build cannot honor: the two mock clauses
    /// whose surfaces have not shipped (T9's recall key, T5's card index) are
    /// absent rather than printed and false.
    #[test]
    fn the_hint_claims_nothing_that_has_not_shipped() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let d = screen_model::build_decision(&state, &rules, &[]);
        let zone =
            decision_zone(&d, &[Row::Family { words: "x" }], &board, &colors_np(&state), &ctx());
        // The recall clause is printed only where the line editor is live: a
        // piped run has no ↑, and this fixture is one.
        assert!(!zone.contains(glyph::HISTORY_KEY), "no recall claim without an editor: {zone}");
        let editing = decision_zone(
            &d,
            &[Row::Family { words: "x" }],
            &board,
            &colors_np(&state),
            &ZoneContext { page: 0, expert: "", game_over: false, recall: true },
        );
        assert!(
            editing.contains(&format!("{} recalls what you typed", glyph::HISTORY_KEY)),
            "and it IS printed where the editor is live — the guide sells that key: {editing}",
        );
        assert!(zone.contains("'back' undoes"), "{zone}");
        // Since T5 the mock's two navigation clauses are both true and both
        // printed, and they name two DIFFERENT screens -- a hint that still
        // said "'view' or '?'" would teach that either key reaches either one.
        assert!(zone.contains("'view' any surface"), "{zone}");
        assert!(zone.contains("'?' the card index"), "{zone}");
    }

    /// The zone ends on the caret, with nothing after it.
    #[test]
    fn the_zone_closes_on_the_prompt_caret() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let d = screen_model::build_decision(&state, &rules, &[]);
        let zone = decision_zone(&d, &[], &board, &colors_np(&state), &ctx());
        assert!(zone.ends_with("> \n"), "{zone:?}");
    }

    /// A page of short options packs into columns; the same count of LONG ones
    /// falls back to a single column. The row budget holds either way — that is
    /// what makes the zone's height bounded by construction rather than by how
    /// the options happen to read.
    ///
    /// The WIDTH bound this used to check alongside moved to
    /// [`no_option_list_at_any_width_costs_more_than_the_row_budget`], where it
    /// is swept rather than sampled at these two shapes (T4 review M2).
    #[test]
    fn the_option_list_packs_columns_or_falls_back_to_one_per_line() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let d = screen_model::build_decision(&state, &rules, &[]);

        let count_rows = |zone: &str| zone.lines().filter(|l| l.trim_start().starts_with('[')).count();

        // Short options pack three deep and 24 of them still cost one page.
        let short: Vec<Row> = (0..24).map(|_| Row::Family { words: "pay tea" }).collect();
        let packed = decision_zone(&d, &short, &board, &colors_np(&state), &ctx());
        assert_eq!(count_rows(&packed), 8, "24 short options pack 3 columns deep:\n{packed}");
        assert!(!packed.contains("more — 'more'"), "...on one page:\n{packed}");

        // Options too wide to share a line fall back to ONE column, and the
        // page then holds only as many as the row budget allows — which is the
        // property that bounds the zone's height whatever the options say.
        let long_text = "x".repeat(150);
        let long: Vec<Row> = (0..24).map(|_| Row::Family { words: &long_text }).collect();
        let single = decision_zone(&d, &long, &board, &colors_np(&state), &ctx());
        assert_eq!(count_rows(&single), OPTION_ROWS, "one column, one option per row:\n{single}");
        assert!(single.contains("more — 'more'"), "a long list pages: {single}");
    }

    /// The row budget is the bound, and it holds at EVERY width an option can
    /// render at — not only at the two the test above happens to pick.
    ///
    /// **And so does the WIDTH bound, which is why it moved here** (T4 review
    /// M2). It used to be checked on the two hand-picked lists above, against
    /// [`crate::screen::FLOOR_W`] — a check that could not see the defect it
    /// existed to catch, because a list of long options is exactly what the
    /// other test does NOT sweep and 200 columns is not the budget any real
    /// frame has. A 22-pawn recovery row measures 326 columns against a 4P
    /// frame that was 180 columns when the T4 review measured it and is 200
    /// since T10; the assertion below is per emitted LINE and against the
    /// count's own `frame_width`, so it follows the layout either way and a row
    /// that cannot be packed still has to wrap.
    #[test]
    fn no_option_list_at_any_width_costs_more_than_the_row_budget() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let d = screen_model::build_decision(&state, &rules, &[]);
        let budget = crate::screen::frame_width(usize::from(board.player_count));
        for width in [1usize, 5, 20, 40, 60, 61, 62, 95, 96, 130, 197, 250] {
            let text = "x".repeat(width);
            for count in [1usize, 2, 13, 27, 40, 200] {
                let rows: Vec<Row> = (0..count).map(|_| Row::Family { words: &text }).collect();
                let zone = decision_zone(&d, &rows, &board, &colors_np(&state), &ctx());
                let drawn = zone.lines().filter(|l| l.trim_start().starts_with('[')).count();
                assert!(
                    drawn <= OPTION_ROWS,
                    "{count} options of {width} columns cost {drawn} rows, past the {OPTION_ROWS}-row budget",
                );
                for line in zone.lines() {
                    assert!(
                        visible_width(line) <= budget,
                        "{count} options of {width} columns drew a {}-column line past the \
                         {budget}-column frame: {line}",
                        visible_width(line),
                    );
                }
            }
        }
    }

    /// Paging walks the list rather than dividing it, so "page N of M" is true
    /// at every page and `more` lands where the footer says.
    #[test]
    fn paging_covers_every_option_exactly_once() {
        // Mixed widths, so the walk cannot be right by accident on a uniform
        // list: a wide option mid-list ends its page early.
        let widths: Vec<usize> = (0..40).map(|i| if i % 7 == 0 { 150 } else { 30 }).collect();
        let mut covered = vec![0usize; widths.len()];
        let total = Page::of(&widths, crate::screen::FLOOR_W, 0).total;
        assert!(total > 1, "the fixture must actually page");
        for p in 0..total {
            let page = Page::of(&widths, crate::screen::FLOOR_W, p);
            assert!(page.start < page.end, "no page may be empty: {p}");
            for slot in covered.iter_mut().take(page.end).skip(page.start) {
                *slot += 1;
            }
        }
        assert!(covered.iter().all(|&c| c == 1), "{covered:?}");
        // A page past the end clamps to the last, never to an empty screen.
        assert_eq!(Page::of(&widths, crate::screen::FLOOR_W, 99).number, total - 1);
        assert_eq!(Page::of(&[], crate::screen::FLOOR_W, 0).total, 1);
        // One option wider than the whole floor still gets its page.
        assert_eq!(
            Page::of(&[500], crate::screen::FLOOR_W, 0),
            Page { start: 0, end: 1, number: 0, total: 1 },
        );
    }

    /// A row too wide for the frame WRAPS, hangs its tail under the option's
    /// own text, and loses nothing.
    ///
    /// Wrapping rather than clipping because what actually runs long on a real
    /// row is the priced detail: a retreat's per-area recovery list, a
    /// movement's fee recipients. Those are what the player is choosing
    /// BETWEEN, so a `…` on the right would cut off the answer to the question
    /// the row was drawn to ask.
    #[test]
    fn a_row_too_wide_for_the_frame_wraps_under_its_own_text() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let d = screen_model::build_decision(&state, &rules, &[]);
        let budget = crate::screen::frame_width(usize::from(board.player_count));

        // The shape a 22-pawn retreat draws: one long comma-run of area names,
        // 326 columns against the 4P frame's 180 when the T4 review measured it.
        let words: Vec<String> =
            (0..24).map(|i| format!("1 from PRODUCTION AREA (A{i:02})")).collect();
        let words = format!("Return the President to your hand — recovering 24 ({})", words.join(", "));
        let zone =
            decision_zone(&d, &[Row::Family { words: &words }], &board, &colors_np(&state), &ctx());

        let drawn: Vec<&str> = zone
            .lines()
            .skip_while(|l| !l.trim_start().starts_with('['))
            .take_while(|l| l.starts_with(OPTION_INDENT))
            .collect();
        assert!(drawn.len() > 1, "a {}-column row must have wrapped:\n{zone}", words.chars().count());
        assert!(drawn[0].starts_with("   [1] "), "{:?}", drawn[0]);
        for line in &drawn {
            assert!(visible_width(line) <= budget, "{} columns: {line}", visible_width(line));
        }
        for line in &drawn[1..] {
            assert!(
                line.starts_with("       ") && !line.trim_start().starts_with('['),
                "a continuation hangs under the option's text, unnumbered: {line:?}",
            );
        }
        // Every word survives the wrap, in order — the whole point of not
        // clipping. Joined on single spaces, since the break eats its own.
        let rejoined: Vec<&str> = drawn.iter().flat_map(|l| l.split_whitespace()).collect();
        let expected: Vec<&str> =
            std::iter::once("[1]").chain(words.split_whitespace()).collect();
        assert_eq!(rejoined, expected, "the wrap dropped or reordered a word");

    }

    /// A wrap that lands INSIDE a tinted run still closes every tone it opens.
    ///
    /// **This is the property [`Cell`] exists for.** The obvious implementation
    /// — render the row to a painted string, then split it at the budget —
    /// leaves the first line holding an opening SGR whose reset went to the
    /// second, and that tone then bleeds across the whole rest of the frame. It
    /// is invisible to a plain-text assertion and glaring on a real terminal,
    /// which is exactly the combination that ships. The fixture is a marked
    /// destination row whose priced tail is long enough to wrap, so the green
    /// `◈` run is what the break has to fall inside.
    #[test]
    fn a_wrap_inside_a_tinted_run_closes_every_tone_it_opens() {
        let (state, rules) = state_np(2);
        let dest = AreaId::new(3);
        let mut board = screen_model::build_board(&state, &rules, &[]);
        for tile in &mut board.tiles {
            tile.reachable = tile.area == dest;
        }
        let d = screen_model::build_decision(&state, &rules, &[]);
        let mv = Move::MovePresident { path: vec![dest], destination: dest };
        // A fee naming enough recipients to run the tinted tail past the frame.
        let fee = FeeModel {
            total: 60,
            to_players: (0..30).map(|i| (PlayerId::new(i % 4), 2)).collect(),
            canal_tolls: 0,
        };
        let rows = [Row::Move { mv: &mv, fee: Some(&fee), words: "fallback" }];

        let plain = decision_zone(&d, &rows, &board, &colors_np(&state), &ctx());
        crate::style::force(true);
        let painted = decision_zone(&d, &rows, &board, &colors_np(&state), &ctx());
        crate::style::force(false);

        assert_eq!(crate::style::strip_ansi(&painted), plain, "painting moved a visible character");
        let marked: Vec<&str> =
            painted.lines().filter(|l| l.contains(glyph::REACHABLE)).collect();
        assert_eq!(marked.len(), 1, "the fixture must draw exactly one marked row:\n{plain}");
        assert!(
            plain.lines().filter(|l| l.starts_with(OPTION_INDENT)).count() > 1,
            "the fixture must actually wrap:\n{plain}",
        );
        for line in painted.lines() {
            let opens = line.matches('\u{1b}').count();
            let closes = line.matches("\u{1b}[0m").count();
            assert_eq!(opens, closes * 2, "a wrapped line left an escape run open: {line:?}");
        }
    }

    /// A row with no space to break at still fits: the line breaks mid-token
    /// rather than shearing the frame, and every character survives.
    #[test]
    fn an_unbreakable_row_breaks_mid_token_rather_than_overrunning() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let d = screen_model::build_decision(&state, &rules, &[]);
        let budget = crate::screen::frame_width(usize::from(board.player_count));

        let words = "x".repeat(500);
        let zone =
            decision_zone(&d, &[Row::Family { words: &words }], &board, &colors_np(&state), &ctx());
        let drawn: Vec<&str> = zone
            .lines()
            .skip_while(|l| !l.trim_start().starts_with('['))
            .take_while(|l| l.starts_with(OPTION_INDENT))
            .collect();
        for line in &drawn {
            assert!(visible_width(line) <= budget, "{} columns: {line}", visible_width(line));
        }
        let kept: usize = drawn.iter().map(|l| l.matches('x').count()).sum();
        assert_eq!(kept, 500, "a mid-token break must not eat characters:\n{zone}");
    }

    /// **The one place the pager's wording is written down** (T5 review S4).
    /// Both shapes, in literals, in the module that owns the formatter — every
    /// other test in the crate compares its surface's footer AGAINST
    /// [`pager_footer`] rather than re-typing the sentence, so this is the
    /// single site a wording change has to pass through.
    #[test]
    fn the_pager_footer_words_both_shapes() {
        assert_eq!(
            pager_footer(12, 0, 2),
            "(12 more — 'more' for page 2 of 2, 'list' back to the first)",
        );
        // The wrap: from the last page `more` returns to the first, and the
        // footer says so rather than counting a remainder it does not have.
        let last = pager_footer(0, 1, 2);
        assert_eq!(last, "(last page — 'more' wraps to page 1 of 2, 'list' back to the first)");
        assert!(!last.contains("(0 more"), "the zero-remainder form reads as a bug: {last}");
        // Mid-list, the "next page" number is the one `more` actually lands on.
        assert!(pager_footer(7, 1, 3).contains("for page 3 of 3"), "{}", pager_footer(7, 1, 3));
    }

    /// The LAST page of a real decision zone words its footer as a wrap — the
    /// property from the surface's side, checked against the formatter rather
    /// than against a second copy of its sentence.
    #[test]
    fn the_last_pages_footer_words_the_wrap_instead_of_counting_zero() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let d = screen_model::build_decision(&state, &rules, &[]);
        let long_text = "x".repeat(150);
        let rows: Vec<Row> = (0..24).map(|_| Row::Family { words: &long_text }).collect();
        let colors = colors_np(&state);

        let first = decision_zone(&d, &rows, &board, &colors, &ctx());
        assert!(first.contains(&pager_footer(12, 0, 2)), "{first}");

        let last =
            decision_zone(&d, &rows, &board, &colors, &ZoneContext { page: 1, ..ctx() });
        assert!(!last.contains("(0 more"), "{last}");
        assert!(last.contains(&pager_footer(0, 1, 2)), "{last}");
    }

    /// A truncated near-miss list SAYS it was truncated. Three rows and three
    /// rows-of-five look identical on screen otherwise, and silence there reads
    /// as "those were all of them" — the opposite of what a cap means.
    #[test]
    fn a_capped_near_miss_list_says_how_many_it_cut() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let mut d = screen_model::build_decision(&state, &rules, &[]);
        d.not_offered = vec![NotOffered {
            area: AreaId::new(1),
            reason: NotOfferedReason::NoAssistantThere,
        }];
        let rows = [Row::Family { words: "x" }];
        let colors = colors_np(&state);

        let complete = decision_zone(&d, &rows, &board, &colors, &ctx());
        assert!(complete.contains("(not offered: "), "{complete}");
        assert!(!complete.contains("and 0 more"), "an uncapped list says nothing: {complete}");

        d.not_offered_suppressed = 2;
        let capped = decision_zone(&d, &rows, &board, &colors, &ctx());
        assert!(capped.contains("(…and 2 more)"), "{capped}");
    }

    /// A teleport states its price ONCE. `path.is_empty()` is the whole
    /// difference between a walk and a hop, and wording it "free teleport" and
    /// then pricing it "· free" said the same thing twice.
    #[test]
    fn a_teleport_row_states_its_price_once() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let dest = AreaId::new(3);

        let hop = Move::MovePresident { path: vec![], destination: dest };
        let row = option_text(0, &Row::Move { mv: &hop, fee: None, words: "fallback" }, &board, &colors_np(&state))
            .painted();
        assert!(row.contains("free teleport"), "{row}");
        assert_eq!(row.matches("free").count(), 1, "the price is stated once: {row}");

        // A walk still prices itself, teleport wording nowhere near it.
        let walk = Move::MovePresident { path: vec![dest], destination: dest };
        let walked = option_text(0, &Row::Move { mv: &walk, fee: None, words: "fallback" }, &board, &colors_np(&state))
            .painted();
        assert!(walked.contains("1 step · free"), "{walked}");
        assert!(!walked.contains("teleport"), "{walked}");

        // ...and a hop the model DID price says what it costs, once — the arm
        // that keeps "free teleport" from being an assumption about fees.
        let paid = FeeModel { total: 2, to_players: vec![(PlayerId::new(1), 2)], canal_tolls: 0 };
        let priced =
            option_text(0, &Row::Move { mv: &hop, fee: Some(&paid), words: "fallback" }, &board, &colors_np(&state))
                .painted();
        // The recipient is named by COLOUR (T11's rule), which is the same
        // vocabulary `describe::placement_fee_note` has always used.
        assert!(priced.contains("teleport · pay ¥2 (2 to Blue)"), "{priced}");
        assert!(!priced.contains("seat 1"), "a player is not a seat here: {priced}");
        assert!(!priced.contains("free"), "{priced}");
    }

    /// **The option row's palette, as the approved mock draws it** (T11's
    /// deliberate pass, deferred by the T4 review).
    ///
    /// Three treatments, and the reason each is what it is: the AREA NAME is
    /// bold because it is what a player scans the list for; the `◈` is the one
    /// green thing because it is the engine's own reachability mark and the
    /// board tints the same tile the same way; the TAIL — steps, price,
    /// recipients — is plain, on every row, whether or not the row is marked.
    ///
    /// That last clause is the one that changed. The tail used to inherit the
    /// mark's green on marked rows and stay plain on unmarked ones, so the same
    /// price read in two colours depending on a fact about the destination —
    /// which is the "option tails get the same tint treatment everywhere"
    /// ruling this test now pins.
    #[test]
    fn an_option_rows_tail_is_plain_whether_or_not_the_row_is_marked() {
        let (state, rules) = state_np(2);
        let dest = AreaId::new(3);
        let walk = Move::MovePresident { path: vec![dest], destination: dest };
        let colors = colors_np(&state);

        // Two boards over the SAME move: one that marks the destination
        // reachable and one that does not.
        let mut marked = screen_model::build_board(&state, &rules, &[]);
        let mut plain = marked.clone();
        for tile in marked.tiles.iter_mut() {
            tile.reachable = tile.area == dest;
        }
        for tile in plain.tiles.iter_mut() {
            tile.reachable = false;
        }

        crate::style::force(true);
        let with_mark =
            option_text(0, &Row::Move { mv: &walk, fee: None, words: "fallback" }, &marked, &colors)
                .painted();
        let without =
            option_text(0, &Row::Move { mv: &walk, fee: None, words: "fallback" }, &plain, &colors)
                .painted();
        // Painted INSIDE the forced window with the rows themselves. Outside it
        // `paint` is the identity, so `bold` would be the bare `"COPPER MINE"`
        // and the two assertions below would degenerate into plain `contains` —
        // passing just as happily against a row that had dropped the tone
        // entirely, which is the whole thing they exist to catch (T11 review
        // F4). The mutation that proves it: delete `Tone::Strong` from the area
        // name in `option_text` and this test must fail.
        let bold = crate::style::paint(Tone::Strong, "COPPER MINE");
        crate::style::force(false);

        assert!(with_mark.contains(glyph::REACHABLE), "the marked row draws its mark");
        assert!(!without.contains(glyph::REACHABLE), "the unmarked one does not");

        // The tail is the same BYTES on both rows, escapes included — which is
        // the whole content of "the same tint treatment everywhere".
        let tail_of = |row: &str| {
            let at = row.rfind("1 step").expect("the tail");
            row[at..].to_string()
        };
        assert_eq!(tail_of(&with_mark), tail_of(&without), "the tail is tinted identically");
        assert!(
            !tail_of(&with_mark).contains('\u{1b}'),
            "and it is plain: {:?}",
            tail_of(&with_mark),
        );

        // The area name is bold on both, which is the other half of the pass.
        assert!(bold.contains('\u{1b}'), "anti-vacuity: `bold` must carry its escape");
        assert!(with_mark.contains(&bold), "the area name leads in bold: {with_mark:?}");
        assert!(without.contains(&bold), "on both rows: {without:?}");
    }

    /// Painting never moves a visible character — the crate's standing
    /// alignment rule, checked on the whole zone including its packed columns.
    #[test]
    fn a_painted_zone_strips_back_to_the_plain_one() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let d = screen_model::build_decision(&state, &rules, &[]);
        // Past `OPTION_ROWS`, so the packed-column path is what round-trips
        // here: at seven rows the list stays one per line and the assertion
        // never saw a padded column at all (T4 review S5).
        let rows: Vec<Row> = (0..24).map(|_| Row::Family { words: "take copper" }).collect();

        crate::style::force(true);
        let painted = decision_zone(&d, &rows, &board, &colors_np(&state), &ctx());
        crate::style::force(false);
        let plain = decision_zone(&d, &rows, &board, &colors_np(&state), &ctx());
        assert_ne!(painted, plain, "forced color must actually paint");
        assert_eq!(crate::style::strip_ansi(&painted), plain);
    }
}
