//! Plain-words rendering of moves, areas, phases, and achievements --
//! the CLI's option text (design docs/plans/2026-07-18-phase2-cli-driver-design.md).

use yokohama_core::*;
use yokohama_core::game::actions::{copper_mine_reward, fishing_ground_reward, silk_mill_reward, tea_plantation_reward};

use crate::render;

pub fn area_type_name(t: AreaType) -> &'static str {
    match t {
        AreaType::CopperMine => "Copper Mine",
        AreaType::SilkMill => "Silk Mill",
        AreaType::TeaPlantation => "Tea Plantation",
        AreaType::FishingGround => "Fishing Ground",
        AreaType::Bank => "Bank",
        AreaType::PortA => "Port A",
        AreaType::PortB => "Port B",
        AreaType::LaboratoryA => "Laboratory A",
        AreaType::LaboratoryB => "Laboratory B",
        AreaType::EmploymentAgency => "Employment Agency",
        AreaType::Chinatown => "Chinatown",
        AreaType::Church => "Church",
        AreaType::Customs => "Customs",
        AreaType::Canal => "Canal",
    }
}

pub fn country_name(c: Country) -> &'static str {
    match c {
        Country::English => "Britain",
        Country::American => "America",
        Country::French => "France",
        Country::German => "Germany",
        Country::Dutch => "Netherlands",
    }
}

pub fn phase_name(p: TurnPhase) -> &'static str {
    match p {
        TurnPhase::AdditionalActionFirst => "Additional actions (before)",
        TurnPhase::Placement => "Placement",
        TurnPhase::Movement => "Movement",
        TurnPhase::AreaAction => "Area action",
        TurnPhase::FivePowerBonus => "5-Power bonus",
        TurnPhase::Building => "Building",
        TurnPhase::Recovery => "Recovery",
        TurnPhase::AdditionalActionSecond => "Additional actions (after)",
        TurnPhase::TurnEnd => "Turn end",
    }
}

pub fn area_label(state: &GameState, area: AreaId) -> String {
    let name = state
        .board_layout
        .areas
        .get(&area)
        .map(|a| area_type_name(a.area_type))
        .unwrap_or("Unknown area");
    format!("{name} (A{:02})", area.0)
}

/// Plain-words statement of an achievement's fulfillment condition.
pub fn achievement_desc(a: AchievementType) -> String {
    match a {
        AchievementType::Copper5 => "Own 5 Copper".to_string(),
        AchievementType::Silk6 => "Own 6 Silk".to_string(),
        AchievementType::Tea7 => "Own 7 Tea".to_string(),
        AchievementType::Seafood7 => "Own 7 Seafood".to_string(),
        AchievementType::Money10 => "Own 10 Yen".to_string(),
        AchievementType::Tokens5 => "Own 5 tokens (5-Power + mercenaries)".to_string(),
        AchievementType::Orders5 => "Complete 5 orders".to_string(),
        AchievementType::Technologies4 => "Own 4 technologies".to_string(),
        AchievementType::Production4 => "Buildings in 4 production areas".to_string(),
        AchievementType::Commercial4 => "Buildings in 4 commercial areas".to_string(),
        AchievementType::Prod3Comm2 => "Buildings in 3 production + 2 commercial areas".to_string(),
        AchievementType::Prod2Comm3 => "Buildings in 2 production + 3 commercial areas".to_string(),
    }
}

/// One-line plain-words statement of a Technology card's effect, for the
/// `labs` view (item 5). A TOTAL match over `TechEffect` -- no wildcard arm,
/// so adding a variant is a compile error here rather than a silently-missing
/// line. Each line is authored from the engine's own `TechEffect` variant
/// docs (`types/cards.rs`); the CLI guide's §8.2 lists card NAMES only, not
/// per-effect wording, so these are the single display source for what a tech
/// does. ASCII only, to stay terminal-safe.
pub fn tech_effect_text(effect: TechEffect) -> &'static str {
    match effect {
        TechEffect::Tram => "Teleport the President to any area without an opponent President",
        TechEffect::Stagecoach => "Move through opponent Presidents without paying the pass-through fee",
        TechEffect::PostalSystem => "May stop and act in an area holding an opponent President",
        TechEffect::Station => "Place the Station token: +1 Power for everyone there; teleport there while you have an assistant in its area",
        TechEffect::Gaslight => "+2 VP when you build a Shophouse",
        TechEffect::ElectricalLight => "+3 VP when you build a Trading House",
        TechEffect::MiningTechnology => "+1 Copper whenever you receive Copper",
        TechEffect::SpinningMill => "Once per turn: discard 1 Silk to gain 2 Yen",
        TechEffect::Ball => "+2 VP when you complete an Order",
        TechEffect::LanguageSchool => "+1 Yen when you complete an Order",
        TechEffect::PatentSystem => "+2 VP when you acquire a Technology",
        TechEffect::LetterPressPrinting => "+1 Yen when you acquire a Technology",
        TechEffect::Winery => "+3 VP when you place an assistant on the Church",
        TechEffect::Brickyard => "+4 VP when you place an assistant on Customs",
        TechEffect::SystemOfStock => "At turn start, if you have 1 Yen or less, refill to 2 Yen",
        TechEffect::University => "Once per turn: pay 1 Yen to take an assistant from the Warehouse",
        TechEffect::Newspaper => "Placement Option A may use 4 different areas instead of 3",
        TechEffect::Telegram => "After placing 2 assistants in one area, may place 1 in an adjacent area",
        TechEffect::Telephone => "During Recovery, may leave 1 assistant in the action area",
        TechEffect::Exposition => "Immediately gain 10 VP when acquired",
    }
}

