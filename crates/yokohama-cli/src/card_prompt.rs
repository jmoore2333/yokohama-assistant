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

use yokohama_core::{AchievementCategory, Country, OrderCardId, Rules, TechCardId};

use crate::describe;
use crate::screen_model::{
    menu_index, CardGood, CardLocation, CardRef, IndexFilter, IndexKind, IndexModel, NUMBER_BASE,
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
/// **The `kind` field is T6's**, and T5's doc said exactly where it would come
/// from: every prompt in the driver identified an ORDER card, so a
/// discriminator would have been a field nothing could branch on. Three more
/// registries later it is what decides which one a bare id names and which
/// vocabulary a name is matched against — the alternative being a grammar that
/// guesses, which is the F4 defect this module exists to end.
pub(crate) struct CardPrompt<'a> {
    /// Which registry this prompt is asking about.
    pub kind: IndexKind,
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
    /// Cards already recorded elsewhere, and where (F6). Typed as [`CardRef`]
    /// so one prompt can carry locations from any registry.
    pub placed: &'a [(CardRef, CardLocation)],
    /// How a bare number reads right now.
    pub numbering: Numbering<'a>,
}

impl<'a> CardPrompt<'a> {
    /// An ORDER prompt — the shape every pre-T6 caller wants, spelled once so
    /// those call sites do not each have to name the kind and the empty pool.
    pub fn orders(
        placed: &'a [(CardRef, CardLocation)],
        numbering: Numbering<'a>,
    ) -> CardPrompt<'a> {
        CardPrompt { kind: IndexKind::Orders, candidates: None, placed, numbering }
    }
}

/// What one line of input at a card prompt turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CardEntry {
    /// A card was named. The caller validates it against its own pool, echoes
    /// it ([`echo`]), and commits.
    Card(CardRef),
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
        return match parse_filter(rest, prompt.kind) {
            Ok(filter) => CardEntry::Filter(filter),
            Err(message) => CardEntry::Refused(message),
        };
    }

    // A bare number: a selection while the index is up, an id otherwise.
    if line.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(n) = line.parse::<usize>() {
            if let Numbering::Selection(index) = prompt.numbering {
                return match index.select(n) {
                    Some(card) => named(prompt, card),
                    None => CardEntry::Refused(selection_range_message(index)),
                };
            }
            if let Ok(id) = u8::try_from(n) {
                // Which registry a bare id names is the prompt's business, not
                // a guess — the whole reason `kind` exists.
                match prompt.kind {
                    IndexKind::Orders => return named(prompt, CardRef::Order(OrderCardId(id))),
                    IndexKind::Techs => return named(prompt, CardRef::Tech(TechCardId(id))),
                    IndexKind::Sites => return named(prompt, CardRef::Site(id)),
                    // Achievement cards carry no number of any kind, so a bare
                    // one at an id prompt names nothing. Saying so beats
                    // inventing an ordering nothing is printed with.
                    IndexKind::Achievements => {
                        return CardEntry::Refused(
                            "achievement cards carry no number — press '?' and pick one from the index."
                                .to_string(),
                        )
                    }
                }
            }
        }
        return CardEntry::Refused(format!("'{line}' is not a card number on this screen."));
    }

    // `O14`/`o14`, `T07`/`t7`, `S12`/`s12` — the same id, spelled the way the
    // registry spells it. Accepted whether or not an index is open: a player who
    // knows the id is not made to find its position on a screen.
    if line.len() > 1 {
        if let Some(id) = prefixed_id(line, prompt.kind) {
            return named(prompt, id);
        }
    }

    // A technology is identified by its NAME (nothing is printed on it to
    // identify it by), so a name or any unambiguous prefix of one picks a card.
    if prompt.kind == IndexKind::Techs {
        return match tech_matches(line, rules).as_slice() {
            [only] => named(prompt, CardRef::Tech(free_copy(prompt, *only, rules))),
            [] => CardEntry::Refused(format!(
                "'{line}' is not a technology. Type its name, its id (like 'T07'), or '?' for the index.",
            )),
            many => {
                let names: Vec<&str> = many
                    .iter()
                    .filter_map(|id| rules.techs.get(*id).map(|c| c.name.as_str()))
                    .collect();
                CardEntry::Refused(ambiguous_message(line, &names))
            }
        };
    }

    // An achievement is identified by its CONDITION, and the type name is the
    // nearest thing to a handle for it — the same shape as a technology's, and
    // the same discipline: an unambiguous prefix picks a card, an ambiguous one
    // is refused naming what it could have meant (`prod` is three cards).
    //
    // The name is an ALIAS, never the required form (R8): every prompt that
    // asks for one of these prints the cards with numbers, so a player who has
    // never seen `Prod3Comm2` types a number instead.
    if prompt.kind == IndexKind::Achievements {
        return match achievement_matches(line).as_slice() {
            [only] => named(prompt, CardRef::Achievement(*only)),
            [] => CardEntry::Refused(format!("'{line}' is not an achievement. {}", how_to_name(prompt.kind))),
            many => {
                let names: Vec<String> = many.iter().map(|ty| format!("{ty:?}")).collect();
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                CardEntry::Refused(ambiguous_message(line, &names))
            }
        };
    }

    // A country name or prefix. Orders carry a FLAG rather than a title, so a
    // country never identifies one card — it identifies a group, which is
    // exactly what the index filter is for, so that is what this becomes
    // rather than a refusal the player has to translate.
    if matches!(prompt.kind, IndexKind::Orders) {
        if let Some(country) = country_prefix(line) {
            return CardEntry::Filter(Some(IndexFilter::Country(country)));
        }
    }

    CardEntry::Refused(format!("'{line}' is not a card. {}", how_to_name(prompt.kind)))
}

