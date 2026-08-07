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
//! above it, so the thing it has to fit is the whole terminal rather than any
//! one count's frame width. At the design floor
//! ([`crate::screen::FLOOR_W`], 200 columns) that buys the mock's eight cards to
//! a row — at the 2-player frame's 198 it would be seven, and the grid would
//! break mid-group for a reason that has nothing to do with the cards.
//!
//! **The floor is the DESIGN, not a measurement, and this surface is the one
//! place that distinction had teeth** (T7 review SHOULD-6). Laying the grid out
//! at 200 columns on a 120-column terminal does not shrink the screen; it makes
//! the terminal wrap every row mid-card, turning the identification surface into
//! the least readable thing on it. `index_view` therefore takes the width it is
//! drawing into — [`crate::screen::index_width`] measures the real terminal
//! where that is knowable and falls back to the floor where it is not — and the
//! wizard says so once when the two differ.

use yokohama_core::{AchievementCategory, PlayerColor};

use crate::board_render::Twin;
use crate::card_render;
use crate::describe;
use crate::screen_model::{IndexCard, IndexFilter, IndexGroupKey, IndexKind, IndexModel};
use crate::style::{glyph, paint, Tone};

/// Columns between two cards on the same row — the mock's own single-space
/// gutter, which is what makes eight 24-column cards fit 200.
const CARD_GAP: usize = 1;

/// Cards drawn side by side at `card` columns each, inside `screen` columns:
/// the last card on a row needs no gutter after it, so `n` cards cost
/// `n * card + (n - 1) * CARD_GAP`. Never zero — a card wider than the screen
/// still gets a row of its own rather than an empty screen.
///
/// **`screen` is a PARAMETER since T7's review (SHOULD-6), and it used to be
/// [`FLOOR_W`].** The floor is a design decision about what the frame is drawn
/// FOR; using it here made it a claim about the terminal, so every index below
/// 200 columns laid out eight cards to a row and the terminal wrapped each row
/// mid-card — at the exact moment a player was reading cards to identify one.
/// Nothing said so, because nothing had measured. The floor is now the FALLBACK
/// ([`crate::screen::index_width`]), which is what it can honestly be.
fn cards_per_row(card: usize, screen: usize) -> usize {
    ((screen + CARD_GAP) / (card + CARD_GAP)).max(1)
}

/// Where this screen is being drawn, which decides ONE line: how to leave it.
///
/// **Two surfaces, and only one of them is an overlay** (T7 review M3). In the
/// REPL the index replaces the whole screen, so Enter really does close it and
/// put the player back at the prompt underneath. In the WIZARD it is printed
/// INLINE — `ask_screened` writes it to the transcript and re-asks the same
/// question below it — so there is nothing to close, Enter is not a way out at
/// all (`card_prompt` reads it as `Cancel`, which a setup question refuses), and
/// `?` redraws rather than closes. A single line claiming both was the finding:
/// it promised an overlay that did not exist and contradicted the prompt's own
/// hint two lines below it.
///
/// One renderer, two contexts, rather than one line hedged to cover both: a
/// sentence true of neither surface is how a player learns to stop reading the
/// chrome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexContext {
    /// Drawn full-screen over a prompt (the REPL's `orders`/`techs`/… views and
    /// its reveal captures).
    Overlay,
    /// Printed inline above a question that is about to be asked again (the
    /// setup wizard's `?`).
    Inline,
}

