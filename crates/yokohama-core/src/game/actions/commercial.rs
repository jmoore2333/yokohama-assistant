//! Commercial area actions (Bank, Employment Agency, Chinatown).

use crate::types::{ChinatownExchange, GameState, PlayerId};

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

/// Apply a Chinatown exchange.
pub fn apply_chinatown_exchange(
    state: &mut GameState,
    player: PlayerId,
    exchange: ChinatownExchange,
) -> Result<(), &'static str> {
    let p = &mut state.players[player.0 as usize];

    match exchange {
        ChinatownExchange::CopperToYen => {
            if p.copper < 1 {
                return Err("Not enough copper");
            }
            p.copper -= 1;
            p.money += 2;
        }
        ChinatownExchange::SilkToYen => {
            if p.silk < 1 {
                return Err("Not enough silk");
            }
            p.silk -= 1;
            p.money += 2;
        }
        ChinatownExchange::TeaToYen => {
            if p.tea < 1 {
                return Err("Not enough tea");
            }
            p.tea -= 1;
            p.money += 1;
        }
        ChinatownExchange::SeafoodToYen => {
            if p.seafood < 1 {
                return Err("Not enough seafood");
            }
            p.seafood -= 1;
            p.money += 1;
        }
        ChinatownExchange::YenToCopper => {
            if p.money < 2 {
                return Err("Not enough money");
            }
            p.money -= 2;
            p.copper += 1;
        }
        ChinatownExchange::YenToSilk => {
            if p.money < 2 {
                return Err("Not enough money");
            }
            p.money -= 2;
            p.silk += 1;
        }
        ChinatownExchange::YenToTea => {
            if p.money < 1 {
                return Err("Not enough money");
            }
            p.money -= 1;
            p.tea += 1;
        }
        ChinatownExchange::YenToSeafood => {
            if p.money < 1 {
                return Err("Not enough money");
            }
            p.money -= 1;
            p.seafood += 1;
        }
        ChinatownExchange::YenToImported => {
            if p.money < 4 {
                return Err("Not enough money");
            }
            p.money -= 4;
            p.imported_goods += 1;
        }
    }

    Ok(())
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