/// The forms that DO name a card here, per registry — the second half of every
/// refusal, because a refusal that does not say what would have worked leaves
/// the player guessing, which is F4 from the other side.
fn how_to_name(kind: IndexKind) -> &'static str {
    match kind {
        IndexKind::Orders => "Type its number, its id (like 'O14'), or '?' for the index.",
        IndexKind::Techs => "Type its name, its id (like 'T07'), or '?' for the index.",
        IndexKind::Achievements => "Type its number, its name (like 'Copper5'), or '?' for the index.",
        IndexKind::Sites => "Type its number (like 'S12' or '12'), or '?' for the index.",
    }
}

/// `O14` / `T07` / `S12` — a letter-prefixed id in the prompt's own registry.
/// The letters are NOT interchangeable: `T07` at an order prompt names nothing,
/// and answering it as `O07` would be the silent wrong answer this grammar is
/// built to refuse.
fn prefixed_id(line: &str, kind: IndexKind) -> Option<CardRef> {
    let (letter, digits) = line.split_at(1);
    let n: u8 = digits.parse().ok()?;
    match (letter.to_ascii_lowercase().as_str(), kind) {
        ("o", IndexKind::Orders) => Some(CardRef::Order(OrderCardId(n))),
        ("t", IndexKind::Techs) => Some(CardRef::Tech(TechCardId(n))),
        ("s", IndexKind::Sites) => Some(CardRef::Site(n)),
        _ => None,
    }
}

/// Every technology whose NAME `needle` prefixes, deduped by TITLE — the two
/// copies of a title are one answer, not an ambiguity.
fn tech_matches(needle: &str, rules: &Rules) -> Vec<TechCardId> {
    let needle = needle.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut cards: Vec<&yokohama_core::TechCard> = rules.techs.iter().collect();
    cards.sort_by_key(|c| c.id.0);
    let mut hits: Vec<TechCardId> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for card in cards {
        if card.name.to_lowercase().starts_with(&needle) && !names.contains(&card.name) {
            names.push(card.name.clone());
            hits.push(card.id);
        }
    }
    hits
}

/// Every achievement type whose NAME `needle` prefixes, in registry order.
///
/// Over `screen_model::achievement_registry` rather than a second list of the
/// twelve: that one is already the crate's roster (core exposes no
/// `AchievementType::ALL`) and it is pinned to four-per-category, so a name
/// typed here and a card on the index can never be drawn from different sets.
fn achievement_matches(needle: &str) -> Vec<yokohama_core::AchievementType> {
    let needle = needle.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    crate::screen_model::achievement_registry()
        .into_iter()
        .map(|a| a.achievement_type)
        .filter(|ty| format!("{ty:?}").to_lowercase().starts_with(&needle))
        .collect()
}

