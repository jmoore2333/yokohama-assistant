//! Pure REPL step function over a `Session`: numbered-option prompts,
//! placement grammar, view/utility commands, and pagination -- live-mode
//! reveal choreography lives in `reveal.rs` (moved there at Task 10)
//! (design docs/plans/2026-07-18-phase2-cli-driver-design.md).
//! No stdin/stdout -- `main.rs` (Task 13) drives this with real I/O.

use yokohama_core::*;

use crate::input::parse_order_id;
use crate::reveal::{claimed_token_area, lab_b_changed, refilled_port_slots, Pending};
use crate::session::{ApplyError, TrackingMode};
use crate::style::{paint, Tone};
use crate::{describe, render};

/// One REPL turn's worth of state layered over a `Session`: which options
/// page is showing, any in-progress live-mode reveal collection, and
/// whether a driver/engine fault has locked the session down.
pub struct Repl {
    pub session: crate::session::Session,
    page: usize,
    pub(crate) pending: Pending,
    /// Set once `finish_apply` sees an `ApplyError::Invariant` outcome from
    /// `Session::apply`, or once a just-completed live reveal group fails
    /// the post-apply desync assertion (`lock_on_order_desync`/
    /// `lock_on_tech_desync`) -- from then on `step` refuses everything
    /// except `save`/`log`/`quit`, so a desynced session can still be
    /// inspected and saved for postmortem but never played further.
    pub(crate) locked: bool,
    /// An in-progress Movement sub-prompt (route pick or guided retreat), or
    /// `None`. A sibling of `pending` rather than a `Pending` variant: `pending`
    /// models LIVE-mode reveal collection (always `None` in simulation, keyed to
    /// hidden-card entry), whereas sub-prompts are pure ENTRY HELPERS over the
    /// engine's own legal list that apply in BOTH modes and never collect hidden
    /// information. The two are structurally mutually exclusive -- a sub-prompt
    /// only ever starts from a clean main prompt (`pick_option` runs only when
    /// `pending` is `None`), and committing out of one never draws a card or
    /// claims a token (President moves / retreats touch neither deck), so
    /// `commit` never parks a fresh `Pending` behind a sub-prompt.
    pub(crate) sub: SubPrompt,
    /// Set true once the FIRST `board` render of a session has printed the
    /// wide-board width note (S3), so it prints exactly once. Lives on the
    /// REPL rather than in `render` because that layer is a pure formatter --
    /// "once per session" is mutable session state the REPL owns.
    board_width_note_shown: bool,
}

/// One line in the Movement phase's collapsed numbered list (design.md:67,
/// destination-first). Outside Movement -- and inside it whenever a choice is
/// queued -- every option is a bare `Direct`, so the list is byte-identical to
/// the ungrouped `legal()` rendering; the two collapsing variants only ever
/// appear in the Movement main list. The legal list stays the SOLE authority:
/// `Route`/`Retreat` are entry helpers whose selections are exact-matched
/// against `legal()` before anything commits (`find_placement_match`).
enum PromptOption {
    /// A single move rendered and committed exactly as today (`describe_move`).
    Direct(Move),
    /// A walk/teleport destination reachable by >1 route label -- selecting it
    /// opens the route sub-prompt over `moves` (all `MovePresident` to `destination`).
    Route { destination: AreaId, moves: Vec<Move> },
    /// The whole retreat family (>1 `ReturnPresidentToHand` variant) collapsed to
    /// one line; `count` is the family size, shown in the entry text. Selecting it
    /// opens the guided retreat sub-prompt (per-area recovery counts, recomputed
    /// from state -- the moves themselves are never cloned into the entry, so a
    /// pathological 100k-variant retreat family stays cheap to display).
    Retreat { count: usize },
}

