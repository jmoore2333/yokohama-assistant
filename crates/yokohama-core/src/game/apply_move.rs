//! Apply moves to game state.

use crate::types::*;
use crate::game::*;
use crate::game::actions::*;

/// Apply a move to the game state, returning the new state. Raw per-arm
/// validation only -- no membership check against `get_legal_moves`. Called
/// deliberately by prechecked hot paths (Session's gate, the fuzz drivers,
/// future search inner loops) AFTER choosing from an enumerated legal list;
/// everything else should go through the checked `apply_move` below.
pub fn apply_move_unchecked(state: &GameState, mv: &Move, rules: &Rules) -> Result<GameState, MoveError> {
    let mut new_state = state.clone();

    match mv {
        Move::EndAdditionalPhase => {
            // apply_move_unchecked is the raw arm, with no legal-list gate
            // of its own -- a Phase-3 bot's search inner loop may call it
            // directly after choosing from its own enumerated list, not
            // necessarily through checked apply_move's pre-filtering -- so
            // it self-defends here the same way UseUniversity/UseSpinningMill
            // defend their once-per-turn flag and apply_mercenary defends
            // mercenary_used (T7 precedent):
            // ending the phase with a choice still queued would advance
            // the turn (handing off to the next player, in the
            // AdditionalActionSecond case) while a reward from THIS turn
            // is still unresolved, so a later resolution of the stale
            // queue would misgrant it to the wrong player (design §5.1
            // queue invariant, I1(a) final-review fix). The queue-first
            // gate in get_legal_moves never offers EndAdditionalPhase
            // mid-queue, so this is unreachable under legal-list play.
            if !new_state.pending_choices.is_empty() {
                return Err(MoveError::RuleViolation(
                    "Cannot end the additional-action phase with a pending choice unresolved",
                ));
            }
            advance_phase(&mut new_state, Some(&rules.techs));
        }

        Move::PlaceThreeAreas { placements } => {
            // Stage-4 raw-arm self-defense (Phase-2.9 census): count cap
            // (3, or 4 with Newspaper -- `placement_limit` is the same
            // tech-ownership lookup the generator uses) and distinctness
            // (Option A is "1 assistant in each of 1-N DIFFERENT areas";
            // 2-in-one-area is Option B's encoding). Checked in usize, so a
            // 260-entry payload can never truncate through a u8 and
            // underflow `in_hand` (the census's conservation-destroying
            // probe). Per-area board-membership + placeability live in
            // `apply_placement` (shared with the Telegram bonus arm).
            let player = new_state.current_player;
            if placements.len() > placement_limit(&new_state, player, &rules.techs) as usize {
                return Err(MoveError::RuleViolation(
                    "Too many placement areas (limit 3, or 4 with Newspaper)",
                ));
            }
            let mut seen = std::collections::BTreeSet::new();
            if !placements.iter().all(|&a| seen.insert(a)) {
                return Err(MoveError::RuleViolation(
                    "Option A placements must be distinct areas",
                ));
            }
            apply_placement(&mut new_state, placements)?;
            // Record president's starting area for this turn's movement
            set_president_start_area(&mut new_state);
            new_state.turn_phase = TurnPhase::Movement;
        }

        Move::PlaceTwoSameArea { area, adjacent_bonus } => {
            // Stage-4 raw-arm self-defense (Phase-2.9 census): the Telegram
            // adjacent-bonus placement is validated BEFORE any mutation --
            // tech ownership + physical adjacency with crossing pairs
            // excluded (transit-only edges), same gates the generator
            // applies. Main-area membership/placeability is checked inside
            // `apply_placement_same_area`.
            if let Some(adj_area) = adjacent_bonus {
                validate_telegram_bonus(&new_state, *area, *adj_area, rules)?;
            }
            apply_placement_same_area(&mut new_state, *area)?;
            // Telegram bonus: place 1 in adjacent area, under the same fee
            // rules as ordinary placement. Reusing apply_placement guarantees
            // this never drifts from generate_placement_moves's cost + adj_cost
            // affordability check.
            if let Some(adj_area) = adjacent_bonus {
                apply_placement(&mut new_state, &[*adj_area])?;
            }
            // Record president's starting area for this turn's movement
            set_president_start_area(&mut new_state);
            new_state.turn_phase = TurnPhase::Movement;
        }

        Move::MovePresident { path, destination } => {
            // Validate: cannot return to starting area
            validate_movement_destination(&new_state, *destination)?;
            // Stage-4 raw-arm self-defense (Phase-2.9 census): STRUCTURAL
            // path validation only -- see `validate_president_path`.
            validate_president_path(&new_state, path, *destination)?;
            apply_movement(&mut new_state, path, *destination, Some(&rules.techs))?;
            new_state.president_start_area = None; // Clear after movement
            new_state.turn_phase = TurnPhase::AreaAction;
        }

        Move::PlacePresident { destination } => {
            // Note: PlacePresident is for placing from hand - no start area restriction
            apply_place_president(&mut new_state, *destination, Some(&rules.techs))?;
            new_state.president_start_area = None; // Clear after placement
            new_state.turn_phase = TurnPhase::AreaAction;
        }

        Move::ReturnPresidentToHand { recover } => {
            apply_return_to_hand(&mut new_state, recover)?;
            new_state.president_start_area = None; // Clear after return
            // Skip to Additional Action Second
            new_state.turn_phase = TurnPhase::AdditionalActionSecond;
        }

        Move::ExecuteAreaAction { action } => {
            apply_area_action(&mut new_state, action, rules)?;
            new_state.turn_phase = TurnPhase::FivePowerBonus;
        }

        Move::Take5PowerToken => {
            apply_take_5_power(&mut new_state, rules)?;
            new_state.turn_phase = TurnPhase::Building;
        }

        Move::Skip5PowerBonus => {
            new_state.turn_phase = TurnPhase::Building;
        }

        Move::BuildShophouse { slot } => {
            apply_build_shophouse(&mut new_state, *slot, rules)?;
            new_state.turn_phase = TurnPhase::Recovery;
        }

        Move::BuildTradingHouse => {
            apply_build_trading_house(&mut new_state, rules)?;
            new_state.turn_phase = TurnPhase::Recovery;
        }

        Move::SkipBuilding => {
            new_state.turn_phase = TurnPhase::Recovery;
        }

        Move::UseMercenary { country, target_area } => {
            apply_mercenary(&mut new_state, *country, *target_area)?;
        }

        Move::FulfillOrder { order_id } => {
            apply_fulfill_order(&mut new_state, *order_id, rules)?;
        }

        Move::FulfillAchievement { achievement } => {
            apply_fulfill_achievement(&mut new_state, *achievement)?;
        }

        Move::UseUniversity => {
            let player = new_state.current_player;
            // apply_university checks only resource preconditions, not the
            // once-per-turn flag -- apply_move_unchecked is the raw arm with
            // no legal-list gate of its own (a Phase-3 bot's search inner
            // loop may call it directly, not just via checked apply_move's
            // legal-move pre-filtering), so it must defend the flag itself,
            // mirroring apply_mercenary's self-defended mercenary_used gate.
            if !can_use_university(&new_state, player, &rules.techs) {
                return Err(MoveError::RuleViolation("University not usable this turn"));
            }
            apply_university(&mut new_state, player).map_err(MoveError::RuleViolation)?;
        }

        Move::UseSpinningMill => {
            let player = new_state.current_player;
            if !can_use_spinning_mill(&new_state, player, &rules.techs) {
                return Err(MoveError::RuleViolation("Textile Mill not usable this turn"));
            }
            apply_spinning_mill(&mut new_state, player).map_err(MoveError::RuleViolation)?;
        }

        Move::ChooseAnyGood { .. }
        | Move::ChooseWarehouseTake { .. }
        | Move::DeclineWarehouseTake
        | Move::ChooseAssistantMove { .. }
        | Move::DeclineAssistantMove
        | Move::ChooseChurchSpace { .. }
        | Move::ChooseCustomsSpace { .. }
        | Move::ChooseRecoveryLeave { .. }
        | Move::ChooseStationPlace { .. } => {
            crate::game::choices::resolve_choice_move(&mut new_state, mv, rules)?;
        }
    }

    // Task 9 (design §4.4): the resolution just above may have left another,
    // still-queued choice further back in the queue degraded to Decline-only
    // (e.g. two queued WarehouseTakes sharing one warehouse item -- the
    // first resolution can empty it out from under the second). Normalize
    // BEFORE the Recovery gate below, so a queue emptied entirely by
    // normalization still triggers Recovery in this same apply.
    crate::game::choices::normalize_queue_head(&mut new_state, rules);

    // Handle Recovery phase automatically -- deferred while a choice from
    // this turn's reward is still unresolved (design 2026-07-18 §5.1-5.2):
    // resolving the last queued choice runs Recovery in that same apply.
    // `resolve_recovery` may itself defer: with Telephone owned it can push
    // `PendingChoice::RecoveryLeave` and return with turn_phase still
    // `Recovery` (design §5.3-5), leaving the phase transition for
    // `ChooseRecoveryLeave`'s resolution to run instead (see
    // `game::choices::resolve_choice_move`).
    if new_state.pending_choices.is_empty() && new_state.turn_phase == TurnPhase::Recovery {
        resolve_recovery(&mut new_state, rules);
    }

    // Check end game triggers
    check_end_game_triggers(&mut new_state);

    Ok(new_state)
}

/// The ONE checked public transition (design §4.1): membership in
/// `get_legal_moves` is the legality contract; per-arm validation then runs
/// in the raw transition. Prechecked hot paths (Session's gate, the fuzz
/// drivers, future search inner loops) call `apply_move_unchecked`
/// deliberately, AFTER choosing from an enumerated legal list.
pub fn apply_move(state: &GameState, mv: &Move, rules: &Rules) -> Result<GameState, MoveError> {
    if !crate::game::legal_moves::get_legal_moves(state, rules).contains(mv) {
        return Err(MoveError::NotLegal);
    }
    apply_move_unchecked(state, mv, rules)
}

/// Set president's starting area for movement restriction.
/// Called when entering Movement phase.
fn set_president_start_area(state: &mut GameState) {
    let player = state.current_player;
    let p = state.player(player);

    state.president_start_area = match p.president_location {
        PresidentLocation::OnBoard(area) => Some(area),
        PresidentLocation::InHand => None,
    };
}

/// Validate that movement destination is not the starting area.
/// "The President pawn may not return to the Area that it started in."
fn validate_movement_destination(state: &GameState, destination: AreaId) -> Result<(), MoveError> {
    if let Some(start) = state.president_start_area {
        if destination == start {
            return Err(MoveError::RuleViolation("Cannot return to starting area in same turn"));
        }
    }
    Ok(())
}

fn advance_phase(state: &mut GameState, tech_registry: Option<&TechRegistry>) {
    state.turn_phase = match state.turn_phase {
        TurnPhase::AdditionalActionFirst => TurnPhase::Placement,
        TurnPhase::AdditionalActionSecond => {
            // End of turn - handle end game logic
            let next_player = state.next_player();

            if state.final_round && next_player == state.config.start_player {
                // Final round completed - the game ends here (rulebook p.9:
                // finish the trigger round, one full final round, equal
                // turns for all). TurnEnd is what both is_game_over impls
                // key on; nothing else ever assigns it.
                TurnPhase::TurnEnd
            } else {
                // Trigger round completes when the turn passes back to the
                // start player - the next full round is the final one.
                if state.end_game_triggered
                    && !state.final_round
                    && next_player == state.config.start_player
                {
                    state.final_round = true;
                }

                // Advance to next player
                state.current_player = next_player;
                state.turn_number += 1;
                state.additional_action_state = AdditionalActionState::new();
                state.current_action_power = None;

                // System of Stock (KB 08-tech-cards.yaml tech_stock_market):
                // "Start of Your Turn: if you have less than 2 yen, increase
                // to 2 yen" - automatic, no player choice.
                if let Some(registry) = tech_registry {
                    apply_turn_start_effects(state, next_player, registry);
                }

                TurnPhase::AdditionalActionFirst
            }
        }
        other => other, // Should not happen
    };
}

/// Structural check shared by every arm that takes an `AreaId` payload:
/// the area must exist on this board and allow placement (`allows_placement`
/// is false only for the Canal). Stage-4 raw-arm self-defense (Phase-2.9
/// census): ghost ids and Canal payloads reach the raw arm when the
/// legal-list gate is bypassed, so each arm must reject them itself.
fn require_placeable_area(state: &GameState, area: AreaId) -> Result<(), MoveError> {
    let config = state
        .board_layout
        .areas
        .get(&area)
        .ok_or(MoveError::RuleViolation("Area does not exist on this board"))?;
    if !config.area_type.allows_placement() {
        return Err(MoveError::RuleViolation("The Canal is never a placeable area"));
    }
    Ok(())
}

/// Telegram adjacent-bonus validation for `Move::PlaceTwoSameArea` (Stage-4
/// raw-arm self-defense): the bonus is only honored when the mover actually
/// OWNS a Telegram copy, and the bonus area is physically adjacent to the
/// main area with canal-crossing pairs excluded (crossings are
/// movement-transit edges ONLY -- design 2026-07-18 decision log, Task 9 --
/// the same exclusion the generator applies). Membership/placeability of
/// both areas is also checked here so the whole move validates before any
/// mutation runs.
fn validate_telegram_bonus(
    state: &GameState,
    area: AreaId,
    adj_area: AreaId,
    rules: &Rules,
) -> Result<(), MoveError> {
    let p = state.player(state.current_player);
    let has_telegram = p.technologies.iter().any(|&tech_id| {
        rules.techs.get(tech_id).map(|t| t.effect == TechEffect::Telegram).unwrap_or(false)
    });
    if !has_telegram {
        return Err(MoveError::RuleViolation(
            "Adjacent-bonus placement requires the Telegram technology",
        ));
    }
    require_placeable_area(state, area)?;
    require_placeable_area(state, adj_area)?;
    if !state.board_layout.adjacent_areas(area).contains(&adj_area) {
        return Err(MoveError::RuleViolation(
            "Telegram bonus area must be adjacent to the main placement area",
        ));
    }
    if state.board_layout.is_canal_crossing(area, adj_area) {
        return Err(MoveError::RuleViolation(
            "Telegram bonus placement may not use a canal-crossing pair",
        ));
    }
    Ok(())
}

/// STRUCTURAL validation for `Move::MovePresident` (Stage-4 raw-arm
/// self-defense, Phase-2.9 census): the president must be OnBoard
/// (`PlacePresident` is the from-hand encoding); the destination and every
/// hop must exist on the board and not be the Canal; and a NON-EMPTY path
/// must be a simple walk -- consecutive hops adjacent (board adjacency,
/// which includes canal-crossing edges), no repeated areas (a repeated hop
/// would multiply pass-through fees), never re-entering the start area, and
/// ending exactly at `destination` (fees are priced off `path`, so the two
/// must agree).
///
/// An EMPTY path stays permitted: it is the Station/Tram free-teleport
/// ENCODING. Whether the mover actually owns the entitling technology --
/// like assistant-chain traversal and affordability -- is SEMANTIC
/// legality and deliberately remains the CHECKED boundary's job (Tier C):
/// the raw arm's contract for those dimensions is legal-list inputs. One
/// tolerance follows from that contract: an empty-path destination equal
/// to the engine's own `station_location` is accepted WITHOUT a
/// board-membership lookup, because the generator's Station teleport is
/// keyed off `station_location` (engine state, not payload) and never
/// consults the areas map -- a strict membership check here would reject a
/// legal-list move on fixture boards whose station area isn't in `areas`
/// (pinned by legal_moves.rs's
/// `test_station_teleport_remains_free_even_onto_opponent_occupied_area`).
/// Ghost/Canal payloads still die: they match neither the board nor the
/// engine's station location.
fn validate_president_path(
    state: &GameState,
    path: &[AreaId],
    destination: AreaId,
) -> Result<(), MoveError> {
    let start = match state.player(state.current_player).president_location {
        PresidentLocation::OnBoard(a) => a,
        PresidentLocation::InHand => {
            return Err(MoveError::RuleViolation(
                "President must be on the board to move (use PlacePresident from hand)",
            ));
        }
    };

    if path.is_empty() {
        // Teleport encoding -- entitlement is Tier C (see doc comment).
        if state.station_location != Some(destination) {
            require_placeable_area(state, destination)?;
        }
        return Ok(());
    }
    require_placeable_area(state, destination)?;

    if *path.last().expect("non-empty") != destination {
        return Err(MoveError::RuleViolation(
            "Movement path must end at the declared destination",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    seen.insert(start);
    let mut prev = start;
    for &hop in path {
        require_placeable_area(state, hop)?;
        if !seen.insert(hop) {
            return Err(MoveError::RuleViolation(
                "Movement path must not repeat areas (simple path)",
            ));
        }
        if !state.board_layout.adjacent_areas(prev).contains(&hop) {
            return Err(MoveError::RuleViolation(
                "Movement path hops must be adjacent on the board",
            ));
        }
        prev = hop;
    }
    Ok(())
}

fn apply_placement(state: &mut GameState, placements: &[AreaId]) -> Result<(), MoveError> {
    let player = state.current_player;

    // Stage-4 raw-arm self-defense: every target area must exist and be
    // placeable (never the Canal) -- checked before any mutation.
    for &area in placements {
        require_placeable_area(state, area)?;
    }

    let p = state.player(player);
    // usize comparison (census fix): `placements.len() as u8` truncated a
    // 260-entry payload to 4, passing this check and then underflowing
    // `in_hand` in the mutating loop below.
    if placements.len() > p.assistants.in_hand as usize {
        return Err(MoveError::RuleViolation("Not enough assistants in hand"));
    }

    // Validate affordability before any mutation (matches calculate_placement_cost,
    // the same function generate_placement_moves used to price this move).
    let cost = calculate_placement_cost(state, player, placements);
    if state.player(player).money < cost {
        return Err(MoveError::NotEnoughMoney);
    }

    let p = state.player_mut(player);
    p.money -= cost;
    for &area in placements {
        p.assistants.in_hand -= 1;
        *p.assistants.on_board.entry(area).or_insert(0) += 1;
    }

    credit_placement_fees(state, player, placements);

    Ok(())
}

fn apply_placement_same_area(state: &mut GameState, area: AreaId) -> Result<(), MoveError> {
    let player = state.current_player;

    // Stage-4 raw-arm self-defense: ghost/Canal main area (see
    // `require_placeable_area`), checked before any mutation.
    require_placeable_area(state, area)?;

    let p = state.player(player);
    if p.assistants.in_hand < 2 {
        return Err(MoveError::RuleViolation("Not enough assistants in hand"));
    }

    // Validate affordability before any mutation. Priced per assistant
    // placed (KB: "pay 1 Yen to each President's owner per assistant
    // placed") - 2 assistants land in `area`, so calculate_placement_cost
    // gets a 2-element slice, matching generate_placement_moves.
    let cost = calculate_placement_cost(state, player, &[area, area]);
    if state.player(player).money < cost {
        return Err(MoveError::NotEnoughMoney);
    }

    let p = state.player_mut(player);
    p.money -= cost;
    p.assistants.in_hand -= 2;
    *p.assistants.on_board.entry(area).or_insert(0) += 2;

    credit_placement_fees(state, player, &[area, area]);

    Ok(())
}

/// Credit 1 Yen to each opponent President owner per assistant the payer
/// placed into that owner's President's area. Mirrors calculate_placement_cost's
/// (area, opponent) pairing exactly, so the payer's deduction there and the
/// sum of credits here always match (KB: "pay 1 Yen to each President's
/// owner per assistant placed").
fn credit_placement_fees(state: &mut GameState, payer: PlayerId, placements: &[AreaId]) {
    for &area in placements {
        let owners: Vec<PlayerId> = state.players.iter()
            .filter(|p| p.id != payer && p.president_location == PresidentLocation::OnBoard(area))
            .map(|p| p.id)
            .collect();
        for owner in owners {
            state.player_mut(owner).money += 1;
        }
    }
}

fn apply_movement(
    state: &mut GameState,
    path: &[AreaId],
    destination: AreaId,
    tech_registry: Option<&TechRegistry>,
) -> Result<(), MoveError> {
    let player = state.current_player;

    // Pay movement costs with the same tech-aware pricing the generator
    // uses (Stagecoach waives president fees); reject before any mutation.
    let cost = calculate_movement_cost_with_tech(state, player, path, tech_registry);
    if state.player(player).money < cost {
        return Err(MoveError::RuleViolation("Not enough money for movement"));
    }
    state.player_mut(player).money -= cost;
    credit_movement_president_fees(state, player, path, tech_registry);

    let p = state.player_mut(player);
    p.president_location = PresidentLocation::OnBoard(destination);

    // Calculate and store power for this action
    let power = calculate_power(state, player, destination);
    state.current_action_power = Some(power);

    Ok(())
}

/// Credit each fee recipient in `route_fee_vector`'s breakdown for `path`:
/// `Some(owner)` entries (intermediate pass-through AND, Postal System, the
/// destination stop -- I2 final-review fix) go to that opponent's
/// President owner; `None` entries are canal tolls and are NOT credited to
/// anyone (they go to the bank). Task 6 (L2 route enumeration, design
/// 2026-07-19 §4.3): this function no longer re-derives who's owed what --
/// it just pays out exactly what `route_fee_vector` priced (which already
/// handles Stagecoach waiving president fees, never tolls), so
/// `apply_movement`'s deduction and the sum of credits here can never
/// drift apart -- single pricing source, end to end.
fn credit_movement_president_fees(
    state: &mut GameState,
    payer: PlayerId,
    path: &[AreaId],
    tech_registry: Option<&TechRegistry>,
) {
    let vector = route_fee_vector(state, payer, path, tech_registry, false);
    for (recipient, amount) in vector {
        if let Some(owner) = recipient {
            state.player_mut(owner).money += amount;
        }
    }
}

fn apply_place_president(
    state: &mut GameState,
    destination: AreaId,
    tech_registry: Option<&TechRegistry>,
) -> Result<(), MoveError> {
    let player = state.current_player;

    // Stage-4 raw-arm self-defense (Phase-2.9 census): PlacePresident is
    // the from-hand encoding -- with the president already OnBoard it
    // would duplicate-place (MovePresident is the on-board encoding). The
    // destination must also exist and be placeable (never the Canal).
    if state.player(player).president_location != PresidentLocation::InHand {
        return Err(MoveError::RuleViolation(
            "President is already on the board (use MovePresident)",
        ));
    }
    require_placeable_area(state, destination)?;

    // Postal System destination stop fee (I2 final-review fix): mirrors
    // calculate_place_president_fee exactly, so the generator's gate and
    // this charge never drift apart.
    let fee = calculate_place_president_fee(state, player, destination, tech_registry);
    if state.player(player).money < fee {
        return Err(MoveError::NotEnoughMoney);
    }
    state.player_mut(player).money -= fee;
    credit_place_president_fee(state, player, destination, tech_registry);

    let p = state.player_mut(player);
    p.president_location = PresidentLocation::OnBoard(destination);

    let power = calculate_power(state, player, destination);
    state.current_action_power = Some(power);

    Ok(())
}

/// Credit 1 Yen to each opponent President's owner occupying `destination`
/// (mirrors calculate_place_president_fee's Stagecoach-waived pricing
/// exactly, so the payer's deduction and the sum of owner credits always
/// match).
fn credit_place_president_fee(
    state: &mut GameState,
    payer: PlayerId,
    destination: AreaId,
    tech_registry: Option<&TechRegistry>,
) {
    let p = state.player(payer);
    let has_stagecoach = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Stagecoach).unwrap_or(false)
        })
    });
    if has_stagecoach {
        return;
    }
    let owners: Vec<PlayerId> = state.players.iter()
        .filter(|o| o.id != payer && o.president_location == PresidentLocation::OnBoard(destination))
        .map(|o| o.id)
        .collect();
    for owner in owners {
        state.player_mut(owner).money += 1;
    }
}

