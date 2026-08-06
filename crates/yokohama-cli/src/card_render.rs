//! The index cards — **one emitter per registry, drawn by every surface that
//! draws a card** (Phase-3.5a spec §0.6, plans T5/T6; design authority is the
//! approved mock `2026-08-05-phase3.5a-mocks/order-index.ans` and its
//! `order_card()`, generalized to the other three registries at T6).
//!
//! **Four card shapes, one entry point.** [`index_card`] dispatches on the
//! model's own [`crate::screen_model::CardFaceModel`], so nothing outside this
//! module decides what a card looks like, and a card is as tall as its own
//! content — a technology with a one-line effect costs one row less than one
//! with three, which is what keeps the tech index inside the design floor.
//! The four widths differ because the four physical cards do: an order card is
//! a grid of tokens, a technology is a paragraph of rules text, an achievement
//! is a sentence, a building site is eight printed rewards.
//!
//! **Why this is its own module.** The card shape landed at T3 inside
//! [`crate::panel_render`], where the dashboard's hand panel needed it, and T5
//! needs the same 24-column card in the full-screen index — with a selection
//! tag on the title and a location note on the last interior line. Copying it
//! would have been two card shapes that agree only while somebody keeps them
//! agreeing, and the whole point of R8's index is that a card LOOKS the same
//! wherever the player meets it: in their hand, in the index, echoed back after
//! they type an id. So the emitter moved out to where both callers can reach
//! it, and [`CardFace`] is the union of what the two of them vary.
//!
//! **The R9 seam, held exactly as its sibling renderers hold it.** This module
//! renders; it never reads game state. Its inputs are printed card data plus a
//! [`CardFace`]'s presentation flags, and the only `yokohama_core` names it
//! touches are the types a card is made of. Glyphs come only from
//! [`crate::style::glyph`] and color only from [`crate::style::Tone`].
//!
//! **Totality.** Every emitted line measures exactly [`CARD_W`] visible
//! columns — [`crate::board_render::exact`] enforces it on every line without
//! exception, so a long country name beside a two-digit selection tag clips
//! rather than shearing the grid it sits in. The tightest real line is the
//! title of a dimmed `NETHERLANDS` card at a two-digit selection:
//! `┌ ` + `[28] ` + `NETHERLANDS` + ` ` + `O21` + ` ┐` is 24 columns exactly,
//! which is the case `the_widest_card_title_still_measures_exactly_card_w`
//! pins.

use yokohama_core::{
    OrderCard, OrderCardId, OrderRewards, PlayerColor, SpaceReward,
    TradeGoodRequirement,
};

use crate::board_render::{color_of, exact, Twin};
use crate::describe;
use crate::screen_model::{AchievementModel, CardFaceModel, CardLocation, IndexCard, IndexKind};
use crate::style::{glyph, GoodKind, Tone};

/// A mini order card's total width, borders included — the mocks' `CARD_W`.
pub const CARD_W: usize = 24;

/// A technology card's width. Four to a row inside the 200-column floor, which
/// is what the effect text needs: the longest of the twenty (`Telegram`,
/// `Station`) runs 103 characters and wraps to three lines at this interior —
/// the widest card that still keeps a six-title flag group to two rows.
pub const TECH_W: usize = 49;

/// An achievement card's width — three to a row, exactly filling the floor at a
/// one-column gutter. Wide because the condition is a SENTENCE (the mixed
/// categories run to 45 characters) and clipping the sentence would leave two
/// cards reading identically.
pub const GOAL_W: usize = 66;

/// A building-site card's width on the INDEX — four to a row, its four spaces
/// in two columns of two.
pub const SITE_W: usize = 49;

/// Lines per mini order card.
pub const CARD_H: usize = 6;

/// Card interior text width: two columns of border and one of pad either side.
pub const CARD_INNER: usize = CARD_W - 4;

/// Columns the title line spends before the id annotation closes it: the id
/// field costs ` Oxx ┐`, six columns, whatever the title did.
const TITLE_W: usize = CARD_W - 6;

