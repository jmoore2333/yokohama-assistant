//! `why`: the account of a move a CLI BOT already played (Phase 3.5c W5 task
//! 5, design §2 — gate ingredient 3's CLI reading, and clause (b) of OD-4(i)'s
//! three-clause executable form: *every CLI bot move retrievable via `why`*).
//!
//! **The split with [`crate::suggest`], stated once**: `suggest` answers *what
//! would you play on MY seat*, before you move; `why` answers *how did the BOT
//! choose the move it already played*, after. The two share a renderer and
//! nothing else, and each refusal here points at the other where that is the
//! honest advice.
//!
//! **RECONSTRUCT BY REPLAY, REFUSE ON MISMATCH.** For event index `n` the
//! decision state is `state_at(record, n)` (the state the mover decided IN),
//! the mover is that state's `current_player`, and the decision key is
//! `BotSeats::decision_seed(n)` — the same `(bot_seed, events.len())` keying
//! the pump used when it appended event `n`. `why` then re-runs [`crate::bot::decide`],
//! the LITERAL function the pump ran, and asserts the reconstructed move equals
//! the recorded one. On a mismatch it REFUSES, naming both moves and both
//! version consts.
//!
//! That gate is load-bearing three ways, and none of them is belt-and-braces:
//!
//! 1. It is a per-invocation DETERMINISM check — a keying regression surfaces
//!    as a refusal rather than as a confident, wrong table.
//! 2. It resolves the mid-history-downgrade ambiguity BY CONSTRUCTION. A seat
//!    downgraded to HUMAN mid-session leaves human moves under a bot-named
//!    provenance; no per-event attribution exists (a `RecordedEvent::Move`
//!    carries no seat, and W5 adds no new event kinds), but reconstruction
//!    mismatches on exactly those moves and refuses instead of fabricating an
//!    explanation for a move the bot never chose.
//! 3. It keeps `why` honest ACROSS BUILDS: a record written by an older impl
//!    reconstructs differently, so it refuses naming the versions.
//!
//! **READ-ONLY, by type.** [`why`] takes `&Session` and applies nothing,
//! records nothing and autosaves nothing — the compiler enforces the sandbox,
//! exactly as it does for `suggest`. Two calls at one address are byte-identical
//! because every input (the record, the event index, the derived seed) is a
//! function of the address alone.
//!
//! **Game over is NOT a refusal.** `suggest` refuses a finished game because
//! there is nothing left to advise; `why` explains history, which survives the
//! end — and post-game reading is exactly the checkpoint's use for it.

use yokohama_core::*;

use crate::bot::{self, BotSeats};
use crate::session::{Session, TrackingMode};
use crate::style::Tone;
use crate::suggest::{Header, Suggestion};
use crate::{describe, suggest};

/// What a one-ply argmax has instead of a line and a ranking — the sentence
/// `why` prints in place of both blocks for a `greedy-v1` mover
/// ([`Header::explanation_only`]).
const ONE_PLY_ACCOUNT: &str = "This contestant takes a one-ply argmax: it scores each successor \
                               position and plays the best one, so it searched no line and \
                               reports no per-candidate ranking — the table below IS its whole \
                               account of the move.";

/// What a position offering exactly ONE move is led with (spec §2.2).
///
/// **The fact is the ENGINE's, and that is why it can be said on both arms.**
/// The gate is `get_legal_moves(&state, rules).len() == 1` at the reconstructed
/// decision state — a statement about the position, which is exactly what this
/// sentence claims. No search artifact could carry it: greedy's `ranking` is
/// empty BY CONSTRUCTION (`bot.rs`) whatever the position offered, so a
/// one-candidate ranking means "greedy" on one arm and "forced" on the other,
/// and reading the two as one fact is how a screen ends up asserting something
/// false about a branching point.
///
/// It COMPOSES with [`ONE_PLY_ACCOUNT`] rather than replacing it: the two
/// sentences are about different things (this one about the position, that one
/// about the contestant), and a greedy seat at a forced point is where both
/// are true at once.
const FORCED_LEAD: &str =
    "Only legal option — nothing was weighed: the position offered exactly one move.";

/// The counter block's WORK line: what the search did.
const SEARCH_COST_LABEL: &str = "Search cost:";

/// The counter block's LIMIT line: where a budget or a guard bit. D7's
/// mandatory step-one set (`budget_binds`, `budget_starved`, `ply_cap_hits`,
/// `free_cap_hits`) all land here, which is the point of splitting at all —
/// "did anything stop this search" is one question and deserves one line.
const GUARD_LABEL: &str = "Budget and guards:";

/// The counter block's DEFECT line: a `counter_rows` name this build's
/// grouping does not place. See [`counter_lines`].
const UNGROUPED_LABEL: &str = "Counters this build does not group (please report):";

/// The `why` screen for `arg` (or for the most recent explainable bot move
/// when `arg` is `None`), or the typed refusal that stands in for it.
pub(crate) fn why(session: &Session, bots: &BotSeats, arg: Option<usize>) -> String {
    why_over(session.mode, session.record(), session.rules(), bots, arg)
}

/// [`why`] over the record directly, which is all it ever reads.
///
/// The seam exists so a refusal row whose fixture the sim driver never writes
/// — a reveal event inside a simulation record — can be driven at all: the
/// arms below are a function of `(mode, record, rules, bots, arg)` and of
/// nothing on a `Session`.
fn why_over(
    mode: TrackingMode,
    record: &GameRecord,
    rules: &Rules,
    bots: &BotSeats,
    arg: Option<usize>,
) -> String {
    match account(mode, record, rules, bots, arg) {
        Ok(screen) => screen,
        // Folded HERE rather than authored pre-wrapped: every refusal below
        // interpolates something unbounded (a parser's error, a `Move`'s
        // `Debug`, a contestant's echo), so a hand-wrapped literal bounds the
        // half that was never the risk. See `suggest::wrap_to`.
        Err(refusal) => suggest::wrap_to(&refusal.to_string(), refusal_width(record)),
    }
}

/// How wide a refusal's prose may run: the editorial measure, or the frame if
/// the frame is narrower. The frame is the GUARANTEE (`screen::wrap` sizes the
/// composed screen to its widest line); [`suggest::PROSE_TEXT_WIDTH`] is what
/// keeps a paragraph readable at a 198-column two-player frame. One measure for
/// both surfaces, so a refusal and the account's own notes cannot disagree
/// about how wide a sentence is.
fn refusal_width(record: &GameRecord) -> usize {
    crate::screen::frame_width(usize::from(record.config.player_count))
        .min(suggest::PROSE_TEXT_WIDTH)
}

/// Why there is no account — design §2.3's table, one variant per row plus the
/// two arms it names in prose (a DIAGNOSTIC controller, and one this build
/// cannot parse at all).
///
/// Typed and named rather than collapsed to one string for `SuggestError`'s
/// reason: they mean genuinely different things to the reader. Three of them
/// say *ask a different question* (live mode, a human's move, a rung's
/// sample), three say *that is not an address* (out of range, a reveal, no bot
/// move yet), and two say *this driver will not pretend* (the reconstruction
/// mismatch, and a contestant it cannot run).
///
/// **Written as prose and folded by machine** ([`why_over`], via
/// [`suggest::wrap_to`]). These are view bodies, and `screen::wrap` sizes the
/// composed frame to its widest line — so one over-wide paragraph here would
/// shove the board sideways for exactly as long as the refusal is on screen
/// (`suggest_cmd`'s live refusal carries the same warning, for the same
/// defect). Hand-wrapping cannot buy that here, because every arm below
/// interpolates something whose length it does not control.
#[derive(Debug)]
pub enum Refusal {
    LiveMode,
    NoBotMoveYet,
    OutOfRange { asked: usize, events: usize },
    NotAMove { index: usize, kind: &'static str },
    HumanSeat { index: usize, seat: PlayerId },
    Rung { index: usize, seat: PlayerId, rung: String },
    Diagnostic { index: usize, seat: PlayerId, why: String },
    UnknownController { index: usize, seat: PlayerId, controller: String, why: String },
    Mismatch(Box<Mismatch>),
    Reconstruction { index: usize, seat: PlayerId, why: String },
    Unaccounted { index: usize, contestant: String },
    Replay { index: usize, message: String },
}

/// The mismatch refusal's parts, boxed so the one heavy variant does not size
/// the whole enum (clippy's `large_enum_variant`, and it is the rarest arm).
#[derive(Debug)]
pub struct Mismatch {
    pub index: usize,
    pub seat: PlayerId,
    pub controller: String,
    /// `Debug`, not a description: the reconstructed move need not be legal in
    /// the TRUE position (it was chosen inside a sampled world), and naming a
    /// move against a state that does not admit it is how a formatter gets to
    /// index something that is not there — `SuggestError::Inconsistent`'s own
    /// reasoning, at the one other site with the same exposure.
    pub reconstructed: String,
    /// The recorded move in words, which is always safe: it applied here.
    pub recorded: String,
    pub recorded_version: Option<u32>,
    pub current_version: u32,
}

/// Every arm is written as PROSE — one logical line per paragraph, with a `\n`
/// only where the layout genuinely needs one (the mismatch's two named moves).
/// [`why_over`] folds the result to the reader's measure, which is the only way
/// a message carrying an unbounded interpolation can be bounded at all.
impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LiveMode => write!(
                f,
                "`why` is simulation-only. A live-tracked game has no bot seats at all — a live \
                 save that names one drops it loudly on load, because a bot cannot read a \
                 face-down card off your table — so there is no bot decision here to reconstruct. \
                 (`suggest` is refused in live mode too, for the neighbouring reason: see guide \
                 §4.)"
            ),
            Self::NoBotMoveYet => write!(
                f,
                "No bot move to explain yet. `why` accounts for a move a BOT seat played, and \
                 this game has recorded none — a rung seat's samples do not count, since a rung \
                 has no valuation to show. What the bot would play on YOUR seat is `suggest`'s \
                 question, asked before you move rather than after."
            ),
            Self::OutOfRange { asked, events } if *events == 0 => write!(
                f,
                "There is no event {asked} to explain: this game has no events recorded yet."
            ),
            Self::OutOfRange { asked, events } => write!(
                f,
                "There is no event {asked} to explain: the log runs 0-{} ({events} event(s) \
                 recorded). `why <n>` takes the number `log` prints beside each line.",
                events - 1,
            ),
            Self::NotAMove { index, kind } => write!(
                f,
                "Event {index} is {kind}, not a move — there was no decision to make, so there is \
                 nothing to account for. `log` shows what it recorded."
            ),
            Self::HumanSeat { index, seat } => write!(
                f,
                "Event {index} was seat {}'s own move, and the record names no bot on that seat. \
                 `why` accounts for BOT moves; what the bot would play on YOUR seat is \
                 `suggest`'s question, asked before you move rather than after.",
                seat.0,
            ),
            Self::Rung { index, seat, rung } => write!(
                f,
                "Seat {} played event {index} as `{rung}`, a rollout RUNG: it SAMPLED that move \
                 from a distribution rather than valuing it, so there is no arithmetic to show. \
                 That is a different fact from a score of zero. Rungs are kept so that every save \
                 this driver ever wrote still resumes; a bot seat configured today plays a \
                 search, which does account for its moves.",
                seat.0,
            ),
            Self::Diagnostic { index, seat, why } => write!(
                f,
                "Seat {} is recorded at event {index} under a controller this driver will not \
                 run: {why}. The seat resumed as a HUMAN seat and this driver never ran that \
                 search, so there is no decision of its here to reconstruct.",
                seat.0,
            ),
            Self::UnknownController { index, seat, controller, why } => write!(
                f,
                "Seat {} is recorded at event {index} under controller \"{controller}\", which \
                 this build does not know how to run ({why}) — so `why` cannot rebuild the \
                 decision it would have made. The seat resumed as a HUMAN seat.",
                seat.0,
            ),
            Self::Mismatch(m) => {
                let recorded_version =
                    m.recorded_version.map_or_else(|| "none".to_string(), |v| format!("v{v}"));
                write!(
                    f,
                    "`why` cannot account for event {} honestly. Replaying seat {}'s decision \
                     under `{}` reproduced\n  \
                     {}\n\
                     but the record says the move played was\n  \
                     {}\n\
                     The record stores this controller at version {recorded_version} and this \
                     build runs v{}. Rather than show you the arithmetic for a move that was not \
                     played, `why` stops here — this is also how a seat that was downgraded to \
                     HUMAN mid-game reads, since its later moves were typed rather than searched. \
                     The game is untouched.",
                    m.index,
                    m.seat.0,
                    m.controller,
                    m.reconstructed,
                    m.recorded,
                    m.current_version,
                )
            }
            Self::Reconstruction { index, seat, why } => write!(
                f,
                "`why` could not re-run seat {}'s decision at event {index}: {why} The game is \
                 untouched.",
                seat.0,
            ),
            Self::Unaccounted { index, contestant } => write!(
                f,
                "`{contestant}` reproduced event {index}'s move but explained nothing, which is a \
                 broken contract in this program rather than something you did. Worth reporting. \
                 The game is untouched."
            ),
            Self::Replay { index, message } => write!(
                f,
                "`why` could not rebuild the position event {index} was decided in: {message} \
                 The game is untouched."
            ),
        }
    }
}

