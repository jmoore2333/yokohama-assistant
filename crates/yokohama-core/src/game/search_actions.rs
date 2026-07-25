//! Factored, lazy bot-facing action source over `&GameState` + `&Rules`
//! (Phase 3.1a; spec `docs/plans/2026-07-25-phase3.1-action-interface-spec.md`).
//!
//! Purely additive to [`get_legal_moves`](crate::game::get_legal_moves): that
//! exhaustive `Vec<Move>` surface is frozen and stays the authority
//! `apply_move` checks against. This module exists because the materialized
//! list has a reachable six-figure worst case — the President-recovery
//! count-vector product `3³·2¹⁴ = 442,368` moves — that allocates before
//! progressive widening can prune it. The factored source yields move
//! *families* first, then pages/samples within a family without ever building
//! the exploding `Vec`.
//!
//! Task 1 (this commit) delivers [`MoveFamily`] + [`families`] — the dispatch
//! skeleton that mirrors `get_legal_moves`' queue-first + phase dispatch. The
//! per-family machinery (the retreat mixed-radix constructor/index/sample, the
//! placement combination construction, the small-family delegations) and the
//! lazy route enumerator land in later 3.1a/3.1b tasks.

use std::collections::BTreeMap;

use crate::game::Rules;
use crate::types::*;

/// A tag naming one decision-phase move family legal at the current state.
///
/// Returned by [`families`], which mirrors [`get_legal_moves`]'s queue-first +
/// `match turn_phase` dispatch (`legal_moves.rs:9-24`) but yields these tags
/// instead of moves; later 3.1a/3.1b tasks page and sample the actual moves
/// per tag.
///
/// Only [`MoveFamily::MovementRetreat`] carries a payload: it owns a clone of
/// the current player's `on_board` assistant map so the exploding retreat
/// family can be indexed/sampled without re-reading `GameState`. Every other
/// family re-derives its moves from `&GameState` + `&Rules` at page time, so a
/// bare tag suffices — the owned snapshot is only paid where it removes a
/// borrow from the returned iterators.
///
/// `MovementRoutes` covers **both** on-board President movement
/// (`Move::MovePresident`, including Station/Tram teleports) and placing the
/// President from hand (`Move::PlacePresident`) — the two ways the President
/// reaches the board. The exhaustive generator emits both from the single
/// `TurnPhase::Movement` branch and there is no separate place-President phase,
/// so they share one family tag. `MovementRetreat` is the
/// `Move::ReturnPresidentToHand` family, offered from either President location
/// (`from_hand` records which).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveFamily {
    /// Phase A/C additional actions: end-phase, mercenaries, fulfill
    /// order/achievement, University, Textile Mill.
    AdditionalAction,
    /// Placement step: Option A distinct-area k-subsets and Option B
    /// two-in-one-area (with the optional Telegram neighbour).
    Placement,
    /// Movement step, President-onto-board: `MovePresident` routes (President
    /// on board) or `PlacePresident` (President in hand). The lazy enumerator
    /// is 3.1b.
    MovementRoutes,
    /// Movement step, `ReturnPresidentToHand`: the per-area recovery-count
    /// product `∏(count + 1)` — the six-figure family. Carries an owned clone
    /// of the current player's `on_board` counts so it is indexable/sampleable
    /// without touching `GameState`; `from_hand` is true when the President is
    /// already in hand (Option B from hand) rather than on the board.
    MovementRetreat {
        on_board: BTreeMap<AreaId, u8>,
        from_hand: bool,
    },
    /// Area-action step: the six optional-component areas plus production and
    /// the Bank.
    AreaAction,
    /// 5-Power bonus step: take the token or skip.
    FivePower,
    /// Building step: build a shophouse / trading house, or skip.
    Building,
    /// A `PendingChoice` heads the queue: only its resolutions are legal
    /// (mirrors the `get_legal_moves` queue short-circuit).
    ChoiceResolution,
}

