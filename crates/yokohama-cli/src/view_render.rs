//! The full-screen VIEWS — spec §0.5's whole roster (plans T6 and T11; design
//! authority for the first two is the approved mock pair
//! `2026-08-05-phase3.5a-mocks/tracks-view.ans` and `sites-view.ans`, and the
//! rest are drawn to their pattern).
//!
//! **Six surfaces plus the menu over them.** T6 built `tracks` and `sites`;
//! T11 added `ports`, `labs`, `hand` and `log`, promoted the `view` menu to a
//! full-screen surface of its own, and made all seven share one title shape
//! ([`view_title`]) and one exit clause ([`exit_line`]). The seventh member of
//! §0.5's list, `goals`, is the achievement INDEX rather than a view here —
//! T6's supersession ruling, since the index draws the same three cards with
//! claim state and per-seat progress.
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
//!
//! **A view says how to leave in words that depend on what it covered up** —
//! [`ViewContext`], the six-lens F1 fix. Everything else here renders from its
//! model alone; the exit clause is the one line whose truth is a fact about the
//! PROMPT UNDERNEATH, so it is the one thing a caller has to tell these
//! functions.

use yokohama_core::{AreaId, PlayerColor, PlayerId, PortZone};

use crate::board_render::{color_of, exact, Twin};
use crate::card_render;
use crate::card_render::{bottom, interior, wrap_words, CARD_W};
use crate::describe;
use crate::screen::FLOOR_W;
use crate::screen_model::{
    menu_number, BoardModel, ChurchSpaceModel, CustomsSpaceModel, HandPanel, LabBoardModel,
    LabSlotModel, LabsPanel, LogPanel, PortBoardModel, PortSlotModel, PortsPanel, TileModel,
    TracksPanel,
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
pub fn tracks_view(
    tracks: &TracksPanel,
    colors: &[PlayerColor],
    context: ViewContext,
) -> String {
    let mut out = String::new();
    out.push_str(&view_title(
        "TRACKS",
        "every Church and Customs space as a card: what it pays, and who is standing on it",
    ));

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
    out.push_str(&exit_line(context));
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
pub fn sites_view(
    board: &BoardModel,
    colors: &[PlayerColor],
    context: ViewContext,
) -> String {
    let mut out = String::new();
    out.push_str(&view_title(
        "BUILDING SITES",
        "every area's site card: each space's printed reward, what is built, and by whom",
    ));

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
    out.push_str(&exit_line(context));
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
// The ports view (T11)
// ---------------------------------------------------------------------------

/// A Laboratory offer card's width. Three to a row, exactly filling the floor at
/// a one-column gutter — [`crate::card_render::GOAL_W`]'s arithmetic at a
/// different card count, and for the same reason it is wide: the purchase text
/// is a SENTENCE (the longest of the twenty runs 103 characters) and a tech
/// nobody can read the effect of is not an offer.
const LAB_VIEW_W: usize = 66;

/// The `ports` view's height in rows at each player count, indexed
/// `players - 2` — MEASURED then set, on [`TRACKS_VIEW_ROWS`]'s terms.
///
/// It varies with the only thing that can vary it: Port B exists at 3 and 4
/// players and not at 2, so the taller two counts draw a second board.
#[cfg(test)]
const PORTS_VIEW_ROWS: [usize; 3] = [16, 25, 25];

/// The `labs` view's height in rows at each player count. Laboratory B is a
/// FOUR-player board only, so 2P and 3P draw one.
#[cfg(test)]
const LABS_VIEW_ROWS: [usize; 3] = [22, 22, 36];

/// The `hand` view's height in rows. It does NOT vary with the player count —
/// a hand is one seat's cards and every seat starts with one — but it does vary
/// with the hand, and the recorded figure is the OPENING one the golden
/// captures, on a row of cards that has not yet grown past eight.
#[cfg(test)]
const HAND_VIEW_ROWS: usize = 14;

/// The `log` view's height at a FULL tail — [`crate::repl::LOG_VIEW_TAIL`]
/// entries, which is the tallest this surface ever draws, and the figure that
/// sizes that constant.
#[cfg(test)]
const LOG_VIEW_ROWS: usize = 48;

/// Both Port boards, every slot as the order card that is on it.
///
/// **The power gate is what makes this a decision surface rather than a list.**
/// A Port take reaches the LOW zone at power 2, MID at 3 and HIGH at 4, so the
/// question a player brings here is not "which cards are out" but "which of them
/// can I reach this turn" — and the answer is printed on the slot label, from
/// the engine's own gate ([`crate::screen_model::port_zone_min_power`]) rather
/// than from a table in this file.
pub fn ports_view(ports: &PortsPanel, context: ViewContext) -> String {
    let mut out = String::new();
    out.push_str(&view_title(
        "PORT OFFER",
        "the order cards on display, the zone each sits in, and the action power that zone takes",
    ));

    for board in &ports.boards {
        out.push_str(&port_rule(board));
        out.push('\n');
        let per_row = ((FLOOR_W + GAP) / (CARD_W + GAP)).max(1);
        for chunk in board.slots.chunks(per_row) {
            for line in slot_label_row(chunk.iter().map(port_slot_label).collect()) {
                out.push_str(&line);
                out.push('\n');
            }
            // An EMPTY slot draws an empty box rather than a card, so the row
            // keeps its shape and the hole reads as "nothing here" instead of
            // shunting the cards to its right one column left.
            let drawn: Vec<Vec<String>> = chunk
                .iter()
                .map(|s| match s.card {
                    Some(id) => card_render::order_card(&card_render::CardFace::new(
                        id,
                        s.printed.as_ref(),
                    )),
                    None => empty_card(CARD_W),
                })
                .collect();
            for line in card_row(&drawn) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out.push('\n');
    }

    out.push_str(&paint(
        Tone::Chrome,
        "A take slides the remaining cards LEFT to close the gap, then refills from the RIGHT — so slot 1 always holds the card that has been out longest.",
    ));
    out.push('\n');
    out.push_str(&deck_line("Order deck", ports.deck_remaining));
    out.push('\n');
    out.push_str(&exit_line(context));
    out.push('\n');
    out.push_str(&paint(Tone::Prompt, "> "));
    out.push('\n');
    out
}

/// `PORT A — take ONE card; LOW needs power 2, MID 3, HIGH 4.  4 of 6 slots filled`
///
/// **The power gate is stated ONCE per board rather than once per slot**, which
/// is where it went at the T11 visual pass. Printed on every label it filled the
/// card's whole 24 columns — correct, asserted, and flush against its
/// neighbour — and it was the same sentence six times over, which is chrome a
/// reader stops seeing. Said once it is a rule; the label then carries the zone,
/// which is the part that differs per slot.
///
/// Each zone's number is [`crate::screen_model::PortSlotModel::min_power`], i.e.
/// still the ENGINE's own `port_accessible_zones` — one statement of a derived
/// fact, not a literal that happens to be typed once.
fn port_rule(board: &PortBoardModel) -> String {
    let filled = board.slots.iter().filter(|s| s.card.is_some()).count();
    let mut t = Twin::default();
    t.tinted(Tone::Strong, &describe::port_board_name(board.board_b).to_uppercase());
    t.raw(&format!(" {} take ONE card; ", glyph::SECTION_DASH));
    t.raw(&zone_gate_clause(board));
    t.raw(".  ");
    t.tinted(Tone::Chrome, &format!("{filled} of {} slots filled", board.slots.len()));
    t.painted
}

/// `LOW needs power 2, MID 3, HIGH 4` — every zone this board actually has, in
/// slot order, each with the gate its own slots carry.
///
/// Built from the board's slots rather than from a list of zones, so a board
/// that stopped having a HIGH zone would stop claiming one.
fn zone_gate_clause(board: &PortBoardModel) -> String {
    let mut seen: Vec<(PortZone, u8)> = Vec::new();
    for slot in &board.slots {
        if !seen.iter().any(|(zone, _)| *zone == slot.zone) {
            seen.push((slot.zone, slot.min_power));
        }
    }
    seen.iter()
        .enumerate()
        .map(|(i, (zone, power))| {
            // The first clause carries the noun and the rest inherit it —
            // "LOW needs power 2, MID 3, HIGH 4" rather than the same five words
            // three times.
            if i == 0 {
                format!("{} needs power {power}", port_zone_name(*zone))
            } else {
                format!("{} {power}", port_zone_name(*zone))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `slot 1 · LOW` — the label row above a card, at exactly [`CARD_W`] columns
/// so it lines up with the card underneath it.
fn port_slot_label(slot: &PortSlotModel) -> Twin {
    let mut t = Twin::default();
    t.tinted(Tone::Chrome, &format!("slot {}", menu_number(slot.slot)));
    t.raw(&format!(" {} ", glyph::SEPARATOR));
    t.tinted(Tone::Strong, port_zone_name(slot.zone));
    t
}

/// The three Port zones as the board prints them.
fn port_zone_name(zone: PortZone) -> &'static str {
    match zone {
        PortZone::Low => "LOW",
        PortZone::Mid => "MID",
        PortZone::High => "HIGH",
    }
}

// ---------------------------------------------------------------------------
// The labs view (T11)
// ---------------------------------------------------------------------------

/// Both Laboratory boards, every slot as the technology on offer there.
///
/// **This is the OFFER, and the technology index is the IDENTIFICATION
/// surface** (T6's two-surfaces ruling). The index answers "which card is
/// this?" over all twenty titles; this one answers "what can I buy right now,
/// and what does it cost" over the at-most-twelve on the table — which is why
/// the price is here and the copy accounting is not.
pub fn labs_view(labs: &LabsPanel, context: ViewContext) -> String {
    let mut out = String::new();
    out.push_str(&view_title(
        "LABORATORY OFFER",
        "the technologies on display, what each one does, and the industry it costs in its slot",
    ));

    for board in &labs.boards {
        out.push_str(&lab_rule(board));
        out.push('\n');
        let per_row = ((FLOOR_W + GAP) / (LAB_VIEW_W + GAP)).max(1);
        for chunk in board.slots.chunks(per_row) {
            let drawn: Vec<Vec<String>> = chunk.iter().map(lab_card).collect();
            for line in card_row(&drawn) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out.push('\n');
    }

    out.push_str(&paint(
        Tone::Chrome,
        "Industry comes from your action power, plus 2 yen or 1 imported good per point — so a card one point out of reach costs 2 yen more, not a turn.",
    ));
    out.push('\n');
    out.push_str(&paint(
        Tone::Chrome,
        "You may own only ONE technology of a given name; ten of the twenty titles are in the box twice, so two players can each hold one.",
    ));
    out.push('\n');
    out.push_str(&deck_line("Technology deck", labs.deck_remaining));
    out.push('\n');
    out.push_str(&exit_line(context));
    out.push('\n');
    out.push_str(&paint(Tone::Prompt, "> "));
    out.push('\n');
    out
}

/// `LABORATORY A — pay the industry printed below …  5 of 6 slots filled`
fn lab_rule(board: &LabBoardModel) -> String {
    let filled = board.slots.iter().filter(|s| s.card.is_some()).count();
    let mut t = Twin::default();
    t.tinted(Tone::Strong, &describe::lab_board_name(board.board_b).to_uppercase());
    t.raw(&format!(
        " {} acquire ONE card by paying its industry; the slot's own surcharge is already in the figure.  ",
        glyph::SECTION_DASH,
    ));
    t.tinted(Tone::Chrome, &format!("{filled} of {} slots filled", board.slots.len()));
    t.painted
}

/// One Laboratory slot as a card, every line exactly [`LAB_VIEW_W`] columns.
fn lab_card(slot: &LabSlotModel) -> Vec<String> {
    let mut top = Twin::default();
    top.raw(&format!("{} ", glyph::BOX_TL));
    top.tinted(Tone::Strong, &format!("slot {}", menu_number(slot.slot)));
    top.raw(&format!(" {} ", glyph::SEPARATOR));
    top.tinted(Tone::Chrome, &format!("surcharge +{}", slot.surcharge));
    top.raw(" ");
    let fill = (LAB_VIEW_W - 1).saturating_sub(top.width());
    top.raw(&String::from(glyph::RULE).repeat(fill));
    top.raw(&glyph::BOX_TR.to_string());

    let mut rows = vec![exact(&top, LAB_VIEW_W)];
    match slot.printed.as_ref() {
        None => {
            let mut empty = Twin::default();
            empty.tinted(
                Tone::Dim,
                if slot.card.is_some() { "not in the registry" } else { "empty" },
            );
            rows.push(interior(&empty, LAB_VIEW_W));
            while rows.len() < LAB_CARD_H - 1 {
                rows.push(interior(&Twin::default(), LAB_VIEW_W));
            }
        }
        Some(printed) => {
            let mut name = Twin::default();
            name.tinted(Tone::Strong, &printed.name.to_uppercase());
            name.pad_to(32);
            name.tinted(Tone::Flag(printed.country), describe::country_name(printed.country));
            rows.push(interior(&name, LAB_VIEW_W));

            let mut cost = Twin::default();
            cost.tinted(
                Tone::Reachable,
                &format!("industry {}", printed.industry_cost + slot.surcharge),
            );
            cost.pad_to(14);
            cost.tinted(
                Tone::Chrome,
                &format!(
                    "card {} + slot {} {} pay the rest in yen or imports",
                    printed.industry_cost, slot.surcharge, glyph::SEPARATOR,
                ),
            );
            rows.push(interior(&cost, LAB_VIEW_W));

            for piece in wrap_words(describe::tech_effect_text(printed.effect), LAB_VIEW_W - 4) {
                let mut t = Twin::default();
                t.tinted(Tone::Chrome, &piece);
                rows.push(interior(&t, LAB_VIEW_W));
            }
            while rows.len() < LAB_CARD_H - 1 {
                rows.push(interior(&Twin::default(), LAB_VIEW_W));
            }
        }
    }
    rows.push(bottom(LAB_VIEW_W));
    rows
}

/// Lines per Laboratory offer card, so a one-line effect and the longest one
/// still square a row off.
///
/// MEASURED then set, over the whole registry rather than the six a board
/// dealt: two rows of chrome (the title and the bottom), the name row, the cost
/// row, and the widest wrap of any of the twenty effect texts — which at this
/// interior is two lines, where the same text costs three on the narrower
/// [`crate::card_render::TECH_W`] index card. Pinned as an EQUALITY by
/// `every_lab_card_is_the_same_height_across_the_whole_registry`, so headroom
/// nobody measured cannot creep in here.
const LAB_CARD_H: usize = 6;

// ---------------------------------------------------------------------------
// The hand view (T11)
// ---------------------------------------------------------------------------

/// One seat's own order cards, drawn at full size.
///
/// **This is the surface the dashboard's hand row promises** (T10): below the
/// board at 3 and 4 players the hand is a NAMED row rather than a drawn card,
/// and the name is only worth printing if the word `hand` opens the cards.
///
/// **The mask is the MODEL's** ([`HandPanel::cards`] is empty whenever
/// `hidden`), so this renderer cannot leak a hidden hand even if it tried to
/// draw one — the leak fence T5 made structural, holding on one more surface.
pub fn hand_view(
    hand: &HandPanel,
    color: Option<PlayerColor>,
    is_bot: bool,
    context: ViewContext,
) -> String {
    let mut out = String::new();
    let who = match color {
        Some(c) => crate::render::color_name(c).to_uppercase(),
        None => format!("SEAT {}", hand.seat.0),
    };
    out.push_str(&view_title(
        &format!("{who}'S HAND"),
        "your order cards at full size: the goods each one needs, and what it pays when you fulfil it",
    ));

    out.push_str(&hand_rule(hand, is_bot));
    out.push('\n');

    if !hand.cards.is_empty() {
        let per_row = ((FLOOR_W + GAP) / (CARD_W + GAP)).max(1);
        for chunk in hand.cards.chunks(per_row) {
            let drawn: Vec<Vec<String>> = chunk
                .iter()
                .map(|c| {
                    card_render::order_card(
                        // **`your starting order`, byte for byte the dashboard's
                        // own note for the same card** (T11 review N5): one
                        // wording, so a player who learns the panel's phrase
                        // meets it again here instead of a second phrase for
                        // one fact.
                        //
                        // The leading `= ` the review asked to keep is dropped
                        // with it, on a MEASUREMENT: `= your starting order` is
                        // 21 columns against a 20-column card interior
                        // ([`card_render::CARD_INNER`]) and clips to
                        // `= your starting ord…`. Keeping the prefix and the
                        // words was not available, and dropping the prefix is
                        // the better half to lose — `= ` is the LOCATION
                        // annotation's mark (`= Port A slot 1`, `= Seat 0's
                        // deal`), which answers where a card already is. This
                        // note answers what the card IS, and losing the prefix
                        // makes the two surfaces identical rather than merely
                        // word-identical.
                        &card_render::CardFace::new(c.id, c.printed.as_ref()).note(
                            c.is_starting_order.then(|| "your starting order".to_string()),
                        ),
                    )
                })
                .collect();
            for line in card_row(&drawn) {
                out.push_str(&line);
                out.push('\n');
            }
        }
    }
    out.push('\n');

    out.push_str(&paint(
        Tone::Chrome,
        "An order is fulfilled as an additional action: hand over the goods it needs, take what it pays, and the card goes to your completed pile.",
    ));
    out.push('\n');
    out.push_str(&exit_line(context));
    out.push('\n');
    out.push_str(&paint(Tone::Prompt, "> "));
    out.push('\n');
    out
}

/// The hand's own section rule: how many cards, and — when there is nothing to
/// draw — WHY, which is three different facts and never a blank screen.
fn hand_rule(hand: &HandPanel, is_bot: bool) -> String {
    let mut t = Twin::default();
    t.tinted(Tone::Strong, "ORDER CARDS");
    t.raw(&format!(" {} ", glyph::SECTION_DASH));
    if hand.hidden {
        t.tinted(
            Tone::Chrome,
            &format!(
                "{} {} contents hidden {} ",
                card_render::plural(hand.card_count, "card"),
                glyph::SEPARATOR,
                glyph::SECTION_DASH,
            ),
        );
        t.tinted(
            Tone::Dim,
            if is_bot {
                "a bot seat's hand stays masked whatever 'privacy' says"
            } else {
                "'privacy' shows them"
            },
        );
    } else if hand.card_count == 0 {
        t.tinted(Tone::Dim, "no cards in hand");
    } else {
        t.tinted(Tone::Chrome, &card_render::plural(hand.card_count, "card"));
    }
    t.painted
}

// ---------------------------------------------------------------------------
// The log view (T11)
// ---------------------------------------------------------------------------

/// The move/event trail, each entry described against the state it applied in.
///
/// **Nothing here re-describes anything** — [`LogEntryModel::text`] is the
/// string `Session` cached at the moment the event applied, so a trading-house
/// take still quotes the price it actually paid and a movement fee still names
/// the President owner who actually received it. This renderer arranges; the
/// history is already written.
pub fn log_view(log: &LogPanel, context: ViewContext) -> String {
    let mut out = String::new();
    out.push_str(&view_title(
        "LOG",
        "what has happened so far, each line worded as it read at the moment it applied",
    ));

    out.push_str(&log_rule(log));
    out.push('\n');

    if log.entries.is_empty() {
        out.push_str(&paint(Tone::Dim, "  nothing has been applied yet"));
        out.push('\n');
    }
    for entry in &log.entries {
        let mut t = Twin::default();
        t.raw("  ");
        t.tinted(Tone::Chrome, &format!("{:>4}", entry.index));
        t.raw("  ");
        t.raw(&entry.text);
        // Clipped to the floor rather than wrapped: a log is read down its
        // index column, and a wrapped entry breaks that column for every entry
        // under it. The whole text stays reachable — `save` writes the record.
        out.push_str(&clipped(&t, FLOOR_W));
        out.push('\n');
    }
    out.push('\n');

    // The `back` clause is this view's own second claim about the prompt
    // underneath, so it branches with the exit line for the reason
    // [`exit_line`] gives: mid-capture nothing has been applied to take off,
    // and `back` there is answered with "this reveal is waiting for an answer".
    out.push_str(&paint(
        Tone::Chrome,
        match context {
            ViewContext::Decision => "Each line reads as it did when it applied, so an old entry still quotes the price it paid and the player it paid. 'back' takes the last one off.",
            ViewContext::Capture => "Each line reads as it did when it applied, so an old entry still quotes the price it paid and the player it paid.",
        },
    ));
    out.push('\n');
    out.push_str(&exit_line(context));
    out.push('\n');
    out.push_str(&paint(Tone::Prompt, "> "));
    out.push('\n');
    out
}

/// `EVENTS — 10 of 47 shown · the 37 before them are in the save file`
fn log_rule(log: &LogPanel) -> String {
    let mut t = Twin::default();
    t.tinted(Tone::Strong, "EVENTS");
    t.raw(&format!(" {} ", glyph::SECTION_DASH));
    let omitted = log.omitted();
    if omitted == 0 {
        t.tinted(
            Tone::Chrome,
            &format!(
                "all {} {} so far",
                log.total,
                if log.total == 1 { "entry" } else { "entries" },
            ),
        );
    } else {
        t.tinted(
            Tone::Chrome,
            &format!(
                "{} of {} shown  {}  the {} before them are in the save file",
                log.entries.len(),
                log.total,
                glyph::SEPARATOR,
                omitted,
            ),
        );
    }
    t.painted
}

// ---------------------------------------------------------------------------
// The view menu (T11)
// ---------------------------------------------------------------------------

/// The numbered menu over every surface in this module and every card index —
/// spec §0.2's "every decision reachable by numbered menu", applied to looking
/// things up.
///
/// **Labels arrive as strings and the numbering is this renderer's**, which is
/// the same split the option zone holds: the REPL owns which surfaces exist and
/// what each row DOES; nothing about a row's number or tint reaches back there.
/// A caller passing labels rather than a menu type is deliberate — the
/// `(label, fn)` pairing that keeps text and behaviour together lives in
/// `Repl::VIEW_MENU`, and a renderer that could see the functions would be a
/// renderer holding a `&mut Repl`.
pub fn view_menu(labels: &[&str]) -> String {
    let mut out = String::new();
    // **Two families, said as two** (T11 review N3). The old gloss read "each
    // row here is its full surface", which is true of ten of the thirteen: the
    // six views and the four card indexes replace the frame. Board, Players and
    // Score do not — they ARE the frame's own panels at full size, so they
    // print back into it. A player who took the promise literally would expect
    // three screens that never arrive.
    out.push_str(&view_title(
        "GAME INFO",
        "the 'view' pattern: every panel on the frame is a summary, and most rows here open its full surface — Board, Players and Score redraw the frame instead",
    ));
    for (i, label) in labels.iter().enumerate() {
        let mut t = Twin::default();
        t.raw("  ");
        t.tinted(Tone::Strong, &format!("[{}]", menu_number(i)));
        t.pad_to(7);
        t.raw(label);
        out.push_str(&clipped(&t, FLOOR_W));
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&paint(
        Tone::Chrome,
        "Every row here is also a word you can type at the prompt, and the two run the same code — so this menu is a reminder, never a second way to be wrong.",
    ));
    out.push('\n');
    out.push_str(&paint(
        Tone::Chrome,
        "Pick a number to open a surface, or 'cancel' to go back to the prompt underneath with nothing entered.",
    ));
    out.push('\n');
    out.push_str(&paint(Tone::Prompt, "> "));
    out.push('\n');
    out
}

// ---------------------------------------------------------------------------
// Shared
// ---------------------------------------------------------------------------

/// The two opening lines every full-screen surface leads with: its NAME in the
/// standing `… — full view` form, and one sentence saying what is on it.
///
/// **One function rather than six copies, and a per-surface gloss rather than
/// one repeated sentence.** T6 shipped the two views it wrote with different
/// title shapes — `tracks` spent its line teaching what a "view" IS, `sites`
/// spent its describing the cards — and at six surfaces neither generalizes: a
/// sentence repeated six times is chrome a reader learns to skip, and a shape
/// that differs per surface is six surfaces rather than one system. So the
/// SHAPE is shared (which is what makes them read as one) and the SENTENCE is
/// each surface's own (which is what makes it worth reading). The pattern
/// itself is taught where the roster is — the `view` menu's own gloss.
fn view_title(name: &str, gloss: &str) -> String {
    titled(&format!("{name} {} full view", glyph::SECTION_DASH), gloss)
}

/// [`view_title`] with the headline supplied whole, for the one full-screen
/// surface that is not on the `view` roster.
///
/// The `… — full view` tail is a promise about the ROSTER — it names a screen
/// you reached by typing a word and can reach again — and [`bot_review`] is not
/// one of those: nothing opens it, it arrives on its own and leaves on a
/// keypress. So it shares the SHAPE (which is what makes the surfaces read as
/// one system) and states its own headline (which is what keeps the tail from
/// promising a word that does not exist).
fn titled(headline: &str, gloss: &str) -> String {
    format!("{}   {}\n\n", paint(Tone::Strong, headline), paint(Tone::Chrome, gloss))
}

/// The bot-turn REVIEW: what the opponent did while the player was not looking,
/// on a screen of its own, waiting for one keypress.
///
/// **It exists because the alternative was a frame that did not fit.** On a load
/// with a bot to move, `main` used to compose the pump's whole narrative and the
/// decision prompt into ONE frame; `screen::wrap` leads with clear-screen, so
/// the board's top rows went off a 50-row terminal before they could be read and
/// the only way back to them was to scroll. The narrative is what overflows, so
/// the narrative is what moves — and it turns out to be the half a player wants
/// to read on its own anyway.
///
/// `narrative` arrives already rendered (`Repl::pump_bots`): the described moves
/// and the one aggregate delta that summarizes them. Nothing here re-describes
/// a move, so the review and the `log` cannot disagree about what happened.
///
/// **Scoped to LOAD, deliberately.** Mid-game the same narrative rides the reply
/// to the player's own move, where it belongs — a pause between every turn would
/// be a keypress tax on the ordinary case, and the ordinary case is not the one
/// that broke.
pub fn bot_review(narrative: &str) -> String {
    let mut out = titled(
        &format!("WHILE YOU WERE AWAY {} the bot's turn", glyph::SECTION_DASH),
        "everything the bot did before handing back to you, and what it changed",
    );
    out.push_str(narrative.trim_end_matches('\n'));
    out.push_str("\n\n");
    // Honest about what this screen is: `main` reads one line and throws it
    // away, so promising that a command typed here RUNS -- the way a view
    // promises it -- would be a promise this surface cannot keep.
    out.push_str(&paint(
        Tone::Chrome,
        "Take your time: the board, the dashboard and your options are all one keypress away, and \
         nothing here is waiting on you.",
    ));
    out.push('\n');
    out.push_str(&paint(
        Tone::Chrome,
        "This screen is a pause, not a prompt — press Enter to go on to your turn. Anything else \
         you type here is discarded, and 'log' will still have every line of it.",
    ));
    out.push('\n');
    out.push_str(&paint(Tone::Prompt, "> "));
    out.push('\n');
    out
}

/// [`exact`] without the PADDING half: clip a line that overruns `width`, and
/// leave a short one short.
///
/// The two are different jobs and only one of them belongs on a full-width row.
/// Inside a card, padding is what squares the box off. On a line that is its own
/// row, it is 150 columns of trailing space on every entry — invisible on a
/// terminal, and noise in every golden diff and every piped capture.
fn clipped(t: &Twin, width: usize) -> String {
    let padded = exact(t, width);
    padded.trim_end().to_string()
}

/// `Order deck: 17 cards left.`, or the honest sentence for an empty one.
fn deck_line(what: &str, remaining: usize) -> String {
    let mut t = Twin::default();
    t.tinted(Tone::Chrome, &format!("{what}: "));
    if remaining == 0 {
        t.tinted(Tone::Dim, "empty — nothing refills from here again");
    } else {
        t.raw(&format!("{} left", card_render::plural(remaining, "card")));
    }
    t.painted
}

/// A row of card-width labels above a row of cards, each padded to [`CARD_W`]
/// so a label sits over its own card.
fn slot_label_row(labels: Vec<Twin>) -> Vec<String> {
    let mut line = String::new();
    for (i, label) in labels.iter().enumerate() {
        if i > 0 {
            line.push_str(&" ".repeat(GAP));
        }
        line.push_str(&exact(label, CARD_W));
    }
    while line.ends_with(' ') {
        line.pop();
    }
    vec![line]
}

/// A blank box at `width`, for a slot with nothing on it: the row keeps its
/// shape and the hole reads as a hole.
fn empty_card(width: usize) -> Vec<String> {
    let mut top = Twin::default();
    top.raw(&format!("{} ", glyph::BOX_TL));
    top.tinted(Tone::Dim, "empty");
    top.raw(" ");
    let fill = (width - 1).saturating_sub(top.width());
    top.raw(&String::from(glyph::RULE).repeat(fill));
    top.raw(&glyph::BOX_TR.to_string());
    let mut rows = vec![exact(&top, width)];
    while rows.len() < card_render::CARD_H - 1 {
        rows.push(interior(&Twin::default(), width));
    }
    rows.push(bottom(width));
    rows
}

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

/// What kind of prompt a view was opened OVER — the one thing about the screen
/// underneath that a view has to be told (six-lens F1).
///
/// The sibling surface has had this since T7 ([`crate::index_render::
/// IndexContext`]) and for the same reason: a full-screen overlay makes
/// promises about a prompt it has covered up, and those promises are not the
/// same at every prompt. The two enums are deliberately NOT one type — the
/// index's distinction is overlay-versus-inline (whether there is a screen to
/// close at all), and this one is about what a re-dispatched LINE does, which
/// is a question the index never has to answer because it re-dispatches
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewContext {
    /// Over a prompt whose list is numbered options — the main decision zone,
    /// and every sub-prompt reached from it.
    Decision,
    /// Over a live reveal capture, where a bare number is a CARD IDENTITY that
    /// records a reveal (and, on the answer that completes a group, applies the
    /// move parked behind it).
    Capture,
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
/// **The second half is CONTEXT-DEPENDENT, and was false at a capture until the
/// six-lens fix wave** (F1). Both of its clauses are claims about the prompt a
/// re-dispatched line lands on, and at a live reveal that prompt is not a
/// numbered option list: a digit is an ORDER-CARD IDENTITY that records a
/// reveal and commits the move parked behind it, and `back` undoes nothing
/// while a group is waiting for an answer. The views are reachable from inside
/// a capture on purpose (`Repl::step` intercepts their six words ahead of the
/// collector, and the capture's own hint advertises them), so a player one
/// keystroke away was being told to type a number — an instruction whose result
/// is a wrong card in `GameRecord.events`, silently. The FIRST half is true in
/// both contexts and does not branch: `cancel` typed HERE closes the view and
/// nothing else, which is exactly the distinction a player who has just learned
/// the capture's `cancel` needs drawn.
///
/// The two halves would not fit one line inside [`crate::screen::FLOOR_W`]; the
/// row they cost is recorded in `both_views_fit_the_fifty_row_design_floor`'s
/// measured table rather than absorbed.
fn exit_line(context: ViewContext) -> String {
    let second = match context {
        ViewContext::Decision => {
            "Any other line closes the view and then RUNS at that prompt — a NUMBER applies that option, \
             and 'back' undoes your last move."
        }
        ViewContext::Capture => {
            "Any other line closes the view and then ANSWERS the reveal underneath — a NUMBER there names the CARD \
             it is recording, and 'back' is not an undo while it waits."
        }
    };
    // Painted per line rather than as one string with a newline in it, so
    // neither line carries a tint across the row break.
    format!(
        "{}\n{}",
        paint(
            Tone::Chrome,
            "Enter or 'cancel' closes this view and puts you back at the prompt underneath, exactly as you left it — \
             nothing is entered and nothing there is cancelled.",
        ),
        paint(Tone::Chrome, second),
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

    /// A real generated game at `players`, for the surfaces that read more of
    /// the state than the two T6 views do.
    pub(crate) fn game(players: u8) -> (yokohama_core::GameState, Rules) {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(players), &rules, 1)
            .expect("a standard board");
        let state =
            finalize_setup(pending, &vec![StartingOrderChoice::Keep(0); players as usize])
                .expect("setup finalizes");
        (state, rules)
    }

    /// Every T11 surface at `players`, in one call: the four new views plus the
    /// menu, each labelled, so a sweep over "every view" cannot silently drop
    /// one when a sixth is added.
    fn t11_views(players: u8) -> Vec<(&'static str, String)> {
        let (state, rules) = game(players);
        let seat = state.current_player;
        let hand = crate::screen_model::build_hand(&state, &rules, seat, false).expect("a seat");
        let color = state.players.iter().find(|p| p.id == seat).map(|p| p.color);
        vec![
            ("ports", ports_view(&crate::screen_model::build_ports(&state, &rules), ViewContext::Decision)),
            ("labs", labs_view(&crate::screen_model::build_labs(&state, &rules), ViewContext::Decision)),
            ("hand", hand_view(&hand, color, false, ViewContext::Decision)),
            ("log", crate::repl::Repl::new(walked_session(players)).log_text()),
            ("view menu", view_menu(&["Port offer — the cards on display", "Log — the trail"])),
        ]
    }

    /// A REAL session, walked far enough that the log carries entries from more
    /// than one seat.
    ///
    /// **The fixture the T11 review found fabricated** (F3): the `log` capture
    /// was built from a hand-written vector of strings, which is a surface that
    /// can say anything at all — and did, carrying `Red:`/`Blue:` prefixes and
    /// phrasings the driver never emitted. A committed capture is only worth
    /// what its input is worth, so this one comes from `Session::apply` and is
    /// rendered by `Repl::log_text`, the same call the `log` command makes.
    ///
    /// Ten moves is what it takes at every count for the walk to cross a seat
    /// boundary, which is what makes the actor prefix visible in the capture at
    /// all; `the_captured_log_entries_are_driver_shaped_and_name_more_than_one_actor`
    /// asserts the crossing rather than trusting it. The three offer/hand views
    /// above deliberately stay at their
    /// OPENING state — they are captures of different surfaces, not of one
    /// moment, and re-shooting them mid-walk would churn three goldens to say
    /// nothing new.
    fn walked_session(players: u8) -> crate::session::Session {
        let mut s = crate::session::Session::new_sim(
            GameConfig::for_player_count(players),
            1,
            vec![StartingOrderChoice::Keep(0); players as usize],
            Rules::standard(),
        )
        .expect("a standard board");
        while s.record().events.len() < 10 {
            // The LAST legal move rather than the first: `legal[0]` is the
            // declining end of every list, so a walk down it logs ten lines of
            // `Place no assistants` and shows nothing about how a long entry
            // sits in the column. This end places, moves and pays.
            let mv = s.legal().pop().expect("a legal move at every step");
            s.apply(mv, Vec::new()).expect("the last legal move applies");
        }
        s
    }

    /// **Every line of every T11 surface fits the design floor, at every player
    /// count** — the property that lets them be drawn full-screen with nothing
    /// above, held for the four new views on the same terms as the two T6 ones.
    #[test]
    fn every_t11_view_line_fits_the_design_floor() {
        for players in [2u8, 3, 4] {
            for (what, text) in t11_views(players) {
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

    /// **Every card line measures exactly its card's width** — the round-3.1
    /// lesson on the two card shapes T11 adds (the Laboratory offer card, and
    /// the empty box a vacant Port slot draws). A card that clipped would shear
    /// the whole row it sits in, and it would do it silently.
    #[test]
    fn every_t11_card_line_measures_exactly_its_card_width() {
        let (state, rules) = game(4);
        for board in &crate::screen_model::build_labs(&state, &rules).boards {
            for slot in &board.slots {
                for line in lab_card(slot) {
                    assert_eq!(visible_width(&line), LAB_VIEW_W, "{:?}", strip_ansi(&line));
                }
            }
        }
        for line in empty_card(CARD_W) {
            assert_eq!(visible_width(&line), CARD_W, "{:?}", strip_ansi(&line));
        }
        // The slot label above a Port card is the row's other member and has to
        // measure the same, or the labels walk off their own cards.
        for board in &crate::screen_model::build_ports(&state, &rules).boards {
            for slot in &board.slots {
                assert_eq!(visible_width(&exact(&port_slot_label(slot), CARD_W)), CARD_W);
            }
        }
    }

    /// **Every Laboratory offer card is the same height**, whatever its effect
    /// text runs to — the property [`LAB_CARD_H`] exists for. Without it a row
    /// of three cards would be ragged along its bottom edge, and the card
    /// underneath the short one would start a line early.
    ///
    /// Swept over the whole registry rather than the six a board happened to
    /// deal, because the tallest one is what matters and which slot carries it
    /// is an accident of the shuffle.
    #[test]
    fn every_lab_card_is_the_same_height_across_the_whole_registry() {
        let rules = Rules::standard();
        let mut widest = 0usize;
        for (slot, card) in rules.techs.iter_sorted_by_id().into_iter().enumerate() {
            let model = LabSlotModel {
                slot: slot % 6,
                surcharge: yokohama_core::LabBoard::position_modifier(slot % 6),
                card: Some(card.id),
                printed: Some(card.clone()),
            };
            let drawn = lab_card(&model);
            assert_eq!(
                drawn.len(),
                LAB_CARD_H,
                "{} draws {} rows against LAB_CARD_H {LAB_CARD_H}",
                card.name,
                drawn.len(),
            );
            widest = widest.max(
                crate::card_render::wrap_words(
                    describe::tech_effect_text(card.effect),
                    LAB_VIEW_W - 4,
                )
                .len(),
            );
        }
        // Anti-vacuity: the constant has to be exactly what the tallest card
        // NEEDS, or it is headroom nothing measured. Two rows of chrome (title,
        // bottom) plus the name and cost rows, plus the wrapped effect.
        assert_eq!(
            LAB_CARD_H,
            2 + 2 + widest,
            "the tallest effect text wraps to {widest} lines; LAB_CARD_H must be exactly that \
             plus the title, the bottom, the name row and the cost row",
        );
        // An empty slot squares off to the same height, so a half-filled board
        // still draws a rectangle.
        assert_eq!(
            lab_card(&LabSlotModel { slot: 0, surcharge: 0, card: None, printed: None }).len(),
            LAB_CARD_H,
        );
    }

    /// **The Port power gate on screen is the ENGINE's**, pinned against
    /// `port_accessible_zones` directly rather than against the three numbers
    /// this renderer happens to print. A zone whose gate moved would move the
    /// label with it — which is the whole reason the model asks the engine.
    #[test]
    fn the_port_zone_labels_state_the_engines_own_power_gate() {
        let (state, rules) = game(4);
        let ports = crate::screen_model::build_ports(&state, &rules);
        let text = strip_ansi(&ports_view(&ports, ViewContext::Decision));
        for board in &ports.boards {
            for slot in &board.slots {
                let gate = slot.min_power;
                assert!(
                    yokohama_core::actions::port_accessible_zones(gate).contains(&slot.zone),
                    "power {gate} must reach {:?}",
                    slot.zone,
                );
                assert!(
                    gate == 0
                        || !yokohama_core::actions::port_accessible_zones(gate - 1)
                            .contains(&slot.zone),
                    "power {} must NOT reach {:?} — the label claims {gate} is the lowest",
                    gate - 1,
                    slot.zone,
                );
                assert!(
                    text.contains(&format!(
                        "slot {} · {}",
                        crate::screen_model::menu_number(slot.slot),
                        port_zone_name(slot.zone),
                    )),
                    "every slot names its zone: {text}",
                );
            }
        }
        // The gate is stated once per board, over every zone that board has —
        // and the sweep above proved each of those three numbers is the lowest
        // power the ENGINE lets through, so this line is derived, not typed.
        assert!(text.contains("LOW needs power 2, MID 3, HIGH 4"), "{text}");
        // Anti-vacuity on the fixture: a board with every slot in one zone
        // would pass the sweep above while proving nothing about the gate.
        let zones: std::collections::BTreeSet<&str> =
            ports.boards[0].slots.iter().map(|s| port_zone_name(s.zone)).collect();
        assert_eq!(zones.len(), 3, "fixture: a board carries all three zones");
    }

    /// **A vacant Port slot draws a hole, not a shifted row.** The engine
    /// refills right-to-left after compaction, so a mid-game board legitimately
    /// has empty slots on the right — and a row that closed the gap would put
    /// every label over the wrong card.
    #[test]
    fn an_empty_port_slot_keeps_its_place_in_the_row() {
        let (state, rules) = game(2);
        let mut ports = crate::screen_model::build_ports(&state, &rules);
        assert!(ports.boards[0].slots.iter().all(|s| s.card.is_some()), "fixture: a full board");
        let taken = ports.boards[0].slots[3].card.expect("a dealt card");
        ports.boards[0].slots[3].card = None;
        ports.boards[0].slots[3].printed = None;

        let text = strip_ansi(&ports_view(&ports, ViewContext::Decision));
        assert!(text.contains("empty"), "the hole says so: {text}");
        assert!(
            !text.contains(&format!("O{:02}", taken.0)),
            "and the card that left is gone from the screen: {text}",
        );
        assert!(text.contains("5 of 6 slots filled"), "the section rule counts it: {text}");
        // The label row still names all six slots in order, which is the
        // property a closed gap would break.
        for slot in 1..=6 {
            assert!(text.contains(&format!("slot {slot} · ")), "slot {slot} kept its place: {text}");
        }
    }

    /// **The Laboratory card prices the slot, not just the card** — the one
    /// arithmetic this surface exists to do. `total = industry_cost +
    /// position_modifier` is the engine's own `total_required`
    /// (`actions/cards.rs`), and a screen showing the printed card cost alone
    /// would under-price every slot past the second.
    #[test]
    fn a_lab_card_prices_the_slot_surcharge_into_its_industry_figure() {
        let (state, rules) = game(4);
        let labs = crate::screen_model::build_labs(&state, &rules);
        let text = strip_ansi(&labs_view(&labs, ViewContext::Decision));
        let mut surcharged = 0usize;
        for board in &labs.boards {
            for slot in &board.slots {
                let Some(printed) = slot.printed.as_ref() else { continue };
                assert_eq!(
                    slot.surcharge,
                    yokohama_core::LabBoard::position_modifier(slot.slot),
                    "the surcharge is the engine's",
                );
                assert!(
                    text.contains(&format!(
                        "industry {}",
                        printed.industry_cost + slot.surcharge
                    )),
                    "{} in slot {} must price at {} + {}: {text}",
                    printed.name,
                    slot.slot,
                    printed.industry_cost,
                    slot.surcharge,
                );
                if slot.surcharge > 0 {
                    surcharged += 1;
                }
            }
        }
        assert!(surcharged >= 4, "fixture: the board deals surcharged slots ({surcharged})");
        // The components stay on screen beside the total, so a player can see
        // WHY a slot costs what it does.
        assert!(text.contains("card ") && text.contains(" + slot "), "{text}");
    }

    /// **Every Laboratory card names its country and prints its WHOLE effect
    /// text** — the property F2 put there and the retired `render::labs_view`
    /// held: five effect one-liners carry qualifiers that change a purchase
    /// decision (the cost a card still charges, the restriction it does not
    /// lift), so a card that clipped one would be teaching the defect that
    /// review removed.
    ///
    /// Swept over the WHOLE registry rather than the six a board dealt, each
    /// slotted through the real renderer — never against a copy of its format
    /// string — which is the shape the retired test used and the reason it
    /// would catch a wording edit.
    #[test]
    fn a_lab_card_shows_its_country_and_its_whole_effect_text() {
        let rules = Rules::standard();
        for card in rules.techs.iter_sorted_by_id() {
            let model = LabSlotModel {
                slot: 4,
                surcharge: yokohama_core::LabBoard::position_modifier(4),
                card: Some(card.id),
                printed: Some(card.clone()),
            };
            let drawn = strip_ansi(&lab_card(&model).join("\n"));
            assert!(
                drawn.contains(describe::country_name(card.country)),
                "{} must name its country: {drawn}",
                card.name,
            );
            // The effect WRAPS rather than clipping, so the whole sentence has
            // to be recoverable by putting the wrapped rows back together.
            let flat = drawn
                .lines()
                .map(|l| l.trim_matches(|c| c == '│' || c == ' '))
                .collect::<Vec<_>>()
                .join(" ");
            let effect = describe::tech_effect_text(card.effect);
            assert!(
                flat.contains(effect),
                "{} must print its whole effect, unclipped: want {effect:?} in {flat:?}",
                card.name,
            );
            assert!(!drawn.contains('…'), "{} clipped something: {drawn}", card.name);
        }
    }

    /// **Both offer views state the deck they refill from** — the retired
    /// views' `order deck: N` / `tech deck: N` lines, which are a public fact
    /// about the table (the physical pile's height) and the thing that tells a
    /// player whether an offer will be replaced at all.
    #[test]
    fn both_offers_state_their_shared_deck() {
        let (state, rules) = game(4);
        let ports = crate::screen_model::build_ports(&state, &rules);
        let labs = crate::screen_model::build_labs(&state, &rules);
        assert_eq!(ports.deck_remaining, state.order_deck.len());
        assert_eq!(labs.deck_remaining, state.tech_deck.len());
        assert!(
            strip_ansi(&ports_view(&ports, ViewContext::Decision))
                .contains(&format!("Order deck: {} cards left", state.order_deck.len())),
        );
        assert!(
            strip_ansi(&labs_view(&labs, ViewContext::Decision))
                .contains(&format!("Technology deck: {} cards left", state.tech_deck.len())),
        );
        // Both boards are named, and the second one only where it exists: a 2P
        // table has no Port B and no Laboratory B, and a screen claiming one
        // would send an operator looking for a board that is in the box.
        let four = strip_ansi(&ports_view(&ports, ViewContext::Decision));
        assert!(four.contains("PORT A") && four.contains("PORT B"), "{four}");
        let (two, two_rules) = game(2);
        let small = strip_ansi(&ports_view(&crate::screen_model::build_ports(&two, &two_rules), ViewContext::Decision));
        assert!(small.contains("PORT A") && !small.contains("PORT B"), "{small}");
        let small_labs = strip_ansi(&labs_view(&crate::screen_model::build_labs(&two, &two_rules), ViewContext::Decision));
        assert!(
            small_labs.contains("LABORATORY A") && !small_labs.contains("LABORATORY B"),
            "{small_labs}",
        );
        // An exhausted deck says so rather than printing a bare zero.
        let mut drained = ports.clone();
        drained.deck_remaining = 0;
        assert!(strip_ansi(&ports_view(&drained, ViewContext::Decision)).contains("nothing refills from here again"));
    }

    /// **A hidden hand renders as a count and nothing else** — the leak fence
    /// (spec §0.12) on the surface that would leak most, checked the way T5
    /// checked it: the MODEL is empty, so the renderer has nothing to leak even
    /// if it tried.
    #[test]
    fn a_hidden_hand_view_carries_no_card_identity_at_all() {
        let (state, rules) = game(2);
        let seat = state.current_player;
        let held: Vec<_> = state
            .players
            .iter()
            .find(|p| p.id == seat)
            .expect("a seat")
            .order_cards_in_hand
            .clone();
        assert!(!held.is_empty(), "fixture: the seat holds cards to hide");

        let hidden = crate::screen_model::build_hand(&state, &rules, seat, true).expect("a seat");
        assert!(hidden.cards.is_empty(), "the MODEL masks, not the renderer");
        let text = strip_ansi(&hand_view(&hidden, Some(PlayerColor::Red), false, ViewContext::Decision));
        for id in &held {
            assert!(
                !text.contains(&format!("O{:02}", id.0)),
                "a hidden hand must not name O{:02}: {text}",
                id.0,
            );
        }
        assert!(text.contains("contents hidden"), "and it says why: {text}");
        assert!(text.contains(&format!("{} card", held.len())), "the count is public: {text}");

        // A BOT's hand says the different true thing: 'privacy' will not open
        // it, so telling the player to press it would be a dead instruction.
        let bot = strip_ansi(&hand_view(&hidden, Some(PlayerColor::Red), true, ViewContext::Decision));
        assert!(bot.contains("bot seat"), "{bot}");
        assert!(!bot.contains("'privacy' shows them"), "{bot}");
    }

    /// **The hand view names the starting order and draws every other card
    /// plain** — the one card in a hand that is not like the others, and the
    /// reason the dashboard's own hand card marks it too.
    #[test]
    fn the_hand_view_marks_the_kept_starting_order() {
        let (state, rules) = game(2);
        let seat = state.current_player;
        let hand = crate::screen_model::build_hand(&state, &rules, seat, false).expect("a seat");
        let starting: Vec<_> = hand.cards.iter().filter(|c| c.is_starting_order).collect();
        assert_eq!(starting.len(), 1, "fixture: exactly one kept starting order");

        let text = strip_ansi(&hand_view(&hand, Some(PlayerColor::Red), false, ViewContext::Decision));
        // The dashboard's note, verbatim — one wording for one fact (T11 N5),
        // and short enough to draw whole on a 20-column card interior, which
        // `= your starting order` was not.
        assert!(text.contains("your starting order"), "{text}");
        assert_eq!(
            text.matches("your starting order").count(),
            1,
            "and only on the one card it belongs to: {text}",
        );
        assert!(!text.contains('\u{2026}'), "and it is not clipped: {text}");
        for card in &hand.cards {
            assert!(text.contains(&format!("O{:02}", card.id.0)), "every card is drawn: {text}");
        }
    }

    /// **The log view arranges; it never re-describes.** The entry text is the
    /// string the caller supplies, verbatim — the fold cache's own output,
    /// which is what keeps an old entry quoting the price it actually paid.
    #[test]
    fn the_log_view_prints_supplied_descriptions_verbatim_with_their_global_index() {
        let entries = vec![
            (7, "Red: Employment Agency — take a trading house for 4 yen".to_string()),
            (8, "Blue: Move President to Bank (A06) — pay 1 yen (1 to Red)".to_string()),
        ];
        let text = strip_ansi(&log_view(&crate::screen_model::build_log(41, entries.clone()), ViewContext::Decision));
        for (index, description) in &entries {
            assert!(text.contains(description), "verbatim: {text}");
            assert!(
                text.contains(&format!("{index:>4}  {description}")),
                "with its global index, which is what a bug report quotes: {text}",
            );
        }
        // The omitted count is stated rather than left to be inferred from a
        // trail that just starts partway through.
        assert!(text.contains("2 of 41 shown"), "{text}");
        assert!(text.contains("the 39 before them"), "{text}");

        // A short game says the honest other thing.
        let all = strip_ansi(&log_view(&crate::screen_model::build_log(2, vec![
            (0, "Red: End additional-action phase".to_string()),
            (1, "Red: Place no assistants".to_string()),
        ]), ViewContext::Decision));
        assert!(all.contains("all 2 entries so far"), "{all}");
        assert!(!all.contains("shown"), "{all}");

        // And an empty one does not draw a blank screen.
        let empty = strip_ansi(&log_view(&crate::screen_model::build_log(0, Vec::new()), ViewContext::Decision));
        assert!(empty.contains("nothing has been applied yet"), "{empty}");
    }

    /// **The committed `log` capture's entries are DRIVER-SHAPED, and the
    /// capture shows more than one actor** (T11 review F3).
    ///
    /// The renderer above is deliberately verbatim, which means the capture is
    /// only as truthful as whatever produced its strings — and what produced
    /// them was a hand-written vector carrying prefixes and phrasings the
    /// driver never emitted. The capture now comes from `Session::apply` via
    /// `Repl::log_text`, so the shape is a consequence rather than a claim, and
    /// this test pins the two things that make the capture worth committing:
    /// every entry reads as `<Colour>: <what happened>` off the record, and
    /// the walk really did cross a seat boundary, so the capture DEMONSTRATES
    /// attribution rather than merely allowing it.
    #[test]
    fn the_captured_log_entries_are_driver_shaped_and_name_more_than_one_actor() {
        for players in [2u8, 3, 4] {
            let session = walked_session(players);
            let text = strip_ansi(&crate::repl::Repl::new(session).log_text());
            let colours: std::collections::BTreeSet<String> = text
                .lines()
                .filter_map(|line| {
                    let line = line.trim_start();
                    let (head, _) = line.split_once("  ")?;
                    let index: usize = head.parse().ok()?;
                    let entry = line.split_once("  ")?.1.trim_start();
                    let (colour, rest) = entry.split_once(": ")?;
                    assert!(
                        !rest.is_empty() && index < 10,
                        "{players}P entry {index} is not `<Colour>: <what happened>`: {entry:?}",
                    );
                    Some(colour.to_string())
                })
                .collect();
            assert!(
                colours.len() >= 2,
                "{players}P: the walk must cross a seat boundary or the capture proves nothing \
                 about attribution — saw {colours:?}",
            );
            for colour in &colours {
                assert!(
                    [
                        PlayerColor::Red,
                        PlayerColor::Blue,
                        PlayerColor::Yellow,
                        PlayerColor::Green
                    ]
                    .iter()
                    .any(|c| crate::render::color_name(*c) == colour),
                    "{colour:?} is not a seat colour this driver names",
                );
            }
        }
    }

    /// **The view menu numbers from [`NUMBER_BASE`]** — T5's ruling, on the
    /// surface it was first written for.
    #[test]
    fn the_view_menu_numbers_its_rows_from_the_numbering_base() {
        let labels = ["First surface", "Second surface", "Third surface"];
        let text = strip_ansi(&view_menu(&labels));
        for (i, label) in labels.iter().enumerate() {
            assert!(
                text.contains(&format!("[{}]  {label}", crate::screen_model::menu_number(i))),
                "row {i}: {text}",
            );
        }
        assert!(!text.contains("[0]"), "1-based: {text}");
        assert!(text.contains("'cancel'"), "and it says how to leave: {text}");
    }

    /// **Every T11 surface stays byte-plain with color off and paints with it
    /// on** — spec §0.12's honesty net, on the four new views and the menu.
    /// Color is FORCED ON in the same test, so it cannot pass by the renderer
    /// painting nothing at all.
    #[test]
    fn the_t11_views_are_escape_free_with_color_off_and_painted_with_it_on() {
        for (what, plain) in t11_views(2) {
            crate::style::force(true);
            let painted = t11_views(2)
                .into_iter()
                .find(|(name, _)| *name == what)
                .map(|(_, text)| text)
                .expect("the same surface");
            crate::style::force(false);
            assert!(!plain.contains('\u{1b}'), "{what}: piped output must carry no escapes");
            assert!(painted.contains('\u{1b}'), "{what}: a real terminal gets color");
            assert_eq!(strip_ansi(&painted), plain, "{what}: the two differ only by escapes");
        }
    }

    /// The four new views fit the 50-row design floor at every player count —
    /// stated as the MEASURED table rather than as a bare bound, exactly as
    /// `both_views_fit_the_fifty_row_design_floor` states the T6 pair's.
    ///
    /// The `log` figure is the one that could grow without a layout change, so
    /// it is measured at a FULL tail ([`crate::repl::LOG_VIEW_TAIL`] entries),
    /// which is the tallest that surface ever draws.
    #[test]
    fn the_t11_views_fit_the_fifty_row_design_floor() {
        for players in [2u8, 3, 4] {
            let (state, rules) = game(players);
            let seat = state.current_player;
            let hand =
                crate::screen_model::build_hand(&state, &rules, seat, false).expect("a seat");
            for (what, text, recorded) in [
                (
                    "ports",
                    ports_view(&crate::screen_model::build_ports(&state, &rules), ViewContext::Decision),
                    PORTS_VIEW_ROWS[usize::from(players - 2)],
                ),
                (
                    "labs",
                    labs_view(&crate::screen_model::build_labs(&state, &rules), ViewContext::Decision),
                    LABS_VIEW_ROWS[usize::from(players - 2)],
                ),
                ("hand", hand_view(&hand, Some(PlayerColor::Red), false, ViewContext::Decision), HAND_VIEW_ROWS),
                (
                    "log",
                    log_view(&crate::screen_model::build_log(
                        1_000,
                        (0..crate::repl::LOG_VIEW_TAIL)
                            .map(|i| (i, format!("Red: a described event {i}")))
                            .collect(),
                    ), ViewContext::Decision),
                    LOG_VIEW_ROWS,
                ),
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

    /// **Every view leads with the same two lines**, so the pattern is taught
    /// once rather than in six wordings — the reason [`view_title`] is a
    /// function. Swept over all six surfaces plus the menu.
    #[test]
    fn every_full_screen_surface_opens_with_the_standing_title_shape() {
        let (tracks, board, colors) = view_models(2);
        let mut surfaces = vec![
            ("tracks", tracks_view(&tracks, &colors, ViewContext::Decision)),
            ("sites", sites_view(&board, &colors, ViewContext::Decision)),
        ];
        surfaces.extend(t11_views(2));
        let mut glosses: Vec<String> = Vec::new();
        for (what, text) in &surfaces {
            let lines: Vec<&str> = text.lines().collect();
            let title = strip_ansi(lines[0]);
            let (name, gloss) = title
                .split_once(" — full view   ")
                .unwrap_or_else(|| panic!("{what}: the title is `NAME — full view   <gloss>`: {title:?}"));
            assert!(
                name.chars().all(|c| !c.is_lowercase()),
                "{what}: the surface's name leads, in caps: {name:?}",
            );
            assert!(gloss.len() > 40, "{what}: and says what is on it: {gloss:?}");
            assert_eq!(lines[1], "", "{what}: a blank row under the title");
            glosses.push(gloss.to_string());
        }
        // The SHAPE is shared and the SENTENCE is not: six identical glosses
        // would be the chrome `view_title`'s doc says a reader learns to skip.
        let unique: std::collections::BTreeSet<&String> = glosses.iter().collect();
        assert_eq!(unique.len(), glosses.len(), "every surface glosses itself: {glosses:?}");
        // The pattern itself is taught exactly once, where the roster is.
        let teaching: Vec<&(&str, String)> = surfaces
            .iter()
            .filter(|(_, text)| text.contains("every panel on the frame is a summary"))
            .collect();
        assert_eq!(teaching.len(), 1, "the pattern is taught once");
        assert_eq!(teaching[0].0, "view menu");
    }

    /// **Every view except the menu says how to leave in the standing words**,
    /// and says them directly above the caret — T6 review M2c's contract, held
    /// on the four surfaces T11 adds.
    ///
    /// The MENU is deliberately excluded and says a different true thing: it is
    /// a one-shot pick, so a number there opens a surface rather than applying
    /// an option, and re-dispatch is not what it does.
    #[test]
    fn every_t11_view_carries_the_standing_exit_clause_above_its_caret() {
        let last = strip_ansi(&exit_line(ViewContext::Decision));
        let last = last.lines().next_back().expect("a last line").to_string();
        for (what, text) in t11_views(2) {
            let text = strip_ansi(&text);
            let lines: Vec<&str> = text.lines().collect();
            let caret = lines.iter().position(|l| l.trim() == ">").expect("the caret");
            if what == "view menu" {
                assert!(text.contains("'cancel' to go back"), "{what}: {text}");
                assert!(!text.contains("RUNS"), "the menu does not re-dispatch: {text}");
            } else {
                assert_eq!(lines[caret - 1], last, "{what}: the way out sits above the prompt");
            }
        }
    }

    /// The four new views as the owner sees them, committed beside the mocks —
    /// CHECKED goldens rather than one-off dumps, the T2/T3/T5/T6 pattern.
    /// Regenerate with `UPDATE_T6_CAPTURE=1` and LOOK AT THE DIFF.
    #[test]
    fn the_committed_t11_captures_are_what_the_renderers_draw() {
        for (what, text) in t11_views(2) {
            assert_capture(&format!("t11-{}.txt", what.replace(' ', "-")), &strip_ansi(&text));
        }
    }

    /// Every line of both views fits the design floor, at every player count —
    /// the property that lets them be drawn full-screen with nothing above.
    #[test]
    fn every_view_line_fits_the_design_floor() {
        for players in [2u8, 3, 4] {
            let (tracks, board, colors) = view_models(players);
            for (what, text) in
                [("tracks", tracks_view(&tracks, &colors, ViewContext::Decision)), ("sites", sites_view(&board, &colors, ViewContext::Decision))]
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
        let text = strip_ansi(&tracks_view(&tracks, &colors, ViewContext::Decision));

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
        let placed = strip_ansi(&tracks_view(&tracks, &colors, ViewContext::Decision));
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
        let text = strip_ansi(&tracks_view(&tracks, &colors, ViewContext::Decision));

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
        let text = strip_ansi(&tracks_view(&three, &colors, ViewContext::Decision));
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
        let text = strip_ansi(&sites_view(&board, &colors, ViewContext::Decision));

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

        let text = strip_ansi(&sites_view(&board, &colors, ViewContext::Decision));
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

        let text = strip_ansi(&sites_view(&board, &colors, ViewContext::Decision));
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
            ("tracks", tracks_view(&tracks, &colors, ViewContext::Decision), {
                crate::style::force(true);
                let p = tracks_view(&tracks, &colors, ViewContext::Decision);
                crate::style::force(false);
                p
            }),
            ("sites", sites_view(&board, &colors, ViewContext::Decision), {
                crate::style::force(true);
                let p = sites_view(&board, &colors, ViewContext::Decision);
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
                ("tracks", tracks_view(&tracks, &colors, ViewContext::Decision), TRACKS_VIEW_ROWS),
                ("sites", sites_view(&board, &colors, ViewContext::Decision), SITES_VIEW_ROWS[usize::from(players - 2)]),
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
    ///
    /// **Every assertion here names a ROW, not the two-line string** (six-lens
    /// F2's NAMED CHECK, and the seventh recurrence of the overlapping-text
    /// class): the two contexts share their first line word for word, so a
    /// `contains` over the whole footer is satisfied by the half that never
    /// varies, and the sibling test below — which is the one that has to bite —
    /// would have been pinning nothing.
    #[test]
    fn the_exit_line_says_a_number_applies_the_option_underneath() {
        let plain = strip_ansi(&exit_line(ViewContext::Decision));
        let rows: Vec<&str> = plain.lines().collect();
        assert_eq!(rows.len(), 2, "two lines: {plain:?}");
        assert!(rows[0].contains("Enter") && rows[0].contains("'cancel'"), "{plain}");
        assert!(
            rows[1].contains("Any other line") && rows[1].contains("RUNS"),
            "the re-dispatch rule is stated as a rule about LINES, not commands: {plain}",
        );
        assert!(
            rows[1].contains("NUMBER applies that option"),
            "and the consequence a player would not expect is named: {plain}",
        );
        // `back` is the one word whose meaning differs from the index's, so the
        // view says what it does here rather than leaving it to be discovered.
        assert!(rows[1].contains("'back' undoes your last move"), "{plain}");

        // Each row is tinted on its own, so neither carries a colour across the
        // break — the reason `exit_line` paints twice instead of painting one
        // string with a newline in it.
        crate::style::force(true);
        let painted = exit_line(ViewContext::Decision);
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
            [("tracks", tracks_view(&tracks, &colors, ViewContext::Decision)), ("sites", sites_view(&board, &colors, ViewContext::Decision))]
        {
            let text = strip_ansi(&text);
            let lines: Vec<&str> = text.lines().collect();
            let caret = lines.iter().position(|l| l.trim() == ">").expect("the caret");
            let last = plain.lines().next_back().expect("a last line");
            assert_eq!(lines[caret - 1], last, "{what}: the way out sits above the prompt");
        }
    }

    /// **Over a reveal capture the second line says what a number does THERE**
    /// (six-lens F1).
    ///
    /// Both of the decision wording's clauses are false at a capture — a digit
    /// is an order-card identity that records a reveal and commits the parked
    /// move, and `back` undoes nothing while a group waits — and the views are
    /// reachable from inside one by design, so following the footer wrote a
    /// wrong card into `GameRecord.events` with no refusal and no echo.
    ///
    /// **Written row by row, and the discriminating assertion is the LAST one**:
    /// a fix that took the context and ignored it would keep every positive
    /// clause here true, so the capture footer is also asserted NOT to carry the
    /// decision sentence. The shared first line is asserted equal on purpose —
    /// `cancel` typed at the view closes the view in both contexts, and a
    /// capture-flavoured rewording of that half would be a second lie.
    #[test]
    fn the_capture_exit_line_names_the_card_question_and_not_an_option() {
        let decision = strip_ansi(&exit_line(ViewContext::Decision));
        let capture = strip_ansi(&exit_line(ViewContext::Capture));
        let d: Vec<&str> = decision.lines().collect();
        let c: Vec<&str> = capture.lines().collect();
        assert_eq!(c.len(), 2, "two lines here too: {capture:?}");
        assert_eq!(c[0], d[0], "the close-this-view half does not vary: {capture:?}");

        assert!(c[1].contains("names the CARD"), "the number's real meaning: {capture}");
        assert!(
            c[1].contains("'back' is not an undo"),
            "and the other false clause is corrected, not dropped: {capture}",
        );
        assert!(
            !c[1].contains("applies that option"),
            "the sentence that is false at a capture must be GONE from the row that carries it — \
             a context the renderer accepts and ignores would pass every check above: {capture}",
        );
        assert!(!c[1].contains("undoes your last move"), "{capture}");

        // Width: the footer is drawn full-screen, so it owes the design floor
        // the same as every other line on these surfaces.
        for row in c.iter().chain(d.iter()) {
            assert!(
                crate::screen::visible_width(row) <= FLOOR_W,
                "the exit line fits the design floor: {} columns\n{row}",
                crate::screen::visible_width(row),
            );
        }
    }

    /// The `log` view's own `back` clause branches with the footer, because it
    /// is the same claim about the same prompt underneath.
    #[test]
    fn the_log_view_only_promises_back_where_back_takes_something_off() {
        let entries = vec![(0, "Red: Place no assistants".to_string())];
        let panel = crate::screen_model::build_log(1, entries);
        let decision = strip_ansi(&log_view(&panel, ViewContext::Decision));
        let capture = strip_ansi(&log_view(&panel, ViewContext::Capture));
        assert!(decision.contains("'back' takes the last one off"), "{decision}");
        assert!(
            !capture.contains("'back' takes the last one off"),
            "nothing has been applied to take off while a reveal is waiting: {capture}",
        );
        // The half that is true in both survives the branch.
        for text in [&decision, &capture] {
            assert!(text.contains("Each line reads as it did when it applied"), "{text}");
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
        assert_capture("t6-tracks-view.txt", &strip_ansi(&tracks_view(&tracks, &colors, ViewContext::Decision)));
    }

    /// The `sites` view's golden, at the same count and on the same terms.
    #[test]
    fn the_committed_sites_capture_is_what_the_renderer_draws() {
        let (_, board, colors) = view_models(2);
        assert_capture("t6-sites-view.txt", &strip_ansi(&sites_view(&board, &colors, ViewContext::Decision)));
    }

    /// Compare `rendered` against the committed capture of that name, or
    /// rewrite it when blessed.
    ///
    /// **THE one capture helper for the whole crate.** Four other modules had
    /// hand-copied this read-compare-or-rewrite block, each behind its own
    /// task-numbered variable, so a "regenerate the captures" sweep driven off
    /// any one doc comment left the other goldens unblessed and the next diff
    /// mixed a real change with a stale one. `board_render`, `screen`,
    /// `index_render` and this module all call THIS now (T12).
    pub(crate) fn assert_capture(name: &str, rendered: &str) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/plans/2026-08-05-phase3.5a-mocks/captures")
            .join(name);
        if blessing_captures() {
            std::fs::create_dir_all(path.parent().expect("a parent"))
                .expect("the captures directory");
            std::fs::write(&path, rendered).expect("write the capture");
        }
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("the committed capture {name}: {e}"));
        assert_eq!(
            committed, rendered,
            "the committed {name} capture is stale; regenerate it with UPDATE_CAPTURES=1 and LOOK \
             AT THE DIFF before committing",
        );
    }

    /// Whether this run may rewrite the committed captures.
    ///
    /// Checked for PRESENCE, not for a value: `UPDATE_CAPTURES=0` and
    /// `UPDATE_CAPTURES=` bless exactly as `=1` does — the variable is a
    /// deliberate act, and a shell that exports it empty still meant it. The
    /// per-task names are kept as aliases so the task-numbered doc comments
    /// scattered through the crate stay true; `UPDATE_CAPTURES` is the one that
    /// blesses every capture at once.
    fn blessing_captures() -> bool {
        [
            "UPDATE_CAPTURES",
            "UPDATE_T2_CAPTURE",
            "UPDATE_T3_CAPTURE",
            "UPDATE_T5_CAPTURE",
            "UPDATE_T6_CAPTURE",
            "UPDATE_T10_CAPTURE",
        ]
        .iter()
        .any(|key| std::env::var_os(key).is_some())
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