/// Recover `recover[area]` assistants from each listed area back to hand
/// and return the President to hand (I5 ruling: any number, not just
/// all-or-nothing per area). Validate-all-then-mutate: every count is
/// checked against the current on-board count before any mutation runs, so
/// a rejected move leaves `state` untouched (matches this file's house
/// pattern, e.g. `apply_placement`'s cost check before the mutating loop).
fn apply_return_to_hand(
    state: &mut GameState,
    recover: &std::collections::BTreeMap<AreaId, u8>,
) -> Result<(), MoveError> {
    let player = state.current_player;
    for (&area, &count) in recover {
        let have = state.player(player).assistants.on_board.get(&area).copied().unwrap_or(0);
        if count == 0 || count > have {
            return Err(MoveError::RuleViolation("recovery count exceeds assistants in area"));
        }
    }

    let p = state.player_mut(player);
    for (&area, &count) in recover {
        let slot = p.assistants.on_board.get_mut(&area).expect("validated above");
        *slot -= count;
        if *slot == 0 {
            p.assistants.on_board.remove(&area);
        }
        p.assistants.in_hand += count;
    }
    p.president_location = PresidentLocation::InHand;

    Ok(())
}

/// Stage-4 raw-arm self-defense (Phase-2.9 census): the simple area-action
/// arms (Produce*/Bank/EmploymentAgency/Chinatown) never resolved the acting
/// area at all, so `ProduceCopper` fired at the Bank when the legal-list
/// gate was bypassed. Resolves the acting area mercenary-aware through the
/// SAME `resolve_acting_area` helper Port/Lab/Church/Customs use, then
/// requires its printed type to match the action being executed.
fn require_acting_area_type(state: &GameState, expected: AreaType) -> Result<(), MoveError> {
    let area = resolve_acting_area(state, state.current_player).map_err(MoveError::RuleViolation)?;
    let area_type = state
        .board_layout
        .areas
        .get(&area)
        .map(|c| c.area_type)
        .ok_or(MoveError::RuleViolation("Acting area does not exist on this board"))?;
    if area_type != expected {
        return Err(MoveError::RuleViolation(
            "Area action does not match the acting area's printed type",
        ));
    }
    Ok(())
}

fn apply_area_action(state: &mut GameState, action: &AreaAction, rules: &Rules) -> Result<(), MoveError> {
    let player = state.current_player;
    let power = state.current_action_power.unwrap_or(0);

    // Pay Trading House passive income to owner (if not the current player)
    apply_trading_house_passive_income(state);

    match action {
        AreaAction::ProduceCopper => {
            require_acting_area_type(state, AreaType::CopperMine)?;
            let reward = copper_mine_reward(power);
            let bonus = copper_bonus(state, player, reward, &rules.techs);
            state.player_mut(player).copper += (reward + bonus) as u32;
        }
        AreaAction::ProduceSilk => {
            require_acting_area_type(state, AreaType::SilkMill)?;
            let reward = silk_mill_reward(power);
            state.player_mut(player).silk += reward as u32;
        }
        AreaAction::ProduceTea => {
            require_acting_area_type(state, AreaType::TeaPlantation)?;
            let reward = tea_plantation_reward(power);
            state.player_mut(player).tea += reward as u32;
        }
        AreaAction::ProduceSeafood => {
            require_acting_area_type(state, AreaType::FishingGround)?;
            let reward = fishing_ground_reward(power);
            state.player_mut(player).seafood += reward as u32;
        }
        AreaAction::Bank => {
            require_acting_area_type(state, AreaType::Bank)?;
            let reward = bank_reward(power);
            state.player_mut(player).money += reward as u32;
        }

        // Employment Agency - retrieve items from warehouse
        AreaAction::EmploymentAgency { assistants, shophouses, trading_houses } => {
            require_acting_area_type(state, AreaType::EmploymentAgency)?;
            apply_employment_agency(state, player, power, *assistants, *shophouses, *trading_houses)?;
        }

        // Other actions - simplified implementations
        AreaAction::Port { cards_taken, second_card_payment } => {
            execute_port_action(state, player, cards_taken, *second_card_payment, power)
                .map_err(MoveError::RuleViolation)?;
        }
        AreaAction::Laboratory { card_id, extra_yen, extra_imported, fifth_tech_payment } => {
            execute_lab_action(
                state, player, *card_id, *extra_yen, *extra_imported, *fifth_tech_payment,
                power, &rules.techs,
            )?;
            apply_lab_acquisition_bonuses(state, player, *card_id, &rules.techs, &rules.orders);

            // Station (design §4.2): the token is placed on acquisition, as a
            // queued choice. copies:1, but the is_none() guard is principled.
            let purchased_effect_is_station =
                rules.techs.get(*card_id).map(|t| t.effect == TechEffect::Station).unwrap_or(false);
            if state.station_location.is_none() && purchased_effect_is_station {
                crate::game::choices::push_choice(state, PendingChoice::StationPlace, rules);
            }
        }
        AreaAction::Chinatown { exchanges } => {
            require_acting_area_type(state, AreaType::Chinatown)?;
            // Task 17 ruling: an empty exchanges list is no longer a legal
            // encoding of "decline Chinatown" -- Skip is the sole canonical
            // decline, mirroring Task 16's Port `cards_taken.is_empty()` rule.
            if exchanges.is_empty() {
                return Err(MoveError::RuleViolation("Must make at least 1 exchange (use Skip to decline)"));
            }
            // Task 17b (KB 07-rules-reference.md line 312: "Number of
            // exchanges = Power level"): the exchange count is capped by
            // the acting power, exactly like every other area action --
            // checked before any exchange executes so a rejected move
            // mutates nothing.
            validate_chinatown_exchanges(exchanges, power).map_err(MoveError::RuleViolation)?;
            for exchange in exchanges {
                apply_chinatown_exchange(state, player, *exchange, &rules.techs).map_err(MoveError::RuleViolation)?;
            }
        }
        AreaAction::Church { copper, silk, tea, seafood, yen, imported_good } => {
            execute_church_action(
                state, player, power, *copper, *silk, *tea, *seafood, *yen, *imported_good, rules,
            ).map_err(MoveError::RuleViolation)?;
        }
        AreaAction::Customs { imported_goods_paid } => {
            execute_customs_action(state, player, power, *imported_goods_paid, rules)
                .map_err(MoveError::RuleViolation)?;
        }

        // Carry out the area action while declining all optional
        // components: grants nothing (no cards, goods, or VP — a 5-power
        // Port Skip does NOT get the +3 VP bonus, since that requires an
        // actual take). Legal only at the six areas with optional
        // components; phase bookkeeping otherwise proceeds exactly like a
        // substantive action (current_action_power and the acting area were
        // already set by the preceding Movement/mercenary step, and
        // Recovery/5-Power/Building gate off those regardless of which
        // AreaAction variant ran).
        AreaAction::Skip => {
            let area = if state.is_mercenary_action {
                state.mercenary_target_area.ok_or(MoveError::RuleViolation("No mercenary target area"))?
            } else {
                match state.player(player).president_location {
                    PresidentLocation::OnBoard(a) => a,
                    _ => return Err(MoveError::RuleViolation("President not on board")),
                }
            };
            let area_type = state.board_layout.areas.get(&area)
                .map(|c| c.area_type)
                .ok_or(MoveError::RuleViolation("Unknown acting area"))?;
            if !area_type.allows_area_action_skip() {
                return Err(MoveError::RuleViolation("Skip is not legal for this area"));
            }
        }
    }

    Ok(())
}

/// Pay passive income to Trading House owner (if not the current player).
/// When a player takes an area action in an area with another player's Trading House,
/// that Trading House owner receives 1 Yen.
fn apply_trading_house_passive_income(state: &mut GameState) {
    let player = state.current_player;
    let p = state.player(player);

    // Determine the action area (president location or mercenary target)
    let area = if state.is_mercenary_action {
        state.mercenary_target_area
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => Some(a),
            PresidentLocation::InHand => None,
        }
    };

    if let Some(area) = area {
        // Check if there's a trading house in this area owned by another player
        if let Some(site) = state.board_layout.building_sites.get(&area) {
            if let Some(th_owner) = site.trading_house_owner {
                // Pay 1 Yen to the owner if it's not the current player
                if th_owner != player {
                    state.player_mut(th_owner).money += 1;
                }
            }
        }
    }
}

/// Employment Agency: retrieve items from warehouse to hand/board.
fn apply_employment_agency(
    state: &mut GameState,
    player: PlayerId,
    power: u8,
    assistants: u8,
    shophouses: u8,
    trading_houses: u8,
) -> Result<(), MoveError> {
    let max_items = employment_agency_tokens_allowed(power);

    // Widened sum (Stage-4 census fix): the u8 sum panicked in debug and
    // wrapped in release for payloads like {200, 100, 0}, letting a wrapped
    // total sneak under the power cap.
    let total_items = assistants as u32 + shophouses as u32 + trading_houses as u32;
    // Task 17 ruling: {0,0,0} is no longer a legal encoding of "decline
    // Employment Agency" -- Skip is the sole canonical decline.
    if total_items == 0 {
        return Err(MoveError::RuleViolation("Must take at least 1 item (use Skip to decline)"));
    }
    if total_items > max_items as u32 {
        return Err(MoveError::RuleViolation("Exceeds maximum items for power level"));
    }

    let p = state.player(player);

    // Validate availability in warehouse
    if assistants > p.assistants.in_warehouse {
        return Err(MoveError::RuleViolation("Not enough assistants in warehouse"));
    }
    if shophouses > p.shophouses.in_warehouse {
        return Err(MoveError::RuleViolation("Not enough shophouses in warehouse"));
    }
    if trading_houses > p.trading_houses.in_warehouse {
        return Err(MoveError::RuleViolation("Not enough trading houses in warehouse"));
    }

    // Calculate total cost
    let shophouse_cost = shophouses as u32 * 2; // 2 Yen each

    // Trading house cost: the printed take progression (4/5/6/7), indexed
    // by how many tokens have already left the warehouse (hand + built).
    let already_taken = p.trading_houses.already_taken();
    let mut th_cost = 0u32;
    for i in 0..trading_houses {
        let th_index = already_taken + i as usize;
        th_cost += TradingHouseState::take_cost(th_index);
    }

    let total_cost = shophouse_cost + th_cost; // Assistants are FREE

    if p.money < total_cost {
        return Err(MoveError::RuleViolation("Not enough Yen to pay for items"));
    }

    // Apply the action
    let p = state.player_mut(player);
    p.money -= total_cost;

    // Move assistants: warehouse -> hand
    p.assistants.in_warehouse -= assistants;
    p.assistants.in_hand += assistants;

    // Move shophouses: warehouse -> hand
    p.shophouses.in_warehouse -= shophouses;
    p.shophouses.in_hand += shophouses;

    // Move trading houses: warehouse -> hand (built later, for free, in the
    // Building step - the cost was just charged above)
    p.trading_houses.in_warehouse -= trading_houses;
    p.trading_houses.in_hand += trading_houses;

    Ok(())
}

fn apply_take_5_power(state: &mut GameState, rules: &Rules) -> Result<(), MoveError> {
    let player = state.current_player;

    // Stage-4 raw-arm self-defense (Phase-2.9 census): the printed rule is
    // "at 5 Power, take the token" -- the raw arm used to grant it at any
    // power, and silently no-op when no token was there (Skip5PowerBonus is
    // the canonical encoding for "nothing to take"). The generator only
    // offers this move at power 5 with a token present, so nothing changes
    // for legal-list inputs.
    if state.current_action_power != Some(5) {
        return Err(MoveError::RuleViolation(
            "5-Power bonus requires the action to have been taken at power 5",
        ));
    }

    // For mercenary sub-turns, use the mercenary target area
    let area = resolve_acting_area(state, player).map_err(MoveError::RuleViolation)?;

    // `Option::take` on an empty/absent slot is a no-op, so the Err path
    // leaves state untouched despite the mutate-shaped call (Stage-4 review
    // F1: correctness relies on exactly that no-op).
    let effect = state
        .board_layout
        .five_power_tokens
        .get_mut(&area)
        .and_then(|slot| slot.take())
        .ok_or(MoveError::RuleViolation("No 5-Power token at the acting area"))?;

    // Mining Technology bonus (KB "applies to": 5-Power tokens) fires only
    // for the Copper variant. Computed here, after the board_layout borrow
    // above ends but before the player is mutably borrowed below.
    let mining_bonus = match effect {
        FivePowerEffect::Copper(n) => copper_bonus(state, player, n, &rules.techs),
        _ => 0,
    };

    let p = state.player_mut(player);
    p.five_power_tokens += 1;

    // Apply effect
    match effect {
        FivePowerEffect::ImportedGoods(n) => p.imported_goods += n as u32,
        FivePowerEffect::Copper(n) => p.copper += n as u32 + mining_bonus as u32,
        FivePowerEffect::Silk(n) => p.silk += n as u32,
        FivePowerEffect::Tea(n) => p.tea += n as u32,
        FivePowerEffect::Seafood(n) => p.seafood += n as u32,
        FivePowerEffect::Money(n) => p.money += n as u32,
    }

    Ok(())
}

fn apply_build_shophouse(state: &mut GameState, slot: u8, rules: &Rules) -> Result<(), MoveError> {
    // `slot` indexes the site's fixed `SPACES_PER_SITE` arrays twice below
    // (`spaces` for the reward, `shophouses_built` for ownership). Validate
    // FIRST so the raw arm returns a typed error rather than aborting the
    // process: checked `apply_move` masks this (the legal list never offers
    // an out-of-range slot), but `apply_move_unchecked` is public and
    // documented for future search inner loops, which cannot catch a panic.
    // Same self-defence precedent as EndAdditionalPhase / UseUniversity /
    // UseSpinningMill / apply_return_to_hand (C1, external review 2026-07-20).
    if slot as usize >= SPACES_PER_SITE {
        return Err(MoveError::InvalidSlot);
    }
    let player = state.current_player;
    let p = state.player(player);

    // For mercenary sub-turns, use the mercenary target area
    let area = if state.is_mercenary_action {
        state.mercenary_target_area.ok_or(MoveError::RuleViolation("No mercenary target area"))?
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => a,
            _ => return Err(MoveError::RuleViolation("President not on board")),
        }
    };

    // Occupancy + duplicate-ownership self-defence (Phase-2.9 divergence
    // fix, same class as the range check above): without these, an unchecked
    // apply onto an occupied slot silently OVERWROTE the other player's
    // shophouse, and a builder could stack a second own shophouse on the
    // site. Checked before any mutation -- zero mutation on error.
    if let Some(site) = state.board_layout.building_sites.get(&area) {
        if site.shophouses_built[slot as usize].is_some() {
            return Err(MoveError::RuleViolation("Shophouse slot already occupied"));
        }
        if site.shophouses_built.contains(&Some(player)) {
            return Err(MoveError::RuleViolation("Already own a shophouse on this site"));
        }
    }

    // Get the space's full reward (before the mutable borrow of the player).
    let reward = state
        .board_layout
        .building_sites
        .get(&area)
        .map(|site| site.spaces[slot as usize])
        .unwrap_or_default();

    // Mining Technology bonus (KB "applies to": Building Site rewards) fires
    // on the EXPLICIT copper field only. any_good no longer auto-resolves to
    // copper -- it queues a player choice instead, and (if later resolved to
    // Copper) is its OWN separate receipt, mining-bonused independently at
    // resolution time (design 2026-07-18 §5.3-2). Computed before the player
    // is mutably borrowed below.
    let mining_bonus = copper_bonus(state, player, reward.copper, &rules.techs);
    // Gaslight (C4 final-review fix): pure passive, no player choice, same
    // class as Winery/Brickyard - computed before the mutable borrow below.
    let gaslight_bonus = building_vp_bonus(state, player, true, &rules.techs);

    let p = state.player_mut(player);
    if p.shophouses.in_hand < 1 {
        return Err(MoveError::RuleViolation("No shophouse in hand"));
    }
    p.shophouses.in_hand -= 1;
    p.shophouses.built.insert(area, slot);
    apply_space_reward(state, player, &reward, mining_bonus, rules);
    state.player_mut(player).victory_points += gaslight_bonus;

    // Update building site
    if let Some(site) = state.board_layout.building_sites.get_mut(&area) {
        site.shophouses_built[slot as usize] = Some(player);
    }

    Ok(())
}

fn apply_build_trading_house(state: &mut GameState, rules: &Rules) -> Result<(), MoveError> {
    let player = state.current_player;
    let p = state.player(player);

    // For mercenary sub-turns, use the mercenary target area
    let area = if state.is_mercenary_action {
        state.mercenary_target_area.ok_or(MoveError::RuleViolation("No mercenary target area"))?
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => a,
            _ => return Err(MoveError::RuleViolation("President not on board")),
        }
    };

    // Occupancy self-defence (Stage-4, Phase-2.9 census: the shophouse
    // overwrite bug class, alive on this arm): without this, an unchecked
    // apply onto an occupied Trading House space silently REPLACED the
    // standing trading house (any owner's -- each site has exactly ONE TH
    // space, so `trading_house_owner: Some(_)` means it's taken). Checked
    // before any mutation -- zero mutation on error. A missing site is
    // tolerated exactly the way the shophouse arm tolerates it (reward
    // defaults; the site update below is `if let Some`).
    if let Some(site) = state.board_layout.building_sites.get(&area) {
        if site.trading_house_owner.is_some() {
            return Err(MoveError::RuleViolation("Trading House space already occupied"));
        }
    }

    let vp = state
        .board_layout
        .building_sites
        .get(&area)
        .map(|site| site.trading_house_reward)
        .unwrap_or(0);
    // Electrical Light (C4 final-review fix): pure passive, no player
    // choice, same class as Winery/Brickyard - computed before the mutable
    // borrow below.
    let electrical_light_bonus = building_vp_bonus(state, player, false, &rules.techs);

    let p = state.player_mut(player);
    if p.trading_houses.in_hand < 1 {
        return Err(MoveError::RuleViolation("No trading house in hand"));
    }

    // Building is FREE - the take progression (4/5/6/7) was paid at the
    // Employment Agency when the token moved warehouse -> hand
    // (rulebook p.7: "Place a Trading House token from your hand").
    p.trading_houses.in_hand -= 1;
    p.trading_houses.built.push(area);
    p.victory_points += vp as u32 + electrical_light_bonus;

    if let Some(site) = state.board_layout.building_sites.get_mut(&area) {
        site.trading_house_owner = Some(player);
    }

    Ok(())
}

/// Apply recovery phase, optionally leaving 1 assistant (Telephone effect).
fn apply_recovery(state: &mut GameState) {
    apply_recovery_with_leave(state, false);
}

/// Apply recovery with explicit control over leaving an assistant.
/// If `leave_one` is true and player has Telephone tech, leaves 1 assistant.
pub fn apply_recovery_with_leave(state: &mut GameState, leave_one: bool) {
    let player = state.current_player;
    let p = state.player(player);

    // During mercenary sub-turn, recover from mercenary target area
    // During normal turn, recover from president's location
    let recovery_area = if state.is_mercenary_action {
        state.mercenary_target_area
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => Some(a),
            PresidentLocation::InHand => None,
        }
    };

    if let Some(area) = recovery_area {
        let p = state.player_mut(player);
        if let Some(&count) = p.assistants.on_board.get(&area) {
            if leave_one && count >= 1 {
                // Telephone effect: leave exactly 1 assistant, even at
                // count == 1 -- which recovers zero (owner-adjudicated rules
                // fix, design 2026-07-18 §5.3-5; the old `count > 1` guard
                // wrongly force-recovered a lone assistant).
                p.assistants.in_hand += count - 1;
                p.assistants.on_board.insert(area, 1);
            } else {
                // Normal recovery: take all assistants
                p.assistants.in_hand += count;
                p.assistants.on_board.remove(&area);
            }
        }
    }
}

/// Recovery-phase entry point (design 2026-07-18 §5.1-5.3, §5.3-5): if the
/// current player owns Telephone AND has >=1 own assistant in the acting
/// area, defer to a `RecoveryLeave` choice instead of recovering
/// automatically -- turn_phase stays `Recovery` until it resolves
/// (`get_legal_moves`' queue-first gate then surfaces the two
/// `ChooseRecoveryLeave` options). Otherwise Recovery stays fully
/// automatic: recover everyone and run the phase transition immediately.
/// The acting area is resolved via the same shared `resolve_acting_area`
/// helper `apply_recovery_with_leave` mirrors inline -- an `Err` (President
/// in hand, no mercenary sub-turn) means no acting area, so there is
/// nothing to leave behind either way and Recovery is a no-op.
fn resolve_recovery(state: &mut GameState, rules: &Rules) {
    let player = state.current_player;
    let has_telephone = state.player(player).technologies.iter().any(|&tech_id| {
        rules.techs.get(tech_id).map(|t| t.effect == TechEffect::Telephone).unwrap_or(false)
    });

    if has_telephone {
        if let Ok(area) = resolve_acting_area(state, player) {
            if state.player(player).assistants.in_area(area) >= 1 {
                push_choice(state, PendingChoice::RecoveryLeave, rules);
                return;
            }
        }
    }

    apply_recovery(state);
    finish_recovery_transition(state);
}

/// Recovery-complete phase transition: return from a mercenary sub-turn to
/// its stored window (with the sub-turn state cleanup), or advance a normal
/// turn to `AdditionalActionSecond`. Extracted verbatim from the
/// end-of-`apply_move` automation block (Task 6) so both the automatic path
/// (`resolve_recovery`) and the `ChooseRecoveryLeave` resolution
/// (`game::choices::resolve_choice_move`) share the exact same transition +
/// cleanup instead of drifting apart.
pub fn finish_recovery_transition(state: &mut GameState) {
    if state.is_mercenary_action {
        // Return to the Additional Action phase we came from
        if let Some(return_phase) = state.mercenary_return_phase {
            state.turn_phase = return_phase;
        } else {
            // Fallback: if return phase not set, go to AdditionalActionSecond
            state.turn_phase = TurnPhase::AdditionalActionSecond;
        }
        // Reset mercenary sub-turn state
        state.is_mercenary_action = false;
        state.mercenary_target_area = None;
        state.mercenary_return_phase = None;
        state.current_action_power = None;
    } else {
        // Normal turn flow: Recovery -> AdditionalActionSecond
        state.turn_phase = TurnPhase::AdditionalActionSecond;
    }
}

fn apply_mercenary(
    state: &mut GameState,
    country: Country,
    target_area: AreaId,
) -> Result<(), MoveError> {
    let player = state.current_player;
    let p = state.player(player);

    // Validate: mercenary not already used this turn
    if state.additional_action_state.mercenary_used {
        return Err(MoveError::RuleViolation("Mercenary already used this turn"));
    }

    // Validate: player has this mercenary available
    let available = p.mercenaries.available.get(&country).copied().unwrap_or(0);
    if available == 0 {
        return Err(MoveError::RuleViolation("No available mercenary of this country"));
    }

    // Validate: target area has player's assistant but NOT president
    let assistant_count = p.assistants.on_board.get(&target_area).copied().unwrap_or(0);
    if assistant_count == 0 {
        return Err(MoveError::RuleViolation("Target area must have your assistant"));
    }
    if let PresidentLocation::OnBoard(loc) = p.president_location {
        if loc == target_area {
            return Err(MoveError::RuleViolation("Target area cannot have your president"));
        }
    }

    // Calculate power at target area for the mercenary action
    // Power = Assistants + Shophouse + TradingHouse + Station (NO president)
    let mut power: u8 = assistant_count;
    if p.shophouses.built.contains_key(&target_area) {
        power += 1;
    }
    if p.trading_houses.built.contains(&target_area) {
        power += 1;
    }
    if state.station_location == Some(target_area) {
        power += 1;
    }
    power = power.min(5);

    // Consume the mercenary (flip face-down)
    let p_mut = state.player_mut(player);
    *p_mut.mercenaries.available.entry(country).or_insert(0) -= 1;
    *p_mut.mercenaries.used.entry(country).or_insert(0) += 1;

    // Mark mercenary as used for this turn
    state.additional_action_state.mercenary_used = true;

    // Store return phase (where to go after sub-turn completes)
    state.mercenary_return_phase = Some(state.turn_phase);

    // Set up mercenary sub-turn state
    state.is_mercenary_action = true;
    state.mercenary_target_area = Some(target_area);
    state.current_action_power = Some(power);

    // Transition to AreaAction phase
    state.turn_phase = TurnPhase::AreaAction;

    Ok(())
}

