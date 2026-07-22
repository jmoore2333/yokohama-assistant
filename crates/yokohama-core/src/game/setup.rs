//! Game setup configuration and initialization.

use crate::game::board_gen::generate_board_layout;
use crate::game::cards::{refill_lab, refill_port};
use crate::game::rules::Rules;
use crate::types::*;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Create initial player states for a game configuration.
pub fn create_players(config: &GameConfig) -> Vec<PlayerState> {
    (0..config.player_count)
        .map(|i| {
            let player_id = PlayerId::new(i);
            let is_start = player_id == config.start_player;
            let mut player = PlayerState::new(player_id, config.player_colors[i as usize], is_start);
            // Set starting money based on turn order
            player.money = config.starting_money(player_id);
            player
        })
        .collect()
}

/// Create port state based on game configuration.
pub fn create_port_state(config: &GameConfig) -> PortState {
    PortState::new(config.has_port_b)
}

/// Create lab state based on game configuration.
pub fn create_lab_state(config: &GameConfig) -> LabState {
    LabState::new(config.has_lab_b)
}

/// A player's decision about their two-card starting-order deal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartingOrderChoice {
    /// Keep the card at this index (0 or 1) from the deal as the public starting order;
    /// the other card is removed from play.
    Keep(u8),
    /// Keep the deal private: neither card is recorded in hand, and the player's starting
    /// order stays hidden information (both dealt cards are removed from play fiction).
    Unknown,
}

/// Output of `new_game`: board, decks, and players are set up and starting-order cards are
/// dealt, but each player's keep/unknown choice hasn't been applied yet. Call `finalize_setup`
/// with the choices to get a playable `GameState`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingSetup {
    pub state: GameState,
    /// Two-card starting-order deal per player, indexed by `PlayerId`.
    pub deals: Vec<[OrderCardId; 2]>,
}

/// Achievement types available to be drawn for category A (trade good holdings).
const ACHIEVEMENTS_A: [AchievementType; 4] = [
    AchievementType::Copper5,
    AchievementType::Silk6,
    AchievementType::Tea7,
    AchievementType::Seafood7,
];
/// Achievement types available to be drawn for category B (collection goals).
const ACHIEVEMENTS_B: [AchievementType; 4] = [
    AchievementType::Money10,
    AchievementType::Tokens5,
    AchievementType::Orders5,
    AchievementType::Technologies4,
];
/// Achievement types available to be drawn for category C (building presence).
const ACHIEVEMENTS_C: [AchievementType; 4] = [
    AchievementType::Production4,
    AchievementType::Commercial4,
    AchievementType::Prod3Comm2,
    AchievementType::Prod2Comm3,
];

/// Deterministically generate a new game from a seed: randomized board layout, shuffled order
/// and tech decks (with player-count removal and Port/Lab initial fills applied), one random
/// Achievement per category, and a two-card starting-order deal per player. The same seed
/// always produces the same `PendingSetup`.
pub fn new_game(mut config: GameConfig, rules: &Rules, seed: u64) -> Result<PendingSetup, &'static str> {
    config.setup_seed = seed;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let board_layout = generate_board_layout(
        &config,
        &mut rng,
        rules.building_sites.clone(),
        rules.five_power_tokens.clone(),
    )
    .map_err(|_| "board generation failed")?;

    // Order deck: all 36, shuffle, remove by player count, deal 2 per player.
    let mut order_deck: Vec<OrderCardId> = rules.orders.iter().map(|c| c.id).collect();
    order_deck.sort_by_key(|id| id.0); // determinism before shuffle
    order_deck.shuffle(&mut rng);
    let orders_removed: Vec<OrderCardId> = (0..config.order_cards_to_remove())
        .filter_map(|_| order_deck.pop())
        .collect();
    let deals: Vec<[OrderCardId; 2]> = (0..config.player_count)
        .map(|_| [order_deck.pop().unwrap(), order_deck.pop().unwrap()])
        .collect();

    // Tech deck: all 30, shuffle.
    let mut tech_deck: Vec<TechCardId> = rules.techs.iter().map(|c| c.id).collect();
    tech_deck.sort_by_key(|id| id.0); // determinism before shuffle
    tech_deck.shuffle(&mut rng);

    // One random Achievement per category.
    let achievements = [
        AchievementCard::new(*ACHIEVEMENTS_A.choose(&mut rng).unwrap()),
        AchievementCard::new(*ACHIEVEMENTS_B.choose(&mut rng).unwrap()),
        AchievementCard::new(*ACHIEVEMENTS_C.choose(&mut rng).unwrap()),
    ];

    let players = create_players(&config);

    let mut state = GameState {
        config: config.clone(),
        board_layout,
        players,
        current_player: config.start_player,
        turn_phase: TurnPhase::AdditionalActionFirst,
        turn_number: 1,
        additional_action_state: AdditionalActionState::new(),
        current_action_power: None,
        is_mercenary_action: false,
        mercenary_target_area: None,
        mercenary_return_phase: None,
        president_start_area: None,
        port_state: create_port_state(&config),
        lab_state: create_lab_state(&config),
        order_deck,
        tech_deck,
        orders_removed,
        church_track: ChurchTrack::new(),
        customs_track: CustomsTrack::new(),
        achievements,
        mercenary_supply: MercenarySupply::new(),
        station_location: None,
        end_game_triggered: false,
        trigger_reason: None,
        final_round: false,
        trigger_player: None,
        pending_choices: Vec::new(),
    };

    // Initial fills: Port A/B and Lab A/B each to their full 6 slots.
    // refill_port fills every empty slot in one call; refill_lab only fills
    // the single rightmost empty slot per call, so it needs one call per slot.
    refill_port(&mut state.port_state.port_a, &mut state.order_deck);
    if let Some(port_b) = state.port_state.port_b.as_mut() {
        refill_port(port_b, &mut state.order_deck);
    }
    for _ in 0..6 {
        refill_lab(&mut state.lab_state.lab_a, &mut state.tech_deck);
    }
    if let Some(lab_b) = state.lab_state.lab_b.as_mut() {
        for _ in 0..6 {
            refill_lab(lab_b, &mut state.tech_deck);
        }
    }

    // Finding D1: the 2-player dummy assistants are board furniture placed
    // once here (like the Port/Lab fills above). reconcile_setup re-types the
    // board but never touches the two tracks, and finalize_setup only resolves
    // starting-order choices, so this single placement carries through BOTH
    // the normal and the live-tracked setup paths unchanged.
    place_setup_dummies(&mut state);

    Ok(PendingSetup { state, deals })
}

