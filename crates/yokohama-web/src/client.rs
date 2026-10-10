//! What a browser receives: the frame, plus what it is being asked.
//!
//! Everything here is a projection. The frame is `screen_model::build_frame`'s,
//! the words are `describe`'s, and the numbered entries are the REPL's own —
//! see [`PromptState`] on why that last one is not negotiable.

use serde::Serialize;
use yokohama_cli::repl::{Repl, SubPrompt};
use yokohama_cli::screen_model::{self, FrameModel, SeatFacts, TitleMode, TitleModel};
use yokohama_cli::{describe, session::TrackingMode};
use yokohama_cli::session::Session;
use yokohama_core::{AreaId, GameState, PlayerId};

use crate::identity::User;
use crate::table::TableDoc;

/// One numbered entry, exactly as the engine-backed list produced it.
#[derive(Debug, Clone, Serialize)]
pub struct OptionRow {
    /// What to send back. 1-based (`screen_model::NUMBER_BASE`).
    pub number: usize,
    /// The entry in plain words, from the CLI's own renderer.
    pub words: String,
    /// True when choosing this opens a further question (a route family, the
    /// retreat family, an Option A/B placement) instead of applying a move.
    /// The client can say "…" rather than implying the turn is about to end.
    pub opens_picker: bool,
    /// **What this row COSTS**, priced by `screen_model::option_fee`.
    ///
    /// `words` is `describe::describe_move`'s, and for a President move that is
    /// a bare "Place President from hand in Copper Mine (A00)" with no price on
    /// it — the terminal's decision zone does not use those words for this move
    /// kind, it re-words the row from `OptionModel::fee`
    /// (`decision_render::move_row`). Without this the browser's rows were
    /// identical in shape whether they cost a yen or nothing, so a player
    /// comparing destinations could not see which one charged them.
    ///
    /// `None` means the move cannot carry a fee at all. A `Some` whose `total`
    /// is 0 means FREE and is not the same thing: `option_fee` returns a zero
    /// fee for every President move, free or not, and the terminal prints
    /// `— free` there because a player choosing between routes is comparing
    /// prices and an absent price reads as an unknown one.
    pub fee: Option<screen_model::FeeModel>,
}

/// What the game is asking for right now.
///
/// **The numbers in `Options` are the REPL's grouped entries, not one per legal
/// move.** Movement collapses to one row per destination with the retreat
/// family folded into a single row; Placement collapses to Option A / Option B
/// / place-none; a queued assistant-move groups by source. A client that
/// numbered `FrameModel::decision::options` — which IS one per legal move —
/// would send back a number naming a different move, and in Movement it would
/// usually still be a legal one, so the game would quietly diverge from what
/// the player clicked. `Repl::option_rows` is the authority and this carries it.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PromptState {
    /// The main decision.
    Options { rows: Vec<OptionRow> },
    /// A numbered sub-list opened by an entry above.
    Pick { title: String, rows: Vec<OptionRow> },
    /// Guided retreat: how many assistants to recover from one area, asked once
    /// per occupied area. `0` is a legal answer meaning "none".
    Retreat {
        area: AreaId,
        area_label: String,
        max: u8,
        answered: Vec<u8>,
        remaining: usize,
    },
    /// Placement Option A: toggle up to `max_areas` areas, then `confirm`.
    ///
    /// `fee` is what the CURRENT selection would cost, live as it changes —
    /// 1 yen per assistant placed, to each president owner in that area.
    /// Without it the browser showed only the ids it had toggled, and a player
    /// pressed Confirm and paid a yen that no surface had stated. The terminal
    /// recomputes this on every redraw (`Repl::sub_prompt_text`).
    PlaceA { selected: Vec<AreaId>, max_areas: usize, fee: Option<screen_model::FeeModel> },
    /// Placement Option B: pick one area for two assistants, then `confirm`.
    ///
    /// Option B places TWO assistants in one area, so its fee is two yen per
    /// president owner there, not one — which is exactly the kind of thing a
    /// player should not have to remember while choosing.
    PlaceB {
        areas: Vec<AreaId>,
        selected: Option<AreaId>,
        fee: Option<screen_model::FeeModel>,
    },
    /// A live-mode card reveal. v1 does not create live games, so this is here
    /// to be reported honestly rather than rendered.
    Reveal,
    /// Nothing is being decided.
    GameOver,
    /// Someone else is to move and this payload is not theirs.
    ///
    /// **The prompt has to be scoped or the frame's scoping is a formality.**
    /// `ClientState::of` builds the decision zone through
    /// `build_decision_for`, whose doc says an observer must not receive the
    /// acting seat's options because "a `FulfillOrder` option names a card the
    /// reader may not be entitled to see" — and then the sibling `prompt` field
    /// was built from `repl.option_rows()`, which is always the CURRENT
    /// player's grouped list. The frame said `options: []` and
    /// `viewer_is_deciding: false` while the prompt beside it carried the
    /// actor's entries.
    ///
    /// Reached whenever the caller's seat is not the acting one ([`viewer_seat`]):
    /// another person is to move, or a bot is. The acting seat's hand is masked
    /// from this payload — so without this the client would hold an option list
    /// derived from a hand the same payload hides.
    Watching { to_move: PlayerId },
    /// The session is locked after a driver or engine fault and accepts no
    /// gameplay at all.
    ///
    /// **It used to report a live `Options` prompt.** `Repl::step` under
    /// lockdown accepts only `save`/`log`/`quit` — every one of which
    /// `grammar` refuses — so every command that could work was fenced off and
    /// every command the client could send was dead, while the payload
    /// carried numbered rows and `your_turn: true`. Measured: a record with a
    /// `lockdown` field answered `prompt.kind: "options"` with one row, and
    /// posting that row returned 200 with the locked refusal buried in
    /// `messages` and the same live prompt returned, forever.
    Locked { reason: String },
}

/// One seat's final score.
#[derive(Debug, Clone, Serialize)]
pub struct ScoreRow {
    pub seat: PlayerId,
    pub total: u32,
}

