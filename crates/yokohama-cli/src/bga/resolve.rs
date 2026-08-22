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

/// True when every observed reward can claim its OWN space on the site — the
/// multiset embedding a multi-build tile demands (two builds never share a
/// space). n ≤ 4 on both sides, so plain backtracking is exact and cheap.
fn rewards_fit_distinct_spaces(
    rewards: &[SpaceReward],
    spaces: &[SpaceReward],
    used: &mut Vec<bool>,
) -> bool {
    let Some((first, rest)) = rewards.split_first() else {
        return true;
    };
    for (i, sp) in spaces.iter().enumerate() {
        if !used[i] && sp == first {
            used[i] = true;
            if rewards_fit_distinct_spaces(rest, spaces, used) {
                return true;
            }
            used[i] = false;
        }
    }
    false
}

/// One built tile awaiting a site card: the cell it occupies, the reward
/// multiset its builds granted, and every site that can host that multiset.
struct ConstrainedCell {
    board_index: usize,
    area: AreaType,
    rewards: Vec<SpaceReward>,
    candidates: Vec<u8>,
}

/// How many candidate placements the assignment search may try before it
/// refuses rather than running unbounded.
///
/// Feasibility here is bipartite matching, where a plain depth-first search is
/// worst-case exponential. The real inputs are tiny — at most one constrained
/// cell per board tile, each with at most 24 candidates, and in every capture
/// measured so far the first descent succeeds outright — so this ceiling is
/// astronomically out of reach in practice. It exists so that a pathological
/// capture STOPS with a typed refusal instead of hanging: an import that cannot
/// decide is an import that says so.
const ASSIGNMENT_SEARCH_BUDGET: u64 = 1_000_000;

/// Assign a distinct site card to every constrained cell, in board order,
/// trying each cell's candidates in ascending index order and BACKTRACKING when
/// a later cell is left with nothing.
///
/// **This strictly generalizes the first-pick walk it replaced.** Both take the
/// lowest available candidate at each cell in the same order, so wherever the
/// old walk succeeded its picks ARE this search's first descent and nothing
/// backtracks — the assignment and the declared assumptions come out identical.
/// The difference is only what happens at a dead end, where the old walk
/// refused and this one reconsiders an earlier cell.
///
/// `chosen` accumulates, per depth, the candidates that were still available at
/// that cell together with the pick made there. Recording availability along
/// the SUCCESSFUL path (rather than at first visit) is what keeps a declared
/// assumption's candidate list honest after a backtrack.
fn assign_sites(
    cells: &[ConstrainedCell],
    depth: usize,
    used: &mut BTreeSet<u8>,
    chosen: &mut Vec<(Vec<u8>, u8)>,
    budget: &mut u64,
) -> Result<bool, ImportError> {
    let Some(cell) = cells.get(depth) else {
        return Ok(true);
    };
    let available: Vec<u8> =
        cell.candidates.iter().copied().filter(|c| !used.contains(c)).collect();
    for &pick in &available {
        if *budget == 0 {
            return Err(ImportError::Sheet {
                detail: format!(
                    "the building-site assignment search hit its {ASSIGNMENT_SEARCH_BUDGET}-step \
                     ceiling over {} built tile(s) without deciding — this capture is not \
                     resolvable within the bound and is refused rather than guessed (drift STOP)",
                    cells.len()
                ),
            });
        }
        *budget -= 1;
        used.insert(pick);
        chosen.push((available.clone(), pick));
        if assign_sites(cells, depth + 1, used, chosen, budget)? {
            return Ok(true);
        }
        chosen.pop();
        used.remove(&pick);
    }
    Ok(false)
}