/// What the RECORD says played a seat, narrowed to what `why` can do about it.
enum SeatReading {
    /// An evaluator arm that accounts for its move.
    Explainable(Contestant),
    /// A human seat — or a record naming no controller for this seat at all,
    /// which is the same fact from the other side (nothing here was a bot).
    Human,
    /// A preserved rollout rung, named.
    Rung(String),
    /// A labelled diagnostic arm, carrying [`bot::eligible`]'s own sentence.
    Diagnostic(String),
    /// A name this build cannot parse, carrying the parser's own sentence.
    Unknown { controller: String, why: String },
}

/// The record's provenance for `seat`, if it carries any.
fn seat_provenance(record: &GameRecord, seat: PlayerId) -> Option<&SeatProvenance> {
    record.run.as_ref()?.seats.get(usize::from(seat.0))
}

/// Read `seat`'s recorded controller through the SAME two functions the resume
/// path uses — `Contestant::from_parts` over both stored halves, then
/// [`bot::eligible`] — so `why` and `BotSeats::from_provenance` cannot disagree
/// about what a seat is. The extra narrowing here is the rung: eligible to
/// PLAY (grandfathered), never eligible to EXPLAIN.
fn read_seat(record: &GameRecord, seat: PlayerId) -> SeatReading {
    let Some(provenance) = seat_provenance(record, seat) else { return SeatReading::Human };
    if provenance.controller == CONTROLLER_HUMAN {
        return SeatReading::Human;
    }
    match Contestant::from_parts(&provenance.controller, &provenance.params) {
        Err(why) => SeatReading::Unknown { controller: provenance.controller.clone(), why },
        Ok(contestant) => match bot::eligible(contestant) {
            Err(why) => SeatReading::Diagnostic(why),
            Ok(Contestant::Policy(rung)) => SeatReading::Rung(rung.as_str().to_string()),
            Ok(explainable) => SeatReading::Explainable(explainable),
        },
    }
}

/// Which kind of event this is, in the words a refusal names it with.
fn event_kind(event: &RecordedEvent) -> &'static str {
    match event {
        RecordedEvent::Move(_) => "a move",
        RecordedEvent::RevealOrders(_) => "an order-deck reveal",
        RecordedEvent::RevealTechs(_) => "a technology-deck reveal",
        RecordedEvent::RevealStartingOrder { .. } => "a starting-order reveal",
    }
}

/// Bare `why`'s address: the HIGHEST event index that is a move AND whose
/// mover's recorded controller is one this driver can account for.
///
/// One FORWARD fold rather than a backwards `state_at` walk, and the
/// difference is not cosmetic: the backwards form re-replays the record from
/// event 0 at every step, so an all-human game — the case this scan exists to
/// refuse — would cost `O(events²)` checked applies before printing "no bot
/// move yet".
///
/// The `ReplayError`'s own `event_index` is DISCARDED and only its message
/// kept: each iteration hands `replay_from` a fresh one-element slice, so that
/// index is structurally always 0 whatever really failed (the trap
/// `Session::load` documents at its own fold). The index printed is this
/// loop's.
fn last_explainable_bot_move(record: &GameRecord, rules: &Rules) -> Result<usize, Refusal> {
    let mut state =
        state_at(record, 0, rules).map_err(|e| Refusal::Replay { index: 0, message: e.message })?;
    let mut found = None;
    for (index, event) in record.events.iter().enumerate() {
        if matches!(event, RecordedEvent::Move(_))
            && matches!(read_seat(record, state.current_player), SeatReading::Explainable(_))
        {
            found = Some(index);
        }
        state = replay_from(&state, std::slice::from_ref(event), rules)
            .map_err(|e| Refusal::Replay { index, message: e.message })?;
    }
    found.ok_or(Refusal::NoBotMoveYet)
}

/// Resolve the address, reconstruct the decision, gate it against the record,
/// and render — design §2.1-§2.4 in the order it states them.
fn account(
    mode: TrackingMode,
    record: &GameRecord,
    rules: &Rules,
    bots: &BotSeats,
    arg: Option<usize>,
) -> Result<String, Refusal> {
    let a = account_parts(mode, record, rules, bots, arg)?;
    Ok(render(a.reconstruction, a.mv, a.explain, &a.state, rules))
}

/// **Where `why` looks, before any search** (security S2c, decision D-6): the index `arg` names or the last
/// explainable bot move, the state that move was decided in (`state_at(record, n)`), its mover and the mover's
/// contestant, with every pre-search refusal in [`account_parts`]'s order — pure replay, so a caller that stops
/// here has paid nothing a hidden hand could vary. The layer's `why` reads the requester's visibility on this and
/// reaches [`account_from`] only when the account may be served.
pub struct Located {
    pub index: usize,
    pub seat: PlayerId,
    pub state: GameState,
    pub recorded: Move,
    pub contestant: Contestant,
    pub recorded_version: Option<u32>,
    pub current_version: u32,
}

pub fn locate(mode: TrackingMode, record: &GameRecord, rules: &Rules, arg: Option<usize>) -> Result<Located, Refusal> {
    if matches!(mode, TrackingMode::Live) {
        return Err(Refusal::LiveMode);
    }
    let events = record.events.len();
    let index = match arg {
        Some(asked) if asked < events => asked,
        Some(asked) => return Err(Refusal::OutOfRange { asked, events }),
        None => last_explainable_bot_move(record, rules)?,
    };
    let RecordedEvent::Move(recorded) = &record.events[index] else {
        return Err(Refusal::NotAMove { index, kind: event_kind(&record.events[index]) });
    };
    // The state the mover DECIDED IN — `state_at(record, n)` is the state
    // after the first n events, which is the position event n was chosen at.
    // `index < events` above, so the past-the-end arm of `state_at` is
    // unreachable from here; its message is still never printed raw (its
    // "replay failed at event N" framing would name an event that replayed
    // perfectly).
    let state = state_at(record, index, rules).map_err(|e| Refusal::Replay { index, message: e.message })?;
    let seat = state.current_player;
    let contestant = match read_seat(record, seat) {
        SeatReading::Explainable(contestant) => contestant,
        SeatReading::Human => return Err(Refusal::HumanSeat { index, seat }),
        SeatReading::Rung(rung) => return Err(Refusal::Rung { index, seat, rung }),
        SeatReading::Diagnostic(why) => return Err(Refusal::Diagnostic { index, seat, why }),
        SeatReading::Unknown { controller, why } => return Err(Refusal::UnknownController { index, seat, controller, why }),
    };
    let (_, current_version, _) = contestant.descriptor();
    let recorded_version = seat_provenance(record, seat).map(|p| p.version);
    Ok(Located { index, seat, state, recorded: recorded.clone(), contestant, recorded_version, current_version })
}

/// The search half: the decision reconstructed from the located state under the pump's own key, gated against the
/// record, the explanation and the forced fact — [`account_parts`]'s second half, verbatim.
pub fn account_from(located: Located, bots: &BotSeats, rules: &Rules) -> Result<WhyAccount, Refusal> {
    let Located { index, seat, state, recorded, contestant, recorded_version, current_version } = located;
    // THE PUMP'S OWN KEY: `BotSeats::decision_seed(n)` over the RECONCILED
    // stream seed — the one `from_provenance` kept (first bot seat's wins, and
    // a conflict has already warned at load). Deliberately not the mover's own
    // `agent_seed`, which would disagree with the pump for the second bot seat
    // of a conflicting record, and deliberately not `suggest::stream_seed`,
    // which is a DIFFERENT derivation: reporting a search the bot never ran is
    // the one defect this whole seam exists to make impossible.
    let decision_seed = bots.decision_seed(index);
    let decision = bot::decide(contestant, &state, rules, seat, decision_seed)
        .map_err(|e| Refusal::Reconstruction { index, seat, why: e.to_string() })?;
    let (controller, _, _) = contestant.descriptor();
    if decision.mv != recorded {
        return Err(Refusal::Mismatch(Box::new(Mismatch {
            index,
            seat,
            controller: contestant.echo(),
            reconstructed: format!("{:?}", decision.mv),
            recorded: describe::describe_move(&recorded, &state, rules),
            recorded_version,
            current_version,
        })));
    }
    let explain = decision.explain.ok_or_else(|| Refusal::Unaccounted { index, contestant: controller.clone() })?;
    // THE FORCED-POSITION FACT, read off the engine rather than off the search
    // ([`FORCED_LEAD`]). Computed here, past every refusal, so a screen that is
    // never rendered never pays for the exhaustive enumeration.
    let forced = get_legal_moves(&state, rules).len() == 1;
    Ok(WhyAccount {
        reconstruction: Reconstruction { contestant, index, seat, decision_seed, recorded_version, current_version, forced },
        mv: recorded,
        explain,
        state,
    })
}

