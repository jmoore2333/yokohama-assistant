//! Board layout generation for Yokohama.
//!
//! The board is a STAGGERED PYRAMID (rulebook p.3 step 2, owner-confirmed from
//! the physical edition 2026-07-21): brick-offset rows, widest on top, of
//! length 4/3/2/1 (2P), 5/4/3/2 (3P), 6/5/4/3 (4P). Each tile in a shorter row
//! sits in the notch between two tiles of the row above it, so a tile touches
//! up to SIX neighbors (2 lateral + 2 above + 2 below) — not the four of a
//! rectangular lattice. Tiles that only meet at a corner are NOT adjacent.
//! (Jeremy's edition and the PDF print mirror-image orientations of the same
//! pyramid; the adjacency GRAPH is identical, and the graph is what the engine
//! models.)
//!
//! AreaIds are assigned row-major, top row first: row 0 gets ids 0..L0, row 1
//! gets L0..L0+L1, and so on. `grid_position` on each area is `(row, col)`
//! with `col` the index within its row.

use rand::seq::SliceRandom;
use rand::Rng;

use crate::types::{
    AreaConfig, AreaId, AreaType, BoardLayout, BoardLayoutBuilder, BuildingSite, FivePowerEffect,
    GameConfig,
};

/// Bump on ANY change to board topology derivation (adjacency, canal
/// crossings, tile multisets). Participates in `Rules::content_hash`.
///
/// Rev 2 (2026-07-21, Task 1): rectangular lattice → staggered pyramid;
/// Canal crossings re-derived to the any-to-any neighbor-pair model.
pub const BOARD_DERIVATION_VERSION: u32 = 2;

/// Complete set of area tiles for a 4-player game.
///
/// Production areas (8 total - 2 of each type):
/// - 2x Copper Mine
/// - 2x Silk Mill
/// - 2x Tea Plantation
/// - 2x Fishing Ground
///
/// Commercial areas (9 total):
/// - 1x Bank
/// - 1x Port A (always)
/// - 1x Port B (3-4 player only)
/// - 1x Laboratory A (always)
/// - 1x Laboratory B (4 player only)
/// - 1x Employment Agency
/// - 1x Chinatown (3-4 player only)
/// - 1x Church
/// - 1x Customs
///
/// Special areas (1 total):
/// - 1x Canal (3-4 player only)
pub fn full_tile_set() -> Vec<AreaType> {
    vec![
        // Production (2 of each)
        AreaType::CopperMine,
        AreaType::CopperMine,
        AreaType::SilkMill,
        AreaType::SilkMill,
        AreaType::TeaPlantation,
        AreaType::TeaPlantation,
        AreaType::FishingGround,
        AreaType::FishingGround,
        // Commercial
        AreaType::Bank,
        AreaType::PortA,
        AreaType::PortB,
        AreaType::LaboratoryA,
        AreaType::LaboratoryB,
        AreaType::EmploymentAgency,
        AreaType::Chinatown,
        AreaType::Church,
        AreaType::Customs,
        // Special
        AreaType::Canal,
    ]
}

/// Get tiles to use for a specific player count.
///
/// Removal rules (from rulebook):
/// - 2 players: Remove Port B, Lab B, Chinatown, Canal, + 1 of each production type
///   Result: 18 - 4 fixed - 4 production = 10 tiles
/// - 3 players: Remove Lab B, + 1 of each of 3 production types (Copper, Silk, Tea)
///   Result: 18 - 1 fixed - 3 production = 14 tiles
/// - 4 players: Use all 18 tiles
///
/// Note: The KB table shows 12/15/18 but removal rules give 10/14/18.
/// Following the explicit removal rules from the rulebook.
pub fn tiles_for_player_count(player_count: u8, rng: &mut impl Rng) -> Vec<AreaType> {
    let mut tiles = full_tile_set();

    match player_count {
        2 => {
            // Remove fixed tiles
            tiles.retain(|t| !matches!(t,
                AreaType::PortB |
                AreaType::LaboratoryB |
                AreaType::Chinatown |
                AreaType::Canal
            ));

            // Remove 1 of each production type (randomly choose which duplicate)
            remove_one_production_duplicate(&mut tiles, AreaType::CopperMine, rng);
            remove_one_production_duplicate(&mut tiles, AreaType::SilkMill, rng);
            remove_one_production_duplicate(&mut tiles, AreaType::TeaPlantation, rng);
            remove_one_production_duplicate(&mut tiles, AreaType::FishingGround, rng);
        }
        3 => {
            // Remove fixed tiles
            tiles.retain(|t| !matches!(t, AreaType::LaboratoryB));

            // Remove 1 of each of 3 production types (Copper, Silk, Tea - keep both Fishing)
            remove_one_production_duplicate(&mut tiles, AreaType::CopperMine, rng);
            remove_one_production_duplicate(&mut tiles, AreaType::SilkMill, rng);
            remove_one_production_duplicate(&mut tiles, AreaType::TeaPlantation, rng);
        }
        4 => {
            // Use all tiles
        }
        _ => panic!("Invalid player count: {}", player_count),
    }

    tiles
}

