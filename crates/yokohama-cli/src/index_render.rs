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

use yokohama_core::{AchievementCategory, PlayerColor};

use crate::board_render::Twin;
use crate::card_render;
use crate::describe;
use crate::screen::FLOOR_W;
use crate::screen_model::{IndexCard, IndexFilter, IndexGroupKey, IndexKind, IndexModel};
use crate::style::{glyph, paint, Tone};

/// Columns between two cards on the same row — the mock's own single-space
/// gutter, which is what makes eight 24-column cards fit 200.
const CARD_GAP: usize = 1;

/// Cards drawn side by side at `width`, computed from the floor rather than
/// stated: the last card on a row needs no gutter after it, so `n` cards cost
/// `n * width + (n - 1) * CARD_GAP`. Never zero — a card wider than the floor
/// still gets a row of its own rather than an empty screen.
fn cards_per_row(width: usize) -> usize {
    ((FLOOR_W + CARD_GAP) / (width + CARD_GAP)).max(1)
}

/// The whole index screen: title, context, the groups, footer, caret.
///
/// `colors` is the seat color table — needed only by the achievement cards,
/// which name claimants and per-seat progress.
pub fn index_view(index: &IndexModel, colors: &[PlayerColor]) -> String {
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
    let per_row = cards_per_row(card_render::card_width(index.kind));
    for group in &index.groups {
        // A single ungrouped run needs no heading — see `IndexGroupKey::All`.
        if let Some(heading) = group_heading(group.key, group.cards.len()) {
            out.push_str(&heading);
            out.push('\n');
        }
        for row in group.cards.chunks(per_row) {
            for line in card_row(row, colors) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out.push('\n');
    }

    out.push_str(&footer_line(index));
    out.push('\n');
    out.push_str(&exit_line());
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
        None => t.raw(&format!("all {}, {}", index.total, grouping_text(index.kind))),
    };
    t.raw("  ");
    t.tinted(Tone::Chrome, &format!("{} {}", glyph::SEPARATOR, id_note(index.kind)));
    t.painted
}

/// How this registry's screen is organized, for the title line.
fn grouping_text(kind: IndexKind) -> &'static str {
    match kind {
        IndexKind::Orders | IndexKind::Techs => "grouped by flag",
        IndexKind::Achievements => "grouped by category",
        IndexKind::Sites => "in card order",
    }
}

/// What the small annotation on each card IS — different per registry, and
/// worth saying, because the whole promise of §0.6 is that the number is never
/// required. On the site index there is no printed number to annotate AT ALL,
/// which is the sharpest form of the same point.
fn id_note(kind: IndexKind) -> &'static str {
    match kind {
        IndexKind::Orders => "small tag = engine id (for saves & reports; never required)",
        IndexKind::Techs => {
            "small tag = engine id, one per physical copy (never required; no number is printed on the card)"
        }
        IndexKind::Achievements => "achievement cards print no number — pick them by what they ask",
        IndexKind::Sites => "SITE n is the number setup entry takes; nothing is printed on the card",
    }
}

/// The recording-context header: what this screen is being opened to answer.
/// Without a target it says what the screen IS, which is the honest form for a
/// standing view — and the reason such a view shows no selection numbers.
///
/// **The no-target branch is scoped to THIS SCREEN, and since T6's review it has
/// to be** (M1). There are now two ways to reach it: nothing is being collected
/// anywhere, or a capture IS live and is asking about a different registry —
/// `techs` typed mid-Port-reveal, which `Repl::index_model` filters to a target
/// of `None` precisely so this screen stops claiming to collect something it
/// cannot. A line reading "nothing is being recorded" would be false in the
/// second case, which is the sentence M1 named. "This screen is recording
/// nothing" is true in both, and the refusal a pick gets is where the live
/// capture's own registry is named.
fn context_line(index: &IndexModel) -> String {
    let text = match index.target {
        Some(target) => format!(
            "Recording: {} {} which card is physically there? Type its [number]. {}",
            describe::recording_target(target),
            glyph::SECTION_DASH,
            filter_help(index.kind),
        ),
        None => format!(
            "Open as a view {} this screen is recording nothing. {}",
            glyph::SECTION_DASH,
            filter_help(index.kind),
        ),
    };
    paint(Tone::Chrome, &text)
}