/// One card as a surface wants it drawn: the printed card, plus the three
/// things the two callers differ on.
///
/// The `note` is deliberately a caller-supplied string rather than a flag:
/// the hand panel's is `"your starting order"` and the index's is a WORDED
/// [`crate::screen_model::CardLocation`], and neither renderer should have to
/// know the other's vocabulary to draw a card.
pub struct CardFace<'a> {
    pub id: OrderCardId,
    /// The printed card, or `None` for an id the registry does not know.
    pub printed: Option<&'a OrderCard>,
    /// The number typed to select this card, shown on the title line.
    pub selection: Option<usize>,
    /// The last interior line, when the surface has something to say about
    /// this card. Replaces the rewards' overflow row, which is the mocks' own
    /// rule — a card that both pays more than one line of rewards AND carries a
    /// note shows the note.
    pub note: Option<String>,
    /// Draw the whole card faint and uncolored: it is already recorded
    /// elsewhere and cannot be entered here.
    pub dim: bool,
}

impl<'a> CardFace<'a> {
    /// A plain card: no selection number, no note, not dimmed.
    pub fn new(id: OrderCardId, printed: Option<&'a OrderCard>) -> Self {
        CardFace { id, printed, selection: None, note: None, dim: false }
    }

    pub fn selection(mut self, n: Option<usize>) -> Self {
        self.selection = n;
        self
    }

    pub fn note(mut self, note: Option<String>) -> Self {
        self.note = note;
        self
    }

    pub fn dim(mut self, dim: bool) -> Self {
        self.dim = dim;
        self
    }
}

/// One mini order card, every line exactly [`CARD_W`] columns:
///
/// ```text
/// ┌ [3] FRANCE       O02 ┐
/// │ 1 Seafood 2 Tea      │
/// │                      │
/// │ ★3 VP                │
/// │ = Port A slot 1      │
/// └──────────────────────┘
/// ```
pub fn order_card(face: &CardFace<'_>) -> Vec<String> {
    let id = format!("O{:02}", face.id.0);
    let tone = |t: Tone| if face.dim { Tone::Dim } else { t };

    let Some(printed) = face.printed else {
        // An id the registry does not know. Draw the box with the id in it
        // rather than a card's worth of invented content.
        let mut top = Twin::default();
        top.raw(&format!("{} ", glyph::BOX_TL));
        push_selection(&mut top, face, tone(Tone::Strong));
        top.tinted(tone(Tone::Chrome), &id);
        let mut unknown = Twin::default();
        unknown.tinted(Tone::Dim, "not in the registry");
        let mut rows = vec![card_top(&mut top), card_interior(&mut unknown)];
        while rows.len() < CARD_H - 1 {
            rows.push(card_interior(&mut Twin::default()));
        }
        rows.push(card_bottom());
        return rows;
    };

    let mut top = Twin::default();
    top.raw(&format!("{} ", glyph::BOX_TL));
    push_selection(&mut top, face, tone(Tone::Strong));
    // Bounded BEFORE it is painted: what is left of the title line after the
    // border and any selection tag. Every country name in the registry fits
    // (`NETHERLANDS` at a two-digit tag is the exact-fit case), so this is a
    // guarantee rather than a behaviour — but it is the guarantee that keeps
    // the closing `┐` on the card, which a whole grid row is aligned on.
    let room = TITLE_W.saturating_sub(top.width());
    let name: String =
        describe::country_name(printed.country).to_uppercase().chars().take(room).collect();
    top.tinted(tone(Tone::Flag(printed.country)), &name);

    let needs = requirement_tokens(&printed.requirements, face.dim);
    let pays = reward_tokens(&printed.rewards, face.dim);

    let mut rows = vec![card_top_with_id(&mut top, &id, tone(Tone::Chrome))];
    rows.push(card_interior(&mut token_row(&needs, 0, 2)));
    rows.push(card_interior(&mut token_row(&needs, 2, 4)));
    rows.push(card_interior(&mut token_row(&pays, 0, 2)));
    match &face.note {
        Some(note) => {
            let mut t = Twin::default();
            t.tinted(if face.dim { Tone::Dim } else { Tone::Chrome }, note);
            rows.push(card_interior(&mut t));
        }
        None => rows.push(card_interior(&mut token_row(&pays, 2, 4))),
    }
    rows.push(card_bottom());
    rows
}

/// `[3] ` on the title line, or nothing when the card cannot be selected.
fn push_selection(t: &mut Twin, face: &CardFace<'_>, tone: Tone) {
    if let Some(n) = face.selection {
        t.tinted(tone, &format!("[{n}]")).raw(" ");
    }
}

// ---------------------------------------------------------------------------
// The other three registries (T6)
// ---------------------------------------------------------------------------

