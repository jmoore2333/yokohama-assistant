//! ASCII rendering of game state for the CLI: read-only views over
//! `GameState` plus a before/after delta summarizer (design
//! docs/plans/2026-07-18-phase2-cli-driver-design.md).
//!
//! Rendering rule: tests assert content (substrings/line counts), not
//! full-screen golden strings, so cosmetic polish during dogfooding doesn't
//! churn tests.

use std::collections::{BTreeMap, HashSet};

use yokohama_core::*;

use crate::describe;
use crate::input::tech_name;

/// Fixed cell width for `board_grid`'s bordered layout.
const CELL_WIDTH: usize = 16;

/// One-line turn/phase/resource summary, e.g.
/// `"Turn 12 | Red | Placement | 5 yen 3 VP | c1 s2 t0 f1 i0 | hand 2"`.
/// `phase_text` fills the third slot verbatim -- the normal phase name
/// (`describe::phase_name`) most of the time, or the pending-choice banner
/// text when `Repl::prompt` (Task 11, design §5.4) detects a queued choice.
/// Deciding WHICH text that is stays the caller's job; this function only
/// formats.
pub fn prompt_header(state: &GameState, phase_text: &str) -> String {
    let p = state.current_player();
    format!(
        "Turn {} | {} | {} | {} yen {} VP | c{} s{} t{} f{} i{} | hand {}",
        state.turn_number,
        color_name(p.color),
        phase_text,
        p.money,
        p.victory_points,
        p.copper,
        p.silk,
        p.tea,
        p.seafood,
        p.imported_goods,
        p.order_cards_in_hand.len(),
    )
}

/// ASCII grid, one bordered cell per area: id + type abbreviation (with a
/// trailing `*` while its 5-Power token is unclaimed), which Presidents are
/// present, which assistants are present, and building ownership.
pub fn board_grid(state: &GameState) -> String {
    let areas = &state.board_layout.areas;
    let rows = areas.values().map(|c| c.grid_position.0 as usize).max().map(|m| m + 1).unwrap_or(0);
    let cols = areas.values().map(|c| c.grid_position.1 as usize).max().map(|m| m + 1).unwrap_or(0);

    let mut grid: Vec<Vec<Option<AreaId>>> = vec![vec![None; cols]; rows];
    for (&id, cfg) in areas.iter() {
        grid[cfg.grid_position.0 as usize][cfg.grid_position.1 as usize] = Some(id);
    }

    let ruler = "-".repeat(cols * (CELL_WIDTH + 1));
    let mut out = String::new();
    out.push_str(&ruler);
    out.push('\n');
    for row in &grid {
        let cells: Vec<[String; 4]> = row
            .iter()
            .map(|slot| match slot {
                Some(id) => area_cell_lines(state, *id),
                None => [fit_cell(""), fit_cell(""), fit_cell(""), fit_cell("")],
            })
            .collect();
        for line_idx in 0..4 {
            let joined = cells.iter().map(|c| c[line_idx].as_str()).collect::<Vec<_>>().join("|");
            out.push_str(&joined);
            out.push('\n');
        }
        out.push_str(&ruler);
        out.push('\n');
    }
    out
}

/// Per Port board: slot number, zone, and the order card there via
/// `order_compact_line` (or `-` when empty), then the shared order-deck size.
pub fn ports_view(state: &GameState, rules: &Rules) -> String {
    let mut out = String::new();
    render_port_board(describe::area_type_name(AreaType::PortA), &state.port_state.port_a, rules, &mut out);
    if let Some(port_b) = &state.port_state.port_b {
        render_port_board(describe::area_type_name(AreaType::PortB), port_b, rules, &mut out);
    }
    out.push_str(&format!("order deck: {}\n", state.order_deck.len()));
    out
}

/// Per Laboratory board: slot number, position surcharge, and the tech card
/// there (name + industry cost), or `-`, then the shared tech-deck size.
pub fn labs_view(state: &GameState, rules: &Rules) -> String {
    let mut out = String::new();
    render_lab_board(describe::area_type_name(AreaType::LaboratoryA), &state.lab_state.lab_a, rules, &mut out);
    if let Some(lab_b) = &state.lab_state.lab_b {
        render_lab_board(describe::area_type_name(AreaType::LaboratoryB), lab_b, rules, &mut out);
    }
    out.push_str(&format!("tech deck: {}\n", state.tech_deck.len()));
    out
}

