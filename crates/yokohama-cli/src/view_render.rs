//! The full-screen VIEWS — `tracks` and `sites` (Phase-3.5a spec §0.5, plan T6;
//! design authority is the approved mock pair
//! `2026-08-05-phase3.5a-mocks/tracks-view.ans` and `sites-view.ans`).
//!
//! **What the view pattern IS, since it is easy to mistake for a bigger panel.**
//! Every dashboard panel on the composed frame is a SUMMARY — the tracks panel
//! prints eight bracketed faith numbers and a VP range, which is enough to see
//! that faith 7 is taken and not enough to decide whether to take faith 9. The
//! view is the same information at full size, card by card, and it replaces the
//! frame rather than sitting under it: one action away, and Enter puts the
//! player back at the prompt they left with its state untouched (spec §0.5's
//! return contract, which the index established at T5 and these inherit).
//!
//! **These two exist as views rather than as bigger panels because they answer
//! DECISIONS.** The `sites` view is the Movement companion — the printed reward
//! on a space is what makes one destination worth a canal toll and another not —
//! which is why §0.5 singles it out and why it is reachable from inside a
//! Movement sub-prompt. The `tracks` view is what a Church or Customs payment is
//! decided against.
//!
//! **The R9 seam, held as the sibling renderers hold it.** This module renders;
//! it never reads game state. Its inputs are [`TracksPanel`] and [`BoardModel`]
//! — the SAME models the dashboard panels are built from, so a view and its
//! panel summary cannot disagree about a fact — plus the seat color table.
//! `GameState`, `Rules` and `Session` are absent by construction.
//!
//! **Width.** Both are full-screen surfaces, so the thing they fit is the design
//! floor ([`crate::screen::FLOOR_W`]) rather than any one count's frame width —
//! T5's precedent, and the reason the site grid holds four cards to a row at
//! every player count instead of re-flowing per count.

use yokohama_core::{AreaId, PlayerColor, PlayerId};

use crate::board_render::{color_of, exact, Twin};
use crate::card_render::{bottom, interior};
use crate::describe;
use crate::screen::FLOOR_W;
use crate::screen_model::{
    BoardModel, ChurchSpaceModel, CustomsSpaceModel, TileModel, TracksPanel,
};
use crate::style::{glyph, paint, Tone};

/// A track space card's width — the mock's `SP_W`, which fits all eight Church
/// spaces on one row inside the floor.
const SPACE_W: usize = 23;

/// A building-site card's width in the `sites` view. Two columns wider than the
/// mock's 44.
///
/// **The ground is the GUTTER, not a clip** — the earlier revision of this doc
/// said the mock's own geometry "would have clipped one of the twenty-four
/// cards", and that overstates what the measurement showed (T6 review S2).
///
/// The widest printed reward in full words is `2 warehouse takes` / `2 assistant
/// moves`, both 17 columns. At 46 the reward field runs from column 11 to 31 of
/// the interior — 20 columns — so the longest reward is followed by three spaces
/// before the builder mark. The same layout at the mock's 44 leaves 18, i.e. the
/// longest reward and a ONE-space gutter: tight against the mark beside it, and
/// legible rather than clipped. So the two extra columns bought breathing room,
/// not correctness.
///
/// What survives from the old wording is the method, which is the part worth
/// keeping: the number was measured against the whole registry rather than
/// against the example card the mock happened to draw
/// (`the_widest_printed_reward_fits_a_site_card`), the same way the round-3.1
/// tile overflow was found.
const SITE_VIEW_W: usize = 46;

/// Columns between cards on a row.
const GAP: usize = 1;

/// The `tracks` view's height in rows, and it does not vary with the player
/// count: the Church has the same eight faith spaces and the Customs board the
/// same seven at every count, so both card rows are one row tall in cards
/// whoever is playing.
///
/// **MEASURED, then set** — the T2/T3 discipline ([`crate::screen`]'s frame
/// tables), applied here at the T6 review (S2). The test this feeds used to
/// assert a bare `<= 50`, which passes at 21 rows and at 49 alike and therefore
/// could not tell growth from headroom.
///
/// `cfg(test)` for the reason `FRAME_HEIGHTS` is: nothing at runtime asks how
/// tall its own view is, so a production constant would be dead code claiming
/// to be used.
#[cfg(test)]
const TRACKS_VIEW_ROWS: usize = 22;

/// The `sites` view's height in rows at each player count, indexed
/// `players - 2` — MEASURED and set on the same terms as [`TRACKS_VIEW_ROWS`].
///
/// This one DOES vary, and by the only thing that can vary: the number of areas
/// holding a building site. The staggered pyramid deals ten, fourteen and
/// eighteen tiles at 2/3/4 players, one of which is the Canal (3P and 4P only)
/// and holds no site — so the grid runs ten, thirteen and seventeen cards at
/// four to a row, i.e. three, four and five rows of eight-row cards.
///
/// **The 4-player figure is two rows under the floor**, which is the tightest
/// this surface gets and the reason the number is written down rather than
/// bounded: the next card row would put it past 50.
#[cfg(test)]
const SITES_VIEW_ROWS: [usize; 3] = [32, 40, 48];

/// Allowance either side of the two tables above, and one row is all it is.
///
/// Both views are built from a fixed registry on a seeded board, so there is no
/// run-to-run noise for a margin to absorb — one row is "just above nothing"
/// rather than an allowance for anything in particular, and it keeps these
/// numbers something a growing surface can actually break.
#[cfg(test)]
const VIEW_ROW_MARGIN: usize = 1;

