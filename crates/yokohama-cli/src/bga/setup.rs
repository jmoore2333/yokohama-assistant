//! Setup-answer synthesis (plan T5): earliest `gamedatas` snapshot + the
//! collapsed feed → a [`SetupAnswers`] sheet whose every field is
//! `Evidenced(value, provenance)` or `Absent(reason)`, and — where the
//! evidence suffices — a live-mode `Session` through the same
//! `Session::new_live` path the wizard uses. **The bridge answers the wizard's
//! questions; it never bypasses the engine, and it never answers a question it
//! lacks evidence for.**
//!
//! Honesty decisions this module encodes, with their measured grounds:
//!
//! - **The deck fence (inventory §8.3, non-negotiable):** `location "1"` rows
//!   in the snapshot's `orders`/`techs` maps carry full identity for cards
//!   not yet dealt. They are consumed as a COUNT ONLY — [`partition_cards`]
//!   tallies them without ever reading their identifying fields, so the leak
//!   is structurally unrepresented in everything downstream. Identity is taken
//!   only from rows at the board (`"2"`), held/completed (`"4"`) or
//!   setup-discarded (`"5"`) locations — the moments our model calls reveals.
//! - **Board sites are `Absent`:** BGA's `build_card` numbering has NO reward
//!   vector anywhere in the capture, so it is unverifiable against our site
//!   ids — and BGA's ORDER numbering being a permutation of ours is exactly
//!   why an unverified number table may not ship (mapping doc §6.3).
//! - **5-Power token faces are `Absent`:** the corpus evidences exactly one
//!   BGA `power_token` code (6 = Import(1), from the one socket `powerBonus`
//!   carrying its tile); codes 2/3/5 are unmapped, and `ObservedBoard.tokens`
//!   is all-or-nothing.
//! - **Achievements match by vector, ambiguity is `Absent`:** the BGA row is
//!   `(class, qty_1, qty_2, bonus_1_vps, bonus_2_vps)`. Class→category is
//!   pinned by the bonus pairs (8/6, 9/7, 10/8 — all three distinct and equal
//!   to the engine's). Quantities can collide (Tea7/Seafood7, Tokens5/Orders5)
//!   and for category C the `qty_1`↔production correspondence is UNEVIDENCED,
//!   so C candidates are computed under both field orders. One candidate →
//!   `Evidenced`; several → `Absent` naming them; zero → a hard error (drift).
//! - **Starting orders are `Unknown` for every seat:** `orderDiscarded` names
//!   the DISCARDED card (proof in inventory §7.2 — the log text is
//!   misleading) and says nothing about the kept one, which the live model
//!   learns later as a completion-time reveal. The discarded identities are
//!   carried as `orders_removed_known` evidence on the sheet — including the
//!   OPPONENT's, which BGA publishes (mapping doc §7.2) but which our
//!   `SetupObservations` surface cannot place without fabricating the other
//!   half of the deal, so the sheet is where it lives.
//! - **Initial Port/Lab are recoverable only from a game-start snapshot**
//!   (plan §0.5): a snapshot taken at `move_nbr >= 2` post-dates the
//!   starting-order discards and the display may have cycled; the H0 corpus's
//!   earliest is move 16, so this game imports with those fields `Absent` and
//!   H3 carries the procedure fix (snapshot at table open).

use super::names::{self, TechLedger, TechLookup};
use super::orders::OrderMapper;
use super::{require_u64, require_u8, BgaMapError, ImportError};
use serde_json::Value;
use std::collections::BTreeMap;
use yokohama_bridge::feed::{flex_u64, CollapsedFeed};
use yokohama_bridge::sanitize::sanitize;
use yokohama_core::{
    row_lengths, AchievementType, AreaType, FivePowerEffect, GameConfig, OrderCardId, PlayerColor,
    PlayerId, Rules, SetupObservations, StartingOrderChoice, TechCardId,
};

/// An answer with its provenance, or its absence with the reason the wizard
/// will still have to ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidence<T> {
    Evidenced { value: T, provenance: String },
    Absent { reason: String },
}

impl<T> Evidence<T> {
    pub fn value(&self) -> Option<&T> {
        match self {
            Evidence::Evidenced { value, .. } => Some(value),
            Evidence::Absent { .. } => None,
        }
    }

    fn evidenced(value: T, provenance: impl Into<String>) -> Self {
        Evidence::Evidenced { value, provenance: provenance.into() }
    }

    fn absent(reason: impl Into<String>) -> Self {
        Evidence::Absent { reason: reason.into() }
    }
}

/// One seat's identity facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatFacts {
    /// Engine seat (0-based, by BGA's `no` field).
    pub seat: u8,
    pub bga_player_id: String,
    pub name: String,
    pub color: Evidence<PlayerColor>,
}

/// A known member of `orders_removed`: a setup-discarded card BGA published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownDiscard {
    pub seat: u8,
    pub card: OrderCardId,
    pub provenance: String,
}

/// The answer sheet — the same questions the live wizard asks, each answered
/// with evidence or declined with a reason.
#[derive(Debug, Clone, PartialEq)]
pub struct SetupAnswers {
    pub table_id: u64,
    pub player_count: u8,
    pub seats: Vec<SeatFacts>,
    /// Engine seat of the start player.
    pub start_player: Evidence<u8>,
    /// Engine seat of the capturing session ("you").
    pub own_seat: Evidence<u8>,
    /// Row-major area types over `board_gen::row_lengths(player_count)`.
    pub board_tiles: Evidence<Vec<AreaType>>,
    /// Building-site card per cell — `Absent` (see module doc).
    pub board_sites: Evidence<Vec<Option<u8>>>,
    /// Face-up 5-Power token faces — `Absent` (see module doc).
    pub board_tokens: Evidence<Vec<Option<FivePowerEffect>>>,
    /// One per category A/B/C, in that order.
    pub achievements: [Evidence<AchievementType>; 3],
    pub port_a: Evidence<[OrderCardId; 6]>,
    pub port_b: Evidence<[OrderCardId; 6]>,
    pub lab_a: Evidence<[TechCardId; 6]>,
    pub lab_b: Evidence<[TechCardId; 6]>,
    /// Per seat. The corpus settles this as `Unknown` everywhere: the kept
    /// card is not in the early notifications, only the discarded one.
    pub starting_order_choices: Vec<Evidence<StartingOrderChoice>>,
    /// Setup-discarded cards BGA published, in seat order — sheet evidence
    /// (the engine's live model keeps its boxed fiction until reveals).
    pub orders_removed_known: Vec<KnownDiscard>,
    /// The §8.3 fence made visible: deck rows counted, never identified.
    pub deck_order_count: u32,
    pub deck_tech_count: u32,
    /// `notifications.move_nbr` of the snapshot the sheet was built from.
    pub snapshot_move_nbr: u64,
}

// ---------------------------------------------------------------------------
// gamedatas parsing helpers
// ---------------------------------------------------------------------------

