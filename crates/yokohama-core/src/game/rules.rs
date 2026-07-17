//! Static game data bundled for the engine API.

use crate::game::board_gen::canonical_five_power_tokens;
use crate::game::cards::{OrderRegistry, TechRegistry};
use crate::types::{BuildingSite, FivePowerEffect};

/// All static card/token data the engine needs. Built once, passed by reference.
#[derive(Debug, Clone)]
pub struct Rules {
    pub orders: OrderRegistry,
    pub techs: TechRegistry,
    pub building_sites: Vec<BuildingSite>,
    pub five_power_tokens: Vec<FivePowerEffect>,
}

impl Rules {
    /// Empty registries with canonical tokens — for unit tests that don't need card data.
    pub fn empty_for_tests() -> Self {
        Self {
            orders: OrderRegistry::new(),
            techs: TechRegistry::new(),
            building_sites: Vec::new(),
            five_power_tokens: canonical_five_power_tokens(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_rules_for_tests_has_canonical_tokens() {
        let rules = Rules::empty_for_tests();
        assert_eq!(rules.five_power_tokens.len(), 20);
        assert!(rules.orders.iter().next().is_none());
    }
}
