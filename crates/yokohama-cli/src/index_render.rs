//! The full-screen card index (Phase-3.5a spec §0.6, requirement R8, plan T5;
//! design authority is the approved mock
//! `2026-08-05-phase3.5a-mocks/order-index.ans`).
//!
//! **What this surface is FOR, because it is not a list of cards.** The
//! physical order cards carry a small corner index nobody hunts for at a table,
//! and tech and achievement cards carry no number at all — so the engine's ids
//! are unusable as an identification mechanism for a person (R8, from the
//! owner's own live-gate attempt: *"not seeing the ID cards or having a lookup
//! table in some way makes it hard to enter"*). This screen inverts it: the
//! player identifies the card by what they SEE — the flag, the goods, the
//! reward — and types the number printed beside it. Ids stay on screen as small
//! annotations for saves and bug reports, and nothing ever requires one.
//!
//! **The fourth module on the R9 seam, held the way its siblings hold it.**
//! This module renders; it never reads game state. Its input is
//! [`crate::screen_model::IndexModel`] — pure data, its locations SUPPLIED by
//! the caller rather than derived (which is what keeps a live game's hidden
//! hands out of it) — plus [`crate::card_render`] for the card itself and
//! [`crate::describe`] for words. `GameState`, `Rules` and `Session` are absent
//! by construction.
//!
//! **Width.** The index is a FULL-SCREEN surface: `screen::wrap` draws no board
//! above it, so the thing it has to fit is the design floor
//! ([`crate::screen::FLOOR_W`], 200 columns) rather than any one count's frame
//! width. That is what buys the mock's eight cards to a row — at the 2-player
//! frame's 198 it would be seven, and the grid would break mid-group for a
//! reason that has nothing to do with the cards.

use yokohama_core::Country;

use crate::board_render::Twin;
use crate::card_render::{self, CardFace, CARD_H, CARD_W};
use crate::describe;
use crate::screen::FLOOR_W;
use crate::screen_model::{IndexCard, IndexFilter, IndexKind, IndexModel};
use crate::style::{glyph, paint, Tone};

/// Columns between two cards on the same row — the mock's own single-space
/// gutter, which is what makes eight 24-column cards fit 200.
const CARD_GAP: usize = 1;

/// Cards drawn side by side, computed from the floor rather than stated: the
/// last card on a row needs no gutter after it, so `n` cards cost
/// `n * CARD_W + (n - 1) * CARD_GAP`.
const CARDS_PER_ROW: usize = (FLOOR_W + CARD_GAP) / (CARD_W + CARD_GAP);

