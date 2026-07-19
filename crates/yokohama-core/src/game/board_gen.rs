//! Board layout generation for Yokohama.
//!
//! The board is a 3x6 grid (18 areas for 4 players) with randomized tile placement.
//! Adjacency is determined by grid position (orthogonal neighbors only).

use rand::seq::SliceRandom;
use rand::Rng;

use crate::types::{
    AreaConfig, AreaId, AreaType, BoardLayout, BoardLayoutBuilder, BuildingSite, FivePowerEffect,
    GameConfig,
};

/// Bump on ANY change to board topology derivation (adjacency, canal
/// crossings, tile multisets). Participates in `Rules::content_hash`.
pub const BOARD_DERIVATION_VERSION: u32 = 1;

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

/// Grid dimensions based on player count.
/// Using rectangular grids that fit the tile counts.
pub fn grid_dimensions(player_count: u8) -> (u8, u8) {
    match player_count {
        2 => (2, 5),  // 10 tiles
        3 => (2, 7),  // 14 tiles
        4 => (3, 6),  // 18 tiles
        _ => panic!("Invalid player count: {}", player_count),
    }
}

/// Generate a randomized board layout for the given configuration.
pub fn generate_board_layout(
    config: &GameConfig,
    rng: &mut impl Rng,
    building_sites: Vec<BuildingSite>,
    five_power_effects: Vec<FivePowerEffect>,
) -> Result<BoardLayout, BoardGenerationError> {
    let (rows, cols) = grid_dimensions(config.player_count);
    let expected_tiles = (rows * cols) as usize;

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

    // Place tiles on grid.
    for (idx, area_type) in tiles.into_iter().enumerate() {
        let row = (idx / cols as usize) as u8;
        let col = (idx % cols as usize) as u8;
        let id = AreaId::new(idx as u8);

        builder.add_area(id, AreaConfig {
            area_type,
            grid_position: (row, col),
        });

        if area_type == AreaType::Canal {
            canal_position = Some((row, col));
        }

        // Add building site and 5-power token (Canal is excluded)
        if area_type != AreaType::Canal {
            builder.set_building_site(id, building_sites[site_idx].clone());
            builder.set_five_power_token(id, five_power_effects[token_idx]);
            site_idx += 1;
            token_idx += 1;
        }

    }

    // Grid-orthogonal adjacency: the base layer every board sits on.
    for (a, b) in grid_orthogonal_adjacency(rows, cols) {
        builder.add_adjacency(a, b);
    }

    // Canal crossings: derive the colinear flank pairs (owner-adjudicated
    // geometry, design 2026-07-18 decision log) and add each pair to BOTH
    // adjacency (so the movement BFS can traverse straight across the Canal
    // without stopping on it) and canal_crossings (so crossing it tolls 1
    // Yen). `derive_canal_crossings` is the single source of truth, also
    // used by `reconcile_board` when a re-typed board moves the Canal.
    if let Some(pos) = canal_position {
        for (a, b) in derive_canal_crossings(pos, rows, cols) {
            builder.add_adjacency(a, b);
            builder.add_canal_crossing(a, b);
        }
    }

    builder.build().map_err(BoardGenerationError::ValidationError)
}

/// Grid-orthogonal adjacency edges (right/down neighbors only, each edge
/// listed once) for a `rows x cols` grid whose AreaIds are assigned
/// row-major (`AreaId(r*cols+c)`) - the layout every board uses; grid
/// positions are fixed for the life of a game, so this is unaffected by
/// re-typing. Callers add each pair bidirectionally. Shared by
/// `generate_board_layout` and `reconcile_board` so a re-typed board's
/// adjacency is identical to generating that tile arrangement directly
/// (design 2026-07-18 decision log).
pub fn grid_orthogonal_adjacency(rows: u8, cols: u8) -> Vec<(AreaId, AreaId)> {
    let mut edges = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let id = AreaId::new(row * cols + col);
            if col + 1 < cols {
                edges.push((id, AreaId::new(row * cols + col + 1)));
            }
            if row + 1 < rows {
                edges.push((id, AreaId::new((row + 1) * cols + col)));
            }
        }
    }
    edges
}

