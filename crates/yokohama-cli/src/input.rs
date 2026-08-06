//! Shared order/tech token parsing and tech-name display (design
//! docs/plans/2026-07-18-choice-encoding-design.md §5.5, riding item 4):
//! extracted from the wizard's near-verbatim prefix-parse copies and the
//! REPL's own order-token copy, plus the triplicated tech-name fallback.
//!
//! **Tech-name ambiguity resolves to the lowest matching id, once, for
//! everyone — and that supersedes a rule this module used to state the other
//! way.** Two policies lived here: the setup wizard REFUSED a name matching
//! two candidates ("recording which physical card sits where"), the live
//! reveal resolved it. Phase 3.5a ruled the refusal to be the defect: ten
//! titles circulate as two IDENTICAL cards (R8, spec §0.6), so asking an
//! operator which of two indistinguishable pieces of cardboard they are
//! holding is a question with no answer — and the escape it offered, `T<id>`,
//! is an engine invention printed on no card at all (F4/F5, the owner's own
//! *"so I just guess through it"*). The wizard now names technologies through
//! `card_prompt`, which collapses a title's copies and hands back the first
//! one nothing has claimed; this module keeps the deck-facing half of the same
//! rule.

use yokohama_core::*;

/// Parse an order-card token ("O17"/"o17" or a bare "17") into its numeric
/// id -- no membership validation; callers check against whatever pool
/// applies (registry, deck, hand, ...).
pub(crate) fn parse_order_id(tok: &str) -> Option<u8> {
    tok.strip_prefix(['O', 'o']).unwrap_or(tok).parse::<u8>().ok()
}

/// Why `parse_tech_entry` rejected a token -- kept structured rather than a
/// formatted `String` because its two callers describe the SAME failure to
/// two very different audiences (fix wave, review of commit 0a01c95 finding
/// 2): the setup wizard is checking the full registry ("not a valid tech
/// card"), a live reveal is checking only the current deck ("not currently
/// in the tech deck" -- a card can be real but already drawn, which is a
/// materially different fact for someone tracking a live game). Sharing one
/// wording between the two silently regressed the reveal path's message
/// when the parsing logic was first unified; each caller now renders its
/// own text from this error via `match`, so the wording can never drift
/// back together by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TechEntryError {
    /// A syntactically valid `T<id>` numeric token, but `id` (the payload)
    /// isn't in the candidate pool.
    NotInPool(u8),
    /// A name prefix that matched zero candidates.
    NoMatch,
}

/// Parse a tech-card token ("T05"/"t05" or a case-insensitive name prefix)
/// into one of `candidates` -- the ids currently in the deck, for its one
/// caller. Numeric tokens are checked against `candidates` too, so a
/// real-but-unavailable card is rejected the same way a name with no match is.
/// A name matching several candidates resolves to the LOWEST id (the copies of
/// a title are interchangeable -- see the module header). Returns a
/// `TechEntryError` rather than a formatted message -- see its doc comment for
/// why.
pub(crate) fn parse_tech_entry(
    tok: &str,
    techs: &TechRegistry,
    candidates: &[TechCardId],
) -> Result<TechCardId, TechEntryError> {
    if let Some(rest) = tok.strip_prefix(['T', 't']) {
        if let Ok(n) = rest.parse::<u8>() {
            let card = TechCardId(n);
            return if candidates.contains(&card) { Ok(card) } else { Err(TechEntryError::NotInPool(n)) };
        }
    }
    let needle = tok.to_lowercase();
    let mut matches: Vec<TechCardId> = candidates
        .iter()
        .copied()
        .filter(|id| techs.get(*id).is_some_and(|c| c.name.to_lowercase().starts_with(&needle)))
        .collect();
    matches.sort_unstable_by_key(|id| id.0);
    match matches.first() {
        Some(lowest) => Ok(*lowest), // sorted ascending above
        None => Err(TechEntryError::NoMatch),
    }
}

/// Tech card's display name, or a `T<id>` fallback for an id absent from
/// the registry (keeps display code panic-free against corrupt/foreign
/// records) -- the one-liner that used to be triplicated across
/// render.rs and describe.rs.
pub(crate) fn tech_name(id: TechCardId, techs: &TechRegistry) -> String {
    techs.get(id).map(|c| c.name.clone()).unwrap_or_else(|| format!("T{:02}", id.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_order_id_accepts_prefixed_and_bare_digits_only() {
        assert_eq!(parse_order_id("O17"), Some(17));
        assert_eq!(parse_order_id("o17"), Some(17));
        assert_eq!(parse_order_id("17"), Some(17));
        assert_eq!(parse_order_id("abc"), None);
        assert_eq!(parse_order_id(""), None);
        assert_eq!(parse_order_id("O999999999999"), None); // overflows u8
    }

    /// **One policy, and the T7 half of the ruling that made it one.** A name
    /// matching both copies of a title resolves to the lowest id rather than
    /// being refused: the two cards are identical, so there is no third answer
    /// the operator could give. `T<id>` still works for anyone who has one.
    #[test]
    fn parse_tech_entry_checks_numeric_against_candidates_and_resolves_a_name_to_the_lowest_id() {
        let rules = Rules::standard();
        // TechCardId(11)/(12) are both "Ball" (data.rs) -- one of the 10
        // titles that circulate as 2 physical copies (per reveal.rs's
        // pre-existing fixture comment).
        let candidates = vec![TechCardId(11), TechCardId(12)];
        assert_eq!(parse_tech_entry("T11", &rules.techs, &candidates), Ok(TechCardId(11)));
        assert_eq!(
            parse_tech_entry("T99", &rules.techs, &candidates),
            Err(TechEntryError::NotInPool(99)),
            "not in candidates"
        );
        assert_eq!(
            parse_tech_entry("zzz", &rules.techs, &candidates),
            Err(TechEntryError::NoMatch),
            "no name match"
        );
        assert_eq!(
            parse_tech_entry("ball", &rules.techs, &candidates),
            Ok(TechCardId(11)),
            "a title's two copies are one answer: the lowest id still in the pool"
        );
        assert_eq!(
            parse_tech_entry("ball", &rules.techs, &[TechCardId(12)]),
            Ok(TechCardId(12)),
            "...and the pool decides which, so a claimed copy is skipped by its absence"
        );
    }

    #[test]
    fn tech_name_looks_up_the_registry_and_falls_back_for_unknown_ids() {
        let rules = Rules::standard();
        assert_eq!(tech_name(TechCardId(11), &rules.techs), "Ball");
        assert_eq!(tech_name(TechCardId(255), &rules.techs), "T255");
    }
}
