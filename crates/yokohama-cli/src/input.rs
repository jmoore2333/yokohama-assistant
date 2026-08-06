//! Tech-name display (design docs/plans/2026-07-18-choice-encoding-design.md
//! §5.5, riding item 4): what is left of the shared token parsing this module
//! was extracted for.
//!
//! **The token parsers retired at T8, and the sequence is worth recording
//! because it is the shape of the whole 3.5a input work.** This module once
//! held three grammars' common half: an order-token parser (`O17`/`17`), a
//! tech-token parser (`T05`/a name prefix), and a `TechEntryError` whose two
//! variants existed so the wizard and the live reveal could word the same
//! failure for different audiences. T7 moved the wizard onto
//! [`crate::card_prompt`], the crate's ONE card grammar (spec §0.2, findings
//! F4/F5); T8 moved the live reveal onto it too, which left both parsers with no
//! caller. They are gone rather than kept warm: a second parser for card tokens
//! is exactly how the driver came to have four grammars, and one that compiles
//! but is unreachable is the version of that nobody notices.
//!
//! What SURVIVED the move, in `card_prompt` and `reveal`: a name matching both
//! copies of a title is ONE answer rather than an ambiguity (ten titles
//! circulate as two IDENTICAL cards — R8, spec §0.6 — so asking an operator
//! which of two indistinguishable pieces of cardboard they hold is a question
//! with no answer), and a live reveal's refusal still talks about the DECK
//! rather than the registry ([`crate::reveal::check_tech_reveal`], where that
//! wording is pinned).

use yokohama_core::*;

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
    fn tech_name_looks_up_the_registry_and_falls_back_for_unknown_ids() {
        let rules = Rules::standard();
        assert_eq!(tech_name(TechCardId(11), &rules.techs), "Ball");
        assert_eq!(tech_name(TechCardId(255), &rules.techs), "T255");
    }
}