/// Per player: color, money, VP, goods, imports, hand order codes, completed
/// order count, technology names, tokens/mercenaries, and President location.
pub fn players_panel(state: &GameState, rules: &Rules) -> String {
    let mut out = String::new();
    for p in &state.players {
        out.push_str(&format!(
            "{}: {} yen  {} VP  c{} s{} t{} f{}  imports {}\n",
            color_name(p.color), p.money, p.victory_points, p.copper, p.silk, p.tea, p.seafood, p.imported_goods,
        ));

        let hand = p.order_cards_in_hand.iter().map(|id| format!("O{:02}", id.0)).collect::<Vec<_>>().join(", ");
        out.push_str(&format!("  hand: {}\n", if hand.is_empty() { "-" } else { &hand }));

        out.push_str(&format!("  completed: {}\n", p.completed_orders.len()));

        let techs = p.technologies.iter().map(|id| tech_name(*id, &rules.techs)).collect::<Vec<_>>().join(", ");
        out.push_str(&format!("  tech: {}\n", if techs.is_empty() { "-" } else { &techs }));

        out.push_str(&format!(
            "  tokens: 5power {}  mercenaries {}\n",
            p.five_power_tokens,
            p.mercenaries.total(),
        ));

        let pres = match p.president_location {
            PresidentLocation::InHand => "in hand".to_string(),
            PresidentLocation::OnBoard(area) => describe::area_label(state, area),
        };
        out.push_str(&format!("  president: {}\n", pres));
    }
    out
}

/// Church Management Board (faith values 4-11, occupant color or `-`, `X` on
/// the 2P dummy-blocked faith-7 space -- `eligible_church_spaces` blocks it
/// exactly the way `eligible_customs_spaces_for_cost` blocks Customs space
/// 4) and Customs Management Board (printed cost via `CustomsTrack::
/// space_cost` -- single source, Task 12 -- occupant or `-`, `X` on the 2P
/// dummy-blocked space).
pub fn tracks_view(state: &GameState) -> String {
    let mut out = String::new();

    out.push_str("Church:\n");
    for i in 0..8 {
        let faith = ChurchTrack::index_to_faith(i);
        let blocked = state.config.player_count == 2 && faith == 7;
        let occ = match state.church_track.slots[i] {
            Some(slot) => color_letter(state.player(slot.player).color).to_string(),
            None if blocked => "X".to_string(),
            None => "-".to_string(),
        };
        out.push_str(&format!("  faith {:2}: {}\n", faith, occ));
    }

    out.push_str("Customs:\n");
    for i in 0..7 {
        let space = CustomsTrack::index_to_space(i);
        let blocked = state.config.player_count == 2 && space == 4;
        let occ = match state.customs_track.slots[i] {
            Some(slot) => color_letter(state.player(slot.player).color).to_string(),
            None if blocked => "X".to_string(),
            None => "-".to_string(),
        };
        out.push_str(&format!("  space {} (cost {}): {}\n", space, CustomsTrack::space_cost(space), occ));
    }

    out
}