/// The §8.3 fence, structurally: split a snapshot card map into a bare COUNT
/// of undealt-deck rows (`location "1"`) and the identity-bearing rows at
/// every other location. The deck arm never touches an identifying field.
fn partition_cards<'v>(map: &'v Value, context: &str) -> Result<(u32, Vec<&'v Value>), ImportError> {
    let obj = map.as_object().ok_or_else(|| ImportError::Sheet {
        detail: format!("{context} is not an object map"),
    })?;
    let mut deck_count = 0u32;
    let mut visible = Vec::new();
    for row in obj.values() {
        let loc = row
            .get("location")
            .and_then(|l| l.as_str())
            .ok_or_else(|| ImportError::Sheet {
                detail: format!("{context} row without a string `location`"),
            })?;
        if loc == "1" {
            deck_count += 1; // count only — identity stays unread (the fence)
        } else {
            visible.push(row);
        }
    }
    Ok((deck_count, visible))
}

struct TileRec {
    tile_key: u64,
    tile_id: u64,
    links: [u64; 6], // up_left, up_right, side_left, side_right, down_left, down_right
}

const LINK_FIELDS: [&str; 6] = ["up_left", "up_right", "side_left", "side_right", "down_left", "down_right"];

fn parse_tiles(gd: &Value) -> Result<BTreeMap<u64, TileRec>, ImportError> {
    let tiles = gd
        .get("tiles")
        .and_then(|t| t.as_object())
        .ok_or_else(|| ImportError::Sheet { detail: "gamedatas.tiles missing or not a map".to_string() })?;
    let mut out = BTreeMap::new();
    for row in tiles.values() {
        let tile_key = require_u64(row, "tile_key", "tile row").map_err(ImportError::Map)?;
        let tile_id = require_u64(row, "tile_id", "tile row").map_err(ImportError::Map)?;
        let mut links = [0u64; 6];
        for (i, f) in LINK_FIELDS.iter().enumerate() {
            links[i] = require_u64(row, f, "tile row").map_err(ImportError::Map)?;
        }
        out.insert(tile_key, TileRec { tile_key, tile_id, links });
    }
    Ok(out)
}

/// Walk BGA's tile links into row-major order, then CROSS-CHECK every link
/// against the brick-offset staggered pyramid our `board_gen` builds — a free
/// differential of our board topology against an independent implementation
/// (plan T5: "a mismatch is a STOP").
fn derive_grid(tiles: &BTreeMap<u64, TileRec>, player_count: u8) -> Result<Vec<u64>, ImportError> {
    let expected_rows: Vec<usize> = row_lengths(player_count)
        .iter()
        .map(|&n| n as usize)
        .collect();

    // Top-left: no up links, no left link. Must be unique.
    let tops: Vec<u64> = tiles
        .values()
        .filter(|t| t.links[0] == 0 && t.links[1] == 0 && t.links[2] == 0)
        .map(|t| t.tile_key)
        .collect();
    let [top] = tops[..] else {
        return Err(ImportError::Sheet {
            detail: format!("expected exactly one top-left tile (no up/left links); found {tops:?}"),
        });
    };

    let get = |key: u64| -> Result<&TileRec, ImportError> {
        tiles.get(&key).ok_or_else(|| ImportError::Sheet {
            detail: format!("tile link points at nonexistent tile_key {key}"),
        })
    };

    let mut rows: Vec<Vec<u64>> = Vec::new();
    let mut first = Some(top);
    while let Some(start) = first {
        let mut row = vec![start];
        let mut cur = get(start)?;
        while cur.links[3] != 0 {
            cur = get(cur.links[3])?;
            row.push(cur.tile_key);
        }
        let head = get(start)?;
        first = match (head.links[4], head.links[5]) {
            (0, 0) => None,
            (0, dr) => Some(dr),
            (dl, _) => Some(dl),
        };
        rows.push(row);
    }

    let got_rows: Vec<usize> = rows.iter().map(|r| r.len()).collect();
    if got_rows != expected_rows {
        return Err(ImportError::Sheet {
            detail: format!(
                "BGA row shape {got_rows:?} does not match the staggered pyramid {expected_rows:?} \
                 at {player_count}P — board-topology STOP"
            ),
        });
    }

    // Full six-link differential against the brick-offset expectation:
    // up_left=(r-1,c), up_right=(r-1,c+1), side_left=(r,c-1),
    // side_right=(r,c+1), down_left=(r+1,c-1), down_right=(r+1,c).
    let at = |r: isize, c: isize| -> u64 {
        if r < 0 || c < 0 {
            return 0;
        }
        rows.get(r as usize)
            .and_then(|row| row.get(c as usize))
            .copied()
            .unwrap_or(0)
    };
    for (r, row) in rows.iter().enumerate() {
        for (c, &key) in row.iter().enumerate() {
            let (r, c) = (r as isize, c as isize);
            let expect = [
                at(r - 1, c),
                at(r - 1, c + 1),
                at(r, c - 1),
                at(r, c + 1),
                at(r + 1, c - 1),
                at(r + 1, c),
            ];
            let t = get(key)?;
            if t.links != expect {
                return Err(ImportError::Sheet {
                    detail: format!(
                        "tile_key {key}: BGA links {:?} disagree with the brick-offset pyramid's {:?} \
                         (fields {LINK_FIELDS:?}) — board-topology STOP",
                        t.links, expect
                    ),
                });
            }
        }
    }

    let flat: Vec<u64> = rows.into_iter().flatten().collect();
    if flat.len() != tiles.len() {
        return Err(ImportError::Sheet {
            detail: format!("grid walk reached {} of {} tiles", flat.len(), tiles.len()),
        });
    }
    Ok(flat)
}

/// Engine quantities per achievement type, encoded from the type names'
/// rulebook meanings (`Copper5` = own 5 copper, ...). For category C the pair
/// is `(production, commercial)`.
fn achievement_quantities(ty: AchievementType) -> (u8, u8) {
    match ty {
        AchievementType::Copper5 => (5, 0),
        AchievementType::Silk6 => (6, 0),
        AchievementType::Tea7 => (7, 0),
        AchievementType::Seafood7 => (7, 0),
        AchievementType::Money10 => (10, 0),
        AchievementType::Tokens5 => (5, 0),
        AchievementType::Orders5 => (5, 0),
        AchievementType::Technologies4 => (4, 0),
        AchievementType::Production4 => (4, 0),
        AchievementType::Commercial4 => (0, 4),
        AchievementType::Prod3Comm2 => (3, 2),
        AchievementType::Prod2Comm3 => (2, 3),
    }
}

const CATEGORY_TYPES: [(u64, [AchievementType; 4]); 3] = [
    (1, [AchievementType::Copper5, AchievementType::Silk6, AchievementType::Tea7, AchievementType::Seafood7]),
    (2, [AchievementType::Money10, AchievementType::Tokens5, AchievementType::Orders5, AchievementType::Technologies4]),
    (3, [AchievementType::Production4, AchievementType::Commercial4, AchievementType::Prod3Comm2, AchievementType::Prod2Comm3]),
];