/// Which surface a card is being drawn onto.
///
/// **The two surfaces differ in what an ABSENCE means, which is the only thing
/// this discriminates** (T7 review M1). On the index, a card carrying no
/// per-seat progress is a card that is not on the table — the screen says so,
/// and saying so is the whole reason the registry face exists. In the ECHO the
/// same absence is an artifact of where the card came from: an echo is always
/// assembled from the registry ([`crate::screen_model::index_card_for`]), so
/// its progress list is empty by construction, and the index's sentence
/// ("not in play") is then a statement about the card the operator has just
/// finished recording as in play.
///
/// One parameter rather than a second entry point: this module's promise is
/// that a card LOOKS the same wherever a player meets it, and two dispatchers
/// differing by one flag is how that promise stops being true.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardContext {
    /// The full-screen index grid, or any surface drawing a card as it stands.
    Index,
    /// The card drawn back after an entry commits (`card_prompt::echo`).
    Echo,
}

/// One card of ANY registry, at that registry's own width.
///
/// The one entry point the index renderer draws through, so "which card shape"
/// is decided once, from the model's own face, rather than by a renderer
/// branching on a kind it was told separately. The match has no `_` arm: a fifth
/// registry cannot reach a screen without a card being designed for it.
///
/// `colors` is the seat color table, needed only by the faces that name a
/// player (an achievement's claimants and progress). Everything else ignores it.
/// `context` is [`CardContext`] — read its doc before adding a second use of it.
pub fn index_card(card: &IndexCard, colors: &[PlayerColor], context: CardContext) -> Vec<String> {
    match &card.face {
        CardFaceModel::Order { id, printed } => order_card(
            &CardFace::new(*id, printed.as_ref())
                .selection(card.selection)
                .note(card.location.map(|at| format!("= {}", describe::card_location(at))))
                .dim(card.location.is_some()),
        ),
        CardFaceModel::Tech { copies, printed, placed } => {
            tech_card(copies, printed, placed, card.selection, card.location.is_some())
        }
        CardFaceModel::Achievement(a) => {
            achievement_card(a, card.selection, card.location.is_some(), colors, context)
        }
        CardFaceModel::Site { index, site } => {
            site_index_card(*index, site, card.selection, card.location, card.location.is_some())
        }
    }
}


/// One technology TITLE: name and cost on the title line, the purchase text
/// underneath, and what is known about its copies on the last interior line.
///
/// **The effect text is the point of this card** (spec §0.6: "techs lead with
/// names and effects"). `describe::tech_effect_text` is its single source — the
/// same string the `labs` offer prints — and it is WRAPPED rather than clipped,
/// because F2 made every qualifier in those lines load-bearing: the cost a card
/// still charges, the restriction it does not lift. A card that clipped
/// `(normal placement fee applies)` would be teaching the defect that review
/// removed.
fn tech_card(
    copies: &[yokohama_core::TechCardId],
    printed: &yokohama_core::TechCard,
    placed: &[(yokohama_core::TechCardId, CardLocation)],
    selection: Option<usize>,
    dim: bool,
) -> Vec<String> {
    let tone = |t: Tone| if dim { Tone::Dim } else { t };
    let ids: Vec<String> = copies.iter().map(|id| format!("T{:02}", id.0)).collect();
    let ids = ids.join("/");

    let mut top = Twin::default();
    top.raw(&format!("{} ", glyph::BOX_TL));
    if let Some(n) = selection {
        top.tinted(tone(Tone::Strong), &format!("[{n}]")).raw(" ");
    }
    top.tinted(tone(Tone::Strong), &printed.name.to_uppercase());
    top.raw("  ").tinted(tone(Tone::Chrome), &format!("industry {}", printed.industry_cost));
    // The id field closes the line, exactly as an order card's does: padded from
    // the plain width, then one space either side of the tag.
    let want = TECH_W - 2 - ids.chars().count() - 2;
    if top.width() < want {
        top.raw(&" ".repeat(want - top.width()));
    }
    top.raw(" ").tinted(tone(Tone::Chrome), &ids).raw(&format!(" {}", glyph::BOX_TR));

    let mut rows = vec![exact(&top, TECH_W)];
    for line in wrap_words(describe::tech_effect_text(printed.effect), TECH_W - 4) {
        let mut t = Twin::default();
        // The effect is the card's CONTENT, so it takes no tint at all — the
        // small annotations around it are what `Chrome` is for, and a card whose
        // every line was chrome would read as one long parenthetical.
        if dim {
            t.tinted(Tone::Dim, &line);
        } else {
            t.raw(&line);
        }
        rows.push(interior(&t, TECH_W));
    }
    let mut tail = Twin::default();
    tail.tinted(if dim { Tone::Dim } else { Tone::Chrome }, &copy_note(copies, placed));
    rows.push(interior(&tail, TECH_W));
    rows.push(bottom(TECH_W));
    rows
}