/// Human-readable lines summarizing everything that changed between `before`
/// and `after`: per-player money/VP/goods/imports diffs, hand and technology
/// gains/losses, assistant placed/moved/recovered/taken lines, church/
/// customs placements, buildings built, mercenary/5-Power token gains,
/// President relocation, phase transition, turn/player handoff, deck-size
/// drops, and the end-game trigger edge.
pub fn delta_summary(before: &GameState, after: &GameState, rules: &Rules) -> Vec<String> {
    let mut lines = Vec::new();

    for (b, a) in before.players.iter().zip(after.players.iter()) {
        let mut parts = Vec::new();
        push_diff(&mut parts, "yen", a.money as i64 - b.money as i64);
        push_diff(&mut parts, "VP", a.victory_points as i64 - b.victory_points as i64);
        push_diff(&mut parts, "copper", a.copper as i64 - b.copper as i64);
        push_diff(&mut parts, "silk", a.silk as i64 - b.silk as i64);
        push_diff(&mut parts, "tea", a.tea as i64 - b.tea as i64);
        push_diff(&mut parts, "seafood", a.seafood as i64 - b.seafood as i64);
        push_diff(&mut parts, "imports", a.imported_goods as i64 - b.imported_goods as i64);
        if !parts.is_empty() {
            lines.push(format!("{}: {}", color_name(a.color), parts.join(", ")));
        }

        let before_hand: HashSet<OrderCardId> = b.order_cards_in_hand.iter().copied().collect();
        let after_hand: HashSet<OrderCardId> = a.order_cards_in_hand.iter().copied().collect();
        let gained = sorted_order_codes(after_hand.difference(&before_hand).copied());
        let lost = sorted_order_codes(before_hand.difference(&after_hand).copied());
        if !gained.is_empty() {
            lines.push(format!("{}: hand +{}", color_name(a.color), gained.join(",")));
        }
        if !lost.is_empty() {
            lines.push(format!("{}: hand -{}", color_name(a.color), lost.join(",")));
        }

        let before_tech: HashSet<TechCardId> = b.technologies.iter().copied().collect();
        let mut gained_tech: Vec<TechCardId> =
            a.technologies.iter().copied().filter(|id| !before_tech.contains(id)).collect();
        gained_tech.sort_unstable_by_key(|id| id.0);
        if !gained_tech.is_empty() {
            let names = gained_tech.iter().map(|id| tech_name(*id, &rules.techs)).collect::<Vec<_>>().join(", ");
            lines.push(format!("{}: tech +{}", color_name(a.color), names));
        }

        assistant_lines(&mut lines, before, after, b, a);
        warehouse_take_lines(&mut lines, b, a);
        church_customs_lines(&mut lines, b, a);
        token_lines(&mut lines, b, a);

        if a.president_location != b.president_location {
            lines.push(format!(
                "{}: President {} -> {}",
                color_name(a.color),
                president_label(after, b.president_location),
                president_label(after, a.president_location),
            ));
        }
    }

    building_lines(&mut lines, before, after);

    if before.turn_phase != after.turn_phase {
        lines.push(format!(
            "Phase: {} -> {}",
            describe::phase_name(before.turn_phase),
            describe::phase_name(after.turn_phase)
        ));
    }

    if before.current_player != after.current_player || before.turn_number != after.turn_number {
        lines.push(format!(
            "Turn {} {} -> Turn {} {}",
            before.turn_number,
            color_name(before.current_player().color),
            after.turn_number,
            color_name(after.current_player().color),
        ));
    }

    if before.order_deck.len() != after.order_deck.len() {
        lines.push(format!("order deck {} -> {}", before.order_deck.len(), after.order_deck.len()));
    }
    if before.tech_deck.len() != after.tech_deck.len() {
        lines.push(format!("tech deck {} -> {}", before.tech_deck.len(), after.tech_deck.len()));
    }

    if after.end_game_triggered && !before.end_game_triggered {
        lines.push(format!("END TRIGGERED: {:?}", after.trigger_reason));
    }

    lines
}

/// Push `"+N label"` / `"-N label"` when `diff` is non-zero; silent at zero.
fn push_diff(parts: &mut Vec<String>, label: &str, diff: i64) {
    if diff > 0 {
        parts.push(format!("+{diff} {label}"));
    } else if diff < 0 {
        parts.push(format!("{diff} {label}"));
    }
}

/// Render order-card ids as sorted `O{:02}` codes (deterministic output from
/// a set difference, which has no inherent order).
fn sorted_order_codes(ids: impl Iterator<Item = OrderCardId>) -> Vec<String> {
    let mut nums: Vec<u8> = ids.map(|id| id.0).collect();
    nums.sort_unstable();
    nums.into_iter().map(|n| format!("O{:02}", n)).collect()
}

/// President location, rendered on `describe::area_label`'s convention
/// (type name + `(A00)`) for consistency with every other area reference in
/// the CLI's move/choice text -- board_layout is immutable after setup, so
/// either state works here; `after` is used for no particular reason beyond
/// being in scope at both call sites.
fn president_label(state: &GameState, loc: PresidentLocation) -> String {
    match loc {
        PresidentLocation::InHand => "hand".to_string(),
        PresidentLocation::OnBoard(area) => describe::area_label(state, area),
    }
}