/// Expected tile count for each player count.
pub fn expected_tile_count(player_count: u8) -> usize {
    match player_count {
        2 => 10,  // 18 - 4 fixed - 4 production
        3 => 14,  // 18 - 1 fixed - 3 production
        4 => 18,
        _ => panic!("Invalid player count: {}", player_count),
    }
}

/// Remove one instance of a production type that has duplicates.
fn remove_one_production_duplicate(tiles: &mut Vec<AreaType>, area_type: AreaType, rng: &mut impl Rng) {
    let indices: Vec<usize> = tiles.iter()
        .enumerate()
        .filter(|(_, t)| **t == area_type)
        .map(|(i, _)| i)
        .collect();

    if indices.len() >= 2 {
        // Randomly pick which one to remove
        let remove_idx = *indices.choose(rng).unwrap();
        tiles.remove(remove_idx);
    }
}

/// Staggered-pyramid row lengths, widest (top) row first, per player count
/// (rulebook p.3 step 2; Jeremy's edition orientation, owner-confirmed
/// 2026-07-21). Sums to [`expected_tile_count`]. The single source of the
/// board's shape: tile placement, [`staggered_adjacency`], and
/// [`derive_canal_crossings`] all read it.
pub fn row_lengths(player_count: u8) -> Vec<u8> {
    match player_count {
        2 => vec![4, 3, 2, 1], // 10 tiles
        3 => vec![5, 4, 3, 2], // 14 tiles
        4 => vec![6, 5, 4, 3], // 18 tiles
        _ => panic!("Invalid player count: {}", player_count),
    }
}

/// Row-major AreaId for staggered cell `(row, col)`: the summed lengths of all
/// rows above `row`, plus `col`. Inverse of [`area_id_to_cell`].
pub fn cell_to_area_id(row: u8, col: u8, rows: &[u8]) -> AreaId {
    let base: u16 = rows[..row as usize].iter().map(|&l| l as u16).sum();
    AreaId::new((base + col as u16) as u8)
}

/// The `(row, col)` of a row-major AreaId on a board with the given `rows`
/// lengths. Inverse of [`cell_to_area_id`]; used by live-board reconciliation
/// to find the Canal's cell after a re-type.
pub fn area_id_to_cell(id: AreaId, rows: &[u8]) -> (u8, u8) {
    let mut remaining = id.0 as u16;
    for (r, &len) in rows.iter().enumerate() {
        if remaining < len as u16 {
            return (r as u8, remaining as u8);
        }
        remaining -= len as u16;
    }
    panic!("AreaId {} is off the board (rows {:?})", id.0, rows);
}

/// The up-to-six brick-offset neighbor CELLS of `(row, col)` that exist on a
/// pyramid with the given `rows` lengths — the one topology primitive
/// [`staggered_adjacency`] and [`derive_canal_crossings`] both build on.
///
/// Row 0 is widest; each shorter row k+1 is offset half a tile so cell
/// (k+1, j) sits in the notch above between (k, j) and (k, j+1). Reading that
/// relation from the perspective of `(row, col)` (brief, Task 1):
/// - lateral: (row, col-1), (row, col+1)
/// - row above (longer): (row-1, col), (row-1, col+1) — none when row == 0
/// - row below (shorter): (row+1, col-1), (row+1, col)
///
/// Each candidate is emitted only where the cell actually exists, so corner
/// and edge tiles simply have fewer neighbors (a tip tile has as few as two).
fn neighbor_cells(row: u8, col: u8, rows: &[u8]) -> Vec<(u8, u8)> {
    let len = |r: usize| rows.get(r).copied().unwrap_or(0) as usize;
    let (r, c) = (row as usize, col as usize);
    let mut out = Vec::with_capacity(6);
    // lateral
    if c > 0 {
        out.push((row, col - 1));
    }
    if c + 1 < len(r) {
        out.push((row, col + 1));
    }
    // row above (longer than this one; both indices exist by construction, but
    // bounds-check for robustness against a non-canonical `rows`)
    if r > 0 {
        if c < len(r - 1) {
            out.push((row - 1, col));
        }
        if c + 1 < len(r - 1) {
            out.push((row - 1, col + 1));
        }
    }
    // row below (shorter than this one)
    if c > 0 && c - 1 < len(r + 1) {
        out.push((row + 1, col - 1));
    }
    if c < len(r + 1) {
        out.push((row + 1, col));
    }
    out
}

/// Whether cells `a` and `b` are brick-offset neighbors (undirected).
fn cells_adjacent(a: (u8, u8), b: (u8, u8), rows: &[u8]) -> bool {
    neighbor_cells(a.0, a.1, rows).contains(&b)
}