// ---------------------------------------------------------------------------
// The tracks view
// ---------------------------------------------------------------------------

/// The Church and Customs management boards, every space as a card.
///
/// **The dummy is a named occupant, not a shaded square** (finding D1): at two
/// players a neutral marker holds Church faith 7 and Customs space 4, it holds
/// that space against every player, and it COMPETES in each track's majority
/// bonus. The mock draws it in reverse video with the word `dummy` on it and
/// says so in both section rules, which is the treatment carried here.
///
/// **The fill counts exclude it** (SP1, brief §3.4): the end-game trigger counts
/// players' pawns and the dummy never moves, so the `n/4` on each section rule
/// is REAL pawns. That reading is the engine's and the model's
/// ([`TracksPanel::church_filled`]); it is stated on screen because a player who
/// counts occupied spaces and gets a different number deserves to know which
/// count the game is played on.
pub fn tracks_view(tracks: &TracksPanel, colors: &[PlayerColor]) -> String {
    let mut out = String::new();
    out.push_str(&paint(Tone::Strong, "TRACKS — full view"));
    out.push_str("   ");
    out.push_str(&paint(
        Tone::Chrome,
        "the 'view' pattern: every frame panel is a summary; this is its full surface",
    ));
    out.push('\n');
    out.push('\n');

    out.push_str(&church_rule(tracks));
    out.push('\n');
    let church: Vec<Vec<String>> = tracks.church.iter().map(|s| church_card(s, colors)).collect();
    for line in card_row(&church) {
        out.push_str(&line);
        out.push('\n');
    }
    out.push('\n');

    out.push_str(&customs_rule(tracks));
    out.push('\n');
    let customs: Vec<Vec<String>> = tracks.customs.iter().map(|s| customs_card(s, colors)).collect();
    for line in card_row(&customs) {
        out.push_str(&line);
        out.push('\n');
    }
    out.push('\n');

    out.push_str(&paint(
        Tone::Chrome,
        "A Customs payment must EXACTLY match a space's printed cost; with both spaces of a cost filled, that payment is unplaceable.",
    ));
    out.push('\n');
    out.push_str(&board_state_line(tracks));
    out.push('\n');
    out.push_str(&exit_line());
    out.push('\n');
    out.push_str(&paint(Tone::Prompt, "> "));
    out.push('\n');
    out
}

/// `CHURCH — pay faith, place an assistant … Ends at 4 REAL assistants …`
fn church_rule(tracks: &TracksPanel) -> String {
    let mut t = Twin::default();
    t.tinted(Tone::Strong, "CHURCH");
    t.raw(&format!(
        " {} pay faith, place an assistant on a space, take its printed reward.  ",
        glyph::SECTION_DASH,
    ));
    t.tinted(
        Tone::Chrome,
        // `6 / 3` is the church majority award, spelled here as a literal
        // because core exposes no constant for it: the two figures live inside
        // `game/scoring.rs`'s `award_top_two(&rankings, [6, 3])` call, not on
        // any public item this renderer could read. Its customs twin is in
        // [`customs_rule`], and the two are the only hardcoded scoring numbers
        // on this surface (T6 review S4).
        &format!(
            "{} of {} REAL assistants placed {} majority 6 / 3 VP{}",
            tracks.church_filled,
            tracks.trigger_at,
            glyph::SEPARATOR,
            dummy_clause(tracks),
        ),
    );
    t.painted
}

/// `CUSTOMS — pay the printed cost in imported goods …`
fn customs_rule(tracks: &TracksPanel) -> String {
    let mut t = Twin::default();
    t.tinted(Tone::Strong, "CUSTOMS");
    t.raw(&format!(
        " {} pay the printed cost in imported goods (power caps it), take the reward.  ",
        glyph::SECTION_DASH,
    ));
    t.tinted(
        Tone::Chrome,
        // The customs majority award, from `game/scoring.rs`'s
        // `award_top_two(&rankings, [8, 4])` — a literal for the same reason
        // [`church_rule`]'s `6 / 3` is one: core exposes no constant.
        &format!(
            "{} of {} REAL assistants placed {} majority 8 / 4 VP{}",
            tracks.customs_filled,
            tracks.trigger_at,
            glyph::SEPARATOR,
            dummy_clause(tracks),
        ),
    );
    t.painted
}

/// The dummy's two facts, on the rules that need them and nowhere else: it never
/// advances the end-count, and it still competes for the majority.
fn dummy_clause(tracks: &TracksPanel) -> String {
    let present = tracks
        .church
        .iter()
        .filter_map(|s| s.occupant)
        .chain(tracks.customs.iter().filter_map(|s| s.occupant))
        .any(|o| o.is_dummy());
    if present {
        format!(" {} the dummy COMPETES but is not counted", glyph::SEPARATOR)
    } else {
        String::new()
    }
}

fn church_card(space: &ChurchSpaceModel, colors: &[PlayerColor]) -> Vec<String> {
    let mut body = Vec::new();
    let mut vp = Twin::default();
    vp.tinted(Tone::Vp, &format!("{}{} VP", glyph::VP, space.vp));
    body.push(vp);
    let mut rider = Twin::default();
    rider.tinted(Tone::Reachable, &format!("+{} assistant moves", space.movement_rider));
    body.push(rider);
    space_card(&format!("faith {}", space.faith), body, space.occupant, colors)
}

