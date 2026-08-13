//! The name-keyed identity tables (plan T4): areas and technologies.
//!
//! **Areas.** BGA's `tile_id` is the AREA IDENTITY (constant across tables);
//! `tile_key` is the board POSITION (per-game). The table below is the ten
//! rows the H0 corpus evidenced by joining `presidentPlacement.trg_tile_key` ↔
//! `tile_name`, `assistantPlacement.tile_keys` ↔ `tile_names` (positional) and
//! `moveAssistant.from_id` ↔ `from_loc` (inventory §9). The seven area types a
//! 2P board does not use are UNOBSERVED (register U11) — their absence is
//! recorded structurally: [`bga_tile_id_for`] is an exhaustive match with no
//! `_` arm, so a new `AreaType` is a compile error and an unevidenced one is
//! an explicit `None` with its reason beside it, never a silent gap. An
//! unknown incoming id or name is a typed hard error.
//!
//! **Technologies.** `(country, req_gears)` does NOT identify a title (six
//! colliding classes; English/3 holds seven cards), names arrive only in
//! `techAcquisition.tech_name`, and only the SOCKET form carries `tech_id`
//! alongside the name — so the table can only ACCUMULATE (plan §0.4). It is a
//! committed JSON ledger (`tech_ledger.json`), one row per proven identity
//! with its evidence and basis, seeded with the twelve rows the card-mapping
//! doc derives (§4.4: 1 DIRECT, 2 DERIVED, 9 FORCED-by-class) and checked at
//! load against the engine's own card data. A lookup miss is
//! [`TechLookup::Unknown`] — a fact the caller must handle, never a fallback
//! identity (§8 MAY NOT: an unresolved tech at import is a hard error naming
//! the id, its class, and its candidates).
//!
//! **Title vs copy** (mapping doc §4.5): for a duplicated title the corpus can
//! never say WHICH engine `TechCardId` a `tech_id` is; the copies are
//! rules-identical, so the choice is free but must be deterministic and
//! stated. The rule, in two halves: COMMITTED rows in ascending `tech_id`
//! order each take the LOWEST engine `TechCardId` of their title not yet
//! taken (so two loads of the same ledger produce identical assignments), and
//! a LEARNED row takes the lowest copy unclaimed AT LEARN TIME — a resolution
//! once handed out is FROZEN and never reassigned (SHOULD-2, H1 review):
//! H3's live path resolves ids across learn boundaries, and a record written
//! between learns must agree with one written after. Determinism across runs
//! therefore holds for the same committed ledger plus the same learn
//! sequence, which is what an importer run is.

use super::BgaMapError;
use std::collections::BTreeMap;
use yokohama_core::{AreaType, Rules, TechCardId};

/// BGA `tile_id` → engine [`AreaType`] — the ten corpus-evidenced rows.
pub fn area_type_from_tile_id(tile_id: u64) -> Result<AreaType, BgaMapError> {
    match tile_id {
        1 => Ok(AreaType::TeaPlantation),
        2 => Ok(AreaType::Bank),
        3 => Ok(AreaType::Customs),
        4 => Ok(AreaType::FishingGround),
        5 => Ok(AreaType::Church),
        6 => Ok(AreaType::EmploymentAgency),
        7 => Ok(AreaType::SilkMill),
        8 => Ok(AreaType::LaboratoryA),
        9 => Ok(AreaType::PortA),
        10 => Ok(AreaType::CopperMine),
        other => Err(BgaMapError::UnknownTileId { tile_id: other }),
    }
}

/// BGA `tile_name` → engine [`AreaType`] — same evidence, name-keyed (the
/// history feed serves names, never keys).
pub fn area_type_from_tile_name(name: &str) -> Result<AreaType, BgaMapError> {
    match name {
        "Tea Plantation" => Ok(AreaType::TeaPlantation),
        "Bank" => Ok(AreaType::Bank),
        "Customs" => Ok(AreaType::Customs),
        "Fishing Ground" => Ok(AreaType::FishingGround),
        "Church" => Ok(AreaType::Church),
        "Employment Agency" => Ok(AreaType::EmploymentAgency),
        "Silk Mill" => Ok(AreaType::SilkMill),
        "Laboratory" => Ok(AreaType::LaboratoryA),
        "Port" => Ok(AreaType::PortA),
        "Copper Mine" => Ok(AreaType::CopperMine),
        other => Err(BgaMapError::UnknownTileName { name: other.to_string() }),
    }
}

