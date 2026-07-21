//! Independent rules oracle for the Yokohama engine (Phase 2.9, Stage 1).
//!
//! **Visible-coupling guarantee (phase plan, Structure section):** this crate
//! depends on `yokohama-core` for TYPES ONLY — `GameState`, `Rules` (the
//! static-data bundle: data, not logic), `Move`, the id types, and the money
//! type (`u32`) — never for logic. No function from `yokohama_core::game`
//! (other than the `Rules` data accessors published by the orchestrator, spec
//! §9-C) may be called here; every rules computation in this crate is authored
//! from the rules sources (`analysis-1/`, `guidingpath.md`'s decision log,
//! CLAUDE.md's Rules-knowledge section) by an agent that has NOT read the
//! engine's `legal_moves.rs` / `apply_move.rs` / `movement.rs` / `scoring.rs` /
//! `choices.rs` / `actions/` / `data.rs`. Any coupling beyond types is visible
//! in `Cargo.toml` imports and is re-checked at every stage review.
//!
//! The public contract is [`oracle_moves`]: for one engine-reachable state it
//! enumerates every legal move in the engine's exact [`Move`] encoding
//! ([`OracleMoves::exact`]) EXCEPT the on-board president-movement family, which
//! is emitted as one [`RouteLabel`] per distinct `(destination, fee-vector)`
//! ([`OracleMoves::movement`]). The label form exists because the engine
//! canonicalizes president routes to one representative per distinct
//! `(destination, mover-cost, credit-vector)`, so exact `Move::MovePresident`
//! values are not comparable across independent implementations but their
//! labels are (spec §1.1).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use yokohama_core::{AreaAction, AreaId, AreaType, GameState, Move, PlayerId, Rules, TurnPhase};

mod additional_actions;
mod area_actions_cards;
mod area_actions_market;
mod area_actions_tracks;
mod buildings_and_bonus;
mod choices;
mod common;
mod movement;
mod placement;
pub mod route_reference;
pub mod scoring;

#[cfg(test)]
mod test_support;

/// Label for one distinct president-movement option, mirroring the engine's L2
/// route canonicalization: one representative per distinct
/// `(destination, mover-cost, credit-vector)` over simple paths (spec §1.1,
/// decision log 2026-07-19 §8 veto row / 2026-07-20 effective-route row).
///
/// `fees` keys every Yen the mover pays on the effective route by recipient:
/// `Some(player)` is a 1-Yen president fee credited to that player; `None` is a
/// bank toll (Canal crossing). An absent key means zero. This mirrors the
/// engine's L2 `RouteVector` keying (`BTreeMap<Option<PlayerId>, u32>` by fee
/// recipient — `types/ids.rs` doc comment).
///
/// `Ord`/`Hash` are derived over `(destination, fees)` purely so labels live in
/// sorted/hashed sets for the runner's set comparison — no semantic meaning
/// attaches to the ordering.
///
/// SERDE (custom, serialization plumbing only): the derived map encoding could
/// not JSON-serialize the `None` (bank-toll) fee key — serde_json rejects null
/// map keys — so `fees` is encoded as an entry LIST instead:
/// `{"destination": <id>, "fees": [[null, 1], [2, 1]]}` (each entry
/// `[recipient, yen]`, recipient `null` = bank). Every label — Canal bank
/// tolls included — round-trips through JSON (pinned by
/// `fees_with_none_key_round_trip_through_json`). Nothing persisted ever used
/// the old derived encoding (artifacts store labels as `route:<Debug>` keys,
/// never serde), so there is no legacy format to read.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RouteLabel {
    /// Where the President ends the Movement step.
    pub destination: AreaId,
    /// Every Yen the mover pays on the effective route, keyed by recipient.
    pub fees: BTreeMap<Option<PlayerId>, u32>,
}

impl Serialize for RouteLabel {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        // BTreeMap iteration order makes the entry list deterministic.
        let fees: Vec<(&Option<PlayerId>, &u32)> = self.fees.iter().collect();
        let mut s = serializer.serialize_struct("RouteLabel", 2)?;
        s.serialize_field("destination", &self.destination)?;
        s.serialize_field("fees", &fees)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for RouteLabel {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            destination: AreaId,
            fees: Vec<(Option<PlayerId>, u32)>,
        }
        let wire = Wire::deserialize(deserializer)?;
        let mut fees = BTreeMap::new();
        for (recipient, yen) in wire.fees {
            if fees.insert(recipient, yen).is_some() {
                return Err(serde::de::Error::custom(format!(
                    "RouteLabel fees entry list repeats recipient {recipient:?}"
                )));
            }
        }
        Ok(RouteLabel { destination: wire.destination, fees })
    }
}