fn customs_card(space: &CustomsSpaceModel, colors: &[PlayerColor]) -> Vec<String> {
    let mut body = Vec::new();
    let mut cost = Twin::default();
    cost.tinted(
        Tone::Good(crate::style::GoodKind::Imports),
        &format!("pay {}", plural(space.import_cost, "import")),
    );
    body.push(cost);
    let mut vp = Twin::default();
    vp.tinted(Tone::Vp, &format!("{}{} VP", glyph::VP, space.vp));
    body.push(vp);
    let mut rider = Twin::default();
    match (space.warehouse_rider, space.yen_rider) {
        (0, 0) => {
            rider.tinted(Tone::Dim, &glyph::SECTION_DASH.to_string());
        }
        (w, 0) => {
            rider.tinted(Tone::Reachable, &format!("+{}", plural(w, "warehouse take")));
        }
        (0, y) => {
            rider.tinted(Tone::Reachable, &format!("+{y} yen"));
        }
        (w, y) => {
            rider.tinted(
                Tone::Reachable,
                &format!("+{} +{y} yen", plural(w, "warehouse take")),
            );
        }
    }
    body.push(rider);
    space_card(&format!("space {}", space.space), body, space.occupant, colors)
}

/// One track space as a card: its label, its printed rewards, then who is on it.
fn space_card(
    label: &str,
    body: Vec<Twin>,
    occupant: Option<PlayerId>,
    colors: &[PlayerColor],
) -> Vec<String> {
    let mut top = Twin::default();
    top.raw(&format!("{} ", glyph::BOX_TL));
    top.tinted(Tone::Strong, label);
    top.raw(" ");
    let fill = (SPACE_W - 1).saturating_sub(top.width());
    top.raw(&String::from(glyph::RULE).repeat(fill));
    top.raw(&glyph::BOX_TR.to_string());

    let mut rows = vec![exact(&top, SPACE_W)];
    for line in &body {
        rows.push(interior(line, SPACE_W));
    }
    rows.push(interior(&occupant_line(occupant, colors), SPACE_W));
    rows.push(bottom(SPACE_W));
    rows
}

/// `empty`, a seat's name in its own color, or the neutral dummy called by name.
fn occupant_line(occupant: Option<PlayerId>, colors: &[PlayerColor]) -> Twin {
    let mut t = Twin::default();
    match occupant {
        None => {
            t.tinted(Tone::Dim, "empty");
        }
        Some(seat) if seat.is_dummy() => {
            t.tinted(Tone::Highlight, " D · neutral dummy ");
        }
        Some(seat) => match color_of(colors, seat) {
            Some(c) => {
                t.tinted(Tone::Player(c), &format!("{} {}", glyph::ASSISTANT, crate::render::color_name(c)));
            }
            None => {
                t.raw(&format!("{} seat {}", glyph::ASSISTANT, seat.0));
            }
        },
    }
    t
}

/// The board-level shared state that reads beside the two tracks: where the
/// Station stands, and how many mercenaries are left.
///
/// **Carried over from the retired `render::tracks_view`, deliberately.** Both
/// are public table facts that appear on no other full surface — the Station
/// grants EVERY player +1 Power in its area, which a live-tracking operator has
/// to be able to read back and reconcile against the physical board (I4), and
/// the mercenary piles are shared supply. The approved mock does not draw them
/// because it was drawn as a study of the two tracks; dropping them to match it
/// would have retired information with no successor.
fn board_state_line(tracks: &TracksPanel) -> String {
    let mut t = Twin::default();
    t.tinted(Tone::Chrome, "Station: ");
    match tracks.station {
        Some(area) => {
            t.raw(&format!("A{:02} {} all players read +1 Power there", area.0, glyph::SECTION_DASH));
        }
        None => {
            t.tinted(Tone::Dim, "not placed");
        }
    }
    let s = &tracks.mercenary_supply;
    t.raw(&format!("   {}   ", glyph::SEPARATOR));
    t.tinted(Tone::Chrome, "Mercenary supply: ");
    t.raw(&format!(
        "GB {}  US {}  FR {}  DE {}  NL {}",
        s.english, s.american, s.french, s.german, s.dutch,
    ));
    t.painted
}

// ---------------------------------------------------------------------------
// The sites view
// ---------------------------------------------------------------------------

/// Every area's building-site card: what each space pays, who has built there,
/// and what the trading house is worth.
///
/// **This is the Movement companion** (spec §0.5). A President moves to take an
/// area's action, but the reason to pay a canal toll to reach one area rather
/// than its neighbour is often what is still unbuilt on it — and that is printed
/// on the card, not on the board. The tile strip shows the same four rewards as
/// compact tokens; this shows them in words, with the builder's NAME rather than
/// a color letter, which is the mock's own instruction.
pub fn sites_view(board: &BoardModel, colors: &[PlayerColor]) -> String {
    let mut out = String::new();
    out.push_str(&paint(Tone::Strong, "BUILDING SITES — full view"));
    out.push_str("   ");
    out.push_str(&paint(
        Tone::Chrome,
        "every area's site card: each space's printed reward, what is built, and by whom",
    ));
    out.push('\n');
    out.push('\n');

    // In `AreaId` order, which is the order the board draws and the order every
    // id-typed grammar in the driver reads.
    let mut tiles: Vec<&TileModel> = board.tiles.iter().filter(|t| t.site.is_some()).collect();
    tiles.sort_by_key(|t| t.area.0);

    if tiles.is_empty() {
        out.push_str(&paint(Tone::Dim, "  this board carries no building sites"));
        out.push('\n');
    }
    let per_row = ((FLOOR_W + GAP) / (SITE_VIEW_W + GAP)).max(1);
    for chunk in tiles.chunks(per_row) {
        let drawn: Vec<Vec<String>> = chunk.iter().map(|t| site_card(t, colors)).collect();
        for line in card_row(&drawn) {
            out.push_str(&line);
            out.push('\n');
        }
    }

    out.push('\n');
    out.push_str(&paint(
        Tone::Chrome,
        "A Shophouse built on a space takes that space's printed reward once; a Trading House builds FREE from hand and scores its VP.",
    ));
    out.push('\n');
    out.push_str(&paint(
        Tone::Chrome,
        "Which tiles touch which is on the board itself; the tolled canal crossings are not, so each card names its own.",
    ));
    out.push('\n');
    out.push_str(&exit_line());
    out.push('\n');
    out.push_str(&paint(Tone::Prompt, "> "));
    out.push('\n');
    out
}

