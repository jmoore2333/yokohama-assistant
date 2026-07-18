//! Legal move generation for each turn phase.

use crate::types::*;
use crate::game::*;
use crate::game::actions::*;

/// Generate all legal moves for the current game state.
pub fn get_legal_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    match state.turn_phase {
        TurnPhase::AdditionalActionFirst | TurnPhase::AdditionalActionSecond => {
            generate_additional_action_moves(state, rules)
        }
        TurnPhase::Placement => generate_placement_moves_with_tech(state, Some(&rules.techs)),
        TurnPhase::Movement => generate_movement_moves_with_tech(state, Some(&rules.techs)),
        TurnPhase::AreaAction => generate_area_action_moves(state, rules),
        TurnPhase::FivePowerBonus => generate_five_power_moves(state),
        TurnPhase::Building => generate_building_moves(state),
        TurnPhase::Recovery => vec![], // Automatic, no moves
        TurnPhase::TurnEnd => vec![],
    }
}

fn generate_additional_action_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    let mut moves = Vec::new();
    let player = state.current_player;
    let p = state.player(player);

    // Always can end the phase
    moves.push(Move::EndAdditionalPhase);

    // Foreign Mercenaries (once per turn)
    if !state.additional_action_state.mercenary_used {
        for (&country, &count) in &p.mercenaries.available {
            if count > 0 {
                // Find valid target areas
                for (&area, &assistant_count) in &p.assistants.on_board {
                    if assistant_count > 0 {
                        // Must not have our president
                        let has_our_president =
                            p.president_location == PresidentLocation::OnBoard(area);
                        if !has_our_president {
                            moves.push(Move::UseMercenary {
                                country,
                                target_area: area,
                            });
                        }
                    }
                }
            }
        }
    }

    // Fulfill Orders — only those the player can actually pay for
    // (legal == applicable; apply_fulfill_order re-checks and must agree).
    for &order_id in &p.order_cards_in_hand {
        if can_fulfill_order(state, player, order_id, &rules.orders) {
            moves.push(Move::FulfillOrder { order_id });
        }
    }

    // Fulfill Achievements — can_claim_achievement covers already-claimed,
    // assistant-in-hand, and the per-type requirement.
    for achievement in &state.achievements {
        if can_claim_achievement(state, player, achievement.achievement_type) {
            moves.push(Move::FulfillAchievement {
                achievement: achievement.achievement_type,
            });
        }
    }

    moves
}

/// Calculate placement cost for a set of areas.
/// `placements` lists one entry per assistant placed (distinct areas for
/// Option A; a single-element slice per area-toll call for Option B/Telegram
/// bonus). KB (01-rules-reference.md:97): "If any President pawn(s) occupy
/// the target area, pay 1 Yen to each President's owner per assistant
/// placed" — so cost sums 1 Yen per (area, opponent-president) pair. Two
/// different opponents can both have a President in the same area (nothing
/// forces either to leave when the other arrives), in which case both are
/// owed a fee. Callers that credit these fees to owners (`apply_move.rs`'s
/// `credit_placement_fees`) must mirror this exact pairing so the payer's
/// deduction and the sum of owner credits always match.
pub(crate) fn calculate_placement_cost(
    state: &GameState,
    player: PlayerId,
    placements: &[AreaId],
) -> u32 {
    let mut cost = 0;
    for &area in placements {
        for other in &state.players {
            if other.id != player && other.president_location == PresidentLocation::OnBoard(area) {
                cost += 1;
            }
        }
    }
    cost
}

