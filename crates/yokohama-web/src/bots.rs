//! **Which bot played a game, in the words a person reads** (M6.5c2,
//! R-6.5c2.10).
//!
//! A bot seat's identity is the RECORD's: every `SeatProvenance` carries its
//! controller's name, that controller's version and every knob, and
//! `Contestant::from_parts` is the inverse that reads the name and the knobs
//! back. This module adds the two things a record cannot say for itself: a
//! SHORT LABEL for each identity (`v4-step9-draft1` is "V4"), and whether THIS
//! build runs it.
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
use yokohama_cli::bot::PROVISIONAL_DEFAULT_BOT;
use yokohama_core::{Contestant, GameRecord, NamedIdentity, RolloutPolicy, CONTROLLER_HUMAN};

use crate::table::{SeatDoc, TableDoc, TableStatus};

/// One bot seat's identity as its record names it: a `SeatProvenance`'s
/// `(controller, version, params)`, without the stream seed (a fact about one
/// game, not about the bot) and the observation channel.
///
/// **The triple, not the name** (review m12): one name can ship at two
/// controller versions or two knob sets, and [`TableBot::runs_here`] compares
/// what a record would have to match to resume under this build.
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
    /// Whether this build runs that bot: the descriptor equals [`production`].
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

/// `TableView.bot` for `doc` (spec §2.4, "Where the lobby gets it").
///
/// - no bot seat: `None`;
/// - undealt (`open` or `dealing`): the production bot, which is what
///   `finalize` will deal, so it runs here;
/// - dealt: the mirror's descriptor — the record's, cached — or `None` for a
///   table whose mirror was last written before the field existed (its row
///   reads "the bot" with no version; review m7).
pub(crate) fn table_bot(doc: &TableDoc) -> Option<TableBot> {
    if !doc.seats.iter().any(|s| matches!(s, SeatDoc::Bot)) {
        return None;
    }
    let bot = match doc.status {
        TableStatus::Open | TableStatus::Dealing => production(),
        TableStatus::Playing => doc.turn.as_ref()?.bot.clone()?,
    };
    Some(TableBot { label: label(&bot), runs_here: bot == production() })
}

/// `ClientState.bots`: every bot seat the record names, labelled.
pub(crate) fn seat_bots(record: &GameRecord) -> Vec<SeatBot> {
    recorded(record).into_iter().map(|(seat, d)| SeatBot { seat, label: label(&d) }).collect()
}

/// The FOREIGN bot seats of `record` (M6.5c2, R-6.5c2.10): every seat whose
/// recorded `(controller, version, params)` is not [`production`]'s. Empty for
/// an all-human game and for every game this build dealt.
pub fn foreign(record: &GameRecord) -> Vec<(u8, BotDescriptor)> {
    let running = production();
    recorded(record).into_iter().filter(|(_, d)| *d != running).collect()
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
/// its labels alone, the refusal would read "This game was played against V4.
/// This server now runs V4 and can't continue it." So, in order:
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
/// descriptors are one bot, which no refusal names ([`foreign`] is every seat
/// that is not [`production`]).
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

/// The sentence a not-over game against a foreign bot is refused with — the
/// layer's own words (spec §2.4), not the CLI's.
pub fn cannot_continue(played: &str, runs: &str) -> String {
    format!("This game was played against {played}. This server now runs {runs} and can't continue it.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use yokohama_cli::bot::BotSeats;

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
        // labels AND their versions tying — only when one controller is
        // unparseable and spelled like the other's label (an unparsed
        // controller's label is its own name); two that differ only in version
        // tie on their labels but stop at the version arm, row 2's shape. So
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
    /// Over the committed pair with seat 1 re-seated as `bga-human`: no bot in
    /// `recorded`, `foreign`, `ClientState.bots` or the Players panel's facts —
    /// and the CLI's "resumes as a HUMAN seat" line, asserted as a PREMISE on
    /// the resident `Repl` so the cell cannot pass by the CLI going quiet, is
    /// masked from the payload's `warnings` (the WHOLE array). The mask is
    /// precise: the same line naming seat 0, which the record gives a typed
    /// `human`, stays.
    #[test]
    fn a_bga_human_seat_is_a_person_not_a_foreign_bot() {
        use crate::client::{facts_for, without_foreign_conversion, ClientState, Viewer};
        use crate::games::test_support::{foreign_pair, PLAYED_ID};
        use yokohama_cli::bga::CONTROLLER_BGA_HUMAN;
        use yokohama_cli::repl::Repl;
        use yokohama_cli::session::Session;
        use yokohama_core::{PlayerId, Rules};

        let (record, _) = foreign_pair(CONTROLLER_BGA_HUMAN, None);
        let record: GameRecord = serde_json::from_str(&record).expect("the pair's record");
        assert_eq!(recorded(&record), Vec::new(), "a person, not a bot");
        assert_eq!(foreign(&record), Vec::new(), "and so not a foreign one");

        let repl = Repl::new(Session::from_record(record.clone(), Rules::standard()).expect("it loads"));
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

        let seat_0 = format!(
            "warning: seat 0 was played by controller \"{CONTROLLER_BGA_HUMAN}\", which this build does not know \
             how to run — it resumes as a HUMAN seat, so you are now playing it (x)"
        );
        assert_eq!(without_foreign_conversion(vec![seat_0.clone()], &record), vec![seat_0], "seat 0 is a typed human");
    }
}