/// The one line that teaches the filter grammar — per registry, because a
/// screen that taught `f silk` on the site index would be teaching a term that
/// index refuses.
fn filter_help(kind: IndexKind) -> String {
    match kind {
        IndexKind::Orders => {
            "'f silk' filters by a good · 'f 8vp' by reward · 'f france' by flag · 'f' clears."
                .to_string()
        }
        IndexKind::Techs => {
            "Type a name or any prefix of one ('tele' is ambiguous, 'telegram' is not) · 'f france' filters by flag · 'f' clears."
                .to_string()
        }
        IndexKind::Achievements => "'f a' filters to one category · 'f' clears.".to_string(),
        IndexKind::Sites => "Pick by number; this index takes no filters.".to_string(),
    }
}

/// **How to leave, stated on every form of this screen** (T5 review M2, and the
/// approved mock's own `Enter cancels.`).
///
/// The recording form had lost it: the header spent its width on the filter
/// grammar and the footer talked about dimmed cards, so a full-screen overlay
/// that had replaced the player's prompt said nothing at all about getting back
/// to it. It sits BELOW the grid rather than in the header — one line above the
/// caret, where a player looks for the way out, and where it does not have to
/// compete with the recording target and four filter clauses for 200 columns.
///
/// **The wording distinguishes the two `cancel`s, which is the point.** On this
/// screen `cancel` closes an OVERLAY; at the reveal prompt underneath, the same
/// word DISCARDS the group being collected. A player who has just learned the
/// second meaning needs to be told this one is not it.
///
/// **`back` is named for the same reason, and it is the word that differs
/// between the two overlays** (T6 review S1). Here it closes, because it belongs
/// to the card-entry grammar every prompt under this screen shares
/// ([`crate::card_prompt::resolve`]); on a full-screen VIEW it re-dispatches
/// instead and undoes the last applied entry. One word, two surfaces, so the
/// surface that closes on it says so rather than leaving it to be discovered.
fn exit_line() -> String {
    paint(
        Tone::Chrome,
        "Enter, '?', 'cancel' or 'back' closes this index and puts you back at the prompt underneath, \
         exactly as you left it — nothing is entered and nothing there is cancelled.",
    )
}

/// `■ GERMANY  (7 cards)` in the flag's own tint, or `None` for a run that has
/// no dimension to be grouped along.
fn group_heading(key: IndexGroupKey, cards: usize) -> Option<String> {
    let (tone, label) = match key {
        IndexGroupKey::Country(c) => {
            (Tone::Flag(c), describe::country_name(c).to_uppercase())
        }
        IndexGroupKey::Category(cat) => (
            Tone::Strong,
            format!("CATEGORY {} {} {}", card_render::category_letter(cat), glyph::SECTION_DASH, category_gloss(cat)),
        ),
        IndexGroupKey::All => return None,
    };
    let mut t = Twin::default();
    t.tinted(tone, &format!("{} {label}", glyph::CATEGORY));
    t.raw("  ");
    t.tinted(Tone::Chrome, &format!("({})", card_render::plural(cards, "card")));
    Some(t.painted)
}

/// What an achievement category IS, in the rulebook's own terms — the heading
/// carries it because `A` names nothing on its own, and the three categories are
/// the reason exactly three cards are in play.
fn category_gloss(c: AchievementCategory) -> &'static str {
    match c {
        AchievementCategory::A => "trade goods you own",
        AchievementCategory::B => "collections you have gathered",
        AchievementCategory::C => "where you have built",
    }
}

