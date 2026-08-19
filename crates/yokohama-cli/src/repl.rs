//! Pure REPL step function over a `Session`: numbered-option prompts,
//! placement grammar, view/utility commands, and pagination -- live-mode
//! reveal choreography lives in `reveal.rs` (moved there at Task 10)
//! (design docs/plans/2026-07-18-phase2-cli-driver-design.md).
//! No stdin/stdout -- `main.rs` (Task 13) drives this with real I/O.

use yokohama_core::*;

use crate::card_prompt::{self, CardEntry, CardPrompt};
use crate::reveal::{lab_b_changed, refilled_port_slots, Pending};
use crate::screen_model::{
    menu_index, menu_number, menu_range, CardLocation, CardRef, IndexKind, IndexModel,
    RecordingTarget, NUMBER_BASE,
};
use crate::session::{ApplyError, TrackingMode};
use crate::style::{paint, Tone};
use crate::{decision_render, describe, render, screen};

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
    /// The full-screen surface open over whatever the player was doing — a card
    /// index (Phase-3.5a T5, spec §0.6) or a view (T6, spec §0.5).
    ///
    /// **An OVERLAY, deliberately, rather than a third capture state beside
    /// `pending` and `sub`.** §0.6 requires the index to be available INSIDE an
    /// input capture — *"that is where it is needed most"* — and §0.5 requires
    /// the same of the views: one action away, and back at the interrupted
    /// prompt with its state preserved. Modelling it as an overlay makes that
    /// structural instead of careful: opening it touches neither `pending` nor
    /// `sub`, so there is no state to preserve and none to restore, and closing
    /// it simply stops drawing it. The one test that could fail is a caller
    /// that CLEARS one of those fields on the way in, and nothing here does.
    pub(crate) overlay: Option<Overlay>,
}

/// The full-screen surface currently drawn over the frame.
pub(crate) enum Overlay {
    /// A card index, and everything about the screen that is not derivable
    /// from the game state underneath it.
    ///
    /// Which is the registry and the FILTER. What is being RECORDED is
    /// re-derived from `pending` on every render ([`Repl::index_target`])
    /// rather than captured at open time, so a reveal group that advances a
    /// slot while the index is up cannot leave the header describing the slot
    /// before it.
    Index { kind: IndexKind, filter: Option<crate::screen_model::IndexFilter> },
    /// A read-only view. Which one is all there is to remember: both are built
    /// fresh from current state on every render, exactly as the frame is.
    View(ViewSurface),
    /// The `view` game-info MENU.
    ///
    /// **An overlay since T11, and it was a sub-prompt before.** A sub-prompt
    /// renders UNDER the composed frame, so the menu — a roster of full-screen
    /// surfaces, titled like one — arrived twenty rows beneath a forty-six-row
    /// board, pushing the frame off the top to make room. Every surface it
    /// lists replaces the frame; it does now too, which is what its own title
    /// claims and what the Forge pass at T11 found it was not doing.
    ///
    /// It keeps its own input arm rather than sharing [`Repl::step_view`]'s:
    /// a view re-dispatches any line it does not recognize (looking at one and
    /// typing `score` should print the score), while a MENU's numbers select
    /// rows, so re-dispatching a digit here would apply a move instead of
    /// opening a surface.
    Menu,
}

/// The full-screen views (spec §0.5's view pattern).
///
/// **The whole roster the spec names**, less the four that ARE card indexes:
/// §0.5 lists `ports`, `labs`, `tracks`, `sites`, `goals`, `hand` and `log`, and
/// `goals` is the achievement index rather than a seventh member here — T6's
/// supersession ruling, since the index draws the same three cards with claim
/// state and per-seat progress and the old dense `achievements_view` had no card
/// on it at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewSurface {
    Tracks,
    Sites,
    Ports,
    Labs,
    /// The per-area action-payout REFERENCE (branch-4 T5) — the seventh
    /// member, and the one that is not a live offer: it answers "what does
    /// acting here pay me, at the power I will have" for every area at once,
    /// which no other surface says anywhere.
    Areas,
    Hand,
    Log,
}

impl ViewSurface {
    /// The command word that opens this view — and, because a toggle should be
    /// the key that opened it, the word that closes it again.
    fn command(self) -> &'static str {
        match self {
            ViewSurface::Tracks => "tracks",
            ViewSurface::Sites => "sites",
            ViewSurface::Ports => "ports",
            ViewSurface::Labs => "labs",
            ViewSurface::Areas => "areas",
            ViewSurface::Hand => "hand",
            ViewSurface::Log => "log",
        }
    }
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
    /// The guided `reveal` flow (T8, spec §0.2's "every decision reachable by
    /// numbered menu"): pick the SEAT whose hidden starting order just became
    /// public from a numbered list of the seats that have one, then name the
    /// card in the crate's one card grammar. `seat` is `None` while the menu is
    /// up and `Some` once it has been picked, mirroring `PlaceB`'s
    /// select -> answer shape.
    ///
    /// **The one variant here that collects HIDDEN information, which the
    /// paragraph above this enum says sub-prompts do not do** — so the claim is
    /// narrowed rather than left false. What that paragraph is really about is
    /// `pending`: a bound reveal is parked behind a move and nothing may be
    /// recorded until the group completes, which is a state machine no ordinary
    /// prompt should be able to enter. This one records a STANDALONE event with
    /// no move behind it (`Session::apply_standalone`, the same call the
    /// `reveal <seat> <card>` alias makes), so it needs no parking and belongs
    /// with the other guided entry helpers. It is live-only for the ordinary
    /// reason: no seat has a hidden starting order in a simulation.
    Reveal { seats: Vec<PlayerId>, seat: Option<PlayerId> },
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

/// The full-screen VIEW a command word opens, if any.
///
/// **A whole-line match, and only ever on one word.** These are intercepted
/// ahead of the capture branches so §0.5's "one action away" holds inside a
/// Movement sub-prompt; that interception is safe precisely because no card id,
/// area id or option number can spell `tracks`.
fn view_surface(word: &str) -> Option<ViewSurface> {
    match word {
        "tracks" => Some(ViewSurface::Tracks),
        "sites" => Some(ViewSurface::Sites),
        "ports" => Some(ViewSurface::Ports),
        "labs" => Some(ViewSurface::Labs),
        "areas" => Some(ViewSurface::Areas),
        "hand" => Some(ViewSurface::Hand),
        "log" => Some(ViewSurface::Log),
        _ => None,
    }
}

/// `There is no Seat 3 in this game — 2 players are seated, Seat 0 to Seat 1.`
///
/// One function rather than one string per caller: the `reveal` alias and the
/// guided menu both refuse this, and a player who reads two wordings for one
/// fact has to work out whether they are one fact.
fn no_such_seat(seat: u8, seated: usize) -> String {
    format!(
        "There is no Seat {seat} in this game — {seated} players are seated, Seat 0 to Seat {}.\n",
        seated.saturating_sub(1),
    )
}

/// `Seat 0's starting order is not hidden — nothing to reveal there.` The other
/// half of the pair above, and a DIFFERENT fact from it (T8 review NOTE-6a).
fn seat_not_hidden(seat: PlayerId) -> String {
    format!("Seat {}'s starting order is not hidden — nothing to reveal there.\n", seat.0)
}

/// The seat a token names at the guided reveal menu, or the refusal to print.
///
/// **IDENTITY numbering, T7's stable-numbering class** (T11 review N6). The
/// menu prints `[2] Seat 2`, so the number beside a row IS the seat and `2`
/// here means what `reveal 2` means at the prompt. It used to be a list
/// POSITION, which reads identically at a table where the hidden seats happen
/// to start at seat 1 and silently selects the wrong seat everywhere else.
///
/// Seat ids are identities rather than menu entries, so they are 0-based and
/// deliberately outside `NUMBER_BASE` — the exclusion T5's numbering ruling
/// names explicitly.
///
/// Refuses in three ways because there are three different things wrong, and a
/// player can only act on the one that applies: a number that is not a seat at
/// this table, a seat whose order is already public, and a token that is not a
/// seat number at all.
fn reveal_seat_choice(tok: &str, seats: &[PlayerId], seated: usize) -> Result<PlayerId, String> {
    let Some(n) = tok.parse::<u8>().ok() else {
        return Err("! pick a seat number from the list, or 'cancel'".to_string());
    };
    if usize::from(n) >= seated {
        return Err(no_such_seat(n, seated));
    }
    let seat = PlayerId::try_new(n)
        .map_err(|_| no_such_seat(n, seated))?;
    if !seats.contains(&seat) {
        return Err(seat_not_hidden(seat));
    }
    Ok(seat)
}

/// The card INDEX a command word opens, if any.
///
/// `goals` opens the achievement index rather than a separate view, which is
/// T6's supersession ruling: the index shows the same three cards with claim
/// state and per-seat progress, one card each, and the old `achievements_view`
/// was that information in a denser shape with no card.
fn index_command(word: &str) -> Option<IndexKind> {
    match word {
        "orders" => Some(IndexKind::Orders),
        "techs" => Some(IndexKind::Techs),
        "goals" => Some(IndexKind::Achievements),
        "sitecards" => Some(IndexKind::Sites),
        _ => None,
    }
}

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

/// Events the `log` VIEW carries. The view is full-screen, so what sizes this
/// is the design floor: 50 rows less the EIGHT this surface spends on chrome —
/// the title and the blank under it, the section rule, the blank under the
/// list, the "each line reads as it did" note, the two-line exit clause and the
/// caret. Forty entries therefore draw 48 rows, leaving TWO of slack against
/// the floor, which is what `view_render`'s `LOG_VIEW_ROWS` records and
/// `the_t11_views_fit_the_fifty_row_design_floor` pins. (The arithmetic here
/// read "nine … with one row of slack" until T11 review N2 counted it.)
pub(crate) const LOG_VIEW_TAIL: usize = 40;

/// Events the lockdown POSTMORTEM dump carries, which is a different surface
/// with a different job: it is pasted into a bug report under the seed and turn
/// number, not read on a screen, and ten entries is what a reader needs to see
/// what led into the fault without scrolling a report past it. Held at its
/// pre-T11 value deliberately — the view changed shape, the dump did not.
const LOG_DUMP_TAIL: usize = 10;

/// The screens a LOAD puts up before the REPL reads its first line.
///
/// **A type rather than eleven lines in `main`, because the interesting rule is
/// a relation between the two fields and `main` is the one place a test cannot
/// reach.** The rule is: a line is consumed from the reader if and only if
/// `review` is `Some`, and `review` is `Some` only where a human is at a
/// terminal to press it ([`crate::input::may_pause`]). With the composition
/// here, that reduces `main` to a `if let Some` whose one extra `next()` is
/// visibly inside it — and everything a piped run promises becomes a property
/// the suite can hold.
pub struct Opening {
    /// The bot-turn review surface to show and wait on, or `None` when there is
    /// nothing to review or nobody to wait for. A full-screen surface, prefix
    /// and all — print it as-is.
    pub review: Option<String>,
    /// The composed frame the player types into. When `review` is `Some` this is
    /// the ordinary opening frame for the position, byte-identical to what the
    /// same load would have drawn with no bot run in it; when it is `None` this
    /// is the whole opening exactly as it has always been.
    pub frame: String,
}

/// What the one line a bot-turn review reads turns out to mean.
///
/// Two arms and no third: this screen offers no choices, so everything that is
/// not the quit word is the same nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dismissal {
    /// The player asked to leave the program, and the review honours it exactly
    /// as every other prompt does.
    Quit,
    /// Anything else, including the bare Enter this screen is asking for.
    Discard,
}

/// Classify the line that dismissed a bot-turn review.
///
/// **A pure function in the library rather than a condition in `main`, which is
/// the point of it** (review SHOULD-2/SHOULD-3). The dismissal is the one input
/// path outside the REPL loop, and the loop is where every other line is judged;
/// leaving its semantics in `main` put an untestable decision on the one screen
/// a player meets before they have typed anything. Here it is a two-arm match
/// `main` cannot get wrong on its own, and the arms are pinned.
///
/// **Tokenized to agree with [`Repl::step`]** — `split_whitespace`, then the
/// same `["quit"] | ["q"]` pattern — so the review and the prompt underneath it
/// cannot disagree about what the quit word is. `step` trims first and this does
/// not, which is not a divergence: `split_whitespace` already discards exactly
/// the surrounding whitespace that trim would have, and the padded cells in
/// `a_dismissal_is_quit_for_exactly_the_words_the_prompt_calls_quit` are what
/// hold the two to the same answer. Case-sensitive for the same reason: `QUIT`
/// is not the quit word at any other prompt either.
pub fn dismissal(line: &str) -> Dismissal {
    let words: Vec<&str> = line.split_whitespace().collect();
    match words.as_slice() {
        ["quit"] | ["q"] => Dismissal::Quit,
        _ => Dismissal::Discard,
    }
}

