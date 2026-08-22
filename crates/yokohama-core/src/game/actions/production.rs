//! Production area actions (Copper Mine, Silk Mill, Tea Plantation, Fishing Ground).

use crate::types::AreaType;

/// The EXPORTED production reward of `area_type` at `power` — the V4 spec §13
/// dispatcher over the four helpers below.
///
/// **A ROUTER, never a table.** Every arm CALLS the shipped helper; there is no
/// second reward derivation anywhere in this function, which is the whole
/// reason it exists (spec §13: "one job, one object, the dispatcher"). Its
/// callers are V4's `X3 MatchedProductionAccess`, the §6 escrow recipe, and the
/// §7 rules-arithmetic pins — three call sites that would otherwise each have
/// grown their own `match`.
///
/// # The domain is the four production areas, and outside it the answer is `None`
///
/// Spec §13 scopes this to "the four production helpers", and §3.2's S1/X3 both
/// sum "over production areas". So the ten other area types are OUTSIDE this
/// function's domain rather than worth zero in it, and the return type says so.
///
/// The alternative — a bare `u8` with 0 outside the domain — would collide two
/// different facts. `copper_mine_reward(2) == 0` is a REAL zero (spec §3.2's
/// cited sparsity fact, and the dead side of §6's escrow recipe); the Bank
/// exports 1..5 Yen through `bank_reward` and is simply not this router's
/// business. A caller keying on nonzero-vs-zero — which is exactly what the §6
/// recipe does — would read the second as the first and call the Bank a dead
/// area. `None` makes that caller handle the domain question explicitly.
///
/// # Two declared narrowings
///
/// * **Power is an INPUT, not something computed here.** All four helpers take
///   power and nothing else (no goods, no tech, no board state), so the caller
///   supplies the power reading it means — V4 supplies `realizable_power`.
/// * **Technology riders are the CALLER's**, exactly as they are for the
///   helpers themselves: Mining Technology's +1 Copper per gain event is applied
///   by `copper_bonus` at the gain site and is deliberately not baked in here.
pub fn exported_reward(area_type: AreaType, power: u8) -> Option<u8> {
    match area_type {
        AreaType::CopperMine => Some(copper_mine_reward(power)),
        AreaType::SilkMill => Some(silk_mill_reward(power)),
        AreaType::TeaPlantation => Some(tea_plantation_reward(power)),
        AreaType::FishingGround => Some(fishing_ground_reward(power)),
        // EXHAUSTIVE, with no `_` arm: a new area type is a compile error here
        // rather than a silent `None` that a caller would read as "exports
        // nothing".
        AreaType::Bank
        | AreaType::PortA
        | AreaType::PortB
        | AreaType::LaboratoryA
        | AreaType::LaboratoryB
        | AreaType::EmploymentAgency
        | AreaType::Chinatown
        | AreaType::Church
        | AreaType::Customs
        | AreaType::Canal => None,
    }
}

/// Calculate Copper Mine rewards based on power.
pub fn copper_mine_reward(power: u8) -> u8 {
    match power {
        0..=2 => 0,
        3 => 1,
        4 => 2,
        _ => 3, // 5+
    }
}

/// Calculate Silk Mill rewards based on power.
pub fn silk_mill_reward(power: u8) -> u8 {
    match power {
        0..=1 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        _ => 4, // 5+
    }
}

/// Calculate Tea Plantation rewards based on power.
pub fn tea_plantation_reward(power: u8) -> u8 {
    power.min(5) // 1:1, capped at 5
}

