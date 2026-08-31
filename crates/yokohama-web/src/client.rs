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
    /// The entry in plain words, from the CLI's own renderer — fees, route
    /// counts and previews included, so a client never prices anything itself.
    pub words: String,
    /// True when choosing this opens a further question (a route family, the
    /// retreat family, an Option A/B placement) instead of applying a move.
    /// The client can say "…" rather than implying the turn is about to end.
    pub opens_picker: bool,
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
    PlaceA { selected: Vec<AreaId>, max_areas: usize },
    /// Placement Option B: pick one area for two assistants, then `confirm`.
    PlaceB { areas: Vec<AreaId>, selected: Option<AreaId> },
    /// A live-mode card reveal. v1 does not create live games, so this is here
    /// to be reported honestly rather than rendered.
    Reveal,
    /// Nothing is being decided.
    GameOver,
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
    pub scores: Option<Vec<ScoreRow>>,
    /// Provenance drift noticed when the game was loaded.
    pub warnings: Vec<String>,
}

/// The seat the browser is playing: the first seat no bot holds.
///
/// v1 is one human against the shipped bot, so "the human seat" is
/// well-defined. When two people play, this becomes a routing question — which
/// is why every payload already carries the seat it was built for rather than
/// leaving the client to assume.
pub fn viewing_seat(repl: &Repl) -> PlayerId {
    repl.seat_facts()
        .iter()
        .position(|f| !f.is_bot)
        .map(|i| PlayerId::new(i as u8))
        .unwrap_or(PlayerId::new(0))
}

fn rows_from(entries: Vec<yokohama_cli::render::DisplayRow>) -> Vec<OptionRow> {
    entries
        .into_iter()
        .enumerate()
        .map(|(i, row)| OptionRow {
            number: screen_model::menu_number(i),
            opens_picker: row.mv.is_none(),
            words: row.words,
        })
        .collect()
}

fn prompt_of(repl: &Repl) -> PromptState {
    if repl.session.is_over() {
        return PromptState::GameOver;
    }
    let state = repl.session.state();
    let rules = repl.session.rules();
    match &repl.sub {
        SubPrompt::None => PromptState::Options { rows: rows_from(repl.option_rows()) },
        SubPrompt::Pick { title, moves } => PromptState::Pick {
            title: title.clone(),
            rows: moves
                .iter()
                .enumerate()
                .map(|(i, mv)| OptionRow {
                    number: screen_model::menu_number(i),
                    words: describe::describe_move(mv, state, rules),
                    opens_picker: false,
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
        SubPrompt::PlaceA { selected, max_areas } => {
            PromptState::PlaceA { selected: selected.clone(), max_areas: *max_areas }
        }
        SubPrompt::PlaceB { areas, selected } => {
            PromptState::PlaceB { areas: areas.clone(), selected: *selected }
        }
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
            seat,
            to_move: state.current_player,
            your_turn: !over && state.current_player == seat,
            frame,
            prompt: prompt_of(repl),
            messages,
            over,
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
