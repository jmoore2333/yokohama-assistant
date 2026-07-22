//! Live-mode reveal choreography extracted from `repl.rs` (Task 10, design
//! docs/plans/2026-07-18-phase2-cli-driver-design.md): the `Pending` state
//! `Repl::commit` parks a move in while the operator answers reveal
//! prompts, the prompt/collection loop, and the post-apply desync guard.

use yokohama_core::*;

use crate::input::{parse_order_id, parse_tech_entry, TechAmbiguity, TechEntryError};
use crate::repl::{Repl, StepOutput};
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
                match parse_order_reveal(
                    words,
                    &self.session.state().order_deck,
                    &self.session.state().orders_removed,
                    &collected,
                ) {
                    Ok(card) => {
                        collected.push(card);
                        if collected.len() == slots.len() {
                            let (out, applied) =
                                self.finish_apply_ex(mv, vec![RecordedEvent::RevealOrders(collected.clone())]);
                            self.lock_on_order_desync(&slots, &collected, board_b, out, applied)
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
                        let (out, applied) =
                            self.finish_apply_ex(mv, vec![RecordedEvent::RevealTechs(vec![card])]);
                        self.lock_on_tech_desync(card, board_b, out, applied)
                    }
                    Err(msg) => {
                        self.pending = Pending::TechReveal { mv, board_b };
                        self.out(format!("{msg}\n{}", self.reveal_prompt()), false)
                    }
                }
            }
        }
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

/// Parse one order-reveal answer: a real numeric id, still UNREVEALED (in the
/// live deck OR the boxed/removed pile), and not already in `collected` (a card
/// can't be drawn twice in one refill). The removed pile counts because at
/// 2p/3p the seeded deck/removed split among the ~18 boxed cards is fiction --
/// the true top-of-deck card can be one the seeded guess parked in
/// `orders_removed`; `GameState::arrange_orders_on_top` exchanges it into the
/// deck on apply (conservation-by-exchange; see its doc), so the operator must
/// be allowed to name it here. Only a card in NEITHER pile is a genuine desync.
pub(crate) fn parse_order_reveal(
    words: &[&str],
    deck: &[OrderCardId],
    removed: &[OrderCardId],
    collected: &[OrderCardId],
) -> Result<OrderCardId, String> {
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
    if !deck.contains(&card) && !removed.contains(&card) {
        return Err(format!(
            "O{n:02} is not an unrevealed order card (not in the deck or the boxed/removed pile)."
        ));
    }
    Ok(card)
}