/// One area's site card. Every line exactly [`SITE_VIEW_W`] columns.
fn site_card(tile: &TileModel, colors: &[PlayerColor]) -> Vec<String> {
    let Some(site) = tile.site.as_ref() else {
        // Unreachable from `sites_view`, which filters on `site.is_some()`;
        // total anyway, on the rule every emitter in this crate holds.
        return vec![bottom(SITE_VIEW_W)];
    };

    let mut top = Twin::default();
    top.raw(&format!("{} ", glyph::BOX_TL));
    match site.index {
        Some(index) => {
            top.tinted(Tone::Strong, &format!("SITE {index}"));
        }
        None => {
            top.tinted(Tone::Dim, "SITE ?");
        }
    }
    top.raw(" ");
    top.tinted(
        Tone::Chrome,
        &format!("{} on {}", glyph::SECTION_DASH, describe::area_type_name(tile.area_type).to_uppercase()),
    );
    top.raw(" ");
    let fill = (SITE_VIEW_W - 1).saturating_sub(top.width());
    top.raw(&String::from(glyph::RULE).repeat(fill));
    top.raw(&glyph::BOX_TR.to_string());

    let mut rows = vec![exact(&top, SITE_VIEW_W)];
    for (i, slot) in site.slots.iter().enumerate() {
        let mut t = Twin::default();
        t.tinted(Tone::Chrome, describe::site_space_name(i));
        t.pad_to(11);
        crate::card_render::push_space_reward(&mut t, &slot.reward, false);
        t.pad_to(31);
        match slot.built_by {
            Some(seat) => {
                t.owned(colors, seat, &format!("{} {}", glyph::BUILT, seat_name(colors, seat)));
            }
            None => {
                t.tinted(Tone::Dim, "open");
            }
        }
        rows.push(interior(&t, SITE_VIEW_W));
    }

    let mut house = Twin::default();
    house.tinted(Tone::Chrome, "trading house");
    house.pad_to(15);
    house.tinted(Tone::Vp, &format!("{}{} VP", glyph::VP, site.trading_house_vp));
    house.pad_to(31);
    match site.trading_house_owner {
        Some(seat) => {
            house.owned(colors, seat, &format!("{} {}", glyph::BUILT, seat_name(colors, seat)));
        }
        None => {
            house.tinted(Tone::Dim, "open");
        }
    }
    rows.push(interior(&house, SITE_VIEW_W));
    rows.push(interior(&identity_line(tile), SITE_VIEW_W));
    rows.push(bottom(SITE_VIEW_W));
    rows
}

