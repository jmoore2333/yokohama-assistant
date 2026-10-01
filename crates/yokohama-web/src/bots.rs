//! **Which bot played a game, in the words a person reads, and whether this
//! build plays it on** (M6.5c2, R-6.5c2.10; cycle F, R-F4).
//!
//! A bot seat's identity is the RECORD's: every `SeatProvenance` carries its
//! controller's name, that controller's version and every knob, and
//! `Contestant::from_parts` is the inverse that reads the name and the knobs
//! back. This module adds the two things a record cannot say for itself: a
//! SHORT LABEL for each identity (`v4-step9-draft1` is "V4"), and whether THIS
//! build runs it.
//!
//! **Whether this build runs it is the CLI's resume's answer, never this
//! module's** (cycle F, R-F4 amending R-6.5c2.10 forward; spec §2.3; R-FP12).
//! [`bots::runs_here`] calls `BotSeats::from_provenance` — the public resume every
//! `Repl` seats its bots through — ONCE per record, and a recorded bot seat
//! runs here exactly when that resume seats a contestant there and says
//! nothing about the seat: no conversion to a human (a controller the CLI's
//! `eligible` refuses, or one `Contestant::from_parts` cannot read), no drift
//! (a controller version or ANY knob, the weights digest included, that
//! differs from what this build derives), no second stream seed. So the web
//! and the `Repl` cannot disagree about a seat, and the web never widens the
//! CLI's rule: the day the CLI admits an identity, a game played against it
//! opens here with no web change, and until then it is refused in the CLI's
//! own words. A seat that does not run carries that reason, cut to its clause
//! and never reworded ([`bots::Runs::No`]).
//!
//! **The labels are the layer's own words**, like `suggest`'s four authored
//! refusals, and they are total by CONSTRUCTION: [`bots::label`] reads a
//! `Contestant` through an exhaustive `match` with no `_` arm at either level —
//! the CLI's `bot::eligible` discipline — so the day core gains a V5 identity
//! this crate stops compiling until V5 has a label. That one line is the only
//! web edit a new bot needs; everything else here follows
//! `bot::PROVISIONAL_DEFAULT_BOT`.
//!
//! **Nothing here stores a bot.** `SeatDoc::Bot` still does not say which one
//! (`table.rs`): a dealt table's is the record's, cached in the turn mirror
//! ([`crate::table::TurnMirror::bot`]), and an undealt table's is the one
//! `finalize` will deal, [`bots::production`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use yokohama_cli::bga::CONTROLLER_BGA_HUMAN;
use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
use yokohama_core::{
    Contestant, GameRecord, NamedIdentity, PlayerId, RolloutPolicy, RunProvenance, SeatProvenance,
    CONTROLLER_HUMAN, OBSERVATION_VIEW, RUN_PROVENANCE_SCHEMA,
};

use crate::table::{SeatDoc, TableDoc, TableStatus};

/// One bot seat's identity as its record names it: a `SeatProvenance`'s
/// `(controller, version, params)`, without the stream seed (a fact about one
/// game, not about the bot) and the observation channel.
///
/// **The triple, not the name** (review m12): one name can ship at two
/// controller versions or two knob sets, and the CLI's resume, which
/// [`runs_here`] asks, holds a record to the version and to every knob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BotDescriptor {
    pub controller: String,
    pub version: u32,
    #[serde(default)]
    pub params: BTreeMap<String, i64>,
}

/// The table's bot on the wire (`TableView.bot`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableBot {
    /// [`label`]'s answer.
    pub label: String,
    /// Whether this build runs that bot: [`runs_here`]'s predicate, over the
    /// descriptor alone (`descriptor_runs`).
    pub runs_here: bool,
}

/// One bot seat of a game on the wire (`ClientState.bots`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SeatBot {
    pub seat: u8,
    pub label: String,
}

/// The bot this build deals — `bot::PROVISIONAL_DEFAULT_BOT` — as a record
/// names it.
///
/// Read through `Contestant::descriptor`, the function `BotSeats::seat_provenance`
/// writes every bot seat through, so a game dealt here compares equal to this
/// by construction, and a re-point of the const re-points this.
pub fn production() -> BotDescriptor {
    let (controller, version, params) = PROVISIONAL_DEFAULT_BOT.descriptor();
    BotDescriptor { controller, version, params }
}

/// The label a person reads for `d`.
///
/// Parsed through `Contestant::from_parts` over the record's two stored halves,
/// as the CLI's resume path reads them, and named by `label_of`. A
/// descriptor this build cannot parse — a name it does not know, a knob out of
/// range — answers its own controller name: an honest label for a bot nothing
/// here can describe better.
pub fn label(d: &BotDescriptor) -> String {
    match Contestant::from_parts(&d.controller, &d.params) {
        Ok(contestant) => label_of(contestant).to_string(),
        Err(_) => d.controller.clone(),
    }
}

/// **Exhaustive, with no `_` arm at either level.** See the module doc.
fn label_of(contestant: Contestant) -> &'static str {
    match contestant {
        Contestant::NamedTable(arm) => match arm {
            // The shipped production identity since 2026-08-30.
            NamedIdentity::V4Step9Draft1 => "V4",
            // `WeightTable::V2` — the table the V2-era bot played — named.
            NamedIdentity::V2Padded => "V2",
            // A research refit of V2's table, never shipped.
            NamedIdentity::V2RefitPadded => "V2 refit",
        },
        // The V2-era production seat (the CLI's default until the V4 ship) and
        // its one-ply sibling, both grandfathered on `WeightTable::V2`.
        Contestant::LookaheadV1 { .. } => "V2 lookahead",
        Contestant::GreedyV1 { .. } => "V2 greedy",
        // The two labelled diagnostics: reachable, never dealt to anyone.
        Contestant::AtomicDiag { .. } => "atomic diagnostic",
        Contestant::MctsV1 { .. } => "MCTS diagnostic",
        // The preserved rollout rungs, which sample a move rather than value it.
        Contestant::Policy(rung) => match rung {
            RolloutPolicy::Uniform => "uniform rung",
            RolloutPolicy::RetreatAverse => "retreat-averse rung",
            RolloutPolicy::FamilyWeighted { .. } => "family-weighted rung",
        },
    }
}

