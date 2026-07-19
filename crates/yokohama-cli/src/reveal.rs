//! Live-mode reveal choreography extracted from `repl.rs` (Task 10, design
//! docs/plans/2026-07-18-phase2-cli-driver-design.md): the `Pending` state
//! `Repl::commit` parks a move in while the operator answers reveal
//! prompts, the prompt/collection loop, and the post-apply desync guard.

use yokohama_core::*;

use crate::describe;
use crate::input::{parse_order_id, parse_tech_entry, TechAmbiguity};
use crate::repl::{Repl, StepOutput};

/// Live-mode reveal-collection state: while a group is being collected,
/// `step` routes input to `collect_reveal` instead of normal command
/// dispatch (see `step`'s pending check) and `out` stops appending the
/// normal legal-options prompt (see `out`'s pending branch). `mv` is the
/// move already confirmed legal against a scratch-applied post-reveal
/// state (`commit`'s `apply_move` call) -- nothing is recorded until the
/// group completes and runs through `Session::apply` for real. Always
/// `None` in `TrackingMode::Simulation`.
pub(crate) enum Pending {
    None,
    /// A Port take refilled slot(s) from the order deck. `slots` holds the
    /// refilled indices on whichever board `board_b` selects, sorted
    /// DESCENDING (draw order -- `refill_port` fills rightmost-empty-first,
    /// popping the deck's top card each time). `collected` grows by one
    /// answer per `collect_reveal` call; the group is complete once its
    /// length reaches `slots.len()`.
    OrderReveals { mv: Move, slots: Vec<usize>, board_b: bool, collected: Vec<OrderCardId> },
    /// A Laboratory action refilled the Lab's one vacated slot from the
    /// tech deck (only one tech ever leaves the Lab per action -- see
    /// `remove_and_slide_lab`'s doc comment -- so there's no slot list to
    /// walk, unlike `OrderReveals`).
    TechReveal { mv: Move, board_b: bool },
    /// `Move::Take5PowerToken` just claimed the (always face-down until
    /// taken) token at `area` -- see `claimed_token_area`.
    TokenReveal { mv: Move, area: AreaId },
}

impl Repl {
    /// The complete message for whatever reveal is currently pending: what
    /// changed and exactly what to type (or an empty string when nothing
    /// is pending). Slots are always shown 1-based; Port slots also carry
    /// their zone, matching `render::ports_view`'s labeling. Prompts walk
    /// `OrderReveals`' `slots` in the order they're stored (DESCENDING --
    /// `commit` sorted them that way), one per `collected.len()`.
    pub(crate) fn reveal_prompt(&self) -> String {
        match &self.pending {
            Pending::None => String::new(),
            Pending::OrderReveals { slots, board_b, collected, .. } => {
                let board_name = if *board_b { "Port B" } else { "Port A" };
                let next_slot = slots[collected.len()];
                let zone = match PortBoard::zone_of(next_slot) {
                    PortZone::Low => "Low",
                    PortZone::Mid => "Mid",
                    PortZone::High => "High",
                };
                format!(
                    "LIVE: {board_name} refilled {} slot(s) from the order deck -- reveal {} of {}. \
                     What card is now in slot {} ({zone})? Enter as 'O<id>' or '<id>' (or 'cancel'):\n",
                    slots.len(), collected.len() + 1, slots.len(), next_slot + 1,
                )
            }
            Pending::TechReveal { board_b, .. } => {
                let board_name = if *board_b { "Laboratory B" } else { "Laboratory A" };
                format!(
                    "LIVE: {board_name} refilled from the tech deck. \
                     What tech card is now there? Enter as 'T<id>' or a name (or 'cancel'):\n"
                )
            }
            Pending::TokenReveal { area, .. } => format!(
                "LIVE: 5-Power token claimed at {}. What was it? \
                 Enter i=Imported c=Copper s=Silk t=Tea f=Seafood y=Yen (or 'cancel'):\n",
                describe::area_label(self.session.state(), *area),
            ),
        }
    }