/// What is known about a title's physical copies: how many there are while none
/// is accounted for, and where they went as they are.
///
/// **The half-placed case is why this is not just a location string.** Ten
/// titles circulate as two cards, so "Telegram is in Lab A" is only ever half an
/// answer — the other copy may still be in the deck, in which case the card must
/// stay live and enterable. Saying `1 of 2 seen` is what lets a player trust the
/// screen when they are holding the second one.
fn copy_note(
    copies: &[yokohama_core::TechCardId],
    placed: &[(yokohama_core::TechCardId, CardLocation)],
) -> String {
    let total = copies.len();
    match placed.len() {
        0 => {
            if total == 1 {
                "1 copy in the deck".to_string()
            } else {
                format!("{total} copies in the deck")
            }
        }
        seen if seen < total => format!(
            "{seen} of {total} seen: {}",
            describe::card_location(placed[0].1),
        ),
        _ => {
            let first = describe::card_location(placed[0].1);
            if placed.len() > 1 {
                format!("= {first} +{} more", placed.len() - 1)
            } else {
                format!("= {first}")
            }
        }
    }
}

/// One achievement: its condition as a sentence, what it pays first and later,
/// who has claimed it, and where every seat stands.
///
/// **The progress row is dropped in an ECHO with nothing to report** (T7 review
/// M1) — see [`CardContext`] for why the same empty list means two different
/// things on the two surfaces. The condition is the CONJUNCTION and not the
/// context alone: an echo that did carry live progress would be worth printing,
/// so what is suppressed is a row with nothing in it rather than a row.
fn achievement_card(
    card: &AchievementModel,
    selection: Option<usize>,
    dim: bool,
    colors: &[PlayerColor],
    context: CardContext,
) -> Vec<String> {
    let tone = |t: Tone| if dim { Tone::Dim } else { t };

    let mut top = Twin::default();
    top.raw(&format!("{} ", glyph::BOX_TL));
    if let Some(n) = selection {
        top.tinted(tone(Tone::Strong), &format!("[{n}]")).raw(" ");
    }
    top.tinted(tone(Tone::Strong), &describe::achievement_desc(card.achievement_type));
    top.raw(" ");
    let fill = (GOAL_W - 1).saturating_sub(top.width());
    top.raw(&String::from(glyph::RULE).repeat(fill));
    top.raw(&glyph::BOX_TR.to_string());

    // VP tiers and claim state on one line: the two facts a player weighs
    // together ("worth 7 to me, 4 if someone beat me to it").
    let mut pay = Twin::default();
    pay.tinted(tone(Tone::Vp), &format!("first {}{} VP", glyph::VP, card.first_place_vp));
    pay.raw(&format!("  {}  ", glyph::SEPARATOR));
    pay.tinted(tone(Tone::Vp), &format!("later {}{} VP", glyph::VP, card.subsequent_vp));
    pay.raw(&format!("  {}  ", glyph::SEPARATOR));
    // MORE than one player may claim the same card — the first at
    // `first_place_vp`, each later one at `subsequent_vp` — so every claimant is
    // named while naming them still fits. Past two the names stop fitting beside
    // the VP columns, and a count is more honest than a list clipped mid-word
    // (the rule `panel_render::achievement_line` already applies).
    match card.claimed_by.len() {
        0 => {
            pay.tinted(Tone::Dim, "unclaimed");
        }
        n if n <= 2 => {
            pay.raw("claimed by ");
            for (i, &seat) in card.claimed_by.iter().enumerate() {
                if i > 0 {
                    pay.raw(", ");
                }
                match color_of(colors, seat) {
                    Some(c) => pay.tinted(tone(Tone::Player(c)), crate::render::color_name(c)),
                    None => pay.raw(&format!("seat {}", seat.0)),
                };
            }
        }
        n => {
            pay.raw(&format!("claimed by {n} players"));
        }
    }

    if card.progress.is_empty() && context == CardContext::Echo {
        return vec![exact(&top, GOAL_W), interior(&pay, GOAL_W), bottom(GOAL_W)];
    }

    let mut progress = Twin::default();
    if card.progress.is_empty() {
        progress.tinted(Tone::Dim, "not in play — no progress to show");
    } else {
        for (i, &(seat, current, needed)) in card.progress.iter().enumerate() {
            if i > 0 {
                progress.raw(&format!("  {}  ", glyph::SEPARATOR));
            }
            let text = format!("{current}/{needed}");
            match color_of(colors, seat) {
                Some(c) => {
                    progress.tinted(tone(Tone::Player(c)), crate::render::color_name(c)).raw(" ");
                    progress.tinted(tone(Tone::Player(c)), &text)
                }
                None => progress.raw(&format!("seat {} {text}", seat.0)),
            };
        }
    }

    vec![exact(&top, GOAL_W), interior(&pay, GOAL_W), interior(&progress, GOAL_W), bottom(GOAL_W)]
}