impl RouteLabel {
    /// Total Yen the mover spends on this route = player credits + bank tolls.
    pub fn mover_cost(&self) -> u32 {
        self.fees.values().sum()
    }
}

/// The oracle's answer for one state: every legal-move family EXCEPT on-board
/// president movement in the engine's exact [`Move`] encoding (`exact`), plus
/// the president-movement family as [`RouteLabel`]s (`movement`).
///
/// Both are duplicate-free; element order carries no meaning (the Stage-1
/// runner compares them as sets). `exact` never contains a
/// `Move::MovePresident`; `movement` is non-empty only in the `Movement` phase
/// with an empty pending-choice queue and the President on the board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleMoves {
    /// Every legal move except on-board `Move::MovePresident`, exact encoding.
    pub exact: Vec<Move>,
    /// The president-movement family, one entry per distinct legal label.
    pub movement: Vec<RouteLabel>,
}

/// Independently-authored legal-move enumeration for `state` under `rules`.
///
/// Dispatch order (spec §1.4): game over ⇒ empty; a non-empty pending-choice
/// queue ⇒ the head choice's resolution moves only (in EVERY phase, Movement
/// included); else per-phase per the §1.4 table. Outputs are deduplicated for
/// determinism (`exact` by `PartialEq` — `Move` derives no `Ord`/`Hash`;
/// `movement` sorted+deduped via its derived `Ord`).
pub fn oracle_moves(state: &GameState, rules: &Rules) -> OracleMoves {
    // Game over: the engine's post-game-over safety is the empty legal list
    // (spec §1.4 / §3.8; decision log 2026-07-18 session-boundary row).
    if state.is_game_over() {
        return OracleMoves { exact: Vec::new(), movement: Vec::new() };
    }

    // Queue-first: a non-empty pending-choice queue means the ONLY legal moves
    // are the head choice's resolutions — in every phase, `Movement` included,
    // and even `EndAdditionalPhase` is rejected while choices are queued
    // (spec §2; decision log 2026-07-19 close-out + FINAL-review I1 rows).
    if !state.pending_choices.is_empty() {
        let mut exact = choices::head_choice_moves(state, rules);
        dedup_moves(&mut exact);
        return OracleMoves { exact, movement: Vec::new() };
    }

    // Per-phase enumeration (spec §1.4 table).
    let (mut exact, mut movement) = match state.turn_phase {
        // Mercenary-return windows ARE these phases (decision log 2026-07-19 §5.3-4).
        TurnPhase::AdditionalActionFirst | TurnPhase::AdditionalActionSecond => {
            (additional_actions::additional_action_moves(state, rules), Vec::new())
        }
        TurnPhase::Placement => (placement::placement_moves(state, rules), Vec::new()),
        // Movement returns both the exact part (PlacePresident + retreat family)
        // and the on-board route labels; the module decides by president location.
        TurnPhase::Movement => movement::movement_moves(state, rules),
        TurnPhase::AreaAction => (area_action_moves(state, rules), Vec::new()),
        TurnPhase::FivePowerBonus => (buildings_and_bonus::five_power_moves(state), Vec::new()),
        TurnPhase::Building => (buildings_and_bonus::building_moves(state, rules), Vec::new()),
        // Recovery is automatic; its only decision is the queued `RecoveryLeave`
        // (handled above by the queue-first branch). A choice-free Recovery
        // state yields the empty set (spec §3.7, believed transient §7.2).
        TurnPhase::Recovery => (Vec::new(), Vec::new()),
        // No moves: terminal rest state at game over (handled above), and a
        // non-terminal `TurnEnd` is believed transient (spec §3.8 / §7.2).
        TurnPhase::TurnEnd => (Vec::new(), Vec::new()),
    };

    dedup_moves(&mut exact);
    dedup_labels(&mut movement);
    OracleMoves { exact, movement }
}

