//! TTY-gated semantic color (Task 16, T2.5). This module owns ALL ANSI
//! knowledge: call sites say `paint(Tone::…, token)` and never embed raw
//! escapes. Color is OFF by default -- `paint` is the identity function
//! until `main.rs` calls `init_from_tty()` on a real terminal -- so every
//! piped run, save file, and existing output test stays byte-identical
//! without doing anything. `NO_COLOR` (https://no-color.org) is honored.
//!
//! Since Phase-3.5a T1 it owns the other half of the presentation
//! vocabulary too: [`glyph`], the closed set of non-ASCII characters the
//! frames draw. Color and glyphs live together because they answer the same
//! call-site question -- "which token do I emit for this meaning?" -- and
//! because both are things an emitter must never spell inline.
//!
//! Alignment rule for callers: escapes add bytes with zero display width,
//! so any fixed-width padding must be computed on the PLAIN text first and
//! `paint` applied to the finished token (see `render::fit_cell_painted`).

use std::sync::atomic::{AtomicBool, Ordering};

use yokohama_core::{AreaType, Country, PlayerColor};

/// Process-wide switch. `false` at rest: a library user that never calls
/// `init_from_tty`/`force` gets plain text forever.
static ENABLED: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
thread_local! {
    /// Per-thread override so `cargo test`'s parallel threads can force
    /// color on/off hermetically: a forced window in one test must never
    /// bleed painted output into another test's plain-text assertions
    /// (all lib tests share one process). Test builds only -- the shipped
    /// binary reads the one process-wide switch above.
    static FORCED: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Enable color iff stdout is a real terminal and `NO_COLOR` is unset.
/// `main.rs` calls this once, first thing; nothing else ever should.
pub fn init_from_tty() {
    use std::io::IsTerminal;
    let on = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    ENABLED.store(on, Ordering::Relaxed);
}

/// Test hook: force color on/off regardless of TTY state. Under `cfg(test)`
/// this is thread-local (see `FORCED`); in normal builds it sets the
/// process-wide switch.
pub fn force(enabled: bool) {
    #[cfg(test)]
    FORCED.with(|f| f.set(Some(enabled)));
    #[cfg(not(test))]
    ENABLED.store(enabled, Ordering::Relaxed);
}

fn enabled() -> bool {
    #[cfg(test)]
    if let Some(forced) = FORCED.with(|f| f.get()) {
        return forced;
    }
    ENABLED.load(Ordering::Relaxed)
}

/// The frame's GLYPH VOCABULARY: every non-ASCII character the redesigned
/// surfaces draw, named once so no emitter ever spells one inline
/// (Phase-3.5a spec §0.4, plan T1). The set is CLOSED and small on purpose
/// -- it is exactly the twenty code points the approved mocks use
/// (`docs/plans/2026-08-05-phase3.5a-mocks/`), so "does this frame match its
/// mock?" stays a question about layout and color and never becomes a
/// question about which character somebody happened to type. ASCII the
/// mocks also use (`[1]` option brackets, `+n` overflow counts, the `*`
/// any-good and `T7` trading-house tokens) is not vocabulary; only the
/// characters a keyboard cannot produce are.
///
/// | constant | glyph | code point | EAW | role in the frame |
/// |---|---|---|---|---|
/// | [`PRESIDENT`](glyph::PRESIDENT) | `▲` | U+25B2 | Ambiguous | the president marker, on the tile title, in its owner's color |
/// | [`ASSISTANT`](glyph::ASSISTANT) | `●` | U+25CF | Ambiguous | one assistant pawn; runs cap at four, then `+n` |
/// | [`REACHABLE`](glyph::REACHABLE) | `◈` | U+25C8 | Ambiguous | a Movement destination the ENGINE's legal list offers (§0.7) |
/// | [`BUILT`](glyph::BUILT) | `⌂` | U+2302 | Neutral | a filled building-site slot, in its builder's color |
/// | [`VP`](glyph::VP) | `★` | U+2605 | Ambiguous | victory points, on cards, site slots and tracks |
/// | [`YEN`](glyph::YEN) | `¥` | U+00A5 | Narrow | money |
/// | [`ACTIVE`](glyph::ACTIVE) | `▶` | U+25B6 | Ambiguous | the seat whose turn it is, in the players panel |
/// | [`CHAIN_STEP`](glyph::CHAIN_STEP) | `▸` | U+25B8 | Neutral | the separator between phases in the phase chain |
/// | [`CATEGORY`](glyph::CATEGORY) | `■` | U+25A0 | Ambiguous | an index's group heading (order flags, goal categories) |
/// | [`HISTORY_KEY`](glyph::HISTORY_KEY) | `↑` | U+2191 | Ambiguous | names the recall key in the standing hint |
/// | [`SEPARATOR`](glyph::SEPARATOR) | `·` | U+00B7 | Ambiguous | between facts packed onto one line |
/// | [`EMPTY_SLOT`](glyph::EMPTY_SLOT) | `·` | U+00B7 | Ambiguous | an unfilled slot (same code point, different job -- see below) |
/// | [`SECTION_DASH`](glyph::SECTION_DASH) | `—` | U+2014 | Ambiguous | em dash: a title's trailing gloss, and a blank site space |
/// | [`RANGE_DASH`](glyph::RANGE_DASH) | `–` | U+2013 | Ambiguous | en dash: a reward RANGE (`★3–14`) |
/// | [`ELLIPSIS`](glyph::ELLIPSIS) | `…` | U+2026 | Ambiguous | an elided middle (the shortened seed in the title strip) |
/// | [`RULE`](glyph::RULE) | `─` | U+2500 | Ambiguous | a horizontal rule; also a box's top and bottom edge |
/// | [`BOX_V`](glyph::BOX_V) | `│` | U+2502 | Ambiguous | a box's side |
/// | [`BOX_TL`](glyph::BOX_TL) | `┌` | U+250C | Ambiguous | box corner, top-left |
/// | [`BOX_TR`](glyph::BOX_TR) | `┐` | U+2510 | Ambiguous | box corner, top-right |
/// | [`BOX_BL`](glyph::BOX_BL) | `└` | U+2514 | Ambiguous | box corner, bottom-left |
/// | [`BOX_BR`](glyph::BOX_BR) | `┘` | U+2518 | Ambiguous | box corner, bottom-right |
///
/// Twenty-one names over twenty code points: [`SEPARATOR`](glyph::SEPARATOR)
/// and [`EMPTY_SLOT`](glyph::EMPTY_SLOT) are both U+00B7 deliberately. They
/// are one character with two jobs, and a call site reads better saying
/// which job it means -- the mocks' `Shophouses ···· Trade ·` run is empty
/// slots, not separators. Composed runs like that one are repetition at the
/// call site, never a constant here.
///
/// **Width decision (the SF3 pin; completes the KNOWN RESIDUAL on
/// `screen::test_support::assert_exact_width`).** The vocabulary spans three
/// East-Asian-Width classes and NO Wide one: `¥` is Narrow, `⌂` and `▸` are
/// Neutral -- both classes are one column everywhere -- and the other
/// eighteen are Ambiguous, which means one column in a Western locale and
/// two under a CJK-configured locale or font. T0's residual note called `★`
/// East-Asian-WIDE; it is not. U+2605 is Ambiguous (UCD 16.0
/// `EastAsianWidth.txt`), so the residual is narrower than recorded -- not
/// absent, and the classes in the table above are the corrected record.
///
/// THE DECISION: this vocabulary is ACCEPTED for the raised-floor design,
/// because the project's target terminals render every glyph in it one
/// column wide -- verified on the owner's terminal across the mock rounds,
/// where the generator's own `vis(row) == TILE_W` guard and the eye agreed
/// at 200 columns. `screen::visible_width` counts CHARS, so for these
/// glyphs column alignment rests on that recorded assumption and not on the
/// guard: a width assert proves a line carries the intended number of
/// CHARACTERS, and this paragraph is the reason that equals the intended
/// number of COLUMNS. A terminal that renders Ambiguous glyphs wide would
/// shear every fixed-width join silently; the fix then is a width TABLE
/// inside `visible_width`, not a change here.
///
/// The [`VOCABULARY`](glyph::VOCABULARY) table below is the executable half:
/// it pins each constant's code point against the class recorded for it,
/// refuses a Wide entry outright, and keeps variation selectors and the
/// emoji planes out -- `★` U+2605 is a text symbol, while `⭐` U+2B50
/// (WHITE MEDIUM STAR) is Emoji_Presentation and would render two columns
/// in color. That substitution is the exact accident these pins exist to
/// catch.
pub mod glyph {
    /// The president marker, on the tile title in its owner's color (V3).
    pub const PRESIDENT: char = '▲';
    /// One assistant pawn.
    pub const ASSISTANT: char = '●';
    /// A Movement destination the engine's own legal list offers.
    pub const REACHABLE: char = '◈';
    /// A built building-site slot, in its builder's color.
    pub const BUILT: char = '⌂';
    /// Victory points.
    pub const VP: char = '★';
    /// Money.
    pub const YEN: char = '¥';
    /// The seat whose turn it is.
    pub const ACTIVE: char = '▶';
    /// The step separator in the phase chain.
    pub const CHAIN_STEP: char = '▸';
    /// An index's group heading.
    pub const CATEGORY: char = '■';
    /// The recall key, named in the standing hint.
    pub const HISTORY_KEY: char = '↑';
    /// Between facts packed onto one line.
    pub const SEPARATOR: char = '·';
    /// An unfilled slot. Same code point as [`SEPARATOR`], different job.
    pub const EMPTY_SLOT: char = '·';
    /// Em dash: a title's trailing gloss, a blank site space.
    pub const SECTION_DASH: char = '—';
    /// En dash: a reward RANGE.
    pub const RANGE_DASH: char = '–';
    /// An elided middle.
    pub const ELLIPSIS: char = '…';
    /// A horizontal rule; also a box's top and bottom edge.
    pub const RULE: char = '─';
    /// A box's side.
    pub const BOX_V: char = '│';
    /// Box corner, top-left.
    pub const BOX_TL: char = '┌';
    /// Box corner, top-right.
    pub const BOX_TR: char = '┐';
    /// Box corner, bottom-left.
    pub const BOX_BL: char = '└';
    /// Box corner, bottom-right.
    pub const BOX_BR: char = '┘';

    /// East-Asian-Width class, as recorded from the Unicode Character
    /// Database (`EastAsianWidth.txt`, UCD 16.0) when this table was
    /// authored. Nothing in the crate COMPUTES this -- there is no Unicode
    /// width table in tree and no dependency to add one -- so a row's class
    /// is a fact checked once, off-line, and then pinned by review. What the
    /// tests do enforce is that no [`Wide`](Eaw::Wide) glyph ever enters the
    /// vocabulary: an Ambiguous glyph is one column on the terminals this
    /// project targets (the decision above), while a Wide one is two columns
    /// EVERYWHERE and `screen::visible_width` could not model it at all.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Eaw {
        /// One column, and has a fullwidth twin elsewhere in Unicode.
        Narrow,
        /// One column outside East Asian context; no fullwidth twin.
        Neutral,
        /// One column in a Western locale, two under a CJK-configured one.
        Ambiguous,
        /// Two columns everywhere -- admissible in this type so the pin
        /// against it can be stated, never admissible in [`VOCABULARY`].
        Wide,
    }

    /// One vocabulary entry: the constant's name as the doc table spells it,
    /// the character itself, the code point the table records, and its
    /// width class.
    #[derive(Debug, Clone, Copy)]
    pub struct Glyph {
        pub name: &'static str,
        pub ch: char,
        pub codepoint: u32,
        pub eaw: Eaw,
    }

    const fn g(name: &'static str, ch: char, codepoint: u32, eaw: Eaw) -> Glyph {
        Glyph { name, ch, codepoint, eaw }
    }

    /// The whole vocabulary as data, in the doc table's order, so a pin can
    /// sweep it rather than naming glyphs one at a time -- a constant added
    /// above and forgotten here is what the length assertion catches.
    pub const VOCABULARY: &[Glyph] = &[
        g("PRESIDENT", PRESIDENT, 0x25B2, Eaw::Ambiguous),
        g("ASSISTANT", ASSISTANT, 0x25CF, Eaw::Ambiguous),
        g("REACHABLE", REACHABLE, 0x25C8, Eaw::Ambiguous),
        g("BUILT", BUILT, 0x2302, Eaw::Neutral),
        g("VP", VP, 0x2605, Eaw::Ambiguous),
        g("YEN", YEN, 0x00A5, Eaw::Narrow),
        g("ACTIVE", ACTIVE, 0x25B6, Eaw::Ambiguous),
        g("CHAIN_STEP", CHAIN_STEP, 0x25B8, Eaw::Neutral),
        g("CATEGORY", CATEGORY, 0x25A0, Eaw::Ambiguous),
        g("HISTORY_KEY", HISTORY_KEY, 0x2191, Eaw::Ambiguous),
        g("SEPARATOR", SEPARATOR, 0x00B7, Eaw::Ambiguous),
        g("EMPTY_SLOT", EMPTY_SLOT, 0x00B7, Eaw::Ambiguous),
        g("SECTION_DASH", SECTION_DASH, 0x2014, Eaw::Ambiguous),
        g("RANGE_DASH", RANGE_DASH, 0x2013, Eaw::Ambiguous),
        g("ELLIPSIS", ELLIPSIS, 0x2026, Eaw::Ambiguous),
        g("RULE", RULE, 0x2500, Eaw::Ambiguous),
        g("BOX_V", BOX_V, 0x2502, Eaw::Ambiguous),
        g("BOX_TL", BOX_TL, 0x250C, Eaw::Ambiguous),
        g("BOX_TR", BOX_TR, 0x2510, Eaw::Ambiguous),
        g("BOX_BL", BOX_BL, 0x2514, Eaw::Ambiguous),
        g("BOX_BR", BOX_BR, 0x2518, Eaw::Ambiguous),
    ];
}

