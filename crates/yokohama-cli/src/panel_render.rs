//! The dashboard: the players/tracks/achievements/hand panels and the title
//! strip, drawn from the screen model alone (Phase-3.5a spec §0.3, plan T3;
//! design authority is the approved mock pair
//! `2026-08-05-phase3.5a-mocks/main-frame.ans` + `main-frame-mid.ans`, whose
//! generator names this column's width `DASH_W`).
//!
//! **The same half of the R9 seam [`crate::board_render`] holds, and the import
//! list is the proof.** This module renders; it never reads game state. Its
//! inputs are [`crate::screen_model`]'s pure data plus a seat color table, and
//! the only `yokohama_core` names it touches are the TYPES a panel matches on.
//! `GameState`, `Rules` and `Session` are absent by construction — the adapter
//! that reaches for them is [`crate::render::dashboard`], one function, on the
//! other side of this seam. Glyphs come only from [`crate::style::glyph`] and
//! color only from [`crate::style::Tone`].
//!
//! **Ragged right, with two exceptions, and the distinction is load-bearing.**
//! A panel is joined beside the board by [`crate::screen`], which pads the
//! BOARD to its own width and appends each dashboard line unpadded — so a
//! content line costs exactly what it says and never widens the frame past what
//! its longest line asked for. What must be exact are the lines that draw a
//! SHAPE: the section rules (which would stop being rules) and the hand card's
//! box (which would shear). Every content line is nonetheless CLIPPED at
//! [`DASH_W`], because a panel line past its column is a line written over
//! whatever sits to its right.
//!
//! **TWO ARRANGEMENTS since T10, and they share their models, not their
//! layout.** [`dashboard`] is the BESIDE form: a [`DASH_W`]-wide strip joined
//! to the right of a 2-player board, panels stacked vertically, one line per
//! owned Technology. [`dashboard_below`] is the BELOW form, at 3 and 4 players,
//! where a 150- or 180-column board leaves no room beside it: the players panel
//! turns on its side into one column per seat and the other three compress onto
//! a single titled row each. Which one runs is [`crate::screen`]'s decision,
//! taken from the geometry.
//!
//! **Where this deviates from the mocks, and why.** Recorded rather than
//! silent. (1) The section rules run to exactly [`DASH_W`]; the mocks'
//! generator computes its fill as `DASH_W - 4 - len(title)` against three
//! columns of fixed chrome and so lands one short, at 73. That is generator
//! arithmetic, not a design intent — the intent is a rule that spans the
//! dashboard. (2) In the BESIDE form Technologies get ONE LINE EACH carrying a
//! summary of what the card does (spec §0.3's "per-owned-tech summary lines");
//! the mocks show a single `Tech  Telegram · Mercenaries …` line, which is that
//! shape at exactly one owned card and has nowhere to put a summary.
//! Mercenaries therefore take their own line. (3) Achievements carry ONE
//! requirement column rather than the mocks' short-name-plus-requirement pair:
//! the widest condition in the set (`Buildings in 3 production + 2 commercial
//! areas`) is 46 columns, and the two-column form was drawn over the three
//! shortest cards in the deck.
//!
//! **And four more in the BELOW form, all of them about columns that are 48
//! wide where the beside strip is 74.** (4) The stride is
//! [`SEAT_COL_W`] = 50 rather than the 4P mock's 49, which lands the block on
//! the 200-column design floor that mock's own frame is drawn at. (5) The mock
//! draws seven lines per seat and no tech, mercenary, seat-number, locked-pawn
//! or next-take figure at all; this keeps every one of them — the first two on
//! their own rows, the last FOUR on a tail row (the orders in hand, the orders
//! filled, the locked pawns and the next-take price), since a fourth zone on the
//! assistants line costs 51 columns and the trade-house line with its price
//! costs 61. That row is over its column at the corner and yields the next-take
//! price alone when it is -- see [`seat_tail_row`], which states the policy and
//! its two measured maxima. Technologies are NAMED rather than explained there, because the
//! shortest [`tech_summary`] needs 34 columns beside a name and a label. (6)
//! The mock folds TRACKS and ACHIEVEMENTS onto ONE 192-column line; they get a
//! row each here, because the 3-player block is [`BELOW_BLOCK_MIN`] columns and
//! a combined line could not be total at both counts — and the split is what
//! buys the achievements back their VP tiers and claim state. (7) The mock's
//! `President in hand · 1 order (hidden)` is ONE line and this draws TWO —
//! [`president_row`] and [`seat_tail_row`] — because the orders count travels
//! with the locked-pawn tally and the next-take price rather than with the
//! President (see (5)), and no information is lost by the split: both facts
//! still appear, on adjacent rows of the same column. **The BESIDE form splits
//! them too since branch-4**, for the opposite reason: the orders facts grew a
//! filled clause with card codes on it and earned [`orders_line`] to themselves,
//! leaving [`president_line`] a location and nothing else.

use yokohama_core::{
    AchievementType, AreaId, OrderCardId, PlayerColor, PlayerId, PresidentLocation,
    TechEffect,
};

use crate::board_render::{color_of, exact, Twin};
use crate::card_render::{self, CardFace};
use crate::describe;
use crate::screen_model::{
    AchievementModel, AchievementsPanel, BoardModel, HandCardModel, HandPanel, Panel, PlayerModel,
    PlayersPanel, TitleMode, TitleModel, TracksPanel,
};
use crate::style::{glyph, GoodKind, Tone};

/// The dashboard column's designed width — the approved mocks' own `DASH_W`.
/// The authority for the number lives here, with the panels that fill it;
/// [`crate::screen`] reads it to decide whether a dashboard fits beside a
/// board.
pub const DASH_W: usize = 74;

/// A mini order card's total width, borders included — the mocks' `CARD_W`.
///
/// **Re-exported, not declared.** The card itself moved to
/// [`crate::card_render`] at T5, so the dashboard's hand and the full-screen
/// index draw the same emitter; this name stays because `screen` and the hand
/// row's own arithmetic pin read it here.
pub use crate::card_render::CARD_W;

use crate::card_render::CARD_H;

/// Columns a panel's content is indented under its section rule.
const INDENT: usize = 2;

/// Column a player's money field starts at. Sized off the longest header
/// segment that line can carry — `▶ YELLOW — your turn [bot]`, 26 columns — so
/// the money, VP and seat columns stay aligned down the panel instead of
/// stepping in and out the way the mocks' two hand-written rows do.
const NAME_COL: usize = 28;

/// Cards drawn in the hand panel before it stops and counts the rest. The
/// panel is a SUMMARY (spec §0.5) and `hand` is the full surface, so the bound
/// is on ROWS rather than on how many orders a player may actually hold.
const HAND_CARDS_SHOWN: usize = 4;

/// Hand cards drawn side by side per row: two 24-column cards and a gap fit
/// [`DASH_W`] with room to spare, three do not.
///
/// `a_hand_row_of_cards_fits_the_dashboard_column` is the pin, and it exists
/// because a hand row is the ONE dashboard line that never passes through
/// [`line`] — a card row is assembled from fixed-width card slices and pushed
/// raw, so nothing clips it and a third card per row would simply write four
/// columns over whatever sits to the right.
const HAND_CARDS_PER_ROW: usize = 2;

/// Columns between two hand cards on the same row. Named rather than spelled
/// at the one push site so the arithmetic pin and the renderer read the same
/// number instead of agreeing by coincidence.
const HAND_CARD_GAP: usize = 2;

/// The bot marker, shared by the two surfaces that draw one.
///
/// Deliberately short: it rides the END of a player's name label, which
/// [`player_header`] pads to [`NAME_COL`] — wide enough to hold the longest
/// badged header there is — so a badged frame is exactly as wide as an unbadged
/// one and guide §2's published widths do not move.
/// `render::bot_badge_is_width_neutral_in_the_dashboard` is that claim's pin.
///
/// **One const rather than two since the T3 review.** The dashboard spelled
/// `" [bot]"` inline here and [`crate::render`] held its own `BOT_BADGE` for the
/// `players` view, so the two surfaces agreed on the string only by coincidence
/// — and a player reading `[bot]` on one screen and something else on the other
/// has no way to tell which is the marker.
/// `render::the_bot_badge_is_one_string_on_every_surface` is the pin.
pub(crate) const BOT_BADGE: &str = " [bot]";

/// Columns the achievement requirement column spans before the VP column.
const GOAL_TEXT_W: usize = 30;
/// Columns the achievement VP column spans before the claim state.
const GOAL_VP_W: usize = 11;

// ---------------------------------------------------------------------------
// Shared line machinery
// ---------------------------------------------------------------------------

/// A content line: painted, never padded, and never past [`DASH_W`].
///
/// Clipping rather than padding is the whole difference between a content line
/// and a shape line — see the module doc. [`exact`] does both jobs, so a line
/// already inside its column comes back untouched.
fn line(t: &Twin) -> String {
    if t.width() <= DASH_W {
        return t.painted.clone();
    }
    exact(t, DASH_W)
}

/// A content line at a width other than [`DASH_W`] — the below arrangement's
/// seat columns (48 wide at 4 players, 56 at 3) and its compressed panel rows
/// (the whole block, 175 or 200). Same rule, one number.
fn line_at(t: &Twin, width: usize) -> String {
    if t.width() <= width {
        return t.painted.clone();
    }
    exact(t, width)
}

/// A section rule: `─ PLAYERS ────…` at exactly [`DASH_W`] columns.
fn header(title: &str) -> String {
    header_at(title, DASH_W)
}

/// [`header`] at an arbitrary width — the below arrangement's blocks are 175
/// and 200 columns, not 74.
fn header_at(title: &str, width: usize) -> String {
    let mut t = Twin::default();
    t.tinted(Tone::Chrome, &format!("{} ", glyph::RULE)).tinted(Tone::Strong, title).raw(" ");
    let fill = width.saturating_sub(t.width());
    t.tinted(Tone::Chrome, &String::from(glyph::RULE).repeat(fill));
    exact(&t, width)
}

/// A section rule that CARRIES its panel instead of introducing it:
/// `─ TRACKS  Church …`, with no trailing fill.
///
/// The approved 4P mock's own shape for a panel compressed onto one row (its
/// line 36 draws the TRACKS rule, its content and the ACHIEVEMENTS summary on a
/// single line and simply stops). Ragged right and clipped at `width`, like
/// every other content line — only [`header_at`] draws a shape.
///
/// **The clip is now a SAFETY NET rather than a working part**, and it is kept
/// deliberately. Since [`BELOW_BLOCK_MIN`] widened the 3-player block past the
/// widest legal compressed row, no state this driver can reach clips here — so
/// the branch is exercised by exactly one thing, its own unit pin
/// (`an_over_wide_compressed_row_is_clipped_to_its_block_not_overrun`) with a
/// synthetic over-wide body. Removing it would leave the whole frame resting on
/// an arithmetic argument about strings nobody re-measures: a row past its block
/// is written over whatever sits to its right, and a clipped tail is a bad
/// screen where an overrun is a broken one.
fn header_row(title: &str, body: &Twin, width: usize) -> String {
    let mut t = Twin::default();
    t.tinted(Tone::Chrome, &format!("{} ", glyph::RULE)).tinted(Tone::Strong, title).raw("  ");
    // Both halves of the pair, appended together — the one place this module
    // concatenates two Twins, and doing it field-by-field is what keeps the
    // plain text and the painted text describing the same line.
    t.plain.push_str(&body.plain);
    t.painted.push_str(&body.painted);
    line_at(&t, width)
}

/// A content line under a section rule, already indented.
fn indented() -> Twin {
    let mut t = Twin::default();
    t.raw(&" ".repeat(INDENT));
    t
}

/// ` · ` — the separator packing facts onto one line.
fn sep(t: &mut Twin) -> &mut Twin {
    t.raw(&format!(" {} ", glyph::SEPARATOR))
}

/// A player color's display name. Local to this module for the same reason
/// [`crate::board_render`] keeps its own `color_letter`: a renderer that named
/// colors through `render` would depend on the module that reads `GameState`.
fn color_name(c: PlayerColor) -> &'static str {
    match c {
        PlayerColor::Red => "RED",
        PlayerColor::Blue => "BLUE",
        PlayerColor::Yellow => "YELLOW",
        PlayerColor::Green => "GREEN",
    }
}

/// `count` with `noun` pluralized by a trailing `s`. Every noun this module
/// counts pluralizes that way.
fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("{count} {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// A board tile's display NAME, or its id when the board model does not hold
/// it. The board is the only place a renderer can learn what area `A06` is —
/// [`PresidentLocation`] carries an id and this module may not open a
/// `GameState` to resolve one.
fn area_name(board: &BoardModel, area: AreaId) -> String {
    board
        .tiles
        .iter()
        .find(|t| t.area == area)
        .map(|t| describe::area_type_name(t.area_type).to_uppercase())
        .unwrap_or_else(|| format!("A{:02}", area.0))
}

// ---------------------------------------------------------------------------
// The title strip
// ---------------------------------------------------------------------------

/// The frame's opening line: the game's name, then the session facts a
/// [`TitleModel`] carries, centred over `width`.
///
/// **The `═` that used to fill this line is gone rather than converted.**
/// style.rs's DEFERRED CONVERSIONS table routed U+2550 here on the reasoning
/// that the strip either adopts a vocabulary glyph or admits its own; the
/// approved mocks answer it a third way — their title strip is centred text on
/// an otherwise empty line, with no rule character at all — so the redesigned
/// strip draws none. (The crate DOES spell that code point again since
/// branch-4: the end-trigger banner took the row's other option and admitted
/// it as [`crate::style::glyph::DOUBLE_RULE`]. Nothing changed here — this
/// strip still draws no rule.)
pub fn title_strip(title: &TitleModel, width: usize) -> String {
    let mut t = Twin::default();
    t.tinted(Tone::Strong, "Y O K O H A M A").raw("   ");

    let mut facts = Twin::default();
    facts.raw(match title.mode {
        TitleMode::Live => "live tracking",
        TitleMode::Sim => "simulation",
    });
    sep(&mut facts).raw(&format!("seed {}", abbreviated_seed(title.seed)));
    if title.autosaved {
        sep(&mut facts).raw("autosaved");
    }
    t.tinted(Tone::Chrome, &facts.plain);

    let pad = width.saturating_sub(t.width()) / 2;
    format!("{}{}", " ".repeat(pad), t.painted)
}

/// A seed short enough to read and long enough to tell two games apart: the
/// last nine digits behind an ellipsis, or the whole number when it is already
/// that short. The ellipsis is never printed over a seed it did not elide — a
/// strip claiming `…7` for the seed 7 would misname the identity a player uses
/// to find a save file.
fn abbreviated_seed(seed: u64) -> String {
    const KEPT: usize = 9;
    let text = seed.to_string();
    if text.len() <= KEPT {
        return text;
    }
    format!("{}{}", glyph::ELLIPSIS, &text[text.len() - KEPT..])
}

// ---------------------------------------------------------------------------
// The dashboard
// ---------------------------------------------------------------------------

/// The whole dashboard: every panel in the order the model lists them,
/// separated by one blank line.
pub fn dashboard(panels: &[Panel], board: &BoardModel, colors: &[PlayerColor]) -> String {
    let mut out = String::new();
    for (i, panel) in panels.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        for l in panel_lines(panel, board, colors) {
            out.push_str(&l);
            out.push('\n');
        }
    }
    out
}

/// One panel's lines, section rule included.
fn panel_lines(panel: &Panel, board: &BoardModel, colors: &[PlayerColor]) -> Vec<String> {
    match panel {
        Panel::Players(p) => players_lines(p, board, colors),
        Panel::Tracks(t) => tracks_lines(t, colors),
        Panel::Achievements(a) => achievements_lines(a, colors),
        Panel::Hand(h) => hand_lines(h),
    }
}

// ---------------------------------------------------------------------------
// Players
// ---------------------------------------------------------------------------