/// The reverse direction, TOTAL over [`AreaType`] by exhaustive match: every
/// variant either carries its evidenced BGA id or an explicit `None` with the
/// reason it has none. No `_` arm — a new area type breaks compilation here
/// rather than slipping through unmapped.
pub fn bga_tile_id_for(area: AreaType) -> Option<u64> {
    match area {
        AreaType::TeaPlantation => Some(1),
        AreaType::Bank => Some(2),
        AreaType::Customs => Some(3),
        AreaType::FishingGround => Some(4),
        AreaType::Church => Some(5),
        AreaType::EmploymentAgency => Some(6),
        AreaType::SilkMill => Some(7),
        AreaType::LaboratoryA => Some(8),
        AreaType::PortA => Some(9),
        AreaType::CopperMine => Some(10),
        // UNOBSERVED (register U11): a 2P board removes these four, and the
        // corpus is one 2P game. Their BGA ids are unknown until a 3P/4P
        // capture exists — None here, hard error on the incoming side.
        AreaType::PortB => None,
        AreaType::LaboratoryB => None,
        AreaType::Chinatown => None,
        AreaType::Canal => None,
    }
}

/// How a ledger row earned its identity (mapping doc §4.4's basis column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerBasis {
    /// One notification carried `tech_name` and `tech_card.tech_id` together.
    Direct,
    /// Inferred from an ownership timeline / elimination over named acquisitions.
    Derived,
    /// Forced by a `(country, gears)` class that is uniform in the engine's set.
    Forced,
}

/// One proven `tech_id → title` identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerRow {
    pub tech_id: u64,
    pub title: String,
    pub country_code: u64,
    pub req_gears: u8,
    pub basis: LedgerBasis,
    pub evidence: String,
}

/// A resolved tech identity: the deterministic engine card plus provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTech {
    pub card: TechCardId,
    pub title: String,
    pub basis: LedgerBasis,
}

/// The lookup result. `Unknown` is a FACT the caller must handle — never a
/// fallback identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TechLookup {
    Known(ResolvedTech),
    Unknown { tech_id: u64 },
}

/// The accumulating tech ledger, checked against engine card data at load.
pub struct TechLedger {
    rows: BTreeMap<u64, LedgerRow>,
    /// Deterministic assignment: tech_id → engine card, computed from rows.
    assignment: BTreeMap<u64, TechCardId>,
    /// Engine facts: title → (ids ascending, country_code, industry_cost).
    engine_titles: BTreeMap<String, (Vec<TechCardId>, u64, u8)>,
}

const LEDGER_JSON: &str = include_str!("tech_ledger.json");

impl TechLedger {
    /// Load the committed ledger and verify every row against `rules`:
    /// unique `tech_id`, a title the engine knows, a `(country, gears)` class
    /// agreeing with the engine card of that title, and no title claimed by
    /// more BGA ids than the engine has physical copies.
    pub fn load(rules: &Rules) -> Result<TechLedger, BgaMapError> {
        Self::from_json(LEDGER_JSON, rules)
    }