/// Resolve the building-site assignment: constrained cells (built tiles whose
/// granted rewards name candidate sites) are matched to distinct site cards by
/// the board-order backtracking search above; free cells then take the lowest
/// unused index. Multi-candidate picks are declared. A tile built MORE than
/// once (measured live at table 896157897: Church takes seat 1's copper-1 build
/// at m56 and seat 0's silk-2 build at m87) constrains its site to one hosting
/// the whole reward MULTISET on distinct spaces — which typically NARROWS the
/// candidate set (the live pair names site_005 uniquely, where copper-1 alone
/// matched it plus nothing else but silk-2 alone matched several).
///
/// **Two refusals, and they say DIFFERENT things** — the distinction was worth
/// a bug (BGA table 896199544, the game-3 import). A cell whose own candidate
/// set is empty is unexplainable on its own: no site card in the verified data
/// hosts what was observed there, which is genuine data drift. A cell that runs
/// out only because another cell needs the same card is a CONFLICT between
/// cells, and saying "no site carries these rewards" about it is simply false —
/// one does. Game 3 was the second kind and was refused with the first kind's
/// text for want of backtracking.
fn resolve_sites(
    answers: &SetupAnswers,
    stream: &FactStream,
    rules: &Rules,
    assumptions: &mut Vec<Assumption>,
) -> Result<Vec<Option<u8>>, ImportError> {
    let board = answers.board_tiles.value().ok_or_else(|| ImportError::Sheet {
        detail: "site resolution needs the evidenced board tiles".to_string(),
    })?;
    // Built tiles and their observed reward multisets, one entry per build.
    let mut built: Vec<(AreaType, Vec<SpaceReward>)> = Vec::new();
    for tf in &stream.facts {
        if let Fact::BuildShop { tile, reward, .. } = &tf.fact {
            match built.iter_mut().find(|(a, _)| a == tile) {
                Some((_, rewards)) => rewards.push(*reward),
                None => built.push((*tile, vec![*reward])),
            }
        }
    }
    let candidates_for = |rewards: &[SpaceReward]| -> Vec<u8> {
        rules
            .building_sites
            .iter()
            .enumerate()
            .filter(|(_, site)| {
                let mut used = vec![false; site.spaces.len()];
                rewards_fit_distinct_spaces(rewards, &site.spaces, &mut used)
            })
            .map(|(i, _)| i as u8)
            .collect()
    };
    let mut used: BTreeSet<u8> = BTreeSet::new();
    let mut out: Vec<Option<u8>> = vec![None; board.len()];

    // Constrained cells, in board order, with their candidate sites.
    let mut cells: Vec<ConstrainedCell> = Vec::new();
    for (i, area) in board.iter().enumerate() {
        if let Some((_, rewards)) = built.iter().find(|(a, _)| a == area) {
            let candidates = candidates_for(rewards);
            // Empty ON ITS OWN, before any competition: nothing in the verified
            // site data hosts what was observed here. Genuine drift.
            if candidates.is_empty() {
                return Err(ImportError::Sheet {
                    detail: format!(
                        "no engine building site carries distinct spaces for all observed build \
                         rewards {rewards:?} — the build(s) on {area:?} are unexplainable (drift STOP)"
                    ),
                });
            }
            cells.push(ConstrainedCell {
                board_index: i,
                area: *area,
                rewards: rewards.clone(),
                candidates,
            });
        }
    }

    let mut chosen: Vec<(Vec<u8>, u8)> = Vec::new();
    let mut budget = ASSIGNMENT_SEARCH_BUDGET;
    if !assign_sites(&cells, 0, &mut used, &mut chosen, &mut budget)? {
        // Every cell is individually explainable, so the failure is a conflict
        // BETWEEN cells competing for the same card(s).
        let conflict = cells
            .iter()
            .map(|c| {
                let list =
                    c.candidates.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ");
                format!("{:?} [{}]", c.area, list)
            })
            .collect::<Vec<_>>()
            .join("; ");
        return Err(ImportError::Sheet {
            detail: format!(
                "no consistent assignment of engine building sites to the built tiles exists — \
                 every site hosting one tile's observed build rewards is needed by another \
                 (drift STOP); tiles and the sites that can host them: {conflict}"
            ),
        });
    }

    for (cell, (available, pick)) in cells.iter().zip(chosen.iter()) {
        if available.len() > 1 {
            let (area, rewards) = (cell.area, &cell.rewards);
            assumptions.push(Assumption {
                what: format!("building site on {area:?}"),
                candidates: available
                    .iter()
                    .map(|c| format!("site index {c}"))
                    .collect::<Vec<_>>()
                    .join(" | "),
                picked: format!("site index {pick}"),
                why_inert: format!(
                    "constrained by the observed build reward(s) {rewards:?}; the candidates \
                     differ only in spaces never built in this capture"
                ),
            });
        }
        out[cell.board_index] = Some(*pick);
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

    /// The multi-build site constraint (live gate table 896157897, the
    /// resolve-time STOP that blocked the whole import once the capture grew
    /// past m87): TWO builds land on Church — m56 grants 1 copper (seat 1),
    /// m87 grants 2 silk (seat 0) — so the tile's site must host BOTH rewards
    /// on DISTINCT spaces. In the verified site data exactly one card does
    /// (site_005: TR silk-2, MR copper-1), so the constrained pick is exact
    /// and no assumption is declared for the tile. Before this fix
    /// `resolve_sites` hard-errored on the second distinct reward
    /// ("multi-build site constraint not implemented").
    #[test]
    fn two_builds_on_one_tile_resolve_to_the_site_hosting_both_rewards() {
        let (feed, answers, mut stream) = excerpt();
        let rules = Rules::standard();
        let copper1 = SpaceReward { copper: 1, ..SpaceReward::default() };
        let silk2 = SpaceReward { silk: 2, ..SpaceReward::default() };
        stream.facts.push(super::super::facts::TimedFact {
            move_id: 56,
            kind: "buildShop".to_string(),
            fact: Fact::BuildShop { seat: 1, tile: AreaType::Church, reward: copper1 },
        });
        stream.facts.push(super::super::facts::TimedFact {
            move_id: 87,
            kind: "buildShop".to_string(),
            fact: Fact::BuildShop { seat: 0, tile: AreaType::Church, reward: silk2 },
        });
        let resolved = resolve(answers, &feed, &stream, &rules).expect("multi-build resolves");
        let board = resolved.answers.board_tiles.value().expect("tiles evidenced");
        let sites = resolved.answers.board_sites.value().expect("sites evidenced");
        let idx = board.iter().position(|a| *a == AreaType::Church).expect("Church on board");
        let site = &rules.building_sites[sites[idx].expect("Church site assigned") as usize];
        assert!(site.spaces.contains(&copper1), "site hosts the copper-1 space");
        assert!(site.spaces.contains(&silk2), "site hosts the silk-2 space");
        assert!(
            !resolved.assumptions.iter().any(|a| a.what.contains("building site on Church")),
            "a single-candidate constrained pick is evidence, not an assumption"
        );
    }

    /// The negative half: the multiset constraint demands DISTINCT spaces, so
    /// two builds granting the same reward need two spaces printing it — and
    /// no site card in the verified data duplicates a space reward (copper-1
    /// exists on exactly one space across all 24 sites). Two copper-1 builds
    /// on one tile are therefore unexplainable and must STOP as drift, not
    /// resolve to the single-space site.
    #[test]
    fn a_reward_multiset_no_site_hosts_is_a_drift_stop() {
        let (feed, answers, mut stream) = excerpt();
        let rules = Rules::standard();
        let copper1 = SpaceReward { copper: 1, ..SpaceReward::default() };
        for (mid, seat) in [(56u64, 1u8), (87, 0)] {
            stream.facts.push(super::super::facts::TimedFact {
                move_id: mid,
                kind: "buildShop".to_string(),
                fact: Fact::BuildShop { seat, tile: AreaType::Church, reward: copper1 },
            });
        }
        let Err(err) = resolve(answers, &feed, &stream, &rules) else {
            panic!("an unhostable reward multiset must STOP");
        };
        match err {
            ImportError::Sheet { detail } => {
                assert!(detail.contains("Church"), "{detail}");
                assert!(detail.contains("drift STOP"), "{detail}");
            }
            other => panic!("expected Sheet drift STOP, got {other:?}"),
        }
    }

    /// Game 3's board order (BGA table 896199544, from the `--answers` sheet).
    const GAME3_BOARD: [AreaType; 10] = [
        AreaType::Church,
        AreaType::SilkMill,
        AreaType::LaboratoryA,
        AreaType::Bank,
        AreaType::Customs,
        AreaType::CopperMine,
        AreaType::PortA,
        AreaType::TeaPlantation,
        AreaType::FishingGround,
        AreaType::EmploymentAgency,
    ];

    /// Rebuild `stream` carrying ONLY the given builds, on the given board.
    fn sited(
        board: Vec<AreaType>,
        builds: &[(AreaType, SpaceReward)],
    ) -> (SetupAnswers, FactStream) {
        let (_feed, mut answers, mut stream) = excerpt();
        answers.board_tiles =
            Evidence::Evidenced { value: board, provenance: "test fixture".to_string() };
        stream.facts.retain(|tf| !matches!(tf.fact, Fact::BuildShop { .. }));
        for (i, (tile, reward)) in builds.iter().enumerate() {
            stream.facts.push(super::super::facts::TimedFact {
                move_id: 100 + i as u64,
                kind: "buildShop".to_string(),
                fact: Fact::BuildShop { seat: 0, tile: *tile, reward: *reward },
            });
        }
        (answers, stream)
    }

    fn movement2() -> SpaceReward {
        SpaceReward { movement: 2, ..SpaceReward::default() }
    }
    fn vp4() -> SpaceReward {
        SpaceReward { vp: 4, ..SpaceReward::default() }
    }

    /// **The game-3 regression (BGA table 896199544), and it is an ASSIGNMENT
    /// bug rather than a data drift.** Five builds land in that game, two of
    /// them on FishingGround — 2 movement and 4 VP — so its site must host both
    /// on distinct spaces, and in the verified data exactly ONE card does
    /// (site index 8 / `site_009`: TR vp-4, MR assistants-2). But `{movement:2}`
    /// alone matches sites [8, 9, 10, 11, 12], and the old resolver walked the
    /// board in order taking `cands.first()` with no backtracking — so
    /// CopperMine (board index 5) consumed site 8 before FishingGround (board
    /// index 8) was ever reached, leaving it an empty candidate set and
    /// STOPPING the whole import.
    ///
    /// A consistent global assignment exists the whole time (FishingGround→8,
    /// CopperMine and TeaPlantation→9/10, SilkMill→2), which is what the
    /// backtracking search finds. The most-constrained tile being visited LAST
    /// is exactly the shape greedy cannot handle.
    #[test]
    fn the_most_constrained_tile_may_be_reached_last_and_still_resolve() {
        let rules = Rules::standard();
        let (answers, stream) = sited(
            GAME3_BOARD.to_vec(),
            &[
                (AreaType::SilkMill, vp4()),
                (AreaType::FishingGround, movement2()),
                (AreaType::TeaPlantation, movement2()),
                (AreaType::FishingGround, vp4()),
                (AreaType::CopperMine, movement2()),
            ],
        );
        let mut assumptions = Vec::new();
        let sites = resolve_sites(&answers, &stream, &rules, &mut assumptions)
            .expect("a consistent global assignment exists and must be found");

        let at = |tile: AreaType| {
            let i = GAME3_BOARD.iter().position(|a| *a == tile).expect("tile on board");
            sites[i].expect("constrained tile assigned")
        };
        // The only site hosting movement-2 AND vp-4 on distinct spaces.
        assert_eq!(at(AreaType::FishingGround), 8, "FishingGround must take its unique site");

        // Every constrained tile's site really hosts its observed rewards, and
        // no two cells share a site card.
        for (tile, rewards) in [
            (AreaType::SilkMill, vec![vp4()]),
            (AreaType::TeaPlantation, vec![movement2()]),
            (AreaType::CopperMine, vec![movement2()]),
            (AreaType::FishingGround, vec![movement2(), vp4()]),
        ] {
            let site = &rules.building_sites[at(tile) as usize];
            let mut used = vec![false; site.spaces.len()];
            assert!(
                rewards_fit_distinct_spaces(&rewards, &site.spaces, &mut used),
                "{tile:?} assigned a site that cannot host its builds"
            );
        }
        let assigned: Vec<u8> = sites.iter().flatten().copied().collect();
        let distinct: BTreeSet<u8> = assigned.iter().copied().collect();
        assert_eq!(assigned.len(), distinct.len(), "a site card was assigned to two cells");
    }

    /// The genuine global refusal, and the message must say what is TRUE.
    ///
    /// Two tiles each observe `{movement:2, vp:4}`, so each on its own is
    /// perfectly explainable — site 8 hosts either — but only ONE card in the
    /// data hosts that pair, so no assignment can satisfy both. The old text
    /// ("no engine building site carries distinct spaces for all observed build
    /// rewards") would be false here, exactly as it was false on game 3: a site
    /// DOES carry them. The refusal names the conflict instead.
    #[test]
    fn a_globally_unsatisfiable_assignment_stops_and_says_so() {
        let rules = Rules::standard();
        let (answers, stream) = sited(
            GAME3_BOARD.to_vec(),
            &[
                (AreaType::FishingGround, movement2()),
                (AreaType::FishingGround, vp4()),
                (AreaType::TeaPlantation, movement2()),
                (AreaType::TeaPlantation, vp4()),
            ],
        );
        let mut assumptions = Vec::new();
        let Err(err) = resolve_sites(&answers, &stream, &rules, &mut assumptions) else {
            panic!("two tiles competing for one site cannot both be assigned");
        };
        match err {
            ImportError::Sheet { detail } => {
                assert!(detail.contains("drift STOP"), "{detail}");
                assert!(
                    detail.contains("no consistent assignment"),
                    "the refusal must name the CONFLICT, not claim no site fits: {detail}"
                );
                assert!(detail.contains("FishingGround"), "{detail}");
                assert!(detail.contains("TeaPlantation"), "{detail}");
                assert!(
                    !detail.contains("no engine building site carries"),
                    "a site DOES carry these rewards — the old text would be a falsehood: {detail}"
                );
            }
            other => panic!("expected Sheet drift STOP, got {other:?}"),
        }
    }

    /// The search must not change what greedy already got right: where the old
    /// first-pick walk succeeded, the first descent of the DFS makes the SAME
    /// picks (it never had to backtrack there), so assignments and declared
    /// assumptions are unchanged. Pinned on a shape greedy handled — one
    /// unconstrained multi-candidate tile plus one uniquely-constrained tile.
    #[test]
    fn where_greedy_succeeded_the_search_makes_the_same_picks() {
        let rules = Rules::standard();
        let (answers, stream) = sited(
            GAME3_BOARD.to_vec(),
            &[(AreaType::SilkMill, vp4()), (AreaType::CopperMine, movement2())],
        );
        let mut assumptions = Vec::new();
        let sites = resolve_sites(&answers, &stream, &rules, &mut assumptions).expect("resolves");
        let at = |tile: AreaType| {
            let i = GAME3_BOARD.iter().position(|a| *a == tile).expect("tile on board");
            sites[i].expect("assigned")
        };
        // Greedy's own answer: lowest matching unused index, board order.
        assert_eq!(at(AreaType::SilkMill), 2, "vp-4 candidates are [2,8,18,19,21,22]");
        assert_eq!(at(AreaType::CopperMine), 8, "movement-2 candidates are [8,9,10,11,12]");
        let silk = assumptions
            .iter()
            .find(|a| a.what.contains("building site on SilkMill"))
            .expect("a multi-candidate constrained pick is declared");
        assert_eq!(silk.candidates, "site index 2 | site index 8 | site index 18 | site index 19 | site index 21 | site index 22");
        assert_eq!(silk.picked, "site index 2");
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
