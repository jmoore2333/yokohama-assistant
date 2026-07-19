//! Shared order/tech token parsing and tech-name display (design
//! docs/plans/2026-07-18-choice-encoding-design.md §5.5, riding item 4):
//! extracted from the wizard's near-verbatim prefix-parse copies and the
//! REPL's own order-token copy, plus the triplicated tech-name fallback.
//!
//! Tech-name ambiguity resolution deliberately keeps TWO policies rather
//! than unifying them (§5.5 item 1, CRITICAL constraint): the setup wizard
//! is recording which physical card sits where, so a genuine ambiguity
//! must be refused rather than guessed (`TechAmbiguity::Reject`); the
//! REPL's live-reveal collection is mid-game, where any same-named copy is
//! interchangeable, so it resolves deterministically to the lowest id
//! (`TechAmbiguity::LowestId`, matching `reveal.rs`'s pre-existing
//! behavior). Do NOT collapse these into one policy.

use yokohama_core::*;

/// Parse an order-card token ("O17"/"o17" or a bare "17") into its numeric
/// id -- no membership validation; callers check against whatever pool
/// applies (registry, deck, hand, ...).
pub(crate) fn parse_order_id(tok: &str) -> Option<u8> {
    tok.strip_prefix(['O', 'o']).unwrap_or(tok).parse::<u8>().ok()
}

/// How to resolve a tech-name prefix that matches more than one candidate
/// (some titles circulate as 2 physical copies under the same name).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TechAmbiguity {
    /// Setup wizard: recording physical identity, so a real ambiguity must
    /// be refused rather than guessed.
    Reject,
    /// REPL live-reveal: any same-named copy is interchangeable mid-game,
    /// so resolve deterministically to the lowest id.
    LowestId,
}

/// Parse a tech-card token ("T05"/"t05" or a case-insensitive name prefix)
/// into one of `candidates` (the wizard passes every registry id; a live
/// reveal passes only the ids currently in the deck). Numeric tokens are
/// checked against `candidates` too, so both callers reject a real-but-
/// unavailable card the same way they reject a name with no match.
pub(crate) fn parse_tech_entry(
    tok: &str,
    techs: &TechRegistry,
    candidates: &[TechCardId],
    ambiguity: TechAmbiguity,
) -> Result<TechCardId, String> {
    if let Some(rest) = tok.strip_prefix(['T', 't']) {
        if let Ok(n) = rest.parse::<u8>() {
            let card = TechCardId(n);
            return if candidates.contains(&card) {
                Ok(card)
            } else {
                Err(format!("'{tok}' is not a valid tech card"))
            };
        }
    }
    let needle = tok.to_lowercase();
    let mut matches: Vec<TechCardId> = candidates
        .iter()
        .copied()
        .filter(|id| techs.get(*id).is_some_and(|c| c.name.to_lowercase().starts_with(&needle)))
        .collect();
    matches.sort_unstable_by_key(|id| id.0);
    match matches.len() {
        0 => Err(format!("'{tok}' does not match any tech card")),
        1 => Ok(matches[0]),
        _ => match ambiguity {
            TechAmbiguity::LowestId => Ok(matches[0]), // already sorted ascending
            TechAmbiguity::Reject => Err(format!("'{tok}' matches more than one tech card -- use T<id>")),
        },
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

    #[test]
    fn parse_tech_entry_checks_numeric_against_candidates_and_resolves_names_by_policy() {
        let rules = Rules::standard();
        // TechCardId(11)/(12) are both "Ball" (data.rs) -- one of the 10
        // titles that circulate as 2 physical copies (per reveal.rs's
        // pre-existing fixture comment).
        let candidates = vec![TechCardId(11), TechCardId(12)];
        assert_eq!(parse_tech_entry("T11", &rules.techs, &candidates, TechAmbiguity::Reject), Ok(TechCardId(11)));
        assert!(parse_tech_entry("T99", &rules.techs, &candidates, TechAmbiguity::Reject).is_err(), "not in candidates");
        assert!(parse_tech_entry("zzz", &rules.techs, &candidates, TechAmbiguity::Reject).is_err(), "no name match");
        assert!(parse_tech_entry("ball", &rules.techs, &candidates, TechAmbiguity::Reject).is_err(), "ambiguous under Reject");
        assert_eq!(
            parse_tech_entry("ball", &rules.techs, &candidates, TechAmbiguity::LowestId),
            Ok(TechCardId(11)),
            "ambiguous under LowestId resolves to the lowest id"
        );
    }

    #[test]
    fn tech_name_looks_up_the_registry_and_falls_back_for_unknown_ids() {
        let rules = Rules::standard();
        assert_eq!(tech_name(TechCardId(11), &rules.techs), "Ball");
        assert_eq!(tech_name(TechCardId(255), &rules.techs), "T255");
    }
}