/// What quitting prints, so the review's quit is the SAME event as the REPL's
/// rather than a lookalike.
///
/// The REPL's own eight arms still spell these bytes inline; this constant is
/// welded to them by `the_reviews_quit_prints_what_every_other_quit_prints`
/// rather than by a refactor of call sites this fix wave has no business
/// touching.
pub const QUIT_TEXT: &str = "bye\n";

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
                     support — they resume as HUMAN seats, so you are playing them",
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
            overlay: None,
        }
    }

    /// Play out any bot turn this load opens on, and compose the screens that
    /// go in front of the player before the REPL loop starts.
    ///
    /// **The bot pump is call site A (spec §9) and has to run here whatever else
    /// this does**: a bot on the start seat moves before the human ever sees a
    /// prompt, because nothing else in the REPL runs until the human types. What
    /// changed at the 3.5a fix wave is only where its NARRATIVE goes.
    ///
    /// Two shapes, and the split is the whole fix:
    ///
    /// * At a terminal with a bot run to show, the narrative becomes its own
    ///   full-screen review ([`crate::view_render::bot_review`]) and the frame
    ///   behind it is the ordinary opening — which then FITS, because the thing
    ///   that overran it is on the other screen.
    /// * Otherwise — a pipe, or nothing to review — the frame is the single
    ///   composed body this has always produced, in the same order, and no
    ///   pause is introduced for a reader that cannot press a key.
    ///
    /// Load WARNINGS go with the frame in both shapes, not with the review. They
    /// are provenance for the game about to be played rather than a report of
    /// what the bot did, and keeping them on the frame is what lets the reviewed
    /// frame be byte-identical to an unreviewed one.
    pub fn opening(&mut self) -> Opening {
        let mut warnings = String::new();
        for warning in &self.session.load_warnings {
            warnings.push_str(warning);
            warnings.push('\n');
        }
        // Provenance for the game about to be played, so it rides with the
        // warnings rather than with the review: a save reloaded after the
        // trigger says so QUIETLY (branch-4 item 5). The banner is
        // transition-fired and a load applies no move, so re-firing it here
        // would claim the trigger had just happened.
        warnings.push_str(&self.end_trigger_load_notice());
        // …but a flip INSIDE the opening pump is a transition this session
        // did observe, so it rides the narrative — into the review when there
        // is one, into the frame body when there is not.
        let mut narrative = self.pump_bots();
        narrative.push_str(&self.take_end_trigger_banner_text());

        // `frame_active` rather than `screen::enabled()`: the question is
        // whether `wrap` is about to compose a frame for this session at all,
        // and that predicate is the single source of truth for it — so the
        // review can never be put up over a body that was never going to be
        // framed.
        if crate::input::may_pause() && screen::frame_active(self) && !narrative.is_empty() {
            let review = screen::full_screen(&crate::view_render::bot_review(&narrative));
            let frame = screen::wrap(self, &format!("{warnings}{}", self.prompt()));
            return Opening { review: Some(review), frame };
        }
        let body = format!("{warnings}{narrative}{}", self.prompt());
        Opening { review: None, frame: screen::wrap(self, &body) }
    }

    /// The one-time END TRIGGERED banner, if this session has just watched the
    /// trigger fire; empty otherwise (branch-4 item 5).
    ///
    /// Drains [`Session::take_end_trigger_banner`], so calling it twice at one
    /// decision point yields the block once — "shown once" is the type's
    /// guarantee rather than call-site discipline. Sized at
    /// `screen::frame_width`, the same width authority `suggest` renders its
    /// body to, so the block matches the frame it lands in at every player
    /// count.
    fn take_end_trigger_banner_text(&mut self) -> String {
        match self.session.take_end_trigger_banner() {
            None => String::new(),
            Some(trigger) => {
                let state = self.session.state();
                render::end_trigger_banner(
                    state,
                    &trigger,
                    screen::frame_width(state.players.len()),
                )
            }
        }
    }

    /// The load-time notice for a save that was ALREADY triggered when it was
    /// written; empty for every other load and for every fresh game.
    ///
    /// Branched on `final_round` because both save states are reachable and
    /// they tell the player different things: the trigger round still has
    /// turns left in it, or the last lap is already running. The reason words
    /// come from the same [`render::trigger_reason_words`] mapping the banner
    /// and the delta line use.
    ///
    /// The `None` reason arm is unreachable on an engine-produced state (core
    /// refuses `end_game_triggered` without a `trigger_reason`, and `load`
    /// replays through the engine), and it stays silent rather than inventing
    /// a reason for a tampered save.
    fn end_trigger_load_notice(&self) -> String {
        let state = self.session.state();
        if !state.end_game_triggered {
            return String::new();
        }
        let Some(trigger) = state.trigger_reason else { return String::new() };
        let words = render::trigger_reason_words(state, &trigger);
        let dash = crate::style::glyph::SECTION_DASH;
        // Three arms, not the spec's two: a FINISHED game keeps both flags
        // true, so the `final_round` branch alone would greet a completed
        // save with "in progress" (T1 review, finding 1). The reason words
        // stay — why the game ended is what a returning reader wants.
        let line = if is_game_over(state) {
            format!("GAME OVER {dash} {words}")
        } else if state.final_round {
            format!("FINAL ROUND in progress {dash} {words}")
        } else {
            format!("END TRIGGERED {dash} {words}; finishing the current round")
        };
        format!("{}\n", paint(Tone::Warn, &line))
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

    /// Whether the body `screen::wrap` is about to compose is a FULL-SCREEN
    /// surface that replaces the frame instead of sitting under it (spec §0.5).
    ///
    /// One predicate rather than a check at each of the two sites, for the same
    /// reason `screen::frame_active` is one: `wrap` suppresses the stage on
    /// exactly this, and `out`/`out_view` draw the surface on exactly this, so
    /// the two can never disagree about whether a board is on screen.
    pub(crate) fn full_screen_surface(&self) -> bool {
        self.overlay.is_some() && !self.locked
    }

    /// Engage a lockdown: flip the in-memory `locked` flag AND persist the
    /// `reason` into the record (so it survives save/reload -- review §3),
    /// returning any autosave-failure warning to append to the caller's output.
    /// Every site that sets `locked` goes through here so the two never drift.
    pub(crate) fn engage_lockdown(&mut self, reason: String) -> Option<String> {
        self.locked = true;
        self.session.lock_down(reason)
    }

    /// The whole prompt screen for the current state: the DECISION ZONE
    /// (Phase-3.5a T4) — phase chain, "YOU ARE DECIDING" line, the numbered
    /// options page, the dimmed near-miss rows, the standing hint and the input
    /// caret. Options are numbered by their GLOBAL 0-based index into the
    /// DISPLAYED entries, paged by [`decision_render`]'s row budget.
    ///
    /// **This replaced the pre-redesign block wholesale**, which was a
    /// `Turn 1 | Red | Placement | …` state line, an optional `Choose: …`
    /// banner (both retired with `render::prompt_header` itself), one option
    /// per row at twenty per page, and a `> enter a number, …` hint. What it
    /// keeps verbatim is the behaviour underneath: global option numbering, the
    /// `more`/`list` pager, and the bare-Enter forced-single-option ceremony
    /// with its apply/open split.
    pub fn prompt(&self) -> String {
        // A locked session (including one reloaded from a persisted lockdown --
        // review §3) shows only its lockdown status: the numbered options can't
        // be used, so rendering them would mislead. `step` allows just
        // save/log/quit from here.
        if self.locked {
            return self.lockdown_status();
        }
        let state = self.session.state();
        let legal = self.session.legal();
        let rows = self.display_rows(&legal);
        render::decision_zone(
            state,
            self.session.rules(),
            &legal,
            &rows,
            &decision_render::ZoneContext {
                page: self.page,
                expert: self.expert_grammar(),
                game_over: self.session.is_over(),
                recall: crate::input::may_edit(),
            },
        )
    }

    /// The display entries as the zone consumes them: the collapsed families
    /// keep their own summaries, every direct entry carries its `Move` so the
    /// zone can price it and mark it.
    ///
    /// Takes the legal list rather than fetching one, so a single `prompt()`
    /// costs exactly one enumeration where the entries, the `◈` marks and the
    /// near-miss rows all read the same list.
    fn display_rows(&self, legal: &[Move]) -> Vec<render::DisplayRow> {
        let state = self.session.state();
        let rules = self.session.rules();
        Self::group_options(state, legal.to_vec())
            .iter()
            .map(|opt| render::DisplayRow {
                mv: match opt {
                    PromptOption::Direct(mv) => Some(mv.clone()),
                    _ => None,
                },
                words: self.render_option(opt, state, rules),
            })
            .collect()
    }

    /// The expert grammar usable at THIS decision, for the standing hint --
    /// `a`/`b` in `Placement`, `r` in `Movement`, `m <from> <to>` while an
    /// `AssistantMove` choice heads the queue (checked ahead of phase, the same
    /// priority order the deciding line uses). Every other decision has no
    /// shorthand of its own and gets an empty string, which the hint omits
    /// rather than padding around.
    ///
    /// **Alias, not instruction** (spec §0.2): every one of these decisions is
    /// reachable by number from the menu above, and the grammars survive for
    /// the player who already knows them.
    fn expert_grammar(&self) -> &'static str {
        let state = self.session.state();
        if matches!(state.pending_choices.first(), Some(PendingChoice::AssistantMove)) {
            return "'m <from> <to>' moves one directly";
        }
        if !state.pending_choices.is_empty() {
            return "";
        }
        match state.turn_phase {
            TurnPhase::Placement => "'a'/'b' place directly",
            TurnPhase::Movement => "'r' retreats directly",
            TurnPhase::AdditionalActionFirst
            | TurnPhase::AreaAction
            | TurnPhase::FivePowerBonus
            | TurnPhase::Building
            | TurnPhase::Recovery
            | TurnPhase::AdditionalActionSecond
            | TurnPhase::TurnEnd => "",
        }
    }

    /// The one-line trailing hint a read-only VIEW command ends with: the
    /// decision zone's own standing hint, with no options above it.
    ///
    /// A view never changes state, so reprinting the whole zone after every
    /// glance at the board is the noise `out_view` exists to avoid (Task 13
    /// test (e)) -- but the hint has to be the SAME hint, or the player learns
    /// two vocabularies. It is therefore rendered by the zone's own function
    /// over an empty row list, which is what makes "same wording" a property of
    /// the code rather than of two strings kept in step by hand.
    fn input_hint(&self) -> String {
        decision_render::standing_hint(
            &decision_render::ZoneContext {
                page: 0,
                expert: self.expert_grammar(),
                game_over: self.session.is_over(),
                recall: crate::input::may_edit(),
            },
            self.session.state().config.player_count,
        )
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
        // The card index is an OVERLAY: while it is up it takes the input, and
        // whatever was underneath -- a reveal collection, a sub-prompt, the
        // main list -- is untouched and comes back exactly as it was
        // (spec §0.6's return contract). Checked after the lockdown gate for
        // the same reason the sub-prompt branch is: a locked session shows its
        // status and nothing else.
        if self.overlay.is_some() {
            return self.step_overlay(&words);
        }
        // `?` opens the card index from ANY prompt -- the main list, a
        // Movement sub-prompt, or mid-reveal-capture. Intercepted HERE, ahead
        // of both capture branches below, because that is the whole point of
        // §0.6: the index is needed most at the moment a capture is asking
        // which card is in a slot, and both of those loops would otherwise
        // swallow the key (the reveal collector as a bad card, the sub-prompt
        // as "finish or cancel this prompt first").
        if matches!(words.as_slice(), ["?"] | ["index"]) {
            return self.open_index(self.index_target().map_or(IndexKind::Orders, |t| t.kind()));
        }
        // **The full-screen surfaces are reachable from inside a capture too**
        // (spec §0.5: "one action away"), and `sites` is the one that names the
        // requirement — it is the Movement companion, so it has to be reachable
        // from the Movement sub-prompt a route pick puts the player in, which
        // would otherwise swallow the word as a bad route number. Intercepted
        // for exactly the same reason `?` is, one line above, and by the same
        // rule: a WHOLE-LINE match on a surface's own name, which no card id,
        // area id or option number can collide with.
        //
        // **Gated on `!self.locked`, and `log` is why.** A locked session shows
        // its status and nothing else (`out`'s first branch), so setting an
        // overlay here while locked would leave the screen showing the lockdown
        // notice with a view invisibly open underneath taking every subsequent
        // line. `log` is on the lockdown allow-list precisely so a desynced
        // game can still be read back into a bug report, so it falls through to
        // the dispatch arm below, which renders the same text without an
        // overlay. Pinned by `the_log_still_reads_under_a_lockdown`.
        if let [word] = words.as_slice() {
            if !self.locked {
                if let Some(surface) = view_surface(word) {
                    self.overlay = Some(Overlay::View(surface));
                    return self.out(String::new(), false);
                }
            }
            if let Some(kind) = index_command(word) {
                return self.open_index(kind);
            }
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
                    self.out("Reveal collection cancelled — no move was applied.\n".to_string(), false)
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
                    self.pick_option(NUMBER_BASE)
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
            ["score"] => self.out_view(self.score_text()),
            // Reachable only while LOCKED — the interception above takes `log`
            // to its full-screen view at every other time. The postmortem block
            // is what a locked session owes: `save`, `log` and `quit` are all a
            // desynced game does, and the whole point of the third is reading
            // the trail into a report.
            ["log"] => self.out_view(self.log_tail()),
            ["suggest"] => self.suggest_cmd(),
            ["why"] => self.why_cmd(None),
            ["why", token] => match token.parse::<usize>() {
                Ok(n) => self.why_cmd(Some(n)),
                Err(_) => self.out(
                    format!(
                        "'{token}' is not an event number — `why <n>` takes the number `log` \
                         prints beside each line, or `why` on its own for the last bot move.\n"
                    ),
                    false,
                ),
            },
            ["why", ..] => self.out(
                "Usage: 'why' for the last bot move, or 'why <n>' for the log's own event number.\n"
                    .to_string(),
                false,
            ),
            ["view"] | ["views"] => {
                self.overlay = Some(Overlay::Menu);
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
            // `back` is `undo`'s alias, added with the T4 decision zone so the
            // approved mocks' standing hint ("'back' undoes the last applied
            // entry") could be printed without qualification. One command, two
            // spellings -- the hint teaches the word a player about to take
            // something back would actually reach for, and `undo` keeps working
            // for everyone who already knows it.
            ["undo"] | ["back"] => self.undo_cmd(),
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
            // The expert alias names both halves; every shorter form opens the
            // guided menu instead of teaching a usage line (spec §0.2).
            ["reveal", seat_tok, card_tok] => self.reveal_cmd(seat_tok, card_tok),
            ["reveal"] => self.reveal_menu(None),
            ["reveal", seat_tok] => {
                match seat_tok.parse::<u8>().ok().and_then(|n| PlayerId::try_new(n).ok()) {
                    Some(seat) => self.reveal_menu(Some(seat)),
                    None => self.out(format!("'{seat_tok}' is not a valid seat number.\n"), false),
                }
            }
            // The GUIDED form leads, matching `HELP_TEXT`'s order (T8 review,
            // NOTE-6b): a player who has just typed a `reveal` line the driver
            // could not read is the last person to be handed only the form that
            // needs both halves memorized.
            ["reveal", ..] => self.out(
                "Usage: 'reveal' to pick the seat and card from menus, or 'reveal <seat> <O..>' in one line.\n"
                    .to_string(),
                false,
            ),
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
        } else if self.overlay.is_some() {
            // The index is an overlay and takes the whole screen: what it
            // replaces is the PROMPT, not the state underneath it, which is
            // still there when it closes.
            text.push_str(&self.overlay_text());
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
        } else if self.overlay.is_some() {
            // Defense in depth, like the sub-prompt arm below: `step` routes
            // every line to the overlay while it is open, so no view command
            // reaches here with the index up.
            text.push_str(&self.overlay_text());
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
        let grid =
            render::board_grid(self.session.state(), self.session.rules(), &self.session.legal());
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
            "{} turn {} | seed {} | {} logged event(s) — only 'save', 'log', 'quit' available\n{reason}",
            paint(Tone::Warn, "[locked]"),
            self.session.state().turn_number,
            self.session.record().seed,
            self.session.record().events.len(),
        )
    }

    fn advance_page(&mut self) {
        // Paging is over the DISPLAYED entry count (grouped in Movement), and
        // the page COUNT comes from the zone's own walk, so `more` wraps
        // exactly where the zone's footer says it will. It is the zone's
        // because a page is a budget of ROWS there, not of options: how many
        // options fit depends on how wide they are.
        let legal = self.session.legal();
        let rows = self.display_rows(&legal);
        let pages = render::decision_pages(self.session.state(), self.session.rules(), &legal, &rows);
        self.page = (self.page + 1) % pages;
    }

    /// Apply the option the player typed. `shown` is the number ON SCREEN, so
    /// this is the one place the display base is turned back into a position
    /// ([`menu_index`]) -- every caller above speaks the player's numbers and
    /// every caller below speaks the list's.
    fn pick_option(&mut self, shown: usize) -> StepOutput {
        // Index into the DISPLAYED entries: a `Direct` commits as before, a
        // `Route`/`Retreat` entry opens its sub-prompt. `display_options` calls
        // `legal()` exactly once (same as the old direct-index path).
        let options = self.display_options();
        let len = options.len();
        match menu_index(shown, len).and_then(|k| options.into_iter().nth(k)) {
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
            None => self.out(format!("option {shown} out of range ({})\n", range_text(len)), false),
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
        Self::group_options(self.session.state(), self.session.legal())
    }

    /// [`Self::display_options`]'s body, over a legal list the caller already
    /// holds. Split out at T4 so `prompt()` can group the SAME list it hands
    /// the decision zone for the `◈` marks and the near-miss rows — one
    /// enumeration per prompt, and no chance of the entries and the marks
    /// reading two different lists.
    fn group_options(state: &GameState, legal: Vec<Move>) -> Vec<PromptOption> {
        // A pending AssistantMove choice groups its options by SOURCE (Task
        // 6 picker) -- checked ahead of phase, same priority order the
        // deciding line/hint use.
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
            // The collapsed families name their areas in the decision zone's
            // own vocabulary (NAME in caps, id only where a grammar has to type
            // it), so one option list does not mix two ways of naming the same
            // tile: a Route entry and the single-route entry beside it are the
            // same kind of thing and now read like it.
            PromptOption::Route { destination, moves } => format!(
                "Move to {} — {} route options",
                describe::area_name_upper(state, *destination),
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
                    // The `m <from> <to>` grammar types this id.
                    MoveSource::Area(a) => describe::area_label_upper(state, *a),
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
        // D3 (2026-08-08): `m <area> hand` returns the assistant to your
        // hand. `m hand hand` assembles a candidate no legal list ever
        // contains (hand-to-hand is never offered), so it falls through to
        // the same rejection any other unoffered combination gets.
        let to = if dst == "hand" {
            MoveTarget::Hand
        } else {
            match parse_raw_area_token(dst) {
                Ok(id) => MoveTarget::Area(id),
                Err(e) => return self.out(format!("{e}\n"), false),
            }
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
            SubPrompt::Reveal { .. } => self.step_reveal(words),
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
    const VIEW_MENU: [ViewEntry; 14] = [
        ("Port offer — the order cards on display, and the power each zone takes", |r| {
            r.open_view(ViewSurface::Ports)
        }),
        ("Laboratory offer — the technologies on display, with what each costs", |r| {
            r.open_view(ViewSurface::Labs)
        }),
        ("Tracks — every Church and Customs space as a card, with who is on it", |r| {
            r.open_view(ViewSurface::Tracks)
        }),
        ("Building sites — every area's site card: rewards, builders, canal tolls", |r| {
            r.open_view(ViewSurface::Sites)
        }),
        ("Areas — what every area's action pays at each power, and what is on it", |r| {
            r.open_view(ViewSurface::Areas)
        }),
        ("Goals — the achievement cards, claim VP, and per-player progress", |r| {
            r.open_index(IndexKind::Achievements)
        }),
        ("Your hand — the current player's order cards, drawn in full", |r| {
            r.open_view(ViewSurface::Hand)
        }),
        ("Log — the move and reveal trail, each line as it read when it applied", |r| {
            r.open_view(ViewSurface::Log)
        }),
        ("Board — tiles, presidents, assistants, buildings", |r| r.board_cmd()),
        // **"public" was doing work it cannot do** (six-lens leak-fence note).
        // A seat this row hides is never disclosed and says `(hidden)`; a seat
        // it does not hide prints its hand CONTENTS, which are public at the
        // table this driver is describing but are not what "public holdings"
        // brings to mind next to a dashboard that shows the same seats as a
        // bare count. The rule is that the label states which it is, so it
        // does: the fence is in `SeatDisplay`, and the label now points at it
        // rather than claiming the surface never shows a hand.
        ("Players — every seat in full: holdings, and any hand not hidden", |r| {
            let t = render::players_panel(r.session.state(), r.session.rules(), &r.seat_display());
            r.out_view(t)
        }),
        ("Score — current victory points and money", |r| {
            let t = r.score_text();
            r.out_view(t)
        }),
        ("Order card index — all 36 cards, grouped by flag ('?' anywhere)", |r| {
            r.open_index(IndexKind::Orders)
        }),
        ("Technology index — all 20 titles by flag, with what each one does", |r| {
            r.open_index(IndexKind::Techs)
        }),
        ("Site card index — all 24 printed cards and their four spaces", |r| {
            r.open_index(IndexKind::Sites)
        }),
    ];

    /// Open a full-screen view over whatever the player is doing — the one
    /// place an overlay is set from a menu row, so the seven rows that open one
    /// cannot drift into seven spellings of the same two lines.
    fn open_view(&mut self, surface: ViewSurface) -> StepOutput {
        self.overlay = Some(Overlay::View(surface));
        self.out(String::new(), false)
    }

    /// Game-info menu input (one-shot): a valid number closes the menu and
    /// dispatches that row's render fn from [`Self::VIEW_MENU`]; `cancel`
    /// closes it; anything else re-asks. Read-only -- no state, record, or
    /// session field is ever touched from here (`board_cmd`'s once-per-
    /// session width-note flag is the sole, display-only exception).
    fn step_view_menu(&mut self, words: &[&str]) -> StepOutput {
        match words {
            // Bare Enter closes it, exactly as it closes every full-screen
            // surface this menu lists — the return contract the views hold, on
            // the screen a player reaches them from.
            [] | ["cancel"] => {
                self.overlay = None;
                self.out(String::new(), false)
            }
            ["quit"] | ["q"] => StepOutput { text: "bye\n".to_string(), quit: true },
            // **`?` still opens the card index from here** (spec §0.6: from ANY
            // prompt). It used to arrive by falling through `step`'s own `?`
            // intercept, which sits ahead of the sub-prompt branch; promoting
            // this menu to an OVERLAY put it behind that intercept instead, so
            // the key has to be answered here or it stops working on the one
            // screen whose whole job is finding things.
            ["?"] | ["index"] => {
                let kind = self.index_target().map_or(IndexKind::Orders, |t| t.kind());
                self.open_index(kind)
            }
            [tok] if !tok.is_empty() && tok.chars().all(|c| c.is_ascii_digit()) => {
                let Some(&(_, render_fn)) = crate::card_prompt::list_index(tok, Self::VIEW_MENU.len())
                    .and_then(|k| Self::VIEW_MENU.get(k))
                else {
                    return self.out(
                        format!("! pick a number {}, or 'cancel'", range_text(Self::VIEW_MENU.len())),
                        false,
                    );
                };
                self.overlay = None;
                render_fn(self)
            }
            _ => self.out("! pick a numbered view, or 'cancel'".to_string(), false),
        }
    }

    // -----------------------------------------------------------------
    // The card index (spec §0.6, R8)
    // -----------------------------------------------------------------

    /// Open a card index over whatever the player is doing. Nothing else
    /// changes: `pending` and `sub` are left exactly as they were, which is
    /// what makes the return contract structural (see [`Repl::overlay`]).
    fn open_index(&mut self, kind: IndexKind) -> StepOutput {
        self.open_index_filtered(kind, None)
    }

    /// The same, narrowed — what `f silk` typed AT a prompt does (T8).
    ///
    /// The grammar accepts a filter wherever it accepts a card, so a filter
    /// typed with no index up has to mean something; opening the index already
    /// narrowed is the reading that matches the wizard's (`ask_card` prints the
    /// filtered screen there), and it is the only one that does not throw the
    /// player's term away.
    pub(crate) fn open_index_filtered(
        &mut self,
        kind: IndexKind,
        filter: Option<crate::screen_model::IndexFilter>,
    ) -> StepOutput {
        self.overlay = Some(Overlay::Index { kind, filter });
        self.out(String::new(), false)
    }

    /// What the index is being opened to answer, re-derived from the live
    /// capture state on every render rather than captured at open time.
    ///
    /// `None` whenever nothing is being recorded — which is also what makes the
    /// screen a read-only VIEW, with no selection numbers on it (F3 from the
    /// other side: a number is printed only where typing it does something).
    ///
    /// **Two capture states can be open, never both.** A bound reveal
    /// (`pending`) is parked behind a move; the guided `reveal` flow
    /// (`SubPrompt::Reveal`) is a standalone command that only ever starts from
    /// a clean main prompt, where `pending` is `None` by construction. The
    /// bound one is read first so the order is stated rather than assumed.
    ///
    /// **A tech reveal is a card prompt too, and T6 routes it** — the hand-off
    /// T5 named. Until the tech index existed, `?` there opened the ORDER index,
    /// a screen of thirty-six cards that cannot answer "which technology
    /// refilled the Lab"; now it opens the technology index in recording
    /// context, with the same leak fence (`index_placements` supplies the
    /// locations; nothing is derived).
    fn index_target(&self) -> Option<RecordingTarget> {
        if let Some(model) = self.reveal_model() {
            return Some(model.target);
        }
        match &self.sub {
            // Only once a seat is chosen: while the seat MENU is up the question
            // is "whose card", which the order index cannot answer, so numbering
            // its cards would print numbers that mean nothing here (F3).
            SubPrompt::Reveal { seat: Some(seat), .. } => {
                Some(RecordingTarget::StartingOrder { seat: *seat })
            }
            SubPrompt::Reveal { .. }
            | SubPrompt::None
            | SubPrompt::Pick { .. }
            | SubPrompt::Retreat { .. }
            | SubPrompt::PlaceA { .. }
            | SubPrompt::PlaceB { .. } => None,
        }
    }

    /// Where the index may honestly say a card already is.
    ///
    /// **Public information only, and that is a leak fence rather than an
    /// omission.** The port displays are face-up, and the viewing seat's own
    /// hand is already on screen in the dashboard unless privacy masks it — so
    /// both are things this player can see anyway. What is deliberately NOT
    /// here is any other seat's hand: in a live game an opponent's starting
    /// order is hidden, and an index that dimmed it with "in Seat 1's hand"
    /// would leak the one card the whole observation boundary exists to
    /// protect.
    pub(crate) fn index_placements(&self) -> Vec<(CardRef, CardLocation)> {
        let state = self.session.state();
        let mut placed = Vec::new();
        for (board_b, board) in
            [(false, Some(&state.port_state.port_a)), (true, state.port_state.port_b.as_ref())]
        {
            let Some(board) = board else { continue };
            for (slot, card) in board.slots.iter().enumerate() {
                if let Some(id) = card {
                    placed.push((CardRef::Order(*id), CardLocation::PortSlot { board_b, slot }));
                }
            }
        }
        let me = state.current_player;
        if !self.seat_display().hides_hand(me) {
            if let Some(p) = state.players.iter().find(|p| p.id == me) {
                for &id in &p.order_cards_in_hand {
                    placed.push((CardRef::Order(id), CardLocation::OwnHand));
                }
            }
        }
        if let Pending::OrderReveals { collected, .. } = &self.pending {
            for &id in collected {
                placed.push((CardRef::Order(id), CardLocation::ThisReveal));
            }
        }
        // **The technology half of the fence.** The two Lab displays are
        // face-up, and a bought Technology sits face-up in front of its owner
        // (`PlayerState::technologies` is public, unlike a hand) — so both are
        // things everyone at the table can already see. Nothing here reads the
        // tech DECK, which is the hidden set a live game reconciles rather than
        // knows.
        for (board_b, board) in
            [(false, Some(&state.lab_state.lab_a)), (true, state.lab_state.lab_b.as_ref())]
        {
            let Some(board) = board else { continue };
            for (slot, card) in board.slots.iter().enumerate() {
                if let Some(id) = card {
                    placed.push((CardRef::Tech(*id), CardLocation::LabSlot { board_b, slot }));
                }
            }
        }
        for p in &state.players {
            for &id in &p.technologies {
                placed.push((CardRef::Tech(id), CardLocation::OwnedBy { seat: p.id }));
            }
        }
        // Every building-site card in play is face-up on its area from setup.
        for (&area, site) in &state.board_layout.building_sites {
            if let Some(index) = crate::screen_model::printed_site_index(site, self.session.rules())
            {
                placed.push((CardRef::Site(index), CardLocation::OnArea { area }));
            }
        }
        placed
    }

    /// The index model for the current position, registry and filter.
    fn index_model(&self, kind: IndexKind) -> IndexModel {
        let filter = match self.overlay {
            Some(Overlay::Index { filter, .. }) => filter,
            _ => None,
        };
        let placed = self.index_placements();
        // **The target is the live capture's ONLY when the capture is asking
        // about the registry on screen** (T6 review M1). A capture records ONE
        // registry, so `techs` typed mid-Port-reveal used to draw a
        // `Recording: Port A slot …` header over twenty numbered technology
        // cards whose numbers could not answer that question — a screen
        // claiming to be collecting something it cannot collect. Filtered, that
        // screen becomes what it actually is: a read-only view, with no
        // selection numbers on it (F3 from the other side).
        let target = self.index_target().filter(|t| t.kind() == kind);
        // The achievement population is the three cards IN PLAY, with their
        // live claim state and per-seat progress — the wizard's is the whole
        // twelve-card registry, which is the same surface answering a different
        // question (`build_achievement_index`).
        crate::screen_model::build_index(
            kind,
            self.session.rules(),
            &placed,
            &crate::screen_model::build_achievements(self.session.state()).cards,
            target,
            filter,
        )
    }

    pub(crate) fn index_text(&self, kind: IndexKind) -> String {
        crate::index_render::index_view(
            &self.index_model(kind),
            &self.seat_colors(),
            crate::index_render::IndexContext::Overlay,
            screen::index_width(),
        )
    }

    /// The seat color table every card and view renderer takes, in seat order.
    pub(crate) fn seat_colors(&self) -> Vec<PlayerColor> {
        self.session.state().players.iter().map(|p| p.color).collect()
    }

    /// The text of whichever full-screen surface is open.
    pub(crate) fn overlay_text(&self) -> String {
        match &self.overlay {
            Some(Overlay::Index { kind, .. }) => self.index_text(*kind),
            Some(Overlay::View(surface)) => self.view_text(*surface),
            Some(Overlay::Menu) => crate::view_render::view_menu(
                &Self::VIEW_MENU.iter().map(|(label, _)| *label).collect::<Vec<_>>(),
            ),
            None => String::new(),
        }
    }

    /// What a line typed at an open view will land on when the view closes and
    /// re-dispatches it — the one thing [`crate::view_render`] has to be told
    /// (six-lens F1).
    ///
    /// **Re-derived per render rather than captured when the view opened**, on
    /// the rule [`Overlay`]'s own doc states for the index's recording target:
    /// a field remembered at open time can describe a prompt that has since
    /// moved, and there is nothing to gain by remembering one here. Nothing can
    /// change `pending` while a view is up in any case — `step_view` clears the
    /// overlay BEFORE re-dispatching, so no input path runs underneath one.
    ///
    /// `pending` alone is the test: a bound reveal is the state in which a bare
    /// number is a card identity. The guided `reveal` flow is a `SubPrompt`
    /// asking the same kind of question, and it reaches the same branch through
    /// [`Repl::index_target`] — which is why that is what this reads.
    fn view_context(&self) -> crate::view_render::ViewContext {
        match self.index_target() {
            Some(_) => crate::view_render::ViewContext::Capture,
            None => crate::view_render::ViewContext::Decision,
        }
    }

    /// One of spec §0.5's full-screen views, built from the SAME models the
    /// dashboard panels are — so a panel summary and its full surface can never
    /// disagree about a fact.
    fn view_text(&self, surface: ViewSurface) -> String {
        let state = self.session.state();
        let colors = self.seat_colors();
        let context = self.view_context();
        match surface {
            ViewSurface::Tracks => crate::view_render::tracks_view(
                &crate::screen_model::build_tracks(state),
                &colors,
                context,
            ),
            ViewSurface::Sites => crate::view_render::sites_view(
                // No legal list: this surface draws no `◈` marks, and asking
                // for one would cost an enumeration to feed a field nothing
                // here reads.
                &crate::screen_model::build_board(state, self.session.rules(), &[]),
                &colors,
                context,
            ),
            ViewSurface::Ports => crate::view_render::ports_view(
                &crate::screen_model::build_ports(state, self.session.rules()),
                context,
            ),
            ViewSurface::Labs => crate::view_render::labs_view(
                &crate::screen_model::build_labs(state, self.session.rules()),
                context,
            ),
            ViewSurface::Areas => crate::view_render::areas_view(
                // No legal list, for the `sites` view's two reasons: this
                // surface draws no `◈` marks, and an unmarked board is the
                // truthful model of "nothing is offered" rather than a
                // defaulted one.
                &crate::screen_model::build_areas(state, self.session.rules()),
                &colors,
                context,
            ),
            ViewSurface::Hand => self.hand_text(),
            ViewSurface::Log => self.log_text(),
        }
    }

    /// One line of input while a full-screen surface is open.
    fn step_overlay(&mut self, words: &[&str]) -> StepOutput {
        if matches!(words, ["quit"] | ["q"]) {
            return StepOutput { text: "bye\n".to_string(), quit: true };
        }
        match self.overlay {
            Some(Overlay::Index { kind, .. }) => self.step_index(words, kind),
            Some(Overlay::View(surface)) => self.step_view(words, surface),
            Some(Overlay::Menu) => self.step_view_menu(words),
            None => self.out(String::new(), false),
        }
    }

    /// One line of input while a VIEW is open.
    ///
    /// **A different rule from the index's, because a view is a different kind
    /// of screen.** An index is an INPUT surface: its numbers select, so a
    /// mistyped line has to be refused there and leave the screen up. A view
    /// selects nothing — it is a thing to look at — so any command a player
    /// types while looking at one is a command they want RUN, and holding it
    /// hostage behind "press Enter first" would make §0.5's "one action away"
    /// cost two.
    ///
    /// The one input that closes without re-dispatching is the EMPTY line, and
    /// that exclusion is load-bearing rather than tidy: bare Enter at a
    /// single-option prompt applies that option (the ceremony rule), so a blank
    /// line re-dispatched from a view would apply a move the player pressed
    /// Enter to stop looking at a screen.
    ///
    /// **`back` is deliberately NOT one of the close words here, and that is an
    /// asymmetry with the index rather than an oversight** (T6 review S1). At
    /// the index `back` is part of the card-entry grammar and closes the
    /// overlay; here it re-dispatches like any other line, which means it does
    /// what the standing hint under every decision says `back` does — takes the
    /// last applied entry back. Making it a second spelling of `cancel` on this
    /// screen made that hint false at exactly the moment a player is reading it.
    fn step_view(&mut self, words: &[&str], surface: ViewSurface) -> StepOutput {
        self.overlay = None;
        // A reveal capture's prompt is a complete self-describing message its
        // callers pass as `text` (see `out`'s pending branch), so returning to
        // one means passing it — the same return contract the index honors.
        let back = |repl: &Repl| match repl.pending {
            Pending::None => String::new(),
            _ => repl.reveal_prompt(),
        };
        match words {
            [] => {
                let text = back(self);
                self.out(text, false)
            }
            ["cancel"] => {
                let text = back(self);
                self.out(text, false)
            }
            // The key that opened it closes it — the affordance a player
            // reaches for first.
            [word] if *word == surface.command() => {
                let text = back(self);
                self.out(text, false)
            }
            _ => self.step(&words.join(" ")),
        }
    }

    /// One line of input while the index is open.
    ///
    /// Everything goes through `card_prompt::resolve`, which is the same
    /// grammar the prompts underneath use — so a number, an id, `?` and a
    /// filter mean the same thing here as they do there, which is the whole
    /// content of §0.2's "one card-entry grammar everywhere".
    fn step_index(&mut self, words: &[&str], kind: IndexKind) -> StepOutput {
        let line = words.join(" ");
        let model = self.index_model(kind);
        let placed = self.index_placements();
        // The one prompt in the driver whose `placed` list can carry an
        // `OwnedBy` (see `index_placements`), so the one that needs the colour
        // table: a technology already held is refused BY THE PLAYER holding it,
        // in the words the panel and the index use for them.
        let colors = self.seat_colors();
        let prompt = CardPrompt {
            kind,
            candidates: None,
            placed: &placed,
            numbering: card_prompt::Numbering::Selection(&model),
            colors: &colors,
        };
        match card_prompt::resolve(&prompt, &line, self.session.rules()) {
            // `?` toggles: the key that opened the screen closes it, which is
            // the affordance a player reaches for first.
            CardEntry::Cancel | CardEntry::OpenIndex => {
                self.overlay = None;
                // `out` re-draws whatever was underneath -- except a reveal
                // capture, whose prompt is a complete self-describing message
                // its callers pass as `text` (see `out`'s pending branch). So
                // this passes it, which is what returns the player to the exact
                // question they left.
                let back = match self.pending {
                    Pending::None => String::new(),
                    _ => self.reveal_prompt(),
                };
                self.out(back, false)
            }
            CardEntry::Filter(new_filter) => {
                if let Some(Overlay::Index { filter, .. }) = self.overlay.as_mut() {
                    *filter = new_filter;
                }
                self.out(String::new(), false)
            }
            CardEntry::Refused(message) => self.out(message, false),
            CardEntry::Card(card) => self.index_selection(card),
        }
    }

    /// A card picked off the index screen, handed to whatever prompt is
    /// underneath.
    ///
    /// The selection is fed in as the TYPED FORM of the same answer, so the
    /// index adds no authority of its own: the reveal collector's own parser
    /// still checks the card against the deck and the boxed pile, and refuses
    /// it with its own words if it does not belong. What the index bought the
    /// player is finding the card, not skipping the check.
    fn index_selection(&mut self, card: CardRef) -> StepOutput {
        match (card, &self.pending) {
            (CardRef::Order(id), Pending::OrderReveals { .. }) => {
                let echo = card_prompt::echo(CardRef::Order(id), self.session.rules(), &[]);
                self.overlay = None;
                let token = format!("O{:02}", id.0);
                let out = self.collect_reveal(&[&token]);
                StepOutput { text: format!("{echo}{}", out.text), quit: out.quit }
            }
            (CardRef::Tech(id), Pending::TechReveal { .. }) => {
                self.overlay = None;
                let token = format!("T{:02}", id.0);
                let out = self.collect_reveal(&[&token]);
                StepOutput { text: out.text, quit: out.quit }
            }
            // The guided `reveal` flow, once a seat has been picked — the same
            // hand-off, to the other capture (T8). Fed in as the typed form for
            // the same reason: `apply_standalone` still rules on whether this
            // seat can reveal this card, so the index buys finding the card and
            // nothing else.
            //
            // **No echo added here, unlike the arm above**, and the asymmetry is
            // in the two capture paths rather than in this function:
            // `collect_reveal` does not draw the card (a bound reveal never
            // did), while `commit_starting_order` does — so echoing again on
            // this side would print the same card twice on one screen.
            (CardRef::Order(id), Pending::None)
                if matches!(self.sub, SubPrompt::Reveal { seat: Some(_), .. }) =>
            {
                self.overlay = None;
                let token = format!("O{:02}", id.0);
                self.step_reveal(&[&token])
            }
            // Either nothing is being recorded, or the pick is from a registry
            // the prompt underneath is not asking about. Both are inspections
            // rather than answers — but they are DIFFERENT inspections, and one
            // message for both told the second player something false: a
            // collection was live, so "nothing is being recorded" was simply
            // untrue, and it left them with no way to know their pick was
            // merely aimed at the wrong registry (T6 review M1).
            _ => {
                let tag = card_prompt::card_tag(card);
                let message = match self.index_target() {
                    None => format!(
                        "{tag} — nothing is being recorded right now. Enter or 'cancel' closes the index.",
                    ),
                    Some(target) => format!(
                        "{tag} — this capture is asking about {}. 'cancel' closes the index.",
                        describe::registry_name(target.kind()),
                    ),
                };
                self.out(message, false)
            }
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
                        crate::card_prompt::list_index(tok, n)
                            .and_then(|k| moves.get(k).cloned())
                            .ok_or(n)
                    }
                    _ => return self.out(String::new(), false), // unreachable: dispatched as Pick
                };
                match selected {
                    Ok(mv) => {
                        self.sub = SubPrompt::None;
                        self.page = 0;
                        self.commit(mv)
                    }
                    Err(n) => self.out(format!("! pick a number {}, or 'cancel'", range_text(n)), false),
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
                    out.push_str(&format!(
                        "[{}] {}\n",
                        menu_number(start + i),
                        describe::describe_move(mv, state, rules),
                    ));
                }
                // The DECISION ZONE's pager vocabulary, from the zone's own
                // formatter (T5 review S4). This footer used to read `...N more
                // (type 'more')` over `page 2/3` while the main list said `(N
                // more — 'more' for page 2 of 3, 'list' back to the first)`, so
                // a player who had learned one screen's words met different ones
                // on the next -- for a pager that behaves identically. The two
                // wordings were reconciled by hand once; calling
                // `decision_render::pager_footer` is what stops them drifting
                // again.
                if total_pages > 1 {
                    out.push_str(&crate::decision_render::pager_footer(
                        n - end,
                        self.page,
                        total_pages,
                    ));
                    out.push('\n');
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
            SubPrompt::Reveal { seats, seat } => match seat {
                // The card half is the SAME composed screen a bound reveal
                // draws, from the same model and the same renderer — so the two
                // reveal flows in this driver look like one flow, which is the
                // whole content of "one card grammar everywhere" at the level a
                // player actually experiences it.
                Some(seat) => crate::decision_render::reveal_zone(
                    &crate::screen_model::RevealModel {
                        target: RecordingTarget::StartingOrder { seat: *seat },
                        collected: Vec::new(),
                    },
                    state.config.player_count,
                ),
                None => {
                    let mut out = String::from("Whose starting order just became public?\n");
                    for seat in seats.iter() {
                        let color = state
                            .players
                            .iter()
                            .find(|p| p.id == *seat)
                            .map(|p| render::color_name(p.color))
                            .unwrap_or("that seat");
                        // Numbered by the seat's OWN identity, not by its
                        // position in this list (T11 review N6) — see
                        // [`reveal_seat_choice`], which is the other half.
                        out.push_str(&format!("[{}] Seat {} — {color}\n", seat.0, seat.0));
                    }
                    out.push_str("> pick a number, or 'cancel'\n");
                    out
                }
            },
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
        // The re-pump deterministically REPLAYS the unwound line, and if that
        // line crossed the end-game trigger the replay re-arms the banner
        // (`Session::undo` dropped the pre-undo pending one). A replayed flip
        // is still an observed transition, so it speaks HERE — leaving it
        // armed would hand a stale announcement to the next apply's screen.
        let banner = self.take_end_trigger_banner_text();
        if !banner.is_empty() {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&banner);
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
                // AFTER the pump, and drained here rather than per-apply, so
                // ONE screen carries at most one banner however many applies
                // fed it (branch-4 item 5): the human's own move and every bot
                // reply to it route through `Session::apply`, which arms on the
                // transition, and this is the single place their combined
                // output is assembled.
                let banner = self.take_end_trigger_banner_text();
                if !banner.is_empty() {
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                    text.push_str(&banner);
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
                let mut text = format!("{}\n", paint(Tone::Warn, &format!("LOCKDOWN — {msg}")));
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
            let Some(contestant) = self.bots.contestant_at(seat) else { break };
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
            let chosen =
                chooser.choose(self.session.state(), self.session.rules(), seat, contestant, decision_seed);
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
                    let mut text = format!("{}\n", paint(Tone::Warn, &format!("LOCKDOWN — {reason}")));
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
                        out.push_str(&format!("{}\n", paint(Tone::Warn, &format!("LOCKDOWN — {reason}"))));
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
                 playing. (Bot seats are sim-only for the same family of reason — see guide §4.)"
                    .to_string(),
            );
        }
        if self.session.is_over() {
            return self.out_view("The game is over — there is no move to suggest.".to_string());
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

    /// `why`: how a BOT seat chose the move it already played (W5 task 5,
    /// design §2), reconstructed by replaying the record to that position and
    /// re-running the recorded contestant under the pump's own decision key.
    ///
    /// **`&self`, like [`Self::suggest_cmd`] and for the same reason**: the
    /// compiler enforces the sandbox rather than a convention, so this cannot
    /// record an event, move the state or trip an autosave even by accident.
    ///
    /// The two commands split the game between them, which is OD-4(i)'s own
    /// division: `suggest` answers for YOUR seat before you move, `why`
    /// accounts for a BOT's move after it. Each refusal here that has a better
    /// address points at the other one by name.
    fn why_cmd(&self, arg: Option<usize>) -> StepOutput {
        self.out_view(crate::why::why(&self.session, &self.bots, arg))
    }

    /// The seats a `reveal` could legitimately be about: live-tracked players
    /// whose starting order the engine still holds as hidden, in seat order.
    ///
    /// The ENGINE's flag, never a guess of the driver's — `reveal_starting_order`
    /// refuses any other seat, so a menu built from anything else would be
    /// offering rows the authority below it rejects (F2's defect, at a different
    /// prompt).
    fn revealable_seats(&self) -> Vec<PlayerId> {
        self.session
            .state()
            .players
            .iter()
            .filter(|p| p.starting_order_hidden)
            .map(|p| p.id)
            .collect()
    }

    /// `reveal` with nothing to name a card: the GUIDED path (T8, spec §0.2 —
    /// "every decision reachable by numbered menu"). Opens the seat menu, or
    /// jumps straight to the card question when the seat is already named and
    /// only the card is missing.
    ///
    /// Refuses with a reason rather than an empty menu when no seat has a hidden
    /// starting order — which is every simulation and any live game where they
    /// have all been revealed. An empty numbered list is the shape that makes a
    /// player wonder what they typed wrong.
    fn reveal_menu(&mut self, seat: Option<PlayerId>) -> StepOutput {
        // EXISTENCE first, then hiddenness — two facts, and the first one has to
        // be settled before the second is even a question (T8 review, NOTE-6a).
        // `PlayerId::try_new` admits 0-3 whatever the table size, so `reveal 3`
        // at a two-player table reached the hidden-check and came back "Seat 3's
        // starting order is not hidden", which is true of nothing.
        let seated = self.session.state().players.len();
        if let Some(seat) = seat {
            if usize::from(seat.0) >= seated {
                return self.out(no_such_seat(seat.0, seated), false);
            }
        }
        let seats = self.revealable_seats();
        if seats.is_empty() {
            return self.out(
                "No seat has a hidden starting order right now — there is nothing to reveal.\n"
                    .to_string(),
                false,
            );
        }
        if let Some(seat) = seat {
            if !seats.contains(&seat) {
                return self.out(seat_not_hidden(seat), false);
            }
        }
        // A single candidate still shows its menu rather than auto-picking: a
        // reveal writes an event into the record, and choosing a seat on the
        // player's behalf is the one shortcut that could record the wrong one
        // without them ever reading a seat number.
        self.sub = SubPrompt::Reveal { seats, seat };
        self.out(String::new(), false)
    }

    /// One line inside the guided `reveal` flow: a numbered seat pick, then the
    /// card, in the crate's one card grammar.
    fn step_reveal(&mut self, words: &[&str]) -> StepOutput {
        let (seats, seat) = match &self.sub {
            SubPrompt::Reveal { seats, seat } => (seats.clone(), *seat),
            _ => return self.out(String::new(), false), // unreachable: dispatched as Reveal
        };
        match words {
            ["quit"] | ["q"] => return StepOutput { text: "bye\n".to_string(), quit: true },
            ["cancel"] => {
                self.sub = SubPrompt::None;
                return self.out("Reveal cancelled — nothing was recorded.".to_string(), false);
            }
            _ => {}
        }
        let Some(seat) = seat else {
            let seated = self.session.state().players.len();
            let [tok] = words else {
                return self.out("! pick a seat number from the list, or 'cancel'".to_string(), false);
            };
            // The refusal keeps the menu OPEN and re-asks, exactly as the card
            // question below does: a mistyped seat costs one line, not the flow.
            let picked = match reveal_seat_choice(tok, &seats, seated) {
                Ok(seat) => seat,
                Err(refusal) => return self.out(refusal, false),
            };
            self.sub = SubPrompt::Reveal { seats, seat: Some(picked) };
            return self.out(String::new(), false);
        };
        let placed = self.index_placements();
        let prompt = CardPrompt::orders(&placed, card_prompt::Numbering::Ids);
        match card_prompt::resolve(&prompt, &words.join(" "), self.session.rules()) {
            // **The flow closes on SUCCESS ONLY, and that is not tidiness.**
            // The engine is the authority on whether this seat can reveal this
            // card, and it refuses with one line ("Illegal: card not found" —
            // what a card already in somebody's hand gets). Closing the flow
            // there would drop the player back at the main prompt and make them
            // retype `reveal` and re-pick the seat to correct one typo, which is
            // the tool-fighting this phase exists to end. Found by driving the
            // real binary in a PTY, not by reading the code.
            CardEntry::Card(CardRef::Order(card)) => {
                let (text, applied) = self.commit_starting_order(seat, card);
                // Cleared BEFORE the screen is composed, not after: `out`
                // appends whatever prompt is standing, so clearing it second
                // prints the answered question again under its own answer.
                if applied {
                    self.sub = SubPrompt::None;
                }
                self.out(text, false)
            }
            CardEntry::OpenIndex => self.open_index(IndexKind::Orders),
            CardEntry::Filter(filter) => self.open_index_filtered(IndexKind::Orders, filter),
            // Bare Enter and `back` back OUT here, unlike at a bound reveal —
            // and the difference is what is at stake, not a preference. There
            // nothing may be lost silently because a MOVE is parked behind the
            // question; here nothing is parked at all, so leaving costs the
            // player one word to start again.
            CardEntry::Cancel => {
                self.sub = SubPrompt::None;
                self.out("Reveal cancelled — nothing was recorded.".to_string(), false)
            }
            CardEntry::Refused(message) => self.out(message, false),
            CardEntry::Card(other) => self.out(
                format!("{} is not an order card.", card_prompt::card_tag(other)),
                false,
            ),
        }
    }

    /// `reveal <seat> <card>`: make a live-tracked opponent's hidden
    /// starting order public via `Session::apply_standalone`. Unlike the
    /// bound reveals above, this is a normal top-level command usable any
    /// time outside lockdown/collection -- it isn't triggered by a move.
    ///
    /// **Kept verbatim as the EXPERT ALIAS** (spec §0.2: today's grammars
    /// "survive as expert aliases only"), with [`Repl::reveal_menu`] as the path
    /// that needs nothing memorized. Both end at
    /// [`Repl::commit_starting_order`], so the two cannot record different
    /// things.
    fn reveal_cmd(&mut self, seat_tok: &str, card_tok: &str) -> StepOutput {
        let seat = match seat_tok.parse::<u8>().ok().and_then(|n| PlayerId::try_new(n).ok()) {
            Some(p) => p,
            None => return self.out(format!("'{seat_tok}' is not a valid seat number.\n"), false),
        };
        // The card half goes through the ONE card-entry grammar (§0.2/§0.6),
        // so `reveal` takes an id, `?` for the index, and refuses a card that
        // is visibly somewhere else BY NAMING WHERE (F6) -- the same three
        // behaviours every other card prompt has. What it does not take from
        // the grammar is authority: the engine's `apply_standalone` still
        // rules on whether this seat can reveal this card.
        //
        // **The location refusal runs BEFORE the engine, and that ordering is
        // safe by CONSERVATION rather than by luck** (T5 review). A refusal
        // ahead of the authority is only honest if it can never pre-empt a
        // legal answer, and here it cannot: `index_placements` reports a card
        // in a port display slot, in the viewing seat's own visible hand, or
        // already named in the reveal being collected -- and a hidden starting
        // order lives in the deck/removed split, which is DISJOINT from all
        // three. A card the engine would accept as this seat's starting order
        // is therefore never one this check has a location for, so the only
        // entries it turns away are ones `apply_standalone` would refuse
        // anyway -- with a worse message, since the engine cannot say where
        // the operator has already put the card.
        let placed = self.index_placements();
        let prompt = CardPrompt::orders(&placed, card_prompt::Numbering::Ids);
        let card = match card_prompt::resolve(&prompt, card_tok, self.session.rules()) {
            // An ORDER prompt, so the grammar can only ever hand back an order
            // card; the other arms are refused rather than matched loosely, so
            // a future kind cannot slip through as a starting order.
            CardEntry::Card(CardRef::Order(id)) => id,
            CardEntry::OpenIndex => return self.open_index(IndexKind::Orders),
            CardEntry::Refused(message) => return self.out(message, false),
            CardEntry::Card(_) | CardEntry::Cancel | CardEntry::Filter(_) => {
                return self.out("Usage: reveal <seat> <O..>".to_string(), false)
            }
        };
        // The alias is one-shot: it took both halves on one line, so there is no
        // flow to hold open and the success flag has no reader here.
        let (text, _applied) = self.commit_starting_order(seat, card);
        self.out(text, false)
    }

    /// Record one seat's starting order — the shared tail of the `reveal`
    /// alias and the guided menu (T8) — as SCREEN TEXT plus whether it APPLIED.
    ///
    /// **One function so the two paths cannot record different things.** The
    /// event, the echo, the confirmation wording and the delta surfacing are all
    /// here; what differs upstream is only how the seat and the card were named,
    /// and what each does with a refusal (the alias prints it and is done, the
    /// guided flow stays open on the question).
    ///
    /// It returns TEXT rather than a finished `StepOutput` for one reason worth
    /// stating: `out` appends whatever prompt is currently standing, so the
    /// caller has to be able to put the driver into its post-answer state before
    /// the screen is composed. Composing first and tidying after prints the
    /// question again underneath its own answer.
    ///
    /// **The pool check below is the same one the bound capture runs** (T8
    /// review, SHOULD-4). Without it this path went straight to the engine, so
    /// the guided flow could offer a card at the index and then refuse it with
    /// `Illegal: card not found` while the other reveal prompt stated the rule in
    /// words — one rule, two answers, and the worse one on the path a player is
    /// steered onto. It cannot pre-empt a legal answer: `reveal_starting_order`
    /// finds its card through `extract_order_from_deck`, whose search is exactly
    /// the deck and the removed pile [`reveal::check_order_reveal`] admits. The
    /// engine remains the authority — it still rules on the SEAT, on invariants,
    /// and on a finished game — and nothing about the record stream moves: a
    /// refusal here replaces a refusal there, and neither records anything.
    fn commit_starting_order(&mut self, seat: PlayerId, card: OrderCardId) -> (String, bool) {
        if let Err(message) = crate::reveal::check_order_reveal(
            card,
            &self.session.state().order_deck,
            &self.session.state().orders_removed,
        ) {
            return (format!("{message}\n"), false);
        }
        // The card echoed back in full before it commits (§0.6): a reveal is
        // recorded into the game record, and an id typed one digit wrong is
        // exactly the entry a player wants to catch on sight.
        let echo = card_prompt::echo(CardRef::Order(card), self.session.rules(), &[]);
        match self.session.apply_standalone(RecordedEvent::RevealStartingOrder { player: seat, card }) {
            Ok(delta) => {
                // Surface the delta the same way `undo`/`apply` do (item 5,
                // third review): it carries the state diff (the card entering
                // the seat's hand) AND any autosave-failure warning, which this
                // used to discard -- leaving a failed live-tracking autosave
                // invisible on the composed frame (same bug as the `undo` I3
                // fix). `StepOutput.text` is the only channel `main.rs` renders.
                let mut text =
                    format!("{echo}Seat {} revealed O{:02} as their starting order.\n", seat.0, card.0);
                for line in delta {
                    text.push_str(&line);
                    text.push('\n');
                }
                (text, true)
            }
            Err(e) => (format!("Illegal: {e}\n"), false),
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
                None => return self.out("No save path set — use 'save <path>'.\n".to_string(), false),
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

    /// The `hand` full-screen view (T11): the CURRENT player's order cards,
    /// drawn at full size.
    ///
    /// **Per-seat masking, like the panel's.** `hand` shows whoever is to move,
    /// and with bots at the table that is a human at every prompt — but a halted
    /// pump can leave a bot to move, and the bot's cards are not the human's to
    /// read. The mask is resolved HERE and handed to the model, which then
    /// carries no cards at all; the renderer cannot leak what it was never
    /// given.
    fn hand_text(&self) -> String {
        let state = self.session.state();
        let p = state.current_player();
        let seats = self.seat_display();
        let hand = crate::screen_model::build_hand(
            state,
            self.session.rules(),
            p.id,
            seats.hides_hand(p.id),
        );
        match hand {
            Some(hand) => crate::view_render::hand_view(
                &hand,
                crate::board_render::color_of(&self.seat_colors(), p.id),
                seats.is_bot(p.id),
                self.view_context(),
            ),
            // Unreachable: `current_player()` came out of `state.players`.
            // Total anyway, on the rule every emitter in this crate holds.
            None => String::new(),
        }
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
        let start = n.saturating_sub(LOG_DUMP_TAIL);
        let mut out = render::section(&format!("Log ({} of {} event(s))", n - start, n));
        for i in start..n {
            out.push_str(&format!("  {}: {}\n", i, self.session.event_description(i)));
        }
        out
    }

    /// The `log` full-screen view (T11) over the same cached descriptions
    /// [`Self::log_tail`] reads.
    ///
    /// **The two are deliberately different surfaces.** This one is the view a
    /// player opens; `log_tail` is the postmortem block a LOCKDOWN dump embeds,
    /// where there is no frame to replace and the seed and turn number are
    /// printed above it. The fold cache, the tail length and the global indexes
    /// are the same in both, so they can differ in styling and never in fact.
    pub(crate) fn log_text(&self) -> String {
        let n = self.session.record().events.len();
        let start = n.saturating_sub(LOG_VIEW_TAIL);
        let described = (start..n)
            .map(|i| (i, self.session.event_description(i).to_string()))
            .collect::<Vec<_>>();
        crate::view_render::log_view(
            &crate::screen_model::build_log(n, described),
            self.view_context(),
        )
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
///
/// **Labelled `Copper Mine (A00)`, which is the driver's ONE area label**
/// (six-lens note). This wrote `A00 Copper Mine` while Option B — the other
/// half of the same decision, one keystroke away — wrote `Copper Mine (A00)`
/// through [`describe::area_label`], the form every other surface uses: the
/// guided retreat's questions, the routes title, a President's location, the
/// log. Two orders for one fact on two screens of one decision is the sort of
/// difference a player has to stop and parse, and the T11 pass ("every surface
/// speaks the same language") is the rule it was on the wrong side of.
///
/// The column width is MEASURED off the cells rather than set to a literal:
/// the label got longer when it flipped, and a hard-coded pad is how a future
/// long area name silently starts colliding with its neighbour.
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
            "{}{}",
            describe::area_label(state, id),
            if fee { " (fee)" } else { "" }
        ));
    }
    let column = cells.iter().map(|c| c.chars().count()).max().unwrap_or(0) + 2;
    let mut out = String::new();
    for chunk in cells.chunks(3) {
        out.push(' ');
        for c in chunk {
            out.push_str(&format!(" {c:<column$}"));
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
  m <from> <to>   resolve a pending assistant-move choice (<from> is 'hand' or an area id; <to> is an area id, or 'hand' to return the pawn)
  r [id[:n]]...   Movement Option B: retreat the President, recovering all (bare id) or n assistants per area ('r' alone recovers none)
  board           show the area grid and its legend (▲ president, ● assistants, ◈ a Movement destination, [St] Station, site-slot tokens)
  players         show each player's resources, hand, tech, tokens, President
  ports           open the PORT OFFER view: every slot's order card, and the power its zone takes
  labs            open the LABORATORY OFFER view: every slot's technology, what it does, and what it costs
  tracks          open the TRACKS view: every Church and Customs space as a card, with the 2P dummy
  sites           open the BUILDING SITES view: every area's site card, its builders, and its canal tolls
  areas           open the AREAS reference: what every area's action pays at each power, its site, and who is on it
  hand            open YOUR HAND view: the current player's order cards, drawn in full
  orders          open the ORDER CARD INDEX: all 36 cards, grouped by flag, full-screen
  techs           open the TECHNOLOGY INDEX: all 20 titles by flag, each with its full purchase text
  goals           open the ACHIEVEMENTS index: the cards in play by category, with claim VP and progress
  sitecards       open the SITE CARD INDEX: all 24 printed cards, for setup entry
  ?               open the card index for whatever is being asked ('index' also works) — works mid-reveal and mid-sub-prompt, and returns you to it
  score           show current VP/money (or the final scores once the game is over)
  log             open the LOG view: the move and reveal trail, each line as it read when it applied
  view            open the numbered game-info menu over all of the views and indexes above ('views' also works)
  suggest         simulation only: what the built-in bot would play from your seat, with its arithmetic
  why [n]         simulation only: how a bot seat chose its last move (or the log's event n), with the same arithmetic
  privacy         toggle hiding hand contents in views (default: hidden in live mode)
  panel           toggle the players panel inside the composed screen (terminal only)
  screen          toggle the composed screen itself (terminal only; pipes are always plain)
  more            show the next page of legal options (wraps to page 1)
  list            reset to the first page of legal options
  undo            undo the last move (and any reveals bound to it) ('back' also works)
  save [path]     save to path (or to the last-used path if omitted)
  reveal          live mode: pick the seat whose hidden starting order just became public from a numbered menu, then name the card
  reveal <seat> <O..>  the same thing in one line, for anyone who knows both halves
  cancel          back out of whatever is open: either reveal (live), the view menu, a card index, a full-screen view, a numbered pick, a Placement picker, guided retreat
  help            show this text
  quit            exit the REPL ('q' also works)
";

/// How many 20-per-page screens `n` entries need, minimum 1 -- shared by the
/// Movement PICK sub-prompt's "page N/M" footer and its own `more` wraparound,
/// so the footer's M always matches where `more` actually wraps (Codex review
/// I-4 page-count fix).
///
/// **The MAIN list stopped using this at T4** and now pages by ROWS
/// (`decision_render::page_count`), because the decision zone packs short
/// options into columns and "how many options fit" is no longer a constant. The
/// sub-prompt still draws one entry per line, so a fixed twenty per page is
/// still exactly what it means.
fn total_pages_for(n: usize) -> usize {
    n.div_ceil(20).max(1)
}

/// The numbers a list of `n` entries accepts, as a range message says them:
/// `1-10`, or the degenerate forms for one entry and none.
///
/// One helper for every numbered prompt in the driver, which is the point: the
/// three range messages used to spell their own arithmetic (`0-{len-1}`,
/// `0-{VIEW_MENU.len()-1}`, `0..{len}`) and disagreed on the bracket style
/// while agreeing on the base. Now they cannot disagree on either.
fn range_text(n: usize) -> String {
    match menu_range(n) {
        None => "nothing to pick here".to_string(),
        Some((lo, hi)) if lo == hi => format!("{lo}"),
        Some((lo, hi)) => format!("{lo}-{hi}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;
    use crate::style::test_support::assert_no_ascii_prose_dash;
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

    /// The option NUMBER attached to `needle` on a rendered prompt: the `[k]`
    /// immediately before it.
    ///
    /// Scans backwards from the needle rather than taking a line's first
    /// bracket, because the T4 zone packs a long option list into COLUMNS, so
    /// one line can carry two or three entries and "the number at the start of
    /// the line the text is on" stops being that text's number.
    fn option_number(prompt: &str, needle: &str) -> usize {
        // Scanned forward over the `[k]` markers rather than backward from the
        // needle: the deciding line names things like "Option B" too (it says
        // what the phase is asking), and searching backward from the first
        // occurrence would land in a sentence with no option number in front
        // of it at all.
        let mut rest = prompt;
        while let Some(open) = rest.find('[') {
            let close = match rest[open..].find(']') {
                Some(c) => open + c,
                None => break,
            };
            let cell_end = rest[close..]
                .find('[')
                .map(|next| close + next)
                .unwrap_or(rest.len());
            if rest[close..cell_end].contains(needle) {
                if let Ok(k) = rest[open + 1..close].parse() {
                    return k;
                }
            }
            rest = &rest[close + 1..];
        }
        panic!("no numbered option carrying {needle:?} in:\n{prompt}")
    }

    /// The DISPLAY NUMBER (what a numbered answer addresses) of the first
    /// grouped entry whose rendered text contains `needle` -- fixtures pick
    /// by text now that grouping means raw legal-list positions no longer
    /// address the prompt (Task 6).
    ///
    /// A NUMBER since T5, not a position: the driver's lists are 1-based
    /// (`screen_model::NUMBER_BASE`), so a fixture that typed a position would
    /// pick the entry before the one it found.
    fn display_number_containing(r: &Repl, needle: &str) -> Option<usize> {
        let state = r.session.state();
        let rules = r.session.rules();
        r.display_options()
            .iter()
            .position(|o| r.render_option(o, state, rules).contains(needle))
            .map(menu_number)
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
    /// still (198/175/200 since the T10 review). Guide §2 states the frame widths; making
    /// the note frame-aware is on the carry-forward list.
    #[test]
    fn board_command_prints_a_one_time_width_note_on_a_wide_board() {
        let mut r = repl_np(4, 7);
        let width =
            render::board_grid(r.session.state(), r.session.rules(), &[]).lines().next().map(|l| l.chars().count()).unwrap_or(0);
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
    /// reason `screen::FRAME_HEIGHTS` records in full: T3 stacked the whole
    /// designed dashboard below the board at 3 and 4 players, which put those
    /// two opening frames at 79 and 87 rows until T10 re-laid them into the 4P
    /// mock's seat columns and brought them back to 49 and 48. What this test
    /// was ever really checking is that
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
            let menu = r.overlay_text();
            // A NUMBERED row, not merely a line mentioning the word: the
            // menu's own gloss names Board too (it is one of the three rows
            // that redraw the frame instead of opening a surface), and a
            // first-match scan finds the title.
            let board_idx = menu
                .lines()
                .map(|l| l.trim())
                .filter(|l| l.starts_with('['))
                .find(|l| l.contains("Board"))
                .and_then(|l| l.strip_prefix('[')?.split(']').next()?.parse::<usize>().ok())
                .expect("the Board entry carries its number");
            let out = r.step(&board_idx.to_string());
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
        assert!(p.contains("[1]"), "{p}");
        assert!(p.contains("End additional-action phase"), "{p}");
    }

    /// The T4 cutover: the prompt IS the decision zone. Its four parts are all
    /// there, in order, and the pre-redesign block's own lines are gone —
    /// checked by absence as well as presence, since a zone drawn UNDER the old
    /// header would satisfy every "contains" assertion above.
    #[test]
    fn the_prompt_is_the_decision_zone_and_the_old_block_is_gone() {
        let r = repl_2p(4);
        let p = r.prompt();

        let chain = p.find("TURN 1 ").expect("the phase chain leads");
        let deciding = p.find("YOU ARE DECIDING").expect("the deciding line");
        let option = p.find("[1]").expect("the numbered options");
        let hint = p.find("'help' for commands").expect("the standing hint");
        assert!(chain < deciding && deciding < option && option < hint, "zone order:\n{p}");
        assert!(p.ends_with("> \n"), "the caret closes the zone: {p:?}");
        assert!(p.contains(" ADDITIONAL "), "the current phase is lit: {p}");

        // The retired block.
        assert!(!p.contains("Turn 1 | Red |"), "the old state header must be gone:\n{p}");
        assert!(!p.contains("> enter a number"), "the old hint must be gone:\n{p}");
        assert!(!p.contains("Choose:"), "the old choice banner must be gone:\n{p}");
    }

    /// The ◈ feed (spec §0.7): the marks the BOARD draws during Movement are
    /// exactly the destinations the ENGINE's list offers — checked by taking
    /// the marks off the rendered board and the destinations off `legal()`,
    /// with nothing in between deriving either.
    #[test]
    fn the_board_marks_exactly_the_engines_movement_destinations() {
        let r = movement_grouping_fixture();
        let legal = r.session.legal();
        let offered = crate::screen_model::president_destinations(&legal);
        assert!(!offered.is_empty(), "the fixture must offer destinations");

        // Tiles sit side by side on a line, so a mark is read against the id
        // it FOLLOWS: split each title line on its `A<nn> ` labels and keep the
        // ids whose own segment carries the glyph.
        let grid = render::board_grid(r.session.state(), r.session.rules(), &legal);
        let mut got: Vec<u8> = Vec::new();
        for line in grid.lines() {
            let mut id: Option<u8> = None;
            let mut carries = false;
            for tok in line.split_whitespace() {
                if let Some(n) = tok.strip_prefix('A').and_then(|n| n.parse::<u8>().ok()) {
                    if carries {
                        got.extend(id);
                    }
                    id = Some(n);
                    carries = false;
                } else if tok.contains(crate::style::glyph::REACHABLE) {
                    carries = true;
                }
            }
            if carries {
                got.extend(id);
            }
        }
        let mut expected: Vec<u8> = offered.iter().map(|a| a.0).collect();
        expected.sort_unstable();
        got.sort_unstable();
        assert_eq!(got, expected, "board marks vs engine destinations:\n{grid}");

        // ...and OUTSIDE a Movement decision the same board draws none, because
        // the engine offers no President move rather than because the CLI
        // decided the phase was wrong.
        let opening = repl_2p(4);
        let plain = render::board_grid(
            opening.session.state(),
            opening.session.rules(),
            &opening.session.legal(),
        );
        assert!(!plain.contains(crate::style::glyph::REACHABLE), "{plain}");
    }

    /// The dimmed near-miss rows reach the screen, and they only ever name an
    /// area the engine did NOT offer.
    #[test]
    fn the_zone_prints_the_near_miss_rows_for_unoffered_neighbours() {
        let r = movement_grouping_fixture_with(0);
        let p = r.prompt();
        let rows: Vec<&str> = p.lines().filter(|l| l.contains("(not offered:")).collect();
        assert!(!rows.is_empty(), "the fixture must have a near miss:\n{p}");
        let offered = crate::screen_model::president_destinations(&r.session.legal());
        for row in &rows {
            for area in &offered {
                let name = crate::describe::area_name_upper(r.session.state(), *area);
                assert!(!row.contains(&name), "an OFFERED area must never be a near miss: {row}");
            }
        }
    }

    /// `back` is `undo`'s alias, which is what makes the standing hint's
    /// "'back' undoes the last applied entry" a true sentence rather than a
    /// promise about a command that does not exist.
    #[test]
    fn back_is_undo() {
        let mut r = repl_2p(4);
        r.step("1");
        let after_move = r.session.record().events.len();
        assert_eq!(after_move, 1, "one applied entry to take back");

        let out = r.step("back");
        assert_eq!(r.session.record().events.len(), 0, "'back' took it back: {}", out.text);
        assert_eq!(r.session.state().turn_phase, TurnPhase::AdditionalActionFirst);
        // Same command, same refusal when there is nothing left.
        assert!(r.step("back").text.contains("Cannot undo"), "{}", r.step("back").text);
        assert!(HELP_TEXT.contains("'back' also works"), "and it is documented");
    }

    /// **Every line of `help` fits the design floor** (six-lens F7).
    ///
    /// The `cancel` row had grown to 209 visible columns — the ONE line the
    /// whole driver emitted over 200 — so `help` on the 200-column window the
    /// guide tells testers to size sheared mid-word ("…either Placement picker,
    /// guide" / "d retreat"). Nothing covered it: `screen`'s width guard
    /// measures composed FRAMES (`worst_case_frames`), and this text is not one
    /// — it is a `out_view` body — while `guide_sync` checks HELP_TEXT for
    /// command coverage and §5 parity but never for width.
    ///
    /// Written over EVERY line rather than over the one that was long, so the
    /// class is closed: the next row somebody extends is measured on the way in.
    /// The widest is reported on failure so a deliberate rewrap has the number
    /// it needs.
    #[test]
    fn every_help_line_fits_the_design_floor() {
        let over: Vec<(usize, &str)> = HELP_TEXT
            .lines()
            .map(|l| (crate::screen::visible_width(l), l))
            .filter(|(w, _)| *w > crate::screen::FLOOR_W)
            .collect();
        assert!(
            over.is_empty(),
            "help rows past the {}-column design floor — a terminal at the floor shears them \
             mid-word: {over:#?}",
            crate::screen::FLOOR_W,
        );
        // Anti-vacuity: the text is really being measured, and the assertion
        // has something to bite on rather than a list of empty strings.
        let widest = HELP_TEXT.lines().map(crate::screen::visible_width).max().expect("rows");
        assert!(widest > 100, "the rows are long enough for this to be a real check: {widest}");
    }

    /// Ceremony reduction (Task 6): bare Enter on a forced single-option
    /// prompt applies that option through the normal commit path -- one real
    /// recorded event, phase advanced -- and the prompt advertised it.
    #[test]
    fn bare_enter_applies_a_forced_single_option() {
        let mut r = repl_2p(4);
        let p = r.prompt();
        assert!(p.contains("only option — press Enter to apply"), "{p}");
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
            p.contains("only option — press Enter to open"),
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
        let out = r.step("1");
        assert!(!out.quit);
        assert_ne!(r.session.state().turn_phase, phase_before);
        assert!(out.text.contains("Placement"), "{}", out.text); // delta line + new prompt
    }

    #[test]
    fn placement_grammar_matches_by_multiset() {
        let mut r = repl_2p(4);
        r.step("1"); // -> Placement
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
        r.step("1"); // -> Placement
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
        r.step("1");
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
        r.step("1"); // one real recorded event to undo
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
    /// on all of them.
    ///
    /// The skipped rows are the ones with their OWN layout rather than the
    /// rule-header one: the board grid, and every FULL-SCREEN surface (the four
    /// card indexes since T6, and the two views), each drawn to its own approved
    /// mock and pinned by `index_render`/`view_render`'s own tests. Skipping by
    /// a startswith on the label is deliberate -- a row renamed out of the skip
    /// list rejoins the sweep rather than silently staying out of it.
    ///
    /// **The complement is asserted too**, because a skip list that grew to
    /// cover everything would leave this test passing over an empty sweep: the
    /// rule-header views are counted, and there must still be some.
    #[test]
    fn all_state_views_share_the_rule_header_style() {
        // TWO families since T11, and the partition is the point rather than an
        // exception list. A FULL-SCREEN surface replaces the frame and opens
        // with `NAME — full view   <gloss>`; the two that still render UNDER the
        // frame open with a `─ Title ─` rule. Every menu row is one or the
        // other, both families are non-empty, and no row is both — which is a
        // stronger statement than the old skip-list, and it cannot go stale by
        // somebody forgetting to add a name to it.
        let mut r = repl_2p(4);
        // Spend the board's once-per-session width note first: it prepends a
        // line to the FIRST `board` render only, and a sweep that met it would
        // be asserting about a notice rather than about a view's own opening.
        r.board_cmd();
        let (mut full_screen, mut in_frame) = (Vec::new(), Vec::new());
        for (label, render_fn) in Repl::VIEW_MENU.iter() {
            let out = render_fn(&mut r);
            let first = out.text.lines().next().unwrap_or("").to_string();
            // A full-screen surface leaves the overlay OPEN and puts its text in
            // `overlay_text`, so ask the screen the player is looking at.
            let shown = if r.full_screen_surface() { r.overlay_text() } else { out.text.clone() };
            let opening = shown.lines().next().unwrap_or("").to_string();
            let rule = opening.starts_with("─ ");
            let full = opening.contains(" — full view   ")
                || opening.contains("INDEX")
                || opening.contains("ACHIEVEMENTS");
            // `board` is its OWN opening and always has been: it draws the
            // board grid straight away, because a picture of the board titled
            // "BOARD" is a caption on a thing that needs none. Named here
            // rather than skipped, so the exception is one line of code with a
            // reason on it instead of an entry in a list of layout exemptions.
            let is_board = opening.starts_with(&format!("{}", crate::style::glyph::BOX_TL));
            assert_eq!(
                u8::from(rule) + u8::from(full) + u8::from(is_board),
                1,
                "view '{label}' must open in exactly ONE recognized family, got: {opening:?} \
                 (raw first line {first:?})",
            );
            if full {
                full_screen.push(*label);
            } else if rule {
                in_frame.push(*label);
            }
            r.overlay = None;
        }
        assert!(full_screen.len() >= 9, "most surfaces are full-screen now: {full_screen:?}");
        assert!(
            !in_frame.is_empty(),
            "the rule-header family must still be swept by something: {in_frame:?}",
        );
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
        r.step("1");
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
    ///
    /// **The list shrank at T6 and the reason is not that the property weakened.**
    /// `tracks`, `sites` and `goals` became FULL-SCREEN surfaces (spec §0.5/§0.6),
    /// which do not append a hint at all -- they replace the prompt and carry
    /// their own "how to leave" line. Their property is the return contract, and
    /// it is pinned by `a_view_returns_to_the_prompt_it_interrupted`.
    #[test]
    fn view_commands_show_only_the_input_hint_not_the_full_options_block() {
        let mut r = repl_2p(4);
        // The commands that still render UNDER the frame get the one-line hint
        // and never the options block.
        for cmd in ["board", "players", "score", "help"] {
            let out = r.step(cmd);
            assert!(!out.text.contains("[1]"), "'{cmd}' unexpectedly showed the numbered options block: {}", out.text);
            assert!(
                out.text.trim_end().ends_with("'help' for commands)"),
                "'{cmd}' output is missing the one-line input hint: {}", out.text
            );
        }
        // The four T11 promoted to FULL-SCREEN views show no hint and no
        // options at all — they replace the prompt, and their own exit clause
        // is what tells the player how to get back to it.
        for cmd in ["ports", "labs", "hand", "log"] {
            let out = r.step(cmd);
            assert!(r.full_screen_surface(), "'{cmd}' opens a full-screen view");
            assert!(
                !out.text.contains("'help' for commands)"),
                "'{cmd}' is a view, not a framed body: {}", out.text,
            );
            let shown = r.overlay_text();
            assert!(shown.contains(" — full view   "), "'{cmd}': {shown}");
            assert!(
                shown.contains("closes this view and puts you back at the prompt underneath"),
                "'{cmd}' states its own way out: {shown}",
            );
            r.step("cancel");
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

    /// **One dash, everywhere the player reads prose** (T11's editorial rule).
    ///
    /// The redesigned surface has spoken in em dashes since T2 — every title,
    /// every section rule, every option row — while the driver's MESSAGES were
    /// still writing the pre-redesign `--`, so the same sentence could arrive
    /// in two typographies depending on which layer produced it. The rule now
    /// held: a prose dash in user-visible text is `—`, and the piped protocol
    /// carries it exactly as it carries `·` and `★` (it is escape-free, never
    /// ASCII-only).
    ///
    /// **What is deliberately NOT covered is DECORATION**: the setup wizard's
    /// `--- Board check ---` banners and its `-- row 1 of 4 … --` markers are
    /// horizontal RULES, a different typographic object from a dash, and they
    /// belong to a rolling-text surface the redesign left as it found it.
    ///
    /// Swept over real driver output rather than over the source, so it is a
    /// property of what a player sees.
    ///
    /// **A WALK ONLY REACHES WHAT A PLAYABLE SESSION SHOWS**, which the T11
    /// review demonstrated by injecting a `--` into the lockdown refusal and
    /// watching the suite stay green (F5). The three surfaces below sit outside
    /// any walk by construction — a locked session refuses the loop, a load
    /// warning is produced before the loop starts, and `suggest` refuses only
    /// in live mode — so each is reached through its own producer and checked
    /// with the same rule. They are named, not folded into one blob, because a
    /// fixture that silently stops producing text is the failure mode this
    /// whole test exists to avoid; [`assert_no_ascii_prose_dash`] refuses an
    /// empty one.
    #[test]
    fn no_driver_message_writes_a_prose_dash_in_ascii() {
        let mut r = repl_2p(4);
        let mut seen = String::new();
        // A walk that reaches the messages, the views, the menus, the hints and
        // a refusal — the surfaces a player meets in the first minute.
        for line in [
            "", "help", "view", "cancel", "ports", "cancel", "labs", "cancel", "hand", "cancel",
            "log", "cancel", "tracks", "cancel", "sites", "cancel", "score", "board", "players",
            "privacy", "panel", "screen", "m", "r nonsense", "reveal 9", "zzz", "1", "1",
        ] {
            let out = r.step(line);
            seen.push_str(&out.text);
            seen.push_str(&r.overlay_text());
            seen.push_str(&r.prompt());
        }
        assert!(seen.len() > 20_000, "the walk must actually produce output: {}", seen.len());
        assert!(seen.contains('—'), "anti-vacuity: the surface does use em dashes");
        assert_no_ascii_prose_dash("the opening walk", &seen);

        // (1) The LOCKDOWN refusal — the line the review's injected `--` sat
        // on. A locked session answers everything but save/log/quit with it,
        // so no walk over a playable session can ever print it.
        let mut locked = repl_2p(4);
        locked.locked = true;
        assert_no_ascii_prose_dash("the lockdown refusal", &locked.step("board").text);

        // (2) A LOAD WARNING — produced by `Repl::new` before any step runs,
        // and shown by `main` above the opening frame rather than returned from
        // the loop. The live-record-names-a-bot demotion is the one this
        // driver can construct without a file on disk.
        let bots = BotSeats::new(vec![None, Some(Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT))], 3);
        let mut session = Session::new_live(
            GameConfig::for_player_count(2),
            21,
            SetupObservations::default(),
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        )
        .unwrap();
        session.set_run_seats(bots.seat_provenance());
        let warned = Repl::new(session);
        assert_no_ascii_prose_dash("a load warning", &warned.session.load_warnings.join("\n"));

        // (3) The `suggest` LIVE-MODE refusal, which a simulation session
        // structurally cannot reach.
        let (mut live, _) = repl_live_hidden(21);
        let refusal = live.step("suggest");
        assert!(refusal.text.contains("simulation-only"), "the refusal fired: {}", refusal.text);
        assert_no_ascii_prose_dash("the suggest live-mode refusal", &refusal.text);
    }

    /// **The `view` MENU is a full-screen overlay, like every surface it
    /// lists** (T11, found by the Forge pass).
    ///
    /// As a sub-prompt it rendered UNDER the composed frame, so a menu titled
    /// `— full view` arrived twenty rows beneath a forty-six-row board and
    /// pushed the frame off the top of the terminal to fit. Now it replaces the
    /// frame, `?` still reaches the card index from it (§0.6's "from ANY
    /// prompt"), a number opens its row, and Enter or `cancel` closes it with
    /// nothing entered.
    #[test]
    fn the_view_menu_replaces_the_frame_and_keeps_its_two_keys() {
        let mut r = repl_2p(4);
        r.step("view");
        assert!(r.full_screen_surface(), "the menu replaces the frame");
        screen::force(true);
        let framed = screen::wrap(&r, "");
        screen::force(false);
        assert!(
            !framed.contains("PLAYERS"),
            "no dashboard is drawn under a full-screen surface: {framed:.400}",
        );
        assert!(r.overlay_text().contains("GAME INFO"), "{}", r.overlay_text());

        // `?` reaches the card index from here — the key's promise is "any
        // prompt", and this is the screen a player uses to find things.
        let out = r.step("?");
        assert!(out.text.contains("ORDER CARD INDEX"), "{}", out.text);
        r.step("cancel");

        // A number opens its row; Enter and `cancel` close with nothing done.
        for closer in ["", "cancel"] {
            r.step("view");
            assert!(r.overlay.is_some());
            r.step(closer);
            assert!(r.overlay.is_none(), "{closer:?} closes the menu");
            assert!(r.session.record().events.is_empty(), "and applies nothing");
        }
        r.step("view");
        let row = Repl::VIEW_MENU
            .iter()
            .position(|(label, _)| label.starts_with("Log"))
            .expect("the roster carries the log");
        let out = r.step(&crate::screen_model::menu_number(row).to_string());
        assert!(out.text.is_empty() || !out.text.contains("GAME INFO"), "the menu closed");
        assert!(r.overlay_text().contains("LOG — full view"), "{}", r.overlay_text());
    }

    /// **`log` still reads under a lockdown, and it is the ONE view that
    /// does** (T11).
    ///
    /// A locked session accepts `save`, `log` and `quit` and shows its status
    /// and nothing else — so the full-screen interception is gated on
    /// `!self.locked`, and `log` falls through to the postmortem block instead.
    /// Without that gate the word would set an overlay that `out` never draws,
    /// leaving a desynced game showing its lockdown notice with an invisible
    /// view underneath eating every line the operator typed next — at exactly
    /// the moment the whole point is reading the trail into a bug report.
    #[test]
    fn the_log_still_reads_under_a_lockdown() {
        let mut r = repl_2p(4);
        r.step("1");
        let applied = r.session.record().events.len();
        assert!(applied > 0, "fixture: something is in the log to read back");
        r.engage_lockdown("a synthetic fault".to_string());

        let out = r.step("log");
        assert!(out.text.contains(&format!("of {applied} event(s)")), "the trail is there: {}", out.text);
        assert!(r.overlay.is_none(), "and no invisible overlay was opened: the word fell through");
        assert!(out.text.contains("[locked]") || out.text.contains("locked"), "{}", out.text);

        // The other views stay refused by the allow-list, which is the rule the
        // exception is an exception TO.
        for cmd in ["ports", "labs", "hand", "tracks", "sites"] {
            let out = r.step(cmd);
            assert!(
                out.text.contains("session locked"),
                "'{cmd}' must stay refused while locked: {}", out.text,
            );
            assert!(r.overlay.is_none(), "'{cmd}' must not open an overlay while locked");
        }
    }

    /// **The postmortem dump carries the LAST [`LOG_DUMP_TAIL`] entries**, and
    /// says how many of how many it is showing (T11 review N1).
    ///
    /// The two tails are deliberately different lengths — the view is sized by
    /// the design floor, the dump by what a bug report can carry without being
    /// scrolled past — and only the view's had a pin. An unpinned constant on
    /// the direction that matters here is one nobody notices going wrong: a
    /// dump that quietly showed the FIRST ten would look identical in shape and
    /// be useless for exactly the fault it exists to describe.
    #[test]
    fn the_postmortem_dump_carries_the_last_ten_entries_and_says_so() {
        let mut r = repl_2p(4);
        for _ in 0..(LOG_DUMP_TAIL + 5) {
            step_one_move(&mut r);
        }
        let total = r.session.record().events.len();
        assert!(total > LOG_DUMP_TAIL, "fixture: more events than the dump shows ({total})");
        r.engage_lockdown("a synthetic fault".to_string());

        let dump = r.step("log").text;
        assert!(
            dump.contains(&format!("Log ({LOG_DUMP_TAIL} of {total} event(s))")),
            "the dump states its own window: {dump}",
        );
        // The LAST ten, by their global indexes — the direction the whole
        // surface is for.
        for i in (total - LOG_DUMP_TAIL)..total {
            assert!(dump.contains(&format!("  {i}: ")), "entry {i} is in the dump: {dump}");
        }
        assert!(
            !dump.contains(&format!("  {}: ", total - LOG_DUMP_TAIL - 1)),
            "and the entry just before the window is not: {dump}",
        );
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
        assert!(p0.contains("more — 'more' for page 2 of "), "{p0}");
        let p1 = r.step("more").text;
        assert_ne!(p0, p1);
        // ...and `more` from the last page wraps to the first, as HELP_TEXT says.
        // The LAST page words that wrap rather than counting a remainder it does
        // not have (T4 review S1): it used to print "(0 more — ...)".
        let mut pages = 1;
        while !r.prompt().contains("(last page — 'more' wraps to page 1 of ") {
            assert!(
                !r.prompt().contains("(0 more"),
                "a page with nothing left must not word itself as a count: {}",
                r.prompt(),
            );
            r.step("more");
            pages += 1;
            assert!(pages < 20, "the pager must terminate");
        }
        assert_eq!(r.step("more").text, p0, "the wrap lands back on page 1");
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
        assert!(p.contains("'help' for commands"), "the standing hint is always there: {p}");
        assert!(!p.contains("'a'/'b'"), "{p}");
        assert!(!p.contains("'r' retreats"), "{p}");

        r.step("1"); // -> Placement
        let p = r.prompt();
        assert!(p.contains("'a'/'b' place directly"), "{p}");
        assert!(!p.contains("'r' retreats"), "{p}");

        r.step("a"); // place nothing -> Movement
        assert_eq!(r.session.state().turn_phase, TurnPhase::Movement, "fixture assumption drifted");
        let p = r.prompt();
        assert!(p.contains("'r' retreats directly"), "{p}");
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
        assert!(p.contains("'help' for commands"), "the standing hint is always there: {p}");
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
        let held: Vec<String> = r
            .session
            .state()
            .current_player()
            .order_cards_in_hand
            .iter()
            .map(|id| format!("O{:02}", id.0))
            .collect();
        assert!(!held.is_empty(), "fixture: the seat holds a card to hide");

        // `hand` is a full-screen VIEW since T11, so the cards are on the
        // overlay the player is looking at rather than in the step's own text.
        r.step("hand");
        let open = r.overlay_text();
        for id in &held {
            assert!(open.contains(id), "sim shows the cards by default: {open}");
        }
        r.step("cancel");

        let out = r.step("privacy");
        assert!(out.text.contains("HIDDEN"), "{}", out.text);
        r.step("hand");
        let masked = r.overlay_text();
        assert!(masked.contains("contents hidden"), "{masked}");
        assert!(masked.contains("1 card"), "count survives: {masked}");
        for id in &held {
            assert!(!masked.contains(id), "no card identity may leak: {masked}");
        }
        r.step("cancel");

        r.step("privacy");
        r.step("hand");
        let reopened = r.overlay_text();
        for id in &held {
            assert!(reopened.contains(id), "toggle back restores contents: {reopened}");
        }
    }

    /// Owner scope item 10 (discoverability): `view` opens a one-shot
    /// numbered game-info menu naming every public-state surface; picking a
    /// number renders that surface and drops back to the normal prompt.
    #[test]
    fn view_menu_lists_surfaces_and_renders_the_picked_one() {
        let mut r = repl_2p(4);
        let out = r.step("view");
        assert!(out.text.contains("GAME INFO"), "{}", out.text);
        // Every roster row is named, read off the roster itself — a menu that
        // grew a surface and did not print it is the defect, and a hand-typed
        // list here could only ever catch the ones somebody remembered.
        for (label, _) in Repl::VIEW_MENU.iter() {
            assert!(out.text.contains(label), "menu must name {label}: {}", out.text);
        }
        // The spec's §0.5 roster, by the WORD a player types for each — the
        // half a label list cannot check, since a label is prose and a command
        // is a promise.
        for word in ["Port offer", "Laboratory offer", "Tracks", "Building sites", "Goals", "hand", "Log"] {
            assert!(out.text.contains(word), "the §0.5 roster must name {word}: {}", out.text);
        }
        assert!(out.text.contains("'cancel'"), "{}", out.text);

        // Pick the Goals entry by its number parsed from the menu itself.
        let goals_idx = out
            .text
            .lines()
            .find(|l| l.contains("Goals"))
            .and_then(|l| l.trim_start().strip_prefix('[')?.split(']').next()?.parse::<usize>().ok())
            .expect("Goals entry carries its number");
        let shown = r.step(&goals_idx.to_string());
        // Goals is the ACHIEVEMENT INDEX since T6 -- the same three cards with
        // claim state and per-seat progress, one card each.
        assert!(
            shown.text.contains("ACHIEVEMENTS") && shown.text.contains("CATEGORY A"),
            "picking Goals must open the achievement index: {}",
            shown.text,
        );
        assert!(matches!(r.sub, SubPrompt::None), "the menu is one-shot");
        assert!(r.session.record().events.is_empty(), "views never apply moves");

        // Back at the normal prompt: a move command works again.
        r.step("");
        let p = r.prompt();
        assert!(p.contains("[1]"), "{p}");
    }

    /// `views` aliases the menu; `cancel` closes it; an out-of-range number
    /// re-asks with the range it accepts.
    ///
    /// **`?` is NOT among the aliases since T5**, and that is the approved
    /// mock's own split: its standing hint reads `'view' any surface · '?' the
    /// card index`. The two keys now open two different screens, which is why
    /// this test names only one of them and `the_index_opens_from_every_prompt`
    /// pins the other.
    #[test]
    fn view_menu_aliases_cancel_and_range() {
        let mut r = repl_2p(4);
        for alias in ["view", "views"] {
            let out = r.step(alias);
            assert!(out.text.contains("GAME INFO"), "'{alias}' must open the menu: {}", out.text);
            let out = r.step("cancel");
            assert!(matches!(r.sub, SubPrompt::None), "cancel closes the menu");
            assert!(out.text.contains("[1]"), "main prompt back after cancel: {}", out.text);
        }
        r.step("view");
        let out = r.step("99");
        assert!(
            out.text.contains(&format!("pick a number 1-{}", Repl::VIEW_MENU.len())),
            "out-of-range re-asks with the range it takes: {}",
            out.text,
        );
        assert!(out.text.contains("GAME INFO"), "menu still showing: {}", out.text);
        // The base itself: `0` is below the first row now, and says so rather
        // than wrapping round to the last one.
        let out = r.step("0");
        assert!(out.text.contains("pick a number 1-"), "0 is not a row: {}", out.text);
    }

    /// The hint line on EVERY prompt names the menu AND the card index — the
    /// discoverability layer that survives piped/rolling modes, and (since T5)
    /// the two keys that reach the two kinds of surface.
    #[test]
    fn input_hint_names_the_view_menu() {
        let r = repl_2p(4);
        let p = r.prompt();
        assert!(p.contains("'view' any surface"), "{p}");
        assert!(p.contains("'?' the card index"), "{p}");
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
        r.step("1");
        r.step(&format!("a {}", port_area.0));
        let k = r.session.legal().iter().position(|m| matches!(
            m, Move::PlacePresident { destination } if *destination == port_area
        )).expect("PlacePresident at Port A is legal from hand at game start");
        r.step(&menu_number(k).to_string());
        let take = r.session.legal().iter().position(|m| matches!(
            m, Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } } if cards_taken.len() == 1
        )).expect("1-card port take available");
        let events_before = r.session.record().events.len();
        let out = r.step(&menu_number(take).to_string());
        assert!(!out.text.to_lowercase().contains("slot"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert!(matches!(r.pending, Pending::None));
    }

    /// The refusal a locked session gives, verbatim from `step`'s gate.
    ///
    /// **`contains("locked")` is NOT this test** (six-lens F2). Every screen a
    /// locked session draws ends in `lockdown_status()`, which paints the word
    /// `[locked]` — on ACCEPTED commands as much as refused ones, which
    /// `the_log_still_reads_under_a_lockdown` demonstrates from the other side.
    /// So the needle has to be the refusal SENTENCE, and it has to be matched as
    /// a whole LINE: this is the NAMED CHECK's sixth recurrence in this branch's
    /// review record (T7, T8, T10, T11, T12, here), and the one place it lands
    /// on the mechanism that keeps a desynced live game from being driven
    /// further.
    const LOCK_REFUSAL: &str = "session locked after a driver/engine fault — only save/log/quit";

    /// Whether `out` is the lockdown refusal — the sentence on its own line,
    /// which the status line beneath it cannot satisfy.
    fn is_lock_refusal(out: &StepOutput) -> bool {
        crate::style::strip_ansi(&out.text).lines().any(|l| l.trim() == LOCK_REFUSAL)
    }

    /// A locked 2P session with ONE applied move behind it — so a command that
    /// slipped through the allow-list and rewound the record shows up as a
    /// number, not only as a missing sentence.
    fn locked_after_one_move() -> Repl {
        let mut r = repl_2p(4);
        r.step("1");
        assert_eq!(r.session.record().events.len(), 1, "one move to take back");
        r.locked = true;
        r
    }

    /// **Everything outside the allow-list is refused IN THE REFUSAL'S OWN
    /// WORDS, and does not run** (six-lens F2).
    ///
    /// The allow-list at `step`'s gate is the desync guard's only door, and its
    /// previous pins asserted `contains("locked")` — a needle every locked
    /// screen satisfies. Widening the list with `undo` (the command that rewinds
    /// the record) left the whole package green, and the locked session
    /// happily undid a move while the screen printing that fact still claimed
    /// only save/log/quit were available.
    ///
    /// Two assertions per word, and neither is redundant: the refusal LINE
    /// (a screen that accepted the command cannot produce it) and the
    /// non-execution of the command's own effect — the record length for the
    /// two that rewind it, the overlay for the ones that would open a screen,
    /// the sub-prompt and capture fields for the ones that would open a
    /// question, and whole-state equality for everything.
    #[test]
    fn every_command_outside_the_lockdown_allow_list_is_refused_and_does_nothing() {
        // The whole dispatch surface a locked prompt could be handed: the two
        // spellings of undo, every view word and index word, the menu, the
        // toggles, the pagers, `suggest`, both forms of `why`, `reveal`, a bare
        // number, and one of each move grammar.
        for line in [
            "undo", "back", "board", "players", "score", "help", "view", "views", "more", "list",
            "suggest", "why", "why 0", "privacy", "panel", "screen", "tracks", "sites", "ports",
            "labs", "hand", "goals", "orders", "techs", "sitecards", "?", "index", "reveal",
            "reveal 1", "1", "2", "a 0", "b 0", "m 0", "r 0:1", "", "wat",
        ] {
            let mut r = locked_after_one_move();
            let state_before = r.session.state().clone();
            let out = r.step(line);
            assert!(
                is_lock_refusal(&out),
                "{line:?} must be refused in the lockdown's own sentence — the status line's \
                 `[locked]` rides on accepted commands too:\n{}",
                out.text,
            );
            assert!(!out.quit, "{line:?} must not quit");
            assert_eq!(
                r.session.record().events.len(),
                1,
                "{line:?} changed the record under lockdown:\n{}",
                out.text,
            );
            assert_eq!(*r.session.state(), state_before, "{line:?} changed the state");
            assert!(r.overlay.is_none(), "{line:?} opened a screen under lockdown");
            assert!(matches!(r.sub, SubPrompt::None), "{line:?} opened a sub-prompt");
            assert!(matches!(r.pending, Pending::None), "{line:?} opened a capture");
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
        assert!(is_lock_refusal(&out), "{}", out.text);
        assert_eq!(*r.session.state(), state_before);
        assert_eq!(r.session.record().events.len(), events_before);

        let out = r.step("1");
        assert!(is_lock_refusal(&out), "{}", out.text);
        assert_eq!(*r.session.state(), state_before);
        assert_eq!(r.session.record().events.len(), events_before);

        let out = r.step("log");
        assert!(!is_lock_refusal(&out), "{}", out.text);
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

    // ---- The one-time end-trigger banner (branch-4 item 5) ----

    /// A 2P state doctored so the NEXT applied move fires the
    /// `OrderDeckEmpty` end-game trigger. Twin of `session.rs`'s
    /// `sim_2p_one_move_from_the_trigger`, and conserving for the same reason:
    /// cards go deck -> boxed pile and port slot -> boxed pile, so the engine's
    /// 36-order conservation invariant holds and `Session::apply`'s post-move
    /// `check_invariants` stays a real check rather than something the fixture
    /// has to dodge.
    fn state_one_move_from_the_trigger(seed: u64) -> GameState {
        let mut state = Session::new_sim(
            GameConfig::for_player_count(2),
            seed,
            vec![StartingOrderChoice::Keep(0); 2],
            Rules::standard(),
        )
        .unwrap()
        .state()
        .clone();
        let deck: Vec<_> = std::mem::take(&mut state.order_deck);
        state.orders_removed.extend(deck);
        if !state.port_state.port_a.has_empty_slot() {
            if let Some(card) = state.port_state.port_a.slots[0].take() {
                state.orders_removed.push(card);
            }
        }
        assert!(!state.end_game_triggered, "the fixture must start UNtriggered");
        state
    }

    /// [`step_one_move`]'s dispatch, handing back the output instead of
    /// dropping it — the shape a test asserting on WHAT a move printed needs.
    fn step_one_move_reporting(r: &mut Repl) -> StepOutput {
        let queued = !r.session.state().pending_choices.is_empty();
        match r.session.state().turn_phase {
            TurnPhase::Placement if !queued => r.step("a"),
            TurnPhase::Movement if !queued => r.step("r"),
            _ => r.step("1"),
        }
    }

    /// The banner reaches the inline stream on the flip, and only then.
    ///
    /// Keyed on the BORDER and on the two-phase rule clause rather than on
    /// "END TRIGGERED", because the reworded delta line carries that headline
    /// too — a test that only looked for the headline would pass with no
    /// banner at all, which is precisely the state the owner played through.
    #[test]
    fn the_banner_reaches_the_inline_stream_once() {
        let mut r = Repl::new(Session::for_test_from_state(
            state_one_move_from_the_trigger(4),
            Rules::standard(),
        ));
        let out = step_one_move_reporting(&mut r);
        assert!(r.session.state().end_game_triggered, "the fixture must actually flip");
        assert!(out.text.contains("END TRIGGERED"), "{}", out.text);
        assert!(
            out.text.contains(crate::style::glyph::DOUBLE_RULE),
            "the flip must draw the bordered block, not just a delta line: {}",
            out.text,
        );
        assert!(
            out.text.contains("then every player gets one full final round"),
            "the banner states the rule: {}",
            out.text,
        );

        let next = step_one_move_reporting(&mut r);
        assert!(
            !next.text.contains(crate::style::glyph::DOUBLE_RULE),
            "the banner is one-time — a second screen must not repeat it: {}",
            next.text,
        );
        assert!(
            !next.text.contains("then every player gets one full final round"),
            "{}",
            next.text,
        );
    }

    /// A flip INSIDE a bot run surfaces with the run rather than being
    /// swallowed by it (spec §2.1's bot clause).
    ///
    /// Driven through `opening()` — the load-time pump, spec §9's call site A
    /// — at a forced terminal, so the review branch is the one under test:
    /// that is the shape where the narrative leaves the frame entirely, and
    /// therefore the shape a banner is most easily lost in. The assertion
    /// reads BOTH halves of the [`Opening`] because which half carries the
    /// narrative is `may_pause`'s business, not this property's.
    #[test]
    fn the_bot_review_carries_the_banner_when_the_flip_happened_inside_the_run() {
        let bots = BotSeats::new(
            vec![Some(Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT)), None],
            0x5A17_3B0C_9E44_D218,
        );
        let mut session =
            Session::for_test_from_state(state_one_move_from_the_trigger(4), Rules::standard());
        session.set_run_seats(bots.seat_provenance());
        let mut r = Repl::new(session);

        let opening = at_a_terminal(|| r.opening());
        assert!(r.session.state().end_game_triggered, "the opening pump must cross the trigger");
        let shown = format!("{}{}", opening.review.clone().unwrap_or_default(), opening.frame);
        let plain = crate::style::test_support::strip_ansi(&shown);
        assert!(plain.contains("END TRIGGERED"), "{plain}");
        assert!(
            plain.contains("then every player gets one full final round"),
            "the bot run must not swallow the banner: {plain}",
        );
    }

    /// A save reloaded AFTER the trigger gets a quiet one-line notice, not the
    /// banner — and the notice is branched, because both save states are real
    /// and they mean different things to the player about to move.
    ///
    /// The banner is transition-fired and a load applies no move, so there is
    /// no transition to fire on; re-firing it here would teach the player that
    /// the trigger had just happened. The negative assertion is on the BORDER,
    /// which is the banner's own mark and appears nowhere else in a composed
    /// frame (`screen.rs`'s title-strip pin).
    #[test]
    fn a_loaded_triggered_save_prints_the_branched_notice() {
        for (final_round, expected) in
            [(false, "finishing the current round"), (true, "FINAL ROUND in progress")]
        {
            let mut state = state_one_move_from_the_trigger(4);
            state.end_game_triggered = true;
            state.trigger_reason = Some(EndGameTrigger::ChurchTrackFull);
            state.trigger_player = Some(PlayerId::new(0));
            state.final_round = final_round;

            let mut r = Repl::new(Session::for_test_from_state(state, Rules::standard()));
            let frame = crate::style::test_support::strip_ansi(&r.opening().frame);
            assert!(frame.contains(expected), "final_round={final_round}: {frame}");
            assert!(
                frame.contains("the Church track filled"),
                "the notice names the reason: {frame}",
            );
            assert!(
                !frame.contains(crate::style::glyph::DOUBLE_RULE),
                "a load must not re-fire the banner (final_round={final_round}): {frame}",
            );
        }
    }

    /// An `undo` whose re-pump REPLAYS the flip drains the banner to its own
    /// screen instead of leaking it (T1 review, finding 0). The reachable
    /// shape: the game opens on a bot run that crosses the trigger (banner
    /// drained by `opening`), the human undoes at the bottom, `Session::undo`
    /// unwinds the run, and the deterministic re-pump replays it — re-arming
    /// the banner inside the undo handler, which used to end without a drain,
    /// handing the stale announcement to the NEXT apply's screen. A replayed
    /// flip is still an observed transition, so the banner speaks on the undo
    /// screen (spec §2.1's transition-fired clause), and the following screen
    /// stays clean.
    #[test]
    fn undo_over_a_triggering_opening_run_drains_the_replayed_banner_to_its_own_screen() {
        let bots = BotSeats::new(
            vec![Some(Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT)), None],
            0x5A17_3B0C_9E44_D218,
        );
        let mut session =
            Session::for_test_from_state(state_one_move_from_the_trigger(4), Rules::standard());
        session.set_run_seats(bots.seat_provenance());
        let mut r = Repl::new(session);
        let _ = r.opening();
        assert!(r.session.state().end_game_triggered, "the opening pump must cross the trigger");

        let undone = r.step("undo");
        assert!(r.session.state().end_game_triggered, "the re-pump replays the same flip");
        assert!(
            undone.text.contains(crate::style::glyph::DOUBLE_RULE),
            "the replayed flip's banner belongs to the undo screen: {}",
            undone.text,
        );

        let next = step_one_move_reporting(&mut r);
        assert!(
            !next.text.contains(crate::style::glyph::DOUBLE_RULE),
            "the banner must not leak onto the screen after the undo: {}",
            next.text,
        );
    }

    /// A FINISHED game's save reloads saying so — not "FINAL ROUND in
    /// progress", which misdescribes a game that is over (T1 review, finding
    /// 1: both `end_game_triggered` and `final_round` stay true in a finished
    /// save, so the two-branch notice took the in-progress arm). The reason
    /// words stay: why the game ended is exactly what a returning reader
    /// wants beside the score.
    #[test]
    fn a_finished_save_reloads_with_game_over_wording_not_in_progress() {
        let mut state = create_test_game_state(2);
        state.end_game_triggered = true;
        state.trigger_player = Some(PlayerId::new(0));
        state.trigger_reason = Some(yokohama_core::EndGameTrigger::OrderDeckEmpty);
        state.final_round = true;
        state.turn_phase = TurnPhase::TurnEnd;
        state.current_player = PlayerId::new(1);
        assert!(is_game_over(&state), "fixture must actually satisfy is_game_over");

        let mut r = Repl::new(Session::for_test_from_state(state, Rules::empty_for_tests()));
        let frame = crate::style::test_support::strip_ansi(&r.opening().frame);
        assert!(frame.contains("GAME OVER"), "{frame}");
        assert!(frame.contains("the order deck is empty"), "the reason survives: {frame}");
        assert!(
            !frame.contains("FINAL ROUND in progress"),
            "a finished game is not in progress: {frame}",
        );
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
        r.step(&menu_number(k).to_string());
        assert_eq!(r.session.state().pending_choices.len(), 1);

        let p = r.prompt();
        assert!(p.contains("YOU ARE DECIDING"), "{p}");
        assert!(!p.contains("more to resolve"), "the last of the cascade has nothing behind it: {p}");
        // The deciding line's own AnyGood copy (T4's zone), which the numbered
        // options do not carry -- `describe_move` words each option "(any-good
        // reward)", so this cannot pass on the option list alone.
        assert!(p.contains("which good to take"), "{p}");
        // The four numbered ChooseAnyGood options (describe_move's wording).
        assert!(p.contains("copper"), "{p}");
        assert!(p.contains("silk"), "{p}");
        assert!(p.contains("tea"), "{p}");
        // `seafood`, the one word for this good on every surface (T11 N4).
        assert!(p.contains("seafood"), "{p}");
        assert!(p.contains("(any-good reward)"), "{p}");
    }

    #[test]
    fn undo_mid_any_good_cascade_steps_back_exactly_one_choice() {
        let mut r = repl_with_any_good_cascade();
        assert_eq!(r.session.state().pending_choices.len(), 2);

        let k = r.session.legal().iter()
            .position(|m| matches!(m, Move::ChooseAnyGood { good: TradeGood::Copper }))
            .expect("Copper is always one of the four AnyGood options");
        r.step(&menu_number(k).to_string());
        assert_eq!(r.session.state().pending_choices.len(), 1);
        let events_before = r.session.record().events.len();

        let out = r.step("undo");
        assert!(out.text.contains("Undid"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before - 1);
        assert_eq!(
            r.session.state().pending_choices.len(), 2,
            "undo must step back exactly ONE choice (re-queue the cascade), not unwind further"
        );

        // "the same choice again": still AnyGood, and the deciding line says
        // one more is queued behind it.
        let p = r.prompt();
        assert!(p.contains("which good to take"), "{p}");
        assert!(p.contains("1 more to resolve"), "{p}");

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
        r.step(&menu_number(k1).to_string());
        assert_eq!(r.session.state().pending_choices.len(), 1);

        let k2 = r.session.legal().iter()
            .position(|m| matches!(m, Move::ChooseAnyGood { good: TradeGood::Silk }))
            .expect("Silk is always one of the four AnyGood options");
        r.step(&menu_number(k2).to_string());
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
            r.step(&menu_number(end).to_string());
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
        let idx = display_number_containing(r, &format!("Place President from hand in {church_label}"))
            .or_else(|| display_number_containing(r, &format!("Move President to {church_label}")))
            .expect("President can reach Church turn 1");
        r.step(&idx.to_string());

        let church_mv = r.session.legal().iter().position(|m| matches!(m, Move::ExecuteAreaAction {
            action: AreaAction::Church {
                copper: true, silk: true, tea: false, seafood: false, yen: true, imported_good: false,
            }
        })).expect("paying copper+silk+yen is affordable turn 1 (starting resources include 1 of each)");
        r.step(&menu_number(church_mv).to_string());

        if matches!(r.session.state().pending_choices.first(), Some(PendingChoice::ChurchSpace { .. })) {
            let space6 = r.session.legal().iter().position(|m| matches!(m, Move::ChooseChurchSpace { space: 6 }))
                .expect("space 6 (Faith 6) is reachable and unclaimed turn 1");
            r.step(&menu_number(space6).to_string());
        }

        assert_eq!(
            r.session.state().pending_choices,
            vec![PendingChoice::AssistantMove, PendingChoice::AssistantMove],
            "setup recipe drifted — re-derive against the current engine before trusting this fixture"
        );
        church_area
    }

    #[test]
    fn assistant_move_m_grammar_applies_from_area_and_hand() {
        let mut r = repl_2p(4);
        let church_area = drive_to_double_assistant_move_cascade(&mut r);
        let p = r.prompt();
        assert!(p.contains("which assistant to move"), "{p}");
        assert!(p.contains("1 more to resolve"), "{p}");
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
    fn assistant_move_m_grammar_returns_an_on_board_assistant_to_hand() {
        // D3 (owner ruling 2026-08-08): `m <area> hand` returns an on-board
        // assistant to your hand, where it stays; `m hand hand` assembles a
        // candidate no legal list ever contains and gets the standard
        // rejection.
        let mut r = repl_2p(4);
        let church_area = drive_to_double_assistant_move_cascade(&mut r);
        let events_before = r.session.record().events.len();
        let hand_before = r.session.state().current_player().assistants.in_hand;
        let on_board_before = r.session.state().current_player().assistants.in_area(church_area);
        assert!(on_board_before >= 1, "fixture premise: an own assistant sits on the Church area");

        let out = r.step(&format!("m {} hand", church_area.0));
        assert!(!out.text.to_lowercase().contains("illegal"), "{}", out.text);
        assert_eq!(r.session.record().events.len(), events_before + 1);
        assert_eq!(
            r.session.state().current_player().assistants.in_hand,
            hand_before + 1,
            "the pawn is back in hand"
        );
        assert_eq!(
            r.session.state().current_player().assistants.in_area(church_area),
            on_board_before - 1,
        );

        // hand-to-hand is never offered.
        let out = r.step("m hand hand");
        assert!(out.text.contains("not one of the legal assistant moves"), "{}", out.text);
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
            r.step(&menu_number(end).to_string());
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
            r.step(&menu_number(end).to_string());
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
            .expect("a 2-area Option A placement is legal and affordable turn 1 (same reasoning as the Option B fixture — no President is on board yet to charge a fee)");
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
        movement_grouping_fixture_with(1)
    }

    /// **Guide §4's captured guided-retreat screens are what this fixture
    /// renders** — the fifth and sixth blocks held to the regeneration
    /// discipline (T11 review F2).
    ///
    /// The guide printed `recovering 2 assistant(s)` with an elision in the
    /// middle: a wording this driver has never emitted (the confirm line has
    /// spoken real plurals since T11) inside a transcript nothing checked. Same
    /// class as the Movement block the T4 review found spliced, and the same
    /// answer — the two prompts a player actually meets, captured whole from
    /// one run, with prose rather than a `...` standing in for the lines
    /// between them.
    ///
    /// Regenerate by pasting both blocks into BOTH formats from ONE run of
    /// this fixture.
    #[test]
    fn the_guide_guided_retreat_captures_are_what_the_fixture_renders() {
        const GUIDE_MD: &str = include_str!("../../../docs/cli-guide.md");
        let mut r = movement_grouping_fixture();
        r.step("3");
        let question = crate::style::strip_ansi(&r.sub_prompt_text());
        for answer in ["2", "1", "1", "1"] {
            r.step(answer);
        }
        let confirm = crate::style::strip_ansi(&r.sub_prompt_text());

        for (what, rendered) in [("the first question", &question), ("the confirm", &confirm)] {
            let rendered = rendered.trim_end_matches('\n');
            let opening = rendered.lines().next().expect("a first line");
            let block = GUIDE_MD
                .split("```")
                .map(|part| part.trim_matches('\n'))
                .find(|part| part.starts_with(opening))
                .unwrap_or_else(|| {
                    panic!("cli-guide.md carries no captured guided retreat ({what}):\n{rendered}")
                });
            assert_eq!(
                block, rendered,
                "docs/cli-guide.md's captured guided retreat ({what}) no longer matches what \
                 `movement_grouping_fixture` renders. Re-capture BOTH blocks in BOTH formats from \
                 ONE run of this fixture — never by editing them by hand, and never with an \
                 elision standing in for a line nobody ran (that is what T11 review F2 found).",
            );
        }
    }

    /// An applied move puts the option list back on its FIRST page.
    ///
    /// A player who pages forward on one decision must not land mid-list on the
    /// next: the numbers on that screen belong to a different question, and a
    /// list that opens at `[24]` reads as a driver that lost its place. The
    /// reset lives one frame deeper than the apply — in `apply_and_describe`,
    /// the shared inner of `finish_apply_ex` and the bot pump — so it covers a
    /// bot's replies as well as the human's own move; this pins the property at
    /// the surface that has to hold it either way.
    #[test]
    fn an_applied_move_resets_the_option_page() {
        let mut r = movement_grouping_fixture_with(1);
        r.page = 3; // as if the player had paged forward
        let out = r.step("2"); // the single-route Tea Plantation move
        assert!(!out.text.contains("Illegal"), "fixture assumption drifted: {}", out.text);
        assert_eq!(r.session.record().events.len(), 1, "the move must have applied");
        assert_eq!(r.page, 0, "an applied move reopens the list at page 1");
    }

    /// Guide §4's two captured Movement screens are what these two fixtures
    /// actually render — byte for byte, caret included.
    ///
    /// **Written after the T4 review found the block was a SPLICE** (finding
    /// M1): it showed the `a3 = 1` position's three offered entries with the
    /// `a3 = 0` position's near-miss row pasted under them, and re-pointed at
    /// the Silk Mill — the one neighbour whose absence this screen deliberately
    /// says NOTHING about, because an opponent's President is not a fact it can
    /// read off the state. The guide was documenting, in a captured screen, the
    /// exact lie the design exists to prevent. Nothing but a regeneration check
    /// catches that class: a hand-edited transcript compiles, renders and reads
    /// perfectly well.
    ///
    /// One captured screen per fixture, and the fixtures are the ones the
    /// near-miss tests already use — so a wording change anywhere in the zone
    /// fails HERE with the new text to paste, rather than leaving the guide
    /// quietly describing a driver that no longer exists.
    #[test]
    fn the_guide_movement_captures_are_what_the_fixtures_render() {
        const GUIDE_MD: &str = include_str!("../../../docs/cli-guide.md");
        let blocks: Vec<&str> = GUIDE_MD
            .split("```")
            .map(|part| part.trim_matches('\n'))
            .filter(|part| part.starts_with("TURN 1 · RED's turn   Additional ▸ Placement ▸  MOVEMENT"))
            .collect();
        assert_eq!(
            blocks.len(),
            2,
            "guide §4 must carry exactly two captured Movement screens, one per fixture below",
        );
        for (a3, block) in [1u8, 0].iter().zip(&blocks) {
            let rendered = movement_grouping_fixture_with(*a3).prompt();
            assert_eq!(
                rendered.trim_end_matches('\n'),
                *block,
                "docs/cli-guide.md's captured Movement screen no longer matches what \
                 `movement_grouping_fixture_with({a3})` renders. Re-capture it in BOTH formats \
                 from ONE run of this fixture — never by editing the block by hand, and never by \
                 splicing two runs together (that is exactly what T4 review M1 found). The \
                 rendered screen is above; `guide_sync` pins the .html twin against the .md.",
            );
        }
    }

    /// **The guide's opening decision-zone screen IS what the driver renders**
    /// (T11), the fourth block held to the regeneration discipline.
    ///
    /// It is the FIRST screen a tester reads, and until now it was the last one
    /// nothing checked — a hand-written illustration of the surface the T4
    /// review found fabricated one section further down. Regenerate it by
    /// running `repl_2p(4).prompt()` and pasting the whole block into BOTH
    /// formats.
    #[test]
    fn the_guide_opening_zone_capture_is_what_the_driver_renders() {
        const GUIDE_MD: &str = include_str!("../../../docs/cli-guide.md");
        let rendered = repl_2p(4).prompt();
        let rendered = rendered.trim_end_matches('\n');
        let opening = rendered.lines().next().expect("the phase chain");
        let block = GUIDE_MD
            .split("```")
            .map(|part| part.trim_matches('\n'))
            .find(|part| part.starts_with(opening))
            .unwrap_or_else(|| panic!("cli-guide.md carries no captured opening zone"));
        assert_eq!(
            block, rendered,
            "docs/cli-guide.md's captured opening decision zone no longer matches what \
             `repl_2p(4).prompt()` renders. Re-capture it in BOTH formats from ONE run — never \
             by editing the block by hand. The rendered screen is above.",
        );
    }

    /// The grouping fixture, with `a3_assistants` of the mover's pawns on the
    /// Tea Plantation. At 1 it is a reachable destination; at 0 it becomes a
    /// NEAR MISS — adjacent, unoffered, and with the observable reason for it
    /// sitting right there in the assistant count.
    fn movement_grouping_fixture_with(a3_assistants: u8) -> Repl {
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
        if a3_assistants > 0 {
            players[0].assistants.on_board.insert(a3, a3_assistants);
        }
        // Keep the per-player total at 20 whatever is on the board.
        players[0].assistants.in_warehouse -= 4 + a3_assistants;
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
        let p = r.prompt();

        // Entry 1: the multi-label destination as a route-options line.
        assert!(
            p.contains(&format!(
                "[1] Move to {} — 2 route options",
                crate::describe::area_name_upper(state, multi),
            )),
            "{p}"
        );
        // Entry 2: the single-label destination, worded by the zone from the
        // model -- the area name, the reachable mark the board also draws, the
        // step count off the path, and the engine's price.
        assert!(
            p.contains(&format!(
                "[2] Move to {} {} — 1 step · free",
                crate::describe::area_type_name(state.board_layout.areas[&single].area_type)
                    .to_uppercase(),
                crate::style::glyph::REACHABLE,
            )),
            "{p}"
        );
        // Entry 3: the collapsed retreat family (24 = 3*2*2*2 combinations).
        assert!(
            p.contains("[3] Retreat President to hand — choose recovery (24 combinations; 'r' grammar works too)"),
            "{p}"
        );
        // Exactly three entries -- no [4], and none of the 27 raw legal moves
        // (3 MovePresident + 24 retreats) leaked through ungrouped.
        assert!(!p.contains("[4]"), "grouping must leave exactly 3 entries: {p}");
        assert_eq!(r.session.legal().len(), 27, "fixture assumption: 3 MovePresident + 24 retreats");
    }

    /// The Movement hint keeps the `r` shorthand as the expert ALIAS (§0.2)
    /// and the numbered entry stays the guided way in, so both routes to a
    /// retreat are on screen at once.
    #[test]
    fn movement_hint_names_the_retreat_entry() {
        let r = movement_grouping_fixture();
        let p = r.prompt();
        assert!(p.contains("'r' retreats directly"), "{p}");
        assert!(p.contains("choose recovery"), "the numbered entry is the guided route: {p}");
    }

    /// Route sub-prompt: selecting the multi-label entry lists exactly that
    /// destination's two labels with their unchanged cost/recipient text
    /// (canal toll vs. fee-to-Blue), and a number commits the chosen move.
    #[test]
    fn route_subprompt_lists_labels_then_commits() {
        let mut r = movement_grouping_fixture();
        let (multi, _single) = grouping_fixture_dests();

        let out = r.step("1"); // enter the route sub-prompt for the multi-label dest
        assert!(out.text.contains(&format!("Routes to {}", crate::describe::area_label(r.session.state(), multi))), "{}", out.text);
        // Both labels, with the SAME cost/recipient wording describe_move uses.
        assert!(out.text.contains("[1]") && out.text.contains("[2]"), "{}", out.text);
        assert!(out.text.contains("canal toll"), "canal-toll route label missing: {}", out.text);
        assert!(out.text.contains("to Blue"), "fee-to-Blue route label missing: {}", out.text);
        assert!(!out.text.contains("[3]"), "exactly two routes to this destination: {}", out.text);

        // A number commits that exact MovePresident (assert via the applied event).
        let events_before = r.session.record().events.len();
        let out = r.step("1");
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

        r.step("1"); // into the route sub-prompt
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
        let out = r.step("3");
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
        assert!(out.text.contains("Return President to hand, recovering 5 assistants"), "{}", out.text);
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
        r.step("3"); // enter -> first question is A00 [0-2]

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
        r.step("3");   // enter
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
        r.step("3"); // enter the guided retreat
        let out = r.step("board");
        assert!(out.text.contains("! finish or cancel this prompt first"), "{}", out.text);
        // The view did NOT render, and the question is still showing.
        assert!(!out.text.contains("grid"), "the board view must not render mid-collection: {}", out.text);
        assert!(out.text.contains("recover how many from"), "the question must be re-shown: {}", out.text);
        assert!(r.session.record().events.is_empty());
    }

    /// Lockdown wins over an in-progress sub-prompt: once locked, even
    /// sub-prompt input is refused the same way moves are (only save/log/quit).
    ///
    /// The needle is the refusal sentence rather than the word `locked`, for
    /// the reason [`LOCK_REFUSAL`] states.
    #[test]
    fn subprompt_refused_under_lockdown() {
        let mut r = movement_grouping_fixture();
        r.step("3"); // enter the guided retreat
        r.locked = true;
        let out = r.step("2"); // would be an answer, but lockdown refuses it
        assert!(is_lock_refusal(&out), "{}", out.text);
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
        r.step("1"); // -> Placement
        assert!(r.session.legal().len() > 100, "fixture: the raw wall is still combinatorial");
        let p = r.prompt();
        assert!(p.contains("[1] Option A — place 1 assistant each in up to 3 areas"), "{p}");
        assert!(p.contains("[2] Option B — place 2 assistants in one area (10 eligible"), "{p}");
        assert!(p.contains("[3] Place no assistants"), "{p}");
        assert!(!p.contains("[4]"), "exactly three entries: {p}");
        assert!(!p.contains("page 1/"), "no pagination on the grouped list: {p}");
        assert!(p.contains("'a' grammar works too") && p.contains("'b' grammar works too"), "{p}");
    }

    /// **Both Placement pickers label an area the same way** (six-lens note),
    /// and that way is [`describe::area_label`] — the form the routes title,
    /// the guided retreat, a President's location and the log all use.
    ///
    /// They are the two halves of ONE decision, reached from options [1] and
    /// [2] of the same list, and they used to disagree about which end the id
    /// goes on: `A00 Copper Mine` against `Copper Mine (A00)`.
    #[test]
    fn both_placement_pickers_name_an_area_in_the_drivers_one_label() {
        let area = AreaId::new(3);
        let label = {
            let r = repl_2p(4);
            describe::area_label(r.session.state(), area)
        };
        assert!(label.ends_with(&format!("(A{:02})", area.0)), "fixture: {label:?}");
        let name = label.split(" (A").next().expect("a name before the id").to_string();

        for enter in ["1", "2"] {
            let mut r = repl_2p(4);
            r.step("1"); // -> Placement
            let out = r.step(enter);
            assert!(
                out.text.contains(&label),
                "picker [{enter}] must list areas as {label:?}:\n{}",
                out.text,
            );
            assert!(
                !out.text.contains(&format!("A{:02} {name}", area.0)),
                "picker [{enter}] still carries the id-first form:\n{}",
                out.text,
            );
        }
    }

    /// Option-A picker: toggling two areas assembles the exact 2-area
    /// placement; the screen shows the live selection + legality/fee line;
    /// `confirm` commits it through the standard legal-list match.
    #[test]
    fn place_a_picker_toggles_areas_and_commits() {
        let mut r = repl_2p(4);
        r.step("1");
        let out = r.step("1"); // enter the Option A picker
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
        r.step("1");
        r.step("1"); // enter picker
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
        r.step("1");
        r.step("1"); // enter the Option A picker
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
        r2.step("1");
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
        r.step("1");
        r.step("1"); // enter the Option A picker
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
        r.step("1");
        r.step("1"); // enter picker
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
        r.step("1");
        r.step("1"); // enter picker
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
        r.step("1");
        let out = r.step("2"); // enter the Option B picker
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

        r.step("1"); // -> Placement
        let b_entry =
            display_number_containing(&r, "Option B").expect("Option B entry present at 4P turn 1");
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
        r2.step("1");
        let b_entry = display_number_containing(&r2, "Option B").expect("Option B entry present");
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
        r.step("1");
        let b_entry = display_number_containing(&r, "Option B").expect("Option B entry present");
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
        let out = r.step(&option_number(&p, "Option B").to_string());
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
        let out = r.step("1");
        assert!(out.text.contains("Assistant destinations from"), "{}", out.text);
        let events_before = r.session.record().events.len();
        let out = r.step("1");
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
        assert!(p.contains("[1] Return the President to your hand — recovering no assistants"), "{p}");
        assert!(!p.contains("choose recovery"), "a lone retreat must NOT collapse: {p}");
        assert!(!p.contains("[2]"), "a lone-retreat state has exactly one entry: {p}");

        let events_before = r.session.record().events.len();
        let out = r.step("1");
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

        // Page 1 caps at 20 entries ([1]..[20]) and closes with the DECISION
        // ZONE's own pager footer -- one pager vocabulary in the driver, not
        // two (T5: the sub-prompt used to say `...5 more (type 'more')` over
        // `page 1/2` for behaviour identical to the main list's).
        let p0 = r.sub_prompt_text();
        assert!(p0.contains("[1] ") && p0.contains("[20] "), "{p0}");
        assert!(!p0.contains("[21] "), "page 1 must cap at 20 route entries: {p0}");
        assert!(p0.contains(&crate::decision_render::pager_footer(5, 0, 2)), "{p0}");

        // `more` pages forward to the remaining 5.
        let p1 = r.step("more").text;
        assert!(p1.contains("[21] ") && p1.contains("[25] "), "{p1}");
        assert!(!p1.contains("[20] "), "page 2 must not repeat page-1 entries: {p1}");
        assert!(p1.contains(&crate::decision_render::pager_footer(0, 1, 2)), "{p1}");

        // `list` resets to page 1.
        let p2 = r.step("list").text;
        assert!(p2.contains("[1] ") && p2.contains("'more' for page 2 of 2"), "{p2}");
        // Still in the sub-prompt, nothing applied.
        assert!(matches!(r.sub, SubPrompt::Pick { .. }));
        assert!(r.session.record().events.is_empty());
    }

    /// The pager footer line of a rendered surface, ANSI stripped and trimmed.
    fn pager_line(text: &str) -> String {
        crate::style::strip_ansi(text)
            .lines()
            .find(|l| l.contains("'more'"))
            .map(|l| l.trim().to_string())
            .unwrap_or_else(|| panic!("no pager footer in:\n{text}"))
    }

    /// A footer with every run of digits collapsed to `#` — the SENTENCE
    /// without the counts, so two surfaces showing different-sized lists can
    /// still be compared word for word.
    fn pager_shape(line: &str) -> String {
        let mut out = String::new();
        let mut in_digits = false;
        for c in line.chars() {
            if c.is_ascii_digit() {
                if !in_digits {
                    out.push('#');
                }
                in_digits = true;
            } else {
                in_digits = false;
                out.push(c);
            }
        }
        out
    }

    /// **One pager vocabulary, pinned as AGREEMENT rather than as two literal
    /// copies** (T5 review S4). The main option list and the route sub-prompt
    /// page identically, so they must print one sentence — and each surface's
    /// footer is checked against `decision_render::pager_footer` at that
    /// surface's OWN numbers, which is what makes the shared formatter the
    /// thing under test rather than a wording two tests happen to spell alike.
    ///
    /// Both shapes are swept: a mid-list page (which counts a remainder) and a
    /// last page (which words the wrap). The two surfaces hold lists of
    /// different sizes, so the comparison BETWEEN them is over the sentence with
    /// its counts collapsed; the comparison against the formatter is exact.
    #[test]
    fn one_pager_footer_serves_both_paged_surfaces() {
        // Surface 1: the route sub-prompt, 25 rendering-only labels = 2 pages.
        let mut sub = repl_2p(4);
        let moves: Vec<Move> = (0..25)
            .map(|i| Move::MovePresident { path: vec![], destination: AreaId::new((i % 10) as u8) })
            .collect();
        sub.sub = SubPrompt::Pick { title: "Routes".to_string(), moves };

        // Surface 2: the main option list, an Employment Agency at power 5.
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
        let mut main = Repl::new(Session::for_test_from_state(state, Rules::empty_for_tests()));

        // Mid-list: the route sub-prompt's numbers are known exactly.
        let sub_first = pager_line(&sub.sub_prompt_text());
        assert_eq!(sub_first, crate::decision_render::pager_footer(5, 0, 2), "{sub_first}");
        let main_first = pager_line(&main.prompt());
        assert_eq!(
            pager_shape(&main_first),
            pager_shape(&sub_first),
            "the two paged surfaces must print ONE sentence:\n  main: {main_first}\n  sub:  {sub_first}",
        );

        // Last page: the wrap form, whose only count is the page total.
        let sub_last = pager_line(&sub.step("more").text);
        assert_eq!(sub_last, crate::decision_render::pager_footer(0, 1, 2), "{sub_last}");
        let mut steps = 0;
        while !pager_line(&main.prompt()).starts_with("(last page") {
            main.step("more");
            steps += 1;
            assert!(steps < 20, "the main pager must reach a last page");
        }
        let main_last = pager_line(&main.prompt());
        assert_eq!(
            pager_shape(&main_last),
            pager_shape(&sub_last),
            "...on the last page too:\n  main: {main_last}\n  sub:  {sub_last}",
        );
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
    ///
    /// **Deliberately the RUNG, not the wizard's default** (W5 task 4, where
    /// that default became `lookahead-v1`). Every fixture below wants SOME bot
    /// moving so a pump, a review surface, an `undo` or a lockdown can be
    /// observed; none of them is about which contestant sits there, and a
    /// search per decision would pay for a property no test here asserts. The
    /// rung is not deprecated -- it is simply no longer the default. A test
    /// whose SUBJECT is the shipped default belongs at the wizard, where
    /// `the_wizard_default_bot_seat_is_the_provisional_production_contestant`
    /// pins it.
    fn repl_2p_bots(seed: u64, bot_seats: &[u8]) -> Repl {
        repl_np_bots(2, seed, bot_seats)
    }

    /// [`repl_2p_bots`] at any player count — the shape a review surface needs
    /// to be measured at its ceiling, since only several consecutive bot seats
    /// reach the pump's display cap.
    fn repl_np_bots(players: u8, seed: u64, bot_seats: &[u8]) -> Repl {
        let mut seats = vec![None; players as usize];
        for s in bot_seats {
            seats[*s as usize] = Some(Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT));
        }
        let bots = BotSeats::new(seats, 0x5A17_3B0C_9E44_D218);
        let mut session = Session::new_sim(
            GameConfig::for_player_count(players),
            seed,
            vec![StartingOrderChoice::Keep(0); players as usize],
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
            "exactly one prompt closes the step — the pump must not add its own: {}",
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

    // ---- The load-time bot-turn review (Phase-3.5a fix wave, finding A) ----

    /// A process that looks like a real terminal on BOTH streams — which is
    /// what [`Opening`] asks about, and the only configuration in which it puts
    /// a pause on the screen. Both switches are thread-local test hooks, so
    /// this stays hermetic under a parallel run.
    fn at_a_terminal<T>(body: impl FnOnce() -> T) -> T {
        screen::force(true);
        crate::input::force_pause(true);
        let out = body();
        crate::input::force_pause(false);
        screen::force(false);
        out
    }

    /// **The defect, measured.** `main.rs` composed ONE frame out of the bot's
    /// whole opening turn plus the decision prompt, and `screen::wrap` leads
    /// with clear-screen + home — so on a load with a bot on the start seat the
    /// board's top rows were pushed off a 50-row terminal before the player
    /// could read them, and the only way back to them was to scroll.
    ///
    /// This pins the two heights the fix is FOR, so the fix cannot be undone
    /// without a failing number: the composed-everything frame overruns
    /// `screen`'s recorded 2P opening height (51 rows since branch-4's Orders
    /// line, `FRAME_HEIGHTS`; 49 before it) while the frame the player actually
    /// lands on after the review is back at that recorded height exactly.
    ///
    /// **The floor clause of that sentence went with the Orders line.** Until
    /// 2026-08-18 the reviewed frame was BOTH at its recorded height and inside
    /// the 50-row design floor, and the fixture asserted the all-in-one frame
    /// overran the floor; the 2P opening is now one row past the floor itself
    /// (`screen::FRAME_HEIGHTS`' own doc records why), so what the fixture
    /// asserts is the thing that is still true and still discriminating — the
    /// all-in-one frame overruns the RECORDED OPENING, by enough that a
    /// player loses the board's top rows.
    #[test]
    fn the_bot_narrative_overran_the_design_floor_and_the_reviewed_frame_does_not() {
        let mut r = repl_2p_bots(4, &[0]);
        let (all_in_one, opening) = at_a_terminal(|| {
            // Exactly what `main.rs` built before this fix: warnings, the whole
            // pump, the prompt, one `wrap`.
            let mut probe = repl_2p_bots(4, &[0]);
            let mut body = String::new();
            body.push_str(&probe.pump_bots());
            body.push_str(&probe.prompt());
            let all_in_one = screen::wrap(&probe, &body).lines().count();
            (all_in_one, r.opening())
        });

        let reviewed = opening.frame.lines().count();
        assert!(
            all_in_one > reviewed,
            "fixture: the composed-everything opening must actually overrun the frame the \
             player lands on (it drew {all_in_one} rows against {reviewed}) — otherwise this \
             test is measuring nothing",
        );
        assert_eq!(
            reviewed, 51,
            "and the frame behind the review is the ordinary 2P opening frame, at the height \
             `screen::FRAME_HEIGHTS` records for it (51 since branch-4's Orders line, \
             2026-08-18; 49 before it) — it drew {reviewed} rows",
        );
    }

    /// The review appears on EXACTLY the loads that have a bot turn to review,
    /// and only where there is a human at a terminal to press Enter.
    #[test]
    fn the_review_appears_only_with_a_bot_run_to_review_and_only_at_a_terminal() {
        let with_bot = at_a_terminal(|| repl_2p_bots(4, &[0]).opening());
        assert!(with_bot.review.is_some(), "a bot on the start seat plays a turn the player has not seen");

        // The human is to move: the pump applies nothing, so there is nothing
        // to review and no reason to make anyone press a key.
        let human_to_move = at_a_terminal(|| repl_2p_bots(4, &[1]).opening());
        assert!(human_to_move.review.is_none(), "no bot run means no review screen");

        // The same bot load under a pipe: no pause, whatever else is true.
        let piped = repl_2p_bots(4, &[0]).opening();
        assert!(piped.review.is_none(), "a piped run must never be asked to press Enter");

        // And the second half of the gate on its own: the review is a surface
        // built out of clear-screen escapes, so it needs the FRAME to be on and
        // not merely a keyboard to be attached. Production cannot separate the
        // two (a pausable process has a terminal on stdout, which is what turns
        // frames on) — this is the arm that keeps the conjunct from rotting into
        // a line nothing exercises.
        screen::force(false);
        crate::input::force_pause(true);
        let unframed = repl_2p_bots(4, &[0]).opening();
        crate::input::force_pause(false);
        assert!(unframed.review.is_none(), "no frame to replace means no full-screen surface");

        // **And the FIRST half on its own, which is the arm this test shipped
        // without** (review MUST-1). The piped case above exercises neither
        // conjunct in isolation: `screen::enabled()` is false in every test
        // process, so it was satisfied by the frame clause alone and deleting
        // `may_pause()` left the whole suite green — an unpinned gate guarding
        // the one thing a pipe must never do.
        screen::force(true);
        crate::input::force_pause(false);
        let no_human = repl_2p_bots(4, &[0]).opening();
        screen::force(false);
        assert!(
            no_human.review.is_none(),
            "frames on but stdin not a terminal: a pause would eat the next command"
        );
    }

    /// **The review carries what the player missed** — the bot's described
    /// moves and the aggregate delta the pump summarizes them with — plus the
    /// one instruction for leaving it. Asserted against the pump's OWN text
    /// rather than against re-typed wording, so the two cannot drift.
    #[test]
    fn the_review_carries_the_bot_narrative_and_its_delta_summary() {
        let narrative = repl_2p_bots(4, &[0]).pump_bots();
        let review = at_a_terminal(|| repl_2p_bots(4, &[0]).opening()).review.expect("a bot run");

        for line in narrative.lines() {
            assert!(review.contains(line), "the review must carry every pump line; missing {line:?}:\n{review}");
        }
        assert!(narrative.contains("Seat 0"), "fixture: the narrative describes the bot's moves");
        assert!(
            narrative.lines().any(|l| l.starts_with("Red:") || l.starts_with("Blue:")),
            "fixture: the pump ends with delta_summary lines:\n{narrative}",
        );
        // The WHOLE sentence, not `contains("press Enter")` (review NOTE-1):
        // that needle was also satisfied by the decision zone's "only option —
        // press Enter to apply", so it would have passed on a review whose
        // footer had been replaced by a different screen's text entirely.
        assert!(
            review.contains(crate::view_render::REVIEW_ENTER_LINE),
            "and it says how to leave:\n{review}"
        );
        // A surface that REPLACES the frame, the way every other full-screen
        // surface does — the clear-and-home prefix `screen::wrap` gives them.
        assert!(review.starts_with("\u{1b}[2J\u{1b}[H"), "the review is a full-screen surface");
    }

    /// **The frame behind the review is the frame this position would have
    /// drawn if there had been nothing to review** — byte for byte, so the
    /// review is a pause in front of the ordinary opening and not a second
    /// rendering path that can drift from it.
    ///
    /// The comparison frame is reached by an INDEPENDENT route: the reviewed
    /// session is saved and re-loaded, and the resumed load has a human to move,
    /// so its own `opening` pumps nothing and reviews nothing. The two runs
    /// therefore differ in exactly one thing — whether a bot run happened inside
    /// this process — which is the difference under test.
    #[test]
    fn the_frame_behind_the_review_is_byte_identical_to_a_load_with_nothing_to_review() {
        let mut r = repl_2p_bots(4, &[0]);
        let reviewed = at_a_terminal(|| r.opening());
        assert!(reviewed.review.is_some(), "fixture: there was a bot turn to review");

        let path = std::env::temp_dir().join(format!("yokohama_review_frame_{}.json", std::process::id()));
        r.session.save(&path).expect("save");
        let mut resumed = Repl::new(Session::load(&path, Rules::standard()).expect("load"));
        // The title strip states whether the session autosaves, and a LOADED
        // session has a save path where a wizard-built one does not. Clearing it
        // holds the two routes to their one intended difference; it is not part
        // of what the fix does.
        resumed.session.save_path = None;
        let plain = at_a_terminal(|| resumed.opening());
        std::fs::remove_file(&path).ok();

        assert!(plain.review.is_none(), "fixture: the resumed load has the human to move");
        assert_eq!(reviewed.frame, plain.frame);
    }

    /// **A piped load is byte-for-byte what it was before the review existed.**
    /// The expectation is not re-typed wording — it is the exact expression
    /// `main.rs` used to evaluate, run against a second `Repl` at the same seed,
    /// so this fails on any change of SHAPE (an added pause, a moved warning, a
    /// dropped narrative) rather than on any change of content.
    #[test]
    fn a_piped_load_is_the_same_stream_it_was_before_the_review_existed() {
        let opening = repl_2p_bots(4, &[0]).opening();

        let mut before = repl_2p_bots(4, &[0]);
        let mut body = String::new();
        for warning in &before.session.load_warnings {
            body.push_str(warning);
            body.push('\n');
        }
        body.push_str(&before.pump_bots());
        body.push_str(&before.prompt());

        assert!(opening.review.is_none(), "no pause under a pipe — and so no line consumed");
        assert_eq!(opening.frame, screen::wrap(&before, &body));
    }

    /// The review is a full-screen surface and owes the design floor what every
    /// other one owes it: the TALLEST bot run the pump will ever DISPLAY has to
    /// fit, or the fix for a screen that scrolled is a screen that scrolls.
    ///
    /// **Two shapes, because the 2P cells alone were not measuring the tall
    /// case** (review SHOULD-1). One bot turn at 2P never reaches
    /// [`BOT_LINE_CAP`], so those cells left the pump's own display ceiling
    /// untested; three consecutive bot seats at 4P reach it, and the cap plus
    /// its `…N more` pointer plus the aggregate delta IS the ceiling — past that
    /// point more bot moves add no rows. The cap is asserted REACHED at the 4P
    /// cells, so the claim in this doc comment is a fact about what runs rather
    /// than a hope about the seeds.
    #[test]
    fn the_review_fits_the_fifty_row_design_floor() {
        for seed in [4u64, 7, 11, 19, 23] {
            let review = at_a_terminal(|| repl_2p_bots(seed, &[0]).opening())
                .review
                .unwrap_or_else(|| panic!("seed {seed} must open on a bot turn — a skipped cell measures nothing"));
            let rows = review.lines().count();
            assert!(rows <= 50, "the review at 2P seed {seed} draws {rows} rows against the 50-row floor");
        }

        let mut capped = 0;
        for seed in [4u64, 7, 11, 19, 23, 31, 47] {
            let review = at_a_terminal(|| repl_np_bots(4, seed, &[0, 1, 2]).opening())
                .review
                .unwrap_or_else(|| panic!("4P seed {seed} must open on three bot turns"));
            let rows = review.lines().count();
            assert!(rows <= 50, "the review at 4P seed {seed} draws {rows} rows against the 50-row floor");
            if review.contains("more — type `log`") {
                capped += 1;
            }
        }
        assert!(
            capped > 0,
            "anti-vacuity: no 4P cell reached BOT_LINE_CAP, so nothing here measured the display ceiling \
             this test claims to measure"
        );
    }

    /// **The footer's promise about `log` was FALSE and is now checked against
    /// `log` itself** (review MUST-2). It read "'log' will still have every line
    /// of it", which a player would read as: the screen you just dismissed is
    /// recoverable. It is not — `log` re-describes each move against the state
    /// of the time and carries neither the aggregate delta nor the hand-over
    /// marker, so not one line of the review appears in it verbatim.
    ///
    /// What IS true is that every MOVE is there, and that is what the new
    /// sentence claims. Pinned by finding each move's own description — the part
    /// `describe_move` produced, which the pump prefixes with a seat and `log`
    /// prefixes with a colour — inside the log text.
    #[test]
    fn the_reviews_promise_about_log_is_true_of_the_moves_and_no_more() {
        let mut r = repl_2p_bots(4, &[0]);
        let narrative = r.pump_bots();
        let log = r.log_text();

        let move_lines: Vec<&str> = narrative.lines().filter(|l| l.starts_with("Seat 0 (Red) ")).collect();
        assert!(move_lines.len() >= 3, "fixture: the bot's turn is several moves:\n{narrative}");
        for line in &move_lines {
            let described = line.split_once(" \u{2014} ").expect("a pump move line carries its description").1;
            assert!(
                log.contains(described),
                "the footer promises `log` keeps every move it made, so {described:?} must be in it:\n{log}"
            );
        }

        // The other half of the claim, stated as the measurement that condemned
        // the old wording: NO review line survives verbatim, which is exactly
        // why the promise may not be about lines.
        let verbatim = narrative.lines().filter(|l| !l.is_empty() && log.contains(*l)).count();
        assert_eq!(
            verbatim, 0,
            "if review lines DO start appearing in `log`, this test's premise changed and the \
             footer wording should be revisited deliberately"
        );
    }

    /// The footer says the true thing and no longer says the false one.
    ///
    /// The needle is a WHOLE SENTENCE rather than `contains("press Enter")`
    /// (review NOTE-1): the short needle was also satisfied by the decision
    /// zone's "only option — press Enter to apply", which is the repo's
    /// overlapping-text defect for the seventh time.
    #[test]
    fn the_review_footer_states_what_log_keeps_and_what_a_typed_line_does() {
        let review = at_a_terminal(|| repl_2p_bots(4, &[0]).opening()).review.expect("a bot run");

        assert!(
            review.contains(crate::view_render::REVIEW_ENTER_LINE),
            "the footer must carry its whole sentence:\n{review}"
        );
        assert!(
            review.contains(crate::view_render::REVIEW_DISCLOSURE_LINE),
            "and its disclosure:\n{review}"
        );
        assert!(
            !review.contains("will still have every line"),
            "the retired false promise must not come back:\n{review}"
        );
    }

    /// **`quit` works here, and the screen says so** (review SHOULD-2/SHOULD-3).
    /// The dismissal used to swallow every line including `quit`, which meant a
    /// player quitting at the review got their board back instead — and the
    /// screen's own text told them anything typed was discarded, which was then
    /// true of `quit` too. The classification is a pure function so the
    /// dangerous direction lives in the library rather than in `main`.
    #[test]
    fn a_dismissal_is_quit_for_exactly_the_words_the_prompt_calls_quit() {
        for line in ["quit", "q", "  quit  ", "\tq\t"] {
            assert!(matches!(dismissal(line), Dismissal::Quit), "{line:?} quits at every other prompt");
        }
        for line in ["", " ", "log", "score", "quitter", "q q", "QUIT", "Q", "1"] {
            assert!(
                matches!(dismissal(line), Dismissal::Discard),
                "{line:?} is not the quit word — the review discards it"
            );
        }
    }

    /// The weld: quitting at the review is the SAME event as quitting anywhere
    /// else, down to the bytes. [`QUIT_TEXT`] is what `main` prints there, and
    /// this is what stops it drifting from the eight arms that still spell those
    /// bytes inline.
    #[test]
    fn the_reviews_quit_prints_what_every_other_quit_prints() {
        let mut r = repl_2p_bots(4, &[1]);
        let out = r.step("quit");
        assert!(out.quit, "fixture: `quit` at an ordinary prompt quits");
        assert_eq!(out.text, QUIT_TEXT);
    }

    /// A pause is not a prompt, and must not draw the caret that claims it is
    /// (review SHOULD-3b). Every other full-screen surface ends in one because
    /// every other one takes input that MEANS something; this one reads a line
    /// only to throw it away.
    #[test]
    fn the_review_draws_no_input_caret() {
        let review = at_a_terminal(|| repl_2p_bots(4, &[0]).opening()).review.expect("a bot run");
        assert!(
            !review.trim_end().ends_with('>'),
            "the review must not end in the prompt caret — its own text says it is not a prompt:\n{review}"
        );
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
            _contestant: Contestant,
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
        let bots = BotSeats::new(vec![None, Some(Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT))], 3);
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
            _ => r.step("1"),
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
    // -----------------------------------------------------------------
    // T5: the card index and the numbering base
    // -----------------------------------------------------------------

    /// **`?` opens the index from every prompt this driver has**, which is
    /// spec §0.6's whole point — the index is needed most inside an input
    /// capture, and both capture loops used to swallow the key (the reveal
    /// collector as a bad card, the sub-prompt as `! finish or cancel this
    /// prompt first`).
    #[test]
    fn the_index_opens_from_every_prompt() {
        // The main prompt.
        let mut r = repl_2p(4);
        let out = r.step("?");
        assert!(out.text.contains("ORDER CARD INDEX"), "{}", out.text);
        assert!(r.overlay.is_some(), "the overlay is up");

        // A Movement sub-prompt: the guided retreat, mid-question.
        let mut r = movement_grouping_fixture();
        r.step("3"); // enter the guided retreat
        assert!(matches!(r.sub, SubPrompt::Retreat { .. }));
        let out = r.step("?");
        assert!(out.text.contains("ORDER CARD INDEX"), "{}", out.text);
        assert!(matches!(r.sub, SubPrompt::Retreat { .. }), "the sub-prompt is untouched");

        // The `view` menu is a sub-prompt too, and `?` is no longer its alias.
        let mut r = repl_2p(4);
        r.step("view");
        let out = r.step("?");
        assert!(out.text.contains("ORDER CARD INDEX"), "{}", out.text);
    }

    /// **The return contract, pinned on the hardest case** (spec §0.6): the
    /// index opens mid-reveal, is read-only while it is up, and closing it puts
    /// the operator back on the EXACT question with everything already
    /// collected still collected.
    #[test]
    fn the_index_inside_a_reveal_capture_is_read_only_and_returns_exactly() {
        let (mut r, _) = live_repl_at_a_two_card_port_take();
        let asked = r.reveal_prompt();
        let events_before = r.session.record().events.len();
        let state_before = r.session.state().clone();

        let out = r.step("?");
        assert!(out.text.contains("ORDER CARD INDEX"), "{}", out.text);
        // The context header names the slot being recorded, not a generic view.
        assert!(out.text.contains("Recording: Port"), "{}", out.text);
        assert!(matches!(r.pending, Pending::OrderReveals { .. }), "the capture is untouched");

        // Read-only: filtering and paging around the index changes nothing.
        r.step("f silk");
        r.step("f");
        assert_eq!(r.session.record().events.len(), events_before, "the index records nothing");
        assert_eq!(*r.session.state(), state_before, "the index applies nothing");

        // Closing returns to the exact question.
        let back = r.step("?");
        assert!(back.text.contains(&asked), "must return to the same question: {}", back.text);
        assert!(matches!(r.pending, Pending::OrderReveals { collected, .. } if collected.is_empty()));
    }

    /// A number typed on the index answers the prompt underneath it — the
    /// identification bridge R8 exists to build. The card still goes through
    /// the reveal's own pool check, so the index adds no authority.
    #[test]
    fn an_index_selection_answers_the_reveal_underneath_it() {
        let (mut r, deck_card) = live_repl_at_a_two_card_port_take();
        r.step("?");
        let model = r.index_model(IndexKind::Orders);
        let number = model
            .cards()
            .find(|c| c.face.card_ref() == Some(CardRef::Order(deck_card)))
            .and_then(|c| c.selection)
            .expect("an unplaced deck card is selectable on the recording screen");
        let events_before = r.session.record().events.len();

        let out = r.step(&number.to_string());
        // The card is echoed in full before anything commits (§0.6).
        assert!(out.text.contains(&format!("O{:02}", deck_card.0)), "{}", out.text);
        assert!(r.overlay.is_none(), "the index closes on a selection");
        // A two-slot refill: the first answer advances the group rather than
        // applying, which is the reveal choreography unchanged.
        assert!(matches!(&r.pending, Pending::OrderReveals { collected, .. } if collected == &vec![deck_card]));
        assert_eq!(r.session.record().events.len(), events_before, "nothing applies mid-group");
    }

    /// A card already visible on a Port display is refused BY LOCATION when it
    /// is typed at a card prompt (F6) — the refusal the owner's live-gate
    /// attempt got as a bare "already entered".
    #[test]
    fn a_card_prompt_refusal_names_where_the_card_already_is() {
        let (mut r, _) = repl_live_hidden(21);
        let on_display = r.session.state().port_state.port_a.slots[0].expect("Port A slot 1 is dealt");
        let out = r.step(&format!("reveal 1 O{:02}", on_display.0));
        assert!(out.text.contains("Port A slot 1"), "the refusal names the place: {}", out.text);
        assert!(out.text.contains("cannot be in two places"), "{}", out.text);
        assert!(
            r.session.record().events.iter().all(|e| !matches!(e, RecordedEvent::RevealStartingOrder { .. })),
            "nothing was recorded",
        );
    }

    // -----------------------------------------------------------------
    // T6: the other three registries, and the two full-screen views
    // -----------------------------------------------------------------

    /// **`?` at a live TECH reveal opens the TECHNOLOGY index** — the T5
    /// hand-off, routed. Until T6 it opened the ORDER index: thirty-six cards
    /// that cannot answer "which technology refilled the Lab", which is a screen
    /// answering a question nobody asked.
    ///
    /// The pending state is set directly. That is a fixture rather than a
    /// shortcut: driving a live Laboratory action to its reveal question takes a
    /// dozen moves and would pin the route, not the routing, which is what this
    /// test is about.
    #[test]
    fn a_tech_reveal_routes_the_question_mark_to_the_technology_index() {
        let (mut r, _) = repl_live_hidden(21);
        r.pending = Pending::TechReveal {
            mv: Move::UseUniversity,
            board_b: false,
        };
        let out = r.step("?");
        assert!(out.text.contains("TECHNOLOGY INDEX"), "{}", out.text);
        assert!(
            out.text.contains("Recording: the card that refilled Laboratory A"),
            "in RECORDING context, naming the board it is asking about: {}",
            out.text,
        );
        assert!(matches!(r.pending, Pending::TechReveal { .. }), "the capture is untouched");
        // The leak fence, on the tech half: the locations on screen are the
        // face-up ones the caller supplied (Lab slots, owned techs) and nothing
        // is derived from the tech DECK.
        let filled: Vec<_> = r.session.state().lab_state.lab_a.slots.iter().flatten().collect();
        // Anti-vacuity: an empty board would make the sweep below assert
        // nothing at all, and a fresh game deals six.
        assert_eq!(filled.len(), 6, "a fresh Laboratory A is full; the sweep needs cards to check");
        for id in filled {
            let name = r.session.rules().techs.get(*id).map(|c| c.name.clone()).expect("a card");
            assert!(out.text.contains(&name.to_uppercase()), "{name} is on screen");
        }
        assert!(out.text.contains("Laboratory A slot"), "and says where it is: {}", out.text);
    }

    /// **An index opened over a capture that is asking about a DIFFERENT
    /// registry is a read-only view, and a pick off it names the registry the
    /// capture actually wants** (T6 review M1).
    ///
    /// The defect was two lies on one screen. `techs` typed mid-Port-reveal drew
    /// a `Recording: Port A slot …` header over twenty numbered TECHNOLOGY cards
    /// whose numbers could not answer that question, and picking one was refused
    /// with "nothing is being recorded" while a collection was live. The target
    /// is now filtered to the registry ON SCREEN, so a mismatched index carries
    /// no recording header and no selection numbers at all — and the refusal
    /// says which registry the capture is asking about.
    ///
    /// **Pinned in BOTH directions**, because the routing is per-registry: a fix
    /// that only worked one way would look identical from the side it worked on.
    #[test]
    fn a_cross_registry_index_over_a_capture_is_a_view_and_names_the_registry_asked() {
        // Direction 1: an ORDER reveal is live, and the player opens the
        // TECHNOLOGY index.
        let (mut r, _) = live_repl_at_a_two_card_port_take();
        let out = r.step("techs");
        assert!(out.text.contains("TECHNOLOGY INDEX"), "{}", out.text);
        assert!(
            !out.text.contains("Recording:"),
            "no recording header over a registry this capture cannot take: {}",
            out.text,
        );
        assert!(
            r.index_model(IndexKind::Techs).cards().all(|c| c.selection.is_none()),
            "and no selection numbers — a number here could answer nothing",
        );
        let refusal = r.step(&format!("T{:02}", an_unplaced_tech(&r).0));
        assert!(
            refusal.text.contains("order cards"),
            "the refusal names the registry the capture IS asking about: {}",
            refusal.text,
        );
        assert!(
            !refusal.text.contains("nothing is being recorded"),
            "a collection is live, so that sentence is false: {}",
            refusal.text,
        );
        assert!(r.overlay.is_some(), "the index stays up");
        assert!(matches!(r.pending, Pending::OrderReveals { .. }), "the capture is untouched");

        // Direction 2: a TECH reveal is live, and the player opens the ORDER
        // index. Same fixture shape as the routing test above, for the same
        // reason: driving a live Laboratory action to its reveal question would
        // pin the route rather than the routing.
        let (mut r, _) = repl_live_hidden(21);
        r.pending = Pending::TechReveal { mv: Move::UseUniversity, board_b: false };
        let out = r.step("orders");
        assert!(out.text.contains("ORDER CARD INDEX"), "{}", out.text);
        assert!(!out.text.contains("Recording:"), "{}", out.text);
        assert!(
            r.index_model(IndexKind::Orders).cards().all(|c| c.selection.is_none()),
            "no selection numbers on this side either",
        );
        let refusal = r.step(&format!("O{:02}", an_unplaced_order(&r).0));
        assert!(
            refusal.text.contains("technologies"),
            "the refusal names the registry the capture IS asking about: {}",
            refusal.text,
        );
        assert!(!refusal.text.contains("nothing is being recorded"), "{}", refusal.text);
        assert!(matches!(r.pending, Pending::TechReveal { .. }), "the capture is untouched");

        // And with NOTHING being collected, the message is the other one: the
        // two states are genuinely different and the screen says which it is in.
        let mut r = repl_2p(4);
        r.step("?");
        let refusal = r.step(&format!("O{:02}", an_unplaced_order(&r).0));
        assert!(
            refusal.text.contains("nothing is being recorded right now"),
            "{}",
            refusal.text,
        );
    }

    // -----------------------------------------------------------------
    // T8: the guided reveal, and the alias it keeps
    // -----------------------------------------------------------------

    /// A 3P live session with TWO seats' starting orders hidden, plus what each
    /// of them was actually dealt.
    ///
    /// Two, deliberately: with one hidden seat every wrong seat plumbing gives
    /// the right answer, so a menu that ignored the pick entirely would pass.
    fn live_3p_two_hidden(seed: u64) -> (Repl, Vec<(PlayerId, OrderCardId)>) {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(3);
        let table = new_game(config.clone(), &rules, seed).unwrap();
        let hidden =
            vec![(PlayerId::new(1), table.deals[1][0]), (PlayerId::new(2), table.deals[2][0])];
        let session = Session::new_live(
            config,
            seed,
            SetupObservations::default(),
            vec![
                StartingOrderChoice::Keep(0),
                StartingOrderChoice::Unknown,
                StartingOrderChoice::Unknown,
            ],
            rules,
        )
        .unwrap();
        (Repl::new(session), hidden)
    }

    /// A live 4-player table whose seats 2 AND 3 kept hidden starting orders —
    /// the shape where a seat's list POSITION and its NUMBER genuinely
    /// disagree, which is the only shape that can catch the trap N6 names.
    fn live_4p_two_hidden(seed: u64) -> (Repl, Vec<(PlayerId, OrderCardId)>) {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(4);
        let table = new_game(config.clone(), &rules, seed).unwrap();
        let hidden =
            vec![(PlayerId::new(2), table.deals[2][0]), (PlayerId::new(3), table.deals[3][0])];
        let session = Session::new_live(
            config,
            seed,
            SetupObservations::default(),
            vec![
                StartingOrderChoice::Keep(0),
                StartingOrderChoice::Keep(0),
                StartingOrderChoice::Unknown,
                StartingOrderChoice::Unknown,
            ],
            rules,
        )
        .unwrap();
        (Repl::new(session), hidden)
    }

    /// **The guided seat menu numbers a seat by its own identity** (T11 review
    /// N6, fixing a defect that shipped at T8).
    ///
    /// The menu numbered its rows by LIST POSITION, so at a table where the
    /// first two seats had kept their dealt cards the screen read `[1] Seat 2`
    /// and `[2] Seat 3` — and typing `2`, the number printed beside Seat 2,
    /// selected Seat 3. Every other stable-numbering surface in this driver
    /// (T7's class) numbers by the identity the thing already has, and a seat
    /// has one: `reveal 2` at the prompt and `2` at this menu now name the same
    /// seat, which is the whole content of "one grammar everywhere".
    ///
    /// Held at 4P with the first two seats visible, because that is the only
    /// arrangement where position and identity disagree — at 3P with seats 1
    /// and 2 hidden the old numbering was accidentally right, which is why the
    /// T8 tests never saw it.
    #[test]
    fn the_guided_reveal_menu_numbers_seats_by_identity_not_by_list_position() {
        let (mut r, hidden) = live_4p_two_hidden(21);
        let menu = r.step("reveal");
        assert!(menu.text.contains("[2] Seat 2"), "the number IS the seat: {}", menu.text);
        assert!(menu.text.contains("[3] Seat 3"), "{}", menu.text);
        assert!(
            !menu.text.contains("[1]") && !menu.text.contains("[0]"),
            "no row is numbered by its position in the list: {}",
            menu.text,
        );

        // And typing that number reaches THAT seat.
        r.step("2");
        let asked = crate::style::strip_ansi(&r.sub_prompt_text());
        assert!(asked.contains("Seat 2's hidden starting order"), "{asked}");
        let (seat, card) = hidden[0];
        assert_eq!(seat, PlayerId::new(2), "fixture");
        let out = r.step(&format!("O{:02}", card.0));
        assert!(out.text.contains("Seat 2 revealed"), "{}", out.text);
    }

    /// **The menu's refusals name the SEAT, in the words the `reveal` alias
    /// uses** — pinned on the parser itself and not only on the screen (T7's
    /// standing lesson: a byte-identical backstop refusal makes every wrong
    /// resolver an equivalent mutant, so the contract needs an inner pin).
    #[test]
    fn the_guided_reveal_menu_refuses_a_wrong_seat_number_by_name() {
        let seats = vec![PlayerId::new(2), PlayerId::new(3)];

        assert_eq!(reveal_seat_choice("2", &seats, 4), Ok(PlayerId::new(2)));
        assert_eq!(reveal_seat_choice("3", &seats, 4), Ok(PlayerId::new(3)));

        // A real seat whose order is already public, and a seat the table does
        // not have, are DIFFERENT facts and say so — the T8 review's NOTE-6a
        // distinction, now held on this path too.
        assert_eq!(reveal_seat_choice("1", &seats, 4), Err(seat_not_hidden(PlayerId::new(1))));
        assert_eq!(reveal_seat_choice("9", &seats, 4), Err(no_such_seat(9, 4)));
        assert!(reveal_seat_choice("9", &seats, 4).unwrap_err().contains("no Seat 9"));
        assert!(
            reveal_seat_choice("1", &seats, 4).unwrap_err().contains("Seat 1's starting order is not hidden"),
        );

        // Anything that is not a seat number at all gets the usage line, which
        // names what the menu takes.
        for tok in ["x", "", "-1", "2.0", "seat 2"] {
            let refusal = reveal_seat_choice(tok, &seats, 4).unwrap_err();
            assert!(refusal.contains("seat number"), "{tok:?}: {refusal}");
        }
    }

    /// The same refusals reach the SCREEN and leave the menu open, so a
    /// mistyped seat costs one line rather than the whole flow.
    #[test]
    fn a_wrong_seat_number_keeps_the_guided_reveal_menu_open() {
        let (mut r, _) = live_4p_two_hidden(21);
        r.step("reveal");
        let out = r.step("1");
        assert!(out.text.contains("Seat 1's starting order is not hidden"), "{}", out.text);
        assert!(
            matches!(r.sub, SubPrompt::Reveal { seat: None, .. }),
            "the menu is still up, still asking",
        );
        let out = r.step("9");
        assert!(out.text.contains("no Seat 9"), "{}", out.text);
        assert!(matches!(r.sub, SubPrompt::Reveal { seat: None, .. }));
    }

    /// **`reveal` on its own opens a numbered seat menu, and the menu path
    /// records exactly what the alias records** (spec §0.2: every decision
    /// reachable by numbered menu; today's grammars survive as expert aliases).
    ///
    /// Pinned as an EQUALITY of recorded events rather than as two independent
    /// "it worked" checks: two paths into one record is precisely the shape that
    /// drifts, and the event log is the thing a replay reads back.
    ///
    /// **Driven at the SECOND menu row of two**, which is what makes the
    /// equality bite: a guided path that ignored the pick and revealed the first
    /// hidden seat — or any hard-coded one — passes every check here except this
    /// one.
    #[test]
    fn the_guided_reveal_records_exactly_what_the_alias_records() {
        let (_, hidden) = live_3p_two_hidden(21);
        let (seat, card) = hidden[1]; // seat 2, the SECOND row of the menu

        // The alias, verbatim.
        let (mut r_alias, _) = live_3p_two_hidden(21);
        let out = r_alias.step(&format!("reveal {} O{:02}", seat.0, card.0));
        assert!(out.text.contains("revealed"), "{}", out.text);

        // The menu path: `reveal`, pick the seat by number, name the card.
        let (mut r_menu, _) = live_3p_two_hidden(21);
        let menu = r_menu.step("reveal");
        assert!(menu.text.contains("Whose starting order just became public?"), "{}", menu.text);
        assert!(
            menu.text.contains("[1] Seat 1") && menu.text.contains("[2] Seat 2"),
            "both hidden seats are numbered by their own identity (T11 N6): {}",
            menu.text,
        );
        assert!(!menu.text.contains("Seat 0"), "seat 0 kept its dealt card and is not hidden");
        r_menu.step(&seat.0.to_string());
        // The card question is the SAME composed zone the bound captures draw,
        // and it names the seat that was picked rather than the first one.
        let asked = crate::style::strip_ansi(&r_menu.sub_prompt_text());
        assert!(asked.contains("YOU ARE RECORDING"), "{asked}");
        assert!(asked.contains("Seat 2's hidden starting order"), "{asked}");
        assert!(!asked.contains("Seat 1's"), "{asked}");
        assert!(asked.contains("'cancel' backs out"), "no move is parked here: {asked}");
        let out = r_menu.step(&format!("O{:02}", card.0));
        assert!(
            out.text.contains(&format!("Seat {} revealed O{:02}", seat.0, card.0)),
            "the confirmation names the seat and the card: {}",
            out.text,
        );
        assert!(matches!(r_menu.sub, SubPrompt::None), "the flow closes on success");

        assert_eq!(
            r_menu.session.record().events,
            r_alias.session.record().events,
            "the menu path and the alias must record the same event, byte for byte",
        );
        assert_eq!(r_menu.session.state(), r_alias.session.state());
        assert!(
            r_menu.session.record().events.iter().any(|e| matches!(
                e,
                RecordedEvent::RevealStartingOrder { player, .. } if *player == seat
            )),
            "anti-vacuity: the event is really about the seat that was picked",
        );
    }

    /// The guided flow refuses rather than showing an empty list when there is
    /// nothing to reveal — every simulation, and any live game whose starting
    /// orders are all public. An empty numbered menu is the shape that makes a
    /// player wonder what they typed wrong.
    #[test]
    fn the_guided_reveal_refuses_when_no_seat_has_a_hidden_starting_order() {
        let mut r = repl_2p(4); // a simulation: nothing is hidden from the engine
        let out = r.step("reveal");
        assert!(out.text.contains("nothing to reveal"), "{}", out.text);
        assert!(matches!(r.sub, SubPrompt::None), "no sub-prompt opens");

        // A named seat that is not hidden is refused by SEAT, which is a
        // different fact and says so.
        let (mut r, _) = repl_live_hidden(21);
        let out = r.step("reveal 0");
        assert!(out.text.contains("Seat 0's starting order is not hidden"), "{}", out.text);
        assert!(matches!(r.sub, SubPrompt::None));
    }

    /// **A seat the table does not have is refused as exactly that** (T8 review,
    /// NOTE-6a).
    ///
    /// `reveal 3` at a two-player table used to answer "Seat 3's starting order
    /// is not hidden — nothing to reveal there", which is true of nothing: there
    /// is no Seat 3 to hold a starting order, hidden or otherwise. The
    /// hidden-check answers a question about a REAL seat, so the existence check
    /// has to run ahead of it — and `PlayerId::try_new` cannot do the job, since
    /// it admits 0-3 for every table size.
    #[test]
    fn reveal_refuses_a_seat_the_table_does_not_have() {
        let (mut r, _) = repl_live_hidden(21);
        assert_eq!(r.session.state().players.len(), 2, "fixture: a two-seat table");

        let out = r.step("reveal 3");
        assert!(out.text.contains("no Seat 3"), "the refusal names what is wrong: {}", out.text);
        assert!(
            !out.text.contains("not hidden"),
            "and does not answer the hidden question for a seat that does not exist: {}",
            out.text,
        );
        assert!(matches!(r.sub, SubPrompt::None), "no sub-prompt opens");

        // The seat that DOES exist and simply is not hidden keeps its own
        // wording — two facts, two messages.
        let out = r.step("reveal 0");
        assert!(out.text.contains("Seat 0's starting order is not hidden"), "{}", out.text);

        // ...and the same is true where there is nothing to reveal at all: the
        // more specific fact wins, rather than being masked by the empty-menu
        // refusal that fires first for a real seat.
        let mut sim = repl_2p(4);
        let out = sim.step("reveal 3");
        assert!(out.text.contains("no Seat 3"), "{}", out.text);
        let out = sim.step("reveal");
        assert!(out.text.contains("nothing to reveal"), "{}", out.text);
    }

    /// **A refusal leaves the guided flow OPEN, on the same question.**
    ///
    /// Found by driving the shipped binary in a PTY rather than by reading the
    /// code: the flow closed on any answer, so one mistyped card sent the
    /// operator back to the main prompt to retype `reveal` and re-pick the seat.
    /// The engine stays the authority — `apply_standalone` still runs and can
    /// still refuse — but a refusal is a re-ask, which is what every other prompt
    /// in this driver does.
    ///
    /// The card used is one the deal put in a HAND, which is the reachable way to
    /// get a refusal here: it is a real order card, and it is neither in the deck
    /// nor in the boxed pile, so it is not still hidden.
    ///
    /// **The record stream is asserted across the whole exchange** (T8 review,
    /// SHOULD-4): the refusal now happens one layer earlier, in the driver rather
    /// than in `apply_standalone`, and the thing that must not have changed is
    /// that a refused reveal writes nothing and an accepted one writes exactly
    /// one event.
    #[test]
    fn a_refusal_keeps_the_guided_reveal_open_on_the_same_question() {
        let (mut r, hidden) = repl_live_hidden(21);
        let in_a_hand = r.session.state().players[0]
            .order_cards_in_hand
            .first()
            .copied()
            .expect("seat 0 kept a starting order");
        r.step("reveal");
        r.step(&NUMBER_BASE.to_string());
        let before = r.session.record().events.len();
        let out = r.step(&format!("O{:02}", in_a_hand.0));
        assert!(
            crate::style::strip_ansi(&out.text).contains("is not an unrevealed order card"),
            "the refusal states the rule: {}",
            out.text,
        );
        assert_eq!(
            r.session.record().events.len(),
            before,
            "a refused reveal records nothing: {}",
            out.text,
        );
        assert!(
            matches!(r.sub, SubPrompt::Reveal { seat: Some(s), .. } if s == PlayerId::new(1)),
            "the flow stays open on the SAME seat, so one typo does not cost the whole entry",
        );
        assert!(
            crate::style::strip_ansi(&out.text).contains("Seat 1's hidden starting order"),
            "and the question is re-asked under the refusal: {}",
            out.text,
        );
        // ...and the correct card, typed next, still lands — and the SUCCESS
        // screen does not re-ask. That second half is its own defect and its
        // own fix: `out` appends whatever prompt is standing, so clearing the
        // flow after composing the screen printed the answered question again
        // underneath its own answer (also found in the PTY, not in the code).
        let out = r.step(&format!("O{:02}", hidden.0));
        assert!(out.text.contains("revealed"), "{}", out.text);
        assert!(matches!(r.sub, SubPrompt::None), "success closes it");
        assert!(
            !crate::style::strip_ansi(&out.text).contains("YOU ARE RECORDING"),
            "an answered question must not be re-asked under its own answer: {}",
            out.text,
        );
        assert_eq!(
            r.session.record().events.len(),
            before + 1,
            "and the whole exchange — one refusal, one answer — is exactly one recorded event",
        );
        assert!(
            matches!(
                r.session.record().events.last(),
                Some(RecordedEvent::RevealStartingOrder { player, card })
                    if *player == PlayerId::new(1) && *card == hidden
            ),
            "and it is the reveal that was accepted: {:?}",
            r.session.record().events.last(),
        );
    }

    /// **One refusal per rule: the two reveal paths turn the same card away in
    /// the same words** (T8 review, SHOULD-4).
    ///
    /// The guided flow used to offer a card at the index and then meet it with
    /// the engine's terse `Illegal: card not found`, while the bound capture
    /// stated the rule properly through `reveal::check_order_reveal` — two
    /// prompts, one rule, two answers, and the worse one on the path a player is
    /// steered onto. `commit_starting_order` now runs that same check, so the
    /// wording is single-sourced by construction.
    ///
    /// The expected text is DERIVED from the check itself rather than copied, so
    /// the pin is on the two paths agreeing, not on a literal that would have to
    /// be edited in three places.
    ///
    /// **What did NOT change is the record stream.** The check only converts a
    /// would-be-refused `apply_standalone` — which records nothing — into an
    /// earlier refusal, and the engine is still the authority behind it: this
    /// cannot pre-empt a legal answer, because `reveal_starting_order` finds a
    /// card through `extract_order_from_deck`, whose search is exactly the deck
    /// and the removed pile this check admits.
    #[test]
    fn both_reveal_paths_refuse_a_non_pool_card_in_the_same_words() {
        let (r, _) = repl_live_hidden(21);
        let in_a_hand = r.session.state().players[0]
            .order_cards_in_hand
            .first()
            .copied()
            .expect("seat 0 kept a starting order");
        let expected = crate::reveal::check_order_reveal(
            in_a_hand,
            &r.session.state().order_deck,
            &r.session.state().orders_removed,
        )
        .expect_err("fixture: a card already in a hand is in neither pile");

        // The expert alias, one line.
        let (mut r_alias, _) = repl_live_hidden(21);
        let before = r_alias.session.record().events.len();
        let alias = crate::style::strip_ansi(&r_alias.step(&format!("reveal 1 O{:02}", in_a_hand.0)).text);
        assert!(alias.contains(&expected), "the alias must state the rule: {alias}");
        assert!(!alias.contains("Illegal"), "and not the engine's terse refusal: {alias}");
        assert_eq!(r_alias.session.record().events.len(), before, "nothing recorded: {alias}");

        // The guided menu, same card, same rule.
        let (mut r_menu, _) = repl_live_hidden(21);
        r_menu.step("reveal");
        r_menu.step(&NUMBER_BASE.to_string());
        let menu = crate::style::strip_ansi(&r_menu.step(&format!("O{:02}", in_a_hand.0)).text);
        assert!(menu.contains(&expected), "the guided path must state the same rule: {menu}");
        assert_eq!(r_menu.session.record().events.len(), before, "nothing recorded: {menu}");
    }

    /// **The index answers the guided reveal too** (spec §0.6: the index is
    /// available inside input-capture prompts). Without the routing this pins,
    /// `?` here would open a read-only order index whose numbers select nothing
    /// and whose pick is refused with "nothing is being recorded" — false, since
    /// a capture is open, which is the T6 review's finding at a new prompt.
    #[test]
    fn the_index_inside_the_guided_reveal_answers_it() {
        let (mut r, hidden) = repl_live_hidden(21);
        r.step("reveal");
        r.step(&NUMBER_BASE.to_string());
        let out = r.step("?");
        assert!(out.text.contains("ORDER CARD INDEX"), "{}", out.text);
        assert!(
            out.text.contains("Recording: Seat 1's hidden starting order"),
            "in RECORDING context, naming what it is collecting: {}",
            out.text,
        );
        let number = r
            .index_model(IndexKind::Orders)
            .cards()
            .find(|c| c.face.card_ref() == Some(CardRef::Order(hidden)))
            .and_then(|c| c.selection)
            .expect("an unplaced card is selectable on a recording screen");
        let out = r.step(&number.to_string());
        let plain = crate::style::strip_ansi(&out.text);
        assert!(plain.contains(&format!("O{:02}", hidden.0)), "echoed in full: {plain}");
        assert!(out.text.contains("revealed"), "{}", out.text);
        // ...and DRAWN once. Both this hand-off and `commit_starting_order` can
        // draw the card, and each doing so prints the whole face twice on one
        // screen — which is what the sibling arms above look like if the
        // asymmetry between the two capture paths is not noticed (the bound
        // reveal's collector draws nothing, so that one adds the echo itself).
        // Counted over CARD FACES rather than over mentions of the id, since the
        // confirmation sentence and the delta line legitimately name it too.
        let drawn = plain
            .lines()
            .filter(|l| l.starts_with('┌') && l.contains(&format!("O{:02}", hidden.0)))
            .count();
        assert_eq!(drawn, 1, "the card is drawn exactly once:\n{plain}");
        assert!(r.overlay.is_none(), "the index closes on a selection");
        assert!(matches!(r.sub, SubPrompt::None), "and the flow closes with it");
        assert!(
            r.session.record().events.iter().any(|e| matches!(
                e,
                RecordedEvent::RevealStartingOrder { player, card }
                    if *player == PlayerId::new(1) && *card == hidden
            )),
            "the pick reached the engine as the reveal",
        );
    }

    /// A technology no face-up location on screen already claims, so a pick of
    /// it is refused for the reason under test rather than as a duplicate.
    fn an_unplaced_tech(r: &Repl) -> TechCardId {
        let placed = r.index_placements();
        r.session
            .rules()
            .techs
            .iter()
            .map(|c| c.id)
            .find(|id| !placed.iter().any(|(card, _)| *card == CardRef::Tech(*id)))
            .expect("the registry is larger than the face-up cards")
    }

    /// The order-card twin of [`an_unplaced_tech`].
    fn an_unplaced_order(r: &Repl) -> OrderCardId {
        let placed = r.index_placements();
        r.session
            .rules()
            .orders
            .iter()
            .map(|c| c.id)
            .find(|id| !placed.iter().any(|(card, _)| *card == CardRef::Order(*id)))
            .expect("the registry is larger than the face-up cards")
    }

    /// **A NUMBER typed at an open view closes it and APPLIES that option**
    /// (T6 review M2a) — the load-bearing half of the re-dispatch rule, and the
    /// half nothing pinned: the existing tests covered Enter (which must not
    /// apply) and a read-only command (`score`), so the one input that moves the
    /// game from a screen the player opened to LOOK at something was untested.
    #[test]
    fn a_number_typed_at_a_view_closes_it_and_applies_that_option() {
        let mut r = repl_2p(4);
        assert_eq!(r.display_options().len(), 1, "fixture: a single-option prompt");
        r.step("tracks");
        assert!(r.overlay.is_some(), "the view is up");
        let before = r.session.record().events.len();

        let out = r.step("1");
        assert!(r.overlay.is_none(), "the number closed the view");
        assert_eq!(
            r.session.record().events.len(),
            before + 1,
            "and applied option 1: {}",
            out.text,
        );
        assert_eq!(r.session.state().turn_phase, TurnPhase::Placement, "the game moved");
        assert!(
            !out.text.contains("TRACKS — full view"),
            "the view is gone from the frame: {}",
            out.text,
        );
        // Anchored to the WHOLE delta line, not to the bare word: the phase
        // chain prints a dim title-case `Placement` on every frame where
        // Placement is not the current phase, so `contains("Placement")` alone
        // would survive the delta line disappearing entirely -- the
        // overlapping-text trap this branch has hit repeatedly. It passes today
        // only because the chain uppercases the CURRENT phase.
        assert!(
            out.text.contains("Phase: Additional actions (before) -> Placement"),
            "and the frame that comes back shows the delta: {}",
            out.text,
        );
    }

    /// **The two overlays diverge on unrecognized input, deliberately** (T6
    /// review M2b), and this test names the divergence so a refactor can neither
    /// unify them nor invert them silently.
    ///
    /// A VIEW selects nothing, so a line typed at one is a line the player wants
    /// RUN: it closes and re-dispatches, and junk reaches the main prompt's own
    /// `Unrecognized input`. An INDEX is an input surface whose numbers select,
    /// so a mistyped line has to be REFUSED in the index's own grammar with the
    /// screen still up — re-dispatching there would answer a question the player
    /// can no longer see.
    #[test]
    fn a_view_re_dispatches_the_junk_an_index_refuses() {
        let mut r = repl_2p(4);
        r.step("tracks");
        let out = r.step("zzz");
        assert!(r.overlay.is_none(), "the view closed");
        assert!(
            out.text.contains("Unrecognized input"),
            "and the line ran at the prompt underneath: {}",
            out.text,
        );

        let mut r = repl_2p(4);
        r.step("?");
        let out = r.step("zzz");
        assert!(r.overlay.is_some(), "the index RETAINS the screen");
        assert!(
            !out.text.contains("Unrecognized input"),
            "nothing was re-dispatched: {}",
            out.text,
        );
        assert!(out.text.contains("is not a card"), "it is refused in the index's grammar: {}", out.text);
        assert!(out.text.contains("ORDER CARD INDEX"), "with the screen still drawn: {}", out.text);
    }

    /// **`back` is asymmetric across the two overlays, and that is the ruling**
    /// (T6 review S1).
    ///
    /// In a VIEW it is not a close word: it re-dispatches like any other line,
    /// so it does what the standing hint says `back` does — it takes the last
    /// applied entry back. Treating it as a second spelling of `cancel` there
    /// made the hint false on the one screen a player is most likely to be
    /// reading it from.
    ///
    /// In the INDEX it stays a close word, because that surface's numbers SELECT
    /// and `back` is part of its own card-entry grammar; the exit line names it
    /// rather than leaving a player to find out which of the two it is.
    #[test]
    fn back_undoes_from_a_view_and_closes_the_index() {
        let mut r = repl_2p(4);
        r.step("1");
        assert_eq!(r.session.record().events.len(), 1, "fixture: one entry to take back");
        r.step("tracks");
        let out = r.step("back");
        assert!(r.overlay.is_none(), "the view closed");
        assert_eq!(
            r.session.record().events.len(),
            0,
            "and 'back' genuinely undid the entry: {}",
            out.text,
        );

        let mut r = repl_2p(4);
        r.step("1");
        r.step("?");
        let out = r.step("back");
        assert!(r.overlay.is_none(), "the index closed");
        assert_eq!(
            r.session.record().events.len(),
            1,
            "and nothing was undone: {}",
            out.text,
        );
        assert!(
            r.index_text(IndexKind::Orders).contains("'back'"),
            "and the index's exit line names it: {}",
            r.index_text(IndexKind::Orders),
        );
    }

    /// **The two views are one action away and return with the prompt intact**
    /// (spec §0.5) — including from inside a Movement sub-prompt, which is the
    /// clause `sites` exists for: it is the Movement companion, and a route pick
    /// would otherwise swallow the word as a bad number.
    #[test]
    fn a_view_returns_to_the_prompt_it_interrupted() {
        for (word, title) in [("tracks", "TRACKS — full view"), ("sites", "BUILDING SITES — full view")] {
            // From the main prompt.
            let mut r = repl_2p(4);
            let before = r.prompt();
            let out = r.step(word);
            assert!(out.text.contains(title), "'{word}' opens its view: {}", out.text);
            assert!(r.overlay.is_some(), "the overlay is up");
            let back = r.step("");
            assert!(back.text.contains(&before), "Enter returns to the same prompt: {}", back.text);
            assert!(r.overlay.is_none(), "and the overlay is down");

            // From inside a Movement sub-prompt, with the sub-prompt untouched.
            let mut r = movement_grouping_fixture();
            r.step("3"); // the guided retreat
            assert!(matches!(r.sub, SubPrompt::Retreat { .. }));
            let asked = r.sub_prompt_text();
            let out = r.step(word);
            assert!(out.text.contains(title), "'{word}' opens from a sub-prompt: {}", out.text);
            assert!(matches!(r.sub, SubPrompt::Retreat { .. }), "the sub-prompt is untouched");
            let back = r.step(word); // the key that opened it closes it
            assert!(back.text.contains(&asked), "back at the same question: {}", back.text);
            assert!(matches!(r.sub, SubPrompt::Retreat { .. }));
        }
    }

    /// A view applies nothing and records nothing, however it is left — the
    /// read-only half of the return contract.
    #[test]
    fn a_view_is_read_only() {
        let mut r = repl_2p(4);
        let events = r.session.record().events.len();
        let state = r.session.state().clone();
        r.step("sites");
        r.step("tracks");
        r.step("cancel");
        assert_eq!(r.session.record().events.len(), events, "a view records nothing");
        assert_eq!(*r.session.state(), state, "a view applies nothing");
    }

    /// **Bare Enter closes a view WITHOUT re-dispatching**, and the exclusion is
    /// load-bearing rather than tidy: at a single-option prompt bare Enter
    /// applies that option (the ceremony rule), so a blank line re-dispatched
    /// out of a view would apply a move the player pressed Enter to stop looking
    /// at a screen. Every OTHER line closes the view and runs, which is what
    /// makes §0.5's "one action away" cost one action rather than two.
    #[test]
    fn enter_closes_a_view_without_applying_the_forced_option_underneath() {
        let mut r = repl_2p(4);
        assert_eq!(r.display_options().len(), 1, "fixture: a single-option prompt");
        r.step("tracks");
        let events = r.session.record().events.len();
        r.step("");
        assert_eq!(
            r.session.record().events.len(),
            events,
            "Enter left the view; it must not also apply the forced option",
        );
        assert!(r.overlay.is_none(), "but it did leave the view");

        // ...and a real command typed at a view runs, having closed it.
        r.step("sites");
        let out = r.step("score");
        assert!(r.overlay.is_none(), "the view closed");
        assert!(out.text.contains("Score"), "and the command ran: {}", out.text);
    }

    /// The three commands the T6 surfaces supersede open the NEW surfaces, and
    /// the `view` menu dispatches to the same code path — the property
    /// `VIEW_MENU`'s tuple shape exists to keep true.
    #[test]
    fn the_superseded_commands_open_the_new_surfaces() {
        let mut r = repl_2p(4);
        for (word, title) in [
            ("tracks", "TRACKS — full view"),
            ("sites", "BUILDING SITES — full view"),
            ("goals", "ACHIEVEMENTS"),
            ("techs", "TECHNOLOGY INDEX"),
            ("sitecards", "BUILDING-SITE CARD INDEX"),
            ("orders", "ORDER CARD INDEX"),
        ] {
            let out = r.step(word);
            assert!(out.text.contains(title), "'{word}' must open {title}: {}", out.text);
            r.overlay = None;
        }
    }

    /// The index is a FULL-SCREEN surface: the composed frame draws it alone,
    /// with no board or dashboard above it (spec §0.5). Without this the 45-row
    /// index under a 48-row frame would scroll both away on any real terminal.
    #[test]
    fn the_index_replaces_the_frame_rather_than_sitting_under_it() {
        let mut r = repl_2p(4);
        let out = r.step("?");
        screen::force(true);
        let framed = screen::wrap(&r, &out.text);
        screen::force(false);
        assert!(framed.starts_with("\u{1b}[2J\u{1b}[H"), "still clears the screen first");
        assert!(framed.contains("ORDER CARD INDEX"), "{framed:.200}");
        assert!(!framed.contains("YOKOHAMA"), "no title strip over a full-screen surface");
        assert!(!framed.contains("PLAYERS"), "no dashboard: {framed:.400}");
        assert!(!framed.contains("info: type 'view'"), "no info strip");
        assert!(framed.lines().count() < 50, "the whole surface fits the design floor's rows");
    }

    /// `orders` and the `view` menu's first row reach the same screen `?` does
    /// — three doors, one surface.
    #[test]
    fn the_index_has_three_doors() {
        let by_key = { let mut r = repl_2p(4); r.step("?").text };
        let by_command = { let mut r = repl_2p(4); r.step("orders").text };
        let by_menu = {
            let mut r = repl_2p(4);
            // Found by its LABEL, not by a remembered position: T11 re-ordered
            // the roster so the views lead and the indexes follow, and a test
            // keyed on `1` would have started asserting about the Port offer
            // while still claiming to be about the index.
            let row = Repl::VIEW_MENU
                .iter()
                .position(|(label, _)| label.starts_with("Order card index"))
                .expect("the roster carries the order index");
            r.step("view");
            r.step(&crate::screen_model::menu_number(row).to_string()).text
        };
        assert!(by_key.contains("ORDER CARD INDEX"));
        assert_eq!(by_key, by_command, "'orders' is the same screen as '?'");
        assert_eq!(by_key, by_menu, "so is its menu row");
    }

    /// **The numbering base is ONE base** (T5's ruling): every numbered surface
    /// in the driver starts its list at `NUMBER_BASE`, and the sweep is over
    /// the surfaces rather than over a list somebody remembered to update.
    ///
    /// A `[0]` anywhere in one of these screens is the per-surface drift the
    /// ruling exists to prevent — a player who learns that the first option is
    /// `[1]` will type `1` into the next list too.
    ///
    /// **The WIZARD's listings are swept by its own half of this test**
    /// (`wizard::tests::wizard_listings_start_at_the_same_base_and_identities_are_left_alone`,
    /// T7): they need a scripted setup run to exist, which does not belong in a
    /// REPL fixture, and the tile/token/achievement menus are the surfaces the
    /// F3 report was actually about.
    #[test]
    fn every_numbered_surface_starts_at_the_same_base() {
        let first = format!("[{NUMBER_BASE}]");

        // The main decision zone.
        let mut r = repl_2p(4);
        assert!(r.prompt().contains(&first), "main options: {}", r.prompt());

        // The view menu.
        r.step("view");
        let menu = r.overlay_text();
        // Read against the menu's own first ROW rather than a remembered label,
        // so re-ordering the roster (T11 did) re-points nothing here.
        let first_label = Repl::VIEW_MENU[0].0;
        assert!(menu.contains(&format!("{first}  {first_label}")), "view menu: {menu}");
        assert!(!menu.contains("[0]"), "and nothing numbers from zero: {menu}");
        r.step("cancel");

        // The card index.
        let index = r.index_model(IndexKind::Orders);
        assert!(index.cards().all(|c| c.selection.is_none()), "a standing view has no numbers");
        let recording = crate::screen_model::build_order_index(
            r.session.rules(),
            &[],
            Some(RecordingTarget::PortSetup { board_b: false, slot: 0, of: 6 }),
            None,
        );
        assert_eq!(
            recording.cards().next().and_then(|c| c.selection),
            Some(NUMBER_BASE),
            "the index numbers from the base too",
        );

        // A route sub-prompt.
        let mut r = movement_grouping_fixture();
        r.step("1");
        let pick = r.sub_prompt_text();
        assert!(pick.contains(&first), "route sub-prompt: {pick}");

        // And nothing anywhere shows a zero-th entry.
        for screen in [r.sub_prompt_text(), repl_2p(4).prompt()] {
            assert!(!screen.contains("[0]"), "a zero-based list survived: {screen}");
        }
    }

    /// The two things the base ruling deliberately does NOT move, pinned so a
    /// future sweep cannot "fix" them: a COUNT (zero is a real answer) and an
    /// engine ID (printed on the component, recorded in the save).
    #[test]
    fn counts_and_ids_are_not_menu_numbers() {
        let mut r = movement_grouping_fixture();
        r.step("3"); // the guided retreat
        let q = r.sub_prompt_text();
        assert!(q.contains("[0-"), "a recovery question still offers zero: {q}");
        let out = r.step("0");
        assert!(!out.text.contains("!"), "zero is a valid recovery answer: {}", out.text);

        // Area ids in the Option-B picker are ids, not positions.
        let mut r = repl_2p(4);
        r.step("1");
        let picker = r.step("2").text;
        assert!(picker.contains("(by id)"), "{picker}");
        assert!(!picker.contains("[1] A"), "the eligible areas carry no list numbers: {picker}");
    }

    /// A two-card Port take in live mode, parked at the first reveal question.
    /// Returns the REPL and a card that is genuinely in the unrevealed deck,
    /// so a test can answer the question truthfully.
    fn live_repl_at_a_two_card_port_take() -> (Repl, OrderCardId) {
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
            config,
            901,
            obs,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown],
            rules,
        )
        .unwrap();
        let mut r = Repl::new(session);
        let deck_card = *r.session.state().order_deck.first().expect("an unrevealed card");

        let port_area = *r
            .session
            .state()
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type == AreaType::PortA)
            .unwrap()
            .0;
        r.step("1"); // end the additional phase
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
            .position(|m| matches!(m, Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } } if cards_taken.len() == 2))
            .expect("a 2-card port take is available");
        r.step(&menu_number(take).to_string());
        assert!(matches!(r.pending, Pending::OrderReveals { .. }), "parked at a reveal");
        (r, deck_card)
    }

    // -----------------------------------------------------------------------
    // The areas view (branch-4 T5)
    // -----------------------------------------------------------------------

    /// **`areas` is a full-screen surface like the other six** — it opens over
    /// whatever is up, the key that opened it closes it again, and the `view`
    /// menu reaches the same code path the word does.
    #[test]
    fn areas_opens_and_toggles_like_the_other_surfaces() {
        let mut r = repl_2p(4);
        let out = r.step("areas");
        assert!(r.full_screen_surface(), "'areas' opens a full-screen view");
        assert!(
            !out.text.contains("'help' for commands)"),
            "a view, not a framed body: {}",
            out.text,
        );
        let shown = r.overlay_text();
        assert!(shown.contains("AREAS — full view   "), "{shown:.200}");
        assert!(
            shown.contains("closes this view and puts you back at the prompt underneath"),
            "it states its own way out: {shown:.400}",
        );

        // The key that opened it closes it — the toggle every view holds.
        r.step("areas");
        assert!(!r.full_screen_surface(), "the second press closed it");

        // ...and the menu is the other door onto the same surface.
        let row = Repl::VIEW_MENU
            .iter()
            .position(|(label, _)| label.starts_with("Areas"))
            .expect("the roster carries the areas reference");
        r.step("view");
        let by_menu = r.step(&crate::screen_model::menu_number(row).to_string());
        assert!(by_menu.text.is_empty() || r.full_screen_surface(), "the row opened a surface");
        assert!(r.overlay_text().contains("AREAS — full view"), "{}", r.overlay_text());
    }

    /// `help` documents the word, which is also what `guide_sync` reads it
    /// through to hold both guide formats to the same roster.
    #[test]
    fn help_documents_the_areas_view() {
        assert!(HELP_TEXT.contains("\n  areas "), "the help roster lists it: {HELP_TEXT}");
        assert!(
            HELP_TEXT.lines().any(|l| l.trim_start().starts_with("areas ") && l.contains("AREAS")),
            "and its row says what the surface is",
        );
    }
}
