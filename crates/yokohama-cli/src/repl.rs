//! Pure REPL step function over a `Session`: numbered-option prompts,
//! placement grammar, view/utility commands, pagination, and live-mode
//! reveal choreography (design docs/plans/2026-07-18-phase2-cli-driver-design.md).
//! No stdin/stdout -- `main.rs` (Task 13) drives this with real I/O.

use yokohama_core::*;

use crate::session::TrackingMode;
use crate::{describe, render};

/// Live-mode reveal-collection state: while a group is being collected,
/// `step` routes input to `collect_reveal` instead of normal command
/// dispatch (see `step`'s pending check) and `out` stops appending the
/// normal legal-options prompt (see `out`'s pending branch). `mv` is the
/// move already confirmed legal against a scratch-applied post-reveal
/// state (`commit`'s `apply_move` call) -- nothing is recorded until the
/// group completes and runs through `Session::apply` for real. Always
/// `None` in `TrackingMode::Simulation`.
enum Pending {
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

/// One REPL turn's worth of state layered over a `Session`: which options
/// page is showing, any in-progress live-mode reveal collection, and
/// whether a driver/engine fault has locked the session down.
pub struct Repl {
    pub session: crate::session::Session,
    page: usize,
    pending: Pending,
    /// Set once `finish_apply` sees an `INVARIANT VIOLATION` error from
    /// `Session::apply`, or once a just-completed live order-reveal group
    /// fails the post-apply desync assertion (`lock_on_order_desync`) --
    /// from then on `step` refuses everything except `save`/`log`/`quit`,
    /// so a desynced session can still be inspected and saved for
    /// postmortem but never played further.
    locked: bool,
}

/// The result of one `Repl::step` call: the text to show the user (which
/// ends with the next prompt, unless `quit`), and whether the REPL should
/// exit.
pub struct StepOutput {
    pub text: String,
    pub quit: bool,
}

impl Repl {
    pub fn new(session: crate::session::Session) -> Repl {
        Repl { session, page: 0, pending: Pending::None, locked: false }
    }

    /// The whole prompt screen for the current state: header line, options
    /// page (or a game-over/no-options notice), and an input hint. Legal
    /// options are numbered by their GLOBAL 0-based index into
    /// `session.legal()`, 20 per page.
    pub fn prompt(&self) -> String {
        let state = self.session.state();
        let mut out = render::prompt_header(state);
        out.push('\n');
        if self.session.is_over() {
            out.push_str("game over — commands only\n");
        } else {
            let legal = self.session.legal();
            let n = legal.len();
            if n == 0 {
                out.push_str("(no legal options right now -- commands only)\n");
            } else {
                let start = (self.page * 20).min(n);
                let end = (start + 20).min(n);
                for (i, mv) in legal[start..end].iter().enumerate() {
                    out.push_str(&format!(
                        "[{}] {}\n",
                        start + i,
                        describe::describe_move(mv, state, self.session.rules())
                    ));
                }
                if end < n {
                    out.push_str(&format!("...{} more (type 'more')\n", n - end));
                }
            }
        }
        out.push_str("> enter a number, 'a'/'b' for placement, or a command ('help' for the list)\n");
        out
    }