    fn from_json(text: &str, rules: &Rules) -> Result<TechLedger, BgaMapError> {
        let v: serde_json::Value =
            serde_json::from_str(text).map_err(|e| BgaMapError::LedgerInvalid { detail: e.to_string() })?;
        let raw_rows = v
            .get("rows")
            .and_then(|r| r.as_array())
            .ok_or_else(|| BgaMapError::LedgerInvalid { detail: "missing `rows` array".to_string() })?;

        let mut engine_titles: BTreeMap<String, (Vec<TechCardId>, u64, u8)> = BTreeMap::new();
        for card in rules.techs.iter() {
            let code = super::orders::engine_country_code(card.country);
            let entry = engine_titles
                .entry(card.name.clone())
                .or_insert_with(|| (Vec::new(), code, card.industry_cost));
            if entry.1 != code || entry.2 != card.industry_cost {
                return Err(BgaMapError::LedgerInvalid {
                    detail: format!("engine copies of `{}` disagree on class", card.name),
                });
            }
            entry.0.push(card.id);
        }
        for (ids, _, _) in engine_titles.values_mut().map(|e| (&mut e.0, (), ())) {
            ids.sort_by_key(|id| id.0);
        }

        let mut ledger = TechLedger {
            rows: BTreeMap::new(),
            assignment: BTreeMap::new(),
            engine_titles,
        };
        for raw in raw_rows {
            let tech_id = super::require_u64(raw, "tech_id", "ledger row")?;
            let title = raw
                .get("title")
                .and_then(|t| t.as_str())
                .ok_or_else(|| BgaMapError::MissingField {
                    context: format!("ledger row tech_id {tech_id}"),
                    field: "title".to_string(),
                })?
                .to_string();
            let country_code = super::require_u64(raw, "country_code", "ledger row")?;
            let req_gears = super::require_u8(raw, "req_gears", "ledger row")?;
            let basis = match raw.get("basis").and_then(|b| b.as_str()) {
                Some("DIRECT") => LedgerBasis::Direct,
                Some("DERIVED") => LedgerBasis::Derived,
                Some("FORCED") => LedgerBasis::Forced,
                other => {
                    return Err(BgaMapError::LedgerInvalid {
                        detail: format!("tech_id {tech_id}: basis {other:?} is not DIRECT|DERIVED|FORCED"),
                    })
                }
            };
            let evidence = raw
                .get("evidence")
                .and_then(|e| e.as_str())
                .unwrap_or("")
                .to_string();
            if ledger.rows.contains_key(&tech_id) {
                return Err(BgaMapError::LedgerInvalid {
                    detail: format!("duplicate tech_id {tech_id}"),
                });
            }
            ledger.check_row(tech_id, &title, country_code, req_gears)?;
            ledger.rows.insert(
                tech_id,
                LedgerRow { tech_id, title, country_code, req_gears, basis, evidence },
            );
        }
        ledger.recompute_assignment()?;
        Ok(ledger)
    }

    /// Class + capacity checks for one row against engine data.
    fn check_row(
        &self,
        tech_id: u64,
        title: &str,
        country_code: u64,
        req_gears: u8,
    ) -> Result<(), BgaMapError> {
        let (ids, engine_code, engine_gears) = self
            .engine_titles
            .get(title)
            .ok_or_else(|| BgaMapError::UnknownTechTitle { title: title.to_string() })?;
        if *engine_code != country_code || *engine_gears != req_gears {
            return Err(BgaMapError::TechClassMismatch {
                tech_id,
                title: title.to_string(),
                detail: format!(
                    "observation says country {country_code} / gears {req_gears}, \
                     engine card is country {engine_code} / gears {engine_gears}"
                ),
            });
        }
        let claimed = self.rows.values().filter(|r| r.title == title).count();
        if claimed + 1 > ids.len() {
            return Err(BgaMapError::TitleCapacityExceeded {
                title: title.to_string(),
                capacity: ids.len(),
            });
        }
        Ok(())
    }

    /// The deterministic title→copy rule: rows ascending by `tech_id` each
    /// take the lowest unclaimed engine id of their title.
    fn recompute_assignment(&mut self) -> Result<(), BgaMapError> {
        self.assignment.clear();
        let mut taken: BTreeMap<String, usize> = BTreeMap::new();
        for (tech_id, row) in &self.rows {
            let (ids, _, _) = self
                .engine_titles
                .get(&row.title)
                .ok_or_else(|| BgaMapError::UnknownTechTitle { title: row.title.clone() })?;
            let n = taken.entry(row.title.clone()).or_insert(0);
            let card = *ids.get(*n).ok_or_else(|| BgaMapError::TitleCapacityExceeded {
                title: row.title.clone(),
                capacity: ids.len(),
            })?;
            *n += 1;
            self.assignment.insert(*tech_id, card);
        }
        Ok(())
    }

    pub fn lookup(&self, tech_id: u64) -> TechLookup {
        match (self.rows.get(&tech_id), self.assignment.get(&tech_id)) {
            (Some(row), Some(card)) => TechLookup::Known(ResolvedTech {
                card: *card,
                title: row.title.clone(),
                basis: row.basis,
            }),
            _ => TechLookup::Unknown { tech_id },
        }
    }

    pub fn rows(&self) -> impl Iterator<Item = &LedgerRow> {
        self.rows.values()
    }

