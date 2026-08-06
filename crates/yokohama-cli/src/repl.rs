//! Pure REPL step function over a `Session`: numbered-option prompts,
//! placement grammar, view/utility commands, and pagination -- live-mode
//! reveal choreography lives in `reveal.rs` (moved there at Task 10)
//! (design docs/plans/2026-07-18-phase2-cli-driver-design.md).
//! No stdin/stdout -- `main.rs` (Task 13) drives this with real I/O.

use yokohama_core::*;

use crate::input::parse_order_id;
use crate::reveal::{lab_b_changed, refilled_port_slots, Pending};
use crate::session::{ApplyError, TrackingMode};
use crate::style::{paint, Tone};
use crate::{describe, render, screen};

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
    /// Composed-frame flags (Task 6): `composed` opts this session into the
    /// TTY frame (`screen` command toggles it; only effective when
    /// `screen::enabled()` -- a pipe never frames regardless), `show_panel`
    /// keeps the players box inside that frame (`panel` toggles).
    pub composed: bool,
    pub show_panel: bool,
    /// Privacy flag (Task 6): when set, hand CONTENTS mask to counts in the
    /// `hand` and `players` views (the kept starting order lives in the
    /// hand). Defaults ON in live mode -- the laptop gets passed around a
    /// real table -- and OFF in simulation. `privacy` toggles.
    ///
    /// It governs the HUMAN seats only. A bot's hand is masked whatever this
    /// says (`seat_display`) -- see `bot.rs` for what that mask is and is not.
    pub privacy: bool,
    /// Who plays each seat (Phase 3.2 Task 12, spec §9). Always read back out
    /// of the record's run provenance in [`Repl::new`], so a fresh wizard game
    /// and a resumed save take ONE path; empty (all-human) for every game the
    /// wizard was not asked about, which makes every bot path below a no-op.
    pub(crate) bots: crate::bot::BotSeats,
}

/// One line in the Movement phase's collapsed numbered list
/// (docs/plans/2026-07-18-phase2-cli-driver-design.md, destination-first).
/// Outside Movement -- and inside it whenever a choice is
/// queued -- every option is a bare `Direct`, so the list is byte-identical to
/// the ungrouped `legal()` rendering; the two collapsing variants only ever
/// appear in the Movement main list. The legal list stays the SOLE authority:
/// `Route`/`Retreat` are entry helpers whose selections are exact-matched
/// against `legal()` before anything commits (`find_placement_match`).
enum PromptOption {
    /// A single move rendered and committed exactly as today (`describe_move`).
    Direct(Move),
    /// A walk/teleport destination reachable by >1 route label -- selecting it
    /// opens a numbered pick over `moves` (all `MovePresident` to `destination`).
    Route { destination: AreaId, moves: Vec<Move> },
    /// The whole retreat family (>1 `ReturnPresidentToHand` variant) collapsed to
    /// one line; `count` is the family size, shown in the entry text. Selecting it
    /// opens the guided retreat sub-prompt (per-area recovery counts, recomputed
    /// from state -- the moves themselves are never cloned into the entry, so a
    /// pathological 100k-variant retreat family stays cheap to display).
    Retreat { count: usize },
    /// Placement Option A collapsed (Task 6 picker): the whole non-empty
    /// `PlaceThreeAreas` family behind one entry; selecting it opens the
    /// area-toggle picker. `max_areas` is the largest placement the engine
    /// offered (3, 4 with Newspaper, fewer when the hand runs short).
    PlacementA { max_areas: usize },
    /// Placement Option B collapsed: one entry over the unique eligible
    /// areas; selecting it opens a numbered area pick (then a Telegram
    /// bonus pick when >1 variant exists for the chosen area).
    PlacementB { areas: Vec<AreaId> },
    /// A pending assistant-move choice's options for ONE source, collapsed
    /// (>1 destination); selecting it opens a numbered destination pick.
    MoveFrom { from: MoveSource, moves: Vec<Move> },
}

/// An in-progress Movement sub-prompt. Both variants are ENTRY HELPERS: whatever
/// they assemble is exact-matched against `session.legal()` before `commit`, so
/// neither can construct-and-apply anything the engine didn't offer.
pub(crate) enum SubPrompt {
    None,
    /// A numbered pick over a captured slice of the legal list (route labels,
    /// assistant-move destinations, Telegram-bonus variants): a numeric
    /// answer commits `moves[k]` directly. `title` heads the list screen.
    Pick { title: String, moves: Vec<Move> },
    /// Guided recovery: `areas` are the current player's occupied areas ascending
    /// (AreaId, count-there), captured at entry (state is frozen until commit);
    /// `collected` grows one answer per area. Once `collected.len() == areas.len()`
    /// the confirm line shows, and `confirm` assembles + exact-matches the
    /// `ReturnPresidentToHand { recover }`.
    Retreat { areas: Vec<(AreaId, u8)>, collected: Vec<u8> },
    /// Placement Option A picker (Task 6): toggle areas by id off the board
    /// view; `confirm` multiset-matches the assembled `PlaceThreeAreas`
    /// against the legal list. `max_areas` is display guidance only -- the
    /// legal-list match stays the sole authority.
    PlaceA { selected: Vec<AreaId>, max_areas: usize },
    /// Placement Option B picker: `areas` are the unique eligible areas,
    /// addressed BY AREA ID exactly as they are displayed (C1, third review
    /// -- a list index here silently placed in the wrong area for every id
    /// past the Canal). `selected` holds the id typed so far so the screen
    /// can preview the move before `confirm` commits it (or opens a `Pick`
    /// over that area's Telegram-bonus variants), mirroring Option A's
    /// select -> preview -> confirm shape.
    PlaceB { areas: Vec<AreaId>, selected: Option<AreaId> },
    /// The `view` game-info menu (owner scope item 10, discoverability): a
    /// ONE-SHOT numbered menu over the read-only state views -- picking a
    /// number renders that surface and closes the menu. Purely an entry
    /// surface over the same render functions the direct commands call, so
    /// the two can never disagree.
    ViewMenu,
}

/// The result of one `Repl::step` call: the text to show the user (which
/// ends with the next prompt, unless `quit`), and whether the REPL should
/// exit.
pub struct StepOutput {
    pub text: String,
    pub quit: bool,
}

