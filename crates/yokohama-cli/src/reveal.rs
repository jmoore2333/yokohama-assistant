//! Live-mode reveal choreography extracted from `repl.rs` (Task 10, design
//! docs/plans/2026-07-18-phase2-cli-driver-design.md): the `Pending` state
//! `Repl::commit` parks a move in while the operator answers reveal
//! prompts, the prompt/collection loop, and the post-apply desync guard.
//!
//! **Phase-3.5a T8 changed the SURFACE and nothing underneath it.** The prompt
//! is now the composed reveal zone ([`crate::decision_render::reveal_zone`],
//! from a pure [`RevealModel`]) and the answer goes through the crate's one card
//! grammar ([`crate::card_prompt::resolve`]) instead of a parser of this
//! module's own — so a bare number, `O14`, a technology's NAME, `?` and a filter
//! mean here exactly what they mean at every other card prompt in the driver
//! (spec §0.2, finding F4). What did NOT move: the group is still collected
//! answer by answer with nothing recorded until it completes; the completed
//! group still commits as ONE `Session::apply` (move + reveals, atomic); the
//! reveal events still precede the move in the record; and the post-apply desync
//! guards below are untouched. No record event kind was added, removed or
//! reordered.
//!
//! **The split this module keeps: the grammar identifies, the CALLER validates.**
//! `card_prompt` knows how a card is named and where a card already is; it knows
//! nothing about this prompt's pool, which is "cards not yet revealed" — the
//! order deck PLUS the boxed pile, a rule that exists only here (see
//! [`check_order_reveal`]). So the grammar hands back an identity and the
//! functions below rule on it.

use yokohama_core::*;

use crate::card_prompt::{self, CardEntry, CardPrompt, Numbering};
use crate::repl::{Repl, StepOutput};
use crate::screen_model::{CardRef, IndexKind, RecordingTarget, RevealModel};
use crate::style::{paint, Tone};

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
    ///
    /// There is no token-reveal variant: the 5-Power token faces are FACE-UP
    /// open information observed at setup (§1.1), so a token take reads the
    /// known face and applies with no prompt.
    TechReveal { mv: Move, board_b: bool },
}

impl Repl {
    /// What the pending capture is asking for, as pure model data — or `None`
    /// when nothing is pending.
    ///
    /// Every position it hands over is 0-based, which is the one convention
    /// `RecordingTarget` states: `collected.len()` is both the index of the slot
    /// being filled next and the index of this reveal in the run.
    pub(crate) fn reveal_model(&self) -> Option<RevealModel> {
        match &self.pending {
            Pending::None => None,
            Pending::OrderReveals { slots, board_b, collected, .. } => Some(RevealModel {
                target: RecordingTarget::PortRefill {
                    board_b: *board_b,
                    slot: slots[collected.len()],
                    nth: collected.len(),
                    of: slots.len(),
                },
                collected: collected.iter().map(|&id| CardRef::Order(id)).collect(),
            }),
            Pending::TechReveal { board_b, .. } => Some(RevealModel {
                target: RecordingTarget::TechRefill { board_b: *board_b },
                collected: Vec::new(),
            }),
        }
    }

    /// The complete screen for whatever reveal is currently pending (or an
    /// empty string when nothing is pending) — the composed reveal zone, drawn
    /// from [`Repl::reveal_model`].
    ///
    /// It stays a COMPLETE, self-describing message that its callers pass to
    /// `out` as `text` rather than something `out` appends, which is the return
    /// contract every overlay in `repl.rs` honors: closing the index mid-capture
    /// prints this again and the operator is back on the same question.
    pub(crate) fn reveal_prompt(&self) -> String {
        match self.reveal_model() {
            None => String::new(),
            Some(model) => crate::decision_render::reveal_zone(
                &model,
                self.session.state().config.player_count,
            ),
        }
    }

    /// Feed one line of input into the in-progress reveal collection.
    /// Extracts `self.pending` (leaving `Pending::None` behind) so each arm
    /// owns its fields outright; every non-terminal path restores
    /// `self.pending` before returning. Never panics -- the grammar returns a
    /// typed refusal for malformed/unknown input rather than unwrapping.
    ///
    /// **Two steps, and keeping them apart is the design** (T8): the crate's one
    /// card grammar says WHICH CARD the line names, then this module's own pool
    /// check says whether that card can be the answer here. A grammar that also
    /// knew the pool would be a second opinion about the game state, and the pool
    /// rule is genuinely local — "still unrevealed" means the deck OR the boxed
    /// pile, which is true at no other prompt in the driver.
    pub(crate) fn collect_reveal(&mut self, words: &[&str]) -> StepOutput {
        // Computed BEFORE the `mem::replace` below, because `index_placements`
        // reads `self.pending` for this group's own `ThisReveal` entries.
        let placed = self.index_placements();
        let line = words.join(" ");
        match std::mem::replace(&mut self.pending, Pending::None) {
            // Unreachable in practice: `step` only calls this when
            // `self.pending` just matched `!= Pending::None`. Handled
            // gracefully (not `unreachable!()`) rather than risk a panic.
            Pending::None => self.out(String::new(), false),
            Pending::OrderReveals { mv, slots, board_b, mut collected } => {
                let entry = self.read_card(IndexKind::Orders, &placed, &line);
                match entry {
                    Answer::Card(CardRef::Order(card)) => {
                        match check_order_reveal(
                            card,
                            &self.session.state().order_deck,
                            &self.session.state().orders_removed,
                        ) {
                            Ok(()) => {
                                collected.push(card);
                                if collected.len() == slots.len() {
                                    let (out, applied) = self.finish_apply_ex(
                                        mv,
                                        vec![RecordedEvent::RevealOrders(collected.clone())],
                                    );
                                    self.lock_on_order_desync(&slots, &collected, board_b, out, applied)
                                } else {
                                    self.pending =
                                        Pending::OrderReveals { mv, slots, board_b, collected };
                                    self.out(self.reveal_prompt(), false)
                                }
                            }
                            Err(msg) => {
                                self.pending = Pending::OrderReveals { mv, slots, board_b, collected };
                                self.reask(msg)
                            }
                        }
                    }
                    other => {
                        self.pending = Pending::OrderReveals { mv, slots, board_b, collected };
                        self.after_non_answer(other, IndexKind::Orders)
                    }
                }
            }
            Pending::TechReveal { mv, board_b } => {
                let entry = self.read_card(IndexKind::Techs, &placed, &line);
                match entry {
                    Answer::Card(CardRef::Tech(card)) => {
                        match check_tech_reveal(card, &self.session.state().tech_deck) {
                            Ok(()) => {
                                let (out, applied) = self
                                    .finish_apply_ex(mv, vec![RecordedEvent::RevealTechs(vec![card])]);
                                self.lock_on_tech_desync(card, board_b, out, applied)
                            }
                            Err(msg) => {
                                self.pending = Pending::TechReveal { mv, board_b };
                                self.reask(msg)
                            }
                        }
                    }
                    other => {
                        self.pending = Pending::TechReveal { mv, board_b };
                        self.after_non_answer(other, IndexKind::Techs)
                    }
                }
            }
        }
    }

