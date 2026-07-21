//! Commercial area actions (Bank, Employment Agency, Chinatown).

use crate::types::{ChinatownExchange, GameState, PlayerId};
use crate::game::cards::TechRegistry;
use crate::game::tech_effects::copper_bonus;

/// Bank reward: 1 Yen per power.
pub fn bank_reward(power: u8) -> u8 {
    power.min(5)
}

/// Employment Agency: tokens allowed based on power.
pub fn employment_agency_tokens_allowed(power: u8) -> u8 {
    match power {
        0..=1 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        _ => 4, // 5+
    }
}

/// A single good involved in a Chinatown exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChinatownGood {
    Copper,
    Silk,
    Tea,
    Seafood,
    Money,
    Imported,
}

/// The printed rate of one Chinatown exchange: spend `spend_amount` of
/// `spend`, gain `gain_amount` of `gain` (Chinatown board art). This is the
/// SINGLE source of truth for the nine exchange rates -- both the apply path
/// ([`apply_chinatown_exchange`]) and the legal-move enumeration BFS
/// (`legal_moves::try_apply_chinatown_exchange_simulated`) consume it, so the
/// numbers can never drift between the two (Phase-2.9 review addendum, Tier
/// 1: the apply-side copy was untested and had duplicated the rates).
///
/// The Mining Technology bonus on the copper-gaining exchange is NOT baked
/// in here -- callers add it on top of `gain_amount` for the exchange whose
/// `gain` is [`ChinatownGood::Copper`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChinatownRate {
    pub spend: ChinatownGood,
    pub spend_amount: u32,
    pub gain: ChinatownGood,
    pub gain_amount: u32,
}

/// The nine Chinatown exchange rates (single source of truth -- see
/// [`ChinatownRate`]).
pub fn chinatown_rate(exchange: ChinatownExchange) -> ChinatownRate {
    use ChinatownExchange as X;
    use ChinatownGood::*;
    let (spend, spend_amount, gain, gain_amount) = match exchange {
        X::CopperToYen => (Copper, 1, Money, 2),
        X::SilkToYen => (Silk, 1, Money, 2),
        X::TeaToYen => (Tea, 1, Money, 1),
        X::SeafoodToYen => (Seafood, 1, Money, 1),
        X::YenToCopper => (Money, 2, Copper, 1),
        X::YenToSilk => (Money, 2, Silk, 1),
        X::YenToTea => (Money, 1, Tea, 1),
        X::YenToSeafood => (Money, 1, Seafood, 1),
        X::YenToImported => (Money, 4, Imported, 1),
    };
    ChinatownRate { spend, spend_amount, gain, gain_amount }
}

/// Apply a Chinatown exchange.
pub fn apply_chinatown_exchange(
    state: &mut GameState,
    player: PlayerId,
    exchange: ChinatownExchange,
    tech_registry: &TechRegistry,
) -> Result<(), &'static str> {
    // Mining Technology bonus (KB "applies to": Chinatown) fires on the
    // copper-gaining exchange, which always receives exactly 1 base Copper.
    // Computed before the player is mutably borrowed below.
    let rate = chinatown_rate(exchange);
    let mining_bonus = if rate.gain == ChinatownGood::Copper {
        copper_bonus(state, player, 1, tech_registry) as u32
    } else {
        0
    };

    let p = &mut state.players[player.0 as usize];

    // Spend leg: must hold at least `spend_amount` of `spend`.
    let have = good_amount(p, rate.spend);
    if have < rate.spend_amount {
        return Err(not_enough(rate.spend));
    }
    add_good(p, rate.spend, -(rate.spend_amount as i64));
    // Gain leg: base rate plus the Mining bonus (nonzero only when gaining copper).
    add_good(p, rate.gain, (rate.gain_amount + mining_bonus) as i64);

    Ok(())
}

/// Read a player's stock of `good`.
fn good_amount(p: &crate::types::PlayerState, good: ChinatownGood) -> u32 {
    match good {
        ChinatownGood::Copper => p.copper,
        ChinatownGood::Silk => p.silk,
        ChinatownGood::Tea => p.tea,
        ChinatownGood::Seafood => p.seafood,
        ChinatownGood::Money => p.money,
        ChinatownGood::Imported => p.imported_goods,
    }
}

/// Apply a signed `delta` to a player's stock of `good`.
fn add_good(p: &mut crate::types::PlayerState, good: ChinatownGood, delta: i64) {
    let field = match good {
        ChinatownGood::Copper => &mut p.copper,
        ChinatownGood::Silk => &mut p.silk,
        ChinatownGood::Tea => &mut p.tea,
        ChinatownGood::Seafood => &mut p.seafood,
        ChinatownGood::Money => &mut p.money,
        ChinatownGood::Imported => &mut p.imported_goods,
    };
    *field = (*field as i64 + delta) as u32;
}

/// The historical "Not enough X" message for a failed spend leg.
fn not_enough(good: ChinatownGood) -> &'static str {
    match good {
        ChinatownGood::Copper => "Not enough copper",
        ChinatownGood::Silk => "Not enough silk",
        ChinatownGood::Tea => "Not enough tea",
        ChinatownGood::Seafood => "Not enough seafood",
        ChinatownGood::Money => "Not enough money",
        ChinatownGood::Imported => "Not enough imported goods",
    }
}