    /// Feed one line of input into the in-progress reveal collection.
    /// Extracts `self.pending` (leaving `Pending::None` behind) so each arm
    /// owns its fields outright; every non-terminal path restores
    /// `self.pending` before returning. Never panics -- each parser rejects
    /// malformed/unknown input as an `Err` re-prompt rather than unwrapping.
    pub(crate) fn collect_reveal(&mut self, words: &[&str]) -> StepOutput {
        match std::mem::replace(&mut self.pending, Pending::None) {
            // Unreachable in practice: `step` only calls this when
            // `self.pending` just matched `!= Pending::None`. Handled
            // gracefully (not `unreachable!()`) rather than risk a panic.
            Pending::None => self.out(String::new(), false),
            Pending::OrderReveals { mv, slots, board_b, mut collected } => {
                match parse_order_reveal(words, &self.session.state().order_deck, &collected) {
                    Ok(card) => {
                        collected.push(card);
                        if collected.len() == slots.len() {
                            let (out, was_illegal) =
                                self.finish_apply_ex(mv, vec![RecordedEvent::RevealOrders(collected.clone())]);
                            self.lock_on_order_desync(&slots, &collected, board_b, out, was_illegal)
                        } else {
                            self.pending = Pending::OrderReveals { mv, slots, board_b, collected };
                            self.out(self.reveal_prompt(), false)
                        }
                    }
                    Err(msg) => {
                        self.pending = Pending::OrderReveals { mv, slots, board_b, collected };
                        self.out(format!("{msg}\n{}", self.reveal_prompt()), false)
                    }
                }
            }
            Pending::TechReveal { mv, board_b } => {
                match parse_tech_reveal(words, &self.session.state().tech_deck, &self.session.rules().techs) {
                    Ok(card) => {
                        let (out, was_illegal) =
                            self.finish_apply_ex(mv, vec![RecordedEvent::RevealTechs(vec![card])]);
                        self.lock_on_tech_desync(card, board_b, out, was_illegal)
                    }
                    Err(msg) => {
                        self.pending = Pending::TechReveal { mv, board_b };
                        self.out(format!("{msg}\n{}", self.reveal_prompt()), false)
                    }
                }
            }
            Pending::TokenReveal { mv, area } => match parse_token_letter(words) {
                Ok(effect) => self.finish_apply(mv, vec![RecordedEvent::RevealFivePowerToken { area, effect }]),
                Err(msg) => {
                    self.pending = Pending::TokenReveal { mv, area };
                    self.out(format!("{msg}\n{}", self.reveal_prompt()), false)
                }
            },
        }
    }

    /// Post-apply desync assertion (design 2026-07-18 §5.2): once a
    /// completed order-reveal group actually applies, the named board
    /// slots must hold exactly the typed cards. A mismatch can only mean a
    /// driver/engine bug -- deck arrangement and board refill are both
    /// deterministic given the reveal -- so it's treated like an invariant
    /// violation: lock the session and surface a CRITICAL banner rather
    /// than silently tracking a board that disagrees with the table.
    /// No-ops when `out.quit`/`self.locked`/`was_illegal` -- nothing to
    /// re-check against a move that never actually applied. `was_illegal`
    /// (Task 13) is `finish_apply_ex`'s report of whether the apply outcome
    /// was specifically `ApplyError::Illegal`, keying this skip on the
    /// error's TYPE rather than sniffing `out.text` for the substring
    /// "Illegal:" as a prior version did.
    pub(crate) fn lock_on_order_desync(
        &mut self,
        slots: &[usize],
        collected: &[OrderCardId],
        board_b: bool,
        out: StepOutput,
        was_illegal: bool,
    ) -> StepOutput {
        if out.quit || self.locked || was_illegal {
            return out;
        }
        let state = self.session.state();
        let board = if board_b { state.port_state.port_b.as_ref() } else { Some(&state.port_state.port_a) };
        let matches_board = board.is_some_and(|b| {
            slots.iter().zip(collected.iter()).all(|(&slot, &card)| b.slots.get(slot) == Some(&Some(card)))
        });
        if matches_board {
            return out;
        }
        self.locked = true;
        StepOutput {
            text: format!(
                "{}CRITICAL: desync detected -- the revealed card(s) do not match the board after applying the move. Session locked.\n{}",
                out.text,
                self.lockdown_status(),
            ),
            quit: false,
        }
    }