/// One building-site card as the INDEX shows it: the four spaces in two columns,
/// then the trading house.
///
/// **Abbreviated space keys here, full words in the `sites` view**, and the
/// split is deliberate rather than an inconsistency. This screen is a
/// twenty-four-card grid whose job is to let an operator match the card in their
/// hand at setup, so it spends its width on the REWARDS; the view has one card
/// per area and four lines to spend, so it spells `top-left` out. Both read the
/// same `SpaceReward` through [`space_reward_tokens`], so the two can
/// differ in layout and never in content.
fn site_index_card(
    index: u8,
    site: &yokohama_core::BuildingSite,
    selection: Option<usize>,
    location: Option<CardLocation>,
    dim: bool,
) -> Vec<String> {
    let tone = |t: Tone| if dim { Tone::Dim } else { t };

    let mut top = Twin::default();
    top.raw(&format!("{} ", glyph::BOX_TL));
    if let Some(n) = selection {
        top.tinted(tone(Tone::Strong), &format!("[{n}]")).raw(" ");
    }
    top.tinted(tone(Tone::Strong), &format!("SITE {index}"));
    top.raw(" ");
    let fill = (SITE_W - 1).saturating_sub(top.width());
    top.raw(&String::from(glyph::RULE).repeat(fill));
    top.raw(&glyph::BOX_TR.to_string());

    let column = (SITE_W - 4) / 2;
    let mut rows = vec![exact(&top, SITE_W)];
    for pair in [(0usize, 1usize), (2, 3)] {
        let mut t = Twin::default();
        for (i, slot) in [pair.0, pair.1].into_iter().enumerate() {
            if i > 0 {
                t.pad_to(column);
            }
            t.tinted(tone(Tone::Chrome), &format!("{} ", space_key(slot)));
            push_space_reward(&mut t, &site.spaces[slot], dim);
        }
        rows.push(interior(&t, SITE_W));
    }

    let mut house = Twin::default();
    house.raw("trading house  ");
    house.tinted(tone(Tone::Vp), &format!("{}{} VP", glyph::VP, site.trading_house_reward));
    if let Some(at) = location {
        house.raw("  ").tinted(Tone::Dim, &format!("= {}", describe::card_location(at)));
    }
    rows.push(interior(&house, SITE_W));
    rows.push(bottom(SITE_W));
    rows
}

/// `TL`/`TR`/`ML`/`MR` — the printed card's own space keys, as
/// `09-building-sites.yaml` names them.
fn space_key(slot: usize) -> &'static str {
    match slot {
        0 => "TL",
        1 => "TR",
        2 => "ML",
        3 => "MR",
        _ => "??",
    }
}

/// A content line inside a box of `width`.
pub(crate) fn interior(t: &Twin, width: usize) -> String {
    format!("{} {} {}", glyph::BOX_V, exact(t, width - 4), glyph::BOX_V)
}

/// A box's closing edge at `width`.
pub(crate) fn bottom(width: usize) -> String {
    format!("{}{}{}", glyph::BOX_BL, String::from(glyph::RULE).repeat(width - 2), glyph::BOX_BR)
}