/// Calculate Fishing Ground rewards based on power.
pub fn fishing_ground_reward(power: u8) -> u8 {
    power.min(5) // 1:1, capped at 5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AreaType;

    /// Every non-production area type, listed by hand. A new `AreaType` variant
    /// is already a compile error inside [`exported_reward`]'s exhaustive match;
    /// this list is what makes it a compile error in the TEST too, so a variant
    /// cannot be added to the dispatcher's silent side without a cell saying so.
    const NON_PRODUCTION: [AreaType; 10] = [
        AreaType::Bank,
        AreaType::PortA,
        AreaType::PortB,
        AreaType::LaboratoryA,
        AreaType::LaboratoryB,
        AreaType::EmploymentAgency,
        AreaType::Chinatown,
        AreaType::Church,
        AreaType::Customs,
        AreaType::Canal,
    ];

    /// The dispatcher DISPATCHES: at every power the four production arms return
    /// exactly what the four shipped helpers return, so there is one reward
    /// derivation in the crate and this is a router over it (spec §13: "one job,
    /// one object").
    ///
    /// The sweep runs past the printed range on both ends — 0 is the
    /// no-presence reading and 6 is above the cap, and each helper caps for
    /// itself.
    #[test]
    fn the_dispatcher_agrees_with_every_production_helper_at_every_power() {
        for power in 0..=6u8 {
            assert_eq!(exported_reward(AreaType::CopperMine, power), Some(copper_mine_reward(power)));
            assert_eq!(exported_reward(AreaType::SilkMill, power), Some(silk_mill_reward(power)));
            assert_eq!(
                exported_reward(AreaType::TeaPlantation, power),
                Some(tea_plantation_reward(power))
            );
            assert_eq!(
                exported_reward(AreaType::FishingGround, power),
                Some(fishing_ground_reward(power))
            );
        }
    }

    /// The dispatcher's DOMAIN is the four production areas and nothing else
    /// (spec §13: "over the four production helpers"), and outside it the answer
    /// is `None` — "this router has nothing to say here" — never a number.
    #[test]
    fn the_dispatcher_is_silent_on_every_non_production_area_type() {
        for area_type in NON_PRODUCTION {
            assert!(
                !area_type.is_production(),
                "{area_type:?} is in the non-production list but reads as production"
            );
            for power in 0..=6u8 {
                assert_eq!(
                    exported_reward(area_type, power),
                    None,
                    "{area_type:?} at power {power} is outside the four production helpers"
                );
            }
        }
    }

    /// **WHY THE RETURN TYPE IS AN OPTION.** A Copper Mine at power 2 exports
    /// ZERO — spec §3.2's cited sparsity fact, and the whole basis of the §6
    /// escrow recipe's dead side. A dispatcher returning a bare `u8` would spell
    /// that genuine zero the same way it spelled "the Bank is not one of my
    /// four", and a caller keying on nonzero-vs-zero would then read the Bank —
    /// which pays 1..5 Yen — as a dead area. The two facts are different facts,
    /// so they get different values.
    #[test]
    fn a_sparsity_zero_is_some_zero_and_never_a_none() {
        assert_eq!(exported_reward(AreaType::CopperMine, 2), Some(0));
        assert_eq!(exported_reward(AreaType::SilkMill, 1), Some(0));
        assert_eq!(exported_reward(AreaType::Bank, 2), None);
        assert_ne!(exported_reward(AreaType::CopperMine, 2), exported_reward(AreaType::Bank, 2));
    }

    #[test]
    fn test_copper_mine_rewards() {
        assert_eq!(copper_mine_reward(1), 0);
        assert_eq!(copper_mine_reward(2), 0);
        assert_eq!(copper_mine_reward(3), 1);
        assert_eq!(copper_mine_reward(4), 2);
        assert_eq!(copper_mine_reward(5), 3);
    }

    #[test]
    fn test_silk_mill_rewards() {
        assert_eq!(silk_mill_reward(1), 0);
        assert_eq!(silk_mill_reward(2), 1);
        assert_eq!(silk_mill_reward(3), 2);
        assert_eq!(silk_mill_reward(4), 3);
        assert_eq!(silk_mill_reward(5), 4);
    }

    #[test]
    fn test_tea_plantation_rewards() {
        for power in 1..=5 {
            assert_eq!(tea_plantation_reward(power), power);
        }
    }

    #[test]
    fn test_fishing_ground_rewards() {
        for power in 1..=5 {
            assert_eq!(fishing_ground_reward(power), power);
        }
    }

    #[test]
    fn test_power_zero_returns_zero() {
        // All production areas should return 0 at power 0
        assert_eq!(copper_mine_reward(0), 0);
        assert_eq!(silk_mill_reward(0), 0);
        assert_eq!(tea_plantation_reward(0), 0);
        assert_eq!(fishing_ground_reward(0), 0);
    }

    #[test]
    fn test_power_capped_at_five() {
        // Power above 5 should be treated as 5
        assert_eq!(copper_mine_reward(6), 3);
        assert_eq!(silk_mill_reward(6), 4);
        assert_eq!(tea_plantation_reward(6), 5);
        assert_eq!(fishing_ground_reward(6), 5);
    }
}