    /// One REPL interaction: parse `line`, act on `session`, return output
    /// text. Never panics -- every parse path is guarded (digit-only checks
    /// before `parse`, `Result` matches instead of `unwrap`).
    pub fn step(&mut self, line: &str) -> StepOutput {
        let line = line.trim();
        let words: Vec<&str> = line.split_whitespace().collect();
        // Tokenized once, up front, so the lockdown allow-list sees exactly
        // what the dispatch below sees (same split_whitespace tokens) --
        // matching against the raw `line` string here previously missed the
        // `q` alias `["quit"] | ["q"]` honors below, and could disagree with
        // the dispatch on anything but single-space-separated input.
        if self.locked && !matches!(words.as_slice(), ["save"] | ["save", _] | ["log"] | ["quit"] | ["q"]) {
            return self.out(
                "session locked after a driver/engine fault — only save/log/quit".to_string(),
                false,
            );
        }
        // While a live-mode reveal group is being collected, input goes to
        // the collector instead of normal dispatch -- `cancel` and
        // `quit`/`q` are the only two escapes; any other token (even a
        // normally-valid command like `undo` or `save`) is reveal input,
        // and a bad one gets a friendly re-prompt rather than falling
        // through to command dispatch.
        if !matches!(self.pending, Pending::None) {
            return match words.as_slice() {
                ["cancel"] => {
                    self.pending = Pending::None;
                    self.out("Reveal collection cancelled -- no move was applied.\n".to_string(), false)
                }
                ["quit"] | ["q"] => StepOutput { text: "bye\n".to_string(), quit: true },
                _ => self.collect_reveal(&words),
            };
        }
        match words.as_slice() {
            [] => self.out(String::new(), false),
            ["quit"] | ["q"] => StepOutput { text: "bye\n".to_string(), quit: true },
            ["help"] => self.out(HELP_TEXT.to_string(), false),
            ["board"] => self.out(render::board_grid(self.session.state()), false),
            ["players"] => self.out(render::players_panel(self.session.state(), self.session.rules()), false),
            ["ports"] => self.out(render::ports_view(self.session.state(), self.session.rules()), false),
            ["labs"] => self.out(render::labs_view(self.session.state(), self.session.rules()), false),
            ["tracks"] => self.out(render::tracks_view(self.session.state()), false),
            ["hand"] => self.out(self.hand_text(), false),
            ["score"] => self.out(self.score_text(), false),
            ["log"] => self.out(self.log_tail(), false),
            ["more"] => {
                self.advance_page();
                self.out(String::new(), false)
            }
            ["list"] => {
                self.page = 0;
                self.out(String::new(), false)
            }
            ["undo"] => match self.session.undo() {
                Ok(()) => {
                    self.page = 0;
                    self.out("Undid last entry.\n".to_string(), false)
                }
                Err(e) => self.out(format!("Cannot undo: {e}\n"), false),
            },
            ["save"] => self.save_cmd(None),
            ["save", _] => self.save_cmd(Some(words[1])),
            ["reveal", seat_tok, card_tok] => self.reveal_cmd(seat_tok, card_tok),
            ["reveal", ..] => self.out("Usage: reveal <seat> <O..>\n".to_string(), false),
            ["a", rest @ ..] => self.try_placement_a(rest),
            ["b", rest @ ..] => self.try_placement_b(rest),
            [_] if words[0].chars().all(|c| c.is_ascii_digit()) => match words[0].parse::<usize>() {
                Ok(k) => self.pick_option(k),
                Err(_) => self.out(format!("'{}' is not a valid option number.\n", words[0]), false),
            },
            _ => self.out("Unrecognized input — type 'help'.\n".to_string(), false),
        }
    }

    /// Wraps `text` with the next prompt (normal case), a minimal lockdown
    /// status line (once `locked`), or nothing at all (`quit`, or while a
    /// live-mode reveal is pending). `reveal_prompt()` is a complete,
    /// self-describing message, so every caller that needs it (entering or
    /// continuing a reveal collection) passes it as `text` directly rather
    /// than relying on this wrapper to append it.
    fn out(&self, mut text: String, quit: bool) -> StepOutput {
        if quit {
            return StepOutput { text, quit: true };
        }
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        if self.locked {
            text.push_str(&self.lockdown_status());
        } else if matches!(self.pending, Pending::None) {
            text.push_str(&self.prompt());
        }
        StepOutput { text, quit: false }
    }

    fn lockdown_status(&self) -> String {
        format!(
            "[locked] turn {} | seed {} | {} logged event(s) -- only 'save', 'log', 'quit' available\n",
            self.session.state().turn_number,
            self.session.record().seed,
            self.session.record().events.len(),
        )
    }

