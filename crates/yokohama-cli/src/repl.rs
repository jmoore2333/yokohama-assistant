//! Pure REPL step function over a `Session`: numbered-option prompts,
//! placement grammar, view/utility commands, and pagination (design
//! docs/plans/2026-07-18-phase2-cli-driver-design.md). No stdin/stdout --
//! `main.rs` (Task 13) drives this with real I/O.

use yokohama_core::*;

use crate::{describe, render};

/// Live-mode reveal-collection state (Task 12 adds variants and reads this
/// field). Always `None` on the simulation path this task implements.
#[allow(dead_code)] // read by live mode (Task 12)
enum Pending {
    None,
}

/// One REPL turn's worth of state layered over a `Session`: which options
/// page is showing, any in-progress live-mode reveal collection, and
/// whether a driver/engine fault has locked the session down.
pub struct Repl {
    pub session: crate::session::Session,
    page: usize,
    #[allow(dead_code)] // read by live mode (Task 12)
    pending: Pending,
    /// Set once `commit` sees an `INVARIANT VIOLATION` error from
    /// `Session::apply` -- from then on `step` refuses everything except
    /// `save`/`log`/`quit`, so a desynced session can still be inspected
    /// and saved for postmortem but never played further.
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
    /// status line (once `locked`), or nothing at all (`quit`).
    fn out(&self, mut text: String, quit: bool) -> StepOutput {
        if quit {
            return StepOutput { text, quit: true };
        }
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        if self.locked {
            text.push_str(&self.lockdown_status());
        } else {
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

    /// Apply `mv` (sim path: no reveals -- Task 12 reroutes through reveal
    /// detection). On success, reset pagination and report the delta (plus
    /// a final-score table if the game just ended). On failure, an
    /// `INVARIANT VIOLATION` locks the session down with a postmortem dump;
    /// anything else is an ordinary illegal-move rejection.
    fn commit(&mut self, mv: Move) -> StepOutput {
        match self.session.apply(mv, vec![]) {
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

    fn save_cmd(&mut self, path: Option<&str>) -> StepOutput {
        let target = match path {
            Some(p) => {
                let pb = std::path::PathBuf::from(p);
                self.session.save_path = Some(pb.clone());
                pb
            }
            None => match self.session.save_path.clone() {
                Some(p) => p,
                None => return self.out("No save path set -- use 'save <path>'.\n".to_string(), false),
            },
        };
        match self.session.save(&target) {
            Ok(()) => self.out(format!("Saved to {}\n", target.display()), false),
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
            2, seed,
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