/// Derive the Canal's crossing pairs from its grid position (owner-
/// adjudicated geometry, Jeremy 2026-07-18 decision log): the four
/// colinear straight-through lines through the Canal cell - horizontal,
/// vertical, and both diagonals - each included only when BOTH flanking
/// cells exist on this board's grid (e.g. a 2-row 3p grid can only ever
/// yield the horizontal pair; a 3-row 4p grid can yield all four). Bent
/// transits (enter one flank, exit a non-opposite flank) are illegal and
/// structurally impossible to represent with this pairing, so no extra
/// filtering is needed for that.
///
/// AreaIds are row-major over the fixed grid (`AreaId(r*cols+c)`, the same
/// convention `grid_orthogonal_adjacency` uses), so "does this cell exist"
/// reduces to a bounds check - there is exactly one Canal tile per board,
/// and by construction no flanking cell can ever coincide with it.
pub fn derive_canal_crossings(canal: (u8, u8), rows: u8, cols: u8) -> Vec<(AreaId, AreaId)> {
    let (r, c) = (canal.0 as i16, canal.1 as i16);
    let (rows, cols) = (rows as i16, cols as i16);
    let id_at = |r: i16, c: i16| -> Option<AreaId> {
        (r >= 0 && r < rows && c >= 0 && c < cols).then(|| AreaId::new((r * cols + c) as u8))
    };
    let lines = [
        ((r, c - 1), (r, c + 1)),         // horizontal
        ((r - 1, c), (r + 1, c)),         // vertical
        ((r - 1, c - 1), (r + 1, c + 1)), // diagonal "\"
        ((r - 1, c + 1), (r + 1, c - 1)), // diagonal "/"
    ];
    lines
        .into_iter()
        .filter_map(|((r1, c1), (r2, c2))| Some((id_at(r1, c1)?, id_at(r2, c2)?)))
        .collect()
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
    fn test_grid_dimensions() {
        assert_eq!(grid_dimensions(2), (2, 5));  // 10 tiles
        assert_eq!(grid_dimensions(3), (2, 7));  // 14 tiles
        assert_eq!(grid_dimensions(4), (3, 6));  // 18 tiles
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

        // In a 3x6 grid:
        // - Corner areas have 2 neighbors
        // - Edge areas have 3 neighbors
        // - Interior areas have 4 neighbors
        // Total adjacencies: counted bidirectionally, but we store in adjacency map

        // Every area should have at least 2 neighbors
        for area in layout.areas.keys() {
            let neighbors = layout.adjacent_areas(*area);
            assert!(neighbors.len() >= 2, "Area {:?} should have at least 2 neighbors", area);
        }
    }

    #[test]
    fn test_canal_crossings_marked() {
        // Flank-pair semantics (design 2026-07-18 decision log): the Canal
        // cell's OWN grid-orthogonal neighbors are ordinary, toll-free
        // adjacency (the Canal is never itself traversed) - only the
        // FLANKING pairs across it are crossings.
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

    /// Seeded 3p layout whose Canal lands at AreaId(3), grid (0,3) (pinned
    /// against the current `generate_board_layout` rng-consumption order:
    /// player-count removals -> tile shuffle -> site shuffle -> token
    /// shuffle). A 3p board (2x7 grid) always has a Canal.
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
    fn canal_flanking_areas_are_adjacent_and_marked_as_crossings() {
        let (layout, _) = seeded_3p_layout_with_canal();
        for &(a, b) in &layout.canal_crossings {
            assert!(layout.adjacent_areas(a).contains(&b));
            assert!(layout.is_canal_crossing(a, b));
        }
        assert!(!layout.canal_crossings.is_empty()); // 3p always has a Canal

        // Pin: seed 1's 3p Canal lands at AreaId(3), grid (0,3) - a 2-row
        // (2x7) grid only ever yields the horizontal pair (owner-
        // adjudicated geometry, design 2026-07-18 decision log).
        assert_eq!(
            normalize_pairs(&layout.canal_crossings),
            normalize_pairs(&[(AreaId::new(2), AreaId::new(4))]),
        );
    }

    #[test]
    fn corner_canal_has_no_crossings() {
        // Seed 5's 3p Canal lands in a grid corner, (0,6): every one of the
        // four candidate lines runs off the 2x7 grid, so no crossing exists
        // at all - this is the both-cells-exist filter doing its job, not a
        // special case.
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
        assert_eq!(layout.areas[&AreaId::new(6)].grid_position, (0, 6));
        assert!(layout.canal_crossings.is_empty());
    }

    #[test]
    fn four_player_canal_yields_all_four_crossing_pairs() {
        // Seed 4's 4p Canal lands at AreaId(9), grid (1,3) - the only pin
        // that exercises the vertical + diagonal pairs (a 2-row 3p grid can
        // never produce them).
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

        let expected = [
            (AreaId::new(8), AreaId::new(10)),  // horizontal
            (AreaId::new(3), AreaId::new(15)),  // vertical
            (AreaId::new(2), AreaId::new(16)),  // diagonal "\"
            (AreaId::new(4), AreaId::new(14)),  // diagonal "/"
        ];
        assert_eq!(normalize_pairs(&layout.canal_crossings), normalize_pairs(&expected));
    }

    #[test]
    fn two_player_boards_unchanged() {
        let (layout, _) = seeded_2p_layout();
        assert!(layout.canal_crossings.is_empty());
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
