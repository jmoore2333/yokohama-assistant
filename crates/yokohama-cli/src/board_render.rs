//! The V3 board: staggered rows of fixed-width tiles, drawn from the screen
//! model alone (Phase-3.5a spec §0.4, plan T2; design authority is the approved
//! mock pair `2026-08-05-phase3.5a-mocks/main-frame.ans` +
//! `main-frame-mid.ans`).
//!
//! **Its own half of the R9 seam, and the import list is the proof.** This
//! module renders; it never reads game state. Its inputs are
//! [`crate::screen_model`]'s pure data plus a seat color table, and the only
//! `yokohama_core` names it touches are the four TYPES a tile has to match on
//! ([`AreaType`], [`FivePowerEffect`], [`SpaceReward`], [`PlayerId`]/
//! [`PlayerColor`]). `GameState`, `Rules` and `Session` are absent by
//! construction — the adapter that reaches for them is
//! [`crate::render::board_grid`], one function, on the other side of this
//! seam. Glyphs come only from [`crate::style::glyph`] and color only from
//! [`crate::style::Tone`].
//!
//! **ONE tile shape renders every state.** The mocks show two: a six-row
//! turn-1 tile with a `President: none` line, and a five-row mid-game tile
//! whose freed line carries the site-slot strip. The ruled V3 is the five-row
//! one, and the turn-1 state is not a different shape — it is that shape with
//! nothing standing on it, so the presence line reads `no pawns` and there is
//! no `President: none` line anywhere. The president has moved onto the TITLE
//! as `▲` in its owner's color, which is the whole point of V3: one sweep of
//! the title row answers "where are the presidents".
//!
//! **Totality.** Every emitted line measures exactly [`TILE_W`] visible
//! columns — [`exact`] is the one place that is enforced, it is applied to
//! every line without exception, and it pads short lines and clips long ones
//! rather than letting either shear the frame the board is joined into.
//! Content that cannot fit is a real possibility rather than a theoretical
//! one: a nine-character 5-Power token beside a two-digit site index fills the
//! interior to the last column (the round-3.1 overflow), and a crowded tile
//! can outrun the presence line, which is why that line has a measured
//! fallback rather than a hope.

use std::collections::BTreeMap;

use yokohama_core::{AreaType, FivePowerEffect, PlayerColor, PlayerId, SpaceReward};

use crate::describe;
use crate::screen_model::{BoardModel, TileModel};
use crate::style::{glyph, paint, GoodKind, Tone};

/// One tile's total width, borders included — the mocks' `TILE_W`, and the
/// pitch between neighbours in a row (tiles touch, `┐┌`, the way they sit on
/// the physical table). Rows below indent by HALF of it: the brick offset of
/// the staggered pyramid.
pub const TILE_W: usize = 30;

/// Interior text width: a tile spends two columns on borders and one on pad
/// either side of its content.
const INNER: usize = TILE_W - 4;

/// Lines per tile: the title, three content rows, the bottom edge. Named
/// because the board assembler stacks rows of tiles on it, and a shape change
/// that moved it would otherwise be two edits in two files.
const TILE_LINES: usize = 5;

/// The Station's tile marker. ASCII, and deliberately not vocabulary: no
/// approved mock shows a placed Station (the illustrative states have none),
/// so there is no mock glyph to adopt, and the marker the shipped board
/// already draws is carried over verbatim rather than invented. Dropping it
/// was the other option and it is not one — the Station grants EVERY player +1
/// Power in its area, which is a fact the board must show at a glance.
const STATION: &str = "[St]";

// ---------------------------------------------------------------------------
// Painted/plain twins
// ---------------------------------------------------------------------------

/// A line under construction, kept as (plain, painted) twins.
///
/// The crate's standing alignment rule is that escapes carry bytes and no
/// display width, so any fixed-width decision must be taken on the PLAIN text
/// (style.rs's module doc). Accumulating both at once is how that rule stops
/// being a thing every emitter has to remember: [`Twin::width`] is always the
/// plain width, and [`exact`] is handed both.
///
/// `pub(crate)` since T3: it is the crate's fixed-width line primitive, not the
/// board's private one, and [`crate::panel_render`] builds dashboard rows under
/// the same rule. Shared rather than copied — a second Twin would be a second
/// place for the plain/painted pair to drift apart, which is the one thing this
/// type exists to prevent. It stays declared HERE, with the surface that first
/// needed it, rather than being moved into `style`: `style` owns which escapes
/// exist, and this owns how a line is measured while it is being built.
#[derive(Default)]
pub(crate) struct Twin {
    pub(crate) plain: String,
    pub(crate) painted: String,
}

impl Twin {
    /// Append text that takes no color (structure, separators, labels).
    pub(crate) fn raw(&mut self, s: &str) -> &mut Self {
        self.plain.push_str(s);
        self.painted.push_str(s);
        self
    }

    /// Append text in a tone. `paint` is the identity while color is off, so
    /// the two twins stay equal in that state — which is what makes the piped
    /// protocol byte-plain without this module doing anything.
    pub(crate) fn tinted(&mut self, tone: Tone, s: &str) -> &mut Self {
        self.plain.push_str(s);
        self.painted.push_str(&paint(tone, s));
        self
    }

    /// Append a token in `seat`'s color, or uncolored when the caller's
    /// table does not name that seat — an unknown seat loses its tint and
    /// keeps its mark, never the other way round.
    pub(crate) fn owned(&mut self, colors: &[PlayerColor], seat: PlayerId, s: &str) -> &mut Self {
        match color_of(colors, seat) {
            Some(c) => self.tinted(Tone::Player(c), s),
            None => self.raw(s),
        }
    }

    pub(crate) fn width(&self) -> usize {
        self.plain.chars().count()
    }

    /// Pad with spaces until the PLAIN text reaches `column`, always advancing
    /// by at least one space so two fields can never run together when the
    /// first one overruns its budget.
    pub(crate) fn pad_to(&mut self, column: usize) -> &mut Self {
        let pad = column.saturating_sub(self.width()).max(1);
        self.raw(&" ".repeat(pad))
    }
}

/// `seat`'s color, or `None` for a seat the supplied table does not cover.
/// The table is the caller's `player_colors` in seat order; a model can carry
/// a [`PlayerId`] this renderer has no color for (a fixture, a stale record),
/// and inventing one would attribute a pawn to the wrong player.
pub(crate) fn color_of(colors: &[PlayerColor], seat: PlayerId) -> Option<PlayerColor> {
    colors.get(usize::from(seat.0)).copied()
}

