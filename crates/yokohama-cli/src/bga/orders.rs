//! The vector order-card mapper (plan T3; evidence
//! `docs/plans/2026-08-08-bga-card-mapping.md`).
//!
//! **BGA's card numbering is a permutation of ours and numbers can never
//! transfer** (design §1; the 2026-08-07 O36/O24 mis-entries are exactly what
//! transcribing BGA's printed number as ours produces). The identification key
//! is the requirement+reward VECTOR: all 36 engine orders carry distinct
//! `(req_copper, req_silk, req_tea, req_seafood, vp, money, imports,
//! warehouse, movement)` tuples, and the five reward channels BGA does not
//! carry (goods-as-rewards, any_trade_good) are zero on every card, so the key
//! is not lossy. That uniqueness is a property of CARD DATA a `gen-data` run
//! could break, so [`OrderMapper::from_rules`] asserts it at construction —
//! a future data change fails a test rather than a live game.
//!
//! The requirement 4-tuple ALONE is insufficient — O10 and O32 share
//! `(2,0,1,1)`, and the H0 corpus contains both — which is why the reward half
//! of the key is load-bearing and why a requirements-only mapper is not a
//! simplification but a bug.
//!
//! BGA's `country` code is redundant given the vector — which makes it a free
//! TRIPWIRE: a vector hit with a country miss means one of the two data shapes
//! moved, and that is a hard error even though the match already succeeded.

use super::{require_u8, require_u64, BgaMapError};
use serde_json::Value;
use std::collections::BTreeMap;
use yokohama_core::{Country, OrderCardId, Rules};

/// The nine-field identification key, in the fixed order
/// `req c/s/t/f · vp/yen/imp/wh/mov` (the mapping doc's column order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrderVector {
    pub req_copper: u8,
    pub req_silk: u8,
    pub req_tea: u8,
    pub req_seafood: u8,
    pub bonus_vp: u8,
    pub bonus_yen: u8,
    pub bonus_import: u8,
    pub bonus_wh: u8,
    pub bonus_move: u8,
}

impl OrderVector {
    fn key(&self) -> [u8; 9] {
        [
            self.req_copper,
            self.req_silk,
            self.req_tea,
            self.req_seafood,
            self.bonus_vp,
            self.bonus_yen,
            self.bonus_import,
            self.bonus_wh,
            self.bonus_move,
        ]
    }

    /// Read the key off a BGA order object (`order_card`/`order` args or an
    /// `orders`-map row). Field correspondence per the mapping doc §1:
    /// `req_fish → seafood`, `bonus_wh → warehouse_tokens`, `bonus_move →
    /// movement`. All values arrive as strings or ints; anything else is a
    /// typed error, never a `0`.
    pub fn from_bga_row(row: &Value, context: &str) -> Result<OrderVector, BgaMapError> {
        Ok(OrderVector {
            req_copper: require_u8(row, "req_copper", context)?,
            req_silk: require_u8(row, "req_silk", context)?,
            req_tea: require_u8(row, "req_tea", context)?,
            req_seafood: require_u8(row, "req_fish", context)?,
            bonus_vp: require_u8(row, "bonus_vp", context)?,
            bonus_yen: require_u8(row, "bonus_yen", context)?,
            bonus_import: require_u8(row, "bonus_import", context)?,
            bonus_wh: require_u8(row, "bonus_wh", context)?,
            bonus_move: require_u8(row, "bonus_move", context)?,
        })
    }

    /// The same key read off an ENGINE card — the construction-side half.
    pub fn from_engine(card: &yokohama_core::OrderCard) -> OrderVector {
        OrderVector {
            req_copper: card.requirements.copper,
            req_silk: card.requirements.silk,
            req_tea: card.requirements.tea,
            req_seafood: card.requirements.seafood,
            bonus_vp: card.rewards.victory_points,
            bonus_yen: card.rewards.money,
            bonus_import: card.rewards.imported_goods,
            bonus_wh: card.rewards.warehouse_tokens,
            bonus_move: card.rewards.movement,
        }
    }
}

impl std::fmt::Display for OrderVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "req {}/{}/{}/{} rewards {}/{}/{}/{}/{}",
            self.req_copper,
            self.req_silk,
            self.req_tea,
            self.req_seafood,
            self.bonus_vp,
            self.bonus_yen,
            self.bonus_import,
            self.bonus_wh,
            self.bonus_move
        )
    }
}