    /// Tech-reveal (Laboratory) counterpart to `lock_on_order_desync` (Task
    /// 13 test (d)): once a completed tech-reveal group actually applies,
    /// the refilled Laboratory slot must hold exactly the typed `card`.
    /// Same skip conditions and CRITICAL-banner shape, applied to the one
    /// Lab slot that changed instead of a list of Port slots (a Laboratory
    /// action only ever refills one slot -- see `remove_and_slide_lab`'s
    /// doc comment).
    pub(crate) fn lock_on_tech_desync(
        &mut self,
        card: TechCardId,
        board_b: bool,
        out: StepOutput,
        was_illegal: bool,
    ) -> StepOutput {
        if out.quit || self.locked || was_illegal {
            return out;
        }
        let state = self.session.state();
        let board = if board_b { state.lab_state.lab_b.as_ref() } else { Some(&state.lab_state.lab_a) };
        let matches_board = board.is_some_and(|b| b.slots.contains(&Some(card)));
        if matches_board {
            return out;
        }
        self.locked = true;
        StepOutput {
            text: format!(
                "{}CRITICAL: desync detected -- the revealed card(s) do not match the board after applying the move. Session locked.\n{}",
                out.text,
                self.lockdown_status(),
            ),
            quit: false,
        }
    }
}

/// Which Port board changed, and which of ITS slots now hold a card that
/// wasn't in `before.order_deck` a moment ago -- an identity-independent
/// test that survives Port's full-compaction refill (a 2-card take can
/// vacate non-contiguous slots; `execute_port_action`'s left-compaction can
/// then reshuffle which physical slot each surviving card ends up in, so
/// only "did this card just come from the deck" reliably picks out the
/// refilled slot(s), not "did this slot's card change"). Sorted
/// DESCENDING -- draw order, since `refill_port` fills rightmost-empty-first
/// from the deck's top (`Vec::pop`).
pub(crate) fn refilled_port_slots(before: &GameState, after: &GameState) -> (Vec<usize>, bool) {
    let board_b = match (&before.port_state.port_b, &after.port_state.port_b) {
        (Some(b0), Some(b1)) => b0.slots != b1.slots,
        _ => false,
    };
    let after_board = if board_b { after.port_state.port_b.as_ref() } else { Some(&after.port_state.port_a) };
    let mut slots: Vec<usize> = after_board
        .map(|board| (0..6).filter(|&i| board.slots[i].is_some_and(|c| before.order_deck.contains(&c))).collect())
        .unwrap_or_default();
    slots.sort_unstable_by(|a, b| b.cmp(a));
    (slots, board_b)
}

/// Whether Lab B (rather than Lab A) is the board that just refilled.
pub(crate) fn lab_b_changed(before: &GameState, after: &GameState) -> bool {
    match (&before.lab_state.lab_b, &after.lab_state.lab_b) {
        (Some(b0), Some(b1)) => b0.slots != b1.slots,
        _ => false,
    }
}

/// The area whose 5-Power token just went from unclaimed to claimed.
/// `Move::Take5PowerToken` is only ever legal when the President's area
/// holds one (`generate_five_power_moves`), so this is `Some` at every real
/// call site; `None` is a defensive fallback that's never expected to fire.
pub(crate) fn claimed_token_area(before: &GameState, after: &GameState) -> Option<AreaId> {
    before.board_layout.five_power_tokens.iter().find_map(|(area, effect)| {
        (effect.is_some() && after.board_layout.five_power_tokens.get(area) == Some(&None)).then_some(*area)
    })
}