/// Validate Chinatown exchanges are within power limit.
pub fn validate_chinatown_exchanges(
    exchanges: &[ChinatownExchange],
    power: u8,
) -> Result<(), &'static str> {
    if exchanges.len() > power as usize {
        return Err("Too many exchanges for power level");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bank_reward() {
        assert_eq!(bank_reward(1), 1);
        assert_eq!(bank_reward(3), 3);
        assert_eq!(bank_reward(5), 5);
        assert_eq!(bank_reward(6), 5); // Capped
    }

    #[test]
    fn test_employment_agency_tokens() {
        assert_eq!(employment_agency_tokens_allowed(1), 0);
        assert_eq!(employment_agency_tokens_allowed(2), 1);
        assert_eq!(employment_agency_tokens_allowed(5), 4);
    }

    /// Direct coverage of `apply_chinatown_exchange`'s nine rates against
    /// LITERAL expected deltas (Phase-2.9 review addendum, Tier 1: the
    /// apply-side rates were untested -- mutating `+= 2` to `+= 3` survived
    /// the whole suite). No Mining Technology owned, so YenToCopper gains
    /// exactly its base 1.
    #[test]
    fn apply_chinatown_exchange_nine_rates_literal_deltas() {
        use crate::game::create_test_game_state;
        let techs = TechRegistry::new();
        let p0 = PlayerId::new(0);

        // (exchange, [copper, silk, tea, seafood, money, imported] AFTER),
        // starting from copper=silk=tea=seafood=money=5, imported=0.
        let cases: &[(ChinatownExchange, [u32; 6])] = &[
            (ChinatownExchange::CopperToYen, [4, 5, 5, 5, 7, 0]), // -1 copper, +2 money
            (ChinatownExchange::SilkToYen, [5, 4, 5, 5, 7, 0]),   // -1 silk,   +2 money
            (ChinatownExchange::TeaToYen, [5, 5, 4, 5, 6, 0]),    // -1 tea,    +1 money
            (ChinatownExchange::SeafoodToYen, [5, 5, 5, 4, 6, 0]), // -1 seafood, +1 money
            (ChinatownExchange::YenToCopper, [6, 5, 5, 5, 3, 0]), // -2 money,  +1 copper
            (ChinatownExchange::YenToSilk, [5, 6, 5, 5, 3, 0]),   // -2 money,  +1 silk
            (ChinatownExchange::YenToTea, [5, 5, 6, 5, 4, 0]),    // -1 money,  +1 tea
            (ChinatownExchange::YenToSeafood, [5, 5, 5, 6, 4, 0]), // -1 money, +1 seafood
            (ChinatownExchange::YenToImported, [5, 5, 5, 5, 1, 1]), // -4 money, +1 imported
        ];

        for (exchange, expected) in cases {
            let mut state = create_test_game_state(2);
            {
                let p = &mut state.players[0];
                p.copper = 5;
                p.silk = 5;
                p.tea = 5;
                p.seafood = 5;
                p.money = 5;
                p.imported_goods = 0;
            }
            apply_chinatown_exchange(&mut state, p0, *exchange, &techs)
                .expect("exchange affordable from the 5/5/5/5/5 base");
            let p = &state.players[0];
            let got = [p.copper, p.silk, p.tea, p.seafood, p.money, p.imported_goods];
            assert_eq!(
                got, *expected,
                "{exchange:?}: [copper,silk,tea,seafood,money,imported] deltas"
            );
        }
    }

    /// Each spend leg refuses when the player cannot afford it, with the
    /// historical per-good message.
    #[test]
    fn apply_chinatown_exchange_rejects_when_unaffordable() {
        use crate::game::create_test_game_state;
        let techs = TechRegistry::new();
        let p0 = PlayerId::new(0);
        let mut state = create_test_game_state(2);
        {
            let p = &mut state.players[0];
            p.copper = 0;
            p.money = 0;
        }
        assert_eq!(
            apply_chinatown_exchange(&mut state, p0, ChinatownExchange::CopperToYen, &techs),
            Err("Not enough copper")
        );
        assert_eq!(
            apply_chinatown_exchange(&mut state, p0, ChinatownExchange::YenToImported, &techs),
            Err("Not enough money")
        );
    }

    #[test]
    fn test_chinatown_exchange_validation() {
        let exchanges = vec![
            ChinatownExchange::CopperToYen,
            ChinatownExchange::SilkToYen,
            ChinatownExchange::TeaToYen,
        ];
        assert!(validate_chinatown_exchanges(&exchanges, 3).is_ok());
        assert!(validate_chinatown_exchanges(&exchanges, 2).is_err());
    }

    #[test]
    fn test_power_zero_returns_zero() {
        assert_eq!(bank_reward(0), 0);
        assert_eq!(employment_agency_tokens_allowed(0), 0);
    }

    #[test]
    fn test_employment_agency_full_range() {
        assert_eq!(employment_agency_tokens_allowed(0), 0);
        assert_eq!(employment_agency_tokens_allowed(1), 0);
        assert_eq!(employment_agency_tokens_allowed(2), 1);
        assert_eq!(employment_agency_tokens_allowed(3), 2);
        assert_eq!(employment_agency_tokens_allowed(4), 3);
        assert_eq!(employment_agency_tokens_allowed(5), 4);
        assert_eq!(employment_agency_tokens_allowed(6), 4); // Capped at 4
    }
}