/// Resource kinds that get their own display color, for `Tone::Good`: the
/// four tradeable goods, one variant each mirroring core's `TradeGood`
/// (`types/choices.rs`), plus `Yen` -- not a `TradeGood` in core, but still
/// a countable resource worth coloring in CLI output. Kept as its own
/// CLI-side enum rather than reusing `TradeGood` directly because of that
/// Yen extra, not because core lacks a unified goods enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoodKind {
    Copper,
    Silk,
    Tea,
    Seafood,
    Yen,
    /// Imported goods -- like `Yen`, not a core `TradeGood`, but a counted
    /// resource the goods run displays (Task 6 full-color treatment).
    Imports,
    /// "Any good" -- the wildcard a building site's `*` space and an
    /// any-good choice pay in. Not a resource anybody holds, but a token the
    /// site strip prints, and its own meaning: a `*` slot is not an Imports
    /// slot (3.5a T1, mock `main-frame-mid`'s site strip).
    Any,
}

impl GoodKind {
    /// Every kind, for palette sweeps and legends.
    pub const ALL: [GoodKind; 7] = [
        GoodKind::Copper,
        GoodKind::Silk,
        GoodKind::Tea,
        GoodKind::Seafood,
        GoodKind::Yen,
        GoodKind::Imports,
        GoodKind::Any,
    ];
}

