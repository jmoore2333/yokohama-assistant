//! **One card-identification grammar, used at every point that asks "which
//! card?"** (Phase-3.5a spec §0.2/§0.6, findings F4/F5/F6, plan T5).
//!
//! **The defect this closes, in the owner's own words from the live-gate
//! attempt.** Order prompts took a bare `14`; tech prompts refused `12`, `1`
//! and `2` alike and wanted `T<id>` or a name — *"so I just guess through
//! it"* (F4). Port entry refused `2`, `4` and `6` as *"already entered"* with
//! no hint that they were sitting in the seats' recorded deals (F6). And no
//! prompt anywhere offered a way to LOOK: the tech ids are an engine invention
//! printed on no physical card, so they are unknowable without a table (F5).
//! Three defects, one cause — every prompt invented its own grammar. This
//! module is the grammar, once.
//!
//! **The rule, in one sentence:** a bare number is an id at an id prompt and a
//! selection where an index is on screen; `?` opens the index; a name or
//! prefix works wherever cards have names; a duplicate is refused BY NAME AND
//! LOCATION; and the entry echoes the full card back before it commits.
//!
//! **Why a number is never ambiguous.** The two readings of a bare number —
//! "card O14" and "the 14th card shown" — cannot both be on screen at once,
//! because the index is a FULL-SCREEN surface: while it is open the prompt's
//! own list is not visible, and while it is closed there are no selection
//! numbers to mean. [`Numbering`] carries which of the two states the caller is
//! in, and it is a parameter rather than a guess.
//!
//! **This module parses; it never decides what is legal.** [`resolve`] returns
//! a named card or a refusal — the CALLER checks its own pool (the order deck,
//! the boxed pile, the registry) and commits. That split is deliberate: the
//! live reveal's pool is "unrevealed cards" and the wizard's is "the whole
//! registry", and a shared grammar that also knew those rules would be a
//! second opinion about the game state.

use yokohama_core::{Country, OrderCardId, Rules};

use crate::describe;
use crate::screen_model::{
    menu_index, CardGood, CardLocation, IndexFilter, IndexModel, NUMBER_BASE,
};

/// How a bare number reads at this prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Numbering<'a> {
    /// No index on screen: a bare number is a card ID (`14` == `O14`), which is
    /// the grammar the order prompts already had and F4 asked for everywhere.
    Ids,
    /// The index is open: a bare number is the SELECTION number printed on a
    /// card, resolved through the model that printed it so the number on screen
    /// and the number this answers cannot drift apart.
    Selection(&'a IndexModel),
}

/// One card-entry prompt.
///
/// There is no `kind` field, and its absence is the honest state of things:
/// every card prompt in the driver today identifies an ORDER card, so a
/// discriminator would be a field nothing could branch on. T6 adds one with
/// the tech, site and achievement indexes — the registry it would name is
/// exactly what [`crate::screen_model::IndexKind`] is keyed for.
pub(crate) struct CardPrompt<'a> {
    /// Every card the prompt could legitimately be told about — the registry
    /// for a setup entry, the unrevealed cards for a live reveal. Used ONLY to
    /// word a refusal helpfully; the caller still validates what it gets back.
    ///
    /// `None` is "this prompt states no pool", which is a different fact from
    /// `Some(&[])` — "this prompt's pool is empty, so nothing is enterable" —
    /// and the two used to be the same value (T5 review): an empty slice was the
    /// sentinel for no-restriction, so a caller that computed its pool and got
    /// nothing would have opened the prompt to the whole registry.
    pub candidates: Option<&'a [OrderCardId]>,
    /// Cards already recorded elsewhere, and where (F6).
    pub placed: &'a [(OrderCardId, CardLocation)],
    /// How a bare number reads right now.
    pub numbering: Numbering<'a>,
}

/// What one line of input at a card prompt turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CardEntry {
    /// A card was named. The caller validates it against its own pool, echoes
    /// it ([`echo`]), and commits.
    Card(OrderCardId),
    /// `?` — open the index for this prompt.
    OpenIndex,
    /// `f <term>` — narrow the index. `None` clears the filter.
    Filter(Option<IndexFilter>),
    /// An empty line, `cancel`, or `back`: leave the prompt as it was.
    Cancel,
    /// Nothing usable. The message is ready to print and already names the
    /// location when the refusal is a duplicate.
    Refused(String),
}