/// Every seat `record` names a BOT for, in seat order, with its descriptor.
/// Empty for an all-human game and for a record with no run provenance. A seat
/// index past `u8` is skipped: no record this layer writes has one, and
/// `recorded` is read before `Session::from_record` validates.
///
/// **A bot is any controller but a person's**, and a person is either
/// controller `is_person` names — a seat typed at a keyboard
/// (`CONTROLLER_HUMAN`), or one played by a person on BGA and imported
/// (`yokohama_cli::bga::CONTROLLER_BGA_HUMAN`).
pub fn recorded(record: &GameRecord) -> Vec<(u8, BotDescriptor)> {
    let Some(run) = &record.run else { return Vec::new() };
    run.seats
        .iter()
        .enumerate()
        .filter(|(_, s)| !is_person(&s.controller))
        .filter_map(|(i, s)| {
            let seat = u8::try_from(i).ok()?;
            Some((
                seat,
                BotDescriptor { controller: s.controller.clone(), version: s.version, params: s.params.clone() },
            ))
        })
        .collect()
}

/// Whether a seat's recorded `controller` is a PERSON's: `CONTROLLER_HUMAN`,
/// a seat typed at this keyboard, or `yokohama_cli::bga::CONTROLLER_BGA_HUMAN`,
/// a seat a person played on BGA (M6.5c2 cycle D, straw poll row 144).
///
/// The importer writes `bga-human` rather than `human` so a reader of the file
/// can tell an imported game from a typed one, and the CLI resumes such a seat
/// as a HUMAN one (`bot.rs`'s unparsed arm, "which is how `bga-human` seats
/// from imported records keep resuming as people"). Counted as a bot here, it
/// was a FOREIGN bot — a game "played against bga-human" this server "doesn't
/// run", and a bot badge on a person.
fn is_person(controller: &str) -> bool {
    controller == CONTROLLER_HUMAN || controller == CONTROLLER_BGA_HUMAN
}

/// The seats `record` names [`CONTROLLER_BGA_HUMAN`] for, in seat order: the
/// people the CLI's resume path warns it "resumes as a HUMAN seat", which
/// `client::without_foreign_conversion` masks. Skips a seat past `u8` as
/// [`recorded`] does.
pub(crate) fn bga_human_seats(record: &GameRecord) -> Vec<u8> {
    let Some(run) = &record.run else { return Vec::new() };
    run.seats
        .iter()
        .enumerate()
        .filter(|(_, s)| s.controller == CONTROLLER_BGA_HUMAN)
        .filter_map(|(i, _)| u8::try_from(i).ok())
        .collect()
}

/// Where `doc` and `record` disagree about WHICH SEATS ARE BOTS, the lowest
/// such seat in a sentence; `None` when they agree (M6.5c2 cycle D, the review
/// of Task 4).
///
/// A table's `Bot` seats and the record's [`recorded`] seats are one set in
/// every pair this layer writes: `finalize` stamps the run seats from the
/// table's own seats (`BotSeats::seat_provenance`, a person's as
/// `CONTROLLER_HUMAN`), and has since the first table document. A pair that
/// disagrees was made by hand, and it cannot be played: a table `Bot` seat
/// whose record names a person is a seat the `Repl` resumes as a human while
/// no person holds it at the table, so nothing ever moves it — the shape a
/// `bga-human` record beside the committed pair's table has. Three sentences:
///
/// - a table `Bot` the record names a person for: "seat N is a person in the
///   record (controller) but a bot at the table";
/// - a table `Bot` the record names no controller for — no run provenance, or
///   a run that stops short: "seat N is a bot at the table but the record
///   names no controller for it";
/// - a recorded bot the table seats otherwise: "seat N is a bot in the record
///   (label) but a person at the table" (or "an open seat", or "not a seat").
pub(crate) fn seat_disagreement(doc: &TableDoc, record: &GameRecord) -> Option<String> {
    let bots: BTreeMap<u8, BotDescriptor> = recorded(record).into_iter().collect();
    let run_seats = record.run.as_ref().map_or(&[][..], |run| run.seats.as_slice());
    let span = doc.seats.len().max(run_seats.len()).min(usize::from(u8::MAX) + 1);
    (0..span).find_map(|i| {
        let seat = u8::try_from(i).ok()?;
        let at_table = doc.seats.get(i);
        match (matches!(at_table, Some(SeatDoc::Bot)), bots.get(&seat)) {
            (true, Some(_)) | (false, None) => None,
            (true, None) => Some(match run_seats.get(i) {
                Some(s) => format!("seat {i} is a person in the record ({}) but a bot at the table", s.controller),
                None => format!("seat {i} is a bot at the table but the record names no controller for it"),
            }),
            (false, Some(d)) => Some(format!(
                "seat {i} is a bot in the record ({}) but {} at the table",
                label(d),
                match at_table {
                    Some(SeatDoc::Human { .. }) => "a person",
                    Some(SeatDoc::Open) => "an open seat",
                    // `Bot` is the first arm's; listed so the match has no `_`.
                    Some(SeatDoc::Bot) | None => "not a seat",
                },
            )),
        }
    })
}

/// `TableView.bot` for `doc` (spec §2.4, "Where the lobby gets it").
///
/// - no bot seat: `None`;
/// - undealt (`open` or `dealing`): the production bot, which is what
///   `finalize` will deal, so it runs here;
/// - dealt: the mirror's descriptor — the record's, cached — or `None` for a
///   table whose mirror was last written before the field existed (its row
///   reads "the bot" with no version; review m7).
///
/// `runs_here` is the game route's own predicate (cycle F, spec §2.3), asked
/// of the mirror's descriptor ([`descriptor_runs`]), so a row reads "runs
/// here" exactly when its game opens: a table dealt a bot this build's CLI
/// resumes exactly — the production bot or a grandfathered one — reads its
/// label and opens.
pub(crate) fn table_bot(doc: &TableDoc) -> Option<TableBot> {
    if !doc.seats.iter().any(|s| matches!(s, SeatDoc::Bot)) {
        return None;
    }
    let bot = match doc.status {
        TableStatus::Open | TableStatus::Dealing => production(),
        TableStatus::Playing => doc.turn.as_ref()?.bot.clone()?,
    };
    let runs_here = descriptor_runs(&bot) == Runs::Yes;
    Some(TableBot { label: label(&bot), runs_here })
}