/// Greedy word wrap at `width` columns, never breaking a word that fits.
///
/// A word LONGER than the line is broken rather than allowed to overrun: the
/// twenty effect texts hold no such word today, and a renderer whose totality
/// depends on that staying true is a renderer waiting for a rules correction.
/// Always returns at least one line, so a card's height never depends on whether
/// its text was empty.
pub(crate) fn wrap_words(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let mut word = word;
        while word.chars().count() > width {
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            let head: String = word.chars().take(width).collect();
            let taken = head.len();
            lines.push(head);
            word = &word[taken..];
        }
        let need = if line.is_empty() { word.chars().count() } else { line.chars().count() + 1 + word.chars().count() };
        if need > width && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

/// Cards of `kind` at their own width and lines-per-row gutter.
pub fn card_width(kind: IndexKind) -> usize {
    match kind {
        IndexKind::Orders => CARD_W,
        IndexKind::Techs => TECH_W,
        IndexKind::Achievements => GOAL_W,
        IndexKind::Sites => SITE_W,
    }
}

/// `┌ FRANCE           O02 ┐` — the title padded so the engine id closes the
/// line as a small annotation (spec §0.6: ids are shown, never required).
///
/// **Padded exactly, never by [`Twin::pad_to`], and the difference is a whole
/// column.** `pad_to` always advances by at least one space, which is the right
/// rule for two fields that would otherwise run together — and the wrong one
/// here, because the id field brings its own leading space. A title that lands
/// exactly on its column (`┌ [28] NETHERLANDS`, 18 of the 18 available) would
/// gain a 19th column from that minimum and push the closing `┐` off the card.
/// The name is bounded before it is painted instead (see [`TITLE_W`]), so
/// nothing here ever has to clip a line that already carries escapes.
fn card_top_with_id(t: &mut Twin, id: &str, tone: Tone) -> String {
    if t.width() < TITLE_W {
        t.raw(&" ".repeat(TITLE_W - t.width()));
    }
    t.raw(" ").tinted(tone, id).raw(&format!(" {}", glyph::BOX_TR));
    exact(t, CARD_W)
}

fn card_top(t: &mut Twin) -> String {
    t.pad_to(CARD_W - 1);
    t.raw(&glyph::BOX_TR.to_string());
    exact(t, CARD_W)
}

fn card_interior(t: &mut Twin) -> String {
    interior(t, CARD_W)
}

fn card_bottom() -> String {
    bottom(CARD_W)
}

/// Tokens `[from, to)` of a card's run, space-joined — the two-line split the
/// card shape gives requirements and rewards alike.
fn token_row(tokens: &[(String, Tone)], from: usize, to: usize) -> Twin {
    let mut t = Twin::default();
    for (i, (text, tone)) in tokens.iter().enumerate().take(to).skip(from) {
        if i > from {
            t.raw(" ");
        }
        t.tinted(*tone, text);
    }
    t
}

/// `count` with `noun` pluralized by a trailing `s`. Every noun the card and
/// the dashboard count pluralizes that way.
pub(crate) fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("{count} {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// A card's goods requirement as colored tokens, in the board's own token order
/// (seafood, tea, silk, copper — [`crate::board_render`]'s site strip reads the
/// same way, so a card and a tile never disagree about which good comes first).
pub(crate) fn requirement_tokens(r: &TradeGoodRequirement, dim: bool) -> Vec<(String, Tone)> {
    [
        (r.seafood, "Seafood", GoodKind::Seafood),
        (r.tea, "Tea", GoodKind::Tea),
        (r.silk, "Silk", GoodKind::Silk),
        (r.copper, "Copper", GoodKind::Copper),
    ]
    .into_iter()
    .filter(|&(n, _, _)| n > 0)
    .map(|(n, label, kind)| (format!("{n} {label}"), tint(Tone::Good(kind), dim)))
    .collect()
}

/// What completing a card pays, as colored tokens in a fixed order so the same
/// card always reads the same way.
pub(crate) fn reward_tokens(r: &OrderRewards, dim: bool) -> Vec<(String, Tone)> {
    let mut out: Vec<(String, Tone)> = Vec::new();
    if r.victory_points > 0 {
        out.push((format!("{}{} VP", glyph::VP, r.victory_points), tint(Tone::Vp, dim)));
    }
    if r.money > 0 {
        out.push((
            format!("{}{}", glyph::YEN, r.money),
            tint(Tone::Good(GoodKind::Yen), dim),
        ));
    }
    if r.imported_goods > 0 {
        out.push((
            plural(r.imported_goods as usize, "Import"),
            tint(Tone::Good(GoodKind::Imports), dim),
        ));
    }
    for (n, label, kind) in [
        (r.seafood, "Seafood", GoodKind::Seafood),
        (r.tea, "Tea", GoodKind::Tea),
        (r.silk, "Silk", GoodKind::Silk),
        (r.copper, "Copper", GoodKind::Copper),
        (r.any_trade_good, "Any", GoodKind::Any),
    ] {
        if n > 0 {
            out.push((format!("{n} {label}"), tint(Tone::Good(kind), dim)));
        }
    }
    if r.movement > 0 {
        out.push((format!("{} move", r.movement), tint(Tone::ActionIcon, dim)));
    }
    if r.warehouse_tokens > 0 {
        out.push((
            format!("{}{}", glyph::BUILT, r.warehouse_tokens),
            tint(Tone::ActionIcon, dim),
        ));
    }
    out
}

/// A building-site space's printed reward as FULL-WORD tokens, each in the
/// colour of the thing it pays — the `sites` surfaces' vocabulary (approved mock
/// `sites-view.ans`: `3 Seafood`, `2 any goods`, `blank`).
///
/// **The deliberate contrast with `board_render`'s site strip**, which prints
/// the same reward as `f3`: a tile has twenty-six columns for four spaces and a
/// legend underneath, while these surfaces give a space a line of its own and
/// exist precisely so nothing has to be decoded. Both read the same
/// `SpaceReward` in the same fixed order, so a card and a tile can differ in
/// vocabulary but never in content.
///
/// Multi-part rewards (`{vp: 2, yen: 1}` is printed on four of the twenty-four
/// cards) emit both tokens, so a space that pays two things says so.
pub(crate) fn space_reward_tokens(r: &SpaceReward, dim: bool) -> Vec<(String, Tone)> {
    let mut parts: Vec<(String, Tone)> = Vec::new();
    if r.vp > 0 {
        parts.push((format!("{} VP", r.vp), tint(Tone::Vp, dim)));
    }
    if r.yen > 0 {
        parts.push((format!("{} yen", r.yen), tint(Tone::Good(GoodKind::Yen), dim)));
    }
    for (n, one, many, tone) in [
        (r.seafood, "Seafood", "Seafood", Tone::Good(GoodKind::Seafood)),
        (r.tea, "Tea", "Tea", Tone::Good(GoodKind::Tea)),
        (r.silk, "Silk", "Silk", Tone::Good(GoodKind::Silk)),
        (r.copper, "Copper", "Copper", Tone::Good(GoodKind::Copper)),
        (r.any_good, "any good", "any goods", Tone::Good(GoodKind::Any)),
        (r.movement, "assistant move", "assistant moves", Tone::ActionIcon),
        (r.warehouse_tokens, "warehouse take", "warehouse takes", Tone::ActionIcon),
    ] {
        if n > 0 {
            let word = if n == 1 { one } else { many };
            parts.push((format!("{n} {word}"), tint(tone, dim)));
        }
    }
    if parts.is_empty() {
        // Every card in the set prints one blank space (the ML one), so this is
        // a real face rather than a defensive arm.
        parts.push(("blank".to_string(), Tone::Dim));
    }
    parts
}

/// [`space_reward_tokens`] joined into one plain string — the same words, for
/// the callers that measure rather than paint.
pub fn space_reward_words(r: &SpaceReward) -> String {
    space_reward_tokens(r, false)
        .into_iter()
        .map(|(word, _)| word)
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Append a reward's tokens to `t`, joined by ` + `.
pub(crate) fn push_space_reward(t: &mut Twin, r: &SpaceReward, dim: bool) {
    for (i, (word, tone)) in space_reward_tokens(r, dim).into_iter().enumerate() {
        if i > 0 {
            t.raw(" + ");
        }
        t.tinted(tone, &word);
    }
}

/// A token's tone, or [`Tone::Dim`] on a card that is already recorded
/// elsewhere. Dimming replaces the tint rather than layering over it: SGR has
/// no way to say "faint AND copper", and a dim card that kept its colors would
/// read as selectable at a glance, which is the one thing it is not.
fn tint(tone: Tone, dim: bool) -> Tone {
    if dim {
        Tone::Dim
    } else {
        tone
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::visible_width;
    use crate::style::strip_ansi;
    use yokohama_core::Rules;

    /// Every card in the registry, in every face a surface can ask for, is
    /// exactly [`CARD_W`] columns on every one of its [`CARD_H`] lines.
    #[test]
    fn every_registry_card_measures_exactly_card_w_in_every_face() {
        let rules = Rules::standard();
        let mut seen = 0;
        for printed in rules.orders.iter() {
            seen += 1;
            for face in [
                CardFace::new(printed.id, Some(printed)),
                CardFace::new(printed.id, Some(printed)).selection(Some(36)),
                CardFace::new(printed.id, Some(printed))
                    .selection(Some(28))
                    .note(Some("= Port B slot 6".to_string()))
                    .dim(true),
                CardFace::new(printed.id, None),
            ] {
                let lines = order_card(&face);
                assert_eq!(lines.len(), CARD_H, "card O{:02} line count", printed.id.0);
                for (i, line) in lines.iter().enumerate() {
                    assert_eq!(
                        visible_width(line),
                        CARD_W,
                        "card O{:02} line {i} is {:?}",
                        printed.id.0,
                        strip_ansi(line),
                    );
                }
            }
        }
        assert_eq!(seen, 36, "the standard registry holds 36 order cards");
    }

    /// The tightest title there is: the longest country name, a two-digit
    /// selection tag and a two-digit id, which fill the interior to the last
    /// column. Pinned by NAME because it is the case that would clip first, and
    /// a clip here would silently drop the `┐` a whole grid row is aligned on.
    #[test]
    fn the_widest_card_title_still_measures_exactly_card_w() {
        let rules = Rules::standard();
        let dutch = rules
            .orders
            .iter()
            .find(|c| c.country == yokohama_core::Country::Dutch)
            .expect("the registry has Dutch cards");
        let face = CardFace::new(dutch.id, Some(dutch)).selection(Some(28));
        let title = &order_card(&face)[0];
        assert_eq!(visible_width(title), CARD_W, "{:?}", strip_ansi(title));
        assert!(
            strip_ansi(title).contains("[28] NETHERLANDS"),
            "the tag and the full country name both survive: {:?}",
            strip_ansi(title),
        );
        assert!(
            strip_ansi(title).ends_with(&format!("O{:02} {}", dutch.id.0, glyph::BOX_TR)),
            "the id annotation still closes the line: {:?}",
            strip_ansi(title),
        );
    }

    /// A dimmed card carries no color but its own faintness — the property
    /// that makes "already recorded" readable at a glance rather than a note
    /// you have to find.
    #[test]
    fn a_dimmed_card_paints_only_dim() {
        crate::style::force(true);
        let rules = Rules::standard();
        let printed = rules.orders.iter().next().expect("a card");
        let lines = order_card(
            &CardFace::new(printed.id, Some(printed))
                .note(Some("= Seat 0's dealt card".to_string()))
                .dim(true),
        );
        crate::style::force(false);
        let escapes: Vec<String> = lines
            .iter()
            .flat_map(|l| {
                l.match_indices("\u{1b}[")
                    .map(|(at, _)| {
                        let rest = &l[at + 2..];
                        rest[..rest.find('m').unwrap_or(0)].to_string()
                    })
                    .collect::<Vec<_>>()
            })
            .filter(|code| code != "0")
            .collect();
        assert!(!escapes.is_empty(), "the fixture must actually paint something");
        for code in &escapes {
            assert_eq!(code, "2", "a dimmed card may only emit the faint code: {escapes:?}");
        }
    }

    /// The note takes the last interior line, displacing the rewards' overflow
    /// — the mocks' own rule, and the one place two callers could disagree.
    #[test]
    fn a_note_replaces_the_rewards_overflow_row() {
        let rules = Rules::standard();
        // O24 pays three rewards (7 VP, 2 yen, 1 import), so its run overflows
        // onto the fifth line unless a note claims it.
        let printed = rules.orders.get(OrderCardId(24)).expect("O24 is in the registry");
        assert!(
            reward_tokens(&printed.rewards, false).len() > 2,
            "the fixture card must pay more than one row of rewards",
        );
        let plain = strip_ansi(&order_card(&CardFace::new(printed.id, Some(printed)))[4]);
        assert!(plain.contains("Import"), "without a note the overflow shows: {plain:?}");
        let noted = strip_ansi(
            &order_card(
                &CardFace::new(printed.id, Some(printed)).note(Some("= Port A slot 1".to_string())),
            )[4],
        );
        assert!(noted.contains("= Port A slot 1"), "the note takes the row: {noted:?}");
        assert!(!noted.contains("Import"), "and the overflow gives way: {noted:?}");
    }
}