/// Classify `AssistantState` `on_board`/`in_hand` diffs into "placed"/
/// "moved"/"recovered" lines. Board-to-board pairs are matched first: only
/// the `ChooseAssistantMove` reward ever moves an assistant area-to-area,
/// so pairing a unit of board-loss at one area with a unit of board-gain at
/// another is unambiguous. Any board loss left unpaired is Recovery (the
/// only other way an on-board assistant leaves); any board gain left
/// unpaired came from hand -- during the Placement phase that's a
/// `PlaceThreeAreas`/`PlaceTwoSameArea` placement (the only moves legal in
/// that phase), everywhere else it's a hand-sourced `ChooseAssistantMove`
/// (Placement never queues a reward, so `before.turn_phase` cleanly
/// disambiguates a shape both moves produce identically).
///
/// One caveat, accepted for a v1 CLI display feature: a Church/Customs
/// placement drawn from the acting area (rather than hand, when hand is
/// empty -- `take_church_or_customs_assistant` prefers hand, falls back to
/// the acting area) is reserved out of `board_diff` below so it isn't ALSO
/// reported as a spurious "recovered" line; the reservation itself can
/// mis-attribute by one unit in the extremely rare case where a singleton
/// movement-reward AND a hand-empty Church/Customs placement land in the
/// same move's diff (both draw on the same acting-area count). Not reached
/// by any move shape the current engine actually produces outside a
/// contrived fixture.
fn assistant_lines(lines: &mut Vec<String>, before: &GameState, after: &GameState, b: &PlayerState, a: &PlayerState) {
    let mut board_diff: BTreeMap<AreaId, i64> = BTreeMap::new();
    for (&area, &n) in &b.assistants.on_board {
        *board_diff.entry(area).or_insert(0) -= n as i64;
    }
    for (&area, &n) in &a.assistants.on_board {
        *board_diff.entry(area).or_insert(0) += n as i64;
    }

    let track_placed = a.church_positions.len().saturating_sub(b.church_positions.len())
        + a.customs_positions.len().saturating_sub(b.customs_positions.len());
    let hand_shrink = (b.assistants.in_hand as i64 - a.assistants.in_hand as i64).max(0) as usize;
    let from_board = track_placed.saturating_sub(hand_shrink);
    if from_board > 0 {
        if let PresidentLocation::OnBoard(area) = a.president_location {
            if let Some(d) = board_diff.get_mut(&area) {
                *d += from_board as i64;
                if *d == 0 {
                    board_diff.remove(&area);
                }
            }
        }
    }

    let mut negatives: Vec<(AreaId, i64)> =
        board_diff.iter().filter(|&(_, &d)| d < 0).map(|(&k, &v)| (k, -v)).collect();
    let mut positives: Vec<(AreaId, i64)> =
        board_diff.iter().filter(|&(_, &d)| d > 0).map(|(&k, &v)| (k, v)).collect();

    let mut ni = 0;
    let mut pi = 0;
    while ni < negatives.len() && pi < positives.len() {
        lines.push(format!(
            "{}: assistant moved from {} to {}",
            color_name(a.color),
            describe::area_label(after, negatives[ni].0),
            describe::area_label(after, positives[pi].0),
        ));
        negatives[ni].1 -= 1;
        positives[pi].1 -= 1;
        if negatives[ni].1 == 0 { ni += 1; }
        if positives[pi].1 == 0 { pi += 1; }
    }

    for &(area, n) in &negatives[ni..] {
        if n > 0 {
            lines.push(format!(
                "{}: {} assistant{} recovered from {}",
                color_name(a.color), n, if n == 1 { "" } else { "s" }, describe::area_label(after, area),
            ));
        }
    }
    for &(area, n) in &positives[pi..] {
        if n <= 0 {
            continue;
        }
        if before.turn_phase == TurnPhase::Placement {
            lines.push(format!(
                "{}: {} assistant{} placed in {}",
                color_name(a.color), n, if n == 1 { "" } else { "s" }, describe::area_label(after, area),
            ));
        } else {
            lines.push(format!(
                "{}: assistant moved from hand to {}",
                color_name(a.color), describe::area_label(after, area),
            ));
        }
    }
}

/// "took X (Y yen)" lines for warehouse -> hand token transitions
/// (`ChooseWarehouseTake`, `EmploymentAgency`, and `UseUniversity` all
/// produce the same field-diff shape). Cost is the player's own total money
/// diff when exactly one item type transitioned in this move -- correct
/// even for `UseUniversity`'s non-standard 1-Yen assistant price, since
/// these three move types never bundle with any other cost in the same
/// move. The static take-cost progression is used instead when >1 item
/// type transitioned (only `EmploymentAgency`'s multi-item basket does
/// this, and it always charges the canonical progression anyway, so the
/// two computations agree).
fn warehouse_take_lines(lines: &mut Vec<String>, b: &PlayerState, a: &PlayerState) {
    let assistant_n = (b.assistants.in_warehouse as i64 - a.assistants.in_warehouse as i64).max(0) as u32;
    let shophouse_n = (b.shophouses.in_warehouse as i64 - a.shophouses.in_warehouse as i64).max(0) as u32;
    let th_n = (b.trading_houses.in_warehouse as i64 - a.trading_houses.in_warehouse as i64).max(0) as u32;
    let kinds = (assistant_n > 0) as u32 + (shophouse_n > 0) as u32 + (th_n > 0) as u32;
    if kinds == 0 {
        return;
    }

    let money_diff = a.money as i64 - b.money as i64;
    let actual_cost = (-money_diff).max(0) as u32;
    let already = b.trading_houses.already_taken();

    if assistant_n > 0 {
        let cost = if kinds == 1 { actual_cost } else { 0 };
        push_took_line(lines, a.color, "assistant", "assistants", assistant_n, cost);
    }
    if shophouse_n > 0 {
        let cost = if kinds == 1 { actual_cost } else { 2 * shophouse_n };
        push_took_line(lines, a.color, "shophouse", "shophouses", shophouse_n, cost);
    }
    if th_n > 0 {
        let cost = if kinds == 1 {
            actual_cost
        } else {
            (0..th_n).map(|i| TradingHouseState::take_cost(already + i as usize)).sum()
        };
        push_took_line(lines, a.color, "trading house", "trading houses", th_n, cost);
    }
}