/// One game-info menu row: the label the row shows and the render fn it
/// dispatches to (see [`Repl::VIEW_MENU`]).
type ViewEntry = (&'static str, fn(&mut Repl) -> StepOutput);

/// What one `Session::apply` produced, before any prompt or pump is added --
/// the shared currency of `Repl::apply_and_describe`'s two callers.
enum ApplyOutcome {
    /// Applied; carries `delta_summary`'s lines.
    Applied(Vec<String>),
    /// Not applied; carries the text to show (which, for a lockdown, is the
    /// notice plus the postmortem dump -- `self.locked` is already set).
    Failed(String),
}

/// Most bot moves one pump may apply before halting loudly (spec §9). A bot
/// turn is many decision points -- tens, not hundreds -- so this is a runaway
/// guard, not a budget: hitting it means a bot is not handing over, and the
/// honest response is to stop with the game intact rather than spin.
const PUMP_MOVE_CAP: usize = 200;

/// Most bot moves the pump DESCRIBES on screen; the rest collapse to a pointer
/// at `log`, which holds them all described against the state of the time.
const BOT_LINE_CAP: usize = 10;

impl Repl {
    pub fn new(session: crate::session::Session) -> Repl {
        // Restore a persisted lockdown (review §3): a session saved after a
        // desync/invariant fault carries `record.lockdown`, so a reload comes
        // back locked instead of silently playable again.
        let mut session = session;
        let locked = session.is_locked();
        let privacy = matches!(session.mode, TrackingMode::Live);
        // WHO plays each seat comes from the record, on every path. A resumed
        // save and a wizard-built session are then indistinguishable here,
        // which is what makes `resume_restores_bot_seats_from_provenance` a
        // property of the construction rather than of the load code. Warnings
        // ride `load_warnings`, the channel `main.rs` folds into the opening
        // frame -- a seat this build cannot run must not go unmentioned.
        let (mut bots, mut warnings) = crate::bot::BotSeats::from_provenance(session.record().run.as_ref());
        // Bots are SIM ONLY (spec §9), enforced here rather than trusted: this
        // driver's live tail never offers the question, so a live record naming
        // a bot seat has been hand-edited -- and running one would put an agent
        // in front of `observe`, which refuses a live `Unknown` seat, or worse
        // wait on reveal choreography no bot can answer. Drop them, loudly.
        if matches!(session.mode, TrackingMode::Live) {
            let (without, dropped) = bots.without_bots();
            if !dropped.is_empty() {
                warnings.push(format!(
                    "warning: this LIVE save names bot seat(s) {dropped:?}, which live tracking does not \
                     support -- they resume as HUMAN seats, so you are playing them",
                ));
            }
            bots = without;
        }
        session.load_warnings.extend(warnings);
        Repl {
            session,
            page: 0,
            pending: Pending::None,
            locked,
            sub: SubPrompt::None,
            board_width_note_shown: false,
            composed: true,
            show_panel: true,
            privacy,
            bots,
        }
    }

    /// Per-seat panel presentation: a bot's hand is masked whatever `privacy`
    /// says, a human's obeys the toggle, and bot seats badge.
    pub(crate) fn seat_display(&self) -> render::SeatDisplay {
        let seats = self.session.state().players.len();
        let hide = (0..seats)
            .map(|i| {
                let seat = PlayerId::new(i as u8);
                self.privacy || self.bots.is_bot(seat)
            })
            .collect();
        render::SeatDisplay::new(hide, self.bots.flags())
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
        // The pending-choice banner gets its OWN line rather than the
        // header's phase slot (I6, third review): inlined, it pushed the
        // header to 150 columns past the documented 140, and the terminal
        // wrapped it mid-token -- the goods block's `i0` rendering as `i`
        // with a stray `0 | hand 1` underneath, which reads as corrupted
        // state rather than as a wrapped line. Split, both lines sit inside
        // the bound at every player count, and the banner is if anything
        // more prominent on a line of its own.
        let mut out = render::prompt_header(state, describe::phase_name(state.turn_phase));
        out.push('\n');
        if let Some(head) = state.pending_choices.first() {
            let n = state.pending_choices.len();
            out.push_str(&paint(
                Tone::Prompt,
                &format!(
                    "Choose: {} — {n} choice{} pending",
                    describe::describe_pending_choice(head, state, self.session.rules()),
                    if n == 1 { "" } else { "s" },
                ),
            ));
            out.push('\n');
        }
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
                // Ceremony reduction (Task 6): a forced single-option prompt
                // advertises the bare-Enter shortcut `step`'s `[]` arm honors.
                // What that keystroke DOES depends on the entry, though: a
                // `Direct` commits a move, while a grouped family opens a
                // sub-prompt (`pick_option`'s own split). Saying "apply" for
                // the grouped case over-promised (F4, Codex review of PR #4)
                // -- most visibly in an emergency-retreat position, whose
                // only displayed entry is the Retreat family.
                if n == 1 {
                    let verb = match options.first() {
                        Some(PromptOption::Direct(_)) => "apply",
                        _ => "open",
                    };
                    out.push_str(&format!("(only option — press Enter to {verb})\n"));
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
        const TAIL: &str = "or a command ('view' for game info, 'help' for the list)";
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
                // Deliberately terser than the other two clauses: at 142
                // columns this was the longest line the driver ever printed,
                // past the 140 the guide promises (I6, third review).
                format!("> enter a number, 'r' to retreat (the numbered retreat entry is guided), {TAIL}\n")
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
            // Ceremony reduction (Task 6, owner-chosen over silent
            // auto-advance): bare Enter on a prompt whose displayed list is
            // exactly ONE entry applies that entry through the normal commit
            // path -- recorded honestly like any numbered pick. Multi-option
            // prompts (and game-over/no-option screens) keep the old
            // re-prompt behavior.
            [] => {
                if !self.session.is_over() && self.display_options().len() == 1 {
                    self.pick_option(0)
                } else {
                    self.out(String::new(), false)
                }
            }
            ["quit"] | ["q"] => StepOutput { text: "bye\n".to_string(), quit: true },
            ["help"] => self.out_view(HELP_TEXT.to_string()),
            ["board"] => self.board_cmd(),
            ["players"] => self.out_view(render::players_panel(
                self.session.state(),
                self.session.rules(),
                &self.seat_display(),
            )),
            ["ports"] => self.out_view(render::ports_view(self.session.state(), self.session.rules())),
            ["labs"] => self.out_view(render::labs_view(self.session.state(), self.session.rules())),
            ["tracks"] => self.out_view(render::tracks_view(self.session.state())),
            ["sites"] => self.out_view(render::sites_view(self.session.state(), self.session.rules())),
            ["goals"] => self.out_view(render::achievements_view(self.session.state(), self.session.rules())),
            ["hand"] => self.out_view(self.hand_text()),
            ["score"] => self.out_view(self.score_text()),
            ["log"] => self.out_view(self.log_tail()),
            ["suggest"] => self.suggest_cmd(),
            ["view"] | ["views"] | ["?"] => {
                self.sub = SubPrompt::ViewMenu;
                self.out(String::new(), false)
            }
            ["more"] => {
                self.advance_page();
                self.out(String::new(), false)
            }
            ["list"] => {
                self.page = 0;
                self.out(String::new(), false)
            }
            ["undo"] => self.undo_cmd(),
            ["privacy"] => {
                self.privacy = !self.privacy;
                let word = if self.privacy { "HIDDEN" } else { "shown" };
                self.out(format!("Private info (hand contents) now {word} in views.\n"), false)
            }
            ["panel"] => {
                self.show_panel = !self.show_panel;
                let word = if self.show_panel { "shown" } else { "hidden" };
                self.out(format!("Players panel {word} (composed screen only).\n"), false)
            }
            ["screen"] => {
                self.composed = !self.composed;
                let word = if self.composed { "on" } else { "off" };
                self.out(format!("Composed screen {word} (takes effect on a real terminal).\n"), false)
            }
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
    ///
    /// Inside a composed frame this prints NO board at all (I5, third
    /// review): `screen::wrap` puts the grid and the legend above every body,
    /// so emitting them again cost 62 rows at 4 players in a 140x50 terminal
    /// and scrolled the top of the real board away -- the F3 overflow, back
    /// via the first row of the `view` menu. The width note still fires there
    /// (it is about the terminal, not about this command's output).
    fn board_cmd(&mut self) -> StepOutput {
        let grid = render::board_grid(self.session.state(), self.session.rules());
        let width = grid.lines().next().map(|l| l.chars().count()).unwrap_or(0);

        let mut text = String::new();
        if width > 100 && !self.board_width_note_shown {
            self.board_width_note_shown = true;
            text.push_str(&format!(
                "(board is {width} columns wide — widen the terminal if rows shear; guide §2)\n"
            ));
        }
        if screen::frame_active(self) {
            text.push_str("(the board and its legend are already above — the frame redraws them every turn.)\n");
            text.push_str("(type 'screen' for plain scrolling output instead.)\n");
        } else {
            text.push_str(&grid);
            text.push_str(&render::board_legend(self.session.state(), self.session.rules()));
            text.push('\n');
        }
        self.out_view(text)
    }

    pub(crate) fn lockdown_status(&self) -> String {
        // Surface the persisted reason (review §3) when one is recorded, so a
        // reloaded locked session tells the operator WHY it locked, not just
        // that it did. A directly-forced test lock has no recorded reason.
        //
        // The reason gets its OWN line (I6, third review): inlined between
        // the marker and the turn/seed counters it ran the status line to 157
        // columns -- past the documented 140 -- and a desync lockdown is
        // precisely the moment the operator must be able to READ the screen.
        let reason = self
            .session
            .record()
            .lockdown
            .as_deref()
            .map(|r| format!("  reason: {r}\n"))
            .unwrap_or_default();
        format!(
            "{} turn {} | seed {} | {} logged event(s) -- only 'save', 'log', 'quit' available\n{reason}",
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
            Some(PromptOption::Route { destination, moves }) => {
                let title = format!("Routes to {}", describe::area_label(self.session.state(), destination));
                self.enter_pick_subprompt(title, moves)
            }
            Some(PromptOption::Retreat { .. }) => self.enter_retreat_subprompt(),
            Some(PromptOption::PlacementA { max_areas }) => {
                self.sub = SubPrompt::PlaceA { selected: Vec::new(), max_areas };
                self.out(
                    "Option A — toggle areas by id off the board, then 'confirm'.".to_string(),
                    false,
                )
            }
            Some(PromptOption::PlacementB { areas }) => {
                self.sub = SubPrompt::PlaceB { areas, selected: None };
                self.out(
                    "Option B — type the id of the area to place 2 assistants in, then 'confirm'."
                        .to_string(),
                    false,
                )
            }
            Some(PromptOption::MoveFrom { from, moves }) => {
                let from_label = match from {
                    MoveSource::Hand => "hand".to_string(),
                    MoveSource::Area(a) => describe::area_label(self.session.state(), a),
                };
                self.enter_pick_subprompt(format!("Assistant destinations from {from_label}"), moves)
            }
            None => self.out(format!("option {k} out of range (0..{len})\n"), false),
        }
    }

    /// Build the numbered entries for the current prompt. Outside the Movement
    /// phase -- and inside it while a choice is queued -- every legal move is a
    /// bare `Direct` (byte-identical to the old ungrouped rendering). In the
    /// Movement main list it collapses the family
    /// (docs/plans/2026-07-18-phase2-cli-driver-design.md): one entry per
    /// walk/teleport destination (a `Route` when that destination has >1 fee
    /// label, else the single move `Direct`), each `PlacePresident`/other move
    /// left as `Direct` in place, and the whole `ReturnPresidentToHand` family
    /// folded into one `Retreat` line (unless a lone bare retreat is the only
    /// variant, which stays `Direct`). Single pass over `legal()` into a
    /// `BTreeMap` -- no per-keystroke rescans beyond the one `legal()` call.
    fn display_options(&self) -> Vec<PromptOption> {
        let legal = self.session.legal();
        let state = self.session.state();
        // A pending AssistantMove choice groups its options by SOURCE (Task
        // 6 picker) -- checked ahead of phase, same priority order the
        // banner/hint use.
        if matches!(state.pending_choices.first(), Some(PendingChoice::AssistantMove)) {
            return Self::group_assistant_moves(legal);
        }
        if state.pending_choices.is_empty() {
            match state.turn_phase {
                TurnPhase::Movement => return Self::group_movement(legal),
                TurnPhase::Placement => return Self::group_placement(legal),
                _ => {}
            }
        }
        legal.into_iter().map(PromptOption::Direct).collect()
    }

    /// Movement main-list grouping (destination-first,
    /// docs/plans/2026-07-18-phase2-cli-driver-design.md): one
    /// entry per walk/teleport destination (a `Route` when >1 fee label),
    /// the retreat family folded to one `Retreat` line.
    fn group_movement(legal: Vec<Move>) -> Vec<PromptOption> {
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

    /// Placement main-list grouping (Task 6 picker): the combinatorial
    /// `PlaceThreeAreas` wall (186 options on a fresh 2P board) collapses to
    /// ONE Option-A entry (area-toggle picker), the `PlaceTwoSameArea`
    /// family to ONE Option-B entry (area pick), the empty placement stays a
    /// direct line, and anything else passes through untouched. The
    /// `a`/`b` grammars remain first-class alongside (never removed).
    fn group_placement(legal: Vec<Move>) -> Vec<PromptOption> {
        let mut max_areas = 0usize;
        let mut option_a_present = false;
        let mut empty_placement: Option<Move> = None;
        let mut b_areas: Vec<AreaId> = Vec::new();
        let mut others: Vec<Move> = Vec::new();
        for mv in legal {
            match &mv {
                Move::PlaceThreeAreas { placements } if placements.is_empty() => {
                    empty_placement = Some(mv);
                }
                Move::PlaceThreeAreas { placements } => {
                    option_a_present = true;
                    max_areas = max_areas.max(placements.len());
                }
                Move::PlaceTwoSameArea { area, .. } => {
                    if !b_areas.contains(area) {
                        b_areas.push(*area);
                    }
                }
                _ => others.push(mv),
            }
        }
        b_areas.sort_unstable_by_key(|a| a.0);

        let mut out = Vec::new();
        if option_a_present {
            out.push(PromptOption::PlacementA { max_areas });
        }
        if !b_areas.is_empty() {
            out.push(PromptOption::PlacementB { areas: b_areas });
        }
        if let Some(mv) = empty_placement {
            out.push(PromptOption::Direct(mv));
        }
        out.extend(others.into_iter().map(PromptOption::Direct));
        out
    }

    /// Assistant-move grouping (Task 6 picker): one entry per SOURCE (hand,
    /// each occupied area) opening a destination pick; a single-destination
    /// source stays a direct line (the Route singleton rule); Decline and
    /// anything else pass through untouched.
    fn group_assistant_moves(legal: Vec<Move>) -> Vec<PromptOption> {
        let mut by_source: Vec<(MoveSource, Vec<Move>)> = Vec::new();
        let mut others: Vec<Move> = Vec::new();
        for mv in legal {
            match &mv {
                Move::ChooseAssistantMove { from, .. } => {
                    match by_source.iter_mut().find(|(s, _)| s == from) {
                        Some((_, moves)) => moves.push(mv),
                        None => by_source.push((*from, vec![mv])),
                    }
                }
                _ => others.push(mv),
            }
        }
        let mut out = Vec::new();
        for (from, mut moves) in by_source {
            if moves.len() == 1 {
                out.push(PromptOption::Direct(moves.pop().unwrap()));
            } else {
                out.push(PromptOption::MoveFrom { from, moves });
            }
        }
        out.extend(others.into_iter().map(PromptOption::Direct));
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
            PromptOption::PlacementA { max_areas } => format!(
                "Option A — place 1 assistant each in up to {max_areas} areas (pick areas; 'a' grammar works too)"
            ),
            PromptOption::PlacementB { areas } => format!(
                "Option B — place 2 assistants in one area ({} eligible; 'b' grammar works too)",
                areas.len()
            ),
            PromptOption::MoveFrom { from, moves } => {
                let from_label = match from {
                    MoveSource::Hand => "hand".to_string(),
                    MoveSource::Area(a) => describe::area_label(state, *a),
                };
                format!("Move an assistant from {from_label} — {} destinations", moves.len())
            }
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

    /// Route one line of input into the active sub-prompt. `matches!` takes
    /// only a momentary borrow, so the `&mut self` dispatch is clean.
    fn step_sub_prompt(&mut self, words: &[&str]) -> StepOutput {
        match self.sub {
            SubPrompt::Pick { .. } => self.step_pick(words),
            SubPrompt::Retreat { .. } => self.step_retreat(words),
            SubPrompt::PlaceA { .. } => self.step_place_a(words),
            SubPrompt::PlaceB { .. } => self.step_place_b(words),
            SubPrompt::ViewMenu => self.step_view_menu(words),
            SubPrompt::None => self.out(String::new(), false), // unreachable: gated by caller
        }
    }

    /// One `(label, render)` pair per game-info surface, in menu order: the
    /// label a row shows and the function that row dispatches to travel in
    /// the same tuple, so relabeling, reordering, or adding a surface can't
    /// desynchronize text from behavior (the array index is the menu
    /// number). Each render fn is the SAME code path the surface's direct
    /// command uses. NOTE this couples label to function only -- whether a
    /// label's WORDING matches what its view renders remains a human
    /// judgment (reviewed, not machine-checked).
    const VIEW_MENU: [ViewEntry; 10] = [
        ("Board — tiles, presidents, assistants, buildings", |r| r.board_cmd()),
        ("Players — every player's public holdings in full", |r| {
            let t = render::players_panel(r.session.state(), r.session.rules(), &r.seat_display());
            r.out_view(t)
        }),
        ("Port offer — the order cards on display", |r| {
            let t = render::ports_view(r.session.state(), r.session.rules());
            r.out_view(t)
        }),
        ("Lab offer — the techs on display, with slot surcharges", |r| {
            let t = render::labs_view(r.session.state(), r.session.rules());
            r.out_view(t)
        }),
        ("Tracks — Church/Customs spaces (incl. the 2P dummy), Station, mercenary supply", |r| {
            let t = render::tracks_view(r.session.state());
            r.out_view(t)
        }),
        ("Building sites — printed rewards, what's built and whose, adjacency", |r| {
            let t = render::sites_view(r.session.state(), r.session.rules());
            r.out_view(t)
        }),
        ("Goals — the achievement cards, claim VP, and per-player progress", |r| {
            let t = render::achievements_view(r.session.state(), r.session.rules());
            r.out_view(t)
        }),
        ("Hand — the current player's order cards", |r| {
            let t = r.hand_text();
            r.out_view(t)
        }),
        ("Score — current VP and money", |r| {
            let t = r.score_text();
            r.out_view(t)
        }),
        ("Log — the recent move/event trail", |r| {
            let t = r.log_tail();
            r.out_view(t)
        }),
    ];

    /// Game-info menu input (one-shot): a valid number closes the menu and
    /// dispatches that row's render fn from [`Self::VIEW_MENU`]; `cancel`
    /// closes it; anything else re-asks. Read-only -- no state, record, or
    /// session field is ever touched from here (`board_cmd`'s once-per-
    /// session width-note flag is the sole, display-only exception).
    fn step_view_menu(&mut self, words: &[&str]) -> StepOutput {
        match words {
            ["cancel"] => {
                self.sub = SubPrompt::None;
                self.out(String::new(), false)
            }
            ["quit"] | ["q"] => StepOutput { text: "bye\n".to_string(), quit: true },
            [tok] if !tok.is_empty() && tok.chars().all(|c| c.is_ascii_digit()) => {
                let Some(&(_, render_fn)) =
                    tok.parse::<usize>().ok().and_then(|k| Self::VIEW_MENU.get(k))
                else {
                    return self.out(
                        format!("! pick a number 0-{}, or 'cancel'", Self::VIEW_MENU.len() - 1),
                        false,
                    );
                };
                self.sub = SubPrompt::None;
                render_fn(self)
            }
            _ => self.out("! pick a numbered view, or 'cancel'".to_string(), false),
        }
    }

    /// Numbered-pick sub-prompt input (route labels, assistant destinations,
    /// Telegram variants): a valid number commits that exact move (taken
    /// straight from `legal()` -- never re-derived), `cancel` returns to the
    /// main list, `quit`/`q` exits (the un-applied sub-prompt is simply
    /// discarded -- nothing was recorded, same escape the reveal collector
    /// honors). A bare out-of-range number re-asks with its range; any other
    /// non-answer re-asks with `! finish or cancel this prompt first` (a
    /// view/`save`/`undo` mid-collection would render or mutate against a
    /// state the collection doesn't reflect).
    fn step_pick(&mut self, words: &[&str]) -> StepOutput {
        match words {
            ["cancel"] => {
                self.sub = SubPrompt::None;
                self.page = 0;
                self.out("Selection cancelled — back to the main options.".to_string(), false)
            }
            ["quit"] | ["q"] => StepOutput { text: "bye\n".to_string(), quit: true },
            [tok] if !tok.is_empty() && tok.chars().all(|c| c.is_ascii_digit()) => {
                // Resolve while borrowing `self.sub`, but yield an OWNED result so
                // the borrow is released before the mutation/commit below.
                let selected: Result<Move, usize> = match &self.sub {
                    SubPrompt::Pick { moves, .. } => {
                        let n = moves.len();
                        tok.parse::<usize>().ok().and_then(|k| moves.get(k).cloned()).ok_or(n)
                    }
                    _ => return self.out(String::new(), false), // unreachable: dispatched as Pick
                };
                match selected {
                    Ok(mv) => {
                        self.sub = SubPrompt::None;
                        self.page = 0;
                        self.commit(mv)
                    }
                    Err(n) => self.out(format!("! pick a number 0-{}, or 'cancel'", n.saturating_sub(1)), false),
                }
            }
            // Pick lists page like the main list (brief item 5): `more`/`list`
            // drive its pager rather than being refused, so the mirrored
            // "type 'more'" footer is honest. (Retreat questions are
            // sequential, not a list, so `step_retreat` still refuses them.)
            ["more"] => {
                let n = match &self.sub {
                    SubPrompt::Pick { moves, .. } => moves.len(),
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

    /// Option-A picker input: an area id TOGGLES membership in the selection
    /// (no cap here -- the legal-list match at `confirm` is the sole
    /// authority, so an over-long or unaffordable selection gets the
    /// standard rejection); `confirm` multiset-matches the assembled
    /// `PlaceThreeAreas` (the empty selection legitimately matches "place no
    /// assistants" when the engine offers it); `cancel` returns to the main
    /// list.
    fn step_place_a(&mut self, words: &[&str]) -> StepOutput {
        match words {
            ["cancel"] => {
                self.sub = SubPrompt::None;
                self.page = 0;
                self.out("Placement cancelled — back to the main options.".to_string(), false)
            }
            ["quit"] | ["q"] => StepOutput { text: "bye\n".to_string(), quit: true },
            ["confirm"] => {
                let (selected, max_areas) = match &self.sub {
                    SubPrompt::PlaceA { selected, max_areas } => (selected.clone(), *max_areas),
                    _ => return self.out(String::new(), false),
                };
                let candidate = Move::PlaceThreeAreas { placements: selected.clone() };
                match self.find_placement_match(&candidate) {
                    Some(mv) => {
                        self.sub = SubPrompt::None;
                        self.page = 0;
                        self.commit(mv)
                    }
                    // Keep the picker OPEN with the selection intact (item 1,
                    // third review): dropping to the main list here discarded
                    // everything the operator had toggled. Name the over-limit
                    // case distinctly from a generic legal-list miss (item 2).
                    None => {
                        let msg = if selected.len() > max_areas {
                            format!(
                                "! too many areas — Option A places in at most {max_areas}; deselect {} then 'confirm', or 'cancel'.\n",
                                selected.len() - max_areas,
                            )
                        } else {
                            "! that selection is not currently available — adjust it or 'cancel'.\n".to_string()
                        };
                        self.out(msg, false)
                    }
                }
            }
            // The gate admits the DISPLAYED id form too (`A03`/`a03`, SPOT-1)
            // -- the directory and the Selected echo both print it, so the
            // picker must accept what it shows.
            [tok]
                if !strip_area_prefix(tok).is_empty()
                    && strip_area_prefix(tok).chars().all(|c| c.is_ascii_digit()) =>
            {
                let area = match parse_raw_area_token(tok) {
                    Ok(a) => a,
                    Err(e) => return self.out(format!("! {e}"), false),
                };
                // Validate the id against the real placeable-area set at TOGGLE
                // time (item 1, third review): a numeric-but-nonexistent id
                // (`A99`, or any id past this board's last area) used to enter
                // the selection unchecked and only fail at `confirm`, which then
                // discarded the whole selection. Reject it here with its own
                // message -- the same set the directory above lists -- leaving
                // the selection untouched. (Over the per-turn area limit is NOT
                // rejected here: max_areas is display guidance and the
                // legal-list match at `confirm` stays the sole authority; the
                // live preview and `confirm` name that case distinctly, item 2.)
                let placeable = self
                    .session
                    .state()
                    .board_layout
                    .areas
                    .get(&area)
                    .is_some_and(|c| c.area_type.allows_placement());
                if !placeable {
                    return self.out(
                        format!("! A{:02} is not a placement area on this board.\n", area.0),
                        false,
                    );
                }
                if let SubPrompt::PlaceA { selected, .. } = &mut self.sub {
                    match selected.iter().position(|&a| a == area) {
                        Some(i) => {
                            selected.remove(i);
                        }
                        None => selected.push(area),
                    }
                }
                self.out(String::new(), false)
            }
            _ => self.out(
                "! toggle an area id, 'confirm', or 'cancel' (cancel first to use views/help)".to_string(),
                false,
            ),
        }
    }

    /// Option-B picker input: an area id (bare `15` or the displayed `A15`)
    /// SELECTS that area -- addressed by id, never by list position (C1) --
    /// and the screen previews the move it assembles; `confirm` then commits
    /// its single legal variant, or opens a numbered pick over a
    /// multi-variant area's (Telegram owner's) exact variants. The
    /// select-then-confirm shape is Option A's, and it is what makes a
    /// mis-typed id visible before anything reaches the record.
    fn step_place_b(&mut self, words: &[&str]) -> StepOutput {
        match words {
            ["cancel"] => {
                self.sub = SubPrompt::None;
                self.page = 0;
                self.out("Placement cancelled — back to the main options.".to_string(), false)
            }
            ["quit"] | ["q"] => StepOutput { text: "bye\n".to_string(), quit: true },
            ["confirm"] => {
                let area = match &self.sub {
                    SubPrompt::PlaceB { selected: Some(area), .. } => *area,
                    SubPrompt::PlaceB { selected: None, .. } => {
                        return self.out("! select an area id first, or 'cancel'".to_string(), false)
                    }
                    _ => return self.out(String::new(), false),
                };
                let mut variants = self.place_b_variants(area);
                match variants.len() {
                    0 => {
                        // Unreachable from a live entry (areas came from this
                        // same legal list); defensive fall-back.
                        self.sub = SubPrompt::None;
                        self.page = 0;
                        self.out("Illegal: that placement is not currently available.\n".to_string(), false)
                    }
                    1 => {
                        self.sub = SubPrompt::None;
                        self.page = 0;
                        self.commit(variants.pop().unwrap())
                    }
                    _ => {
                        let title = format!(
                            "Placement in {} — choose the Telegram bonus",
                            describe::area_label(self.session.state(), area)
                        );
                        self.enter_pick_subprompt(title, variants)
                    }
                }
            }
            // The DISPLAYED id form (`A15`/`a15`) and the bare number mean the
            // same area, through the same `strip_area_prefix` path every other
            // entry surface uses (SPOT-1's rule, applied to the fifth path).
            [tok]
                if !strip_area_prefix(tok).is_empty()
                    && strip_area_prefix(tok).chars().all(|c| c.is_ascii_digit()) =>
            {
                let area = match parse_raw_area_token(tok) {
                    Ok(a) => a,
                    Err(e) => return self.out(format!("! {e}"), false),
                };
                let eligible = match &self.sub {
                    SubPrompt::PlaceB { areas, .. } => areas.contains(&area),
                    _ => return self.out(String::new(), false),
                };
                if !eligible {
                    return self.out(
                        format!("! A{:02} is not one of the eligible areas — pick one listed above", area.0),
                        false,
                    );
                }
                if let SubPrompt::PlaceB { selected, .. } = &mut self.sub {
                    *selected = Some(area);
                }
                self.out(String::new(), false)
            }
            _ => self.out(
                "! type an area id, 'confirm', or 'cancel' (cancel first to use views/help)".to_string(),
                false,
            ),
        }
    }

    /// Every legal `PlaceTwoSameArea` in `area` (one, unless a Telegram owner
    /// also has adjacent-bonus variants). Shared by the Option-B picker's
    /// preview line and its `confirm`, so what the screen promises and what
    /// commits are read off the same legal list.
    fn place_b_variants(&self, area: AreaId) -> Vec<Move> {
        self.session
            .legal()
            .into_iter()
            .filter(|m| matches!(m, Move::PlaceTwoSameArea { area: a, .. } if *a == area))
            .collect()
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

    /// Open a numbered-pick sub-prompt over `moves`. `out()` appends
    /// `sub_prompt_text()` (the numbered list) since `self.sub` is now set.
    fn enter_pick_subprompt(&mut self, title: String, moves: Vec<Move>) -> StepOutput {
        self.sub = SubPrompt::Pick { title, moves };
        self.page = 0; // the pick list gets its own paging, starting at page 0
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
            SubPrompt::Pick { title, moves } => {
                // Same 20/page + footer machinery the main list uses (brief item
                // 5). Bounded ~16 in real play, so this rarely pages -- but when
                // it does, `more`/`list` step it (see `step_pick`), so the
                // mirrored "type 'more'" footer stays honest.
                let mut out = format!("{title}:\n");
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
                out.push_str("> pick a number, or 'cancel'\n");
                out
            }
            SubPrompt::PlaceA { selected, max_areas } => {
                let mut out = format!(
                    "Option A — place 1 assistant each in up to {max_areas} areas (toggle by id):\n"
                );
                out.push_str(&placement_area_directory(state));
                let selection_text = if selected.is_empty() {
                    "(none — confirm places no assistants)".to_string()
                } else {
                    selected.iter().map(|a| format!("A{:02}", a.0)).collect::<Vec<_>>().join(", ")
                };
                // Live legality + fee preview: the CANDIDATE the current
                // toggles assemble, checked against the same legal list
                // `confirm` will match (display only -- the match at commit
                // stays the sole authority). An over-limit selection is named
                // distinctly from a generic miss (item 2, third review).
                let candidate = Move::PlaceThreeAreas { placements: selected.clone() };
                let verdict = if selected.len() > *max_areas {
                    format!("too many areas (max {max_areas}) — deselect {}", selected.len() - max_areas)
                } else {
                    match self.find_placement_match(&candidate) {
                        Some(mv) => format!("legal — {}", describe::describe_move(&mv, state, rules)),
                        None => "NOT currently offered (adjust the selection)".to_string(),
                    }
                };
                out.push_str(&format!("Selected: {selection_text}\n  -> {verdict}\n"));
                out.push_str("> type an area id to toggle, 'confirm' to place, or 'cancel'\n");
                out
            }
            SubPrompt::PlaceB { areas, selected } => {
                // Listed BY ID, with no `[i]` numbering at all (C1): the only
                // thing on screen that can be typed here is an area id, so
                // there is no second numbering to confuse it with.
                let mut out = String::from("Option B — place 2 assistants in one area (by id):\n");
                for area in areas {
                    let fee = describe::placement_fee_note(state, &[*area, *area]);
                    out.push_str(&format!("  {}{fee}\n", describe::area_label(state, *area)));
                }
                // Live legality preview on the SELECTED id, read off the same
                // legal list `confirm` commits from -- Option A's `-> legal —`
                // precedent, and the reason a mis-typed id is now visible
                // before it reaches the record.
                let (selection_text, verdict) = match selected {
                    None => ("(none)".to_string(), "type an area id to preview the placement".to_string()),
                    Some(area) => {
                        let variants = self.place_b_variants(*area);
                        let verdict = match variants.len() {
                            0 => "NOT currently offered (pick another area)".to_string(),
                            1 => format!("legal — {}", describe::describe_move(&variants[0], state, rules)),
                            n => format!("legal — {n} Telegram bonus variants (confirm to choose one)"),
                        };
                        (format!("A{:02}", area.0), verdict)
                    }
                };
                out.push_str(&format!("Selected: {selection_text}\n  -> {verdict}\n"));
                out.push_str("> type an area id, 'confirm' to place, or 'cancel'\n");
                out
            }
            SubPrompt::ViewMenu => {
                let mut out = String::from("─ Game info ───────────────────────────\n");
                for (i, (label, _)) in Self::VIEW_MENU.iter().enumerate() {
                    out.push_str(&format!("[{i}] {label}\n"));
                }
                out.push_str("> pick a number to open a view, or 'cancel'\n");
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
    /// like every arm in that module) reveals whether `mv` draws from either
    /// deck; when it does, `mv` is parked in `self.pending` and the operator is
    /// prompted for the refilled card(s) instead of applying immediately. A
    /// 5-Power token take needs no such prompt (its face is observed FACE-UP at
    /// setup, §1.1). Everything else -- including every move in
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
                    // A 5-Power token take needs no reveal: the token faces are
                    // FACE-UP open information observed at setup (§1.1), so the
                    // layout already holds the true face and the take just reads
                    // it -- it falls through to `finish_apply` like any move.
                }
            }
        }
        self.finish_apply(mv, vec![])
    }

    /// `undo`: pop the last entry, and -- with bots at the table -- keep going
    /// back through the bots' whole trailing run, so one `undo` returns the
    /// human to their own last decision instead of dropping them into the
    /// middle of an opponent's turn (spec §9's MUST item 2).
    ///
    /// **The boundary is REPLAY-DERIVED, deliberately, not remembered.**
    /// `RecordedEvent::Move` carries no seat, so who made an entry is knowable
    /// only by rebuilding the state before it and reading `current_player` --
    /// which `Session::undo` already does. Popping one at a time and looking is
    /// therefore not a workaround: it is the only honest attribution available,
    /// and it costs nothing in correctness that a remembered stack would gain,
    /// while surviving save -> quit -> reload by construction (a stack dies
    /// with the process; this is pinned by `undo_after_resume_still_unwinds`).
    fn undo_cmd(&mut self) -> StepOutput {
        let mut warnings = Vec::new();
        let mut entries = 0usize;
        // Set when the unwind ran out of log while still inside a bot run --
        // i.e. nothing of the player's OWN was in it (undoing the opening pump
        // is the reachable case). Worth distinguishing: the re-pump below puts
        // the identical position back, so a report claiming to have undone
        // "your last move" would describe a change the player cannot see.
        let mut hit_the_bottom = false;
        loop {
            match self.session.undo() {
                Ok(w) => {
                    entries += 1;
                    warnings.extend(w);
                }
                // Only the FIRST failure is an error: once something has been
                // undone, running out of log mid-unwind is a boundary, not a
                // fault. The boundary is matched BY KIND, not by position (T12
                // review F2): `Session::undo` also returns Err when a record
                // prefix fails to REPLAY, and reporting that as "nothing of
                // your own to undo" would describe N committed-away entries as
                // a no-op — the silent-failure shape the benign branch exists
                // to avoid.
                Err(e) if entries == 0 => return self.out(format!("Cannot undo: {e}\n"), false),
                Err(e) if e.contains("nothing to undo") => {
                    hit_the_bottom = true;
                    break;
                }
                Err(e) => {
                    return self.out(
                        format!(
                            "Cannot undo: a record prefix failed to replay after {entries} \
                             entr{} were unwound: {e}\n",
                            if entries == 1 { "y" } else { "ies" },
                        ),
                        false,
                    )
                }
            }
            // The state now standing is the one the popped entry was made
            // FROM, so its `current_player` names who made it. While that is a
            // bot, there is more of the bot's run to unwind.
            if !self.bots.is_bot(self.session.state().current_player) {
                break;
            }
        }
        self.page = 0;
        let mut text = if hit_the_bottom {
            format!(
                "Nothing of your own to undo yet — this game opens with the bot's turn, and it replays \
                 unchanged ({entries} entries).\n"
            )
        } else if entries == 1 {
            String::from("Undid last entry.\n")
        } else {
            format!("Undid your last move and the bot's reply ({entries} entries).\n")
        };
        // Warnings ride the same visible channel `apply`'s delta lines and the
        // load warnings use -- the frame erases anything printed outside this
        // text (I3).
        for w in warnings {
            text.push_str(&w);
            text.push('\n');
        }
        // Unwinding past every human entry can leave a BOT to move (undoing the
        // opening pump is the reachable case). Re-pumping puts the game back at
        // a human decision point, and does it deterministically: the decision
        // seeds are a function of the event count, which is back where it was,
        // so the bot replays the same line rather than inventing a new one.
        let pump = self.pump_bots();
        if !pump.is_empty() {
            text.push_str(&pump);
        }
        self.out(text, false)
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
        match self.apply_and_describe(mv, reveals) {
            ApplyOutcome::Applied(delta) => {
                let mut text = delta.join("\n");
                // Bot seats play HERE, inside the human's own step, so one
                // input produces one screen showing the human's move, every
                // bot reply to it, and the prompt that follows -- the second
                // of the two pump call sites (spec §9; `main.rs` is the other).
                let pump = self.pump_bots();
                if !pump.is_empty() {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&pump);
                }
                // AFTER the pump: a bot's reply can be the move that ends the
                // game, and the final table has to describe the finished game
                // rather than the position the human handed over.
                if self.session.is_over() {
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                    text.push_str(&self.final_score_table());
                }
                (self.out(text, false), true)
            }
            ApplyOutcome::Failed(text) => (self.out(text, false), false),
        }
    }

    /// Apply one move+reveals group through `Session::apply` and turn the
    /// result into screen text, WITHOUT appending a prompt or running the bot
    /// pump. The shared inner of `finish_apply_ex` (human input) and
    /// `pump_bots` (bot moves), so both get identical apply semantics and
    /// identical lockdown handling -- the two paths cannot drift on what an
    /// invariant violation does.
    fn apply_and_describe(&mut self, mv: Move, reveals: Vec<RecordedEvent>) -> ApplyOutcome {
        match self.session.apply(mv, reveals) {
            Ok(delta) => {
                self.page = 0;
                ApplyOutcome::Applied(delta)
            }
            Err(ApplyError::Invariant(msg)) => {
                let warning = self.engage_lockdown(format!("invariant violation: {msg}"));
                let mut text = format!("{}\n", paint(Tone::Warn, &format!("LOCKDOWN -- {msg}")));
                text.push_str(&self.lockdown_dump());
                if let Some(w) = warning {
                    text.push_str(&w);
                    text.push('\n');
                }
                ApplyOutcome::Failed(text)
            }
            // T13 rider (final-review cheap batch): a dedicated arm instead
            // of falling into the catch-all's "Illegal: {e}" phrasing, which
            // rendered ApplyError::GameOver's Display text ("the game is
            // already over") as the confusing "Illegal: the game is already
            // over".
            Err(ApplyError::GameOver) => ApplyOutcome::Failed("game over — commands only\n".to_string()),
            Err(e) => ApplyOutcome::Failed(format!("Illegal: {e}\n")),
        }
    }

    /// Play every consecutive bot seat from the current one until a human is to
    /// move (spec §9's MUST item 1). Returns the text to show; never a prompt
    /// (`out` owns that) and never a state change at a human decision point.
    ///
    /// Called from BOTH sites that can leave a bot to move: `main.rs` before
    /// the opening prompt -- without which a bot on the start seat would never
    /// move at all, since nothing else runs until the human types -- and
    /// `finish_apply_ex`'s success arm.
    pub fn pump_bots(&mut self) -> String {
        self.pump_bots_with(&mut crate::bot::HonestChooser)
    }

    /// [`pump_bots`](Self::pump_bots) over an explicit chooser. The seam the
    /// failure-shape tests drive: the honest chooser is supposed to make an
    /// illegal proposal unreachable, and an unreachable path still needs its
    /// handling pinned.
    pub(crate) fn pump_bots_with(&mut self, chooser: &mut dyn crate::bot::BotChooser) -> String {
        // The common case is a human to move, which this makes free -- the
        // whole body below (a state clone included) would otherwise run on
        // every applied move of an all-human game.
        if !self.bots.is_bot(self.session.state().current_player) {
            return String::new();
        }
        // ONE aggregate delta for the whole run (spec §9): a bot turn is many
        // decision points, and a per-move delta block for each buries the
        // human's own move under a screen of the opponent's bookkeeping.
        let before = self.session.state().clone();
        let mut moves: Vec<String> = Vec::new();
        let mut halt: Option<String> = None;

        while !self.locked && !self.session.is_over() {
            let seat = self.session.state().current_player;
            let Some(policy) = self.bots.policy_at(seat) else { break };
            if moves.len() >= PUMP_MOVE_CAP {
                halt = Some(format!(
                    "BOT HALT — the pump has applied {PUMP_MOVE_CAP} moves across consecutive bot \
                     seats (currently seat {}) without reaching a human. \
                     Stopping rather than looping; the game is intact, and `log` shows what it did.",
                    seat.0,
                ));
                break;
            }
            let decision_seed = self.bots.decision_seed(self.session.record().events.len());
            let chosen = chooser.choose(self.session.state(), self.session.rules(), seat, policy, decision_seed);
            let mv = match chosen {
                Ok(mv) => mv,
                // A dead end is the policy honestly finding nothing: halt, do
                // not lock down -- the session is still sound.
                Err(crate::bot::BotError::DeadEnd(why)) => {
                    halt = Some(format!("BOT HALT — seat {} has no move to make ({why}).", seat.0));
                    break;
                }
                // A refused view or belief sample means the driver handed the
                // agent something it cannot honestly play from. The only other
                // option is privileged play, which is the thing this whole
                // chain exists to prevent, so it is a fault.
                Err(e) => {
                    let reason = format!("bot seat {} could not decide honestly: {e}", seat.0);
                    let warning = self.engage_lockdown(reason.clone());
                    let mut text = format!("{}\n", paint(Tone::Warn, &format!("LOCKDOWN -- {reason}")));
                    text.push_str(&self.lockdown_dump());
                    if let Some(w) = warning {
                        text.push_str(&w);
                        text.push('\n');
                    }
                    halt = Some(text);
                    break;
                }
            };
            // Described against the state the move actually ran against, before
            // it is applied -- the same rule `Session::apply` follows for the
            // log cache.
            let line = format!(
                "Seat {} ({}) — {}",
                seat.0,
                render::color_name(self.session.state().current_player().color),
                describe::describe_move(&mv, self.session.state(), self.session.rules()),
            );
            match self.apply_and_describe(mv, vec![]) {
                ApplyOutcome::Applied(_) => moves.push(line),
                ApplyOutcome::Failed(text) => {
                    // The engine refused a move the bot proposed. `apply_and_
                    // describe` already locked down if it was an invariant
                    // violation; anything else (an illegal move above all) is
                    // just as much a driver/engine fault HERE, because the bot
                    // draws from the same engine surface the legal list comes
                    // from -- so it gets the same treatment rather than the
                    // "you typed something illegal" message a human would see.
                    let mut out = text;
                    if !self.locked {
                        let reason = format!(
                            "bot seat {} proposed a move the engine refused (the honest chain must \
                             only ever produce legal moves)",
                            seat.0,
                        );
                        let warning = self.engage_lockdown(reason.clone());
                        out.push_str(&format!("{}\n", paint(Tone::Warn, &format!("LOCKDOWN -- {reason}"))));
                        out.push_str(&self.lockdown_dump());
                        if let Some(w) = warning {
                            out.push_str(&w);
                            out.push('\n');
                        }
                    }
                    halt = Some(out);
                    break;
                }
            }
        }

        let mut text = String::new();
        // At most BOT_LINE_CAP described moves, then a pointer at `log`, which
        // carries the full history described against the state of the time.
        for line in moves.iter().take(BOT_LINE_CAP) {
            text.push_str(line);
            text.push('\n');
        }
        if moves.len() > BOT_LINE_CAP {
            text.push_str(&format!("…{} more — type `log`\n", moves.len() - BOT_LINE_CAP));
        }
        if !moves.is_empty() {
            for line in render::delta_summary(&before, self.session.state(), self.session.rules()) {
                text.push_str(&line);
                text.push('\n');
            }
        }
        if let Some(h) = halt {
            text.push_str(&h);
            if !text.ends_with('\n') {
                text.push('\n');
            }
        }
        text
    }

    /// `suggest`: what Lookahead-v1 would play from YOUR seat's own view, with
    /// the evaluator's term-by-term arithmetic (spec §7).
    ///
    /// **Read-only by construction**, which is the property the whole feature
    /// rests on: this takes `&self`, so it cannot record an event, move the
    /// state, or trip an autosave even by accident -- the compiler enforces
    /// the sandbox rather than a convention. Everything it needs comes off
    /// `session` by shared reference, the agent is a local built per press from
    /// a key derived from PUBLIC quantities (`suggest::stream_seed`), and it is
    /// dropped before this returns. That pairing is exactly what makes a second
    /// press at an unchanged position return the same answer: same position ⟹
    /// same event index ⟹ same key ⟹ same agent ⟹ same search.
    ///
    /// Four refusals, each for its own reason rather than one catch-all:
    /// LIVE mode (the standing 3.2 scope -- see `suggest`'s module doc for why
    /// a live game's sampled worlds are not the position anyone is playing),
    /// a finished game, a seat a BOT is playing (the pump normally leaves a
    /// human to move, so this is reachable only after a halt -- and answering
    /// for a bot's seat would be advising it against you), and the agent's own
    /// errors, which are surfaced verbatim rather than swallowed.
    fn suggest_cmd(&self) -> StepOutput {
        if matches!(self.session.mode, TrackingMode::Live) {
            // Hand-wrapped, like the render's own notes: this is a view body, so
            // a 215-column paragraph would size the composed frame to itself
            // (T10 review M1 -- the same defect, in the refusal).
            return self.out_view(
                "`suggest` is simulation-only.\n\
                 A live-tracked game's hidden cards are a model reconciled from the table, not\n\
                 the engine's own deck, so the bot would be searching a position nobody is\n\
                 playing. (Bot seats are sim-only for the same family of reason -- see guide §4.)"
                    .to_string(),
            );
        }
        if self.session.is_over() {
            return self.out_view("The game is over -- there is no move to suggest.".to_string());
        }
        let seat = self.session.state().current_player;
        if self.bots.is_bot(seat) {
            return self.out_view(format!(
                "Seat {} is played by a bot, and `suggest` answers for YOUR seat only.",
                seat.0,
            ));
        }
        let seed = crate::suggest::stream_seed(self.session.record(), seat);
        match crate::suggest::suggest(self.session.state(), self.session.rules(), seat, seed) {
            Ok(s) => self.out_view(crate::suggest::render_suggestion(
                &s,
                self.session.state(),
                self.session.rules(),
            )),
            Err(e) => self.out_view(format!("No suggestion: {e}")),
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
            Ok(delta) => {
                // Surface the delta the same way `undo`/`apply` do (item 5,
                // third review): it carries the state diff (the card entering
                // the seat's hand) AND any autosave-failure warning, which this
                // used to discard -- leaving a failed live-tracking autosave
                // invisible on the composed frame (same bug as the `undo` I3
                // fix). `StepOutput.text` is the only channel `main.rs` renders.
                let mut text = format!("Seat {} revealed O{:02} as their starting order.\n", seat.0, card.0);
                for line in delta {
                    text.push_str(&line);
                    text.push('\n');
                }
                self.out(text, false)
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
        // Shares render::section, like every other view (item 4), rather than
        // hand-rolling its own dash run.
        let mut out = render::section(&format!("Hand: {:?}", p.color));
        // Per-seat, like the panel: `hand` shows the CURRENT player's cards,
        // and with bots at the table that is a human at every prompt -- but
        // a halted pump can leave a bot to move, and the bot's cards are not
        // the human's to read.
        let seats = self.seat_display();
        if seats.hides_hand(p.id) {
            out.push_str(&format!(
                "  {}\n",
                render::masked_hand(p.order_cards_in_hand.len(), seats.is_bot(p.id))
            ));
        } else if p.order_cards_in_hand.is_empty() {
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
            let mut out = render::section("Score");
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
        let mut out = render::section("Final scores");
        out.push_str(&format!("WINNER: {}\n", render::color_name(state.player(result.winner).color)));
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
        let mut out = render::section(&format!("Log ({} of {} event(s))", n - start, n));
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

/// Compact directory of placeable areas for the Option-A picker, three per
/// line, with a `(fee)` mark wherever an opponent President stands (the
/// exact per-selection amount shows live on the `Selected:` line via
/// `describe_move`'s fee preview). In the composed TTY frame the full board
/// sits right above this; the directory keeps the piped/rolling view
/// self-sufficient.
fn placement_area_directory(state: &GameState) -> String {
    let me = state.current_player;
    let mut cells: Vec<String> = Vec::new();
    for (&id, cfg) in &state.board_layout.areas {
        if !cfg.area_type.allows_placement() {
            continue;
        }
        let fee = state
            .players
            .iter()
            .any(|p| p.id != me && p.president_location == PresidentLocation::OnBoard(id));
        cells.push(format!(
            "A{:02} {}{}",
            id.0,
            describe::area_type_name(cfg.area_type),
            if fee { " (fee)" } else { "" }
        ));
    }
    let mut out = String::new();
    for chunk in cells.chunks(3) {
        out.push(' ');
        for c in chunk {
            out.push_str(&format!(" {:<27}", c));
        }
        while out.ends_with(' ') {
            out.pop();
        }
        out.push('\n');
    }
    out
}

/// Strip one optional leading `A`/`a` from an area token (SPOT-1): every
/// display surface writes ids as `A03`, so every entry path — the pickers
/// and the `a`/`b`/`m`/`r` grammars — accepts that displayed form too.
fn strip_area_prefix(tok: &str) -> &str {
    tok.strip_prefix(['A', 'a']).unwrap_or(tok)
}

/// Parse an area-id token (`3` or the displayed `A03`/`a03` form) with a
/// friendly, non-panicking error: rejects non-numeric tokens and numbers
/// outside AreaId's 0-17 range identically (a token like
/// `"999999999999999999999"` fails `u8::parse`, not `unwrap`).
fn parse_area_id(tok: &str) -> Result<AreaId, String> {
    match strip_area_prefix(tok).parse::<u8>().ok().and_then(|n| AreaId::try_new(n).ok()) {
        Some(id) => Ok(id),
        None => Err(format!("'{tok}' is not a valid area id (0-17)")),
    }
}

/// Parse an `m`-grammar area token (`8` or the displayed `A08` form) into a
/// raw, possibly off-board `AreaId` for legal-move membership testing.
/// Unlike `parse_area_id`, values >=18 are NOT rejected here as a range
/// error -- `AreaId::unchecked` can't panic on them, so a destination like
/// `99` reaches `try_assistant_move`'s `legal()` lookup and fails there
/// with "not one of the legal assistant moves" (its options never include
/// an area that isn't really on the board) instead of a separate, earlier
/// range error. Only genuinely unparseable tokens (non-numeric, or too
/// large for `u8`) are rejected here.
fn parse_raw_area_token(tok: &str) -> Result<AreaId, String> {
    strip_area_prefix(tok)
        .parse::<u8>()
        .map(AreaId::unchecked)
        .map_err(|_| format!("'{tok}' is not a valid area number"))
}

/// The in-app `help` command's text. `pub` so the `guide_sync` integration
/// test can pin every command token against the tester guide's §5 block --
/// this const is the authority; the guide follows it.
pub const HELP_TEXT: &str = "\
Commands:
  <number>        apply the numbered legal option
  <Enter>         on a single-option prompt: apply that only option, or open it if it is a grouped family
  a [id id id]    Placement Option A: place 1 assistant in each of 0-3 areas (4 with Newspaper)
  b <id>          Placement Option B: place 2 assistants in one area
  b <id> t <adj>  Placement Option B with the Telegram bonus: +1 assistant in an adjacent area
  m <from> <to>   resolve a pending assistant-move choice (<from> is 'hand' or an area id)
  r [id[:n]]...   Movement Option B: retreat the President, recovering all (bare id) or n assistants per area ('r' alone recovers none)
  board           show the area grid and its legend (5-Power token faces, [St] Station, · empty build slot, ~ Canal toll)
  players         show each player's resources, hand, tech, tokens, President
  ports           show both Port boards and the order-deck size
  labs            show both Laboratory boards (each tech's country + effect) and the tech-deck size
  tracks          show the Church and Customs tracks with printed VP and rider icons
  sites           show each area's building-site printed rewards, build marks, and adjacency
  goals           show achievement cards with claim VP, claim state, and per-player progress
  hand            show the current player's order-card hand
  score           show current VP/money (or the final scores once the game is over)
  log             show the tail of the move/event log
  view            open the numbered game-info menu over all of the views above ('views'/'?' also work)
  suggest         simulation only: what the built-in bot would play from your seat, with its arithmetic
  privacy         toggle hiding hand contents in views (default: hidden in live mode)
  panel           toggle the players panel inside the composed screen (terminal only)
  screen          toggle the composed screen itself (terminal only; pipes are always plain)
  more            show the next page of legal options (wraps to page 0)
  list            reset to the first page of legal options
  undo            undo the last move (and any reveals bound to it)
  save [path]     save to path (or to the last-used path if omitted)
  reveal <seat> <O..>  live mode: make a hidden opponent starting order public
  cancel          back out of any sub-prompt: reveal collection (live), view menu, numbered pick, either Placement picker, guided retreat
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

    /// The DISPLAY index (what a numbered answer addresses) of the first
    /// grouped entry whose rendered text contains `needle` -- fixtures pick
    /// by text now that grouping means raw legal-list positions no longer
    /// address the prompt (Task 6).
    fn display_index_containing(r: &Repl, needle: &str) -> Option<usize> {
        let state = r.session.state();
        let rules = r.session.rules();
        r.display_options()
            .iter()
            .position(|o| r.render_option(o, state, rules).contains(needle))
    }

    /// A REPL over a fresh `n`-player simulation (n in 2..=4). Used for the
    /// board-width note (S3): only the 4p board renders wider than 100 columns,
    /// which the 3p (90) and 2p (72) boards never do.
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
    /// keyed purely on the board's own rendered width. The note names that
    /// width and points at guide §2, and a second render does NOT repeat it.
    ///
    /// **Every count is the wide case since T2's V3 board** — 120/150/180
    /// columns at 2/3/4 players, all past the note's 100-column trigger, where
    /// the pre-redesign boards were 72/90/108 and only 4 players fired it. The
    /// note keys on the GRID width, not the composed frame's, which is wider
    /// still (198/163/180 since T3). Guide §2 states the frame widths; making
    /// the note frame-aware is on the carry-forward list.
    #[test]
    fn board_command_prints_a_one_time_width_note_on_a_wide_board() {
        let mut r = repl_np(4, 7);
        let width =
            render::board_grid(r.session.state(), r.session.rules()).lines().next().map(|l| l.chars().count()).unwrap_or(0);
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

    /// S3, re-pointed at the V3 board (T2): the note keys on the board's OWN
    /// rendered width, and 30-column tiles put EVERY player count past the
    /// hundred-column threshold — 2 players is 120 now, where it was 72. So
    /// the two-player game, which used to be the narrow case, is no longer
    /// one, and the note fires there too. That is the note telling the truth
    /// rather than the note breaking: a 120-column board on a 100-column
    /// terminal really does shear.
    ///
    /// The other branch keeps its coverage from a board that is genuinely
    /// narrow, which now takes a fixture rather than a player count — and
    /// which doubles as a check that the renderer assumes no board shape.
    #[test]
    fn the_board_width_note_fires_on_the_v3_board_and_still_not_on_a_narrow_one() {
        let mut r = repl_2p(4);
        let first = r.step("board");
        assert!(
            first.text.contains("board is 120 columns wide"),
            "the 2P V3 board is 120 columns and must say so: {}",
            first.text
        );
        assert!(!r.step("board").text.contains("columns wide"), "the note prints once per session");

        // A one-tile board: 30 columns, under the threshold, no note.
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 4).unwrap();
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0); 2]).expect("setup");
        let keep = *state.board_layout.areas.keys().next().expect("an area");
        state.board_layout.areas.retain(|&id, _| id == keep);
        let mut narrow = Repl::new(Session::for_test_from_state(state, rules));
        assert!(!narrow.step("board").text.contains("columns wide"), "a 30-column board is narrow");
    }

    /// I5 (third review): inside the composed frame the board is ALREADY on
    /// screen -- `wrap` re-renders it above every body -- so `board` must not
    /// emit a second copy. It used to emit the grid plus a second legend, 62
    /// rows at 4 players in a 140x50 terminal, losing 12 rows off the top:
    /// exactly the overflow F3 fixed on this branch, reachable again from the
    /// first entry of the `view` menu. In plain (piped / `screen` off) mode
    /// there is no frame, so `board` must still print the grid.
    #[test]
    fn board_command_does_not_duplicate_the_board_inside_the_composed_frame() {
        let mut r = repl_np(4, 7);
        screen::force(true);
        let out = r.step("board");
        screen::force(false);
        assert!(
            !out.text.contains('┌'),
            "the frame already shows the board; the body must not repeat it:\n{}",
            out.text
        );
        assert!(
            out.text.contains("already above"),
            "and it must say where the board is instead: {}",
            out.text
        );
    }

    /// I5's other half: with frames off (a piped run, or `screen` toggled
    /// off) nothing else draws the board, so `board` must still draw it.
    #[test]
    fn board_command_still_prints_the_board_in_plain_mode() {
        let mut r = repl_np(4, 7);
        screen::force(false);
        let out = r.step("board");
        assert!(out.text.contains('┌'), "plain mode has no frame; `board` is the only board:\n{}", out.text);
        assert!(out.text.contains("site slots"), "and its legend with it: {}", out.text);

        // Same in a composed session whose user turned the screen off.
        let mut r2 = repl_np(4, 7);
        r2.step("screen"); // composed = false
        screen::force(true);
        let out2 = r2.step("board");
        screen::force(false);
        assert!(out2.text.contains('┌'), "`screen` off means no frame, so the board must print:\n{}", out2.text);
    }

    /// I5's height pin, mirroring `screen.rs`'s opening-frame one: the
    /// composed frame around the `view` menu's board entry -- the FIRST row of
    /// the discoverability menu, and the path a tester hits before any other --
    /// must not be TALLER than the opening frame at the same count.
    ///
    /// **The bound was a flat 50 rows until T3 and is now relative**, for the
    /// reason `screen::FRAME_HEIGHTS` records in full: T3 stacks the whole
    /// designed dashboard below the board at 3 and 4 players, which puts those
    /// two opening frames at 75 and 84 rows until T10 re-lays them into the 4P
    /// mock's four columns. What this test was ever really checking is that
    /// this PATH costs no more rows than the screen the player was already
    /// looking at -- I5's finding was a menu entry that re-printed a board the
    /// frame above it had already drawn -- and stated that way it keeps its
    /// grip at every count instead of being disabled at two of them. The
    /// absolute floor stays asserted where it still holds, in `screen.rs`.
    #[test]
    fn view_menu_board_entry_is_no_taller_than_the_opening_frame() {
        for n in [2u8, 3, 4] {
            let mut r = repl_np(n, 7);
            screen::force(true);
            let opening = screen::wrap(&r, &r.prompt()).lines().count();
            r.step("view");
            let out = r.step("0"); // Board
            let rows = screen::wrap(&r, &out.text).lines().count();
            screen::force(false);
            assert!(
                rows <= opening,
                "the {n}P view->board frame is {rows} rows against an opening frame of \
                 {opening}: the menu entry must not add a second board"
            );
        }
    }

    /// Item 7 (board connectivity): every `board` render carries its legend.
    /// Since V3 (T2) that legend keys the tile MARKS and the site-slot tokens
    /// -- the compact vocabulary the tiles draw -- rather than the 5-Power
    /// letters the 16-column tile used, which V3 spells out in words.
    #[test]
    fn board_command_prints_the_tile_mark_and_site_slot_legend() {
        let mut r = repl_2p(4);
        let out = r.step("board");
        assert!(out.text.contains("marks:"), "{}", out.text);
        assert!(out.text.contains("site slots:"), "{}", out.text);
    }

    #[test]
    fn prompt_numbers_the_legal_options() {
        let r = repl_2p(4);
        let p = r.prompt();
        assert!(p.contains("[0]"), "{p}");
        assert!(p.contains("End additional-action phase"), "{p}");
    }

    /// Ceremony reduction (Task 6): bare Enter on a forced single-option
    /// prompt applies that option through the normal commit path -- one real
    /// recorded event, phase advanced -- and the prompt advertised it.
    #[test]
    fn bare_enter_applies_a_forced_single_option() {
        let mut r = repl_2p(4);
        let p = r.prompt();
        assert!(p.contains("(only option — press Enter to apply)"), "{p}");
        let events_before = r.session.record().events.len();
        let out = r.step("");
        assert!(!out.quit);
        assert_eq!(r.session.record().events.len(), events_before + 1, "Enter must record the move");
        assert_eq!(r.session.state().turn_phase, TurnPhase::Placement);
        assert!(matches!(
            r.session.record().events.last(),
            Some(RecordedEvent::Move(Move::EndAdditionalPhase))
        ), "{:?}", r.session.record().events.last());
    }

    /// A Movement-phase state whose ONLY displayed entry is the GROUPED
    /// retreat family: the President stands in the single area the player
    /// occupies, so no destination is reachable (ordinary movement needs
    /// the mover's own assistant at the destination) and the two
    /// assistants there give the retreat family three recovery variants,
    /// past the singleton that would stay a `Direct`. Hand-built via
    /// `for_test_from_state` for the same reason the Newspaper fixture
    /// above is: the real path to an emergency-retreat position is many
    /// turns long.
    fn retreat_only_movement_fixture() -> Repl {
        let mut state = create_test_game_state(2);
        state.turn_phase = TurnPhase::Movement;
        let area = AreaId::new(0);
        state
            .board_layout
            .areas
            .insert(area, AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 0) });
        let p = &mut state.players[0];
        p.president_location = PresidentLocation::OnBoard(area);
        p.assistants.in_hand = p.assistants.in_hand.saturating_sub(2);
        p.assistants.on_board.insert(area, 2);
        p.money = 0;
        Repl::new(Session::for_test_from_state(state, Rules::empty_for_tests()))
    }

    /// F4 (Codex adversarial review of PR #4): the bare-Enter hint fires on
    /// one DISPLAYED entry, but a displayed entry is not always a move --
    /// a grouped family (Route/Retreat/placement) opens a sub-prompt
    /// instead. Promising "press Enter to apply" there tells a player their
    /// keystroke commits something when it only opens a picker, which is
    /// exactly the wrong thing to say in the emergency-retreat position
    /// this fixture reproduces. The hint must be family-aware.
    #[test]
    fn bare_enter_hint_says_open_not_apply_for_a_grouped_single_option() {
        let mut r = retreat_only_movement_fixture();
        let p = r.prompt();
        assert_eq!(r.display_options().len(), 1, "fixture must display exactly one entry:\n{p}");
        assert!(p.contains("Retreat President to hand"), "the lone entry is the grouped family:\n{p}");
        assert!(
            p.contains("(only option — press Enter to open)"),
            "a grouped entry OPENS a sub-prompt; it does not apply a move:\n{p}"
        );
        assert!(!p.contains("press Enter to apply"), "must not promise an apply here:\n{p}");

        // And the behavior the wording now describes: Enter opens the
        // guided retreat sub-prompt without recording anything.
        let events_before = r.session.record().events.len();
        let out = r.step("");
        assert!(out.text.contains("Guided retreat"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before, "opening a picker records no move");
    }

    /// Bare Enter on a MULTI-option prompt stays a harmless re-prompt (no
    /// advertisement line either), so a stray newline can never move.
    #[test]
    fn bare_enter_on_multi_option_prompt_applies_nothing() {
        let mut r = repl_2p(4);
        r.step(""); // -> Placement (3 grouped entries)
        let p = r.prompt();
        assert!(!p.contains("press Enter to apply"), "{p}");
        let events_before = r.session.record().events.len();
        r.step("");
        assert_eq!(r.session.record().events.len(), events_before, "Enter must be a no-op here");
        assert_eq!(r.session.state().turn_phase, TurnPhase::Placement);
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
        place_setup_dummies(&mut state); // I9 (PR #5 §2.1): a valid 2P state carries the setup dummy
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

    /// I3 (third review): an autosave failure during `undo` has to be
    /// VISIBLE. It used to go to stderr -- an in-code comment called that a
    /// known wart, "logged for the final whole-branch review rather than
    /// redesigned around" -- and that review then added the composed frame,
    /// which opens with clear-screen + home, turning the warning from ugly
    /// into unreadable. The live scenario: the save target has gone
    /// unwritable, the operator undoes a mis-entry, reads "Undid last
    /// entry.", and the on-disk record still contains the bad move. Assert
    /// the CHANNEL -- `StepOutput.text` is the only thing `main.rs` folds
    /// into a frame -- not merely that some string exists somewhere.
    #[test]
    fn undo_autosave_failure_is_reported_in_the_step_output_not_stderr() {
        let mut r = repl_2p(4);
        r.step("0"); // one real recorded event to undo
        assert_eq!(r.session.record().events.len(), 1, "fixture: something to undo");
        // A save target that cannot be written: its parent directory does not exist.
        r.session.save_path =
            Some(std::env::temp_dir().join("yokohama_i3_no_such_dir").join("game.json"));

        let out = r.step("undo");
        assert!(out.text.contains("Undid last entry."), "{}", out.text);
        assert!(
            out.text.contains("WARNING: autosave failed"),
            "the autosave failure must reach the screen, not stderr: {}",
            out.text
        );
    }

    /// A live Repl where player 1's starting order was dealt but kept hidden
    /// (`Unknown`), returned with the card that was dealt so a test can reveal
    /// it. Seed 21 is the same fixture `session::tests`'
    /// `apply_standalone_reveals_hidden_starting_order_in_a_live_session` uses.
    fn repl_live_hidden(seed: u64) -> (Repl, OrderCardId) {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let hidden = new_game(config.clone(), &rules, seed).unwrap().deals[1][0];
        let session = Session::new_live(
            config,
            seed,
            SetupObservations::default(),
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown],
            rules,
        )
        .unwrap();
        (Repl::new(session), hidden)
    }

    /// Item 5 (third review): `reveal <seat> <card>` discarded the delta
    /// `apply_standalone` returns, swallowing the autosave-failure warning it
    /// carries -- the same family as the already-fixed `undo` delta bug (I3).
    /// The warning must reach the screen (`StepOutput.text`, the only channel
    /// `main.rs` folds into a frame), not stderr where the composed frame
    /// erases it.
    #[test]
    fn reveal_cmd_surfaces_the_autosave_failure_warning() {
        let (mut r, hidden) = repl_live_hidden(21);
        // A save target that cannot be written: its parent directory is absent.
        r.session.save_path =
            Some(std::env::temp_dir().join("yokohama_reveal_no_such_dir").join("game.json"));
        let out = r.step(&format!("reveal 1 O{:02}", hidden.0));
        assert!(out.text.contains("revealed"), "the explicit confirmation still prints: {}", out.text);
        assert!(
            out.text.contains("WARNING: autosave failed"),
            "the autosave failure must reach the screen, not stderr: {}",
            out.text
        );
    }

    /// The other half: a WRITABLE target reveals with no warning -- the
    /// warning is a real failure report, not decoration on every reveal -- and
    /// the surfaced delta shows the revealed card entering the seat's hand.
    #[test]
    fn reveal_cmd_with_a_writable_target_reports_no_warning() {
        let (mut r, hidden) = repl_live_hidden(21);
        let path = std::env::temp_dir().join(format!("yokohama_reveal_ok_{}.json", std::process::id()));
        r.session.save_path = Some(path.clone());
        let out = r.step(&format!("reveal 1 O{:02}", hidden.0));
        std::fs::remove_file(&path).ok();
        assert!(!out.text.contains("autosave failed"), "a successful autosave must stay quiet: {}", out.text);
        assert!(out.text.contains(&format!("hand +O{:02}", hidden.0)), "the delta is surfaced: {}", out.text);
    }

    /// Item 4 (third review): every state VIEW opens with the same `─ Title ─`
    /// rule header. The render.rs pinning test enumerates only the six views
    /// that live there, so it structurally could not see the four that live in
    /// repl.rs (`score`/`log`/`final_score`/`hand`). This drives every view
    /// through the actual `VIEW_MENU` dispatch -- so it cannot miss one that is
    /// in the menu -- plus the game-over `final_score` table (which is not a
    /// menu row, being `score`'s game-over form), and holds the shared header
    /// on all of them. The board grid is its own layout, not a `─ Title ─`
    /// view, and is skipped.
    #[test]
    fn all_state_views_share_the_rule_header_style() {
        let mut r = repl_2p(4);
        for (label, render_fn) in Repl::VIEW_MENU.iter() {
            if label.starts_with("Board") {
                continue;
            }
            let out = render_fn(&mut r);
            assert!(
                out.text.starts_with("─ "),
                "view '{label}' must open with a `─ Title ─` rule header, got: {:?}",
                out.text.lines().next().unwrap_or("")
            );
        }
        // `final_score` is not a menu row (it is `score`'s game-over form);
        // pin it directly. `final_score_table` scores the current state, so it
        // renders on a live game without needing to drive it to a winner.
        assert!(
            r.final_score_table().starts_with("─ "),
            "final_score must open with a rule header, got: {:?}",
            r.final_score_table().lines().next().unwrap_or("")
        );
    }

    /// The other half of I3: a WRITABLE target undoes silently -- the
    /// warning is a real failure report, not decoration on every undo.
    #[test]
    fn undo_with_a_writable_save_target_reports_no_warning() {
        let mut r = repl_2p(4);
        r.step("0");
        let path = std::env::temp_dir().join(format!("yokohama_i3_ok_{}.json", std::process::id()));
        r.session.save_path = Some(path.clone());

        let out = r.step("undo");
        std::fs::remove_file(&path).ok();
        assert!(out.text.contains("Undid last entry."), "{}", out.text);
        assert!(!out.text.contains("autosave failed"), "a successful autosave must stay quiet: {}", out.text);
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
                out.text.trim_end().ends_with("'help' for the list)"),
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

    /// The main-list pager (20/page + "page N/M" footer + `more` wrap) still
    /// serves genuinely large DIRECT lists. Placement no longer produces one
    /// (Task 6 collapsed it to picker entries), so this drives the pager off
    /// an Employment Agency composite list at power 5 -- the engine offers
    /// every take combination up to 4 items, comfortably over 20.
    #[test]
    fn prompt_pages_large_direct_lists_with_page_footer() {
        let mut state = create_test_game_state(2);
        let ea = AreaId::new(0);
        state
            .board_layout
            .areas
            .insert(ea, AreaConfig { area_type: AreaType::EmploymentAgency, grid_position: (0, 0) });
        let me = state.current_player;
        state.players[me.0 as usize].president_location = PresidentLocation::OnBoard(ea);
        state.players[me.0 as usize].money = 20;
        state.players[me.0 as usize].assistants.on_board.insert(ea, 4);
        state.players[me.0 as usize].assistants.in_hand -= 4;
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(5);
        let mut r = Repl::new(Session::for_test_from_state(state, Rules::empty_for_tests()));

        let n = r.session.legal().len();
        assert!(n > 20, "fixture assumption: EA at power 5 must offer >20 composites, got {n}");
        let p0 = r.prompt();
        assert!(p0.contains("more"), "{p0}");
        assert!(p0.contains("page 1/"), "{p0}");
        let p1 = r.step("more").text;
        assert!(p1.contains("page 2/"), "{p1}");
        assert_ne!(p0, p1);
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
        assert!(p.contains("enter a number, or a command ('view' for game info, 'help' for the list)"), "{p}");
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
        assert!(p.contains("enter a number, or a command ('view' for game info, 'help' for the list)"), "{p}");
        assert!(!p.contains("'m <from> <to>'"), "{p}");
        assert!(!p.contains("'a'/'b'"), "{p}");
    }

    /// Task 6 privacy: hidden by default in LIVE mode (the laptop gets
    /// passed around a physical table), open by default in SIM.
    #[test]
    fn privacy_defaults_on_in_live_mode_and_off_in_sim() {
        let sim = repl_2p(4);
        assert!(!sim.privacy, "sim defaults open");

        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let live = Repl::new(
            Session::new_live(
                config, 903, SetupObservations::default(),
                vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown],
                rules,
            )
            .unwrap(),
        );
        assert!(live.privacy, "live defaults hidden");
    }

    /// The `privacy` command toggles, and a hidden hand masks its contents
    /// (count only) in the `hand` view while the toggle-back restores them.
    #[test]
    fn privacy_toggle_masks_the_hand_view() {
        let mut r = repl_2p(4);
        let open = r.step("hand").text;
        assert!(open.contains("O"), "sim shows codes by default: {open}");

        let out = r.step("privacy");
        assert!(out.text.contains("HIDDEN"), "{}", out.text);
        let masked = r.step("hand").text;
        assert!(masked.contains("hidden"), "{masked}");
        assert!(masked.contains("1 card"), "count survives: {masked}");
        assert!(!masked.contains(" -> "), "card text (order_compact_line) must not leak: {masked}");

        r.step("privacy");
        assert!(r.step("hand").text.contains(" -> "), "toggle back restores contents");
    }

    /// Owner scope item 10 (discoverability): `view` opens a one-shot
    /// numbered game-info menu naming every public-state surface; picking a
    /// number renders that surface and drops back to the normal prompt.
    #[test]
    fn view_menu_lists_surfaces_and_renders_the_picked_one() {
        let mut r = repl_2p(4);
        let out = r.step("view");
        assert!(out.text.contains("Game info"), "{}", out.text);
        for name in ["Board", "Players", "Port offer", "Lab offer", "Tracks", "Building sites", "Goals", "Hand", "Score", "Log"] {
            assert!(out.text.contains(name), "menu must name {name}: {}", out.text);
        }
        assert!(out.text.contains("'cancel'"), "{}", out.text);

        // Pick the Goals entry by its number parsed from the menu itself.
        let goals_idx = out
            .text
            .lines()
            .find(|l| l.contains("Goals"))
            .and_then(|l| l.strip_prefix('[')?.split(']').next()?.parse::<usize>().ok())
            .expect("Goals entry carries its number");
        let shown = r.step(&goals_idx.to_string());
        assert!(shown.text.contains("G0:"), "picking Goals must render the goals view: {}", shown.text);
        assert!(matches!(r.sub, SubPrompt::None), "the menu is one-shot");
        assert!(r.session.record().events.is_empty(), "views never apply moves");

        // Back at the normal prompt: a move command works again.
        let p = r.prompt();
        assert!(p.contains("[0]"), "{p}");
    }

    /// `views` and `?` alias the menu; `cancel` closes it; an out-of-range
    /// number re-asks.
    #[test]
    fn view_menu_aliases_cancel_and_range() {
        let mut r = repl_2p(4);
        for alias in ["views", "?"] {
            let out = r.step(alias);
            assert!(out.text.contains("Game info"), "'{alias}' must open the menu: {}", out.text);
            let out = r.step("cancel");
            assert!(matches!(r.sub, SubPrompt::None), "cancel closes the menu");
            assert!(out.text.contains("[0]"), "main prompt back after cancel: {}", out.text);
        }
        r.step("view");
        let out = r.step("99");
        assert!(out.text.contains("pick a number"), "out-of-range re-asks: {}", out.text);
        assert!(out.text.contains("Game info"), "menu still showing: {}", out.text);
    }

    /// The hint line on EVERY prompt names the menu — the discoverability
    /// layer that survives piped/rolling modes.
    #[test]
    fn input_hint_names_the_view_menu() {
        let r = repl_2p(4);
        let p = r.prompt();
        assert!(p.contains("('view' for game info"), "{p}");
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
        place_setup_dummies(&mut state); // I9 (PR #5 §2.1): a valid 2P state carries the setup dummy
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
        // Task 6: the Placement list is grouped (picker entries), so raw
        // legal-list indices no longer address it -- the `b` grammar does
        // the same placement in one line.
        assert!(
            r.session.legal().iter().any(|m| matches!(m,
                Move::PlaceTwoSameArea { area, adjacent_bonus: None } if *area == church_area)),
            "Option B at Church is legal turn 1"
        );
        r.step(&format!("b {}", church_area.0));

        // Movement phase: PlacePresident entries stay Direct in the grouped
        // display; find the one for Church by its rendered text.
        let church_label = crate::describe::area_label(r.session.state(), church_area);
        let idx = display_index_containing(r, &format!("Place President from hand in {church_label}"))
            .or_else(|| display_index_containing(r, &format!("Move President to {church_label}")))
            .expect("President can reach Church turn 1");
        r.step(&idx.to_string());

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
        // Task 6 (grouped Placement list): drive the placement via the `b`
        // grammar rather than a raw legal-list index.
        let area = r
            .session
            .legal()
            .iter()
            .find_map(|m| match m {
                Move::PlaceTwoSameArea { area, adjacent_bonus: None } => Some(*area),
                _ => None,
            })
            .expect("some Option B placement is legal and affordable turn 1 (no President is on board yet to charge a fee)");
        r.step(&format!("b {}", area.0));
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
        // Task 6 (grouped Placement list): drive the placement via the `a`
        // grammar rather than a raw legal-list index.
        let (a, b) = r
            .session
            .legal()
            .iter()
            .find_map(|m| match m {
                Move::PlaceThreeAreas { placements } if placements.len() == 2 => {
                    Some((placements[0], placements[1]))
                }
                _ => None,
            })
            .expect("a 2-area Option A placement is legal and affordable turn 1 (same reasoning as the Option B fixture -- no President is on board yet to charge a fee)");
        r.step(&format!("a {} {}", a.0, b.0));
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

        let mut state = GameState {
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
        place_setup_dummies(&mut state); // I9 (PR #5 §2.1): a valid 2P state carries the setup dummy
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
        let mut state = GameState {
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
        place_setup_dummies(&mut state); // I9 (PR #5 §2.1): a valid 2P state carries the setup dummy
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

    // ===================================================================
    // Task 6: Placement pickers + assistant-move grouping.

    /// The Placement main list collapses the 186-option wall into picker
    /// entries: Option A, Option B, the empty placement -- three lines, no
    /// pagination footer.
    #[test]
    fn placement_list_collapses_to_picker_entries() {
        let mut r = repl_2p(4);
        r.step("0"); // -> Placement
        assert!(r.session.legal().len() > 100, "fixture: the raw wall is still combinatorial");
        let p = r.prompt();
        assert!(p.contains("[0] Option A — place 1 assistant each in up to 3 areas"), "{p}");
        assert!(p.contains("[1] Option B — place 2 assistants in one area (10 eligible"), "{p}");
        assert!(p.contains("[2] Place no assistants"), "{p}");
        assert!(!p.contains("[3]"), "exactly three entries: {p}");
        assert!(!p.contains("page 1/"), "no pagination on the grouped list: {p}");
        assert!(p.contains("'a' grammar works too") && p.contains("'b' grammar works too"), "{p}");
    }

    /// Option-A picker: toggling two areas assembles the exact 2-area
    /// placement; the screen shows the live selection + legality/fee line;
    /// `confirm` commits it through the standard legal-list match.
    #[test]
    fn place_a_picker_toggles_areas_and_commits() {
        let mut r = repl_2p(4);
        r.step("0");
        let out = r.step("0"); // enter the Option A picker
        assert!(out.text.contains("toggle areas by id"), "{}", out.text);
        assert!(out.text.contains("confirm places no assistants"), "{}", out.text);

        let out = r.step("3");
        assert!(out.text.contains("Selected: A03"), "{}", out.text);
        assert!(out.text.contains("legal — "), "a 1-area selection is offered: {}", out.text);
        let out = r.step("5");
        assert!(out.text.contains("Selected: A03, A05"), "{}", out.text);

        let events_before = r.session.record().events.len();
        let out = r.step("confirm");
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert!(matches!(
            r.session.record().events.last(),
            Some(RecordedEvent::Move(Move::PlaceThreeAreas { placements }))
                if placements.len() == 2
                    && placements.contains(&AreaId::new(3))
                    && placements.contains(&AreaId::new(5))
        ), "{:?}", r.session.record().events.last());
    }

    /// Toggling an area a second time removes it; confirming an empty
    /// selection applies the engine's own "place no assistants" option.
    #[test]
    fn place_a_picker_toggle_off_and_empty_confirm() {
        let mut r = repl_2p(4);
        r.step("0");
        r.step("0"); // enter picker
        r.step("3");
        let out = r.step("3"); // toggle back off
        assert!(out.text.contains("confirm places no assistants"), "{}", out.text);

        let events_before = r.session.record().events.len();
        let out = r.step("confirm");
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert!(matches!(
            r.session.record().events.last(),
            Some(RecordedEvent::Move(Move::PlaceThreeAreas { placements })) if placements.is_empty()
        ), "{:?}", r.session.record().events.last());
    }

    /// SPOT-1 regression: the picker accepts the id format it DISPLAYS --
    /// the directory and Selected echo print `A03`, so `A03`/`a05` toggle
    /// exactly like `3`/`5` (case-insensitive prefix), and the commit is the
    /// same multiset.
    #[test]
    fn place_a_picker_accepts_the_displayed_area_id_format() {
        let mut r = repl_2p(4);
        r.step("0");
        r.step("0"); // enter the Option A picker
        let out = r.step("A03");
        assert!(out.text.contains("Selected: A03"), "displayed-format toggle must work: {}", out.text);
        let out = r.step("a05");
        assert!(out.text.contains("Selected: A03, A05"), "lowercase prefix too: {}", out.text);
        let out = r.step("A03"); // prefixed toggle-OFF must match the same area
        assert!(out.text.contains("Selected: A05"), "prefixed re-toggle removes: {}", out.text);

        let events_before = r.session.record().events.len();
        let out = r.step("confirm");
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert!(matches!(
            r.session.record().events.last(),
            Some(RecordedEvent::Move(Move::PlaceThreeAreas { placements }))
                if placements.len() == 1 && placements.contains(&AreaId::new(5))
        ), "{:?}", r.session.record().events.last());
    }

    /// SPOT-1 companion: the `a`/`b`/`m`/`r` grammars accept the displayed
    /// `A<id>` form through the same shared prefix strip.
    #[test]
    fn grammars_accept_the_displayed_area_id_format() {
        // `b A<id>` places, then `r A<id>:1` retreats with a count.
        let mut r = repl_2p(4);
        let area = drive_to_movement_with_on_board_assistants(&mut r);
        let events_before = r.session.record().events.len();
        let out = r.step(&format!("r A{:02}:1", area.0));
        assert!(!out.text.contains("not in the legal list"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);

        // `a A.. a..` multiset-matches like bare ids.
        let mut r2 = repl_2p(4);
        r2.step("0");
        let two = r2
            .session
            .legal()
            .iter()
            .find_map(|m| match m {
                Move::PlaceThreeAreas { placements } if placements.len() == 2 => {
                    Some((placements[0], placements[1]))
                }
                _ => None,
            })
            .expect("a 2-area Option A exists");
        let out = r2.step(&format!("a A{:02} a{}", two.0.0, two.1.0));
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r2.session.state().turn_phase, TurnPhase::Movement);

        // `m A<from> A<to>` resolves the pending assistant-move choice.
        let mut r3 = repl_2p(4);
        let church = drive_to_double_assistant_move_cascade(&mut r3);
        let out = r3.step(&format!("m A{:02} A09", church.0));
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r3.session.state().pending_choices, vec![PendingChoice::AssistantMove]);
    }

    /// SPOT-2: the Option-A picker's refusal line says WHY commands are
    /// unavailable and how to get out.
    #[test]
    fn place_a_picker_refusal_names_the_way_out() {
        let mut r = repl_2p(4);
        r.step("0");
        r.step("0"); // enter the Option A picker
        let out = r.step("help");
        assert!(
            out.text.contains("! toggle an area id, 'confirm', or 'cancel' (cancel first to use views/help)"),
            "{}",
            out.text
        );
        assert!(!out.text.contains("Commands:"), "help must not render mid-picker: {}", out.text);
    }

    /// Item 1 (third review): a numeric-but-nonexistent area id (A99 on a
    /// 10-area board) is rejected at TOGGLE time against the real placeable
    /// set, with its OWN message, and the existing selection is untouched --
    /// it never enters the selection to be discarded at `confirm`.
    #[test]
    fn place_a_picker_rejects_a_nonexistent_area_id_at_toggle_and_keeps_selection() {
        let mut r = repl_2p(4);
        r.step("0");
        r.step("0"); // enter picker
        r.step("3"); // a real area
        // Fixture sanity: the 2P board has 10 areas (ids 0-9), so A99 is off-board.
        assert_eq!(r.session.state().board_layout.areas.len(), 10, "2P board is 10 areas");
        let out = r.step("99"); // A99: parses as a u8, but not an area on this board
        assert!(out.text.contains("not a placement area"), "bad id gets its own message: {}", out.text);
        // The prior selection survives, and A99 never entered it: the Selected
        // line is exactly A03 (the message above echoes "A99", so a bare
        // !contains("A99") would be wrong -- pin the Selected line instead).
        assert!(out.text.contains("Selected: A03\n"), "the prior selection survives untouched: {}", out.text);
    }

    /// Item 2 (third review): an over-limit selection (4 areas without
    /// Newspaper) survives toggling -- max_areas is display guidance, the
    /// legal-list match stays the sole authority -- but it now gets its OWN
    /// message (distinct from the bad-id message) in both the live preview and
    /// at `confirm`, and `confirm` KEEPS the selection open (item 1) instead
    /// of discarding it to the main list. Deselecting one then commits.
    #[test]
    fn place_a_picker_over_limit_has_its_own_message_and_keeps_the_selection() {
        let mut r = repl_2p(4);
        r.step("0");
        r.step("0"); // enter picker
        r.step("3");
        r.step("5");
        r.step("6");
        let out = r.step("7"); // 4th area, over the max of 3
        assert!(out.text.contains("too many areas"), "live preview names over-limit distinctly: {}", out.text);
        assert!(!out.text.contains("not a placement area"), "distinct from the bad-id message: {}", out.text);

        let events_before = r.session.record().events.len();
        let out = r.step("confirm");
        assert!(out.text.contains("too many areas"), "confirm names over-limit distinctly: {}", out.text);
        assert_eq!(r.session.record().events.len(), events_before, "nothing applies");
        // Picker STAYS OPEN with the selection intact (item 1: don't discard).
        assert!(out.text.contains("Selected: A03, A05, A06, A07"), "selection kept on rejection: {}", out.text);

        // Deselecting one returns to a legal 3-area placement, which commits.
        r.step("7");
        let out = r.step("confirm");
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert!(!out.text.contains("too many"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1, "the 3-area placement applies");
    }

    /// Option-B picker: an id-addressed area list with per-area fee notes;
    /// typing an id previews that area's placement and `confirm` commits its
    /// single variant. No `[i]` numbering exists here at all (C1) -- the only
    /// thing typeable is an area id.
    #[test]
    fn place_b_picker_lists_areas_and_commits() {
        let mut r = repl_2p(4);
        r.step("0");
        let out = r.step("1"); // enter the Option B picker
        assert!(out.text.contains("Option B — place 2 assistants in one area (by id):"), "{}", out.text);
        assert!(!out.text.contains("[0] "), "areas must not be numbered: {}", out.text);
        assert!(out.text.contains("Selected: (none)"), "{}", out.text);

        let events_before = r.session.record().events.len();
        let out = r.step("2");
        assert!(out.text.contains("Selected: A02"), "{}", out.text);
        assert!(out.text.contains("legal — "), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before, "selecting must not commit");
        let out = r.step("confirm");
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert!(matches!(
            r.session.record().events.last(),
            Some(RecordedEvent::Move(Move::PlaceTwoSameArea { area, adjacent_bonus: None }))
                if area.0 == 2
        ), "{:?}", r.session.record().events.last());
    }

    /// C1 regression (third review, CRITICAL): the Option-B picker is driven
    /// by the area IDS it displays, never by a list position. `b_areas` omits
    /// the Canal (no `PlaceTwoSameArea` is ever legal there) while the id
    /// space still contains it, so on a 4-player board whose Canal sits
    /// mid-pyramid every area past it had a list index one BELOW its id: an
    /// operator reading `A15` off the screen and typing `15` placed two
    /// assistants in `A16` -- instantly, with no preview and no confirm --
    /// silently diverging a live-tracked record from the physical table.
    /// Seed 0 puts the 4-player Canal at A14, so A15..A17 are all past it.
    #[test]
    fn place_b_picker_places_in_the_area_id_typed_not_the_list_index() {
        let mut r = repl_np(4, 0);
        let canal = *r
            .session
            .state()
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type == AreaType::Canal)
            .expect("the 4-player board has a Canal")
            .0;
        let target = AreaId::new(canal.0 + 1);
        assert!(
            r.session.state().board_layout.areas.contains_key(&target),
            "fixture: the Canal (A{:02}) must sit BEFORE the last area, or there is no off-by-one to test",
            canal.0
        );

        r.step("0"); // -> Placement
        let b_entry =
            display_index_containing(&r, "Option B").expect("Option B entry present at 4P turn 1");
        let out = r.step(&b_entry.to_string());
        assert!(
            !out.text.contains(&format!("(A{:02})", canal.0)),
            "the Canal is not Option-B eligible, so its absence is what shifts every later id: {}",
            out.text
        );

        // The displayed id form is never refused, and it selects the area it
        // names -- the id, not the position it happens to occupy.
        let out = r.step(&format!("A{:02}", target.0));
        assert!(
            !out.text.contains("pick a numbered area"),
            "the picker must accept the id form it displays: {}",
            out.text
        );
        assert!(
            out.text.contains(&format!("Selected: A{:02}", target.0)),
            "the picker must echo the selected id: {}",
            out.text
        );
        assert!(out.text.contains("legal — "), "and preview the move before it commits: {}", out.text);

        let before = r.session.record().events.len();
        let out = r.step("confirm");
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), before + 1);
        assert!(
            matches!(
                r.session.record().events.last(),
                Some(RecordedEvent::Move(Move::PlaceTwoSameArea { area, .. })) if *area == target
            ),
            "typing A{:02} must place in A{:02}, not its neighbour: {:?}",
            target.0,
            target.0,
            r.session.record().events.last()
        );

        // ...and a BARE number means the same id, not a list index.
        let mut r2 = repl_np(4, 0);
        r2.step("0");
        let b_entry = display_index_containing(&r2, "Option B").expect("Option B entry present");
        r2.step(&b_entry.to_string());
        r2.step(&target.0.to_string());
        r2.step("confirm");
        assert!(
            matches!(
                r2.session.record().events.last(),
                Some(RecordedEvent::Move(Move::PlaceTwoSameArea { area, .. })) if *area == target
            ),
            "a bare number is an area id too: {:?}",
            r2.session.record().events.last()
        );
    }

    /// C1 rider: an id that is NOT one of the eligible areas is refused by
    /// name and nothing is applied -- the picker never silently substitutes a
    /// neighbour for an id it cannot honor.
    #[test]
    fn place_b_picker_refuses_an_ineligible_area_id_by_name() {
        let mut r = repl_np(4, 0);
        let canal = *r
            .session
            .state()
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type == AreaType::Canal)
            .expect("the 4-player board has a Canal")
            .0;
        r.step("0");
        let b_entry = display_index_containing(&r, "Option B").expect("Option B entry present");
        r.step(&b_entry.to_string());

        let before = r.session.record().events.len();
        let out = r.step(&format!("A{:02}", canal.0));
        assert!(
            out.text.contains(&format!("A{:02}", canal.0)) && out.text.contains("not one of the"),
            "an ineligible id must be refused BY NAME: {}",
            out.text
        );
        assert_eq!(r.session.record().events.len(), before, "nothing applies");
        let out = r.step("confirm");
        assert!(out.text.contains("select an area id first"), "and nothing is selected: {}", out.text);
        assert_eq!(r.session.record().events.len(), before, "still nothing applies");
    }

    /// The Option-B picker's per-area lines carry the fee note when an
    /// opponent President stands there (single-sourced from the same
    /// describe-side preview the option text uses), and only there.
    #[test]
    fn place_b_picker_marks_fee_areas() {
        let mut state = create_test_game_state(2);
        state.turn_phase = TurnPhase::Placement;
        let a0 = AreaId::new(0);
        let a1 = AreaId::new(1);
        state
            .board_layout
            .areas
            .insert(a0, AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        state
            .board_layout
            .areas
            .insert(a1, AreaConfig { area_type: AreaType::Church, grid_position: (0, 1) });
        state.players[1].president_location = PresidentLocation::OnBoard(a0); // Blue blocks A00
        let mut r = Repl::new(Session::for_test_from_state(state, Rules::empty_for_tests()));

        let p = r.prompt();
        let b_entry = p
            .lines()
            .find(|l| l.contains("Option B"))
            .expect("Option B entry present")
            .trim_start()
            .trim_start_matches('[')
            .chars()
            .next()
            .unwrap();
        let out = r.step(&b_entry.to_string());
        let a0_line = out.text.lines().find(|l| l.contains("(A00)")).expect("A00 listed");
        assert!(a0_line.contains("fee 2 yen (2 to Blue)"), "{a0_line}");
        let a1_line = out.text.lines().find(|l| l.contains("(A01)")).expect("A01 listed");
        assert!(!a1_line.contains("fee"), "no opponent President at A01: {a1_line}");
    }

    /// A pending AssistantMove choice groups its flat source x destination
    /// options by SOURCE; picking a source opens the numbered destination
    /// list; committing applies the exact engine move. Decline stays a
    /// direct line, and the `m` grammar still bypasses everything.
    #[test]
    fn assistant_move_options_group_by_source() {
        let mut r = repl_2p(4);
        drive_to_double_assistant_move_cascade(&mut r);
        let p = r.prompt();
        assert!(p.contains("Move an assistant from"), "{p}");
        assert!(p.contains("destinations"), "grouped entries show a destination count: {p}");
        assert!(p.contains("Decline the assistant move"), "{p}");
        let flat = r.session.legal().len();
        let shown = p.matches("\n[").count();
        assert!(shown < flat, "grouping must shrink the list ({shown} shown vs {flat} legal): {p}");

        // Pick the first grouped entry -> destination list -> commit [0].
        let out = r.step("0");
        assert!(out.text.contains("Assistant destinations from"), "{}", out.text);
        let events_before = r.session.record().events.len();
        let out = r.step("0");
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert!(matches!(
            r.session.record().events.last(),
            Some(RecordedEvent::Move(Move::ChooseAssistantMove { .. }))
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
        r.sub = SubPrompt::Pick { title: "Routes to Bank (A00)".to_string(), moves };

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
        assert!(matches!(r.sub, SubPrompt::Pick { .. }));
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

    // ---- Phase 3.2 Task 12: play vs bot (spec §9) ----

    use crate::bot::{BotChooser, BotError, BotSeats};

    /// A 2P sim `Repl` whose seats in `bot_seats` are played by the shipped
    /// family-weighted rung. Built the way the wizard builds one -- provenance
    /// stamped on the session, `Repl::new` reading its bot seats back out of it
    /// -- so every fixture here exercises the SAME restoration path a resumed
    /// save does, rather than a test-only constructor that could drift from it.
    fn repl_2p_bots(seed: u64, bot_seats: &[u8]) -> Repl {
        let mut seats = vec![None; 2];
        for s in bot_seats {
            seats[*s as usize] = Some(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT);
        }
        let bots = BotSeats::new(seats, 0x5A17_3B0C_9E44_D218);
        let mut session = Session::new_sim(
            GameConfig::for_player_count(2),
            seed,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        )
        .unwrap();
        session.set_run_seats(bots.seat_provenance());
        Repl::new(session)
    }

    /// MUST item 1 (spec §9), call site A: a bot on the START seat must have
    /// played before the human ever sees a prompt -- `main.rs` pumps before
    /// printing the opening frame. Without it a bot on seat 0 never moves at
    /// all, because nothing else in the REPL runs until the human types.
    #[test]
    fn pump_plays_the_start_seat_before_the_first_prompt() {
        let mut r = repl_2p_bots(4, &[0]);
        assert_eq!(r.session.state().current_player.0, 0, "fixture: seat 0 opens");

        let text = r.pump_bots();

        assert!(!r.session.record().events.is_empty(), "the start-seat bot must have moved: {text}");
        assert_eq!(r.session.state().current_player.0, 1, "the pump must hand over to the human seat");
        assert!(text.contains("Seat 0"), "each bot move is reported with its seat: {text}");
        assert!(!r.locked, "an ordinary bot turn must not lock the session down: {text}");
    }

    /// Walk the current seat's turn forward through the SESSION until exactly
    /// one decision is left before control passes to the next seat, and hand
    /// that last move back. A turn is many decision points for ONE player, so
    /// a fixture that wants to watch a hand-over has to reach the end of a turn
    /// first; the last move is left for the caller to make through the REPL,
    /// which is the code path these tests are actually about.
    fn advance_to_the_handover_move(r: &mut Repl) -> Move {
        for _ in 0..400 {
            let legal = r.session.legal();
            let mv = legal.first().expect("a decision point with at least one move").clone();
            let after = apply_move_unchecked(r.session.state(), &mv, r.session.rules()).expect("legal applies");
            if after.current_player != r.session.state().current_player {
                return mv;
            }
            r.session.apply(mv, vec![]).expect("legal applies");
        }
        panic!("the turn never handed over within 400 decisions");
    }

    /// The pump is a no-op at a human decision point, and it never appends a
    /// prompt of its own -- the prompt belongs to `out`, which its callers run
    /// after it (double-prompting is exactly what a second prompt source causes).
    #[test]
    fn pump_stops_at_a_human_seat_and_appends_no_prompt() {
        let mut r = repl_2p_bots(4, &[1]);
        assert!(r.pump_bots().is_empty(), "no bot to play while the human is to move");
        assert!(r.session.record().events.is_empty(), "nothing may be applied at a human seat");

        // The human finishes their turn; the pump then plays seat 1 inside the
        // same step (`finish_apply` is the commit path every numbered pick,
        // grammar and picker confirmation funnels through).
        let handover = advance_to_the_handover_move(&mut r);
        let out = r.finish_apply(handover, vec![]);

        assert!(out.text.contains("Seat 1"), "the bot's reply rides the human's step output: {}", out.text);
        assert_eq!(r.session.state().current_player.0, 0, "control comes back to the human");
        assert_eq!(
            out.text.matches("\n> ").count(),
            1,
            "exactly one prompt closes the step -- the pump must not add its own: {}",
            out.text
        );
    }

    /// MUST item 2: one `undo` unwinds the whole trailing bot run PLUS the
    /// human's own last move, so undo means what a player thinks it means. The
    /// boundary is REPLAY-DERIVED (pop, rebuild, look at `current_player`),
    /// never a remembered stack -- `RecordedEvent::Move` carries no seat.
    #[test]
    fn undo_unwinds_the_bot_run_by_replay_derived_attribution() {
        let mut r = repl_2p_bots(4, &[1]);
        let handover = advance_to_the_handover_move(&mut r);
        let before_human_move = r.session.record().events.len();
        r.finish_apply(handover, vec![]);
        let unwound = r.session.record().events.len() - before_human_move;
        assert!(unwound >= 2, "fixture: the human's move plus at least one bot move, got {unwound}");
        assert_eq!(r.session.state().current_player.0, 0, "fixture: the bot handed back");

        let out = r.step("undo");

        assert_eq!(
            r.session.record().events.len(),
            before_human_move,
            "the human's own move must come back too, and nothing before it: {}",
            out.text
        );
        assert_eq!(r.session.state().current_player.0, 0, "the human is to move again");
        assert!(
            out.text.contains(&format!("Undid your last move and the bot's reply ({unwound} entries).")),
            "the report must state the honest entry count ({unwound}): {}",
            out.text
        );
    }

    /// The same unwind after save -> quit -> reload. This is the case a
    /// remembered boundary stack cannot serve (the stack dies with the process),
    /// which is why attribution is derived from the rebuilt state instead.
    #[test]
    fn undo_after_resume_still_unwinds() {
        let mut r = repl_2p_bots(4, &[1]);
        let handover = advance_to_the_handover_move(&mut r);
        let before_human_move = r.session.record().events.len();
        r.finish_apply(handover, vec![]);
        let unwound = r.session.record().events.len() - before_human_move;
        assert!(unwound >= 2, "fixture: the human's move plus a bot reply");
        let path = std::env::temp_dir().join(format!("yokohama_bot_resume_{}.json", std::process::id()));
        r.session.save(&path).expect("save");

        let mut resumed = Repl::new(Session::load(&path, Rules::standard()).expect("load"));
        let out = resumed.step("undo");

        assert_eq!(resumed.session.record().events.len(), before_human_move, "{}", out.text);
        assert_eq!(resumed.session.state().current_player.0, 0);
        assert!(
            out.text.contains(&format!("({unwound} entries)")),
            "the resumed unwind must report the same count: {}",
            out.text
        );
        std::fs::remove_file(&path).ok();
    }

    /// `undo` with a bot on the START seat and no human move yet: the unwind
    /// runs out of log inside the bot's opening turn, the re-pump replays that
    /// turn from the same decision seeds, and the position is identical. The
    /// report has to SAY that rather than claim to have taken back a move the
    /// player never made -- otherwise it describes a change nobody can see.
    #[test]
    fn undo_before_the_human_has_moved_reports_nothing_of_theirs_and_replays_the_same_turn() {
        let mut r = repl_2p_bots(4, &[0]);
        r.pump_bots();
        let opening = r.session.record().events.clone();
        assert!(!opening.is_empty(), "fixture: the start-seat bot opened the game");

        let out = r.step("undo");

        assert!(
            out.text.contains("Nothing of your own to undo yet"),
            "the report must not claim a move the player never made: {}",
            out.text
        );
        assert_eq!(
            r.session.record().events,
            opening,
            "the same decision seeds must replay the same opening turn"
        );
        assert_eq!(r.session.state().current_player.0, 1, "and the human is still the one to move");
    }

    /// The `hand:` line of the players view for `seat`.
    fn panel_hand_line(panel: &str, seat: u8) -> String {
        let key = format!("(seat {seat}):");
        let start = panel.find(&key).unwrap_or_else(|| panic!("no seat {seat} in panel:\n{panel}"));
        panel[start..]
            .lines()
            .find(|l| l.trim_start().starts_with("hand:"))
            .unwrap_or_else(|| panic!("no hand line after seat {seat}:\n{panel}"))
            .to_string()
    }

    /// MUST item 3: a bot's hand holds its unfulfilled starting order, and
    /// `privacy` defaults OFF in sim -- so the flag alone would print the bot's
    /// private card to the player it is playing against. The mask is PER SEAT:
    /// bot hands are always hidden, the human's still obeys the toggle.
    #[test]
    fn bot_hand_is_masked_regardless_of_privacy() {
        let mut r = repl_2p_bots(4, &[1]);
        assert!(!r.privacy, "fixture: sim opens with privacy off");

        let open = r.step("players").text;
        assert!(panel_hand_line(&open, 0).contains('O'), "the human's own hand shows: {}", panel_hand_line(&open, 0));
        assert!(
            panel_hand_line(&open, 1).contains("hidden"),
            "the bot's hand must be masked even with privacy off: {}",
            panel_hand_line(&open, 1)
        );

        // The REASON differs, and it has to: `privacy` does not reveal a bot's
        // hand, so telling the player it toggles would be a false promise.
        assert!(
            panel_hand_line(&open, 1).contains("bot seat"),
            "a bot's mask must name the bot, not the toggle: {}",
            panel_hand_line(&open, 1)
        );

        r.step("privacy");
        let hidden = r.step("players").text;
        assert!(
            panel_hand_line(&hidden, 0).contains("'privacy' to toggle"),
            "the toggle still governs the human seat"
        );
        assert!(panel_hand_line(&hidden, 1).contains("bot seat"), "the bot stays masked, for its own reason");
    }

    /// A chooser that proposes whatever it was built with -- the seam that lets
    /// a test drive the failure shapes the honest chooser is supposed to make
    /// unreachable.
    /// Variant-PRESERVING (branch review, should-fix 5): the earlier form
    /// collapsed every error to `DeadEnd`, so `pump_bots_with`'s lockdown
    /// catch-all was unreachable through this seam and a future test passing
    /// `BotError::Belief(..)` would silently exercise the WRONG failure path
    /// while its message assertion still matched.
    struct FixedChooser(Result<Move, ()>, Option<fn() -> BotError>);

    impl FixedChooser {
        fn ok(mv: Move) -> Self {
            Self(Ok(mv), None)
        }
        fn err(make: fn() -> BotError) -> Self {
            Self(Err(()), Some(make))
        }
    }

    impl BotChooser for FixedChooser {
        fn choose(
            &mut self,
            _state: &GameState,
            _rules: &Rules,
            _seat: PlayerId,
            _policy: RolloutPolicy,
            _decision_seed: u64,
        ) -> Result<Move, BotError> {
            match &self.0 {
                Ok(mv) => Ok(mv.clone()),
                Err(()) => Err((self.1.expect("err chooser built with a maker"))()),
            }
        }
    }

    /// Failure shape 1 (spec §9): a bot move the engine's own legal list refuses
    /// is a driver/engine fault, not a mis-entry -- it gets the `Invariant`
    /// treatment (lockdown, with the reason on screen), never a silent skip or a
    /// fallback to some other move.
    #[test]
    fn bot_illegal_pick_locks_down_with_reason() {
        let mut r = repl_2p_bots(4, &[0]);
        // Nobody owns the Textile Mill at setup, and the opening phase is not an
        // additional-action phase either: this is refused on both counts.
        let mut rogue = FixedChooser::ok(Move::UseSpinningMill);

        let text = r.pump_bots_with(&mut rogue);

        assert!(r.locked, "an illegal bot proposal must lock the session down: {text}");
        assert!(text.contains("LOCKDOWN"), "{text}");
        assert!(text.contains("seat 0"), "the reason must name the seat: {text}");
        assert!(r.session.is_locked(), "the lockdown must persist into the record");
        assert!(r.session.record().events.is_empty(), "nothing may be applied");
    }

    /// Failure shape 2: a dead end (the policy has no move to make) halts the
    /// pump loudly and leaves the session PLAYABLE -- it is not a corruption.
    #[test]
    fn bot_dead_end_halts_the_pump_without_locking_down() {
        let mut r = repl_2p_bots(4, &[0]);
        let mut dead = FixedChooser::err(|| BotError::DeadEnd("no move".to_string()));

        let text = r.pump_bots_with(&mut dead);

        assert!(!r.locked, "a dead end is a halt, not a fault: {text}");
        assert!(text.contains("BOT HALT"), "the halt must be visible: {text}");
        assert!(text.contains("seat 0"), "and must name the seat: {text}");
        assert!(r.session.record().events.is_empty());
    }

    /// Resume restores WHO plays each seat from the record's run provenance --
    /// the same field the arena writes (spec §5), read back through the one
    /// construction path.
    #[test]
    fn resume_restores_bot_seats_from_provenance() {
        let r = repl_2p_bots(4, &[1]);
        let path = std::env::temp_dir().join(format!("yokohama_bot_prov_{}.json", std::process::id()));
        r.session.save(&path).expect("save");

        let resumed = Repl::new(Session::load(&path, Rules::standard()).expect("load"));

        assert!(!resumed.bots.is_bot(PlayerId::new(0)), "seat 0 was the human");
        assert!(resumed.bots.is_bot(PlayerId::new(1)), "seat 1 must come back a bot");
        assert_eq!(
            resumed.bots.seed(),
            r.bots.seed(),
            "the bot's stream seed rides the record, so a resumed game plays the same line"
        );
        let seats = &resumed.session.record().run.as_ref().expect("provenance").seats;
        assert_eq!(seats[0].controller, yokohama_core::CONTROLLER_HUMAN);
        assert_eq!(seats[1].controller, RolloutPolicy::FAMILY_WEIGHTED_DEFAULT.as_str());
        assert!(seats.iter().all(|s| s.observation == yokohama_core::OBSERVATION_VIEW));
        std::fs::remove_file(&path).ok();
    }

    /// Bots are SIM ONLY (spec §9). A live record that names one — which this
    /// driver cannot write, so it has been edited — resumes with that seat
    /// HUMAN and a warning on the opening frame, rather than putting an agent
    /// in front of a view `observe` will refuse.
    #[test]
    fn a_live_record_naming_a_bot_seat_resumes_it_as_human_with_a_warning() {
        let bots = BotSeats::new(vec![None, Some(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT)], 3);
        let mut session = Session::new_live(
            GameConfig::for_player_count(2),
            21,
            SetupObservations::default(),
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        )
        .unwrap();
        session.set_run_seats(bots.seat_provenance());

        let r = Repl::new(session);

        assert!(!r.bots.any(), "no seat may be played by a bot in live mode");
        assert!(
            r.session.load_warnings.iter().any(|w| w.contains("LIVE") && w.contains("HUMAN")),
            "the demotion must be visible: {:?}",
            r.session.load_warnings
        );
    }

    // ---- `suggest` (Phase 3.3 Task 10, spec §7) ---------------------------

    /// Play the game forward one move through the REPL's own dispatch, avoiding
    /// the two phases whose option `0` is a GROUPED family (which would open a
    /// sub-prompt instead of applying): Placement takes the `a` grammar's empty
    /// placement, Movement takes the `r` grammar's bare retreat, and every other
    /// phase -- and any phase at all while a choice is queued -- lists nothing
    /// but `Direct` entries, so `0` applies one.
    fn step_one_move(r: &mut Repl) {
        let queued = !r.session.state().pending_choices.is_empty();
        match r.session.state().turn_phase {
            TurnPhase::Placement if !queued => r.step("a"),
            TurnPhase::Movement if !queued => r.step("r"),
            _ => r.step("0"),
        };
    }

    /// A 2P simulation driven a few turns in and stopped at the first position
    /// that is BOTH mid-game (assistants on the board, more than ten events
    /// recorded) and genuinely BRANCHING -- more than one legal move.
    ///
    /// The branching half is load-bearing rather than tidy: a forced decision
    /// point renders "No runner-up: this decision point offered exactly one
    /// move", so a fixture that stopped at one would exercise neither the
    /// runner-up rows nor the widest lines the screen can print, and the frame
    /// pin below would be measuring a narrower body than a player will see.
    fn repl_mid_game(seed: u64) -> Repl {
        repl_mid_game_np(2, seed)
    }

    /// `repl_mid_game` at any player count (T10 review M1): the width pin has to
    /// render the 4-player screen, where the composed frame is NARROWEST (108
    /// columns) while the board is widest -- the case a 2P fixture cannot show.
    /// Same stopping condition, so the 2P fixture above is unchanged.
    fn repl_mid_game_np(players: u8, seed: u64) -> Repl {
        let mut r = repl_np(players, seed);
        drive_to_movement_with_on_board_assistants(&mut r);
        for _ in 0..40 {
            if r.session.is_over() {
                break;
            }
            if r.session.record().events.len() > 10 && r.session.legal().len() > 1 {
                return r;
            }
            step_one_move(&mut r);
        }
        panic!("{players}P: no branching mid-game decision point within 40 moves of setup");
    }

    /// **Spec §7's deterministic-repeatable clause, driven through `step`.** Two
    /// presses at one position return BYTE-IDENTICAL screens, and neither
    /// touches the game.
    ///
    /// The anti-vacuity halves matter as much as the equality: the screen has to
    /// carry a real suggestion (a refusal would also compare equal to itself),
    /// and the state/record/legal-list have to be untouched afterwards (a
    /// command that applied its own suggestion would still be repeatable in the
    /// trivial sense of always agreeing with itself at ITS new position).
    #[test]
    fn suggest_twice_at_one_position_prints_the_same_screen_and_moves_nothing() {
        let mut r = repl_mid_game(4);
        let state_before = r.session.state().clone();
        let events_before = r.session.record().events.len();

        let first = r.step("suggest");
        let second = r.step("suggest");

        assert_eq!(first.text, second.text, "a second press must not change the answer");
        assert!(first.text.contains("Suggested move:"), "a real suggestion, not a refusal:\n{}", first.text);
        assert!(first.text.contains("TOTAL"), "the term table must be there:\n{}", first.text);
        assert!(first.text.contains("victory_points"), "{}", first.text);
        assert!(
            first.text.contains("Runner-ups ("),
            "the fixture must be a branching point, so the runner-up rows are exercised too:\n{}",
            first.text
        );
        assert_eq!(*r.session.state(), state_before, "suggest must not move the game on");
        assert_eq!(r.session.record().events.len(), events_before, "and must record nothing");

        // ...and the answer IS a move the engine offers right now. Read as the
        // whole answer BLOCK rather than one line: a move description too wide
        // for the frame is reflowed across lines (T10 review M1), and wrapping
        // collapses the whitespace it breaks at, so both sides are compared word
        // by word.
        let words = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        let suggested = words(
            &first
                .text
                .lines()
                .skip_while(|l| !l.starts_with("Suggested move:"))
                .skip(1)
                .take_while(|l| l.starts_with("  "))
                .collect::<Vec<_>>()
                .join(" "),
        );
        assert!(!suggested.is_empty(), "the suggested move block:\n{}", first.text);
        assert!(
            r.session
                .legal()
                .iter()
                .any(|m| words(&describe::describe_move(m, r.session.state(), r.session.rules()))
                    == suggested),
            "the suggested move must be in the engine's own legal list: {suggested:?}"
        );
    }

    /// **The determinism clause across save -> quit -> reload**, which is the
    /// half the same-process pin above cannot reach and the half a natural
    /// implementation gets wrong: an agent (or an RNG, or a press counter) cached
    /// on the `Repl` dies with the process, so a resumed game would answer its
    /// own unchanged position with a DIFFERENT screen while still agreeing with
    /// itself in any one session. `bot.rs` states the same hazard for the bot's
    /// decision stream (its module doc: "a save/reload would silently restart
    /// the" stream) — this is that lesson applied to a read-only surface.
    ///
    /// It holds here because every component of `suggest::stream_seed` is a field
    /// of the SERIALIZED `GameRecord` — the seed, the player count, the start
    /// seat, each seat's kept starting order, and the event count — so the key is
    /// re-derived rather than remembered. Compared byte for byte, prompt line
    /// included, because that is what the player sees.
    #[test]
    fn suggest_after_save_and_reload_prints_the_same_screen() {
        let mut r = repl_mid_game(4);
        let before = r.step("suggest").text;
        assert!(
            before.contains("Suggested move:"),
            "fixture: a real suggestion, not a refusal:\n{before}"
        );

        let path = std::env::temp_dir()
            .join(format!("yokohama_suggest_resume_{}.json", std::process::id()));
        r.session.save(&path).expect("save");
        let resumed_session = Session::load(&path, Rules::standard()).expect("load");
        std::fs::remove_file(&path).ok();
        let mut resumed = Repl::new(resumed_session);
        assert_eq!(
            resumed.session.record().events.len(),
            r.session.record().events.len(),
            "fixture: the reload must land on the same position"
        );

        let after = resumed.step("suggest").text;
        assert_eq!(before, after, "a resumed game must answer its own position identically");
    }

    /// Anti-vacuity for the pin above: the stream is keyed by the EVENT INDEX,
    /// so pressing `suggest` after a move is a different question -- the pin
    /// would otherwise also pass on a `suggest` that ignored the position
    /// entirely and printed a constant.
    #[test]
    fn the_suggest_stream_moves_with_the_position() {
        let mut r = repl_mid_game(4);
        let before = r.step("suggest").text;
        step_one_move(&mut r);
        let after = r.step("suggest").text;
        assert_ne!(before, after, "the key must include the event index");
    }

    /// **`suggest` is deliberately absent in live mode** (spec §7; 3.2's CLI
    /// scope stands). It refuses with the reason, applies nothing, and leaves
    /// the session exactly as it was.
    #[test]
    fn suggest_is_refused_in_live_mode() {
        let (mut r, _) = repl_live_hidden(21);
        let state_before = r.session.state().clone();

        let out = r.step("suggest");

        assert!(out.text.contains("simulation-only"), "the refusal must say so: {}", out.text);
        assert!(!out.text.contains("Suggested move:"), "and must not suggest anything: {}", out.text);
        assert_eq!(*r.session.state(), state_before);
        assert!(r.session.record().events.is_empty());
    }

    /// `suggest` answers for YOUR seat. A bot's seat is refused by name -- only
    /// reachable after a halted pump, but the alternative (advising the bot's
    /// side against you) is worse than a message.
    #[test]
    fn suggest_refuses_a_seat_a_bot_is_playing() {
        let mut r = repl_2p_bots(4, &[0]);
        assert!(r.bots.is_bot(r.session.state().current_player), "fixture: a bot is to move");

        let out = r.step("suggest");

        assert!(out.text.contains("played by a bot"), "{}", out.text);
        assert!(!out.text.contains("Suggested move:"), "{}", out.text);
    }

    /// **The suggest screen inside the COMPOSED FRAME, at every player count**
    /// (T10 review M1). `screen::wrap` sizes a frame to its widest line, so a
    /// body that outran the board would shove the whole game screen sideways for
    /// one command and back again -- the jitter F3 fixed vertically, in the other
    /// axis.
    ///
    /// What this replaced could not see the failure it was named for. It asserted
    /// `width(suggest) <= width(plain)` on ONE session, drove `repl_mid_game(4)`
    /// -- which is 2 players at seed 4, not 4 players -- and its own fixture
    /// assertion said "the 2P frame" while claiming to have measured a 4P one. The
    /// 4-player screen, the only one where the frame (108) is narrower than the
    /// widest thing `suggest` can print (135), was never rendered.
    ///
    /// Four assertions per count, each for its own reason:
    ///
    /// 1. Every line of the SUGGEST RENDER fits that count's frame width. The
    ///    render is isolated from the standing prompt block first, because
    ///    `out_view` appends the prompt to every view command and the prompt is a
    ///    different surface with a different budget (guide §2's 140 columns, held
    ///    by `screen`'s own pins) -- measuring them together would assert this
    ///    render's bound on someone else's line.
    /// 2. The printed body IS `suggest::render_suggestion`, byte for byte, at the
    ///    same state and the same stream key. **This is what makes the pin
    ///    non-vacuous, and it was measured rather than assumed**: an arbitrary
    ///    branching position's own suggestion usually fits every frame with no
    ///    help at all, so assertion 1 alone passes even on a render with the bound
    ///    ripped out. What is checked here is that this command prints the BOUNDED
    ///    render rather than formatting a screen of its own -- and the over-wide
    ///    case is driven where it can be controlled, in `suggest`'s own
    ///    `a_move_too_wide_for_the_frame_is_reflowed_and_rows_are_elided` (which
    ///    answer a search picks cannot be dictated from here).
    /// 3. The frame around the suggestion is no wider than the frame around an
    ///    ordinary prompt: the no-jitter property, which is about the composed
    ///    screen rather than about one body.
    /// 4. The mid-game frame is no NARROWER than the width the body was bounded
    ///    by. `screen::FRAME_WIDTHS` holds OPENING widths and a body is bounded
    ///    before the frame it lands in exists, so this is the check that the table
    ///    is a floor at a real position and not just at turn one.
    #[test]
    fn the_suggest_screen_fits_the_composed_frame_at_every_player_count() {
        for players in [2u8, 3, 4] {
            let mut r = repl_mid_game_np(players, 4);
            let body = r.step("suggest").text;
            assert!(body.contains("Suggested move:"), "{players}P fixture: a real suggestion: {body}");

            // The prompt block alone, at the same (untouched) state: what
            // `out_view` appended to the render above.
            let tail = r.out_view(String::new()).text;
            let render = body.strip_suffix(&tail).unwrap_or_else(|| {
                panic!("a view body is its render plus the standing prompt block:\n{body}")
            });

            let budget = screen::frame_width(players as usize);
            for line in render.lines() {
                let w = screen::visible_width(line);
                assert!(
                    w <= budget,
                    "{players}P: a rendered line is {w} columns against the frame's {budget}: {line:?}",
                );
            }

            // The state is untouched by a press, so the same key re-derives the
            // same answer (the determinism pins above) and this comparison is
            // exact rather than approximate.
            let seat = r.session.state().current_player;
            let key = crate::suggest::stream_seed(r.session.record(), seat);
            let s = crate::suggest::suggest(r.session.state(), r.session.rules(), seat, key)
                .expect("the same suggestion the command just printed");
            assert_eq!(
                render,
                crate::suggest::render_suggestion(&s, r.session.state(), r.session.rules()),
                "{players}P: the command must print the width-bounded render, not its own format",
            );

            screen::force(true);
            let plain = screen::wrap(&r, &r.prompt());
            let suggest = screen::wrap(&r, &body);
            screen::force(false);
            let width = |frame: &str| frame.lines().map(screen::visible_width).max().unwrap_or(0);
            assert!(
                width(&suggest) <= width(&plain),
                "{players}P: the suggest body widened the frame from {} to {} columns",
                width(&plain),
                width(&suggest),
            );
            assert!(
                width(&plain) >= budget,
                "{players}P: the mid-game frame is {} columns, under the {budget} the body was \
                 bounded by -- FRAME_WIDTHS is no longer a floor",
                width(&plain),
            );
        }
    }

    /// A finished game has nothing to suggest, and the refusal lands BEFORE the
    /// search rather than after it: a terminal position offers no move, so the
    /// agent would answer with a dead-end error message where a sentence
    /// belongs.
    #[test]
    fn suggest_refuses_a_finished_game() {
        let mut state = repl_2p(4).session.state().clone();
        state.end_game_triggered = true;
        state.final_round = true;
        state.trigger_player = Some(PlayerId::new(0));
        // The seat before the start player, at TurnEnd: everyone has had their
        // last turn (`GameState::is_game_over`'s own condition).
        state.current_player = PlayerId::new(1);
        state.turn_phase = TurnPhase::TurnEnd;
        let mut r = Repl::new(Session::for_test_from_state(state, Rules::standard()));
        assert!(r.session.is_over(), "fixture: the game must read as finished");

        let out = r.step("suggest");

        assert!(out.text.contains("game is over"), "{}", out.text);
        assert!(!out.text.contains("Suggested move:"), "{}", out.text);
    }

    /// A locked session refuses `suggest` along with everything else that is
    /// not save/log/quit -- checked because `suggest` is read-only and it would
    /// be easy to argue it into the allow-list, where it would run a search
    /// against a state the driver has already declared untrustworthy.
    #[test]
    fn a_locked_session_refuses_suggest() {
        let mut r = repl_2p(4);
        r.engage_lockdown("test".to_string());
        let out = r.step("suggest");
        assert!(out.text.contains("session locked"), "{}", out.text);
        assert!(!out.text.contains("Suggested move:"), "{}", out.text);
    }

    /// Same `(bot seed, event count)` in, same move out: the decision seed is a
    /// pure function of a PUBLIC counter and the recorded seed, which is what
    /// makes a resumed game replay the bot's own line rather than a fresh one.
    #[test]
    fn bot_decisions_are_reproducible_from_the_recorded_seed() {
        let mut a = repl_2p_bots(11, &[0]);
        let mut b = repl_2p_bots(11, &[0]);
        a.pump_bots();
        b.pump_bots();
        assert_eq!(a.session.record().events, b.session.record().events);
        assert!(!a.session.record().events.is_empty(), "fixture: the bot actually moved");
    }
}
