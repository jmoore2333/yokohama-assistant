//! TTY-gated semantic color (Task 16, T2.5). This module owns ALL ANSI
//! knowledge: call sites say `paint(Tone::…, token)` and never embed raw
//! escapes. Color is OFF by default -- `paint` is the identity function
//! until `main.rs` calls `init_from_tty()` on a real terminal -- so every
//! piped run, save file, and existing output test stays byte-identical
//! without doing anything. `NO_COLOR` (https://no-color.org) is honored.
//!
//! Alignment rule for callers: escapes add bytes with zero display width,
//! so any fixed-width padding must be computed on the PLAIN text first and
//! `paint` applied to the finished token (see `render::fit_cell_painted`).

use std::sync::atomic::{AtomicBool, Ordering};

use yokohama_core::{AreaType, PlayerColor};

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
}

/// Semantic tones -- call sites name WHAT a token is, this module decides
/// how that looks. One tone per meaning keeps the palette disciplined:
/// players in their own color, goods in resource colors, VP bright,
/// warnings loud, prompts bold, de-emphasis dim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Player(PlayerColor),
    Good(GoodKind),
    /// A board tile's display name/art, colored by area type (Task 6
    /// redesign: production areas in their good's color, commercial areas
    /// in distinct hues, the Canal in water cyan).
    Tile(AreaType),
    Vp,
    Warn,
    Prompt,
    Dim,
}

/// SGR parameter string for a tone (the `N` of `ESC[Nm`). Players map to
/// their ANSI base color; goods to resource-evocative colors (copper 33,
/// silk 37, tea 32, seafood 36, yen bright-yellow 93); VP is bold bright
/// white, warnings bold red, prompts bold, dim faint.
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
        Tone::Vp => "1;97",
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
}