/// The whole index screen: title, context, the flag groups, footer, caret.
pub fn index_view(index: &IndexModel) -> String {
    let mut out = String::new();
    out.push_str(&title_line(index));
    out.push('\n');
    out.push_str(&context_line(index));
    out.push('\n');
    out.push('\n');

    if index.groups.is_empty() {
        out.push_str(&paint(Tone::Dim, "  no cards match that filter — 'f' alone clears it"));
        out.push('\n');
        out.push('\n');
    }
    for group in &index.groups {
        out.push_str(&group_heading(group.country, group.cards.len()));
        out.push('\n');
        for row in group.cards.chunks(CARDS_PER_ROW) {
            for line in card_row(row) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out.push('\n');
    }

    out.push_str(&footer_line(index));
    out.push('\n');
    out.push_str(&paint(Tone::Prompt, "> "));
    out.push('\n');
    out
}

/// `ORDER CARD INDEX — all 36, grouped by flag · small tag = engine id …`
fn title_line(index: &IndexModel) -> String {
    let mut t = Twin::default();
    t.tinted(Tone::Strong, kind_title(index.kind));
    t.raw(&format!(" {} ", glyph::SECTION_DASH));
    match index.filter {
        Some(f) => t.raw(&format!(
            "{} of {}, {}",
            index.shown,
            index.total,
            filter_text(f),
        )),
        None => t.raw(&format!("all {}, grouped by flag", index.total)),
    };
    t.raw("  ");
    t.tinted(
        Tone::Chrome,
        &format!(
            "{} small tag = engine id (for saves & reports; never required)",
            glyph::SEPARATOR,
        ),
    );
    t.painted
}

/// The recording-context header: what this screen is being opened to answer.
/// Without a target it says what the screen IS, which is the honest form for a
/// standing view — and the reason such a view shows no selection numbers.
fn context_line(index: &IndexModel) -> String {
    let text = match index.target {
        Some(target) => format!(
            "Recording: {} {} which card is physically there? Type its [number]. {}",
            describe::recording_target(target),
            glyph::SECTION_DASH,
            filter_help(),
        ),
        None => format!(
            "Open as a view {} nothing is being recorded. {} Enter or 'cancel' returns to your prompt.",
            glyph::SECTION_DASH,
            filter_help(),
        ),
    };
    paint(Tone::Chrome, &text)
}

/// The one line that teaches the filter grammar, in both context forms.
fn filter_help() -> String {
    "'f silk' filters by a good · 'f 8vp' by reward · 'f france' by flag · 'f' clears.".to_string()
}

/// `■ GERMANY  (7 cards)` in the flag's own tint.
fn group_heading(country: Country, cards: usize) -> String {
    let mut t = Twin::default();
    t.tinted(
        Tone::Flag(country),
        &format!("{} {}", glyph::CATEGORY, describe::country_name(country).to_uppercase()),
    );
    t.raw("  ");
    t.tinted(Tone::Chrome, &format!("({})", card_render::plural(cards, "card")));
    t.painted
}

/// One row of cards, as [`CARD_H`] assembled lines.
fn card_row(cards: &[IndexCard]) -> Vec<String> {
    let drawn: Vec<Vec<String>> = cards.iter().map(card_lines).collect();
    (0..CARD_H)
        .map(|i| {
            let mut line = String::new();
            for (j, card) in drawn.iter().enumerate() {
                if j > 0 {
                    line.push_str(&" ".repeat(CARD_GAP));
                }
                line.push_str(&card[i]);
            }
            line
        })
        .collect()
}

fn card_lines(card: &IndexCard) -> Vec<String> {
    card_render::order_card(
        &CardFace::new(card.id, card.printed.as_ref())
            .selection(card.selection)
            .note(card.location.map(|at| format!("= {}", describe::card_location(at))))
            .dim(card.location.is_some()),
    )
}

fn footer_line(index: &IndexModel) -> String {
    let text = if index.cards().any(|c| c.location.is_some()) {
        format!(
            "Dimmed cards are already recorded elsewhere and cannot be entered twice {} each says where it is.",
            glyph::SECTION_DASH,
        )
    } else {
        format!(
            "No card here is recorded anywhere else yet {} any of them can be entered.",
            glyph::SECTION_DASH,
        )
    };
    paint(Tone::Chrome, &text)
}

fn kind_title(kind: IndexKind) -> &'static str {
    match kind {
        IndexKind::Orders => "ORDER CARD INDEX",
    }
}