/// Area-Action facade (spec §5, Author A): dispatch on the acting area's
/// `AreaType` to the owning module, then append `AreaAction::Skip` iff the area
/// permits declining all optional components (`allows_area_action_skip`, spec
/// §3.4). The per-area modules return already-`ExecuteAreaAction`-wrapped moves
/// WITHOUT `Skip`; the single canonical decline is added here.
fn area_action_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    // No acting area (President in hand and not a mercenary sub-turn) ⇒ no
    // area action is reachable; defensive empty.
    let Some(area) = common::acting_area(state) else {
        return Vec::new();
    };
    let Some(cfg) = state.board_layout.areas.get(&area) else {
        return Vec::new();
    };
    let area_type = cfg.area_type;
    let power = common::action_power(state, state.current_player, area);

    let mut moves = match area_type {
        AreaType::CopperMine
        | AreaType::SilkMill
        | AreaType::TeaPlantation
        | AreaType::FishingGround
        | AreaType::Bank => {
            area_actions_market::production_or_bank_move(area_type).into_iter().collect()
        }
        AreaType::PortA | AreaType::PortB => {
            area_actions_cards::port_moves(state, rules, area_type, power)
        }
        AreaType::LaboratoryA | AreaType::LaboratoryB => {
            area_actions_cards::laboratory_moves(state, rules, area_type, power)
        }
        AreaType::EmploymentAgency => area_actions_market::employment_moves(state, power),
        AreaType::Chinatown => area_actions_market::chinatown_moves(state, rules, power),
        AreaType::Church => area_actions_tracks::church_moves(state, power, area),
        AreaType::Customs => area_actions_tracks::customs_moves(state, power, area),
        // The Canal is never an acting area (no President or mercenary target
        // may sit there); defensive empty.
        AreaType::Canal => Vec::new(),
    };

    if area_type.allows_area_action_skip() {
        moves.push(Move::ExecuteAreaAction { action: AreaAction::Skip });
    }
    moves
}

/// Deduplicate `exact` in place, preserving first-occurrence order. `Move`
/// derives neither `Ord` nor `Hash`, so the key is the derived `Debug`
/// rendering — a faithful stable identity for these fully-structural types
/// (all fields printed; `BTreeMap` renders in deterministic order), the same
/// reasoning the difftest comparison keys rest on. Replaces the original
/// O(n²) `PartialEq` scan, which was quadratic-catastrophic on the retreat
/// family (2^17 moves at the Stage-3 worst-case battery state ⇒ minutes per
/// call; Stage-3 harness finding, 2026-07-20 — orchestrator perf plumbing,
/// zero semantic change, pinned by `dedup_moves_matches_quadratic_reference`).
fn dedup_moves(v: &mut Vec<Move>) {
    let mut seen = std::collections::HashSet::with_capacity(v.len());
    v.retain(|m| seen.insert(format!("{m:?}")));
}

