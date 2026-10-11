//! **The echo rule** (security S1c, findings D12 and D28). An error sentence
//! that repeats what the caller sent is the right sentence — a refusal that
//! only says "no" gets worked around — and it is also an amplifier and a
//! channel: the review measured a 940 KB line answered with a 3.3 MB body,
//! and read the raw path, the id, the filter term and the identity header
//! echoed whole. One helper bounds every echo; the sites keep their words.

/// How much of a caller's value a sentence repeats. Sixty-four characters is
/// the longest id or name this API admits by rule — a game id; a name's
/// thirty-two — so a legitimate one is never cut, and a hostile value is cut
/// to the size of a legitimate one. (An email may run to 254 bytes; the
/// sites that echo an address echo one they refused.)
pub const ECHO_MAX: usize = 64;

/// `value` as a sentence repeats it: the first [`ECHO_MAX`] characters,
/// escaped as `{:?}` escapes a `str` (double quotes, backslashes, controls,
/// combining marks and every non-printable code point as `\u{…}`), and `…`
/// after a cut. NO quotes of its own — the site writes them, so a site that
/// quoted with `{x:?}` writes `"{}"` and reads byte-identically for every
/// value of at most [`ECHO_MAX`] characters.
///
/// **`{:?}` itself, less its two quotes, and not `str::escape_debug`**: the
/// latter escapes a single quote (`it\'s`) and leaves a combining mark after
/// the first character raw, so an apostrophe in a name would have read
/// differently from the `{x:?}` sentence it replaced.
pub fn echo(value: &str) -> String {
    let shown: String = value.chars().take(ECHO_MAX).collect();
    let cut = value.chars().nth(ECHO_MAX).is_some();
    let mut out = format!("{shown:?}");
    out.pop();
    out.remove(0);
    if cut {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A short, clean value reads exactly as `{:?}` read it, less the quotes —
    /// the premise every pinned sentence in the crate rests on. The last two
    /// are where `str::escape_debug` reads differently: an apostrophe and a
    /// combining mark after the first character.
    #[test]
    fn a_short_clean_value_is_itself() {
        for v in ["wire", "/api/nope", "x.js", "red@wire.test", "A.b-c_d", "a b", "it's", "e\u{301}"] {
            assert_eq!(format!("\"{}\"", echo(v)), format!("{v:?}"));
        }
    }

    /// The cut is at sixty-four CHARACTERS, marked, and never inside a code
    /// point; the escape turns a control, a format character and a quote into
    /// their escapes, so no echo carries a raw newline or a soft hyphen.
    #[test]
    fn a_long_or_hostile_value_is_cut_and_escaped() {
        let long = "x".repeat(ECHO_MAX + 1);
        let e = echo(&long);
        assert_eq!(e.chars().count(), ECHO_MAX + 1, "{e}");
        assert!(e.ends_with('…'));
        assert_eq!(echo(&"x".repeat(ECHO_MAX)), "x".repeat(ECHO_MAX), "exactly the bound is not cut");
        let soft = "\u{ad}".repeat(1_000);
        let e = echo(&soft);
        assert!(!e.contains('\u{ad}'), "a format character is escaped: {e}");
        assert!(e.starts_with("\\u{ad}"), "{e}");
        assert!(e.len() < 64 * 8, "the escape of sixty-four code points is bounded: {}", e.len());
        assert_eq!(echo("1\nsave x"), "1\\nsave x");
        assert_eq!(echo("say \"hi\""), "say \\\"hi\\\"");
        let wide = "日本".repeat(40);
        assert_eq!(echo(&wide).chars().count(), ECHO_MAX + 1, "cut by character, not by byte");
    }
}