    /// One line at a card prompt, in the crate's one grammar, reduced to what
    /// THIS module can act on.
    ///
    /// **The location check runs ahead of the pool check, and the ordering is
    /// safe for a reason that has to be stated in TWO halves** — the same
    /// argument `Repl::reveal_cmd` makes for the standalone reveal, made again
    /// here because the pool is different.
    ///
    /// `index_placements` reports a card in a Port or Laboratory display slot, in
    /// the viewing seat's own visible hand, in a seat's face-up technologies, on
    /// an area — and, reachable only here, already named in the group being
    /// collected.
    ///
    /// **Half one, the DISJOINT locations.** A card that just came off a deck is
    /// in none of the face-up places above: the thirty-six order cards and thirty
    /// technologies are each partitioned across the piles, so `deck ∪ removed` is
    /// disjoint from every one of them. The only entries the location check turns
    /// away there are ones the pool check would refuse anyway — with a worse
    /// message, since the pool cannot say where the operator has already seen the
    /// card.
    ///
    /// **Half two, `CardLocation::ThisReveal`, which is the DELIBERATE
    /// intersection and the whole reason the location check runs first.** A card
    /// this group has already collected is still sitting in `order_deck` —
    /// nothing is applied until the group completes — so the pool would happily
    /// accept it a second time. `ThisReveal` is the one location that meets the
    /// pool, it meets it in exactly the cards already collected, and that overlap
    /// IS the duplicate refusal (F6). Read the disjointness invariant as a claim
    /// about the other locations; stated over all of them it would be false, and
    /// the rule it protects would be gone.
    ///
    /// `the_location_check_can_never_pre_empt_a_truthful_reveal` sweeps both
    /// halves rather than leaving either as an argument.
    fn read_card(&self, kind: IndexKind, placed: &[(CardRef, crate::screen_model::CardLocation)], line: &str) -> Answer {
        let prompt = CardPrompt { kind, candidates: None, placed, numbering: Numbering::Ids };
        match card_prompt::resolve(&prompt, line, self.session.rules()) {
            CardEntry::Card(card) => Answer::Card(card),
            CardEntry::OpenIndex => Answer::OpenIndex(None),
            CardEntry::Filter(filter) => Answer::OpenIndex(filter),
            CardEntry::Cancel => Answer::Nothing,
            CardEntry::Refused(message) => Answer::Refused(message),
        }
    }

    /// Everything a capture does with a line that was not a usable card.
    ///
    /// **A blank line and `back` re-ask; they never drop the group** — the one
    /// behaviour worth stating outright, because the shared grammar reads both as
    /// `Cancel` and the obvious wiring would have made bare Enter throw away a
    /// parked move with no confirmation. `cancel` still discards, and it still
    /// does so in `Repl::step`, one layer above this function, which is why the
    /// word never reaches here.
    fn after_non_answer(&mut self, answer: Answer, kind: IndexKind) -> StepOutput {
        match answer {
            Answer::OpenIndex(filter) => self.open_index_filtered(kind, filter),
            Answer::Refused(message) => self.reask(message),
            Answer::Nothing => self.reask(format!(
                "this reveal is waiting for an answer -- {}, or 'cancel' to drop the move",
                card_prompt::entry_hint(kind, false),
            )),
            // A card from a registry this capture is not asking about.
            //
            // **Unreachable today, and kept because "today" is doing work in
            // that sentence.** `card_prompt::resolve` reads an id in the
            // PROMPT's own registry, so an order capture cannot be handed a
            // technology: `T07` there is not a cross-registry card, it is not a
            // card at all, and the grammar refuses it with the forms that would
            // have worked. This arm exists so that a registry which ever DOES
            // resolve across kinds meets a message rather than a silent
            // acceptance of the wrong card — the failure the T6 review found
            // when the same collision was reachable through the index.
            Answer::Card(card) => self.reask(format!(
                "{} is not one of this reveal's {} -- it is asking about {}.",
                card_prompt::card_tag(card),
                crate::describe::registry_name(card.kind()),
                crate::describe::registry_name(kind),
            )),
        }
    }

    /// Re-ask the pending capture, with `message` above the question.
    fn reask(&self, message: String) -> StepOutput {
        self.out(format!("{message}\n{}", self.reveal_prompt()), false)
    }