/// Generate a randomized board layout for the given configuration.
pub fn generate_board_layout(
    config: &GameConfig,
    rng: &mut impl Rng,
    building_sites: Vec<BuildingSite>,
    five_power_effects: Vec<FivePowerEffect>,
) -> Result<BoardLayout, BoardGenerationError> {
    let rows = row_lengths(config.player_count);
    let expected_tiles = expected_tile_count(config.player_count);

    // Get and shuffle tiles
    let mut tiles = tiles_for_player_count(config.player_count, rng);
    if tiles.len() != expected_tiles {
        return Err(BoardGenerationError::TileCountMismatch {
            expected: expected_tiles,
            got: tiles.len(),
        });
    }
    tiles.shuffle(rng);

    // Verify we have enough building sites and 5-power tokens
    // (Canal doesn't get either)
    let non_canal_count = tiles.iter().filter(|t| **t != AreaType::Canal).count();
    if building_sites.len() < non_canal_count {
        return Err(BoardGenerationError::InsufficientBuildingSites {
            needed: non_canal_count,
            got: building_sites.len(),
        });
    }
    if five_power_effects.len() < non_canal_count {
        return Err(BoardGenerationError::InsufficientFivePowerTokens {
            needed: non_canal_count,
            got: five_power_effects.len(),
        });
    }

    // Shuffle building sites and 5-power tokens
    let mut building_sites = building_sites;
    let mut five_power_effects = five_power_effects;
    building_sites.shuffle(rng);
    five_power_effects.shuffle(rng);

    // Build the layout
    let mut builder = BoardLayoutBuilder::new();
    let mut site_idx = 0;
    let mut token_idx = 0;
    let mut canal_position: Option<(u8, u8)> = None;

    // Place tiles row by row, top (widest) row first, assigning row-major
    // AreaIds and staggered `(row, col)` grid positions.
    let mut idx = 0usize;
    let mut tiles = tiles.into_iter();
    for (r, &len) in rows.iter().enumerate() {
        for col in 0..len {
            let area_type = tiles.next().expect("tile count validated above");
            let id = AreaId::new(idx as u8);

            builder.add_area(id, AreaConfig {
                area_type,
                grid_position: (r as u8, col),
            });

            if area_type == AreaType::Canal {
                canal_position = Some((r as u8, col));
            }

            // Add building site and 5-power token (Canal is excluded)
            if area_type != AreaType::Canal {
                builder.set_building_site(id, building_sites[site_idx].clone());
                builder.set_five_power_token(id, five_power_effects[token_idx]);
                site_idx += 1;
                token_idx += 1;
            }
            idx += 1;
        }
    }

    // Staggered brick-offset adjacency: the base layer every board sits on.
    for (a, b) in staggered_adjacency(&rows) {
        builder.add_adjacency(a, b);
    }

    // Canal crossings (any-to-any, decision log 2026-07-21): add each tolled
    // transit pair to BOTH adjacency (so the movement BFS can traverse
    // straight across the Canal without stopping on it) and canal_crossings
    // (so crossing it tolls 1 Yen to the bank). `derive_canal_crossings` is
    // the single source of truth, also used by `reconcile_board` when a
    // re-typed board moves the Canal.
    if let Some(pos) = canal_position {
        for (a, b) in derive_canal_crossings(pos, &rows) {
            builder.add_adjacency(a, b);
            builder.add_canal_crossing(a, b);
        }
    }

    builder.build().map_err(BoardGenerationError::ValidationError)
}

/// Undirected brick-offset adjacency edges of the staggered pyramid with the
/// given `rows` lengths, each edge listed once (smaller AreaId first). AreaIds
/// are row-major, top row first — grid positions are fixed for the life of a
/// game, so this is unaffected by re-typing. Callers add each pair
/// bidirectionally. Shared by `generate_board_layout` and `reconcile_board` so
/// a re-typed board's adjacency is identical to generating that tile
/// arrangement directly.
pub fn staggered_adjacency(rows: &[u8]) -> Vec<(AreaId, AreaId)> {
    let mut edges = Vec::new();
    for (r, &len) in rows.iter().enumerate() {
        for col in 0..len {
            let id = cell_to_area_id(r as u8, col, rows);
            for (nr, nc) in neighbor_cells(r as u8, col, rows) {
                let nid = cell_to_area_id(nr, nc, rows);
                if nid.0 > id.0 {
                    edges.push((id, nid)); // emit each undirected edge exactly once
                }
            }
        }
    }
    edges
}

/// Derive the Canal's TOLLED crossing pairs from its grid position on the
/// staggered pyramid.
///
/// Canal transit is ANY-TO-ANY (decision log 2026-07-21, superseding the
/// 2026-07-18/19 colinear-through-lines model, which was reasoned on the old
/// rectangular grid): the Canal connects ANY of its adjacent tiles to ANY
/// other. So the derivation is every unordered pair of the Canal's neighbors.
///
/// A pair already joined by a free brick-offset edge keeps that free edge and
/// gets NO toll marker: the free direct step wins (the owner ruling notes the
/// tolled transit there is legal but strictly dominated, and self-resolves),
/// and the movement toll test [`BoardLayout::is_canal_crossing`] would
/// otherwise toll that free edge unconditionally. Only the pairs the Canal
/// genuinely bridges — neighbors NOT otherwise adjacent — become tolled
/// crossings. Every neighbor pair therefore ends up CONNECTED in `adjacency`
/// (the already-adjacent ones freely, the rest via these tolled crossings),
/// which is exactly "connects any to any, free edge governs". The caller adds
/// each returned pair to both adjacency and `canal_crossings`.
pub fn derive_canal_crossings(canal: (u8, u8), rows: &[u8]) -> Vec<(AreaId, AreaId)> {
    let neighbors = neighbor_cells(canal.0, canal.1, rows);
    let mut out = Vec::new();
    for i in 0..neighbors.len() {
        for j in (i + 1)..neighbors.len() {
            let (a, b) = (neighbors[i], neighbors[j]);
            if !cells_adjacent(a, b, rows) {
                out.push((cell_to_area_id(a.0, a.1, rows), cell_to_area_id(b.0, b.1, rows)));
            }
        }
    }
    out
}