/// Applies a Shophouse space's full reward when a Shophouse is built there.
/// `mining_bonus` is the caller-precomputed Mining Technology flat bonus (see
/// `copper_bonus`) for the reward's EXPLICIT copper field only; any_good no
/// longer auto-resolves to copper here -- it queues an `AnyGood` choice per
/// icon instead (design 2026-07-18 §5.1/§5.3-2), which is its own separate
/// receipt if later resolved to Copper (see `resolve_choice_move`). Likewise
/// warehouse_tokens queues a `WarehouseTake` choice per icon (§5.2), and
/// movement queues an `AssistantMove` choice per icon (§5.3) instead of
/// being a no-op -- all three push through `push_choice` so a singleton
/// option set (e.g. an empty warehouse) auto-resolves uniformly.
fn apply_space_reward(state: &mut GameState, player: PlayerId, reward: &SpaceReward, mining_bonus: u8, rules: &Rules) {
    let p = state.player_mut(player);
    p.victory_points += reward.vp as u32;
    p.money += reward.yen as u32;
    p.seafood += reward.seafood as u32;
    p.tea += reward.tea as u32;
    p.silk += reward.silk as u32;
    p.copper += reward.copper as u32 + mining_bonus as u32;
    for _ in 0..reward.warehouse_tokens {
        push_choice(state, PendingChoice::WarehouseTake, rules);
    }
    for _ in 0..reward.any_good {
        push_choice(state, PendingChoice::AnyGood, rules);
    }
    for _ in 0..reward.movement {
        push_choice(state, PendingChoice::AssistantMove, rules);
    }
}

fn apply_fulfill_order(
    state: &mut GameState,
    order_id: OrderCardId,
    rules: &Rules,
) -> Result<(), MoveError> {
    let player = state.current_player;
    let p = state.player(player);

    // Verify order is in player's hand
    if !p.order_cards_in_hand.contains(&order_id) {
        return Err(MoveError::RuleViolation("Order card not in hand"));
    }

    // Look up order details from registry
    let order = match rules.orders.get(order_id) {
        Some(o) => o,
        None => return Err(MoveError::RuleViolation("Order card not found in registry")),
    };

    // Clone requirements, rewards and country before mutable borrow
    let req = order.requirements;
    let rewards = order.rewards;
    let country = order.country;

    // Check if player has required trade goods
    if p.copper < req.copper as u32
        || p.silk < req.silk as u32
        || p.tea < req.tea as u32
        || p.seafood < req.seafood as u32
    {
        return Err(MoveError::RuleViolation("Insufficient trade goods to fulfill order"));
    }

    // Mining Technology bonus for the EXPLICIT copper reward -- its own
    // receipt, fired here at the triggering move. Any any_trade_good the
    // player later resolves to Copper is a SEPARATE receipt, mining-bonused
    // independently at choice-resolution time (design 2026-07-18 §5.3-2).
    let mining_bonus = copper_bonus(state, player, rewards.copper, &rules.techs);

    // Deduct trade goods
    let p = state.player_mut(player);
    p.copper -= req.copper as u32;
    p.silk -= req.silk as u32;
    p.tea -= req.tea as u32;
    p.seafood -= req.seafood as u32;

    // Award rewards
    p.victory_points += rewards.victory_points as u32;
    p.money += rewards.money as u32;
    p.imported_goods += rewards.imported_goods as u32;
    p.copper += rewards.copper as u32 + mining_bonus as u32;
    p.silk += rewards.silk as u32;
    p.tea += rewards.tea as u32;
    p.seafood += rewards.seafood as u32;

    // Move order from hand to completed
    p.order_cards_in_hand.retain(|&id| id != order_id);
    p.completed_orders.push(order_id);

    // Warehouse-token reward: one WarehouseTake choice per icon (design 2026-07-18 §5.2).
    for _ in 0..rewards.warehouse_tokens {
        push_choice(state, PendingChoice::WarehouseTake, rules);
    }

    // Any-good reward: one AnyGood choice per icon (design 2026-07-18 §5.1).
    for _ in 0..rewards.any_trade_good {
        push_choice(state, PendingChoice::AnyGood, rules);
    }

    // Movement reward: one AssistantMove choice per icon (design 2026-07-18 §5.3).
    for _ in 0..rewards.movement {
        push_choice(state, PendingChoice::AssistantMove, rules);
    }

    // Ball (+2 VP) and Language School (+1 Yen) trigger on every completed
    // Order, stacking per owned physical copy (see ball_vp_bonus /
    // language_school_yen_bonus).
    let ball_bonus = ball_vp_bonus(state, player, &rules.techs);
    state.player_mut(player).victory_points += ball_bonus;
    let language_school_bonus = language_school_yen_bonus(state, player, &rules.techs);
    state.player_mut(player).money += language_school_bonus;

    // Foreign Mercenary eligibility (rulebook p.8 "Matching Country icons"):
    // re-run the same country-icon check Lab acquisition uses
    // (apply_lab_acquisition_bonuses), for the just-completed order's country.
    check_mercenary_eligibility(state, player, country, &rules.orders, &rules.techs);

    Ok(())
}

fn apply_fulfill_achievement(
    state: &mut GameState,
    achievement: AchievementType,
) -> Result<(), MoveError> {
    let player = state.current_player;
    // Stage-4 raw-arm self-defense (Phase-2.9 census): only the game's 3
    // drawn achievement cards are claimable -- `claim_achievement` used to
    // fall through its card-search loop for an undrawn type, locking an
    // assistant against a card that is not in this game.
    if !state.achievements.iter().any(|c| c.achievement_type == achievement) {
        return Err(MoveError::RuleViolation(
            "Achievement is not among this game's drawn achievement cards",
        ));
    }
    claim_achievement(state, player, achievement).map_err(MoveError::RuleViolation)?;
    Ok(())
}