/// The whole payload for one game.
#[derive(Debug, Clone, Serialize)]
pub struct ClientState {
    pub id: String,
    /// `"sim"` or `"live"`.
    pub mode: &'static str,
    /// The seat this payload was built FOR — whose hand is unmasked and whose
    /// decision zone carries options. Not necessarily the seat to move.
    ///
    /// **`null` for a SPECTATOR** (M6.3, R-3.7): a watcher holds no seat, and
    /// answering with the seat the frame happened to be built for would name a
    /// seat the reader has no claim on. A seated payload is byte-identical to
    /// what it was — `Some(n)` serializes as `n`.
    pub seat: Option<PlayerId>,
    /// Whose turn it is.
    pub to_move: PlayerId,
    /// True when `seat` is the seat to move.
    pub your_turn: bool,
    pub frame: FrameModel,
    pub prompt: PromptState,
    /// What the last input produced: the delta summary, or a refusal.
    pub messages: Vec<String>,
    pub over: bool,
    /// The end-trigger strip, worded by the CLI's own `render::end_trigger_line`.
    ///
    /// **A rendered string on purpose, and the one place this crate serves
    /// one.** Every end-game trigger's proximity in a line — `end triggers:
    /// deck 5 · church 4/4 · customs 1/4 · shophouses 2/8 · trade houses 1/4 ·
    /// FINAL ROUND`. Three of those numbers are in the frame already
    /// (`trigger_at`, `church_filled`, `customs_filled`) and two are added
    /// below, but the THRESHOLDS 8 and 4 are literals inside the engine's
    /// `end_game.rs` with no name to import, so a client composing this itself
    /// would be writing two rules constants into a stylesheet. Calling the
    /// CLI's renderer instead means the browser and the terminal cannot
    /// disagree about how close the game is to ending, which is the whole point
    /// of the parity work. The line is plain by construction — its own doc says
    /// so — and this process never initialises the ANSI layer.
    pub end_triggers: String,
    /// Cards left in the order deck. In no frame model, and one of the two
    /// facts the end-trigger strip needs that a frame cannot supply.
    ///
    /// A public count of a hidden pile, and legitimately so: the physical
    /// deck's HEIGHT is on the table for everyone. Identities stay in
    /// `GameState` and reach no model.
    pub deck_remaining: usize,
    /// Whether a trigger has already fired, so the last round is running.
    pub end_game_triggered: bool,
    pub scores: Option<Vec<ScoreRow>>,
    /// Provenance drift noticed when the game was loaded.
    pub warnings: Vec<String>,
    /// The record's length — how many events this game has applied.
    ///
    /// A client that answered from a payload carrying `events: n` and gets
    /// back `events: m > n` knows the reply applied the events `[n, m)`,
    /// which is exactly the run the turn playback walks (M5.5 spec §3.2.2). A
    /// count and not an event list: the walk reads the positions from
    /// `/at/{k}` and the words from `/log`, both of which already exist.
    pub events: usize,
    /// Every bot seat the RECORD names, labelled (M6.5c2, R-6.5c2.10) — empty
    /// for an all-human game. From the record's `run.seats`, not from the
    /// resident `Repl`'s bot table.
    pub bots: Vec<crate::bots::SeatBot>,
}

impl PromptState {
    /// The tag a client echoes back to say which prompt it was answering.
    ///
    /// The same string serde puts in `kind`, restated here so the client has a
    /// value to send rather than having to parse its own payload back.
    pub fn kind(&self) -> &'static str {
        match self {
            PromptState::Options { .. } => "options",
            PromptState::Pick { .. } => "pick",
            PromptState::Retreat { .. } => "retreat",
            PromptState::PlaceA { .. } => "place_a",
            PromptState::PlaceB { .. } => "place_b",
            PromptState::Reveal => "reveal",
            PromptState::GameOver => "game_over",
            PromptState::Locked { .. } => "locked",
            PromptState::Watching { .. } => "watching",
        }
    }
}

/// The seat a payload is built FOR (spec §5.1): the acting seat when the caller
/// holds it, else the caller's lowest seat, else `None` — which the route turns
/// into a 403 (R-1.5).
///
/// **It replaces two functions, and the pair is worth naming.** `viewing_seat`
/// followed the MOVER, because before M6.1 nobody was asking: one browser held
/// every seat, and the only honest answer to "whose payload is this" was "the
/// one about to play". `replay_seat` existed because that answer re-pointed
/// every turn and `/at/{k}` needed a pin that did not — the lowest non-bot seat,
/// stable for the life of the game. An AUTHENTICATED route needs neither: the
/// stable seat is the caller's own, so one function answers both routes and the
/// replay's pin is now per caller rather than per game.
///
/// **The acting-seat arm is what makes a hot seat playable.** One person in
/// several seats gets the seat that is to move, so the screen follows the turn
/// around the table exactly as it did before there were identities; a person
/// who holds no acting seat gets their lowest, and watches.
pub fn viewer_seat(table: &TableDoc, user: &User, state: &GameState) -> Option<PlayerId> {
    let mine = table.seats_of(user);
    if mine.contains(&state.current_player) {
        return Some(state.current_player);
    }
    mine.first().copied()
}

/// WHO a payload is built for: a person in a seat, or somebody watching.
///
/// **The `Option<PlayerId>` this replaces would have been the same two cases
/// with no name on either**, and every builder downstream would have had to
/// decide for itself what `None` meant. Named, the spectator's rules live in
/// one place: every hand hidden, no options, `seat: null` on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Viewer {
    Seat(PlayerId),
    /// A signed-in caller with no seat, admitted at a playing or finished table
    /// (M6.3, R-3.5). Every hand hidden, no options, `seat: null` on the wire.
    Spectator,
}

/// The seat a spectator's frame is BUILT for (spec §4): any seat that is not
/// the acting one — at two, three and four players there is always one.
///
/// **Nothing leaks through the choice, and that is why `build_frame` is not
/// widened** (R-3.8). `build_dashboard` reads `seat` for exactly one thing, its
/// own hand panel, which [`facts_for`] hides and `drop_hand_for` then removes
/// entirely; and `build_decision_for` returns `viewer_is_deciding: false` over
/// an empty option list for any seat that is not acting. So the choice decides
/// nothing a spectator can see — which is what makes it a definition rather
/// than a stand-in with a secret.
pub fn stand_in(state: &GameState) -> PlayerId {
    PlayerId::new((state.current_player.0 + 1) % state.players.len() as u8)
}

/// R-1.7: a hand is shown to its own seat only, bots included, hot-seat
/// included. The other seats show as counts, which is what `hand_hidden` means
/// to every panel builder downstream. **A spectator's payload hides every one
/// of them** (M6.3, R-3.7): a watcher holds no seat, so no hand is theirs.
///
/// **This is the SERVER's answer to `build_frame`'s `seats`, and it replaces
/// `Repl::seat_facts()` in this crate.** That method folds the REPL's `privacy`
/// flag — one operator's toggle at one terminal, which hides every hand at once
/// so a person can hand the keyboard over. A per-seat payload has no use for
/// it: each seat already sees only its own, always, and there is nobody to hand
/// the screen to.
///
/// **`is_bot` is the RECORD's answer since M6.5c2** (R-6.5c2.10; spec §2.4's
/// fourth mask), and this re-rules the sentence that stood here — "`is_bot` is
/// still the REPL's own reconciled answer ([`Repl::bots`]), never re-derived
/// here" — by name. The record's `run.seats` is the source `ClientState.bots`
/// reads too, so the panel's badge and the payload's labels cannot disagree.
/// They differ from the `Repl`'s answer in exactly one place: a FINISHED game
/// played against a bot this build does not run, whose seat `Repl::new`
/// (`BotSeats::from_provenance`) has turned into a human one inside the
/// resident `Repl`, and whose Players panel must still say a bot played it.
/// Everywhere else the two agree, because every bot the web deals is
/// `PROVISIONAL_DEFAULT_BOT`, which `from_provenance` restores.
pub fn facts_for(repl: &Repl, viewer: Viewer) -> Vec<SeatFacts> {
    let bots: Vec<u8> =
        crate::bots::recorded(repl.session.record()).into_iter().map(|(seat, _)| seat).collect();
    repl.session
        .state()
        .players
        .iter()
        .map(|p| SeatFacts {
            is_bot: bots.contains(&p.id.0),
            hand_hidden: match viewer {
                Viewer::Seat(s) => p.id != s,
                Viewer::Spectator => true,
            },
        })
        .collect()
}