/// An in-progress Movement sub-prompt. Both variants are ENTRY HELPERS: whatever
/// they assemble is exact-matched against `session.legal()` before `commit`, so
/// neither can construct-and-apply anything the engine didn't offer.
pub(crate) enum SubPrompt {
    None,
    /// Picking one of a multi-label destination's route labels. `moves` are the
    /// `MovePresident` variants to `destination` (bounded small -- one per distinct
    /// fee vector); a numeric answer commits `moves[k]` directly.
    Route { destination: AreaId, moves: Vec<Move> },
    /// Guided recovery: `areas` are the current player's occupied areas ascending
    /// (AreaId, count-there), captured at entry (state is frozen until commit);
    /// `collected` grows one answer per area. Once `collected.len() == areas.len()`
    /// the confirm line shows, and `confirm` assembles + exact-matches the
    /// `ReturnPresidentToHand { recover }`.
    Retreat { areas: Vec<(AreaId, u8)>, collected: Vec<u8> },
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
        // Restore a persisted lockdown (review §3): a session saved after a
        // desync/invariant fault carries `record.lockdown`, so a reload comes
        // back locked instead of silently playable again.
        let locked = session.is_locked();
        Repl {
            session,
            page: 0,
            pending: Pending::None,
            locked,
            sub: SubPrompt::None,
            board_width_note_shown: false,
        }
    }

    /// Engage a lockdown: flip the in-memory `locked` flag AND persist the
    /// `reason` into the record (so it survives save/reload -- review §3),
    /// returning any autosave-failure warning to append to the caller's output.
    /// Every site that sets `locked` goes through here so the two never drift.
    pub(crate) fn engage_lockdown(&mut self, reason: String) -> Option<String> {
        self.locked = true;
        self.session.lock_down(reason)
    }

    /// The whole prompt screen for the current state: header line, options
    /// page (or a game-over/no-options notice), and an input hint. Legal
    /// options are numbered by their GLOBAL 0-based index into
    /// `session.legal()`, 20 per page.
    pub fn prompt(&self) -> String {
        // A locked session (including one reloaded from a persisted lockdown --
        // review §3) shows only its lockdown status: the numbered options can't
        // be used, so rendering them would mislead. `step` allows just
        // save/log/quit from here.
        if self.locked {
            return self.lockdown_status();
        }
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
            // The Movement phase collapses its list into `display_options`
            // (destination-first + one retreat line); every other phase yields
            // one `Direct` per legal move, so this renders byte-for-byte as the
            // old `legal()` loop did. Paging is over the DISPLAYED entries, not
            // the raw legal list, so a grouped Movement list pages by its own
            // (much smaller) count.
            let options = self.display_options();
            let n = options.len();
            if n == 0 {
                out.push_str("(no legal options right now -- commands only)\n");
            } else {
                let total_pages = total_pages_for(n);
                let start = (self.page * 20).min(n);
                let end = (start + 20).min(n);
                for (i, opt) in options[start..end].iter().enumerate() {
                    out.push_str(&format!(
                        "[{}] {}\n",
                        start + i,
                        self.render_option(opt, state, self.session.rules())
                    ));
                }
                if end < n {
                    out.push_str(&format!("...{} more (type 'more')\n", n - end));
                }
                if total_pages > 1 {
                    out.push_str(&format!("page {}/{total_pages}\n", self.page + 1));
                }
            }
        }
        out.push_str(&self.input_hint());
        out
    }

    /// The one-line trailing hint every prompt screen ends with -- shared by
    /// `prompt()`'s full block and `out_view`'s hint-only block so they
    /// agree on its wording verbatim (Task 13; made phase-aware for the
    /// Codex review I-4 fix). Names only the shorthand that's ACTUALLY
    /// usable right now: `a`/`b` in `Placement`, `r` in `Movement`, or
    /// `m <from> <to>` (plus the numbered Decline) while an
    /// `AssistantMove` choice sits at the head of the queue -- checked
    /// ahead of phase, same priority order `prompt()`'s own `phase_text`
    /// banner uses. Every other phase, and every other pending-choice kind
    /// (`AnyGood`, `WarehouseTake`, `ChurchSpace`, ...), has no shorthand of
    /// its own, so it gets the bare numbers+'help' hint.
    fn input_hint(&self) -> String {
        const TAIL: &str = "or a command ('help' for the list)";
        let state = self.session.state();
        if matches!(state.pending_choices.first(), Some(PendingChoice::AssistantMove)) {
            return format!("> enter a number, 'm <from> <to>' to resolve (or the numbered Decline), {TAIL}\n");
        }
        if !state.pending_choices.is_empty() {
            return format!("> enter a number, {TAIL}\n");
        }
        match state.turn_phase {
            TurnPhase::Placement => format!("> enter a number, 'a'/'b' for placement, {TAIL}\n"),
            TurnPhase::Movement => {
                format!("> enter a number, 'r' to retreat (or the numbered retreat entry for guided recovery), {TAIL}\n")
            }
            TurnPhase::AdditionalActionFirst
            | TurnPhase::AreaAction
            | TurnPhase::FivePowerBonus
            | TurnPhase::Building
            | TurnPhase::Recovery
            | TurnPhase::AdditionalActionSecond
            | TurnPhase::TurnEnd => format!("> enter a number, {TAIL}\n"),
        }
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
        // A Movement sub-prompt (route pick / guided retreat) captures input the
        // same way the reveal collector does -- checked AFTER the lockdown gate
        // above, so lockdown wins (entering or continuing a sub-prompt while
        // locked is refused exactly like a move). Structurally exclusive with the
        // `pending` reveal branch below: a sub-prompt only ever starts from a
        // clean main prompt (see `SubPrompt`'s doc comment).
        if !matches!(self.sub, SubPrompt::None) {
            return self.step_sub_prompt(&words);
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
            ["help"] => self.out_view(HELP_TEXT.to_string()),
            ["board"] => self.board_cmd(),
            ["players"] => self.out_view(render::players_panel(self.session.state(), self.session.rules())),
            ["ports"] => self.out_view(render::ports_view(self.session.state(), self.session.rules())),
            ["labs"] => self.out_view(render::labs_view(self.session.state(), self.session.rules())),
            ["tracks"] => self.out_view(render::tracks_view(self.session.state())),
            ["sites"] => self.out_view(render::sites_view(self.session.state(), self.session.rules())),
            ["goals"] => self.out_view(render::achievements_view(self.session.state(), self.session.rules())),
            ["hand"] => self.out_view(self.hand_text()),
            ["score"] => self.out_view(self.score_text()),
            ["log"] => self.out_view(self.log_tail()),
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
            ["r", rest @ ..] => self.try_retreat(rest),
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
        } else if !matches!(self.sub, SubPrompt::None) {
            // Inside a Movement sub-prompt: append the sub-prompt screen (route
            // list / current recovery question / confirm line) instead of the
            // main numbered list -- symmetric with the `pending` reveal branch.
            text.push_str(&self.sub_prompt_text());
        } else if matches!(self.pending, Pending::None) {
            text.push_str(&self.prompt());
        }
        StepOutput { text, quit: false }
    }

    /// Like `out`, but for read-only VIEW commands (`board`/`players`/
    /// `ports`/`labs`/`tracks`/`sites`/`goals`/`hand`/`score`/`log`/`help`):
    /// appends just
    /// the phase-aware input hint (`input_hint()`), not the full
    /// header+phase+numbered-options block `prompt()` builds (Task 13 test
    /// (e)). A view command never changes state, so which options are
    /// legal never changes either -- reprinting up to 20 numbered options
    /// after every single glance at the board was pure noise (closed a
    /// dogfooding friction note). Mirrors `out`'s lockdown/pending
    /// short-circuits for defense in depth, even though today only `log`
    /// can reach here while locked (`step`'s lockdown allow-list) and no
    /// view command is reachable mid-reveal-collection (`step`'s
    /// pending-active branch routes elsewhere entirely).
    pub(crate) fn out_view(&self, mut text: String) -> StepOutput {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        if self.locked {
            text.push_str(&self.lockdown_status());
        } else if !matches!(self.sub, SubPrompt::None) {
            // Defense in depth: no view command is reachable mid-sub-prompt
            // (`step`'s sub-prompt branch routes elsewhere entirely), so this
            // mirrors `out`'s short-circuit rather than being load-bearing.
            text.push_str(&self.sub_prompt_text());
        } else if matches!(self.pending, Pending::None) {
            text.push_str(&self.input_hint());
        }
        StepOutput { text, quit: false }
    }

    /// The `board` view command: the box-drawing grid, plus a one-line legend
    /// footer (item 7) and -- on the FIRST render of a session whose board is
    /// wider than 100 columns -- a one-time width note (S3). The note is keyed
    /// on the board's OWN rendered width (all grid rows share one width, so
    /// the first line's column count is it), never on the terminal size (std
    /// exposes none). `&mut self` only to flip the once-per-session flag.
    fn board_cmd(&mut self) -> StepOutput {
        let grid = render::board_grid(self.session.state());
        let width = grid.lines().next().map(|l| l.chars().count()).unwrap_or(0);

        let mut text = String::new();
        if width > 100 && !self.board_width_note_shown {
            self.board_width_note_shown = true;
            text.push_str(&format!(
                "(board is {width} columns wide — widen the terminal if rows shear; guide §2)\n"
            ));
        }
        text.push_str(&grid);
        text.push_str(render::board_legend());
        text.push('\n');
        self.out_view(text)
    }

    pub(crate) fn lockdown_status(&self) -> String {
        // Surface the persisted reason (review §3) when one is recorded, so a
        // reloaded locked session tells the operator WHY it locked, not just
        // that it did. A directly-forced test lock has no recorded reason.
        let reason = self
            .session
            .record()
            .lockdown
            .as_deref()
            .map(|r| format!(" [{r}]"))
            .unwrap_or_default();
        format!(
            "{}{reason} turn {} | seed {} | {} logged event(s) -- only 'save', 'log', 'quit' available\n",
            paint(Tone::Warn, "[locked]"),
            self.session.state().turn_number,
            self.session.record().seed,
            self.session.record().events.len(),
        )
    }

    fn advance_page(&mut self) {
        // Paging is over the DISPLAYED entry count (grouped in Movement), so
        // `more` wraps exactly where `prompt()`'s "page N/M" footer says it will.
        let n = self.display_options().len();
        self.page = (self.page + 1) % total_pages_for(n);
    }

    fn pick_option(&mut self, k: usize) -> StepOutput {
        // Index into the DISPLAYED entries: a `Direct` commits as before, a
        // `Route`/`Retreat` entry opens its sub-prompt. `display_options` calls
        // `legal()` exactly once (same as the old direct-index path).
        let options = self.display_options();
        let len = options.len();
        match options.into_iter().nth(k) {
            Some(PromptOption::Direct(mv)) => self.commit(mv),
            Some(PromptOption::Route { destination, moves }) => self.enter_route_subprompt(destination, moves),
            Some(PromptOption::Retreat { .. }) => self.enter_retreat_subprompt(),
            None => self.out(format!("option {k} out of range (0..{len})\n"), false),
        }
    }

    /// Build the numbered entries for the current prompt. Outside the Movement
    /// phase -- and inside it while a choice is queued -- every legal move is a
    /// bare `Direct` (byte-identical to the old ungrouped rendering). In the
    /// Movement main list it collapses the family (design.md:67): one entry per
    /// walk/teleport destination (a `Route` when that destination has >1 fee
    /// label, else the single move `Direct`), each `PlacePresident`/other move
    /// left as `Direct` in place, and the whole `ReturnPresidentToHand` family
    /// folded into one `Retreat` line (unless a lone bare retreat is the only
    /// variant, which stays `Direct`). Single pass over `legal()` into a
    /// `BTreeMap` -- no per-keystroke rescans beyond the one `legal()` call.
    fn display_options(&self) -> Vec<PromptOption> {
        let legal = self.session.legal();
        let state = self.session.state();
        let group = state.turn_phase == TurnPhase::Movement && state.pending_choices.is_empty();
        if !group {
            return legal.into_iter().map(PromptOption::Direct).collect();
        }

        let mut routes: std::collections::BTreeMap<AreaId, Vec<Move>> = std::collections::BTreeMap::new();
        let mut others: Vec<Move> = Vec::new();
        let mut retreat_count = 0usize;
        let mut lone_retreat: Option<Move> = None;
        for mv in legal {
            match &mv {
                Move::MovePresident { destination, .. } => {
                    let dest = *destination;
                    routes.entry(dest).or_default().push(mv);
                }
                Move::ReturnPresidentToHand { .. } => {
                    retreat_count += 1;
                    if lone_retreat.is_none() {
                        lone_retreat = Some(mv); // kept only for the singleton case
                    }
                }
                _ => others.push(mv),
            }
        }

        let mut out = Vec::new();
        for (destination, mut moves) in routes {
            if moves.len() == 1 {
                out.push(PromptOption::Direct(moves.pop().unwrap()));
            } else {
                out.push(PromptOption::Route { destination, moves });
            }
        }
        out.extend(others.into_iter().map(PromptOption::Direct));
        match retreat_count {
            0 => {}
            1 => out.push(PromptOption::Direct(lone_retreat.unwrap())),
            n => out.push(PromptOption::Retreat { count: n }),
        }
        out
    }

    /// One numbered-line's text for a display entry. `Direct` reuses
    /// `describe_move` verbatim; the two collapsing variants get their own
    /// single-line summaries (the `<k>` counts come from the grouped family sizes).
    fn render_option(&self, opt: &PromptOption, state: &GameState, rules: &Rules) -> String {
        match opt {
            PromptOption::Direct(mv) => describe::describe_move(mv, state, rules),
            PromptOption::Route { destination, moves } => format!(
                "Move President to {} — {} route options",
                describe::area_label(state, *destination),
                moves.len()
            ),
            PromptOption::Retreat { count } => format!(
                "Retreat President to hand — choose recovery ({count} combinations; 'r' grammar works too)"
            ),
        }
    }

    /// Parse `a [id id id [id]]` (0-3 area ids, or 0-4 for a Newspaper
    /// owner) into a `PlaceThreeAreas` candidate and match it against
    /// `legal()` by placement-multiset equality (the generator's internal
    /// ordering must not matter). This function only bounds the token
    /// COUNT (at most 4, Newspaper or not) -- `find_placement_match` below
    /// is the sole legality authority, so a 4-id entry from a non-Newspaper
    /// owner parses fine here and simply never matches anything in
    /// `legal()`, falling through to the same "not currently available"
    /// rejection any other engine-unoffered combination gets.
    fn try_placement_a(&mut self, rest: &[&str]) -> StepOutput {
        if rest.len() > 4 {
            return self.out(format!("Option A takes at most 4 area ids, got {}.\n", rest.len()), false);
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

    /// Parse `r [<area>[:<n>]]...` (Movement Option B: retreat the
    /// President) into a `ReturnPresidentToHand` candidate and exact-match
    /// it against `legal()` via `find_placement_match`'s catch-all
    /// (`BTreeMap` equality is canonical, so plain `==` suffices -- no
    /// multiset dance like Option A's areas). `r` alone (`rest` empty)
    /// builds the empty map -- recover nothing. A bare area token recovers
    /// ALL assistants the current player has there right now (read from
    /// state); `<area>:<n>` recovers exactly `n` (`n` >= 1; `:0` is
    /// rejected the same as a non-numeric count, matching
    /// `ReturnPresidentToHand`'s own invariant that a present entry is
    /// never zero-valued). Never bypasses the generator: a combination the
    /// rules don't actually offer -- including a bare area with nothing
    /// there, which builds a zero-valued entry no legal option ever has --
    /// gets the same "not in the legal list" rejection regardless of why
    /// it's wrong.
    fn try_retreat(&mut self, rest: &[&str]) -> StepOutput {
        let mut recover = std::collections::BTreeMap::new();
        let me = self.session.state().current_player;
        for tok in rest {
            let (area_tok, count_tok) = match tok.split_once(':') {
                Some((a, n)) => (a, Some(n)),
                None => (*tok, None),
            };
            let Ok(area) = parse_raw_area_token(area_tok) else {
                return self.out(format!("Bad area token '{area_tok}'. Usage: r [<area>[:<n>]]...\n"), false);
            };
            let count = match count_tok {
                Some(n) => match n.parse::<u8>() {
                    Ok(k) if k > 0 => k,
                    _ => return self.out(format!("Bad count in '{tok}'.\n"), false),
                },
                None => self.session.state().player(me).assistants.in_area(area),
            };
            recover.insert(area, count);
        }
        let candidate = Move::ReturnPresidentToHand { recover };
        match self.find_placement_match(&candidate) {
            Some(mv) => self.commit(mv),
            None => self.out("That retreat is not in the legal list (check areas/counts).\n".to_string(), false),
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

    /// Route one line of input into the active Movement sub-prompt. `matches!`
    /// takes only a momentary borrow, so the `&mut self` dispatch is clean.
    fn step_sub_prompt(&mut self, words: &[&str]) -> StepOutput {
        if matches!(self.sub, SubPrompt::Route { .. }) {
            self.step_route(words)
        } else {
            self.step_retreat(words)
        }
    }

    /// Route sub-prompt input: a valid route number commits that exact
    /// `MovePresident` (taken straight from `legal()` -- never re-derived),
    /// `cancel` returns to the main list, `quit`/`q` exits (the un-applied
    /// sub-prompt is simply discarded -- nothing was recorded, same escape the
    /// reveal collector honors). A bare out-of-range number re-asks with its
    /// range; any other non-answer re-asks with `! finish or cancel this prompt
    /// first` (a view/`save`/`undo`/`more` mid-collection would render or mutate
    /// against a state the collection doesn't reflect).
    fn step_route(&mut self, words: &[&str]) -> StepOutput {
        match words {
            ["cancel"] => {
                self.sub = SubPrompt::None;
                self.page = 0;
                self.out("Route selection cancelled — back to the movement options.".to_string(), false)
            }
            ["quit"] | ["q"] => StepOutput { text: "bye\n".to_string(), quit: true },
            [tok] if !tok.is_empty() && tok.chars().all(|c| c.is_ascii_digit()) => {
                // Resolve while borrowing `self.sub`, but yield an OWNED result so
                // the borrow is released before the mutation/commit below.
                let selected: Result<Move, usize> = match &self.sub {
                    SubPrompt::Route { moves, .. } => {
                        let n = moves.len();
                        tok.parse::<usize>().ok().and_then(|k| moves.get(k).cloned()).ok_or(n)
                    }
                    _ => return self.out(String::new(), false), // unreachable: dispatched as Route
                };
                match selected {
                    Ok(mv) => {
                        self.sub = SubPrompt::None;
                        self.page = 0;
                        self.commit(mv)
                    }
                    Err(n) => self.out(format!("! pick a route 0-{}, or 'cancel'", n.saturating_sub(1)), false),
                }
            }
            // The route list pages like the main list (brief item 5): `more`/
            // `list` drive its pager rather than being refused, so the mirrored
            // "type 'more'" footer is honest. (A deliberate refinement of the
            // discipline's default "more re-asks" -- only for the route list,
            // which is the only paged sub-prompt surface; retreat questions are
            // sequential, not a list, so `step_retreat` still refuses them.)
            ["more"] => {
                let n = match &self.sub {
                    SubPrompt::Route { moves, .. } => moves.len(),
                    _ => 0,
                };
                self.page = (self.page + 1) % total_pages_for(n);
                self.out(String::new(), false)
            }
            ["list"] => {
                self.page = 0;
                self.out(String::new(), false)
            }
            _ => self.out("! finish or cancel this prompt first".to_string(), false),
        }
    }

    /// Guided retreat input. During the per-area questions a valid count
    /// (`0..=` that area's own on-board total) advances to the next area; once
    /// every area is answered, `confirm` assembles and applies the retreat.
    /// `cancel`/`quit` behave as in the route sub-prompt. A count outside the
    /// area's range (including one exceeding what's available) re-asks the SAME
    /// question with its range; any other non-answer (a view command, garbage)
    /// re-asks with `! finish or cancel this prompt first`.
    fn step_retreat(&mut self, words: &[&str]) -> StepOutput {
        let (areas_len, collected_len) = match &self.sub {
            SubPrompt::Retreat { areas, collected } => (areas.len(), collected.len()),
            _ => return self.out(String::new(), false), // unreachable: dispatched as Retreat
        };
        let confirming = collected_len >= areas_len;
        match words {
            ["cancel"] => {
                self.sub = SubPrompt::None;
                self.page = 0;
                self.out("Retreat cancelled — back to the movement options.".to_string(), false)
            }
            ["quit"] | ["q"] => StepOutput { text: "bye\n".to_string(), quit: true },
            _ if confirming => match words {
                ["confirm"] => self.confirm_retreat(),
                _ => self.out("! finish or cancel this prompt first".to_string(), false),
            },
            [tok] if !tok.is_empty() && tok.chars().all(|c| c.is_ascii_digit()) => {
                let max = match &self.sub {
                    SubPrompt::Retreat { areas, collected } => areas[collected.len()].1,
                    _ => return self.out(String::new(), false),
                };
                match tok.parse::<u8>() {
                    Ok(n) if n <= max => {
                        if let SubPrompt::Retreat { collected, .. } = &mut self.sub {
                            collected.push(n);
                        }
                        // out() appends the next question, or the confirm line.
                        self.out(String::new(), false)
                    }
                    _ => self.out(format!("! recover 0-{max} from this area, or 'cancel'"), false),
                }
            }
            _ => self.out("! finish or cancel this prompt first".to_string(), false),
        }
    }

    /// Assemble the collected per-area counts into a `ReturnPresidentToHand`
    /// (zero-count areas omitted -- an absent entry is the engine's own encoding
    /// for "recover none there"), then exact-match against `legal()` and commit.
    /// The per-area bounds were honored at every question, so a legal member
    /// always results; the match stays the sole legality authority regardless
    /// (a non-member gets the standard rejection and drops back to the main list).
    fn confirm_retreat(&mut self) -> StepOutput {
        let recover: std::collections::BTreeMap<AreaId, u8> = match &self.sub {
            SubPrompt::Retreat { areas, collected } => areas
                .iter()
                .enumerate()
                .filter_map(|(i, &(area, _))| (collected[i] > 0).then_some((area, collected[i])))
                .collect(),
            _ => return self.out(String::new(), false),
        };
        self.sub = SubPrompt::None;
        self.page = 0;
        let candidate = Move::ReturnPresidentToHand { recover };
        match self.find_placement_match(&candidate) {
            Some(mv) => self.commit(mv),
            None => self.out("Illegal: that retreat is not in the legal list".to_string(), false),
        }
    }

    /// Open the route sub-prompt for a multi-label destination. `out()` appends
    /// `sub_prompt_text()` (the route list) since `self.sub` is now set.
    fn enter_route_subprompt(&mut self, destination: AreaId, moves: Vec<Move>) -> StepOutput {
        self.sub = SubPrompt::Route { destination, moves };
        self.page = 0; // the route list gets its own paging, starting at page 0
        self.out(String::new(), false)
    }

    /// Open the guided retreat sub-prompt: capture the current player's occupied
    /// areas (ascending AreaId, with their on-board counts -- the same set and
    /// order `enumerate_recovery_options` walks) and ask the first. Reached only
    /// from a collapsed `Retreat` entry, which exists only when >1 retreat
    /// variant is legal -- i.e. at least one occupied area -- so `areas` is
    /// non-empty and the confirm step is never reached with zero questions asked.
    fn enter_retreat_subprompt(&mut self) -> StepOutput {
        let me = self.session.state().current_player;
        let areas: Vec<(AreaId, u8)> = self
            .session
            .state()
            .player(me)
            .assistants
            .on_board
            .iter()
            .filter(|(_, &c)| c > 0)
            .map(|(&a, &c)| (a, c))
            .collect();
        self.sub = SubPrompt::Retreat { areas, collected: Vec::new() };
        self.out("Guided retreat — enter how many to recover from each area, then confirm.".to_string(), false)
    }

    /// The sub-prompt screen for whatever `self.sub` holds (empty when `None`):
    /// the route label list (each via `describe_move`, so cost/recipient text is
    /// unchanged), the current per-area recovery question, or the confirm line
    /// (the assembled move rendered exactly by `describe_move`).
    fn sub_prompt_text(&self) -> String {
        let state = self.session.state();
        let rules = self.session.rules();
        match &self.sub {
            SubPrompt::None => String::new(),
            SubPrompt::Route { destination, moves } => {
                // Same 20/page + footer machinery the main list uses (brief item
                // 5). Bounded ~16 in real play, so this rarely pages -- but when
                // it does, `more`/`list` step it (see `step_route`), so the
                // mirrored "type 'more'" footer stays honest.
                let mut out = format!("Routes to {}:\n", describe::area_label(state, *destination));
                let n = moves.len();
                let total_pages = total_pages_for(n);
                let start = (self.page * 20).min(n);
                let end = (start + 20).min(n);
                for (i, mv) in moves[start..end].iter().enumerate() {
                    out.push_str(&format!("[{}] {}\n", start + i, describe::describe_move(mv, state, rules)));
                }
                if end < n {
                    out.push_str(&format!("...{} more (type 'more')\n", n - end));
                }
                if total_pages > 1 {
                    out.push_str(&format!("page {}/{total_pages}\n", self.page + 1));
                }
                out.push_str("> pick a route number, or 'cancel'\n");
                out
            }
            SubPrompt::Retreat { areas, collected } => {
                if collected.len() < areas.len() {
                    let (area, max) = areas[collected.len()];
                    format!(
                        "recover how many from {} [0-{}]?\n> enter a number, or 'cancel'\n",
                        describe::area_label(state, area),
                        max
                    )
                } else {
                    let recover: std::collections::BTreeMap<AreaId, u8> = areas
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &(area, _))| (collected[i] > 0).then_some((area, collected[i])))
                        .collect();
                    let mv = Move::ReturnPresidentToHand { recover };
                    format!("{}\n> confirm / cancel\n", describe::describe_move(&mv, state, rules))
                }
            }
        }
    }

    /// Route `mv` through live-mode reveal detection (`TrackingMode::Live`
    /// only), then apply. A scratch-apply against the CURRENT state (never
    /// mutates -- the probe below calls `apply_move_unchecked`, which clones
    /// like every arm in that module) reveals whether `mv` draws
    /// from either deck or claims a 5-Power token; when it does, `mv` is
    /// parked in `self.pending` and the operator is prompted instead of
    /// applying immediately. Everything else -- including every move in
    /// `TrackingMode::Simulation` -- applies at once via `finish_apply`.
    fn commit(&mut self, mv: Move) -> StepOutput {
        if matches!(self.session.mode, TrackingMode::Live) {
            // apply_move_unchecked: this is a scratch probe of a move that
            // came from the numbered legal list (never mutates self.session
            // -- see the doc comment above); the real apply behind it goes
            // through Session::apply's checked gate.
            match apply_move_unchecked(self.session.state(), &mv, self.session.rules()) {
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
    /// table if the game just ended). On failure, an `ApplyError::Invariant`
    /// locks the session down with a postmortem dump; anything else is an
    /// ordinary illegal-move rejection. This is the sim path's entire
    /// `commit` (reveals always empty there) and the terminal step of every
    /// live-mode reveal collection that doesn't need the desync guard (see
    /// `finish_apply_ex` for the two that do).
    pub(crate) fn finish_apply(&mut self, mv: Move, reveals: Vec<RecordedEvent>) -> StepOutput {
        self.finish_apply_ex(mv, reveals).0
    }

    /// Same as `finish_apply`, but also reports whether the apply actually
    /// SUCCEEDED (Task 13; inverted to `applied: bool` semantics in the
    /// I1/I2/I3 final-review fix wave's cheap batch) -- `reveal.rs`'s
    /// `lock_on_order_desync`/`lock_on_tech_desync` need that to skip their
    /// post-apply board re-check whenever nothing actually changed. `applied`
    /// covers EVERY error variant, not just `ApplyError::Illegal`: the
    /// narrower `was_illegal` flag this replaces would have let a
    /// `GameOver`/`Reveal` error slip through as "not illegal" and trigger a
    /// false-positive desync lockdown against a board that was never
    /// touched.
    pub(crate) fn finish_apply_ex(&mut self, mv: Move, reveals: Vec<RecordedEvent>) -> (StepOutput, bool) {
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
                (self.out(text, false), true)
            }
            Err(ApplyError::Invariant(msg)) => {
                let warning = self.engage_lockdown(format!("invariant violation: {msg}"));
                let mut text = format!("{}\n", paint(Tone::Warn, &format!("LOCKDOWN -- {msg}")));
                text.push_str(&self.lockdown_dump());
                if let Some(w) = warning {
                    text.push_str(&w);
                    text.push('\n');
                }
                (self.out(text, false), false)
            }
            // T13 rider (final-review cheap batch): a dedicated arm instead
            // of falling into the catch-all's "Illegal: {e}" phrasing, which
            // rendered ApplyError::GameOver's Display text ("the game is
            // already over") as the confusing "Illegal: the game is already
            // over".
            Err(ApplyError::GameOver) => (self.out("game over — commands only\n".to_string(), false), false),
            Err(e) => (self.out(format!("Illegal: {e}\n"), false), false),
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
    /// winner announcement + per-player breakdown table instead.
    fn score_text(&self) -> String {
        if self.session.is_over() {
            self.final_score_table()
        } else {
            let mut out = String::from("Score:\n");
            for p in &self.session.state().players {
                out.push_str(&format!(
                    "  {:?}: {}, {} yen\n",
                    p.color,
                    paint(Tone::Vp, &format!("{} VP", p.victory_points)),
                    p.money,
                ));
            }
            out
        }
    }

    /// `WINNER: {color}` followed by one ranked line per standing, each with
    /// its full `ScoreBreakdown` (`final_result`, scoring.rs) so the reason
    /// behind the total is visible, not just the number -- the bot phase's
    /// terminal utility needs the breakdown, and a human reading the
    /// game-over screen does too.
    fn final_score_table(&self) -> String {
        let result = self.session.final_result();
        let state = self.session.state();
        let mut out = format!("WINNER: {}\n", render::color_name(state.player(result.winner).color));
        for s in &result.standings {
            let b = &s.breakdown;
            out.push_str(&format!(
                "#{} {}: {} (base {} + church {} + customs {} + tech {} + country {} + remainder {})\n",
                s.rank,
                render::color_name(state.player(s.player).color),
                paint(Tone::Vp, &format!("{} VP", b.total)),
                b.base_vp,
                b.church,
                b.customs,
                b.technology,
                b.country,
                b.remainder,
            ));
        }
        out
    }

    /// Last 10 logged events (moves and reveal reconciliations) in plain
    /// words, each with its global index into `record().events`. Reads
    /// `Session::event_description` -- a description CACHED at the moment
    /// the event applied (T2.4) -- rather than re-describing against the
    /// CURRENT state as this used to: an old event's state-dependent text
    /// (a trading-house take's printed price, a movement fee's recipient)
    /// would otherwise silently go stale once state moved on past it.
    fn log_tail(&self) -> String {
        let n = self.session.record().events.len();
        let start = n.saturating_sub(10);
        let mut out = format!("log ({} of {} event(s)):\n", n - start, n);
        for i in start..n {
            out.push_str(&format!("  {}: {}\n", i, self.session.event_description(i)));
        }
        out
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

/// The in-app `help` command's text. `pub` so the `guide_sync` integration
/// test can pin every command token against the tester guide's §5 block --
/// this const is the authority; the guide follows it.
pub const HELP_TEXT: &str = "\
Commands:
  <number>        apply the numbered legal option
  a [id id id]    Placement Option A: place 1 assistant in each of 0-3 areas (4 with Newspaper)
  b <id>          Placement Option B: place 2 assistants in one area
  b <id> t <adj>  Placement Option B with the Telegram bonus: +1 assistant in an adjacent area
  m <from> <to>   resolve a pending assistant-move choice (<from> is 'hand' or an area id)
  r [id[:n]]...   Movement Option B: retreat the President, recovering all (bare id) or n assistants per area ('r' alone recovers none)
  board           show the area grid (legend: * = unclaimed 5-Power token)
  players         show each player's resources, hand, tech, tokens, President
  ports           show both Port boards and the order-deck size
  labs            show both Laboratory boards (each tech's country + effect) and the tech-deck size
  tracks          show the Church and Customs tracks with printed VP and rider icons
  sites           show each area's building-site printed rewards, build marks, and adjacency
  goals           show achievement cards with claim VP, claim state, and per-player progress
  hand            show the current player's order-card hand
  score           show current VP/money (or the final scores once the game is over)
  log             show the tail of the move/event log
  more            show the next page of legal options (wraps to page 0)
  list            reset to the first page of legal options
  undo            undo the last move (and any reveals bound to it)
  save [path]     save to path (or to the last-used path if omitted)
  reveal <seat> <O..>  live mode: make a hidden opponent starting order public
  cancel          abort an in-progress reveal prompt (live mode) or a route/retreat sub-prompt
  help            show this text
  quit            exit the REPL ('q' also works)
";

/// How many 20-per-page screens `n` legal options need, minimum 1 -- shared
/// by `prompt()`'s "page N/M" footer and `advance_page`'s wraparound so the
/// footer's M always matches where `more` actually wraps (Codex review I-4
/// page-count fix).
fn total_pages_for(n: usize) -> usize {
    n.div_ceil(20).max(1)
}

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

    /// A REPL over a fresh `n`-player simulation (n in 2..=4). Used for the
    /// board-width note (S3): 3p/4p boards render wider than 100 columns,
    /// which the 2p board (86 columns) never does.
    fn repl_np(n: u8, seed: u64) -> Repl {
        Repl::new(
            Session::new_sim(
                GameConfig::for_player_count(n),
                seed,
                vec![StartingOrderChoice::Keep(0); n as usize],
                Rules::standard(),
            )
            .unwrap(),
        )
    }

    /// S3 (pre-gate view batch): the FIRST `board` render of a session, on a
    /// board wider than 100 columns, prints a one-time width note -- there is
    /// no terminal-size detection (std has none; no new deps), so the note is
    /// keyed purely on the board's own rendered width. A 4-player board is
    /// 103 columns; the note names that width and points at guide §2, and a
    /// second render does NOT repeat it. (On the staggered pyramid the 3-player
    /// board is only 86 columns, so 4 players is the wide case now.)
    #[test]
    fn board_command_prints_a_one_time_width_note_on_a_wide_board() {
        let mut r = repl_np(4, 7);
        let width =
            render::board_grid(r.session.state()).lines().next().map(|l| l.chars().count()).unwrap_or(0);
        assert!(width > 100, "4p board should exceed 100 columns, got {width}");

        let first = r.step("board");
        assert!(
            first.text.contains(&format!("board is {width} columns wide")),
            "first render must note width: {}",
            first.text
        );
        assert!(first.text.contains("guide §2"), "{}", first.text);

        let second = r.step("board");
        assert!(!second.text.contains("columns wide"), "note must print once per session: {}", second.text);
    }

    /// S3: a narrow board (2 players = 69 columns, <= 100) never triggers the
    /// note, on the first render or any later one.
    #[test]
    fn board_command_omits_the_width_note_on_a_narrow_board() {
        let mut r = repl_2p(4);
        let out = r.step("board");
        assert!(!out.text.contains("columns wide"), "{}", out.text);
    }

    /// Item 7 (board connectivity): every `board` render carries a one-line
    /// legend documenting the `*` = unclaimed 5-Power token mark.
    #[test]
    fn board_command_prints_the_five_power_legend() {
        let mut r = repl_2p(4);
        let out = r.step("board");
        assert!(out.text.contains("* = unclaimed 5-Power token"), "{}", out.text);
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

    /// Codex review Item 1 (I-3/I-4 disposition): `legal_moves.rs`'s
    /// `generate_placement_moves_with_tech` generates a 4-area Option-A
    /// placement once the current player owns Newspaper tech (`max_areas =
    /// 4`) -- this fixture reaches that state directly via
    /// `for_test_from_state` rather than playing a real game up to a
    /// Newspaper acquisition (itself a multi-turn Laboratory visit),
    /// mirroring core's own `test_placement_limit` fixture
    /// (`tech_effects.rs`) for the tech insertion and
    /// `repl_with_any_good_cascade` below for the state-injection style
    /// generally. `Rules::empty_for_tests()` plus ONE inserted Newspaper
    /// card (not `Rules::standard()`) for the same reason
    /// `repl_with_any_good_cascade` documents: `create_test_game_state`'s
    /// empty decks would otherwise fail `check_invariants`'
    /// order/tech-conservation gates, which only fire when the registry
    /// holds the full physical count (36 orders / 30 techs) -- a
    /// single-card registry never trips them.
    fn newspaper_placement_fixture() -> Repl {
        let mut state = create_test_game_state(2);
        state.turn_phase = TurnPhase::Placement;

        let mut areas = std::collections::BTreeMap::new();
        for i in 0u8..4 {
            areas.insert(AreaId::new(i), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, i) });
        }
        state.board_layout = BoardLayout {
            areas,
            adjacency: std::collections::BTreeMap::new(),
            canal_crossings: vec![],
            building_sites: std::collections::BTreeMap::new(),
            five_power_tokens: std::collections::BTreeMap::new(),
        };

        let mut techs = TechRegistry::new();
        techs.insert(
            TechCard {
                id: TechCardId(7),
                name: "Newspaper".to_string(),
                country: Country::Dutch,
                industry_cost: 5,
                effect: TechEffect::Newspaper,
            },
            "tech_newspaper",
        );
        state.players[0].technologies.push(TechCardId(7));

        let rules = Rules { techs, ..Rules::empty_for_tests() };
        Repl::new(Session::for_test_from_state(state, rules))
    }

    /// RED-discriminating (Codex I-3): before this fix, `try_placement_a`
    /// hard-rejected any `a` entry with more than 3 area ids, so this
    /// engine-legal 4-area Newspaper placement was reachable only by its
    /// raw numbered option -- typing the `a` grammar for it failed at parse
    /// time with "Option A takes at most 3 area ids, got 4." instead of
    /// applying.
    #[test]
    fn placement_a_grammar_reaches_newspaper_4_area_placement() {
        let mut r = newspaper_placement_fixture();
        let events_before = r.session.record().events.len();
        let out = r.step("a 0 1 2 3");
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert!(!out.text.contains("at most"), "{}", out.text);
        assert_eq!(
            r.session.record().events.len(), events_before + 1,
            "the 4-area placement must actually apply: {}", out.text
        );
        assert_eq!(r.session.state().turn_phase, TurnPhase::Movement);
    }

    /// A 4-id `a` entry now parses (the count bound moved to 4), but a
    /// player WITHOUT Newspaper never has a 4-area option in `legal()` --
    /// `find_placement_match` is the sole legality authority, so this falls
    /// through to the ordinary "not currently available" rejection, same as
    /// any other combination the engine didn't generate.
    #[test]
    fn placement_a_grammar_rejects_4_ids_without_newspaper() {
        let mut r = repl_2p(4);
        r.step("0"); // -> Placement
        let out = r.step("a 0 1 2 3");
        assert!(out.text.contains("Illegal: that placement is not currently available."), "{}", out.text);
    }

    /// The token-count bound itself moved from 3 to 4, not away entirely --
    /// 5 ids is still a parse-time rejection for everyone, Newspaper or not.
    #[test]
    fn placement_a_grammar_rejects_more_than_four_ids() {
        let mut r = repl_2p(4);
        let out = r.step("a 0 1 2 3 4");
        assert!(out.text.contains("Option A takes at most 4 area ids, got 5."), "{}", out.text);
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

    /// Task 13 test (e): a "view" command (one that only reads current state
    /// -- never changes it) is followed by just the one-line input hint, not
    /// the full header+phase+numbered-options block `prompt()` builds. Closes
    /// a dogfooding friction note: re-showing up to 20 numbered options after
    /// every single glance at the board/hand/log was pure noise since the
    /// glance itself never changes which options are legal.
    #[test]
    fn view_commands_show_only_the_input_hint_not_the_full_options_block() {
        let mut r = repl_2p(4);
        for cmd in ["board", "players", "ports", "labs", "tracks", "sites", "goals", "hand", "score", "log", "help"] {
            let out = r.step(cmd);
            assert!(!out.text.contains("[0]"), "'{cmd}' unexpectedly showed the numbered options block: {}", out.text);
            assert!(
                out.text.trim_end().ends_with("('help' for the list)"),
                "'{cmd}' output is missing the one-line input hint: {}", out.text
            );
        }
    }

    /// Task 16 (T2.5): the `score` view paints its VP totals -- stripping
    /// the escapes back out must reproduce the plain text exactly, and a
    /// view command's painted output must never differ structurally.
    #[test]
    fn forced_color_score_text_strips_back_to_plain() {
        let mut r = repl_2p(4);
        crate::style::force(true);
        let colored = r.step("score").text;
        crate::style::force(false);
        let plain = r.step("score").text;
        assert_ne!(colored, plain, "forced mode must actually paint something");
        assert_eq!(crate::style::test_support::strip_ansi(&colored), plain);
    }

    /// Task 16 (T2.5): the lockdown status line's `[locked]` marker is
    /// painted as a warning -- same strip-equality invariant.
    #[test]
    fn forced_color_lockdown_status_strips_back_to_plain() {
        let mut r = repl_2p(4);
        r.locked = true;
        crate::style::force(true);
        let colored = r.step("log").text;
        crate::style::force(false);
        let plain = r.step("log").text;
        assert_ne!(colored, plain, "forced mode must actually paint something");
        assert_eq!(crate::style::test_support::strip_ansi(&colored), plain);
    }

    /// Task 13 test (f): `help`'s own text should document the `q` alias for
    /// `quit` that `step`'s dispatch already honors (`["quit"] | ["q"]`) --
    /// checked via the quoted form `'q'` (this codebase's convention for a
    /// literal input token, e.g. reveal.rs's "or 'cancel'") rather than a
    /// bare `"q"`, which the word "quit" itself already contains and so
    /// would never have gone RED.
    #[test]
    fn help_text_documents_the_q_quit_alias() {
        let mut r = repl_2p(4);
        let out = r.step("help");
        assert!(out.text.contains("'q'"), "{}", out.text);
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

    /// Codex review Item 2 (page-count footer): the pager already told a
    /// player how many options were left ("...N more"), but never how many
    /// PAGES there were in total, or which one they were currently on --
    /// this pins the new "page N/M" footer on both the first and second
    /// page of a real Placement-phase list (verified >20 options here, so
    /// the pagination path actually engages -- `Placement`'s combinatorial
    /// Option-A enumeration is comfortably over that on a real board).
    #[test]
    fn prompt_shows_page_count_footer_when_paginated() {
        let mut r = repl_2p(4);
        r.step("0"); // -> Placement
        let n = r.session.legal().len();
        assert!(n > 20, "fixture assumption: Placement must offer >20 options here, got {n}");
        let p0 = r.prompt();
        assert!(p0.contains("page 1/"), "{p0}");
        let p1 = r.step("more").text;
        assert!(p1.contains("page 2/"), "{p1}");
    }

    /// Codex review Item 2 (phase-aware hint): the trailing input hint used
    /// to be one static line naming EVERY shorthand regardless of whether
    /// it applied right now. Walks a fresh session through the three phase
    /// buckets the dispatch calls out: a phase with no shorthand at all
    /// (`AdditionalActionFirst`), `Placement` (`a`/`b`), and `Movement`
    /// (`r`) -- each assertion checks both that the CURRENT phase's
    /// shorthand is mentioned and that the OTHER phase's isn't, so this
    /// can't pass on a hint that just lists everything unconditionally.
    #[test]
    fn input_hint_is_phase_aware() {
        let mut r = repl_2p(4);
        let p = r.prompt();
        assert!(p.contains("enter a number, or a command ('help' for the list)"), "{p}");
        assert!(!p.contains("'a'/'b'"), "{p}");
        assert!(!p.contains("'r' to retreat"), "{p}");

        r.step("0"); // -> Placement
        let p = r.prompt();
        assert!(p.contains("'a'/'b' for placement"), "{p}");
        assert!(!p.contains("'r' to retreat"), "{p}");

        r.step("a"); // place nothing -> Movement
        assert_eq!(r.session.state().turn_phase, TurnPhase::Movement, "fixture assumption drifted");
        let p = r.prompt();
        assert!(p.contains("'r' to retreat"), "{p}");
        assert!(!p.contains("'a'/'b'"), "{p}");
    }

    /// Codex review Item 2 (phase-aware hint, AssistantMove case): a
    /// pending `AssistantMove` choice at the head of the queue gets its own
    /// hint naming the `m <from> <to>` grammar and the numbered Decline --
    /// checked against the genuine 2-deep cascade fixture
    /// `drive_to_double_assistant_move_cascade` builds below (real Church
    /// visit, not synthesized `pending_choices`).
    #[test]
    fn input_hint_mentions_m_grammar_during_assistant_move_choice() {
        let mut r = repl_2p(4);
        drive_to_double_assistant_move_cascade(&mut r);
        let p = r.prompt();
        assert!(p.contains("'m <from> <to>'"), "{p}");
        assert!(p.contains("Decline"), "{p}");
    }

    /// Codex review Item 2 (phase-aware hint, "other choices" case): a
    /// pending choice that ISN'T `AssistantMove` (here, `AnyGood`) falls
    /// back to the plain numbers+'help' hint -- it has no shorthand of its
    /// own, and critically must NOT inherit the underlying phase's
    /// shorthand (or `m`'s) just because a choice happens to be queued
    /// mid-phase.
    #[test]
    fn input_hint_is_plain_for_non_assistant_move_pending_choices() {
        let r = repl_with_any_good_cascade();
        let p = r.prompt();
        assert!(p.contains("enter a number, or a command ('help' for the list)"), "{p}");
        assert!(!p.contains("'m <from> <to>'"), "{p}");
        assert!(!p.contains("'a'/'b'"), "{p}");
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

    /// T13 rider (final-review cheap batch): `finish_apply_ex`'s catch-all
    /// used to render `ApplyError::GameOver` (Display: "the game is already
    /// over") as the confusing "Illegal: the game is already over" -- a
    /// dedicated match arm now reports it plainly instead. `Session::legal()`
    /// naturally empties out once the game is over (`TurnPhase::TurnEnd`
    /// offers no moves), so ordinary `step()` dispatch can never reach this
    /// arm -- exercised directly against `finish_apply_ex`, `pub(crate)` for
    /// exactly this kind of defensive-API coverage (same reasoning
    /// `Session::apply`'s own `is_over()` gate documents). Fixture mirrors
    /// `session.rs`'s `apply_on_finished_game_returns_typed_game_over`.
    #[test]
    fn finish_apply_ex_reports_game_over_plainly_not_illegal_prefixed() {
        let mut state = create_test_game_state(2);
        state.end_game_triggered = true;
        state.trigger_player = Some(PlayerId::new(0));
        state.trigger_reason = Some(yokohama_core::EndGameTrigger::OrderDeckEmpty);
        state.final_round = true;
        state.turn_phase = TurnPhase::TurnEnd;
        state.current_player = PlayerId::new(1);
        assert!(is_game_over(&state), "fixture must actually satisfy is_game_over");

        let mut r = Repl::new(Session::for_test_from_state(state, Rules::empty_for_tests()));
        let (out, applied) = r.finish_apply_ex(Move::EndAdditionalPhase, vec![]);
        assert!(!applied, "GameOver must report applied = false, same as every other error");
        assert!(out.text.contains("game over"), "{}", out.text);
        assert!(!out.text.contains("Illegal"), "{}", out.text);
    }

    /// Task 10: once the game is over, `score` announces the winner and a
    /// full per-player breakdown (`final_result`'s `ScoreBreakdown`), not
    /// just a bare VP total -- this is the bot phase's terminal utility, so
    /// the CLI needs to show the WHY behind the final number, not just the
    /// number. Fixture mirrors
    /// `finish_apply_ex_reports_game_over_plainly_not_illegal_prefixed`
    /// immediately above.
    #[test]
    fn game_over_screen_announces_winner_and_breakdown() {
        let mut state = create_test_game_state(2);
        state.end_game_triggered = true;
        state.trigger_player = Some(PlayerId::new(0));
        state.trigger_reason = Some(yokohama_core::EndGameTrigger::OrderDeckEmpty);
        state.final_round = true;
        state.turn_phase = TurnPhase::TurnEnd;
        state.current_player = PlayerId::new(1);
        state.players[0].victory_points = 20;
        state.players[1].victory_points = 5;
        assert!(is_game_over(&state), "fixture must actually satisfy is_game_over");

        let mut r = Repl::new(Session::for_test_from_state(state, Rules::empty_for_tests()));
        let out = r.step("score");
        assert!(out.text.contains("WINNER"), "{}", out.text);
        assert!(out.text.contains("church"), "breakdown lines expected: {}", out.text);
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

    /// Task 13 rider (T11 review): the checkpoint-fold path (`Session::undo`'s
    /// `for_test_from_state` branch, now delegating to core's `replay_from`
    /// instead of a hand-kept CLI-side mirror) exercised with a NON-empty
    /// events slice. The sibling test above only ever undoes back to an
    /// EMPTY record (a zero-event fold, since it resolves just ONE of the two
    /// choices before undoing) -- this resolves BOTH, so the first undo must
    /// fold exactly 1 event (the first `ChooseAnyGood`) back over the
    /// checkpoint to re-derive the one-choice-pending state, not just return
    /// the checkpoint verbatim.
    #[test]
    fn undo_after_both_any_good_choices_folds_a_non_empty_checkpoint_slice() {
        let mut r = repl_with_any_good_cascade();
        assert_eq!(r.session.state().pending_choices.len(), 2);

        let k1 = r.session.legal().iter()
            .position(|m| matches!(m, Move::ChooseAnyGood { good: TradeGood::Copper }))
            .expect("Copper is always one of the four AnyGood options");
        r.step(&k1.to_string());
        assert_eq!(r.session.state().pending_choices.len(), 1);

        let k2 = r.session.legal().iter()
            .position(|m| matches!(m, Move::ChooseAnyGood { good: TradeGood::Silk }))
            .expect("Silk is always one of the four AnyGood options");
        r.step(&k2.to_string());
        assert!(r.session.state().pending_choices.is_empty());
        assert_eq!(r.session.record().events.len(), 2);

        // Undo #1: folds exactly 1 event (the first ChooseAnyGood) over the
        // checkpoint -- the non-empty fold this test targets.
        let out = r.step("undo");
        assert!(out.text.contains("Undid"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), 1);
        assert_eq!(
            r.session.state().pending_choices.len(), 1,
            "re-shown state must be exactly what folding 1 event over the checkpoint produces"
        );

        // Undo #2: folds 0 events -- back to the checkpoint verbatim.
        let out = r.step("undo");
        assert!(out.text.contains("Undid"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), 0);
        assert_eq!(r.session.state().pending_choices.len(), 2);

        // Undo #3: nothing left to undo.
        let out = r.step("undo");
        assert!(out.text.contains("Cannot undo"), "{}", out.text);
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

    /// Turn-1 setup for the retreat (`r`) grammar tests: reach Movement
    /// phase via a real, affordable Option-B placement (2 assistants, one
    /// area) -- same `.position()`-search walking style
    /// `drive_to_double_assistant_move_cascade` uses, but stops right after
    /// the placement itself instead of moving the President or triggering
    /// an AssistantMove cascade, since retreat only needs on-board
    /// assistants to recover.
    ///
    /// Deliberately does NOT try to get the President onto the board first:
    /// every player's `president_location` starts `InHand`
    /// (`PlayerState::new`'s default) and stays that way across the
    /// Placement -> Movement transition (`set_president_start_area` only
    /// records the OLD location for movement-fee bookkeeping, and
    /// `PlaceTwoSameArea`'s apply arm never touches `president_location`),
    /// so a genuine "President already on board" Movement decision needs a
    /// full extra turn round-trip to reach. `ReturnPresidentToHand` is
    /// offered either way -- `generate_movement_moves_with_tech`'s
    /// `enumerate_recovery_options(p)` call is identical in both the
    /// `InHand` and `OnBoard` arms -- so this simpler turn-1 fixture is
    /// enough to exercise the grammar. Returns the area the 2 assistants
    /// landed in.
    fn drive_to_movement_with_on_board_assistants(r: &mut Repl) -> AreaId {
        if !r.session.legal().iter().any(|m| matches!(m, Move::PlaceTwoSameArea { .. })) {
            let end = r.session.legal().iter().position(|m| matches!(m, Move::EndAdditionalPhase))
                .expect("EndAdditionalPhase reaches Placement");
            r.step(&end.to_string());
        }
        let legal = r.session.legal();
        let place = legal.iter().position(|m| matches!(m, Move::PlaceTwoSameArea { adjacent_bonus: None, .. }))
            .expect("some Option B placement is legal and affordable turn 1 (no President is on board yet to charge a fee)");
        let area = match &legal[place] {
            Move::PlaceTwoSameArea { area, .. } => *area,
            _ => unreachable!("position() above only matches PlaceTwoSameArea"),
        };
        r.step(&place.to_string());
        assert_eq!(r.session.state().turn_phase, TurnPhase::Movement, "placement must reach Movement phase");
        area
    }

    #[test]
    fn r_grammar_builds_the_exact_retreat_move_and_applies_it() {
        let mut r = repl_2p(4);
        let area = drive_to_movement_with_on_board_assistants(&mut r);
        let me = r.session.state().current_player;
        let events_before = r.session.record().events.len();

        let out = r.step(&format!("r {}:1", area.0));
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert!(!out.text.contains("not in the legal list"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert!(matches!(
            r.session.record().events.last(),
            Some(RecordedEvent::Move(Move::ReturnPresidentToHand { recover }))
                if recover.len() == 1 && recover.get(&area) == Some(&1)
        ), "{:?}", r.session.record().events.last());
        assert_eq!(r.session.state().player(me).assistants.in_area(area), 1);
    }

    #[test]
    fn r_with_no_args_is_the_recover_nothing_retreat() {
        let mut r = repl_2p(4);
        let area = drive_to_movement_with_on_board_assistants(&mut r);
        let me = r.session.state().current_player;
        let events_before = r.session.record().events.len();

        let out = r.step("r");
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert!(matches!(
            r.session.record().events.last(),
            Some(RecordedEvent::Move(Move::ReturnPresidentToHand { recover })) if recover.is_empty()
        ), "{:?}", r.session.record().events.last());
        // Recovering nothing leaves both on-board assistants exactly where
        // they were -- only the President itself (already in hand turn 1;
        // see the fixture's doc comment) would move.
        assert_eq!(r.session.state().player(me).assistants.in_area(area), 2);
    }

    #[test]
    fn r_with_bad_count_reports_not_legal_without_applying() {
        let mut r = repl_2p(4);
        let area = drive_to_movement_with_on_board_assistants(&mut r);
        let events_before = r.session.record().events.len();
        let state_before = r.session.state().clone();

        // Only 2 assistants are on board there -- 9 is never legal.
        let out = r.step(&format!("r {}:9", area.0));
        assert!(out.text.contains("not in the legal list"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before);
        assert_eq!(*r.session.state(), state_before);
    }

    /// Turn-1 setup for the retreat grammar's multi-token case: an Option-A
    /// placement of exactly 2 areas (1 assistant each) instead of
    /// `drive_to_movement_with_on_board_assistants`'s single-area/2-count
    /// shape -- needed to exercise `try_retreat`'s loop across MORE THAN
    /// ONE `rest` token (the "..." in `r [<area>[:<n>]]...`), which none of
    /// the single-token tests above touch. Returns the two occupied areas.
    fn drive_to_movement_with_two_occupied_areas(r: &mut Repl) -> (AreaId, AreaId) {
        if !r.session.legal().iter().any(|m| matches!(m, Move::PlaceThreeAreas { .. })) {
            let end = r.session.legal().iter().position(|m| matches!(m, Move::EndAdditionalPhase))
                .expect("EndAdditionalPhase reaches Placement");
            r.step(&end.to_string());
        }
        let legal = r.session.legal();
        let place = legal.iter()
            .position(|m| matches!(m, Move::PlaceThreeAreas { placements } if placements.len() == 2))
            .expect("a 2-area Option A placement is legal and affordable turn 1 (same reasoning as the Option B fixture -- no President is on board yet to charge a fee)");
        let (a, b) = match &legal[place] {
            Move::PlaceThreeAreas { placements } => (placements[0], placements[1]),
            _ => unreachable!("position() above only matches PlaceThreeAreas"),
        };
        r.step(&place.to_string());
        assert_eq!(r.session.state().turn_phase, TurnPhase::Movement, "placement must reach Movement phase");
        (a, b)
    }

    #[test]
    fn r_grammar_combines_multiple_area_tokens_into_one_retreat() {
        let mut r = repl_2p(4);
        let (a, b) = drive_to_movement_with_two_occupied_areas(&mut r);
        let me = r.session.state().current_player;
        let events_before = r.session.record().events.len();

        // Mix a bare token (recover ALL there -- 1) with an explicit `:1`
        // for the other area, in a single command.
        let out = r.step(&format!("r {} {}:1", a.0, b.0));
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert!(!out.text.contains("not in the legal list"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert!(matches!(
            r.session.record().events.last(),
            Some(RecordedEvent::Move(Move::ReturnPresidentToHand { recover }))
                if recover.len() == 2 && recover.get(&a) == Some(&1) && recover.get(&b) == Some(&1)
        ), "{:?}", r.session.record().events.last());
        assert_eq!(r.session.state().player(me).assistants.in_area(a), 0);
        assert_eq!(r.session.state().player(me).assistants.in_area(b), 0);
    }

    // ===================================================================
    // Pre-gate Task A: Movement family collapse + sub-prompts.
    //
    // Fixture: a hand-built Movement-phase state (mirrors describe.rs's
    // `movement_cost_fixture` geometry, but with the current player's OWN
    // assistants placed so `get_legal_moves` actually enumerates the routes
    // and the recovery combinations):
    //
    //   A00 Bank         Red's President sits here (the hub); Red: 2 assistants
    //   A01 Copper Mine  a MULTI-LABEL destination -- reachable by the direct
    //                    canal crossing A0->A1 (1 canal toll to the bank) AND
    //                    the long way A0->A2->A1 (1 fee to Blue, whose
    //                    President blocks A2). Two distinct (destination, fee)
    //                    labels -> one "2 route options" entry + route sub-prompt.
    //   A02 Silk Mill    Blue's President sits here (pass-through only, never a
    //                    stop) -- the source of the fee label above.
    //   A03 Tea Plantation  a SINGLE-LABEL destination (only A0->A3, free) ->
    //                       stays a plain numbered move.
    //
    // Red also has 1 assistant each in A1/A2/A3, so the retreat family is the
    // full 3*2*2*2 = 24-combination Cartesian product -> one collapsed
    // "choose recovery" entry + the guided retreat sub-prompt.
    //
    // `Rules::empty_for_tests()` (empty registries) so `check_invariants`'
    // order/tech conservation gates stay off; the per-player assistant total
    // is held at 20 by draining the warehouse to match the 6 placed on board.
    fn movement_grouping_fixture() -> Repl {
        use std::collections::BTreeMap;
        let a0 = AreaId::new(0);
        let a1 = AreaId::new(1);
        let a2 = AreaId::new(2);
        let a3 = AreaId::new(3);

        let mut areas = BTreeMap::new();
        areas.insert(a0, AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(a1, AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });
        areas.insert(a2, AreaConfig { area_type: AreaType::SilkMill, grid_position: (1, 0) });
        areas.insert(a3, AreaConfig { area_type: AreaType::TeaPlantation, grid_position: (1, 1) });

        let mut adjacency = BTreeMap::new();
        adjacency.insert(a0, vec![a1, a2, a3]);
        adjacency.insert(a1, vec![a0, a2]);
        adjacency.insert(a2, vec![a0, a1]);
        adjacency.insert(a3, vec![a0]);

        let board_layout = BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![(a0, a1)],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };

        let config = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));
        let mut players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];
        players[0].president_location = PresidentLocation::OnBoard(a0);
        players[0].money = 10;
        players[0].assistants.on_board.insert(a0, 2);
        players[0].assistants.on_board.insert(a1, 1);
        players[0].assistants.on_board.insert(a2, 1);
        players[0].assistants.on_board.insert(a3, 1);
        players[0].assistants.in_warehouse -= 5; // 5 placed on board -> keep the per-player total at 20
        players[1].president_location = PresidentLocation::OnBoard(a2); // Blue blocks A2
        players[1].money = 10;

        let state = GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::Movement,
            turn_number: 1,
            additional_action_state: AdditionalActionState::new(),
            current_action_power: None,
            is_mercenary_action: false,
            mercenary_target_area: None,
            mercenary_return_phase: None,
            president_start_area: None,
            port_state: PortState::new(false),
            lab_state: LabState::new(false),
            order_deck: vec![],
            tech_deck: vec![],
            orders_removed: vec![],
            church_track: ChurchTrack::new(),
            customs_track: CustomsTrack::new(),
            achievements: [
                AchievementCard::new(AchievementType::Copper5),
                AchievementCard::new(AchievementType::Money10),
                AchievementCard::new(AchievementType::Production4),
            ],
            mercenary_supply: MercenarySupply::new(),
            station_location: None,
            end_game_triggered: false,
            trigger_reason: None,
            final_round: false,
            trigger_player: None,
            pending_choices: Vec::new(),
        };
        Repl::new(Session::for_test_from_state(state, Rules::empty_for_tests()))
    }

    /// Movement-phase state with the President on the board but NO on-board
    /// assistants for the current player: the ONLY legal move is the bare
    /// `ReturnPresidentToHand {}` (recover nothing) -- exactly one retreat
    /// variant, so it must stay a DIRECT numbered entry (no collapse, no
    /// guided sub-prompt hop). Used by `bare_retreat_only_state_keeps_direct_entry`.
    fn movement_bare_retreat_fixture() -> Repl {
        use std::collections::BTreeMap;
        let a0 = AreaId::new(0);
        let mut areas = BTreeMap::new();
        areas.insert(a0, AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        let mut adjacency = BTreeMap::new();
        adjacency.insert(a0, vec![]);
        let board_layout = BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };
        let config = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));
        let mut players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];
        players[0].president_location = PresidentLocation::OnBoard(a0); // on board, but no assistants anywhere
        let state = GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::Movement,
            turn_number: 1,
            additional_action_state: AdditionalActionState::new(),
            current_action_power: None,
            is_mercenary_action: false,
            mercenary_target_area: None,
            mercenary_return_phase: None,
            president_start_area: None,
            port_state: PortState::new(false),
            lab_state: LabState::new(false),
            order_deck: vec![],
            tech_deck: vec![],
            orders_removed: vec![],
            church_track: ChurchTrack::new(),
            customs_track: CustomsTrack::new(),
            achievements: [
                AchievementCard::new(AchievementType::Copper5),
                AchievementCard::new(AchievementType::Money10),
                AchievementCard::new(AchievementType::Production4),
            ],
            mercenary_supply: MercenarySupply::new(),
            station_location: None,
            end_game_triggered: false,
            trigger_reason: None,
            final_round: false,
            trigger_player: None,
            pending_choices: Vec::new(),
        };
        Repl::new(Session::for_test_from_state(state, Rules::empty_for_tests()))
    }

    /// Convenience: the raw legal-list positions the fixture pins, so the
    /// tests can assert against the actual engine enumeration rather than
    /// re-deriving it. Returns (multi-label dest, single-label dest).
    fn grouping_fixture_dests() -> (AreaId, AreaId) {
        (AreaId::new(1), AreaId::new(3))
    }

    /// Main-list grouping: one numbered entry per destination -- the
    /// multi-label destination collapses to a "N route options" line, the
    /// single-label one renders as the move itself, and the whole retreat
    /// family collapses to one "choose recovery" line. Three entries total.
    #[test]
    fn movement_list_groups_destinations_and_collapses_retreats() {
        let r = movement_grouping_fixture();
        let (multi, single) = grouping_fixture_dests();
        let state = r.session.state();
        let rules = r.session.rules();
        let p = r.prompt();

        // Entry 0: the multi-label destination as a route-options line.
        assert!(
            p.contains(&format!("[0] Move President to {} — 2 route options", crate::describe::area_label(state, multi))),
            "{p}"
        );
        // Entry 1: the single-label destination rendered exactly as the move.
        let single_move = Move::MovePresident { path: vec![single], destination: single };
        assert!(
            p.contains(&format!("[1] {}", crate::describe::describe_move(&single_move, state, rules))),
            "{p}"
        );
        // Entry 2: the collapsed retreat family (24 = 3*2*2*2 combinations).
        assert!(
            p.contains("[2] Retreat President to hand — choose recovery (24 combinations; 'r' grammar works too)"),
            "{p}"
        );
        // Exactly three entries -- no [3], and none of the 27 raw legal moves
        // (3 MovePresident + 24 retreats) leaked through ungrouped.
        assert!(!p.contains("[3]"), "grouping must leave exactly 3 entries: {p}");
        assert_eq!(r.session.legal().len(), 27, "fixture assumption: 3 MovePresident + 24 retreats");
    }

    /// The Movement hint keeps the `'r' to retreat` shorthand AND now names
    /// the numbered retreat entry (brief item 6), all on one line.
    #[test]
    fn movement_hint_names_the_retreat_entry() {
        let r = movement_grouping_fixture();
        let p = r.prompt();
        assert!(p.contains("'r' to retreat"), "{p}");
        assert!(p.contains("retreat entry"), "{p}");
    }

    /// Route sub-prompt: selecting the multi-label entry lists exactly that
    /// destination's two labels with their unchanged cost/recipient text
    /// (canal toll vs. fee-to-Blue), and a number commits the chosen move.
    #[test]
    fn route_subprompt_lists_labels_then_commits() {
        let mut r = movement_grouping_fixture();
        let (multi, _single) = grouping_fixture_dests();

        let out = r.step("0"); // enter the route sub-prompt for the multi-label dest
        assert!(out.text.contains(&format!("Routes to {}", crate::describe::area_label(r.session.state(), multi))), "{}", out.text);
        // Both labels, with the SAME cost/recipient wording describe_move uses.
        assert!(out.text.contains("[0]") && out.text.contains("[1]"), "{}", out.text);
        assert!(out.text.contains("canal toll"), "canal-toll route label missing: {}", out.text);
        assert!(out.text.contains("to Blue"), "fee-to-Blue route label missing: {}", out.text);
        assert!(!out.text.contains("[2]"), "exactly two routes to this destination: {}", out.text);

        // A number commits that exact MovePresident (assert via the applied event).
        let events_before = r.session.record().events.len();
        let out = r.step("0");
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert!(matches!(
            r.session.record().events.last(),
            Some(RecordedEvent::Move(Move::MovePresident { destination, .. })) if *destination == multi
        ), "{:?}", r.session.record().events.last());
    }

    /// Route sub-prompt `cancel` returns cleanly to the main list with the
    /// full grouped list re-shown, applying nothing.
    #[test]
    fn route_subprompt_cancel_restores_main_list() {
        let mut r = movement_grouping_fixture();
        let events_before = r.session.record().events.len();
        let state_before = r.session.state().clone();

        r.step("0"); // into the route sub-prompt
        let out = r.step("cancel");
        assert!(out.text.to_lowercase().contains("cancel"), "{}", out.text);
        // The full grouped main list is back (the route-options entry again).
        assert!(out.text.contains("2 route options"), "main list must be re-shown: {}", out.text);
        assert_eq!(r.session.record().events.len(), events_before, "cancel applies nothing");
        assert_eq!(*r.session.state(), state_before);
    }

    /// Guided retreat: the collapsed retreat entry opens a per-area question
    /// sequence (ascending AreaId), then a confirm line; confirming applies
    /// the exact `ReturnPresidentToHand { recover }` the answers assemble.
    #[test]
    fn guided_retreat_collects_counts_confirms_and_applies() {
        let mut r = movement_grouping_fixture();
        let (a0, a1, a2, a3) = (AreaId::new(0), AreaId::new(1), AreaId::new(2), AreaId::new(3));

        // Enter the guided retreat (entry [2]).
        let out = r.step("2");
        assert!(out.text.contains(&format!("recover how many from {}", crate::describe::area_label(r.session.state(), a0))), "{}", out.text);
        assert!(out.text.contains("[0-2]"), "A00 holds 2 assistants: {}", out.text);

        // Answer each area in ascending order: 2 from A0, 1 from A1, 1 from A2, 1 from A3.
        let out = r.step("2");
        assert!(out.text.contains(&format!("recover how many from {}", crate::describe::area_label(r.session.state(), a1))), "{}", out.text);
        let out = r.step("1");
        assert!(out.text.contains(&format!("recover how many from {}", crate::describe::area_label(r.session.state(), a2))), "{}", out.text);
        let out = r.step("1");
        assert!(out.text.contains(&format!("recover how many from {}", crate::describe::area_label(r.session.state(), a3))), "{}", out.text);

        // Last answer -> the confirm line shows the assembled move via describe_move.
        let out = r.step("1");
        assert!(out.text.contains("Return President to hand, recovering 5 assistant(s)"), "{}", out.text);
        assert!(out.text.contains("confirm"), "confirm prompt expected: {}", out.text);

        let events_before = r.session.record().events.len();
        let out = r.step("confirm");
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        let expected: std::collections::BTreeMap<AreaId, u8> =
            [(a0, 2u8), (a1, 1), (a2, 1), (a3, 1)].into_iter().collect();
        assert!(matches!(
            r.session.record().events.last(),
            Some(RecordedEvent::Move(Move::ReturnPresidentToHand { recover })) if *recover == expected
        ), "{:?}", r.session.record().events.last());
    }

    /// Guided retreat re-asks the SAME question on an out-of-range count
    /// (count-exceeding-available: A00 holds 2, so 3 is refused) and on
    /// garbage, applying nothing until a valid answer arrives.
    #[test]
    fn guided_retreat_reasks_on_out_of_range_and_garbage() {
        let mut r = movement_grouping_fixture();
        let a0 = AreaId::new(0);
        r.step("2"); // enter -> first question is A00 [0-2]

        // Out of range (3 > 2 available): re-ask, nothing collected.
        let out = r.step("3");
        assert!(out.text.contains("0-2"), "out-of-range must re-ask with the range: {}", out.text);
        assert!(out.text.contains(&format!("recover how many from {}", crate::describe::area_label(r.session.state(), a0))), "still on A00's question: {}", out.text);

        // Garbage: re-ask, still on A00's question.
        let out = r.step("banana");
        assert!(out.text.contains(&format!("recover how many from {}", crate::describe::area_label(r.session.state(), a0))), "garbage must re-ask A00's question: {}", out.text);

        // A valid answer now advances to A01's question.
        let out = r.step("2");
        assert!(out.text.contains(&format!("recover how many from {}", crate::describe::area_label(r.session.state(), AreaId::new(1)))), "{}", out.text);
        assert!(r.session.record().events.is_empty(), "nothing may apply during collection");
    }

    /// Guided retreat `cancel` mid-questions returns to the main list,
    /// applying nothing.
    #[test]
    fn guided_retreat_cancel_mid_questions_restores_main_list() {
        let mut r = movement_grouping_fixture();
        let state_before = r.session.state().clone();
        r.step("2");   // enter
        r.step("2");   // answer A00
        let out = r.step("cancel");
        assert!(out.text.to_lowercase().contains("cancel"), "{}", out.text);
        assert!(out.text.contains("choose recovery"), "main list must be re-shown: {}", out.text);
        assert!(r.session.record().events.is_empty());
        assert_eq!(*r.session.state(), state_before);
    }

    /// Sub-prompt input discipline: a view command mid-collection re-asks
    /// with `! finish or cancel this prompt first` and never renders the view
    /// (a board glance would be against a state the collection doesn't reflect).
    #[test]
    fn subprompt_view_command_reasks_with_finish_or_cancel() {
        let mut r = movement_grouping_fixture();
        r.step("2"); // enter the guided retreat
        let out = r.step("board");
        assert!(out.text.contains("! finish or cancel this prompt first"), "{}", out.text);
        // The view did NOT render, and the question is still showing.
        assert!(!out.text.contains("grid"), "the board view must not render mid-collection: {}", out.text);
        assert!(out.text.contains("recover how many from"), "the question must be re-shown: {}", out.text);
        assert!(r.session.record().events.is_empty());
    }

    /// Lockdown wins over an in-progress sub-prompt: once locked, even
    /// sub-prompt input is refused the same way moves are (only save/log/quit).
    #[test]
    fn subprompt_refused_under_lockdown() {
        let mut r = movement_grouping_fixture();
        r.step("2"); // enter the guided retreat
        r.locked = true;
        let out = r.step("2"); // would be an answer, but lockdown refuses it
        assert!(out.text.contains("locked"), "{}", out.text);
        assert!(r.session.record().events.is_empty());
    }

    /// Regression: the `r` grammar still assembles + applies the exact retreat
    /// with the new grouping active -- it BYPASSES the guided sub-prompt (no
    /// hop), matching straight against the legal list.
    #[test]
    fn r_grammar_bypasses_the_guided_flow_with_grouping_active() {
        let mut r = movement_grouping_fixture();
        let a1 = AreaId::new(1);
        let events_before = r.session.record().events.len();
        let out = r.step(&format!("r {}:1", a1.0));
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert!(!out.text.contains("not in the legal list"), "{}", out.text);
        // Black-box proof of bypass: the retreat applied in ONE step, and no
        // guided per-area question or confirm line ever appeared.
        assert!(!out.text.contains("recover how many") && !out.text.contains("confirm"), "the r grammar must not open the guided sub-prompt: {}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert!(matches!(
            r.session.record().events.last(),
            Some(RecordedEvent::Move(Move::ReturnPresidentToHand { recover }))
                if recover.len() == 1 && recover.get(&a1) == Some(&1)
        ), "{:?}", r.session.record().events.last());
    }

    /// Bare-retreat-only Movement state (no on-board assistants): exactly one
    /// retreat variant, so it stays a DIRECT numbered entry -- picking it
    /// commits immediately with no guided sub-prompt.
    #[test]
    fn bare_retreat_only_state_keeps_direct_entry() {
        let mut r = movement_bare_retreat_fixture();
        let p = r.prompt();
        assert!(p.contains("[0] Return President to hand (recover no assistants)"), "{p}");
        assert!(!p.contains("choose recovery"), "a lone retreat must NOT collapse: {p}");
        assert!(!p.contains("[1]"), "a lone-retreat state has exactly one entry: {p}");

        let events_before = r.session.record().events.len();
        let out = r.step("0");
        assert!(!out.text.contains("recover how many") && !out.text.contains("confirm"), "a direct bare retreat must not hop into a sub-prompt: {}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert!(matches!(
            r.session.record().events.last(),
            Some(RecordedEvent::Move(Move::ReturnPresidentToHand { recover })) if recover.is_empty()
        ), "{:?}", r.session.record().events.last());
    }

    /// Task A review Minor 1: a route sub-prompt list uses the SAME 20/page +
    /// footer machinery the main list does (brief item 5). Route lists are
    /// bounded ~16 in real play, so this is defensive -- the state is
    /// constructed directly with 25 rendering-only labels (never committed, so
    /// they need not be legal) to exercise the cap + footer + `more`/`list`.
    #[test]
    fn route_subprompt_pages_a_long_route_list() {
        let mut r = repl_2p(4);
        let moves: Vec<Move> =
            (0..25).map(|i| Move::MovePresident { path: vec![], destination: AreaId::new((i % 10) as u8) }).collect();
        r.sub = SubPrompt::Route { destination: AreaId::new(0), moves };

        // Page 1 caps at 20 entries ([0]..[19]) and shows both footer lines.
        let p0 = r.sub_prompt_text();
        assert!(p0.contains("[0] ") && p0.contains("[19] "), "{p0}");
        assert!(!p0.contains("[20] "), "page 1 must cap at 20 route entries: {p0}");
        assert!(p0.contains("...5 more (type 'more')"), "{p0}");
        assert!(p0.contains("page 1/2"), "{p0}");

        // `more` pages forward to the remaining 5.
        let p1 = r.step("more").text;
        assert!(p1.contains("[20] ") && p1.contains("[24] "), "{p1}");
        assert!(!p1.contains("[19] "), "page 2 must not repeat page-1 entries: {p1}");
        assert!(p1.contains("page 2/2"), "{p1}");

        // `list` resets to page 1.
        let p2 = r.step("list").text;
        assert!(p2.contains("[0] ") && p2.contains("page 1/2"), "{p2}");
        // Still in the sub-prompt, nothing applied.
        assert!(matches!(r.sub, SubPrompt::Route { .. }));
        assert!(r.session.record().events.is_empty());
    }

    /// Task A review Minor 2: `cancel` now also aborts a route/retreat
    /// sub-prompt (both modes), so `help` must say so, not just "reveal prompt".
    #[test]
    fn help_text_cancel_line_covers_sub_prompts() {
        let mut r = repl_2p(4);
        let out = r.step("help");
        assert!(out.text.contains("sub-prompt"), "{}", out.text);
    }
}
