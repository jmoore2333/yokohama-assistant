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
enum Refusal {
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
struct Mismatch {
    index: usize,
    seat: PlayerId,
    controller: String,
    /// `Debug`, not a description: the reconstructed move need not be legal in
    /// the TRUE position (it was chosen inside a sampled world), and naming a
    /// move against a state that does not admit it is how a formatter gets to
    /// index something that is not there — `SuggestError::Inconsistent`'s own
    /// reasoning, at the one other site with the same exposure.
    reconstructed: String,
    /// The recorded move in words, which is always safe: it applied here.
    recorded: String,
    recorded_version: Option<u32>,
    current_version: u32,
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
    let state = state_at(record, index, rules)
        .map_err(|e| Refusal::Replay { index, message: e.message })?;
    let seat = state.current_player;
    let contestant = match read_seat(record, seat) {
        SeatReading::Explainable(contestant) => contestant,
        SeatReading::Human => return Err(Refusal::HumanSeat { index, seat }),
        SeatReading::Rung(rung) => return Err(Refusal::Rung { index, seat, rung }),
        SeatReading::Diagnostic(why) => return Err(Refusal::Diagnostic { index, seat, why }),
        SeatReading::Unknown { controller, why } => {
            return Err(Refusal::UnknownController { index, seat, controller, why })
        }
    };

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

    let (controller, current_version, _) = contestant.descriptor();
    let recorded_version = seat_provenance(record, seat).map(|p| p.version);
    if decision.mv != *recorded {
        return Err(Refusal::Mismatch(Box::new(Mismatch {
            index,
            seat,
            controller: contestant.echo(),
            reconstructed: format!("{:?}", decision.mv),
            recorded: describe::describe_move(recorded, &state, rules),
            recorded_version,
            current_version,
        })));
    }
    let explain = decision
        .explain
        .ok_or_else(|| Refusal::Unaccounted { index, contestant: controller.clone() })?;

    Ok(render(
        Reconstruction { contestant, index, seat, decision_seed, recorded_version, current_version },
        decision.mv,
        explain,
        &state,
        rules,
    ))
}

/// What the header says about the reconstruction, as one argument rather than
/// six positional ones.
struct Reconstruction {
    contestant: Contestant,
    index: usize,
    seat: PlayerId,
    decision_seed: u64,
    recorded_version: Option<u32>,
    current_version: u32,
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
    if let Some(stats) = explain.stats {
        notes.push((
            Tone::Dim,
            format!(
                "Search cost: {} successor position(s) scored across {} root candidate(s).",
                stats.scored, stats.root_candidates,
            ),
        ));
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
            Contestant::LookaheadV1 { .. } => None,
            // Unreachable: `read_seat` admits only the two arms above.
            Contestant::Policy(_) | Contestant::MctsV1 { .. } | Contestant::AtomicDiag { .. } => {
                None
            }
        },
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
}