/// Semantic tones -- call sites name WHAT a token is, this module decides
/// how that looks. One tone per meaning keeps the palette disciplined:
/// players in their own color, goods in resource colors, VP bright,
/// warnings loud, prompts bold, de-emphasis dim.
///
/// Two tones may share an SGR code without that being a fork -- `Player`
/// yellow and `Good(Copper)` are both `33` today -- because the discipline
/// is one tone per MEANING, not one tone per color. What the discipline
/// does forbid is a second tone for a meaning that already has one; the
/// 3.5a additions below are all meanings the pre-redesign palette had no
/// name for, and the redesign's owner-tinted marks (the `▲` president, the
/// `●` pawn runs, the `⌂` built slot) deliberately reuse `Player` rather
/// than adding mark-shaped tones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Player(PlayerColor),
    Good(GoodKind),
    /// A board tile's display name/art, colored by area type (Task 6
    /// redesign: production areas in their good's color, commercial areas
    /// in distinct hues, the Canal in water cyan).
    Tile(AreaType),
    /// An order card's nationality, in that flag's tint -- the card indexes
    /// group by flag, so the tint is how a group reads at a glance (3.5a
    /// §0.6, mock `order-index`).
    Flag(Country),
    Vp,
    /// A Movement destination the ENGINE's legal list offers -- the `◈`
    /// mark and the tile it tints (§0.7). Its own meaning, never a
    /// player's: the CLI computes no legality, it colors what it is told.
    Reachable,
    /// Reverse video: the one lit item in a run of alternatives -- the
    /// current phase in the chain, the neutral dummy's occupied space.
    Highlight,
    /// Emphasis inside a line: a card title, a player name, an option
    /// number. Bold, like `Prompt`, but not the same meaning -- `Prompt` is
    /// the input caret and lives at the bottom of a frame, while `Strong`
    /// is how any line raises one token above its neighbours.
    Strong,
    /// Structural chrome and small annotations: a section rule, an engine id
    /// tag, a parenthetical hint. Gray, distinct from `Dim`'s faint --
    /// `Dim` de-emphasizes CONTENT (an empty slot, a spent option), `Chrome`
    /// colors what was never content to begin with.
    Chrome,
    Warn,
    Prompt,
    Dim,
}