/// The first copy of `id`'s TITLE that nothing has claimed yet.
///
/// A player naming `telegram` means the card in their hand, not a particular
/// engine id — so when one copy is already recorded, the name has to resolve to
/// the OTHER one or the entry would be refused as a duplicate of itself.
fn free_copy(prompt: &CardPrompt<'_>, id: TechCardId, rules: &Rules) -> TechCardId {
    let Some(name) = rules.techs.get(id).map(|c| c.name.clone()) else {
        return id;
    };
    let mut copies: Vec<TechCardId> =
        rules.techs.iter().filter(|c| c.name == name).map(|c| c.id).collect();
    copies.sort_unstable_by_key(|c| c.0);
    copies
        .iter()
        .copied()
        .find(|c| !prompt.placed.iter().any(|(placed, _)| *placed == CardRef::Tech(*c)))
        .unwrap_or(id)
}

/// A named card, refused if it is already recorded somewhere (F6: the refusal
/// names where) or if it is not a card this prompt can take.
fn named(prompt: &CardPrompt<'_>, card: CardRef) -> CardEntry {
    if let Some((_, at)) = prompt.placed.iter().find(|(placed, _)| *placed == card) {
        return CardEntry::Refused(format!(
            "{} is {} — a card cannot be in two places.",
            card_tag(card),
            describe::card_location(*at),
        ));
    }
    if let CardRef::Order(id) = card {
        if prompt.candidates.is_some_and(|pool| !pool.contains(&id)) {
            return CardEntry::Refused(format!(
                "O{:02} is not one of the cards this prompt can take.",
                id.0,
            ));
        }
    }
    CardEntry::Card(card)
}