/// The whole index screen: title, context, the groups, footer, and — on an
/// overlay — the caret.
///
/// `colors` is the seat color table — needed only by the achievement cards,
/// which name claimants and per-seat progress. `context` decides the exit line
/// and whether a caret is drawn at all — see [`IndexContext`]. `screen_width` is
/// the columns to lay the grid out in; callers pass
/// [`crate::screen::index_width`] and tests pass the width they are asserting
/// about.
pub fn index_view(
    index: &IndexModel,
    colors: &[PlayerColor],
    context: IndexContext,
    screen_width: usize,
) -> String {
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
    let per_row = cards_per_row(card_render::card_width(index.kind), screen_width);
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
    out.push_str(&exit_line(context));
    out.push('\n');
    // **The caret belongs to the OVERLAY, and only there** (T7 review M3, the
    // real-terminal addendum). On an overlay it is the screen's own prompt —
    // the line the player answers, which is why the way out sits directly above
    // it. Printed INLINE it is a dead prompt: `ask_screened` writes this screen
    // into the transcript and then prints the question, with its own caret,
    // underneath — so the wizard was leaving three stray `>` lines per setup
    // run, each one an input line that reads nothing. Suppressing it is what
    // makes this a wizard MODE rather than a reworded overlay.
    if context == IndexContext::Overlay {
        out.push_str(&paint(Tone::Prompt, "> "));
        out.push('\n');
    }
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
///
/// **And it says a DIFFERENT true thing inline** (T7 review M3). The overlay
/// sentence names four ways to close a screen; at a wizard `?` there is no
/// screen to close — the question is printed again underneath — so all four
/// were wrong there at once. Worse, `Enter` and `cancel` are not even inert at
/// a setup question: [`crate::card_prompt::resolve`] reads both as `Cancel`,
/// and `ask_card` answers that with "this question needs an answer", so the
/// line was advertising an exit the next keystroke would refuse. The inline
/// wording states the two things that ARE true — the question continues below,
/// and `abort` is the one way out of setup — and it agrees with the prompt's
/// own hint (`'?' to redraw it`) instead of contradicting it one line later.
fn exit_line(context: IndexContext) -> String {
    let text = match context {
        IndexContext::Overlay => {
            "Enter, '?', 'cancel' or 'back' closes this index and puts you back at the prompt \
             underneath, exactly as you left it — nothing is entered and nothing there is cancelled."
        }
        IndexContext::Inline => {
            "Nothing here is entered yet and nothing you have already entered is lost — the \
             question continues below, so answer it ('?' redraws this index). 'abort' leaves setup."
        }
    };
    paint(Tone::Chrome, text)
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
            format!("CATEGORY {} {} {}", describe::category_letter(cat), glyph::SECTION_DASH, category_gloss(cat)),
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
    let drawn: Vec<Vec<String>> = cards
        .iter()
        .map(|c| card_render::index_card(c, colors, card_render::CardContext::Index))
        .collect();
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
            format!("in category {}", describe::category_letter(cat))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::{visible_width, FLOOR_W};
    use crate::screen_model::{build_order_index, CardGood, CardLocation, RecordingTarget};
    use yokohama_core::Country;
    use crate::style::strip_ansi;
    use yokohama_core::{OrderCardId, PlayerId, Rules};

    fn view(target: Option<RecordingTarget>, filter: Option<IndexFilter>) -> String {
        let rules = Rules::standard();
        index_view(&build_order_index(&rules, &[], target, filter), &[], IndexContext::Overlay, FLOOR_W)
    }

    const A_TARGET: RecordingTarget =
        RecordingTarget::PortSetup { board_b: false, slot: 2, of: 6 };

    const TECH_TARGET: RecordingTarget = RecordingTarget::TechRefill { board_b: false };

    /// ONE registry's model, at one of the two shapes — what the per-kind sweeps
    /// build their screens from.
    fn one_index(kind: IndexKind, target: Option<RecordingTarget>) -> IndexModel {
        let rules = Rules::standard();
        match kind {
            IndexKind::Orders => build_order_index(&rules, &[], target, None),
            IndexKind::Techs => crate::screen_model::build_tech_index(&rules, &[], target, None),
            IndexKind::Achievements => crate::screen_model::build_achievement_index(
                &crate::screen_model::build_achievements(&a_state()).cards,
                &[],
                target,
                None,
            ),
            IndexKind::Sites => crate::screen_model::build_site_index(&rules, &[], target, None),
        }
    }

    /// Every registry's screen, at both of the shapes it can be opened in —
    /// what the per-kind sweeps below run over.
    fn every_index(target: Option<RecordingTarget>) -> Vec<(IndexKind, String)> {
        let state = a_state();
        let colors: Vec<yokohama_core::PlayerColor> =
            state.players.iter().map(|p| p.color).collect();
        IndexKind::ALL
            .into_iter()
            .map(|kind| {
                (kind, index_view(&one_index(kind, target), &colors, IndexContext::Overlay, FLOOR_W))
            })
            .collect()
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
        let text = strip_ansi(&index_view(&model, &[], IndexContext::Overlay, FLOOR_W));

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
            strip_ansi(&index_view(&half, &[], IndexContext::Overlay, FLOOR_W)).contains("1 of 2 seen: Laboratory A slot 3"),
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
            strip_ansi(&index_view(&both, &[], IndexContext::Overlay, FLOOR_W)).contains("= Laboratory A slot 3 +1 more"),
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
            strip_ansi(&index_view(&crate::screen_model::build_achievement_index(&panel.cards, &[], None, None), &colors, IndexContext::Overlay, FLOOR_W));

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
            strip_ansi(&index_view(&crate::screen_model::build_achievement_index(&panel.cards, &[], None, None), &colors, IndexContext::Overlay, FLOOR_W));
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
        let text = strip_ansi(&index_view(&model, &[], IndexContext::Overlay, FLOOR_W));
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
        let text = strip_ansi(&index_view(&model, &[], IndexContext::Overlay, FLOOR_W));
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
            IndexContext::Overlay,
            FLOOR_W,
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

    /// **The way out says the truth about the surface it is drawn on** (T7
    /// review M3), at every registry and both shapes.
    ///
    /// The overlay sentence — four keys that close a screen and put you back at
    /// the prompt underneath — was printed inline in the wizard too, where none
    /// of the four closes anything: the index is written into the transcript and
    /// the question is asked again below it. Two of the four are worse than
    /// inert there, since `card_prompt` reads Enter and `cancel` as `Cancel` and
    /// `ask_card` answers that with "this question needs an answer". And the
    /// same screen's own prompt hint says `'?' to redraw it` one line later,
    /// which the word `closes` directly contradicts.
    ///
    /// Both halves are asserted, in both directions: each context says its own
    /// sentence AND does not say the other's, so a future edit that collapses
    /// the two back into one line fails here rather than passing whichever half
    /// it kept.
    #[test]
    fn the_way_out_is_the_truth_about_the_surface_it_is_drawn_on() {
        for target in [None, Some(A_TARGET)] {
            for (kind, _) in every_index(target) {
                let model = one_index(kind, target);
                let overlay = strip_ansi(&index_view(&model, &[], IndexContext::Overlay, FLOOR_W));
                let inline = strip_ansi(&index_view(&model, &[], IndexContext::Inline, FLOOR_W));

                assert!(
                    overlay.contains("closes this index and puts you back at the prompt underneath"),
                    "{kind:?} overlay:\n{overlay}",
                );
                assert!(
                    !inline.contains("closes this index"),
                    "{kind:?} inline must not promise to close a screen that is not there:\n{inline}",
                );
                assert!(
                    inline.contains("the question continues below, so answer it ('?' redraws this index)"),
                    "{kind:?} inline:\n{inline}",
                );
                assert!(
                    inline.contains("'abort' leaves setup"),
                    "{kind:?} inline must name the one exit a setup question has:\n{inline}",
                );
                assert!(
                    !overlay.contains("'abort'"),
                    "{kind:?} overlay has no setup to leave:\n{overlay}",
                );
                // The two differ in exactly one line, so nothing else moved with
                // the wording.
                let differing = overlay
                    .lines()
                    .zip(inline.lines())
                    .filter(|(a, b)| a != b)
                    .count();
                assert_eq!(differing, 1, "{kind:?}: only the way-out line differs");

                // **And the inline form draws no caret** (the real-terminal
                // addendum to M3). An overlay's `> ` is the line the player
                // answers; printed inline it is a dead prompt above the real
                // question, and the wizard was leaving one per index — three in
                // a live setup run. Checked as a LINE rather than a substring,
                // since a card face can legitimately hold a `>`.
                assert!(
                    !inline.lines().any(|l| l.trim() == ">"),
                    "{kind:?} inline must draw no caret of its own:\n{inline}",
                );
                assert_eq!(
                    overlay.lines().count(),
                    inline.lines().count() + 1,
                    "{kind:?}: the caret is the only line the overlay has extra",
                );
                assert_eq!(
                    overlay.lines().next_back().map(str::trim),
                    Some(">"),
                    "{kind:?}: ...and the overlay still ends at its caret",
                );
            }
        }
    }

    /// **The grid is laid out in the columns it is actually being drawn into**
    /// (T7 review SHOULD-6). `cards_per_row` computed from `FLOOR_W`, so at any
    /// width below 200 the index emitted rows wider than the terminal and the
    /// terminal wrapped them MID-CARD — a grid of half-cards, on the one screen
    /// whose whole job is letting a player recognize a card.
    ///
    /// Asserted over the CARD ROWS rather than the whole screen: the header and
    /// footer are prose and soft-wrap harmlessly, while a card row that overruns
    /// is the defect. Swept over every registry, because the four card widths
    /// (24 / 49 / 66 / 49) put the breakpoints in four different places.
    #[test]
    fn every_registry_lays_its_cards_out_inside_the_width_it_is_given() {
        for width in [80usize, 100, 120, 160, 200, 240] {
            for kind in IndexKind::ALL {
                let model = one_index(kind, Some(A_TARGET));
                let card = card_render::card_width(kind);
                let text = strip_ansi(&index_view(&model, &[], IndexContext::Overlay, width));
                let rows: Vec<&str> = text
                    .lines()
                    .filter(|l| l.starts_with('┌') || l.starts_with('│') || l.starts_with('└'))
                    .collect();
                assert!(!rows.is_empty(), "{kind:?} at {width}: no card rows were drawn");
                for row in &rows {
                    // A card wider than the whole screen cannot fit and gets a
                    // row of its own; that is the floor of what is achievable,
                    // and the notice is what tells the player.
                    let bound = width.max(card);
                    assert!(
                        visible_width(row) <= bound,
                        "{kind:?} at {width} columns drew a {}-column row (card is {card}): {row:?}",
                        visible_width(row),
                    );
                }
                // The row really is packed, not one card per line by accident:
                // the widest row uses the last card that fits.
                let per_row = cards_per_row(card, width);
                let widest = rows.iter().map(|r| visible_width(r)).max().expect("rows");
                let expected = (per_row * (card + CARD_GAP)).saturating_sub(CARD_GAP);
                assert!(
                    widest <= expected,
                    "{kind:?} at {width}: {widest} columns exceeds {per_row} cards' {expected}",
                );
            }
        }
    }

    /// The arithmetic itself, at the widths a person actually uses — and the
    /// mock's eight-to-a-row is the 200-column case rather than the rule.
    #[test]
    fn cards_per_row_is_computed_from_the_screen_not_from_the_floor() {
        use crate::card_render::CARD_W;
        assert_eq!(cards_per_row(CARD_W, FLOOR_W), 8, "the mock's row, at the design floor");
        assert_eq!(cards_per_row(CARD_W, 120), 4, "a 120-column window takes four order cards");
        assert_eq!(cards_per_row(CARD_W, 80), 3);
        assert_eq!(cards_per_row(CARD_W, 240), 9, "and a wide one takes more, not eight");
        // An achievement card is 66 columns: three at the floor, one at 120.
        assert_eq!(cards_per_row(crate::card_render::GOAL_W, FLOOR_W), 3);
        assert_eq!(cards_per_row(crate::card_render::GOAL_W, 120), 1);
        // Never zero, however narrow — a row of one clipped card beats none.
        assert_eq!(cards_per_row(crate::card_render::GOAL_W, 20), 1);
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
        assert_eq!(cards_per_row(CARD_W, FLOOR_W), 8, "the mock draws eight 24-column cards to a row");
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
            IndexContext::Overlay,
            FLOOR_W,
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
        let mut locations = vec![
            CardLocation::OwnHand,
            CardLocation::ThisReveal,
            // T7's: the achievement in play for a category, which the wizard's
            // own index dims while the next category is being asked.
            CardLocation::AchievementSlot { category: AchievementCategory::A },
        ];
        for seat in 0..4u8 {
            locations.push(CardLocation::Deal { seat: PlayerId::new(seat) });
        }
        for board_b in [false, true] {
            for slot in 0..6 {
                locations.push(CardLocation::PortSlot { board_b, slot });
            }
        }
        // T11: `OwnedBy` is worded with the seat's COLOUR where the caller has
        // a table and with its number where it does not, so BOTH forms are
        // swept — the widest colour name (`Yellow`) is not obviously shorter
        // than `Seat 3`, and only measuring says which the card has to fit.
        //
        // `LabSlot` is deliberately NOT here, and measuring is how that became
        // deliberate: `Laboratory A slot 1` is 19 columns against this face's
        // 18. It never lands on one — a lab slot is where a TECHNOLOGY is, and
        // a technology draws on the 49-column `TECH_W` card — so asserting it
        // here would be asserting about a card that is never drawn, and the
        // honest thing is to say so rather than to widen a budget for it.
        for seat in 0..4u8 {
            locations.push(CardLocation::OwnedBy { seat: PlayerId::new(seat) });
        }
        let every_colour = [
            yokohama_core::PlayerColor::Red,
            yokohama_core::PlayerColor::Blue,
            yokohama_core::PlayerColor::Yellow,
            yokohama_core::PlayerColor::Green,
        ];
        for at in locations {
            for colors in [&[][..], &every_colour[..]] {
                let text = describe::card_location(at, colors);
                assert!(
                    text.chars().count() <= room,
                    "{text:?} is {} columns; a card face has room for {room}",
                    text.chars().count(),
                );
            }
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
            IndexContext::Overlay,
            FLOOR_W,
        ));
        crate::view_render::tests::assert_capture("t5-order-index.txt", &rendered);
    }

    /// A filter narrows the grid, renumbers it from the base, and says what it
    /// did — including the empty case, which must not render a blank screen
    /// with no way out.
    #[test]
    fn a_filter_narrows_renumbers_and_says_so() {
        let rules = Rules::standard();
        let index =
            build_order_index(&rules, &[], Some(A_TARGET), Some(IndexFilter::Country(Country::Dutch)));
        let text = strip_ansi(&index_view(&index, &[], IndexContext::Overlay, FLOOR_W));
        assert_eq!(index.shown, 5, "the Dutch group is five cards");
        assert!(text.contains("5 of 36, from Netherlands"), "{text:.300}");
        assert!(text.contains("[1] "), "the filtered grid renumbers from the base");
        assert!(!text.contains("[6] "), "and stops at what it shows: {text:.900}");

        let empty = strip_ansi(&view(Some(A_TARGET), Some(IndexFilter::MinVp(200))));
        assert!(empty.contains("no cards match that filter"), "{empty:.300}");
        assert!(empty.contains("'f' clears"), "the way out is named on the screen");
    }

    /// **The ACHIEVEMENT index does not renumber under a filter** (T7 review
    /// M2), which is the exception the test above is the rule for.
    ///
    /// It is the one registry whose numbers are printed by a SECOND surface: the
    /// wizard's category question draws its own four cards from this model and
    /// leaves them on screen, so `f b` used to put `[1] Copper5` (the still
    /// visible menu) and `[1] Money10` (the fresh index) in front of an operator
    /// at once — F3 verbatim, at the prompt F1 was found at. Numbering over the
    /// whole population makes one number mean one card whatever the screen shows.
    ///
    /// Asserted against the UNFILTERED model's own numbers rather than against
    /// literals, so this pins the AGREEMENT rather than today's card order.
    #[test]
    fn the_achievement_index_keeps_its_numbers_when_a_filter_narrows_it() {
        let all = crate::screen_model::achievement_registry();
        let target = RecordingTarget::AchievementSetup { category: AchievementCategory::A };
        let full = crate::screen_model::build_achievement_index(&all, &[], Some(target), None);
        let numbers: Vec<(usize, crate::screen_model::CardRef)> = full
            .cards()
            .map(|c| (c.selection.expect("a recording screen numbers"), c.face.card_ref().unwrap()))
            .collect();
        assert_eq!(numbers.len(), 12);

        const CATEGORIES: [AchievementCategory; 3] =
            [AchievementCategory::A, AchievementCategory::B, AchievementCategory::C];
        for (position, category) in CATEGORIES.into_iter().enumerate() {
            let narrowed = crate::screen_model::build_achievement_index(
                &all,
                &[],
                Some(target),
                Some(IndexFilter::Category(category)),
            );
            assert_eq!(narrowed.shown, 4, "a category is four cards");
            for card in narrowed.cards() {
                let number = card.selection.expect("still numbered");
                assert_eq!(
                    (number, card.face.card_ref().unwrap()),
                    *numbers.iter().find(|(n, _)| *n == number).expect("the same number"),
                    "{category:?}: a number must name the same card filtered or not",
                );
                // ...through the model's own resolver, which is what a typed
                // number actually goes through.
                assert_eq!(narrowed.select(number), full.select(number));
            }
            // Category B is [5]-[8], so the screen is NOT numbered from the base
            // — the deliberate contrast with every other registry.
            let lowest = narrowed.cards().filter_map(|c| c.selection).min().unwrap();
            assert_eq!(
                lowest,
                crate::screen_model::menu_number(position * 4),
                "{category:?} keeps its slice of the twelve",
            );
        }
    }

    /// The ORDER index still compacts, which is the other half of the same
    /// ruling: nothing else prints its numbers, so a filtered screen numbering
    /// `[5]`-`[9]` would just be gaps with no second surface to agree with.
    #[test]
    fn the_other_registries_still_renumber_from_the_base_under_a_filter() {
        let rules = Rules::standard();
        for (kind, filter) in [
            (IndexKind::Orders, IndexFilter::Country(Country::Dutch)),
            (IndexKind::Techs, IndexFilter::Country(Country::Dutch)),
        ] {
            let narrowed = crate::screen_model::build_index(
                kind,
                &rules,
                &[],
                &crate::screen_model::achievement_registry(),
                Some(A_TARGET),
                Some(filter),
            );
            let numbers: Vec<usize> = narrowed.cards().filter_map(|c| c.selection).collect();
            assert!(!numbers.is_empty(), "{kind:?}: the fixture must narrow to something");
            assert_eq!(
                numbers,
                (0..numbers.len()).map(crate::screen_model::menu_number).collect::<Vec<_>>(),
                "{kind:?} numbers its filtered screen from the base with no gaps",
            );
        }
    }
}
