//! Plain-words rendering of moves, areas, phases, and achievements --
//! the CLI's option text (design docs/plans/2026-07-18-phase2-cli-driver-design.md).

use yokohama_core::*;

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
/// `Move::ChooseWarehouseTake` -- assistants free, shophouses 2 Yen, trading
/// houses at the printed take progression indexed by tokens already out of
/// the warehouse (mirrors `warehouse_take_options`'s own pricing).
fn warehouse_item_cost(p: &PlayerState, item: WarehouseItem) -> u32 {
    match item {
        WarehouseItem::Assistant => 0,
        WarehouseItem::Shophouse => 2,
        WarehouseItem::TradingHouse => {
            let already = p.trading_houses.in_hand as usize + p.trading_houses.total_built();
            TradingHouseState::take_cost(already)
        }
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

/// One plain-words line describing an `AreaAction` (called from
/// `describe_move`'s `ExecuteAreaAction` arm). `state` is accepted for
/// interface symmetry with `describe_move`'s call site but is currently
/// unused: no area action's description depends on board state.
fn describe_area_action(action: &AreaAction, _state: &GameState, rules: &Rules) -> String {
    match action {
        AreaAction::ProduceCopper => "Produce copper".to_string(),
        AreaAction::ProduceSilk => "Produce silk".to_string(),
        AreaAction::ProduceTea => "Produce tea".to_string(),
        AreaAction::ProduceSeafood => "Produce seafood".to_string(),
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
            let name = rules
                .techs
                .get(*card_id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| format!("T{:02}", card_id.0));
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
        Move::MovePresident { path, destination } => format!(
            "Move President to {} via {}",
            area_label(state, *destination),
            path.iter().map(|a| area_label(state, *a)).collect::<Vec<_>>().join(" -> ")
        ),
        Move::ReturnPresidentToHand { recover_assistants_from } if recover_assistants_from.is_empty() => {
            "Return President to hand (recover no assistants)".to_string()
        }
        Move::ReturnPresidentToHand { recover_assistants_from } => format!(
            "Return President to hand, recovering assistants from {}",
            recover_assistants_from.iter().map(|a| area_label(state, *a)).collect::<Vec<_>>().join(", ")
        ),
        Move::PlacePresident { destination } => {
            format!("Place President from hand in {}", area_label(state, *destination))
        }
        Move::ExecuteAreaAction { action } => describe_area_action(action, state, rules),
        Move::Take5PowerToken => "Take the 5-Power token".to_string(),
        Move::Skip5PowerBonus => "Skip the 5-Power token".to_string(),
        Move::BuildShophouse { slot } => format!("Build a Shophouse (space {})", slot + 1),
        Move::BuildTradingHouse => "Build a Trading House".to_string(),
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn describe_pending_choice_covers_all_six_variants() {
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
}