/// SGR parameter string for a tone (the `N` of `ESC[Nm`). Players map to
/// their ANSI base color; goods to resource-evocative colors (copper 33,
/// silk 37, tea 32, seafood 36, yen bright-yellow 93); VP is bold bright
/// white, warnings bold red, prompts bold, dim faint.
///
/// The 3.5a additions take their values from the approved mock palette
/// (`docs/plans/2026-08-05-phase3.5a-mocks/mockgen.py`): flags in national
/// tints, two of which are 256-color because no 16-color slot reads as
/// either (Dutch orange `38;5;208`, French blue `38;5;69`); reachable green
/// 32; highlight reverse-video 7; strong bold 1; chrome gray 90.
///
/// **PALETTE REFRESH -- RECORDED HERE, DELIBERATELY NOT APPLIED.** The same
/// mocks also re-color seven tones that already existed, and this table is
/// the record of what they become, not of what they are:
///
/// | tone | shipped | approved mock |
/// |---|---|---|
/// | `Good(Copper)` | `33` | `38;5;172` (ingot orange-brown) |
/// | `Good(Silk)` | `37` | `97` |
/// | `Good(Seafood)` | `36` | `96` |
/// | `Good(Imports)` | `95` | `93` |
/// | `Good(Yen)` | `93` | `38;5;220` (coin gold) |
/// | `Vp` | `1;97` | `38;5;75` (star blue) |
/// | `Player(_)` | `31/34/32/33` | `91/94/92/93` (bright) |
///
/// Applying a row RECOLORS a live surface: every one of those tones has
/// call sites in the pre-redesign frame, so a change here would repaint a
/// region before the task that renders it has landed, which is what T1's
/// "no existing output byte moves" forbids. The rows belong to the tasks
/// that cut their consumers over -- goods and VP with the board and site
/// strips (T2/T6), the player colors with the dashboard (T3) -- and each
/// should move its own row and delete it from this table.
///
/// One consequence is live in the meantime and worth knowing rather than
/// discovering: `Good(Any)` ships at `95`, which is `Good(Imports)`'s
/// current code, so the two paint identically until the Imports row moves
/// to `93` as the mocks have it. The MEANINGS are already distinct; only
/// the colors coincide, and only until then.
fn sgr(tone: Tone) -> &'static str {
    match tone {
        Tone::Player(PlayerColor::Red) => "31",
        Tone::Player(PlayerColor::Blue) => "34",
        Tone::Player(PlayerColor::Green) => "32",
        Tone::Player(PlayerColor::Yellow) => "33",
        Tone::Good(GoodKind::Copper) => "33",
        Tone::Good(GoodKind::Silk) => "37",
        Tone::Good(GoodKind::Tea) => "32",
        Tone::Good(GoodKind::Seafood) => "36",
        Tone::Good(GoodKind::Yen) => "93",
        Tone::Good(GoodKind::Imports) => "95",
        Tone::Good(GoodKind::Any) => "95",
        Tone::Tile(AreaType::CopperMine) => "33",
        Tone::Tile(AreaType::SilkMill) => "37",
        Tone::Tile(AreaType::TeaPlantation) => "32",
        Tone::Tile(AreaType::FishingGround) => "36",
        Tone::Tile(AreaType::Bank) => "93",
        Tone::Tile(AreaType::PortA | AreaType::PortB) => "94",
        Tone::Tile(AreaType::LaboratoryA | AreaType::LaboratoryB) => "95",
        Tone::Tile(AreaType::EmploymentAgency) => "90",
        Tone::Tile(AreaType::Chinatown) => "91",
        Tone::Tile(AreaType::Church) => "97",
        Tone::Tile(AreaType::Customs) => "34",
        Tone::Tile(AreaType::Canal) => "96",
        Tone::Flag(Country::English) => "95",
        Tone::Flag(Country::American) => "96",
        Tone::Flag(Country::French) => "38;5;69",
        Tone::Flag(Country::German) => "93",
        Tone::Flag(Country::Dutch) => "38;5;208",
        Tone::Vp => "1;97",
        Tone::Reachable => "32",
        Tone::Highlight => "7",
        Tone::Strong => "1",
        Tone::Chrome => "90",
        Tone::Warn => "1;31",
        Tone::Prompt => "1",
        Tone::Dim => "2",
    }
}