/// One row of cards, padded to the tallest card in the row.
///
/// **Ragged rows, sized per row.** The four registries have four card heights,
/// and within the tech index a card is as tall as its own effect text. A fixed
/// height would have to be the worst case everywhere — three lines of effect
/// text on every technology — and paying for `Gaslight`'s one-line effect as if
/// it were `Telegram`'s three costs that screen five rows it does not have.
fn card_row(cards: &[IndexCard], colors: &[PlayerColor]) -> Vec<String> {
    let drawn: Vec<Vec<String>> = cards.iter().map(|c| card_render::index_card(c, colors)).collect();
    let height = drawn.iter().map(Vec::len).max().unwrap_or(0);
    (0..height)
        .map(|i| {
            let mut line = String::new();
            for (j, card) in drawn.iter().enumerate() {
                if j > 0 {
                    line.push_str(&" ".repeat(CARD_GAP));
                }
                // A card shorter than its row leaves blank columns rather than
                // shifting the cards to its right onto a different pitch.
                match card.get(i) {
                    Some(text) => line.push_str(text),
                    None => line.push_str(&" ".repeat(width_of(card))),
                }
            }
            while line.ends_with(' ') {
                line.pop();
            }
            line
        })
        .collect()
}

/// A drawn card's own width, measured off its bottom edge — the one line every
/// card shape emits at exactly its full width with no escapes in it.
fn width_of(card: &[String]) -> usize {
    card.last().map(|l| crate::screen::visible_width(l)).unwrap_or(0)
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
        IndexKind::Techs => "TECHNOLOGY INDEX",
        IndexKind::Achievements => "ACHIEVEMENTS",
        IndexKind::Sites => "BUILDING-SITE CARD INDEX",
    }
}