/// Parse one order-reveal answer: a real numeric id, currently in `deck`,
/// and not already in `collected` (a card can't be drawn twice in one refill).
pub(crate) fn parse_order_reveal(words: &[&str], deck: &[OrderCardId], collected: &[OrderCardId]) -> Result<OrderCardId, String> {
    if words.len() != 1 {
        return Err("Enter exactly one order-card code (e.g. 'O17' or '17'), or 'cancel'.".to_string());
    }
    let n = match parse_order_id(words[0]) {
        Some(n) => n,
        None => return Err(format!("'{}' is not a valid order-card code.", words[0])),
    };
    let card = OrderCardId(n);
    if collected.contains(&card) {
        return Err(format!("O{n:02} was already recorded for this reveal."));
    }
    if !deck.contains(&card) {
        return Err(format!("O{n:02} is not currently in the order deck."));
    }
    Ok(card)
}

/// Parse one tech-reveal answer: `T<id>` (validated against `deck`) or a
/// case-insensitive name prefix, resolved to the LOWEST matching id
/// currently in `deck` (some titles circulate as 2 physical copies) --
/// thin wrapper over the shared `parse_tech_entry` (design 2026-07-18
/// §5.5 riding item 4), which is why an ambiguous name here resolves
/// (`TechAmbiguity::LowestId`) rather than being rejected the way the
/// setup wizard's `parse_tech_entry` call does.
pub(crate) fn parse_tech_reveal(words: &[&str], deck: &[TechCardId], techs: &TechRegistry) -> Result<TechCardId, String> {
    if words.len() != 1 {
        return Err("Enter exactly one tech code (e.g. 'T05' or a name), or 'cancel'.".to_string());
    }
    parse_tech_entry(words[0], techs, deck, TechAmbiguity::LowestId)
}