/// Wrap `s` in the tone's ANSI escape when color is enabled; return it
/// unchanged when disabled (the at-rest default).
pub fn paint(tone: Tone, s: &str) -> String {
    if enabled() {
        format!("\u{1b}[{}m{}\u{1b}[0m", sgr(tone), s)
    } else {
        s.to_string()
    }
}

/// Remove `ESC…` escape runs (SGR `ESC[…m` and the cursor controls the
/// frame composer emits), leaving the plain text. Promoted out of
/// test-support (Task 6): `screen::visible_width` computes display width
/// with it when joining the board and panel, so painted lines never shear
/// the seam. The equality `strip_ansi(colored) == plain` remains the
/// alignment invariant the render tests pin: painting must never add,
/// drop, or reorder a single visible character.
pub(crate) fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c2 in chars.by_ref() {
                if c2.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
pub(crate) mod test_support {
    /// Test alias for the crate-internal `strip_ansi` (kept so existing
    /// test imports read unchanged).
    pub(crate) use super::strip_ansi;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_is_identity_when_disabled_and_wraps_when_forced() {
        force(false);
        assert_eq!(paint(Tone::Vp, "5 VP"), "5 VP");
        force(true);
        let s = paint(Tone::Vp, "5 VP");
        assert!(s.starts_with("\u{1b}[") && s.ends_with("\u{1b}[0m") && s.contains("5 VP"), "{s:?}");
        force(false);
    }

    #[test]
    fn each_player_color_paints_distinctly() {
        force(true);
        let painted: Vec<String> =
            PlayerColor::ALL.iter().map(|&c| paint(Tone::Player(c), "X")).collect();
        force(false);
        for (i, a) in painted.iter().enumerate() {
            for b in &painted[i + 1..] {
                assert_ne!(a, b, "two player colors share an SGR code");
            }
        }
    }

    #[test]
    fn strip_ansi_inverts_paint() {
        force(true);
        let s = paint(Tone::Warn, "CRITICAL");
        force(false);
        assert_eq!(test_support::strip_ansi(&s), "CRITICAL");
    }

    /// The tones 3.5a adds, as a roster the paint pins can sweep. Not
    /// exhaustive over `Tone` by construction -- `sgr`'s match is what makes
    /// a new variant a compile error -- so a later task that adds a tone
    /// adds its row here too.
    fn tones_added_by_the_redesign() -> Vec<Tone> {
        let mut tones: Vec<Tone> = Country::ALL.iter().map(|&c| Tone::Flag(c)).collect();
        tones.extend([
            Tone::Reachable,
            Tone::Highlight,
            Tone::Strong,
            Tone::Chrome,
            Tone::Good(GoodKind::Any),
        ]);
        tones
    }

    /// The redesign's tones inherit the module's whole point: at rest they
    /// are the identity, byte for byte. That is the state a piped run and a
    /// `NO_COLOR` run both leave the switch in -- `init_from_tty`'s one
    /// `&&` is the only path `NO_COLOR` takes to `paint`, so off-state
    /// identity IS the NO_COLOR property for a tone -- and it is why adding
    /// palette entries cannot move a single byte of the line protocol.
    #[test]
    fn every_new_tone_is_the_identity_while_color_is_off() {
        force(false);
        for tone in tones_added_by_the_redesign() {
            assert_eq!(paint(tone, "token"), "token", "{tone:?} painted while color was off");
        }
    }

    /// ...and each one actually paints when color is on, wrapping the token
    /// without disturbing it. A tone whose SGR string were empty or
    /// malformed would still round-trip through `strip_ansi`, so the
    /// wrapper shape is checked too.
    #[test]
    fn every_new_tone_paints_and_restores_when_color_is_on() {
        force(true);
        let painted: Vec<(Tone, String)> =
            tones_added_by_the_redesign().into_iter().map(|t| (t, paint(t, "token"))).collect();
        force(false);
        for (tone, s) in painted {
            assert!(s.starts_with("\u{1b}["), "{tone:?} did not open an escape: {s:?}");
            assert!(s.ends_with("\u{1b}[0m"), "{tone:?} did not reset: {s:?}");
            assert_eq!(test_support::strip_ansi(&s), "token", "{tone:?} disturbed its token");
        }
    }

    /// The card indexes group orders by flag and lean on the tint to make a
    /// group readable at a glance, so two nations sharing a code would
    /// silently merge two groups into one.
    #[test]
    fn each_flag_tint_paints_distinctly() {
        force(true);
        let painted: Vec<String> =
            Country::ALL.iter().map(|&c| paint(Tone::Flag(c), "X")).collect();
        force(false);
        for (i, a) in painted.iter().enumerate() {
            for b in &painted[i + 1..] {
                assert_ne!(a, b, "two flag tints share an SGR code");
            }
        }
    }

    /// The vocabulary's doc table is the shipped RECORD of what each glyph
    /// is, and a record that drifts from its constants is worse than none:
    /// the width decision it carries would then describe characters the
    /// frames do not draw. Every row is checked against its constant.
    ///
    /// Two shapes are refused outright. A variation selector would make
    /// "one char" a lie the moment it were composed onto a base character,
    /// and an emoji-plane code point renders wide and in color on every
    /// modern terminal -- the `★` U+2605 -> `⭐` U+2B50 substitution being
    /// the plausible accident, since the two look alike in a font picker
    /// and only one of them is a text symbol.
    #[test]
    fn the_glyph_table_records_every_constant_truthfully() {
        let mut names = std::collections::BTreeSet::new();
        for gl in glyph::VOCABULARY {
            assert!(names.insert(gl.name), "{} appears twice in the vocabulary", gl.name);
            assert_eq!(
                gl.ch as u32, gl.codepoint,
                "{}: the table records U+{:04X} but the constant is U+{:04X}",
                gl.name, gl.codepoint, gl.ch as u32,
            );
            assert_ne!(
                gl.eaw,
                glyph::Eaw::Wide,
                "{} is recorded East-Asian-Wide: two columns everywhere, which \
                 screen::visible_width cannot model -- the vocabulary admits no such glyph",
                gl.name,
            );
            assert!(
                !matches!(gl.ch, '\u{FE00}'..='\u{FE0F}' | '\u{E0100}'..='\u{E01EF}'),
                "{} is a variation selector, not a glyph",
                gl.name,
            );
            assert!(
                gl.codepoint < 0x1F000,
                "{} sits in the emoji planes; the vocabulary is text symbols only",
                gl.name,
            );
        }
        assert_eq!(
            glyph::VOCABULARY.len(),
            21,
            "the vocabulary is twenty-one names over twenty code points \
             (SEPARATOR and EMPTY_SLOT share U+00B7) -- a constant added to the module \
             without a row here is a glyph no pin covers",
        );
    }

    /// Each glyph measures ONE column, plain and painted alike. The painted
    /// half is the crate's standing alignment rule applied to the new
    /// vocabulary (escapes carry zero display width); the plain half is the
    /// assumption the module doc's width decision records -- `visible_width`
    /// counts chars, and every fixed-width join in the redesign is built on
    /// each of these characters counting once.
    #[test]
    fn every_glyph_measures_one_column_plain_and_painted() {
        use crate::screen::visible_width;
        for gl in glyph::VOCABULARY {
            let plain = format!("[{}] ok", gl.ch);
            force(true);
            let painted = paint(Tone::Flag(Country::Dutch), &plain);
            let painted_16 = paint(Tone::Chrome, &plain);
            force(false);

            assert_eq!(visible_width(&gl.ch.to_string()), 1, "{} is not one column", gl.name);
            assert_eq!(
                visible_width(&plain),
                plain.chars().count(),
                "{}: visible width and char count disagree on the plain token",
                gl.name,
            );
            // A 256-color tone's escape is the long one (`38;5;208`); the
            // 16-color twin catches a stripper that only handles one shape.
            for token in [&painted, &painted_16] {
                assert_eq!(
                    visible_width(token),
                    visible_width(&plain),
                    "{}: painting changed the measured width",
                    gl.name,
                );
                assert_eq!(
                    test_support::strip_ansi(token),
                    plain,
                    "{}: painting disturbed the visible characters",
                    gl.name,
                );
            }
        }
    }
}