/// The area's own id, and its tolled canal crossings when it has any.
///
/// **The id is here because every grammar in the driver takes one** — `a`, `b`,
/// `m`, the Placement pickers — so a card that told a player what a space pays
/// without telling them what to type would send them back to the board to look
/// it up.
///
/// **The crossings are here because they are on no other surface.** Ordinary
/// adjacency is legible from the board: the staggered pyramid draws touching
/// tiles touching, each with its id on its title. A canal crossing is not
/// geometric — it is a tolled edge between two shores that may sit nowhere near
/// each other — and the retired `render::sites_view` was the only place it was
/// ever printed. This carries that forward; the free-neighbour list it also
/// printed is the part the board itself now answers, and is the one thing this
/// surface deliberately drops.
fn identity_line(tile: &TileModel) -> Twin {
    let mut t = Twin::default();
    t.tinted(Tone::Chrome, &format!("A{:02}", tile.area.0));
    if !tile.canal_crossings.is_empty() {
        t.raw(&format!("  {}  ", glyph::SEPARATOR));
        t.tinted(Tone::Chrome, "canal toll to ");
        t.raw(
            &tile
                .canal_crossings
                .iter()
                .map(|a: &AreaId| format!("A{:02}", a.0))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    t
}

// ---------------------------------------------------------------------------
// Shared
// ---------------------------------------------------------------------------

/// A seat's display name, or its number when the caller's table does not cover
/// it — the same fallback [`Twin::owned`] makes for the tint.
fn seat_name(colors: &[PlayerColor], seat: PlayerId) -> String {
    match color_of(colors, seat) {
        Some(c) => crate::render::color_name(c).to_string(),
        None => format!("seat {}", seat.0),
    }
}

/// `1 import` / `2 imports` — the one plural this module needs.
fn plural(count: u8, noun: &str) -> String {
    if count == 1 {
        format!("{count} {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// Cards side by side, padded to the tallest in the row.
fn card_row(cards: &[Vec<String>]) -> Vec<String> {
    let height = cards.iter().map(Vec::len).max().unwrap_or(0);
    (0..height)
        .map(|i| {
            let mut line = String::new();
            for (j, card) in cards.iter().enumerate() {
                if j > 0 {
                    line.push_str(&" ".repeat(GAP));
                }
                match card.get(i) {
                    Some(text) => line.push_str(text),
                    None => line.push_str(
                        &" ".repeat(card.last().map(|l| crate::screen::visible_width(l)).unwrap_or(0)),
                    ),
                }
            }
            while line.ends_with(' ') {
                line.pop();
            }
            line
        })
        .collect()
}

/// How to leave, on every view — the clause T5's review made standing for the
/// index (M2) and which every full-screen surface owes for the same reason: it
/// has replaced the prompt the player was in the middle of.
///
/// **Two lines, because the contract has two halves and the second one moves
/// the game** (T6 review M2c). The old single line ended "Any other command
/// closes it and runs", which is true and reads as being about commands like
/// `score`. The input it does not bring to mind is a NUMBER: with options up
/// underneath, a digit typed here closes the view and APPLIES that option — the
/// one outcome a player who opened a read-only screen is not expecting. So the
/// rule is stated over LINES rather than commands, the number is named, and
/// `back` — the one word whose meaning differs from the index's, where it
/// closes — says what it does here.
///
/// The two halves would not fit one line inside [`crate::screen::FLOOR_W`]; the
/// row they cost is recorded in `both_views_fit_the_fifty_row_design_floor`'s
/// measured table rather than absorbed.
fn exit_line() -> String {
    // Painted per line rather than as one string with a newline in it, so
    // neither line carries a tint across the row break.
    format!(
        "{}\n{}",
        paint(
            Tone::Chrome,
            "Enter or 'cancel' closes this view and puts you back at the prompt underneath, exactly as you left it — \
             nothing is entered and nothing there is cancelled.",
        ),
        paint(
            Tone::Chrome,
            "Any other line closes the view and then RUNS at that prompt — a NUMBER applies that option, \
             and 'back' undoes your last move.",
        ),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::screen::visible_width;
    use crate::style::strip_ansi;
    use yokohama_core::{finalize_setup, new_game, GameConfig, Rules, StartingOrderChoice};

    /// The two models these views consume, plus the seat colors, from a REAL
    /// generated board at `players` — so the geometry-sensitive assertions run
    /// against the shipped topology (3P/4P carry a Canal, 2P does not) rather
    /// than an injected fixture.
    fn view_models(players: u8) -> (TracksPanel, BoardModel, Vec<PlayerColor>) {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(players), &rules, 1)
            .expect("a standard board");
        let state =
            finalize_setup(pending, &vec![StartingOrderChoice::Keep(0); players as usize])
                .expect("setup finalizes");
        let colors = state.players.iter().map(|p| p.color).collect();
        (
            crate::screen_model::build_tracks(&state),
            crate::screen_model::build_board(&state, &rules, &[]),
            colors,
        )
    }

    /// Every line of both views fits the design floor, at every player count —
    /// the property that lets them be drawn full-screen with nothing above.
    #[test]
    fn every_view_line_fits_the_design_floor() {
        for players in [2u8, 3, 4] {
            let (tracks, board, colors) = view_models(players);
            for (what, text) in
                [("tracks", tracks_view(&tracks, &colors)), ("sites", sites_view(&board, &colors))]
            {
                for line in text.lines() {
                    assert!(
                        visible_width(line) <= FLOOR_W,
                        "{what} at {players}P: {} columns is past the floor: {:?}",
                        visible_width(line),
                        strip_ansi(line),
                    );
                }
            }
        }
    }

    /// **Every card line measures exactly its card's width.** The round-3.1
    /// lesson, applied to the two surfaces T6 adds: a card that clipped would
    /// shear the whole row it sits in, and it would do it silently.
    #[test]
    fn every_card_line_measures_exactly_its_card_width() {
        let (tracks, board, colors) = view_models(4);
        for space in &tracks.church {
            for line in church_card(space, &colors) {
                assert_eq!(visible_width(&line), SPACE_W, "{:?}", strip_ansi(&line));
            }
        }
        for space in &tracks.customs {
            for line in customs_card(space, &colors) {
                assert_eq!(visible_width(&line), SPACE_W, "{:?}", strip_ansi(&line));
            }
        }
        for tile in board.tiles.iter().filter(|t| t.site.is_some()) {
            for line in site_card(tile, &colors) {
                assert_eq!(visible_width(&line), SITE_VIEW_W, "{:?}", strip_ansi(&line));
            }
        }
    }

    /// **The `tracks` view carries every fact the retired `render::tracks_view`
    /// carried** — the migration half of T6's retirement census. Each clause
    /// below is a property that surface asserted and this one now owes: the
    /// printed VP and rider on every space, the printed import cost, the
    /// Station's location, and the shared mercenary supply.
    #[test]
    fn the_tracks_view_carries_every_printed_reward_and_the_board_state() {
        let (mut tracks, _, colors) = view_models(2);
        let text = strip_ansi(&tracks_view(&tracks, &colors));

        for space in &tracks.church {
            assert!(text.contains(&format!("faith {}", space.faith)), "{text}");
            assert!(
                text.contains(&format!("★{} VP", space.vp)),
                "faith {} must print its VP: {text}",
                space.faith,
            );
        }
        for space in &tracks.customs {
            assert!(text.contains(&format!("space {}", space.space)), "{text}");
            assert!(
                text.contains(&format!("pay {} import", space.import_cost)),
                "customs space {} must print its printed cost: {text}",
                space.space,
            );
        }
        // The two board-level facts that live on no other full surface.
        assert!(text.contains("Station: not placed"), "{text}");
        assert!(text.contains("Mercenary supply:"), "{text}");
        assert!(text.contains(&format!("GB {}", tracks.mercenary_supply.english)), "{text}");

        tracks.station = Some(yokohama_core::AreaId::new(3));
        let placed = strip_ansi(&tracks_view(&tracks, &colors));
        assert!(placed.contains("Station: A03"), "{placed}");
        assert!(placed.contains("+1 Power"), "and what it does there: {placed}");
    }

    /// **The 2-player dummy is a named occupant that COMPETES** (finding D1),
    /// and the fill counts exclude it (SP1). The retired view said both in a
    /// legend line; this one says the first on the space itself and the second
    /// on the section rule the count sits in.
    #[test]
    fn the_dummy_is_named_on_its_space_and_excluded_from_the_fill_count() {
        let (tracks, _, colors) = view_models(2);
        let text = strip_ansi(&tracks_view(&tracks, &colors));

        // The engine seats the 2P dummy on Church faith 7 and Customs space 4.
        assert_eq!(
            tracks.church.iter().find(|s| s.faith == 7).and_then(|s| s.occupant),
            Some(yokohama_core::PlayerId::DUMMY),
            "fixture: the 2P dummy holds faith 7",
        );
        assert!(text.contains("D · neutral dummy"), "the space names it: {text}");
        assert!(text.contains("COMPETES"), "and says it competes for the majority: {text}");
        assert_eq!(tracks.church_filled, 0, "the dummy is not a REAL pawn");
        assert!(
            text.contains(&format!("{} of {} REAL assistants", 0, tracks.trigger_at)),
            "the fill count is stated as REAL pawns: {text}",
        );

        // At three players there is no dummy, and the clause does not appear —
        // a legend for a treatment nothing on screen uses teaches a rule the
        // player then looks for and cannot find.
        let (three, _, colors) = view_models(3);
        let text = strip_ansi(&tracks_view(&three, &colors));
        assert!(!text.contains("dummy"), "no dummy at 3P, no dummy clause: {text}");
    }

    /// **The `sites` view carries every fact the retired `render::sites_view`
    /// carried, except the one T6 retires deliberately** — the other half of the
    /// census. Present: a card per sited area, the area id, the four printed
    /// rewards, the trading-house VP. Absent by decision: the `adj:` free
    /// neighbour list, which the staggered board itself now answers (see
    /// [`identity_line`]).
    #[test]
    fn the_sites_view_draws_one_card_per_sited_area_with_its_id_and_rewards() {
        let (_, board, colors) = view_models(2);
        let text = strip_ansi(&sites_view(&board, &colors));

        let sited: Vec<&TileModel> = board.tiles.iter().filter(|t| t.site.is_some()).collect();
        assert!(sited.len() >= 9, "fixture: a 2P board deals nine or ten sites");
        for tile in &sited {
            let site = tile.site.as_ref().expect("filtered on it");
            assert!(
                text.contains(&format!("A{:02}", tile.area.0)),
                "every card names its area so the id grammars can be typed: {text}",
            );
            assert!(
                text.contains(&format!("SITE {}", site.index.expect("a dealt registry card"))),
                "and its site card number: {text}",
            );
            for slot in &site.slots {
                // The WORDS, and only the words. This clause used to be
                // `contains(space_name) || contains(words)`, and the left half
                // is true of every card on the screen — every site card prints
                // all four space names whatever their rewards say — so the
                // disjunction passed whether or not a single reward was
                // rendered (T6 review S3).
                let words = crate::card_render::space_reward_words(&slot.reward);
                assert!(
                    text.contains(&words),
                    "the printed reward must be on screen in full words: {words:?}\n{text}",
                );
            }
            assert!(
                text.contains(&format!("★{} VP", site.trading_house_vp)),
                "the trading house VP: {text}",
            );
        }
        // The Canal holds no building site, so it gets no card — the one tile
        // this surface is silent about, and it is silent because there is
        // nothing printed there.
        assert!(!text.contains("on CANAL"), "the Canal has no site card: {text}");
    }

    /// A built space names its BUILDER, and it names them POSITIONALLY — the
    /// property the retired `build_marks` tests held. The engine offers one
    /// `BuildShophouse` per EMPTY slot, so `shophouses_built` carries gaps, and
    /// a card that packed the owners together would attribute a builder to the
    /// wrong printed reward.
    #[test]
    fn a_built_space_names_its_builder_beside_its_own_printed_reward() {
        let (_, mut board, colors) = view_models(2);
        let seat = yokohama_core::PlayerId::new(1);
        let tile = board
            .tiles
            .iter_mut()
            .find(|t| t.site.is_some())
            .expect("a sited tile");
        let site = tile.site.as_mut().expect("filtered on it");
        // The THIRD space, so a packed rendering would show the builder against
        // the first space's reward instead.
        site.slots[2].built_by = Some(seat);
        site.trading_house_owner = Some(seat);
        let area = tile.area;

        let text = strip_ansi(&sites_view(&board, &colors));
        let card: Vec<&str> = text
            .lines()
            .skip_while(|l| !l.contains(&format!("A{:02}", area.0)))
            .collect();
        assert!(!card.is_empty(), "the card is on screen: {text}");
        let built: Vec<&str> = text.lines().filter(|l| l.contains("⌂ Blue")).collect();
        assert_eq!(built.len(), 2, "one built space and one trading house: {built:?}");
        assert!(
            built[0].contains("mid-left"),
            "the builder sits on the space they built, not the first one: {:?}",
            built[0],
        );
        assert!(built[1].contains("trading house"), "{:?}", built[1]);
    }

    /// **Canal crossings show on BOTH members of a pair** — the property the
    /// retired `sites_view_shows_canal_crossings_bidirectionally` held, and the
    /// reason the identity line exists at all: a tolled crossing is on no other
    /// surface. Injected directly, exactly as that test did: the renderer reads
    /// the pair list, so board geometry is irrelevant to the display.
    #[test]
    fn a_canal_crossing_shows_on_both_shores() {
        let (_, mut board, colors) = view_models(2);
        let sited: Vec<AreaId> = board
            .tiles
            .iter()
            .filter(|t| t.site.is_some())
            .map(|t| t.area)
            .collect();
        let (a, b) = (sited[0], sited[1]);
        for tile in board.tiles.iter_mut() {
            if tile.area == a {
                tile.canal_crossings = vec![b];
            } else if tile.area == b {
                tile.canal_crossings = vec![a];
            }
        }

        let text = strip_ansi(&sites_view(&board, &colors));
        for (from, to) in [(a, b), (b, a)] {
            let line = text
                .lines()
                .find(|l| l.contains(&format!("A{:02}", from.0)) && l.contains("canal toll"))
                .unwrap_or_else(|| panic!("A{:02}'s identity line in\n{text}", from.0));
            assert!(
                line.contains(&format!("A{:02}", to.0)),
                "the crossing must show on this shore too: {line}",
            );
        }
    }

    /// **The model, not this renderer, decides what a crossing IS.** Pinned
    /// against `board_layout.canal_crossings` at 3P and 4P — the counts where a
    /// Canal exists — so a re-derivation from geometry could not creep back in.
    #[test]
    fn the_crossing_set_is_the_engines_own_pair_list() {
        for players in [3u8, 4] {
            let rules = yokohama_core::Rules::standard();
            let pending =
                yokohama_core::new_game(yokohama_core::GameConfig::for_player_count(players), &rules, 1)
                    .expect("a standard board");
            let state = yokohama_core::finalize_setup(
                pending,
                &vec![yokohama_core::StartingOrderChoice::Keep(0); players as usize],
            )
            .expect("setup finalizes");
            let board = crate::screen_model::build_board(&state, &rules, &[]);
            assert!(
                !state.board_layout.canal_crossings.is_empty(),
                "fixture: a {players}P board has a Canal",
            );
            for &(a, b) in &state.board_layout.canal_crossings {
                for (from, to) in [(a, b), (b, a)] {
                    let tile = board.tiles.iter().find(|t| t.area == from).expect("the tile");
                    assert!(
                        tile.canal_crossings.contains(&to),
                        "A{:02} must list its crossing to A{:02}",
                        from.0,
                        to.0,
                    );
                }
            }
        }
    }

    /// **Both views stay byte-plain with color off and paint with it on** —
    /// spec §0.12's honesty net, and the property the two retired
    /// `colored_*_view_still_contains_the_plain_text` tests held. Color is
    /// FORCED ON in the same test, so it cannot pass by the renderer painting
    /// nothing at all.
    #[test]
    fn the_views_are_escape_free_with_color_off_and_painted_with_it_on() {
        let (tracks, board, colors) = view_models(2);
        for (what, plain, painted) in [
            ("tracks", tracks_view(&tracks, &colors), {
                crate::style::force(true);
                let p = tracks_view(&tracks, &colors);
                crate::style::force(false);
                p
            }),
            ("sites", sites_view(&board, &colors), {
                crate::style::force(true);
                let p = sites_view(&board, &colors);
                crate::style::force(false);
                p
            }),
        ] {
            assert!(!plain.contains('\u{1b}'), "{what}: piped output must carry no escapes");
            assert!(painted.contains('\u{1b}'), "{what}: a real terminal gets color");
            assert_eq!(strip_ansi(&painted), plain, "{what}: the two differ only by escapes");
        }
    }

    /// Both views fit the 50-row design floor at every player count — the height
    /// half of the discipline the width test states.
    ///
    /// **Stated as the MEASURED table rather than as a bare bound** (T6 review
    /// S2, restating this to T2's discipline). `rows <= 50` passes at 21 rows
    /// and at 49 alike, so it could not tell growth from headroom: the 4-player
    /// `sites` view could have doubled and this would have stayed green until
    /// the day it went red, with no recorded history to compare against. What is
    /// asserted now is [`TRACKS_VIEW_ROWS`] and [`SITES_VIEW_ROWS`], both ways —
    /// a surface that SHRANK past the margin has left a recorded number stale,
    /// which is the same defect from the other side — and the floor is checked
    /// against the RECORDED figure, so raising an entry cannot quietly raise the
    /// ceiling with it.
    #[test]
    fn both_views_fit_the_fifty_row_design_floor() {
        for players in [2u8, 3, 4] {
            let (tracks, board, colors) = view_models(players);
            for (what, text, recorded) in [
                ("tracks", tracks_view(&tracks, &colors), TRACKS_VIEW_ROWS),
                ("sites", sites_view(&board, &colors), SITES_VIEW_ROWS[usize::from(players - 2)]),
            ] {
                let rows = text.lines().count();
                assert!(
                    rows <= recorded + VIEW_ROW_MARGIN,
                    "{what} at {players}P draws {rows} rows against a recorded {recorded} \
                     (+{VIEW_ROW_MARGIN}) — re-measure the table deliberately rather than raising \
                     this entry until the run goes quiet",
                );
                assert!(
                    rows + VIEW_ROW_MARGIN >= recorded,
                    "{what} at {players}P draws {rows} rows against a recorded {recorded} — the \
                     surface shrank and the recorded figure is now stale",
                );
                assert!(
                    recorded + VIEW_ROW_MARGIN <= 50,
                    "{what} at {players}P: the recorded height plus its margin must itself fit \
                     the 50-row design floor",
                );
            }
        }
    }

    /// **The exit line states the whole contract, including the half that MOVES
    /// THE GAME** (T6 review M2c).
    ///
    /// It used to end "Any other command closes it and runs" — true, and read by
    /// a player as being about commands like `score`. The input it does not
    /// bring to mind is a NUMBER: at a prompt with options up, a digit typed
    /// while looking at a view closes it and APPLIES that option, which is the
    /// one outcome a player opening a read-only screen is not expecting. So the
    /// line says it, in the words `a_number_typed_at_a_view_closes_it_and_
    /// applies_that_option` pins the behavior with.
    #[test]
    fn the_exit_line_says_a_number_applies_the_option_underneath() {
        let plain = strip_ansi(&exit_line());
        assert!(plain.contains("Enter") && plain.contains("'cancel'"), "{plain}");
        assert!(
            plain.contains("Any other line") && plain.contains("RUNS"),
            "the re-dispatch rule is stated as a rule about LINES, not commands: {plain}",
        );
        assert!(
            plain.contains("NUMBER applies that option"),
            "and the consequence a player would not expect is named: {plain}",
        );
        // `back` is the one word whose meaning differs from the index's, so the
        // view says what it does here rather than leaving it to be discovered.
        assert!(plain.contains("'back'"), "{plain}");

        // Each row is tinted on its own, so neither carries a colour across the
        // break — the reason `exit_line` paints twice instead of painting one
        // string with a newline in it.
        crate::style::force(true);
        let painted = exit_line();
        crate::style::force(false);
        let rows: Vec<&str> = painted.lines().collect();
        assert_eq!(rows.len(), 2, "two lines: {painted:?}");
        for row in &rows {
            assert!(row.starts_with('\u{1b}'), "each row opens its own tint: {row:?}");
            assert!(row.ends_with('m'), "and closes it: {row:?}");
        }

        // It is on both views, above the caret, where a player looks for the
        // way out — the placement the index's own exit line holds.
        let (tracks, board, colors) = view_models(2);
        for (what, text) in
            [("tracks", tracks_view(&tracks, &colors)), ("sites", sites_view(&board, &colors))]
        {
            let text = strip_ansi(&text);
            let lines: Vec<&str> = text.lines().collect();
            let caret = lines.iter().position(|l| l.trim() == ">").expect("the caret");
            let last = plain.lines().next_back().expect("a last line");
            assert_eq!(lines[caret - 1], last, "{what}: the way out sits above the prompt");
        }
    }

    /// The `tracks` view as the owner sees it, committed beside the mock it is
    /// drawn from — held as a CHECKED golden rather than dumped once, so it
    /// cannot quietly stop being what the code renders (the T2/T3/T5 pattern).
    ///
    /// The 2-player shape, because that is the count the live gate game is
    /// played at and the only one where the neutral dummy is on the board.
    /// Plain text, because a capture full of escapes is one nobody can read in a
    /// diff. Regenerate with `UPDATE_T6_CAPTURE=1 cargo test -p yokohama-cli
    /// --lib the_committed_tracks_capture` after a deliberate change, and LOOK
    /// AT THE DIFF before committing it.
    #[test]
    fn the_committed_tracks_capture_is_what_the_renderer_draws() {
        let (tracks, _, colors) = view_models(2);
        assert_capture("t6-tracks-view.txt", &strip_ansi(&tracks_view(&tracks, &colors)));
    }

    /// The `sites` view's golden, at the same count and on the same terms.
    #[test]
    fn the_committed_sites_capture_is_what_the_renderer_draws() {
        let (_, board, colors) = view_models(2);
        assert_capture("t6-sites-view.txt", &strip_ansi(&sites_view(&board, &colors)));
    }

    /// Compare `rendered` against the committed capture of that name, or
    /// rewrite it under `UPDATE_T6_CAPTURE`.
    pub(crate) fn assert_capture(name: &str, rendered: &str) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/plans/2026-08-05-phase3.5a-mocks/captures")
            .join(name);
        if std::env::var_os("UPDATE_T6_CAPTURE").is_some() {
            std::fs::create_dir_all(path.parent().expect("a parent"))
                .expect("the captures directory");
            std::fs::write(&path, rendered).expect("write the capture");
        }
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("the committed capture {name}: {e}"));
        assert_eq!(
            committed, rendered,
            "the committed {name} capture is stale; regenerate it with UPDATE_T6_CAPTURE=1",
        );
    }

    /// The widest printed reward in FULL WORDS still fits its column on both
    /// surfaces. Swept over the whole registry rather than the cards this board
    /// happened to deal, because the widest one is what matters and which card
    /// carries it is an accident of the shuffle.
    #[test]
    fn the_widest_printed_reward_fits_a_site_card() {
        let rules = yokohama_core::Rules::standard();
        let widest = rules
            .building_sites
            .iter()
            .flat_map(|s| s.spaces.iter())
            .map(crate::card_render::space_reward_words)
            .max_by_key(|w| w.chars().count())
            .expect("the standard registry deals 24 site cards");
        // The reward column runs from 11 to 31 (`pad_to` advances by at least
        // one, so a reward exactly 20 wide still leaves the builder mark whole).
        assert!(
            widest.chars().count() < 31 - 11,
            "{widest:?} is {} columns; the sites view gives a reward {} before the builder mark",
            widest.chars().count(),
            31 - 11,
        );
    }
}