/// The candidate engine types one BGA achievement row can be, under both
/// field-order hypotheses (the qty_1↔production correspondence is
/// unevidenced — module doc). Exposed for H2's resolver, which turns a
/// claimed-category ambiguity into a replay-filtered hypothesis and an
/// unclaimed one into a declared assumption.
pub(crate) fn achievement_candidates(row: &Value) -> Result<(u64, Vec<AchievementType>), ImportError> {
    let class = require_u64(row, "achievement_class", "achievement row").map_err(ImportError::Map)?;
    let qty_1 = require_u8(row, "qty_1", "achievement row").map_err(ImportError::Map)?;
    let qty_2 = require_u8(row, "qty_2", "achievement row").map_err(ImportError::Map)?;
    let types = CATEGORY_TYPES
        .iter()
        .find(|(c, _)| *c == class)
        .map(|(_, t)| t)
        .ok_or_else(|| ImportError::Sheet {
            detail: format!("achievement_class {class} outside the pinned 1..=3 table"),
        })?;
    let mut candidates: Vec<AchievementType> = types
        .iter()
        .copied()
        .filter(|ty| {
            let (a, b) = achievement_quantities(*ty);
            (a, b) == (qty_1, qty_2) || (a, b) == (qty_2, qty_1)
        })
        .collect();
    candidates.dedup();
    Ok((class, candidates))
}

fn match_achievement(row: &Value) -> Result<Evidence<AchievementType>, ImportError> {
    let class = require_u64(row, "achievement_class", "achievement row").map_err(ImportError::Map)?;
    // require_u8, not `as u8` (NOTE-1, H1 review): a silent truncation at
    // >= 256 could false-match a drifted quantity instead of erroring.
    let qty_1 = require_u8(row, "qty_1", "achievement row").map_err(ImportError::Map)?;
    let qty_2 = require_u8(row, "qty_2", "achievement row").map_err(ImportError::Map)?;
    let bonus_1 = require_u8(row, "bonus_1_vps", "achievement row").map_err(ImportError::Map)?;
    let bonus_2 = require_u8(row, "bonus_2_vps", "achievement row").map_err(ImportError::Map)?;

    let types = CATEGORY_TYPES
        .iter()
        .find(|(c, _)| *c == class)
        .map(|(_, t)| t)
        .ok_or_else(|| ImportError::Sheet {
            detail: format!("achievement_class {class} outside the pinned 1..=3 table"),
        })?;
    // Class↔category is pinned by the bonus pairs — verify, don't assume.
    let sample = types[0];
    if bonus_1 != sample.first_place_vp() || bonus_2 != sample.subsequent_vp() {
        return Err(ImportError::Sheet {
            detail: format!(
                "achievement class {class} bonuses {bonus_1}/{bonus_2} disagree with the engine \
                 category's {}/{} — drift STOP",
                sample.first_place_vp(),
                sample.subsequent_vp()
            ),
        });
    }
    // Candidates under BOTH field-order hypotheses — the qty_1↔production
    // correspondence is unevidenced (module doc).
    let mut candidates: Vec<AchievementType> = types
        .iter()
        .copied()
        .filter(|ty| {
            let (a, b) = achievement_quantities(*ty);
            (a, b) == (qty_1, qty_2) || (a, b) == (qty_2, qty_1)
        })
        .collect();
    candidates.dedup();
    match candidates.as_slice() {
        [] => Err(ImportError::Sheet {
            detail: format!("achievement class {class} qty {qty_1}/{qty_2} matches no engine type — drift STOP"),
        }),
        [one] => Ok(Evidence::evidenced(
            *one,
            format!("gamedatas achievement class {class} qty {qty_1}/{qty_2}, unique vector match"),
        )),
        several => Ok(Evidence::absent(format!(
            "ambiguous: class {class} qty {qty_1}/{qty_2} matches {several:?} — the BGA row \
             carries no discriminating field; the wizard must ask"
        ))),
    }
}