fn push_took_line(lines: &mut Vec<String>, color: PlayerColor, singular: &str, plural: &str, n: u32, cost: u32) {
    let word = if n == 1 { singular.to_string() } else { format!("{n} {plural}") };
    let cost_text = if cost == 0 { "free".to_string() } else { format!("{cost} yen") };
    lines.push(format!("{}: took {} ({})", color_name(color), word, cost_text));
}

/// "placed on Church faith-N (+M VP)" / "placed on Customs space N (+M VP)"
/// lines from `church_positions`/`customs_positions` growing (append-only:
/// once placed, an assistant's track slot is never vacated, so the newly
/// appended tail is exactly what changed this move). VP shown is the
/// printed base reward (`ChurchTrack`/`CustomsTrack::vp_reward`), matching
/// `describe_move`'s preview text for the same choice -- not necessarily
/// the player's total VP delta for the move, which can also include a
/// passive tech bonus (Winery/Brickyard) this line doesn't itemize.
fn church_customs_lines(lines: &mut Vec<String>, b: &PlayerState, a: &PlayerState) {
    for &space in &a.church_positions[b.church_positions.len()..] {
        lines.push(format!(
            "{}: placed on Church faith-{space} (+{} VP)",
            color_name(a.color), ChurchTrack::vp_reward(space),
        ));
    }
    for &space in &a.customs_positions[b.customs_positions.len()..] {
        lines.push(format!(
            "{}: placed on Customs space {space} (+{} VP)",
            color_name(a.color), CustomsTrack::vp_reward(space),
        ));
    }
}

/// "gained mercenary (Country)" / "gained 5-Power token" lines. A per-move
/// gain above 1 (not reachable under current rules -- `award_mercenaries_
/// for_country_icons` awards at most 1 per country per call) is suffixed
/// `x{n}` rather than pluralized, so the line still contains the literal
/// substring "gained mercenary" regardless.
fn token_lines(lines: &mut Vec<String>, b: &PlayerState, a: &PlayerState) {
    for &country in &Country::ALL {
        let before_n = b.mercenaries.available.get(&country).copied().unwrap_or(0)
            + b.mercenaries.used.get(&country).copied().unwrap_or(0);
        let after_n = a.mercenaries.available.get(&country).copied().unwrap_or(0)
            + a.mercenaries.used.get(&country).copied().unwrap_or(0);
        if after_n > before_n {
            let n = after_n - before_n;
            let suffix = if n == 1 { String::new() } else { format!(" x{n}") };
            lines.push(format!(
                "{}: gained mercenary ({}){}",
                color_name(a.color), describe::country_name(country), suffix,
            ));
        }
    }

    if a.five_power_tokens > b.five_power_tokens {
        let n = a.five_power_tokens - b.five_power_tokens;
        let suffix = if n == 1 { String::new() } else { format!(" x{n}") };
        lines.push(format!("{}: gained 5-Power token{}", color_name(a.color), suffix));
    }
}

/// "built shophouse"/"built trading house" lines from `board_layout.
/// building_sites` diffs -- never removed once set, so a `None -> Some`
/// transition on a slot/trading-house-owner is the only case to detect.
/// Board-level (not per-player), since a building site's ownership fields
/// name the owner directly.
fn building_lines(lines: &mut Vec<String>, before: &GameState, after: &GameState) {
    for (area, after_site) in &after.board_layout.building_sites {
        let before_site = before.board_layout.building_sites.get(area);
        for slot in 0..4 {
            let before_owner = before_site.and_then(|s| s.shophouses_built[slot]);
            if before_owner.is_none() {
                if let Some(owner) = after_site.shophouses_built[slot] {
                    lines.push(format!(
                        "{}: built shophouse at {}",
                        color_name(after.player(owner).color), describe::area_label(after, *area),
                    ));
                }
            }
        }
        let before_th = before_site.and_then(|s| s.trading_house_owner);
        if before_th.is_none() {
            if let Some(owner) = after_site.trading_house_owner {
                lines.push(format!(
                    "{}: built trading house at {}",
                    color_name(after.player(owner).color), describe::area_label(after, *area),
                ));
            }
        }
    }
}