/// [`account`] without the wording — see [`WhyAccount`].
///
/// Every refusal in this module is raised here, by its two halves [`locate`] and
/// [`account_from`], so the two surfaces cannot disagree about what is
/// explainable: the screen is this function's output with words on it, and
/// nothing else consults the record.
pub fn account_parts(mode: TrackingMode, record: &GameRecord, rules: &Rules, bots: &BotSeats, arg: Option<usize>) -> Result<WhyAccount, Refusal> {
    account_from(locate(mode, record, rules, arg)?, bots, rules)
}

/// Which grouping line a `counter_rows` name belongs on.
///
/// **Exhaustive by NAME, with no catch-all.** The names are core's vocabulary
/// verbatim (spec §2.6: one vocabulary across the instruments), and a name this
/// list does not carry is a counter that arrived after this screen was written
/// — the [`Ungrouped`](Grouping::Ungrouped) arm exists so that fact reaches the
/// reader instead of being swallowed by a `_ =>`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Grouping {
    /// Work the search did.
    Work,
    /// A budget or a guard that bit.
    Limit,
    /// A name this build has no line for — a defect, printed as one.
    Ungrouped,
}

/// [`Grouping`] for one counter name.
fn grouping_of(name: &str) -> Grouping {
    match name {
        "root_candidates" | "nodes" | "handles" | "route_dfs_nodes" | "forced_nodes" | "leaves"
        | "scored" | "root_scored" | "max_line" | "free_extension_steps" => Grouping::Work,
        "budget_binds" | "budget_starved" | "placement_cap_binds" | "ply_cap_hits"
        | "free_cap_hits" => Grouping::Limit,
        _ => Grouping::Ungrouped,
    }
}

/// The counter block: `rows` grouped onto at most three labelled lines, in
/// `counter_rows`' own declaration order within each.
///
/// Takes a SLICE rather than a `&SearchStats` so the ungrouped arm can be
/// driven at all: a sixteenth counter cannot be manufactured from this crate,
/// which is precisely what core's no-`..` destructure guarantees.
///
/// The names are printed VERBATIM — they are the same strings the arena's
/// report, the probe and this screen all use, and a friendlier spelling here
/// would mean a reader could not grep one instrument's output with another's
/// vocabulary.
fn counter_lines(rows: &[(&str, u64)]) -> Vec<String> {
    let mut lines = Vec::new();
    for (label, group) in [
        (SEARCH_COST_LABEL, Grouping::Work),
        (GUARD_LABEL, Grouping::Limit),
        (UNGROUPED_LABEL, Grouping::Ungrouped),
    ] {
        let cells: Vec<String> = rows
            .iter()
            .filter(|(name, _)| grouping_of(name) == group)
            .map(|(name, value)| format!("{name} {value}"))
            .collect();
        if !cells.is_empty() {
            lines.push(format!(
                "{label} {}",
                cells.join(&format!(" {} ", crate::style::glyph::SEPARATOR)),
            ));
        }
    }
    lines
}

/// What the header says about the reconstruction, as one argument rather than
/// six positional ones.
///
/// Readable rather than private because it is also the PROVENANCE half of
/// [`WhyAccount`]: a consumer that renders its own screen still has to say
/// which contestant, at which event, under which key, or it is showing
/// arithmetic with no claim attached to it.
pub struct Reconstruction {
    pub contestant: Contestant,
    pub index: usize,
    pub seat: PlayerId,
    pub decision_seed: u64,
    pub recorded_version: Option<u32>,
    pub current_version: u32,
    /// The engine's legal list held exactly one move at the decision state —
    /// see [`FORCED_LEAD`].
    pub forced: bool,
}

/// A successful reconstruction, BEFORE any of it is worded.
///
/// The split exists because `why`'s account and `why`'s screen are two
/// different things and only one of them is a terminal's business. Everything
/// up to here is the honesty work — resolve the event, replay to the decision
/// state, read the seat's controller, re-derive the pump's own key, run the
/// same `decide`, and refuse if the result is not the move the record carries.
/// Everything after it is prose at 80 columns. A consumer that draws its own
/// table wants the first half and none of the second, and it must not
/// reimplement the first half: the refusals ARE the honesty gate.
pub struct WhyAccount {
    pub reconstruction: Reconstruction,
    /// The reconstructed move, already proven equal to the recorded one.
    pub mv: Move,
    pub explain: bot::DecisionExplanation,
    /// The state the mover decided in. Carried because the words for a move,
    /// and for every move of the continuation, are computed against the
    /// position they were chosen at — not against the position now.
    pub state: GameState,
}