/// BGA country code → engine [`Country`]. Pinned from the 20 matched corpus
/// pairs plus two independent corroborations (flag nouns in completion logs,
/// end-of-game flag-counter balance) — mapping doc §3. **Code 1 = American is
/// DERIVED, not directly named**: no American order or tech was completed in
/// the corpus game, so it rests on elimination (2..=5 are name-pinned and the
/// counter suffix set is exactly {us, gb, ge, fr, ne}), the one matched pair
/// BGA 13 → O31, and the tech-class multiset agreement — three independent
/// supports, no contradiction, but an annotation the sheet carries.
pub fn engine_country(code: u64) -> Result<Country, BgaMapError> {
    match code {
        1 => Ok(Country::American),
        2 => Ok(Country::German),
        3 => Ok(Country::French),
        4 => Ok(Country::Dutch),
        5 => Ok(Country::English),
        other => Err(BgaMapError::UnknownCountryCode { code: other }),
    }
}

/// The reverse direction: engine [`Country`] → BGA code (the tripwire check,
/// and the class comparisons in the tech ledger).
pub fn engine_country_code(country: Country) -> u64 {
    match country {
        Country::American => 1,
        Country::German => 2,
        Country::French => 3,
        Country::Dutch => 4,
        Country::English => 5,
    }
}

/// A successfully identified order row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedOrder {
    pub card: OrderCardId,
    pub bga_order_id: u64,
    /// BGA's per-table instance handle (deck slot), when the row carries one.
    pub bga_order_key: Option<u64>,
}

/// The vector table, built ONCE from `Rules` with uniqueness asserted.
#[derive(Debug)]
pub struct OrderMapper {
    by_vector: BTreeMap<[u8; 9], (OrderCardId, Country)>,
}

impl OrderMapper {
    pub fn from_rules(rules: &Rules) -> Result<OrderMapper, BgaMapError> {
        let mut by_vector = BTreeMap::new();
        for card in rules.orders.iter_sorted_by_id() {
            let vec = OrderVector::from_engine(card);
            if let Some((prior, _)) = by_vector.insert(vec.key(), (card.id, card.country)) {
                return Err(BgaMapError::DuplicateVectorInRules {
                    first: prior.0,
                    second: card.id.0,
                });
            }
        }
        Ok(OrderMapper { by_vector })
    }

    /// Vector → card, no country information (used where BGA gives none).
    pub fn map_vector(&self, v: &OrderVector, context: &str) -> Result<OrderCardId, BgaMapError> {
        self.by_vector
            .get(&v.key())
            .map(|(id, _)| *id)
            .ok_or_else(|| BgaMapError::NoVectorMatch {
                context: context.to_string(),
                vector: v.to_string(),
            })
    }

