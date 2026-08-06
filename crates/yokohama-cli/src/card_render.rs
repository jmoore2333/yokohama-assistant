//! The mini order card — **one emitter, drawn by every surface that draws a
//! card** (Phase-3.5a spec §0.6, plan T5; design authority is the approved mock
//! `2026-08-05-phase3.5a-mocks/order-index.ans` and its `order_card()`).
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

use yokohama_core::{OrderCard, OrderCardId, OrderRewards, TradeGoodRequirement};

use crate::board_render::{exact, Twin};
use crate::describe;
use crate::style::{glyph, GoodKind, Tone};

/// A mini order card's total width, borders included — the mocks' `CARD_W`.
pub const CARD_W: usize = 24;

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
    format!("{} {} {}", glyph::BOX_V, exact(t, CARD_INNER), glyph::BOX_V)
}

fn card_bottom() -> String {
    format!("{}{}{}", glyph::BOX_BL, String::from(glyph::RULE).repeat(CARD_W - 2), glyph::BOX_BR)
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