/// Resolve one line at a card prompt.
///
/// The order of the arms is the grammar's precedence and it is not arbitrary:
/// the two ESCAPES (`?`, `cancel`) are checked before anything that could be
/// mistaken for a card, so neither can be shadowed by a registry that happens
/// to hold a card called `cancel` one day — the same reasoning `wizard::ask`
/// applies to `abort`.
pub(crate) fn resolve(prompt: &CardPrompt<'_>, line: &str, rules: &Rules) -> CardEntry {
    let line = line.trim();
    if line.is_empty() || line.eq_ignore_ascii_case("cancel") || line.eq_ignore_ascii_case("back") {
        return CardEntry::Cancel;
    }
    if line == "?" || line.eq_ignore_ascii_case("index") {
        return CardEntry::OpenIndex;
    }
    if let Some(rest) = strip_filter_prefix(line) {
        return match parse_filter(rest, rules) {
            Ok(filter) => CardEntry::Filter(filter),
            Err(message) => CardEntry::Refused(message),
        };
    }

    // A bare number: a selection while the index is up, an id otherwise.
    if line.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(n) = line.parse::<usize>() {
            if let Numbering::Selection(index) = prompt.numbering {
                return match index.select(n) {
                    Some(id) => named(prompt, id),
                    None => CardEntry::Refused(selection_range_message(index)),
                };
            }
            if let Ok(id) = u8::try_from(n) {
                return named(prompt, OrderCardId(id));
            }
        }
        return CardEntry::Refused(format!("'{line}' is not a card number on this screen."));
    }

    // `O14`/`o14` — the same id, spelled the way the corner of the card spells
    // it. Accepted whether or not an index is open: a player who knows the id
    // is not made to find its position on a screen.
    if let Some(id) = crate::input::parse_order_id(line).filter(|_| line.len() > 1) {
        return named(prompt, OrderCardId(id));
    }

    // A country name or prefix. Orders carry a FLAG rather than a title, so a
    // country never identifies one card — it identifies a group, which is
    // exactly what the index filter is for, so that is what this becomes
    // rather than a refusal the player has to translate.
    if let Some(country) = country_prefix(line) {
        return CardEntry::Filter(Some(IndexFilter::Country(country)));
    }

    CardEntry::Refused(format!(
        "'{line}' is not a card. Type its number, its id (like 'O14'), or '?' for the index.",
    ))
}

/// A named card, refused if it is already recorded somewhere (F6: the refusal
/// names where) or if it is not a card at all.
fn named(prompt: &CardPrompt<'_>, id: OrderCardId) -> CardEntry {
    if let Some((_, at)) = prompt.placed.iter().find(|(placed, _)| *placed == id) {
        return CardEntry::Refused(format!(
            "O{:02} is {} — a card cannot be in two places.",
            id.0,
            describe::card_location(*at),
        ));
    }
    if prompt.candidates.is_some_and(|pool| !pool.contains(&id)) {
        return CardEntry::Refused(format!(
            "O{:02} is not one of the cards this prompt can take.",
            id.0,
        ));
    }
    CardEntry::Card(id)
}