    /// Map a full BGA order object: vector match PLUS the country tripwire.
    /// No match, a malformed row, or a country disagreement is a typed error
    /// and never a guess.
    pub fn map_row(&self, row: &Value, context: &str) -> Result<MappedOrder, BgaMapError> {
        let bga_order_id = require_u64(row, "order_id", context)?;
        let vector = OrderVector::from_bga_row(row, context)?;
        let (card, country) = *self
            .by_vector
            .get(&vector.key())
            .ok_or_else(|| BgaMapError::NoVectorMatch {
                context: context.to_string(),
                vector: vector.to_string(),
            })?;
        let bga_country_code = require_u64(row, "country", context)?;
        // The tripwire: redundant, therefore diagnostic.
        engine_country(bga_country_code)?;
        let expected_code = engine_country_code(country);
        if bga_country_code != expected_code {
            return Err(BgaMapError::CountryDisagreement {
                bga_order_id,
                bga_country_code,
                matched_card: card.0,
                expected_code,
            });
        }
        let bga_order_key = row.get("order_key").and_then(yokohama_bridge::feed::flex_u64);
        Ok(MappedOrder {
            card,
            bga_order_id,
            bga_order_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn mapper() -> OrderMapper {
        OrderMapper::from_rules(&Rules::standard()).expect("36 unique vectors")
    }

    fn row(order_id: u64, country: u64, req: [u8; 4], rew: [u8; 5]) -> Value {
        // String-typed fields, as BGA serves them.
        json!({
            "order_id": order_id.to_string(),
            "country": country.to_string(),
            "req_copper": req[0].to_string(), "req_silk": req[1].to_string(),
            "req_tea": req[2].to_string(), "req_fish": req[3].to_string(),
            "bonus_vp": rew[0].to_string(), "bonus_yen": rew[1].to_string(),
            "bonus_import": rew[2].to_string(), "bonus_wh": rew[3].to_string(),
            "bonus_move": rew[4].to_string(),
            "order_key": "7", "location": "2", "position": "1", "completed": "0",
        })
    }

    /// One golden row: (bga_order_id, bga_country, req c/s/t/f,
    /// rewards vp/yen/imp/wh/mov, expected engine card).
    type GoldenRow = (u64, u64, [u8; 4], [u8; 5], u8);

    /// The 20 BGA order rows the H0 corpus observed, verbatim from the
    /// card-mapping evidence table §2 — the regression golden set.
    const CORPUS_ROWS: [GoldenRow; 20] = [
        (2, 4, [1, 0, 1, 2], [2, 1, 1, 0, 0], 6),
        (3, 4, [1, 1, 1, 0], [3, 0, 1, 0, 0], 8),
        (5, 4, [0, 2, 2, 2], [11, 0, 0, 0, 0], 21),
        (7, 2, [0, 2, 1, 1], [5, 0, 0, 1, 0], 4),
        (10, 2, [1, 2, 0, 2], [8, 1, 0, 0, 0], 17),
        (11, 2, [0, 3, 0, 1], [2, 1, 1, 0, 0], 9),
        (13, 1, [1, 0, 0, 3], [2, 0, 0, 0, 2], 31),
        (21, 5, [2, 1, 2, 0], [8, 0, 1, 0, 0], 20),
        (22, 5, [2, 0, 1, 1], [4, 0, 0, 0, 2], 10),
        (23, 5, [3, 1, 1, 0], [9, 0, 1, 0, 0], 15),
        (24, 5, [2, 0, 0, 1], [4, 1, 0, 0, 0], 3),
        (25, 5, [2, 0, 0, 2], [1, 2, 0, 1, 0], 27),
        (26, 5, [3, 1, 0, 2], [7, 2, 1, 0, 0], 24),
        (27, 5, [2, 1, 0, 2], [8, 0, 1, 0, 0], 13),
        (28, 5, [2, 0, 1, 1], [5, 0, 1, 0, 0], 32),
        (29, 3, [1, 0, 3, 0], [3, 0, 1, 0, 0], 7),
        (30, 3, [0, 0, 2, 1], [3, 0, 0, 0, 0], 2),
        (33, 3, [0, 1, 4, 0], [8, 0, 0, 0, 0], 29),
        (34, 3, [1, 2, 3, 0], [9, 0, 1, 0, 0], 23),
        (36, 3, [0, 2, 3, 0], [4, 2, 0, 0, 0], 30),
    ];

    #[test]
    fn all_36_engine_vectors_are_unique_at_construction() {
        // The key's uniqueness is asserted, not trusted — a gen-data change
        // that broke it must fail here rather than at a live import.
        assert_eq!(mapper().by_vector.len(), 36);
    }

    #[test]
    fn a_synthetic_vector_collision_is_a_construction_error() {
        let mut rules = Rules::empty_for_tests();
        let card = |id: u8| yokohama_core::OrderCard {
            id: OrderCardId(id),
            country: Country::English,
            requirements: yokohama_core::TradeGoodRequirement::new(1, 2, 3, 4),
            rewards: yokohama_core::OrderRewards {
                victory_points: 5,
                ..Default::default()
            },
        };
        rules.orders.insert(card(1), "o1");
        rules.orders.insert(card(2), "o2");
        assert_eq!(
            OrderMapper::from_rules(&rules).unwrap_err(),
            BgaMapError::DuplicateVectorInRules { first: 1, second: 2 }
        );
    }

    #[test]
    fn all_20_corpus_rows_map_to_their_evidenced_engine_cards() {
        let m = mapper();
        for (oid, country, req, rew, expect) in CORPUS_ROWS {
            let mapped = m.map_row(&row(oid, country, req, rew), "corpus row").unwrap();
            assert_eq!(mapped.card, OrderCardId(expect), "BGA order {oid}");
            assert_eq!(mapped.bga_order_id, oid);
            assert_eq!(mapped.bga_order_key, Some(7));
        }
    }

    #[test]
    fn the_five_cross_table_permutation_pairs_reproduce() {
        // Design §1's pairs, measured on the EXPIRED table #895608640 and
        // re-derived independently on #895724349 — the cross-table stability
        // fixture (mapping doc §2.1).
        let expected: [(u64, u8); 5] = [(5, 21), (10, 17), (11, 9), (21, 20), (28, 32)];
        for (bga, engine) in expected {
            let (_, _, req, rew, expect) = CORPUS_ROWS
                .iter()
                .find(|r| r.0 == bga)
                .copied()
                .expect("pair is in the corpus set");
            assert_eq!(expect, engine);
            let mapped = mapper().map_row(&row(bga, CORPUS_ROWS.iter().find(|r| r.0 == bga).unwrap().1, req, rew), "pair").unwrap();
            assert_eq!(mapped.card, OrderCardId(engine));
        }
    }

    #[test]
    fn the_2026_08_07_mis_entries_map_to_what_bga_actually_showed() {
        // The manual-tracking experiment recorded O36 and O24 by transcribing
        // BGA's printed numbers. The mapper, given the same BGA rows, returns
        // what the cards actually were (plan §5).
        let m = mapper();
        let bga36 = m.map_row(&row(36, 3, [0, 2, 3, 0], [4, 2, 0, 0, 0]), "mis-entry 36").unwrap();
        assert_eq!(bga36.card, OrderCardId(30), "BGA order 36 is our O30, never O36");
        let bga24 = m.map_row(&row(24, 5, [2, 0, 0, 1], [4, 1, 0, 0, 0]), "mis-entry 24").unwrap();
        assert_eq!(bga24.card, OrderCardId(3), "BGA order 24 is our O03, never O24");
    }

    #[test]
    fn our_o24_and_o36_remain_identifiable_by_their_own_vectors() {
        // The inverse half of the fixture: the engine cards the human
        // MEANT to record are themselves unambiguous when they appear.
        let rules = Rules::standard();
        let m = mapper();
        for id in [24u8, 36] {
            let card = rules.orders.get(OrderCardId(id)).unwrap();
            let v = OrderVector::from_engine(card);
            assert_eq!(m.map_vector(&v, "inverse").unwrap(), OrderCardId(id));
        }
        // And O24's vector is NOT what BGA row 24 carries.
        let o24 = OrderVector::from_engine(rules.orders.get(OrderCardId(24)).unwrap());
        let bga24 = OrderVector::from_bga_row(&row(24, 5, [2, 0, 0, 1], [4, 1, 0, 0, 0]), "x").unwrap();
        assert_ne!(o24, bga24);
    }

    #[test]
    fn an_unmatched_vector_is_a_typed_error_naming_what_was_seen() {
        let err = mapper()
            .map_row(&row(99, 5, [4, 4, 4, 4], [1, 1, 1, 1, 1]), "drift probe")
            .unwrap_err();
        match err {
            BgaMapError::NoVectorMatch { vector, .. } => {
                assert!(vector.contains("req 4/4/4/4"), "{vector}");
            }
            other => panic!("expected NoVectorMatch, got {other:?}"),
        }
    }

    #[test]
    fn a_country_disagreement_is_a_hard_error_even_though_the_vector_matched() {
        // BGA 36's vector with a Dutch code instead of French: the tripwire.
        let err = mapper()
            .map_row(&row(36, 4, [0, 2, 3, 0], [4, 2, 0, 0, 0]), "tripwire")
            .unwrap_err();
        assert_eq!(
            err,
            BgaMapError::CountryDisagreement {
                bga_order_id: 36,
                bga_country_code: 4,
                matched_card: 30,
                expected_code: 3,
            }
        );
    }

    #[test]
    fn a_non_numeric_field_is_an_error_not_a_zero() {
        let mut bad = row(2, 4, [1, 0, 1, 2], [2, 1, 1, 0, 0]);
        bad["req_tea"] = serde_json::json!("plenty");
        match mapper().map_row(&bad, "malformed").unwrap_err() {
            BgaMapError::NonNumeric { field, value, .. } => {
                assert_eq!(field, "req_tea");
                assert_eq!(value, "\"plenty\"");
            }
            other => panic!("expected NonNumeric, got {other:?}"),
        }
    }

    #[test]
    fn an_unknown_country_code_is_a_typed_error() {
        assert_eq!(
            engine_country(6).unwrap_err(),
            BgaMapError::UnknownCountryCode { code: 6 }
        );
    }
}