/// `ClientState.bots`: every bot seat the record names, labelled.
pub(crate) fn seat_bots(record: &GameRecord) -> Vec<SeatBot> {
    recorded(record).into_iter().map(|(seat, d)| SeatBot { seat, label: label(&d) }).collect()
}

/// Whether THIS build plays a recorded bot seat on (cycle F, R-F4; R-FP12):
/// the CLI's resume, asked ([`runs_here`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Runs {
    /// The CLI's resume seats the exact bot the record names, and says nothing
    /// about the seat.
    Yes,
    /// It does not, and this is why: the CLI's own warning about the seat,
    /// cut to its clause (`reason_of`) — never reworded.
    No(String),
}

/// Every bot seat `record` names ([`recorded`]), in seat order, with whether
/// THIS build plays it on (cycle F, R-F4; R-FP12) — decided by ONE call of
/// the CLI's public resume, `BotSeats::from_provenance(record.run)`:
///
/// - [`Runs::Yes`] when the resume seats a contestant there
///   (`contestant_at(seat)` is `Some`) and NO warning names the seat — no line
///   beginning `warning: seat {i} `, the stream-seed line included, which
///   makes a record carrying two bot seeds foreign (no web table deals two);
/// - [`Runs::No`] otherwise, carrying every warning that names the seat, each
///   cut to its clause (`reason_of`), joined by `; ` in the CLI's order.
///
/// The CLI pushes a warning on both of its conversion arms, so a seat it
/// seats no bot at always has one; were one ever missing, the seat still does
/// not run, and its reason says only that.
pub fn runs_here(record: &GameRecord) -> Vec<(u8, BotDescriptor, Runs)> {
    resumed(record.run.as_ref(), recorded(record))
}

/// [`runs_here`]'s answer for ONE descriptor: a record naming `d` at a seat,
/// resumed. A table carries its bot in the turn mirror rather than its
/// record, so this is what the lobby row's `TableBot.runs_here` reads — and
/// the row and the game route answer from one predicate.
pub(crate) fn descriptor_runs(d: &BotDescriptor) -> Runs {
    let run = RunProvenance {
        schema: RUN_PROVENANCE_SCHEMA,
        master_seed: None,
        run_id: None,
        seats: vec![SeatProvenance {
            controller: d.controller.clone(),
            version: d.version,
            params: d.params.clone(),
            agent_seed: None,
            observation: OBSERVATION_VIEW.to_string(),
        }],
    };
    let (seats, warnings) = BotSeats::from_provenance(Some(&run));
    verdict(0, &d.controller, &seats, &warnings)
}

/// The CLI's resume, asked ONCE of `run`, and each of `bots` — seats of that
/// run — answered from what it said ([`verdict`]).
fn resumed(run: Option<&RunProvenance>, bots: Vec<(u8, BotDescriptor)>) -> Vec<(u8, BotDescriptor, Runs)> {
    let (seats, warnings) = BotSeats::from_provenance(run);
    bots.into_iter()
        .map(|(seat, d)| {
            let runs = verdict(seat, &d.controller, &seats, &warnings);
            (seat, d, runs)
        })
        .collect()
}

/// [`runs_here`]'s rule for one recorded bot `seat` named `controller`, over
/// what ONE resume answered: the `seats` it built and the `warnings` it said.
fn verdict(seat: u8, controller: &str, seats: &BotSeats, warnings: &[String]) -> Runs {
    // The CLI's own prefix, space included, so seat 1 never reads seat 10's
    // lines.
    let prefix = format!("warning: seat {seat} ");
    let reasons: Vec<String> = warnings
        .iter()
        .filter_map(|w| w.strip_prefix(prefix.as_str()))
        .map(|said| reason_of(said, controller))
        .collect();
    match (seats.contestant_at(PlayerId::new(seat)), reasons.is_empty()) {
        (Some(_), true) => Runs::Yes,
        (_, false) => Runs::No(reasons.join("; ")),
        (None, true) => Runs::No(format!("the CLI's resume seats no bot at seat {seat}")),
    }
}

/// The tail of the CLI's line for a seat it converts to a human — both
/// conversion arms of `BotSeats::from_provenance` — which is false on the web:
/// nothing here converts a seat, and nobody is now playing it.
const CONVERTED_TAIL: &str = " — it resumes as a HUMAN seat, so you are now playing it";
/// The tail of the CLI's drift line (`identity_drift_warnings`): the CLI plays
/// the seat on under the current build; the web does not.
const DRIFT_TAIL: &str = " — the seat still plays, under the CURRENT build";
/// The head of the tail of the CLI's stream-seed line: what the CLI then does
/// about a second seed.
const SEED_TAIL: &str = " — this driver plays every bot seat off one stream";

/// One CLI warning about a seat — `said`, after its `warning: seat {i} ` —
/// cut to its clause: the CLI's reason, never reworded (cycle F, R-FP12).
/// Four shapes, each held whole with its cut by
/// `the_cli_s_sentence_shapes_and_their_cuts_are_held_verbatim`:
///
/// - a drift line, `records … — the seat still plays, under the CURRENT
///   build`: the tail cut — "records weights=A, but this build derives
///   weights=B", or `records controller "…" at version A, but this build
///   runs version B`;
/// - the stream-seed line: cut at its tail — "records bot stream seed A, but
///   an earlier bot seat records B";
/// - `eligible`'s refusal, `was played by controller "{c}", but {why} — it
///   resumes as a HUMAN seat, …`: `{why}`, `eligible`'s own sentence;
/// - a controller `from_parts` cannot read, `was played by controller "{c}",
///   which this build does not know how to run — it resumes as a HUMAN seat,
///   … ({why})`: the tail and `was played by ` cut, so the relative clause
///   keeps the noun it hangs on — `controller "{c}", which this build does
///   not know how to run ({why})`, `{why}` being `from_parts`' sentence.
///
/// A line in no shape it knows is answered after its prefix, whole: a CLI
/// rewording still refuses, in the CLI's words, and the pin cell fails first.
fn reason_of(said: &str, controller: &str) -> String {
    if let Some(drift) = said.strip_suffix(DRIFT_TAIL) {
        return drift.to_string();
    }
    if let Some((seed, _)) = said.split_once(SEED_TAIL) {
        return seed.to_string();
    }
    let converted = said.replacen(CONVERTED_TAIL, "", 1);
    let refused = format!("was played by controller \"{controller}\", but ");
    if let Some(why) = converted.strip_prefix(refused.as_str()) {
        return why.to_string();
    }
    match converted.strip_prefix("was played by ") {
        Some(unread) => unread.to_string(),
        None => converted,
    }
}

