//! Static game data bundled for the engine API.

use crate::game::board_gen::canonical_five_power_tokens;
use crate::game::cards::{standard_building_sites, OrderRegistry, TechRegistry};
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

    /// Full standard card data and canonical tokens (verified 2026-07-17).
    pub fn standard() -> Self {
        Self {
            orders: OrderRegistry::standard(),
            techs: TechRegistry::standard(),
            building_sites: standard_building_sites(),
            five_power_tokens: canonical_five_power_tokens(),
        }
    }

    /// Stable content hash of the rules data (design §4.6): FNV-1a 64 over a
    /// canonical line-per-entry string in sorted-id order, plus the board
    /// derivation version. Hand-rolled — std's Hasher is not stable across
    /// releases and this hash lives in saved records.
    pub fn content_hash(&self) -> String {
        fn fnv1a(bytes: &[u8], mut h: u64) -> u64 {
            for &b in bytes { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
            h
        }
        let mut canon = String::new();
        for o in self.orders.iter_sorted_by_id() {
            canon.push_str(&format!("O|{:?}\n", o));
        }
        for t in self.techs.iter_sorted_by_id() {
            canon.push_str(&format!("T|{:?}\n", t));
        }
        for (i, s) in self.building_sites.iter().enumerate() {
            canon.push_str(&format!("S|{i}|{:?}\n", s));
        }
        for (i, f) in self.five_power_tokens.iter().enumerate() {
            canon.push_str(&format!("F|{i}|{:?}\n", f));
        }
        canon.push_str(&format!("B|{}\n", crate::game::board_gen::BOARD_DERIVATION_VERSION));
        format!("{:016x}", fnv1a(canon.as_bytes(), 0xcbf29ce484222325))
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

    #[test]
    fn content_hash_is_stable_and_distinguishes_data() {
        let a = Rules::standard().content_hash();
        let b = Rules::standard().content_hash();
        assert_eq!(a, b, "hash must be deterministic");
        assert_eq!(a.len(), 16, "fnv1a-64 hex");
        assert_ne!(a, Rules::empty_for_tests().content_hash());
    }
}
