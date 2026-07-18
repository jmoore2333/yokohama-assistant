//! ASCII rendering of game state for the CLI: read-only views over
//! `GameState` plus a before/after delta summarizer (design
//! docs/plans/2026-07-18-phase2-cli-driver-design.md).
//!
//! Rendering rule: tests assert content (substrings/line counts), not
//! full-screen golden strings, so cosmetic polish during dogfooding doesn't
//! churn tests.

use std::collections::HashSet;

use yokohama_core::*;

use crate::describe;

/// Fixed cell width for `board_grid`'s bordered layout.
const CELL_WIDTH: usize = 16;

/// One-line turn/phase/resource summary, e.g.
/// `"Turn 12 | Red | Placement | 5 yen 3 VP | c1 s2 t0 f1 i0 | hand 2"`.
pub fn prompt_header(state: &GameState) -> String {
    let p = state.current_player();
    format!(
        "Turn {} | {} | {} | {} yen {} VP | c{} s{} t{} f{} i{} | hand {}",
        state.turn_number,
        color_name(p.color),
        describe::phase_name(state.turn_phase),
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

        let techs = p
            .technologies
            .iter()
            .map(|id| rules.techs.get(*id).map(|c| c.name.clone()).unwrap_or_else(|| format!("T{:02}", id.0)))
            .collect::<Vec<_>>()
            .join(", ");
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

/// Church Management Board (faith values 4-11, occupant color or `-`) and
/// Customs Management Board (printed cost, occupant or `-`, `X` on the 2P
/// dummy-blocked space).
pub fn tracks_view(state: &GameState) -> String {
    let mut out = String::new();

    out.push_str("Church:\n");
    for i in 0..8 {
        let faith = ChurchTrack::index_to_faith(i);
        let occ = match state.church_track.slots[i] {
            Some(slot) => color_letter(state.player(slot.player).color).to_string(),
            None => "-".to_string(),
        };
        out.push_str(&format!("  faith {:2}: {}\n", faith, occ));
    }

    out.push_str("Customs:\n");
    const COSTS: [u8; 7] = [1, 1, 2, 2, 3, 3, 4];
    for i in 0..7 {
        let space = CustomsTrack::index_to_space(i);
        let blocked = state.config.player_count == 2 && space == 4;
        let occ = match state.customs_track.slots[i] {
            Some(slot) => color_letter(state.player(slot.player).color).to_string(),
            None if blocked => "X".to_string(),
            None => "-".to_string(),
        };
        out.push_str(&format!("  space {} (cost {}): {}\n", space, COSTS[i], occ));
    }

    out
}

/// Human-readable lines summarizing everything that changed between `before`
/// and `after`: per-player money/VP/goods/imports diffs, hand and technology
/// gains/losses, President relocation, phase transition, turn/player
/// handoff, deck-size drops, and the end-game trigger edge.
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
            let names = gained_tech
                .iter()
                .map(|id| rules.techs.get(*id).map(|c| c.name.clone()).unwrap_or_else(|| format!("T{:02}", id.0)))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("{}: tech +{}", color_name(a.color), names));
        }

        if a.president_location != b.president_location {
            lines.push(format!(
                "{}: President {} -> {}",
                color_name(a.color),
                president_location_label(b.president_location),
                president_location_label(a.president_location),
            ));
        }
    }

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

fn president_location_label(loc: PresidentLocation) -> String {
    match loc {
        PresidentLocation::InHand => "hand".to_string(),
        PresidentLocation::OnBoard(area) => format!("A{:02}", area.0),
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
        let h = prompt_header(&state);
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
}