/// Emit `t`'s painted text at EXACTLY `width` visible columns.
///
/// Short pads. Long falls back to the PLAIN text, clipped and closed with `…`:
/// an escape run cannot be cut safely at an arbitrary column (the cut would
/// leave the terminal in the tone), and a clipped word is a smaller lie than a
/// sheared frame. This is the only width decision in the module, so every line
/// inherits the guarantee whether or not its builder thought about it.
///
/// `pub(crate)` for the same reason as [`Twin`]: the dashboard's boxed rows
/// (its section rules and its hand card) hold the identical guarantee.
pub(crate) fn exact(t: &Twin, width: usize) -> String {
    let measured = t.width();
    if measured == width {
        return t.painted.clone();
    }
    if measured < width {
        return format!("{}{}", t.painted, " ".repeat(width - measured));
    }
    let mut clipped: String = t.plain.chars().take(width.saturating_sub(1)).collect();
    clipped.push(glyph::ELLIPSIS);
    clipped.chars().take(width).collect()
}

/// A content line wrapped in its box sides: `│ ` + exactly [`INNER`] columns +
/// ` │`.
fn interior(t: &Twin) -> String {
    format!("{} {} {}", glyph::BOX_V, exact(t, INNER), glyph::BOX_V)
}

// ---------------------------------------------------------------------------
// One tile
// ---------------------------------------------------------------------------

/// One V3 tile as five lines, each exactly [`TILE_W`] visible columns.
///
/// `colors` is the seat color table (seat order), the one presentation fact
/// [`TileModel`] does not carry: the model names a pawn's owner by
/// [`PlayerId`], and the owner-tinted marks need that seat's color.
pub fn tile_lines(tile: &TileModel, colors: &[PlayerColor]) -> Vec<String> {
    let mut lines = vec![title_line(tile, colors)];
    if tile.area_type == AreaType::Canal {
        lines.extend(canal_lines());
    } else {
        lines.push(interior(&token_line(tile)));
        lines.push(interior(&presence_line(tile, colors)));
        lines.push(interior(&site_strip(tile, colors)));
    }
    lines.push(format!(
        "{}{}{}",
        glyph::BOX_BL,
        String::from(glyph::RULE).repeat(TILE_W - 2),
        glyph::BOX_BR,
    ));
    lines
}

/// The title: `┌ A01 BANK ▲ ◈ ─────┐` — the id small, the FULL name in bold,
/// then every mark that stands on this tile, then rule to the corner.
///
/// The name is what gives, and it gives last: the marks are the tile's live
/// state and the name is a label the player can also read off the physical
/// board, so a title that cannot hold both clips the NAME (with `…`) rather
/// than dropping a president.
fn title_line(tile: &TileModel, colors: &[PlayerColor]) -> String {
    let id = format!("A{:02}", tile.area.0);

    let mut marks = Twin::default();
    for &seat in &tile.presidents {
        marks.raw(" ").owned(colors, seat, &glyph::PRESIDENT.to_string());
    }
    if tile.has_station {
        marks.raw(" ").tinted(Tone::Strong, STATION);
    }
    if tile.reachable {
        marks.raw(" ").tinted(Tone::Reachable, &glyph::REACHABLE.to_string());
    }

    // `┌` + ` ` + id + ` ` + name + marks + ` ` + fill + `┐`.
    let fixed = 1 + 1 + id.chars().count() + 1 + marks.width() + 1 + 1;
    let name = describe::area_type_name(tile.area_type).to_uppercase();
    let budget = TILE_W.saturating_sub(fixed);
    let name = if name.chars().count() > budget {
        let mut clipped: String = name.chars().take(budget.saturating_sub(1)).collect();
        clipped.push(glyph::ELLIPSIS);
        clipped
    } else {
        name
    };

    let mut t = Twin::default();
    t.raw(&glyph::BOX_TL.to_string())
        .raw(" ")
        .tinted(Tone::Chrome, &id)
        .raw(" ")
        .tinted(Tone::Strong, &name);
    t.plain.push_str(&marks.plain);
    t.painted.push_str(&marks.painted);
    t.raw(" ");
    let fill = TILE_W.saturating_sub(t.width() + 1);
    t.raw(&String::from(glyph::RULE).repeat(fill)).raw(&glyph::BOX_TR.to_string());
    exact(&t, TILE_W)
}

/// `5-Power: 3 Seafood Site 12` — the face-up token in its good's color and
/// the building site's printed index.
///
/// ONE space between them, which is the four-player mock's spacing rather than
/// the two-player mock's two: at nine token characters and a two-digit index
/// the two-space form measures 27 into a 26-column interior, and that is
/// exactly the round-3.1 overflow. One shape renders every tile, so the
/// spacing that fits every tile is the one it uses.
///
/// `None` means TAKEN. The model flattens "claimed" and "no slot at all" into
/// one `None` ([`TileModel::token`]), and the Canal — the one tile that never
/// had a slot — never reaches this line, so on any tile that does, an absent
/// token is one a player has already collected.
fn token_line(tile: &TileModel) -> Twin {
    let mut t = Twin::default();
    t.raw("5-Power: ");
    match tile.token {
        Some(effect) => {
            let (label, kind) = five_power_token(effect);
            t.tinted(Tone::Good(kind), &label);
        }
        None => {
            t.tinted(Tone::Dim, "taken");
        }
    }
    let site = match tile.site.as_ref().and_then(|s| s.index) {
        Some(i) => i.to_string(),
        None => "?".to_string(),
    };
    t.raw(" ").tinted(Tone::Chrome, &format!("Site {site}"));
    t
}

/// The board's compact 5-Power token (`1 Import`, `3 Seafood`) and the color
/// it carries.
///
/// Deliberately NOT [`describe::five_power_effect_label`], which words the same
/// six faces as prose for option text and the `log` (`1 imported goods`). This
/// is a board TOKEN: title case, short enough for a tile, and matching the
/// approved mocks' faces character for character.
fn five_power_token(effect: FivePowerEffect) -> (String, GoodKind) {
    match effect {
        FivePowerEffect::ImportedGoods(n) => (format!("{n} Import"), GoodKind::Imports),
        FivePowerEffect::Copper(n) => (format!("{n} Copper"), GoodKind::Copper),
        FivePowerEffect::Silk(n) => (format!("{n} Silk"), GoodKind::Silk),
        FivePowerEffect::Tea(n) => (format!("{n} Tea"), GoodKind::Tea),
        FivePowerEffect::Seafood(n) => (format!("{n} Seafood"), GoodKind::Seafood),
        FivePowerEffect::Money(n) => (format!("{n} Yen"), GoodKind::Yen),
    }
}