    fn advance_page(&mut self) {
        let n = self.session.legal().len();
        let total_pages = n.div_ceil(20).max(1);
        self.page = (self.page + 1) % total_pages;
    }

    fn pick_option(&mut self, k: usize) -> StepOutput {
        let legal = self.session.legal();
        if k >= legal.len() {
            return self.out(format!("option {k} out of range (0..{})\n", legal.len()), false);
        }
        let mv = legal[k].clone();
        self.commit(mv)
    }

    /// Parse `a [id id id]` (0-3 area ids) into a `PlaceThreeAreas`
    /// candidate and match it against `legal()` by placement-multiset
    /// equality (the generator's internal ordering must not matter).
    fn try_placement_a(&mut self, rest: &[&str]) -> StepOutput {
        if rest.len() > 3 {
            return self.out(format!("Option A takes at most 3 area ids, got {}.\n", rest.len()), false);
        }
        let mut placements = Vec::with_capacity(rest.len());
        for tok in rest {
            match parse_area_id(tok) {
                Ok(id) => placements.push(id),
                Err(e) => return self.out(format!("{e}\n"), false),
            }
        }
        let candidate = Move::PlaceThreeAreas { placements };
        match self.find_placement_match(&candidate) {
            Some(mv) => self.commit(mv),
            None => self.out("Illegal: that placement is not currently available.\n".to_string(), false),
        }
    }

    /// Parse `b <id>` / `b <id> t <adj>` into a `PlaceTwoSameArea` candidate
    /// and match it against `legal()`.
    fn try_placement_b(&mut self, rest: &[&str]) -> StepOutput {
        if rest.is_empty() || rest.len() == 2 || rest.len() > 3 {
            return self.out("Usage: 'b <id>' or 'b <id> t <adj>'.\n".to_string(), false);
        }
        let area = match parse_area_id(rest[0]) {
            Ok(id) => id,
            Err(e) => return self.out(format!("{e}\n"), false),
        };
        let adjacent_bonus = if rest.len() == 3 {
            if rest[1] != "t" {
                return self.out("Usage: 'b <id> t <adj>'.\n".to_string(), false);
            }
            match parse_area_id(rest[2]) {
                Ok(id) => Some(id),
                Err(e) => return self.out(format!("{e}\n"), false),
            }
        } else {
            None
        };
        let candidate = Move::PlaceTwoSameArea { area, adjacent_bonus };
        match self.find_placement_match(&candidate) {
            Some(mv) => self.commit(mv),
            None => self.out("Illegal: that placement is not currently available.\n".to_string(), false),
        }
    }

    /// Find the legal move matching `candidate`: `PlaceThreeAreas` matches
    /// by placement multiset (order-independent), everything else by exact
    /// equality.
    fn find_placement_match(&self, candidate: &Move) -> Option<Move> {
        let key = |v: &Vec<AreaId>| {
            let mut k: Vec<u8> = v.iter().map(|a| a.0).collect();
            k.sort_unstable();
            k
        };
        self.session.legal().into_iter().find(|m| match (m, candidate) {
            (Move::PlaceThreeAreas { placements: a }, Move::PlaceThreeAreas { placements: b }) => key(a) == key(b),
            (m, c) => m == c,
        })
    }