/// A filter as the title states it. Worded here rather than in the model for
/// the same reason every other word in this crate is (R9).
fn filter_text(filter: IndexFilter) -> String {
    match filter {
        IndexFilter::Good(g) => format!("needing {}", describe::card_good_name(g)),
        IndexFilter::MinVp(n) => format!("paying {}{n} VP or more", glyph::VP),
        IndexFilter::Country(c) => format!("from {}", describe::country_name(c)),
        IndexFilter::Category(cat) => {
            format!("in category {}", card_render::category_letter(cat))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::visible_width;
    use crate::screen_model::{build_order_index, CardGood, CardLocation, RecordingTarget};
    use yokohama_core::Country;
    use crate::style::strip_ansi;
    use yokohama_core::{OrderCardId, PlayerId, Rules};

    fn view(target: Option<RecordingTarget>, filter: Option<IndexFilter>) -> String {
        let rules = Rules::standard();
        index_view(&build_order_index(&rules, &[], target, filter), &[])
    }

    const A_TARGET: RecordingTarget =
        RecordingTarget::PortSetup { board_b: false, slot: 2, of: 6 };

    const TECH_TARGET: RecordingTarget = RecordingTarget::TechRefill { board_b: false };

    /// Every registry's screen, at both of the shapes it can be opened in —
    /// what the per-kind sweeps below run over.
    fn every_index(target: Option<RecordingTarget>) -> Vec<(IndexKind, String)> {
        let rules = Rules::standard();
        let state = a_state();
        let colors: Vec<yokohama_core::PlayerColor> =
            state.players.iter().map(|p| p.color).collect();
        vec![
            (
                IndexKind::Orders,
                index_view(&build_order_index(&rules, &[], target, None), &colors),
            ),
            (
                IndexKind::Techs,
                index_view(
                    &crate::screen_model::build_tech_index(&rules, &[], target, None),
                    &colors,
                ),
            ),
            (
                IndexKind::Achievements,
                index_view(
                    &crate::screen_model::build_achievement_index(
                        &crate::screen_model::build_achievements(&state).cards,
                        &[],
                        target,
                        None,
                    ),
                    &colors,
                ),
            ),
            (
                IndexKind::Sites,
                index_view(
                    &crate::screen_model::build_site_index(&rules, &[], target, None),
                    &colors,
                ),
            ),
        ]
    }

    /// A real 2-player game, for the achievement index's live claim state and
    /// per-seat progress.
    fn a_state() -> yokohama_core::GameState {
        let rules = Rules::standard();
        let pending =
            yokohama_core::new_game(yokohama_core::GameConfig::for_player_count(2), &rules, 1)
                .expect("a standard board");
        yokohama_core::finalize_setup(
            pending,
            &[yokohama_core::StartingOrderChoice::Keep(0); 2],
        )
        .expect("setup finalizes")
    }

    /// **Every registry's screen fits the design floor and the 50-row height**,
    /// in both the standing-view and the recording shape. The tech index is the
    /// tall one — twenty titles at four to a row, with three lines of effect
    /// text on the longest — so the height clause is what keeps its own card
    /// geometry honest.
    #[test]
    fn every_registry_fits_the_design_floor_in_both_shapes() {
        for target in [None, Some(A_TARGET)] {
            for (kind, text) in every_index(target) {
                for line in text.lines() {
                    assert!(
                        visible_width(line) <= FLOOR_W,
                        "{kind:?}: {} columns is past the floor: {:?}",
                        visible_width(line),
                        strip_ansi(line),
                    );
                }
                let rows = text.lines().count();
                assert!(rows <= 50, "{kind:?} is {rows} rows, past the 50-row design floor");
            }
        }
    }

    /// **One card per TECHNOLOGY TITLE, not per physical card**, with the
    /// purchase text `describe::tech_effect_text` owns — spec §0.6's "techs lead
    /// with names and effects", and the reason the tech index is not simply the
    /// order index pointed at another registry: ten of the twenty titles
    /// circulate as two identical cards, and a screen that drew both would ask
    /// the player to tell apart two pieces of cardboard the printer did not.
    #[test]
    fn the_tech_index_shows_titles_with_their_full_purchase_text() {
        let rules = Rules::standard();
        let model = crate::screen_model::build_tech_index(&rules, &[], Some(TECH_TARGET), None);
        assert_eq!(model.total, 20, "twenty titles from thirty physical cards");
        let text = strip_ansi(&index_view(&model, &[]));

        let mut seen = std::collections::BTreeSet::new();
        for card in rules.techs.iter() {
            assert!(
                text.contains(&card.name.to_uppercase()),
                "every title is on screen: {}",
                card.name,
            );
            // The effect text, wrapped rather than clipped: every word of it
            // survives somewhere on the screen (F2 made the qualifiers
            // load-bearing, so a clipped `(normal placement fee applies)` would
            // be teaching the defect that review removed).
            for word in describe::tech_effect_text(card.effect).split_whitespace() {
                assert!(text.contains(word), "{:?} lost the word {word:?}", card.name);
            }
            assert!(
                text.contains(&format!("industry {}", card.industry_cost)),
                "the gear cost a purchase is priced on: {}",
                card.name,
            );
            seen.insert(card.name.clone());
        }
        assert_eq!(seen.len(), 20);
        // Both engine ids of a two-copy title, as the small annotation.
        assert!(text.contains("T01/T02"), "a two-copy title shows both ids: {text:.400}");
    }

    /// A title with ONE copy accounted for stays live and says so; a title with
    /// BOTH accounted for dims. The half-placed case is the one a naive
    /// implementation gets wrong, and getting it wrong refuses a true answer:
    /// the player really can be holding the second Telegram.
    #[test]
    fn a_tech_title_dims_only_once_every_copy_is_accounted_for() {
        let rules = Rules::standard();
        let mut copies: Vec<yokohama_core::TechCardId> = rules
            .techs
            .iter()
            .filter(|c| c.name == "Telegram")
            .map(|c| c.id)
            .collect();
        copies.sort_unstable_by_key(|c| c.0);
        assert_eq!(copies.len(), 2, "fixture: Telegram circulates as two cards");

        let half = crate::screen_model::build_tech_index(
            &rules,
            &[(copies[0], CardLocation::LabSlot { board_b: false, slot: 2 })],
            Some(TECH_TARGET),
            None,
        );
        let card = half
            .cards()
            .find(|c| matches!(&c.face, crate::screen_model::CardFaceModel::Tech { printed, .. } if printed.name == "Telegram"))
            .expect("Telegram is on screen");
        assert!(card.location.is_none(), "one copy left, so the card stays live");
        assert_eq!(
            card.face.card_ref(),
            Some(crate::screen_model::CardRef::Tech(copies[1])),
            "and a selection resolves to the copy nobody has claimed",
        );
        assert!(
            strip_ansi(&index_view(&half, &[])).contains("1 of 2 seen: Laboratory A slot 3"),
            "the screen says which copy went where",
        );

        let both = crate::screen_model::build_tech_index(
            &rules,
            &[
                (copies[0], CardLocation::LabSlot { board_b: false, slot: 2 }),
                (copies[1], CardLocation::OwnedBy { seat: PlayerId::new(1) }),
            ],
            Some(TECH_TARGET),
            None,
        );
        let card = both
            .cards()
            .find(|c| matches!(&c.face, crate::screen_model::CardFaceModel::Tech { printed, .. } if printed.name == "Telegram"))
            .expect("Telegram is on screen");
        assert!(card.location.is_some(), "both copies accounted for, so the card dims");
        assert!(
            strip_ansi(&index_view(&both, &[])).contains("= Laboratory A slot 3 +1 more"),
            "and it says both are spoken for",
        );
    }

    /// **The achievement index carries every fact the retired
    /// `render::achievements_view` carried** — the migration half of T6's
    /// retirement census: the printed condition, the first/subsequent claim VP
    /// from the registry, the claim state, and every seat's live progress from
    /// the engine's own `achievement_progress`.
    #[test]
    fn the_achievement_index_carries_condition_claim_vp_and_every_seats_progress() {
        let state = a_state();
        let colors: Vec<yokohama_core::PlayerColor> =
            state.players.iter().map(|p| p.color).collect();
        let panel = crate::screen_model::build_achievements(&state);
        let text =
            strip_ansi(&index_view(&crate::screen_model::build_achievement_index(&panel.cards, &[], None, None), &colors));

        assert_eq!(panel.cards.len(), 3, "one card per category is in play");
        for card in &panel.cards {
            assert!(
                text.contains(&describe::achievement_desc(card.achievement_type)),
                "the printed condition: {text}",
            );
            assert!(
                text.contains(&format!("first ★{} VP", card.first_place_vp))
                    && text.contains(&format!("later ★{} VP", card.subsequent_vp)),
                "both claim tiers from the registry: {text}",
            );
            for &(seat, current, needed) in &card.progress {
                let name = crate::render::color_name(colors[usize::from(seat.0)]);
                assert!(
                    text.contains(&format!("{name} {current}/{needed}")),
                    "every seat's progress, from the engine's own arithmetic: {text}",
                );
            }
        }
        assert!(text.contains("unclaimed"), "the claim state on an untouched board: {text}");
        for letter in ["CATEGORY A", "CATEGORY B", "CATEGORY C"] {
            assert!(text.contains(letter), "grouped by category: {text}");
        }
    }

    /// A claimed card names its claimant — the other half of the retired view's
    /// claim-state property.
    #[test]
    fn a_claimed_achievement_names_who_took_it() {
        let mut state = a_state();
        let seat = PlayerId::new(1);
        state.achievements[0].claimed_by.push(seat);
        let colors: Vec<yokohama_core::PlayerColor> =
            state.players.iter().map(|p| p.color).collect();
        let panel = crate::screen_model::build_achievements(&state);
        let text =
            strip_ansi(&index_view(&crate::screen_model::build_achievement_index(&panel.cards, &[], None, None), &colors));
        assert!(text.contains("claimed by Blue"), "{text}");
    }

    /// **The achievement index serves BOTH populations from one builder** — the
    /// three cards in play (the `goals` surface) and all twelve types (the shape
    /// a setup prompt needs). Deriving the population inside the builder would
    /// have forced one of the two to be a second screen.
    #[test]
    fn the_achievement_index_also_renders_the_whole_registry_for_setup() {
        let all = crate::screen_model::achievement_registry();
        assert_eq!(all.len(), 12, "twelve achievement types exist");
        let model = crate::screen_model::build_achievement_index(&all, &[], Some(A_TARGET), None);
        assert_eq!(model.total, 12);
        assert_eq!(model.groups.len(), 3, "grouped A/B/C");
        for group in &model.groups {
            assert_eq!(group.cards.len(), 4, "four types per category");
        }
        let text = strip_ansi(&index_view(&model, &[]));
        assert!(text.contains("[1] "), "a recording screen numbers its cards");
        assert!(text.contains("[12] "), "through to the last one: {text:.300}");
        assert!(
            text.contains("not in play"),
            "a registry card has no progress and says so rather than printing zeros: {text:.600}",
        );
    }

    /// The site index shows every printed card by the number setup entry takes,
    /// with all four spaces and the trading house — the identification surface
    /// for a card that prints no number at all.
    #[test]
    fn the_site_index_shows_every_card_by_the_number_setup_takes() {
        let rules = Rules::standard();
        let model = crate::screen_model::build_site_index(&rules, &[], Some(A_TARGET), None);
        assert_eq!(model.total, 24, "twenty-four printed site cards");
        assert_eq!(model.groups.len(), 1, "one ungrouped run");
        let text = strip_ansi(&index_view(&model, &[]));
        for (i, site) in rules.building_sites.iter().enumerate() {
            assert!(text.contains(&format!("SITE {i}")), "card {i} is on screen: {text:.300}");
            assert!(
                text.contains(&format!("★{} VP", site.trading_house_reward)),
                "with its trading-house VP",
            );
        }
        // The wizard's own number, unrenumbered: `NUMBER_BASE` governs SELECTION
        // numbers, and a site index is an identity (see its declaration).
        assert!(text.contains("SITE 0"), "the registry's own 0-based index: {text:.300}");
    }

    /// The TECHNOLOGY index as the owner sees it, committed beside the mock
    /// pattern it is drawn from and held as a CHECKED golden (the T2/T3/T5
    /// pattern). The standing-VIEW form, since it depends on nothing but the
    /// registry and is therefore the same screen on every board.
    ///
    /// Regenerate with `UPDATE_T6_CAPTURE=1 cargo test -p yokohama-cli --lib
    /// the_committed_tech_index_capture`, and LOOK AT THE DIFF.
    #[test]
    fn the_committed_tech_index_capture_is_what_the_renderer_draws() {
        let rules = Rules::standard();
        let rendered = strip_ansi(&index_view(
            &crate::screen_model::build_tech_index(&rules, &[], None, None),
            &[],
        ));
        crate::view_render::tests::assert_capture("t6-tech-index.txt", &rendered);
    }

    /// **Every registry's screen states how to leave it**, in both shapes —
    /// T5's review M2, swept across the three surfaces T6 adds rather than
    /// re-checked only on the one it was found on.
    #[test]
    fn every_registry_states_the_way_out() {
        for target in [None, Some(A_TARGET)] {
            for (kind, text) in every_index(target) {
                let plain = strip_ansi(&text);
                let line = plain
                    .lines()
                    .find(|l| l.contains("closes this index"))
                    .unwrap_or_else(|| panic!("{kind:?} states no way out:\n{plain}"));
                assert!(line.contains("Enter") && line.contains("'cancel'"), "{kind:?}: {line}");
                let lines: Vec<&str> = plain.lines().collect();
                let caret = lines.iter().position(|l| l.trim() == ">").expect("the caret");
                assert_eq!(lines[caret - 1], line, "{kind:?}: the way out sits above the prompt");
            }
        }
    }

    /// **Every registry's screen is byte-plain with color off and painted with
    /// it on** — spec §0.12's honesty net, swept across all four rather than
    /// pinned on the one T5 shipped.
    #[test]
    fn every_registry_is_escape_free_with_color_off_and_painted_with_it_on() {
        let plain = every_index(Some(A_TARGET));
        crate::style::force(true);
        let painted = every_index(Some(A_TARGET));
        crate::style::force(false);
        for ((kind, plain), (_, painted)) in plain.into_iter().zip(painted) {
            assert!(!plain.contains('\u{1b}'), "{kind:?}: piped output must carry no escapes");
            assert!(painted.contains('\u{1b}'), "{kind:?}: a real terminal gets color");
            assert_eq!(strip_ansi(&painted), plain, "{kind:?}: the two differ only by escapes");
        }
    }

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
        use crate::card_render::CARD_W;
        const CARDS_PER_ROW: usize = (FLOOR_W + CARD_GAP) / (CARD_W + CARD_GAP);
        assert_eq!(cards_per_row(CARD_W), 8, "the mock draws eight 24-column cards to a row");
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
        // Scoped to the screen, because a live capture asking about a DIFFERENT
        // registry reaches this same branch (T6 review M1 — see `context_line`).
        assert!(standing.contains("this screen is recording nothing"), "{standing:.400}");
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
        let text = strip_ansi(&index_view(
            &build_order_index(&rules, &placed, Some(A_TARGET), None),
            &[],
        ));
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

    /// **Every form of this screen says how to leave it** (T5 review M2). The
    /// RECORDING form is the one that had lost the clause, and it is the form
    /// that needs it most: it appears over a prompt the player was in the middle
    /// of answering, so "how do I get back?" is the first question a full-screen
    /// takeover raises.
    ///
    /// Swept over every shape the screen takes — including the empty filter,
    /// where a player with nothing on screen has the fewest other cues — and the
    /// wording is checked for the DISTINCTION as well as the keys: `cancel` here
    /// closes the overlay, while `cancel` at the reveal prompt underneath
    /// discards the group being collected, and a line that said only "cancel
    /// returns" would leave a player who had learned the second meaning guessing.
    #[test]
    fn every_form_of_the_index_states_the_way_out() {
        for (what, text) in [
            ("a view", view(None, None)),
            ("a recording screen", view(Some(A_TARGET), None)),
            (
                "a filtered screen",
                view(Some(A_TARGET), Some(IndexFilter::Good(CardGood::Silk))),
            ),
            ("an empty filter", view(Some(A_TARGET), Some(IndexFilter::MinVp(200)))),
        ] {
            let plain = strip_ansi(&text);
            let line = plain
                .lines()
                .find(|l| l.contains("closes this index"))
                .unwrap_or_else(|| panic!("{what} states no way out:\n{plain}"));
            assert!(line.contains("Enter") && line.contains("'cancel'"), "{what}: {line}");
            assert!(
                line.contains("nothing there is cancelled"),
                "{what} must separate closing this screen from cancelling the prompt \
                 underneath, where 'cancel' discards a reveal: {line}",
            );
            // Directly above the caret, which is where a way out belongs.
            let lines: Vec<&str> = plain.lines().collect();
            let caret = lines.iter().position(|l| l.trim() == ">").expect("the caret");
            assert_eq!(lines[caret - 1], line, "{what}: the way out sits above the prompt");
        }
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
        let rendered = strip_ansi(&index_view(
            &build_order_index(
                &rules,
                &placed,
                Some(RecordingTarget::PortSetup { board_b: false, slot: 2, of: 6 }),
                None,
            ),
            &[],
        ));

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
        let text = strip_ansi(&index_view(&index, &[]));
        assert_eq!(index.shown, 5, "the Dutch group is five cards");
        assert!(text.contains("5 of 36, from Netherlands"), "{text:.300}");
        assert!(text.contains("[1] "), "the filtered grid renumbers from the base");
        assert!(!text.contains("[6] "), "and stops at what it shows: {text:.900}");

        let empty = strip_ansi(&view(Some(A_TARGET), Some(IndexFilter::MinVp(200))));
        assert!(empty.contains("no cards match that filter"), "{empty:.300}");
        assert!(empty.contains("'f' clears"), "the way out is named on the screen");
    }
}