/// The assistants standing here: one `●` run per seat in its color, capped at
/// four glyphs with `+n` for the rest, or a dim `no pawns`.
///
/// The runs have a MEASURED fallback, not an assumed fit. Four seats at their
/// widest (`●●●●+16`) want 34 columns of a 26-column interior, so a crowded
/// tile would clip a player's pawns away entirely — and a pawn the board does
/// not draw is a pawn the player does not know about. When the runs do not
/// fit, the line falls back to the letter-and-count form the pre-redesign
/// board used (`R12 B3`), which is bounded at 19 columns for four seats and
/// tells the whole truth in less space.
fn presence_line(tile: &TileModel, colors: &[PlayerColor]) -> Twin {
    let present: Vec<(PlayerId, u8)> =
        tile.assistants.iter().copied().filter(|&(_, n)| n > 0).collect();
    if present.is_empty() {
        let mut t = Twin::default();
        t.tinted(Tone::Dim, "no pawns");
        return t;
    }

    let mut runs = Twin::default();
    for (i, &(seat, n)) in present.iter().enumerate() {
        if i > 0 {
            runs.raw("  ");
        }
        runs.owned(colors, seat, &pawn_run(n));
    }
    if runs.width() <= INNER {
        return runs;
    }

    let mut counts = Twin::default();
    for (i, &(seat, n)) in present.iter().enumerate() {
        if i > 0 {
            counts.raw(" ");
        }
        let letter = color_of(colors, seat).map(color_letter).unwrap_or('?');
        counts.owned(colors, seat, &format!("{letter}{n}"));
    }
    counts
}

/// `●●●●+3` — up to four pawn glyphs, then the remainder as a count.
fn pawn_run(n: u8) -> String {
    let shown = usize::from(n.min(4));
    let mut run = String::from(glyph::ASSISTANT).repeat(shown);
    if n > 4 {
        run.push_str(&format!("+{}", n - 4));
    }
    run
}

/// `sites ★1 ⌂ — t2  │ T7` — the building site's four printed spaces followed
/// by the trading-house VP.
///
/// A built space shows its builder's `⌂` in that player's color; an empty one
/// shows what it still pays, in the mocks' compact token vocabulary (the
/// legend under the board is the key). A space with nothing printed on it —
/// every card in the set prints one, the ML space — is an em dash.
fn site_strip(tile: &TileModel, colors: &[PlayerColor]) -> Twin {
    let mut t = Twin::default();
    let Some(site) = tile.site.as_ref() else {
        t.tinted(Tone::Dim, "no building site");
        return t;
    };

    t.raw("sites ");
    for (i, slot) in site.slots.iter().enumerate() {
        if i > 0 {
            t.raw(" ");
        }
        match slot.built_by {
            Some(seat) => {
                t.owned(colors, seat, &glyph::BUILT.to_string());
            }
            None => push_reward(&mut t, &slot.reward),
        }
    }

    t.raw("  ").tinted(Tone::Chrome, &glyph::BOX_V.to_string()).raw(" ");
    let house = format!("T{}", site.trading_house_vp);
    match site.trading_house_owner {
        Some(seat) => t.owned(colors, seat, &house),
        None => t.raw(&house),
    };
    t
}

/// One space's printed reward as compact tokens, or an em dash for a blank
/// space. Multi-part rewards (`{vp: 2, yen: 1}` is printed on four of the
/// twenty-four cards) emit both tokens in this fixed order, so the same card
/// always reads the same way.
fn push_reward(t: &mut Twin, r: &SpaceReward) {
    let parts: [(u8, char, Tone); 9] = [
        (r.vp, glyph::VP, Tone::Vp),
        (r.yen, glyph::YEN, Tone::Good(GoodKind::Yen)),
        (r.seafood, 'f', Tone::Good(GoodKind::Seafood)),
        (r.tea, 't', Tone::Good(GoodKind::Tea)),
        (r.silk, 's', Tone::Good(GoodKind::Silk)),
        (r.copper, 'c', Tone::Good(GoodKind::Copper)),
        (r.any_good, '*', Tone::Good(GoodKind::Any)),
        // The YAML's `assistants` icon; the engine calls the field `movement`
        // (rulebook p.10 — the icon MOVES an assistant), and the mocks' legend
        // words it back as `a=assistants`.
        (r.movement, 'a', Tone::ActionIcon),
        // The YAML's `shophouse_build` icon, `warehouse_tokens` in the engine.
        (r.warehouse_tokens, glyph::BUILT, Tone::ActionIcon),
    ];
    let mut printed = false;
    for (n, mark, tone) in parts {
        if n > 0 {
            t.tinted(tone, &format!("{mark}{n}"));
            printed = true;
        }
    }
    if !printed {
        t.tinted(Tone::Chrome, &glyph::SECTION_DASH.to_string());
    }
}

/// The Canal's three interior lines: it is transit only, so it has no token,
/// no site and nothing that can ever stand on it, and its own shape says so
/// rather than drawing three empty rows of a shape that does not apply.
fn canal_lines() -> Vec<String> {
    let mut toll = Twin::default();
    toll.tinted(Tone::Good(GoodKind::Yen), &format!("{}1", glyph::YEN))
        .raw(" to the bank / crossing");

    let mut first = Twin::default();
    first.tinted(Tone::Dim, &format!("transit only {} no pawns,", glyph::SECTION_DASH));
    let mut second = Twin::default();
    second.tinted(Tone::Dim, "no tokens, no buildings");

    vec![interior(&first), interior(&second), interior(&toll)]
}

// ---------------------------------------------------------------------------
// The board
// ---------------------------------------------------------------------------