/// Finding D1 (rulebook p.9 NOTE, Jeremy's edition -- not the bundled PDF; see guidingpath's 2026-07-21 DL rows): in a 2-player game a "dummy" assistant of
/// an unused colour is placed during setup on Church faith-7 and Customs
/// space-4. Modelled as a real slot occupant owned by the sentinel
/// [`PlayerId::DUMMY`] — the ONE representation from which every consumer
/// derives: it BLOCKS real placement on those spaces (occupancy, via
/// `eligible_church_spaces` / `eligible_customs_spaces_for_cost`), it is
/// EXCLUDED from the end-game trigger's real-pawn tally (`total_assistants`
/// skips it, so a 2P game still ends on the 4th REAL pawn — the printed
/// 5-total being 4 real + 1 dummy), and it COMPETES in the Church/Customs
/// majority bonuses (`game::scoring`). No-op for 3+ players, which have no
/// dummy. `arrival_order` is irrelevant (final scoring's count-tie is
/// positional, not by arrival) and is left 0 without advancing
/// `next_arrival_order`, so real placements are numbered exactly as before.
pub(crate) fn place_setup_dummies(state: &mut GameState) {
    if state.config.player_count != 2 {
        return;
    }
    let church_idx = ChurchTrack::faith_to_index(7).expect("faith 7 is on the track");
    state.church_track.slots[church_idx] = Some(ChurchSlot {
        player: PlayerId::DUMMY,
        arrival_order: 0,
    });
    let customs_idx = CustomsTrack::space_to_index(4).expect("space 4 is on the track");
    state.customs_track.slots[customs_idx] = Some(CustomsSlot {
        player: PlayerId::DUMMY,
        arrival_order: 0,
    });
}

/// Inverse of [`place_setup_dummies`]: remove the 2-player dummy from Church
/// faith-7 and Customs space-4 if (and only if) it is the sentinel occupant.
/// Used by tests that convert a 2P fixture up to 3/4 players, where no dummy
/// exists — it leaves any real-player pawn on those spaces untouched.
#[cfg(test)]
pub(crate) fn clear_setup_dummies(state: &mut GameState) {
    let church_idx = ChurchTrack::faith_to_index(7).expect("faith 7 is on the track");
    if state.church_track.slots[church_idx].map(|s| s.player) == Some(PlayerId::DUMMY) {
        state.church_track.slots[church_idx] = None;
    }
    let customs_idx = CustomsTrack::space_to_index(4).expect("space 4 is on the track");
    if state.customs_track.slots[customs_idx].map(|s| s.player) == Some(PlayerId::DUMMY) {
        state.customs_track.slots[customs_idx] = None;
    }
}

/// Resolve each player's starting-order choice against a `PendingSetup`, producing a playable
/// `GameState`. `choices` must have one entry per player, in `PlayerId` order.
pub fn finalize_setup(
    pending: PendingSetup,
    choices: &[StartingOrderChoice],
) -> Result<GameState, &'static str> {
    let PendingSetup { mut state, deals } = pending;
    if choices.len() != deals.len() {
        return Err("choices length must match player count");
    }

    for (i, choice) in choices.iter().enumerate() {
        let deal = deals[i];
        match choice {
            StartingOrderChoice::Keep(kept_index) => {
                let kept_index = *kept_index as usize;
                if kept_index > 1 {
                    return Err("Keep index must be 0 or 1");
                }
                let kept = deal[kept_index];
                let other = deal[1 - kept_index];
                state.players[i].order_cards_in_hand.push(kept);
                state.players[i].starting_order = Some(kept);
                state.orders_removed.push(other);
            }
            StartingOrderChoice::Unknown => {
                state.players[i].starting_order_hidden = true;
                state.orders_removed.push(deal[0]);
                state.orders_removed.push(deal[1]);
            }
        }
    }

    Ok(state)
}

/// Physical-table observations to reconcile into a seeded `PendingSetup`
/// before `finalize_setup` (design 2026-07-18 §5.2). All fields optional so
/// partial observation works; the live wizard fills everything observable.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupObservations {
    pub board: Option<ObservedBoard>,
    pub achievements: Option<[AchievementType; 3]>,
    pub port_a: Option<[OrderCardId; 6]>,
    pub port_b: Option<[OrderCardId; 6]>,
    pub lab_a: Option<[TechCardId; 6]>,
    pub lab_b: Option<[TechCardId; 6]>,
    /// Observed two-card deals — normally only the tracker's own seat.
    pub deals: Vec<(PlayerId, [OrderCardId; 2])>,
}

/// Observed physical board arrangement (handled in reconcile_setup's board
/// pass): area tile per grid cell and building-site card per grid cell,
/// row-major over `board_gen::row_lengths` (the staggered pyramid). `sites`
/// is None exactly where the tile carries no site (Canal).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedBoard {
    pub tiles: Vec<crate::types::AreaType>,
    pub sites: Vec<Option<u8>>,
}

/// Every place an order card can sit inside a PendingSetup.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OrderLoc {
    Deck(usize),
    Removed(usize),
    PortA(usize),
    PortB(usize),
    Deal { player: usize, slot: usize },
}

fn get_order(p: &PendingSetup, loc: OrderLoc) -> OrderCardId {
    match loc {
        OrderLoc::Deck(i) => p.state.order_deck[i],
        OrderLoc::Removed(i) => p.state.orders_removed[i],
        OrderLoc::PortA(i) => p.state.port_state.port_a.slots[i].expect("setup port slot occupied"),
        OrderLoc::PortB(i) => p.state.port_state.port_b.as_ref().expect("port B present").slots[i].expect("setup port slot occupied"),
        OrderLoc::Deal { player, slot } => p.deals[player][slot],
    }
}