/// The full card, drawn, for the echo spec §0.6 requires before an entered id
/// commits: *"entered ids echo the full card back before committing"*.
///
/// It is an ECHO rather than a confirmation step, and that is a ruling worth
/// stating: R6 asks for minimum ceremony at the table, and a second keystroke
/// per card would double the cost of the very flow this phase exists to speed
/// up. What the player gets is the card on screen, in the same shape they just
/// saw it in the index, above the result of their entry — with `back` still
/// there if it is the wrong one.
pub(crate) fn echo(id: OrderCardId, rules: &Rules) -> String {
    let printed = rules.orders.get(id);
    let mut out = String::new();
    for line in crate::card_render::order_card(&crate::card_render::CardFace::new(id, printed)) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// `f`, `f `, `filter ` — the filter grammar the index's own help line teaches.
///
/// **Case-insensitive, like every other clause of this grammar** (T5 review S1):
/// `?`, `index`, `cancel`, `back` and `O14` all ignore case, and a `F silk` that
/// fell through to "is not a card" would be the one keyword in the module that
/// cared about the shift key.
fn strip_filter_prefix(line: &str) -> Option<&str> {
    for prefix in ["filter", "f"] {
        // `get` rather than slicing: a line whose first bytes are mid-character
        // yields `None` here instead of panicking on a char boundary.
        if line.get(..prefix.len()).is_some_and(|head| head.eq_ignore_ascii_case(prefix)) {
            let rest = &line[prefix.len()..];
            if rest.is_empty() || rest.starts_with(' ') {
                return Some(rest.trim());
            }
        }
    }
    None
}

/// Parse a filter term: a good, a VP floor (`8vp`), or a country. An empty term
/// CLEARS the filter (`Ok(None)`); anything the grammar does not recognize is a
/// REFUSAL naming the forms that do work.
///
/// **An unrecognized term used to clear the filter**, which is the quietest kind
/// of wrong answer a prompt can give (T5 review): `f 8` looked like it had done
/// something — the screen visibly changed — while what it actually did was throw
/// away the filter the player was reading. The refusal costs one line and leaves
/// the screen exactly where it was.
///
/// **Prefixes refuse ambiguity rather than resolving it**, the rule
/// [`country_prefix`] already followed and the goods table did not: `f s` used
/// to answer with seafood because seafood is listed above silk, so the player
/// got a confident wrong screen instead of a question. One rule for both now,
/// pinned by `a_prefix_that_names_two_things_is_refused_not_guessed`.
pub(crate) fn parse_filter(term: &str, _rules: &Rules) -> Result<Option<IndexFilter>, String> {
    // Messages quote what was TYPED; matching is done on the folded form.
    let typed = term.trim();
    let term = typed.to_lowercase();
    if term.is_empty() {
        return Ok(None);
    }
    match good_matches(&term).as_slice() {
        [only] => return Ok(Some(IndexFilter::Good(*only))),
        [] => {}
        many => {
            let names: Vec<&str> = many.iter().map(|g| describe::card_good_name(*g)).collect();
            return Err(ambiguous_message(typed, &names));
        }
    }
    if let Some(digits) = term.strip_suffix("vp").or_else(|| term.strip_suffix(" vp")) {
        if let Ok(n) = digits.trim().parse::<u8>() {
            return Ok(Some(IndexFilter::MinVp(n)));
        }
    }
    match country_matches(&term).as_slice() {
        [only] => Ok(Some(IndexFilter::Country(*only))),
        [] => Err(unrecognized_message(typed)),
        many => {
            let names: Vec<&str> = many.iter().map(|c| describe::country_name(*c)).collect();
            Err(ambiguous_message(typed, &names))
        }
    }
}

/// Every good a term prefixes, deduped by GOOD rather than by word — `fish` and
/// `seafood` name one good, so the alias never reads as an ambiguity, while `s`
/// names two and does.
fn good_matches(needle: &str) -> Vec<CardGood> {
    const WORDS: [(&str, CardGood); 5] = [
        ("seafood", CardGood::Seafood),
        ("fish", CardGood::Seafood),
        ("tea", CardGood::Tea),
        ("silk", CardGood::Silk),
        ("copper", CardGood::Copper),
    ];
    let mut hits: Vec<CardGood> = Vec::new();
    for (word, good) in WORDS {
        if word.starts_with(needle) && !hits.contains(&good) {
            hits.push(good);
        }
    }
    hits
}

/// Every country a term prefixes, in `Country::ALL` order.
fn country_matches(term: &str) -> Vec<Country> {
    let needle = term.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    Country::ALL
        .into_iter()
        .filter(|c| describe::country_name(*c).to_lowercase().starts_with(&needle))
        .collect()
}

/// A country named by any prefix of its display name — `fr`, `france`,
/// `Netherlands`. Prefixes are unique across the five names from the first
/// letter, which `country_prefixes_are_unique_from_one_letter` pins rather
/// than assumes; an ambiguous one names no country.
fn country_prefix(term: &str) -> Option<Country> {
    match country_matches(term).as_slice() {
        [only] => Some(*only),
        _ => None,
    }
}

/// `'s' could be seafood or silk — type more of the word.`
fn ambiguous_message(term: &str, names: &[&str]) -> String {
    format!("'{term}' could be {} — type more of the word.", join_or(names))
}

/// `'8' is not a good, flag, or '<n>vp' — …`, with the three forms that work.
fn unrecognized_message(term: &str) -> String {
    format!(
        "'{term}' is not a good, flag, or '<n>vp' — try 'f silk', 'f 8vp', 'f france', or 'f' to clear.",
    )
}

/// `a`, `a or b`, `a, b or c` — the list an ambiguity names.
fn join_or(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [only] => only.to_string(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// The range message a selection number outside the screen gets.
fn selection_range_message(index: &IndexModel) -> String {
    match index.cards().filter_map(|c| c.selection).max() {
        Some(highest) => format!("pick a number {NUMBER_BASE}-{highest} from this screen, or '?' to close it"),
        None => "nothing on this screen is selectable — Enter returns to your prompt".to_string(),
    }
}

/// The 0-based position a typed selection refers to in a list of `len`, for the
/// numbered surfaces that are NOT card indexes. Re-exported through this module
/// so every parser in the crate reaches one place for the base.
pub(crate) fn list_index(token: &str, len: usize) -> Option<usize> {
    token.parse::<usize>().ok().and_then(|n| menu_index(n, len))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen_model::{build_order_index, RecordingTarget};
    use yokohama_core::PlayerId;

    const TARGET: RecordingTarget = RecordingTarget::PortSetup { board_b: false, slot: 0, of: 6 };

    fn prompt<'a>(
        placed: &'a [(OrderCardId, CardLocation)],
        numbering: Numbering<'a>,
    ) -> CardPrompt<'a> {
        CardPrompt { candidates: None, placed, numbering }
    }

    /// **The grammar matrix.** Every accepted spelling and every refusal shape,
    /// in one table, at BOTH numbering states — the table is the specification
    /// of §0.2's "one card-entry grammar everywhere", and a case that is not in
    /// it is a case no prompt promises.
    #[test]
    fn the_grammar_matrix_holds_at_both_numbering_states() {
        let rules = Rules::standard();
        let index = build_order_index(&rules, &[], Some(TARGET), None);
        // The index numbers cards in `Country::ALL` order, so selection 1 is
        // the first British card rather than O01 — which is precisely why a
        // selection must never be read as an id.
        let first_shown = index.select(NUMBER_BASE).expect("a first card");

        let cases: &[(&str, CardEntry, CardEntry)] = &[
            // (input, expected with Ids, expected with the index open)
            ("", CardEntry::Cancel, CardEntry::Cancel),
            ("cancel", CardEntry::Cancel, CardEntry::Cancel),
            ("back", CardEntry::Cancel, CardEntry::Cancel),
            ("?", CardEntry::OpenIndex, CardEntry::OpenIndex),
            ("index", CardEntry::OpenIndex, CardEntry::OpenIndex),
            (
                "O14",
                CardEntry::Card(OrderCardId(14)),
                CardEntry::Card(OrderCardId(14)),
            ),
            (
                "o14",
                CardEntry::Card(OrderCardId(14)),
                CardEntry::Card(OrderCardId(14)),
            ),
            (
                "1",
                CardEntry::Card(OrderCardId(1)),
                CardEntry::Card(first_shown),
            ),
            (
                "f silk",
                CardEntry::Filter(Some(IndexFilter::Good(CardGood::Silk))),
                CardEntry::Filter(Some(IndexFilter::Good(CardGood::Silk))),
            ),
            (
                "f 8vp",
                CardEntry::Filter(Some(IndexFilter::MinVp(8))),
                CardEntry::Filter(Some(IndexFilter::MinVp(8))),
            ),
            (
                "f france",
                CardEntry::Filter(Some(IndexFilter::Country(Country::French))),
                CardEntry::Filter(Some(IndexFilter::Country(Country::French))),
            ),
            ("f", CardEntry::Filter(None), CardEntry::Filter(None)),
            (
                "france",
                CardEntry::Filter(Some(IndexFilter::Country(Country::French))),
                CardEntry::Filter(Some(IndexFilter::Country(Country::French))),
            ),
        ];

        for (input, want_ids, want_selection) in cases {
            assert_eq!(
                &resolve(&prompt(&[], Numbering::Ids), input, &rules),
                want_ids,
                "{input:?} at an id prompt",
            );
            assert_eq!(
                &resolve(&prompt(&[], Numbering::Selection(&index)), input, &rules),
                want_selection,
                "{input:?} with the index open",
            );
        }
    }

    /// The refusals, which are the half of the grammar F6 is about: each one
    /// says what to do next, and a duplicate says where the card already is.
    #[test]
    fn every_refusal_names_a_way_forward() {
        let rules = Rules::standard();
        let placed = [(OrderCardId(4), CardLocation::Deal { seat: PlayerId::new(0) })];
        let index = build_order_index(&rules, &placed, Some(TARGET), None);

        let duplicate = resolve(&prompt(&placed, Numbering::Ids), "O04", &rules);
        assert_eq!(
            duplicate,
            CardEntry::Refused("O04 is Seat 0's deal — a card cannot be in two places.".to_string()),
            "a duplicate refusal names the location (F6), not just the fact",
        );
        // The same card, reached through the index by its number, is refused
        // the SAME way -- the two paths share one rule rather than two.
        let number = index
            .cards()
            .find(|c| c.id == OrderCardId(4))
            .and_then(|c| c.selection)
            .expect("O04 is on the screen");
        assert_eq!(
            resolve(&prompt(&placed, Numbering::Selection(&index)), &number.to_string(), &rules),
            duplicate,
        );

        let junk = resolve(&prompt(&[], Numbering::Ids), "wat", &rules);
        assert!(
            matches!(&junk, CardEntry::Refused(m) if m.contains("'?' for the index")),
            "{junk:?}",
        );

        let out_of_range = resolve(&prompt(&[], Numbering::Selection(&index)), "99", &rules);
        assert!(
            matches!(&out_of_range, CardEntry::Refused(m) if m.contains("1-36")),
            "an out-of-range selection states the range on THIS screen: {out_of_range:?}",
        );

        let not_a_candidate =
            CardPrompt { candidates: Some(&[OrderCardId(7)]), placed: &[], numbering: Numbering::Ids };
        assert!(
            matches!(
                resolve(&not_a_candidate, "O08", &rules),
                CardEntry::Refused(m) if m.contains("cannot take") || m.contains("can take"),
            ),
            "a card outside the pool is refused as such",
        );
    }

    /// **One prefix discipline for goods and flags alike** (T5 review S2). The
    /// countries always refused an ambiguous prefix; the goods table resolved
    /// one by list order, so `f s` filtered to seafood — a confident wrong
    /// screen, and the worst shape a refusal can take, because the player has no
    /// reason to doubt it. Now both name what the term could have meant.
    #[test]
    fn a_prefix_that_names_two_things_is_refused_not_guessed() {
        let rules = Rules::standard();
        let refusal = parse_filter("s", &rules).expect_err("'s' names two goods");
        assert!(refusal.contains("seafood") && refusal.contains("silk"), "{refusal}");
        assert!(refusal.contains("type more of the word"), "the way forward: {refusal}");

        // Through the whole grammar, not just the parser: `f s` reaches the
        // player as a refusal rather than as a filter.
        let index = build_order_index(&rules, &[], Some(TARGET), None);
        for numbering in [Numbering::Ids, Numbering::Selection(&index)] {
            assert_eq!(
                resolve(&prompt(&[], numbering), "f s", &rules),
                CardEntry::Refused(refusal.clone()),
            );
        }

        // Every good is still reachable — by its whole word, and by the
        // shortest prefix that names it alone.
        for (term, good) in [
            ("seafood", CardGood::Seafood),
            ("se", CardGood::Seafood),
            ("fish", CardGood::Seafood),
            ("tea", CardGood::Tea),
            ("t", CardGood::Tea),
            ("silk", CardGood::Silk),
            ("si", CardGood::Silk),
            ("copper", CardGood::Copper),
            ("c", CardGood::Copper),
        ] {
            assert_eq!(
                parse_filter(term, &rules),
                Ok(Some(IndexFilter::Good(good))),
                "{term:?} names one good",
            );
        }
    }

    /// **An unrecognized filter term is a refusal, not a silent clear** (T5
    /// review notes). `f 8` used to wipe the filter the player was reading and
    /// look, from the screen, like it had filtered by something — the one
    /// outcome a prompt must never produce, because nothing on screen says it
    /// went wrong. `f` alone still clears, which is what the help line teaches.
    #[test]
    fn an_unrecognized_filter_term_is_refused_and_f_alone_still_clears() {
        let rules = Rules::standard();
        for term in ["8", "wat", "xvp", "zz"] {
            let refusal = parse_filter(term, &rules)
                .expect_err(&format!("{term:?} must be refused, not applied or cleared"));
            assert!(
                refusal.contains(&format!("'{term}'")) && refusal.contains("is not a good, flag"),
                "{term:?}: {refusal}",
            );
            assert!(refusal.contains("'f' to clear"), "the way to clear is named: {refusal}");
        }
        // ...and the term that DOES clear still clears, through the grammar.
        assert_eq!(parse_filter("", &rules), Ok(None));
        assert_eq!(resolve(&prompt(&[], Numbering::Ids), "f", &rules), CardEntry::Filter(None));
        assert!(matches!(
            resolve(&prompt(&[], Numbering::Ids), "f 8", &rules),
            CardEntry::Refused(m) if m.contains("is not a good, flag"),
        ));
    }

    /// The filter keyword ignores case like every other clause of the grammar
    /// (T5 review S1) — `?`, `cancel` and `O14` always did.
    #[test]
    fn the_filter_keyword_is_case_insensitive() {
        let rules = Rules::standard();
        let want = CardEntry::Filter(Some(IndexFilter::Good(CardGood::Silk)));
        for line in ["f silk", "F silk", "filter silk", "Filter silk", "FILTER SILK", "F SILK"] {
            assert_eq!(&resolve(&prompt(&[], Numbering::Ids), line, &rules), &want, "{line:?}");
        }
        for line in ["f", "F", "filter", "FILTER"] {
            assert_eq!(
                &resolve(&prompt(&[], Numbering::Ids), line, &rules),
                &CardEntry::Filter(None),
                "{line:?} clears",
            );
        }
        // Not a false positive on a word that merely starts with the keyword:
        // `france` is a flag, not `f` + `rance`.
        assert_eq!(
            resolve(&prompt(&[], Numbering::Ids), "france", &rules),
            CardEntry::Filter(Some(IndexFilter::Country(Country::French))),
        );
    }

    /// A country prefix has to be unambiguous from its first letter, or `f b`
    /// would silently pick one of two flags. Pinned rather than assumed —
    /// `country_name` is editable and a future rename could collide.
    #[test]
    fn country_prefixes_are_unique_from_one_letter() {
        let firsts: Vec<char> = Country::ALL
            .into_iter()
            .map(|c| describe::country_name(c).chars().next().expect("a name"))
            .collect();
        for c in Country::ALL {
            let first = describe::country_name(c).chars().next().expect("a name");
            assert_eq!(
                country_prefix(&first.to_string()),
                Some(c),
                "{first:?} must name exactly one country; firsts are {firsts:?}",
            );
        }
    }

    /// The echo is the whole card, not a line about it — the difference
    /// between "you typed O14" and "you recorded THIS card".
    #[test]
    fn the_echo_draws_the_whole_card() {
        let rules = Rules::standard();
        let text = echo(OrderCardId(14), &rules);
        assert_eq!(text.lines().count(), crate::card_render::CARD_H);
        assert!(text.contains("O14"), "{text}");
        assert!(text.contains("GERMANY"), "the flag a player would recognize: {text}");
    }

    /// An id for a card the registry does not hold still resolves to a CARD —
    /// the pool check is the caller's, and this module refusing it here would
    /// be the second opinion the module doc rules out.
    #[test]
    fn an_unknown_id_is_the_callers_business() {
        let rules = Rules::standard();
        assert_eq!(
            resolve(&prompt(&[], Numbering::Ids), "O99", &rules),
            CardEntry::Card(OrderCardId(99)),
        );
    }
}
