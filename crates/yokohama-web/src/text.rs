//! The character rule for a person's text (security S2c, finding D8; decision D-4): a name, a display name, a
//! watcher's name and a chat line carry NO control (Cc), format (Cf) or line-separator (Zl, Zp) character. `Cc` is
//! `char::is_control`; `Cf` is the table below; `Zl` and `Zp` are two code points. Nothing else is judged here —
//! confusable letters are provenance's problem, which the tag answers (`chat::tag_of`), not this rule's. **What the
//! rule does not reach**, measured at the plan review (L1-F4): the default-ignorable LETTERS and marks outside Cc/Cf
//! (U+115F, U+1160, U+3164, U+FFA0 the Hangul fillers, U+034F, U+17B4, U+17B5) and U+2800 (Braille blank, So) pass,
//! so `"Bob\u{3164}"` reads as `Bob`; the tag is what tells such a watcher from the seat holder (a seat carries none).
//! And U+200C, the zero-width NON-joiner that Persian and several Indic scripts spell with, is Cf and is refused.
//!
//! **Why a table and not a crate:** this module compiles to every host — the NAS binary, the browser's wasm and
//! the Worker's — and the fence admits nothing new to that tree; the Cf category is 21 ranges, which a reader can
//! check against the UCD in a minute. **Transcribed from Unicode 17.0.0**, the toolchain's own version
//! (`char::UNICODE_VERSION`, pinned by a cell): `https://www.unicode.org/Public/17.0.0/ucd/extracted/DerivedGeneralCategory.txt`
//! (dated 2025-07-24), the lines whose category is `Cf`, in the file's order. A later toolchain that moves its
//! tables fails the pin, which names this comment.

/// Unicode 17.0.0's `General_Category=Cf`, 21 ranges, 170 code points — see the module doc for the source.
pub const FORMAT_RANGES: [(u32, u32); 21] = [
    (0x00AD, 0x00AD),
    (0x0600, 0x0605),
    (0x061C, 0x061C),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x180E, 0x180E),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x2064),
    (0x2066, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0001, 0xE0001),
    (0xE0020, 0xE007F),
];

/// How many code points the table covers — the UCD's own count for the category, pinned by a cell that sums the
/// ranges AND counts the predicate over the whole scalar space.
pub const FORMAT_COUNT: usize = 170;

/// The rule, as every refusal words it: the sentence names the rule, never the character (the echo rule's spirit —
/// an invisible character quoted back would be invisible in the refusal too).
pub const INVISIBLE_RULE: &str = "no control, invisible or direction-changing character";

/// `General_Category=Cf` in Unicode 17.0.0.
pub fn is_format(c: char) -> bool {
    let u = c as u32;
    FORMAT_RANGES.iter().any(|&(a, b)| (a..=b).contains(&u))
}

/// `Zl` (U+2028 LINE SEPARATOR) and `Zp` (U+2029 PARAGRAPH SEPARATOR): invisible, and a line break a "one line"
/// rule must refuse as it refuses `\n`.
pub fn is_line_separator(c: char) -> bool {
    matches!(c, '\u{2028}' | '\u{2029}')
}

/// The first character of `s` the rule refuses, or `None` for text a person can see whole. The character is for a
/// CELL to assert on; a sentence carries [`INVISIBLE_RULE`].
pub fn invisible(s: &str) -> Option<char> {
    s.chars().find(|&c| c.is_control() || is_format(c) || is_line_separator(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The table IS Unicode 17.0.0's General_Category Cf** — the toolchain's own version (`char::UNICODE_VERSION`),
    /// so `is_control`'s Cc and this table's Cf are read from the same edition: 21 sorted, disjoint ranges, 170 code
    /// points, every bound a scalar value, and the predicate counts exactly those over the whole space. A toolchain
    /// that moves its tables fails the first line, which says what to do.
    #[test]
    fn the_format_table_is_unicode_17s_cf_sorted_disjoint_and_170_long() {
        assert_eq!(char::UNICODE_VERSION, (17, 0, 0), "the toolchain's Unicode tables moved: re-transcribe FORMAT_RANGES from that version's DerivedGeneralCategory.txt and move this pin");
        assert_eq!(FORMAT_RANGES.len(), 21);
        let total: u32 = FORMAT_RANGES.iter().map(|&(a, b)| b - a + 1).sum();
        assert_eq!(total as usize, FORMAT_COUNT);
        assert_eq!(FORMAT_COUNT, 170, "the UCD's count for 17.0.0 (2025-07-24)");
        for w in FORMAT_RANGES.windows(2) {
            assert!(w[0].1 < w[1].0, "sorted and disjoint: {:?} then {:?}", w[0], w[1]);
        }
        for &(a, b) in &FORMAT_RANGES {
            assert!(a <= b, "{a:04X}..{b:04X}");
            assert!(char::from_u32(a).is_some() && char::from_u32(b).is_some(), "{a:04X}..{b:04X} must be scalar values");
        }
        let counted = (0..=0x10FFFF).filter_map(char::from_u32).filter(|c| is_format(*c)).count();
        assert_eq!(counted, FORMAT_COUNT, "the predicate and the table disagree");
        assert_eq!((0..=0x10FFFF).filter_map(char::from_u32).filter(|c| c.is_control()).count(), 65, "Cc, the toolchain's");
    }

    /// **What the rule refuses and what it admits** (finding D8, measured: a watcher posted as `"Bob\u{200B}"` and
    /// read back as the seat holder "Bob"; `"\u{202E}boB"` reversed the line). One representative from every family
    /// — zero-width, bidi override and isolate, the BOM, the soft hyphen, a tag character, a musical format
    /// character, the Mongolian vowel separator, the two line separators, a control — against visible text in four
    /// scripts, an emoji, a flag (two regional indicators) and a keycap (a variation selector, category Mn, passes).
    /// The joiner is Cf and is refused: a multi-person emoji sequence is the accepted loss (decision D-4).
    #[test]
    fn invisible_refuses_control_format_and_line_separators_and_admits_visible_text() {
        for bad in [
            "a\u{200B}b", "\u{202E}evil", "x\u{2066}y", "\u{FEFF}bom", "soft\u{00AD}hyphen", "tag\u{E0041}",
            "music\u{1D173}", "\u{180E}", "\u{110BD}", "\u{E0001}", "line\u{2028}two", "para\u{2029}two",
            "a\tb", "ctl\u{7}", "join\u{200D}er", "\u{061C}", "\u{FFF9}",
        ] {
            assert!(invisible(bad).is_some(), "{bad:?} must be refused");
        }
        for good in ["héllo", "日本語", "Кириллица", "العربية", "🙂 ok", "\u{1F1EF}\u{1F1F5}", "1\u{FE0F}\u{20E3}", "Ben Castle", "a..b-c_d", "x y"] {
            assert_eq!(invisible(good), None, "{good:?} is visible text");
        }
        assert_eq!(invisible("ok\u{202E}x"), Some('\u{202E}'), "the first offending character, for a cell — never for a sentence");
        assert_eq!(INVISIBLE_RULE, "no control, invisible or direction-changing character");
    }
}