/// `warnings` less the line `Repl::new` added for each seat it turned from a
/// FOREIGN bot into a human (M6.5c2, R-6.5c2.10; spec §2.4's first mask) —
/// `warning: seat N was played by controller "…", … it resumes as a HUMAN
/// seat, so you are now playing it`, which on the web is false: nothing here
/// converts a bot's seat, and nobody is now playing it.
///
/// **And the same line for a seat a person played on BGA** (M6.5c2 cycle D,
/// straw poll row 144): the CLI cannot parse `bga-human` as a bot, so it
/// resumes that seat as a human one and says so — true of a person, and false
/// on the web in the same way, since nobody here took their seat over. Once
/// [`crate::bots::recorded`] stopped counting such a seat as a bot it left
/// [`crate::bots::foreign`], so it is named here from
/// [`crate::bots::bga_human_seats`] instead.
///
/// **By the CLI's own prefix** (`bot.rs`, `BotSeats::from_provenance`'s two
/// conversion arms), per seat and naming its controller, so no other
/// warning can match. `Session::load_warnings` is a `Vec<String>` and a typed
/// alternative would need a CLI change, so the cells that pin this assert the
/// WHOLE array of a finished foreign game and of a `bga-human` one — empty —
/// and fail loudly the day the CLI's wording moves, rather than passing by
/// matching the prefix itself.
pub(crate) fn without_foreign_conversion(
    warnings: Vec<String>,
    record: &yokohama_core::GameRecord,
) -> Vec<String> {
    let bga = yokohama_cli::bga::CONTROLLER_BGA_HUMAN;
    let converted: Vec<String> = crate::bots::foreign(record)
        .into_iter()
        .map(|(seat, d, _)| (seat, d.controller))
        .chain(crate::bots::bga_human_seats(record).into_iter().map(|seat| (seat, bga.to_string())))
        .map(|(seat, controller)| format!("warning: seat {seat} was played by controller \"{controller}\""))
        .collect();
    warnings.into_iter().filter(|w| !converted.iter().any(|p| w.starts_with(p.as_str()))).collect()
}

/// The `PlayerId` the CLI's builders are handed for this viewer.
fn built_for(state: &GameState, viewer: Viewer) -> PlayerId {
    match viewer {
        Viewer::Seat(s) => s,
        Viewer::Spectator => stand_in(state),
    }
}

/// **A spectator's dashboard carries no `Hand` panel at all.**
///
/// The facts hide every hand, but hiding is not removing: `build_hand` returns
/// `Some(HandPanel { hidden: true, cards: [], card_count })` for the seat the
/// frame was built for, so a spectator's screen would draw one seat's masked
/// hand under the caption every other payload puts the READER's hand under —
/// singling out a seat as "yours" that is nobody's. The other panels are
/// untouched, because the Players panel already carries every seat's masked
/// count and that is the honest picture.
fn drop_hand_for(viewer: Viewer, frame: &mut FrameModel) {
    if matches!(viewer, Viewer::Spectator) {
        frame.dashboard.retain(|p| !matches!(p, screen_model::Panel::Hand(_)));
    }
}

fn rows_from(
    entries: Vec<yokohama_cli::render::DisplayRow>,
    state: &yokohama_core::GameState,
    rules: &yokohama_core::Rules,
) -> Vec<OptionRow> {
    entries
        .into_iter()
        .enumerate()
        .map(|(i, row)| OptionRow {
            number: screen_model::menu_number(i),
            opens_picker: row.mv.is_none(),
            // Priced by the CLI's own function, never by a rule restated here.
            // A row that opens a picker has no single move and so no price yet.
            fee: row.mv.as_ref().and_then(|mv| screen_model::option_fee(state, rules, mv)),
            words: row.words,
        })
        .collect()
}

/// The current prompt's tag, without building the whole prompt.
pub fn prompt_kind(repl: &Repl) -> &'static str {
    prompt_of(repl).kind()
}

fn prompt_of_seat(repl: &Repl, seat: PlayerId) -> PromptState {
    let acting = repl.session.state().current_player;
    // The one scoping rule, applied before anything is read: a payload built
    // for a seat that is not the actor carries no answerable list at all.
    if !repl.session.is_over() && repl.session.record().lockdown.is_none() && seat != acting {
        return PromptState::Watching { to_move: acting };
    }
    prompt_of(repl)
}

/// [`prompt_of_seat`] for a viewer that may hold no seat.
///
/// A spectator is ALWAYS watching while the game is live — the one thing a
/// watcher is never asked is a question — and hears whatever the game itself
/// has to say once it is over or locked, which is what every caller hears
/// there.
fn prompt_of_viewer(repl: &Repl, viewer: Viewer) -> PromptState {
    match viewer {
        Viewer::Seat(s) => prompt_of_seat(repl, s),
        Viewer::Spectator => {
            if repl.session.record().lockdown.is_none() && !repl.session.is_over() {
                PromptState::Watching { to_move: repl.session.state().current_player }
            } else {
                prompt_of(repl)
            }
        }
    }
}

fn prompt_of(repl: &Repl) -> PromptState {
    // Read from the RECORD, which is where `Session::lock_down` puts it and
    // therefore what survives a rebuild — not from a flag this layer cannot see.
    if let Some(reason) = repl.session.record().lockdown.clone() {
        return PromptState::Locked { reason };
    }
    if repl.session.is_over() {
        return PromptState::GameOver;
    }
    let state = repl.session.state();
    let rules = repl.session.rules();
    match &repl.sub {
        SubPrompt::None => {
            PromptState::Options { rows: rows_from(repl.option_rows(), state, rules) }
        }
        SubPrompt::Pick { title, moves } => PromptState::Pick {
            title: title.clone(),
            rows: moves
                .iter()
                .enumerate()
                .map(|(i, mv)| OptionRow {
                    number: screen_model::menu_number(i),
                    words: describe::describe_move(mv, state, rules),
                    opens_picker: false,
                    fee: screen_model::option_fee(state, rules, mv),
                })
                .collect(),
        },
        SubPrompt::Retreat { areas, collected } => {
            // The question is always the next unanswered area, and `max` is
            // read live off that area rather than remembered, exactly as the
            // REPL reads it.
            let (area, max) = areas.get(collected.len()).copied().unwrap_or((AreaId::new(0), 0));
            PromptState::Retreat {
                area,
                area_label: describe::area_label(state, area),
                max,
                answered: collected.clone(),
                remaining: areas.len().saturating_sub(collected.len()),
            }
        }
        SubPrompt::PlaceA { selected, max_areas } => PromptState::PlaceA {
            fee: screen_model::placement_fee(state, selected),
            selected: selected.clone(),
            max_areas: *max_areas,
        },
        SubPrompt::PlaceB { areas, selected } => PromptState::PlaceB {
            // Option B is two assistants in ONE area, which `placement_fee`
            // prices by REPEATING the area — the same shape the engine's own
            // `PlaceTwoSameArea` fee takes.
            fee: selected.map(|a| vec![a, a]).and_then(|two| screen_model::placement_fee(state, &two)),
            areas: areas.clone(),
            selected: *selected,
        },
        SubPrompt::Reveal { .. } => PromptState::Reveal,
    }
}