/// Every seat's public picture: who is to move, what they hold, what they have
/// built, what they own, and where their President stands.
///
/// Hand CONTENTS never appear here — only counts — so the panel is safe to
/// leave on screen while a live table passes the laptop around, whatever the
/// privacy toggle says. `(hidden)` beside the orders count declares the mask
/// rather than being it: [`HandPanel`] is the only panel carrying card
/// identities, and it empties itself.
fn players_lines(panel: &PlayersPanel, board: &BoardModel, colors: &[PlayerColor]) -> Vec<String> {
    let mut out = vec![header("PLAYERS")];
    for (i, p) in panel.players.iter().enumerate() {
        if i > 0 {
            out.push(String::new());
        }
        out.push(player_header(p, colors));
        out.push(goods_line(p));
        out.push(assistants_line(p));
        out.push(shophouses_line(p));
        out.push(trading_houses_line(p));
        out.push(orders_line(p));
        out.extend(tech_lines(p));
        out.push(mercenaries_line(p));
        out.push(president_line(p, board, colors));
    }
    out
}

/// `▶ RED — your turn [bot]     ¥3   0 VP   seat 0`.
///
/// The seat NUMBER survives the redesign deliberately. It is the key
/// `reveal <seat>` is typed with, it appears on no other always-visible
/// surface, and the mocks simply do not draw a live-tracking state that needs
/// it — so it lands here as a chrome annotation at the right, where the order
/// cards put their engine ids, rather than being dropped because an
/// illustrative frame had no use for it.
fn player_header(p: &PlayerModel, colors: &[PlayerColor]) -> String {
    let mut label = String::new();
    if p.is_current {
        label.push(glyph::ACTIVE);
        label.push(' ');
    } else {
        label.push_str("  ");
    }
    label.push_str(color_name(p.color));
    if p.is_current {
        label.push_str(" — your turn");
    }
    if p.is_bot {
        label.push_str(BOT_BADGE);
    }

    // Painted as ONE run so the marker, the name and the badge read as a
    // single owner-colored label rather than three tokens that happen to touch.
    let mut t = Twin::default();
    match color_of(colors, p.seat) {
        Some(c) => t.tinted(Tone::Player(c), &label),
        None => t.raw(&label),
    };
    t.pad_to(NAME_COL);
    t.tinted(Tone::Good(GoodKind::Yen), &format!("{}{}", glyph::YEN, p.money));
    t.pad_to(NAME_COL + 6);
    t.tinted(Tone::Vp, &format!("{} VP", p.victory_points));
    t.pad_to(NAME_COL + 14);
    t.tinted(Tone::Chrome, &format!("seat {}", p.seat.0));
    line(&t)
}

/// `Goods  1 Copper · 1 Silk · 1 Tea · 1 Seafood · 0 Imports`, each token in
/// its resource color.
///
/// Imported goods are the one token that dims at zero, which is the mocks' own
/// rule rather than an inconsistency inherited by accident: they are the
/// resource a player routinely holds none of, and both approved frames draw
/// `0 Imports` faint beside a colored `0 Copper`.
fn goods_line(p: &PlayerModel) -> String {
    let g = &p.goods;
    let mut t = indented();
    t.raw("Goods  ");
    for (i, (n, label, kind)) in [
        (g.copper, "Copper", GoodKind::Copper),
        (g.silk, "Silk", GoodKind::Silk),
        (g.tea, "Tea", GoodKind::Tea),
        (g.seafood, "Seafood", GoodKind::Seafood),
    ]
    .into_iter()
    .enumerate()
    {
        if i > 0 {
            sep(&mut t);
        }
        t.tinted(Tone::Good(kind), &format!("{n} {label}"));
    }
    sep(&mut t);
    let imports = plural(g.imports as usize, "Import");
    if g.imports == 0 {
        t.tinted(Tone::Dim, &imports);
    } else {
        t.tinted(Tone::Good(GoodKind::Imports), &imports);
    }
    line(&t)
}

/// `Assistants  8 in hand · 0 on board · 12 in warehouse · 3 locked`.
///
/// The `locked` tally appears only when pawns actually sit on goals or tracks,
/// and it exists so the printed numbers reconcile to the twenty pawns a player
/// owns: hand + board + warehouse alone invites the table-side conservation
/// check and then fails it.
fn assistants_line(p: &PlayerModel) -> String {
    let a = &p.assistants;
    let mut t = indented();
    t.raw("Assistants  ");
    zone_count(&mut t, u32::from(a.in_hand), "in hand");
    sep(&mut t).raw(&format!("{} on board", a.on_board));
    sep(&mut t).raw(&format!("{} in warehouse", a.in_warehouse));
    if a.locked() > 0 {
        sep(&mut t).raw(&format!("{} locked", a.locked()));
    }
    line(&t)
}

fn shophouses_line(p: &PlayerModel) -> String {
    let s = &p.shophouses;
    let mut t = indented();
    t.raw("Shophouses  ");
    zone_count(&mut t, u32::from(s.in_hand), "in hand");
    sep(&mut t).raw(&format!("{} in warehouse", s.in_warehouse));
    sep(&mut t).raw(&format!("{} built", s.built));
    line(&t)
}

fn trading_houses_line(p: &PlayerModel) -> String {
    let h = &p.trading_houses;
    let mut t = indented();
    t.raw("Trade houses  ");
    zone_count(&mut t, u32::from(h.in_hand), "in hand");
    sep(&mut t).raw(&format!("{} in warehouse", h.in_warehouse));
    sep(&mut t).raw(&format!("{} built", h.built));
    t.raw(" ")
        .tinted(Tone::Chrome, &format!("(next take {}{})", glyph::YEN, h.next_take_cost));
    line(&t)
}

/// A zone's leading count, emphasised only when there is something there.
/// The primary approved mock bolds `8 in hand` and `2 in hand` and leaves
/// `0 in hand` plain, which is the rule this reproduces: what is in hand is
/// what a player can spend this turn.
fn zone_count(t: &mut Twin, n: u32, label: &str) {
    let text = format!("{n} {label}");
    if n > 0 {
        t.tinted(Tone::Strong, &text);
    } else {
        t.raw(&text);
    }
}

/// One line per owned Technology, or a single dim `none yet`.
///
/// The SUMMARY is this module's own short table rather than
/// [`describe::tech_effect_text`], and the two are not competing sources for
/// one fact — they answer different questions at different widths.
/// `tech_effect_text` is the PURCHASE text the `labs` view prints, authored
/// under a rule that it carry every qualifier which would change a buying
/// decision, and it runs to 101 columns doing so. This is a REMINDER of a card
/// already owned, sized to sit inside a 74-column panel beside its name, with
/// the full text one `labs` away.
fn tech_lines(p: &PlayerModel) -> Vec<String> {
    if p.technologies.is_empty() {
        let mut t = indented();
        t.raw("Tech  ").tinted(Tone::Dim, "none yet");
        return vec![line(&t)];
    }
    p.technologies
        .iter()
        .map(|tech| {
            let mut t = indented();
            t.raw("Tech  ");
            match &tech.printed {
                Some(card) => {
                    t.tinted(Tone::Strong, &card.name);
                    t.raw(&format!(" {} ", glyph::SECTION_DASH));
                    t.tinted(Tone::Chrome, tech_summary(card.effect));
                }
                // An id the registry does not know (a corrupt record, or a
                // fixture on `Rules::empty_for_tests`). Show the id rather than
                // invent a name — the model takes the same posture.
                None => {
                    t.tinted(Tone::Strong, &format!("T{:02}", tech.id.0));
                }
            }
            line(&t)
        })
        .collect()
}

/// What an owned Technology does, in one panel-width line.
///
/// A TOTAL match over `TechEffect` — no wildcard arm, so a new card is a
/// compile error here rather than a silently blank line. Authored from the
/// rules sources the long form was ([`describe::tech_effect_text`]'s authoring
/// rule names them: rulebook pp. 15-16, the KB reference, and guidingpath's
/// decision log), then cut to fit.
///
/// **The budget is PER CARD**, and it is what
/// `every_technology_summary_fits_the_panel_beside_its_name` computes and
/// reports: a summary may spend whatever [`DASH_W`] has left once the indent,
/// the `Tech  ` label, ITS OWN card name and the dash are paid for — 43 columns
/// beside `Letterpress Printing`, 59 beside `Tram`. Charging every card the
/// longest name's cost is a bound that holds and a budget that lies, and three
/// of the lines below (Station, Postal System, Stagecoach) need the columns the
/// real budget gives them to carry a qualifier that changes how the card plays.
fn tech_summary(effect: TechEffect) -> &'static str {
    match effect {
        TechEffect::Tram => "teleport to any opponent-free area",
        TechEffect::Stagecoach => "through opponent Presidents free; stops too w/ Postal",
        TechEffect::PostalSystem => "stop on opponents, 1 Yen each (free w/ Stagecoach)",
        TechEffect::Station => "+1 Power for ALL there; teleport needs your pawn there",
        TechEffect::Gaslight => "+2 VP when you build a Shophouse",
        TechEffect::ElectricalLight => "+3 VP when you build a Trading House",
        TechEffect::MiningTechnology => "+1 Copper each time you gain Copper",
        TechEffect::SpinningMill => "once a turn: discard 1 Silk, gain 2 Yen",
        TechEffect::Ball => "+2 VP when you complete an Order",
        TechEffect::LanguageSchool => "+1 Yen when you complete an Order",
        TechEffect::PatentSystem => "+2 VP per Technology bought after it",
        TechEffect::LetterPressPrinting => "+1 Yen per Technology bought after it",
        TechEffect::Winery => "+3 VP on the Church action with a pawn",
        TechEffect::Brickyard => "+4 VP on the Customs action with a pawn",
        TechEffect::SystemOfStock => "turn start: up to 2 Yen if you hold 0-1",
        TechEffect::University => "once a turn: 1 Yen takes a warehouse pawn",
        TechEffect::Newspaper => "Placement A may use 4 areas, not 3",
        TechEffect::Telegram => "after 2 in an area, may place 1 adjacent; fee applies",
        TechEffect::Telephone => "Recovery: may leave 1 pawn in the area",
        TechEffect::Exposition => "+10 VP, taken when acquired",
    }
}

/// `Mercenaries  1 unused · 0 used`. Both numbers always, because the unused
/// ones are worth 1 VP each at the end and the used ones are not — a single
/// total would hide the only distinction that scores.
fn mercenaries_line(p: &PlayerModel) -> String {
    let mut t = indented();
    t.raw("Mercenaries  ");
    zone_count(&mut t, u32::from(p.mercenaries_unused), "unused");
    sep(&mut t).raw(&format!("{} used", p.mercenaries_used));
    line(&t)
}

/// `Orders  2 in hand · 3 filled (O23, O08, O12)`.
///
/// **Its own line since branch-4, and the in-hand count MOVED here** from
/// [`president_line`], which now states a location and nothing else: one fact,
/// one home. The count is emphasised at nonzero like every other in-hand zone
/// ([`zone_count`]) because orders in hand are exactly what a seat can fill
/// this turn, and `(hidden)` still DECLARES the mask beside it rather than
/// being it — [`HandPanel`] is the only panel that carries card identities from
/// a hand.
///
/// **The filled clause is public on every seat.** Fulfilling an order turns the
/// card face up in front of its player, so a masked bot seat's filled codes are
/// as public as its VP; the mask covers the HAND, and the two halves of this
/// line are on opposite sides of that line deliberately.
///
/// The codes are the Vec's own order — the CHRONOLOGY of a seat's game, which
/// is the reading that carries information (spec §2.5, decision 3) — and they
/// elide from the END when they would outrun [`DASH_W`]; see
/// [`append_filled_codes`].
fn orders_line(p: &PlayerModel) -> String {
    let mut t = indented();
    t.raw("Orders  ");
    zone_count(&mut t, p.orders_in_hand as u32, "in hand");
    if p.hand_hidden {
        t.raw(" ").tinted(Tone::Chrome, "(hidden)");
    }
    sep(&mut t).raw(&format!("{} filled", p.orders_filled.len()));
    append_filled_codes(&mut t, &p.orders_filled, DASH_W);
    line(&t)
}

/// ` (O23, O08, O12)` appended to `t`, or ` (O23, O08, …)` when the whole list
/// would not fit `budget`.
///
/// **The elision is SPECIFIED rather than discovered** (spec §5 decision 10).
/// [`line`] would otherwise clip this line at [`DASH_W`] and take the closing
/// paren with it, leaving a code sheared mid-token — and a seat can legally
/// hold enough filled orders to reach that. So:
///
/// - codes drop from the END, the earliest ones being the ones a player is
///   least able to reconstruct from the table;
/// - the paren ALWAYS closes, so an elided tail never reads as a clipped one;
/// - the `N filled` COUNT stays exact — it is drawn before this is called and
///   nothing here can move it.
///
/// The mark is the vocabulary's own [`glyph::ELLIPSIS`], the same one every
/// other elided surface in the crate draws.
///
/// `budget` is a parameter rather than the constant so the pin can state the
/// arithmetic at a width it chooses; the one caller passes [`DASH_W`].
fn append_filled_codes(t: &mut Twin, filled: &[OrderCardId], budget: usize) {
    if filled.is_empty() {
        return;
    }
    let codes: Vec<String> = filled.iter().map(|id| format!("O{:02}", id.0)).collect();
    let available = budget.saturating_sub(t.width());

    // ` (` + the codes joined by `, ` + `)`.
    let full = 3 + codes.iter().map(|c| c.chars().count()).sum::<usize>() + 2 * (codes.len() - 1);
    if full <= available {
        t.tinted(Tone::Chrome, &format!(" ({})", codes.join(", ")));
        return;
    }

    // ` (…)` is 4 columns, and each code kept in front of the mark costs its
    // own width plus the `, ` that joins it.
    let mut cost = 4;
    let mut shown = 0usize;
    for c in &codes {
        let add = c.chars().count() + 2;
        if cost + add > available {
            break;
        }
        cost += add;
        shown += 1;
    }
    if shown == 0 {
        // Not even the mark fits: say nothing rather than open a paren this
        // function cannot close. Unreachable from any legal state (the widest
        // legal prefix leaves 33 columns and the mark needs 4), and the pin
        // sweeps that domain -- this is the defence that keeps the guarantee
        // above true of every caller rather than of one.
        if available >= 4 {
            t.tinted(Tone::Chrome, &format!(" ({})", glyph::ELLIPSIS));
        }
        return;
    }
    t.tinted(
        Tone::Chrome,
        &format!(" ({}, {})", codes[..shown].join(", "), glyph::ELLIPSIS),
    );
}

/// `President ▲ on SILK MILL`.
///
/// The area is NAMED, not numbered: the board already answers "which tile is
/// A06", and this line answers "where is my President" for a player reading the
/// panel rather than scanning the board.
///
/// **Location only since branch-4.** It used to carry ` · 1 order in hand
/// (hidden)` as a tail; that fact now has [`orders_line`] to itself, beside the
/// filled codes it belongs with.
fn president_line(p: &PlayerModel, board: &BoardModel, colors: &[PlayerColor]) -> String {
    let mut t = indented();
    t.raw("President ");
    match p.president {
        PresidentLocation::InHand => {
            t.tinted(Tone::Dim, "in hand");
        }
        PresidentLocation::OnBoard(area) => {
            t.owned(colors, p.seat, &glyph::PRESIDENT.to_string());
            t.raw(&format!(" on {}", area_name(board, area)));
        }
    }
    line(&t)
}

// ---------------------------------------------------------------------------
// Tracks
// ---------------------------------------------------------------------------

/// Both management boards, one row each: every space in printed order, who
/// stands there, how close the track is to ending the game, and what the
/// spaces pay.
///
/// **The fill counts are REAL pawns** (`church_filled` / `customs_filled`), so
/// the 2-player dummy is drawn on its space and excluded from the count — the
/// end-game trigger counts players' pawns and the dummy never moves. That is
/// why its space is labelled rather than merely shaded, and why the section
/// rule says so wherever a dummy is on the board.
fn tracks_lines(t: &TracksPanel, colors: &[PlayerColor]) -> Vec<String> {
    let has_dummy = t
        .church
        .iter()
        .filter_map(|s| s.occupant)
        .chain(t.customs.iter().filter_map(|s| s.occupant))
        .any(|o| o.is_dummy());

    let mut title = format!("TRACKS  (ends at {} of YOUR pawns", t.trigger_at);
    if has_dummy {
        title.push_str(&format!(" {} D = dummy, not counted", glyph::SEPARATOR));
    }
    title.push(')');

    let mut church = indented();
    church.raw("Church   faith ");
    for space in &t.church {
        track_space(&mut church, &space.faith.to_string(), space.occupant, colors);
    }
    church.raw(&format!(" {}/{}", t.church_filled, t.trigger_at));
    sep(&mut church).tinted(Tone::Vp, &vp_range(t.church.iter().map(|s| s.vp)));
    if t.church.iter().any(|s| s.movement_rider > 0) {
        church.tinted(Tone::Vp, "+mv");
    }

    let mut customs = indented();
    customs.raw("Customs  pay   ");
    for space in &t.customs {
        track_space(&mut customs, &space.import_cost.to_string(), space.occupant, colors);
    }
    customs.raw(&format!(" imports {}/{}", t.customs_filled, t.trigger_at));
    sep(&mut customs).tinted(Tone::Vp, &vp_range(t.customs.iter().map(|s| s.vp)));

    let supply = t.mercenary_supply.english
        + t.mercenary_supply.american
        + t.mercenary_supply.french
        + t.mercenary_supply.german
        + t.mercenary_supply.dutch;
    let mut tail = indented();
    tail.tinted(
        Tone::Chrome,
        &format!(
            "Mercenary supply: {supply} left {} ('tracks' renders every space)",
            glyph::SEPARATOR,
        ),
    );

    vec![header(&title), line(&church), line(&customs), line(&tail)]
}