fn generate_placement_moves_with_tech(state: &GameState, tech_registry: Option<&TechRegistry>) -> Vec<Move> {
    let mut moves = Vec::new();
    let player = state.current_player;
    let p = state.player(player);
    let player_money = p.money;

    if p.assistants.in_hand == 0 {
        // Must place 0 (degenerate case)
        moves.push(Move::PlaceThreeAreas { placements: vec![] });
        return moves;
    }

    // Check for Newspaper technology (allows 4 areas instead of 3)
    let has_newspaper = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Newspaper).unwrap_or(false)
        })
    });
    let max_areas = if has_newspaper { 4 } else { 3 };

    // Get all placeable areas (not Canal)
    let placeable_areas: Vec<AreaId> = state
        .board_layout
        .areas
        .iter()
        .filter(|(_, config)| config.area_type.allows_placement())
        .map(|(&id, _)| id)
        .collect();

    // Option A: 1 assistant in each of 1-N different areas (N = 3 or 4 with Newspaper)
    // Note: Placement costs 1 Yen per area with opponent President
    for i in 0..=placeable_areas.len().min(max_areas).min(p.assistants.in_hand as usize) {
        if i == 0 {
            // Empty placement is always free
            moves.push(Move::PlaceThreeAreas { placements: vec![] });
        } else if i == 1 {
            for &area in &placeable_areas {
                let placements = vec![area];
                let cost = calculate_placement_cost(state, player, &placements);
                if player_money >= cost {
                    moves.push(Move::PlaceThreeAreas { placements });
                }
            }
        } else if i == 2 && p.assistants.in_hand >= 2 {
            for (j, &area1) in placeable_areas.iter().enumerate() {
                for &area2 in &placeable_areas[j + 1..] {
                    let placements = vec![area1, area2];
                    let cost = calculate_placement_cost(state, player, &placements);
                    if player_money >= cost {
                        moves.push(Move::PlaceThreeAreas { placements });
                    }
                }
            }
        } else if i == 3 && p.assistants.in_hand >= 3 {
            for (j, &area1) in placeable_areas.iter().enumerate() {
                for (k, &area2) in placeable_areas[j + 1..].iter().enumerate() {
                    for &area3 in &placeable_areas[j + k + 2..] {
                        let placements = vec![area1, area2, area3];
                        let cost = calculate_placement_cost(state, player, &placements);
                        if player_money >= cost {
                            moves.push(Move::PlaceThreeAreas { placements });
                        }
                    }
                }
            }
        } else if i == 4 && p.assistants.in_hand >= 4 && has_newspaper {
            // 4-area placement with Newspaper
            for (j, &area1) in placeable_areas.iter().enumerate() {
                for (k, &area2) in placeable_areas[j + 1..].iter().enumerate() {
                    for (l, &area3) in placeable_areas[j + k + 2..].iter().enumerate() {
                        for &area4 in &placeable_areas[j + k + l + 3..] {
                            let placements = vec![area1, area2, area3, area4];
                            let cost = calculate_placement_cost(state, player, &placements);
                            if player_money >= cost {
                                moves.push(Move::PlaceThreeAreas { placements });
                            }
                        }
                    }
                }
            }
        }
    }

    // Option B: 2 assistants in 1 area (if have at least 2)
    // Cost is 1 Yen if opponent President is in the area
    // With Telegram tech and 3+ assistants, may also place 1 in adjacent area
    let has_telegram = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Telegram).unwrap_or(false)
        })
    });

    if p.assistants.in_hand >= 2 {
        for &area in &placeable_areas {
            // 2 assistants land in `area`: price per assistant placed (a
            // 2-element slice), matching apply_placement_same_area/
            // credit_placement_fees exactly (C1 final-review fix).
            let cost = calculate_placement_cost(state, player, &[area, area]);
            if player_money >= cost {
                // Basic move without Telegram bonus
                moves.push(Move::PlaceTwoSameArea { area, adjacent_bonus: None });

                // With Telegram and 3+ assistants, offer adjacent placements
                if has_telegram && p.assistants.in_hand >= 3 {
                    for &adj_area in state.board_layout.adjacent_areas(area) {
                        // Adjacent area must be placeable
                        if let Some(adj_config) = state.board_layout.areas.get(&adj_area) {
                            if adj_config.area_type.allows_placement() {
                                // Check cost for adjacent area too
                                let adj_cost = calculate_placement_cost(state, player, &[adj_area]);
                                if player_money >= cost + adj_cost {
                                    moves.push(Move::PlaceTwoSameArea {
                                        area,
                                        adjacent_bonus: Some(adj_area),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    moves
}

fn generate_movement_moves_with_tech(state: &GameState, tech_registry: Option<&TechRegistry>) -> Vec<Move> {
    let mut moves = Vec::new();
    let player = state.current_player;
    let p = state.player(player);

    match p.president_location {
        PresidentLocation::InHand => {
            // Option A: place president on any Possible Action Area
            for &area in state.board_layout.areas.keys() {
                if is_possible_action_area_with_tech(state, player, area, tech_registry) {
                    moves.push(Move::PlacePresident { destination: area });
                }
            }

            // Option B is always available (rulebook B2 makes the Movement
            // step required with a choice of TWO options, and Option B's
            // recover-assistants clause is not conditioned on the president
            // being on the board). From hand this means "stay in hand,
            // recover any assistants" - and it guarantees the required
            // Movement step can never dead-end when no Possible Action
            // Area exists.
            let areas_with_assistants: Vec<AreaId> = p
                .assistants
                .on_board
                .iter()
                .filter(|(_, &count)| count > 0)
                .map(|(&area, _)| area)
                .collect();
            moves.push(Move::ReturnPresidentToHand {
                recover_assistants_from: areas_with_assistants,
            });
        }
        PresidentLocation::OnBoard(current) => {
            // Move to reachable areas the player can afford: price each path
            // with the same tech-aware cost apply_movement charges
            // (president pass-through fees + canal tolls), so legal ==
            // applicable. One path per destination (BFS order) - a cheaper
            // alternate route to the same destination is not searched for.
            for (dest, path) in find_reachable_areas_with_tech(state, player, current, tech_registry) {
                if calculate_movement_cost_with_tech(state, player, &path, tech_registry)
                    <= p.money
                {
                    moves.push(Move::MovePresident {
                        path,
                        destination: dest,
                    });
                }
            }

            // Station teleport (free move to station area if player has Station tech)
            if let Some(station_area) = can_teleport_to_station_with_tech(state, player, tech_registry) {
                // Only add if not already at station and not already in moves
                if station_area != current && !moves.iter().any(|m| matches!(m, Move::MovePresident { destination, .. } if *destination == station_area)) {
                    moves.push(Move::MovePresident {
                        path: vec![], // Free teleport, no path needed
                        destination: station_area,
                    });
                }
            }

            // Return to hand (emergency exit, always available)
            let areas_with_assistants: Vec<AreaId> = p
                .assistants
                .on_board
                .iter()
                .filter(|(_, &count)| count > 0)
                .map(|(&area, _)| area)
                .collect();

            moves.push(Move::ReturnPresidentToHand {
                recover_assistants_from: areas_with_assistants,
            });
        }
    }

    moves
}

fn generate_area_action_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    let player = state.current_player;
    let p = state.player(player);

    // For mercenary sub-turns, use the mercenary target area instead of president location
    let area = if state.is_mercenary_action {
        match state.mercenary_target_area {
            Some(a) => a,
            None => return vec![], // Should never happen if state is consistent
        }
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => a,
            PresidentLocation::InHand => return vec![],
        }
    };

    let area_config = match state.board_layout.areas.get(&area) {
        Some(c) => c,
        None => return vec![],
    };

    let power = state.current_action_power.unwrap_or(0);

    // Port and Laboratory need the actual board contents (which slots are
    // filled, and by which A/B board) to enumerate real moves, so they're
    // resolved here rather than by the type-only placeholder dispatch below.
    match area_config.area_type {
        AreaType::PortA => generate_port_actions(state, player, power, &state.port_state.port_a),
        AreaType::PortB => match &state.port_state.port_b {
            Some(port) => generate_port_actions(state, player, power, port),
            // Board/layout mismatch (a PortB area exists but its board was
            // never initialized) is unreachable by construction -- layout
            // and board state both derive from the same GameConfig. The
            // empty vec here is DELIBERATE, not a bug: an empty legal-move
            // list in a non-terminal decision phase is Task 18's designed
            // fuzz corruption alarm (orchestrator adjudication, Task 16
            // review) -- emitting Skip instead would let a corrupted board
            // state fuzz silently to completion rather than tripping it.
            None => vec![],
        },
        AreaType::LaboratoryA => {
            generate_lab_actions(state, player, power, &state.lab_state.lab_a, rules)
        }
        AreaType::LaboratoryB => match &state.lab_state.lab_b {
            Some(lab) => generate_lab_actions(state, player, power, lab, rules),
            // Same fallback rationale as PortB above: unreachable by
            // construction, empty vec is the designed fuzz corruption
            // alarm -- do NOT paper over with Skip.
            None => vec![],
        },
        AreaType::Church => generate_church_actions(state, player, power),
        AreaType::Customs => generate_customs_actions(state, player, power),
        AreaType::EmploymentAgency => generate_employment_agency_actions(state, player, power),
        AreaType::Chinatown => generate_chinatown_actions(state, player, power, rules),
        AreaType::CopperMine => vec![Move::ExecuteAreaAction { action: AreaAction::ProduceCopper }],
        AreaType::SilkMill => vec![Move::ExecuteAreaAction { action: AreaAction::ProduceSilk }],
        AreaType::TeaPlantation => vec![Move::ExecuteAreaAction { action: AreaAction::ProduceTea }],
        AreaType::FishingGround => vec![Move::ExecuteAreaAction { action: AreaAction::ProduceSeafood }],
        AreaType::Bank => vec![Move::ExecuteAreaAction { action: AreaAction::Bank }],
        // Canal can never be an acting area: no assistants may ever be
        // placed there (`allows_placement` excludes it) so it's never a
        // Possible Action Area, and mercenaries can't select it either
        // (Task 16 review, orchestrator-adjudicated) -- unreachable by
        // construction, same fuzz-corruption-alarm rationale as above.
        AreaType::Canal => vec![],
    }
}

/// Enumerate legal Port moves at `power` against the resolved acting board
/// (Port A or B -- the caller has already resolved which one from the
/// acting area, mirroring `execute_port_action`'s own resolution). Accessible
/// cards are the `Some` slots within power-accessible zones. Singles need
/// hand length + 1 <= 3; pairs need hand length + 2 <= 3 and offer one move
/// per affordable payment (TwoYen iff money >= 2, OneImportedGood iff
/// imported goods >= 1). Skip is always legal and always offered (Task 16 /
/// decision-log optionality ruling).
fn generate_port_actions(state: &GameState, player: PlayerId, power: u8, port: &PortBoard) -> Vec<Move> {
    let p = state.player(player);
    let hand_len = p.order_cards_in_hand.len();
    let accessible_zones = port_accessible_zones(power);

    // Accessible cards, in board slot order (canonical order for pairs below).
    let accessible: Vec<OrderCardId> = port.slots.iter().enumerate()
        .filter_map(|(i, slot)| {
            if accessible_zones.contains(&PortBoard::zone_of(i)) { *slot } else { None }
        })
        .collect();

    let mut moves = Vec::new();

    if hand_len < 3 {
        for &card in &accessible {
            moves.push(Move::ExecuteAreaAction {
                action: AreaAction::Port { cards_taken: vec![card], second_card_payment: None },
            });
        }
    }

    if hand_len + 2 <= 3 {
        for i in 0..accessible.len() {
            let first = accessible[i];
            for &second in &accessible[i + 1..] {
                if p.money >= 2 {
                    moves.push(Move::ExecuteAreaAction {
                        action: AreaAction::Port {
                            cards_taken: vec![first, second],
                            second_card_payment: Some(PortPayment::TwoYen),
                        },
                    });
                }
                if p.imported_goods >= 1 {
                    moves.push(Move::ExecuteAreaAction {
                        action: AreaAction::Port {
                            cards_taken: vec![first, second],
                            second_card_payment: Some(PortPayment::OneImportedGood),
                        },
                    });
                }
            }
        }
    }

    moves.push(Move::ExecuteAreaAction { action: AreaAction::Skip });
    moves
}

/// Enumerate legal Laboratory moves at `power` against the resolved acting
/// board (Lab A or B -- mirrors `execute_lab_action`'s own resolution). For
/// each filled slot, emits every exact-cover payment mix for the Industry
/// Value deficit (no overpays: `extra_imported = k`, `extra_yen =
/// 2*(deficit-k)` for every affordable `k` in `0..=deficit`), forking into
/// affordable `fifth_tech_payment` variants once the player owns 4+
/// technologies (a mix with neither payment affordable emits nothing). Skip
/// is always legal and always offered (Task 16 / decision-log optionality
/// ruling).
fn generate_lab_actions(state: &GameState, player: PlayerId, power: u8, lab: &LabBoard, rules: &Rules) -> Vec<Move> {
    let p = state.player(player);
    let needs_fifth_payment = p.technologies.len() >= 4;

    let mut moves = Vec::new();

    for (slot_index, slot) in lab.slots.iter().enumerate() {
        let Some(card_id) = *slot else { continue };
        let Some(tech_card) = rules.techs.get(card_id) else { continue };

        let required = tech_card.industry_cost + lab_position_surcharge(slot_index);
        let deficit = required.saturating_sub(power.min(5));

        for k in 0..=deficit {
            if (k as u32) > p.imported_goods {
                continue;
            }
            let extra_yen = 2 * (deficit - k);
            if (extra_yen as u32) > p.money {
                continue;
            }

            if needs_fifth_payment {
                if p.money >= extra_yen as u32 + 2 {
                    moves.push(Move::ExecuteAreaAction {
                        action: AreaAction::Laboratory {
                            card_id, extra_yen, extra_imported: k, fifth_tech_payment: Some(true),
                        },
                    });
                }
                if p.imported_goods > k as u32 {
                    moves.push(Move::ExecuteAreaAction {
                        action: AreaAction::Laboratory {
                            card_id, extra_yen, extra_imported: k, fifth_tech_payment: Some(false),
                        },
                    });
                }
            } else {
                moves.push(Move::ExecuteAreaAction {
                    action: AreaAction::Laboratory {
                        card_id, extra_yen, extra_imported: k, fifth_tech_payment: None,
                    },
                });
            }
        }
    }

    moves.push(Move::ExecuteAreaAction { action: AreaAction::Skip });
    moves
}

/// Enumerate legal Church moves at `power`: every one of the 64 goods-flag
/// combinations the player can both AFFORD (stock/assistant, per
/// `validate_church_action`) and that the executor would actually ACCEPT
/// (total Faith >= 4 with an eligible empty space, per
/// `highest_eligible_church_space`) -- reusing both pure executor helpers
/// directly so enumeration can never drift from `execute_church_action`'s
/// own acceptance criteria. Skip is always legal and always offered (Task
/// 16/17 decision-log optionality ruling).
fn generate_church_actions(state: &GameState, player: PlayerId, power: u8) -> Vec<Move> {
    let mut moves = Vec::new();

    for mask in 0u8..64 {
        let copper = mask & 1 != 0;
        let silk = mask & 2 != 0;
        let tea = mask & 4 != 0;
        let seafood = mask & 8 != 0;
        let yen = mask & 16 != 0;
        let imported_good = mask & 32 != 0;

        if validate_church_action(state, player, copper, silk, tea, seafood, yen, imported_good).is_err() {
            continue;
        }

        let total_faith = calculate_church_faith(power, copper, silk, tea, seafood, yen, imported_good);
        if highest_eligible_church_space(&state.church_track, total_faith, state.config.player_count).is_none() {
            continue;
        }

        moves.push(Move::ExecuteAreaAction {
            action: AreaAction::Church { copper, silk, tea, seafood, yen, imported_good },
        });
    }

    moves.push(Move::ExecuteAreaAction { action: AreaAction::Skip });
    moves
}

/// Enumerate legal Customs moves at `power`: one move per payable N in
/// 1..=`customs_max_imported_goods(power)` for which the player both can
/// legally pay it (`validate_customs_action`, which also gates on assistant
/// availability) and an eligible empty track space exists
/// (`eligible_customs_space_for_cost`) -- reusing both pure executor
/// helpers directly so enumeration can never drift from
/// `execute_customs_action`'s own acceptance criteria. Skip is always
/// legal and always offered (Task 16/17 decision-log optionality ruling).
fn generate_customs_actions(state: &GameState, player: PlayerId, power: u8) -> Vec<Move> {
    let mut moves = Vec::new();

    for n in 1..=customs_max_imported_goods(power) {
        if validate_customs_action(state, player, n, power).is_ok()
            && eligible_customs_space_for_cost(&state.customs_track, n, state.config.player_count).is_some()
        {
            moves.push(Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid: n } });
        }
    }

    moves.push(Move::ExecuteAreaAction { action: AreaAction::Skip });
    moves
}

/// Enumerate legal Employment Agency moves at `power`: every
/// (assistants, shophouses, trading_houses) triple with 1 <= total <=
/// `employment_agency_tokens_allowed(power)`, each component within
/// warehouse stock, and the total cost affordable -- mirrors
/// `apply_employment_agency`'s exact cost computation (apply_move.rs:
/// assistants free, shophouses 2 Yen each, trading houses at the 4/5/6/7
/// already-built progression) so a change there can't silently drift from
/// this enumeration. Skip is always legal and always offered (Task 16/17
/// decision-log optionality ruling).
fn generate_employment_agency_actions(state: &GameState, player: PlayerId, power: u8) -> Vec<Move> {
    let p = state.player(player);
    let allowed = employment_agency_tokens_allowed(power) as u32;
    let already_taken = p.trading_houses.in_hand as usize + p.trading_houses.total_built();

    let mut moves = Vec::new();

    for a in 0..=p.assistants.in_warehouse {
        for s in 0..=p.shophouses.in_warehouse {
            for t in 0..=p.trading_houses.in_warehouse {
                let total = a as u32 + s as u32 + t as u32;
                if total == 0 || total > allowed {
                    continue;
                }

                let shophouse_cost = s as u32 * 2;
                let mut th_cost = 0u32;
                for i in 0..t {
                    let th_index = already_taken + i as usize;
                    th_cost += match th_index {
                        0 => 4,
                        1 => 5,
                        2 => 6,
                        _ => 7,
                    };
                }

                if p.money < shophouse_cost + th_cost {
                    continue;
                }

                moves.push(Move::ExecuteAreaAction {
                    action: AreaAction::EmploymentAgency { assistants: a, shophouses: s, trading_houses: t },
                });
            }
        }
    }

    moves.push(Move::ExecuteAreaAction { action: AreaAction::Skip });
    moves
}

/// Documented v1 cap on the length of a single Chinatown move's exchange
/// sequence, as enumerated by `generate_chinatown_actions`. The KB rule
/// ("Number of exchanges = Power level", 07-rules-reference.md line 312)
/// is now enforced end-to-end (Task 17b): `apply_area_action`'s Chinatown
/// arm calls `validate_chinatown_exchanges` before any exchange executes,
/// and the generator caps its own BFS depth at
/// `min(power.min(5) as usize, CHINATOWN_ENUM_DEPTH)`. This constant only
/// actually bites at power 5: a legal 5-exchange move exists per the rule
/// there but is deliberately NOT enumerated (a plan-sanctioned v1
/// enumeration bound, not a rules gap). At every power below 5, `power`
/// itself is the binding constraint and this constant never limits
/// anything.
pub const CHINATOWN_ENUM_DEPTH: usize = 4;

/// The 9 Chinatown exchange kinds, used to drive the enumeration BFS below.
const ALL_CHINATOWN_EXCHANGES: [ChinatownExchange; 9] = [
    ChinatownExchange::CopperToYen,
    ChinatownExchange::SilkToYen,
    ChinatownExchange::TeaToYen,
    ChinatownExchange::SeafoodToYen,
    ChinatownExchange::YenToCopper,
    ChinatownExchange::YenToSilk,
    ChinatownExchange::YenToTea,
    ChinatownExchange::YenToSeafood,
    ChinatownExchange::YenToImported,
];

/// Simulated resource snapshot for Chinatown BFS enumeration -- mirrors the
/// subset of `PlayerState` fields `apply_chinatown_exchange` reads/writes.
#[derive(Clone, Copy)]
struct ChinatownResources {
    copper: u32,
    silk: u32,
    tea: u32,
    seafood: u32,
    money: u32,
    imported_goods: u32,
}

/// Applies `exchange` to `res` exactly as `apply_chinatown_exchange` would,
/// returning `None` if infeasible (mirrors its cost checks precisely).
/// `mining_bonus` is the player's fixed per-event Mining Technology bonus
/// (owned technologies don't change across a sequence of Chinatown
/// exchanges, so the caller computes this once via `copper_bonus`).
fn try_apply_chinatown_exchange_simulated(
    res: ChinatownResources,
    exchange: ChinatownExchange,
    mining_bonus: u32,
) -> Option<ChinatownResources> {
    let mut r = res;
    match exchange {
        ChinatownExchange::CopperToYen => {
            if r.copper < 1 { return None; }
            r.copper -= 1;
            r.money += 2;
        }
        ChinatownExchange::SilkToYen => {
            if r.silk < 1 { return None; }
            r.silk -= 1;
            r.money += 2;
        }
        ChinatownExchange::TeaToYen => {
            if r.tea < 1 { return None; }
            r.tea -= 1;
            r.money += 1;
        }
        ChinatownExchange::SeafoodToYen => {
            if r.seafood < 1 { return None; }
            r.seafood -= 1;
            r.money += 1;
        }
        ChinatownExchange::YenToCopper => {
            if r.money < 2 { return None; }
            r.money -= 2;
            r.copper += 1 + mining_bonus;
        }
        ChinatownExchange::YenToSilk => {
            if r.money < 2 { return None; }
            r.money -= 2;
            r.silk += 1;
        }
        ChinatownExchange::YenToTea => {
            if r.money < 1 { return None; }
            r.money -= 1;
            r.tea += 1;
        }
        ChinatownExchange::YenToSeafood => {
            if r.money < 1 { return None; }
            r.money -= 1;
            r.seafood += 1;
        }
        ChinatownExchange::YenToImported => {
            if r.money < 4 { return None; }
            r.money -= 4;
            r.imported_goods += 1;
        }
    }
    Some(r)
}

/// Canonical dedupe key for a multiset of exchanges: a fixed-size count per
/// variant (order-independent), so two different valid orderings of the
/// same exchanges collapse to one move.
fn chinatown_exchange_index(e: ChinatownExchange) -> usize {
    match e {
        ChinatownExchange::CopperToYen => 0,
        ChinatownExchange::SilkToYen => 1,
        ChinatownExchange::TeaToYen => 2,
        ChinatownExchange::SeafoodToYen => 3,
        ChinatownExchange::YenToCopper => 4,
        ChinatownExchange::YenToSilk => 5,
        ChinatownExchange::YenToTea => 6,
        ChinatownExchange::YenToSeafood => 7,
        ChinatownExchange::YenToImported => 8,
    }
}

fn chinatown_multiset_key(exchanges: &[ChinatownExchange]) -> [u8; 9] {
    let mut counts = [0u8; 9];
    for &e in exchanges {
        counts[chinatown_exchange_index(e)] += 1;
    }
    counts
}

/// Enumerate legal Chinatown moves via breadth-first search over exchange
/// sequences, up to `min(power.min(5) as usize, CHINATOWN_ENUM_DEPTH)`
/// exchanges (Task 17b: the KB caps the exchange count at the acting
/// power, exactly mirroring `validate_chinatown_exchanges`, which
/// `apply_area_action`'s Chinatown arm now enforces), starting from the
/// player's actual resource tuple and admitting only exchanges valid on
/// the running simulated state (mirroring `apply_chinatown_exchange`'s
/// costs/yields exactly, including the Mining Technology bonus on
/// YenToCopper). Processing strictly by increasing sequence length means
/// the first witness recorded for any reachable multiset is always found
/// at the shortest length that multiset is reachable, though ANY valid
/// order suffices -- the emitted `exchanges` vec is always the literal BFS
/// path that reached it (a valid execution order BY CONSTRUCTION), never
/// independently sorted: sorting a multiset can produce an order that
/// fails validation (e.g. money 0, 1 copper: [CopperToYen, YenToTea] is
/// valid, the reverse is not). One move per distinct multiset. Chinatown
/// places no assistant, so (unlike the other five permissive areas) hand
/// availability never gates it. Skip is always legal and always offered
/// (Task 16/17 decision-log optionality ruling).
fn generate_chinatown_actions(state: &GameState, player: PlayerId, power: u8, rules: &Rules) -> Vec<Move> {
    let p = state.player(player);
    let mining_bonus = copper_bonus(state, player, 1, &rules.techs) as u32;
    let start = ChinatownResources {
        copper: p.copper,
        silk: p.silk,
        tea: p.tea,
        seafood: p.seafood,
        money: p.money,
        imported_goods: p.imported_goods,
    };
    let depth = (power.min(5) as usize).min(CHINATOWN_ENUM_DEPTH);

    let mut moves = Vec::new();
    let mut seen: std::collections::HashSet<[u8; 9]> = std::collections::HashSet::new();
    let mut frontier: Vec<(ChinatownResources, Vec<ChinatownExchange>)> = vec![(start, Vec::new())];

    for _ in 0..depth {
        let mut next_frontier = Vec::new();
        for (res, path) in &frontier {
            for &exchange in &ALL_CHINATOWN_EXCHANGES {
                if let Some(next_res) = try_apply_chinatown_exchange_simulated(*res, exchange, mining_bonus) {
                    let mut next_path = path.clone();
                    next_path.push(exchange);
                    if seen.insert(chinatown_multiset_key(&next_path)) {
                        moves.push(Move::ExecuteAreaAction {
                            action: AreaAction::Chinatown { exchanges: next_path.clone() },
                        });
                    }
                    next_frontier.push((next_res, next_path));
                }
            }
        }
        frontier = next_frontier;
    }

    moves.push(Move::ExecuteAreaAction { action: AreaAction::Skip });
    moves
}

fn generate_five_power_moves(state: &GameState) -> Vec<Move> {
    let player = state.current_player;
    let p = state.player(player);

    // Check if power was 5 and token is still there
    if state.current_action_power == Some(5) {
        // For mercenary sub-turns, use the mercenary target area
        let area = if state.is_mercenary_action {
            state.mercenary_target_area
        } else {
            match p.president_location {
                PresidentLocation::OnBoard(a) => Some(a),
                PresidentLocation::InHand => None,
            }
        };

        if let Some(area) = area {
            if let Some(Some(_)) = state.board_layout.five_power_tokens.get(&area) {
                return vec![Move::Take5PowerToken, Move::Skip5PowerBonus];
            }
        }
    }

    vec![Move::Skip5PowerBonus]
}

fn generate_building_moves(state: &GameState) -> Vec<Move> {
    let mut moves = vec![Move::SkipBuilding];
    let player = state.current_player;
    let p = state.player(player);

    // Need 4+ power action
    if state.current_action_power.unwrap_or(0) < 4 {
        return moves;
    }

    // For mercenary sub-turns, use the mercenary target area instead of president location
    let area = if state.is_mercenary_action {
        match state.mercenary_target_area {
            Some(a) => a,
            None => return moves,
        }
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => a,
            PresidentLocation::InHand => return moves,
        }
    };

    let power = state.current_action_power.unwrap_or(0);

    if let Some(site) = state.board_layout.building_sites.get(&area) {
        // Shophouse: requires Power 4+
        if power >= 4 && p.shophouses.in_hand > 0 {
            if let Some(slot) = site.next_shophouse_slot(player) {
                moves.push(Move::BuildShophouse { slot: slot as u8 });
            }
        }

        // Trading House: requires Power 5; built from HAND for free (the
        // cost was paid at the Employment Agency take)
        if power >= 5 && p.trading_houses.in_hand > 0 && site.can_build_trading_house() {
            moves.push(Move::BuildTradingHouse);
        }
    }

    moves
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn create_test_state() -> GameState {
        let config = GameConfig::new(
            2,
            vec![PlayerColor::Red, PlayerColor::Blue],
            PlayerId::new(0),
        );

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
        }
    }

    #[test]
    fn test_placement_moves_with_assistants() {
        let state = create_test_state();
        let moves = generate_placement_moves_with_tech(&state, None);

        // Should have placement options
        assert!(!moves.is_empty());

        // Should include empty placement (0 assistants)
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements } if placements.is_empty())));

        // Should include single area placements
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements } if placements.len() == 1)));

        // Should include Option B (2 in same area) since we have 8 assistants
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceTwoSameArea { .. })));
    }

    #[test]
    fn test_additional_action_always_has_end() {
        let mut state = create_test_state();
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);

        // Should always include EndAdditionalPhase
        assert!(moves.iter().any(|m| matches!(m, Move::EndAdditionalPhase)));
    }

    #[test]
    fn test_building_offers_trading_house_iff_token_in_hand() {
        // Build is free-from-hand (cost was paid at the Employment take):
        // money is irrelevant, a hand token is required.
        let mut state = create_test_state();
        let area = AreaId::new(0);
        state.turn_phase = TurnPhase::Building;
        state.current_action_power = Some(5);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.board_layout.building_sites.insert(area, BuildingSite::new([1, 2, 3, 4], 7));
        let rules = Rules::empty_for_tests();

        state.players[0].money = 100;
        state.players[0].trading_houses.in_hand = 0;
        state.players[0].trading_houses.in_warehouse = 4;
        let moves = get_legal_moves(&state, &rules);
        assert!(
            !moves.iter().any(|m| matches!(m, Move::BuildTradingHouse)),
            "no token in hand: cannot build regardless of money"
        );

        state.players[0].money = 0;
        state.players[0].trading_houses.in_hand = 1;
        state.players[0].trading_houses.in_warehouse = 3;
        let moves = get_legal_moves(&state, &rules);
        assert!(moves.iter().any(|m| matches!(m, Move::BuildTradingHouse)));
    }

    #[test]
    fn test_movement_from_hand_always_offers_option_b() {
        // President in hand and every area holding our assistants also
        // holds an opponent president: no Possible Action Area exists, so
        // Option A (place) has zero targets. The Movement step is required
        // and Option B's recover clause is not conditioned on the president
        // being on the board - staying in hand (recovering any assistants)
        // must be offered, or the phase machine dead-ends (found by fuzz:
        // 3p seed 1, turn 177).
        let mut state = create_test_state();
        state.turn_phase = TurnPhase::Movement;
        let rules = Rules::empty_for_tests();
        state.players[0].president_location = PresidentLocation::InHand;
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);
        state.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));

        let moves = get_legal_moves(&state, &rules);

        assert!(
            !moves.iter().any(|m| matches!(m, Move::PlacePresident { .. })),
            "no possible action area exists"
        );
        assert!(
            moves.iter().any(|m| matches!(m, Move::ReturnPresidentToHand { .. })),
            "Option B (stay in hand, recover assistants) must be offered"
        );
    }

    #[test]
    fn test_enumeration_is_deterministic_across_identical_constructions() {
        // Two states built identically (same seed) must enumerate identical
        // move LISTS - same set AND same order. HashMap iteration order is
        // per-instance random, which breaks replay determinism and
        // minimal-repro fuzz seeds; every map that feeds enumeration is a
        // BTreeMap for exactly this reason.
        let rules = Rules::standard();
        let build = || {
            let config = GameConfig::for_player_count(4);
            let pending = new_game(config, &rules, 7).unwrap();
            let choices: Vec<StartingOrderChoice> =
                (0..4).map(|_| StartingOrderChoice::Keep(0)).collect();
            let state = finalize_setup(pending, &choices).unwrap();
            // Advance into Placement, where enumeration iterates the board.
            apply_move(&state, &Move::EndAdditionalPhase, &rules).unwrap()
        };
        let a = build();
        let b = build();
        assert_eq!(get_legal_moves(&a, &rules), get_legal_moves(&b, &rules));
    }

    #[test]
    fn test_movement_moves_priced_by_president_pass_through_fees() {
        let mut state = create_test_state();
        state.turn_phase = TurnPhase::Movement;
        let rules = Rules::empty_for_tests();
        // Chain 0 - 1 - 2: president at 0, opponent president at 1 (1 Yen
        // toll to pass), own assistants at 1 and 2 make 2 reachable.
        state.board_layout.areas.insert(
            AreaId::new(2),
            AreaConfig { area_type: AreaType::SilkMill, grid_position: (0, 2) },
        );
        state.board_layout.adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        state.board_layout.adjacency.insert(AreaId::new(1), vec![AreaId::new(0), AreaId::new(2)]);
        state.board_layout.adjacency.insert(AreaId::new(2), vec![AreaId::new(1)]);
        state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);
        state.players[0].assistants.on_board.insert(AreaId::new(2), 1);
        state.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));

        state.players[0].money = 0;
        let moves = get_legal_moves(&state, &rules);
        assert!(
            !moves.iter().any(|m| matches!(m, Move::MovePresident { destination, .. } if *destination == AreaId::new(2))),
            "unaffordable pass-through move must not be offered"
        );

        state.players[0].money = 1;
        let moves = get_legal_moves(&state, &rules);
        assert!(moves
            .iter()
            .any(|m| matches!(m, Move::MovePresident { destination, .. } if *destination == AreaId::new(2))));
    }

    fn postal_system_tech_registry(id: TechCardId) -> TechRegistry {
        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(
            TechCard {
                id,
                name: "Postal System".to_string(),
                industry_cost: 3,
                country: Country::English,
                effect: TechEffect::PostalSystem,
            },
            "tech_postal_system",
        );
        tech_registry
    }

    #[test]
    fn test_postal_system_allows_stopping_in_opponent_occupied_area() {
        // KB 08-tech-cards.yaml tech_postal_system: "Movement Step: You may
        // place your President in an area occupied by another President."
        // President at 0, own assistant at 1, opponent president ALSO at 1.
        let mut state = create_test_state();
        state.turn_phase = TurnPhase::Movement;
        state.board_layout.adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        state.board_layout.adjacency.insert(AreaId::new(1), vec![AreaId::new(0)]);
        state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);
        state.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));

        let rules = Rules::empty_for_tests();
        let moves = get_legal_moves(&state, &rules);
        assert!(
            !moves.iter().any(|m| matches!(m, Move::MovePresident { destination, .. } if *destination == AreaId::new(1))),
            "without Postal System, an opponent-occupied area is not a Possible Action Area"
        );

        state.players[0].technologies.push(TechCardId(200));
        let mut rules = Rules::empty_for_tests();
        rules.techs = postal_system_tech_registry(TechCardId(200));

        let moves = get_legal_moves(&state, &rules);
        let mv = moves
            .iter()
            .find(|m| matches!(m, Move::MovePresident { destination, .. } if *destination == AreaId::new(1)))
            .expect("Postal System owner should be offered a stop in the opponent-occupied area")
            .clone();

        let result = apply_move(&state, &mv, &rules);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().players[0].president_location, PresidentLocation::OnBoard(AreaId::new(1)));
    }

    #[test]
    fn test_postal_system_allows_placement_from_hand_into_opponent_occupied_area() {
        let mut state = create_test_state();
        state.turn_phase = TurnPhase::Movement;
        state.players[0].president_location = PresidentLocation::InHand;
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);
        state.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));

        let rules = Rules::empty_for_tests();
        let moves = get_legal_moves(&state, &rules);
        assert!(
            !moves.iter().any(|m| matches!(m, Move::PlacePresident { destination } if *destination == AreaId::new(1))),
            "without Postal System, cannot place president into an opponent-occupied area"
        );

        state.players[0].technologies.push(TechCardId(201));
        let mut rules = Rules::empty_for_tests();
        rules.techs = postal_system_tech_registry(TechCardId(201));

        let moves = get_legal_moves(&state, &rules);
        let mv = moves
            .iter()
            .find(|m| matches!(m, Move::PlacePresident { destination } if *destination == AreaId::new(1)))
            .expect("Postal System owner should be able to place president into the opponent-occupied area")
            .clone();

        let result = apply_move(&state, &mv, &rules);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().players[0].president_location, PresidentLocation::OnBoard(AreaId::new(1)));
    }

    #[test]
    fn test_postal_system_does_not_waive_pass_through_fees() {
        // Postal System's KB text is purely about stopping rights; the
        // separate pass-through toll (calculate_movement_cost_full) checks
        // only Stagecoach, so it must still apply unchanged.
        let mut state = create_test_state();
        state.turn_phase = TurnPhase::Movement;
        state.board_layout.areas.insert(
            AreaId::new(2),
            AreaConfig { area_type: AreaType::SilkMill, grid_position: (0, 2) },
        );
        state.board_layout.adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        state.board_layout.adjacency.insert(AreaId::new(1), vec![AreaId::new(0), AreaId::new(2)]);
        state.board_layout.adjacency.insert(AreaId::new(2), vec![AreaId::new(1)]);
        state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);
        state.players[0].assistants.on_board.insert(AreaId::new(2), 1);
        state.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));

        state.players[0].technologies.push(TechCardId(202));
        let mut rules = Rules::empty_for_tests();
        rules.techs = postal_system_tech_registry(TechCardId(202));

        state.players[0].money = 0;
        let moves = get_legal_moves(&state, &rules);
        assert!(
            !moves.iter().any(|m| matches!(m, Move::MovePresident { destination, .. } if *destination == AreaId::new(2))),
            "pass-through fee must still apply even though the mover owns Postal System"
        );

        state.players[0].money = 1;
        let moves = get_legal_moves(&state, &rules);
        assert!(moves
            .iter()
            .any(|m| matches!(m, Move::MovePresident { destination, .. } if *destination == AreaId::new(2))));
    }

    #[test]
    fn test_additional_action_offers_only_fulfillable_orders() {
        let mut state = create_test_state();
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        let mut rules = Rules::empty_for_tests();
        rules.orders.insert(
            OrderCard {
                id: OrderCardId(1),
                country: Country::English,
                requirements: TradeGoodRequirement::new(2, 0, 0, 0),
                rewards: OrderRewards { victory_points: 5, ..Default::default() },
            },
            "O1",
        );
        state.players[0].order_cards_in_hand.push(OrderCardId(1));

        state.players[0].copper = 0;
        let moves = get_legal_moves(&state, &rules);
        assert!(
            !moves.iter().any(|m| matches!(m, Move::FulfillOrder { .. })),
            "order with unmet requirements must not be offered"
        );

        state.players[0].copper = 2;
        let moves = get_legal_moves(&state, &rules);
        assert!(moves
            .iter()
            .any(|m| matches!(m, Move::FulfillOrder { order_id } if *order_id == OrderCardId(1))));
    }

    #[test]
    fn test_additional_action_offers_only_claimable_achievements() {
        let mut state = create_test_state();
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        let rules = Rules::empty_for_tests();
        state.achievements = [
            AchievementCard { achievement_type: AchievementType::Copper5, claimed_by: vec![] },
            AchievementCard { achievement_type: AchievementType::Silk6, claimed_by: vec![] },
            AchievementCard { achievement_type: AchievementType::Tea7, claimed_by: vec![] },
        ];
        state.players[0].assistants.in_hand = 1;

        state.players[0].copper = 0;
        let moves = get_legal_moves(&state, &rules);
        assert!(
            !moves.iter().any(|m| matches!(m, Move::FulfillAchievement { .. })),
            "achievement with unmet requirements must not be offered"
        );

        state.players[0].copper = 5;
        let moves = get_legal_moves(&state, &rules);
        assert!(moves.iter().any(|m| matches!(
            m,
            Move::FulfillAchievement { achievement: AchievementType::Copper5 }
        )));

        // Requirements met but no assistant in hand to lock on the card:
        state.players[0].assistants.in_hand = 0;
        let moves = get_legal_moves(&state, &rules);
        assert!(!moves.iter().any(|m| matches!(m, Move::FulfillAchievement { .. })));
    }

    #[test]
    fn test_building_requires_power_4() {
        let mut state = create_test_state();
        state.turn_phase = TurnPhase::Building;
        state.current_action_power = Some(3); // Not enough
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);

        // Should only have Skip
        assert_eq!(moves.len(), 1);
        assert!(matches!(moves[0], Move::SkipBuilding));
    }

    #[test]
    fn test_shophouse_requires_power_4() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        state.turn_phase = TurnPhase::Building;
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].shophouses.in_hand = 1;

        // Add a building site (shophouse rewards: [2, 3, 4, 5], trading house reward: 10)
        state.board_layout.building_sites.insert(area, BuildingSite::new([2, 3, 4, 5], 10));
        let rules = Rules::empty_for_tests();

        // Power 3 - no shophouse option
        state.current_action_power = Some(3);
        let moves = get_legal_moves(&state, &rules);
        assert!(!moves.iter().any(|m| matches!(m, Move::BuildShophouse { .. })));

        // Power 4 - shophouse available
        state.current_action_power = Some(4);
        let moves = get_legal_moves(&state, &rules);
        assert!(moves.iter().any(|m| matches!(m, Move::BuildShophouse { .. })));
    }

    #[test]
    fn test_trading_house_requires_power_5() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        state.turn_phase = TurnPhase::Building;
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].trading_houses.in_hand = 1;

        // Add a building site with no trading house yet
        state.board_layout.building_sites.insert(area, BuildingSite::new([2, 3, 4, 5], 10));
        let rules = Rules::empty_for_tests();

        // Power 4 - trading house NOT available (needs 5)
        state.current_action_power = Some(4);
        let moves = get_legal_moves(&state, &rules);
        assert!(!moves.iter().any(|m| matches!(m, Move::BuildTradingHouse)));

        // Power 5 - trading house available
        state.current_action_power = Some(5);
        let moves = get_legal_moves(&state, &rules);
        assert!(moves.iter().any(|m| matches!(m, Move::BuildTradingHouse)));
    }

    #[test]
    fn test_both_buildings_at_power_5() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        state.turn_phase = TurnPhase::Building;
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].shophouses.in_hand = 1;
        state.players[0].trading_houses.in_hand = 1;

        // Add a building site
        state.board_layout.building_sites.insert(area, BuildingSite::new([2, 3, 4, 5], 10));
        let rules = Rules::empty_for_tests();

        // Power 5 - both available
        state.current_action_power = Some(5);
        let moves = get_legal_moves(&state, &rules);
        assert!(moves.iter().any(|m| matches!(m, Move::BuildShophouse { .. })));
        assert!(moves.iter().any(|m| matches!(m, Move::BuildTradingHouse)));
        assert!(moves.iter().any(|m| matches!(m, Move::SkipBuilding)));
    }

    #[test]
    fn test_place_two_same_area_enumeration_prices_per_assistant_placed() {
        // C1 final-review fix: Option B places 2 assistants in `area`, so 1
        // opponent President there costs 2 Yen (1 per assistant placed), not
        // 1. At money=1 the move must not be offered; at money=2 it must.
        let mut state = create_test_state();
        let bank_area = AreaId::new(0);
        state.players[1].president_location = PresidentLocation::OnBoard(bank_area);
        state.players[0].assistants.in_hand = 2;
        state.turn_phase = TurnPhase::Placement;
        let rules = Rules::empty_for_tests();

        state.players[0].money = 1;
        let moves = get_legal_moves(&state, &rules);
        assert!(
            !moves.iter().any(|m| matches!(m, Move::PlaceTwoSameArea { area, .. } if *area == bank_area)),
            "money 1 must not afford the 2-assistant fee of 2"
        );

        state.players[0].money = 2;
        let moves = get_legal_moves(&state, &rules);
        assert!(
            moves.iter().any(|m| matches!(m, Move::PlaceTwoSameArea { area, .. } if *area == bank_area)),
            "money 2 must afford the 2-assistant fee of 2"
        );
    }

    #[test]
    fn test_placement_cost_opponent_president() {
        let mut state = create_test_state();
        let bank_area = AreaId::new(0);

        // Setup: Opponent President in Bank, player has 0 Yen
        state.players[1].president_location = PresidentLocation::OnBoard(bank_area);
        state.players[0].money = 0;
        state.turn_phase = TurnPhase::Placement;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);

        // Verify: Cannot place in Bank (filtered from legal moves)
        // Empty placement should still be available
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements } if placements.is_empty())));

        // Cannot place in Bank (area 0) since it costs 1 Yen and player has 0
        assert!(!moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 1 && placements[0] == bank_area)));

        // Cannot use PlaceTwoSameArea in Bank either
        assert!(!moves.iter().any(|m| matches!(m, Move::PlaceTwoSameArea { area, .. } if *area == bank_area)));

        // But CAN place in CopperMine (area 1) - no opponent President there
        let mine_area = AreaId::new(1);
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 1 && placements[0] == mine_area)));
    }

    #[test]
    fn test_placement_cost_multiple_areas() {
        let mut state = create_test_state();
        let bank_area = AreaId::new(0);
        let mine_area = AreaId::new(1);

        // Setup: Opponent Presidents in Bank AND CopperMine (need 2nd player for this)
        // Actually, one opponent can only have one president
        // So let's use: Player 1 president in Bank (costs 1 Yen)
        state.players[1].president_location = PresidentLocation::OnBoard(bank_area);
        state.players[0].money = 1;
        state.turn_phase = TurnPhase::Placement;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);

        // With 1 Yen, can place in Bank alone (costs 1)
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 1 && placements[0] == bank_area)));

        // Can also place in CopperMine alone (costs 0)
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 1 && placements[0] == mine_area)));

        // Can place in both Bank AND CopperMine together (costs 1 total - only Bank has opponent)
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 2 && placements.contains(&bank_area) && placements.contains(&mine_area))));

        // Now with 0 Yen, cannot place in Bank
        state.players[0].money = 0;
        let moves = get_legal_moves(&state, &rules);

        // Cannot place in Bank alone
        assert!(!moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 1 && placements[0] == bank_area)));

        // Cannot place in both either (costs 1)
        assert!(!moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 2 && placements.contains(&bank_area))));

        // But can still place in CopperMine alone (costs 0)
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 1 && placements[0] == mine_area)));
    }

    #[test]
    fn test_calculate_placement_cost() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        let bank_area = AreaId::new(0);
        let mine_area = AreaId::new(1);

        // No opponent presidents - cost is 0
        assert_eq!(calculate_placement_cost(&state, player, &[bank_area]), 0);
        assert_eq!(calculate_placement_cost(&state, player, &[mine_area]), 0);
        assert_eq!(calculate_placement_cost(&state, player, &[bank_area, mine_area]), 0);

        // Opponent president in Bank - cost is 1 for Bank, 0 for Mine
        state.players[1].president_location = PresidentLocation::OnBoard(bank_area);
        assert_eq!(calculate_placement_cost(&state, player, &[bank_area]), 1);
        assert_eq!(calculate_placement_cost(&state, player, &[mine_area]), 0);
        assert_eq!(calculate_placement_cost(&state, player, &[bank_area, mine_area]), 1);

        // Empty placement always costs 0
        assert_eq!(calculate_placement_cost(&state, player, &[]), 0);
    }

    #[test]
    fn test_calculate_placement_cost_sums_per_distinct_opponent_sharing_area() {
        // KB (01-rules-reference.md:97): "If any President pawn(s) occupy the
        // target area, pay 1 Yen to each President's owner per assistant
        // placed." Two DIFFERENT opponents' Presidents can both sit in the
        // same area (each owns only one President, but nothing forces either
        // to leave when the other arrives), so the cost must sum per distinct
        // opponent owner in that area, not cap at 1 per area.
        let mut state = create_test_state();
        state.players.push(PlayerState::new(PlayerId::new(2), PlayerColor::Yellow, false));
        let player = PlayerId::new(0);
        let bank_area = AreaId::new(0);

        // Both opponents' Presidents in Bank.
        state.players[1].president_location = PresidentLocation::OnBoard(bank_area);
        state.players[2].president_location = PresidentLocation::OnBoard(bank_area);

        assert_eq!(calculate_placement_cost(&state, player, &[bank_area]), 2);
    }

    #[test]
    fn test_station_teleport_included_in_movement() {
        let mut state = create_test_state();
        let current_area = AreaId::new(0);
        let station_area = AreaId::new(2); // Different area

        // Setup: President on board, Station technology, station placed
        state.turn_phase = TurnPhase::Movement;
        state.players[0].president_location = PresidentLocation::OnBoard(current_area);
        state.players[0].assistants.on_board.insert(station_area, 1);
        state.station_location = Some(station_area);

        // Give player Station tech
        let station_tech = TechCardId(100); // Arbitrary ID for test
        state.players[0].technologies.push(station_tech);

        // Create tech registry with Station tech
        let mut tech_registry = TechRegistry::new();
        let station_card = TechCard {
            id: station_tech,
            name: "Station".to_string(),
            industry_cost: 4,
            country: Country::German,
            effect: TechEffect::Station,
        };
        tech_registry.insert(station_card, "tech_station");

        // Get legal moves with tech awareness
        let mut rules = Rules::empty_for_tests();
        rules.techs = tech_registry;
        let moves = get_legal_moves(&state, &rules);

        // Should include station teleport move (free move to station area)
        let has_teleport = moves.iter().any(|m| matches!(m,
            Move::MovePresident { destination, .. } if *destination == station_area
        ));

        assert!(has_teleport, "Should include station teleport move to station area");
    }

    #[test]
    fn test_telegram_adjacent_bonus() {
        let mut state = create_test_state();
        let main_area = AreaId::new(0); // Bank
        let adjacent_area = AreaId::new(1); // CopperMine

        // Add adjacency: Bank <-> CopperMine
        state.board_layout.adjacency.insert(main_area, vec![adjacent_area]);
        state.board_layout.adjacency.insert(adjacent_area, vec![main_area]);

        state.turn_phase = TurnPhase::Placement;
        state.players[0].assistants.in_hand = 3; // Need 3 for bonus (2 + 1)

        // Without Telegram tech - only basic PlaceTwoSameArea moves
        let rules_no_tech = Rules::empty_for_tests();
        let moves_no_tech = get_legal_moves(&state, &rules_no_tech);
        let has_adjacent_bonus_without_tech = moves_no_tech.iter().any(|m| matches!(m,
            Move::PlaceTwoSameArea { area, adjacent_bonus: Some(_) } if *area == main_area
        ));
        assert!(!has_adjacent_bonus_without_tech, "Without Telegram, should not have adjacent bonus");

        // With Telegram tech - should have adjacent bonus moves
        let telegram_tech = TechCardId(101);
        state.players[0].technologies.push(telegram_tech);

        let mut tech_registry = TechRegistry::new();
        let telegram_card = TechCard {
            id: telegram_tech,
            name: "Telegram".to_string(),
            industry_cost: 3,
            country: Country::American,
            effect: TechEffect::Telegram,
        };
        tech_registry.insert(telegram_card, "tech_telegram");

        let mut rules_with_tech = Rules::empty_for_tests();
        rules_with_tech.techs = tech_registry;
        let moves_with_tech = get_legal_moves(&state, &rules_with_tech);

        // Should have basic move (adjacent_bonus: None)
        let has_basic = moves_with_tech.iter().any(|m| matches!(m,
            Move::PlaceTwoSameArea { area, adjacent_bonus: None } if *area == main_area
        ));
        assert!(has_basic, "Should have basic PlaceTwoSameArea move");

        // Should have at least one adjacent bonus move
        let has_adjacent_bonus = moves_with_tech.iter().any(|m| matches!(m,
            Move::PlaceTwoSameArea { adjacent_bonus: Some(_), .. }
        ));
        assert!(has_adjacent_bonus, "With Telegram, should have adjacent bonus options");
    }

    // --- Task 16: Port & Laboratory enumeration -----------------------

    /// Builds a 6-slot PortBoard with the given card ids seeded left to
    /// right (index 0..5) -- mirrors `actions::cards::tests::seeded_port`,
    /// duplicated locally since that helper is private to its own module.
    fn seeded_port(ids: [u8; 6]) -> PortBoard {
        PortBoard { slots: ids.map(|id| Some(OrderCardId(id))) }
    }

    /// Places a PortA area on the board, seeds it, puts the president
    /// there, and enters the AreaAction phase at the given power.
    fn create_port_area_state(power: u8, port: PortBoard) -> GameState {
        let mut state = create_test_state();
        let port_area = AreaId::new(5);
        state.board_layout.areas.insert(port_area, AreaConfig {
            area_type: AreaType::PortA,
            grid_position: (0, 5),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(port_area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(power);
        state.port_state.port_a = port;
        state
    }

    /// Places a LaboratoryA area on the board, puts the president there,
    /// and enters the AreaAction phase at the given power (slots left empty
    /// for the caller to fill).
    fn create_lab_area_state(power: u8) -> GameState {
        let mut state = create_test_state();
        let lab_area = AreaId::new(5);
        state.board_layout.areas.insert(lab_area, AreaConfig {
            area_type: AreaType::LaboratoryA,
            grid_position: (0, 5),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(lab_area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(power);
        state
    }

    fn make_test_tech_card(id: u8, cost: u8) -> TechCard {
        TechCard {
            id: TechCardId(id),
            name: "Test Tech".to_string(),
            country: Country::English,
            industry_cost: cost,
            effect: TechEffect::Tram,
        }
    }

    /// Task 16 "every-move-applies" property: every move a generator emits
    /// must independently pass `apply_move`. The fuzz suite (Task 18)
    /// enforces this globally; this is the focused per-test version.
    fn assert_all_moves_apply(state: &GameState, moves: &[Move], rules: &Rules) {
        for m in moves {
            let result = apply_move(state, m, rules);
            assert!(result.is_ok(), "generated move failed to apply: {m:?} ({result:?})");
        }
    }

    #[test]
    fn port_enumeration_power_4_all_slots_singles_pairs_and_skip() {
        // Power 4, all 6 slots filled, hand empty, money >= 2, imports >= 1:
        // 6 singles + C(6,2)=15 pairs x 2 payments + 1 Skip = 37.
        let mut state = create_port_area_state(4, seeded_port([1, 2, 3, 4, 5, 6]));
        state.players[0].money = 2;
        state.players[0].imported_goods = 1;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 37);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn port_enumeration_power_2_low_zone_only() {
        // Power 2 -> only Low zone (slots 0-1) accessible: 2 singles + 1
        // pair x 2 payments + 1 Skip = 5.
        let mut state = create_port_area_state(2, seeded_port([1, 2, 3, 4, 5, 6]));
        state.players[0].money = 2;
        state.players[0].imported_goods = 1;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 5);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn port_enumeration_power_2_hand_2_only_singles() {
        // Hand already holds 2 orders: singles (2+1=3, OK) but pairs
        // (2+2=4 > 3) would exceed the hand limit, so only singles + Skip
        // appear: 2 + 0 + 1 = 3.
        let mut state = create_port_area_state(2, seeded_port([1, 2, 3, 4, 5, 6]));
        state.players[0].money = 2;
        state.players[0].imported_goods = 1;
        state.players[0].order_cards_in_hand = vec![OrderCardId(200), OrderCardId(201)];
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 3);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn port_enumeration_power_1_is_skip_only() {
        // Power 1 -> no zone accessible: Skip is the only legal move.
        let state = create_port_area_state(1, seeded_port([1, 2, 3, 4, 5, 6]));
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 1);
        assert!(matches!(&moves[0], Move::ExecuteAreaAction { action: AreaAction::Skip }));
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn port_enumeration_includes_same_zone_pair() {
        // Power 2: the only possible pair is the two Low-zone cards (slots
        // 0,1) -- pins the same-zone double-take case.
        let mut state = create_port_area_state(2, seeded_port([1, 2, 3, 4, 5, 6]));
        state.players[0].money = 2;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        let has_same_zone_pair = moves.iter().any(|m| matches!(m,
            Move::ExecuteAreaAction {
                action: AreaAction::Port {
                    cards_taken,
                    second_card_payment: Some(PortPayment::TwoYen),
                }
            } if cards_taken == &vec![OrderCardId(1), OrderCardId(2)]
        ));
        assert!(has_same_zone_pair, "expected a pair move taking both Low-zone cards {{1,2}}");
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn port_enumeration_during_mercenary_sub_turn_resolves_mercenary_target_area() {
        // President is in hand (never placed) -- Port enumeration must
        // resolve the acting area from mercenary_target_area, mirroring how
        // execute_port_action resolves the acting board during a mercenary
        // sub-turn. Same resources as the power-2 pin: 2 + 1x2 + 1 = 5.
        let mut state = create_test_state();
        let port_area = AreaId::new(5);
        state.board_layout.areas.insert(port_area, AreaConfig {
            area_type: AreaType::PortA,
            grid_position: (0, 5),
        });
        state.port_state.port_a = seeded_port([1, 2, 3, 4, 5, 6]);
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(port_area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(2);
        state.players[0].money = 2;
        state.players[0].imported_goods = 1;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 5);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn lab_enumeration_power_3_two_deficits_no_overpay() {
        // Power 3: slot0 cost 3 (deficit 0) -> 1 move; slot2 cost 3 +
        // surcharge 1 = required 4 (deficit 1) -> 2 exact-cover moves
        // ({extra_yen:2},{extra_imported:1}); + 1 Skip = 4. <4 techs owned,
        // so fifth_tech_payment stays None throughout.
        let mut state = create_lab_area_state(3);
        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));
        state.lab_state.lab_a.slots[2] = Some(TechCardId(2));
        state.players[0].money = 2;
        state.players[0].imported_goods = 1;

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(make_test_tech_card(1, 3), "tech_1");
        rules.techs.insert(make_test_tech_card(2, 3), "tech_2");

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 4);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    // --- Task 17: Church, Customs, Employment Agency & Chinatown enumeration ---

    /// Places a Church area on the board, puts the president there, and
    /// enters the AreaAction phase at the given power.
    fn create_church_area_state(power: u8) -> GameState {
        let mut state = create_test_state();
        let area = AreaId::new(5);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Church,
            grid_position: (0, 5),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(power);
        state
    }

    /// Places a Customs area on the board, puts the president there, and
    /// enters the AreaAction phase at the given power.
    fn create_customs_area_state(power: u8) -> GameState {
        let mut state = create_test_state();
        let area = AreaId::new(5);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Customs,
            grid_position: (0, 5),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(power);
        state
    }

    /// Places an Employment Agency area on the board, puts the president
    /// there, and enters the AreaAction phase at the given power.
    fn create_employment_agency_area_state(power: u8) -> GameState {
        let mut state = create_test_state();
        let area = AreaId::new(5);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::EmploymentAgency,
            grid_position: (0, 5),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(power);
        state
    }

    /// Places a Chinatown area on the board, puts the president there, and
    /// enters the AreaAction phase at the given power (Task 17b: power now
    /// caps the BFS enumeration depth -- see `CHINATOWN_ENUM_DEPTH`'s doc
    /// comment).
    fn create_chinatown_area_state(power: u8) -> GameState {
        let mut state = create_test_state();
        let area = AreaId::new(5);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Chinatown,
            grid_position: (0, 5),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(power);
        state
    }

    /// Bumps a 2P test state to 3 players (mirrors
    /// `actions::tracks::tests::create_test_state_3p`).
    fn make_3p(state: &mut GameState) {
        state.config.player_count = 3;
        state.players.push(PlayerState::new(PlayerId::new(2), PlayerColor::Yellow, false));
    }

    #[test]
    fn church_enumeration_stock_limited_low_faith() {
        // 3P, power 3, stock {copper:1, rest 0}, 1 assistant, empty board:
        // faith 3 (all-false subset) is below the floor of 4 ->
        // executor-illegal; faith 4 ({copper}) has an eligible empty space
        // -> 1 substantive move + Skip = 2.
        let mut state = create_church_area_state(3);
        make_3p(&mut state);
        state.players[0].copper = 1;
        state.players[0].silk = 0;
        state.players[0].tea = 0;
        state.players[0].seafood = 0;
        state.players[0].money = 0;
        state.players[0].imported_goods = 0;
        state.players[0].assistants.in_hand = 1;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 2);
        assert!(moves.iter().any(|m| matches!(m,
            Move::ExecuteAreaAction { action: AreaAction::Church {
                copper: true, silk: false, tea: false, seafood: false, yen: false, imported_good: false,
            } }
        )));
        assert!(!moves.iter().any(|m| matches!(m,
            Move::ExecuteAreaAction { action: AreaAction::Church {
                copper: false, silk: false, tea: false, seafood: false, yen: false, imported_good: false,
            } }
        )), "faith-3 all-false subset is executor-illegal (below the floor of 4)");
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn church_enumeration_all_64_subsets_at_power_5() {
        // 3P, power 5, all six stocks >= 1, assistant, empty board: every
        // one of the 64 flag combinations reaches faith >= 5 (>= floor 4)
        // with an empty eligible space (board is empty) -> 64 + Skip = 65.
        let mut state = create_church_area_state(5);
        make_3p(&mut state);
        state.players[0].copper = 1;
        state.players[0].silk = 1;
        state.players[0].tea = 1;
        state.players[0].seafood = 1;
        state.players[0].money = 1;
        state.players[0].imported_goods = 1;
        state.players[0].assistants.in_hand = 1;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 65);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn church_enumeration_no_assistant_is_skip_only() {
        let mut state = create_church_area_state(5);
        state.players[0].assistants.in_hand = 0;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 1);
        assert!(matches!(&moves[0], Move::ExecuteAreaAction { action: AreaAction::Skip }));
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn customs_enumeration_fresh_board_power_4() {
        // 3P, power 4 (cap 3), imports 2, assistant, fresh board -> {1,2} + Skip = 3.
        let mut state = create_customs_area_state(4);
        make_3p(&mut state);
        state.players[0].imported_goods = 2;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 3);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn customs_enumeration_high_n_space_occupied() {
        // 3P, power 5 (cap 4), imports 4, assistant, space 7 (cost-4's only
        // space) occupied -> {1,2,3} + Skip = 4.
        let mut state = create_customs_area_state(5);
        make_3p(&mut state);
        state.players[0].imported_goods = 4;
        let idx7 = CustomsTrack::space_to_index(7).unwrap();
        state.customs_track.slots[idx7] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
        state.customs_track.next_arrival_order = 1;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 4);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn customs_enumeration_2p_blocks_dummy_space() {
        // 2P, power 3 (cap 2), imports 2, assistant, space 3 occupied ->
        // N=2's fallback (space 4) is the 2P printed dummy -> only {1} + Skip = 2.
        let mut state = create_customs_area_state(3);
        state.players[0].imported_goods = 2;
        let idx3 = CustomsTrack::space_to_index(3).unwrap();
        state.customs_track.slots[idx3] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
        state.customs_track.next_arrival_order = 1;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 2);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn customs_enumeration_no_assistant_is_skip_only() {
        let mut state = create_customs_area_state(4);
        state.players[0].imported_goods = 3;
        state.players[0].assistants.in_hand = 0;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 1);
        assert!(matches!(&moves[0], Move::ExecuteAreaAction { action: AreaAction::Skip }));
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn employment_enumeration_bounded_compositions_money_2() {
        // Power 3 (allowed 2), warehouse 2 assistants/1 shophouse/0 TH,
        // money 2 -> (1,0,0),(2,0,0),(0,1,0),(1,1,0) + Skip = 5.
        let mut state = create_employment_agency_area_state(3);
        state.players[0].assistants.in_warehouse = 2;
        state.players[0].shophouses.in_warehouse = 1;
        state.players[0].trading_houses.in_warehouse = 0;
        state.players[0].money = 2;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 5);
        for (a, s, t) in [(1, 0, 0), (2, 0, 0), (0, 1, 0), (1, 1, 0)] {
            assert!(moves.iter().any(|m| matches!(m,
                Move::ExecuteAreaAction { action: AreaAction::EmploymentAgency { assistants, shophouses, trading_houses } }
                if *assistants == a && *shophouses == s && *trading_houses == t
            )), "expected ({a},{s},{t}) among the generated moves");
        }
        assert!(!moves.iter().any(|m| matches!(m,
            Move::ExecuteAreaAction { action: AreaAction::EmploymentAgency { assistants: 0, shophouses: 0, trading_houses: 0 } }
        )), "the all-zero encoding must never be emitted (Skip is the sole decline)");
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn employment_enumeration_shophouse_unaffordable_at_money_1() {
        let mut state = create_employment_agency_area_state(3);
        state.players[0].assistants.in_warehouse = 2;
        state.players[0].shophouses.in_warehouse = 1;
        state.players[0].trading_houses.in_warehouse = 0;
        state.players[0].money = 1;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 3);
        for (a, s, t) in [(1, 0, 0), (2, 0, 0)] {
            assert!(moves.iter().any(|m| matches!(m,
                Move::ExecuteAreaAction { action: AreaAction::EmploymentAgency { assistants, shophouses, trading_houses } }
                if *assistants == a && *shophouses == s && *trading_houses == t
            )));
        }
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn employment_enumeration_trading_house_progression_money_9() {
        // Power 5 (allowed 4), warehouse 0/0/2, 0 built, money 9:
        // (0,0,1) costs 4, (0,0,2) costs 4+5=9 -> both + Skip = 3.
        let mut state = create_employment_agency_area_state(5);
        state.players[0].assistants.in_warehouse = 0;
        state.players[0].shophouses.in_warehouse = 0;
        state.players[0].trading_houses.in_warehouse = 2;
        state.players[0].money = 9;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 3);
        for t in [1, 2] {
            assert!(moves.iter().any(|m| matches!(m,
                Move::ExecuteAreaAction { action: AreaAction::EmploymentAgency { assistants: 0, shophouses: 0, trading_houses } }
                if *trading_houses == t
            )));
        }
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn employment_enumeration_trading_house_progression_money_8() {
        let mut state = create_employment_agency_area_state(5);
        state.players[0].assistants.in_warehouse = 0;
        state.players[0].shophouses.in_warehouse = 0;
        state.players[0].trading_houses.in_warehouse = 2;
        state.players[0].money = 8;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 2);
        assert!(moves.iter().any(|m| matches!(m,
            Move::ExecuteAreaAction { action: AreaAction::EmploymentAgency { assistants: 0, shophouses: 0, trading_houses: 1 } }
        )));
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn chinatown_enumeration_hand_derived_small_fixture_power_3() {
        // Fixture: money 1, everything else 0, power 3 (Task 17b: depth cap
        // = min(3,4) = 3). HAND DERIVATION (full walk-through in the Task
        // 17/17b report): the reachable exchange graph is a 2-leaf star --
        // R=(money:1) <-> T=(tea:1) via YenToTea/TeaToYen, and R <->
        // S=(seafood:1) via YenToSeafood/SeafoodToYen -- since money never
        // reaches 2 (so the Copper/Silk/Imported branches are permanently
        // unreachable from this fixture). Valid walks of length L starting
        // at R number 2^ceil(L/2) (one binary choice -- Tea-branch or
        // Seafood-branch -- per odd step, with the following even step
        // forced back to R): L=1 -> 2, L=2 -> 2, L=3 -> 4. The length-4
        // collision (the two walks that visit both leaves once each
        // collapse to the same multiset) never happens at depth <= 3, so
        // all 2+2+4 = 8 raw sequences are distinct multisets -> 8 + Skip =
        // 9. This also pins the depth cap: the R<->T/S cycle is periodic
        // (reachable indefinitely), so anything beyond 9 here would mean
        // the power-capped depth stopped bounding the search.
        let mut state = create_chinatown_area_state(3);
        state.players[0].copper = 0;
        state.players[0].silk = 0;
        state.players[0].tea = 0;
        state.players[0].seafood = 0;
        state.players[0].money = 1;
        state.players[0].imported_goods = 0;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 9);
        assert!(!moves.iter().any(|m| matches!(m,
            Move::ExecuteAreaAction { action: AreaAction::Chinatown { exchanges } } if exchanges.is_empty()
        )), "the empty encoding must never be emitted (Skip is the sole decline)");
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn chinatown_enumeration_hand_derived_small_fixture_power_2_caps_depth() {
        // Same fixture as the power-3 pin above, at power 2 (depth cap =
        // min(2,4) = 2): only the L=1 (2 raw) and L=2 (2 raw) walks survive
        // -- {YenToTea}, {YenToSeafood}, {YenToTea,TeaToYen},
        // {YenToSeafood,SeafoodToYen} -- all 4 distinct (no collision is
        // possible below length 4). 4 + Skip = 5.
        let mut state = create_chinatown_area_state(2);
        state.players[0].copper = 0;
        state.players[0].silk = 0;
        state.players[0].tea = 0;
        state.players[0].seafood = 0;
        state.players[0].money = 1;
        state.players[0].imported_goods = 0;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 5);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn chinatown_enumeration_preserves_valid_execution_order_not_sorted() {
        // Fixture: 1 copper, money 0, rest 0, power 3. [CopperToYen,
        // YenToTea] is a valid order (sell the copper first, then buy tea
        // with the proceeds); the reverse [YenToTea, CopperToYen] is NOT
        // valid (YenToTea needs money >= 1, but money starts at 0) -- the
        // generator must emit the valid order, never a sorted one.
        let mut state = create_chinatown_area_state(3);
        state.players[0].copper = 1;
        state.players[0].silk = 0;
        state.players[0].tea = 0;
        state.players[0].seafood = 0;
        state.players[0].money = 0;
        state.players[0].imported_goods = 0;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert!(moves.iter().any(|m| matches!(m,
            Move::ExecuteAreaAction { action: AreaAction::Chinatown { exchanges } }
            if exchanges == &vec![ChinatownExchange::CopperToYen, ChinatownExchange::YenToTea]
        )), "expected the valid order [CopperToYen, YenToTea] among the generated moves");
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn chinatown_enumeration_mining_bonus_three_exchange_chain_discriminates_drop() {
        // Player owns 1 Mining Technology copy, money 4, copper 0, power 3
        // (depth cap = min(3,4) = 3). At the start, only YenToCopper is
        // resource-valid (copper=0 blocks CopperToYen/etc.; money=4 covers
        // YenToCopper's cost of 2).
        //
        // This targets the 3-exchange multiset {YenToCopper:1,
        // CopperToYen:2}, which GENUINELY discriminates a dropped Mining
        // bonus -- unlike a 2-exchange [YenToCopper, CopperToYen] pin
        // (reachable either way, since base copper alone is enough to
        // unlock a single CopperToYen):
        //   WITH the bonus: step1 YenToCopper -> money 2, copper 2 (1 base
        //   + 1 mining); step2 CopperToYen -> copper 1, money 4; step3
        //   CopperToYen -> copper 0, money 6 -- all three steps valid.
        //   WITHOUT the bonus: step1 YenToCopper -> copper 1 (base only);
        //   step2 CopperToYen -> copper 0; step3 CopperToYen requires
        //   copper >= 1 -- UNREACHABLE.
        // The only valid execution order for this multiset is
        // [YenToCopper, CopperToYen, CopperToYen] (leading with CopperToYen
        // is impossible since copper starts at 0), so the exact-vector
        // match below pins the same fact a multiset check would.
        let mut state = create_chinatown_area_state(3);
        state.players[0].copper = 0;
        state.players[0].silk = 0;
        state.players[0].tea = 0;
        state.players[0].seafood = 0;
        state.players[0].money = 4;
        state.players[0].imported_goods = 0;

        let mut rules = Rules::empty_for_tests();
        let mining = TechCard {
            id: TechCardId(1),
            name: "Mining Technology".to_string(),
            country: Country::German,
            industry_cost: 3,
            effect: TechEffect::MiningTechnology,
        };
        rules.techs.insert(mining, "tech_mining");
        state.players[0].technologies.push(TechCardId(1));

        let moves = get_legal_moves(&state, &rules);
        assert!(moves.iter().any(|m| matches!(m,
            Move::ExecuteAreaAction { action: AreaAction::Chinatown { exchanges } }
            if exchanges == &vec![
                ChinatownExchange::YenToCopper,
                ChinatownExchange::CopperToYen,
                ChinatownExchange::CopperToYen,
            ]
        )), "expected [YenToCopper, CopperToYen, CopperToYen] among the generated moves");
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn church_enumeration_2p_blocks_faith_7_discriminating_pair() {
        // 2P, power 5, stock exactly {copper:1, silk:1, rest 0}, assistant,
        // church spaces 4/5/6 occupied (7-11 empty). Only 4 stock-valid
        // combos exist: {} (faith 5), {c} (faith 6), {s} (faith 6), {c,s}
        // (faith 7). {}/{c}/{s} all fall through occupied spaces 5/6/4 with
        // nothing left above -> None. {c,s}'s faith-7 slot is empty but 2P
        // blocks it outright (printed dummy) -> also None. Every combo is
        // executor-illegal -> Skip only = 1.
        let mut state = create_church_area_state(5);
        state.players[0].copper = 1;
        state.players[0].silk = 1;
        state.players[0].tea = 0;
        state.players[0].seafood = 0;
        state.players[0].money = 0;
        state.players[0].imported_goods = 0;
        state.players[0].assistants.in_hand = 1;
        for faith in [4, 5, 6] {
            let idx = ChurchTrack::faith_to_index(faith).unwrap();
            state.church_track.slots[idx] = Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 0 });
        }
        state.church_track.next_arrival_order = 3;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 1);
        assert!(matches!(&moves[0], Move::ExecuteAreaAction { action: AreaAction::Skip }));
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn church_enumeration_3p_allows_faith_7_discriminating_pair() {
        // Identical fixture to the 2P pin above, but 3P: {c,s}'s faith-7
        // slot is empty and NOT blocked (no 2P dummy) -> eligible. {}/{c}/{s}
        // still fall through the occupied 4/5/6 spaces -> None. Result:
        // {c,s} + Skip = 2.
        let mut state = create_church_area_state(5);
        make_3p(&mut state);
        state.players[0].copper = 1;
        state.players[0].silk = 1;
        state.players[0].tea = 0;
        state.players[0].seafood = 0;
        state.players[0].money = 0;
        state.players[0].imported_goods = 0;
        state.players[0].assistants.in_hand = 1;
        for faith in [4, 5, 6] {
            let idx = ChurchTrack::faith_to_index(faith).unwrap();
            state.church_track.slots[idx] = Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 0 });
        }
        state.church_track.next_arrival_order = 3;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 2);
        assert!(moves.iter().any(|m| matches!(m,
            Move::ExecuteAreaAction { action: AreaAction::Church {
                copper: true, silk: true, tea: false, seafood: false, yen: false, imported_good: false,
            } }
        )));
        assert_all_moves_apply(&state, &moves, &rules);
    }
}
