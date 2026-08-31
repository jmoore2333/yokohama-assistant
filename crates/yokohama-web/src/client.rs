//! What a browser receives: the frame, plus what it is being asked.
//!
//! Everything here is a projection. The frame is `screen_model::build_frame`'s,
//! the words are `describe`'s, and the numbered entries are the REPL's own —
//! see [`PromptState`] on why that last one is not negotiable.

use serde::Serialize;
use yokohama_cli::repl::{Repl, SubPrompt};
use yokohama_cli::screen_model::{self, FrameModel, TitleMode, TitleModel};
use yokohama_cli::{describe, session::TrackingMode};
use yokohama_core::{AreaId, PlayerId};

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
    /// The setup seed, as a STRING.
    ///
    /// **Read this and not `frame.title.seed`.** That field is a `u64` and
    /// serializes as a JSON number, which a browser cannot hold: `JSON.parse`
    /// rounds anything past 2^53, and a real seed is around 1.8e19. It stays a
    /// number because `screen_model` is the CLI's model and this port does not
    /// move it; this field is the web layer's lossless answer. See
    /// [`crate::games::WireSeed`].
    pub seed: crate::games::WireSeed,
    /// The seat this payload was built FOR — whose hand is unmasked and whose
    /// decision zone carries options. Not necessarily the seat to move.
    pub seat: PlayerId,
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
        }
    }
}

/// The seat this payload is built for: **the seat to move**, exactly as the
/// terminal does it.
///
/// `screen.rs:608` passes `state.current_player` to `render::dashboard`, so the
/// CLI's hand panel follows the ACTOR — which is what a single operator driving
/// a table needs, and this port inherits that model rather than inventing one.
///
/// **It used to return the first seat no bot holds, and that was a parity
/// divergence with teeth.** At 3P or 4P with one bot, seats 0 and 2 are both
/// human; the frame was built for seat 0 while `PromptState` carried seat 2's
/// options, because those come from `Repl::option_rows()` and the REPL has only
/// ever one current player. The screen then showed RED's hand, said "seat 2 is
/// deciding", and offered seat 2's moves as live buttons — three statements
/// about three different seats, with the acting seat's hand nowhere.
///
/// The bot arm is a real fallback and not defensiveness: `step` pumps bot seats
/// after every applied move, so a returned payload is normally on a human, but
/// a payload built mid-pump would otherwise carry a bot's seat. Its hand stays
/// masked either way — `SeatFacts.hand_hidden` folds `is_bot` in regardless of
/// the privacy toggle — so this only decides whose panel is shown, never
/// whether a hidden hand leaks.
pub fn viewing_seat(repl: &Repl) -> PlayerId {
    let facts = repl.seat_facts();
    let acting = repl.session.state().current_player;
    let is_bot = facts.get(usize::from(acting.0)).is_some_and(|f| f.is_bot);
    if !is_bot {
        return acting;
    }
    facts
        .iter()
        .position(|f| !f.is_bot)
        .map(|i| PlayerId::new(i as u8))
        .unwrap_or(acting)
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
    /// Build the payload for `repl`, carrying `messages` from whatever produced
    /// this state (an applied move's deltas, or a refusal).
    pub fn of(id: &str, repl: &Repl, messages: Vec<String>) -> Self {
        let seat = viewing_seat(repl);
        let session = &repl.session;
        let state = session.state();
        let over = session.is_over();

        let frame = screen_model::build_frame(
            state,
            session.rules(),
            &repl.seat_facts(),
            seat,
            &session.legal(),
            TitleModel {
                mode: match session.mode {
                    TrackingMode::Simulation => TitleMode::Sim,
                    TrackingMode::Live => TitleMode::Live,
                },
                seed: session.record().seed,
                // This server persists after every applied move, so the answer
                // is unconditionally yes -- it is not the CLI's `save_path`.
                autosaved: true,
            },
        );

        Self {
            id: id.to_string(),
            mode: match session.mode {
                TrackingMode::Simulation => "sim",
                TrackingMode::Live => "live",
            },
            seed: crate::games::WireSeed(session.record().seed),
            seat,
            to_move: state.current_player,
            your_turn: !over && state.current_player == seat,
            frame,
            prompt: prompt_of(repl),
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
            warnings: session.load_warnings.clone(),
        }
    }
}