    /// Absorb one `techAcquisition` observation (socket form: `tech_name` +
    /// `tech_card` together). A contradiction with an existing row or with the
    /// engine's class data is a hard error; a consistent new identity appends
    /// (in memory — persisting the grown ledger back to the source file is a
    /// deliberate human step, since the committed ledger is reviewed
    /// evidence, not a cache).
    pub fn learn_from(
        &mut self,
        tech_id: u64,
        tech_name: &str,
        country_code: u64,
        req_gears: u8,
    ) -> Result<(), BgaMapError> {
        if let Some(existing) = self.rows.get(&tech_id) {
            if existing.title != tech_name
                || existing.country_code != country_code
                || existing.req_gears != req_gears
            {
                return Err(BgaMapError::LedgerContradiction {
                    tech_id,
                    detail: format!(
                        "ledger has `{}` (country {}, gears {}), observation says `{}` (country {}, gears {})",
                        existing.title,
                        existing.country_code,
                        existing.req_gears,
                        tech_name,
                        country_code,
                        req_gears
                    ),
                });
            }
            return Ok(());
        }
        self.check_row(tech_id, tech_name, country_code, req_gears)?;
        // FROZEN incremental assignment (SHOULD-2, module doc): the newcomer
        // takes the lowest engine copy of its title not already handed out —
        // never `recompute_assignment`, which reruns the ascending-`tech_id`
        // rule over ALL rows and would steal an existing id's card whenever
        // the learned id sorts below it.
        let (ids, _, _) = self
            .engine_titles
            .get(tech_name)
            .ok_or_else(|| BgaMapError::UnknownTechTitle { title: tech_name.to_string() })?;
        let card = ids
            .iter()
            .copied()
            .find(|id| !self.assignment.values().any(|c| c == id))
            .ok_or_else(|| BgaMapError::TitleCapacityExceeded {
                title: tech_name.to_string(),
                capacity: ids.len(),
            })?;
        self.rows.insert(
            tech_id,
            LedgerRow {
                tech_id,
                title: tech_name.to_string(),
                country_code,
                req_gears,
                basis: LedgerBasis::Direct,
                evidence: "learned live from a socket techAcquisition".to_string(),
            },
        );
        self.assignment.insert(tech_id, card);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ledger() -> TechLedger {
        TechLedger::load(&Rules::standard()).expect("committed ledger is valid")
    }

    #[test]
    fn the_committed_ledger_loads_with_its_fourteen_evidenced_rows() {
        let l = ledger();
        // 12 at the 3.5b close (mapping doc §4.4's resolved table) + the N3
        // manual commit's two DIRECT rows (W2, 2026-08-12): tech_id 5 Stock
        // Market from table 896157897 and tech_id 16 Ball from 896199544.
        assert_eq!(l.rows().count(), 14, "the resolved table after the N3 commit");
        // The one DIRECT row.
        match l.lookup(2) {
            TechLookup::Known(r) => {
                assert_eq!(r.title, "Mining Technology");
                assert_eq!(r.basis, LedgerBasis::Direct);
            }
            other => panic!("tech_id 2 must be known, got {other:?}"),
        }
    }

    #[test]
    fn duplicated_titles_are_assigned_deterministically_and_injectively() {
        // tech_ids 14 and 24 are both Exposition; the lower id takes the
        // lower engine card (mapping doc §4.5: deterministic and stated).
        let l = ledger();
        let (a, b) = match (l.lookup(14), l.lookup(24)) {
            (TechLookup::Known(a), TechLookup::Known(b)) => (a, b),
            other => panic!("both Expositions must resolve, got {other:?}"),
        };
        assert_eq!(a.title, "Exposition");
        assert_eq!(b.title, "Exposition");
        assert_ne!(a.card, b.card, "the two copies must be distinct cards");
        assert!(a.card.0 < b.card.0, "ascending tech_id takes ascending engine id");
    }

    #[test]
    fn an_unledgered_tech_id_is_unknown_never_a_fallback() {
        // tech_id 1 is English/4 — Station or Textile Mill, UNOBSERVED.
        assert_eq!(ledger().lookup(1), TechLookup::Unknown { tech_id: 1 });
    }

    #[test]
    fn learning_a_consistent_new_identity_appends() {
        let mut l = ledger();
        // Station is English (code 5), 4 gears — a legal future observation
        // for tech_id 1.
        l.learn_from(1, "Station", 5, 4).unwrap();
        match l.lookup(1) {
            TechLookup::Known(r) => assert_eq!(r.title, "Station"),
            other => panic!("expected Known, got {other:?}"),
        }
    }

    /// SHOULD-2 (H1 review): a resolution once handed out never changes.
    /// University is a duplicated title with one committed row (tech_id 12,
    /// holding the LOWER engine copy); learning a NEW LOWER tech_id of the
    /// same title must give the newcomer the REMAINING copy, not rerun the
    /// ascending-id rule and steal 12's — H3's live path resolves ids across
    /// learn boundaries, and a record written between learns must agree with
    /// one written after.
    #[test]
    fn learning_never_reassigns_an_already_resolved_identity() {
        let mut l = ledger();
        let before = match l.lookup(12) {
            TechLookup::Known(r) => r.card,
            other => panic!("tech_id 12 must be known, got {other:?}"),
        };
        // English/3 University, at a tech_id BELOW 12. (Was tech_id 5 until
        // the N3 commit shipped 5 as Stock Market; 6 is still unobserved.)
        l.learn_from(6, "University", 5, 3).unwrap();
        let after = match l.lookup(12) {
            TechLookup::Known(r) => r.card,
            other => panic!("tech_id 12 must still be known, got {other:?}"),
        };
        assert_eq!(before, after, "tech_id 12's resolution moved across a learn boundary");
        let learned = match l.lookup(6) {
            TechLookup::Known(r) => r,
            other => panic!("the learned id must resolve, got {other:?}"),
        };
        assert_eq!(learned.title, "University");
        assert_ne!(learned.card, after, "the two copies must stay distinct cards");
    }

    #[test]
    fn learning_a_contradiction_of_an_existing_row_is_a_hard_error() {
        let mut l = ledger();
        let err = l.learn_from(2, "Brickyard", 2, 3).unwrap_err();
        match err {
            BgaMapError::LedgerContradiction { tech_id: 2, .. } => {}
            other => panic!("expected LedgerContradiction, got {other:?}"),
        }
    }

    #[test]
    fn learning_a_class_mismatch_is_a_hard_error() {
        let mut l = ledger();
        // Tram is American/5; an observation claiming Dutch/3 means the data
        // moved. (Fixture id 5→6 at the N3 commit: 5 shipped as Stock Market.)
        let err = l.learn_from(6, "Tram", 4, 3).unwrap_err();
        match err {
            BgaMapError::TechClassMismatch { tech_id: 6, .. } => {}
            other => panic!("expected TechClassMismatch, got {other:?}"),
        }
    }

    #[test]
    fn learning_beyond_a_titles_physical_copies_is_a_hard_error() {
        let mut l = ledger();
        // Language School has two copies; ids 11 and 21 already claim both.
        let err = l.learn_from(99, "Language School", 1, 3).unwrap_err();
        match err {
            BgaMapError::TitleCapacityExceeded { title, capacity: 2 } => {
                assert_eq!(title, "Language School");
            }
            other => panic!("expected TitleCapacityExceeded, got {other:?}"),
        }
    }

    #[test]
    fn the_evidenced_area_rows_round_trip_and_the_match_is_total() {
        for (id, area) in [
            (1, AreaType::TeaPlantation),
            (2, AreaType::Bank),
            (3, AreaType::Customs),
            (4, AreaType::FishingGround),
            (5, AreaType::Church),
            (6, AreaType::EmploymentAgency),
            (7, AreaType::SilkMill),
            (8, AreaType::LaboratoryA),
            (9, AreaType::PortA),
            (10, AreaType::CopperMine),
        ] {
            assert_eq!(area_type_from_tile_id(id).unwrap(), area);
            assert_eq!(bga_tile_id_for(area), Some(id));
        }
        // The unevidenced four are explicit Nones, and incoming unknowns error.
        for area in [AreaType::PortB, AreaType::LaboratoryB, AreaType::Chinatown, AreaType::Canal] {
            assert_eq!(bga_tile_id_for(area), None);
        }
        assert_eq!(
            area_type_from_tile_id(11).unwrap_err(),
            BgaMapError::UnknownTileId { tile_id: 11 }
        );
        assert_eq!(
            area_type_from_tile_name("Chinatown").unwrap_err(),
            BgaMapError::UnknownTileName { name: "Chinatown".to_string() }
        );
    }
}