/// One track space: `[4]` empty, `[5●]` occupied in its owner's color, `[7 D]`
/// for the neutral 2-player marker in reverse video.
fn track_space(t: &mut Twin, label: &str, occupant: Option<PlayerId>, colors: &[PlayerColor]) {
    match occupant {
        Some(seat) if seat.is_dummy() => {
            t.tinted(Tone::Highlight, &format!("[{label} D]"));
        }
        Some(seat) => {
            t.owned(colors, seat, &format!("[{label}{}]", glyph::ASSISTANT));
        }
        None => {
            t.tinted(Tone::Dim, &format!("[{label}]"));
        }
    }
}

/// `★3–14` — what the cheapest and dearest space on a track pay. Computed from
/// the spaces the model carries rather than stated, so a rules correction to a
/// printed reward moves this line with it.
fn vp_range(vps: impl Iterator<Item = u8>) -> String {
    let mut lo = u8::MAX;
    let mut hi = 0u8;
    for v in vps {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if lo > hi {
        return format!("{}{}", glyph::VP, glyph::SECTION_DASH);
    }
    if lo == hi {
        return format!("{}{lo}", glyph::VP);
    }
    format!("{}{lo}{}{hi}", glyph::VP, glyph::RANGE_DASH)
}

// ---------------------------------------------------------------------------
// Achievements
// ---------------------------------------------------------------------------

/// The three cards in play, one per category, with what each asks, what it pays
/// first and later, and who has taken it.
fn achievements_lines(panel: &AchievementsPanel, colors: &[PlayerColor]) -> Vec<String> {
    let mut out = vec![header(&format!(
        "ACHIEVEMENTS  (one per category {} first / later VP)",
        glyph::SECTION_DASH,
    ))];
    for card in &panel.cards {
        out.push(achievement_line(card, colors));
    }
    out
}

fn achievement_line(card: &AchievementModel, colors: &[PlayerColor]) -> String {
    let mut t = indented();
    t.tinted(Tone::Strong, describe::category_letter(card.category));
    t.raw(&format!(" {} ", glyph::SEPARATOR));
    t.raw(achievement_requirement(card.achievement_type));
    t.pad_to(INDENT + 4 + GOAL_TEXT_W);
    t.tinted(Tone::Vp, &format!("{} / {} VP", card.first_place_vp, card.subsequent_vp));
    t.pad_to(INDENT + 4 + GOAL_TEXT_W + GOAL_VP_W);

    match card.claimed_by.len() {
        0 => {
            t.tinted(Tone::Dim, "unclaimed");
        }
        // Two names still read as names. Beyond that they stop fitting beside
        // the columns before them, and a count is more honest than a list this
        // line would clip mid-word.
        n if n <= 2 => {
            t.raw("claimed by ");
            for (i, &seat) in card.claimed_by.iter().enumerate() {
                if i > 0 {
                    t.raw(", ");
                }
                match color_of(colors, seat) {
                    Some(c) => t.tinted(Tone::Player(c), color_name(c)),
                    None => t.raw(&format!("seat {}", seat.0)),
                };
            }
        }
        n => {
            t.raw(&format!("claimed by {n} players"));
        }
    }
    line(&t)
}

/// An Achievement's condition in at most [`GOAL_TEXT_W`] - 1 columns.
///
/// A TOTAL match, for the same reason [`tech_summary`] is one. It is the panel
/// twin of [`describe::achievement_desc`], which stays the `goals` view's
/// wording: that one runs to 46 columns on the mixed-category cards, which is
/// past what this line can spend before its VP column.
fn achievement_requirement(a: AchievementType) -> &'static str {
    match a {
        AchievementType::Copper5 => "own 5 copper",
        AchievementType::Silk6 => "own 6 silk",
        AchievementType::Tea7 => "own 7 tea",
        AchievementType::Seafood7 => "own 7 seafood",
        AchievementType::Money10 => "own 10 yen",
        AchievementType::Tokens5 => "own 5 tokens (5-Power + merc)",
        AchievementType::Orders5 => "complete 5 orders",
        AchievementType::Technologies4 => "own 4 technologies",
        AchievementType::Production4 => "buildings in 4 production",
        AchievementType::Commercial4 => "buildings in 4 commercial",
        AchievementType::Prod3Comm2 => "buildings in 3 prod + 2 comm",
        AchievementType::Prod2Comm3 => "buildings in 2 prod + 3 comm",
    }
}

// ---------------------------------------------------------------------------
// Hand
// ---------------------------------------------------------------------------

/// The viewing seat's own Order cards, drawn as cards.
///
/// A masked panel prints its COUNT and nothing else, and it can do no more than
/// that by construction: [`HandPanel`] empties its own card list when hidden,
/// so this renderer has nothing to leak even if it forgot to check.
fn hand_lines(panel: &HandPanel) -> Vec<String> {
    let mut out = vec![header(&format!(
        "YOUR HAND  ('hand' expands {} 'privacy' hides)",
        glyph::SEPARATOR,
    ))];

    if panel.hidden {
        let mut t = indented();
        t.tinted(
            Tone::Dim,
            &format!("{} {} hidden", plural(panel.card_count, "card"), glyph::SECTION_DASH),
        );
        out.push(line(&t));
        return out;
    }
    if panel.cards.is_empty() {
        let mut t = indented();
        t.tinted(Tone::Dim, "no orders in hand");
        out.push(line(&t));
        return out;
    }

    let shown = panel.cards.len().min(HAND_CARDS_SHOWN);
    for row in panel.cards[..shown].chunks(HAND_CARDS_PER_ROW) {
        let cards: Vec<Vec<String>> = row.iter().map(order_card).collect();
        for i in 0..CARD_H {
            let mut text = " ".repeat(INDENT);
            for (j, card) in cards.iter().enumerate() {
                if j > 0 {
                    text.push_str(&" ".repeat(HAND_CARD_GAP));
                }
                text.push_str(&card[i]);
            }
            out.push(text);
        }
    }
    if panel.cards.len() > shown {
        let mut t = indented();
        t.tinted(
            Tone::Chrome,
            &format!("+{} more {} 'hand' expands", panel.cards.len() - shown, glyph::SECTION_DASH),
        );
        out.push(line(&t));
    }
    out
}

/// One hand card, through the crate's ONE card emitter
/// ([`crate::card_render::order_card`]).
///
/// The last interior line carries the starting-order note when there is one and
/// the rewards' overflow otherwise — the mocks' own rule, now enforced in the
/// emitter for both of its callers. A card that both pays more than one line of
/// rewards AND is the kept starting deal therefore shows the note; `hand` is
/// the surface that never elides.
fn order_card(card: &HandCardModel) -> Vec<String> {
    card_render::order_card(
        &CardFace::new(card.id, card.printed.as_ref())
            .note(card.is_starting_order.then(|| "your starting order".to_string())),
    )
}

// ---------------------------------------------------------------------------
// The below arrangement (3 and 4 players)
// ---------------------------------------------------------------------------

/// The designed per-seat COLUMN stride when the dashboard stacks BELOW the
/// board (spec §0.1's 3P/4P ruling; design authority is the approved
/// `frame-4p.ans`, whose player columns start at 0, 49, 98 and 147).
///
/// **50 rather than the mock's 49, because 50 lands the four-player block
/// exactly on the 200-column design floor** — four columns of 50 is 200, which
/// is the width that mock's own frame is drawn at (its closing rule measures
/// 200). The number is a STRIDE, not a content width: [`SEAT_COL_GAP`] of it is
/// the gutter, so a column writes 48 columns and the mock writes 43 into 49.
///
/// It is a FLOOR on the stride, not a cap — [`crate::screen`] hands this module
/// a block at least as wide as the board and legend already above it AND at
/// least [`BELOW_BLOCK_MIN`], so at 3 players the 175-column block gives
/// columns of 58 rather than 50. What the
/// constant guarantees is the other direction: four columns of a 180-column
/// board would be 45 wide, and the seat header alone spends 46 of them.
pub const SEAT_COL_W: usize = 50;

/// Columns between two seat columns. Two, matching [`HAND_CARD_GAP`]: the
/// columns are ragged-right text rather than boxes, so the gutter only has to
/// stop the longest line of one column touching the shortest of the next.
const SEAT_COL_GAP: usize = 2;

/// The NARROWEST block the below arrangement may be handed — the constant that
/// widened the 3-player frame off the site-slot legend's 163 columns at the T10
/// review.
///
/// **What it protects is the three COMPRESSED panels, not the seat columns.**
/// [`tracks_row`], [`achievements_row`] and [`hand_row`] each fold a whole
/// panel onto one ragged row, and a row longer than its block does not overrun
/// the frame — [`line_at`] clips it, which costs the row's TAIL. On the
/// achievements row that tail is the claim state, so a clipped row quietly
/// stops saying who has taken each card, which is the one fact on it that moves
/// during a game. A clip is the safe failure, not an acceptable one.
///
/// **MEASURED, then set**, and swept rather than argued — which is the whole
/// reason this constant exists. T10 shipped a 162-column figure taken from a
/// fixture that hard-coded seats 0 and 1, i.e. `RED, BLUE`, the NARROWEST
/// claimant pair there is; the real maxima are wider, and at 163 columns 580 of
/// the 32,768 legal 3-player achievements states clipped:
///
/// | row | 3P legal max | 4P legal max | swept over |
/// |---|---|---|---|
/// | achievements | **171** | **174** | all 64 card triples x every claimant subset of every card (32,768 states at 3P, 262,144 at 4P) |
/// | tracks | **170** | **170** | 0..15 pawns graduated across both boards, up to every space occupied |
///
/// 175 is the widest of the four plus one column of margin. It BINDS at 3
/// players only: at 4 the block is already 200 (four columns of
/// [`SEAT_COL_W`]), which is why the 4-player maxima are the wider pair and
/// still never come near their block.
///
/// One constant rather than one per count, deliberately. The two counts differ
/// only in which seat colors can appear in a claim state (`BLUE, YELLOW` at 3
/// against `YELLOW, GREEN` at 4), which is not a layout fact worth a table —
/// and a minimum below some count's legal maximum would be a number nobody
/// could read without first asking which count it was for.
pub const BELOW_BLOCK_MIN: usize = 175;

/// Rows a seat column spends, whatever the seat owns — and the reason the 3P
/// and 4P frames stopped growing in play.
///
/// **The beside arrangement is ragged and the below arrangement cannot be.**
/// Beside the board a seat's block runs one line per owned Technology and the
/// dashboard simply gets longer; stacked below it, four seats sit SIDE BY SIDE,
/// so a ragged block would leave three columns short and the frame's height
/// would be the tallest seat's. Fixing the count makes the below dashboard a
/// constant-height block: the 3P and 4P frames now open and stay at the same
/// height for the whole game, and the only thing that can move them is the
/// decision zone's own twelve-row page (`decision_render::OPTION_ROWS`).
///
/// Ten is what the fields cost: the mock's seven (header, two goods rows, the
/// three zone rows, President) plus Technologies, Mercenaries, and the tail row
/// that carries the four facts no zone row has the columns for.
const SEAT_ROWS: usize = 10;

/// Column the seat header's money field starts at, and its two successors —
/// [`NAME_COL`]'s below-arrangement twin, sized to the same longest label
/// (`▶ YELLOW — your turn [bot]`, 26 columns) inside a 48-column budget rather
/// than a 74-column one. `seat 3` therefore ends at column 46 and the seat
/// number survives the arrangement (T3 kept it as the `reveal <seat>` key).
const SEAT_MONEY_COL: usize = 27;
const SEAT_VP_COL: usize = 33;
const SEAT_NUMBER_COL: usize = 40;

/// The whole dashboard, stacked BELOW the board across `width` columns.
///
/// [`dashboard`]'s twin, and deliberately a SECOND function rather than a
/// parameter on the first: the two arrangements share their models and none of
/// their layout. Beside the board a panel is a 74-column strip stacked
/// vertically; below it the players panel is one column per seat and the other
/// three are single titled rows, because 20 rows of board leave the design
/// floor's 50 with nothing like the 45 rows the vertical form wants.
///
/// The blank-line rule: **a block of more than one row is followed by one.**
/// Only the players block is multi-row, so the three compressed panels stack
/// straight onto each other, each being its own titled rule — the approved 4P
/// mock's own shape.
pub fn dashboard_below(
    panels: &[Panel],
    board: &BoardModel,
    colors: &[PlayerColor],
    width: usize,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    for panel in panels {
        let block = below_panel_lines(panel, board, colors, width);
        let multi = block.len() > 1;
        lines.extend(block);
        if multi {
            lines.push(String::new());
        }
    }
    let mut out = String::new();
    for l in lines {
        out.push_str(&l);
        out.push('\n');
    }
    out
}

fn below_panel_lines(
    panel: &Panel,
    board: &BoardModel,
    colors: &[PlayerColor],
    width: usize,
) -> Vec<String> {
    match panel {
        Panel::Players(p) => players_columns(p, board, colors, width),
        Panel::Tracks(t) => vec![tracks_row(t, colors, width)],
        Panel::Achievements(a) => vec![achievements_row(a, colors, width)],
        Panel::Hand(h) => vec![hand_row(h, width)],
    }
}

/// The players panel as one column per seat, in seat order, under a rule that
/// spans the block.
fn players_columns(
    panel: &PlayersPanel,
    board: &BoardModel,
    colors: &[PlayerColor],
    width: usize,
) -> Vec<String> {
    let seats = panel.players.len().max(1);
    // Never narrower than a seat column has to be: a block that cannot hold
    // `seats` columns of the designed stride is a caller error, and the safe
    // direction is a block that overruns its width rather than columns that
    // overwrite each other.
    //
    // KNOWN-EQUIVALENT MUTANT (T10 review N3, recorded so it is not re-derived):
    // dropping the `.max(SEAT_COL_W)` changes nothing at either shipped
    // geometry — 175 / 3 is 58 and 200 / 4 is exactly 50 — so no test can
    // distinguish it today. It stays as the defence it is written to be: a
    // caller that hands this a narrower block gets an overrunning row rather
    // than seat columns written over each other.
    let col_w = (width / seats).max(SEAT_COL_W);
    let content_w = col_w - SEAT_COL_GAP;
    let columns: Vec<Vec<String>> =
        panel.players.iter().map(|p| seat_column(p, board, colors, content_w)).collect();

    let mut out = vec![header_at("PLAYERS", width)];
    for row in 0..SEAT_ROWS {
        let mut text = String::new();
        for column in &columns {
            let cell = &column[row];
            text.push_str(cell);
            // Every cell went through `line_at(_, content_w)`, so the pad is at
            // least SEAT_COL_GAP and this subtraction cannot wrap.
            text.push_str(&" ".repeat(col_w - crate::screen::visible_width(cell)));
        }
        while text.ends_with(' ') {
            text.pop();
        }
        out.push(text);
    }
    out
}