/// Parse one tech-reveal answer: `T<id>` (validated against `deck`) or a
/// case-insensitive name prefix, resolved to the LOWEST matching id
/// currently in `deck` (some titles circulate as 2 physical copies) --
/// thin wrapper over the shared `parse_tech_entry` (design 2026-07-18
/// §5.5 riding item 4), which is why an ambiguous name here resolves
/// (`TechAmbiguity::LowestId`) rather than being rejected the way the
/// setup wizard's `parse_tech_entry` call does. Renders its OWN
/// deck-flavored wording from `TechEntryError` (fix wave, review of commit
/// 0a01c95 finding 2: these exact messages regressed to the wizard's
/// registry-flavored wording when the two were first unified -- "not
/// currently in the tech deck" matters here because a card can be real but
/// already drawn, a materially different fact from "not a valid tech
/// card"). `Ambiguous` is unreachable under `LowestId` (kept as a safe,
/// non-panicking fallback rather than `unreachable!()` in case that ever
/// changes).
pub(crate) fn parse_tech_reveal(words: &[&str], deck: &[TechCardId], techs: &TechRegistry) -> Result<TechCardId, String> {
    if words.len() != 1 {
        return Err("Enter exactly one tech code (e.g. 'T05' or a name), or 'cancel'.".to_string());
    }
    let tok = words[0];
    parse_tech_entry(tok, techs, deck, TechAmbiguity::LowestId).map_err(|e| match e {
        TechEntryError::NotInPool(n) => format!("T{n:02} is not currently in the tech deck."),
        TechEntryError::NoMatch | TechEntryError::Ambiguous => {
            format!("'{tok}' does not match any tech card currently in the deck.")
        }
    })
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
        r.step("0");
        r.step(&format!("a {}", port_area.0));
        let k = r.session.legal().iter().position(|m| matches!(
            m, Move::PlacePresident { destination } if *destination == port_area
        )).expect("PlacePresident at Port A is legal from hand at game start");
        r.step(&k.to_string());
        let take = r.session.legal().iter().position(|m| matches!(
            m, Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } } if cards_taken.len() == 1
        )).expect("1-card port take available");
        let out = r.step(&take.to_string());
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
        assert!(r.session.state().lab_state.lab_a.slots.contains(&Some(top_tech)));
        assert_eq!(replay(r.session.record(), r.session.rules()).unwrap(), *r.session.state());
    }

    // The three reveal-answer parsers below are pure functions, so their
    // junk-input handling is tested directly rather than through a full
    // Repl/Session (cheaper, and pins the exact rejection/resolution rules
    // rather than just "the REPL didn't crash").

    #[test]
    fn parse_order_reveal_never_panics_on_junk_and_rejects_correctly() {
        let deck = vec![OrderCardId(5), OrderCardId(9)];
        let boxed = vec![OrderCardId(30)]; // physically real, boxed at setup
        let none: Vec<OrderCardId> = vec![];
        assert!(parse_order_reveal(&[], &deck, &boxed, &none).is_err()); // empty input line
        assert!(parse_order_reveal(&["O"], &deck, &boxed, &none).is_err());
        assert!(parse_order_reveal(&["O99999"], &deck, &boxed, &none).is_err()); // overflows u8
        assert!(parse_order_reveal(&["🎉"], &deck, &boxed, &none).is_err());
        assert!(parse_order_reveal(&["O1"], &deck, &boxed, &none).is_err()); // valid code, in neither pile
        assert!(parse_order_reveal(&["O5", "O9"], &deck, &boxed, &none).is_err()); // too many tokens
        assert_eq!(parse_order_reveal(&["O5"], &deck, &boxed, &none), Ok(OrderCardId(5)));
        assert_eq!(parse_order_reveal(&["9"], &deck, &boxed, &none), Ok(OrderCardId(9))); // bare digits, no 'O'
        // A boxed/removed card is a valid reveal (conservation-by-exchange):
        assert_eq!(parse_order_reveal(&["O30"], &deck, &boxed, &none), Ok(OrderCardId(30)));
        let already = vec![OrderCardId(5)];
        assert!(parse_order_reveal(&["O5"], &deck, &boxed, &already).is_err(), "a card can't be drawn twice");
    }

    #[test]
    fn parse_tech_reveal_never_panics_on_junk_and_resolves_name_prefix_to_lowest_id() {
        let rules = Rules::standard();
        // TechCardId(11) and (12) are both named "Ball" (data.rs) -- one of
        // the 10 titles that circulate as 2 physical copies.
        let ball_deck = vec![TechCardId(11), TechCardId(12)];
        assert!(parse_tech_reveal(&[], &ball_deck, &rules.techs).is_err()); // empty input line
        assert!(parse_tech_reveal(&["T"], &ball_deck, &rules.techs).is_err()); // no T-named card in THIS deck
        assert!(parse_tech_reveal(&["🎉"], &ball_deck, &rules.techs).is_err());
        assert!(parse_tech_reveal(&["Zzzz"], &ball_deck, &rules.techs).is_err()); // no name match
        assert_eq!(parse_tech_reveal(&["ball"], &ball_deck, &rules.techs), Ok(TechCardId(11)), "ambiguous name resolves to the LOWEST id");
        assert_eq!(parse_tech_reveal(&["T12"], &ball_deck, &rules.techs), Ok(TechCardId(12)));

        // Message texts pinned (fix wave, review of commit 0a01c95 finding
        // 2): a live-reveal rejection must talk about the DECK, not the
        // registry -- a card can be real but already drawn, which is a
        // different fact from "not a valid tech card" (the wizard's
        // registry-flavored wording, which must NOT leak in here).
        let not_in_deck = parse_tech_reveal(&["T99"], &ball_deck, &rules.techs).unwrap_err(); // valid code, not in deck
        assert_eq!(not_in_deck, "T99 is not currently in the tech deck.", "{not_in_deck}");
        let no_match = parse_tech_reveal(&["Zzzz"], &ball_deck, &rules.techs).unwrap_err();
        assert_eq!(no_match, "'Zzzz' does not match any tech card currently in the deck.", "{no_match}");
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