/// Plain-words name of a trade good, for `Move::ChooseAnyGood`.
fn trade_good_name(g: TradeGood) -> &'static str {
    match g {
        TradeGood::Copper => "copper",
        TradeGood::Silk => "silk",
        TradeGood::Tea => "tea",
        TradeGood::Seafood => "fish",
    }
}

/// Plain-words article + name of a warehouse item, for
/// `Move::ChooseWarehouseTake` (e.g. "an assistant", "a shophouse").
fn warehouse_item_name(item: WarehouseItem) -> &'static str {
    match item {
        WarehouseItem::Assistant => "an assistant",
        WarehouseItem::Shophouse => "a shophouse",
        WarehouseItem::TradingHouse => "a trading house",
    }
}

/// Yen cost of taking `item` from the warehouse right now, for describing
/// `Move::ChooseWarehouseTake` -- assistants free, shophouses 2 Yen (both via
/// `WarehouseItem::flat_cost`, T2-m3 single source), trading houses at the
/// printed take progression indexed by tokens already out of the warehouse
/// (`TradingHouseState::already_taken`, T2-m2 single source; mirrors
/// `warehouse_take_options`'s own pricing).
fn warehouse_item_cost(p: &PlayerState, item: WarehouseItem) -> u32 {
    match item.flat_cost() {
        Some(cost) => cost,
        None => TradingHouseState::take_cost(p.trading_houses.already_taken()),
    }
}

/// Plain-words trade text for a single Chinatown exchange.
fn chinatown_exchange_text(exchange: ChinatownExchange) -> &'static str {
    match exchange {
        ChinatownExchange::CopperToYen => "1 copper -> 2 yen",
        ChinatownExchange::SilkToYen => "1 silk -> 2 yen",
        ChinatownExchange::TeaToYen => "1 tea -> 1 yen",
        ChinatownExchange::SeafoodToYen => "1 fish -> 1 yen",
        ChinatownExchange::YenToCopper => "2 yen -> 1 copper",
        ChinatownExchange::YenToSilk => "2 yen -> 1 silk",
        ChinatownExchange::YenToTea => "1 yen -> 1 tea",
        ChinatownExchange::YenToSeafood => "1 yen -> 1 fish",
        ChinatownExchange::YenToImported => "4 yen -> 1 imported good",
    }
}

/// The area this AreaAction targets: the mercenary sub-turn's target area
/// when `state.is_mercenary_action`, else the current player's President
/// location on board -- mirrors the engine's own acting-area resolution in
/// `game::legal_moves::generate_area_action_moves`/`generate_building_
/// moves` so a preview never disagrees with what the engine will actually
/// act on. `None` when the President is still in hand: unreached by any
/// legal `ExecuteAreaAction`/`BuildShophouse`/`BuildTradingHouse` move
/// (both phases require the President on board first) but kept as a
/// defensive fallback for direct state construction (fixtures/tests).
fn acting_area(state: &GameState) -> Option<AreaId> {
    if state.is_mercenary_action {
        state.mercenary_target_area
    } else {
        match state.current_player().president_location {
            PresidentLocation::OnBoard(a) => Some(a),
            PresidentLocation::InHand => None,
        }
    }
}

/// Production-area preview text: `"{verb} (power {p} -> +{r} {good})"`
/// where `r` is the REAL reward `reward_fn` computes for power `p`.
/// Production is NOT always power-for-power -- Copper Mine and Silk Mill
/// under-produce at low power (e.g. `copper_mine_reward(3) == 1`); only
/// Tea Plantation/Fishing Ground are a flat 1:1 (`game::actions::
/// production`) -- so this always calls the engine's own reward function
/// rather than echoing the power number, which would misstate the yield
/// for the two non-linear areas. Falls back to the bare verb when the
/// acting area can't be determined (see `acting_area`).
fn production_preview(state: &GameState, verb: &str, good: TradeGood, reward_fn: fn(u8) -> u8) -> String {
    match acting_area(state) {
        Some(area) => {
            // Recomputed rather than read from state.current_action_power,
            // but the two are always equal here -- including the mercenary
            // formula, which omits the President's own +1 exactly where the
            // target area is guaranteed not to hold it.
            let power = calculate_power(state, state.current_player, area);
            let reward = reward_fn(power);
            format!("{verb} (power {power} -> +{reward} {})", trade_good_name(good))
        }
        None => verb.to_string(),
    }
}

