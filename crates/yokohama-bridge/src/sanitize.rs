//! Display-boundary sanitizer for untrusted captured text.
//!
//! Every string in a capture file is written by the BGA page, and the page is
//! exactly as trustworthy as whatever script runs on it (design threat model;
//! H0 review N-5): an opponent's display name, a `payload.source`, a tile or
//! tech title can each carry arbitrary bytes. When such a string becomes
//! TERMINAL TEXT — an error message, an answer-sheet line, a live spectate
//! frame — raw control bytes let it clear the screen, recolour later output,
//! reset the window title, or plant an OSC-8 hyperlink. The importer holds
//! itself to the same zero-ESC discipline the CLI's `piped_dogfood` gate
//! enforces for the driver, so it routes every off-the-wire string through
//! [`sanitize`] at the point it becomes display text.
//!
//! The rule: any Unicode control (`char::is_control` — C0 `U+0000..=U+001F`,
//! DEL `U+007F`, C1 `U+0080..=U+009F`, which is exactly ESC/BEL/CR/LF/TAB and
//! the OSC introducers) is replaced by a visible, ASCII-only `\u{..}` escape.
//! Ordinary text — including every non-ASCII letter a real display name uses —
//! passes through untouched.

use std::fmt::Write;

/// Replace every control character (C0, DEL, C1) with a visible `\u{..}`
/// escape, leaving all other text verbatim.
///
/// Idempotent: the escape it emits contains only ASCII graphics, so a second
/// pass over sanitized text changes nothing — a format site may wrap a string
/// that is already clean without harm.
pub fn sanitize(s: &str) -> String {
    // The overwhelmingly common case is clean text; avoid rebuilding the string
    // when there is nothing to escape.
    if !s.chars().any(char::is_control) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if c.is_control() {
            // `\u{1b}`, `\u{07}`, `\u{9b}` — self-describing and free of any
            // control byte, so it can never re-enter the terminal as an escape.
            let _ = write!(out, "\\u{{{:02x}}}", c as u32);
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::sanitize;

    #[test]
    fn clean_text_including_unicode_letters_passes_through_verbatim() {
        for s in ["XeroxMan", "acmuriel", "Table 895724349", "café — ünïcode ok", ""] {
            assert_eq!(sanitize(s), s, "clean text must be untouched");
        }
    }

    #[test]
    fn every_control_class_is_escaped_and_no_control_byte_survives() {
        // ESC, BEL, CR, LF, TAB (C0); DEL; a C1 introducer — the full attack
        // vocabulary the finding named (screen clear, colour, OSC title, OSC-8).
        let hostile = "\u{1b}[2J\u{1b}[31mPWNED\u{07}\u{0d}\u{0a}\u{7f}\u{9b}end";
        let clean = sanitize(hostile);
        assert!(
            !clean.chars().any(char::is_control),
            "no control character may survive: {clean:?}"
        );
        assert!(!clean.contains('\u{1b}'), "no raw ESC byte: {clean:?}");
        // The escapes are visible and name the codepoint they replaced.
        assert!(clean.contains("\\u{1b}"), "ESC shows as \\u{{1b}}: {clean:?}");
        assert!(clean.contains("\\u{07}"), "BEL shows as \\u{{07}}: {clean:?}");
        assert!(clean.contains("\\u{7f}"), "DEL shows as \\u{{7f}}: {clean:?}");
        // The printable runs survive (they are separated by the escapes).
        assert!(clean.contains("PWNED"), "printable text is preserved: {clean:?}");
        assert!(clean.ends_with("end"), "the trailing text is preserved: {clean:?}");
    }

    #[test]
    fn sanitizing_is_idempotent() {
        let once = sanitize("name\u{1b}[0m\u{0a}x");
        assert_eq!(sanitize(&once), once, "a second pass must change nothing");
    }
}