fn render_port_board(label: &str, board: &PortBoard, rules: &Rules, out: &mut String) {
    out.push_str(label);
    out.push('\n');
    for i in 0..6 {
        let zone = match PortBoard::zone_of(i) {
            PortZone::Low => "Low",
            PortZone::Mid => "Mid",
            PortZone::High => "High",
        };
        let line = match board.slots[i] {
            Some(card_id) => {
                rules.orders.get(card_id).map(order_compact_line).unwrap_or_else(|| format!("O{:02}", card_id.0))
            }
            None => "-".to_string(),
        };
        out.push_str(&format!("{}. {}  {}\n", i + 1, zone, line));
    }
}

fn render_lab_board(label: &str, board: &LabBoard, rules: &Rules, out: &mut String) {
    out.push_str(label);
    out.push('\n');
    for i in 0..6 {
        let surcharge = LabBoard::position_modifier(i);
        let line = match board.slots[i] {
            Some(tech_id) => match rules.techs.get(tech_id) {
                Some(card) => format!("{} (cost {})", card.name, card.industry_cost),
                None => format!("T{:02}", tech_id.0),
            },
            None => "-".to_string(),
        };
        out.push_str(&format!("{}. +{}  {}\n", i + 1, surcharge, line));
    }
}

/// The 4 fixed-width lines for one `board_grid` cell.
fn area_cell_lines(state: &GameState, id: AreaId) -> [String; 4] {
    let cfg = &state.board_layout.areas[&id];
    let marker = matches!(state.board_layout.five_power_tokens.get(&id), Some(Some(_)));
    let header = format!("A{:02} {}{}", id.0, area_abbrev(cfg.area_type), if marker { " *" } else { "" });

    let pres: String = state
        .players
        .iter()
        .filter(|p| p.president_location == PresidentLocation::OnBoard(id))
        .map(|p| color_letter(p.color))
        .collect();
    let pres_line = format!("P: {}", if pres.is_empty() { "-".to_string() } else { pres });

    let assist_parts: Vec<String> = state
        .players
        .iter()
        .filter_map(|p| {
            let n = p.assistants.in_area(id);
            if n > 0 { Some(format!("{}{}", color_letter(p.color), n)) } else { None }
        })
        .collect();
    let assist_line = if assist_parts.is_empty() { "-".to_string() } else { assist_parts.join(" ") };

    let build_line = match state.board_layout.building_sites.get(&id) {
        Some(site) => {
            let sh: String =
                site.shophouses_built.iter().filter_map(|o| o.map(|pid| color_letter(state.player(pid).color))).collect();
            let th = site
                .trading_house_owner
                .map(|pid| color_letter(state.player(pid).color).to_string())
                .unwrap_or_else(|| "-".to_string());
            format!("sh {} | th {}", if sh.is_empty() { "-".to_string() } else { sh }, th)
        }
        None => "no site".to_string(),
    };

    [fit_cell(&header), fit_cell(&pres_line), fit_cell(&assist_line), fit_cell(&build_line)]
}

/// Truncate/pad to `CELL_WIDTH` so grid rows line up under `|` separators.
fn fit_cell(s: &str) -> String {
    let truncated: String = s.chars().take(CELL_WIDTH).collect();
    format!("{:<width$}", truncated, width = CELL_WIDTH)
}

fn area_abbrev(t: AreaType) -> &'static str {
    match t {
        AreaType::CopperMine => "CuM",
        AreaType::SilkMill => "SiM",
        AreaType::TeaPlantation => "TeaP",
        AreaType::FishingGround => "FishG",
        AreaType::Bank => "Bank",
        AreaType::PortA => "PortA",
        AreaType::PortB => "PortB",
        AreaType::LaboratoryA => "LabA",
        AreaType::LaboratoryB => "LabB",
        AreaType::EmploymentAgency => "EmpAg",
        AreaType::Chinatown => "Chna",
        AreaType::Church => "Chrch",
        AreaType::Customs => "Cstms",
        AreaType::Canal => "Canal",
    }
}

fn color_name(c: PlayerColor) -> &'static str {
    match c {
        PlayerColor::Red => "Red",
        PlayerColor::Blue => "Blue",
        PlayerColor::Yellow => "Yellow",
        PlayerColor::Green => "Green",
    }
}