/// One plain-words line describing an `AreaAction` (called from
/// `describe_move`'s `ExecuteAreaAction` arm). `state` locates the acting
/// area (`acting_area`) so the production arms (Copper Mine/Silk Mill/Tea
/// Plantation/Fishing Ground) can preview the real yield at the current
/// power (Task 12, T2.2a) -- every other arm's text is still independent
/// of board state.
fn describe_area_action(action: &AreaAction, state: &GameState, rules: &Rules) -> String {
    match action {
        AreaAction::ProduceCopper => production_preview(state, "Produce copper", TradeGood::Copper, copper_mine_reward),
        AreaAction::ProduceSilk => production_preview(state, "Produce silk", TradeGood::Silk, silk_mill_reward),
        AreaAction::ProduceTea => production_preview(state, "Produce tea", TradeGood::Tea, tea_plantation_reward),
        AreaAction::ProduceSeafood => {
            production_preview(state, "Produce seafood", TradeGood::Seafood, fishing_ground_reward)
        }
        AreaAction::Bank => "Take money at the Bank".to_string(),
        AreaAction::Port { cards_taken, second_card_payment } => {
            let cards = cards_taken.iter().map(|id| format!("O{:02}", id.0)).collect::<Vec<_>>().join(", ");
            let mut s = format!("Port: take {cards}");
            match second_card_payment {
                Some(PortPayment::TwoYen) => s.push_str(", paying 2 yen"),
                Some(PortPayment::OneImportedGood) => s.push_str(", paying 1 imported good"),
                None => {}
            }
            s
        }
        AreaAction::Laboratory { card_id, extra_yen, extra_imported, fifth_tech_payment } => {
            let name = crate::input::tech_name(*card_id, &rules.techs);
            let mut s = format!("Laboratory: acquire {name}");
            if *extra_yen > 0 {
                s.push_str(&format!(", +{extra_yen} yen"));
            }
            if *extra_imported > 0 {
                s.push_str(&format!(", +{extra_imported} imported"));
            }
            match fifth_tech_payment {
                Some(true) => s.push_str(", 5th-tech fee: 2 yen"),
                Some(false) => s.push_str(", 5th-tech fee: 1 imported good"),
                None => {}
            }
            s
        }
        AreaAction::EmploymentAgency { assistants, shophouses, trading_houses } => {
            let mut parts = Vec::new();
            if *assistants > 0 {
                parts.push(format!("{assistants} assistant{}", if *assistants == 1 { "" } else { "s" }));
            }
            if *shophouses > 0 {
                parts.push(format!("{shophouses} shophouse{}", if *shophouses == 1 { "" } else { "s" }));
            }
            if *trading_houses > 0 {
                parts.push(format!(
                    "{trading_houses} trading house{}",
                    if *trading_houses == 1 { "" } else { "s" }
                ));
            }
            format!("Employment Agency: take {}", parts.join(", "))
        }
        AreaAction::Chinatown { exchanges } => {
            let parts = exchanges.iter().map(|e| chinatown_exchange_text(*e)).collect::<Vec<_>>().join(", ");
            format!("Chinatown: {parts}")
        }
        AreaAction::Church { copper, silk, tea, seafood, yen, imported_good } => {
            let mut parts = Vec::new();
            if *copper {
                parts.push("copper");
            }
            if *silk {
                parts.push("silk");
            }
            if *tea {
                parts.push("tea");
            }
            if *seafood {
                parts.push("fish");
            }
            if *yen {
                parts.push("yen");
            }
            if *imported_good {
                parts.push("imported good");
            }
            if parts.is_empty() {
                "Church: pay nothing (faith from power only)".to_string()
            } else {
                format!("Church: pay {}", parts.join(", "))
            }
        }
        AreaAction::Customs { imported_goods_paid } => {
            if *imported_goods_paid == 1 {
                "Customs: pay 1 imported good".to_string()
            } else {
                format!("Customs: pay {imported_goods_paid} imported goods")
            }
        }
        AreaAction::Skip => "Take the area action, declining all optional parts".to_string(),
    }
}