/// Sort + dedup labels via `RouteLabel`'s derived `Ord` (the L2 identity is
/// structural equality on `(destination, fees)`).
fn dedup_labels(v: &mut Vec<RouteLabel>) {
    v.sort();
    v.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::two_player_state;
    use std::collections::BTreeMap;
    use std::collections::HashSet;
    use yokohama_core::{PendingChoice, PlayerId, Rules, TurnPhase};

    /// The HashSet-keyed dedup is semantically identical to the original
    /// quadratic PartialEq scan: same survivors, same first-occurrence
    /// order (orchestrator perf plumbing pin, Stage-3 2026-07-20).
    #[test]
    fn dedup_moves_matches_quadratic_reference() {
        fn quadratic(v: &mut Vec<Move>) {
            let mut i = 0;
            while i < v.len() {
                if v[..i].contains(&v[i]) {
                    v.remove(i);
                } else {
                    i += 1;
                }
            }
        }
        let a = |n: u8| Move::PlaceThreeAreas {
            placements: vec![yokohama_core::AreaId::new(n)],
        };
        let retreat = |pairs: &[(u8, u8)]| Move::ReturnPresidentToHand {
            recover: pairs
                .iter()
                .map(|&(ar, c)| (yokohama_core::AreaId::new(ar), c))
                .collect(),
        };
        let sample = vec![
            a(1),
            Move::EndAdditionalPhase,
            a(1), // dup of [0]
            retreat(&[(2, 1)]),
            a(2),
            retreat(&[(2, 1)]), // dup of [3]
            Move::EndAdditionalPhase, // dup of [1]
            retreat(&[(2, 2)]), // NOT a dup (different count)
        ];
        let mut fast = sample.clone();
        let mut slow = sample;
        dedup_moves(&mut fast);
        quadratic(&mut slow);
        assert_eq!(fast, slow, "same survivors in the same first-occurrence order");
        assert_eq!(fast.len(), 5);
    }

    // --- RouteLabel type behavior (spec §1.1 / §9-A) ---

    #[test]
    fn route_label_mover_cost_sums_all_fees() {
        let mut fees = BTreeMap::new();
        fees.insert(Some(PlayerId::new(1)), 1u32); // president fee
        fees.insert(None, 1u32); // bank (canal) toll
        let l = RouteLabel { destination: AreaId::new(2), fees };
        assert_eq!(l.mover_cost(), 2);
    }

    #[test]
    fn route_label_mover_cost_is_zero_for_free_teleport() {
        let l = RouteLabel { destination: AreaId::new(3), fees: BTreeMap::new() };
        assert_eq!(l.mover_cost(), 0);
    }

    #[test]
    fn route_label_ord_is_total_and_deterministic() {
        let bare = |d: u8| RouteLabel { destination: AreaId::new(d), fees: BTreeMap::new() };
        let mut v = [bare(3), bare(1), bare(2)];
        v.sort();
        assert_eq!(
            v.iter().map(|x| x.destination).collect::<Vec<_>>(),
            vec![AreaId::new(1), AreaId::new(2), AreaId::new(3)],
        );
    }

    #[test]
    fn route_label_usable_in_hashset_with_option_keys() {
        // The runner compares label sets in-memory; prove Hash+Eq work over a
        // non-empty Option-keyed fee map (both Some and None keys).
        let mut fees = BTreeMap::new();
        fees.insert(Some(PlayerId::new(1)), 1u32);
        fees.insert(None, 2u32);
        let label = RouteLabel { destination: AreaId::new(2), fees };
        let mut set = HashSet::new();
        set.insert(label.clone());
        assert!(set.contains(&label));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn fees_with_none_key_round_trip_through_json() {
        // The derived map encoding could not JSON-serialize a `None` (bank-toll)
        // fee key — serde_json rejects null map keys. The custom entry-LIST
        // encoding (`"fees": [[null, 1], [2, 1]]`) exists precisely so EVERY
        // label — Canal bank tolls included — round-trips through JSON.
        let mut fees = BTreeMap::new();
        fees.insert(None, 1u32); // bank (Canal) toll
        fees.insert(Some(PlayerId::new(2)), 1u32); // president fee
        let label = RouteLabel { destination: AreaId::new(2), fees };
        let json = serde_json::to_string(&label).expect("bank-toll labels must serialize");
        assert!(json.contains("null"), "the bank key must be encoded as an entry with null: {json}");
        let back: RouteLabel = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(label, back);
    }

    #[test]
    fn route_label_serde_round_trip_empty_fees() {
        // Empty-fee (free-teleport) labels round-trip cleanly through JSON.
        let original = RouteLabel { destination: AreaId::new(5), fees: BTreeMap::new() };
        let json = serde_json::to_string(&original).expect("serialize");
        let back: RouteLabel = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, back);
    }

    // --- Dispatch (spec §1.4 table; §6.2 dispatch row) ---

    #[test]
    fn game_over_yields_all_empty() {
        let mut s = two_player_state();
        // 2p: start_player 0 ⇒ player_before_start = 1; TurnEnd + final round.
        s.end_game_triggered = true;
        s.final_round = true;
        s.trigger_player = Some(PlayerId::new(0));
        s.current_player = PlayerId::new(1);
        s.turn_phase = TurnPhase::TurnEnd;
        assert!(s.is_game_over(), "fixture must be game-over");
        let om = oracle_moves(&s, &Rules::empty_for_tests());
        assert!(om.exact.is_empty());
        assert!(om.movement.is_empty());
    }

    #[test]
    fn queue_head_routes_to_choices_even_in_movement_phase() {
        // Decision log 2026-07-19 close-out: queue-first, movement empty too.
        let mut s = two_player_state();
        s.turn_phase = TurnPhase::Movement;
        s.pending_choices = vec![PendingChoice::AnyGood];
        let om = oracle_moves(&s, &Rules::empty_for_tests());
        assert_eq!(om.exact.len(), 4, "AnyGood ⇒ exactly 4 ChooseAnyGood moves");
        assert!(om.movement.is_empty(), "movement is empty while a choice is queued");
    }

    #[test]
    fn recovery_with_empty_queue_is_empty() {
        let mut s = two_player_state();
        s.turn_phase = TurnPhase::Recovery;
        let om = oracle_moves(&s, &Rules::empty_for_tests());
        assert!(om.exact.is_empty());
        assert!(om.movement.is_empty());
    }

    #[test]
    fn non_terminal_turn_end_is_empty() {
        let mut s = two_player_state();
        s.turn_phase = TurnPhase::TurnEnd;
        assert!(!s.is_game_over());
        let om = oracle_moves(&s, &Rules::empty_for_tests());
        assert!(om.exact.is_empty());
        assert!(om.movement.is_empty());
    }
}