    /// Route `mv` through live-mode reveal detection (`TrackingMode::Live`
    /// only), then apply. A scratch-apply against the CURRENT state (never
    /// mutates -- `apply_move` always clones) reveals whether `mv` draws
    /// from either deck or claims a 5-Power token; when it does, `mv` is
    /// parked in `self.pending` and the operator is prompted instead of
    /// applying immediately. Everything else -- including every move in
    /// `TrackingMode::Simulation` -- applies at once via `finish_apply`.
    fn commit(&mut self, mv: Move) -> StepOutput {
        if matches!(self.session.mode, TrackingMode::Live) {
            match apply_move(self.session.state(), &mv, self.session.rules()) {
                Err(e) => return self.out(format!("Illegal: {e}\n"), false),
                Ok(scratch) => {
                    let d_orders = self.session.state().order_deck.len() - scratch.order_deck.len();
                    let d_techs = self.session.state().tech_deck.len() - scratch.tech_deck.len();
                    if d_orders > 0 {
                        let (slots, board_b) = refilled_port_slots(self.session.state(), &scratch);
                        self.pending = Pending::OrderReveals { mv, slots, board_b, collected: Vec::new() };
                        return self.out(self.reveal_prompt(), false);
                    }
                    if d_techs > 0 {
                        let board_b = lab_b_changed(self.session.state(), &scratch);
                        self.pending = Pending::TechReveal { mv, board_b };
                        return self.out(self.reveal_prompt(), false);
                    }
                    if matches!(mv, Move::Take5PowerToken) {
                        if let Some(area) = claimed_token_area(self.session.state(), &scratch) {
                            self.pending = Pending::TokenReveal { mv, area };
                            return self.out(self.reveal_prompt(), false);
                        }
                    }
                }
            }
        }
        self.finish_apply(mv, vec![])
    }

    /// Apply `mv` + `reveals` as one atomic group (`Session::apply`). On
    /// success, reset pagination and report the delta (plus a final-score
    /// table if the game just ended). On failure, an `INVARIANT VIOLATION`
    /// locks the session down with a postmortem dump; anything else is an
    /// ordinary illegal-move rejection. This is the sim path's entire
    /// `commit` (reveals always empty there) and the terminal step of
    /// every live-mode reveal collection.
    fn finish_apply(&mut self, mv: Move, reveals: Vec<RecordedEvent>) -> StepOutput {
        match self.session.apply(mv, reveals) {
            Ok(delta) => {
                self.page = 0;
                let mut text = delta.join("\n");
                if self.session.is_over() {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&self.final_score_table());
                }
                self.out(text, false)
            }
            Err(e) => {
                if e.starts_with("INVARIANT VIOLATION") {
                    self.locked = true;
                    let mut text = format!("LOCKDOWN -- {e}\n");
                    text.push_str(&self.lockdown_dump());
                    self.out(text, false)
                } else {
                    self.out(format!("Illegal: {e}\n"), false)
                }
            }
        }
    }

