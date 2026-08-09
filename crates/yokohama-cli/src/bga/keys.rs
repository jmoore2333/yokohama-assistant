//! The deal-order KEY MODEL (H2, census §1) — the mechanism that overturned
//! plan §0.5's bound.
//!
//! BGA's `order_key`/`tech_key` are DEAL-ORDER SLOT KEYS, measured against
//! every snapshot and socket map of the H0 corpus (census §1): displays slide
//! left keeping relative order, refills enter at the right with strictly
//! ascending keys, and the undrawn deck is always the ascending TAIL of the
//! key space. That makes the key layout derivable at 2P:
//!
//! - order keys `1..=2N` are the dealt starting orders, two consecutive keys
//!   per seat (pinned per-seat by the `orderDiscarded` events, which name the
//!   discarded card WITH its key);
//! - order keys `2N+1..=2N+6` are the initial Port A display, positions 1..6;
//! - order keys `2N+7..` are the deck in draw order;
//! - tech keys `1..=6` are the initial Lab A display; `7..` the deck.
//!
//! **The §8.3 deck fence is honored by construction**: identity is ingested
//! ONLY from rows at location 2/4/5 (display / held-or-completed / setup
//! discard) and from card-bearing notification args — never from a
//! `location "1"` row, whose KEY (a slot number) is all this module reads
//! (for the tail invariant below). A card's identity is time-invariant, so an
//! identity observed at any legitimate moment may name that key at its
//! earlier reveal moment; what the fence forbids is identities that were
//! never legitimately observable, and those stay structurally absent here.
//!
//! Every mechanic above is CHECKED, not assumed: a key observed with two
//! different identities, a deck row keyed below a visible row in the same
//! map, or a display whose position order disagrees with key order is a typed
//! error — the model refuses rather than guesses (design §6).

use super::orders::{OrderMapper, OrderVector};
use super::{require_u64, BgaMapError, ImportError};
use serde_json::Value;
use std::collections::BTreeMap;
use yokohama_bridge::feed::{CollapsedFeed, Packet};
use yokohama_core::OrderCardId;

/// What one scan pass learned about a keyed card map (orders or techs).
#[derive(Debug, Clone, Default)]
pub struct KeyModel {
    /// `order_key` → engine card, from every location-2/4/5 sighting.
    pub order_cards: BTreeMap<u64, OrderCardId>,
    /// `order_key` → BGA `order_id` (provenance / diagnostics).
    pub order_ids: BTreeMap<u64, u64>,
    /// `tech_key` → BGA `tech_id` (identity resolution is the ledger's job).
    pub tech_ids: BTreeMap<u64, u64>,
    /// BGA `tech_id` → `(country_code, req_gears)` — the class, from
    /// legitimate (non-deck) sightings; the ledger's corroboration input.
    pub tech_classes: BTreeMap<u64, (u64, u8)>,
    /// `order_key` → BGA player_id that ended up holding/completing it
    /// (location-4 sightings) — the thin-take ownership evidence.
    pub order_owner: BTreeMap<u64, String>,
    /// Highest order key seen anywhere (incl. deck keys — count only).
    pub max_order_key: u64,
    pub max_tech_key: u64,
}

fn is_deck(row: &Value) -> bool {
    row.get("location").and_then(|l| l.as_str()) == Some("1")
}