/// The FOREIGN bot seats of `record` (M6.5c2, R-6.5c2.10; cycle F, R-F4):
/// [`runs_here`]'s [`Runs::No`] seats, each with the CLI's reason. Empty for
/// an all-human game, for every game this build dealt, and for a game against
/// any bot this build's CLI resumes exactly — a grandfathered `lookahead-v1`
/// among them.
pub fn foreign(record: &GameRecord) -> Vec<(u8, BotDescriptor, String)> {
    runs_here(record)
        .into_iter()
        .filter_map(|(seat, d, runs)| match runs {
            Runs::Yes => None,
            Runs::No(why) => Some((seat, d, why)),
        })
        .collect()
}

/// The pair a foreign-bot refusal names beside its sentence
/// (`"foreign_bot": {"played", "runs"}`): the two strings
/// [`distinguishing_labels`] answers for the bot the game was played against
/// and the one this build runs — their labels, told apart where they tie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ForeignBot {
    pub played: String,
    pub runs: String,
}

/// The two labels a foreign-bot refusal names — the bot a game was `played`
/// against and the one this build `runs` — told apart when [`label`] alone
/// would name one bot twice (M6.5c2, R-C5; spec §2.4 as amended).
///
/// [`label`] names an identity, not a build of it: `Contestant::from_parts`
/// never reads the controller version and takes the identity keys
/// (`eval_impl`, `weights`, `placement_cap`) at any value. So a same-name
/// re-point — `v4-step9-draft1` shipped at a new controller version — labels
/// both bots "V4", and a knobbed controller at two knob sets labels both alike
/// (`lookahead-v1` at `b=4` and at `b=8` are both "V2 lookahead"). Named by
/// its labels alone, the refusal's pair would read `played: "V4"`, `runs:
/// "V4"`, and `why` would say "This move was made by V4" on a server that
/// runs V4. So, in order:
///
/// - the two labels differ: they stand as they are (`V2`, `V4`);
/// - they tie: each gains its controller version, `{label}.{version}`
///   (`V4.2`, `V4.3`);
/// - those tie too: each gains its controller and every parameter,
///   `{label} ({controller} k=v,…)`, the parameters in `BTreeMap` order —
///   `{label} ({controller})` when it has none.
///
/// **Against [`production`]** — the only `runs` any caller passes — two
/// different descriptors answer two different strings. The last arm is reached
/// only when the labels and the versions tie. Then either the controllers
/// differ, and it prints both; or both are production's controller and
/// `played` PARSED (unparsed, its label would be that controller's name, never
/// production's label), so each key is one `from_parts` accepts — none holds
/// `=` or `,` — and the parameters that differ print differently. Between two
/// ARBITRARY descriptors that does not hold: crafted keys can render two maps
/// alike (`{a: 1, b: 2}` and `{"a=1,b": 2}` both print `a=1,b=2`). Equal
/// descriptors are one bot, and a refusal names production's own descriptor
/// only for a record whose second bot seat carries another stream seed — the
/// CLI's seed line ([`runs_here`]) — which no web table deals.
///
/// **The screens keep [`label`]**: the lobby's row, the Players panel and
/// `ClientState.bots` name one bot each, and "V4" is its name there. Only a
/// sentence naming TWO needs them told apart.
pub fn distinguishing_labels(played: &BotDescriptor, runs: &BotDescriptor) -> (String, String) {
    let (played_label, runs_label) = (label(played), label(runs));
    if played_label != runs_label {
        return (played_label, runs_label);
    }
    let versioned = |l: &str, d: &BotDescriptor| format!("{l}.{}", d.version);
    let (played_versioned, runs_versioned) = (versioned(&played_label, played), versioned(&runs_label, runs));
    if played_versioned != runs_versioned {
        return (played_versioned, runs_versioned);
    }
    let knobbed = |l: &str, d: &BotDescriptor| {
        let params: Vec<String> = d.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
        if params.is_empty() {
            format!("{l} ({})", d.controller)
        } else {
            format!("{l} ({} {})", d.controller, params.join(","))
        }
    };
    (knobbed(&played_label, played), knobbed(&runs_label, runs))
}