/// One plain-words line describing a `Move` -- the CLI's option text.
pub fn describe_move(mv: &Move, state: &GameState, rules: &Rules) -> String {
    match mv {
        Move::UseMercenary { country, target_area } => {
            format!("Use a {} mercenary at {}", country_name(*country), area_label(state, *target_area))
        }
        Move::FulfillAchievement { achievement } => {
            format!("Claim achievement: {}", achievement_desc(*achievement))
        }
        Move::FulfillOrder { order_id } => match rules.orders.get(*order_id) {
            Some(card) => format!("Fulfill {}", order_prompt_line(card)),
            None => format!("Fulfill order O{:02}", order_id.0),
        },
        Move::UseUniversity => "Use University: pay 1 yen, take an assistant from the warehouse".to_string(),
        Move::UseSpinningMill => "Use Textile Mill: discard 1 silk, gain 2 yen".to_string(),
        Move::EndAdditionalPhase => "End additional-action phase".to_string(),
        Move::PlaceThreeAreas { placements } if placements.is_empty() => "Place no assistants".to_string(),
        Move::PlaceThreeAreas { placements } => format!(
            "Place 1 assistant in each of: {}",
            placements.iter().map(|a| area_label(state, *a)).collect::<Vec<_>>().join(", ")
        ),
        Move::PlaceTwoSameArea { area, adjacent_bonus } => {
            let mut s = format!("Place 2 assistants in {}", area_label(state, *area));
            if let Some(adj) = adjacent_bonus {
                s.push_str(&format!(" and 1 in {} (Telegram)", area_label(state, *adj)));
            }
            s
        }
        Move::MovePresident { path, destination } if path.is_empty() => {
            format!("Move President directly to {} (free teleport)", area_label(state, *destination))
        }
        Move::MovePresident { path, destination } => {
            // `path` already carries the destination as its last element --
            // the same slice `apply_movement` prices (Task 6) -- so it feeds
            // `route_fee_vector` unchanged, never `path` + `destination`
            // appended (that would double-count the last hop).
            let vector = route_fee_vector(state, state.current_player, path, Some(&rules.techs), false);
            let cost_text = if vector.is_empty() {
                "free".to_string()
            } else {
                let total: u32 = vector.values().sum();
                // Player entries first (in PlayerId order), canal toll (the
                // `None` bank entry) last -- matches the design's example
                // wording, not `RouteVector`'s natural key order (`None`
                // sorts first in a BTreeMap since `None < Some(_)`).
                let parts: Vec<String> = vector
                    .iter()
                    .filter_map(|(recipient, amount)| {
                        recipient.map(|player_id| {
                            format!("{amount} to {}", render::color_name(state.player(player_id).color))
                        })
                    })
                    .chain(vector.get(&None).map(|amount| {
                        format!("{amount} canal toll{}", if *amount == 1 { "" } else { "s" })
                    }))
                    .collect();
                format!("pay {total} yen ({})", parts.join(", "))
            };
            format!("Move President to {} — {}", area_label(state, *destination), cost_text)
        }
        // Counts encoding (I5 ruling): per-area counts, not just which areas
        // -- two retreats differing only by count (e.g. recover 1 vs 2 from
        // the same area) must render distinguishably (Task 7's report;
        // Task 8 gives this its real treatment).
        Move::ReturnPresidentToHand { recover } => {
            if recover.is_empty() {
                "Return President to hand (recover no assistants)".to_string()
            } else {
                let total: u32 = recover.values().map(|&c| c as u32).sum();
                let parts = recover
                    .iter()
                    .map(|(a, c)| format!("A{:02} x{}", a.0, c))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Return President to hand, recovering {total} assistant(s) ({parts})")
            }
        }
        Move::PlacePresident { destination } => {
            format!("Place President from hand in {}", area_label(state, *destination))
        }
        Move::ExecuteAreaAction { action } => describe_area_action(action, state, rules),
        Move::Take5PowerToken => "Take the 5-Power token".to_string(),
        Move::Skip5PowerBonus => "Skip the 5-Power token".to_string(),
        Move::BuildShophouse { slot } => {
            // The space number is ALWAYS printed (Phase-2.9 per-slot
            // enumeration: one option per empty slot, and two blank slots
            // share the reward text "-", so the reward alone can't
            // distinguish them at the table).
            let reward = acting_area(state)
                .and_then(|a| state.board_layout.building_sites.get(&a))
                // and_then, not map: `slot_reward_text` yields None for an
                // out-of-range slot (a tampered record's can be), which falls
                // through to the bare "(space N)" wording below instead of
                // panicking. See C1, external review 2026-07-20.
                .and_then(|site| slot_reward_text(site, *slot as usize));
            // usize arithmetic: `slot + 1` on the raw u8 debug-overflows at
            // slot = 255 (tampered record) — the None arm must stay total.
            let space = (*slot as usize) + 1;
            match reward {
                Some(r) => format!("Build a Shophouse (space {space}: {r})"),
                None => format!("Build a Shophouse (space {space})"),
            }
        }
        Move::BuildTradingHouse => {
            let reward = acting_area(state).and_then(|a| state.board_layout.building_sites.get(&a));
            match reward {
                Some(site) => format!("Build a Trading House ({}vp)", site.trading_house_reward),
                None => "Build a Trading House".to_string(),
            }
        }
        Move::SkipBuilding => "Skip building".to_string(),
        Move::ChooseAnyGood { good } => format!("Take 1 {} (any-good reward)", trade_good_name(*good)),
        Move::ChooseWarehouseTake { item } => {
            let cost = warehouse_item_cost(state.current_player(), *item);
            let cost_text = if cost == 0 { "free".to_string() } else { format!("{cost} yen") };
            format!("Take {} from the warehouse ({cost_text})", warehouse_item_name(*item))
        }
        Move::DeclineWarehouseTake => "Decline the warehouse take".to_string(),
        Move::ChooseAssistantMove { from, to } => {
            let from_label = match from {
                MoveSource::Hand => "hand".to_string(),
                MoveSource::Area(a) => area_label(state, *a),
            };
            format!("Move an assistant from {from_label} to {} (reward move)", area_label(state, *to))
        }
        Move::DeclineAssistantMove => "Decline the assistant move".to_string(),
        Move::ChooseChurchSpace { space } => {
            format!("Place on Church faith-{space} space ({} VP)", ChurchTrack::vp_reward(*space))
        }
        Move::ChooseCustomsSpace { space } => {
            format!("Place on Customs space {space} ({} VP)", CustomsTrack::vp_reward(*space))
        }
        Move::ChooseRecoveryLeave { leave_one: false } => {
            "Recover all assistants from the action area".to_string()
        }
        Move::ChooseRecoveryLeave { leave_one: true } => {
            "Leave 1 assistant behind (Telephone)".to_string()
        }
        Move::ChooseStationPlace { area } => {
            format!("Place the Station token in {}", area_label(state, *area))
        }
    }
}

/// One plain-words line describing a `RecordedEvent` as it happened: a
/// `Move` delegates to `describe_move` against `state`, while each
/// `Reveal*` variant's wording depends only on the event's own fields, not
/// `state` -- `state`/`rules` are threaded through regardless so every
/// caller can call this ONE function uniformly regardless of variant,
/// passing the state as of when the event actually happened. Moved here
/// (T2.4) from the CLI's old `Repl::describe_event`, which always described
/// against whatever state was CURRENT when `log` was invoked -- harmless
/// for the Reveal arms (state-independent), but wrong for `Move` once
/// state-dependent wording (movement fees/recipients, a trading-house
/// take's printed price) went stale after the state it described moved on.
/// `Session` (`apply`/`apply_standalone`/`load`) now calls this once per
/// event, at the moment the event is folded, and caches the result.
pub fn describe_recorded_event(ev: &RecordedEvent, state: &GameState, rules: &Rules) -> String {
    match ev {
        RecordedEvent::Move(mv) => describe_move(mv, state, rules),
        RecordedEvent::RevealOrders(cards) => format!("reveal orders: {} card(s)", cards.len()),
        RecordedEvent::RevealTechs(cards) => format!("reveal techs: {} card(s)", cards.len()),
        RecordedEvent::RevealStartingOrder { player, card } => {
            format!("reveal starting order: P{} = O{:02}", player.0, card.0)
        }
        RecordedEvent::RevealFivePowerToken { area, effect } => {
            format!("reveal 5-power at A{:02}: {:?}", area.0, effect)
        }
    }
}