/// The screen: `why`'s own header over `suggest`'s body (design §2.4 — the
/// table is not forked, and the parameterization is a header, not a rewrite).
fn render(
    r: Reconstruction,
    mv: Move,
    explain: bot::DecisionExplanation,
    state: &GameState,
    rules: &Rules,
) -> String {
    let (controller, _, _) = r.contestant.descriptor();
    let mut notes = vec![(
        Tone::Dim,
        format!(
            "This move was reconstructed by replay under THIS build's consts — evaluator impl {}, \
             {controller} impl v{} — from decision seed {:#018x}, the key the bot's own pump \
             derived from the recorded stream seed and the event count.",
            eval::EVAL_IMPL_VERSION,
            r.current_version,
            r.decision_seed,
        ),
    )];
    // THE WHOLE COUNTER SURFACE, not a chosen two of it (spec §2.6). What
    // stood here printed `scored` and `root_candidates` and dropped the other
    // thirteen — including every counter that says whether a budget or a guard
    // bit, which is exactly what a reader asking "why that move" needs when the
    // answer is "the search ran out of room".
    if let Some(stats) = explain.stats {
        for line in counter_lines(&driven_runner::counter_rows(&stats)) {
            notes.push((Tone::Dim, line));
        }
    }
    // Version drift WARNS and does not refuse: the move still reproduced, so
    // the table is a true account of a decision an older build made — but the
    // arithmetic below is this build's, and a reader comparing two runs needs
    // to know that. Params drift (a moved weight table, a bumped eval impl) is
    // already warned per field at LOAD by `BotSeats::from_provenance`, which is
    // where a cross-build resume announces itself.
    if r.recorded_version.is_some_and(|recorded| recorded != r.current_version) {
        notes.push((
            Tone::Warn,
            format!(
                "WARNING: the record stores {controller} at version {}, and this build runs \
                 version {} — the move still reproduces, so the account below is THIS build's \
                 reading of a move an older one played.",
                r.recorded_version.unwrap_or_default(),
                r.current_version,
            ),
        ));
    }

    let header = Header {
        title: "Why",
        identity: format!(" · {} · event {}", r.contestant.echo(), r.index),
        notes,
        move_label: "The move it played:",
        // By the ARM rather than by the data: greedy reports no line and no
        // ranking BY CONSTRUCTION, which is a different fact from a search
        // that happened to find one candidate. Exhaustive with no `_`, the
        // `bot::eligible` discipline — a contestant added to core lands here
        // as a question rather than as a default.
        explanation_only: match r.contestant {
            Contestant::GreedyV1 { .. } => Some(ONE_PLY_ACCOUNT),
            // A SEARCH has a line and a ranking, so there is no leaner block to
            // fall back to. The named arms belong here and not in the group
            // below: since the 2026-08-30 ship commit one of them is the seat
            // this driver plays, and a named arm IS a `LookaheadAgent` — same
            // continuation, same ranked roots, same fifteen counters.
            Contestant::LookaheadV1 { .. } | Contestant::NamedTable(_) => None,
            // Unreachable: `read_seat` routes through `bot::eligible`, which
            // admits no rung to the EXPLAINABLE reading (a rung reports as
            // `SeatReading::Rung`) and refuses both diagnostic arms outright.
            Contestant::Policy(_)
            | Contestant::MctsV1 { .. }
            | Contestant::AtomicDiag { .. } => None,
        },
        forced_lead: r.forced.then_some(FORCED_LEAD),
    };

    let suggestion = Suggestion {
        seat: r.seat,
        mv,
        explanation: explain.explanation,
        continuation: explain.continuation,
        ranking: explain.ranking,
        // The stream field is the HEADER's business and this header prints the
        // decision seed itself, so nothing reads this one — it is filled with
        // the same value rather than a placeholder so the struct cannot lie.
        stream_seed: r.decision_seed,
        search_hit_ply_cap: explain.hit_ply_cap,
    };
    suggest::render_with_header(&suggestion, &header, state, rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::{self, PROVISIONAL_DEFAULT_BOT};
    use crate::repl::Repl;
    use crate::session::Session;
    use std::sync::OnceLock;
    use yokohama_core::lookahead::SearchStats;

    /// The bot stream seed every fixture here keys with. A LITERAL rather than
    /// a draw: `why`'s whole claim is that it re-derives the pump's own key, so
    /// a fixture whose seed moved per run could not tell a reconstruction that
    /// used the right stream from one that used any stream at all.
    const FIXTURE_BOT_SEED: u64 = 0x5A17_3B0C_9E44_D218;

    /// A 2P simulation whose seat 1 is played by `contestant`, built the way
    /// the WIZARD builds one — provenance stamped on the session, `Repl::new`
    /// reading the table back out of it — so every fixture here goes through
    /// the same resume path a save does rather than a test-only constructor.
    fn bot_repl(seed: u64, contestant: Contestant) -> Repl {
        let bots = BotSeats::new(vec![None, Some(contestant)], FIXTURE_BOT_SEED);
        let mut session = Session::new_sim(
            GameConfig::for_player_count(2),
            seed,
            vec![StartingOrderChoice::Keep(0); 2],
            Rules::standard(),
        )
        .expect("a standard 2-player simulation");
        session.set_run_seats(bots.seat_provenance());
        Repl::new(session)
    }

    /// A 2P simulation with no bot seat and no run provenance at all — the
    /// shape a game started before anyone answered the bot question has.
    fn human_repl(seed: u64) -> Repl {
        Repl::new(
            Session::new_sim(
                GameConfig::for_player_count(2),
                seed,
                vec![StartingOrderChoice::Keep(0); 2],
                Rules::standard(),
            )
            .expect("a standard 2-player simulation"),
        )
    }

    /// A LIVE session, the one mode `why` refuses outright.
    fn live_repl(seed: u64) -> Repl {
        Repl::new(
            Session::new_live(
                GameConfig::for_player_count(2),
                seed,
                SetupObservations::default(),
                vec![StartingOrderChoice::Keep(0); 2],
                Rules::standard(),
            )
            .expect("a standard 2-player live game"),
        )
    }

    /// Play `r` forward until `done` holds (or the game ends), and say how many
    /// steps it took.
    ///
    /// **The BOT half runs the shipped pump** — `pump_bots`, keyed by
    /// `(bot_seed, events.len())` — because that is precisely the decision
    /// `why` claims to reconstruct; a fixture that chose the bot's moves itself
    /// would pin nothing. The HUMAN half takes moves off the ENGINE's own legal
    /// list, ROTATING the index rather than always taking `[0]` (`suggest`'s
    /// `sim_mid_game` idiom, for its reason: a take-the-first line degenerates,
    /// never builds anything and never pays a fee).
    fn play(r: &mut Repl, limit: usize, mut done: impl FnMut(&Repl) -> bool) -> usize {
        for step in 0..limit {
            if r.session.is_over() || done(r) {
                return step;
            }
            if r.bots.is_bot(r.session.state().current_player) {
                r.pump_bots();
            } else {
                let legal = r.session.legal();
                let mv = legal[step % legal.len()].clone();
                r.session.apply(mv, vec![]).expect("a move from the engine's own list applies");
            }
        }
        panic!("the fixture's condition was not reached within {limit} steps");
    }

    /// Every event index whose mover was `seat`.
    ///
    /// An INDEPENDENT forward fold, deliberately: this module's own address
    /// resolution is the thing under test, so a fixture that asked it which
    /// events were the bot's would be checking a claim against itself.
    fn move_indices_of(record: &GameRecord, rules: &Rules, seat: PlayerId) -> Vec<usize> {
        let mut state = state_at(record, 0, rules).expect("the post-setup state");
        let mut out = Vec::new();
        for (i, ev) in record.events.iter().enumerate() {
            if matches!(ev, RecordedEvent::Move(_)) && state.current_player == seat {
                out.push(i);
            }
            state = replay_from(&state, std::slice::from_ref(ev), rules).expect("the record folds");
        }
        out
    }

    /// Every index of [`move_indices_of`] whose decision point offered MORE
    /// THAN ONE move.
    ///
    /// **Load-bearing rather than tidy**, and `suggest`'s own fixtures learned
    /// it first: the bot's first move of a game is the forced
    /// `End additional-action phase`, so a fixture that stopped at the first
    /// bot move at all would render "No runner-up: this decision point offered
    /// exactly one move" and a one-step line — exercising neither the
    /// runner-up rows nor the claim that greedy's leaner block replaces a block
    /// that would otherwise have had content.
    fn branching_move_indices_of(record: &GameRecord, rules: &Rules, seat: PlayerId) -> Vec<usize> {
        move_indices_of(record, rules, seat)
            .into_iter()
            .filter(|&i| {
                let state = state_at(record, i, rules).expect("the decision state");
                get_legal_moves(&state, rules).len() > 1
            })
            .collect()
    }

    /// Play until the bot has moved at all, and hand back its FIRST move —
    /// asserted here to be a FORCED one, which is the fixture's whole point.
    ///
    /// The assertion is the fixture rather than a decoration: the bot's first
    /// move of a game is the `End additional-action phase` its opening turn
    /// begins with, and that position offers exactly one move
    /// ([`branching_move_indices_of`]'s doc is the same observation from the
    /// other side). Asserting it means a rules change that gives that position
    /// a second move fails HERE, naming the fixture, instead of quietly
    /// un-testing the forced lead.
    fn play_to_a_forced_bot_move(r: &mut Repl) -> usize {
        let rules = Rules::standard();
        play(r, 400, |r| {
            !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty()
        });
        let first = *move_indices_of(r.session.record(), &rules, PlayerId::new(1))
            .first()
            .expect("fixture: the bot must have moved");
        let state = state_at(r.session.record(), first, &rules).expect("the decision state");
        assert_eq!(
            get_legal_moves(&state, &rules).len(),
            1,
            "fixture: the bot's first move must be a FORCED one",
        );
        first
    }

    /// Play until the bot has made a move at a branching decision point, and
    /// hand back the first such index.
    fn play_to_a_branching_bot_move(r: &mut Repl) -> usize {
        let rules = Rules::standard();
        play(r, 400, |r| {
            !branching_move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1))
                .is_empty()
        });
        *branching_move_indices_of(r.session.record(), &rules, PlayerId::new(1))
            .first()
            .expect("fixture: the bot must have decided at a branching point")
    }

    /// The move recorded at `index`, in the words the screen names it with.
    fn described_move_at(record: &GameRecord, rules: &Rules, index: usize) -> String {
        let state = state_at(record, index, rules).expect("the decision state");
        let RecordedEvent::Move(mv) = &record.events[index] else { panic!("event {index} is not a move") };
        crate::describe::describe_move(mv, &state, rules)
    }

    /// Whitespace-normalized, so a move description the render REFLOWED across
    /// an indented second line still compares equal to the one-line original
    /// (`wrap_words` splits on whitespace and joins with single spaces, and the
    /// answer wraps rather than eliding — `suggest`'s two treatments).
    fn flat(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// The marker a real `why` screen carries and no refusal does.
    const ACCOUNTED: &str = "Score at that point:";

    /// A completed 2P human-vs-bot game, played ONCE for the whole binary.
    ///
    /// Two tests need a finished game (the after-game-over pin and the
    /// every-move pin) and it is the most expensive fixture here — a full game
    /// of turn-boundary search — so it is memoized rather than replayed, the
    /// `policy_diversity` precedent. Only the RECORD and the seat table travel:
    /// `why` reads nothing else, which is itself worth stating.
    struct Completed {
        record: GameRecord,
        bots: BotSeats,
        rules: Rules,
        bot_moves: Vec<usize>,
    }

    fn completed_game() -> &'static Completed {
        static GAME: OnceLock<Completed> = OnceLock::new();
        GAME.get_or_init(|| {
            let mut r = bot_repl(20_260_812, PROVISIONAL_DEFAULT_BOT);
            play(&mut r, 5_000, |_| false);
            assert!(r.session.is_over(), "the fixture must reach a finished game");
            let rules = Rules::standard();
            let record = r.session.record().clone();
            let bot_moves = move_indices_of(&record, &rules, PlayerId::new(1));
            Completed { record, bots: r.bots.clone(), rules, bot_moves }
        })
    }

    // ---- the refusal table (design §2.3), one test per row -----------------

    /// Live tracking has no bot seats at all — a live save that names one drops
    /// it loudly on load — so there is no bot decision here to reconstruct.
    #[test]
    fn why_refuses_in_live_mode() {
        let r = live_repl(21);
        let out = why(&r.session, &r.bots, None);
        assert!(out.contains("simulation-only"), "{out}");
        assert!(out.contains("live"), "the refusal must name the mode it is about: {out}");
        assert!(!out.contains(ACCOUNTED), "a refusal must not print an account: {out}");
        assert_eq!(why(&r.session, &r.bots, Some(0)), out, "the indexed form refuses the same way");
    }

    /// Bare `why` before any bot has moved says so, rather than reaching for
    /// the human's own last move.
    #[test]
    fn why_refuses_with_no_bot_move_yet() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        play(&mut r, 40, |r| r.session.record().events.len() >= 3);
        let rules = Rules::standard();
        assert!(
            move_indices_of(r.session.record(), &rules, PlayerId::new(1)).is_empty(),
            "fixture: the bot must not have moved yet",
        );

        let out = why(&r.session, &r.bots, None);
        assert!(out.contains("No bot move"), "{out}");
        assert!(out.contains("suggest"), "and it points at the command that DOES answer: {out}");
        assert!(!out.contains(ACCOUNTED), "{out}");
    }

    /// An index past the end names the valid RANGE, and never the underlying
    /// `ReplayError`'s `Display` form (whose "replay failed at event N" prefix
    /// would claim a failure at an event that replayed perfectly — T2 review
    /// NOTE-2, carried to this task).
    #[test]
    fn why_refuses_an_out_of_range_index_naming_the_range() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        play(&mut r, 40, |r| r.session.record().events.len() >= 3);
        let n = r.session.record().events.len();

        for asked in [n, n + 1, 9_999] {
            let out = why(&r.session, &r.bots, Some(asked));
            assert!(out.contains(&format!("0-{}", n - 1)), "the valid range, for {asked}: {out}");
            assert!(out.contains(&asked.to_string()), "and what was asked for: {out}");
            assert!(!out.contains("replay failed"), "never the ReplayError Display form: {out}");
            assert!(!out.contains(ACCOUNTED), "{out}");
        }

        // A game with nothing recorded has no range to name at all, which is a
        // different sentence rather than the degenerate `0--1`.
        let fresh = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let out = why(&fresh.session, &fresh.bots, Some(0));
        assert!(out.contains("no events"), "{out}");
        assert!(!out.contains("0-"), "there is no range to print: {out}");
    }

    /// An event that is a REVEAL is not a decision, and the refusal says which
    /// kind of reveal it is.
    ///
    /// The fixture SPLICES the reveal onto a simulation record, because the sim
    /// driver never writes one (reveals are live-mode choreography) while the
    /// record TYPE admits one everywhere. It is spliced at the END and
    /// addressed directly, so `state_at(record, n)` folds `events[..n]` and
    /// never the spliced event itself — which is what lets the starting-order
    /// row (unreplayable in a simulation, where no seat's order is hidden) be
    /// covered on the same footing as the two deck reveals.
    #[test]
    fn why_refuses_a_reveal_index_naming_it() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        play(&mut r, 40, |r| r.session.record().events.len() >= 3);
        let rules = Rules::standard();
        let base = r.session.record().clone();
        let top_order = *r.session.state().order_deck.last().expect("a card on the order deck");
        let top_tech = *r.session.state().tech_deck.last().expect("a card on the tech deck");

        for (event, word) in [
            (RecordedEvent::RevealOrders(vec![top_order]), "order"),
            (RecordedEvent::RevealTechs(vec![top_tech]), "technology"),
            (
                RecordedEvent::RevealStartingOrder { player: PlayerId::new(1), card: top_order },
                "starting-order",
            ),
        ] {
            let mut record = base.clone();
            record.events.push(event);
            let n = record.events.len() - 1;
            let out = why_over(TrackingMode::Simulation, &record, &rules, &r.bots, Some(n));
            assert!(out.contains("reveal"), "{out}");
            assert!(out.contains(word), "the refusal must say WHICH kind: {out}");
            assert!(out.contains(&n.to_string()), "and which event: {out}");
            assert!(!out.contains(ACCOUNTED), "{out}");
        }
    }

    /// A human seat's move is `suggest`'s question, asked before you play it —
    /// the OD-4(i) split made a sentence.
    #[test]
    fn why_points_a_human_move_at_suggest() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        play(&mut r, 40, |r| r.session.record().events.len() >= 3);
        let out = why(&r.session, &r.bots, Some(0));
        assert!(out.contains("suggest"), "{out}");
        assert!(out.contains("seat 0"), "the refusal names the seat: {out}");
        assert!(!out.contains(ACCOUNTED), "{out}");

        // ...and a record carrying no run provenance at all reads the same way:
        // nothing there was played by a bot.
        let mut plain = human_repl(4);
        play(&mut plain, 40, |r| r.session.record().events.len() >= 3);
        let out = why(&plain.session, &plain.bots, Some(0));
        assert!(out.contains("suggest"), "{out}");
        assert!(!out.contains(ACCOUNTED), "{out}");
    }

    /// A rollout RUNG samples its move from a distribution: there is no
    /// valuation to show, which is a different fact from a zero one. The
    /// refusal names the rung, and bare `why` skips rung moves entirely rather
    /// than landing on one and refusing.
    #[test]
    fn why_names_a_rung_seat_as_sampled_not_valued() {
        let rung = Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT);
        let mut r = bot_repl(4, rung);
        let rules = Rules::standard();
        play(&mut r, 200, |r| !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty());
        let played = move_indices_of(r.session.record(), &rules, PlayerId::new(1));
        assert!(!played.is_empty(), "fixture: the rung seat must have moved");

        let out = why(&r.session, &r.bots, Some(played[0]));
        assert!(out.contains("family-weighted"), "the rung is named: {out}");
        // Case-insensitive on purpose: the pin is the CLAIM (this move was
        // sampled, not valued), and the screen's own emphasis capitalization is
        // typography rather than content.
        assert!(out.to_lowercase().contains("sampled"), "{out}");
        assert!(!out.contains(ACCOUNTED), "{out}");

        let bare = why(&r.session, &r.bots, None);
        assert!(
            bare.contains("No bot move"),
            "bare `why` targets EXPLAINABLE moves only, so a rung game has none: {bare}",
        );
    }

    /// A reconstruction that disagrees with the record REFUSES, naming both
    /// moves and both version consts, rather than presenting an account of a
    /// move that was never played.
    ///
    /// The fixture is the mid-history-downgrade shape design §2.2 names: a
    /// record whose provenance says `lookahead-v1` over a move a human typed.
    /// The human's move is chosen AGAINST the reconstruction — the test asks
    /// what the search would play and then plays something else — so the pin
    /// cannot pass by a coincidence of the seed.
    #[test]
    fn why_refuses_a_reconstruction_mismatch_naming_both_moves() {
        let bots = BotSeats::new(vec![Some(PROVISIONAL_DEFAULT_BOT), None], FIXTURE_BOT_SEED);
        let mut session = Session::new_sim(
            GameConfig::for_player_count(2),
            4,
            vec![StartingOrderChoice::Keep(0); 2],
            Rules::standard(),
        )
        .expect("a 2P simulation");
        let rules = Rules::standard();

        // Walk seat 0 to a branching decision point, then play a move the
        // search does NOT choose at this position's own key.
        let mut n = 0usize;
        loop {
            assert!(n < 40, "no branching seat-0 decision point within 40 moves");
            let legal = session.legal();
            if legal.len() > 1 && session.state().current_player == PlayerId::new(0) {
                break;
            }
            session.apply(legal[0].clone(), vec![]).expect("legal applies");
            n += 1;
        }
        let seat = session.state().current_player;
        let searched = bot::decide(
            PROVISIONAL_DEFAULT_BOT,
            session.state(),
            &rules,
            seat,
            bots.decision_seed(n),
        )
        .expect("the search decides")
        .mv;
        let typed = session
            .legal()
            .into_iter()
            .find(|m| *m != searched)
            .expect("fixture: a branching point offers a second move");
        session.apply(typed.clone(), vec![]).expect("legal applies");
        session.set_run_seats(bots.seat_provenance());

        let out = why(&session, &bots, Some(n));
        assert!(!out.contains(ACCOUNTED), "a mismatch must not print an account: {out}");
        assert!(out.contains(&format!("{searched:?}")), "the reconstructed move: {out}");
        assert!(
            flat(&out).contains(&flat(&described_move_at(session.record(), &rules, n))),
            "and the move the record says was played: {out}",
        );
        let version = PROVISIONAL_DEFAULT_BOT.descriptor().1;
        assert!(out.contains(&format!("v{version}")), "both versions are named: {out}");

        // The two version slots are INDEPENDENT (T5 review SHOULD-2): a
        // same-build fixture reads v2/v2, which a formatter printing the
        // CURRENT version in both slots would satisfy. Doctor the recorded
        // side and both numbers must appear — cross-build honesty is what
        // §2.2 leans on this refusal for.
        let mut doctored = session.record().clone();
        doctored.run.as_mut().expect("provenance").seats[0].version = 99;
        let out = why_over(TrackingMode::Simulation, &doctored, &rules, &bots, Some(n));
        assert!(!out.contains(ACCOUNTED), "still a mismatch: {out}");
        assert!(out.contains("v99"), "the RECORDED version slot: {out}");
        assert!(out.contains(&format!("v{version}")), "the CURRENT version slot: {out}");
    }

    /// **Design §2.4's warning line, pinned on the ACCOUNT side** (T5 review
    /// SHOULD-1): a recorded version that differs while the move still
    /// reproduces WARNS and RENDERS — the honesty middle ground between
    /// silence and the mismatch refusal. The resume path's per-field drift
    /// warnings (bot.rs) are a different surface with different text; this is
    /// the line `why` itself prints.
    #[test]
    fn a_version_drift_that_still_reproduces_warns_and_renders_the_account() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        play(&mut r, 200, |r| !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty());
        let played = move_indices_of(r.session.record(), &rules, PlayerId::new(1));
        let mut record = r.session.record().clone();
        record.run.as_mut().expect("provenance").seats[1].version = 1;

        let out = why_over(TrackingMode::Simulation, &record, &rules, &r.bots, Some(played[0]));
        assert!(out.contains(ACCOUNTED), "the account still renders: {out}");
        assert!(out.contains("WARNING"), "{out}");
        assert!(
            out.contains("version 1") && out.contains("version 2"),
            "both versions named in the warning: {out}",
        );
    }

    /// A DIAGNOSTIC controller refuses in the diagnostic's own words, not the
    /// human seat's: the seat resumed HUMAN because this driver will not run
    /// that arm, and saying "a human played it" would be false about the
    /// record.
    #[test]
    fn why_refuses_a_diagnostic_controller_in_its_own_words() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        play(&mut r, 200, |r| !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty());
        let played = move_indices_of(r.session.record(), &rules, PlayerId::new(1));
        let mut record = r.session.record().clone();
        let (controller, version, params) = Contestant::MCTS_V1_DEFAULT.descriptor();
        record.run.as_mut().expect("provenance").seats[1] =
            SeatProvenance { controller, version, params, agent_seed: Some(FIXTURE_BOT_SEED), observation: OBSERVATION_VIEW.to_string() };

        let out = why_over(TrackingMode::Simulation, &record, &rules, &r.bots, Some(played[0]));
        assert!(out.contains("mcts-v1"), "{out}");
        assert!(out.contains("diagnostic"), "{out}");
        assert!(!out.contains(ACCOUNTED), "{out}");
    }

    /// A controller name this build cannot parse refuses naming it AND naming
    /// what the parser said — the same discipline `BotSeats::from_provenance`
    /// follows for the resume warning. `bga-human` is the live case: imported
    /// records carry it on every seat.
    #[test]
    fn why_refuses_an_unknown_controller_naming_it() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        play(&mut r, 200, |r| !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty());
        let played = move_indices_of(r.session.record(), &rules, PlayerId::new(1));
        let mut record = r.session.record().clone();
        record.run.as_mut().expect("provenance").seats[1].controller = "bga-human".to_string();

        let out = why_over(TrackingMode::Simulation, &record, &rules, &r.bots, Some(played[0]));
        assert!(out.contains("bga-human"), "{out}");
        assert!(out.contains("unknown controller"), "the parser's own words reach the reader: {out}");
        assert!(!out.contains(ACCOUNTED), "{out}");
    }

    // ---- the account itself ------------------------------------------------

    /// Bare `why` accounts for the most recent bot move: the header names the
    /// seat, the contestant's ECHO, the event index and the decision seed the
    /// pump keyed that decision with, and the body carries the explanation
    /// table and the move the record says was played.
    ///
    /// The BODY half is asserted at a BRANCHING bot decision rather than at the
    /// last one, which need not be: a forced point renders a one-step line and
    /// "No runner-up", so the two blocks that carry most of the screen would go
    /// unexercised (`suggest`'s `repl_mid_game` learned this first).
    #[test]
    fn why_explains_the_last_bot_move_with_the_contestant_echo_header() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        let branching = play_to_a_branching_bot_move(&mut r);
        let played = move_indices_of(r.session.record(), &rules, PlayerId::new(1));
        let last = *played.last().expect("fixture: the bot must have moved");

        let out = why(&r.session, &r.bots, None);
        assert!(out.contains(&PROVISIONAL_DEFAULT_BOT.echo()), "the contestant echo: {out}");
        assert!(out.contains(&format!("event {last}")), "the log's own index: {out}");
        assert!(
            out.contains(&format!("{:#018x}", r.bots.decision_seed(last))),
            "the decision seed the pump used: {out}",
        );
        assert!(out.contains("Seat 1"), "{out}");
        assert!(out.contains("reconstructed by replay"), "{out}");
        assert!(out.contains(ACCOUNTED), "{out}");
        assert!(out.contains("TOTAL") && out.contains("victory_points"), "the term table: {out}");
        assert!(
            flat(&out).contains(&flat(&described_move_at(r.session.record(), &rules, last))),
            "and it names the move the record carries: {out}",
        );
        // The same address by number is the same answer.
        assert_eq!(why(&r.session, &r.bots, Some(last)), out);

        // ...and at a point that offered a real choice, the search's own two
        // blocks are there with content in them.
        let branching_out = why(&r.session, &r.bots, Some(branching));
        assert!(branching_out.contains("The line it searched"), "{branching_out}");
        assert!(branching_out.contains("Runner-ups ("), "{branching_out}");
        assert!(branching_out.contains("Search cost:"), "{branching_out}");
        assert!(
            flat(&branching_out).contains(&flat(&described_move_at(r.session.record(), &rules, branching))),
            "{branching_out}",
        );
    }

    /// **The reconstruction key is the RECONCILED stream seed, not the mover's
    /// own `agent_seed`** — the one choice in this module a reader would most
    /// reasonably get backwards, and the one that cannot be observed on a
    /// single-bot fixture, where the two coincide.
    ///
    /// The pump keys EVERY bot seat off one stream (`BotSeats` carries a single
    /// `seed`), and `from_provenance` reconciles a record naming two by keeping
    /// the first bot seat's and warning. So a record whose second bot seat
    /// records a different seed — which only a hand edit produces, and which
    /// the resume path already warns about — still has to reconstruct off the
    /// stream the game was actually played on. Reading the mover's own field
    /// instead would refuse every one of that seat's moves as a mismatch.
    #[test]
    fn the_reconstruction_uses_the_reconciled_stream_seed_not_the_movers_own() {
        // 3 players, TWO bot seats off one stream: the only shape whose
        // per-seat seeds can disagree at all.
        let bots = BotSeats::new(
            vec![None, Some(PROVISIONAL_DEFAULT_BOT), Some(PROVISIONAL_DEFAULT_BOT)],
            FIXTURE_BOT_SEED,
        );
        let mut session = Session::new_sim(
            GameConfig::for_player_count(3),
            4,
            vec![StartingOrderChoice::Keep(0); 3],
            Rules::standard(),
        )
        .expect("a standard 3-player simulation");
        session.set_run_seats(bots.seat_provenance());
        let mut r = Repl::new(session);
        let rules = Rules::standard();
        let second = PlayerId::new(2);
        // Driven a real distance in rather than to the seat's first decision:
        // a search's argmax is mostly insensitive to its key (the seed picks
        // the sampled world and breaks ties), so the anti-vacuity clause below
        // needs enough decisions for the two keyings to actually part.
        play(&mut r, 400, |r| {
            branching_move_indices_of(r.session.record(), &Rules::standard(), second).len() >= 12
        });
        let played = move_indices_of(r.session.record(), &rules, second);
        let branching = branching_move_indices_of(r.session.record(), &rules, second);
        assert!(branching.len() >= 12, "fixture: the SECOND bot seat must have really decided");

        // Doctor the second bot seat's recorded stream. Resume keeps the
        // first's, loudly — which is the stream the pump played on.
        let doctored = FIXTURE_BOT_SEED ^ 0xFFFF_FFFF;
        let mut record = r.session.record().clone();
        record.run.as_mut().expect("provenance").seats[2].agent_seed = Some(doctored);
        let (resumed, warnings) = BotSeats::from_provenance(record.run.as_ref());
        assert_eq!(resumed.seed(), FIXTURE_BOT_SEED, "the first bot seat's stream wins");
        assert!(
            warnings.iter().any(|w| w.contains(&doctored.to_string())),
            "the conflict is already loud at load: {warnings:?}",
        );

        for &i in &played {
            let out = why_over(TrackingMode::Simulation, &record, &rules, &resumed, Some(i));
            assert!(
                out.contains(ACCOUNTED),
                "seat 2's event {i} must reconstruct off the RECONCILED stream:\n{out}",
            );
        }

        // Anti-vacuity: the mover's own recorded seed really would key a
        // DIFFERENT search, so the sweep above distinguishes the two choices
        // rather than passing under either.
        let wrong_stream = BotSeats::new(vec![None, None, Some(PROVISIONAL_DEFAULT_BOT)], doctored);
        let diverges = branching.iter().any(|&i| {
            let state = state_at(&record, i, &rules).expect("the decision state");
            let RecordedEvent::Move(recorded) = &record.events[i] else { return false };
            bot::decide(
                PROVISIONAL_DEFAULT_BOT,
                &state,
                &rules,
                second,
                wrong_stream.decision_seed(i),
            )
            .is_ok_and(|d| d.mv != *recorded)
        });
        assert!(
            diverges,
            "at no branching decision does the mover's own recorded seed choose differently, so \
             this fixture cannot tell the two keyings apart",
        );
    }

    /// **Read-only, asserted rather than implied.** Two calls agree byte for
    /// byte; the state, the record and the legal list are where they were; and
    /// an autosave target that a commit WOULD rewrite is left absent.
    #[test]
    fn why_is_byte_identical_on_repeat_and_applies_records_autosaves_nothing() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        play(&mut r, 200, |r| !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty());

        let path = std::env::temp_dir().join(format!("yokohama_why_autosave_{}.json", std::process::id()));
        std::fs::remove_file(&path).ok();
        r.session.save_path = Some(path.clone());
        let state_before = r.session.state().clone();
        let events_before = r.session.record().events.len();

        let first = why(&r.session, &r.bots, None);
        let second = why(&r.session, &r.bots, None);

        assert_eq!(first, second, "the same address must give the same bytes");
        assert!(first.contains(ACCOUNTED), "a real account, not a refusal: {first}");
        assert_eq!(*r.session.state(), state_before, "`why` must not move the game on");
        assert_eq!(r.session.record().events.len(), events_before, "and must record nothing");
        assert!(!path.exists(), "and must not autosave: {}", path.display());
        std::fs::remove_file(&path).ok();
    }

    /// A finished game still answers: `why` explains history, and history
    /// survives the end (design §2.3 — game over is NOT a refusal).
    #[test]
    fn why_still_answers_after_game_over() {
        let game = completed_game();
        let out = why_over(TrackingMode::Simulation, &game.record, &game.rules, &game.bots, None);
        assert!(out.contains(ACCOUNTED), "a finished game still accounts for its bot moves: {out}");
        assert!(out.contains(&PROVISIONAL_DEFAULT_BOT.echo()), "{out}");
    }

    /// **The every-move pin — OD-4(i) clause (b) made executable**: over a
    /// seeded 2P human-vs-bot game played to completion, EVERY event the bot
    /// made is retrievable via `why`, and each answer names the move the record
    /// carries. Zero refusals.
    ///
    /// Quadratic in the event count BY CONSTRUCTION and said out loud (T2
    /// review NOTE-5): each call replays the record to its own position, so the
    /// sweep is `O(events²)` checked applies plus one search per bot move.
    /// That is fine at CLI scales — a few hundred events — and it is the price
    /// of a substrate that caches nothing, which is design §3's own choice.
    #[test]
    fn why_reconstructs_every_bot_move_of_a_completed_game() {
        let game = completed_game();
        assert!(
            game.bot_moves.len() > 30,
            "anti-vacuity: the fixture must be a whole game, not a handful of moves ({})",
            game.bot_moves.len(),
        );
        for &i in &game.bot_moves {
            let out = why_over(TrackingMode::Simulation, &game.record, &game.rules, &game.bots, Some(i));
            assert!(out.contains(ACCOUNTED), "event {i} was refused:\n{out}");
            assert!(
                flat(&out).contains(&flat(&described_move_at(&game.record, &game.rules, i))),
                "event {i}'s account must name the move the record carries:\n{out}",
            );
        }
    }

    /// **Greedy renders the LEANER block.** A one-ply argmax searches no line
    /// and reports no per-candidate vector, so the two blocks a search fills
    /// are replaced by one sentence saying why — never by "no runner-up: this
    /// decision point offered exactly one move", which would be false about a
    /// branching position.
    #[test]
    fn a_greedy_mover_renders_the_leaner_block() {
        let mut r = bot_repl(4, Contestant::GREEDY_V1_DEFAULT);
        // A BRANCHING decision, so "no runner-up: this decision point offered
        // exactly one move" would be a FALSE sentence here rather than a
        // harmless one — which is what makes suppressing it a repair.
        let branching = play_to_a_branching_bot_move(&mut r);

        let out = why(&r.session, &r.bots, Some(branching));
        assert!(out.contains(ACCOUNTED), "{out}");
        assert!(out.contains(&Contestant::GREEDY_V1_DEFAULT.echo()), "{out}");
        assert!(out.contains("one-ply"), "the leaner block says WHY it is leaner: {out}");
        assert!(!out.contains("The line it searched"), "{out}");
        assert!(!out.contains("No line to show"), "{out}");
        assert!(!out.contains("Runner-ups ("), "{out}");
        assert!(!out.contains("No runner-up"), "a branching point DID offer others: {out}");
        // ...and the search-only note stays off too: greedy runs no search, so
        // there are no counters to report.
        assert!(!out.contains("Search cost:"), "{out}");
    }

    /// **A forced position is LED with the fact that nothing was weighed**
    /// (spec §2.2). The sentence is a claim about the POSITION — the engine's
    /// own legal list held exactly one move — so it is stated before the
    /// provenance note and before the table, and the runner-up block is
    /// omitted whole rather than printed as "No runner-up", which would invite
    /// the reader to think a ranking was computed and came up short.
    #[test]
    fn a_forced_move_leads_with_only_legal_option_and_omits_runner_ups() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let forced = play_to_a_forced_bot_move(&mut r);

        let out = why(&r.session, &r.bots, Some(forced));
        assert!(out.contains(ACCOUNTED), "the account still renders: {out}");
        assert!(flat(&out).contains(FORCED_LEAD), "{out}");

        let lead_at = out.find("Only legal option").expect("the lead line");
        let note_at = out.find("reconstructed by replay").expect("the provenance note");
        let table_at = out.find(ACCOUNTED).expect("the table");
        assert!(lead_at < note_at, "the lead comes before the provenance note: {out}");
        assert!(lead_at < table_at, "and before any table: {out}");

        assert!(!out.contains("Runner-ups ("), "there are none to rank: {out}");
        assert!(!out.contains("No runner-up"), "the block is omitted, not restated: {out}");
    }

    /// The control: a position that DID offer a choice carries no such lead,
    /// and its runner-up block is where it always was.
    #[test]
    fn an_unforced_move_has_no_forced_lead() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let branching = play_to_a_branching_bot_move(&mut r);

        let out = why(&r.session, &r.bots, Some(branching));
        assert!(out.contains(ACCOUNTED), "{out}");
        assert!(!out.contains("Only legal option"), "a branching point weighed candidates: {out}");
        assert!(out.contains("Runner-ups ("), "{out}");
    }

    /// **The lead COMPOSES with the one-ply sentence rather than replacing it**
    /// (spec §2.2's both-arms clause). The two say different things — the lead
    /// is about the position, `ONE_PLY_ACCOUNT` is about the contestant — and
    /// a greedy seat at its forced first move is the one address where both
    /// are true at once.
    #[test]
    fn a_forced_greedy_move_composes_the_lead_with_its_one_ply_account() {
        let mut r = bot_repl(4, Contestant::GREEDY_V1_DEFAULT);
        let forced = play_to_a_forced_bot_move(&mut r);

        let out = why(&r.session, &r.bots, Some(forced));
        assert!(out.contains(ACCOUNTED), "{out}");
        assert!(flat(&out).contains(FORCED_LEAD), "the position's own fact: {out}");
        assert!(out.contains("one-ply"), "and the contestant's: {out}");
        assert!(!out.contains("Runner-ups ("), "{out}");
        assert!(!out.contains("No runner-up"), "{out}");
    }

    /// **All fifteen counters, with the values the search actually reported**
    /// (spec §2.6). The old note printed two of them, which is the gap
    /// `driven_runner::counter_rows` was built to close: its body destructures
    /// `SearchStats` with no `..`, so a sixteenth counter is a compile error
    /// THERE, the array grows, and [`counter_lines`]' ungrouped arm surfaces it
    /// HERE. The chain has no silent link, and this is the end of it.
    ///
    /// The values are re-derived by re-running the SAME decision rather than
    /// scraped off the screen: a block that printed fifteen names against
    /// fifteen zeroes would pass a names-only pin.
    #[test]
    fn the_counter_block_names_all_fifteen_rows() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        let branching = play_to_a_branching_bot_move(&mut r);
        let out = flat(&why(&r.session, &r.bots, Some(branching)));

        let state = state_at(r.session.record(), branching, &rules).expect("the decision state");
        let stats = bot::decide(
            PROVISIONAL_DEFAULT_BOT,
            &state,
            &rules,
            PlayerId::new(1),
            r.bots.decision_seed(branching),
        )
        .expect("the search decides")
        .explain
        .expect("and explains")
        .stats
        .expect("a search reports counters");

        let rows = driven_runner::counter_rows(&stats);
        for (name, value) in rows {
            assert!(
                out.contains(&format!("{name} {value}")),
                "the block must carry {name} at its reported value {value}: {out}",
            );
        }
        // Anti-vacuity: the counters must not be all-zero, or the pin above
        // would hold for any name paired with any zero.
        assert!(
            rows.iter().filter(|(_, v)| *v > 0).count() >= 4,
            "the fixture's search must have done real work: {rows:?}",
        );
    }

    /// The block is TWO groupings, and every line of it fits the frame.
    ///
    /// The split is by what a counter measures — work done on the search line,
    /// limits that bit on the guard line — so D7's mandatory set
    /// (binds/starved/ply/free-cap) reads as one answer to one question
    /// instead of being spread across a wall of names.
    #[test]
    fn the_two_line_grouping_holds() {
        crate::style::force(false);
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let branching = play_to_a_branching_bot_move(&mut r);
        let out = why(&r.session, &r.bots, Some(branching));

        assert!(out.contains(SEARCH_COST_LABEL), "{out}");
        assert!(out.contains(GUARD_LABEL), "{out}");
        assert!(
            !out.contains(UNGROUPED_LABEL),
            "every shipped counter is grouped, so the defect line must stay off: {out}",
        );
        // Each name lands on exactly ONE of the two lines. Counted over WHOLE
        // whitespace tokens, never as a substring: `nodes` is inside both
        // `route_dfs_nodes` and `forced_nodes`, so a `matches()` count would
        // read 3 for a name that is printed once.
        let tokens: Vec<&str> = out.split_whitespace().collect();
        for (name, _) in driven_runner::counter_rows(&SearchStats::default()) {
            assert_eq!(
                tokens.iter().filter(|t| **t == name).count(),
                1,
                "{name} must appear once, on one grouping line: {out}",
            );
        }

        let frame = crate::screen::frame_width(2);
        for line in out.lines() {
            let width = crate::screen::visible_width(line);
            assert!(width <= frame, "a line is {width} columns against {frame}: {line:?}");
        }
    }

    /// **A counter this build does not know is a DEFECT, said out loud.** The
    /// grouping is a match with no catch-all, and what it cannot place goes on
    /// its own line naming it — because the alternative, dropping it, is
    /// exactly the "why printed 2 of 15" failure one layer up.
    ///
    /// Driven through [`counter_lines`] with a fabricated row: a real
    /// sixteenth counter cannot be manufactured from this crate (that is the
    /// point of core's no-`..` destructure), so the arm is exercised at the
    /// seam it defends rather than left to a future edit to discover.
    #[test]
    fn an_ungrouped_counter_is_surfaced_rather_than_dropped() {
        let lines = counter_lines(&[("scored", 7), ("brand_new_counter", 3), ("budget_binds", 1)]);
        let joined = lines.join(" ");
        assert!(joined.contains("scored 7"), "{joined}");
        assert!(joined.contains("budget_binds 1"), "{joined}");
        assert!(joined.contains(UNGROUPED_LABEL), "{joined}");
        assert!(joined.contains("brand_new_counter 3"), "the unknown one is NAMED: {joined}");

        // ...and a fully-known list prints no such line.
        let known = counter_lines(&driven_runner::counter_rows(&SearchStats::default())).join(" ");
        assert!(!known.contains(UNGROUPED_LABEL), "{known}");
    }

    /// The explanation table's rows, in render order: every line from the
    /// headings row to the `TOTAL` row inclusive.
    ///
    /// Sliced by the two strings the renderer OWNS (`TABLE_HEADINGS` at their
    /// real column spacing, `TOTAL_LABEL`), so a column-width edit moves this
    /// helper's answer rather than silently changing what it measures.
    fn table_block(text: &str) -> Vec<&str> {
        let headings = crate::suggest::table_row(crate::suggest::TABLE_HEADINGS);
        let start = text
            .lines()
            .position(|l| l == headings)
            .unwrap_or_else(|| panic!("no table headings row in:\n{text}"));
        let rows: Vec<&str> = text.lines().skip(start).collect();
        let end = rows
            .iter()
            .position(|l| l.contains(crate::suggest::TOTAL_LABEL))
            .expect("no TOTAL row");
        rows[..=end].to_vec()
    }

    /// The `product` cell of a table row — its last whitespace token, which is
    /// the column the marker's semantics are about.
    fn product_cell(row: &str) -> &str {
        row.split_whitespace().next_back().unwrap_or("")
    }

    /// **The nonzero-product aid, all three parts** (spec §2.3): a marker in a
    /// lead column on exactly the rows whose product is nonzero, the zero rows
    /// dimmed on a terminal, and a computed summary under the table saying how
    /// many rows carried the move.
    ///
    /// **Canonical row ORDER does not move** — cross-referencing a screen
    /// against `eval::explain` or against an older capture has to keep working,
    /// which is why the aid is a column and a tone rather than a re-sort.
    ///
    /// The marker's column is the row's FIRST character, taken out of the two
    /// spaces every row already began with, so the table's alignment and its
    /// total width are untouched: that is asserted here rather than assumed.
    #[test]
    fn nonzero_product_rows_carry_the_mark_and_zeros_dim() {
        crate::style::force(false);
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let branching = play_to_a_branching_bot_move(&mut r);
        let out = why(&r.session, &r.bots, Some(branching));

        let rows = table_block(&out);
        let widths: std::collections::BTreeSet<usize> =
            rows.iter().map(|l| crate::screen::visible_width(l)).collect();
        assert_eq!(widths.len(), 1, "the table's columns must stay square: {rows:#?}");

        let mark = crate::style::glyph::NONZERO_MARK;
        let mut marked = 0usize;
        let mut story = 0usize;
        for row in &rows[1..] {
            let is_total = row.contains(crate::suggest::TOTAL_LABEL);
            let carries = row.starts_with(mark);
            if is_total {
                assert!(!carries, "the TOTAL row is a sum, not a story row: {row:?}");
                continue;
            }
            story += 1;
            let product: i64 = product_cell(row).parse().expect("a numeric product cell");
            assert_eq!(
                carries,
                product != 0,
                "the marker must track the product cell exactly: {row:?}",
            );
            marked += usize::from(carries);
        }
        assert!(marked > 0 && marked < story, "anti-vacuity: the fixture needs both kinds");

        assert!(
            out.contains(&format!(
                "{marked} of {story} {}",
                crate::suggest::NONZERO_SUMMARY_TAIL
            )),
            "the summary line must count what the table marked: {out}",
        );

        // ...and on a terminal the zero rows are the DIM ones.
        crate::style::force(true);
        let painted = why(&r.session, &r.bots, Some(branching));
        for row in &table_block(&painted)[1..] {
            if row.contains(crate::suggest::TOTAL_LABEL) {
                continue;
            }
            let dim = row.contains('\u{1b}');
            assert_eq!(
                dim,
                !row.contains(mark),
                "zero-product rows dim, marked rows do not: {row:?}",
            );
        }
        crate::style::force(false);
    }

    /// The aid is present on TTY and piped ALIKE, so the two stay
    /// byte-identical modulo color: the marker is a CHARACTER, never a paint.
    #[test]
    fn piped_output_is_escape_free() {
        crate::style::force(false);
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let branching = play_to_a_branching_bot_move(&mut r);
        let out = why(&r.session, &r.bots, Some(branching));
        assert!(!out.contains('\u{1b}'), "piped output carries no ANSI:\n{out}");
        assert!(
            out.contains(crate::style::glyph::NONZERO_MARK),
            "and the marker still reaches it:\n{out}",
        );
    }

    /// **Every line of every `why` screen fits the frame, and the prose fits
    /// the editorial measure under it** — the property `screen::wrap` makes
    /// load-bearing (it sizes the composed frame to its widest line, so one
    /// over-wide body shoves the board sideways for as long as it is up).
    ///
    /// Written over the REFUSALS above all, because that is where the risk is:
    /// each one interpolates something whose length it does not control (a
    /// parser's error listing every contestant name, a `Move`'s `Debug`, a
    /// contestant's echo), and a hand-wrapped literal bounds only the half that
    /// was never in doubt. Measured with `screen::visible_width`, the same
    /// escape-stripping measure the frame guard uses; and the screens must be
    /// plain, since nothing here paints outside `Body`.
    #[test]
    fn every_why_screen_fits_the_frame_and_says_its_dashes_in_em() {
        crate::style::force(false);
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        let branching = play_to_a_branching_bot_move(&mut r);
        let mut screens: Vec<(String, String)> = vec![
            ("the account".to_string(), why(&r.session, &r.bots, Some(branching))),
            ("no bot move".to_string(), why(&human_repl(4).session, &BotSeats::none(), None)),
            ("live".to_string(), why(&live_repl(21).session, &BotSeats::none(), None)),
            ("out of range".to_string(), why(&r.session, &r.bots, Some(9_999))),
            ("human seat".to_string(), why(&r.session, &r.bots, Some(0))),
        ];
        // The unknown-controller arm carries the LONGEST interpolation in the
        // module: the core parser's refusal lists every contestant name it
        // knows, which alone runs past 150 columns.
        let mut record = r.session.record().clone();
        record.run.as_mut().expect("provenance").seats[1].controller = "bga-human".to_string();
        screens.push((
            "unknown controller".to_string(),
            why_over(TrackingMode::Simulation, &record, &rules, &r.bots, Some(branching)),
        ));
        let mut reveal = r.session.record().clone();
        reveal.events.push(RecordedEvent::RevealTechs(vec![*r
            .session
            .state()
            .tech_deck
            .last()
            .expect("a tech card")]));
        let n = reveal.events.len() - 1;
        screens.push((
            "reveal".to_string(),
            why_over(TrackingMode::Simulation, &reveal, &rules, &r.bots, Some(n)),
        ));

        let frame = crate::screen::frame_width(2);
        for (what, text) in &screens {
            assert!(!text.is_empty(), "{what}: the fixture produced no text");
            assert!(!text.contains('\u{1b}'), "{what}: piped output must carry no ANSI:\n{text}");
            crate::style::test_support::assert_no_ascii_prose_dash(what, text);
            for line in text.lines() {
                let width = crate::screen::visible_width(line);
                assert!(width <= frame, "{what}: a line is {width} columns against {frame}: {line:?}");
            }
        }
        // Anti-vacuity, in the direction that matters: at least one refusal is
        // long enough that a bound could bite it, so the sweep above is not
        // measuring a handful of short sentences.
        let longest = screens
            .iter()
            .flat_map(|(_, t)| t.lines())
            .map(crate::screen::visible_width)
            .max()
            .expect("lines");
        assert!(longest > 60, "the fixtures must produce real paragraphs, widest was {longest}");
    }

    // ---- the command ------------------------------------------------------

    /// `why` and `why <n>` reach the same body through the REPL, `why` on a
    /// non-number says so, and the command is in `help`.
    #[test]
    fn the_repl_dispatches_both_forms_of_why() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        play(&mut r, 200, |r| !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty());
        let last = *move_indices_of(r.session.record(), &rules, PlayerId::new(1)).last().expect("a bot move");

        let bare = r.step("why");
        assert!(bare.text.contains(ACCOUNTED), "{}", bare.text);
        assert!(!bare.quit);
        let numbered = r.step(&format!("why {last}"));
        assert_eq!(numbered.text, bare.text, "the two forms address the same event here");

        let bad = r.step("why banana");
        assert!(bad.text.contains("banana"), "{}", bad.text);
        assert!(!bad.text.contains(ACCOUNTED), "{}", bad.text);

        assert!(
            crate::repl::HELP_TEXT.lines().any(|l| l.split_whitespace().next() == Some("why")),
            "`why` must be listed in the in-app help",
        );
    }

    /// **The split is a split, not a fork.** `account_parts` raises every
    /// refusal and does every honesty check — resolve the event, replay to the
    /// decision state, read the seat's controller, re-derive the pump's key,
    /// re-run `decide`, and refuse unless the result equals the recorded move.
    /// `account` is that output with words on it. A consumer drawing its own
    /// table has to be able to rely on the first half without reimplementing
    /// it, so the parts are pinned to carry what the screen is made of, and the
    /// screen is pinned to be a pure function of the parts.
    #[test]
    fn the_account_parts_are_what_the_screen_is_made_of() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        let branching = play_to_a_branching_bot_move(&mut r);

        let parts =
            account_parts(r.session.mode, r.session.record(), &rules, &r.bots, Some(branching))
                .expect("the fixture's bot move reconstructs");

        // The provenance half — what a consumer must say to make the arithmetic
        // a claim rather than a table of numbers.
        assert_eq!(parts.reconstruction.index, branching);
        assert_eq!(parts.reconstruction.seat, PlayerId::new(1));
        assert_eq!(parts.reconstruction.decision_seed, r.bots.decision_seed(branching));
        assert_eq!(parts.reconstruction.contestant, PROVISIONAL_DEFAULT_BOT);

        // The arithmetic half arrives structured, which is the whole point:
        // nothing here has been through a formatter.
        assert!(!parts.explain.explanation.terms.is_empty(), "the term table is data");

        // The move is the record's own. That equality IS the honesty gate and
        // was enforced inside `account_parts`, so asserting it here proves the
        // gate ran rather than adding a second one.
        let RecordedEvent::Move(recorded) = &r.session.record().events[branching] else {
            panic!("fixture: event {branching} is a move");
        };
        assert_eq!(&parts.mv, recorded);

        // ...and the screen is that and nothing else.
        let screen = why(&r.session, &r.bots, Some(branching));
        assert_eq!(
            render(parts.reconstruction, parts.mv, parts.explain, &parts.state, &rules),
            screen,
            "the rendered parts ARE the screen",
        );
    }

    /// **`account_parts` is `locate` then `account_from`, and the halves meet exactly** (security S2c, decision
    /// D-6 — the layer reads the requester's visibility between them, so a withheld answer never reaches
    /// `decide`): on the module's vs-bot fixture the located index, seat, recorded move and versions equal the
    /// account's; a record with no bot move refuses from `locate` with `NoBotMoveYet`; an out-of-range index with
    /// `OutOfRange` — every pre-search refusal is `locate`'s. `locate` is pure replay: this cell holds no bot seats
    /// at all for its first call.
    #[test]
    fn locate_is_the_pre_search_half_of_account_parts_and_the_halves_agree() {
        // The fixture is `the_account_parts_are_what_the_screen_is_made_of`'s own three lines:
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        let branching = play_to_a_branching_bot_move(&mut r);
        let (session, bots) = (&r.session, &r.bots);
        let none_yet = human_repl(4);
        assert!(matches!(locate(TrackingMode::Simulation, none_yet.session.record(), &rules, None), Err(Refusal::NoBotMoveYet)));
        let located = locate(TrackingMode::Simulation, session.record(), &rules, None).expect("a bot move to locate");
        let account = account_parts(TrackingMode::Simulation, session.record(), &rules, bots, None).expect("an account");
        assert_eq!((located.index, located.seat), (account.reconstruction.index, account.reconstruction.seat));
        assert_eq!(located.recorded, account.mv);
        assert_eq!(located.recorded_version, account.reconstruction.recorded_version);
        assert_eq!(located.current_version, account.reconstruction.current_version);
        assert_eq!(located.contestant.echo(), account.reconstruction.contestant.echo());
        let again = account_from(locate(TrackingMode::Simulation, session.record(), &rules, None).unwrap(), bots, &rules).unwrap();
        assert_eq!(again.reconstruction.decision_seed, account.reconstruction.decision_seed);
        assert_eq!(again.mv, account.mv);
        assert!(matches!(locate(TrackingMode::Simulation, session.record(), &rules, Some(10_000)), Err(Refusal::OutOfRange { .. })));
        assert!(matches!(locate(TrackingMode::Live, session.record(), &rules, None), Err(Refusal::LiveMode)));
        let at = locate(TrackingMode::Simulation, session.record(), &rules, Some(branching)).expect("the branching bot move");
        let RecordedEvent::Move(recorded) = &session.record().events[branching] else { panic!("fixture: a move") };
        let version = PROVISIONAL_DEFAULT_BOT.descriptor().1;
        assert_eq!((at.index, at.seat, at.contestant), (branching, PlayerId::new(1), PROVISIONAL_DEFAULT_BOT));
        assert_eq!((at.current_version, at.recorded_version), (version, Some(version)));
        assert_eq!(&at.recorded, recorded);
        // **M40's full catcher** (M40a, Task 6's probe): ONE human move after the bot's, so the record's last move
        // is seat 0's while a bot move exists — bare `why` addresses the LAST EXPLAINABLE BOT move, not the last
        // move. The variant that keeps `last_explainable_bot_move`'s refusal and then takes the last move passed
        // every other cell in this crate.
        assert!(!r.bots.is_bot(r.session.state().current_player), "premise: the fixture stops on the human's turn");
        let human = r.session.legal()[0].clone();
        r.session.apply(human, vec![]).expect("a move from the engine's own list applies");
        let record = r.session.record();
        let last = record.events.len() - 1;
        assert_eq!(move_indices_of(record, &rules, PlayerId::new(0)).last(), Some(&last), "premise: the record's last event is seat 0's move");
        let last_bot = *move_indices_of(record, &rules, PlayerId::new(1)).last().expect("fixture: the bot has moved");
        let bare = locate(TrackingMode::Simulation, record, &rules, None).expect("a bot move to locate");
        assert_eq!(bare.index, last_bot, "bare `why` addresses the last explainable BOT move, not the last move");
    }

    /// A refusal is raised in ONE place. When the parts refuse, the screen
    /// refuses for the same reason and in the same words — the screen being
    /// exactly that refusal folded to the reader's measure. Two surfaces that
    /// each decided what was explainable would eventually disagree, and the
    /// disagreement would be a bot move explained to a reader it should not
    /// have been.
    #[test]
    fn both_surfaces_refuse_from_the_same_gate() {
        let r = human_repl(4);
        let rules = Rules::standard();
        let record = r.session.record();

        let Err(refusal) = account_parts(r.session.mode, record, &rules, &r.bots, None) else {
            panic!("a table with no bot seat has nothing to account for");
        };
        assert!(matches!(refusal, Refusal::NoBotMoveYet), "{refusal:?}");
        assert_eq!(
            why(&r.session, &r.bots, None),
            suggest::wrap_to(&refusal.to_string(), refusal_width(record)),
            "the screen is the refusal, folded — not a second sentence about it",
        );
    }
}