fn parse_color(name: &str) -> Option<PlayerColor> {
    match name {
        "red" => Some(PlayerColor::Red),
        "blue" => Some(PlayerColor::Blue),
        "yellow" => Some(PlayerColor::Yellow),
        "green" => Some(PlayerColor::Green),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// synthesis
// ---------------------------------------------------------------------------

/// Build the answer sheet from the collapsed feed's earliest snapshot plus
/// the early notifications.
pub fn synthesize(feed: &CollapsedFeed, rules: &Rules) -> Result<SetupAnswers, ImportError> {
    let snapshot = feed.earliest_snapshot().ok_or(ImportError::NoSnapshot)?;
    let gd = &snapshot.gamedatas;
    let mapper = OrderMapper::from_rules(rules)?;
    let ledger = TechLedger::load(rules)?;

    // --- seats -------------------------------------------------------------
    let players = gd
        .get("players")
        .and_then(|p| p.as_object())
        .ok_or_else(|| ImportError::Sheet { detail: "gamedatas.players missing".to_string() })?;
    let player_count = u8::try_from(players.len()).map_err(|_| ImportError::Sheet {
        detail: format!("{} players", players.len()),
    })?;
    if !(2..=4).contains(&player_count) {
        return Err(ImportError::Sheet {
            detail: format!("{player_count} players is outside 2..=4"),
        });
    }
    let mut seats: Vec<SeatFacts> = Vec::new();
    for (pid, row) in players {
        let no = require_u64(row, "no", "player row").map_err(ImportError::Map)?;
        if no == 0 || no > player_count as u64 {
            return Err(ImportError::Sheet {
                detail: format!("player `no` {no} outside 1..={player_count}"),
            });
        }
        let name = row.get("name").and_then(|n| n.as_str()).unwrap_or("<unnamed>").to_string();
        let color_name = row.get("color_name").and_then(|c| c.as_str()).unwrap_or("");
        let color = match parse_color(color_name) {
            Some(c) => Evidence::evidenced(c, format!("gamedatas color_name `{color_name}`")),
            None => Evidence::absent(format!(
                "BGA color_name `{color_name}` is outside the engine's red/blue/yellow/green set"
            )),
        };
        seats.push(SeatFacts {
            seat: (no - 1) as u8,
            bga_player_id: pid.clone(),
            name,
            color,
        });
    }
    seats.sort_by_key(|s| s.seat);
    if seats.iter().enumerate().any(|(i, s)| s.seat != i as u8) {
        return Err(ImportError::Sheet {
            detail: "player `no` fields do not form a 1..=N sequence".to_string(),
        });
    }
    // Cross-check the independent seat-order source. `playerorder` is
    // VIEWER-ROTATED — BGA serves it starting from the client's own seat
    // (measured live at the gate game, table 896157897: the viewer sat second,
    // so playerorder began with them while the `no` fields held the absolute
    // order; the proving corpus's viewer happened to sit where the two agreed
    // as a plain rotation too). The honest cross-check is therefore ROTATION
    // equality: same players, same cyclic order. A non-rotation permutation
    // is still a seat-order STOP.
    if let Some(order) = gd.get("playerorder").and_then(|o| o.as_array()) {
        let from_order: Vec<String> = order.iter().filter_map(|v| flex_u64(v).map(|n| n.to_string())).collect();
        let from_no: Vec<String> = seats.iter().map(|s| s.bga_player_id.clone()).collect();
        if !is_rotation_of(&from_order, &from_no) {
            return Err(ImportError::Sheet {
                detail: format!(
                    "playerorder {from_order:?} is not a rotation of the `no` fields' {from_no:?} — seat-order STOP"
                ),
            });
        }
    }
    let seat_of = |player_id: &str| seats.iter().find(|s| s.bga_player_id == player_id).map(|s| s.seat);

    // --- board -------------------------------------------------------------
    let tiles = parse_tiles(gd)?;
    let grid = derive_grid(&tiles, player_count)?;
    let mut board_tiles = Vec::with_capacity(grid.len());
    for key in &grid {
        let rec = &tiles[key];
        board_tiles.push(names::area_type_from_tile_id(rec.tile_id).map_err(ImportError::Map)?);
    }
    let board_tiles = Evidence::evidenced(
        board_tiles,
        "gamedatas.tiles links walked row-major; all six link fields cross-checked against the \
         brick-offset pyramid",
    );
    let board_sites: Evidence<Vec<Option<u8>>> = Evidence::absent(
        "BGA's build_card numbering is unverified against our site ids — no reward vector exists \
         in the capture to check it, and BGA's ORDER numbering being a permutation of ours is \
         exactly why an unverified number table may not ship (mapping doc §6.3)",
    );
    let board_tokens: Evidence<Vec<Option<FivePowerEffect>>> = Evidence::absent(
        "BGA power_token codes are unmapped except 6 = Import(1) (one socket powerBonus with its \
         tile); codes 2/3/5 need future powerBonus evidence",
    );

    // --- achievements ------------------------------------------------------
    let ach_map = gd
        .get("achievements")
        .and_then(|a| a.as_object())
        .ok_or_else(|| ImportError::Sheet { detail: "gamedatas.achievements missing".to_string() })?;
    let mut per_class: BTreeMap<u64, Evidence<AchievementType>> = BTreeMap::new();
    for row in ach_map.values() {
        let class = require_u64(row, "achievement_class", "achievement row").map_err(ImportError::Map)?;
        if per_class.insert(class, match_achievement(row)?).is_some() {
            return Err(ImportError::Sheet {
                detail: format!("two achievements of class {class} in play"),
            });
        }
    }
    let achievements: [Evidence<AchievementType>; 3] = [1u64, 2, 3].map(|c| {
        per_class
            .get(&c)
            .cloned()
            .unwrap_or_else(|| Evidence::absent(format!("no class-{c} achievement in gamedatas")))
    });

    // --- decks and displays (the §8.3 fence lives in partition_cards) ------
    let orders_map = gd.get("orders").cloned().unwrap_or_else(|| Value::Object(Default::default()));
    let (deck_order_count, _visible_orders) = partition_cards(&orders_map, "gamedatas.orders")?;
    let techs_map = gd.get("techs").cloned().unwrap_or_else(|| Value::Object(Default::default()));
    let (deck_tech_count, visible_techs) = partition_cards(&techs_map, "gamedatas.techs")?;

    let snapshot_move_nbr = gd
        .get("notifications")
        .and_then(|n| n.get("move_nbr"))
        .and_then(flex_u64)
        .unwrap_or(u64::MAX);

    // Initial Port/Lab: only a game-start snapshot can testify (plan §0.5).
    let not_game_start = snapshot_move_nbr >= 2;
    let (port_a, lab_a): (Evidence<[OrderCardId; 6]>, Evidence<[TechCardId; 6]>) = if not_game_start {
        let reason = format!(
            "earliest snapshot is at move {snapshot_move_nbr}, not game start — the initial six \
             are unrecoverable backwards (history orderRefresh names no card; plan §0.5); H3's \
             procedure fix is a snapshot at table open"
        );
        (Evidence::absent(reason.clone()), Evidence::absent(reason))
    } else if player_count > 2 {
        let reason = "3P/4P split of location-2 rows between boards A and B is unobserved (U16)".to_string();
        (Evidence::absent(reason.clone()), Evidence::absent(reason))
    } else {
        // Game-start 2P snapshot: the six location-2 rows per display, by
        // position. Any failure to identify all six is a typed error — a
        // partial display would be a guessed display.
        let port = read_display_orders(&orders_map, &mapper)?;
        let lab = read_display_techs(&visible_techs, &ledger)?;
        (port, lab)
    };
    let off_board = |what: &str| format!("{what} is not in play at {player_count}P");
    let port_b: Evidence<[OrderCardId; 6]> = if player_count >= 3 {
        Evidence::absent("3P/4P display split between boards A and B is unobserved (U16)")
    } else {
        Evidence::absent(off_board("Port B"))
    };
    let lab_b: Evidence<[TechCardId; 6]> = if player_count >= 4 {
        Evidence::absent("3P/4P display split between boards A and B is unobserved (U16)")
    } else {
        Evidence::absent(off_board("Lab B"))
    };

    // --- starting orders and the published discards ------------------------
    let mut orders_removed_known = Vec::new();
    for mv in &feed.moves {
        for pk in [mv.history.as_ref(), mv.socket.as_ref()].into_iter().flatten() {
            for n in &pk.notifications {
                if n.kind == "orderDiscarded" {
                    let order = n.args.get("order").ok_or_else(|| {
                        ImportError::Map(BgaMapError::MissingField {
                            context: format!("orderDiscarded at move {}", mv.move_id),
                            field: "order".to_string(),
                        })
                    })?;
                    let mapped = mapper.map_row(order, &format!("orderDiscarded at move {}", mv.move_id))?;
                    let pid = n.args.get("player_id").and_then(|p| p.as_str()).unwrap_or("");
                    let seat = seat_of(pid).ok_or_else(|| ImportError::Sheet {
                        detail: format!("orderDiscarded by unknown player_id {pid}"),
                    })?;
                    let discard = KnownDiscard {
                        seat,
                        card: mapped.card,
                        provenance: format!("orderDiscarded at move {} (BGA order {})", mv.move_id, mapped.bga_order_id),
                    };
                    if !orders_removed_known.contains(&discard) {
                        orders_removed_known.push(discard);
                    }
                }
            }
        }
    }
    orders_removed_known.sort_by_key(|d| d.seat);
    let starting_order_choices: Vec<Evidence<StartingOrderChoice>> = seats
        .iter()
        .map(|s| {
            Evidence::evidenced(
                StartingOrderChoice::Unknown,
                format!(
                    "seat {}: orderDiscarded names only the DISCARDED card; the kept card is \
                     revealed on completion, which is the live model's normal path",
                    s.seat
                ),
            )
        })
        .collect();

    // --- start player and own seat -----------------------------------------
    let start_player = feed
        .moves
        .iter()
        .find_map(|mv| {
            [mv.history.as_ref(), mv.socket.as_ref()]
                .into_iter()
                .flatten()
                .flat_map(|pk| pk.notifications.iter().map(move |n| (mv.move_id, n)))
                .find(|(_, n)| n.kind == "assistantPlacement")
        })
        .and_then(|(mid, n)| {
            let pid = n.args.get("player_id").and_then(|p| p.as_str())?;
            let seat = seat_of(pid)?;
            Some(Evidence::evidenced(
                seat,
                format!("first assistantPlacement (move {mid}) — the start player owns the first Placement"),
            ))
        })
        .unwrap_or_else(|| Evidence::absent("no assistantPlacement in the captured feed".to_string()));

    let own_candidates: Vec<u8> = feed
        .report
        .chrome
        .player_channels
        .iter()
        .filter_map(|ch| ch.strip_prefix("/player/p"))
        .filter_map(seat_of)
        .collect();
    let own_seat = match own_candidates.as_slice() {
        [one] => Evidence::evidenced(
            *one,
            "the socket /player/p<id> channel — BGA subscribes a client only to its own player channel",
        ),
        [] => Evidence::absent(
            "no /player/ channel in the capture (no websocket window, or a history-only import)",
        ),
        several => Evidence::absent(format!("multiple /player/ channels seen for seats {several:?}")),
    };

    Ok(SetupAnswers {
        table_id: feed.table_id,
        player_count,
        seats,
        start_player,
        own_seat,
        board_tiles,
        board_sites,
        board_tokens,
        achievements,
        port_a,
        port_b,
        lab_a,
        lab_b,
        starting_order_choices,
        orders_removed_known,
        deck_order_count,
        deck_tech_count,
        snapshot_move_nbr,
    })
}

/// Six location-2 order rows by `position` 1..=6 → the Port A display.
fn read_display_orders(
    orders_map: &Value,
    mapper: &OrderMapper,
) -> Result<Evidence<[OrderCardId; 6]>, ImportError> {
    let (_, visible) = partition_cards(orders_map, "gamedatas.orders")?;
    let mut slots: [Option<OrderCardId>; 6] = [None; 6];
    for row in visible {
        if row.get("location").and_then(|l| l.as_str()) != Some("2") {
            continue;
        }
        let pos = require_u64(row, "position", "display order row").map_err(ImportError::Map)?;
        if !(1..=6).contains(&pos) {
            return Err(ImportError::Sheet {
                detail: format!("display order at position {pos} outside 1..=6"),
            });
        }
        let mapped = mapper.map_row(row, "display order row")?;
        if slots[(pos - 1) as usize].replace(mapped.card).is_some() {
            return Err(ImportError::Sheet {
                detail: format!("two display orders at position {pos}"),
            });
        }
    }
    let mut out = [OrderCardId(0); 6];
    for (i, s) in slots.iter().enumerate() {
        out[i] = s.ok_or_else(|| ImportError::Sheet {
            detail: format!("game-start snapshot has no display order at position {}", i + 1),
        })?;
    }
    Ok(Evidence::evidenced(out, "gamedatas orders at location 2, positions 1..6"))
}

/// Six location-2 tech rows → the Lab A display, through the ledger; a single
/// `Unknown` id makes the whole field `Absent` naming it (plan T5).
fn read_display_techs(
    visible: &[&Value],
    ledger: &TechLedger,
) -> Result<Evidence<[TechCardId; 6]>, ImportError> {
    let mut slots: [Option<TechCardId>; 6] = [None; 6];
    let mut unknown: Vec<u64> = Vec::new();
    for row in visible {
        if row.get("location").and_then(|l| l.as_str()) != Some("2") {
            continue;
        }
        let pos = require_u64(row, "position", "display tech row").map_err(ImportError::Map)?;
        if !(1..=6).contains(&pos) {
            return Err(ImportError::Sheet {
                detail: format!("display tech at position {pos} outside 1..=6"),
            });
        }
        let tech_id = require_u64(row, "tech_id", "display tech row").map_err(ImportError::Map)?;
        match ledger.lookup(tech_id) {
            TechLookup::Known(r) => {
                if slots[(pos - 1) as usize].replace(r.card).is_some() {
                    return Err(ImportError::Sheet {
                        detail: format!("two display techs at position {pos}"),
                    });
                }
            }
            TechLookup::Unknown { tech_id } => unknown.push(tech_id),
        }
    }
    if !unknown.is_empty() {
        return Ok(Evidence::absent(format!(
            "tech ledger has no identity for tech_id(s) {unknown:?} — an unresolved title is \
             never defaulted; the wizard must ask (mapping doc §8 MAY NOT)"
        )));
    }
    let mut out = [TechCardId(0); 6];
    for (i, s) in slots.iter().enumerate() {
        out[i] = s.ok_or_else(|| ImportError::Sheet {
            detail: format!("game-start snapshot has no display tech at position {}", i + 1),
        })?;
    }
    Ok(Evidence::evidenced(out, "gamedatas techs at location 2, positions 1..6, through the ledger"))
}

// ---------------------------------------------------------------------------
// session construction
// ---------------------------------------------------------------------------

/// What `Session::new_live` needs beyond the sheet's always-present facts.
/// Returns the missing-field reasons when the evidence does not suffice —
/// the `Incomplete` refusal's content.
pub fn missing_for_session(answers: &SetupAnswers) -> Vec<String> {
    let mut missing = Vec::new();
    let mut need = |name: &str, absent: Option<&str>| {
        if let Some(reason) = absent {
            missing.push(format!("{name} ({reason})"));
        }
    };
    let reason_of = |e: &Evidence<_>| -> Option<String> {
        match e {
            Evidence::Absent { reason } => Some(reason.clone()),
            Evidence::Evidenced { .. } => None,
        }
    };
    // Board and achievements are what makes the record's world the real
    // table's world; a seed-fiction board under a live record would corrupt
    // every later move reference.
    need("board tiles", reason_of(&self_generic(&answers.board_tiles)).as_deref());
    need("board sites", reason_of(&self_generic(&answers.board_sites)).as_deref());
    need("board 5-Power tokens", reason_of(&self_generic(&answers.board_tokens)).as_deref());
    for (i, a) in answers.achievements.iter().enumerate() {
        let cat = ["A", "B", "C"][i];
        need(&format!("achievement {cat}"), reason_of(&self_generic(a)).as_deref());
    }
    need("Port A initial six", reason_of(&self_generic(&answers.port_a)).as_deref());
    need("Lab A initial six", reason_of(&self_generic(&answers.lab_a)).as_deref());
    if answers.player_count >= 3 {
        need("Port B initial six", reason_of(&self_generic(&answers.port_b)).as_deref());
    }
    if answers.player_count >= 4 {
        need("Lab B initial six", reason_of(&self_generic(&answers.lab_b)).as_deref());
    }
    need("start player", reason_of(&self_generic(&answers.start_player)).as_deref());
    for s in &answers.seats {
        need(&format!("seat {} color", s.seat), reason_of(&self_generic(&s.color)).as_deref());
    }
    missing
}

/// Erase the value type so `missing_for_session` can treat every field alike.
fn self_generic<T: Clone>(e: &Evidence<T>) -> Evidence<()> {
    match e {
        Evidence::Evidenced { provenance, .. } => Evidence::Evidenced { value: (), provenance: provenance.clone() },
        Evidence::Absent { reason } => Evidence::Absent { reason: reason.clone() },
    }
}

/// Build the live-mode `Session` — the same `new_live` path as the wizard,
/// with the synthesized observations and per-seat `Unknown` choices. The
/// setup seed is the TABLE ID: deterministic and stated, so two importer runs
/// produce identical records (there is no physical seed to observe; in live
/// mode the seed only shapes the hidden-fiction remainder).
pub fn build_session(answers: &SetupAnswers, rules: Rules) -> Result<crate::session::Session, ImportError> {
    let missing = missing_for_session(answers);
    if !missing.is_empty() {
        return Err(ImportError::Incomplete { missing });
    }
    let colors: Vec<PlayerColor> = answers
        .seats
        .iter()
        .map(|s| *s.color.value().expect("checked by missing_for_session"))
        .collect();
    let start_seat = *answers.start_player.value().expect("checked");
    let mut config = GameConfig::new(
        answers.player_count,
        colors,
        PlayerId::new(start_seat),
    );
    config.has_port_b = answers.player_count >= 3;
    config.has_lab_b = answers.player_count >= 4;

    let obs = SetupObservations {
        board: Some(yokohama_core::ObservedBoard {
            tiles: answers.board_tiles.value().expect("checked").clone(),
            sites: answers.board_sites.value().expect("checked").clone(),
            tokens: Some(answers.board_tokens.value().expect("checked").clone()),
        }),
        achievements: Some([
            *answers.achievements[0].value().expect("checked"),
            *answers.achievements[1].value().expect("checked"),
            *answers.achievements[2].value().expect("checked"),
        ]),
        port_a: answers.port_a.value().copied(),
        port_b: answers.port_b.value().copied(),
        lab_a: answers.lab_a.value().copied(),
        lab_b: answers.lab_b.value().copied(),
        deals: Vec::new(),
    };
    let choices: Vec<StartingOrderChoice> = answers
        .starting_order_choices
        .iter()
        .map(|e| e.value().copied().unwrap_or(StartingOrderChoice::Unknown))
        .collect();
    let mut session = crate::session::Session::new_live(config, answers.table_id, obs, choices, rules)
        .map_err(|e| ImportError::Session { detail: e })?;

    // MUST-1 (H1 review): stamp the comparison-only provenance the plan §1
    // promises — without it a record saved through `--out` is
    // indistinguishable from a typed live game, and the map §1.5/§1.6 fence
    // has nothing to ride. Per seat: the `bga-human` controller (the seat was
    // played by a human ON BGA, not at this keyboard), the importer version,
    // and the BGA player id in params. `observation` is `OBSERVATION_VIEW`:
    // BGA's client is own-perspective — the same seat-relative visibility
    // class that constant names — and the record itself is live-mode with the
    // opponent's hand hidden.
    let seats: Result<Vec<yokohama_core::SeatProvenance>, ImportError> = answers
        .seats
        .iter()
        .map(|s| {
            let pid: i64 = s.bga_player_id.parse().map_err(|_| ImportError::Sheet {
                detail: format!(
                    "seat {}: BGA player id `{}` is not numeric — cannot stamp provenance params",
                    s.seat, s.bga_player_id
                ),
            })?;
            Ok(yokohama_core::SeatProvenance {
                controller: super::CONTROLLER_BGA_HUMAN.to_string(),
                version: super::BGA_IMPORT_VERSION,
                params: std::iter::once(("bga_player_id".to_string(), pid)).collect(),
                agent_seed: None,
                observation: yokohama_core::OBSERVATION_VIEW.to_string(),
            })
        })
        .collect();
    session.set_run_seats(seats?);
    session.set_run_id(&format!("bga-import/{}", answers.table_id));
    Ok(session)
}

// ---------------------------------------------------------------------------
// sheet rendering
// ---------------------------------------------------------------------------

fn fmt_evidence<T: std::fmt::Debug>(e: &Evidence<T>) -> String {
    match e {
        Evidence::Evidenced { value, provenance } => format!("{value:?}\n      evidence: {provenance}"),
        Evidence::Absent { reason } => format!("ABSENT — {reason}"),
    }
}

/// The `--answers` sheet: every field, every provenance, every reason —
/// deterministic ordering, plain text.
pub fn render_sheet(a: &SetupAnswers) -> String {
    let mut out = String::new();
    let mut line = |s: String| {
        out.push_str(&s);
        out.push('\n');
    };
    line(format!("BGA setup answer sheet — table {}", a.table_id));
    line(format!("  players: {}", a.player_count));
    for s in &a.seats {
        // The seat name and player id are the SOFTEST captured strings — an
        // opponent chooses their own BGA display name — so they are escaped
        // here at the point they become sheet text (F1 / lens-1 MUST).
        line(format!(
            "  seat {}: {} (BGA id {}) — color {}",
            s.seat,
            sanitize(&s.name),
            sanitize(&s.bga_player_id),
            fmt_evidence(&s.color)
        ));
    }
    line(format!("  start player: {}", fmt_evidence(&a.start_player)));
    line(format!("  own seat: {}", fmt_evidence(&a.own_seat)));
    line(format!("  board tiles (row-major): {}", fmt_evidence(&a.board_tiles)));
    line(format!("  board sites: {}", fmt_evidence(&a.board_sites)));
    line(format!("  board 5-Power tokens: {}", fmt_evidence(&a.board_tokens)));
    for (i, ach) in a.achievements.iter().enumerate() {
        line(format!("  achievement {}: {}", ["A", "B", "C"][i], fmt_evidence(ach)));
    }
    line(format!("  Port A initial six: {}", fmt_evidence(&a.port_a)));
    line(format!("  Port B initial six: {}", fmt_evidence(&a.port_b)));
    line(format!("  Lab A initial six: {}", fmt_evidence(&a.lab_a)));
    line(format!("  Lab B initial six: {}", fmt_evidence(&a.lab_b)));
    for (i, c) in a.starting_order_choices.iter().enumerate() {
        line(format!("  seat {i} starting order: {}", fmt_evidence(c)));
    }
    if a.orders_removed_known.is_empty() {
        line("  published setup discards: none observed".to_string());
    }
    for d in &a.orders_removed_known {
        line(format!(
            "  published setup discard: seat {} boxed O{:02}\n      evidence: {}",
            d.seat, d.card.0, d.provenance
        ));
    }
    line(format!(
        "  undealt deck (fence: counted, never identified): {} orders, {} techs",
        a.deck_order_count, a.deck_tech_count
    ));
    line(format!("  snapshot taken at move: {}", a.snapshot_move_nbr));
    let missing = missing_for_session(a);
    if missing.is_empty() {
        line("  => evidence suffices for a live-mode Session".to_string());
    } else {
        line("  => the wizard would still ask:".to_string());
        for m in &missing {
            line(format!("     - {m}"));
        }
    }
    out
}


/// Cyclic-order equality: `a` is `b` rotated by some shift. `playerorder` is
/// the `no` order rotated to start at the viewing client's seat, so equality
/// up to rotation is the honest cross-check; a NON-rotation permutation means
/// the two sources genuinely disagree. At 2 players every permutation is a
/// rotation, so this check has teeth only at 3+ — pinned directly below.
fn is_rotation_of(a: &[String], b: &[String]) -> bool {
    a.len() == b.len()
        && !b.is_empty()
        && (0..b.len()).any(|shift| (0..b.len()).all(|i| a[i] == b[(i + shift) % b.len()]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use yokohama_bridge::feed::read_and_collapse;

    const TABLE: u64 = 895_724_349;

    /// The bridge crate's committed redacted excerpt — same workspace, fixed
    /// relative path. The unit tier runs on this so CI-without-corpus works.
    fn excerpt_feed() -> CollapsedFeed {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../yokohama-bridge/tests/data/excerpt-895724349.ndjson"
        );
        let text = std::fs::read_to_string(path).expect("committed excerpt exists");
        read_and_collapse(&text, Some(TABLE)).expect("excerpt collapses")
    }

    fn sheet() -> SetupAnswers {
        synthesize(&excerpt_feed(), &Rules::standard()).expect("sheet synthesizes")
    }


    /// The live gate-game regression (table 896157897): `playerorder` arrives
    /// VIEWER-ROTATED — the client's own seat first — so a viewer sitting
    /// second produces a rotation of the `no` order, not equality. The sheet
    /// must synthesize; seats stay derived from `no`.
    #[test]
    fn a_viewer_rotated_playerorder_is_accepted_and_seats_come_from_no() {
        let mut feed = excerpt_feed();
        let po = feed.snapshots[0].gamedatas["playerorder"].clone();
        let rotated: Vec<serde_json::Value> = po.as_array().expect("playerorder array").iter().rev().cloned().collect();
        feed.snapshots[0].gamedatas["playerorder"] = serde_json::json!(rotated);
        let a = synthesize(&feed, &Rules::standard()).expect("a rotation is the viewer's perspective, not a disagreement");
        assert_eq!(a.seats[0].name, "Player_A", "seat order comes from the `no` fields, not from playerorder");
    }

    /// The teeth (unreachable at 2P where every permutation is a rotation): a
    /// 3P non-rotation permutation is a genuine source disagreement.
    #[test]
    fn a_non_rotation_playerorder_still_stops() {
        let a = ["1".to_string(), "2".to_string(), "3".to_string()];
        assert!(is_rotation_of(&["2".into(), "3".into(), "1".into()], &a));
        assert!(is_rotation_of(&["3".into(), "1".into(), "2".into()], &a));
        assert!(!is_rotation_of(&["1".into(), "3".into(), "2".into()], &a), "a swap of two among three is not a rotation");
        assert!(!is_rotation_of(&["1".into(), "2".into()], &a), "length mismatch is never a rotation");
    }

    #[test]
    fn the_excerpt_sheet_carries_the_seats_start_player_and_own_seat() {
        let a = sheet();
        assert_eq!(a.player_count, 2);
        assert_eq!(a.seats[0].name, "Player_A");
        assert_eq!(a.seats[1].name, "Player_B");
        assert_eq!(a.seats[0].color.value(), Some(&PlayerColor::Red));
        assert_eq!(a.seats[1].color.value(), Some(&PlayerColor::Green));
        // First assistantPlacement (move 4) is Player_A's — the start seat.
        assert_eq!(a.start_player.value(), Some(&0));
        // The /player/p11111111 channel names the capturing session.
        assert_eq!(a.own_seat.value(), Some(&0));
    }

    /// F1 (lens-1 MUST): a captured seat name carrying terminal control bytes —
    /// an opponent chooses their own BGA display name — is ESCAPED before it
    /// reaches the answer sheet a tester reads in their terminal. The importer
    /// never rejects a name (design: hard-error on identity, not on cosmetics),
    /// so the hostile name survives synthesis but renders with zero ESC/BEL.
    #[test]
    fn a_hostile_seat_name_is_escaped_in_the_rendered_sheet() {
        let mut feed = excerpt_feed();
        // ESC[2J (clear) · ESC[31m (red) · PWN · BEL — Player_A is seat 0.
        let hostile = "\u{1b}[2J\u{1b}[31mPWN\u{07}";
        feed.snapshots[0].gamedatas["players"]["11111111"]["name"] =
            serde_json::json!(hostile);
        let a = synthesize(&feed, &Rules::standard()).expect("a hostile name still synthesizes");
        assert!(a.seats.iter().any(|s| s.name == hostile), "the raw name is carried in state");
        let sheet = render_sheet(&a);
        assert!(!sheet.contains('\u{1b}'), "no raw ESC byte may reach the sheet");
        assert!(!sheet.contains('\u{07}'), "no raw BEL byte may reach the sheet");
        assert!(sheet.contains("\\u{1b}"), "the ESC shows as a visible escape");
        assert!(sheet.contains("PWN"), "the printable part of the name survives");
    }

    #[test]
    fn the_excerpt_board_derives_row_major_and_survives_the_topology_differential() {
        let a = sheet();
        let tiles = a.board_tiles.value().expect("board evidenced");
        assert_eq!(
            tiles,
            &vec![
                AreaType::TeaPlantation,
                AreaType::PortA,
                AreaType::EmploymentAgency,
                AreaType::Bank,
                AreaType::FishingGround,
                AreaType::SilkMill,
                AreaType::Church,
                AreaType::Customs,
                AreaType::LaboratoryA,
                AreaType::CopperMine,
            ],
            "tile_keys 1..10 row-major through the evidenced tile_id table"
        );
        // Sites and tokens stay honest Absents.
        assert!(a.board_sites.value().is_none());
        assert!(a.board_tokens.value().is_none());
    }

    #[test]
    fn a_corrupted_tile_link_is_a_board_topology_stop() {
        let mut feed = excerpt_feed();
        // Break one link in the snapshot: tile_key 1's side_right 2 -> 3.
        // The row WALK trips on this one (row shape comes out wrong).
        let gd = &mut feed.snapshots[0].gamedatas;
        gd["tiles"]["1"]["side_right"] = serde_json::json!("3");
        match synthesize(&feed, &Rules::standard()).unwrap_err() {
            ImportError::Sheet { detail } => {
                assert!(detail.contains("STOP"), "{detail}");
            }
            other => panic!("expected Sheet STOP, got {other:?}"),
        }
    }

    /// SHOULD-1 (H1 review, escaped mutation M8): the six-link differential
    /// must be discriminated by a fixture the row WALK cannot catch. Tile 6's
    /// `up_left` is a link the walk never reads (top detection survives — the
    /// field stays nonzero — and the walk chains only `side_right` plus the
    /// row heads' down links), so ONLY the full differential trips here:
    /// deleting it leaves the walk green and this test red.
    #[test]
    fn a_corrupted_up_link_is_caught_by_the_six_link_differential() {
        let mut feed = excerpt_feed();
        // Tile 6 sits at row 1 col 1; its true up_left is tile_key 2.
        let gd = &mut feed.snapshots[0].gamedatas;
        gd["tiles"]["6"]["up_left"] = serde_json::json!("3");
        match synthesize(&feed, &Rules::standard()).unwrap_err() {
            ImportError::Sheet { detail } => {
                assert!(
                    detail.contains("brick-offset pyramid"),
                    "the differential, not the walk, must name this STOP: {detail}"
                );
            }
            other => panic!("expected Sheet STOP, got {other:?}"),
        }
    }

    #[test]
    fn the_excerpt_achievements_are_all_honestly_ambiguous() {
        // Class A qty 7 = Tea7|Seafood7; class B qty 5 = Tokens5|Orders5;
        // class C qty 2/3 = Prod2Comm3|Prod3Comm2 (field order unevidenced).
        let a = sheet();
        for (i, ach) in a.achievements.iter().enumerate() {
            match ach {
                Evidence::Absent { reason } => {
                    assert!(reason.contains("ambiguous"), "achievement {i}: {reason}");
                }
                other => panic!("achievement {i} should be ambiguous here, got {other:?}"),
            }
        }
    }

    #[test]
    fn an_unambiguous_achievement_row_is_evidenced() {
        let row = serde_json::json!({
            "achievement_class": "1", "qty_1": "5", "qty_2": "0",
            "bonus_1_vps": "8", "bonus_2_vps": "6",
        });
        match match_achievement(&row).unwrap() {
            Evidence::Evidenced { value, .. } => assert_eq!(value, AchievementType::Copper5),
            other => panic!("expected Evidenced Copper5, got {other:?}"),
        }
    }

    #[test]
    fn an_achievement_bonus_disagreement_is_a_drift_stop() {
        let row = serde_json::json!({
            "achievement_class": "1", "qty_1": "5", "qty_2": "0",
            "bonus_1_vps": "9", "bonus_2_vps": "6",
        });
        assert!(matches!(match_achievement(&row), Err(ImportError::Sheet { .. })));
    }

    #[test]
    fn the_excerpt_discards_and_deck_counts_honor_the_fence() {
        let a = sheet();
        // Both seats Unknown; the published discards ride the sheet.
        assert_eq!(a.starting_order_choices.len(), 2);
        for c in &a.starting_order_choices {
            assert_eq!(c.value(), Some(&StartingOrderChoice::Unknown));
        }
        let discards: Vec<(u8, u8)> = a.orders_removed_known.iter().map(|d| (d.seat, d.card.0)).collect();
        assert_eq!(discards, vec![(0, 3), (1, 27)], "BGA 24 → O03 (seat 0), BGA 25 → O27 (seat 1)");
        // The snapshot's 8 undrawn deck rows are a COUNT — identity unread.
        assert_eq!(a.deck_order_count, 8);
        assert_eq!(a.deck_tech_count, 23);
    }

    #[test]
    fn the_mid_game_snapshot_leaves_port_and_lab_absent_with_the_measured_reason() {
        let a = sheet();
        assert_eq!(a.snapshot_move_nbr, 16);
        for (name, absent) in [
            ("port_a", a.port_a.value().is_none()),
            ("lab_a", a.lab_a.value().is_none()),
        ] {
            assert!(absent, "{name} must be Absent for a move-16 snapshot");
        }
    }

    #[test]
    fn the_incomplete_refusal_names_exactly_what_the_wizard_would_ask() {
        let a = sheet();
        let missing = missing_for_session(&a);
        let names: Vec<&str> = missing.iter().map(|m| m.split(" (").next().unwrap()).collect();
        assert_eq!(
            names,
            vec![
                "board sites",
                "board 5-Power tokens",
                "achievement A",
                "achievement B",
                "achievement C",
                "Port A initial six",
                "Lab A initial six",
            ]
        );
        match build_session(&a, Rules::standard()) {
            Err(ImportError::Incomplete { missing: m }) => assert_eq!(m.len(), 7),
            Err(other) => panic!("expected Incomplete, got {other:?}"),
            Ok(_) => panic!("a session must not build from an incomplete sheet"),
        }
    }

    /// A synthetic FULLY-evidenced sheet: what a table-open snapshot plus
    /// resolved site/token/achievement evidence will one day produce. The
    /// values are engine-legal (the 2P area set, ten distinct site cards, ten
    /// Import(1) tokens — the pool holds exactly ten) so `new_live`'s
    /// reconcile accepts them; the point is the CONSTRUCTION PATH, which the
    /// real corpus cannot yet drive.
    fn complete_sheet() -> SetupAnswers {
        let mut a = sheet();
        let ev = |p: &str| p.to_string();
        a.board_sites = Evidence::Evidenced {
            value: (1..=10).map(|n| Some(n as u8)).collect(),
            provenance: ev("synthetic fixture"),
        };
        a.board_tokens = Evidence::Evidenced {
            value: vec![Some(FivePowerEffect::ImportedGoods(1)); 10],
            provenance: ev("synthetic fixture"),
        };
        a.achievements = [
            Evidence::Evidenced { value: AchievementType::Copper5, provenance: ev("synthetic") },
            Evidence::Evidenced { value: AchievementType::Money10, provenance: ev("synthetic") },
            Evidence::Evidenced { value: AchievementType::Production4, provenance: ev("synthetic") },
        ];
        a.port_a = Evidence::Evidenced {
            value: [1, 2, 3, 4, 5, 6].map(OrderCardId),
            provenance: ev("synthetic"),
        };
        a.lab_a = Evidence::Evidenced {
            value: [1, 2, 3, 4, 5, 6].map(TechCardId),
            provenance: ev("synthetic"),
        };
        a
    }

    #[test]
    fn a_complete_sheet_builds_a_live_mode_session_through_new_live() {
        let a = complete_sheet();
        let session = build_session(&a, Rules::standard()).expect("session builds");
        assert_eq!(session.mode, crate::session::TrackingMode::Live);
        let state = session.state();
        // The observed board came through reconcile: the areas are the
        // sheet's, IN ROW-MAJOR POSITION — compare via grid_position ordering
        // (SHOULD-3, H1 review: a Debug-string sort checks only the SET, so a
        // scrambled row-major order would have passed it).
        let mut cells: Vec<((u8, u8), AreaType)> = state
            .board_layout
            .areas
            .values()
            .map(|c| (c.grid_position, c.area_type))
            .collect();
        cells.sort_by_key(|(pos, _)| *pos);
        let areas: Vec<AreaType> = cells.into_iter().map(|(_, t)| t).collect();
        assert_eq!(&areas, a.board_tiles.value().unwrap());
        // Both seats Unknown: hidden starting orders, four boxed fiction deals.
        assert!(state.players.iter().all(|p| p.starting_order_hidden));
        // The record is live-mode: setup observations present.
        assert!(session.record().setup_observations.is_some());
        // Port A holds exactly the observed six.
        let port: Vec<OrderCardId> = state.port_state.port_a.slots.iter().flatten().copied().collect();
        assert_eq!(port, [1, 2, 3, 4, 5, 6].map(OrderCardId).to_vec());
    }

    /// MUST-1 (H1 review): the promised comparison-only provenance is REAL —
    /// a record this importer emits is distinguishable from a typed live game
    /// by its `run` stamp, which is the hook the map §1.5/§1.6 fence rides.
    #[test]
    fn the_built_session_carries_comparison_only_bga_provenance() {
        let a = complete_sheet();
        let session = build_session(&a, Rules::standard()).expect("session builds");
        let run = session.record().run.as_ref().expect("provenance stamped");
        assert_eq!(
            run.run_id.as_deref(),
            Some("bga-import/895724349"),
            "the comparison-only fence rides the run_id prefix (plan §1)"
        );
        assert_eq!(run.master_seed, None, "no seed schedule above an import — honest absence");
        assert_eq!(run.seats.len(), 2);
        for seat in &run.seats {
            assert_eq!(seat.controller, super::super::CONTROLLER_BGA_HUMAN);
            assert_eq!(seat.version, super::super::BGA_IMPORT_VERSION);
            assert_eq!(seat.observation, yokohama_core::OBSERVATION_VIEW);
            assert_eq!(seat.agent_seed, None, "a BGA human has no RNG stream of ours");
        }
        assert_eq!(run.seats[0].params.get("bga_player_id"), Some(&11_111_111i64));
        assert_eq!(run.seats[1].params.get("bga_player_id"), Some(&22_222_222i64));
    }

    #[test]
    fn the_rendered_sheet_names_every_absence_and_the_fence() {
        let text = render_sheet(&sheet());
        assert!(text.contains("ABSENT"), "absences printed");
        assert!(text.contains("counted, never identified"), "the fence is stated");
        assert!(text.contains("the wizard would still ask"), "refusal is actionable");
        assert!(text.contains("seat 0 boxed O03"), "published discard evidence");
        let complete = render_sheet(&complete_sheet());
        assert!(complete.contains("evidence suffices for a live-mode Session"));
    }
}