/// How a refusal names a card: its id where one exists, its condition where
/// none does.
pub(crate) fn card_tag(card: CardRef) -> String {
    match card {
        CardRef::Order(id) => format!("O{:02}", id.0),
        CardRef::Tech(id) => format!("T{:02}", id.0),
        CardRef::Site(index) => format!("Site {index}"),
        CardRef::Achievement(a) => describe::achievement_desc(a),
    }
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
///
/// **All four registries since T7**, through the same `index_card` the index
/// draws with: a wizard that echoed an order card and said nothing back for a
/// technology would be teaching that one of the two entries had been read and
/// the other merely accepted. `colors` is needed only by an achievement's
/// claimant line and is ignored by everything else.
///
/// An identity no registry holds draws nothing — the caller's own pool check is
/// what refuses it, and there is no card here to show.
pub(crate) fn echo(card: CardRef, rules: &Rules, colors: &[yokohama_core::PlayerColor]) -> String {
    let Some(drawn) = crate::screen_model::index_card_for(card, rules) else {
        return String::new();
    };
    let mut out = String::new();
    for line in crate::card_render::index_card(&drawn, colors) {
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
pub(crate) fn parse_filter(term: &str, kind: IndexKind) -> Result<Option<IndexFilter>, String> {
    // Messages quote what was TYPED; matching is done on the folded form.
    let typed = term.trim();
    let term = typed.to_lowercase();
    if term.is_empty() {
        return Ok(None);
    }
    // **A filter this registry cannot answer is refused, not applied.** The two
    // failure shapes look identical on screen — an empty grid — and only one of
    // them is a real answer; see `IndexFilter::applies_to`.
    let applicable = |f: IndexFilter| -> Result<Option<IndexFilter>, String> {
        if f.applies_to(kind) {
            Ok(Some(f))
        } else {
            Err(inapplicable_message(typed, kind))
        }
    };

    // **Only where a category is a thing there IS**, and the guard is the point:
    // `c` is copper on the order index and category C on the achievement index,
    // and which one it means is decided by the prompt's registry rather than by
    // a precedence order that would silently take `f c` away from copper.
    if kind == IndexKind::Achievements {
        if let Some(category) = category_term(&term) {
            return applicable(IndexFilter::Category(category));
        }
    }
    match good_matches(&term).as_slice() {
        [only] => return applicable(IndexFilter::Good(*only)),
        [] => {}
        many => {
            let names: Vec<&str> = many.iter().map(|g| describe::card_good_name(*g)).collect();
            return Err(ambiguous_message(typed, &names));
        }
    }
    if let Some(digits) = term.strip_suffix("vp").or_else(|| term.strip_suffix(" vp")) {
        if let Ok(n) = digits.trim().parse::<u8>() {
            return applicable(IndexFilter::MinVp(n));
        }
    }
    match country_matches(&term).as_slice() {
        [only] => applicable(IndexFilter::Country(*only)),
        [] => Err(unrecognized_message(typed, kind)),
        many => {
            let names: Vec<&str> = many.iter().map(|c| describe::country_name(*c)).collect();
            Err(ambiguous_message(typed, &names))
        }
    }
}

/// `a`, `b`, `c` — an achievement category, which is the one filter dimension
/// that index has. Single letters only: the categories have no printed names,
/// so there is nothing longer to prefix-match against.
fn category_term(term: &str) -> Option<AchievementCategory> {
    match term {
        "a" => Some(AchievementCategory::A),
        "b" => Some(AchievementCategory::B),
        "c" => Some(AchievementCategory::C),
        _ => None,
    }
}

/// `'silk' is a real filter, but not on this index — …`
fn inapplicable_message(term: &str, kind: IndexKind) -> String {
    format!("'{term}' is not something this index can filter by. {}", filter_forms(kind))
}

/// The filter terms a registry does take.
fn filter_forms(kind: IndexKind) -> &'static str {
    match kind {
        IndexKind::Orders => "Try 'f silk', 'f 8vp', 'f france', or 'f' to clear.",
        IndexKind::Techs => "Try 'f france', or 'f' to clear.",
        IndexKind::Achievements => "Try 'f a', 'f b', 'f c', or 'f' to clear.",
        IndexKind::Sites => "This index takes no filters; 'f' clears.",
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

/// `'8' is not a good, flag, or '<n>vp' — …`, with the forms that work here.
fn unrecognized_message(term: &str, kind: IndexKind) -> String {
    format!("'{term}' is not a good, flag, or '<n>vp' — {}", filter_forms(kind))
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
    use crate::screen_model::{build_order_index, build_tech_index, RecordingTarget};
    use yokohama_core::PlayerId;

    const TARGET: RecordingTarget = RecordingTarget::PortSetup { board_b: false, slot: 0, of: 6 };
    const TECH_TARGET: RecordingTarget = RecordingTarget::TechRefill { board_b: false };

    fn prompt<'a>(
        placed: &'a [(CardRef, CardLocation)],
        numbering: Numbering<'a>,
    ) -> CardPrompt<'a> {
        CardPrompt::orders(placed, numbering)
    }

    /// A TECH prompt over the same shared grammar.
    fn tech_prompt<'a>(
        placed: &'a [(CardRef, CardLocation)],
        numbering: Numbering<'a>,
    ) -> CardPrompt<'a> {
        CardPrompt { kind: IndexKind::Techs, candidates: None, placed, numbering }
    }

    /// The order card an entry names.
    fn order(n: u8) -> CardEntry {
        CardEntry::Card(CardRef::Order(OrderCardId(n)))
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
            ("O14", order(14), order(14)),
            ("o14", order(14), order(14)),
            ("1", order(1), CardEntry::Card(first_shown)),
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
        let placed = [(CardRef::Order(OrderCardId(4)), CardLocation::Deal { seat: PlayerId::new(0) })];
        let index = build_order_index(&rules, &[(OrderCardId(4), CardLocation::Deal { seat: PlayerId::new(0) })], Some(TARGET), None);

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
            .find(|c| c.face.card_ref() == Some(CardRef::Order(OrderCardId(4))))
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

        let not_a_candidate = CardPrompt {
            kind: IndexKind::Orders,
            candidates: Some(&[OrderCardId(7)]),
            placed: &[],
            numbering: Numbering::Ids,
        };
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
        let refusal = parse_filter("s", IndexKind::Orders).expect_err("'s' names two goods");
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
                parse_filter(term, IndexKind::Orders),
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
            let refusal = parse_filter(term, IndexKind::Orders)
                .expect_err(&format!("{term:?} must be refused, not applied or cleared"));
            assert!(
                refusal.contains(&format!("'{term}'")) && refusal.contains("is not a good, flag"),
                "{term:?}: {refusal}",
            );
            assert!(refusal.contains("'f' to clear"), "the way to clear is named: {refusal}");
        }
        // ...and the term that DOES clear still clears, through the grammar.
        assert_eq!(parse_filter("", IndexKind::Orders), Ok(None));
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
    ///
    /// **And the order echo did not move when T7 generalized it to four
    /// registries**: it is still exactly `card_render::order_card`'s plain
    /// face, asserted line for line rather than by shape, because the REPL's
    /// reveal path prints this string and a silently restyled card there would
    /// be a change nobody asked for.
    #[test]
    fn the_echo_draws_the_whole_card() {
        let rules = Rules::standard();
        let text = echo(CardRef::Order(OrderCardId(14)), &rules, &[]);
        assert_eq!(text.lines().count(), crate::card_render::CARD_H);
        assert!(text.contains("O14"), "{text}");
        assert!(text.contains("GERMANY"), "the flag a player would recognize: {text}");

        let direct = crate::card_render::order_card(&crate::card_render::CardFace::new(
            OrderCardId(14),
            rules.orders.get(OrderCardId(14)),
        ));
        assert_eq!(text.lines().collect::<Vec<_>>(), direct, "the shipped order echo is unchanged");
    }

    /// **Every registry echoes** (T7). A wizard that drew the card back for an
    /// order and said nothing for a technology would be telling the operator
    /// that one entry had been read and the other merely swallowed — and the
    /// tech and site cards are the two whose content a person cannot check
    /// against a printed number, so they are the ones the echo is FOR.
    #[test]
    fn every_registry_echoes_its_own_card() {
        let rules = Rules::standard();
        let telegram = rules.techs.iter().find(|c| c.name == "Telegram").expect("a Telegram");
        for (card, needle) in [
            (CardRef::Tech(telegram.id), "TELEGRAM"),
            (CardRef::Site(3), "SITE 3"),
            (CardRef::Achievement(yokohama_core::AchievementType::Copper5), "Own 5 Copper"),
        ] {
            let text = echo(card, &rules, &[]);
            assert!(text.contains(needle), "{card:?} must echo its own card: {text}");
            assert!(!text.contains('['), "an echo carries no selection number: {text}");
        }

        // An identity no registry holds draws nothing rather than an invented
        // card — the caller's pool check is what refuses it.
        assert_eq!(echo(CardRef::Tech(TechCardId(99)), &rules, &[]), "");
        assert_eq!(echo(CardRef::Site(99), &rules, &[]), "");
    }

    /// An achievement name is an ACCEPTED ALIAS at an achievement prompt (T7,
    /// the F1 menu's other door), under the same prefix discipline the goods,
    /// flags and technology titles follow: `prod` names three cards and is
    /// refused rather than guessed.
    #[test]
    fn an_achievement_is_named_by_its_type_name_and_an_ambiguous_prefix_is_refused() {
        let rules = Rules::standard();
        let goals = CardPrompt {
            kind: IndexKind::Achievements,
            candidates: None,
            placed: &[],
            numbering: Numbering::Ids,
        };
        use yokohama_core::AchievementType;
        for (typed, want) in [
            ("Copper5", AchievementType::Copper5),
            ("copper5", AchievementType::Copper5),
            ("money", AchievementType::Money10),
            ("tokens", AchievementType::Tokens5),
        ] {
            assert_eq!(
                resolve(&goals, typed, &rules),
                CardEntry::Card(CardRef::Achievement(want)),
                "{typed:?}",
            );
        }
        let ambiguous = resolve(&goals, "prod", &rules);
        assert!(
            matches!(&ambiguous, CardEntry::Refused(m)
                if m.contains("Production4") && m.contains("Prod3Comm2") && m.contains("type more")),
            "{ambiguous:?}",
        );
        // ...and a name that is nothing at all still says what would work.
        let junk = resolve(&goals, "zzz", &rules);
        assert!(
            matches!(&junk, CardEntry::Refused(m) if m.contains("'?' for the index")),
            "{junk:?}",
        );
    }

    /// **A technology is named by its NAME**, because nothing is printed on the
    /// card to name it by — the sharpest form of F4/F5, and the reason `kind`
    /// exists on this struct. Every one of the twenty titles resolves by its
    /// whole name; an ambiguous prefix is refused rather than guessed (the same
    /// discipline the goods and the flags follow).
    #[test]
    fn a_technology_is_named_by_its_name_and_an_ambiguous_prefix_is_refused() {
        let rules = Rules::standard();
        for card in rules.techs.iter() {
            let by_name = resolve(&tech_prompt(&[], Numbering::Ids), &card.name, &rules);
            // The FIRST copy of the title, which is what "the card in my hand"
            // means when nothing has claimed either copy yet.
            let mut copies: Vec<TechCardId> =
                rules.techs.iter().filter(|c| c.name == card.name).map(|c| c.id).collect();
            copies.sort_unstable_by_key(|c| c.0);
            assert_eq!(
                by_name,
                CardEntry::Card(CardRef::Tech(copies[0])),
                "{:?} must resolve by name",
                card.name,
            );
            // ...and case-insensitively, like every other clause.
            assert_eq!(
                resolve(&tech_prompt(&[], Numbering::Ids), &card.name.to_uppercase(), &rules),
                by_name,
            );
        }

        // `tele` prefixes Telegram and Telephone: refused, naming both.
        let ambiguous = resolve(&tech_prompt(&[], Numbering::Ids), "tele", &rules);
        assert!(
            matches!(&ambiguous, CardEntry::Refused(m)
                if m.contains("Telegram") && m.contains("Telephone") && m.contains("type more")),
            "{ambiguous:?}",
        );
        // One more letter settles it.
        assert!(matches!(
            resolve(&tech_prompt(&[], Numbering::Ids), "telep", &rules),
            CardEntry::Card(CardRef::Tech(_)),
        ));
    }

    /// **A name resolves to the copy nobody has claimed.** A player typing
    /// `telegram` means the card in their hand, not a particular engine id — so
    /// with one copy already in a Lab slot the name has to reach the OTHER one,
    /// or the entry would be refused as a duplicate of itself.
    #[test]
    fn a_tech_name_resolves_past_a_copy_that_is_already_placed() {
        let rules = Rules::standard();
        let mut copies: Vec<TechCardId> =
            rules.techs.iter().filter(|c| c.name == "Telegram").map(|c| c.id).collect();
        copies.sort_unstable_by_key(|c| c.0);
        let placed = [(
            CardRef::Tech(copies[0]),
            CardLocation::LabSlot { board_b: false, slot: 0 },
        )];
        assert_eq!(
            resolve(&tech_prompt(&placed, Numbering::Ids), "telegram", &rules),
            CardEntry::Card(CardRef::Tech(copies[1])),
        );
        // The placed copy is still refused BY LOCATION when named by its id.
        let by_id =
            resolve(&tech_prompt(&placed, Numbering::Ids), &format!("T{:02}", copies[0].0), &rules);
        assert!(
            matches!(&by_id, CardEntry::Refused(m)
                if m.contains("Laboratory A slot 1") && m.contains("two places")),
            "{by_id:?}",
        );
    }

    /// **A bare id means what the PROMPT'S registry says it means**, and the
    /// letter prefixes are not interchangeable: `T07` at an order prompt names
    /// nothing, and answering it as `O07` would be exactly the silent wrong
    /// answer this grammar exists to refuse.
    #[test]
    fn a_bare_id_reads_in_the_prompts_own_registry() {
        let rules = Rules::standard();
        assert_eq!(resolve(&prompt(&[], Numbering::Ids), "7", &rules), order(7));
        assert_eq!(
            resolve(&tech_prompt(&[], Numbering::Ids), "7", &rules),
            CardEntry::Card(CardRef::Tech(TechCardId(7))),
        );
        let site = CardPrompt { kind: IndexKind::Sites, candidates: None, placed: &[], numbering: Numbering::Ids };
        assert_eq!(resolve(&site, "7", &rules), CardEntry::Card(CardRef::Site(7)));
        assert_eq!(resolve(&site, "S07", &rules), CardEntry::Card(CardRef::Site(7)));

        // Cross-registry spellings name nothing rather than the wrong card.
        for (kind, wrong) in [(IndexKind::Orders, "T07"), (IndexKind::Techs, "O07"), (IndexKind::Sites, "O07")] {
            let p = CardPrompt { kind, candidates: None, placed: &[], numbering: Numbering::Ids };
            assert!(
                matches!(resolve(&p, wrong, &rules), CardEntry::Refused(_)),
                "{wrong:?} must not resolve at a {kind:?} prompt",
            );
        }

        // An achievement card carries no number at all, and the refusal says so
        // rather than inventing an ordering nothing is printed with.
        let goals = CardPrompt {
            kind: IndexKind::Achievements,
            candidates: None,
            placed: &[],
            numbering: Numbering::Ids,
        };
        assert!(
            matches!(resolve(&goals, "3", &rules), CardEntry::Refused(m) if m.contains("no number")),
            "an achievement id prompt refuses a bare number with the reason",
        );
    }

    /// **A filter this registry cannot answer is REFUSED, not applied.** The two
    /// failure shapes look identical on screen — an empty grid — and only one of
    /// them is a real answer, so the inapplicable one has to say so and name
    /// what would work here.
    #[test]
    fn a_filter_that_does_not_apply_to_this_registry_is_refused_with_the_forms_that_do() {
        // `silk` is a real order filter and nothing at all on the site index.
        let refusal = parse_filter("silk", IndexKind::Sites)
            .expect_err("the site index cannot filter by a good");
        assert!(refusal.contains("this index can filter by"), "{refusal}");
        assert!(refusal.contains("takes no filters"), "and what it does take: {refusal}");

        // `c` is COPPER at an order prompt and CATEGORY C at an achievement
        // one — decided by the registry, never by a precedence order that would
        // quietly take `f c` away from copper.
        assert_eq!(
            parse_filter("c", IndexKind::Orders),
            Ok(Some(IndexFilter::Good(CardGood::Copper))),
        );
        assert_eq!(
            parse_filter("c", IndexKind::Achievements),
            Ok(Some(IndexFilter::Category(AchievementCategory::C))),
        );
        // A flag filters orders and techs, and nothing else.
        assert_eq!(
            parse_filter("france", IndexKind::Techs),
            Ok(Some(IndexFilter::Country(Country::French))),
        );
        assert!(parse_filter("france", IndexKind::Achievements).is_err());
        // `f` alone still clears, at every registry.
        for kind in IndexKind::ALL {
            assert_eq!(parse_filter("", kind), Ok(None), "{kind:?}");
        }
        // **The parser never hands back a filter its own registry cannot
        // answer.** Stated over the parser's OUTPUT rather than over the term's
        // intent, because a term can legitimately mean different filters at
        // different registries — `a` is category A on the achievement index and
        // AMERICA at an order prompt, and both are right.
        for kind in IndexKind::ALL {
            for term in ["silk", "8vp", "france", "a", "b", "c", "fish", "copper"] {
                if let Ok(Some(filter)) = parse_filter(term, kind) {
                    assert!(
                        filter.applies_to(kind),
                        "{term:?} at {kind:?} parsed to {filter:?}, which does not apply there",
                    );
                }
            }
        }
    }

    /// A selection number typed on a TECH screen resolves through that model,
    /// exactly as an order selection does — the shared rule, at a second
    /// registry.
    #[test]
    fn a_tech_selection_resolves_through_its_own_index_model() {
        let rules = Rules::standard();
        let index = build_tech_index(&rules, &[], Some(TECH_TARGET), None);
        let first = index.select(NUMBER_BASE).expect("a first card");
        assert_eq!(
            resolve(&tech_prompt(&[], Numbering::Selection(&index)), "1", &rules),
            CardEntry::Card(first),
        );
        assert!(matches!(first, CardRef::Tech(_)), "a tech screen selects techs: {first:?}");
        let out_of_range = resolve(&tech_prompt(&[], Numbering::Selection(&index)), "99", &rules);
        assert!(
            matches!(&out_of_range, CardEntry::Refused(m) if m.contains("1-20")),
            "{out_of_range:?}",
        );
    }

    /// An id for a card the registry does not hold still resolves to a CARD —
    /// the pool check is the caller's, and this module refusing it here would
    /// be the second opinion the module doc rules out.
    #[test]
    fn an_unknown_id_is_the_callers_business() {
        let rules = Rules::standard();
        assert_eq!(
            resolve(&prompt(&[], Numbering::Ids), "O99", &rules),
            order(99),
        );
    }
}
