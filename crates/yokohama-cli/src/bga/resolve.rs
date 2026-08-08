//! Setup RESOLUTION (H2): the corpus-evidence passes that turn H1's honest
//! `Absent` sheet fields into a Session-buildable setup — census §1/§2.
//!
//! Three evidence classes come out, and the distinction is load-bearing:
//!
//! - **EVIDENCED** — derived from observations (the key model's initial
//!   displays, token faces joined through the shadow president fold,
//!   ledger/class-resolved tech identities). Carried as normal provenance.
//! - **HYPOTHESIS** — a claimed achievement whose BGA row matches two engine
//!   types. The REPLAY decides: the import runs once per candidate and keeps
//!   the assignment(s) that replay clean. More than one survivor is an error,
//!   not a coin flip.
//! - **ASSUMPTION** — a value the corpus provably never exercises (an
//!   unclaimed ambiguous achievement, an untaken token face, an unbuilt
//!   tile's site, an unacquired ambiguous tech title). Filled with a
//!   deterministic canonical pick, DECLARED on the resolution and in the
//!   acceptance report, and verified inert by the replay reaching the end
//!   without touching it. Never silent.

use super::facts::{Fact, FactStream};
use super::keys::KeyModel;
use super::names::{TechLedger, TechLookup};
use super::setup::{achievement_candidates, Evidence, SetupAnswers};
use super::ImportError;
use std::collections::{BTreeMap, BTreeSet};
use yokohama_core::{
    AchievementType, AreaType, FivePowerEffect, Rules, SpaceReward, TechCardId,
};

/// One declared assumption (census §2's table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assumption {
    pub what: String,
    pub candidates: String,
    pub picked: String,
    pub why_inert: String,
}

/// One replay-decidable hypothesis: a claimed achievement category with two
/// candidate types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchHypothesis {
    pub category: usize,
    pub candidates: Vec<AchievementType>,
}

/// The resolved setup: a fully-filled answer sheet (modulo hypotheses, which
/// the import loop substitutes per candidate), plus the declarations.
pub struct Resolved {
    pub answers: SetupAnswers,
    pub assumptions: Vec<Assumption>,
    pub hypotheses: Vec<AchHypothesis>,
    pub key_model: KeyModel,
    /// tech_key → engine card for every key that could be resolved.
    pub tech_cards: BTreeMap<u64, TechCardId>,
}

/// Resolve tech identity for a BGA tech_id: ledger first, then the
/// class-unique rule, then a declared assumption (lowest engine copy among
/// class candidates). Returns (card, assumption?).
fn resolve_tech_id(
    tech_id: u64,
    key_model: &KeyModel,
    ledger: &TechLedger,
    rules: &Rules,
    used: &mut BTreeSet<u8>,
) -> Result<(TechCardId, Option<Assumption>), ImportError> {
    if let TechLookup::Known(r) = ledger.lookup(tech_id) {
        used.insert(r.card.0);
        return Ok((r.card, None));
    }
    let (country, gears) = key_model.tech_classes.get(&tech_id).copied().ok_or_else(|| {
        ImportError::Sheet {
            detail: format!(
                "tech_id {tech_id} has no ledger row and no observed (country, gears) class — \
                 identity unresolvable"
            ),
        }
    })?;
    // Engine cards in this class, excluding copies already used. SORTED by
    // card id, because `TechRegistry` is a `HashMap` whose iteration order is
    // process-seeded (the same fact wizard.rs pins for its tile roster) — an
    // unsorted `.first()` here picked a PROCESS-RANDOM card for every
    // assumption-resolved class, which is what made two imports of one capture
    // disagree on a lab reveal's identity (caught by H3's follow-vs-batch
    // byte-identity pin). The sort is also what makes the documented rule
    // literal: "the LOWEST engine copy among class candidates".
    let mut class_cards: Vec<&yokohama_core::TechCard> = rules
        .techs
        .iter()
        .filter(|c| {
            super::orders::engine_country_code(c.country) == country
                && c.industry_cost == gears
                && !used.contains(&c.id.0)
        })
        .collect();
    class_cards.sort_by_key(|c| c.id.0);
    let titles: BTreeSet<&str> = class_cards.iter().map(|c| c.name.as_str()).collect();
    match class_cards.first() {
        None => Err(ImportError::Sheet {
            detail: format!(
                "tech_id {tech_id}: class (country {country}, gears {gears}) has no unclaimed \
                 engine card — class data disagrees"
            ),
        }),
        Some(first) => {
            let card = first.id;
            used.insert(card.0);
            if titles.len() == 1 {
                // Class-unique title: forced, evidence-grade.
                Ok((card, None))
            } else {
                Ok((
                    card,
                    Some(Assumption {
                        what: format!("tech_id {tech_id} title"),
                        candidates: titles.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" | "),
                        picked: first.name.clone(),
                        why_inert: "never acquired by a player in this capture — identity only \
                                    names a display card"
                            .to_string(),
                    }),
                ))
            }
        }
    }
}