impl ClientState {
    /// Build the payload for `repl` as `viewer` sees it, carrying `messages`
    /// from whatever produced this state (an applied move's deltas, or a
    /// refusal).
    ///
    /// `viewer` is the route's admission answer — [`viewer_seat`]'s for a seat
    /// holder, [`Viewer::Spectator`] for an admitted watcher — resolved before
    /// the lock is taken. A caller with neither never reaches here.
    pub fn of(id: &str, repl: &Repl, viewer: Viewer, messages: Vec<String>) -> Self {
        let session = &repl.session;
        let state = session.state();
        let over = session.is_over();
        let seat = built_for(state, viewer);

        let mut frame = screen_model::build_frame(
            state,
            session.rules(),
            &facts_for(repl, viewer),
            seat,
            &session.legal(),
            TitleModel {
                mode: match session.mode {
                    TrackingMode::Simulation => TitleMode::Sim,
                    TrackingMode::Live => TitleMode::Live,
                },
                // **`None`, and this is the whole of A6's fix on this side.**
                // The setup seed is hidden-set member #1 (`observe.rs`): it
                // regenerates the deck, the box, the board and every deal, so a
                // payload carrying it hands the entire hidden set to whoever
                // holds the payload. The CLI passes `Some` because it is one
                // operator reading their own title strip; a frame served over
                // HTTP declines. **And nothing else on this wire carries one
                // either, as of M6.1 §8**: the last exception was
                // `POST /api/games/preview`, which drew a seed and round-tripped
                // it back into `create`; the flow is create-then-deal now, the
                // seed lives only in the table document, and
                // `no_committed_payload_carries_the_setup_seed` searches every
                // committed payload for it with no exemption list at all.
                seed: None,
                // This server persists after every applied move, so the answer
                // is unconditionally yes -- it is not the CLI's `save_path`.
                autosaved: true,
            },
        );
        drop_hand_for(viewer, &mut frame);

        Self {
            id: id.to_string(),
            mode: match session.mode {
                TrackingMode::Simulation => "sim",
                TrackingMode::Live => "live",
            },
            seat: match viewer {
                Viewer::Seat(s) => Some(s),
                Viewer::Spectator => None,
            },
            to_move: state.current_player,
            your_turn: !over && matches!(viewer, Viewer::Seat(s) if s == state.current_player),
            frame,
            prompt: prompt_of_viewer(repl, viewer),
            messages,
            over,
            end_triggers: yokohama_cli::render::end_trigger_line(state),
            deck_remaining: state.order_deck.len(),
            end_game_triggered: state.end_game_triggered,
            scores: over.then(|| {
                session
                    .final_scores()
                    .into_iter()
                    .map(|(seat, total)| ScoreRow { seat, total })
                    .collect()
            }),
            warnings: without_version_drift(
                without_foreign_conversion(session.load_warnings.clone(), session.record()),
                session.record(),
            ),
            events: session.record().events.len(),
            bots: crate::bots::seat_bots(session.record()),
        }
    }
}

/// The CLI's drift lines that are not news on the web, dropped from what the
/// game screen shows — TWO masks, and nothing else:
///
/// 1. **The engine-version line** (M6.5c2, spec §2.6, R-6.5c2.19).
///    `engine_version` is provenance only (`GameRecord`'s own doc), and every
///    save carries it; after a release bump the line would sit on EVERY
///    existing game.
/// 2. **Every `warning: seat N records ` line for a seat [`crate::bots::foreign`]
///    names** (M6.5c2 cycle D, straw poll row 143) — the CLI's identity-drift
///    lines (`identity_drift_warnings`, a controller version or a knob that
///    differs from this build's) and its stream-seed line, which share the
///    prefix. Each tells a CLI player the seat "still plays, under the CURRENT
///    build" (or "will not reproduce its recorded line"): the CLI's truth, and
///    false here, where a foreign game reaches this screen only FINISHED — an
///    unfinished one is the typed 409, "This server can't continue it: …" —
///    so no build plays that seat again. A same-name re-point
///    (V4's name at another controller version) is the case that reaches it:
///    finished, it opened with the line beside the refusals that name its bot
///    "V4.3". **This reverses R-C18** (spec §10.8), which kept the CLI-worded
///    warnings apart from §2.6's filter and left this line for cycle D. A seat
///    that is NOT foreign keeps every line.
///
/// The CLI's own frame is unchanged. **A prefix match on the CLI's `format!`**
/// (`Session::from_record`, `bot.rs`), because `load_warnings` is a
/// `pub Vec<String>` and a typed alternative would be a CLI change: the pins
/// that keep it honest assert an EMPTY array (`browser_played_pin.rs`,
/// `a_finished_same_name_re_point_shows_no_drift_warning`), never "no drift
/// line".
pub fn without_version_drift(warnings: Vec<String>, record: &yokohama_core::GameRecord) -> Vec<String> {
    const DRIFT: &str = "warning: GameRecord.engine_version=";
    let seats: Vec<String> =
        crate::bots::foreign(record).into_iter().map(|(seat, _, _)| format!("warning: seat {seat} records ")).collect();
    warnings
        .into_iter()
        .filter(|w| !w.starts_with(DRIFT) && !seats.iter().any(|p| w.starts_with(p.as_str())))
        .collect()
}