/// The sentence a not-over game against a foreign bot is refused with (cycle
/// F, spec §2.3): the layer's own head, naming the bot the game was played
/// against, and after the colon the CLI's own reason — [`Runs::No`]'s clause —
/// `This game was played against {played}. This server can't continue it:
/// {why}`.
///
/// **`_runs` is in the signature and not in the sentence.** Every caller hands
/// in the pair the wire carries beside the sentence (`"foreign_bot": {"played",
/// "runs"}`, which keeps both), but the sentence no longer names the bot this
/// build deals: a build now plays on every bot its CLI resumes exactly, so
/// "now runs V4" would offer one of them as the reason, and the reason is the
/// CLI's.
pub fn cannot_continue(played: &str, _runs: &str, why: &str) -> String {
    format!("This game was played against {played}. This server can't continue it: {why}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor_of(c: Contestant) -> BotDescriptor {
        let (controller, version, params) = c.descriptor();
        BotDescriptor { controller, version, params }
    }

    /// `production()` is the triple a bot seat is DEALT under — the provenance
    /// `finalize` stamps through `BotSeats::seat_provenance` — so a game this
    /// build deals always runs here.
    #[test]
    fn the_production_bot_is_the_triple_every_bot_seat_is_dealt_under() {
        let dealt = BotSeats::new(vec![Some(PROVISIONAL_DEFAULT_BOT)], 7).seat_provenance();
        let seat = &dealt[0];
        assert_eq!(
            production(),
            BotDescriptor { controller: seat.controller.clone(), version: seat.version, params: seat.params.clone() },
        );
    }

    /// Every identity core can name has a label, V4 is "V4", and no two
    /// labels are one word — the compiler holds the first (the `match` has no
    /// `_` arm) and this cell holds the words.
    #[test]
    fn every_identity_is_labelled_and_the_labels_are_distinct() {
        let table = [
            (Contestant::NamedTable(NamedIdentity::V4Step9Draft1), "V4"),
            (Contestant::NamedTable(NamedIdentity::V2Padded), "V2"),
            (Contestant::NamedTable(NamedIdentity::V2RefitPadded), "V2 refit"),
            (Contestant::LOOKAHEAD_V1_DEFAULT, "V2 lookahead"),
            (Contestant::GREEDY_V1_DEFAULT, "V2 greedy"),
            (Contestant::ATOMIC_DIAG_DEFAULT, "atomic diagnostic"),
            (Contestant::MCTS_V1_DEFAULT, "MCTS diagnostic"),
            (Contestant::Policy(RolloutPolicy::Uniform), "uniform rung"),
            (Contestant::Policy(RolloutPolicy::RetreatAverse), "retreat-averse rung"),
            (Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT), "family-weighted rung"),
        ];
        // Every named identity is in the table: `NamedIdentity::ALL` is core's
        // own list, so a fourth arm fails HERE as well as at the `match`.
        for arm in NamedIdentity::ALL {
            assert!(table.iter().any(|(c, _)| *c == Contestant::NamedTable(arm)), "{arm:?} has no row");
        }
        for (contestant, want) in table {
            // Through the RECORD's halves, as a stored game reaches `label`.
            assert_eq!(label(&descriptor_of(contestant)), want, "{}", contestant.echo());
        }
        let words: std::collections::BTreeSet<&str> = table.iter().map(|(_, w)| *w).collect();
        assert_eq!(words.len(), table.len(), "two identities read as one label");
    }

    /// **A foreign-bot refusal never names one bot twice** (M6.5c2, R-C5): two
    /// labels that differ stand as they are; two that tie gain each side's
    /// controller version; two that tie on that too gain each side's controller
    /// and knobs, or the controller alone when there are none. Four rows — one
    /// per arm, and the last arm's knobless form — each pair distinct.
    #[test]
    fn a_refusal_s_two_labels_are_told_apart_by_version_and_then_by_knobs() {
        // Row 1: labels that differ are today's pair — the corpus's V2 vs V4.
        let v2 = descriptor_of(Contestant::NamedTable(NamedIdentity::V2Padded));
        assert_eq!(distinguishing_labels(&v2, &production()), ("V2".to_string(), "V4".to_string()));

        // Row 2: a same-name re-point — V4's name at the next controller
        // version, both strings read off V4's own, so a real bump moves nothing.
        let v4 = descriptor_of(Contestant::NamedTable(NamedIdentity::V4Step9Draft1));
        let repointed = BotDescriptor { version: v4.version + 1, ..v4.clone() };
        assert_eq!(label(&v4), label(&repointed), "premise: the two labels tie");
        assert_eq!(
            distinguishing_labels(&v4, &repointed),
            (format!("V4.{}", v4.version), format!("V4.{}", v4.version + 1)),
        );

        // Row 3: one controller at one version, another knob — the parameters
        // in `BTreeMap` order, the two identity keys read off the descriptor.
        let default = descriptor_of(Contestant::LOOKAHEAD_V1_DEFAULT);
        let Contestant::LookaheadV1 { k, m, cap, .. } = Contestant::LOOKAHEAD_V1_DEFAULT else {
            unreachable!("the const is the lookahead arm")
        };
        let wider = descriptor_of(Contestant::LookaheadV1 { b: 8, k, m, cap });
        assert_eq!(label(&default), label(&wider), "premise: the two labels tie");
        assert_eq!(default.version, wider.version, "premise: and so do the versions");
        let (eval_impl, weights) = (default.params["eval_impl"], default.params["weights"]);
        let pair = distinguishing_labels(&default, &wider);
        assert_eq!(
            pair,
            (
                format!("V2 lookahead (lookahead-v1 b=4,eval_impl={eval_impl},k=1,m=8,weights={weights})"),
                format!("V2 lookahead (lookahead-v1 b=8,eval_impl={eval_impl},k=1,m=8,weights={weights})"),
            ),
        );
        assert_ne!(pair.0, pair.1, "the pair is distinct");

        // Row 4: a knobless pair renders the controller alone, no trailing
        // space. Two DIFFERENT knobless descriptors reach this arm — their
        // labels AND their versions tying — when they name one label by two
        // controller spellings: an unparseable controller spelled like the
        // other's label (an unparsed controller's label is its own name), or
        // a rung's underscore alias (`retreat_averse` beside
        // `retreat-averse`; `policy.rs` parses both, `family_weighted` too);
        // two that differ only in version tie on their labels but stop at the version arm, row 2's shape. So
        // the row is that: "uniform rung" against the uniform rung.
        let uniform = descriptor_of(Contestant::Policy(RolloutPolicy::Uniform));
        let spelled = BotDescriptor { controller: "uniform rung".to_string(), ..uniform.clone() };
        assert!(uniform.params.is_empty(), "premise: knobless");
        assert_eq!(label(&spelled), label(&uniform), "premise: the two labels tie");
        assert_eq!(
            distinguishing_labels(&spelled, &uniform),
            ("uniform rung (uniform rung)".to_string(), "uniform rung (uniform)".to_string()),
        );
    }

    /// A descriptor this build cannot parse answers its controller name.
    #[test]
    fn an_unparseable_descriptor_answers_its_controller() {
        let unknown = BotDescriptor { controller: "bga-human".into(), version: 1, params: BTreeMap::new() };
        assert_eq!(label(&unknown), "bga-human");
        let mut bad_knob = descriptor_of(Contestant::LOOKAHEAD_V1_DEFAULT);
        bad_knob.params.insert("b".into(), 0);
        assert_eq!(label(&bad_knob), "lookahead-v1", "`from_parts` refuses b=0, so the name stands");
    }

    /// `recorded` is the record's non-human seats, in seat order.
    #[test]
    fn recorded_names_the_record_s_bot_seats_in_order() {
        use yokohama_core::{GameConfig, Rules, StartingOrderChoice};
        let rules = Rules::standard();
        let mut record = GameRecord::new(
            7,
            GameConfig::for_player_count(2),
            vec![StartingOrderChoice::Keep(0); 2],
            &rules,
        );
        assert!(recorded(&record).is_empty(), "no run provenance: no bot");
        record.set_run_seats(BotSeats::new(vec![None, None], 7).seat_provenance());
        assert!(recorded(&record).is_empty(), "an all-human game");
        record.set_run_seats(BotSeats::new(vec![None, Some(PROVISIONAL_DEFAULT_BOT)], 7).seat_provenance());
        assert_eq!(recorded(&record), vec![(1, production())]);
        assert_eq!(seat_bots(&record), vec![SeatBot { seat: 1, label: "V4".into() }]);
    }

    /// **A seat a person played on BGA is a person here too** (M6.5c2 cycle D,
    /// straw poll row 144). The importer names such a seat `bga-human`
    /// (`yokohama_cli::bga::CONTROLLER_BGA_HUMAN`) rather than `human`, so a
    /// reader of the file can tell an imported game from a typed one, and the
    /// CLI resumes it as a HUMAN seat. Counted as a bot here it was a FOREIGN
    /// one: an unfinished game refused as "played against bga-human", a
    /// finished one refused its undo, and a bot badge on a person.
    ///
    /// Over a CONSISTENT pair — the committed game with seat 1 re-seated as
    /// `bga-human` in the record and seated as a person, `Ada Bell`, at the
    /// table ([`crate::games::test_support::bga_human_pair`]) — loaded into a
    /// store and rebuilt through [`crate::games::Games::handle`], the path
    /// every route takes:
    ///
    /// - no bot in `recorded`, `foreign`, `ClientState.bots`, the Players
    ///   panel's facts or the table's `bot`;
    /// - the CLI's "resumes as a HUMAN seat" line, asserted as a PREMISE on the
    ///   resident `Repl` so the cell cannot pass by the CLI going quiet, masked
    ///   from the payload's `warnings` (the WHOLE array) — and precisely: the
    ///   same line naming seat 0, which the record gives a typed `human`, stays;
    /// - unfinished, the game OPENS rather than refusing, with seat 1 to move
    ///   and seat 1's holder the one whose turn it is — a person can move it.
    ///
    /// The INCONSISTENT pair — the same record beside a table that seats a bot
    /// there — is `api.rs`'s
    /// `a_pair_whose_table_and_record_disagree_about_a_bot_is_refused_in_one_409`.
    #[test]
    fn a_bga_human_seat_is_a_person_not_a_foreign_bot() {
        use crate::client::{facts_for, without_foreign_conversion, ClientState, Viewer};
        use crate::games::test_support::{
            bga_human_pair, games, who, BGA_SEAT_HOLDER, BOT_TO_MOVE_AT, PLAYED_ID,
        };
        use crate::store::Doc;
        use yokohama_cli::bga::CONTROLLER_BGA_HUMAN;
        use yokohama_core::PlayerId;

        let holding = |cut: Option<usize>| {
            let (record, table) = bga_human_pair(cut);
            let games = games();
            games.load_document(Doc::Record, PLAYED_ID, &record).unwrap();
            games.load_document(Doc::Table, PLAYED_ID, &table).unwrap();
            let record: GameRecord = serde_json::from_str(&record).expect("the pair's record");
            (games, record)
        };

        let (games, record) = holding(None);
        assert_eq!(recorded(&record), Vec::new(), "a person, not a bot");
        assert_eq!(foreign(&record), Vec::new(), "and so not a foreign one");
        let handle = games.handle(PLAYED_ID).expect("a consistent pair opens");
        let repl = handle.lock().unwrap();
        assert!(repl.session.is_over(), "premise: finished");
        let said = format!("warning: seat 1 was played by controller \"{CONTROLLER_BGA_HUMAN}\"");
        assert!(
            repl.session.load_warnings.iter().any(|w| w.starts_with(&said) && w.contains("resumes as a HUMAN seat")),
            "premise: the CLI says it — {:?}",
            repl.session.load_warnings,
        );
        let state = ClientState::of(PLAYED_ID, &repl, Viewer::Seat(PlayerId::new(0)), Vec::new());
        assert_eq!(state.bots, Vec::new(), "`ClientState.bots` names no bot");
        assert_eq!(state.warnings, Vec::<String>::new(), "the conversion line masked, and nothing else shown");
        let facts = facts_for(&repl, Viewer::Seat(PlayerId::new(0)));
        assert_eq!(facts.iter().map(|f| f.is_bot).collect::<Vec<_>>(), [false, false], "no badge on a person");
        let Ok(table) = crate::api::get_table(&games, &who("local@browser"), PLAYED_ID) else {
            panic!("the table reads")
        };
        assert_eq!(table.bot, None, "the lobby row names no bot either");

        let seat_0 = format!(
            "warning: seat 0 was played by controller \"{CONTROLLER_BGA_HUMAN}\", which this build does not know \
             how to run — it resumes as a HUMAN seat, so you are now playing it (x)"
        );
        assert_eq!(without_foreign_conversion(vec![seat_0.clone()], &record), vec![seat_0], "seat 0 is a typed human");

        let (games, _) = holding(Some(BOT_TO_MOVE_AT));
        let handle = games.handle(PLAYED_ID).expect("unfinished, a consistent pair opens");
        let repl = handle.lock().unwrap();
        assert!(!repl.session.is_over(), "premise: unfinished");
        assert_eq!(repl.session.state().current_player, PlayerId::new(1), "premise: seat 1 to move");
        let ada = ClientState::of(PLAYED_ID, &repl, Viewer::Seat(PlayerId::new(1)), Vec::new());
        assert!(ada.your_turn, "seat 1's holder, {BGA_SEAT_HOLDER}, is the one to move it");
    }

    // ---- Cycle F (R-F4; spec §2.3; R-FP12): the CLI's resume decides -------

    /// A record of `players` seats — a person at seat 0, and every other seat
    /// the bot this build deals, as `finalize` stamps it — handed to `edit`
    /// to rename, re-version or re-knob a bot seat the way a record written by
    /// ANOTHER build would carry it.
    fn record_of(players: u8, edit: impl FnOnce(&mut [SeatProvenance])) -> GameRecord {
        use yokohama_core::{GameConfig, Rules, StartingOrderChoice};
        let mut record = GameRecord::new(
            7,
            GameConfig::for_player_count(players),
            vec![StartingOrderChoice::Keep(0); usize::from(players)],
            &Rules::standard(),
        );
        let mut table = vec![None];
        table.resize(usize::from(players), Some(PROVISIONAL_DEFAULT_BOT));
        let mut seats = BotSeats::new(table, 7).seat_provenance();
        edit(seats.as_mut_slice());
        record.set_run_seats(seats);
        record
    }

    /// A two-seat record whose seat 1 carries `d`, all three halves.
    fn against(d: &BotDescriptor) -> GameRecord {
        record_of(2, |seats| {
            seats[1].controller = d.controller.clone();
            seats[1].version = d.version;
            seats[1].params = d.params.clone();
        })
    }

    /// [`runs_here`]'s answer for seat 1 of a two-seat record, the only bot.
    fn seat_1(record: &GameRecord) -> Runs {
        match runs_here(record).as_slice() {
            [(1, _, runs)] => runs.clone(),
            other => panic!("premise: one bot, at seat 1 — {other:?}"),
        }
    }

    /// **A bot the CLI resumes exactly plays on, whatever this build deals**
    /// (cycle F, R-F4; Review Focus 2; the catcher for `runs_here` comparing
    /// to production again). A genuine `lookahead-v1` record — the contestant's
    /// OWN descriptor, all three halves — runs here though production is V4,
    /// as does every arm the CLI's `eligible` admits; and production, as ever.
    #[test]
    fn a_genuine_lookahead_v1_seat_runs_here_while_production_is_v4() {
        let genuine = descriptor_of(Contestant::LOOKAHEAD_V1_DEFAULT);
        assert_ne!(genuine, production(), "premise: not the bot this build deals");
        let record = against(&genuine);
        assert_eq!(seat_1(&record), Runs::Yes, "the genuine record runs here");
        assert_eq!(foreign(&record), Vec::new(), "and so it is not foreign");
        for c in [
            Contestant::PRODUCTION,
            Contestant::GREEDY_V1_DEFAULT,
            Contestant::Policy(RolloutPolicy::Uniform),
            Contestant::Policy(RolloutPolicy::RetreatAverse),
            Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
        ] {
            assert_eq!(seat_1(&against(&descriptor_of(c))), Runs::Yes, "{}", c.echo());
        }
    }

    /// **An identity the CLI's `eligible` refuses is refused in `eligible`'s
    /// own words** (cycle F, Review Focus 2 — the shape V4 would have had the
    /// day V5 shipped, until the 2026-10-01 hotfix grandfathered it in the
    /// CLI's `eligible`; the catcher for `runs_here` admitting one). Its seat
    /// is reported with the reason, and the same code admits it the day the
    /// CLI does.
    #[test]
    fn an_ineligible_named_identity_is_refused_in_eligible_s_own_words() {
        let v2 = against(&descriptor_of(Contestant::NamedTable(NamedIdentity::V2Padded)));
        assert_eq!(
            seat_1(&v2),
            Runs::No(
                "v2-padded is a named research identity for arena weight comparisons, not available as a CLI bot"
                    .to_string()
            ),
        );
        let refit = descriptor_of(Contestant::NamedTable(NamedIdentity::V2RefitPadded));
        assert_eq!(
            seat_1(&against(&refit)),
            Runs::No(format!(
                "{} is a named research identity for arena weight comparisons, not available as a CLI bot",
                refit.controller,
            )),
        );
        let (seat, d, why) = foreign(&v2).pop().expect("the seat is foreign");
        assert_eq!((seat, d.controller.as_str()), (1, "v2-padded"));
        assert!(why.starts_with("v2-padded is a named research identity"), "{why}");
    }

    /// **A drift the CLI only WARNS about does not run here** (cycle F, review
    /// F-5; the catcher for `runs_here` ignoring a drift warning): the CLI plays
    /// a moved weights digest on "under the CURRENT build" — a different bot
    /// under the recorded name — so the web refuses it, naming the key. Two
    /// rows: production's own name and version with its digest moved by one,
    /// and the MONGREL `games.rs` fixture shape — `lookahead-v1`'s name over
    /// V4's knobs, whose digest is V4's.
    #[test]
    fn a_weights_drift_under_the_same_name_and_version_does_not_run_here() {
        let running = production();
        let digest = running.params["weights"];
        let mut moved = running.clone();
        moved.params.insert("weights".to_string(), digest.wrapping_add(1));
        assert_eq!(
            seat_1(&against(&moved)),
            Runs::No(format!(
                "records weights={}, but this build derives weights={digest}",
                digest.wrapping_add(1)
            )),
        );
        let mongrel = BotDescriptor { controller: "lookahead-v1".to_string(), ..running.clone() };
        let lookahead = descriptor_of(Contestant::LOOKAHEAD_V1_DEFAULT).params["weights"];
        assert_ne!(lookahead, digest, "premise: the two tables differ");
        assert_eq!(
            seat_1(&against(&mongrel)),
            Runs::No(format!("records weights={digest}, but this build derives weights={lookahead}")),
        );
    }

    /// A controller version this build does not run is drift too, naming both
    /// versions.
    #[test]
    fn a_version_drift_does_not_run_here_naming_both_versions() {
        let running = production();
        let next = BotDescriptor { version: running.version + 1, ..running.clone() };
        assert_eq!(
            seat_1(&against(&next)),
            Runs::No(format!(
                "records controller \"{}\" at version {}, but this build runs version {}",
                running.controller,
                running.version + 1,
                running.version,
            )),
        );
    }

    /// A named identity at knobs it never had is refused by `from_parts`, and
    /// the reason is `from_parts`' own sentence, after the CLI's clause.
    #[test]
    fn a_named_identity_at_knobs_it_never_had_is_refused_in_from_parts_words() {
        let running = production();
        let mut wider = running.clone();
        wider.params.insert("b".to_string(), 8);
        let refusal = Contestant::from_parts(&wider.controller, &wider.params).expect_err("premise: refused");
        assert!(refusal.contains("knobless named identity"), "premise: from_parts' sentence — {refusal}");
        assert_eq!(
            seat_1(&against(&wider)),
            Runs::No(format!(
                "controller \"{}\", which this build does not know how to run ({refusal})",
                running.controller
            )),
        );
    }

    /// A controller this build does not know, and a diagnostic, are refused
    /// with the CLI's reason: `from_parts`' sentence, and `eligible`'s.
    #[test]
    fn an_unknown_controller_and_a_diagnostic_are_refused_with_the_cli_s_reason() {
        let unknown = BotDescriptor { controller: "nope-v9".to_string(), version: 1, params: BTreeMap::new() };
        let refusal = Contestant::from_parts("nope-v9", &BTreeMap::new()).expect_err("premise: unknown");
        assert_eq!(
            seat_1(&against(&unknown)),
            Runs::No(format!("controller \"nope-v9\", which this build does not know how to run ({refusal})")),
        );
        for diagnostic in [Contestant::ATOMIC_DIAG_DEFAULT, Contestant::MCTS_V1_DEFAULT] {
            let d = descriptor_of(diagnostic);
            assert_eq!(
                seat_1(&against(&d)),
                Runs::No(format!("{} is a diagnostic contestant, not available as a CLI bot", d.controller)),
                "{}",
                diagnostic.echo(),
            );
        }
    }

    /// **The CLI's sentence shapes, held verbatim, and their cuts** (cycle F,
    /// R-FP12; the catcher for a tail cut from the wrong sentence). Every
    /// warning `BotSeats::from_provenance` writes about a seat — both
    /// conversion arms, both drift lines, the stream-seed line — is asserted
    /// WHOLE, so a CLI rewording fails here first rather than quietly changing
    /// a refusal; and beside each, the clause [`runs_here`] cuts from it: the
    /// `warning: seat {i} ` prefix gone, and the tail — " — it resumes as a
    /// HUMAN seat, so you are now playing it", " — the seat still plays, under
    /// the CURRENT build", or " — this driver plays every bot seat off one
    /// stream, …" — gone with it.
    #[test]
    fn the_cli_s_sentence_shapes_and_their_cuts_are_held_verbatim() {
        /// A shape's name, its record, the CLI's warnings whole, and the cut
        /// per bot seat.
        type Shape = (&'static str, GameRecord, Vec<String>, Vec<(u8, Runs)>);
        let running = production();
        let digest = running.params["weights"];
        let unknown = Contestant::from_parts("nope-v9", &BTreeMap::new()).expect_err("premise: unknown");
        let shapes: Vec<Shape> = vec![
            (
                "eligible refuses: converted, eligible's sentence",
                against(&descriptor_of(Contestant::NamedTable(NamedIdentity::V2Padded))),
                vec![
                    "warning: seat 1 was played by controller \"v2-padded\", but v2-padded is a named research \
                     identity for arena weight comparisons, not available as a CLI bot — it resumes as a HUMAN \
                     seat, so you are now playing it"
                        .to_string(),
                ],
                vec![(
                    1,
                    Runs::No(
                        "v2-padded is a named research identity for arena weight comparisons, not available as a \
                         CLI bot"
                            .to_string(),
                    ),
                )],
            ),
            (
                "from_parts refuses: converted, from_parts' sentence",
                against(&BotDescriptor { controller: "nope-v9".to_string(), version: 1, params: BTreeMap::new() }),
                vec![format!(
                    "warning: seat 1 was played by controller \"nope-v9\", which this build does not know how to \
                     run — it resumes as a HUMAN seat, so you are now playing it ({unknown})"
                )],
                vec![(
                    1,
                    Runs::No(format!(
                        "controller \"nope-v9\", which this build does not know how to run ({unknown})"
                    )),
                )],
            ),
            (
                "a version drift and a knob drift on one seat: both, in the CLI's order",
                against(&BotDescriptor {
                    version: running.version + 1,
                    params: {
                        let mut p = running.params.clone();
                        p.insert("weights".to_string(), digest.wrapping_add(1));
                        p
                    },
                    ..running.clone()
                }),
                vec![
                    format!(
                        "warning: seat 1 records controller \"{}\" at version {}, but this build runs version {} \
                         — the seat still plays, under the CURRENT build",
                        running.controller,
                        running.version + 1,
                        running.version,
                    ),
                    format!(
                        "warning: seat 1 records weights={}, but this build derives weights={digest} — the seat \
                         still plays, under the CURRENT build",
                        digest.wrapping_add(1),
                    ),
                ],
                vec![(
                    1,
                    Runs::No(format!(
                        "records controller \"{}\" at version {}, but this build runs version {}; records \
                         weights={}, but this build derives weights={digest}",
                        running.controller,
                        running.version + 1,
                        running.version,
                        digest.wrapping_add(1),
                    )),
                )],
            ),
            (
                "a second stream seed: the later seat is foreign, the earlier one runs",
                record_of(3, |seats| seats[2].agent_seed = Some(8)),
                vec![
                    "warning: seat 2 records bot stream seed 8, but an earlier bot seat records 7 — this driver \
                     plays every bot seat off one stream, so it keeps 7 and seat 2 will not reproduce its \
                     recorded line"
                        .to_string(),
                ],
                vec![
                    (1, Runs::Yes),
                    (2, Runs::No("records bot stream seed 8, but an earlier bot seat records 7".to_string())),
                ],
            ),
        ];
        for (shape, record, said, want) in shapes {
            let (_, warnings) = BotSeats::from_provenance(record.run.as_ref());
            assert_eq!(warnings, said, "{shape}: the CLI's words, whole");
            let got: Vec<(u8, Runs)> = runs_here(&record).into_iter().map(|(seat, _, runs)| (seat, runs)).collect();
            assert_eq!(got, want, "{shape}: the cut");
        }
    }

    /// **The lobby row's `runs_here` is the same predicate** (cycle F, spec
    /// §2.3): asked of one descriptor, it answers as the record would.
    #[test]
    fn a_descriptor_runs_here_exactly_when_a_record_naming_it_would() {
        for d in [
            production(),
            descriptor_of(Contestant::LOOKAHEAD_V1_DEFAULT),
            descriptor_of(Contestant::NamedTable(NamedIdentity::V2Padded)),
            BotDescriptor { version: production().version + 1, ..production() },
            BotDescriptor { controller: "lookahead-v1".to_string(), ..production() },
            BotDescriptor { controller: "nope-v9".to_string(), version: 1, params: BTreeMap::new() },
        ] {
            assert_eq!(descriptor_runs(&d), seat_1(&against(&d)), "{d:?}");
        }
        assert_eq!(descriptor_runs(&descriptor_of(Contestant::LOOKAHEAD_V1_DEFAULT)), Runs::Yes);
    }
}
