//! §3.4 production / Bank / Employment Agency / Chinatown — lane D implements
//! per spec §3.4.
//!
//! Skeleton authored by lane A so sibling lanes compile against these EXACT
//! signatures; bodies are `todo!()` until lane D lands. `Skip` is NOT added
//! here — the lib.rs area-action facade appends the single canonical decline
//! (production areas and the Bank never get one, `allows_area_action_skip`).

use std::collections::BTreeSet;

use yokohama_core::{AreaAction, AreaType, ChinatownExchange, GameState, Move, TradingHouseState};

/// The nine Chinatown exchange rates in declaration order [RB p.13]. This
/// order is the fixed total order used for the §9-B multiset representative
/// (`exchanges` sorted by variant); Imported Goods have no sell variant — they
/// "can never be sold" [RB p.13 Note].
const CHINATOWN_VARIANTS: [ChinatownExchange; 9] = [
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

/// Index of an exchange in [`CHINATOWN_VARIANTS`] — the fixed total order for
/// the §9-B sorted representative. Private helper (common.rs lacks it; noted
/// in the lane report). Test-only: production code sorts by emitting the
/// index-keyed DFS keys directly.
#[cfg(test)]
fn variant_index(e: ChinatownExchange) -> usize {
    CHINATOWN_VARIANTS
        .iter()
        .position(|v| *v == e)
        .expect("every ChinatownExchange variant is listed")
}

/// §3.4 production areas / Bank: the single imperative "Gain …" move for the
/// area type (`ProduceCopper`/`ProduceSilk`/`ProduceTea`/`ProduceSeafood`/
/// `Bank`), or `None` for a non-production, non-Bank area type. No `Skip`.
pub(crate) fn production_or_bank_move(area_type: AreaType) -> Option<Move> {
    // Production areas and the Bank have an imperative "Gain ..." wording —
    // their single move is the only legal action, no Skip [RB p.10-11;
    // DL 2026-07-17 area-action-optionality row].
    let action = match area_type {
        AreaType::CopperMine => AreaAction::ProduceCopper,
        AreaType::SilkMill => AreaAction::ProduceSilk,
        AreaType::TeaPlantation => AreaAction::ProduceTea,
        AreaType::FishingGround => AreaAction::ProduceSeafood,
        AreaType::Bank => AreaAction::Bank,
        _ => return None,
    };
    Some(Move::ExecuteAreaAction { action })
}

/// §3.4 Employment Agency: every `EmploymentAgency { a, s, t }` with
/// `1 ≤ a+s+t ≤ power-1`, warehouse-available and affordable (TH take-cost
/// progression).
pub(crate) fn employment_moves(state: &GameState, power: u8) -> Vec<Move> {
    let p = state.current_player();
    // Power -> tokens takeable: 0/1/2/3/4 at power 1-5 [RB p.12].
    let max_total = power.min(5).saturating_sub(1);
    let mut out = Vec::new();
    for a in 0..=max_total.min(p.assistants.in_warehouse) {
        for s in 0..=(max_total - a).min(p.shophouses.in_warehouse) {
            for t in 0..=(max_total - a - s).min(p.trading_houses.in_warehouse) {
                // 0/0/0 is validator-ILLEGAL — Skip is the sole canonical
                // decline [DL 2026-07-17 optionality row].
                if a + s + t == 0 {
                    continue;
                }
                // Assistants free; Shophouses 2 Yen each; Trading Houses the
                // printed 4/5/6/7 progression indexed by tokens already out of
                // the warehouse, summed across a multi-token take [RB p.13;
                // DL 2026-07-17 staging row].
                let th_cost: u32 = (0..t as usize)
                    .map(|j| TradingHouseState::take_cost(p.trading_houses.already_taken() + j))
                    .sum();
                let cost = 2 * u32::from(s) + th_cost;
                if cost <= p.money {
                    out.push(Move::ExecuteAreaAction {
                        action: AreaAction::EmploymentAgency {
                            assistants: a,
                            shophouses: s,
                            trading_houses: t,
                        },
                    });
                }
            }
        }
    }
    out
}

/// §3.4 Chinatown: one representative `Chinatown` move per achievable exchange
/// multiset (spec §9-B), `1 ≤ len ≤ power`, sequential running-balance feasible.
pub(crate) fn chinatown_moves(state: &GameState, power: u8) -> Vec<Move> {
    let p = state.current_player();
    // Sequence length 1..=power [RB p.13 "The number of times you may exchange
    // depends on the Action Power"; CM power-cap line]. `[]` is
    // validator-ILLEGAL (Skip canonical) [DL 2026-07-17 optionality row].
    let max_len = usize::from(power.min(5));
    let start = ChinatownBalance {
        money: p.money,
        copper: p.copper,
        silk: p.silk,
        tea: p.tea,
        seafood: p.seafood,
    };
    // Achievable multisets (§9-B): a multiset is emitted iff AT LEAST ONE
    // ordering passes the sequential running-balance validation (§7.1-A3
    // sequential reading: proceeds of an earlier exchange may fund a later
    // one). The DFS walks feasible SEQUENCES and records each prefix's sorted
    // multiset key, so exactly the achievable multisets are collected.
    let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
    let mut prefix: Vec<u8> = Vec::new();
    chinatown_dfs(start, max_len, &mut prefix, &mut seen);
    seen.into_iter()
        .map(|key| Move::ExecuteAreaAction {
            action: AreaAction::Chinatown {
                // Representative sorted by variant — the key is already the
                // sorted index vector (§9-B fixed total order).
                exchanges: key.into_iter().map(|i| CHINATOWN_VARIANTS[usize::from(i)]).collect(),
            },
        })
        .collect()
}

/// Running balance for the sequential Chinatown validation. Imported Goods
/// are deliberately NOT tracked: they can never be sold [RB p.13 Note] and no
/// exchange spends them, so gaining one (YenToImported) cannot affect later
/// feasibility.
#[derive(Clone, Copy)]
struct ChinatownBalance {
    money: u32,
    copper: u32,
    silk: u32,
    tea: u32,
    seafood: u32,
}

/// Apply one exchange to the running balance; `None` when unaffordable at this
/// point in the sequence. Rates [RB p.13]: Copper/Silk sell for 2 Yen, Tea/
/// Seafood for 1; buys mirror those prices; 4 Yen buys 1 Imported Good.
fn chinatown_apply(b: ChinatownBalance, variant: u8) -> Option<ChinatownBalance> {
    let mut b = b;
    match CHINATOWN_VARIANTS[usize::from(variant)] {
        ChinatownExchange::CopperToYen => {
            b.copper = b.copper.checked_sub(1)?;
            b.money += 2;
        }
        ChinatownExchange::SilkToYen => {
            b.silk = b.silk.checked_sub(1)?;
            b.money += 2;
        }
        ChinatownExchange::TeaToYen => {
            b.tea = b.tea.checked_sub(1)?;
            b.money += 1;
        }
        ChinatownExchange::SeafoodToYen => {
            b.seafood = b.seafood.checked_sub(1)?;
            b.money += 1;
        }
        ChinatownExchange::YenToCopper => {
            b.money = b.money.checked_sub(2)?;
            b.copper += 1;
        }
        ChinatownExchange::YenToSilk => {
            b.money = b.money.checked_sub(2)?;
            b.silk += 1;
        }
        ChinatownExchange::YenToTea => {
            b.money = b.money.checked_sub(1)?;
            b.tea += 1;
        }
        ChinatownExchange::YenToSeafood => {
            b.money = b.money.checked_sub(1)?;
            b.seafood += 1;
        }
        ChinatownExchange::YenToImported => {
            b.money = b.money.checked_sub(4)?;
            // Imported Goods are gained but never spendable here (see
            // ChinatownBalance doc) — nothing else to record.
        }
    }
    Some(b)
}

/// DFS over feasible exchange sequences up to `remaining` further steps,
/// recording each visited prefix's sorted multiset key into `seen`.
///
/// Prune on an already-seen key: an exchange's balance delta is fixed per
/// variant, so a multiset determines the resulting balance regardless of
/// order — the first visit of a key explored the identical subtree.
fn chinatown_dfs(
    balance: ChinatownBalance,
    remaining: usize,
    prefix: &mut Vec<u8>,
    seen: &mut BTreeSet<Vec<u8>>,
) {
    if remaining == 0 {
        return;
    }
    for variant in 0..CHINATOWN_VARIANTS.len() as u8 {
        let Some(next) = chinatown_apply(balance, variant) else {
            continue;
        };
        prefix.push(variant);
        let mut key = prefix.clone();
        key.sort_unstable();
        if seen.insert(key) {
            chinatown_dfs(next, remaining - 1, prefix, seen);
        }
        prefix.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::two_player_state;
    use yokohama_core::{
        AreaAction, AreaId, ChinatownExchange, Move, PresidentLocation, Rules, TurnPhase,
    };

    fn ea(action: AreaAction) -> Move {
        Move::ExecuteAreaAction { action }
    }

    fn employment(a: u8, s: u8, t: u8) -> Move {
        ea(AreaAction::EmploymentAgency { assistants: a, shophouses: s, trading_houses: t })
    }

    fn chinatown(exchanges: Vec<ChinatownExchange>) -> Move {
        ea(AreaAction::Chinatown { exchanges })
    }

    // --- production / Bank (§3.4; F3 [RB p.7]) ---

    #[test]
    fn production_or_bank_maps_the_imperative_move() {
        // [RB p.10-11]: each production area / the Bank has exactly one "Gain"
        // move; every other area type has none from this helper.
        assert_eq!(
            production_or_bank_move(AreaType::CopperMine),
            Some(ea(AreaAction::ProduceCopper))
        );
        assert_eq!(production_or_bank_move(AreaType::SilkMill), Some(ea(AreaAction::ProduceSilk)));
        assert_eq!(
            production_or_bank_move(AreaType::TeaPlantation),
            Some(ea(AreaAction::ProduceTea))
        );
        assert_eq!(
            production_or_bank_move(AreaType::FishingGround),
            Some(ea(AreaAction::ProduceSeafood))
        );
        assert_eq!(production_or_bank_move(AreaType::Bank), Some(ea(AreaAction::Bank)));
        assert_eq!(production_or_bank_move(AreaType::Church), None);
        assert_eq!(production_or_bank_move(AreaType::PortA), None);
        assert_eq!(production_or_bank_move(AreaType::Canal), None);
    }

    #[test]
    fn f3_production_area_has_single_imperative_move_no_skip() {
        // F3 [RB p.7 power-tally example]: own president + 3 assistants +
        // 1 shophouse on Silk Mill => power 5; the production move is
        // ProduceSilk ALONE — no Skip at a production area (DL 2026-07-17
        // optionality row).
        let mut s = two_player_state();
        let silk = AreaId::new(1);
        s.turn_phase = TurnPhase::AreaAction;
        s.players[0].president_location = PresidentLocation::OnBoard(silk);
        s.players[0].assistants.on_board.insert(silk, 3);
        s.players[0].shophouses.built.insert(silk, 0);
        let om = crate::oracle_moves(&s, &Rules::empty_for_tests());
        assert_eq!(om.exact, vec![ea(AreaAction::ProduceSilk)]);
        assert!(om.movement.is_empty());
    }

    // --- Employment Agency (§3.4; §6.2 market row) ---

    #[test]
    fn employment_token_cap_is_power_minus_one() {
        // [RB p.12] power->tokens 0/1/2/3/4.
        let mut s = two_player_state();
        s.players[0].money = 100;
        let moves = employment_moves(&s, 3);
        assert!(moves.contains(&employment(2, 0, 0)));
        assert!(!moves.contains(&employment(3, 0, 0)));
        assert!(moves
            .iter()
            .all(|m| !matches!(m, Move::ExecuteAreaAction { action: AreaAction::EmploymentAgency { assistants, shophouses, trading_houses } } if assistants + shophouses + trading_houses > 2)));
        assert!(employment_moves(&s, 1).is_empty(), "power 1 takes 0 tokens");
    }

    #[test]
    fn employment_zero_zero_zero_never_emitted() {
        // 0/0/0 is validator-ILLEGAL — Skip is the sole canonical decline
        // (DL 2026-07-17 optionality row).
        let s = two_player_state();
        for power in 1..=5u8 {
            assert!(!employment_moves(&s, power).contains(&employment(0, 0, 0)));
        }
    }

    #[test]
    fn employment_shophouse_costs_two_yen() {
        // [RB p.13] Shophouses cost 2 Yen at the Employment Agency.
        let mut s = two_player_state();
        s.players[0].money = 1;
        assert!(!employment_moves(&s, 5).contains(&employment(0, 1, 0)));
        s.players[0].money = 2;
        assert!(employment_moves(&s, 5).contains(&employment(0, 1, 0)));
    }

    #[test]
    fn employment_trading_house_take_cost_summation() {
        // [RB p.13] 4/5/6/7 progression, summed across a multi-token take
        // (DL 2026-07-17 staging row).
        let mut s = two_player_state();
        s.players[0].money = 9; // 4 + 5 = 9 for the first two takes
        assert!(employment_moves(&s, 5).contains(&employment(0, 0, 2)));
        s.players[0].money = 8;
        assert!(!employment_moves(&s, 5).contains(&employment(0, 0, 2)));
    }

    #[test]
    fn employment_trading_house_progression_indexed_by_already_taken() {
        // With 1 token already out of the warehouse the NEXT take costs 5.
        let mut s = two_player_state();
        s.players[0].trading_houses.in_hand = 1;
        s.players[0].trading_houses.in_warehouse = 3;
        s.players[0].money = 4;
        assert!(!employment_moves(&s, 5).contains(&employment(0, 0, 1)));
        s.players[0].money = 5;
        assert!(employment_moves(&s, 5).contains(&employment(0, 0, 1)));
    }

    #[test]
    fn employment_respects_warehouse_availability() {
        let mut s = two_player_state();
        s.players[0].money = 100;
        s.players[0].trading_houses.in_warehouse = 0;
        s.players[0].assistants.in_warehouse = 1;
        let moves = employment_moves(&s, 5);
        assert!(moves
            .iter()
            .all(|m| !matches!(m, Move::ExecuteAreaAction { action: AreaAction::EmploymentAgency { trading_houses, .. } } if *trading_houses > 0)));
        assert!(moves.contains(&employment(1, 0, 0)));
        assert!(!moves.contains(&employment(2, 0, 0)));
    }

    // --- Chinatown (§3.4 as amended by §9-B; §6.2 market row) ---

    #[test]
    fn chinatown_sequential_feasibility_pin() {
        // §6.2 pin fixture for the §7.1-A3 sequential reading: 0 Yen +
        // 1 Copper, power 2 => sell copper (+2 Yen) THEN buy tea (-1 Yen) is
        // achievable; the multiset representative [CopperToYen, YenToTea] must
        // be emitted even though no ordering starting with YenToTea is feasible.
        let mut s = two_player_state();
        s.players[0].money = 0;
        s.players[0].copper = 1;
        s.players[0].silk = 0;
        s.players[0].tea = 0;
        s.players[0].seafood = 0;
        let moves = chinatown_moves(&s, 2);
        assert!(moves
            .contains(&chinatown(vec![ChinatownExchange::CopperToYen, ChinatownExchange::YenToTea])));
        assert!(moves.contains(&chinatown(vec![ChinatownExchange::CopperToYen])));
        // Alone, the buy is unaffordable (0 Yen).
        assert!(!moves.contains(&chinatown(vec![ChinatownExchange::YenToTea])));
    }

    #[test]
    fn chinatown_buy_then_sell_ordering_is_found() {
        // Achievable = at least ONE ordering feasible (§9-B): with 2 Yen and
        // no copper, {YenToCopper, CopperToYen} works only buy-first.
        let mut s = two_player_state();
        s.players[0].money = 2;
        s.players[0].copper = 0;
        s.players[0].silk = 0;
        s.players[0].tea = 0;
        s.players[0].seafood = 0;
        let moves = chinatown_moves(&s, 2);
        assert!(moves.contains(&chinatown(vec![
            ChinatownExchange::CopperToYen,
            ChinatownExchange::YenToCopper,
        ])));
    }

    #[test]
    fn chinatown_length_capped_by_power() {
        // [RB p.13] "The number of times you may exchange depends on the
        // Action Power"; cap is the power (CM Chinatown-cap line).
        let mut s = two_player_state();
        s.players[0].money = 20;
        let moves = chinatown_moves(&s, 2);
        assert!(!moves.is_empty());
        assert!(moves.iter().all(|m| matches!(
            m,
            Move::ExecuteAreaAction { action: AreaAction::Chinatown { exchanges } }
                if (1..=2).contains(&exchanges.len())
        )));
    }

    #[test]
    fn chinatown_imported_goods_never_fund_or_sell() {
        // [RB p.13 Note] Imported Goods can never be sold; with nothing else
        // to spend, no exchange is feasible.
        let mut s = two_player_state();
        s.players[0].money = 0;
        s.players[0].copper = 0;
        s.players[0].silk = 0;
        s.players[0].tea = 0;
        s.players[0].seafood = 0;
        s.players[0].imported_goods = 5;
        assert!(chinatown_moves(&s, 5).is_empty());
    }

    #[test]
    fn chinatown_representatives_are_sorted_and_unique() {
        // §9-B: ONE representative per achievable multiset, exchanges sorted
        // by variant (fixed total order).
        let mut s = two_player_state();
        s.players[0].money = 6;
        let moves = chinatown_moves(&s, 3);
        let mut seen = std::collections::BTreeSet::new();
        for m in &moves {
            let Move::ExecuteAreaAction { action: AreaAction::Chinatown { exchanges } } = m else {
                panic!("chinatown_moves emitted a non-Chinatown move: {m:?}");
            };
            let idxs: Vec<usize> = exchanges.iter().map(|e| variant_index(*e)).collect();
            assert!(idxs.windows(2).all(|w| w[0] <= w[1]), "unsorted representative: {m:?}");
            assert!(seen.insert(idxs), "duplicate multiset: {m:?}");
        }
    }
}