/// Canonical 5-power token pool from the physical game.
///
/// Total: 20 tokens
/// - 10x Import (1 imported good)
/// - 2x Copper (2 copper)
/// - 2x Silk (2 silk)
/// - 2x Fish (3 seafood)
/// - 2x Tea (3 tea)
/// - 2x Yen (3 money)
pub fn canonical_five_power_tokens() -> Vec<FivePowerEffect> {
    let mut tokens = Vec::with_capacity(20);

    // 10x Import (1 imported good)
    for _ in 0..10 {
        tokens.push(FivePowerEffect::ImportedGoods(1));
    }

    // 2x Copper (2 copper)
    for _ in 0..2 {
        tokens.push(FivePowerEffect::Copper(2));
    }

    // 2x Silk (2 silk)
    for _ in 0..2 {
        tokens.push(FivePowerEffect::Silk(2));
    }

    // 2x Fish (3 seafood)
    for _ in 0..2 {
        tokens.push(FivePowerEffect::Seafood(3));
    }

    // 2x Tea (3 tea)
    for _ in 0..2 {
        tokens.push(FivePowerEffect::Tea(3));
    }

    // 2x Yen (3 money)
    for _ in 0..2 {
        tokens.push(FivePowerEffect::Money(3));
    }

    tokens
}

/// Get 5-power tokens needed for a specific player count.
///
/// Token requirements by player count (non-canal areas):
/// - 2P: 10 tokens (10 areas, no canal)
/// - 3P: 13 tokens (14 areas - 1 canal)
/// - 4P: 17 tokens (18 areas - 1 canal)
pub fn five_power_tokens_for_player_count(player_count: u8) -> Vec<FivePowerEffect> {
    let needed = match player_count {
        2 => 10,
        3 => 13,
        4 => 17,
        _ => panic!("Invalid player count: {}", player_count),
    };

    let mut tokens = canonical_five_power_tokens();
    tokens.truncate(needed);
    tokens
}

/// Errors during board generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardGenerationError {
    TileCountMismatch { expected: usize, got: usize },
    InsufficientBuildingSites { needed: usize, got: usize },
    InsufficientFivePowerTokens { needed: usize, got: usize },
    ValidationError(crate::types::BoardLayoutError),
}

impl std::fmt::Display for BoardGenerationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TileCountMismatch { expected, got } => {
                write!(f, "Tile count mismatch: expected {}, got {}", expected, got)
            }
            Self::InsufficientBuildingSites { needed, got } => {
                write!(f, "Insufficient building sites: need {}, got {}", needed, got)
            }
            Self::InsufficientFivePowerTokens { needed, got } => {
                write!(f, "Insufficient 5-power tokens: need {}, got {}", needed, got)
            }
            Self::ValidationError(e) => write!(f, "Board validation error: {}", e),
        }
    }
}

