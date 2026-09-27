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
//! refusals, and they are total by CONSTRUCTION: [`label`] reads a
//! `Contestant` through an exhaustive `match` with no `_` arm at either level —
//! the CLI's `bot::eligible` discipline — so the day core gains a V5 identity
//! this crate stops compiling until V5 has a label. That one line is the only
//! web edit a new bot needs; everything else here follows
//! `bot::PROVISIONAL_DEFAULT_BOT`.
//!
//! **Nothing here stores a bot.** `SeatDoc::Bot` still does not say which one
//! (`table.rs`): a dealt table's is the record's, cached in the turn mirror
//! ([`crate::table::TurnMirror::bot`]), and an undealt table's is the one
//! `finalize` will deal, [`production`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
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
/// as the CLI's resume path reads them, and named by [`label_of`]. A
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

/// Every seat `record` names a non-human controller for, in seat order, with
/// its descriptor. Empty for an all-human game and for a record with no run
/// provenance. A seat index past `u8` is skipped: no record this layer writes
/// has one, and `recorded` is read before `Session::from_record` validates.
pub fn recorded(record: &GameRecord) -> Vec<(u8, BotDescriptor)> {
    let Some(run) = &record.run else { return Vec::new() };
    run.seats
        .iter()
        .enumerate()
        .filter(|(_, s)| s.controller != CONTROLLER_HUMAN)
        .filter_map(|(i, s)| {
            let seat = u8::try_from(i).ok()?;
            Some((
                seat,
                BotDescriptor { controller: s.controller.clone(), version: s.version, params: s.params.clone() },
            ))
        })
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
}
