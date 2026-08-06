//! The decision zone: the phase chain, the "YOU ARE DECIDING" line, the
//! numbered options, the dimmed near-miss rows and the standing hint — drawn
//! from the screen model alone (Phase-3.5a spec §0.7/§0.8, plan T4; design
//! authority is the approved mock pair
//! `2026-08-05-phase3.5a-mocks/main-frame.ans` + `main-frame-mid.ans`).
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
    BoardModel, Deciding, DecisionModel, FeeModel, NotOffered, NotOfferedReason,
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
        let (texts, widths) = option_texts(rows, board);
        let budget = column_budget(board);
        let page = Page::of(&widths, budget, ctx.page);
        out.push_str(&option_block(&texts, &widths, &page, budget));
        for row in &d.not_offered {
            out.push_str(OPTION_INDENT);
            out.push_str(&paint(Tone::Dim, &not_offered_row(row, board)));
            out.push('\n');
        }
        if page.total > 1 {
            out.push_str(OPTION_INDENT);
            out.push_str(&paint(
                Tone::Chrome,
                &format!(
                    "({} more — 'more' for page {} of {}, 'list' back to the first)",
                    rows.len() - page.end,
                    page.next_number(),
                    page.total,
                ),
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

    /// The 1-based number `more` will land on: the next page, wrapping.
    fn next_number(&self) -> usize {
        (self.number + 1) % self.total + 1
    }
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

/// Every option's rendered text and its PLAIN width, in list order.
///
/// Rendered once and measured once: the pager needs the widths to decide where
/// a page ends, and the renderer needs the same texts to draw it, so computing
/// them apart would be two chances to disagree about where page two starts.
fn option_texts(rows: &[Row<'_>], board: &BoardModel) -> (Vec<String>, Vec<usize>) {
    let texts: Vec<String> =
        rows.iter().enumerate().map(|(i, row)| option_text(i, row, board)).collect();
    let widths = texts.iter().map(|t| crate::style::strip_ansi(t).chars().count()).collect();
    (texts, widths)
}

/// How many pages a row list makes — the pager's wraparound, walked by exactly
/// the same function the footer's "page N of M" reads.
pub fn page_count(rows: &[Row<'_>], board: &BoardModel) -> usize {
    page_bounds(&option_texts(rows, board).1, column_budget(board)).len()
}

/// The numbered options of one page, packed into as many columns as fit.
fn option_block(all: &[String], plain_all: &[usize], page: &Page, budget: usize) -> String {
    let texts = &all[page.start..page.end];
    let plain = &plain_all[page.start..page.end];
    let widest = plain.iter().copied().max().unwrap_or(0);
    let columns = columns_for(texts.len(), widest, budget);

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
            line.push_str(&texts[i]);
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
/// The distinction is not cosmetic. A 3-player frame is 163 columns wide — the
/// board is narrower than the floor and the dashboard stacks below it — so a
/// zone packed to 200 would print option rows 19 columns past the widest thing
/// above them, and the frame's own width pin would (correctly) call that a
/// defect. The player count is the one layout fact the board model carries, and
/// this is the one place the zone needs it.
fn column_budget(board: &BoardModel) -> usize {
    crate::screen::frame_width(usize::from(board.player_count))
}

/// One numbered option: `[3] Move to CUSTOMS ◈ — 1 step · free`.
fn option_text(number: usize, row: &Row<'_>, board: &BoardModel) -> String {
    let mut t = Twin::default();
    t.tinted(Tone::Strong, &format!("[{number}]"));
    t.raw(" ");
    match row {
        Row::Family { words } => {
            t.raw(words);
        }
        Row::Move { mv, fee, words } => match move_row(mv, *fee, board) {
            Some(worded) => {
                t.raw(&worded.plain_prefix).tinted(worded.tone, &worded.marked);
            }
            None => {
                t.raw(words);
            }
        },
    };
    t.painted
}

/// A re-worded move row: the part that takes no tint, and the part that does
/// (the `◈` mark and what follows it).
struct WordedMove {
    plain_prefix: String,
    marked: String,
    tone: Tone,
}

/// The five move kinds the mocks draw, worded from the MODEL — destination
/// name from the board, `◈` from its reachable mark, step count from the
/// path, fee from the priced [`FeeModel`]. `None` for every other kind, which
/// leaves [`option_text`] printing `describe`'s own words (module header).
fn move_row(mv: &Move, fee: Option<&FeeModel>, board: &BoardModel) -> Option<WordedMove> {
    match mv {
        Move::MovePresident { path, destination } => {
            let steps = if path.is_empty() {
                "free teleport".to_string()
            } else {
                format!("{} step{}", path.len(), if path.len() == 1 { "" } else { "s" })
            };
            Some(destination_row(
                "Move to",
                *destination,
                board,
                &format!("{steps} {} {}", glyph::SEPARATOR, fee_words(fee)),
            ))
        }
        Move::PlacePresident { destination } => Some(destination_row(
            "Place your President in",
            *destination,
            board,
            &fee_words(fee),
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
            Some(WordedMove {
                plain_prefix: format!(
                    "Return the President to your hand {} {tail}",
                    glyph::SECTION_DASH
                ),
                marked: String::new(),
                tone: Tone::Chrome,
            })
        }
        Move::PlaceThreeAreas { placements } if !placements.is_empty() => {
            let areas: Vec<String> =
                placements.iter().map(|a| area_name_with_id(board, *a)).collect();
            Some(WordedMove {
                plain_prefix: format!(
                    "Place 1 assistant in {}{}",
                    areas.join(", "),
                    fee_tail(fee),
                ),
                marked: String::new(),
                tone: Tone::Chrome,
            })
        }
        Move::PlaceTwoSameArea { area, adjacent_bonus } => {
            let bonus = match adjacent_bonus {
                Some(adj) => format!(" and 1 in {} (Telegram)", area_name_with_id(board, *adj)),
                None => String::new(),
            };
            Some(WordedMove {
                plain_prefix: format!(
                    "Place 2 assistants in {}{bonus}{}",
                    area_name_with_id(board, *area),
                    fee_tail(fee),
                ),
                marked: String::new(),
                tone: Tone::Chrome,
            })
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
    WordedMove {
        plain_prefix: format!("{verb} {} ", area_name(board, area)),
        marked: if marked { format!("{} ", glyph::REACHABLE) } else { String::new() },
        tone: Tone::Reachable,
    }
    .with_tail(tail)
}

impl WordedMove {
    /// Append the `— <tail>` clause after the marked segment.
    ///
    /// The tail rides the PREFIX rather than the marked segment: only the `◈`
    /// itself is tinted [`Tone::Reachable`], and a fee painted in the
    /// destination-mark color would claim a meaning it does not have.
    fn with_tail(mut self, tail: &str) -> WordedMove {
        // The mark already carries its trailing space when present; without it
        // the prefix's own space closes the gap.
        let dash = format!("{} {tail}", glyph::SECTION_DASH);
        if self.marked.is_empty() {
            self.plain_prefix.push_str(&dash);
        } else {
            self.marked.push_str(&dash);
        }
        self
    }
}

/// `free` / `pay ¥2 (2 to Blue)` / `pay ¥3 (2 to Blue, 1 canal toll)`.
///
/// Player recipients first in seat order, the bank's canal toll last — the
/// order `describe` prints and the order the money actually moves in.
/// [`FeeModel`] carries no color table, so recipients are named by SEAT here;
/// the callers that have colors say so in their own words.
fn fee_words(fee: Option<&FeeModel>) -> String {
    let Some(fee) = fee else { return "free".to_string() };
    if fee.total == 0 {
        return "free".to_string();
    }
    let mut parts: Vec<String> =
        fee.to_players.iter().map(|(id, amount)| format!("{amount} to seat {}", id.0)).collect();
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
fn fee_tail(fee: Option<&FeeModel>) -> String {
    match fee {
        Some(fee) if fee.total > 0 => format!(" {} {}", glyph::SECTION_DASH, fee_words(Some(fee))),
        _ => String::new(),
    }
}

// ---------------------------------------------------------------------------
// The standing hint
// ---------------------------------------------------------------------------

/// The mocks' standing hint, minus the two clauses that are not true yet.
///
/// **Two deviations from the approved mock, both because the mock describes a
/// finished 3.5a and this is the middle of one.** The mock's hint names
/// `↑ recalls what you typed` — the line editor is T9's, and until it ships an
/// arrow key produces an escape sequence, so claiming recall would be a lie
/// printed on every screen. It also names `'?' the card index` — the index is
/// T5's, and `?` today opens the view menu, which is what this says instead.
/// Both clauses come back with the surfaces that make them true; neither is
/// dropped.
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
    parts.push("'back' undoes the last applied entry".to_string());
    parts.push("'view' or '?' any surface".to_string());
    parts.push("'help' for commands".to_string());
    wrap_hint(&parts, crate::screen::frame_width(usize::from(players)))
}

/// The hint's clauses packed into as few lines as fit `budget` columns, each
/// continued line signalled by the trailing separator on the one before it.
///
/// **It wraps rather than being written short enough to fit**, because "short
/// enough" is not one length: the frame is 198 columns at 2 players and 163 at
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

    fn ctx() -> ZoneContext<'static> {
        ZoneContext { page: 0, expert: "", game_over: false }
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
        let unmarked = option_text(0, &Row::Move { mv: &mv, fee: None, words: "fallback" }, &board);
        assert!(!unmarked.contains(glyph::REACHABLE), "{unmarked}");

        for tile in &mut board.tiles {
            tile.reachable = tile.area == dest;
        }
        let marked = option_text(0, &Row::Move { mv: &mv, fee: None, words: "fallback" }, &board);
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
        let words = fee_words(Some(&paid));
        assert_eq!(words, "pay ¥3 (2 to seat 1, 1 canal toll)", "{words}");
        assert_eq!(fee_words(None), "free");
        assert_eq!(
            fee_words(Some(&FeeModel { total: 0, to_players: Vec::new(), canal_tolls: 0 })),
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
        let row = option_text(4, &Row::Move { mv: &mv, fee: None, words: &words }, &board);
        assert_eq!(row, format!("[4] {words}"), "{row}");
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
        assert!(!zone.contains(glyph::HISTORY_KEY), "no recall claim before T9: {zone}");
        assert!(!zone.contains("card index"), "no index claim before T5: {zone}");
        assert!(zone.contains("'back' undoes"), "{zone}");
        assert!(zone.contains("'view' or '?'"), "{zone}");
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
    /// falls back to a single column rather than running past the design floor.
    /// The row budget holds either way — that is what makes the zone's height
    /// bounded by construction rather than by how the options happen to read.
    #[test]
    fn the_option_list_packs_columns_but_never_past_the_floor() {
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

        for zone in [&packed, &single] {
            for line in zone.lines() {
                assert!(
                    visible_width(line) <= crate::screen::FLOOR_W,
                    "a packed row ran past the floor ({} columns): {line}",
                    visible_width(line),
                );
            }
        }
    }

    /// The row budget is the bound, and it holds at EVERY width an option can
    /// render at — not only at the two the test above happens to pick.
    #[test]
    fn no_option_list_at_any_width_costs_more_than_the_row_budget() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let d = screen_model::build_decision(&state, &rules, &[]);
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

    /// Painting never moves a visible character — the crate's standing
    /// alignment rule, checked on the whole zone including its packed columns.
    #[test]
    fn a_painted_zone_strips_back_to_the_plain_one() {
        let (state, rules) = state_np(2);
        let board = screen_model::build_board(&state, &rules, &[]);
        let d = screen_model::build_decision(&state, &rules, &[]);
        let rows: Vec<Row> = (0..7).map(|_| Row::Family { words: "take copper" }).collect();

        crate::style::force(true);
        let painted = decision_zone(&d, &rows, &board, &colors_np(&state), &ctx());
        crate::style::force(false);
        let plain = decision_zone(&d, &rows, &board, &colors_np(&state), &ctx());
        assert_ne!(painted, plain, "forced color must actually paint");
        assert_eq!(crate::style::strip_ansi(&painted), plain);
    }
}