    /// Post-apply desync assertion (design 2026-07-18 §5.2): once a
    /// completed order-reveal group actually applies, the named board
    /// slots must hold exactly the typed cards. A mismatch can only mean a
    /// driver/engine bug -- deck arrangement and board refill are both
    /// deterministic given the reveal -- so it's treated like an invariant
    /// violation: lock the session and surface a CRITICAL banner rather
    /// than silently tracking a board that disagrees with the table.
    /// No-ops when `out.quit`/`self.locked`/`!applied` -- nothing to
    /// re-check against a move that never actually applied. `applied`
    /// (Task 13; inverted from the narrower `was_illegal` flag in the
    /// I1/I2/I3 final-review fix wave's cheap batch) is `finish_apply_ex`'s
    /// report of whether the apply actually SUCCEEDED, covering every error
    /// variant -- the old `was_illegal` flag only caught
    /// `ApplyError::Illegal` and would have let a `GameOver`/`Reveal` error
    /// fall through into a false-positive desync check against a board that
    /// was never touched.
    pub(crate) fn lock_on_order_desync(
        &mut self,
        slots: &[usize],
        collected: &[OrderCardId],
        board_b: bool,
        out: StepOutput,
        applied: bool,
    ) -> StepOutput {
        if out.quit || self.locked || !applied {
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
        let warning = self.engage_lockdown(
            "desync: revealed order card(s) did not match the board after applying".to_string(),
        );
        StepOutput {
            text: lockdown_text(&out.text, &self.lockdown_status(), warning),
            quit: false,
        }
    }

    /// Tech-reveal (Laboratory) counterpart to `lock_on_order_desync` (Task
    /// 13 test (d); `applied` inverted from `was_illegal` alongside it, same
    /// reasoning). Once a completed tech-reveal group actually applies, the
    /// typed `card` must be present somewhere on the board (`b.slots.contains`,
    /// board-wide -- NOT indexed to the one specific slot that just
    /// changed). That's sound, not just convenient: a Laboratory action
    /// only ever refills exactly one slot (see `remove_and_slide_lab`'s doc
    /// comment), and a card fresh off the deck can't already have been
    /// sitting in one of the other five, so "present anywhere on the
    /// board" and "present in the refilled slot" are equivalent here.
    /// Same skip conditions and CRITICAL-banner shape as
    /// `lock_on_order_desync`.
    pub(crate) fn lock_on_tech_desync(
        &mut self,
        card: TechCardId,
        board_b: bool,
        out: StepOutput,
        applied: bool,
    ) -> StepOutput {
        if out.quit || self.locked || !applied {
            return out;
        }
        let state = self.session.state();
        let board = if board_b { state.lab_state.lab_b.as_ref() } else { Some(&state.lab_state.lab_a) };
        let matches_board = board.is_some_and(|b| b.slots.contains(&Some(card)));
        if matches_board {
            return out;
        }
        let warning = self.engage_lockdown(
            "desync: revealed tech card was not on the board after applying".to_string(),
        );
        StepOutput {
            text: lockdown_text(&out.text, &self.lockdown_status(), warning),
            quit: false,
        }
    }
}

/// What one line at a capture prompt turned out to be, once the shared grammar
/// has read it. A narrowing of [`CardEntry`]: a filter and a `?` both end up
/// opening the index here, so they arrive as one thing.
enum Answer {
    Card(CardRef),
    OpenIndex(Option<crate::screen_model::IndexFilter>),
    /// A blank line, or `back`. Never a discard — see [`Repl::after_non_answer`].
    Nothing,
    Refused(String),
}

/// Assemble a desync-lockdown message: the prior output, the CRITICAL banner,
/// the lockdown status line, and a persist-autosave-failure warning if the
/// lockdown couldn't be written to disk (review §3: the lockdown is persisted
/// on lock, and a failed persist must stay visible, not be swallowed).
fn lockdown_text(prior: &str, status: &str, persist_warning: Option<String>) -> String {
    let mut text = format!("{prior}{}\n{status}", paint(Tone::Warn, DESYNC_BANNER));
    if let Some(w) = persist_warning {
        text.push_str(&w);
        text.push('\n');
    }
    text
}

/// Shared by both desync guards -- one banner, painted `Warn` as a unit so
/// the whole sentence lands in bold red on a color terminal.
const DESYNC_BANNER: &str =
    "CRITICAL: desync detected -- the revealed card(s) do not match the board after applying the move. Session locked.";

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

/// This prompt's POOL check for an order card: still UNREVEALED, i.e. in the
/// live deck OR the boxed/removed pile. The removed pile counts because at
/// 2p/3p the seeded deck/removed split among the ~18 boxed cards is fiction --
/// the true top-of-deck card can be one the seeded guess parked in
/// `orders_removed`; `GameState::arrange_orders_on_top` exchanges it into the
/// deck on apply (conservation-by-exchange; see its doc), so the operator must
/// be allowed to name it here. Only a card in NEITHER pile is a genuine desync.
///
/// **The "already recorded for this reveal" half moved out at T8** and is not
/// gone: a card the group has already collected reaches the grammar as a
/// `CardLocation::ThisReveal` placement, so it is refused BY LOCATION with every
/// other duplicate rather than by a rule this prompt kept to itself (F6). What
/// is left here is the one rule that is genuinely local — no other prompt in the
/// driver has a pool of "cards nobody has seen yet".
pub(crate) fn check_order_reveal(
    card: OrderCardId,
    deck: &[OrderCardId],
    removed: &[OrderCardId],
) -> Result<(), String> {
    if !deck.contains(&card) && !removed.contains(&card) {
        return Err(format!(
            "O{:02} is not an unrevealed order card (not in the deck or the boxed/removed pile).",
            card.0,
        ));
    }
    Ok(())
}

/// The same check for a technology: the named card must still be in the tech
/// deck. Techs are never boxed at setup, so unlike an order card there is no
/// second pile to allow and a card that is not in the deck is a real mismatch.
///
/// Its own DECK-FLAVORED wording, kept verbatim through T8 (fix wave, review of
/// commit 0a01c95 finding 2: these exact messages regressed to the wizard's
/// registry-flavored wording when the two were first unified -- "not currently
/// in the tech deck" matters here because a card can be real but already drawn,
/// a materially different fact from "not a valid tech card"). Reached with an id
/// the grammar has ALREADY resolved — from a bare number, a `T07`, or a name —
/// so the only fact left for this function to state is the pool one. A spelling
/// that named no card never arrives here at all; that refusal is `card_prompt`'s,
/// in `card_prompt`'s words.
pub(crate) fn check_tech_reveal(card: TechCardId, deck: &[TechCardId]) -> Result<(), String> {
    if !deck.contains(&card) {
        return Err(format!("T{:02} is not currently in the tech deck.", card.0));
    }
    Ok(())
}

/// Parse a 5-Power token-effect letter into the canonical pool's fixed
/// magnitude for that kind -- one letter per `FivePowerEffect` variant (the
/// phantom `VictoryPoints`/`Movement` variants were removed once the
/// physical box check confirmed only these six faces exist).
pub(crate) fn parse_token_letter(words: &[&str]) -> Result<FivePowerEffect, String> {
    let usage = "Enter one letter: i/c/s/t/f/y.".to_string();
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
    use crate::screen_model::menu_number;
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

    /// A live 2P session parked at a one-card Port-A refill question, plus a
    /// card genuinely in the unrevealed deck.
    ///
    /// The walk is `live_port_take_collects_reveals_then_applies_them`'s,
    /// factored out for T8's tests rather than copied into each of them; that
    /// test keeps its own inline copy, comment and all, because its assertions
    /// are the pinned Step-1 choreography and it should not start depending on
    /// a helper a later task could quietly change.
    fn live_repl_at_a_port_refill() -> (Repl, OrderCardId) {
        let (mut r, others) = live_repl();
        let port_area = *r
            .session
            .state()
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type == AreaType::PortA)
            .unwrap()
            .0;
        r.step("1");
        r.step(&format!("a {}", port_area.0));
        let k = r
            .session
            .legal()
            .iter()
            .position(|m| matches!(m, Move::PlacePresident { destination } if *destination == port_area))
            .expect("PlacePresident at Port A is legal from hand at game start");
        r.step(&menu_number(k).to_string());
        let take = r
            .session
            .legal()
            .iter()
            .position(|m| matches!(
                m,
                Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } }
                    if cards_taken.len() == 1
            ))
            .expect("1-card port take available");
        r.step(&menu_number(take).to_string());
        assert!(matches!(r.pending, Pending::OrderReveals { .. }), "parked at a reveal");
        (r, others[0])
    }

    /// The same walk parked at a TWO-slot Port refill, so the group has a second
    /// question — the only shape in the driver where a card can already be
    /// `CardLocation::ThisReveal` while still sitting in the deck.
    fn live_repl_at_a_two_slot_port_refill() -> Repl {
        let (mut r, _) = live_repl();
        let port_area = *r
            .session
            .state()
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type == AreaType::PortA)
            .unwrap()
            .0;
        r.step("1");
        r.step(&format!("a {}", port_area.0));
        let k = r
            .session
            .legal()
            .iter()
            .position(|m| matches!(m, Move::PlacePresident { destination } if *destination == port_area))
            .expect("PlacePresident at Port A");
        r.step(&menu_number(k).to_string());
        let take = r
            .session
            .legal()
            .iter()
            .position(|m| matches!(
                m,
                Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } }
                    if cards_taken.len() == 2
            ))
            .expect("a 2-card port take is available");
        r.step(&menu_number(take).to_string());
        assert!(matches!(r.pending, Pending::OrderReveals { .. }), "parked at a 2-slot reveal");
        r
    }

    /// **The capture takes the crate's ONE card grammar** (spec §0.2, finding
    /// F4) — the T8 requirement, exercised at the prompt rather than at the
    /// parser. Every spelling that names a card at any other prompt in the
    /// driver names one here, and every non-card line does what it does
    /// everywhere else.
    ///
    /// Before T8 this prompt had a grammar of its own: `?` reached it as a bad
    /// card, `f silk` as a bad card, and a technology id as a bad card — three
    /// different things, one message.
    #[test]
    fn the_capture_takes_the_crates_one_card_grammar() {
        // Every spelling of the same id reaches the same card, from a fresh
        // capture each time so the group state cannot carry between them.
        for spelling in ["O{n}", "o{n}", "{n}"] {
            let (mut r, deck_card) = live_repl_at_a_port_refill();
            let events_before = r.session.record().events.len();
            let out = r.step(&spelling.replace("{n}", &deck_card.0.to_string()));
            assert!(!out.text.contains("CRITICAL"), "{spelling}: {}", out.text);
            assert_eq!(
                r.session.record().events.len(),
                events_before + 2,
                "{spelling} must record RevealOrders + Move: {}",
                out.text,
            );
            assert!(
                r.session.state().port_state.port_a.slots.contains(&Some(deck_card)),
                "{spelling} must name the same card",
            );
        }

        // `?` opens the index over the capture (T5's contract, re-verified from
        // the grammar's side), and `f silk` opens it already narrowed rather
        // than being read as a bad card.
        let (mut r, _) = live_repl_at_a_port_refill();
        let out = r.step("f silk");
        assert!(out.text.contains("ORDER CARD INDEX"), "{}", out.text);
        assert!(matches!(r.pending, Pending::OrderReveals { .. }), "the capture is untouched");
        assert!(
            matches!(
                r.overlay,
                Some(crate::repl::Overlay::Index {
                    filter: Some(crate::screen_model::IndexFilter::Good(
                        crate::screen_model::CardGood::Silk
                    )),
                    ..
                })
            ),
            "the filter the player typed is the one on screen",
        );

        // A technology's spelling names NOTHING at an order capture — it is not
        // silently read as `O07`, which is the wrong-card acceptance the shared
        // grammar exists to refuse (`card_prompt`'s
        // `a_bare_id_reads_in_the_prompts_own_registry`, at this prompt).
        let (mut r, _) = live_repl_at_a_port_refill();
        let events_before = r.session.record().events.len();
        let out = r.step("T07");
        assert!(out.text.contains("is not a card"), "{}", out.text);
        assert!(out.text.contains("like 'O14'"), "and says what would have worked: {}", out.text);
        assert_eq!(r.session.record().events.len(), events_before, "nothing was recorded");
        assert!(matches!(r.pending, Pending::OrderReveals { .. }), "nothing was collected");

        // And junk says what would have worked, which is the half of F4 the old
        // prompt's "'x' is not a valid order-card code" left out.
        let (mut r, _) = live_repl_at_a_port_refill();
        let out = r.step("wat");
        assert!(out.text.contains("'?' for the index"), "{}", out.text);
    }

    /// **A card the operator can SEE somewhere else is refused by location**
    /// (F6), through the same rule every other card prompt uses. The pre-T8
    /// capture refused it as "not an unrevealed order card", which is true and
    /// tells a person at a table nothing.
    #[test]
    fn a_capture_refusal_names_where_the_card_already_is() {
        let (mut r, _) = live_repl_at_a_port_refill();
        let on_display = r.session.state().port_state.port_a.slots[0].expect("Port A slot 1 is dealt");
        let events_before = r.session.record().events.len();
        let out = r.step(&format!("O{:02}", on_display.0));
        assert!(out.text.contains("Port A slot 1"), "the refusal names the place: {}", out.text);
        assert!(out.text.contains("cannot be in two places"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before, "nothing was recorded");
        assert!(matches!(r.pending, Pending::OrderReveals { .. }), "the capture is still waiting");
    }

    /// The same rule for a card already named IN THIS GROUP: the check that
    /// used to live in this module's own parser now arrives as a
    /// `CardLocation::ThisReveal` placement, so a two-slot refill still cannot
    /// be told the same card twice — and now says where it already is.
    ///
    /// **The assertion is aimed at the REFUSAL LINE, not at the screen** (T8
    /// review, mutation M13). A screen-wide `contains("in this reveal")` is
    /// satisfied by the prompt's own `already recorded in this reveal — O10`
    /// status line, which is drawn from `collected` and says nothing about which
    /// `CardLocation` the duplicate was refused by: pointing `index_placements`
    /// at `OwnHand` instead left the refusal reading "O10 is in your hand" and
    /// the test still passed. So the line under test is picked out by the phrase
    /// only the refusal carries, and BOTH halves of it are then read off that one
    /// line. This is the T7 lesson recurring — two byte-identical text blocks on
    /// one screen blind a transcript assertion — and the answer is the same:
    /// assert on the line that carries the fact, and unit-pin the string it is
    /// built from (`describe`'s `the_this_reveal_location_says_in_this_reveal`).
    #[test]
    fn a_card_already_named_in_this_group_is_refused_by_location() {
        let mut r = live_repl_at_a_two_slot_port_refill();
        let first = *r.session.state().order_deck.first().expect("an unrevealed card");
        r.step(&format!("O{:02}", first.0));
        let events_before = r.session.record().events.len();
        let out = r.step(&format!("O{:02}", first.0));
        let plain = crate::style::strip_ansi(&out.text);
        let refusal = plain
            .lines()
            .find(|l| l.contains("cannot be in two places"))
            .unwrap_or_else(|| panic!("no duplicate refusal on the screen:\n{plain}"));
        assert!(
            refusal.contains(&format!("O{:02}", first.0)),
            "the refusal names the card that was refused: {refusal}",
        );
        assert!(
            refusal.contains("in this reveal"),
            "and names WHERE it already is, on that same line — the prompt's own \
             'already recorded in this reveal' status line does not count: {refusal}",
        );
        assert_eq!(r.session.record().events.len(), events_before, "nothing applied");
        assert!(
            matches!(&r.pending, Pending::OrderReveals { collected, .. } if collected.len() == 1),
            "the group still holds exactly the one answer it had",
        );
    }

    /// **The location check runs ahead of the pool check, and can never
    /// pre-empt a truthful reveal** — the conservation argument
    /// [`Repl::read_card`] makes, swept instead of asserted, in the two halves
    /// that argument has.
    ///
    /// **Half one, the disjoint locations.** For BOTH registries: no card the
    /// pool would accept has a location the grammar could refuse it by. The order
    /// side is the one that matters at a live table (the boxed pile is ~18 cards
    /// at 2P and every one of them is a legitimate answer); the tech side is what
    /// makes name resolution safe, since `card_prompt` resolves a title to the
    /// copy nothing has CLAIMED while this prompt's pool is the copies still in
    /// the DECK — two rules that agree only because the placements and the deck
    /// partition the registry.
    ///
    /// **Half two, the one deliberate intersection** (T8 review, SHOULD-1). The
    /// first two blocks run on an EMPTY `collected`, so `CardLocation::ThisReveal`
    /// — the one location that does meet the pool, and the one the duplicate
    /// refusal is built on — was outside the sweep by construction. Mid-group the
    /// intersection must be non-empty and must be EXACTLY the collected cards:
    /// smaller and a duplicate slips through, larger and a truthful reveal is
    /// pre-empted after all.
    #[test]
    fn the_location_check_can_never_pre_empt_a_truthful_reveal() {
        let (r, _) = live_repl_at_a_port_refill();
        let placed = r.index_placements();
        let state = r.session.state();

        let pool: Vec<OrderCardId> =
            state.order_deck.iter().chain(state.orders_removed.iter()).copied().collect();
        assert!(pool.len() > 10, "fixture: a 2P live game leaves a large unrevealed pool");
        for card in &pool {
            assert!(
                !placed.iter().any(|(c, _)| *c == CardRef::Order(*card)),
                "O{:02} is in the unrevealed pool AND carries a location — the refusal would \
                 pre-empt a truthful reveal",
                card.0,
            );
        }

        // The tech partition, stated the other way round as well: every physical
        // technology is either in the deck or placed, never both and never
        // neither — which is exactly what makes "the copy nothing has claimed"
        // and "the lowest copy still in the deck" the same card.
        let deck: Vec<TechCardId> = state.tech_deck.clone();
        let placed_techs: Vec<TechCardId> = placed
            .iter()
            .filter_map(|(c, _)| match c {
                CardRef::Tech(id) => Some(*id),
                _ => None,
            })
            .collect();
        for id in &deck {
            assert!(!placed_techs.contains(id), "T{:02} is in the deck AND placed", id.0);
        }
        let total = r.session.rules().techs.iter().count();
        assert_eq!(
            deck.len() + placed_techs.len(),
            total,
            "the {total} physical technologies must be exactly deck + placed",
        );

        // Half two: one answer collected at a two-slot refill, which is the only
        // state in the driver where a card is both in the pool and placed.
        let mut r = live_repl_at_a_two_slot_port_refill();
        let collected = *r.session.state().order_deck.first().expect("an unrevealed card");
        r.step(&format!("O{:02}", collected.0));
        assert!(
            matches!(&r.pending, Pending::OrderReveals { collected, .. } if collected.len() == 1),
            "fixture: the group holds exactly one answer and is still asking",
        );
        let placed = r.index_placements();
        let state = r.session.state();
        let pool: Vec<OrderCardId> =
            state.order_deck.iter().chain(state.orders_removed.iter()).copied().collect();
        let intersection: Vec<(OrderCardId, crate::screen_model::CardLocation)> = pool
            .iter()
            .filter_map(|card| {
                placed
                    .iter()
                    .find(|(placed, _)| *placed == CardRef::Order(*card))
                    .map(|(_, at)| (*card, *at))
            })
            .collect();
        assert_eq!(
            intersection,
            vec![(collected, crate::screen_model::CardLocation::ThisReveal)],
            "mid-group, the pool must meet the placements in exactly the collected card, as a \
             ThisReveal entry — anything more pre-empts a truthful reveal, anything less lets the \
             same card be recorded twice",
        );
    }

    /// **A blank line, and `back`, RE-ASK — they never drop the parked move.**
    ///
    /// The shared grammar reads both as `Cancel`, and the obvious wiring would
    /// have made bare Enter throw a parked move away with no confirmation, which
    /// is the quietest destructive act a prompt can perform. `cancel` still
    /// discards, and the message names it as the way to.
    #[test]
    fn an_empty_line_or_back_re_asks_and_never_drops_the_parked_move() {
        for line in ["", "back", "   "] {
            let (mut r, _) = live_repl_at_a_port_refill();
            let out = r.step(line);
            assert!(
                matches!(r.pending, Pending::OrderReveals { .. }),
                "{line:?} must leave the move parked",
            );
            assert!(out.text.contains("waiting for an answer"), "{line:?}: {}", out.text);
            assert!(out.text.contains("'cancel'"), "and names the way out: {}", out.text);
        }
        // ...and the word that DOES discard still discards, unchanged.
        let (mut r, _) = live_repl_at_a_port_refill();
        let out = r.step("cancel");
        assert!(matches!(r.pending, Pending::None), "cancel drops the group");
        assert!(out.text.contains("no move was applied"), "{}", out.text);
    }

    /// A live 2P session parked at the Laboratory's one-slot tech-refill
    /// question, reached by the walk `live_lab_action_collects_tech_reveal_then_
    /// applies_it` uses (that test keeps its own inline copy for the same reason
    /// the Port one does — its assertions are the pinned choreography).
    ///
    /// A REAL parked move, not a hand-set `Pending::TechReveal`, because what the
    /// tests below assert is that an answer REACHES THE RECORD: a synthetic
    /// pending carrying a move the position does not allow completes the group
    /// and then fails the apply, so nothing is recorded and every positive
    /// assertion about the answer has to be weakened back into a negative.
    fn live_repl_at_a_tech_refill() -> Repl {
        let (mut r, _) = live_repl();
        let lab_area = *r
            .session
            .state()
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type == AreaType::LaboratoryA)
            .unwrap()
            .0;
        r.step("1");
        r.step(&format!("a {}", lab_area.0));
        let k = r
            .session
            .legal()
            .iter()
            .position(|m| matches!(m, Move::PlacePresident { destination } if *destination == lab_area))
            .expect("PlacePresident at Laboratory A is legal from hand at game start");
        r.step(&menu_number(k).to_string());
        let take = r
            .session
            .legal()
            .iter()
            .position(|m| matches!(m, Move::ExecuteAreaAction { action: AreaAction::Laboratory { .. } }))
            .expect("a Laboratory acquisition is available");
        r.step(&menu_number(take).to_string());
        assert!(matches!(r.pending, Pending::TechReveal { .. }), "parked at a tech reveal");
        r
    }

    /// The tech capture through the one grammar: a NAME names the card (R8 —
    /// nothing is printed on a technology to identify it by), and the pool
    /// refusal still talks about the deck.
    ///
    /// **The first half asserts what the name RESOLVED TO, not merely that one
    /// message did not appear** (T8 review, mutation M1). It used to read
    /// `!contains("not currently in the tech deck")`, which is satisfied by any
    /// other outcome — including the total grammar failure the review injected by
    /// making `tech_matches` return nothing, whose refusal is worded "'…' is not
    /// a technology" and sailed straight through. A negative assertion about one
    /// string is not evidence that names work; the recorded event is.
    ///
    /// The TITLE is chosen rather than taken off the deck's top, because
    /// `card_prompt` resolves a name to the lowest-id copy NOTHING HAS CLAIMED
    /// and the old fixture named whatever card happened to be on top — so the
    /// comment claiming the resolved copy was "genuinely the one we are naming"
    /// was describing a coincidence. Here the title is picked with both copies
    /// still in the deck and its resolution asserted up front, so the id the
    /// record must carry is known before the answer is typed.
    #[test]
    fn the_tech_capture_takes_a_name_and_keeps_its_deck_flavored_refusal() {
        let mut r = live_repl_at_a_tech_refill();
        let rules = r.session.rules().clone();
        let deck = r.session.state().tech_deck.clone();
        // A title every copy of which is still in the deck: the grammar's
        // free-copy rule then resolves the name to the LOWEST-id copy, and this
        // prompt's pool accepts it.
        let (name, expected) = rules
            .techs
            .iter()
            .map(|c| c.name.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .find_map(|name| {
                let mut copies: Vec<TechCardId> =
                    rules.techs.iter().filter(|c| c.name == name).map(|c| c.id).collect();
                copies.sort_unstable_by_key(|c| c.0);
                copies.iter().all(|c| deck.contains(c)).then(|| (name, copies[0]))
            })
            .expect("some title has every copy still in the deck at game start");
        // The fixture's own precondition, asserted rather than assumed: the name
        // is unambiguous and resolves to exactly the id the record is checked for.
        let placed = r.index_placements();
        assert!(
            !placed.iter().any(|(c, _)| *c == CardRef::Tech(expected)),
            "fixture: the copy the name resolves to must be unclaimed",
        );
        let events_before = r.session.record().events.len();

        let out = r.step(&name);

        assert!(
            r.session.record().events.iter().any(|e| matches!(
                e, RecordedEvent::RevealTechs(cards) if cards == &vec![expected]
            )),
            "the title {name:?} must have named T{:02} and reached the record as the reveal: {}",
            expected.0,
            out.text,
        );
        assert_eq!(
            r.session.record().events.len(),
            events_before + 2,
            "RevealTechs + the parked move, as one group: {}",
            out.text,
        );
        assert!(
            r.session.state().lab_state.lab_a.slots.contains(&Some(expected)),
            "and the named card is physically on the board: {}",
            out.text,
        );
        assert!(matches!(r.pending, Pending::None), "the group completed: {}", out.text);

        // A technology that is real but already OFF the deck is refused, never
        // accepted. Which of the two refusals it meets is decided by
        // conservation, not by preference: every non-deck technology is placed
        // SOMEWHERE (the tech half of
        // `the_location_check_can_never_pre_empt_a_truthful_reveal`), so through
        // the REPL it is the location that speaks and `check_tech_reveal`'s
        // deck-flavored wording is pinned directly instead, by
        // `the_tech_pool_is_the_deck_alone_and_says_so_in_deck_words`.
        let mut r = live_repl_at_a_tech_refill();
        let on_board = r.session.state().lab_state.lab_a.slots[0].expect("Lab A slot 1 is dealt");
        let events_before = r.session.record().events.len();
        let out = r.step(&format!("T{:02}", on_board.0));
        assert!(matches!(r.pending, Pending::TechReveal { .. }), "still waiting: {}", out.text);
        assert_eq!(r.session.record().events.len(), events_before, "nothing was recorded");
        assert!(
            out.text.contains("Laboratory A slot 1") || out.text.contains("not currently in the tech deck"),
            "refused by location or by pool, never accepted: {}",
            out.text,
        );
    }

    /// **The guide's §6 captures are what these fixtures actually render** —
    /// the T4-review rule, applied to T8's three screens.
    ///
    /// That review found a FABRICATED transcript in the guide: a splice that
    /// printed a row the code refuses to print. The answer then was to
    /// regenerate the block from the fixture and pin it byte-exact, so the guide
    /// cannot drift and cannot be written by hand. Same here, for the reveal
    /// screens a tester reads before their first live game. `guide_sync` pins
    /// the `.html` twin against the `.md`; this test says the `.md` is the
    /// driver's own output.
    #[test]
    fn the_guide_reveal_captures_are_what_the_fixtures_render() {
        const GUIDE_MD: &str = include_str!("../../../docs/cli-guide.md");
        let blocks: Vec<&str> =
            GUIDE_MD.split("```").map(|part| part.trim_matches('\n')).collect();
        let has = |text: &str| {
            blocks.iter().any(|b| *b == text.trim_end_matches('\n'))
        };

        // (1) A bound Port refill, from the walk that produces one.
        let (r, _) = live_repl_at_a_port_refill();
        let port = crate::style::strip_ansi(&r.reveal_prompt());
        assert!(
            has(&port),
            "docs/cli-guide.md §6's captured Port-refill screen is not what \
             `live_repl_at_a_port_refill` renders. Re-capture it in BOTH formats from ONE run of \
             this fixture -- never by editing the block by hand (T4 review M1). Rendered:\n{port}",
        );

        // (2) and (3) The guided reveal: the seat menu, then the card question.
        let (mut r, _) = live_repl();
        let menu = crate::style::strip_ansi(&r.step("reveal").text);
        assert!(
            has(&menu),
            "docs/cli-guide.md §6's captured seat menu is not what `reveal` renders. \
             Rendered:\n{menu}",
        );
        let card = crate::style::strip_ansi(&r.step("1").text);
        assert!(
            has(&card),
            "docs/cli-guide.md §6's captured starting-order question is not what the guided \
             reveal renders. Rendered:\n{card}",
        );
    }

    /// **The prompt is the composed reveal zone** (T8's surface half): the
    /// banner sentence, the question naming the exact slot, and a hint whose
    /// clauses are all true HERE — no `back`, no `view`, both of which the
    /// capture branch of `step` never reaches.
    #[test]
    fn the_capture_prompt_is_the_composed_reveal_zone() {
        let (mut r, _) = live_repl_at_a_port_refill();
        let text = crate::style::strip_ansi(&r.reveal_prompt());
        assert!(text.contains("LIVE TRACKING"), "{text}");
        assert!(text.contains("refilled 1 slot from the order deck."), "{text}");
        assert!(text.contains("YOU ARE RECORDING"), "{text}");
        assert!(text.contains("Port A slot"), "{text}");
        assert!(text.contains("which order card is it?"), "{text}");
        assert!(text.contains("'?' for the card index"), "the grammar is on screen: {text}");
        assert!(text.contains("'cancel' drops the move"), "{text}");
        assert!(!text.contains("'back' undoes"), "the decision zone's hint is FALSE here: {text}");
        assert!(text.trim_end().ends_with('>'), "the caret is the last thing on it: {text:?}");

        // The already-recorded line appears only once the group holds something,
        // and names what it holds.
        assert!(!text.contains("already recorded"), "nothing collected yet: {text}");
        r.pending = Pending::OrderReveals {
            mv: Move::EndAdditionalPhase,
            slots: vec![3, 1],
            board_b: false,
            collected: vec![OrderCardId(4)],
        };
        let text = crate::style::strip_ansi(&r.reveal_prompt());
        assert!(text.contains("already recorded in this reveal"), "{text}");
        assert!(text.contains("O04"), "{text}");
        assert!(text.contains("(reveal 2 of 2)"), "and the run position: {text}");
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
        r.step("1");        // EndAdditionalPhase
        r.step(&format!("a {}", port_area.0)); // place 1 assistant at Port A
        let k = r.session.legal().iter().position(|m| matches!(
            m, Move::PlacePresident { destination } if *destination == port_area
        )).expect("PlacePresident at Port A is legal from hand at game start");
        r.step(&menu_number(k).to_string());
        // Pick a 1-card port take.
        let take = r.session.legal().iter().position(|m| matches!(
            m, Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } } if cards_taken.len() == 1
        )).expect("1-card port take available");
        let out = r.step(&menu_number(take).to_string());
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

    /// Review §3 blocker 2 (the live gate is unrunnable): at 2p, ~18 of 36
    /// order cards are boxed face-down at setup, so the engine's `order_deck`
    /// is a seeded guess and the true top-of-deck card can be one the guess
    /// parked in `orders_removed`. The operator, revealing what physically
    /// refilled the Port, must be able to type THAT card. Before the
    /// conservation-by-exchange fix the reveal was refused ("not currently in
    /// the order deck") and re-prompted forever; now it is exchanged into the
    /// deck, drawn, and lands on the board, conserving the 36-card multiset.
    #[test]
    fn live_port_reveal_accepts_a_card_the_seeded_setup_boxed() {
        let (mut r, _others) = live_repl();
        // Same walk to a Port-A take as live_port_take_...; see its comment.
        let port_area = *r.session.state().board_layout.areas.iter()
            .find(|(_, c)| c.area_type == AreaType::PortA).unwrap().0;
        r.step("1");
        r.step(&format!("a {}", port_area.0));
        let k = r.session.legal().iter().position(|m| matches!(
            m, Move::PlacePresident { destination } if *destination == port_area
        )).expect("PlacePresident at Port A is legal from hand at game start");
        r.step(&menu_number(k).to_string());
        let take = r.session.legal().iter().position(|m| matches!(
            m, Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } } if cards_taken.len() == 1
        )).expect("1-card port take available");
        let out = r.step(&menu_number(take).to_string());
        assert!(out.text.to_lowercase().contains("slot"), "{}", out.text);
        // A physically-real order card the seeded setup boxed: in orders_removed,
        // never in the deck (conservation makes the two piles disjoint).
        let boxed = *r.session.state().orders_removed.iter()
            .find(|c| !r.session.state().order_deck.contains(c))
            .expect("2p setup boxes ~18 order cards into orders_removed");
        assert!(!r.session.state().order_deck.contains(&boxed));
        let events_before = r.session.record().events.len();
        let out = r.step(&format!("O{}", boxed.0));
        // Accepted (not refused), applied without a desync lock, and the boxed
        // card the operator revealed is now physically on the Port A board.
        assert!(!out.text.to_lowercase().contains("not currently in the order deck"),
            "truthful reveal of a boxed card was refused: {}", out.text);
        assert!(!out.text.contains("CRITICAL"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 2, "RevealOrders + Move: {}", out.text);
        assert!(r.session.state().port_state.port_a.slots.contains(&Some(boxed)),
            "the revealed boxed card is now on Port A");
        // The record still replays exactly (conservation + reveal fold agree).
        assert_eq!(replay(r.session.record(), r.session.rules()).unwrap(), *r.session.state());
    }

    /// Task 13 test (c), second half: `lock_on_order_desync` must skip its
    /// check because the apply did NOT succeed (`applied = false`), not
    /// because the formatted text happened to contain the substring
    /// "Illegal:" -- proven directly by handing it a `StepOutput` whose text
    /// contains no such substring at all. A pre-Task-13 string-sniffing
    /// check would fail to skip here (no match) and go on to report a
    /// false-positive desync lock against a board that was deliberately
    /// never checked for realism. `applied = false` covers every error
    /// variant (final-review cheap batch: inverted from the narrower
    /// `was_illegal`, which only covered `ApplyError::Illegal`).
    #[test]
    fn lock_on_order_desync_skips_when_the_apply_did_not_succeed() {
        let (mut r, _others) = live_repl();
        let out = StepOutput { text: "some unrelated message\n".to_string(), quit: false };
        let out = r.lock_on_order_desync(&[0], &[OrderCardId(1)], false, out, false);
        assert!(!r.locked, "must skip the desync check when applied is false, regardless of text content");
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
        let out = r.lock_on_tech_desync(wrong_card, false, out, true);
        assert!(r.locked, "a typed card absent from the board must lock the session");
        assert!(out.text.contains("CRITICAL"), "{}", out.text);
    }

    /// Review §3: a desync lockdown must SURVIVE save -> quit -> reload. The
    /// `locked` flag lived only on the `Repl` before this fix, so a reloaded
    /// session silently came back UNLOCKED and playable. Now the reason is
    /// persisted into `GameRecord.lockdown` and the driver restores `locked`
    /// from it on load.
    #[test]
    fn a_desync_lockdown_survives_save_and_reload() {
        let (mut r, _others) = live_repl();
        // Force a desync lock via the tech guard (a card never on any board).
        let out = StepOutput { text: String::new(), quit: false };
        let out = r.lock_on_tech_desync(TechCardId(200), false, out, true);
        assert!(r.locked && out.text.contains("CRITICAL"), "guard must lock: {}", out.text);
        // Persist and reload through the real save/load path.
        let path = std::env::temp_dir().join(format!("yokohama_lockdown_{}.json", std::process::id()));
        r.session.save(&path).unwrap();
        let reloaded = Session::load(&path, Rules::standard()).unwrap();
        let r2 = Repl::new(reloaded);
        std::fs::remove_file(&path).ok();
        assert!(r2.locked, "a desync lockdown must survive reload, not silently clear");
        // The reason is preserved for the operator/postmortem, not just the flag.
        assert!(r2.session.record().lockdown.is_some(), "the lockdown reason must be persisted");
    }

    /// `lock_on_tech_desync`'s no-op path, mirroring
    /// `lock_on_order_desync_skips_when_the_apply_did_not_succeed`.
    #[test]
    fn lock_on_tech_desync_skips_when_the_apply_did_not_succeed() {
        let (mut r, _others) = live_repl();
        let out = StepOutput { text: "some unrelated message\n".to_string(), quit: false };
        let out = r.lock_on_tech_desync(TechCardId(200), false, out, false);
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
        r.step("1");
        r.step(&format!("a {}", port_area.0));
        let k = r.session.legal().iter().position(|m| matches!(
            m, Move::PlacePresident { destination } if *destination == port_area
        )).unwrap();
        r.step(&menu_number(k).to_string());
        let before = r.session.state().clone();
        let events_before = r.session.record().events.len();
        let take = r.session.legal().iter().position(|m| matches!(
            m, Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } } if cards_taken.len() == 1
        )).unwrap();
        r.step(&menu_number(take).to_string());
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
        r.step("1");
        r.step(&format!("a {}", lab_area.0));
        let k = r.session.legal().iter().position(|m| matches!(
            m, Move::PlacePresident { destination } if *destination == lab_area
        )).expect("PlacePresident at Laboratory A is legal from hand at game start");
        r.step(&menu_number(k).to_string());
        let take = r.session.legal().iter().position(|m| matches!(
            m, Move::ExecuteAreaAction { action: AreaAction::Laboratory { .. } }
        )).expect("a Laboratory acquisition is available");
        let out = r.step(&menu_number(take).to_string());
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
        assert!(r.session.state().lab_state.lab_a.slots.contains(&Some(top_tech)));
        assert_eq!(replay(r.session.record(), r.session.rules()).unwrap(), *r.session.state());
    }

    // The pool checks below are pure functions, so their rules are pinned
    // directly rather than through a full Repl/Session (cheaper, and pins the
    // exact rule rather than just "the REPL didn't crash"). What they no longer
    // pin is the GRAMMAR -- junk input, spellings, name prefixes and duplicates
    // are `card_prompt`'s, tested in its own matrix and exercised end-to-end by
    // `the_capture_takes_the_crates_one_card_grammar` below.

    /// **The pool rule, and the one thing about it that matters at a live
    /// table:** a card the seeded setup BOXED is a legitimate reveal, because at
    /// 2-3 players most of the 36-card deck is never seen and the engine's split
    /// between deck and boxed pile is a guess (`arrange_orders_on_top`
    /// exchanges on apply). Only a card in neither pile is a real mismatch.
    #[test]
    fn the_order_pool_admits_the_deck_and_the_boxed_pile_and_nothing_else() {
        let deck = vec![OrderCardId(5), OrderCardId(9)];
        let boxed = vec![OrderCardId(30)]; // physically real, boxed at setup
        assert_eq!(check_order_reveal(OrderCardId(5), &deck, &boxed), Ok(()));
        assert_eq!(check_order_reveal(OrderCardId(30), &deck, &boxed), Ok(()));
        let refusal = check_order_reveal(OrderCardId(1), &deck, &boxed)
            .expect_err("in neither pile");
        assert_eq!(
            refusal,
            "O01 is not an unrevealed order card (not in the deck or the boxed/removed pile).",
        );
    }

    /// The tech half, and its wording, which is NOT the registry's (fix wave,
    /// review of commit 0a01c95 finding 2): a live-reveal rejection must talk
    /// about the DECK, because a card can be real but already drawn -- a
    /// materially different fact from "not a valid tech card". Techs are never
    /// boxed at setup, so there is no second pile to admit.
    #[test]
    fn the_tech_pool_is_the_deck_alone_and_says_so_in_deck_words() {
        let ball_deck = vec![TechCardId(11), TechCardId(12)];
        assert_eq!(check_tech_reveal(TechCardId(11), &ball_deck), Ok(()));
        let not_in_deck =
            check_tech_reveal(TechCardId(99), &ball_deck).expect_err("valid code, not in deck");
        assert_eq!(not_in_deck, "T99 is not currently in the tech deck.", "{not_in_deck}");
        assert!(!not_in_deck.contains("valid tech card"), "the registry's wording must not leak in");
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
