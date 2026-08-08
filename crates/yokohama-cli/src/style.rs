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
//! `paint` applied to the finished token (see [`crate::board_render::exact`],
//! which is the helper that does it -- it measures a `Twin`'s plain half and
//! pads the painted one to match).

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
/// surfaces draw, named once so those surfaces never spell one inline
/// (Phase-3.5a spec §0.4, plan T1). Read that rule as binding on the
/// REDESIGNED surfaces, task by task as each cuts over: T1 ships the
/// vocabulary and no consumers, so today's frame still spells its own
/// characters, and the non-ASCII ones that are NOT vocabulary are registered
/// under DEFERRED CONVERSIONS below. The set is CLOSED and small on purpose
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
/// East-Asian-Width classes and NO Wide one, counted by NAME (the basis used
/// everywhere below): `¥` is Narrow, `⌂` and `▸` are Neutral -- both classes
/// are one column everywhere -- and the other eighteen of the twenty-one
/// names are Ambiguous, which means one column in a Western locale and
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
/// MEMBERSHIP is not tested, it is CONSTRUCTED: the constants and the
/// [`VOCABULARY`](glyph::VOCABULARY) rows are emitted from the same source
/// lines by the `vocabulary!` macro, so a glyph declared with no row covering
/// it is unrepresentable in the vocabulary's own declaration rather than
/// merely unnoticed. Read that macro's doc for the failure this closes, why
/// no test could have closed it, and the one deviation it leaves to review.
///
/// The table below is the executable half of everything else,
/// and it is worth being exact about which half. It pins each row's code
/// point against the constant it names, refuses a [`Wide`](glyph::Eaw::Wide)
/// entry outright, and keeps the astral planes out. It does NOT compute a
/// row's width class -- nothing in this crate can, which is what
/// [`Eaw`](glyph::Eaw)'s own doc says: Narrow/Neutral/Ambiguous is a fact
/// checked off-line against the UCD and then pinned by review. Nor is the
/// astral guard an emoji guard. `⭐` U+2B50 (WHITE MEDIUM STAR) is
/// Emoji_Presentation and would render two columns in color, but it sits in
/// the BMP and sails straight through; a `★` -> `⭐` substitution is caught
/// by the class column and the review that checks it, not by an assert.
///
/// **DEFERRED CONVERSIONS -- non-vocabulary glyphs the live frame still
/// draws.** Three non-ASCII characters are spelled inline by emitters today
/// and are deliberately NOT vocabulary: none of them appears in an approved
/// mock, and admitting a glyph no mock uses would widen the one set whose
/// whole value is being closed. Each is registered here against the task that
/// owns its region, so the call gets made once, with the surface, instead of
/// being forgotten (sites are as of T1).
///
/// **All three surviving rows are DEFERRED TO 3.5b, ruled at the six-lens
/// close (2026-08-07), and that is a decision rather than the rot it would
/// otherwise read as.** They were owed to T11, which closed without them; the
/// review re-checked what they cost and the answer is nothing measurable —
/// every one is East-Asian-Width Ambiguous, so a single column each, and no
/// layout moves whether they convert or not. What they DO cost is exactness:
/// `§` is the one code point the live frame emits that the closed vocabulary
/// does not contain (the census is emitted-minus-vocabulary = {U+00A7}), so
/// "the emitted set is the vocabulary" is true of nineteen of twenty glyphs
/// and stated here about the twentieth. Converting three prose characters in
/// the same wave that rewrote two footers and the wizard's numbering was the
/// scope call; the rows stay, with their owing task moved, so a reader meets a
/// judgement and not an oversight.
///
/// | glyph | code point | live site | owing task |
/// |---|---|---|---|
/// | `═` | U+2550 | ~~`screen.rs`, the title strip~~ | **DONE (T3)** |
/// | `§` | U+00A7 | `repl.rs`'s two guide cross-references (the board width note, the live-bot warning) | 3.5b |
/// | `−` | U+2212 | [`crate::suggest`]'s honesty note, the minus | 3.5b |
/// | `×` | U+00D7 | [`crate::suggest`]'s honesty note, the times | 3.5b |
///
/// The owing task either replaces the character or admits it here with a row
/// and a class. What it must not do is leave one spelled inline inside a
/// surface the redesign has already claimed. (The two `suggest` rows carried
/// line numbers — `:122`/`:123` — which had drifted by fifteen lines by the
/// six-lens close, so they name the module now: a citation that rots is worse
/// than one that is coarse.)
///
/// **The `═` row is discharged, and by a third answer neither option
/// anticipated: the character is GONE rather than converted.** It was re-routed
/// from T2 to T3 because the title strip carries the session facts a
/// [`crate::screen_model::TitleModel`] holds and so lands with the surface that
/// plumbs that model. T3 then found that the approved mocks' strip is centred
/// text on an otherwise empty line — it draws no rule at all — so
/// [`crate::panel_render::title_strip`] draws none, and U+2550 is no longer
/// spelled anywhere in the crate. The row is kept, struck through, because
/// "which task discharged this and how" is the question a reader arrives with.
pub mod glyph {
    /// Declares the vocabulary ONCE. One source line per glyph carries its
    /// doc comment, its constant name, its character, the code point recorded
    /// for it and its width class, and the macro emits BOTH the `pub const`
    /// and the matching [`VOCABULARY`] row from that single line.
    ///
    /// This is a closure, not tidiness. The constants and the table used to be
    /// two hand-written lists with a length assertion between them, and a
    /// length assertion cannot tell a missing row from a missing constant: add
    /// a constant, skip its row, and the count still matches whatever it
    /// matched before -- every test green, the glyph covered by no pin, and
    /// the module doc claiming otherwise. No test could have closed that hole,
    /// because a test can only check the rows it is handed. The fix has to be
    /// structural, so it is: inside the vocabulary's declaration -- the list
    /// below, the one place a glyph is declared -- a constant and its row are
    /// the SAME LINE and cannot diverge. Forgetting a row is not an omission
    /// you can make any more; it does not parse, and the crate does not build.
    /// The macro is the test that could not be written.
    ///
    /// What that does NOT close, said plainly rather than left implied:
    /// nothing in Rust forbids writing a bare `pub const FOO: char = '…';`
    /// into this module beside the list, and such a constant would carry no
    /// row and break no test. But that is a different act from forgetting a
    /// row -- a declaration made OUTSIDE the vocabulary rather than an
    /// omission inside it -- and it reads on sight as the one line that does
    /// not look like its twenty-one neighbours. The two-list divergence is
    /// structurally gone; the deviation is left to review, which is where the
    /// width classes below already live.
    ///
    /// (The types it expands into -- [`Glyph`], [`Eaw`] and the `g`
    /// constructor -- are defined below, after the table they describe.)
    macro_rules! vocabulary {
        ($(
            $(#[$row_doc:meta])*
            $name:ident = $ch:literal, $codepoint:literal, $eaw:ident;
        )*) => {
            $(
                $(#[$row_doc])*
                pub const $name: char = $ch;
            )*

            /// The whole vocabulary as data, in the doc table's order, so a
            /// pin can sweep it rather than naming glyphs one at a time.
            /// Generated from the same source lines as the constants above,
            /// so the two cannot disagree about WHICH glyphs exist; what the
            /// pins still buy is whether each row tells the truth about the
            /// one it names.
            pub const VOCABULARY: &[Glyph] = &[
                $( g(stringify!($name), $name, $codepoint, Eaw::$eaw), )*
            ];
        };
    }

    vocabulary! {
        /// The president marker, on the tile title in its owner's color (V3).
        PRESIDENT = '▲', 0x25B2, Ambiguous;
        /// One assistant pawn.
        ASSISTANT = '●', 0x25CF, Ambiguous;
        /// A Movement destination the engine's own legal list offers.
        REACHABLE = '◈', 0x25C8, Ambiguous;
        /// A built building-site slot, in its builder's color.
        BUILT = '⌂', 0x2302, Neutral;
        /// Victory points.
        VP = '★', 0x2605, Ambiguous;
        /// Money.
        YEN = '¥', 0x00A5, Narrow;
        /// The seat whose turn it is.
        ACTIVE = '▶', 0x25B6, Ambiguous;
        /// The step separator in the phase chain.
        CHAIN_STEP = '▸', 0x25B8, Neutral;
        /// An index's group heading.
        CATEGORY = '■', 0x25A0, Ambiguous;
        /// The recall key, named in the standing hint.
        HISTORY_KEY = '↑', 0x2191, Ambiguous;
        /// Between facts packed onto one line.
        SEPARATOR = '·', 0x00B7, Ambiguous;
        /// An unfilled slot. Same code point as [`SEPARATOR`], different job.
        EMPTY_SLOT = '·', 0x00B7, Ambiguous;
        /// Em dash: a title's trailing gloss, a blank site space.
        SECTION_DASH = '—', 0x2014, Ambiguous;
        /// En dash: a reward RANGE.
        RANGE_DASH = '–', 0x2013, Ambiguous;
        /// An elided middle.
        ELLIPSIS = '…', 0x2026, Ambiguous;
        /// A horizontal rule; also a box's top and bottom edge.
        RULE = '─', 0x2500, Ambiguous;
        /// A box's side.
        BOX_V = '│', 0x2502, Ambiguous;
        /// Box corner, top-left.
        BOX_TL = '┌', 0x250C, Ambiguous;
        /// Box corner, top-right.
        BOX_TR = '┐', 0x2510, Ambiguous;
        /// Box corner, bottom-left.
        BOX_BL = '└', 0x2514, Ambiguous;
        /// Box corner, bottom-right.
        BOX_BR = '┘', 0x2518, Ambiguous;
    }

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
/// yellow and `Flag(German)` are both `93` today -- because the discipline
/// is one tone per MEANING, not one tone per color. The share is allowed
/// where the two never appear in one block: a flag tints an order card's
/// nationality inside the card's border and is never a seat header, so the
/// yellow it shares with a seat cannot be mistaken for one. Where two tones
/// DO co-occur the share is a defect, which is what
/// `no_player_color_shares_a_code_with_a_good` pins for the `Player`/`Good`
/// pair (see [`sgr`]'s SF-Yellow note). What the discipline
/// does forbid outright is a second tone for a meaning that already has one; the
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
    ///
    /// Ships at `32`, the approved mock's value. That used to collide with
    /// `Player(Green)` -- invisible at 2P, where there is no green seat, and a
    /// real overlap at 3P/4P. **T3's palette refresh settled it**: the player
    /// colors moved to their bright twins and green is `92`, so a green seat's
    /// marks and a Movement destination no longer paint the same. The Tea
    /// Plantation tile is still `32`; that one is harmless while `Tile(_)` has
    /// no call sites at all (see [`sgr`]'s UNCONSUMED note).
    Reachable,
    /// A printed building-site space that pays an ACTION rather than goods or
    /// points: the assistants icon (`a`) and the shophouse-build icon (`⌂`),
    /// the two tokens on a site strip with no resource of their own to take a
    /// color from. White, which is the approved mocks' own value for both
    /// entries of their site-slot token table (`mockgen.py`'s `TOKC`).
    ///
    /// Its own meaning rather than [`Tone::Strong`], which is what it shipped
    /// as at T2 and what the T2 review sent back (N5): `Strong` is EMPHASIS,
    /// the thing a line does to raise one token above its neighbours, and on a
    /// site strip these two are not emphasised at all — they sit beside `★1`
    /// and `t2` as equals, and bolding them said the opposite.
    ActionIcon,
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

/// Every [`AreaType`], for the [`Tone::Tile`] half of [`Tone::all`]. Core
/// exposes no `AreaType::ALL` the way it does for `PlayerColor` and `Country`,
/// and this branch is CLI-only, so the roster is spelled here -- but not left
/// to good intentions: [`sgr`] and [`every_tone_is_accounted_for`] both name
/// every variant with no wildcard, so a new area type in core is a compile
/// error in this file before it can be a hole in this list.
const AREA_TYPES: [AreaType; 14] = [
    AreaType::CopperMine,
    AreaType::SilkMill,
    AreaType::TeaPlantation,
    AreaType::FishingGround,
    AreaType::Bank,
    AreaType::PortA,
    AreaType::PortB,
    AreaType::LaboratoryA,
    AreaType::LaboratoryB,
    AreaType::EmploymentAgency,
    AreaType::Chinatown,
    AreaType::Church,
    AreaType::Customs,
    AreaType::Canal,
];

impl Tone {
    /// Every tone there is: the parameterized families expanded from their
    /// own rosters, then the unit variants.
    ///
    /// FAIL-CLOSED, and the mechanism is the whole point. A hand-kept list of
    /// "the tones worth sweeping" fails OPEN -- the tone somebody forgets is
    /// exactly the one no pin covers, and the sweep still passes, reporting
    /// health it never measured. So this roster is composed from the component
    /// `::ALL`s rather than typed out, and [`every_tone_is_accounted_for`]
    /// carries a wildcard-free `match` naming every `Tone` variant and every
    /// variant of every family inside them. Adding a tone -- or a
    /// `PlayerColor`, `GoodKind`, `AreaType` or `Country` -- is a COMPILE
    /// error there, and the arm you are forced to write is the reminder that
    /// its family belongs here.
    pub fn all() -> Vec<Tone> {
        // The hook that keeps the checker alive: an uncalled private fn is
        // dead code. Its match is exhaustive by PATTERN, so one call proves
        // exactly as much as sweeping the whole roster would -- the guarantee
        // belongs to the compiler, not to this line.
        every_tone_is_accounted_for(Tone::Vp);

        let mut tones: Vec<Tone> = PlayerColor::ALL.map(Tone::Player).into();
        tones.extend(GoodKind::ALL.map(Tone::Good));
        tones.extend(AREA_TYPES.map(Tone::Tile));
        tones.extend(Country::ALL.map(Tone::Flag));
        tones.extend([
            Tone::Vp,
            Tone::Reachable,
            Tone::ActionIcon,
            Tone::Highlight,
            Tone::Strong,
            Tone::Chrome,
            Tone::Warn,
            Tone::Prompt,
            Tone::Dim,
        ]);
        tones
    }
}

/// The closure mechanism behind [`Tone::all`] -- see its doc for why. No
/// wildcard, every variant named, every arm a no-op: this function exists to
/// be COMPILED, not to be run.
fn every_tone_is_accounted_for(tone: Tone) {
    match tone {
        Tone::Player(
            PlayerColor::Red | PlayerColor::Blue | PlayerColor::Green | PlayerColor::Yellow,
        ) => (),
        Tone::Good(
            GoodKind::Copper
            | GoodKind::Silk
            | GoodKind::Tea
            | GoodKind::Seafood
            | GoodKind::Yen
            | GoodKind::Imports
            | GoodKind::Any,
        ) => (),
        Tone::Tile(
            AreaType::CopperMine
            | AreaType::SilkMill
            | AreaType::TeaPlantation
            | AreaType::FishingGround
            | AreaType::Bank
            | AreaType::PortA
            | AreaType::PortB
            | AreaType::LaboratoryA
            | AreaType::LaboratoryB
            | AreaType::EmploymentAgency
            | AreaType::Chinatown
            | AreaType::Church
            | AreaType::Customs
            | AreaType::Canal,
        ) => (),
        Tone::Flag(
            Country::English
            | Country::American
            | Country::French
            | Country::German
            | Country::Dutch,
        ) => (),
        Tone::Vp => (),
        Tone::Reachable => (),
        Tone::ActionIcon => (),
        Tone::Highlight => (),
        Tone::Strong => (),
        Tone::Chrome => (),
        Tone::Warn => (),
        Tone::Prompt => (),
        Tone::Dim => (),
    }
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
/// **PALETTE REFRESH -- COMPLETE.** The mocks also re-colored tones that
/// already existed, and this is the record of that now-empty table.
///
/// Applying a row RECOLORS a live surface, so each belonged to the task that
/// cut its consumers over. **T2 took the six it owed** -- `Good(Copper)` 33 ->
/// `38;5;172`, `Good(Silk)` 37 -> `97`, `Good(Seafood)` 36 -> `96`,
/// `Good(Imports)` 95 -> `93`, `Good(Yen)` 93 -> `38;5;220`, `Vp` 1;97 ->
/// `38;5;75` -- because the V3 board's 5-Power tokens and site-slot strips
/// are their densest consumer, and a board painted from one palette beside a
/// panel painted from another is worse than either.
///
/// **T3 took the last one: `Player(_)` 31/34/32/33 -> `91/94/92/93`,** the
/// mocks' bright twins, with the dashboard's owner-tinted names, pawn runs and
/// track markers as the consumer that made it due. It settles the collision
/// `Tone::Reachable` has carried since T1: that tone ships at `32`, which WAS
/// `Player(Green)` and the Tea Plantation tile, so a green seat's marks and the
/// `◈` a Movement destination is tinted with painted identically at 3 and 4
/// players. Green is now `92` and the two read apart.
///
/// **Two tones still sit at `32` beside `Reachable`, and only one of them is
/// dead** — the T3 review's correction to what this paragraph used to say.
/// `Tile(TeaPlantation)` is the harmless one, for the reason `Tile(_)` is noted
/// as UNCONSUMED below: nothing draws it. `Good(Tea)` is `32` as well and is
/// very much LIVE — it counts tea in the goods strips — so `32` is a share on a
/// board that draws both. It is ACCEPTED as a CONTEXTUAL share rather than
/// fixed: `Reachable` paints a `◈` mark beside a tile's title, `Good(Tea)`
/// paints counts inside a strip, and the two never stand where one could be
/// read as the other. That is the same disposition as `Flag(German)` sharing
/// `93` with `Player(Yellow)`, and the opposite of the SF-Yellow case below,
/// where the two families DID land in one block.
///
/// The T2 refresh also closed a collision this table used to warn about:
/// `Good(Any)` and `Good(Imports)` both shipped at `95` and painted
/// identically, which the site strip would have shown on a single tile (a
/// `*n` any-good space beside a `1 Import` token). Imports moved off `95` and
/// the two now read apart, which is what `Good(Any)` was added for.
///
/// **SF-YELLOW (T3 review): `Good(Imports)` `93` -> `38;5;178`, and this is a
/// DELIBERATE DEPARTURE FROM THE MOCKS.** T2 moved Imports to `93` to clear the
/// `95` collision above; T3 then moved the player colors to their bright twins,
/// which put `Player(Yellow)` on `93` too — and unlike the shares above, these
/// two co-occur. A dashboard player block is a `Player(_)` seat header with
/// that seat's goods counted underneath, so on any board carrying a yellow seat
/// — the default at 3 and 4 players, and a 2-player game whose wizard assigned
/// it — that seat's name and its imports count painted the same, and the tint
/// that exists to say whose block this is stopped saying it. Player identity
/// wins: Imports takes
/// `38;5;178`, a near-yellow gold that reads as the mock's `93` at a glance
/// while being a distinct code.
///
/// The departure is recorded rather than silent because the MOCK ITSELF carried
/// this collision, so there is no approved value to defer to: `mockgen.py`
/// binds `YELLOW = c(93)`, hands it to the imports good (`GOOD["imports"]`) and
/// hands the same `93` to the YELLOW seat column, then draws a frame in which
/// both appear. Departing from an approved mock is a call this crate records
/// rather than makes quietly — the same posture as the SP1-precedent
/// departures — and what is departed from here is a value the mock could not
/// have meant, since it makes its own frame ambiguous.
/// `no_player_color_shares_a_code_with_a_good` is the pin, and it sweeps the
/// whole `Good` roster rather than the pair that collided.
///
/// **UNCONSUMED: the whole [`Tone::Tile`] family** (T2 review S1). It has
/// fourteen entries below and, since T2, **zero call sites** -- V3 draws a
/// tile's name in [`Tone::Strong`] and carries the area's identity in the tile
/// BOX and its contents instead of in the name's hue, so nothing asks for an
/// area-type tint any more. The family is deliberately kept rather than
/// deleted in this fix wave: it costs nothing at runtime, [`Tone::all`] still
/// sweeps it (so all fourteen stay identity-pinned and paint-pinned like every
/// other tone), and T4's decision zone and T5's card index are both surfaces
/// that may want an area tint back. **Removing it or re-using it is a T11/T12
/// review call, not this wave's** -- what this note buys is that whoever gets
/// there finds "unconsumed on purpose since T2" written down instead of
/// inferring it from a grep that returns nothing.
fn sgr(tone: Tone) -> &'static str {
    match tone {
        Tone::Player(PlayerColor::Red) => "91",
        Tone::Player(PlayerColor::Blue) => "94",
        Tone::Player(PlayerColor::Green) => "92",
        Tone::Player(PlayerColor::Yellow) => "93",
        Tone::Good(GoodKind::Copper) => "38;5;172",
        Tone::Good(GoodKind::Silk) => "97",
        Tone::Good(GoodKind::Tea) => "32",
        Tone::Good(GoodKind::Seafood) => "96",
        Tone::Good(GoodKind::Yen) => "38;5;220",
        Tone::Good(GoodKind::Imports) => "38;5;178",
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
        Tone::Vp => "38;5;75",
        Tone::Reachable => "32",
        Tone::ActionIcon => "97",
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

    /// **One dash, everywhere the player reads prose** — T11's editorial rule,
    /// as an assertion any surface can point at itself.
    ///
    /// It lives here because `style` owns the crate's typography, and it is a
    /// FUNCTION because the rule's reach is the thing that keeps failing: the
    /// T11 sweep walked the REPL, which never visits a locked session, never
    /// loads a doctored save and never refuses `suggest` in live mode — so
    /// three whole message surfaces were outside every check, and the review
    /// proved it by injecting a `--` into the lockdown refusal and watching the
    /// suite stay green. Each of those surfaces now calls this where its own
    /// fixture already has the string, which is cheaper than teaching one walk
    /// to reach every corner of the driver.
    ///
    /// `--- Board check ---` and `-- row 1 of 4 --` are DECORATION, a
    /// horizontal rule rather than a dash, and are deliberately not matched:
    /// the pattern is a space-flanked `--`, which no rule of that shape writes.
    pub(crate) fn assert_no_ascii_prose_dash(what: &str, text: &str) {
        assert!(!text.is_empty(), "{what}: nothing to check — the fixture produced no text");
        for (n, line) in text.lines().enumerate() {
            assert!(
                !line.contains(" -- "),
                "{what} line {n} writes an ASCII prose dash where the surface speaks em dashes: \
                 {line:?}",
            );
        }
    }
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

    /// **The two families that co-occur inside one player block must stay
    /// disjoint** — the T3 review's SF-Yellow finding, made a pin.
    ///
    /// A dashboard player block is a seat header painted `Player(_)` with that
    /// seat's goods counted beneath it in `Good(_)`, and the whole point of the
    /// owner tint is that it says WHOSE the block is. `Good(Imports)` shipped at
    /// `93`, which is `Player(Yellow)`: on any board carrying a yellow seat (the
    /// default at 3 and 4 players) that seat's name and its imports count
    /// painted identically, so the one token that was supposed to identify the
    /// seat read as just another resource. Imports moved to `38;5;178` and this
    /// test is what keeps it moved.
    ///
    /// Sweeping the whole [`GoodKind::ALL`] roster rather than the one pair that
    /// collided: the fix would be worth little if the next good added took `91`.
    ///
    /// **A share OUTSIDE these two families is not what this pin forbids** —
    /// [`sgr`]'s doc states the discipline as one tone per MEANING, and two tones
    /// may legitimately paint alike where no surface shows them together.
    /// `Flag(German)` is `93`, the same as `Player(Yellow)`, and that one is
    /// ACCEPTED: flag tints are drawn on an order card's nationality, inside the
    /// card's border, and never as a seat header — the two never sit in one
    /// block the way a header and a goods count do. Same disposition, same
    /// reasoning, as the `Good(Tea)`/`Reachable` share [`sgr`] records.
    #[test]
    fn no_player_color_shares_a_code_with_a_good() {
        for &c in PlayerColor::ALL.iter() {
            for &g in GoodKind::ALL.iter() {
                assert_ne!(
                    sgr(Tone::Player(c)),
                    sgr(Tone::Good(g)),
                    "Player({c:?}) and Good({g:?}) both paint {} — a seat header and that \
                     seat's own goods counts sit in one dashboard block, so the owner tint \
                     stops identifying the owner",
                    sgr(Tone::Player(c)),
                );
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

    /// EVERY tone is the identity at rest, byte for byte -- the module's
    /// whole point, and the state a piped run and a `NO_COLOR` run both leave
    /// the switch in (`init_from_tty`'s one `&&` is the only path `NO_COLOR`
    /// takes to `paint`, so off-state identity IS the NO_COLOR property for a
    /// tone). It is why adding palette entries cannot move a single byte of
    /// the line protocol.
    ///
    /// Swept over [`Tone::all`], not over the tones one task happened to add:
    /// a roster assembled by hand is a roster that quietly stops being
    /// complete, and this pin would then pass by measuring less. See
    /// `Tone::all`'s doc for how a new variant is forced into it.
    #[test]
    fn every_tone_is_the_identity_while_color_is_off() {
        force(false);
        for tone in Tone::all() {
            assert_eq!(paint(tone, "token"), "token", "{tone:?} painted while color was off");
        }
    }

    /// ...and every one of them actually paints when color is on, wrapping
    /// the token without disturbing it. A tone whose SGR string were empty or
    /// malformed would still round-trip through `strip_ansi`, so the wrapper
    /// shape is checked too.
    #[test]
    fn every_tone_paints_and_restores_when_color_is_on() {
        force(true);
        let painted: Vec<(Tone, String)> =
            Tone::all().into_iter().map(|t| (t, paint(t, "token"))).collect();
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

    /// Each row's recorded facts, checked against the character it names.
    /// WHICH glyphs exist can no longer drift -- constants and rows come off
    /// the same source lines, see the `vocabulary!` macro -- so what is left
    /// to check is whether a row tells the truth about its own glyph, and the
    /// doc table's width decision rests on exactly that.
    ///
    /// Being precise about what each pin buys, because three of them are
    /// narrower than they look:
    ///
    /// - the code point IS checked against the constant, so no row can record
    ///   U+2605 beside a character that is not `★`;
    /// - `Wide` is refused outright -- two columns everywhere is the one class
    ///   `screen::visible_width` could not model at all;
    /// - a row's Narrow/Neutral/Ambiguous class is NOT computed. Nothing in
    ///   the crate can compute it (no Unicode width table in tree, no
    ///   dependency to add one), so the class is a fact checked off-line
    ///   against the UCD and pinned by review, exactly as [`glyph::Eaw`]'s own
    ///   doc states. A row mis-recording `★` as Narrow passes here;
    /// - the astral guard is a PLANE check and no more. It keeps U+1F000-and-up
    ///   out and says nothing about a BMP emoji: `⭐` U+2B50 renders wide and
    ///   in color yet would sail through, so the `★` -> `⭐` substitution is
    ///   caught by the class column and its review, not by this assert;
    /// - a variation selector cannot be COMPOSED onto a glyph here at all,
    ///   because `char` is one scalar value. That is the type's guarantee, not
    ///   a test's. The assert covers only the residue -- a lone selector
    ///   declared as though it were a glyph.
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
                "{} sits in the astral planes; the vocabulary is BMP text symbols only",
                gl.name,
            );
        }

        // The one sanctioned duplicate, stated as a PROPERTY rather than as a
        // count. The retired length literal could only say "twenty-one names
        // over twenty code points" and could not say which twenty; this names
        // the pair, so a second glyph that silently repeats an existing
        // character -- two names for one mark, where one name was wanted --
        // fails instead of quietly restoring the arithmetic.
        let mut by_codepoint: std::collections::BTreeMap<u32, Vec<&str>> = Default::default();
        for gl in glyph::VOCABULARY {
            by_codepoint.entry(gl.codepoint).or_default().push(gl.name);
        }
        let shared: Vec<(u32, Vec<&str>)> =
            by_codepoint.into_iter().filter(|(_, names)| names.len() > 1).collect();
        assert_eq!(
            shared,
            vec![(0x00B7, vec!["SEPARATOR", "EMPTY_SLOT"])],
            "U+00B7 carried by SEPARATOR and EMPTY_SLOT is the vocabulary's only shared \
             code point -- one character with two jobs. Any other sharing is two names \
             for one mark, and wants one name",
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