/// The whole staggered board: tiles grouped into rows by their own
/// `grid_position`, each row indented half a tile below the one above, every
/// line padded to one width so a side panel can be joined line by line without
/// shear.
///
/// No board SHAPE is assumed — rows and columns come from the model, so a
/// fixture geometry (or a future player count) draws by the same rule the
/// pyramid does, and a gap in a row holds its column rather than closing up.
pub fn board_view(board: &BoardModel, colors: &[PlayerColor]) -> String {
    let mut rows: BTreeMap<u8, BTreeMap<u8, &TileModel>> = BTreeMap::new();
    for tile in &board.tiles {
        rows.entry(tile.grid_position.0).or_default().insert(tile.grid_position.1, tile);
    }
    if rows.is_empty() {
        return String::new();
    }

    let indent_of = |r: u8| usize::from(r) * (TILE_W / 2);
    let span_of = |r: u8, cols: &BTreeMap<u8, &TileModel>| {
        indent_of(r) + (usize::from(*cols.keys().next_back().expect("a non-empty row")) + 1) * TILE_W
    };
    let width = rows.iter().map(|(&r, cols)| span_of(r, cols)).max().expect("a non-empty board");

    let mut out = String::new();
    for (&r, cols) in &rows {
        let mut lines = vec![(indent_of(r), " ".repeat(indent_of(r))); TILE_LINES];
        for col in 0..=*cols.keys().next_back().expect("a non-empty row") {
            match cols.get(&col) {
                Some(tile) => {
                    for (line, cell) in lines.iter_mut().zip(tile_lines(tile, colors)) {
                        line.0 += TILE_W;
                        line.1.push_str(&cell);
                    }
                }
                None => {
                    for line in lines.iter_mut() {
                        line.0 += TILE_W;
                        line.1.push_str(&" ".repeat(TILE_W));
                    }
                }
            }
        }
        for (measured, text) in lines {
            out.push_str(&text);
            out.push_str(&" ".repeat(width - measured));
            out.push('\n');
        }
    }
    out
}

/// The key to everything the board draws: the tile marks, then the site-slot
/// tokens, then the Canal's crossing rule on a board that has one.
///
/// Both lines exist because the V3 tile is denser than the shape it replaced:
/// the marks moved onto the title and the freed line became a strip of compact
/// tokens, and a compact token nobody can decode is decoration. The site-slot
/// half is the approved mock's own legend; the marks half is its companion,
/// since the mocks explain the strip in the frame and the marks in prose.
///
/// `has_canal` is the ONE board fact this legend reads, so it is taken as that
/// fact rather than as a whole [`BoardModel`] (T2 review N3). It used to take
/// the model and scan its tiles, which meant every composed frame built a
/// second full model — one for the board, one to answer a single yes/no — and a
/// model built to be scanned once for a boolean is a cost with no reader.
pub fn board_legend(has_canal: bool) -> String {
    let sep = format!(" {} ", glyph::SEPARATOR);
    let mut out = Twin::default();
    out.tinted(Tone::Chrome, "marks: ")
        .raw(&glyph::PRESIDENT.to_string())
        .tinted(Tone::Chrome, " president (owner color)")
        .tinted(Tone::Chrome, &sep)
        .raw(&glyph::ASSISTANT.to_string())
        .tinted(Tone::Chrome, " assistants (4, then +n)")
        .tinted(Tone::Chrome, &sep)
        .tinted(Tone::Reachable, &glyph::REACHABLE.to_string())
        .tinted(Tone::Chrome, " a Movement destination the engine offers")
        .tinted(Tone::Chrome, &sep)
        .raw(STATION)
        .tinted(Tone::Chrome, " Station (+1 Power for everyone)");

    let mut slots = Twin::default();
    slots
        .tinted(Tone::Chrome, "site slots: ")
        // White, not bare: the approved mock paints this sample `⌂` in the
        // site-slot token table's own white (T3's carried-over T2 flag). It is
        // a SAMPLE of a mark the strip draws in its builder's color, so it
        // takes the neutral tone the rest of that table takes rather than
        // claiming an owner it does not have.
        .tinted(Tone::ActionIcon, &glyph::BUILT.to_string())
        .tinted(Tone::Chrome, " built (owner color)")
        .tinted(Tone::Chrome, &sep)
        .tinted(Tone::Vp, &format!("{}n", glyph::VP))
        .tinted(Tone::Chrome, " VP")
        .tinted(Tone::Chrome, &sep)
        .tinted(Tone::Good(GoodKind::Yen), &format!("{}n", glyph::YEN))
        .tinted(Tone::Chrome, " yen")
        .tinted(Tone::Chrome, &sep)
        .tinted(Tone::Good(GoodKind::Seafood), "f")
        .tinted(Tone::Chrome, "/")
        .tinted(Tone::Good(GoodKind::Tea), "t")
        .tinted(Tone::Chrome, "/")
        .tinted(Tone::Good(GoodKind::Silk), "s")
        .tinted(Tone::Chrome, "/")
        .tinted(Tone::Good(GoodKind::Copper), "c")
        .tinted(Tone::Chrome, "n goods")
        .tinted(Tone::Chrome, &sep)
        .tinted(Tone::Good(GoodKind::Any), "*n")
        .tinted(Tone::Chrome, " any good")
        .tinted(Tone::Chrome, &sep)
        // The KEY has to paint in the same tone as the token it keys, or the
        // legend stops being a key -- so these two follow `push_reward` onto
        // `ActionIcon` rather than staying bold beside a white `a1` on a tile.
        .tinted(Tone::ActionIcon, "an")
        .tinted(Tone::Chrome, " assistants")
        .tinted(Tone::Chrome, &sep)
        .tinted(Tone::ActionIcon, &format!("{}n", glyph::BUILT))
        .tinted(Tone::Chrome, " build")
        .tinted(Tone::Chrome, &sep)
        .tinted(Tone::Chrome, "Tn trading-house VP")
        .tinted(Tone::Chrome, &format!(" {} 'sites' renders the full cards", glyph::SECTION_DASH));

    let mut text = format!("{}\n{}", out.painted, slots.painted);
    if has_canal {
        let mut canal = Twin::default();
        canal.tinted(
            Tone::Chrome,
            &format!(
                "Canal: the President may cross shore to shore for {}1 to the bank \
                 ('sites' lists the pairs)",
                glyph::YEN,
            ),
        );
        text.push('\n');
        text.push_str(&canal.painted);
    }
    text
}