/// A filter as the title states it. Worded here rather than in the model for
/// the same reason every other word in this crate is (R9).
fn filter_text(filter: IndexFilter) -> String {
    match filter {
        IndexFilter::Good(g) => format!("needing {}", describe::card_good_name(g)),
        IndexFilter::MinVp(n) => format!("paying {}{n} VP or more", glyph::VP),
        IndexFilter::Country(c) => format!("from {}", describe::country_name(c)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::visible_width;
    use crate::screen_model::{build_order_index, CardGood, CardLocation, RecordingTarget};
    use crate::style::strip_ansi;
    use yokohama_core::{OrderCardId, PlayerId, Rules};

    fn view(target: Option<RecordingTarget>, filter: Option<IndexFilter>) -> String {
        let rules = Rules::standard();
        index_view(&build_order_index(&rules, &[], target, filter))
    }

    const A_TARGET: RecordingTarget =
        RecordingTarget::PortSetup { board_b: false, slot: 2, of: 6 };

    /// The whole surface fits the design floor at every shape it can take —
    /// the property that lets it be drawn full-screen with nothing above it.
    #[test]
    fn every_index_line_fits_the_design_floor() {
        for (what, text) in [
            ("a view", view(None, None)),
            ("a recording screen", view(Some(A_TARGET), None)),
            (
                "a filtered screen",
                view(Some(A_TARGET), Some(IndexFilter::Good(CardGood::Silk))),
            ),
            (
                "an empty filter",
                view(Some(A_TARGET), Some(IndexFilter::MinVp(200))),
            ),
        ] {
            for line in text.lines() {
                assert!(
                    visible_width(line) <= FLOOR_W,
                    "{what}: {} columns is past the floor: {:?}",
                    visible_width(line),
                    strip_ansi(line),
                );
            }
        }
    }

    /// The mock's grid: a full flag group of eight cards on ONE row. Pinned as
    /// arithmetic over the constants rather than as a literal, so a card-width
    /// change restates it instead of silently breaking the group.
    #[test]
    fn the_row_holds_the_mocks_eight_cards() {
        assert_eq!(CARDS_PER_ROW, 8, "the mock draws eight 24-column cards to a row");
        // The arithmetic behind that 8, in a `const` block so it is checked at
        // compile time rather than by an assertion the optimizer can fold: one
        // more card would put the row past the floor.
        const { assert!((CARDS_PER_ROW + 1) * (CARD_W + CARD_GAP) - CARD_GAP > FLOOR_W) };
        let text = view(Some(A_TARGET), None);
        let widest = text.lines().map(visible_width).max().expect("lines");
        assert_eq!(widest, 199, "a full eight-card row is the widest line on the screen");
    }

    /// A standing view carries no selection numbers, and a recording screen
    /// carries one per card — F3 from the other side: a number is shown only
    /// where typing it does something.
    #[test]
    fn numbers_appear_only_when_something_is_being_recorded() {
        let recording = strip_ansi(&view(Some(A_TARGET), None));
        assert!(recording.contains("[1] "), "the first card is selectable: {recording:.400}");
        assert!(recording.contains("[36] "), "and so is the last one");
        assert!(recording.contains("Type its [number]"));

        let standing = strip_ansi(&view(None, None));
        assert!(!standing.contains("[1] "), "a view offers no selection: {standing:.400}");
        assert!(standing.contains("nothing is being recorded"));
    }

    /// The whole point of the dim treatment: an already-recorded card says
    /// WHERE it is, on the card, in the words the refusal message would use.
    #[test]
    fn a_recorded_card_says_where_it_is_on_its_own_face() {
        let rules = Rules::standard();
        let placed = [
            (OrderCardId(4), CardLocation::Deal { seat: PlayerId::new(0) }),
            (OrderCardId(1), CardLocation::PortSlot { board_b: false, slot: 0 }),
        ];
        let text = strip_ansi(&index_view(&build_order_index(
            &rules,
            &placed,
            Some(A_TARGET),
            None,
        )));
        assert!(text.contains("= Seat 0's deal"), "{text:.600}");
        assert!(text.contains("= Port A slot 1"), "{text:.600}");
        assert!(text.contains("Dimmed cards are already recorded elsewhere"));
    }

    /// An index with nothing placed must not print the dimmed-cards footer —
    /// a legend for a treatment nothing on screen uses teaches a rule the
    /// player then looks for and cannot find.
    #[test]
    fn the_footer_states_what_this_screen_actually_shows() {
        assert!(strip_ansi(&view(Some(A_TARGET), None)).contains("No card here is recorded"));
    }

    /// Every location the index can print fits ON a card, behind its `= `.
    /// A location that clipped would be the one thing the dim treatment exists
    /// to say, half-said — and it would clip silently, since `exact` is doing
    /// its job. Swept over every seat and every port slot rather than spot-
    /// checked, because the longest one is what matters and it is not obvious
    /// which that is.
    #[test]
    fn card_locations_fit_a_card_face() {
        let room = crate::card_render::CARD_INNER - 2; // the leading "= "
        let mut locations = vec![CardLocation::OwnHand, CardLocation::ThisReveal];
        for seat in 0..4u8 {
            locations.push(CardLocation::Deal { seat: PlayerId::new(seat) });
        }
        for board_b in [false, true] {
            for slot in 0..6 {
                locations.push(CardLocation::PortSlot { board_b, slot });
            }
        }
        for at in locations {
            let text = describe::card_location(at);
            assert!(
                text.chars().count() <= room,
                "{text:?} is {} columns; a card face has room for {room}",
                text.chars().count(),
            );
        }
    }

    /// **The piped protocol stays byte-plain on the new surface too** (spec
    /// §0.12's honesty net). Every other screen in the driver inherits this
    /// from `style::paint` being the identity while color is off; a new
    /// full-screen surface is exactly where a hand-written escape would slip
    /// in, so it is asserted here rather than assumed — with color FORCED ON in
    /// the same test, so the check cannot pass by the renderer painting nothing
    /// at all.
    #[test]
    fn the_index_is_escape_free_with_color_off_and_painted_with_it_on() {
        let plain = view(Some(A_TARGET), None);
        assert!(!plain.contains('\u{1b}'), "piped output must carry no escapes");

        crate::style::force(true);
        let painted = view(Some(A_TARGET), None);
        crate::style::force(false);
        assert!(painted.contains('\u{1b}'), "a real terminal gets color");
        assert_eq!(strip_ansi(&painted), plain, "the two differ only by escapes");
    }

    /// The index as the owner sees it, committed beside the mock it is drawn
    /// from — held as a CHECKED golden rather than dumped once, so it cannot
    /// quietly stop being what the code renders (the T2/T3 pattern).
    ///
    /// The RECORDING form, because that is the screen the redesign exists for:
    /// selection numbers on every card, a context header naming the slot, and
    /// two dimmed cards showing both location shapes. Compare it against
    /// `order-index.ans` directly; the deviations are recorded in the commit.
    ///
    /// Plain text, because a capture full of escapes is a capture nobody can
    /// read in a diff. Regenerate with `UPDATE_T5_CAPTURE=1 cargo test -p
    /// yokohama-cli --lib the_committed_index_capture` after a deliberate
    /// change, and LOOK AT THE DIFF before committing it.
    #[test]
    fn the_committed_index_capture_is_what_the_renderer_draws() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/plans/2026-08-05-phase3.5a-mocks/captures/t5-order-index.txt",
        );
        let rules = Rules::standard();
        let placed = [
            (OrderCardId(4), CardLocation::Deal { seat: PlayerId::new(0) }),
            (OrderCardId(1), CardLocation::PortSlot { board_b: false, slot: 0 }),
        ];
        let rendered = strip_ansi(&index_view(&build_order_index(
            &rules,
            &placed,
            Some(RecordingTarget::PortSetup { board_b: false, slot: 2, of: 6 }),
            None,
        )));

        if std::env::var_os("UPDATE_T5_CAPTURE").is_some() {
            std::fs::create_dir_all(std::path::Path::new(path).parent().expect("a parent"))
                .expect("the captures directory");
            std::fs::write(path, &rendered).expect("write the capture");
        }
        let committed = std::fs::read_to_string(path).expect("the committed capture");
        assert_eq!(
            committed, rendered,
            "the committed order-index capture is stale; regenerate it with UPDATE_T5_CAPTURE=1",
        );
    }

    /// A filter narrows the grid, renumbers it from the base, and says what it
    /// did — including the empty case, which must not render a blank screen
    /// with no way out.
    #[test]
    fn a_filter_narrows_renumbers_and_says_so() {
        let rules = Rules::standard();
        let index =
            build_order_index(&rules, &[], Some(A_TARGET), Some(IndexFilter::Country(Country::Dutch)));
        let text = strip_ansi(&index_view(&index));
        assert_eq!(index.shown, 5, "the Dutch group is five cards");
        assert!(text.contains("5 of 36, from Netherlands"), "{text:.300}");
        assert!(text.contains("[1] "), "the filtered grid renumbers from the base");
        assert!(!text.contains("[6] "), "and stops at what it shows: {text:.900}");

        let empty = strip_ansi(&view(Some(A_TARGET), Some(IndexFilter::MinVp(200))));
        assert!(empty.contains("no cards match that filter"), "{empty:.300}");
        assert!(empty.contains("'f' clears"), "the way out is named on the screen");
    }
}