/// The move families legal at `state`, mirroring [`get_legal_moves`]'s
/// queue-first + phase dispatch (`legal_moves.rs:9-24`) but yielding family
/// tags rather than moves, in the same order `get_legal_moves` emits their
/// moves.
///
/// Structural by design: it emits the tag(s) the current phase dispatches to
/// and does **not** probe a family for emptiness. A degenerate state whose
/// phase-family happens to be empty still gets its tag — the family's own
/// paging yields the empty set there, so the per-family and union equivalences
/// (`⋃ iter_family == get_legal_moves`) still hold. `Recovery` and `TurnEnd`
/// are automatic/terminal and dispatch to no family.
///
/// `rules` is unused today — the dispatch is a pure function of the turn phase,
/// the pending-choice queue, and the President's location — but is part of the
/// shared `ActionSource` signature the later per-family functions use.
pub fn families(state: &GameState, _rules: &Rules) -> Vec<MoveFamily> {
    // Queue short-circuit (mirrors `legal_moves.rs:9-11`): while a
    // `PendingChoice` heads the queue, the only legal moves are its
    // resolutions, whatever the turn phase.
    if !state.pending_choices.is_empty() {
        return vec![MoveFamily::ChoiceResolution];
    }

    match state.turn_phase {
        TurnPhase::AdditionalActionFirst | TurnPhase::AdditionalActionSecond => {
            vec![MoveFamily::AdditionalAction]
        }
        TurnPhase::Placement => vec![MoveFamily::Placement],
        TurnPhase::Movement => {
            // Both President locations emit a routes family (PlacePresident
            // from hand, MovePresident on board) followed by the retreat
            // family — the same order `generate_movement_moves_with_tech`
            // uses (`legal_moves.rs:254-373`).
            let p = state.current_player();
            vec![
                MoveFamily::MovementRoutes,
                MoveFamily::MovementRetreat {
                    on_board: p.assistants.on_board.clone(),
                    from_hand: matches!(p.president_location, PresidentLocation::InHand),
                },
            ]
        }
        TurnPhase::AreaAction => vec![MoveFamily::AreaAction],
        TurnPhase::FivePowerBonus => vec![MoveFamily::FivePower],
        TurnPhase::Building => vec![MoveFamily::Building],
        TurnPhase::Recovery | TurnPhase::TurnEnd => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::get_legal_moves;

    /// Minimal 2-player state (mirrors `legal_moves::tests::create_test_state`)
    /// with a two-area board — enough for `get_legal_moves` to produce real
    /// moves to cross-check the family tags against. Defaults to `Placement`.
    fn base_state() -> GameState {
        let config = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));

        let mut areas = BTreeMap::new();
        areas.insert(AreaId::new(0), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(AreaId::new(1), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });

        let board_layout = BoardLayout {
            areas,
            adjacency: BTreeMap::new(),
            canal_crossings: vec![],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };

        let players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];

        GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::Placement,
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
        }
    }

    fn at_phase(phase: TurnPhase) -> GameState {
        let mut s = base_state();
        s.turn_phase = phase;
        s
    }

    /// Which `MoveFamily` a given `Move` belongs to, as a discriminant so the
    /// payload-carrying `MovementRetreat` compares by variant only. Exhaustive
    /// on purpose (no `_` arm): a new `Move` variant must be classified here,
    /// or this test stops compiling.
    fn family_kind(mv: &Move) -> std::mem::Discriminant<MoveFamily> {
        use std::mem::discriminant as d;
        match mv {
            Move::UseMercenary { .. }
            | Move::UseUniversity
            | Move::UseSpinningMill
            | Move::FulfillAchievement { .. }
            | Move::FulfillOrder { .. }
            | Move::EndAdditionalPhase => d(&MoveFamily::AdditionalAction),

            Move::PlaceThreeAreas { .. } | Move::PlaceTwoSameArea { .. } => {
                d(&MoveFamily::Placement)
            }

            Move::MovePresident { .. } | Move::PlacePresident { .. } => {
                d(&MoveFamily::MovementRoutes)
            }

            Move::ReturnPresidentToHand { .. } => {
                d(&MoveFamily::MovementRetreat { on_board: BTreeMap::new(), from_hand: false })
            }

            Move::ExecuteAreaAction { .. } => d(&MoveFamily::AreaAction),

            Move::Take5PowerToken | Move::Skip5PowerBonus => d(&MoveFamily::FivePower),

            Move::BuildShophouse { .. } | Move::BuildTradingHouse | Move::SkipBuilding => {
                d(&MoveFamily::Building)
            }

            Move::ChooseAnyGood { .. }
            | Move::ChooseWarehouseTake { .. }
            | Move::DeclineWarehouseTake
            | Move::ChooseAssistantMove { .. }
            | Move::DeclineAssistantMove
            | Move::ChooseChurchSpace { .. }
            | Move::ChooseCustomsSpace { .. }
            | Move::ChooseRecoveryLeave { .. }
            | Move::ChooseStationPlace { .. } => d(&MoveFamily::ChoiceResolution),
        }
    }

    #[test]
    fn families_match_phase_dispatch() {
        let rules = Rules::empty_for_tests();

        // Single-family phases.
        assert_eq!(families(&at_phase(TurnPhase::Placement), &rules), vec![MoveFamily::Placement]);
        assert_eq!(
            families(&at_phase(TurnPhase::AdditionalActionFirst), &rules),
            vec![MoveFamily::AdditionalAction],
        );
        assert_eq!(
            families(&at_phase(TurnPhase::AdditionalActionSecond), &rules),
            vec![MoveFamily::AdditionalAction],
        );
        assert_eq!(families(&at_phase(TurnPhase::AreaAction), &rules), vec![MoveFamily::AreaAction]);
        assert_eq!(
            families(&at_phase(TurnPhase::FivePowerBonus), &rules),
            vec![MoveFamily::FivePower],
        );
        assert_eq!(families(&at_phase(TurnPhase::Building), &rules), vec![MoveFamily::Building]);

        // Automatic / terminal phases dispatch to no family.
        assert_eq!(families(&at_phase(TurnPhase::Recovery), &rules), vec![]);
        assert_eq!(families(&at_phase(TurnPhase::TurnEnd), &rules), vec![]);
    }

    #[test]
    fn movement_yields_routes_then_retreat_with_snapshot() {
        let rules = Rules::empty_for_tests();

        let mut on_board = BTreeMap::new();
        on_board.insert(AreaId::new(0), 2);
        on_board.insert(AreaId::new(1), 1);

        // President in hand: routes family (PlacePresident) then retreat with
        // from_hand=true, carrying an exact clone of on_board.
        let mut in_hand = at_phase(TurnPhase::Movement);
        in_hand.players[0].president_location = PresidentLocation::InHand;
        in_hand.players[0].assistants.on_board = on_board.clone();
        assert_eq!(
            families(&in_hand, &rules),
            vec![
                MoveFamily::MovementRoutes,
                MoveFamily::MovementRetreat { on_board: on_board.clone(), from_hand: true },
            ],
        );

        // President on board: same routes-then-retreat order and snapshot, but
        // from_hand=false.
        let mut on_board_state = in_hand.clone();
        on_board_state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        assert_eq!(
            families(&on_board_state, &rules),
            vec![
                MoveFamily::MovementRoutes,
                MoveFamily::MovementRetreat { on_board, from_hand: false },
            ],
        );
    }

    #[test]
    fn pending_choice_short_circuits_dispatch() {
        let rules = Rules::empty_for_tests();

        // A pending choice overrides the turn phase entirely (mirrors
        // legal_moves.rs:9-11): at Placement, only ChoiceResolution.
        let mut at_placement = at_phase(TurnPhase::Placement);
        at_placement.pending_choices.push(PendingChoice::AnyGood);
        assert_eq!(families(&at_placement, &rules), vec![MoveFamily::ChoiceResolution]);

        // And it wins over Movement too (which would otherwise be two families).
        let mut at_movement = at_phase(TurnPhase::Movement);
        at_movement.pending_choices.push(PendingChoice::AnyGood);
        assert_eq!(families(&at_movement, &rules), vec![MoveFamily::ChoiceResolution]);
    }

    #[test]
    fn families_cover_every_legal_move() {
        let rules = Rules::empty_for_tests();

        // Each move get_legal_moves emits must belong to a family families()
        // lists — the union of the factored families reproduces the whole
        // legal list. States chosen so each is non-empty and panic-free
        // (Movement OnBoard is pinned structurally in the test above, so it is
        // not re-driven through the route generator here).
        let mut cases: Vec<GameState> = Vec::new();

        // Placement: real placement moves.
        cases.push(at_phase(TurnPhase::Placement));

        // Additional action: EndAdditionalPhase is always offered.
        cases.push(at_phase(TurnPhase::AdditionalActionFirst));

        // Movement, President in hand with an own assistant on area 0 -> both a
        // PlacePresident (MovementRoutes) and ReturnPresidentToHand
        // (MovementRetreat) are emitted.
        let mut movement = at_phase(TurnPhase::Movement);
        movement.players[0].president_location = PresidentLocation::InHand;
        movement.players[0].assistants.on_board.insert(AreaId::new(0), 1);
        cases.push(movement);

        // Pending choice -> ChooseAnyGood resolutions (ChoiceResolution).
        let mut choice = at_phase(TurnPhase::Placement);
        choice.pending_choices.push(PendingChoice::AnyGood);
        cases.push(choice);

        for state in &cases {
            let legal = get_legal_moves(state, &rules);
            assert!(
                !legal.is_empty(),
                "case at {:?} must have legal moves to cross-check",
                state.turn_phase,
            );
            let fams = families(state, &rules);
            let fam_kinds: Vec<_> = fams.iter().map(std::mem::discriminant).collect();
            for mv in &legal {
                assert!(
                    fam_kinds.contains(&family_kind(mv)),
                    "legal move {mv:?} at {:?} maps to no family in {fams:?}",
                    state.turn_phase,
                );
            }
        }
    }
}