fn set_order(p: &mut PendingSetup, loc: OrderLoc, card: OrderCardId) {
    match loc {
        OrderLoc::Deck(i) => p.state.order_deck[i] = card,
        OrderLoc::Removed(i) => p.state.orders_removed[i] = card,
        OrderLoc::PortA(i) => p.state.port_state.port_a.slots[i] = Some(card),
        OrderLoc::PortB(i) => p.state.port_state.port_b.as_mut().expect("port B present").slots[i] = Some(card),
        OrderLoc::Deal { player, slot } => p.deals[player][slot] = card,
    }
}

fn locate_order(p: &PendingSetup, card: OrderCardId) -> Option<OrderLoc> {
    if let Some(i) = p.state.order_deck.iter().position(|&c| c == card) {
        return Some(OrderLoc::Deck(i));
    }
    if let Some(i) = p.state.orders_removed.iter().position(|&c| c == card) {
        return Some(OrderLoc::Removed(i));
    }
    if let Some(i) = p.state.port_state.port_a.slots.iter().position(|&s| s == Some(card)) {
        return Some(OrderLoc::PortA(i));
    }
    if let Some(b) = &p.state.port_state.port_b {
        if let Some(i) = b.slots.iter().position(|&s| s == Some(card)) {
            return Some(OrderLoc::PortB(i));
        }
    }
    for (player, deal) in p.deals.iter().enumerate() {
        for (slot, c) in deal.iter().enumerate() {
            if *c == card {
                return Some(OrderLoc::Deal { player, slot });
            }
        }
    }
    None
}

/// Swap `card` into `dest` from wherever it currently sits. No-op if already there.
fn place_order(p: &mut PendingSetup, dest: OrderLoc, card: OrderCardId) -> Result<(), &'static str> {
    let src = locate_order(p, card).ok_or("observed order card not present anywhere in this setup")?;
    if src == dest {
        return Ok(());
    }
    let displaced = get_order(p, dest);
    set_order(p, dest, card);
    set_order(p, src, displaced);
    Ok(())
}

/// Every place a tech card can sit inside a PendingSetup.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TechLoc {
    Deck(usize),
    LabA(usize),
    LabB(usize),
}

fn get_tech(p: &PendingSetup, loc: TechLoc) -> TechCardId {
    match loc {
        TechLoc::Deck(i) => p.state.tech_deck[i],
        TechLoc::LabA(i) => p.state.lab_state.lab_a.slots[i].expect("setup lab slot occupied"),
        TechLoc::LabB(i) => p.state.lab_state.lab_b.as_ref().expect("lab B present").slots[i].expect("setup lab slot occupied"),
    }
}

fn set_tech(p: &mut PendingSetup, loc: TechLoc, card: TechCardId) {
    match loc {
        TechLoc::Deck(i) => p.state.tech_deck[i] = card,
        TechLoc::LabA(i) => p.state.lab_state.lab_a.slots[i] = Some(card),
        TechLoc::LabB(i) => p.state.lab_state.lab_b.as_mut().expect("lab B present").slots[i] = Some(card),
    }
}

fn locate_tech(p: &PendingSetup, card: TechCardId) -> Option<TechLoc> {
    if let Some(i) = p.state.tech_deck.iter().position(|&c| c == card) {
        return Some(TechLoc::Deck(i));
    }
    if let Some(i) = p.state.lab_state.lab_a.slots.iter().position(|&s| s == Some(card)) {
        return Some(TechLoc::LabA(i));
    }
    if let Some(b) = &p.state.lab_state.lab_b {
        if let Some(i) = b.slots.iter().position(|&s| s == Some(card)) {
            return Some(TechLoc::LabB(i));
        }
    }
    None
}

/// Swap `card` into `dest` from wherever it currently sits. No-op if already there.
fn place_tech(p: &mut PendingSetup, dest: TechLoc, card: TechCardId) -> Result<(), &'static str> {
    let src = locate_tech(p, card).ok_or("observed tech card not present anywhere in this setup")?;
    if src == dest {
        return Ok(());
    }
    let displaced = get_tech(p, dest);
    set_tech(p, dest, card);
    set_tech(p, src, displaced);
    Ok(())
}

/// Reconcile physical-table observations into a seeded PendingSetup by
/// swapping observed cards into their observed zones (conservation by
/// exchange), replacing achievements, and (board pass) re-typing the grid.
/// Pure and deterministic; called by `replay` when a record carries
/// observations, and by the live wizard.
pub fn reconcile_setup(
    mut pending: PendingSetup,
    obs: &SetupObservations,
    rules: &Rules,
) -> Result<PendingSetup, &'static str> {
    // --- validation ---
    if obs.port_b.is_some() && pending.state.port_state.port_b.is_none() {
        return Err("port B observed but not in play at this player count");
    }
    if obs.lab_b.is_some() && pending.state.lab_state.lab_b.is_none() {
        return Err("lab B observed but not in play at this player count");
    }
    let mut seen_orders = std::collections::BTreeSet::new();
    let observed_orders: Vec<OrderCardId> = obs
        .port_a.iter().flatten().copied()
        .chain(obs.port_b.iter().flatten().copied())
        .chain(obs.deals.iter().flat_map(|(_, d)| d.iter().copied()))
        .collect();
    for c in &observed_orders {
        if rules.orders.get(*c).is_none() {
            return Err("observed order card id not in the registry");
        }
        if !seen_orders.insert(c.0) {
            return Err("duplicate order card across observations");
        }
    }
    let mut seen_techs = std::collections::BTreeSet::new();
    let observed_techs: Vec<TechCardId> =
        obs.lab_a.iter().flatten().copied().chain(obs.lab_b.iter().flatten().copied()).collect();
    for c in &observed_techs {
        if rules.techs.get(*c).is_none() {
            return Err("observed tech card id not in the registry");
        }
        if !seen_techs.insert(c.0) {
            return Err("duplicate tech card across observations");
        }
    }
    let mut seen_players = std::collections::BTreeSet::new();
    for (pid, _) in &obs.deals {
        if pid.0 as usize >= pending.deals.len() {
            return Err("observed deal for a player not in this game");
        }
        if !seen_players.insert(pid.0) {
            return Err("duplicate player in observed deals");
        }
    }
    if let Some(board) = &obs.board {
        reconcile_board(&mut pending, board, rules)?; // Task 6; stub errors until then
    }
    if let Some(ach) = &obs.achievements {
        pending.state.achievements = [
            AchievementCard::new(ach[0]),
            AchievementCard::new(ach[1]),
            AchievementCard::new(ach[2]),
        ];
    }
    // --- swaps, fixed order ---
    if let Some(cards) = &obs.port_a {
        for (i, c) in cards.iter().enumerate() {
            place_order(&mut pending, OrderLoc::PortA(i), *c)?;
        }
    }
    if let Some(cards) = &obs.port_b {
        for (i, c) in cards.iter().enumerate() {
            place_order(&mut pending, OrderLoc::PortB(i), *c)?;
        }
    }
    for (pid, deal) in &obs.deals {
        for (slot, c) in deal.iter().enumerate() {
            place_order(&mut pending, OrderLoc::Deal { player: pid.0 as usize, slot }, *c)?;
        }
    }
    if let Some(cards) = &obs.lab_a {
        for (i, c) in cards.iter().enumerate() {
            place_tech(&mut pending, TechLoc::LabA(i), *c)?;
        }
    }
    if let Some(cards) = &obs.lab_b {
        for (i, c) in cards.iter().enumerate() {
            place_tech(&mut pending, TechLoc::LabB(i), *c)?;
        }
    }
    Ok(pending)
}