impl std::error::Error for BoardGenerationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn make_test_building_sites(count: usize) -> Vec<BuildingSite> {
        (0..count)
            .map(|i| BuildingSite::new(
                [(i % 4 + 1) as u8, (i % 4 + 2) as u8, (i % 4 + 3) as u8, (i % 4 + 4) as u8],
                (i % 5 + 5) as u8,
            ))
            .collect()
    }

    fn make_test_five_power_tokens(count: usize) -> Vec<FivePowerEffect> {
        (0..count)
            .map(|i| match i % 6 {
                0 => FivePowerEffect::ImportedGoods(1),
                1 => FivePowerEffect::Copper(2),
                2 => FivePowerEffect::Silk(2),
                3 => FivePowerEffect::Seafood(3),
                4 => FivePowerEffect::Tea(3),
                _ => FivePowerEffect::Money(3),
            })
            .collect()
    }

    #[test]
    fn test_full_tile_set_count() {
        let tiles = full_tile_set();
        assert_eq!(tiles.len(), 18, "Full tile set should have 18 tiles");
    }

    #[test]
    fn test_tiles_for_4_player() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let tiles = tiles_for_player_count(4, &mut rng);
        assert_eq!(tiles.len(), 18, "4-player should have 18 tiles");
    }

    #[test]
    fn test_tiles_for_3_player() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let tiles = tiles_for_player_count(3, &mut rng);
        assert_eq!(tiles.len(), 14, "3-player should have 14 tiles (18 - LabB - 3 production)");

        // Should not have Lab B
        assert!(!tiles.contains(&AreaType::LaboratoryB));

        // Should still have Canal, Chinatown, Port B
        assert!(tiles.contains(&AreaType::Canal));
        assert!(tiles.contains(&AreaType::Chinatown));
        assert!(tiles.contains(&AreaType::PortB));

        // Should have 1 of Copper, Silk, Tea (one duplicate removed each)
        assert_eq!(tiles.iter().filter(|t| **t == AreaType::CopperMine).count(), 1);
        assert_eq!(tiles.iter().filter(|t| **t == AreaType::SilkMill).count(), 1);
        assert_eq!(tiles.iter().filter(|t| **t == AreaType::TeaPlantation).count(), 1);
        // Should still have 2 Fishing Grounds
        assert_eq!(tiles.iter().filter(|t| **t == AreaType::FishingGround).count(), 2);
    }

    #[test]
    fn test_tiles_for_2_player() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let tiles = tiles_for_player_count(2, &mut rng);
        assert_eq!(tiles.len(), 10, "2-player should have 10 tiles (18 - 4 fixed - 4 production)");

        // Should not have these
        assert!(!tiles.contains(&AreaType::PortB));
        assert!(!tiles.contains(&AreaType::LaboratoryB));
        assert!(!tiles.contains(&AreaType::Chinatown));
        assert!(!tiles.contains(&AreaType::Canal));

        // Should have exactly 1 of each production type
        assert_eq!(tiles.iter().filter(|t| **t == AreaType::CopperMine).count(), 1);
        assert_eq!(tiles.iter().filter(|t| **t == AreaType::SilkMill).count(), 1);
        assert_eq!(tiles.iter().filter(|t| **t == AreaType::TeaPlantation).count(), 1);
        assert_eq!(tiles.iter().filter(|t| **t == AreaType::FishingGround).count(), 1);
    }

    #[test]
    fn test_row_lengths_are_the_staggered_pyramid() {
        assert_eq!(row_lengths(2), vec![4, 3, 2, 1]); // 10 tiles
        assert_eq!(row_lengths(3), vec![5, 4, 3, 2]); // 14 tiles
        assert_eq!(row_lengths(4), vec![6, 5, 4, 3]); // 18 tiles
    }

    /// The two independent tile-count sources must agree.
    #[test]
    fn row_lengths_sum_matches_expected_tile_count() {
        for pc in [2u8, 3, 4] {
            let sum: usize = row_lengths(pc).iter().map(|&l| l as usize).sum();
            assert_eq!(sum, expected_tile_count(pc), "row_lengths sum vs expected_tile_count at {pc}P");
        }
    }

    #[test]
    fn test_generate_board_layout_4p() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let config = GameConfig::for_player_count(4);

        // 4p has Canal, so 17 non-canal areas need sites/tokens
        let building_sites = make_test_building_sites(17);
        let tokens = make_test_five_power_tokens(17);

        let layout = generate_board_layout(&config, &mut rng, building_sites, tokens)
            .expect("Should generate valid layout");

        assert_eq!(layout.areas.len(), 18);
        assert_eq!(layout.building_sites.len(), 17); // Canal excluded
        assert_eq!(layout.five_power_tokens.len(), 17);
    }

    #[test]
    fn test_generate_board_layout_2p() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let config = GameConfig::for_player_count(2);

        // 2p has no Canal, so all 10 areas need sites/tokens
        let building_sites = make_test_building_sites(10);
        let tokens = make_test_five_power_tokens(10);

        let layout = generate_board_layout(&config, &mut rng, building_sites, tokens)
            .expect("Should generate valid layout");

        assert_eq!(layout.areas.len(), 10);
        assert_eq!(layout.building_sites.len(), 10);
        assert_eq!(layout.five_power_tokens.len(), 10);
    }

    #[test]
    fn test_adjacency_computed() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let config = GameConfig::for_player_count(4);
        let building_sites = make_test_building_sites(17);
        let tokens = make_test_five_power_tokens(17);

        let layout = generate_board_layout(&config, &mut rng, building_sites, tokens)
            .expect("Should generate valid layout");

        // Every area has at least 2 neighbors on the staggered pyramid (even a
        // tip). Canal transit crossings only ADD adjacency, so this floor holds
        // on the full graph; the ≤6 topology bound is pinned separately on the
        // pure brick edges (a canal-neighbor's TOTAL degree can exceed 6).
        for area in layout.areas.keys() {
            let neighbors = layout.adjacent_areas(*area);
            assert!(neighbors.len() >= 2, "Area {:?} should have at least 2 neighbors", area);
        }
    }

    #[test]
    fn staggered_topology_degree_is_at_most_six() {
        // The PURE brick-offset topology (2 lateral + 2 above + 2 below) tops
        // out at six neighbors. (Canal transit crossings are layered on top and
        // legitimately raise a canal-neighbor's total degree past six — that is
        // added connectivity, not a topology defect, so it is excluded here.)
        for pc in [2u8, 3, 4] {
            let rows = row_lengths(pc);
            let mut deg: std::collections::BTreeMap<u8, usize> = std::collections::BTreeMap::new();
            for (a, b) in staggered_adjacency(&rows) {
                *deg.entry(a.0).or_insert(0) += 1;
                *deg.entry(b.0).or_insert(0) += 1;
            }
            for (id, d) in deg {
                assert!(d <= 6, "{pc}P area {id} has staggered degree {d} > 6");
            }
        }
    }

    #[test]
    fn test_canal_crossings_marked() {
        // Any-to-any transit (decision log 2026-07-21): a crossing pair joins
        // two of the Canal's NEIGHBORS across it; the Canal cell itself is
        // never a crossing endpoint (it is never traversed/stopped on), so its
        // own brick-offset neighbors stay ordinary, toll-free adjacency.
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let config = GameConfig::for_player_count(4);
        let building_sites = make_test_building_sites(17);
        let tokens = make_test_five_power_tokens(17);

        let layout = generate_board_layout(&config, &mut rng, building_sites, tokens)
            .expect("Should generate valid layout");

        // Find the Canal area
        let canal_area = layout.areas.iter()
            .find(|(_, c)| c.area_type == AreaType::Canal)
            .map(|(id, _)| *id);

        let canal = canal_area.expect("4p board always has a Canal");
        for neighbor in layout.adjacent_areas(canal) {
            assert!(
                !layout.is_canal_crossing(canal, *neighbor),
                "Canal's own grid-orthogonal neighbor {:?} should NOT be a crossing \
                 (the Canal cell itself is never traversed)", neighbor
            );
        }
        // Every derived crossing pair is bidirectionally adjacent and marked.
        for &(a, b) in &layout.canal_crossings {
            assert!(layout.adjacent_areas(a).contains(&b));
            assert!(layout.adjacent_areas(b).contains(&a));
            assert!(layout.is_canal_crossing(a, b));
        }
    }

    /// Sort each pair's endpoints and sort the resulting list, for
    /// order-independent comparison of a set of undirected crossing pairs.
    fn normalize_pairs(pairs: &[(AreaId, AreaId)]) -> Vec<(AreaId, AreaId)> {
        let mut v: Vec<(AreaId, AreaId)> = pairs
            .iter()
            .map(|&(a, b)| if a.0 <= b.0 { (a, b) } else { (b, a) })
            .collect();
        v.sort_by_key(|&(a, b)| (a.0, b.0));
        v
    }

    /// Seeded 3p layout whose Canal lands at AreaId(3), staggered grid (0,3)
    /// (pinned against `generate_board_layout`'s rng-consumption order:
    /// player-count removals -> tile shuffle -> site shuffle -> token
    /// shuffle — unchanged by the topology switch, so the Canal keeps the same
    /// AreaId). A 3p board always has a Canal.
    fn seeded_3p_layout_with_canal() -> (BoardLayout, GameConfig) {
        let config = GameConfig::for_player_count(3);
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let building_sites = make_test_building_sites(13);
        let tokens = make_test_five_power_tokens(13);
        let layout = generate_board_layout(&config, &mut rng, building_sites, tokens)
            .expect("should generate valid layout");
        (layout, config)
    }

    /// Seeded 2p layout (2p never has a Canal tile at all).
    fn seeded_2p_layout() -> (BoardLayout, GameConfig) {
        let config = GameConfig::for_player_count(2);
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let building_sites = make_test_building_sites(10);
        let tokens = make_test_five_power_tokens(10);
        let layout = generate_board_layout(&config, &mut rng, building_sites, tokens)
            .expect("should generate valid layout");
        (layout, config)
    }

    #[test]
    fn canal_transit_pairs_are_adjacent_and_marked_as_crossings() {
        let (layout, _) = seeded_3p_layout_with_canal();
        for &(a, b) in &layout.canal_crossings {
            assert!(layout.adjacent_areas(a).contains(&b));
            assert!(layout.is_canal_crossing(a, b));
        }
        assert!(!layout.canal_crossings.is_empty()); // 3p always has a Canal

        // Pin: seed 1's 3p Canal lands at AreaId(3), staggered grid (0,3) — a
        // TOP-row cell, so it has no upper neighbors: only its lateral pair
        // (2,4) and its two lower tiles (7,8). Of the six neighbor pairs,
        // three are already brick-offset-adjacent (they keep their free edge
        // and get no toll); the three the Canal genuinely bridges are the
        // tolled crossings (any-to-any, decision log 2026-07-21).
        assert_eq!(
            normalize_pairs(&layout.canal_crossings),
            normalize_pairs(&[
                (AreaId::new(2), AreaId::new(4)), // lateral-left  <-> lateral-right
                (AreaId::new(2), AreaId::new(8)), // lateral-left  <-> lower-right
                (AreaId::new(4), AreaId::new(7)), // lateral-right <-> lower-left
            ]),
        );
    }

    #[test]
    fn interior_canal_yields_nine_transit_pairs() {
        // Seed 5's 3p Canal lands at AreaId(6), staggered grid (1,1) — a fully
        // interior cell with all six neighbors, so C(6,2)=15 neighbor pairs,
        // of which six are already brick-offset-adjacent (free) and NINE are
        // tolled any-to-any transits. (On the old 2x7 lattice this seed put
        // the Canal in a corner with zero crossings — the topology switch is
        // exactly what makes it interior.)
        let config = GameConfig::for_player_count(3);
        let mut rng = ChaCha8Rng::seed_from_u64(5);
        let building_sites = make_test_building_sites(13);
        let tokens = make_test_five_power_tokens(13);
        let layout = generate_board_layout(&config, &mut rng, building_sites, tokens)
            .expect("should generate valid layout");

        let canal = layout.areas.iter()
            .find(|(_, c)| c.area_type == AreaType::Canal)
            .map(|(id, _)| *id);
        assert_eq!(canal, Some(AreaId::new(6)));
        assert_eq!(layout.areas[&AreaId::new(6)].grid_position, (1, 1));

        // Neighbors of (1,1): LL=5 LR=7 UL=1 UR=2 DL=9 DR=10. The nine
        // non-adjacent (tolled) pairs:
        let expected = [
            (AreaId::new(5), AreaId::new(7)),  // LL-LR
            (AreaId::new(5), AreaId::new(2)),  // LL-UR
            (AreaId::new(5), AreaId::new(10)), // LL-DR
            (AreaId::new(7), AreaId::new(1)),  // LR-UL
            (AreaId::new(7), AreaId::new(9)),  // LR-DL
            (AreaId::new(1), AreaId::new(9)),  // UL-DL
            (AreaId::new(1), AreaId::new(10)), // UL-DR
            (AreaId::new(2), AreaId::new(9)),  // UR-DL
            (AreaId::new(2), AreaId::new(10)), // UR-DR
        ];
        assert_eq!(normalize_pairs(&layout.canal_crossings), normalize_pairs(&expected));
    }

    #[test]
    fn four_player_interior_canal_yields_nine_transit_pairs() {
        // Seed 4's 4p Canal lands at AreaId(9), staggered grid (1,3) — also a
        // fully interior cell, exercising all six neighbors on the 4p board.
        let config = GameConfig::for_player_count(4);
        let mut rng = ChaCha8Rng::seed_from_u64(4);
        let building_sites = make_test_building_sites(17);
        let tokens = make_test_five_power_tokens(17);
        let layout = generate_board_layout(&config, &mut rng, building_sites, tokens)
            .expect("should generate valid layout");

        let canal = layout.areas.iter()
            .find(|(_, c)| c.area_type == AreaType::Canal)
            .map(|(id, _)| *id);
        assert_eq!(canal, Some(AreaId::new(9)));
        assert_eq!(layout.areas[&AreaId::new(9)].grid_position, (1, 3));

        // Neighbors of (1,3): LL=8 LR=10 UL=3 UR=4 DL=13 DR=14. The nine
        // non-adjacent (tolled) pairs:
        let expected = [
            (AreaId::new(8), AreaId::new(10)),  // LL-LR (old "horizontal")
            (AreaId::new(8), AreaId::new(4)),   // LL-UR
            (AreaId::new(8), AreaId::new(14)),  // LL-DR
            (AreaId::new(10), AreaId::new(3)),  // LR-UL
            (AreaId::new(10), AreaId::new(13)), // LR-DL
            (AreaId::new(3), AreaId::new(13)),  // UL-DL
            (AreaId::new(3), AreaId::new(14)),  // UL-DR (old "vertical")
            (AreaId::new(4), AreaId::new(13)),  // UR-DL
            (AreaId::new(4), AreaId::new(14)),  // UR-DR
        ];
        assert_eq!(normalize_pairs(&layout.canal_crossings), normalize_pairs(&expected));
    }

    #[test]
    fn two_player_boards_unchanged() {
        let (layout, _) = seeded_2p_layout();
        assert!(layout.canal_crossings.is_empty());
    }

    // ---- Layout-fidelity pins (Task 1): the board is a staggered pyramid,
    // not a rectangular lattice. These assert the corrected topology on the
    // GENERATED board so it can never silently regress. ----

    /// A generated board for `pc`, seeded deterministically (topology is
    /// seed-independent, so any fixed seed exercises the same shape/edges).
    fn generate_for(pc: u8) -> BoardLayout {
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let config = GameConfig::for_player_count(pc);
        let n = expected_tile_count(pc);
        generate_board_layout(&config, &mut rng, make_test_building_sites(n), make_test_five_power_tokens(n))
            .expect("valid layout")
    }

    /// Tiles per row (top row first) realized by the generated grid positions.
    fn row_shape(layout: &BoardLayout) -> Vec<usize> {
        let mut per_row: std::collections::BTreeMap<u8, usize> = std::collections::BTreeMap::new();
        for cfg in layout.areas.values() {
            *per_row.entry(cfg.grid_position.0).or_insert(0) += 1;
        }
        per_row.into_values().collect()
    }

    /// Distinct undirected adjacency edges MINUS the Canal transit crossings
    /// (which are layered onto adjacency on top of the pure staggered
    /// topology) — i.e. the pure brick-offset edge count.
    fn brick_edge_count(layout: &BoardLayout) -> usize {
        let mut edges = std::collections::BTreeSet::new();
        for (a, ns) in &layout.adjacency {
            for n in ns {
                edges.insert(if a.0 <= n.0 { (a.0, n.0) } else { (n.0, a.0) });
            }
        }
        edges.len() - layout.canal_crossings.len()
    }

    /// The grid id at a given `(row, col)` on the generated board.
    fn id_at(layout: &BoardLayout, rc: (u8, u8)) -> AreaId {
        *layout.areas.iter().find(|(_, c)| c.grid_position == rc).map(|(id, _)| id).expect("cell exists")
    }

    #[test]
    fn layout_fidelity_row_shape_is_the_staggered_pyramid() {
        assert_eq!(row_shape(&generate_for(2)), vec![4, 3, 2, 1]);
        assert_eq!(row_shape(&generate_for(3)), vec![5, 4, 3, 2]);
        assert_eq!(row_shape(&generate_for(4)), vec![6, 5, 4, 3]);
    }

    #[test]
    fn layout_fidelity_undirected_edge_count_matches_the_pyramid() {
        assert_eq!(brick_edge_count(&generate_for(2)), 18);
        assert_eq!(brick_edge_count(&generate_for(3)), 28);
        assert_eq!(brick_edge_count(&generate_for(4)), 38);
    }

    /// Defining brick-offset relation: a tile in a shorter row sits in the
    /// notch above and touches BOTH tiles of the row above it. The old 3x6
    /// lattice put grid (1,0) under only (0,0), missing (0,1).
    #[test]
    fn staggered_tile_touches_both_notch_neighbors_above() {
        let layout = generate_for(4);
        let t10 = id_at(&layout, (1, 0));
        let (t00, t01) = (id_at(&layout, (0, 0)), id_at(&layout, (0, 1)));
        let ns = layout.adjacent_areas(t10);
        assert!(ns.contains(&t00), "(1,0) must touch (0,0)");
        assert!(ns.contains(&t01), "(1,0) must touch (0,1) — the notch neighbor the lattice missed");
    }

    #[test]
    fn test_deterministic_with_same_seed() {
        let config = GameConfig::for_player_count(4);
        let building_sites = make_test_building_sites(17);
        let tokens = make_test_five_power_tokens(17);

        let mut rng1 = ChaCha8Rng::seed_from_u64(12345);
        let layout1 = generate_board_layout(&config, &mut rng1, building_sites.clone(), tokens.clone())
            .expect("Should generate valid layout");

        let mut rng2 = ChaCha8Rng::seed_from_u64(12345);
        let layout2 = generate_board_layout(&config, &mut rng2, building_sites, tokens)
            .expect("Should generate valid layout");

        // Same seed should produce same layout
        assert_eq!(layout1.areas, layout2.areas);
    }

    #[test]
    fn test_canonical_five_power_tokens_count() {
        let tokens = canonical_five_power_tokens();
        // Itemized count: 10 import + 2 each of (copper, silk, fish, tea, yen) = 20
        assert_eq!(tokens.len(), 20, "Physical game has 20 5-power tokens");
    }

    #[test]
    fn test_canonical_five_power_tokens_distribution() {
        let tokens = canonical_five_power_tokens();

        // Count each type
        let import_count = tokens.iter()
            .filter(|t| matches!(t, FivePowerEffect::ImportedGoods(1)))
            .count();
        let copper_count = tokens.iter()
            .filter(|t| matches!(t, FivePowerEffect::Copper(2)))
            .count();
        let silk_count = tokens.iter()
            .filter(|t| matches!(t, FivePowerEffect::Silk(2)))
            .count();
        let fish_count = tokens.iter()
            .filter(|t| matches!(t, FivePowerEffect::Seafood(3)))
            .count();
        let tea_count = tokens.iter()
            .filter(|t| matches!(t, FivePowerEffect::Tea(3)))
            .count();
        let money_count = tokens.iter()
            .filter(|t| matches!(t, FivePowerEffect::Money(3)))
            .count();

        assert_eq!(import_count, 10, "10x Import tokens");
        assert_eq!(copper_count, 2, "2x Copper tokens");
        assert_eq!(silk_count, 2, "2x Silk tokens");
        assert_eq!(fish_count, 2, "2x Fish tokens");
        assert_eq!(tea_count, 2, "2x Tea tokens");
        assert_eq!(money_count, 2, "2x Money tokens");
    }

    #[test]
    fn test_five_power_tokens_for_player_count() {
        assert_eq!(five_power_tokens_for_player_count(2).len(), 10);
        assert_eq!(five_power_tokens_for_player_count(3).len(), 13);
        assert_eq!(five_power_tokens_for_player_count(4).len(), 17);
    }

    #[test]
    fn test_different_seeds_produce_different_layouts() {
        let config = GameConfig::for_player_count(4);
        let building_sites = make_test_building_sites(17);
        let tokens = make_test_five_power_tokens(17);

        let mut rng1 = ChaCha8Rng::seed_from_u64(111);
        let layout1 = generate_board_layout(&config, &mut rng1, building_sites.clone(), tokens.clone())
            .expect("Should generate valid layout");

        let mut rng2 = ChaCha8Rng::seed_from_u64(222);
        let layout2 = generate_board_layout(&config, &mut rng2, building_sites, tokens)
            .expect("Should generate valid layout");

        // Different seeds should (very likely) produce different layouts
        // Check that at least one position differs
        let different = layout1.areas.iter().any(|(id, config1)| {
            layout2.areas.get(id).map(|config2| config1 != config2).unwrap_or(true)
        });
        assert!(different, "Different seeds should produce different layouts");
    }
}