/// Parse a 5-Power token-effect letter into the canonical pool's fixed
/// magnitude for that kind -- one letter per `FivePowerEffect` variant (the
/// phantom `VictoryPoints`/`Movement` variants were removed once the
/// physical box check confirmed only these six faces exist).
pub(crate) fn parse_token_letter(words: &[&str]) -> Result<FivePowerEffect, String> {
    let usage = "Enter one letter: i/c/s/t/f/y (or 'cancel').".to_string();
    if words.len() != 1 {
        return Err(usage);
    }
    let mut chars = words[0].chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return Err(usage);
    };
    match c.to_ascii_lowercase() {
        'i' => Ok(FivePowerEffect::ImportedGoods(1)),
        'c' => Ok(FivePowerEffect::Copper(2)),
        's' => Ok(FivePowerEffect::Silk(2)),
        't' => Ok(FivePowerEffect::Tea(3)),
        'f' => Ok(FivePowerEffect::Seafood(3)),
        'y' => Ok(FivePowerEffect::Money(3)),
        _ => Err(format!("'{}' is not a valid token letter (i/c/s/t/f/y).", words[0])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;

    fn live_repl() -> (Repl, Vec<OrderCardId>) {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let table = new_game(config.clone(), &rules, 900).unwrap();
        let obs = SetupObservations {
            port_a: Some(std::array::from_fn(|i| table.state.port_state.port_a.slots[i].unwrap())),
            lab_a: Some(std::array::from_fn(|i| table.state.lab_state.lab_a.slots[i].unwrap())),
            deals: vec![(PlayerId::new(0), table.deals[0])],
            ..Default::default()
        };
        let session = Session::new_live(
            config, 901, obs,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown],
            rules,
        ).unwrap();
        let deck_bottom: Vec<OrderCardId> = session.state().order_deck.iter().take(2).copied().collect();
        (Repl::new(session), deck_bottom)
    }

    #[test]
    fn live_port_take_collects_reveals_then_applies_them() {
        let (mut r, others) = live_repl();
        // Walk to AreaAction with the President on Port A: end additionals,
        // then place 1 assistant AT Port A (the brief's literal "place
        // nothing" script leaves `PlacePresident` with zero legal
        // destinations -- Option A only offers a "Possible Action Area",
        // which per movement.rs's `is_possible_action_area_full` requires
        // the player's own assistant already there; a bare "a" places none
        // anywhere, so `port_area` can never appear regardless of seed.
        // Placing 1 assistant there instead is free on turn 1 -- no
        // president has moved yet, so `calculate_placement_cost` finds no
        // other president at the area to charge a fee for), then place the
        // President directly at Port A via the numbered list.
        let port_area = *r.session.state().board_layout.areas.iter()
            .find(|(_, c)| c.area_type == AreaType::PortA).unwrap().0;
        r.step("0");        // EndAdditionalPhase
        r.step(&format!("a {}", port_area.0)); // place 1 assistant at Port A
        let k = r.session.legal().iter().position(|m| matches!(
            m, Move::PlacePresident { destination } if *destination == port_area
        )).expect("PlacePresident at Port A is legal from hand at game start");
        r.step(&k.to_string());
        // Pick a 1-card port take.
        let take = r.session.legal().iter().position(|m| matches!(
            m, Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } } if cards_taken.len() == 1
        )).expect("1-card port take available");
        let out = r.step(&take.to_string());
        // Live mode must now be ASKING for the refill card, not applied yet.
        assert!(out.text.to_lowercase().contains("slot"), "{}", out.text);
        let events_before = r.session.record().events.len();
        // Answer with a DIFFERENT card than the seeded top: bottom-of-deck card.
        let out = r.step(&format!("O{}", others[0].0));
        assert!(!out.text.contains("CRITICAL"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 2); // RevealOrders + Move
        // The typed card is now physically on the Port A board.
        assert!(r.session.state().port_state.port_a.slots.iter().any(|s| *s == Some(others[0])));
        assert_eq!(replay(r.session.record(), r.session.rules()).unwrap(), *r.session.state());
    }

    /// Task 13 test (c), second half: `lock_on_order_desync` must skip its
    /// check because the outcome's TYPE was `ApplyError::Illegal`, not
    /// because the formatted text happened to contain the substring
    /// "Illegal:" -- proven directly by handing it a `StepOutput` whose text
    /// contains no such substring at all, alongside `was_illegal = true`.
    /// A pre-Task-13 string-sniffing check would fail to skip here (no
    /// match) and go on to report a false-positive desync lock against a
    /// board that was deliberately never checked for realism.
    #[test]
    fn lock_on_order_desync_skips_on_illegal_type_not_string_content() {
        let (mut r, _others) = live_repl();
        let out = StepOutput { text: "some unrelated message\n".to_string(), quit: false };
        let out = r.lock_on_order_desync(&[0], &[OrderCardId(1)], false, out, true);
        assert!(!r.locked, "must skip the desync check when was_illegal is true, regardless of text content");
        assert_eq!(out.text, "some unrelated message\n", "output must pass through unchanged when skipped");
    }

    /// Task 13 test (d): the tech-reveal (Laboratory) counterpart to the
    /// order-reveal (Port) desync guard. Real live play can never actually
    /// produce a mismatch here (the operator's own answer is what gets
    /// arranged onto the deck, then immediately drawn -- see
    /// `lock_on_order_desync`'s doc comment on this being a driver/engine-bug
    /// tripwire, not a reachable-in-correct-play state), so this proves the
    /// GUARD FUNCTION itself locks on a mismatch by calling it directly with
    /// a card that was deliberately never placed on the board.
    #[test]
    fn lab_refill_reveal_mismatch_locks_the_session() {
        let (mut r, _others) = live_repl();
        let wrong_card = TechCardId(200); // never dealt, never on any board
        let out = StepOutput { text: String::new(), quit: false };
        let out = r.lock_on_tech_desync(wrong_card, false, out, false);
        assert!(r.locked, "a typed card absent from the board must lock the session");
        assert!(out.text.contains("CRITICAL"), "{}", out.text);
    }

    /// `lock_on_tech_desync`'s no-op path, mirroring
    /// `lock_on_order_desync_skips_on_illegal_type_not_string_content`.
    #[test]
    fn lock_on_tech_desync_skips_on_illegal_type() {
        let (mut r, _others) = live_repl();
        let out = StepOutput { text: "some unrelated message\n".to_string(), quit: false };
        let out = r.lock_on_tech_desync(TechCardId(200), false, out, true);
        assert!(!r.locked);
        assert_eq!(out.text, "some unrelated message\n");
    }

    #[test]
    fn undo_after_live_reveal_group_removes_the_whole_group() {
        let (mut r, others) = live_repl();
        // See live_port_take_collects_reveals_then_applies_them's comment:
        // an assistant must land at Port A before PlacePresident can target it.
        let port_area = *r.session.state().board_layout.areas.iter()
            .find(|(_, c)| c.area_type == AreaType::PortA).unwrap().0;
        r.step("0");
        r.step(&format!("a {}", port_area.0));
        let k = r.session.legal().iter().position(|m| matches!(
            m, Move::PlacePresident { destination } if *destination == port_area
        )).unwrap();
        r.step(&k.to_string());
        let before = r.session.state().clone();
        let events_before = r.session.record().events.len();
        let take = r.session.legal().iter().position(|m| matches!(
            m, Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } } if cards_taken.len() == 1
        )).unwrap();
        r.step(&take.to_string());
        r.step(&format!("O{}", others[0].0));
        r.step("undo");
        assert_eq!(*r.session.state(), before);
        assert_eq!(r.session.record().events.len(), events_before);
    }

    /// Not one of the brief's pinned Step-1 tests, but added to exercise the
    /// `TechReveal` arm the pinned tests never reach (they only drive a Port
    /// take, so `OrderReveals` is the only variant proven end-to-end by the
    /// brief itself) -- same shape as `live_port_take_...`, mirrored onto
    /// Laboratory A instead of Port A.
    #[test]
    fn live_lab_action_collects_tech_reveal_then_applies_it() {
        let (mut r, _others) = live_repl();
        let lab_area = *r.session.state().board_layout.areas.iter()
            .find(|(_, c)| c.area_type == AreaType::LaboratoryA).unwrap().0;
        r.step("0");
        r.step(&format!("a {}", lab_area.0));
        let k = r.session.legal().iter().position(|m| matches!(
            m, Move::PlacePresident { destination } if *destination == lab_area
        )).expect("PlacePresident at Laboratory A is legal from hand at game start");
        r.step(&k.to_string());
        let take = r.session.legal().iter().position(|m| matches!(
            m, Move::ExecuteAreaAction { action: AreaAction::Laboratory { .. } }
        )).expect("a Laboratory acquisition is available");
        let out = r.step(&take.to_string());
        // Live mode must now be ASKING for the refilled tech card, not applied yet.
        assert!(out.text.to_lowercase().contains("tech"), "{}", out.text);
        let events_before = r.session.record().events.len();
        // Answer with the deck's actual next draw (top = last element) --
        // TechReveal carries no desync guard to prove wrong, so this just
        // proves the group applies and the named card lands on the board.
        let top_tech = *r.session.state().tech_deck.last().unwrap();
        let out = r.step(&format!("T{}", top_tech.0));
        assert!(!out.text.contains("Illegal"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 2); // RevealTechs + Move
        assert!(r.session.state().lab_state.lab_a.slots.iter().any(|s| *s == Some(top_tech)));
        assert_eq!(replay(r.session.record(), r.session.rules()).unwrap(), *r.session.state());
    }

    // The three reveal-answer parsers below are pure functions, so their
    // junk-input handling is tested directly rather than through a full
    // Repl/Session (cheaper, and pins the exact rejection/resolution rules
    // rather than just "the REPL didn't crash").

    #[test]
    fn parse_order_reveal_never_panics_on_junk_and_rejects_correctly() {
        let deck = vec![OrderCardId(5), OrderCardId(9)];
        let none: Vec<OrderCardId> = vec![];
        assert!(parse_order_reveal(&[], &deck, &none).is_err()); // empty input line
        assert!(parse_order_reveal(&["O"], &deck, &none).is_err());
        assert!(parse_order_reveal(&["O99999"], &deck, &none).is_err()); // overflows u8
        assert!(parse_order_reveal(&["🎉"], &deck, &none).is_err());
        assert!(parse_order_reveal(&["O1"], &deck, &none).is_err()); // valid code, not in deck
        assert!(parse_order_reveal(&["O5", "O9"], &deck, &none).is_err()); // too many tokens
        assert_eq!(parse_order_reveal(&["O5"], &deck, &none), Ok(OrderCardId(5)));
        assert_eq!(parse_order_reveal(&["9"], &deck, &none), Ok(OrderCardId(9))); // bare digits, no 'O'
        let already = vec![OrderCardId(5)];
        assert!(parse_order_reveal(&["O5"], &deck, &already).is_err(), "a card can't be drawn twice");
    }

    #[test]
    fn parse_tech_reveal_never_panics_on_junk_and_resolves_name_prefix_to_lowest_id() {
        let rules = Rules::standard();
        // TechCardId(11) and (12) are both named "Ball" (data.rs) -- one of
        // the 10 titles that circulate as 2 physical copies.
        let ball_deck = vec![TechCardId(11), TechCardId(12)];
        assert!(parse_tech_reveal(&[], &ball_deck, &rules.techs).is_err()); // empty input line
        assert!(parse_tech_reveal(&["T"], &ball_deck, &rules.techs).is_err()); // no T-named card in THIS deck
        assert!(parse_tech_reveal(&["T99"], &ball_deck, &rules.techs).is_err()); // valid code, not in deck
        assert!(parse_tech_reveal(&["🎉"], &ball_deck, &rules.techs).is_err());
        assert!(parse_tech_reveal(&["Zzzz"], &ball_deck, &rules.techs).is_err()); // no name match
        assert_eq!(parse_tech_reveal(&["ball"], &ball_deck, &rules.techs), Ok(TechCardId(11)), "ambiguous name resolves to the LOWEST id");
        assert_eq!(parse_tech_reveal(&["T12"], &ball_deck, &rules.techs), Ok(TechCardId(12)));
    }

    #[test]
    fn parse_token_letter_never_panics_on_junk_and_maps_every_kind() {
        assert!(parse_token_letter(&[]).is_err());
        assert!(parse_token_letter(&["ix"]).is_err());
        assert!(parse_token_letter(&["🎉"]).is_err());
        assert!(parse_token_letter(&["z"]).is_err());
        assert_eq!(parse_token_letter(&["i"]), Ok(FivePowerEffect::ImportedGoods(1)));
        assert_eq!(parse_token_letter(&["C"]), Ok(FivePowerEffect::Copper(2)), "letters are case-insensitive");
        assert_eq!(parse_token_letter(&["s"]), Ok(FivePowerEffect::Silk(2)));
        assert_eq!(parse_token_letter(&["t"]), Ok(FivePowerEffect::Tea(3)));
        assert_eq!(parse_token_letter(&["f"]), Ok(FivePowerEffect::Seafood(3)));
        assert_eq!(parse_token_letter(&["y"]), Ok(FivePowerEffect::Money(3)));
    }
}