// End-game trigger detection lives in end_game.rs (check_end_game_triggers,
// all FIVE triggers incl. OrderDeckEmpty). A private 4-trigger copy here
// used to shadow it; deleted after fuzz surfaced 500-turn non-terminations.

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn create_minimal_state(player_count: u8) -> GameState {
        let colors = match player_count {
            2 => vec![PlayerColor::Red, PlayerColor::Blue],
            3 => vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow],
            4 => vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green],
            _ => vec![PlayerColor::Red, PlayerColor::Blue],
        };
        let actual_count = colors.len() as u8;

        let config = GameConfig::new(actual_count, colors, PlayerId::new(0));

        let board_layout = BoardLayout {
            areas: BTreeMap::new(),
            adjacency: BTreeMap::new(),
            canal_crossings: vec![],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };

        let players: Vec<PlayerState> = (0..actual_count)
            .map(|i| {
                let color = match i {
                    0 => PlayerColor::Red,
                    1 => PlayerColor::Blue,
                    2 => PlayerColor::Yellow,
                    _ => PlayerColor::Green,
                };
                PlayerState::new(PlayerId::new(i), color, i == 0)
            })
            .collect();

        GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::AdditionalActionFirst,
            turn_number: 1,
            additional_action_state: AdditionalActionState::new(),
            current_action_power: None,
            is_mercenary_action: false,
            mercenary_target_area: None,
            mercenary_return_phase: None,
            president_start_area: None,
            port_state: PortState::new(player_count >= 3),
            lab_state: LabState::new(player_count >= 4),
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

    #[test]
    fn test_phase_advance() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;

        advance_phase(&mut state, None);
        assert_eq!(state.turn_phase, TurnPhase::Placement);
    }

    /// I1(a) final-review fix (design §5.1 queue invariant): apply_move is
    /// the pure authoritative API (a Phase-3 bot, or a tampered record, may
    /// call it directly, not just via legal-move pre-filtering) -- ending
    /// the additional-action phase with a choice still queued would
    /// advance the turn (hand off to the next player, in the
    /// AdditionalActionSecond case) while a reward from THIS turn is still
    /// unresolved, so a later resolution of the stale queue would
    /// misgrant it to the wrong player. Mirrors UseUniversity/
    /// UseSpinningMill's self-defended once-per-turn gate (T7) / apply_mercenary's
    /// self-defended mercenary_used gate: the queue-first gate in
    /// get_legal_moves never offers EndAdditionalPhase while
    /// pending_choices is non-empty, so this never actually fires under
    /// legal-list play.
    #[test]
    fn end_additional_phase_rejected_with_pending_choice_queued() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        state.pending_choices.push(PendingChoice::AnyGood);
        let rules = Rules::empty_for_tests();

        // apply_move_unchecked: per the doc comment above, the queue-first
        // gate in get_legal_moves never offers EndAdditionalPhase while a
        // choice is queued, so this never fires under checked apply_move --
        // this test targets the raw arm's self-defense directly.
        let result = apply_move_unchecked(&state, &Move::EndAdditionalPhase, &rules);
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        assert_eq!(state.turn_phase, TurnPhase::AdditionalActionFirst, "original state must be untouched");
    }

    #[test]
    fn test_advance_phase_applies_system_of_stock_at_the_incoming_players_turn_start() {
        // KB 08-tech-cards.yaml tech_stock_market ("System of Stock" in the
        // rulebook): "Start of Your Turn: if you have less than 2 yen,
        // increase to 2 yen" - fully automatic, no player choice. Pins that
        // advance_phase's next-player transition is the wiring point.
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionSecond;
        state.current_player = PlayerId::new(0);
        state.players[1].technologies.push(TechCardId(1));
        state.players[1].money = 0;

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(
            TechCard { id: TechCardId(1), name: "Stock Market".to_string(), industry_cost: 3, country: Country::Dutch, effect: TechEffect::SystemOfStock },
            "tech_stock_market",
        );

        advance_phase(&mut state, Some(&tech_registry));

        assert_eq!(state.current_player, PlayerId::new(1));
        assert_eq!(state.players[1].money, 2);
    }

    #[test]
    fn test_final_round_completion_reaches_turn_end() {
        // Last player of the final round finishing their turn: the phase
        // machine must land on TurnEnd so is_game_over can ever be true
        // (rulebook p.9: finish the trigger round, one full final round,
        // equal turns for all).
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionSecond;
        state.end_game_triggered = true;
        state.final_round = true;
        // check_end_game_triggers always records the trigger player alongside
        // the flag; is_game_over requires it.
        state.trigger_player = Some(PlayerId::new(0));
        // With 2 players and start_player 0, current 1 -> next_player() == 0 == start.
        state.current_player = PlayerId::new(1);

        advance_phase(&mut state, None);

        assert_eq!(state.turn_phase, TurnPhase::TurnEnd);
        assert!(state.is_game_over());
    }

    #[test]
    fn test_employment_agency_th_take_moves_token_to_hand_with_take_progression() {
        // Rulebook: Employment Agency moves tokens warehouse -> hand, paying
        // the printed take progression (4/5/6/7 per trading house taken);
        // Building later places from HAND for free. The old model charged
        // at build time and never delivered the employment take at all.
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::EmploymentAgency,
            grid_position: (0, 0),
        });
        state.turn_phase = TurnPhase::AreaAction;
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(5); // allowed 4 items
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].money = 9;
        state.players[0].trading_houses.in_warehouse = 4;
        state.players[0].trading_houses.in_hand = 0;
        let rules = Rules::empty_for_tests();

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::EmploymentAgency { assistants: 0, shophouses: 0, trading_houses: 2 },
        };
        let next = apply_move(&state, &mv, &rules).unwrap();
        let th = &next.players[0].trading_houses;

        assert_eq!(th.in_warehouse, 2);
        assert_eq!(th.in_hand, 2, "taken tokens must land in hand");
        assert_eq!(next.players[0].money, 0, "1st take costs 4, 2nd costs 5");
        assert_eq!(th.total_built(), 0);
    }

    #[test]
    fn test_build_trading_house_places_from_hand_free() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.board_layout.building_sites.insert(area, BuildingSite::new([1, 2, 3, 4], 7));
        state.turn_phase = TurnPhase::Building;
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(5);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].money = 0; // build is FREE - cost was paid at Employment
        state.players[0].trading_houses.in_hand = 1;
        state.players[0].trading_houses.in_warehouse = 3;
        let rules = Rules::empty_for_tests();

        let next = apply_move(&state, &Move::BuildTradingHouse, &rules).unwrap();
        let th = &next.players[0].trading_houses;

        assert_eq!(th.in_hand, 0);
        assert_eq!(th.in_warehouse, 3, "warehouse untouched by building");
        assert_eq!(th.built, vec![area]);
        assert_eq!(next.players[0].money, 0, "no charge at build time");
        assert_eq!(next.players[0].victory_points, 7);
    }

    fn electrical_light_tech_fixture(id: u8) -> TechCard {
        TechCard {
            id: TechCardId(id),
            name: "Electrical Light".to_string(),
            country: Country::French,
            industry_cost: 4,
            effect: TechEffect::ElectricalLight,
        }
    }

    #[test]
    fn test_build_trading_house_electrical_light_vp_bonus() {
        // C4 final-review fix: Electrical Light (+3 VP per owned copy when
        // building a Trading House) was computed by building_vp_bonus but
        // never called from apply_build_trading_house.
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.board_layout.building_sites.insert(area, BuildingSite::new([1, 2, 3, 4], 7));
        state.turn_phase = TurnPhase::Building;
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(5);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].money = 0;
        state.players[0].trading_houses.in_hand = 1;

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(electrical_light_tech_fixture(1), "tech_electrical_light");
        state.players[0].technologies.push(TechCardId(1));

        let next = apply_move(&state, &Move::BuildTradingHouse, &rules).unwrap();
        // 7 (site VP) + 3 (Electrical Light, 1 owned copy).
        assert_eq!(next.players[0].victory_points, 10);
    }

    #[test]
    fn test_build_trading_house_electrical_light_bonus_stacks_per_owned_copy() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.board_layout.building_sites.insert(area, BuildingSite::new([1, 2, 3, 4], 7));
        state.turn_phase = TurnPhase::Building;
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(5);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].money = 0;
        state.players[0].trading_houses.in_hand = 1;

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(electrical_light_tech_fixture(1), "tech_electrical_light_1");
        rules.techs.insert(electrical_light_tech_fixture(2), "tech_electrical_light_2");
        state.players[0].technologies.push(TechCardId(1));
        state.players[0].technologies.push(TechCardId(2));

        let next = apply_move(&state, &Move::BuildTradingHouse, &rules).unwrap();
        // 7 (site VP) + 6 (Electrical Light, 2 owned copies x 3 VP).
        assert_eq!(next.players[0].victory_points, 13);
    }

    #[test]
    fn test_build_trading_house_requires_token_in_hand() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.board_layout.building_sites.insert(area, BuildingSite::new([1, 2, 3, 4], 7));
        state.turn_phase = TurnPhase::Building;
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(5);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].money = 100;
        state.players[0].trading_houses.in_hand = 0;
        state.players[0].trading_houses.in_warehouse = 4;
        let rules = Rules::empty_for_tests();

        assert!(apply_move(&state, &Move::BuildTradingHouse, &rules).is_err());
    }

    #[test]
    fn test_order_deck_empty_trigger_fires_through_apply_move() {
        // The complete 5-trigger checker lives in end_game.rs; apply_move
        // used to shadow it with a private 4-trigger copy that could never
        // fire OrderDeckEmpty (found by fuzz: 500-turn non-terminations
        // with trigger_reason null).
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        state.order_deck = vec![];
        // port_a's six slots start empty in this fixture
        let rules = Rules::empty_for_tests();

        let next = apply_move(&state, &Move::EndAdditionalPhase, &rules).unwrap();

        assert!(next.end_game_triggered);
        assert!(matches!(next.trigger_reason, Some(EndGameTrigger::OrderDeckEmpty)));
    }

    fn movement_toll_fixture() -> GameState {
        // Chain 0 - 1 - 2: president at 0, opponent president at 1 (toll),
        // own assistants at 1 and 2 so 2 is reachable through 1.
        let mut state = create_minimal_state(2);
        for (id, ty) in [(0, AreaType::Bank), (1, AreaType::CopperMine), (2, AreaType::SilkMill)] {
            state.board_layout.areas.insert(
                AreaId::new(id),
                AreaConfig { area_type: ty, grid_position: (0, id) },
            );
        }
        state.board_layout.adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        state.board_layout.adjacency.insert(AreaId::new(1), vec![AreaId::new(0), AreaId::new(2)]);
        state.board_layout.adjacency.insert(AreaId::new(2), vec![AreaId::new(1)]);
        state.turn_phase = TurnPhase::Movement;
        state.current_player = PlayerId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);
        state.players[0].assistants.on_board.insert(AreaId::new(2), 1);
        state.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));
        state
    }

    #[test]
    fn test_movement_pass_through_fee_paid_to_president_owner() {
        let mut state = movement_toll_fixture();
        state.players[0].money = 2;
        state.players[1].money = 0;
        let rules = Rules::empty_for_tests();

        let mv = Move::MovePresident {
            path: vec![AreaId::new(1), AreaId::new(2)],
            destination: AreaId::new(2),
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        assert_eq!(next.players[0].money, 1);
        assert_eq!(
            next.players[1].money, 1,
            "pass-through fee is paid TO the president's owner, not the bank"
        );
    }

    #[test]
    fn test_movement_with_stagecoach_is_free_and_uncredited() {
        // Stagecoach waives president pass-through fees entirely; apply must
        // price with the same tech-aware cost the generator uses so a
        // Stagecoach owner's generated move applies even at 0 money.
        let mut state = movement_toll_fixture();
        state.players[0].money = 0;
        state.players[1].money = 0;
        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(
            TechCard {
                id: TechCardId(1),
                name: "Stagecoach".to_string(),
                country: Country::German,
                industry_cost: 3,
                effect: TechEffect::Stagecoach,
            },
            "T1",
        );
        state.players[0].technologies.push(TechCardId(1));

        let mv = Move::MovePresident {
            path: vec![AreaId::new(1), AreaId::new(2)],
            destination: AreaId::new(2),
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        assert_eq!(next.players[0].money, 0);
        assert_eq!(next.players[1].money, 0, "no fee waived by Stagecoach is credited");
    }

    #[test]
    fn test_trigger_round_completion_starts_final_round() {
        // Trigger fired mid-round: when the turn passes back to the start
        // player, final_round begins (and the game does NOT end yet).
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionSecond;
        state.end_game_triggered = true;
        state.final_round = false;
        state.current_player = PlayerId::new(1);

        advance_phase(&mut state, None);

        assert!(state.final_round);
        assert_eq!(state.turn_phase, TurnPhase::AdditionalActionFirst);
        assert_eq!(state.current_player, PlayerId::new(0));
        assert!(!state.is_game_over());
    }

    #[test]
    fn test_placement_decrements_hand() {
        let mut state = create_minimal_state(2);

        // Add an area
        state.board_layout.areas.insert(AreaId::new(0), AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });

        let initial_hand = state.player(PlayerId::new(0)).assistants.in_hand;

        let result = apply_placement(&mut state, &[AreaId::new(0)]);
        assert!(result.is_ok());

        assert_eq!(
            state.player(PlayerId::new(0)).assistants.in_hand,
            initial_hand - 1
        );
    }

    #[test]
    fn test_placement_charges_fee_to_payer_and_credits_single_owner() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.turn_phase = TurnPhase::Placement;
        state.players[1].president_location = PresidentLocation::OnBoard(area);
        state.players[0].money = 3;
        let owner_money_before = state.players[1].money;

        let rules = Rules::empty_for_tests();
        let result = apply_move(&state, &Move::PlaceThreeAreas { placements: vec![area] }, &rules);
        assert!(result.is_ok());
        let next = result.unwrap();

        assert_eq!(next.players[0].money, 2);
        assert_eq!(next.players[1].money, owner_money_before + 1);
    }

    #[test]
    fn test_placement_insufficient_money_rejected_zero_mutation() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.players[1].president_location = PresidentLocation::OnBoard(area);
        state.players[0].money = 0;

        let hand_before = state.players[0].assistants.in_hand;
        let board_before = state.players[0].assistants.on_board.clone();
        let money_before = state.players[0].money;
        let owner_money_before = state.players[1].money;

        let result = apply_placement(&mut state, &[area]);
        assert_eq!(result, Err(MoveError::NotEnoughMoney));

        // Zero mutation: a rejected placement must leave everything as it was.
        assert_eq!(state.players[0].assistants.in_hand, hand_before);
        assert_eq!(state.players[0].assistants.on_board, board_before);
        assert_eq!(state.players[0].money, money_before);
        assert_eq!(state.players[1].money, owner_money_before);
    }

    #[test]
    fn test_placement_own_president_in_area_costs_nothing_no_self_credit() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.turn_phase = TurnPhase::Placement;
        // The CURRENT player's own President is already in the target area.
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].money = 3;

        let rules = Rules::empty_for_tests();
        let result = apply_move(&state, &Move::PlaceThreeAreas { placements: vec![area] }, &rules);
        assert!(result.is_ok());
        let next = result.unwrap();

        // Own President in the area is not an opponent President: no fee, no self-credit.
        assert_eq!(next.players[0].money, 3);
    }

    #[test]
    fn test_placement_money_exactly_equal_to_fee_succeeds_with_zero_left() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.turn_phase = TurnPhase::Placement;
        state.players[1].president_location = PresidentLocation::OnBoard(area);
        // Fee for 1 opponent President in the area is exactly 1 Yen.
        state.players[0].money = 1;
        let owner_money_before = state.players[1].money;

        let rules = Rules::empty_for_tests();
        let result = apply_move(&state, &Move::PlaceThreeAreas { placements: vec![area] }, &rules);
        assert!(result.is_ok());
        let next = result.unwrap();

        assert_eq!(next.players[0].money, 0);
        assert_eq!(next.players[1].money, owner_money_before + 1);
    }

    #[test]
    fn test_placement_multi_president_area_charges_and_credits_each_owner() {
        // Two different opponents' Presidents both sit in the same area.
        let mut state = create_minimal_state(3);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.turn_phase = TurnPhase::Placement;
        state.players[1].president_location = PresidentLocation::OnBoard(area);
        state.players[2].president_location = PresidentLocation::OnBoard(area);
        state.players[0].money = 5;
        let owner1_money_before = state.players[1].money;
        let owner2_money_before = state.players[2].money;

        let rules = Rules::empty_for_tests();
        let result = apply_move(&state, &Move::PlaceThreeAreas { placements: vec![area] }, &rules);
        assert!(result.is_ok());
        let next = result.unwrap();

        assert_eq!(next.players[0].money, 3); // paid 1 to each of 2 owners
        assert_eq!(next.players[1].money, owner1_money_before + 1);
        assert_eq!(next.players[2].money, owner2_money_before + 1);
    }

    #[test]
    fn test_place_two_same_area_charges_fee_per_assistant_placed() {
        // KB (01-rules-reference.md:97) / rulebook p.5: pay 1 Yen to the
        // President's owner PER ASSISTANT PLACED. Option B places 2
        // assistants in `area`, so 1 opponent President there means a fee of
        // 2, not 1 (C1 final-review finding: the plan's Task 15 instruction
        // baked in the pre-existing mispricing, and both the payer's charge
        // and the owner's credit were wrong).
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.turn_phase = TurnPhase::Placement;
        state.players[1].president_location = PresidentLocation::OnBoard(area);
        state.players[0].money = 3;
        let owner_money_before = state.players[1].money;

        let rules = Rules::empty_for_tests();
        let result = apply_move(
            &state,
            &Move::PlaceTwoSameArea { area, adjacent_bonus: None },
            &rules,
        );
        assert!(result.is_ok());
        let next = result.unwrap();

        // 2 assistants land in `area` with 1 opponent President there: fee
        // is 1 Yen PER assistant placed = 2 total.
        assert_eq!(next.players[0].money, 1);
        assert_eq!(next.players[1].money, owner_money_before + 2);
        assert_eq!(next.players[0].assistants.on_board.get(&area), Some(&2));
    }

    #[test]
    fn test_place_two_same_area_rejects_when_money_insufficient_for_per_assistant_fee() {
        // Same fixture as above but with only 1 Yen: the correct fee (2) is
        // unaffordable even though the old (buggy) per-area-flat fee (1)
        // would have been affordable.
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.turn_phase = TurnPhase::Placement;
        state.players[1].president_location = PresidentLocation::OnBoard(area);
        state.players[0].money = 1;

        // apply_move_unchecked: this exact placement is unaffordable, so
        // get_legal_moves never offers it -- checked apply_move would
        // reject with NotLegal before reaching the raw NotEnoughMoney arm
        // under test.
        let rules = Rules::empty_for_tests();
        let result = apply_move_unchecked(
            &state,
            &Move::PlaceTwoSameArea { area, adjacent_bonus: None },
            &rules,
        );
        assert!(matches!(result, Err(MoveError::NotEnoughMoney)));
    }

    #[test]
    fn test_place_two_same_area_telegram_bonus_charges_both_areas() {
        let mut state = create_minimal_state(3);
        let main_area = AreaId::new(0);
        let adj_area = AreaId::new(1);
        state.board_layout.areas.insert(main_area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.board_layout.areas.insert(adj_area, AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 1),
        });
        state.board_layout.adjacency.insert(main_area, vec![adj_area]);
        state.board_layout.adjacency.insert(adj_area, vec![main_area]);
        state.turn_phase = TurnPhase::Placement;
        state.players[1].president_location = PresidentLocation::OnBoard(main_area);
        state.players[2].president_location = PresidentLocation::OnBoard(adj_area);
        state.players[0].money = 5;
        let owner1_money_before = state.players[1].money;
        let owner2_money_before = state.players[2].money;

        // apply_move_unchecked keeps this test targeting the raw fee math
        // of the bonus arm directly. Since the Stage-4 raw-arm self-defense
        // wave, that arm ALSO structurally validates the bonus (Telegram
        // ownership + adjacency), so the fixture now equips both -- the
        // assertions below stay purely about the fee arithmetic.
        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(telegram_tech_fixture(1), "tech_telegram");
        state.players[0].technologies.push(TechCardId(1));
        let result = apply_move_unchecked(
            &state,
            &Move::PlaceTwoSameArea { area: main_area, adjacent_bonus: Some(adj_area) },
            &rules,
        );
        assert!(result.is_ok());
        let next = result.unwrap();

        // main_area gets 2 assistants (fee 2, 1 per assistant) + adj_area
        // gets 1 assistant via the Telegram bonus (fee 1) = 3 total.
        assert_eq!(next.players[0].money, 2);
        assert_eq!(next.players[1].money, owner1_money_before + 2);
        assert_eq!(next.players[2].money, owner2_money_before + 1);
        assert_eq!(next.players[0].assistants.on_board.get(&main_area), Some(&2));
        assert_eq!(next.players[0].assistants.on_board.get(&adj_area), Some(&1));
    }

    #[test]
    fn test_mercenary_sub_turn_setup() {
        let mut state = create_minimal_state(2);
        let player = state.current_player;
        let target = AreaId::new(5);

        // Give player a mercenary and assistant at target area
        {
            let p = state.player_mut(player);
            p.mercenaries.available.insert(Country::English, 1);
            p.assistants.on_board.insert(target, 2);
            p.president_location = PresidentLocation::OnBoard(AreaId::new(1));
        }

        let result = apply_mercenary(&mut state, Country::English, target);
        assert!(result.is_ok());

        // Verify sub-turn state
        assert!(state.is_mercenary_action);
        assert_eq!(state.mercenary_target_area, Some(target));
        assert_eq!(state.turn_phase, TurnPhase::AreaAction);
    }

    #[test]
    fn test_mercenary_sub_turn_ignores_opponent_president_at_target_area() {
        // "Mercenaries bypass opponent Presidents (no blocking, no fees)"
        // (CLAUDE.md commonly-missed rule) - pinned through the real
        // production path. apply_mercenary's own validation checks only
        // that the MOVER'S OWN President isn't at the target area; an
        // opponent's President there is not checked at all, so the
        // sub-turn must start normally regardless.
        let mut state = create_minimal_state(2);
        let player = state.current_player;
        let target = AreaId::new(5);

        {
            let p = state.player_mut(player);
            p.mercenaries.available.insert(Country::English, 1);
            p.assistants.on_board.insert(target, 1);
            p.president_location = PresidentLocation::OnBoard(AreaId::new(1));
        }
        state.players[1].president_location = PresidentLocation::OnBoard(target);

        let result = apply_mercenary(&mut state, Country::English, target);
        assert!(result.is_ok());
        assert!(state.is_mercenary_action);
        assert_eq!(state.mercenary_target_area, Some(target));
    }

    #[test]
    fn test_mercenary_once_per_turn() {
        let mut state = create_minimal_state(2);
        let player = state.current_player;
        let target1 = AreaId::new(5);
        let target2 = AreaId::new(6);

        // Give player two mercenaries and assistants at both target areas
        {
            let p = state.player_mut(player);
            p.mercenaries.available.insert(Country::English, 1);
            p.mercenaries.available.insert(Country::French, 1);
            p.assistants.on_board.insert(target1, 2);
            p.assistants.on_board.insert(target2, 2);
            p.president_location = PresidentLocation::OnBoard(AreaId::new(1));
        }

        // First use should succeed
        assert!(!state.additional_action_state.mercenary_used);
        let result = apply_mercenary(&mut state, Country::English, target1);
        assert!(result.is_ok());
        assert!(state.additional_action_state.mercenary_used);

        // Reset mercenary state (simulating completing the sub-turn)
        state.is_mercenary_action = false;
        state.mercenary_target_area = None;
        state.turn_phase = TurnPhase::AdditionalActionSecond;

        // Second use should fail (once per turn)
        // The mercenary_used flag should prevent this
        assert!(state.additional_action_state.mercenary_used);

        // Verify flag persists across both additional action phases
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        assert!(state.additional_action_state.mercenary_used);
    }

    #[test]
    fn test_recovery_normal() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);

        // Place president and 3 assistants
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.on_board.insert(area, 3);
        state.players[0].assistants.in_hand = 5;

        apply_recovery_with_leave(&mut state, false);

        // All 3 assistants should be recovered
        assert_eq!(state.players[0].assistants.in_hand, 8);
        assert_eq!(state.players[0].assistants.on_board.get(&area), None);
    }

    #[test]
    fn test_recovery_with_telephone() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);

        // Place president and 3 assistants
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.on_board.insert(area, 3);
        state.players[0].assistants.in_hand = 5;

        // With Telephone effect (leave_one = true)
        apply_recovery_with_leave(&mut state, true);

        // Only 2 assistants should be recovered, 1 remains
        assert_eq!(state.players[0].assistants.in_hand, 7);
        assert_eq!(state.players[0].assistants.on_board.get(&area), Some(&1));
    }

    #[test]
    fn test_recovery_with_telephone_only_one_assistant() {
        // Corrected per design 2026-07-18 §5.3-5 (owner-adjudicated rules
        // fix): "may leave 1 assistant in the action area" applies at
        // count == 1 too, which means recovering ZERO -- not force-recovering
        // the lone pawn. This pin previously asserted the opposite; that was
        // the bug Task 6 fixes in `apply_recovery_with_leave`.
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);

        // Place president and only 1 assistant
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.on_board.insert(area, 1);
        state.players[0].assistants.in_hand = 5;

        // With Telephone effect and only 1 assistant - leaves it, recovers none
        apply_recovery_with_leave(&mut state, true);

        assert_eq!(state.players[0].assistants.in_hand, 5);
        assert_eq!(state.players[0].assistants.on_board.get(&area), Some(&1));
    }

    #[test]
    fn test_five_power_other_effects() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);

        // Place president
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.current_action_power = Some(5);
        state.turn_phase = TurnPhase::FivePowerBonus;

        // Players start with: 1 copper, 1 silk, 1 tea, 1 seafood, 3 money (start player)
        // Test various token effects - expected values are base + bonus
        let test_cases = vec![
            (FivePowerEffect::Copper(2), 1 + 2, 1, 3, 0u32),       // +2 copper
            (FivePowerEffect::Silk(1), 1, 1 + 1, 3, 0),            // +1 silk
            (FivePowerEffect::Money(3), 1, 1, 3 + 3, 0),           // +3 money
        ];
        let rules = Rules::empty_for_tests();

        for (effect, exp_copper, exp_silk, exp_money, exp_vp) in test_cases {
            let mut test_state = state.clone();
            test_state.board_layout.five_power_tokens.insert(action_area, Some(effect));

            let result = apply_move(&test_state, &Move::Take5PowerToken, &rules);
            assert!(result.is_ok());

            let new_state = result.unwrap();
            assert_eq!(new_state.players[0].copper, exp_copper);
            assert_eq!(new_state.players[0].silk, exp_silk);
            assert_eq!(new_state.players[0].money, exp_money);
            assert_eq!(new_state.players[0].victory_points, exp_vp);
        }
    }

    #[test]
    fn test_five_power_copper_token_mining_technology_bonus() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.current_action_power = Some(5);
        state.turn_phase = TurnPhase::FivePowerBonus;
        state.board_layout.five_power_tokens.insert(action_area, Some(FivePowerEffect::Copper(2)));

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(mining_tech_fixture(), "tech_mining");
        state.players[0].technologies.push(TechCardId(1));

        let starting_copper = state.players[0].copper;
        let result = apply_move(&state, &Move::Take5PowerToken, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        // Base +2 copper from the token + 1 Mining bonus (1 owned copy).
        assert_eq!(new_state.players[0].copper, starting_copper + 3);
    }

    #[test]
    fn test_five_power_silk_token_mining_technology_no_bonus() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.current_action_power = Some(5);
        state.turn_phase = TurnPhase::FivePowerBonus;
        state.board_layout.five_power_tokens.insert(action_area, Some(FivePowerEffect::Silk(2)));

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(mining_tech_fixture(), "tech_mining");
        state.players[0].technologies.push(TechCardId(1));

        let starting_copper = state.players[0].copper;
        let starting_silk = state.players[0].silk;
        let result = apply_move(&state, &Move::Take5PowerToken, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        // A non-copper token effect through the same function must not trigger Mining.
        assert_eq!(new_state.players[0].silk, starting_silk + 2);
        assert_eq!(new_state.players[0].copper, starting_copper);
    }

    #[test]
    fn test_trading_house_passive_income() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);

        // Player 0 is current player, Player 1 owns trading house at action area
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.players[0].assistants.on_board.insert(action_area, 2);
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(3);
        state.turn_phase = TurnPhase::AreaAction;

        // Add building site with Player 1's trading house
        let mut site = BuildingSite::new([2, 3, 4, 5], 10);
        site.trading_house_owner = Some(PlayerId::new(1));
        state.board_layout.building_sites.insert(action_area, site);
        state.board_layout.areas.insert(action_area, AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 0),
        });

        // Record initial money
        let player1_initial_money = state.players[1].money;
        let rules = Rules::empty_for_tests();

        // Player 0 takes action
        let result = apply_move(&state, &Move::ExecuteAreaAction {
            action: AreaAction::ProduceCopper,
        }, &rules);
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Player 1 should have received 1 Yen passive income
        assert_eq!(new_state.players[1].money, player1_initial_money + 1);
    }

    #[test]
    fn test_trading_house_no_passive_income_own_building() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);

        // Player 0 owns their own trading house at the action area
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.players[0].assistants.on_board.insert(action_area, 2);
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(3);
        state.turn_phase = TurnPhase::AreaAction;

        // Add building site with Player 0's own trading house
        let mut site = BuildingSite::new([2, 3, 4, 5], 10);
        site.trading_house_owner = Some(PlayerId::new(0)); // Own building
        state.board_layout.building_sites.insert(action_area, site);
        state.board_layout.areas.insert(action_area, AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 0),
        });

        // Record initial money for player 1 (we only check that player 1 is unchanged)
        let player1_initial_money = state.players[1].money;
        let rules = Rules::empty_for_tests();

        // Player 0 takes action
        let result = apply_move(&state, &Move::ExecuteAreaAction {
            action: AreaAction::ProduceCopper,
        }, &rules);
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Neither player should receive passive income (player 0 owns their own building)
        // Player 0's money may change due to the action, but player 1 should be unchanged
        assert_eq!(new_state.players[1].money, player1_initial_money);
    }

    #[test]
    fn test_trading_house_passive_income_mercenary_action() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(5);

        // Set up mercenary action at area where Player 1 owns trading house
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(action_area);
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(2);
        state.turn_phase = TurnPhase::AreaAction;

        state.players[0].assistants.on_board.insert(action_area, 1);

        // Add building site with Player 1's trading house
        let mut site = BuildingSite::new([2, 3, 4, 5], 10);
        site.trading_house_owner = Some(PlayerId::new(1));
        state.board_layout.building_sites.insert(action_area, site);
        state.board_layout.areas.insert(action_area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 5),
        });

        let player1_initial_money = state.players[1].money;
        let rules = Rules::empty_for_tests();

        // Player 0 takes mercenary action
        let result = apply_move(&state, &Move::ExecuteAreaAction {
            action: AreaAction::Bank,
        }, &rules);
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Player 1 should receive passive income even during mercenary action
        assert_eq!(new_state.players[1].money, player1_initial_money + 1);
    }

    #[test]
    fn test_produce_copper_mining_technology_bonus_at_power_3() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(3);
        state.turn_phase = TurnPhase::AreaAction;
        state.board_layout.areas.insert(action_area, AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 0),
        });

        let mut rules = Rules::empty_for_tests();
        let mining = TechCard {
            id: TechCardId(1),
            name: "Mining Technology".to_string(),
            country: Country::German,
            industry_cost: 3,
            effect: TechEffect::MiningTechnology,
        };
        rules.techs.insert(mining, "tech_mining");

        // Without the tech: power 3 -> base copper_mine_reward = 1, no bonus.
        let result = apply_move(&state, &Move::ExecuteAreaAction {
            action: AreaAction::ProduceCopper,
        }, &rules);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().players[0].copper, state.players[0].copper + 1);

        // With the tech: +1 additional copper on receipt (2 total).
        state.players[0].technologies.push(TechCardId(1));
        let result = apply_move(&state, &Move::ExecuteAreaAction {
            action: AreaAction::ProduceCopper,
        }, &rules);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().players[0].copper, state.players[0].copper + 2);
    }

    #[test]
    fn test_produce_copper_mining_technology_no_bonus_at_power_2_zero_receipt() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(2);
        state.turn_phase = TurnPhase::AreaAction;
        state.board_layout.areas.insert(action_area, AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 0),
        });

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

        // Power 2 -> copper_mine_reward = 0 (nothing received), so Mining's
        // "on receipt" bonus does not fire even though the tech is owned.
        let result = apply_move(&state, &Move::ExecuteAreaAction {
            action: AreaAction::ProduceCopper,
        }, &rules);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().players[0].copper, state.players[0].copper);
    }

    fn mining_tech_fixture() -> TechCard {
        TechCard {
            id: TechCardId(1),
            name: "Mining Technology".to_string(),
            country: Country::German,
            industry_cost: 3,
            effect: TechEffect::MiningTechnology,
        }
    }

    fn gaslight_tech_fixture(id: u8) -> TechCard {
        TechCard {
            id: TechCardId(id),
            name: "Gaslight".to_string(),
            country: Country::French,
            industry_cost: 3,
            effect: TechEffect::Gaslight,
        }
    }

    #[test]
    fn test_build_shophouse_gaslight_vp_bonus() {
        // C4 final-review fix: Gaslight (+2 VP per owned copy when building
        // a Shophouse) was computed by building_vp_bonus but never called
        // from apply_build_shophouse.
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].shophouses.in_hand = 1;

        let mut site = BuildingSite::new([0, 0, 0, 0], 10);
        site.spaces[0] = SpaceReward { vp: 2, ..Default::default() };
        state.board_layout.building_sites.insert(area, site);

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(gaslight_tech_fixture(1), "tech_gaslight");
        state.players[0].technologies.push(TechCardId(1));

        // apply_move_unchecked: create_minimal_state defaults turn_phase to
        // AdditionalActionFirst with no action power, so get_legal_moves
        // never offers BuildShophouse here -- this test targets the raw
        // build-reward arm directly (same for the rest of this file's
        // test_build_shophouse_* cluster and its sibling shophouse tests).
        let result = apply_move_unchecked(&state, &Move::BuildShophouse { slot: 0 }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        // 2 (space VP) + 2 (Gaslight, 1 owned copy).
        assert_eq!(new_state.players[0].victory_points, 4);
    }

    #[test]
    fn test_build_shophouse_gaslight_bonus_stacks_per_owned_copy() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].shophouses.in_hand = 1;

        let mut site = BuildingSite::new([0, 0, 0, 0], 10);
        site.spaces[0] = SpaceReward { vp: 2, ..Default::default() };
        state.board_layout.building_sites.insert(area, site);

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(gaslight_tech_fixture(1), "tech_gaslight_1");
        rules.techs.insert(gaslight_tech_fixture(2), "tech_gaslight_2");
        state.players[0].technologies.push(TechCardId(1));
        state.players[0].technologies.push(TechCardId(2));

        // apply_move_unchecked: see test_build_shophouse_gaslight_vp_bonus.
        let result = apply_move_unchecked(&state, &Move::BuildShophouse { slot: 0 }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        // 2 (space VP) + 4 (Gaslight, 2 owned copies x 2 VP).
        assert_eq!(new_state.players[0].victory_points, 6);
    }

    /// C1 regression (external six-agent review, 2026-07-20): the raw build
    /// arm indexed `site.spaces[slot as usize]` (and `shophouses_built[..]`)
    /// into a `[SpaceReward; 4]` with NO bound check, so an out-of-range slot
    /// aborted the process instead of returning a `MoveError`. Checked
    /// `apply_move` masked it -- the legal list never offers such a slot --
    /// but `apply_move_unchecked` is public and documented for future search
    /// inner loops, and a search loop cannot catch a panic. This arm now
    /// self-defends the way `EndAdditionalPhase`, `UseUniversity`,
    /// `UseSpinningMill` and `apply_return_to_hand` already do.
    #[test]
    fn out_of_range_shophouse_slot_is_rejected_by_the_raw_arm_not_a_panic() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].shophouses.in_hand = 1;
        state.board_layout.building_sites.insert(area, BuildingSite::new([0, 0, 0, 0], 10));
        let rules = Rules::empty_for_tests();

        // apply_move_unchecked: see test_build_shophouse_gaslight_vp_bonus --
        // the raw arm IS the subject here, since it is what must self-defend.
        // 4 is the first out-of-range slot for a 4-space site; 250 is the
        // value the review's reproduction used; u8::MAX is the ceiling.
        for slot in [4u8, 250u8, u8::MAX] {
            match apply_move_unchecked(&state, &Move::BuildShophouse { slot }, &rules) {
                Err(MoveError::InvalidSlot) => {}
                Err(other) => panic!("slot {slot} must be InvalidSlot, got {other:?}"),
                Ok(_) => panic!("slot {slot} is out of range and must be rejected, was accepted"),
            }
        }
    }

    /// Phase-2.9 divergence fix (ENGINE BUG -- completeness): the raw arm
    /// checked only slot range and token count, so an unchecked apply onto an
    /// OCCUPIED slot silently overwrote the other player's shophouse. Same
    /// self-defence class as the out-of-range guard above (C1 precedent).
    #[test]
    fn occupied_shophouse_slot_is_rejected_by_the_raw_arm_not_overwritten() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].shophouses.in_hand = 1;
        let mut site = BuildingSite::new([2, 3, 4, 5], 10);
        site.shophouses_built[1] = Some(PlayerId::new(1));
        state.board_layout.building_sites.insert(area, site);
        let rules = Rules::empty_for_tests();

        // apply_move_unchecked: the raw arm IS the subject -- it must
        // self-defend even when the legal list is bypassed.
        match apply_move_unchecked(&state, &Move::BuildShophouse { slot: 1 }, &rules) {
            Err(MoveError::RuleViolation(_)) => {}
            Err(other) => panic!("occupied slot must be a RuleViolation, got {other:?}"),
            Ok(_) => panic!("occupied slot 1 must be rejected, was accepted (overwrite)"),
        }
        // Zero mutation on error: the input state is untouched.
        assert_eq!(
            state.board_layout.building_sites[&area].shophouses_built[1],
            Some(PlayerId::new(1))
        );
        assert_eq!(state.players[0].shophouses.in_hand, 1);
        assert!(state.players[0].shophouses.built.is_empty());
    }

    /// Companion to the occupied-slot guard: a builder who already owns a
    /// shophouse anywhere on the site may not add a second one, even via the
    /// unchecked path onto a different (empty) slot.
    #[test]
    fn duplicate_own_shophouse_on_site_is_rejected_by_the_raw_arm() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].shophouses.in_hand = 1;
        let mut site = BuildingSite::new([2, 3, 4, 5], 10);
        site.shophouses_built[2] = Some(PlayerId::new(0));
        state.board_layout.building_sites.insert(area, site);
        let rules = Rules::empty_for_tests();

        // apply_move_unchecked: see the occupied-slot test above.
        match apply_move_unchecked(&state, &Move::BuildShophouse { slot: 0 }, &rules) {
            Err(MoveError::RuleViolation(_)) => {}
            Err(other) => panic!("own duplicate must be a RuleViolation, got {other:?}"),
            Ok(_) => panic!("second own shophouse on the site must be rejected"),
        }
        // Zero mutation on error: the input state is untouched.
        assert_eq!(state.board_layout.building_sites[&area].shophouses_built[0], None);
        assert_eq!(state.players[0].shophouses.in_hand, 1);
    }

    /// Slot-specific printed rewards flow from the slot ACTUALLY built:
    /// building slot 3 grants spaces[3]'s reward, not spaces[0]'s. This pins
    /// the payoff side of per-slot enumeration (the four spaces print
    /// materially different rewards, so the choice of slot matters).
    #[test]
    fn shophouse_reward_comes_from_the_slot_actually_built() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].shophouses.in_hand = 1;
        let starting_tea = state.players[0].tea;

        let mut site = BuildingSite::new([0, 0, 0, 0], 10);
        site.spaces[0] = SpaceReward { vp: 9, ..Default::default() };
        site.spaces[3] = SpaceReward { tea: 2, ..Default::default() };
        state.board_layout.building_sites.insert(area, site);
        let rules = Rules::empty_for_tests();

        // apply_move_unchecked: see test_build_shophouse_gaslight_vp_bonus.
        let result = apply_move_unchecked(&state, &Move::BuildShophouse { slot: 3 }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        assert_eq!(new_state.players[0].tea, starting_tea + 2, "slot 3 grants spaces[3]'s tea");
        assert_eq!(new_state.players[0].victory_points, 0, "spaces[0]'s VP must NOT be granted");
        assert_eq!(new_state.board_layout.building_sites[&area].shophouses_built[3], Some(PlayerId::new(0)));
        assert_eq!(new_state.board_layout.building_sites[&area].shophouses_built[0], None);
    }

    #[test]
    fn test_chinatown_yen_to_copper_mining_technology_bonus() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(2);
        state.turn_phase = TurnPhase::AreaAction;
        state.board_layout.areas.insert(action_area, AreaConfig {
            area_type: AreaType::Chinatown,
            grid_position: (0, 0),
        });
        state.players[0].money = 10;

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(mining_tech_fixture(), "tech_mining");
        state.players[0].technologies.push(TechCardId(1));

        let starting_copper = state.players[0].copper;
        let result = apply_move(&state, &Move::ExecuteAreaAction {
            action: AreaAction::Chinatown { exchanges: vec![ChinatownExchange::YenToCopper] },
        }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        // Base 1 copper from the exchange + 1 Mining bonus (1 owned copy).
        assert_eq!(new_state.players[0].copper, starting_copper + 2);
        assert_eq!(new_state.players[0].money, 8); // still pays the normal 2 Yen
    }

    #[test]
    fn test_chinatown_yen_to_silk_mining_technology_no_bonus() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(2);
        state.turn_phase = TurnPhase::AreaAction;
        state.board_layout.areas.insert(action_area, AreaConfig {
            area_type: AreaType::Chinatown,
            grid_position: (0, 0),
        });
        state.players[0].money = 10;

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(mining_tech_fixture(), "tech_mining");
        state.players[0].technologies.push(TechCardId(1));

        let starting_copper = state.players[0].copper;
        let starting_silk = state.players[0].silk;
        let result = apply_move(&state, &Move::ExecuteAreaAction {
            action: AreaAction::Chinatown { exchanges: vec![ChinatownExchange::YenToSilk] },
        }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        // A non-copper exchange through the same function must not trigger Mining.
        assert_eq!(new_state.players[0].silk, starting_silk + 1);
        assert_eq!(new_state.players[0].copper, starting_copper);
    }

    #[test]
    fn test_chinatown_double_yen_to_copper_mining_bonus_stacks_per_exchange() {
        // Task 17 pin: 2 sequential YenToCopper exchanges in ONE Chinatown
        // move each independently trigger the per-receipt Mining bonus
        // (apply_chinatown_exchange recomputes the bonus fresh for every
        // exchange in the AreaAction::Chinatown loop) -- 2 base copper (1
        // per exchange) + 2 Mining bonus (1 owned copy x 2 exchanges) = 4.
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(4);
        state.turn_phase = TurnPhase::AreaAction;
        state.board_layout.areas.insert(action_area, AreaConfig {
            area_type: AreaType::Chinatown,
            grid_position: (0, 0),
        });
        state.players[0].money = 4;
        state.players[0].copper = 0;

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(mining_tech_fixture(), "tech_mining");
        state.players[0].technologies.push(TechCardId(1));

        let result = apply_move(&state, &Move::ExecuteAreaAction {
            action: AreaAction::Chinatown {
                exchanges: vec![ChinatownExchange::YenToCopper, ChinatownExchange::YenToCopper],
            },
        }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        assert_eq!(new_state.players[0].copper, 4); // 2 exchanges x (1 base + 1 mining)
        assert_eq!(new_state.players[0].money, 0); // 4 - 2*2
    }

    #[test]
    fn test_cannot_return_to_start_area() {
        let mut state = create_minimal_state(2);

        // Set up board with connected areas: 0 -- 1 -- 2
        state.board_layout.areas.insert(AreaId::new(0), AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.board_layout.areas.insert(AreaId::new(1), AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 1),
        });
        state.board_layout.areas.insert(AreaId::new(2), AreaConfig {
            area_type: AreaType::SilkMill,
            grid_position: (0, 2),
        });
        state.board_layout.adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        state.board_layout.adjacency.insert(AreaId::new(1), vec![AreaId::new(0), AreaId::new(2)]);
        state.board_layout.adjacency.insert(AreaId::new(2), vec![AreaId::new(1)]);

        // President starts in area 0
        state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        state.players[0].assistants.on_board.insert(AreaId::new(0), 1);
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);

        // Simulate entering Movement phase (set start area)
        state.turn_phase = TurnPhase::Placement;
        let rules = Rules::empty_for_tests();
        let result = apply_move(&state, &Move::PlaceThreeAreas { placements: vec![] }, &rules);
        assert!(result.is_ok());
        let state = result.unwrap();

        // president_start_area should be set to area 0
        assert_eq!(state.president_start_area, Some(AreaId::new(0)));
        assert_eq!(state.turn_phase, TurnPhase::Movement);

        // Try to move back to area 0 (should fail). apply_move_unchecked:
        // get_legal_moves excludes the starting area from movement
        // destinations too, so this targets the raw arm's own
        // validate_movement_destination check directly.
        let result = apply_move_unchecked(&state, &Move::MovePresident {
            path: vec![AreaId::new(1), AreaId::new(0)],
            destination: AreaId::new(0),
        }, &rules);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(MoveError::RuleViolation("Cannot return to starting area in same turn"))
        ));
    }

    #[test]
    fn test_can_move_to_different_area() {
        let mut state = create_minimal_state(2);

        // Set up board
        state.board_layout.areas.insert(AreaId::new(0), AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.board_layout.areas.insert(AreaId::new(1), AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 1),
        });
        state.board_layout.adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        state.board_layout.adjacency.insert(AreaId::new(1), vec![AreaId::new(0)]);

        // President starts in area 0
        state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        state.players[0].assistants.on_board.insert(AreaId::new(0), 1);
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);

        // Enter Movement phase
        state.turn_phase = TurnPhase::Placement;
        let rules = Rules::empty_for_tests();
        let result = apply_move(&state, &Move::PlaceThreeAreas { placements: vec![] }, &rules);
        assert!(result.is_ok());
        let state = result.unwrap();

        // Move to area 1 (different from start) - should succeed
        let result = apply_move(&state, &Move::MovePresident {
            path: vec![AreaId::new(1)],
            destination: AreaId::new(1),
        }, &rules);
        assert!(result.is_ok());

        let new_state = result.unwrap();
        // president_start_area should be cleared after movement
        assert_eq!(new_state.president_start_area, None);
        assert_eq!(new_state.turn_phase, TurnPhase::AreaAction);
    }

    #[test]
    fn test_president_from_hand_no_start_restriction() {
        let mut state = create_minimal_state(2);

        // President is in hand
        state.players[0].president_location = PresidentLocation::InHand;
        state.players[0].assistants.on_board.insert(AreaId::new(0), 1);

        state.board_layout.areas.insert(AreaId::new(0), AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });

        // Enter Movement phase (no start area since president is in hand)
        state.turn_phase = TurnPhase::Placement;
        let rules = Rules::empty_for_tests();
        let result = apply_move(&state, &Move::PlaceThreeAreas { placements: vec![] }, &rules);
        assert!(result.is_ok());
        let state = result.unwrap();

        // No start area should be set
        assert_eq!(state.president_start_area, None);

        // Place president - should succeed (no start area restriction)
        let result = apply_move(&state, &Move::PlacePresident {
            destination: AreaId::new(0),
        }, &rules);
        assert!(result.is_ok());
    }

    #[test]
    fn partial_recovery_moves_exactly_the_requested_counts() {
        // I5 ruling (2026-07-19): recovering a strict subset of one area's
        // count must leave the remainder on board, untouched -- not the
        // old all-or-nothing-per-area semantics.
        let mut state = create_minimal_state(2);
        let a0 = AreaId::new(0);
        let a3 = AreaId::new(3);
        state.players[0].president_location = PresidentLocation::OnBoard(a0);
        state.players[0].assistants.on_board.insert(a0, 2);
        state.players[0].assistants.on_board.insert(a3, 1);
        let hand_before = state.players[0].assistants.in_hand;

        let mut recover = BTreeMap::new();
        recover.insert(a0, 1);
        let rules = Rules::empty_for_tests();
        let result = apply_move_unchecked(&state, &Move::ReturnPresidentToHand { recover }, &rules);
        let new_state = result.expect("partial recovery within on-board counts should succeed");

        let p = &new_state.players[0];
        assert_eq!(p.assistants.in_hand, hand_before + 1);
        assert_eq!(p.assistants.on_board.get(&a0), Some(&1), "1 of 2 remains at A0");
        assert_eq!(p.assistants.on_board.get(&a3), Some(&1), "A3 untouched");
        assert_eq!(p.president_location, PresidentLocation::InHand);
    }

    #[test]
    fn over_count_recovery_is_rejected_by_the_raw_arm() {
        // recover {A0: 3} exceeds on_board {A0: 2}. The checked path
        // already rejects this shape by membership (it's not in
        // enumerate_recovery_options's cartesian product); this pins the
        // apply-layer guard directly, per the standing I1 lesson that raw
        // arms must self-defend rather than relying solely on the
        // legal-move pre-filter (T1: apply_move_unchecked is called by
        // prechecked hot paths, but must still refuse an out-of-range
        // count rather than underflow or over-credit).
        let mut state = create_minimal_state(2);
        let a0 = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(a0);
        state.players[0].assistants.on_board.insert(a0, 2);
        let hand_before = state.players[0].assistants.in_hand;

        let mut recover = BTreeMap::new();
        recover.insert(a0, 3);
        let rules = Rules::empty_for_tests();
        let result = apply_move_unchecked(&state, &Move::ReturnPresidentToHand { recover }, &rules);

        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        // The original state (owned by the test, passed by shared
        // reference) must be untouched.
        assert_eq!(state.players[0].assistants.in_hand, hand_before);
        assert_eq!(state.players[0].assistants.on_board.get(&a0), Some(&2));
        assert_eq!(state.players[0].president_location, PresidentLocation::OnBoard(a0));
    }

    #[test]
    fn test_fulfill_order_awards_vp() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;

        // Give player the required resources
        state.players[0].copper = 2;
        state.players[0].silk = 1;

        // Create order card
        let order_id = OrderCardId(42);
        state.players[0].order_cards_in_hand.push(order_id);

        // Create order registry with the card
        let mut order_registry = OrderRegistry::new();
        let order_card = OrderCard {
            id: order_id,
            country: Country::English,
            requirements: TradeGoodRequirement::new(2, 1, 0, 0), // 2 copper, 1 silk
            rewards: OrderRewards {
                victory_points: 5,
                money: 3,
                imported_goods: 0,
                copper: 0,
                silk: 0,
                tea: 0,
                seafood: 0,
                movement: 0,
                any_trade_good: 0,
                warehouse_tokens: 0,
            },
        };
        order_registry.insert(order_card, "test_order");
        let mut rules = Rules::empty_for_tests();
        rules.orders = order_registry;

        // Fulfill order
        let result = apply_move(
            &state,
            &Move::FulfillOrder { order_id },
            &rules,
        );
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Verify VP awarded
        assert_eq!(new_state.players[0].victory_points, 5);
        // Verify money awarded
        assert_eq!(new_state.players[0].money, 3 + state.players[0].money); // 3 reward + starting money
        // Verify trade goods deducted
        assert_eq!(new_state.players[0].copper, 0); // 2 - 2 = 0
        assert_eq!(new_state.players[0].silk, 0);   // 1 - 1 = 0
        // Verify order moved to completed
        assert!(!new_state.players[0].order_cards_in_hand.contains(&order_id));
        assert!(new_state.players[0].completed_orders.contains(&order_id));
    }

    #[test]
    fn test_fulfill_order_requires_resources() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;

        // Give player INSUFFICIENT resources
        state.players[0].copper = 1; // Need 2
        state.players[0].silk = 1;

        let order_id = OrderCardId(42);
        state.players[0].order_cards_in_hand.push(order_id);

        let mut order_registry = OrderRegistry::new();
        let order_card = OrderCard {
            id: order_id,
            country: Country::English,
            requirements: TradeGoodRequirement::new(2, 1, 0, 0),
            rewards: OrderRewards::default(),
        };
        order_registry.insert(order_card, "test_order");
        let mut rules = Rules::empty_for_tests();
        rules.orders = order_registry;

        // Fulfill order should fail. apply_move_unchecked: insufficient
        // resources means get_legal_moves never offers this FulfillOrder,
        // so this targets the raw resource-check arm directly.
        let result = apply_move_unchecked(
            &state,
            &Move::FulfillOrder { order_id },
            &rules,
        );
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(MoveError::RuleViolation("Insufficient trade goods to fulfill order"))
        ));
    }

    #[test]
    fn test_fulfill_achievement_awards_vp() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;

        // Give player enough copper for Copper5 achievement
        state.players[0].copper = 5;
        state.players[0].assistants.in_hand = 2;
        let rules = Rules::empty_for_tests();

        // Fulfill achievement
        let result = apply_move(&state, &Move::FulfillAchievement {
            achievement: AchievementType::Copper5,
        }, &rules);
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Verify VP awarded (first place = 8 VP for Category A)
        assert_eq!(new_state.players[0].victory_points, 8);
        // Verify assistant locked on achievement
        assert_eq!(new_state.players[0].assistants.in_hand, 1);
        assert_eq!(new_state.players[0].assistants.on_achievements, 1);
        // Verify player added to claimed list
        assert!(new_state.achievements.iter().any(|a|
            a.achievement_type == AchievementType::Copper5 &&
            a.claimed_by.contains(&PlayerId::new(0))
        ));
    }

    #[test]
    fn test_fulfill_achievement_subsequent_player() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;

        // First player claims achievement
        state.players[0].copper = 5;
        state.players[0].assistants.in_hand = 2;
        let rules = Rules::empty_for_tests();
        let state = apply_move(&state, &Move::FulfillAchievement {
            achievement: AchievementType::Copper5,
        }, &rules).unwrap();

        // Switch to player 2
        let mut state = state;
        state.current_player = PlayerId::new(1);
        state.players[1].copper = 5;
        state.players[1].assistants.in_hand = 2;

        // Second player claims same achievement
        let result = apply_move(&state, &Move::FulfillAchievement {
            achievement: AchievementType::Copper5,
        }, &rules);
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Verify subsequent VP (6 VP for Category A)
        assert_eq!(new_state.players[1].victory_points, 6);
    }

    #[test]
    fn test_fulfill_order_warehouse_tokens_takes_assistant() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;

        let order_id = OrderCardId(42);
        state.players[0].order_cards_in_hand.push(order_id);
        let initial_hand = state.players[0].assistants.in_hand;
        let initial_warehouse = state.players[0].assistants.in_warehouse;

        let mut order_registry = OrderRegistry::new();
        let order_card = OrderCard {
            id: order_id,
            country: Country::English,
            requirements: TradeGoodRequirement::default(),
            rewards: OrderRewards {
                warehouse_tokens: 1,
                ..Default::default()
            },
        };
        order_registry.insert(order_card, "test_order");
        let mut rules = Rules::empty_for_tests();
        rules.orders = order_registry;

        let result = apply_move(&state, &Move::FulfillOrder { order_id }, &rules);
        assert!(result.is_ok());
        let mid = result.unwrap();

        // warehouse_tokens now queues a WarehouseTake choice instead of
        // auto-taking (design §5.2) -- resolve it before checking the transfer.
        assert_eq!(mid.pending_choices, vec![PendingChoice::WarehouseTake]);
        let new_state = apply_move(
            &mid, &Move::ChooseWarehouseTake { item: WarehouseItem::Assistant }, &rules,
        ).unwrap();

        // 1 assistant moved warehouse -> hand
        assert_eq!(new_state.players[0].assistants.in_hand, initial_hand + 1);
        assert_eq!(new_state.players[0].assistants.in_warehouse, initial_warehouse - 1);
        assert!(new_state.pending_choices.is_empty());
    }

    #[test]
    fn test_fulfill_order_warehouse_tokens_empty_warehouse_is_ok() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        state.players[0].assistants.in_warehouse = 0;
        state.players[0].shophouses.in_warehouse = 0;
        state.players[0].trading_houses.in_warehouse = 0;

        let order_id = OrderCardId(42);
        state.players[0].order_cards_in_hand.push(order_id);
        let initial_hand = state.players[0].assistants.in_hand;

        let mut order_registry = OrderRegistry::new();
        let order_card = OrderCard {
            id: order_id,
            country: Country::English,
            requirements: TradeGoodRequirement::default(),
            rewards: OrderRewards {
                warehouse_tokens: 3,
                ..Default::default()
            },
        };
        order_registry.insert(order_card, "test_order");
        let mut rules = Rules::empty_for_tests();
        rules.orders = order_registry;

        let result = apply_move(&state, &Move::FulfillOrder { order_id }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        // Warehouse already empty: every take auto-declines (singleton rule,
        // design §5.1), so nothing queues and nothing changes.
        assert!(new_state.pending_choices.is_empty());
        assert_eq!(new_state.players[0].assistants.in_warehouse, 0);
        assert_eq!(new_state.players[0].assistants.in_hand, initial_hand);
    }

    #[test]
    fn test_fulfill_order_movement_queues_assistant_move_choices() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        state.board_layout.areas.insert(AreaId::new(0), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        state.board_layout.areas.insert(AreaId::new(1), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });
        state.players[0].assistants.on_board.insert(AreaId::new(0), 2);
        let initial_hand = state.players[0].assistants.in_hand;

        let order_id = OrderCardId(42);
        state.players[0].order_cards_in_hand.push(order_id);

        let mut order_registry = OrderRegistry::new();
        let order_card = OrderCard {
            id: order_id,
            country: Country::English,
            requirements: TradeGoodRequirement::default(),
            rewards: OrderRewards {
                movement: 2,
                ..Default::default()
            },
        };
        order_registry.insert(order_card, "test_order");
        let mut rules = Rules::empty_for_tests();
        rules.orders = order_registry;

        let result = apply_move(&state, &Move::FulfillOrder { order_id }, &rules);
        assert!(result.is_ok());
        let mid = result.unwrap();

        // movement now queues an AssistantMove choice per icon instead of
        // being a no-op (design 2026-07-18 §5.3) -- two icons here, so two
        // choices queue.
        assert_eq!(mid.pending_choices, vec![PendingChoice::AssistantMove, PendingChoice::AssistantMove]);

        let after_first = apply_move(
            &mid,
            &Move::ChooseAssistantMove { from: MoveSource::Area(AreaId::new(0)), to: AreaId::new(1) },
            &rules,
        ).unwrap();
        assert_eq!(after_first.pending_choices, vec![PendingChoice::AssistantMove]);
        assert_eq!(after_first.players[0].assistants.on_board[&AreaId::new(1)], 1);

        let after_second = apply_move(&after_first, &Move::DeclineAssistantMove, &rules).unwrap();
        assert!(after_second.pending_choices.is_empty());
        assert_eq!(after_second.players[0].assistants.in_hand, initial_hand);
    }

    #[test]
    fn test_fulfill_order_movement_with_no_destinations_auto_declines() {
        // Degenerate case: create_minimal_state's default board has no areas
        // at all, so AssistantMove has zero valid destinations and each icon
        // auto-declines instead of queuing (design §5.1 singleton rule) --
        // apply_fulfill_order must still succeed and leave assistants intact.
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        state.players[0].assistants.on_board.insert(AreaId::new(0), 2);

        let order_id = OrderCardId(42);
        state.players[0].order_cards_in_hand.push(order_id);
        let before = state.players[0].assistants.clone();

        let mut order_registry = OrderRegistry::new();
        let order_card = OrderCard {
            id: order_id,
            country: Country::English,
            requirements: TradeGoodRequirement::default(),
            rewards: OrderRewards {
                movement: 2,
                ..Default::default()
            },
        };
        order_registry.insert(order_card, "test_order");
        let mut rules = Rules::empty_for_tests();
        rules.orders = order_registry;

        let result = apply_move(&state, &Move::FulfillOrder { order_id }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        assert!(new_state.pending_choices.is_empty());
        assert_eq!(new_state.players[0].assistants, before);
    }

    #[test]
    fn test_fulfill_order_awards_ball_and_language_school_bonuses() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        state.players[0].technologies.push(TechCardId(100));
        state.players[0].technologies.push(TechCardId(101));

        let order_id = OrderCardId(42);
        state.players[0].order_cards_in_hand.push(order_id);
        let starting_money = state.players[0].money;

        let mut order_registry = OrderRegistry::new();
        let order_card = OrderCard {
            id: order_id,
            country: Country::English,
            requirements: TradeGoodRequirement::default(),
            rewards: OrderRewards { victory_points: 5, money: 3, ..Default::default() },
        };
        order_registry.insert(order_card, "test_order");

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(
            TechCard { id: TechCardId(100), name: "Ball".to_string(), country: Country::English, industry_cost: 5, effect: TechEffect::Ball },
            "tech_ball",
        );
        tech_registry.insert(
            TechCard { id: TechCardId(101), name: "Language School".to_string(), country: Country::American, industry_cost: 3, effect: TechEffect::LanguageSchool },
            "tech_language_school",
        );

        let mut rules = Rules::empty_for_tests();
        rules.orders = order_registry;
        rules.techs = tech_registry;

        let result = apply_move(&state, &Move::FulfillOrder { order_id }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        // Base 5 VP reward + Ball's +2 VP.
        assert_eq!(new_state.players[0].victory_points, 5 + 2);
        // Starting money + 3 reward + Language School's +1 Yen.
        assert_eq!(new_state.players[0].money, starting_money + 3 + 1);
    }

    #[test]
    fn apply_university_wiring() {
        let mut state = create_minimal_state(2);
        state.players[0].money = 3;
        state.players[0].assistants.in_warehouse = 2;
        state.players[0].assistants.in_hand = 5;

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(
            TechCard {
                id: TechCardId(93),
                name: "University".to_string(),
                country: Country::French,
                industry_cost: 4,
                effect: TechEffect::University,
            },
            "tech_university",
        );
        state.players[0].technologies.push(TechCardId(93));
        let mut rules = Rules::empty_for_tests();
        rules.techs = tech_registry;

        let before_phase = state.turn_phase;
        let next = apply_move(&state, &Move::UseUniversity, &rules).unwrap();

        assert_eq!(next.players[0].money, 2);
        assert_eq!(next.players[0].assistants.in_warehouse, 1);
        assert_eq!(next.players[0].assistants.in_hand, 6);
        assert!(next.additional_action_state.university_used);
        assert_eq!(next.turn_phase, before_phase, "additional-action moves leave turn_phase alone");
    }

    #[test]
    fn spinning_mill_offered_and_applies() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionSecond;
        state.players[0].silk = 1;
        state.players[0].money = 0;

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(
            TechCard {
                id: TechCardId(94),
                name: "Textile Mill".to_string(),
                country: Country::Dutch,
                industry_cost: 3,
                effect: TechEffect::SpinningMill,
            },
            "tech_spinning_mill",
        );
        state.players[0].technologies.push(TechCardId(94));
        let mut rules = Rules::empty_for_tests();
        rules.techs = tech_registry;

        let moves = get_legal_moves(&state, &rules);
        assert!(
            moves.iter().any(|m| matches!(m, Move::UseSpinningMill)),
            "must be offered in AdditionalActionSecond"
        );

        let next = apply_move(&state, &Move::UseSpinningMill, &rules).unwrap();
        assert_eq!(next.players[0].silk, 0);
        assert_eq!(next.players[0].money, 2);
        assert!(next.additional_action_state.spinning_mill_used);
        assert_eq!(next.turn_phase, TurnPhase::AdditionalActionSecond, "additional-action moves leave turn_phase alone");
    }

    #[test]
    fn once_per_turn_flags_reset_next_turn() {
        // Drive both moves for real this turn (not a hand-set flag), then
        // hand off via EndAdditionalPhase and confirm the flags come back
        // clear (advance_phase's existing AdditionalActionState::new()
        // reset) so the same wiring can offer them again next turn.
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionSecond;
        state.players[0].silk = 1;

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(
            TechCard {
                id: TechCardId(95),
                name: "University".to_string(),
                country: Country::French,
                industry_cost: 4,
                effect: TechEffect::University,
            },
            "tech_university",
        );
        tech_registry.insert(
            TechCard {
                id: TechCardId(96),
                name: "Textile Mill".to_string(),
                country: Country::Dutch,
                industry_cost: 3,
                effect: TechEffect::SpinningMill,
            },
            "tech_spinning_mill",
        );
        state.players[0].technologies.push(TechCardId(95));
        state.players[0].technologies.push(TechCardId(96));
        let mut rules = Rules::empty_for_tests();
        rules.techs = tech_registry;

        let used = apply_move(&state, &Move::UseUniversity, &rules).unwrap();
        let used = apply_move(&used, &Move::UseSpinningMill, &rules).unwrap();
        assert!(used.additional_action_state.university_used);
        assert!(used.additional_action_state.spinning_mill_used);

        let next = apply_move(&used, &Move::EndAdditionalPhase, &rules).unwrap();
        assert!(!next.additional_action_state.university_used);
        assert!(!next.additional_action_state.spinning_mill_used);
    }

    #[test]
    fn apply_university_rejects_second_use_same_turn() {
        // Resource headroom (money 3, warehouse 2) deliberately survives a
        // first use (money 2, warehouse 1) with room to spare, so a second
        // same-turn call submitted directly through apply_move -- without
        // pre-filtering through get_legal_moves, exactly as a Phase-3 bot
        // will call this pure API at volume -- can only be blocked by the
        // university_used flag, not by apply_university's own resource
        // checks. Mirrors apply_mercenary's self-defended once-per-turn gate.
        let mut state = create_minimal_state(2);
        state.players[0].money = 3;
        state.players[0].assistants.in_warehouse = 2;
        state.players[0].assistants.in_hand = 5;

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(
            TechCard {
                id: TechCardId(97),
                name: "University".to_string(),
                country: Country::French,
                industry_cost: 4,
                effect: TechEffect::University,
            },
            "tech_university",
        );
        state.players[0].technologies.push(TechCardId(97));
        let mut rules = Rules::empty_for_tests();
        rules.techs = tech_registry;

        let after_first = apply_move(&state, &Move::UseUniversity, &rules).unwrap();
        assert!(after_first.additional_action_state.university_used);
        let snapshot_money = after_first.players[0].money;
        let snapshot_warehouse = after_first.players[0].assistants.in_warehouse;
        let snapshot_hand = after_first.players[0].assistants.in_hand;

        // apply_move_unchecked: with university_used set, get_legal_moves no
        // longer offers UseUniversity, so this is exactly the raw-arm
        // self-defense path the comment above describes -- a bot calling
        // apply_move_unchecked directly, without pre-filtering.
        let second = apply_move_unchecked(&after_first, &Move::UseUniversity, &rules);
        assert!(
            matches!(second, Err(MoveError::RuleViolation(_))),
            "second same-turn use must be rejected, got {second:?}"
        );

        // apply_move never mutates its input -- this pins that contract
        // explicitly for this move, on top of the type-level guarantee.
        assert_eq!(after_first.players[0].money, snapshot_money);
        assert_eq!(after_first.players[0].assistants.in_warehouse, snapshot_warehouse);
        assert_eq!(after_first.players[0].assistants.in_hand, snapshot_hand);
    }

    #[test]
    fn apply_spinning_mill_rejects_second_use_same_turn() {
        // Silk headroom (2) deliberately survives a first use (1 remains),
        // isolating the missing spinning_mill_used guard as the only thing
        // that can block a second same-turn call -- see
        // apply_university_rejects_second_use_same_turn for the full
        // rationale (same shape, mirrored for the other move).
        let mut state = create_minimal_state(2);
        state.players[0].silk = 2;

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(
            TechCard {
                id: TechCardId(98),
                name: "Textile Mill".to_string(),
                country: Country::Dutch,
                industry_cost: 3,
                effect: TechEffect::SpinningMill,
            },
            "tech_spinning_mill",
        );
        state.players[0].technologies.push(TechCardId(98));
        let mut rules = Rules::empty_for_tests();
        rules.techs = tech_registry;

        let after_first = apply_move(&state, &Move::UseSpinningMill, &rules).unwrap();
        assert!(after_first.additional_action_state.spinning_mill_used);
        let snapshot_silk = after_first.players[0].silk;
        let snapshot_money = after_first.players[0].money;

        // apply_move_unchecked: same reasoning as
        // apply_university_rejects_second_use_same_turn -- with
        // spinning_mill_used set, get_legal_moves no longer offers
        // UseSpinningMill.
        let second = apply_move_unchecked(&after_first, &Move::UseSpinningMill, &rules);
        assert!(
            matches!(second, Err(MoveError::RuleViolation(_))),
            "second same-turn use must be rejected, got {second:?}"
        );

        assert_eq!(after_first.players[0].silk, snapshot_silk);
        assert_eq!(after_first.players[0].money, snapshot_money);
    }

    #[test]
    fn test_fulfill_order_awards_mercenary_at_every_second_completed_order_of_a_country() {
        // Rulebook p.8 "Matching Country icons": a mercenary of that country
        // is awarded once completed-order + owned-tech country icons reach
        // each even count (2, 4, 6, ...) — pinning check_mercenary_eligibility's
        // real semantics (game/cards.rs), not redesigning them.
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;

        let country = Country::English;
        let mut order_registry = OrderRegistry::new();
        for i in 1..=4u8 {
            order_registry.insert(
                OrderCard {
                    id: OrderCardId(i),
                    country,
                    requirements: TradeGoodRequirement::default(),
                    rewards: OrderRewards::default(),
                },
                &format!("test_order_{i}"),
            );
        }
        let mut rules = Rules::empty_for_tests();
        rules.orders = order_registry;

        let starting_supply = state.mercenary_supply.english;

        // 1st completed order of this country: count=1, no award.
        state.players[0].order_cards_in_hand.push(OrderCardId(1));
        state = apply_move(&state, &Move::FulfillOrder { order_id: OrderCardId(1) }, &rules).unwrap();
        assert_eq!(state.players[0].mercenaries.total(), 0);

        // 2nd: count=2 -> first mercenary awarded.
        state.players[0].order_cards_in_hand.push(OrderCardId(2));
        state = apply_move(&state, &Move::FulfillOrder { order_id: OrderCardId(2) }, &rules).unwrap();
        assert_eq!(state.players[0].mercenaries.available.get(&country).copied().unwrap_or(0), 1);
        assert_eq!(state.mercenary_supply.english, starting_supply - 1);

        // 3rd: count=3 -> no NEW award (multiples-of-2 rule).
        state.players[0].order_cards_in_hand.push(OrderCardId(3));
        state = apply_move(&state, &Move::FulfillOrder { order_id: OrderCardId(3) }, &rules).unwrap();
        assert_eq!(state.players[0].mercenaries.available.get(&country).copied().unwrap_or(0), 1);
        assert_eq!(state.mercenary_supply.english, starting_supply - 1);

        // 4th: count=4 -> second mercenary awarded.
        state.players[0].order_cards_in_hand.push(OrderCardId(4));
        state = apply_move(&state, &Move::FulfillOrder { order_id: OrderCardId(4) }, &rules).unwrap();
        assert_eq!(state.players[0].mercenaries.available.get(&country).copied().unwrap_or(0), 2);
        assert_eq!(state.mercenary_supply.english, starting_supply - 2);
    }

    #[test]
    fn test_fulfill_order_no_mercenary_award_when_supply_empty() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        state.mercenary_supply.dutch = 0; // deplete supply before any awards are due

        let country = Country::Dutch;
        let mut order_registry = OrderRegistry::new();
        for i in 1..=2u8 {
            order_registry.insert(
                OrderCard {
                    id: OrderCardId(i),
                    country,
                    requirements: TradeGoodRequirement::default(),
                    rewards: OrderRewards::default(),
                },
                &format!("test_dutch_order_{i}"),
            );
        }
        let mut rules = Rules::empty_for_tests();
        rules.orders = order_registry;

        state.players[0].order_cards_in_hand.push(OrderCardId(1));
        state = apply_move(&state, &Move::FulfillOrder { order_id: OrderCardId(1) }, &rules).unwrap();
        state.players[0].order_cards_in_hand.push(OrderCardId(2));
        state = apply_move(&state, &Move::FulfillOrder { order_id: OrderCardId(2) }, &rules).unwrap();

        // Count reached 2 (would normally award), but supply is empty:
        // no award, no panic, supply stays at zero.
        assert_eq!(state.players[0].mercenaries.total(), 0);
        assert_eq!(state.mercenary_supply.dutch, 0);
    }

    #[test]
    fn test_build_shophouse_awards_trade_good_space_reward() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        let starting_tea = state.players[0].tea;

        let mut site = BuildingSite::new([0, 0, 0, 0], 10);
        site.spaces[0] = SpaceReward { tea: 2, ..Default::default() };
        state.board_layout.building_sites.insert(area, site);
        let rules = Rules::empty_for_tests();

        // apply_move_unchecked: see test_build_shophouse_gaslight_vp_bonus.
        let result = apply_move_unchecked(&state, &Move::BuildShophouse { slot: 0 }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        assert_eq!(new_state.players[0].tea, starting_tea + 2);
        assert_eq!(new_state.players[0].victory_points, 0);
    }

    #[test]
    fn test_build_shophouse_awards_vp_and_yen_space_reward() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        let starting_money = state.players[0].money;

        let mut site = BuildingSite::new([0, 0, 0, 0], 10);
        site.spaces[1] = SpaceReward { vp: 2, yen: 1, ..Default::default() };
        state.board_layout.building_sites.insert(area, site);
        let rules = Rules::empty_for_tests();

        // apply_move_unchecked: see test_build_shophouse_gaslight_vp_bonus.
        let result = apply_move_unchecked(&state, &Move::BuildShophouse { slot: 1 }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        assert_eq!(new_state.players[0].victory_points, 2);
        assert_eq!(new_state.players[0].money, starting_money + 1);
    }

    #[test]
    fn test_build_shophouse_awards_any_good_and_warehouse_space_reward() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        let starting_copper = state.players[0].copper;
        let starting_hand = state.players[0].assistants.in_hand;
        let starting_warehouse = state.players[0].assistants.in_warehouse;

        let mut site = BuildingSite::new([0, 0, 0, 0], 10);
        site.spaces[2] = SpaceReward { any_good: 1, warehouse_tokens: 1, ..Default::default() };
        state.board_layout.building_sites.insert(area, site);
        let rules = Rules::empty_for_tests();

        // apply_move_unchecked: see test_build_shophouse_gaslight_vp_bonus.
        // Applied consistently through this test's whole chain (not just the
        // initial build) even though the two queued-choice resolutions below
        // would likely also pass the checked membership test on their own.
        let result = apply_move_unchecked(&state, &Move::BuildShophouse { slot: 2 }, &rules);
        assert!(result.is_ok());
        let mid = result.unwrap();

        // Neither reward auto-resolves anymore (design §5.1-5.2): both queue,
        // warehouse-take first (matches the reward struct's push order).
        assert_eq!(mid.pending_choices, vec![PendingChoice::WarehouseTake, PendingChoice::AnyGood]);
        assert_eq!(mid.players[0].copper, starting_copper);
        assert_eq!(mid.players[0].assistants.in_hand, starting_hand);
        assert_eq!(mid.players[0].assistants.in_warehouse, starting_warehouse);

        let after_wh = apply_move_unchecked(
            &mid, &Move::ChooseWarehouseTake { item: WarehouseItem::Assistant }, &rules,
        ).unwrap();
        assert_eq!(after_wh.pending_choices, vec![PendingChoice::AnyGood]);
        assert_eq!(after_wh.players[0].assistants.in_hand, starting_hand + 1);
        assert_eq!(after_wh.players[0].assistants.in_warehouse, starting_warehouse - 1);

        let done = apply_move_unchecked(&after_wh, &Move::ChooseAnyGood { good: TradeGood::Copper }, &rules).unwrap();
        assert_eq!(done.players[0].copper, starting_copper + 1);
        assert!(done.pending_choices.is_empty());
    }

    #[test]
    fn test_build_shophouse_vp_only_space_unchanged_by_refactor() {
        // Regression pin: no test previously exercised apply_build_shophouse
        // directly (this file's other BuildingSite::new([2,3,4,5], 10) users
        // are the trading-house passive-income tests). Confirms the
        // apply_space_reward refactor doesn't change vp-only building payout.
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.board_layout.building_sites.insert(area, BuildingSite::new([2, 3, 4, 5], 10));
        let rules = Rules::empty_for_tests();

        // apply_move_unchecked: see test_build_shophouse_gaslight_vp_bonus.
        let result = apply_move_unchecked(&state, &Move::BuildShophouse { slot: 2 }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        assert_eq!(new_state.players[0].victory_points, 4);
        assert_eq!(new_state.players[0].shophouses.built.get(&area), Some(&2));
    }

    #[test]
    fn test_build_shophouse_copper_reward_mining_technology_bonus() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        let starting_copper = state.players[0].copper;

        let mut site = BuildingSite::new([0, 0, 0, 0], 10);
        site.spaces[0] = SpaceReward { copper: 2, ..Default::default() };
        state.board_layout.building_sites.insert(area, site);

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(mining_tech_fixture(), "tech_mining");
        state.players[0].technologies.push(TechCardId(1));

        // apply_move_unchecked: see test_build_shophouse_gaslight_vp_bonus.
        let result = apply_move_unchecked(&state, &Move::BuildShophouse { slot: 0 }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        // Base 2 copper from the explicit copper field + 1 Mining bonus (1 owned copy).
        assert_eq!(new_state.players[0].copper, starting_copper + 3);
    }

    #[test]
    fn test_build_shophouse_any_good_reward_mining_technology_bonus() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        let starting_copper = state.players[0].copper;

        let mut site = BuildingSite::new([0, 0, 0, 0], 10);
        site.spaces[0] = SpaceReward { any_good: 1, ..Default::default() };
        state.board_layout.building_sites.insert(area, site);

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(mining_tech_fixture(), "tech_mining");
        state.players[0].technologies.push(TechCardId(1));

        // apply_move_unchecked: see test_build_shophouse_gaslight_vp_bonus.
        let mid = apply_move_unchecked(&state, &Move::BuildShophouse { slot: 0 }, &rules).unwrap();
        // any_good queues a choice instead of auto-granting copper at build
        // time (design §5.3-2).
        assert_eq!(mid.pending_choices, vec![PendingChoice::AnyGood]);
        assert_eq!(mid.players[0].copper, starting_copper);

        // Resolving the choice as Copper: 1 base + 1 Mining bonus (1 owned
        // copy). Mining applies because the CHOSEN good IS copper.
        let done = apply_move_unchecked(&mid, &Move::ChooseAnyGood { good: TradeGood::Copper }, &rules).unwrap();
        assert_eq!(done.players[0].copper, starting_copper + 2);
    }

    #[test]
    fn test_build_shophouse_non_copper_reward_mining_technology_no_bonus() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        let starting_copper = state.players[0].copper;
        let starting_tea = state.players[0].tea;

        let mut site = BuildingSite::new([0, 0, 0, 0], 10);
        site.spaces[0] = SpaceReward { tea: 2, ..Default::default() };
        state.board_layout.building_sites.insert(area, site);

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(mining_tech_fixture(), "tech_mining");
        state.players[0].technologies.push(TechCardId(1));

        // apply_move_unchecked: see test_build_shophouse_gaslight_vp_bonus.
        let result = apply_move_unchecked(&state, &Move::BuildShophouse { slot: 0 }, &rules);
        assert!(result.is_ok());
        let new_state = result.unwrap();

        // A reward with no copper and no any_good must not trigger Mining.
        assert_eq!(new_state.players[0].tea, starting_tea + 2);
        assert_eq!(new_state.players[0].copper, starting_copper);
    }

    #[test]
    fn test_build_shophouse_copper_and_any_good_are_separate_mining_receipts() {
        // Design 2026-07-18 §5.3-2: Mining Technology receipts are per-icon.
        // A space with BOTH an explicit copper field and an any_good field
        // now produces TWO separate copper-receipt events -- the explicit
        // field at build time, and (if resolved to Copper) the any-good
        // choice at resolution time -- each independently eligible for the
        // flat per-copy Mining bonus. This supersedes the old "combined
        // receipt" precompute that fired the bonus once for both fields
        // together (that precompute is dismantled by this task).
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        let starting_copper = state.players[0].copper;

        let mut site = BuildingSite::new([0, 0, 0, 0], 10);
        site.spaces[0] = SpaceReward { copper: 1, any_good: 1, ..Default::default() };
        state.board_layout.building_sites.insert(area, site);

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(mining_tech_fixture(), "tech_mining");
        state.players[0].technologies.push(TechCardId(1));

        // apply_move_unchecked: see test_build_shophouse_gaslight_vp_bonus.
        let mid = apply_move_unchecked(&state, &Move::BuildShophouse { slot: 0 }, &rules).unwrap();
        // Explicit copper receipt fires immediately: 1 base + 1 Mining bonus.
        assert_eq!(mid.players[0].copper, starting_copper + 2);
        assert_eq!(mid.pending_choices, vec![PendingChoice::AnyGood]);

        let done = apply_move_unchecked(&mid, &Move::ChooseAnyGood { good: TradeGood::Copper }, &rules).unwrap();
        // Any-good-as-copper is its OWN receipt: another 1 base + 1 Mining bonus.
        assert_eq!(done.players[0].copper, starting_copper + 4);
    }

    #[test]
    fn port_action_takes_card_refills_and_pays_second() {
        let rules = Rules::standard();
        let mut state = create_test_game_state(2);
        let port_area = AreaId::new(0);
        state.board_layout.areas.insert(port_area, AreaConfig {
            area_type: AreaType::PortA,
            grid_position: (0, 0),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(port_area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(4);
        // 6 slots: Low(1,2), Mid(3,4), High(5,6).
        state.port_state.port_a.slots = [1, 2, 3, 4, 5, 6].map(|id| Some(OrderCardId(id)));
        state.order_deck = vec![OrderCardId(10)];
        state.players[0].money = 5;

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Port {
                cards_taken: vec![OrderCardId(1), OrderCardId(3)], // Low(1) + Mid(3)
                second_card_payment: Some(PortPayment::TwoYen),
            },
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        let p = &next.players[0];
        assert_eq!(p.order_cards_in_hand, vec![OrderCardId(1), OrderCardId(3)]);
        assert_eq!(p.money, 3); // paid 2 yen for second card

        // Taking indices 0 and 2 leaves cards 2,4,5,6, which compact to
        // indices 0-3 (rules ref 01-rules-reference.md: "Slide cards left,
        // refill from deck to rightmost spaces"). With only 1 card in the
        // deck, refill fills the rightmost empty space (index 5), leaving
        // index 4 empty.
        assert_eq!(
            next.port_state.port_a.slots,
            [
                Some(OrderCardId(2)),
                Some(OrderCardId(4)),
                Some(OrderCardId(5)),
                Some(OrderCardId(6)),
                None,
                Some(OrderCardId(10)),
            ]
        );
        assert!(next.order_deck.is_empty());
    }

    #[test]
    fn port_action_during_mercenary_sub_turn_resolves_mercenary_target_area() {
        // President is in hand (default) and never placed anywhere; a Port
        // action must still work by resolving the acting area from
        // mercenary_target_area, not president_location.
        let rules = Rules::standard();
        let mut state = create_test_game_state(2);
        let port_area = AreaId::new(5);
        state.board_layout.areas.insert(port_area, AreaConfig {
            area_type: AreaType::PortA,
            grid_position: (0, 5),
        });
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(port_area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(2); // power 2 -> Low only, 1 card, free
        state.port_state.port_a.slots[0] = Some(OrderCardId(7));

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Port {
                cards_taken: vec![OrderCardId(7)],
                second_card_payment: None,
            },
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        assert_eq!(next.players[0].order_cards_in_hand, vec![OrderCardId(7)]);
    }

    #[test]
    fn mercenary_port_take_at_five_grants() {
        // Pin task (Task 8, 2026-07-18, design §5.3-1): a mercenary sub-turn
        // Port take runs through the same execute_port_action path as a
        // normal turn (area resolved from mercenary_target_area instead of
        // president_location), so the take-gated Power-5 +3 VP bonus must
        // fire there too.
        let rules = Rules::standard();
        let mut state = create_test_game_state(2);
        let port_area = AreaId::new(5);
        state.board_layout.areas.insert(port_area, AreaConfig {
            area_type: AreaType::PortA,
            grid_position: (0, 5),
        });
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(port_area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(5);
        state.port_state.port_a.slots[0] = Some(OrderCardId(7));
        let starting_vp = state.players[0].victory_points;

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Port {
                cards_taken: vec![OrderCardId(7)],
                second_card_payment: None,
            },
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        assert_eq!(next.players[0].order_cards_in_hand, vec![OrderCardId(7)]);
        assert_eq!(next.players[0].victory_points, starting_vp + 3);
    }

    /// Places a LaboratoryA area on the board and puts the president there.
    fn place_president_at_lab_a(state: &mut GameState) -> AreaId {
        let lab_area = AreaId::new(0);
        state.board_layout.areas.insert(lab_area, AreaConfig {
            area_type: AreaType::LaboratoryA,
            grid_position: (0, 0),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(lab_area);
        lab_area
    }

    #[test]
    fn lab_action_acquires_tech_and_refills() {
        let rules = Rules::standard();
        let mut state = create_test_game_state(2);
        place_president_at_lab_a(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);
        // TechCardId(24) = Mining Technology, German, industry_cost 3 (data.rs) —
        // an unrelated tech, so this test stays focused on wiring + refill.
        state.lab_state.lab_a.slots[0] = Some(TechCardId(24));
        state.tech_deck = vec![TechCardId(200)];

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Laboratory {
                card_id: TechCardId(24), extra_yen: 0, extra_imported: 0, fifth_tech_payment: None,
            },
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        assert!(next.players[0].technologies.contains(&TechCardId(24)));
        // Only slot 0 was occupied; taking it leaves the whole board empty,
        // so refill picks the rightmost empty slot (index 5).
        assert_eq!(next.lab_state.lab_a.slots[5], Some(TechCardId(200)));
        assert!(next.tech_deck.is_empty());
    }

    #[test]
    fn lab_action_during_mercenary_sub_turn_resolves_mercenary_target_area() {
        // President is in hand (default) and never placed anywhere; a
        // Laboratory action must still work by resolving the acting area
        // from mercenary_target_area, not president_location.
        let rules = Rules::standard();
        let mut state = create_test_game_state(2);
        let lab_area = AreaId::new(5);
        state.board_layout.areas.insert(lab_area, AreaConfig {
            area_type: AreaType::LaboratoryA,
            grid_position: (0, 5),
        });
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(lab_area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);
        state.lab_state.lab_a.slots[0] = Some(TechCardId(24));

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Laboratory {
                card_id: TechCardId(24), extra_yen: 0, extra_imported: 0, fifth_tech_payment: None,
            },
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        assert!(next.players[0].technologies.contains(&TechCardId(24)));
    }

    #[test]
    fn lab_action_rejects_card_on_other_lab_board() {
        // Cross-board buying is illegal: the president acts at Lab A, but
        // the named card lives only on Lab B, so apply_move must surface the
        // typed MoveError::CardNotFound, not a generic RuleViolation.
        let rules = Rules::standard();
        let mut state = create_test_game_state(2);
        place_president_at_lab_a(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);
        state.lab_state.lab_b = Some(LabBoard::new());
        state.lab_state.lab_b.as_mut().unwrap().slots[0] = Some(TechCardId(24));

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Laboratory {
                card_id: TechCardId(24), extra_yen: 0, extra_imported: 0, fifth_tech_payment: None,
            },
        };
        // apply_move_unchecked: get_legal_moves only enumerates Laboratory
        // actions for cards actually present on the acting board, so this
        // cross-board card id is never legal -- targets the raw arm's
        // CardNotFound check directly.
        let result = apply_move_unchecked(&state, &mv, &rules);
        assert!(matches!(result, Err(MoveError::CardNotFound)));
    }

    #[test]
    fn lab_action_awards_patent_system_vp_bonus() {
        let rules = Rules::standard();
        let mut state = create_test_game_state(2);
        place_president_at_lab_a(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);
        // TechCardId(26) = Patent System (real data; only 1 physical copy).
        state.players[0].technologies = vec![TechCardId(26)];
        let starting_vp = state.players[0].victory_points;
        state.lab_state.lab_a.slots[0] = Some(TechCardId(24)); // unrelated tech being bought

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Laboratory {
                card_id: TechCardId(24), extra_yen: 0, extra_imported: 0, fifth_tech_payment: None,
            },
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        assert!(next.players[0].technologies.contains(&TechCardId(24)));
        assert_eq!(next.players[0].victory_points, starting_vp + 2);
    }

    #[test]
    fn lab_action_awards_letterpress_yen_bonus_stacking_two_owned_copies() {
        let rules = Rules::standard();
        let mut state = create_test_game_state(2);
        place_president_at_lab_a(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);
        // TechCardId(18)/(19) = both real Letterpress Printing physical copies.
        state.players[0].technologies = vec![TechCardId(18), TechCardId(19)];
        let starting_money = state.players[0].money;
        state.lab_state.lab_a.slots[0] = Some(TechCardId(24));

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Laboratory {
                card_id: TechCardId(24), extra_yen: 0, extra_imported: 0, fifth_tech_payment: None,
            },
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        // Cost is fully covered by power alone (0 extra yen), so the only
        // money change is +1 Yen per owned Letterpress copy (2 owned).
        assert_eq!(next.players[0].money, starting_money + 2);
    }

    #[test]
    fn lab_action_exposition_self_trigger_awards_10_vp() {
        let rules = Rules::standard();
        let mut state = create_test_game_state(2);
        place_president_at_lab_a(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(5); // Exposition costs 5, slot 0 has no surcharge.
        let starting_vp = state.players[0].victory_points;
        // TechCardId(9) = Exposition (real data), being bought.
        state.lab_state.lab_a.slots[0] = Some(TechCardId(9));

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Laboratory {
                card_id: TechCardId(9), extra_yen: 0, extra_imported: 0, fifth_tech_payment: None,
            },
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        assert!(next.players[0].technologies.contains(&TechCardId(9)));
        assert_eq!(next.players[0].victory_points, starting_vp + 10);
    }

    #[test]
    fn lab_action_reruns_mercenary_eligibility_for_acquired_country() {
        let rules = Rules::standard();
        let mut state = create_test_game_state(2);
        place_president_at_lab_a(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);
        // TechCardId(29) = Brickyard, German (already owned).
        state.players[0].technologies = vec![TechCardId(29)];
        // TechCardId(24) = Mining Technology, German (being bought) -> 2 German icons.
        state.lab_state.lab_a.slots[0] = Some(TechCardId(24));

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Laboratory {
                card_id: TechCardId(24), extra_yen: 0, extra_imported: 0, fifth_tech_payment: None,
            },
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        assert_eq!(next.players[0].mercenaries.available.get(&Country::German), Some(&1));
        assert_eq!(next.mercenary_supply.german, 1); // started at 2, one awarded
    }

    #[test]
    fn lab_action_wires_fifth_tech_payment() {
        let rules = Rules::standard();
        let mut state = create_test_game_state(2);
        place_president_at_lab_a(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);
        state.players[0].money = 10;
        // 4 existing techs -> the 5th acquisition requires the penalty.
        state.players[0].technologies =
            vec![TechCardId(1), TechCardId(2), TechCardId(3), TechCardId(4)];
        state.lab_state.lab_a.slots[0] = Some(TechCardId(24)); // cost 3, covered by power alone

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Laboratory {
                card_id: TechCardId(24), extra_yen: 0, extra_imported: 0,
                fifth_tech_payment: Some(true), // pay 2 Yen
            },
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        assert!(next.players[0].technologies.contains(&TechCardId(24)));
        assert_eq!(next.players[0].technologies.len(), 5);
        assert_eq!(next.players[0].money, 10 - 2);
    }

    /// Places a Church area on the board and puts the president there.
    fn place_president_at_church(state: &mut GameState) -> AreaId {
        let church_area = AreaId::new(0);
        state.board_layout.areas.insert(church_area, AreaConfig {
            area_type: AreaType::Church,
            grid_position: (0, 0),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(church_area);
        church_area
    }

    #[test]
    fn church_action_places_assistant_and_awards_vp() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(2);
        place_president_at_church(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);
        state.players[0].copper = 1;
        state.players[0].silk = 1;

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Church {
                copper: true, silk: true, tea: false, seafood: false, yen: false, imported_good: false,
            },
        };
        let mid = apply_move(&state, &mv, &rules).unwrap();
        // Faith 5 -> 2 eligible spaces (4,5) on a fresh track -> goods paid,
        // placement deferred behind a ChurchSpace choice (design §5.2).
        assert_eq!(mid.pending_choices, vec![PendingChoice::ChurchSpace { total_faith: 5 }]);
        let next = apply_move(&mid, &Move::ChooseChurchSpace { space: 5 }, &rules).unwrap();

        let p = &next.players[0];
        assert_eq!(p.assistants.in_hand, 7);
        assert_eq!(p.assistants.on_church, 1);
        assert_eq!(p.copper, 0);
        assert_eq!(p.silk, 0);
        assert_eq!(
            next.church_track.slots[ChurchTrack::faith_to_index(5).unwrap()].unwrap().player,
            p.id
        );
        assert_eq!(p.church_positions, vec![5]);
        assert_eq!(p.victory_points, ChurchTrack::vp_reward(5) as u32);
    }

    #[test]
    fn church_action_during_mercenary_sub_turn_resolves_mercenary_target_area() {
        // President is in hand (default) and never placed anywhere; a
        // Church action must still work by resolving the acting area from
        // mercenary_target_area, not president_location.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(2);
        let church_area = AreaId::new(5);
        state.board_layout.areas.insert(church_area, AreaConfig {
            area_type: AreaType::Church,
            grid_position: (0, 5),
        });
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(church_area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(4);

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Church {
                copper: false, silk: false, tea: false, seafood: false, yen: false, imported_good: false,
            },
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        assert_eq!(next.players[0].church_positions, vec![4]);
    }

    /// Places a Customs area on the board and puts the president there.
    fn place_president_at_customs(state: &mut GameState) -> AreaId {
        let customs_area = AreaId::new(0);
        state.board_layout.areas.insert(customs_area, AreaConfig {
            area_type: AreaType::Customs,
            grid_position: (0, 0),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(customs_area);
        customs_area
    }

    #[test]
    fn customs_action_places_assistant_and_awards_vp() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(2);
        place_president_at_customs(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);
        state.players[0].imported_goods = 1;

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Customs { imported_goods_paid: 1 },
        };
        let mid = apply_move(&state, &mv, &rules).unwrap();
        // Cost 1 -> spaces 1,2 both empty on a fresh track -> 2 eligible ->
        // placement deferred behind a CustomsSpace choice (design §5.2).
        assert_eq!(mid.pending_choices, vec![PendingChoice::CustomsSpace { cost: 1 }]);
        let next = apply_move(&mid, &Move::ChooseCustomsSpace { space: 1 }, &rules).unwrap();

        let p = &next.players[0];
        assert_eq!(p.assistants.on_customs, 1);
        assert_eq!(p.imported_goods, 0);
        assert_eq!(
            next.customs_track.slots[CustomsTrack::space_to_index(1).unwrap()].unwrap().player,
            p.id
        );
        assert_eq!(p.customs_positions, vec![1]);
        assert_eq!(p.victory_points, CustomsTrack::vp_reward(1) as u32);
    }

    #[test]
    fn customs_action_during_mercenary_sub_turn_resolves_mercenary_target_area() {
        // President is in hand (default) and never placed anywhere; a
        // Customs action must still work by resolving the acting area from
        // mercenary_target_area, not president_location.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(2);
        // Finding D1: create_test_game_state builds an ad-hoc 2P state without
        // the setup dummy; place it so the space-4 block below is occupancy-
        // derived (this fixture uses the scoring-shared helper, which stays
        // dummy-free for the isolated majority tests).
        crate::game::place_setup_dummies(&mut state);
        let customs_area = AreaId::new(5);
        state.board_layout.areas.insert(customs_area, AreaConfig {
            area_type: AreaType::Customs,
            grid_position: (0, 5),
        });
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(customs_area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(4);
        state.players[0].imported_goods = 2;

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Customs { imported_goods_paid: 2 },
        };
        let next = apply_move(&state, &mv, &rules).unwrap();

        // Power 4 (max 3), pay 2 -> cost pair {3,4}; this fixture is 2P, so
        // the printed dummy pawn blocks space 4, leaving space 3 as the
        // sole candidate -> places inline, no CustomsSpace choice (mirrors
        // customs_2p_space4_dummy_never_offered in tracks.rs, exercised
        // here through the mercenary path).
        assert_eq!(next.players[0].customs_positions, vec![3]);
        assert!(next.pending_choices.iter().all(|c| !matches!(c, PendingChoice::CustomsSpace { .. })));
    }

    #[test]
    fn skip_is_legal_at_each_permissive_area() {
        // Task 16 optionality ruling: Skip is legal at the six areas with
        // optional components, in a 4-player game so PortB/LaboratoryB
        // boards exist too.
        let rules = Rules::empty_for_tests();
        let permissive = [
            AreaType::PortA,
            AreaType::PortB,
            AreaType::LaboratoryA,
            AreaType::LaboratoryB,
            AreaType::EmploymentAgency,
            AreaType::Chinatown,
            AreaType::Church,
            AreaType::Customs,
        ];

        for (i, &area_type) in permissive.iter().enumerate() {
            let mut state = create_test_game_state(4);
            let area = AreaId::new(i as u8);
            state.board_layout.areas.insert(area, AreaConfig { area_type, grid_position: (0, i as u8) });
            state.players[0].president_location = PresidentLocation::OnBoard(area);
            state.turn_phase = TurnPhase::AreaAction;
            state.current_action_power = Some(3);

            let mv = Move::ExecuteAreaAction { action: AreaAction::Skip };
            let result = apply_move(&state, &mv, &rules);
            assert!(result.is_ok(), "Skip should be legal at {area_type:?}: {result:?}");
        }
    }

    #[test]
    fn skip_is_illegal_at_production_areas_and_bank() {
        // Production areas and the Bank have no optional component: their
        // imperative "Gain ..." move is the only legal action there.
        let rules = Rules::empty_for_tests();
        let no_skip = [
            AreaType::CopperMine,
            AreaType::SilkMill,
            AreaType::TeaPlantation,
            AreaType::FishingGround,
            AreaType::Bank,
        ];

        for (i, &area_type) in no_skip.iter().enumerate() {
            let mut state = create_test_game_state(2);
            let area = AreaId::new(i as u8);
            state.board_layout.areas.insert(area, AreaConfig { area_type, grid_position: (0, i as u8) });
            state.players[0].president_location = PresidentLocation::OnBoard(area);
            state.turn_phase = TurnPhase::AreaAction;
            state.current_action_power = Some(3);

            let mv = Move::ExecuteAreaAction { action: AreaAction::Skip };
            let result = apply_move(&state, &mv, &rules);
            assert!(result.is_err(), "Skip should be illegal at {area_type:?}");
        }
    }

    #[test]
    fn skip_grants_nothing() {
        // Skip carries out the area action while declining every optional
        // component: no cards, goods, or VP change hands.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig { area_type: AreaType::Church, grid_position: (0, 0) });
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);
        state.players[0].money = 5;
        state.players[0].copper = 2;
        state.players[0].imported_goods = 1;
        let before = state.players[0].clone();

        let mv = Move::ExecuteAreaAction { action: AreaAction::Skip };
        let next = apply_move(&state, &mv, &rules).unwrap();

        let after = &next.players[0];
        assert_eq!(after.money, before.money);
        assert_eq!(after.copper, before.copper);
        assert_eq!(after.imported_goods, before.imported_goods);
        assert_eq!(after.victory_points, before.victory_points);
        assert_eq!(after.order_cards_in_hand, before.order_cards_in_hand);
        assert_eq!(after.technologies, before.technologies);
    }

    #[test]
    fn skip_at_power_5_port_does_not_grant_vp_bonus() {
        // v1 ruling: Skip declines all optional components, including the
        // Port power-5 +3 VP bonus, which requires an actual take.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig { area_type: AreaType::PortA, grid_position: (0, 0) });
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(5);
        let starting_vp = state.players[0].victory_points;

        let mv = Move::ExecuteAreaAction { action: AreaAction::Skip };
        let next = apply_move(&state, &mv, &rules).unwrap();

        assert_eq!(next.players[0].victory_points, starting_vp);
    }

    #[test]
    fn skip_at_five_power_port_grants_no_vp() {
        // owner-adjudicated 2026-07-18: +3 VP is take-gated; full hand bars
        // the action; Skip grants nothing (design §5.3-1).
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig { area_type: AreaType::PortA, grid_position: (0, 0) });
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(5);
        let starting_vp = state.players[0].victory_points;

        let mv = Move::ExecuteAreaAction { action: AreaAction::Skip };
        let next = apply_move(&state, &mv, &rules).unwrap();

        assert_eq!(next.players[0].victory_points, starting_vp);
    }

    #[test]
    fn skip_legal_during_mercenary_sub_turn_resolves_mercenary_target_area() {
        // President is in hand (default) and never placed anywhere; Skip
        // must still work by resolving the acting area from
        // mercenary_target_area, not president_location (mirrors the
        // Customs mercenary sub-turn test above).
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(2);
        let church_area = AreaId::new(5);
        state.board_layout.areas.insert(church_area, AreaConfig {
            area_type: AreaType::Church,
            grid_position: (0, 5),
        });
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(church_area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(2);

        let mv = Move::ExecuteAreaAction { action: AreaAction::Skip };
        let result = apply_move(&state, &mv, &rules);
        assert!(result.is_ok());
    }

    /// Places an Employment Agency area on the board and puts the president there.
    fn place_president_at_employment_agency(state: &mut GameState) -> AreaId {
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::EmploymentAgency,
            grid_position: (0, 0),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        area
    }

    /// Places a Chinatown area on the board and puts the president there.
    fn place_president_at_chinatown(state: &mut GameState) -> AreaId {
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Chinatown,
            grid_position: (0, 0),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        area
    }

    #[test]
    fn chinatown_rejects_empty_exchanges() {
        // Task 17 ruling: Chinatown{exchanges: []} is no longer a legal
        // encoding of "decline Chinatown" -- Skip is the sole canonical
        // decline (mirrors Task 16's Port `cards_taken.is_empty()` rule).
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(2);
        place_president_at_chinatown(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);

        let mv = Move::ExecuteAreaAction { action: AreaAction::Chinatown { exchanges: vec![] } };
        let result = apply_move(&state, &mv, &rules);
        assert!(result.is_err());
    }

    #[test]
    fn chinatown_rejects_exchange_count_over_power() {
        // Task 17b (KB 07-rules-reference.md line 312: "Number of exchanges
        // = Power level"): 3 exchanges at power 2 exceeds the cap and must
        // be rejected -- before any exchange executes, so nothing mutates.
        // Each individual exchange is otherwise resource-valid (money=10 is
        // plenty for 3 x YenToTea), so the ONLY reason for rejection is the
        // count cap, not an incidental resource shortfall.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(2);
        place_president_at_chinatown(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(2);
        state.players[0].money = 10;
        let money_before = state.players[0].money;
        let tea_before = state.players[0].tea;

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::Chinatown {
                exchanges: vec![ChinatownExchange::YenToTea, ChinatownExchange::YenToTea, ChinatownExchange::YenToTea],
            },
        };
        let result = apply_move(&state, &mv, &rules);
        assert!(result.is_err());
        // Zero mutation: the original state (passed by shared reference)
        // is untouched, and there is no successful new state to inspect.
        assert_eq!(state.players[0].money, money_before);
        assert_eq!(state.players[0].tea, tea_before);
    }

    #[test]
    fn employment_agency_rejects_all_zero_items() {
        // Task 17 ruling: EmploymentAgency{0,0,0} is no longer a legal
        // encoding of "decline Employment Agency" -- Skip is the sole
        // canonical decline.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(2);
        place_president_at_employment_agency(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);

        let mv = Move::ExecuteAreaAction {
            action: AreaAction::EmploymentAgency { assistants: 0, shophouses: 0, trading_houses: 0 },
        };
        let result = apply_move(&state, &mv, &rules);
        assert!(result.is_err());
    }

    #[test]
    fn customs_rejects_zero_payment_via_apply_move() {
        // Move-level symmetry check (executor-level coverage already exists:
        // test_execute_customs_action_rejects_zero_payment in actions/tracks.rs).
        let rules = Rules::empty_for_tests();
        let mut state = create_test_game_state(2);
        place_president_at_customs(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);
        state.players[0].imported_goods = 3;

        let mv = Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid: 0 } };
        let result = apply_move(&state, &mv, &rules);
        assert!(result.is_err());
    }

    // --- Pending-choice queue + AnyGood (design 2026-07-18 §5.1-5.2) ---

    /// Order O01 in hand, rewarding `any_trade_good` icons and nothing else
    /// (default requirements so the reward path is reachable unconditionally).
    fn any_good_order_fixture(any_trade_good: u8) -> (GameState, Rules) {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        let order_id = OrderCardId(1);
        state.players[0].order_cards_in_hand.push(order_id);

        let mut order_registry = OrderRegistry::new();
        order_registry.insert(
            OrderCard {
                id: order_id,
                country: Country::English,
                requirements: TradeGoodRequirement::default(),
                rewards: OrderRewards { any_trade_good, ..Default::default() },
            },
            "test_any_good_order",
        );
        let mut rules = Rules::empty_for_tests();
        rules.orders = order_registry;

        (state, rules)
    }

    /// The acting area shared by `building_any_good_fixture` and its test --
    /// a free function (not a state field) so the test can name it without
    /// holding a reference into the fixture's state.
    fn acting_area() -> AreaId {
        AreaId::new(0)
    }

    /// Building phase, president on `acting_area()`, whose site TR space
    /// (`spaces[1]`) grants 1 any_good. Player holds a shophouse in hand and
    /// an assistant on the acting area, so Recovery deferral is observable.
    fn building_any_good_fixture() -> (GameState, Rules) {
        let mut state = create_minimal_state(2);
        let area = acting_area();
        state.board_layout.areas.insert(area, AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        let mut site = BuildingSite::new([0, 0, 0, 0], 0);
        site.spaces[1] = SpaceReward { any_good: 1, ..Default::default() }; // TR
        state.board_layout.building_sites.insert(area, site);
        state.turn_phase = TurnPhase::Building;
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(4); // Shophouse requires Power 4+
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].shophouses.in_hand = 1;
        state.players[0].assistants.on_board.insert(area, 1);
        let rules = Rules::empty_for_tests();
        (state, rules)
    }

    /// Order O01 in hand, rewarding 1 explicit copper + 1 any_trade_good;
    /// player owns 1 Mining Technology copy.
    fn mining_any_good_fixture() -> (GameState, Rules) {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        let order_id = OrderCardId(1);
        state.players[0].order_cards_in_hand.push(order_id);
        state.players[0].technologies.push(TechCardId(1));

        let mut order_registry = OrderRegistry::new();
        order_registry.insert(
            OrderCard {
                id: order_id,
                country: Country::English,
                requirements: TradeGoodRequirement::default(),
                rewards: OrderRewards { copper: 1, any_trade_good: 1, ..Default::default() },
            },
            "test_mining_any_good_order",
        );
        let mut rules = Rules::empty_for_tests();
        rules.orders = order_registry;
        rules.techs.insert(mining_tech_fixture(), "tech_mining");

        (state, rules)
    }

    #[test]
    fn fulfill_order_with_any_good_queues_choice_and_gates_legal_moves() {
        // Fixture: an order whose rewards have any_trade_good: 2, player holds it + goods.
        let (mut state, rules) = any_good_order_fixture(2); // helper below
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        let after = apply_move(&state, &Move::FulfillOrder { order_id: OrderCardId(1) }, &rules).unwrap();
        assert_eq!(after.pending_choices, vec![PendingChoice::AnyGood, PendingChoice::AnyGood]);
        let legal = get_legal_moves(&after, &rules);
        assert_eq!(legal.len(), 4); // exactly the four ChooseAnyGood moves, nothing else
        assert!(legal.iter().all(|m| matches!(m, Move::ChooseAnyGood { .. })));
        // No copper was auto-granted:
        assert_eq!(after.current_player().copper, state.current_player().copper);
    }

    #[test]
    fn choose_any_good_resolves_head_and_grants_chosen_good() {
        let (state, rules) = any_good_order_fixture(1);
        let mut s = apply_move(&state, &Move::FulfillOrder { order_id: OrderCardId(1) }, &rules).unwrap();
        let silk_before = s.current_player().silk;
        s = apply_move(&s, &Move::ChooseAnyGood { good: TradeGood::Silk }, &rules).unwrap();
        assert_eq!(s.current_player().silk, silk_before + 1);
        assert!(s.pending_choices.is_empty());
    }

    #[test]
    fn choice_moves_illegal_when_no_choice_pending_and_wrong_choice_rejected() {
        let (state, rules) = any_good_order_fixture(1);
        assert!(apply_move(&state, &Move::ChooseAnyGood { good: TradeGood::Tea }, &rules).is_err());
    }

    #[test]
    fn shophouse_space_any_good_queues_and_defers_recovery() {
        // Fixture: Building phase, president on an area whose site TR space has any_good: 1,
        // player has a shophouse in hand and an assistant on the acting area.
        let (state, rules) = building_any_good_fixture();
        // apply_move_unchecked: the fixture hardcodes slot 1 to land on the
        // TR any_good space, but next_shophouse_slot allocates slot 0 first
        // (it's the only empty one here), so get_legal_moves never offers
        // slot 1 -- this test targets the raw build-reward arm directly.
        let mid = apply_move_unchecked(&state, &Move::BuildShophouse { slot: 1 }, &rules).unwrap();
        assert_eq!(mid.pending_choices, vec![PendingChoice::AnyGood]);
        // Recovery DEFERRED: the acting-area assistant has NOT been recovered yet.
        assert!(mid.current_player().assistants.on_board.contains_key(&acting_area()));
        let done = apply_move_unchecked(&mid, &Move::ChooseAnyGood { good: TradeGood::Copper }, &rules).unwrap();
        // Resolving the last choice runs the deferred Recovery + transition in the same apply.
        assert!(!done.current_player().assistants.on_board.contains_key(&acting_area()));
        assert_eq!(done.turn_phase, TurnPhase::AdditionalActionSecond);
    }

    #[test]
    fn any_good_copper_choice_fires_mining_bonus_per_icon() {
        // Player owns 1 Mining Technology; order rewards: copper 1, any_trade_good 1.
        let (state, rules) = mining_any_good_fixture();
        let s1 = apply_move(&state, &Move::FulfillOrder { order_id: OrderCardId(1) }, &rules).unwrap();
        // Explicit copper receipt fired at the trigger: +1 copper +1 mining.
        let s2 = apply_move(&s1, &Move::ChooseAnyGood { good: TradeGood::Copper }, &rules).unwrap();
        // Any-good-as-copper is its OWN receipt: +1 copper +1 mining again (design §5.3-2).
        assert_eq!(s2.current_player().copper, state.current_player().copper + 4);
    }

    // --- RecoveryLeave (Telephone) choice (design 2026-07-18 §5.2, §5.3-5, Task 6) ---

    fn telephone_tech_fixture(id: u8) -> TechCard {
        TechCard {
            id: TechCardId(id),
            name: "Telephone".to_string(),
            country: Country::American,
            industry_cost: 6,
            effect: TechEffect::Telephone,
        }
    }

    /// Building phase, president on `acting_area()` with `assistants` of the
    /// player's own assistants there, no site rewards -- `SkipBuilding`
    /// drives straight into Recovery with no other choice already queued.
    fn plain_building_fixture(assistants: u8) -> (GameState, Rules) {
        let mut state = create_minimal_state(2);
        let area = acting_area();
        state.board_layout.areas.insert(area, AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        state.turn_phase = TurnPhase::Building;
        state.current_player = PlayerId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.on_board.insert(area, assistants);
        let rules = Rules::empty_for_tests();
        (state, rules)
    }

    /// `plain_building_fixture` plus the current player owning 1 Telephone.
    fn telephone_building_fixture(assistants: u8) -> (GameState, Rules) {
        let (mut state, mut rules) = plain_building_fixture(assistants);
        rules.techs.insert(telephone_tech_fixture(1), "tech_telephone");
        state.players[0].technologies.push(TechCardId(1));
        (state, rules)
    }

    #[test]
    fn recovery_with_telephone_queues_leave_choice() {
        let (state, rules) = telephone_building_fixture(/*assistants on acting area*/ 2);
        let s = apply_move(&state, &Move::SkipBuilding, &rules).unwrap();
        assert_eq!(s.turn_phase, TurnPhase::Recovery); // paused, not auto-completed
        assert_eq!(s.pending_choices, vec![PendingChoice::RecoveryLeave]);
        assert_eq!(get_legal_moves(&s, &rules), vec![
            Move::ChooseRecoveryLeave { leave_one: false },
            Move::ChooseRecoveryLeave { leave_one: true },
        ]);
    }

    #[test]
    fn telephone_leave_one_keeps_lone_assistant() {
        // count == 1: leaving means recovering ZERO (design §5.3-5; fixes the old pin).
        let (state, rules) = telephone_building_fixture(1);
        let mid = apply_move(&state, &Move::SkipBuilding, &rules).unwrap();
        let s = apply_move(&mid, &Move::ChooseRecoveryLeave { leave_one: true }, &rules).unwrap();
        assert_eq!(s.current_player().assistants.on_board[&acting_area()], 1);
        assert_eq!(s.turn_phase, TurnPhase::AdditionalActionSecond);
    }

    #[test]
    fn recovery_without_telephone_stays_automatic() {
        let (state, rules) = plain_building_fixture(2);
        let s = apply_move(&state, &Move::SkipBuilding, &rules).unwrap();
        assert_eq!(s.turn_phase, TurnPhase::AdditionalActionSecond); // no pause
        assert!(s.pending_choices.is_empty());
    }

    #[test]
    fn telephone_leave_false_recovers_everyone() {
        // The complementary resolution to leave_one: true -- recovers all,
        // same outcome as the no-Telephone automatic path.
        let (state, rules) = telephone_building_fixture(2);
        let mid = apply_move(&state, &Move::SkipBuilding, &rules).unwrap();
        let s = apply_move(&mid, &Move::ChooseRecoveryLeave { leave_one: false }, &rules).unwrap();
        assert_eq!(s.current_player().assistants.on_board.get(&acting_area()), None);
        assert_eq!(s.turn_phase, TurnPhase::AdditionalActionSecond);
    }

    // --- Phase-2.9 Stage 4: raw-arm self-defense (census-driven guards) ---
    //
    // Each test below is the census probe's payload, replayed as a spec:
    // the raw arm must reject it with a typed MoveError (never panic, wrap,
    // or silently accept) and must leave the input state untouched. Tier C
    // (full legality re-derivation: assistant-chain traversal, teleport
    // tech entitlement, affordability enumeration) deliberately stays the
    // CHECKED boundary's job -- see the guard doc comments.

    /// `count` Bank areas 0..count on the board, player 0 with a full hand.
    fn stage4_placement_fixture(count: u8) -> GameState {
        let mut state = create_minimal_state(2);
        for i in 0..count {
            state.board_layout.areas.insert(AreaId::new(i), AreaConfig {
                area_type: AreaType::Bank,
                grid_position: (0, i),
            });
        }
        state.turn_phase = TurnPhase::Placement;
        state.players[0].assistants.in_hand = 8;
        state.players[0].money = 5;
        state
    }

    fn newspaper_tech_fixture(id: u8) -> TechCard {
        TechCard {
            id: TechCardId(id),
            name: "Newspaper".to_string(),
            country: Country::Dutch,
            industry_cost: 5,
            effect: TechEffect::Newspaper,
        }
    }

    #[test]
    fn option_a_placement_rejects_more_than_three_areas_without_newspaper() {
        let state = stage4_placement_fixture(6);
        let rules = Rules::empty_for_tests();
        for n in [4u8, 6u8] {
            let placements: Vec<AreaId> = (0..n).map(AreaId::new).collect();
            let result = apply_move_unchecked(
                &state, &Move::PlaceThreeAreas { placements }, &rules,
            );
            assert!(
                matches!(result, Err(MoveError::RuleViolation(_))),
                "{n} distinct areas must exceed the 3-area cap, got {result:?}"
            );
        }
        assert_eq!(state.players[0].assistants.in_hand, 8, "input state untouched");
    }

    #[test]
    fn option_a_placement_allows_four_areas_with_newspaper_but_not_five() {
        let mut state = stage4_placement_fixture(6);
        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(newspaper_tech_fixture(1), "tech_newspaper");
        state.players[0].technologies.push(TechCardId(1));

        let four: Vec<AreaId> = (0..4).map(AreaId::new).collect();
        assert!(
            apply_move_unchecked(&state, &Move::PlaceThreeAreas { placements: four }, &rules).is_ok(),
            "4 areas are legal with Newspaper"
        );

        let five: Vec<AreaId> = (0..5).map(AreaId::new).collect();
        let result =
            apply_move_unchecked(&state, &Move::PlaceThreeAreas { placements: five }, &rules);
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
    }

    #[test]
    fn option_a_placement_rejects_duplicate_areas() {
        let state = stage4_placement_fixture(3);
        let rules = Rules::empty_for_tests();
        let a = AreaId::new(0);
        // 2 entries -- under the count cap, so distinctness is the ONLY
        // possible rejection reason (Option A is "1 in each of 1-3
        // DIFFERENT areas"; 2-in-one-area is Option B's encoding).
        let result = apply_move_unchecked(
            &state, &Move::PlaceThreeAreas { placements: vec![a, a] }, &rules,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        assert_eq!(state.players[0].assistants.in_hand, 8, "input state untouched");
    }

    #[test]
    fn option_a_placement_rejects_ghost_and_canal_areas() {
        let mut state = stage4_placement_fixture(2);
        let canal = AreaId::new(9);
        state.board_layout.areas.insert(canal, AreaConfig {
            area_type: AreaType::Canal,
            grid_position: (1, 0),
        });
        let rules = Rules::empty_for_tests();

        for bad in [AreaId::unchecked(200), canal] {
            let result = apply_move_unchecked(
                &state, &Move::PlaceThreeAreas { placements: vec![bad] }, &rules,
            );
            assert!(
                matches!(result, Err(MoveError::RuleViolation(_))),
                "placement into {bad:?} must be rejected, got {result:?}"
            );
        }
        assert_eq!(state.players[0].assistants.in_hand, 8, "input state untouched");
    }

    #[test]
    fn option_a_placement_260_entry_vector_is_rejected_not_underflowed() {
        // Census probe: `placements.len() as u8` TRUNCATED 260 to 4, which
        // passed the in-hand check (hand 8), then the mutating loop
        // decremented in_hand 260 times -- debug panic, release wrap
        // destroying assistant conservation. The count cap (usize
        // arithmetic) must reject this with a typed error instead.
        let state = stage4_placement_fixture(2);
        let rules = Rules::empty_for_tests();
        let placements: Vec<AreaId> = std::iter::repeat_n(AreaId::new(0), 260).collect();
        let result =
            apply_move_unchecked(&state, &Move::PlaceThreeAreas { placements }, &rules);
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        assert_eq!(state.players[0].assistants.in_hand, 8, "input state untouched");
        assert!(state.players[0].assistants.on_board.is_empty(), "input state untouched");
    }

    /// Two adjacent Bank areas 0,1 (bidirectional adjacency), Placement
    /// phase, player 0 with hand 8 / money 5.
    fn stage4_option_b_fixture() -> GameState {
        let mut state = stage4_placement_fixture(2);
        state.board_layout.adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        state.board_layout.adjacency.insert(AreaId::new(1), vec![AreaId::new(0)]);
        state
    }

    fn telegram_tech_fixture(id: u8) -> TechCard {
        TechCard {
            id: TechCardId(id),
            name: "Telegram".to_string(),
            country: Country::American,
            industry_cost: 4,
            effect: TechEffect::Telegram,
        }
    }

    #[test]
    fn place_two_same_area_telegram_bonus_requires_tech_ownership() {
        // Census probe: the bonus placement was honored with NO Telegram
        // tech owned (the generator's gate, bypassed at the raw arm).
        let state = stage4_option_b_fixture();
        let rules = Rules::empty_for_tests();
        let result = apply_move_unchecked(
            &state,
            &Move::PlaceTwoSameArea { area: AreaId::new(0), adjacent_bonus: Some(AreaId::new(1)) },
            &rules,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        assert_eq!(state.players[0].assistants.in_hand, 8, "input state untouched");
    }

    #[test]
    fn place_two_same_area_telegram_bonus_requires_physical_adjacency() {
        // Areas 0 and 3 exist but are NOT adjacent.
        let mut state = stage4_option_b_fixture();
        state.board_layout.areas.insert(AreaId::new(3), AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (1, 1),
        });
        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(telegram_tech_fixture(1), "tech_telegram");
        state.players[0].technologies.push(TechCardId(1));

        let result = apply_move_unchecked(
            &state,
            &Move::PlaceTwoSameArea { area: AreaId::new(0), adjacent_bonus: Some(AreaId::new(3)) },
            &rules,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
    }

    #[test]
    fn place_two_same_area_telegram_bonus_rejects_canal_crossing_pair() {
        // Crossing pairs are movement-transit edges ONLY (design 2026-07-18
        // decision log, Task 9): adjacency lists the pair, but Telegram's
        // bonus placement must exclude it -- same rule the generator applies.
        let mut state = stage4_option_b_fixture();
        state.board_layout.canal_crossings.push((AreaId::new(0), AreaId::new(1)));
        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(telegram_tech_fixture(1), "tech_telegram");
        state.players[0].technologies.push(TechCardId(1));

        let result = apply_move_unchecked(
            &state,
            &Move::PlaceTwoSameArea { area: AreaId::new(0), adjacent_bonus: Some(AreaId::new(1)) },
            &rules,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
    }

    #[test]
    fn place_two_same_area_rejects_ghost_and_canal_main_area() {
        let mut state = stage4_option_b_fixture();
        let canal = AreaId::new(9);
        state.board_layout.areas.insert(canal, AreaConfig {
            area_type: AreaType::Canal,
            grid_position: (1, 0),
        });
        let rules = Rules::empty_for_tests();

        for bad in [AreaId::unchecked(200), canal] {
            let result = apply_move_unchecked(
                &state,
                &Move::PlaceTwoSameArea { area: bad, adjacent_bonus: None },
                &rules,
            );
            assert!(
                matches!(result, Err(MoveError::RuleViolation(_))),
                "Option B into {bad:?} must be rejected, got {result:?}"
            );
        }
        assert_eq!(state.players[0].assistants.in_hand, 8, "input state untouched");
    }

    /// Chain 0 - 1 - 2 of Bank/CopperMine/SilkMill, president at 0, own
    /// assistants at 1 and 2 (same shape as `movement_toll_fixture`, minus
    /// the opponent president), Movement phase.
    fn stage4_movement_fixture() -> GameState {
        let mut state = create_minimal_state(2);
        for (id, ty) in [(0, AreaType::Bank), (1, AreaType::CopperMine), (2, AreaType::SilkMill)] {
            state.board_layout.areas.insert(
                AreaId::new(id),
                AreaConfig { area_type: ty, grid_position: (0, id) },
            );
        }
        state.board_layout.adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        state.board_layout.adjacency.insert(AreaId::new(1), vec![AreaId::new(0), AreaId::new(2)]);
        state.board_layout.adjacency.insert(AreaId::new(2), vec![AreaId::new(1)]);
        state.turn_phase = TurnPhase::Movement;
        state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);
        state.players[0].assistants.on_board.insert(AreaId::new(2), 1);
        state.players[0].money = 5;
        state
    }

    #[test]
    fn move_president_requires_president_on_board() {
        // Census probe: MovePresident applied with the president InHand
        // (PlacePresident is the from-hand encoding).
        let mut state = stage4_movement_fixture();
        state.players[0].president_location = PresidentLocation::InHand;
        let rules = Rules::empty_for_tests();

        for path in [vec![], vec![AreaId::new(1)]] {
            let result = apply_move_unchecked(
                &state,
                &Move::MovePresident { path, destination: AreaId::new(1) },
                &rules,
            );
            assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        }
        assert_eq!(state.players[0].president_location, PresidentLocation::InHand);
    }

    #[test]
    fn move_president_rejects_ghost_and_canal_destination() {
        // Ghost destination + Canal parking, via the empty-path (teleport)
        // encoding so destination validation is the ONLY gate in play.
        let mut state = stage4_movement_fixture();
        let canal = AreaId::new(9);
        state.board_layout.areas.insert(canal, AreaConfig {
            area_type: AreaType::Canal,
            grid_position: (1, 0),
        });
        let rules = Rules::empty_for_tests();

        for bad in [AreaId::unchecked(200), canal] {
            let result = apply_move_unchecked(
                &state,
                &Move::MovePresident { path: vec![], destination: bad },
                &rules,
            );
            assert!(
                matches!(result, Err(MoveError::RuleViolation(_))),
                "movement to {bad:?} must be rejected, got {result:?}"
            );
        }
        assert_eq!(
            state.players[0].president_location,
            PresidentLocation::OnBoard(AreaId::new(0)),
            "input state untouched"
        );
    }

    #[test]
    fn move_president_rejects_path_destination_disagreement() {
        // Census probe: fees were priced off `path` while the president
        // landed on `destination` -- the two must agree.
        let state = stage4_movement_fixture();
        let rules = Rules::empty_for_tests();
        let result = apply_move_unchecked(
            &state,
            &Move::MovePresident { path: vec![AreaId::new(1)], destination: AreaId::new(2) },
            &rules,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
    }

    #[test]
    fn move_president_rejects_non_adjacent_hop() {
        // Census probe: teleport-by-path -- a declared hop 0 -> 2 with no
        // board edge between them.
        let state = stage4_movement_fixture();
        let rules = Rules::empty_for_tests();
        let result = apply_move_unchecked(
            &state,
            &Move::MovePresident { path: vec![AreaId::new(2)], destination: AreaId::new(2) },
            &rules,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
    }

    #[test]
    fn move_president_rejects_repeated_areas_in_path() {
        // Census probe: duplicated fee hops -- a path that re-walks 1 - 2
        // repeatedly would multiply pass-through fees. Paths must be simple.
        let state = stage4_movement_fixture();
        let rules = Rules::empty_for_tests();
        let result = apply_move_unchecked(
            &state,
            &Move::MovePresident {
                path: vec![AreaId::new(1), AreaId::new(2), AreaId::new(1), AreaId::new(2)],
                destination: AreaId::new(2),
            },
            &rules,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
    }

    #[test]
    fn move_president_empty_path_teleport_stays_permitted() {
        // Empty path is the Station/Tram teleport ENCODING and must stay
        // applicable at the raw arm -- 0 and 2 are not adjacent, so only a
        // teleport explains this move. Whether the mover actually OWNS the
        // entitling tech is Tier C (checked-boundary scope), documented on
        // the guard.
        let state = stage4_movement_fixture();
        let rules = Rules::empty_for_tests();
        let result = apply_move_unchecked(
            &state,
            &Move::MovePresident { path: vec![], destination: AreaId::new(2) },
            &rules,
        );
        let next = result.expect("empty-path teleport encoding must stay permitted");
        assert_eq!(next.players[0].president_location, PresidentLocation::OnBoard(AreaId::new(2)));
    }

    #[test]
    fn place_president_requires_president_in_hand() {
        let state = stage4_movement_fixture(); // president OnBoard(0)
        let rules = Rules::empty_for_tests();
        let result = apply_move_unchecked(
            &state,
            &Move::PlacePresident { destination: AreaId::new(1) },
            &rules,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        assert_eq!(
            state.players[0].president_location,
            PresidentLocation::OnBoard(AreaId::new(0)),
            "input state untouched"
        );
    }

    #[test]
    fn place_president_rejects_ghost_and_canal_destination() {
        let mut state = stage4_movement_fixture();
        state.players[0].president_location = PresidentLocation::InHand;
        let canal = AreaId::new(9);
        state.board_layout.areas.insert(canal, AreaConfig {
            area_type: AreaType::Canal,
            grid_position: (1, 0),
        });
        let rules = Rules::empty_for_tests();

        for bad in [AreaId::unchecked(200), canal] {
            let result = apply_move_unchecked(
                &state,
                &Move::PlacePresident { destination: bad },
                &rules,
            );
            assert!(
                matches!(result, Err(MoveError::RuleViolation(_))),
                "placement to {bad:?} must be rejected, got {result:?}"
            );
        }
    }

    #[test]
    fn area_actions_require_matching_acting_area_type() {
        // Census probe: ProduceCopper fired at the Bank (and every other
        // mismatched pairing) -- the simple arms never resolved the acting
        // area at all. Each must now check it, mercenary-aware, exactly
        // like Port/Lab do.
        let rules = Rules::empty_for_tests();
        let cases: Vec<(AreaType, AreaAction)> = vec![
            (AreaType::Bank, AreaAction::ProduceCopper),
            (AreaType::CopperMine, AreaAction::ProduceSilk),
            (AreaType::SilkMill, AreaAction::ProduceTea),
            (AreaType::TeaPlantation, AreaAction::ProduceSeafood),
            (AreaType::CopperMine, AreaAction::Bank),
            (AreaType::Bank, AreaAction::EmploymentAgency { assistants: 1, shophouses: 0, trading_houses: 0 }),
            (AreaType::Bank, AreaAction::Chinatown { exchanges: vec![ChinatownExchange::YenToTea] }),
        ];
        for (area_type, action) in cases {
            let mut state = create_minimal_state(2);
            let area = AreaId::new(0);
            state.board_layout.areas.insert(area, AreaConfig { area_type, grid_position: (0, 0) });
            state.turn_phase = TurnPhase::AreaAction;
            state.current_action_power = Some(3);
            state.players[0].president_location = PresidentLocation::OnBoard(area);
            state.players[0].money = 5;
            state.players[0].assistants.in_warehouse = 2;

            let result = apply_move_unchecked(
                &state, &Move::ExecuteAreaAction { action: action.clone() }, &rules,
            );
            assert!(
                matches!(result, Err(MoveError::RuleViolation(_))),
                "{action:?} at {area_type:?} must be rejected, got {result:?}"
            );
        }
    }

    #[test]
    fn area_action_type_check_is_mercenary_aware() {
        // The acting area during a mercenary sub-turn is the TARGET area,
        // not wherever the president is: a Bank action at a Bank target
        // must pass even with the president parked at a CopperMine.
        let rules = Rules::empty_for_tests();
        let mut state = create_minimal_state(2);
        let bank = AreaId::new(0);
        let mine = AreaId::new(1);
        state.board_layout.areas.insert(bank, AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        state.board_layout.areas.insert(mine, AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(2);
        state.players[0].president_location = PresidentLocation::OnBoard(mine);
        state.players[0].assistants.on_board.insert(bank, 1);
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(bank);

        let result = apply_move_unchecked(
            &state, &Move::ExecuteAreaAction { action: AreaAction::Bank }, &rules,
        );
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn employment_agency_item_sum_cannot_overflow_u8() {
        // Census probe: `assistants + shophouses + trading_houses` summed in
        // u8 -- 200 + 100 panics in debug and wraps to 44 in release,
        // sneaking under the power cap. The sum must be widened.
        let rules = Rules::empty_for_tests();
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::EmploymentAgency,
            grid_position: (0, 0),
        });
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(5);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].money = 20;

        let result = apply_move_unchecked(
            &state,
            &Move::ExecuteAreaAction {
                action: AreaAction::EmploymentAgency { assistants: 200, shophouses: 100, trading_houses: 0 },
            },
            &rules,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
    }

    #[test]
    fn take_5_power_token_requires_power_five() {
        // Census probe: the token was granted at power 1 -- the printed
        // rule ("5 Power: take the token") never checked provenance.
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.turn_phase = TurnPhase::FivePowerBonus;
        state.current_action_power = Some(1);
        state.board_layout.five_power_tokens.insert(area, Some(FivePowerEffect::Money(3)));
        let rules = Rules::empty_for_tests();

        let result = apply_move_unchecked(&state, &Move::Take5PowerToken, &rules);
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        // Input untouched: token still on the board, nothing granted.
        assert_eq!(
            state.board_layout.five_power_tokens.get(&area),
            Some(&Some(FivePowerEffect::Money(3)))
        );
    }

    #[test]
    fn take_5_power_token_requires_a_token_present() {
        // At power 5 but with the token already claimed (None) or the area
        // never stocked, the raw arm used to succeed as a silent no-op --
        // Skip5PowerBonus is the canonical encoding for "no token to take".
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.turn_phase = TurnPhase::FivePowerBonus;
        state.current_action_power = Some(5);
        let rules = Rules::empty_for_tests();

        // Area never stocked with a token at all.
        let result = apply_move_unchecked(&state, &Move::Take5PowerToken, &rules);
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");

        // Token slot exists but was already claimed.
        state.board_layout.five_power_tokens.insert(area, None);
        let result = apply_move_unchecked(&state, &Move::Take5PowerToken, &rules);
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
    }

    #[test]
    fn build_trading_house_rejects_occupied_space() {
        // Census probe: the shophouse overwrite bug class, alive on the
        // trading-house arm -- building onto a site whose TH space is taken
        // silently REPLACED the other player's trading house.
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        let mut site = BuildingSite::new([1, 2, 3, 4], 7);
        site.trading_house_owner = Some(PlayerId::new(1));
        state.board_layout.building_sites.insert(area, site);
        state.turn_phase = TurnPhase::Building;
        state.current_action_power = Some(5);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].trading_houses.in_hand = 1;
        let rules = Rules::empty_for_tests();

        let result = apply_move_unchecked(&state, &Move::BuildTradingHouse, &rules);
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        // Zero mutation: the other player's trading house still stands.
        assert_eq!(
            state.board_layout.building_sites[&area].trading_house_owner,
            Some(PlayerId::new(1))
        );
        assert_eq!(state.players[0].trading_houses.in_hand, 1);
        assert!(state.players[0].trading_houses.built.is_empty());
    }

    #[test]
    fn fulfill_achievement_rejects_achievement_not_among_drawn_cards() {
        // Census probe: create_minimal_state draws Copper5/Money10/
        // Production4; claiming Silk6 (requirements MET -- silk 6, assistant
        // in hand) still succeeded, locking an assistant against a card
        // that is not in this game.
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        state.players[0].silk = 6;
        state.players[0].assistants.in_hand = 2;
        let rules = Rules::empty_for_tests();

        let result = apply_move_unchecked(
            &state,
            &Move::FulfillAchievement { achievement: AchievementType::Silk6 },
            &rules,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        assert_eq!(state.players[0].assistants.in_hand, 2, "no assistant locked");
        assert_eq!(state.players[0].assistants.on_achievements, 0);
    }

    #[test]
    fn apply_is_deterministic_for_legal_moves_on_a_real_midgame_state() {
        // Stage-4 PROPERTY: applying the same (state, move) twice yields
        // PartialEq-identical results -- apply_move_unchecked is a pure
        // function of its inputs (no hidden RNG, no interior mutability).
        // Driven over a REAL seeded game folded to several depths, checking
        // every legal move at each checkpoint.
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let pending = new_game(config, &rules, 424242).expect("setup");
        let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(1)];
        let mut state = finalize_setup(pending, &choices).expect("finalize");

        // Deterministic pseudo-random move picks (xorshift), no rand dep.
        let mut rng_state: u64 = 0x9E3779B97F4A7C15;
        let mut next = || {
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 7;
            rng_state ^= rng_state << 17;
            rng_state
        };

        let mut checked_moves = 0usize;
        for depth in 0..60 {
            if state.is_game_over() {
                break;
            }
            let legal = get_legal_moves(&state, &rules);
            assert!(!legal.is_empty(), "no legal moves at depth {depth}");

            if depth % 10 == 0 {
                for mv in &legal {
                    let a = apply_move_unchecked(&state, mv, &rules)
                        .expect("legal move must apply");
                    let b = apply_move_unchecked(&state, mv, &rules)
                        .expect("legal move must apply on the second try too");
                    assert_eq!(a, b, "apply must be deterministic for {mv:?} at depth {depth}");
                    checked_moves += 1;
                }
            }

            let pick = (next() as usize) % legal.len();
            state = apply_move_unchecked(&state, &legal[pick], &rules).expect("fold move");
        }
        assert!(checked_moves > 0, "property must have exercised at least one move");
    }

    #[test]
    fn mercenary_sub_turn_telephone_recovery_queues_and_returns_to_window() {
        // The mercenary sub-turn's acting area is the mercenary target, not
        // wherever the real President happens to be -- put the President in
        // hand (irrelevant to a mercenary sub-turn) so a fallback to
        // president_location would leave nothing to recover and fail below.
        let (mut state, rules) = telephone_building_fixture(2);
        let target = acting_area();
        state.players[0].president_location = PresidentLocation::InHand;
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(target);
        state.mercenary_return_phase = Some(TurnPhase::AdditionalActionFirst);
        state.current_action_power = Some(2);

        let mid = apply_move(&state, &Move::SkipBuilding, &rules).unwrap();
        assert_eq!(mid.turn_phase, TurnPhase::Recovery);
        assert_eq!(mid.pending_choices, vec![PendingChoice::RecoveryLeave]);

        let s = apply_move(&mid, &Move::ChooseRecoveryLeave { leave_one: true }, &rules).unwrap();
        assert_eq!(s.current_player().assistants.on_board[&target], 1);
        // Returns to the STORED mercenary window, not the normal AdditionalActionSecond.
        assert_eq!(s.turn_phase, TurnPhase::AdditionalActionFirst);
        assert!(!s.is_mercenary_action);
        assert_eq!(s.mercenary_target_area, None);
        assert_eq!(s.mercenary_return_phase, None);
        assert_eq!(s.current_action_power, None);
    }
}