/// One seat's column: exactly [`SEAT_ROWS`] lines, each at most `w` columns.
///
/// The wording is the approved 4P mock's compressed set (`8 hand · 0 board · 12
/// warehouse`), not [`players_lines`]'s long one (`8 in hand · 0 on board · 12
/// in warehouse`) — and the two mocks differ the same way, which is what makes
/// this a per-arrangement design rather than an inconsistency: `main-frame.ans`
/// draws the long form beside a 120-column board and `frame-4p.ans` draws the
/// short one under a 180-column one. The long form is 53 columns and a column
/// is 48.
fn seat_column(
    p: &PlayerModel,
    board: &BoardModel,
    colors: &[PlayerColor],
    w: usize,
) -> Vec<String> {
    let (goods_first, goods_rest) = goods_rows(p, w);
    let rows = vec![
        seat_header(p, colors, w),
        goods_first,
        goods_rest,
        assistants_row(p, w),
        shophouses_row(p, w),
        trading_houses_row(p, w),
        tech_row(p, w),
        mercenaries_row(p, w),
        president_row(p, board, colors, w),
        seat_tail_row(p, w),
    ];
    debug_assert_eq!(rows.len(), SEAT_ROWS, "a seat column is a fixed-height block");
    rows
}

/// `▶ RED — your turn [bot]    ¥3   0 VP  seat 0`.
fn seat_header(p: &PlayerModel, colors: &[PlayerColor], w: usize) -> String {
    let mut label = String::new();
    if p.is_current {
        label.push(glyph::ACTIVE);
        label.push(' ');
    } else {
        label.push_str("  ");
    }
    label.push_str(color_name(p.color));
    if p.is_current {
        label.push_str(" — your turn");
    }
    if p.is_bot {
        label.push_str(BOT_BADGE);
    }

    let mut t = Twin::default();
    match color_of(colors, p.seat) {
        Some(c) => t.tinted(Tone::Player(c), &label),
        None => t.raw(&label),
    };
    t.pad_to(SEAT_MONEY_COL);
    t.tinted(Tone::Good(GoodKind::Yen), &format!("{}{}", glyph::YEN, p.money));
    t.pad_to(SEAT_VP_COL);
    t.tinted(Tone::Vp, &format!("{} VP", p.victory_points));
    t.pad_to(SEAT_NUMBER_COL);
    t.tinted(Tone::Chrome, &format!("seat {}", p.seat.0));
    line_at(&t, w)
}

/// The five goods over TWO rows, the second aligned under the first good — the
/// approved 4P mock's own split, and what the beside panel's one-line form
/// costs at 57 columns against a 48-column budget.
fn goods_rows(p: &PlayerModel, w: usize) -> (String, String) {
    let g = &p.goods;
    let label = "Goods  ";
    let mut first = indented();
    first.raw(label);
    for (i, (n, name, kind)) in [
        (g.copper, "Copper", GoodKind::Copper),
        (g.silk, "Silk", GoodKind::Silk),
        (g.tea, "Tea", GoodKind::Tea),
    ]
    .into_iter()
    .enumerate()
    {
        if i > 0 {
            sep(&mut first);
        }
        first.tinted(Tone::Good(kind), &format!("{n} {name}"));
    }

    let mut rest = Twin::default();
    rest.raw(&" ".repeat(INDENT + label.len()));
    rest.tinted(Tone::Good(GoodKind::Seafood), &format!("{} Seafood", g.seafood));
    sep(&mut rest);
    // Imports dim at zero, the mocks' own rule (see `goods_line`).
    let imports = plural(g.imports as usize, "Import");
    if g.imports == 0 {
        rest.tinted(Tone::Dim, &imports);
    } else {
        rest.tinted(Tone::Good(GoodKind::Imports), &imports);
    }
    (line_at(&first, w), line_at(&rest, w))
}

/// `Assistants  8 hand · 0 board · 12 warehouse`. The locked tally rides
/// [`seat_tail_row`] rather than this line: a fourth zone here costs 51 columns
/// against 48, and clipping the very field that exists to make the numbers
/// reconcile would be worse than moving it.
fn assistants_row(p: &PlayerModel, w: usize) -> String {
    let a = &p.assistants;
    let mut t = indented();
    t.raw("Assistants  ");
    zone_count(&mut t, u32::from(a.in_hand), "hand");
    sep(&mut t).raw(&format!("{} board", a.on_board));
    sep(&mut t).raw(&format!("{} warehouse", a.in_warehouse));
    line_at(&t, w)
}

fn shophouses_row(p: &PlayerModel, w: usize) -> String {
    let s = &p.shophouses;
    let mut t = indented();
    t.raw("Shophouses  ");
    zone_count(&mut t, u32::from(s.in_hand), "hand");
    sep(&mut t).raw(&format!("{} warehouse", s.in_warehouse));
    sep(&mut t).raw(&format!("{} built", s.built));
    line_at(&t, w)
}

fn trading_houses_row(p: &PlayerModel, w: usize) -> String {
    let h = &p.trading_houses;
    let mut t = indented();
    t.raw("Trade houses  ");
    zone_count(&mut t, u32::from(h.in_hand), "hand");
    sep(&mut t).raw(&format!("{} warehouse", h.in_warehouse));
    sep(&mut t).raw(&format!("{} built", h.built));
    line_at(&t, w)
}

/// Owned Technology NAMES on one line, `+N more` for whatever does not fit.
///
/// The beside arrangement gives each card its own line and a summary of what it
/// does; a 48-column column cannot — the shortest summary in
/// [`tech_summary`] needs 34 columns beside a name and the label. So the below
/// arrangement keeps the FACT (which cards this seat owns, which is what an
/// opponent reads a dashboard for) and leaves the effects to `labs` and the
/// 2-player panel. The mock draws no tech line at all at 4 players.
fn tech_row(p: &PlayerModel, w: usize) -> String {
    let mut t = indented();
    t.raw("Tech  ");
    if p.technologies.is_empty() {
        t.tinted(Tone::Dim, "none yet");
        return line_at(&t, w);
    }
    let names: Vec<String> = p
        .technologies
        .iter()
        .map(|tech| match &tech.printed {
            Some(card) => card.name.clone(),
            None => format!("T{:02}", tech.id.0),
        })
        .collect();

    // Fit as many whole names as the column holds, then count the rest. A
    // clipped name identifies nothing, and `+N more` is the idiom the hand
    // panel already uses for the same problem.
    let mut shown = 0usize;
    for (i, name) in names.iter().enumerate() {
        let sep_cost = if i > 0 { 3 } else { 0 };
        let tail_cost = if i + 1 < names.len() {
            // Room for `· +N more` after this name, or this is the last one it
            // can carry.
            3 + format!("+{} more", names.len() - i - 1).chars().count()
        } else {
            0
        };
        if t.width() + sep_cost + name.chars().count() + tail_cost > w {
            break;
        }
        if i > 0 {
            sep(&mut t);
        }
        t.tinted(Tone::Strong, name);
        shown += 1;
    }
    if shown == 0 {
        // A single name too long for the column: say how many there are rather
        // than draw a clipped one.
        t.tinted(Tone::Chrome, &format!("{} owned", names.len()));
    } else if shown < names.len() {
        sep(&mut t).tinted(Tone::Chrome, &format!("+{} more", names.len() - shown));
    }
    line_at(&t, w)
}

fn mercenaries_row(p: &PlayerModel, w: usize) -> String {
    let mut t = indented();
    t.raw("Mercenaries  ");
    zone_count(&mut t, u32::from(p.mercenaries_unused), "unused");
    sep(&mut t).raw(&format!("{} used", p.mercenaries_used));
    line_at(&t, w)
}

/// `President ▲ EMPLOYMENT AGENCY`, or `President in hand`. The beside form's
/// ` on ` is dropped and the orders count moves to [`seat_tail_row`]: the
/// longest area name is 17 columns and the full beside line runs to 53.
fn president_row(
    p: &PlayerModel,
    board: &BoardModel,
    colors: &[PlayerColor],
    w: usize,
) -> String {
    let mut t = indented();
    t.raw("President ");
    match p.president {
        PresidentLocation::InHand => {
            t.tinted(Tone::Dim, "in hand");
        }
        PresidentLocation::OnBoard(area) => {
            t.owned(colors, p.seat, &glyph::PRESIDENT.to_string());
            t.raw(&format!(" {}", area_name(board, area)));
        }
    }
    line_at(&t, w)
}

/// The FOUR facts no zone row has the columns for: the orders in hand (with
/// the mask declared, exactly as the beside panel declares it), the orders this
/// seat has FILLED, the assistants locked on goals and tracks, and the next
/// Employment-Agency take price.
///
/// A row rather than four tails, because all of them are short and none of them
/// fits where it belongs: `· 3 locked` costs 11 columns on a 40-column
/// assistants line, `(next take ¥4)` costs 15 on a 46-column trade-house line,
/// and the column is 48.
///
/// **The filled count is a COUNT here and the codes are the 2-player Orders
/// line's business.** [`orders_line`] names the cards because a 74-column strip
/// can hold them; this column is 48 and the shortest useful parenthetical
/// (`(O23)`) costs 6 on a row that is already over budget. That is the owner's
/// ask under-delivered at 3 and 4 players BY MEASUREMENT, and spec §2.5 records
/// it as such rather than calling it a design.
///
/// **THE ROW NO LONGER FITS ITS COLUMN ON ITS OWN, so it has a YIELD POLICY**
/// (spec §5 decision 9, branch-4). Over the constrained joint domain — 36 order
/// cards exist, so a seat's in-hand count plus its filled count cannot exceed 36
/// — the four-fact worst case measures **59 columns against 48**. Where the T11
/// verdict below found one column of margin over three facts, the fourth fact
/// spends it and 11 more, so something has to give and WHICH is a decision
/// rather than an accident:
///
/// - **`next take ¥N` drops FIRST and ALONE.** It is appended only when the
///   whole row still fits, so no other fact can be displaced by it.
/// - **The orders, filled, locked and mask facts never yield.** They are
///   state a player cannot recover from anywhere else on the screen; the
///   next-take price is recoverable at the Employment Agency's own decision
///   preview, and [`trading_houses_line`] prints it in full in the beside
///   arrangement.
/// - **One drop is always enough**, which is what makes the policy total rather
///   than a first step: the widest row WITHOUT the price measures 44 columns.
///   `the_widest_legal_seat_tail_row_still_fits_its_column` measures both halves
///   (48 keeping, 44 yielding) so a future word that made one drop insufficient
///   fails there instead of clipping.
///
/// The yield is NOT a rare corner: a masked seat with any locked pawn already
/// exceeds the budget at 4 players, so an opponent bot column routinely shows
/// three facts rather than four. That is the cost of the fourth fact, stated.
///
/// **`next take ¥N` was re-examined at T11 and KEPT, which is a verdict rather
/// than an omission** (the T10 review flagged it: reword it if you can do
/// better in the columns available, else keep it and say why).
///
/// The complaint is real — on a row reading `1 order · 3 locked · next take
/// ¥4` nothing says WHAT is being taken, and "take" is also what an assistant
/// and a shophouse leave the warehouse by. What settled it was the measurement:
/// the budget for a disambiguating word was ZERO columns, not one. Every
/// candidate costs more than that (`next trade house ¥4` is +7; `next house ¥4`
/// is +1 AND ambiguous between the two building kinds), and the abbreviations
/// that would fit are exactly what the owner's full-words ruling excludes.
/// **Branch-4 conditionally RE-OPENS that verdict** rather than settling it
/// again: a fact that now yields at the corner is a different question from a
/// fact that fits with one column to spare, and spec §5 decision 9 routes it to
/// the owner. Until they rule, the words stay and the two places that CAN carry
/// the context do — [`trading_houses_line`] prints `(next take ¥4)` on the
/// Trade houses line itself, where the noun is one row up in the beside
/// arrangement, and the guide says what the tail row's copy of it counts.
/// `next take` is the Employment Agency's own vocabulary for the printed
/// 4/5/6/7 progression, not an abbreviation of longer words.
fn seat_tail_row(p: &PlayerModel, w: usize) -> String {
    let mut t = indented();
    t.raw(&plural(p.orders_in_hand, "order"));
    if p.hand_hidden {
        t.raw(" ").tinted(Tone::Chrome, "(hidden)");
    }
    sep(&mut t).raw(&format!("{} filled", p.orders_filled.len()));
    let locked = p.assistants.locked();
    if locked > 0 {
        sep(&mut t).raw(&format!("{locked} locked"));
    }
    // The one fact allowed to yield, and it yields by never being written
    // rather than by being clipped: a truncated price is a WRONG price, where
    // an absent one sends a player to the Employment Agency preview that owns
    // the number.
    let take = format!("next take {}{}", glyph::YEN, p.trading_houses.next_take_cost);
    if t.width() + 3 + take.chars().count() <= w {
        sep(&mut t).tinted(Tone::Chrome, &take);
    }
    line_at(&t, w)
}

/// Both management boards on ONE titled row.
///
/// The beside arrangement spends four rows on this panel and the below one has
/// none to spare, so the spaces, the fills and the reward ranges pack onto the
/// rule itself — the approved 4P mock's own move (its line 36 is exactly this
/// shape). What is dropped against the beside form is the trailing
/// `'tracks' renders every space` hint, which the frame's info strip already
/// names on every screen.
///
/// **This row GROWS WITH PAWNS**, which is what the T10 review found it had not
/// been measured against: an occupied space draws `[7●]` where an empty one
/// draws `[7]`, so every pawn on either board costs a column and the tail —
/// the mercenary supply — is what a clip takes. Fully occupied (all 8 Church
/// spaces and all 7 Customs), the row measures **170 columns**, held by
/// `the_widest_legal_tracks_row_survives_its_block_at_both_counts` and inside
/// [`BELOW_BLOCK_MIN`] at both counts.
fn tracks_row(t: &TracksPanel, colors: &[PlayerColor], width: usize) -> String {
    let has_dummy = t
        .church
        .iter()
        .filter_map(|s| s.occupant)
        .chain(t.customs.iter().filter_map(|s| s.occupant))
        .any(|o| o.is_dummy());

    let mut body = Twin::default();
    body.raw("Church ");
    for space in &t.church {
        track_space(&mut body, &space.faith.to_string(), space.occupant, colors);
    }
    body.raw(&format!(" {}/{}", t.church_filled, t.trigger_at));
    body.raw(" ").tinted(Tone::Vp, &vp_range(t.church.iter().map(|s| s.vp)));
    if t.church.iter().any(|s| s.movement_rider > 0) {
        body.tinted(Tone::Vp, "+mv");
    }

    sep(&mut body).raw("Customs ");
    for space in &t.customs {
        track_space(&mut body, &space.import_cost.to_string(), space.occupant, colors);
    }
    body.raw(&format!(" imports {}/{}", t.customs_filled, t.trigger_at));
    body.raw(" ").tinted(Tone::Vp, &vp_range(t.customs.iter().map(|s| s.vp)));

    let supply = t.mercenary_supply.english
        + t.mercenary_supply.american
        + t.mercenary_supply.french
        + t.mercenary_supply.german
        + t.mercenary_supply.dutch;
    sep(&mut body).tinted(Tone::Chrome, &format!("{supply} mercenaries left"));
    if has_dummy {
        sep(&mut body).tinted(Tone::Chrome, "D = dummy, not counted");
    }

    header_row(&format!("TRACKS  (ends at {} of YOUR pawns)", t.trigger_at), &body, width)
}

/// The three cards in play on ONE titled row, each naming its category, what it
/// asks, what it pays first and later, and who has taken it.
///
/// Ragged rather than three fixed cells, and that is what makes it fit: the
/// three cards are one per CATEGORY, so the row can only ever carry one
/// requirement out of each deck.
///
/// **This is the WIDEST row the below arrangement draws, and the number is
/// swept rather than reasoned.** Every one of the 64 legal card triples, at
/// every claimant subset of every card, measures at most **171 columns at 3
/// players and 174 at 4** — the maxima [`BELOW_BLOCK_MIN`] is set from, held by
/// `the_widest_legal_achievements_row_survives_its_block_at_both_counts`. The
/// figure T10 shipped (161/162) was measured on a fixture that hard-coded seats
/// 0 and 1, i.e. the narrowest claimant pair in the game, and 580 of the 32,768
/// legal 3-player states were clipping at the block it licensed.
///
/// A state somehow wider than its block still clips rather than overruns —
/// [`line_at`] is the guarantee, and the arithmetic above is only what keeps
/// the guarantee from having to fire.
fn achievements_row(panel: &AchievementsPanel, colors: &[PlayerColor], width: usize) -> String {
    let mut body = Twin::default();
    for (i, card) in panel.cards.iter().enumerate() {
        if i > 0 {
            sep(&mut body);
        }
        body.tinted(Tone::Strong, describe::category_letter(card.category));
        body.raw(&format!(" {} ", glyph::SEPARATOR));
        body.raw(achievement_requirement(card.achievement_type));
        body.raw("  ");
        body.tinted(Tone::Vp, &format!("{}/{} VP", card.first_place_vp, card.subsequent_vp));
        body.raw("  ");
        claim_state(&mut body, card, colors);
    }
    header_row("ACHIEVEMENTS", &body, width)
}