fn color_letter(c: PlayerColor) -> char {
    color_name(c).chars().next().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state_2p() -> GameState {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 1).unwrap();
        finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)]).unwrap()
    }

    #[test]
    fn prompt_header_shows_turn_color_phase_and_money() {
        let state = test_state_2p();
        let h = prompt_header(&state, describe::phase_name(state.turn_phase));
        assert!(h.contains("Turn 1"), "{h}");
        assert!(h.contains("Red"), "{h}");
        assert!(h.contains("Additional actions (before)"), "{h}");
        assert!(h.contains("3 yen"), "{h}");
    }

    #[test]
    fn board_grid_has_one_cell_per_area_with_ids() {
        let state = test_state_2p();
        let g = board_grid(&state);
        for id in state.board_layout.areas.keys() {
            assert!(g.contains(&format!("A{:02}", id.0)), "missing A{:02} in\n{g}", id.0);
        }
    }

    #[test]
    fn ports_view_lists_six_slots_and_deck_count() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let v = ports_view(&state, &rules);
        assert!(v.contains("Port A"), "{v}");
        for zone in ["Low", "Mid", "High"] {
            assert!(v.contains(zone), "{v}");
        }
        assert!(v.contains(&format!("order deck: {}", state.order_deck.len())), "{v}");
    }

    #[test]
    fn labs_view_lists_surcharges_and_deck_count() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let v = labs_view(&state, &rules);
        assert!(v.contains("Laboratory A"), "{v}");
        assert!(v.contains("+0"), "{v}");
        assert!(v.contains("+1"), "{v}");
        assert!(v.contains("+2"), "{v}");
        assert!(v.contains(&format!("tech deck: {}", state.tech_deck.len())), "{v}");
    }

    #[test]
    fn players_panel_lists_each_player_by_color_and_money() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let v = players_panel(&state, &rules);
        assert!(v.contains("Red"), "{v}");
        assert!(v.contains("Blue"), "{v}");
        assert!(v.contains("3 yen"), "{v}");
        assert!(v.contains("4 yen"), "{v}");
    }

    #[test]
    fn tracks_view_shows_church_and_customs_with_2p_block() {
        let state = test_state_2p();
        let v = tracks_view(&state);
        assert!(v.contains("Church"), "{v}");
        assert!(v.contains("Customs"), "{v}");
        assert!(v.contains('X'), "{v}");
    }

    /// Task 12: the engine blocks Church faith-7 in 2p games exactly the
    /// way it blocks Customs space 4 (`eligible_church_spaces` /
    /// `eligible_customs_spaces_for_cost`, both gate on `player_count == 2`)
    /// -- `tracks_view` must mark it with the same `X` dummy convention.
    #[test]
    fn tracks_view_marks_church_faith_7_dummy_at_2p() {
        let state = test_state_2p();
        let v = tracks_view(&state);
        // Computed via the same `{:2}` width the SUT formats with, so this
        // assertion doesn't hardcode a specific padding guess.
        let needle = format!("faith {:2}:", 7);
        let line = v.lines().find(|l| l.contains(&needle)).expect("faith-7 line present");
        assert!(line.trim_end().ends_with('X'), "{line}");
    }

    /// Reviewer-check per the Task 12 brief: `render.rs` must single-source
    /// the Customs cost progression through `CustomsTrack::space_cost`
    /// rather than a hardcoded array literal. Built via `format!` from bare
    /// numbers (not typed as the bracketed literal itself) so this
    /// assertion doesn't quine against its own source text.
    #[test]
    fn render_source_no_longer_hardcodes_customs_costs() {
        let forbidden = format!("[{}, {}, {}, {}, {}, {}, {}]", 1, 1, 2, 2, 3, 3, 4);
        let src = include_str!("render.rs");
        assert!(!src.contains(&forbidden), "customs costs must be single-sourced via CustomsTrack::space_cost");
    }

    #[test]
    fn delta_summary_reports_a_production_gain() {
        let rules = Rules::standard();
        let state = test_state_2p();
        // Drive to the first Placement decision, then walk a fixed legal path:
        // EndAdditionalPhase is always legal in AdditionalActionFirst.
        let s1 = apply_move(&state, &Move::EndAdditionalPhase, &rules).unwrap();
        let lines = delta_summary(&state, &s1, &rules);
        assert!(lines.iter().any(|l| l.contains("Placement")), "{lines:?}");
    }

    #[test]
    fn delta_summary_reports_a_shophouse_build() {
        let rules = Rules::standard();
        let before = test_state_2p();
        let mut after = before.clone();
        let area = *before.board_layout.building_sites.keys().next().expect("a building site exists");
        after.board_layout.building_sites.get_mut(&area).unwrap().shophouses_built[0] = Some(PlayerId::new(0));
        let lines = delta_summary(&before, &after, &rules);
        assert!(lines.iter().any(|l| l.contains("built shophouse")), "{lines:?}");
    }

    #[test]
    fn delta_summary_reports_a_trading_house_build() {
        let rules = Rules::standard();
        let before = test_state_2p();
        let mut after = before.clone();
        let area = *before.board_layout.building_sites.keys().next().expect("a building site exists");
        after.board_layout.building_sites.get_mut(&area).unwrap().trading_house_owner = Some(PlayerId::new(1));
        let lines = delta_summary(&before, &after, &rules);
        assert!(lines.iter().any(|l| l.contains("built trading house")), "{lines:?}");
    }

    #[test]
    fn delta_summary_reports_a_mercenary_gain() {
        let rules = Rules::standard();
        let before = test_state_2p();
        let mut after = before.clone();
        after.players[0].mercenaries.available.insert(Country::English, 1);
        let lines = delta_summary(&before, &after, &rules);
        assert!(lines.iter().any(|l| l.contains("gained mercenary")), "{lines:?}");
    }

    #[test]
    fn delta_summary_reports_a_five_power_token_gain() {
        let rules = Rules::standard();
        let before = test_state_2p();
        let mut after = before.clone();
        after.players[0].five_power_tokens += 1;
        let lines = delta_summary(&before, &after, &rules);
        assert!(lines.iter().any(|l| l.contains("gained 5-Power token")), "{lines:?}");
    }

    #[test]
    fn delta_summary_reports_a_customs_placement() {
        let rules = Rules::standard();
        let before = test_state_2p();
        let mut after = before.clone();
        after.players[0].assistants.in_hand -= 1;
        after.players[0].assistants.on_customs += 1;
        after.players[0].customs_positions.push(3);
        after.players[0].victory_points += CustomsTrack::vp_reward(3) as u32;
        let lines = delta_summary(&before, &after, &rules);
        let expected = format!("placed on Customs space 3 (+{} VP)", CustomsTrack::vp_reward(3));
        assert!(lines.iter().any(|l| l.contains(&expected)), "{lines:?}");
    }

    /// President-label consistency (Task 12 riding item): the delta line
    /// must read through `describe::area_label` (type name + `(A00)`), not
    /// the old bare `A00` format.
    #[test]
    fn delta_summary_president_relocation_uses_area_label() {
        let rules = Rules::standard();
        let before = test_state_2p();
        let mut after = before.clone();
        let area = *before.board_layout.areas.keys().next().expect("at least one area");
        after.players[0].president_location = PresidentLocation::OnBoard(area);
        let lines = delta_summary(&before, &after, &rules);
        let expected = describe::area_label(&after, area);
        let line = lines.iter().find(|l| l.contains("President")).expect("a President relocation line");
        assert!(line.contains(&expected), "{line}");
    }

    #[test]
    fn delta_summary_reports_assistant_placement_during_placement_phase() {
        let rules = Rules::standard();
        let mut before = test_state_2p();
        before.turn_phase = TurnPhase::Placement;
        let mut after = before.clone();
        let area = *before
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type != AreaType::Canal)
            .expect("a non-Canal area exists")
            .0;
        after.players[0].assistants.in_hand -= 1;
        *after.players[0].assistants.on_board.entry(area).or_insert(0) += 1;
        let lines = delta_summary(&before, &after, &rules);
        assert!(lines.iter().any(|l| l.contains("placed in")), "{lines:?}");
    }

    #[test]
    fn delta_summary_reports_assistant_recovery() {
        let rules = Rules::standard();
        let mut before = test_state_2p();
        let area = *before
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type != AreaType::Canal)
            .expect("a non-Canal area exists")
            .0;
        before.players[0].assistants.in_hand -= 2;
        before.players[0].assistants.on_board.insert(area, 2);
        let mut after = before.clone();
        after.players[0].assistants.on_board.remove(&area);
        after.players[0].assistants.in_hand += 2;
        let lines = delta_summary(&before, &after, &rules);
        assert!(lines.iter().any(|l| l.contains("recovered from")), "{lines:?}");
    }

    /// The "took" cost figure must reflect what actually happened, not a
    /// pricing assumption -- a plain warehouse->hand assistant transfer
    /// with no other change is priced at the player's own (zero) money
    /// diff here, exercising the "free" formatting branch directly.
    #[test]
    fn delta_summary_took_line_prices_a_free_assistant_take() {
        let rules = Rules::standard();
        let before = test_state_2p();
        let mut after = before.clone();
        after.players[0].assistants.in_warehouse -= 1;
        after.players[0].assistants.in_hand += 1;
        let lines = delta_summary(&before, &after, &rules);
        assert!(lines.iter().any(|l| l.contains("took assistant (free)")), "{lines:?}");
    }
}
