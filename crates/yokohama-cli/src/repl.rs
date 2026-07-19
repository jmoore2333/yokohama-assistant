//! Pure REPL step function over a `Session`: numbered-option prompts,
//! placement grammar, view/utility commands, pagination, and live-mode
//! reveal choreography (design docs/plans/2026-07-18-phase2-cli-driver-design.md).
//! No stdin/stdout -- `main.rs` (Task 13) drives this with real I/O.

use yokohama_core::*;

use crate::reveal::{claimed_token_area, lab_b_changed, refilled_port_slots, Pending};
use crate::session::TrackingMode;
use crate::{describe, render};

/// One REPL turn's worth of state layered over a `Session`: which options
/// page is showing, any in-progress live-mode reveal collection, and
/// whether a driver/engine fault has locked the session down.
pub struct Repl {
    pub session: crate::session::Session,
    page: usize,
    pub(crate) pending: Pending,
    /// Set once `finish_apply` sees an `INVARIANT VIOLATION` error from
    /// `Session::apply`, or once a just-completed live order-reveal group
    /// fails the post-apply desync assertion (`lock_on_order_desync`) --
    /// from then on `step` refuses everything except `save`/`log`/`quit`,
    /// so a desynced session can still be inspected and saved for
    /// postmortem but never played further.
    pub(crate) locked: bool,
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
        let phase_text = match state.pending_choices.first() {
            Some(head) => {
                let n = state.pending_choices.len();
                format!(
                    "Choose: {} — {n} choice{} pending",
                    describe::describe_pending_choice(head, state, self.session.rules()),
                    if n == 1 { "" } else { "s" },
                )
            }
            None => describe::phase_name(state.turn_phase).to_string(),
        };
        let mut out = render::prompt_header(state, &phase_text);
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
            ["m", src, dst] => self.try_assistant_move(src, dst),
            ["m", ..] => self.out("Usage: m <from> <to>\n".to_string(), false),
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
    pub(crate) fn out(&self, mut text: String, quit: bool) -> StepOutput {
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

    pub(crate) fn lockdown_status(&self) -> String {
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

    /// Parse `m <src> <dst>` (`src` is `hand` or an area id) into a
    /// `ChooseAssistantMove` candidate and exact-match it against `legal()`
    /// via `find_placement_match` (its catch-all arm already does plain
    /// equality for every non-`PlaceThreeAreas` move, so this reuses it
    /// as-is rather than duplicating the lookup). Area tokens go through
    /// `parse_raw_area_token`, not `parse_area_id` -- an out-of-range
    /// destination like `99` must fall through to the SAME "not one of the
    /// legal assistant moves" rejection an on-board-but-wrong area gets,
    /// not a separate range error, and `AreaId::new`'s panic-on->=18 rules
    /// out using it directly on raw user input.
    fn try_assistant_move(&mut self, src: &str, dst: &str) -> StepOutput {
        let from = if src == "hand" {
            MoveSource::Hand
        } else {
            match parse_raw_area_token(src) {
                Ok(id) => MoveSource::Area(id),
                Err(e) => return self.out(format!("{e}\n"), false),
            }
        };
        let to = match parse_raw_area_token(dst) {
            Ok(id) => id,
            Err(e) => return self.out(format!("{e}\n"), false),
        };
        let candidate = Move::ChooseAssistantMove { from, to };
        match self.find_placement_match(&candidate) {
            Some(mv) => self.commit(mv),
            None => self.out("Illegal: that is not one of the legal assistant moves.\n".to_string(), false),
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
    pub(crate) fn finish_apply(&mut self, mv: Move, reveals: Vec<RecordedEvent>) -> StepOutput {
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

/// Parse an `m`-grammar area token into a raw, possibly off-board `AreaId`
/// for legal-move membership testing. Unlike `parse_area_id`, values >=18
/// are NOT rejected here as a range error -- `AreaId::unchecked` can't
/// panic on them, so a destination like `99` reaches `try_assistant_move`'s
/// `legal()` lookup and fails there with "not one of the legal assistant
/// moves" (its options never include an area that isn't really on the
/// board) instead of a separate, earlier range error. Only genuinely
/// unparseable tokens (non-numeric, or too large for `u8`) are rejected
/// here.
fn parse_raw_area_token(tok: &str) -> Result<AreaId, String> {
    tok.parse::<u8>().map(AreaId::unchecked).map_err(|_| format!("'{tok}' is not a valid area number"))
}

/// Parse an order-card token ("O17" or "17") into its numeric id -- no
/// validation against any deck; callers check membership themselves.
pub(crate) fn parse_order_id(tok: &str) -> Option<u8> {
    tok.strip_prefix(['O', 'o']).unwrap_or(tok).parse::<u8>().ok()
}

const HELP_TEXT: &str = "\
Commands:
  <number>        apply the numbered legal option
  a [id id id]    Placement Option A: place 1 assistant in each of 0-3 areas
  b <id>          Placement Option B: place 2 assistants in one area
  b <id> t <adj>  Placement Option B with the Telegram bonus: +1 assistant in an adjacent area
  m <from> <to>   resolve a pending assistant-move choice (<from> is 'hand' or an area id)
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

    /// A `Repl` wrapping a `Session::for_test_from_state`-constructed
    /// session with `pending_choices = [AnyGood, AnyGood]` already
    /// queued -- the genuine 2-deep cascade a Shophouse-build reward
    /// produces on a slot worth 2 any-good icons.
    ///
    /// Built via direct state construction (`push_choice`, mirroring the
    /// exact pattern the engine's own choice tests use --
    /// `crates/yokohama-core/src/game/choices.rs`) rather than a random
    /// walk through real gameplay: `PendingChoice::AnyGood` needs a
    /// Power-4+ Shophouse build, itself needing Telegram tech or an
    /// already-built Shophouse/Trading House there -- both multi-turn, so
    /// there is no short REAL move sequence to record instead (Task 11's
    /// report has the full reachability trace). A prior version of this
    /// fixture drove a live session through ~971 real moves picked by
    /// `rng.gen_range(0..legal().len())`; code review correctly flagged
    /// that as fragile -- it silently re-targets a DIFFERENT state if
    /// `get_legal_moves`'s enumeration order ever changes (a new Move
    /// variant, a reordered match arm), since the walk has no idea WHICH
    /// move index it's picking, only "the k-th one, whatever that is
    /// today." State injection has no such dependency: it names the exact
    /// `PendingChoice` variant it wants, twice.
    ///
    /// `Rules::empty_for_tests()`, not `Rules::standard()`, is required:
    /// `create_test_game_state`'s empty decks/board fail `check_invariants`'
    /// order/tech-conservation checks under the full 36-order/30-tech
    /// registries (see `invariants.rs`'s own `baseline()` fixture, which
    /// pairs the two for exactly this reason).
    fn repl_with_any_good_cascade() -> Repl {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(2);
        push_choice(&mut state, PendingChoice::AnyGood, &rules);
        push_choice(&mut state, PendingChoice::AnyGood, &rules);
        Repl::new(Session::for_test_from_state(state, rules))
    }

    #[test]
    fn any_good_cascade_banner_shows_choose_count_and_numbered_options() {
        let mut r = repl_with_any_good_cascade();
        assert_eq!(r.session.state().pending_choices.len(), 2);

        // Resolve the first of the two -- lands on exactly "1 choice
        // pending", the substring this test pins.
        let k = r.session.legal().iter()
            .position(|m| matches!(m, Move::ChooseAnyGood { good: TradeGood::Copper }))
            .expect("Copper is always one of the four AnyGood options");
        r.step(&k.to_string());
        assert_eq!(r.session.state().pending_choices.len(), 1);

        let p = r.prompt();
        assert!(p.contains("Choose:"), "{p}");
        assert!(p.contains("1 choice pending"), "{p}");
        // describe_pending_choice's own AnyGood banner text (the "reward:"
        // colon form is unique to it -- describe_move's per-option text
        // below uses "(any-good reward)" in parens instead, so this can't
        // pass on the numbered options' wording alone).
        assert!(p.contains("any-good reward: choose"), "{p}");
        // The four numbered ChooseAnyGood options (describe_move's wording).
        assert!(p.contains("copper"), "{p}");
        assert!(p.contains("silk"), "{p}");
        assert!(p.contains("tea"), "{p}");
        assert!(p.contains("fish"), "{p}");
        assert!(p.contains("(any-good reward)"), "{p}");
    }

    #[test]
    fn undo_mid_any_good_cascade_steps_back_exactly_one_choice() {
        let mut r = repl_with_any_good_cascade();
        assert_eq!(r.session.state().pending_choices.len(), 2);

        let k = r.session.legal().iter()
            .position(|m| matches!(m, Move::ChooseAnyGood { good: TradeGood::Copper }))
            .expect("Copper is always one of the four AnyGood options");
        r.step(&k.to_string());
        assert_eq!(r.session.state().pending_choices.len(), 1);
        let events_before = r.session.record().events.len();

        let out = r.step("undo");
        assert!(out.text.contains("Undid"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before - 1);
        assert_eq!(
            r.session.state().pending_choices.len(), 2,
            "undo must step back exactly ONE choice (re-queue the cascade), not unwind further"
        );

        // "the same choice again": still AnyGood, same banner shape.
        let p = r.prompt();
        assert!(p.contains("Choose:"), "{p}");
        assert!(p.contains("2 choice"), "{p}");

        // Stronger discriminator (code-review request): undo isn't a no-op
        // or a fixed reset -- there is nothing LEFT to undo at the
        // checkpoint (record.events is now empty), so a SECOND undo must
        // fail distinguishably rather than silently repeating the first
        // undo's effect (or doing nothing while reporting success).
        let out2 = r.step("undo");
        assert!(out2.text.contains("Cannot undo"), "{}", out2.text);
        assert_eq!(
            r.session.state().pending_choices.len(), 2,
            "a failed second undo must not mutate state"
        );
    }

    /// Turn-1 setup shared by the `m`-grammar tests: Option-B-places 2
    /// assistants at the board's Church area, moves the President there
    /// too (Power 3 = 1 president + 2 assistants), pays copper+silk+yen (3
    /// distinct payment types -> Faith = power(3) + 3 = 6), then picks
    /// Church space 6 specifically (`ChurchTrack::movement_rider(6) ==
    /// 2`, spaces 4/5 are the OTHER eligible-at-faith-6 spaces but only
    /// carry 1 rider each) -- lands on a genuine 2-deep AssistantMove
    /// cascade. Verified deterministic for seed 4 (this file's own
    /// standard fixture seed, via `repl_2p`) by an offline exploration;
    /// see Task 11 report. Returns the Church area's id, since the `m`
    /// grammar needs it as a literal area-number source.
    fn drive_to_double_assistant_move_cascade(r: &mut Repl) -> AreaId {
        let church_area = *r.session.state().board_layout.areas.iter()
            .find(|(_, c)| c.area_type == AreaType::Church)
            .expect("this board has a Church area").0;

        if !r.session.legal().iter().any(|m| matches!(m, Move::PlaceTwoSameArea { .. })) {
            let end = r.session.legal().iter().position(|m| matches!(m, Move::EndAdditionalPhase))
                .expect("EndAdditionalPhase reaches Placement");
            r.step(&end.to_string());
        }
        let place = r.session.legal().iter().position(|m| matches!(m,
            Move::PlaceTwoSameArea { area, adjacent_bonus: None } if *area == church_area
        )).expect("Option B at Church is legal turn 1");
        r.step(&place.to_string());

        let pres = r.session.legal().iter().position(|m| {
            matches!(m, Move::PlacePresident { destination } if *destination == church_area)
                || matches!(m, Move::MovePresident { destination, .. } if *destination == church_area)
        }).expect("President can reach Church turn 1");
        r.step(&pres.to_string());

        let church_mv = r.session.legal().iter().position(|m| matches!(m, Move::ExecuteAreaAction {
            action: AreaAction::Church {
                copper: true, silk: true, tea: false, seafood: false, yen: true, imported_good: false,
            }
        })).expect("paying copper+silk+yen is affordable turn 1 (starting resources include 1 of each)");
        r.step(&church_mv.to_string());

        if matches!(r.session.state().pending_choices.first(), Some(PendingChoice::ChurchSpace { .. })) {
            let space6 = r.session.legal().iter().position(|m| matches!(m, Move::ChooseChurchSpace { space: 6 }))
                .expect("space 6 (Faith 6) is reachable and unclaimed turn 1");
            r.step(&space6.to_string());
        }

        assert_eq!(
            r.session.state().pending_choices,
            vec![PendingChoice::AssistantMove, PendingChoice::AssistantMove],
            "setup recipe drifted -- re-derive against the current engine before trusting this fixture"
        );
        church_area
    }

    #[test]
    fn assistant_move_m_grammar_applies_from_area_and_hand() {
        let mut r = repl_2p(4);
        let church_area = drive_to_double_assistant_move_cascade(&mut r);
        let p = r.prompt();
        assert!(p.contains("Choose:"), "{p}");
        assert!(p.contains("2 choice"), "{p}");
        let events_before = r.session.record().events.len();

        // First rider: source is the on-board area holding our assistants.
        let out = r.step(&format!("m {} 9", church_area.0));
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert_eq!(r.session.state().pending_choices, vec![PendingChoice::AssistantMove]);

        // Second rider: source is hand.
        let out = r.step("m hand 9");
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 2);
        assert!(r.session.state().pending_choices.is_empty());
    }

    #[test]
    fn assistant_move_m_grammar_rejects_out_of_range_destination() {
        let mut r = repl_2p(4);
        let church_area = drive_to_double_assistant_move_cascade(&mut r);
        let events_before = r.session.record().events.len();
        let state_before = r.session.state().clone();

        let out = r.step(&format!("m {} 99", church_area.0));
        assert!(out.text.contains("not one of the legal assistant moves"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before);
        assert_eq!(*r.session.state(), state_before);
    }
}