/// One historical position, projected from the record — the M4 replay payload.
///
/// **Why this is a separate, narrower type and not a `ClientState` at `k`.**
/// Six fields of `ClientState` are built from the LIVE state and would be
/// present-tense lies if a historical payload inherited them: `over` (a mid-game
/// board under "GAME OVER"), `scores` (a final tally beside turn 12),
/// `deck_remaining`, `end_game_triggered`, `end_triggers` and `your_turn`. Every
/// one of them re-derives correctly from `state_at(record, k, rules)`, so the
/// point is not that `ClientState` could not be made right — it is that a
/// separate type makes their provenance a COMPILE-TIME fact instead of a review
/// item, in a lane whose last three milestones each shipped a false sentence
/// about themselves.
///
/// **Three fields are absent by construction rather than answered:**
///
/// - `your_turn` — the one of the six that is NOT a function of `GameState`
///   alone: it compares against a seat that lives on the `Repl`, not in any
///   record. It is also meaningless for a position nobody is playing.
/// - `prompt` — `PromptState` is built from `Repl::sub`, an in-memory
///   sub-prompt. The record stores only APPLIED moves, so an intra-move picker
///   state is unrepresentable at any `k`.
/// - `warnings` — a load-time session fact, not a property of `state_k`.
///
/// `frame` nests the EXISTING [`FrameModel`], so `Board` and `Dashboard` render
/// a historical position through the same components with no new client types,
/// and the wire contract pays one entry rather than one per nested shape.
#[derive(Debug, Clone, Serialize)]
pub struct HistoricalFrame {
    /// `k` — the number of events APPLIED, not an event index.
    pub index: usize,
    /// `record.events.len()`. The track has `total + 1` stops, and at the last
    /// one there is a state but no event to name.
    pub total: usize,
    /// The seat this payload was built FOR — whose hand is unmasked.
    ///
    /// **[`viewer_seat`]'s answer, and therefore PER CALLER** (M6.1 §5.1). M4's
    /// security sentence is kept and re-pointed: a sweep over `0..=total`
    /// returns THEIR SEATS' hands and no other PERSON's, at any `k`, however
    /// many turns pass between requests — and a second caller sweeping the same
    /// game harvests their own, not this one's.
    ///
    /// **"Stable per caller" means never another person's seat, not always the
    /// same integer** (§5.3 read against §5.1, controller ruling R-4.2). A
    /// caller holding one seat sees that seat at every `k`, forever. A hot-seat
    /// caller holding two sees whichever of THEIR OWN is acting, so the pin
    /// moves among their seats as the turn passes and never outside them — which
    /// is the property the fence is for, and is why a sweep is still not a way
    /// to harvest a second player's hand history.
    ///
    /// **Two earlier answers, and why neither survives an identity.** The first
    /// draft followed the mover, so the pin re-pointed every turn and two sweeps
    /// — or one sweep across a turn change — harvested both seats. M4 replaced
    /// it with the lowest non-bot seat, stable for the life of the game, because
    /// an UNAUTHENTICATED route had no better pin available. An authenticated
    /// route does: the caller's own, which is stable for the same reason and is
    /// additionally the seat the caller is entitled to.
    ///
    /// **`null` for a SPECTATOR** (M6.3, R-3.7), exactly as on the live
    /// payload: a watcher's replay sweep returns no seat's hands at any `k`,
    /// so there is no seat to name.
    pub seat: Option<PlayerId>,
    /// Whose turn it was at `k`.
    pub to_move: PlayerId,
    pub frame: FrameModel,
    pub over: bool,
    /// The end-trigger strip AS IT READ AT `k`, worded by the CLI's own
    /// renderer — the same call the live payload makes, against `state_k`.
    pub end_triggers: String,
    pub deck_remaining: usize,
    pub end_game_triggered: bool,
    pub scores: Option<Vec<ScoreRow>>,
    /// The port and laboratory offers AT `k` — face up at the time, and
    /// properties of the position, which is the test M4's Q5 applied to keep
    /// this frame narrow (the fields it dropped were present-tense lies at a
    /// past position; these are not). The turn playback needs them so a port
    /// take shows the card leaving (M5.5 spec §3.2.1).
    pub ports: screen_model::PortsPanel,
    pub labs: screen_model::LabsPanel,
}

impl HistoricalFrame {
    /// Project `repl`'s record at `k`, from the session's nearest kept state
    /// (`Session::state_before`; security S1b, finding A3). The caller
    /// range-checks `k` first; an error from here is a genuine fold failure.
    ///
    /// **No provenance verdict is computed here, deliberately.** `/at/{k}` is a
    /// projection over a record the server already holds; adding a
    /// `semantics_rev` or terminal-digest check would make it a second,
    /// divergent authority on record validity. Checking is
    /// `Session::from_record`'s job, on the path where a record enters from
    /// disk.
    pub fn at(repl: &mut Repl, k: usize, viewer: Viewer) -> Result<Self, String> {
        let state = repl.session.state_before(k)?;
        Ok(Self::from_state(repl, k, &state, viewer))
    }

    /// The frames of every position in `[from, to)`, by ONE forward pass: the
    /// state at `from` from the session's ladder, then one event folded per
    /// position through the predicate (`Session::fold_event`). The caller
    /// range-checks `from` and `to`. An error names the RECORD's index.
    pub fn run(repl: &mut Repl, from: usize, to: usize, viewer: Viewer) -> Result<Vec<Self>, String> {
        let mut state = repl.session.state_before(from)?;
        let mut frames = Vec::with_capacity(to.saturating_sub(from));
        for k in from..to {
            frames.push(Self::from_state(repl, k, &state, viewer));
            if k + 1 < to {
                let ev = &repl.session.record().events[k];
                state = Session::fold_event(&state, ev, repl.session.rules()).map_err(|e| format!("at event {k}, {e}"))?;
            }
        }
        Ok(frames)
    }