impl KeyModel {
    fn ingest_order_row(
        &mut self,
        row: &Value,
        mapper: &OrderMapper,
        context: &str,
    ) -> Result<(), ImportError> {
        let key = require_u64(row, "order_key", context).map_err(ImportError::Map)?;
        self.max_order_key = self.max_order_key.max(key);
        if is_deck(row) {
            return Ok(()); // the fence: a deck row contributes its KEY only
        }
        let mapped = mapper.map_row(row, context)?;
        if let Some(prev) = self.order_cards.insert(key, mapped.card) {
            if prev != mapped.card {
                return Err(ImportError::Sheet {
                    detail: format!(
                        "order_key {key} observed as two different cards (O{:02} then O{:02}) — \
                         the key model's identity invariant is broken ({context})",
                        prev.0, mapped.card.0
                    ),
                });
            }
        }
        self.order_ids.insert(key, mapped.bga_order_id);
        if row.get("location").and_then(|l| l.as_str()) == Some("4") {
            if let Some(pid) = row.get("player_id").and_then(|p| p.as_str()) {
                if pid != "0" {
                    if let Some(prev) = self.order_owner.insert(key, pid.to_string()) {
                        if prev != pid {
                            return Err(ImportError::Sheet {
                                detail: format!(
                                    "order_key {key} observed held by two different players \
                                     ({prev} then {pid}) — ownership is stable once taken ({context})"
                                ),
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn ingest_tech_row(&mut self, row: &Value, context: &str) -> Result<(), ImportError> {
        let key = require_u64(row, "tech_key", context).map_err(ImportError::Map)?;
        self.max_tech_key = self.max_tech_key.max(key);
        if is_deck(row) {
            return Ok(());
        }
        let tech_id = require_u64(row, "tech_id", context).map_err(ImportError::Map)?;
        if let (Ok(country), Ok(gears)) = (
            require_u64(row, "country", context),
            super::require_u8(row, "req_gears", context),
        ) {
            if let Some(prev) = self.tech_classes.insert(tech_id, (country, gears)) {
                if prev != (country, gears) {
                    return Err(ImportError::Sheet {
                        detail: format!(
                            "tech_id {tech_id} observed with two different classes \
                             ({prev:?} then {:?}) — {context}",
                            (country, gears)
                        ),
                    });
                }
            }
        }
        if let Some(prev) = self.tech_ids.insert(key, tech_id) {
            if prev != tech_id {
                return Err(ImportError::Sheet {
                    detail: format!(
                        "tech_key {key} observed as two different tech_ids ({prev} then {tech_id}) — \
                         the key model's identity invariant is broken ({context})"
                    ),
                });
            }
        }
        Ok(())
    }

    /// Scan one keyed card MAP (a snapshot's or a socket notification's
    /// `orders`/`techs` object), enforcing the two mechanics invariants:
    /// deck keys are the ascending TAIL, and display positions ascend with
    /// keys (the slide-left mechanic the census measured).
    fn ingest_map(
        &mut self,
        map: &Value,
        orders: bool,
        mapper: &OrderMapper,
        context: &str,
    ) -> Result<(), ImportError> {
        let obj = match map.as_object() {
            Some(o) => o,
            None => return Ok(()), // absent map — nothing to scan
        };
        let key_field = if orders { "order_key" } else { "tech_key" };
        let mut min_deck = u64::MAX;
        let mut max_visible = 0u64;
        let mut display: Vec<(u64, u64)> = Vec::new(); // (position, key)
        for row in obj.values() {
            let key = require_u64(row, key_field, context).map_err(ImportError::Map)?;
            if is_deck(row) {
                min_deck = min_deck.min(key);
            } else {
                max_visible = max_visible.max(key);
                if row.get("location").and_then(|l| l.as_str()) == Some("2") {
                    let pos = require_u64(row, "position", context).map_err(ImportError::Map)?;
                    display.push((pos, key));
                }
            }
            if orders {
                self.ingest_order_row(row, mapper, context)?;
            } else {
                self.ingest_tech_row(row, context)?;
            }
        }
        if min_deck != u64::MAX && min_deck <= max_visible {
            return Err(ImportError::Sheet {
                detail: format!(
                    "{context}: deck {key_field} {min_deck} is not above every visible key \
                     (max visible {max_visible}) — the ascending-tail deck mechanic is broken"
                ),
            });
        }
        display.sort();
        if display.windows(2).any(|w| w[0].1 >= w[1].1) {
            return Err(ImportError::Sheet {
                detail: format!(
                    "{context}: display keys are not ascending with position ({display:?}) — \
                     the slide-left display mechanic is broken"
                ),
            });
        }
        Ok(())
    }

    fn ingest_notification_args(
        &mut self,
        args: &Value,
        mapper: &OrderMapper,
        context: &str,
    ) -> Result<(), ImportError> {
        // Card-bearing single objects: `order` (orderDiscarded/orderCompletion),
        // `order_card` (orderAcquisition/orderRefresh), `tech_card`
        // (techAcquisition). Each names a card at a legitimate reveal moment.
        for field in ["order", "order_card"] {
            if let Some(row) = args.get(field) {
                if row.is_object() {
                    self.ingest_order_row(row, mapper, &format!("{context}.{field}"))?;
                }
            }
        }
        if let Some(row) = args.get("tech_card") {
            if row.is_object() {
                self.ingest_tech_row(row, &format!("{context}.tech_card"))?;
            }
        }
        // Whole maps riding socket notifications.
        if let Some(map) = args.get("orders") {
            self.ingest_map(map, true, mapper, &format!("{context}.orders"))?;
        }
        if let Some(map) = args.get("techs") {
            self.ingest_map(map, false, mapper, &format!("{context}.techs"))?;
        }
        Ok(())
    }

    fn ingest_packet(&mut self, pk: &Packet, mapper: &OrderMapper) -> Result<(), ImportError> {
        for n in &pk.notifications {
            if n.args.is_object() {
                let ctx = format!("move {} {}", pk.move_id.unwrap_or(0), n.kind);
                self.ingest_notification_args(&n.args, mapper, &ctx)?;
            }
        }
        Ok(())
    }

    /// Build the model from everything the collapsed feed legitimately
    /// observed: all snapshots' card maps plus every card-bearing
    /// notification on either transport.
    pub fn build(feed: &CollapsedFeed, mapper: &OrderMapper) -> Result<KeyModel, ImportError> {
        let mut model = KeyModel::default();
        for (i, snap) in feed.snapshots.iter().enumerate() {
            let gd = &snap.gamedatas;
            if let Some(map) = gd.get("orders") {
                model.ingest_map(map, true, mapper, &format!("snapshot {i} orders"))?;
            }
            if let Some(map) = gd.get("techs") {
                model.ingest_map(map, false, mapper, &format!("snapshot {i} techs"))?;
            }
        }
        for mv in &feed.moves {
            for pk in [mv.history.as_ref(), mv.socket.as_ref()].into_iter().flatten() {
                model.ingest_packet(pk, mapper)?;
            }
        }
        Ok(model)
    }

    /// The initial Port A keys at `player_count` seats: `2N+1..=2N+6`.
    pub fn initial_port_keys(player_count: u8) -> [u64; 6] {
        let base = 2 * player_count as u64;
        [base + 1, base + 2, base + 3, base + 4, base + 5, base + 6]
    }

    /// The first order deck key: `2N+7`.
    pub fn first_order_deck_key(player_count: u8) -> u64 {
        2 * player_count as u64 + 7
    }

    /// Engine identity of an order key, or a typed error naming the gap.
    pub fn order_card(&self, key: u64) -> Result<OrderCardId, ImportError> {
        self.order_cards.get(&key).copied().ok_or_else(|| ImportError::Sheet {
            detail: format!(
                "order_key {key} was never observed at a legitimate location (2/4/5) anywhere \
                 in the capture — its identity is unknown and may not be guessed"
            ),
        })
    }

    /// BGA tech_id of a tech key, or a typed error naming the gap.
    pub fn tech_id(&self, key: u64) -> Result<u64, ImportError> {
        self.tech_ids.get(&key).copied().ok_or_else(|| ImportError::Sheet {
            detail: format!(
                "tech_key {key} was never observed at a legitimate location anywhere in the \
                 capture — its identity is unknown and may not be guessed"
            ),
        })
    }
}

/// Read the full nine-field vector + key + id off any BGA order row —
/// re-exported convenience for callers that hold a row.
pub fn order_vector(row: &Value, context: &str) -> Result<OrderVector, BgaMapError> {
    OrderVector::from_bga_row(row, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yokohama_bridge::feed::read_and_collapse;
    use yokohama_core::Rules;

    fn excerpt_feed() -> CollapsedFeed {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../yokohama-bridge/tests/data/excerpt-895724349.ndjson"
        );
        let text = std::fs::read_to_string(path).expect("committed excerpt exists");
        read_and_collapse(&text, Some(895_724_349)).expect("excerpt collapses")
    }

    #[test]
    fn the_excerpt_key_model_reads_identities_and_honors_the_fence() {
        let mapper = OrderMapper::from_rules(&Rules::standard()).unwrap();
        let model = KeyModel::build(&excerpt_feed(), &mapper).unwrap();
        // The excerpt's snapshot carries display keys 7..=12 and held keys
        // 2/4/5/6, discards name keys 1 and 3 — so 1..=12 are identified and
        // the 8 deck rows (13..=20) contribute a count, never an identity.
        for k in 1..=12u64 {
            assert!(model.order_cards.contains_key(&k), "order_key {k} identified");
        }
        for k in 13..=20u64 {
            assert!(!model.order_cards.contains_key(&k), "deck key {k} must stay unidentified");
        }
        assert_eq!(model.max_order_key, 20);
        // Initial Port A at 2P = keys 5..=10.
        assert_eq!(KeyModel::initial_port_keys(2), [5, 6, 7, 8, 9, 10]);
        assert_eq!(KeyModel::first_order_deck_key(2), 11);
        // Tech keys: display 2..=7 + held 1 identified; deck 8..=30 not.
        for k in 1..=7u64 {
            assert!(model.tech_ids.contains_key(&k), "tech_key {k} identified");
        }
        assert!(!model.tech_ids.contains_key(&8));
        assert_eq!(model.max_tech_key, 30);
        // A gap asks for evidence rather than inventing it.
        assert!(model.order_card(13).is_err());
        assert!(model.tech_id(8).is_err());
    }

    #[test]
    fn a_key_with_two_identities_is_a_typed_stop() {
        let mapper = OrderMapper::from_rules(&Rules::standard()).unwrap();
        let mut feed = excerpt_feed();
        // Corrupt the snapshot: give order_key 1 the identity of a different
        // card. Key 1 is ALSO named by the m2 orderDiscarded notification, so
        // the second sighting contradicts the first.
        let gd = &mut feed.snapshots[0].gamedatas;
        let mut row2 = gd["orders"]["2"].clone();
        row2["order_key"] = serde_json::json!("1");
        row2["location"] = serde_json::json!("5");
        gd["orders"]["1"] = row2;
        let err = KeyModel::build(&feed, &mapper).unwrap_err();
        match err {
            ImportError::Sheet { detail } => {
                assert!(detail.contains("two different cards"), "{detail}");
            }
            other => panic!("expected identity-invariant Sheet error, got {other:?}"),
        }
    }

    #[test]
    fn a_deck_key_below_a_visible_key_breaks_the_tail_invariant() {
        let mapper = OrderMapper::from_rules(&Rules::standard()).unwrap();
        let mut feed = excerpt_feed();
        let gd = &mut feed.snapshots[0].gamedatas;
        // Make a deck row claim key 3 (below the visible keys).
        gd["orders"]["13"]["order_key"] = serde_json::json!("3");
        let err = KeyModel::build(&feed, &mapper).unwrap_err();
        match err {
            ImportError::Sheet { detail } => {
                assert!(detail.contains("ascending-tail"), "{detail}");
            }
            other => panic!("expected tail-invariant Sheet error, got {other:?}"),
        }
    }
}