/// The shadow president fold: at each `PowerToken` fact, the acting seat's
/// president tile is the destination of that seat's latest `President` fact
/// (a mercenary activation never moves the president; a `ReturnPresident`
/// clears it). This is fact-level bookkeeping, no engine state involved.
fn power_token_tiles(stream: &FactStream) -> Result<Vec<(AreaType, FivePowerEffect)>, ImportError> {
    let mut president: BTreeMap<u8, AreaType> = BTreeMap::new();
    let mut out = Vec::new();
    for tf in &stream.facts {
        match &tf.fact {
            Fact::President { seat, dest, .. } => {
                president.insert(*seat, *dest);
            }
            Fact::ReturnPresident { seat } => {
                president.remove(seat);
            }
            Fact::PowerToken { seat, face } => {
                let tile = president.get(seat).copied().ok_or_else(|| ImportError::Sheet {
                    detail: format!(
                        "move {}: powerBonus with no shadow president location for seat {seat}",
                        tf.move_id
                    ),
                })?;
                out.push((tile, *face));
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Resolve the 5-Power token face per board cell. Direct evidence: the taken
/// tiles' faces (shadow fold). Untaken tiles share a face with a taken tile
/// of the same BGA code where possible; a code never taken is a declared
/// assumption constrained by the canonical pool's remaining multiset.
fn resolve_tokens(
    answers: &SetupAnswers,
    stream: &FactStream,
    feed: &yokohama_bridge::feed::CollapsedFeed,
    assumptions: &mut Vec<Assumption>,
) -> Result<Vec<Option<FivePowerEffect>>, ImportError> {
    // Tile codes per AreaType, from the earliest snapshot (all tokens present).
    let snap = feed.earliest_snapshot().ok_or(ImportError::NoSnapshot)?;
    let tiles = snap.gamedatas.get("tiles").and_then(|t| t.as_object()).ok_or_else(|| {
        ImportError::Sheet { detail: "gamedatas.tiles missing for token resolution".to_string() }
    })?;
    let mut code_by_area: Vec<(AreaType, u64)> = Vec::new();
    for row in tiles.values() {
        let tile_id = super::require_u64(row, "tile_id", "tile row").map_err(ImportError::Map)?;
        let code = super::require_u64(row, "power_token", "tile row").map_err(ImportError::Map)?;
        let area = super::names::area_type_from_tile_id(tile_id).map_err(ImportError::Map)?;
        code_by_area.push((area, code));
    }
    let code_of = |area: AreaType| code_by_area.iter().find(|(a, _)| *a == area).map(|(_, c)| *c);
    // Direct joins: taken tile → face; face consistency per code is checked.
    let mut face_by_code: BTreeMap<u64, FivePowerEffect> = BTreeMap::new();
    for (tile, face) in power_token_tiles(stream)? {
        let code = code_of(tile).ok_or_else(|| ImportError::Sheet {
            detail: format!("power token taken on {tile:?}, which has no snapshot tile row"),
        })?;
        if code == 0 {
            return Err(ImportError::Sheet {
                detail: format!("power token taken on {tile:?} whose snapshot code is 0"),
            });
        }
        if let Some(prev) = face_by_code.insert(code, face) {
            if prev != face {
                return Err(ImportError::Sheet {
                    detail: format!(
                        "power_token code {code} observed as two different faces \
                         ({prev:?} then {face:?}) — the code table is inconsistent"
                    ),
                });
            }
        }
    }
    // Board demand per code, and the canonical pool as a multiset.
    let board = answers.board_tiles.value().ok_or_else(|| ImportError::Sheet {
        detail: "token resolution needs the evidenced board tiles".to_string(),
    })?;
    let mut code_count: BTreeMap<u64, u8> = BTreeMap::new();
    for area in board {
        if *area == AreaType::Canal {
            continue;
        }
        let code = code_of(*area).ok_or_else(|| ImportError::Sheet {
            detail: format!("no snapshot tile row for board area {area:?}"),
        })?;
        *code_count.entry(code).or_insert(0) += 1;
    }
    let mut pool: Vec<(FivePowerEffect, u8)> = Vec::new();
    for face in yokohama_core::canonical_five_power_tokens() {
        match pool.iter_mut().find(|(f, _)| *f == face) {
            Some((_, n)) => *n += 1,
            None => pool.push((face, 1)),
        }
    }
    let take = |pool: &mut Vec<(FivePowerEffect, u8)>, face: FivePowerEffect, n: u8| -> bool {
        match pool.iter_mut().find(|(f, c)| *f == face && *c >= n) {
            Some((_, c)) => {
                *c -= n;
                true
            }
            None => false,
        }
    };
    // Mapped codes consume their board copies first.
    let mut face_of_code: BTreeMap<u64, FivePowerEffect> = BTreeMap::new();
    for (code, n) in &code_count {
        if let Some(face) = face_by_code.get(code) {
            if !take(&mut pool, *face, *n) {
                return Err(ImportError::Sheet {
                    detail: format!(
                        "code {code} needs {n} copies of {face:?} but the canonical pool cannot \
                         supply them — token resolution inconsistent"
                    ),
                });
            }
            face_of_code.insert(*code, *face);
        }
    }
    // Unmapped codes: distinct codes are distinct printed faces, so the pick
    // must be a face no other code uses, with enough pool copies. The
    // candidate list is declared; the pick is the first candidate in
    // canonical pool order (deterministic).
    // Descending demand, so a high-multiplicity code (only Import can cover
    // five board copies) claims its face before a single-copy code does.
    let mut unmapped: Vec<(u64, u8)> = code_count
        .iter()
        .filter(|(c, _)| !face_of_code.contains_key(c))
        .map(|(c, n)| (*c, *n))
        .collect();
    unmapped.sort_by_key(|(c, n)| (std::cmp::Reverse(*n), *c));
    for (code, n) in &unmapped {
        let candidates: Vec<FivePowerEffect> = pool
            .iter()
            .filter(|(f, c)| *c >= *n && !face_of_code.values().any(|used| used == f))
            .map(|(f, _)| *f)
            .collect();
        let pick = *candidates.first().ok_or_else(|| ImportError::Sheet {
            detail: format!("no pool face can cover code {code} (needs {n} copies)"),
        })?;
        assert!(take(&mut pool, pick, *n), "candidate filter guaranteed supply");
        assumptions.push(Assumption {
            what: format!("5-Power token face for BGA code {code} ({n} tile(s))"),
            candidates: candidates.iter().map(|f| format!("{f:?}")).collect::<Vec<_>>().join(" | "),
            picked: format!("{pick:?}"),
            why_inert: "no token of this code was ever taken in the capture — its face grants \
                        nothing and scores nothing"
                .to_string(),
        });
        face_of_code.insert(*code, pick);
    }
    let mut out = Vec::with_capacity(board.len());
    for area in board {
        if *area == AreaType::Canal {
            out.push(None);
            continue;
        }
        let code = code_of(*area).expect("checked above");
        out.push(Some(face_of_code[&code]));
    }
    Ok(out)
}

/// Resolve the building-site assignment: constrained cells (built tiles whose
/// granted reward names candidate sites) take the lowest matching unused
/// index; free cells take the lowest unused index. Multi-candidate picks are
/// declared.
fn resolve_sites(
    answers: &SetupAnswers,
    stream: &FactStream,
    rules: &Rules,
    assumptions: &mut Vec<Assumption>,
) -> Result<Vec<Option<u8>>, ImportError> {
    let board = answers.board_tiles.value().ok_or_else(|| ImportError::Sheet {
        detail: "site resolution needs the evidenced board tiles".to_string(),
    })?;
    // Built tiles and their observed rewards.
    let mut built: Vec<(AreaType, SpaceReward)> = Vec::new();
    for tf in &stream.facts {
        if let Fact::BuildShop { tile, reward, .. } = &tf.fact {
            match built.iter().find(|(a, _)| a == tile) {
                Some((_, prev)) if prev != reward => {
                    return Err(ImportError::Sheet {
                        detail: format!(
                            "two builds on {tile:?} with different rewards ({prev:?} vs {reward:?}) \
                             — multi-build site constraint not implemented; extend resolve_sites"
                        ),
                    });
                }
                Some(_) => {}
                None => built.push((*tile, *reward)),
            }
        }
    }
    let candidates_for = |reward: &SpaceReward| -> Vec<u8> {
        rules
            .building_sites
            .iter()
            .enumerate()
            .filter(|(_, site)| site.spaces.iter().any(|sp| sp == reward))
            .map(|(i, _)| i as u8)
            .collect()
    };
    let mut used: BTreeSet<u8> = BTreeSet::new();
    let mut out: Vec<Option<u8>> = vec![None; board.len()];
    // Constrained cells first.
    for (i, area) in board.iter().enumerate() {
        if let Some((_, reward)) = built.iter().find(|(a, _)| a == area) {
            let cands: Vec<u8> = candidates_for(reward).into_iter().filter(|c| !used.contains(c)).collect();
            let pick = *cands.first().ok_or_else(|| ImportError::Sheet {
                detail: format!(
                    "no engine building site carries a space rewarding {reward:?} — the build on \
                     {area:?} is unexplainable (drift STOP)"
                ),
            })?;
            used.insert(pick);
            if cands.len() > 1 {
                assumptions.push(Assumption {
                    what: format!("building site on {area:?}"),
                    candidates: cands.iter().map(|c| format!("site index {c}")).collect::<Vec<_>>().join(" | "),
                    picked: format!("site index {pick}"),
                    why_inert: format!(
                        "constrained by the observed build reward {reward:?}; the candidates \
                         differ only in spaces never built in this capture"
                    ),
                });
            }
            out[i] = Some(pick);
        }
    }
    // Free cells.
    for (i, area) in board.iter().enumerate() {
        if out[i].is_some() || *area == AreaType::Canal {
            continue;
        }
        let pick = (0..rules.building_sites.len() as u8)
            .find(|c| !used.contains(c))
            .ok_or_else(|| ImportError::Sheet {
                detail: "site registry exhausted".to_string(),
            })?;
        used.insert(pick);
        assumptions.push(Assumption {
            what: format!("building site on {area:?}"),
            candidates: "any unassigned site index".to_string(),
            picked: format!("site index {pick}"),
            why_inert: "no building was ever placed on this tile in the capture".to_string(),
        });
        out[i] = Some(pick);
    }
    Ok(out)
}

/// Run every resolution pass over the H1 sheet.
pub fn resolve(
    mut answers: SetupAnswers,
    feed: &yokohama_bridge::feed::CollapsedFeed,
    stream: &FactStream,
    rules: &Rules,
) -> Result<Resolved, ImportError> {
    let mapper = super::orders::OrderMapper::from_rules(rules)?;
    let key_model = KeyModel::build(feed, &mapper)?;
    let mut ledger = TechLedger::load(rules)?;
    let mut assumptions = Vec::new();
    let mut hypotheses = Vec::new();

    // Learn every socket techAcquisition into the ledger first (id+name+class
    // together — the DIRECT basis).
    for tf in &stream.facts {
        if let Fact::LabTake { tech_name, tech_id_key: Some((tech_id, _)), .. } = &tf.fact {
            if let Some((country, gears)) = key_model.tech_classes.get(tech_id) {
                ledger
                    .learn_from(*tech_id, tech_name, *country, *gears)
                    .map_err(ImportError::Map)?;
            }
        }
    }

    // --- initial Port A (key arithmetic, census §1) -------------------------
    if answers.player_count != 2 {
        return Err(ImportError::Sheet {
            detail: format!(
                "{}P key-layout is unobserved (census scope: one 2P table) — resolution refuses",
                answers.player_count
            ),
        });
    }
    let port_keys = KeyModel::initial_port_keys(answers.player_count);
    let mut port = [yokohama_core::OrderCardId(0); 6];
    for (i, k) in port_keys.iter().enumerate() {
        port[i] = key_model.order_card(*k)?;
    }
    answers.port_a = Evidence::Evidenced {
        value: port,
        provenance: format!(
            "key model: initial Port A = order_keys {port_keys:?} (deal-order slot keys; census §1)"
        ),
    };

    // --- initial Lab A ------------------------------------------------------
    let mut tech_cards: BTreeMap<u64, TechCardId> = BTreeMap::new();
    let mut used_tech_ids: BTreeSet<u8> = BTreeSet::new();
    // Resolve committed/learned ledger assignments first so copies stay stable.
    let all_tech_ids: BTreeSet<u64> = key_model.tech_ids.values().copied().collect();
    for tech_id in &all_tech_ids {
        if let TechLookup::Known(r) = ledger.lookup(*tech_id) {
            used_tech_ids.insert(r.card.0);
        }
    }
    for (key, tech_id) in &key_model.tech_ids {
        let (card, assumption) =
            resolve_tech_id(*tech_id, &key_model, &ledger, rules, &mut used_tech_ids)?;
        if let Some(a) = assumption {
            assumptions.push(a);
        }
        tech_cards.insert(*key, card);
    }
    let mut lab = [TechCardId(0); 6];
    for (i, k) in (1..=6u64).enumerate() {
        lab[i] = *tech_cards.get(&k).ok_or_else(|| ImportError::Sheet {
            detail: format!("initial Lab A tech_key {k} was never observed — identity unknown"),
        })?;
    }
    answers.lab_a = Evidence::Evidenced {
        value: lab,
        provenance: "key model: initial Lab A = tech_keys 1..=6 through the ledger/class \
                     resolution (census §1)"
            .to_string(),
    };

    // --- achievements -------------------------------------------------------
    let snap = feed.earliest_snapshot().ok_or(ImportError::NoSnapshot)?;
    let ach_map = snap
        .gamedatas
        .get("achievements")
        .and_then(|a| a.as_object())
        .ok_or_else(|| ImportError::Sheet { detail: "gamedatas.achievements missing".to_string() })?;
    let claimed: BTreeSet<usize> = stream
        .facts
        .iter()
        .filter_map(|tf| match tf.fact {
            Fact::AchievementClaimed { category, .. } => Some(category),
            _ => None,
        })
        .collect();
    for row in ach_map.values() {
        let (class, candidates) = achievement_candidates(row)?;
        let category = (class - 1) as usize;
        match candidates.as_slice() {
            [] => {
                return Err(ImportError::Sheet {
                    detail: format!("achievement class {class} matches no engine type — drift STOP"),
                })
            }
            [one] => {
                answers.achievements[category] = Evidence::Evidenced {
                    value: *one,
                    provenance: format!("gamedatas achievement class {class}, unique vector match"),
                };
            }
            several if claimed.contains(&category) => {
                hypotheses.push(AchHypothesis { category, candidates: several.to_vec() });
                // Placeholder: the import loop substitutes each candidate.
                answers.achievements[category] = Evidence::Evidenced {
                    value: several[0],
                    provenance: format!(
                        "HYPOTHESIS slot: claimed category {category} with candidates {several:?} — \
                         the replay decides"
                    ),
                };
            }
            several => {
                answers.achievements[category] = Evidence::Evidenced {
                    value: several[0],
                    provenance: format!("declared assumption among {several:?}"),
                };
                assumptions.push(Assumption {
                    what: format!("achievement category {}", ["A", "B", "C"][category]),
                    candidates: format!("{several:?}"),
                    picked: format!("{:?}", several[0]),
                    why_inert: "never claimed in this capture — identity affects only the \
                                unexercised legality surface"
                        .to_string(),
                });
            }
        }
    }

    // --- tokens and sites ---------------------------------------------------
    let tokens = resolve_tokens(&answers, stream, feed, &mut assumptions)?;
    answers.board_tokens = Evidence::Evidenced {
        value: tokens,
        provenance: "powerBonus faces joined through the shadow president fold; untaken codes \
                     declared (census §2)"
            .to_string(),
    };
    let sites = resolve_sites(&answers, stream, rules, &mut assumptions)?;
    answers.board_sites = Evidence::Evidenced {
        value: sites,
        provenance: "build-reward constrained assignment; unbuilt tiles declared (census §2)"
            .to_string(),
    };

    Ok(Resolved { answers, assumptions, hypotheses, key_model, tech_cards })
}

#[cfg(test)]
mod tests {
    use super::*;
    use yokohama_bridge::feed::read_and_collapse;

    fn excerpt() -> (yokohama_bridge::feed::CollapsedFeed, SetupAnswers, FactStream) {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../yokohama-bridge/tests/data/excerpt-895724349.ndjson"
        );
        let text = std::fs::read_to_string(path).expect("committed excerpt exists");
        let feed = read_and_collapse(&text, Some(895_724_349)).expect("excerpt collapses");
        let rules = Rules::standard();
        let answers = super::super::setup::synthesize(&feed, &rules).expect("sheet");
        let seats = answers.seats.clone();
        let mapper = super::super::orders::OrderMapper::from_rules(&rules).unwrap();
        let stream = super::super::facts::extract(
            &feed,
            &|pid| seats.iter().find(|s| s.bga_player_id == pid).map(|s| s.seat),
            &mapper,
        )
        .expect("facts extract");
        (feed, answers, stream)
    }

    #[test]
    fn the_excerpt_resolves_port_and_lab_initial_sixes_from_the_key_model() {
        let (feed, answers, stream) = excerpt();
        let rules = Rules::standard();
        let resolved = resolve(answers, &feed, &stream, &rules).expect("resolves");
        let port = resolved.answers.port_a.value().expect("port evidenced");
        // Keys 5..=10 → BGA order ids 23, 22, 7, 21, 11, 33 → engine cards
        // via the vector mapper (identities pinned by the H1 mapper tests).
        let ids: Vec<u64> = (5..=10).map(|k| resolved.key_model.order_ids[&k]).collect();
        assert_eq!(ids, vec![23, 22, 7, 21, 11, 33]);
        assert_eq!(port.len(), 6);
        let lab = resolved.answers.lab_a.value().expect("lab evidenced");
        assert_eq!(lab.len(), 6);
        // tech_keys 1..=6 are ids 12,4,26,14,2,27 — all in the committed ledger.
        let tids: Vec<u64> = (1..=6).map(|k| resolved.key_model.tech_ids[&k]).collect();
        assert_eq!(tids, vec![12, 4, 26, 14, 2, 27]);
    }

    #[test]
    fn every_assumption_is_declared_with_candidates_and_inertness() {
        let (feed, answers, stream) = excerpt();
        let rules = Rules::standard();
        let resolved = resolve(answers, &feed, &stream, &rules).expect("resolves");
        // The excerpt has no build facts and no powerBonus facts, so every
        // site and every token face is a declared assumption, and no
        // achievement was claimed so all three ambiguous rows are assumptions.
        assert!(
            resolved.assumptions.iter().any(|a| a.what.contains("building site")),
            "sites declared"
        );
        assert!(
            resolved.assumptions.iter().any(|a| a.what.contains("5-Power token")),
            "token faces declared"
        );
        for a in &resolved.assumptions {
            assert!(!a.why_inert.is_empty(), "{:?} must state inertness", a.what);
        }
        assert!(resolved.hypotheses.is_empty(), "no claims in the excerpt window");
    }
}