    /// The complete message for whatever reveal is currently pending: what
    /// changed and exactly what to type (or an empty string when nothing
    /// is pending). Slots are always shown 1-based; Port slots also carry
    /// their zone, matching `render::ports_view`'s labeling. Prompts walk
    /// `OrderReveals`' `slots` in the order they're stored (DESCENDING --
    /// `commit` sorted them that way), one per `collected.len()`.
    fn reveal_prompt(&self) -> String {
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
    fn collect_reveal(&mut self, words: &[&str]) -> StepOutput {
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
                            let out = self.finish_apply(mv, vec![RecordedEvent::RevealOrders(collected.clone())]);
                            self.lock_on_order_desync(&slots, &collected, board_b, out)
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
                    Ok(card) => self.finish_apply(mv, vec![RecordedEvent::RevealTechs(vec![card])]),
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
    /// No-ops when `finish_apply` already failed or locked -- nothing to
    /// re-check against a move that never actually applied.
    fn lock_on_order_desync(
        &mut self,
        slots: &[usize],
        collected: &[OrderCardId],
        board_b: bool,
        out: StepOutput,
    ) -> StepOutput {
        if out.quit || self.locked || out.text.contains("Illegal:") {
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

    /// `reveal <seat> <card>`: make a live-tracked opponent's hidden
    /// starting order public via `Session::apply_standalone`. Unlike the
    /// bound reveals above, this is a normal top-level command usable any
    /// time outside lockdown/collection -- it isn't triggered by a move.
    fn reveal_cmd(&mut self, seat_tok: &str, card_tok: &str) -> StepOutput {
        let seat = match seat_tok.parse::<u8>().ok().and_then(|n| PlayerId::try_new(n).ok()) {
            Some(p) => p,
            None => return self.out(format!("'{seat_tok}' is not a valid seat number.\n"), false),
        };
        let card = match parse_order_id(card_tok) {
            Some(n) => OrderCardId(n),
            None => return self.out(format!("'{card_tok}' is not a valid order-card code.\n"), false),
        };
        match self.session.apply_standalone(RecordedEvent::RevealStartingOrder { player: seat, card }) {
            Ok(_delta) => {
                self.out(format!("Seat {} revealed O{:02} as their starting order.\n", seat.0, card.0), false)
            }
            Err(e) => self.out(format!("Illegal: {e}\n"), false),
        }
    }

    /// `target`'s `save_path` update only happens on the success arm below --
    /// setting it eagerly (before the write is confirmed) would let a
    /// typo'd path (e.g. a nonexistent directory) silently redirect every
    /// future autosave at a target that can never be written.
    fn save_cmd(&mut self, path: Option<&str>) -> StepOutput {
        let target = match path {
            Some(p) => std::path::PathBuf::from(p),
            None => match self.session.save_path.clone() {
                Some(p) => p,
                None => return self.out("No save path set -- use 'save <path>'.\n".to_string(), false),
            },
        };
        match self.session.save(&target) {
            Ok(()) => {
                self.session.save_path = Some(target.clone());
                self.out(format!("Saved to {}\n", target.display()), false)
            }
            Err(e) => self.out(format!("Save failed: {e}\n"), false),
        }
    }

    fn hand_text(&self) -> String {
        let state = self.session.state();
        let rules = self.session.rules();
        let p = state.current_player();
        let mut out = format!("{:?} hand:\n", p.color);
        if p.order_cards_in_hand.is_empty() {
            out.push_str("  (empty)\n");
        } else {
            for id in &p.order_cards_in_hand {
                let line = rules.orders.get(*id).map(order_compact_line).unwrap_or_else(|| format!("O{:02}", id.0));
                out.push_str(&format!("  {line}\n"));
            }
        }
        out
    }

    /// Current VP + money per player; once `is_over`, the authoritative
    /// `final_scores()` table instead.
    fn score_text(&self) -> String {
        if self.session.is_over() {
            self.final_score_table()
        } else {
            let mut out = String::from("Score:\n");
            for p in &self.session.state().players {
                out.push_str(&format!("  {:?}: {} VP, {} yen\n", p.color, p.victory_points, p.money));
            }
            out
        }
    }

    fn final_score_table(&self) -> String {
        let mut out = String::from("Final scores:\n");
        for (pid, score) in self.session.final_scores() {
            out.push_str(&format!("  {:?}: {} VP\n", self.session.state().player(pid).color, score));
        }
        out
    }

    /// Last 10 logged events (moves and reveal reconciliations) in
    /// plain words, each with its global index into `record().events`.
    fn log_tail(&self) -> String {
        let events = &self.session.record().events;
        let n = events.len();
        let start = n.saturating_sub(10);
        let mut out = format!("log ({} of {} event(s)):\n", n - start, n);
        for (i, ev) in events[start..].iter().enumerate() {
            out.push_str(&format!("  {}: {}\n", start + i, self.describe_event(ev)));
        }
        out
    }

    fn describe_event(&self, ev: &RecordedEvent) -> String {
        match ev {
            RecordedEvent::Move(mv) => describe::describe_move(mv, self.session.state(), self.session.rules()),
            RecordedEvent::RevealOrders(cards) => format!("reveal orders: {} card(s)", cards.len()),
            RecordedEvent::RevealTechs(cards) => format!("reveal techs: {} card(s)", cards.len()),
            RecordedEvent::RevealStartingOrder { player, card } => {
                format!("reveal starting order: P{} = O{:02}", player.0, card.0)
            }
            RecordedEvent::RevealFivePowerToken { area, effect } => {
                format!("reveal 5-power at A{:02}: {:?}", area.0, effect)
            }
        }
    }

    fn lockdown_dump(&self) -> String {
        format!(
            "seed: {}\nturn: {}\n{}",
            self.session.record().seed,
            self.session.state().turn_number,
            self.log_tail(),
        )
    }
}

/// Parse an area-id token with a friendly, non-panicking error: rejects
/// non-numeric tokens and numbers outside AreaId's 0-17 range identically
/// (a token like `"999999999999999999999"` fails `u8::parse`, not `unwrap`).
fn parse_area_id(tok: &str) -> Result<AreaId, String> {
    match tok.parse::<u8>().ok().and_then(|n| AreaId::try_new(n).ok()) {
        Some(id) => Ok(id),
        None => Err(format!("'{tok}' is not a valid area id (0-17)")),
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
fn refilled_port_slots(before: &GameState, after: &GameState) -> (Vec<usize>, bool) {
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
fn lab_b_changed(before: &GameState, after: &GameState) -> bool {
    match (&before.lab_state.lab_b, &after.lab_state.lab_b) {
        (Some(b0), Some(b1)) => b0.slots != b1.slots,
        _ => false,
    }
}

/// The area whose 5-Power token just went from unclaimed to claimed.
/// `Move::Take5PowerToken` is only ever legal when the President's area
/// holds one (`generate_five_power_moves`), so this is `Some` at every real
/// call site; `None` is a defensive fallback that's never expected to fire.
fn claimed_token_area(before: &GameState, after: &GameState) -> Option<AreaId> {
    before.board_layout.five_power_tokens.iter().find_map(|(area, effect)| {
        (effect.is_some() && after.board_layout.five_power_tokens.get(area) == Some(&None)).then_some(*area)
    })
}

/// Parse an order-card token ("O17" or "17") into its numeric id -- no
/// validation against any deck; callers check membership themselves.
fn parse_order_id(tok: &str) -> Option<u8> {
    tok.strip_prefix(['O', 'o']).unwrap_or(tok).parse::<u8>().ok()
}

/// Parse one order-reveal answer: a real numeric id, currently in `deck`,
/// and not already in `collected` (a card can't be drawn twice in one refill).
fn parse_order_reveal(words: &[&str], deck: &[OrderCardId], collected: &[OrderCardId]) -> Result<OrderCardId, String> {
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
/// currently in `deck` (some titles circulate as 2 physical copies).
fn parse_tech_reveal(words: &[&str], deck: &[TechCardId], techs: &TechRegistry) -> Result<TechCardId, String> {
    if words.len() != 1 {
        return Err("Enter exactly one tech code (e.g. 'T05' or a name), or 'cancel'.".to_string());
    }
    let tok = words[0];
    if let Some(rest) = tok.strip_prefix(['T', 't']) {
        if let Ok(n) = rest.parse::<u8>() {
            let card = TechCardId(n);
            return if deck.contains(&card) {
                Ok(card)
            } else {
                Err(format!("T{n:02} is not currently in the tech deck."))
            };
        }
    }
    let needle = tok.to_lowercase();
    let mut matches: Vec<TechCardId> = deck
        .iter()
        .copied()
        .filter(|id| techs.get(*id).is_some_and(|c| c.name.to_lowercase().starts_with(&needle)))
        .collect();
    matches.sort_unstable_by_key(|id| id.0);
    matches.into_iter().next().ok_or_else(|| format!("'{tok}' does not match any tech card currently in the deck."))
}

/// Parse a 5-Power token-effect letter into the canonical pool's fixed
/// magnitude for that kind -- one letter per `FivePowerEffect` variant (the
/// phantom `VictoryPoints`/`Movement` variants were removed once the
/// physical box check confirmed only these six faces exist).
fn parse_token_letter(words: &[&str]) -> Result<FivePowerEffect, String> {
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

const HELP_TEXT: &str = "\
Commands:
  <number>        apply the numbered legal option
  a [id id id]    Placement Option A: place 1 assistant in each of 0-3 areas
  b <id>          Placement Option B: place 2 assistants in one area
  b <id> t <adj>  Placement Option B with the Telegram bonus: +1 assistant in an adjacent area
  board           show the area grid
  players         show each player's resources, hand, tech, tokens, President
  ports           show both Port boards and the order-deck size
  labs            show both Laboratory boards and the tech-deck size
  tracks          show the Church and Customs tracks
  hand            show the current player's order-card hand
  score           show current VP/money (or the final scores once the game is over)
  log             show the tail of the move/event log
  more            show the next page of legal options (wraps to page 0)
  list            reset to the first page of legal options
  undo            undo the last move (and any reveals bound to it)
  save [path]     save to path (or to the last-used path if omitted)
  reveal <seat> <O..>  live mode: make a hidden opponent starting order public
  cancel          live mode: abort an in-progress card/token reveal prompt
  help            show this text
  quit            exit the REPL
";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;
    // `repl`'s own top-level already glob-imports yokohama_core (needed by
    // the impl code), and `super::*` carries that into this module too --
    // this second import is the brief's Step-1 test text verbatim, kept
    // as-is and just silenced rather than diverging from the spec (same
    // precedent as `session::tests`).
    #[allow(unused_imports)]
    use yokohama_core::*;

    fn repl_2p(seed: u64) -> Repl {
        Repl::new(Session::new_sim(
            GameConfig::for_player_count(2), seed,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        ).unwrap())
    }

    #[test]
    fn prompt_numbers_the_legal_options() {
        let r = repl_2p(4);
        let p = r.prompt();
        assert!(p.contains("[0]"), "{p}");
        assert!(p.contains("End additional-action phase"), "{p}");
    }

    #[test]
    fn numeric_input_applies_that_option_and_reprompts() {
        let mut r = repl_2p(4);
        let phase_before = r.session.state().turn_phase;
        let out = r.step("0");
        assert!(!out.quit);
        assert_ne!(r.session.state().turn_phase, phase_before);
        assert!(out.text.contains("Placement"), "{}", out.text); // delta line + new prompt
    }

    #[test]
    fn placement_grammar_matches_by_multiset() {
        let mut r = repl_2p(4);
        r.step("0"); // -> Placement
        let legal = r.session.legal();
        let some_a = legal.iter().find_map(|m| match m {
            Move::PlaceThreeAreas { placements } if placements.len() == 2 => Some(placements.clone()),
            _ => None,
        }).expect("a 2-area Option A exists");
        // Enter the two areas in REVERSED order — must still match.
        let line = format!("a {} {}", some_a[1].0, some_a[0].0);
        let events_before = r.session.record().events.len();
        r.step(&line);
        assert_eq!(r.session.record().events.len(), events_before + 1);
    }

    #[test]
    fn bad_input_and_illegal_grammar_are_rejected_gently() {
        let mut r = repl_2p(4);
        let out = r.step("banana");
        assert!(out.text.contains("Unrecognized"), "{}", out.text);
        let out = r.step("a 99"); // AreaId out of range
        assert!(out.text.to_lowercase().contains("area"), "{}", out.text);
        let out = r.step("999"); // option index out of range
        assert!(out.text.contains("out of range"), "{}", out.text);
    }

    #[test]
    fn undo_save_quit_commands_work() {
        let mut r = repl_2p(4);
        r.step("0");
        let out = r.step("undo");
        assert!(out.text.contains("Undid"), "{}", out.text);
        assert!(r.session.record().events.is_empty());
        let path = std::env::temp_dir().join(format!("yokohama_repl_{}.json", std::process::id()));
        let out = r.step(&format!("save {}", path.display()));
        assert!(out.text.contains("Saved"), "{}", out.text);
        assert!(path.exists());
        std::fs::remove_file(&path).ok();
        assert!(r.step("quit").quit);
    }

    /// Regression for `save_cmd` setting `save_path` BEFORE the save is
    /// confirmed to succeed (final-review ledger-minor bundled with the
    /// autosave-atomicity Important finding): a typo'd `save <bad path>`
    /// must report the failure and leave `save_path` exactly as it was, so
    /// every future autosave doesn't get silently redirected at a target
    /// that can never be written.
    #[test]
    fn save_cmd_failure_does_not_clobber_save_path() {
        let mut r = repl_2p(4);
        let original = std::env::temp_dir().join(format!("yokohama_repl_original_{}.json", std::process::id()));
        r.session.save_path = Some(original.clone());
        let bad = format!("/nonexistent-dir-yokohama-test-{}/should-fail.json", std::process::id());
        let out = r.step(&format!("save {bad}"));
        assert!(out.text.contains("Save failed"), "{}", out.text);
        assert_eq!(
            r.session.save_path,
            Some(original),
            "a failed save must not redirect future autosaves at a bad path"
        );
    }

    #[test]
    fn pagination_pages_through_large_placement_lists() {
        let mut r = repl_2p(4);
        r.step("0");
        let n = r.session.legal().len();
        if n > 20 {
            let p0 = r.prompt();
            assert!(p0.contains("more"), "{p0}");
            let p1 = r.step("more").text;
            assert_ne!(p0, p1);
        }
    }

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

    #[test]
    fn reveal_command_makes_hidden_starting_order_public() {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let pending = new_game(config.clone(), &rules, 902).unwrap();
        let hidden = pending.deals[1][0];
        let session = Session::new_live(
            config, 902, SetupObservations::default(),
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown],
            rules,
        ).unwrap();
        let mut r = Repl::new(session);
        let out = r.step(&format!("reveal 1 O{}", hidden.0));
        assert!(out.text.contains("revealed"), "{}", out.text);
        assert_eq!(r.session.state().players[1].starting_order, Some(hidden));
        assert_eq!(replay(r.session.record(), r.session.rules()).unwrap(), *r.session.state());
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

    /// Sim-mode guard (brief's parenthetical requirement): the identical
    /// 1-card Port take that triggers reveal-prompting in `TrackingMode::Live`
    /// applies immediately in `TrackingMode::Simulation` -- no prompt, no
    /// pending collection, exactly one new event (the Move alone).
    #[test]
    fn sim_mode_port_take_applies_immediately_with_no_reveal_prompt() {
        let mut r = repl_2p(4);
        // See live_port_take_collects_reveals_then_applies_them's comment:
        // an assistant must land at Port A before PlacePresident can target it.
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
        let events_before = r.session.record().events.len();
        let out = r.step(&take.to_string());
        assert!(!out.text.to_lowercase().contains("slot"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert!(matches!(r.pending, Pending::None));
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

    /// Lockdown restricts to exactly `save`/`log`/`quit` (and the `q` alias
    /// the normal dispatch already honors) -- view commands and numeric
    /// picks are rejected and change nothing, while `log`/`save` still work.
    #[test]
    fn locked_mode_allows_only_save_log_quit_and_honors_q_alias() {
        let mut r = repl_2p(4);
        r.locked = true;
        let events_before = r.session.record().events.len();
        let state_before = r.session.state().clone();

        let out = r.step("board");
        assert!(out.text.contains("locked"), "{}", out.text);
        assert_eq!(*r.session.state(), state_before);
        assert_eq!(r.session.record().events.len(), events_before);

        let out = r.step("0");
        assert!(out.text.contains("locked"), "{}", out.text);
        assert_eq!(*r.session.state(), state_before);
        assert_eq!(r.session.record().events.len(), events_before);

        let out = r.step("log");
        assert!(!out.text.contains("only save/log/quit"), "{}", out.text);

        let path = std::env::temp_dir().join(format!("yokohama_repl_locked_{}.json", std::process::id()));
        let out = r.step(&format!("save {}", path.display()));
        assert!(out.text.contains("Saved"), "{}", out.text);
        assert!(path.exists());
        std::fs::remove_file(&path).ok();

        assert!(r.step("q").quit, "'q' must quit while locked, same as 'quit'");
    }
}