/// Board pass: re-type the fixed row-major staggered grid to the observed
/// tiles, reassign building-site cards from registry indices, rebuild the
/// 5-Power token pool over the new non-Canal cells (seeded values, order
/// preserved), and rebuild adjacency and Canal crossings for the new Canal
/// cell. Grid positions never move (AreaId i is always the same staggered
/// cell), but the any-to-any Canal-crossing adjacency depends on WHICH cell is
/// the Canal, so a re-type CAN move it. Adjacency is rebuilt wholesale from the
/// same shared helpers `generate_board_layout` uses (`staggered_adjacency` +
/// `derive_canal_crossings`), so reconciled output is identical to generating
/// this tile arrangement directly.
fn reconcile_board(
    pending: &mut PendingSetup,
    board: &ObservedBoard,
    rules: &Rules,
) -> Result<(), &'static str> {
    use crate::types::AreaType;
    let layout = &mut pending.state.board_layout;
    let n = layout.areas.len();
    if board.tiles.len() != n {
        return Err("observed tile count does not match this player count's grid");
    }
    if board.sites.len() != n {
        return Err("observed site list length must match the tile list");
    }
    // Multiset equality vs the seeded layout (which already encodes the
    // player-count removals) — also the physical-verification tripwire
    // (design 2026-07-18 §5.2).
    let mut want: Vec<AreaType> = layout.areas.values().map(|a| a.area_type).collect();
    let mut got: Vec<AreaType> = board.tiles.clone();
    want.sort_by_key(|t: &AreaType| format!("{t:?}"));
    got.sort_by_key(|t: &AreaType| format!("{t:?}"));
    if want != got {
        return Err("observed tiles are not a permutation of this player count's tile set");
    }
    let mut seen_sites = std::collections::BTreeSet::new();
    for (i, s) in board.sites.iter().enumerate() {
        match s {
            None if board.tiles[i] != AreaType::Canal => return Err("missing site on a non-Canal tile"),
            Some(_) if board.tiles[i] == AreaType::Canal => return Err("Canal cannot carry a building site"),
            Some(idx) => {
                if *idx as usize >= rules.building_sites.len() {
                    return Err("observed building-site index out of range");
                }
                if !seen_sites.insert(*idx) {
                    return Err("duplicate building-site card across cells");
                }
            }
            None => {}
        }
    }
    // Seeded token values, reassigned over the NEW non-Canal ids in order.
    let tokens: Vec<Option<crate::types::FivePowerEffect>> =
        layout.five_power_tokens.values().copied().collect();
    // Re-type areas (grid positions are fixed: AreaId i is cell (i/cols, i%cols)).
    for (i, t) in board.tiles.iter().enumerate() {
        layout.areas.get_mut(&AreaId::new(i as u8)).expect("grid area exists").area_type = *t;
    }
    // Reassign sites.
    layout.building_sites.clear();
    for (i, s) in board.sites.iter().enumerate() {
        if let Some(idx) = s {
            layout.building_sites.insert(AreaId::new(i as u8), rules.building_sites[*idx as usize].clone());
        }
    }
    // Reassign tokens over new non-Canal ids.
    layout.five_power_tokens.clear();
    let mut token_iter = tokens.into_iter();
    for (i, t) in board.tiles.iter().enumerate() {
        if *t != AreaType::Canal {
            layout.five_power_tokens.insert(
                AreaId::new(i as u8),
                token_iter.next().ok_or("token pool smaller than non-Canal cell count")?,
            );
        }
    }
    // Rebuild adjacency: staggered brick-offset base plus the new Canal cell's
    // derived any-to-any transit pairs (both layered exactly the way
    // generation does).
    let rows = crate::game::board_gen::row_lengths(pending.state.config.player_count);
    layout.adjacency.clear();
    for (a, b) in crate::game::board_gen::staggered_adjacency(&rows) {
        layout.adjacency.entry(a).or_default().push(b);
        layout.adjacency.entry(b).or_default().push(a);
    }
    layout.canal_crossings.clear();
    if let Some(ci) = board.tiles.iter().position(|t| *t == AreaType::Canal) {
        let pos = crate::game::board_gen::area_id_to_cell(AreaId::new(ci as u8), &rows);
        for (a, b) in crate::game::board_gen::derive_canal_crossings(pos, &rows) {
            layout.adjacency.entry(a).or_default().push(b);
            layout.adjacency.entry(b).or_default().push(a);
            layout.canal_crossings.push((a, b));
        }
    }
    Ok(())
}