/// One plain-words summary line for the pending choice heading
/// `GameState::pending_choices` -- feeds the `Choose: {...} -- {n}
/// choice(s) pending` banner `Repl::prompt` renders in place of the normal
/// phase name whenever the queue is non-empty (Task 11, design §5.4).
/// Wildcard-free match: every current and future `PendingChoice` variant
/// must be named here explicitly. `rules` is accepted for interface
/// symmetry with `describe_move`'s call site (same precedent as
/// `describe_area_action`'s `_state`) but no variant currently needs it --
/// every choice type's summary is derived from `state` and the choice's own
/// fields alone.
pub fn describe_pending_choice(head: &PendingChoice, state: &GameState, _rules: &Rules) -> String {
    match head {
        PendingChoice::AnyGood => "any-good reward: choose 1 of copper/silk/tea/fish".to_string(),
        PendingChoice::WarehouseTake => {
            let p = state.current_player();
            let parts: Vec<String> = warehouse_take_options(p)
                .into_iter()
                .map(|item| {
                    let cost = warehouse_item_cost(p, item);
                    let label = match item {
                        WarehouseItem::Assistant => "assistant",
                        WarehouseItem::Shophouse => "shophouse",
                        WarehouseItem::TradingHouse => "TH",
                    };
                    if cost == 0 { format!("{label} free") } else { format!("{label} {cost}¥") }
                })
                .collect();
            format!("warehouse take ({})", parts.join(" / "))
        }
        PendingChoice::AssistantMove => "assistant move (reward): choose a source and destination, or decline".to_string(),
        PendingChoice::ChurchSpace { total_faith } => format!("Church space at faith {total_faith}"),
        PendingChoice::CustomsSpace { cost } => format!("Customs space at cost {cost}"),
        PendingChoice::RecoveryLeave => "Telephone: leave 1 assistant behind, or recover all".to_string(),
        PendingChoice::StationPlace => "place the Station token (all players gain +1 Power in its area)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn state_2p() -> GameState {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 1).unwrap();
        finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)]).unwrap()
    }

    #[test]
    fn area_label_names_type_and_id() {
        let state = state_2p();
        let some_area = *state.board_layout.areas.keys().next().unwrap();
        let label = area_label(&state, some_area);
        assert!(label.contains(&format!("(A{:02})", some_area.0)), "{label}");
    }

    #[test]
    fn placement_moves_read_as_plain_words() {
        let state = state_2p();
        let a3 = AreaId::new(3);
        let a5 = AreaId::new(5);
        let rules = Rules::standard();
        let none = describe_move(&Move::PlaceThreeAreas { placements: vec![] }, &state, &rules);
        assert_eq!(none, "Place no assistants");
        let two = describe_move(&Move::PlaceTwoSameArea { area: a3, adjacent_bonus: Some(a5) }, &state, &rules);
        assert!(two.starts_with("Place 2 assistants in "), "{two}");
        assert!(two.contains("Telegram"), "{two}");
    }

    #[test]
    fn area_actions_read_as_plain_words() {
        let state = state_2p();
        let rules = Rules::standard();
        assert_eq!(
            describe_move(&Move::ExecuteAreaAction { action: AreaAction::ProduceCopper }, &state, &rules),
            "Produce copper"
        );
        assert_eq!(
            describe_move(&Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid: 2 } }, &state, &rules),
            "Customs: pay 2 imported goods"
        );
        let skip = describe_move(&Move::ExecuteAreaAction { action: AreaAction::Skip }, &state, &rules);
        assert!(skip.contains("declin"), "{skip}");
        let ch = describe_move(
            &Move::ExecuteAreaAction {
                action: AreaAction::Chinatown { exchanges: vec![ChinatownExchange::CopperToYen] },
            },
            &state,
            &rules,
        );
        assert!(ch.contains("1 copper -> 2 yen"), "{ch}");
    }

    #[test]
    fn warehouse_take_moves_read_as_plain_words() {
        // Fresh game: no trading houses out yet, so the first take is 4 yen.
        let state = state_2p();
        let rules = Rules::standard();
        assert_eq!(
            describe_move(&Move::ChooseWarehouseTake { item: WarehouseItem::Assistant }, &state, &rules),
            "Take an assistant from the warehouse (free)"
        );
        assert_eq!(
            describe_move(&Move::ChooseWarehouseTake { item: WarehouseItem::Shophouse }, &state, &rules),
            "Take a shophouse from the warehouse (2 yen)"
        );
        assert_eq!(
            describe_move(&Move::ChooseWarehouseTake { item: WarehouseItem::TradingHouse }, &state, &rules),
            "Take a trading house from the warehouse (4 yen)"
        );
        assert_eq!(describe_move(&Move::DeclineWarehouseTake, &state, &rules), "Decline the warehouse take");
    }

    #[test]
    fn assistant_move_moves_read_as_plain_words() {
        let state = state_2p();
        let rules = Rules::standard();
        let some_area = *state.board_layout.areas.keys().next().unwrap();

        let from_hand = describe_move(&Move::ChooseAssistantMove { from: MoveSource::Hand, to: some_area }, &state, &rules);
        assert!(from_hand.starts_with("Move an assistant from hand to "), "{from_hand}");
        assert!(from_hand.ends_with("(reward move)"), "{from_hand}");

        let from_area = describe_move(&Move::ChooseAssistantMove { from: MoveSource::Area(some_area), to: some_area }, &state, &rules);
        assert!(from_area.contains(&format!("(A{:02})", some_area.0)), "{from_area}");
        assert!(!from_area.contains("hand"), "{from_area}");

        assert_eq!(describe_move(&Move::DeclineAssistantMove, &state, &rules), "Decline the assistant move");
    }

    #[test]
    fn church_space_choice_reads_as_plain_words_with_vp() {
        let state = state_2p();
        let rules = Rules::standard();
        assert_eq!(
            describe_move(&Move::ChooseChurchSpace { space: 6 }, &state, &rules),
            format!("Place on Church faith-6 space ({} VP)", ChurchTrack::vp_reward(6))
        );
    }

    #[test]
    fn customs_space_choice_reads_as_plain_words_with_vp() {
        let state = state_2p();
        let rules = Rules::standard();
        assert_eq!(
            describe_move(&Move::ChooseCustomsSpace { space: 3 }, &state, &rules),
            format!("Place on Customs space 3 ({} VP)", CustomsTrack::vp_reward(3))
        );
    }

    #[test]
    fn retreat_descriptions_show_counts_or_none() {
        let state = state_2p();
        let rules = Rules::standard();
        let none = describe_move(
            &Move::ReturnPresidentToHand { recover: std::collections::BTreeMap::new() },
            &state,
            &rules,
        );
        assert_eq!(none, "Return President to hand (recover no assistants)");

        let mut recover = std::collections::BTreeMap::new();
        recover.insert(AreaId::new(0), 2);
        recover.insert(AreaId::new(3), 1);
        let some = describe_move(&Move::ReturnPresidentToHand { recover }, &state, &rules);
        assert_eq!(some, "Return President to hand, recovering 3 assistant(s) (A00 x2, A03 x1)");
    }

    #[test]
    fn recovery_leave_choice_reads_as_plain_words() {
        let state = state_2p();
        let rules = Rules::standard();
        assert_eq!(
            describe_move(&Move::ChooseRecoveryLeave { leave_one: false }, &state, &rules),
            "Recover all assistants from the action area"
        );
        assert_eq!(
            describe_move(&Move::ChooseRecoveryLeave { leave_one: true }, &state, &rules),
            "Leave 1 assistant behind (Telephone)"
        );
    }

    #[test]
    fn station_place_choice_reads_as_plain_words() {
        let state = state_2p();
        let rules = Rules::standard();
        let some_area = *state.board_layout.areas.keys().next().unwrap();
        let text = describe_move(&Move::ChooseStationPlace { area: some_area }, &state, &rules);
        assert!(text.starts_with("Place the Station token in "), "{text}");
        assert!(text.contains(&format!("(A{:02})", some_area.0)), "{text}");
    }

    #[test]
    fn describe_pending_choice_covers_all_seven_variants() {
        let mut state = state_2p();
        let rules = Rules::standard();
        // Fresh game already has >=1 of each warehouse item type (12/6/4 in
        // the warehouse respectively); bump money so all three (including
        // the 4-yen first Trading House take) are affordable and listed,
        // regardless of which seat happens to be "current" (the start seat
        // begins with only 3 yen -- see `GameConfig::starting_money`).
        state.players[state.current_player.0 as usize].money = 10;

        let any_good = describe_pending_choice(&PendingChoice::AnyGood, &state, &rules);
        assert!(any_good.contains("any-good"), "{any_good}");

        let warehouse = describe_pending_choice(&PendingChoice::WarehouseTake, &state, &rules);
        assert!(warehouse.contains("assistant free"), "{warehouse}");
        assert!(warehouse.contains("shophouse 2¥"), "{warehouse}");
        assert!(warehouse.contains("TH 4¥"), "{warehouse}");

        let assistant_move = describe_pending_choice(&PendingChoice::AssistantMove, &state, &rules);
        assert!(assistant_move.contains("assistant move"), "{assistant_move}");

        let church = describe_pending_choice(&PendingChoice::ChurchSpace { total_faith: 6 }, &state, &rules);
        assert!(church.contains("Church") && church.contains('6'), "{church}");

        let customs = describe_pending_choice(&PendingChoice::CustomsSpace { cost: 2 }, &state, &rules);
        assert!(customs.contains("Customs") && customs.contains('2'), "{customs}");

        let recovery = describe_pending_choice(&PendingChoice::RecoveryLeave, &state, &rules);
        assert!(recovery.contains("Telephone"), "{recovery}");

        let station = describe_pending_choice(&PendingChoice::StationPlace, &state, &rules);
        assert!(station.contains("Station") && station.contains("Power"), "{station}");
    }

    #[test]
    fn university_and_spinning_mill_read_as_plain_words() {
        let state = state_2p();
        let rules = Rules::standard();
        assert_eq!(
            describe_move(&Move::UseUniversity, &state, &rules),
            "Use University: pay 1 yen, take an assistant from the warehouse"
        );
        assert_eq!(
            describe_move(&Move::UseSpinningMill, &state, &rules),
            "Use Textile Mill: discard 1 silk, gain 2 yen"
        );
    }

    /// Task 12 (T2.2a): a BuildShophouse option must show the space's
    /// PRINTED reward, not a bare "(space N)" placeholder -- the CLI must
    /// never require the player to cross-reference the physical site card
    /// themselves to know what a build actually pays out.
    #[test]
    fn build_shophouse_options_print_the_spaces_printed_reward() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let (area, site) = state
            .board_layout
            .building_sites
            .iter()
            .next()
            .map(|(&a, s)| (a, s.clone()))
            .expect("a building site exists");
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(area);

        let text = describe_move(&Move::BuildShophouse { slot: 0 }, &state, &rules);
        assert!(!text.ends_with("(space 1)"), "bare space-number text must be gone: {text}");
        let expected = slot_reward_text(&site, 0).expect("slot 0 is always in range");
        assert!(text.contains(&expected), "{text}");
    }

    /// Phase-2.9 per-slot enumeration fix: the Building phase now offers one
    /// `BuildShophouse` per empty slot, so each option must name WHICH space
    /// it builds on alongside that space's printed reward -- two blank
    /// spaces would otherwise render identical `(-)` options a tester at the
    /// table cannot tell apart without opening the `sites` view.
    #[test]
    fn tampered_out_of_range_shophouse_slot_describes_without_panicking() {
        // slot = 255 can only come from a tampered record; the description
        // must stay total (usize arithmetic, no u8 overflow) — Stage-1
        // review finding 5.
        let state = state_2p();
        let rules = Rules::standard();
        let text = describe_move(&Move::BuildShophouse { slot: 255 }, &state, &rules);
        assert!(text.contains("space 256"), "total fallback wording: {text}");
    }

    #[test]
    fn build_shophouse_options_identify_the_slot_and_its_reward() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let (area, site) = state
            .board_layout
            .building_sites
            .iter()
            .next()
            .map(|(&a, s)| (a, s.clone()))
            .expect("a building site exists");
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(area);

        let mut texts = Vec::new();
        for slot in 0u8..4 {
            let text = describe_move(&Move::BuildShophouse { slot }, &state, &rules);
            assert!(
                text.contains(&format!("space {}", slot + 1)),
                "option must name its space (1-based): {text}"
            );
            let reward = slot_reward_text(&site, slot as usize).expect("slot in range");
            assert!(text.contains(&reward), "option must keep the printed reward: {text}");
            texts.push(text);
        }
        for (i, a) in texts.iter().enumerate() {
            for b in texts.iter().skip(i + 1) {
                assert_ne!(a, b, "per-slot options must be pairwise distinguishable");
            }
        }
    }

    /// Task 12 riding item: BuildTradingHouse should likewise show the
    /// site's printed Trading-House VP rather than a bare label.
    #[test]
    fn build_trading_house_option_prints_the_sites_th_reward() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let (area, site) = state
            .board_layout
            .building_sites
            .iter()
            .next()
            .map(|(&a, s)| (a, s.clone()))
            .expect("a building site exists");
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(area);

        let text = describe_move(&Move::BuildTradingHouse, &state, &rules);
        assert!(text.contains(&format!("{}vp", site.trading_house_reward)), "{text}");
    }

    /// Task 12 (T2.2a): production-area action options must preview the
    /// yield at the CURRENT power, computed the same way the engine
    /// computes it (`calculate_power`) -- not just named.
    #[test]
    fn production_area_actions_preview_the_yield() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let area = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type == AreaType::TeaPlantation)
            .expect("a Tea Plantation area exists")
            .0;
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(area);
        state.players[cp.0 as usize].assistants.on_board.insert(area, 2);
        state.players[cp.0 as usize].assistants.in_hand -= 2;
        // President (1) + 2 assistants = power 3; Tea Plantation is a flat
        // 1:1 reward, so power 3 -> +3 tea.

        let text =
            describe_move(&Move::ExecuteAreaAction { action: AreaAction::ProduceTea }, &state, &rules);
        assert!(text.contains("power 3"), "{text}");
        assert!(text.contains("+3 tea"), "{text}");
    }

    /// Task 12 correctness guard: Copper Mine's reward table is NOT 1:1
    /// with power (power 3 -> only 1 copper, per `copper_mine_reward`) --
    /// the preview must show the REAL yield, not echo the power number, or
    /// it would actively mislead a player about what the build pays out.
    #[test]
    fn production_preview_uses_the_real_non_linear_reward_not_the_power_number() {
        let mut state = state_2p();
        let rules = Rules::standard();
        let area = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type == AreaType::CopperMine)
            .expect("a Copper Mine area exists")
            .0;
        let cp = state.current_player;
        state.players[cp.0 as usize].president_location = PresidentLocation::OnBoard(area);
        state.players[cp.0 as usize].assistants.on_board.insert(area, 2);
        state.players[cp.0 as usize].assistants.in_hand -= 2;
        // President (1) + 2 assistants = power 3; Copper Mine at power 3 -> 1 copper.

        let text =
            describe_move(&Move::ExecuteAreaAction { action: AreaAction::ProduceCopper }, &state, &rules);
        assert!(text.contains("power 3"), "{text}");
        assert!(text.contains("+1 copper"), "{text}");
        assert!(!text.contains("+3 copper"), "{text}");
    }

    /// Labs country + effect (pre-gate view batch item 5): `tech_effect_text`
    /// is a TOTAL match over `TechEffect` (no wildcard arm -- a future variant
    /// is a compile error). Every real tech card must therefore map to a
    /// non-empty effect line. Driven off the standard registry, which carries
    /// all 20 effects.
    #[test]
    fn every_tech_effect_has_nonempty_text() {
        let rules = Rules::standard();
        for card in rules.techs.iter() {
            assert!(!tech_effect_text(card.effect).is_empty(), "{:?} has empty effect text", card.effect);
        }
    }

    /// Spot-check a few effect lines against the engine's `TechEffect` docs so
    /// a regression in the mapping is caught, not just emptiness.
    #[test]
    fn tech_effect_text_spot_checks() {
        assert_eq!(tech_effect_text(TechEffect::Exposition), "Immediately gain 10 VP when acquired");
        assert!(tech_effect_text(TechEffect::Gaslight).contains("Shophouse"), "{}", tech_effect_text(TechEffect::Gaslight));
        assert!(tech_effect_text(TechEffect::Brickyard).contains("Customs"), "{}", tech_effect_text(TechEffect::Brickyard));
        assert!(tech_effect_text(TechEffect::University).contains("Warehouse"), "{}", tech_effect_text(TechEffect::University));
    }

    /// Task B review minor: the Station effect line must not promise an
    /// UNCONDITIONAL teleport -- the engine gates it on the owner holding an
    /// assistant in the Station's area (`can_teleport_to_station`,
    /// movement.rs:245-248: `in_area(station_area) > 0`). The one-liner must
    /// therefore carry that assistant-presence gate, not just say "teleport
    /// to it".
    #[test]
    fn station_effect_line_carries_the_teleport_gate() {
        let line = tech_effect_text(TechEffect::Station);
        assert!(line.contains("teleport"), "{line}");
        assert!(line.contains("assistant"), "teleport must read as gated on an assistant: {line}");
    }

    #[test]
    fn every_phase_has_a_plain_name() {
        for p in [
            TurnPhase::AdditionalActionFirst, TurnPhase::Placement, TurnPhase::Movement,
            TurnPhase::AreaAction, TurnPhase::FivePowerBonus, TurnPhase::Building,
            TurnPhase::Recovery, TurnPhase::AdditionalActionSecond, TurnPhase::TurnEnd,
        ] {
            assert!(!phase_name(p).is_empty());
        }
    }

    /// Task 13 (T2.2b): a small hand-built board for MovePresident cost
    /// tests. `state_2p()` can't reach a canal at all -- the standard
    /// 2-player board never places a Canal tile (board_gen.rs's
    /// `two_player_boards_unchanged` pins `canal_crossings.is_empty()` for
    /// every seeded 2p layout) -- so this mirrors movement.rs's Task 6
    /// fixture pattern (`canal_crossing_fixture` / `l2_diamond_fixture`)
    /// locally: those are `#[cfg(test)]`-private to yokohama-core and not
    /// reachable across the crate boundary. `route_fee_vector` only reads
    /// `canal_crossings` and `president_location`s (never `adjacency`), so
    /// the board is deliberately minimal: a `start` hub with three spokes --
    /// one tolled, one fee-bearing (Blue's President sits there), one free.
    fn movement_cost_fixture() -> (GameState, Rules) {
        let start = AreaId::new(0);
        let toll_dest = AreaId::new(1);
        let fee_dest = AreaId::new(2);
        let free_dest = AreaId::new(3);

        let mut areas = BTreeMap::new();
        areas.insert(start, AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(toll_dest, AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });
        areas.insert(fee_dest, AreaConfig { area_type: AreaType::SilkMill, grid_position: (0, 2) });
        areas.insert(free_dest, AreaConfig { area_type: AreaType::TeaPlantation, grid_position: (0, 3) });

        let mut adjacency = BTreeMap::new();
        adjacency.insert(start, vec![toll_dest, fee_dest, free_dest]);
        adjacency.insert(toll_dest, vec![start, fee_dest]);
        adjacency.insert(fee_dest, vec![start, toll_dest, free_dest]);
        adjacency.insert(free_dest, vec![start, fee_dest]);

        let board_layout = BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![(start, toll_dest)],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };

        let config = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));
        let mut players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];
        players[0].president_location = PresidentLocation::OnBoard(start);
        players[0].money = 10;
        players[1].president_location = PresidentLocation::OnBoard(fee_dest); // Blue blocks fee_dest
        players[1].money = 10;

        let state = GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::Movement,
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
        };

        (state, Rules::empty_for_tests())
    }

    /// Task 13 (T2.2b): a MovePresident option must show WHO gets paid, not
    /// just how much -- a player choosing between two equal-cost routes
    /// needs to see that one pays an opponent and the other pays the bank
    /// (canal toll) before picking. Also pins that a non-empty zero-cost
    /// path reads as "free", NOT the teleport wording (that stays reserved
    /// for the empty-path Station/Tram case).
    #[test]
    fn movement_descriptions_name_cost_and_recipients() {
        let (state, rules) = movement_cost_fixture();
        let toll_dest = AreaId::new(1);
        let fee_dest = AreaId::new(2);
        let free_dest = AreaId::new(3);

        // Canal fixture -- the toll route:
        let toll_move = Move::MovePresident { path: vec![toll_dest], destination: toll_dest };
        let text = describe_move(&toll_move, &state, &rules);
        assert!(text.contains("canal toll"), "{text}");

        // the fee route (Blue's President sits at fee_dest):
        let fee_move = Move::MovePresident { path: vec![fee_dest], destination: fee_dest };
        let text2 = describe_move(&fee_move, &state, &rules);
        assert!(text2.contains("1 to "), "{text2}");
        assert!(text2.contains("1 to Blue"), "{text2}");

        // a free move: no canal, no opponent -- and it must NOT read as a
        // teleport (non-empty path):
        let free_move = Move::MovePresident { path: vec![free_dest], destination: free_dest };
        let text3 = describe_move(&free_move, &state, &rules);
        assert!(text3.contains("free"), "{text3}");
        assert!(!text3.contains("teleport"), "{text3}");

        // both a toll AND a fee on the same route (path through toll_dest,
        // stopping where Blue's President sits) -- pins the exact wording
        // and the players-before-canal-toll part ordering.
        let combo_move = Move::MovePresident { path: vec![toll_dest, fee_dest], destination: fee_dest };
        let text4 = describe_move(&combo_move, &state, &rules);
        assert_eq!(
            text4,
            format!("Move President to {} — pay 2 yen (1 to Blue, 1 canal toll)", area_label(&state, fee_dest))
        );
    }
}