    /// The projection of ONE position, given its state — what `at` and `run`
    /// share, so the two cannot drift (a run's frame IS `at`'s, by
    /// construction and by the cell that compares them by value).
    pub fn from_state(
        repl: &Repl,
        k: usize,
        state: &yokohama_core::GameState,
        viewer: Viewer,
    ) -> Self {
        let session = &repl.session;
        let record = session.record();
        let total = record.events.len();
        let rules = session.rules();
        let seat = built_for(state, viewer);

        // **Paid deliberately, not defensively.** `build_board` derives
        // `tile.reachable` from the legal list, so a historical board built with
        // an empty one silently marks every tile unreachable and the `◈` mark
        // vanishes with NO error. `state_at` already ran one enumeration per
        // replayed Move event through the checked `apply_move`, so one more is
        // strictly cheaper than one more event.
        let legal = yokohama_core::get_legal_moves(state, rules);
        let over = yokohama_core::is_game_over(state);

        let mut frame = screen_model::build_frame(
            state,
            rules,
            &facts_for(repl, viewer),
            seat,
            &legal,
            TitleModel {
                mode: match session.mode {
                    TrackingMode::Simulation => TitleMode::Sim,
                    TrackingMode::Live => TitleMode::Live,
                },
                // Hidden-set member #1, exactly as on the live payload.
                seed: None,
                // **A session property, not a position property.** The field
                // means "autosave has a live target", and this server persists
                // after every applied move — which is as true while you look at
                // turn 3 as it is at the head. It does not become false by
                // looking backwards, so this is not the present-tense lie the
                // six fields above would be.
                autosaved: true,
            },
        );
        drop_hand_for(viewer, &mut frame);

        Self {
            index: k,
            total,
            seat: match viewer {
                Viewer::Seat(s) => Some(s),
                Viewer::Spectator => None,
            },
            to_move: state.current_player,
            frame,
            over,
            end_triggers: yokohama_cli::render::end_trigger_line(state),
            deck_remaining: state.order_deck.len(),
            end_game_triggered: state.end_game_triggered,
            scores: over.then(|| {
                yokohama_core::calculate_final_scores(state, &rules.orders, &rules.techs)
                    .into_iter()
                    .map(|(seat, total)| ScoreRow { seat, total })
                    .collect()
            }),
            ports: screen_model::build_ports(state, rules),
            labs: screen_model::build_labs(state, rules),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::{SeatDoc, TableStatus};
    use yokohama_cli::bot::BotSeats;
    use yokohama_cli::session::Session;
    use yokohama_core::{
        finalize_setup, new_game, GameConfig, GameState, PlayerColor, Rules, StartingOrderChoice,
    };

    /// A playing table whose seats are the named emails, in order.
    fn table_of(emails: &[&str]) -> TableDoc {
        let players = emails.len() as u8;
        TableDoc {
            schema: 1,
            id: "t".into(),
            name: None,
            created_by: emails[0].to_string(),
            created_at: 0,
            players,
            colors: [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow]
                [..usize::from(players)]
                .to_vec(),
            start_seat: 0,
            seats: emails
                .iter()
                .map(|e| SeatDoc::Human { user: (*e).to_string(), name: None, bound_at_create: true })
                .collect(),
            keeps: vec![Some(1); usize::from(players)],
            seed: 7,
            status: TableStatus::Playing,
            turn: None,
            lock: None,
            unlocked: Vec::new(),
            archived_at: None,
        }
    }

    /// A real 3P position with `current_player` moved to `seat`.
    ///
    /// [`viewer_seat`] reads exactly one field of the state, so the position
    /// itself is scenery — but it is a REAL one rather than a construction,
    /// because `GameState` has no cheap honest constructor and a hand-built one
    /// would be a second statement of the setup.
    fn state_with_current(seat: PlayerId) -> GameState {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(3), &rules, 7).expect("a setup");
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0); 3]).expect("a game");
        state.current_player = seat;
        state
    }

    /// The corpus-seed 2P game with no bots — built exactly as
    /// `wire_fixtures.rs`'s `build(vec![None, None])` builds it, so the hand
    /// this cell reads is the hand the committed corpus carries.
    fn two_human_repl() -> Repl {
        let rules = Rules::standard();
        let mut config = GameConfig::for_player_count(2);
        config.player_colors = vec![PlayerColor::Red, PlayerColor::Blue];
        config.start_player = PlayerId::new(0);
        let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)];
        let mut session =
            Session::new_sim(config, 20_260_831, choices, rules).expect("a fixture game");
        let bots = BotSeats::new(vec![None, None], 0x4649_5854_5552_4553);
        session.set_run_seats(bots.seat_provenance());
        Repl::new(session)
    }

    /// **`ClientState.bots` is the record's bot seats, labelled** (M6.5c2,
    /// R-6.5c2.10): empty for an all-human game, the bot's seat and label for a
    /// vs-bot one.
    #[test]
    fn the_payload_names_the_records_bot_seats() {
        use yokohama_cli::bot::PROVISIONAL_DEFAULT_BOT;
        let humans = two_human_repl();
        let payload = ClientState::of("t", &humans, Viewer::Seat(PlayerId::new(0)), Vec::new());
        assert!(payload.bots.is_empty(), "an all-human game names no bot");

        let mut config = GameConfig::for_player_count(2);
        config.player_colors = vec![PlayerColor::Red, PlayerColor::Blue];
        config.start_player = PlayerId::new(0);
        let mut session = Session::new_sim(config, 20_260_831, vec![StartingOrderChoice::Keep(0); 2], Rules::standard())
            .expect("a game");
        session.set_run_seats(BotSeats::new(vec![None, Some(PROVISIONAL_DEFAULT_BOT)], 7).seat_provenance());
        let vs_bot = Repl::new(session);
        let payload = ClientState::of("t", &vs_bot, Viewer::Seat(PlayerId::new(0)), Vec::new());
        assert_eq!(
            serde_json::to_value(&payload.bots).expect("serializable"),
            serde_json::json!([{ "seat": 1, "label": crate::bots::label(&crate::bots::production()) }]),
        );
    }

    /// The three arms of [`viewer_seat`], including the hot seat, in one cell —
    /// because the claim is the PRECEDENCE, and each arm alone is satisfied by
    /// an implementation that ignores the other two.
    #[test]
    fn the_viewer_is_the_acting_seat_when_mine_else_my_lowest_else_none() {
        let (red, blue, green) = (
            User::parse("red@t.test").expect("an email"),
            User::parse("blue@t.test").expect("an email"),
            User::parse("green@t.test").expect("an email"),
        );
        // A 3P hot seat for red: seats 0 and 2 are red's, seat 1 is blue's.
        let mut doc = table_of(&["red@t.test", "blue@t.test", "red@t.test"]);
        let state = state_with_current(PlayerId::new(2));
        assert_eq!(viewer_seat(&doc, &red, &state), Some(PlayerId::new(2)), "acting and mine");
        assert_eq!(
            viewer_seat(&doc, &blue, &state),
            Some(PlayerId::new(1)),
            "not acting: my lowest"
        );
        assert_eq!(viewer_seat(&doc, &green, &state), None, "a stranger has no seat to view from");
        doc.seats[2] = SeatDoc::Bot;
        assert_eq!(
            viewer_seat(&doc, &red, &state),
            Some(PlayerId::new(0)),
            "a bot is acting: my lowest"
        );
    }

    /// R-1.7 at the model: one hand panel, the viewer's, and every other seat
    /// masked on the panel that carries the rest.
    #[test]
    fn the_facts_hide_every_hand_but_the_viewers_bots_included() {
        let repl = two_human_repl();
        let f = facts_for(&repl, Viewer::Seat(PlayerId::new(1)));
        assert_eq!(f.iter().map(|s| s.hand_hidden).collect::<Vec<_>>(), vec![true, false]);
        assert!(f.iter().all(|s| !s.is_bot), "this fixture seats no bot");

        let payload = ClientState::of("t", &repl, Viewer::Seat(PlayerId::new(1)), Vec::new());
        assert_eq!(payload.seat, Some(PlayerId::new(1)));
        let hands: Vec<(u8, bool, usize)> = payload
            .frame
            .dashboard
            .iter()
            .filter_map(|p| match p {
                screen_model::Panel::Hand(h) => Some((h.seat.0, h.hidden, h.cards.len())),
                _ => None,
            })
            .collect();
        assert_eq!(
            hands,
            vec![(1, false, 1)],
            "the viewer's own hand, open, and no other Hand panel"
        );

        // The mask the OTHER seats wear instead — a flag on the Players panel,
        // never a hand panel of their own. Asserted here because `hands` above
        // would read the same if the model had simply stopped masking.
        let rows: Vec<(u8, bool)> = payload
            .frame
            .dashboard
            .iter()
            .filter_map(|p| match p {
                screen_model::Panel::Players(pp) => Some(pp),
                _ => None,
            })
            .flat_map(|pp| pp.players.iter().map(|p| (p.seat.0, p.hand_hidden)))
            .collect();
        assert_eq!(rows, vec![(0, true), (1, false)]);

        // And the payload is the VIEWER's in its other two seat-shaped fields:
        // seat 0 is to move, so seat 1 is watching and it is not its turn.
        assert!(!payload.your_turn);
        assert_eq!(payload.prompt.kind(), "watching");
    }

    // ---------------------------------------------------------------------
    // M6.3 — the spectator's projection (spec §4, R-3.7)
    // ---------------------------------------------------------------------

    /// An `n`-player game with every seat a person, at the corpus seed.
    ///
    /// [`two_human_repl`] with the player count opened up: §4's claim is made
    /// at 2P, 3P and 4P because [`stand_in`] is arithmetic over the player
    /// count, and "there is always a seat that is not acting" is a sentence
    /// about all three.
    fn human_repl(n: u8) -> Repl {
        let rules = Rules::standard();
        let mut config = GameConfig::for_player_count(n);
        config.player_colors =
            [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green]
                [..usize::from(n)]
                .to_vec();
        config.start_player = PlayerId::new(0);
        let choices = vec![StartingOrderChoice::Keep(0); usize::from(n)];
        let mut session =
            Session::new_sim(config, 20_260_831, choices, rules).expect("a fixture game");
        let bots = BotSeats::new(vec![None; usize::from(n)], 0x4649_5854_5552_4553);
        session.set_run_seats(bots.seat_provenance());
        Repl::new(session)
    }

    /// Every hand in the frame, as `(seat, hidden, cards)` — the reading both
    /// cells below are about.
    fn hand_panels(frame: &FrameModel) -> Vec<(u8, bool, usize)> {
        frame
            .dashboard
            .iter()
            .filter_map(|p| match p {
                screen_model::Panel::Hand(h) => Some((h.seat.0, h.hidden, h.cards.len())),
                _ => None,
            })
            .collect()
    }

    /// **The mechanism the drop removes is real, at every player count** — so
    /// the cell below is removing something rather than asserting an absence
    /// that was already there.
    ///
    /// `build_hand` answers `Some(HandPanel { hidden: true, cards: [], … })`
    /// for a hidden seat, not `None`, so a spectator's dashboard would carry
    /// one masked hand under the caption every other payload puts the READER's
    /// hand under — singling out a seat as "yours" that is nobody's.
    #[test]
    fn the_builder_hands_a_spectator_a_masked_hand_panel_before_the_projection_drops_it() {
        for n in [2u8, 3, 4] {
            let repl = human_repl(n);
            let state = repl.session.state();
            let seat = stand_in(state);
            assert_ne!(seat, state.current_player, "{n}P: the stand-in is never the actor");
            let frame = screen_model::build_frame(
                state,
                repl.session.rules(),
                &facts_for(&repl, Viewer::Spectator),
                seat,
                &repl.session.legal(),
                TitleModel { mode: TitleMode::Sim, seed: None, autosaved: true },
            );
            assert_eq!(
                hand_panels(&frame),
                vec![(seat.0, true, 0)],
                "{n}P: the builder draws the stand-in's hand, hidden and empty",
            );
        }
    }

    /// **A spectator's live payload, by value, at 2P, 3P and 4P** (R-3.7).
    ///
    /// One cell over the six consequences rather than six cells over one,
    /// because what is claimed is that they hold TOGETHER: an implementation
    /// that nulls the seat and leaves the options on it satisfies the first
    /// and is the defect.
    #[test]
    fn a_spectators_payload_names_no_seat_hides_every_hand_and_is_asked_nothing() {
        for n in [2u8, 3, 4] {
            let repl = human_repl(n);
            let acting = repl.session.state().current_player;
            let payload = ClientState::of("t", &repl, Viewer::Spectator, Vec::new());

            assert_eq!(payload.seat, None, "{n}P: a watcher holds no seat");
            assert!(!payload.your_turn, "{n}P");
            assert_eq!(payload.prompt.kind(), "watching", "{n}P");
            assert!(
                matches!(payload.prompt, PromptState::Watching { to_move } if to_move == acting),
                "{n}P: and it names who IS to move",
            );
            assert!(hand_panels(&payload.frame).is_empty(), "{n}P: no Hand panel at all");
            assert!(!payload.frame.decision.viewer_is_deciding, "{n}P");
            assert!(payload.frame.decision.options.is_empty(), "{n}P: no options");
            assert_eq!(
                payload.frame.decision.seat, acting,
                "{n}P: the DECIDING seat is still named — it is public",
            );

            // Every seat masked, on the panel that carries the counts. The
            // absence of a Hand panel above would read the same if the model
            // had simply stopped masking.
            let rows: Vec<(u8, bool)> = payload
                .frame
                .dashboard
                .iter()
                .filter_map(|p| match p {
                    screen_model::Panel::Players(pp) => Some(pp),
                    _ => None,
                })
                .flat_map(|pp| pp.players.iter().map(|p| (p.seat.0, p.hand_hidden)))
                .collect();
            assert_eq!(rows.len(), usize::from(n), "{n}P: one row per seat");
            assert!(rows.iter().all(|(_, hidden)| *hidden), "{n}P: every hand hidden: {rows:?}");

            // And `seat: null` is what reaches the WIRE, not a zero.
            let json = serde_json::to_value(&payload).expect("a payload");
            assert_eq!(json["seat"], serde_json::Value::Null, "{n}P: {json}");
        }
    }

    /// The same claim on the REPLAY payload, at both ends of the track.
    ///
    /// `k = total` is the live position and `k = 0` is the board before any
    /// move — the two stops M4's own cells use, because at `total` the frame
    /// would prove nothing if it were merely the live one, and at `0` the
    /// position is empty enough that a leak would be conspicuous.
    #[test]
    fn a_spectators_replay_frame_names_no_seat_at_either_end_of_the_track() {
        let mut repl = human_repl(3);
        let total = repl.session.record().events.len();
        for k in [0, total] {
            let frame = HistoricalFrame::at(&mut repl, k, Viewer::Spectator).expect("a position");
            assert_eq!(frame.seat, None, "at {k}");
            assert!(hand_panels(&frame.frame).is_empty(), "at {k}: no Hand panel");
            assert!(!frame.frame.decision.viewer_is_deciding, "at {k}");
            assert!(frame.frame.decision.options.is_empty(), "at {k}");
            let json = serde_json::to_value(&frame).expect("a payload");
            assert_eq!(json["seat"], serde_json::Value::Null, "at {k}: {json}");
        }

        // The anti-vacuity half: a SEATED replay frame at the same stops still
        // names its seat and still carries that seat's own hand.
        let seated = HistoricalFrame::at(&mut repl, 0, Viewer::Seat(PlayerId::new(1)))
            .expect("a position");
        assert_eq!(seated.seat, Some(PlayerId::new(1)));
        assert_eq!(hand_panels(&seated.frame), vec![(1, false, 1)]);
    }

    /// **Once the game is over a spectator hears what the GAME has to say**,
    /// which is what every caller hears there: the prompt is `game_over` and
    /// the final scores are on the payload.
    ///
    /// The finish is a both-bot pump of the very game the fixture describes —
    /// the `a_finished_two_person_game` mechanism, at the `Repl` level, because
    /// the subject here is the projection and not a route.
    #[test]
    fn a_spectator_at_a_finished_game_is_told_it_is_over_and_reads_the_scores() {
        use yokohama_cli::bot::PROVISIONAL_DEFAULT_BOT;
        let rules = Rules::standard();
        let mut config = GameConfig::for_player_count(2);
        config.player_colors = vec![PlayerColor::Red, PlayerColor::Blue];
        config.start_player = PlayerId::new(0);
        let mut session = Session::new_sim(
            config,
            20_260_831,
            vec![StartingOrderChoice::Keep(0); 2],
            rules,
        )
        .expect("a game");
        let bots = BotSeats::new(
            vec![Some(PROVISIONAL_DEFAULT_BOT), Some(PROVISIONAL_DEFAULT_BOT)],
            20_260_831,
        );
        session.set_run_seats(bots.seat_provenance());
        let mut repl = Repl::new(session);
        for _ in 0..400 {
            if repl.session.is_over() {
                break;
            }
            repl.pump_bots();
        }
        assert!(repl.session.is_over(), "the premise: a finished game");

        let payload = ClientState::of("t", &repl, Viewer::Spectator, Vec::new());
        assert_eq!(payload.seat, None);
        assert!(!payload.your_turn);
        assert_eq!(payload.prompt.kind(), "game_over", "not `watching` — there is nothing to watch");
        assert_eq!(
            payload.scores.as_ref().map(Vec::len),
            Some(2),
            "the final standings are public",
        );
        assert!(hand_panels(&payload.frame).is_empty(), "and still no hand");
    }

    /// **Only the engine-version drift line is dropped** (M6.5c2, spec §2.6,
    /// R-6.5c2.19) — **and, for a FOREIGN seat alone, its `records` lines**
    /// (cycle D, straw poll row 143). The match is a PREFIX on the CLI's own
    /// `format!` (`Session::from_record`, `bot.rs`), because `load_warnings` is
    /// a `pub Vec<String>` and a typed alternative would be a CLI change. So
    /// this cell holds the filter's PRECISION — every other warning stays, a
    /// line that merely quotes the field is kept, and over a record whose
    /// seat 1 is foreign the same `records` line naming seat 0 (a person)
    /// stays — while
    /// `browser_played_pin.rs`'s `a_record_from_another_release_opens_with_no_warning_and_keeps_its_version`
    /// and `a_finished_same_name_re_point_shows_no_drift_warning` hold that it
    /// matches the CLI at all.
    #[test]
    fn only_the_engine_version_line_and_a_foreign_seats_records_lines_are_dropped() {
        let home: yokohama_core::GameRecord =
            serde_json::from_str(crate::games::test_support::PLAYED_RECORD).expect("the pair's record");
        assert!(crate::bots::foreign(&home).is_empty(), "premise: no foreign seat");
        let drift = "warning: GameRecord.engine_version=\"0.5.0\" does not match running engine version \
                     \"0.6.0\" — replaying anyway (v1 does not refuse on a version mismatch)"
            .to_string();
        let rules = "warning: GameRecord.rules_hash=\"x\" does not match running rules content hash \"y\" — \
                     replaying anyway (v1 does not refuse on a rules_hash mismatch)"
            .to_string();
        let seat = "warning: seat 1 was played by controller \"z\", which this build does not know how to run"
            .to_string();
        let quoted = format!("note: {drift}");
        let records = |n: u8| {
            format!(
                "warning: seat {n} records controller \"v2-padded\" at version 1, but this build runs \
                 version 2 — the seat still plays, under the CURRENT build"
            )
        };
        assert_eq!(
            without_version_drift(
                vec![drift.clone(), rules.clone(), seat.clone(), quoted.clone(), records(1)],
                &home,
            ),
            vec![rules.clone(), seat.clone(), quoted.clone(), records(1)],
            "no foreign seat: a `records` line stays",
        );
        assert_eq!(without_version_drift(vec![drift.clone()], &home), Vec::<String>::new());
        assert_eq!(without_version_drift(Vec::new(), &home), Vec::<String>::new());

        let (away, _) = crate::games::test_support::foreign_pair("v2-padded", None);
        let away: yokohama_core::GameRecord = serde_json::from_str(&away).expect("the pair's record");
        let knob = "warning: seat 1 records b=8, but this build derives b=4 — the seat still plays, under the \
                    CURRENT build"
            .to_string();
        assert_eq!(
            without_version_drift(vec![drift, records(1), knob, records(0), rules.clone(), seat.clone()], &away),
            vec![records(0), rules, seat],
            "seat 1 is foreign: its version and knob lines go, seat 0's stays",
        );
    }

    /// **Only the FOREIGN seat's conversion line is dropped** (M6.5c2, spec
    /// §2.4's first mask, R-6.5c2.10) — the first stage's PRECISION, which the
    /// finished-foreign cell's `[]` cannot see: a filter that emptied every
    /// game's warnings passes that cell. Over the committed pair re-seated
    /// against `v2-padded` (seat 1 foreign, seat 0 a person), fed lines in the
    /// CLI's own shapes (`BotSeats::from_provenance`'s two conversion arms,
    /// `identity_drift_warnings`, `Session::from_record`'s `rules_hash` line):
    /// seat 1's two conversion lines go, and a `rules_hash` line, seat 1's
    /// version-drift line and a conversion-shaped line naming the HUMAN seat 0
    /// all stay, in order.
    #[test]
    fn only_the_foreign_seats_conversion_line_is_dropped() {
        let (record, _) = crate::games::test_support::foreign_pair("v2-padded", None);
        let record: yokohama_core::GameRecord = serde_json::from_str(&record).expect("the pair's record");
        let converted = "warning: seat 1 was played by controller \"v2-padded\", which this build does not know \
                         how to run — it resumes as a HUMAN seat, so you are now playing it (x)"
            .to_string();
        let refused = "warning: seat 1 was played by controller \"v2-padded\", but y — it resumes as a HUMAN \
                       seat, so you are now playing it"
            .to_string();
        let rules = "warning: GameRecord.rules_hash=\"x\" does not match running rules content hash \"y\" — \
                     replaying anyway (v1 does not refuse on a rules_hash mismatch)"
            .to_string();
        let drift = "warning: seat 1 records controller \"v2-padded\" at version 1, but this build runs \
                     version 2 — the seat still plays, under the CURRENT build"
            .to_string();
        let human = "warning: seat 0 was played by controller \"v2-padded\", which this build does not know \
                     how to run — it resumes as a HUMAN seat, so you are now playing it (x)"
            .to_string();
        assert_eq!(
            without_foreign_conversion(
                vec![converted.clone(), rules.clone(), refused, drift.clone(), human.clone()],
                &record,
            ),
            vec![rules, drift, human],
        );
        assert_eq!(without_foreign_conversion(vec![converted], &record), Vec::<String>::new());
    }
}