/// Validate that a GameState is consistent with its GameConfig.
/// Returns Ok(()) if valid, or an error message describing the inconsistency.
pub fn validate_game_state_consistency(state: &GameState) -> Result<(), &'static str> {
    let config = &state.config;

    // Validate player count
    if state.players.len() != config.player_count as usize {
        return Err("Player count mismatch");
    }

    // Validate Port B presence
    let has_port_b = state.port_state.port_b.is_some();
    if has_port_b != config.has_port_b {
        return Err("Port B presence mismatch");
    }

    // Validate Lab B presence
    let has_lab_b = state.lab_state.lab_b.is_some();
    if has_lab_b != config.has_lab_b {
        return Err("Lab B presence mismatch");
    }

    // Validate board layout doesn't have removed areas
    let removed_types = config.removed_area_types();
    for area_config in state.board_layout.areas.values() {
        if removed_types.contains(&area_config.area_type) {
            return Err("Board contains area that should be removed for player count");
        }
    }

    Ok(())
}

/// Create a minimal test game state for the given player count.
/// This is primarily for testing purposes.
pub fn create_test_game_state(player_count: u8) -> GameState {
    let config = GameConfig::for_player_count(player_count);
    let players = create_players(&config);

    GameState {
        config: config.clone(),
        board_layout: BoardLayout {
            areas: BTreeMap::new(),
            adjacency: BTreeMap::new(),
            canal_crossings: vec![],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        },
        players,
        current_player: config.start_player,
        turn_phase: TurnPhase::AdditionalActionFirst,
        turn_number: 1,
        additional_action_state: AdditionalActionState::new(),
        current_action_power: None,
        is_mercenary_action: false,
        mercenary_target_area: None,
        mercenary_return_phase: None,
        president_start_area: None,
        port_state: create_port_state(&config),
        lab_state: create_lab_state(&config),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_2_player_config() {
        let config = GameConfig::for_player_count(2);

        assert_eq!(config.player_count, 2);
        assert!(!config.has_port_b);
        assert!(!config.has_lab_b);
        assert_eq!(config.track_trigger(), 4);
        assert_eq!(config.order_cards_to_remove(), 16);
        assert_eq!(config.inactive_area_count(), 5);

        // Starting money
        assert_eq!(config.starting_money(PlayerId::new(0)), 3); // Start player
        assert_eq!(config.starting_money(PlayerId::new(1)), 4); // Other player
    }

    #[test]
    fn test_3_player_config() {
        let config = GameConfig::for_player_count(3);

        assert_eq!(config.player_count, 3);
        assert!(config.has_port_b);
        assert!(!config.has_lab_b);
        assert_eq!(config.track_trigger(), 5);
        assert_eq!(config.order_cards_to_remove(), 5);
        assert_eq!(config.inactive_area_count(), 3);
    }

    #[test]
    fn test_4_player_config() {
        let config = GameConfig::for_player_count(4);

        assert_eq!(config.player_count, 4);
        assert!(config.has_port_b);
        assert!(config.has_lab_b);
        assert_eq!(config.track_trigger(), 6);
        assert_eq!(config.order_cards_to_remove(), 0);
        assert_eq!(config.inactive_area_count(), 0);
    }

    /// Finding D1 (rulebook p.9 NOTE, Jeremy's edition -- not the bundled PDF; see guidingpath's 2026-07-21 DL rows): 2-player setup places a "dummy"
    /// assistant of an unused colour on Church faith-7 (index 3) and Customs
    /// space-4 (index 3), owned by the sentinel `PlayerId::DUMMY`. Modelled as
    /// a real slot occupant so placement blocking and majority competition
    /// both fall out of the one representation.
    #[test]
    fn finalize_setup_places_the_2p_dummy_on_faith7_and_space4() {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 3).unwrap();
        let state = finalize_setup(
            pending,
            &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
        )
        .unwrap();

        assert_eq!(
            state.church_track.slots[ChurchTrack::faith_to_index(7).unwrap()].map(|s| s.player),
            Some(PlayerId::DUMMY),
            "2P Church faith-7 must be held by the dummy"
        );
        assert_eq!(
            state.customs_track.slots[CustomsTrack::space_to_index(4).unwrap()].map(|s| s.player),
            Some(PlayerId::DUMMY),
            "2P Customs space-4 must be held by the dummy"
        );
        // The dummy is furniture, not a real-player pawn: excluded from the
        // end-game trigger's real-pawn tally (a 2P game still ends on the 4th
        // REAL pawn -- the printed 5-total being 4 real + 1 dummy).
        assert_eq!(
            state.church_track.total_assistants(),
            0,
            "the dummy is excluded from the real-pawn trigger count"
        );
        assert_eq!(state.customs_track.total_assistants(), 0);
    }

    /// 3- and 4-player games have no dummy: faith-7 and space-4 are ordinary
    /// open spaces there.
    #[test]
    fn finalize_setup_places_no_dummy_above_two_players() {
        let rules = Rules::standard();
        for players in [3u8, 4] {
            let pending = new_game(GameConfig::for_player_count(players), &rules, 3).unwrap();
            let choices: Vec<StartingOrderChoice> =
                (0..players).map(|_| StartingOrderChoice::Keep(0)).collect();
            let state = finalize_setup(pending, &choices).unwrap();
            assert!(
                state.church_track.slots[ChurchTrack::faith_to_index(7).unwrap()].is_none(),
                "{players}P: Church faith-7 must be open (no dummy)"
            );
            assert!(
                state.customs_track.slots[CustomsTrack::space_to_index(4).unwrap()].is_none(),
                "{players}P: Customs space-4 must be open (no dummy)"
            );
        }
    }

    #[test]
    fn test_create_players() {
        let config = GameConfig::for_player_count(3);
        let players = create_players(&config);

        assert_eq!(players.len(), 3);

        // Start player (player 0) gets 3 Yen
        assert_eq!(players[0].money, 3);
        assert_eq!(players[0].color, PlayerColor::Red);

        // Other players get 4 Yen
        assert_eq!(players[1].money, 4);
        assert_eq!(players[1].color, PlayerColor::Blue);
        assert_eq!(players[2].money, 4);
        assert_eq!(players[2].color, PlayerColor::Yellow);
    }

    #[test]
    fn test_port_state_by_player_count() {
        // 2 players: no Port B
        let config_2p = GameConfig::for_player_count(2);
        let port_2p = create_port_state(&config_2p);
        assert!(port_2p.port_b.is_none());

        // 3 players: has Port B
        let config_3p = GameConfig::for_player_count(3);
        let port_3p = create_port_state(&config_3p);
        assert!(port_3p.port_b.is_some());

        // 4 players: has Port B
        let config_4p = GameConfig::for_player_count(4);
        let port_4p = create_port_state(&config_4p);
        assert!(port_4p.port_b.is_some());
    }

    #[test]
    fn test_lab_state_by_player_count() {
        // 2 players: no Lab B
        let config_2p = GameConfig::for_player_count(2);
        let lab_2p = create_lab_state(&config_2p);
        assert!(lab_2p.lab_b.is_none());

        // 3 players: no Lab B
        let config_3p = GameConfig::for_player_count(3);
        let lab_3p = create_lab_state(&config_3p);
        assert!(lab_3p.lab_b.is_none());

        // 4 players: has Lab B
        let config_4p = GameConfig::for_player_count(4);
        let lab_4p = create_lab_state(&config_4p);
        assert!(lab_4p.lab_b.is_some());
    }

    #[test]
    fn test_create_test_game_state() {
        // 2-player game
        let state_2p = create_test_game_state(2);
        assert_eq!(state_2p.players.len(), 2);
        assert!(state_2p.port_state.port_b.is_none());
        assert!(state_2p.lab_state.lab_b.is_none());
        assert!(state_2p.order_deck.is_empty());

        // 4-player game
        let state_4p = create_test_game_state(4);
        assert_eq!(state_4p.players.len(), 4);
        assert!(state_4p.port_state.port_b.is_some());
        assert!(state_4p.lab_state.lab_b.is_some());
        assert!(state_4p.order_deck.is_empty());
    }

    #[test]
    fn test_start_player_rotation() {
        // Test that non-zero start player gets correct money
        let config = GameConfig::new(
            3,
            vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow],
            PlayerId::new(1), // Player 1 starts
        );

        let players = create_players(&config);

        assert_eq!(players[0].money, 4); // Not start player
        assert_eq!(players[1].money, 3); // Start player
        assert_eq!(players[2].money, 4); // Not start player
    }

    #[test]
    fn test_validate_game_state_consistency() {
        // Valid states should pass
        let state_2p = create_test_game_state(2);
        assert!(validate_game_state_consistency(&state_2p).is_ok());

        let state_3p = create_test_game_state(3);
        assert!(validate_game_state_consistency(&state_3p).is_ok());

        let state_4p = create_test_game_state(4);
        assert!(validate_game_state_consistency(&state_4p).is_ok());
    }

    #[test]
    fn test_validate_detects_player_count_mismatch() {
        let mut state = create_test_game_state(2);
        // Add an extra player - should now be invalid
        state.players.push(PlayerState::new(PlayerId::new(2), PlayerColor::Yellow, false));

        let result = validate_game_state_consistency(&state);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Player count mismatch");
    }

    #[test]
    fn test_validate_detects_port_b_mismatch() {
        let mut state = create_test_game_state(2);
        // 2p shouldn't have Port B, but let's add it
        state.port_state.port_b = Some(PortBoard::new());

        let result = validate_game_state_consistency(&state);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Port B presence mismatch");
    }

    #[test]
    fn test_validate_detects_lab_b_mismatch() {
        let mut state = create_test_game_state(3);
        // 3p shouldn't have Lab B, but let's add it
        state.lab_state.lab_b = Some(LabBoard::new());

        let result = validate_game_state_consistency(&state);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Lab B presence mismatch");
    }

    #[test]
    fn test_validate_detects_invalid_area_for_player_count() {
        let mut state = create_test_game_state(2);
        // 2p shouldn't have LaboratoryB
        state.board_layout.areas.insert(
            AreaId::new(17), // Valid area ID (0-17 range)
            AreaConfig {
                area_type: AreaType::LaboratoryB,
                grid_position: (0, 0),
            },
        );

        let result = validate_game_state_consistency(&state);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Board contains area that should be removed for player count");
    }

    #[test]
    fn new_game_is_seed_deterministic_and_decks_are_correct() {
        let rules = Rules::standard();
        let a = new_game(GameConfig::for_player_count(4), &rules, 42).unwrap();
        let b = new_game(GameConfig::for_player_count(4), &rules, 42).unwrap();
        assert_eq!(a.state, b.state);
        assert_eq!(a.deals, b.deals);
        // 4p: 36 orders - 0 removed - 8 dealt (2x4) - 12 port fill (6 each, A+B) = 16 in deck
        assert_eq!(a.state.order_deck.len(), 16);
        // 30 techs - 12 lab fill (6 each, A+B) = 18
        assert_eq!(a.state.tech_deck.len(), 18);
        assert!(a.state.port_state.port_a.is_full());
        assert!(a.state.lab_state.lab_b.as_ref().unwrap().slots.iter().all(|s| s.is_some()));
    }

    #[test]
    fn finalize_setup_applies_keep_and_unknown_choices() {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 7).unwrap();
        let deals = pending.deals.clone();
        let state = finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown]).unwrap();
        // P0 kept deal[0]: it's in hand and recorded as starting order; deal[1] removed from play
        assert_eq!(state.players[0].order_cards_in_hand, vec![deals[0][0]]);
        assert_eq!(state.players[0].starting_order, Some(deals[0][0]));
        assert!(state.orders_removed.contains(&deals[0][1]));
        // P1 unknown: nothing in hand, flag set, both dealt cards out of the deck fiction
        assert!(state.players[1].order_cards_in_hand.is_empty());
        assert!(state.players[1].starting_order_hidden);
        assert!(state.orders_removed.contains(&deals[1][0]) && state.orders_removed.contains(&deals[1][1]));
    }

    fn order_multiset(p: &PendingSetup) -> std::collections::BTreeMap<u8, usize> {
        let mut m = std::collections::BTreeMap::new();
        let mut add = |id: OrderCardId| *m.entry(id.0).or_insert(0) += 1;
        p.state.order_deck.iter().copied().for_each(&mut add);
        p.state.orders_removed.iter().copied().for_each(&mut add);
        p.state.port_state.port_a.slots.iter().flatten().copied().for_each(&mut add);
        if let Some(b) = &p.state.port_state.port_b {
            b.slots.iter().flatten().copied().for_each(&mut add);
        }
        p.deals.iter().flat_map(|d| d.iter()).copied().for_each(&mut add);
        m
    }

    #[test]
    fn reconcile_setup_makes_observed_zones_match_and_conserves_cards() {
        let rules = Rules::standard();
        // Two different seeds play the role of "engine fiction" vs "physical table".
        let pending = new_game(GameConfig::for_player_count(2), &rules, 11).unwrap();
        let table = new_game(GameConfig::for_player_count(2), &rules, 99).unwrap();
        let obs = SetupObservations {
            port_a: Some(std::array::from_fn(|i| table.state.port_state.port_a.slots[i].unwrap())),
            lab_a: Some(std::array::from_fn(|i| table.state.lab_state.lab_a.slots[i].unwrap())),
            deals: vec![(PlayerId::new(0), table.deals[0])],
            ..Default::default()
        };
        let before_multiset = order_multiset(&pending);
        let reconciled = reconcile_setup(pending, &obs, &rules).unwrap();
        for i in 0..6 {
            assert_eq!(reconciled.state.port_state.port_a.slots[i], Some(obs.port_a.unwrap()[i]));
            assert_eq!(reconciled.state.lab_state.lab_a.slots[i], Some(obs.lab_a.unwrap()[i]));
        }
        assert_eq!(reconciled.deals[0], table.deals[0]);
        assert_eq!(order_multiset(&reconciled), before_multiset); // conservation
        // Determinism: same inputs, same output.
        let again = reconcile_setup(new_game(GameConfig::for_player_count(2), &rules, 11).unwrap(), &obs, &rules).unwrap();
        assert_eq!(again, reconciled);
    }

    /// Finding D1: the live-tracked setup path (new_game -> reconcile_setup ->
    /// finalize_setup) yields the SAME dummy occupancy as the normal path.
    /// reconcile_setup re-types the board and swaps cards but never touches the
    /// two tracks, so the dummy new_game placed carries through unchanged.
    #[test]
    fn reconcile_then_finalize_2p_still_carries_the_dummy() {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 11).unwrap();
        let table = new_game(GameConfig::for_player_count(2), &rules, 99).unwrap();
        let obs = SetupObservations {
            port_a: Some(std::array::from_fn(|i| table.state.port_state.port_a.slots[i].unwrap())),
            deals: vec![(PlayerId::new(0), table.deals[0])],
            ..Default::default()
        };
        let reconciled = reconcile_setup(pending, &obs, &rules).unwrap();
        let state = finalize_setup(
            reconciled,
            &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
        )
        .unwrap();
        assert_eq!(
            state.church_track.slots[ChurchTrack::faith_to_index(7).unwrap()].map(|s| s.player),
            Some(PlayerId::DUMMY),
            "the dummy must survive the reconcile path onto Church faith-7"
        );
        assert_eq!(
            state.customs_track.slots[CustomsTrack::space_to_index(4).unwrap()].map(|s| s.player),
            Some(PlayerId::DUMMY),
            "the dummy must survive the reconcile path onto Customs space-4"
        );
    }

    #[test]
    fn reconcile_setup_rejects_bad_observations() {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 11).unwrap();
        // Duplicate order id across observed zones.
        let dup = SetupObservations {
            port_a: Some([OrderCardId(1), OrderCardId(1), OrderCardId(2), OrderCardId(3), OrderCardId(4), OrderCardId(5)]),
            ..Default::default()
        };
        assert!(reconcile_setup(pending.clone(), &dup, &rules).is_err());
        // Port B observed in a 2p game (no Port B).
        let no_b = SetupObservations {
            port_b: Some([OrderCardId(1), OrderCardId(2), OrderCardId(3), OrderCardId(4), OrderCardId(5), OrderCardId(6)]),
            ..Default::default()
        };
        assert!(reconcile_setup(pending.clone(), &no_b, &rules).is_err());
        // Unknown order id (0 is not a printed corner index; real ids are 1-36).
        let bad_id = SetupObservations {
            deals: vec![(PlayerId::new(0), [OrderCardId(0), OrderCardId(2)])],
            ..Default::default()
        };
        assert!(reconcile_setup(pending.clone(), &bad_id, &rules).is_err());
        // Achievements replacement works.
        let ach = SetupObservations {
            achievements: Some([AchievementType::Silk6, AchievementType::Orders5, AchievementType::Commercial4]),
            ..Default::default()
        };
        let r = reconcile_setup(pending, &ach, &rules).unwrap();
        assert_eq!(r.state.achievements[0].achievement_type, AchievementType::Silk6);
        assert!(r.state.achievements[0].claimed_by.is_empty());
    }

    #[test]
    fn reconcile_board_retypes_grid_sites_and_canal() {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(3), &rules, 5).unwrap();
        // Observed board: the seeded tiles REVERSED (a valid permutation), sites 0..13 in order.
        let rows = crate::game::board_gen::row_lengths(3);
        let n = crate::game::board_gen::expected_tile_count(3);
        let mut tiles: Vec<AreaType> = (0..n)
            .map(|i| pending.state.board_layout.areas[&AreaId::new(i as u8)].area_type)
            .collect();
        tiles.reverse();
        let sites: Vec<Option<u8>> = tiles
            .iter()
            .scan(0u8, |next, t| {
                Some(if *t == AreaType::Canal { None } else { let s = *next; *next += 1; Some(s) })
            })
            .collect();
        let obs = SetupObservations {
            board: Some(ObservedBoard { tiles: tiles.clone(), sites: sites.clone() }),
            ..Default::default()
        };
        let r = reconcile_setup(pending, &obs, &rules).unwrap();
        let layout = &r.state.board_layout;
        // Types permuted as observed; grid positions unchanged (row-major ids).
        for (i, tile) in tiles.iter().enumerate().take(n) {
            let id = AreaId::new(i as u8);
            assert_eq!(layout.areas[&id].area_type, *tile);
            assert_eq!(
                layout.areas[&id].grid_position,
                crate::game::board_gen::area_id_to_cell(id, &rows)
            );
        }
        // Sites follow the observed assignment; Canal has none.
        // (No `clone_reset` helper on BuildingSite — compare the printed
        // fields directly; built-state fields are fresh at setup so
        // asserting they equal the `BuildingSite::new` defaults suffices.)
        for (i, site) in sites.iter().enumerate().take(n) {
            let id = AreaId::new(i as u8);
            match *site {
                Some(s) => {
                    let expected = &rules.building_sites[s as usize];
                    let got = &layout.building_sites[&id];
                    assert_eq!(got.spaces, expected.spaces);
                    assert_eq!(got.trading_house_reward, expected.trading_house_reward);
                    assert_eq!(got.shophouses_built, [None; 4]);
                    assert_eq!(got.trading_house_owner, None);
                }
                None => assert!(!layout.building_sites.contains_key(&id)),
            }
        }
        // Tokens: same count as before, all on non-Canal areas, all unclaimed.
        assert_eq!(layout.five_power_tokens.len(), n - 1);
        assert!(layout.five_power_tokens.values().all(|t| t.is_some()));
        assert!(!layout.five_power_tokens.contains_key(
            &AreaId::new(tiles.iter().position(|t| *t == AreaType::Canal).unwrap() as u8)
        ));
        // Canal crossings follow the any-to-any transit model (decision log
        // 2026-07-21) for the Canal's NEW cell, matching derive_canal_crossings
        // exactly (reconcile reuses the same helper generation does) — and each
        // pair is bidirectionally adjacent and marked as a crossing.
        let canal_i = tiles.iter().position(|t| *t == AreaType::Canal).unwrap();
        let canal_pos = crate::game::board_gen::area_id_to_cell(AreaId::new(canal_i as u8), &rows);
        let expected = crate::game::board_gen::derive_canal_crossings(canal_pos, &rows);
        assert_eq!(layout.canal_crossings, expected);
        for &(a, b) in &layout.canal_crossings {
            assert!(layout.adjacent_areas(a).contains(&b));
            assert!(layout.adjacent_areas(b).contains(&a));
            assert!(layout.is_canal_crossing(a, b));
        }
    }

    #[test]
    fn reconcile_board_rejects_multiset_and_site_violations() {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 5).unwrap();
        let n = pending.state.board_layout.areas.len();
        // Wrong multiset: all Bank tiles.
        let obs = SetupObservations {
            board: Some(ObservedBoard { tiles: vec![AreaType::Bank; n], sites: vec![Some(0); n] }),
            ..Default::default()
        };
        assert!(reconcile_setup(pending.clone(), &obs, &rules).is_err());
        // Duplicate site index.
        let tiles: Vec<AreaType> = (0..n)
            .map(|i| pending.state.board_layout.areas[&AreaId::new(i as u8)].area_type)
            .collect();
        let obs = SetupObservations {
            board: Some(ObservedBoard { tiles, sites: vec![Some(0); n] }),
            ..Default::default()
        };
        assert!(reconcile_setup(pending, &obs, &rules).is_err());
    }

    #[test]
    fn reconcile_board_2p_no_canal_keeps_all_tokens_and_empty_crossings() {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 123).unwrap();
        // 2p never has a Canal tile (tiles_for_player_count strips it).
        let rows = crate::game::board_gen::row_lengths(2);
        let n = crate::game::board_gen::expected_tile_count(2);
        let mut tiles: Vec<AreaType> = (0..n)
            .map(|i| pending.state.board_layout.areas[&AreaId::new(i as u8)].area_type)
            .collect();
        assert!(!tiles.contains(&AreaType::Canal));
        tiles.reverse(); // a valid re-permutation (all 2p games share this tile multiset)
        // Every cell carries a distinct site (no Canal to skip).
        let sites: Vec<Option<u8>> = (0..n as u8).map(Some).collect();
        let obs = SetupObservations {
            board: Some(ObservedBoard { tiles: tiles.clone(), sites: sites.clone() }),
            ..Default::default()
        };
        let r = reconcile_setup(pending, &obs, &rules).unwrap();
        let layout = &r.state.board_layout;
        // (c) Types permuted as observed; grid positions unchanged (row-major
        // staggered pyramid, 4/3/2/1).
        for (i, tile) in tiles.iter().enumerate().take(n) {
            let id = AreaId::new(i as u8);
            assert_eq!(layout.areas[&id].area_type, *tile);
            assert_eq!(
                layout.areas[&id].grid_position,
                crate::game::board_gen::area_id_to_cell(id, &rows)
            );
        }
        // (d) Sites follow the observed assignment (every cell, since there's no Canal).
        for (i, site) in sites.iter().enumerate().take(n) {
            let id = AreaId::new(i as u8);
            let s = site.expect("no Canal in a 2p board, so every cell has a site");
            let expected = &rules.building_sites[s as usize];
            let got = &layout.building_sites[&id];
            assert_eq!(got.spaces, expected.spaces);
            assert_eq!(got.trading_house_reward, expected.trading_house_reward);
            assert_eq!(got.shophouses_built, [None; 4]);
            assert_eq!(got.trading_house_owner, None);
        }
        // (b) Full token reindex: all 10 cells get a token, all unclaimed.
        assert_eq!(layout.five_power_tokens.len(), 10);
        assert_eq!(n, 10);
        assert!(layout.five_power_tokens.values().all(|t| t.is_some()));
        // (a) No Canal cell exists, so the `if let Some(ci)` branch in
        // reconcile_board is never taken — crossings stay empty.
        assert!(layout.canal_crossings.is_empty());
    }
}