/// Who has taken an Achievement, in the fewest columns that stay truthful: the
/// claimants by name up to two, a count past that, `unclaimed` for none.
fn claim_state(t: &mut Twin, card: &AchievementModel, colors: &[PlayerColor]) {
    match card.claimed_by.len() {
        0 => {
            t.tinted(Tone::Dim, "unclaimed");
        }
        n if n <= 2 => {
            for (i, &seat) in card.claimed_by.iter().enumerate() {
                if i > 0 {
                    t.raw(", ");
                }
                match color_of(colors, seat) {
                    Some(c) => t.tinted(Tone::Player(c), color_name(c)),
                    None => t.raw(&format!("seat {}", seat.0)),
                };
            }
        }
        n => {
            t.raw(&format!("{n} players"));
        }
    }
}

/// The viewing seat's own Order cards on ONE titled row, named rather than
/// drawn.
///
/// The beside arrangement draws them as cards, which costs [`CARD_H`] rows plus
/// a rule; below a 20-row board inside a 50-row floor there are not six rows to
/// spend, and `hand` renders the cards full-screen one word away. The approved
/// 4P mock draws no hand panel at all, so a named row is more than it asks for.
///
/// The mask is the model's, not this renderer's: a hidden [`HandPanel`] arrives
/// with an EMPTY card list, so the count below is the only thing there is to
/// print even if this function forgot to check `hidden`.
fn hand_row(panel: &HandPanel, width: usize) -> String {
    let mut body = Twin::default();
    if panel.hidden {
        body.tinted(
            Tone::Dim,
            &format!("{} {} hidden", plural(panel.card_count, "card"), glyph::SECTION_DASH),
        );
        return header_row("YOUR HAND", &body, width);
    }
    if panel.cards.is_empty() {
        body.tinted(Tone::Dim, "no orders in hand");
        return header_row("YOUR HAND", &body, width);
    }
    for (i, card) in panel.cards.iter().enumerate() {
        if i > 0 {
            sep(&mut body);
        }
        body.tinted(Tone::Strong, &format!("O{:02}", card.id.0));
        if let Some(printed) = &card.printed {
            body.raw(&format!(" {}", describe::country_name(printed.country)));
        }
    }
    sep(&mut body).tinted(Tone::Chrome, "'hand' expands");
    header_row("YOUR HAND", &body, width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::test_support::assert_exact_width;
    use crate::screen::visible_width;
    use crate::style::strip_ansi;
    use yokohama_core::*;

    fn state_np(n: u8) -> (GameState, Rules) {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(n), &rules, 7).expect("a setup");
        let state = finalize_setup(pending, &vec![StartingOrderChoice::Keep(0); n as usize])
            .expect("a finalized game");
        (state, rules)
    }

    fn colors_np(state: &GameState) -> Vec<PlayerColor> {
        state.players.iter().map(|p| p.color).collect()
    }

    /// Every panel, in frame order, for a fresh `n`-player game.
    fn panels_np(n: u8) -> (Vec<Panel>, BoardModel, Vec<PlayerColor>) {
        let (state, rules) = state_np(n);
        let seats = crate::screen_model::SeatFacts::all_open(state.players.len());
        let board = crate::screen_model::build_board(&state, &rules, &[]);
        // Through the model's own assembler, so these tests render the panel
        // set (and the ORDER) the shipped adapter renders, not a second list
        // that agrees with it today.
        let panels =
            crate::screen_model::build_dashboard(&state, &rules, &seats, PlayerId::new(0));
        (panels, board, colors_np(&state))
    }

    fn render_np(n: u8) -> String {
        let (panels, board, colors) = panels_np(n);
        dashboard(&panels, &board, &colors)
    }

    // -----------------------------------------------------------------------
    // Width discipline
    // -----------------------------------------------------------------------

    /// THE panel-width property, and the one a side-by-side join depends on:
    /// no dashboard line may exceed [`DASH_W`], at any player count. Both
    /// directions are NOT pinned here — a dashboard is deliberately ragged
    /// right (module doc), so short lines are the design.
    #[test]
    fn no_dashboard_line_exceeds_the_designed_width_at_any_count() {
        for n in [2u8, 3, 4] {
            for l in render_np(n).lines() {
                assert!(
                    visible_width(l) <= DASH_W,
                    "a {n}P dashboard line is {} columns, past DASH_W {DASH_W}: {:?}",
                    visible_width(l),
                    strip_ansi(l),
                );
            }
        }
    }

    /// The lines that draw a SHAPE are exact, because a rule one column short
    /// stops looking like a rule. Every section header, at every count.
    #[test]
    fn every_section_rule_spans_exactly_the_designed_width() {
        for n in [2u8, 3, 4] {
            let text = render_np(n);
            let rules: Vec<&str> =
                text.lines().filter(|l| strip_ansi(l).starts_with(glyph::RULE)).collect();
            assert_eq!(rules.len(), 4, "one rule per panel at {n}P:\n{text}");
            assert_exact_width(&rules, DASH_W);
        }
    }

    /// A content line clipped at the boundary keeps the ellipsis the crate's
    /// other fixed-width surfaces use, rather than shearing.
    #[test]
    fn a_content_line_past_the_boundary_is_clipped_not_sheared() {
        let mut t = Twin::default();
        t.raw(&"x".repeat(DASH_W + 10));
        let clipped = line(&t);
        assert_eq!(visible_width(&clipped), DASH_W);
        assert!(clipped.ends_with(glyph::ELLIPSIS), "{clipped}");
    }

    /// ...and a line inside the boundary is returned untouched: no padding, no
    /// trailing spaces to widen a frame nobody asked to widen.
    #[test]
    fn a_content_line_inside_the_boundary_is_not_padded() {
        let mut t = Twin::default();
        t.raw("short");
        assert_eq!(line(&t), "short");
    }

    /// Every mini order card is exactly [`CARD_W`] columns on every line, for
    /// every card in the registry — the card twin of T2's tile-width assert,
    /// and the same round-3.1 lesson (a card that overflows shears the panel it
    /// sits in).
    #[test]
    fn every_registry_order_card_renders_at_exactly_the_card_width() {
        let rules = Rules::standard();
        for printed in rules.orders.iter_sorted_by_id() {
            for starting in [false, true] {
                let card = HandCardModel {
                    id: printed.id,
                    printed: Some(printed.clone()),
                    is_starting_order: starting,
                };
                let lines = order_card(&card);
                assert_eq!(lines.len(), CARD_H, "card O{:02} line count", printed.id.0);
                assert_exact_width(&lines, CARD_W);
            }
        }
    }

    /// A card whose id the registry does not know still draws a box rather than
    /// panicking or inventing content — the same totality posture the model
    /// takes on the field it comes from.
    #[test]
    fn an_unknown_order_card_still_draws_a_card() {
        let card =
            HandCardModel { id: OrderCardId(250), printed: None, is_starting_order: false };
        let lines = order_card(&card);
        assert_exact_width(&lines, CARD_W);
        let text = lines.join("\n");
        assert!(strip_ansi(&text).contains("O250"), "{text}");
        assert!(strip_ansi(&text).contains("not in the registry"), "{text}");
    }

    // -----------------------------------------------------------------------
    // Content
    // -----------------------------------------------------------------------

    /// The players panel carries the whole set spec §0.3 names, and the
    /// current seat is the marked one.
    #[test]
    fn the_players_panel_shows_every_public_field_and_marks_the_current_seat() {
        let text = strip_ansi(&render_np(2));
        for want in [
            "PLAYERS",
            "Goods  ",
            "Assistants  ",
            "Shophouses  ",
            "Trade houses  ",
            "(next take ¥4)",
            "Orders  ",
            "Tech  ",
            "Mercenaries  ",
            "President ",
        ] {
            assert!(text.contains(want), "want {want:?} in the panel:\n{text}");
        }
        let red = text.lines().find(|l| l.contains("RED")).expect("a RED line");
        assert!(red.starts_with(&format!("{} RED", glyph::ACTIVE)), "current seat marked: {red}");
        let blue = text.lines().find(|l| l.contains("BLUE")).expect("a BLUE line");
        assert!(blue.starts_with("  BLUE"), "a waiting seat is unmarked: {blue}");
        assert!(red.contains("your turn"), "and says whose turn it is: {red}");
    }

    /// The money, VP and seat columns line up down the panel however long the
    /// name label is — which is the point of padding to a fixed column, and the
    /// property that keeps a bot badge from shifting a neighbour's numbers.
    #[test]
    fn the_money_and_vp_columns_align_across_seats_even_with_a_bot_badge() {
        let (state, rules) = state_np(4);
        let seats: Vec<crate::screen_model::SeatFacts> = (0..4)
            .map(|i| crate::screen_model::SeatFacts {
                // Badge alternate seats: the widest and narrowest labels then
                // sit next to each other.
                is_bot: i % 2 == 1,
                hand_hidden: false,
            })
            .collect();
        let panel = crate::screen_model::build_players(&state, &rules, &seats);
        let lines = players_lines(&panel, &crate::screen_model::build_board(&state, &rules, &[]), &colors_np(&state));

        // HEADER lines only. `¥` also appears in every seat's trade-house
        // line, so keying on the glyph alone finds two rows per seat.
        //
        // The column is a CHAR count, not `str::find`'s byte offset: the
        // current seat's label carries `▶` and `—` at three bytes each, so a
        // byte index reports it four columns to the right of where it renders
        // and this test would fail a panel that is correctly aligned.
        let columns: Vec<usize> = lines
            .iter()
            .map(|l| strip_ansi(l))
            .filter(|l| PlayerColor::ALL.iter().any(|&c| l.contains(color_name(c))))
            .map(|l| {
                l.chars().position(|c| c == glyph::YEN).expect("a header line carries money")
            })
            .collect();
        assert_eq!(columns.len(), 4, "one header line per seat");
        assert!(
            columns.iter().all(|&c| c == columns[0]),
            "the money column must not move between seats: {columns:?}",
        );
        assert!(
            lines.iter().any(|l| strip_ansi(l).contains("[bot]")),
            "the badge must actually render",
        );
    }

    /// One line per owned technology, each naming the card and what it does;
    /// a seat with none says so once.
    #[test]
    fn technologies_get_one_summary_line_each() {
        let (mut state, rules) = state_np(2);
        let ids: Vec<TechCardId> =
            rules.techs.iter_sorted_by_id().iter().take(3).map(|c| c.id).collect();
        state.players[0].technologies = ids.clone();
        let seats = crate::screen_model::SeatFacts::all_open(2);
        let panel = crate::screen_model::build_players(&state, &rules, &seats);
        let lines: Vec<String> =
            players_lines(&panel, &crate::screen_model::build_board(&state, &rules, &[]), &colors_np(&state))
                .iter()
                .map(|l| strip_ansi(l))
                .collect();

        let tech: Vec<&String> = lines.iter().filter(|l| l.trim_start().starts_with("Tech  ")).collect();
        // Three for seat 0, one `none yet` for seat 1.
        assert_eq!(tech.len(), 4, "{lines:#?}");
        for id in &ids {
            let card = rules.techs.get(*id).expect("a registry tech");
            assert!(
                tech.iter().any(|l| l.contains(&card.name)),
                "want {:?} named on its own line: {tech:#?}",
                card.name,
            );
        }
        assert!(tech.iter().any(|l| l.contains("none yet")), "{tech:#?}");
    }

    /// Every technology in the registry has a summary, and every summary fits
    /// the panel beside ITS OWN card name. Sweeps the REGISTRY, so a card whose
    /// name grows is caught here rather than on screen.
    ///
    /// **PER-CARD since the T3 review, and that is a correction rather than a
    /// relaxation.** The bound used to charge every summary the width of
    /// `Letterpress Printing` — 20 columns, the longest name in the set — which
    /// is a true bound but not the one [`tech_lines`] applies: it writes each
    /// summary beside its OWN name, so `Tram` has sixteen more columns to spend
    /// than the conservative form ever admitted. That gap was not free. Three
    /// summaries below carry a rules qualifier that only fits once the budget is
    /// the real one, and the conservative bound is why they shipped without it.
    /// What replaces it is the SAME arithmetic [`tech_lines`] lays out, per
    /// card, which is exactly where [`line`] would clip.
    ///
    /// Clipping is also why the sweep RENDERS. A summary one column over comes
    /// back from `line` clipped to exactly [`DASH_W`] with an ellipsis, so a
    /// width assert on rendered output passes on the very line it should catch;
    /// the rendered half therefore asserts the summary arrived INTACT, and the
    /// arithmetic half names the budget the author has to write inside.
    #[test]
    fn every_technology_summary_fits_the_panel_beside_its_name() {
        let rules = Rules::standard();
        for card in rules.techs.iter_sorted_by_id() {
            let summary = tech_summary(card.effect);
            assert!(!summary.is_empty(), "{} has no summary", card.name);
            // `  Tech  ` + THIS card's name + ` — ` + this summary.
            let width =
                INDENT + "Tech  ".len() + card.name.chars().count() + 3 + summary.chars().count();
            assert!(
                width <= DASH_W,
                "{:?}'s line runs to {width} columns, past DASH_W {DASH_W}. Its summary may \
                 spend {} columns and spends {}: {summary:?}",
                card.name,
                DASH_W - INDENT - "Tech  ".len() - card.name.chars().count() - 3,
                summary.chars().count(),
            );
        }

        // ...and the same sweep through the renderer, one card per distinct
        // effect, checking that nothing was clipped on the way out.
        let mut seen: std::collections::BTreeSet<String> = Default::default();
        let mut ids = Vec::new();
        for card in rules.techs.iter_sorted_by_id() {
            if seen.insert(format!("{:?}", card.effect)) {
                ids.push(card.id);
            }
        }
        let (mut state, _) = state_np(2);
        state.players[0].technologies = ids;
        let seats = crate::screen_model::SeatFacts::all_open(2);
        let panel = crate::screen_model::build_players(&state, &rules, &seats);
        let rendered: Vec<String> = players_lines(
            &panel,
            &crate::screen_model::build_board(&state, &rules, &[]),
            &colors_np(&state),
        )
        .iter()
        .map(|l| strip_ansi(l))
        .collect();
        for card in rules.techs.iter_sorted_by_id() {
            // Keyed on the whole `Tech  NAME —` label, not on the bare name: two
            // summaries name each other's card (Postal System and Stagecoach),
            // so a bare-name search finds the wrong row.
            let head = format!("Tech  {} {}", card.name, glyph::SECTION_DASH);
            let want = format!("{head} {}", tech_summary(card.effect));
            let row = rendered
                .iter()
                .find(|l| l.contains(&head))
                .unwrap_or_else(|| panic!("{:?} draws no line: {rendered:#?}", card.name));
            assert!(
                row.contains(&want),
                "{:?}'s summary arrived clipped or altered; want {want:?} in {row:?}",
                card.name,
            );
            assert!(
                visible_width(row) <= DASH_W,
                "{:?}'s rendered line is {} columns, past DASH_W {DASH_W}: {row:?}",
                card.name,
                visible_width(row),
            );
        }
    }

    /// The three summaries the T3 review sent back, plus the two it flagged as
    /// SHOULD and the one NOTE — each pinned on the rules fact it was missing,
    /// so a future re-cut for width cannot quietly drop the qualifier again.
    ///
    /// Every clause here is a rule the ENGINE implements, cited to the code that
    /// implements it, and the pins are on MEANING rather than on the exact
    /// wording: the assertions ask whether the fact survived, not whether the
    /// sentence is the one written today.
    #[test]
    fn tech_summaries_carry_the_qualifiers_that_change_how_a_card_plays() {
        // Station: the teleport exists only with one of YOUR OWN assistants on
        // the Station area (`movement::can_teleport_to_station_with_tech`), and
        // the +1 Power is for ALL players standing there, not for its owner.
        let station = tech_summary(TechEffect::Station);
        assert!(
            station.contains("ALL"),
            "Station's +1 Power is for every player in its area: {station:?}",
        );
        assert!(
            station.contains("pawn") || station.contains("assistant"),
            "Station's teleport needs one of your own pawns there: {station:?}",
        );

        // Textile Mill spends a Silk to gain Yen, and the direction is the whole
        // card (`tech_effects`'s `UseSpinningMill`).
        let mill = tech_summary(TechEffect::SpinningMill);
        assert!(
            mill.contains("discard 1 Silk") && mill.contains("gain 2 Yen"),
            "Textile Mill discards Silk to GAIN Yen; a summary that reads the \
             other way describes a card nobody would buy: {mill:?}",
        );

        // Stock Market tops up at 1 Yen OR LESS, so a broke player is the case
        // it most matters for. A summary naming only "1" excludes zero.
        let stock = tech_summary(TechEffect::SystemOfStock);
        assert!(
            stock.contains("0-1") || stock.contains("or less") || stock.contains("under 2"),
            "Stock Market refills at 1 Yen or less, zero included: {stock:?}",
        );

        // The Postal System / Stagecoach interaction runs both ways
        // (`movement::find_reachable_areas_with_tech` waives the president fee
        // outright under Stagecoach), so each card's line names the other.
        let postal = tech_summary(TechEffect::PostalSystem);
        assert!(
            postal.contains("Stagecoach"),
            "Postal System's per-owner fee is WAIVED under Stagecoach, which is \
             the difference between a card that costs Yen and one that does not: {postal:?}",
        );
        let stagecoach = tech_summary(TechEffect::Stagecoach);
        assert!(
            stagecoach.contains("Postal"),
            "Stagecoach makes Postal System's stops free too: {stagecoach:?}",
        );

        // Telegram's bonus placement is a DECLINABLE queued choice, not an
        // automatic one — the `may` is the difference.
        let telegram = tech_summary(TechEffect::Telegram);
        assert!(
            telegram.contains("may"),
            "Telegram's adjacent placement is a choice the player may decline: {telegram:?}",
        );
    }

    /// The tracks panel draws every printed space, marks the 2-player dummy
    /// distinctly, and counts only REAL pawns toward the end-game trigger
    /// (SP1 — the dummy never moves, so counting it would misreport how close
    /// the game is to ending).
    #[test]
    fn the_tracks_panel_marks_the_dummy_and_excludes_it_from_the_fill() {
        let (state, _) = state_np(2);
        let panel = crate::screen_model::build_tracks(&state);
        let text: String =
            tracks_lines(&panel, &colors_np(&state)).iter().map(|l| strip_ansi(l)).collect::<Vec<_>>().join("\n");

        assert!(text.contains("D = dummy, not counted"), "{text}");
        assert!(text.contains("[7 D]"), "the church dummy sits on faith 7: {text}");
        assert!(text.contains("[2 D]"), "the customs dummy sits on the space that costs 2: {text}");
        assert!(text.contains("0/4"), "a fresh 2P game has no REAL pawns on either track: {text}");
        // Every printed space appears.
        for faith in 4..=11u8 {
            assert!(text.contains(&format!("[{faith}")), "faith {faith} missing: {text}");
        }
    }

    /// A real pawn on a track is drawn in its owner's color with a pawn glyph,
    /// and it counts.
    #[test]
    fn a_real_pawn_on_a_track_is_marked_and_counted() {
        let (mut state, _) = state_np(2);
        state.church_track.slots[1] =
            Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 0 });
        let panel = crate::screen_model::build_tracks(&state);
        let text: String =
            tracks_lines(&panel, &colors_np(&state)).iter().map(|l| strip_ansi(l)).collect::<Vec<_>>().join("\n");
        assert!(text.contains(&format!("[5{}]", glyph::ASSISTANT)), "{text}");
        assert!(text.contains("1/4"), "a real pawn moves the fill: {text}");
    }

    /// At 3 and 4 players there is no dummy, so the panel does not explain one.
    #[test]
    fn the_tracks_panel_does_not_mention_a_dummy_that_is_not_there() {
        for n in [3u8, 4] {
            let text = strip_ansi(&render_np(n));
            assert!(!text.contains("dummy"), "{n}P has no dummy:\n{text}");
            assert!(!text.contains(" D]"), "{n}P draws no dummy marker:\n{text}");
        }
    }

    /// The VP range is COMPUTED from the spaces the model carries, so a rules
    /// correction moves the line with it rather than leaving a stale constant.
    #[test]
    fn the_track_reward_range_is_computed_from_the_printed_spaces() {
        assert_eq!(vp_range([3u8, 14, 7].into_iter()), format!("{}3{}14", glyph::VP, glyph::RANGE_DASH));
        assert_eq!(vp_range([5u8, 5].into_iter()), format!("{}5", glyph::VP));
        assert_eq!(vp_range(std::iter::empty()), format!("{}{}", glyph::VP, glyph::SECTION_DASH));
    }

    /// Every achievement in the deck has a requirement short enough for its
    /// column, and the three in play render with their VP tiers and claim
    /// state.
    #[test]
    fn every_achievement_requirement_fits_its_column() {
        for a in [
            AchievementType::Copper5,
            AchievementType::Silk6,
            AchievementType::Tea7,
            AchievementType::Seafood7,
            AchievementType::Money10,
            AchievementType::Tokens5,
            AchievementType::Orders5,
            AchievementType::Technologies4,
            AchievementType::Production4,
            AchievementType::Commercial4,
            AchievementType::Prod3Comm2,
            AchievementType::Prod2Comm3,
        ] {
            let text = achievement_requirement(a);
            assert!(
                text.chars().count() < GOAL_TEXT_W,
                "{a:?}'s requirement is {} columns, past the {GOAL_TEXT_W}-column column: {text:?}",
                text.chars().count(),
            );
        }
    }

    #[test]
    fn the_achievements_panel_shows_tiers_and_claim_state() {
        let (mut state, _) = state_np(2);
        let text = |s: &GameState| {
            strip_ansi(
                &achievements_lines(&crate::screen_model::build_achievements(s), &colors_np(s))
                    .join("\n"),
            )
        };
        let fresh = text(&state);
        assert!(fresh.contains("ACHIEVEMENTS"), "{fresh}");
        assert_eq!(fresh.lines().filter(|l| l.contains("unclaimed")).count(), 3, "{fresh}");
        for letter in ["A ·", "B ·", "C ·"] {
            assert!(fresh.contains(letter), "want a {letter} row:\n{fresh}");
        }

        state.achievements[0].claimed_by = vec![PlayerId::new(0)];
        assert!(text(&state).contains("claimed by RED"), "{}", text(&state));

        // Beyond two claimants the line counts rather than listing, so it
        // cannot clip a name in half.
        state.achievements[0].claimed_by =
            vec![PlayerId::new(0), PlayerId::new(1), PlayerId::new(0)];
        assert!(text(&state).contains("claimed by 3 players"), "{}", text(&state));
    }

    /// The hand panel draws the seat's own cards, and a MASKED one carries the
    /// count and nothing else — the model has already emptied the list, so
    /// there is nothing here that could leak.
    #[test]
    fn the_hand_panel_draws_cards_open_and_only_a_count_when_masked() {
        let (state, rules) = state_np(2);
        let open = crate::screen_model::build_hand(&state, &rules, PlayerId::new(0), false)
            .expect("seat 0");
        let open_text = strip_ansi(&hand_lines(&open).join("\n"));
        assert!(open_text.contains("YOUR HAND"), "{open_text}");
        assert!(open_text.contains(glyph::BOX_TL), "an open hand draws a card: {open_text}");
        assert!(open_text.contains("your starting order"), "{open_text}");

        let masked = crate::screen_model::build_hand(&state, &rules, PlayerId::new(0), true)
            .expect("seat 0");
        let masked_text = strip_ansi(&hand_lines(&masked).join("\n"));
        assert!(masked_text.contains("1 card"), "the count survives the mask: {masked_text}");
        assert!(masked_text.contains("hidden"), "{masked_text}");
        assert!(!masked_text.contains(glyph::BOX_TL), "a masked hand draws no card: {masked_text}");
    }

    /// A hand bigger than the panel shows draws what it can and counts the
    /// rest, rather than growing without bound.
    #[test]
    fn an_oversized_hand_draws_what_it_can_and_counts_the_rest() {
        let (state, rules) = state_np(2);
        let mut panel = crate::screen_model::build_hand(&state, &rules, PlayerId::new(0), false)
            .expect("seat 0");
        let card = panel.cards[0].clone();
        panel.cards = vec![card; HAND_CARDS_SHOWN + 3];
        panel.card_count = panel.cards.len();
        let text = strip_ansi(&hand_lines(&panel).join("\n"));
        assert_eq!(
            text.matches(glyph::BOX_TL).count(),
            HAND_CARDS_SHOWN,
            "exactly the shown cards are drawn:\n{text}"
        );
        assert!(text.contains("+3 more"), "{text}");
    }

    /// A row of hand cards fits the dashboard column — asserted, since the T3
    /// review, at the two places it can stop being true.
    ///
    /// **This line is the panel's one unguarded width.** Every other content
    /// line goes through [`line`] and is clipped at [`DASH_W`]; a card row is
    /// built from fixed-width card slices and pushed raw, because clipping a
    /// six-line box would shear it rather than shorten it. So the guard has to
    /// live here, and until now it did not: the sweep in
    /// `no_dashboard_line_exceeds_the_designed_width_at_any_count` measures an
    /// OPENING hand, which holds one card and therefore one card per row, and
    /// [`HAND_CARDS_SHOWN`] being 4 means the renderer already anticipates rows
    /// it was never shown drawing. Set [`HAND_CARDS_PER_ROW`] to 3 and both
    /// halves below fail at 78 columns against 74; the sweep alone did not
    /// notice.
    ///
    /// The ARITHMETIC half names the budget — the width the renderer composes,
    /// off the same three constants it composes from. The RENDERED half draws
    /// a hand deep enough to fill a row and measures what actually came out, so
    /// a row that grows by some route the arithmetic does not model still
    /// fails.
    #[test]
    fn a_hand_row_of_cards_fits_the_dashboard_column() {
        let n = HAND_CARDS_PER_ROW;
        let row_w = INDENT + n * CARD_W + n.saturating_sub(1) * HAND_CARD_GAP;
        assert!(
            row_w <= DASH_W,
            "{n} hand cards a row costs {row_w} columns \
             (INDENT {INDENT} + {n}x CARD_W {CARD_W} + {}x HAND_CARD_GAP {HAND_CARD_GAP}), \
             past DASH_W {DASH_W} — and nothing clips a card row",
            n - 1,
        );

        let (state, rules) = state_np(2);
        let mut panel = crate::screen_model::build_hand(&state, &rules, PlayerId::new(0), false)
            .expect("seat 0");
        // Enough cards to fill a full row and start another, whatever
        // HAND_CARDS_PER_ROW is set to.
        let card = panel.cards[0].clone();
        panel.cards = vec![card; HAND_CARDS_PER_ROW + 1];
        panel.card_count = panel.cards.len();
        let rendered = hand_lines(&panel);
        assert!(
            rendered.iter().filter(|l| l.contains(glyph::BOX_TL)).count() >= 2,
            "the fixture must actually draw more than one row of cards:\n{rendered:#?}",
        );
        for l in &rendered {
            assert!(
                visible_width(l) <= DASH_W,
                "a hand row is {} columns, past DASH_W {DASH_W}: {:?}",
                visible_width(l),
                strip_ansi(l),
            );
        }
    }

    // -----------------------------------------------------------------------
    // Title strip
    // -----------------------------------------------------------------------

    /// The strip states the mode, the seed and whether the game is autosaving,
    /// centred over the frame it heads.
    #[test]
    fn the_title_strip_states_mode_seed_and_autosave() {
        let live = TitleModel { mode: TitleMode::Live, seed: 1234, autosaved: true };
        let text = strip_ansi(&title_strip(&live, 100));
        assert!(text.contains("Y O K O H A M A"), "{text}");
        assert!(text.contains("live tracking"), "{text}");
        assert!(text.contains("seed 1234"), "{text}");
        assert!(text.contains("autosaved"), "{text}");

        let sim = TitleModel { mode: TitleMode::Sim, seed: 1234, autosaved: false };
        let sim_text = strip_ansi(&title_strip(&sim, 100));
        assert!(sim_text.contains("simulation"), "{sim_text}");
        assert!(!sim_text.contains("autosaved"), "a session with no save target says nothing: {sim_text}");
    }

    /// A long seed is elided from the LEFT, keeping the digits that vary; a
    /// short one is printed whole, because an ellipsis over nothing would
    /// misname the identity a player types to find a save file.
    #[test]
    fn a_long_seed_is_elided_and_a_short_one_is_not() {
        assert_eq!(abbreviated_seed(7), "7");
        assert_eq!(abbreviated_seed(123_456_789), "123456789");
        assert_eq!(
            abbreviated_seed(11_964_555_000),
            format!("{}964555000", glyph::ELLIPSIS)
        );
        assert_eq!(
            abbreviated_seed(u64::MAX),
            format!("{}709551615", glyph::ELLIPSIS)
        );
    }

    /// Centring is measured on VISIBLE width, so a painted strip sits where its
    /// plain twin does rather than drifting left by the length of its escapes.
    #[test]
    fn the_title_strip_centres_on_visible_width() {
        let title = TitleModel { mode: TitleMode::Sim, seed: 7, autosaved: false };
        let plain = title_strip(&title, 120);
        crate::style::force(true);
        let painted = title_strip(&title, 120);
        crate::style::force(false);
        assert_ne!(plain, painted, "forced color must actually paint");
        assert_eq!(strip_ansi(&painted), plain);
        assert!(visible_width(&plain) <= 120, "{plain:?}");
    }

    // -----------------------------------------------------------------------
    // Painting
    // -----------------------------------------------------------------------

    /// Painting must never move a visible character — the crate's alignment
    /// invariant, on the surface with the most owner-tinted tokens in it.
    #[test]
    fn a_painted_dashboard_strips_back_to_the_plain_one() {
        let (panels, board, colors) = panels_np(4);
        let plain = dashboard(&panels, &board, &colors);
        crate::style::force(true);
        let painted = dashboard(&panels, &board, &colors);
        crate::style::force(false);
        assert_ne!(painted, plain, "forced color must actually paint");
        assert_eq!(strip_ansi(&painted), plain);
    }

    // -----------------------------------------------------------------------
    // The below arrangement (T10)
    // -----------------------------------------------------------------------

    /// The block width [`crate::screen`] hands the below arrangement at `n`
    /// players: `seats x SEAT_COL_W` against what the stage already occupies
    /// (the 163-column site-slot legend at 3, the 180-column board at 4) and
    /// against [`BELOW_BLOCK_MIN`], which is the one that wins at 3.
    ///
    /// A mirror of `screen::below_block_width`, and deliberately a crude one —
    /// these tests are about what a block of a given width renders, so they
    /// state the width rather than compose a frame to discover it. The
    /// COMPOSED numbers are pinned where they are produced, by
    /// `screen::the_frame_width_table_matches_the_composed_opening_frame`.
    fn block_np(n: u8) -> usize {
        (usize::from(n) * SEAT_COL_W).max(if n == 3 { 163 } else { 180 }).max(BELOW_BLOCK_MIN)
    }

    /// Every legal Achievement card triple: one per category, which is the
    /// engine's own rule for the three in play, so the product is 4 x 4 x 4.
    fn achievement_triples() -> Vec<[AchievementType; 3]> {
        const A: [AchievementType; 4] = [
            AchievementType::Copper5,
            AchievementType::Silk6,
            AchievementType::Tea7,
            AchievementType::Seafood7,
        ];
        const B: [AchievementType; 4] = [
            AchievementType::Money10,
            AchievementType::Tokens5,
            AchievementType::Orders5,
            AchievementType::Technologies4,
        ];
        const C: [AchievementType; 4] = [
            AchievementType::Production4,
            AchievementType::Commercial4,
            AchievementType::Prod3Comm2,
            AchievementType::Prod2Comm3,
        ];
        let mut out = Vec::new();
        for a in A {
            for b in B {
                for c in C {
                    out.push([a, b, c]);
                }
            }
        }
        out
    }

    /// Every claim state one card can be in at `n` players: the claimants are a
    /// SUBSET of the seats, so this is the power set.
    ///
    /// Subsets rather than ORDERED claim lists, and the reason is width: a
    /// two-claimant state renders `NAME, NAME`, whose width does not depend on
    /// which name comes first, and every other arity renders a name or a count.
    /// The engine's `claimed_by` is ordered; the row's width is not.
    fn claim_subsets(n: u8) -> Vec<Vec<PlayerId>> {
        (0..(1u32 << n))
            .map(|mask| {
                (0..n).filter(|i| mask & (1 << i) != 0).map(PlayerId::new).collect()
            })
            .collect()
    }

    fn below_np(n: u8) -> String {
        let (panels, board, colors) = panels_np(n);
        dashboard_below(&panels, &board, &colors, block_np(n))
    }

    /// THE below-arrangement width property, and the frame's whole width
    /// budget: no line may exceed the block it was given, and the section
    /// rules span it exactly.
    #[test]
    fn no_below_dashboard_line_exceeds_its_block_at_three_and_four_players() {
        for n in [3u8, 4] {
            let block = block_np(n);
            let text = below_np(n);
            for l in text.lines() {
                assert!(
                    visible_width(l) <= block,
                    "a {n}P below-dashboard line is {} columns, past the {block}-column \
                     block: {:?}",
                    visible_width(l),
                    strip_ansi(l),
                );
            }
            let rules: Vec<&str> =
                text.lines().filter(|l| strip_ansi(l).starts_with(glyph::RULE)).collect();
            assert_eq!(rules.len(), 4, "one rule per panel at {n}P:\n{text}");
            // Only the PLAYERS rule fills its block; the three compressed
            // panels are titled rules that carry content and stop where their
            // content does (the approved 4P mock's own shape).
            assert_exact_width(&rules[..1], block);
        }
    }

    /// The below dashboard is a FIXED-HEIGHT block, and that is what retires
    /// the 3P/4P height debt: every seat spends exactly [`SEAT_ROWS`] rows
    /// whatever it owns, so no position in a game can grow the frame.
    #[test]
    fn the_below_dashboard_is_a_fixed_height_block_at_every_position() {
        for n in [3u8, 4] {
            let opening = below_np(n).lines().count();
            let (state, rules) = state_np(n);
            let mut loaded = state.clone();
            // A seat that owns half the tech deck, holds a hand, and has pawns
            // locked on goals and tracks: every field that could add a line.
            let ids: Vec<TechCardId> =
                rules.techs.iter_sorted_by_id().iter().take(10).map(|c| c.id).collect();
            loaded.players[0].technologies = ids;
            loaded.players[0].assistants.on_achievements = 3;
            loaded.players[0].assistants.on_church = 1;
            loaded.players[1].order_cards_in_hand =
                rules.orders.iter_sorted_by_id().iter().take(6).map(|c| c.id).collect();
            let seats = crate::screen_model::SeatFacts::all_open(usize::from(n));
            let panels =
                crate::screen_model::build_dashboard(&loaded, &rules, &seats, PlayerId::new(0));
            let board = crate::screen_model::build_board(&loaded, &rules, &[]);
            let colors = colors_np(&loaded);
            let text = dashboard_below(&panels, &board, &colors, block_np(n));
            assert_eq!(
                text.lines().count(),
                opening,
                "a loaded {n}P position must render the same number of rows as the \
                 opening one:\n{text}"
            );
            for l in text.lines() {
                assert!(
                    visible_width(l) <= block_np(n),
                    "a loaded {n}P line is {} columns, past its block: {:?}",
                    visible_width(l),
                    strip_ansi(l),
                );
            }
        }
    }

    /// Every seat gets a column, in seat order, carrying the whole public
    /// picture — the same fields the beside arrangement shows, in the approved
    /// 4P mock's compressed wording.
    #[test]
    fn a_below_seat_column_carries_every_public_field() {
        let text = strip_ansi(&below_np(4));
        for want in [
            "PLAYERS", "Goods  ", "Assistants  ", "Shophouses  ", "Trade houses  ", "Tech  ",
            "Mercenaries  ", "President ", "filled", "next take ", "TRACKS", "ACHIEVEMENTS",
            "YOUR HAND",
        ] {
            assert!(text.contains(want), "want {want:?} in the below dashboard:\n{text}");
        }
        for name in ["RED", "BLUE", "YELLOW", "GREEN"] {
            assert!(text.contains(name), "every seat gets a column: {name}\n{text}");
        }
        // The compressed noun set is the 4P mock's, not the beside panel's.
        assert!(text.contains("8 hand"), "the mock's compressed zone wording:\n{text}");
        assert!(
            !text.contains("in warehouse"),
            "the below column has no room for the long form:\n{text}"
        );
        // One header row carries every seat, so the four names share a line.
        let header = text.lines().find(|l| l.contains("RED")).expect("a header row");
        for name in ["BLUE", "YELLOW", "GREEN"] {
            assert!(header.contains(name), "the seats are COLUMNS, not rows: {header:?}");
        }
    }

    /// The compressed ACHIEVEMENTS row survives EVERY legal content at BOTH
    /// blocks — the one place the below arrangement rests on arithmetic rather
    /// than on a shape, so the arithmetic is swept rather than argued.
    ///
    /// [`line_at`] clips, so a row that outgrew its block would not break the
    /// frame; it would quietly lose its TAIL, which is the third card's claim
    /// state — the one fact on this row that moves during a game.
    ///
    /// **This test replaced a fixture that measured one hand-built row and got
    /// the number wrong.** T10 recorded "the widest legal row is 162 columns"
    /// from a fixture that set `claimed_by` to seats 0 and 1 — `RED, BLUE`, the
    /// NARROWEST claimant pair in the game. The widest is `BLUE, YELLOW` at 3
    /// players and `YELLOW, GREEN` at 4, and at the 163-column block T10
    /// shipped, **580 of the 32,768 legal 3-player states clipped**. A pin that
    /// states one state's width can only ever be as right as its author's
    /// guess about which state is widest; this one enumerates.
    ///
    /// **MEASURED, then set:** 171 columns at 3 players over 32,768 states,
    /// 174 at 4 over 262,144 — the two numbers [`BELOW_BLOCK_MIN`] is derived
    /// from, asserted here as equalities so a wording change that moves either
    /// one fails next to the constant it feeds rather than eating its margin
    /// silently.
    #[test]
    fn the_widest_legal_achievements_row_survives_its_block_at_both_counts() {
        for (n, want_max) in [(3u8, 171usize), (4, 174)] {
            let (state, _) = state_np(n);
            let colors = colors_np(&state);
            let block = block_np(n);
            let subsets = claim_subsets(n);
            let mut swept = 0usize;
            let mut worst = (0usize, String::new());
            for triple in achievement_triples() {
                for ca in &subsets {
                    for cb in &subsets {
                        for cc in &subsets {
                            let cards = [(triple[0], ca), (triple[1], cb), (triple[2], cc)]
                                .into_iter()
                                .map(|(t, claim)| AchievementModel {
                                    achievement_type: t,
                                    category: t.category(),
                                    first_place_vp: t.first_place_vp(),
                                    subsequent_vp: t.subsequent_vp(),
                                    claimed_by: (*claim).clone(),
                                    // Per-seat progress is the `goals` view's
                                    // business; this row never reads it.
                                    progress: Vec::new(),
                                })
                                .collect();
                            let row = achievements_row(
                                &AchievementsPanel { cards },
                                &colors,
                                block,
                            );
                            let plain = strip_ansi(&row);
                            assert!(
                                !plain.contains(glyph::ELLIPSIS),
                                "a legal {n}P achievements row clipped inside a {block}-column \
                                 block: {plain:?}",
                            );
                            swept += 1;
                            if visible_width(&row) > worst.0 {
                                worst = (visible_width(&row), plain);
                            }
                        }
                    }
                }
            }
            assert_eq!(
                swept,
                64 * subsets.len().pow(3),
                "the sweep must cover every card triple at every claim state",
            );
            assert_eq!(
                worst.0, want_max,
                "the widest legal {n}P achievements row measures {} columns, not the recorded \
                 {want_max}: {:?}\nIf that is deliberate, RE-MEASURE BELOW_BLOCK_MIN from this \
                 sweep -- it is set from these two numbers",
                worst.0, worst.1,
            );
            assert!(
                worst.0 < block,
                "{n}P: the widest legal row ({}) must fit its {block}-column block with room \
                 to spare",
                worst.0,
            );
        }
    }

    /// The compressed TRACKS row survives every pawn count, at both blocks.
    ///
    /// **This row grows with the game and T10 never measured it that way.** An
    /// occupied space draws `[7●]` where an empty one draws `[7]`, so every
    /// pawn placed on either management board costs the row a column, and what
    /// a clip takes is the tail — the mercenary supply. The review found it
    /// clipping from 9 pawns at the 163-column block.
    ///
    /// Graduated 0..=15 pawns, which walks past every count's end-game trigger
    /// (5 real pawns at 3 players, 6 at 4) and on to the structural ceiling:
    /// all 8 Church spaces and all 7 Customs occupied. The trigger only starts
    /// a final round rather than freezing the boards, so the reachable maximum
    /// is somewhere above the trigger and below the ceiling — sweeping to the
    /// ceiling makes the pin independent of exactly where.
    ///
    /// **MEASURED: 170 columns at both counts**, at 15 pawns. The 2-player
    /// dummy's ` · D = dummy, not counted` tail is 24 columns more and is NOT
    /// swept here: the dummy exists only at 2 players and 2 players draws the
    /// BESIDE arrangement, which does not use this row at all.
    #[test]
    fn the_widest_legal_tracks_row_survives_its_block_at_both_counts() {
        for n in [3u8, 4] {
            let (state, _) = state_np(n);
            let colors = colors_np(&state);
            let block = block_np(n);
            let mut worst = (0usize, String::new(), 0usize);
            for pawns in 0..=15usize {
                let mut s = state.clone();
                for i in 0..pawns.min(s.church_track.slots.len()) {
                    s.church_track.slots[i] = Some(ChurchSlot {
                        player: PlayerId::new((i % usize::from(n)) as u8),
                        arrival_order: i as u8,
                    });
                }
                let on_customs = pawns.saturating_sub(s.church_track.slots.len());
                for i in 0..on_customs.min(s.customs_track.slots.len()) {
                    s.customs_track.slots[i] = Some(CustomsSlot {
                        player: PlayerId::new((i % usize::from(n)) as u8),
                        arrival_order: i as u8,
                    });
                }
                let row = tracks_row(&crate::screen_model::build_tracks(&s), &colors, block);
                let plain = strip_ansi(&row);
                assert!(
                    !plain.contains(glyph::ELLIPSIS),
                    "the {n}P tracks row clipped at {pawns} pawns inside a {block}-column \
                     block: {plain:?}",
                );
                assert!(
                    plain.contains("mercenaries left"),
                    "the row's TAIL is what a clip takes, so it is what a pass must show: \
                     {plain:?}",
                );
                if visible_width(&row) > worst.0 {
                    worst = (visible_width(&row), plain, pawns);
                }
            }
            assert_eq!(
                worst.0, 170,
                "the widest {n}P tracks row measures {} columns at {} pawns, not the recorded \
                 170: {:?}\nIf that is deliberate, RE-MEASURE BELOW_BLOCK_MIN",
                worst.0, worst.2, worst.1,
            );
            assert_eq!(worst.2, 15, "the widest row must be the fullest one");
        }
    }

    /// The 3-4 player seat tail row carries the FILLED count and never the
    /// codes — at any width, including ones far wider than a seat column, so a
    /// renderer that only withheld them because they did not fit fails here.
    #[test]
    fn the_seat_tail_row_carries_the_filled_count() {
        let (mut state, rules) = state_np(4);
        state.players[0].completed_orders =
            vec![OrderCardId(23), OrderCardId(8), OrderCardId(12)];
        let seats = crate::screen_model::SeatFacts::all_open(4);
        let panel = crate::screen_model::build_players(&state, &rules, &seats);
        for w in [SEAT_COL_W - SEAT_COL_GAP, 56, DASH_W, 200] {
            let row = strip_ansi(&seat_tail_row(&panel.players[0], w));
            assert!(row.contains("3 filled"), "at width {w}: {row:?}");
            for id in [23u8, 8, 12] {
                assert!(
                    !row.contains(&format!("O{id:02}")),
                    "the tail row names no card at any width -- the codes are the 2-player \
                     Orders line's business (width {w}): {row:?}",
                );
            }
        }
    }

    /// The seat TAIL row is the tightest line in the column layout, so it gets
    /// the measured-maximum treatment its column-mates do not need (T10 review
    /// N1).
    ///
    /// **RE-MEASURED at branch-4's Orders work over FOUR facts and a
    /// CONSTRAINED joint domain.** The row gained the filled count, and its
    /// four-fact worst case does not fit: swept naively it reaches 59 columns
    /// against the 48 a 4-player column gives it. So the row has a YIELD POLICY
    /// (spec §5 decision 9) and this pin is what proves the policy sufficient
    /// rather than plausible.
    ///
    /// The domain is the whole row at once, because the facts are drawn on one
    /// line and it is their SUM that has to fit — and it is JOINT: 36 order
    /// cards exist, so `in hand + filled <= 36` (703 pairs), crossed with the
    /// mask, the locked-pawn tally (0..=18, the three Achievement cards plus
    /// both management boards) and the whole 4/5/6/7 Employment-Agency
    /// progression. 106,856 rows, every one of them rendered.
    ///
    /// **MEASURED, both halves:**
    ///
    /// - the widest row that KEEPS its next-take price is **48 columns**, which
    ///   is the column exactly — the price is appended only when it fits, so
    ///   this is a budget met rather than a margin;
    /// - the widest row that YIELDS it is **44**, and that is the number which
    ///   makes the policy total: dropping one fact is always enough, so the row
    ///   never reaches [`line_at`]'s clip and no fact is ever half-drawn.
    ///
    /// The old reading was 47 against 48 over three facts; the one column of
    /// margin it recorded is spent, which is exactly what spec §5 decision 9
    /// says it is spending it on.
    #[test]
    fn the_widest_legal_seat_tail_row_still_fits_its_column() {
        let (state, rules) = state_np(4);
        let seats = crate::screen_model::SeatFacts::all_open(4);
        let panel = crate::screen_model::build_players(&state, &rules, &seats);
        let content_w = SEAT_COL_W - SEAT_COL_GAP;
        let ids: Vec<OrderCardId> = (1..=36u8).rev().map(OrderCardId).collect();
        let mut swept = 0usize;
        let mut worst_kept = (0usize, String::new());
        let mut worst_yielded = (0usize, String::new());
        for orders in 0..=36usize {
            for filled in 0..=(36 - orders) {
                for hidden in [false, true] {
                    for locked in 0..=18u8 {
                        for take in [4u8, 5, 6, 7] {
                            let mut p = panel.players[0].clone();
                            p.orders_in_hand = orders;
                            p.orders_filled = ids[..filled].to_vec();
                            p.hand_hidden = hidden;
                            // Spread the locked pawns across the three places
                            // they can be, capped at what each holds: 3
                            // Achievement cards, 8 Church spaces, 7 Customs
                            // spaces.
                            p.assistants.on_achievements = locked.min(3);
                            p.assistants.on_church = locked.saturating_sub(3).min(8);
                            p.assistants.on_customs = locked.saturating_sub(11).min(7);
                            p.trading_houses.next_take_cost = take;
                            let row = seat_tail_row(&p, content_w);
                            let plain = strip_ansi(&row);
                            swept += 1;

                            assert!(
                                !plain.contains(glyph::ELLIPSIS),
                                "a legal tail row clipped inside a {content_w}-column \
                                 column: {plain:?}",
                            );
                            // The three facts that never yield are on every row
                            // whatever else had to go.
                            assert!(
                                plain.contains(&format!("{filled} filled")),
                                "the filled count never yields: {plain:?}",
                            );
                            assert!(
                                locked == 0 || plain.contains(&format!("{locked} locked")),
                                "the locked tally never yields: {plain:?}",
                            );
                            assert_eq!(
                                plain.matches("(hidden)").count(),
                                usize::from(hidden),
                                "the mask never yields: {plain:?}",
                            );

                            let slot = if plain.contains("next take") {
                                &mut worst_kept
                            } else {
                                &mut worst_yielded
                            };
                            if visible_width(&row) > slot.0 {
                                *slot = (visible_width(&row), plain);
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(
            swept,
            703 * 2 * 19 * 4,
            "the sweep must cover every legal (in hand, filled) pair at every mask, locked \
             tally and take price",
        );
        assert_eq!(
            worst_kept.0, 48,
            "the widest legal tail row that keeps its next-take price measures {} columns, \
             not the recorded 48: {:?}",
            worst_kept.0, worst_kept.1,
        );
        assert_eq!(
            worst_yielded.0, 44,
            "the widest legal tail row that yields it measures {} columns, not the recorded \
             44: {:?}\nThat number is what makes the yield policy TOTAL -- if it ever \
             exceeds the column, dropping one fact stopped being enough and a second fact \
             needs a stated order",
            worst_yielded.0, worst_yielded.1,
        );
        for (label, worst) in [("keeping", &worst_kept), ("yielding", &worst_yielded)] {
            assert!(
                worst.0 <= content_w,
                "and the widest {label} row ({}) must fit the {content_w}-column column",
                worst.0,
            );
        }
    }

    /// [`header_row`]'s clip, pinned on its own (T10 review MUST-1(c)).
    ///
    /// **The branch is unreachable from any legal state since
    /// [`BELOW_BLOCK_MIN`] widened the 3-player block**, which is exactly why
    /// it needs this: the review deleted the clip entirely and all 569 tests
    /// stayed green. An untested safety net is not a safety net, and the thing
    /// it prevents — a compressed row written over whatever sits to its right —
    /// is worse than the truncation it produces.
    ///
    /// Synthetic input, because the point is that no real one reaches here.
    #[test]
    fn an_over_wide_compressed_row_is_clipped_to_its_block_not_overrun() {
        let mut body = Twin::default();
        body.raw(&"x".repeat(200));
        let row = header_row("TRACKS", &body, 40);
        assert_eq!(visible_width(&row), 40, "a clipped row is EXACTLY its block: {row:?}");
        assert!(
            strip_ansi(&row).ends_with(glyph::ELLIPSIS),
            "and it says it was clipped: {row:?}",
        );
        // The other direction, so this pin cannot pass by clipping everything:
        // a row inside its block is left alone, ragged right.
        let mut short = Twin::default();
        short.raw("fits");
        let row = header_row("TRACKS", &short, 40);
        assert_eq!(strip_ansi(&row), "─ TRACKS  fits", "an inside-block row is untouched");
    }

    /// The privacy fence is identical at every count: a masked seat's hand
    /// shows a count and no card identity, and the panel that could leak one
    /// is empty before it reaches this renderer.
    #[test]
    fn the_below_hand_row_masks_exactly_as_the_beside_panel_does() {
        let (state, rules) = state_np(4);
        let board = crate::screen_model::build_board(&state, &rules, &[]);
        let colors = colors_np(&state);
        let card = state.players[0].order_cards_in_hand[0];
        let printed = rules.orders.get(card).expect("a registry order");

        let open = crate::screen_model::build_hand(&state, &rules, PlayerId::new(0), false)
            .expect("seat 0");
        let open_text = strip_ansi(&dashboard_below(
            &[Panel::Hand(open)],
            &board,
            &colors,
            block_np(4),
        ));
        assert!(open_text.contains(&format!("O{:02}", printed.id.0)), "{open_text}");

        let masked = crate::screen_model::build_hand(&state, &rules, PlayerId::new(0), true)
            .expect("seat 0");
        let masked_text = strip_ansi(&dashboard_below(
            &[Panel::Hand(masked)],
            &board,
            &colors,
            block_np(4),
        ));
        assert!(masked_text.contains("1 card"), "the count survives the mask: {masked_text}");
        assert!(masked_text.contains("hidden"), "{masked_text}");
        assert!(
            !masked_text.contains(&format!("O{:02}", printed.id.0)),
            "a masked hand names no card: {masked_text}"
        );
    }

    /// A bot seat is badged in the column layout exactly as it is beside the
    /// board, and its hand stays masked — 3.2's rule, unchanged by the
    /// arrangement.
    ///
    /// **The mask half of this used to be checked with `text.contains("hidden")`
    /// over the WHOLE frame, and that assertion was vacuous**: the YOUR HAND row
    /// says `1 card — hidden` for the viewing seat, so the frame contains the
    /// word whether or not any seat column carries its own marker. Disabling
    /// [`seat_tail_row`]'s hidden branch entirely passed all 558 tests. That is
    /// the crate's THIRD recurrence of one lesson — a whole-frame `contains` is
    /// satisfied by overlapping text elsewhere on the screen — so the marker is
    /// pinned per LINE below, by
    /// `the_hidden_marker_is_drawn_on_exactly_the_masked_seats_tail_row`.
    #[test]
    fn a_bot_seat_is_badged_and_masked_in_the_column_layout() {
        let (state, rules) = state_np(4);
        let seats: Vec<crate::screen_model::SeatFacts> = (0..4)
            .map(|i| crate::screen_model::SeatFacts { is_bot: i == 2, hand_hidden: i == 2 })
            .collect();
        let panels = crate::screen_model::build_dashboard(&state, &rules, &seats, PlayerId::new(2));
        let board = crate::screen_model::build_board(&state, &rules, &[]);
        let text = strip_ansi(&dashboard_below(&panels, &board, &colors_np(&state), block_np(4)));
        assert!(text.contains(BOT_BADGE.trim()), "the bot seat is badged:\n{text}");
        assert!(text.contains("hidden"), "and its own hand stays masked:\n{text}");
    }

    /// The per-seat `(hidden)` marker, asserted where it is DRAWN (T10 review
    /// SHOULD-3).
    ///
    /// Two properties one whole-frame `contains` cannot express: the masked
    /// seat's tail row carries the marker exactly ONCE, and an open seat's tail
    /// row carries it NOT AT ALL. Checked on the tail rows themselves — the
    /// column-layout rows that begin with an orders count — rather than on the
    /// frame, because the frame also contains the viewing seat's YOUR HAND row
    /// and that row says `hidden` in every masked game.
    ///
    /// Masking two of four seats rather than one, so a renderer that drew the
    /// marker on the FIRST masked column and stopped fails here too.
    #[test]
    fn the_hidden_marker_is_drawn_on_exactly_the_masked_seats_tail_row() {
        let (state, rules) = state_np(4);
        let masked = [false, true, false, true];
        let seats: Vec<crate::screen_model::SeatFacts> = masked
            .iter()
            .map(|&m| crate::screen_model::SeatFacts { is_bot: m, hand_hidden: m })
            .collect();
        let panel = crate::screen_model::build_players(&state, &rules, &seats);
        for (seat, want_marker) in masked.iter().enumerate().map(|(i, m)| (i, *m)) {
            let row = strip_ansi(&seat_tail_row(
                &panel.players[seat],
                SEAT_COL_W - SEAT_COL_GAP,
            ));
            assert_eq!(
                row.matches("(hidden)").count(),
                usize::from(want_marker),
                "seat {seat} is {} masked, so its tail row must carry the marker {} time(s): \
                 {row:?}",
                if want_marker { "" } else { "NOT" },
                usize::from(want_marker),
            );
            // Anti-vacuity: the row must be a tail row at all, not an empty
            // string that trivially contains nothing.
            assert!(row.contains("next take"), "seat {seat}'s tail row: {row:?}");
        }
    }

    /// Painting must never move a visible character — the crate's alignment
    /// invariant, on the arrangement that pads columns to a fixed stride.
    #[test]
    fn a_painted_below_dashboard_strips_back_to_the_plain_one() {
        let (panels, board, colors) = panels_np(4);
        let plain = dashboard_below(&panels, &board, &colors, block_np(4));
        crate::style::force(true);
        let painted = dashboard_below(&panels, &board, &colors, block_np(4));
        crate::style::force(false);
        assert_ne!(painted, plain, "forced color must actually paint");
        assert_eq!(strip_ansi(&painted), plain);
    }

    /// A seat the color table does not cover keeps its content and loses only
    /// its tint — never the other way round.
    #[test]
    fn a_seat_with_no_color_still_renders() {
        let (panels, board, _) = panels_np(2);
        let text = dashboard(&panels, &board, &[]);
        assert!(strip_ansi(&text).contains("RED"), "{text}");
        assert!(strip_ansi(&text).contains("BLUE"), "{text}");
    }

    /// The President's area is NAMED. An id the board model does not carry
    /// falls back to the id rather than to a wrong name.
    #[test]
    fn the_president_line_names_its_area() {
        let (mut state, rules) = state_np(2);
        let area = AreaId(3);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        let seats = crate::screen_model::SeatFacts::all_open(2);
        let board = crate::screen_model::build_board(&state, &rules, &[]);
        let panel = crate::screen_model::build_players(&state, &rules, &seats);
        let text = strip_ansi(&players_lines(&panel, &board, &colors_np(&state)).join("\n"));
        let expected = describe::area_type_name(
            state.board_layout.areas.get(&area).expect("a real area").area_type,
        )
        .to_uppercase();
        assert!(text.contains(&format!("on {expected}")), "want {expected:?} in:\n{text}");

        let empty = BoardModel { player_count: 2, tiles: Vec::new() };
        let fallback = strip_ansi(&players_lines(&panel, &empty, &colors_np(&state)).join("\n"));
        assert!(fallback.contains("on A03"), "{fallback}");
    }

    // -----------------------------------------------------------------------
    // The orders line (branch-4 item 7)
    // -----------------------------------------------------------------------

    /// The owner's own example, rendered: the count a seat holds, the count it
    /// has filled, and WHICH cards those were — in COMPLETION order, which is
    /// the order `Player::completed_orders` is written in and the only order
    /// that carries information (spec §2.5, decision 3).
    ///
    /// The negative half is the point of the positive one: the ascending form
    /// of the same three codes must NOT appear, so a renderer that sorted them
    /// on the way out fails here rather than reading plausibly.
    #[test]
    fn the_orders_line_states_hand_count_and_filled_codes_in_completion_order() {
        let (mut state, rules) = state_np(2);
        let extra = state.order_deck.pop().expect("the setup deck is not empty");
        {
            let p = &mut state.players[0];
            p.order_cards_in_hand.push(extra);
            p.completed_orders = vec![OrderCardId(23), OrderCardId(8), OrderCardId(12)];
        }
        let seats = crate::screen_model::SeatFacts::all_open(2);
        let board = crate::screen_model::build_board(&state, &rules, &[]);
        let panel = crate::screen_model::build_players(&state, &rules, &seats);
        let text = strip_ansi(&players_lines(&panel, &board, &colors_np(&state)).join("\n"));

        let want = format!("Orders  2 in hand {} 3 filled (O23, O08, O12)", glyph::SEPARATOR);
        assert!(text.contains(&want), "want {want:?} in:\n{text}");
        assert!(
            !text.contains("(O08, O12, O23)"),
            "the codes are the CHRONOLOGY, so the sorted form is a defect, not an equivalent:\n\
             {text}",
        );
    }

    /// A seat that has filled nothing says so and opens no parenthetical — an
    /// empty `()` would read as a card whose code the renderer lost.
    #[test]
    fn zero_filled_has_no_parenthetical() {
        let (state, rules) = state_np(2);
        let seats = crate::screen_model::SeatFacts::all_open(2);
        let panel = crate::screen_model::build_players(&state, &rules, &seats);
        let row = strip_ansi(&orders_line(&panel.players[0]));
        assert!(
            row.ends_with("0 filled"),
            "a seat with nothing filled ends its orders line at the count: {row:?}",
        );
    }

    /// One fact, one home: the in-hand count MOVED to the orders line, so the
    /// President line is location-only. Both President states, since a line
    /// reading `President in hand` legitimately contains those words.
    #[test]
    fn the_president_line_no_longer_carries_the_order_clause() {
        let (mut state, rules) = state_np(2);
        state.players[1].president_location = PresidentLocation::OnBoard(AreaId(3));
        let seats = crate::screen_model::SeatFacts::all_open(2);
        let board = crate::screen_model::build_board(&state, &rules, &[]);
        let panel = crate::screen_model::build_players(&state, &rules, &seats);
        let colors = colors_np(&state);
        for p in &panel.players {
            let row = strip_ansi(&president_line(p, &board, &colors));
            assert!(
                !row.contains("order"),
                "the President line states a LOCATION and nothing else: {row:?}",
            );
            assert!(
                !row.contains(glyph::SEPARATOR),
                "one fact needs no separator: {row:?}",
            );
        }
    }

    /// A masked seat's orders line keeps the SAME treatment the President line
    /// gave it — the count with `(hidden)` declaring the mask beside it — and
    /// the filled clause is public on every seat, because fulfilling an order
    /// is a public act with the card turned face up (spec §2.5).
    #[test]
    fn a_hidden_bot_hand_puts_the_hidden_marker_on_the_count_not_the_filled_clause() {
        let (mut state, rules) = state_np(2);
        let extra = state.order_deck.pop().expect("the setup deck is not empty");
        {
            let p = &mut state.players[1];
            p.order_cards_in_hand.push(extra);
            p.completed_orders = vec![OrderCardId(23), OrderCardId(8)];
        }
        let seats = vec![
            crate::screen_model::SeatFacts { is_bot: false, hand_hidden: false },
            crate::screen_model::SeatFacts { is_bot: true, hand_hidden: true },
        ];
        let panel = crate::screen_model::build_players(&state, &rules, &seats);
        let row = strip_ansi(&orders_line(&panel.players[1]));
        let want = format!("Orders  2 in hand (hidden) {} 2 filled (O23, O08)", glyph::SEPARATOR);
        assert_eq!(row.trim_start(), want, "the masked seat's orders line");
    }

    /// The 2-player orders line is the one dashboard line whose length grows
    /// with a card COUNT rather than with a word, so its budget is swept rather
    /// than argued — the same treatment the achievements, tracks and seat-tail
    /// rows get.
    ///
    /// **The domain is JOINT and constrained by the deck**: 36 order cards
    /// exist, so a seat's in-hand count plus its filled count cannot exceed 36
    /// (`in_hand + filled <= 36`, 703 pairs), each at both mask states — 1,406
    /// states, every one of them rendered.
    ///
    /// **MEASURED: 74 columns, exactly [`DASH_W`]** — the elision is what
    /// guarantees that rather than a margin that happens to hold, so the widest
    /// legal line FILLS its column and none of them clips. What a clip would
    /// look like is a line ending in `…`; what this renders instead ends in
    /// `)`, with the `N filled` count exact whether or not its codes were
    /// elided (spec §5 decision 10).
    #[test]
    fn the_widest_legal_orders_line_survives_its_budget() {
        let (state, rules) = state_np(2);
        let seats = crate::screen_model::SeatFacts::all_open(2);
        let panel = crate::screen_model::build_players(&state, &rules, &seats);
        // Descending, so a renderer that sorted the codes could not be told
        // apart from one that did not by width alone -- and every printed
        // corner index is two digits under `O{:02}`, so the sweep's widths do
        // not depend on WHICH cards these are.
        let ids: Vec<OrderCardId> = (1..=36u8).rev().map(OrderCardId).collect();

        let mut swept = 0usize;
        let mut worst = (0usize, String::new());
        for in_hand in 0..=36usize {
            for filled in 0..=(36 - in_hand) {
                for hidden in [false, true] {
                    let mut p = panel.players[0].clone();
                    p.orders_in_hand = in_hand;
                    p.orders_filled = ids[..filled].to_vec();
                    p.hand_hidden = hidden;
                    let row = orders_line(&p);
                    let plain = strip_ansi(&row);
                    swept += 1;

                    assert!(
                        plain.contains(&format!("{filled} filled")),
                        "the count stays EXACT however the codes elide: {plain:?}",
                    );
                    if filled == 0 {
                        assert!(plain.ends_with("0 filled"), "{plain:?}");
                    } else {
                        assert!(
                            plain.contains(" filled ("),
                            "a seat with filled orders opens its codes: {plain:?}",
                        );
                        assert!(
                            plain.ends_with(')'),
                            "the paren ALWAYS closes -- an elided tail is not a clipped one: \
                             {plain:?}",
                        );
                    }
                    assert!(
                        visible_width(&row) <= DASH_W,
                        "a legal orders line is {} columns, past DASH_W {DASH_W}: {plain:?}",
                        visible_width(&row),
                    );
                    if visible_width(&row) > worst.0 {
                        worst = (visible_width(&row), plain);
                    }
                }
            }
        }
        assert_eq!(swept, 703 * 2, "the sweep must cover every legal (in hand, filled) pair");
        assert_eq!(
            worst.0, 74,
            "the widest legal orders line measures {} columns, not the recorded 74: {:?}",
            worst.0, worst.1,
        );
    }
}