/// A player color's initial, the letter the crowded-tile fallback counts with.
fn color_letter(c: PlayerColor) -> char {
    match c {
        PlayerColor::Red => 'R',
        PlayerColor::Blue => 'B',
        PlayerColor::Yellow => 'Y',
        PlayerColor::Green => 'G',
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::test_support::assert_exact_width;
    use crate::screen::visible_width;
    use crate::style::strip_ansi;
    use yokohama_core::*;

    /// A real generated board at `n` players — the fixture shape the rest of
    /// the crate's render tests use.
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

    fn model_np(n: u8) -> (BoardModel, Vec<PlayerColor>) {
        let (state, rules) = state_np(n);
        (crate::screen_model::build_board(&state, &rules, &[]), colors_np(&state))
    }

    /// A tile with nothing on it: the opening state of any area.
    fn bare_tile() -> TileModel {
        TileModel {
            area: AreaId::new(0),
            area_type: AreaType::CopperMine,
            grid_position: (0, 0),
            token: Some(FivePowerEffect::ImportedGoods(1)),
            site: Some(crate::screen_model::SiteModel {
                index: Some(2),
                slots: [
                    slot(SpaceReward { vp: 1, ..Default::default() }, None),
                    slot(SpaceReward { seafood: 3, ..Default::default() }, None),
                    slot(SpaceReward::default(), None),
                    slot(SpaceReward { seafood: 2, ..Default::default() }, None),
                ],
                trading_house_vp: 7,
                trading_house_owner: None,
            }),
            presidents: Vec::new(),
            has_station: false,
            assistants: vec![(PlayerId::new(0), 0), (PlayerId::new(1), 0)],
            reachable: false,
            canal_crossings: Vec::new(),
        }
    }

    fn slot(
        reward: SpaceReward,
        built_by: Option<PlayerId>,
    ) -> crate::screen_model::SiteSlotModel {
        crate::screen_model::SiteSlotModel { built_by, reward }
    }

    const TWO: [PlayerColor; 2] = [PlayerColor::Red, PlayerColor::Blue];

    // ---- shape ----

    /// The ONE shape, at the state the mocks disagreed about: turn 1. Five
    /// lines, the presence line covering the empty case, and no
    /// `President: none` line anywhere in V3.
    #[test]
    fn the_turn_one_tile_is_the_v3_five_row_shape() {
        let lines = tile_lines(&bare_tile(), &TWO);
        assert_eq!(
            lines,
            vec![
                "┌ A00 COPPER MINE ───────────┐",
                "│ 5-Power: 1 Import Site 2   │",
                "│ no pawns                   │",
                "│ sites ★1 f3 — f2  │ T7     │",
                "└────────────────────────────┘",
            ],
        );
        assert!(!lines.iter().any(|l| l.contains("President")), "V3 has no President line: {lines:?}");
    }

    /// The mid-game shape is the SAME five rows with the marks turned on: the
    /// president on the title in its owner's color, the reachable mark beside
    /// it, pawn runs on the presence line, a built slot in its builder's
    /// color.
    #[test]
    fn the_mid_game_tile_is_the_same_shape_with_its_marks_lit() {
        let mut tile = bare_tile();
        tile.area = AreaId::new(1);
        tile.area_type = AreaType::Bank;
        tile.token = Some(FivePowerEffect::Tea(3));
        tile.presidents = vec![PlayerId::new(1)];
        tile.reachable = true;
        tile.assistants = vec![(PlayerId::new(0), 1), (PlayerId::new(1), 2)];
        if let Some(site) = tile.site.as_mut() {
            site.index = Some(3);
            site.slots[1] = slot(SpaceReward { vp: 4, ..Default::default() }, None);
            site.slots[3] = slot(SpaceReward { tea: 2, ..Default::default() }, Some(PlayerId::new(1)));
        }
        assert_eq!(
            tile_lines(&tile, &TWO),
            vec![
                "┌ A01 BANK ▲ ◈ ──────────────┐",
                "│ 5-Power: 3 Tea Site 3      │",
                "│ ●  ●●                      │",
                "│ sites ★1 ★4 — ⌂  │ T7      │",
                "└────────────────────────────┘",
            ],
        );
    }

    /// The Canal is transit only and its shape says so: no token line, no
    /// presence line, no site strip.
    #[test]
    fn the_canal_renders_its_transit_only_shape() {
        let (board, colors) = model_np(4);
        let canal = board
            .tiles
            .iter()
            .find(|t| t.area_type == AreaType::Canal)
            .expect("the 4P board has a Canal");
        let lines = tile_lines(canal, &colors);
        assert_exact_width(&lines, TILE_W);
        let text = lines.join("\n");
        assert!(text.contains("CANAL"), "{text}");
        assert!(text.contains("transit only"), "{text}");
        assert!(text.contains("to the bank / crossing"), "{text}");
        assert!(!text.contains("5-Power"), "the Canal never carried a token: {text}");
        assert!(!text.contains("sites"), "the Canal has no building site: {text}");
    }

    // ---- width ----

    /// Every line of every tile of every real board, at every player count.
    #[test]
    fn every_tile_line_is_exactly_tile_w_at_every_player_count() {
        for n in [2u8, 3, 4] {
            let (board, colors) = model_np(n);
            for tile in &board.tiles {
                assert_exact_width(&tile_lines(tile, &colors), TILE_W);
            }
        }
    }

    /// THE round-3.1 overflow, as its own named regression: the widest 5-Power
    /// face this game prints ("3 Seafood", nine characters) beside a two-digit
    /// site index. It fills the interior to its last column and must not spill
    /// past it.
    #[test]
    fn a_nine_character_token_with_a_two_digit_site_index_still_fits() {
        let mut tile = bare_tile();
        tile.token = Some(FivePowerEffect::Seafood(3));
        if let Some(site) = tile.site.as_mut() {
            site.index = Some(12);
        }
        let lines = tile_lines(&tile, &TWO);
        assert_exact_width(&lines, TILE_W);
        assert!(
            strip_ansi(&lines[1]).contains("3 Seafood Site 12"),
            "both facts must survive intact: {:?}",
            strip_ansi(&lines[1]),
        );
        assert!(!lines[1].contains(glyph::ELLIPSIS), "it FITS; nothing should clip: {:?}", lines[1]);
    }

    /// **No card in the printed registry loses a token off its site strip** —
    /// swept over all of `Rules::standard().building_sites` × every built-state
    /// (T2 review N1).
    ///
    /// The gap this closes is that [`exact`] clips SILENTLY and by design: a
    /// strip one column too wide comes back at exactly [`INNER`] columns with
    /// its tail replaced by `…`, so it passes every width pin in this section
    /// while having dropped a printed reward the player needs. Width tests
    /// cannot see that. The margin is not comfortable either — the widest card
    /// in the set renders at exactly 26 of 26 interior columns, so ONE more
    /// token anywhere (a data fix, a new registry card, a wider mark in
    /// [`push_reward`]) starts cutting rewards off the board with nothing
    /// failing.
    ///
    /// The built-state axis matters even though building only ever makes a
    /// strip NARROWER (a `⌂` replaces a reward that is at least as wide): it is
    /// the trading-house owner and the per-slot builders that colour the line,
    /// and a paint bug that added visible characters would show up here as a
    /// clip. Every combination is swept rather than argued — 24 cards × 16
    /// built-states × owned/unowned, which is cheap.
    ///
    /// Anti-vacuity is in the test, not in a comment: the same predicate is run
    /// against a fabricated card carrying every token this renderer can print,
    /// and it must SEE that one clip. A detector that cannot fail is not
    /// evidence that nothing is wrong.
    #[test]
    fn no_registry_building_site_clips_its_slot_strip() {
        let rules = Rules::standard();
        assert!(!rules.building_sites.is_empty(), "fixture: the registry must have cards in it");

        // The site strip is the tile's fourth line -- title, token, presence,
        // sites, bottom edge. Pinned against TILE_LINES so a shape change
        // cannot quietly point this sweep at a different row.
        const SITE_LINE: usize = 3;

        let strip_of = |site: crate::screen_model::SiteModel| -> String {
            let mut tile = bare_tile();
            tile.site = Some(site);
            let lines = tile_lines(&tile, &TWO);
            assert_eq!(lines.len(), TILE_LINES, "tile shape moved; SITE_LINE is stale");
            assert_exact_width(&lines, TILE_W);
            strip_ansi(&lines[SITE_LINE])
        };

        let mut widest = (0usize, String::new());
        for (index, card) in rules.building_sites.iter().enumerate() {
            for built in 0u8..16 {
                for owner in [None, Some(PlayerId::new(1))] {
                    let site = crate::screen_model::SiteModel {
                        index: Some(index as u8),
                        slots: std::array::from_fn(|i| {
                            let builder = (built >> i) & 1 == 1;
                            slot(card.spaces[i], builder.then(|| PlayerId::new(0)))
                        }),
                        trading_house_vp: card.trading_house_reward,
                        trading_house_owner: owner,
                    };
                    let line = strip_of(site);
                    assert!(
                        !line.contains(glyph::ELLIPSIS),
                        "card {index} (built mask {built:#06b}, owner {owner:?}) clips its site \
                         strip -- a printed reward is being dropped off the board: {line:?}",
                    );
                    let content = line.trim_matches(glyph::BOX_V).trim().chars().count();
                    if content > widest.0 {
                        widest = (content, line);
                    }
                }
            }
        }
        // Recorded, not asserted: how close the set actually runs to the
        // interior budget is the fact that says whether this sweep is a
        // formality or the only thing standing between the registry and a
        // silent cut. It is not a formality.
        assert!(
            widest.0 <= INNER,
            "the widest registry strip is {} columns against an interior of {INNER}: {:?}",
            widest.0,
            widest.1,
        );

        // ...and the detector fires on a card that really is too wide: every
        // token `push_reward` can emit, on one space.
        let loud = SpaceReward {
            vp: 1,
            yen: 1,
            seafood: 1,
            tea: 1,
            silk: 1,
            copper: 1,
            any_good: 1,
            movement: 1,
            warehouse_tokens: 1,
        };
        let over = strip_of(crate::screen_model::SiteModel {
            index: Some(0),
            slots: [
                slot(loud, None),
                slot(SpaceReward::default(), None),
                slot(SpaceReward::default(), None),
                slot(SpaceReward::default(), None),
            ],
            trading_house_vp: 7,
            trading_house_owner: None,
        });
        assert!(
            over.contains(glyph::ELLIPSIS),
            "the sweep's own clip detector must be able to fail: {over:?}",
        );
    }

    /// The other end of the same budget: the longest area name, every seat's
    /// president standing on it, the Station and the reachable mark. The marks
    /// are the live state, so the NAME is what clips.
    #[test]
    fn a_title_that_cannot_hold_everything_clips_the_name_and_keeps_the_marks() {
        let mut tile = bare_tile();
        tile.area_type = AreaType::EmploymentAgency;
        tile.presidents = (0..4).map(PlayerId::new).collect();
        tile.has_station = true;
        tile.reachable = true;
        let colors = [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green];
        let lines = tile_lines(&tile, &colors);
        assert_exact_width(&lines, TILE_W);
        let title = strip_ansi(&lines[0]);
        assert_eq!(title.matches(glyph::PRESIDENT).count(), 4, "no president may be dropped: {title:?}");
        assert!(title.contains(STATION), "the Station mark survives: {title:?}");
        assert!(title.contains(glyph::REACHABLE), "the reachable mark survives: {title:?}");
        assert!(title.contains(glyph::ELLIPSIS), "the name is what gives: {title:?}");
    }

    /// A tile crowded past what `●` runs can express falls back to counts
    /// rather than clipping a player's pawns off the board.
    #[test]
    fn a_crowded_tile_counts_pawns_rather_than_losing_them() {
        let mut tile = bare_tile();
        tile.assistants = (0..4).map(|i| (PlayerId::new(i), 20)).collect();
        let colors = [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green];
        let lines = tile_lines(&tile, &colors);
        assert_exact_width(&lines, TILE_W);
        let presence = strip_ansi(&lines[2]);
        assert_eq!(
            presence.trim_matches(glyph::BOX_V).trim(),
            "R20 B20 Y20 G20",
            "every seat's count must survive: {presence:?}",
        );
    }

    #[test]
    fn pawn_runs_cap_at_four_glyphs_and_count_the_remainder() {
        assert_eq!(pawn_run(0), "");
        assert_eq!(pawn_run(1), "●");
        assert_eq!(pawn_run(4), "●●●●");
        assert_eq!(pawn_run(5), "●●●●+1");
        assert_eq!(pawn_run(20), "●●●●+16");
    }

    // ---- content ----

    /// Each of the six printed faces, in the mocks' own wording, and each in
    /// its good's color rather than a single token tone.
    #[test]
    fn the_five_power_token_reads_as_the_mocks_print_it() {
        assert_eq!(five_power_token(FivePowerEffect::ImportedGoods(1)).0, "1 Import");
        assert_eq!(five_power_token(FivePowerEffect::Copper(2)).0, "2 Copper");
        assert_eq!(five_power_token(FivePowerEffect::Silk(2)).0, "2 Silk");
        assert_eq!(five_power_token(FivePowerEffect::Tea(3)).0, "3 Tea");
        assert_eq!(five_power_token(FivePowerEffect::Seafood(3)).0, "3 Seafood");
        assert_eq!(five_power_token(FivePowerEffect::Money(3)).0, "3 Yen");
        let kinds: Vec<GoodKind> = [
            FivePowerEffect::ImportedGoods(1),
            FivePowerEffect::Copper(2),
            FivePowerEffect::Silk(2),
            FivePowerEffect::Tea(3),
            FivePowerEffect::Seafood(3),
            FivePowerEffect::Money(3),
        ]
        .into_iter()
        .map(|e| five_power_token(e).1)
        .collect();
        for (i, a) in kinds.iter().enumerate() {
            for b in &kinds[i + 1..] {
                assert_ne!(a, b, "two faces share a color kind");
            }
        }
    }

    /// A claimed token says so rather than leaving a blank the player has to
    /// interpret.
    #[test]
    fn a_taken_five_power_token_reads_as_taken() {
        let mut tile = bare_tile();
        tile.token = None;
        let lines = tile_lines(&tile, &TWO);
        assert_exact_width(&lines, TILE_W);
        assert!(strip_ansi(&lines[1]).contains("5-Power: taken"), "{:?}", strip_ansi(&lines[1]));
    }

    /// Both halves of a two-part printed reward render; four of the
    /// twenty-four site cards print one.
    #[test]
    fn a_two_part_space_reward_prints_both_tokens() {
        let mut tile = bare_tile();
        if let Some(site) = tile.site.as_mut() {
            site.slots[1] = slot(SpaceReward { vp: 2, yen: 1, ..Default::default() }, None);
        }
        let lines = tile_lines(&tile, &TWO);
        assert_exact_width(&lines, TILE_W);
        assert!(strip_ansi(&lines[3]).contains("★2¥1"), "{:?}", strip_ansi(&lines[3]));
    }

    /// Every printed reward kind in the twenty-four-card set has a token, and
    /// a blank space is an em dash rather than nothing at all.
    #[test]
    fn every_printed_reward_kind_has_its_own_token() {
        let cases: [(SpaceReward, &str); 10] = [
            (SpaceReward { vp: 3, ..Default::default() }, "★3"),
            (SpaceReward { yen: 2, ..Default::default() }, "¥2"),
            (SpaceReward { seafood: 3, ..Default::default() }, "f3"),
            (SpaceReward { tea: 2, ..Default::default() }, "t2"),
            (SpaceReward { silk: 2, ..Default::default() }, "s2"),
            (SpaceReward { copper: 1, ..Default::default() }, "c1"),
            (SpaceReward { any_good: 2, ..Default::default() }, "*2"),
            (SpaceReward { movement: 2, ..Default::default() }, "a2"),
            (SpaceReward { warehouse_tokens: 2, ..Default::default() }, "⌂2"),
            (SpaceReward::default(), "—"),
        ];
        for (reward, expected) in cases {
            let mut t = Twin::default();
            push_reward(&mut t, &reward);
            assert_eq!(t.plain, expected, "{reward:?}");
        }
    }

    /// A built trading house takes its owner's color; an unbuilt one is plain
    /// text, exactly as the mock prints it.
    #[test]
    fn the_trading_house_vp_colors_only_once_it_is_built() {
        let mut tile = bare_tile();
        crate::style::force(true);
        let unbuilt = site_strip(&tile, &TWO).painted;
        if let Some(site) = tile.site.as_mut() {
            site.trading_house_owner = Some(PlayerId::new(1));
        }
        let built = site_strip(&tile, &TWO).painted;
        crate::style::force(false);
        assert!(unbuilt.ends_with("T7"), "an unbuilt trading house paints nothing: {unbuilt:?}");
        assert!(
            built.ends_with(&paint_forced(Tone::Player(PlayerColor::Blue), "T7")),
            "a built one takes its owner's color: {built:?}",
        );
    }

    fn paint_forced(tone: Tone, s: &str) -> String {
        crate::style::force(true);
        let out = paint(tone, s);
        crate::style::force(false);
        out
    }

    /// A model whose seats this renderer has no color for keeps every mark
    /// and loses only the tint — never the other way round.
    #[test]
    fn an_unknown_seat_keeps_its_mark_and_loses_its_color() {
        let mut tile = bare_tile();
        tile.presidents = vec![PlayerId::new(3)];
        tile.assistants = vec![(PlayerId::new(3), 2)];
        crate::style::force(true);
        let lines = tile_lines(&tile, &TWO);
        crate::style::force(false);
        assert_exact_width(&lines, TILE_W);
        assert!(strip_ansi(&lines[0]).contains(glyph::PRESIDENT), "{:?}", strip_ansi(&lines[0]));
        assert!(strip_ansi(&lines[2]).contains("●●"), "{:?}", strip_ansi(&lines[2]));
    }

    // ---- the board ----

    /// The staggered pyramid: rows from the model's own grid positions, each
    /// indented half a tile below the one above, every line one width.
    #[test]
    fn the_board_is_a_staggered_pyramid_of_v3_tiles() {
        for (n, rows) in [(2u8, vec![4, 3, 2, 1]), (3, vec![5, 4, 3, 2]), (4, vec![6, 5, 4, 3])] {
            let (board, colors) = model_np(n);
            let text = board_view(&board, &colors);
            let lines: Vec<&str> = text.lines().collect();
            assert_eq!(lines.len(), rows.len() * 5, "{n}P: five lines per row of tiles");

            let widths: std::collections::HashSet<usize> =
                lines.iter().map(|l| visible_width(l)).collect();
            assert_eq!(widths.len(), 1, "{n}P board lines must share one width");
            assert_eq!(
                *widths.iter().next().unwrap(),
                rows[0] * TILE_W,
                "{n}P board width is its widest row",
            );

            for (r, &len) in rows.iter().enumerate() {
                let top = lines[r * 5];
                assert_eq!(top.matches(glyph::BOX_TL).count(), len, "{n}P row {r} tile count");
                assert_eq!(
                    top.chars().take_while(|c| *c == ' ').count(),
                    r * (TILE_W / 2),
                    "{n}P row {r} brick offset",
                );
            }
        }
    }

    /// The board assumes no SHAPE: a fixture with a hole in a row keeps the
    /// column rather than closing up, so the geometry the model states is the
    /// geometry that draws.
    #[test]
    fn a_gap_in_a_row_holds_its_column() {
        let (mut board, colors) = model_np(2);
        let dropped = board
            .tiles
            .iter()
            .position(|t| t.grid_position == (0, 1))
            .expect("the 2P board has a (0,1) tile");
        board.tiles.remove(dropped);
        let text = board_view(&board, &colors);
        let top = text.lines().next().expect("a board line");
        assert_eq!(top.matches(glyph::BOX_TL).count(), 3, "three tiles left in the top row");
        assert_eq!(visible_width(top), 4 * TILE_W, "the row still spans four columns: {top:?}");
    }

    /// Painting must never move a visible character — the crate's alignment
    /// invariant, on the surface where fixed-width joins are densest.
    #[test]
    fn a_painted_board_strips_back_to_the_plain_one() {
        let (mut board, colors) = model_np(2);
        board.tiles[0].presidents = vec![PlayerId::new(0), PlayerId::new(1)];
        board.tiles[0].assistants = vec![(PlayerId::new(0), 3), (PlayerId::new(1), 1)];
        board.tiles[0].reachable = true;
        board.tiles[1].has_station = true;
        let plain = board_view(&board, &colors);
        crate::style::force(true);
        let colored = board_view(&board, &colors);
        crate::style::force(false);
        assert_ne!(colored, plain, "forced color must actually paint");
        assert_eq!(strip_ansi(&colored), plain);
    }

    /// The committed capture the orchestrator and the owner `cat` against the
    /// approved mock — held as a CHECKED golden rather than dumped once, so it
    /// cannot quietly stop being what the code renders.
    ///
    /// Board region only (the file name says so): that is what T2 ships, and it
    /// is the part that stays put while T3 and T4 rebuild the dashboard and the
    /// decision zone around it. Plain text, because a capture full of escapes
    /// is a capture nobody can read in a diff.
    ///
    /// Regenerate with `UPDATE_T2_CAPTURE=1 cargo test -p yokohama-cli --lib
    /// the_committed_capture` after a deliberate board change, and look at the
    /// diff before committing it.
    #[test]
    fn the_committed_capture_is_what_the_board_renders() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/plans/2026-08-05-phase3.5a-mocks/captures/t2-board-2p.txt",
        );
        let (state, rules) = state_np(2);
        let board = crate::screen_model::build_board(&state, &rules, &[]);
        let colors = colors_np(&state);
        let rendered =
            format!("{}{}\n", board_view(&board, &colors), board_legend(has_canal(&board)));

        if std::env::var_os("UPDATE_T2_CAPTURE").is_some() {
            std::fs::create_dir_all(std::path::Path::new(path).parent().expect("a parent"))
                .expect("the captures directory");
            std::fs::write(path, &rendered).expect("write the capture");
        }
        let committed = std::fs::read_to_string(path).expect("the committed capture");
        assert_eq!(
            committed, rendered,
            "the committed 2P board capture is stale; regenerate it with UPDATE_T2_CAPTURE=1",
        );
    }

    /// Two tiles and the legend, at the busiest state a tile can reach — the
    /// fixture guide §8.5 is written from.
    ///
    /// A president, a Station, two seats' assistant runs with one over the
    /// four-glyph cap, a built site slot, and the Canal beside them, because
    /// §8.5 has to describe every one of those and a capture of an untouched
    /// opening board shows none of them.
    fn guide_tile_capture() -> String {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(4), &rules, 7).expect("a board");
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0); 4]).expect("setup");
        state.players[0].assistants.on_board.insert(AreaId::new(3), 2);
        state.players[1].assistants.on_board.insert(AreaId::new(3), 6);
        state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(3));
        state.station_location = Some(AreaId::new(3));
        let site = state
            .board_layout
            .building_sites
            .get_mut(&AreaId::new(3))
            .expect("A03 carries a site");
        site.shophouses_built[1] = Some(PlayerId::new(1));
        site.trading_house_owner = Some(PlayerId::new(0));
        let board = crate::screen_model::build_board(&state, &rules, &[]);
        let colors = colors_np(&state);
        let mut out = String::new();
        for area in [AreaId::new(3), AreaId::new(10)] {
            let tile = board.tiles.iter().find(|t| t.area == area).expect("the tile");
            for line in tile_lines(tile, &colors) {
                out.push_str(&strip_ansi(&line));
                out.push('\n');
            }
        }
        out.push_str(&strip_ansi(&board_legend(true)));
        out.push('\n');
        out
    }

    /// **Guide §8.5's captured tile IS what the renderer draws** (T11 review
    /// F1), the regeneration discipline the guide's Movement, reveal, opening
    /// and `suggest` blocks already hold, applied to the fifth.
    ///
    /// That section described a DEAD surface: a table of abbreviated tile names
    /// (`Copper Mn`, `Tea Plant`) that this module's own tests assert must not
    /// appear, and an anatomy — `A03 Copper Mn c`, a `P:` line, `R2 B1`, a `~`
    /// water Canal — from before V3. Every word of it read plausibly. Nothing
    /// but a regeneration check catches that, which is the same lesson the T4
    /// review wrote against the Movement transcript.
    ///
    /// Regenerate by running this test and pasting the printed block into BOTH
    /// formats; it fails with the current rendering in the message.
    #[test]
    fn the_guide_tile_anatomy_is_what_the_renderer_draws() {
        const GUIDE_MD: &str = include_str!("../../../docs/cli-guide.md");
        let rendered = guide_tile_capture();
        let rendered = rendered.trim_end_matches('\n');
        let opening = rendered.lines().next().expect("the title line");
        let block = GUIDE_MD
            .split("```")
            .map(|part| part.trim_matches('\n'))
            .find(|part| part.starts_with(opening))
            .unwrap_or_else(|| panic!("cli-guide.md carries no captured tile block:\n{rendered}"));
        assert_eq!(
            block, rendered,
            "docs/cli-guide.md's §8.5 tile capture no longer matches what `guide_tile_capture` \
             renders. Re-capture it in BOTH formats from ONE run of this fixture — never by \
             editing the block by hand. The rendered block is above; `guide_sync` pins the .html \
             twin against the .md.",
        );
    }

    /// The legend takes the Canal fact rather than a board (N3), so the tests
    /// that want it FROM a board ask the model the same question `render.rs`
    /// asks `board_layout` -- one place, so a fixture and the shipped adapter
    /// cannot answer it differently.
    fn has_canal(board: &BoardModel) -> bool {
        board.tiles.iter().any(|t| t.area_type == AreaType::Canal)
    }

    /// The legend is the key to what the tiles draw: every mark and every
    /// site-slot token it can show.
    #[test]
    fn the_legend_names_every_mark_the_tiles_draw() {
        let (board, _) = model_np(2);
        let legend = board_legend(has_canal(&board));
        for needle in [
            &glyph::PRESIDENT.to_string()[..],
            &glyph::ASSISTANT.to_string(),
            &glyph::REACHABLE.to_string(),
            &glyph::BUILT.to_string(),
            &glyph::VP.to_string(),
            STATION,
            "site slots",
            "'sites'",
        ] {
            assert!(strip_ansi(&legend).contains(needle), "legend omits {needle:?}: {legend}");
        }
        assert!(!legend.contains("Canal"), "the 2P board has no Canal: {legend}");

        let (board4, _) = model_np(4);
        assert!(has_canal(&board4), "fixture: the 4P board is the one with a Canal on it");
        assert!(board_legend(true).contains("Canal"), "a board with a Canal states its rule");
    }
}
